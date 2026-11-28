//! P18 全量 Melee 机制 apply 第 1 批的 CPU 正向回归（w3-core 侧）。
//!
//! 本文件只放**正向断言**（改坏验红反例 / 回归八类 / p17 R2 生产结算 A-B
//! 由第 2 批补齐，用户已拍板）。覆盖范围（均为编译期合成整数 `[待核]`
//! 占位、零文件 IO、零资产、不依赖 `w3-content` / 渲染 / 网络）：
//! - R1.4：「工人 + 主城 + 金矿」混合选中集合移动过滤，仅工人响应 `Move`；
//! - ADR 0002 §6-8：新增 `MeleeMechanismsState` 六字段进状态哈希 /
//!   `first_difference()` / 快照往返四处同步；
//! - R6.1 / R6.5：三类经验机制消费者落在生产结算路径且集合内均分入账不退化；
//! - R6.3 / R9.3：酒馆买活半血无蓝、参数与祭坛复活分离、费用绝对上限；
//! - R5.3 / R6.1：二维克制矩阵与经验阈值分档的编译期合成表 / 递推公式族。
//!
//! 全部数值为合成原创编译期整数 `[待核]`，不冒充经典规则事实
//! （`WC3-MELEE-RULES.md §14` / ADR 0001 / ADR 0004）。

mod creep_common;

use std::collections::BTreeSet;

use creep_common::{attack, seeded_match, with_hero};
use w3_core::{
    ARMOR_TYPE_COUNT, ATTACK_TYPE_COUNT, BuybackCost, Command, CommandKind, EntityId, Frame,
    GameplayCommand, GameplayCommandKind, HeroDeathRecord, HeroState, MECHANISM_BUILDING_XP_SWITCH,
    MatchConfig, MeleeMechanismsState, NeutralCategory, OrderRecord, RtsMatch, SCALE,
    SYNTHETIC_BUYBACK_CONSTANTS, SYNTHETIC_BUYBACK_HP_PERCENT, SYNTHETIC_BUYBACK_MANA_PERCENT,
    SYNTHETIC_DAMAGE_MATRIX, SYNTHETIC_HERO_XP_THRESHOLDS, SYNTHETIC_REVIVE_CONSTANTS, SimFixed,
    StaticWorld, buyback_cost, restore_by_percent, scale_damage_by_ratio, synthetic_damage_ratio,
    synthetic_hero_level_xp_factor, synthetic_level_tier, synthetic_xp_threshold,
};

/// 干净环境对局：三个 frame-0 `Spawn` 实体，坐标互异（合成占位 `[待核]`）。
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

/// 两个同脚本英雄对局（各自独立的冻结定义 / 开局脚本），供逐帧交错比对。
fn two_hero_matches() -> (RtsMatch, RtsMatch) {
    let (a, content_a) = seeded_match();
    let (b, content_b) = seeded_match();
    (with_hero(a, content_a), with_hero(b, content_b))
}

// ---------------------------------------------------------------------------
// 1. R1 / R1.4 可移动性边界（brief 第 7 项「断言要在」）
// ---------------------------------------------------------------------------

/// 混合选中集合（工人 / 主城 / 金矿）对地面下达移动：仅工人生成 `Move`，
/// 主城（运行时动态建筑 `speed == 0`）与金矿（中立资源点登记）
/// MUST NOT 响应 `Move`、MUST NOT 改变位置（R1.1 / R1.4 / D7）。
///
/// 本回归模拟表现层「未过滤」的最坏输入（每个选中成员各提交一条 `Move`），
/// 由内核命令分发处的只读可移动性派生守住边界，防止已知 bug 复发。
/// 判据用定义层 `speed_raw` 在运行时 `UnitState.speed` 的投影与中立类别
/// 登记，MUST NOT 用建筑名字符串 / DefId 白名单（R1.2 / R13.3）。
#[test]
fn mixed_selection_only_movable_worker_responds_to_move() {
    let (sim, content) = three_unit_match();
    let mut snapshot = sim.capture_snapshot();
    // 主城：运行时动态建筑，`speed == 0` 即定义层不可移动（R1.2 数据驱动）。
    snapshot
        .positions
        .iter_mut()
        .find(|record| record.entity == 2)
        .unwrap()
        .speed = 0;
    // 金矿：中立资源点登记（R1.1 / R1.5），由既有 `is_actionable` 过滤。
    snapshot
        .creep_camps
        .neutral
        .insert(EntityId::new(3).unwrap(), NeutralCategory::CreepHostile);
    // 注入后的权威状态是本回归的比较基准（非旁路手搓，走 restore 生产路径）。
    let injected = snapshot.clone();
    let mut sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();

    // 混合选中集合一次性下达移动（表现层未过滤的极端输入）。
    for (index, entity) in [1u64, 2, 3].iter().enumerate() {
        sim.submit_command(Command::new(
            Frame::new(1),
            1,
            index as u32 + 1,
            CommandKind::Move {
                entity: EntityId::new(*entity).unwrap(),
                x: SimFixed::from_int(5).unwrap(),
                y: SimFixed::from_int(5).unwrap(),
                group: 0,
                append: false,
            },
        ))
        .unwrap();
    }
    sim.step();
    let after = sim.capture_snapshot();
    let worker = after.positions.iter().find(|p| p.entity == 1).unwrap();
    let town_hall = after.positions.iter().find(|p| p.entity == 2).unwrap();
    let gold_mine = after.positions.iter().find(|p| p.entity == 3).unwrap();

    // 仅工人生成 Move 订单，落点由既有稳定 EntityId 顺序派生（D7）。
    assert!(
        matches!(worker.order, OrderRecord::Move { .. }),
        "reason=only_movable_worker_generates_move"
    );
    // 主城 / 金矿订单保持 Idle、位置不变。
    let b_town = injected.positions.iter().find(|p| p.entity == 2).unwrap();
    let b_mine = injected.positions.iter().find(|p| p.entity == 3).unwrap();
    assert_eq!(
        town_hall.order,
        OrderRecord::Idle,
        "reason=town_hall_no_move"
    );
    assert_eq!(
        gold_mine.order,
        OrderRecord::Idle,
        "reason=gold_mine_no_move"
    );
    assert_eq!(
        (town_hall.x, town_hall.y),
        (b_town.x, b_town.y),
        "reason=town_hall_position_fixed"
    );
    assert_eq!(
        (gold_mine.x, gold_mine.y),
        (b_mine.x, b_mine.y),
        "reason=gold_mine_position_fixed"
    );
    // 过滤只读派生，MUST NOT 改写中立登记等无关权威状态（D7 零影响）。
    assert_eq!(
        after.creep_camps.neutral, injected.creep_camps.neutral,
        "reason=neutral_registration_not_rewritten"
    );
}

// ---------------------------------------------------------------------------
// 2. ADR 0002 §6-8 新增权威运行态字段四处同步
// ---------------------------------------------------------------------------

/// 两局仅 `MeleeMechanismsState` 某字段取值不同：`compute_state_hash()`
/// 必不同、`first_difference()` 报告该字段诊断名、快照往返逐字节一致。
/// 若新增字段漏接哈希 / 差异诊断 / 恢复，本断言必失败而不是绕过
/// （`CODEBUDDY.md §5` / spec Requirement 3 / D3 / D5）。
#[test]
fn melee_mechanisms_fields_enter_hash_first_difference_and_snapshot() {
    let sim = RtsMatch::create(MatchConfig::default());
    let content = sim.config().content_hash;
    let mut snapshot = sim.capture_snapshot();
    // 经真实 capture_snapshot → 改 public 字段 → restore_snapshot 生产路径注入，
    // 非旁路手搓权威状态（沿用 p17 体例）。
    snapshot
        .melee_mechanisms
        .hero_tiers
        .insert(EntityId::new(1).unwrap(), 3);
    let variant =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();

    assert_ne!(
        sim.compute_state_hash(),
        variant.compute_state_hash(),
        "reason=new_field_enters_state_hash"
    );
    assert_eq!(
        sim.first_difference(&variant),
        Some("melee_hero_tiers"),
        "reason=new_field_reported_by_first_difference"
    );
    assert_eq!(
        variant.capture_snapshot(),
        snapshot,
        "reason=snapshot_roundtrip_byte_equal"
    );
    // 六字段各自有独立诊断名（按稳定顺序），漏接任一字段都可定位。
    let base = MeleeMechanismsState::default();
    assert_eq!(
        base.first_difference(&snapshot.melee_mechanisms),
        Some("melee_hero_tiers"),
        "reason=container_first_difference_field_name"
    );
}

// ---------------------------------------------------------------------------
// 3. R6.1 / R6.5 三类机制消费者落在生产结算路径
// ---------------------------------------------------------------------------

/// 三类经验机制（布尔开关 / 召唤物折减 / 等级分档查表）作为消费者落在
/// `settle_creep_deaths` 生产结算路径：结算登记进顶层权威运行态，且
/// p17 已锁定的「集合内均分入账」数值 MUST NOT 退化（D1 / p17 R3 缺口）。
#[test]
fn three_xp_mechanisms_are_consumed_in_production_settlement() {
    let (sim, content) = seeded_match();
    let mut sim = with_hero(sim, content);
    let creeps: Vec<EntityId> = sim.creep_camps().creeps.keys().copied().collect();
    let baseline_hash = sim.compute_state_hash();
    // 同一击杀脚本提交（英雄 1 攻击两只野怪，稳定 Sequence 排序）。
    for (index, creep) in creeps.iter().enumerate() {
        attack(&mut sim, 1, 1, creep.value(), index as u32 + 1);
    }
    sim.step();

    let melee = sim.melee_mechanisms_state();
    let camps = sim.creep_camps();
    assert_eq!(
        melee.xp_settlements.len(),
        camps.kill_rewards.len(),
        "reason=one_mechanism_record_per_settled_creep"
    );
    assert!(
        !melee.xp_settlements.is_empty(),
        "reason=mechanism_consumers_are_wired_into_settlement"
    );
    let hero = EntityId::new(1).unwrap();
    for record in melee.xp_settlements.values() {
        assert_eq!(
            record.building_switch, MECHANISM_BUILDING_XP_SWITCH,
            "reason=building_xp_switch_consumed"
        );
        // 击杀者为普通英雄（非召唤物），折减因子取中性 100%。
        assert_eq!(
            record.summon_factor_percent, 100,
            "reason=summon_factor_consumed_for_non_summon_killer"
        );
        // 等级分档查表按稳定 EntityId 顺序登记，英雄 1 级取首档。
        assert_eq!(
            record.tier_factors.get(&hero),
            Some(&synthetic_hero_level_xp_factor(1)),
            "reason=hero_level_tier_lookup_consumed"
        );
        assert!(
            record.adjusted_xp_total > 0,
            "reason=i128_adjusted_xp_pool_is_positive"
        );
    }
    // R6.1 阈值分档运行态按入账后累计经验派生。
    assert!(
        melee.hero_tiers.contains_key(&hero),
        "reason=xp_threshold_tier_derived_at_settlement"
    );
    // R5.3 克制矩阵查表产出按稳定攻击对登记。
    assert!(
        !melee.damage_outcomes.is_empty(),
        "reason=damage_matrix_consumed_in_attack_settlement"
    );

    // p17 锁定行为：集合内均分入账数值不退化（集合内英雄共用同一 share）。
    for reward in camps.kill_rewards.values() {
        let credited: BTreeSet<i64> = reward.xp_credited.values().copied().collect();
        assert_eq!(
            credited.len(),
            1,
            "reason=even_split_unchanged_by_mechanism_wiring"
        );
    }
    assert_ne!(
        sim.compute_state_hash(),
        baseline_hash,
        "reason=state_hash_advances_after_settlement"
    );
}

/// 双实例交错逐帧哈希 / `first_difference()` 一致（ADR 0002 §8）：
/// 两局从同一冻结定义 + 同一开局脚本重建，提交同一击杀脚本后交错推进。
#[test]
fn per_frame_hash_matches_same_script_baseline_after_mechanism_wiring() {
    let (mut a, mut b) = two_hero_matches();
    let creeps: Vec<EntityId> = a.creep_camps().creeps.keys().copied().collect();
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=same_script_same_initial_hash"
    );
    for (index, creep) in creeps.iter().enumerate() {
        let sequence = index as u32 + 1;
        attack(&mut a, 1, 1, creep.value(), sequence);
        attack(&mut b, 1, 1, creep.value(), sequence);
    }
    for _ in 0..8 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=per_frame_hash_matches_baseline"
        );
        assert_eq!(a.first_difference(&b), None, "reason=no_first_difference");
    }
    // 快照往返：恢复后与销毁前逐帧一致（新增字段进 v10 快照）。
    let snapshot = a.capture_snapshot();
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
        a.step();
        restored.step();
        assert_eq!(
            a.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=restored_matches_baseline"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. R6.3 / R9.3 酒馆买活半血无蓝 + 参数分离 + 绝对上限
// ---------------------------------------------------------------------------

/// 酒馆买活（`BuybackHero`，p18 唯一新增命令变体）半血无蓝入场，
/// 费用 / 时间按独立 `BuybackConstants` 的「基数 + 等级系数 → 绝对上限」
/// 计算；祭坛 `ReviveConstants` 的恢复百分比 / 费用 MUST NOT 被串用
/// （R6.3 / D4 / G4）。
#[test]
fn buyback_hero_enters_half_hp_no_mana_and_altar_params_are_separated() {
    let (sim, content) = three_unit_match();
    let mut snapshot = sim.capture_snapshot();
    let hero = EntityId::new(1).unwrap();
    let tavern = EntityId::new(2).unwrap();
    // 英雄阵亡 + 死亡登记：经真实快照注入生产路径构造（非旁路手搓）。
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
    snapshot.gameplay.players.entry(1).or_default().gold = 1000;
    let mut sim = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();

    // 新命令变体经公开命令面提交（校验 / 冻结到达序号 / 稳定排序 / 穷尽分发）。
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::BuybackHero { hero, tavern },
    ))
    .unwrap();
    sim.step();

    let after = sim.capture_snapshot();
    let revived = after
        .positions
        .iter()
        .find(|p| p.entity == hero.value())
        .unwrap();
    assert!(revived.alive, "reason=buyback_hero_is_alive");
    assert_eq!(
        revived.hp,
        restore_by_percent(100, SYNTHETIC_BUYBACK_HP_PERCENT),
        "reason=buyback_enters_half_hp"
    );
    assert_eq!(
        revived.gameplay.mana,
        restore_by_percent(100, SYNTHETIC_BUYBACK_MANA_PERCENT),
        "reason=buyback_enters_no_mana"
    );
    assert_eq!(revived.hp, 50, "reason=half_hp_synthetic_placeholder");
    assert_eq!(
        revived.gameplay.mana, 0,
        "reason=no_mana_synthetic_placeholder"
    );

    // 权威运行态登记买活参数来源（进哈希 / 快照 / 差异诊断）。
    let melee = sim.melee_mechanisms_state();
    let record = melee.buybacks.get(&hero).unwrap();
    assert_eq!(
        record.hp_percent, SYNTHETIC_BUYBACK_HP_PERCENT,
        "reason=buyback_hp_param_recorded"
    );
    assert_eq!(
        record.mana_percent, SYNTHETIC_BUYBACK_MANA_PERCENT,
        "reason=buyback_mana_param_recorded"
    );
    let level_cost: BuybackCost = buyback_cost(3);
    assert_eq!(record.gold, level_cost.gold, "reason=buyback_cost_recorded");
    assert_eq!(
        after.gameplay.players.get(&1).unwrap().gold,
        1000 - level_cost.gold,
        "reason=buyback_deducts_exact_gold"
    );

    // 参数分离：买活半血无蓝 / 更高费用 MUST NOT 与祭坛满血满蓝串用。
    let altar = SYNTHETIC_REVIVE_CONSTANTS.cost(3);
    assert_ne!(
        SYNTHETIC_BUYBACK_HP_PERCENT, SYNTHETIC_REVIVE_CONSTANTS.hp_restore_percent,
        "reason=hp_params_are_separated"
    );
    assert_ne!(
        SYNTHETIC_BUYBACK_MANA_PERCENT, SYNTHETIC_REVIVE_CONSTANTS.mana_restore_percent,
        "reason=mana_params_are_separated"
    );
    assert_ne!(
        level_cost.gold, altar.gold,
        "reason=cost_params_are_separated"
    );
    assert!(
        SYNTHETIC_BUYBACK_HP_PERCENT < SYNTHETIC_REVIVE_CONSTANTS.hp_restore_percent
            && SYNTHETIC_BUYBACK_MANA_PERCENT < SYNTHETIC_REVIVE_CONSTANTS.mana_restore_percent,
        "reason=buyback_strictly_weaker_than_altar"
    );

    // 「基数 + 等级系数 → 绝对上限」：极端等级在绝对上限处截断、无负值、不 panic。
    let extreme = buyback_cost(u32::MAX);
    assert_eq!(
        extreme.gold, SYNTHETIC_BUYBACK_CONSTANTS.gold_cap,
        "reason=gold_absolute_cap"
    );
    assert_eq!(
        extreme.frames, SYNTHETIC_BUYBACK_CONSTANTS.frames_cap,
        "reason=frames_absolute_cap"
    );
    assert!(
        extreme.gold >= 0 && extreme.lumber >= 0 && extreme.frames >= 1,
        "reason=no_negative_or_zero_frames"
    );
    assert_eq!(
        extreme,
        buyback_cost(1000),
        "reason=u32_max_does_not_overflow"
    );
}

/// 新命令变体的实体合法性校验：`hero` / `tavern` MUST NOT 为 `EntityId::NONE`
/// （ADR 0002 §3 提交时冻结到达序号 + 执行时重新校验）。
#[test]
fn buyback_hero_command_rejects_none_entities() {
    let (mut sim, _content) = three_unit_match();
    let none = GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::BuybackHero {
            hero: EntityId::NONE,
            tavern: EntityId::new(2).unwrap(),
        },
    );
    assert!(
        sim.submit_gameplay_command(none).is_err(),
        "reason=none_hero_rejected_at_submission"
    );
    let none_tavern = GameplayCommand::new(
        Frame::new(1),
        1,
        2,
        GameplayCommandKind::BuybackHero {
            hero: EntityId::new(1).unwrap(),
            tavern: EntityId::NONE,
        },
    );
    assert!(
        sim.submit_gameplay_command(none_tavern).is_err(),
        "reason=none_tavern_rejected_at_submission"
    );
}

// ---------------------------------------------------------------------------
// 5. R5.3 / R6.1 编译期合成表 / 递推公式族（干净环境、纯整数、零 IO）
// ---------------------------------------------------------------------------

/// 二维克制矩阵与经验阈值分档为编译期合成整数常量表 `[待核]`，
/// 不同组合按二维查表确定产出、递推公式族确定性外延、整数缩放无 `f64`
/// （R5.3 / R6.1 / ADR 0002 §1 / D1 / G4）。
#[test]
fn synthetic_tables_and_formula_families_are_integer_only() {
    // 二维矩阵：维度编译期常量，不同攻击 / 护甲组合不同产出。
    assert!(
        ATTACK_TYPE_COUNT >= 2,
        "reason=attack_dimension_non_trivial"
    );
    assert!(ARMOR_TYPE_COUNT >= 2, "reason=armor_dimension_non_trivial");
    assert_eq!(
        SYNTHETIC_DAMAGE_MATRIX.len(),
        ATTACK_TYPE_COUNT,
        "reason=matrix_rows"
    );
    assert_eq!(
        SYNTHETIC_DAMAGE_MATRIX[0].len(),
        ARMOR_TYPE_COUNT,
        "reason=matrix_columns"
    );
    assert_ne!(
        synthetic_damage_ratio(0, 0),
        synthetic_damage_ratio(1, 2),
        "reason=two_dimensional_lookup_varies_by_pair"
    );
    // 越界确定性钳到末行 / 末列，不引入分支不确定性。
    assert_eq!(
        synthetic_damage_ratio(usize::MAX, usize::MAX),
        synthetic_damage_ratio(ATTACK_TYPE_COUNT - 1, ARMOR_TYPE_COUNT - 1),
        "reason=ratio_lookup_clamps_out_of_range"
    );
    // 整数缩放：i128 中间量、向零截断、负值钳 0。
    assert_eq!(
        scale_damage_by_ratio(100, 90),
        90,
        "reason=integer_ratio_scale"
    );
    assert_eq!(
        scale_damage_by_ratio(100, 150),
        150,
        "reason=integer_ratio_amplify"
    );
    assert_eq!(
        scale_damage_by_ratio(-5, 200),
        0,
        "reason=negative_damage_clamped"
    );
    assert_eq!(
        scale_damage_by_ratio(100, -1),
        0,
        "reason=negative_ratio_clamped"
    );

    // 阈值分档表严格递增 + 递推公式族线性外延（全整数、无浮点）。
    assert!(
        SYNTHETIC_HERO_XP_THRESHOLDS
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "reason=xp_thresholds_strictly_increasing"
    );
    for level in 1..=SYNTHETIC_HERO_XP_THRESHOLDS.len() as u32 {
        assert_eq!(
            synthetic_xp_threshold(level),
            SYNTHETIC_HERO_XP_THRESHOLDS[level as usize - 1],
            "reason=in_table_threshold_is_exact"
        );
    }
    let n = SYNTHETIC_HERO_XP_THRESHOLDS.len();
    let step = SYNTHETIC_HERO_XP_THRESHOLDS[n - 1] - SYNTHETIC_HERO_XP_THRESHOLDS[n - 2];
    assert_eq!(
        synthetic_xp_threshold(n as u32 + 1),
        SYNTHETIC_HERO_XP_THRESHOLDS[n - 1] + step,
        "reason=recurrence_extends_past_table"
    );
    // 表驱动档位：xp 落在哪档由冻结常量确定，越界经验钳到末档不病态循环。
    assert_eq!(synthetic_level_tier(0), 1, "reason=tier_one_at_zero_xp");
    assert_eq!(
        synthetic_level_tier(SYNTHETIC_HERO_XP_THRESHOLDS[1]),
        2,
        "reason=tier_two_at_second_threshold"
    );
    assert_eq!(
        synthetic_level_tier(i64::MAX / 4),
        SYNTHETIC_HERO_XP_THRESHOLDS.len() as u32,
        "reason=tier_capped_at_table_length"
    );
    // 等级分档递减因子查表（p17 结构，本批接线复用）。
    assert!(
        synthetic_hero_level_xp_factor(1) > synthetic_hero_level_xp_factor(2)
            && synthetic_hero_level_xp_factor(2) > synthetic_hero_level_xp_factor(u32::MAX),
        "reason=hero_level_factor_strictly_decreasing"
    );
    // 全部数值以 Q47.16 原始值 / 整数百分比表达，SCALE 是编译期整数。
    assert_eq!(SCALE, 1i64 << 16, "reason=q47_16_integer_scale");
}
