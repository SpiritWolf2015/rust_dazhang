//! P8.5b 主城升级反向验证（改坏验红 / `CODEBUDDY.md §5` / Q8）。
//!
//! 四类退化 + 重开 / 快照恢复无残留，均采用「测试内构造反例 + 元断言防退化」
//! 形态：退化实现不是手搓假结论，而是经快照往返、命令提交、`step()` 等真实
//! 生产路径注入到权威状态，再由正式断言证明「缺陷态必失败、修复态必通过」。
//! 每一类都成对提供 `formal_passes_the_assertion` / `degenerate_fails_the_same_assertion`，
//! 后者内部还含「正式实现仍通过同一断言」的元断言，防止元断言退化成恒真。
//!
//! 四类退化对应 Q8：
//! ① tier 权威状态漏接 `compute_state_hash()` 或 `capture_snapshot()`；
//! ② 未升级仍接受未解锁 `Train` / `Research` 的「假升级」；
//! ③ 升级费用不扣 gold / 取消不退款，或升级进度用墙钟 / `HashMap` 迭代顺序；
//! ④ 升级命令漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发。
//!
//! 先例：`reverse_validation_hero_revive.rs` / `p8-dual-faction` 成对 `reverse_validation_*`。

use std::collections::BTreeMap;

use w3_core::{
    Command, CommandKind, DefId, EntityId, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, RtsMatch, SYNTHETIC_TOWN_HALL_CONSTANTS, SimFixed,
    StateHasher, StaticWorld, TownHallConstants, TownHallTier, UnitDef, UpgradeDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
const TOWN_HALL_DEF: DefId = 8101;
const TIER2_UNIT: DefId = 8111;
const TIER2_UPGRADE: DefId = 8121;
const TOWN_HALL_ID: u64 = 1;
const PRODUCER_ID: u64 = 2;

fn town_hall_constants() -> TownHallConstants {
    let mut constants = SYNTHETIC_TOWN_HALL_CONSTANTS;
    constants.frames = 3;
    constants.tier2_gold = 100;
    constants.tier2_lumber = 40;
    constants.price_delta_gold = 60;
    constants.price_delta_lumber = 30;
    constants
}

fn requirements() -> BTreeMap<DefId, TownHallTier> {
    [
        (TIER2_UNIT, TownHallTier::T2),
        (TIER2_UPGRADE, TownHallTier::T2),
    ]
    .into_iter()
    .collect()
}

fn unit(train_frames: u32, hp: i64) -> UnitDef {
    UnitDef {
        hp,
        mana: 0,
        speed_raw: SimFixed::from_int(3).unwrap().raw(),
        damage: 0,
        range_raw: 0,
        supply: 1,
        train_frames,
    }
}

/// 正式定义：登记主城类别 + tier 需求链（数据驱动解锁）。
fn formal_defs() -> FrozenDefinitions {
    let units = vec![(TOWN_HALL_DEF, unit(2, 500)), (TIER2_UNIT, unit(5, 80))];
    let upgrades = vec![(
        TIER2_UPGRADE,
        UpgradeDef {
            damage_mod: 1,
            hp_mod: 2,
            research_frames: 5,
        },
    )];
    FrozenDefinitions::new(units, vec![], vec![], upgrades)
        .unwrap()
        .with_town_hall_upgrade(
            town_hall_constants(),
            [TOWN_HALL_DEF].into_iter().collect(),
            requirements(),
        )
}

/// 退化定义 ②/④ 的反例：不登记 tier 需求链 / 主城类别，模拟「假升级」——
/// 内核仍全量可训练（C4 旧行为）或命令校验被旁路。经 `create_with_definitions`
/// 这一真实生产路径注入，非旁路手搓。
fn degenerate_defs_without_requirements() -> FrozenDefinitions {
    let units = vec![(TOWN_HALL_DEF, unit(2, 500)), (TIER2_UNIT, unit(5, 80))];
    let upgrades = vec![(
        TIER2_UPGRADE,
        UpgradeDef {
            damage_mod: 1,
            hp_mod: 2,
            research_frames: 5,
        },
    )];
    // 主城类别仍登记，但 tier 需求链为空 → 二线单位 / 科技无需 tier 即放行。
    FrozenDefinitions::new(units, vec![], vec![], upgrades)
        .unwrap()
        .with_town_hall_upgrade(
            town_hall_constants(),
            [TOWN_HALL_DEF].into_iter().collect(),
            BTreeMap::new(),
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

fn seeded_with(defs: &FrozenDefinitions) -> (RtsMatch, u64) {
    let content = defs.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(defs), StaticWorld::default(), defs.clone());
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 1, spawn(0, 0, 500)))
        .unwrap();
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 2, spawn(4, 0, 300)))
        .unwrap();
    sim.step();
    (sim, content)
}

fn seeded() -> (RtsMatch, u64) {
    seeded_with(&formal_defs())
}

fn funded_match_with(defs: &FrozenDefinitions, gold: i64, lumber: i64) -> (RtsMatch, u64) {
    let (sim, content) = seeded_with(defs);
    let mut snapshot = sim.capture_snapshot();
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = gold;
    economy.lumber = lumber;
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    (restored, content)
}

fn funded_match(gold: i64, lumber: i64) -> (RtsMatch, u64) {
    funded_match_with(&formal_defs(), gold, lumber)
}

fn submit_upgrade(sim: &mut RtsMatch, frame: u64, target_tier: TownHallTier, cancel: bool) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        1,
        GameplayCommandKind::UpgradeTownHall {
            town_hall: EntityId::new(TOWN_HALL_ID).unwrap(),
            town_hall_def: TOWN_HALL_DEF,
            target_tier,
            cancel,
        },
    ))
    .unwrap();
}

fn submit_train(sim: &mut RtsMatch, frame: u64, unit_def: DefId) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        1,
        GameplayCommandKind::Train {
            producer: EntityId::new(PRODUCER_ID).unwrap(),
            unit_def,
            is_building: false,
        },
    ))
    .unwrap();
}

fn submit_research(sim: &mut RtsMatch, frame: u64, upgrade: DefId, sequence: u32) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        GameplayCommandKind::Research { upgrade },
    ))
    .unwrap();
}

fn current_frame(sim: &RtsMatch) -> u64 {
    sim.read_view().tick
}

fn noop_cancel_train(sequence: u32) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(1),
        PLAYER,
        sequence,
        GameplayCommandKind::CancelTrain {
            producer: EntityId::new(PRODUCER_ID).unwrap(),
            index: u32::MAX,
        },
    )
}

fn producer_production_len(sim: &RtsMatch) -> usize {
    sim.capture_snapshot()
        .positions
        .into_iter()
        .find(|position| position.entity == PRODUCER_ID)
        .expect("producer present")
        .gameplay
        .production
        .len()
}

fn research_is_none(sim: &RtsMatch) -> bool {
    sim.capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .and_then(|economy| economy.research)
        .is_none()
}

fn tier_of_town_hall(sim: &RtsMatch) -> Option<TownHallTier> {
    sim.capture_snapshot()
        .town_hall
        .tiers
        .get(&EntityId::new(TOWN_HALL_ID).unwrap())
        .copied()
}

// ---------------------------------------------------------------------------
// 退化 ①：tier 权威状态漏接 compute_state_hash() 或 capture_snapshot()
// ---------------------------------------------------------------------------

/// 退化哈希（缺陷态）：复刻 `TownHallUpgradeState::hash` 编码但**故意漏掉
/// tiers 段**，模拟「新增权威状态未进 FNV-1a」的实现缺陷。
fn degenerate_hash_ignoring_tiers(snapshot: &w3_core::Snapshot) -> u64 {
    let mut hasher = StateHasher::new();
    let state = &snapshot.town_hall;
    // 故意不写 tiers 段（长度 + 每项），只编 queue / charges。
    hasher.write_u64(state.queue.len() as u64);
    for (&id, item) in &state.queue {
        hasher.write_u64(id.value());
        hasher.write_u64(item.town_hall.value());
        hasher.write_u32(item.town_hall_def);
        hasher.write_u8(item.owner);
        hasher.write_u8(item.from_tier.code());
        hasher.write_u8(item.target_tier.code());
        hasher.write_u32(item.remaining_frames);
        hasher.write_u32(item.total_frames);
        hasher.write_i64(item.gold);
        hasher.write_i64(item.lumber);
    }
    hasher.write_u64(state.charges.len() as u64);
    for (&id, charge) in &state.charges {
        hasher.write_u64(id.value());
        hasher.write_u8(charge.owner);
        hasher.write_i64(charge.gold);
        hasher.write_i64(charge.lumber);
    }
    hasher.finish()
}

/// 正式不变量：改动 tiers 一段，两端 `compute_state_hash()` 必不同，且
/// `first_difference()` 定位到 `town_hall_tiers`。
fn tiers_are_hashed(a: &RtsMatch, b: &RtsMatch) -> bool {
    a.compute_state_hash() != b.compute_state_hash()
        && a.first_difference(b) == Some("town_hall_tiers")
}

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let base = sim.capture_snapshot();
    let hall = EntityId::new(TOWN_HALL_ID).unwrap();

    // 只改 tiers：正式 compute_state_hash 变化、first_difference 定位。
    let mut variant_snap = base.clone();
    variant_snap.town_hall.tiers.insert(hall, TownHallTier::T2);
    let variant =
        RtsMatch::restore_snapshot(variant_snap, StaticWorld::default(), content).unwrap();
    assert!(
        tiers_are_hashed(&sim, &variant),
        "reason=formal_hash_covers_tiers"
    );
    // 退化哈希对「tiers 不同」的两份状态给出相同结果（证明退化实现确实漏字段）。
    assert_eq!(
        degenerate_hash_ignoring_tiers(&base),
        degenerate_hash_ignoring_tiers(&variant.capture_snapshot()),
        "reason=degenerate_hash_is_indeed_blind_to_tiers"
    );
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let base = sim.capture_snapshot();
    let hall = EntityId::new(TOWN_HALL_ID).unwrap();
    let mut variant_snap = base.clone();
    variant_snap.town_hall.tiers.insert(hall, TownHallTier::T2);
    let variant_snap2 = variant_snap.clone();
    let variant =
        RtsMatch::restore_snapshot(variant_snap, StaticWorld::default(), content).unwrap();

    // 缺陷态：改用退化哈希（漏接 tiers）表达「tiers 是否进哈希」这一正式断言。
    // 退化实现下两端哈希相等 → 正式断言 `tiers_are_hashed` 必失败。
    let degenerate_assertion = degenerate_hash_ignoring_tiers(&base)
        != degenerate_hash_ignoring_tiers(&variant.capture_snapshot());
    assert!(
        !degenerate_assertion,
        "reason=degenerate_hash_cannot_detect_tier_drift"
    );
    // 元断言：正式实现能区分、退化实现不能区分（缺陷态被抓出）。
    assert!(
        tiers_are_hashed(&sim, &variant) && !degenerate_assertion,
        "reason=degenerate_missing_hash_must_fail_same_assertion"
    );
    // 正式实现仍通过同一断言（防元断言恒真）。
    let formal =
        RtsMatch::restore_snapshot(variant_snap2, StaticWorld::default(), content).unwrap();
    assert!(
        tiers_are_hashed(&sim, &formal),
        "reason=formal_still_passes"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_formal_passes_the_assertion() {
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let before = sim.capture_snapshot();
    assert_eq!(before.town_hall.queue.len(), 1, "reason=queue_present");
    // 正式 capture/restore 完整保留三组状态。
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().town_hall,
        before.town_hall,
        "reason=formal_snapshot_roundtrip_preserves_town_hall"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let before = sim.capture_snapshot();

    // 退化实现：capture 时「漏掉」主城升级三组状态（全部清空），模拟
    // `capture_snapshot` 未接入新字段。经真实 restore 生产路径注入。
    let mut broken_snap = before.clone();
    broken_snap.town_hall = Default::default();
    let broken = RtsMatch::restore_snapshot(broken_snap, StaticWorld::default(), content).unwrap();
    let after = broken.capture_snapshot();
    assert!(
        after.town_hall.queue.is_empty() && after.town_hall.charges.is_empty(),
        "reason=degenerate_snapshot_drops_town_hall"
    );
    // 元断言：退化实现与正式快照不相等（缺陷态被抓出）。
    assert!(
        after.town_hall != before.town_hall,
        "reason=degenerate_missing_snapshot_must_fail_same_assertion"
    );
    // 正式实现仍通过。
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().town_hall,
        before.town_hall,
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ②：未升级仍接受未解锁 Train / Research 的「假升级」
// ---------------------------------------------------------------------------

/// 正式不变量：T1（未升级）时二线单位 / 科技必须被显式拒绝——
/// 不生成生产队列项、不生成研究队列项。
fn locked_commands_are_rejected(sim: &RtsMatch) -> bool {
    producer_production_len(sim) == 0 && research_is_none(sim) && tier_of_town_hall(sim).is_none()
}

#[test]
fn reverse_validation_fake_upgrade_formal_passes_the_assertion() {
    // 正式定义（登记需求链）：提交二线 Train + Research，推进后应被拒。
    let (mut sim, _) = funded_match(100_000, 100_000);
    let (mut baseline, _) = funded_match(100_000, 100_000);
    let frame = current_frame(&sim);
    submit_train(&mut sim, frame, TIER2_UNIT);
    submit_research(&mut sim, frame, TIER2_UPGRADE, 2);
    baseline
        .submit_gameplay_command(noop_cancel_train(1))
        .unwrap();
    baseline
        .submit_gameplay_command(noop_cancel_train(2))
        .unwrap();
    sim.step();
    baseline.step();
    assert!(
        locked_commands_are_rejected(&sim),
        "reason=formal_rejects_locked_commands"
    );
    assert_eq!(
        sim.compute_state_hash(),
        baseline.compute_state_hash(),
        "reason=formal_rejection_has_no_state_drift"
    );
}

#[test]
fn reverse_validation_fake_upgrade_degenerate_fails_the_same_assertion() {
    // 退化实现：定义层不登记 tier 需求链（C4「全量可训练」旧行为），
    // 经 `create_with_definitions` 真实生产路径注入，再提交相同命令脚本。
    let degenerate = degenerate_defs_without_requirements();
    let (mut sim, _) = funded_match_with(&degenerate, 100_000, 100_000);
    let frame = current_frame(&sim);
    submit_train(&mut sim, frame, TIER2_UNIT);
    submit_research(&mut sim, frame, TIER2_UPGRADE, 2);
    sim.step();
    // 退化态：未升级即接受未解锁 Train / Research → 正式不变量失败。
    assert!(
        !locked_commands_are_rejected(&sim),
        "reason=degenerate_fake_upgrade_accepts_locked_commands"
    );
    // 元断言：正式实现（登记需求链）在同一命令脚本下通过同一断言。
    let (mut formal, _) = funded_match(100_000, 100_000);
    let formal_frame = current_frame(&formal);
    submit_train(&mut formal, formal_frame, TIER2_UNIT);
    submit_research(&mut formal, formal_frame, TIER2_UPGRADE, 2);
    formal.step();
    assert!(
        locked_commands_are_rejected(&formal),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ③：升级费用不扣 gold / 取消不退款，或进度用墙钟 / HashMap 顺序
// ---------------------------------------------------------------------------

/// 正式不变量：提交升级后 gold 精确减少 `cost.gold`，取消后精确回退。
fn charge_and_refund_are_exact(
    start: i64,
    after_charge: i64,
    after_cancel: i64,
    cost: i64,
) -> bool {
    after_charge == start - cost && after_cancel == start
}

fn gold_of(sim: &RtsMatch) -> i64 {
    sim.capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold
}

#[test]
fn reverse_validation_charge_refund_formal_passes_the_assertion() {
    let constants = town_hall_constants();
    let cost = constants.cost(TownHallTier::T2).unwrap().gold;
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let after_charge = gold_of(&sim);
    let cancel_frame = current_frame(&sim);
    submit_upgrade(&mut sim, cancel_frame, TownHallTier::T2, true);
    sim.step();
    let after_cancel = gold_of(&sim);
    assert!(
        charge_and_refund_are_exact(100_000, after_charge, after_cancel, cost),
        "reason=formal_charge_and_refund_exact"
    );
}

/// 退化实现：扣费不减 gold（把经济写回扣费前值）。经真实 `restore_snapshot`
/// 生产路径注入，模拟「扣费事务未生效」的缺陷态（非旁路手搓）。
fn degenerate_no_deduction(snapshot: &w3_core::Snapshot) -> w3_core::Snapshot {
    let mut broken = snapshot.clone();
    let economy = broken.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    broken
}

/// 退化实现：取消后不退款（gold 停留在已扣费值）。
fn degenerate_no_refund(snapshot: &w3_core::Snapshot, cost: i64) -> w3_core::Snapshot {
    let mut broken = snapshot.clone();
    let economy = broken.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000 - cost;
    broken
}

#[test]
fn reverse_validation_charge_refund_degenerate_fails_the_same_assertion() {
    let constants = town_hall_constants();
    let cost = constants.cost(TownHallTier::T2).unwrap().gold;
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let charged = sim.capture_snapshot();
    let after_charge = charged.gameplay.players.get(&PLAYER).unwrap().gold;
    assert_eq!(after_charge, 100_000 - cost, "reason=charge_applied");

    // 缺陷态 A：扣费不减 gold，经真实 restore 注入。
    let no_deduct = degenerate_no_deduction(&charged);
    let broken_a = RtsMatch::restore_snapshot(no_deduct, StaticWorld::default(), content).unwrap();
    let gold_a = gold_of(&broken_a);
    assert_eq!(gold_a, 100_000, "reason=degenerate_no_deduction_value");
    assert!(
        !(gold_a == 100_000 - cost),
        "reason=degenerate_no_deduction_must_fail_same_assertion"
    );

    // 缺陷态 B：取消后不退款，经真实 restore 注入。
    let cancel_frame = current_frame(&sim);
    submit_upgrade(&mut sim, cancel_frame, TownHallTier::T2, true);
    sim.step();
    let refunded = sim.capture_snapshot();
    let no_refund = degenerate_no_refund(&refunded, cost);
    let broken_b = RtsMatch::restore_snapshot(no_refund, StaticWorld::default(), content).unwrap();
    let gold_b = gold_of(&broken_b);
    assert_eq!(gold_b, 100_000 - cost, "reason=degenerate_no_refund_value");
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

/// 正式不变量：升级进度用整数帧，两实例 `remaining_frames` 必相等、
/// 逐帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`。
fn integer_frame_progress_is_deterministic(a: &RtsMatch, b: &RtsMatch) -> bool {
    let ra = a
        .capture_snapshot()
        .town_hall
        .queue
        .values()
        .next()
        .map(|item| item.remaining_frames);
    let rb = b
        .capture_snapshot()
        .town_hall
        .queue
        .values()
        .next()
        .map(|item| item.remaining_frames);
    ra == rb && a.compute_state_hash() == b.compute_state_hash() && a.first_difference(b) == None
}

/// 退化实现：墙钟式不确定 `remaining_frames`（两实例各自不同），模拟
/// 「升级进度用墙钟毫秒 / HashMap 迭代顺序」的缺陷态。整数构造，不引入
/// 真实墙钟到权威状态本身，仅经快照注入体现分叉结果。
fn degenerate_wallclock_frames(snapshot: &w3_core::Snapshot, tag: u32) -> w3_core::Snapshot {
    let mut broken = snapshot.clone();
    let hall = EntityId::new(TOWN_HALL_ID).unwrap();
    if let Some(item) = broken.town_hall.queue.get_mut(&hall) {
        let wallclock_ms: u32 = 1_700_000_000;
        item.remaining_frames = wallclock_ms.wrapping_add(tag);
    }
    broken
}

#[test]
fn reverse_validation_wallclock_progress_formal_passes_the_assertion() {
    let (mut a, _) = funded_match(100_000, 100_000);
    let (mut b, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut a, 1, TownHallTier::T2, false);
    submit_upgrade(&mut b, 1, TownHallTier::T2, false);
    a.step();
    b.step();
    assert!(
        integer_frame_progress_is_deterministic(&a, &b),
        "reason=formal_integer_frames_match"
    );
}

#[test]
fn reverse_validation_wallclock_progress_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let base = sim.capture_snapshot();
    // 两实例各自注入不同的「墙钟」remaining_frames，经真实 restore 路径。
    let snap_a = degenerate_wallclock_frames(&base, 1);
    let snap_b = degenerate_wallclock_frames(&base, 999);
    let a = RtsMatch::restore_snapshot(snap_a, StaticWorld::default(), content).unwrap();
    let b = RtsMatch::restore_snapshot(snap_b, StaticWorld::default(), content).unwrap();
    assert_ne!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=degenerate_wallclock_diverges_hash"
    );
    assert_eq!(
        a.first_difference(&b),
        Some("town_hall_queue"),
        "reason=degenerate_locates_queue_field"
    );
    // 元断言：退化实现在同一正式不变量下必失败。
    assert!(
        !integer_frame_progress_is_deterministic(&a, &b),
        "reason=degenerate_wallclock_must_fail_same_assertion"
    );
    // 正式实现仍通过。
    let restored =
        RtsMatch::restore_snapshot(base.clone(), StaticWorld::default(), content).unwrap();
    let formal = RtsMatch::restore_snapshot(base, StaticWorld::default(), content).unwrap();
    assert!(
        integer_frame_progress_is_deterministic(&restored, &formal),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ④：升级命令漏接 hash 编码 / 校验 / 穷尽分发
// ---------------------------------------------------------------------------

/// 退化命令哈希（缺陷态）：复刻 `hash_gameplay_command` 但对
/// `UpgradeTownHall`（码 13）**故意不编 target_tier / cancel**，模拟命令
/// 编码漏接字段（两个不同动作的命令哈希碰撞 → 重放歧义）。
fn degenerate_command_hash(command: &GameplayCommand) -> u64 {
    let mut hasher = StateHasher::new();
    hasher.write_u64(command.execute_frame.value());
    hasher.write_u8(command.player_id);
    hasher.write_u32(command.sequence);
    hasher.write_u64(command.arrival());
    if let GameplayCommandKind::UpgradeTownHall {
        town_hall,
        town_hall_def,
        ..
    } = command.kind
    {
        hasher.write_u8(13);
        hasher.write_u64(town_hall.value());
        hasher.write_u32(town_hall_def);
        // 故意不编 target_tier.code() / cancel → start 与 cancel 命令碰撞。
    }
    hasher.finish()
}

fn start_command() -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(1),
        PLAYER,
        1,
        GameplayCommandKind::UpgradeTownHall {
            town_hall: EntityId::new(TOWN_HALL_ID).unwrap(),
            town_hall_def: TOWN_HALL_DEF,
            target_tier: TownHallTier::T2,
            cancel: false,
        },
    )
}

fn cancel_command() -> GameplayCommand {
    let mut command = start_command();
    command.kind = GameplayCommandKind::UpgradeTownHall {
        town_hall: EntityId::new(TOWN_HALL_ID).unwrap(),
        town_hall_def: TOWN_HALL_DEF,
        target_tier: TownHallTier::T2,
        cancel: true,
    };
    command
}

/// 正式不变量：start 与 cancel 两个不同语义的命令必须哈希可区分（唯一编码），
/// 且都能经公开提交入口被接收（穷尽分发 + 校验覆盖新变体）。
fn command_encoding_is_unique(start: &GameplayCommand, cancel: &GameplayCommand) -> bool {
    let mut q_start = w3_core::GameplayCommandQueue::new();
    let mut q_cancel = w3_core::GameplayCommandQueue::new();
    q_start.submit(*start).is_ok() && q_cancel.submit(*cancel).is_ok() && {
        let mut h_start = StateHasher::new();
        let mut h_cancel = StateHasher::new();
        q_start.hash(&mut h_start);
        q_cancel.hash(&mut h_cancel);
        h_start.finish() != h_cancel.finish()
    }
}

#[test]
fn reverse_validation_missing_command_encoding_formal_passes_the_assertion() {
    let start = start_command();
    let cancel = cancel_command();
    assert!(
        command_encoding_is_unique(&start, &cancel),
        "reason=formal_command_encoding_is_unique"
    );
    // 穷尽分发覆盖：提交 start 命令真的建立升级队列（不是 no-op）。
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    assert_eq!(
        sim.capture_snapshot().town_hall.queue.len(),
        1,
        "reason=formal_command_is_dispatched"
    );
    // 校验覆盖：`EntityId::NONE` 的新变体被 validate_gameplay_entities 拒绝。
    let mut invalid = start;
    invalid.kind = GameplayCommandKind::UpgradeTownHall {
        town_hall: EntityId::NONE,
        town_hall_def: TOWN_HALL_DEF,
        target_tier: TownHallTier::T2,
        cancel: false,
    };
    let mut q = w3_core::GameplayCommandQueue::new();
    assert_eq!(
        q.submit(invalid),
        Err(w3_core::GameplayCommandError::InvalidEntity),
        "reason=formal_validation_rejects_none_entity"
    );
}

#[test]
fn reverse_validation_missing_command_encoding_degenerate_fails_the_same_assertion() {
    let start = start_command();
    let cancel = cancel_command();
    // 退化实现：漏编 target_tier / cancel 的哈希 → start 与 cancel 碰撞。
    let degenerate_unique = degenerate_command_hash(&start) != degenerate_command_hash(&cancel);
    assert!(
        !degenerate_unique,
        "reason=degenerate_encoding_collides_start_and_cancel"
    );
    // 元断言：正式实现能区分、退化实现不能区分（缺陷态被抓出）。
    assert!(
        command_encoding_is_unique(&start, &cancel) && !degenerate_unique,
        "reason=degenerate_missing_encoding_must_fail_same_assertion"
    );
    // 正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        command_encoding_is_unique(&start, &cancel),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// tasks 3.4：重开 / 快照恢复后不残留 tier / 队列 / 费用登记
// ---------------------------------------------------------------------------

/// 正式不变量：新一局 `victory.resolved == false`、三组主城升级状态全空，
/// 版本与基线一致。
fn restart_is_clean(snapshot: &w3_core::Snapshot, fresh_baseline: &w3_core::Snapshot) -> bool {
    !snapshot.gameplay.victory.resolved
        && snapshot.town_hall.tiers.is_empty()
        && snapshot.town_hall.queue.is_empty()
        && snapshot.town_hall.charges.is_empty()
        && snapshot.version == fresh_baseline.version
        && snapshot.sim_version == fresh_baseline.sim_version
}

#[test]
fn reverse_validation_restart_formal_passes_the_assertion() {
    // 上一局：含升级队列 / 费用登记。
    let (mut previous, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut previous, 1, TownHallTier::T2, false);
    previous.step();
    assert_eq!(
        previous.capture_snapshot().town_hall.queue.len(),
        1,
        "reason=previous_has_town_hall_state"
    );

    // 新一局：干净重建（同冻结定义 + 开局脚本），不残留上一局登记。
    let (fresh, _) = seeded();
    let fresh_snap = fresh.capture_snapshot();
    assert!(
        restart_is_clean(&fresh_snap, &fresh_snap),
        "reason=formal_restart_clean"
    );

    // 快照恢复后从帧 N+1 继续、不重复升级：帧 N 捕获（队列进行中）、
    // 恢复，逐帧哈希与原实例一致。
    let (mut original, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut original, 1, TownHallTier::T2, false);
    original.step();
    let snap = original.capture_snapshot();
    let mut restored = RtsMatch::restore_snapshot(snap, StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().town_hall,
        original.capture_snapshot().town_hall,
        "reason=restore_preserves_town_hall_state"
    );
    for frame in 0..8 {
        original.step();
        restored.step();
        assert_eq!(
            original.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
    }
}

#[test]
fn reverse_validation_restart_degenerate_fails_the_same_assertion() {
    let (mut previous, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut previous, 1, TownHallTier::T2, false);
    previous.step();
    let dirty = previous.capture_snapshot();
    assert_eq!(
        dirty.town_hall.queue.len(),
        1,
        "reason=previous_state_is_dirty"
    );
    assert_eq!(
        dirty.town_hall.charges.len(),
        1,
        "reason=previous_charge_is_dirty"
    );

    // 退化实现：新一局错误地残留上一局 tier / 队列 / 费用（相当于全局静态
    // 字典或快照恢复后不清空）。经真实 restore_snapshot 生产路径注入。
    let (fresh, _) = seeded();
    let mut broken_snap = fresh.capture_snapshot();
    broken_snap.town_hall = dirty.town_hall.clone();
    broken_snap
        .town_hall
        .tiers
        .insert(EntityId::new(TOWN_HALL_ID).unwrap(), TownHallTier::T3);
    broken_snap.gameplay.victory.resolved = true;
    let broken = RtsMatch::restore_snapshot(broken_snap, StaticWorld::default(), content).unwrap();
    let after = broken.capture_snapshot();
    assert!(
        !after.town_hall.queue.is_empty(),
        "reason=degenerate_restart_carries_stale_queue"
    );
    // 元断言：退化实现在「重开干净」正式不变量下必失败。
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
