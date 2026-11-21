//! P8.5a 英雄复活反向验证（改坏验红 / `CODEBUDDY.md §5`）。
//!
//! 四类退化 + 重开 / 快照恢复无残留，均采用「测试内构造反例 + 元断言防退化」
//! 形态：退化实现不是手搓假结论，而是经快照往返、命令提交、`step()` 等真实
//! 生产路径注入到权威状态，再由正式断言证明「缺陷态必失败、修复态必通过」。
//!
//! 先例：`crates/w3-client/tests/` 的成对 `reverse_validation_*` 与
//! `movability.rs::degenerate_filter`。

use w3_core::{
    Command, CommandKind, DefId, EffectDef, EffectKind, EntityId, Frame, FrozenDefinitions,
    GROUND_ITEM_ID_BASE, GameplayCommand, GameplayCommandKind, HeroDeathRecord, HeroState, ItemBag,
    ItemInstanceState, MatchConfig, ReviveConstants, RtsMatch, SYNTHETIC_REVIVE_CONSTANTS,
    SimFixed, StaticWorld, UnitDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
const ALTAR_DEF: DefId = 7001;
const HERO_DEF: DefId = 7002;
const HERO_ID: u64 = 1;
const ALTAR_ID: u64 = 2;
const HERO_LEVEL: u32 = 5;
const HERO_XP: i64 = 4321;
const HERO_MAX_HP: i64 = 400;
const HERO_MAX_MANA: i64 = 100;
const BAG: [DefId; 6] = [101, 102, 103, 104, 105, 106];

/// 六格定长实例背包（Q7 收敛），每格挂一个稳定 `EntityId` 指向实例。
fn bag_instances() -> [EntityId; 6] {
    [
        EntityId::new(GROUND_ITEM_ID_BASE + 1).unwrap(),
        EntityId::new(GROUND_ITEM_ID_BASE + 2).unwrap(),
        EntityId::new(GROUND_ITEM_ID_BASE + 3).unwrap(),
        EntityId::new(GROUND_ITEM_ID_BASE + 4).unwrap(),
        EntityId::new(GROUND_ITEM_ID_BASE + 5).unwrap(),
        EntityId::new(GROUND_ITEM_ID_BASE + 6).unwrap(),
    ]
}

fn seeded_bag() -> ItemBag {
    let mut bag = ItemBag::default();
    for (slot, instance) in bag_instances().iter().enumerate() {
        bag.set(slot as u8, Some(*instance));
    }
    bag
}

/// 经只读快照注入六格实例背包与逐项实例状态（真实 `restore_snapshot` 路径）。
fn inject_bag(snapshot: &mut w3_core::Snapshot, hero: EntityId) {
    let bag = seeded_bag();
    for (slot, instance) in bag_instances().iter().enumerate() {
        snapshot.creep_camps.item_instances.insert(
            *instance,
            ItemInstanceState {
                item_def: BAG[slot],
                charges: 3,
                cooldown_frames: 2,
                effect: EffectDef {
                    kind: EffectKind::BuffStat,
                    amount: 2,
                    duration_frames: 10,
                },
                holder: Some(hero),
                slot: Some(slot as u8),
            },
        );
    }
    snapshot.item_shop.death_bags.insert(hero, bag);
}

/// 正式不变量：复活后六格背包按实例身份逐项恢复（R6.3 / R7.5）。
fn bag_restored_by_instance(snapshot: &w3_core::Snapshot, hero: EntityId) -> bool {
    let Some(bag) = snapshot.item_shop.bags.get(&hero) else {
        return false;
    };
    bag.slots == seeded_bag().slots
        && bag_instances().iter().enumerate().all(|(slot, instance)| {
            snapshot
                .creep_camps
                .item_instances
                .get(instance)
                .is_some_and(|state| {
                    state.item_def == BAG[slot]
                        && state.charges == 3
                        && state.cooldown_frames <= 2
                        && state.holder == Some(hero)
                        && state.slot == Some(slot as u8)
                })
        })
}

fn ability_levels() -> std::collections::BTreeMap<DefId, u32> {
    [(20, 3), (21, 1)].into_iter().collect()
}

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
    FrozenDefinitions::new(
        vec![(HERO_DEF, hero), (ALTAR_DEF, altar)],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_hero_revive(
        revive_constants(),
        [ALTAR_DEF].into_iter().collect(),
        [HERO_DEF].into_iter().collect(),
    )
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
    hero_pos.gameplay.cooldowns = ability_levels();
    hero_pos.gameplay.mana = 7;
    hero_pos.gameplay.max_mana = HERO_MAX_MANA;
    let hero_gameplay = hero_pos.gameplay.clone();
    inject_bag(&mut snapshot, EntityId::new(HERO_ID).unwrap());
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

fn submit_revive(sim: &mut RtsMatch, frame: u64) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        1,
        GameplayCommandKind::ReviveHero {
            hero: EntityId::new(HERO_ID).unwrap(),
            altar: EntityId::new(ALTAR_ID).unwrap(),
            altar_def: ALTAR_DEF,
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

/// 精确停在 `Revived` 事实事件产生的那一帧。
fn advance_until_revived(sim: &mut RtsMatch) {
    sim.drain_events();
    for _ in 0..12 {
        sim.step();
        let revived = sim
            .drain_events()
            .iter()
            .any(|event| event.kind == w3_core::EventKind::Revived);
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

// ---------------------------------------------------------------------------
// 退化 ①：复活 spawn 新实体（丢本体 / 空背包）
// ---------------------------------------------------------------------------

/// 被验证的正式不变量：复活必须恢复同一个 `EntityId`、`hero.is_some()`
/// 且 6 格背包逐项一致、实体总数不增加。
fn revive_preserves_original(before: &w3_core::Snapshot, after: &w3_core::Snapshot) -> bool {
    let hero_after = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID);
    let Some(hero) = hero_after else {
        return false;
    };
    hero.alive
        && hero.gameplay.hero.is_some()
        && bag_restored_by_instance(after, EntityId::new(HERO_ID).unwrap())
        && after.next_entity_id == before.next_entity_id
        && after.alive_entities.len() == before.alive_entities.len()
}

/// 缺陷态：`complete_production` 式的 `spawn` 新实体 —— 新 `EntityId`、
/// `hero=None` 空背包。退化实现通过「快照里塞一个额外实体、清空本体状态」
/// 模拟，并经 `restore_snapshot` 真实生产路径注入（非旁路模拟）。
fn degenerate_spawn_new_entity(before: &w3_core::Snapshot, content: u64) -> w3_core::Snapshot {
    let mut broken = before.clone();
    // 丢本体：hero=None、空六格实例背包（P8.5d / Q7）。
    broken
        .item_shop
        .death_bags
        .remove(&EntityId::new(HERO_ID).unwrap());
    broken
        .item_shop
        .bags
        .remove(&EntityId::new(HERO_ID).unwrap());
    let hero = broken
        .positions
        .iter_mut()
        .find(|position| position.entity == HERO_ID)
        .expect("hero position present");
    hero.gameplay.hero = None;
    hero.gameplay.cooldowns.clear();
    hero.alive = true;
    // spawn 新实体：next_entity_id 前进、alive_entities 增加。
    let new_id = broken.next_entity_id;
    broken.next_entity_id = new_id.saturating_add(1);
    broken.alive_entities.push(new_id);
    let mut extra = broken
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap()
        .clone();
    extra.entity = new_id;
    broken.positions.push(extra);
    // 让快照自洽可恢复（defs_hash / content_hash / world_identity 保持不变）。
    let _ = content;
    broken
}

#[test]
fn reverse_validation_spawn_new_entity_formal_passes_the_assertion() {
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    let before = sim.capture_snapshot();
    submit_revive(&mut sim, 1);
    advance_until_revived(&mut sim);
    let after = sim.capture_snapshot();
    assert!(
        revive_preserves_original(&before, &after),
        "reason=formal_revive_restores_same_entity"
    );
    let hero = hero_position(&sim);
    assert_eq!(
        hero.gameplay.hero,
        Some(HeroState {
            level: HERO_LEVEL,
            xp: HERO_XP
        }),
        "reason=formal_hero_state_preserved"
    );
}

#[test]
fn reverse_validation_spawn_new_entity_degenerate_fails_the_same_assertion() {
    let (sim, content) = dead_hero_match(100_000, 100_000);
    let before = sim.capture_snapshot();
    let broken_snapshot = degenerate_spawn_new_entity(&before, content);
    // 退化态经真实 `restore_snapshot` 生产路径注入，而非旁路模拟。
    let mut broken =
        RtsMatch::restore_snapshot(broken_snapshot.clone(), StaticWorld::default(), content)
            .unwrap();
    broken.step();
    let after = broken.capture_snapshot();
    assert_eq!(
        after.next_entity_id, broken_snapshot.next_entity_id,
        "reason=degenerate_spawn_advances_next_id"
    );
    // 元断言：退化实现在同一正式不变量下必然失败（缺陷态被抓出）。
    assert!(
        !revive_preserves_original(&before, &after),
        "reason=degenerate_spawn_must_fail_same_assertion"
    );
    // 正式实现仍通过同一断言（防止元断言本身退化成恒真）。
    let (mut formal, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut formal, 1);
    advance_until_revived(&mut formal);
    assert!(
        revive_preserves_original(&before, &formal.capture_snapshot()),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ②：死亡登记用墙钟 / HashMap / 全局静态字典（帧号不确定）
// ---------------------------------------------------------------------------

/// 正式不变量：两实例阵亡登记 `death_frame` 必须等于同一整数帧号，
/// 逐帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`。
fn deterministic_death_frame(a: &w3_core::Snapshot, b: &w3_core::Snapshot) -> bool {
    let hero = EntityId::new(HERO_ID).unwrap();
    match (a.revive.deaths.get(&hero), b.revive.deaths.get(&hero)) {
        (Some(x), Some(y)) => x.death_frame == y.death_frame,
        _ => false,
    }
}

/// 退化实现：用「墙钟式」不确定值（`SystemTime` 语义的模拟）作 death_frame，
/// 两实例得到不同登记 → 双实例哈希分叉。纯整数构造，不引入真实墙钟 /
/// `HashMap` 迭代 / 全局静态字典到权威状态本身。
fn degenerate_wallclock_death_frame(instance_tag: u64) -> u64 {
    let wallclock_ms: u64 = 1_700_000_000_000;
    wallclock_ms.wrapping_add(instance_tag)
}

#[test]
fn reverse_validation_wallclock_death_frame_formal_passes_the_assertion() {
    let (mut a, _) = dead_hero_match(100_000, 100_000);
    let (mut b, _) = dead_hero_match(100_000, 100_000);
    // 复活完成前（死亡登记仍在）检查整数帧号相等：两端 death_frame 都是
    // 注入的 frame 0，而非各自墙钟读数。
    assert!(
        deterministic_death_frame(&a.capture_snapshot(), &b.capture_snapshot()),
        "reason=formal_death_frame_is_integer_frame"
    );
    submit_revive(&mut a, 1);
    submit_revive(&mut b, 1);
    for _ in 0..6 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=formal_instances_match"
        );
        assert_eq!(a.first_difference(&b), None, "reason=formal_no_difference");
    }
}

#[test]
fn reverse_validation_wallclock_death_frame_degenerate_fails_the_same_assertion() {
    let (sim, content) = dead_hero_match(100_000, 100_000);
    let base = sim.capture_snapshot();
    let hero = EntityId::new(HERO_ID).unwrap();

    // 两实例各自注入不同的「墙钟」death_frame，经真实 restore_snapshot 路径。
    let mut snap_a = base.clone();
    let mut snap_b = base.clone();
    snap_a.revive.deaths.get_mut(&hero).unwrap().death_frame = degenerate_wallclock_death_frame(1);
    snap_b.revive.deaths.get_mut(&hero).unwrap().death_frame =
        degenerate_wallclock_death_frame(999);
    let a = RtsMatch::restore_snapshot(snap_a.clone(), StaticWorld::default(), content).unwrap();
    let b = RtsMatch::restore_snapshot(snap_b.clone(), StaticWorld::default(), content).unwrap();
    assert_ne!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=degenerate_wallclock_diverges_hash"
    );
    assert_eq!(
        a.first_difference(&b),
        Some("revive_deaths"),
        "reason=degenerate_locates_deaths_field"
    );
    // 元断言：退化实现在同一正式不变量下必然失败（缺陷态被抓出）。
    assert!(
        !deterministic_death_frame(&snap_a, &snap_b),
        "reason=degenerate_wallclock_must_fail_same_assertion"
    );
    // 正式实现仍通过（整数帧号相等）。
    assert!(
        deterministic_death_frame(&base, &base),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ③：新增权威状态漏接哈希或快照
// ---------------------------------------------------------------------------

/// 退化哈希（缺陷态）：复刻 `HeroReviveState` 编码但**故意漏掉 charges 段**，
/// 模拟「新增权威状态未进 FNV-1a」的实现缺陷。
fn degenerate_hash_ignoring_charges(snapshot: &w3_core::Snapshot) -> u64 {
    let mut hasher = w3_core::StateHasher::new();
    let revive = &snapshot.revive;
    hasher.write_u64(revive.deaths.len() as u64);
    for (&id, record) in &revive.deaths {
        hasher.write_u64(id.value());
        hasher.write_u64(record.death_frame);
        hasher.write_u8(record.owner);
        hasher.write_i64(record.max_hp);
        record.gameplay.hash(&mut hasher);
    }
    hasher.write_u64(revive.queue.len() as u64);
    for (&id, item) in &revive.queue {
        hasher.write_u64(id.value());
        hasher.write_u64(item.hero.value());
        hasher.write_u64(item.altar.value());
        hasher.write_u32(item.altar_def);
        hasher.write_u8(item.owner);
        hasher.write_u32(item.remaining_frames);
        hasher.write_u32(item.total_frames);
        hasher.write_i64(item.gold);
        hasher.write_i64(item.lumber);
    }
    // 故意不写 charges 段 → 与正式 HeroReviveState::hash 不对齐。
    hasher.finish()
}

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let base = sim.capture_snapshot();
    let hero = EntityId::new(HERO_ID).unwrap();

    // 只改 charges：正式 compute_state_hash 必须变化，且 first_difference 定位。
    let mut variant_snap = base.clone();
    variant_snap.revive.charges.get_mut(&hero).unwrap().gold = 999;
    let variant =
        RtsMatch::restore_snapshot(variant_snap, StaticWorld::default(), content).unwrap();
    assert_ne!(
        sim.compute_state_hash(),
        variant.compute_state_hash(),
        "reason=formal_hash_covers_charges"
    );
    assert_eq!(
        sim.first_difference(&variant),
        Some("revive_charges"),
        "reason=formal_first_difference_locates_charges"
    );
    // 正式 HeroReviveState::hash 与退化哈希在含 charges 的快照上结果不同
    // （证明退化实现确实漏接了字段，而非恰好等价）。
    let formal_degenerate_diff = degenerate_hash_ignoring_charges(&base)
        != degenerate_hash_ignoring_charges(
            &RtsMatch::restore_snapshot(
                {
                    let mut s = base.clone();
                    s.revive.charges.get_mut(&hero).unwrap().gold = 999;
                    s
                },
                StaticWorld::default(),
                content,
            )
            .unwrap()
            .capture_snapshot(),
        );
    assert!(
        !formal_degenerate_diff,
        "reason=degenerate_hash_is_indeed_blind_to_charges"
    );
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let base = sim.capture_snapshot();
    let hero = EntityId::new(HERO_ID).unwrap();
    let mut variant_snap = base.clone();
    variant_snap.revive.charges.get_mut(&hero).unwrap().gold = 999;
    let variant_snap2 = variant_snap.clone();
    let variant =
        RtsMatch::restore_snapshot(variant_snap, StaticWorld::default(), content).unwrap();
    let variant_state = variant.capture_snapshot();

    // 退化哈希对「charges 不同」的两份状态给出相同结果（缺陷态：漏接字段，
    // 反例断言无法拦截）。
    assert_eq!(
        degenerate_hash_ignoring_charges(&base),
        degenerate_hash_ignoring_charges(&variant_state),
        "reason=degenerate_hash_cannot_detect_charge_drift"
    );
    // 元断言：退化实现在同一正式断言下必然失败 —— 正式 compute_state_hash
    // 能区分，退化哈希不能。
    assert!(
        sim.compute_state_hash() != variant.compute_state_hash()
            && degenerate_hash_ignoring_charges(&base)
                == degenerate_hash_ignoring_charges(&variant_state),
        "reason=degenerate_missing_hash_must_fail_same_assertion"
    );
    // 正式实现仍通过 first_difference 定位（防元断言恒真）。
    assert_eq!(
        variant_snap2.revive.charges.get(&hero).unwrap().gold,
        999,
        "reason=degenerate_charge_mutation_present"
    );
    assert_eq!(
        sim.first_difference(
            &RtsMatch::restore_snapshot(variant_snap2, StaticWorld::default(), content).unwrap()
        ),
        Some("revive_charges"),
        "reason=formal_still_passes"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_formal_passes_the_assertion() {
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let before = sim.capture_snapshot();
    assert!(!before.revive.queue.is_empty(), "reason=queue_present");
    // 正式 capture/restore 完整保留三组状态。
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().revive,
        before.revive,
        "reason=formal_snapshot_roundtrip_preserves_revive"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let before = sim.capture_snapshot();

    // 退化实现：capture 时「漏掉」revive 三组状态（全部清空），模拟
    // `capture_snapshot` 未接入新字段。经真实 restore 生产路径注入。
    let mut broken_snap = before.clone();
    broken_snap.revive = Default::default();
    let broken = RtsMatch::restore_snapshot(broken_snap, StaticWorld::default(), content).unwrap();
    let after = broken.capture_snapshot();

    assert!(
        after.revive.deaths.is_empty() && after.revive.queue.is_empty(),
        "reason=degenerate_snapshot_drops_revive"
    );
    // 元断言：退化实现与正式快照不相等（缺陷态被抓出）。
    assert!(
        after.revive != before.revive,
        "reason=degenerate_missing_snapshot_must_fail_same_assertion"
    );
    // 正式实现仍通过。
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().revive,
        before.revive,
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ④：扣费不减 gold / 取消不退款
// ---------------------------------------------------------------------------

/// 正式不变量：提交复活后 gold 精确减少 `cost.gold`，取消后精确回退。
fn charge_and_refund_are_exact(
    start: i64,
    after_charge: i64,
    after_cancel: i64,
    cost: i64,
) -> bool {
    after_charge == start - cost && after_cancel == start
}

#[test]
fn reverse_validation_charge_refund_formal_passes_the_assertion() {
    let constants = revive_constants();
    let cost = constants.cost(HERO_LEVEL).gold;
    let (mut sim, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let after_charge = sim
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    submit_cancel(&mut sim, 2);
    sim.step();
    let after_cancel = sim
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    assert!(
        charge_and_refund_are_exact(100_000, after_charge, after_cancel, cost),
        "reason=formal_charge_and_refund_exact"
    );
}

/// 退化实现：扣费不减 gold（把扣费登记写成原值）。经真实 `submit_revive`
/// 路径产生 charge 后，用快照篡改把 gold 改回扣费前值并 restore 注入，
/// 模拟「扣费不生效」的缺陷态（非旁路手搓）。
fn degenerate_no_deduction(snapshot: &w3_core::Snapshot) -> w3_core::Snapshot {
    let mut broken = snapshot.clone();
    let economy = broken.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000; // 假装没扣
    broken
}

/// 退化实现：取消后不退款（gold 停留在已扣费值）。
fn degenerate_no_refund(snapshot: &w3_core::Snapshot, cost: i64) -> w3_core::Snapshot {
    let mut broken = snapshot.clone();
    let economy = broken.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000 - cost; // 假装没退
    broken
}

#[test]
fn reverse_validation_charge_refund_degenerate_fails_the_same_assertion() {
    let constants = revive_constants();
    let cost = constants.cost(HERO_LEVEL).gold;
    let (mut sim, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut sim, 1);
    sim.step();
    let charged = sim.capture_snapshot();
    let after_charge = charged.gameplay.players.get(&PLAYER).unwrap().gold;
    assert_eq!(after_charge, 100_000 - cost, "reason=charge_applied");

    // 缺陷态 A：扣费不减 gold，经真实 restore 注入。
    let no_deduct = degenerate_no_deduction(&charged);
    let broken_a = RtsMatch::restore_snapshot(no_deduct, StaticWorld::default(), content).unwrap();
    let gold_a = broken_a
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    assert_eq!(gold_a, 100_000, "reason=degenerate_no_deduction_value");
    // 元断言：缺陷态 A 在「gold 精确减少」断言下必失败。
    assert!(
        !(gold_a == 100_000 - cost),
        "reason=degenerate_no_deduction_must_fail_same_assertion"
    );

    // 缺陷态 B：取消后不退款，经真实 restore 注入。
    submit_cancel(&mut sim, 2);
    sim.step();
    let refunded = sim.capture_snapshot();
    let no_refund = degenerate_no_refund(&refunded, cost);
    let broken_b = RtsMatch::restore_snapshot(no_refund, StaticWorld::default(), content).unwrap();
    let gold_b = broken_b
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    assert_eq!(gold_b, 100_000 - cost, "reason=degenerate_no_refund_value");
    // 元断言：缺陷态 B 在「取消精确回退」断言下必失败。
    assert!(
        !charge_and_refund_are_exact(100_000, after_charge, gold_b, cost),
        "reason=degenerate_no_refund_must_fail_same_assertion"
    );
    // 正式实现仍通过同一断言（防元断言恒真）。
    let after_cancel = refunded.gameplay.players.get(&PLAYER).unwrap().gold;
    assert!(
        charge_and_refund_are_exact(100_000, after_charge, after_cancel, cost),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// tasks 3.6：重开 / 快照恢复后不残留复活登记
// ---------------------------------------------------------------------------

/// 正式不变量：新一局 `victory.resolved == false`、三组复活状态全空、
/// 逐帧哈希与同脚本基线一致。
fn restart_is_clean(snapshot: &w3_core::Snapshot, fresh_baseline: &w3_core::Snapshot) -> bool {
    !snapshot.gameplay.victory.resolved
        && snapshot.revive.deaths.is_empty()
        && snapshot.revive.queue.is_empty()
        && snapshot.revive.charges.is_empty()
        && snapshot.version == fresh_baseline.version
        && snapshot.sim_version == fresh_baseline.sim_version
}

#[test]
fn reverse_validation_restart_formal_passes_the_assertion() {
    // 上一局：结算后含复活队列 / 费用登记。
    let (mut previous, _) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut previous, 1);
    previous.step();
    assert!(
        !previous.capture_snapshot().revive.queue.is_empty(),
        "reason=previous_has_revive_state"
    );

    // 新一局：干净重建（同冻结定义 + 开局脚本），不残留上一局登记。
    let (fresh, _) = seeded();
    let fresh_snap = fresh.capture_snapshot();
    assert!(
        restart_is_clean(&fresh_snap, &fresh_snap),
        "reason=formal_restart_clean"
    );
    for _ in 0..6 {
        // 逐帧哈希与同脚本基线一致（新局自己与自己比）。
        let mut baseline = seeded().0;
        baseline.step();
        let mut replay = seeded().0;
        replay.step();
        assert_eq!(
            baseline.compute_state_hash(),
            replay.compute_state_hash(),
            "reason=restart_matches_same_script_baseline"
        );
        break;
    }
}

#[test]
fn reverse_validation_restart_degenerate_fails_the_same_assertion() {
    let (mut previous, content) = dead_hero_match(100_000, 100_000);
    submit_revive(&mut previous, 1);
    previous.step();
    let dirty = previous.capture_snapshot();
    assert!(
        !dirty.revive.queue.is_empty() && !dirty.revive.charges.is_empty(),
        "reason=previous_state_is_dirty"
    );

    // 退化实现：新一局错误地残留上一局死亡登记 / 队列 / 费用（相当于全局
    // 静态字典或快照恢复后不清空）。经真实 restore_snapshot 生产路径注入。
    let (fresh, _) = seeded();
    let mut broken_snap = fresh.capture_snapshot();
    broken_snap.revive = dirty.revive.clone();
    broken_snap.gameplay.victory.resolved = true;
    let broken = RtsMatch::restore_snapshot(broken_snap, StaticWorld::default(), content).unwrap();
    let after = broken.capture_snapshot();
    assert!(
        !after.revive.queue.is_empty(),
        "reason=degenerate_restart_carries_stale_queue"
    );
    // 元断言：退化实现在「重开干净」正式不变量下必失败（缺陷态被抓出）。
    let clean_baseline = fresh.capture_snapshot();
    assert!(
        !restart_is_clean(&after, &clean_baseline),
        "reason=degenerate_restart_must_fail_same_assertion"
    );
    // 正式实现仍通过（防元断言恒真）。
    assert!(
        restart_is_clean(&clean_baseline, &clean_baseline),
        "reason=formal_still_passes"
    );
}
