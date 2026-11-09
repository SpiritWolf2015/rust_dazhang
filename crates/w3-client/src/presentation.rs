//! 只读呈现管线：把固定累加器派生的 `alpha` 接到既有 `Scene::interpolate_all`。
//!
//! 本模块只消费 `WorldView` / 事实事件拷贝与一个表现层 `alpha`，从不持有
//! `RtsMatch`，因此结构上无法写回位置 / 朝向 / 生命，也无法把 `alpha`、
//! 相机、选中或 HUD 状态塞进权威状态、FNV-1a 状态哈希或 `SNAPSHOT_VERSION`
//! （当前 3）快照（ADR 0002 §1、§5、design D2 / D4）。

use w3_core::{Event, WorldView};
use w3_render::{FrameInput, ModelLibrary, Scene, SceneVertex, TerrainMesh};

/// 呈现一帧只读数据：消费视图与事件、按 `alpha` 插值、生成 GPU 顶点。
///
/// `alpha` MUST 只来自固定累加器的整数余数（见 [`crate::FrameAccumulator::alpha`]），
/// 且只允许 `f32` 存在于本表现层；它被钳到 `[0, 1]` 后仅用于
/// `Scene::interpolate_all`。重复消费同一视图或事件批次、任意 `alpha`
/// 连续插值、模型 / 字体资源缺失，都只改 `Scene` 内的派生视觉数据，
/// MUST NOT 改变任何权威状态或其哈希。
pub fn present(
    scene: &mut Scene,
    view: &WorldView,
    events: &[Event],
    alpha: f32,
    library: &ModelLibrary,
    terrain: &TerrainMesh,
) -> Vec<SceneVertex> {
    scene.consume(FrameInput { view, events }, library);
    scene.interpolate_all(alpha.clamp(0.0, 1.0));
    scene.build_vertices(terrain)
}
