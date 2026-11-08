//! 帧边界处已完成提交的完整、带版本的对局状态。
//!
//! 快照是第 N 帧提交完成后的状态，恢复后从第 N+1 帧继续。包含随机流、
//! 下一 ID 计数器、已接收但尚未执行的命令、全部单位订单与寻路状态、
//! 动态占地与群体规划进度。行为由 ID 与显式数据重建，绝不来自闭包、
//! 任务或活动对象。静态地图不嵌入快照，只记录其身份用于恢复校验。
//!
//! v2 由本 change 引入，开发期内静态地图身份编码（含 `origin`）定稿；
//! 未发布过其它 v2 快照，恢复时 `world_identity` 校验会拒绝编码不一致的
//! 世界，不提供跨版本迁移。

use crate::command::Command;

pub const SNAPSHOT_VERSION: u32 = 2;
pub const SIM_VERSION: u32 = 2;

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
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    pub version: u32,
    pub sim_version: u32,
    pub seed: u64,
    pub content_hash: u64,
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotError {
    UnsupportedVersion { found: u32 },
    SimVersionMismatch { found: u32, expected: u32 },
    ContentMismatch,
    WorldMismatch,
    UnknownEntity(u64),
    MissingMotion,
    MalformedGroupPlan,
}
