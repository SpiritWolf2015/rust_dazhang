//! 统一贴图数组（`texture_2d_array` 的 CPU 侧数据，真 GPU 纹理管线用）。
//!
//! WC3 模型贴图尺寸各异（64×64 ~ 512×512）、地表 tileset 是 512×256。
//! GPU 侧用 `texture_2d_array` 要求所有层同尺寸，故本模块把每张贴图**等比
//! resize 到统一正方形**（最近邻，确定性），按 URI 去重后堆叠成层，供
//! `w3-render` 一次性上传为数组纹理。
//!
//! 全部是纯 CPU 派生数据；贴图内容来自玩家本机合法导出的经典客户端，
//! 仓库零资产（ADR 0001）。不进权威状态 / 哈希 / 快照。

use std::collections::BTreeMap;
use std::path::Path;

use crate::textures::{RgbaImage, load_rgba};

/// 每层统一边长。256 足够呈现 WC3 低模贴图细节，且 251 层 × 256²×4 ≈ 65 MB，
/// 在常见 GPU 上传预算内。
pub const TEXTURE_LAYER_SIZE: u32 = 256;
/// 数组纹理层数上限（防御异常导出导致显存爆炸；纯表现层参数）。
pub const MAX_TEXTURE_LAYERS: u32 = 512;

/// 上传 GPU 的数组纹理数据：逐层拼接的 RGBA8，`layer × (size²×4)`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureArray {
    pub size: u32,
    pub layer_count: u32,
    /// 长度 = `layer_count × size × size × 4`。
    pub pixels: Vec<u8>,
}

impl TextureArray {
    pub fn is_empty(&self) -> bool {
        self.layer_count == 0 || self.pixels.is_empty()
    }

    /// 单层字节数。
    pub fn layer_bytes(&self) -> usize {
        (self.size * self.size * 4) as usize
    }

    /// 数据完整性：逐层拼接的 `pixels` 长度必须恰好等于 `layer_count × 单层字节数`。
    /// 供 GPU 上传前校验，避免 `pixels` 被写坏时 `get(..)` 静默跳过某层上传、
    /// 读到未初始化层导致花屏（显式失败 / 显式降级，不静默）。
    pub fn is_valid(&self) -> bool {
        if self.size == 0 || self.is_empty() {
            return false;
        }
        let expected = (self.layer_count as usize)
            .checked_mul(self.layer_bytes())
            .unwrap_or(usize::MAX);
        self.pixels.len() == expected
    }

    /// 追加一层贴图（最近邻 resize 到统一尺寸），返回新层索引。
    ///
    /// 供装配层把 6 张地表 tileset 追加到「模型贴图数组」之后：层索引从当前
    /// `layer_count` 起连续分配，天然不与模型贴图冲突。即使数组为空（无模型
    /// 资产）也可调用，得到只含地表层的合法数组。纯 CPU、确定性，不进权威
    /// 状态 / 哈希 / 快照。
    pub fn push_image(&mut self, image: &RgbaImage) -> u32 {
        let layer = self.layer_count;
        self.pixels
            .extend_from_slice(&resize_nearest(image, self.size));
        self.layer_count += 1;
        layer
    }
}

/// 构建期：URI → 层索引映射 + 已 resize 的层像素。
#[derive(Clone, Debug)]
pub struct TextureArrayBuilder {
    size: u32,
    layers: Vec<Vec<u8>>,
    index: BTreeMap<String, u32>,
}

impl Default for TextureArrayBuilder {
    fn default() -> Self {
        Self::new(TEXTURE_LAYER_SIZE)
    }
}

impl TextureArrayBuilder {
    pub fn new(size: u32) -> Self {
        Self {
            size: size.max(1),
            layers: Vec::new(),
            index: BTreeMap::new(),
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn layer_count(&self) -> u32 {
        self.layers.len() as u32
    }

    /// 归一化去重键：大小写不敏感、反斜杠转正斜杠。
    fn key(uri: &str) -> String {
        uri.replace('\\', "/").to_ascii_lowercase()
    }

    /// 登记一张贴图，返回其层索引。已存在则复用；超出上限则回退到第 0 层。
    /// 纯 CPU、确定性、可重复调用。
    pub fn push(&mut self, uri: &str, image: &RgbaImage) -> u32 {
        let key = Self::key(uri);
        if let Some(&layer) = self.index.get(&key) {
            return layer;
        }
        if self.layers.len() as u32 >= MAX_TEXTURE_LAYERS {
            return 0;
        }
        let layer = self.layers.len() as u32;
        self.layers.push(resize_nearest(image, self.size));
        self.index.insert(key, layer);
        layer
    }

    /// 查已登记 URI 的层索引（未登记返回 `None`）。
    pub fn layer_of(&self, uri: &str) -> Option<u32> {
        self.index.get(&Self::key(uri)).copied()
    }

    pub fn build(&self) -> TextureArray {
        let mut pixels =
            Vec::with_capacity(self.layers.len() * (self.size * self.size * 4) as usize);
        for layer in &self.layers {
            pixels.extend_from_slice(layer);
        }
        TextureArray {
            size: self.size,
            layer_count: self.layers.len() as u32,
            pixels,
        }
    }
}

/// 最近邻 resize 到 `size × size` RGBA8；越界 / 退化输入返回纯白不透明层，
/// 保证绝不 panic、长度恒为 `size²×4`。确定性。
pub fn resize_nearest(image: &RgbaImage, size: u32) -> Vec<u8> {
    let size = size.max(1);
    let mut out = vec![255u8; (size as usize) * (size as usize) * 4];
    if image.width == 0 || image.height == 0 || image.pixels.len() < 4 {
        return out;
    }
    let row_len = (image.width as usize).saturating_mul(4);
    for dy in 0..size {
        let sy = ((dy as u64 * image.height as u64) / size as u64) as u32;
        for dx in 0..size {
            let sx = ((dx as u64 * image.width as u64) / size as u64) as u32;
            let src = (sy as usize)
                .saturating_mul(row_len)
                .saturating_add((sx as usize).saturating_mul(4));
            let dst = ((dy as usize) * size as usize + dx as usize) * 4;
            if let Some(pixel) = image.pixels.get(src..src + 4) {
                out[dst..dst + 4].copy_from_slice(pixel);
            }
        }
    }
    out
}

/// 从磁盘加载并按 URI 登记（便捷入口；失败返回英文错误文本）。
pub fn push_path(builder: &mut TextureArrayBuilder, uri: &str, base: &Path) -> Result<u32, String> {
    let path = base.join(uri);
    let image = load_rgba(&path).map_err(|e| e.to_string())?;
    Ok(builder.push(uri, &image))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            pixels.extend_from_slice(&rgba);
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn resize_nearest_produces_exact_layer_size() {
        let image = solid(2, 2, [10, 20, 30, 255]);
        let out = resize_nearest(&image, 8);
        assert_eq!(out.len(), 8 * 8 * 4);
        assert!(out.chunks_exact(4).all(|p| p == [10, 20, 30, 255]));
    }

    #[test]
    fn resize_nearest_is_deterministic_and_handles_degenerate() {
        let image = solid(3, 5, [100, 150, 200, 128]);
        assert_eq!(resize_nearest(&image, 16), resize_nearest(&image, 16));
        let empty = RgbaImage {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
        let out = resize_nearest(&empty, 4);
        assert_eq!(out.len(), 4 * 4 * 4);
        assert!(out.chunks_exact(4).all(|p| p == [255, 255, 255, 255]));
    }

    #[test]
    fn builder_deduplicates_by_normalized_uri() {
        let mut b = TextureArrayBuilder::new(4);
        let image = solid(2, 2, [1, 2, 3, 255]);
        let a = b.push("Textures\\Foo.png", &image);
        let c = b.push("textures/foo.png", &image);
        assert_eq!(a, c, "大小写 / 反斜杠应归一化为同一层");
        assert_eq!(b.layer_count(), 1);
        let d = b.push("textures/bar.png", &image);
        assert_eq!(d, 1);
        assert_eq!(b.layer_count(), 2);
    }

    #[test]
    fn build_concatenates_layers_with_stable_order() {
        let mut b = TextureArrayBuilder::new(4);
        let red = solid(1, 1, [255, 0, 0, 255]);
        let green = solid(1, 1, [0, 255, 0, 255]);
        let r = b.push("r.png", &red);
        let g = b.push("g.png", &green);
        assert_eq!((r, g), (0, 1));
        let arr = b.build();
        assert_eq!(arr.size, 4);
        assert_eq!(arr.layer_count, 2);
        assert_eq!(arr.pixels.len(), 2 * 4 * 4 * 4);
        // 第 0 层全红、第 1 层全绿。
        let layer_bytes = 4 * 4 * 4;
        assert!(
            arr.pixels[..layer_bytes]
                .chunks_exact(4)
                .all(|p| p == [255, 0, 0, 255])
        );
        assert!(
            arr.pixels[layer_bytes..]
                .chunks_exact(4)
                .all(|p| p == [0, 255, 0, 255])
        );
    }

    #[test]
    fn is_valid_rejects_truncated_or_corrupt_pixel_buffers() {
        let mut b = TextureArrayBuilder::new(4);
        let image = solid(1, 1, [9, 8, 7, 255]);
        b.push("a.png", &image);
        b.push("b.png", &image);
        let valid = b.build();
        assert!(valid.is_valid(), "builder 产物必须完整");

        // 少一个字节：长度不符，必须判无效。
        let mut truncated = valid.clone();
        truncated.pixels.pop();
        assert!(!truncated.is_valid());
        // layer_count 与 pixels 不符（声称 3 层但只有 2 层数据）。
        let mut bad_count = valid.clone();
        bad_count.layer_count = 3;
        assert!(!bad_count.is_valid());
        // size=0 的退化数组无效。
        let mut zero = valid.clone();
        zero.size = 0;
        assert!(!zero.is_valid());
        // 空数组无效（无贴图资产，走占位纹理路径）。
        assert!(
            !TextureArray {
                size: TEXTURE_LAYER_SIZE,
                layer_count: 0,
                pixels: Vec::new(),
            }
            .is_valid()
        );
    }

    #[test]
    fn build_is_deterministic_and_layer_of_works() {
        let mut b = TextureArrayBuilder::new(4);
        let image = solid(2, 2, [9, 8, 7, 255]);
        b.push("a.png", &image);
        b.push("b.png", &image);
        let x = b.build();
        let y = b.build();
        assert_eq!(x, y);
        assert_eq!(b.layer_of("A.png"), Some(0));
        assert_eq!(b.layer_of("missing.png"), None);
        assert!(!x.is_empty());
    }

    #[test]
    fn layer_cap_falls_back_to_zero() {
        let mut b = TextureArrayBuilder::new(2);
        let image = solid(1, 1, [1, 1, 1, 255]);
        for i in 0..MAX_TEXTURE_LAYERS {
            b.push(&format!("u{i}.png"), &image);
        }
        assert_eq!(b.layer_count(), MAX_TEXTURE_LAYERS);
        let overflow = b.push("overflow.png", &image);
        assert_eq!(overflow, 0, "超上限应回退到第 0 层而非 panic");
        assert_eq!(b.layer_count(), MAX_TEXTURE_LAYERS);
    }

    #[test]
    fn push_image_appends_after_existing_layers() {
        let mut b = TextureArrayBuilder::new(4);
        let red = solid(1, 1, [255, 0, 0, 255]);
        b.push("model.png", &red);
        let mut arr = b.build();
        assert_eq!(arr.layer_count, 1);
        let layer_bytes = arr.layer_bytes();
        let green = solid(1, 1, [0, 255, 0, 255]);
        let layer = arr.push_image(&green);
        assert_eq!(layer, 1, "追加层索引应接在已有层之后，不与模型冲突");
        assert_eq!(arr.layer_count, 2);
        assert_eq!(arr.pixels.len(), 2 * layer_bytes);
        // 旧层（模型贴图）不受影响。
        assert!(
            arr.pixels[..layer_bytes]
                .chunks_exact(4)
                .all(|p| p == [255, 0, 0, 255])
        );
        // 新层（地表 tileset）为绿色。
        assert!(
            arr.pixels[layer_bytes..]
                .chunks_exact(4)
                .all(|p| p == [0, 255, 0, 255])
        );
    }

    #[test]
    fn push_image_on_empty_array_builds_valid_terrain_only_array() {
        // 无模型资产时（CI）数组为空；追加 6 张地表仍应得到合法数组。
        let mut arr = TextureArray {
            size: TEXTURE_LAYER_SIZE,
            layer_count: 0,
            pixels: Vec::new(),
        };
        let tile = solid(2, 2, [90, 120, 60, 255]);
        let mut layers = Vec::new();
        for _ in 0..6 {
            layers.push(arr.push_image(&tile));
        }
        assert_eq!(layers, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(arr.layer_count, 6);
        assert_eq!(arr.pixels.len(), 6 * arr.layer_bytes());
        assert!(!arr.is_empty());
    }
}
