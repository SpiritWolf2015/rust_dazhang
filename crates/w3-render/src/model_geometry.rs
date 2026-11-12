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
fn bake_submesh(sub: &SubMesh, out: &mut Vec<BakedTriangle>) {
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

        // alpha 镂空：三角形重心 UV 采样 alpha 低于阈值则整片丢弃。
        // 仅在声明 MASK 且带贴图时启用（树叶 / 旗帜等）。
        if sub.alpha_cutoff > 0.0 {
            if let Some(image) = sub.texture.as_ref() {
                let cu = (sub.uvs.get(i0).copied().unwrap_or([0.0, 0.0])[0]
                    + sub.uvs.get(i1).copied().unwrap_or([0.0, 0.0])[0]
                    + sub.uvs.get(i2).copied().unwrap_or([0.0, 0.0])[0])
                    / 3.0;
                let cv = (sub.uvs.get(i0).copied().unwrap_or([0.0, 0.0])[1]
                    + sub.uvs.get(i1).copied().unwrap_or([0.0, 0.0])[1]
                    + sub.uvs.get(i2).copied().unwrap_or([0.0, 0.0])[1])
                    / 3.0;
                if sample_uv(image, cu, cv)[3] < sub.alpha_cutoff {
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
    use w3_content::SubMesh;

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
        assert!(baked.is_empty(), "透明三角形应被 alpha 剔除");
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
}
