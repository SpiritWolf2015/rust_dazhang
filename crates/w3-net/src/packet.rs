//! 本项目私有 Event 线格式（W3N1）。
//!
//! 借鉴 `vera20k` 的 lockstep / reliable UDP 架构，但**不复用**其 RA2
//! `FRAMEINFO` / `FRAMESYNC` 包布局：magic、包头、命令编码与校验和全部
//! 自定义，显式小端整数编码，与 ADR 0002 的确定性字节约定一致。
//! 只编码玩家输入命令，绝不编码游戏状态。

use w3_core::{Command, CommandKind, EntityId, Frame, SimFixed};

use crate::error::NetError;

/// 四字节 magic，明确与 RA2 原生包区分。
pub const MAGIC: [u8; 4] = *b"W3N1";
/// 协议版本；客户端与 relay 必须一致，不匹配明确拒绝（design 的 Risks）。
pub const PROTOCOL_VERSION: u8 = 1;
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

    /// 显式小端编码；校验和覆盖包头前 21 字节与整个 payload。
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

// ---- payload 读写 ----

#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(value);
    }
    fn finish(self) -> Vec<u8> {
        self.0
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], NetError> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| NetError::Malformed("offset overflow".into()))?;
        if end > self.bytes.len() {
            return Err(NetError::Malformed(format!(
                "truncated payload: at={} need={} len={}",
                self.at,
                count,
                self.bytes.len()
            )));
        }
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, NetError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, NetError> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32, NetError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Result<u64, NetError> {
        let s = self.take(8)?;
        Ok(u64::from_le_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }
    fn i32(&mut self) -> Result<i32, NetError> {
        let s = self.take(4)?;
        Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i64(&mut self) -> Result<i64, NetError> {
        let s = self.take(8)?;
        Ok(i64::from_le_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }
    fn finish(self) -> Result<(), NetError> {
        if self.at != self.bytes.len() {
            return Err(NetError::Malformed(format!(
                "trailing bytes: consumed={} total={}",
                self.at,
                self.bytes.len()
            )));
        }
        Ok(())
    }
}

fn entity_from_raw(raw: u64) -> EntityId {
    // 0 保留为"无实体"，与 `EntityId::NONE` 一致。
    EntityId::new(raw).unwrap_or(EntityId::NONE)
}

/// 一个帧的完整命令集（relay 权威生成，客户端按序应用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCommands {
    pub frame: u64,
    pub commands: Vec<Command>,
}

/// 编码一批命令；`arrival` 刻意不编码，由本地按稳定序提交时分配。
pub fn encode_commands(commands: &[Command]) -> Result<Vec<u8>, NetError> {
    if commands.len() > MAX_PAYLOAD {
        return Err(NetError::Malformed(format!(
            "too many commands: count={}",
            commands.len()
        )));
    }
    let mut buf = Buf::default();
    buf.u16(commands.len() as u16);
    for command in commands {
        buf.u64(command.execute_frame.value());
        buf.u8(command.player_id);
        buf.u32(command.sequence);
        encode_kind(&command.kind, &mut buf);
    }
    Ok(buf.finish())
}

fn encode_kind(kind: &CommandKind, buf: &mut Buf) {
    match *kind {
        CommandKind::Noop => buf.u8(0),
        CommandKind::Spawn {
            x,
            y,
            hp,
            speed,
            attack_damage,
            attack_range,
        } => {
            buf.u8(1);
            buf.i64(x.raw());
            buf.i64(y.raw());
            buf.i64(hp);
            buf.i64(speed.raw());
            buf.i64(attack_damage);
            buf.i64(attack_range.raw());
        }
        CommandKind::Move {
            entity,
            x,
            y,
            group,
            append,
        } => {
            buf.u8(2);
            buf.u64(entity.value());
            buf.i64(x.raw());
            buf.i64(y.raw());
            buf.u32(group);
            buf.u8(u8::from(append));
        }
        CommandKind::Stop { entity } => {
            buf.u8(3);
            buf.u64(entity.value());
        }
        CommandKind::Attack { attacker, target } => {
            buf.u8(4);
            buf.u64(attacker.value());
            buf.u64(target.value());
        }
        CommandKind::SetObstacle { id, x, y, w, h } => {
            buf.u8(5);
            buf.u64(id);
            buf.i32(x);
            buf.i32(y);
            buf.i32(w);
            buf.i32(h);
        }
        CommandKind::RemoveObstacle { id } => {
            buf.u8(6);
            buf.u64(id);
        }
    }
}

/// 解码一批命令；所有字段显式小端、长度严格校验。
pub fn decode_commands(bytes: &[u8]) -> Result<Vec<Command>, NetError> {
    let mut reader = Reader::new(bytes);
    let count = reader.u16()? as usize;
    let mut commands = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = Frame::new(reader.u64()?);
        let player_id = reader.u8()?;
        let sequence = reader.u32()?;
        let kind = decode_kind(&mut reader)?;
        commands.push(Command::new(frame, player_id, sequence, kind));
    }
    reader.finish()?;
    Ok(commands)
}

fn decode_kind(reader: &mut Reader<'_>) -> Result<CommandKind, NetError> {
    let tag = reader.u8()?;
    Ok(match tag {
        0 => CommandKind::Noop,
        1 => CommandKind::Spawn {
            x: SimFixed::from_raw(reader.i64()?),
            y: SimFixed::from_raw(reader.i64()?),
            hp: reader.i64()?,
            speed: SimFixed::from_raw(reader.i64()?),
            attack_damage: reader.i64()?,
            attack_range: SimFixed::from_raw(reader.i64()?),
        },
        2 => CommandKind::Move {
            entity: entity_from_raw(reader.u64()?),
            x: SimFixed::from_raw(reader.i64()?),
            y: SimFixed::from_raw(reader.i64()?),
            group: reader.u32()?,
            append: reader.u8()? != 0,
        },
        3 => CommandKind::Stop {
            entity: entity_from_raw(reader.u64()?),
        },
        4 => CommandKind::Attack {
            attacker: entity_from_raw(reader.u64()?),
            target: entity_from_raw(reader.u64()?),
        },
        5 => CommandKind::SetObstacle {
            id: reader.u64()?,
            x: reader.i32()?,
            y: reader.i32()?,
            w: reader.i32()?,
            h: reader.i32()?,
        },
        6 => CommandKind::RemoveObstacle { id: reader.u64()? },
        other => return Err(NetError::Malformed(format!("unknown command tag={other}"))),
    })
}

// ---- 各类 payload 构造 / 解析 ----

pub fn payload_welcome(player_id: u8, current_frame: u64) -> Vec<u8> {
    let mut buf = Buf::default();
    buf.u8(player_id);
    buf.u64(current_frame);
    buf.finish()
}

pub fn parse_welcome(bytes: &[u8]) -> Result<(u8, u64), NetError> {
    let mut reader = Reader::new(bytes);
    let player_id = reader.u8()?;
    let current_frame = reader.u64()?;
    reader.finish()?;
    Ok((player_id, current_frame))
}

pub fn payload_ack(sequence: u32) -> Vec<u8> {
    let mut buf = Buf::default();
    buf.u32(sequence);
    buf.finish()
}

pub fn parse_ack(bytes: &[u8]) -> Result<u32, NetError> {
    let mut reader = Reader::new(bytes);
    let sequence = reader.u32()?;
    reader.finish()?;
    Ok(sequence)
}

pub fn payload_checksum(hash: u64) -> Vec<u8> {
    hash.to_le_bytes().to_vec()
}

pub fn parse_checksum(bytes: &[u8]) -> Result<u64, NetError> {
    let mut reader = Reader::new(bytes);
    let hash = reader.u64()?;
    reader.finish()?;
    Ok(hash)
}

/// `from_frame` 为重连补发的起始帧号（客户端下一个待应用帧）。
pub fn payload_resync_request(player_id: u8, from_frame: u64) -> Vec<u8> {
    let mut buf = Buf::default();
    buf.u8(player_id);
    buf.u64(from_frame);
    buf.finish()
}

pub fn parse_resync_request(bytes: &[u8]) -> Result<(u8, u64), NetError> {
    let mut reader = Reader::new(bytes);
    let player_id = reader.u8()?;
    let from_frame = reader.u64()?;
    reader.finish()?;
    Ok((player_id, from_frame))
}

/// 补发多帧命令流（断线重连追帧用）。
pub fn payload_resync_replay(frames: &[FrameCommands]) -> Result<Vec<u8>, NetError> {
    if frames.len() > MAX_PAYLOAD {
        return Err(NetError::Malformed(format!(
            "too many replay frames: count={}",
            frames.len()
        )));
    }
    let mut buf = Buf::default();
    buf.u16(frames.len() as u16);
    for frame in frames {
        buf.u64(frame.frame);
        let commands = encode_commands(&frame.commands)?;
        buf.u16(commands.len() as u16);
        buf.bytes(&commands);
    }
    Ok(buf.finish())
}

pub fn parse_resync_replay(bytes: &[u8]) -> Result<Vec<FrameCommands>, NetError> {
    let mut reader = Reader::new(bytes);
    let count = reader.u16()? as usize;
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        let frame = reader.u64()?;
        let len = reader.u16()? as usize;
        let commands = decode_commands(reader.take(len)?)?;
        frames.push(FrameCommands { frame, commands });
    }
    reader.finish()?;
    Ok(frames)
}

pub fn payload_reject(reason: &str) -> Vec<u8> {
    reason.as_bytes().to_vec()
}

pub fn parse_reject(bytes: &[u8]) -> Result<String, NetError> {
    String::from_utf8(bytes.to_vec())
        .map_err(|e| NetError::Malformed(format!("reject reason not utf8: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
