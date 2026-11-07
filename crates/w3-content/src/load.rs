//! 文件边界加载：把本机导出的 JSON 读入 `ParsedMap`。
//!
//! `terrain-heightfield.json` 为必需；`units.json` 与 `doodads.json` 可选
//! （地图可以合法地没有摆放物）。本模块不写文件，也不访问模拟 crate。

use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::heightfield::{HeightfieldError, TerrainHeightfield};
use crate::placements::{DoodadPlacement, DoodadsFile, PlacementError, UnitPlacement, UnitsFile};
use crate::table::{SlkTable, TableError};

#[derive(Debug)]
pub enum ContentError {
    Io { path: PathBuf, message: String },
    Json { path: PathBuf, message: String },
    Heightfield(HeightfieldError),
    Placements(PlacementError),
    Table(TableError),
    MissingHeightfield(PathBuf),
    MissingTable(PathBuf),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(f, "cannot read {}: {message}", path.display())
            }
            Self::Json { path, message } => {
                write!(f, "invalid JSON in {}: {message}", path.display())
            }
            Self::Heightfield(error) => write!(f, "invalid heightfield: {error:?}"),
            Self::Placements(error) => write!(f, "invalid placements: {error:?}"),
            Self::Table(error) => write!(f, "invalid table: {error:?}"),
            Self::MissingHeightfield(path) => {
                write!(f, "missing terrain-heightfield.json in {}", path.display())
            }
            Self::MissingTable(path) => write!(f, "missing table file {}", path.display()),
        }
    }
}

impl std::error::Error for ContentError {}

#[derive(Clone, Debug)]
pub struct ParsedMap {
    pub heightfield: TerrainHeightfield,
    pub units: Vec<UnitPlacement>,
    pub doodads: Vec<DoodadPlacement>,
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, ContentError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| ContentError::Json {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ContentError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
    }
}

/// 加载一个已解析的地图目录（`assets/map-parsed/<slug>/`）。
pub fn load_map(dir: &Path) -> Result<ParsedMap, ContentError> {
    let heightfield_path = dir.join("terrain-heightfield.json");
    let heightfield: TerrainHeightfield = read_json(&heightfield_path)?
        .ok_or_else(|| ContentError::MissingHeightfield(heightfield_path.clone()))?;
    heightfield.validate().map_err(ContentError::Heightfield)?;

    let units = match read_json::<UnitsFile>(&dir.join("units.json"))? {
        Some(file) => {
            file.validate().map_err(ContentError::Placements)?;
            file.units
        }
        None => Vec::new(),
    };

    let doodads = match read_json::<DoodadsFile>(&dir.join("doodads.json"))? {
        Some(file) => {
            file.validate().map_err(ContentError::Placements)?;
            file.doodads
        }
        None => Vec::new(),
    };

    Ok(ParsedMap {
        heightfield,
        units,
        doodads,
    })
}

/// 加载一张已导出的 SLK 表 JSON。
pub fn load_table(path: &Path) -> Result<SlkTable, ContentError> {
    let table: SlkTable =
        read_json(path)?.ok_or_else(|| ContentError::MissingTable(path.to_path_buf()))?;
    table.validate().map_err(ContentError::Table)?;
    Ok(table)
}
