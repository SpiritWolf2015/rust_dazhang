//! P18 全量 Melee 机制：编译期合成占位常量与权威运行态容器。
//!
//! 本模块只提供纯整数的数据结构与确定性编码，不读取文件、不依赖
//! `w3-content` / 渲染 / 网络，也不持有闭包、活动对象、墙钟或浮点。
//! 分两层，MUST NOT 混淆（ADR 0002 §6-8 / `design.md` D1）：
//! - **冻结占位层**：全部为编译期合成原创整数常量 `[待核]`，不编入
//!   `compute_defs_hash()`、不改写 `content_hash()`、不冒充经典规则真值；
//!   与 p17 `creep.rs` 底部三类经验机制常量同一体例。
//! - **权威运行态层**：`MeleeMechanismsState` 及其成员，每帧由 `RtsMatch`
//!   独占写入，MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的
//!   快照与 `first_difference()`（四处同步，`CODEBUDDY.md §5`）。
//!
//! 业务规则：R5.3（攻击 × 护甲克制矩阵）/ R6.1（经验阈值分档）/
//! R6.3（祭坛复活与酒馆买活参数分离）/ R6.5（经验两环节分离）/
//! R7.4（固定回购比例 + 交互距离）/ R8.5（三级 leash 归巢）/
//! R1.4（定义层可移动性过滤）。数值一律 `[待核]` 合成占位整数，
//! 不冻结原版真值、不引入原版字段名 / rawcode / 技能名（§14 / ADR 0001 / 0004）。

use std::collections::BTreeMap;

use crate::entity::EntityId;
use crate::fixed::SCALE;
use crate::state_hash::StateHasher;

// ---------------------------------------------------------------------------
// 冻结占位层：编译期合成原创整数常量 `[待核]`
// ---------------------------------------------------------------------------

/// 攻击类型维度数（编译期合成整数 `[待核]`；为维度占位，非原版攻击类型名）。
pub const ATTACK_TYPE_COUNT: usize = 4;
/// 护甲类型维度数（编译期合成整数 `[待核]`；为维度占位，非原版护甲类型名）。
pub const ARMOR_TYPE_COUNT: usize = 4;

/// 「攻击类型 × 护甲类型」二维克制倍率表（百分比整数，编译期合成 `[待核]`）。
///
/// 行 = 攻击类型码、列 = 护甲类型码；数值全为合成原创结构占位、非原版倍率
/// 真值。矩阵只作二维查表结构对齐登记，MUST NOT 冒充倍率已核实
/// （R5.3 / `design.md` G4 / D1）；不编入 `compute_defs_hash()`。
pub const SYNTHETIC_DAMAGE_MATRIX: [[i64; ARMOR_TYPE_COUNT]; ATTACK_TYPE_COUNT] = [
    [100, 90, 80, 70],
    [105, 100, 90, 80],
    [110, 105, 100, 90],
    [115, 110, 105, 100],
];

/// 按二维坐标查克制倍率（纯整数、越界钳到末行 / 末列，不引入浮点）。
///
/// 类型码由稳定 `EntityId` 确定性派生或以权威运行态注入，MUST NOT 出现
/// 原版攻击 / 护甲类型名或 rawcode（R5.3 / R1.2 数据驱动）。
pub fn synthetic_damage_ratio(attack_code: usize, armor_code: usize) -> i64 {
    let row = attack_code.min(ATTACK_TYPE_COUNT - 1);
    let col = armor_code.min(ARMOR_TYPE_COUNT - 1);
    SYNTHETIC_DAMAGE_MATRIX[row][col]
}

/// 按克制倍率缩放伤害得到二维查表的结构占位产出（`i128` 中间量、整数截断、
/// 无浮点，ADR 0002 §1）。仅供权威运行态登记查表产出，MUST NOT 反写实际
/// 伤害结算（倍率 `[待核]`、不冒充已核实真值）。
pub fn scale_damage_by_ratio(base_damage: i64, ratio_percent: i64) -> i64 {
    let scaled = i128::from(base_damage.max(0)) * i128::from(ratio_percent.max(0)) / 100;
    scaled.clamp(0, i128::from(i64::MAX)) as i64
}

/// 英雄经验阈值分档表（编译期合成整数 `[待核]`；递推公式族的基数档）。
///
/// 下标对应「等级减一」档位的累计经验门槛；数值全为合成原创占位、非原版
/// 经验阈值真值。表驱动结构不编入 `compute_defs_hash()`（R6.1 / `design.md` G4）。
pub const SYNTHETIC_HERO_XP_THRESHOLDS: [i64; 4] = [0, 8, 24, 48];

/// 按等级推该档累计经验阈值（递推公式族结构，`[待核]` 合成占位）。
///
/// 表内档直接查表；超出表长按末两档的递推步长整数线性外延，越界与负增长
/// 均由饱和运算兜底，全整数、无浮点、无分支不确定性。
pub fn synthetic_xp_threshold(level: u32) -> i64 {
    let n = SYNTHETIC_HERO_XP_THRESHOLDS.len();
    let index = level.saturating_sub(1) as usize;
    if index < n {
        return SYNTHETIC_HERO_XP_THRESHOLDS[index];
    }
    let last = SYNTHETIC_HERO_XP_THRESHOLDS[n - 1];
    let step = last
        .saturating_sub(SYNTHETIC_HERO_XP_THRESHOLDS[n - 2])
        .max(1);
    last.saturating_add(step.saturating_mul((index - n + 1) as i64))
}

/// 按累计经验查分档表确定档位（表驱动；升级按冻结 `[待核]` 分档常量确定
/// 发生）。档位封顶到表长，纯整数、无浮点，越界经验不会病态循环。
pub fn synthetic_level_tier(xp: i64) -> u32 {
    let mut tier = 1u32;
    for level in 2..=SYNTHETIC_HERO_XP_THRESHOLDS.len() as u32 {
        if xp >= synthetic_xp_threshold(level) {
            tier = level;
        } else {
            break;
        }
    }
    tier
}

/// 酒馆买活的冻结常量集合（全为合成原创编译期整数常量 `[待核]`）。
///
/// 与祭坛复活的 [`crate::gameplay::ReviveConstants`] 是**两套独立参数**：
/// 买活费用更高、入场半血无蓝；祭坛复活的恢复百分比 / 费用 MUST NOT 被
/// 本集合串用（R6.3 / `design.md` G4）。费用按「基数 + 等级系数 → 绝对
/// 上限」表驱动，不编入 `compute_defs_hash()`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuybackConstants {
    /// 买活黄金基数 `[待核]`。
    pub gold_base: i64,
    /// 每英雄等级的黄金系数 `[待核]`。
    pub gold_per_level: i64,
    /// 黄金绝对上限 `[待核]`。
    pub gold_cap: i64,
    /// 买活木材基数 `[待核]`。
    pub lumber_base: i64,
    /// 每英雄等级的木材系数 `[待核]`。
    pub lumber_per_level: i64,
    /// 木材绝对上限 `[待核]`。
    pub lumber_cap: i64,
    /// 买活时间基数（30 Hz 整数帧 `[待核]`）。
    pub frames_base: u32,
    /// 每英雄等级的时间系数（整数帧 `[待核]`）。
    pub frames_per_level: u32,
    /// 买活时间绝对上限（整数帧 `[待核]`）。
    pub frames_cap: u32,
}

/// 合成原创的买活冻结常量 `[待核]`；数值不冒充经典规则、不来自任何资产。
pub const SYNTHETIC_BUYBACK_CONSTANTS: BuybackConstants = BuybackConstants {
    gold_base: 200,
    gold_per_level: 40,
    gold_cap: 900,
    lumber_base: 0,
    lumber_per_level: 0,
    lumber_cap: 0,
    frames_base: 1,
    frames_per_level: 1,
    frames_cap: 60,
};

/// 一次买活按英雄等级确定性结算出的费用与时间（全整数）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuybackCost {
    pub gold: i64,
    pub lumber: i64,
    pub frames: u32,
}

/// 「基数 + 等级系数 → 绝对上限」表驱动计算买活费用 / 时间。
///
/// `i128` 中间量防溢出、整数除法向零截断、在绝对上限处截断不越界无负值、
/// 绝不引入浮点（R6.3 / R9.3 / ADR 0002 §1）。与祭坛
/// [`crate::gameplay::ReviveConstants::cost`] 参数完全分离。
pub fn buyback_cost(level: u32) -> BuybackCost {
    let c = SYNTHETIC_BUYBACK_CONSTANTS;
    let step = i128::from(level.saturating_sub(1));
    let clamp_resource = |base: i64, per: i64, cap: i64| -> i64 {
        let raw = i128::from(base) + i128::from(per) * step;
        raw.min(i128::from(cap)).max(0) as i64
    };
    let frames_raw = i128::from(c.frames_base) + i128::from(c.frames_per_level) * step;
    BuybackCost {
        gold: clamp_resource(c.gold_base, c.gold_per_level, c.gold_cap),
        lumber: clamp_resource(c.lumber_base, c.lumber_per_level, c.lumber_cap),
        frames: frames_raw.min(i128::from(c.frames_cap)).max(1) as u32,
    }
}

/// 酒馆买活入场生命百分比（半血 `[待核]`；祭坛复活 MUST NOT 受此影响）。
pub const SYNTHETIC_BUYBACK_HP_PERCENT: i64 = 50;
/// 酒馆买活入场法力百分比（无蓝 `[待核]`；祭坛复活 MUST NOT 受此影响）。
pub const SYNTHETIC_BUYBACK_MANA_PERCENT: i64 = 0;

/// 固定回购比例百分比（编译期合成整数 `[待核]`；显著低于购入价）。
pub const SYNTHETIC_REPURCHASE_RATIO_PERCENT: i64 = 40;
/// 回购交互距离（Q47.16 raw，编译期合成整数 `[待核]`）。
pub const SYNTHETIC_REPURCHASE_INTERACT_RADIUS_RAW: i64 = 8 * SCALE;

// ---------------------------------------------------------------------------
// 权威运行态层：p18 机制全量的顶层权威状态容器
// ---------------------------------------------------------------------------

/// 三类经验机制逐次击杀结算记录（编译期合成占位常量的作用值，全整数）。
///
/// `building_switch` 是「建筑击杀是否授予经验」布尔开关作用值；
/// `summon_factor_percent` 是召唤物击杀经验折减因子作用值；
/// `tier_factors` 是按稳定 `EntityId` 顺序、依英雄等级分档递减的经验因子
/// 查表结果；`adjusted_xp_total` 是门控 / 折减后按 `i128` 整数截断的经验池。
/// 三类机制只登记 / 派生本记录（R6.1 / R6.5 两环节分离），MUST NOT 改写
/// 集合内均分入账的 `xp_credited` / `state.xp` 既有数值（p17 锁定行为）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct XpSettlementRecord {
    pub building_switch: u8,
    pub summon_factor_percent: i64,
    pub tier_factors: BTreeMap<EntityId, i64>,
    pub adjusted_xp_total: i64,
}

/// 酒馆买活登记（半血无蓝入场参数与费用 / 时间，全整数 `[待核]`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuybackRecord {
    pub hp_percent: i64,
    pub mana_percent: i64,
    pub gold: i64,
    pub lumber: i64,
    pub frames: u32,
}

/// 回购成交登记（固定回购比例 + 交互距离内的成交价，全整数 `[待核]`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RepurchaseRecord {
    pub price: i64,
    pub ratio_percent: i64,
}

/// P18 全量 Melee 机制权威运行态的唯一顶层容器。
///
/// 全部落 `BTreeMap` 确定性有序容器、按稳定 `EntityId` 顺序遍历；字段私有
/// 由 `RtsMatch` 独占写入，MUST 同时进入 FNV-1a 状态哈希
/// （`compute_state_hash`）、由 `SNAPSHOT_VERSION` 定义的 `Snapshot`、
/// `first_difference()` 与 `restore_snapshot` 四处（ADR 0002 §6-8 /
/// `CODEBUDDY.md §5`）。机制常量属冻结占位层、不进本容器之外；本容器只
/// 存每帧结算派生出的运行态，MUST NOT 含 fog / 视野 / 表现层派生结果。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MeleeMechanismsState {
    /// 三类经验机制逐次结算记录（键 = 被击杀野怪 `EntityId`，稳定序）。
    pub xp_settlements: BTreeMap<EntityId, XpSettlementRecord>,
    /// R6.1 经验阈值分档运行态（键 = 英雄 `EntityId`）：按累计经验查表推得的档位。
    pub hero_tiers: BTreeMap<EntityId, u32>,
    /// R5.3 克制矩阵查表产出（键 = `(攻击方, 受击方)` 稳定 `EntityId` 对）：
    /// 按二维查表倍率整数缩放后的结构占位伤害，不同组合确定不同产出。
    pub damage_outcomes: BTreeMap<(EntityId, EntityId), i64>,
    /// R6.3 酒馆买活登记（键 = 英雄 `EntityId`）：半血无蓝入场参数。
    pub buybacks: BTreeMap<EntityId, BuybackRecord>,
    /// R7.4 回购成交登记（键 = 物品实例 `EntityId`）：成交价与固定比例。
    pub repurchases: BTreeMap<EntityId, RepurchaseRecord>,
    /// R8.5 leash 归巢完成累计（键 = 野怪 `EntityId`）：确定性计数。
    pub leash_returns: BTreeMap<EntityId, u32>,
}

impl MeleeMechanismsState {
    /// 全部权威运行态显式小端编码进 FNV-1a；按稳定键顺序遍历。
    /// 新增字段 MUST 同步接入本方法、`first_difference()` 与快照
    /// （`CODEBUDDY.md §5` 硬门禁）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.xp_settlements.len() as u64);
        for (&creep, record) in &self.xp_settlements {
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

        hasher.write_u64(self.hero_tiers.len() as u64);
        for (&hero, &tier) in &self.hero_tiers {
            hasher.write_u64(hero.value());
            hasher.write_u32(tier);
        }

        hasher.write_u64(self.damage_outcomes.len() as u64);
        for (&(attacker, target), &outcome) in &self.damage_outcomes {
            hasher.write_u64(attacker.value());
            hasher.write_u64(target.value());
            hasher.write_i64(outcome);
        }

        hasher.write_u64(self.buybacks.len() as u64);
        for (&hero, record) in &self.buybacks {
            hasher.write_u64(hero.value());
            hasher.write_i64(record.hp_percent);
            hasher.write_i64(record.mana_percent);
            hasher.write_i64(record.gold);
            hasher.write_i64(record.lumber);
            hasher.write_u32(record.frames);
        }

        hasher.write_u64(self.repurchases.len() as u64);
        for (&instance, record) in &self.repurchases {
            hasher.write_u64(instance.value());
            hasher.write_i64(record.price);
            hasher.write_i64(record.ratio_percent);
        }

        hasher.write_u64(self.leash_returns.len() as u64);
        for (&creep, &count) in &self.leash_returns {
            hasher.write_u64(creep.value());
            hasher.write_u32(count);
        }
    }

    /// 按同一稳定顺序返回首个差异字段名，便于 desync 诊断（ADR 0002 §6）。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.xp_settlements != other.xp_settlements {
            return Some("melee_xp_settlements");
        }
        if self.hero_tiers != other.hero_tiers {
            return Some("melee_hero_tiers");
        }
        if self.damage_outcomes != other.damage_outcomes {
            return Some("melee_damage_outcomes");
        }
        if self.buybacks != other.buybacks {
            return Some("melee_buybacks");
        }
        if self.repurchases != other.repurchases {
            return Some("melee_repurchases");
        }
        if self.leash_returns != other.leash_returns {
            return Some("melee_leash_returns");
        }
        None
    }
}
