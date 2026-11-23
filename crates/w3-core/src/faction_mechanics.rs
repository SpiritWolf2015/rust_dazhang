//! 族特色机制（P9 / Q2 / Q4）：冻结常量（定义层）与运行态（权威状态）。
//!
//! 本模块严格区分两层，避免「定义层内容变化」与「新增权威状态」混同
//! （ADR 0002 §6-8 / `design.md` D2 / D4）：
//! - [`FactionMechanicsConstants`] 是**冻结定义层内容**，9 个合成原创
//!   编译期整数 `[待核]`，经 `compute_defs_hash` 进入 `defs_hash` /
//!   `content_hash` 快照头部；MUST NOT 进入 `compute_state_hash()`。
//! - [`FactionMechanicsState`] 是 `RtsMatch` 顶层**权威运行态**，全落
//!   `BTreeMap` / `BTreeSet` 确定性有序容器、按稳定 `EntityId` 顺序遍历，
//!   四处同步进 FNV-1a 状态哈希 / `Snapshot` / `first_difference()` /
//!   `restore_snapshot`（`CODEBUDDY.md §5`）。
//!
//! R10.5 / §0.3 T3 边界（Q2' 已拍板）：迷雾 / 视野**可见性** MUST
//! 表现层只读派生；本模块只登记「隐身目标判定 / 反隐 / 视线阻挡 /
//! 昼夜相位」这类**玩法机制输入与判定结果**，MUST NOT 存放当前可见 /
//! 已探索 / 视野可见性网格等 fog 派生结果。vera20k fog 进哈希属于
//! 对照差异 D1，明确不借鉴。
//!
//! 回血条件建模为定义层枚举 [`RegenCondition`]（`NONE / ALWAYS /
//! BLIGHT / DAY / NIGHT`，参考对照 R9 只借语义、AGPLv3 零代码复制），
//! 逐 30 Hz 整数帧查表结算；MUST NOT 用种族名硬编码或族别分类
//! 白名单。`w3-core` 全仓无族别分类字段（Q6），族别仍由上层
//! `DefId` 段表达，本模块不引入任何族别标签。
//!
//! 两套昼夜常量口径未统一：本模块 `day_frames = 600` /
//! `night_frames = 480`（与 client `P9_DAY_FRAMES` / `P9_NIGHT_FRAMES`
//! 一一对应语义），而 [`crate::creep::CreepConstants`] 的
//! `day_frames = 600` / `night_frames = 600` 是另一套独立冻结常量。
//! 两者按 Q3' 已拍板**如实并列登记为 `[待核]` 由人核对**，
//! MUST NOT 静默统一、MUST NOT 改活文档 `WC3-MELEE-RULES.md`（§14）。

use std::collections::{BTreeMap, BTreeSet};

use crate::entity::EntityId;
use crate::gameplay::DefId;
use crate::state_hash::StateHasher;

// ---------------------------------------------------------------------------
// 昼夜相位（定义层枚举）
// ---------------------------------------------------------------------------

/// 昼夜相位离散值（R10.2）。全部以显式 [`DayNightPhase::code`] 字节
/// 编码进哈希 / 快照 / 差异诊断，不依赖枚举判别式内存布局。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DayNightPhase {
    /// 白昼。
    Day,
    /// 夜晚。
    Night,
}

impl DayNightPhase {
    /// 稳定的单字节编码；值域固定为 1 / 2。
    pub const fn code(self) -> u8 {
        match self {
            DayNightPhase::Day => 1,
            DayNightPhase::Night => 2,
        }
    }

    /// 从显式编码还原相位；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(DayNightPhase::Day),
            2 => Some(DayNightPhase::Night),
            _ => None,
        }
    }

    /// 相位切换（昼→夜、夜→昼），整数帧阈值到达时调用。
    pub const fn toggled(self) -> Self {
        match self {
            DayNightPhase::Day => DayNightPhase::Night,
            DayNightPhase::Night => DayNightPhase::Day,
        }
    }
}

impl Default for DayNightPhase {
    fn default() -> Self {
        DayNightPhase::Day
    }
}

// ---------------------------------------------------------------------------
// 回血条件（定义层枚举，R10.3 / 参考对照 R9）
// ---------------------------------------------------------------------------

/// 回血条件定义层枚举（R10.3 `WC3-MELEE-RULES.md:224`）。
///
/// 逐 tick 由本枚举查表决定是否结算回血，MUST NOT 用族别名硬编码 /
/// 族别条件分支白名单（参考对照 R9 只借 `regenType` 机制语义，
/// AGPLv3 零代码复制）。全部以显式 [`RegenCondition::code`] 字节编码。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegenCondition {
    /// 不回血。
    None,
    /// 昼夜均回血。
    Always,
    /// 仅当腐地网格非空时回血（合成 `[待核]` 规则）。
    Blight,
    /// 仅白昼回血。
    Day,
    /// 仅夜晚回血。
    Night,
}

impl RegenCondition {
    /// 稳定的单字节编码；值域固定为 0 / 1 / 2 / 3 / 4。
    pub const fn code(self) -> u8 {
        match self {
            RegenCondition::None => 0,
            RegenCondition::Always => 1,
            RegenCondition::Blight => 2,
            RegenCondition::Day => 3,
            RegenCondition::Night => 4,
        }
    }

    /// 从显式编码还原回血条件；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(RegenCondition::None),
            1 => Some(RegenCondition::Always),
            2 => Some(RegenCondition::Blight),
            3 => Some(RegenCondition::Day),
            4 => Some(RegenCondition::Night),
            _ => None,
        }
    }

    /// 逐 tick 查表：根据整数帧派生的昼夜相位与腐地登记判定本帧
    /// 是否结算回血。全整数、无墙钟、无浮点。
    pub const fn applies(self, phase: DayNightPhase, blight_present: bool) -> bool {
        match self {
            RegenCondition::None => false,
            RegenCondition::Always => true,
            RegenCondition::Blight => blight_present,
            RegenCondition::Day => matches!(phase, DayNightPhase::Day),
            RegenCondition::Night => matches!(phase, DayNightPhase::Night),
        }
    }
}

impl Default for RegenCondition {
    fn default() -> Self {
        RegenCondition::None
    }
}

// ---------------------------------------------------------------------------
// 冻结常量（定义层内容）
// ---------------------------------------------------------------------------

/// 族特色机制冻结常量集合（9 个合成 / 原创编译期整数常量 `[待核]`）。
///
/// 字段与 client `crates/w3-client/src/melee/defs.rs` 的 9 个 `P9_*`
/// 编译期常量一一对应语义（Q3' 已拍板）：昼夜时长 / 腐地扩张 /
/// 尸体衰减 / 腐地回血速率 / 月亮井恢复量 / 缠绕·诅咒金矿间隔 /
/// 隐身反隐半径。全部为合成原创整数，不冒充原版、不读本机导出、
/// 零文件 IO（ADR 0001 / ADR 0004 / §14）。
///
/// 改任一常量即改变 [`crate::gameplay::FrozenDefinitions::content_hash`]
/// 并进入快照头部（`defs_hash` / `content_hash`）；这是**定义层内容变化**，
/// 与 [`FactionMechanicsState`] 新增权威状态严格区分，
/// MUST NOT 单独进入 `compute_state_hash()`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FactionMechanicsConstants {
    /// 昼夜循环：白昼时长（整数逻辑帧），`[待核]`。
    pub day_frames: u32,
    /// 昼夜循环：夜晚时长（整数逻辑帧），`[待核]`。
    pub night_frames: u32,
    /// 腐地扩张：相邻格扩张间隔（整数逻辑帧），`[待核]`。
    pub blight_expansion_frames: u32,
    /// 尸体衰减：单个尸体存活时长（整数逻辑帧），`[待核]`。
    pub corpse_decay_frames: u32,
    /// 腐地回血：每逻辑帧回复量（整数），`[待核]`。
    pub blight_regen_per_frame: i64,
    /// 月亮井：单次回复量（整数），`[待核]`。
    pub moon_well_restore: i64,
    /// 缠绕金矿：采集结算间隔（整数逻辑帧），`[待核]`。
    pub entangle_mine_interval: u32,
    /// 诅咒金矿：采集结算间隔（整数逻辑帧），`[待核]`。
    pub haunted_mine_interval: u32,
    /// 隐身反隐：反隐半径（整数世界单位），`[待核]`。
    pub stealth_detect_radius: i64,
}

/// 合成 / 原创的族特色机制冻结常量 `[待核]`。
///
/// 取值与 client `P9_DAY_FRAMES` / `P9_NIGHT_FRAMES` /
/// `P9_BLIGHT_EXPANSION_FRAMES` / `P9_CORPSE_DECAY_FRAMES` /
/// `P9_BLIGHT_REGEN_PER_FRAME` / `P9_MOON_WELL_RESTORE` /
/// `P9_ENTANGLE_MINE_INTERVAL` / `P9_HAUNTED_MINE_INTERVAL` /
/// `P9_STEALTH_DETECT_RADIUS` 一致；core MUST NOT 反向依赖 client，
/// 故在本模块独立权威定义（Q3'）。数值不冒充经典规则、不来自任何资产。
pub const SYNTHETIC_FACTION_MECHANICS: FactionMechanicsConstants = FactionMechanicsConstants {
    day_frames: 600,
    night_frames: 480,
    blight_expansion_frames: 90,
    corpse_decay_frames: 900,
    blight_regen_per_frame: 1,
    moon_well_restore: 50,
    entangle_mine_interval: 60,
    haunted_mine_interval: 60,
    stealth_detect_radius: 8,
};

impl Default for FactionMechanicsConstants {
    fn default() -> Self {
        SYNTHETIC_FACTION_MECHANICS
    }
}

impl FactionMechanicsConstants {
    /// 显式小端逐字段编码进定义哈希（`compute_defs_hash` 新增段）。
    /// 任一常量变化都改变结果（Q3 / tasks 2.3 / 2.4）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.day_frames);
        hasher.write_u32(self.night_frames);
        hasher.write_u32(self.blight_expansion_frames);
        hasher.write_u32(self.corpse_decay_frames);
        hasher.write_i64(self.blight_regen_per_frame);
        hasher.write_i64(self.moon_well_restore);
        hasher.write_u32(self.entangle_mine_interval);
        hasher.write_u32(self.haunted_mine_interval);
        hasher.write_i64(self.stealth_detect_radius);
    }

    /// 按同一稳定顺序返回首个差异字段，供 `defs_hash` 内容敏感性诊断。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.day_frames != other.day_frames {
            return Some("faction_const_day_frames");
        }
        if self.night_frames != other.night_frames {
            return Some("faction_const_night_frames");
        }
        if self.blight_expansion_frames != other.blight_expansion_frames {
            return Some("faction_const_blight_expansion_frames");
        }
        if self.corpse_decay_frames != other.corpse_decay_frames {
            return Some("faction_const_corpse_decay_frames");
        }
        if self.blight_regen_per_frame != other.blight_regen_per_frame {
            return Some("faction_const_blight_regen_per_frame");
        }
        if self.moon_well_restore != other.moon_well_restore {
            return Some("faction_const_moon_well_restore");
        }
        if self.entangle_mine_interval != other.entangle_mine_interval {
            return Some("faction_const_entangle_mine_interval");
        }
        if self.haunted_mine_interval != other.haunted_mine_interval {
            return Some("faction_const_haunted_mine_interval");
        }
        if self.stealth_detect_radius != other.stealth_detect_radius {
            return Some("faction_const_stealth_detect_radius");
        }
        None
    }
}

// ---------------------------------------------------------------------------
// 运行态支撑类型（全部 Copy / Eq，可重建、可显式编码）
// ---------------------------------------------------------------------------

/// 金矿绑定类别（R1.5 缠绕 / 诅咒金矿）。显式字节编码。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MineKind {
    /// 不死族被诅咒的金矿（侍僧环绕获取）。
    Haunted,
    /// 暗夜精灵小精灵缠绕金矿。
    Entangled,
}

impl MineKind {
    /// 稳定单字节编码，值域固定为 1 / 2。
    pub const fn code(self) -> u8 {
        match self {
            MineKind::Haunted => 1,
            MineKind::Entangled => 2,
        }
    }

    /// 从显式编码还原；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(MineKind::Haunted),
            2 => Some(MineKind::Entangled),
            _ => None,
        }
    }
}

impl Default for MineKind {
    fn default() -> Self {
        MineKind::Haunted
    }
}

/// 单个金矿绑定的权威记录（稳定 `EntityId` 键）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MineRecord {
    /// 金矿绑定类别（数据驱动，非族名白名单）。
    pub kind: MineKind,
    /// 距下次采集结算的剩余整数帧。
    pub progress_frames: u32,
}

/// 隐身目标判定的权威记录（R10.4 / S3）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StealthRecord {
    /// 当前是否处于隐身（隐身目标判定为玩法机制，非 fog 可见性）。
    pub cloaked: bool,
    /// 是否仅夜间隐身（S3 暗夜夜间隐身特色）。
    pub night_only: bool,
}

/// 回血登记的权威记录（R10.3，定义层枚举逐 tick 查表）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RegenRecord {
    /// 定义层回血条件枚举（非种族名硬编码）。
    pub condition: RegenCondition,
    /// 已累计回血量（整数，`SimFixed` 同源整数域）。
    pub accumulated: i64,
}

/// 昼夜时钟的权威运行态（R10.2 全量循环，整数帧派生）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DayNightClock {
    /// 当前相位。
    pub phase: DayNightPhase,
    /// 当前相位已过整数逻辑帧。
    pub elapsed_frames: u32,
    /// 已完成的昼夜周期数。
    pub cycles: u32,
}

/// 族特色机制事实登记类别（幂等事实，稳定 `seq` 顺序）。
///
/// 只登记**玩法机制事实**，MUST NOT 登记迷雾 / 视野可见性派生结果
/// （R10.5 / §0.3 T3）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactionFactKind {
    /// 腐地完成一格扩张。
    BlightExpanded,
    /// 尸体衰减到 0 后移除。
    CorpseDecayed,
    /// 金矿完成一次采集结算。
    MineSettled,
    /// 昼夜相位切换。
    PhaseChanged,
    /// 隐身目标被反隐揭示。
    StealthRevealed,
    /// 夜间隐身生效 / 解除。
    NightCloakApplied,
    /// 回血条件本帧首次生效。
    RegenApplied,
    /// 野怪睡眠状态切换。
    CreepSleepChanged,
}

impl FactionFactKind {
    /// 稳定单字节编码，值域固定为 1..=8。
    pub const fn code(self) -> u8 {
        match self {
            FactionFactKind::BlightExpanded => 1,
            FactionFactKind::CorpseDecayed => 2,
            FactionFactKind::MineSettled => 3,
            FactionFactKind::PhaseChanged => 4,
            FactionFactKind::StealthRevealed => 5,
            FactionFactKind::NightCloakApplied => 6,
            FactionFactKind::RegenApplied => 7,
            FactionFactKind::CreepSleepChanged => 8,
        }
    }
}

/// 单条族特色机制事实（无 payload 游戏状态，稳定 `seq` 顺序）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FactionFact {
    /// 稳定事实序号。
    pub seq: u64,
    /// 结算整数帧号。
    pub frame: u64,
    /// 关联实体（相位切换用 `EntityId::NONE`）。
    pub entity: EntityId,
    /// 事实类别。
    pub kind: FactionFactKind,
}

// ---------------------------------------------------------------------------
// 族特色机制顶层权威运行态（RtsMatch / Snapshot 顶层字段）
// ---------------------------------------------------------------------------

/// 9 类族特色机制 + Q4 全量机制的顶层权威运行态（Q2 / Q4 已拍板）。
///
/// 全部字段落 `BTreeMap` / `BTreeSet` 确定性有序容器或标量整数，
/// 按稳定 `EntityId` / `(i32, i32)` / `seq` 顺序遍历；无 `HashMap`
/// 迭代顺序、无墙钟、无 `f32` / `f64`、无表现层独立随机源
/// （ADR 0002 §1-4）。
///
/// 覆盖 Q2 九组：腐地建造约束（R4.6）/ 尸体·食尸回血·召唤骷髅
/// （`:119`）/ 诅咒金矿（R1.5）/ 古树状态 / 昼夜相位 / 隐身目标判定
/// / 月亮井 / 缠绕金矿 / 夜间隐身；并覆盖 Q4 六类机制（隐身反隐
/// R10.4 / 昼夜循环全量 R10.2 / R10.3 回血 / R5.6 视线阻挡 /
/// S3 暗夜夜间隐身 / R8.4 野怪睡眠）的逐帧确定性结算。
///
/// **R10.5 边界**：`detected` 是「隐身目标是否被反隐单位揭示」的
/// 玩法判定结果，`sight_blockers` 是「哪些实体阻挡视线」的机制输入，
/// 二者都不是迷雾当前可见 / 已探索 / 视野可见性网格；后者 MUST
/// 表现层只读派生，MUST NOT 进权威状态 / 哈希 / 快照 /
/// `first_difference()`。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FactionMechanicsState {
    /// R4.6 腐地覆盖格（建造合法性查表，稳定 `(x, y)` 顺序）。
    pub blight_cells: BTreeSet<(i32, i32)>,
    /// 腐地扩张源（建筑 `EntityId` → 已过整数帧进度）。
    pub blight_progress: BTreeMap<EntityId, u32>,
    /// `:119` 尸体（尸体 `EntityId` → 剩余衰减整数帧）。
    pub corpses: BTreeMap<EntityId, u32>,
    /// `:119` 食尸回血 / 召唤骷髅消费计数（`EntityId` → 次数）。
    pub corpse_consumptions: BTreeMap<EntityId, u32>,
    /// R1.5 诅咒 / 缠绕金矿绑定记录（金矿 `EntityId` → 记录）。
    pub mines: BTreeMap<EntityId, MineRecord>,
    /// 古树扎根状态（`EntityId` → `true` 扎根不可移动 / `false` 拔根）。
    pub rooted_ancients: BTreeMap<EntityId, bool>,
    /// 月亮井（`EntityId` → 剩余恢复量，整数）。
    pub moon_wells: BTreeMap<EntityId, i64>,
    /// R10.2 昼夜时钟（整数帧派生，权威登记）。
    pub day_night: DayNightClock,
    /// R10.4 / S3 隐身目标判定记录（`EntityId` → 记录）。
    pub stealth: BTreeMap<EntityId, StealthRecord>,
    /// R10.4 反隐单位登记（稳定 `EntityId` 顺序）。
    pub detectors: BTreeSet<EntityId>,
    /// R10.4 本帧被反隐揭示的隐身目标（玩法判定，非 fog 可见性）。
    pub detected: BTreeSet<EntityId>,
    /// S3 夜间隐身候选单位（夜间自动隐身）。
    pub night_cloak: BTreeSet<EntityId>,
    /// R10.3 回血登记（定义层 `RegenCondition` 枚举逐 tick 查表）。
    pub regen: BTreeMap<EntityId, RegenRecord>,
    /// R5.6 视线阻挡机制输入登记（稳定 `EntityId` 顺序）。
    pub sight_blockers: BTreeSet<EntityId>,
    /// R8.4 野怪睡眠登记（`true` 夜间睡眠 / `false` 夜间清醒例外）。
    pub creep_sleep: BTreeMap<EntityId, bool>,
    /// R8.4 本帧睡眠结算结果（稳定 `EntityId` 顺序）。
    pub sleeping_creeps: BTreeSet<EntityId>,
    /// 幂等事实登记（稳定 `seq` 顺序）。
    pub facts: BTreeMap<u64, FactionFact>,
    /// 下一个事实序号。
    pub next_fact_seq: u64,
}

/// 全部进入哈希 / 快照 / 差异诊断的字段名（稳定顺序）。
/// 供 fog 路径隔离回归与反向验证枚举，MUST NOT 含 fog / 可见性字段。
pub const FACTION_HASHED_FIELDS: &[&str] = &[
    "faction_blight_cells",
    "faction_blight_progress",
    "faction_corpses",
    "faction_corpse_consumptions",
    "faction_mines",
    "faction_rooted_ancients",
    "faction_moon_wells",
    "faction_day_night",
    "faction_stealth",
    "faction_detectors",
    "faction_detected",
    "faction_night_cloak",
    "faction_regen",
    "faction_sight_blockers",
    "faction_creep_sleep",
    "faction_sleeping_creeps",
    "faction_facts",
    "faction_next_fact_seq",
];

/// 显式小端编码辅助：网格格 `(i32, i32)`。
fn hash_cell(hasher: &mut StateHasher, x: i32, y: i32) {
    hasher.write_i64(i64::from(x));
    hasher.write_i64(i64::from(y));
}

/// 显式小端编码辅助：稳定 `EntityId` 集合。
fn hash_entity_set(hasher: &mut StateHasher, set: &BTreeSet<EntityId>) {
    hasher.write_u64(set.len() as u64);
    for &id in set {
        hasher.write_u64(id.value());
    }
}

impl FactionMechanicsState {
    /// 全部权威状态显式小端编码进 FNV-1a；字段顺序与
    /// [`FactionMechanicsState::first_difference`] 完全一致。
    /// 新增字段 MUST 同步接入本方法、`first_difference()` 与快照
    /// （`CODEBUDDY.md §5` 硬门禁）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.blight_cells.len() as u64);
        for &(x, y) in &self.blight_cells {
            hash_cell(hasher, x, y);
        }

        hasher.write_u64(self.blight_progress.len() as u64);
        for (&id, &progress) in &self.blight_progress {
            hasher.write_u64(id.value());
            hasher.write_u32(progress);
        }

        hasher.write_u64(self.corpses.len() as u64);
        for (&id, &remaining) in &self.corpses {
            hasher.write_u64(id.value());
            hasher.write_u32(remaining);
        }

        hasher.write_u64(self.corpse_consumptions.len() as u64);
        for (&id, &count) in &self.corpse_consumptions {
            hasher.write_u64(id.value());
            hasher.write_u32(count);
        }

        hasher.write_u64(self.mines.len() as u64);
        for (&id, &mine) in &self.mines {
            hasher.write_u64(id.value());
            hasher.write_u8(mine.kind.code());
            hasher.write_u32(mine.progress_frames);
        }

        hasher.write_u64(self.rooted_ancients.len() as u64);
        for (&id, &rooted) in &self.rooted_ancients {
            hasher.write_u64(id.value());
            hasher.write_u8(u8::from(rooted));
        }

        hasher.write_u64(self.moon_wells.len() as u64);
        for (&id, &restore) in &self.moon_wells {
            hasher.write_u64(id.value());
            hasher.write_i64(restore);
        }

        hasher.write_u8(self.day_night.phase.code());
        hasher.write_u32(self.day_night.elapsed_frames);
        hasher.write_u32(self.day_night.cycles);

        hasher.write_u64(self.stealth.len() as u64);
        for (&id, &record) in &self.stealth {
            hasher.write_u64(id.value());
            hasher.write_u8(u8::from(record.cloaked));
            hasher.write_u8(u8::from(record.night_only));
        }

        hash_entity_set(hasher, &self.detectors);
        hash_entity_set(hasher, &self.detected);
        hash_entity_set(hasher, &self.night_cloak);

        hasher.write_u64(self.regen.len() as u64);
        for (&id, &record) in &self.regen {
            hasher.write_u64(id.value());
            hasher.write_u8(record.condition.code());
            hasher.write_i64(record.accumulated);
        }

        hash_entity_set(hasher, &self.sight_blockers);

        hasher.write_u64(self.creep_sleep.len() as u64);
        for (&id, &sleeps) in &self.creep_sleep {
            hasher.write_u64(id.value());
            hasher.write_u8(u8::from(sleeps));
        }

        hash_entity_set(hasher, &self.sleeping_creeps);

        hasher.write_u64(self.facts.len() as u64);
        for (&seq, &fact) in &self.facts {
            hasher.write_u64(seq);
            hasher.write_u64(fact.frame);
            hasher.write_u64(fact.entity.value());
            hasher.write_u8(fact.kind.code());
        }
        hasher.write_u64(self.next_fact_seq);
    }

    /// 按同一稳定顺序返回首个差异字段名，便于 desync 诊断（ADR 0002 §6）。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.blight_cells != other.blight_cells {
            return Some("faction_blight_cells");
        }
        if self.blight_progress != other.blight_progress {
            return Some("faction_blight_progress");
        }
        if self.corpses != other.corpses {
            return Some("faction_corpses");
        }
        if self.corpse_consumptions != other.corpse_consumptions {
            return Some("faction_corpse_consumptions");
        }
        if self.mines != other.mines {
            return Some("faction_mines");
        }
        if self.rooted_ancients != other.rooted_ancients {
            return Some("faction_rooted_ancients");
        }
        if self.moon_wells != other.moon_wells {
            return Some("faction_moon_wells");
        }
        if self.day_night != other.day_night {
            return Some("faction_day_night");
        }
        if self.stealth != other.stealth {
            return Some("faction_stealth");
        }
        if self.detectors != other.detectors {
            return Some("faction_detectors");
        }
        if self.detected != other.detected {
            return Some("faction_detected");
        }
        if self.night_cloak != other.night_cloak {
            return Some("faction_night_cloak");
        }
        if self.regen != other.regen {
            return Some("faction_regen");
        }
        if self.sight_blockers != other.sight_blockers {
            return Some("faction_sight_blockers");
        }
        if self.creep_sleep != other.creep_sleep {
            return Some("faction_creep_sleep");
        }
        if self.sleeping_creeps != other.sleeping_creeps {
            return Some("faction_sleeping_creeps");
        }
        if self.facts != other.facts {
            return Some("faction_facts");
        }
        if self.next_fact_seq != other.next_fact_seq {
            return Some("faction_next_fact_seq");
        }
        None
    }

    /// 登记一条幂等事实（稳定 `seq` 顺序，无 payload 游戏状态）。
    fn record_fact(&mut self, frame: u64, entity: EntityId, kind: FactionFactKind) {
        let seq = self.next_fact_seq;
        self.facts.insert(
            seq,
            FactionFact {
                seq,
                frame,
                entity,
                kind,
            },
        );
        self.next_fact_seq = seq.saturating_add(1);
    }

    /// 按 30 Hz 整数帧逐 tick 查表结算全量机制（Q4 已拍板）。
    ///
    /// 全部输入为当前权威状态、`frame` 整数帧号与冻结常量；无墙钟、
    /// 无 `f32` / `f64`、无 `HashMap` 迭代顺序、无独立随机源。
    /// 回血条件由定义层 [`RegenCondition`] 枚举查表决定，MUST NOT
    /// 用种族名硬编码。仅在离散事件（相位切换 / 扩张 / 衰减完成 /
    /// 采集结算 / 反隐揭示 / 夜间隐身切换 / 回血首次生效 / 野怪睡眠
    /// 切换）登记事实，避免空转帧污染权威状态。
    pub fn settle(&mut self, frame: u64, constants: &FactionMechanicsConstants) {
        let mut pending_facts: Vec<(EntityId, FactionFactKind)> = Vec::new();

        // 1. R10.2 昼夜循环全量：整数帧推进，达到相位阈值即切换。
        self.day_night.elapsed_frames = self.day_night.elapsed_frames.saturating_add(1);
        let limit = match self.day_night.phase {
            DayNightPhase::Day => constants.day_frames.max(1),
            DayNightPhase::Night => constants.night_frames.max(1),
        };
        let phase_changed = self.day_night.elapsed_frames >= limit;
        if phase_changed {
            self.day_night.elapsed_frames = 0;
            self.day_night.phase = self.day_night.phase.toggled();
            self.day_night.cycles = self.day_night.cycles.saturating_add(1);
            pending_facts.push((EntityId::NONE, FactionFactKind::PhaseChanged));
        }

        // 2. R4.6 腐地扩张：每个源按稳定 EntityId 顺序推进整数帧进度。
        let expansion_interval = constants.blight_expansion_frames.max(1);
        let sources: Vec<EntityId> = self.blight_progress.keys().copied().collect();
        for source in sources {
            let cycles = self.day_night.cycles;
            if let Some(progress) = self.blight_progress.get_mut(&source) {
                *progress = progress.saturating_add(1);
                if *progress >= expansion_interval {
                    *progress = 0;
                    // 合成 [待核] 扩张规则：按源 id 与周期数确定性派生新格。
                    let x = i32::try_from(source.value() % 32).unwrap_or(0);
                    let y = i32::try_from(u64::from(cycles) % 32).unwrap_or(0);
                    self.blight_cells.insert((x, y));
                    pending_facts.push((source, FactionFactKind::BlightExpanded));
                }
            }
        }

        // 3. `:119` 尸体衰减：逐帧递减，到 0 移除，供食尸 / 召唤骷髅查询。
        let corpse_ids: Vec<EntityId> = self.corpses.keys().copied().collect();
        for corpse in corpse_ids {
            let decayed = match self.corpses.get_mut(&corpse) {
                Some(remaining) => {
                    *remaining = remaining.saturating_sub(1);
                    *remaining == 0
                }
                None => false,
            };
            if decayed {
                self.corpses.remove(&corpse);
                pending_facts.push((corpse, FactionFactKind::CorpseDecayed));
            }
        }

        // 4. R1.5 诅咒 / 缠绕金矿：按各自间隔确定性推进采集结算。
        let mine_ids: Vec<EntityId> = self.mines.keys().copied().collect();
        for mine in mine_ids {
            let interval = constants.haunted_mine_interval.max(1);
            let entangle_interval = constants.entangle_mine_interval.max(1);
            let settled = match self.mines.get_mut(&mine) {
                Some(record) => {
                    let bound = match record.kind {
                        MineKind::Haunted => interval,
                        MineKind::Entangled => entangle_interval,
                    };
                    record.progress_frames = record.progress_frames.saturating_add(1);
                    if record.progress_frames >= bound {
                        record.progress_frames = 0;
                        true
                    } else {
                        false
                    }
                }
                None => false,
            };
            if settled {
                pending_facts.push((mine, FactionFactKind::MineSettled));
            }
        }

        // 5. R10.3 回血：定义层枚举逐 tick 查表，非种族名硬编码。
        let blight_present = !self.blight_cells.is_empty();
        let phase = self.day_night.phase;
        let regen_ids: Vec<EntityId> = self.regen.keys().copied().collect();
        for unit in regen_ids {
            let applies = self
                .regen
                .get(&unit)
                .is_some_and(|record| record.condition.applies(phase, blight_present));
            if applies {
                if let Some(record) = self.regen.get_mut(&unit) {
                    let first_tick = record.accumulated == 0;
                    record.accumulated = record
                        .accumulated
                        .saturating_add(constants.blight_regen_per_frame);
                    if first_tick {
                        pending_facts.push((unit, FactionFactKind::RegenApplied));
                    }
                }
            }
        }

        // 6. S3 夜间隐身 + R10.4 隐身反隐：相位切换时同步夜间隐身态，
        //    再按反隐登记确定性重算被揭示目标集合（玩法判定，非 fog）。
        if phase_changed {
            let night = matches!(phase, DayNightPhase::Night);
            let cloak_ids: Vec<EntityId> = self.night_cloak.iter().copied().collect();
            for unit in cloak_ids {
                let record = self.stealth.entry(unit).or_default();
                record.night_only = true;
                record.cloaked = night;
                pending_facts.push((unit, FactionFactKind::NightCloakApplied));
            }
        }
        let mut detected = BTreeSet::new();
        // 反隐半径冻结常量作为判定门（合成 `[待核]` 规则：半径 > 0 且
        // 存在反隐登记才揭示隐身目标；玩法判定结果，非 fog 可见性）。
        let detection_active = !self.detectors.is_empty() && constants.stealth_detect_radius > 0;
        for (&unit, &record) in &self.stealth {
            if record.cloaked && detection_active {
                detected.insert(unit);
            }
        }
        for &unit in detected.difference(&self.detected) {
            pending_facts.push((unit, FactionFactKind::StealthRevealed));
        }
        self.detected = detected;

        // 7. R8.4 野怪睡眠：夜晚对登记单位结算睡眠，例外单位保持清醒。
        let night = matches!(phase, DayNightPhase::Night);
        let mut sleeping = BTreeSet::new();
        if night {
            for (&creep, &sleeps) in &self.creep_sleep {
                if sleeps {
                    sleeping.insert(creep);
                }
            }
        }
        if sleeping != self.sleeping_creeps {
            let changed: Vec<EntityId> = sleeping
                .symmetric_difference(&self.sleeping_creeps)
                .copied()
                .collect();
            for creep in changed {
                pending_facts.push((creep, FactionFactKind::CreepSleepChanged));
            }
            self.sleeping_creeps = sleeping;
        }

        // 8. 事实按稳定顺序登记（先按触发顺序，再按 seq 递增）。
        for (entity, kind) in pending_facts {
            self.record_fact(frame, entity, kind);
        }
    }
}

/// 定义层「回血条件按 `DefId` 类别登记」的辅助：稳定 `DefId` 顺序编码。
/// 用于 `FrozenDefinitions.regen_conditions` 段（`compute_defs_hash`）。
pub fn hash_regen_conditions(hasher: &mut StateHasher, table: &BTreeMap<DefId, RegenCondition>) {
    hasher.write_u64(table.len() as u64);
    for (&def_id, &condition) in table {
        hasher.write_u32(def_id);
        hasher.write_u8(condition.code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_constants_are_pending_and_match_client_semantics() {
        let constants = SYNTHETIC_FACTION_MECHANICS;
        assert_eq!(constants.day_frames, 600);
        assert_eq!(constants.night_frames, 480);
        assert_eq!(constants.blight_expansion_frames, 90);
        assert_eq!(constants.corpse_decay_frames, 900);
        assert_eq!(constants.blight_regen_per_frame, 1);
        assert_eq!(constants.moon_well_restore, 50);
        assert_eq!(constants.entangle_mine_interval, 60);
        assert_eq!(constants.haunted_mine_interval, 60);
        assert_eq!(constants.stealth_detect_radius, 8);
        // 两套昼夜常量口径并列 [待核]，MUST NOT 静默统一（Q3'）。
        assert_eq!(SYNTHETIC_CREEP_NIGHT_FRAMES, 600);
        assert_ne!(constants.night_frames, SYNTHETIC_CREEP_NIGHT_FRAMES);
    }

    /// `CreepConstants` 的另一套夜晚时长 [待核]（并列登记，不统一）。
    const SYNTHETIC_CREEP_NIGHT_FRAMES: u32 = 600;

    #[test]
    fn constants_hash_is_content_sensitive() {
        let base = SYNTHETIC_FACTION_MECHANICS;
        let mut h_base = StateHasher::new();
        base.hash(&mut h_base);
        let mut changed = base;
        changed.corpse_decay_frames += 1;
        let mut h_changed = StateHasher::new();
        changed.hash(&mut h_changed);
        assert_ne!(h_base.finish(), h_changed.finish());
        assert_eq!(
            base.first_difference(&changed),
            Some("faction_const_corpse_decay_frames")
        );
    }

    #[test]
    fn regen_condition_lookup_is_enum_driven() {
        assert!(RegenCondition::Always.applies(DayNightPhase::Day, false));
        assert!(RegenCondition::Day.applies(DayNightPhase::Day, false));
        assert!(!RegenCondition::Day.applies(DayNightPhase::Night, false));
        assert!(RegenCondition::Night.applies(DayNightPhase::Night, false));
        assert!(!RegenCondition::Blight.applies(DayNightPhase::Day, false));
        assert!(RegenCondition::Blight.applies(DayNightPhase::Day, true));
        assert!(!RegenCondition::None.applies(DayNightPhase::Day, true));
    }

    #[test]
    fn enum_codes_are_stable() {
        assert_eq!(DayNightPhase::Day.code(), 1);
        assert_eq!(DayNightPhase::Night.code(), 2);
        assert_eq!(DayNightPhase::from_code(2), Some(DayNightPhase::Night));
        assert_eq!(DayNightPhase::from_code(9), None);
        assert_eq!(RegenCondition::Blight.code(), 2);
        assert_eq!(RegenCondition::from_code(4), Some(RegenCondition::Night));
        assert_eq!(RegenCondition::from_code(9), None);
        assert_eq!(MineKind::Entangled.code(), 2);
        assert_eq!(MineKind::from_code(1), Some(MineKind::Haunted));
    }
}
