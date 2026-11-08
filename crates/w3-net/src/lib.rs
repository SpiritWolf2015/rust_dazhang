//! w3-net：确定性 lockstep 联机传输层。
//!
//! 本 crate 只编排玩家输入命令，绝不传输或反写游戏状态；`w3-core`
//! 不依赖本 crate。到达顺序不决定执行顺序：命令按稳定键排序后依序
//! 提交，使 `Command.arrival` 在每个客户端一致（见 ADR 0002）。
//!
//! 模块分层：
//! - [`packet`]：本项目私有 `W3N1` 线格式（不复用 RA2 包布局）。
//! - [`transport`]：真实 UDP 与可注入故障的内存回环。
//! - [`reliable`]：按 sequence 去重、ACK、确定性 tick 重传。
//! - [`frame`]：帧边界编排（客户端排序 / relay 缺失输入关闭）。
//! - [`checksum`]：逐帧校验交换与 desync 显式上报。
//! - [`reconnect`]：断线重连、追帧、重放不重复执行。
//! - [`session`]：以上组件的客户端会话（不持有 `RtsMatch`）。

pub mod checksum;
pub mod error;
pub mod frame;
pub mod logging;
pub mod packet;
pub mod reconnect;
pub mod reliable;
pub mod session;
pub mod transport;

pub use checksum::{ChecksumExchange, DesyncReport, FrameChecksum, first_difference_field};
pub use error::NetError;
pub use frame::{
    DEFAULT_INPUT_TIMEOUT_FRAMES, FrameAssembler, FrameScheduler, FrameStep, apply_frame, stabilize,
};
pub use logging::init_logging;
pub use packet::{FrameCommands, MAGIC, PROTOCOL_VERSION, Packet, PacketKind};
pub use reconnect::{
    ReconnectController, ReconnectPhase, SessionCursor, resync_replay_packet, resync_request_packet,
};
pub use reliable::{ReliableReceiver, ReliableSender, ack_packet};
pub use session::ClientSession;
pub use transport::{
    Endpoint, FaultInjection, LoopbackBus, LoopbackTransport, SharedBus, Transport, UdpTransport,
};
