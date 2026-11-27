//! P17 已核对机制的反向验证（改坏验红 / `CODEBUDDY.md §5` 形态①）。
//!
//! 形态①：**测试内构造缺陷态 + 元断言防退化**。退化实现不是手搓假结论，
//! 而是「复刻结算逻辑的测试内辅助函数」；每一类缺陷态都有一对断言：
//! 正式实现在同一不变量下**必通过**、退化实现**必失败**，并用
//! `assert_ne!(degenerate, formal)` 作元断言，防止退化实现悄悄等价于正式
//! 实现（那样「缺陷态必失败」会退化成恒真、`assert!(!...)` 变成摆设）。
//!
//! 先例：`crates/w3-core/tests/reverse_validation_hero_revive.rs` /
//! `reverse_validation_ability_system.rs` / `reverse_validation_creep_camps.rs`。
//! 覆盖缺陷态：排除满级英雄 / 均分改全额 / 集合改不稳定顺序 / 重复入账 /
//! 去掉复活绝对上限 / 引入浮点（以整数复刻浮点舍入语义，测试内不写
//! 任何浮点类型）/ 祭坛与买活参数串用。
//!
//! 规则依据：R6.5 / R6.1 / R6.3 / R9.3 / ADR 0002 §1-8。
//! 全部数值为合成原创编译期整数 `[待核]`，零文件 IO、零资产。

use w3_core::{EntityId, ReviveConstants, ReviveCost, SYNTHETIC_REVIVE_CONSTANTS};

/// 触发经验均分缺陷态的合成总量 `[待核]`：26 / 4 向零截断为 6，
/// 而浮点式四舍五入为 7，两者 MUST 不同（元断言的可分辨性前提）。
const DEFECT_TOTAL: i64 = 26;
/// 触发绝对上限缺陷态的高等级 `[待核]`。
const HIGH_LEVEL: u32 = 1000;
/// 触发参数串用缺陷态的等级 `[待核]`。
const CROSSOVER_LEVEL: u32 = 5;

/// 四个己方英雄的稳定 `EntityId`（合成占位 `[待核]`），其中 1 号满级。
fn heroes() -> Vec<EntityId> {
    vec![
        EntityId::new(1).unwrap(),
        EntityId::new(2).unwrap(),
        EntityId::new(3).unwrap(),
        EntityId::new(4).unwrap(),
    ]
}

fn maxed_hero() -> EntityId {
    EntityId::new(1).unwrap()
}

/// 测试内复刻的结算结果数据形状：分摊集合（第一环节）+ 入账登记
/// （第二环节）+ 集合是否按稳定 `EntityId` 顺序承载。
#[derive(Clone, PartialEq, Eq, Debug)]
struct Settlement {
    candidates: Vec<EntityId>,
    credited: Vec<(EntityId, i64)>,
    stable_order: bool,
}

/// 测试内复刻的 R6.5 正式实现：范围 + 存活 + 己方英雄进 `BTreeSet` 稳定序
/// 集合（满级英雄 MUST 参与），`share = total / count` 整数均分、只入账一次。
fn formal_settlement(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut candidates = all_heroes.to_vec();
    candidates.sort_by_key(|id| id.value());
    let count = candidates.len() as i64;
    let share = if count > 0 { total / count } else { 0 };
    let credited = candidates.iter().map(|&id| (id, share)).collect();
    Settlement {
        candidates,
        credited,
        stable_order: true,
    }
}

/// 缺陷态①：排除满级英雄（把 `level >= MAXED_LEVEL` 的英雄挡在集合外）。
fn degenerate_exclude_maxed(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut formal = formal_settlement(all_heroes, total);
    formal.candidates.retain(|id| *id != maxed_hero());
    formal.credited.retain(|(id, _)| *id != maxed_hero());
    formal
}

/// 缺陷态②：均分改全额（每个英雄都拿 `total`，漏掉 `total / count`）。
fn degenerate_full_share(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut formal = formal_settlement(all_heroes, total);
    formal.credited = formal.candidates.iter().map(|&id| (id, total)).collect();
    formal
}

/// 缺陷态③：集合改不稳定顺序（反转遍历序，模拟 `HashMap` 迭代序）。
fn degenerate_unstable_order(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut formal = formal_settlement(all_heroes, total);
    formal.candidates.reverse();
    formal.credited.reverse();
    formal.stable_order = false;
    formal
}

/// 缺陷态④：重复入账（同一英雄登记两次，破坏「只结算一次」）。
fn degenerate_double_credit(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut formal = formal_settlement(all_heroes, total);
    let doubled: Vec<(EntityId, i64)> = formal
        .credited
        .iter()
        .flat_map(|&(id, xp)| [(id, xp), (id, xp)])
        .collect();
    formal.credited = doubled;
    formal
}

/// 缺陷态⑤：引入浮点（以整数复刻「浮点四舍五入」舍入语义，MUST NOT
/// 在测试或任何权威状态里写浮点类型；ADR 0002 §1）。
fn degenerate_float_rounding(all_heroes: &[EntityId], total: i64) -> Settlement {
    let mut formal = formal_settlement(all_heroes, total);
    let count = formal.candidates.len() as i64;
    // 浮点 round 语义的整数等价：`(total + count / 2) / count`。
    let rounded = if count > 0 {
        (total + count / 2) / count
    } else {
        0
    };
    formal.credited = formal.candidates.iter().map(|&id| (id, rounded)).collect();
    formal
}

/// 正式不变量：满级英雄 MUST 在集合内、稳定 `EntityId` 顺序、集合内英雄
/// 均分同一 `share`、只入账一次、两环节覆盖同一集合（ADR 0002 §3 / R6.5）。
fn settlement_is_exact(settlement: &Settlement, all_heroes: &[EntityId], total: i64) -> bool {
    let mut expected = all_heroes.to_vec();
    expected.sort_by_key(|id| id.value());
    if !settlement.stable_order || settlement.candidates != expected {
        return false;
    }
    // 满级英雄 MUST 参与分摊（2026-11-26 拍板 / §14 / §15）。
    if !settlement.candidates.contains(&maxed_hero()) {
        return false;
    }
    let count = expected.len() as i64;
    if count == 0 || settlement.credited.len() != expected.len() {
        return false;
    }
    let share = total / count;
    // 稳定序 + 每个英雄恰好一次 + 均分同一整数 `share`。
    let mut seen = Vec::new();
    for (index, (id, xp)) in settlement.credited.iter().enumerate() {
        if *xp != share || *id != expected[index] || seen.contains(id) {
            return false;
        }
        seen.push(*id);
    }
    true
}

/// 独立复刻 R6.3「基数 + 等级系数 → 绝对上限」公式（纯 `i128` 整数），
/// 作为复活不变量的期望值来源，不调用被测 `cost()` 实现。
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

/// 缺陷态⑥：去掉复活绝对上限（线性增长而不截断到 `cap`）。
fn degenerate_revive_no_cap(constants: &ReviveConstants, level: u32) -> ReviveCost {
    let step = i128::from(level.saturating_sub(1));
    let linear = |base: i64, per: i64| -> i64 {
        let raw = i128::from(base) + i128::from(per) * step;
        i64::try_from(raw.max(0)).unwrap_or(i64::MAX)
    };
    let linear_frames = |base: u32, per: u32| -> u32 {
        let raw = i128::from(base) + i128::from(per) * step;
        u32::try_from(raw.max(1)).unwrap_or(u32::MAX)
    };
    ReviveCost {
        gold: linear(constants.gold_base, constants.gold_per_level),
        lumber: linear(constants.lumber_base, constants.lumber_per_level),
        food: constants.food_cost,
        frames: linear_frames(constants.frames_base, constants.frames_per_level),
    }
}

/// 缺陷态⑦：祭坛与买活参数串用（把恢复百分比当作费用上限，跨参数组混用）。
fn degenerate_revive_param_crossover(constants: &ReviveConstants, level: u32) -> ReviveCost {
    let step = i128::from(level.saturating_sub(1));
    let crossed = |base: i64, per: i64| -> i64 {
        let raw = i128::from(base) + i128::from(per) * step;
        // 串用：`hp_restore_percent` 被误当作黄金绝对上限。
        let bounded = raw.min(i128::from(constants.hp_restore_percent)).max(0);
        i64::try_from(bounded).unwrap_or(constants.hp_restore_percent)
    };
    ReviveCost {
        gold: crossed(constants.gold_base, constants.gold_per_level),
        lumber: crossed(constants.lumber_base, constants.lumber_per_level),
        food: constants.food_cost,
        frames: expected_revive_cost(constants, level).frames,
    }
}

/// 复活正式不变量：与冻结常量的 `i128` 整数公式逐项一致，且三类费用 /
/// 时间各自在绝对上限内、无负值、帧数至少 1。
fn revive_is_exact(cost: &ReviveCost, constants: &ReviveConstants, level: u32) -> bool {
    *cost == expected_revive_cost(constants, level)
        && cost.gold <= constants.gold_cap
        && cost.lumber <= constants.lumber_cap
        && cost.frames <= constants.frames_cap
        && cost.gold >= 0
        && cost.lumber >= 0
        && cost.frames >= 1
}

// ---------------------------------------------------------------------------
// 缺陷态①：排除满级英雄
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_exclude_maxed_hero_formal_passes_the_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_settlement_keeps_maxed_hero"
    );
}

#[test]
fn reverse_validation_exclude_maxed_hero_degenerate_fails_the_same_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    let degenerate = degenerate_exclude_maxed(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_still_passes"
    );
    assert!(
        !settlement_is_exact(&degenerate, &heroes(), DEFECT_TOTAL),
        "reason=degenerate_excluding_maxed_hero_must_fail"
    );
    // 元断言：退化实现 MUST 与正式实现不同，防止「缺陷态必失败」退化成恒真。
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态②：均分改全额
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_full_share_formal_passes_the_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_settlement_splits_evenly"
    );
}

#[test]
fn reverse_validation_full_share_degenerate_fails_the_same_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    let degenerate = degenerate_full_share(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_still_passes"
    );
    assert!(
        !settlement_is_exact(&degenerate, &heroes(), DEFECT_TOTAL),
        "reason=degenerate_full_share_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态③：集合改不稳定顺序
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_unstable_order_formal_passes_the_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_settlement_is_stably_ordered"
    );
}

#[test]
fn reverse_validation_unstable_order_degenerate_fails_the_same_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    let degenerate = degenerate_unstable_order(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_still_passes"
    );
    assert!(
        !settlement_is_exact(&degenerate, &heroes(), DEFECT_TOTAL),
        "reason=degenerate_unstable_order_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态④：重复入账
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_double_credit_formal_passes_the_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_settlement_credits_once"
    );
}

#[test]
fn reverse_validation_double_credit_degenerate_fails_the_same_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    let degenerate = degenerate_double_credit(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_still_passes"
    );
    assert!(
        !settlement_is_exact(&degenerate, &heroes(), DEFECT_TOTAL),
        "reason=degenerate_double_credit_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑤：引入浮点（整数复刻浮点舍入语义）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_float_rounding_formal_passes_the_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_settlement_uses_integer_truncation"
    );
}

#[test]
fn reverse_validation_float_rounding_degenerate_fails_the_same_assertion() {
    let formal = formal_settlement(&heroes(), DEFECT_TOTAL);
    let degenerate = degenerate_float_rounding(&heroes(), DEFECT_TOTAL);
    // 前置可分辨性：26 / 4 向零截断为 6、浮点式四舍五入为 7。
    assert_eq!(
        DEFECT_TOTAL / heroes().len() as i64,
        6,
        "reason=truncation_is_6"
    );
    assert_eq!(
        (DEFECT_TOTAL + heroes().len() as i64 / 2) / heroes().len() as i64,
        7,
        "reason=float_rounding_is_7"
    );
    assert!(
        settlement_is_exact(&formal, &heroes(), DEFECT_TOTAL),
        "reason=formal_still_passes"
    );
    assert!(
        !settlement_is_exact(&degenerate, &heroes(), DEFECT_TOTAL),
        "reason=degenerate_float_rounding_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑥：去掉复活绝对上限
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_revive_no_cap_formal_passes_the_assertion() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let formal = constants.cost(HIGH_LEVEL);
    assert!(
        revive_is_exact(&formal, &constants, HIGH_LEVEL),
        "reason=formal_cost_clamps_at_absolute_caps"
    );
}

#[test]
fn reverse_validation_revive_no_cap_degenerate_fails_the_same_assertion() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let formal = constants.cost(HIGH_LEVEL);
    let degenerate = degenerate_revive_no_cap(&constants, HIGH_LEVEL);
    assert!(
        revive_is_exact(&formal, &constants, HIGH_LEVEL),
        "reason=formal_still_passes"
    );
    assert!(
        !revive_is_exact(&degenerate, &constants, HIGH_LEVEL),
        "reason=degenerate_without_cap_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 缺陷态⑦：祭坛与买活参数串用
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_param_crossover_formal_passes_the_assertion() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let formal = constants.cost(CROSSOVER_LEVEL);
    assert!(
        revive_is_exact(&formal, &constants, CROSSOVER_LEVEL),
        "reason=formal_cost_uses_separated_parameters"
    );
}

#[test]
fn reverse_validation_param_crossover_degenerate_fails_the_same_assertion() {
    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let formal = constants.cost(CROSSOVER_LEVEL);
    let degenerate = degenerate_revive_param_crossover(&constants, CROSSOVER_LEVEL);
    assert!(
        revive_is_exact(&formal, &constants, CROSSOVER_LEVEL),
        "reason=formal_still_passes"
    );
    assert!(
        !revive_is_exact(&degenerate, &constants, CROSSOVER_LEVEL),
        "reason=degenerate_param_crossover_must_fail"
    );
    assert_ne!(
        degenerate, formal,
        "reason=degenerate_must_differ_from_formal"
    );
}

// ---------------------------------------------------------------------------
// 元断言汇总：七类缺陷态 MUST 全部与正式实现不同且被同一不变量拦截
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_meta_assertions_guard_all_defect_states() {
    let all = heroes();
    let formal_xp = formal_settlement(&all, DEFECT_TOTAL);
    let xp_degenerates = [
        degenerate_exclude_maxed(&all, DEFECT_TOTAL),
        degenerate_full_share(&all, DEFECT_TOTAL),
        degenerate_unstable_order(&all, DEFECT_TOTAL),
        degenerate_double_credit(&all, DEFECT_TOTAL),
        degenerate_float_rounding(&all, DEFECT_TOTAL),
    ];
    assert!(
        settlement_is_exact(&formal_xp, &all, DEFECT_TOTAL),
        "reason=formal_xp_passes"
    );
    for degenerate in &xp_degenerates {
        assert!(
            !settlement_is_exact(degenerate, &all, DEFECT_TOTAL),
            "reason=every_xp_degenerate_state_must_fail"
        );
        assert_ne!(
            degenerate, &formal_xp,
            "reason=every_xp_degenerate_must_differ_from_formal"
        );
    }

    let constants = SYNTHETIC_REVIVE_CONSTANTS;
    let formal_cap = constants.cost(HIGH_LEVEL);
    let cap_degenerates = [
        degenerate_revive_no_cap(&constants, HIGH_LEVEL),
        degenerate_revive_param_crossover(&constants, HIGH_LEVEL),
    ];
    assert!(
        revive_is_exact(&formal_cap, &constants, HIGH_LEVEL),
        "reason=formal_cap_passes"
    );
    for degenerate in &cap_degenerates {
        assert!(
            !revive_is_exact(degenerate, &constants, HIGH_LEVEL),
            "reason=every_cap_degenerate_state_must_fail"
        );
        assert_ne!(
            degenerate, &formal_cap,
            "reason=every_cap_degenerate_must_differ_from_formal"
        );
    }
}
