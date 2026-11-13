//! 本机导出 glTF 2.0 模型子集解析（真实模型视觉阶段）。
//!
//! 上游 `asset-convert` 把 WC3 的 MDX 转成标准 glTF 2.0：顶点 / 法线 / UV /
//! 索引交错存放在外链 `.bin`，材质贴图为相对路径的 PNG。本模块只读取这个
//! **中立子集**，输出纯 `f32` 几何与 RGBA8 贴图，不含任何游戏资产、不复制
//! 上游源码；路径来自 gitignored 的本机 `config.toml`（ADR 0001）。
//!
//! ## 坐标系与单位
//!
//! 上游 `wc3ToGltfVec3(x, y, z) = [x, z, -y]` 把 WC3 的 Z-up 转成 glTF 的
//! Y-up，因此 glTF `POSITION` 的 `y` 是**高度**、`z` 是取反后的水平轴。
//! 顶点坐标保持为 glTF 里的**原始值**（WC3 模型单位，未乘节点 `scale`）——
//! 节点 `scale=0.01` 是上游给 Godot 的展示单位换算，与本仓库的玩法世界缩放
//! 无关：装配层直接用 `w3-render::model_geometry::MODEL_WORLD_SCALE`（=2/128）
//! 处理 glTF→玩法世界转换。解析出的 `node_scale` 当前未被消费，保留为规范
//! 字段供后续精确单位换算，MUST NOT 误当成已生效的缩放。
//!
//! ## 骨骼
//!
//! glTF 规范下 skinned mesh 的 `POSITION` 即为 bind pose（站立姿势），静态
//! 呈现直接取用即可，本模块不采样 `animations` / `JOINTS_0` / `WEIGHTS_0`；
//! 逐帧骨骼蒙皮留后续阶段。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::textures::{RgbaImage, TextureError, load_rgba};

/// glTF `componentType` 常量。
const BYTE: u32 = 5120;
const UNSIGNED_BYTE: u32 = 5121;
const SHORT: u32 = 5122;
const UNSIGNED_SHORT: u32 = 5123;
const UNSIGNED_INT: u32 = 5125;
const FLOAT: u32 = 5126;

/// glTF 图元模式：4 = TRIANGLES（本模块只支持三角形）。
const MODE_TRIANGLES: u32 = 4;

/// 解析错误；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GltfError {
    /// 文件读取 / glTF JSON 或 `.bin` 解码失败。
    Io { path: PathBuf, message: String },
    /// JSON 结构非法或字段缺失。
    Json { message: String },
    /// 非 2.0 版本。
    UnsupportedVersion { found: String },
    /// accessor / bufferView / buffer 索引越界或长度不足。
    InvalidAccessor { index: usize, message: String },
    /// 不支持的图元模式 / `componentType` / accessor 类型。
    Unsupported { message: String },
    /// 贴图解码失败。
    Texture { path: PathBuf, message: String },
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "cannot read {}: {message}", path.display()),
            Self::Json { message } => write!(f, "invalid glTF json: {message}"),
            Self::UnsupportedVersion { found } => {
                write!(f, "unsupported glTF version: {found}")
            }
            Self::InvalidAccessor { index, message } => {
                write!(f, "invalid accessor[{index}]: {message}")
            }
            Self::Unsupported { message } => write!(f, "unsupported glTF feature: {message}"),
            Self::Texture { path, message } => {
                write!(f, "cannot decode {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for GltfError {}

// ---- glTF 2.0 JSON 子集（camelCase）----

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfAsset {
    #[serde(default)]
    version: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfBuffer {
    uri: Option<String>,
    #[serde(default)]
    byte_length: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfBufferView {
    buffer: usize,
    #[serde(default)]
    byte_offset: usize,
    /// glTF 声明的视图长度；当前按 accessor 的 `count × stride` 逐元素边界
    /// 检查读取，该字段保留为规范项供后续整体校验使用。
    #[allow(dead_code)]
    byte_length: usize,
    byte_stride: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfAccessor {
    buffer_view: Option<usize>,
    #[serde(default)]
    byte_offset: usize,
    component_type: u32,
    count: usize,
    #[serde(rename = "type")]
    kind: String,
    /// 整数分量是否归一化；本模块位置 / 索引均取原值，保留为规范项。
    #[serde(default)]
    #[allow(dead_code)]
    normalized: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct GltfPrimitive {
    attributes: BTreeMap<String, usize>,
    indices: Option<usize>,
    material: Option<usize>,
    #[serde(default = "default_mode")]
    mode: u32,
}

fn default_mode() -> u32 {
    MODE_TRIANGLES
}

#[derive(Clone, Debug, Deserialize)]
struct GltfMesh {
    #[serde(default)]
    primitives: Vec<GltfPrimitive>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct GltfTextureRef {
    index: usize,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfPbr {
    #[serde(default)]
    base_color_texture: Option<GltfTextureRef>,
    #[serde(default)]
    base_color_factor: Option<[f32; 4]>,
}

/// 上游 WC3 材质元数据（`extras` 里的 `wc3FilterMode` 等）。
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfMaterialExtras {
    #[serde(default)]
    wc3_filter_mode: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfMaterial {
    #[serde(default)]
    pbr_metallic_roughness: Option<GltfPbr>,
    #[serde(default)]
    alpha_mode: Option<String>,
    #[serde(default)]
    alpha_cutoff: Option<f32>,
    #[serde(default)]
    double_sided: bool,
    #[serde(default)]
    extras: GltfMaterialExtras,
}

#[derive(Clone, Debug, Deserialize)]
struct GltfTexture {
    source: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
struct GltfImage {
    uri: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GltfDoc {
    #[serde(default)]
    asset: GltfAsset,
    #[serde(default)]
    buffers: Vec<GltfBuffer>,
    #[serde(default)]
    buffer_views: Vec<GltfBufferView>,
    #[serde(default)]
    accessors: Vec<GltfAccessor>,
    #[serde(default)]
    meshes: Vec<GltfMesh>,
    #[serde(default)]
    materials: Vec<GltfMaterial>,
    #[serde(default)]
    textures: Vec<GltfTexture>,
    #[serde(default)]
    images: Vec<GltfImage>,
}

// ---- 输出：中立模型网格 ----

/// WC3 geoset 的混合模式（上游 `extras.wc3FilterMode`）。
///
/// `Opaque` / `Mask` 是**实体几何**（建筑、单位、树叶面片）；`Blend` /
/// `Additive` / `AddAlpha` / `Modulate` / `Modulate2x` 是**特效层**（法术光环、
/// 队伍辉光、地面 UberSplat、死亡烟雾等），在 WC3 里按加色 / alpha 混合叠加，
/// 不是实体表面。表现层烘焙 MUST 过滤掉特效层，否则会画成白色纸片 / 黑块。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GeosetFilter {
    /// 0：不透明实体。
    #[default]
    Opaque,
    /// 1：alpha 镂空实体（树叶 / 旗帜），按 alphaCutoff 剔除。
    Mask,
    /// 2：alpha 混合特效。
    Blend,
    /// 3：加色特效（辉光 / 光效）。
    Additive,
    /// 4：加 alpha 特效。
    AddAlpha,
    /// 5：调制特效。
    Modulate,
    /// 6：2x 调制特效。
    Modulate2x,
    /// 地面投影贴片（UberSplat / Splats / 阴影）：`wc3FilterMode` 常为 0，
    /// 但它是铺在地面的大四边形投影、不是实体几何，放大后会变成刺眼的大
    /// 黑 / 白三角。按贴图 URI 特征识别并过滤。
    GroundDecal,
}

impl GeosetFilter {
    /// 从 `wc3FilterMode` 数字构造；未知值保守视为实体（不误删几何）。
    pub fn from_wc3(value: u32) -> Self {
        match value {
            1 => Self::Mask,
            2 => Self::Blend,
            3 => Self::Additive,
            4 => Self::AddAlpha,
            5 => Self::Modulate,
            6 => Self::Modulate2x,
            _ => Self::Opaque,
        }
    }

    /// 是否为实体几何（应被烘焙成 3D 网格）。特效层 / 地面贴片返回 false。
    pub fn is_entity(self) -> bool {
        matches!(self, Self::Opaque | Self::Mask)
    }

    /// 贴图 URI 是否为地面投影贴片（UberSplat / Splats 目录）。
    ///
    /// 仅按「`Splats/` 目录」或「文件名含 `ubersplat`」这类**路径/文件名级**
    /// 特征识别，不用 `shadow` / `shockwave` 等裸子串（可能误判含这些字样的
    /// 实体贴图而丢几何）。Shockwave 等加色特效已由 `wc3FilterMode` 判定为
    /// 非实体，无需 URI 兜底。
    fn is_ground_decal_uri(uri: &str) -> bool {
        let lower = uri.replace('\\', "/").to_ascii_lowercase();
        let filename = lower.rsplit('/').next().unwrap_or(&lower);
        lower.contains("/splats/") || filename.contains("ubersplat")
    }
}

/// 一个材质子网格：交错顶点展开成并行数组 + 三角形索引 + 贴图与混合设定。
#[derive(Clone, Debug, PartialEq)]
pub struct SubMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// 三角形索引（长度 = 3 × 三角形数）。
    pub indices: Vec<u32>,
    /// 材质的 baseColorFactor（缺省白色）。
    pub base_color: [f32; 4],
    /// `alphaMode=MASK` 的阈值（缺省 0.5）；`OPAQUE`/`BLEND` 为 0。
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    /// geoset 混合模式（决定实体 / 特效层）。
    pub filter_mode: GeosetFilter,
    /// 解码后的 RGBA8 贴图（纯数据）。`load_gltf` 填充；`parse_gltf` 为 `None`。
    pub texture: Option<RgbaImage>,
    /// 贴图相对 URI（供图集打包 / 诊断）。
    pub texture_uri: Option<String>,
}

/// 解析后的模型：若干子网格 + 根节点 scale（供装配层换算，缺省 `[1,1,1]`）。
#[derive(Clone, Debug, PartialEq)]
pub struct ModelMesh {
    pub submeshes: Vec<SubMesh>,
    pub node_scale: [f32; 3],
}

impl ModelMesh {
    /// 三角形总数（跨所有子网格）。
    pub fn triangle_count(&self) -> usize {
        self.submeshes.iter().map(|s| s.indices.len() / 3).sum()
    }

    /// 顶点总数（跨所有子网格）。
    pub fn vertex_count(&self) -> usize {
        self.submeshes.iter().map(|s| s.positions.len()).sum()
    }
}

fn component_size(component_type: u32) -> Result<usize, GltfError> {
    match component_type {
        BYTE | UNSIGNED_BYTE => Ok(1),
        SHORT | UNSIGNED_SHORT => Ok(2),
        UNSIGNED_INT | FLOAT => Ok(4),
        other => Err(GltfError::Unsupported {
            message: format!("componentType={other}"),
        }),
    }
}

fn type_components(kind: &str) -> Result<usize, GltfError> {
    match kind {
        "SCALAR" => Ok(1),
        "VEC2" => Ok(2),
        "VEC3" => Ok(3),
        "VEC4" => Ok(4),
        other => Err(GltfError::Unsupported {
            message: format!("accessor type={other}"),
        }),
    }
}

/// 读一个标量并转成 `f32`（整数类型按原值，不归一化——位置 / 索引需要原值）。
fn read_scalar(bytes: &[u8], offset: usize, component_type: u32) -> Result<f32, GltfError> {
    let size = component_size(component_type)?;
    let end = offset
        .checked_add(size)
        .ok_or_else(|| GltfError::InvalidAccessor {
            index: 0,
            message: "offset overflow".into(),
        })?;
    let chunk = bytes
        .get(offset..end)
        .ok_or_else(|| GltfError::InvalidAccessor {
            index: 0,
            message: format!("byte range {offset}..{end} out of bounds"),
        })?;
    Ok(match component_type {
        BYTE => i8::from_le_bytes([chunk[0]]) as f32,
        UNSIGNED_BYTE => chunk[0] as f32,
        SHORT => i16::from_le_bytes([chunk[0], chunk[1]]) as f32,
        UNSIGNED_SHORT => u16::from_le_bytes([chunk[0], chunk[1]]) as f32,
        UNSIGNED_INT => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f32,
        FLOAT => f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
        _ => unreachable!("component_size already validated"),
    })
}

/// 按 accessor 读取 `count × components` 个 `f32`，处理 `byteStride` 交错。
fn read_accessor_f32(
    doc: &GltfDoc,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<(Vec<f32>, usize), GltfError> {
    let accessor = doc
        .accessors
        .get(accessor_index)
        .ok_or_else(|| GltfError::InvalidAccessor {
            index: accessor_index,
            message: "index out of range".into(),
        })?;
    let components = type_components(&accessor.kind)?;
    let comp_size = component_size(accessor.component_type)?;
    let buffer_view_index = accessor
        .buffer_view
        .ok_or_else(|| GltfError::InvalidAccessor {
            index: accessor_index,
            message: "missing bufferView".into(),
        })?;
    let view =
        doc.buffer_views
            .get(buffer_view_index)
            .ok_or_else(|| GltfError::InvalidAccessor {
                index: accessor_index,
                message: format!("bufferView={buffer_view_index} out of range"),
            })?;
    let bytes = buffers
        .get(view.buffer)
        .ok_or_else(|| GltfError::InvalidAccessor {
            index: accessor_index,
            message: format!("buffer={} out of range", view.buffer),
        })?;
    let packed = comp_size * components;
    let stride = view.byte_stride.unwrap_or(packed);
    if stride < packed {
        return Err(GltfError::InvalidAccessor {
            index: accessor_index,
            message: format!("byteStride={stride} < packed={packed}"),
        });
    }
    let base = view.byte_offset.saturating_add(accessor.byte_offset);
    let mut out = Vec::with_capacity(accessor.count.saturating_mul(components));
    for element in 0..accessor.count {
        let element_base = base.saturating_add(element.saturating_mul(stride));
        for component in 0..components {
            let offset = element_base.saturating_add(component.saturating_mul(comp_size));
            out.push(read_scalar(bytes, offset, accessor.component_type)?);
        }
    }
    Ok((out, components))
}

/// 读取索引 accessor 为 `Vec<u32>`。
fn read_indices(
    doc: &GltfDoc,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<u32>, GltfError> {
    let (values, components) = read_accessor_f32(doc, buffers, accessor_index)?;
    if components != 1 {
        return Err(GltfError::InvalidAccessor {
            index: accessor_index,
            message: "index accessor must be SCALAR".into(),
        });
    }
    Ok(values.iter().map(|&v| v as u32).collect())
}

fn f32_chunk(values: &[f32], components: usize) -> impl Iterator<Item = &[f32]> {
    values.chunks_exact(components)
}

fn parse_submesh(
    doc: &GltfDoc,
    buffers: &[Vec<u8>],
    primitive: &GltfPrimitive,
) -> Result<SubMesh, GltfError> {
    if primitive.mode != MODE_TRIANGLES {
        return Err(GltfError::Unsupported {
            message: format!("primitive mode={}", primitive.mode),
        });
    }
    let position_index = *primitive
        .attributes
        .get("POSITION")
        .ok_or_else(|| GltfError::Json {
            message: "primitive missing POSITION".into(),
        })?;
    let (position_values, pc) = read_accessor_f32(doc, buffers, position_index)?;
    if pc != 3 {
        return Err(GltfError::InvalidAccessor {
            index: position_index,
            message: "POSITION must be VEC3".into(),
        });
    }
    let positions: Vec<[f32; 3]> = f32_chunk(&position_values, 3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    // NORMAL 缺省时给朝上的法线（不 panic，退化但可呈现）。
    let normals: Vec<[f32; 3]> = match primitive.attributes.get("NORMAL") {
        Some(&index) => {
            let (values, nc) = read_accessor_f32(doc, buffers, index)?;
            if nc == 3 {
                f32_chunk(&values, 3).map(|c| [c[0], c[1], c[2]]).collect()
            } else {
                vec![[0.0, 1.0, 0.0]; positions.len()]
            }
        }
        None => vec![[0.0, 1.0, 0.0]; positions.len()],
    };

    // TEXCOORD_0 缺省时用零 UV（贴图采样落到左上角，不 panic）。
    let uvs: Vec<[f32; 2]> = match primitive.attributes.get("TEXCOORD_0") {
        Some(&index) => {
            let (values, uc) = read_accessor_f32(doc, buffers, index)?;
            if uc == 2 {
                f32_chunk(&values, 2).map(|c| [c[0], c[1]]).collect()
            } else {
                vec![[0.0, 0.0]; positions.len()]
            }
        }
        None => vec![[0.0, 0.0]; positions.len()],
    };

    let indices = match primitive.indices {
        Some(index) => read_indices(doc, buffers, index)?,
        None => (0..positions.len() as u32).collect(),
    };

    let material = primitive
        .material
        .and_then(|index| doc.materials.get(index));
    let base_color = material
        .and_then(|m| m.pbr_metallic_roughness.as_ref())
        .and_then(|pbr| pbr.base_color_factor)
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let alpha_cutoff = match material.and_then(|m| m.alpha_mode.as_deref()) {
        Some("MASK") => material.and_then(|m| m.alpha_cutoff).unwrap_or(0.5),
        _ => 0.0,
    };
    let double_sided = material.is_some_and(|m| m.double_sided);

    // 贴图 URI：material.pbr.baseColorTexture.index → textures[].source → images[].uri。
    let texture_uri = material
        .and_then(|m| m.pbr_metallic_roughness.as_ref())
        .and_then(|pbr| pbr.base_color_texture.as_ref())
        .and_then(|tex_ref| doc.textures.get(tex_ref.index))
        .and_then(|texture| texture.source)
        .and_then(|source| doc.images.get(source))
        .and_then(|image| image.uri.clone());

    // geoset 混合模式：优先用上游 `extras.wc3FilterMode`（WC3 语义最准）；
    // 缺失时按 glTF `alphaMode` 兜底推断（BLEND 视为特效层）。
    let mut filter_mode = match material.and_then(|m| m.extras.wc3_filter_mode) {
        Some(value) => GeosetFilter::from_wc3(value),
        None => match material.and_then(|m| m.alpha_mode.as_deref()) {
            Some("MASK") => GeosetFilter::Mask,
            Some("BLEND") => GeosetFilter::Blend,
            _ => GeosetFilter::Opaque,
        },
    };
    // 地面投影贴片（UberSplat 等）即使用 Opaque 标记也不是实体几何，覆盖为
    // GroundDecal 让烘焙层过滤，避免放大后变成大黑 / 白三角。
    if let Some(uri) = texture_uri.as_deref() {
        if filter_mode.is_entity() && GeosetFilter::is_ground_decal_uri(uri) {
            filter_mode = GeosetFilter::GroundDecal;
        }
    }

    // 防御：索引越界或数组长度不齐会破坏后续顶点展开，显式报错。
    let vertex_count = positions.len();
    if normals.len() != vertex_count || uvs.len() != vertex_count {
        return Err(GltfError::InvalidAccessor {
            index: position_index,
            message: "POSITION/NORMAL/TEXCOORD_0 length mismatch".into(),
        });
    }
    if indices.iter().any(|&i| i as usize >= vertex_count) {
        return Err(GltfError::InvalidAccessor {
            index: position_index,
            message: "index out of POSITION range".into(),
        });
    }

    Ok(SubMesh {
        positions,
        normals,
        uvs,
        indices,
        base_color,
        alpha_cutoff,
        double_sided,
        filter_mode,
        texture: None,
        texture_uri,
    })
}

/// 解析 glTF JSON + 已读入内存的 `.bin` 缓冲；纯函数，零文件 IO，供 CPU 单测。
/// 贴图 `texture` 一律为 `None`，仅保留 `texture_uri`（由 [`load_gltf`] 解码）。
pub fn parse_gltf(json: &str, buffers: &[Vec<u8>]) -> Result<ModelMesh, GltfError> {
    let doc: GltfDoc = serde_json::from_str(json).map_err(|error| GltfError::Json {
        message: error.to_string(),
    })?;
    if !doc.asset.version.is_empty() && !doc.asset.version.starts_with("2.") {
        return Err(GltfError::UnsupportedVersion {
            found: doc.asset.version.clone(),
        });
    }
    for (index, buffer) in doc.buffers.iter().enumerate() {
        if buffer.byte_length > 0
            && buffers
                .get(index)
                .is_none_or(|b| b.len() < buffer.byte_length)
        {
            return Err(GltfError::InvalidAccessor {
                index,
                message: format!(
                    "buffer[{}] byteLength={} shorter than provided",
                    index, buffer.byte_length
                ),
            });
        }
    }

    let mut submeshes = Vec::new();
    for mesh in &doc.meshes {
        for primitive in &mesh.primitives {
            submeshes.push(parse_submesh(&doc, buffers, primitive)?);
        }
    }
    if submeshes.is_empty() {
        return Err(GltfError::Json {
            message: "no triangle primitives found".into(),
        });
    }
    Ok(ModelMesh {
        submeshes,
        node_scale: [1.0, 1.0, 1.0],
    })
}

/// 从本机 `.gltf` 文件加载模型：读 JSON、外链 `.bin`、解码 PNG 贴图。
/// 相对路径相对 `.gltf` 所在目录解析；失败显式返回错误，不静默降级。
pub fn load_gltf(path: &Path) -> Result<ModelMesh, GltfError> {
    let json = std::fs::read_to_string(path).map_err(|error| GltfError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let doc: GltfDoc = serde_json::from_str(&json).map_err(|error| GltfError::Json {
        message: error.to_string(),
    })?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut buffers = Vec::with_capacity(doc.buffers.len());
    for buffer in &doc.buffers {
        match &buffer.uri {
            Some(uri) if !uri.starts_with("data:") => {
                let bin_path = base.join(uri);
                let bytes = std::fs::read(&bin_path).map_err(|error| GltfError::Io {
                    path: bin_path.clone(),
                    message: error.to_string(),
                })?;
                buffers.push(bytes);
            }
            _ => {
                return Err(GltfError::Unsupported {
                    message: "embedded/base64 buffer not supported".into(),
                });
            }
        }
    }

    let mut mesh = parse_gltf(&json, &buffers)?;

    // 节点 scale（根节点通常为 [0.01,0.01,0.01]）作为规范字段保留，当前
    // 装配层未消费（glTF→玩法世界转换直接用 MODEL_WORLD_SCALE）。
    // 这里读 nodes[0].scale（glTF 子集的根节点），缺省 [1,1,1]。
    #[derive(Deserialize)]
    struct NodeScale {
        #[serde(default)]
        scale: Option<[f32; 3]>,
    }
    #[derive(Deserialize)]
    struct NodesDoc {
        #[serde(default)]
        nodes: Vec<NodeScale>,
    }
    let nodes: NodesDoc = serde_json::from_str(&json).unwrap_or(NodesDoc { nodes: Vec::new() });
    if let Some(scale) = nodes.nodes.first().and_then(|n| n.scale) {
        mesh.node_scale = scale;
    }

    // 解码贴图：同名 URI 复用一份 RGBA，避免重复读盘。
    let mut cache: BTreeMap<String, RgbaImage> = BTreeMap::new();
    for submesh in &mut mesh.submeshes {
        let Some(uri) = submesh.texture_uri.clone() else {
            continue;
        };
        if let Some(image) = cache.get(&uri) {
            submesh.texture = Some(image.clone());
            continue;
        }
        let image_path = base.join(&uri);
        let image = load_rgba(&image_path).map_err(|error: TextureError| GltfError::Texture {
            path: image_path.clone(),
            message: error.to_string(),
        })?;
        cache.insert(uri, image.clone());
        submesh.texture = Some(image);
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个最小 glTF：1 个三角形、交错顶点（POSITION/NORMAL/UV）、
    /// u16 索引、外链 .bin、无贴图。返回 (json, bin)。
    fn minimal_gltf() -> (String, Vec<u8>) {
        // bufferView[0]：交错顶点，stride = (3+3+2)*4 = 32，3 个顶点 = 96 字节。
        // bufferView[1]：u16 索引 3 个 = 6 字节，4 字节对齐到 8。
        let mut bin = Vec::new();
        for (pos, nrm, uv) in [
            ([0.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0], [0.0f32, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ] {
            for v in pos.iter().chain(nrm.iter()).chain(uv.iter()) {
                bin.extend_from_slice(&v.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        bin.extend_from_slice(&[0u8; 2]); // bufferView 尾部 4 字节对齐

        let json = r#"{
            "asset": {"version": "2.0"},
            "buffers": [{"uri": "m.bin", "byteLength": 104}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 96, "byteStride": 32},
                {"buffer": 0, "byteOffset": 96, "byteLength": 8}
            ],
            "accessors": [
                {"bufferView": 0, "byteOffset": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 0, "byteOffset": 12, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 0, "byteOffset": 24, "componentType": 5126, "count": 3, "type": "VEC2"},
                {"bufferView": 1, "byteOffset": 0, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2},
                "indices": 3, "material": 0, "mode": 4
            }]}],
            "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [1,1,1,1]}}]
        }"#;
        (json.to_string(), bin)
    }

    #[test]
    fn parses_interleaved_triangle_strictly() {
        let (json, bin) = minimal_gltf();
        let mesh = parse_gltf(&json, &[bin]).expect("parse minimal glTF");
        assert_eq!(mesh.submeshes.len(), 1);
        let sub = &mesh.submeshes[0];
        assert_eq!(sub.positions.len(), 3);
        assert_eq!(sub.normals.len(), 3);
        assert_eq!(sub.uvs.len(), 3);
        assert_eq!(sub.indices, vec![0, 1, 2]);
        // 交错读取：第二个顶点的 POSITION 应为 [1,0,0]。
        assert_eq!(sub.positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(sub.uvs[1], [1.0, 0.0]);
        assert_eq!(sub.normals[0], [0.0, 1.0, 0.0]);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertex_count(), 3);
        assert!(sub.texture.is_none());
    }

    #[test]
    fn parse_is_deterministic() {
        let (json, bin) = minimal_gltf();
        let a = parse_gltf(&json, &[bin.clone()]).unwrap();
        let b = parse_gltf(&json, &[bin]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parses_u32_indices() {
        // 与 minimal_gltf 相同的交错顶点，但索引用 UNSIGNED_INT(5125)。
        let (base_json, _) = minimal_gltf();
        let json = base_json
            .replace(
                r#"{"bufferView": 1, "byteOffset": 0, "byteLength": 8}"#,
                r#"{"bufferView": 1, "byteOffset": 0, "byteLength": 12}"#,
            )
            .replace(
                r#"{"bufferView": 1, "byteOffset": 0, "componentType": 5123, "count": 3, "type": "SCALAR"}"#,
                r#"{"bufferView": 1, "byteOffset": 0, "componentType": 5125, "count": 3, "type": "SCALAR"}"#,
            );
        // bin：3 个交错顶点（96 字节）+ 3 个 u32 索引。
        let mut bin = Vec::new();
        for (pos, nrm, uv) in [
            ([0.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0], [0.0f32, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ] {
            for v in pos.iter().chain(nrm.iter()).chain(uv.iter()) {
                bin.extend_from_slice(&v.to_le_bytes());
            }
        }
        for i in [0u32, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        let mesh = parse_gltf(&json, &[bin]).expect("parse u32-index glTF");
        assert_eq!(mesh.submeshes[0].indices, vec![0, 1, 2]);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn rejects_out_of_range_index() {
        let (mut json, bin) = minimal_gltf();
        // 把索引 accessor 的 componentType 改成 u32 会越界读到垃圾，改为
        // 直接构造一个索引含 99 的 glTF：改 bin 第 96 字节起为 u16 99。
        let mut bad = bin.clone();
        bad[96] = 99;
        let _ = &mut json;
        let result = parse_gltf(&json, &[bad]);
        assert!(matches!(result, Err(GltfError::InvalidAccessor { .. })));
    }

    #[test]
    fn rejects_unsupported_version_and_bad_json() {
        let (_, bin) = minimal_gltf();
        let v1 = r#"{"asset":{"version":"1.0"},"meshes":[]}"#;
        assert!(matches!(
            parse_gltf(v1, &[bin.clone()]),
            Err(GltfError::UnsupportedVersion { .. })
        ));
        assert!(matches!(
            parse_gltf("not json", &[bin]),
            Err(GltfError::Json { .. })
        ));
    }

    #[test]
    fn rejects_short_buffer_and_missing_position() {
        let (json, _) = minimal_gltf();
        assert!(matches!(
            parse_gltf(&json, &[vec![0u8; 4]]),
            Err(GltfError::InvalidAccessor { .. })
        ));
        let no_pos = r#"{"asset":{"version":"2.0"},"buffers":[],
            "meshes":[{"primitives":[{"attributes":{},"mode":4}]}]}"#;
        assert!(matches!(
            parse_gltf(no_pos, &[]),
            Err(GltfError::Json { .. })
        ));
    }

    #[test]
    fn alpha_cutoff_and_double_sided_from_material() {
        let (base_json, bin) = minimal_gltf();
        let json = base_json.replace(
            "\"materials\": [{\"pbrMetallicRoughness\": {\"baseColorFactor\": [1,1,1,1]}}]",
            "\"materials\": [{\"alphaMode\": \"MASK\", \"alphaCutoff\": 0.75, \"doubleSided\": true}]",
        );
        let mesh = parse_gltf(&json, &[bin]).unwrap();
        let sub = &mesh.submeshes[0];
        assert!((sub.alpha_cutoff - 0.75).abs() < 1e-6);
        assert!(sub.double_sided);
        assert_eq!(sub.filter_mode, GeosetFilter::Mask, "MASK 兜底推断");
    }

    #[test]
    fn wc3_filter_mode_is_parsed_and_classified() {
        let (base_json, bin) = minimal_gltf();
        let material =
            "\"materials\": [{\"pbrMetallicRoughness\": {\"baseColorFactor\": [1,1,1,1]}}]";
        for (value, expected) in [
            (0u32, GeosetFilter::Opaque),
            (1, GeosetFilter::Mask),
            (2, GeosetFilter::Blend),
            (3, GeosetFilter::Additive),
            (4, GeosetFilter::AddAlpha),
            (5, GeosetFilter::Modulate),
            (6, GeosetFilter::Modulate2x),
        ] {
            let replacement =
                format!("\"materials\": [{{\"extras\": {{\"wc3FilterMode\": {value}}}}}]");
            let json = base_json.replace(material, &replacement);
            let mesh = parse_gltf(&json, &[bin.clone()]).unwrap();
            assert_eq!(mesh.submeshes[0].filter_mode, expected, "value={value}");
        }
        // 实体 / 特效分类。
        assert!(GeosetFilter::Opaque.is_entity());
        assert!(GeosetFilter::Mask.is_entity());
        assert!(!GeosetFilter::Blend.is_entity());
        assert!(!GeosetFilter::Additive.is_entity());
        assert!(!GeosetFilter::Modulate2x.is_entity());
    }

    #[test]
    fn ground_decal_uri_is_detected_and_not_entity() {
        assert!(GeosetFilter::is_ground_decal_uri(
            "../../../ReplaceableTextures/Splats/HumanTownHallUberSplat.png"
        ));
        assert!(GeosetFilter::is_ground_decal_uri("x/Splats/Foo.png"));
        // 收紧后：含 shadow 字样的实体贴图不再被误判为地面贴片。
        assert!(!GeosetFilter::is_ground_decal_uri(
            "Textures/CastleShadow.png"
        ));
        assert!(!GeosetFilter::is_ground_decal_uri(
            "Textures/Shockwave1.png"
        ));
        assert!(!GeosetFilter::is_ground_decal_uri("./Footman.png"));
        assert!(!GeosetFilter::GroundDecal.is_entity());
        // 贴图 URI 命中 Splats 的 Opaque 材质应被覆盖为 GroundDecal。
        let (base_json, bin) = minimal_gltf();
        let material =
            "\"materials\": [{\"pbrMetallicRoughness\": {\"baseColorFactor\": [1,1,1,1]}}]";
        let replacement = "\"images\": [{\"uri\": \"x/Splats/UberSplat.png\"}], \
            \"textures\": [{\"source\": 0}], \
            \"materials\": [{\"pbrMetallicRoughness\": {\"baseColorTexture\": {\"index\": 0}}}]";
        let json = base_json.replace(material, replacement);
        let mesh = parse_gltf(&json, &[bin]).unwrap();
        assert_eq!(mesh.submeshes[0].filter_mode, GeosetFilter::GroundDecal);
    }

    #[test]
    fn load_gltf_missing_file_reports_io() {
        assert!(matches!(
            load_gltf(Path::new("C:/definitely-not-a-real-model/missing.gltf")),
            Err(GltfError::Io { .. })
        ));
    }
}
