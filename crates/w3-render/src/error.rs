//! 渲染层错误类型。
//!
//! 错误文本一律英文且结构化（见 CODEBUDDY.md 第 3 节）；资产缺失属于
//! 可降级错误，调用方据此让单位不显示，但绝不反写模拟状态。

use thiserror::Error;

/// 渲染层对外错误。
#[derive(Debug, Error)]
pub enum RenderError {
    /// 本机没有可用 GPU 适配器，或设备 / 交换链创建失败。
    #[error("gpu device unavailable: {0}")]
    GpuUnavailable(String),
    /// 本机导出资产缺失或路径不可读。
    #[error("asset load failed: {0}")]
    Asset(String),
    /// 高度场维度或平行数组非法。
    #[error("invalid heightfield: {0}")]
    Terrain(String),
    /// 模型 / 骨骼 / 动画序列 JSON 非法。
    #[error("invalid model: {0}")]
    Model(String),
    /// 窗口表面获取或配置失败。
    #[error("surface error: {0}")]
    Surface(String),
    /// GPU readback / 截图编码失败。
    #[error("readback failed: {0}")]
    Readback(String),
    /// 本机文件读写失败。
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 解析失败。
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
