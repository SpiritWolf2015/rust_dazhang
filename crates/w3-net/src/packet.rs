//! 本项目私有 Event 线格式（W3N1）的帧头。
//!
//! 借鉴 `vera20k` 的 lockstep / reliable UDP 架构，但**不复用**其 RA2
//! `FRAMEINFO` / `FRAMESYNC` 包布局：magic、包头与校验和自定义，显式小端
//! 整数编码，与 ADR 0002 的确定性字节约定一致。
//! payload 层改用 Protocol Buffers（见 `payload.rs` / ADR 0005）；**帧头
//! 的 25B 布局与处理逻辑保持不变**，继续承载帧号、sequence、版本与校验。

use crate::error::NetError;

// payload 编解码 API 显式重导出，保持 `packet` 模块下的调用方 import 不变；
// 不导出 `pb` 生成类型与内部辅助，避免公开面扩散到帧头模块之外（ISP / SRP）。
pub use crate::payload::{
    FrameCommands, decode_commands, encode_commands, entity_from_raw, parse_ack, parse_checksum,
    parse_reject, parse_resync_replay, parse_resync_request, parse_welcome, payload_ack,
    payload_checksum, payload_reject, payload_resync_replay, payload_resync_request,
    payload_welcome,
};

/// 四字节 magic，明确与 RA2 原生包区分。
pub const MAGIC: [u8; 4] = *b"W3N1";
/// 协议版本；客户端与 relay 必须一致，不匹配明确拒绝。
/// 2 = payload 改用 protobuf（ADR 0005，BREAKING，schema 变更须继续递增）。
pub const PROTOCOL_VERSION: u8 = 2;
/// 固定包头长度：magic4 + version1 + kind1 + sender1 + seq4 + frame8 + len2 + sum4。
pub const HEADER_LEN: usize = 25;
const MAX_PAYLOAD: usize = u16::MAX as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketKind {
    Hello = 1,
    Welcome = 2,
    Input = 3,
    Frame = 4,
    Ack = 5,
    Checksum = 6,
    ResyncRequest = 7,
    ResyncReplay = 8,
    Reject = 9,
}

impl PacketKind {
    pub fn from_tag(tag: u8) -> Option<Self> {
        Some(match tag {
            1 => Self::Hello,
            2 => Self::Welcome,
            3 => Self::Input,
            4 => Self::Frame,
            5 => Self::Ack,
            6 => Self::Checksum,
            7 => Self::ResyncRequest,
            8 => Self::ResyncReplay,
            9 => Self::Reject,
            _ => return None,
        })
    }

    pub fn tag(self) -> u8 {
        self as u8
    }
}

/// 一个网络包；`frame` 是本包所属逻辑帧，`sequence` 供可靠层去重与确认。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub version: u8,
    pub kind: PacketKind,
    pub sender: u8,
    pub sequence: u32,
    pub frame: u64,
    pub payload: Vec<u8>,
}

/// FNV-1a 32 位校验和，与 `w3-core` 状态哈希同族但只用于线格式完整性。
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

impl Packet {
    pub fn new(kind: PacketKind, sender: u8, sequence: u32, frame: u64, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind,
            sender,
            sequence,
            frame,
            payload,
        }
    }

    /// 显式小端编码；校验和覆盖包头前 21 字节与整个（protobuf 编码后的）payload。
    pub fn encode(&self) -> Result<Vec<u8>, NetError> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(NetError::Malformed(format!(
                "payload too large: len={}",
                self.payload.len()
            )));
        }
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len());
        out.extend_from_slice(&MAGIC);
        out.push(self.version);
        out.push(self.kind.tag());
        out.push(self.sender);
        out.extend_from_slice(&self.sequence.to_le_bytes());
        out.extend_from_slice(&self.frame.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        let mut digest = Vec::with_capacity(21 + self.payload.len());
        digest.extend_from_slice(&out[..21]);
        digest.extend_from_slice(&self.payload);
        out.extend_from_slice(&fnv1a(&digest).to_le_bytes());
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    /// 解码并校验 magic / 版本 / 长度 / 校验和；任一失败显式报错。
    pub fn decode(bytes: &[u8]) -> Result<Self, NetError> {
        if bytes.len() < HEADER_LEN {
            return Err(NetError::Malformed(format!(
                "packet too short: len={}",
                bytes.len()
            )));
        }
        if bytes[..4] != MAGIC[..] {
            return Err(NetError::Malformed("bad magic".into()));
        }
        let version = bytes[4];
        if version != PROTOCOL_VERSION {
            return Err(NetError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                found: version,
            });
        }
        let kind = PacketKind::from_tag(bytes[5])
            .ok_or_else(|| NetError::Malformed(format!("unknown kind={}", bytes[5])))?;
        let sender = bytes[6];
        let sequence = read_u32(bytes, 7);
        let frame = read_u64(bytes, 11);
        let payload_len = u16::from_le_bytes([bytes[19], bytes[20]]) as usize;
        let expected_sum = read_u32(bytes, 21);
        if bytes.len() != HEADER_LEN + payload_len {
            return Err(NetError::Malformed(format!(
                "length mismatch: header={} actual={}",
                payload_len,
                bytes.len() - HEADER_LEN
            )));
        }
        let mut digest = Vec::with_capacity(21 + payload_len);
        digest.extend_from_slice(&bytes[..21]);
        digest.extend_from_slice(&bytes[HEADER_LEN..]);
        if fnv1a(&digest) != expected_sum {
            return Err(NetError::Malformed("checksum mismatch".into()));
        }
        Ok(Self {
            version,
            kind,
            sender,
            sequence,
            frame,
            payload: bytes[HEADER_LEN..].to_vec(),
        })
    }
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        bytes[at],
        bytes[at + 1],
        bytes[at + 2],
        bytes[at + 3],
        bytes[at + 4],
        bytes[at + 5],
        bytes[at + 6],
        bytes[at + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::{Command, CommandKind, EntityId, Frame, SimFixed};

    fn sample_commands() -> Vec<Command> {
        vec![
            Command::new(Frame::new(3), 1, 7, CommandKind::Noop),
            Command::new(
                Frame::new(3),
                2,
                8,
                CommandKind::Spawn {
                    x: SimFixed::from_raw(65536),
                    y: SimFixed::from_raw(-131072),
                    hp: 100,
                    speed: SimFixed::from_raw(196608),
                    attack_damage: 10,
                    attack_range: SimFixed::from_raw(655360),
                },
            ),
            Command::new(
                Frame::new(4),
                1,
                9,
                CommandKind::Move {
                    entity: EntityId::NONE,
                    x: SimFixed::from_raw(1),
                    y: SimFixed::from_raw(2),
                    group: 3,
                    append: true,
                },
            ),
            Command::new(
                Frame::new(4),
                2,
                10,
                CommandKind::Stop {
                    entity: entity_from_raw(42),
                },
            ),
            Command::new(
                Frame::new(5),
                1,
                11,
                CommandKind::Attack {
                    attacker: entity_from_raw(1),
                    target: entity_from_raw(2),
                },
            ),
            Command::new(
                Frame::new(5),
                2,
                12,
                CommandKind::SetObstacle {
                    id: 9,
                    x: -1,
                    y: 2,
                    w: 3,
                    h: 4,
                },
            ),
            Command::new(Frame::new(6), 1, 13, CommandKind::RemoveObstacle { id: 9 }),
        ]
    }

    #[test]
    fn packet_roundtrip_for_every_kind() {
        let kinds = [
            PacketKind::Hello,
            PacketKind::Welcome,
            PacketKind::Input,
            PacketKind::Frame,
            PacketKind::Ack,
            PacketKind::Checksum,
            PacketKind::ResyncRequest,
            PacketKind::ResyncReplay,
            PacketKind::Reject,
        ];
        for kind in kinds {
            let packet = Packet::new(kind, 3, 0x0102_0304, 77, vec![1, 2, 3, 250]);
            let bytes = packet.encode().unwrap();
            assert_eq!(Packet::decode(&bytes).unwrap(), packet, "kind={kind:?}");
        }
    }

    #[test]
    fn header_is_little_endian_and_magic_is_private() {
        let bytes = Packet::new(
            PacketKind::Ack,
            1,
            0x0403_0201,
            0x0807_0605_0403_0201,
            vec![],
        )
        .encode()
        .unwrap();
        assert_eq!(&bytes[..4], &MAGIC);
        assert_eq!(bytes[4], PROTOCOL_VERSION);
        assert_eq!(bytes[5], PacketKind::Ack.tag());
        assert_eq!(&bytes[7..11], &[1, 2, 3, 4]);
        assert_eq!(&bytes[11..19], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(bytes.len(), HEADER_LEN);
    }

    #[test]
    fn malformed_packets_are_rejected() {
        let good = Packet::new(PacketKind::Ack, 1, 1, 1, vec![9])
            .encode()
            .unwrap();
        assert!(matches!(
            Packet::decode(&good[..HEADER_LEN - 1]),
            Err(NetError::Malformed(_))
        ));

        let mut bad_magic = good.clone();
        bad_magic[0] ^= 0xff;
        assert!(matches!(
            Packet::decode(&bad_magic),
            Err(NetError::Malformed(_))
        ));

        let mut bad_version = good.clone();
        bad_version[4] = PROTOCOL_VERSION + 1;
        assert!(matches!(
            Packet::decode(&bad_version),
            Err(NetError::VersionMismatch { .. })
        ));

        let mut bad_sum = good.clone();
        bad_sum[HEADER_LEN] ^= 0xff;
        assert!(matches!(
            Packet::decode(&bad_sum),
            Err(NetError::Malformed(_))
        ));

        let mut bad_len = good.clone();
        bad_len.push(0);
        assert!(matches!(
            Packet::decode(&bad_len),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn commands_roundtrip_covers_every_kind() {
        let commands = sample_commands();
        let bytes = encode_commands(&commands).unwrap();
        let decoded = decode_commands(&bytes).unwrap();
        assert_eq!(decoded.len(), commands.len());
        for (a, b) in decoded.iter().zip(commands.iter()) {
            assert_eq!(a.execute_frame, b.execute_frame);
            assert_eq!(a.player_id, b.player_id);
            assert_eq!(a.sequence, b.sequence);
            assert_eq!(a.kind, b.kind);
        }
    }

    #[test]
    fn truncated_command_payload_is_rejected() {
        let bytes = encode_commands(&sample_commands()).unwrap();
        assert!(matches!(
            decode_commands(&bytes[..bytes.len() - 1]),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn resync_replay_roundtrip() {
        let frames = vec![
            FrameCommands {
                frame: 2,
                commands: sample_commands()[..2].to_vec(),
            },
            FrameCommands {
                frame: 3,
                commands: vec![],
            },
        ];
        let bytes = payload_resync_replay(&frames).unwrap();
        assert_eq!(parse_resync_replay(&bytes).unwrap(), frames);
    }

    #[test]
    fn scalar_payloads_roundtrip() {
        let (p, f) = parse_welcome(&payload_welcome(2, 99)).unwrap();
        assert_eq!((p, f), (2, 99));
        assert_eq!(parse_ack(&payload_ack(12345)).unwrap(), 12345);
        assert_eq!(
            parse_checksum(&payload_checksum(0xdead_beef)).unwrap(),
            0xdead_beef
        );
        assert_eq!(
            parse_resync_request(&payload_resync_request(1, 42)).unwrap(),
            (1, 42)
        );
        assert_eq!(
            parse_reject(&payload_reject("version mismatch")).unwrap(),
            "version mismatch"
        );
    }

    #[test]
    fn frame_header_unchanged_for_protobuf_payload() {
        let payload = encode_commands(&sample_commands()).unwrap();
        let bytes = Packet::new(PacketKind::Frame, 0, 7, 4, payload.clone())
            .encode()
            .unwrap();
        assert_eq!(&bytes[..4], &MAGIC);
        assert_eq!(
            bytes[4], 2,
            "PROTOCOL_VERSION bumped for breaking payload change"
        );
        assert_eq!(&bytes[7..11], &7u32.to_le_bytes());
        assert_eq!(&bytes[11..19], &4u64.to_le_bytes());
        assert_eq!(&bytes[19..21], &(payload.len() as u16).to_le_bytes());
        assert_eq!(&bytes[HEADER_LEN..], &payload[..]);
    }
}
