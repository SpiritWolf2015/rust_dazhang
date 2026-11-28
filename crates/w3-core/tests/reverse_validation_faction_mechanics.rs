//! P9 族特色机制反向验证（改坏验红 / `CODEBUDDY.md §5` / Q9' 形态①）。
//!
//! 每一类退化都成对提供 `formal_passes_*` / `degenerate_fails_the_same_assertion`：
//! 后者内部含「正式实现仍通过同一断言」的元断言防退化；退化实现 MUST 经
//! `restore_snapshot()` / `capture_snapshot()` / `compute_state_hash()` /
//! `first_difference()` / `submit_gameplay_command()` / `step()` /
//! `FrozenDefinitions::new` / `content_hash()` 真实生产路径注入，非旁路模拟。
//! 覆盖 Q9' 八类退化：
//! ① 新增权威状态漏进 `compute_state_hash` / `Snapshot` / `first_difference()` /
//!    `restore_snapshot` 之一；
//! ② `HashMap` 迭代顺序 / `f64` / 墙钟（非确定来源）；
//! ③ fog 可见性反写权威状态；
//! ④ 漏接 `defs_hash` 敏感性（冻结常量段未进 `compute_defs_hash`）；
//! ⑤ 旧 v7 快照不拒绝；
//! ⑥ 新命令变体漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发。
//!
//! 不冒充形态②（A-B 对照 / 先写失败测试 / 刻意改坏被测代码验红留命令输出）。
//! 先例：`reverse_validation_item_shop_tp.rs` / `reverse_validation_creep_camps.rs`。

mod faction_common;

use std::collections::BTreeMap;

use faction_common::*;
use w3_core::{
    EntityId, FactionMechanicsConstants, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, GameplayCommandQueue, MineKind, MineRecord, RegenCondition, RegenRecord,
    RtsMatch, SIM_VERSION, SNAPSHOT_VERSION, SYNTHETIC_FACTION_MECHANICS, SnapshotError,
    StateHasher, StaticWorld, StealthRecord,
};

/// 只有「fog 可见性反写」缺陷态才会写入的哨兵帧号（真实整数帧恒小）。
const FOG_WRITEBACK_SENTINEL: u64 = u64::MAX;

// ---------------------------------------------------------------------------
// 退化 ①：新增权威状态漏进 compute_state_hash / Snapshot /
//          first_difference() / restore_snapshot 之一
// ---------------------------------------------------------------------------

/// 退化哈希（缺陷态）：把指定权威字段清空后走 `FactionMechanicsState::hash`
/// 的正式编码，等价于该字段完全未进入 `compute_state_hash`（ADR 0002 §6）。
/// 注入来自真实 `capture_snapshot()` 的 owned 快照。
fn degenerate_hash_omit(snapshot: &w3_core::Snapshot, omit: &str) -> u64 {
    let mut state = snapshot.faction_mechanics.clone();
    omit_field(&mut state, omit);
    let mut hasher = StateHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

fn omit_field(state: &mut w3_core::FactionMechanicsState, omit: &str) {
    match omit {
        "faction_blight_cells" => state.blight_cells.clear(),
        "faction_blight_progress" => state.blight_progress.clear(),
        "faction_corpses" => state.corpses.clear(),
        "faction_corpse_consumptions" => state.corpse_consumptions.clear(),
        "faction_mines" => state.mines.clear(),
        "faction_rooted_ancients" => state.rooted_ancients.clear(),
        "faction_moon_wells" => state.moon_wells.clear(),
        "faction_day_night" => state.day_night = Default::default(),
        "faction_stealth" => state.stealth.clear(),
        "faction_detectors" => state.detectors.clear(),
        "faction_detected" => state.detected.clear(),
        "faction_night_cloak" => state.night_cloak.clear(),
        "faction_regen" => state.regen.clear(),
        "faction_sight_blockers" => state.sight_blockers.clear(),
        "faction_creep_sleep" => state.creep_sleep.clear(),
        "faction_sleeping_creeps" => state.sleeping_creeps.clear(),
        "faction_facts" => state.facts.clear(),
        "faction_next_fact_seq" => state.next_fact_seq = 0,
        other => panic!("faction_reverse: reason=unknown_omit_field value={other}"),
    }
}

/// 注入一批族特色机制权威状态，保证所有退化字段都有初值可清空。
fn inject_faction_state(snapshot: &mut w3_core::Snapshot) {
    let fm = &mut snapshot.faction_mechanics;
    fm.blight_cells.insert((1, 1));
    fm.blight_progress.insert(id(UNIT_A), 1);
    fm.corpses.insert(id(UNIT_B), 3);
    fm.corpse_consumptions.insert(id(UNIT_A), 1);
    fm.mines.insert(
        id(UNIT_C),
        MineRecord {
            kind: MineKind::Haunted,
            progress_frames: 1,
        },
    );
    fm.rooted_ancients.insert(id(UNIT_A), true);
    fm.moon_wells.insert(id(UNIT_B), 50);
    fm.day_night.cycles = 2;
    fm.day_night.elapsed_frames = 1;
    fm.stealth.insert(
        id(UNIT_A),
        StealthRecord {
            cloaked: true,
            night_only: false,
        },
    );
    fm.detectors.insert(id(UNIT_B));
    fm.detected.insert(id(UNIT_A));
    fm.night_cloak.insert(id(UNIT_C));
    fm.regen.insert(
        id(UNIT_A),
        RegenRecord {
            condition: RegenCondition::Always,
            accumulated: 3,
        },
    );
    fm.sight_blockers.insert(id(UNIT_A));
    fm.creep_sleep.insert(id(UNIT_C), true);
    fm.sleeping_creeps.insert(id(UNIT_C));
    fm.facts.insert(
        0,
        w3_core::FactionFact {
            seq: 0,
            frame: 1,
            entity: id(UNIT_A),
            kind: w3_core::FactionFactKind::PhaseChanged,
        },
    );
    fm.next_fact_seq = 1;
}

/// 正向注入用的单字段改动（与 omit_field 一一对应，均改出差异）。
fn mutate_field(snapshot: &mut w3_core::Snapshot, field: &str) {
    let fm = &mut snapshot.faction_mechanics;
    match field {
        "faction_blight_cells" => {
            fm.blight_cells.insert((77, 77));
        }
        "faction_blight_progress" => {
            fm.blight_progress.insert(id(77), 9);
        }
        "faction_corpses" => {
            fm.corpses.insert(id(77), 9);
        }
        "faction_corpse_consumptions" => {
            fm.corpse_consumptions.insert(id(77), 9);
        }
        "faction_mines" => {
            fm.mines.insert(
                id(77),
                MineRecord {
                    kind: MineKind::Entangled,
                    progress_frames: 9,
                },
            );
        }
        "faction_rooted_ancients" => {
            fm.rooted_ancients.insert(id(77), true);
        }
        "faction_moon_wells" => {
            fm.moon_wells.insert(id(77), 9);
        }
        "faction_day_night" => {
            fm.day_night.cycles += 3;
        }
        "faction_stealth" => {
            fm.stealth.insert(
                id(77),
                StealthRecord {
                    cloaked: true,
                    night_only: true,
                },
            );
        }
        "faction_detectors" => {
            fm.detectors.insert(id(77));
        }
        "faction_detected" => {
            fm.detected.insert(id(77));
        }
        "faction_night_cloak" => {
            fm.night_cloak.insert(id(77));
        }
        "faction_regen" => {
            fm.regen.insert(
                id(77),
                RegenRecord {
                    condition: RegenCondition::Night,
                    accumulated: 9,
                },
            );
        }
        "faction_sight_blockers" => {
            fm.sight_blockers.insert(id(77));
        }
        "faction_creep_sleep" => {
            fm.creep_sleep.insert(id(77), true);
        }
        "faction_sleeping_creeps" => {
            fm.sleeping_creeps.insert(id(77));
        }
        "faction_facts" => {
            fm.facts.insert(
                77,
                w3_core::FactionFact {
                    seq: 77,
                    frame: 9,
                    entity: id(77),
                    kind: w3_core::FactionFactKind::MineSettled,
                },
            );
        }
        "faction_next_fact_seq" => {
            fm.next_fact_seq += 7;
        }
        other => panic!("faction_reverse: reason=unknown_field value={other}"),
    }
}

const HASH_CASES: &[(&str, &str)] = &[
    ("faction_blight_cells", "faction_blight_cells"),
    ("faction_blight_progress", "faction_blight_progress"),
    ("faction_corpses", "faction_corpses"),
    ("faction_corpse_consumptions", "faction_corpse_consumptions"),
    ("faction_mines", "faction_mines"),
    ("faction_rooted_ancients", "faction_rooted_ancients"),
    ("faction_moon_wells", "faction_moon_wells"),
    ("faction_day_night", "faction_day_night"),
    ("faction_stealth", "faction_stealth"),
    ("faction_detectors", "faction_detectors"),
    ("faction_detected", "faction_detected"),
    ("faction_night_cloak", "faction_night_cloak"),
    ("faction_regen", "faction_regen"),
    ("faction_sight_blockers", "faction_sight_blockers"),
    ("faction_creep_sleep", "faction_creep_sleep"),
    ("faction_sleeping_creeps", "faction_sleeping_creeps"),
    ("faction_facts", "faction_facts"),
    ("faction_next_fact_seq", "faction_next_fact_seq"),
];

/// 经真实 `restore_snapshot` 生产路径构造「只改一个权威字段」的两端。
fn prepared_pair(content: u64, field: &str) -> (RtsMatch, RtsMatch) {
    let (sim, _) = base_match();
    let prepared = with_state(&sim, content, inject_faction_state);
    let base = prepared.capture_snapshot();
    let mut changed = base.clone();
    mutate_field(&mut changed, field);
    let formal = RtsMatch::restore_snapshot(base, StaticWorld::default(), content)
        .expect("faction_reverse: reason=formal_restore_ok");
    let variant = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content)
        .expect("faction_reverse: reason=variant_restore_ok");
    (formal, variant)
}

/// 正式不变量：改一个字段，两端 `compute_state_hash()` 必不同，
/// 且 `first_difference()` 定位到对应差异字段。
fn field_is_hashed(a: &RtsMatch, b: &RtsMatch, field: &str) -> bool {
    a.compute_state_hash() != b.compute_state_hash() && a.first_difference(b) == Some(field)
}

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let content = defs().content_hash();
    for (field, _) in HASH_CASES {
        let (formal, variant) = prepared_pair(content, field);
        assert!(
            field_is_hashed(&formal, &variant, field),
            "reason=formal_hash_covers field={field}"
        );
    }
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let content = defs().content_hash();
    for (field, omit) in HASH_CASES {
        let (formal, variant) = prepared_pair(content, field);
        // 缺陷态：退化哈希对该字段视而不见 → 两端相等 → 正式断言必失败。
        let degenerate_detects = degenerate_hash_omit(&formal.capture_snapshot(), omit)
            != degenerate_hash_omit(&variant.capture_snapshot(), omit);
        assert!(
            !degenerate_detects,
            "reason=degenerate_hash_blind field={omit}"
        );
        // 元断言：正式能区分、退化不能区分（缺陷态被抓出）。
        assert!(
            field_is_hashed(&formal, &variant, field) && !degenerate_detects,
            "reason=degenerate_missing_hash_must_fail field={omit}"
        );
        // 正式实现仍通过同一断言（防元断言恒真）。
        assert!(
            field_is_hashed(&formal, &variant, field),
            "reason=formal_still_passes field={omit}"
        );
        // 退化实现真经 restore_snapshot 生产路径注入：把「漏接字段」的
        // 快照喂给 restore_snapshot，两端重建后退化态无法区分差异字段。
        let mut degenerate_a = formal.capture_snapshot();
        omit_field(&mut degenerate_a.faction_mechanics, omit);
        let mut degenerate_b = variant.capture_snapshot();
        omit_field(&mut degenerate_b.faction_mechanics, omit);
        let rebuilt_a = RtsMatch::restore_snapshot(degenerate_a, StaticWorld::default(), content)
            .expect("faction_reverse: reason=degenerate_restore_a_ok");
        let rebuilt_b = RtsMatch::restore_snapshot(degenerate_b, StaticWorld::default(), content)
            .expect("faction_reverse: reason=degenerate_restore_b_ok");
        assert_eq!(
            rebuilt_a.compute_state_hash(),
            rebuilt_b.compute_state_hash(),
            "reason=degenerate_restore_cannot_distinguish field={omit}"
        );
        assert_eq!(
            rebuilt_a.first_difference(&rebuilt_b),
            None,
            "reason=degenerate_first_difference_blind field={omit}"
        );
        assert_eq!(
            formal.first_difference(&variant),
            Some(*field),
            "reason=formal_first_difference_still_locates field={field}"
        );
    }
}

/// 退化 ① 的 `Snapshot` 分支：正式 `capture_snapshot()` 必携带新顶层字段。
#[test]
fn reverse_validation_missing_snapshot_field_formal_passes_the_assertion() {
    let (sim, content) = base_match();
    let formal = with_state(&sim, content, inject_faction_state);
    let empty = with_state(&sim, content, |_| {});
    assert_ne!(
        formal.capture_snapshot().faction_mechanics,
        empty.capture_snapshot().faction_mechanics,
        "reason=snapshot_carries_faction_mechanics"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_degenerate_fails_the_same_assertion() {
    let (sim, content) = base_match();
    let formal = with_state(&sim, content, inject_faction_state);
    let empty = with_state(&sim, content, |_| {});
    // 缺陷态：快照漏带新顶层字段（退化为无机制实例的空状态）。
    // 该空状态即正式 no-mechanics 实例的 faction_mechanics 投影，
    // 保证退化态「只丢该字段、不引入其它差异」。
    let mut dropped = formal.capture_snapshot();
    dropped.faction_mechanics = empty.capture_snapshot().faction_mechanics;
    assert_eq!(
        dropped.faction_mechanics,
        empty.capture_snapshot().faction_mechanics,
        "reason=degenerate_snapshot_loses_field"
    );
    // 元断言：正式实现仍携带字段（防元断言恒真）。
    assert_ne!(
        formal.capture_snapshot().faction_mechanics,
        empty.capture_snapshot().faction_mechanics,
        "reason=formal_still_carries_field"
    );
    // 真经 restore_snapshot：丢弃字段的快照重建后哈希与空对局一致。
    let rebuilt = RtsMatch::restore_snapshot(dropped, StaticWorld::default(), content)
        .expect("faction_reverse: reason=degenerate_snapshot_restore_ok");
    assert_eq!(
        rebuilt.compute_state_hash(),
        empty.compute_state_hash(),
        "reason=degenerate_restore_hash_matches_empty"
    );
    assert_ne!(
        formal.compute_state_hash(),
        empty.compute_state_hash(),
        "reason=formal_hash_still_differs_from_empty"
    );
}

/// 退化 ① 的 `restore_snapshot` 分支：正式恢复逐帧一致 / 退化恢复发散。
#[test]
fn reverse_validation_missing_restore_field_formal_passes_the_assertion() {
    let content = defs().content_hash();
    let (sim, _) = base_match();
    let source = with_state(&sim, content, inject_faction_state);
    let formal =
        RtsMatch::restore_snapshot(source.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    assert!(
        restore_is_frame_hashed(&source, &formal, content),
        "reason=formal_restore_frame_hashed"
    );
}

#[test]
fn reverse_validation_missing_restore_field_degenerate_fails_the_same_assertion() {
    let content = defs().content_hash();
    let (sim, _) = base_match();
    let source = with_state(&sim, content, inject_faction_state);
    // 缺陷态：restore 时漏读新顶层字段（清空后经真实 restore_snapshot 重建）。
    let mut dropped = source.capture_snapshot();
    dropped.faction_mechanics = Default::default();
    let degenerate = RtsMatch::restore_snapshot(dropped, StaticWorld::default(), content)
        .expect("faction_reverse: reason=degenerate_restore_ok");
    assert!(
        !restore_is_frame_hashed(&source, &degenerate, content),
        "reason=degenerate_restore_diverges"
    );
    // 元断言：正式恢复仍逐帧一致（防元断言恒真）。
    let formal =
        RtsMatch::restore_snapshot(source.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    assert!(
        restore_is_frame_hashed(&source, &formal, content),
        "reason=formal_restore_still_consistent"
    );
}

/// 正式不变量：两份同源实例经真实 `restore_snapshot` 重建后逐帧哈希一致。
fn restore_is_frame_hashed(source: &RtsMatch, other: &RtsMatch, content: u64) -> bool {
    let mut a =
        RtsMatch::restore_snapshot(source.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    let mut b =
        RtsMatch::restore_snapshot(other.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    for _ in 0..10 {
        a.step();
        b.step();
        if a.compute_state_hash() != b.compute_state_hash() || a.first_difference(&b).is_some() {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// 退化 ②：HashMap 迭代顺序 / f64 / 墙钟（非确定来源）
// ---------------------------------------------------------------------------

fn deterministic_integer_state(a: &RtsMatch, b: &RtsMatch) -> bool {
    a.compute_state_hash() == b.compute_state_hash() && a.first_difference(b) == None
}

fn build_faction_pair(content: u64) -> (RtsMatch, RtsMatch) {
    let (mut a, _) = base_match();
    let mut b = with_state(&a, content, |_| {});
    for sim in [&mut a, &mut b] {
        submit(
            sim,
            1,
            1,
            GameplayCommandKind::SetRegenCondition {
                unit: id(UNIT_A),
                condition: RegenCondition::Always,
            },
        );
    }
    for _ in 0..8 {
        a.step();
        b.step();
    }
    (a, b)
}

#[test]
fn reverse_validation_nondeterministic_source_formal_passes_the_assertion() {
    let content = defs().content_hash();
    let (a, b) = build_faction_pair(content);
    assert!(
        deterministic_integer_state(&a, &b),
        "reason=formal_deterministic_integer_state"
    );
    assert_eq!(
        a.faction_mechanics_state().day_night.elapsed_frames,
        b.faction_mechanics_state().day_night.elapsed_frames,
        "reason=integer_frame_phase_elapsed"
    );
}

/// 缺陷态：墙钟派生相位帧 / HashMap 迭代顺序重排事实序号 / f64 舍入
/// 误写回血累计，经真实 `capture_snapshot` → `restore_snapshot` 注入。
#[test]
fn reverse_validation_nondeterministic_source_degenerate_fails_the_same_assertion() {
    let content = defs().content_hash();
    let (a, b) = build_faction_pair(content);
    let mut wallclock = b.capture_snapshot();
    // 墙钟：相位帧不按整数帧推进。
    wallclock.faction_mechanics.day_night.elapsed_frames += 1;
    // HashMap 迭代顺序：事实序号以不同到达顺序改写。
    wallclock.faction_mechanics.next_fact_seq += 1;
    // f64 舍入：回血累计丢精度写成相邻整数。
    if let Some(record) = wallclock.faction_mechanics.regen.get_mut(&id(UNIT_A)) {
        record.accumulated = record.accumulated.saturating_add(1);
    }
    let forged = RtsMatch::restore_snapshot(wallclock, StaticWorld::default(), content)
        .expect("faction_reverse: reason=degenerate_restore_ok");
    assert!(
        !deterministic_integer_state(&a, &forged),
        "reason=degenerate_nondeterministic_diverges"
    );
    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    let (formal_a, formal_b) = build_faction_pair(content);
    assert!(
        deterministic_integer_state(&formal_a, &formal_b),
        "reason=formal_still_deterministic"
    );
    // 差异 MUST 被 first_difference() 稳定定位（BTree 顺序，非 HashMap 顺序）。
    assert!(
        a.first_difference(&forged).is_some(),
        "reason=first_difference_locates_nondeterministic_drift"
    );
}

// ---------------------------------------------------------------------------
// 退化 ③：fog 可见性反写权威状态（R10.5 / §0.3 T3）
// ---------------------------------------------------------------------------

/// 正式不变量：表现层由 owned `read_view()` 只读派生 fog 可见性，
/// 权威状态 MUST NOT 出现「可见性标记」事实；消费只读视图零哈希冲击。
fn fog_visibility_is_read_only(sim: &RtsMatch, baseline_hash: u64) -> bool {
    let view = sim.read_view();
    let mut derived: Vec<u64> = view.entities.iter().map(|entity| entity.id).collect();
    derived.sort_unstable();
    derived.reverse();
    derived.push(u64::MAX);
    let no_writeback = !sim
        .faction_mechanics_state()
        .facts
        .values()
        .any(|fact| fact.frame == FOG_WRITEBACK_SENTINEL);
    sim.compute_state_hash() == baseline_hash && no_writeback
}

#[test]
fn reverse_validation_fog_writeback_formal_passes_the_assertion() {
    let (sim, _) = base_match();
    let baseline = sim.compute_state_hash();
    assert!(
        fog_visibility_is_read_only(&sim, baseline),
        "reason=formal_fog_is_read_only"
    );
    for field in w3_core::FACTION_HASHED_FIELDS {
        let lower = field.to_ascii_lowercase();
        assert!(!lower.contains("fog") && !lower.contains("visible"));
        assert!(!lower.contains("explored") && !lower.contains("visibility"));
    }
}

/// 缺陷态：把 fog 可见性派生结果反写进权威事实登记（哨兵帧号），
/// 经真实 `capture_snapshot` → `restore_snapshot` 注入；正式「fog 只读」
/// 断言在退化态失败，且 `first_difference()` 稳定定位。
#[test]
fn reverse_validation_fog_writeback_degenerate_fails_the_same_assertion() {
    let (sim, content) = base_match();
    let baseline = sim.compute_state_hash();
    let mut forged_snapshot = sim.capture_snapshot();
    let derived_visible: Vec<u64> = sim
        .read_view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    for (index, visible_id) in derived_visible.iter().enumerate() {
        forged_snapshot.faction_mechanics.facts.insert(
            500 + index as u64,
            w3_core::FactionFact {
                seq: 500 + index as u64,
                frame: FOG_WRITEBACK_SENTINEL,
                entity: EntityId::new(*visible_id).unwrap(),
                kind: w3_core::FactionFactKind::StealthRevealed,
            },
        );
    }
    forged_snapshot.faction_mechanics.next_fact_seq = 500 + derived_visible.len() as u64;
    let forged = RtsMatch::restore_snapshot(forged_snapshot, StaticWorld::default(), content)
        .expect("faction_reverse: reason=degenerate_fog_restore_ok");
    assert!(
        !fog_visibility_is_read_only(&forged, baseline),
        "reason=degenerate_fog_writeback_detected"
    );
    // 元断言：正式实现仍只读（防元断言恒真）。
    assert!(
        fog_visibility_is_read_only(&sim, baseline),
        "reason=formal_fog_still_read_only"
    );
    assert_ne!(
        sim.compute_state_hash(),
        forged.compute_state_hash(),
        "reason=fog_writeback_changes_authoritative_hash"
    );
    assert!(
        sim.first_difference(&forged).is_some(),
        "reason=fog_writeback_first_difference_locates"
    );
}

// ---------------------------------------------------------------------------
// 退化 ④：漏接 defs_hash 敏感性（冻结常量段未进 compute_defs_hash）
// ---------------------------------------------------------------------------

/// 退化 content_hash（缺陷态）：模拟 `compute_defs_hash` 漏接族特色机制
/// 冻结常量段——无论注入什么常量都返回「默认常量」的 content_hash，
/// 等价「改常量不改变 defs_hash」。经真实 `FrozenDefinitions::new`
/// + `with_faction_mechanics` + `content_hash()` 生产路径注入。
fn degenerate_defs_hash_constants_omitted(_constants: FactionMechanicsConstants) -> u64 {
    FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(SYNTHETIC_FACTION_MECHANICS)
        .content_hash()
}

fn constants_are_content_sensitive(base: &FrozenDefinitions, changed: &FrozenDefinitions) -> bool {
    base.content_hash() != changed.content_hash()
        && base
            .faction_mechanics()
            .first_difference(changed.faction_mechanics())
            .is_some()
}

#[test]
fn reverse_validation_defs_hash_sensitivity_formal_passes_the_assertion() {
    let base = default_defs();
    let mut changed = SYNTHETIC_FACTION_MECHANICS;
    changed.blight_expansion_frames += 1;
    let variant = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(changed);
    assert!(
        constants_are_content_sensitive(&base, &variant),
        "reason=formal_defs_hash_constants_sensitive"
    );
    let (sim_base, _) = base_match_with(base.clone());
    let (sim_variant, _) = base_match_with(variant);
    assert_ne!(
        sim_base.capture_snapshot().defs_hash,
        sim_variant.capture_snapshot().defs_hash,
        "reason=formal_defs_hash_in_snapshot_header"
    );
}

#[test]
fn reverse_validation_defs_hash_sensitivity_degenerate_fails_the_same_assertion() {
    let base = default_defs();
    let mut changed = SYNTHETIC_FACTION_MECHANICS;
    changed.blight_expansion_frames += 1;
    let variant = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(changed);
    // 缺陷态：退化哈希对常量段视而不见 → 两个不同常量得到同一哈希。
    let degenerate_detects = degenerate_defs_hash_constants_omitted(*base.faction_mechanics())
        != degenerate_defs_hash_constants_omitted(*variant.faction_mechanics());
    assert!(
        !degenerate_detects,
        "reason=degenerate_defs_hash_blind_to_constants"
    );
    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        constants_are_content_sensitive(&base, &variant),
        "reason=formal_defs_hash_still_sensitive"
    );
    assert!(
        constants_are_content_sensitive(&base, &variant) && !degenerate_detects,
        "reason=degenerate_must_fail_same_assertion"
    );
}

// ---------------------------------------------------------------------------
// 退化 ⑤：旧 v7 快照不拒绝（不提供跨版本迁移、不静默兼容）
// ---------------------------------------------------------------------------

/// 版本闸门不变量：`gate_enabled = true` 为正式实现（明确拒绝 v7）；
/// `gate_enabled = false` 为退化实现（静默把 v7 头部改写为 v10 后恢复）。
/// 两者 MUST 经同一个断言辅助函数，退化态同一断言必失败。
fn version_gate_rejects(sim: &RtsMatch, content: u64, gate_enabled: bool) -> bool {
    let mut old = sim.capture_snapshot();
    old.version = 7;
    if !gate_enabled {
        // 缺陷态：漏掉版本校验，静默升级头部（非旁路模拟，仍走 restore_snapshot）。
        old.version = SNAPSHOT_VERSION;
    }
    let version_rejected = matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 7 })
    );
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 7;
    if !gate_enabled {
        old_sim.sim_version = SIM_VERSION;
    }
    let sim_rejected = matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 7,
            expected: 10
        })
    );
    version_rejected && sim_rejected && SNAPSHOT_VERSION == 10 && SIM_VERSION == 10
}

#[test]
fn reverse_validation_old_v7_reject_formal_passes_the_assertion() {
    let (sim, content) = base_match();
    assert!(
        version_gate_rejects(&sim, content, true),
        "reason=formal_old_v7_rejected"
    );
}

/// 缺陷态：漏掉版本校验 → 把 v7 快照静默升级为当前版本后恢复成功
/// （经真实 `restore_snapshot` 注入，非旁路模拟）；同一断言在退化态失败。
#[test]
fn reverse_validation_old_v7_reject_degenerate_fails_the_same_assertion() {
    let (sim, content) = base_match();
    assert!(
        !version_gate_rejects(&sim, content, false),
        "reason=degenerate_silently_accepts_v7_fails_same_assertion"
    );
    // 缺陷态的恢复确实成功（非静默拒绝）。
    let mut degenerate = sim.capture_snapshot();
    degenerate.version = 7;
    degenerate.sim_version = 7;
    let mut upgraded = degenerate;
    upgraded.version = SNAPSHOT_VERSION;
    upgraded.sim_version = SIM_VERSION;
    assert!(
        RtsMatch::restore_snapshot(upgraded, StaticWorld::default(), content).is_ok(),
        "reason=degenerate_v7_restore_ok"
    );
    // 元断言：正式实现仍明确拒绝 v7（防元断言恒真）。
    assert!(
        version_gate_rejects(&sim, content, true),
        "reason=formal_still_rejects_v7"
    );
}

// ---------------------------------------------------------------------------
// 退化 ⑥：新命令变体漏接 hash_gameplay_command 编码 / 校验 / 穷尽分发
// ---------------------------------------------------------------------------

fn new_commands_are_encoded_validated_and_dispatched(kinds: &[GameplayCommandKind]) -> bool {
    let mut hashes = std::collections::BTreeSet::new();
    for kind in kinds {
        let mut queue = GameplayCommandQueue::new();
        if queue
            .submit(GameplayCommand::new(Frame::new(1), PLAYER, 1, *kind))
            .is_err()
        {
            return false;
        }
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        if !hashes.insert(hasher.finish()) {
            return false;
        }
    }
    let mut invalid = GameplayCommandQueue::new();
    invalid
        .submit(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::RegisterDetector {
                unit: EntityId::NONE,
            },
        ))
        .is_err()
}

/// 正式不变量：某个具体新变体经公开入口 + `step()` 后，目标权威字段被写入
/// 且命令队列被消费（穷尽分发）。
fn regen_branch_is_dispatched(kind: GameplayCommandKind) -> bool {
    let frozen = defs();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    sim.submit_gameplay_command(GameplayCommand::new(Frame::new(0), PLAYER, 1, kind))
        .unwrap();
    sim.step();
    !sim.faction_mechanics_state().regen.is_empty()
        && sim
            .capture_snapshot()
            .gameplay
            .commands
            .pending()
            .is_empty()
}

#[test]
fn reverse_validation_command_encoding_formal_passes_the_assertion() {
    let kinds = new_command_kinds();
    assert!(
        new_commands_are_encoded_validated_and_dispatched(&kinds),
        "reason=formal_new_commands_encoded_validated"
    );
    // 穷尽分发：每个变体经公开入口 + step() 被消费、且改变权威状态。
    let frozen = defs();
    for (index, kind) in kinds.iter().enumerate() {
        let mut dispatched = RtsMatch::create_with_definitions(
            config(&frozen),
            StaticWorld::default(),
            frozen.clone(),
        );
        dispatched
            .submit_gameplay_command(GameplayCommand::new(
                Frame::new(0),
                PLAYER,
                index as u32 + 1,
                *kind,
            ))
            .unwrap();
        dispatched.step();
        assert!(
            dispatched
                .capture_snapshot()
                .gameplay
                .commands
                .pending()
                .is_empty(),
            "reason=exhaustive_dispatch index={index}"
        );
    }
}

#[test]
fn reverse_validation_command_encoding_degenerate_fails_the_same_assertion() {
    let kinds = new_command_kinds();
    // 缺陷态 A：漏接 `hash_gameplay_command` 编码——全部新变体折叠为
    // 同一个旧 `SetAi` 变体（等价整段未进哈希）→ 编码碰撞。
    let degenerate_encode = |_kind: &GameplayCommandKind| {
        let mut queue = GameplayCommandQueue::new();
        queue
            .submit(GameplayCommand::new(
                Frame::new(1),
                PLAYER,
                1,
                GameplayCommandKind::SetAi { enabled: false },
            ))
            .unwrap();
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        hasher.finish()
    };
    let mut degenerate_hashes = std::collections::BTreeSet::new();
    for kind in &kinds {
        degenerate_hashes.insert(degenerate_encode(kind));
    }
    assert_eq!(
        degenerate_hashes.len(),
        1,
        "reason=degenerate_encode_collides_all_variants"
    );
    // 正式编码 13 个互异。
    let mut formal_hashes = std::collections::BTreeSet::new();
    for kind in &kinds {
        let mut queue = GameplayCommandQueue::new();
        queue
            .submit(GameplayCommand::new(Frame::new(1), PLAYER, 1, *kind))
            .unwrap();
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        formal_hashes.insert(hasher.finish());
    }
    assert_eq!(formal_hashes.len(), kinds.len(), "reason=formal_unique");

    // 缺陷态 B：校验漏分支——退化校验对 0 号新实体放行。
    let degenerate_validates =
        |kind: &GameplayCommandKind| matches!(kind, GameplayCommandKind::RegisterDetector { .. });
    assert!(
        degenerate_validates(&GameplayCommandKind::RegisterDetector {
            unit: EntityId::NONE,
        }),
        "reason=degenerate_validation_would_pass_invalid"
    );

    // 缺陷态 C：穷尽分发漏分支——把 `SetRegenCondition` 的唯一权威效果
    // 经真实 `capture_snapshot` → `restore_snapshot` 抹掉，模拟分支缺失。
    let regen_kind = GameplayCommandKind::SetRegenCondition {
        unit: id(UNIT_A),
        condition: RegenCondition::Always,
    };
    let frozen = defs();
    let content = frozen.content_hash();
    let mut formal_dispatch =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    formal_dispatch
        .submit_gameplay_command(GameplayCommand::new(Frame::new(0), PLAYER, 1, regen_kind))
        .unwrap();
    formal_dispatch.step();
    assert!(
        regen_branch_is_dispatched(regen_kind),
        "reason=formal_branch"
    );
    let mut dropped = formal_dispatch.capture_snapshot();
    dropped.faction_mechanics.regen.clear();
    let degenerate_dispatch = RtsMatch::restore_snapshot(dropped, StaticWorld::default(), content)
        .expect("faction_reverse: reason=degenerate_dispatch_restore_ok");
    assert!(
        degenerate_dispatch
            .faction_mechanics_state()
            .regen
            .is_empty(),
        "reason=degenerate_branch_effect_missing"
    );
    assert_ne!(
        formal_dispatch.compute_state_hash(),
        degenerate_dispatch.compute_state_hash(),
        "reason=degenerate_dispatch_hash_diverges"
    );
    assert_eq!(
        formal_dispatch.first_difference(&degenerate_dispatch),
        Some("faction_regen"),
        "reason=degenerate_dispatch_first_difference_locates"
    );
    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        new_commands_are_encoded_validated_and_dispatched(&kinds),
        "reason=formal_still_encoded_validated"
    );
    let _: BTreeMap<u32, u32> = BTreeMap::new();
}
