//! 本机导出 PNG 贴图的解码（真实地图视觉阶段）。
//!
//! 上游 `asset-convert` 把 BLP 转成 RGBA8 PNG；本模块只负责把这些 PNG
//! 读成中立的 `RgbaImage`，**不含任何游戏资产**、不复制任何上游源码。
//! 路径来自 gitignored 的本机 `config.toml`（ADR 0001），仓库零资产。
//!
//! 地表 tileset 顺序与 `TerrainHeightfield.ground_tilesets`
//! （`Ldrt` / `Ldro` / `Ldrg` / `Lrok` / `Lgrs` / `Lgrd`）一一对应；
//! 6 张贴图尺寸一致，便于 GPU 侧组装成 `texture_2d_array`。

use std::path::{Path, PathBuf};

/// 解码后的 RGBA8 图像；纯数据，不含文件句柄。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// 长度 = width × height × 4，行优先。
    pub pixels: Vec<u8>,
}

/// 贴图读取错误；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextureError {
    /// 文件缺失或无法读取 / 解码。
    Load { path: PathBuf, message: String },
    /// 6 张 tileset 贴图尺寸不一致，无法组装数组纹理。
    InconsistentTilesetSize {
        index: usize,
        width: u32,
        height: u32,
        expected: (u32, u32),
    },
    /// 贴图数量与 `groundTilesets` 数量不符。
    TilesetCount { found: usize, expected: usize },
}

impl std::fmt::Display for TextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load { path, message } => {
                write!(f, "cannot decode {}: {message}", path.display())
            }
            Self::InconsistentTilesetSize {
                index,
                width,
                height,
                expected,
            } => write!(f, "tileset[{index}] size {width}x{height} != {expected:?}"),
            Self::TilesetCount { found, expected } => {
                write!(f, "tileset count {found} != {expected}")
            }
        }
    }
}

impl std::error::Error for TextureError {}

/// 解码一张 PNG 为 RGBA8；显式失败，不静默回落。
pub fn load_rgba(path: &Path) -> Result<RgbaImage, TextureError> {
    let image = image::open(path).map_err(|error| TextureError::Load {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let rgba = image.to_rgba8();
    Ok(RgbaImage {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

/// 6 张 Lordaeron Summer 地表贴图的文件名，与 `groundTilesets` 顺序对应。
/// 顺序来自上游 `TerrainArt/Terrain.json` 的 `Ldrt..Lgrd` 记录。
pub const LORDAERON_SUMMER_FILES: [&str; 6] = [
    "Lords_Dirt.png",
    "Lords_DirtRough.png",
    "Lords_DirtGrass.png",
    "Lords_Rock.png",
    "Lords_Grass.png",
    "Lords_GrassDark.png",
];

/// 读取 6 张 tileset PNG；校验数量与尺寸一致，供 GPU 组装数组纹理。
pub fn load_tileset(dir: &Path) -> Result<Vec<RgbaImage>, TextureError> {
    let mut images = Vec::with_capacity(LORDAERON_SUMMER_FILES.len());
    for name in LORDAERON_SUMMER_FILES {
        images.push(load_rgba(&dir.join(name))?);
    }
    if images.len() != LORDAERON_SUMMER_FILES.len() {
        return Err(TextureError::TilesetCount {
            found: images.len(),
            expected: LORDAERON_SUMMER_FILES.len(),
        });
    }
    let expected = (images[0].width, images[0].height);
    for (index, image) in images.iter().enumerate() {
        if (image.width, image.height) != expected {
            return Err(TextureError::InconsistentTilesetSize {
                index,
                width: image.width,
                height: image.height,
                expected,
            });
        }
    }
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rgba_reports_missing_file_explicitly() {
        let path = Path::new("C:/definitely-not-a-real-tileset-dir/missing.png");
        assert!(matches!(load_rgba(path), Err(TextureError::Load { .. })));
    }

    #[test]
    fn tileset_filenames_are_six_unique() {
        assert_eq!(LORDAERON_SUMMER_FILES.len(), 6);
        let unique: std::collections::BTreeSet<&str> =
            LORDAERON_SUMMER_FILES.iter().copied().collect();
        assert_eq!(unique.len(), 6);
    }
}
