//! Complete, versioned match state at a committed frame boundary.
//!
//! A snapshot is the state *after* frame N has been committed; restoring it
//! resumes at frame N+1. It contains the RNG stream, the next ID counter and
//! accepted-but-not-yet-executed commands. Behaviour is rebuilt from IDs and
//! explicit data, never from closures, tasks or live objects.

use crate::command::Command;

pub const SNAPSHOT_VERSION: u32 = 1;
pub const SIM_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PositionRecord {
    pub entity: u64,
    pub x: i64,
    pub y: i64,
    pub target: Option<(i64, i64)>,
    pub speed: i64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    pub version: u32,
    pub sim_version: u32,
    pub seed: u64,
    pub content_hash: u64,
    pub logic_fps: u32,
    pub tick: u64,
    pub rng: u64,
    pub next_entity_id: u64,
    pub alive_entities: Vec<u64>,
    pub positions: Vec<PositionRecord>,
    pub pending_commands: Vec<Command>,
    pub next_arrival: u64,
    pub next_event_seq: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapshotError {
    UnsupportedVersion { found: u32 },
    SimVersionMismatch { found: u32, expected: u32 },
    ContentMismatch,
    UnknownEntity(u64),
    MissingMotion,
}
