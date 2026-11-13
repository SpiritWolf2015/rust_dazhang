//! 真实导出地图 + 真实 glTF 模型的表现层装配（对齐 `godot_warcraft3` 水位）。
//!
//! 本模块把本机 `map-parsed/<slug>/` 的导出数据装配成客户端可直接呈现的
//! `TerrainMesh` 与静态摆放 `StaticMarker`：
//!
//! - 原始导出坐标（`f64`，世界单位约 ±8192）是**冻结输入**（ADR 0002），
//!   不进入权威状态、状态哈希或快照；这里只在表现层缩放到玩法世界尺度。
//! - `TerrainHeightfield` 的网格拓扑（129×97）、地表 / 悬崖 / 层高 / 水体
//!   平行数组全部保留，仅缩放 `tileSize` / `centerOffset` / `heights` /
//!   `waterHeights`，使真实地图形状、分区、崖层、水体与摆放相对位置不变。
//! - 缩放目标是让 `columns * cell_size` 与既有玩法世界（约 256 单位）一致，
//!   这样相机、拾取、`StaticWorld`、玩法单位坐标全部无需改动。
//! - 阶段 1 用 6 种 Lordaeron Summer 地表调色板 + 程序化几何区分摆放；
//!   阶段 2 起从本机 `tileset_dir` / `minimap_image` 读取真实 BLP→PNG
//!   地表贴图与小地图底图；阶段 3 起从 `models_dir` / `slk_dir` 经
//!   [`crate::model_assembly`] 装配真实 glTF WC3 模型，把 2588 个静态摆放
//!   与 6 类动态单位替换为真实网格（程序化几何仅无资产 / 缺键时回退）。
//!   以上全部是纯表现层数据，不进权威状态 / 哈希 / 快照；逐帧骨骼动画与
//!   逐像素 GPU 纹理过滤留后续阶段。
//!
//! 路径来源：gitignored 的本机 `config.toml`（ADR 0001 / CODEBUDDY.md §6）。
//! 这里只做极简 `key = "value"` 解析（不引入 `toml` 解析 crate）；真实 glTF
//! 解析与 PNG 解码在 `w3-content` 内复用已有 `serde_json` / `image`，阶段 3
//! 零新增 Cargo 依赖。仓库不含任何导出资产。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use w3_content::{
    CenterOffset, DoodadPlacement, ModelCatalog, ParsedMap, RgbaImage, TerrainHeightfield,
    TextureArray, UnitPlacement, load_rgba, load_tileset,
};
use w3_render::{
    BakedModel, SceneVertex, StaticMarker, StaticShape, TerrainMesh, TerrainTextureLayers,
};

use crate::model_assembly::{AssembledModels, ModelStats, assemble_models};

/// 上游导出的瓦片边长（Echo Isles `tileSize=128`）。
const EXPORT_TILE_SIZE: f64 = 128.0;
/// 缩放到玩法世界后的瓦片边长：129 格 × 2 ≈ 258 世界单位，与合成世界对齐。
const WORLD_TILE_SIZE: f64 = 2.0;
/// 表现层坐标缩放比（只作用于 `f64` 冻结输入与 `f32` 呈现数据）。
pub(super) const WORLD_SCALE: f64 = WORLD_TILE_SIZE / EXPORT_TILE_SIZE;

/// 静态摆放基准尺寸（相对 258 的地图跨度）。
const MARKER_SIZE: f32 = 3.5;

/// 摆放语义色（阶段 1 用稳定色块，阶段 2 由真实贴图/模型替换）。
const TREE_COLOR: [f32; 4] = [0.16, 0.42, 0.18, 1.0];
const ROCK_COLOR: [f32; 4] = [0.48, 0.47, 0.44, 1.0];
const GOLD_COLOR: [f32; 4] = [0.92, 0.75, 0.16, 1.0];
const STONE_COLOR: [f32; 4] = [0.62, 0.58, 0.50, 1.0];
const START_COLOR: [f32; 4] = [0.20, 0.80, 0.90, 1.0];

/// 本机导出路径（来自 gitignored `config.toml`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealMapPaths {
    /// `assets/map-parsed/<slug>/` 目录。
    pub map_dir: PathBuf,
    /// 可选：`slk-exported/TerrainArt/Terrain.json`（阶段 2 贴图映射用）。
    pub tileset_table: Option<PathBuf>,
    /// 可选：`war3mapMap.png`（阶段 2 小地图底图用）。
    pub minimap_image: Option<PathBuf>,
    /// 可选：6 张 BLP→PNG 地表贴图目录（阶段 2 真实地表用）。
    pub tileset_dir: Option<PathBuf>,
    /// 可选：`asset-converted/` 目录（阶段 3 真实 glTF 模型用）。
    pub models_dir: Option<PathBuf>,
    /// 可选：`slk-exported/` 目录（阶段 3 模型映射表用）。
    pub slk_dir: Option<PathBuf>,
    /// 可选：真实水面贴图 PNG（阶段 4 真 GPU 纹理管线的水贴图层）。
    /// 未配置时 `water` 层置负、水顶点回退半透明纯色（不阻断装配）；
    /// 已配置但读取 / 解码失败 MUST 显式报 `RealMapError::Texture`，与其余
    /// 已配置资产口径一致，不静默回落。
    pub water_texture: Option<PathBuf>,
}

/// 装配错误；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealMapError {
    /// 缺 gitignored `config.toml` 或其中缺 `map_dir`。
    MissingConfig { path: PathBuf, key: &'static str },
    /// `config.toml` 无法读取。
    ConfigIo { path: PathBuf, message: String },
    /// 导出目录读取失败（缺失 / 维度或数组长度不符）。
    MapLoad { message: String },
    /// 缩放后的高度场无法生成地形网格。
    Terrain { message: String },
    /// 已配置的贴图资源读取 / 解码失败（不静默回落）。
    Texture { path: PathBuf, message: String },
    /// 已配置的模型目录 / 映射表读取失败（不静默回落）。
    Model { path: PathBuf, message: String },
}

impl std::fmt::Display for RealMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingConfig { path, key } => {
                write!(f, "missing {key} in {}", path.display())
            }
            Self::ConfigIo { path, message } => {
                write!(f, "cannot read {}: {message}", path.display())
            }
            Self::MapLoad { message } => write!(f, "cannot load exported map: {message}"),
            Self::Terrain { message } => write!(f, "cannot build terrain: {message}"),
            Self::Texture { path, message } => {
                write!(f, "cannot decode {}: {message}", path.display())
            }
            Self::Model { path, message } => {
                write!(f, "cannot load models at {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for RealMapError {}

/// 极简解析 gitignored `config.toml` 的 `key = "value"` 行，忽略注释与未知键。
/// 不引入 toml 依赖；只识别 map_dir / tileset_table / minimap_image /
/// tileset_dir / models_dir / slk_dir / water_texture 七个键。
pub fn load_paths(config_path: &Path) -> Result<RealMapPaths, RealMapError> {
    let text = std::fs::read_to_string(config_path).map_err(|error| RealMapError::ConfigIo {
        path: config_path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut map_dir = None;
    let mut tileset_table = None;
    let mut minimap_image = None;
    let mut tileset_dir = None;
    let mut models_dir = None;
    let mut slk_dir = None;
    let mut water_texture = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "map_dir" => map_dir = Some(PathBuf::from(value)),
            "tileset_table" => tileset_table = Some(PathBuf::from(value)),
            "minimap_image" => minimap_image = Some(PathBuf::from(value)),
            "tileset_dir" => tileset_dir = Some(PathBuf::from(value)),
            "models_dir" => models_dir = Some(PathBuf::from(value)),
            "slk_dir" => slk_dir = Some(PathBuf::from(value)),
            "water_texture" => water_texture = Some(PathBuf::from(value)),
            _ => {}
        }
    }
    let map_dir = map_dir.ok_or_else(|| RealMapError::MissingConfig {
        path: config_path.to_path_buf(),
        key: "map_dir",
    })?;
    Ok(RealMapPaths {
        map_dir,
        tileset_table,
        minimap_image,
        tileset_dir,
        models_dir,
        slk_dir,
        water_texture,
    })
}

/// 把导出高度场缩放到玩法世界尺度；网格拓扑与所有索引数组保持不变。
///
/// 只缩放几何量（`tileSize` / `centerOffset` / `heights` / `waterHeights`），
/// 地表 / 悬崖 / 层高 / 变体 / 标记位都是索引或层号，不能缩放。
fn scale_heightfield(source: &TerrainHeightfield) -> TerrainHeightfield {
    let mut scaled = source.clone();
    scaled.tile_size = Some(WORLD_TILE_SIZE);
    scaled.center_offset = scaled.center_offset.map(|offset| CenterOffset {
        x: offset.x * WORLD_SCALE,
        y: offset.y * WORLD_SCALE,
    });
    for height in &mut scaled.heights {
        *height *= WORLD_SCALE;
    }
    for water in &mut scaled.water_heights {
        *water *= WORLD_SCALE;
    }
    scaled
}

/// 导出单位摆放 → 静态标记（按 FourCC 语义分类）。
fn unit_marker(unit: &UnitPlacement) -> StaticMarker {
    let (shape, color, size) = match unit.type_id.as_str() {
        "ngol" => (StaticShape::GoldMine, GOLD_COLOR, MARKER_SIZE * 1.4),
        "sloc" => (StaticShape::StartLocation, START_COLOR, MARKER_SIZE * 1.2),
        // 中立建筑 / 野怪建筑（ntav 酒馆、ngme 地精商店等）。
        id if id.starts_with('n') => (StaticShape::Building, STONE_COLOR, MARKER_SIZE * 1.3),
        // 人族建筑（htow 主城、hbar 兵营等）。
        _ => (StaticShape::Building, STONE_COLOR, MARKER_SIZE * 1.3),
    };
    StaticMarker {
        x: (unit.position.x * WORLD_SCALE) as f32,
        y: (unit.position.y * WORLD_SCALE) as f32,
        size,
        color,
        shape,
    }
}

/// 导出装饰物摆放 → 静态标记；绝大多数是树木，岩石/杂物单独区分。
/// `variation` 做确定性的尺寸变化，不引入随机源。
fn doodad_marker(doodad: &DoodadPlacement) -> StaticMarker {
    let id = doodad.id.as_str();
    let is_rock = id.contains("rk") || id.contains("Rk") || id.contains("Rock");
    let (shape, color) = if is_rock {
        (StaticShape::Rock, ROCK_COLOR)
    } else {
        (StaticShape::Tree, TREE_COLOR)
    };
    let variation = doodad.variation.rem_euclid(3) as f32;
    StaticMarker {
        x: (doodad.position.x * WORLD_SCALE) as f32,
        y: (doodad.position.y * WORLD_SCALE) as f32,
        size: MARKER_SIZE * (0.8 + variation * 0.2),
        color,
        shape,
    }
}

/// 从 `ParsedMap` 装配真实地图（纯函数，便于 CPU 自测）。
pub fn assemble_from(parsed: &ParsedMap) -> Result<RealMap, RealMapError> {
    let scaled = scale_heightfield(&parsed.heightfield);
    let terrain =
        TerrainMesh::from_heightfield(&scaled).map_err(|error| RealMapError::Terrain {
            message: format!("{error:?}"),
        })?;

    let mut markers = Vec::with_capacity(parsed.units.len() + parsed.doodads.len());
    for unit in &parsed.units {
        markers.push(unit_marker(unit));
    }
    for doodad in &parsed.doodads {
        markers.push(doodad_marker(doodad));
    }
    Ok(RealMap {
        terrain,
        markers,
        terrain_tiles: None,
        minimap_image: None,
        static_model_vertices: Vec::new(),
        entity_models: BTreeMap::new(),
        textures: TextureArray {
            size: w3_content::TEXTURE_LAYER_SIZE,
            layer_count: 0,
            pixels: Vec::new(),
        },
        terrain_texture_layers: None,
        model_stats: ModelStats::default(),
    })
}

/// 装配产物：真实地形网格 + 静态摆放 + 真实模型（全部为表现层数据）。
#[derive(Clone, Debug)]
pub struct RealMap {
    pub terrain: TerrainMesh,
    pub markers: Vec<StaticMarker>,
    /// 真实地表 tileset 贴图的 CPU 副本（6 层 RGBA8）；`None` 回退调色板。
    /// 纯表现层数据，MUST NOT 进入权威状态 / 状态哈希 / 快照。
    pub terrain_tiles: Option<Vec<RgbaImage>>,
    /// 真实 `war3mapMap.png` 小地图底图（RGBA8）；`None` 用纯色底。
    pub minimap_image: Option<RgbaImage>,
    /// 静态摆放的真实 glTF 模型（预展开顶点）；空则回退程序化几何。
    pub static_model_vertices: Vec<SceneVertex>,
    /// 动态实体类型键 → 真实烘焙模型。
    pub entity_models: BTreeMap<String, BakedModel>,
    /// 上传 GPU 的统一贴图数组（`texture_2d_array`）；空数组 = 顶点色路径。
    pub textures: TextureArray,
    /// 地形 GPU 平铺贴图层索引；`None` 时地形回退 CPU 顶点色。
    /// 装配层把 6 张 tileset + 1 张水贴图追加到 `textures` 后生成，供
    /// `Scene::set_terrain_texture_layers` 注入，纯表现层数据。
    pub terrain_texture_layers: Option<TerrainTextureLayers>,
    /// 模型装配统计（只读诊断）。
    pub model_stats: ModelStats,
}

/// 从本机 `config.toml` 指向的导出目录装配真实地图。
///
/// 缺配置或导出数据无效时显式返回错误，MUST NOT 静默回落到合成 demo。
pub fn assemble(config_path: &Path) -> Result<(RealMap, RealMapPaths), RealMapError> {
    let paths = load_paths(config_path)?;
    let parsed = w3_content::load_map(&paths.map_dir).map_err(|error| RealMapError::MapLoad {
        message: error.to_string(),
    })?;
    let mut map = assemble_from(&parsed)?;
    // 贴图是可选的本机资产：未配置时优雅回退到调色板（CI / 未跑 convert 的
    // 环境）；已配置但读取 / 解码失败 MUST 显式报错，不静默回落成色块 demo。
    // 两者都是纯表现层数据，不进权威状态 / 状态哈希 / 快照（ADR 0002）。
    if let Some(dir) = &paths.tileset_dir {
        map.terrain_tiles = Some(load_tileset(dir).map_err(|error| RealMapError::Texture {
            path: dir.clone(),
            message: error.to_string(),
        })?);
    }
    if let Some(image) = &paths.minimap_image {
        map.minimap_image = Some(load_rgba(image).map_err(|error| RealMapError::Texture {
            path: image.clone(),
            message: error.to_string(),
        })?);
    }
    // 阶段 3：真实 glTF 模型。models_dir / slk_dir 未配置时优雅降级到程序化
    // 几何（CI / 未转换模型）；已配置但目录读取失败 MUST 显式报错。
    if let (Some(models_dir), Some(slk_dir)) = (&paths.models_dir, &paths.slk_dir) {
        let catalog =
            ModelCatalog::load(models_dir, slk_dir).map_err(|error| RealMapError::Model {
                path: models_dir.clone(),
                message: error.to_string(),
            })?;
        let assembled: AssembledModels = assemble_models(&parsed, &map.terrain, catalog);
        map.static_model_vertices = assembled.static_vertices;
        map.entity_models = assembled.entity_models;
        map.textures = assembled.textures;
        map.model_stats = assembled.stats;
    }
    // 阶段 4（真 GPU 纹理管线）：把地表 tileset 与水面贴图追加到统一
    // `texture_2d_array`。模型顶点已引用 `textures` 的前 N 层，`push_image`
    // 从 `layer_count` 起追加，层索引不冲突。地形据此走 GPU 逐像素平铺采样，
    // 消除 CPU 顶点色的纯色块 / 浅蓝平板。缺 tileset 时不设置 layers，地形
    // 优雅回退 CPU 顶点色（CI / 无本机资产）。
    if let Some(tiles) = &map.terrain_tiles {
        let mut ground = [-1.0f32; 6];
        for (index, image) in tiles.iter().take(6).enumerate() {
            ground[index] = map.textures.push_image(image) as f32;
        }
        let water = match &paths.water_texture {
            Some(path) => {
                let image = load_rgba(path).map_err(|error| RealMapError::Texture {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
                map.textures.push_image(&image) as f32
            }
            // 未配置水贴图：water 置负，水顶点回退半透明纯色，不阻断装配。
            None => -1.0,
        };
        map.terrain_texture_layers = Some(TerrainTextureLayers { ground, water });
    }
    Ok((map, paths))
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_content::{DoodadsFile, Position3, UnitsFile};

    /// 构造一份模拟真实导出 schema 的合成 fixture（不读本机资产）。
    fn fixture() -> ParsedMap {
        let width = 5usize;
        let height = 3usize;
        let count = width * height;
        ParsedMap {
            heightfield: TerrainHeightfield {
                tilepoint_width: width,
                tilepoint_height: height,
                map_width: width - 1,
                map_height: height - 1,
                center_offset: Some(CenterOffset {
                    x: -8192.0,
                    y: -6144.0,
                }),
                main_tileset: Some("L".to_string()),
                main_tileset_name: Some("Lordaeron Summer".to_string()),
                ground_tilesets: vec![
                    "Ldrt".into(),
                    "Ldro".into(),
                    "Ldrg".into(),
                    "Lrok".into(),
                    "Lgrs".into(),
                    "Lgrd".into(),
                ],
                cliff_tilesets: vec!["CLdi".into(), "CLgr".into()],
                tile_size: Some(EXPORT_TILE_SIZE),
                heights: vec![226.0; count],
                ground_textures: (0..count).map(|i| (i % 6) as i64).collect(),
                ground_variations: vec![0; count],
                cliff_textures: vec![15; count],
                cliff_variations: vec![0; count],
                layer_heights: vec![4; count],
                water_heights: vec![384.0; count],
                flags_packed: vec![17; count],
            },
            units: UnitsFile {
                format_version: 1,
                subversion: 0,
                count: 3,
                units: vec![
                    UnitPlacement {
                        type_id: "ngol".to_string(),
                        variation: 0,
                        position: Position3 {
                            x: -5632.0,
                            y: 3584.0,
                            z: 574.25,
                        },
                        angle: 0.0,
                        owner: 15,
                        flags: 2,
                        creation_number: 0,
                    },
                    UnitPlacement {
                        type_id: "sloc".to_string(),
                        variation: 0,
                        position: Position3 {
                            x: 4096.0,
                            y: -2048.0,
                            z: 512.0,
                        },
                        angle: 0.0,
                        owner: 0,
                        flags: 2,
                        creation_number: 1,
                    },
                    UnitPlacement {
                        type_id: "ntav".to_string(),
                        variation: 0,
                        position: Position3 {
                            x: 0.0,
                            y: 0.0,
                            z: 512.0,
                        },
                        angle: 0.0,
                        owner: 15,
                        flags: 2,
                        creation_number: 2,
                    },
                ],
            }
            .units,
            doodads: DoodadsFile {
                format_version: 1,
                subversion: 0,
                count: 2,
                doodads: vec![
                    DoodadPlacement {
                        id: "LTlt".to_string(),
                        variation: 5,
                        position: Position3 {
                            x: -5824.0,
                            y: 4416.0,
                            z: 538.0,
                        },
                        angle: 0.0,
                        flags: 2,
                        life: 100,
                        creation_number: 0,
                    },
                    DoodadPlacement {
                        id: "LRrk".to_string(),
                        variation: 0,
                        position: Position3 {
                            x: 1024.0,
                            y: 1024.0,
                            z: 512.0,
                        },
                        angle: 0.0,
                        flags: 2,
                        life: 100,
                        creation_number: 1,
                    },
                ],
            }
            .doodads,
        }
    }

    #[test]
    fn assemble_preserves_topology_and_scales_geometry() {
        let map = assemble_from(&fixture()).expect("fixture assembles");
        assert_eq!(map.terrain.vertices.len(), 15);
        assert_eq!(map.markers.len(), 5);
        // 缩放后瓦片边长与玩法世界对齐。
        assert!((map.terrain.cell_size - WORLD_TILE_SIZE as f32).abs() < 1.0e-6);
        // 高度按缩放比变换，索引数组保持原值。
        let expected = (226.0 * WORLD_SCALE) as f32;
        assert!((map.terrain.vertices[0].position[1] - expected).abs() < 1.0e-3);
        assert_eq!(map.terrain.vertices[0].surface, 0.0);
        // 静态摆放坐标同步缩放，且分类正确。
        assert!(map.markers.iter().any(|m| m.shape == StaticShape::GoldMine));
        assert!(
            map.markers
                .iter()
                .any(|m| m.shape == StaticShape::StartLocation)
        );
        assert!(map.markers.iter().any(|m| m.shape == StaticShape::Building));
        assert!(map.markers.iter().any(|m| m.shape == StaticShape::Tree));
        assert!(map.markers.iter().any(|m| m.shape == StaticShape::Rock));
        let gold = map
            .markers
            .iter()
            .find(|m| m.shape == StaticShape::GoldMine)
            .unwrap();
        assert!((gold.x - (-5632.0 * WORLD_SCALE) as f32).abs() < 1.0e-3);
    }

    #[test]
    fn assemble_is_deterministic() {
        let a = assemble_from(&fixture()).unwrap();
        let b = assemble_from(&fixture()).unwrap();
        assert_eq!(a.terrain.vertices, b.terrain.vertices);
        assert_eq!(a.markers, b.markers);
    }

    #[test]
    fn load_paths_parses_keys_and_skips_comments() {
        let dir = std::env::temp_dir().join("w3_real_map_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "# local export paths\nmap_dir = \"C:/maps/echoisles\"\ntileset_table = 'C:/slk/Terrain.json'\ntileset_dir = 'C:/converted/LordaeronSummer'\nwater_texture = 'C:/converted/Water/A_Water00.png'\nmodels_dir = 'C:/converted'\nslk_dir = 'C:/slk'\nunknown = 1\n",
        )
        .unwrap();
        let paths = load_paths(&path).unwrap();
        assert_eq!(paths.map_dir, PathBuf::from("C:/maps/echoisles"));
        assert_eq!(
            paths.tileset_table,
            Some(PathBuf::from("C:/slk/Terrain.json"))
        );
        assert_eq!(
            paths.tileset_dir,
            Some(PathBuf::from("C:/converted/LordaeronSummer"))
        );
        assert_eq!(
            paths.water_texture,
            Some(PathBuf::from("C:/converted/Water/A_Water00.png"))
        );
        assert_eq!(paths.models_dir, Some(PathBuf::from("C:/converted")));
        assert_eq!(paths.slk_dir, Some(PathBuf::from("C:/slk")));
        assert_eq!(paths.minimap_image, None);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn load_paths_reports_missing_key() {
        let dir = std::env::temp_dir().join("w3_real_map_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.toml");
        std::fs::write(&path, "# no map_dir\n").unwrap();
        assert_eq!(
            load_paths(&path),
            Err(RealMapError::MissingConfig {
                path: path.clone(),
                key: "map_dir",
            })
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn assemble_from_has_no_embedded_textures() {
        // 纯装配函数不读任何文件；贴图只由 `assemble` 按配置注入。
        // 无本机资产的 CI 走 6 色调色板回退，字段契约必须稳定。
        let map = assemble_from(&fixture()).unwrap();
        assert!(map.terrain_tiles.is_none());
        assert!(map.minimap_image.is_none());
        assert!(map.static_model_vertices.is_empty());
        assert!(map.entity_models.is_empty());
        assert!(map.terrain_texture_layers.is_none());
    }
}
