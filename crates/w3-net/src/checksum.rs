//! 逐帧 checksum 交换、比对与首个差异字段上报。
//!
//! 不一致 MUST 被检测并显式返回 `NetError::Desync`，绝不静默继续；
//! `field` 通过 `w3_core::RtsMatch::first_difference()` 定位（spec Scenario）。

use std::collections::BTreeMap;

use w3_core::RtsMatch;

use crate::error::NetError;
use crate::packet::{Packet, PacketKind, parse_checksum, payload_checksum};

/// 单个客户端在某一帧的状态哈希。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameChecksum {
    pub frame: u64,
    pub hash: u64,
}

impl FrameChecksum {
    pub fn new(frame: u64, hash: u64) -> Self {
        Self { frame, hash }
    }

    pub fn packet(&self, sender: u8, sequence: u32) -> Packet {
        Packet::new(
            PacketKind::Checksum,
            sender,
            sequence,
            self.frame,
            payload_checksum(self.hash),
        )
    }

    pub fn from_packet(packet: &Packet) -> Result<Self, NetError> {
        if packet.kind != PacketKind::Checksum {
            return Err(NetError::Malformed(format!(
                "expected checksum packet, got {:?}",
                packet.kind
            )));
        }
        Ok(Self {
            frame: packet.frame,
            hash: parse_checksum(&packet.payload)?,
        })
    }
}

/// desync 报告；`field` 为 `first_difference()` 给出的首个差异字段名。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesyncReport {
    pub frame: u64,
    pub local_hash: u64,
    pub remote_hash: u64,
    pub field: Option<&'static str>,
}

impl DesyncReport {
    /// 用两个本地实例补上首个差异字段（验收测试 / 影子实例路径）。
    pub fn localize(mut self, local: &RtsMatch, reference: &RtsMatch) -> Self {
        self.field = first_difference_field(local, reference);
        self
    }
}

/// 直接包装 `RtsMatch::first_difference()`，供网络层定位差异字段。
pub fn first_difference_field(local: &RtsMatch, reference: &RtsMatch) -> Option<&'static str> {
    local.first_difference(reference)
}

/// 记录并比对双方逐帧哈希。
#[derive(Default)]
pub struct ChecksumExchange {
    local: BTreeMap<u64, u64>,
    remote: BTreeMap<u64, u64>,
    reports: Vec<DesyncReport>,
}

impl ChecksumExchange {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录本地某帧哈希。
    pub fn record_local(&mut self, frame: u64, hash: u64) {
        self.local.insert(frame, hash);
    }

    /// 记录对端哈希；若本地已有同帧且不一致，返回 `Err(NetError::Desync)`
    /// 并保存报告，绝不静默继续。
    pub fn record_remote(&mut self, checksum: FrameChecksum) -> Result<(), NetError> {
        self.remote.insert(checksum.frame, checksum.hash);
        if let Some(&local_hash) = self.local.get(&checksum.frame) {
            if local_hash != checksum.hash {
                let report = DesyncReport {
                    frame: checksum.frame,
                    local_hash,
                    remote_hash: checksum.hash,
                    field: None,
                };
                self.reports.push(report);
                return Err(NetError::Desync {
                    frame: checksum.frame,
                    local: local_hash,
                    remote: checksum.hash,
                    field: None,
                });
            }
        }
        Ok(())
    }

    /// 用两个本地实例为指定帧的报告补上差异字段。
    pub fn localize_report(&mut self, frame: u64, local: &RtsMatch, reference: &RtsMatch) {
        if let Some(report) = self.reports.iter_mut().find(|report| report.frame == frame) {
            report.field = first_difference_field(local, reference);
        }
    }

    pub fn reports(&self) -> &[DesyncReport] {
        &self.reports
    }

    pub fn has_desync(&self) -> bool {
        !self.reports.is_empty()
    }

    /// 双方都有记录但哈希不一致的帧（用于诊断）。
    pub fn mismatched_frames(&self) -> Vec<u64> {
        self.local
            .iter()
            .filter_map(|(&frame, &hash)| {
                self.remote
                    .get(&frame)
                    .filter(|&&remote| remote != hash)
                    .map(|_| frame)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::MatchConfig;

    #[test]
    fn checksum_packet_roundtrip() {
        let checksum = FrameChecksum::new(7, 0xdead_beef_cafe);
        let packet = checksum.packet(2, 42);
        assert_eq!(packet.kind, PacketKind::Checksum);
        assert_eq!(FrameChecksum::from_packet(&packet).unwrap(), checksum);
    }

    #[test]
    fn matching_checksums_are_accepted() {
        let mut exchange = ChecksumExchange::new();
        exchange.record_local(3, 1234);
        exchange.record_remote(FrameChecksum::new(3, 1234)).unwrap();
        assert!(!exchange.has_desync());
        assert!(exchange.mismatched_frames().is_empty());
    }

    #[test]
    fn mismatched_checksum_fails_loudly() {
        let mut exchange = ChecksumExchange::new();
        exchange.record_local(3, 1234);
        let result = exchange.record_remote(FrameChecksum::new(3, 9999));
        assert!(matches!(result, Err(NetError::Desync { frame: 3, .. })));
        assert!(exchange.has_desync());
        assert_eq!(exchange.mismatched_frames(), vec![3]);
        assert_eq!(
            exchange.reports()[0],
            DesyncReport {
                frame: 3,
                local_hash: 1234,
                remote_hash: 9999,
                field: None,
            }
        );
    }

    #[test]
    fn first_difference_field_is_reported() {
        let mut local = RtsMatch::create(MatchConfig::default());
        let reference = RtsMatch::create(MatchConfig::default());
        local.step();
        // 两个实例帧号不同，first_difference 应报告具体字段。
        let field = first_difference_field(&local, &reference);
        assert_eq!(field, Some("tick"));

        let mut exchange = ChecksumExchange::new();
        exchange.record_local(0, 1);
        let _ = exchange.record_remote(FrameChecksum::new(0, 2));
        exchange.localize_report(0, &local, &reference);
        assert_eq!(exchange.reports()[0].field, Some("tick"));
    }
}
