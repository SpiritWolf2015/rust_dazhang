//! 人工呈现验收入口（任务 3.3）。
//!
//! 用法：
//! `cargo run -p w3-render --example render_frame -- <assets-dir> <output.png>`
//!
//! - `<assets-dir>`：本机导出目录，读 `terrain-heightfield.json` 与 `models/*.json`；
//!   省略时用合成高度场，单位因模型缺失降级为不显示（spec Scenario）。
//! - `<output.png>`：截图路径，默认 `w3-render-frame.png`。
//!
//! 需要本机有 GPU 与显示设备；导出资产与截图都留在玩家机器，绝不入库
//! （见 ADR 0001）。渲染层只读模拟视图，截图只用于呈现残差记录，不进入
//! 状态哈希。

use std::path::PathBuf;
use std::sync::Arc;

use w3_core::{
    Command, CommandKind, Event, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld, WorldView,
};
use w3_render::gpu::{Camera, GpuContext, readback};
use w3_render::{FrameInput, FrameProducer, ModelLibrary, RenderError, Scene, TerrainMesh};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// 默认截图落在已 gitignored 的 `target/`，避免人工验收产物被 `git add -A` 误入库。
const DEFAULT_OUTPUT: &str = "target/w3-render-frame.png";
const MAX_FRAMES: u32 = 3;

/// app 层帧源：持有唯一权威状态 `RtsMatch`，每帧 step 后只交出只读拷贝。
struct SimProducer {
    sim: RtsMatch,
}

impl SimProducer {
    fn new() -> Self {
        let mut sim = RtsMatch::create_with_world(
            MatchConfig {
                seed: 7,
                logic_fps: 30,
                content_hash: 0,
            },
            StaticWorld::open_world(16, 16, SimFixed::from_int(8).unwrap()),
        );
        for i in 0..6u32 {
            sim.submit_command(Command::new(
                Frame::new(0),
                (i % 2) as u8,
                i + 1,
                CommandKind::Spawn {
                    x: SimFixed::from_int(20 + i64::from(i) * 8).unwrap(),
                    y: SimFixed::from_int(30).unwrap(),
                    hp: 100,
                    speed: SimFixed::from_int(3).unwrap(),
                    attack_damage: 10,
                    attack_range: SimFixed::from_int(10).unwrap(),
                },
            ))
            .unwrap();
        }
        Self { sim }
    }
}

impl FrameProducer for SimProducer {
    fn next_frame(&mut self) -> (WorldView, Vec<Event>) {
        self.sim.step();
        (self.sim.read_view(), self.sim.drain_events())
    }
}

/// 无本机导出数据时的合成高度场（中间低、四周高的碗状地形）。
fn fallback_terrain() -> TerrainMesh {
    let size = 17usize;
    let mut heights = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - (size / 2) as f64;
            let dy = y as f64 - (size / 2) as f64;
            heights.push((dx * dx + dy * dy).sqrt() * 1.5);
        }
    }
    let hf = w3_content::TerrainHeightfield {
        tilepoint_width: size,
        tilepoint_height: size,
        map_width: size - 1,
        map_height: size - 1,
        center_offset: None,
        main_tileset: None,
        main_tileset_name: None,
        ground_tilesets: Vec::new(),
        cliff_tilesets: Vec::new(),
        tile_size: Some(128.0),
        heights,
        ground_textures: Vec::new(),
        ground_variations: Vec::new(),
        cliff_textures: Vec::new(),
        cliff_variations: Vec::new(),
        layer_heights: Vec::new(),
        water_heights: Vec::new(),
        flags_packed: Vec::new(),
    };
    TerrainMesh::from_heightfield(&hf).expect("synthetic heightfield is valid")
}

/// 呈现验收应用：渲染到窗口并用 GPU readback 写一张 PNG 截图。
struct ScreenshotApp {
    producer: SimProducer,
    scene: Scene,
    terrain: TerrainMesh,
    library: ModelLibrary,
    gpu: Option<GpuContext>,
    window: Option<Arc<Window>>,
    output: PathBuf,
    frames: u32,
}

impl ScreenshotApp {
    fn render_once(&mut self) -> Result<(), RenderError> {
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError::GpuUnavailable("gpu context not ready".into()))?;
        let (view, events) = self.producer.next_frame();
        self.scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &self.library,
        );
        self.scene.interpolate_all(1.0);
        let vertices = self.scene.build_vertices(&self.terrain);
        let aspect = gpu.width as f32 / gpu.height as f32;
        let view_proj = Camera { aspect }.matrix(&self.terrain);

        // 呈现到交换链。
        gpu.render_frame(&vertices, &view_proj)?;
        // 呈现验收：离屏 readback → PNG（产物留本机，不入库）。
        let rgba = gpu.capture_rgba(&vertices, &view_proj)?;
        readback::write_png(&self.output, &rgba, gpu.width, gpu.height)?;
        log::info!(target: "w3_render", "frame captured: frame={} width={} height={} vertices={}", self.frames, gpu.width, gpu.height, vertices.len());
        self.frames += 1;
        Ok(())
    }
}

impl ApplicationHandler<()> for ScreenshotApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let window = match event_loop
            .create_window(Window::default_attributes().with_title("w3-render acceptance"))
        {
            Ok(window) => Arc::new(window),
            Err(error) => {
                log::error!(target: "w3_render", "window creation failed: error={}", error);
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        match GpuContext::new(&window, size.width, size.height) {
            Ok(gpu) => {
                self.gpu = Some(gpu);
                self.window = Some(window);
            }
            Err(error) => {
                log::error!(target: "w3_render", "gpu init failed: error={}", error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.render_once() {
                    log::error!(target: "w3_render", "render failed: frame={} error={}", self.frames, error);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.frames >= MAX_FRAMES {
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let assets = args.next().map(PathBuf::from);
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT));

    let terrain = match assets.as_ref() {
        Some(dir) => {
            let text = std::fs::read_to_string(dir.join("terrain-heightfield.json"))?;
            let hf: w3_content::TerrainHeightfield = serde_json::from_str(&text)?;
            TerrainMesh::from_heightfield(&hf)?
        }
        None => fallback_terrain(),
    };

    let mut library = ModelLibrary::default();
    if let Some(dir) = assets.as_ref() {
        let models = dir.join("models");
        if models.is_dir() {
            let count = library.load_dir(&models)?;
            log::info!(target: "w3_render", "models loaded: count={}", count);
        }
    }
    // 资产缺失时单位降级为不显示，模拟照常推进（spec Scenario）。

    let mut app = ScreenshotApp {
        producer: SimProducer::new(),
        scene: Scene::default(),
        terrain,
        library,
        gpu: None,
        window: None,
        output,
        frames: 0,
    };
    let event_loop = EventLoop::builder().build()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
