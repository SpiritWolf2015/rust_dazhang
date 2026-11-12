//! 模型、骨骼动画、队伍色与挂点（表现层数据结构）。
//!
//! 复用上游 `godot_warcraft3/docs/design/asset-convert/` 记录的 MDX 语义：
//! 骨骼（Bone）、动画序列（Sequence）、GeosetAnim、FilterMode、挂点。
//! 转换器产出的 glTF/JSON 留在玩家本机，由 `ModelLibrary` 按路径读取；
//! 本 crate 不提交任何资产（见 ADR 0001）。所有动画采样都是纯表现层
//! `f32` 运算，绝不写回模拟状态。

use std::collections::BTreeMap;
use std::path::Path;

use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;
use w3_core::EntityView;

use crate::error::RenderError;

/// 默认动画帧率（30 Hz 冻结，见 ADR 0002）。
const DEFAULT_FPS: f32 = 30.0;

fn default_fps() -> f32 {
    DEFAULT_FPS
}

fn default_one() -> f32 {
    1.0
}

fn default_white() -> [f32; 3] {
    [1.0; 3]
}

// ---- 上游转换规格 JSON（glTF / 导出元数据，字段采用 camelCase）----

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoneDef {
    pub name: String,
    #[serde(default)]
    pub parent: Option<usize>,
    #[serde(default)]
    pub pivot: [f32; 3],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceDef {
    pub name: String,
    pub start: f32,
    pub end: f32,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default)]
    pub looping: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeosetAnimDef {
    pub geoset: u32,
    #[serde(default = "default_one")]
    pub alpha: f32,
    #[serde(default = "default_white")]
    pub color: [f32; 3],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDef {
    pub name: String,
    pub bone: usize,
    #[serde(default)]
    pub offset: [f32; 3],
}

/// MDX 材质混合模式（上游 FilterMode）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum FilterMode {
    Opaque,
    Blend,
    Additive,
    AddAlpha,
    Modulate,
    Modulate2x,
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::Opaque
    }
}

impl FilterMode {
    /// 是否需要透明混合（供 GPU 管线选择 blend state）。
    pub fn is_transparent(self) -> bool {
        !matches!(self, Self::Opaque)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialDef {
    #[serde(default)]
    pub filter_mode: Option<FilterMode>,
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default)]
    pub team_color: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDef {
    pub bone: usize,
    pub times: Vec<f32>,
    #[serde(default)]
    pub translations: Vec<[f32; 3]>,
    #[serde(default)]
    pub rotations: Vec<[f32; 4]>,
    #[serde(default)]
    pub scales: Vec<[f32; 3]>,
}

/// 上游 MDX→glTF 转换产物的 JSON 子集。
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDocument {
    #[serde(default)]
    pub bones: Vec<BoneDef>,
    #[serde(default)]
    pub sequences: Vec<SequenceDef>,
    #[serde(default)]
    pub geoset_anims: Vec<GeosetAnimDef>,
    #[serde(default)]
    pub attachments: Vec<AttachmentDef>,
    #[serde(default)]
    pub materials: Vec<MaterialDef>,
    #[serde(default)]
    pub tracks: Vec<TrackDef>,
}

// ---- 运行时结构 ----

#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub pivot: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct Sequence {
    pub name: String,
    pub start: f32,
    pub end: f32,
    pub fps: f32,
    pub looping: bool,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub bone: usize,
    pub times: Vec<f32>,
    pub translations: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
}

#[derive(Clone, Debug)]
pub struct GeosetAnim {
    pub geoset: u32,
    pub alpha: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub bone: usize,
    pub offset: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct Material {
    pub filter_mode: FilterMode,
    pub texture: Option<String>,
    pub team_color: bool,
}

/// 应用队伍色之后的材质，供 GPU 管线使用。
#[derive(Clone, Debug)]
pub struct ResolvedMaterial {
    pub filter_mode: FilterMode,
    pub texture: Option<String>,
    pub team_color: [f32; 4],
}

impl Material {
    pub fn resolve(&self, owner: u8) -> ResolvedMaterial {
        ResolvedMaterial {
            filter_mode: self.filter_mode,
            texture: self.texture.clone(),
            team_color: if self.team_color {
                team_color(owner)
            } else {
                [1.0; 4]
            },
        }
    }
}

/// 12 支队伍的队伍色（RGBA）。索引按 `owner % 12` 取。
pub const TEAM_COLORS: [[f32; 4]; 12] = [
    [0.89, 0.11, 0.11, 1.0],
    [0.16, 0.35, 0.94, 1.0],
    [0.12, 0.75, 0.35, 1.0],
    [0.55, 0.25, 0.75, 1.0],
    [0.98, 0.95, 0.20, 1.0],
    [0.98, 0.55, 0.10, 1.0],
    [0.20, 0.80, 0.85, 1.0],
    [0.90, 0.35, 0.70, 1.0],
    [0.60, 0.60, 0.60, 1.0],
    [0.45, 0.70, 0.95, 1.0],
    [0.20, 0.45, 0.20, 1.0],
    [0.55, 0.30, 0.15, 1.0],
];

/// 按 `owner` 取队伍色；越界回绕，绝不 panic。
pub fn team_color(owner: u8) -> [f32; 4] {
    TEAM_COLORS[owner as usize % TEAM_COLORS.len()]
}

/// 解析后的模型：骨骼层级、动画序列、关键帧轨道、挂点与材质。
#[derive(Clone, Debug, Default)]
pub struct Model {
    pub bones: Vec<Bone>,
    pub sequences: Vec<Sequence>,
    pub tracks: Vec<Track>,
    pub attachments: BTreeMap<String, Attachment>,
    pub geoset_anims: Vec<GeosetAnim>,
    pub materials: Vec<Material>,
}

fn valid_optional_len(len: usize, times: usize) -> bool {
    len == 0 || len == times
}

/// 取关键帧区间并线性插值 TRS。缺帧 / 空轨道降级为单位矩阵或默认值，
/// 不抛错，保证动画异常不影响呈现也不影响模拟。
fn sample_track(track: &Track, frame: f32) -> Mat4 {
    let times = &track.times;
    let mut k = 0usize;
    while k + 1 < times.len() && times[k + 1] <= frame {
        k += 1;
    }
    let vec3_at = |values: &[[f32; 3]], idx: usize, default: [f32; 3]| -> Vec3 {
        Vec3::from_array(values.get(idx).copied().unwrap_or(default))
    };
    let quat_at = |values: &[[f32; 4]], idx: usize| -> Quat {
        Quat::from_array(values.get(idx).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]))
    };

    let (translation, rotation, scale) = if k + 1 < times.len() && times[k + 1] > times[k] {
        let alpha = ((frame - times[k]) / (times[k + 1] - times[k])).clamp(0.0, 1.0);
        (
            vec3_at(&track.translations, k, [0.0; 3])
                .lerp(vec3_at(&track.translations, k + 1, [0.0; 3]), alpha),
            quat_at(&track.rotations, k).slerp(quat_at(&track.rotations, k + 1), alpha),
            vec3_at(&track.scales, k, [1.0; 3])
                .lerp(vec3_at(&track.scales, k + 1, [1.0; 3]), alpha),
        )
    } else {
        (
            vec3_at(&track.translations, k, [0.0; 3]),
            quat_at(&track.rotations, k),
            vec3_at(&track.scales, k, [1.0; 3]),
        )
    };
    Mat4::from_scale_rotation_translation(scale, rotation, translation)
}

/// 递归累积父链矩阵；`visiting` 防止异常数据里的环导致无限递归。
fn global_matrix(
    index: usize,
    bones: &[Bone],
    locals: &[Mat4],
    cache: &mut [Option<Mat4>],
    visiting: &mut [bool],
) -> Mat4 {
    if let Some(matrix) = cache[index] {
        return matrix;
    }
    if visiting[index] {
        return locals[index];
    }
    visiting[index] = true;
    let local = locals[index];
    let matrix = match bones[index].parent {
        Some(parent) if parent < bones.len() && parent != index => {
            global_matrix(parent, bones, locals, cache, visiting) * local
        }
        _ => local,
    };
    visiting[index] = false;
    cache[index] = Some(matrix);
    matrix
}

impl Model {
    /// 从上游转换规格 JSON 文本解析模型。
    pub fn from_json(text: &str) -> Result<Self, RenderError> {
        let doc: ModelDocument = serde_json::from_str(text)?;
        let bones: Vec<Bone> = doc
            .bones
            .iter()
            .map(|b| Bone {
                name: b.name.clone(),
                parent: b.parent,
                pivot: b.pivot,
            })
            .collect();
        let sequences: Vec<Sequence> = doc
            .sequences
            .iter()
            .map(|s| Sequence {
                name: s.name.clone(),
                start: s.start,
                end: s.end,
                fps: s.fps,
                looping: s.looping,
            })
            .collect();

        let mut tracks = Vec::new();
        for def in &doc.tracks {
            if def.times.is_empty() || def.bone >= bones.len() {
                log::warn!(target: "w3_render", "track skipped: bone={} reason=invalid_index", def.bone);
                continue;
            }
            let times = def.times.len();
            if !valid_optional_len(def.translations.len(), times)
                || !valid_optional_len(def.rotations.len(), times)
                || !valid_optional_len(def.scales.len(), times)
            {
                return Err(RenderError::Model(format!(
                    "track bone={} has mismatched keyframe arrays",
                    def.bone
                )));
            }
            tracks.push(Track {
                bone: def.bone,
                times: def.times.clone(),
                translations: def.translations.clone(),
                rotations: def.rotations.clone(),
                scales: def.scales.clone(),
            });
        }

        let mut attachments = BTreeMap::new();
        for def in &doc.attachments {
            if def.bone >= bones.len() {
                log::warn!(target: "w3_render", "attachment skipped: name={} reason=invalid_bone", def.name);
                continue;
            }
            attachments.insert(
                def.name.clone(),
                Attachment {
                    bone: def.bone,
                    offset: def.offset,
                },
            );
        }

        let geoset_anims = doc
            .geoset_anims
            .iter()
            .map(|g| GeosetAnim {
                geoset: g.geoset,
                alpha: g.alpha,
                color: g.color,
            })
            .collect();
        let materials = doc
            .materials
            .iter()
            .map(|m| Material {
                filter_mode: m.filter_mode.unwrap_or_default(),
                texture: m.texture.clone(),
                team_color: m.team_color,
            })
            .collect();

        Ok(Self {
            bones,
            sequences,
            tracks,
            attachments,
            geoset_anims,
            materials,
        })
    }

    /// 从本机文件加载；缺失 / 不可读时返回 `RenderError::Asset`，
    /// 由调用方降级为"单位不显示"。
    pub fn from_path(path: &Path) -> Result<Self, RenderError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| RenderError::Asset(format!("path={} error={}", path.display(), e)))?;
        Self::from_json(&text)
    }

    /// 动画序列时长（秒）。
    pub fn sequence_duration(&self, sequence: usize) -> f32 {
        match self.sequences.get(sequence) {
            Some(seq) => {
                let fps = if seq.fps > 0.0 { seq.fps } else { DEFAULT_FPS };
                (seq.end - seq.start).max(0.0) / fps
            }
            None => 0.0,
        }
    }

    /// 按名字查找序列索引，用于事件驱动的动画切换。
    pub fn sequence_index_by_name(&self, name: &str) -> Option<usize> {
        self.sequences.iter().position(|s| s.name == name)
    }

    fn sequence_frame(&self, sequence: usize, time_seconds: f32) -> f32 {
        let seq = match self.sequences.get(sequence) {
            Some(seq) => seq,
            None => return 0.0,
        };
        let fps = if seq.fps > 0.0 { seq.fps } else { DEFAULT_FPS };
        let span = (seq.end - seq.start).max(1.0);
        let elapsed = time_seconds.max(0.0) * fps;
        if seq.looping {
            seq.start + (elapsed % span)
        } else {
            seq.start + elapsed.min(span)
        }
    }

    /// 在给定表现层时刻采样所有骨骼的全局矩阵（长度 = `bones.len()`）。
    /// 纯函数，不依赖墙钟、不写回任何状态。
    pub fn sample(&self, sequence: usize, time_seconds: f32) -> Vec<Mat4> {
        let count = self.bones.len();
        if count == 0 {
            return Vec::new();
        }
        let frame = self.sequence_frame(sequence, time_seconds);
        let mut locals = vec![Mat4::IDENTITY; count];
        for track in &self.tracks {
            if track.bone < count {
                locals[track.bone] = sample_track(track, frame);
            }
        }
        let mut cache = vec![None; count];
        let mut visiting = vec![false; count];
        (0..count)
            .map(|i| global_matrix(i, &self.bones, &locals, &mut cache, &mut visiting))
            .collect()
    }

    /// 挂点世界坐标：用骨骼全局矩阵变换挂点偏移。
    pub fn attachment_world(&self, name: &str, globals: &[Mat4]) -> Option<Vec3> {
        let attachment = self.attachments.get(name)?;
        let matrix = globals.get(attachment.bone)?;
        Some(matrix.transform_point3(Vec3::from_array(attachment.offset)))
    }
}

/// 本机模型库：`key → Model`、`owner → key` 与 `实体 ID → key` 绑定。
/// 资源缺失时 `resolve` 返回 `None`，实例降级为不显示（spec Scenario）。
#[derive(Default)]
pub struct ModelLibrary {
    models: BTreeMap<String, Model>,
    bindings: BTreeMap<u8, String>,
    /// 按实体 ID 绑定的类型标签（表现层专属，不进权威状态）。
    entity_bindings: BTreeMap<u64, String>,
}

impl ModelLibrary {
    pub fn insert(&mut self, key: impl Into<String>, model: Model) {
        self.models.insert(key.into(), model);
    }

    pub fn bind_team(&mut self, owner: u8, key: &str) {
        self.bindings.insert(owner, key.to_string());
    }

    /// 按实体 ID 绑定类型标签，使不同单位类型呈现不同几何；
    /// 该绑定是纯表现层数据，MUST NOT 进入权威状态、状态哈希或快照。
    pub fn bind_entity(&mut self, entity_id: u64, key: &str) {
        self.entity_bindings.insert(entity_id, key.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&Model> {
        self.models.get(key)
    }

    /// 把实体映射到模型 key：优先按实体 ID 的类型绑定，其次按 `owner`
    /// 的队伍绑定，最后回退 `"default"`；库里没有该 key 时返回 `None`。
    pub fn resolve(&self, entity: &EntityView) -> Option<String> {
        let key = self
            .entity_bindings
            .get(&entity.id)
            .or_else(|| self.bindings.get(&entity.owner))
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        self.models.contains_key(&key).then_some(key)
    }

    /// 从本机目录加载所有 `*.json` 模型，文件名（去扩展名）作为 key。
    /// 单个非法文件只记录警告并跳过，不阻断其余资源。
    pub fn load_dir(&mut self, dir: &Path) -> Result<usize, RenderError> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| RenderError::Asset(format!("dir={} error={}", dir.display(), e)))?;
        let mut loaded = 0usize;
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let key = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            match Model::from_path(&path) {
                Ok(model) => {
                    self.models.insert(key, model);
                    loaded += 1;
                }
                Err(error) => {
                    log::warn!(target: "w3_render", "model skipped: key={} error={}", key, error);
                }
            }
        }
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{
        "bones": [
            {"name": "root", "parent": null, "pivot": [0,0,0]},
            {"name": "hand", "parent": 0, "pivot": [1,0,0]}
        ],
        "sequences": [
            {"name": "stand", "start": 0, "end": 30, "fps": 30, "looping": true}
        ],
        "attachments": [
            {"name": "weapon", "bone": 1, "offset": [0,0,1]}
        ],
        "materials": [
            {"filterMode": "Blend", "teamColor": true}
        ],
        "geosetAnims": [
            {"geoset": 0, "alpha": 1.0}
        ],
        "tracks": [
            {"bone": 0, "times": [0, 30],
             "translations": [[0,0,0],[1,0,0]],
             "rotations": [[0,0,0,1],[0,0,0,1]],
             "scales": [[1,1,1],[1,1,1]]}
        ]
    }"#;

    fn entity(owner: u8) -> EntityView {
        EntityView {
            id: 1,
            x: 0,
            y: 0,
            facing: 0,
            hp: 100,
            owner,
            alive: true,
            has_target: false,
        }
    }

    #[test]
    fn parses_document_and_samples_animation() {
        let model = Model::from_json(DOC).unwrap();
        assert_eq!(model.bones.len(), 2);
        assert_eq!(model.sequences.len(), 1);
        assert_eq!(model.attachments.len(), 1);
        assert_eq!(model.materials[0].filter_mode, FilterMode::Blend);
        assert!(model.sequence_duration(0) - 1.0 < 1e-6);

        let at_start = model.sample(0, 0.0);
        let at_mid = model.sample(0, 0.5);
        let start_x = at_start[0].to_cols_array()[12];
        let mid_x = at_mid[0].to_cols_array()[12];
        assert!(start_x.abs() < 1e-6, "start_x={start_x}");
        assert!((mid_x - 0.5).abs() < 1e-4, "mid_x={mid_x}");

        // 循环：2 秒回绕到接近起点。
        let wrapped = model.sample(0, 2.0);
        assert!(wrapped[0].to_cols_array()[12].abs() < 1e-6);
    }

    #[test]
    fn attachment_follows_bone_matrix() {
        let model = Model::from_json(DOC).unwrap();
        let globals = model.sample(0, 0.0);
        let point = model.attachment_world("weapon", &globals).unwrap();
        assert!((point.x).abs() < 1e-6);
        assert!((point.z - 1.0).abs() < 1e-6);
        assert!(model.attachment_world("missing", &globals).is_none());
    }

    #[test]
    fn team_color_and_material_resolution() {
        assert_eq!(team_color(0), TEAM_COLORS[0]);
        assert_eq!(team_color(12), TEAM_COLORS[0]);
        let model = Model::from_json(DOC).unwrap();
        let resolved = model.materials[0].resolve(2);
        assert_eq!(resolved.filter_mode, FilterMode::Blend);
        assert!(resolved.filter_mode.is_transparent());
        assert_eq!(resolved.team_color, team_color(2));
    }

    #[test]
    fn missing_and_bound_models() {
        let mut library = ModelLibrary::default();
        assert_eq!(library.resolve(&entity(0)), None);
        library.insert("default", Model::from_json(DOC).unwrap());
        assert_eq!(library.resolve(&entity(0)), Some("default".into()));
        library.bind_team(0, "footman");
        assert_eq!(library.resolve(&entity(0)), None);
        library.insert("footman", Model::default());
        assert_eq!(library.resolve(&entity(0)), Some("footman".into()));
    }

    #[test]
    fn mismatched_keyframe_arrays_are_rejected() {
        let bad = DOC.replace("[[0,0,0],[1,0,0]]", "[[0,0,0]]");
        assert!(matches!(Model::from_json(&bad), Err(RenderError::Model(_))));
    }

    #[test]
    fn missing_file_returns_asset_error() {
        let result = Model::from_path(Path::new("does-not-exist/model.json"));
        assert!(matches!(result, Err(RenderError::Asset(_))));
    }
}
