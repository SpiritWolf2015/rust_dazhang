//! 本机导出模型目录索引与「摆放 id → glTF」映射（真实模型视觉阶段）。
//!
//! 上游 `asset-convert` 把 WC3 MDX 转成 922 个 glTF；SLK 表
//! `Units/unitUI.json` / `Doodads/Doodads.json` 的 `file` 字段给出每个单位 /
//! 装饰物的模型逻辑路径。本模块把两者拼成可查询的 [`ModelCatalog`]，供表现层
//! 装配 [`crate::gltf::load_gltf`] 读取真实网格。
//!
//! 三类现实约束（只读核实 EI 地图，2026-11-12）：
//! - 单位 `typeId` 基本都能经 `file` 精确命中；`sloc`（起始点）无模型。
//! - 装饰物 `file` 大小写 / `numVar` 变体后缀与 glTF 路径不完全一致，需
//!   大小写不敏感 + 变体前缀匹配。
//! - 占 EI 地图约 64% 的树（`LTlt` 等）在 SLK `file` 为空，属 tileset 树，
//!   实际模型在 `Doodads/Terrain/LordaeronTree/LordaeronTree{N}.gltf`；
//!   少数其它 tileset 装饰物未在本机 convert 子集内产出。这两类由
//!   [`ModelClass`] 语义兜底到**同类真实 glTF**，绝不回退到程序化几何。
//!
//! 本模块只读本机 `config.toml` 指向的目录，仓库零资产（ADR 0001）；索引与
//! 映射是纯表现层派生数据，MUST NOT 进入权威状态 / 状态哈希 / 快照。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 摆放语义类别，用于 `file` 缺失时兜底到同类真实模型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelClass {
    Tree,
    Rock,
    Plant,
    Water,
    Structure,
    Unit,
    Generic,
}

/// 索引错误；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelIndexError {
    /// 目录读取失败。
    Io { path: PathBuf, message: String },
    /// SLK 表 JSON 非法。
    Json { path: PathBuf, message: String },
}

impl std::fmt::Display for ModelIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "cannot read {}: {message}", path.display()),
            Self::Json { path, message } => {
                write!(f, "cannot parse {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ModelIndexError {}

/// 动态实体类型键 → 真实模型逻辑路径（合成冻结定义对应 WC3 单位/建筑）。
pub const ENTITY_MODEL_FILES: &[(&str, &str)] = &[
    ("peasant", "units/Human/Peasant/Peasant"),
    ("soldier", "units/Human/Footman/Footman"),
    ("town-hall", "buildings/Human/TownHall/TownHall"),
    ("barracks", "buildings/Human/HumanBarracks/HumanBarracks"),
    ("gold-mine", "buildings/Other/GoldMine/GoldMine"),
    ("tree", "Doodads/Terrain/LordaeronTree/LordaeronTree"),
];

/// 按类别兜底的真实模型逻辑路径候选（确定性取用，均为本机 convert 产物）。
const FALLBACK_FILES: &[(ModelClass, &[&str])] = &[
    (
        ModelClass::Tree,
        &[
            "Doodads/Terrain/LordaeronTree/LordaeronTree",
            "doodads/LordaeronSummer/Plants/Wheat/Wheat",
        ],
    ),
    (
        ModelClass::Plant,
        &[
            "doodads/LordaeronSummer/Plants/GrassTuft/GrassTuft",
            "doodads/LordaeronSummer/Plants/Corn/Corn",
            "doodads/LordaeronSummer/Plants/RiverRushes/RiverRushes",
        ],
    ),
    (
        ModelClass::Rock,
        &["doodads/LordaeronSummer/Rocks/Lords_Rock/Lords_Rock"],
    ),
    (
        ModelClass::Water,
        &[
            // `Water/Shoreline/Shoreline0.gltf` 是上游空占位（0 mesh），
            // 显式排除；改用有真实网格的水体 / 睡莲模型。
            "doodads/LordaeronSummer/Water/ShorelineTide/ShorelineTide",
            "doodads/LordaeronSummer/Water/ShorelineWave/ShorelineWave",
            "doodads/LordaeronSummer/Plants/LilyPad/LilyPad",
        ],
    ),
    (
        ModelClass::Structure,
        &["doodads/LordaeronSummer/Structures/Well/Well"],
    ),
    (ModelClass::Unit, &["units/Human/Footman/Footman"]),
    (
        ModelClass::Generic,
        &["doodads/LordaeronSummer/Props/Brazier/Brazier"],
    ),
];

fn normalize(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn read_id_file_map(
    path: &Path,
    id_key: &str,
) -> Result<BTreeMap<String, String>, ModelIndexError> {
    let text = std::fs::read_to_string(path).map_err(|error| ModelIndexError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| ModelIndexError::Json {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    let records = value.get("records");
    let mut out = BTreeMap::new();
    // records 兼容数组或对象（key → row）两种导出形式。
    let rows: Vec<&serde_json::Value> = match records {
        Some(serde_json::Value::Array(list)) => list.iter().collect(),
        Some(serde_json::Value::Object(map)) => map.values().collect(),
        _ => Vec::new(),
    };
    for row in rows {
        let id = row.get(id_key).and_then(|v| v.as_str());
        let file = row.get("file").and_then(|v| v.as_str());
        if let (Some(id), Some(file)) = (id, file) {
            if !file.is_empty() {
                out.insert(id.to_string(), file.to_string());
            }
        }
    }
    Ok(out)
}

/// 模型目录索引：`逻辑路径(小写) → gltf 绝对路径` + SLK `id → file` 映射。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelCatalog {
    /// 去扩展名的小写相对路径 → 绝对路径。
    by_path: BTreeMap<String, PathBuf>,
    /// 逻辑路径前缀（小写）→ 该前缀下所有 gltf 绝对路径（变体池，已排序）。
    variants: BTreeMap<String, Vec<PathBuf>>,
    doodad_files: BTreeMap<String, String>,
    unit_files: BTreeMap<String, String>,
}

impl ModelCatalog {
    /// 从 `asset-converted/` 与 `slk-exported/` 构建索引；纯读取、可复现。
    pub fn load(converted_root: &Path, slk_root: &Path) -> Result<Self, ModelIndexError> {
        let mut by_path = BTreeMap::new();
        let mut variants: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        scan_gltf(converted_root, converted_root, &mut by_path, &mut variants)?;

        let doodad_files = read_id_file_map(&slk_root.join("Doodads/Doodads.json"), "doodID")?;
        let unit_files = read_id_file_map(&slk_root.join("Units/unitUI.json"), "unitUIID")?;
        for list in variants.values_mut() {
            list.sort();
            list.dedup();
        }
        Ok(Self {
            by_path,
            variants,
            doodad_files,
            unit_files,
        })
    }

    /// 索引到的 glTF 总数。
    pub fn len(&self) -> usize {
        self.by_path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// 精确逻辑路径查找（大小写不敏感）；找不到时尝试 `file + 数字变体` 前缀池。
    fn lookup(&self, file: &str, variation: u32) -> Option<PathBuf> {
        let key = normalize(file);
        if let Some(path) = self.by_path.get(&key) {
            return Some(path.clone());
        }
        // 变体：把 `.../Foo` 当作前缀，从 `.../Foo0`、`.../Foo1`… 中按变体取。
        let pool = self.variants.get(&key)?;
        if pool.is_empty() {
            return None;
        }
        Some(pool[(variation as usize) % pool.len()].clone())
    }

    /// 单位 `typeId` → glTF（经 SLK `file` 精确匹配）。
    pub fn resolve_unit(&self, type_id: &str) -> Option<PathBuf> {
        let file = self.unit_files.get(type_id)?;
        self.lookup(file, 0)
    }

    /// 动态实体类型键（`"peasant"` 等）→ glTF。
    pub fn resolve_entity_key(&self, key: &str) -> Option<PathBuf> {
        let file = ENTITY_MODEL_FILES
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, f)| *f)?;
        self.lookup(file, 0)
    }

    /// 装饰物 `id` + 变体 → 有序候选 glTF 列表（精确 `file` 优先，其次同类
    /// 兜底候选）。装配层依次尝试，可跳过上游空占位 / 损坏模型。确定性、去重。
    pub fn resolve_doodad_paths(
        &self,
        id: &str,
        variation: u32,
        class: ModelClass,
    ) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        let push = |path: PathBuf, out: &mut Vec<PathBuf>| {
            if !out.contains(&path) {
                out.push(path);
            }
        };
        if let Some(file) = self.doodad_files.get(id) {
            if let Some(path) = self.lookup(file, variation) {
                push(path, &mut out);
            }
        }
        let fallback = FALLBACK_FILES
            .iter()
            .find(|(c, _)| *c == class)
            .map(|(_, f)| *f)
            .unwrap_or(&[]);
        let generic = FALLBACK_FILES
            .iter()
            .find(|(c, _)| *c == ModelClass::Generic)
            .map(|(_, f)| *f)
            .unwrap_or(&[]);
        for file in fallback.iter().chain(generic.iter()) {
            if let Some(path) = self.lookup(file, variation) {
                push(path, &mut out);
            }
        }
        out
    }

    /// 装饰物 `id` + 变体 → 单个 glTF（取首个候选）；保留既有便捷入口。
    pub fn resolve_doodad(&self, id: &str, variation: u32, class: ModelClass) -> Option<PathBuf> {
        self.resolve_doodad_paths(id, variation, class)
            .into_iter()
            .next()
    }
}

fn scan_gltf(
    root: &Path,
    dir: &Path,
    by_path: &mut BTreeMap<String, PathBuf>,
    variants: &mut BTreeMap<String, Vec<PathBuf>>,
) -> Result<(), ModelIndexError> {
    let entries = std::fs::read_dir(dir).map_err(|error| ModelIndexError::Io {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ModelIndexError::Io {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            scan_gltf(root, &path, by_path, variants)?;
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("gltf") {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let rel_str = relative.to_string_lossy();
        let stem = rel_str
            .strip_suffix(".gltf")
            .or_else(|| rel_str.strip_suffix(".GLTF"))
            .unwrap_or(&rel_str);
        let key = normalize(stem);
        // 变体前缀：去掉末尾数字，归入同一池（LordaeronTree0..N、GrassTuft0..N）。
        let prefix = key
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string();
        variants.entry(prefix).or_default().push(path.clone());
        by_path.insert(key, path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造合成目录：几个逻辑路径 + 变体 + 合成 SLK，不依赖本机资产。
    fn fixture(base: &Path) -> (PathBuf, PathBuf) {
        let conv = base.join("converted");
        let slk = base.join("slk");
        for rel in [
            "units/Human/Footman/Footman.gltf",
            "units/Human/Peasant/Peasant.gltf",
            "Doodads/Terrain/LordaeronTree/LordaeronTree0.gltf",
            "Doodads/Terrain/LordaeronTree/LordaeronTree1.gltf",
            "doodads/LordaeronSummer/Rocks/Lords_Rock/Lords_Rock0.gltf",
            "doodads/LordaeronSummer/Rocks/Lords_Rock/Lords_Rock1.gltf",
        ] {
            let p = conv.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"{}").unwrap();
        }
        std::fs::create_dir_all(slk.join("Units")).unwrap();
        std::fs::create_dir_all(slk.join("Doodads")).unwrap();
        std::fs::write(
            slk.join("Units/unitUI.json"),
            r#"{"records":[{"unitUIID":"hfoo","file":"units\\human\\Footman\\Footman"}]}"#,
        )
        .unwrap();
        std::fs::write(
            slk.join("Doodads/Doodads.json"),
            r#"{"records":[
                {"doodID":"LRrk","file":"Doodads\\LordaeronSummer\\Rocks\\Lords_Rock\\Lords_Rock"},
                {"doodID":"LTlt","file":""}
            ]}"#,
        )
        .unwrap();
        (conv, slk)
    }

    #[test]
    fn catalog_indexes_real_layout_and_resolves_units() {
        let dir = std::env::temp_dir().join("w3_model_index_test_a");
        let _ = std::fs::remove_dir_all(&dir);
        let (conv, slk) = fixture(&dir);
        let catalog = ModelCatalog::load(&conv, &slk).unwrap();
        assert_eq!(catalog.len(), 6);
        let footman = catalog.resolve_unit("hfoo").expect("unit resolves");
        assert!(footman.ends_with("Footman/Footman.gltf"));
        assert!(catalog.resolve_unit("sloc").is_none());
        assert!(catalog.resolve_entity_key("peasant").is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn case_insensitive_and_variant_lookup() {
        let dir = std::env::temp_dir().join("w3_model_index_test_b");
        let _ = std::fs::remove_dir_all(&dir);
        let (conv, slk) = fixture(&dir);
        let catalog = ModelCatalog::load(&conv, &slk).unwrap();
        // LRrk 的 file 指向 Lords_Rock，实际产物是 Lords_Rock0/1 变体 → 前缀池命中。
        let rock = catalog
            .resolve_doodad("LRrk", 0, ModelClass::Rock)
            .expect("rock variant resolves");
        assert!(rock.to_string_lossy().contains("Lords_Rock"));
        let rock1 = catalog
            .resolve_doodad("LRrk", 1, ModelClass::Rock)
            .expect("rock variant 1");
        assert!(rock1.to_string_lossy().contains("Lords_Rock"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tree_falls_back_to_real_tree_model() {
        let dir = std::env::temp_dir().join("w3_model_index_test_c");
        let _ = std::fs::remove_dir_all(&dir);
        let (conv, slk) = fixture(&dir);
        let catalog = ModelCatalog::load(&conv, &slk).unwrap();
        // LTlt 的 file 为空 → 按 Tree 类别兜底到 LordaeronTree 变体（真实树模型）。
        let tree = catalog
            .resolve_doodad("LTlt", 3, ModelClass::Tree)
            .expect("tree fallback resolves");
        assert!(tree.to_string_lossy().contains("LordaeronTree"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lookup_is_deterministic_and_bounded() {
        let dir = std::env::temp_dir().join("w3_model_index_test_d");
        let _ = std::fs::remove_dir_all(&dir);
        let (conv, slk) = fixture(&dir);
        let a = ModelCatalog::load(&conv, &slk).unwrap();
        let b = ModelCatalog::load(&conv, &slk).unwrap();
        assert_eq!(a, b, "索引扫描必须可复现、稳定排序");
        for variation in 0..64u32 {
            let Some(path) = a.resolve_doodad("LRrk", variation, ModelClass::Rock) else {
                continue;
            };
            assert!(path.exists());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_doodad_paths_returns_deduped_candidates() {
        let dir = std::env::temp_dir().join("w3_model_index_test_e");
        let _ = std::fs::remove_dir_all(&dir);
        let (conv, slk) = fixture(&dir);
        let catalog = ModelCatalog::load(&conv, &slk).unwrap();
        // 树 LTlt（file 空）应返回兜底候选列表（LordaeronTree 变体 + Generic），
        // 供装配层在首候选为空占位 / 损坏时依次尝试。
        let paths = catalog.resolve_doodad_paths("LTlt", 0, ModelClass::Tree);
        assert!(!paths.is_empty(), "树应有多个兜底候选");
        // 候选去重。
        let mut dedup = paths.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(paths.len(), dedup.len(), "候选列表不应重复");
        assert!(paths.iter().all(|p| p.exists()), "候选均指向真实文件");
        // 与单候选入口一致、确定性。
        assert_eq!(
            paths[0],
            catalog.resolve_doodad("LTlt", 0, ModelClass::Tree).unwrap()
        );
        assert_eq!(
            paths,
            catalog.resolve_doodad_paths("LTlt", 0, ModelClass::Tree)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_dir_reports_error() {
        let missing = Path::new("C:/definitely-not-a-real-converted-dir");
        assert!(matches!(
            ModelCatalog::load(missing, missing),
            Err(ModelIndexError::Io { .. })
        ));
    }
}
