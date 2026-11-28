//! P11 技能系统确定性测试的公共夹具。
//!
//! 全部定义为合成原创编译期整数常量，零文件 IO、零资产、零外部依赖；
//! `FrozenDefinitions` 同时登记技能清单条目与可施放 `AbilityDef`，
//! 供双实例、回放、快照往返与反向验证复用同一输入流。

// 公共夹具按测试目标选择性复用；未使用函数不产生噪声。
#![allow(dead_code)]

use std::collections::BTreeSet;

use w3_core::{
    ABILITY_CATALOG_BASE, AbilityCategory, AbilityDef, AbilityTargetKind, BEHAVIOR_CATALOG_BASE,
    BehaviorKind, BuffCategory, Command, CommandKind, DefId, EffectDef, EffectKind, EntityId,
    FactionSlot, Frame, FrozenDefinitions, GameplayCommand, GameplayCommandKind, MatchConfig,
    RtsMatch, SIM_VERSION, SNAPSHOT_VERSION, SYNTHETIC_ABILITY_CATALOG, SYNTHETIC_REVIVE_CONSTANTS,
    SimFixed, StaticWorld, UnitDef,
};

/// 本地玩家编号。
pub const PLAYER: u8 = 1;
/// 目标单位所属玩家编号。
pub const TARGET_PLAYER: u8 = 2;
/// 生产者定义 ID。
pub const PRODUCER_DEF: DefId = 100;
/// 英雄定义 ID；由 `hero_defs` 登记为定义层英雄类别。
pub const HERO_DEF: DefId = 101;

/// 稳定 `EntityId` 构造；0 号 ID 只作 `NONE` 哨兵。
pub fn id(value: u64) -> EntityId {
    EntityId::new(value).expect("ability_common: reason=invalid_entity_id")
}

/// 施法者 / 生产者实体 ID：帧 0 按玩家顺序生成 1 / 2，训练英雄为 3。
pub fn producer() -> EntityId {
    id(1)
}

/// 敌方目标实体 ID。
pub fn target() -> EntityId {
    id(2)
}

/// 由 `Train` 产出的英雄实体 ID。
pub fn hero() -> EntityId {
    id(3)
}

/// 合成原创单位定义；`speed_raw` 为 Q47.16 整数原始值。
pub fn unit_def(speed_raw: i64) -> UnitDef {
    UnitDef {
        hp: 100,
        mana: 100,
        speed_raw,
        damage: 1,
        range_raw: 10 << 16,
        supply: 0,
        train_frames: 1,
    }
}

/// 按行为种类生成冻结 `AbilityDef`；效果组合仍走 `EffectDef`，不引入脚本 VM。
pub fn ability_def(entry_behavior: DefId, kind: EffectKind) -> AbilityDef {
    let amount = if kind == EffectKind::RemoveBuffCategory {
        i64::from(BuffCategory::Magic.code())
    } else {
        0
    };
    AbilityDef {
        mana_cost: 0,
        cooldown_frames: 0,
        cast_frames: 0,
        behavior: entry_behavior,
        effects: vec![
            EffectDef {
                kind,
                amount,
                duration_frames: 1,
            },
            EffectDef {
                kind: EffectKind::BuffStat,
                amount: 1,
                duration_frames: 4,
            },
        ],
    }
}

/// 由技能清单条目与行为目录生成全部可施放能力定义。
pub fn ability_defs() -> FrozenDefinitions {
    let units = vec![(PRODUCER_DEF, unit_def(0)), (HERO_DEF, unit_def(3 << 16))];
    let mut abilities = Vec::new();
    for entry in SYNTHETIC_ABILITY_CATALOG.iter().copied() {
        let offset = entry
            .behavior_id
            .checked_sub(BEHAVIOR_CATALOG_BASE)
            .expect("ability_common: reason=behavior_id_before_base");
        let behavior = BehaviorKind::from_code(
            u8::try_from(offset).expect("ability_common: reason=behavior_id_out_of_range"),
        )
        .expect("ability_common: reason=unknown_behavior_code");
        abilities.push((
            entry.id,
            ability_def(entry.behavior_id, behavior.effect_kind()),
        ));
    }
    let hero_defs: BTreeSet<DefId> = [HERO_DEF].into_iter().collect();
    FrozenDefinitions::new(units, abilities, Vec::new(), Vec::new())
        .expect("ability_common: reason=definitions_invalid")
        .with_hero_revive(SYNTHETIC_REVIVE_CONSTANTS, BTreeSet::new(), hero_defs)
}

/// 固定配置；内容哈希取冻结定义内容哈希，保证恢复头部校验一致。
pub fn config(defs: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: 0x5eed,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    }
}

/// 帧 0 生成命令；参数均为 Q47.16 整数原始坐标。
pub fn spawn_kind(x: i64, y: i64) -> CommandKind {
    CommandKind::Spawn {
        x: SimFixed::from_int(x).expect("ability_common: reason=fixed_conversion_failed"),
        y: SimFixed::from_int(y).expect("ability_common: reason=fixed_conversion_failed"),
        hp: 100,
        speed: SimFixed::from_int(3).expect("ability_common: reason=fixed_conversion_failed"),
        attack_damage: 1,
        attack_range: SimFixed::from_int(100)
            .expect("ability_common: reason=fixed_conversion_failed"),
    }
}

/// 玩法命令构造；提交时由队列冻结到达序号。
pub fn gp(frame: u64, player: u8, sequence: u32, kind: GameplayCommandKind) -> GameplayCommand {
    GameplayCommand::new(Frame::new(frame), player, sequence, kind)
}

/// 基础对局：帧 0 两个 Spawn、帧 1 训练英雄，附加命令须在帧 2 之后执行。
pub fn setup(extra: &[GameplayCommand]) -> (RtsMatch, FrozenDefinitions, u64) {
    let defs = ability_defs();
    let cfg = config(&defs);
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 1, spawn_kind(0, 0)))
        .expect("ability_common: reason=spawn_producer_rejected");
    sim.submit_command(Command::new(
        Frame::new(0),
        TARGET_PLAYER,
        1,
        spawn_kind(1, 0),
    ))
    .expect("ability_common: reason=spawn_target_rejected");
    sim.submit_gameplay_command(gp(
        1,
        PLAYER,
        1,
        GameplayCommandKind::Train {
            producer: producer(),
            unit_def: HERO_DEF,
            is_building: false,
        },
    ))
    .expect("ability_common: reason=train_hero_rejected");
    for &command in extra {
        sim.submit_gameplay_command(command)
            .expect("ability_common: reason=extra_command_rejected");
    }
    (sim, defs, cfg.content_hash)
}

/// 找到某席位下指定行为种类的可施放 `DefId`。
pub fn ability_for_behavior(faction: FactionSlot, behavior: BehaviorKind) -> DefId {
    let behavior_id = BEHAVIOR_CATALOG_BASE + u32::from(behavior.code());
    SYNTHETIC_ABILITY_CATALOG
        .iter()
        .copied()
        .find(|entry| entry.faction == faction && entry.behavior_id == behavior_id)
        .map(|entry| entry.id)
        .expect("ability_common: reason=behavior_ability_missing")
}

/// 找到某席位下第一项英雄大招 `DefId`。
pub fn ultimate_for(faction: FactionSlot) -> DefId {
    SYNTHETIC_ABILITY_CATALOG
        .iter()
        .copied()
        .find(|entry| entry.faction == faction && entry.category == AbilityCategory::HeroUltimate)
        .map(|entry| entry.id)
        .expect("ability_common: reason=ultimate_ability_missing")
}

/// 找到某席位下第一项英雄普通技能 `DefId`。
pub fn basic_for(faction: FactionSlot) -> DefId {
    SYNTHETIC_ABILITY_CATALOG
        .iter()
        .copied()
        .find(|entry| entry.faction == faction && entry.category == AbilityCategory::HeroBasic)
        .map(|entry| entry.id)
        .expect("ability_common: reason=basic_ability_missing")
}

/// 四席位通用的技能命令帧流：升级 → 学习大招 / 普通技能 → 九类原语施法。
pub fn command_stream(faction: FactionSlot) -> Vec<GameplayCommand> {
    let hero = hero();
    let target = target();
    let mut commands = Vec::new();
    for index in 0..5u64 {
        commands.push(gp(
            2 + index,
            PLAYER,
            100 + index as u32,
            GameplayCommandKind::LevelUpHero { hero },
        ));
    }
    commands.push(gp(
        7,
        PLAYER,
        200,
        GameplayCommandKind::LearnAbility {
            hero,
            ability: ultimate_for(faction),
        },
    ));
    commands.push(gp(
        8,
        PLAYER,
        201,
        GameplayCommandKind::LearnAbility {
            hero,
            ability: basic_for(faction),
        },
    ));
    let mut frame = 9u64;
    let mut sequence = 300u32;
    for behavior in BehaviorKind::ALL {
        let ability = ability_for_behavior(faction, behavior);
        let kind = match behavior.code() % 4 {
            0 => GameplayCommandKind::CastPoint {
                caster: hero,
                ability,
                x: 2 << 16,
                y: 3 << 16,
            },
            1 => GameplayCommandKind::CastArea {
                caster: hero,
                ability,
                x: 4 << 16,
                y: 5 << 16,
                radius_raw: 6 << 16,
            },
            2 => GameplayCommandKind::CastDirection {
                caster: hero,
                ability,
                dx: 1 << 16,
                dy: 2 << 16,
            },
            _ => GameplayCommandKind::Cast {
                caster: hero,
                ability,
                target,
            },
        };
        commands.push(gp(frame, PLAYER, sequence, kind));
        frame += 1;
        sequence += 1;
    }
    commands
}

/// 纯整数 LCG，与内核 `next_rng` 同算法；用于验证每帧恰好一次主流消费。
pub fn next_rng(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// 稳定的点目标形态，供反向验证构造差异字段。
pub fn point_target() -> AbilityTargetKind {
    AbilityTargetKind::Point {
        x: 7 << 16,
        y: 8 << 16,
    }
}

/// 当前版本号硬断言的公共入口。
pub fn assert_v10_versions() {
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    assert_eq!(SIM_VERSION, 10, "reason=sim_version_10");
    assert_eq!(
        ABILITY_CATALOG_BASE % 2,
        0,
        "reason=synthetic_catalog_segment_is_even"
    );
}
