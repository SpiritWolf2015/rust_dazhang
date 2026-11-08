//! 冻结边界：`f64` 世界坐标 → `SimFixed`（见 ADR 0004）。
//!
//! 用 `f64::to_bits()` 做纯整数 IEEE-754 位分解，再以 `i128` 整数运算
//! 计算 `round(value × 2^16)`。全程不执行任何 `f64` 算术、不调用宿主
//! 数学库，因此「相同 JSON 文本 → 相同位模式 → 相同 raw」在任何平台一致。

use std::fmt;

use w3_core::{FRACTIONAL_BITS, SimFixed};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrozenError {
    /// NaN 或 ±Inf。
    NotFinite,
    /// 超出 Q47.16 可表示范围（|value × 2^16| > i64::MAX）。
    Overflow,
}

impl fmt::Display for FrozenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => f.write_str("value is not finite"),
            Self::Overflow => f.write_str("value overflows Q47.16 range"),
        }
    }
}

impl std::error::Error for FrozenError {}

/// 把冻结的 `f64` 世界坐标转换为 `SimFixed`。
///
/// 缩放 1:1（1 世界单位 = `2^FRACTIONAL_BITS` raw）；舍入为
/// round half away from zero；拒绝 NaN / Inf 与溢出。
pub fn freeze_f64(value: f64) -> Result<SimFixed, FrozenError> {
    let bits = value.to_bits();
    let negative = bits >> 63 != 0;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    if exponent == 0x7ff {
        return Err(FrozenError::NotFinite);
    }

    // value = significand × 2^exp2，其中 exp2 已含 ×2^16 的缩放。
    // normal:   (2^52 + mantissa) × 2^(exponent - 1023 - 52 + 16)
    // subnormal: mantissa × 2^(-1074 + 16)
    let (significand, exp2): (i128, i32) = if exponent == 0 {
        if mantissa == 0 {
            return Ok(SimFixed::ZERO);
        }
        (mantissa as i128, -1074 + FRACTIONAL_BITS as i32)
    } else {
        (
            (mantissa | (1u64 << 52)) as i128,
            exponent - 1023 - 52 + FRACTIONAL_BITS as i32,
        )
    };

    let magnitude: i128 = if exp2 >= 0 {
        // significand < 2^53；shift >= 64 时必然超出 i64。
        let shift = exp2 as u32;
        if shift >= 64 {
            return Err(FrozenError::Overflow);
        }
        let shifted = significand << shift;
        if shifted > i64::MAX as i128 {
            return Err(FrozenError::Overflow);
        }
        shifted
    } else {
        let shift = (-exp2) as u32;
        if shift > 54 {
            // significand < 2^53 < 2^(shift-1)，商为 0 且不足半单位。
            0
        } else {
            let quotient = significand >> shift;
            let remainder = significand & ((1i128 << shift) - 1);
            let half = 1i128 << (shift - 1);
            if remainder >= half {
                quotient + 1
            } else {
                quotient
            }
        }
    };

    if magnitude > i64::MAX as i128 {
        return Err(FrozenError::Overflow);
    }
    let raw = if negative {
        -(magnitude as i64)
    } else {
        magnitude as i64
    };
    Ok(SimFixed::from_raw(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCALE: i64 = 1 << FRACTIONAL_BITS;

    #[test]
    fn zero_and_integers_are_exact() {
        assert_eq!(freeze_f64(0.0).unwrap(), SimFixed::ZERO);
        assert_eq!(freeze_f64(1.0).unwrap().raw(), SCALE);
        assert_eq!(freeze_f64(-1.0).unwrap().raw(), -SCALE);
        assert_eq!(freeze_f64(1.5).unwrap().raw(), 3 * SCALE / 2);
        assert_eq!(freeze_f64(-1.5).unwrap().raw(), -3 * SCALE / 2);
        assert_eq!(freeze_f64(0.5).unwrap().raw(), SCALE / 2);
    }

    #[test]
    fn rounding_is_half_away_from_zero_and_symmetric() {
        // value × SCALE = 0.5 → 1；-0.5 → -1。
        assert_eq!(freeze_f64(1.0 / 131_072.0).unwrap().raw(), 1);
        assert_eq!(freeze_f64(-1.0 / 131_072.0).unwrap().raw(), -1);
        // 0.25 → 0；不足半单位不下溢成 1。
        assert_eq!(freeze_f64(1.0 / 262_144.0).unwrap().raw(), 0);
        assert_eq!(freeze_f64(-1.0 / 262_144.0).unwrap().raw(), 0);
    }

    #[test]
    fn non_finite_values_are_rejected() {
        assert_eq!(freeze_f64(f64::NAN), Err(FrozenError::NotFinite));
        assert_eq!(freeze_f64(f64::INFINITY), Err(FrozenError::NotFinite));
        assert_eq!(freeze_f64(f64::NEG_INFINITY), Err(FrozenError::NotFinite));
    }

    #[test]
    fn overflow_is_explicit() {
        assert_eq!(freeze_f64(1e15), Err(FrozenError::Overflow));
        assert_eq!(freeze_f64(-1e15), Err(FrozenError::Overflow));
        assert_eq!(freeze_f64(f64::MAX), Err(FrozenError::Overflow));
    }

    #[test]
    fn subnormals_freeze_to_zero() {
        assert_eq!(freeze_f64(f64::from_bits(1)).unwrap(), SimFixed::ZERO);
        assert_eq!(freeze_f64(-f64::from_bits(1)).unwrap(), SimFixed::ZERO);
    }

    #[test]
    fn echo_isles_coordinate_bounds_do_not_overflow() {
        // Echo Isles 量级：地图坐标约 ±8192 世界单位，heightfield 129×97
        // 顶点、97 单位摆放。以这些边界值验证冻结与后续 Q47.16 运算。
        let bounds = [8192.0, -8192.0, 4096.0, 128.0, 96.0, 12_513.0];
        for &value in &bounds {
            let fixed = freeze_f64(value).unwrap();
            assert!(fixed.checked_mul(fixed).is_ok(), "mul overflow at {value}");
            assert!(fixed.checked_div(SimFixed::ONE).is_ok());
        }
        let east = freeze_f64(8192.0).unwrap();
        let west = freeze_f64(-8192.0).unwrap();
        assert_eq!(east.raw(), 8192 * SCALE);
        assert_eq!(west.raw(), -8192 * SCALE);
        assert!(east.checked_mul(west).is_ok());
        assert!(east.checked_add(west).unwrap() == SimFixed::ZERO);
    }

    #[test]
    fn freeze_is_bit_identical_across_repeated_calls() {
        let value = 1234.5678_f64;
        let first = freeze_f64(value).unwrap();
        for _ in 0..64 {
            assert_eq!(freeze_f64(value).unwrap(), first);
        }
    }
}
