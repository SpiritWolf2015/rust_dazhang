//! 相机输入映射（表现层，tasks 7.1–7.4、design D5）。
//!
//! 相机几何（MVP 矩阵）留在 `w3-render::Camera`；本模块只把 winit 键鼠 /
//! 滚轮事件归一化为平移 / 缩放 / 旋转的**表现层状态**，再构造 `Camera`。
//! 这些 `f32` 位姿 MUST NOT 反写 `RtsMatch`、进入状态哈希或快照；
//! `WindowEvent::Resized` 只更新宽高比与交换链，MUST NOT 触发 `step()`。

use w3_render::Camera;

/// 平移步长（世界单位 / 按键事件）。
pub const PAN_STEP: f32 = 32.0;
/// 缩放步长（每次滚轮事件）。
pub const ZOOM_STEP: f32 = 0.1;
/// 旋转步长（弧度 / 按键事件）。
pub const YAW_STEP: f32 = 0.05;
/// 缩放上下限，防止矩阵退化。
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 4.0;

/// 归一化的相机输入动作；宿主把 winit 事件映射为此枚举。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraAction {
    /// 按 XZ 世界单位平移。
    Pan([f32; 2]),
    /// 缩放增量（正为拉近）。
    Zoom(f32),
    /// 绕 Y 轴旋转增量（弧度）。
    Rotate(f32),
    /// 视口尺寸变化：只更新宽高比，不推进逻辑帧。
    Resize(u32, u32),
}

/// 表现层相机状态；纯 `f32`，不进哈希 / 快照。
#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    aspect: f32,
    pan: [f32; 2],
    zoom: f32,
    yaw: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            pan: [0.0; 2],
            zoom: 1.0,
            yaw: 0.0,
        }
    }
}

impl CameraState {
    /// 以初始宽高比创建默认相机状态。
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            ..Self::default()
        }
    }

    /// 应用一个归一化相机输入动作；`Resize` 只改宽高比。
    pub fn apply(&mut self, action: CameraAction) {
        match action {
            CameraAction::Pan(delta) => {
                if delta.iter().all(|v| v.is_finite()) {
                    self.pan[0] += delta[0];
                    self.pan[1] += delta[1];
                }
            }
            CameraAction::Zoom(delta) => {
                if delta.is_finite() {
                    self.zoom = (self.zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);
                }
            }
            CameraAction::Rotate(delta) => {
                if delta.is_finite() {
                    self.yaw += delta;
                }
            }
            CameraAction::Resize(width, height) => {
                if height > 0 {
                    self.aspect = width as f32 / height as f32;
                }
            }
        }
    }

    /// 供输入拾取使用的表现层平移 / 缩放（映射到 `ScreenMapper`）。
    pub fn pan(&self) -> [f32; 2] {
        self.pan
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// 构造 `w3-render::Camera` 交给既有呈现管线生成 MVP。
    pub fn camera(&self) -> Camera {
        Camera::new(self.aspect)
            .with_pan(self.pan)
            .with_zoom(self.zoom)
            .with_yaw(self.yaw)
    }
}
