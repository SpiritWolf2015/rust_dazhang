//! 显式、与平台无关的状态哈希（对小端字节做 FNV-1a）。
//! 权威状态绝不使用 `DefaultHasher`、`HashMap` 迭代顺序或对象身份。

const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Debug)]
pub struct StateHasher(u64);

impl StateHasher {
    pub fn new() -> Self {
        Self(OFFSET_BASIS)
    }

    pub fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_le_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_encoding_gives_same_hash() {
        let mut a = StateHasher::new();
        a.write_u64(42);
        a.write_u8(7);
        let mut b = StateHasher::new();
        b.write_u64(42);
        b.write_u8(7);
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn one_changed_field_changes_the_hash() {
        let mut a = StateHasher::new();
        a.write_i64(-9);
        let mut b = StateHasher::new();
        b.write_i64(-8);
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn byte_order_is_explicit() {
        let mut le = StateHasher::new();
        le.write_bytes(&1u64.to_le_bytes());
        let mut be = StateHasher::new();
        be.write_bytes(&1u64.to_be_bytes());
        assert_ne!(le.finish(), be.finish());
    }
}
