//! P8.5c 反向验证（改坏验红 / `CODEBUDDY.md §5` / Q9 形态①）。
//!
//! 每一类退化都成对提供 `formal_passes_the_assertion` /
//! `degenerate_fails_the_same_assertion`，后者内部含「正式实现仍通过同一
//! 断言」的元断言防退化；退化实现 MUST 经 `restore_snapshot` /
//! `create_with_definitions` / `submit_gameplay_command` / `step()` 等真实
//! 生产路径注入，非旁路手搓结论。覆盖 tasks 7.3 五类退化：
//! ① 野怪 / 地面物品 / 随机码消费序列漏接哈希或快照；
//! ② 掉落用墙钟 / `HashMap` 迭代顺序 / `randomize()` 等非确定随机源；
//! ③ 击杀无经验金币 / 表现层反写 xp 或 gold / 掉落幂等退化；
//! ④ 掉落随机码消费序列不进哈希 / 快照 / `first_difference()`；
//! ⑤ 命令 / 事件漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发。
//! 另含 tasks 3.4「重开 / 快照恢复无残留」与 R8.5「无限追击」退化。
//!
//! 先例：`reverse_validation_hero_revive.rs` / `reverse_validation_town_hall_upgrade.rs`。

mod creep_common;

use creep_common::*;
use w3_core::{
    EntityId, EventKind, Frame, FrozenDefinitions, GameplayCommand, GameplayCommandKind,
    LeashPhase, NEUTRAL_OWNER, NeutralCategory, RtsMatch, SimFixed, Snapshot, StateHasher,
    StaticWorld,
};

/// 退化哈希（缺陷态）：模拟「新增权威状态漏接 FNV-1a」——把指定字段清空后
/// 再走 `CreepCampState::hash` 的正式编码，等价于该段完全未进入哈希。
fn degenerate_hash_omit(snapshot: &Snapshot, omit: &str) -> u64 {
    let state = degenerate_omit_state(snapshot, omit);
    let mut hasher = StateHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

/// 退化「漏接字段」：把指定权威状态段清空（等价于该段未进哈希 / 快照）。
/// 既可复刻哈希编码，也可经真实 `restore_snapshot` 注入缺陷态。
fn degenerate_omit_state(snapshot: &Snapshot, omit: &str) -> w3_core::CreepCampState {
    let mut state = snapshot.creep_camps.clone();
    match omit {
        "creeps" => state.creeps.clear(),
        "ground_items" => state.ground_items.clear(),
        "rng_draws" => state.rng_draws.clear(),
        "rng_state" => state.rng_state = 0,
        "kill_rewards" => state.kill_rewards.clear(),
        "neutral" => state.neutral.clear(),
        _ => panic!("creep_reverse: reason=unknown_omit_field value={omit}"),
    }
    state
}

/// 正式不变量：改动指定字段，两端 `compute_state_hash()` 必不同，
/// 且 `first_difference()` 定位到对应差异字段。
fn creep_field_is_hashed(a: &RtsMatch, b: &RtsMatch, field: &str) -> bool {
    a.compute_state_hash() != b.compute_state_hash() && a.first_difference(b) == Some(field)
}

/// 构造「正式 vs 仅目标字段不同」的两端（经真实快照恢复生产路径注入）。
fn variant_pair(field: &str, content: u64) -> (RtsMatch, RtsMatch) {
    let (mut sim, _) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let base = sim.capture_snapshot();
    let mut changed = base.clone();
    let creep = creep_ids()[0];
    match field {
        "creep_creeps" => {
            changed.creep_camps.creeps.get_mut(&creep).unwrap().phase = LeashPhase::Returning;
        }
        "creep_ground_items" => {
            let id = *changed.creep_camps.ground_items.keys().next().unwrap();
            changed.creep_camps.ground_items.get_mut(&id).unwrap().x += 1;
        }
        "creep_rng_draws" => {
            changed.creep_camps.rng_draws.get_mut(&0).unwrap().roll += 1;
        }
        "creep_rng_state" => changed.creep_camps.rng_state ^= 1,
        "creep_kill_rewards" => {
            changed
                .creep_camps
                .kill_rewards
                .get_mut(&creep)
                .unwrap()
                .gold += 1;
        }
        "creep_neutral" => {
            changed.creep_camps.neutral.clear();
        }
        _ => panic!("creep_reverse: reason=unknown_field value={field}"),
    }
    let formal = RtsMatch::restore_snapshot(base, StaticWorld::default(), content).unwrap();
    let variant = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();
    (formal, variant)
}

// ---------------------------------------------------------------------------
// 退化 ①：野怪 / 地面物品 / 消费序列漏接 compute_state_hash()
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let content = config(&defs()).content_hash;
    for field in [
        "creep_creeps",
        "creep_ground_items",
        "creep_rng_draws",
        "creep_rng_state",
        "creep_kill_rewards",
        "creep_neutral",
    ] {
        let (formal, variant) = variant_pair(field, content);
        assert!(
            creep_field_is_hashed(&formal, &variant, field),
            "reason=formal_hash_covers_{field}"
        );
    }
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    for (field, omit) in [
        ("creep_creeps", "creeps"),
        ("creep_ground_items", "ground_items"),
        ("creep_rng_draws", "rng_draws"),
        ("creep_rng_state", "rng_state"),
        ("creep_kill_rewards", "kill_rewards"),
        ("creep_neutral", "neutral"),
    ] {
        let (formal, variant) = variant_pair(field, content);
        // 缺陷态：退化哈希对该字段视而不见 → 两端相等 → 正式断言必失败。
        let degenerate_detects = degenerate_hash_omit(&formal.capture_snapshot(), omit)
            != degenerate_hash_omit(&variant.capture_snapshot(), omit);
        assert!(
            !degenerate_detects,
            "reason=degenerate_hash_blind_to_{omit}"
        );
        // 元断言：正式实现能区分、退化实现不能区分（缺陷态被抓出）。
        assert!(
            creep_field_is_hashed(&formal, &variant, field) && !degenerate_detects,
            "reason=degenerate_missing_hash_must_fail_same_assertion_{omit}"
        );
        // 正式实现仍通过同一断言（防元断言恒真）。
        assert!(
            creep_field_is_hashed(&formal, &variant, field),
            "reason=formal_still_passes_{omit}"
        );

        // 退化实现真经生产路径注入：把「漏接字段」的快照喂给
        // `restore_snapshot`，两端重建后退化态无法区分差异字段。
        let mut degenerate_a = formal.capture_snapshot();
        degenerate_a.creep_camps = degenerate_omit_state(&degenerate_a, omit);
        let mut degenerate_b = variant.capture_snapshot();
        degenerate_b.creep_camps = degenerate_omit_state(&degenerate_b, omit);
        let rebuilt_a =
            RtsMatch::restore_snapshot(degenerate_a, StaticWorld::default(), content).unwrap();
        let rebuilt_b =
            RtsMatch::restore_snapshot(degenerate_b, StaticWorld::default(), content).unwrap();
        assert!(
            rebuilt_a.compute_state_hash() == rebuilt_b.compute_state_hash()
                && rebuilt_a.first_difference(&rebuilt_b) == None,
            "reason=degenerate_restore_cannot_distinguish_{omit}"
        );
    }
}

// ---------------------------------------------------------------------------
// 退化 ①（续）：漏接 capture_snapshot() / restore_snapshot()
// ---------------------------------------------------------------------------

/// 正式不变量：帧 N 捕获 / 恢复后野怪 / 地面物品 / 消费序列 / 收益登记完整保留。
fn snapshot_roundtrip_is_complete(before: &Snapshot, after: &Snapshot) -> bool {
    before.creep_camps.camps == after.creep_camps.camps
        && before.creep_camps.creeps == after.creep_camps.creeps
        && before.creep_camps.neutral == after.creep_camps.neutral
        && before.creep_camps.ground_items == after.creep_camps.ground_items
        && before.creep_camps.item_instances == after.creep_camps.item_instances
        && before.creep_camps.rng_draws == after.creep_camps.rng_draws
        && before.creep_camps.rng_state == after.creep_camps.rng_state
        && before.creep_camps.kill_rewards == after.creep_camps.kill_rewards
        && before.creep_camps.death_processed == after.creep_camps.death_processed
}

#[test]
fn reverse_validation_missing_snapshot_field_formal_passes_the_assertion() {
    let (mut sim, content) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let before = sim.capture_snapshot();
    assert!(
        !before.creep_camps.ground_items.is_empty(),
        "reason=state_present"
    );
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    let after = restored.capture_snapshot();
    assert!(
        snapshot_roundtrip_is_complete(&before, &after),
        "reason=formal_snapshot_roundtrip_complete"
    );
    // 恢复后继续推进，逐帧哈希与未中断实例一致（R12.4）。
    let mut uninterrupted =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    for frame in 0..8 {
        sim.step();
        uninterrupted.step();
        assert_eq!(
            sim.compute_state_hash(),
            uninterrupted.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
    }
}

#[test]
fn reverse_validation_missing_snapshot_field_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let before = sim.capture_snapshot();

    // 退化实现：capture 时漏掉整组野怪权威状态。经真实 restore 生产路径注入。
    let mut broken_snap = before.clone();
    broken_snap.creep_camps = Default::default();
    let broken = RtsMatch::restore_snapshot(broken_snap, StaticWorld::default(), content).unwrap();
    let after = broken.capture_snapshot();
    assert!(
        after.creep_camps.camps.is_empty() && after.creep_camps.ground_items.is_empty(),
        "reason=degenerate_snapshot_drops_creep_state"
    );
    assert!(
        !snapshot_roundtrip_is_complete(&before, &after),
        "reason=degenerate_missing_snapshot_must_fail_same_assertion"
    );
    // 正式实现仍通过（防元断言恒真）。
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        snapshot_roundtrip_is_complete(&before, &restored.capture_snapshot()),
        "reason=formal_still_passes"
    );

    // 另一退化：只漏接消费序列 / 地面物品的一段，也必被正式不变量抓出。
    let mut partial = before.clone();
    partial.creep_camps.rng_draws.clear();
    partial.creep_camps.ground_items.clear();
    let partial_restored =
        RtsMatch::restore_snapshot(partial, StaticWorld::default(), content).unwrap();
    assert!(
        !snapshot_roundtrip_is_complete(&before, &partial_restored.capture_snapshot()),
        "reason=degenerate_partial_snapshot_must_fail"
    );
}

// ---------------------------------------------------------------------------
// 退化 ②：掉落用墙钟 / HashMap 迭代顺序 / randomize() 等非确定随机源
// ---------------------------------------------------------------------------

/// 正式不变量：同 seed / 同冻结定义 / 同脚本的两个实例，掉落与消费序列逐帧一致。
fn same_seed_drops_match(a: &RtsMatch, b: &RtsMatch) -> bool {
    let state_a = a.creep_camps();
    let state_b = b.creep_camps();
    state_a.rng_draws == state_b.rng_draws
        && state_a.ground_items == state_b.ground_items
        && state_a.rng_state == state_b.rng_state
        && a.compute_state_hash() == b.compute_state_hash()
        && a.first_difference(b) == None
}

/// 退化实现：用宿主相关的「墙钟标签」改写事件驱动随机流初态（等价于
/// `randomize()` / `Time.get_ticks_msec()` / 无序集合枚举），经真实
/// `restore_snapshot` 注入；两个实例给出不同标签即模拟两端独立随机源。
fn degenerate_wallclock_rng(sim: &RtsMatch, content: u64, tag: u64) -> RtsMatch {
    let mut snapshot = sim.capture_snapshot();
    snapshot.creep_camps.rng_state = snapshot.creep_camps.rng_state.wrapping_add(tag);
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

#[test]
fn reverse_validation_non_deterministic_rng_formal_passes_the_assertion() {
    let (mut a, _) = seeded_match();
    let (mut b, _) = seeded_match();
    kill_both_and_settle(&mut a, 1);
    kill_both_and_settle(&mut b, 1);
    assert!(
        same_seed_drops_match(&a, &b),
        "reason=formal_same_seed_drops_match"
    );
    for _ in 0..8 {
        a.step();
        b.step();
        assert!(same_seed_drops_match(&a, &b), "reason=formal_frame_match");
    }
}

#[test]
fn reverse_validation_non_deterministic_rng_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let (seed_a, _) = seeded_match();
    let (seed_b, _) = seeded_match();
    // 退化：两端随机流被不同墙钟标签污染。
    let mut a = degenerate_wallclock_rng(&seed_a, content, 0);
    let mut b = degenerate_wallclock_rng(&seed_b, content, 1);
    kill_both_and_settle(&mut a, 1);
    kill_both_and_settle(&mut b, 1);
    let degenerate_matches = same_seed_drops_match(&a, &b);
    assert!(!degenerate_matches, "reason=wallclock_tags_diverge_drops");
    // 元断言：正式实现能一致、退化实现不能一致（缺陷态被抓出）。
    let (mut formal_a, _) = seeded_match();
    let (mut formal_b, _) = seeded_match();
    kill_both_and_settle(&mut formal_a, 1);
    kill_both_and_settle(&mut formal_b, 1);
    assert!(
        same_seed_drops_match(&formal_a, &formal_b) && !degenerate_matches,
        "reason=degenerate_wallclock_must_fail_same_assertion"
    );
    assert!(
        same_seed_drops_match(&formal_a, &formal_b),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ③：击杀无经验金币 / 表现层反写 xp 或 gold / 掉落幂等退化
// ---------------------------------------------------------------------------

/// 正式不变量：击杀收益精确入账（R6.1 / R8.3），且只结算一次（幂等）。
fn rewards_are_exact(sim: &RtsMatch) -> bool {
    let state = sim.creep_camps();
    let snapshot = sim.capture_snapshot();
    let hero_xp = snapshot
        .positions
        .iter()
        .find(|record| record.entity == 1)
        .and_then(|record| record.gameplay.hero.map(|hero| hero.xp))
        .unwrap_or(0);
    state.kill_rewards.values().all(|reward| {
        reward.gold == CREEP_GOLD
            && reward.xp_credited.get(&EntityId::new(1).unwrap()) == Some(&CREEP_XP)
    }) && snapshot.gameplay.players.get(&PLAYER).map(|e| e.gold) == Some(CREEP_GOLD)
        && hero_xp == CREEP_XP
        && state.ground_items.len() == 2
        && state.rng_draws.len() == 2
}

fn hero_kill_match() -> (RtsMatch, u64) {
    let (sim, content) = seeded_match();
    let mut sim = with_hero(sim, content);
    // 只杀一个野怪，便于精确断言单次收益。
    attack(&mut sim, 1, 1, 2, 1);
    sim.step();
    (sim, content)
}

#[test]
fn reverse_validation_missing_rewards_formal_passes_the_assertion() {
    let (sim, _) = hero_kill_match();
    assert!(rewards_are_exact(&sim), "reason=formal_rewards_exact");
    // 重复投递死亡通知不产生第二次结算（Q6 / tasks 4.2）。
    let mut sim = sim;
    sim.step();
    assert!(rewards_are_exact(&sim), "reason=formal_no_resettlement");
}

#[test]
fn reverse_validation_missing_rewards_degenerate_fails_the_same_assertion() {
    let (sim, content) = hero_kill_match();

    // 退化 ③a：击杀无收益 / 表现层反写 xp·gold（把权威字段清零后
    // 经真实 restore 注入）。
    let mut broken = sim.capture_snapshot();
    for reward in broken.creep_camps.kill_rewards.values_mut() {
        reward.gold = 0;
        reward.xp_credited.clear();
    }
    broken.gameplay.players.get_mut(&PLAYER).unwrap().gold = 0;
    for record in &mut broken.positions {
        if let Some(hero) = record.gameplay.hero.as_mut() {
            hero.xp = 0;
        }
    }
    let no_reward = RtsMatch::restore_snapshot(broken, StaticWorld::default(), content).unwrap();
    assert!(
        !rewards_are_exact(&no_reward),
        "reason=degenerate_no_reward_must_fail"
    );

    // 退化 ③b：掉落幂等退化（清空「已处理死亡」登记并重复投递通知），
    // 真实生产路径下产生第二次掉落与随机码消费。
    let mut missing_registry = sim.capture_snapshot();
    missing_registry.creep_camps.death_processed.clear();
    let mut duplicate =
        RtsMatch::restore_snapshot(missing_registry, StaticWorld::default(), content).unwrap();
    assert!(duplicate.notify_creep_death(creep_ids()[0], EntityId::new(1).unwrap(), PLAYER));
    duplicate.step();
    assert!(
        !rewards_are_exact(&duplicate),
        "reason=degenerate_duplicate_drop_must_fail"
    );
    assert!(
        duplicate.creep_camps().rng_draws.len() > 2,
        "reason=degenerate_double_rng_consumption"
    );

    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    assert!(rewards_are_exact(&sim), "reason=formal_still_passes");
}

// ---------------------------------------------------------------------------
// 退化 ④：消费序列不进哈希 / 快照 / first_difference()
// ---------------------------------------------------------------------------

/// 正式不变量：消费序列改动 → 哈希变化、首差异字段定位、
/// 快照往返完整保留消费序列与流状态。
fn rng_sequence_is_hashed_and_snapshotted(
    a: &RtsMatch,
    b: &RtsMatch,
    before: &Snapshot,
    after: &Snapshot,
) -> bool {
    a.compute_state_hash() != b.compute_state_hash()
        && a.first_difference(b) == Some("creep_rng_draws")
        && before.creep_camps.rng_draws == after.creep_camps.rng_draws
        && before.creep_camps.rng_state == after.creep_camps.rng_state
        && before.creep_camps.next_rng_seq == after.creep_camps.next_rng_seq
}

#[test]
fn reverse_validation_missing_rng_sequence_formal_passes_the_assertion() {
    let (mut sim, content) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let before = sim.capture_snapshot();
    let mut changed = before.clone();
    changed.creep_camps.rng_draws.get_mut(&0).unwrap().rng_after ^= 1;
    let variant = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        rng_sequence_is_hashed_and_snapshotted(
            &sim,
            &variant,
            &before,
            &restored.capture_snapshot()
        ),
        "reason=formal_rng_sequence_covered"
    );
}

#[test]
fn reverse_validation_missing_rng_sequence_degenerate_fails_the_same_assertion() {
    let (mut sim, content) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let before = sim.capture_snapshot();
    let mut changed = before.clone();
    changed.creep_camps.rng_draws.get_mut(&0).unwrap().rng_after ^= 1;
    let variant = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();

    // 退化：消费序列段完全漏接哈希 → 两端退化哈希相等。
    let degenerate_detects = degenerate_hash_omit(&before, "rng_draws")
        != degenerate_hash_omit(&variant.capture_snapshot(), "rng_draws");
    assert!(!degenerate_detects, "reason=degenerate_blind_to_rng_draws");
    // 退化：快照漏接消费序列 → 往返后丢失。
    let mut broken = before.clone();
    broken.creep_camps.rng_draws.clear();
    broken.creep_camps.next_rng_seq = 0;
    let broken_restored =
        RtsMatch::restore_snapshot(broken, StaticWorld::default(), content).unwrap();
    assert!(
        !rng_sequence_is_hashed_and_snapshotted(
            &sim,
            &variant,
            &before,
            &broken_restored.capture_snapshot()
        ),
        "reason=degenerate_missing_rng_sequence_must_fail"
    );
    // 正式实现仍通过（防元断言恒真）。
    let formal_restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        rng_sequence_is_hashed_and_snapshotted(
            &sim,
            &variant,
            &before,
            &formal_restored.capture_snapshot()
        ),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ⑤：命令 / 事件漏接编码 / 校验 / 穷尽分发
// ---------------------------------------------------------------------------

/// 退化命令哈希（缺陷态）：复刻 `hash_gameplay_command` 但对
/// `SeedCreepCamp`（码 14）故意不编 `script`，模拟编码漏接字段。
fn degenerate_command_hash(command: &GameplayCommand) -> u64 {
    let mut hasher = StateHasher::new();
    hasher.write_u64(command.execute_frame.value());
    hasher.write_u8(command.player_id);
    hasher.write_u32(command.sequence);
    hasher.write_u64(command.arrival());
    if let GameplayCommandKind::SeedCreepCamp { camp, .. } = command.kind {
        hasher.write_u8(14);
        hasher.write_u32(camp);
        // 故意不编 script → 两个不同冻结脚本的命令哈希碰撞。
    }
    hasher.finish()
}

fn command_a() -> GameplayCommand {
    seed_camp(0, CAMP_ID, 41)
}

fn command_b() -> GameplayCommand {
    seed_camp(0, CAMP_ID, 42)
}

/// 正式不变量：两个不同冻结脚本的命令编码唯一、都能被接收、
/// 且 `SeedCreepCamp` 真的被穷尽分发（营地装配，不是 no-op）。
fn creep_command_encoding_is_unique(a: &GameplayCommand, b: &GameplayCommand) -> bool {
    let mut q_a = w3_core::GameplayCommandQueue::new();
    let mut q_b = w3_core::GameplayCommandQueue::new();
    q_a.submit(*a).is_ok()
        && q_b.submit(*b).is_ok()
        && {
            let mut h_a = StateHasher::new();
            let mut h_b = StateHasher::new();
            q_a.hash(&mut h_a);
            q_b.hash(&mut h_b);
            h_a.finish() != h_b.finish()
        }
        && q_a.freeze_frame(Frame::new(0)).len() == 1
        && q_b.freeze_frame(Frame::new(0)).len() == 1
}

/// 退化事实事件（缺陷态）：复用既有 `Died`、不新增 `EventKind` 变体，
/// 表现层无法区分击杀 / 掉落（C9 / Q5 被否备选）。
fn degenerate_event_kind(kind: EventKind) -> EventKind {
    match kind {
        EventKind::CreepCampSeeded
        | EventKind::CreepEngaged
        | EventKind::CreepLeashReturning
        | EventKind::CreepLeashReturned
        | EventKind::CreepDied
        | EventKind::ItemDropped => EventKind::Died,
        other => other,
    }
}

/// 正式不变量：新增事实事件变体齐备且互不混淆（只读表现层可消费）。
fn creep_fact_events_are_distinct(events: &[w3_core::Event]) -> bool {
    events
        .iter()
        .filter(|event| event.kind == EventKind::CreepDied)
        .count()
        == 2
        && events
            .iter()
            .filter(|event| event.kind == EventKind::ItemDropped)
            .count()
            == 4
        && events
            .iter()
            .any(|event| event.kind == EventKind::CreepCampSeeded)
        && EventKind::CreepDied != EventKind::Died
        && EventKind::ItemDropped != EventKind::Died
}

#[test]
fn reverse_validation_missing_command_encoding_formal_passes_the_assertion() {
    let a = command_a();
    let b = command_b();
    assert!(
        creep_command_encoding_is_unique(&a, &b),
        "reason=formal_command_encoding_unique"
    );
    // 校验覆盖：0 号营地 / 脚本被拒。
    let mut queue = w3_core::GameplayCommandQueue::new();
    assert_eq!(
        queue.submit(seed_camp(0, 0, CAMP_SCRIPT)),
        Err(w3_core::GameplayCommandError::InvalidEntity)
    );
    // 穷尽分发：正式实现真的装配营地。
    let (sim, _) = seeded_match();
    assert_eq!(sim.creep_camps().camps.len(), 1, "reason=dispatched");
    // 事实事件变体齐备。
    let (mut sim, _) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let mut events = sim.drain_events();
    let prior = sim.drain_events();
    events.extend(prior);
    assert!(
        creep_fact_events_are_distinct(&events),
        "reason=formal_fact_events_distinct"
    );
}

#[test]
fn reverse_validation_missing_command_encoding_degenerate_fails_the_same_assertion() {
    let a = command_a();
    let b = command_b();
    // 退化：漏编 script → 两条不同语义命令碰撞。
    let degenerate_unique = degenerate_command_hash(&a) != degenerate_command_hash(&b);
    assert!(!degenerate_unique, "reason=degenerate_encoding_collides");
    assert!(
        creep_command_encoding_is_unique(&a, &b) && !degenerate_unique,
        "reason=degenerate_missing_encoding_must_fail_same_assertion"
    );
    assert!(
        creep_command_encoding_is_unique(&a, &b),
        "reason=formal_still_passes"
    );

    // 退化事件面：复用 Died 不新增变体 → 击杀 / 掉落事实不可区分。
    let (mut sim, _) = seeded_match();
    let seed_events = sim.drain_events();
    kill_both_and_settle(&mut sim, 1);
    let mut events = seed_events;
    events.extend(sim.drain_events());
    assert!(
        creep_fact_events_are_distinct(&events),
        "reason=formal_events_present"
    );
    let collapsed: Vec<EventKind> = events
        .iter()
        .map(|event| degenerate_event_kind(event.kind))
        .collect();
    assert!(
        collapsed
            .iter()
            .all(|kind| *kind != EventKind::CreepDied && *kind != EventKind::ItemDropped),
        "reason=degenerate_events_collapsed"
    );
    let collapsed_events: Vec<w3_core::Event> = events
        .iter()
        .map(|event| {
            let mut copy = *event;
            copy.kind = degenerate_event_kind(event.kind);
            copy
        })
        .collect();
    assert!(
        !creep_fact_events_are_distinct(&collapsed_events),
        "reason=degenerate_missing_events_must_fail_same_assertion"
    );
    assert!(
        creep_fact_events_are_distinct(&events),
        "reason=formal_still_passes_events"
    );
}

// ---------------------------------------------------------------------------
// tasks 3.4：重开 / 快照恢复后不残留野怪 / 掉落 / 消费序列 / 收益
// ---------------------------------------------------------------------------

/// 正式不变量：新一局 `victory.resolved == false`、全部野怪权威状态为空、
/// 版本与基线一致（R11.5 / R12.4）。
fn restart_is_clean(snapshot: &Snapshot, baseline: &Snapshot) -> bool {
    !snapshot.gameplay.victory.resolved
        && snapshot.creep_camps.camps.is_empty()
        && snapshot.creep_camps.creeps.is_empty()
        && snapshot.creep_camps.neutral.is_empty()
        && snapshot.creep_camps.ground_items.is_empty()
        && snapshot.creep_camps.item_instances.is_empty()
        && snapshot.creep_camps.rng_draws.is_empty()
        && snapshot.creep_camps.kill_rewards.is_empty()
        && snapshot.creep_camps.death_processed.is_empty()
        && snapshot.version == baseline.version
        && snapshot.sim_version == baseline.sim_version
}

#[test]
fn reverse_validation_restart_formal_passes_the_assertion() {
    let (mut previous, _) = seeded_match();
    kill_both_and_settle(&mut previous, 1);
    let dirty = previous.capture_snapshot();
    assert!(
        !dirty.creep_camps.ground_items.is_empty(),
        "reason=previous_dirty"
    );

    let fresh = fresh_match();
    let clean = fresh.capture_snapshot();
    assert!(
        restart_is_clean(&clean, &clean),
        "reason=formal_restart_clean"
    );

    // 帧 N 捕获 / 恢复后继续到 M，不重复掉落（Q6 / R12.4）。
    let content = config(&defs()).content_hash;
    let mut original =
        RtsMatch::restore_snapshot(dirty.clone(), StaticWorld::default(), content).unwrap();
    let mut restored = RtsMatch::restore_snapshot(dirty, StaticWorld::default(), content).unwrap();
    for frame in 0..8 {
        original.step();
        restored.step();
        assert_eq!(
            original.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
        assert_eq!(
            restored.creep_camps().ground_items.len(),
            original.creep_camps().ground_items.len(),
            "reason=no_duplicate_drops_frame_{frame}"
        );
    }
}

#[test]
fn reverse_validation_restart_degenerate_fails_the_same_assertion() {
    let (mut previous, content) = seeded_match();
    kill_both_and_settle(&mut previous, 1);
    let dirty = previous.capture_snapshot();

    // 退化实现：重开残留上一局野怪 / 掉落 / 消费序列 / 收益登记，
    // 并错误地设置 `victory.resolved`。经真实 restore 生产路径注入。
    let fresh = fresh_match();
    let clean_baseline = fresh.capture_snapshot();
    let mut broken = clean_baseline.clone();
    broken.creep_camps = dirty.creep_camps.clone();
    broken.gameplay.victory.resolved = true;
    let broken_sim = RtsMatch::restore_snapshot(broken, StaticWorld::default(), content).unwrap();
    let after = broken_sim.capture_snapshot();
    assert!(
        !after.creep_camps.ground_items.is_empty(),
        "reason=stale_drops"
    );
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

// ---------------------------------------------------------------------------
// R8.5：无限追出营地的退化实现（D3 / tasks 5.1）
// ---------------------------------------------------------------------------

/// 正式不变量：被引出 leash 半径之外的野怪 MUST 切 RETURNING 并最终归巢。
fn leash_is_bounded(sim: &RtsMatch, creep: EntityId) -> bool {
    let state = sim.creep_camps();
    let Some(creep_state) = state.creeps.get(&creep) else {
        return false;
    };
    let record = sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep.value())
        .cloned();
    let Some(record) = record else {
        return false;
    };
    let dx = (record.x - HOME_X) as i128;
    let dy = (record.y - HOME_Y) as i128;
    let inside_leash = dx * dx + dy * dy <= (2 * SCALE as i128) * (2 * SCALE as i128);
    // 要么已归巢（IDLE / RETURNING 且回到 leash 内），要么仍在界内。
    matches!(creep_state.phase, LeashPhase::Idle | LeashPhase::Returning) && inside_leash
        || inside_leash
}

#[test]
fn reverse_validation_infinite_chase_formal_passes_the_assertion() {
    let frozen = defs();
    let content = frozen.content_hash();
    let cfg = w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), frozen.clone());
    sim.submit_command(spawn(HOME_X + 10 * SCALE, HOME_Y, 100, 100))
        .unwrap();
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    // 把野怪放到 leash 之外（经真实快照恢复注入）。
    let mut snapshot = sim.capture_snapshot();
    for record in &mut snapshot.positions {
        if record.entity == creep_ids()[0].value() {
            record.x = HOME_X + 3 * SCALE;
        }
    }
    if let Some(state) = snapshot.creep_camps.creeps.get_mut(&creep_ids()[0]) {
        state.phase = LeashPhase::Engaged;
        state.target = Some(EntityId::new(1).unwrap());
    }
    let mut sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    for _ in 0..24 {
        sim.step();
    }
    assert!(
        leash_is_bounded(&sim, creep_ids()[0]),
        "reason=formal_leash_bounded"
    );
}

#[test]
fn reverse_validation_infinite_chase_degenerate_fails_the_same_assertion() {
    // 退化实现：冻结定义把 leash 半径拉到极大（等价于删掉边界判据）。
    // 经同一 `with_creep_camps` / `create_with_definitions` 生产路径注入。
    let frozen = no_leash_defs();
    let content = frozen.content_hash();
    let cfg = w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), frozen.clone());
    sim.submit_command(spawn(HOME_X + 10 * SCALE, HOME_Y, 100, 100))
        .unwrap();
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    for record in &mut snapshot.positions {
        if record.entity == creep_ids()[0].value() {
            record.x = HOME_X + 3 * SCALE;
        }
    }
    if let Some(state) = snapshot.creep_camps.creeps.get_mut(&creep_ids()[0]) {
        state.phase = LeashPhase::Engaged;
        state.target = Some(EntityId::new(1).unwrap());
    }
    let mut sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    for _ in 0..24 {
        sim.step();
    }
    let state = sim.creep_camps();
    assert_eq!(
        state.creeps[&creep_ids()[0]].phase,
        LeashPhase::Engaged,
        "reason=degenerate_stays_engaged_forever"
    );
    assert!(
        !leash_is_bounded(&sim, creep_ids()[0]),
        "reason=degenerate_infinite_chase_must_fail_same_assertion"
    );
    // 正式实现仍通过同一断言（防元断言恒真）。
    let formal = defs();
    let formal_content = formal.content_hash();
    let cfg = w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: formal_content,
    };
    let mut formal_sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), formal);
    formal_sim
        .submit_command(spawn(HOME_X + 10 * SCALE, HOME_Y, 100, 100))
        .unwrap();
    formal_sim
        .submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    formal_sim.step();
    assert!(
        leash_is_bounded(&formal_sim, creep_ids()[0]),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化：定义层冻结常量段漏接 compute_defs_hash（tasks 2.2）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_missing_defs_hash_formal_passes_the_assertion() {
    let base = defs();
    let mut changed_constants = constants();
    changed_constants.acquisition_range_raw += 1;
    let variant = FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(changed_constants, scripts(), tables(), categories());
    assert_ne!(
        base.content_hash(),
        variant.content_hash(),
        "reason=formal_constants_in_defs_hash"
    );
    // 每层（脚本 / set 概率 / 类别 / EffectDef）都进 defs_hash。
    let mut changed_scripts = scripts();
    changed_scripts.get_mut(&CAMP_SCRIPT).unwrap().xp += 1;
    let b = FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(constants(), changed_scripts, tables(), categories());
    assert_ne!(
        base.content_hash(),
        b.content_hash(),
        "reason=script_in_defs_hash"
    );
}

#[test]
fn reverse_validation_missing_defs_hash_degenerate_fails_the_same_assertion() {
    // 退化实现：冻结常量段漏接 compute_defs_hash → 改常量后 content_hash 不变。
    // 用「只编四张基础定义表」的退化哈希模拟该缺陷（D4 / tasks 2.2）。
    let degenerate_defs_hash = |frozen: &FrozenDefinitions| {
        let mut hasher = StateHasher::new();
        hasher.write_u64(frozen.units().len() as u64);
        for (&id, unit) in frozen.units() {
            hasher.write_u32(id);
            hasher.write_i64(unit.hp);
            hasher.write_i64(unit.mana);
        }
        hasher.finish()
    };
    let base = defs();
    let mut constants = constants();
    constants.acquisition_range_raw += 1;
    let variant = FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(constants, scripts(), tables(), categories());
    let degenerate_detects = degenerate_defs_hash(&base) != degenerate_defs_hash(&variant);
    assert!(
        !degenerate_detects,
        "reason=degenerate_blind_to_creep_constants"
    );
    assert!(
        base.content_hash() != variant.content_hash() && !degenerate_detects,
        "reason=degenerate_missing_defs_hash_must_fail_same_assertion"
    );
    assert!(
        base.content_hash() != variant.content_hash(),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化：Q7 把野怪挂玩家 owner 下 spawn（R8.1 / R1.4）
// ---------------------------------------------------------------------------

/// 正式不变量：野怪 owner 为中立常量且中立类别登记为敌对。
fn creeps_are_neutral(snapshot: &Snapshot) -> bool {
    snapshot.creep_camps.creeps.keys().all(|id| {
        snapshot
            .positions
            .iter()
            .find(|record| record.entity == id.value())
            .is_some_and(|record| record.owner == NEUTRAL_OWNER)
            && snapshot.creep_camps.neutral.get(id) == Some(&NeutralCategory::CreepHostile)
    }) && !snapshot.creep_camps.neutral.is_empty()
}

#[test]
fn reverse_validation_neutral_owner_formal_passes_the_assertion() {
    let (sim, _) = seeded_match();
    assert!(
        creeps_are_neutral(&sim.capture_snapshot()),
        "reason=formal_neutral"
    );
}

#[test]
fn reverse_validation_neutral_owner_degenerate_fails_the_same_assertion() {
    let (sim, content) = seeded_match();
    // 退化实现：野怪挂玩家 owner 下 spawn（现状资源点做法），
    // 玩家即可用 `Move` 命令控制野怪（R8.1 / R1.4 缺陷态）。
    let mut broken = sim.capture_snapshot();
    for record in &mut broken.positions {
        if broken
            .creep_camps
            .creeps
            .contains_key(&EntityId::new(record.entity).unwrap())
        {
            record.owner = PLAYER;
        }
    }
    broken.creep_camps.neutral.clear();
    let mut broken_sim =
        RtsMatch::restore_snapshot(broken, StaticWorld::default(), content).unwrap();
    assert!(
        !creeps_are_neutral(&broken_sim.capture_snapshot()),
        "reason=degenerate_player_owner_must_fail"
    );
    // 真实生产路径：退化态下玩家 `Move` 真的改变野怪位置。
    let creep = creep_ids()[0];
    let before = broken_sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep.value())
        .unwrap()
        .x;
    broken_sim
        .submit_command(w3_core::Command::new(
            Frame::new(1),
            PLAYER,
            1,
            w3_core::CommandKind::Move {
                entity: creep,
                x: SimFixed::from_int(7).unwrap(),
                y: SimFixed::from_int(7).unwrap(),
                group: 0,
                append: false,
            },
        ))
        .unwrap();
    broken_sim.step();
    broken_sim.step();
    let after = broken_sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep.value())
        .unwrap()
        .x;
    assert_ne!(
        after, before,
        "reason=degenerate_creep_is_player_controllable"
    );
    // 正式实现仍通过（防元断言恒真）。
    let (formal, _) = seeded_match();
    assert!(
        creeps_are_neutral(&formal.capture_snapshot()),
        "reason=formal_still_passes"
    );
}
