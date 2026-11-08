//! 可靠 UDP 语义：按 sequence 去重、ACK 确认、确定性 tick 重传。
//!
//! 重传由**逻辑 tick** 驱动，不读墙钟（ADR 0002）；到达顺序不决定执行
//! 顺序，排序交给 [`crate::frame::FrameScheduler`]。ACK 为轻量控制包，
//! 不走可靠队列，避免无限递归。

use std::collections::{BTreeMap, BTreeSet};

use crate::error::NetError;
use crate::packet::{Packet, PacketKind, payload_ack};

struct Pending {
    bytes: Vec<u8>,
    sent_tick: u64,
}

/// 已发送但未确认的数据包队列。
pub struct ReliableSender {
    next_sequence: u32,
    unacked: BTreeMap<u32, Pending>,
    retransmit_after: u64,
}

impl ReliableSender {
    pub fn new(retransmit_after: u64) -> Self {
        Self {
            next_sequence: 1,
            unacked: BTreeMap::new(),
            retransmit_after: retransmit_after.max(1),
        }
    }

    /// 分配 sequence 并返回可直接发送的编码字节。
    pub fn queue(&mut self, mut packet: Packet) -> Result<(u32, Vec<u8>), NetError> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if self.next_sequence == 0 {
            self.next_sequence = 1;
        }
        packet.sequence = sequence;
        let bytes = packet.encode()?;
        self.unacked.insert(
            sequence,
            Pending {
                bytes: bytes.clone(),
                sent_tick: 0,
            },
        );
        Ok((sequence, bytes))
    }

    /// 确认一个 sequence；返回是否确实移除了未确认项。
    pub fn on_ack(&mut self, sequence: u32) -> bool {
        self.unacked.remove(&sequence).is_some()
    }

    /// 以确定性逻辑 tick 驱动重传；返回需重发的编码字节。
    pub fn tick(&mut self, tick: u64) -> Vec<Vec<u8>> {
        let mut retransmissions = Vec::new();
        for pending in self.unacked.values_mut() {
            if tick.saturating_sub(pending.sent_tick) >= self.retransmit_after {
                pending.sent_tick = tick;
                retransmissions.push(pending.bytes.clone());
            }
        }
        retransmissions
    }

    pub fn pending_count(&self) -> usize {
        self.unacked.len()
    }

    pub fn next_sequence(&self) -> u32 {
        self.next_sequence
    }
}

/// 接收侧去重集合；重复包也回 ACK，用于修复丢失的 ACK。
#[derive(Default)]
pub struct ReliableReceiver {
    delivered: BTreeSet<u32>,
}

impl ReliableReceiver {
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回 `(是否为新包, 应 ACK 的 sequence)`。
    pub fn accept(&mut self, sequence: u32) -> (bool, u32) {
        let is_new = self.delivered.insert(sequence);
        (is_new, sequence)
    }

    pub fn delivered_count(&self) -> usize {
        self.delivered.len()
    }
}

/// 构造一个 ACK 控制包（不进可靠队列）。
pub fn ack_packet(sender: u8, acked_sequence: u32, frame: u64) -> Packet {
    Packet::new(
        PacketKind::Ack,
        sender,
        acked_sequence,
        frame,
        payload_ack(acked_sequence),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::parse_ack;

    fn data_packet() -> Packet {
        Packet::new(PacketKind::Input, 1, 0, 4, vec![1, 2, 3])
    }

    #[test]
    fn queue_assigns_increasing_sequences_and_encodes() {
        let mut sender = ReliableSender::new(3);
        let (s1, b1) = sender.queue(data_packet()).unwrap();
        let (s2, b2) = sender.queue(data_packet()).unwrap();
        assert_eq!((s1, s2), (1, 2));
        assert_eq!(sender.next_sequence(), 3);
        assert_eq!(sender.pending_count(), 2);
        assert_eq!(Packet::decode(&b1).unwrap().sequence, s1);
        assert_eq!(Packet::decode(&b2).unwrap().sequence, s2);
    }

    #[test]
    fn ack_removes_pending_and_is_idempotent() {
        let mut sender = ReliableSender::new(3);
        let (sequence, _) = sender.queue(data_packet()).unwrap();
        assert!(sender.on_ack(sequence));
        assert!(!sender.on_ack(sequence));
        assert_eq!(sender.pending_count(), 0);
    }

    #[test]
    fn retransmission_is_driven_by_tick_not_wall_clock() {
        let mut sender = ReliableSender::new(3);
        sender.queue(data_packet()).unwrap();
        assert!(sender.tick(1).is_empty());
        assert!(sender.tick(2).is_empty());
        let first = sender.tick(3);
        assert_eq!(first.len(), 1, "should retransmit once at threshold");
        assert!(
            sender.tick(4).is_empty(),
            "should not immediately retransmit again"
        );
        assert_eq!(sender.tick(6).len(), 1);
        sender.on_ack(1);
        assert!(
            sender.tick(9).is_empty(),
            "should stop retransmitting after ack"
        );
    }

    #[test]
    fn receiver_deduplicates_but_still_acks_duplicates() {
        let mut receiver = ReliableReceiver::new();
        assert_eq!(receiver.accept(7), (true, 7));
        assert_eq!(receiver.accept(7), (false, 7));
        assert_eq!(receiver.accept(9), (true, 9));
        assert_eq!(receiver.delivered_count(), 2);
        let ack = ack_packet(2, 7, 4);
        assert_eq!(parse_ack(&ack.payload).unwrap(), 7);
        assert_eq!(ack.kind, PacketKind::Ack);
    }
}
