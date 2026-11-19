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

use crate::entity::EntityId;
use crate::frame::Frame;
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
/// 计算，祭坛复活与酒馆买活的参数 / 命令分离（酒馆按 Q4 不在本期范围）。
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
// 冻结内容定义
// ---------------------------------------------------------------------------

/// 效果种类。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectKind {
    Damage,
    Heal,
    BuffStat,
    Stun,
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
/// `hero_defs` 为定义层登记的祭坛 / 英雄类别 DefId 集合。三者都编入
/// `compute_defs_hash`，改常量或改类别登记即改变 `content_hash()`（Q3）。
/// 类别判定只查这两份定义层数据（数据驱动），MUST NOT 用建筑名字符串
/// 白名单或硬编码 DefId；`hero_defs` 让英雄经既有 `Train` 生产路径产生，
/// 使 `HeroState` 不再是死字段。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FrozenDefinitions {
    units: BTreeMap<DefId, UnitDef>,
    abilities: BTreeMap<DefId, AbilityDef>,
    items: BTreeMap<DefId, ItemDef>,
    upgrades: BTreeMap<DefId, UpgradeDef>,
    revive: ReviveConstants,
    altar_defs: BTreeSet<DefId>,
    hero_defs: BTreeSet<DefId>,
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
    }
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
/// 逐项承载 `hero.level` / `hero.xp`、6 格背包、技能等级 / 冷却登记
/// （`cooldowns`）、法力与 Buff，复活时回填到同一实体。
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

/// 生产 / 建造队列项。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProductionItem {
    pub unit_def: DefId,
    pub remaining_frames: u32,
    pub is_building: bool,
}

/// 单个单位的玩法运行态（全部整数，可进入哈希 / 快照 / 差异诊断）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UnitGameplayState {
    pub mana: i64,
    pub max_mana: i64,
    pub buffs: Vec<BuffInstance>,
    pub cast: Option<CastRuntime>,
    pub cooldowns: BTreeMap<DefId, u32>,
    pub hero: Option<HeroState>,
    pub bag: Vec<DefId>,
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
        hasher.write_u64(self.bag.len() as u64);
        for &item in &self.bag {
            hasher.write_u32(item);
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
        _ => false,
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
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct WorldGameplayState {
    pub players: BTreeMap<u8, PlayerEconomy>,
    pub victory: VictoryState,
    pub ai: AiState,
    pub waves: BTreeMap<u32, WaveState>,
    pub auras: BTreeMap<u32, AuraState>,
    pub behaviors: BTreeMap<DefId, BehaviorSpec>,
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
        if let Some(spec) = world.behaviors.get(&def.behavior) {
            effects.push(spec.extra_effect);
        }
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
