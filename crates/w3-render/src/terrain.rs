//! 从冻结 `TerrainHeightfield` 构建 3D 地形网格（纯 CPU，只读）。
//!
//! 高度场的 `f64` 顶点是冻结输入；本模块只在表现层把它转成 `f32` 顶点，
//! 不修改源数据，也不写入任何权威状态（见 ADR 0002 / 0004）。悬崖层与
//! 水体作为顶点属性保留，由着色器按导出数据呈现。

use std::collections::BTreeSet;

use w3_content::TerrainHeightfield;

use crate::error::RenderError;

/// WC3 标准瓦片边长（上游 `tileSize` 缺失时的回退值）。
const DEFAULT_TILE_SIZE: f32 = 128.0;

/// 地形顶点，Y-up 右手系：`position = [x, height, z]`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// 地表纹理索引（来自 `groundTextures`）。
    pub surface: f32,
    /// 悬崖纹理索引（来自 `cliffTextures`）。
    pub cliff: f32,
    /// 层高（来自 `layerHeights`）。
    pub layer: f32,
    /// 水体高度（来自 `waterHeights`）。
    pub water: f32,
}

impl Default for TerrainVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            surface: 0.0,
            cliff: 0.0,
            layer: 0.0,
            water: 0.0,
        }
    }
}

/// 由冻结高度场生成的 3D 地形网格；顶点与索引均为表现层数据。
#[derive(Clone, Debug)]
pub struct TerrainMesh {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub columns: u32,
    pub rows: u32,
    pub cell_size: f32,
    pub origin: [f32; 2],
    /// `layerHeights` 中不同层高的数量，用于悬崖层呈现。
    pub cliff_layers: u32,
    /// 水体最高位，用于水体呈现。
    pub water_level: f32,
}

fn read_i64(values: &[i64], index: usize) -> f32 {
    values.get(index).copied().unwrap_or(0) as f32
}

fn read_f64(values: &[f64], index: usize) -> f32 {
    values.get(index).copied().unwrap_or(0.0) as f32
}

/// 取邻居高度，越界时钳到网格边缘，保证边界法线也有定义。
fn neighbour(hf: &TerrainHeightfield, ix: u32, iy: u32, columns: u32, rows: u32) -> f32 {
    let cx = ix.min(columns - 1);
    let cy = iy.min(rows - 1);
    hf.height_at(cx as usize, cy as usize).unwrap_or(0.0) as f32
}

/// 中心差分法线；`+Y` 为向上分量。
fn compute_normal(
    hf: &TerrainHeightfield,
    ix: u32,
    iy: u32,
    columns: u32,
    rows: u32,
    cell: f32,
) -> [f32; 3] {
    let left = neighbour(hf, ix.saturating_sub(1), iy, columns, rows);
    let right = neighbour(hf, (ix + 1).min(columns - 1), iy, columns, rows);
    let down = neighbour(hf, ix, iy.saturating_sub(1), columns, rows);
    let up = neighbour(hf, ix, (iy + 1).min(rows - 1), columns, rows);
    let nx = left - right;
    let ny = 2.0 * cell;
    let nz = down - up;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 0.0 {
        [nx / len, ny / len, nz / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

impl TerrainMesh {
    /// 从只读高度场生成网格。校验失败时返回 `RenderError::Terrain`，
    /// 不会修改传入的 `TerrainHeightfield`。
    pub fn from_heightfield(hf: &TerrainHeightfield) -> Result<Self, RenderError> {
        let expected = hf
            .validate()
            .map_err(|e| RenderError::Terrain(format!("{e:?}")))?;
        if expected == 0 {
            return Err(RenderError::Terrain("empty heightfield".into()));
        }
        let columns = u32::try_from(hf.tilepoint_width)
            .map_err(|_| RenderError::Terrain("tilepointWidth overflow".into()))?;
        let rows = u32::try_from(hf.tilepoint_height)
            .map_err(|_| RenderError::Terrain("tilepointHeight overflow".into()))?;

        let raw_cell = hf.tile_size.unwrap_or(DEFAULT_TILE_SIZE as f64) as f32;
        let cell = if raw_cell > 0.0 {
            raw_cell
        } else {
            DEFAULT_TILE_SIZE
        };
        let origin = match &hf.center_offset {
            Some(offset) => [offset.x as f32, offset.y as f32],
            None => [0.0, 0.0],
        };

        let mut vertices = Vec::with_capacity(expected);
        for iy in 0..rows {
            for ix in 0..columns {
                let index = (iy as usize) * hf.tilepoint_width + ix as usize;
                vertices.push(TerrainVertex {
                    position: [
                        origin[0] + ix as f32 * cell,
                        hf.heights[index] as f32,
                        origin[1] + iy as f32 * cell,
                    ],
                    normal: compute_normal(hf, ix, iy, columns, rows, cell),
                    surface: read_i64(&hf.ground_textures, index),
                    cliff: read_i64(&hf.cliff_textures, index),
                    layer: read_i64(&hf.layer_heights, index),
                    water: read_f64(&hf.water_heights, index),
                });
            }
        }

        let mut indices = Vec::new();
        if columns >= 2 && rows >= 2 {
            indices.reserve((columns as usize - 1) * (rows as usize - 1) * 6);
            for iy in 0..rows - 1 {
                for ix in 0..columns - 1 {
                    let a = iy * columns + ix;
                    let b = a + 1;
                    let c = a + columns;
                    let d = c + 1;
                    // 绕序保证朝上（+Y）法线，从 +Y 俯视为逆时针。
                    indices.extend_from_slice(&[a, c, b, b, c, d]);
                }
            }
        }

        let cliff_layers = hf
            .layer_heights
            .iter()
            .copied()
            .collect::<BTreeSet<i64>>()
            .len() as u32;
        let water_level = hf.water_heights.iter().copied().fold(0.0_f64, f64::max) as f32;

        Ok(Self {
            vertices,
            indices,
            columns,
            rows,
            cell_size: cell,
            origin,
            cliff_layers,
            water_level,
        })
    }

    /// 世界坐标 `(x, z)` 处的双线性插值高度，用于把单位贴到地形上。
    pub fn height_at_world(&self, x: f32, z: f32) -> f32 {
        let cols = self.columns as usize;
        if self.vertices.is_empty() {
            return 0.0;
        }
        if self.columns < 2 || self.rows < 2 {
            return self.vertices[0].position[1];
        }
        let fx = ((x - self.origin[0]) / self.cell_size).clamp(0.0, (self.columns - 1) as f32);
        let fz = ((z - self.origin[1]) / self.cell_size).clamp(0.0, (self.rows - 1) as f32);
        let ix0 = fx.floor() as usize;
        let iz0 = fz.floor() as usize;
        let ix1 = (ix0 + 1).min(cols - 1);
        let iz1 = (iz0 + 1).min(self.rows as usize - 1);
        let tx = fx - ix0 as f32;
        let tz = fz - iz0 as f32;
        let height = |ix: usize, iz: usize| self.vertices[iz * cols + ix].position[1];
        let h0 = height(ix0, iz0) * (1.0 - tx) + height(ix1, iz0) * tx;
        let h1 = height(ix0, iz1) * (1.0 - tx) + height(ix1, iz1) * tx;
        h0 * (1.0 - tz) + h1 * tz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "tilepointWidth": 3, "tilepointHeight": 3,
        "mapWidth": 2, "mapHeight": 2,
        "heights": [0,1,2,3,4,5,6,7,8],
        "groundTextures": [0,1,0,1,0,1,0,1,0],
        "cliffTextures": [0,0,1,1,0,0,1,1,0],
        "layerHeights": [0,0,0,1,1,1,2,2,2],
        "waterHeights": [0,0,0,0,0,0,1,1,1]
    }"#;

    fn parse(json: &str) -> TerrainHeightfield {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn builds_grid_vertices_and_indices() {
        let hf = parse(VALID);
        let mesh = TerrainMesh::from_heightfield(&hf).unwrap();
        assert_eq!(mesh.vertices.len(), 9);
        assert_eq!(mesh.indices.len(), 24);
        assert_eq!(mesh.columns, 3);
        assert_eq!(mesh.rows, 3);
        assert_eq!(mesh.vertices[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices[1].position, [128.0, 1.0, 0.0]);
        assert_eq!(mesh.vertices[3].position, [0.0, 3.0, 128.0]);
        assert_eq!(mesh.indices[..6], [0, 3, 1, 1, 3, 4]);
        assert_eq!(mesh.cliff_layers, 3);
        assert_eq!(mesh.water_level, 1.0);
        for v in &mesh.vertices {
            assert!(v.normal[1] > 0.0, "法线必须朝上");
        }
    }

    #[test]
    fn source_heightfield_is_not_modified() {
        let hf = parse(VALID);
        let before = hf.heights.clone();
        let _ = TerrainMesh::from_heightfield(&hf).unwrap();
        assert_eq!(hf.heights, before);
        assert_eq!(hf.tilepoint_width, 3);
    }

    #[test]
    fn invalid_array_length_is_rejected() {
        let hf = parse(&VALID.replace("[0,1,2,3,4,5,6,7,8]", "[0,1,2]"));
        assert!(matches!(
            TerrainMesh::from_heightfield(&hf),
            Err(RenderError::Terrain(_))
        ));
    }

    #[test]
    fn bilinear_height_is_interpolated() {
        let hf = parse(VALID);
        let mesh = TerrainMesh::from_heightfield(&hf).unwrap();
        // 四个角 0,1,3,4 在 (64,64) 处的均值为 2.0。
        let h = mesh.height_at_world(64.0, 64.0);
        assert!((h - 2.0).abs() < 1e-4, "height={h}");
    }
}
