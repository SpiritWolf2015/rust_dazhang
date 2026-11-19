//! `W3N1` 包的 payload 层：Protocol Buffers 编解码（ADR 0005）。
//!
//! 自有 25B 帧头在 `packet.rs` 保持不变；本模块只负责 payload 的
//! schema-first 编码。canonical 约束见 `src/proto/w3net.proto` 顶部注释：
//! 无 `map`、无 `float` / `double`、字段编号只追加、`SimFixed` 用 `raw()`。
//! prost 解码默认丢弃 unknown fields，不存储也不回显，因此天然满足
//! 「禁止 unknown fields 透传」；编码使用规范 varint。
//!
//! 自 `PROTOCOL_VERSION = 3` 起，顶层 `CommandBatch` 同时承载普通 `Command`
//! 与玩法 `GameplayCommand`（独立 `GameplayCommandBatch` 子批）。两类命令
//! 是两条独立队列、各自独立 `arrival`；`arrival` 一律不编码，由各端提交时
//! 按稳定序本地分配。Input / Frame / ResyncReplay 三条路径共用这一批结构。
//!
//! `PROTOCOL_VERSION = 4` 在玩法 oneof 只追加 `ReviveHero` / `CancelRevive`
//! 两个变体（tag 33 / 34，跳过 `reserved 14 to 32`）；帧编排、`arrival` 语义
//! 与「恰好一次 `step()`」推进点一律不变（ADR 0005 §3）。

use prost::Message;
use w3_core::{
    Command, CommandKind, EffectDef, EffectKind, EntityId, Frame, GameplayCommand,
    GameplayCommandKind, SimFixed,
};

use crate::error::NetError;

/// `prost-build` 由 `src/proto/w3net.proto` 生成的消息类型。
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/w3net.rs"));
}

const MAX_PAYLOAD: usize = u16::MAX as usize;

/// 一个帧的完整命令集（relay 权威生成，客户端按序应用）。
/// `commands` 与 `gameplay` 是两条独立队列，`arrival` 均不编码。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCommands {
    pub frame: u64,
    pub commands: Vec<Command>,
    pub gameplay: Vec<GameplayCommand>,
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

/// 校验两类命令条数上限；编码与解码两侧共用，超限显式拒绝。
fn check_batch_len(commands: usize, gameplay: usize) -> Result<(), NetError> {
    let total = commands.saturating_add(gameplay);
    if commands > MAX_PAYLOAD || gameplay > MAX_PAYLOAD || total > MAX_PAYLOAD {
        return Err(NetError::Malformed(format!(
            "command batch too large: commands={commands} gameplay={gameplay} total={total} max={MAX_PAYLOAD}"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// EffectKind 稳定码（显式、平台无关的固定值，与 w3-core 状态哈希编码一致）
// ---------------------------------------------------------------------------
// 稳定码映射表：
//   0 -> EffectKind::Damage
//   1 -> EffectKind::Heal
//   2 -> EffectKind::BuffStat
//   3 -> EffectKind::Stun
// 线格式用 uint32 承载；任何未知码一律返回 NetError::Malformed，不猜测、
// 不静默降级、不按枚举索引隐式转换。

const EFFECT_KIND_DAMAGE: u32 = 0;
const EFFECT_KIND_HEAL: u32 = 1;
const EFFECT_KIND_BUFF_STAT: u32 = 2;
const EFFECT_KIND_STUN: u32 = 3;

fn to_effect_kind_code(kind: EffectKind) -> u32 {
    match kind {
        EffectKind::Damage => EFFECT_KIND_DAMAGE,
        EffectKind::Heal => EFFECT_KIND_HEAL,
        EffectKind::BuffStat => EFFECT_KIND_BUFF_STAT,
        EffectKind::Stun => EFFECT_KIND_STUN,
    }
}

fn from_effect_kind_code(code: u32) -> Result<EffectKind, NetError> {
    match code {
        EFFECT_KIND_DAMAGE => Ok(EffectKind::Damage),
        EFFECT_KIND_HEAL => Ok(EffectKind::Heal),
        EFFECT_KIND_BUFF_STAT => Ok(EffectKind::BuffStat),
        EFFECT_KIND_STUN => Ok(EffectKind::Stun),
        _ => Err(NetError::Malformed(format!(
            "unknown effect kind: code={code}"
        ))),
    }
}

fn to_pb_effect(effect: &EffectDef) -> pb::EffectDef {
    pb::EffectDef {
        effect_kind: to_effect_kind_code(effect.kind),
        amount: effect.amount,
        duration_frames: effect.duration_frames,
    }
}

fn from_pb_effect(effect: pb::EffectDef) -> Result<EffectDef, NetError> {
    Ok(EffectDef {
        kind: from_effect_kind_code(effect.effect_kind)?,
        amount: effect.amount,
        duration_frames: effect.duration_frames,
    })
}

// ---------------------------------------------------------------------------
// 普通命令：to_pb_kind / from_pb_kind 保持原样，不改 CommandKind
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 玩法命令：to_pb_gameplay_kind / from_pb_gameplay_kind 双向穷尽映射（12 变体）
// ---------------------------------------------------------------------------

/// 把 12 类 `GameplayCommandKind` 映射为 protobuf `GameplayCommand.kind` oneof；穷尽匹配。
pub fn to_pb_gameplay_kind(kind: &GameplayCommandKind) -> pb::gameplay_command::Kind {
    match *kind {
        GameplayCommandKind::Cast {
            caster,
            ability,
            target,
        } => pb::gameplay_command::Kind::Cast(pb::GameplayCast {
            caster: caster.value(),
            ability,
            target: target.value(),
        }),
        GameplayCommandKind::Train {
            producer,
            unit_def,
            is_building,
        } => pb::gameplay_command::Kind::Train(pb::GameplayTrain {
            producer: producer.value(),
            unit_def,
            is_building,
        }),
        GameplayCommandKind::CancelTrain { producer, index } => {
            pb::gameplay_command::Kind::CancelTrain(pb::GameplayCancelTrain {
                producer: producer.value(),
                index,
            })
        }
        GameplayCommandKind::Research { upgrade } => {
            pb::gameplay_command::Kind::Research(pb::GameplayResearch { upgrade })
        }
        GameplayCommandKind::BuyItem { unit, item } => {
            pb::gameplay_command::Kind::BuyItem(pb::GameplayBuyItem {
                unit: unit.value(),
                item,
            })
        }
        GameplayCommandKind::Gather { unit } => {
            pb::gameplay_command::Kind::Gather(pb::GameplayGather { unit: unit.value() })
        }
        GameplayCommandKind::RegisterBehavior { id, effect } => {
            pb::gameplay_command::Kind::RegisterBehavior(pb::GameplayRegisterBehavior {
                id,
                effect: Some(to_pb_effect(&effect)),
            })
        }
        GameplayCommandKind::SpawnWave { id, period } => {
            pb::gameplay_command::Kind::SpawnWave(pb::GameplaySpawnWave { id, period })
        }
        GameplayCommandKind::SetAi { enabled } => {
            pb::gameplay_command::Kind::SetAi(pb::GameplaySetAi { enabled })
        }
        GameplayCommandKind::Leave { player } => {
            pb::gameplay_command::Kind::Leave(pb::GameplayLeave {
                player: u32::from(player),
            })
        }
        GameplayCommandKind::ReviveHero {
            hero,
            altar,
            altar_def,
        } => pb::gameplay_command::Kind::ReviveHero(pb::GameplayReviveHero {
            hero: hero.value(),
            altar: altar.value(),
            altar_def,
        }),
        GameplayCommandKind::CancelRevive { hero } => {
            pb::gameplay_command::Kind::CancelRevive(pb::GameplayCancelRevive {
                hero: hero.value(),
            })
        }
    }
}

/// 把 protobuf `GameplayCommand.kind` oneof 还原为 12 类 `GameplayCommandKind`；穷尽匹配。
/// 缺 `kind` / 缺 `effect` / 未知效果码 / 数值溢出一律显式返回 `NetError`，不猜测、不回退。
pub fn from_pb_gameplay_kind(
    kind: pb::gameplay_command::Kind,
) -> Result<GameplayCommandKind, NetError> {
    Ok(match kind {
        pb::gameplay_command::Kind::Cast(m) => GameplayCommandKind::Cast {
            caster: entity_from_raw(m.caster),
            ability: m.ability,
            target: entity_from_raw(m.target),
        },
        pb::gameplay_command::Kind::Train(m) => GameplayCommandKind::Train {
            producer: entity_from_raw(m.producer),
            unit_def: m.unit_def,
            is_building: m.is_building,
        },
        pb::gameplay_command::Kind::CancelTrain(m) => GameplayCommandKind::CancelTrain {
            producer: entity_from_raw(m.producer),
            index: m.index,
        },
        pb::gameplay_command::Kind::Research(m) => {
            GameplayCommandKind::Research { upgrade: m.upgrade }
        }
        pb::gameplay_command::Kind::BuyItem(m) => GameplayCommandKind::BuyItem {
            unit: entity_from_raw(m.unit),
            item: m.item,
        },
        pb::gameplay_command::Kind::Gather(m) => GameplayCommandKind::Gather {
            unit: entity_from_raw(m.unit),
        },
        pb::gameplay_command::Kind::RegisterBehavior(m) => {
            let effect = m.effect.ok_or_else(|| {
                NetError::Malformed("missing effect: kind=RegisterBehavior".into())
            })?;
            GameplayCommandKind::RegisterBehavior {
                id: m.id,
                effect: from_pb_effect(effect)?,
            }
        }
        pb::gameplay_command::Kind::SpawnWave(m) => GameplayCommandKind::SpawnWave {
            id: m.id,
            period: m.period,
        },
        pb::gameplay_command::Kind::SetAi(m) => GameplayCommandKind::SetAi { enabled: m.enabled },
        pb::gameplay_command::Kind::Leave(m) => GameplayCommandKind::Leave {
            player: u8::try_from(m.player).map_err(|_| overflow_error("player", m.player))?,
        },
        // uint64 -> EntityId(u64) / uint32 -> DefId(u32) 均为无损定宽映射，
        // 无收窄转换，因此不存在数值溢出分支；缺字段由 oneof 的 Option 拦截。
        pb::gameplay_command::Kind::ReviveHero(m) => GameplayCommandKind::ReviveHero {
            hero: entity_from_raw(m.hero),
            altar: entity_from_raw(m.altar),
            altar_def: m.altar_def,
        },
        pb::gameplay_command::Kind::CancelRevive(m) => GameplayCommandKind::CancelRevive {
            hero: entity_from_raw(m.hero),
        },
    })
}

fn to_pb_gameplay_command(command: &GameplayCommand) -> pb::GameplayCommand {
    pb::GameplayCommand {
        execute_frame: command.execute_frame.value(),
        player_id: u32::from(command.player_id),
        sequence: command.sequence,
        kind: Some(to_pb_gameplay_kind(&command.kind)),
    }
}

fn from_pb_gameplay_command(command: pb::GameplayCommand) -> Result<GameplayCommand, NetError> {
    let player_id = u8::try_from(command.player_id)
        .map_err(|_| overflow_error("player_id", command.player_id))?;
    let kind = command
        .kind
        .ok_or_else(|| NetError::Malformed("missing gameplay command kind".into()))?;
    Ok(GameplayCommand::new(
        Frame::new(command.execute_frame),
        player_id,
        command.sequence,
        from_pb_gameplay_kind(kind)?,
    ))
}

/// 把玩法命令批映射为 `GameplayCommandBatch`；条数超限显式拒绝。
pub fn to_pb_gameplay_batch(
    gameplay: &[GameplayCommand],
) -> Result<pb::GameplayCommandBatch, NetError> {
    check_batch_len(0, gameplay.len())?;
    let mut batch = pb::GameplayCommandBatch::default();
    for command in gameplay {
        batch.commands.push(to_pb_gameplay_command(command));
    }
    Ok(batch)
}

/// 把 `GameplayCommandBatch` 还原为玩法命令批；缺 kind / 溢出显式拒绝。
pub fn from_pb_gameplay_batch(
    batch: pb::GameplayCommandBatch,
) -> Result<Vec<GameplayCommand>, NetError> {
    check_batch_len(0, batch.commands.len())?;
    let mut gameplay = Vec::with_capacity(batch.commands.len());
    for command in batch.commands {
        gameplay.push(from_pb_gameplay_command(command)?);
    }
    Ok(gameplay)
}

fn to_pb_batch(
    commands: &[Command],
    gameplay: &[GameplayCommand],
) -> Result<pb::CommandBatch, NetError> {
    check_batch_len(commands.len(), gameplay.len())?;
    let mut batch = pb::CommandBatch::default();
    for command in commands {
        batch.commands.push(pb::Command {
            execute_frame: command.execute_frame.value(),
            player_id: u32::from(command.player_id),
            sequence: command.sequence,
            kind: Some(to_pb_kind(&command.kind)),
        });
    }
    // 玩法命令为空时不写该字段，保持 canonical 编码（解码→再编码逐字节一致）。
    if !gameplay.is_empty() {
        batch.gameplay = Some(to_pb_gameplay_batch(gameplay)?);
    }
    Ok(batch)
}

fn from_pb_batch(
    batch: pb::CommandBatch,
) -> Result<(Vec<Command>, Vec<GameplayCommand>), NetError> {
    let gameplay_len = batch
        .gameplay
        .as_ref()
        .map_or(0, |gameplay| gameplay.commands.len());
    check_batch_len(batch.commands.len(), gameplay_len)?;
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
    let gameplay = match batch.gameplay {
        Some(gameplay) => from_pb_gameplay_batch(gameplay)?,
        None => Vec::new(),
    };
    Ok((commands, gameplay))
}

/// 编码一批命令（普通 + 玩法）；`arrival` 刻意不编码，由本地按稳定序提交时分配。
pub fn encode_commands(
    commands: &[Command],
    gameplay: &[GameplayCommand],
) -> Result<Vec<u8>, NetError> {
    Ok(to_pb_batch(commands, gameplay)?.encode_to_vec())
}

/// 解码一批命令，返回 `(普通命令, 玩法命令)`；unknown fields 由 prost 丢弃，绝不透传 / 回显。
pub fn decode_commands(bytes: &[u8]) -> Result<(Vec<Command>, Vec<GameplayCommand>), NetError> {
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

/// 补发多帧命令流（断线重连追帧用）；每帧同时补发普通与玩法两类命令，
/// 顺序与原始权威帧批一致。两类命令同装在 `FrameCommands.commands` 这个
/// `CommandBatch` 内，因此 `FrameCommands` / `ResyncReplay` 无需新增字段。
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
            commands: Some(to_pb_batch(&frame.commands, &frame.gameplay)?),
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
        let (commands, gameplay) = from_pb_batch(batch)?;
        frames.push(FrameCommands {
            frame: frame.frame,
            commands,
            gameplay,
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

    /// 覆盖全部 12 类玩法变体的样本（含 1 种 EffectKind 稳定码）；
    /// 其余 3 种 EffectKind 稳定码由专门用例覆盖。
    fn sample_gameplay() -> Vec<GameplayCommand> {
        let caster = entity_from_raw(1);
        let target = entity_from_raw(2);
        let producer = entity_from_raw(3);
        vec![
            GameplayCommand::new(
                Frame::new(3),
                1,
                1,
                GameplayCommandKind::Cast {
                    caster,
                    ability: 10,
                    target,
                },
            ),
            GameplayCommand::new(
                Frame::new(3),
                1,
                2,
                GameplayCommandKind::Train {
                    producer,
                    unit_def: 11,
                    is_building: true,
                },
            ),
            GameplayCommand::new(
                Frame::new(3),
                1,
                3,
                GameplayCommandKind::CancelTrain { producer, index: 4 },
            ),
            GameplayCommand::new(
                Frame::new(4),
                2,
                1,
                GameplayCommandKind::Research { upgrade: 12 },
            ),
            GameplayCommand::new(
                Frame::new(4),
                2,
                2,
                GameplayCommandKind::BuyItem {
                    unit: producer,
                    item: 13,
                },
            ),
            GameplayCommand::new(
                Frame::new(4),
                2,
                3,
                GameplayCommandKind::Gather { unit: producer },
            ),
            GameplayCommand::new(
                Frame::new(5),
                1,
                1,
                GameplayCommandKind::RegisterBehavior {
                    id: 14,
                    effect: EffectDef {
                        kind: EffectKind::Damage,
                        amount: -7,
                        duration_frames: 30,
                    },
                },
            ),
            GameplayCommand::new(
                Frame::new(5),
                1,
                2,
                GameplayCommandKind::SpawnWave { id: 15, period: 60 },
            ),
            GameplayCommand::new(
                Frame::new(5),
                2,
                1,
                GameplayCommandKind::SetAi { enabled: true },
            ),
            GameplayCommand::new(
                Frame::new(6),
                2,
                2,
                GameplayCommandKind::Leave { player: 2 },
            ),
            GameplayCommand::new(
                Frame::new(7),
                1,
                1,
                GameplayCommandKind::ReviveHero {
                    hero: entity_from_raw(20),
                    altar: entity_from_raw(21),
                    altar_def: 22,
                },
            ),
            GameplayCommand::new(
                Frame::new(7),
                2,
                2,
                GameplayCommandKind::CancelRevive {
                    hero: entity_from_raw(20),
                },
            ),
        ]
    }

    fn assert_gameplay_eq(decoded: &[GameplayCommand], expected: &[GameplayCommand]) {
        assert_eq!(decoded.len(), expected.len());
        for (a, b) in decoded.iter().zip(expected.iter()) {
            // arrival 不编码、不参与比较。
            assert_eq!(a.execute_frame, b.execute_frame);
            assert_eq!(a.player_id, b.player_id);
            assert_eq!(a.sequence, b.sequence);
            assert_eq!(a.kind, b.kind);
        }
    }

    #[test]
    fn encoding_is_canonical_and_roundtrips() {
        let commands = sample();
        let first = encode_commands(&commands, &[]).unwrap();
        let second = encode_commands(&commands, &[]).unwrap();
        assert_eq!(
            first, second,
            "same logical commands must produce same bytes"
        );
        let (decoded, gameplay) = decode_commands(&first).unwrap();
        assert!(gameplay.is_empty());
        assert_eq!(
            encode_commands(&decoded, &gameplay).unwrap(),
            first,
            "decode -> encode must be canonical"
        );
        assert_eq!(decoded.len(), commands.len());
        for (a, b) in decoded.iter().zip(commands.iter()) {
            assert_eq!(a.execute_frame, b.execute_frame);
            assert_eq!(a.player_id, b.player_id);
            assert_eq!(a.sequence, b.sequence);
            assert_eq!(a.kind, b.kind);
        }
    }

    #[test]
    fn both_command_types_encode_is_canonical() {
        let commands = sample();
        let gameplay = sample_gameplay();
        let first = encode_commands(&commands, &gameplay).unwrap();
        let second = encode_commands(&commands, &gameplay).unwrap();
        assert_eq!(first, second, "two encodes must be byte-identical");
        let (decoded_commands, decoded_gameplay) = decode_commands(&first).unwrap();
        assert_eq!(
            encode_commands(&decoded_commands, &decoded_gameplay).unwrap(),
            first,
            "decode -> encode must be canonical for both types"
        );
        assert_eq!(decoded_commands.len(), commands.len());
        assert_gameplay_eq(&decoded_gameplay, &gameplay);
        // 空玩法批不写该字段，与无玩法批的编码结果逐字节一致。
        assert_eq!(
            encode_commands(&commands, &[]).unwrap(),
            encode_commands(&commands, &[]).unwrap()
        );
    }

    #[test]
    fn all_twelve_gameplay_kinds_roundtrip() {
        let gameplay = sample_gameplay();
        assert_eq!(gameplay.len(), 12, "sample must cover every gameplay kind");
        let bytes = encode_commands(&[], &gameplay).unwrap();
        let (commands, decoded) = decode_commands(&bytes).unwrap();
        assert!(commands.is_empty());
        assert_gameplay_eq(&decoded, &gameplay);
        // 逐个变体单独往返，避免批量编码掩盖某个变体的歧义映射。
        for command in &gameplay {
            let one = encode_commands(&[], std::slice::from_ref(command)).unwrap();
            let (_, one_decoded) = decode_commands(&one).unwrap();
            assert_gameplay_eq(&one_decoded, std::slice::from_ref(command));
        }
    }

    #[test]
    fn all_effect_kind_codes_roundtrip_and_unknown_is_rejected() {
        for kind in [
            EffectKind::Damage,
            EffectKind::Heal,
            EffectKind::BuffStat,
            EffectKind::Stun,
        ] {
            let effect = EffectDef {
                kind,
                amount: 12345,
                duration_frames: 9,
            };
            let command = GameplayCommand::new(
                Frame::new(1),
                1,
                1,
                GameplayCommandKind::RegisterBehavior { id: 1, effect },
            );
            let bytes = encode_commands(&[], &[command]).unwrap();
            let (_, decoded) = decode_commands(&bytes).unwrap();
            assert_eq!(decoded[0].kind, command.kind);
        }
        assert!(matches!(
            from_effect_kind_code(4),
            Err(NetError::Malformed(_))
        ));
        assert!(matches!(
            from_effect_kind_code(u32::MAX),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn unknown_effect_kind_code_is_rejected() {
        let bad = pb::CommandBatch {
            commands: Vec::new(),
            gameplay: Some(pb::GameplayCommandBatch {
                commands: vec![pb::GameplayCommand {
                    execute_frame: 1,
                    player_id: 1,
                    sequence: 1,
                    kind: Some(pb::gameplay_command::Kind::RegisterBehavior(
                        pb::GameplayRegisterBehavior {
                            id: 1,
                            effect: Some(pb::EffectDef {
                                effect_kind: 99,
                                amount: 1,
                                duration_frames: 2,
                            }),
                        },
                    )),
                }],
            }),
        };
        assert!(matches!(
            decode_commands(&bad.encode_to_vec()),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn missing_gameplay_kind_and_effect_are_rejected() {
        let missing_kind = pb::CommandBatch {
            commands: Vec::new(),
            gameplay: Some(pb::GameplayCommandBatch {
                commands: vec![pb::GameplayCommand {
                    execute_frame: 1,
                    player_id: 1,
                    sequence: 1,
                    kind: None,
                }],
            }),
        };
        assert!(matches!(
            decode_commands(&missing_kind.encode_to_vec()),
            Err(NetError::Malformed(_))
        ));

        let missing_effect = pb::CommandBatch {
            commands: Vec::new(),
            gameplay: Some(pb::GameplayCommandBatch {
                commands: vec![pb::GameplayCommand {
                    execute_frame: 1,
                    player_id: 1,
                    sequence: 1,
                    kind: Some(pb::gameplay_command::Kind::RegisterBehavior(
                        pb::GameplayRegisterBehavior {
                            id: 1,
                            effect: None,
                        },
                    )),
                }],
            }),
        };
        assert!(matches!(
            decode_commands(&missing_effect.encode_to_vec()),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn gameplay_numeric_overflow_is_rejected() {
        let overflow_player = pb::CommandBatch {
            commands: Vec::new(),
            gameplay: Some(pb::GameplayCommandBatch {
                commands: vec![pb::GameplayCommand {
                    execute_frame: 1,
                    player_id: 300,
                    sequence: 1,
                    kind: Some(pb::gameplay_command::Kind::SetAi(pb::GameplaySetAi {
                        enabled: true,
                    })),
                }],
            }),
        };
        assert!(matches!(
            decode_commands(&overflow_player.encode_to_vec()),
            Err(NetError::Malformed(_))
        ));

        let overflow_leave = pb::CommandBatch {
            commands: Vec::new(),
            gameplay: Some(pb::GameplayCommandBatch {
                commands: vec![pb::GameplayCommand {
                    execute_frame: 1,
                    player_id: 1,
                    sequence: 1,
                    kind: Some(pb::gameplay_command::Kind::Leave(pb::GameplayLeave {
                        player: 300,
                    })),
                }],
            }),
        };
        assert!(matches!(
            decode_commands(&overflow_leave.encode_to_vec()),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn oversized_batches_are_rejected() {
        let ordinary = Command::new(Frame::new(0), 1, 1, CommandKind::Noop);
        let gameplay = GameplayCommand::new(
            Frame::new(0),
            1,
            1,
            GameplayCommandKind::SetAi { enabled: true },
        );
        let too_many = MAX_PAYLOAD + 1;
        assert!(matches!(
            encode_commands(&vec![ordinary; too_many], &[]),
            Err(NetError::Malformed(_))
        ));
        assert!(matches!(
            encode_commands(&[], &vec![gameplay; too_many]),
            Err(NetError::Malformed(_))
        ));
        assert!(matches!(
            encode_commands(&vec![ordinary; MAX_PAYLOAD], &vec![gameplay; MAX_PAYLOAD]),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn unknown_fields_are_dropped_not_echoed() {
        let mut bytes = encode_commands(&sample(), &sample_gameplay()).unwrap();
        // 追加一个合法但未知字段（field 1000, varint 0）。
        bytes.extend_from_slice(&[0xc0, 0x3e, 0x00]);
        let (commands, gameplay) = decode_commands(&bytes).unwrap();
        let reencoded = encode_commands(&commands, &gameplay).unwrap();
        assert!(
            !reencoded.ends_with(&[0xc0, 0x3e, 0x00]),
            "unknown fields must not be echoed"
        );
        assert_eq!(
            reencoded,
            encode_commands(&sample(), &sample_gameplay()).unwrap()
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
        assert!(
            schema.contains("int64"),
            "SimFixed / amount must use int64 raw"
        );
        assert!(
            schema.contains("reserved"),
            "field numbers must be reserved"
        );
        // 玩法 schema 只追加约束：独立消息、顶层 tag 9、oneof 保留段。
        assert!(schema.contains("message GameplayCommand {"));
        assert!(schema.contains("GameplayCommandBatch gameplay = 9;"));
        assert!(schema.contains("reserved 14 to 32;"));
        assert!(schema.contains("message GameplayCommandBatch {"));
        assert!(schema.contains("reserved 2 to 8;"));
        assert!(schema.contains("int64 amount = 2;"));
        // 既有字段编号 / 保留段零改动。
        assert!(schema.contains("reserved 11 to 16;"));
        assert!(schema.contains("reserved 3 to 6;"));
    }

    #[test]
    fn truncated_payload_is_rejected() {
        let bytes = encode_commands(&sample(), &sample_gameplay()).unwrap();
        assert!(matches!(
            decode_commands(&bytes[..bytes.len() - 1]),
            Err(NetError::Malformed(_))
        ));
    }

    #[test]
    fn resync_replay_carries_both_command_types_in_order() {
        let gameplay = sample_gameplay();
        let frames = vec![
            FrameCommands {
                frame: 2,
                commands: sample(),
                gameplay: gameplay[..3].to_vec(),
            },
            FrameCommands {
                frame: 3,
                commands: vec![],
                gameplay: vec![],
            },
            FrameCommands {
                frame: 4,
                commands: sample()[..1].to_vec(),
                gameplay: gameplay[3..].to_vec(),
            },
        ];
        let bytes = payload_resync_replay(&frames).unwrap();
        assert_eq!(parse_resync_replay(&bytes).unwrap(), frames);
        // 重复编码逐字节一致（canonical）。
        assert_eq!(payload_resync_replay(&frames).unwrap(), bytes);
    }

    // ---- PROTOCOL_VERSION 4：新增玩法变体的线格式回归 ----

    // 手工拼装 protobuf 原语（仅测试用），用于构造 canonical / 非 canonical 反例。
    fn put_varint(out: &mut Vec<u8>, mut value: u64) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    fn field_varint(tag: u32, value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, (u64::from(tag) << 3) | 0);
        put_varint(&mut out, value);
        out
    }

    fn field_bytes(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, (u64::from(tag) << 3) | 2);
        put_varint(&mut out, payload.len() as u64);
        out.extend_from_slice(payload);
        out
    }

    /// 手工拼装一条 `GameplayCommand`：oneof kind 落在调用方指定的 tag 上，
    /// 载荷为 `GameplayReviveHero` 形状，便于把新变体错放进 reserved 段做反例。
    fn gameplay_command_with_kind_tag(tag: u32, hero: u64, altar: u64, altar_def: u32) -> Vec<u8> {
        let mut revive = Vec::new();
        revive.extend(field_varint(1, hero));
        revive.extend(field_varint(2, altar));
        revive.extend(field_varint(3, u64::from(altar_def)));
        let mut command = Vec::new();
        command.extend(field_varint(1, 3)); // execute_frame = 3
        command.extend(field_varint(2, 1)); // player_id = 1
        command.extend(field_varint(3, 1)); // sequence = 1
        command.extend(field_bytes(tag, &revive));
        command
    }

    /// 把一条玩法命令装进 `GameplayCommandBatch`（field 1）与 `CommandBatch`（field 9）。
    fn command_batch_with_gameplay(command: Vec<u8>) -> Vec<u8> {
        field_bytes(9, &field_bytes(1, &command))
    }

    #[test]
    fn revive_and_cancel_variants_roundtrip_is_canonical() {
        // 含定宽边界值：uint64 / uint32 无损，且不产生收窄溢出。
        let gameplay = vec![
            GameplayCommand::new(
                Frame::new(3),
                1,
                1,
                GameplayCommandKind::ReviveHero {
                    hero: entity_from_raw(20),
                    altar: entity_from_raw(21),
                    altar_def: 22,
                },
            ),
            GameplayCommand::new(
                Frame::new(3),
                2,
                2,
                GameplayCommandKind::ReviveHero {
                    hero: entity_from_raw(u64::MAX),
                    altar: entity_from_raw(1),
                    altar_def: u32::MAX,
                },
            ),
            GameplayCommand::new(
                Frame::new(4),
                1,
                3,
                GameplayCommandKind::CancelRevive {
                    hero: entity_from_raw(u64::MAX),
                },
            ),
        ];
        let first = encode_commands(&[], &gameplay).unwrap();
        let second = encode_commands(&[], &gameplay).unwrap();
        assert_eq!(first, second, "two encodes must be byte-identical");
        let (commands, decoded) = decode_commands(&first).unwrap();
        assert!(commands.is_empty());
        assert_gameplay_eq(&decoded, &gameplay);
        assert_eq!(
            encode_commands(&commands, &decoded).unwrap(),
            first,
            "decode -> encode must be canonical"
        );
        // 逐条单独往返，避免批量编码掩盖某个变体的歧义映射。
        for command in &gameplay {
            let one = encode_commands(&[], std::slice::from_ref(command)).unwrap();
            let (_, one_decoded) = decode_commands(&one).unwrap();
            assert_gameplay_eq(&one_decoded, std::slice::from_ref(command));
        }
    }

    #[test]
    fn appended_variants_use_wire_tags_33_and_34() {
        // tag 33 / wire 2 -> key varint (33<<3)|2 = 266 -> [0x8a, 0x02]。
        let revive = encode_commands(
            &[],
            &[GameplayCommand::new(
                Frame::new(3),
                1,
                1,
                GameplayCommandKind::ReviveHero {
                    hero: entity_from_raw(5),
                    altar: entity_from_raw(6),
                    altar_def: 7,
                },
            )],
        )
        .unwrap();
        assert!(
            revive.windows(2).any(|w| w == &[0x8a, 0x02][..]),
            "ReviveHero must be encoded on tag 33"
        );
        // tag 34 / wire 2 -> key varint (34<<3)|2 = 274 -> [0x92, 0x02]。
        let cancel = encode_commands(
            &[],
            &[GameplayCommand::new(
                Frame::new(3),
                1,
                1,
                GameplayCommandKind::CancelRevive {
                    hero: entity_from_raw(5),
                },
            )],
        )
        .unwrap();
        assert!(
            cancel.windows(2).any(|w| w == &[0x92, 0x02][..]),
            "CancelRevive must be encoded on tag 34"
        );
    }

    #[test]
    fn reserved_tag_for_new_variant_is_rejected_not_silently_interpreted() {
        // 缺陷态：把 ReviveHero 载荷错放进 reserved 14 to 32 段的 tag 14。
        // prost 把该 tag 视作 unknown field 丢弃，kind 变 None，解码必须
        // 显式返回 Malformed，绝不猜测成某个玩法变体、也不静默降级。
        let defect = command_batch_with_gameplay(gameplay_command_with_kind_tag(14, 5, 6, 7));
        assert!(matches!(
            decode_commands(&defect),
            Err(NetError::Malformed(_))
        ));
        // reserved 段另一端 tag 32 同样被拒。
        let defect_high = command_batch_with_gameplay(gameplay_command_with_kind_tag(32, 5, 6, 7));
        assert!(matches!(
            decode_commands(&defect_high),
            Err(NetError::Malformed(_))
        ));

        // 元断言防退化：同一载荷只改回规范 tag 33 / 34 时必须成功解码，
        // 证明上面的拒绝仅来自「reserved tag 被丢弃」，而非载荷本身畸形。
        let revive = command_batch_with_gameplay(gameplay_command_with_kind_tag(33, 5, 6, 7));
        let (commands, gameplay) = decode_commands(&revive).unwrap();
        assert!(commands.is_empty());
        assert_eq!(gameplay.len(), 1);
        assert_eq!(
            gameplay[0].kind,
            GameplayCommandKind::ReviveHero {
                hero: entity_from_raw(5),
                altar: entity_from_raw(6),
                altar_def: 7,
            }
        );
        // tag 34 载荷为 CancelRevive 形状（field 1 = hero）。
        let mut cancel = Vec::new();
        cancel.extend(field_varint(1, 3));
        cancel.extend(field_varint(2, 1));
        cancel.extend(field_varint(3, 1));
        cancel.extend(field_bytes(34, &field_varint(1, 5)));
        let cancel_batch = command_batch_with_gameplay(cancel);
        let (_, gameplay) = decode_commands(&cancel_batch).unwrap();
        assert_eq!(
            gameplay[0].kind,
            GameplayCommandKind::CancelRevive {
                hero: entity_from_raw(5),
            }
        );
    }

    #[test]
    fn non_canonical_varint_is_rejected_or_normalized_never_echoed() {
        // 溢出型非规范 varint（10 个以上 continuation 字节）：prost 报
        // invalid varint，decode_commands 明确转成 NetError::Malformed。
        let mut overlong_cmd = vec![0x08]; // execute_frame field 1, wire type 0
        overlong_cmd.extend_from_slice(&[0x80u8; 11]);
        overlong_cmd.push(0x00);
        overlong_cmd.extend(field_varint(2, 1));
        overlong_cmd.extend(field_varint(3, 1));
        let overflow = command_batch_with_gameplay(overlong_cmd);
        assert!(matches!(
            decode_commands(&overflow),
            Err(NetError::Malformed(_))
        ));

        // overlong 但可解码的 varint（0x81 0x00 表示 1）：prost 宽容解码，
        // 但解码 -> 再编码产出规范最短字节，绝不回显非规范字节（canonical）。
        // ADR 0005 §3 的「解码 → 再编码」canonical 往返校验正是此约束。
        let mut non_canonical = vec![0x08];
        non_canonical.extend_from_slice(&[0x81, 0x00]); // execute_frame = 1（overlong）
        non_canonical.extend(field_varint(2, 1)); // player_id = 1
        non_canonical.extend(field_varint(3, 1)); // sequence = 1
        let mut revive = Vec::new();
        revive.extend(field_varint(1, 5));
        revive.extend(field_varint(2, 6));
        revive.extend(field_varint(3, 7));
        non_canonical.extend(field_bytes(33, &revive));
        let bytes = command_batch_with_gameplay(non_canonical.clone());
        let (commands, gameplay) = decode_commands(&bytes).unwrap();
        assert_eq!(
            gameplay[0].kind,
            GameplayCommandKind::ReviveHero {
                hero: entity_from_raw(5),
                altar: entity_from_raw(6),
                altar_def: 7,
            }
        );
        let reencoded = encode_commands(&commands, &gameplay).unwrap();
        // 再编码等于同一逻辑命令的规范编码，且绝不回显 overlong 字节。
        let canonical_command = GameplayCommand::new(
            Frame::new(1),
            1,
            1,
            GameplayCommandKind::ReviveHero {
                hero: entity_from_raw(5),
                altar: entity_from_raw(6),
                altar_def: 7,
            },
        );
        assert_eq!(
            reencoded,
            encode_commands(&[], &[canonical_command]).unwrap(),
            "decode -> encode must normalize to canonical bytes"
        );
        assert_ne!(reencoded, bytes, "non-canonical bytes must not be echoed");
        assert!(
            reencoded.windows(2).any(|w| w == &[0x08, 0x01][..]),
            "re-encode must use minimal varint for execute_frame=1"
        );
        assert!(
            !reencoded.windows(3).any(|w| w == &[0x08, 0x81, 0x00][..]),
            "overlong varint must not survive re-encode"
        );
    }

    #[test]
    fn schema_appends_only_tags_33_and_34_and_preserves_reserved() {
        let schema = include_str!("proto/w3net.proto");
        assert!(schema.contains("reserved 14 to 32;"));
        assert!(schema.contains("message GameplayReviveHero {"));
        assert!(schema.contains("message GameplayCancelRevive {"));
        assert!(schema.contains("uint64 hero = 1;"));
        assert!(schema.contains("uint64 altar = 2;"));
        assert!(schema.contains("uint32 altar_def = 3;"));
        // 只追加：解析玩法 oneof 的全部 tag，必须恰好是既有 4–13 加 33 / 34。
        let oneof = schema
            .split("oneof kind {")
            .nth(2)
            .expect("gameplay oneof missing")
            .split('}')
            .next()
            .expect("gameplay oneof body missing");
        let tags: Vec<u32> = oneof
            .lines()
            .filter_map(|line| {
                let code = line.split("//").next().unwrap_or("").trim();
                let value = code.rsplit("= ").next()?.trim().trim_end_matches(';');
                value.parse().ok()
            })
            .collect();
        assert_eq!(
            tags,
            vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 33, 34],
            "oneof tags must be append-only (4-13 + 33/34)"
        );
        // 新消息尾部保留 reserved；既有编号段零改动。
        assert!(schema.contains("reserved 4 to 8;"));
        assert!(schema.contains("reserved 2 to 8;"));
        assert!(schema.contains("reserved 11 to 16;"));
    }
}
