//! `W3N1` 包的 payload 层：Protocol Buffers 编解码（ADR 0005）。
//!
//! 自有 25B 帧头在 `packet.rs` 保持不变；本模块只负责 payload 的
//! schema-first 编码。canonical 约束见 `src/proto/w3net.proto` 顶部注释：
//! 无 `map`、无 `float` / `double`、字段编号只追加、`SimFixed` 用 `raw()`。
//! prost 解码默认丢弃 unknown fields，不存储也不回显，因此天然满足
//! 「禁止 unknown fields 透传」；编码使用规范 varint。

use prost::Message;
use w3_core::{Command, CommandKind, EntityId, Frame, SimFixed};

use crate::error::NetError;

/// `prost-build` 由 `src/proto/w3net.proto` 生成的消息类型。
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/w3net.rs"));
}

const MAX_PAYLOAD: usize = u16::MAX as usize;

/// 一个帧的完整命令集（relay 权威生成，客户端按序应用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCommands {
    pub frame: u64,
    pub commands: Vec<Command>,
}

/// 0 保留为「无实体」，与 `EntityId::NONE` 一致。
pub fn entity_from_raw(raw: u64) -> EntityId {
    EntityId::new(raw).unwrap_or(EntityId::NONE)
}

fn decode_error(error: prost::DecodeError) -> NetError {
    NetError::Malformed(format!("protobuf decode failed: {error}"))
}

fn overflow_error(field: &str, value: impl std::fmt::Display) -> NetError {
    NetError::Malformed(format!("{field} out of range: {value}"))
}

fn to_pb_kind(kind: &CommandKind) -> pb::command::Kind {
    match *kind {
        CommandKind::Noop => pb::command::Kind::Noop(pb::Noop {}),
        CommandKind::Spawn {
            x,
            y,
            hp,
            speed,
            attack_damage,
            attack_range,
        } => pb::command::Kind::Spawn(pb::Spawn {
            x: x.raw(),
            y: y.raw(),
            hp,
            speed: speed.raw(),
            attack_damage,
            attack_range: attack_range.raw(),
        }),
        CommandKind::Move {
            entity,
            x,
            y,
            group,
            append,
        } => pb::command::Kind::MoveCmd(pb::MoveCommand {
            entity: entity.value(),
            x: x.raw(),
            y: y.raw(),
            group,
            append,
        }),
        CommandKind::Stop { entity } => pb::command::Kind::Stop(pb::Stop {
            entity: entity.value(),
        }),
        CommandKind::Attack { attacker, target } => pb::command::Kind::Attack(pb::Attack {
            attacker: attacker.value(),
            target: target.value(),
        }),
        CommandKind::SetObstacle { id, x, y, w, h } => {
            pb::command::Kind::SetObstacle(pb::SetObstacle { id, x, y, w, h })
        }
        CommandKind::RemoveObstacle { id } => {
            pb::command::Kind::RemoveObstacle(pb::RemoveObstacle { id })
        }
    }
}

fn from_pb_kind(kind: pb::command::Kind) -> CommandKind {
    match kind {
        pb::command::Kind::Noop(_) => CommandKind::Noop,
        pb::command::Kind::Spawn(m) => CommandKind::Spawn {
            x: SimFixed::from_raw(m.x),
            y: SimFixed::from_raw(m.y),
            hp: m.hp,
            speed: SimFixed::from_raw(m.speed),
            attack_damage: m.attack_damage,
            attack_range: SimFixed::from_raw(m.attack_range),
        },
        pb::command::Kind::MoveCmd(m) => CommandKind::Move {
            entity: entity_from_raw(m.entity),
            x: SimFixed::from_raw(m.x),
            y: SimFixed::from_raw(m.y),
            group: m.group,
            append: m.append,
        },
        pb::command::Kind::Stop(m) => CommandKind::Stop {
            entity: entity_from_raw(m.entity),
        },
        pb::command::Kind::Attack(m) => CommandKind::Attack {
            attacker: entity_from_raw(m.attacker),
            target: entity_from_raw(m.target),
        },
        pb::command::Kind::SetObstacle(m) => CommandKind::SetObstacle {
            id: m.id,
            x: m.x,
            y: m.y,
            w: m.w,
            h: m.h,
        },
        pb::command::Kind::RemoveObstacle(m) => CommandKind::RemoveObstacle { id: m.id },
    }
}

fn to_pb_batch(commands: &[Command]) -> Result<pb::CommandBatch, NetError> {
    if commands.len() > MAX_PAYLOAD {
        return Err(NetError::Malformed(format!(
            "too many commands: count={}",
            commands.len()
        )));
    }
    let mut batch = pb::CommandBatch::default();
    for command in commands {
        batch.commands.push(pb::Command {
            execute_frame: command.execute_frame.value(),
            player_id: u32::from(command.player_id),
            sequence: command.sequence,
            kind: Some(to_pb_kind(&command.kind)),
        });
    }
    Ok(batch)
}

fn from_pb_batch(batch: pb::CommandBatch) -> Result<Vec<Command>, NetError> {
    let mut commands = Vec::with_capacity(batch.commands.len());
    for command in batch.commands {
        let player_id = u8::try_from(command.player_id)
            .map_err(|_| overflow_error("player_id", command.player_id))?;
        let kind = command
            .kind
            .ok_or_else(|| NetError::Malformed("missing command kind".into()))?;
        commands.push(Command::new(
            Frame::new(command.execute_frame),
            player_id,
            command.sequence,
            from_pb_kind(kind),
        ));
    }
    Ok(commands)
}

/// 编码一批命令；`arrival` 刻意不编码，由本地按稳定序提交时分配。
pub fn encode_commands(commands: &[Command]) -> Result<Vec<u8>, NetError> {
    Ok(to_pb_batch(commands)?.encode_to_vec())
}

/// 解码一批命令；unknown fields 由 prost 丢弃，绝不透传 / 回显。
pub fn decode_commands(bytes: &[u8]) -> Result<Vec<Command>, NetError> {
    from_pb_batch(pb::CommandBatch::decode(bytes).map_err(decode_error)?)
}

// ---- 各类控制包 payload ----

pub fn payload_welcome(player_id: u8, current_frame: u64) -> Vec<u8> {
    pb::Welcome {
        player_id: u32::from(player_id),
        current_frame,
    }
    .encode_to_vec()
}

pub fn parse_welcome(bytes: &[u8]) -> Result<(u8, u64), NetError> {
    let message = pb::Welcome::decode(bytes).map_err(decode_error)?;
    let player_id = u8::try_from(message.player_id)
        .map_err(|_| overflow_error("player_id", message.player_id))?;
    Ok((player_id, message.current_frame))
}

pub fn payload_ack(sequence: u32) -> Vec<u8> {
    pb::Ack { sequence }.encode_to_vec()
}

pub fn parse_ack(bytes: &[u8]) -> Result<u32, NetError> {
    Ok(pb::Ack::decode(bytes).map_err(decode_error)?.sequence)
}

pub fn payload_checksum(hash: u64) -> Vec<u8> {
    pb::Checksum { hash }.encode_to_vec()
}

pub fn parse_checksum(bytes: &[u8]) -> Result<u64, NetError> {
    Ok(pb::Checksum::decode(bytes).map_err(decode_error)?.hash)
}

/// `from_frame` 为重连补发的起始帧号（客户端下一个待应用帧）。
pub fn payload_resync_request(player_id: u8, from_frame: u64) -> Vec<u8> {
    pb::ResyncRequest {
        player_id: u32::from(player_id),
        from_frame,
    }
    .encode_to_vec()
}

pub fn parse_resync_request(bytes: &[u8]) -> Result<(u8, u64), NetError> {
    let message = pb::ResyncRequest::decode(bytes).map_err(decode_error)?;
    let player_id = u8::try_from(message.player_id)
        .map_err(|_| overflow_error("player_id", message.player_id))?;
    Ok((player_id, message.from_frame))
}

/// 补发多帧命令流（断线重连追帧用）。
pub fn payload_resync_replay(frames: &[FrameCommands]) -> Result<Vec<u8>, NetError> {
    if frames.len() > MAX_PAYLOAD {
        return Err(NetError::Malformed(format!(
            "too many replay frames: count={}",
            frames.len()
        )));
    }
    let mut replay = pb::ResyncReplay::default();
    for frame in frames {
        replay.frames.push(pb::FrameCommands {
            frame: frame.frame,
            commands: Some(to_pb_batch(&frame.commands)?),
        });
    }
    Ok(replay.encode_to_vec())
}

pub fn parse_resync_replay(bytes: &[u8]) -> Result<Vec<FrameCommands>, NetError> {
    let replay = pb::ResyncReplay::decode(bytes).map_err(decode_error)?;
    let mut frames = Vec::with_capacity(replay.frames.len());
    for frame in replay.frames {
        let batch = frame
            .commands
            .ok_or_else(|| NetError::Malformed("missing frame commands".into()))?;
        frames.push(FrameCommands {
            frame: frame.frame,
            commands: from_pb_batch(batch)?,
        });
    }
    Ok(frames)
}

pub fn payload_reject(reason: &str) -> Vec<u8> {
    pb::Reject {
        reason: reason.to_owned(),
    }
    .encode_to_vec()
}

pub fn parse_reject(bytes: &[u8]) -> Result<String, NetError> {
    Ok(pb::Reject::decode(bytes).map_err(decode_error)?.reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Command> {
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
                    entity: entity_from_raw(42),
                    x: SimFixed::from_raw(1),
                    y: SimFixed::from_raw(2),
                    group: 3,
                    append: true,
                },
            ),
        ]
    }

    #[test]
    fn encoding_is_canonical_and_roundtrips() {
        let commands = sample();
        let first = encode_commands(&commands).unwrap();
        let second = encode_commands(&commands).unwrap();
        assert_eq!(
            first, second,
            "same logical commands must produce same bytes"
        );
        let decoded = decode_commands(&first).unwrap();
        let reencoded = encode_commands(&decoded).unwrap();
        assert_eq!(first, reencoded, "decode -> encode must be canonical");
        assert_eq!(decoded.len(), commands.len());
        for (a, b) in decoded.iter().zip(commands.iter()) {
            assert_eq!(a.execute_frame, b.execute_frame);
            assert_eq!(a.player_id, b.player_id);
            assert_eq!(a.sequence, b.sequence);
            assert_eq!(a.kind, b.kind);
        }
    }

    #[test]
    fn unknown_fields_are_dropped_not_echoed() {
        let mut bytes = encode_commands(&sample()).unwrap();
        // 追加一个合法但未知字段（field 1000, varint 0）。
        bytes.extend_from_slice(&[0xc0, 0x3e, 0x00]);
        let decoded = decode_commands(&bytes).unwrap();
        let reencoded = encode_commands(&decoded).unwrap();
        assert!(
            !reencoded.ends_with(&[0xc0, 0x3e, 0x00]),
            "unknown fields must not be echoed"
        );
    }

    #[test]
    fn schema_forbids_map_and_floating_point() {
        let schema = include_str!("proto/w3net.proto");
        // 只看去掉行内注释后的代码，避免把「禁止 float」这类说明算成字段。
        for line in schema.lines() {
            let code = line.split("//").next().unwrap_or("");
            for forbidden in ["map<", "float", "double"] {
                assert!(
                    !code.contains(forbidden),
                    "schema must not declare {forbidden}: {line}"
                );
            }
        }
        assert!(schema.contains("int64"), "SimFixed must use int64 raw");
        assert!(
            schema.contains("reserved"),
            "field numbers must be reserved"
        );
    }

    #[test]
    fn truncated_payload_is_rejected() {
        let bytes = encode_commands(&sample()).unwrap();
        assert!(matches!(
            decode_commands(&bytes[..bytes.len() - 1]),
            Err(NetError::Malformed(_))
        ));
    }
}
