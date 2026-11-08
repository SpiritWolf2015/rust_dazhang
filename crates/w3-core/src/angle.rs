//! 离散角度与运动倍率（纯整数，见 ADR 0004）。
//!
//! 朝向是一整圈 `ANGLE_SCALE` 单位的整数角，不使用弧度、`π` 或三角函数；
//! 转向 / 坡度速度倍率用 `SimFixed` 整数多项式计算，避免浮点超越函数。

use crate::fixed::{SCALE, SimFixed};

/// 一整圈对应的角度单位数（16 位角分辨率）。
pub const ANGLE_SCALE: i64 = 1 << 16;
/// 半圈，对应 `π`；朝向归一化到 `[-HALF_TURN, HALF_TURN)`。
pub const HALF_TURN: i64 = ANGLE_SCALE / 2;

/// 转向速度倍率的下限（0.12）。
pub const MIN_TURN_SCALE: SimFixed = SimFixed::from_ratio(12, 100);
/// 上坡速度倍率下限（0.6）。
pub const UPHILL_MIN_SCALE: SimFixed = SimFixed::from_ratio(60, 100);
/// 下坡速度倍率下限（0.85）。
pub const DOWNHILL_MIN_SCALE: SimFixed = SimFixed::from_ratio(85, 100);
/// 30 度封顶对应的斜率（tan 30° ≈ 0.5774）。
pub const MAX_SLOPE: SimFixed = SimFixed::from_ratio(5774, 10_000);
/// 全速 / 最低倍率的角度阈值（度）。
pub const TURN_FULL_DEG: i64 = 25;
pub const TURN_MIN_DEG: i64 = 140;

const fn normalize_raw(raw: i64) -> i64 {
    let mut a = raw % ANGLE_SCALE;
    if a >= HALF_TURN {
        a -= ANGLE_SCALE;
    }
    if a < -HALF_TURN {
        a += ANGLE_SCALE;
    }
    a
}

/// 16 个方向的单位向量（Q16 近似 cos / sin），以及稳定选向表。
/// 索引 k 对应角度 `k * ANGLE_SCALE / 16`。
const DIRECTIONS: [(i64, i64); 16] = [
    (65536, 0),
    (60547, 25080),
    (46341, 46341),
    (25080, 60547),
    (0, 65536),
    (-25080, 60547),
    (-46341, 46341),
    (-60547, 25080),
    (-65536, 0),
    (-60547, -25080),
    (-46341, -46341),
    (-25080, -60547),
    (0, -65536),
    (25080, -60547),
    (46341, -46341),
    (60547, -25080),
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Angle(i64);

impl Angle {
    pub const ZERO: Self = Self(0);

    pub const fn from_raw(raw: i64) -> Self {
        Self(normalize_raw(raw))
    }

    pub const fn raw(self) -> i64 {
        self.0
    }

    /// 从 `self` 转到 `to` 的最短有符号角差，范围 `[-HALF_TURN, HALF_TURN)`。
    pub fn difference(self, to: Angle) -> i64 {
        normalize_raw(to.0 - self.0)
    }

    /// 按每帧整数增量朝 `desired` 转向，受 `max_step` 限制。
    pub fn turn_towards(self, desired: Angle, max_step: i64) -> Angle {
        let step_limit = max_step.max(0);
        let step = self.difference(desired).clamp(-step_limit, step_limit);
        Self::from_raw(self.0.saturating_add(step))
    }

    /// 由方向向量取最近的 16 个离散方向之一；零向量返回 `ZERO`。
    /// 平局取表中最小索引，保证确定性。
    pub fn from_direction(dx: i64, dy: i64) -> Angle {
        if dx == 0 && dy == 0 {
            return Angle::ZERO;
        }
        let mut best = 0usize;
        let mut best_dot = i128::MIN;
        for (k, &(fx, fy)) in DIRECTIONS.iter().enumerate() {
            let dot = (dx as i128) * (fx as i128) + (dy as i128) * (fy as i128);
            if dot > best_dot {
                best_dot = dot;
                best = k;
            }
        }
        Angle::from_raw((best as i64) * ANGLE_SCALE / 16)
    }

    /// 转向速度倍率：25° 内全速，140° 及以上取 0.12，中间 smoothstep。
    pub fn turn_speed_scale(self, desired: Angle) -> SimFixed {
        let degrees = self.difference(desired).saturating_abs() * 360 / ANGLE_SCALE;
        if degrees <= TURN_FULL_DEG {
            return SimFixed::ONE;
        }
        let span = TURN_MIN_DEG - TURN_FULL_DEG;
        let t_num = (degrees - TURN_FULL_DEG).min(span);
        let progress = SimFixed::from_raw(t_num * SCALE / span);
        let two = SimFixed::from_raw(2 * SCALE);
        let three = SimFixed::from_raw(3 * SCALE);
        let smooth = mul(mul(progress, progress), sub(three, mul(two, progress)));
        sub(
            SimFixed::ONE,
            mul(sub(SimFixed::ONE, MIN_TURN_SCALE), smooth),
        )
    }
}

/// 坡度速度倍率：`rise / distance` 的斜率线性映射到 [下限, 1]。
/// 上坡与下坡下限不同；平地或零距离不降速。
pub fn slope_speed_scale(rise: SimFixed, distance: SimFixed) -> SimFixed {
    if distance.raw() <= 0 || rise.raw() == 0 {
        return SimFixed::ONE;
    }
    let slope = rise.checked_div(distance).unwrap_or(SimFixed::ZERO);
    let abs_slope = slope.saturating_abs();
    let progress = if abs_slope.raw() >= MAX_SLOPE.raw() {
        SimFixed::ONE
    } else {
        abs_slope.checked_div(MAX_SLOPE).unwrap_or(SimFixed::ONE)
    };
    let limit = if rise.raw() > 0 {
        UPHILL_MIN_SCALE
    } else {
        DOWNHILL_MIN_SCALE
    };
    sub(SimFixed::ONE, mul(sub(SimFixed::ONE, limit), progress))
}

fn mul(a: SimFixed, b: SimFixed) -> SimFixed {
    a.checked_mul(b).expect("bounded motion scale")
}

fn sub(a: SimFixed, b: SimFixed) -> SimFixed {
    a.checked_sub(b).expect("bounded motion scale")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_normalize_into_half_turn_range() {
        assert_eq!(Angle::from_raw(ANGLE_SCALE).raw(), 0);
        assert_eq!(Angle::from_raw(HALF_TURN).raw(), -HALF_TURN);
        assert_eq!(Angle::from_raw(-HALF_TURN - 1).raw(), HALF_TURN - 1);
    }

    #[test]
    fn turn_towards_respects_the_rate_limit() {
        let start = Angle::ZERO;
        let desired = Angle::from_raw(ANGLE_SCALE / 4);
        assert_eq!(start.turn_towards(desired, 100).raw(), 100);
        assert_eq!(start.turn_towards(desired, 100_000).raw(), desired.raw());
    }

    #[test]
    fn turn_scale_is_between_min_and_one() {
        let desired = Angle::ZERO;
        assert_eq!(desired.turn_speed_scale(desired), SimFixed::ONE);
        let opposite = Angle::from_raw(HALF_TURN);
        assert_eq!(desired.turn_speed_scale(opposite), MIN_TURN_SCALE);
        let mid = Angle::from_raw(ANGLE_SCALE / 8);
        let scale = desired.turn_speed_scale(mid);
        assert!(scale.raw() > MIN_TURN_SCALE.raw() && scale.raw() < SCALE);
    }

    #[test]
    fn directions_are_stable_and_quantized() {
        assert_eq!(Angle::from_direction(100, 0), Angle::ZERO);
        assert_eq!(Angle::from_direction(0, 100).raw(), ANGLE_SCALE / 4);
        assert_eq!(Angle::from_direction(-100, 0).raw(), -HALF_TURN);
        assert_eq!(Angle::from_direction(0, 0), Angle::ZERO);
    }

    #[test]
    fn slope_scale_only_slows_real_slopes() {
        assert_eq!(
            slope_speed_scale(SimFixed::ZERO, SimFixed::ONE),
            SimFixed::ONE
        );
        let uphill = slope_speed_scale(SimFixed::from_int(1).unwrap(), SimFixed::ONE);
        let downhill = slope_speed_scale(SimFixed::from_int(-1).unwrap(), SimFixed::ONE);
        assert_eq!(uphill, UPHILL_MIN_SCALE);
        assert_eq!(downhill, DOWNHILL_MIN_SCALE);
    }
}
