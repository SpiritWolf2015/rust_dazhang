//! 真实 glTF 模型网格的表现层烘焙与展开（真实模型视觉阶段）。
//!
//! 上游 [`w3_content::ModelMesh`] 是中立读取结果（WC3 模型局部坐标、Y-up、
//! 带 UV 与 RGBA8 贴图）。本模块把它烘焙成**表现层三角形**：逐顶点按 UV
//! 采样真实贴图像素写入顶点色、按三角形重心 alpha 剔除树叶镂空，再按摆放
//! 位姿展开成 [`SceneVertex`]。全部是纯 CPU `f32` 派生数据，MUST NOT 进入
//! 权威状态、状态哈希或快照（ADR 0002）。
//!
//! 采用「CPU 烘焙进顶点色」而非 GPU 逐像素纹理采样，是为保持既有单管线 /
//! 单 draw call / `SceneVertex` 布局零改动（前几轮反复黑屏的经验），同时
//! 让真实模型几何与真实贴图颜色上屏；逐像素过滤与骨骼蒙皮留后续阶段。

use w3_content::{ModelMesh, RgbaImage, SubMesh};

use crate::scene::SceneVertex;

/// glTF → 玩法世界坐标缩放：与 `real_map` 的 `WORLD_SCALE = 2/128` 一致。
/// glTF `POSITION` 是 WC3 模型单位（1 tile = 128），玩法世界 1 tile = 2。
pub const MODEL_WORLD_SCALE: f32 = 2.0 / 128.0;

/// 真实模型的**表现层显示放大系数**。
///
/// WC3 模型按 `MODEL_WORLD_SCALE` 换算后是真实比例（农民约 2 世界单位、主城
/// 约 18），但既有玩法世界把整张 EI 地图压到 256 单位跨度、默认 `zoom=1` 相机
/// 俯瞰全图时，2 单位的农民只有几个像素，肉眼无法分辨是不是模型。此前程序化
/// 四面体用 `UNIT_MARKER_SIZE=24` 是刻意放大的占位符，反而显眼。
///
/// 为让真实 WC3 模型在 RTS 局部视角下清晰可辨，静态摆放与动态单位装配统一
/// 乘此系数（纯表现层，不进权威状态 / 哈希 / 快照）。2.0 由离线离屏渲染对比
/// 选定：树冠可见、密度可接受、又不过大糊屏。
pub const MODEL_DISPLAY_SCALE: f32 = 2.0;

/// 动态单位（农民 / 士兵 / 主城 / 兵营等可操作实体）的**额外**放大系数。
///
/// 静态摆放（树 / 岩石）用 [`MODEL_DISPLAY_SCALE`]；动态单位是玩家要选中 /
/// 操作、且数量少（开局 12 个），需要在局部战场视角下更醒目。总放大 =
/// `MODEL_DISPLAY_SCALE × ENTITY_DISPLAY_SCALE`（=4）。由离线离屏渲染对比
/// 选定：城堡尖顶 / 人形农民清晰可辨、又不过大糊屏。纯表现层参数。
pub const ENTITY_DISPLAY_SCALE: f32 = 2.0;

/// 无贴图 / 采样失败时的中性顶点色（不透明灰白，不做纯色块假象）。
fn neutral_color(base: [f32; 4]) -> [f32; 4] {
    [base[0] * 0.78, base[1] * 0.78, base[2] * 0.78, base[3]]
}

/// 按 UV 采样一张 RGBA8 贴图，返回线性化前的 `[0,1]` RGBA。
/// UV 走 glTF 约定（左上原点、V 向下、可 repeat），越界安全、确定性。
fn sample_uv(image: &RgbaImage, u: f32, v: f32) -> [f32; 4] {
    if image.width == 0 || image.height == 0 || image.pixels.len() < 4 {
        return [1.0, 1.0, 1.0, 1.0];
    }
    let fu = u.rem_euclid(1.0);
    let fv = v.rem_euclid(1.0);
    let px = ((fu * image.width as f32) as u32).min(image.width - 1);
    let py = ((fv * image.height as f32) as u32).min(image.height - 1);
    let index = (py * image.width + px) as usize * 4;
    let Some(pixel) = image.pixels.get(index..index + 4) else {
        return [1.0, 1.0, 1.0, 1.0];
    };
    [
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
        pixel[3] as f32 / 255.0,
    ]
}

/// 烘焙后的三角形：三个顶点各带局部位置、法线与已调制的顶点色。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakedTriangle {
    pub position: [[f32; 3]; 3],
    pub normal: [[f32; 3]; 3],
    pub color: [[f32; 4]; 3],
}

/// 真实模型的表现层三角形集合（局部 WC3 模型坐标，Y-up）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BakedModel {
    pub triangles: Vec<BakedTriangle>,
}

impl BakedModel {
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

/// 把一个子网格烘焙成三角形：贴图色 × baseColor，重心 alpha 剔除。
///
/// **过滤特效层 geoset**：WC3 的 Blend / Additive / UberSplat / 辉光 / 烟雾等
/// 材质是加色或 alpha 混合的特效，不是实体表面；若当不透明几何画出来会变成
/// 白色纸片 / 黑块（离线特写已复现）。只烘焙 `Opaque` / `Mask` 实体几何。
fn bake_submesh(sub: &SubMesh, out: &mut Vec<BakedTriangle>) {
    if !sub.filter_mode.is_entity() {
        return;
    }
    for tri in sub.indices.chunks_exact(3) {
        let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let idx = [i0, i1, i2];

        // 顶点色：有贴图按各自 UV 采样，无贴图用中性色；都乘材质 baseColor。
        let mut color = [[0.0f32; 4]; 3];
        for (k, &vi) in idx.iter().enumerate() {
            let sampled = match sub.texture.as_ref() {
                Some(image) => {
                    let uv = sub.uvs.get(vi).copied().unwrap_or([0.0, 0.0]);
                    sample_uv(image, uv[0], uv[1])
                }
                None => [1.0, 1.0, 1.0, 1.0],
            };
            let has_texture = sub.texture.is_some();
            let base = if has_texture {
                sampled
            } else {
                neutral_color(sub.base_color)
            };
            color[k] = [
                base[0] * sub.base_color[0],
                base[1] * sub.base_color[1],
                base[2] * sub.base_color[2],
                base[3] * sub.base_color[3],
            ];
        }

        // alpha 镂空（树叶 / 旗帜等 MASK 材质）：**仅当三个顶点 UV 采样全部
        // 透明**才整片丢弃。早期版本按「重心单点」剔除，树冠贴图大面积半透明
        // 时重心常落透明区，会误删整个树冠 billboard 面片、只剩树干细杆。
        // 保守剔除保留含树冠的面片；顶点色 alpha 已取自贴图采样，配合 pipeline
        // 的 ALPHA_BLENDING 呈现柔和镂空边缘，而非实心色块。
        if sub.alpha_cutoff > 0.0 {
            if let Some(image) = sub.texture.as_ref() {
                let all_transparent = idx.iter().all(|&vi| {
                    let uv = sub.uvs.get(vi).copied().unwrap_or([0.0, 0.0]);
                    sample_uv(image, uv[0], uv[1])[3] < sub.alpha_cutoff
                });
                if all_transparent {
                    continue;
                }
            }
        }

        let mut position = [[0.0f32; 3]; 3];
        let mut normal = [[0.0f32; 3]; 3];
        for (k, &vi) in idx.iter().enumerate() {
            position[k] = sub.positions.get(vi).copied().unwrap_or([0.0; 3]);
            normal[k] = sub.normals.get(vi).copied().unwrap_or([0.0, 1.0, 0.0]);
        }
        out.push(BakedTriangle {
            position,
            normal,
            color,
        });
    }
}

/// 把 [`ModelMesh`] 烘焙成表现层三角形（纯函数、确定性）。
pub fn bake_model(mesh: &ModelMesh) -> BakedModel {
    let mut triangles = Vec::new();
    for sub in &mesh.submeshes {
        bake_submesh(sub, &mut triangles);
    }
    BakedModel { triangles }
}

/// 摆放位姿：世界原点（已含地形高度）、绕 Y 朝向、额外缩放。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTransform {
    /// 世界坐标 `[x, terrain_height, z]`。
    pub origin: [f32; 3],
    /// 绕 Y 轴朝向（弧度）。
    pub yaw: f32,
    /// 尺寸系数（1.0 = 按导出比例）。
    pub scale: f32,
}

impl Default for ModelTransform {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            yaw: 0.0,
            scale: 1.0,
        }
    }
}

/// glTF 局部顶点 → 玩法世界坐标。
///
/// 上游 `wc3ToGltfVec3(x, y, z) = [x, z, -y]`：glTF 的 `y` 是高度、
/// `z` 是取反的水平轴。玩法世界同为 Y-up，故 `world_z = -gltf_z`。
pub fn gltf_to_world(local: [f32; 3], transform: &ModelTransform) -> [f32; 3] {
    let s = MODEL_WORLD_SCALE * transform.scale;
    let (sin, cos) = transform.yaw.sin_cos();
    // 先按 WC3 水平轴还原（gz 取反），再绕 Y 旋转水平分量。
    let lx = local[0] * s;
    let ly = local[1] * s;
    let lz = -local[2] * s;
    let rx = lx * cos - lz * sin;
    let rz = lx * sin + lz * cos;
    [
        transform.origin[0] + rx,
        transform.origin[1] + ly,
        transform.origin[2] + rz,
    ]
}

/// 把烘焙模型按位姿展开成 [`SceneVertex`]（`surface = -1` 单位标记）。
/// 法线同步随 yaw 旋转，保证光照方向正确。
pub fn expand_model(model: &BakedModel, transform: &ModelTransform) -> Vec<SceneVertex> {
    let (sin, cos) = transform.yaw.sin_cos();
    let mut vertices = Vec::with_capacity(model.triangles.len() * 3);
    for tri in &model.triangles {
        for k in 0..3 {
            let n = tri.normal[k];
            vertices.push(SceneVertex {
                position: gltf_to_world(tri.position[k], transform),
                normal: [n[0] * cos - n[2] * sin, n[1], n[0] * sin + n[2] * cos],
                color: tri.color[k],
                surface: -1.0,
            });
        }
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_content::{GeosetFilter, SubMesh};

    fn solid_image(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
        RgbaImage {
            width: w,
            height: h,
            pixels: rgba.repeat((w * h) as usize),
        }
    }

    fn simple_submesh() -> SubMesh {
        SubMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![0, 1, 2],
            base_color: [1.0, 1.0, 1.0, 1.0],
            alpha_cutoff: 0.0,
            double_sided: false,
            filter_mode: GeosetFilter::Opaque,
            texture: None,
            texture_uri: None,
        }
    }

    #[test]
    fn bake_produces_one_triangle_with_neutral_color_without_texture() {
        let mesh = ModelMesh {
            submeshes: vec![simple_submesh()],
            node_scale: [1.0; 3],
        };
        let baked = bake_model(&mesh);
        assert_eq!(baked.triangle_count(), 1);
        // 无贴图走中性色而非纯白/纯色块。
        assert!(baked.triangles[0].color[0][0] < 1.0);
        assert!(baked.triangles[0].color[0][0] > 0.0);
    }

    #[test]
    fn bake_samples_real_texture_pixels() {
        let mut sub = simple_submesh();
        sub.texture = Some(solid_image(2, 2, [200, 100, 50, 255]));
        let baked = bake_model(&ModelMesh {
            submeshes: vec![sub],
            node_scale: [1.0; 3],
        });
        let c = baked.triangles[0].color[0];
        assert!((c[0] - 200.0 / 255.0).abs() < 1e-6);
        assert!((c[1] - 100.0 / 255.0).abs() < 1e-6);
        assert!((c[2] - 50.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn bake_drops_fully_transparent_masked_triangles() {
        let mut sub = simple_submesh();
        sub.alpha_cutoff = 0.75;
        sub.texture = Some(solid_image(2, 2, [0, 255, 0, 0])); // 全透明
        let baked = bake_model(&ModelMesh {
            submeshes: vec![sub],
            node_scale: [1.0; 3],
        });
        assert!(baked.is_empty(), "全透明三角形应被 alpha 剔除");
    }

    /// 构造「左列不透明 / 右列全透明」的 2×2 树冠贴图（`u<0.5` 不透明）。
    fn canopy_texture() -> RgbaImage {
        let mut pixels = Vec::new();
        for _ in 0..2 {
            pixels.extend_from_slice(&[30, 140, 40, 255]); // 树冠：不透明
            pixels.extend_from_slice(&[0, 0, 0, 0]); // 间隙：全透明
        }
        RgbaImage {
            width: 2,
            height: 2,
            pixels,
        }
    }

    #[test]
    fn bake_keeps_tree_canopy_when_centroid_is_transparent_but_a_vertex_is_opaque() {
        // 关键回归：树冠 billboard 面片**重心落透明区、但至少一个顶点落不透明
        // 树冠区**。早期「按重心单点 alpha 剔除」会据此删掉整片（只剩树干细杆），
        // 新的「三顶点全透明才删」保守剔除必须保留它。UV 构造刻意让两种逻辑
        // 结果相反，才能真正捕获回归（critic B1）。
        let mut sub = simple_submesh();
        sub.filter_mode = GeosetFilter::Mask;
        sub.alpha_cutoff = 0.75;
        let texture = canopy_texture();
        sub.texture = Some(texture.clone());
        // 两顶点在右列(u=0.9 透明)、一顶点在左列(u=0.1 不透明)：
        // 重心 cu=(0.9+0.9+0.1)/3=0.633 → 落透明列；但存在不透明顶点。
        sub.uvs = vec![[0.9, 0.25], [0.9, 0.75], [0.1, 0.5]];

        // 元断言：证明构造在旧「重心剔除」逻辑下会被删、在新逻辑下应保留。
        let cu = (sub.uvs[0][0] + sub.uvs[1][0] + sub.uvs[2][0]) / 3.0;
        let cv = (sub.uvs[0][1] + sub.uvs[1][1] + sub.uvs[2][1]) / 3.0;
        assert!(
            sample_uv(&texture, cu, cv)[3] < sub.alpha_cutoff,
            "重心必须落透明区（否则旧逻辑也保留，测试失去判别力）"
        );
        assert!(
            sub.uvs
                .iter()
                .any(|uv| sample_uv(&texture, uv[0], uv[1])[3] >= sub.alpha_cutoff),
            "至少一个顶点必须落不透明树冠区"
        );

        let baked = bake_model(&ModelMesh {
            submeshes: vec![sub],
            node_scale: [1.0; 3],
        });
        assert_eq!(baked.triangle_count(), 1, "含树冠的面片不得被误删");
    }

    #[test]
    fn bake_still_drops_triangle_with_all_vertices_transparent() {
        // 反向保护：保守剔除不能退化成「永不删」。三顶点 UV 全落透明列 → 删。
        let mut sub = simple_submesh();
        sub.filter_mode = GeosetFilter::Mask;
        sub.alpha_cutoff = 0.75;
        sub.texture = Some(canopy_texture());
        sub.uvs = vec![[0.9, 0.25], [0.9, 0.75], [0.9, 0.5]]; // 全透明列
        let baked = bake_model(&ModelMesh {
            submeshes: vec![sub],
            node_scale: [1.0; 3],
        });
        assert!(baked.is_empty(), "全透明三角形仍应被剔除");
    }

    #[test]
    fn gltf_to_world_flips_z_and_scales() {
        let t = ModelTransform::default();
        // 局部 [128, 256, -128] → 世界 [2, 4, 2]（gz 取反、×2/128）。
        let w = gltf_to_world([128.0, 256.0, -128.0], &t);
        assert!((w[0] - 2.0).abs() < 1e-5);
        assert!((w[1] - 4.0).abs() < 1e-5);
        assert!((w[2] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn expand_is_deterministic_and_marks_surface() {
        let mesh = ModelMesh {
            submeshes: vec![simple_submesh()],
            node_scale: [1.0; 3],
        };
        let baked = bake_model(&mesh);
        let t = ModelTransform {
            origin: [10.0, 2.0, -5.0],
            yaw: 0.3,
            scale: 1.2,
        };
        let a = expand_model(&baked, &t);
        let b = expand_model(&baked, &t);
        assert_eq!(a, b, "模型展开必须可复现");
        assert_eq!(a.len(), 3);
        assert!(a.iter().all(|v| v.surface < 0.0));
        assert!(a.iter().all(|v| v.position.iter().all(|c| c.is_finite())));
        // 顶点被平移到 origin 附近。
        assert!(a[0].position[0] > 9.0);
    }

    #[test]
    fn sample_uv_is_repeatable_and_oob_safe() {
        let image = solid_image(2, 2, [10, 20, 30, 255]);
        for (u, v) in [(-3.0, 7.0), (0.5, 0.5), (1e6, -1e6)] {
            let c = sample_uv(&image, u, v);
            assert!(c.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)));
        }
    }

    #[test]
    fn effect_geosets_are_filtered_out() {
        // 一个实体 geoset + 所有特效 geoset 类型，后者应全部被烘焙层跳过。
        let entity = simple_submesh();
        let effects: Vec<SubMesh> = [
            GeosetFilter::Blend,
            GeosetFilter::Additive,
            GeosetFilter::AddAlpha,
            GeosetFilter::Modulate,
            GeosetFilter::Modulate2x,
        ]
        .iter()
        .map(|&filter_mode| {
            let mut sub = simple_submesh();
            sub.filter_mode = filter_mode;
            // 特效层贴白色不透明纹理（正是白色纸片 / 黑块的成因）。
            sub.texture = Some(solid_image(2, 2, [255, 255, 255, 255]));
            sub
        })
        .collect();
        let mut submeshes = vec![entity];
        submeshes.extend(effects);
        let baked = bake_model(&ModelMesh {
            submeshes,
            node_scale: [1.0; 3],
        });
        assert_eq!(
            baked.triangle_count(),
            1,
            "只应保留 1 个实体 geoset 的三角形，特效层全部剔除"
        );
    }

    #[test]
    fn mask_entity_geoset_is_kept() {
        // MASK（树叶 / 旗帜）是实体几何，不能被误当特效剔除。
        let mut sub = simple_submesh();
        sub.filter_mode = GeosetFilter::Mask;
        sub.alpha_cutoff = 0.75;
        sub.texture = Some(solid_image(2, 2, [20, 120, 30, 255])); // 全不透明
        let baked = bake_model(&ModelMesh {
            submeshes: vec![sub],
            node_scale: [1.0; 3],
        });
        assert_eq!(baked.triangle_count(), 1, "MASK 实体应保留");
    }
}
