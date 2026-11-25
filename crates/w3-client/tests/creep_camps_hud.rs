//! P8.5c 野怪营地与掉落：真实装配 CPU 回归 + HUD 只读派生（tasks 7.1 / 7.3）。
//!
//! 全部为合成命令脚本与 `synthetic_definitions()` 真实装配，零资产、零文件 IO、
//! 不开 GPU 窗口、不引入墙钟、不依赖 `w3-core/tests` 私有夹具。本文件只证明
//! 「内核 + 真实装配 CPU 回归层」的 seed / 清野 / 击杀收益 / 掉落可达，且
//! 双实例同脚本逐帧 `compute_state_hash()` / `capture_snapshot()` 一致；
//! 「真窗口 UI / AI 触发层」按 Q2 / Q3 / Q6 范围外承接，当前无 `U` 键 /
//! 命令卡 / AI 注入接线，不能在真窗口触发，不冒充真窗口验收。
//!
//! HUD 侧只从只读 `Snapshot.creep_camps` 与六类事实事件拷贝派生；不持有
//! `RtsMatch` 写句柄，不进状态哈希 / 快照 / `first_difference()`。

use w3_client::CreepCampHudModel;
use w3_client::melee::defs::{
    CREEP_CAMP_ID, CREEP_CAMP_SCRIPT, CREEP_DROP_TABLE, CREEP_GOLD, CREEP_HOME_X, CREEP_HOME_Y,
    CREEP_ITEM_ALWAYS, CREEP_ITEM_CHOICE, CREEP_UNIT, CREEP_XP, synthetic_definitions,
};
use w3_core::{
    AbilityDef, Command, CommandKind, DefId, EntityId, Event, EventKind, Frame, FrozenDefinitions,
    GameplayCommand, GameplayCommandKind, HeroState, ItemDef, MatchConfig, NEUTRAL_OWNER, RtsMatch,
    SNAPSHOT_VERSION, SimFixed, StaticWorld, UnitDef, UpgradeDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
const PLAYER_ENTITY: u64 = 1;
const CREEP_ENTITY_IDS: [u64; 2] = [2, 3];

/// 由整数字面量构造 Q47.16 定点值；显式处理错误，不静默吞错。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("reason=fixed_conversion_failed")
}

/// frame-0 玩家单位：生命 / 伤害 / 射程足够经既有 `Attack` 命令面稳定清野。
fn spawn(x: i64, y: i64) -> Command {
    Command::new(
        Frame::new(0),
        PLAYER,
        1,
        CommandKind::Spawn {
            x: SimFixed::from_raw(x),
            y: SimFixed::from_raw(y),
            hp: 100,
            speed: fixed(3),
            attack_damage: 100,
            attack_range: fixed(100),
        },
    )
}

/// frame-0 系统级营地装配命令；同源 `player_id` 固定为 `NEUTRAL_OWNER`。
fn seed_camp(frame: u64, camp: u32, script: u32) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        NEUTRAL_OWNER,
        1,
        GameplayCommandKind::SeedCreepCamp { camp, script },
    )
}

/// 用真实 `FrozenDefinitions` 装配对局；`seed_camp_enabled` 控制是否提交营地命令。
fn build_match(defs: &FrozenDefinitions, script: u32, seed_camp_enabled: bool) -> RtsMatch {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    sim.submit_command(spawn(CREEP_HOME_X, CREEP_HOME_Y))
        .expect("reason=spawn_command_rejected");
    if seed_camp_enabled {
        sim.submit_gameplay_command(seed_camp(0, CREEP_CAMP_ID, script))
            .expect("reason=seed_camp_command_rejected");
    }
    sim.step();
    sim
}

/// 经真实 `capture_snapshot` / `restore_snapshot` 生产路径把玩家单位标记为英雄。
/// 这是 CPU 夹具注入初始状态，不是表现层反写权威状态。
fn with_hero(sim: RtsMatch, content: u64) -> RtsMatch {
    let mut snapshot = sim.capture_snapshot();
    let record = snapshot
        .positions
        .iter_mut()
        .find(|record| record.entity == PLAYER_ENTITY)
        .expect("reason=missing_player_entity");
    record.gameplay.hero = Some(HeroState { level: 1, xp: 0 });
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
        .expect("reason=hero_snapshot_restore_failed")
}

/// 经既有 `Attack` 命令面击杀两只野怪，并让 `settle_creep_deaths` 在 `step()` 结算。
fn kill_both(sim: &mut RtsMatch, frame: u64) {
    for (index, target) in CREEP_ENTITY_IDS.into_iter().enumerate() {
        let attacker = EntityId::new(PLAYER_ENTITY).expect("reason=invalid_attacker_id");
        let target_id = EntityId::new(target).expect("reason=invalid_creep_id");
        sim.submit_command(Command::new(
            Frame::new(frame),
            PLAYER,
            index as u32 + 1,
            CommandKind::Attack {
                attacker,
                target: target_id,
            },
        ))
        .expect("reason=attack_command_rejected");
    }
    sim.step();
}

/// 与正式装配同源、但故意漏接 `with_creep_camps` 冻结段的退化定义（缺陷态）。
/// 复活 / 主城升级段仍同源登记，使唯一差异就是野怪营地脚本 / 掉落表 / 类别缺失。
fn defs_without_creep_segment() -> FrozenDefinitions {
    let full = synthetic_definitions();
    let units: Vec<(DefId, UnitDef)> = full.units().iter().map(|(&id, &def)| (id, def)).collect();
    let abilities: Vec<(DefId, AbilityDef)> = full
        .abilities()
        .iter()
        .map(|(&id, def)| (id, def.clone()))
        .collect();
    let items: Vec<(DefId, ItemDef)> = full.items().iter().map(|(&id, &def)| (id, def)).collect();
    let upgrades: Vec<(DefId, UpgradeDef)> = full
        .upgrades()
        .iter()
        .map(|(&id, &def)| (id, def))
        .collect();
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("reason=degenerate_definitions_invalid")
        .with_hero_revive(
            *full.revive_constants(),
            full.altar_defs().clone(),
            full.hero_defs().clone(),
        )
        .with_town_hall_upgrade(
            *full.town_hall_constants(),
            full.town_hall_defs().clone(),
            full.tier_requirements().clone(),
        )
        // P8.5d：同源登记商店段，使唯一差异仍是野怪营地脚本 / 掉落表 / 类别缺失。
        .with_item_shop(full.item_shop().clone())
}

/// 六类事实事件拷贝，供 HUD 只读派生回归使用；`Event` 不携带 payload。
fn fact_events() -> Vec<Event> {
    vec![
        Event {
            frame: 1,
            sequence: 900_001,
            entity: 1,
            kind: EventKind::CreepCampSeeded,
        },
        Event {
            frame: 1,
            sequence: 900_002,
            entity: 2,
            kind: EventKind::CreepEngaged,
        },
        Event {
            frame: 1,
            sequence: 900_003,
            entity: 2,
            kind: EventKind::CreepLeashReturning,
        },
        Event {
            frame: 1,
            sequence: 900_004,
            entity: 2,
            kind: EventKind::CreepLeashReturned,
        },
        Event {
            frame: 1,
            sequence: 900_005,
            entity: 2,
            kind: EventKind::CreepDied,
        },
        Event {
            frame: 1,
            sequence: 900_006,
            entity: 4,
            kind: EventKind::ItemDropped,
        },
    ]
}

// ---------------------------------------------------------------------------
// 任务 A：真实装配接入野营冻结段的 CPU 证据
// ---------------------------------------------------------------------------

#[test]
fn real_assembly_seeds_creep_camp_and_camps_are_non_empty() {
    let defs = synthetic_definitions();
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    assert!(defs.is_creep(CREEP_UNIT), "reason=creep_unit_registered");
    assert!(
        defs.is_ground_item(CREEP_ITEM_ALWAYS) && defs.is_ground_item(CREEP_ITEM_CHOICE),
        "reason=drop_items_registered"
    );
    assert!(
        defs.camp_script(CREEP_CAMP_SCRIPT).is_some(),
        "reason=camp_script_registered"
    );
    assert!(
        defs.drop_table(CREEP_DROP_TABLE).is_some(),
        "reason=drop_table_registered"
    );

    let sim = build_match(&defs, CREEP_CAMP_SCRIPT, true);
    let state = sim.creep_camps();
    assert_eq!(state.camps.len(), 1, "reason=one_camp_seeded");
    assert_eq!(state.creeps.len(), 2, "reason=two_creeps_seeded");
    assert!(
        state.camps.contains_key(&CREEP_CAMP_ID),
        "reason=camp_id_registered"
    );
    assert!(
        !state.camps[&CREEP_CAMP_ID].cleared,
        "reason=camp_not_cleared_yet"
    );
    let snapshot = sim.capture_snapshot();
    assert_eq!(
        snapshot.version, SNAPSHOT_VERSION,
        "reason=snapshot_version_header"
    );
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_header"
    );
    assert_eq!(
        snapshot.content_hash,
        defs.content_hash(),
        "reason=content_hash_header"
    );
}

#[test]
fn real_assembly_camp_clear_settles_rewards_drops_and_frame_hashes_match() {
    let defs = synthetic_definitions();
    let content = defs.content_hash();
    let mut a = with_hero(build_match(&defs, CREEP_CAMP_SCRIPT, true), content);
    let mut b = with_hero(build_match(&defs, CREEP_CAMP_SCRIPT, true), content);
    kill_both(&mut a, 1);
    kill_both(&mut b, 1);

    let state = a.creep_camps();
    assert_eq!(state.kill_rewards.len(), 2, "reason=one_reward_per_creep");
    assert_eq!(state.death_processed.len(), 2, "reason=idempotent_deaths");
    assert!(state.camps[&CREEP_CAMP_ID].cleared, "reason=camp_cleared");
    assert!(
        !state.ground_items.is_empty(),
        "reason=drops_in_authoritative_state"
    );
    assert!(
        !state.item_instances.is_empty(),
        "reason=item_instances_in_authoritative_state"
    );
    assert!(
        !state.rng_draws.is_empty(),
        "reason=rng_draws_in_authoritative_state"
    );
    let snapshot = a.capture_snapshot();
    assert_eq!(
        snapshot.gameplay.players[&PLAYER].gold,
        2 * CREEP_GOLD,
        "reason=gold_in_authoritative_economy"
    );
    let hero_xp = snapshot
        .positions
        .iter()
        .find(|record| record.entity == PLAYER_ENTITY)
        .and_then(|record| record.gameplay.hero)
        .map(|hero| hero.xp);
    assert_eq!(
        hero_xp,
        Some(2 * CREEP_XP),
        "reason=xp_in_authoritative_hero_state"
    );

    for frame in 0..12 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=replay_hash_frame_{frame}"
        );
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=replay_snapshot_frame_{frame}"
        );
    }
}

#[test]
fn real_assembly_unseeded_and_seeded_same_script_are_distinct() {
    let defs = synthetic_definitions();
    let script = CREEP_CAMP_SCRIPT;
    assert!(
        defs.camp_script(script).is_some(),
        "reason=same_script_available"
    );
    let mut seeded = build_match(&defs, script, true);
    let mut unseeded = build_match(&defs, script, false);
    assert_eq!(
        seeded.creep_camps().camps.len(),
        1,
        "reason=seeded_has_camp"
    );
    assert!(
        unseeded.creep_camps().camps.is_empty(),
        "reason=unseeded_has_no_camp"
    );
    assert_ne!(
        seeded.compute_state_hash(),
        unseeded.compute_state_hash(),
        "reason=seed_changes_authoritative_state"
    );

    seeded.step();
    unseeded.step();
    assert_ne!(
        seeded.compute_state_hash(),
        unseeded.compute_state_hash(),
        "reason=seed_still_distinct_after_step"
    );
}

#[test]
fn reverse_validation_missing_creep_segment_makes_seed_unreachable() {
    let full = synthetic_definitions();
    let degenerate = defs_without_creep_segment();
    assert_ne!(
        full.content_hash(),
        degenerate.content_hash(),
        "reason=creep_segment_enters_content_hash"
    );
    assert!(
        full.camp_script(CREEP_CAMP_SCRIPT).is_some(),
        "reason=formal_script_available"
    );
    assert!(
        degenerate.camp_script(CREEP_CAMP_SCRIPT).is_none(),
        "reason=degenerate_script_missing"
    );
    assert!(
        degenerate.drop_table(CREEP_DROP_TABLE).is_none(),
        "reason=degenerate_drop_table_missing"
    );
    assert!(
        !degenerate.is_creep(CREEP_UNIT),
        "reason=degenerate_creep_category_missing"
    );

    let formal = build_match(&full, CREEP_CAMP_SCRIPT, true);
    let broken = build_match(&degenerate, CREEP_CAMP_SCRIPT, true);
    assert_eq!(
        formal.creep_camps().camps.len(),
        1,
        "reason=formal_seed_reachable"
    );
    assert!(
        broken.creep_camps().camps.is_empty(),
        "reason=degenerate_seed_unreachable"
    );
    assert!(
        broken.creep_camps().creeps.is_empty(),
        "reason=degenerate_spawn_unreachable"
    );
}

// ---------------------------------------------------------------------------
// 任务 B：HUD 只读派生与权威状态隔离
// ---------------------------------------------------------------------------

#[test]
fn creep_camp_hud_derives_only_from_snapshot_and_fact_events() {
    let defs = synthetic_definitions();
    let content = defs.content_hash();
    let mut sim = with_hero(build_match(&defs, CREEP_CAMP_SCRIPT, true), content);
    kill_both(&mut sim, 1);
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let events = fact_events();

    // CPU 侧连续派生 16 帧；几何重绘隔离由 `w3-render/tests/isolation.rs` 覆盖。
    for _ in 0..16 {
        let model = CreepCampHudModel::derive(&snapshot_before, &events);
        assert_eq!(model.camp_count, 1, "reason=one_camp_derived");
        assert_eq!(model.cleared_camps, 1, "reason=camp_cleared_derived");
        assert_eq!(model.total_creeps, 2, "reason=creep_bars_derived");
        assert_eq!(model.alive_creeps, 0, "reason=no_alive_creep_derived");
        assert_eq!(model.creep_bars.len(), 2, "reason=stable_creep_bar_count");
        assert!(
            !model.drop_markers.is_empty(),
            "reason=drop_markers_derived"
        );
        assert!(model.item_instances >= 2, "reason=item_instances_derived");
        assert_eq!(model.reward_count, 2, "reason=reward_count_derived");
        assert_eq!(model.reward_xp, 2 * CREEP_XP, "reason=reward_xp_derived");
        assert_eq!(
            model.reward_gold,
            2 * CREEP_GOLD,
            "reason=reward_gold_derived"
        );
        assert_eq!(model.rng_draws, 4, "reason=rng_draws_derived");
        assert_eq!(model.seeded_events, 1, "reason=seeded_event_derived");
        assert_eq!(model.engaged_events, 1, "reason=engaged_event_derived");
        assert_eq!(
            model.leash_returning_events, 1,
            "reason=returning_event_derived"
        );
        assert_eq!(
            model.leash_returned_events, 1,
            "reason=returned_event_derived"
        );
        assert_eq!(model.creep_died_events, 1, "reason=died_event_derived");
        assert_eq!(model.item_dropped_events, 1, "reason=dropped_event_derived");
    }

    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=hud_derive_does_not_change_state_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=hud_derive_does_not_change_snapshot"
    );
}
