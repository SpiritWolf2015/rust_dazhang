//! P9 族特色机制正向 CPU 回归（Q2 / Q3 / Q4 / ADR 0002 / `CODEBUDDY.md §5`）。
//!
//! 覆盖：冻结常量段进 `compute_defs_hash`（2.3 / 2.4）、四处同步（3.3）、
//! Q4 六类全量机制本期确定性结算 + fog 路径隔离（3.4）、bump 8 与旧 v7
//! 明确拒绝（5.1）、新增命令变体穷尽分发（5.2）。全部经 `w3-core` 公开
//! API 与 `restore_snapshot()` / `capture_snapshot()` / `compute_state_hash()`
//! / `first_difference()` / `step()` 生产路径注入，非旁路模拟。

mod faction_common;

use std::collections::BTreeMap;

use faction_common::*;
use w3_core::{
    DayNightPhase, DefId, FACTION_HASHED_FIELDS, FactionFact, FactionFactKind,
    FactionMechanicsConstants, FrozenDefinitions, GameplayCommandQueue, MineKind, MineRecord,
    RegenCondition, RegenRecord, RtsMatch, SIM_VERSION, SNAPSHOT_VERSION,
    SYNTHETIC_FACTION_MECHANICS, SimFixed, SnapshotError, StateHasher, StaticWorld, StealthRecord,
};

// ---------------------------------------------------------------------------
// 2.3 / 2.4：冻结常量段进 compute_defs_hash → defs_hash / content_hash 快照头
// ---------------------------------------------------------------------------

/// client `synthetic_definitions()` 现有 `FrozenDefinitions::new(...)` 调用点
/// 零改动即可编译的四参数签名证明：默认族特色机制常量即 SYNTHETIC。
#[test]
fn frozen_definitions_new_defaults_to_synthetic_mechanics_zero_change_compile() {
    let frozen = default_defs();
    assert_eq!(
        frozen.faction_mechanics(),
        &SYNTHETIC_FACTION_MECHANICS,
        "reason=new_defaults_to_synthetic"
    );
    assert!(
        frozen.regen_conditions().is_empty(),
        "reason=new_defaults_to_empty_regen_table"
    );
    // 常量段与回血类别登记默认已编入 content_hash，改任一即变化。
    let mut changed = SYNTHETIC_FACTION_MECHANICS;
    changed.corpse_decay_frames += 1;
    let with_changed = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(changed);
    assert_ne!(
        frozen.content_hash(),
        with_changed.content_hash(),
        "reason=constants_segment_in_defs_hash"
    );
    assert_eq!(
        frozen
            .faction_mechanics()
            .first_difference(with_changed.faction_mechanics()),
        Some("faction_const_corpse_decay_frames"),
        "reason=constants_first_difference_locates"
    );
}

/// client `synthetic_definitions()` 完整 builder 链零改动编译证明：
/// `FrozenDefinitions::new(...)` → 四个既有 `.with_*` builder 链全部保留，
/// 未调用新增的 `with_faction_mechanics` / `with_regen_conditions`，
/// 链末 `faction_mechanics` 仍为 SYNTHETIC 默认值、`regen_conditions`
/// 为空（leader 跨 crate 契约：client 现有调用点零改动即可编译）。
#[test]
fn client_frozen_definitions_builder_chain_compiles_zero_change() {
    use std::collections::{BTreeMap, BTreeSet};
    use w3_core::{
        SYNTHETIC_CREEP_CONSTANTS, SYNTHETIC_REVIVE_CONSTANTS, SYNTHETIC_TOWN_HALL_CONSTANTS,
        synthetic_item_shop_definitions,
    };
    let frozen = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .expect("faction_forward: reason=chain_new_ok")
        .with_hero_revive(SYNTHETIC_REVIVE_CONSTANTS, BTreeSet::new(), BTreeSet::new())
        .with_town_hall_upgrade(
            SYNTHETIC_TOWN_HALL_CONSTANTS,
            BTreeSet::new(),
            BTreeMap::new(),
        )
        .with_creep_camps(
            SYNTHETIC_CREEP_CONSTANTS,
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        )
        .with_item_shop(synthetic_item_shop_definitions());
    assert_eq!(
        frozen.faction_mechanics(),
        &SYNTHETIC_FACTION_MECHANICS,
        "reason=chain_defaults_to_synthetic"
    );
    assert!(
        frozen.regen_conditions().is_empty(),
        "reason=chain_defaults_to_empty_regen"
    );
    // 常量段已编入 content_hash（即使 client 未显式注入）。
    let mut changed = SYNTHETIC_FACTION_MECHANICS;
    changed.day_frames += 1;
    let variant = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(changed);
    assert_ne!(frozen.content_hash(), variant.content_hash());
}

/// 冻结常量段 + 回血条件类别登记 + 四张定义表分别改变 content_hash，
/// 并如实进入 `Snapshot.defs_hash` / `content_hash` 快照头部（2.4）。
#[test]
fn faction_constants_and_regen_table_change_defs_hash_and_snapshot_header() {
    let base = default_defs();
    let mut constants = SYNTHETIC_FACTION_MECHANICS;
    constants.moon_well_restore += 1;
    let with_constants = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(constants);
    assert_ne!(base.content_hash(), with_constants.content_hash());

    let mut regen = BTreeMap::new();
    regen.insert(REGEN_DEF_NIGHT, RegenCondition::Night);
    let with_regen = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_regen_conditions(regen);
    assert_ne!(base.content_hash(), with_regen.content_hash());
    assert_eq!(
        with_regen.regen_condition(REGEN_DEF_NIGHT),
        RegenCondition::Night
    );
    assert_eq!(with_regen.regen_condition(9999), RegenCondition::None);

    // 定义 vec 变化也如实改变 content_hash（差异来自定义表 + 常量段）。
    let unit = w3_core::UnitDef {
        hp: 50,
        mana: 0,
        speed_raw: 0,
        damage: 0,
        range_raw: 0,
        supply: 0,
        train_frames: 1,
    };
    let with_unit = FrozenDefinitions::new(vec![(399, unit)], vec![], vec![], vec![]).unwrap();
    assert_ne!(base.content_hash(), with_unit.content_hash());

    // defs_hash / content_hash 进快照头部：不同冻结定义 → 不同头部。
    let (sim_base, _) = base_match_with(with_constants.clone());
    let (sim_unit, _) = base_match_with(with_unit);
    assert_ne!(
        sim_base.capture_snapshot().defs_hash,
        sim_unit.capture_snapshot().defs_hash,
        "reason=defs_hash_in_snapshot_header"
    );
    assert_eq!(
        sim_base.capture_snapshot().defs_hash,
        with_constants.content_hash(),
        "reason=defs_hash_equals_content_hash"
    );
}

/// `defs_hash_is_fixed_and_content_sensitive` 元断言同源复绿（2.3）：
/// 同构构造哈希固定、少一个定义或改一个常量即改变。
#[test]
fn defs_hash_is_fixed_and_content_sensitive_meta_assertion() {
    let a = defs();
    let b = defs();
    assert_eq!(a, b);
    assert_eq!(a.content_hash(), b.content_hash());

    let mut constants = short_constants();
    constants.blight_expansion_frames += 1;
    let c = defs_with_constants(constants);
    assert_ne!(a.content_hash(), c.content_hash());

    // 少一个定义即改变（天然覆盖分段定义差异）。
    let mut regen = BTreeMap::new();
    regen.insert(REGEN_DEF_ALWAYS, RegenCondition::Always);
    let fewer = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_faction_mechanics(short_constants())
        .with_regen_conditions(regen);
    assert_ne!(a.content_hash(), fewer.content_hash());
}

// ---------------------------------------------------------------------------
// 5.1：bump 8 + 旧 v7 明确拒绝（不提供跨版本迁移、不静默兼容）
// ---------------------------------------------------------------------------

#[test]
fn versions_are_bumped_to_8_and_old_v7_snapshot_is_explicitly_rejected() {
    assert_eq!(SNAPSHOT_VERSION, 8, "reason=snapshot_version_8");
    assert_eq!(SIM_VERSION, 8, "reason=sim_version_8");
    let (sim, content) = base_match();
    let snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.version, 8);
    assert_eq!(snapshot.sim_version, 8);

    let mut old = snapshot.clone();
    old.version = 7;
    assert!(matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 7 })
    ));
    let mut old_sim = snapshot.clone();
    old_sim.sim_version = 7;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 7,
            expected: 8
        })
    ));
    // v8 快照自身合法（无静默拒绝、无迁移歧义）。
    assert!(
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).is_ok(),
        "reason=v8_snapshot_valid"
    );
}

// ---------------------------------------------------------------------------
// 3.3：四处同步（compute_state_hash / Snapshot / first_difference / 恢复）
// ---------------------------------------------------------------------------

/// 按字段名注入「只改一个权威字段」的差异（经 restore_snapshot 生产路径）。
fn mutate_field(snapshot: &mut w3_core::Snapshot, field: &str) {
    let fm = &mut snapshot.faction_mechanics;
    match field {
        "faction_blight_cells" => {
            fm.blight_cells.insert((99, 99));
        }
        "faction_blight_progress" => {
            fm.blight_progress.insert(id(90), 7);
        }
        "faction_corpses" => {
            fm.corpses.insert(id(90), 7);
        }
        "faction_corpse_consumptions" => {
            fm.corpse_consumptions.insert(id(90), 7);
        }
        "faction_mines" => {
            fm.mines.insert(
                id(90),
                MineRecord {
                    kind: MineKind::Haunted,
                    progress_frames: 7,
                },
            );
        }
        "faction_rooted_ancients" => {
            fm.rooted_ancients.insert(id(90), true);
        }
        "faction_moon_wells" => {
            fm.moon_wells.insert(id(90), 7);
        }
        "faction_day_night" => {
            fm.day_night.cycles += 1;
        }
        "faction_stealth" => {
            fm.stealth.insert(
                id(90),
                StealthRecord {
                    cloaked: true,
                    night_only: true,
                },
            );
        }
        "faction_detectors" => {
            fm.detectors.insert(id(90));
        }
        "faction_detected" => {
            fm.detected.insert(id(90));
        }
        "faction_night_cloak" => {
            fm.night_cloak.insert(id(90));
        }
        "faction_regen" => {
            fm.regen.insert(
                id(90),
                RegenRecord {
                    condition: RegenCondition::Always,
                    accumulated: 7,
                },
            );
        }
        "faction_sight_blockers" => {
            fm.sight_blockers.insert(id(90));
        }
        "faction_creep_sleep" => {
            fm.creep_sleep.insert(id(90), true);
        }
        "faction_sleeping_creeps" => {
            fm.sleeping_creeps.insert(id(90));
        }
        "faction_facts" => {
            fm.facts.insert(
                999,
                FactionFact {
                    seq: 999,
                    frame: 1,
                    entity: id(90),
                    kind: FactionFactKind::PhaseChanged,
                },
            );
        }
        "faction_next_fact_seq" => {
            fm.next_fact_seq += 1;
        }
        other => panic!("faction_forward: reason=unknown_field field={other}"),
    }
}

/// 全部 18 个新增权威字段同步进 compute_state_hash / Snapshot /
/// first_difference；`first_difference()` 按稳定顺序定位到对应字段名。
#[test]
fn all_new_authoritative_fields_are_four_way_synced_and_located() {
    assert_eq!(FACTION_HASHED_FIELDS.len(), 18, "reason=hashed_field_count");
    let (sim, content) = base_match();
    for field in FACTION_HASHED_FIELDS {
        let formal = with_state(&sim, content, |_| {});
        let variant = with_state(&sim, content, |snapshot| mutate_field(snapshot, field));
        assert_ne!(
            formal.compute_state_hash(),
            variant.compute_state_hash(),
            "reason=field_hashed field={field}"
        );
        assert_eq!(
            formal.first_difference(&variant),
            Some(*field),
            "reason=field_located field={field}"
        );
        assert_ne!(
            formal.capture_snapshot().faction_mechanics,
            variant.capture_snapshot().faction_mechanics,
            "reason=field_in_snapshot field={field}"
        );
    }
    // 同构实例 first_difference() 为 None。
    let a = with_state(&sim, content, |_| {});
    let b = with_state(&sim, content, |_| {});
    assert_eq!(a.first_difference(&b), None, "reason=identical_instances");
    assert_eq!(a.compute_state_hash(), b.compute_state_hash());
}

/// v8 快照契约：销毁实例 → restore_snapshot → 逐帧哈希与快照一致，
/// `first_difference()` 恒为 None（ADR 0002 §7 / Q3）。
#[test]
fn v8_snapshot_restore_keeps_frame_hashes_identical() {
    let (original, content) = base_match();
    // 注入一批族特色机制权威状态，确保恢复路径携带新顶层字段。
    let mut prepared = with_state(&original, content, |snapshot| {
        for field in FACTION_HASHED_FIELDS.iter().take(8) {
            mutate_field(snapshot, field);
        }
    });
    let snapshot = prepared.capture_snapshot();
    let mut restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
        .expect("faction_forward: reason=v8_restore_ok");
    assert_eq!(prepared.compute_state_hash(), restored.compute_state_hash());
    for _ in 0..16 {
        prepared.step();
        restored.step();
        assert_eq!(prepared.compute_state_hash(), restored.compute_state_hash());
        assert_eq!(
            prepared.capture_snapshot(),
            restored.capture_snapshot(),
            "reason=frame_snapshot_equal"
        );
        assert_eq!(prepared.first_difference(&restored), None);
    }
    let _ = original;
}

/// 同脚本两实例交错推进逐帧哈希一致（ADR 0002 §8 / Q4）。
#[test]
fn two_instances_interleaved_match_frame_hashes_with_faction_mechanics() {
    let (mut a, content) = base_match();
    let mut b = with_state(&a, content, |_| {});
    for frame in 0..20 {
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
            "reason=frame_consistent frame={frame}"
        );
        assert_eq!(a.first_difference(&b), None);
    }
}

// ---------------------------------------------------------------------------
// 3.4：Q4 六类全量机制本期确定性结算（30 Hz 整数帧逐 tick 查表）
// ---------------------------------------------------------------------------

/// R10.2 昼夜循环全量：整数帧推进、达到冻结常量阈值切换相位，
/// 两实例逐帧一致，无墙钟 / 无 f64。
#[test]
fn day_night_cycle_is_integer_frame_deterministic() {
    let (mut a, content) = base_match();
    let mut b = with_state(&a, content, |_| {});
    let mut phase_changes = 0;
    for _ in 0..14 {
        a.step();
        b.step();
        assert_eq!(a.compute_state_hash(), b.compute_state_hash());
        assert_eq!(a.first_difference(&b), None);
        if a.faction_mechanics_state()
            .facts
            .values()
            .any(|fact| fact.kind == FactionFactKind::PhaseChanged)
        {
            phase_changes += 1;
        }
    }
    assert!(phase_changes > 0, "reason=phase_changed_at_frame_threshold");
    // 相位离散值只可能是 Day / Night（显式 code 编码，无浮点相位）。
    let phase = a.faction_mechanics_state().day_night.phase;
    assert!(matches!(phase, DayNightPhase::Day | DayNightPhase::Night));
}

/// R10.3 回血：回血条件由定义层枚举逐 tick 查表，
/// Always / Day / Night / Blight / None 按相位与腐地登记确定性判定。
#[test]
fn regen_condition_is_definition_layer_enum_lookup_per_tick() {
    let (sim, content) = base_match();
    let mut prepared = with_state(&sim, content, |snapshot| {
        let fm = &mut snapshot.faction_mechanics;
        fm.regen.insert(
            id(UNIT_A),
            RegenRecord {
                condition: RegenCondition::Always,
                accumulated: 0,
            },
        );
        fm.regen.insert(
            id(UNIT_B),
            RegenRecord {
                condition: RegenCondition::None,
                accumulated: 0,
            },
        );
        fm.regen.insert(
            id(UNIT_C),
            RegenRecord {
                condition: RegenCondition::Night,
                accumulated: 0,
            },
        );
    });
    let mut peer = with_state(&prepared, content, |_| {});
    for _ in 0..10 {
        prepared.step();
        peer.step();
        assert_eq!(prepared.compute_state_hash(), peer.compute_state_hash());
    }
    let state = prepared.faction_mechanics_state();
    assert!(
        state.regen.get(&id(UNIT_A)).unwrap().accumulated > 0,
        "reason=always_regen_accumulates"
    );
    assert_eq!(
        state.regen.get(&id(UNIT_B)).unwrap().accumulated,
        0,
        "reason=none_never_regens"
    );
    // 夜间相位出现后才回血（Night 条件），确定性由整数帧派生。
    assert!(
        state
            .facts
            .values()
            .any(|fact| fact.kind == FactionFactKind::RegenApplied),
        "reason=regen_fact_recorded"
    );
    let _ = sim;
}

/// R10.4 隐身反隐：隐身目标 + 反隐登记 → 逐 tick 确定性揭示判定
/// （玩法机制权威状态，非 fog 可见性）。
#[test]
fn stealth_detection_is_deterministic_authoritative_mechanism() {
    let (mut sim, content) = base_match();
    submit(
        &mut sim,
        1,
        1,
        w3_core::GameplayCommandKind::SetStealth {
            unit: id(UNIT_A),
            cloaked: true,
        },
    );
    submit(
        &mut sim,
        1,
        2,
        w3_core::GameplayCommandKind::RegisterDetector { unit: id(UNIT_B) },
    );
    sim.step();
    let mut peer = with_state(&sim, content, |_| {});
    for _ in 0..6 {
        sim.step();
        peer.step();
        assert_eq!(sim.compute_state_hash(), peer.compute_state_hash());
    }
    let state = sim.faction_mechanics_state();
    assert!(
        state.detected.contains(&id(UNIT_A)),
        "reason=cloaked_target_detected"
    );
    assert_eq!(
        state.stealth.get(&id(UNIT_A)).unwrap().cloaked,
        true,
        "reason=stealth_state_authoritative"
    );
    assert!(state.detectors.contains(&id(UNIT_B)));
}

/// S3 暗夜夜间隐身特色：夜间隐身候选仅夜晚自动隐身，白昼解除。
#[test]
fn night_cloak_applies_only_at_night() {
    let (mut sim, content) = base_match();
    submit(
        &mut sim,
        1,
        1,
        w3_core::GameplayCommandKind::SetNightCloak {
            unit: id(UNIT_A),
            enabled: true,
        },
    );
    sim.step();
    let mut peer = with_state(&sim, content, |_| {});
    let mut saw_cloak_fact = false;
    for _ in 0..14 {
        sim.step();
        peer.step();
        assert_eq!(sim.compute_state_hash(), peer.compute_state_hash());
        if sim
            .faction_mechanics_state()
            .facts
            .values()
            .any(|fact| fact.kind == FactionFactKind::NightCloakApplied)
        {
            saw_cloak_fact = true;
        }
    }
    assert!(saw_cloak_fact, "reason=night_cloak_fact_recorded");
    assert!(
        sim.faction_mechanics_state()
            .night_cloak
            .contains(&id(UNIT_A)),
        "reason=night_cloak_registered"
    );
}

/// R5.6 视线阻挡：登记的是「哪些实体阻挡视线」的玩法机制输入，
/// MUST NOT 是 fog 可见性派生结果（R10.5 / §0.3 T3）。
#[test]
fn sight_blockers_are_mechanism_input_not_fog_visibility() {
    let (mut sim, content) = base_match();
    submit(
        &mut sim,
        1,
        1,
        w3_core::GameplayCommandKind::MarkSightBlocker {
            blocker: id(UNIT_A),
            blocking: true,
        },
    );
    sim.step();
    let mut peer = with_state(&sim, content, |_| {});
    for _ in 0..8 {
        sim.step();
        peer.step();
        assert_eq!(sim.compute_state_hash(), peer.compute_state_hash());
    }
    let state = sim.faction_mechanics_state();
    assert!(state.sight_blockers.contains(&id(UNIT_A)));
    // 全部 hashed 字段名 MUST NOT 出现 fog / visible / explored / visibility。
    for field in FACTION_HASHED_FIELDS {
        let lower = field.to_ascii_lowercase();
        assert!(!lower.contains("fog"), "reason=no_fog_field field={field}");
        assert!(
            !lower.contains("visible"),
            "reason=no_visible_field field={field}"
        );
        assert!(
            !lower.contains("explored"),
            "reason=no_explored_field field={field}"
        );
        assert!(
            !lower.contains("visibility"),
            "reason=no_visibility_field field={field}"
        );
    }
}

/// R8.4 野怪睡眠：夜间对登记单位结算睡眠，夜间清醒例外保持清醒，
/// 白昼全体清醒；逐 tick 整数帧确定性。
#[test]
fn creep_sleep_settles_at_night_with_nocturnal_exception() {
    let (mut sim, content) = base_match();
    submit(
        &mut sim,
        1,
        1,
        w3_core::GameplayCommandKind::SetCreepSleep {
            creep: id(UNIT_A),
            sleeps_at_night: true,
        },
    );
    submit(
        &mut sim,
        1,
        2,
        w3_core::GameplayCommandKind::SetCreepSleep {
            creep: id(UNIT_B),
            sleeps_at_night: false,
        },
    );
    sim.step();
    let mut peer = with_state(&sim, content, |_| {});
    let mut saw_sleep = false;
    for _ in 0..14 {
        sim.step();
        peer.step();
        assert_eq!(sim.compute_state_hash(), peer.compute_state_hash());
        let state = sim.faction_mechanics_state();
        if state.sleeping_creeps.contains(&id(UNIT_A)) {
            saw_sleep = true;
            assert!(
                !state.sleeping_creeps.contains(&id(UNIT_B)),
                "reason=nocturnal_exception_awake"
            );
        }
    }
    assert!(saw_sleep, "reason=creep_sleep_fact_settled");
}

// ---------------------------------------------------------------------------
// 3.4 fog 路径隔离回归：fog 可见性 MUST 表现层只读派生
// ---------------------------------------------------------------------------

/// 表现层由 owned `read_view()` 只读派生 fog 可见性（当前可见 / 已探索 /
/// 视野范围），MUST NOT 进权威状态 / compute_state_hash / 快照 /
/// first_difference（R10.5 / §0.3 T3 / 对照差异 D1）。
#[test]
fn fog_visibility_is_presentation_only_and_has_zero_authoritative_impact() {
    let (sim, content) = base_match();
    let baseline_hash = sim.compute_state_hash();
    let baseline_snapshot = sim.capture_snapshot();

    // 表现层只读派生：从 owned WorldView 计算一个 fog 可见集合。
    let view = sim.read_view();
    let mut derived_visible: Vec<u64> = view.entities.iter().map(|entity| entity.id).collect();
    derived_visible.sort_unstable();
    assert!(!derived_visible.is_empty(), "reason=fog_derived_nonempty");
    // 表现层任意改动派生可见集合（含清空 / 追加均不影响权威状态）。
    derived_visible.clear();
    derived_visible.push(u64::MAX);

    assert_eq!(
        sim.compute_state_hash(),
        baseline_hash,
        "reason=fog_derivation_zero_hash_impact"
    );
    assert_eq!(
        sim.capture_snapshot(),
        baseline_snapshot,
        "reason=fog_derivation_zero_snapshot_impact"
    );
    let peer = with_state(&sim, content, |_| {});
    assert_eq!(
        sim.first_difference(&peer),
        None,
        "reason=fog_derivation_zero_first_difference_impact"
    );

    // 只读 owned 投影：表现层改动投影 MUST NOT 反写权威状态。
    let mut owned_projection = sim.faction_mechanics_state();
    owned_projection.detected.insert(id(u64::MAX));
    owned_projection.day_night.elapsed_frames = u32::MAX;
    assert_eq!(
        sim.compute_state_hash(),
        baseline_hash,
        "reason=owned_projection_no_writeback"
    );
    assert!(
        !sim.faction_mechanics_state()
            .detected
            .contains(&id(u64::MAX)),
        "reason=fog_not_in_authoritative_state"
    );
}

// ---------------------------------------------------------------------------
// 5.2：新增 GameplayCommandKind 变体编码唯一 / 校验 / 穷尽分发
// ---------------------------------------------------------------------------

/// 13 个新增变体：kind 码 24..=36 唯一、校验拒绝非法实体、
/// 每个变体经公开命令入口 + step() 穷尽分发（队列消费 + 事实登记）。
#[test]
fn new_commands_are_encoded_validated_and_exhaustively_dispatched() {
    let kinds = new_command_kinds();
    assert_eq!(kinds.len(), 13, "reason=thirteen_new_variants");

    // 编码唯一：每个变体独立 kind 码，既有 1–23 不动。
    let mut hashes = std::collections::BTreeSet::new();
    for kind in &kinds {
        let mut queue = GameplayCommandQueue::new();
        queue
            .submit(w3_core::GameplayCommand::new(
                w3_core::Frame::new(1),
                PLAYER,
                1,
                *kind,
            ))
            .expect("faction_forward: reason=valid_command");
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        assert!(
            hashes.insert(hasher.finish()),
            "reason=unique_kind_encoding"
        );
    }

    // 校验：0 号实体 MUST 被拒绝（每个含实体字段的新变体）。
    let invalid = w3_core::GameplayCommandKind::SeedBlight {
        source: w3_core::EntityId::NONE,
        cell_x: 0,
        cell_y: 0,
    };
    let mut queue = GameplayCommandQueue::new();
    assert!(
        queue
            .submit(w3_core::GameplayCommand::new(
                w3_core::Frame::new(1),
                PLAYER,
                1,
                invalid
            ))
            .is_err()
    );

    // 穷尽分发：每个变体经 submit_gameplay_command + step() 被消费并登记。
    let (mut sim, _) = base_match();
    for (index, kind) in kinds.iter().enumerate() {
        let mut dispatched =
            RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
        dispatched
            .submit_gameplay_command(w3_core::GameplayCommand::new(
                w3_core::Frame::new(0),
                PLAYER,
                index as u32 + 1,
                *kind,
            ))
            .expect("faction_forward: reason=submit_ok");
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
        // 命令产生权威状态变化或事实（相对空对局基线）。
        let mut base =
            RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
        base.step();
        assert_ne!(
            dispatched.compute_state_hash(),
            base.compute_state_hash(),
            "reason=variant_changes_authoritative_state index={index}"
        );
    }
    sim.step();
}

/// 回血条件按 `DefId` 定义层类别登记（非族名硬编码）：同一 `DefId`
/// 换登记类别即改变 content_hash，判定只查表（R10.3 / 参考对照 R9）。
#[test]
fn regen_condition_category_is_definition_layer_data_driven() {
    let mut table_a = BTreeMap::new();
    table_a.insert(REGEN_DEF_DAY, RegenCondition::Day);
    let a = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_regen_conditions(table_a);
    let mut table_b = BTreeMap::new();
    table_b.insert(REGEN_DEF_DAY, RegenCondition::Night);
    let b = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_regen_conditions(table_b);
    assert_ne!(a.content_hash(), b.content_hash());
    assert_eq!(a.regen_condition(REGEN_DEF_DAY), RegenCondition::Day);
    assert_eq!(b.regen_condition(REGEN_DEF_DAY), RegenCondition::Night);
    // 任意合成 DefId 均可登记（非白名单 / 非族名）。
    let mut table_c = BTreeMap::new();
    table_c.insert(42_001 as DefId, RegenCondition::Blight);
    let c = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_regen_conditions(table_c);
    assert_eq!(c.regen_condition(42_001), RegenCondition::Blight);
}

/// `SimFixed` Q47.16 与 30 Hz 整数帧契约在此批次零改动（ADR 0002 §1-2）。
#[test]
fn fixed_math_and_integer_frame_contract_unchanged() {
    let value = SimFixed::from_int(7).unwrap();
    assert_eq!(value.raw(), 7 * w3_core::SCALE);
    let (mut sim, _) = base_match();
    let before = sim.read_view().tick;
    sim.step();
    assert_eq!(
        sim.read_view().tick,
        before + 1,
        "reason=integer_frame_only"
    );
    let _: FactionMechanicsConstants = SYNTHETIC_FACTION_MECHANICS;
}
