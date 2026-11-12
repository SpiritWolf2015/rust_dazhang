//! 真实模型装配：把导出地图的摆放与动态实体类型键解析成表现层顶点。
//!
//! 阶段 3（真实模型）：[`w3_content::ModelCatalog`] 把 `typeId` / `doodadID`
//! 映射到本机 glTF，`w3_content::load_gltf` 读网格 + PNG 贴图，
//! `w3_render::bake_model` 烘焙成三角形（贴图色 + alpha 镂空），再按摆放位姿
//! 展开成 [`SceneVertex`]。全部是**纯表现层派生数据**，MUST NOT 进入权威状态、
//! 状态哈希或快照（ADR 0002）。
//!
//! - 静态摆放（2588 个）：按 `WORLD_SCALE` 缩放到玩法世界、贴地、套用导出
//!   `angle`，预展开成一份静态顶点流，装配一次后由 `Scene` 逐帧拼接。
//! - 动态实体：`ENTITY_MODEL_FILES` 的类型键烘焙成 `BakedModel`，按实例位姿
//!   每帧展开。
//! - 同一 glTF 只解码一次（树占 1588 个摆放），用 `BTreeMap` 缓存保证稳定顺序。
//! - 缺失 / 损坏的单个模型只记英文警告并跳过该摆放，其余照常；不静默冒充成功。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use w3_content::{ENTITY_MODEL_FILES, ModelCatalog, ModelClass, ParsedMap, load_gltf};
use w3_render::{BakedModel, ModelTransform, SceneVertex, TerrainMesh, bake_model, expand_model};

use crate::real_map::WORLD_SCALE;

/// 静态模型顶点预算上限（防御异常导出导致内存 / 上传爆炸；纯表现层参数）。
const MAX_STATIC_MODEL_VERTICES: usize = 3_000_000;
/// 单个摆放烘焙模型的最大三角形数（超出跳过，防御异常资产）。
const MAX_TRIANGLES_PER_MODEL: usize = 20_000;

/// 装配统计（只读诊断，供日志与验收）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModelStats {
    /// 静态摆放总数。
    pub placements: usize,
    /// 成功解析到真实模型的摆放数。
    pub resolved: usize,
    /// 解析失败 / 无模型而跳过的摆放数。
    pub skipped: usize,
    /// 静态真实模型预展开顶点数。
    pub static_vertices: usize,
    /// 烘焙的动态实体模型数。
    pub entity_models: usize,
    /// 命中缓存（未重复解码）的次数。
    pub cache_hits: usize,
    /// 因顶点预算被截断的摆放数。
    pub budget_truncated: usize,
}

/// 装配产物：静态顶点流 + 动态实体模型库 + 统计。
#[derive(Clone, Debug)]
pub struct AssembledModels {
    pub static_vertices: Vec<SceneVertex>,
    pub entity_models: BTreeMap<String, BakedModel>,
    pub stats: ModelStats,
}

/// `doodadID` → 语义类别，用于 `file` 缺失时兜底到同类真实模型。
/// 与 `real_map::doodad_marker` 的树 / 岩石分类保持一致，并补足植物 / 水体。
fn doodad_class(id: &str) -> ModelClass {
    if id.starts_with("LT") || id.contains("Tree") {
        return ModelClass::Tree;
    }
    if id.contains("rk") || id.contains("Rk") || id.contains("Rock") {
        return ModelClass::Rock;
    }
    // 水体 / 水生装饰物（前缀含 W 的类别码或 Water 语义）。
    if id.starts_with("ZW") || id.starts_with("NW") || id.starts_with("AW") || id.starts_with("IW")
    {
        return ModelClass::Water;
    }
    ModelClass::Plant
}

/// 单位 `typeId` → 语义类别（建筑 / 单位 / 金矿 / 起始点）。
fn unit_class(type_id: &str) -> ModelClass {
    match type_id {
        "sloc" => ModelClass::Generic,
        id if id.starts_with('n') || id.starts_with('h') || id.starts_with('o') => {
            ModelClass::Structure
        }
        _ => ModelClass::Unit,
    }
}

fn to_model_class(shape: w3_render::StaticShape) -> ModelClass {
    use w3_render::StaticShape;
    match shape {
        StaticShape::Tree => ModelClass::Tree,
        StaticShape::Rock => ModelClass::Rock,
        StaticShape::GoldMine | StaticShape::Building => ModelClass::Structure,
        StaticShape::StartLocation | StaticShape::Generic => ModelClass::Generic,
    }
}

/// 把导出的 `angle`（弧度）转成表现层 yaw；非有限值退化为 0。
fn to_yaw(angle: f64) -> f32 {
    if angle.is_finite() { angle as f32 } else { 0.0 }
}

/// 摆放的世界坐标（缩放到玩法世界 + 贴地）。
fn placement_origin(position: &w3_content::Position3, terrain: &TerrainMesh) -> [f32; 3] {
    let x = (position.x * WORLD_SCALE) as f32;
    let z = (position.y * WORLD_SCALE) as f32;
    let y = terrain.height_at_world(x, z);
    [x, y, z]
}

struct ModelCache {
    catalog: ModelCatalog,
    baked: BTreeMap<PathBuf, BakedModel>,
    hits: usize,
}

impl ModelCache {
    fn new(catalog: ModelCatalog) -> Self {
        Self {
            catalog,
            baked: BTreeMap::new(),
            hits: 0,
        }
    }

    /// 按 glTF 路径取烘焙模型（带缓存）；失败返回 `None` 并记录英文警告。
    fn get(&mut self, path: &Path) -> Option<&BakedModel> {
        if self.baked.contains_key(path) {
            self.hits += 1;
            return self.baked.get(path);
        }
        let mesh = match load_gltf(path) {
            Ok(mesh) => mesh,
            Err(error) => {
                log::warn!(
                    target: "w3_client",
                    "model skipped: path={} error={}",
                    path.display(),
                    error
                );
                return None;
            }
        };
        let baked = bake_model(&mesh);
        if baked.is_empty() {
            log::warn!(
                target: "w3_client",
                "model skipped: path={} reason=empty_mesh",
                path.display()
            );
            return None;
        }
        if baked.triangle_count() > MAX_TRIANGLES_PER_MODEL {
            log::warn!(
                target: "w3_client",
                "model skipped: path={} reason=too_many_triangles count={}",
                path.display(),
                baked.triangle_count()
            );
            return None;
        }
        self.baked.insert(path.to_path_buf(), baked);
        self.baked.get(path)
    }
}

/// 解析并展开单个摆放，追加到 `static_vertices`。
///
/// 按候选顺序依次尝试，跳过上游空占位 / 损坏模型，取第一个能烘焙出真实网格
/// 的候选；全部失败才记 skipped。确定性，不引入随机源。
fn place_one(
    cache: &mut ModelCache,
    paths: Vec<PathBuf>,
    origin: [f32; 3],
    yaw: f32,
    static_vertices: &mut Vec<SceneVertex>,
    stats: &mut ModelStats,
) {
    stats.placements += 1;
    if paths.is_empty() {
        stats.skipped += 1;
        return;
    }
    if static_vertices.len() >= MAX_STATIC_MODEL_VERTICES {
        stats.budget_truncated += 1;
        return;
    }
    let transform = ModelTransform {
        origin,
        yaw,
        scale: 1.0,
    };
    for path in &paths {
        // 借出缓存模型 → 展开后立即释放借用，避免与后续可变借用冲突。
        let expanded = cache.get(path).map(|model| expand_model(model, &transform));
        if let Some(vertices) = expanded {
            static_vertices.extend(vertices);
            stats.resolved += 1;
            return;
        }
    }
    stats.skipped += 1;
}

/// 从 `ParsedMap` + 模型目录装配真实模型（纯函数，便于 CPU 自测）。
///
/// `catalog` 由调用方从 gitignored 的本机目录构建；无本机资产时可传入空
/// `ModelCatalog`，此时返回空顶点流（CI / 无资产环境优雅降级，既有测试不受影响）。
pub fn assemble_models(
    parsed: &ParsedMap,
    terrain: &TerrainMesh,
    catalog: ModelCatalog,
) -> AssembledModels {
    let mut cache = ModelCache::new(catalog);
    let mut stats = ModelStats::default();
    let mut static_vertices: Vec<SceneVertex> = Vec::new();

    for unit in &parsed.units {
        let origin = placement_origin(&unit.position, terrain);
        let class = unit_class(&unit.type_id);
        let variation = unit.variation.rem_euclid(8) as u32;
        let mut paths: Vec<PathBuf> = Vec::new();
        if let Some(path) = cache.catalog.resolve_unit(&unit.type_id) {
            paths.push(path);
        }
        for candidate in cache
            .catalog
            .resolve_doodad_paths(&unit.type_id, variation, class)
        {
            if !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }
        place_one(
            &mut cache,
            paths,
            origin,
            to_yaw(unit.angle),
            &mut static_vertices,
            &mut stats,
        );
    }
    for doodad in &parsed.doodads {
        let origin = placement_origin(&doodad.position, terrain);
        let class = doodad_class(&doodad.id);
        let paths = cache.catalog.resolve_doodad_paths(
            &doodad.id,
            doodad.variation.rem_euclid(8) as u32,
            class,
        );
        place_one(
            &mut cache,
            paths,
            origin,
            to_yaw(doodad.angle),
            &mut static_vertices,
            &mut stats,
        );
    }

    stats.static_vertices = static_vertices.len();
    stats.cache_hits = cache.hits;

    // 动态实体类型键 → 烘焙模型。
    let mut entity_models = BTreeMap::new();
    for (key, _) in ENTITY_MODEL_FILES {
        if let Some(path) = cache.catalog.resolve_entity_key(key) {
            if let Some(model) = cache.get(&path) {
                entity_models.insert((*key).to_string(), model.clone());
            }
        }
    }
    stats.entity_models = entity_models.len();

    if stats.skipped > 0 || stats.budget_truncated > 0 {
        log::warn!(
            target: "w3_client",
            "model assembly partial: placements={} resolved={} skipped={} truncated={} entity_models={}",
            stats.placements, stats.resolved, stats.skipped, stats.budget_truncated, stats.entity_models
        );
    }
    log::info!(
        target: "w3_client",
        "models assembled: placements={} resolved={} static_vertices={} entity_models={} cache_hits={}",
        stats.placements, stats.resolved, stats.static_vertices, stats.entity_models, stats.cache_hits
    );

    AssembledModels {
        static_vertices,
        entity_models,
        stats,
    }
}

/// 由 `StaticShape` 反查 `ModelClass`（供复用既有形状分类的调用方）。
pub fn class_for_shape(shape: w3_render::StaticShape) -> ModelClass {
    to_model_class(shape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_content::{CenterOffset, DoodadsFile, TerrainHeightfield, UnitsFile};

    fn empty_parsed() -> ParsedMap {
        ParsedMap {
            heightfield: TerrainHeightfield {
                tilepoint_width: 2,
                tilepoint_height: 2,
                map_width: 1,
                map_height: 1,
                center_offset: Some(CenterOffset { x: 0.0, y: 0.0 }),
                main_tileset: None,
                main_tileset_name: None,
                ground_tilesets: vec![],
                cliff_tilesets: vec![],
                tile_size: Some(128.0),
                heights: vec![0.0; 4],
                ground_textures: vec![0; 4],
                ground_variations: vec![0; 4],
                cliff_textures: vec![15; 4],
                cliff_variations: vec![0; 4],
                layer_heights: vec![0; 4],
                water_heights: vec![0.0; 4],
                flags_packed: vec![0; 4],
            },
            units: UnitsFile {
                format_version: 1,
                subversion: 0,
                count: 0,
                units: vec![],
            }
            .units,
            doodads: DoodadsFile {
                format_version: 1,
                subversion: 0,
                count: 0,
                doodads: vec![],
            }
            .doodads,
        }
    }

    fn terrain() -> TerrainMesh {
        let hf = TerrainHeightfield {
            tilepoint_width: 2,
            tilepoint_height: 2,
            map_width: 1,
            map_height: 1,
            center_offset: None,
            main_tileset: None,
            main_tileset_name: None,
            ground_tilesets: vec![],
            cliff_tilesets: vec![],
            tile_size: Some(128.0),
            heights: vec![0.0; 4],
            ground_textures: vec![0; 4],
            ground_variations: vec![0; 4],
            cliff_textures: vec![15; 4],
            cliff_variations: vec![0; 4],
            layer_heights: vec![0; 4],
            water_heights: vec![0.0; 4],
            flags_packed: vec![0; 4],
        };
        TerrainMesh::from_heightfield(&hf).unwrap()
    }

    #[test]
    fn empty_catalog_degrades_to_no_model_vertices() {
        let parsed = empty_parsed();
        let result = assemble_models(&parsed, &terrain(), ModelCatalog::default());
        assert!(result.static_vertices.is_empty());
        assert!(result.entity_models.is_empty());
        assert_eq!(result.stats.placements, 0);
    }

    #[test]
    fn doodad_and_unit_classification_is_stable() {
        assert_eq!(doodad_class("LTlt"), ModelClass::Tree);
        assert_eq!(doodad_class("LRrk"), ModelClass::Rock);
        assert_eq!(doodad_class("ZWsw"), ModelClass::Water);
        assert_eq!(doodad_class("ZPsh"), ModelClass::Plant);
        assert_eq!(unit_class("sloc"), ModelClass::Generic);
        assert_eq!(unit_class("htow"), ModelClass::Structure);
        assert_eq!(unit_class("ngol"), ModelClass::Structure);
    }

    #[test]
    fn to_yaw_rejects_non_finite() {
        assert_eq!(to_yaw(f64::NAN), 0.0);
        assert_eq!(to_yaw(f64::INFINITY), 0.0);
        assert!((to_yaw(1.5) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn placement_origin_is_finite_and_scaled() {
        let terrain = terrain();
        let origin = placement_origin(
            &w3_content::Position3 {
                x: 128.0,
                y: 256.0,
                z: 0.0,
            },
            &terrain,
        );
        assert!((origin[0] - (128.0 * WORLD_SCALE) as f32).abs() < 1e-5);
        assert!((origin[2] - (256.0 * WORLD_SCALE) as f32).abs() < 1e-5);
        assert!(origin[1].is_finite(), "高度来自地形插值，必须有限");
    }
}
