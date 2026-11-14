//! 只读消费边界与表现层场景状态。
//!
//! `Scene` 是表现层**唯一**持有实体状态的地方：它只消费 `WorldView` 的
//! 所有权拷贝与 `Event` 切片，从不持有 `RtsMatch`，因此结构上无法反写
//! 任何权威模拟状态（见 ADR 0002 与 `3d-rendering` spec）。
//! 视觉插值、动画切换、模型缺失降级全部只改本结构内的 `f32` / 枚举数据。

use std::collections::{BTreeMap, BTreeSet};

use w3_content::RgbaImage;
use w3_core::{ANGLE_SCALE, Event, EventKind, SCALE, WorldView};

use crate::model::{ModelLibrary, team_color};
use crate::model_geometry::{
    BakedModel, ENTITY_DISPLAY_SCALE, MODEL_DISPLAY_SCALE, ModelTransform, expand_model,
};
use crate::terrain::{TerrainMesh, TerrainVertex};

/// 定点世界坐标（Q47.16）→ 表现层 `f32` 世界单位。
pub const fn world_to_f32(raw: i64) -> f32 {
    raw as f32 / SCALE as f32
}

/// 离散角（一整圈 = `ANGLE_SCALE`）→ 弧度，仅供表现层旋转。
pub fn facing_to_radians(raw: i64) -> f32 {
    raw as f32 / ANGLE_SCALE as f32 * std::f32::consts::TAU
}

/// 一帧只读输入：调用方从 `RtsMatch::read_view()` / `drain_events()` 取得。
/// 生命周期只覆盖一次 `consume` 调用，`Scene` 不会保存它。
pub struct FrameInput<'a> {
    pub view: &'a WorldView,
    pub events: &'a [Event],
}

/// 纹理采样模式（写入顶点 `tex_mode`，由片元着色器分支）。
/// - `0` 顶点色：`color` 即最终颜色（程序化几何 / 无贴图资产 / 纯色水体）。
/// - `1` 纹理不透明：采样 `texture_2d_array` 第 `tex_layer` 层。
/// - `2` 纹理 + alpha test：采样并对 alpha 低于阈值者 `discard`（树叶 / 旗帜）。
pub const TEX_MODE_COLOR: f32 = 0.0;
pub const TEX_MODE_TEXTURE: f32 = 1.0;
pub const TEX_MODE_ALPHA_TEST: f32 = 2.0;

/// 地形 GPU 平铺贴图在统一 `texture_2d_array` 中的层索引。
///
/// 由装配层把 6 张地表 tileset + 1 张水贴图追加到模型贴图数组后注入
/// [`Scene`]；`build_vertices` 据此让地形改走 GPU 逐像素平铺采样，而不再用
/// CPU 逐顶点颜色（后者三角形内是插值纯色，无法呈现贴图细节）。层索引均为
/// 表现层 `f32`，负数表示该层缺失、对应顶点回退到纯色。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainTextureLayers {
    /// 6 张地表 tileset 层索引，与 `groundTextures` 的 0..5 对应。
    pub ground: [f32; 6],
    /// 水面贴图层索引；负数表示无水贴图、水顶点回退半透明纯色。
    pub water: f32,
}

/// GPU 顶点；`#[repr(C)]` + `bytemuck::Pod`，可直接上传。
///
/// 位置 / 法线 / 颜色之外新增贴图 UV、数组纹理层与采样模式，供真 GPU 纹理
/// 管线使用；`surface` 保留旧语义（地表索引 / 单位 -1），供 HUD 与 CPU 验收。
/// 全部字段为 `f32`，15 个分量 = 60 字节，无 padding，`Pod` 合法。
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    /// 贴图 UV（地形为世界平铺坐标，模型为 glTF UV）。
    pub uv: [f32; 2],
    /// `texture_2d_array` 层索引；负值表示不使用纹理。
    pub tex_layer: f32,
    /// [`TEX_MODE_COLOR`] / [`TEX_MODE_TEXTURE`] / [`TEX_MODE_ALPHA_TEST`]。
    pub tex_mode: f32,
    /// 地表索引；单位标记为 -1。
    pub surface: f32,
}

impl SceneVertex {
    /// 构造纯顶点色顶点（程序化几何 / 测试 / 纯色），采样模式为 `COLOR`。
    /// 保留既有路径语义，纹理相关字段取中性值。
    pub fn colored(position: [f32; 3], normal: [f32; 3], color: [f32; 4], surface: f32) -> Self {
        Self {
            position,
            normal,
            color,
            uv: [0.0, 0.0],
            tex_layer: -1.0,
            tex_mode: TEX_MODE_COLOR,
            surface,
        }
    }

    /// 构造纹理采样顶点（地形 / 真实 glTF 模型）。
    pub fn textured(
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        uv: [f32; 2],
        tex_layer: f32,
        tex_mode: f32,
        surface: f32,
    ) -> Self {
        Self {
            position,
            normal,
            color,
            uv,
            tex_layer,
            tex_mode,
            surface,
        }
    }
}

/// 表现层实例：`EntityId → 视觉状态`。全部为派生数据，绝不回写内核。
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub id: u64,
    /// 插值后的当前视觉位置（世界单位）。
    pub x: f32,
    pub y: f32,
    /// 上一帧 / 本帧逻辑位置，用于两帧之间插值。
    pub prev_x: f32,
    pub prev_y: f32,
    pub target_x: f32,
    pub target_y: f32,
    /// 朝向（弧度）。
    pub facing: f32,
    pub team: u8,
    pub hp: i64,
    pub alive: bool,
    /// 模型资源键；`None` 表示资源缺失，单位不显示。
    pub model_key: Option<String>,
    /// 当前动画序列索引。
    pub sequence: usize,
}

impl Instance {
    /// 两帧之间视觉插值：只改 `x` / `y`，不触碰 `target_*` 或任何权威状态。
    pub fn interpolate(&mut self, alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        self.x = self.prev_x + (self.target_x - self.prev_x) * alpha;
        self.y = self.prev_y + (self.target_y - self.prev_y) * alpha;
    }
}

/// 表现层持有的场景状态。`BTreeMap` 保证稳定迭代顺序，使 readback 可复现。
#[derive(Default)]
pub struct Scene {
    instances: BTreeMap<u64, Instance>,
    /// 导出地图的静态摆放（units / doodads），纯表现层，MUST NOT 进入
    /// 权威状态 / 状态哈希 / 快照；由 `set_static_markers` 注入，
    /// `consume` 不触碰。
    static_markers: Vec<StaticMarker>,
    /// 真实地表 tileset 贴图的 CPU 副本（6 层 RGBA8）。纯表现层数据，
    /// 顶点色由它按 `surface` 层索引 + 世界 XZ 平铺采样得到；
    /// `None` 时回退到 6 色调色板（既有路径 / CI 无本机资产）。
    terrain_tiles: Option<Vec<RgbaImage>>,
    /// 地形 GPU 平铺贴图（6 张地表 tileset + 水面）在统一 `texture_2d_array`
    /// 中的层索引。`Some` 时地形改走 GPU 逐像素平铺采样（真 GPU 纹理管线），
    /// `None` 时回退 CPU 顶点色。纯表现层数据，由
    /// [`Scene::set_terrain_texture_layers`] 注入，`consume` 不触碰；
    /// 不进权威状态 / 状态哈希 / 快照（ADR 0002）。
    terrain_texture_layers: Option<TerrainTextureLayers>,
    /// 真实地图静态摆放的 glTF 模型（已按位姿预展开成 `SceneVertex`）。
    /// 由装配层注入，`build_vertices` 直接拼接；`consume` 不触碰。
    static_model_vertices: Vec<SceneVertex>,
    /// 动态实体类型键 → 真实烘焙模型（按实例位姿逐帧展开）。
    /// `BTreeMap` 保证稳定迭代；缺键时该实例回退到程序化几何。
    entity_models: BTreeMap<String, BakedModel>,
    /// 已消费事件的最高 sequence，用于重复投递去重。
    max_sequence: Option<u64>,
    /// 最近消费的逻辑帧号。
    last_tick: u64,
    /// 实际应用的事件数（诊断用，表现层元数据）。
    applied_events: u64,
}

fn switch_sequence(instance: &mut Instance, library: &ModelLibrary, name: &str) {
    let key = match &instance.model_key {
        Some(key) => key,
        None => return,
    };
    if let Some(model) = library.get(key) {
        if let Some(index) = model.sequence_index_by_name(name) {
            instance.sequence = index;
        }
    }
}

/// 应用一个已去重的事件到实例上；纯表现层副作用。
fn apply_event(instance: &mut Instance, event: &Event, library: &ModelLibrary) {
    match event.kind {
        EventKind::Spawned => {
            // 实例已由视图同步创建，这里保持幂等。
        }
        EventKind::Died => switch_sequence(instance, library, "death"),
        EventKind::AttackHit => switch_sequence(instance, library, "attack"),
        EventKind::Arrived | EventKind::Stopped => {
            switch_sequence(instance, library, "stand");
        }
        EventKind::MoveFailed => {}
    }
}

/// Lordaeron Summer 的 6 种地表调色板，索引与 `groundTilesets`
/// （`Ldrt` / `Ldro` / `Ldrg` / `Lrok` / `Lgrs` / `Lgrd`）一一对应。
/// 阶段 1 用稳定色块区分地表分区；真实 BLP 贴图由后续阶段接入。
pub const LORDAERON_SUMMER_TILES: [[f32; 4]; 6] = [
    [0.55, 0.45, 0.30, 1.0], // Ldrt  Dirt
    [0.46, 0.36, 0.25, 1.0], // Ldro  Rough Dirt
    [0.49, 0.55, 0.29, 1.0], // Ldrg  Grassy Dirt
    [0.53, 0.52, 0.49, 1.0], // Lrok  Rock
    [0.34, 0.57, 0.27, 1.0], // Lgrs  Grass
    [0.26, 0.44, 0.21, 1.0], // Lgrd  Dark Grass
];

/// `cliffTextures` 中 15 表示「无悬崖」，0..14 才是真实悬崖层。
const NO_CLIFF: f32 = 15.0;

/// 真实 tileset 贴图在世界 XZ 上的平铺周期（缩放后玩法世界单位）。
const TILE_REPEAT_WORLD: f32 = 16.0;
/// 可见水判定裕量（缩放后玩法世界单位）。
///
/// 真实 EI 高度场里，海床 `water-ground`（原始单位）=109~159、浅滩 / 陆地
/// 沟壑 =0~84，二者之间有 90~109 的天然间隙。直接 `water > ground` 会把
/// 浅滩也判成水，在绿地里铺出锯齿水面；加裕量只保留「明显低于水位的海床」
/// 为可见水，滤掉浅滩锯齿。1.6 世界单位 ≈ 原始 102 高度单位（离线量化
/// 落在间隙内），既滤浅滩又不误杀海床。纯表现层常量，不改权威状态 / 快照，
/// 仅影响地形 / 水面着色。
const WATER_VISIBLE_MARGIN: f32 = 1.6;

/// 从一张 RGBA8 tileset 贴图按世界 XZ 平铺采样一个像素，转为线性 RGBA。
/// 固定算法、同机可复现，供 CPU 侧把真实贴图颜色烘进地形顶点色（无需 GPU
/// 纹理采样 / shader 改动，避免真窗口画面在 CLI 侧无法验证的风险）；仅表现层
/// 派生数据，不进入权威状态 / 状态哈希 / 快照，不声称跨平台确定性。
fn sample_tile(image: &RgbaImage, world_x: f32, world_z: f32) -> [f32; 4] {
    if image.width == 0 || image.height == 0 || image.pixels.len() < 4 {
        return LORDAERON_SUMMER_TILES[0];
    }
    let u = (world_x / TILE_REPEAT_WORLD).rem_euclid(1.0);
    // 上游 BLP→PNG 未做 1-V 翻转（见 convert 报告）；这里按俯视方向翻转 V。
    let v = 1.0 - (world_z / TILE_REPEAT_WORLD).rem_euclid(1.0);
    let px = ((u * image.width as f32) as u32).min(image.width - 1);
    let py = ((v * image.height as f32) as u32).min(image.height - 1);
    let index = (py * image.width + px) as usize * 4;
    let Some(pixel) = image.pixels.get(index..index + 4) else {
        return LORDAERON_SUMMER_TILES[0];
    };
    // sRGB→线性近似（gamma 2.2），保证光照叠加后颜色不发灰；固定算法、
    // 仅用于表现层顶点色，同机可复现，不进入权威状态 / 哈希 / 快照。
    let to_linear = |c: u8| (c as f32 / 255.0).powf(2.2);
    [
        to_linear(pixel[0]),
        to_linear(pixel[1]),
        to_linear(pixel[2]),
        pixel[3] as f32 / 255.0,
    ]
}

/// 可见水判定：水位明显高于地面顶点才呈现水体（含 [`WATER_VISIBLE_MARGIN`]）。
///
/// `waterHeights` 是水面基准（真实导出为恒定水位，未含层高修正），`position[1]`
/// 是含层高修正的最终地面高度；二者都是缩放后的表现层 `f32`。加裕量滤掉
/// 水位线附近的浅滩 / 沟壑，避免绿地里出现锯齿水面。EI 为岛屿地图，浅海
/// 占比高，水被渲染成有贴图的水面（见 [`build_terrain_vertex`]），而非旧路径
/// 的纯蓝平板。
fn is_water_vertex(vertex: &TerrainVertex) -> bool {
    vertex.water > vertex.position[1] + WATER_VISIBLE_MARGIN
}

/// 地形顶点：真 GPU 纹理管线路径下按水/陆分流到 `textured` 顶点，交由片元
/// 着色器按世界 XZ 逐像素平铺采样 `texture_2d_array`（清晰地表 / 水面贴图）；
/// 无贴图数组时回退 CPU 逐顶点颜色 [`terrain_color`]（CI / 既有入口零改动）。
///
/// `uv` 用世界坐标除以平铺周期，配合 `Repeat` sampler 自动平铺。水顶点采样
/// 水波纹贴图并叠浅蓝调制色；`is_water_vertex` 带裕量滤掉浅滩，使水岸以
/// 顶点级过渡而非把浅水铺进绿地。`tex_layer` 在着色器里是
/// `@interpolate(flat)`，跨水陆三角形取 provoking 顶点的层，水岸硬对齐格点、
/// 不产生半透明插值伪影。纯表现层派生数据。
fn build_terrain_vertex(
    vertex: &TerrainVertex,
    tiles: Option<&[RgbaImage]>,
    layers: Option<&TerrainTextureLayers>,
) -> SceneVertex {
    let Some(layers) = layers else {
        return SceneVertex::colored(
            vertex.position,
            vertex.normal,
            terrain_color(vertex, tiles),
            vertex.surface,
        );
    };
    let water = is_water_vertex(vertex);
    let layer = if water {
        layers.water
    } else {
        let index = (vertex.surface as i64).clamp(0, 5) as usize;
        layers.ground[index]
    };
    // 层缺失（负数）时该顶点回退纯色，避免采样到错误层出现花屏。
    if layer < 0.0 {
        let fallback = if water {
            [0.18, 0.38, 0.62, 0.92]
        } else {
            terrain_color(vertex, tiles)
        };
        return SceneVertex::colored(vertex.position, vertex.normal, fallback, vertex.surface);
    }
    let tint = if water {
        [0.78, 0.90, 1.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    };
    let uv = [
        vertex.position[0] / TILE_REPEAT_WORLD,
        vertex.position[2] / TILE_REPEAT_WORLD,
    ];
    SceneVertex::textured(
        vertex.position,
        vertex.normal,
        tint,
        uv,
        layer,
        TEX_MODE_TEXTURE,
        vertex.surface,
    )
}

/// 地形顶点色：优先真实 tileset 贴图采样，否则回退 6 色调色板。
fn terrain_color(vertex: &TerrainVertex, tiles: Option<&[RgbaImage]>) -> [f32; 4] {
    // CPU 回退路径与 GPU 路径共用同一可见水判定（带裕量），避免判定漂移。
    if is_water_vertex(vertex) {
        return [0.18, 0.38, 0.62, 0.92];
    }
    // 悬崖：15 是「无」哨兵值，0..14 才是真实悬崖层（含 0）。
    if vertex.cliff >= 0.0 && vertex.cliff < NO_CLIFF && vertex.layer > 0.0 {
        return [0.42, 0.37, 0.31, 1.0];
    }
    let index = vertex.surface as i64;
    if let Some(image) = tiles.and_then(|t| t.get(index as usize)) {
        return sample_tile(image, vertex.position[0], vertex.position[2]);
    }
    // 回退：groundTextures 索引取调色板，越界回退到主地表。
    let tile = if (0..6).contains(&index) {
        LORDAERON_SUMMER_TILES[index as usize]
    } else {
        LORDAERON_SUMMER_TILES[0]
    };
    let shade = (vertex.layer * 0.07).fract();
    [
        tile[0] * (0.92 + shade * 0.12),
        tile[1] * (0.92 + shade * 0.12),
        tile[2] * (0.92 + shade * 0.12),
        tile[3],
    ]
}

/// 测试用回退路径入口：`None` 贴图即 6 色调色板。生产路径由
/// `build_vertices` 直接调用 `terrain_color(vertex, tiles)`，故本函数仅在
/// 测试编译中存在，用于验证无本机资产时的降级表现。
#[cfg(test)]
fn surface_color(vertex: &TerrainVertex) -> [f32; 4] {
    terrain_color(vertex, None)
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    [n[0] / len, n[1] / len, n[2] / len]
}

/// 只读派生的实体展示提示（design D5）：队伍色、选中环与血条比例。
/// 纯函数，不写回任何状态、不进入 FNV-1a 状态哈希或快照。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayHints {
    pub team_color: [f32; 4],
    pub selection_ring: Option<[f32; 4]>,
    pub hp_fraction: f32,
}

/// 由只读字段派生展示提示；`selected` 来自表现层选中集合。
pub fn display_hints(team: u8, hp: i64, max_hp: i64, selected: bool) -> DisplayHints {
    DisplayHints {
        team_color: team_color(team),
        selection_ring: selected.then_some([0.40, 1.0, 0.40, 1.0]),
        hp_fraction: (hp as f32 / max_hp.max(1) as f32).clamp(0.0, 1.0),
    }
}

pub const UNIT_MARKER_SIZE: f32 = 24.0;

/// 压入一个三角形（单位几何统一入口），应用给定队伍色。
fn push_triangle(
    vertices: &mut Vec<SceneVertex>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    color: [f32; 4],
) {
    let normal = triangle_normal(p0, p1, p2);
    for position in [p0, p1, p2] {
        vertices.push(SceneVertex::colored(position, normal, color, -1.0));
    }
}

/// 几何锚点：世界 XZ 坐标与朝向。动态单位与导出地图的静态摆放共用同一套
/// 几何函数，避免重复；全部为表现层 `f32`，不写回权威状态。
#[derive(Clone, Copy, Debug)]
struct MarkerPose {
    x: f32,
    y: f32,
    facing: f32,
}

impl MarkerPose {
    fn from_instance(instance: &Instance) -> Self {
        Self {
            x: instance.x,
            y: instance.y,
            facing: instance.facing,
        }
    }
}

/// 追加可缩放、可着色的四面体（单位占位标记）。
fn append_tetra(
    vertices: &mut Vec<SceneVertex>,
    pose: &MarkerPose,
    terrain: &TerrainMesh,
    size: f32,
    color: [f32; 4],
) {
    let base_y = terrain.height_at_world(pose.x, pose.y);
    let (sin, cos) = pose.facing.sin_cos();
    let forward = [cos, sin];
    let apex = [pose.x, base_y + size * 1.6, pose.y];
    let front = [
        pose.x + forward[0] * size,
        base_y,
        pose.y + forward[1] * size,
    ];
    let left = [
        pose.x - forward[0] * size * 0.5 - forward[1] * size * 0.6,
        base_y,
        pose.y - forward[1] * size * 0.5 + forward[0] * size * 0.6,
    ];
    let right = [
        pose.x - forward[0] * size * 0.5 + forward[1] * size * 0.6,
        base_y,
        pose.y - forward[1] * size * 0.5 - forward[0] * size * 0.6,
    ];
    for &(p0, p1, p2) in &[
        (apex, front, left),
        (apex, left, right),
        (apex, right, front),
        (front, right, left),
    ] {
        push_triangle(vertices, p0, p1, p2, color);
    }
}

/// 追加一个朝向标记（四面体）作为单位占位呈现，贴地并应用队伍色。
fn append_unit_marker(vertices: &mut Vec<SceneVertex>, instance: &Instance, terrain: &TerrainMesh) {
    let color = team_color(instance.team);
    append_tetra(
        vertices,
        &MarkerPose::from_instance(instance),
        terrain,
        UNIT_MARKER_SIZE,
        color,
    );
}

/// 追加长方体（建筑）：四个侧面与顶面，不含底面（贴地）。
fn append_box(
    vertices: &mut Vec<SceneVertex>,
    pose: &MarkerPose,
    terrain: &TerrainMesh,
    half: f32,
    height: f32,
    color: [f32; 4],
) {
    let base_y = terrain.height_at_world(pose.x, pose.y);
    let (x, z) = (pose.x, pose.y);
    let b0 = [x - half, base_y, z - half];
    let b1 = [x + half, base_y, z - half];
    let b2 = [x + half, base_y, z + half];
    let b3 = [x - half, base_y, z + half];
    let t0 = [x - half, base_y + height, z - half];
    let t1 = [x + half, base_y + height, z - half];
    let t2 = [x + half, base_y + height, z + half];
    let t3 = [x - half, base_y + height, z + half];
    push_triangle(vertices, b0, b1, t1, color);
    push_triangle(vertices, b0, t1, t0, color);
    push_triangle(vertices, b1, b2, t2, color);
    push_triangle(vertices, b1, t2, t1, color);
    push_triangle(vertices, b2, b3, t3, color);
    push_triangle(vertices, b2, t3, t2, color);
    push_triangle(vertices, b3, b0, t0, color);
    push_triangle(vertices, b3, t0, t3, color);
    push_triangle(vertices, t0, t1, t2, color);
    push_triangle(vertices, t0, t2, t3, color);
}

/// 追加八面体（金矿）：上下两顶点与四个赤道点。
fn append_octahedron(
    vertices: &mut Vec<SceneVertex>,
    pose: &MarkerPose,
    terrain: &TerrainMesh,
    size: f32,
    color: [f32; 4],
) {
    let base_y = terrain.height_at_world(pose.x, pose.y);
    let (x, z) = (pose.x, pose.y);
    let top = [x, base_y + size * 1.6, z];
    let bottom = [x, base_y, z];
    let east = [x + size, base_y + size * 0.8, z];
    let south = [x, base_y + size * 0.8, z + size];
    let west = [x - size, base_y + size * 0.8, z];
    let north = [x, base_y + size * 0.8, z - size];
    for &(a, b) in &[(east, south), (south, west), (west, north), (north, east)] {
        push_triangle(vertices, top, a, b, color);
        push_triangle(vertices, bottom, b, a, color);
    }
}

/// 追加圆锥（树）：六段侧面三角形。
fn append_cone(
    vertices: &mut Vec<SceneVertex>,
    pose: &MarkerPose,
    terrain: &TerrainMesh,
    size: f32,
    color: [f32; 4],
) {
    let base_y = terrain.height_at_world(pose.x, pose.y);
    let (x, z) = (pose.x, pose.y);
    let apex = [x, base_y + size * 2.2, z];
    let segments = 6usize;
    let ring: Vec<[f32; 3]> = (0..segments)
        .map(|index| {
            let angle = (index as f32) * std::f32::consts::TAU / segments as f32;
            [x + angle.cos() * size, base_y, z + angle.sin() * size]
        })
        .collect();
    for index in 0..segments {
        let next = (index + 1) % segments;
        push_triangle(vertices, apex, ring[index], ring[next], color);
    }
}

/// 按类型键分派几何；不同键输出不同形状（design D5）。
/// 全部为程序化几何、零资产读取，且不写回任何权威状态。
fn append_typed_marker(
    vertices: &mut Vec<SceneVertex>,
    instance: &Instance,
    terrain: &TerrainMesh,
    key: &str,
) {
    let pose = MarkerPose::from_instance(instance);
    let color = team_color(instance.team);
    let size = UNIT_MARKER_SIZE;
    match key {
        "town-hall" => append_box(vertices, &pose, terrain, size * 0.9, size * 1.4, color),
        "barracks" => append_box(vertices, &pose, terrain, size * 0.7, size * 0.9, color),
        "gold-mine" => append_octahedron(vertices, &pose, terrain, size * 0.9, color),
        "tree" => append_cone(vertices, &pose, terrain, size * 0.8, color),
        "peasant" => append_tetra(vertices, &pose, terrain, size * 0.55, color),
        _ => append_unit_marker(vertices, instance, terrain),
    }
}

/// 静态摆放物的形状分类（由装配层按导出 `typeId`/`id` 判定）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticShape {
    /// 树木 / 灌木：圆锥。
    Tree,
    /// 金矿：八面体。
    GoldMine,
    /// 建筑：长方体。
    Building,
    /// 岩石 / 杂物：小八面体。
    Rock,
    /// 起始点：小四面体。
    StartLocation,
    /// 其它单位 / 中立建筑：小四面体。
    Generic,
}

/// 导出地图的静态摆放（units.json / doodads.json）。纯表现层几何，
/// MUST NOT 进入权威状态、状态哈希或快照；由 [`Scene::set_static_markers`]
/// 一次性注入，`consume` 不触碰。
#[derive(Clone, Debug, PartialEq)]
pub struct StaticMarker {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub shape: StaticShape,
}

impl StaticMarker {
    fn pose(&self) -> MarkerPose {
        MarkerPose {
            x: self.x,
            y: self.y,
            facing: 0.0,
        }
    }
}

/// 追加一个静态摆放物；颜色由装配层给出（不用队伍色）。
fn append_static_marker(
    vertices: &mut Vec<SceneVertex>,
    marker: &StaticMarker,
    terrain: &TerrainMesh,
) {
    let pose = marker.pose();
    let size = marker.size;
    let color = marker.color;
    match marker.shape {
        StaticShape::Tree => append_cone(vertices, &pose, terrain, size, color),
        StaticShape::GoldMine => append_octahedron(vertices, &pose, terrain, size * 0.9, color),
        StaticShape::Building => {
            append_box(vertices, &pose, terrain, size * 0.7, size * 1.3, color)
        }
        StaticShape::Rock => append_octahedron(vertices, &pose, terrain, size * 0.5, color),
        StaticShape::StartLocation => append_tetra(vertices, &pose, terrain, size * 0.5, color),
        StaticShape::Generic => append_tetra(vertices, &pose, terrain, size * 0.6, color),
    }
}

impl Scene {
    /// 消费一帧只读视图与事件，更新表现层实例映射。
    ///
    /// 权威来源是 `view.entities`（与事件是否丢失无关）；事件只用于动画
    /// 序列切换，并按 `sequence` 去重（`drain_events()` 可能重复投递）。
    /// 本函数不接触 `RtsMatch`，因此无法改变模拟推进。
    pub fn consume(&mut self, input: FrameInput<'_>, library: &ModelLibrary) {
        let view = input.view;

        // 1. 按只读视图同步实例（唯一权威来源）。
        let mut present = BTreeSet::new();
        for entity in &view.entities {
            present.insert(entity.id);
            let target_x = world_to_f32(entity.x);
            let target_y = world_to_f32(entity.y);
            if let Some(instance) = self.instances.get_mut(&entity.id) {
                instance.prev_x = instance.target_x;
                instance.prev_y = instance.target_y;
                instance.target_x = target_x;
                instance.target_y = target_y;
                instance.facing = facing_to_radians(entity.facing);
                instance.team = entity.owner;
                instance.hp = entity.hp;
                instance.alive = entity.alive;
            } else {
                self.instances.insert(
                    entity.id,
                    Instance {
                        id: entity.id,
                        x: target_x,
                        y: target_y,
                        prev_x: target_x,
                        prev_y: target_y,
                        target_x,
                        target_y,
                        facing: facing_to_radians(entity.facing),
                        team: entity.owner,
                        hp: entity.hp,
                        alive: entity.alive,
                        model_key: library.resolve(entity),
                        sequence: 0,
                    },
                );
            }
        }
        self.instances.retain(|id, _| present.contains(id));

        // 2. 事件消费（按 sequence 稳定排序 + 去重）。
        let mut events: Vec<&Event> = input.events.iter().collect();
        events.sort_by_key(|event| event.sequence);
        for event in events {
            if let Some(max) = self.max_sequence {
                if event.sequence <= max {
                    continue;
                }
            }
            if let Some(instance) = self.instances.get_mut(&event.entity) {
                apply_event(instance, event, library);
            }
            self.max_sequence = Some(match self.max_sequence {
                Some(max) => max.max(event.sequence),
                None => event.sequence,
            });
            self.applied_events += 1;
        }
        self.last_tick = view.tick;
    }

    /// 稳定顺序迭代所有实例（只读）。
    pub fn instances(&self) -> impl Iterator<Item = &Instance> {
        self.instances.values()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn last_tick(&self) -> u64 {
        self.last_tick
    }

    pub fn max_sequence(&self) -> Option<u64> {
        self.max_sequence
    }

    pub fn applied_events(&self) -> u64 {
        self.applied_events
    }

    /// 对所有实例做视觉插值；只改表现层 `x` / `y`。
    pub fn interpolate_all(&mut self, alpha: f32) {
        for instance in self.instances.values_mut() {
            instance.interpolate(alpha);
        }
    }

    /// 注入导出地图的静态摆放（纯表现层）；`consume` 不会清除它们。
    /// 数据来自本机导出 JSON，MUST NOT 进入权威状态 / 状态哈希 / 快照。
    pub fn set_static_markers(&mut self, markers: Vec<StaticMarker>) {
        self.static_markers = markers;
    }

    /// 静态摆放数量（只读，供验收与诊断）。
    pub fn static_marker_count(&self) -> usize {
        self.static_markers.len()
    }

    /// 注入真实 tileset 贴图（6 层 RGBA8，纯表现层数据）。
    /// `None` 时地形回退到 6 色调色板；`consume` 不触碰本字段。
    pub fn set_terrain_tiles(&mut self, tiles: Option<Vec<RgbaImage>>) {
        self.terrain_tiles = tiles;
    }

    /// 注入地形 GPU 平铺贴图的层索引（真 GPU 纹理管线）。
    /// `Some` 时地形走 GPU 逐像素平铺采样，`None` 时回退 CPU 顶点色。
    /// 纯表现层数据，`consume` 不触碰；不进权威状态 / 状态哈希 / 快照。
    pub fn set_terrain_texture_layers(&mut self, layers: Option<TerrainTextureLayers>) {
        self.terrain_texture_layers = layers;
    }

    /// 注入真实地图静态摆放的 glTF 模型（已按位姿预展开的顶点）。
    /// 非空时 `build_vertices` 用它替代 `static_markers` 的程序化几何；
    /// 纯表现层数据，`consume` 不触碰。
    pub fn set_static_model_vertices(&mut self, vertices: Vec<SceneVertex>) {
        self.static_model_vertices = vertices;
    }

    /// 静态真实模型顶点数（只读，供验收 / 诊断）。
    pub fn static_model_vertex_count(&self) -> usize {
        self.static_model_vertices.len()
    }

    /// 注入动态实体类型键 → 真实烘焙模型；缺键的实例回退到程序化几何。
    pub fn set_entity_models(&mut self, models: BTreeMap<String, BakedModel>) {
        self.entity_models = models;
    }

    /// 真实绑定模型数（只读，供验收 / 诊断）。
    pub fn entity_model_count(&self) -> usize {
        self.entity_models.len()
    }

    /// 生成一帧的非索引 GPU 顶点：地形三角形 + 静态摆放 + 可见单位标记。
    /// 纯函数，稳定顺序、无随机源，便于 GPU readback 逐帧比对。
    ///
    /// 真实模型阶段：静态摆放优先用 [`Self::set_static_model_vertices`]
    /// 注入的真实网格顶点，动态单位优先用 [`Self::set_entity_models`]
    /// 绑定的真实网格按实例位姿展开；两者缺失时才回退程序化几何。
    pub fn build_vertices(&self, terrain: &TerrainMesh) -> Vec<SceneVertex> {
        let tiles = self.terrain_tiles.as_deref();
        let dynamic_model_estimate: usize = self
            .instances
            .values()
            .filter(|i| i.alive)
            .filter_map(|i| i.model_key.as_deref())
            .filter_map(|key| self.entity_models.get(key))
            .map(|m| m.triangles.len() * 3)
            .sum();
        let mut vertices = Vec::with_capacity(
            terrain.indices.len()
                + self.static_model_vertices.len()
                + dynamic_model_estimate
                + (self.instances.len() + self.static_markers.len()) * 12,
        );
        let layers = self.terrain_texture_layers.as_ref();
        for &index in &terrain.indices {
            let vertex = &terrain.vertices[index as usize];
            // 真 GPU 纹理管线：有层索引时地形走 `textured` 顶点，由片元着色器
            // 按世界 XZ 逐像素平铺采样地表 / 水面贴图；无层索引（CI / 旧入口）
            // 回退 CPU 顶点色 `terrain_color`，既有测试零改动。
            vertices.push(build_terrain_vertex(vertex, tiles, layers));
        }
        // 静态摆放先于动态单位，保证 readback 顺序稳定。
        if self.static_model_vertices.is_empty() {
            for marker in &self.static_markers {
                append_static_marker(&mut vertices, marker, terrain);
            }
        } else {
            vertices.extend_from_slice(&self.static_model_vertices);
        }
        for instance in self.instances.values() {
            if !instance.alive {
                continue;
            }
            vertices.extend(self.instance_vertices(instance, terrain));
        }
        vertices
    }

    /// 单个存活实例的真实顶点流：优先绑定的真实 glTF 网格，缺键回退程序化
    /// 占位标记。`build_vertices` 与 `entity_screen_boxes` 共用本方法，确保
    /// 「用户看到的模型几何」与「HUD / 框选用的屏幕包围盒」逐顶点一致，
    /// 消除两套几何口径造成的选中框错位。纯表现层派生数据。
    fn instance_vertices(&self, instance: &Instance, terrain: &TerrainMesh) -> Vec<SceneVertex> {
        let Some(key) = instance.model_key.as_deref() else {
            return Vec::new();
        };
        if let Some(model) = self.entity_models.get(key) {
            if !model.is_empty() {
                let transform = ModelTransform {
                    origin: [
                        instance.x,
                        terrain.height_at_world(instance.x, instance.y),
                        instance.y,
                    ],
                    yaw: instance.facing,
                    scale: MODEL_DISPLAY_SCALE * ENTITY_DISPLAY_SCALE,
                };
                return expand_model(model, &transform);
            }
        }
        let mut vertices = Vec::new();
        append_typed_marker(&mut vertices, instance, terrain, key);
        vertices
    }

    /// 每个存活实体**真实模型顶点**投影到屏幕的包围盒（物理像素坐标），
    /// 作为 HUD 单位标记 / 选中框 / 框选命中的单一真值源。
    ///
    /// 背景：旧 `ScreenProjection::world_to_screen` / `unit_screen_bounds`
    /// 按程序化四面体 `UNIT_MARKER_SIZE=24` 的固定低空平面（y=19.2）估算，
    /// 与阶段 3 起放大 4× 且贴地（y≈地形高）的真实 glTF 模型严重错位；
    /// zoom 拉近后偏差放大到上百像素、包围盒近全屏，导致「选框不贴合、
    /// 选不上」。本方法直接投影 `instance_vertices` 的真实顶点，天然贴合
    /// 用户所见，且对任意模型尺度 / 地形高度 / 相机 zoom 都成立。
    ///
    /// 返回 `BTreeMap<实体ID, (屏幕最小点, 屏幕最大点)>`；模型在相机背后
    /// （投影失败）时不纳入。`projection` 与渲染 `Camera::matrix` 同源。
    pub fn entity_screen_boxes(
        &self,
        terrain: &TerrainMesh,
        projection: &crate::gpu::ScreenProjection,
    ) -> BTreeMap<u64, ([f64; 2], [f64; 2])> {
        let mut out = BTreeMap::new();
        for instance in self.instances.values() {
            if !instance.alive {
                continue;
            }
            let vertices = self.instance_vertices(instance, terrain);
            if vertices.is_empty() {
                continue;
            }
            let mut min = [f64::INFINITY, f64::INFINITY];
            let mut max = [f64::NEG_INFINITY, f64::NEG_INFINITY];
            for v in &vertices {
                let p = projection.project_world_point(
                    f64::from(v.position[0]),
                    f64::from(v.position[1]),
                    f64::from(v.position[2]),
                );
                for axis in 0..2 {
                    min[axis] = min[axis].min(p[axis]);
                    max[axis] = max[axis].max(p[axis]);
                }
            }
            // 投影失败（相机背后）返回 -1e6；以有限性判定，避免污染包围盒。
            if min.iter().all(|v| v.is_finite() && *v > -1.0e5)
                && max.iter().all(|v| v.is_finite() && *v < 1.0e5)
            {
                out.insert(instance.id, (min, max));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::EntityView;

    fn entity(id: u64, owner: u8, x: i64) -> EntityView {
        EntityView {
            id,
            x,
            y: 0,
            facing: 0,
            hp: 100,
            owner,
            alive: true,
            has_target: false,
        }
    }

    fn view(tick: u64, entities: Vec<EntityView>) -> WorldView {
        WorldView {
            tick,
            rng: 0,
            entities,
        }
    }

    fn event(sequence: u64, entity: u64, kind: EventKind) -> Event {
        Event {
            frame: 0,
            sequence,
            entity,
            kind,
        }
    }

    fn terrain() -> TerrainMesh {
        let hf: w3_content::TerrainHeightfield = serde_json::from_str(
            r#"{"tilepointWidth":2,"tilepointHeight":2,"mapWidth":1,"mapHeight":1,
                "heights":[0,0,0,0]}"#,
        )
        .unwrap();
        TerrainMesh::from_heightfield(&hf).unwrap()
    }

    #[test]
    fn consume_builds_instances_from_read_only_view() {
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 2, 65536), entity(2, 3, -131072)]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert_eq!(scene.len(), 2);
        assert_eq!(scene.last_tick(), 1);
        let ids: Vec<u64> = scene.instances().map(|i| i.id).collect();
        assert_eq!(ids, vec![1, 2]);
        let first = scene.instances().next().unwrap();
        assert!((first.target_x - 1.0).abs() < 1e-6);
        assert_eq!(first.team, 2);
        // 空模型库 → 资源缺失，单位不显示。
        assert_eq!(first.model_key, None);
    }

    #[test]
    fn despawned_entities_are_removed() {
        let mut scene = Scene::default();
        let library = ModelLibrary::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 0), entity(2, 0, 0)]),
                events: &[],
            },
            &library,
        );
        assert_eq!(scene.len(), 2);
        scene.consume(
            FrameInput {
                view: &view(2, vec![entity(1, 0, 0)]),
                events: &[],
            },
            &library,
        );
        assert_eq!(scene.len(), 1);
    }

    #[test]
    fn events_are_deduplicated_by_sequence() {
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 0, 0)]);
        let events = vec![
            event(1, 1, EventKind::AttackHit),
            event(2, 1, EventKind::Arrived),
        ];
        let library = ModelLibrary::default();
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 2);
        assert_eq!(scene.max_sequence(), Some(2));

        // 重复投递同一批：全部被去重。
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 2);

        // 新 sequence 才被应用。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[event(3, 1, EventKind::Died)],
            },
            &library,
        );
        assert_eq!(scene.applied_events(), 3);
        assert_eq!(scene.max_sequence(), Some(3));
    }

    #[test]
    fn interpolation_only_moves_visual_position() {
        let mut scene = Scene::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 0)]),
                events: &[],
            },
            &ModelLibrary::default(),
        );
        // 制造一次位置推进。
        scene.consume(
            FrameInput {
                view: &view(2, vec![entity(1, 0, 131072)]),
                events: &[],
            },
            &ModelLibrary::default(),
        );
        let instance = scene.instances().next().unwrap();
        assert_eq!(instance.prev_x, 0.0);
        assert_eq!(instance.target_x, 2.0);
        scene.interpolate_all(0.5);
        let instance = scene.instances().next().unwrap();
        assert!((instance.x - 1.0).abs() < 1e-6);
        // 逻辑目标不受插值影响。
        assert_eq!(instance.target_x, 2.0);
        assert_eq!(instance.prev_x, 0.0);
    }

    #[test]
    fn build_vertices_is_deterministic_and_skips_missing_models() {
        let mut library = ModelLibrary::default();
        library.insert("default", crate::model::Model::default());
        let mut scene = Scene::default();
        scene.consume(
            FrameInput {
                view: &view(1, vec![entity(1, 0, 65536)]),
                events: &[],
            },
            &library,
        );
        let mesh = terrain();
        let a = scene.build_vertices(&mesh);
        let b = scene.build_vertices(&mesh);
        assert_eq!(a, b, "顶点生成必须可复现");
        // 地形 6 索引 + 1 个单位标记 12 顶点 = 18。
        assert_eq!(a.len(), mesh.indices.len() + 12);
        assert!(a.iter().any(|v| v.surface < 0.0), "应含单位标记");
    }

    fn test_instance(key: &str, team: u8) -> Instance {
        Instance {
            id: 1,
            x: 0.0,
            y: 0.0,
            prev_x: 0.0,
            prev_y: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            facing: 0.0,
            team,
            hp: 100,
            alive: true,
            model_key: Some(key.to_string()),
            sequence: 0,
        }
    }

    #[test]
    fn typed_markers_produce_distinct_geometry() {
        let mesh = terrain();
        let keys = [
            "default",
            "peasant",
            "soldier",
            "town-hall",
            "barracks",
            "gold-mine",
            "tree",
        ];
        let mut counts = Vec::new();
        for key in keys {
            let instance = test_instance(key, 1);
            let mut vertices = Vec::new();
            append_typed_marker(&mut vertices, &instance, &mesh, key);
            assert!(!vertices.is_empty());
            assert!(vertices.iter().all(|v| v.surface < 0.0));
            counts.push(vertices.len());
        }
        let distinct: std::collections::BTreeSet<usize> = counts.iter().copied().collect();
        assert!(distinct.len() >= 3, "不同实体类型必须呈现不同几何");
        assert_ne!(counts[3], counts[5], "主城与金矿几何必须不同");
        assert_ne!(counts[1], counts[3], "农民与主城几何必须不同");
    }

    #[test]
    fn display_hints_derive_team_color_ring_and_hp() {
        let red = display_hints(1, 50, 100, false);
        let blue = display_hints(2, 50, 100, false);
        assert_ne!(red.team_color, blue.team_color, "敌我队伍色必须不同");
        assert_eq!(red.selection_ring, None);
        let selected = display_hints(1, 50, 100, true);
        assert!(selected.selection_ring.is_some());
        assert!((selected.hp_fraction - 0.5).abs() < 1.0e-6);
        assert_eq!(display_hints(1, 200, 100, false).hp_fraction, 1.0);
    }

    #[test]
    fn entity_binding_takes_priority_in_resolve() {
        let mut library = ModelLibrary::default();
        for key in ["default", "town-hall", "peasant"] {
            library.insert(key, crate::model::Model::default());
        }
        library.bind_team(1, "default");
        library.bind_entity(1, "town-hall");
        assert_eq!(
            library.resolve(&entity(1, 1, 0)).as_deref(),
            Some("town-hall")
        );
        assert_eq!(
            library.resolve(&entity(2, 1, 0)).as_deref(),
            Some("default")
        );
    }

    fn terrain_vertex(height: f32, surface: f32, cliff: f32, water: f32) -> TerrainVertex {
        TerrainVertex {
            position: [0.0, height, 0.0],
            normal: [0.0, 1.0, 0.0],
            surface,
            cliff,
            layer: 4.0,
            water,
        }
    }

    #[test]
    fn surface_color_water_requires_water_above_ground() {
        // 真实导出 waterHeights 恒为正；只有水面高于地面顶点才是水。
        let dry = surface_color(&terrain_vertex(722.0, 0.0, 15.0, 384.0));
        let wet = surface_color(&terrain_vertex(226.0, 0.0, 15.0, 384.0));
        assert_ne!(dry, wet, "同一 waterHeights 下陆地与水下必须不同");
        // dry 走地表调色板（非蓝），wet 走水体蓝。
        assert!(dry[1] > dry[2], "陆地偏绿而非水蓝");
        assert!(wet[2] > wet[1], "水下偏蓝");
    }

    #[test]
    fn surface_color_cliff_sentinel_is_not_a_cliff() {
        // cliffTextures=15 是「无悬崖」哨兵，不能刷成崖色。
        let no_cliff = surface_color(&terrain_vertex(500.0, 0.0, 15.0, 0.0));
        let real_cliff = surface_color(&terrain_vertex(500.0, 0.0, 1.0, 0.0));
        assert_ne!(no_cliff, real_cliff);
    }

    #[test]
    fn ground_tileset_indices_produce_distinct_colors() {
        let mut seen = std::collections::BTreeSet::new();
        for index in 0..6 {
            let color = surface_color(&terrain_vertex(500.0, index as f32, 15.0, 0.0));
            let key = color.map(|c| (c * 1000.0) as i64);
            seen.insert(key);
        }
        assert_eq!(seen.len(), 6, "6 种地表 tileset 必须呈现 6 种可区分颜色");
    }

    #[test]
    fn static_markers_render_and_survive_consume() {
        let mesh = terrain();
        let mut scene = Scene::default();
        let markers = vec![
            StaticMarker {
                x: 0.0,
                y: 0.0,
                size: 6.0,
                color: [0.1, 0.5, 0.2, 1.0],
                shape: StaticShape::Tree,
            },
            StaticMarker {
                x: 1.0,
                y: 0.0,
                size: 6.0,
                color: [0.9, 0.8, 0.1, 1.0],
                shape: StaticShape::GoldMine,
            },
        ];
        scene.set_static_markers(markers);
        assert_eq!(scene.static_marker_count(), 2);

        let before = scene.build_vertices(&mesh);
        assert!(before.len() > mesh.indices.len(), "静态摆放必须进入顶点");

        // consume 只同步动态实例，不清除静态摆放。
        let empty_view = view(2, vec![]);
        scene.consume(
            FrameInput {
                view: &empty_view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert_eq!(scene.static_marker_count(), 2);
        let after = scene.build_vertices(&mesh);
        assert_eq!(before, after, "静态摆放顶点必须稳定可复现");
        assert!(after.iter().any(|v| v.color[1] > 0.4 && v.surface < 0.0));
    }

    #[test]
    fn static_shapes_produce_distinct_geometry() {
        let mesh = terrain();
        let shapes = [
            StaticShape::Tree,
            StaticShape::GoldMine,
            StaticShape::Building,
            StaticShape::Rock,
            StaticShape::StartLocation,
            StaticShape::Generic,
        ];
        let mut counts = Vec::new();
        for shape in shapes {
            let marker = StaticMarker {
                x: 0.0,
                y: 0.0,
                size: 6.0,
                color: [0.5, 0.5, 0.5, 1.0],
                shape,
            };
            let mut vertices = Vec::new();
            append_static_marker(&mut vertices, &marker, &mesh);
            assert!(!vertices.is_empty());
            counts.push(vertices.len());
        }
        let distinct: std::collections::BTreeSet<usize> = counts.iter().copied().collect();
        assert!(distinct.len() >= 3, "静态摆放形状必须可区分");
    }

    fn solid_tile(color: [u8; 4]) -> RgbaImage {
        RgbaImage {
            width: 2,
            height: 2,
            pixels: color.repeat(4),
        }
    }

    #[test]
    fn terrain_color_uses_real_tile_pixels_not_palette() {
        let vertex = terrain_vertex(500.0, 2.0, 15.0, 0.0);
        // 无贴图：走 6 色调色板（index 2）。
        let palette = terrain_color(&vertex, None);
        // 有 6 层贴图：index 2 是纯红，采样结果应接近红色而非调色板绿。
        let tiles: Vec<RgbaImage> = (0..6).map(|_| solid_tile([40, 200, 40, 255])).collect();
        let mut red_tiles = tiles.clone();
        red_tiles[2] = solid_tile([220, 30, 30, 255]);
        let sampled = terrain_color(&vertex, Some(&red_tiles));
        assert_ne!(sampled, palette, "贴图路径必须与调色板路径不同");
        assert!(
            sampled[0] > sampled[1],
            "采样应取到红色 tile，红通道大于绿通道"
        );
    }

    #[test]
    fn sample_tile_is_deterministic_and_oob_safe() {
        let image = solid_tile([100, 150, 200, 255]);
        let a = sample_tile(&image, 3.0, 7.0);
        let b = sample_tile(&image, 3.0, 7.0);
        assert_eq!(a, b);
        // 任意世界坐标（含负 / 越界）都落在 [0,1]，不 panic。
        for (x, z) in [(-130.0, -97.0), (0.0, 0.0), (1.0e6, -1.0e6)] {
            let c = sample_tile(&image, x, z);
            assert!(c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn build_vertices_real_tiles_survive_consume_and_are_stable() {
        let mesh = terrain();
        let mut scene = Scene::default();
        let tiles: Vec<RgbaImage> = (0..6).map(|_| solid_tile([90, 120, 60, 255])).collect();
        scene.set_terrain_tiles(Some(tiles));
        let before = scene.build_vertices(&mesh);
        let view = view(3, vec![]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        let after = scene.build_vertices(&mesh);
        assert_eq!(before, after, "真实贴图采样可复现、consume 不改地形顶点色");
    }

    /// 构造一个多三角形的真实模型（顶点数远超程序化标记的 12）。
    fn baked_model(triangles: usize) -> BakedModel {
        let mut list = Vec::with_capacity(triangles);
        for i in 0..triangles {
            let h = i as f32 * 0.1;
            list.push(crate::model_geometry::BakedTriangle::colored(
                [[0.0, h, 0.0], [2.0, h, 0.0], [0.0, h + 1.0, 0.0]],
                [[0.0, 1.0, 0.0]; 3],
                [[0.2, 0.6, 0.3, 1.0]; 3],
            ));
        }
        BakedModel { triangles: list }
    }

    #[test]
    fn static_real_models_replace_programmatic_markers() {
        let mesh = terrain();
        let mut scene = Scene::default();
        scene.set_static_markers(vec![StaticMarker {
            x: 1.0,
            y: 2.0,
            size: 6.0,
            color: [0.1, 0.5, 0.2, 1.0],
            shape: StaticShape::Tree,
        }]);
        let programmatic = scene.build_vertices(&mesh).len();

        let model = baked_model(40);
        let vertices = expand_model(
            &model,
            &ModelTransform {
                origin: [1.0, 0.0, 2.0],
                ..Default::default()
            },
        );
        scene.set_static_model_vertices(vertices.clone());
        assert_eq!(scene.static_model_vertex_count(), 120);
        let with_real = scene.build_vertices(&mesh);
        // 真实模型 120 顶点 >> 单个圆锥标记，且静态程序化几何被替换。
        assert!(with_real.len() > programmatic);
        assert_eq!(
            with_real.iter().filter(|v| v.surface < 0.0).count(),
            120,
            "静态摆放应全部来自真实模型顶点"
        );
        // 确定性 + consume 不清除。
        assert_eq!(with_real, scene.build_vertices(&mesh));
        let empty_view = view(4, vec![]);
        scene.consume(
            FrameInput {
                view: &empty_view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert_eq!(scene.static_model_vertex_count(), 120);
    }

    #[test]
    fn dynamic_real_models_replace_typed_markers_and_are_stable() {
        let mut library = ModelLibrary::default();
        library.insert("peasant", crate::model::Model::default());
        library.bind_entity(1, "peasant");
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 0, 0)]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let mesh = terrain();
        let before = scene.build_vertices(&mesh);
        // 程序化几何：四面体 12 顶点。
        assert_eq!(before.iter().filter(|v| v.surface < 0.0).count(), 12);

        let mut models = BTreeMap::new();
        models.insert("peasant".to_string(), baked_model(30));
        scene.set_entity_models(models);
        assert_eq!(scene.entity_model_count(), 1);
        let after = scene.build_vertices(&mesh);
        assert_eq!(
            after.iter().filter(|v| v.surface < 0.0).count(),
            90,
            "动态单位应由真实网格的 30 三角形展开"
        );
        assert!(after.len() > before.len());
        assert_eq!(after, scene.build_vertices(&mesh), "逐帧展开可复现");
    }

    #[test]
    fn missing_entity_model_falls_back_to_programmatic_marker() {
        let mut library = ModelLibrary::default();
        library.insert("peasant", crate::model::Model::default());
        library.bind_entity(1, "peasant");
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 0, 0)]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        // 注入了其它键的模型，peasant 无绑定 → 回退四面体。
        let mut models = BTreeMap::new();
        models.insert("soldier".to_string(), baked_model(10));
        scene.set_entity_models(models);
        let mesh = terrain();
        let vertices = scene.build_vertices(&mesh);
        assert_eq!(
            vertices.iter().filter(|v| v.surface < 0.0).count(),
            12,
            "缺键回退到程序化几何"
        );
    }

    fn test_layers() -> TerrainTextureLayers {
        TerrainTextureLayers {
            ground: [10.0, 11.0, 12.0, 13.0, 14.0, 15.0],
            water: 20.0,
        }
    }

    #[test]
    fn water_visible_margin_keeps_seabed_filters_shallows() {
        // 缩放后水位 6.0、裕量 1.6：海床 water-ground>1.6 判水，浅滩 <1.6 判陆。
        let seabed = terrain_vertex(3.0, 0.0, 15.0, 6.0);
        assert!(is_water_vertex(&seabed), "明显低于水位的海床应为可见水");
        let shallow = terrain_vertex(5.0, 0.0, 15.0, 6.0);
        assert!(
            !is_water_vertex(&shallow),
            "水位线附近的浅滩不应判为水（避免绿地锯齿水面）"
        );
        let dry = terrain_vertex(9.0, 0.0, 15.0, 6.0);
        assert!(!is_water_vertex(&dry), "高于水位的陆地不是水");
    }

    #[test]
    fn gpu_terrain_split_routes_land_and_water_to_different_layers() {
        let layers = test_layers();
        // 陆地顶点：地表层 3，白色调制，UV 按世界 XZ 平铺。
        let land_vertex = TerrainVertex {
            position: [32.0, 9.0, 16.0],
            normal: [0.0, 1.0, 0.0],
            surface: 3.0,
            cliff: 15.0,
            layer: 4.0,
            water: 6.0,
        };
        let land = build_terrain_vertex(&land_vertex, None, Some(&layers));
        assert_eq!(land.tex_mode, TEX_MODE_TEXTURE);
        assert_eq!(land.tex_layer, 13.0, "surface=3 应取 ground[3]");
        assert_eq!(land.uv, [2.0, 1.0], "uv = 世界坐标 / 16");
        assert_eq!(land.color, [1.0, 1.0, 1.0, 1.0], "陆地调制色为白色");

        // 水顶点：水层，浅蓝调制。
        let water_vertex = TerrainVertex {
            position: [0.0, 3.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            surface: 0.0,
            cliff: 15.0,
            layer: 4.0,
            water: 6.0,
        };
        let water = build_terrain_vertex(&water_vertex, None, Some(&layers));
        assert_eq!(water.tex_mode, TEX_MODE_TEXTURE);
        assert_eq!(water.tex_layer, 20.0, "水顶点应取 water 层");
        assert!(water.color[2] > water.color[0], "水调制色偏蓝");
    }

    #[test]
    fn gpu_terrain_without_layers_falls_back_to_colored() {
        let vertex = terrain_vertex(500.0, 2.0, 15.0, 0.0);
        let result = build_terrain_vertex(&vertex, None, None);
        assert_eq!(result.tex_mode, TEX_MODE_COLOR, "无 layers 回退 CPU 顶点色");
        assert_eq!(result.tex_layer, -1.0);
    }

    #[test]
    fn gpu_terrain_missing_layer_falls_back_to_solid_color() {
        let mut layers = test_layers();
        layers.ground[0] = -1.0;
        layers.water = -1.0;
        let vertex = terrain_vertex(9.0, 0.0, 15.0, 6.0);
        let land = build_terrain_vertex(&vertex, None, Some(&layers));
        assert_eq!(land.tex_mode, TEX_MODE_COLOR, "层缺失回退纯色，不花屏");
        let seabed = terrain_vertex(3.0, 0.0, 15.0, 6.0);
        let water = build_terrain_vertex(&seabed, None, Some(&layers));
        assert_eq!(water.tex_mode, TEX_MODE_COLOR);
    }

    #[test]
    fn terrain_texture_layers_build_vertices_is_deterministic() {
        let mesh = terrain();
        let mut scene = Scene::default();
        scene.set_terrain_texture_layers(Some(test_layers()));
        let a = scene.build_vertices(&mesh);
        let b = scene.build_vertices(&mesh);
        assert_eq!(a, b, "GPU 地形顶点逐帧可复现");
        assert!(
            a.iter()
                .filter(|v| v.surface >= 0.0)
                .all(|v| v.tex_mode == TEX_MODE_TEXTURE),
            "有 layers 时地形全部走 GPU 纹理采样"
        );
        // consume 不触碰 layers。
        let view = view(5, vec![]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert_eq!(a, scene.build_vertices(&mesh));
    }

    /// 构造一个带真实烘焙模型的 Scene：单个实体绑定 `peasant`，模型为
    /// `baked_model(n)` 的已知三角形集合。供 `entity_screen_boxes` 单测用。
    fn scene_with_real_model(triangles: usize) -> Scene {
        let mut library = ModelLibrary::default();
        library.insert("peasant", crate::model::Model::default());
        library.bind_entity(1, "peasant");
        let mut scene = Scene::default();
        let view = view(1, vec![entity(1, 1, 0)]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let mut models = BTreeMap::new();
        models.insert("peasant".to_string(), baked_model(triangles));
        scene.set_entity_models(models);
        scene
    }

    #[test]
    fn entity_screen_boxes_match_real_model_vertices() {
        use crate::Camera;
        use crate::gpu::ScreenProjection;
        let scene = scene_with_real_model(40);
        let mesh = terrain();
        let camera = Camera::new(16.0 / 9.0);
        let width = 1280.0f32;
        let height = 720.0f32;
        let projection = ScreenProjection::new(&camera, &mesh, width, height);
        let boxes = scene.entity_screen_boxes(&mesh, &projection);
        let (min, max) = boxes.get(&1).expect("entity 1 should have a screen box");

        // ① box 有限、落在视口内、尺寸合理（非旧 24 单位四面体的近全屏巨框）。
        assert!(min.iter().chain(max.iter()).all(|v| v.is_finite()));
        let w = max[0] - min[0];
        let h = max[1] - min[1];
        assert!(w > 0.0 && h > 0.0, "包围盒应有正尺寸，实际 {w}x{h}");
        assert!(
            w < f64::from(width) && h < f64::from(height),
            "包围盒不应超过视口，实际 {w}x{h}"
        );

        // ② box 精确等于该实体真实模型顶点投影的 AABB（单一真值源自洽）。
        let mut vmin = [f64::INFINITY, f64::INFINITY];
        let mut vmax = [f64::NEG_INFINITY, f64::NEG_INFINITY];
        for v in &scene.instance_vertices(scene.instances.get(&1).unwrap(), &mesh) {
            let p = projection.project_world_point(
                f64::from(v.position[0]),
                f64::from(v.position[1]),
                f64::from(v.position[2]),
            );
            for a in 0..2 {
                vmin[a] = vmin[a].min(p[a]);
                vmax[a] = vmax[a].max(p[a]);
            }
        }
        for a in 0..2 {
            assert!((min[a] - vmin[a]).abs() < 1.0e-6);
            assert!((max[a] - vmax[a]).abs() < 1.0e-6);
        }

        // ③ box 中心与旧 `world_to_screen`（低空四面体平面）明显不同，
        // 证明 HUD / 框选确实换了真值源、不再用错位的旧口径。
        let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
        let old = projection.world_to_screen([0.0, 0.0]);
        assert!(
            (center[1] - old[1]).abs() > 1.0,
            "真实模型中心不应等于旧低空平面投影点，center={center:?} old={old:?}"
        );
    }

    #[test]
    fn entity_screen_boxes_are_deterministic_and_skip_dead() {
        use crate::Camera;
        use crate::gpu::ScreenProjection;
        let mut scene = scene_with_real_model(20);
        let mesh = terrain();
        let camera = Camera::new(1.0);
        let projection = ScreenProjection::new(&camera, &mesh, 640.0, 480.0);
        let a = scene.entity_screen_boxes(&mesh, &projection);
        let b = scene.entity_screen_boxes(&mesh, &projection);
        assert_eq!(a, b, "逐帧投影可复现");
        assert!(a.contains_key(&1));
        // 死亡实体不产出 box：消费一个该实体 alive=false 的视图。
        let mut dead = entity(1, 1, 0);
        dead.alive = false;
        let view = view(2, vec![dead]);
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &ModelLibrary::default(),
        );
        assert!(
            !scene
                .entity_screen_boxes(&mesh, &projection)
                .contains_key(&1),
            "死亡实体不应有屏幕包围盒"
        );
    }
}
