//! w3-render：只读 3D 呈现层（P3）。
//!
//! 本 crate 是表现层，唯一允许持有的实体状态是 [`scene::Scene`] 里的
//! `EntityId → 实例` 映射；它只消费 `w3_core::WorldView` 的所有权拷贝与
//! 事实事件切片，绝不持有 `RtsMatch`，也绝不反写任何权威模拟状态
//! （见 ADR 0002 与 `3d-rendering` spec）。
//!
//! 分层：`w3-core` 不依赖本 crate；本 crate 依赖 `w3-core` / `w3-content`
//! 读取冻结输入。所有 `f32` / `f64` 只出现在表现层顶点、矩阵与插值中，
//! 不进入快照与状态哈希；模型加载失败、动画丢帧、位置插值都只影响呈现。
//!
//! 复用的上游规格见 `godot_warcraft3/docs/design/asset-convert/`：
//! MDX 骨骼、Sequence、GeosetAnim、FilterMode、PE2 挂点。导出资产留在
//! 玩家本机，绝不入库（见 ADR 0001）。

pub mod error;
pub mod gpu;
pub mod model;
pub mod scene;
pub mod terrain;

pub use error::RenderError;
pub use gpu::{Camera, FrameProducer, GpuContext, RendererApp, ScreenProjection};
pub use model::{
    Attachment, Bone, FilterMode, GeosetAnim, Material, Model, ModelDocument, ModelLibrary,
    ResolvedMaterial, Sequence, TEAM_COLORS, Track, team_color,
};
pub use scene::{
    DisplayHints, FrameInput, Instance, LORDAERON_SUMMER_TILES, Scene, SceneVertex, StaticMarker,
    StaticShape, UNIT_MARKER_SIZE, display_hints, facing_to_radians, world_to_f32,
};
pub use terrain::{TerrainMesh, TerrainVertex};
