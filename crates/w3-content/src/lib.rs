//! P1 content reader: turn the upstream `godot_warcraft3` JSON exports into
//! neutral Rust structures.
//!
//! Scope (ADR 0003, P1): read `map-parsed/<slug>/` (terrain heightfield, unit
//! and doodad placements) and `slk-exported/*.json` tables. **No native MPQ,
//! SLK, MDX or W3X parser lives here** — the upstream Node tools own that step.
//!
//! ## Determinism boundary (ADR 0002)
//!
//! Coordinates in this crate are the raw `f64` values from the upstream JSON.
//! They are **frozen input data**, not authoritative simulation state: they
//! never enter snapshots or state hashes, and `w3-core` does not depend on
//! this crate. P2 must convert them to `SimFixed` through an explicitly frozen
//! rounding rule before any value reaches the simulation.
//!
//! ## Asset boundary (ADR 0001)
//!
//! This crate contains only readers. The JSON it consumes is derived from the
//! player's own legally obtained classic client and must stay on their machine;
//! none of it is committed to this repository. Tests use synthetic fixtures.

pub mod heightfield;
pub mod load;
pub mod placements;
pub mod table;

pub use heightfield::{CenterOffset, HeightfieldError, TerrainHeightfield};
pub use load::{ContentError, ParsedMap, load_map, load_table};
pub use placements::{
    DoodadPlacement, DoodadsFile, PlacementError, Position3, UnitPlacement, UnitsFile,
};
pub use table::SlkTable;
