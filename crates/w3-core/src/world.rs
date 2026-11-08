//! 冻结的静态世界数据（由 app 层注入，见 ADR 0004）。
//!
//! `StaticWorld` 只读、纯整数，`w3-core` 不依赖 `w3-content`。地图数据本身
//! 不进入快照，只用 `identity` 在恢复时校验静态地图身份。
//!
//! 世界原点 `origin` 对应上游 `centerOffset`（地图左下角世界坐标），
//! `cell_size` 对应 `tileSize`；单位世界坐标原样 1:1 冻结，不平移。

use crate::fixed::SimFixed;
use crate::state_hash::StateHasher;

/// 无地图注入时使用的默认开放网格边长（格子数）。
pub const DEFAULT_GRID_SIZE: u32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}

impl GridCell {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 行优先索引；越界返回 `None`。
    pub fn index(self, width: u32, height: u32) -> Option<usize> {
        if self.x < 0 || self.y < 0 || self.x >= width as i32 || self.y >= height as i32 {
            return None;
        }
        Some((self.y as usize) * (width as usize) + (self.x as usize))
    }

    pub fn from_index(index: usize, width: u32) -> Self {
        Self::new(
            (index % width as usize) as i32,
            (index / width as usize) as i32,
        )
    }
}

/// 静态地图身份：FNV-1a over 显式小端编码的宽 / 高 / 格距 / 原点 /
/// 阻挡 / 高度。
fn compute_identity(
    width: u32,
    height: u32,
    cell_size: SimFixed,
    origin: (SimFixed, SimFixed),
    blocked: &[bool],
    heights: &[SimFixed],
) -> u64 {
    let mut hasher = StateHasher::new();
    hasher.write_u32(width);
    hasher.write_u32(height);
    hasher.write_i64(cell_size.raw());
    hasher.write_i64(origin.0.raw());
    hasher.write_i64(origin.1.raw());
    hasher.write_u64(blocked.len() as u64);
    for &cell in blocked {
        hasher.write_u8(u8::from(cell));
    }
    hasher.write_u64(heights.len() as u64);
    for &height in heights {
        hasher.write_i64(height.raw());
    }
    hasher.finish()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StaticWorld {
    width: u32,
    height: u32,
    cell_size: SimFixed,
    origin: (SimFixed, SimFixed),
    blocked: Vec<bool>,
    heights: Vec<SimFixed>,
    identity: u64,
}

impl StaticWorld {
    /// 构造冻结世界。阻挡 / 高度数组按 `width * height` 截断或补默认值，
    /// 多余的上游顶点由 app 层负责降采样，不在此隐式换算。
    pub fn new(
        width: u32,
        height: u32,
        cell_size: SimFixed,
        origin: (SimFixed, SimFixed),
        blocked: Vec<bool>,
        heights: Vec<SimFixed>,
    ) -> Self {
        let cells = (width as usize).saturating_mul(height as usize);
        let mut blocked = blocked;
        blocked.resize(cells, false);
        let mut heights = heights;
        heights.resize(cells, SimFixed::ZERO);
        let identity = compute_identity(width, height, cell_size, origin, &blocked, &heights);
        Self {
            width,
            height,
            cell_size,
            origin,
            blocked,
            heights,
            identity,
        }
    }

    /// 全可走、全零高度、原点在 (0, 0) 的世界。
    pub fn open_world(width: u32, height: u32, cell_size: SimFixed) -> Self {
        Self::new(
            width,
            height,
            cell_size,
            (SimFixed::ZERO, SimFixed::ZERO),
            Vec::new(),
            Vec::new(),
        )
    }

    pub fn identity(&self) -> u64 {
        self.identity
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cell_size(&self) -> SimFixed {
        self.cell_size
    }

    pub fn origin(&self) -> (SimFixed, SimFixed) {
        self.origin
    }

    pub fn is_walkable(&self, cell: GridCell) -> bool {
        match cell.index(self.width, self.height) {
            Some(index) => !self.blocked[index],
            None => false,
        }
    }

    pub fn height_at(&self, cell: GridCell) -> SimFixed {
        match cell.index(self.width, self.height) {
            Some(index) => self.heights[index],
            None => SimFixed::ZERO,
        }
    }

    /// 格子中心的世界坐标（含原点偏移）。
    pub fn cell_center(&self, cell: GridCell) -> (SimFixed, SimFixed) {
        let half = self.cell_size.raw() / 2;
        let x = self
            .origin
            .0
            .raw()
            .saturating_add((cell.x as i64).saturating_mul(self.cell_size.raw()))
            .saturating_add(half);
        let y = self
            .origin
            .1
            .raw()
            .saturating_add((cell.y as i64).saturating_mul(self.cell_size.raw()))
            .saturating_add(half);
        (SimFixed::from_raw(x), SimFixed::from_raw(y))
    }

    /// 世界坐标 → 格子；减原点后按 floor 换算，再 clamp 到有效范围。
    pub fn world_to_cell(&self, x: SimFixed, y: SimFixed) -> GridCell {
        let size = self.cell_size.raw().max(1);
        let max_x = self.width.saturating_sub(1) as i64;
        let max_y = self.height.saturating_sub(1) as i64;
        GridCell::new(
            x.raw()
                .saturating_sub(self.origin.0.raw())
                .div_euclid(size)
                .clamp(0, max_x) as i32,
            y.raw()
                .saturating_sub(self.origin.1.raw())
                .div_euclid(size)
                .clamp(0, max_y) as i32,
        )
    }
}

impl Default for StaticWorld {
    fn default() -> Self {
        Self::open_world(DEFAULT_GRID_SIZE, DEFAULT_GRID_SIZE, SimFixed::ONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCALE: i64 = 1 << 16;

    #[test]
    fn cell_index_is_row_major_and_bounds_checked() {
        let cell = GridCell::new(2, 1);
        assert_eq!(cell.index(4, 4), Some(6));
        assert_eq!(GridCell::from_index(6, 4), cell);
        assert_eq!(GridCell::new(-1, 0).index(4, 4), None);
        assert_eq!(GridCell::new(4, 0).index(4, 4), None);
    }

    #[test]
    fn different_maps_have_different_identities() {
        let zero = SimFixed::ZERO;
        let a = StaticWorld::open_world(4, 4, SimFixed::ONE);
        let b = StaticWorld::open_world(4, 4, SimFixed::from_int(2).unwrap());
        let c = StaticWorld::new(
            4,
            4,
            SimFixed::ONE,
            (zero, zero),
            vec![true; 16],
            vec![zero; 16],
        );
        let d = StaticWorld::new(
            4,
            4,
            SimFixed::ONE,
            (SimFixed::from_int(-8192).unwrap(), zero),
            Vec::new(),
            Vec::new(),
        );
        assert_ne!(a.identity(), b.identity());
        assert_ne!(a.identity(), c.identity());
        assert_ne!(a.identity(), d.identity(), "origin must affect identity");
        assert_eq!(
            a.identity(),
            StaticWorld::open_world(4, 4, SimFixed::ONE).identity()
        );
    }

    #[test]
    fn world_to_cell_and_center_roundtrip() {
        let world = StaticWorld::open_world(8, 8, SimFixed::from_int(4).unwrap());
        let cell = GridCell::new(3, 5);
        let (x, y) = world.cell_center(cell);
        assert_eq!(world.world_to_cell(x, y), cell);
        assert_eq!(x.raw(), 14 * SCALE);
    }

    #[test]
    fn origin_shifts_the_cell_mapping() {
        // Echo Isles 量级：cell_size=128，origin=(-8192,-6144)。
        let size = SimFixed::from_int(128).unwrap();
        let origin = (
            SimFixed::from_int(-8192).unwrap(),
            SimFixed::from_int(-6144).unwrap(),
        );
        let world = StaticWorld::new(128, 96, size, origin, Vec::new(), Vec::new());
        // 地图左下角 → 格 (0,0)；中心附近 → 格 (64,48)。
        assert_eq!(world.world_to_cell(origin.0, origin.1), GridCell::new(0, 0));
        let centre = world.cell_center(GridCell::new(64, 48));
        assert_eq!(
            world.world_to_cell(centre.0, centre.1),
            GridCell::new(64, 48)
        );
        assert_eq!(centre.0.raw(), (-8192 + 64 * 128 + 64) * SCALE);
    }
}
