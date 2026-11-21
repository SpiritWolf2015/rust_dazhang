//! 帧边界处已完成提交的完整、带版本的对局状态。
//!
//! 快照是第 N 帧提交完成后的状态，恢复后从第 N+1 帧继续。包含随机流、
//! 下一 ID 计数器、已接收但尚未执行的命令、全部单位订单与寻路状态、
//! 动态占地与群体规划进度。行为由 ID 与显式数据重建，绝不来自闭包、
//! 任务或活动对象。静态地图不嵌入快照，只记录其身份用于恢复校验。
//!
//! v3 由 P5 引入：在 v2 基础上追加冻结内容定义、内容哈希头与全部玩法
//! 运行态。未发布过其它 v3 快照，恢复时 `world_identity` / `defs_hash` /
//! `content_hash` 校验会拒绝编码不一致或被篡改的状态，不提供跨版本迁移。
//!
//! v4 由 P8.5a 引入：追加英雄复活三组权威状态（死亡登记 / 复活队列 /
//! 费用登记），并同步 `SNAPSHOT_VERSION` 与 `SIM_VERSION` 到 4。旧 v3
//! 快照因版本不匹配被明确拒绝（`SnapshotError::UnsupportedVersion`），
//! 不提供跨版本迁移（ADR 0002 §7）。
//!
//! v5 由 P8.5b 引入：追加主城升级三组权威状态（等级 / 升级队列 /
//! 费用登记），并同步 `SNAPSHOT_VERSION` 与 `SIM_VERSION` 到 5。旧 v4
//! 及更早快照按版本不匹配明确拒绝，不提供跨版本迁移（ADR 0002 §7 / Q1）。
//!
//! v6 由 P8.5c 引入：追加野怪 / 营地 / 中立归属 / leash / home 状态机 /
//! 掉落事实 / 地面物品 / 物品实例 / 击杀收益登记 / 确定性随机码消费序列
//! 等权威状态，并同步 `SNAPSHOT_VERSION` 与 `SIM_VERSION` 到 6。旧 v5
//! 及更早快照按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝，
//! 不提供跨版本迁移（ADR 0002 §7 / Q4）。
//!
//! v7 由 P8.5d 引入：追加顶层权威状态 `item_shop`（六格定长实例背包 /
//! 复活死亡背包 / 拾取购买使用丢弃移交售卖事实 / 商店库存与刷新计数 /
//! 回城传送施法帧目标中断次数 / 确定性随机码消费序列 / 购买事务扣费登记），
//! 并把 `ItemInstanceState` 扩展出 `holder` / `slot`；商店 / 道具 / 回城
//! 冻结定义段进入 `defs_hash` / `content_hash` 快照头部。同步
//! `SNAPSHOT_VERSION` 与 `SIM_VERSION` 到 7。旧 v6 及更早快照按
//! `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本
//! 迁移（ADR 0002 §7 / Q4）。

use crate::command::Command;
use crate::creep::CreepCampState;
use crate::gameplay::{
    FrozenDefinitions, HeroReviveState, TownHallUpgradeState, UnitGameplayState, WorldGameplayState,
};
use crate::item_shop::ItemShopState;

pub const SNAPSHOT_VERSION: u32 = 7;
pub const SIM_VERSION: u32 = 7;

/// 订单的快照表示（与权威 `Order` 一一对应）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderRecord {
    Idle,
    Stop,
    Move { x: i64, y: i64, group: u32 },
    Attack { target: u64 },
}

/// 单个动态占地矩形。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObstacleRecord {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// 一个进行中的群体规划作业。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GroupPlanRecord {
    pub group: u32,
    pub goal: (i64, i64),
    pub members: Vec<u64>,
    pub slots: Vec<(i64, i64)>,
    pub progress: u32,
}

/// 单个单位在帧边界的完整权威状态（订单、寻路、朝向、战斗与生命）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PositionRecord {
    pub entity: u64,
    pub x: i64,
    pub y: i64,
    pub facing: i64,
    pub speed: i64,
    pub hp: i64,
    pub max_hp: i64,
    pub owner: u8,
    pub attack_damage: i64,
    pub attack_range: i64,
    pub turn_rate: i64,
    pub alive: bool,
    pub order: OrderRecord,
    pub pending: Vec<OrderRecord>,
    pub goal: Option<(i64, i64)>,
    pub path: Vec<(i64, i64)>,
    pub path_index: u32,
    pub blocked_frames: u32,
    pub group: u32,
    /// 单位玩法运行态（法力 / Buff / 施法 / 冷却 / 英雄 / 背包 / 生产队列）。
    pub gameplay: UnitGameplayState,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    pub version: u32,
    pub sim_version: u32,
    pub seed: u64,
    /// 对局配置的内容身份（P2 既有字段，由 app 层提供）。
    pub content_hash: u64,
    /// 冻结内容定义的内容哈希，恢复时与 `defs` 重新计算的结果比对。
    pub defs_hash: u64,
    pub logic_fps: u32,
    pub tick: u64,
    pub rng: u64,
    pub next_entity_id: u64,
    pub alive_entities: Vec<u64>,
    pub positions: Vec<PositionRecord>,
    pub pending_commands: Vec<Command>,
    pub next_arrival: u64,
    pub next_event_seq: u64,
    /// 静态地图身份，恢复时与实际注入的世界比对（ADR 0004）。
    pub world_identity: u64,
    pub obstacles: Vec<ObstacleRecord>,
    pub group_plans: Vec<GroupPlanRecord>,
    /// 冻结内容定义（恢复时据此重建，不依赖内容层）。
    pub defs: FrozenDefinitions,
    /// 全局玩法运行态（经济 / 胜负 / AI / 光环 / 波次 / 行为注册 / 玩法命令）。
    pub gameplay: WorldGameplayState,
    /// 英雄复活三组权威状态（死亡登记 / 复活队列 / 费用登记，v4 新增）。
    /// 帧 N 提交后的完整状态必须包含它们，恢复后从 N+1 继续不重复复活
    /// （ADR 0002 §7 / R12.4）。
    pub revive: HeroReviveState,
    /// 主城升级三组权威状态（等级 / 升级队列 / 费用登记，v5 新增）。
    /// 帧 N 提交后的完整状态必须包含它们，恢复后从 N+1 继续不残留 /
    /// 不重复升级（ADR 0002 §7 / R12.4）。
    pub town_hall: TownHallUpgradeState,
    /// 野怪营地与掉落全部权威状态（v6 新增，P8.5c / Q1 / D1）。
    /// 帧 N 提交后的完整状态必须包含营地 / 野怪 / 中立归属 / 地面物品 /
    /// 物品实例 / 死亡登记 / 收益登记 / 随机码消费序列，恢复后从 N+1
    /// 继续不重复掉落、不残留上一局状态（ADR 0002 §6-8 / R12.4）。
    pub creep_camps: CreepCampState,
    /// 物品 / 商店 / 回城卷轴全部权威状态（v7 新增，P8.5d / Q4 / D1）。
    /// 帧 N 提交后的完整状态必须包含六格定长实例背包 / 复活死亡背包 /
    /// 拾取购买使用丢弃移交售卖事实 / 商店库存与刷新计数 / 回城传送 /
    /// 随机码消费序列 / 购买扣费登记，恢复后从 N+1 继续不残留 /
    /// 不重复购买或刷新（ADR 0002 §6-8 / R11.5 / R12.4）。
    pub item_shop: ItemShopState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotError {
    UnsupportedVersion { found: u32 },
    SimVersionMismatch { found: u32, expected: u32 },
    ContentMismatch,
    DefsMismatch,
    WorldMismatch,
    UnknownEntity(u64),
    MissingMotion,
    MalformedGroupPlan,
}
