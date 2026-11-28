//! P5 玩法系统：冻结内容定义、统一结算、技能三层扩展与可保存运行态。
//!
//! 本模块只提供纯整数的数据结构与确定性规则，不读取文件、不依赖
//! `w3-content` / 渲染 / 网络，也不持有闭包或活动对象。所有权威状态
//! 都可由显式数据重建，并同时进入状态哈希与快照（ADR 0002）。
//!
//! 设计要点：
//! - 冻结定义在对局创建时校验并冻结，类别内 ID 唯一，运行期无可变 API。
//! - 生命 / 法力 / 伤害 / 治疗 / Buff / 属性修正由 [`Settlement`] 单一所有者处理。
//! - 技能采用「模板 / 效果组合 / 按行为 ID 注册」三层扩展，不引入脚本 VM。
//! - 玩家与 AI 命令共用 [`GameplayCommand`] 的校验与执行入口，AI 每帧确定性重算，
//!   内部 AI 命令不写回命令队列，重放只注入外部命令。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ability_system::{AbilitySystemDefinitions, AbilitySystemState, BuffCategory};
use crate::creep::{
    CampScript, CreepConstants, DropTable, NeutralCategory, SYNTHETIC_CREEP_CONSTANTS,
};
use crate::entity::EntityId;
use crate::faction_mechanics::{
    FactionMechanicsConstants, RegenCondition, SYNTHETIC_FACTION_MECHANICS, hash_regen_conditions,
};
use crate::frame::Frame;
use crate::item_shop::ItemShopDefinitions;
use crate::state_hash::StateHasher;

/// 冻结定义内容哈希的版本域分隔；编码变化时递增。
pub const CONTENT_HASH_SCHEMA: u64 = 1;

/// 内容定义 ID。
pub type DefId = u32;

// ---------------------------------------------------------------------------
// 英雄复活冻结常量（P8.5a / Q3）
// ---------------------------------------------------------------------------

/// 祭坛英雄复活的冻结常量集合（全为合成 / 原创编译期整数常量 `[待核]`）。
///
/// 所有字段都是编译期整数，不含 `f64`、MPQ 派生数据、真实导出 JSON、
/// 参考项目单位名或 rawcode。费用按「基数 + 等级系数 → 绝对上限」表驱动
/// 计算。P18 起祭坛复活与酒馆买活的参数 / 命令分离已落地：祭坛复活沿用
/// 本套常量、`ReviveHero` 命令与队列恢复满生命 / 满法力；酒馆买活走
/// `BuybackHero` 命令与独立的 `BuybackConstants`，以半血无蓝入场，
/// MUST NOT 串用本套恢复百分比 / 费用参数（R6.3 / `design.md` G4）。
/// 改任一常量即改变 [`FrozenDefinitions::content_hash`]，进而改变快照头部。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReviveConstants {
    /// 复活黄金基数。
    pub gold_base: i64,
    /// 每英雄等级的黄金系数。
    pub gold_per_level: i64,
    /// 黄金绝对上限。
    pub gold_cap: i64,
    /// 复活木材基数。
    pub lumber_base: i64,
    /// 每英雄等级的木材系数。
    pub lumber_per_level: i64,
    /// 木材绝对上限。
    pub lumber_cap: i64,
    /// 复活占用的人口（恢复既有实体、不新增实体，合成约定取 0）。
    pub food_cost: i64,
    /// 复活时间基数（30 Hz 整数帧）。
    pub frames_base: u32,
    /// 每英雄等级的复活时间系数（整数帧）。
    pub frames_per_level: u32,
    /// 复活时间绝对上限（整数帧）。
    pub frames_cap: u32,
    /// 复活恢复生命的整数百分比（100 表示满生命）。
    pub hp_restore_percent: i64,
    /// 复活恢复法力的整数百分比（100 表示满法力）。
    pub mana_restore_percent: i64,
}

/// 合成 / 原创的复活冻结常量 `[待核]`；数值不冒充经典规则、不来自任何资产。
pub const SYNTHETIC_REVIVE_CONSTANTS: ReviveConstants = ReviveConstants {
    gold_base: 100,
    gold_per_level: 25,
    gold_cap: 400,
    lumber_base: 20,
    lumber_per_level: 5,
    lumber_cap: 100,
    food_cost: 0,
    frames_base: 60,
    frames_per_level: 15,
    frames_cap: 300,
    hp_restore_percent: 100,
    mana_restore_percent: 100,
};

impl Default for ReviveConstants {
    fn default() -> Self {
        SYNTHETIC_REVIVE_CONSTANTS
    }
}

/// 一次复活按英雄等级确定性结算出的费用与时间（全整数）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReviveCost {
    pub gold: i64,
    pub lumber: i64,
    pub food: i64,
    pub frames: u32,
}

impl ReviveConstants {
    /// 「基数 + 等级系数 → 绝对上限」表驱动计算；`i128` 中间量防溢出，
    /// 整数除法向零截断，绝不引入浮点（ADR 0002 §1）。
    pub fn cost(&self, level: u32) -> ReviveCost {
        let step = i128::from(level.saturating_sub(1));
        let clamp_gold = |base: i64, per: i64, cap: i64| -> i64 {
            let raw = i128::from(base) + i128::from(per) * step;
            raw.min(i128::from(cap)).max(0) as i64
        };
        let clamp_frames = |base: u32, per: u32, cap: u32| -> u32 {
            let raw = i128::from(base) + i128::from(per) * step;
            raw.min(i128::from(cap)).max(1) as u32
        };
        ReviveCost {
            gold: clamp_gold(self.gold_base, self.gold_per_level, self.gold_cap),
            lumber: clamp_gold(self.lumber_base, self.lumber_per_level, self.lumber_cap),
            food: self.food_cost,
            frames: clamp_frames(self.frames_base, self.frames_per_level, self.frames_cap),
        }
    }

    /// 显式小端编码进定义哈希；任一常量变化都改变结果。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_i64(self.gold_base);
        hasher.write_i64(self.gold_per_level);
        hasher.write_i64(self.gold_cap);
        hasher.write_i64(self.lumber_base);
        hasher.write_i64(self.lumber_per_level);
        hasher.write_i64(self.lumber_cap);
        hasher.write_i64(self.food_cost);
        hasher.write_u32(self.frames_base);
        hasher.write_u32(self.frames_per_level);
        hasher.write_u32(self.frames_cap);
        hasher.write_i64(self.hp_restore_percent);
        hasher.write_i64(self.mana_restore_percent);
    }
}

/// 按整数百分比恢复数值（复活生命 / 法力用），`i128` 中间量、全整数。
pub fn restore_by_percent(max_value: i64, percent: i64) -> i64 {
    let cap = i128::from(max_value.max(0));
    let scaled = cap * i128::from(percent) / 100;
    scaled.clamp(0, cap) as i64
}

// ---------------------------------------------------------------------------
// 主城升级冻结常量与等级枚举（P8.5b / Q3 / D2）
// ---------------------------------------------------------------------------

/// 主城等级（离散 T1 / T2 / T3，R4.1 可升级两次）。
///
/// 该类型是定义层 tier 需求链、顶层权威状态与升级命令共用的离散值，
/// 全部以显式 [`TownHallTier::code`] 字节编码进入哈希 / 快照 / 命令编码，
/// 不依赖枚举判别式的内存布局，保证跨平台确定性（ADR 0002 §6）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum TownHallTier {
    /// 初始等级。
    T1,
    /// 第二次等级（第一次升级后）。
    T2,
    /// 最高等级（第二次升级后）。
    T3,
}

impl TownHallTier {
    /// 稳定的小端无关单字节编码；值域固定为 1 / 2 / 3。
    pub const fn code(self) -> u8 {
        match self {
            TownHallTier::T1 => 1,
            TownHallTier::T2 => 2,
            TownHallTier::T3 => 3,
        }
    }

    /// 从显式编码还原等级；非法值返回 `None`。供线格式整数 ↔ 枚举对齐，
    /// 避免下游各自硬编码判别式（ADR 0005 canonical 约束）。
    pub const fn from_code(code: u8) -> Option<TownHallTier> {
        match code {
            1 => Some(TownHallTier::T1),
            2 => Some(TownHallTier::T2),
            3 => Some(TownHallTier::T3),
            _ => None,
        }
    }

    /// 下一级等级；T3 为顶，返回 `None`（逐级递进，禁止跳级）。
    pub const fn successor(self) -> Option<TownHallTier> {
        match self {
            TownHallTier::T1 => Some(TownHallTier::T2),
            TownHallTier::T2 => Some(TownHallTier::T3),
            TownHallTier::T3 => None,
        }
    }
}

impl Default for TownHallTier {
    fn default() -> Self {
        TownHallTier::T1
    }
}

/// 主城升级冻结常量集合（全为合成 / 原创编译期整数常量 `[待核]`）。
///
/// 费用按「T1→T2 基础费用 + T2→T3 差价」表驱动计算，MUST NOT 把导出
/// 总价直接当升级扣款（`CLASSIC_RULES_BASELINE.md` 语义，只借机制不借值）。
/// 时间为 30 Hz 整数帧，不含 `f64` / MPQ 派生 / 真实导出 JSON / 参考项目
/// 单位名或 rawcode。改任一常量即改变 [`FrozenDefinitions::content_hash`]。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TownHallConstants {
    /// T1→T2 升级黄金基础费用。
    pub tier2_gold: i64,
    /// T1→T2 升级木材基础费用。
    pub tier2_lumber: i64,
    /// T2→T3 相对 T1→T2 的黄金差价。
    pub price_delta_gold: i64,
    /// T2→T3 相对 T1→T2 的木材差价。
    pub price_delta_lumber: i64,
    /// 每级升级时间（30 Hz 整数帧）。
    pub frames: u32,
}

/// 一次升级按目标等级确定性结算出的费用与时间（全整数）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TownHallCost {
    pub gold: i64,
    pub lumber: i64,
    pub frames: u32,
}

impl TownHallConstants {
    /// 按目标等级取费用 / 时间；T1 不是可升级目标，返回 `None`。
    /// T3 采用「基础费用 + 差价」而非绝对总价，避免把导出总价当扣款。
    pub fn cost(&self, target: TownHallTier) -> Option<TownHallCost> {
        let frames = self.frames.max(1);
        match target {
            TownHallTier::T1 => None,
            TownHallTier::T2 => Some(TownHallCost {
                gold: self.tier2_gold,
                lumber: self.tier2_lumber,
                frames,
            }),
            TownHallTier::T3 => Some(TownHallCost {
                gold: self.tier2_gold.saturating_add(self.price_delta_gold),
                lumber: self.tier2_lumber.saturating_add(self.price_delta_lumber),
                frames,
            }),
        }
    }

    /// 显式小端编码进定义哈希；任一常量变化都改变结果。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_i64(self.tier2_gold);
        hasher.write_i64(self.tier2_lumber);
        hasher.write_i64(self.price_delta_gold);
        hasher.write_i64(self.price_delta_lumber);
        hasher.write_u32(self.frames);
    }
}

/// 合成 / 原创的主城升级冻结常量 `[待核]`；数值不冒充经典规则、不来自任何资产。
pub const SYNTHETIC_TOWN_HALL_CONSTANTS: TownHallConstants = TownHallConstants {
    tier2_gold: 150,
    tier2_lumber: 100,
    price_delta_gold: 100,
    price_delta_lumber: 60,
    frames: 60,
};

impl Default for TownHallConstants {
    fn default() -> Self {
        SYNTHETIC_TOWN_HALL_CONSTANTS
    }
}

// ---------------------------------------------------------------------------
// 冻结内容定义
// ---------------------------------------------------------------------------

/// 效果种类；旧码 0–3 逐字节不动，P11 原语只追加唯一码。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum EffectKind {
    Damage,
    Heal,
    BuffStat,
    Stun,
    /// 持续 AOE / zone。
    Zone,
    /// 召唤实体。
    Summon,
    /// 光环受益集。
    AuraBeneficiaries,
    /// 被动 proc 订阅。
    PassiveProc,
    /// 变身 / 状态切换。
    Transform,
    /// 点目标。
    PointTarget,
    /// Buff 分类移除。
    RemoveBuffCategory,
    /// 链接弹射。
    ChainBounce,
    /// 随机消费。
    RandomDraw,
}

/// 效果定义：效果组合的基本单元。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EffectDef {
    pub kind: EffectKind,
    pub amount: i64,
    pub duration_frames: u32,
}

/// 单位定义（字段均为冻结的整数初值）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnitDef {
    pub hp: i64,
    pub mana: i64,
    pub speed_raw: i64,
    pub damage: i64,
    pub range_raw: i64,
    pub supply: i64,
    pub train_frames: u32,
}

/// 技能定义：模板参数 + 效果组合 + 注册行为 ID。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilityDef {
    pub mana_cost: i64,
    pub cooldown_frames: u32,
    pub cast_frames: u32,
    pub behavior: DefId,
    pub effects: Vec<EffectDef>,
}

/// 物品定义（属性修正）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemDef {
    pub hp_mod: i64,
    pub damage_mod: i64,
}

/// 升级 / 科技定义。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UpgradeDef {
    pub damage_mod: i64,
    pub hp_mod: i64,
    pub research_frames: u32,
}

/// 冻结定义集合：类别内 ID 唯一，创建后不提供任何可变 API。
///
/// 四类定义字段全部私有，外部只能读取；`hash` 在 [`FrozenDefinitions::new`]
/// 中按当前内容一次性计算，由于没有可变入口，缓存与内容始终一致。
///
/// P8.5a 追加三个私有冻结字段：`revive` 为复活常量段，`altar_defs` /
/// `hero_defs` 为定义层登记的祭坛 / 英雄类别 DefId 集合。
/// P8.5b 追加三个私有冻结字段：`town_hall` 为主城升级冻结常量段，
/// `town_hall_defs` 为定义层登记的主城类别 DefId 集合，
/// `tier_requirements` 为「二线单位 / 科技 DefId → 所需 tier」需求链。
/// P8.5c 追加四个私有冻结字段：`creep` 为野怪冻结常量段，
/// `camp_scripts` / `drop_tables` 为营地脚本与「表 → 互斥 set → 概率条目」
/// 分层掉落表，`neutral_categories` 为中立类别 / 阵营标记登记（Q7 / Q8）。
/// P8.5d 追加一个私有冻结字段 `item_shop`：商店货架清单 / 类别 / 价格 /
/// 回购价 / 库存上限 / 刷新间隔 / 解锁时间 / 回城施法帧 / 次数 / 拾取半径 /
/// 售卖比例 / 友方主城分类标记全为合成原创编译期整数常量 `[待核]`，
/// 经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部（Q6 / Q8）。
/// 全部编入 `compute_defs_hash`，改常量或改类别 / 脚本 / 概率条目 / 货架登记
/// 即改变 `content_hash()`（Q3 / Q8）。类别与解锁判定只查定义层数据
/// （数据驱动），MUST NOT 用建筑名字符串白名单或硬编码 DefId（R1.2 / R13.3）。
/// 注意：这些是**冻结定义层内容**，不是权威状态字段；野怪 / 营地 / 掉落 /
/// 随机码消费序列的运行态在 `CreepCampState`，物品 / 商店 / 回城的
/// 六格背包 / 库存 / 传送 / 消费序列运行态在 `ItemShopState`（D1 / D4）。
///
/// P9 追加两个私有冻结字段：`faction_mechanics` 为族特色机制 9 个
/// 合成原创编译期整数 `[待核]` 常量段，`regen_conditions` 为「`DefId`
/// → 回血条件定义层枚举」类别登记（R10.3 / 参考对照 R9）。两者经
/// `compute_defs_hash` 进入 `defs_hash` / `content_hash` 快照头部
/// （Q3' / D2）；这是**定义层内容变化**，与 Q2 新增的
/// `FactionMechanicsState` 顶层权威状态严格区分，MUST NOT 混入
/// `compute_state_hash()`。回血条件由枚举查表、MUST NOT 用族名白名单。
///
/// P11 追加一个私有冻结字段 `ability_system`：四席位技能清单、行为目录、
/// buff 分类与合成原创整数 `[待核]` 常量全部经 `compute_defs_hash` 进入
/// `defs_hash` / `content_hash` 快照头部；这是**定义层内容**，与
/// `WorldGameplayState.abilities` 的技能权威运行态严格区分，MUST NOT
/// 作为运行态字段重复进入状态哈希。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FrozenDefinitions {
    units: BTreeMap<DefId, UnitDef>,
    abilities: BTreeMap<DefId, AbilityDef>,
    items: BTreeMap<DefId, ItemDef>,
    upgrades: BTreeMap<DefId, UpgradeDef>,
    revive: ReviveConstants,
    altar_defs: BTreeSet<DefId>,
    hero_defs: BTreeSet<DefId>,
    town_hall: TownHallConstants,
    town_hall_defs: BTreeSet<DefId>,
    tier_requirements: BTreeMap<DefId, TownHallTier>,
    creep: CreepConstants,
    camp_scripts: BTreeMap<u32, CampScript>,
    drop_tables: BTreeMap<u32, DropTable>,
    neutral_categories: BTreeMap<DefId, NeutralCategory>,
    item_shop: ItemShopDefinitions,
    faction_mechanics: FactionMechanicsConstants,
    regen_conditions: BTreeMap<DefId, RegenCondition>,
    ability_system: AbilitySystemDefinitions,
    hash: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DefinitionError {
    DuplicateId(DefId),
}

impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "duplicate definition id: {id}"),
        }
    }
}

impl std::error::Error for DefinitionError {}

fn insert_unique<T>(
    map: &mut BTreeMap<DefId, T>,
    entries: Vec<(DefId, T)>,
) -> Result<(), DefinitionError> {
    for (id, value) in entries {
        if map.insert(id, value).is_some() {
            return Err(DefinitionError::DuplicateId(id));
        }
    }
    Ok(())
}

/// 效果种类的稳定字节编码。
pub fn effect_kind_code(kind: EffectKind) -> u8 {
    match kind {
        EffectKind::Damage => 0,
        EffectKind::Heal => 1,
        EffectKind::BuffStat => 2,
        EffectKind::Stun => 3,
        EffectKind::Zone => 4,
        EffectKind::Summon => 5,
        EffectKind::AuraBeneficiaries => 6,
        EffectKind::PassiveProc => 7,
        EffectKind::Transform => 8,
        EffectKind::PointTarget => 9,
        EffectKind::RemoveBuffCategory => 10,
        EffectKind::ChainBounce => 11,
        EffectKind::RandomDraw => 12,
    }
}

/// 由稳定字节编码解码效果种类；未知码显式拒绝，不静默回落。
pub fn decode_effect_kind(code: u8) -> Option<EffectKind> {
    Some(match code {
        0 => EffectKind::Damage,
        1 => EffectKind::Heal,
        2 => EffectKind::BuffStat,
        3 => EffectKind::Stun,
        4 => EffectKind::Zone,
        5 => EffectKind::Summon,
        6 => EffectKind::AuraBeneficiaries,
        7 => EffectKind::PassiveProc,
        8 => EffectKind::Transform,
        9 => EffectKind::PointTarget,
        10 => EffectKind::RemoveBuffCategory,
        11 => EffectKind::ChainBounce,
        12 => EffectKind::RandomDraw,
        _ => return None,
    })
}

fn hash_effects(hasher: &mut StateHasher, effects: &[EffectDef]) {
    hasher.write_u64(effects.len() as u64);
    for effect in effects {
        hasher.write_u8(effect_kind_code(effect.kind));
        hasher.write_i64(effect.amount);
        hasher.write_u32(effect.duration_frames);
    }
}

fn compute_defs_hash(defs: &FrozenDefinitions) -> u64 {
    let mut hasher = StateHasher::new();
    hasher.write_u64(CONTENT_HASH_SCHEMA);
    hasher.write_u64(defs.units.len() as u64);
    for (&id, def) in &defs.units {
        hasher.write_u32(id);
        hasher.write_i64(def.hp);
        hasher.write_i64(def.mana);
        hasher.write_i64(def.speed_raw);
        hasher.write_i64(def.damage);
        hasher.write_i64(def.range_raw);
        hasher.write_i64(def.supply);
        hasher.write_u32(def.train_frames);
    }
    hasher.write_u64(defs.abilities.len() as u64);
    for (&id, def) in &defs.abilities {
        hasher.write_u32(id);
        hasher.write_i64(def.mana_cost);
        hasher.write_u32(def.cooldown_frames);
        hasher.write_u32(def.cast_frames);
        hasher.write_u32(def.behavior);
        hash_effects(&mut hasher, &def.effects);
    }
    hasher.write_u64(defs.items.len() as u64);
    for (&id, def) in &defs.items {
        hasher.write_u32(id);
        hasher.write_i64(def.hp_mod);
        hasher.write_i64(def.damage_mod);
    }
    hasher.write_u64(defs.upgrades.len() as u64);
    for (&id, def) in &defs.upgrades {
        hasher.write_u32(id);
        hasher.write_i64(def.damage_mod);
        hasher.write_i64(def.hp_mod);
        hasher.write_u32(def.research_frames);
    }
    // P8.5a：复活冻结常量段编入定义哈希，改常量即改变 content_hash()。
    defs.revive.hash(&mut hasher);
    // P8.5a：祭坛 / 英雄类别登记按稳定 DefId 顺序编入（定义层数据驱动）。
    hasher.write_u64(defs.altar_defs.len() as u64);
    for &id in &defs.altar_defs {
        hasher.write_u32(id);
    }
    hasher.write_u64(defs.hero_defs.len() as u64);
    for &id in &defs.hero_defs {
        hasher.write_u32(id);
    }
    // P8.5b：主城升级冻结常量段、主城类别登记、tier 需求链编入定义哈希。
    // 任一段变化都会改变 content_hash() 并进入快照头部（D2 / Q3）。
    defs.town_hall.hash(&mut hasher);
    hasher.write_u64(defs.town_hall_defs.len() as u64);
    for &id in &defs.town_hall_defs {
        hasher.write_u32(id);
    }
    hasher.write_u64(defs.tier_requirements.len() as u64);
    for (&id, &tier) in &defs.tier_requirements {
        hasher.write_u32(id);
        hasher.write_u8(tier.code());
    }
    // P8.5c：野怪冻结常量段、营地脚本、掉落表分层、中立类别登记编入
    // 定义哈希。任一段（常量 / 脚本 / set / 概率条目 / 类别）变化都会
    // 改变 content_hash() 并进入快照头部（Q8 / tasks 2.2 / D4）。
    defs.creep.hash(&mut hasher);
    hasher.write_u64(defs.camp_scripts.len() as u64);
    for (&script_id, script) in &defs.camp_scripts {
        hasher.write_u32(script_id);
        script.hash(&mut hasher);
    }
    hasher.write_u64(defs.drop_tables.len() as u64);
    for (&table_id, table) in &defs.drop_tables {
        hasher.write_u32(table_id);
        table.hash(&mut hasher);
    }
    hasher.write_u64(defs.neutral_categories.len() as u64);
    for (&def_id, &category) in &defs.neutral_categories {
        hasher.write_u32(def_id);
        hasher.write_u8(category.code());
    }
    // P8.5d：商店 / 道具 / 回城冻结定义段编入定义哈希。任一段
    // （常量 / 货架 / 商店表 / 类别 / 冷却组 / 效果 / 回城卷轴登记）变化
    // 都会改变 content_hash() 并进入快照头部（Q8 / tasks 1.2 / D4）。
    defs.item_shop.hash(&mut hasher);
    // P9：族特色机制冻结常量段 + 回血条件定义层类别登记编入定义哈希。
    // 9 个整数逐字段小端编码，改任一常量 / 改任一 `DefId` 的回血条件
    // 都会改变 content_hash() 并进入快照头部（Q3' / D2 / tasks 2.3 / 2.4）。
    // 定义层内容变化与 Q2 新增权威状态严格区分，MUST NOT 进 compute_state_hash。
    defs.faction_mechanics.hash(&mut hasher);
    hash_regen_conditions(&mut hasher, &defs.regen_conditions);
    // P11：技能清单 / 行为目录 / buff 分类 / 冻结常量进入定义哈希。
    // 改任一段都会改变 content_hash() 与快照头部；这不携带运行态。
    defs.ability_system.hash(&mut hasher);
    hasher.finish()
}

impl FrozenDefinitions {
    /// 校验并冻结四类定义；类别内重复 ID 直接拒绝。
    pub fn new(
        units: Vec<(DefId, UnitDef)>,
        abilities: Vec<(DefId, AbilityDef)>,
        items: Vec<(DefId, ItemDef)>,
        upgrades: Vec<(DefId, UpgradeDef)>,
    ) -> Result<Self, DefinitionError> {
        let mut defs = Self {
            units: BTreeMap::new(),
            abilities: BTreeMap::new(),
            items: BTreeMap::new(),
            upgrades: BTreeMap::new(),
            revive: ReviveConstants::default(),
            altar_defs: BTreeSet::new(),
            hero_defs: BTreeSet::new(),
            town_hall: TownHallConstants::default(),
            town_hall_defs: BTreeSet::new(),
            tier_requirements: BTreeMap::new(),
            creep: CreepConstants::default(),
            camp_scripts: BTreeMap::new(),
            drop_tables: BTreeMap::new(),
            neutral_categories: BTreeMap::new(),
            item_shop: ItemShopDefinitions::default(),
            // P9：族特色机制冻结常量段由默认合成常量初始化，让 client
            // `synthetic_definitions()` 现有 `FrozenDefinitions::new(...)`
            // 调用点零改动即可编译（Q3' / leader 跨 crate 契约）。
            // `regen_conditions` 默认空表，类别登记由可选 builder 注入。
            faction_mechanics: SYNTHETIC_FACTION_MECHANICS,
            regen_conditions: BTreeMap::new(),
            // P11：默认注入编译期合成原创技能定义，既有四参数签名不变。
            ability_system: AbilitySystemDefinitions::default(),
            hash: 0,
        };
        insert_unique(&mut defs.units, units)?;
        insert_unique(&mut defs.abilities, abilities)?;
        insert_unique(&mut defs.items, items)?;
        insert_unique(&mut defs.upgrades, upgrades)?;
        defs.hash = compute_defs_hash(&defs);
        Ok(defs)
    }

    /// 空定义集，供未注入内容定义的现有入口使用。
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), Vec::new(), Vec::new())
            .expect("empty definitions are always valid")
    }

    /// 纯新增构造路径：注入复活冻结常量与祭坛 / 英雄类别登记，重算内容哈希。
    ///
    /// 不改动 [`FrozenDefinitions::new`] 的四参数签名，既有调用方零改动；
    /// 需要祭坛复活的装配层可链式调用本方法。类别集合是定义层数据，
    /// 判定与建筑名 / DefId 数值白名单无关（R1.2 / R13.3 / D2 / Q3）。
    pub fn with_hero_revive(
        mut self,
        revive: ReviveConstants,
        altar_defs: BTreeSet<DefId>,
        hero_defs: BTreeSet<DefId>,
    ) -> Self {
        self.revive = revive;
        self.altar_defs = altar_defs;
        self.hero_defs = hero_defs;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入主城升级冻结常量、主城类别集合与 tier 需求链，
    /// 重算内容哈希。既有四参数签名不变，需要主城升级的装配层链式调用。
    ///
    /// 类别与需求链是定义层数据（数据驱动），判定与建筑名 / DefId 数值
    /// 白名单无关（R1.2 / R13.3 / D2 / Q3）；改任一常量 / 类别 / 需求链
    /// 即改变 `content_hash()` 并进入快照头部。
    pub fn with_town_hall_upgrade(
        mut self,
        constants: TownHallConstants,
        town_hall_defs: BTreeSet<DefId>,
        tier_requirements: BTreeMap<DefId, TownHallTier>,
    ) -> Self {
        self.town_hall = constants;
        self.town_hall_defs = town_hall_defs;
        self.tier_requirements = tier_requirements;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入 P8.5c 野怪 / 营地 / 掉落冻结常量段与中立类别
    /// 登记，重算内容哈希。既有四参数签名不变，需要野怪营地的装配层链式
    /// 调用；改任一常量 / 脚本 / 概率条目 / 类别即改变 `content_hash()`。
    ///
    /// 判定只查定义层类别与 set 数据（数据驱动），MUST NOT 用建筑名字符串
    /// 白名单或硬编码 DefId；以非经典命名的 DefId 登记后仍可判定
    /// （R1.2 / R13.3 / D4 / tasks 2.3）。
    pub fn with_creep_camps(
        mut self,
        constants: CreepConstants,
        camp_scripts: BTreeMap<u32, CampScript>,
        drop_tables: BTreeMap<u32, DropTable>,
        neutral_categories: BTreeMap<DefId, NeutralCategory>,
    ) -> Self {
        self.creep = constants;
        self.camp_scripts = camp_scripts;
        self.drop_tables = drop_tables;
        self.neutral_categories = neutral_categories;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入 P8.5d 商店 / 道具 / 回城冻结定义段，重算内容哈希。
    ///
    /// 既有四参数签名不变，需要物品 / 商店 / 回城卷轴的装配层链式调用。
    /// 货架清单 / 类别 / 价格 / 回购价 / 库存上限 / 刷新间隔 / 解锁时间 /
    /// 回城施法帧 / 次数 / 拾取半径 / 售卖比例 / 友方主城分类标记全为
    /// 冻结定义层合成原创整数 `[待核]`；改任一段即改变 `content_hash()`
    /// 并进入快照头部（Q6 / Q8 / tasks 1.2）。判定只查定义层类别登记
    /// （数据驱动），MUST NOT 用建筑名白名单 / 硬编码 DefId / rawcode。
    pub fn with_item_shop(mut self, definitions: ItemShopDefinitions) -> Self {
        self.item_shop = definitions;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入 P9 族特色机制冻结常量段，重算内容哈希。
    ///
    /// 既有四参数 [`FrozenDefinitions::new`] 签名不变，未显式注入的装配层
    /// 自动使用 `SYNTHETIC_FACTION_MECHANICS` 默认值（client 零改动编译）；
    /// 内容敏感性测试可链式调用本方法注入不同常量，验证 `content_hash()`
    /// 如实变化（Q3' / tasks 2.3 / 2.4 / 4.3 反向验证）。
    pub fn with_faction_mechanics(mut self, constants: FactionMechanicsConstants) -> Self {
        self.faction_mechanics = constants;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入「`DefId` → 回血条件定义层枚举」类别登记，
    /// 重算内容哈希。类别由定义层数据驱动（数据驱动），MUST NOT 用
    /// 族名白名单（R1.2 / R10.3 / 参考对照 R9）。
    pub fn with_regen_conditions(
        mut self,
        regen_conditions: BTreeMap<DefId, RegenCondition>,
    ) -> Self {
        self.regen_conditions = regen_conditions;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 纯新增构造路径：注入 P11 技能系统冻结定义并重算内容哈希。
    ///
    /// 既有四参数 [`FrozenDefinitions::new`] 签名不变；未显式注入的装配层
    /// 使用默认合成原创清单 / 行为目录 / 常量。改任一段即改变
    /// `content_hash()` 与快照头部，MUST NOT 改写技能权威运行态。
    pub fn with_ability_system(mut self, definitions: AbilitySystemDefinitions) -> Self {
        self.ability_system = definitions;
        self.hash = compute_defs_hash(&self);
        self
    }

    /// 技能系统冻结定义（只读，P11 / D1 / D3）。
    pub fn ability_system(&self) -> &AbilitySystemDefinitions {
        &self.ability_system
    }

    /// 族特色机制冻结常量段（只读，Q3'）。
    pub fn faction_mechanics(&self) -> &FactionMechanicsConstants {
        &self.faction_mechanics
    }

    /// 冻结定义层回血条件类别登记（只读，稳定 `DefId` 顺序，R10.3）。
    pub fn regen_conditions(&self) -> &BTreeMap<DefId, RegenCondition> {
        &self.regen_conditions
    }

    /// 按 `DefId` 查回血条件；未登记返回 `RegenCondition::None`。
    /// 逐 tick 查表用，非族名硬编码（R10.3 / 参考对照 R9）。
    pub fn regen_condition(&self, def_id: DefId) -> RegenCondition {
        self.regen_conditions
            .get(&def_id)
            .copied()
            .unwrap_or_default()
    }

    /// 物品 / 商店 / 回城冻结定义段（只读，Q6 / Q8）。
    pub fn item_shop(&self) -> &ItemShopDefinitions {
        &self.item_shop
    }

    /// 野怪冻结常量（只读，Q8）。
    pub fn creep_constants(&self) -> &CreepConstants {
        &self.creep
    }

    /// 冻结营地脚本登记（只读，稳定脚本 id 顺序）。
    pub fn camp_scripts(&self) -> &BTreeMap<u32, CampScript> {
        &self.camp_scripts
    }

    /// 按冻结脚本标识查营地脚本（Q5 `SeedCreepCamp` 命令用）。
    pub fn camp_script(&self, script_id: u32) -> Option<&CampScript> {
        self.camp_scripts.get(&script_id)
    }

    /// 冻结掉落表（只读，稳定表 id 顺序）。
    pub fn drop_tables(&self) -> &BTreeMap<u32, DropTable> {
        &self.drop_tables
    }

    /// 按表 id 查冻结掉落表（表 → 互斥 set → 概率条目，R7.2 / R8.6）。
    pub fn drop_table(&self, table_id: u32) -> Option<&DropTable> {
        self.drop_tables.get(&table_id)
    }

    /// 定义层中立类别登记（只读，稳定 DefId 顺序，Q7）。
    pub fn neutral_categories(&self) -> &BTreeMap<DefId, NeutralCategory> {
        &self.neutral_categories
    }

    /// 该 DefId 的中立类别 / 阵营标记；`None` 表示玩家阵营普通实体。
    pub fn neutral_category(&self, def_id: DefId) -> Option<NeutralCategory> {
        self.neutral_categories.get(&def_id).copied()
    }

    /// 定义层类别驱动：该 DefId 是否被登记为中立敌对野怪（R8.1）。
    pub fn is_creep(&self, def_id: DefId) -> bool {
        self.neutral_category(def_id) == Some(NeutralCategory::CreepHostile)
    }

    /// 定义层类别驱动：该 DefId 是否被登记为无主地面物品（R1.1）。
    pub fn is_ground_item(&self, def_id: DefId) -> bool {
        self.neutral_category(def_id) == Some(NeutralCategory::GroundItemOwnerless)
    }

    /// 默认野怪冻结常量（供未显式注入的装配层使用，合成原创 `[待核]`）。
    pub fn synthetic_creep_constants() -> CreepConstants {
        SYNTHETIC_CREEP_CONSTANTS
    }

    /// 定义层类别驱动：该 DefId 是否被登记为主城（MUST NOT 用名字白名单）。
    pub fn is_town_hall(&self, def_id: DefId) -> bool {
        self.town_hall_defs.contains(&def_id)
    }

    /// 定义层需求链：该二线单位 / 科技 DefId 解锁所需的最低 tier；
    /// `None` 表示无 tier 门槛（T1 即可）。按 tier 取需求（Warsmash 语义）。
    pub fn tier_requirement(&self, def_id: DefId) -> Option<TownHallTier> {
        self.tier_requirements.get(&def_id).copied()
    }

    /// 主城升级冻结常量（只读）。
    pub fn town_hall_constants(&self) -> &TownHallConstants {
        &self.town_hall
    }

    /// 已登记的主城 DefId 集合（只读，稳定顺序）。
    pub fn town_hall_defs(&self) -> &BTreeSet<DefId> {
        &self.town_hall_defs
    }

    /// tier 需求链（只读，稳定顺序）。
    pub fn tier_requirements(&self) -> &BTreeMap<DefId, TownHallTier> {
        &self.tier_requirements
    }

    /// 定义层类别驱动：该 DefId 是否被登记为祭坛，MUST NOT 用名字白名单。
    pub fn is_altar(&self, def_id: DefId) -> bool {
        self.altar_defs.contains(&def_id)
    }

    /// 定义层类别驱动：该 DefId 是否被登记为英雄。
    pub fn is_hero(&self, def_id: DefId) -> bool {
        self.hero_defs.contains(&def_id)
    }

    /// 已登记的祭坛 DefId 集合（只读，稳定顺序）。
    pub fn altar_defs(&self) -> &BTreeSet<DefId> {
        &self.altar_defs
    }

    /// 已登记的英雄 DefId 集合（只读，稳定顺序）。
    pub fn hero_defs(&self) -> &BTreeSet<DefId> {
        &self.hero_defs
    }

    /// 复活冻结常量（只读）。
    pub fn revive_constants(&self) -> &ReviveConstants {
        &self.revive
    }

    /// 按英雄等级确定性计算复活费用（读冻结常量，不含浮点）。
    pub fn revive_cost(&self, level: u32) -> ReviveCost {
        self.revive.cost(level)
    }

    /// 冻结定义的内容哈希；该值是对局配置的一部分，进入快照头与状态哈希。
    pub fn content_hash(&self) -> u64 {
        self.hash
    }

    /// 只读视图：运行期不能替换、增删或改动任何定义。
    pub fn units(&self) -> &BTreeMap<DefId, UnitDef> {
        &self.units
    }

    pub fn abilities(&self) -> &BTreeMap<DefId, AbilityDef> {
        &self.abilities
    }

    pub fn items(&self) -> &BTreeMap<DefId, ItemDef> {
        &self.items
    }

    pub fn upgrades(&self) -> &BTreeMap<DefId, UpgradeDef> {
        &self.upgrades
    }
}

impl Default for FrozenDefinitions {
    fn default() -> Self {
        Self::empty()
    }
}

// ---------------------------------------------------------------------------
// 统一结算服务
// ---------------------------------------------------------------------------

/// 效果执行结果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectOutcome {
    Applied,
    NoEffect,
}

/// 可结算字段的借用视图：所有改血 / 改法力 / Buff / 属性修正经此进入。
///
/// 仅 `pub(crate)`：下游 crate 既不能构造 `Vitals`，也不能拿到内部可变引用，
/// 从而无法绕过 [`Settlement`] 直接改写权威状态。
pub(crate) struct Vitals<'a> {
    pub(crate) hp: &'a mut i64,
    pub(crate) max_hp: i64,
    pub(crate) alive: &'a mut bool,
    pub(crate) mana: &'a mut i64,
    pub(crate) buffs: &'a mut Vec<BuffInstance>,
    pub(crate) stat_mods: &'a mut StatMods,
}

/// 单一结算所有者：技能、物品、攻击都不得绕过它直接改权威字段。
/// 仅 `pub(crate)`：下游 crate 无法构造 `Vitals`、也无法调用任何结算方法。
pub(crate) struct Settlement;

impl Settlement {
    /// 费用提交事务：余额不足、费用为负或已死亡返回 false，不产生任何改动。
    pub(crate) fn commit_cost(vitals: &mut Vitals<'_>, cost: i64) -> bool {
        if !*vitals.alive || cost < 0 || *vitals.mana < cost {
            return false;
        }
        *vitals.mana -= cost;
        true
    }

    /// 伤害结算：目标已死亡或非正伤害返回 `NoEffect`，死亡时生命归零。
    pub(crate) fn damage(vitals: &mut Vitals<'_>, amount: i64) -> EffectOutcome {
        if !*vitals.alive || amount <= 0 {
            return EffectOutcome::NoEffect;
        }
        let remaining = vitals.hp.saturating_sub(amount);
        if remaining <= 0 {
            *vitals.hp = 0;
            *vitals.alive = false;
        } else {
            *vitals.hp = remaining;
        }
        EffectOutcome::Applied
    }

    /// 治疗结算：不超过生命上限，目标已死亡或未存活返回 `NoEffect`。
    pub(crate) fn heal(vitals: &mut Vitals<'_>, amount: i64) -> EffectOutcome {
        if !*vitals.alive || amount <= 0 {
            return EffectOutcome::NoEffect;
        }
        let cap = vitals.max_hp.saturating_add(vitals.stat_mods.max_hp);
        *vitals.hp = (*vitals.hp).saturating_add(amount).min(cap);
        EffectOutcome::Applied
    }

    /// 施加 Buff 实例（含眩晕），目标已死亡返回 `NoEffect`。
    pub(crate) fn buff(vitals: &mut Vitals<'_>, effect: &EffectDef) -> EffectOutcome {
        if !*vitals.alive {
            return EffectOutcome::NoEffect;
        }
        vitals.buffs.push(BuffInstance {
            kind: effect.kind,
            amount: effect.amount,
            remaining: effect.duration_frames,
        });
        EffectOutcome::Applied
    }

    /// 属性修正（Buff 效果链）：同时登记一条短期 Buff 记录。
    pub(crate) fn stat_mod(vitals: &mut Vitals<'_>, effect: &EffectDef) -> EffectOutcome {
        if !*vitals.alive {
            return EffectOutcome::NoEffect;
        }
        vitals.stat_mods.damage = vitals.stat_mods.damage.saturating_add(effect.amount);
        vitals.buffs.push(BuffInstance {
            kind: effect.kind,
            amount: effect.amount,
            remaining: effect.duration_frames,
        });
        EffectOutcome::Applied
    }

    /// 物品 / 科技的属性修正：经结算服务同时更新攻击与生命上限修正。
    pub(crate) fn apply_stat_mod(
        vitals: &mut Vitals<'_>,
        damage_mod: i64,
        hp_mod: i64,
    ) -> EffectOutcome {
        if !*vitals.alive {
            return EffectOutcome::NoEffect;
        }
        vitals.stat_mods.damage = vitals.stat_mods.damage.saturating_add(damage_mod);
        vitals.stat_mods.max_hp = vitals.stat_mods.max_hp.saturating_add(hp_mod);
        EffectOutcome::Applied
    }

    /// 目标过滤：按稳定顺序保留存活候选。
    pub(crate) fn filter_targets(
        candidates: &[EntityId],
        is_alive: impl Fn(EntityId) -> bool,
    ) -> Vec<EntityId> {
        candidates
            .iter()
            .copied()
            .filter(|id| is_alive(*id))
            .collect()
    }

    /// Buff 分类移除：仍由单一结算所有者改写 Buff 列表，不开放可变引用。
    pub(crate) fn remove_buff_category(
        vitals: &mut Vitals<'_>,
        category: BuffCategory,
    ) -> EffectOutcome {
        if category == BuffCategory::None || !*vitals.alive {
            return EffectOutcome::NoEffect;
        }
        let before = vitals.buffs.len();
        vitals
            .buffs
            .retain(|buff| BuffCategory::from_effect_kind(buff.kind) != category);
        if vitals.buffs.len() != before {
            EffectOutcome::Applied
        } else {
            EffectOutcome::NoEffect
        }
    }
}

// ---------------------------------------------------------------------------
// 可保存运行态
// ---------------------------------------------------------------------------

/// Buff 实例。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuffInstance {
    pub kind: EffectKind,
    pub amount: i64,
    pub remaining: u32,
}

/// 属性修正。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StatMods {
    pub damage: i64,
    pub max_hp: i64,
}

/// 施法 / 引导运行态。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CastRuntime {
    pub ability: DefId,
    pub target: EntityId,
    pub remaining_frames: u32,
    pub channel: bool,
}

/// 英雄运行态。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HeroState {
    pub level: u32,
    pub xp: i64,
}

// ---------------------------------------------------------------------------
// 英雄复活权威状态（P8.5a / D1 / D2 / D4）
// ---------------------------------------------------------------------------

/// 英雄死亡登记：稳定 `EntityId` 键对应一条，含整数帧号与待恢复状态快照。
///
/// `death_frame` 是阵亡发生的整数逻辑帧号（`Frame::value()`），MUST NOT 使用
/// 墙钟 / 渲染帧率；`gameplay` 是阵亡瞬间完整的 [`UnitGameplayState`] 快照，
/// 逐项承载 `hero.level` / `hero.xp`、技能等级 / 冷却登记（`cooldowns`）、
/// 法力与 Buff，复活时回填到同一实体。P8.5d 起六格定长实例背包由
/// `ItemShopState.death_bags` 按实例 `EntityId` 身份恢复（R6.3），
/// MUST NOT 回退为 DefId 序列。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HeroDeathRecord {
    pub death_frame: u64,
    pub owner: u8,
    pub gameplay: UnitGameplayState,
    pub max_hp: i64,
}

/// 祭坛复活队列项：目标英雄 `EntityId` + 剩余帧数 + 本次费用。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReviveQueueItem {
    pub hero: EntityId,
    pub altar: EntityId,
    pub altar_def: DefId,
    pub owner: u8,
    pub remaining_frames: u32,
    pub total_frames: u32,
    pub gold: i64,
    pub lumber: i64,
}

/// 冻结扣费登记：事务性退款的精确逆运算依据，与队列效果执行链分离。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReviveCharge {
    pub owner: u8,
    pub gold: i64,
    pub lumber: i64,
}

/// 三组英雄复活权威状态的唯一容器，全部按稳定 `EntityId` 顺序的 `BTreeMap`。
///
/// 死亡登记、复活队列项、费用登记三组 MUST 同时进入 FNV-1a 状态哈希、
/// 由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()` 差异字段集合
/// （ADR 0002 §6-8 / `CODEBUDDY.md §5`）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HeroReviveState {
    pub deaths: BTreeMap<EntityId, HeroDeathRecord>,
    pub queue: BTreeMap<EntityId, ReviveQueueItem>,
    pub charges: BTreeMap<EntityId, ReviveCharge>,
}

/// 稳定字节编码 `UnitGameplayState`（委托其既有 `hash`，避免重复定义）。
fn hash_hero_gameplay(hasher: &mut StateHasher, gameplay: &UnitGameplayState) {
    gameplay.hash(hasher);
}

impl HeroReviveState {
    /// 三组状态显式小端编码进 FNV-1a；按稳定 `EntityId` 顺序遍历。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.deaths.len() as u64);
        for (&id, record) in &self.deaths {
            hasher.write_u64(id.value());
            hasher.write_u64(record.death_frame);
            hasher.write_u8(record.owner);
            hasher.write_i64(record.max_hp);
            hash_hero_gameplay(hasher, &record.gameplay);
        }
        hasher.write_u64(self.queue.len() as u64);
        for (&id, item) in &self.queue {
            hasher.write_u64(id.value());
            hasher.write_u64(item.hero.value());
            hasher.write_u64(item.altar.value());
            hasher.write_u32(item.altar_def);
            hasher.write_u8(item.owner);
            hasher.write_u32(item.remaining_frames);
            hasher.write_u32(item.total_frames);
            hasher.write_i64(item.gold);
            hasher.write_i64(item.lumber);
        }
        hasher.write_u64(self.charges.len() as u64);
        for (&id, charge) in &self.charges {
            hasher.write_u64(id.value());
            hasher.write_u8(charge.owner);
            hasher.write_i64(charge.gold);
            hasher.write_i64(charge.lumber);
        }
    }

    /// 按同一稳定顺序返回首个差异字段，便于 desync 诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.deaths != other.deaths {
            return Some("revive_deaths");
        }
        if self.queue != other.queue {
            return Some("revive_queue");
        }
        if self.charges != other.charges {
            return Some("revive_charges");
        }
        None
    }
}

// ---------------------------------------------------------------------------
// 主城升级权威状态（P8.5b / D1 / D4 / Q1）
// ---------------------------------------------------------------------------

/// 主城升级队列项：目标主城实体 + 来源 / 目标等级 + 整数帧剩余时间 + 本次费用。
///
/// `remaining_frames` 是 30 Hz 整数帧剩余量，MUST NOT 使用墙钟 / 渲染帧率 /
/// `f64`（ADR 0002 §2-3）；按稳定 `EntityId` 顺序登记在 `BTreeMap`。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TownHallUpgradeItem {
    pub town_hall: EntityId,
    pub town_hall_def: DefId,
    pub owner: u8,
    pub from_tier: TownHallTier,
    pub target_tier: TownHallTier,
    pub remaining_frames: u32,
    pub total_frames: u32,
    pub gold: i64,
    pub lumber: i64,
}

/// 升级扣费登记：事务性退款的精确逆运算依据，与队列效果执行链分离。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TownHallUpgradeCharge {
    pub owner: u8,
    pub gold: i64,
    pub lumber: i64,
}

/// 三组主城升级权威状态的唯一容器，全部按稳定 `EntityId` 顺序的 `BTreeMap`。
///
/// `tiers` 是离散等级 T1 / T2 / T3 的顶层权威状态（独立字段，MUST NOT 复用
/// `PlayerEconomy.upgrades` 布尔科技旗标，C5）；`queue` 是升级进度；
/// `charges` 是冻结扣费登记。三组 MUST 同时进入 FNV-1a 状态哈希、由
/// `SNAPSHOT_VERSION` 定义的快照与 `first_difference()` 差异字段集合
/// （ADR 0002 §6-8 / `CODEBUDDY.md §5`）。
///
/// 注意：tier **需求链**属定义层冻结常量（进 `defs_hash` / `content_hash`），
/// 与本结构的 tier **权威状态**严格区分（D1 / D2）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TownHallUpgradeState {
    pub tiers: BTreeMap<EntityId, TownHallTier>,
    pub queue: BTreeMap<EntityId, TownHallUpgradeItem>,
    pub charges: BTreeMap<EntityId, TownHallUpgradeCharge>,
}

impl TownHallUpgradeState {
    /// 三组状态显式小端编码进 FNV-1a；按稳定 `EntityId` 顺序遍历。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.tiers.len() as u64);
        for (&id, &tier) in &self.tiers {
            hasher.write_u64(id.value());
            hasher.write_u8(tier.code());
        }
        hasher.write_u64(self.queue.len() as u64);
        for (&id, item) in &self.queue {
            hasher.write_u64(id.value());
            hasher.write_u64(item.town_hall.value());
            hasher.write_u32(item.town_hall_def);
            hasher.write_u8(item.owner);
            hasher.write_u8(item.from_tier.code());
            hasher.write_u8(item.target_tier.code());
            hasher.write_u32(item.remaining_frames);
            hasher.write_u32(item.total_frames);
            hasher.write_i64(item.gold);
            hasher.write_i64(item.lumber);
        }
        hasher.write_u64(self.charges.len() as u64);
        for (&id, charge) in &self.charges {
            hasher.write_u64(id.value());
            hasher.write_u8(charge.owner);
            hasher.write_i64(charge.gold);
            hasher.write_i64(charge.lumber);
        }
    }

    /// 按同一稳定顺序返回首个差异字段，便于 desync 诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.tiers != other.tiers {
            return Some("town_hall_tiers");
        }
        if self.queue != other.queue {
            return Some("town_hall_queue");
        }
        if self.charges != other.charges {
            return Some("town_hall_charges");
        }
        None
    }
}

/// 生产 / 建造队列项。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProductionItem {
    pub unit_def: DefId,
    pub remaining_frames: u32,
    pub is_building: bool,
}

/// 单个单位的玩法运行态（全部整数，可进入哈希 / 快照 / 差异诊断）。
///
/// P8.5d：P5 遗留的 `bag: Vec<DefId>` DefId 序列已收敛为顶层权威状态
/// `ItemShopState.bags` 的六格定长实例背包（每格挂稳定 `EntityId` 指向
/// `CreepCampState.item_instances`），MUST NOT 回退为 DefId 序列（Q7 / R6.2）。
/// 复活恢复时 `HeroDeathRecord.gameplay` 只负责法力 / Buff / 冷却 / 英雄，
/// 六格实例背包由 `ItemShopState.death_bags` 按实例身份恢复（R6.3）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UnitGameplayState {
    pub mana: i64,
    pub max_mana: i64,
    pub buffs: Vec<BuffInstance>,
    pub cast: Option<CastRuntime>,
    pub cooldowns: BTreeMap<DefId, u32>,
    pub hero: Option<HeroState>,
    pub production: Vec<ProductionItem>,
    pub stat_mods: StatMods,
    pub gathered: i64,
}

impl UnitGameplayState {
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_i64(self.mana);
        hasher.write_i64(self.max_mana);
        hasher.write_u64(self.buffs.len() as u64);
        for buff in &self.buffs {
            hasher.write_u8(effect_kind_code(buff.kind));
            hasher.write_i64(buff.amount);
            hasher.write_u32(buff.remaining);
        }
        match self.cast {
            Some(cast) => {
                hasher.write_u8(1);
                hasher.write_u32(cast.ability);
                hasher.write_u64(cast.target.value());
                hasher.write_u32(cast.remaining_frames);
                hasher.write_u8(u8::from(cast.channel));
            }
            None => hasher.write_u8(0),
        }
        hasher.write_u64(self.cooldowns.len() as u64);
        for (&ability, &remaining) in &self.cooldowns {
            hasher.write_u32(ability);
            hasher.write_u32(remaining);
        }
        match self.hero {
            Some(hero) => {
                hasher.write_u8(1);
                hasher.write_u32(hero.level);
                hasher.write_i64(hero.xp);
            }
            None => hasher.write_u8(0),
        }
        hasher.write_u64(self.production.len() as u64);
        for item in &self.production {
            hasher.write_u32(item.unit_def);
            hasher.write_u32(item.remaining_frames);
            hasher.write_u8(u8::from(item.is_building));
        }
        hasher.write_i64(self.stat_mods.damage);
        hasher.write_i64(self.stat_mods.max_hp);
        hasher.write_i64(self.gathered);
    }
}

/// 玩家经济：库存 / 人口 / 采集 / 科技进度 / 存活标记。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlayerEconomy {
    pub gold: i64,
    pub lumber: i64,
    pub supply_used: i64,
    pub supply_cap: i64,
    pub produced: i64,
    pub research: Option<(DefId, u32)>,
    pub upgrades: BTreeMap<DefId, bool>,
    pub active: bool,
}

impl Default for PlayerEconomy {
    fn default() -> Self {
        Self {
            gold: 0,
            lumber: 0,
            supply_used: 0,
            supply_cap: 0,
            produced: 0,
            research: None,
            upgrades: BTreeMap::new(),
            active: true,
        }
    }
}

/// 胜负状态：在既定帧阶段只结算一次。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct VictoryState {
    pub resolved: bool,
    pub winner: Option<u8>,
    pub resolved_frame: u64,
}

/// AI 状态：确定性重算，内部命令不注入重放。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AiState {
    pub enabled: bool,
    pub next_decision_frame: u64,
    pub decision_period: u32,
    pub pending_tasks: BTreeMap<u32, u32>,
}

/// 光环实例。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AuraState {
    pub source: EntityId,
    pub ability: DefId,
    pub remaining: u32,
}

/// 波次运行态。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaveState {
    pub period: u32,
    pub remaining: u32,
    pub count: u32,
}

/// 注册行为：按行为 ID 决定特殊机制，只存数据不存闭包。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BehaviorSpec {
    pub id: DefId,
    pub extra_effect: EffectDef,
}

// ---------------------------------------------------------------------------
// 玩法命令（与网络 `Command` 分离，避免改动 protobuf 穷尽匹配）
// ---------------------------------------------------------------------------

/// 玩法命令种类。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameplayCommandKind {
    Cast {
        caster: EntityId,
        ability: DefId,
        target: EntityId,
    },
    Train {
        producer: EntityId,
        unit_def: DefId,
        is_building: bool,
    },
    CancelTrain {
        producer: EntityId,
        index: u32,
    },
    Research {
        upgrade: DefId,
    },
    BuyItem {
        unit: EntityId,
        item: DefId,
    },
    Gather {
        unit: EntityId,
    },
    RegisterBehavior {
        id: DefId,
        effect: EffectDef,
    },
    SpawnWave {
        id: u32,
        period: u32,
    },
    SetAi {
        enabled: bool,
    },
    Leave {
        player: u8,
    },
    /// 提交祭坛复活：指定阵亡英雄、本族祭坛实体与祭坛定义 ID。
    ReviveHero {
        hero: EntityId,
        altar: EntityId,
        altar_def: DefId,
    },
    /// 取消某英雄的复活队列项并事务性退款。
    CancelRevive {
        hero: EntityId,
    },
    /// 主城升级 / 取消升级（P8.5b / Q2 / R-2）。
    ///
    /// 单命令参数 `cancel` 区分两种动作：`false` 发起向 `target_tier`
    /// 的升级（须为当前 tier 的下一级），`true` 取消该主城进行中的升级
    /// 并事务性退款（此时 `target_tier` 被忽略）。这是 R-2 授权的
    /// 「复用单命令参数表达」，MUST NOT 新增第二个 `CancelUpgradeTownHall`
    /// 变体，以维持「1 个变体 / 码 13 / proto 只追加 1 个 tag」口径。
    UpgradeTownHall {
        town_hall: EntityId,
        town_hall_def: DefId,
        target_tier: TownHallTier,
        cancel: bool,
    },
    /// P8.5c 系统级确定性命令：按冻结整数帧号初始化一个野怪营地（Q5 / D6）。
    ///
    /// 野怪 / 掉落是 `step()` 内部由 `seed` 驱动的被动系统，但营地 spawn
    /// 必须由 lockstep 两端一致注入，因此做成 frame-0（或任意冻结帧）
    /// 系统级命令：`camp` 是营地实例 id，`script` 是冻结
    /// [`crate::creep::CampScript`] 标识。`player_id` MUST 为
    /// [`crate::creep::NEUTRAL_OWNER`]（系统级，非玩家命令）；执行时重新
    /// 校验冻结脚本 / 中立类别 / 掉落表与幂等，不挂任何玩家 owner。
    /// 击杀 / 掉落仍是 `step()` 内部确定性事件，用新增 [`crate::sim_match::EventKind`]
    /// 事实事件供表现层只读消费，不与本命令面的被动系统语义矛盾。
    SeedCreepCamp {
        camp: u32,
        script: u32,
    },
    /// P8.5d 拾取地面物品：英雄走近 P8.5c 掉落的同一 `EntityId` 实例，
    /// MUST NOT 新建实例 / 重置充能冷却（R1.1 / R7.2 / R7.5）。
    PickupGroundItem {
        hero: EntityId,
        ground_item: EntityId,
    },
    /// P8.5d 商店购买事务：`shop` 是定义层商店 id，`shelf` 是冻结货架 id。
    /// 「距离 / 库存 / 黄金 / 背包空位校验 → 精确扣费 → 建实例 → 入包 →
    /// 库存递减」，失败事务性回滚退款（R7.3 / R9.3）。
    BuyFromShop {
        hero: EntityId,
        shop: u32,
        shelf: u32,
    },
    /// P8.5d 使用道具：先验证充能 > 0 / 冷却到期 / 目标合法再提交，
    /// 失败不扣次数（R7.5 / R7.6）。`target` 为 `EntityId::NONE` 表示自身。
    UseItem {
        hero: EntityId,
        instance: EntityId,
        target: EntityId,
    },
    /// P8.5d 丢弃道具：实例以同一 `EntityId` 回无主地面物品，
    /// 剩余充能 / 冷却「转交也不能刷新」（R1.1 / R7.5）。
    DropItem {
        hero: EntityId,
        instance: EntityId,
    },
    /// P8.5d 移交道具：两英雄六格背包间迁移同一实例，目标满包显式拒绝（R6.2）。
    TransferItem {
        from_hero: EntityId,
        to_hero: EntityId,
        instance: EntityId,
    },
    /// P8.5d 售卖道具：按实例移出背包、精确入账冻结回购价（R7.4 / R9.3）。
    SellItem {
        hero: EntityId,
        instance: EntityId,
        shop: u32,
    },
    /// P8.5d 使用回城卷轴：目标按「友方主城分类」定义层数据驱动选出，
    /// 无可用友方主城显式拒单不扣次数（R7.1 / 结论 7）。单英雄传送，
    /// MUST NOT 与大法师 MassTeleport 群体传送混淆。
    UseTownPortal {
        hero: EntityId,
        instance: EntityId,
    },
    /// P8.5d 显式中断回城施法（R7.5）；施法期主城被毁也由帧阶段自动取消。
    CancelTownPortal {
        hero: EntityId,
    },
    /// P8.5d 系统级市场刷新通知：`batch` 为批次 id，同一批次幂等不重复消费，
    /// 随机码复用 P8.5c 事件驱动流（R8.6 / Q6）。`player_id` MUST 为
    /// [`crate::creep::NEUTRAL_OWNER`]。
    RefreshMarket {
        shop: u32,
        batch: u32,
    },
    // -----------------------------------------------------------------------
    // P9 / Q2 / Q4 族特色机制命令面（kind 码 24 起，既有 1–23 不动）。
    //
    // 每个变体经 `GameplayCommand::new` 冻结 `(execute_frame, player_id,
    // sequence)`、由 `GameplayCommandQueue` 冻结到达序号、按
    // `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序执行；
    // 校验 / 哈希编码 / 穷尽分发三处同步（ADR 0002 §3 / §6）。
    // 命令只登记玩法机制输入，逐 30 Hz 整数帧结算由
    // `FactionMechanicsState::settle` 确定性完成；fog 可见性 MUST
    // 表现层只读派生，MUST NOT 由任何命令写进权威状态（R10.5 / §0.3 T3）。
    // -----------------------------------------------------------------------
    /// 腐地建造约束（R4.6）：登记一个腐地扩张源与初始腐地格。
    /// `cell_x` / `cell_y` 为整数网格坐标，`source` MUST 非 `EntityId::NONE`。
    SeedBlight {
        source: EntityId,
        cell_x: i32,
        cell_y: i32,
    },
    /// 尸体登记（`:119`）：登记一具尸体与剩余衰减整数帧，供食尸回血 /
    /// 召唤骷髅查表。`corpse` MUST 非 `EntityId::NONE`。
    RaiseCorpse {
        corpse: EntityId,
        decay_frames: u32,
    },
    /// 食尸回血 / 召唤骷髅（`:119`）：消费者消费指定尸体，确定性累计次数。
    /// `consumer` / `corpse` MUST 非 `EntityId::NONE`。
    ConsumeCorpse {
        consumer: EntityId,
        corpse: EntityId,
    },
    /// 诅咒金矿（R1.5）：登记金矿绑定为被诅咒的金矿（侍僧环绕获取）。
    /// `mine` MUST 非 `EntityId::NONE`。
    HauntGoldMine {
        mine: EntityId,
    },
    /// 缠绕金矿（R1.5）：登记金矿绑定为小精灵缠绕金矿。
    /// `mine` MUST 非 `EntityId::NONE`。
    EntangleGoldMine {
        mine: EntityId,
    },
    /// 古树状态：切换扎根（`rooted = true`，不可移动）/ 拔根。
    /// 可移动性由定义层数据决定，MUST NOT 用名字 / 段位白名单（R1.2）。
    /// `ancient` MUST 非 `EntityId::NONE`。
    SetAncientRoot {
        ancient: EntityId,
        rooted: bool,
    },
    /// 月亮井（R10.3）：登记月亮井及单次恢复量（合成原创整数 `[待核]`）。
    /// `well` MUST 非 `EntityId::NONE`。
    BindMoonWell {
        well: EntityId,
        restore: i64,
    },
    /// 隐身目标判定（R10.4 / S3）：登记单位的隐身态（玩法机制，
    /// 非 fog 可见性）。`unit` MUST 非 `EntityId::NONE`。
    SetStealth {
        unit: EntityId,
        cloaked: bool,
    },
    /// 反隐单位登记（R10.4）：把单位加入反隐集合。
    /// `unit` MUST 非 `EntityId::NONE`。
    RegisterDetector {
        unit: EntityId,
    },
    /// 暗夜夜间隐身特色（S3）：登记 / 移除夜间自动隐身候选单位。
    /// `unit` MUST 非 `EntityId::NONE`。
    SetNightCloak {
        unit: EntityId,
        enabled: bool,
    },
    /// 视线阻挡（R5.6 机制输入）：登记 / 移除阻挡视线的实体。
    /// 只登记机制输入，MUST NOT 存放 fog 可见性派生结果（R10.5）。
    /// `blocker` MUST 非 `EntityId::NONE`。
    MarkSightBlocker {
        blocker: EntityId,
        blocking: bool,
    },
    /// 野怪睡眠（R8.4）：登记野怪是否夜间睡眠（`false` 为夜间清醒例外）。
    /// `creep` MUST 非 `EntityId::NONE`。
    SetCreepSleep {
        creep: EntityId,
        sleeps_at_night: bool,
    },
    /// 回血条件（R10.3）：登记单位的定义层回血条件枚举，逐 tick 查表结算。
    /// `condition` 为 [`RegenCondition`]，线格式以 `condition.code()` 的
    /// `u8` 编码（值域 0..=4）。MUST NOT 用族名硬编码 / 白名单。
    /// `unit` MUST 非 `EntityId::NONE`。
    SetRegenCondition {
        unit: EntityId,
        condition: RegenCondition,
    },
    // -----------------------------------------------------------------------
    // P11 技能命令面（kind 码 37 起，既有 1–36 不动）。
    //
    // 施法仍走 `GameplayCommandKind::Cast` 系列；建造走
    // `Train { is_building: true }`，攻击 / 移动 / 停止走 `CommandKind`。
    // 新命令按同一队列冻结到达序号并按
    // `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序。
    // -----------------------------------------------------------------------
    /// 点目标施法；坐标为 Q47.16 整数原始值。
    CastPoint {
        caster: EntityId,
        ability: DefId,
        x: i64,
        y: i64,
    },
    /// 范围目标施法；半径使用 Q47.16 整数原始值。
    CastArea {
        caster: EntityId,
        ability: DefId,
        x: i64,
        y: i64,
        radius_raw: i64,
    },
    /// 方向目标施法；方向向量使用整数原始值。
    CastDirection {
        caster: EntityId,
        ability: DefId,
        dx: i64,
        dy: i64,
    },
    /// 学习 / 升级英雄技能；6 级前大招由冻结清单显式拒绝。
    LearnAbility {
        hero: EntityId,
        ability: DefId,
    },
    /// 英雄升级并累积技能点；本期不含 AI 自主升级 / 施法路径。
    LevelUpHero {
        hero: EntityId,
    },
    // -----------------------------------------------------------------------
    // P18 唯一新增玩法命令变体（kind 码 42，既有码 1–41 一个不动）。
    //
    // R7.4 回购价 / 交互距离走既有 `SellItem` 命令面与资源结算，
    // R8.5 三级 leash 走 `step()` 内部野怪 AI 结算，R6.1 经验阈值分档 /
    // R6.5 三类经验机制 / R5.3 克制矩阵均为击杀 / 攻击结算逻辑，
    // 全部 MUST NOT 新增命令变体或借命令旁路改写 R1.4 可移动性过滤。
    // -----------------------------------------------------------------------
    /// 酒馆买活：指定阵亡英雄与酒馆实体，按独立买活参数半血无蓝入场
    /// （R6.3）。与祭坛 `ReviveHero` 参数 / 命令分离；`hero` / `tavern`
    /// MUST 均非 `EntityId::NONE`。
    BuybackHero {
        hero: EntityId,
        tavern: EntityId,
    },
}

/// 玩法命令。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameplayCommand {
    pub execute_frame: Frame,
    pub player_id: u8,
    pub sequence: u32,
    pub kind: GameplayCommandKind,
    arrival: u64,
}

impl GameplayCommand {
    pub fn new(
        execute_frame: Frame,
        player_id: u8,
        sequence: u32,
        kind: GameplayCommandKind,
    ) -> Self {
        Self {
            execute_frame,
            player_id,
            sequence,
            kind,
            arrival: 0,
        }
    }

    pub const fn arrival(&self) -> u64 {
        self.arrival
    }

    fn with_arrival(mut self, arrival: u64) -> Self {
        self.arrival = arrival;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameplayCommandError {
    InvalidEntity,
    FrameInPast,
}

impl fmt::Display for GameplayCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEntity => f.write_str("invalid entity in gameplay command"),
            Self::FrameInPast => f.write_str("gameplay command frame is in the past"),
        }
    }
}

impl std::error::Error for GameplayCommandError {}

fn hash_gameplay_command(hasher: &mut StateHasher, command: &GameplayCommand) {
    hasher.write_u64(command.execute_frame.value());
    hasher.write_u8(command.player_id);
    hasher.write_u32(command.sequence);
    hasher.write_u64(command.arrival());
    match command.kind {
        GameplayCommandKind::Cast {
            caster,
            ability,
            target,
        } => {
            hasher.write_u8(1);
            hasher.write_u64(caster.value());
            hasher.write_u32(ability);
            hasher.write_u64(target.value());
        }
        GameplayCommandKind::Train {
            producer,
            unit_def,
            is_building,
        } => {
            hasher.write_u8(2);
            hasher.write_u64(producer.value());
            hasher.write_u32(unit_def);
            hasher.write_u8(u8::from(is_building));
        }
        GameplayCommandKind::CancelTrain { producer, index } => {
            hasher.write_u8(3);
            hasher.write_u64(producer.value());
            hasher.write_u32(index);
        }
        GameplayCommandKind::Research { upgrade } => {
            hasher.write_u8(4);
            hasher.write_u32(upgrade);
        }
        GameplayCommandKind::BuyItem { unit, item } => {
            hasher.write_u8(5);
            hasher.write_u64(unit.value());
            hasher.write_u32(item);
        }
        GameplayCommandKind::Gather { unit } => {
            hasher.write_u8(6);
            hasher.write_u64(unit.value());
        }
        GameplayCommandKind::RegisterBehavior { id, effect } => {
            hasher.write_u8(7);
            hasher.write_u32(id);
            hasher.write_u8(effect_kind_code(effect.kind));
            hasher.write_i64(effect.amount);
            hasher.write_u32(effect.duration_frames);
        }
        GameplayCommandKind::SpawnWave { id, period } => {
            hasher.write_u8(8);
            hasher.write_u32(id);
            hasher.write_u32(period);
        }
        GameplayCommandKind::SetAi { enabled } => {
            hasher.write_u8(9);
            hasher.write_u8(u8::from(enabled));
        }
        GameplayCommandKind::Leave { player } => {
            hasher.write_u8(10);
            hasher.write_u8(player);
        }
        GameplayCommandKind::ReviveHero {
            hero,
            altar,
            altar_def,
        } => {
            hasher.write_u8(11);
            hasher.write_u64(hero.value());
            hasher.write_u64(altar.value());
            hasher.write_u32(altar_def);
        }
        GameplayCommandKind::CancelRevive { hero } => {
            hasher.write_u8(12);
            hasher.write_u64(hero.value());
        }
        GameplayCommandKind::UpgradeTownHall {
            town_hall,
            town_hall_def,
            target_tier,
            cancel,
        } => {
            // P8.5b：追加唯一编码 13；target_tier 用显式 code，不依赖枚举布局。
            hasher.write_u8(13);
            hasher.write_u64(town_hall.value());
            hasher.write_u32(town_hall_def);
            hasher.write_u8(target_tier.code());
            hasher.write_u8(u8::from(cancel));
        }
        GameplayCommandKind::SeedCreepCamp { camp, script } => {
            // P8.5c：追加唯一编码 14；camp / script 均显式小端编码，
            // 不与既有 1–13 变体碰撞，且与校验 / 穷尽分发同步（Q5 / D6）。
            hasher.write_u8(14);
            hasher.write_u32(camp);
            hasher.write_u32(script);
        }
        // P8.5d：追加唯一编码 15–23，既有码 1–14 一个不动；每个字段
        // 显式小端编码，且与校验 / 穷尽分发三处同步（Q5 / D2）。
        GameplayCommandKind::PickupGroundItem { hero, ground_item } => {
            hasher.write_u8(15);
            hasher.write_u64(hero.value());
            hasher.write_u64(ground_item.value());
        }
        GameplayCommandKind::BuyFromShop { hero, shop, shelf } => {
            hasher.write_u8(16);
            hasher.write_u64(hero.value());
            hasher.write_u32(shop);
            hasher.write_u32(shelf);
        }
        GameplayCommandKind::UseItem {
            hero,
            instance,
            target,
        } => {
            hasher.write_u8(17);
            hasher.write_u64(hero.value());
            hasher.write_u64(instance.value());
            hasher.write_u64(target.value());
        }
        GameplayCommandKind::DropItem { hero, instance } => {
            hasher.write_u8(18);
            hasher.write_u64(hero.value());
            hasher.write_u64(instance.value());
        }
        GameplayCommandKind::TransferItem {
            from_hero,
            to_hero,
            instance,
        } => {
            hasher.write_u8(19);
            hasher.write_u64(from_hero.value());
            hasher.write_u64(to_hero.value());
            hasher.write_u64(instance.value());
        }
        GameplayCommandKind::SellItem {
            hero,
            instance,
            shop,
        } => {
            hasher.write_u8(20);
            hasher.write_u64(hero.value());
            hasher.write_u64(instance.value());
            hasher.write_u32(shop);
        }
        GameplayCommandKind::UseTownPortal { hero, instance } => {
            hasher.write_u8(21);
            hasher.write_u64(hero.value());
            hasher.write_u64(instance.value());
        }
        GameplayCommandKind::CancelTownPortal { hero } => {
            hasher.write_u8(22);
            hasher.write_u64(hero.value());
        }
        GameplayCommandKind::RefreshMarket { shop, batch } => {
            hasher.write_u8(23);
            hasher.write_u32(shop);
            hasher.write_u32(batch);
        }
        // P9：追加唯一编码 24–36，既有码 1–23 一个不动；每个字段显式
        // 小端编码，且与校验 / 穷尽分发三处同步（Q5 / D2）。
        GameplayCommandKind::SeedBlight {
            source,
            cell_x,
            cell_y,
        } => {
            hasher.write_u8(24);
            hasher.write_u64(source.value());
            hasher.write_i64(i64::from(cell_x));
            hasher.write_i64(i64::from(cell_y));
        }
        GameplayCommandKind::RaiseCorpse {
            corpse,
            decay_frames,
        } => {
            hasher.write_u8(25);
            hasher.write_u64(corpse.value());
            hasher.write_u32(decay_frames);
        }
        GameplayCommandKind::ConsumeCorpse { consumer, corpse } => {
            hasher.write_u8(26);
            hasher.write_u64(consumer.value());
            hasher.write_u64(corpse.value());
        }
        GameplayCommandKind::HauntGoldMine { mine } => {
            hasher.write_u8(27);
            hasher.write_u64(mine.value());
        }
        GameplayCommandKind::EntangleGoldMine { mine } => {
            hasher.write_u8(28);
            hasher.write_u64(mine.value());
        }
        GameplayCommandKind::SetAncientRoot { ancient, rooted } => {
            hasher.write_u8(29);
            hasher.write_u64(ancient.value());
            hasher.write_u8(u8::from(rooted));
        }
        GameplayCommandKind::BindMoonWell { well, restore } => {
            hasher.write_u8(30);
            hasher.write_u64(well.value());
            hasher.write_i64(restore);
        }
        GameplayCommandKind::SetStealth { unit, cloaked } => {
            hasher.write_u8(31);
            hasher.write_u64(unit.value());
            hasher.write_u8(u8::from(cloaked));
        }
        GameplayCommandKind::RegisterDetector { unit } => {
            hasher.write_u8(32);
            hasher.write_u64(unit.value());
        }
        GameplayCommandKind::SetNightCloak { unit, enabled } => {
            hasher.write_u8(33);
            hasher.write_u64(unit.value());
            hasher.write_u8(u8::from(enabled));
        }
        GameplayCommandKind::MarkSightBlocker { blocker, blocking } => {
            hasher.write_u8(34);
            hasher.write_u64(blocker.value());
            hasher.write_u8(u8::from(blocking));
        }
        GameplayCommandKind::SetCreepSleep {
            creep,
            sleeps_at_night,
        } => {
            hasher.write_u8(35);
            hasher.write_u64(creep.value());
            hasher.write_u8(u8::from(sleeps_at_night));
        }
        GameplayCommandKind::SetRegenCondition { unit, condition } => {
            hasher.write_u8(36);
            hasher.write_u64(unit.value());
            hasher.write_u8(condition.code());
        }
        // P11：追加唯一编码 37–41，既有码 1–36 一个不动；每个字段显式
        // 小端编码，且与校验 / 穷尽分发三处同步（D6 / Q3 / Q5）。
        GameplayCommandKind::CastPoint {
            caster,
            ability,
            x,
            y,
        } => {
            hasher.write_u8(37);
            hasher.write_u64(caster.value());
            hasher.write_u32(ability);
            hasher.write_i64(x);
            hasher.write_i64(y);
        }
        GameplayCommandKind::CastArea {
            caster,
            ability,
            x,
            y,
            radius_raw,
        } => {
            hasher.write_u8(38);
            hasher.write_u64(caster.value());
            hasher.write_u32(ability);
            hasher.write_i64(x);
            hasher.write_i64(y);
            hasher.write_i64(radius_raw);
        }
        GameplayCommandKind::CastDirection {
            caster,
            ability,
            dx,
            dy,
        } => {
            hasher.write_u8(39);
            hasher.write_u64(caster.value());
            hasher.write_u32(ability);
            hasher.write_i64(dx);
            hasher.write_i64(dy);
        }
        GameplayCommandKind::LearnAbility { hero, ability } => {
            hasher.write_u8(40);
            hasher.write_u64(hero.value());
            hasher.write_u32(ability);
        }
        GameplayCommandKind::LevelUpHero { hero } => {
            hasher.write_u8(41);
            hasher.write_u64(hero.value());
        }
        // P18：追加唯一编码 42，既有码 1–41 一个不动；两个实体字段显式
        // 小端编码，且与校验 / 穷尽分发三处同步（D4 / Q1）。
        GameplayCommandKind::BuybackHero { hero, tavern } => {
            hasher.write_u8(42);
            hasher.write_u64(hero.value());
            hasher.write_u64(tavern.value());
        }
    }
}

/// 玩家与 AI 命令共用的实体合法性校验（同一校验入口的一部分）。
pub(crate) fn validate_gameplay_entities(
    kind: &GameplayCommandKind,
) -> Result<(), GameplayCommandError> {
    let invalid = match *kind {
        GameplayCommandKind::Cast { caster, target, .. } => {
            caster == EntityId::NONE || target == EntityId::NONE
        }
        GameplayCommandKind::Train { producer, .. }
        | GameplayCommandKind::CancelTrain { producer, .. }
        | GameplayCommandKind::BuyItem { unit: producer, .. }
        | GameplayCommandKind::Gather { unit: producer } => producer == EntityId::NONE,
        GameplayCommandKind::ReviveHero {
            hero,
            altar,
            altar_def: _,
        } => hero == EntityId::NONE || altar == EntityId::NONE,
        GameplayCommandKind::CancelRevive { hero } => hero == EntityId::NONE,
        GameplayCommandKind::UpgradeTownHall { town_hall, .. } => town_hall == EntityId::NONE,
        // P8.5c：营地 id / 冻结脚本 id 均不得为 0（系统级命令的实体合法性
        // 校验），与 hash 编码 / 穷尽分发三处同步（Q5 / D6）。
        GameplayCommandKind::SeedCreepCamp { camp, script } => camp == 0 || script == 0,
        // P8.5d：`UseItem` 的 `target` 允许 `EntityId::NONE` 表示自身，
        // 其余实体字段 MUST NOT 为 0；商店 / 货架 / 批次 id 不得为 0。
        GameplayCommandKind::PickupGroundItem { hero, ground_item } => {
            hero == EntityId::NONE || ground_item == EntityId::NONE
        }
        GameplayCommandKind::BuyFromShop { hero, shop, shelf } => {
            hero == EntityId::NONE || shop == 0 || shelf == 0
        }
        GameplayCommandKind::UseItem { hero, instance, .. } => {
            hero == EntityId::NONE || instance == EntityId::NONE
        }
        GameplayCommandKind::DropItem { hero, instance } => {
            hero == EntityId::NONE || instance == EntityId::NONE
        }
        GameplayCommandKind::TransferItem {
            from_hero,
            to_hero,
            instance,
        } => from_hero == EntityId::NONE || to_hero == EntityId::NONE || instance == EntityId::NONE,
        GameplayCommandKind::SellItem {
            hero,
            instance,
            shop,
        } => hero == EntityId::NONE || instance == EntityId::NONE || shop == 0,
        GameplayCommandKind::UseTownPortal { hero, instance } => {
            hero == EntityId::NONE || instance == EntityId::NONE
        }
        GameplayCommandKind::CancelTownPortal { hero } => hero == EntityId::NONE,
        GameplayCommandKind::RefreshMarket { shop, batch } => shop == 0 || batch == 0,
        // 既有 1–23 中无实体字段的变体显式列出（行为不变），使本 match
        // 完全穷尽：任何将来新增变体都会触发编译错误，MUST NOT 靠
        // `_ => false` 默认放行（ADR 0002 §3 / Q5 三处同步）。
        GameplayCommandKind::Research { .. }
        | GameplayCommandKind::RegisterBehavior { .. }
        | GameplayCommandKind::SpawnWave { .. }
        | GameplayCommandKind::SetAi { .. }
        | GameplayCommandKind::Leave { .. } => false,
        // P9 / Q2 / Q4 新增变体：实体字段 MUST 逐个校验，
        // 新增命令变体漏分支即触发编译错误（Q5 / 反向验证形态①）。
        GameplayCommandKind::SeedBlight { source, .. } => source == EntityId::NONE,
        GameplayCommandKind::RaiseCorpse { corpse, .. } => corpse == EntityId::NONE,
        GameplayCommandKind::ConsumeCorpse { consumer, corpse } => {
            consumer == EntityId::NONE || corpse == EntityId::NONE
        }
        GameplayCommandKind::HauntGoldMine { mine } => mine == EntityId::NONE,
        GameplayCommandKind::EntangleGoldMine { mine } => mine == EntityId::NONE,
        GameplayCommandKind::SetAncientRoot { ancient, .. } => ancient == EntityId::NONE,
        GameplayCommandKind::BindMoonWell { well, .. } => well == EntityId::NONE,
        GameplayCommandKind::SetStealth { unit, .. } => unit == EntityId::NONE,
        GameplayCommandKind::RegisterDetector { unit } => unit == EntityId::NONE,
        GameplayCommandKind::SetNightCloak { unit, .. } => unit == EntityId::NONE,
        GameplayCommandKind::MarkSightBlocker { blocker, .. } => blocker == EntityId::NONE,
        GameplayCommandKind::SetCreepSleep { creep, .. } => creep == EntityId::NONE,
        GameplayCommandKind::SetRegenCondition { unit, .. } => unit == EntityId::NONE,
        // P11 新增命令：实体字段与非负半径 MUST 逐个校验，新增变体漏分支
        // 即触发编译错误，MUST NOT 靠默认放行（ADR 0002 §3）。
        GameplayCommandKind::CastPoint { caster, .. }
        | GameplayCommandKind::CastDirection { caster, .. } => caster == EntityId::NONE,
        GameplayCommandKind::CastArea {
            caster, radius_raw, ..
        } => caster == EntityId::NONE || radius_raw < 0,
        GameplayCommandKind::LearnAbility { hero, .. }
        | GameplayCommandKind::LevelUpHero { hero } => hero == EntityId::NONE,
        // P18 唯一新增变体：酒馆买活两个实体字段 MUST 逐个校验，新增变体
        // 漏分支即触发编译错误，MUST NOT 靠默认放行（ADR 0002 §3）。
        GameplayCommandKind::BuybackHero { hero, tavern } => {
            hero == EntityId::NONE || tavern == EntityId::NONE
        }
    };
    if invalid {
        return Err(GameplayCommandError::InvalidEntity);
    }
    Ok(())
}

/// 玩法命令队列：与 `CommandQueue` 相同的稳定排序键。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GameplayCommandQueue {
    pending: Vec<GameplayCommand>,
    next_arrival: u64,
}

impl GameplayCommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// 校验、冻结到达序号并入队；被接收不等于最终执行成功。
    pub fn submit(&mut self, command: GameplayCommand) -> Result<u64, GameplayCommandError> {
        validate_gameplay_entities(&command.kind)?;
        let arrival = self.next_arrival;
        self.next_arrival += 1;
        self.pending.push(command.with_arrival(arrival));
        Ok(arrival)
    }

    pub fn freeze_frame(&mut self, frame: Frame) -> Vec<GameplayCommand> {
        let (mut due, rest): (Vec<GameplayCommand>, Vec<GameplayCommand>) = self
            .pending
            .drain(..)
            .partition(|command| command.execute_frame <= frame);
        self.pending = rest;
        due.sort_by_key(|command| {
            (
                command.execute_frame,
                command.player_id,
                command.sequence,
                command.arrival(),
            )
        });
        due
    }

    pub fn pending(&self) -> &[GameplayCommand] {
        &self.pending
    }

    pub fn next_arrival(&self) -> u64 {
        self.next_arrival
    }

    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.next_arrival);
        hasher.write_u64(self.pending.len() as u64);
        for command in &self.pending {
            hash_gameplay_command(hasher, command);
        }
    }
}

// ---------------------------------------------------------------------------
// 全局玩法运行态（唯一容器）
// ---------------------------------------------------------------------------

/// 全部玩法运行态的唯一容器，便于进入哈希 / 快照 / 差异诊断。
///
/// P11：`abilities` 承载技能系统新增权威运行态，与 `commands` 的未执行命令
/// 分离；该字段 MUST 同时进入本容器哈希、本容器差异诊断、快照克隆与恢复。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct WorldGameplayState {
    pub players: BTreeMap<u8, PlayerEconomy>,
    pub victory: VictoryState,
    pub ai: AiState,
    pub waves: BTreeMap<u32, WaveState>,
    pub auras: BTreeMap<u32, AuraState>,
    pub behaviors: BTreeMap<DefId, BehaviorSpec>,
    pub abilities: AbilitySystemState,
    pub commands: GameplayCommandQueue,
}

impl WorldGameplayState {
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.players.len() as u64);
        for (&player, economy) in &self.players {
            hasher.write_u8(player);
            hasher.write_i64(economy.gold);
            hasher.write_i64(economy.lumber);
            hasher.write_i64(economy.supply_used);
            hasher.write_i64(economy.supply_cap);
            hasher.write_i64(economy.produced);
            match economy.research {
                Some((upgrade, remaining)) => {
                    hasher.write_u8(1);
                    hasher.write_u32(upgrade);
                    hasher.write_u32(remaining);
                }
                None => hasher.write_u8(0),
            }
            hasher.write_u64(economy.upgrades.len() as u64);
            for (&upgrade, &done) in &economy.upgrades {
                hasher.write_u32(upgrade);
                hasher.write_u8(u8::from(done));
            }
            hasher.write_u8(u8::from(economy.active));
        }
        hasher.write_u8(u8::from(self.victory.resolved));
        // Option 必须带 tag，区分 `None` 与 `Some(0)`，避免可构造的哈希碰撞。
        match self.victory.winner {
            Some(player) => {
                hasher.write_u8(1);
                hasher.write_u8(player);
            }
            None => hasher.write_u8(0),
        }
        hasher.write_u64(self.victory.resolved_frame);
        hasher.write_u8(u8::from(self.ai.enabled));
        hasher.write_u64(self.ai.next_decision_frame);
        hasher.write_u32(self.ai.decision_period);
        hasher.write_u64(self.ai.pending_tasks.len() as u64);
        for (&id, &remaining) in &self.ai.pending_tasks {
            hasher.write_u32(id);
            hasher.write_u32(remaining);
        }
        hasher.write_u64(self.waves.len() as u64);
        for (&id, wave) in &self.waves {
            hasher.write_u32(id);
            hasher.write_u32(wave.period);
            hasher.write_u32(wave.remaining);
            hasher.write_u32(wave.count);
        }
        hasher.write_u64(self.auras.len() as u64);
        for (&id, aura) in &self.auras {
            hasher.write_u32(id);
            hasher.write_u64(aura.source.value());
            hasher.write_u32(aura.ability);
            hasher.write_u32(aura.remaining);
        }
        hasher.write_u64(self.behaviors.len() as u64);
        for (&id, spec) in &self.behaviors {
            hasher.write_u32(id);
            hasher.write_u32(spec.id);
            hasher.write_u8(effect_kind_code(spec.extra_effect.kind));
            hasher.write_i64(spec.extra_effect.amount);
            hasher.write_u32(spec.extra_effect.duration_frames);
        }
        // P11：技能系统全部权威运行态进入 FNV-1a，字段顺序与
        // AbilitySystemState::first_difference() 一致。
        self.abilities.hash(hasher);
        self.commands.hash(hasher);
    }

    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.players != other.players {
            return Some("gameplay_players");
        }
        if self.victory != other.victory {
            return Some("victory");
        }
        if self.ai != other.ai {
            return Some("ai");
        }
        if self.waves != other.waves {
            return Some("waves");
        }
        if self.auras != other.auras {
            return Some("auras");
        }
        if self.behaviors != other.behaviors {
            return Some("behaviors");
        }
        if let Some(field) = self.abilities.first_difference(&other.abilities) {
            return Some(field);
        }
        if self.commands != other.commands {
            return Some("gameplay_commands");
        }
        None
    }
}

/// 技能三层扩展：模板流程 + 效果组合 + 按行为 ID 注册。
///
/// 返回本次施法要执行的效果链；行为由 ID 与显式数据决定，不含闭包。
pub fn resolve_effects(
    defs: &FrozenDefinitions,
    world: &WorldGameplayState,
    ability: DefId,
) -> Vec<EffectDef> {
    let Some(def) = defs.abilities.get(&ability) else {
        return Vec::new();
    };
    let mut effects = def.effects.clone();
    if def.behavior != 0 {
        // P11：先在单一行为目录查表追加原语，再保留既有注册行为的扩展效果。
        if let Some(entry) = defs.ability_system().behaviors.entry_by_id(def.behavior) {
            effects.push(EffectDef {
                kind: entry.effect_kind,
                amount: i64::from(def.behavior),
                duration_frames: def.cooldown_frames,
            });
        }
        if let Some(spec) = world.behaviors.get(&def.behavior) {
            effects.push(spec.extra_effect);
        }
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creep::DropSet;

    fn unit_def() -> UnitDef {
        UnitDef {
            hp: 100,
            mana: 50,
            speed_raw: 3 << 16,
            damage: 10,
            range_raw: 10 << 16,
            supply: 1,
            train_frames: 5,
        }
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let result = FrozenDefinitions::new(
            vec![(1, unit_def()), (1, unit_def())],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(result, Err(DefinitionError::DuplicateId(1)));
    }

    #[test]
    fn content_hash_is_stable_and_content_sensitive() {
        let a = FrozenDefinitions::new(vec![(1, unit_def())], Vec::new(), Vec::new(), Vec::new())
            .unwrap();
        let b = FrozenDefinitions::new(vec![(1, unit_def())], Vec::new(), Vec::new(), Vec::new())
            .unwrap();
        assert_eq!(a.content_hash(), b.content_hash());
        let mut changed = unit_def();
        changed.hp = 200;
        let c =
            FrozenDefinitions::new(vec![(1, changed)], Vec::new(), Vec::new(), Vec::new()).unwrap();
        assert_ne!(a.content_hash(), c.content_hash());
    }

    #[test]
    fn gameplay_queue_uses_stable_sort_key() {
        let mut queue = GameplayCommandQueue::new();
        queue
            .submit(GameplayCommand::new(
                Frame::new(1),
                2,
                1,
                GameplayCommandKind::SetAi { enabled: true },
            ))
            .unwrap();
        queue
            .submit(GameplayCommand::new(
                Frame::new(1),
                1,
                2,
                GameplayCommandKind::SetAi { enabled: false },
            ))
            .unwrap();
        let due = queue.freeze_frame(Frame::new(1));
        assert_eq!(due[0].player_id, 1);
        assert_eq!(due[1].player_id, 2);
    }

    // -----------------------------------------------------------------------
    // P8.5c 定义层单元测试（tasks 2.2 / 2.3 / 6.2）
    // -----------------------------------------------------------------------

    /// 构造一份全合成 / 原创编译期整数的野怪冻结定义（零文件 IO、零资产）。
    fn creep_fixture() -> (
        CreepConstants,
        BTreeMap<u32, CampScript>,
        BTreeMap<u32, DropTable>,
        BTreeMap<DefId, NeutralCategory>,
    ) {
        let mut constants = SYNTHETIC_CREEP_CONSTANTS;
        constants.leash_radius_raw = 9 * (1 << 16);
        let scripts = [(
            41u32,
            CampScript {
                tier: 1,
                xp: 25,
                gold: 12,
                home_x: 3 << 16,
                home_y: 4 << 16,
                drop_table: 91,
                creep_defs: vec![9101, 9102],
            },
        )]
        .into_iter()
        .collect();
        let tables = [(
            91u32,
            DropTable {
                sets: [(
                    7u32,
                    DropSet {
                        entries: [(9201u32, 60u32), (9202, 40)].into_iter().collect(),
                    },
                )]
                .into_iter()
                .collect(),
                effects: [
                    (
                        9201u32,
                        EffectDef {
                            kind: EffectKind::BuffStat,
                            amount: 2,
                            duration_frames: 10,
                        },
                    ),
                    (
                        9202,
                        EffectDef {
                            kind: EffectKind::Heal,
                            amount: 5,
                            duration_frames: 0,
                        },
                    ),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect();
        let categories = [
            (9101, NeutralCategory::CreepHostile),
            (9102, NeutralCategory::CreepHostile),
            (9201, NeutralCategory::GroundItemOwnerless),
            (9202, NeutralCategory::GroundItemOwnerless),
        ]
        .into_iter()
        .collect();
        (constants, scripts, tables, categories)
    }

    fn creep_defs() -> FrozenDefinitions {
        let (constants, scripts, tables, categories) = creep_fixture();
        FrozenDefinitions::new(vec![(9101, unit_def())], Vec::new(), Vec::new(), Vec::new())
            .unwrap()
            .with_creep_camps(constants, scripts, tables, categories)
    }

    /// tasks 2.2：冻结常量 / 营地脚本 / 掉落表（含互斥 set 与概率条目）/
    /// 中立类别登记四段任一段改动都改变 `content_hash()`。
    #[test]
    fn creep_frozen_segments_all_change_content_hash() {
        let base = creep_defs();
        let (constants, scripts, tables, categories) = creep_fixture();

        let mut changed_constants = constants;
        changed_constants.leash_radius_raw += 1;
        let a = FrozenDefinitions::new(vec![(9101, unit_def())], vec![], vec![], vec![])
            .unwrap()
            .with_creep_camps(
                changed_constants,
                scripts.clone(),
                tables.clone(),
                categories.clone(),
            );
        assert_ne!(base.content_hash(), a.content_hash(), "constants segment");

        let mut changed_scripts = scripts.clone();
        changed_scripts.get_mut(&41).unwrap().xp += 1;
        let b = FrozenDefinitions::new(vec![(9101, unit_def())], vec![], vec![], vec![])
            .unwrap()
            .with_creep_camps(
                constants,
                changed_scripts,
                tables.clone(),
                categories.clone(),
            );
        assert_ne!(base.content_hash(), b.content_hash(), "camp script segment");

        let mut changed_tables = tables.clone();
        changed_tables
            .get_mut(&91)
            .unwrap()
            .sets
            .get_mut(&7)
            .unwrap()
            .entries
            .insert(9201, 61);
        let c = FrozenDefinitions::new(vec![(9101, unit_def())], vec![], vec![], vec![])
            .unwrap()
            .with_creep_camps(
                constants,
                scripts.clone(),
                changed_tables,
                categories.clone(),
            );
        assert_ne!(
            base.content_hash(),
            c.content_hash(),
            "drop table probability segment"
        );

        let mut changed_categories = categories.clone();
        changed_categories.insert(9999, NeutralCategory::CreepHostile);
        let d = FrozenDefinitions::new(vec![(9101, unit_def())], vec![], vec![], vec![])
            .unwrap()
            .with_creep_camps(constants, scripts, tables, changed_categories);
        assert_ne!(
            base.content_hash(),
            d.content_hash(),
            "neutral category segment"
        );

        // 掉落表的 EffectDef 段（R7.6）同样进定义哈希。
        let (c2, s2, mut t2, cat2) = creep_fixture();
        if let Some(effect) = t2.get_mut(&91).unwrap().effects.get_mut(&9201) {
            effect.amount += 1;
        }
        let e = FrozenDefinitions::new(vec![(9101, unit_def())], vec![], vec![], vec![])
            .unwrap()
            .with_creep_camps(c2, s2, t2, cat2);
        assert_ne!(base.content_hash(), e.content_hash(), "effect def segment");
    }

    /// tasks 2.3：可用性与中立类别由定义层数据驱动；非经典命名的 DefId
    /// 仍按冻结定义判定，MUST NOT 依赖建筑名白名单或硬编码 DefId。
    #[test]
    fn creep_categories_are_data_driven_for_non_classic_def_ids() {
        let defs = creep_defs();
        assert!(defs.is_creep(9101));
        assert!(defs.is_creep(9102));
        assert!(defs.is_ground_item(9201));
        assert!(defs.is_ground_item(9202));
        assert!(!defs.is_creep(9201));
        assert!(!defs.is_ground_item(9101));
        assert!(!defs.is_creep(1));
        assert_eq!(
            defs.neutral_category(9101),
            Some(NeutralCategory::CreepHostile)
        );
        assert_eq!(
            defs.neutral_category(9202),
            Some(NeutralCategory::GroundItemOwnerless)
        );
        assert_eq!(defs.camp_script(41).map(|s| s.drop_table), Some(91));
        assert_eq!(
            defs.drop_table(91)
                .and_then(|t| t.sets.get(&7))
                .map(|s| s.total_weight()),
            Some(100)
        );
        assert!(defs.camp_script(42).is_none());
        assert!(defs.drop_table(92).is_none());
    }

    /// tasks 6.2：`SeedCreepCamp`（码 14）编码唯一，camp / script 两个字段
    /// 均参与哈希；不同语义的命令 MUST NOT 碰撞。
    #[test]
    fn seed_creep_camp_command_encoding_is_unique() {
        let command = |camp: u32, script: u32| {
            GameplayCommand::new(
                Frame::new(0),
                0,
                1,
                GameplayCommandKind::SeedCreepCamp { camp, script },
            )
        };
        let hash = |cmd: GameplayCommand| {
            let mut queue = GameplayCommandQueue::new();
            queue.submit(cmd).unwrap();
            let mut hasher = StateHasher::new();
            queue.hash(&mut hasher);
            hasher.finish()
        };
        assert_ne!(
            hash(command(1, 41)),
            hash(command(2, 41)),
            "camp must be encoded"
        );
        assert_ne!(
            hash(command(1, 41)),
            hash(command(1, 42)),
            "script must be encoded"
        );
        // 校验覆盖：0 号营地 / 脚本被公开提交入口明确拒绝。
        let mut queue = GameplayCommandQueue::new();
        assert_eq!(
            queue.submit(command(0, 41)),
            Err(GameplayCommandError::InvalidEntity)
        );
        assert_eq!(
            queue.submit(command(1, 0)),
            Err(GameplayCommandError::InvalidEntity)
        );
    }
}
