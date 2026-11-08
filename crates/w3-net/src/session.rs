//! 客户端会话：把传输 / 可靠层 / 帧编排 / checksum / 重连串起来。
//!
//! 本结构**不持有** `RtsMatch`；权威状态由 app 层持有并调用
//! [`crate::frame::apply_frame`]。网络层只编排输入命令与校验值。

use std::collections::VecDeque;

use w3_core::Command;

use crate::checksum::{ChecksumExchange, FrameChecksum};
use crate::error::NetError;
use crate::frame::{DEFAULT_INPUT_TIMEOUT_FRAMES, FrameScheduler};
use crate::packet::{
    FrameCommands, Packet, PacketKind, decode_commands, encode_commands, parse_ack, parse_reject,
    parse_resync_replay, parse_welcome,
};
use crate::reconnect::{ReconnectController, SessionCursor, resync_request_packet};
use crate::reliable::{ReliableReceiver, ReliableSender, ack_packet};
use crate::transport::{Endpoint, Transport};

/// 一个联机客户端会话。
pub struct ClientSession<T: Transport> {
    transport: T,
    relay: Endpoint,
    player_id: u8,
    scheduler: FrameScheduler,
    checksums: ChecksumExchange,
    reconnect: ReconnectController,
    sender: ReliableSender,
    receiver: ReliableReceiver,
    ready: VecDeque<FrameCommands>,
    tick: u64,
}

impl<T: Transport> ClientSession<T> {
    pub fn new(transport: T, relay: Endpoint, player_id: u8, start_frame: u64) -> Self {
        Self::with_timeout(
            transport,
            relay,
            player_id,
            start_frame,
            DEFAULT_INPUT_TIMEOUT_FRAMES,
        )
    }

    pub fn with_timeout(
        transport: T,
        relay: Endpoint,
        player_id: u8,
        start_frame: u64,
        input_timeout: u64,
    ) -> Self {
        Self {
            transport,
            relay,
            player_id,
            scheduler: FrameScheduler::new(start_frame, input_timeout),
            checksums: ChecksumExchange::new(),
            reconnect: ReconnectController::new(SessionCursor::new(player_id, start_frame)),
            sender: ReliableSender::new(3),
            receiver: ReliableReceiver::new(),
            ready: VecDeque::new(),
            tick: 0,
        }
    }

    pub fn player_id(&self) -> u8 {
        self.player_id
    }

    pub fn checksums(&self) -> &ChecksumExchange {
        &self.checksums
    }

    pub fn reconnect(&self) -> &ReconnectController {
        &self.reconnect
    }

    pub fn scheduler(&self) -> &FrameScheduler {
        &self.scheduler
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// 发送本帧输入到 relay（走可靠队列，丢包会重传）。
    pub fn submit_input(&mut self, frame: u64, commands: &[Command]) -> Result<(), NetError> {
        let packet = Packet::new(
            PacketKind::Input,
            self.player_id,
            0,
            frame,
            encode_commands(commands)?,
        );
        let (_, bytes) = self.sender.queue(packet)?;
        self.transport.send(&self.relay, &bytes)?;
        Ok(())
    }

    /// 上报本帧状态哈希（控制包，直接发送）。
    pub fn report_checksum(&mut self, frame: u64, hash: u64) -> Result<(), NetError> {
        self.checksums.record_local(frame, hash);
        let bytes = FrameChecksum::new(frame, hash)
            .packet(self.player_id, 0)
            .encode()?;
        self.transport.send(&self.relay, &bytes)?;
        Ok(())
    }

    /// 轮询网络，处理所有待收包；返回当前待应用帧数。
    /// 连续未收到权威帧时累计缺帧等待，用于确定性超时判定。
    pub fn poll(&mut self) -> Result<usize, NetError> {
        let mut received_frames = false;
        while let Some((from, bytes)) = self.transport.recv()? {
            let packet = Packet::decode(&bytes)?;
            if matches!(packet.kind, PacketKind::Frame | PacketKind::ResyncReplay) {
                received_frames = true;
            }
            self.handle_packet(packet, from)?;
        }
        if !received_frames {
            self.scheduler.note_wait();
        }
        Ok(self.ready.len())
    }

    fn handle_packet(&mut self, packet: Packet, from: Endpoint) -> Result<(), NetError> {
        match packet.kind {
            PacketKind::Ack => {
                let sequence = parse_ack(&packet.payload)?;
                self.sender.on_ack(sequence);
            }
            PacketKind::Frame => {
                let (is_new, sequence) = self.receiver.accept(packet.sequence);
                self.send_ack(sequence, &from)?;
                if is_new {
                    let commands = decode_commands(&packet.payload)?;
                    self.scheduler.offer([FrameCommands {
                        frame: packet.frame,
                        commands,
                    }]);
                    self.drain_scheduler();
                }
            }
            PacketKind::ResyncReplay => {
                let frames = parse_resync_replay(&packet.payload)?;
                let plan = self.reconnect.plan_replay(&frames);
                self.scheduler.offer(plan);
                self.drain_scheduler();
                self.reconnect.finish_catch_up();
            }
            PacketKind::Checksum => {
                let checksum = FrameChecksum::from_packet(&packet)?;
                // 不一致会以 Err(Desync) 显式返回，绝不静默继续。
                self.checksums.record_remote(checksum)?;
            }
            PacketKind::Welcome => {
                let (player_id, current_frame) = parse_welcome(&packet.payload)?;
                self.player_id = player_id;
                // 中途加入或 relay 已推进时，对齐帧基准，避免死等旧帧。
                self.scheduler.rebase(current_frame);
            }
            PacketKind::Reject => {
                return Err(NetError::Session(parse_reject(&packet.payload)?));
            }
            PacketKind::Hello | PacketKind::Input | PacketKind::ResyncRequest => {
                log::warn!(target: "w3_net", "unexpected uplink packet: kind={:?}", packet.kind);
            }
        }
        Ok(())
    }

    fn send_ack(&mut self, sequence: u32, destination: &Endpoint) -> Result<(), NetError> {
        let bytes = ack_packet(self.player_id, sequence, 0).encode()?;
        self.transport.send(destination, &bytes)?;
        Ok(())
    }

    /// 把调度器中连续就绪的帧全部移入待应用队列（不累计缺帧等待）。
    fn drain_scheduler(&mut self) {
        while let Some(frame) = self.scheduler.take_ready() {
            self.ready.push_back(frame);
        }
    }

    /// 取出下一帧待应用命令集，并推进输入游标。
    pub fn pop_ready(&mut self) -> Option<FrameCommands> {
        let frame = self.ready.pop_front()?;
        self.reconnect.commit(frame.frame);
        Some(frame)
    }

    /// 推进一个逻辑 tick：重传未确认包、累计缺帧等待；
    /// 返回是否超时（app 层据此发起重同步）。
    pub fn tick(&mut self) -> Result<bool, NetError> {
        self.tick += 1;
        for bytes in self.sender.tick(self.tick) {
            self.transport.send(&self.relay, &bytes)?;
        }
        Ok(self.scheduler.has_timed_out())
    }

    /// 发起重连请求（断线后由 app 层调用）。
    pub fn request_resync(&mut self) -> Result<(), NetError> {
        self.reconnect.disconnect();
        self.reconnect.begin_catch_up();
        let cursor = self.reconnect.cursor();
        let bytes = resync_request_packet(self.player_id, cursor).encode()?;
        self.transport.send(&self.relay, &bytes)?;
        Ok(())
    }
}
