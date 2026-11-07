//! Fixed-point arithmetic for authoritative simulation state.
//!
//! Q47.16 over `i64`, with `i128` intermediates. All operations are integer
//! only, so results are bit-identical on every platform and CPU. No `f32`/`f64`
//! may enter simulation state.

pub const FRACTIONAL_BITS: u32 = 16;
pub const SCALE: i64 = 1i64 << FRACTIONAL_BITS;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct SimFixed(i64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FixedError {
    Overflow,
    DivisionByZero,
}

impl SimFixed {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(SCALE);

    /// Wrap a raw Q16 value. Only snapshot/restore and tests should need this.
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i64 {
        self.0
    }

    pub fn from_int(value: i64) -> Result<Self, FixedError> {
        value
            .checked_mul(SCALE)
            .map(Self)
            .ok_or(FixedError::Overflow)
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(FixedError::Overflow)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(FixedError::Overflow)
    }

    /// `(a * b) / SCALE`, computed in i128 so no intermediate overflows.
    /// Integer division truncates toward zero on every Rust target.
    pub fn checked_mul(self, rhs: Self) -> Result<Self, FixedError> {
        let product = (self.0 as i128) * (rhs.0 as i128);
        let scaled = product / (SCALE as i128);
        i64::try_from(scaled)
            .map(Self)
            .map_err(|_| FixedError::Overflow)
    }

    /// `(a * SCALE) / b`, computed in i128.
    pub fn checked_div(self, rhs: Self) -> Result<Self, FixedError> {
        if rhs.0 == 0 {
            return Err(FixedError::DivisionByZero);
        }
        let numerator = (self.0 as i128) * (SCALE as i128);
        let quotient = numerator / (rhs.0 as i128);
        i64::try_from(quotient)
            .map(Self)
            .map_err(|_| FixedError::Overflow)
    }

    pub fn floor_to_i64(self) -> i64 {
        self.0 >> FRACTIONAL_BITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_roundtrip() {
        let v = SimFixed::from_int(7).unwrap();
        assert_eq!(v.raw(), 7 * SCALE);
        assert_eq!(v.floor_to_i64(), 7);
    }

    #[test]
    fn mul_and_div_are_exact_for_small_integers() {
        let two = SimFixed::from_int(2).unwrap();
        let three = SimFixed::from_int(3).unwrap();
        let six = SimFixed::from_int(6).unwrap();
        assert_eq!(three.checked_mul(two).unwrap(), six);
        assert_eq!(six.checked_div(two).unwrap(), three);
    }

    #[test]
    fn division_by_zero_is_rejected() {
        assert_eq!(
            SimFixed::ONE.checked_div(SimFixed::ZERO),
            Err(FixedError::DivisionByZero)
        );
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        assert_eq!(
            SimFixed::from_raw(i64::MAX).checked_add(SimFixed::ONE),
            Err(FixedError::Overflow)
        );
    }

    #[test]
    fn repeated_math_is_bit_identical() {
        let a = SimFixed::from_raw(123_456_789);
        let b = SimFixed::from_raw(-987_654);
        let first = a.checked_mul(b).unwrap().checked_div(a).unwrap();
        for _ in 0..32 {
            assert_eq!(a.checked_mul(b).unwrap().checked_div(a).unwrap(), first);
        }
    }
}
