//! 帧边界处已完成提交的完整、带版本的对局状态。
//!
//! 快照是第 N 帧提交完成后的状态，恢复后从第 N+1 帧继续。包含随机流、
//! 下一 ID 计数器，以及已接收但尚未执行的命令。行为由 ID 与显式数据重建，
//! 绝不来自闭包、任务或活动对象。

use crate::command::Command;

pub const SNAPSHOT_VERSION: u32 = 1;
pub const SIM_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PositionRecord {
    pub entity: u64,
    pub x: i64,
    pub y: i64,
    pub target: Option<(i64, i64)>,
    pub speed: i64,
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotError {
    UnsupportedVersion { found: u32 },
    SimVersionMismatch { found: u32, expected: u32 },
    ContentMismatch,
    UnknownEntity(u64),
    MissingMotion,
}
