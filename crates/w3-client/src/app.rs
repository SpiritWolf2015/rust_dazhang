//! 真实窗口宿主应用（B2：累加器节拍接入 winit 事件循环；tasks 4.1、7.3）。
//!
//! [`ClientApp`] 组合复用 `w3-render` 的 [`GpuContext`] / [`Scene`] / [`Camera`]
//! 与 [`FrameProducer`] 接缝（design D1 CARP、D6），本地模式用
//! [`FixedStepClient`]、联机模式用 [`crate::net_driver::NetSessionDriver`]，
//! 两者实现同一 [`StepDriver`] trait，共用同一呈现管线。
//!
//! **时钟归客户端宿主**（design D2 SRP）：[`HostClock`] 是唯一使用墙钟
//! （`std::time::Instant`）的地方，只把经过的纳秒换算成**整数 tick** 后喂给
//! [`FixedStepClient::advance`]；累加器为纯整数状态机，决定本轮推进多少个
//! 整数逻辑帧，并逐个调用**无参** `RtsMatch::step()`。
//!
//! 硬红线：
//! - `RedrawRequested` / VSync / GPU 耗时 MUST NOT 决定逻辑帧数（节拍在
//!   `about_to_wait` 累计，呈现只读）；
//! - `WindowEvent::Resized` 只更新 aspect 与交换链，MUST NOT 触发 `step()`；
//! - 墙钟 `Instant` 只存在于本宿主层，MUST NOT 进入 `w3-core`、状态哈希或快照；
//! - 命令只经 `submit_command` / `submit_gameplay_command` 提交，失败显式报错；
//! - HUD 只读，选中集合 / 相机位姿 / 累加器余数一律表现层专属。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use egui_wgpu::ScreenDescriptor;
use w3_content::RgbaImage;
use w3_core::{CommandError, Event, GameplayCommandError, Snapshot, WorldView};
use w3_render::{
    BakedModel, Camera, FrameInput, FrameProducer, GpuContext, ModelLibrary, RenderError, Scene,
    SceneVertex, StaticMarker, TerrainMesh,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::accumulator::{FrameAdvance, TICKS_PER_SECOND};
use crate::camera::{CameraAction, CameraState, PAN_STEP, YAW_STEP, ZOOM_STEP};
use crate::client::FixedStepClient;
use crate::hud::{self, HudAssets, HudModel, MinimapBackdrop, minimap_backdrop};
use crate::i18n::{Texts, ZH};
use crate::input::{ClientCommand, InputAction, InputError, InputState, ScreenMapper};
use crate::melee::EntityKind;

const NANOS_PER_SECOND: u128 = 1_000_000_000;
/// 小地图底图的降采样网格边长（纯表现层参数，不进权威状态 / 哈希）。
const MINIMAP_GRID: usize = 32;

/// F1..F9 映射到编组编号（表现层，不进权威状态）。
fn assign_group_number(key: KeyCode) -> Option<u8> {
    match key {
        KeyCode::F1 => Some(1),
        KeyCode::F2 => Some(2),
        KeyCode::F3 => Some(3),
        KeyCode::F4 => Some(4),
        KeyCode::F5 => Some(5),
        KeyCode::F6 => Some(6),
        KeyCode::F7 => Some(7),
        KeyCode::F8 => Some(8),
        KeyCode::F9 => Some(9),
        _ => None,
    }
}

/// 数字键 1..9 映射到按组选择编号。
fn select_group_number(key: KeyCode) -> Option<u8> {
    match key {
        KeyCode::Digit1 => Some(1),
        KeyCode::Digit2 => Some(2),
        KeyCode::Digit3 => Some(3),
        KeyCode::Digit4 => Some(4),
        KeyCode::Digit5 => Some(5),
        KeyCode::Digit6 => Some(6),
        KeyCode::Digit7 => Some(7),
        KeyCode::Digit8 => Some(8),
        KeyCode::Digit9 => Some(9),
        _ => None,
    }
}

/// 宿主错误；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientError {
    /// 普通命令被内核拒绝。
    Command(CommandError),
    /// 玩法命令被内核拒绝。
    Gameplay(GameplayCommandError),
    /// 输入翻译前置条件不满足。
    Input(InputError),
    /// 网络提交 / 轮询失败（详情以英文结构化 `key=value` 日志输出）。
    Network,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(error) => write!(f, "command rejected: {error:?}"),
            Self::Gameplay(error) => write!(f, "gameplay command rejected: {error}"),
            Self::Input(error) => write!(f, "input rejected: {error}"),
            Self::Network => f.write_str("network input failed"),
        }
    }
}

/// 宿主时钟：把墙钟增量换算成整数 tick 的**唯一**位置（宿主层）。
///
/// 内部只做整数纳秒运算（无浮点、无 delta 传给 `step()`）。`Instant` 不外泄，
/// 换算结果 `u64` tick 是纯整数，喂给累加器后由累加器决定逻辑帧数。
#[derive(Debug, Default)]
pub struct HostClock {
    last: Option<Instant>,
    carry_nanos: u128,
}

impl HostClock {
    /// 创建未启动的宿主时钟（首次 `poll_ticks` 返回 0）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 取走自上次调用以来累计的整数 tick 数。
    ///
    /// 墙钟只在此处读取；`carry_nanos` 保留不足一个 tick 的余数，避免
    /// 长时间运行的整数截断漂移。全程无浮点运算。
    pub fn poll_ticks(&mut self) -> u64 {
        let now = Instant::now();
        let delta_nanos = match self.last {
            Some(last) => now.saturating_duration_since(last).as_nanos(),
            None => 0,
        };
        self.last = Some(now);
        let scaled = delta_nanos
            .saturating_mul(TICKS_PER_SECOND as u128)
            .saturating_add(self.carry_nanos);
        let ticks = scaled / NANOS_PER_SECOND;
        self.carry_nanos = scaled % NANOS_PER_SECOND;
        ticks.min(u64::MAX as u128) as u64
    }
}

/// 本地与联机共用的呈现管线驱动接缝（design D6 / LSP）。
///
/// 累加器 / 时钟归客户端宿主；`FrameProducer` 只负责交出当前只读视图与
/// 事件拷贝，MUST NOT 推进模拟。联机模式的
/// [`crate::net_driver::NetSessionDriver`] 实现本 trait 即可复用同一
/// [`ClientApp`]，网络帧到达 MUST NOT 驱动累加器。
pub trait StepDriver: FrameProducer {
    /// 按整数 tick 推进累加器（由累加器调用无参 `step()`）。
    fn advance_ticks(&mut self, elapsed_ticks: u64) -> FrameAdvance;
    /// 只经两个公开提交入口入队命令；失败显式返回错误。
    fn enqueue(&mut self, command: ClientCommand) -> Result<(), ClientError>;
    /// 只读视图（用于输入命中测试与 HUD，不推进模拟）。
    fn view(&self) -> WorldView;
    /// 只读快照拷贝（用于 HUD 资源 / 生产队列，不推进模拟）。
    fn snapshot(&self) -> Snapshot;
}

impl StepDriver for FixedStepClient {
    fn advance_ticks(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        self.advance(elapsed_ticks)
    }

    fn enqueue(&mut self, command: ClientCommand) -> Result<(), ClientError> {
        match command {
            ClientCommand::Simulation(command) => self
                .submit_command(command)
                .map(|_| ())
                .map_err(ClientError::Command),
            ClientCommand::Gameplay(command) => self
                .submit_gameplay_command(command)
                .map(|_| ())
                .map_err(ClientError::Gameplay),
        }
    }

    fn view(&self) -> WorldView {
        self.sim().read_view()
    }

    fn snapshot(&self) -> Snapshot {
        self.sim().capture_snapshot()
    }
}

/// 真实 winit 窗口应用：装配固定步进、输入、HUD、相机与 egui。
pub struct ClientApp<P: StepDriver> {
    driver: P,
    scene: Scene,
    terrain: TerrainMesh,
    library: ModelLibrary,
    camera: CameraState,
    input: InputState,
    hud_assets: HudAssets,
    texts: Texts,
    /// 真实小地图底图的 CPU 降采样颜色网格（纯表现层，注入后 HUD 只读绘制）。
    minimap_backdrop: Option<MinimapBackdrop>,
    /// 装配层注入的实体类型标签（表现层专属，不进权威状态 / 哈希 / 快照）。
    entity_kinds: BTreeMap<u64, EntityKind>,
    /// 命令卡点击待翻译的输入动作（表现层，在 GPU 借用结束后提交）。
    pending_input_action: Option<InputAction>,

    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,

    /// 唯一墙钟，只在宿主层用于换算整数 tick。
    clock: HostClock,
    cursor: [f64; 2],
    dragging: bool,
    last_events: Vec<Event>,
    max_frames: u32,
    frames: u32,
    last_error: Option<ClientError>,
}

impl<P: StepDriver + 'static> ClientApp<P> {
    /// 装配应用；`max_frames` 限制呈现帧数，便于无人值守验收。
    pub fn new(driver: P, terrain: TerrainMesh, library: ModelLibrary, max_frames: u32) -> Self {
        let extent = terrain.columns.max(terrain.rows) as f64 * terrain.cell_size as f64;
        let _ = extent; // HUD 拾取映射在 `mapper` 内按当前地形实时计算
        Self {
            driver,
            scene: Scene::default(),
            terrain,
            library,
            camera: CameraState::default(),
            input: InputState::default(),
            hud_assets: HudAssets::degraded(),
            texts: ZH,
            minimap_backdrop: None,
            entity_kinds: BTreeMap::new(),
            pending_input_action: None,
            window: None,
            gpu: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            egui_renderer: None,
            clock: HostClock::new(),
            cursor: [0.0; 2],
            dragging: false,
            last_events: Vec::new(),
            max_frames,
            frames: 0,
            last_error: None,
        }
    }

    /// 注入实体类型标签（表现层数据），供命令卡 / 选择面板只读派生。
    /// 不改动既有 `new` 签名，既有入口不受影响。
    pub fn with_entity_kinds(mut self, kinds: BTreeMap<u64, EntityKind>) -> Self {
        self.entity_kinds = kinds;
        self
    }

    /// 注入导出地图的静态摆放（真实地图入口用，纯表现层）。
    /// `consume` 不触碰这些标记；既有入口不受影响。
    pub fn with_static_markers(mut self, markers: Vec<StaticMarker>) -> Self {
        self.scene.set_static_markers(markers);
        self
    }

    /// 注入真实地表 tileset 贴图（6 层 RGBA8，纯表现层 CPU 采样数据）。
    /// `None` 时地形回退到 6 色调色板；既有入口不受影响。
    pub fn with_terrain_tiles(mut self, tiles: Option<Vec<RgbaImage>>) -> Self {
        self.scene.set_terrain_tiles(tiles);
        self
    }

    /// 注入真实小地图底图并降采样为颜色网格（纯 CPU，不上传 GPU 纹理）。
    pub fn with_minimap_image(mut self, image: Option<RgbaImage>) -> Self {
        self.minimap_backdrop = image.map(|image| minimap_backdrop(&image, MINIMAP_GRID));
        self
    }

    /// 注入真实地图静态摆放的 glTF 模型（预展开顶点，纯表现层）。
    /// 非空时替代程序化几何；既有入口不受影响。
    pub fn with_static_model_vertices(mut self, vertices: Vec<SceneVertex>) -> Self {
        self.scene.set_static_model_vertices(vertices);
        self
    }

    /// 注入动态实体类型键 → 真实烘焙模型（纯表现层）。
    pub fn with_entity_models(mut self, models: BTreeMap<String, BakedModel>) -> Self {
        self.scene.set_entity_models(models);
        self
    }

    /// 设置初始相机（拉近到局部战场视角 + 平移到出生点，纯表现层）。
    ///
    /// 真实 WC3 模型按真实比例时，默认 `zoom=1` 俯瞰全图会把农民 / 建筑压成
    /// 几个像素。此 builder 让可玩入口一打开就呈现可辨的局部战场（RTS 常规
    /// 视角），玩家仍可用滚轮 / WASD 自由缩放平移查看全图。不改 `CameraState`
    /// 全局默认，既有入口与验收测试不受影响；`set_initial_view` 显式赋值并
    /// clamp，不隐含依赖默认 zoom、重复调用幂等。
    pub fn with_initial_camera(mut self, zoom: f32, pan: [f32; 2]) -> Self {
        self.camera.set_initial_view(zoom, pan);
        self
    }

    /// 运行 winit 事件循环直到关闭或达到 `max_frames`。
    pub fn run(mut self) -> Result<(), RenderError> {
        let event_loop = EventLoop::builder()
            .build()
            .map_err(|e| RenderError::Surface(format!("event loop: {e:?}")))?;
        event_loop
            .run_app(&mut self)
            .map_err(|e| RenderError::Surface(format!("run_app: {e:?}")))
    }

    /// 只读驱动句柄（验收用）。
    pub fn driver(&self) -> &P {
        &self.driver
    }

    /// 可写驱动句柄（验收用；只能调公开 API，字段私有）。
    pub fn driver_mut(&mut self) -> &mut P {
        &mut self.driver
    }

    /// 只读当前 HUD 展示数据；不改变任何权威状态。
    pub fn hud_model(&self) -> HudModel {
        let view = self.driver.view();
        let snapshot = self.driver.snapshot();
        let (width, height) = self.viewport_size();
        let mapper = self.mapper(width, height);
        HudModel::collect(
            &view,
            &snapshot,
            self.input.selection(),
            &self.last_events,
            &mapper,
            self.input.local_player(),
        )
        .with_kinds(self.entity_kinds.clone())
        .with_minimap_backdrop(self.minimap_backdrop.clone())
    }

    /// 最近一次显式错误（输入 / 命令提交失败），供验收读取。
    pub fn last_error(&self) -> Option<ClientError> {
        self.last_error
    }

    /// 由宿主显式喂入整数 tick（验收用；真实循环由 [`HostClock`] 驱动）。
    /// 返回值中的 `steps` 即本轮推进的整数逻辑帧数，`step()` 全程无参。
    pub fn advance_with_ticks(&mut self, elapsed_ticks: u64) -> FrameAdvance {
        self.driver.advance_ticks(elapsed_ticks)
    }

    /// 应用一个归一化输入动作（验收与 winit 事件共用同一路径）。
    pub fn apply_input(&mut self, action: InputAction) -> Result<Vec<ClientCommand>, ClientError> {
        let view = self.driver.view();
        let (width, height) = self.viewport_size();
        let mapper = self.mapper(width, height);
        let events = self.last_events.clone();
        let commands = self
            .input
            .apply(action, &view, &mapper, &events)
            .map_err(ClientError::Input)?;
        for command in &commands {
            self.driver.enqueue(command.clone())?;
        }
        Ok(commands)
    }

    /// 应用一个归一化相机动作（纯表现层状态）。
    pub fn apply_camera(&mut self, action: CameraAction) {
        self.camera.apply(action);
    }

    /// 当前相机（表现层位姿，交给 `w3-render` 生成 MVP）。
    pub fn camera(&self) -> Camera {
        self.camera.camera()
    }

    /// HUD 字体 / 文案资源状态。
    pub fn hud_assets(&self) -> HudAssets {
        self.hud_assets
    }

    /// HUD i18n 文案。
    pub fn texts(&self) -> Texts {
        self.texts
    }

    /// 只读选中集合（表现层）。
    pub fn selection(&self) -> &std::collections::BTreeSet<u64> {
        self.input.selection()
    }

    fn viewport_size(&self) -> (f64, f64) {
        match &self.gpu {
            Some(gpu) => (gpu.width as f64, gpu.height as f64),
            None => (1280.0, 720.0),
        }
    }

    fn mapper(&self, width: f64, height: f64) -> ScreenMapper {
        let extent = (self.terrain.columns.max(self.terrain.rows) as f64
            * self.terrain.cell_size as f64)
            .max(1.0);
        // 拾取复用与 `render_one` 同源的 `Camera::matrix` 投影，消除
        // 「渲染用透视、拾取用线性」两套口径造成的框选 / 命中错位。
        let projection = w3_render::ScreenProjection::new(
            &self.camera.camera(),
            &self.terrain,
            width as f32,
            height as f32,
        );
        // winit 鼠标与 GPU 交换链是物理像素，egui HUD 是逻辑像素；把窗口
        // 缩放比交给 `ScreenMapper`，HUD 绘制前经 `to_logical` 换算。
        let pixels_per_point = self
            .window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor() as f64);
        ScreenMapper::new(
            width,
            height,
            extent,
            [self.camera.pan()[0] as f64, self.camera.pan()[1] as f64],
            self.camera.zoom() as f64,
        )
        .with_projection(projection)
        .with_pixels_per_point(pixels_per_point)
    }

    /// 当前拾取映射（供验收 / 诊断读取，不推进模拟）。
    pub fn screen_mapper(&self) -> ScreenMapper {
        let (width, height) = self.viewport_size();
        self.mapper(width, height)
    }

    fn render_one(&mut self) -> Result<(), RenderError> {
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError::GpuUnavailable("gpu context not ready".into()))?;
        let window = self
            .window
            .as_ref()
            .ok_or_else(|| RenderError::GpuUnavailable("window not ready".into()))?;

        // 只交出当前只读视图与事件拷贝；逻辑帧推进绝不在此发生。
        let (view, events) = self.driver.next_frame();
        self.scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &self.library,
        );
        // alpha 只来自累加器余数，仅存在于表现层。
        let alpha = self.driver.interpolation_alpha();
        self.scene.interpolate_all(alpha);
        let vertices = self.scene.build_vertices(&self.terrain);
        let view_proj = self.camera.camera().matrix(&self.terrain);

        let (width, height) = self.viewport_size();
        let mapper = self.mapper(width, height);
        let snapshot = self.driver.snapshot();
        let hud_model = HudModel::collect(
            &view,
            &snapshot,
            self.input.selection(),
            &events,
            &mapper,
            self.input.local_player(),
        )
        .with_kinds(self.entity_kinds.clone())
        .with_minimap_backdrop(self.minimap_backdrop.clone());
        self.last_events = events.clone();

        let texts = self.texts;
        let hud_assets = self.hud_assets;
        let raw_input = self
            .egui_state
            .as_mut()
            .ok_or_else(|| RenderError::Surface("egui state not ready".into()))?
            .take_egui_input(window.as_ref());
        let mut interaction = hud::HudInteraction::default();
        let output = self.egui_ctx.run_pass(raw_input, |ctx| {
            interaction = hud::draw_with_minimap(ctx, &hud_model, &texts, hud_assets);
        });
        // 小地图点击只平移表现层相机，MUST NOT 生成权威命令或改变哈希。
        if let Some(click) = interaction.minimap_click {
            let extent = hud_model.minimap.extent;
            let target = [(click[0] - 0.5) * extent, (click[1] - 0.5) * extent];
            let current = self.camera.pan();
            self.camera.apply(CameraAction::Pan([
                target[0] - current[0],
                target[1] - current[1],
            ]));
        }
        // 命令卡点击：翻译为归一化输入动作，只经既有公开提交入口入队。
        if let Some(action) = interaction.command_action {
            let input_action = match action {
                hud::CommandAction::Gather => InputAction::Gather,
                hud::CommandAction::Build => InputAction::Build,
                hud::CommandAction::Train => InputAction::Train,
                hud::CommandAction::Cancel => InputAction::CancelTrain,
                hud::CommandAction::Stop => InputAction::Stop,
                hud::CommandAction::Move | hud::CommandAction::Attack => {
                    InputAction::ContextAction(self.cursor)
                }
            };
            // `gpu` 的不可变借用仍在，故只记录待处理动作；由
            // `RedrawRequested` 分支在借用结束后统一提交。
            self.pending_input_action = Some(input_action);
        }
        let egui::FullOutput {
            platform_output,
            mut textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = output;
        let paint_jobs = self.egui_ctx.tessellate(shapes, pixels_per_point);
        let screen = ScreenDescriptor {
            size_in_pixels: [gpu.width, gpu.height],
            pixels_per_point,
        };
        let mut renderer = self.egui_renderer.take();

        let result = gpu.render_frame_with_overlay(
            &vertices,
            &view_proj,
            |device, queue, color_view, format, encoder| {
                let painter = renderer.get_or_insert_with(|| {
                    egui_wgpu::Renderer::new(device, format, egui_wgpu::RendererOptions::default())
                });
                for (id, deltas) in &textures_delta.set {
                    for delta in deltas {
                        painter.update_texture(device, queue, *id, delta);
                    }
                }
                let _callbacks =
                    painter.update_buffers(device, queue, encoder, &paint_jobs, &screen);
                let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })];
                // 单独作用域确保 render pass 先 drop，编码器随后才能 finish。
                {
                    let mut pass = encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("w3-client egui hud pass"),
                            color_attachments: &color_attachments,
                            depth_stencil_attachment: None,
                            ..Default::default()
                        })
                        .forget_lifetime();
                    painter.render(&mut pass, &paint_jobs, &screen);
                }
                // delta 已全部应用：清空后再丢弃，满足 egui `TexturesDelta`
                // 的 Drop 契约（未处理 delta 会触发 debug_assert）。纯表现层。
                for id in &textures_delta.free {
                    painter.free_texture(id);
                }
                textures_delta.clear();
            },
        );
        self.egui_renderer = renderer;
        result?;

        self.egui_state
            .as_mut()
            .expect("egui state checked above")
            .handle_platform_output(window.as_ref(), platform_output);
        self.frames += 1;
        Ok(())
    }

    /// 把未被 egui 消费的 winit 事件归一化为输入 / 相机动作。
    fn route_game_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = [position.x, position.y];
                if self.dragging
                    && self
                        .apply_input(InputAction::SelectDrag(self.cursor))
                        .is_err()
                {
                    // 拖拽只改表现层状态，失败也仅记录，不改变模拟推进。
                    log::warn!(target: "w3_client", "select drag ignored: reason=input_error");
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => {
                    self.dragging = true;
                    self.report_input(InputAction::SelectStart(self.cursor));
                }
                (ElementState::Released, MouseButton::Left) => {
                    self.dragging = false;
                    self.report_input(InputAction::SelectEnd(self.cursor));
                }
                (ElementState::Released, MouseButton::Right) => {
                    self.report_input(InputAction::ContextAction(self.cursor));
                }
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(delta) => delta.y as f32 / 32.0,
                };
                self.camera.apply(CameraAction::Zoom(lines * ZOOM_STEP));
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyQ) => {
                        self.report_input(InputAction::Cast(self.cursor));
                    }
                    PhysicalKey::Code(KeyCode::KeyT) => {
                        self.report_input(InputAction::Train);
                    }
                    PhysicalKey::Code(KeyCode::KeyC) => {
                        self.report_input(InputAction::CancelTrain);
                    }
                    PhysicalKey::Code(KeyCode::KeyG) => {
                        self.report_input(InputAction::Gather);
                    }
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        self.report_input(InputAction::Build);
                    }
                    PhysicalKey::Code(KeyCode::KeyH) => {
                        self.report_input(InputAction::Stop);
                    }
                    PhysicalKey::Code(KeyCode::KeyW) => {
                        self.camera.apply(CameraAction::Pan([0.0, -PAN_STEP]));
                    }
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        self.camera.apply(CameraAction::Pan([0.0, PAN_STEP]));
                    }
                    PhysicalKey::Code(KeyCode::KeyA) => {
                        self.camera.apply(CameraAction::Pan([-PAN_STEP, 0.0]));
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        self.camera.apply(CameraAction::Pan([PAN_STEP, 0.0]));
                    }
                    PhysicalKey::Code(KeyCode::KeyZ) => {
                        self.camera.apply(CameraAction::Rotate(-YAW_STEP));
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        self.camera.apply(CameraAction::Rotate(YAW_STEP));
                    }
                    PhysicalKey::Code(key) => {
                        if let Some(group) = assign_group_number(key) {
                            self.report_input(InputAction::AssignGroup(group));
                        } else if let Some(group) = select_group_number(key) {
                            self.report_input(InputAction::SelectGroup(group));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// 输入动作统一入口：失败以英文结构化日志明确报告，MUST NOT 静默降级。
    fn report_input(&mut self, action: InputAction) {
        if let Err(error) = self.apply_input(action) {
            log::warn!(target: "w3_client", "input action rejected: action={:?} error={}", action, error);
            self.last_error = Some(error);
        }
    }
}

impl<P: StepDriver + 'static> ApplicationHandler<()> for ClientApp<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        // 窗口标题走 i18n 层，只存在于表现层，不进日志 / 权威状态 / 状态哈希。
        // 默认 1920×1080 逻辑尺寸，便于人工窗口冒烟；高 DPI 由 winit 换算。
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(self.texts.window_title)
                .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                log::error!(target: "w3_client", "window creation failed: error={}", error);
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        log::info!(target: "w3_client", "window created: width={} height={} scale_factor={}", size.width, size.height, window.scale_factor());
        let gpu = match GpuContext::new(&window, size.width, size.height) {
            Ok(gpu) => gpu,
            Err(error) => {
                log::error!(target: "w3_client", "gpu init failed: error={}", error);
                event_loop.exit();
                return;
            }
        };
        self.camera
            .apply(CameraAction::Resize(size.width, size.height));
        self.egui_state = Some(egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref() as &dyn wgpu::rwh::HasDisplayHandle,
            Some(window.scale_factor() as f32),
            None,
            None,
        ));
        self.hud_assets = match HudAssets::prepare(&self.egui_ctx) {
            Ok(assets) => assets,
            Err(error) => {
                log::warn!(target: "w3_client", "hud font unavailable: error={} element=text_labels fallback=skip_text", error);
                HudAssets::degraded()
            }
        };
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // egui 优先消费；未消费的事件才交给游戏输入 / 相机。
        let consumed = match (&self.window, &mut self.egui_state) {
            (Some(window), Some(state)) => state.on_window_event(window.as_ref(), &event).consumed,
            _ => false,
        };
        if !consumed {
            self.route_game_event(&event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                // 只更新交换链与宽高比，MUST NOT 触发 step()。
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                self.camera
                    .apply(CameraAction::Resize(size.width, size.height));
            }
            WindowEvent::RedrawRequested => {
                // 呈现只读：逻辑帧数在 about_to_wait 由整数 tick 决定。
                if let Err(error) = self.render_one() {
                    log::error!(target: "w3_client", "render failed: frame={} error={}", self.frames, error);
                }
                // 命令卡点击：GPU 借用结束后，经既有公开提交入口入队。
                if let Some(action) = self.pending_input_action.take() {
                    self.report_input(action);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.frames >= self.max_frames {
            event_loop.exit();
            return;
        }
        // 唯一节拍源：宿主层把墙钟增量换算成整数 tick，累加器决定逻辑帧数，
        // 再逐个调用无参 step()。RedrawRequested / VSync / GPU 耗时不参与。
        let ticks = self.clock.poll_ticks();
        self.driver.advance_ticks(ticks);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
