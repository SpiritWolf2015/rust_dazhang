//! 整数逻辑帧。本模块刻意不引入 delta、墙钟或渲染帧率：
//! `advance()` 每次恰好推进一帧。

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Frame(u64);

impl Frame {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameClock {
    current: Frame,
    logic_fps: u32,
}

impl FrameClock {
    /// `logic_fps` 在对局整个生命周期内冻结不变。
    pub const fn new(logic_fps: u32) -> Self {
        Self {
            current: Frame::ZERO,
            logic_fps,
        }
    }

    /// 仅供快照恢复使用，不属于公开的帧 API。
    pub(crate) const fn at(frame: Frame, logic_fps: u32) -> Self {
        Self {
            current: frame,
            logic_fps,
        }
    }

    pub const fn current(&self) -> Frame {
        self.current
    }

    pub const fn logic_fps(&self) -> u32 {
        self.logic_fps
    }

    /// 提交当前帧并推进到下一帧。
    pub fn advance(&mut self) -> Frame {
        self.current = Frame(self.current.0 + 1);
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_moves_exactly_one_frame() {
        let mut clock = FrameClock::new(30);
        assert_eq!(clock.current(), Frame::ZERO);
        assert_eq!(clock.advance(), Frame::new(1));
        assert_eq!(clock.advance(), Frame::new(2));
        // 调用方传入的 delta 无法改变步长。
        for _ in 0..1_000 {
            let before = clock.current().value();
            clock.advance();
            assert_eq!(clock.current().value(), before + 1);
        }
    }

    #[test]
    fn logic_fps_is_frozen_at_construction() {
        let clock = FrameClock::new(45);
        for _ in 0..10 {
            // 没有 setter；要改变它只能新建一个时钟。
            assert_eq!(clock.logic_fps(), 45);
        }
    }
}
