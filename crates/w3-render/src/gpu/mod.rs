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

/// 只读帧源：由 app 层实现（持有 `RtsMatch` 并逐帧 step），只向渲染层
/// 交付 owned `WorldView` 与事件拷贝。渲染层由此无法反写权威状态。
pub trait FrameProducer {
    fn next_frame(&mut self) -> (WorldView, Vec<Event>);
}

/// 表现层相机：按地形范围生成固定视角的 MVP（WebGPU 深度 0..1）。
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub aspect: f32,
}

impl Camera {
    pub fn matrix(self, terrain: &TerrainMesh) -> glam::Mat4 {
        let extent = (terrain.columns.max(terrain.rows) as f32 * terrain.cell_size).max(1.0);
        let cx = terrain.origin[0] + terrain.columns as f32 * terrain.cell_size * 0.5;
        let cz = terrain.origin[1] + terrain.rows as f32 * terrain.cell_size * 0.5;
        let eye = glam::Vec3::new(cx + extent * 0.7, extent * 1.1, cz + extent * 0.9);
        let target = glam::Vec3::new(cx, 0.0, cz);
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(0.9, self.aspect.max(0.001), 1.0, extent * 8.0);
        proj * view
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
    pub fn render_frame(
        &self,
        vertices: &[SceneVertex],
        view_proj: &glam::Mat4,
    ) -> Result<(), RenderError> {
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
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError::GpuUnavailable("gpu context not ready".into()))?;
        let aspect = gpu.width as f32 / gpu.height as f32;
        let view_proj = Camera { aspect }.matrix(&self.terrain);
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
        let matrix = Camera { aspect: 1.5 }.matrix(&terrain);
        assert!(matrix.to_cols_array().iter().all(|v| v.is_finite()));
    }
}
