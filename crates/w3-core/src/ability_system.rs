//! P11 技能系统：冻结定义层、行为目录与权威运行态。
//!
//! 本模块只承载定宽整数与 `BTreeMap` / `BTreeSet` 稳定序数据，不读取文件、
//! 不依赖内容层 / 网络 / 表现层，不持有闭包或独立随机流。技能清单与行为
//! 分类属冻结定义，经 `compute_defs_hash` 进入 `defs_hash` / `content_hash`
//! 快照头部；zone / 召唤 / 光环受益集 / 被动 proc / 变身 / buff 分类 /
//! 链接 / `rng_draws` 属权威运行态，MUST 同时进入状态哈希、快照、
//! `first_difference()` 与恢复路径（ADR 0002 §6-8）。
//!
//! 清单结构按四个席位 × 四英雄 × 四技能与代表性单位 / 道具技能登记，
//! 全部为合成原创 `DefId`、合成原创整数 `[待核]`，不含经典技能名、
//! 四字符 rawcode、导出数值或资产路径。

use std::collections::{BTreeMap, BTreeSet};

use crate::entity::EntityId;
use crate::gameplay::{DefId, EffectKind};
use crate::state_hash::StateHasher;

/// 技能清单的起始 `DefId` 段；纯合成原创，仅用于稳定排序。
pub const ABILITY_CATALOG_BASE: DefId = 20_000;
/// 行为目录的起始 `DefId` 段；与技能清单分段，避免语义混用。
pub const BEHAVIOR_CATALOG_BASE: DefId = 21_000;
/// zone / chain 等非实体稳定标识的起始 ID 段。
pub const ABILITY_ZONE_ID_BASE: u64 = 30_000;
/// 链接弹射表的稳定标识起始 ID 段。
pub const ABILITY_CHAIN_ID_BASE: u64 = 40_000;
/// 行为目录种类数量；新增原语时只扩展注册表，不改既有码。
pub const BEHAVIOR_KIND_COUNT: usize = 9;
/// 四席位 × 四英雄 × 四技能 + 每席位代表性单位 / 道具技能的清单规模。
pub const ABILITY_CATALOG_LEN: usize = 72;

/// 四个族席位的合成占位；不对应经典名称 / rawcode，也不作规则数值来源。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum FactionSlot {
    First,
    Second,
    Third,
    Fourth,
}

impl FactionSlot {
    /// 全部四席位的稳定顺序。
    pub const ALL: [Self; 4] = [Self::First, Self::Second, Self::Third, Self::Fourth];

    /// 冻结定义哈希使用的稳定字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::First => 1,
            Self::Second => 2,
            Self::Third => 3,
            Self::Fourth => 4,
        }
    }

    /// 由索引生成席位；仅供编译期清单表使用。
    const fn from_index(index: usize) -> Self {
        match index % Self::ALL.len() {
            0 => Self::First,
            1 => Self::Second,
            2 => Self::Third,
            _ => Self::Fourth,
        }
    }
}

/// 技能清单条目的分类；对齐清单结构，不登记经典具名技能。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum AbilityCategory {
    HeroBasic,
    HeroUltimate,
    Unit,
    Item,
}

impl AbilityCategory {
    /// 冻结定义哈希使用的稳定字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::HeroBasic => 1,
            Self::HeroUltimate => 2,
            Self::Unit => 3,
            Self::Item => 4,
        }
    }
}

/// 单条合成原创技能清单登记；数值均为结构元数据 / `[待核]` 占位。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct AbilityCatalogEntry {
    pub id: DefId,
    pub faction: FactionSlot,
    pub hero_slot: u8,
    pub category: AbilityCategory,
    pub levels: u8,
    pub level_req: u8,
    pub behavior_id: DefId,
}

const EMPTY_ABILITY_CATALOG_ENTRY: AbilityCatalogEntry = AbilityCatalogEntry {
    id: 0,
    faction: FactionSlot::First,
    hero_slot: 0,
    category: AbilityCategory::HeroBasic,
    levels: 0,
    level_req: 0,
    behavior_id: 0,
};

/// 由席位索引生成合成原创 `DefId`，仅用于冻结清单稳定排序。
const fn ability_id(index: usize) -> DefId {
    ABILITY_CATALOG_BASE + index as DefId
}

/// 由全局索引轮换行为 ID，保证四席位清单覆盖全部最小必要原语。
const fn behavior_id(index: usize) -> DefId {
    BEHAVIOR_CATALOG_BASE + (index % BEHAVIOR_KIND_COUNT) as DefId
}

/// 生成纯编译期常量表；零文件 IO、零资产、零宿主环境输入。
const fn synthetic_ability_catalog() -> [AbilityCatalogEntry; ABILITY_CATALOG_LEN] {
    let mut entries = [EMPTY_ABILITY_CATALOG_ENTRY; ABILITY_CATALOG_LEN];
    let mut faction = 0usize;
    let mut index = 0usize;
    while faction < FactionSlot::ALL.len() {
        let slot = FactionSlot::from_index(faction);
        let mut hero = 0usize;
        while hero < 4 {
            let mut skill = 0usize;
            while skill < 4 {
                let ultimate = skill == 3;
                entries[index] = AbilityCatalogEntry {
                    id: ability_id(index),
                    faction: slot,
                    hero_slot: hero as u8,
                    category: if ultimate {
                        AbilityCategory::HeroUltimate
                    } else {
                        AbilityCategory::HeroBasic
                    },
                    levels: if ultimate { 1 } else { 3 },
                    level_req: if ultimate { 6 } else { 1 },
                    behavior_id: behavior_id(index),
                };
                index += 1;
                skill += 1;
            }
            hero += 1;
        }
        entries[index] = AbilityCatalogEntry {
            id: ability_id(index),
            faction: slot,
            hero_slot: 0,
            category: AbilityCategory::Unit,
            levels: 1,
            level_req: 1,
            behavior_id: behavior_id(index),
        };
        index += 1;
        entries[index] = AbilityCatalogEntry {
            id: ability_id(index),
            faction: slot,
            hero_slot: 0,
            category: AbilityCategory::Item,
            levels: 1,
            level_req: 1,
            behavior_id: behavior_id(index),
        };
        index += 1;
        faction += 1;
    }
    entries
}

/// 四席位技能清单的编译期常量表；条目与分类是结构口径，不冒充经典具名技能。
pub const SYNTHETIC_ABILITY_CATALOG: [AbilityCatalogEntry; ABILITY_CATALOG_LEN] =
    synthetic_ability_catalog();

/// 行为原语种类；单一目录按此类型查表分派，避免散落 `if behavior == ...`。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum BehaviorKind {
    Zone,
    Summon,
    Aura,
    PassiveProc,
    Transform,
    PointTarget,
    RemoveBuffCategory,
    ChainBounce,
    RandomDraw,
}

impl BehaviorKind {
    /// 全部最小必要原语的稳定顺序。
    pub const ALL: [Self; BEHAVIOR_KIND_COUNT] = [
        Self::Zone,
        Self::Summon,
        Self::Aura,
        Self::PassiveProc,
        Self::Transform,
        Self::PointTarget,
        Self::RemoveBuffCategory,
        Self::ChainBounce,
        Self::RandomDraw,
    ];

    /// 行为目录哈希使用的稳定字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::Zone => 0,
            Self::Summon => 1,
            Self::Aura => 2,
            Self::PassiveProc => 3,
            Self::Transform => 4,
            Self::PointTarget => 5,
            Self::RemoveBuffCategory => 6,
            Self::ChainBounce => 7,
            Self::RandomDraw => 8,
        }
    }

    /// 由稳定码解码行为种类；未知码显式拒绝。
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::Zone,
            1 => Self::Summon,
            2 => Self::Aura,
            3 => Self::PassiveProc,
            4 => Self::Transform,
            5 => Self::PointTarget,
            6 => Self::RemoveBuffCategory,
            7 => Self::ChainBounce,
            8 => Self::RandomDraw,
            _ => return None,
        })
    }

    /// 行为种类对应的 `EffectKind` 原语；新增原语只扩展本表。
    pub const fn effect_kind(self) -> EffectKind {
        match self {
            Self::Zone => EffectKind::Zone,
            Self::Summon => EffectKind::Summon,
            Self::Aura => EffectKind::AuraBeneficiaries,
            Self::PassiveProc => EffectKind::PassiveProc,
            Self::Transform => EffectKind::Transform,
            Self::PointTarget => EffectKind::PointTarget,
            Self::RemoveBuffCategory => EffectKind::RemoveBuffCategory,
            Self::ChainBounce => EffectKind::ChainBounce,
            Self::RandomDraw => EffectKind::RandomDraw,
        }
    }
}

/// 行为目录条目；`id` 是冻结定义 ID，`kind` 决定查表分派。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct BehaviorCatalogEntry {
    pub id: DefId,
    pub kind: BehaviorKind,
    pub effect_kind: EffectKind,
}

/// 单一行为目录：按行为 ID 和效果种类双向查表，注册新行为即新增条目。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BehaviorCatalog {
    by_id: BTreeMap<DefId, BehaviorCatalogEntry>,
    by_effect: BTreeMap<EffectKind, BehaviorKind>,
}

impl BehaviorCatalog {
    /// 构造九个最小必要原语的默认目录。
    pub fn synthetic() -> Self {
        let mut catalog = Self {
            by_id: BTreeMap::new(),
            by_effect: BTreeMap::new(),
        };
        for (offset, kind) in BehaviorKind::ALL.iter().copied().enumerate() {
            catalog.register(BEHAVIOR_CATALOG_BASE + offset as DefId, kind);
        }
        catalog
    }

    /// 注册一条行为；OCP 扩展点只改数据，不改散落分支。
    pub fn register(&mut self, id: DefId, kind: BehaviorKind) {
        let entry = BehaviorCatalogEntry {
            id,
            kind,
            effect_kind: kind.effect_kind(),
        };
        self.by_id.insert(id, entry);
        self.by_effect.insert(entry.effect_kind, kind);
    }

    /// 按冻结行为 ID 查表分派。
    pub fn dispatch_by_id(&self, id: DefId) -> Option<BehaviorKind> {
        self.by_id.get(&id).map(|entry| entry.kind)
    }

    /// 按效果种类查表分派，供未登记行为 ID 的效果链回落使用。
    pub fn dispatch_by_effect(&self, kind: EffectKind) -> Option<BehaviorKind> {
        self.by_effect.get(&kind).copied()
    }

    /// 读取冻结行为条目，供效果链追加目录原语。
    pub fn entry_by_id(&self, id: DefId) -> Option<&BehaviorCatalogEntry> {
        self.by_id.get(&id)
    }

    /// 全部行为条目的只读稳定顺序。
    pub fn entries(&self) -> &BTreeMap<DefId, BehaviorCatalogEntry> {
        &self.by_id
    }

    /// 显式小端编码进冻结定义哈希。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.by_id.len() as u64);
        for (&id, entry) in &self.by_id {
            hasher.write_u32(id);
            hasher.write_u8(entry.kind.code());
            hasher.write_u8(crate::gameplay::effect_kind_code(entry.effect_kind));
        }
    }

    /// 返回目录差异字段；供冻结定义内容敏感性诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        (self != other).then_some("behavior_catalog")
    }
}

impl Default for BehaviorCatalog {
    fn default() -> Self {
        Self::synthetic()
    }
}

/// 技能系统冻结常量；全部为合成原创整数 `[待核]`，不是经典数值。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbilitySystemConstants {
    pub zone_frames: u32,
    pub summon_frames: u32,
    pub summon_hp: i64,
    pub aura_radius_raw: i64,
    pub proc_modulus: u64,
    pub transform_frames: u32,
    pub chain_bounces: u32,
    pub chain_damage: i64,
    pub point_radius_raw: i64,
    pub rng_mix_salt: u64,
}

/// 默认冻结常量；改任一字段都会改变 `content_hash()`。
pub const SYNTHETIC_ABILITY_SYSTEM_CONSTANTS: AbilitySystemConstants = AbilitySystemConstants {
    zone_frames: 12,
    summon_frames: 30,
    summon_hp: 50,
    aura_radius_raw: 8 << 16,
    proc_modulus: 100,
    transform_frames: 20,
    chain_bounces: 3,
    chain_damage: 5,
    point_radius_raw: 3 << 16,
    rng_mix_salt: 0x9e37_79b9_7f4a_7c15,
};

impl AbilitySystemConstants {
    /// 显式小端编码进冻结定义哈希。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.zone_frames);
        hasher.write_u32(self.summon_frames);
        hasher.write_i64(self.summon_hp);
        hasher.write_i64(self.aura_radius_raw);
        hasher.write_u64(self.proc_modulus);
        hasher.write_u32(self.transform_frames);
        hasher.write_u32(self.chain_bounces);
        hasher.write_i64(self.chain_damage);
        hasher.write_i64(self.point_radius_raw);
        hasher.write_u64(self.rng_mix_salt);
    }

    /// 返回首个差异字段；供冻结定义内容敏感性诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        (self != other).then_some("ability_constants")
    }
}

impl Default for AbilitySystemConstants {
    fn default() -> Self {
        SYNTHETIC_ABILITY_SYSTEM_CONSTANTS
    }
}

/// 冻结技能清单目录；稳定 `DefId` 顺序，不暴露运行期可变入口。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilityCatalog {
    entries: BTreeMap<DefId, AbilityCatalogEntry>,
}

impl AbilityCatalog {
    /// 由编译期常量表构造目录。
    pub fn synthetic() -> Self {
        let entries = SYNTHETIC_ABILITY_CATALOG
            .iter()
            .copied()
            .map(|entry| (entry.id, entry))
            .collect();
        Self { entries }
    }

    /// 按 `DefId` 读取清单条目。
    pub fn get(&self, id: DefId) -> Option<&AbilityCatalogEntry> {
        self.entries.get(&id)
    }

    /// 全部清单条目的只读稳定顺序。
    pub fn entries(&self) -> &BTreeMap<DefId, AbilityCatalogEntry> {
        &self.entries
    }

    /// 显式小端编码进冻结定义哈希。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.entries.len() as u64);
        for (&id, entry) in &self.entries {
            hasher.write_u32(id);
            hasher.write_u8(entry.faction.code());
            hasher.write_u8(entry.hero_slot);
            hasher.write_u8(entry.category.code());
            hasher.write_u8(entry.levels);
            hasher.write_u8(entry.level_req);
            hasher.write_u32(entry.behavior_id);
        }
    }

    /// 返回清单差异字段；供冻结定义内容敏感性诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        (self != other).then_some("ability_catalog")
    }
}

impl Default for AbilityCatalog {
    fn default() -> Self {
        Self::synthetic()
    }
}

/// 技能系统冻结定义集合；只进 `defs_hash` / `content_hash`，不进运行态哈希。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilitySystemDefinitions {
    pub constants: AbilitySystemConstants,
    pub abilities: AbilityCatalog,
    pub behaviors: BehaviorCatalog,
    pub buff_categories: BTreeMap<DefId, BuffCategory>,
}

impl AbilitySystemDefinitions {
    /// 构造默认合成原创定义；所有字段均可由纯数据重建。
    pub fn synthetic() -> Self {
        let behaviors = BehaviorCatalog::synthetic();
        let buff_categories = behaviors
            .entries()
            .iter()
            .map(|(&id, entry)| (id, BuffCategory::from_effect_kind(entry.effect_kind)))
            .collect();
        Self {
            constants: SYNTHETIC_ABILITY_SYSTEM_CONSTANTS,
            abilities: AbilityCatalog::synthetic(),
            behaviors,
            buff_categories,
        }
    }

    /// 显式小端编码进冻结定义哈希；改任一段即改变 `content_hash()`。
    pub fn hash(&self, hasher: &mut StateHasher) {
        self.constants.hash(hasher);
        self.abilities.hash(hasher);
        self.behaviors.hash(hasher);
        hasher.write_u64(self.buff_categories.len() as u64);
        for (&id, &category) in &self.buff_categories {
            hasher.write_u32(id);
            hasher.write_u8(category.code());
        }
    }

    /// 返回首个差异字段；供冻结定义内容敏感性诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.constants != other.constants {
            return Some("ability_constants");
        }
        if self.abilities != other.abilities {
            return Some("ability_catalog");
        }
        if self.behaviors != other.behaviors {
            return Some("behavior_catalog");
        }
        if self.buff_categories != other.buff_categories {
            return Some("buff_categories");
        }
        None
    }
}

impl Default for AbilitySystemDefinitions {
    fn default() -> Self {
        Self::synthetic()
    }
}

/// Buff 分类；用于分类移除与权威分类登记，不承载 fog / 可见性。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum BuffCategory {
    None,
    Physical,
    Magic,
    Control,
    Summon,
    Transform,
}

impl BuffCategory {
    /// 冻结定义与运行态共用的稳定字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Physical => 1,
            Self::Magic => 2,
            Self::Control => 3,
            Self::Summon => 4,
            Self::Transform => 5,
        }
    }

    /// 由稳定码解码分类；未知码显式拒绝。
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::None,
            1 => Self::Physical,
            2 => Self::Magic,
            3 => Self::Control,
            4 => Self::Summon,
            5 => Self::Transform,
            _ => return None,
        })
    }

    /// 由效果种类映射 Buff 分类；新增 `EffectKind` MUST 同步本穷尽匹配。
    pub const fn from_effect_kind(kind: EffectKind) -> Self {
        match kind {
            EffectKind::Damage => Self::Physical,
            EffectKind::Heal => Self::Magic,
            EffectKind::BuffStat => Self::Magic,
            EffectKind::Stun => Self::Control,
            EffectKind::Zone => Self::Magic,
            EffectKind::Summon => Self::Summon,
            EffectKind::AuraBeneficiaries => Self::Magic,
            EffectKind::PassiveProc => Self::Physical,
            EffectKind::Transform => Self::Transform,
            EffectKind::PointTarget => Self::Magic,
            EffectKind::RemoveBuffCategory => Self::Magic,
            EffectKind::ChainBounce => Self::Physical,
            EffectKind::RandomDraw => Self::Physical,
        }
    }
}

impl Default for BuffCategory {
    fn default() -> Self {
        Self::None
    }
}

/// 施法目标形态；点 / 范围 / 方向全部使用 Q47.16 整数原始值。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum AbilityTargetKind {
    Entity(EntityId),
    Point { x: i64, y: i64 },
    Area { x: i64, y: i64, radius_raw: i64 },
    Direction { dx: i64, dy: i64 },
}

impl AbilityTargetKind {
    /// 稳定字节编码，供命令哈希 / 运行态哈希复用。
    pub fn code(self) -> u8 {
        match self {
            Self::Entity(_) => 0,
            Self::Point { .. } => 1,
            Self::Area { .. } => 2,
            Self::Direction { .. } => 3,
        }
    }

    /// 实体目标或回落目标；非实体目标用施法者作为 `CastRuntime.target`。
    pub fn entity_or(self, fallback: EntityId) -> EntityId {
        match self {
            Self::Entity(id) => id,
            _ => fallback,
        }
    }

    /// 目标点坐标；实体 / 方向目标回落到原点。
    pub fn point(self) -> (i64, i64) {
        match self {
            Self::Point { x, y } | Self::Area { x, y, .. } => (x, y),
            Self::Entity(_) | Self::Direction { .. } => (0, 0),
        }
    }

    /// 范围半径；非范围目标回落到冻结常量默认半径。
    pub fn radius_raw(self, fallback: i64) -> i64 {
        match self {
            Self::Area { radius_raw, .. } => radius_raw.max(0),
            _ => fallback.max(0),
        }
    }

    /// 方向向量；非方向目标回落到零向量。
    pub fn direction(self) -> (i64, i64) {
        match self {
            Self::Direction { dx, dy } => (dx, dy),
            _ => (0, 0),
        }
    }

    /// 显式小端编码进 FNV-1a；Option 语义由调用方写 tag。
    pub fn hash(self, hasher: &mut StateHasher) {
        hasher.write_u8(self.code());
        match self {
            Self::Entity(id) => hasher.write_u64(id.value()),
            Self::Point { x, y } => {
                hasher.write_i64(x);
                hasher.write_i64(y);
            }
            Self::Area { x, y, radius_raw } => {
                hasher.write_i64(x);
                hasher.write_i64(y);
                hasher.write_i64(radius_raw);
            }
            Self::Direction { dx, dy } => {
                hasher.write_i64(dx);
                hasher.write_i64(dy);
            }
        }
    }
}

impl Default for AbilityTargetKind {
    fn default() -> Self {
        Self::Entity(EntityId::NONE)
    }
}

/// 持续 AOE / zone 的权威运行态。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ZoneState {
    pub id: EntityId,
    pub source: EntityId,
    pub ability: DefId,
    pub owner: u8,
    pub x: i64,
    pub y: i64,
    pub radius_raw: i64,
    pub remaining: u32,
    pub beneficiaries: BTreeSet<EntityId>,
}

/// 召唤实体的权威运行态；实体本身仍由 `EntityStore` / `units` 承载。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SummonState {
    pub summon: EntityId,
    pub owner: EntityId,
    pub player: u8,
    pub ability: DefId,
    pub remaining: u32,
}

/// 光环受益集；稳定 `EntityId` 集合，不存 fog / 可见性结果。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AuraBeneficiarySet {
    pub source: EntityId,
    pub ability: DefId,
    pub beneficiaries: BTreeSet<EntityId>,
}

/// 被动 proc 订阅；`remaining == u32::MAX` 表示常驻订阅。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PassiveProcState {
    pub subscribed_frame: u64,
    pub remaining: u32,
    pub random: bool,
}

/// 变身 / 状态切换的权威运行态。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TransformState {
    pub ability: DefId,
    pub state_code: u8,
    pub remaining: u32,
}

/// Buff 分类移除与链接弹射的链接状态。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChainState {
    pub id: EntityId,
    pub source: EntityId,
    pub target: EntityId,
    pub ability: DefId,
    pub bounces: u32,
    pub remaining: u32,
}

/// 点 / 范围 / 方向目标的事实运行态；表现层只读派生不得反写。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PointTargetState {
    pub caster: EntityId,
    pub ability: DefId,
    pub target: AbilityTargetKind,
    pub frame: u64,
}

/// 进行中施法的权威登记；恢复后从 N+1 继续不重复施放。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PendingAbilityCast {
    pub caster: EntityId,
    pub ability: DefId,
    pub target: AbilityTargetKind,
    pub player: u8,
    pub frame: u64,
}

/// 技能随机消费顺序；主 LCG 每帧只推进一次，本记录只编码派生值。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbilityRngDraw {
    pub seq: u64,
    pub frame: u64,
    pub unit: EntityId,
    pub ability: DefId,
    pub rng_before: u64,
    pub rng_after: u64,
    pub roll: u64,
    pub behavior: BehaviorKind,
}

/// 全部新增权威运行态字段名；四处同步与反向验证以此清单为准。
pub const ABILITY_HASHED_FIELDS: &[&str] = &[
    "ability_hero_skill_points",
    "ability_levels",
    "ability_zones",
    "ability_summons",
    "ability_aura_beneficiaries",
    "ability_passive_procs",
    "ability_transformations",
    "ability_buff_categories",
    "ability_chains",
    "ability_point_targets",
    "ability_pending_casts",
    "ability_rng_draws",
    "ability_next_zone_id",
    "ability_next_chain_id",
    "ability_next_rng_seq",
];

/// P11 技能系统权威运行态；全部字段按稳定 `EntityId` / `DefId` / `seq` 顺序。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilitySystemState {
    /// 英雄技能点（`EntityId` → 剩余点数）。
    pub hero_skill_points: BTreeMap<EntityId, u32>,
    /// 已学技能等级（`(EntityId, DefId)` → 等级）。
    pub ability_levels: BTreeMap<(EntityId, DefId), u32>,
    /// 持续 AOE / zone（稳定 zone ID → 状态）。
    pub zones: BTreeMap<EntityId, ZoneState>,
    /// 召唤登记（召唤实体 `EntityId` → 状态）。
    pub summons: BTreeMap<EntityId, SummonState>,
    /// 光环受益集（来源 `EntityId` → 受益集）。
    pub aura_beneficiaries: BTreeMap<EntityId, AuraBeneficiarySet>,
    /// 被动 proc 订阅（`(EntityId, DefId)` → 状态）。
    pub passive_procs: BTreeMap<(EntityId, DefId), PassiveProcState>,
    /// 变身 / 状态切换（目标 `EntityId` → 状态）。
    pub transformations: BTreeMap<EntityId, TransformState>,
    /// Buff 分类登记（单位 `EntityId` → 分类集合）。
    pub buff_categories: BTreeMap<EntityId, BTreeSet<BuffCategory>>,
    /// 链接弹射（稳定链接 ID → 状态）。
    pub chains: BTreeMap<EntityId, ChainState>,
    /// 点 / 范围 / 方向目标（施法者 `EntityId` → 目标）。
    pub point_targets: BTreeMap<EntityId, PointTargetState>,
    /// 进行中施法（施法者 `EntityId` → 冻结输入）。
    pub pending_casts: BTreeMap<EntityId, PendingAbilityCast>,
    /// 随机消费顺序（稳定 `seq` → 记录）。
    pub rng_draws: BTreeMap<u64, AbilityRngDraw>,
    /// 下一个 zone 稳定标识。
    pub next_zone_id: u64,
    /// 下一个链接稳定标识。
    pub next_chain_id: u64,
    /// 下一个随机消费序号。
    pub next_rng_seq: u64,
}

impl Default for AbilitySystemState {
    fn default() -> Self {
        Self {
            hero_skill_points: BTreeMap::new(),
            ability_levels: BTreeMap::new(),
            zones: BTreeMap::new(),
            summons: BTreeMap::new(),
            aura_beneficiaries: BTreeMap::new(),
            passive_procs: BTreeMap::new(),
            transformations: BTreeMap::new(),
            buff_categories: BTreeMap::new(),
            chains: BTreeMap::new(),
            point_targets: BTreeMap::new(),
            pending_casts: BTreeMap::new(),
            rng_draws: BTreeMap::new(),
            next_zone_id: ABILITY_ZONE_ID_BASE,
            next_chain_id: ABILITY_CHAIN_ID_BASE,
            next_rng_seq: 0,
        }
    }
}

fn hash_entity_set(hasher: &mut StateHasher, set: &BTreeSet<EntityId>) {
    hasher.write_u64(set.len() as u64);
    for &id in set {
        hasher.write_u64(id.value());
    }
}

impl AbilitySystemState {
    /// 全部权威字段显式小端编码进 FNV-1a；顺序与 `first_difference()` 一致。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.hero_skill_points.len() as u64);
        for (&id, &points) in &self.hero_skill_points {
            hasher.write_u64(id.value());
            hasher.write_u32(points);
        }

        hasher.write_u64(self.ability_levels.len() as u64);
        for (key, &level) in &self.ability_levels {
            hasher.write_u64(key.0.value());
            hasher.write_u32(key.1);
            hasher.write_u32(level);
        }

        hasher.write_u64(self.zones.len() as u64);
        for (&id, zone) in &self.zones {
            hasher.write_u64(id.value());
            hasher.write_u64(zone.id.value());
            hasher.write_u64(zone.source.value());
            hasher.write_u32(zone.ability);
            hasher.write_u8(zone.owner);
            hasher.write_i64(zone.x);
            hasher.write_i64(zone.y);
            hasher.write_i64(zone.radius_raw);
            hasher.write_u32(zone.remaining);
            hash_entity_set(hasher, &zone.beneficiaries);
        }

        hasher.write_u64(self.summons.len() as u64);
        for (&id, summon) in &self.summons {
            hasher.write_u64(id.value());
            hasher.write_u64(summon.summon.value());
            hasher.write_u64(summon.owner.value());
            hasher.write_u8(summon.player);
            hasher.write_u32(summon.ability);
            hasher.write_u32(summon.remaining);
        }

        hasher.write_u64(self.aura_beneficiaries.len() as u64);
        for (&id, aura) in &self.aura_beneficiaries {
            hasher.write_u64(id.value());
            hasher.write_u64(aura.source.value());
            hasher.write_u32(aura.ability);
            hash_entity_set(hasher, &aura.beneficiaries);
        }

        hasher.write_u64(self.passive_procs.len() as u64);
        for (key, state) in &self.passive_procs {
            hasher.write_u64(key.0.value());
            hasher.write_u32(key.1);
            hasher.write_u64(state.subscribed_frame);
            hasher.write_u32(state.remaining);
            hasher.write_u8(u8::from(state.random));
        }

        hasher.write_u64(self.transformations.len() as u64);
        for (&id, state) in &self.transformations {
            hasher.write_u64(id.value());
            hasher.write_u32(state.ability);
            hasher.write_u8(state.state_code);
            hasher.write_u32(state.remaining);
        }

        hasher.write_u64(self.buff_categories.len() as u64);
        for (&id, categories) in &self.buff_categories {
            hasher.write_u64(id.value());
            hasher.write_u64(categories.len() as u64);
            for &category in categories {
                hasher.write_u8(category.code());
            }
        }

        hasher.write_u64(self.chains.len() as u64);
        for (&id, chain) in &self.chains {
            hasher.write_u64(id.value());
            hasher.write_u64(chain.id.value());
            hasher.write_u64(chain.source.value());
            hasher.write_u64(chain.target.value());
            hasher.write_u32(chain.ability);
            hasher.write_u32(chain.bounces);
            hasher.write_u32(chain.remaining);
        }

        hasher.write_u64(self.point_targets.len() as u64);
        for (&id, point) in &self.point_targets {
            hasher.write_u64(id.value());
            hasher.write_u64(point.caster.value());
            hasher.write_u32(point.ability);
            point.target.hash(hasher);
            hasher.write_u64(point.frame);
        }

        hasher.write_u64(self.pending_casts.len() as u64);
        for (&id, pending) in &self.pending_casts {
            hasher.write_u64(id.value());
            hasher.write_u64(pending.caster.value());
            hasher.write_u32(pending.ability);
            pending.target.hash(hasher);
            hasher.write_u8(pending.player);
            hasher.write_u64(pending.frame);
        }

        hasher.write_u64(self.rng_draws.len() as u64);
        for (&seq, draw) in &self.rng_draws {
            hasher.write_u64(seq);
            hasher.write_u64(draw.seq);
            hasher.write_u64(draw.frame);
            hasher.write_u64(draw.unit.value());
            hasher.write_u32(draw.ability);
            hasher.write_u64(draw.rng_before);
            hasher.write_u64(draw.rng_after);
            hasher.write_u64(draw.roll);
            hasher.write_u8(draw.behavior.code());
        }

        hasher.write_u64(self.next_zone_id);
        hasher.write_u64(self.next_chain_id);
        hasher.write_u64(self.next_rng_seq);
    }

    /// 按同一稳定顺序返回首个差异字段，便于 desync 诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.hero_skill_points != other.hero_skill_points {
            return Some("ability_hero_skill_points");
        }
        if self.ability_levels != other.ability_levels {
            return Some("ability_levels");
        }
        if self.zones != other.zones {
            return Some("ability_zones");
        }
        if self.summons != other.summons {
            return Some("ability_summons");
        }
        if self.aura_beneficiaries != other.aura_beneficiaries {
            return Some("ability_aura_beneficiaries");
        }
        if self.passive_procs != other.passive_procs {
            return Some("ability_passive_procs");
        }
        if self.transformations != other.transformations {
            return Some("ability_transformations");
        }
        if self.buff_categories != other.buff_categories {
            return Some("ability_buff_categories");
        }
        if self.chains != other.chains {
            return Some("ability_chains");
        }
        if self.point_targets != other.point_targets {
            return Some("ability_point_targets");
        }
        if self.pending_casts != other.pending_casts {
            return Some("ability_pending_casts");
        }
        if self.rng_draws != other.rng_draws {
            return Some("ability_rng_draws");
        }
        if self.next_zone_id != other.next_zone_id {
            return Some("ability_next_zone_id");
        }
        if self.next_chain_id != other.next_chain_id {
            return Some("ability_next_chain_id");
        }
        if self.next_rng_seq != other.next_rng_seq {
            return Some("ability_next_rng_seq");
        }
        None
    }
}
