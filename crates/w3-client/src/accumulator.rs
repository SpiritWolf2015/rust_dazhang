//! 固定 30 Hz 整数帧累加器（客户端表现层时钟）。
//!
//! 本模块是纯整数状态机：只累计宿主显式喂入的整数节拍（ticks），把累计量
//! 按冻结的逻辑帧步长整除，商决定本轮应推进的 `RtsMatch::step()` 次数，
//! 余数决定表现层插值 `alpha`。它不读取墙钟、不接受 `f32` / `f64` 时间
//! 参数、不持有任何权威状态、也不写回模拟；相同整数节拍序列必然产生相同
//! 的逻辑帧序列（ADR 0002 §2、design D2）。
//!
//! 节拍分辨率取 30 / 60 / 144 的最小公倍数，令常见显示器刷新率都能整除
//! 成整数节拍，避免任何浮点或墙钟换算。逻辑帧数与 `RedrawRequested` 的
//! 次数无关，只取决于累计的整数节拍总量，因此 60 Hz 与 144 Hz 呈现同一
//! 命令脚本时逐帧帧号与状态哈希一致。

/// 冻结的逻辑帧率（ADR 0002 §2、ADR 0003「规模与帧率」）。
pub const LOGIC_FPS: u32 = 30;

/// 一秒的整数节拍数，取 30 / 60 / 144 的最小公倍数。
pub const TICKS_PER_SECOND: u64 = 720;

/// 一个逻辑帧对应的整数节拍数（720 / 30 = 24）。
pub const TICKS_PER_LOGIC_FRAME: u64 = TICKS_PER_SECOND / LOGIC_FPS as u64;

/// 单次追帧上限（spiral of death 保护）。一次 `advance` 最多补 8 个逻辑帧，
/// 超出部分以英文结构化日志明确报告后丢弃，MUST NOT 静默吞掉。
pub const MAX_CATCHUP_STEPS: u32 = 8;

/// 一次推进的换算结果，全部为表现层数据，绝不进入权威状态、状态哈希或快照。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameAdvance {
    /// 本轮应推进的整数逻辑帧数（已受追帧上限约束）。
    pub steps: u32,
    /// 表现层插值系数，只来自累加器余数，范围 `[0, 1)`。
    pub alpha: f32,
    /// 因追帧上限被显式丢弃的整数节拍数；0 表示未超限。
    pub dropped_ticks: u64,
}

/// 固定 30 Hz 整数帧累加器；纯表现层状态，不进哈希与快照。
#[derive(Clone, Debug, Default)]
pub struct FrameAccumulator {
    /// 累计但尚未换算成逻辑帧的整数节拍余数，恒小于一个逻辑帧步长。
    pending: u64,
}

impl FrameAccumulator {
    /// 创建零余数的累加器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 累计整数节拍并换算为逻辑帧数；MUST NOT 读取墙钟或调用 `step()`。
    ///
    /// 调用方（客户端事件循环）按 `plan.steps` 逐次调用无参 `RtsMatch::step()`；
    /// `plan.alpha` 只用于 `Scene::interpolate_all`。追帧超限时以英文
    /// `key=value` 日志明确报告并把余数收敛到不足一帧，防止无限补帧。
    pub fn advance(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        self.pending = self.pending.saturating_add(elapsed_ticks);
        let whole = self.pending / TICKS_PER_LOGIC_FRAME;
        let steps = whole.min(MAX_CATCHUP_STEPS as u64) as u32;
        self.pending -= u64::from(steps) * TICKS_PER_LOGIC_FRAME;

        // 追帧上限触发：剩余仍满一帧及以上的部分被显式丢弃并报告，
        // 只保留不足一帧的余数用于 alpha，避免 spiral of death。
        let mut dropped_ticks = 0;
        if self.pending >= TICKS_PER_LOGIC_FRAME {
            let frame_ticks = (self.pending / TICKS_PER_LOGIC_FRAME) * TICKS_PER_LOGIC_FRAME;
            dropped_ticks = frame_ticks;
            self.pending -= frame_ticks;
            log::warn!(
                target: "w3_client",
                "catch-up limit exceeded: cap={} dropped_ticks={} dropped_frames={}",
                MAX_CATCHUP_STEPS,
                dropped_ticks,
                dropped_ticks / TICKS_PER_LOGIC_FRAME,
            );
        }

        FrameAdvance {
            steps,
            alpha: self.alpha(),
            dropped_ticks,
        }
    }

    /// 表现层插值系数，仅来自累加器余数；`f32` 只存在于表现层，
    /// MUST NOT 写回位置 / 朝向 / 生命，也不进状态哈希或快照。
    pub fn alpha(&self) -> f32 {
        self.pending as f32 / TICKS_PER_LOGIC_FRAME as f32
    }

    /// 当前整数节拍余数，供诊断与验收；表现层专属。
    pub fn pending_ticks(&self) -> u64 {
        self.pending
    }
}
