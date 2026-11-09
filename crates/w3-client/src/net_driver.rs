//! 联机会话适配器（tasks 8.1–8.5、design D6、ADR 0005）。
//!
//! 把 `w3_net::ClientSession<T>` 适配成批次 B 定义的 [`StepDriver`] 接缝，
//! 从而与本地 [`FixedStepClient`] 共用同一 [`ClientApp`] / `Scene` /
//! `GpuContext` / `Camera` / `FrameProducer` 呈现管线，不出现第二套呈现
//! 代码，也不出现第二份 `RtsMatch` 权威状态。
//!
//! **双 `step()` 规避（批次 C 最高风险点，design D6）**：
//! - 本地模式：[`FixedStepClient::advance`] 由纯整数累加器换算出整数帧数，
//!   逐个调用**无参** `RtsMatch::step()`；
//! - 联机模式：本适配器**绝不直接调用 `sim.step()`**。唯一的模拟推进点是
//!   [`w3_net::apply_frame`]（`w3-net/src/frame.rs:30`，其第 45 行内部
//!   **恰好一次** `sim.step()`，第 46 行返回 `compute_state_hash()`）。
//! - 联机下的 [`FrameAccumulator`] 只做整数 tick → 整数帧预算换算与表现层
//!   `alpha` 余数，MUST NOT 推进模拟。两条路径按 `StepDriver` 实现二选一，
//!   在任何一次 `advance_ticks` 中绝不叠加。
//!
//! 节拍隔离（ADR 0002 §2、design D6）：节拍源仍是 `ClientApp::about_to_wait`
//! 的 `HostClock` 整数 tick；网络包到达 / 抖动 / 重传 / 乱序 / 断线恢复
//! 只经 `poll()` 把权威帧命令排入就绪队列，MUST NOT 驱动累加器、改 `step()`
//! 次数或改命令排序。缺输入按帧边界等待（不按墙钟补帧、不无限阻塞），
//! `FrameScheduler` 超时语义不变，超时以英文结构化日志明确报告。
//!
//! 命令排序：`apply_frame` 内部调用 `stabilize`，按
//! `ExecuteFrame → PlayerId → Sequence` 稳定排序后依序 `submit_command`，
//! 使 `Command.arrival` 在每个客户端一致。联机键鼠命令只经
//! `ClientSession::submit_input` 走上行广播，MUST NOT 本地 `submit_command`。

use w3_core::{Event, MatchConfig, RtsMatch, Snapshot, StaticWorld, WorldView};
use w3_net::{ClientSession, NetError, Transport, apply_frame};
use w3_render::FrameProducer;

use crate::accumulator::{FrameAccumulator, FrameAdvance, LOGIC_FPS};
use crate::app::{ClientError, StepDriver};
use crate::input::ClientCommand;

/// 本地与联机共用的呈现管线驱动（联机实现）。
///
/// 持有唯一 `RtsMatch` 权威写入者；`ClientSession` 只编排输入命令与校验值，
/// 不持有模拟（见 `w3-net/src/session.rs` 契约）。`applied_trace` 是纯表现层
/// 诊断数据，MUST NOT 进入状态哈希或快照。
pub struct NetSessionDriver<T: Transport> {
    sim: RtsMatch,
    session: ClientSession<T>,
    accumulator: FrameAccumulator,
    applied_trace: Vec<(u64, u64)>,
    timeout_reported: bool,
}

impl<T: Transport> NetSessionDriver<T> {
    /// 用冻结的 `logic_fps = 30` 创建本地权威状态，并接管一个联机会话。
    pub fn new(
        seed: u64,
        content_hash: u64,
        world: StaticWorld,
        session: ClientSession<T>,
    ) -> Self {
        let config = MatchConfig {
            seed,
            logic_fps: LOGIC_FPS,
            content_hash,
        };
        Self {
            sim: RtsMatch::create_with_world(config, world),
            session,
            accumulator: FrameAccumulator::new(),
            applied_trace: Vec::new(),
            timeout_reported: false,
        }
    }

    /// 只读权威句柄（验收用）；类型系统只给出 `&RtsMatch`，无法写入字段。
    pub fn sim(&self) -> &RtsMatch {
        &self.sim
    }

    /// 只读会话句柄（校验值 / 调度器 / 重连游标诊断）。
    pub fn session(&self) -> &ClientSession<T> {
        &self.session
    }

    /// 可写会话句柄（供宿主注入权威帧包 / 发起重同步）。
    pub fn session_mut(&mut self) -> &mut ClientSession<T> {
        &mut self.session
    }

    /// 已应用的冻结帧 `(frame, compute_state_hash())` 序列（表现层诊断）。
    pub fn applied_trace(&self) -> &[(u64, u64)] {
        &self.applied_trace
    }

    /// 当前表现层插值系数，只来自累加器余数。
    pub fn alpha(&self) -> f32 {
        self.accumulator.alpha()
    }

    /// 断线恢复：由宿主在 `FrameScheduler` 超时后显式调用。
    /// 只发重连包，MUST NOT 驱动累加器或改变 `step()` 次数。
    pub fn request_resync(&mut self) -> Result<(), NetError> {
        let result = self.session.request_resync();
        if let Err(error) = &result {
            log::warn!(target: "w3_client", "resync request failed: error={}", error);
        }
        self.timeout_reported = false;
        result
    }

    /// 轮询网络（只把权威帧命令排入就绪队列，不推进模拟）。
    fn poll_network(&mut self) {
        if let Err(error) = self.session.poll() {
            log::warn!(target: "w3_client", "network poll failed: error={}", error);
        }
    }

    /// 按整数帧预算消费已冻结帧；**唯一**的模拟推进点。
    ///
    /// 每个预算帧恰好对应一个 `pop_ready()` 冻结帧，经 `apply_frame` 完成
    /// 「稳定排序 → 依序 `submit_command` → 恰好一次无参 `step()`」。
    /// 缺输入时本帧不推进（按帧边界等待），不按墙钟补帧、不无限阻塞。
    fn consume_ready_frames(&mut self, budget: u32) {
        for _ in 0..budget {
            let Some(frame) = self.session.pop_ready() else {
                break;
            };
            match apply_frame(&mut self.sim, &frame.commands) {
                Ok(hash) => {
                    // ADR 0005：逐帧上报状态哈希（`apply_frame` 返回的正是
                    // `compute_state_hash()`），不传输游戏状态。
                    if let Err(error) = self.session.report_checksum(frame.frame, hash) {
                        log::warn!(
                            target: "w3_client",
                            "checksum report failed: frame={} error={}",
                            frame.frame,
                            error,
                        );
                    }
                    self.applied_trace.push((frame.frame, hash));
                }
                Err(error) => {
                    // 显式失败：以英文结构化日志报告，MUST NOT 静默跳过该帧。
                    log::warn!(
                        target: "w3_client",
                        "lockstep frame rejected: frame={} commands={} error={}",
                        frame.frame,
                        frame.commands.len(),
                        error,
                    );
                    break;
                }
            }
        }
    }

    /// 推进网络内部 tick（重传 / 缺帧超时），不触碰 `RtsMatch`。
    fn tick_network(&mut self) {
        match self.session.tick() {
            Ok(true) => {
                if !self.timeout_reported {
                    log::warn!(
                        target: "w3_client",
                        "frame input timeout: frame={} action=request_resync",
                        self.session.scheduler().next_frame(),
                    );
                    self.timeout_reported = true;
                }
            }
            Ok(false) => self.timeout_reported = false,
            Err(error) => {
                log::warn!(target: "w3_client", "network tick failed: error={}", error);
            }
        }
    }
}

impl<T: Transport> FrameProducer for NetSessionDriver<T> {
    /// 只交出当前只读视图与事件拷贝，MUST NOT 推进模拟。
    fn next_frame(&mut self) -> (WorldView, Vec<Event>) {
        (self.sim.read_view(), self.sim.drain_events())
    }

    /// 表现层插值系数，只来自固定累加器的整数余数。
    fn interpolation_alpha(&self) -> f32 {
        self.accumulator.alpha()
    }
}

impl<T: Transport> StepDriver for NetSessionDriver<T> {
    /// 按整数 tick 换算帧预算并消费就绪帧。
    ///
    /// 本地模式由累加器直接 `step()`；本联机实现**不**调用 `step()`，
    /// 把推进完全交给 [`w3_net::apply_frame`]。累加器只产出一个整数帧
    /// 预算与表现层 `alpha`，二者绝不叠加。
    fn advance_ticks(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        // 1) 纯整数累加器：tick → 帧预算 + 表现层 alpha（不推进模拟）。
        let plan = self.accumulator.advance(elapsed_ticks);
        // 2) 网络包到达只填充就绪队列，MUST NOT 驱动累加器或改帧序。
        self.poll_network();
        // 3) 帧预算 → 已冻结帧 → apply_frame（唯一模拟推进点，内部一次 step）。
        self.consume_ready_frames(plan.steps);
        // 4) 网络 tick（重传 / 超时），不触碰 RtsMatch。
        self.tick_network();
        plan
    }

    /// 联机键鼠命令只走 `ClientSession::submit_input` 上行广播，
    /// MUST NOT 本地 `submit_command`（否则不进 lockstep 广播）。
    /// `GameplayCommand` 不在 ADR 0005 protobuf schema 内，显式返回错误。
    fn enqueue(&mut self, command: ClientCommand) -> Result<(), ClientError> {
        match command {
            ClientCommand::Simulation(command) => {
                let frame = command.execute_frame.value();
                match self.session.submit_input(frame, &[command]) {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        log::warn!(
                            target: "w3_client",
                            "input uplink failed: frame={} player={} error={}",
                            frame,
                            command.player_id,
                            error,
                        );
                        Err(ClientError::Network)
                    }
                }
            }
            ClientCommand::Gameplay(_) => {
                // ADR 0005：W3N1 protobuf 载荷只承载 `CommandKind`，
                // 不含 `GameplayCommandKind`（Cast / Train / CancelTrain）。
                log::warn!(
                    target: "w3_client",
                    "gameplay command unsupported: reason=wire_format_command_only",
                );
                Err(ClientError::GameplayUnsupportedOnline)
            }
        }
    }

    fn view(&self) -> WorldView {
        self.sim.read_view()
    }

    fn snapshot(&self) -> Snapshot {
        self.sim.capture_snapshot()
    }
}
