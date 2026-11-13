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
///
/// `Camera::matrix` 里 `distance = extent / zoom`（`extent≈258`）。默认开局
/// 取 `zoom=7`（视野约 100×56 世界单位，主城 + 农民簇 + 周边陆地完整可见，
/// 见 `examples/melee.rs`）。上限从 8 放宽到 20 **仅是给玩家滚轮进一步拉近
/// 看单位细节、以及离线取景留出余量**，不改变默认开局取景；下限 0.25 保持
/// 可拉远看全图。纯表现层 clamp 上限，不进权威状态 / 哈希 / 快照。
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 20.0;

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

    /// 显式设置初始缩放与平移（纯表现层，用于可玩入口一开局呈现局部战场
    /// 视角）。直接赋值而非经增量 `Zoom(delta)`，不隐含依赖 `default().zoom`。
    /// `zoom` 经 MIN/MAX clamp；`pan` 取有限值，非有限则忽略该轴。
    pub fn set_initial_view(&mut self, zoom: f32, pan: [f32; 2]) {
        if zoom.is_finite() {
            self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        }
        if pan[0].is_finite() {
            self.pan[0] = pan[0];
        }
        if pan[1].is_finite() {
            self.pan[1] = pan[1];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_zoom_is_within_bounds_not_clamped() {
        // 默认开局 `zoom=7` 生效（不改哈希/快照，纯表现层）。
        let mut state = CameraState::default();
        state.set_initial_view(7.0, [-72.0, 34.0]);
        assert_eq!(state.zoom(), 7.0, "开局 zoom 应生效");
        assert_eq!(state.pan(), [-72.0, 34.0]);

        // 判别力：`zoom=12` 在旧的 MAX_ZOOM=8 下会被 clamp 到 8，放宽到 20
        // 后必须原样生效。此断言守护「上限确已放宽、滚轮可进一步拉近」，
        // 若有人把 MAX_ZOOM 退回 8 此处失败（而非与常量自比较恒真）。
        state.set_initial_view(12.0, [0.0, 0.0]);
        assert_eq!(
            state.zoom(),
            12.0,
            "12.0 应落在放宽后的上限内；若被 clamp 说明 MAX_ZOOM 被回退"
        );

        // 越界仍被 clamp（上限提高不破坏既有保护）。
        state.set_initial_view(1000.0, [0.0, 0.0]);
        assert_eq!(state.zoom(), MAX_ZOOM);
        state.set_initial_view(-5.0, [0.0, 0.0]);
        assert_eq!(state.zoom(), MIN_ZOOM);
    }
}
