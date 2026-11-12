//! 一局 1v1 Melee 对战的装配层（change `melee-playable-loop`）。
//!
//! 本模块只经 `w3-core` 既有公开 API 把玩法命令串成可玩闭环，**不新增任何
//! 权威状态**、不改动 `w3-core`（ADR 0002）。所有类型标签、AI 脚本与内容
//! 定义都是表现层 / 装配层数据，MUST NOT 进入 FNV-1a 状态哈希或
//! `SNAPSHOT_VERSION`（当前 3）定义的快照。

pub mod ai;
pub mod defs;
pub mod driver;
pub mod setup;

pub use ai::AiScript;
pub use defs::{ExportSource, synthetic_definitions};
pub use driver::MeleeDriver;
pub use setup::{AI_PLAYER, EntityKind, LOCAL_PLAYER, MeleeSetup, synthetic_world};
