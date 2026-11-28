//! P18 apply 第 2 批：八类 CPU 确定性回归（tasks §5.1 / spec Requirement 11）。
//!
//! 八类（每类一个独立 `#[test]`，全部经 `RtsMatch` 公开 API 连到生产路径）：
//! 三类机制接线 / 买活半血无蓝 / 经验阈值分档 / 克制矩阵 / 回购交互距离 /
//! leash 三级归巢 / v10 快照往返 / 旧 v9 明确拒绝。
//!
//! 每类都带双实例交错逐帧 `compute_state_hash()` 一致或
//! `capture_snapshot()` / `restore_snapshot()` 往返逐字节一致，MUST NOT
//! 由表现层反写权威状态（ADR 0002 §5-8 / `CODEBUDDY.md §5`）。
//!
//! 形态① 成对反例（改坏验红）在 `reverse_validation_p18_mechanisms.rs`；
//! 本文件只放正式态正向回归。全部数值为编译期合成整数 `[待核]`，
//! 零文件 IO、零资产、不依赖其它 crate（ADR 0001 / 0004）。

mod creep_common;
mod item_shop_common;

use creep_common::{attack, seeded_match, with_hero};
use item_shop_common as shop;
use std::collections::BTreeSet;
use w3_core::{
    Command, CommandKind, EntityId, Frame, GameplayCommand, GameplayCommandKind, HeroDeathRecord,
    HeroState, LeashPhase, MECHANISM_BUILDING_XP_SWITCH, MatchConfig, RtsMatch, SCALE, SIM_VERSION,
    SNAPSHOT_VERSION, SimFixed, SnapshotError, StaticWorld,
};

/// 两个同脚本英雄对局（各自独立的冻结定义 / 开局脚本），供逐帧交错比对。
fn two_hero_matches() -> (RtsMatch, RtsMatch) {
    let (a, content_a) = seeded_match();
    let (b, content_b) = seeded_match();
    (with_hero(a, content_a), with_hero(b, content_b))
}

/// 提交同一击杀脚本后交错推进，逐帧哈希 / `first_difference()` 一致。
fn assert_interleaved_hashes(a: &mut RtsMatch, b: &mut RtsMatch, frames: usize) {
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=same_script_same_initial_hash"
    );
    for _ in 0..frames {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=per_frame_hash_matches_baseline"
        );
        assert_eq!(a.first_difference(b), None, "reason=no_first_difference");
    }
}

/// 干净环境对局：三个 frame-0 `Spawn` 实体（合成占位 `[待核]`）。
fn three_unit_match() -> (RtsMatch, u64) {
    let mut sim = RtsMatch::create(MatchConfig::default());
    for sequence in 1..=3u32 {
        sim.submit_command(Command::new(
            Frame::new(0),
            1,
            sequence,
            CommandKind::Spawn {
                x: SimFixed::from_int(sequence as i64 - 1).unwrap(),
                y: SimFixed::ZERO,
                hp: 100,
                speed: SimFixed::from_int(3).unwrap(),
                attack_damage: 0,
                attack_range: SimFixed::from_int(1).unwrap(),
            },
        ))
        .unwrap();
    }
    sim.step();
    let content = sim.config().content_hash;
    (sim, content)
}

/// 注入阵亡英雄死亡登记与资源（经真实 capture/restore 生产路径）。
fn dead_hero_match(gold: i64) -> (RtsMatch, u64, EntityId, EntityId) {
    let (sim, content) = three_unit_match();
    let hero = EntityId::new(1).unwrap();
    let tavern = EntityId::new(2).unwrap();
    let mut snapshot = sim.capture_snapshot();
    {
        let record = snapshot
            .positions
            .iter_mut()
            .find(|p| p.entity == hero.value())
            .unwrap();
        record.alive = false;
        record.hp = 0;
        record.gameplay.hero = Some(HeroState { level: 3, xp: 0 });
        record.gameplay.max_mana = 100;
        record.gameplay.mana = 100;
    }
    let death_gameplay = snapshot
        .positions
        .iter()
        .find(|p| p.entity == hero.value())
        .unwrap()
        .gameplay
        .clone();
    snapshot.revive.deaths.insert(
        hero,
        HeroDeathRecord {
            death_frame: 0,
            owner: 1,
            gameplay: death_gameplay,
            max_hp: 100,
        },
    );
    snapshot.gameplay.players.entry(1).or_default().gold = gold;
    let sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    (sim, content, hero, tavern)
}

// ---------------------------------------------------------------------------
// 1. 三类经验机制接线（R6.1 / R6.5）
// ---------------------------------------------------------------------------

/// 三类机制消费者落在生产结算路径：结算登记进顶层权威运行态，逐帧哈希一致。
#[test]
fn regression_xp_mechanisms_wired_in_settlement_is_frame_deterministic() {
    let (mut a, mut b) = two_hero_matches();
    let creeps = a.creep_camps().creeps.keys().copied().collect::<Vec<_>>();
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut a, 1, 1, creep.value(), index as u32 + 1);
        attack(&mut b, 1, 1, creep.value(), index as u32 + 1);
    }
    assert_interleaved_hashes(&mut a, &mut b, 8);

    let melee = a.melee_mechanisms_state();
    assert_eq!(
        melee.xp_settlements.len(),
        creeps.len(),
        "reason=one_mechanism_record_per_settled_creep"
    );
    for record in melee.xp_settlements.values() {
        assert_eq!(
            record.building_switch, MECHANISM_BUILDING_XP_SWITCH,
            "reason=building_switch_consumed_in_settlement"
        );
        assert!(
            record.summon_factor_percent > 0 && record.summon_factor_percent <= 100,
            "reason=summon_factor_consumed_in_settlement"
        );
        assert!(
            !record.tier_factors.is_empty(),
            "reason=hero_level_tier_lookup_consumed_in_settlement"
        );
        assert!(
            record.adjusted_xp_total > 0,
            "reason=i128_adjusted_pool_is_positive"
        );
    }
    assert!(
        !melee.hero_tiers.is_empty(),
        "reason=threshold_tier_derived_in_settlement"
    );
    assert!(
        !melee.damage_outcomes.is_empty(),
        "reason=damage_matrix_consumed_in_attack_settlement"
    );
}

// ---------------------------------------------------------------------------
// 2. 买活半血无蓝（R6.3 / R9.3）
// ---------------------------------------------------------------------------

/// 酒馆买活半血无蓝入场，扣费精确，逐帧哈希与只读状态一致。
#[test]
fn regression_buyback_half_hp_no_mana_is_frame_deterministic() {
    let (mut sim, _content, hero, tavern) = dead_hero_match(1000);
    let before_gold = sim
        .capture_snapshot()
        .gameplay
        .players
        .get(&1)
        .unwrap()
        .gold;
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::BuybackHero { hero, tavern },
    ))
    .unwrap();
    sim.step();
    let snapshot = sim.capture_snapshot();
    let revived = snapshot
        .positions
        .iter()
        .find(|p| p.entity == hero.value())
        .unwrap();
    assert!(revived.alive, "reason=buyback_hero_is_alive");
    assert_eq!(revived.hp, 50, "reason=buyback_enters_half_hp");
    assert_eq!(revived.gameplay.mana, 0, "reason=buyback_enters_no_mana");
    let melee = sim.melee_mechanisms_state();
    let record = melee.buybacks.get(&hero).unwrap();
    assert!(
        snapshot.gameplay.players.get(&1).unwrap().gold < before_gold,
        "reason=buyback_deducts_gold"
    );
    assert_eq!(
        record.hp_percent, 50,
        "reason=buyback_half_hp_param_recorded"
    );
    // 快照往返逐字节一致（新增字段进 v10 快照）。
    let mut restored = RtsMatch::restore_snapshot(
        snapshot.clone(),
        StaticWorld::default(),
        snapshot.content_hash,
    )
    .unwrap();
    assert_eq!(
        restored.capture_snapshot(),
        snapshot,
        "reason=snapshot_roundtrip_byte_equal"
    );
    for _ in 0..4 {
        sim.step();
        restored.step();
        assert_eq!(
            sim.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=restored_matches_baseline"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. 经验阈值分档（R6.1）
// ---------------------------------------------------------------------------

/// 阈值分档运行态按入账后累计经验派生，逐帧哈希一致。
#[test]
fn regression_xp_threshold_tiers_is_frame_deterministic() {
    let (mut a, mut b) = two_hero_matches();
    let creeps = a.creep_camps().creeps.keys().copied().collect::<Vec<_>>();
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut a, 1, 1, creep.value(), index as u32 + 1);
        attack(&mut b, 1, 1, creep.value(), index as u32 + 1);
    }
    assert_interleaved_hashes(&mut a, &mut b, 6);

    let melee = a.melee_mechanisms_state();
    let snapshot = a.capture_snapshot();
    for (hero, tier) in &melee.hero_tiers {
        let xp = snapshot
            .positions
            .iter()
            .find(|p| p.entity == hero.value())
            .and_then(|p| p.gameplay.hero.map(|h| h.xp))
            .unwrap_or(0);
        assert_eq!(
            *tier,
            w3_core::synthetic_level_tier(xp),
            "reason=tier_is_determined_by_frozen_threshold"
        );
        assert!(*tier >= 1, "reason=tier_is_non_negative");
    }
    // 英雄自身也被登记（集合内均分入账不退化）。
    assert!(
        melee.hero_tiers.contains_key(&EntityId::new(1).unwrap()),
        "reason=hero_present_in_threshold_tier_registry"
    );
}

// ---------------------------------------------------------------------------
// 4. 克制矩阵（R5.3）
// ---------------------------------------------------------------------------

/// 二维克制矩阵查表产出按稳定攻击对登记、逐帧哈希一致。
#[test]
fn regression_damage_matrix_lookup_is_frame_deterministic() {
    let (mut a, mut b) = two_hero_matches();
    let creeps = a.creep_camps().creeps.keys().copied().collect::<Vec<_>>();
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut a, 1, 1, creep.value(), index as u32 + 1);
        attack(&mut b, 1, 1, creep.value(), index as u32 + 1);
    }
    assert_interleaved_hashes(&mut a, &mut b, 6);

    let melee = a.melee_mechanisms_state();
    assert!(
        !melee.damage_outcomes.is_empty(),
        "reason=damage_outcomes_registered_for_attack_pairs"
    );
    // 不同攻击对可由二维查表复现（键按稳定 EntityId 对）。
    let distinct_pairs: BTreeSet<(u64, u64)> = melee
        .damage_outcomes
        .keys()
        .map(|(attacker, target)| (attacker.value(), target.value()))
        .collect();
    assert_eq!(
        distinct_pairs.len(),
        melee.damage_outcomes.len(),
        "reason=stable_attack_pair_keys"
    );
    for (&(attacker, target), &outcome) in &melee.damage_outcomes {
        let ratio = w3_core::synthetic_damage_ratio(
            (attacker.value() % w3_core::ATTACK_TYPE_COUNT as u64) as usize,
            (target.value() % w3_core::ARMOR_TYPE_COUNT as u64) as usize,
        );
        assert_eq!(
            outcome,
            w3_core::scale_damage_by_ratio(100, ratio),
            "reason=outcome_reproduced_by_two_dimensional_lookup"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. 回购交互距离（R7.4 / R9.3）
// ---------------------------------------------------------------------------

/// 固定回购比例 + 交互距离：距离内成交、距离外拒单不改资源，逐帧哈希一致。
#[test]
fn regression_repurchase_distance_gate_is_frame_deterministic() {
    let (base, content) = shop::base_match(0, true);
    // 距离内：英雄出生位置与商店冻结坐标重合（SHOP_X / SHOP_Y）。
    let mut inside = shop::with_state(&base, content, |snapshot| {
        shop::inject_bag(snapshot, shop::hero(), &[shop::ITEM_CONSUMABLE]);
    });
    shop::submit(
        &mut inside,
        1,
        shop::PLAYER,
        1,
        GameplayCommandKind::SellItem {
            hero: shop::hero(),
            instance: shop::bag_instance(0),
            shop: shop::SHOP,
        },
    );
    shop::step_n(&mut inside, 1);
    assert_eq!(
        shop::gold(&inside),
        shop::SELL_CONSUMABLE,
        "reason=inside_radius_credits_exact_buyback_price"
    );
    assert!(
        inside
            .melee_mechanisms_state()
            .repurchases
            .contains_key(&shop::bag_instance(0)),
        "reason=inside_radius_records_repurchase"
    );

    // 距离外：把英雄移到交互半径之外，两个同脚本对局均提交同一 SellItem，
    // 交错逐帧哈希一致（拒单确定性）；拒单不改资源 / 背包 / 实例。
    let build_outside = || {
        shop::with_state(&base, content, |snapshot| {
            shop::inject_bag(snapshot, shop::hero(), &[shop::ITEM_CONSUMABLE]);
            for record in snapshot.positions.iter_mut() {
                if record.entity == shop::HERO_ID {
                    record.x = shop::SHOP_X + 100 * SCALE;
                    record.y = shop::SHOP_Y + 100 * SCALE;
                }
            }
        })
    };
    let mut outside = build_outside();
    let mut outside_b = build_outside();
    for target in [&mut outside, &mut outside_b] {
        shop::submit(
            target,
            1,
            shop::PLAYER,
            1,
            GameplayCommandKind::SellItem {
                hero: shop::hero(),
                instance: shop::bag_instance(0),
                shop: shop::SHOP,
            },
        );
    }
    assert_interleaved_hashes(&mut outside, &mut outside_b, 2);
    assert_eq!(
        shop::gold(&outside),
        0,
        "reason=outside_radius_no_gold_change"
    );
    assert!(
        !outside
            .melee_mechanisms_state()
            .repurchases
            .contains_key(&shop::bag_instance(0)),
        "reason=outside_radius_no_repurchase_record"
    );
    let bag = outside
        .item_shop_state()
        .bags
        .get(&shop::hero())
        .copied()
        .unwrap();
    assert!(
        bag.slots.contains(&Some(shop::bag_instance(0))),
        "reason=outside_radius_instance_kept_in_bag"
    );
}

// ---------------------------------------------------------------------------
// 6. leash 三级归巢（R8.5）
// ---------------------------------------------------------------------------

/// 归巢判据由冻结半径 / 阈值驱动，回 home 后恢复 IDLE，计数进运行态。
#[test]
fn regression_leash_three_phase_return_is_frame_deterministic() {
    let (sim, content) = seeded_match();
    let creep = creep_common::creep_ids()[0];
    // 经真实快照往返把野怪置 RETURNING（生产路径注入，home 距离 0）；
    // 同时把玩家单位移到 `acquisitionRange` 之外，保证归巢后不会重新索敌
    // （与 `creep_camps.rs::leash_match` 同构，避免 IDLE 又被切回 ENGAGED）。
    let mut snapshot = sim.capture_snapshot();
    if let Some(state) = snapshot.creep_camps.creeps.get_mut(&creep) {
        state.phase = LeashPhase::Returning;
        state.target = None;
    }
    for record in snapshot.positions.iter_mut() {
        if record.entity == 1 {
            record.x = creep_common::HOME_X + 10 * SCALE;
            record.y = creep_common::HOME_Y;
        }
    }
    let mut a =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();
    let mut b = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    assert_interleaved_hashes(&mut a, &mut b, 4);

    let melee = a.melee_mechanisms_state();
    assert_eq!(
        melee.leash_returns.get(&creep),
        Some(&1),
        "reason=leash_return_counted_once"
    );
    assert_eq!(
        a.creep_camps().creeps[&creep].phase,
        LeashPhase::Idle,
        "reason=idle_after_return"
    );
    // 快照往返逐字节一致。
    let snap = a.capture_snapshot();
    let restored =
        RtsMatch::restore_snapshot(snap.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot(),
        snap,
        "reason=leash_snapshot_roundtrip_byte_equal"
    );
}

// ---------------------------------------------------------------------------
// 7. v10 快照往返（ADR 0002 §7）
// ---------------------------------------------------------------------------

/// v10 快照头部完整、恢复后逐帧哈希与销毁前一致、无残留。
#[test]
fn regression_v10_snapshot_roundtrip_is_frame_deterministic() {
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    assert_eq!(SIM_VERSION, 10, "reason=sim_version_10");
    let (a, content) = seeded_match();
    let mut a = with_hero(a, content);
    let creeps = a.creep_camps().creeps.keys().copied().collect::<Vec<_>>();
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut a, 1, 1, creep.value(), index as u32 + 1);
    }
    a.step();
    let snapshot = a.capture_snapshot();
    assert_eq!(snapshot.version, 10, "reason=snapshot_header_v10");
    assert_eq!(snapshot.sim_version, 10, "reason=sim_header_v10");
    assert!(
        !snapshot.melee_mechanisms.xp_settlements.is_empty(),
        "reason=v10_snapshot_carries_mechanism_state"
    );
    let mut restored =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot(),
        snapshot,
        "reason=snapshot_roundtrip_byte_equal"
    );
    assert_interleaved_hashes(&mut a, &mut restored, 6);
}

// ---------------------------------------------------------------------------
// 8. 旧 v9 快照 / 回放明确拒绝（tasks §3.4 / spec Req 7）
// ---------------------------------------------------------------------------

/// 注入被拒的旧版本 9 头部：UnsupportedVersion / SimVersionMismatch 明确拒绝，
/// 不改状态、不静默降级、不提供跨版本迁移。
#[test]
fn regression_old_v9_snapshot_and_replay_are_explicitly_rejected() {
    let (sim, content) = seeded_match();
    let baseline_hash = sim.compute_state_hash();
    let snapshot = sim.capture_snapshot();

    // version = 9 是被拒的旧版本（历史注入值，MUST NOT 当作当前版本）。
    let mut old_version = snapshot.clone();
    old_version.version = 9;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_version, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 9 })
    ));
    let mut old_sim = snapshot;
    old_sim.sim_version = 9;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 9,
            expected: 10
        })
    ));
    // 拒单 MUST NOT 改写原对局权威状态。
    assert_eq!(
        sim.compute_state_hash(),
        baseline_hash,
        "reason=rejection_does_not_mutate_state"
    );
    // 当前 v10 快照自身合法（无静默拒绝、无迁移歧义）；改坏验红反例见
    // `reverse_validation_p18_mechanisms.rs` 的 version gate。
    let current = sim.capture_snapshot();
    assert!(
        RtsMatch::restore_snapshot(current, StaticWorld::default(), content).is_ok(),
        "reason=v10_snapshot_valid"
    );
}
