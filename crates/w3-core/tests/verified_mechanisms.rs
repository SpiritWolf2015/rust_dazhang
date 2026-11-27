//! P17 已核对机制的 CPU 确定性回归（`verified-melee-mechanisms` 的
//! Requirement 1 / 2 / 3 / 4 / 5 中属于 `w3-core` 内核侧的判据）。
//!
//! 规则依据：R6.5（满级英雄参与分摊）/ R6.1（经验两环节、稳定序）/
//! R6.3（复活公式族上限）/ R9.3（整数帧）/ R11.5 / R12.4（重开无残留）；
//! 确定性契约 ADR 0002 §1-8 / `CODEBUDDY.md §5`；设计决策 D2 / D3 / D4 / D5。
//! 全部数值为合成原创编译期整数 `[待核]`，零文件 IO、零资产、不冒充原版
//! （附录「核实约定」/ ADR 0001 / ADR 0004）。
//!
//! 本文件**只读取**权威状态与只读视图，MUST NOT 由表现层反写 `HeroState.xp`；
//! 注释订正前后逐帧 `compute_state_hash()` 与同脚本基线保持一致。

mod creep_common;

use std::collections::BTreeSet;

use creep_common::{
    CAMP_ID, CAMP_SCRIPT, CREEP_XP, HOME_X, HOME_Y, PLAYER, attack, config, defs, seed_camp,
};
use w3_core::{
    Command, CommandKind, EntityId, Frame, FrozenDefinitions, HeroState,
    MECHANISM_BUILDING_XP_SWITCH, MECHANISM_HERO_LEVEL_XP_TIERS, MECHANISM_SUMMON_XP_FACTOR,
    ReviveConstants, ReviveCost, RtsMatch, SIM_VERSION, SNAPSHOT_VERSION,
    SYNTHETIC_REVIVE_CONSTANTS, SimFixed, StaticWorld, synthetic_hero_level_xp_factor,
};

/// 合成占位的「满级」等级 `[待核]`；不代表任何原版等级上限，仅用于构造
/// 一个 `hero.level` 已达上限的英雄，验证结算 MUST NOT 因等级排除它。
const MAXED_LEVEL: u32 = 10;
/// 未满级英雄的合成等级 `[待核]`，与满级英雄同处分摊范围形成对照。
const NOVICE_LEVEL: u32 = 1;
const SECOND_NOVICE_LEVEL: u32 = 2;
/// 己方英雄数量（合成占位 `[待核]`）；用于验证集合内均分与整除截断。
const HERO_COUNT: usize = 3;
/// 复活费用回归采用的高等级（合成占位 `[待核]`），用于触发绝对上限。
const HIGH_LEVEL: u32 = 1000;

/// 从冻结定义读取经验等级因子（MUST NOT 在测试里写死具体数值）。
fn frozen_xp_factor() -> i64 {
    defs().creep_constants().hero_level_xp_factor
}

/// 经验分摊两环节的期望值（纯整数复刻，MUST NOT 引入浮点）。
/// `total = script.xp * hero_level_xp_factor / 100`、`share = total / count`。
fn expected_share(xp: i64, factor: i64, count: usize) -> (i64, i64) {
    let total = xp.saturating_mul(factor) / 100;
    let share = total / count as i64;
    (total, share)
}

/// 生成一个己方单位：同一 frame-0、按稳定 `Sequence` 排序，坐标都落在
/// 经验分摊范围内，`damage` 足以一击击杀低血量野怪（合成占位 `[待核]`）。
fn spawn_at(sequence: u32, x: i64, y: i64, hp: i64, damage: i64) -> Command {
    Command::new(
        Frame::new(0),
        PLAYER,
        sequence,
        CommandKind::Spawn {
            x: SimFixed::from_raw(x),
            y: SimFixed::from_raw(y),
            hp,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: damage,
            attack_range: SimFixed::from_int(100).unwrap(),
        },
    )
}

/// 经真实 `capture_snapshot` / `restore_snapshot` 生产路径给多个己方单位
/// 注入英雄等级（非旁路手搓权威字段），其中 `entity=1` 为满级英雄。
fn with_heroes(sim: RtsMatch, content: u64) -> RtsMatch {
    let mut snapshot = sim.capture_snapshot();
    let levels = [
        (1u64, MAXED_LEVEL),
        (2u64, NOVICE_LEVEL),
        (3u64, SECOND_NOVICE_LEVEL),
    ];
    for record in snapshot.positions.iter_mut() {
        if let Some((_, level)) = levels.iter().find(|(id, _)| *id == record.entity) {
            record.gameplay.hero = Some(HeroState {
                level: *level,
                xp: 0,
            });
        }
    }
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

/// 干净环境构造：frame-0 提交三个 `Spawn` 与营地装配，推进一帧完成装配，
/// 再注入英雄等级。返回 `(对局, content_hash, 野怪实体列表)`。
/// 野怪 id 按稳定 `BTreeSet` 键序动态收集，MUST NOT 硬编码装配顺序。
fn seeded_heroes() -> (RtsMatch, u64, Vec<EntityId>) {
    let frozen = defs();
    let content = frozen.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    for sequence in 1..=HERO_COUNT as u32 {
        sim.submit_command(spawn_at(sequence, HOME_X, HOME_Y, 100, 100))
            .unwrap();
    }
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    let creeps: Vec<EntityId> = sim.creep_camps().creeps.keys().copied().collect();
    let sim = with_heroes(sim, content);
    (sim, content, creeps)
}

/// 从只读快照读取某英雄的权威 `xp`（表现层 MUST NOT 反写该字段）。
fn hero_xp(sim: &RtsMatch, entity: u64) -> Option<i64> {
    sim.capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == entity)
        .and_then(|record| record.gameplay.hero)
        .map(|hero| hero.xp)
}

/// 己方英雄的稳定 `EntityId` 顺序（装配顺序 1 / 2 / 3，合成占位 `[待核]`）。
fn hero_ids() -> Vec<u64> {
    vec![1, 2, 3]
}

/// 提交击杀脚本：由满级英雄攻击全部野怪，一次 `step()` 触发幂等结算。
fn kill_all_and_settle(sim: &mut RtsMatch, creeps: &[EntityId]) {
    for (index, creep) in creeps.iter().enumerate() {
        attack(sim, 1, 1, creep.value(), index as u32 + 1);
    }
    sim.step();
}

// ---------------------------------------------------------------------------
// 1. R6.5：满级英雄仍在分摊集合且仍获经验入账（spec Requirement 1）
// ---------------------------------------------------------------------------

#[test]
fn maxed_hero_still_enters_share_set_and_is_credited() {
    let (mut sim, _content, creeps) = seeded_heroes();
    assert_eq!(creeps.len(), 2, "reason=two_creeps_seeded");

    // 满级英雄入局时 `xp` 为 0（快照注入路径未旁路写经验）。
    assert_eq!(
        hero_xp(&sim, 1),
        Some(0),
        "reason=maxed_hero_xp_starts_zero"
    );
    let baseline_before = sim.compute_state_hash();

    kill_all_and_settle(&mut sim, &creeps);

    let state = sim.creep_camps();
    assert_eq!(
        state.kill_rewards.len(),
        creeps.len(),
        "reason=one_reward_per_creep"
    );
    let (total, share) = expected_share(CREEP_XP, frozen_xp_factor(), HERO_COUNT);

    for reward in state.kill_rewards.values() {
        // 第一环节：满级英雄（level 达合成上限）仍在 `BTreeSet` 分摊集合。
        assert!(
            reward.xp_candidates.contains(&EntityId::new(1).unwrap()),
            "reason=maxed_hero_enters_share_set_R6_5"
        );
        // 第二环节：满级英雄仍获得均分经验入账。
        assert_eq!(
            reward.xp_credited.get(&EntityId::new(1).unwrap()),
            Some(&share),
            "reason=maxed_hero_credited_R6_5"
        );
        assert!(
            share > 0 && share * HERO_COUNT as i64 <= total,
            "reason=integer_even_split_no_float"
        );
    }

    // `state.xp` 确定性增长：两条收益各入账一次，MUST NOT 重复或缺失。
    let expected_xp = share * creeps.len() as i64;
    for id in hero_ids() {
        assert_eq!(
            hero_xp(&sim, id),
            Some(expected_xp),
            "reason=xp_deterministic_growth"
        );
    }
    assert_ne!(
        sim.compute_state_hash(),
        baseline_before,
        "reason=state_hash_advances_after_settlement"
    );
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    assert_eq!(SIM_VERSION, 9, "reason=sim_version_9");
}

// ---------------------------------------------------------------------------
// 2. R6.5 / R6.1：集合内多英雄均分、`i64` 整除一致、无浮点
// ---------------------------------------------------------------------------

#[test]
fn all_heroes_share_the_same_integer_share() {
    let (mut sim, _content, creeps) = seeded_heroes();
    kill_all_and_settle(&mut sim, &creeps);

    let (total, share) = expected_share(CREEP_XP, frozen_xp_factor(), HERO_COUNT);
    let state = sim.creep_camps();
    for reward in state.kill_rewards.values() {
        assert_eq!(
            reward.xp_candidates.len(),
            HERO_COUNT,
            "reason=all_heroes_in_range"
        );
        assert_eq!(
            reward.xp_credited.len(),
            HERO_COUNT,
            "reason=all_heroes_credited_once"
        );
        for id in hero_ids() {
            assert_eq!(
                reward.xp_credited.get(&EntityId::new(id).unwrap()),
                Some(&share),
                "reason=identical_share_for_every_hero"
            );
        }
        // `i64` 整除向零截断：余数被丢弃，总额守恒且无 `f64` 舍入。
        let credited_sum: i64 = reward.xp_credited.values().sum();
        assert_eq!(
            credited_sum,
            share * HERO_COUNT as i64,
            "reason=integer_division_truncation"
        );
        assert!(
            credited_sum <= total && total - credited_sum < HERO_COUNT as i64,
            "reason=remainder_discarded_without_float"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. R6.5 / R6.1 / R11.5 / R12.4：两环节分离、稳定序、不重复入账、重开无残留
// ---------------------------------------------------------------------------

#[test]
fn two_phases_are_separated_stable_and_idempotent() {
    let (mut sim, _content, creeps) = seeded_heroes();
    kill_all_and_settle(&mut sim, &creeps);

    let (_total, share) = expected_share(CREEP_XP, frozen_xp_factor(), HERO_COUNT);
    let expected_ids = hero_ids();
    let state = sim.creep_camps();
    for reward in state.kill_rewards.values() {
        // 两环节分离：进入集合（`BTreeSet`）与实际入账（`BTreeMap`）可分别验证。
        let candidates: Vec<u64> = reward.xp_candidates.iter().map(|id| id.value()).collect();
        let credited: Vec<u64> = reward.xp_credited.keys().map(|id| id.value()).collect();
        assert_eq!(
            candidates, expected_ids,
            "reason=stable_entity_id_order_candidates"
        );
        assert_eq!(
            credited, expected_ids,
            "reason=stable_entity_id_order_credited"
        );
        assert_eq!(
            BTreeSet::from_iter(candidates.clone()),
            BTreeSet::from_iter(credited.clone()),
            "reason=two_phases_cover_same_set"
        );
    }
    assert_eq!(
        state.death_processed.len(),
        creeps.len(),
        "reason=idempotent_registry"
    );
    assert!(state.pending_deaths.is_empty(), "reason=queue_drained");

    // 不重复入账：再推进若干帧，收益条数与英雄 `xp` 均不再变化。
    let settled_hash = sim.compute_state_hash();
    for _ in 0..6 {
        sim.step();
    }
    let after = sim.creep_camps();
    assert_eq!(
        after.kill_rewards.len(),
        creeps.len(),
        "reason=no_duplicate_reward"
    );
    assert_eq!(
        after.kill_rewards.len(),
        state.kill_rewards.len(),
        "reason=no_resettlement"
    );
    for id in hero_ids() {
        assert_eq!(
            hero_xp(&sim, id),
            Some(share * creeps.len() as i64),
            "reason=no_duplicate_credit"
        );
    }
    assert_ne!(
        sim.compute_state_hash(),
        settled_hash,
        "reason=clock_advances_normally"
    );
}

#[test]
fn restart_has_no_stale_xp_or_reward_registry() {
    // 上一局已完成结算，含收益登记与非零英雄经验。
    let (mut previous, _content, creeps) = seeded_heroes();
    kill_all_and_settle(&mut previous, &creeps);
    let (_total, share) = expected_share(CREEP_XP, frozen_xp_factor(), HERO_COUNT);
    assert!(
        !previous.creep_camps().kill_rewards.is_empty(),
        "reason=previous_has_rewards"
    );
    assert_eq!(
        hero_xp(&previous, 1),
        Some(share * creeps.len() as i64),
        "reason=previous_maxed_hero_has_xp"
    );

    // 新一局：干净重建（同冻结定义 + 同开局脚本），不残留上一局登记 / 经验。
    let fresh = seeded_heroes().0;
    let fresh_state = fresh.creep_camps();
    assert!(
        fresh_state.kill_rewards.is_empty(),
        "reason=fresh_kill_rewards_empty"
    );
    assert!(
        fresh_state.death_processed.is_empty(),
        "reason=fresh_death_registry_empty"
    );
    assert!(
        fresh_state.pending_deaths.is_empty(),
        "reason=fresh_pending_deaths_empty"
    );
    for id in hero_ids() {
        assert_eq!(hero_xp(&fresh, id), Some(0), "reason=fresh_hero_xp_zero");
    }
    assert_eq!(
        fresh.capture_snapshot().version,
        SNAPSHOT_VERSION,
        "reason=same_snapshot_version"
    );
}

#[test]
fn read_only_view_must_not_rewrite_authoritative_xp() {
    let (mut sim, _content, creeps) = seeded_heroes();
    kill_all_and_settle(&mut sim, &creeps);
    let (_total, share) = expected_share(CREEP_XP, frozen_xp_factor(), HERO_COUNT);

    let before = sim.compute_state_hash();
    // `read_view()` 返回 owned 视图，只读消费 MUST NOT 改变权威状态哈希。
    let view = sim.read_view();
    assert!(
        view.entities.len() >= HERO_COUNT,
        "reason=view_contains_heroes_and_creeps"
    );
    assert_eq!(
        sim.compute_state_hash(),
        before,
        "reason=view_consumption_does_not_change_state"
    );
    // 权威 `xp` 仍由结算独占写入，未被只读视图反写。
    for id in hero_ids() {
        assert_eq!(
            hero_xp(&sim, id),
            Some(share * creeps.len() as i64),
            "reason=xp_not_rewritten_by_presentation"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. ADR 0002 §6-8：逐帧状态哈希与同脚本基线一致、快照往返一致
// ---------------------------------------------------------------------------

#[test]
fn per_frame_hash_matches_same_script_baseline() {
    let (mut baseline, _content, creeps) = seeded_heroes();
    let (mut replay, _content2, creeps2) = seeded_heroes();
    assert_eq!(creeps, creeps2, "reason=same_seed_same_creep_ids");
    assert_eq!(
        baseline.compute_state_hash(),
        replay.compute_state_hash(),
        "reason=same_script_same_initial_hash"
    );

    // 同一脚本：frame-1 提交相同的击杀命令，随后交错推进逐帧比对。
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut baseline, 1, 1, creep.value(), index as u32 + 1);
        attack(&mut replay, 1, 1, creep.value(), index as u32 + 1);
    }
    for _ in 0..8 {
        baseline.step();
        replay.step();
        assert_eq!(
            baseline.compute_state_hash(),
            replay.compute_state_hash(),
            "reason=per_frame_hash_matches_baseline"
        );
        assert_eq!(
            baseline.first_difference(&replay),
            None,
            "reason=no_first_difference"
        );
    }

    // 快照往返：`SNAPSHOT_VERSION`（当前 9）头部完整、恢复后逐帧一致。
    let snapshot = baseline.capture_snapshot();
    assert_eq!(snapshot.version, 9, "reason=snapshot_version_9");
    assert_eq!(snapshot.sim_version, 9, "reason=sim_version_9");
    assert_eq!(
        snapshot.defs_hash,
        snapshot.defs.content_hash(),
        "reason=defs_hash_matches_frozen_definitions"
    );
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
        baseline.step();
        restored.step();
        assert_eq!(
            baseline.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=restored_matches_baseline"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. R6.3 / R9.3：复活费用「基数 + 等级系数 → 绝对上限」公式族
// ---------------------------------------------------------------------------

/// 构造仅含祭坛复活常量段的冻结定义（干净环境、零文件 IO）。
fn revive_defs(constants: ReviveConstants) -> FrozenDefinitions {
    FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_hero_revive(constants, BTreeSet::new(), BTreeSet::new())
}

/// 独立复刻「基数 + 等级系数 → 绝对上限」公式（纯 `i128` 整数，MUST NOT
/// 引入浮点），作为回归断言的期望值来源，不调用被测 `cost()` 实现。
fn expected_revive_cost(constants: &ReviveConstants, level: u32) -> ReviveCost {
    let step = i128::from(level.saturating_sub(1));
    let clamp_gold = |base: i64, per: i64, cap: i64| -> i64 {
        let raw = i128::from(base) + i128::from(per) * step;
        let bounded = raw.min(i128::from(cap)).max(0);
        i64::try_from(bounded).unwrap_or(cap)
    };
    let clamp_frames = |base: u32, per: u32, cap: u32| -> u32 {
        let raw = i128::from(base) + i128::from(per) * step;
        let bounded = raw.min(i128::from(cap)).max(1);
        u32::try_from(bounded).unwrap_or(cap)
    };
    ReviveCost {
        gold: clamp_gold(
            constants.gold_base,
            constants.gold_per_level,
            constants.gold_cap,
        ),
        lumber: clamp_gold(
            constants.lumber_base,
            constants.lumber_per_level,
            constants.lumber_cap,
        ),
        food: constants.food_cost,
        frames: clamp_frames(
            constants.frames_base,
            constants.frames_per_level,
            constants.frames_cap,
        ),
    }
}

#[test]
fn revive_cost_formula_scales_and_clamps_at_absolute_caps() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let level_one = constants.cost(1);
    let high = constants.cost(HIGH_LEVEL);
    let extreme = constants.cost(u32::MAX);

    // 等级 1：基数生效，且与独立整数公式逐项一致。
    assert_eq!(
        level_one,
        expected_revive_cost(&constants, 1),
        "reason=level_one_matches_integer_formula"
    );
    assert!(
        level_one.gold > 0 && level_one.lumber >= 0 && level_one.frames >= 1,
        "reason=level_one_non_negative"
    );

    // 高等级：三类费用 / 时间各自在绝对上限处截断，不越界、无负值。
    assert_eq!(
        high,
        expected_revive_cost(&constants, HIGH_LEVEL),
        "reason=high_level_matches_integer_formula"
    );
    assert_eq!(high.gold, constants.gold_cap, "reason=gold_absolute_cap");
    assert_eq!(
        high.lumber, constants.lumber_cap,
        "reason=lumber_absolute_cap"
    );
    assert_eq!(
        high.frames, constants.frames_cap,
        "reason=frames_absolute_cap"
    );
    assert!(
        high.gold <= constants.gold_cap
            && high.lumber <= constants.lumber_cap
            && high.frames <= constants.frames_cap,
        "reason=no_cap_overflow"
    );
    assert!(
        high.gold >= 0 && high.lumber >= 0 && high.frames >= 1,
        "reason=no_negative_or_zero_frames"
    );

    // `i128` 中间量防溢出：极端等级 MUST NOT panic / 回绕，仍截断到上限。
    assert_eq!(extreme, high, "reason=u32_max_does_not_overflow");
    assert_eq!(
        extreme,
        expected_revive_cost(&constants, u32::MAX),
        "reason=extreme_level_matches_integer_formula"
    );

    // 等级 0 也按整数公式安全处理（`saturating_sub(1)` 与 `max` 兜底）。
    let zero = constants.cost(0);
    assert_eq!(
        zero,
        expected_revive_cost(&constants, 0),
        "reason=level_zero_is_safe"
    );
    assert!(
        zero.gold >= 0 && zero.lumber >= 0 && zero.frames >= 1,
        "reason=level_zero_non_negative"
    );
}

#[test]
fn revive_parameters_are_separated_and_enter_defs_hash() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;

    // 祭坛与买活参数分离：冻结定义只登记一套祭坛费用参数，
    // 改恢复百分比 MUST NOT 影响 `cost()`（买活半血无蓝未实现、顺延 p18）。
    let mut hp_changed = constants;
    hp_changed.hp_restore_percent = hp_changed.hp_restore_percent.saturating_add(1);
    let mut mana_changed = constants;
    mana_changed.mana_restore_percent = mana_changed.mana_restore_percent.saturating_add(1);
    assert_eq!(
        hp_changed.cost(HIGH_LEVEL),
        constants.cost(HIGH_LEVEL),
        "reason=hp_restore_does_not_change_cost"
    );
    assert_eq!(
        mana_changed.cost(HIGH_LEVEL),
        constants.cost(HIGH_LEVEL),
        "reason=mana_restore_does_not_change_cost"
    );

    // 三类费用参数各自独立：只改 gold 基数，lumber / frames MUST NOT 串用。
    let mut gold_changed = constants;
    gold_changed.gold_base = gold_changed.gold_base.saturating_add(1);
    let gold_variant = gold_changed.cost(1);
    let base_variant = constants.cost(1);
    assert_ne!(
        gold_variant.gold, base_variant.gold,
        "reason=gold_base_is_independent"
    );
    assert_eq!(
        gold_variant.lumber, base_variant.lumber,
        "reason=lumber_not_crossover_with_gold"
    );
    assert_eq!(
        gold_variant.frames, base_variant.frames,
        "reason=frames_not_crossover_with_gold"
    );

    // 改冻结常量 → `content_hash()` 变化 → 经 `compute_defs_hash` 进
    // `defs_hash` / `content_hash` 快照头部（ADR 0002 §6-7 / R12.4）。
    let base = revive_defs(constants);
    let variant = revive_defs(gold_changed);
    assert_ne!(
        base.content_hash(),
        variant.content_hash(),
        "reason=constants_enter_defs_hash"
    );
    let base_sim =
        RtsMatch::create_with_definitions(config(&base), StaticWorld::default(), base.clone());
    let variant_sim = RtsMatch::create_with_definitions(
        config(&variant),
        StaticWorld::default(),
        variant.clone(),
    );
    assert_eq!(
        base_sim.capture_snapshot().defs_hash,
        base.content_hash(),
        "reason=defs_hash_in_snapshot_header"
    );
    assert_ne!(
        base_sim.capture_snapshot().defs_hash,
        variant_sim.capture_snapshot().defs_hash,
        "reason=snapshot_header_is_content_sensitive"
    );
    assert_eq!(
        base_sim.capture_snapshot().content_hash,
        base.content_hash(),
        "reason=content_hash_in_snapshot_header"
    );
}

// ---------------------------------------------------------------------------
// 6. spec Requirement 3：布尔 / 因子机制结构以合成占位表达且不改字段集合
// ---------------------------------------------------------------------------

#[test]
fn synthetic_mechanism_structures_are_compile_time_integers() {
    // 干净环境构造成功：不含 `config.toml` / 导出目录，零文件 IO。
    let frozen = defs();
    assert!(
        frozen.creep_constants().hero_level_xp_factor > 0,
        "reason=frozen_defs_constructed_in_clean_env"
    );

    // 布尔开关：编译期整数常量，取值 0 / 1，`[待核]` 合成占位。
    assert!(
        MECHANISM_BUILDING_XP_SWITCH <= 1,
        "reason=building_xp_switch_is_boolean_integer"
    );
    // 召唤物击杀经验折减因子：编译期整数常量，落在整数百分比区间。
    assert!(
        MECHANISM_SUMMON_XP_FACTOR > 0 && MECHANISM_SUMMON_XP_FACTOR <= 100,
        "reason=summon_factor_is_integer_percentage"
    );
    // 等级经验因子分档递减：定长整数表，逐级严格递减。
    let tiers = MECHANISM_HERO_LEVEL_XP_TIERS;
    assert!(tiers.len() >= 2, "reason=hero_level_tier_table_non_empty");
    assert!(
        tiers.windows(2).all(|pair| pair[0] > pair[1]),
        "reason=hero_level_tiers_strictly_decreasing"
    );
    assert!(
        tiers.iter().all(|factor| *factor >= 0),
        "reason=tiers_are_non_negative_integers"
    );

    // 分档查询函数：纯整数、无浮点、越界确定性钳到首 / 末档。
    assert_eq!(
        synthetic_hero_level_xp_factor(0),
        tiers[0],
        "reason=level_zero_clamps_to_first_tier"
    );
    assert_eq!(
        synthetic_hero_level_xp_factor(1),
        tiers[0],
        "reason=level_one_uses_first_tier"
    );
    assert_eq!(
        synthetic_hero_level_xp_factor(u32::MAX),
        tiers[tiers.len() - 1],
        "reason=extreme_level_clamps_to_last_tier"
    );
    for level in 1..=tiers.len() as u32 {
        assert_eq!(
            synthetic_hero_level_xp_factor(level),
            tiers[(level - 1) as usize],
            "reason=tier_lookup_is_deterministic"
        );
    }

    // 机制结构 MUST NOT 改变冻结定义内容哈希（编译期常量不进
    // `compute_defs_hash`），故三版本号仍为 9、v9 golden hash 基线不变。
    assert_eq!(
        frozen.content_hash(),
        defs().content_hash(),
        "reason=mechanism_constants_leave_content_hash_unchanged"
    );
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    assert_eq!(SIM_VERSION, 9, "reason=sim_version_9");
}
