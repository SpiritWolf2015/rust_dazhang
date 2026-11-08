//! 八方向网格寻路与动态占地（纯整数，见 ADR 0004）。
//!
//! 代价为整数 octile：直线 1000、斜线 1414（√2 的保守近似）；
//! 启发式 `1000·max + 414·min` 保持可采纳。开放集用带显式索引的
//! `BinaryHeap`，平局由 `(score, index, cost)` 全序打破，与平台无关。

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::world::{GridCell, StaticWorld};

pub const STRAIGHT_COST: i64 = 1000;
pub const DIAG_COST: i64 = 1414;
/// 单次寻路的默认展开节点预算（含起终点）。
pub const DEFAULT_NODE_BUDGET: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GridArea {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl GridArea {
    /// 矩形左上角包含、右下角不包含。
    pub fn contains(&self, cell: GridCell) -> bool {
        cell.x >= self.x
            && cell.y >= self.y
            && cell.x < self.x.saturating_add(self.w)
            && cell.y < self.y.saturating_add(self.h)
    }
}

/// 稳定 ID 顺序的动态占地，由帧命令增删。
pub type DynamicObstacles = BTreeMap<u64, GridArea>;

fn is_blocked(world: &StaticWorld, obstacles: &DynamicObstacles, cell: GridCell) -> bool {
    !world.is_walkable(cell) || obstacles.values().any(|area| area.contains(cell))
}

fn heuristic(from: GridCell, to: GridCell) -> i64 {
    let dx = (from.x as i64 - to.x as i64).abs();
    let dy = (from.y as i64 - to.y as i64).abs();
    let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
    STRAIGHT_COST * hi + (DIAG_COST - STRAIGHT_COST) * lo
}

/// 同步八方向 A*。搜索状态属于每次调用；`None` 表示端点非法、
/// 不可达或超出节点预算。路径含起终点，不做平滑或吸附。
pub fn find_path(
    world: &StaticWorld,
    obstacles: &DynamicObstacles,
    start: GridCell,
    goal: GridCell,
    budget: usize,
) -> Option<Vec<GridCell>> {
    let width = world.width();
    let height = world.height();
    let start_index = start.index(width, height)?;
    let goal_index = goal.index(width, height)?;
    if is_blocked(world, obstacles, start) || is_blocked(world, obstacles, goal) {
        return None;
    }
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || budget == 0 {
        return None;
    }

    let mut costs = vec![i64::MAX; count];
    let mut parents = vec![-1i32; count];
    let mut closed = vec![false; count];
    costs[start_index] = 0;

    // (score, index, cost)：index 显式打破等分平局，队列实现顺序无关。
    let mut open: BinaryHeap<Reverse<(i64, i32, i64)>> = BinaryHeap::new();
    open.push(Reverse((heuristic(start, goal), start_index as i32, 0)));

    let mut expanded = 0usize;
    while let Some(Reverse((_score, index, cost))) = open.pop() {
        let index = index as usize;
        if closed[index] || cost != costs[index] {
            continue;
        }
        if expanded >= budget {
            return None;
        }
        closed[index] = true;
        expanded += 1;
        if index == goal_index {
            return Some(reconstruct(&parents, index, width));
        }

        let cell = GridCell::from_index(index, width);
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let next = GridCell::new(cell.x + dx, cell.y + dy);
                let Some(next_index) = next.index(width, height) else {
                    continue;
                };
                if closed[next_index] || is_blocked(world, obstacles, next) {
                    continue;
                }
                let diagonal = dx != 0 && dy != 0;
                if diagonal {
                    // 对角线两侧也必须可走，避免贴角穿越。
                    let side_a = GridCell::new(cell.x, next.y);
                    let side_b = GridCell::new(next.x, cell.y);
                    if is_blocked(world, obstacles, side_a) || is_blocked(world, obstacles, side_b)
                    {
                        continue;
                    }
                }
                let step_cost = if diagonal { DIAG_COST } else { STRAIGHT_COST };
                let candidate = cost.saturating_add(step_cost);
                if candidate >= costs[next_index] {
                    continue;
                }
                costs[next_index] = candidate;
                parents[next_index] = index as i32;
                let score = candidate.saturating_add(heuristic(next, goal));
                open.push(Reverse((score, next_index as i32, candidate)));
            }
        }
    }
    None
}

fn reconstruct(parents: &[i32], goal_index: usize, width: u32) -> Vec<GridCell> {
    let mut path = Vec::new();
    let mut current = goal_index as i32;
    while current >= 0 {
        path.push(GridCell::from_index(current as usize, width));
        current = parents[current as usize];
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::SimFixed;

    fn world(width: u32, height: u32) -> StaticWorld {
        StaticWorld::open_world(width, height, SimFixed::ONE)
    }

    #[test]
    fn straight_line_path_includes_both_endpoints() {
        let path = find_path(
            &world(8, 8),
            &BTreeMap::new(),
            GridCell::new(0, 0),
            GridCell::new(3, 0),
            DEFAULT_NODE_BUDGET,
        )
        .unwrap();
        assert_eq!(path.first(), Some(&GridCell::new(0, 0)));
        assert_eq!(path.last(), Some(&GridCell::new(3, 0)));
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn static_wall_forces_a_detour() {
        let mut blocked = vec![false; 8 * 8];
        for y in 0..7 {
            blocked[(y * 8 + 3) as usize] = true;
        }
        let world = StaticWorld::new(
            8,
            8,
            SimFixed::ONE,
            (SimFixed::ZERO, SimFixed::ZERO),
            blocked,
            Vec::new(),
        );
        let path = find_path(
            &world,
            &BTreeMap::new(),
            GridCell::new(0, 0),
            GridCell::new(5, 0),
            DEFAULT_NODE_BUDGET,
        )
        .unwrap();
        assert!(path.iter().all(|cell| cell.x != 3 || cell.y == 7));
        assert!(path.len() > 6);
    }

    #[test]
    fn dynamic_obstacle_changes_the_route() {
        let world = world(8, 8);
        let empty = find_path(
            &world,
            &BTreeMap::new(),
            GridCell::new(0, 0),
            GridCell::new(4, 0),
            DEFAULT_NODE_BUDGET,
        )
        .unwrap();
        let mut obstacles = BTreeMap::new();
        obstacles.insert(
            1u64,
            GridArea {
                x: 2,
                y: 0,
                w: 1,
                h: 4,
            },
        );
        let detour = find_path(
            &world,
            &obstacles,
            GridCell::new(0, 0),
            GridCell::new(4, 0),
            DEFAULT_NODE_BUDGET,
        )
        .unwrap();
        assert!(detour.len() > empty.len());
        assert!(detour.iter().all(|cell| !(cell.x == 2 && cell.y < 4)));
    }

    #[test]
    fn blocked_goal_and_budget_are_rejected() {
        let mut blocked = vec![false; 8 * 8];
        blocked[0] = false;
        let world = StaticWorld::new(
            2,
            2,
            SimFixed::ONE,
            (SimFixed::ZERO, SimFixed::ZERO),
            blocked,
            Vec::new(),
        );
        let mut all_blocked = BTreeMap::new();
        all_blocked.insert(
            1u64,
            GridArea {
                x: 1,
                y: 1,
                w: 1,
                h: 1,
            },
        );
        assert!(
            find_path(
                &world,
                &all_blocked,
                GridCell::new(0, 0),
                GridCell::new(1, 1),
                DEFAULT_NODE_BUDGET,
            )
            .is_none()
        );
        assert!(
            find_path(
                &world,
                &BTreeMap::new(),
                GridCell::new(0, 0),
                GridCell::new(1, 1),
                1,
            )
            .is_none()
        );
    }
}
