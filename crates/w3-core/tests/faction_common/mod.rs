//! P9 族特色机制回归的共享夹具（Q2 / Q4 / ADR 0002）。
//!
//! 全部数值 / DefId 均为合成原创编译期整数常量 `[待核]`，零文件 IO、
//! 零资产、不依赖 `w3-content`，只经 `w3-core` 公开 API 构造（Q8 / §14）。
//! 场景注入一律走 `capture_snapshot()` → 改 public 字段 → `restore_snapshot()`
//! 这一既有确定性生产路径（非旁路手搓权威状态）；命令驱动一律走
//! `submit_gameplay_command()` → `step()` 公开入口。

#![allow(dead_code)]

use std::collections::BTreeMap;

use w3_core::{
    Command, CommandKind, DefId, EntityId, FactionMechanicsConstants, Frame, FrozenDefinitions,
    GameplayCommand, GameplayCommandKind, MatchConfig, RegenCondition, RtsMatch,
    SYNTHETIC_FACTION_MECHANICS, SimFixed, StaticWorld,
};

pub const SEED: u64 = 0x5eed;
pub const PLAYER: u8 = 1;
pub const OTHER_PLAYER: u8 = 2;
/// frame-0 出生实体（id 1 / 2 / 3）。
pub const UNIT_A: u64 = 1;
pub const UNIT_B: u64 = 2;
pub const UNIT_C: u64 = 3;
/// 合成定义层类别：回血条件登记用的 DefId（3xx 段语义，避开既有段）。
pub const REGEN_DEF_ALWAYS: DefId = 391;
pub const REGEN_DEF_NIGHT: DefId = 392;
pub const REGEN_DEF_DAY: DefId = 393;
pub const REGEN_DEF_BLIGHT: DefId = 394;
pub const REGEN_DEF_NONE: DefId = 395;

/// 缩短昼夜 / 扩张 / 衰减 / 采集间隔便于逐帧验证，其余沿用合成默认值。
/// 全为合成原创编译期整数 `[待核]`（Q3'）。
pub fn short_constants() -> FactionMechanicsConstants {
    let mut constants = SYNTHETIC_FACTION_MECHANICS;
    constants.day_frames = 4;
    constants.night_frames = 3;
    constants.blight_expansion_frames = 2;
    constants.corpse_decay_frames = 3;
    constants.entangle_mine_interval = 2;
    constants.haunted_mine_interval = 3;
    constants
}

/// 冻结定义：默认四张空表 + 显式注入缩短常量 + 回血条件类别登记。
/// 改常量 / 改类别登记都会改变 `content_hash()`（定义层内容变化）。
pub fn defs() -> FrozenDefinitions {
    defs_with_constants(short_constants())
}

/// 以指定族特色机制常量构造冻结定义。
pub fn defs_with_constants(constants: FactionMechanicsConstants) -> FrozenDefinitions {
    let mut regen = BTreeMap::new();
    regen.insert(REGEN_DEF_ALWAYS, RegenCondition::Always);
    regen.insert(REGEN_DEF_NIGHT, RegenCondition::Night);
    regen.insert(REGEN_DEF_DAY, RegenCondition::Day);
    regen.insert(REGEN_DEF_BLIGHT, RegenCondition::Blight);
    regen.insert(REGEN_DEF_NONE, RegenCondition::None);
    FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(constants)
        .with_regen_conditions(regen)
}

/// 默认常量（未显式注入）的冻结定义，用于 client 零改动编译 / 内容敏感性对照。
pub fn default_defs() -> FrozenDefinitions {
    FrozenDefinitions::new(vec![], vec![], vec![], vec![]).unwrap()
}

pub fn config(frozen: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: frozen.content_hash(),
    }
}

fn spawn(sequence: u32) -> Command {
    Command::new(
        Frame::new(0),
        PLAYER,
        sequence,
        CommandKind::Spawn {
            x: SimFixed::from_int(2).unwrap(),
            y: SimFixed::from_int(2).unwrap(),
            hp: 100,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: 0,
            attack_range: SimFixed::from_int(10).unwrap(),
        },
    )
}

/// 正式对局：frame-0 出生三个实体（id 1 / 2 / 3）→ `step()`。
pub fn base_match() -> (RtsMatch, u64) {
    let frozen = defs();
    let content = frozen.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    for sequence in 1..=3u32 {
        sim.submit_command(spawn(sequence)).unwrap();
    }
    sim.step();
    (sim, content)
}

/// 以指定冻结定义构造正式对局（frame-0 三实体 + 一帧）。
pub fn base_match_with(frozen: FrozenDefinitions) -> (RtsMatch, u64) {
    let content = frozen.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    for sequence in 1..=3u32 {
        sim.submit_command(spawn(sequence)).unwrap();
    }
    sim.step();
    (sim, content)
}

/// 经真实 `restore_snapshot` 生产路径注入任意只读快照改动。
pub fn with_state<F>(sim: &RtsMatch, content: u64, mutate: F) -> RtsMatch
where
    F: FnOnce(&mut w3_core::Snapshot),
{
    let mut snapshot = sim.capture_snapshot();
    mutate(&mut snapshot);
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

/// 推进指定整数帧数（不接受 delta / 墙钟，ADR 0002 §2）。
pub fn step_n(sim: &mut RtsMatch, frames: usize) {
    for _ in 0..frames {
        sim.step();
    }
}

/// 提交一条玩法命令（公开入口，冻结帧号 / 到达序号）。
pub fn submit(sim: &mut RtsMatch, frame: u64, sequence: u32, kind: GameplayCommandKind) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        kind,
    ))
    .unwrap();
}

pub fn id(value: u64) -> EntityId {
    EntityId::new(value).unwrap()
}

/// 13 个新增 `GameplayCommandKind` 变体（kind 码 24..=36），供穷尽分发 /
/// 编码唯一性 / 校验反向验证共用。字段全为合成原创 `[待核]` 值。
pub fn new_command_kinds() -> Vec<GameplayCommandKind> {
    vec![
        GameplayCommandKind::SeedBlight {
            source: id(UNIT_A),
            cell_x: 3,
            cell_y: 4,
        },
        GameplayCommandKind::RaiseCorpse {
            corpse: id(UNIT_B),
            decay_frames: 5,
        },
        GameplayCommandKind::ConsumeCorpse {
            consumer: id(UNIT_A),
            corpse: id(UNIT_B),
        },
        GameplayCommandKind::HauntGoldMine { mine: id(UNIT_C) },
        GameplayCommandKind::EntangleGoldMine { mine: id(UNIT_C) },
        GameplayCommandKind::SetAncientRoot {
            ancient: id(UNIT_A),
            rooted: true,
        },
        GameplayCommandKind::BindMoonWell {
            well: id(UNIT_B),
            restore: 50,
        },
        GameplayCommandKind::SetStealth {
            unit: id(UNIT_A),
            cloaked: true,
        },
        GameplayCommandKind::RegisterDetector { unit: id(UNIT_B) },
        GameplayCommandKind::SetNightCloak {
            unit: id(UNIT_C),
            enabled: true,
        },
        GameplayCommandKind::MarkSightBlocker {
            blocker: id(UNIT_A),
            blocking: true,
        },
        GameplayCommandKind::SetCreepSleep {
            creep: id(UNIT_C),
            sleeps_at_night: true,
        },
        GameplayCommandKind::SetRegenCondition {
            unit: id(UNIT_A),
            condition: RegenCondition::Always,
        },
    ]
}
