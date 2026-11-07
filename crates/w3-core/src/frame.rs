//! Integer logic frames. There is deliberately no delta, no wall clock and no
//! render frame rate anywhere in this module: `advance()` always moves exactly
//! one frame.

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
    /// `logic_fps` is frozen for the lifetime of the match.
    pub const fn new(logic_fps: u32) -> Self {
        Self {
            current: Frame::ZERO,
            logic_fps,
        }
    }

    /// Snapshot restore only. Not part of the public frame API.
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

    /// Commit the current frame and move to the next one.
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
        // No caller-supplied delta can change the step size.
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
            // There is no setter; the only way to change it is a new clock.
            assert_eq!(clock.logic_fps(), 45);
        }
    }
}
