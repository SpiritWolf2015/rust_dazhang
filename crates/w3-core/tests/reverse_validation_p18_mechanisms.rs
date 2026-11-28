//! P18 apply 第 2 批：全量 Melee 机制反向验证（改坏验红 / `CODEBUDDY.md §5` 形态① / 用户 Q3）。
//!
//! 形态①：**测试内构造缺陷态 + 元断言防退化**。每一类缺陷态都有一对断言：
//! 正式实现在同一不变量下**必通过**、退化实现**必失败**，并用
//! `assert_ne!(degenerate, formal)` 作元断言，防止退化实现悄悄等价于正式
//! 实现（那样「缺陷态必失败」会退化成恒真、`assert!(!...)` 变成摆设）。
//!
//! 八类缺陷态（用户 Q3 / tasks §5.2）：
//! 布尔开关恒真 / 折减因子置 1 / 分档表单档 / 去掉复活上限 /
//! 矩阵退化为单值 / leash 无限追击 / 漏写哈希或快照字段 / 去掉旧 v9 拒绝。
//! 其中「漏写哈希字段」与「去掉旧 v9 拒绝」连到真实 `MeleeMechanismsState::hash`
//! 与 `RtsMatch::restore_snapshot` 公开 API（退化态为同路径的测试内复刻），
//! 证据链连到生产，不冒充已完成。
//!
//! 全部数值为编译期合成原创整数 `[待核]`，零文件 IO、零资产、无 `f64`
//! （ADR 0001 / 0002 / 0004）。p17 R2 生产结算 A-B（形态②）为单独一次
//! 「改生产 `xp_candidates` → 正向回归变红 → 恢复」的实做，命令输出见
//! 交付报告（本文件不冒充该 A-B）。

use w3_core::{
    EntityId, MECHANISM_BUILDING_XP_SWITCH, MECHANISM_HERO_LEVEL_XP_TIERS,
    MECHANISM_SUMMON_XP_FACTOR, MeleeMechanismsState, SIM_VERSION, SNAPSHOT_VERSION,
    SYNTHETIC_BUYBACK_CONSTANTS, SYNTHETIC_DAMAGE_MATRIX, SnapshotError, StateHasher, StaticWorld,
    synthetic_damage_ratio, synthetic_hero_level_xp_factor,
};

mod creep_common;
mod item_shop_common;

use creep_common::{no_leash_defs, seeded_match};

/// 触发复活绝对上限缺陷态的高等级 `[待核]`。
const HIGH_LEVEL: u32 = 1000;
/// 触发 leash 无限追击缺陷态的越界距离（Q47.16 raw，`[待核]`）：
/// 严格大于冻结 `leash_radius_raw`（12 * SCALE），MUST 判定归巢。
const CHASE_DISTANCE_RAW: i64 = 20 * (1 << 16);
/// 触发「折减因子置 1」/「开关恒真」缺陷态的合成经验池 `[待核]`。
const XP_BASE: i64 = 240;

// ---------------------------------------------------------------------------
// 缺陷态①：布尔开关恒真（建筑击杀永远授予经验）
// ---------------------------------------------------------------------------

/// 正式实现：建筑击杀布尔开关门控经验池，开关为 0 且击杀者是建筑时归零。
fn formal_building_gate(switch: u8, killer_is_building: bool, base: i64) -> i64 {
    if switch == 0 && killer_is_building {
        0
    } else {
        base
    }
}

/// 缺陷态：布尔开关恒真（忽略开关，建筑击杀照常授予经验）。
fn degenerate_building_gate_always_on(_switch: u8, _killer_is_building: bool, base: i64) -> i64 {
    base
}

/// 不变量：开关为 0 且建筑击杀 MUST 归零（R6.5 / 编译期合成占位）。
fn building_gate_is_exact(gated: i64, base: i64) -> bool {
    gated == 0 && base > 0
}

#[test]
fn reverse_validation_building_switch_formal_passes_the_assertion() {
    // 合成开关 0（生产常量 MECHANISM_BUILDING_XP_SWITCH 当前为 1，此处注入 0
    // 构造可分辨的门控场景；断言针对门控逻辑本身，不改生产常量）。
    let formal = formal_building_gate(0, true, XP_BASE);
    assert!(
        building_gate_is_exact(formal, XP_BASE),
        "reason=formal_building_switch_blocks_building_kill_xp"
    );
    assert_eq!(
        MECHANISM_BUILDING_XP_SWITCH, 1,
        "reason=production_switch_is_one_by_design"
    );
}

#[test]
fn reverse_validation_building_switch_degenerate_fails_the_same_assertion() {
    let formal = formal_building_gate(0, true, XP_BASE);
    let degenerate = degenerate_building_gate_always_on(0, true, XP_BASE);
    assert!(
        building_gate_is_exact(formal, XP_BASE),
        "reason=formal_still_passes"
    );
    assert!(
        !building_gate_is_exact(degenerate, XP_BASE),
        "reason=degenerate_always_on_must_fail"
    );
    // 元断言：退化实现 MUST 与正式实现不同，防「缺陷态必失败」退化成恒真。
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态②：折减因子置 1（召唤物击杀不折减）
// ---------------------------------------------------------------------------

/// 正式实现：召唤物击杀经验池按合成折减因子缩放（`i128` 中间量）。
fn formal_summon_scale(base: i64, factor_percent: i64) -> i64 {
    (i128::from(base.max(0)) * i128::from(factor_percent.max(0)) / 100)
        .clamp(0, i128::from(i64::MAX)) as i64
}

/// 缺陷态：折减因子置 1（100%，不折减）。
fn degenerate_summon_scale_no_reduction(base: i64) -> i64 {
    base
}

/// 不变量：折减后经验池等于合成折减因子的整数缩放结果，且严格小于原值。
fn summon_scale_is_exact(scaled: i64, base: i64, factor_percent: i64) -> bool {
    let expected = formal_summon_scale(base, factor_percent);
    scaled == expected && scaled < base && factor_percent > 0 && factor_percent < 100
}

#[test]
fn reverse_validation_summon_factor_formal_passes_the_assertion() {
    let formal = formal_summon_scale(XP_BASE, MECHANISM_SUMMON_XP_FACTOR);
    assert!(
        summon_scale_is_exact(formal, XP_BASE, MECHANISM_SUMMON_XP_FACTOR),
        "reason=formal_summon_factor_reduces_xp"
    );
}

#[test]
fn reverse_validation_summon_factor_degenerate_fails_the_same_assertion() {
    let formal = formal_summon_scale(XP_BASE, MECHANISM_SUMMON_XP_FACTOR);
    let degenerate = degenerate_summon_scale_no_reduction(XP_BASE);
    assert!(
        summon_scale_is_exact(formal, XP_BASE, MECHANISM_SUMMON_XP_FACTOR),
        "reason=formal_still_passes"
    );
    assert!(
        !summon_scale_is_exact(degenerate, XP_BASE, MECHANISM_SUMMON_XP_FACTOR),
        "reason=degenerate_factor_one_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态③：分档表单档（等级经验因子不递减）
// ---------------------------------------------------------------------------

/// 正式实现：按英雄等级查分档递减经验因子（稳定 EntityId 顺序）。
fn formal_tier_factors(levels: &[u32]) -> Vec<i64> {
    levels
        .iter()
        .map(|&level| synthetic_hero_level_xp_factor(level))
        .collect()
}

/// 缺陷态：分档表退化为单档（全部返回首档）。
fn degenerate_tier_factors_single_tier(levels: &[u32]) -> Vec<i64> {
    let first = MECHANISM_HERO_LEVEL_XP_TIERS[0];
    levels.iter().map(|_| first).collect()
}

/// 不变量：分档因子严格递减、与冻结表一致、非全等（R6.1）。
fn tier_factors_are_exact(factors: &[i64]) -> bool {
    factors.len() >= 2 && factors.windows(2).all(|pair| pair[0] > pair[1])
}

#[test]
fn reverse_validation_tier_table_formal_passes_the_assertion() {
    let formal = formal_tier_factors(&[1, 2, 3, 4]);
    assert!(
        tier_factors_are_exact(&formal),
        "reason=formal_tier_table_strictly_decreasing"
    );
}

#[test]
fn reverse_validation_tier_table_degenerate_fails_the_same_assertion() {
    let formal = formal_tier_factors(&[1, 2, 3, 4]);
    let degenerate = degenerate_tier_factors_single_tier(&[1, 2, 3, 4]);
    assert!(
        tier_factors_are_exact(&formal),
        "reason=formal_still_passes"
    );
    assert!(
        !tier_factors_are_exact(&degenerate),
        "reason=degenerate_single_tier_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态④：去掉复活绝对上限（费用线性增长不截断）
// ---------------------------------------------------------------------------

/// 正式实现：「基数 + 等级系数 → 绝对上限」`i128` 整数截断。
fn formal_buyback_cost(level: u32) -> (i64, u32) {
    let c = SYNTHETIC_BUYBACK_CONSTANTS;
    let step = i128::from(level.saturating_sub(1));
    let gold = (i128::from(c.gold_base) + i128::from(c.gold_per_level) * step)
        .min(i128::from(c.gold_cap))
        .max(0) as i64;
    let frames = (i128::from(c.frames_base) + i128::from(c.frames_per_level) * step)
        .min(i128::from(c.frames_cap))
        .max(1) as u32;
    (gold, frames)
}

/// 缺陷态：去掉绝对上限（线性增长，不 clamp）。
fn degenerate_buyback_cost_no_cap(level: u32) -> (i64, u32) {
    let c = SYNTHETIC_BUYBACK_CONSTANTS;
    let step = i128::from(level.saturating_sub(1));
    let gold = i64::try_from(i128::from(c.gold_base) + i128::from(c.gold_per_level) * step)
        .unwrap_or(i64::MAX);
    let frames = u32::try_from(i128::from(c.frames_base) + i128::from(c.frames_per_level) * step)
        .unwrap_or(u32::MAX);
    (gold, frames)
}

/// 不变量：费用 / 时间在绝对上限内、无负值、与正式 `i128` 公式逐项一致。
fn buyback_cost_is_exact(cost: (i64, u32), level: u32) -> bool {
    let c = SYNTHETIC_BUYBACK_CONSTANTS;
    cost == formal_buyback_cost(level)
        && cost.0 <= c.gold_cap
        && cost.0 >= 0
        && cost.1 <= c.frames_cap
        && cost.1 >= 1
}

#[test]
fn reverse_validation_buyback_cap_formal_passes_the_assertion() {
    let formal = formal_buyback_cost(HIGH_LEVEL);
    assert!(
        buyback_cost_is_exact(formal, HIGH_LEVEL),
        "reason=formal_buyback_cost_clamps_at_cap"
    );
}

#[test]
fn reverse_validation_buyback_cap_degenerate_fails_the_same_assertion() {
    let formal = formal_buyback_cost(HIGH_LEVEL);
    let degenerate = degenerate_buyback_cost_no_cap(HIGH_LEVEL);
    assert!(
        buyback_cost_is_exact(formal, HIGH_LEVEL),
        "reason=formal_still_passes"
    );
    assert!(
        !buyback_cost_is_exact(degenerate, HIGH_LEVEL),
        "reason=degenerate_no_cap_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑤：矩阵退化为单值（所有攻击 / 护甲组合相同倍率）
// ---------------------------------------------------------------------------

/// 正式实现：二维克制矩阵查表（不同组合可分辨）。
fn formal_matrix_ratios() -> Vec<i64> {
    (0..w3_core::ATTACK_TYPE_COUNT)
        .flat_map(|a| (0..w3_core::ARMOR_TYPE_COUNT).map(move |b| synthetic_damage_ratio(a, b)))
        .collect()
}

/// 缺陷态：矩阵退化为单值（全部 100%）。
fn degenerate_matrix_single_value() -> Vec<i64> {
    let n = w3_core::ATTACK_TYPE_COUNT * w3_core::ARMOR_TYPE_COUNT;
    vec![100; n]
}

/// 不变量：矩阵至少两个不同倍率（二维查表有鉴别力），与冻结表一致。
fn matrix_is_exact(ratios: &[i64]) -> bool {
    let distinct: std::collections::BTreeSet<i64> = ratios.iter().copied().collect();
    distinct.len() >= 2 && ratios == formal_matrix_ratios()
}

#[test]
fn reverse_validation_damage_matrix_formal_passes_the_assertion() {
    let formal = formal_matrix_ratios();
    assert!(
        matrix_is_exact(&formal),
        "reason=formal_matrix_has_distinct_ratios"
    );
    assert_eq!(
        SYNTHETIC_DAMAGE_MATRIX.len(),
        w3_core::ATTACK_TYPE_COUNT,
        "reason=frozen_matrix_rows"
    );
}

#[test]
fn reverse_validation_damage_matrix_degenerate_fails_the_same_assertion() {
    let formal = formal_matrix_ratios();
    let degenerate = degenerate_matrix_single_value();
    assert!(matrix_is_exact(&formal), "reason=formal_still_passes");
    assert!(
        !matrix_is_exact(&degenerate),
        "reason=degenerate_single_value_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑥：leash 无限追击（越过归巢半径不切 RETURNING）
// ---------------------------------------------------------------------------

/// 正式实现：home 距离越过冻结 leash 半径即切 RETURNING 归巢（R8.5）。
fn formal_leash_should_return(home_distance_raw: i64, leash_radius_raw: i64) -> bool {
    home_distance_raw > leash_radius_raw && leash_radius_raw > 0
}

/// 缺陷态：leash 半径拉到极大（无限追击，永不归巢）。
fn degenerate_leash_never_returns(home_distance_raw: i64) -> bool {
    let infinite_radius = 1_000_000 * (1 << 16);
    home_distance_raw > infinite_radius
}

/// 不变量：越过归巢半径 MUST 归巢，且冻结半径 MUST 有限（MUST NOT 无限追击）。
fn leash_is_exact(should_return: bool, leash_radius_raw: i64) -> bool {
    should_return && leash_radius_raw > 0 && leash_radius_raw < CHASE_DISTANCE_RAW * 4
}

#[test]
fn reverse_validation_leash_formal_passes_the_assertion() {
    let radius = w3_core::SYNTHETIC_CREEP_CONSTANTS.leash_radius_raw;
    let formal = formal_leash_should_return(CHASE_DISTANCE_RAW, radius);
    assert!(
        leash_is_exact(formal, radius),
        "reason=formal_leash_returns_inside_finite_radius"
    );
    // 退化定义经同一生产构造路径可得到无限半径（数据驱动，非手搓结论）。
    let bad_radius = no_leash_defs().creep_constants().leash_radius_raw;
    assert!(
        bad_radius > radius,
        "reason=no_leash_defs_has_larger_radius"
    );
}

#[test]
fn reverse_validation_leash_degenerate_fails_the_same_assertion() {
    let radius = w3_core::SYNTHETIC_CREEP_CONSTANTS.leash_radius_raw;
    let formal = formal_leash_should_return(CHASE_DISTANCE_RAW, radius);
    let degenerate = degenerate_leash_never_returns(CHASE_DISTANCE_RAW);
    assert!(leash_is_exact(formal, radius), "reason=formal_still_passes");
    assert!(
        !leash_is_exact(degenerate, radius) && !degenerate,
        "reason=degenerate_infinite_chase_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑦：漏写哈希或快照字段（连真实 MeleeMechanismsState::hash）
// ---------------------------------------------------------------------------

/// 构造两份仅 `leash_returns` 不同的权威运行态（经公开字段注入）。
fn melee_pair_with_distinct_leash() -> (MeleeMechanismsState, MeleeMechanismsState) {
    let base = MeleeMechanismsState::default();
    let mut variant = base.clone();
    let creep = EntityId::new(2).unwrap();
    variant.leash_returns.insert(creep, 1);
    (base, variant)
}

/// 正式实现：真实 `MeleeMechanismsState::hash`（六字段全编码）。
fn formal_melee_hash(state: &MeleeMechanismsState) -> u64 {
    let mut hasher = StateHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

/// 缺陷态：测试内复刻哈希但**漏写 `leash_returns` 段**（模拟哈希 / 快照字段漏接）。
fn degenerate_melee_hash_omit_leash(state: &MeleeMechanismsState) -> u64 {
    let mut hasher = StateHasher::new();
    // 仅复刻前五字段的稳定编码（与前五段逐字一致），第六段故意漏写。
    hasher.write_u64(state.xp_settlements.len() as u64);
    for (&creep, record) in &state.xp_settlements {
        hasher.write_u64(creep.value());
        hasher.write_u8(record.building_switch);
        hasher.write_i64(record.summon_factor_percent);
        hasher.write_u64(record.tier_factors.len() as u64);
        for (&hero, &factor) in &record.tier_factors {
            hasher.write_u64(hero.value());
            hasher.write_i64(factor);
        }
        hasher.write_i64(record.adjusted_xp_total);
    }
    hasher.write_u64(state.hero_tiers.len() as u64);
    for (&hero, &tier) in &state.hero_tiers {
        hasher.write_u64(hero.value());
        hasher.write_u32(tier);
    }
    hasher.write_u64(state.damage_outcomes.len() as u64);
    for (&(attacker, target), &outcome) in &state.damage_outcomes {
        hasher.write_u64(attacker.value());
        hasher.write_u64(target.value());
        hasher.write_i64(outcome);
    }
    hasher.write_u64(state.buybacks.len() as u64);
    for (&hero, record) in &state.buybacks {
        hasher.write_u64(hero.value());
        hasher.write_i64(record.hp_percent);
        hasher.write_i64(record.mana_percent);
        hasher.write_i64(record.gold);
        hasher.write_i64(record.lumber);
        hasher.write_u32(record.frames);
    }
    hasher.write_u64(state.repurchases.len() as u64);
    for (&instance, record) in &state.repurchases {
        hasher.write_u64(instance.value());
        hasher.write_i64(record.price);
        hasher.write_i64(record.ratio_percent);
    }
    hasher.finish()
}

/// 不变量：两局仅该字段不同时，状态哈希 MUST 不同、first_difference 报告该字段。
fn hash_reports_field(hash_base: u64, hash_variant: u64, field: Option<&'static str>) -> bool {
    hash_base != hash_variant && field == Some("melee_leash_returns")
}

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let (base, variant) = melee_pair_with_distinct_leash();
    let h_base = formal_melee_hash(&base);
    let h_variant = formal_melee_hash(&variant);
    let field = base.first_difference(&variant);
    assert!(
        hash_reports_field(h_base, h_variant, field),
        "reason=formal_hash_distinguishes_leash_field"
    );
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let (base, variant) = melee_pair_with_distinct_leash();
    let h_base = degenerate_melee_hash_omit_leash(&base);
    let h_variant = degenerate_melee_hash_omit_leash(&variant);
    let field = base.first_difference(&variant);
    assert!(
        !hash_reports_field(h_base, h_variant, field),
        "reason=degenerate_hash_collides_when_field_omitted"
    );
    // 元断言：退化哈希 MUST 与正式哈希可分辨（正式态确实编码了第六段）。
    assert_ne!(
        formal_melee_hash(&variant),
        degenerate_melee_hash_omit_leash(&variant),
        "reason=degenerate_hash_must_differ_from_formal"
    );
    assert_eq!(
        h_base, h_variant,
        "reason=omitted_field_makes_both_states_hash_equal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑧：去掉旧 v9 拒绝（连真实 RtsMatch::restore_snapshot）
// ---------------------------------------------------------------------------

/// 版本闸门不变量：`gate_enabled = true` 为正式实现（明确拒绝 v9）；
/// `gate_enabled = false` 为退化实现（静默把 v9 头部改写为当前 v10 后恢复）。
/// 两者 MUST 经同一个断言辅助函数，退化态同一断言必失败。
fn version_gate_rejects_v9(sim: &w3_core::RtsMatch, content: u64, gate_enabled: bool) -> bool {
    let mut old = sim.capture_snapshot();
    old.version = 9; // 注入被拒的旧版本 9（历史值，MUST NOT 当作当前版本）。
    if !gate_enabled {
        // 缺陷态：漏掉版本校验，静默升级头部（非旁路模拟，仍走 restore_snapshot）。
        old.version = SNAPSHOT_VERSION;
    }
    let version_rejected = matches!(
        w3_core::RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 9 })
    );
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 9;
    if !gate_enabled {
        old_sim.sim_version = SIM_VERSION;
    }
    let sim_rejected = matches!(
        w3_core::RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 9,
            expected: 10
        })
    );
    version_rejected && sim_rejected && SNAPSHOT_VERSION == 10 && SIM_VERSION == 10
}

#[test]
fn reverse_validation_old_v9_reject_formal_passes_the_assertion() {
    let (sim, content) = seeded_match();
    assert!(
        version_gate_rejects_v9(&sim, content, true),
        "reason=formal_old_v9_explicitly_rejected"
    );
}

#[test]
fn reverse_validation_old_v9_reject_degenerate_fails_the_same_assertion() {
    let (sim, content) = seeded_match();
    assert!(
        version_gate_rejects_v9(&sim, content, true),
        "reason=formal_still_rejects_v9"
    );
    assert!(
        !version_gate_rejects_v9(&sim, content, false),
        "reason=degenerate_silently_accepts_v9_fails_same_assertion"
    );
    // 元断言：退化态的恢复确实成功（非静默拒绝），证明闸门被去掉而非碰巧。
    let mut degraded = sim.capture_snapshot();
    degraded.version = 9;
    degraded.sim_version = 9;
    let mut upgraded = degraded;
    upgraded.version = SNAPSHOT_VERSION;
    upgraded.sim_version = SIM_VERSION;
    assert!(
        w3_core::RtsMatch::restore_snapshot(upgraded, StaticWorld::default(), content).is_ok(),
        "reason=degenerate_v9_restore_ok"
    );
}

// ---------------------------------------------------------------------------
// 元断言汇总：所有退化实现 MUST 与正式实现可分辨（防恒真）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_meta_assertions_guard_all_defect_states() {
    // 每类缺陷态的「正式值」与「退化值」MUST 不等，保证成对断言有鉴别力。
    let formal_building = formal_building_gate(0, true, XP_BASE);
    let degenerate_building = degenerate_building_gate_always_on(0, true, XP_BASE);
    assert_ne!(
        formal_building, degenerate_building,
        "reason=building_switch_defect_is_distinguishable"
    );

    let formal_summon = formal_summon_scale(XP_BASE, MECHANISM_SUMMON_XP_FACTOR);
    let degenerate_summon = degenerate_summon_scale_no_reduction(XP_BASE);
    assert_ne!(
        formal_summon, degenerate_summon,
        "reason=summon_factor_defect_is_distinguishable"
    );

    let levels = [1, 2, 3, 4];
    assert_ne!(
        formal_tier_factors(&levels),
        degenerate_tier_factors_single_tier(&levels),
        "reason=tier_table_defect_is_distinguishable"
    );

    assert_ne!(
        formal_buyback_cost(HIGH_LEVEL),
        degenerate_buyback_cost_no_cap(HIGH_LEVEL),
        "reason=buyback_cap_defect_is_distinguishable"
    );

    assert_ne!(
        formal_matrix_ratios(),
        degenerate_matrix_single_value(),
        "reason=matrix_defect_is_distinguishable"
    );

    let radius = w3_core::SYNTHETIC_CREEP_CONSTANTS.leash_radius_raw;
    assert_ne!(
        formal_leash_should_return(CHASE_DISTANCE_RAW, radius),
        degenerate_leash_never_returns(CHASE_DISTANCE_RAW),
        "reason=leash_defect_is_distinguishable"
    );

    let (base, variant) = melee_pair_with_distinct_leash();
    assert_ne!(
        formal_melee_hash(&base),
        formal_melee_hash(&variant),
        "reason=formal_hash_is_content_sensitive"
    );
    assert_eq!(
        degenerate_melee_hash_omit_leash(&base),
        degenerate_melee_hash_omit_leash(&variant),
        "reason=degenerate_hash_collides"
    );

    let (sim, content) = seeded_match();
    assert_ne!(
        version_gate_rejects_v9(&sim, content, true),
        version_gate_rejects_v9(&sim, content, false),
        "reason=version_gate_defect_is_distinguishable"
    );
}
