//! P8.5c 野怪营地与掉落回归的共享夹具（合成原创编译期整数常量 `[待核]`）。
//!
//! 全部数值 / DefId 均为非经典命名、零文件 IO、零资产、不依赖 `w3-content`，
//! 只经 `w3-core` 公开 API 构造（Q8 / §14 / ADR 0001 / ADR 0004）。
//! 业务规则：R1.1 / R1.4 / R6.1 / R6.5 / R7.2 / R7.5 / R7.6 / R8.1–R8.6。

#![allow(dead_code)]

use std::collections::BTreeMap;

use w3_core::{
    CampScript, CreepConstants, DefId, DropSet, DropTable, EffectDef, EffectKind, EntityId, Frame,
    FrozenDefinitions, GameplayCommand, GameplayCommandKind, HeroState, MatchConfig, NEUTRAL_OWNER,
    NeutralCategory, RtsMatch, SYNTHETIC_CREEP_CONSTANTS, SimFixed, StaticWorld, UnitDef,
};

pub const SEED: u64 = 0x5eed;
pub const PLAYER: u8 = 1;
pub const OTHER_PLAYER: u8 = 2;
/// 非经典命名的野怪 DefId（合成原创 `[待核]`，MUST NOT 复用 9xx owner 语义）。
pub const CREEP_DEF: DefId = 9101;
/// 无主地面物品 DefId（同名道具多实例独立，R7.5）。
pub const ITEM_DEF: DefId = 9201;
pub const ITEM_B: DefId = 9202;
pub const CAMP_ID: u32 = 7;
pub const CAMP_SCRIPT: u32 = 41;
pub const DROP_TABLE: u32 = 91;
pub const SET_ALWAYS: u32 = 1;
pub const SET_EXCLUSIVE: u32 = 2;
pub const SCALE: i64 = 1 << 16;
pub const HOME_X: i64 = 2 * SCALE;
pub const HOME_Y: i64 = 2 * SCALE;
pub const CREEP_HP: i64 = 50;
pub const CREEP_XP: i64 = 24;
pub const CREEP_GOLD: i64 = 12;

/// 野怪单位定义（可移动、低伤害，便于确定性测试）。
pub fn creep_unit() -> UnitDef {
    UnitDef {
        hp: CREEP_HP,
        mana: 0,
        speed_raw: SimFixed::from_int(6).unwrap().raw(),
        damage: 5,
        range_raw: SimFixed::from_int(2).unwrap().raw(),
        supply: 0,
        train_frames: 1,
    }
}

/// 收紧 leash / 归巢半径以便 8×8 默认世界内可观测（数据驱动，`[待核]`）。
pub fn constants() -> CreepConstants {
    CreepConstants {
        leash_radius_raw: 2 * SCALE,
        return_threshold_raw: 1 * SCALE,
        acquisition_range_raw: 3 * SCALE,
        xp_share_radius_raw: 5 * SCALE,
        ai_decision_period: 1,
        ..SYNTHETIC_CREEP_CONSTANTS
    }
}

/// 冻结营地脚本：一个营地、两只野怪（R8.2 分级 / R8.3 收益）。
pub fn scripts() -> BTreeMap<u32, CampScript> {
    [(
        CAMP_SCRIPT,
        CampScript {
            tier: 1,
            xp: CREEP_XP,
            gold: CREEP_GOLD,
            home_x: HOME_X,
            home_y: HOME_Y,
            drop_table: DROP_TABLE,
            creep_defs: vec![CREEP_DEF, CREEP_DEF],
        },
    )]
    .into_iter()
    .collect()
}

/// 冻结掉落表：表 → 两个互斥 set → 概率条目（R7.2 / R8.6 / 结论 3）。
/// set 1 恒定掉 ITEM_DEF；set 2 在 ITEM_DEF / ITEM_B 中互斥二选一。
/// 效果走既有 `EffectDef` 组合（R7.6 / N5），不含 JASS VM。
pub fn tables() -> BTreeMap<u32, DropTable> {
    [(
        DROP_TABLE,
        DropTable {
            sets: [
                (
                    SET_ALWAYS,
                    DropSet {
                        entries: [(ITEM_DEF, 100u32)].into_iter().collect(),
                    },
                ),
                (
                    SET_EXCLUSIVE,
                    DropSet {
                        entries: [(ITEM_DEF, 60u32), (ITEM_B, 40u32)].into_iter().collect(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            effects: [
                (
                    ITEM_DEF,
                    EffectDef {
                        kind: EffectKind::BuffStat,
                        amount: 2,
                        duration_frames: 10,
                    },
                ),
                (
                    ITEM_B,
                    EffectDef {
                        kind: EffectKind::Heal,
                        amount: 5,
                        duration_frames: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        },
    )]
    .into_iter()
    .collect()
}

/// 定义层中立类别 / 阵营标记（Q7 / R8.1 / R1.1），非白名单 / 非硬编码。
pub fn categories() -> BTreeMap<DefId, NeutralCategory> {
    [
        (CREEP_DEF, NeutralCategory::CreepHostile),
        (ITEM_DEF, NeutralCategory::GroundItemOwnerless),
        (ITEM_B, NeutralCategory::GroundItemOwnerless),
    ]
    .into_iter()
    .collect()
}

/// 正式冻结定义（Q8）。
pub fn defs() -> FrozenDefinitions {
    FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(constants(), scripts(), tables(), categories())
}

/// 退化定义：leash 半径拉到极大，模拟「无限追出营地」（R8.5 缺陷态）。
/// 经同一 `with_creep_camps` / `create_with_definitions` 生产路径注入。
pub fn no_leash_defs() -> FrozenDefinitions {
    let mut bad = constants();
    bad.leash_radius_raw = 1_000_000 * SCALE;
    FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(bad, scripts(), tables(), categories())
}

pub fn config(frozen: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: frozen.content_hash(),
    }
}

/// frame-0 系统级营地装配命令（Q5，同源 `player_id = NEUTRAL_OWNER`）。
pub fn seed_camp(frame: u64, camp: u32, script: u32) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        NEUTRAL_OWNER,
        1,
        GameplayCommandKind::SeedCreepCamp { camp, script },
    )
}

/// 玩家单位 Spawn（普通命令面）。
pub fn spawn(x: i64, y: i64, hp: i64, damage: i64) -> w3_core::Command {
    w3_core::Command::new(
        Frame::new(0),
        PLAYER,
        1,
        w3_core::CommandKind::Spawn {
            x: SimFixed::from_raw(x),
            y: SimFixed::from_raw(y),
            hp,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: damage,
            attack_range: SimFixed::from_int(100).unwrap(),
        },
    )
}

/// 无野怪的干净对局（用于重开 / 退回基线）。
pub fn fresh_match() -> RtsMatch {
    RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs())
}

/// 正式对局：frame-0 提交营地装配 + 玩家单位，推进一帧完成装配。
/// 返回 `(sim, content_hash)`；野怪实体 id 为 2 / 3（玩家单位 id 为 1）。
pub fn seeded_match() -> (RtsMatch, u64) {
    let frozen = defs();
    let content = frozen.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    sim.submit_command(spawn(HOME_X, HOME_Y, 100, 100)).unwrap();
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    (sim, content)
}

/// 经真实 `capture_snapshot` / `restore_snapshot` 生产路径把玩家单位标记为
/// 英雄（R6.1 经验入账），非旁路手搓权威字段。
pub fn with_hero(sim: RtsMatch, content: u64) -> RtsMatch {
    let mut snapshot = sim.capture_snapshot();
    let record = snapshot
        .positions
        .iter_mut()
        .find(|record| record.entity == 1)
        .unwrap();
    record.gameplay.hero = Some(HeroState { level: 1, xp: 0 });
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

/// 提交一次 `Attack` 命令（既有攻击命令面，稳定排序键）。
pub fn attack(sim: &mut RtsMatch, frame: u64, attacker: u64, target: u64, sequence: u32) {
    sim.submit_command(w3_core::Command::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        w3_core::CommandKind::Attack {
            attacker: EntityId::new(attacker).unwrap(),
            target: EntityId::new(target).unwrap(),
        },
    ))
    .unwrap();
}

/// 野怪实体 id（装配顺序：玩家单位 1，野怪 2 / 3）。
pub fn creep_ids() -> Vec<EntityId> {
    vec![EntityId::new(2).unwrap(), EntityId::new(3).unwrap()]
}

/// 推进到所有野怪死亡并完成结算（双实例 / 回放共用脚本）。
pub fn kill_both_and_settle(sim: &mut RtsMatch, frame: u64) {
    attack(sim, frame, 1, 2, 1);
    attack(sim, frame, 1, 3, 2);
    sim.step();
}
