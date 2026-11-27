//! P8.5c 野怪营地与掉落：冻结定义类型与权威状态容器。
//!
//! 本模块只提供纯整数的数据结构与确定性编码，不读取文件、不依赖
//! `w3-content` / 渲染 / 网络，也不持有闭包、活动对象、墙钟或浮点。
//! 分两层，MUST NOT 混淆（ADR 0002 §6-8 / D1 / D4）：
//! - **冻结定义层**：`CreepConstants` / `CampScript` / `DropTable` /
//!   `NeutralCategory`。这些是编译期合成原创整数常量 `[待核]`，经
//!   `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部；
//!   改任一常量即改变 `content_hash()`，属「定义层内容变化」。
//! - **权威状态层**：`CreepCampState` 及其成员。这些是每帧由 `RtsMatch`
//!   独占写入的运行态，MUST 同步进 FNV-1a 状态哈希、快照与
//!   `first_difference()`；`rng_state` / `rng_draws` 是确定性随机码
//!   消费契约（Q6 / R8.6）的权威序列。
//!
//! 业务规则：R1.1（无主地面物品）/ R1.4（框选命令过滤）/ R6.1（经验）/
//! R6.5（分摊两环节）/ R7.2（掉落来源）/ R7.5（实例独立）/ R7.6（`EffectDef`）/
//! R8.1（对所有玩家敌对）/ R8.2（营地分级）/ R8.3（金币）/ R8.5（leash）/
//! R8.6（冻结随机码）。数值全部 `[待核]`，不冒充原版（§14 / ADR 0001）。

use std::collections::{BTreeMap, BTreeSet};

use crate::entity::EntityId;
use crate::fixed::SCALE;
use crate::gameplay::{DefId, EffectDef, effect_kind_code};
use crate::state_hash::StateHasher;

/// 中立 owner 常量：玩家号从 1 起，`0` 保留给中立实体与系统级命令。
///
/// 野怪 MUST NOT 挂现有玩家 owner 下 spawn（Q7 / R8.1）；`9xx` 仍是
/// client 侧 DefId 段语义，MUST NOT 复用为 owner。野怪不响应玩家 `Move`
/// 由 `is_actionable` 的中立登记判据保证（R1.4 / residual R-e）。
pub const NEUTRAL_OWNER: u8 = 0;

/// 地面物品 / 物品实例的稳定 ID 起始段，与 `EntityStore` 的单位 ID 不重叠。
///
/// 地面物品不是 `EntityStore` 实体，因此不出现在 `read_view()` 的实体集合、
/// 不参与移动积分，MUST NOT 出现在 `Move` 命令生成集合（R1.4）。
pub const GROUND_ITEM_ID_BASE: u64 = 1u64 << 32;

/// 事件驱动掉落随机流的域分隔常量：与每帧 LCG 主流的 `seed` 派生路径区分。
/// 纯整数编译期常量 `[待核]`，不含浮点 / 墙钟 / 表现层随机源（Q6 / R8.6）。
pub const CREEP_RNG_SALT: u64 = 0x4352_4545_505f_524e;

/// 中立类别 / 阵营标记（冻结定义层，Q7）。
///
/// 「对所有玩家敌对」（R8.1）与「无主地面物品」（R1.1）由定义层登记表达，
/// MUST NOT 用建筑名字符串白名单或硬编码 DefId（R1.2 / R13.3 / D4）。
/// 非经典命名的 DefId 只要登记进类别集合即可被判定。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum NeutralCategory {
    /// 中立敌对野怪（R8.1）。
    CreepHostile,
    /// 无主地面物品（R1.1）。
    GroundItemOwnerless,
}

impl NeutralCategory {
    /// 稳定的单字节编码，不依赖枚举内存布局（ADR 0002 §6）。
    pub const fn code(self) -> u8 {
        match self {
            NeutralCategory::CreepHostile => 1,
            NeutralCategory::GroundItemOwnerless => 2,
        }
    }

    /// 从显式编码还原；非法值返回 `None`，供线格式整数 ↔ 枚举对齐。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(NeutralCategory::CreepHostile),
            2 => Some(NeutralCategory::GroundItemOwnerless),
            _ => None,
        }
    }
}

/// 野怪 leash / 归巢状态机（R8.5 / D3）。
///
/// `RETURNING` 途中不索敌 / 不反击；归巢到 home 锚点且进入归巢阈值后
/// 恢复 `Idle`。`Default` 为 `Idle`。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum LeashPhase {
    /// 营地待机，按定义层 `acquisitionRange` 索敌。
    #[default]
    Idle,
    /// 已锁定目标并追击，受 leash 半径约束。
    Engaged,
    /// 越界 / 目标失效后归巢，途中不索敌。
    Returning,
}

impl LeashPhase {
    /// 稳定的单字节编码。
    pub const fn code(self) -> u8 {
        match self {
            LeashPhase::Idle => 1,
            LeashPhase::Engaged => 2,
            LeashPhase::Returning => 3,
        }
    }

    /// 从显式编码还原；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(LeashPhase::Idle),
            2 => Some(LeashPhase::Engaged),
            3 => Some(LeashPhase::Returning),
            _ => None,
        }
    }
}

/// 野怪 / 营地冻结常量集合（全为合成 / 原创编译期整数常量 `[待核]`）。
///
/// 距离字段是 Q47.16 原始值（与 `SimFixed::raw()` 同域），时间字段是
/// 30 Hz 整数帧。R8.4 昼夜时长只登记冻结常量、睡眠行为按 Q3 留后续
/// change；随机池类别编码只登记整数码、不读本机导出。改任一常量即
/// 改变 `content_hash()`（Q8 / §14 / ADR 0001 / ADR 0004）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CreepConstants {
    /// leash 半径（Q47.16 raw）：越界即停止追击并归巢（R8.5）。
    pub leash_radius_raw: i64,
    /// 归巢阈值（Q47.16 raw）：回到该范围内即恢复 IDLE。
    pub return_threshold_raw: i64,
    /// 警戒 / 追击范围（Q47.16 raw），由定义层数据驱动（R8.5）。
    pub acquisition_range_raw: i64,
    /// 经验分摊范围（Q47.16 raw），R6.5「进入集合」判据。
    pub xp_share_radius_raw: i64,
    /// R8.4 昼夜时长（整数帧 `[待核]`；睡眠行为本期不实现）。
    pub day_frames: u32,
    /// R8.4 夜间时长（整数帧 `[待核]`；睡眠行为本期不实现）。
    pub night_frames: u32,
    /// R6.5 经验实际入账的等级因子百分比（`[待核]`）。
    pub hero_level_xp_factor: i64,
    /// R8.2 营地分级编码（绿 / 黄 / 红，`[待核]`）。
    pub camp_tiers: [u8; 3],
    /// 随机池类别编码（`[待核]`；不读真实导出数据）。
    pub random_pool_category: u32,
    /// 地面物品实例初始充能（R7.5，`[待核]`）。
    pub item_charges: u32,
    /// 地面物品实例初始冷却（30 Hz 整数帧，R7.5 / `[待核]`）。
    pub item_cooldown_frames: u32,
    /// 野怪 AI 确定性重算周期（整数帧 `[待核]`）：每帧仍推进既有订单与
    /// 攻击对，只在决策点重算追击路径，与 `run_ai` 重算框架同构（D3）。
    pub ai_decision_period: u32,
}

/// 合成 / 原创的野怪冻结常量 `[待核]`；数值不冒充经典规则、不来自任何资产。
pub const SYNTHETIC_CREEP_CONSTANTS: CreepConstants = CreepConstants {
    leash_radius_raw: 12 * SCALE,
    return_threshold_raw: 2 * SCALE,
    acquisition_range_raw: 8 * SCALE,
    xp_share_radius_raw: 10 * SCALE,
    day_frames: 600,
    night_frames: 600,
    hero_level_xp_factor: 100,
    camp_tiers: [1, 2, 3],
    random_pool_category: 7,
    item_charges: 3,
    item_cooldown_frames: 10,
    ai_decision_period: 4,
};

impl Default for CreepConstants {
    fn default() -> Self {
        SYNTHETIC_CREEP_CONSTANTS
    }
}

impl CreepConstants {
    /// 显式小端编码进定义哈希；任一常量变化都改变结果（Q8 / tasks 2.2）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_i64(self.leash_radius_raw);
        hasher.write_i64(self.return_threshold_raw);
        hasher.write_i64(self.acquisition_range_raw);
        hasher.write_i64(self.xp_share_radius_raw);
        hasher.write_u32(self.day_frames);
        hasher.write_u32(self.night_frames);
        hasher.write_i64(self.hero_level_xp_factor);
        for &tier in &self.camp_tiers {
            hasher.write_u8(tier);
        }
        hasher.write_u32(self.random_pool_category);
        hasher.write_u32(self.item_charges);
        hasher.write_u32(self.item_cooldown_frames);
        hasher.write_u32(self.ai_decision_period);
    }
}

/// 冻结野怪营地脚本：`SeedCreepCamp` 命令按 `script` 标识查表装配（Q5）。
///
/// 全部字段为合成原创编译期整数 `[待核]`；`creep_defs` 是稳定顺序的
/// DefId 列表，判定只查定义层类别集合，MUST NOT 硬编码 / 白名单（R1.2）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CampScript {
    /// R8.2 营地分级编码（绿 / 黄 / 红，`[待核]`）。
    pub tier: u8,
    /// R6.1 / R8.3 击杀经验与金币（`[待核]`）。
    pub xp: i64,
    /// R8.3 击杀方金币收益（`[待核]`）。
    pub gold: i64,
    /// home 锚点 x（Q47.16 raw）。
    pub home_x: i64,
    /// home 锚点 y（Q47.16 raw）。
    pub home_y: i64,
    /// 关联掉落表 id（R7.2 / R8.6）。
    pub drop_table: u32,
    /// 营地野怪 DefId 列表（稳定顺序，按定义层类别校验）。
    pub creep_defs: Vec<DefId>,
}

impl CampScript {
    /// 显式小端编码进定义哈希。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u8(self.tier);
        hasher.write_i64(self.xp);
        hasher.write_i64(self.gold);
        hasher.write_i64(self.home_x);
        hasher.write_i64(self.home_y);
        hasher.write_u32(self.drop_table);
        hasher.write_u64(self.creep_defs.len() as u64);
        for &def in &self.creep_defs {
            hasher.write_u32(def);
        }
    }
}

/// 掉落互斥组：一次抽取至多命中一个 `DefId`（R7.2 / R8.6）。
///
/// `entries` 以 `DefId` 为键的确定性有序容器，值为概率权重 `[待核]`；
/// 同一互斥组 MUST NOT 同时掉两件。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DropSet {
    pub entries: BTreeMap<DefId, u32>,
}

impl DropSet {
    /// 互斥组权重之和；显式小端编码进定义哈希。
    pub fn total_weight(&self) -> u64 {
        self.entries.values().map(|&w| u64::from(w)).sum()
    }

    /// 显式小端编码进定义哈希（表 → set → 概率条目分层）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.entries.len() as u64);
        for (&item_def, &weight) in &self.entries {
            hasher.write_u32(item_def);
            hasher.write_u32(weight);
        }
    }
}

/// 掉落表：表 → 多个互斥 set → 概率条目（R7.2 / R8.6 / 结论 3）。
///
/// 道具效果 MUST 走既有 `EffectDef` 组合、MUST NOT 引入 JASS VM
/// （R7.6 / N5）；`effects` 以 DefId 为键登记每个可掉落道具的效果定义，
/// 与 `ItemDef` 的属性修正同属冻结定义层内容，不进权威状态。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DropTable {
    pub sets: BTreeMap<u32, DropSet>,
    pub effects: BTreeMap<DefId, EffectDef>,
}

impl DropTable {
    /// 显式小端编码进定义哈希；任一段变化都改变 `content_hash()`。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.sets.len() as u64);
        for (&set_id, set) in &self.sets {
            hasher.write_u32(set_id);
            set.hash(hasher);
        }
        hasher.write_u64(self.effects.len() as u64);
        for (&item_def, effect) in &self.effects {
            hasher.write_u32(item_def);
            hasher.write_u8(effect_kind_code(effect.kind));
            hasher.write_i64(effect.amount);
            hasher.write_u32(effect.duration_frames);
        }
    }
}

/// 单个野怪的 leash / home 权威状态（D1 / D3 / R8.5）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CreepState {
    /// 所属营地 id（稳定 `u32` 键）。
    pub camp: u32,
    /// 冻结定义的野怪 DefId（数据驱动，非白名单）。
    pub def_id: DefId,
    /// home 锚点 x（Q47.16 raw）。
    pub home_x: i64,
    /// home 锚点 y（Q47.16 raw）。
    pub home_y: i64,
    /// leash 状态机当前阶段。
    pub phase: LeashPhase,
    /// 当前锁定目标（仅 `Engaged` 有效）。
    pub target: Option<EntityId>,
}

/// 单个营地的权威状态（home 锚点 / 聚类 / 分级 / 清野标记）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CampState {
    /// 冻结脚本 id（关联 `CampScript`）。
    pub script: u32,
    /// R8.2 分级编码（`[待核]`）。
    pub tier: u8,
    /// home 锚点 x（Q47.16 raw）。
    pub home_x: i64,
    /// home 锚点 y（Q47.16 raw）。
    pub home_y: i64,
    /// 营地成员（稳定 `EntityId` 顺序）。
    pub members: BTreeSet<EntityId>,
    /// 全清标记（R11.5 / R12.4 重开无残留判据）。
    pub cleared: bool,
}

/// 无主地面物品权威状态（R1.1 / R7.2）。
///
/// 位置由掉落结算一次性写入，之后不随任何运动系统变化（R1.4）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GroundItemState {
    pub item_def: DefId,
    pub camp: u32,
    pub x: i64,
    pub y: i64,
    /// 对应的物品实例 id（同名道具多实例独立，R7.5）。
    pub instance: EntityId,
}

/// 物品实例权威状态：同名道具的多个实例各有独立充能 / 冷却（R7.5）。
///
/// 效果走既有 `EffectDef` 组合、MUST NOT 引入 JASS VM（R7.6 / N5）。
/// P8.5d 追加 `holder` / `slot`：`Some` 表示被某英雄六格背包持有，
/// `None` 表示无主地面物品（R1.1 / R6.2）。持有者与槽位是
/// `ItemShopState.bags` 的反向索引，两者 MUST 同步进哈希 / 快照 /
/// `first_difference()`（ADR 0002 §6-8 / Q7）。拾取 / 丢弃 / 移交 MUST
/// 保持同一 `EntityId` 身份与剩余充能 / 冷却（「转交也不能刷新」）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ItemInstanceState {
    pub item_def: DefId,
    pub charges: u32,
    pub cooldown_frames: u32,
    pub effect: EffectDef,
    /// 持有英雄；`None` 表示无主地面物品（R1.1）。
    pub holder: Option<EntityId>,
    /// 六格背包槽位 `0..6`；`None` 表示不在背包。
    pub slot: Option<u8>,
}

/// 击杀收益登记（R6.1 / R6.5 / R8.3，按稳定 `EntityId` 只结算一次）。
///
/// `xp_candidates` 是 R6.5「进入分摊集合」环节，`xp_credited` 是
/// 「实际入账 / 等级因子」环节；两环节 MUST 分离可验证。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KillReward {
    pub creep: EntityId,
    pub camp: u32,
    pub frame: u64,
    pub killer: EntityId,
    pub killer_owner: u8,
    pub xp_candidates: BTreeSet<EntityId>,
    pub xp_credited: BTreeMap<EntityId, i64>,
    pub gold: i64,
    pub drops: Vec<EntityId>,
}

/// 一次确定性随机码消费记录（Q6 / R8.6）。
///
/// 消费序列以 `seq` 为稳定键登记；`rng_before` / `rng_after` 是同一
/// match-owned 整数 LCG 的前后状态，重放 MUST 逐帧一致。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RngDraw {
    pub seq: u64,
    pub frame: u64,
    pub creep: EntityId,
    pub table: u32,
    pub set: u32,
    pub rng_before: u64,
    pub rng_after: u64,
    pub roll: u64,
    pub picked: Option<DefId>,
}

/// 野怪死亡通知（供共享结算服务与确定性复核投递，幂等）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CreepDeathNotice {
    pub killer: EntityId,
    pub killer_owner: u8,
    pub frame: u64,
}

/// P8.5c 野怪 / 营地 / 掉落全部权威状态的唯一容器。
///
/// 全部落 `BTreeMap` / `BTreeSet` 确定性有序容器，按稳定 `EntityId` /
/// `u32` / `u64` 键顺序遍历；MUST 同时进入 FNV-1a 状态哈希、由
/// `SNAPSHOT_VERSION`（当前 9）定义的快照与 `first_difference()`
/// （ADR 0002 §6-8 / `CODEBUDDY.md §5` / D1 / D5）。
/// P8.5d 起 `item_instances` 追加 `holder` / `slot`，与
/// `ItemShopState.bags` 同属单一实例身份口径（Q7）。
///
/// `rng_state` 是独立事件驱动掉落流（由 `seed` 派生，随快照恢复），
/// `rng_draws` / `next_rng_seq` 是显式消费序列编码；两者与每帧恰好
/// 消耗一次的主流 `RtsMatch.rng` 分离，MUST NOT 用墙钟 / `f64` /
/// `HashMap` 迭代顺序 / 表现层随机源（Q6 / R8.6 / D2）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CreepCampState {
    /// 营地登记（稳定 `u32` 营地 id）。
    pub camps: BTreeMap<u32, CampState>,
    /// 野怪登记（稳定 `EntityId`）。
    pub creeps: BTreeMap<EntityId, CreepState>,
    /// 中立归属独立登记（Q7 / R8.1 / R1.1）；野怪与地面物品均在此。
    pub neutral: BTreeMap<EntityId, NeutralCategory>,
    /// 无主地面物品（稳定 `EntityId`，R1.1）。
    pub ground_items: BTreeMap<EntityId, GroundItemState>,
    /// 物品实例（稳定 `EntityId`，R7.5）。
    pub item_instances: BTreeMap<EntityId, ItemInstanceState>,
    /// 已处理死亡登记：幂等结算的唯一判据（R7.2 / R8.6）。
    pub death_processed: BTreeSet<EntityId>,
    /// 待结算死亡通知（同一野怪重复投递只结算一次）。
    pub pending_deaths: BTreeMap<EntityId, CreepDeathNotice>,
    /// 击杀收益登记（稳定 `EntityId`）。
    pub kill_rewards: BTreeMap<EntityId, KillReward>,
    /// 确定性随机码消费序列（稳定 `seq`）。
    pub rng_draws: BTreeMap<u64, RngDraw>,
    /// 下一个消费序号。
    pub next_rng_seq: u64,
    /// 事件驱动掉落流状态（由 `seed` 派生，随快照恢复）。
    pub rng_state: u64,
    /// 下一个地面物品 / 物品实例 id。
    pub next_ground_item_id: u64,
}

/// 显式小端编码辅助：`Option<EntityId>` 带存在标记，避免哈希碰撞。
fn hash_opt_entity(hasher: &mut StateHasher, value: Option<EntityId>) {
    match value {
        Some(id) => {
            hasher.write_u8(1);
            hasher.write_u64(id.value());
        }
        None => hasher.write_u8(0),
    }
}

impl CreepCampState {
    /// 全部权威状态显式小端编码进 FNV-1a；按稳定键顺序遍历。
    /// 新增字段 MUST 同步接入本方法、`first_difference()` 与快照
    /// （`CODEBUDDY.md §5` 硬门禁）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.camps.len() as u64);
        for (&camp_id, camp) in &self.camps {
            hasher.write_u32(camp_id);
            hasher.write_u32(camp.script);
            hasher.write_u8(camp.tier);
            hasher.write_i64(camp.home_x);
            hasher.write_i64(camp.home_y);
            hasher.write_u64(camp.members.len() as u64);
            for &id in &camp.members {
                hasher.write_u64(id.value());
            }
            hasher.write_u8(u8::from(camp.cleared));
        }

        hasher.write_u64(self.creeps.len() as u64);
        for (&id, creep) in &self.creeps {
            hasher.write_u64(id.value());
            hasher.write_u32(creep.camp);
            hasher.write_u32(creep.def_id);
            hasher.write_i64(creep.home_x);
            hasher.write_i64(creep.home_y);
            hasher.write_u8(creep.phase.code());
            hash_opt_entity(hasher, creep.target);
        }

        hasher.write_u64(self.neutral.len() as u64);
        for (&id, &category) in &self.neutral {
            hasher.write_u64(id.value());
            hasher.write_u8(category.code());
        }

        hasher.write_u64(self.ground_items.len() as u64);
        for (&id, item) in &self.ground_items {
            hasher.write_u64(id.value());
            hasher.write_u32(item.item_def);
            hasher.write_u32(item.camp);
            hasher.write_i64(item.x);
            hasher.write_i64(item.y);
            hasher.write_u64(item.instance.value());
        }

        hasher.write_u64(self.item_instances.len() as u64);
        for (&id, instance) in &self.item_instances {
            hasher.write_u64(id.value());
            hasher.write_u32(instance.item_def);
            hasher.write_u32(instance.charges);
            hasher.write_u32(instance.cooldown_frames);
            hasher.write_u8(effect_kind_code(instance.effect.kind));
            hasher.write_i64(instance.effect.amount);
            hasher.write_u32(instance.effect.duration_frames);
            // P8.5d：实例 owner 与 slot 显式小端编码；Option 带存在标记，
            // 避免 `Some(0)` 与 `None` 可构造的哈希碰撞（ADR 0002 §6）。
            match instance.holder {
                Some(hero) => {
                    hasher.write_u8(1);
                    hasher.write_u64(hero.value());
                }
                None => hasher.write_u8(0),
            }
            match instance.slot {
                Some(slot) => {
                    hasher.write_u8(1);
                    hasher.write_u8(slot);
                }
                None => hasher.write_u8(0),
            }
        }

        hasher.write_u64(self.death_processed.len() as u64);
        for &id in &self.death_processed {
            hasher.write_u64(id.value());
        }

        hasher.write_u64(self.pending_deaths.len() as u64);
        for (&id, notice) in &self.pending_deaths {
            hasher.write_u64(id.value());
            hasher.write_u64(notice.killer.value());
            hasher.write_u8(notice.killer_owner);
            hasher.write_u64(notice.frame);
        }

        hasher.write_u64(self.kill_rewards.len() as u64);
        for (&id, reward) in &self.kill_rewards {
            hasher.write_u64(id.value());
            hasher.write_u64(reward.creep.value());
            hasher.write_u32(reward.camp);
            hasher.write_u64(reward.frame);
            hasher.write_u64(reward.killer.value());
            hasher.write_u8(reward.killer_owner);
            hasher.write_u64(reward.xp_candidates.len() as u64);
            for &hero in &reward.xp_candidates {
                hasher.write_u64(hero.value());
            }
            hasher.write_u64(reward.xp_credited.len() as u64);
            for (&hero, &xp) in &reward.xp_credited {
                hasher.write_u64(hero.value());
                hasher.write_i64(xp);
            }
            hasher.write_i64(reward.gold);
            hasher.write_u64(reward.drops.len() as u64);
            for &drop in &reward.drops {
                hasher.write_u64(drop.value());
            }
        }

        hasher.write_u64(self.rng_draws.len() as u64);
        for (&seq, draw) in &self.rng_draws {
            hasher.write_u64(seq);
            hasher.write_u64(draw.frame);
            hasher.write_u64(draw.creep.value());
            hasher.write_u32(draw.table);
            hasher.write_u32(draw.set);
            hasher.write_u64(draw.rng_before);
            hasher.write_u64(draw.rng_after);
            hasher.write_u64(draw.roll);
            match draw.picked {
                Some(item_def) => {
                    hasher.write_u8(1);
                    hasher.write_u32(item_def);
                }
                None => hasher.write_u8(0),
            }
        }
        hasher.write_u64(self.next_rng_seq);
        hasher.write_u64(self.rng_state);
        hasher.write_u64(self.next_ground_item_id);
    }

    /// 按同一稳定顺序返回首个差异字段名，便于 desync 诊断（ADR 0002 §6）。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.camps != other.camps {
            return Some("creep_camps");
        }
        if self.creeps != other.creeps {
            return Some("creep_creeps");
        }
        if self.neutral != other.neutral {
            return Some("creep_neutral");
        }
        if self.ground_items != other.ground_items {
            return Some("creep_ground_items");
        }
        // P8.5d：先按稳定 EntityId 顺序单独定位实例 owner / slot 差异，
        // 使新增字段有独立诊断名；只改充能或冷却时落到下一分支。
        let holders =
            |state: &CreepCampState| -> BTreeMap<EntityId, (Option<EntityId>, Option<u8>)> {
                state
                    .item_instances
                    .iter()
                    .map(|(&id, instance)| (id, (instance.holder, instance.slot)))
                    .collect()
            };
        if holders(self) != holders(other) {
            return Some("creep_item_holders");
        }
        if self.item_instances != other.item_instances {
            return Some("creep_item_instances");
        }
        if self.death_processed != other.death_processed {
            return Some("creep_death_processed");
        }
        if self.pending_deaths != other.pending_deaths {
            return Some("creep_pending_deaths");
        }
        if self.kill_rewards != other.kill_rewards {
            return Some("creep_kill_rewards");
        }
        if self.rng_draws != other.rng_draws {
            return Some("creep_rng_draws");
        }
        if self.next_rng_seq != other.next_rng_seq {
            return Some("creep_next_rng_seq");
        }
        if self.rng_state != other.rng_state {
            return Some("creep_rng_state");
        }
        if self.next_ground_item_id != other.next_ground_item_id {
            return Some("creep_next_ground_item_id");
        }
        None
    }
}

// ---------------------------------------------------------------------------
// R6.1 / R6.5 布尔与因子机制结构（编译期合成占位 `[待核]`）
// ---------------------------------------------------------------------------
//
// 本节只登记三类机制的**结构**，全部是编译期合成原创整数常量 `[待核]`：
// 布尔开关、经验折减因子、按等级分档递减的因子表。它们 MUST NOT 接进
// `sim_match.rs` 的野怪死亡结算（结算行为保持不变，`design.md` D2 / D3），
// 也不新增 / 不修改任何运行时权威状态字段集合（D7）。
//
// 关键约束：本节常量 MUST NOT 编入 `CreepConstants::hash()` /
// `compute_defs_hash()`。`content_hash()` 与逐帧 `compute_state_hash()`
// 因此逐字节不变，`SNAPSHOT_VERSION` / `SIM_VERSION` 仍为 9，v9 golden
// hash 基线不受影响（本 change 不 bump、不重取基线，双平台逐字节比对仍
// 待人工 / 待 VM）。这与 `NEUTRAL_OWNER` / `GROUND_ITEM_ID_BASE` /
// `CREEP_RNG_SALT` 等编译期常量不进定义哈希的既有体例一致。
//
// 「建筑击杀者」「召唤物击杀者」的运行时实体分类判定、「按英雄等级分档
// 实际入账」的结算接线，需要新增权威状态字段 / 实体分类或改结算表达式，
// 超出首期最小闭环，按 `design.md` D7 / tasks 3.2 顺延 p18 并登记为
// 已知限制；数值一律 `[待核]` 合成占位，不冻结任何真值
// （附录「核实约定」 / ADR 0001 / ADR 0004）。

/// 建筑击杀者是否授予经验的布尔开关（编译期合成整数 `[待核]`）。
///
/// `1` 表示授予、`0` 表示不授予；只登记布尔开关这一机制结构，
/// 判定「击杀者是否建筑」的运行时实体分类顺延 p18。
pub const MECHANISM_BUILDING_XP_SWITCH: u8 = 1;

/// 召唤物击杀经验折减因子百分比（编译期合成整数 `[待核]`）。
///
/// 只登记折减因子这一机制结构，召唤物击杀者判定顺延 p18。
pub const MECHANISM_SUMMON_XP_FACTOR: i64 = 50;

/// 按英雄等级分档递减的经验因子表（编译期合成整数 `[待核]`）。
///
/// 下标对应「等级减一」档位，取值逐级递减；只登记分档表这一机制结构，
/// 等级阈值分档与实际入账接线顺延 p18。
pub const MECHANISM_HERO_LEVEL_XP_TIERS: [i64; 4] = [100, 80, 60, 40];

/// 按英雄等级查分档递减经验因子（纯整数、无浮点、越界钳到末档）。
///
/// 只表达「分档递减」公式族结构，MUST NOT 被结算路径读取；
/// `level` 为 0 与超出表长时都确定性钳到首 / 末档，不引入分支不确定性。
pub fn synthetic_hero_level_xp_factor(level: u32) -> i64 {
    let tiers = MECHANISM_HERO_LEVEL_XP_TIERS;
    let last = (tiers.len() - 1) as u32;
    let index = level.saturating_sub(1).min(last) as usize;
    tiers[index]
}
