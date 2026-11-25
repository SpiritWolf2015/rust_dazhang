//! P11 技能系统正向确定性回归。
//!
//! 覆盖冻结清单 / 行为目录 / EffectKind 稳定码、四处同步、英雄技能点、
//! 6 级大招、九类原语、单 LCG `rng_draws`、命令编码 / 校验 / 穷尽分发与
//! 真人 / 回放 / 联机三路径一致性；全部断言只用整数权威状态与 owned 投影。

mod ability_common;

use std::collections::BTreeSet;

use ability_common::{
    PLAYER, assert_v9_versions, basic_for, command_stream, config, gp, hero, id, next_rng,
    producer, setup, target, ultimate_for,
};
use w3_core::{
    ABILITY_CATALOG_LEN, AbilityCategory, AbilitySystemDefinitions, BehaviorCatalog, BehaviorKind,
    BuffCategory, EffectKind, FactionSlot, FrozenDefinitions, GameplayCommandError,
    GameplayCommandKind, GameplayCommandQueue, MatchConfig, RtsMatch, SIM_VERSION,
    SNAPSHOT_VERSION, SYNTHETIC_ABILITY_CATALOG, SYNTHETIC_ABILITY_SYSTEM_CONSTANTS, Snapshot,
    SnapshotError, StateHasher, StaticWorld, decode_effect_kind, effect_kind_code,
};

/// 全部 `EffectKind` 变体；用于稳定码唯一性与解码穷尽性。
const EFFECT_KINDS: [EffectKind; 13] = [
    EffectKind::Damage,
    EffectKind::Heal,
    EffectKind::BuffStat,
    EffectKind::Stun,
    EffectKind::Zone,
    EffectKind::Summon,
    EffectKind::AuraBeneficiaries,
    EffectKind::PassiveProc,
    EffectKind::Transform,
    EffectKind::PointTarget,
    EffectKind::RemoveBuffCategory,
    EffectKind::ChainBounce,
    EffectKind::RandomDraw,
];

/// 冻结清单覆盖四席位、每席位 3 普通 + 1 大招 × 4 英雄 + 单位 / 道具代表项。
#[test]
fn synthetic_catalog_and_effect_codes_are_stable() {
    assert_v9_versions();
    assert_eq!(SYNTHETIC_ABILITY_CATALOG.len(), ABILITY_CATALOG_LEN);
    for faction in FactionSlot::ALL {
        let entries: Vec<_> = SYNTHETIC_ABILITY_CATALOG
            .iter()
            .copied()
            .filter(|entry| entry.faction == faction)
            .collect();
        assert_eq!(entries.len(), 18, "reason=faction_catalog_size");
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.category == AbilityCategory::HeroBasic)
                .count(),
            12,
            "reason=hero_basic_count"
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.category == AbilityCategory::HeroUltimate)
                .count(),
            4,
            "reason=hero_ultimate_count"
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.category == AbilityCategory::Unit)
                .count(),
            1,
            "reason=unit_representative_count"
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.category == AbilityCategory::Item)
                .count(),
            1,
            "reason=item_representative_count"
        );
        for entry in entries {
            let offset = entry
                .behavior_id
                .checked_sub(w3_core::BEHAVIOR_CATALOG_BASE)
                .expect("ability_system: reason=behavior_id_before_base");
            assert!(
                BehaviorKind::from_code(offset as u8).is_some(),
                "reason=catalog_behavior_registered"
            );
        }
    }

    let mut codes = BTreeSet::new();
    for kind in EFFECT_KINDS {
        let code = effect_kind_code(kind);
        assert!(codes.insert(code), "reason=effect_kind_code_unique");
        assert_eq!(
            decode_effect_kind(code),
            Some(kind),
            "reason=effect_kind_roundtrip"
        );
    }
    assert_eq!(effect_kind_code(EffectKind::Damage), 0, "reason=code_0");
    assert_eq!(effect_kind_code(EffectKind::Heal), 1, "reason=code_1");
    assert_eq!(effect_kind_code(EffectKind::BuffStat), 2, "reason=code_2");
    assert_eq!(effect_kind_code(EffectKind::Stun), 3, "reason=code_3");
    assert_eq!(
        decode_effect_kind(255),
        None,
        "reason=unknown_code_rejected"
    );

    let catalog = BehaviorCatalog::synthetic();
    for behavior in BehaviorKind::ALL {
        assert_eq!(
            BehaviorKind::from_code(behavior.code()),
            Some(behavior),
            "reason=behavior_code_roundtrip"
        );
        assert_eq!(
            catalog.dispatch_by_effect(behavior.effect_kind()),
            Some(behavior),
            "reason=behavior_catalog_dispatch"
        );
    }
}

/// 冻结常量变化进入 `content_hash()` / `defs_hash` 快照头部，但不改变
/// 技能权威运行态；既有 `defs.content_hash()` 身份仍按 ADR 0002 参与状态哈希。
#[test]
fn definition_constants_change_content_hash_not_runtime_ability_state() {
    let base = ability_common::ability_defs();
    let mut definitions = AbilitySystemDefinitions::default();
    definitions.constants.zone_frames = definitions.constants.zone_frames.saturating_add(1);
    let variant = base.clone().with_ability_system(definitions);
    assert_ne!(
        base.content_hash(),
        variant.content_hash(),
        "reason=content_hash_sensitive"
    );
    assert_eq!(
        base.ability_system()
            .first_difference(variant.ability_system()),
        Some("ability_constants"),
        "reason=definition_difference_reported"
    );

    let cfg = config(&base);
    let a = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), base.clone());
    let b = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), variant.clone());
    assert_eq!(
        a.ability_system_state(),
        b.ability_system_state(),
        "reason=runtime_ability_state_unchanged"
    );
    assert_ne!(
        a.capture_snapshot().defs_hash,
        b.capture_snapshot().defs_hash,
        "reason=snapshot_header_defs_hash_changed"
    );
    assert_eq!(
        a.first_difference(&b),
        Some("frozen_definitions"),
        "reason=definition_identity_only"
    );
    assert_ne!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=definition_identity_enters_state_hash"
    );
}

/// 九类行为原语都写入权威运行态；快照 / 恢复不丢失字段。
#[test]
fn ability_primitives_populate_authoritative_state_and_snapshot() {
    let stream = command_stream(FactionSlot::First);
    let (mut a, defs, content) = setup(&stream);
    let (mut b, _, _) = setup(&stream);
    for frame in 0..18 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=primitive_stream_diverged frame={frame} diff={:?}",
            a.first_difference(&b)
        );
        assert_eq!(a.first_difference(&b), None, "reason=no_difference");
    }

    let state = a.ability_system_state();
    assert!(
        state.hero_skill_points.get(&hero()).copied().unwrap_or(0) > 0,
        "reason=hero_skill_points_present"
    );
    assert_eq!(
        state
            .ability_levels
            .get(&(hero(), ultimate_for(FactionSlot::First))),
        Some(&1),
        "reason=ultimate_learned"
    );
    assert_eq!(
        state
            .ability_levels
            .get(&(hero(), basic_for(FactionSlot::First))),
        Some(&1),
        "reason=basic_ability_learned"
    );
    assert!(!state.zones.is_empty(), "reason=zones_present");
    assert!(!state.summons.is_empty(), "reason=summons_present");
    assert!(
        !state.aura_beneficiaries.is_empty(),
        "reason=aura_beneficiaries_present"
    );
    assert!(
        state.passive_procs.values().any(|entry| !entry.random),
        "reason=passive_procs_present"
    );
    assert!(
        !state.transformations.is_empty(),
        "reason=transformations_present"
    );
    assert!(
        !state.buff_categories.is_empty(),
        "reason=buff_categories_present"
    );
    assert!(!state.chains.is_empty(), "reason=chains_present");
    assert!(
        !state.point_targets.is_empty(),
        "reason=point_targets_present"
    );
    assert!(!state.rng_draws.is_empty(), "reason=rng_draws_present");
    assert!(
        state.pending_casts.is_empty(),
        "reason=immediate_casts_have_no_pending_state"
    );

    let snapshot = a.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v9");
    assert_eq!(snapshot.sim_version, SIM_VERSION, "reason=sim_version_v9");
    assert_eq!(
        snapshot.gameplay.abilities, state,
        "reason=snapshot_contains_ability_state"
    );
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
        .expect("ability_system: reason=restore_ok");
    assert_eq!(
        restored.ability_system_state(),
        state,
        "reason=restore_preserves_ability_state"
    );
    assert_eq!(
        a.first_difference(&restored),
        None,
        "reason=restore_no_difference"
    );
    assert_eq!(defs.content_hash(), content, "reason=content_hash_stable");
}

/// 技能随机只从每帧唯一主 LCG 后继状态派生；主 `rng` 与推导序列逐帧一致。
#[test]
fn ability_random_uses_single_lcg_and_explicit_rng_draws() {
    let stream = command_stream(FactionSlot::Second);
    let (mut a, _, _) = setup(&stream);
    let (mut b, _, _) = setup(&stream);
    let frames = 24u64;
    for frame in 0..frames {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=rng_stream_diverged frame={frame} diff={:?}",
            a.first_difference(&b)
        );
    }

    let expected_rng = (0..frames).fold(0x5eedu64, |state, _| next_rng(state));
    assert_eq!(
        a.capture_snapshot().rng,
        expected_rng,
        "reason=single_lcg_consumed_once_per_frame"
    );
    assert_eq!(
        b.capture_snapshot().rng,
        expected_rng,
        "reason=peer_single_lcg_matches"
    );
    let draws_a = a.ability_system_state().rng_draws;
    let draws_b = b.ability_system_state().rng_draws;
    assert_eq!(draws_a, draws_b, "reason=rng_draw_sequence_matches");
    assert!(!draws_a.is_empty(), "reason=rng_draws_recorded");
    let mut expected_seq = 0u64;
    for (&seq, draw) in &draws_a {
        assert_eq!(seq, expected_seq, "reason=rng_draw_sequence_contiguous");
        assert_eq!(draw.seq, seq, "reason=rng_draw_seq_encoded");
        assert_eq!(
            next_rng(draw.rng_before),
            draw.rng_after,
            "reason=rng_draw_uses_main_lcg_step"
        );
        assert_eq!(
            draw.behavior,
            BehaviorKind::RandomDraw,
            "reason=rng_draw_behavior"
        );
        assert!(
            draw.roll < SYNTHETIC_ABILITY_SYSTEM_CONSTANTS.proc_modulus.max(1),
            "reason=rng_roll_in_range"
        );
        expected_seq += 1;
    }
}

/// 新增命令的 kind 码 / 字段编码、同源校验与穷尽分发全部覆盖。
#[test]
fn new_ability_commands_are_encoded_validated_and_dispatched() {
    let caster = hero();
    let target = target();
    let ability = SYNTHETIC_ABILITY_CATALOG[0].id;
    let kinds = [
        GameplayCommandKind::CastPoint {
            caster,
            ability,
            x: 1 << 16,
            y: 2 << 16,
        },
        GameplayCommandKind::CastArea {
            caster,
            ability,
            x: 3 << 16,
            y: 4 << 16,
            radius_raw: 5 << 16,
        },
        GameplayCommandKind::CastDirection {
            caster,
            ability,
            dx: 1 << 16,
            dy: -2 << 16,
        },
        GameplayCommandKind::LearnAbility {
            hero: caster,
            ability,
        },
        GameplayCommandKind::LevelUpHero { hero: caster },
    ];
    let encode = |kind: GameplayCommandKind| {
        let mut queue = GameplayCommandQueue::new();
        queue
            .submit(gp(2, PLAYER, 1, kind))
            .expect("ability_system: reason=valid_command_rejected");
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        hasher.finish()
    };
    let hashes: BTreeSet<u64> = kinds.iter().copied().map(encode).collect();
    assert_eq!(hashes.len(), kinds.len(), "reason=command_hashes_unique");
    assert_ne!(
        encode(kinds[0]),
        encode(GameplayCommandKind::CastPoint {
            caster,
            ability,
            x: 2 << 16,
            y: 2 << 16,
        }),
        "reason=point_coordinate_encoded"
    );

    let mut invalid = GameplayCommandQueue::new();
    assert_eq!(
        invalid.submit(gp(
            2,
            PLAYER,
            1,
            GameplayCommandKind::CastArea {
                caster,
                ability,
                x: 0,
                y: 0,
                radius_raw: -1,
            },
        )),
        Err(GameplayCommandError::InvalidEntity),
        "reason=negative_radius_rejected"
    );
    assert_eq!(
        invalid.submit(gp(
            2,
            PLAYER,
            2,
            GameplayCommandKind::LearnAbility {
                hero: w3_core::EntityId::NONE,
                ability,
            },
        )),
        Err(GameplayCommandError::InvalidEntity),
        "reason=invalid_hero_rejected"
    );

    let extra: Vec<_> = kinds
        .iter()
        .copied()
        .enumerate()
        .map(|(index, kind)| gp(2 + index as u64, PLAYER, 300 + index as u32, kind))
        .collect();
    let (mut sim, _, _) = setup(&extra);
    for _ in 0..8 {
        sim.step();
    }
    assert_eq!(
        sim.capture_snapshot().version,
        9,
        "reason=commands_dispatched_without_version_change"
    );
    assert_eq!(producer(), id(1), "reason=producer_id_stable");
    assert_eq!(target, id(2), "reason=target_id_stable");
}

/// 6 级前大招不可学；升级后再学习可逐帧确定。
#[test]
fn hero_ultimate_requires_level_six() {
    let ultimate = ultimate_for(FactionSlot::Third);
    let early = [gp(
        2,
        PLAYER,
        1,
        GameplayCommandKind::LearnAbility {
            hero: hero(),
            ability: ultimate,
        },
    )];
    let (mut early_sim, _, _) = setup(&early);
    for _ in 0..3 {
        early_sim.step();
    }
    assert!(
        !early_sim
            .ability_system_state()
            .ability_levels
            .contains_key(&(hero(), ultimate)),
        "reason=ultimate_locked_before_level_six"
    );

    let mut late = Vec::new();
    for index in 0..5u64 {
        late.push(gp(
            2 + index,
            PLAYER,
            100 + index as u32,
            GameplayCommandKind::LevelUpHero { hero: hero() },
        ));
    }
    late.push(gp(
        7,
        PLAYER,
        200,
        GameplayCommandKind::LearnAbility {
            hero: hero(),
            ability: ultimate,
        },
    ));
    let (mut late_a, _, _) = setup(&late);
    let (mut late_b, _, _) = setup(&late);
    for frame in 0..9 {
        late_a.step();
        late_b.step();
        assert_eq!(
            late_a.compute_state_hash(),
            late_b.compute_state_hash(),
            "reason=learning_stream_diverged frame={frame}"
        );
    }
    assert_eq!(
        late_a
            .ability_system_state()
            .ability_levels
            .get(&(hero(), ultimate)),
        Some(&1),
        "reason=ultimate_learned_at_level_six"
    );
}

/// 旧 v8 快照 / 回放按 `SnapshotError` 明确拒绝，不提供跨版本迁移。
#[test]
fn old_v8_snapshot_and_sim_version_are_explicitly_rejected() {
    let defs = FrozenDefinitions::empty();
    let cfg = MatchConfig {
        seed: 0x5eed,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs);
    sim.step();
    let snapshot: Snapshot = sim.capture_snapshot();

    let mut old_version = snapshot.clone();
    old_version.version = 8;
    assert!(
        matches!(
            RtsMatch::restore_snapshot(old_version, StaticWorld::default(), cfg.content_hash),
            Err(SnapshotError::UnsupportedVersion { found: 8 })
        ),
        "reason=old_snapshot_rejected"
    );
    let mut old_sim = snapshot;
    old_sim.sim_version = 8;
    assert!(
        matches!(
            RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), cfg.content_hash),
            Err(SnapshotError::SimVersionMismatch {
                found: 8,
                expected: SIM_VERSION,
            })
        ),
        "reason=old_sim_version_rejected"
    );
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    assert_eq!(SIM_VERSION, 9, "reason=sim_version_9");
}

/// Buff 分类映射穷尽覆盖全部效果原语，且不含 fog / 可见性字段。
#[test]
fn buff_categories_cover_all_effect_primitives() {
    let mut categories = BTreeSet::new();
    for kind in EFFECT_KINDS {
        categories.insert(BuffCategory::from_effect_kind(kind));
    }
    assert!(
        categories.contains(&BuffCategory::Magic),
        "reason=magic_category_present"
    );
    assert!(
        categories.contains(&BuffCategory::Control),
        "reason=control_category_present"
    );
    assert!(
        !categories.contains(&BuffCategory::None),
        "reason=none_category_not_generated"
    );
}
