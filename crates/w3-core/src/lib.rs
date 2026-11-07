//! Deterministic, engine-independent RTS simulation kernel (P0 skeleton).
//!
//! This crate owns the authoritative simulation state. It has no dependency on
//! rendering, UI, audio, networking, file loading or wall-clock time.
//!
//! Determinism contract (see `docs/decisions/0002-determinism.md`, to be added):
//! - All authoritative state math uses [`fixed::SimFixed`] (integer only).
//! - A match advances by whole logic frames; callers cannot supply a delta.
//! - Commands are frozen per frame and executed in a stable order.
//! - State hashing uses an explicit byte encoding and stable ID order.
//! - The RNG stream is owned by the match and is part of every snapshot.
//!
//! Public entry points mirror the kernel contract: `create`, `submit_command`,
//! `step`, `read_view`, `drain_events`, `capture_snapshot`,
//! `restore_snapshot`, `compute_state_hash`, `first_difference`.

pub mod command;
pub mod entity;
pub mod fixed;
pub mod frame;
pub mod sim_match;
pub mod snapshot;
pub mod state_hash;

pub use command::{Command, CommandError, CommandKind, CommandQueue};
pub use entity::{EntityId, EntityStore};
pub use fixed::{FRACTIONAL_BITS, FixedError, SCALE, SimFixed};
pub use frame::{Frame, FrameClock};
pub use sim_match::{Event, EventKind, MatchConfig, RtsMatch, WorldView};
pub use snapshot::{Snapshot, SnapshotError};
pub use state_hash::StateHasher;
