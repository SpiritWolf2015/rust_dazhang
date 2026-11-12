//! `MeleeDriver`：包装 `FixedStepClient`，注入 AI 外部命令与胜负检测
//! （design D1 / D3）。
//!
//! 本结构只经两个公开提交入口入队命令，不持有第二份对局状态；AI 脚本
//! 与「敌方主城消失 → `Leave`」检测都是装配层逻辑，`w3-core` 零改动。
//! 胜负由既有 `resolve_victory` 在内核既定帧阶段结算。

use w3_core::{Event, Frame, GameplayCommand, GameplayCommandKind, Snapshot, WorldView};
use w3_render::FrameProducer;

use crate::accumulator::FrameAdvance;
use crate::app::{ClientError, StepDriver};
use crate::client::FixedStepClient;
use crate::input::ClientCommand;
use crate::melee::ai::AiScript;
use crate::melee::setup::{AI_PLAYER, MeleeSetup};

/// `Leave` 命令使用的高位序列，避免与 AI 脚本序列冲突。
const LEAVE_SEQUENCE: u32 = 1_000_000;

/// 本地一局 Melee 的 `StepDriver` 实现。
pub struct MeleeDriver {
    inner: FixedStepClient,
    ai_hall: u64,
    defeat_submitted: bool,
}

impl MeleeDriver {
    /// 由已装配的开局驱动与 AI 脚本构造。
    ///
    /// 全部 AI 外部命令在开局**一次性入队**，带固定的 `ExecuteFrame` /
    /// `PlayerId` / `Sequence`，由内核按帧冻结执行（ADR 0002）。这样命令
    /// 提交时机与 `advance_ticks` 的墙钟调用粒度无关，不同刷新率 / 卡顿
    /// 下逐帧状态哈希一致；重放注入同一脚本即可复现。
    pub fn new(inner: FixedStepClient, setup: &MeleeSetup, horizon_frames: u64) -> Self {
        let mut driver = Self {
            inner,
            ai_hall: setup.ai_hall.value(),
            defeat_submitted: false,
        };
        for command in AiScript::new(setup, horizon_frames).commands {
            if let Err(error) = driver.inner.submit_gameplay_command(command) {
                log::warn!(target: "w3_client", "ai command rejected: error={}", error);
            }
        }
        driver
    }

    /// 只读驱动句柄，供确定性验收读取状态哈希与快照。
    pub fn sim(&self) -> &w3_core::RtsMatch {
        self.inner.sim()
    }
}

impl StepDriver for MeleeDriver {
    fn advance_ticks(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        let ai_hall = self.ai_hall;
        let mut defeat_submitted = self.defeat_submitted;
        // 每个逻辑帧 `step()` 之前检测敌方主城是否仍存活：一旦消失，按当前
        // 冻结帧号提交 `Leave`，由既有 `resolve_victory` 结算。逐帧检测避免
        // 一次推进多帧时把结算帧号绑定到墙钟调用粒度。
        let plan = self.inner.advance_with_hook(elapsed_ticks, |sim| {
            if defeat_submitted {
                return;
            }
            let view = sim.read_view();
            // 开局尚未执行 Spawn 时视图为空，不能据此判定主城被毁。
            if view.entities.is_empty() {
                return;
            }
            if view.entities.iter().any(|e| e.id == ai_hall && e.alive) {
                return;
            }
            let frame = Frame::new(view.tick.saturating_add(1));
            let command = GameplayCommand::new(
                frame,
                AI_PLAYER,
                LEAVE_SEQUENCE,
                GameplayCommandKind::Leave { player: AI_PLAYER },
            );
            if let Err(error) = sim.submit_gameplay_command(command) {
                log::warn!(target: "w3_client", "ai leave rejected: error={}", error);
            }
            defeat_submitted = true;
        });
        self.defeat_submitted = defeat_submitted;
        plan
    }

    fn enqueue(&mut self, command: ClientCommand) -> Result<(), ClientError> {
        StepDriver::enqueue(&mut self.inner, command)
    }

    fn view(&self) -> WorldView {
        self.inner.sim().read_view()
    }

    fn snapshot(&self) -> Snapshot {
        self.inner.sim().capture_snapshot()
    }
}

impl FrameProducer for MeleeDriver {
    fn next_frame(&mut self) -> (WorldView, Vec<Event>) {
        FrameProducer::next_frame(&mut self.inner)
    }

    fn interpolation_alpha(&self) -> f32 {
        self.inner.alpha()
    }
}
