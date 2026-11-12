//! w3-client：可玩客户端集成层（P6 批次 B）。
//!
//! 本 crate 是**表现层**，不是权威状态的所有者。唯一的权威写入者仍是
//! `w3_core::RtsMatch`（ADR 0002 §5）；本 crate 只经公开 API 提交命令、
//! 驱动 `step()`，并只读消费 `WorldView` 与事实事件。任何新状态——整数帧
//! 累加器余数、插值 `alpha`、窗口尺寸、选中集合、HUD 布局、相机位姿、
//! `Instant` 宿主时钟——一律为表现层专属，MUST NOT 进入 FNV-1a 状态哈希或
//! `SNAPSHOT_VERSION`（当前 3）定义的快照。
//!
//! 逻辑帧由冻结的 `logic_fps = 30` 整数帧累加器驱动（ADR 0002 §2）：
//! `step()` 不接受 delta、墙钟或刷新率；`RedrawRequested`、垂直同步与 GPU
//! 呈现耗时都不决定逻辑帧数。节拍源 [`app::HostClock`] 只在宿主层把墙钟
//! 增量换算成整数 tick，累加器决定推进多少个整数帧并逐个调用无参 `step()`。
//!
//! `FrameProducer` 是本地与联机对局共用的唯一只读呈现接缝；本地用
//! [`FixedStepClient`]、联机（批次 C）用 `ClientSession` 适配器，两者实现
//! 同一 [`app::StepDriver`] trait，共用 [`app::ClientApp`] 与 `w3-render` 的
//! `Scene` / `GpuContext` / `Camera` 呈现管线（design D1 / D6）。
//!
//! 分层：`w3-core` 零改动且不反向依赖本 crate；本批次依赖 `w3-core` /
//! `w3-render` / `w3-content` / `w3-app`（`freeze_f64` 冻结边界）/ `w3-net`
//! （批次 C 联机适配，只依赖不侵入），UI 用 `egui` 家族（取 MIT 一侧）。
//! 不引入通用游戏引擎（Bevy 等）或任何自动并行 ECS 调度
//! （ADR 0003「明确不做」、design D1）。
//!
//! 批次 C：[`net_driver::NetSessionDriver`] 把 `w3_net::ClientSession` 适配成
//! 同一 [`app::StepDriver`] 接缝，与本地 [`FixedStepClient`] 共用同一呈现
//! 管线；联机模式绝不直接调用 `step()`，唯一推进点是 `w3_net::apply_frame`
//! （一帧内普通命令与玩法命令按含固定 `type_tag` 的统一稳定键归并后全部
//! 提交，恰好一次无参 `step()`），两条路径不叠加。
//!
//! 命令携带由冻结帧号推导的 `ExecuteFrame` / `PlayerId` / `Sequence`；键鼠
//! 坐标到 `SimFixed` 只经 `w3_app::freeze_f64` 冻结边界（ADR 0004），
//! 不新增浮点到权威状态的路径。UI 中文文案走 [`i18n`]，不进日志、权威状态
//! 或状态哈希；字体用 egui 内置默认字体，不加载任何资产文件（ADR 0001）。

pub mod accumulator;
pub mod app;
pub mod camera;
pub mod client;
pub mod hud;
pub mod i18n;
pub mod input;
pub mod melee;
pub mod net_driver;
pub mod presentation;
pub mod real_map;

pub use accumulator::{
    FrameAccumulator, FrameAdvance, LOGIC_FPS, MAX_CATCHUP_STEPS, TICKS_PER_LOGIC_FRAME,
    TICKS_PER_SECOND,
};
pub use app::{ClientApp, ClientError, HostClock, StepDriver};
pub use camera::{CameraAction, CameraState};
pub use client::FixedStepClient;
pub use hud::{HudAssetError, HudAssets, HudModel, draw};
pub use i18n::{Texts, ZH};
pub use input::{
    ClientCommand, InputAction, InputError, InputState, LEAD_FRAMES, LOCAL_PLAYER, ScreenMapper,
};
pub use net_driver::NetSessionDriver;
pub use presentation::present;
