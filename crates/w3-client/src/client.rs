//! 客户端确定性驱动：把整数帧累加器换算出的逻辑帧数，经公开 `step()`
//! 推进唯一权威状态 `RtsMatch`，并只交出只读视图与事件拷贝。
//!
//! 本模块严格遵守 ADR 0002 §5：不持有 `&mut RtsMatch` 的字段写入路径、
//! 不新增权威状态字段；命令只经 `submit_command` / `submit_gameplay_command`
//! 提交，`step()` 只以无参形式调用。累加器余数、`alpha` 等全部是表现层
//! 专属数据，不进入 FNV-1a 状态哈希与 `SNAPSHOT_VERSION`（当前 3）快照。

use w3_core::{
    Command, CommandError, Event, GameplayCommand, GameplayCommandError, MatchConfig, RtsMatch,
    StaticWorld, WorldView,
};
use w3_render::FrameProducer;

use crate::accumulator::{FrameAccumulator, FrameAdvance, LOGIC_FPS, TICKS_PER_LOGIC_FRAME};

/// 固定步进客户端：持有唯一 `RtsMatch` 与表现层累加器。
///
/// 它是本地对局的 `FrameProducer`；逻辑帧推进由 [`FixedStepClient::advance`]
/// 独立驱动，与 `next_frame()` 只读取帧彻底分离（BREAKING 语义）。
pub struct FixedStepClient {
    sim: RtsMatch,
    accumulator: FrameAccumulator,
}

impl FixedStepClient {
    /// 用冻结的 `logic_fps = 30` 创建对局，杜绝传入可变帧率。
    pub fn new(seed: u64, content_hash: u64, world: StaticWorld) -> Self {
        let config = MatchConfig {
            seed,
            logic_fps: LOGIC_FPS,
            content_hash,
        };
        Self {
            sim: RtsMatch::create_with_world(config, world),
            accumulator: FrameAccumulator::new(),
        }
    }

    /// 从既有对局装配（例如联机 session 恢复），校验其帧率仍为冻结值。
    pub fn from_match(sim: RtsMatch) -> Self {
        let fps = sim.config().logic_fps;
        assert_eq!(
            fps, LOGIC_FPS,
            "logic_fps must equal frozen value: expected={} actual={}",
            LOGIC_FPS, fps
        );
        Self {
            sim,
            accumulator: FrameAccumulator::new(),
        }
    }

    /// 喂入整数节拍，按累加器换算推进若干整数逻辑帧。
    ///
    /// `step()` 不接受 delta、墙钟或刷新率；推进次数只由累加器的整数余数
    /// 决定。返回换算结果供表现层用 `alpha` 插值，并报告追帧超限。
    pub fn advance(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        self.advance_with_observer(elapsed_ticks, |_| {})
    }

    /// 喂入整数节拍并在每个推进的逻辑帧后以只读方式回调 `observe`，供
    /// 重放验收逐帧采集帧号与状态哈希；生产路径用 [`Self::advance`]。
    /// `step()` 全程无参，从不接受 delta、墙钟或刷新率。
    pub fn advance_with_observer<F>(&mut self, elapsed_ticks: u64, mut observe: F) -> FrameAdvance
    where
        F: FnMut(&RtsMatch),
    {
        let plan = self.accumulator.advance(elapsed_ticks);
        for _ in 0..plan.steps {
            self.sim.step();
            observe(&self.sim);
        }
        plan
    }

    /// 便捷入口：按一个逻辑帧的整数步长推进恰好一帧。
    pub fn advance_one_frame(&mut self) -> FrameAdvance {
        self.advance(TICKS_PER_LOGIC_FRAME)
    }

    /// 命令只经 `RtsMatch::submit_command` 提交；失败显式返回错误，不静默降级。
    pub fn submit_command(&mut self, command: Command) -> Result<u64, CommandError> {
        self.sim.submit_command(command)
    }

    /// 玩法命令只经 `RtsMatch::submit_gameplay_command` 提交；失败显式返回错误。
    pub fn submit_gameplay_command(
        &mut self,
        command: GameplayCommand,
    ) -> Result<u64, GameplayCommandError> {
        self.sim.submit_gameplay_command(command)
    }

    /// 只读权威句柄，仅供 `compute_state_hash` / `capture_snapshot` /
    /// `read_view` 等确定性验收；类型系统只给出 `&RtsMatch`，无法写入。
    pub fn sim(&self) -> &RtsMatch {
        &self.sim
    }

    /// 当前表现层插值系数，只来自累加器余数。
    pub fn alpha(&self) -> f32 {
        self.accumulator.alpha()
    }
}

impl FrameProducer for FixedStepClient {
    /// 只交出当前只读视图与事件拷贝，MUST NOT 调用 `RtsMatch::step()`。
    fn next_frame(&mut self) -> (WorldView, Vec<Event>) {
        (self.sim.read_view(), self.sim.drain_events())
    }

    /// 表现层插值系数，只来自固定累加器的整数余数（ADR 0002 §2、design D2）。
    fn interpolation_alpha(&self) -> f32 {
        self.alpha()
    }
}
