//! P11 技能系统形态①反向验证（测试内反例 + 元断言防退化）。
//!
//! 覆盖 `ABILITY_HASHED_FIELDS` 中每个新增权威运行态字段：正式实现经
//! `compute_state_hash()` / `capture_snapshot()` / `restore_snapshot()` /
//! `first_difference()` 生产路径检测差异；退化态分别模拟漏写哈希、快照、
//! 差异诊断或恢复字段，同一断言必须失败。成对测试名 / 元断言即证据。

mod ability_common;

use std::collections::BTreeSet;

use ability_common::{PLAYER, config, hero, id, next_rng, point_target, target};
use w3_core::{
    ABILITY_CATALOG_BASE, ABILITY_CHAIN_ID_BASE, ABILITY_HASHED_FIELDS, ABILITY_ZONE_ID_BASE,
    AbilityRngDraw, AbilitySystemState, AuraBeneficiarySet, BehaviorKind, BuffCategory, ChainState,
    PassiveProcState, PendingAbilityCast, PointTargetState, RtsMatch, Snapshot, StateHasher,
    StaticWorld, SummonState, TransformState, ZoneState,
};

/// 供所有反向验证复用的基线快照；不注入技能命令，技能运行态保持默认值。
fn base_snapshot() -> (Snapshot, u64) {
    let defs = ability_common::ability_defs();
    let cfg = config(&defs);
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
    sim.step();
    (sim.capture_snapshot(), cfg.content_hash)
}

/// 构造只改动一个字段的运行态变体；字段名来自 `ABILITY_HASHED_FIELDS`。
fn state_variant(field: &str) -> AbilitySystemState {
    let mut state = AbilitySystemState::default();
    let ability = ABILITY_CATALOG_BASE;
    match field {
        "ability_hero_skill_points" => {
            state.hero_skill_points.insert(hero(), 3);
        }
        "ability_levels" => {
            state.ability_levels.insert((hero(), ability), 2);
        }
        "ability_zones" => {
            let zone_id = id(ABILITY_ZONE_ID_BASE);
            state.zones.insert(
                zone_id,
                ZoneState {
                    id: zone_id,
                    source: hero(),
                    ability,
                    owner: PLAYER,
                    x: 1 << 16,
                    y: 2 << 16,
                    radius_raw: 3 << 16,
                    remaining: 7,
                    beneficiaries: BTreeSet::from([id(1), id(2)]),
                },
            );
        }
        "ability_summons" => {
            let summon_id = id(4);
            state.summons.insert(
                summon_id,
                SummonState {
                    summon: summon_id,
                    owner: hero(),
                    player: PLAYER,
                    ability,
                    remaining: 7,
                },
            );
        }
        "ability_aura_beneficiaries" => {
            state.aura_beneficiaries.insert(
                hero(),
                AuraBeneficiarySet {
                    source: hero(),
                    ability,
                    beneficiaries: BTreeSet::from([id(1), id(2)]),
                },
            );
        }
        "ability_passive_procs" => {
            state.passive_procs.insert(
                (hero(), ability),
                PassiveProcState {
                    subscribed_frame: 2,
                    remaining: 7,
                    random: false,
                },
            );
        }
        "ability_transformations" => {
            state.transformations.insert(
                hero(),
                TransformState {
                    ability,
                    state_code: 1,
                    remaining: 7,
                },
            );
        }
        "ability_buff_categories" => {
            state
                .buff_categories
                .insert(target(), BTreeSet::from([BuffCategory::Magic]));
        }
        "ability_chains" => {
            let chain_id = id(ABILITY_CHAIN_ID_BASE);
            state.chains.insert(
                chain_id,
                ChainState {
                    id: chain_id,
                    source: hero(),
                    target: target(),
                    ability,
                    bounces: 3,
                    remaining: 7,
                },
            );
        }
        "ability_point_targets" => {
            state.point_targets.insert(
                hero(),
                PointTargetState {
                    caster: hero(),
                    ability,
                    target: point_target(),
                    frame: 2,
                },
            );
        }
        "ability_pending_casts" => {
            state.pending_casts.insert(
                hero(),
                PendingAbilityCast {
                    caster: hero(),
                    ability,
                    target: point_target(),
                    player: PLAYER,
                    frame: 2,
                },
            );
        }
        "ability_rng_draws" => {
            let rng_before = 0x5eedu64;
            state.rng_draws.insert(
                0,
                AbilityRngDraw {
                    seq: 0,
                    frame: 2,
                    unit: hero(),
                    ability,
                    rng_before,
                    rng_after: next_rng(rng_before),
                    roll: 7,
                    behavior: BehaviorKind::RandomDraw,
                },
            );
        }
        "ability_next_zone_id" => {
            state.next_zone_id = state.next_zone_id.saturating_add(1);
        }
        "ability_next_chain_id" => {
            state.next_chain_id = state.next_chain_id.saturating_add(1);
        }
        "ability_next_rng_seq" => {
            state.next_rng_seq = state.next_rng_seq.saturating_add(1);
        }
        _ => panic!("reason=unknown_ability_field field={field}"),
    }
    state
}

/// 退化态：把指定字段从权威运行态 / 快照中抹去，模拟漏写四处同步之一。
fn clear_field(state: &mut AbilitySystemState, field: &str) {
    match field {
        "ability_hero_skill_points" => state.hero_skill_points.clear(),
        "ability_levels" => state.ability_levels.clear(),
        "ability_zones" => state.zones.clear(),
        "ability_summons" => state.summons.clear(),
        "ability_aura_beneficiaries" => state.aura_beneficiaries.clear(),
        "ability_passive_procs" => state.passive_procs.clear(),
        "ability_transformations" => state.transformations.clear(),
        "ability_buff_categories" => state.buff_categories.clear(),
        "ability_chains" => state.chains.clear(),
        "ability_point_targets" => state.point_targets.clear(),
        "ability_pending_casts" => state.pending_casts.clear(),
        "ability_rng_draws" => state.rng_draws.clear(),
        "ability_next_zone_id" => state.next_zone_id = ABILITY_ZONE_ID_BASE,
        "ability_next_chain_id" => state.next_chain_id = ABILITY_CHAIN_ID_BASE,
        "ability_next_rng_seq" => state.next_rng_seq = 0,
        _ => panic!("reason=unknown_ability_field field={field}"),
    }
}

/// 把运行态注入真实快照并经 `restore_snapshot()` 恢复；返回状态哈希。
fn restored_hash(state: &AbilitySystemState, base: &Snapshot, content: u64) -> u64 {
    let mut snapshot = base.clone();
    snapshot.gameplay.abilities = state.clone();
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
        .expect("reverse_validation_ability_system: reason=restore_ok")
        .compute_state_hash()
}

/// 正式 / 退化共用的同步断言：哈希、差异字段、快照、恢复四处必须同时成立。
fn field_is_synchronized(
    base: &Snapshot,
    variant: &Snapshot,
    content: u64,
    field: &'static str,
    omit_field: bool,
) -> bool {
    let mut formal_base = base.clone();
    let mut formal_variant = variant.clone();
    if omit_field {
        clear_field(&mut formal_base.gameplay.abilities, field);
        clear_field(&mut formal_variant.gameplay.abilities, field);
    }
    let a = RtsMatch::restore_snapshot(formal_base.clone(), StaticWorld::default(), content)
        .expect("reverse_validation_ability_system: reason=base_restore_ok");
    let b = RtsMatch::restore_snapshot(formal_variant.clone(), StaticWorld::default(), content)
        .expect("reverse_validation_ability_system: reason=variant_restore_ok");
    let hash_detects = a.compute_state_hash() != b.compute_state_hash();
    let difference_detects = a.first_difference(&b) == Some(field);
    let snapshot_detects = a.capture_snapshot().gameplay.abilities
        == formal_base.gameplay.abilities
        && b.capture_snapshot().gameplay.abilities == formal_variant.gameplay.abilities;
    let restore_detects = a.ability_system_state() == formal_base.gameplay.abilities
        && b.ability_system_state() == formal_variant.gameplay.abilities;
    hash_detects && difference_detects && snapshot_detects && restore_detects
}

/// 稳定字段清单：无重复、数量与四处同步实现一致。
#[test]
fn ability_hashed_field_set_is_stable() {
    let fields: BTreeSet<&str> = ABILITY_HASHED_FIELDS.iter().copied().collect();
    assert_eq!(
        fields.len(),
        ABILITY_HASHED_FIELDS.len(),
        "reason=hashed_fields_unique"
    );
    assert_eq!(ABILITY_HASHED_FIELDS.len(), 15, "reason=hashed_field_count");
    assert!(
        ABILITY_HASHED_FIELDS.contains(&"ability_rng_draws"),
        "reason=rng_draws_registered"
    );
}

/// 正式实现：每个新权威字段都能经四处同步生产路径检测到差异。
#[test]
fn reverse_validation_ability_state_sync_formal_passes_the_assertion() {
    let (base, content) = base_snapshot();
    assert_eq!(
        base.gameplay.abilities,
        AbilitySystemState::default(),
        "reason=baseline_state_is_default"
    );
    for &field in ABILITY_HASHED_FIELDS {
        let mut variant = base.clone();
        variant.gameplay.abilities = state_variant(field);
        assert!(
            field_is_synchronized(&base, &variant, content, field, false),
            "reason=formal_field_synchronized field={field}"
        );
    }
}

/// 退化实现：抹去同一字段后，四处同步的同一断言必失败；元断言防恒真。
#[test]
fn reverse_validation_ability_state_sync_degenerate_fails_the_same_assertion() {
    let (base, content) = base_snapshot();
    for &field in ABILITY_HASHED_FIELDS {
        let mut variant = base.clone();
        variant.gameplay.abilities = state_variant(field);
        assert!(
            !field_is_synchronized(&base, &variant, content, field, true),
            "reason=degenerate_must_fail_same_assertion field={field}"
        );
        assert!(
            field_is_synchronized(&base, &variant, content, field, false),
            "reason=formal_still_passes field={field}"
        );
    }
}

/// 退化哈希：把字段从快照恢复前抹去，正式 `compute_state_hash()` 能检测，
/// 退化态两个不同字段变体哈希相同；元断言保证正式态未退化。
#[test]
fn reverse_validation_hash_omission_degenerate_fails_the_same_assertion() {
    let (base, content) = base_snapshot();
    for &field in ABILITY_HASHED_FIELDS {
        let formal_base = AbilitySystemState::default();
        let formal_variant = state_variant(field);
        let formal_a = restored_hash(&formal_base, &base, content);
        let formal_b = restored_hash(&formal_variant, &base, content);
        assert_ne!(
            formal_a, formal_b,
            "reason=formal_hash_detects_field field={field}"
        );

        let mut degenerate_base = AbilitySystemState::default();
        let mut degenerate_variant = state_variant(field);
        clear_field(&mut degenerate_base, field);
        clear_field(&mut degenerate_variant, field);
        assert_eq!(
            restored_hash(&degenerate_base, &base, content),
            restored_hash(&degenerate_variant, &base, content),
            "reason=degenerate_hash_blind_to_field field={field}"
        );
        assert_ne!(
            formal_a, formal_b,
            "reason=formal_hash_still_sensitive field={field}"
        );
    }
}

/// 退化差异诊断：正式 `first_difference()` 返回字段名，退化实现恒返回 `None`。
#[test]
fn reverse_validation_first_difference_omission_degenerate_fails_the_same_assertion() {
    let (base, content) = base_snapshot();
    let degenerate_first_difference = |_: &RtsMatch, _: &RtsMatch| None::<&'static str>;
    for &field in ABILITY_HASHED_FIELDS {
        let mut variant = base.clone();
        variant.gameplay.abilities = state_variant(field);
        let a = RtsMatch::restore_snapshot(base.clone(), StaticWorld::default(), content)
            .expect("reverse_validation_ability_system: reason=base_restore_ok");
        let b = RtsMatch::restore_snapshot(variant, StaticWorld::default(), content)
            .expect("reverse_validation_ability_system: reason=variant_restore_ok");
        assert_eq!(
            a.first_difference(&b),
            Some(field),
            "reason=formal_first_difference field={field}"
        );
        assert_eq!(
            degenerate_first_difference(&a, &b),
            None,
            "reason=degenerate_first_difference_blind field={field}"
        );
    }
}

/// 退化快照 / 恢复：正式捕获保留字段，退化捕获前抹去字段后恢复必丢失。
#[test]
fn reverse_validation_snapshot_and_restore_omission_degenerate_fails_the_same_assertion() {
    let (base, content) = base_snapshot();
    for &field in ABILITY_HASHED_FIELDS {
        let expected = state_variant(field);
        let mut formal = base.clone();
        formal.gameplay.abilities = expected.clone();
        let restored = RtsMatch::restore_snapshot(formal, StaticWorld::default(), content)
            .expect("reverse_validation_ability_system: reason=formal_restore_ok");
        assert_eq!(
            restored.capture_snapshot().gameplay.abilities,
            expected,
            "reason=formal_snapshot_preserves_field field={field}"
        );

        let mut degenerate = base.clone();
        degenerate.gameplay.abilities = expected.clone();
        clear_field(&mut degenerate.gameplay.abilities, field);
        let restored_degenerate =
            RtsMatch::restore_snapshot(degenerate, StaticWorld::default(), content)
                .expect("reverse_validation_ability_system: reason=degenerate_restore_ok");
        assert_eq!(
            restored_degenerate.capture_snapshot().gameplay.abilities,
            AbilitySystemState::default(),
            "reason=degenerate_snapshot_loses_field field={field}"
        );
        assert_ne!(
            restored_degenerate.capture_snapshot().gameplay.abilities,
            expected,
            "reason=degenerate_must_not_equal_formal field={field}"
        );
    }
}

/// 退化定义哈希：技能冻结常量漏接 `compute_defs_hash` 时内容哈希不变，
/// 正式实现改常量必改变 `content_hash()` 与快照头部 `defs_hash`。
#[test]
fn reverse_validation_defs_hash_sensitivity_degenerate_fails_the_same_assertion() {
    let base = ability_common::ability_defs();
    let mut definitions = w3_core::AbilitySystemDefinitions::default();
    definitions.constants.zone_frames = definitions.constants.zone_frames.saturating_add(1);
    let variant = base.clone().with_ability_system(definitions);
    assert!(
        base.content_hash() != variant.content_hash()
            && base
                .ability_system()
                .first_difference(variant.ability_system())
                .is_some(),
        "reason=formal_defs_hash_sensitive"
    );

    let degenerate_content_hash = |_: &w3_core::AbilitySystemDefinitions| {
        let mut hasher = StateHasher::new();
        w3_core::AbilitySystemDefinitions::default().hash(&mut hasher);
        hasher.finish()
    };
    let degenerate_detects = degenerate_content_hash(base.ability_system())
        != degenerate_content_hash(variant.ability_system());
    assert!(
        !degenerate_detects,
        "reason=degenerate_defs_hash_blind_to_constants"
    );
    assert!(
        base.content_hash() != variant.content_hash(),
        "reason=formal_defs_hash_still_sensitive"
    );

    let cfg = config(&base);
    let a = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), base.clone());
    let b = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), variant.clone());
    assert_ne!(
        a.capture_snapshot().defs_hash,
        b.capture_snapshot().defs_hash,
        "reason=formal_snapshot_header_sensitive"
    );
    assert_eq!(
        a.ability_system_state(),
        b.ability_system_state(),
        "reason=runtime_ability_state_not_mixed_into_definitions"
    );
}
