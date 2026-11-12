//! wgpu / winit GPU 基建：窗口、交换链、渲染管线与只读帧循环。
//!
//! 本模块只消费 [`crate::scene::Scene`] 生成的只读顶点与 `Camera` 矩阵，
//! 不接触 `RtsMatch`。`RendererApp` 通过 [`FrameProducer`] 从 app 层取得
//! owned `WorldView` + 事件拷贝，表现层与确定性内核之间只有这条只读边界。
//!
//! `SharedWindow` 让 wgpu surface 与 app 共享同一个 `Arc<Window>`，避免
//! unsafe 表面创建，也避免自引用结构体。

mod block_on;
pub mod readback;

use std::sync::Arc;

use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use w3_core::{Event, WorldView};

use crate::error::RenderError;
use crate::model::ModelLibrary;
use crate::scene::{FrameInput, Scene, SceneVertex};
use crate::terrain::TerrainMesh;

use block_on::block_on;

const SHADER: &str = include_str!("../shaders/scene.wgsl");
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// 一个 `mat4x4<f32>` uniform 的字节数。
const UNIFORM_SIZE: u64 = 64;

/// 让 surface 与 app 共享窗口所有权：surface 持一个 Arc，app 持另一个 Arc
/// 用于 `request_redraw`。实现 `rwh` 句柄 trait 委托给内层 `Window`。
struct SharedWindow(Arc<Window>);

impl HasWindowHandle for SharedWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for SharedWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

/// 只读帧源：本地与联机对局共用的唯一呈现管线接缝。
///
/// 实现者持有唯一 `RtsMatch`，但 [`FrameProducer::next_frame`] 只交出
/// **当前**只读 `WorldView` 与事件拷贝，MUST NOT 调用 `RtsMatch::step()`；
/// 逻辑帧推进由客户端固定 30 Hz 整数帧累加器独立驱动（ADR 0002 §2、
/// design D2）。[`FrameProducer::interpolation_alpha`] 把累加器余数作为
/// 表现层插值系数送入渲染帧：默认 1.0 表示不插值，便于现有实现者零改动
/// 迁移；该系数只存在于表现层，MUST NOT 写回权威状态、状态哈希或快照。
pub trait FrameProducer {
    /// 只交出当前只读视图与事件拷贝，MUST NOT 推进模拟。
    fn next_frame(&mut self) -> (WorldView, Vec<Event>);

    /// 表现层插值系数，来自固定累加器的整数余数；默认不插值。
    fn interpolation_alpha(&self) -> f32 {
        1.0
    }
}

/// 表现层相机：按地形范围生成 MVP（WebGPU 深度 0..1），支持平移 / 缩放 / 旋转。
///
/// 全部字段都是**表现层专属**的 `f32` 状态，MUST NOT 反写 `RtsMatch`、
/// 进入 FNV-1a 状态哈希或 `SNAPSHOT_VERSION`（当前 3）快照（design D5）。
/// 默认参数（pan 为零、zoom 为 1、yaw 为 0）生成与 P3 既有验收一致的固定视角，
/// 因此扩展后已有呈现基线不变。
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// 视口宽高比。
    pub aspect: f32,
    /// 观察目标相对地形中心的 XZ 平移（世界单位）。
    pub pan: [f32; 2],
    /// 缩放倍率（> 0，1.0 为默认距离）。
    pub zoom: f32,
    /// 绕 Y 轴旋转（弧度）。
    pub yaw: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            pan: [0.0; 2],
            zoom: 1.0,
            yaw: 0.0,
        }
    }
}

impl Camera {
    /// 按视口宽高比构造默认相机，收窄 `Camera { aspect }` 字面量的 BREAKING 破坏面。
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            ..Self::default()
        }
    }

    /// 设置平移（表现层输入映射专用，链式构造）。
    pub fn with_pan(mut self, pan: [f32; 2]) -> Self {
        self.pan = pan;
        self
    }

    /// 设置缩放，非法值钳到正值，避免矩阵退化。
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        self
    }

    /// 设置绕 Y 轴旋转（弧度）。
    pub fn with_yaw(mut self, yaw: f32) -> Self {
        self.yaw = if yaw.is_finite() { yaw } else { 0.0 };
        self
    }

    /// 生成 MVP；`pan` / `zoom` / `yaw` 只影响观察矩阵，不触碰任何权威状态。
    pub fn matrix(self, terrain: &TerrainMesh) -> glam::Mat4 {
        let extent = (terrain.columns.max(terrain.rows) as f32 * terrain.cell_size).max(1.0);
        let cx = terrain.origin[0] + terrain.columns as f32 * terrain.cell_size * 0.5 + self.pan[0];
        let cz = terrain.origin[1] + terrain.rows as f32 * terrain.cell_size * 0.5 + self.pan[1];
        let distance = extent / self.zoom.max(0.05);
        let (sin, cos) = self.yaw.sin_cos();
        let eye = glam::Vec3::new(
            cx + distance * (0.7 * cos + 0.9 * sin),
            distance * 1.1,
            cz + distance * (-0.7 * sin + 0.9 * cos),
        );
        let target = glam::Vec3::new(cx, 0.0, cz);
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(0.9, self.aspect.max(0.001), 1.0, extent * 8.0);
        proj * view
    }
}

/// 屏幕 ↔ 世界（XZ 平面）的拾取投影，与 [`Camera::matrix`] 由同一组参数构造。
///
/// 输入拾取（框选 / 命中 / HUD 位置）复用它，消除「渲染用透视斜视投影、
/// 拾取用线性正交映射」两套口径造成的错位。`world_to_screen` 以单位占位
/// 标记的中心平面为基准（与用户看到的四面体对齐），`screen_to_world` 以
/// y=0 地面为基准（命令目标落在地面）；二者各司其职，均为表现层 `f32` /
/// `f64` 运算，MUST NOT 进入权威状态、状态哈希或快照。
#[derive(Clone, Copy, Debug)]
pub struct ScreenProjection {
    view_proj: glam::Mat4,
    inverse: glam::Mat4,
    width: f32,
    height: f32,
}

impl ScreenProjection {
    /// 由相机与地形构造；矩阵与 [`Camera::matrix`] 输出逐位一致。
    pub fn new(camera: &Camera, terrain: &TerrainMesh, width: f32, height: f32) -> Self {
        let view_proj = (*camera).matrix(terrain);
        Self {
            view_proj,
            inverse: view_proj.inverse(),
            width: width.max(1.0),
            height: height.max(1.0),
        }
    }

    fn project_point(&self, x: f64, y: f64, z: f64) -> [f64; 2] {
        let clip = self.view_proj * glam::Vec4::new(x as f32, y as f32, z as f32, 1.0);
        if clip.w <= 0.0 {
            // 点在相机背后：返回屏幕外的确定值，使命中测试自然失败。
            return [-1.0e6, -1.0e6];
        }
        let ndc = clip.truncate() / clip.w;
        [
            (f64::from(ndc.x) + 1.0) * 0.5 * f64::from(self.width),
            (1.0 - f64::from(ndc.y)) * 0.5 * f64::from(self.height),
        ]
    }

    /// 单位世界坐标 `(x, z)` → 屏幕像素坐标，投影到单位占位标记中心平面。
    pub fn world_to_screen(&self, world: [f64; 2]) -> [f64; 2] {
        self.project_point(
            world[0],
            f64::from(crate::scene::UNIT_MARKER_SIZE * 0.8),
            world[1],
        )
    }

    /// 单位占位标记在屏幕上的保守包围盒：覆盖任意朝向的四面体，
    /// 供框选 / 命中 / HUD 选中框使用，避免固定小半径漏点。
    pub fn unit_screen_bounds(&self, world: [f64; 2]) -> ([f64; 2], [f64; 2]) {
        let size = f64::from(crate::scene::UNIT_MARKER_SIZE);
        let mut min = [f64::MAX, f64::MAX];
        let mut max = [f64::MIN, f64::MIN];
        for (dx, y, dz) in [
            (0.0, size * 1.6, 0.0),
            (-size, 0.0, -size),
            (size, 0.0, -size),
            (-size, 0.0, size),
            (size, 0.0, size),
        ] {
            let point = self.project_point(world[0] + dx, y, world[1] + dz);
            for axis in 0..2 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        (min, max)
    }

    /// 屏幕像素坐标 → 与 y=0 地面平面的交点世界坐标 `(x, z)`；
    /// 射线与地面平行时返回 `None`，由调用方显式报错。
    pub fn screen_to_world(&self, screen: [f64; 2]) -> Option<[f64; 2]> {
        let nx = (screen[0] / f64::from(self.width)) * 2.0 - 1.0;
        let ny = 1.0 - (screen[1] / f64::from(self.height)) * 2.0;
        let unproject = |z: f32| {
            let point = self.inverse * glam::Vec4::new(nx as f32, ny as f32, z, 1.0);
            if point.w.abs() < 1.0e-9 {
                None
            } else {
                Some(point.truncate() / point.w)
            }
        };
        let near = unproject(-1.0)?;
        let far = unproject(1.0)?;
        let direction = far - near;
        if direction.y.abs() < 1.0e-9 {
            return None;
        }
        let t = -near.y / direction.y;
        let hit = near + direction * t;
        Some([f64::from(hit.x), f64::from(hit.z)])
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn terrain() -> TerrainMesh {
        let hf: w3_content::TerrainHeightfield = serde_json::from_str(
            r#"{"tilepointWidth":2,"tilepointHeight":2,"mapWidth":1,"mapHeight":1,
                "heights":[0,0,0,0]}"#,
        )
        .unwrap();
        TerrainMesh::from_heightfield(&hf).unwrap()
    }

    #[test]
    fn screen_to_world_inverts_ground_projection() {
        let camera = Camera::new(16.0 / 9.0);
        let terrain = terrain();
        let view_proj = camera.matrix(&terrain);
        let projection = ScreenProjection::new(&camera, &terrain, 1280.0, 720.0);
        for world in [[0.0, 0.0], [32.0, -16.0], [-32.0, 24.0], [8.0, 8.0]] {
            // 地面点（y=0）经 `Camera::matrix` 投影到屏幕。
            let clip = view_proj * glam::Vec4::new(world[0] as f32, 0.0, world[1] as f32, 1.0);
            let ndc = clip.truncate() / clip.w;
            let screen = [
                (f64::from(ndc.x) + 1.0) * 0.5 * 1280.0,
                (1.0 - f64::from(ndc.y)) * 0.5 * 720.0,
            ];
            let back = projection.screen_to_world(screen).expect("ground hit");
            // f32 矩阵求逆带来的误差远小于 1 像素，对拾取无影响。
            assert!(
                (back[0] - world[0]).abs() < 0.1 && (back[1] - world[1]).abs() < 0.1,
                "world={world:?} screen={screen:?} back={back:?}"
            );
        }
    }

    #[test]
    fn projection_matches_camera_matrix() {
        let terrain = terrain();
        let camera = Camera::new(1.5)
            .with_pan([8.0, -4.0])
            .with_zoom(1.5)
            .with_yaw(0.3);
        let view_proj = camera.matrix(&terrain);
        let projection = ScreenProjection::new(&camera, &terrain, 800.0, 600.0);
        let world = [16.0, 24.0];
        let pick_height = crate::scene::UNIT_MARKER_SIZE * 0.8;
        let clip = view_proj * glam::Vec4::new(world[0] as f32, pick_height, world[1] as f32, 1.0);
        let ndc = clip.truncate() / clip.w;
        let expected = [
            (f64::from(ndc.x) + 1.0) * 0.5 * 800.0,
            (1.0 - f64::from(ndc.y)) * 0.5 * 600.0,
        ];
        let actual = projection.world_to_screen(world);
        assert!((actual[0] - expected[0]).abs() < 1.0e-6);
        assert!((actual[1] - expected[1]).abs() < 1.0e-6);
    }
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("w3-render depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// 一条场景管线的全部资源；地形与单位共用一个 pipeline（CPU 已展开三角形）。
struct PipelineResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

fn build_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> PipelineResources {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("w3-render scene shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("w3-render globals"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("w3-render pipeline layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        ..Default::default()
    });
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("w3-render mvp"),
        size: UNIFORM_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("w3-render globals"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("w3-render scene pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SceneVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 24,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 40,
                        shader_location: 3,
                    },
                ],
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Ccw,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    PipelineResources {
        pipeline,
        bind_group,
        uniform_buffer,
    }
}

/// 记录一次场景 pass；`color_view` / `depth_view` 决定渲染到交换链还是离屏纹理。
fn record_scene(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    resources: &PipelineResources,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
) {
    let color_attachments = [Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.02,
                g: 0.03,
                b: 0.06,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        },
    })];
    let descriptor = wgpu::RenderPassDescriptor {
        label: Some("w3-render scene pass"),
        color_attachments: &color_attachments,
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    };
    let mut pass = encoder.begin_render_pass(&descriptor);
    pass.set_pipeline(&resources.pipeline);
    pass.set_bind_group(0, &resources.bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.draw(0..vertex_count, 0..1);
}

/// GPU 上下文：持有窗口表面、设备、队列、交换链配置与场景管线。
pub struct GpuContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    resources: PipelineResources,
    depth_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl GpuContext {
    /// 在本机窗口上初始化 GPU；无适配器 / 设备失败时返回
    /// `RenderError::GpuUnavailable`，由调用方降级处理，绝不 panic。
    pub fn new(window: &Arc<Window>, width: u32, height: u32) -> Result<Self, RenderError> {
        let width = width.max(1);
        let height = height.max(1);
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(SharedWindow(Arc::clone(window)))
            .map_err(|e| RenderError::Surface(format!("create_surface: {e:?}")))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| RenderError::GpuUnavailable(format!("request_adapter: {e:?}")))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("w3-render device"),
            ..Default::default()
        }))
        .map_err(|e| RenderError::GpuUnavailable(format!("request_device: {e:?}")))?;
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| RenderError::Surface("surface incompatible with adapter".into()))?;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface.configure(&device, &config);
        let format = config.format;
        let resources = build_pipeline(&device, format);
        let depth_view = create_depth_view(&device, width, height);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            resources,
            depth_view,
            width,
            height,
            format,
        })
    }

    /// 交换链尺寸变化：重配 surface 并重建深度纹理。
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    /// 渲染一帧到交换链并 present。顶点与 MVP 都是只读表现层数据。
    ///
    /// 这是对 [`GpuContext::render_frame_with_overlay`] 的无 overlay 便捷封装，
    /// 保持既有 `RendererApp` / 呈现验收调用点零改动。
    pub fn render_frame(
        &self,
        vertices: &[SceneVertex],
        view_proj: &glam::Mat4,
    ) -> Result<(), RenderError> {
        self.render_frame_with_overlay(vertices, view_proj, |_, _, _, _, _| {})
    }

    /// 渲染 3D 场景后，把同一交换链纹理的只读 GPU 句柄交给 `overlay` 叠加 2D 表现层。
    ///
    /// `overlay` 只接收 `wgpu` 原生类型（设备 / 队列 / 颜色视图 / 格式 / 命令编码器），
    /// 不引用任何 UI 库：客户端可在同一张纹理上以 `LoadOp::Load` 追加 HUD pass，
    /// 从而复用既有 surface / device / queue / 深度纹理与场景管线（design D4 CARP、
    /// DIP）。该回调在 `record_scene` 之后、`submit` / `present` 之前执行，
    /// 纯表现层，MUST NOT 触碰 `RtsMatch`、状态哈希或快照；回调 MUST NOT
    /// 访问或修改任何非呈现资源或权威模拟状态，只用于在同一交换链纹理上
    /// 追加呈现 pass。
    pub fn render_frame_with_overlay<F>(
        &self,
        vertices: &[SceneVertex],
        view_proj: &glam::Mat4,
        overlay: F,
    ) -> Result<(), RenderError>
    where
        F: FnOnce(
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
            wgpu::TextureFormat,
            &mut wgpu::CommandEncoder,
        ),
    {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => return Err(RenderError::Surface(format!("acquire frame: {other:?}"))),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("w3-render frame vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.queue.write_buffer(
            &self.resources.uniform_buffer,
            0,
            bytemuck::cast_slice(&view_proj.to_cols_array()),
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        record_scene(
            &mut encoder,
            &view,
            &self.depth_view,
            &self.resources,
            &vertex_buffer,
            vertices.len() as u32,
        );
        overlay(&self.device, &self.queue, &view, self.format, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }

    /// GPU readback：离屏渲染同一场景，把颜色纹理拷回 CPU，返回紧凑 RGBA。
    /// 供呈现验收截图使用；行对齐填充与 BGRA 交换都在此处理。
    pub fn capture_rgba(
        &self,
        vertices: &[SceneVertex],
        view_proj: &glam::Mat4,
    ) -> Result<Vec<u8>, RenderError> {
        let width = self.width;
        let height = self.height;
        // copy_texture_to_buffer 要求 bytes_per_row 为 256 的倍数。
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("w3-render readback color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = create_depth_view(&self.device, width, height);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("w3-render readback vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.queue.write_buffer(
            &self.resources.uniform_buffer,
            0,
            bytemuck::cast_slice(&view_proj.to_cols_array()),
        );
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("w3-render readback buffer"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        record_scene(
            &mut encoder,
            &color_view,
            &depth_view,
            &self.resources,
            &vertex_buffer,
            vertices.len() as u32,
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|e| RenderError::Readback(format!("device poll: {e:?}")))?;
        receiver
            .recv()
            .map_err(|e| RenderError::Readback(format!("map callback: {e}")))?
            .map_err(|e| RenderError::Readback(format!("map async: {e:?}")))?;
        let padded = {
            let mapped = slice
                .get_mapped_range()
                .map_err(|e| RenderError::Readback(format!("get_mapped_range: {e:?}")))?;
            mapped.to_vec()
        };
        readback_buffer.unmap();

        let mut rgba = readback::remove_row_padding(&padded, width, height, bytes_per_row)?;
        if matches!(
            self.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            readback::swap_bgra_to_rgba(&mut rgba);
        }
        Ok(rgba)
    }
}

/// 离屏（headless）渲染一帧并把颜色读回 CPU，返回紧凑 RGBA。
///
/// 不依赖任何窗口 / surface：自建 `Instance` → `adapter` → `device`、
/// 复用同一 `build_pipeline` / `record_scene` / readback 逻辑，用 `Rgba8Unorm`
/// 离屏纹理。专供**提交前 GPU 链路自测**——把「真实模型贴图是否真的上屏、
/// 是否黑屏 / 纯色块」变成可在 CLI 断言的事实，而不必开窗口人工观察。
/// 无 GPU 适配器时返回 `GpuUnavailable`，由测试跳过（不冒充通过）。
pub fn render_headless_rgba(
    vertices: &[SceneVertex],
    view_proj: &glam::Mat4,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
    let width = width.max(1);
    let height = height.max(1);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // 空顶点流：无需建 GPU 资源，返回与 `record_scene` Clear 一致的深蓝背景
    // （0.02, 0.03, 0.06, 1.0 → RGBA8），也让 wgpu 的空 buffer slice 无从触发。
    if vertices.is_empty() {
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..(width as usize * height as usize) {
            rgba.extend_from_slice(&[5, 8, 15, 255]);
        }
        return Ok(rgba);
    }
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .map_err(|e| RenderError::GpuUnavailable(format!("headless request_adapter: {e:?}")))?;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("w3-render headless device"),
        ..Default::default()
    }))
    .map_err(|e| RenderError::GpuUnavailable(format!("headless request_device: {e:?}")))?;

    let resources = build_pipeline(&device, format);
    let depth_view = create_depth_view(&device, width, height);
    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("w3-render headless color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("w3-render headless vertices"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    queue.write_buffer(
        &resources.uniform_buffer,
        0,
        bytemuck::cast_slice(&view_proj.to_cols_array()),
    );

    let bytes_per_row = (width * 4).div_ceil(256) * 256;
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("w3-render headless readback"),
        size: u64::from(bytes_per_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    record_scene(
        &mut encoder,
        &color_view,
        &depth_view,
        &resources,
        &vertex_buffer,
        vertices.len() as u32,
    );
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &color_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .map_err(|e| RenderError::Readback(format!("headless device poll: {e:?}")))?;
    receiver
        .recv()
        .map_err(|e| RenderError::Readback(format!("headless map callback: {e}")))?
        .map_err(|e| RenderError::Readback(format!("headless map async: {e:?}")))?;
    let padded = {
        let mapped = slice
            .get_mapped_range()
            .map_err(|e| RenderError::Readback(format!("headless get_mapped_range: {e:?}")))?;
        mapped.to_vec()
    };
    readback_buffer.unmap();
    readback::remove_row_padding(&padded, width, height, bytes_per_row)
}

/// winit 应用：把 `FrameProducer` 的只读帧喂给 `Scene`，再交给 `GpuContext`。
pub struct RendererApp<P: FrameProducer> {
    producer: P,
    scene: Scene,
    terrain: TerrainMesh,
    library: ModelLibrary,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    max_frames: u32,
    frames: u32,
}

impl<P: FrameProducer> RendererApp<P> {
    /// `max_frames` 限制渲染帧数，便于无人值守的验收运行。
    pub fn new(producer: P, terrain: TerrainMesh, library: ModelLibrary, max_frames: u32) -> Self {
        Self {
            producer,
            scene: Scene::default(),
            terrain,
            library,
            window: None,
            gpu: None,
            max_frames,
            frames: 0,
        }
    }

    /// 运行事件循环直到 `max_frames` 帧或窗口关闭。
    pub fn run(mut self) -> Result<(), RenderError> {
        let event_loop = EventLoop::builder()
            .build()
            .map_err(|e| RenderError::Surface(format!("event loop: {e:?}")))?;
        event_loop
            .run_app(&mut self)
            .map_err(|e| RenderError::Surface(format!("run_app: {e:?}")))
    }

    fn render_one(&mut self) -> Result<(), RenderError> {
        // alpha 只来自接缝提供的累加器余数，仅存在于表现层。
        let alpha = self.producer.interpolation_alpha();
        let (view, events) = self.producer.next_frame();
        self.scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &self.library,
        );
        self.scene.interpolate_all(alpha);
        let vertices = self.scene.build_vertices(&self.terrain);
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError::GpuUnavailable("gpu context not ready".into()))?;
        let aspect = gpu.width as f32 / gpu.height as f32;
        let view_proj = Camera::new(aspect).matrix(&self.terrain);
        gpu.render_frame(&vertices, &view_proj)?;
        self.frames += 1;
        Ok(())
    }
}

impl<P: FrameProducer> ApplicationHandler<()> for RendererApp<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attributes = Window::default_attributes().with_title("w3-render");
        let window = match event_loop.create_window(attributes) {
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
                if let Err(error) = self.render_one() {
                    log::error!(target: "w3_render", "frame failed: frame={} error={}", self.frames, error);
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
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_matrix_is_finite() {
        let hf: w3_content::TerrainHeightfield = serde_json::from_str(
            r#"{"tilepointWidth":2,"tilepointHeight":2,"mapWidth":1,"mapHeight":1,
                "heights":[0,0,0,0]}"#,
        )
        .unwrap();
        let terrain = TerrainMesh::from_heightfield(&hf).unwrap();
        let matrix = Camera::new(1.5).matrix(&terrain);
        assert!(matrix.to_cols_array().iter().all(|v| v.is_finite()));
        // 相机位姿是表现层专属：平移 / 缩放 / 旋转只改变 MVP，仍保持有限。
        let moved = Camera::new(1.5)
            .with_pan([16.0, -8.0])
            .with_zoom(1.75)
            .with_yaw(0.6)
            .matrix(&terrain);
        assert!(moved.to_cols_array().iter().all(|v| v.is_finite()));
        assert_ne!(moved, matrix);
    }
}
