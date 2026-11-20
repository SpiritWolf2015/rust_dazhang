//! P8.5a 英雄复活正向验收与确定性回归（ADR 0002 / R6.3 / R11.2 / R12.4）。
//!
//! 测试全部经公开 API 构造场景：`capture_snapshot()` 取帧边界完整状态 →
//! 修改只读快照的 public 字段注入阵亡英雄（`hero` / 背包 / 技能登记 / 死亡
//! 登记）→ `restore_snapshot()` 恢复 → `submit_gameplay_command` 提交复活 →
//! `step()` 逐帧推进到队列完成。退化态不是手搓假结构，而是走快照往返这一
//! 既有确定性生产路径（先例 `determinism.rs::victory_winner_option_is_hashed_with_a_tag`）。

use std::collections::BTreeSet;

use w3_core::{
    Command, CommandKind, DefId, EntityId, EventKind, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, HeroDeathRecord, HeroState, MatchConfig, ReviveConstants, RtsMatch,
    SIM_VERSION, SNAPSHOT_VERSION, SYNTHETIC_REVIVE_CONSTANTS, SimFixed, SnapshotError,
    StaticWorld, UnitDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
/// 合成 / 原创祭坛 DefId：刻意不叫 "halt"，验证类别驱动而非名字 / 硬编码白名单。
const ALTAR_DEF: DefId = 7001;
/// 合成 / 原创英雄 DefId。
const HERO_DEF: DefId = 7002;
const HERO_ID: u64 = 1;
const ALTAR_ID: u64 = 2;
const HERO_LEVEL: u32 = 5;
const HERO_XP: i64 = 4321;
const HERO_MAX_HP: i64 = 400;
const HERO_MAX_MANA: i64 = 100;
/// 6 格背包逐项保留的合成物品 DefId `[待核]`。
const BAG: [DefId; 6] = [101, 102, 103, 104, 105, 106];
/// 技能登记 / 等级快照（DefId → 等级整数）。
fn ability_levels() -> std::collections::BTreeMap<DefId, u32> {
    [(20, 3), (21, 1)].into_iter().collect()
}

/// 缩短复活时间便于逐帧验证，其余全为合成原创编译期整数常量 `[待核]`。
fn revive_constants() -> ReviveConstants {
    let mut constants = SYNTHETIC_REVIVE_CONSTANTS;
    constants.frames_base = 3;
    constants.frames_per_level = 0;
    constants.frames_cap = 8;
    constants.gold_base = 100;
    constants.gold_per_level = 20;
    constants.gold_cap = 500;
    constants.lumber_base = 20;
    constants.lumber_per_level = 0;
    constants.lumber_cap = 100;
    constants.food_cost = 0;
    constants.hp_restore_percent = 100;
    constants.mana_restore_percent = 100;
    constants
}

/// 定义层类别登记：`ALTAR_DEF` 进祭坛集合、`HERO_DEF` 进英雄集合，
/// 复活常量段与类别集合经 `compute_defs_hash` 进 `content_hash()`。
fn hero_defs() -> FrozenDefinitions {
    let movable = SimFixed::from_int(3).unwrap().raw();
    let range = SimFixed::from_int(10).unwrap().raw();
    let hero = UnitDef {
        hp: HERO_MAX_HP,
        mana: HERO_MAX_MANA,
        speed_raw: movable,
        damage: 10,
        range_raw: range,
        supply: 1,
        train_frames: 2,
    };
    let altar = UnitDef {
        hp: 300,
        mana: 0,
        speed_raw: 0,
        damage: 0,
        range_raw: 0,
        supply: 0,
        train_frames: 2,
    };
    let altar_defs: BTreeSet<DefId> = [ALTAR_DEF].into_iter().collect();
    let hero_def_set: BTreeSet<DefId> = [HERO_DEF].into_iter().collect();
    FrozenDefinitions::new(
        vec![(HERO_DEF, hero), (ALTAR_DEF, altar)],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_hero_revive(revive_constants(), altar_defs, hero_def_set)
}

fn config(defs: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    }
}

fn spawn(x: i64, y: i64, hp: i64) -> CommandKind {
    CommandKind::Spawn {
        x: SimFixed::from_int(x).unwrap(),
        y: SimFixed::from_int(y).unwrap(),
        hp,
        speed: SimFixed::from_int(3).unwrap(),
        attack_damage: 0,
        attack_range: SimFixed::from_int(10).unwrap(),
    }
}

/// 生成英雄与祭坛实体并推进一帧，返回 `(对局, content_hash)`。
fn seeded() -> (RtsMatch, u64) {
    let defs = hero_defs();
    let content = defs.content_hash();
    let mut sim = RtsMatch::create_with_definitions(config(&defs), StaticWorld::default(), defs);
    sim.submit_command(Command::new(
        Frame::new(0),
        PLAYER,
        1,
        spawn(0, 0, HERO_MAX_HP),
    ))
    .unwrap();
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 2, spawn(5, 0, 300)))
        .unwrap();
    sim.step();
    (sim, content)
}

/// 经快照往返注入「已阵亡英雄 + 死亡登记 + 祭坛 + 充足经济」。
/// 死亡实体 `alive=false` 但 `hero` / `bag` / 技能登记全部保留在 `gameplay`，
/// 死亡登记含整数帧号 `death_frame` 与同一份状态快照。
fn dead_hero_match(gold: i64, lumber: i64) -> (RtsMatch, u64) {
    let (sim, content) = seeded();
    let mut snapshot = sim.capture_snapshot();
    let hero_pos = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == HERO_ID)
        .expect("hero position present");
    hero_pos.alive = false;
    hero_pos.hp = 0;
    hero_pos.max_hp = HERO_MAX_HP;
    hero_pos.gameplay.hero = Some(HeroState {
        level: HERO_LEVEL,
        xp: HERO_XP,
    });
    hero_pos.gameplay.bag = BAG.to_vec();
    hero_pos.gameplay.cooldowns = ability_levels();
    hero_pos.gameplay.mana = 7;
    hero_pos.gameplay.max_mana = HERO_MAX_MANA;
    let hero_gameplay = hero_pos.gameplay.clone();
    snapshot.revive.deaths.insert(
        EntityId::new(HERO_ID).unwrap(),
        HeroDeathRecord {
            death_frame: 0,
            owner: PLAYER,
            gameplay: hero_gameplay,
            max_hp: HERO_MAX_HP,
        },
    );
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = gold;
    economy.lumber = lumber;
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    (restored, content)
}

fn submit_revive(sim: &mut RtsMatch, frame: u64, altar_def: DefId) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        1,
        GameplayCommandKind::ReviveHero {
            hero: EntityId::new(HERO_ID).unwrap(),
            altar: EntityId::new(ALTAR_ID).unwrap(),
            altar_def,
        },
    ))
    .unwrap();
}

fn submit_cancel(sim: &mut RtsMatch, frame: u64) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        2,
        GameplayCommandKind::CancelRevive {
            hero: EntityId::new(HERO_ID).unwrap(),
        },
    ))
    .unwrap();
}

/// 推进到复活队列完成（帧数取常量上限的足额余量）。
fn advance_to_completion(sim: &mut RtsMatch) {
    for _ in 0..12 {
        sim.step();
    }
}

/// 精确停在 `Revived` 事实事件产生的那一帧：验证复活那一刻逐项恢复。
/// 不额外推进，避免存活英雄的 `cooldowns` 被后续帧自然递减（那是既有
/// 玩法计时器语义，与「复活恢复快照」无关）。
fn advance_until_revived(sim: &mut RtsMatch) {
    sim.drain_events();
    for _ in 0..12 {
        sim.step();
        let revived = sim
            .drain_events()
            .iter()
            .any(|event| event.kind == EventKind::Revived);
        if revived {
            return;
        }
    }
    panic!("reason=revive_never_completed");
}

fn hero_position(sim: &RtsMatch) -> w3_core::PositionRecord {
    sim.capture_snapshot()
        .positions
        .into_iter()
        .find(|position| position.entity == HERO_ID)
        .expect("hero position present")
}

/// 核心验收线：复活恢复同一个英雄实体（同一 `EntityId`、等级 / 经验 /
/// 6 格背包 / 技能登记逐项保留，置回存活、按冻结常量恢复生命 / 法力、
/// 改到祭坛位置），MUST NOT `spawn` 新实体、MUST NOT `hero=None` 空背包。
#[test]
fn revive_restores_same_entity_with_all_state() {
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    let before = sim.capture_snapshot();
    let before_next_id = before.next_entity_id;
    let before_alive = before.alive_entities.len();

    submit_revive(&mut sim, 1, ALTAR_DEF);
    advance_until_revived(&mut sim);

    let after = sim.capture_snapshot();
    let hero = hero_position(&sim);
    assert!(hero.alive, "reason=hero_restored_alive");
    assert_eq!(hero.entity, HERO_ID, "reason=same_entity_id");
    assert_eq!(
        hero.gameplay.hero,
        Some(HeroState {
            level: HERO_LEVEL,
            xp: HERO_XP
        }),
        "reason=level_and_xp_preserved"
    );
    assert_eq!(hero.gameplay.bag, BAG.to_vec(), "reason=bag_preserved");
    assert_eq!(
        hero.gameplay.cooldowns,
        ability_levels(),
        "reason=ability_levels_preserved"
    );
    assert_eq!(
        hero.hp, HERO_MAX_HP,
        "reason=hp_restored_by_frozen_constant"
    );
    assert_eq!(
        hero.gameplay.mana, HERO_MAX_MANA,
        "reason=mana_restored_by_frozen_constant"
    );
    // 改到祭坛位置（祭坛 spawn 在 (5,0)）。
    assert_eq!(
        hero.x,
        SimFixed::from_int(5).unwrap().raw(),
        "reason=altar_x"
    );
    assert_eq!(hero.y, 0, "reason=altar_y");
    // 无新增 EntityId：不 spawn 新实体。
    assert_eq!(
        after.next_entity_id, before_next_id,
        "reason=no_new_entity_id"
    );
    assert_eq!(
        after.alive_entities.len(),
        before_alive,
        "reason=entity_count_unchanged"
    );
    // 三组权威状态在完成后清空，不残留半复活登记。
    assert!(after.revive.deaths.is_empty(), "reason=death_cleared");
    assert!(after.revive.queue.is_empty(), "reason=queue_cleared");
    assert!(after.revive.charges.is_empty(), "reason=charge_cleared");
    assert!(
        !after.gameplay.victory.resolved,
        "reason=hero_death_does_not_resolve_victory"
    );
}

/// 英雄死亡不判负：仅一方 `active` 仍真时 `victory.resolved == false`。
#[test]
fn hero_death_does_not_resolve_victory() {
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    sim.step();
    let snap = sim.capture_snapshot();
    assert!(
        !snap.gameplay.victory.resolved,
        "reason=death_alone_does_not_end_match"
    );
    assert!(
        snap.gameplay.players.get(&PLAYER).unwrap().active,
        "reason=player_still_active"
    );
    // 死亡实体仍在权威状态里（不删实体）。
    assert!(
        snap.alive_entities.contains(&HERO_ID),
        "reason=dead_entity_retained"
    );
}

/// 事实事件：复活完成后恰好一次 `Revived`，事件序号在 `drain_events` 后不重置。
#[test]
fn revived_event_is_emitted_once() {
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    sim.drain_events();
    submit_revive(&mut sim, 1, ALTAR_DEF);
    advance_to_completion(&mut sim);
    let events = sim.drain_events();
    let revived = events
        .iter()
        .filter(|event| event.kind == EventKind::Revived && event.entity == HERO_ID)
        .count();
    assert_eq!(revived, 1, "reason=revived_emitted_exactly_once");
    assert!(
        sim.drain_events().is_empty(),
        "reason=no_duplicate_events_after_drain"
    );
}

/// Q3：冻结常量随等级确定性计算费用 / 时间；改常量或改祭坛登记即改变
/// `content_hash()`（经 `compute_defs_hash` 进 `defs_hash` / 快照头部）。
#[test]
fn revive_cost_is_level_driven_and_in_defs_hash() {
    let defs = hero_defs();
    let cost_l1 = defs.revive_cost(1);
    let cost_l5 = defs.revive_cost(5);
    assert!(cost_l5.gold > cost_l1.gold, "reason=gold_scales_with_level");
    assert!(
        cost_l5.frames >= cost_l1.frames,
        "reason=frames_non_decreasing_with_level"
    );
    let capped = defs.revive_cost(1000);
    assert_eq!(
        capped.gold,
        revive_constants().gold_cap,
        "reason=absolute_cap_applied"
    );

    // 改常量 → content_hash 变化。
    let mut changed = revive_constants();
    changed.gold_base += 1;
    let variant = hero_defs_with_constants(changed);
    assert_ne!(
        defs.content_hash(),
        variant.content_hash(),
        "reason=constants_in_defs_hash"
    );
    // 改祭坛类别登记 → content_hash 变化（数据驱动，非名字白名单）。
    let empty_altars = FrozenDefinitions::new(
        defs.units()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_hero_revive(revive_constants(), BTreeSet::new(), BTreeSet::new());
    assert_ne!(
        defs.content_hash(),
        empty_altars.content_hash(),
        "reason=altar_category_in_defs_hash"
    );
}

fn hero_defs_with_constants(constants: ReviveConstants) -> FrozenDefinitions {
    let defs = hero_defs();
    let units: Vec<(DefId, UnitDef)> = defs
        .units()
        .iter()
        .map(|(&id, &value)| (id, value))
        .collect();
    let altar_defs: BTreeSet<DefId> = [ALTAR_DEF].into_iter().collect();
    let hero_def_set: BTreeSet<DefId> = [HERO_DEF].into_iter().collect();
    FrozenDefinitions::new(units, vec![], vec![], vec![])
        .unwrap()
        .with_hero_revive(constants, altar_defs, hero_def_set)
}

/// 祭坛类别由定义层数据驱动：以非 "halt" 命名的 `ALTAR_DEF` 仍可复活；
/// 未登记的 DefId 被显式拒绝（MUST NOT 用建筑名字符串 / 硬编码白名单）。
#[test]
fn altar_category_is_definition_driven() {
    // ALTAR_DEF=7001 与 "halt" / 108 / 208 都无关，只要登记为祭坛即可复活。
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1, ALTAR_DEF);
    advance_to_completion(&mut sim);
    assert!(hero_position(&sim).alive, "reason=non_halt_altar_revives");

    // 未登记类别：命令被显式拒绝，不生成队列、不扣费、不免费复活。
    let (mut other, _) = dead_hero_match(100_000, 100_000);
    let gold_before = other
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    submit_revive(&mut other, 1, 9999);
    other.step();
    let snap = other.capture_snapshot();
    assert!(
        snap.revive.queue.is_empty(),
        "reason=unknown_category_rejected"
    );
    assert!(
        snap.revive.charges.is_empty(),
        "reason=unknown_category_no_charge"
    );
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).unwrap().gold,
        gold_before,
        "reason=unknown_category_no_deduction"
    );
    assert!(!hero_position(&other).alive, "reason=no_free_revive");
}

/// 费用事务：扣费精确、取消退款精确、资源不足显式拒绝；四条中止路径
/// （取消 / 祭坛被毁 / 对局失败 / 英雄再次死亡）都不生成半复活实体。
#[test]
fn charge_refund_and_insufficient_resources_are_exact() {
    let constants = revive_constants();
    let expected = constants.cost(HERO_LEVEL);

    // 提交 → 扣费精确。
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1, ALTAR_DEF);
    sim.step();
    let after_submit = sim.capture_snapshot();
    let economy = after_submit.gameplay.players.get(&PLAYER).unwrap();
    assert_eq!(
        economy.gold,
        100_000 - expected.gold,
        "reason=gold_deducted"
    );
    assert_eq!(
        economy.lumber,
        100_000 - expected.lumber,
        "reason=lumber_deducted"
    );
    assert_eq!(
        after_submit
            .revive
            .charges
            .get(&EntityId::new(HERO_ID).unwrap())
            .unwrap()
            .gold,
        expected.gold,
        "reason=charge_frozen"
    );

    // 取消 → 事务性退款精确回退，死亡登记保留（可再次复活）。
    submit_cancel(&mut sim, 2);
    sim.step();
    let snap = sim.capture_snapshot();
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).unwrap().gold,
        100_000,
        "reason=cancel_refunds_gold"
    );
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).unwrap().lumber,
        100_000,
        "reason=cancel_refunds_lumber"
    );
    assert!(snap.revive.queue.is_empty(), "reason=cancel_clears_queue");
    assert!(
        snap.revive.charges.is_empty(),
        "reason=cancel_clears_charge"
    );
    assert!(
        snap.revive
            .deaths
            .contains_key(&EntityId::new(HERO_ID).unwrap()),
        "reason=death_registry_kept_for_retry"
    );
    assert!(!hero_position(&sim).alive, "reason=no_half_revived_entity");

    // 资源不足 → 显式拒绝，不生成队列 / 不扣费 / 不免费复活。
    let (mut poor, _) = dead_hero_match(0, 0);
    submit_revive(&mut poor, 1, ALTAR_DEF);
    poor.step();
    let poor_snap = poor.capture_snapshot();
    assert!(
        poor_snap.revive.queue.is_empty(),
        "reason=insufficient_rejected"
    );
    assert!(
        poor_snap.revive.charges.is_empty(),
        "reason=insufficient_no_charge"
    );
    assert_eq!(
        poor_snap.gameplay.players.get(&PLAYER).unwrap().gold,
        0,
        "reason=insufficient_no_deduction"
    );
    assert!(!hero_position(&poor).alive, "reason=no_free_revive");
}

/// 中止路径：对局失败（victory.resolved）与祭坛被毁都触发事务性退款。
#[test]
fn defeat_and_altar_destroyed_refund_without_half_revive() {
    // 对局失败：注入已结算的 victory，队列推进时应退款。
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1, ALTAR_DEF);
    sim.step();
    let mut snap = sim.capture_snapshot();
    snap.gameplay.victory.resolved = true;
    let mut failed = RtsMatch::restore_snapshot(snap, StaticWorld::default(), content).unwrap();
    failed.step();
    let failed_snap = failed.capture_snapshot();
    assert_eq!(
        failed_snap.gameplay.players.get(&PLAYER).unwrap().gold,
        100_000,
        "reason=defeat_refunds_gold"
    );
    assert!(
        failed_snap.revive.queue.is_empty(),
        "reason=defeat_clears_queue"
    );
    assert!(
        !hero_position(&failed).alive,
        "reason=defeat_no_half_revive"
    );

    // 祭坛被毁：把祭坛实体置为 alive=false，队列推进时应退款。
    let (mut sim2, content2) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim2, 1, ALTAR_DEF);
    sim2.step();
    let mut snap2 = sim2.capture_snapshot();
    let altar = snap2
        .positions
        .iter_mut()
        .find(|position| position.entity == ALTAR_ID)
        .expect("altar position present");
    altar.alive = false;
    let mut destroyed =
        RtsMatch::restore_snapshot(snap2, StaticWorld::default(), content2).unwrap();
    destroyed.step();
    let destroyed_snap = destroyed.capture_snapshot();
    assert_eq!(
        destroyed_snap.gameplay.players.get(&PLAYER).unwrap().gold,
        100_000,
        "reason=altar_destroyed_refunds_gold"
    );
    assert!(
        destroyed_snap.revive.queue.is_empty(),
        "reason=altar_destroyed_clears_queue"
    );
    assert!(
        !hero_position(&destroyed).alive,
        "reason=altar_destroyed_no_half_revive"
    );
}

/// 确定性：双实例交错推进（含复活命令）逐帧 `compute_state_hash()` 与
/// `capture_snapshot()` 相等，`first_difference()` 为 `None`。
#[test]
fn two_instances_interleaved_match_frame_hashes() {
    let (mut a, _) = dead_hero_match(100_000, 100_000);
    let (mut b, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut a, 1, ALTAR_DEF);
    submit_revive(&mut b, 1, ALTAR_DEF);
    let mut changed = false;
    for frame in 0..16 {
        if frame % 2 == 0 {
            a.step();
            b.step();
        } else {
            b.step();
            a.step();
        }
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=frame_{frame}_hash_match field={:?}",
            a.first_difference(&b)
        );
        assert_eq!(
            a.first_difference(&b),
            None,
            "reason=frame_{frame}_no_difference"
        );
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=frame_{frame}_snapshot_match"
        );
        if frame == 1 {
            changed = true;
        }
    }
    assert!(changed, "reason=scenario_actually_advanced");
    assert!(hero_position(&a).alive, "reason=both_revived");
}

/// 确定性：帧 N 捕获 / 销毁 / 恢复后继续到帧 M 逐帧一致，且复活不重复
/// （`Revived` 恰好一次），三组状态不残留 / 不丢失（R12.4）。
#[test]
fn snapshot_roundtrip_continues_identically() {
    let (mut original, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut original, 1, ALTAR_DEF);
    original.step(); // 队列进行中捕获
    let snapshot = original.capture_snapshot();
    assert!(
        !snapshot.revive.queue.is_empty(),
        "reason=queue_in_snapshot"
    );
    assert!(
        !snapshot.revive.charges.is_empty(),
        "reason=charge_in_snapshot"
    );
    assert!(
        !snapshot.revive.deaths.is_empty(),
        "reason=death_in_snapshot"
    );
    let mut restored =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        original.compute_state_hash(),
        restored.compute_state_hash(),
        "reason=restore_preserves_hash"
    );
    assert_eq!(
        restored.capture_snapshot().revive,
        snapshot.revive,
        "reason=restore_preserves_revive_state"
    );
    original.drain_events();
    restored.drain_events();
    for frame in 0..12 {
        original.step();
        restored.step();
        assert_eq!(
            original.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
    }
    let events = restored.drain_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == EventKind::Revived)
            .count(),
        1,
        "reason=no_duplicate_revive"
    );
    assert!(hero_position(&restored).alive, "reason=restored_revived");
}

/// 确定性：回放同脚本逐帧哈希与原始一致，重放不双重注入复活命令。
#[test]
fn replay_matches_original() {
    let (mut original, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut original, 1, ALTAR_DEF);
    let expected: Vec<u64> = (0..16)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();

    let (mut replay, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut replay, 1, ALTAR_DEF);
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.compute_state_hash(),
            *hash,
            "reason=replay_frame_{frame}"
        );
    }
}

/// 三组新增权威状态 MUST 同时进入状态哈希、快照与 `first_difference()`：
/// 只改死亡登记一项，两端哈希不同、差异诊断能定位到 `revive_deaths`。
#[test]
fn new_revive_fields_are_hashed_and_snapshot_safe() {
    let (mut a, content) = dead_hero_match(100_000, 100_000);
    let mut snap_b = a.capture_snapshot();
    // 仅改三组状态之一：死亡登记的 death_frame。
    let hero = EntityId::new(HERO_ID).unwrap();
    let record = snap_b.revive.deaths.get_mut(&hero).unwrap();
    record.death_frame = 7;
    let b = RtsMatch::restore_snapshot(snap_b, StaticWorld::default(), content).unwrap();
    assert_ne!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=deaths_in_state_hash"
    );
    assert_eq!(
        a.first_difference(&b),
        Some("revive_deaths"),
        "reason=first_difference_locates_deaths"
    );
    assert_ne!(
        a.capture_snapshot(),
        b.capture_snapshot(),
        "reason=deaths_in_snapshot"
    );

    // 队列与费用登记同样进哈希 / 差异诊断。
    submit_revive(&mut a, 1, ALTAR_DEF);
    a.step();
    let mut snap_c = a.capture_snapshot();
    snap_c.revive.queue.get_mut(&hero).unwrap().remaining_frames = 99;
    let c = RtsMatch::restore_snapshot(snap_c, StaticWorld::default(), content).unwrap();
    assert_eq!(
        a.first_difference(&c),
        Some("revive_queue"),
        "reason=first_difference_locates_queue"
    );
    let mut snap_d = a.capture_snapshot();
    snap_d.revive.charges.get_mut(&hero).unwrap().gold = 999;
    let d = RtsMatch::restore_snapshot(snap_d, StaticWorld::default(), content).unwrap();
    assert_eq!(
        a.first_difference(&d),
        Some("revive_charges"),
        "reason=first_difference_locates_charges"
    );
}

/// Q1：旧版本快照被 `SnapshotError` 明确拒绝，且 `SNAPSHOT_VERSION` /
/// `SIM_VERSION` 已随 P8.5c bump 到 6（v3 / v4 / v5 均属旧版本，不提供迁移）。
#[test]
fn old_v3_snapshot_is_rejected() {
    assert_eq!(SNAPSHOT_VERSION, 6, "reason=snapshot_version_6");
    assert_eq!(SIM_VERSION, 6, "reason=sim_version_6");
    let (sim, content) = dead_hero_match(100_000, 100_000);
    let mut old = sim.capture_snapshot();
    old.version = 3;
    assert!(matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 3 })
    ));
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 3;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 3,
            expected: 6
        })
    ));
}

/// 真实生产路径死亡登记：英雄经 `Attack` 命令由攻击结算致死，
/// `register_hero_death` 写入的 `death_frame` 必须是整数逻辑帧号（而非墙钟
/// 毫秒 / 渲染帧率），并发 `HeroDied` 事实事件、不删实体、不判负。
/// 场景只改 public 快照字段注入英雄本体与攻击者伤害，再走真实 `Attack`。
#[test]
fn hero_death_registration_uses_integer_frame_number() {
    let (sim, content) = seeded();
    let mut snapshot = sim.capture_snapshot();
    let hero = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == HERO_ID)
        .expect("hero position present");
    hero.alive = true;
    hero.hp = 1;
    hero.max_hp = HERO_MAX_HP;
    hero.gameplay.hero = Some(HeroState {
        level: HERO_LEVEL,
        xp: HERO_XP,
    });
    hero.gameplay.bag = BAG.to_vec();
    // 把第二个实体当作攻击者：注入足够伤害与射程（走真实 Attack 结算）。
    let attacker = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == ALTAR_ID)
        .expect("attacker position present");
    attacker.attack_damage = 999;
    attacker.attack_range = SimFixed::from_int(100).unwrap().raw();
    let mut sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    sim.drain_events();

    // 在整数帧 1 提交攻击，step 时该帧结算致死。
    const ATTACK_FRAME: u64 = 1;
    sim.submit_command(Command::new(
        Frame::new(ATTACK_FRAME),
        PLAYER,
        1,
        CommandKind::Attack {
            attacker: EntityId::new(ALTAR_ID).unwrap(),
            target: EntityId::new(HERO_ID).unwrap(),
        },
    ))
    .unwrap();
    sim.step();

    let after = sim.capture_snapshot();
    let hero_id = EntityId::new(HERO_ID).unwrap();
    let record = after
        .revive
        .deaths
        .get(&hero_id)
        .expect("death registered through real attack path");
    // 整数逻辑帧号：等于攻击结算帧，且量级远小于墙钟毫秒。
    assert_eq!(
        record.death_frame, ATTACK_FRAME,
        "reason=death_frame_is_integer_logic_frame"
    );
    assert!(
        record.death_frame < 1000,
        "reason=death_frame_is_not_wallclock_millis"
    );
    assert_eq!(record.owner, PLAYER, "reason=death_owner_recorded");
    // 英雄本体状态快照随死亡登记保留。
    assert_eq!(
        record.gameplay.hero,
        Some(HeroState {
            level: HERO_LEVEL,
            xp: HERO_XP
        }),
        "reason=death_snapshot_keeps_hero"
    );
    assert_eq!(
        record.gameplay.bag,
        BAG.to_vec(),
        "reason=death_snapshot_keeps_bag"
    );
    // 不删实体、不判负、发事实事件。
    assert!(
        after.alive_entities.contains(&HERO_ID),
        "reason=entity_retained"
    );
    assert!(!after.gameplay.victory.resolved, "reason=not_defeat");
    assert!(
        sim.drain_events()
            .iter()
            .any(|event| event.kind == EventKind::HeroDied && event.entity == HERO_ID),
        "reason=hero_died_event"
    );
}

/// ADR 0002 §5：只读视图与事件消费不改写权威状态（含复活三组状态）。
#[test]
fn readonly_view_and_events_do_not_change_authoritative_state() {
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1, ALTAR_DEF);
    sim.step();
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let _view = sim.read_view();
    let _view = sim.read_view();
    let _events = sim.drain_events();
    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=read_only_does_not_change_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=read_only_does_not_change_snapshot"
    );
}
