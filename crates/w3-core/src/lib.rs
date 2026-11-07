//! 确定性、与引擎无关的 RTS 模拟内核（P0 骨架）。
//!
//! 本 crate 独占权威模拟状态，不依赖渲染、UI、音频、网络、文件加载或墙钟时间。
//!
//! 确定性契约（见 `docs/decisions/0002-determinism.md`）：
//! - 所有权威状态数学只使用 [`fixed::SimFixed`] 整数运算。
//! - 对局按整数逻辑帧推进，调用方不能传入 delta。
//! - 命令按帧冻结后以稳定顺序执行。
//! - 状态哈希使用显式字节编码与稳定 ID 顺序。
//! - 随机流由对局独占，并写入每一份快照。
//!
//! 公共入口对应内核契约：`create`、`submit_command`、`step`、`read_view`、
//! `drain_events`、`capture_snapshot`、`restore_snapshot`、
//! `compute_state_hash`、`first_difference`。

pub mod command;
pub mod entity;
pub mod fixed;
pub mod frame;
pub mod sim_match;
pub mod snapshot;
pub mod state_hash;

pub use command::{Command, CommandError, CommandKind, CommandQueue};
pub use entity::{EntityId, EntityStore};
pub use fixed::{FRACTIONAL_BITS, FixedError, SCALE, SimFixed};
pub use frame::{Frame, FrameClock};
pub use sim_match::{Event, EventKind, MatchConfig, RtsMatch, WorldView};
pub use snapshot::{Snapshot, SnapshotError};
pub use state_hash::StateHasher;
