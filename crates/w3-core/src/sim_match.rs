//! The unique owner of authoritative match state.
//!
//! `RtsMatch` is the only writer. Every public read returns an owned copy;
//! no mutable internal collection is ever exposed. `step()` advances exactly
//! one logic frame and takes no delta.

use std::collections::BTreeMap;

use crate::command::{Command, CommandError, CommandKind, CommandQueue};
use crate::entity::{EntityId, EntityStore};
use crate::fixed::SimFixed;
use crate::frame::{Frame, FrameClock};
use crate::snapshot::{PositionRecord, SIM_VERSION, SNAPSHOT_VERSION, Snapshot, SnapshotError};
use crate::state_hash::StateHasher;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatchConfig {
    pub seed: u64,
    pub logic_fps: u32,
    pub content_hash: u64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            logic_fps: 30,
            content_hash: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Motion {
    x: SimFixed,
    y: SimFixed,
    target: Option<(SimFixed, SimFixed)>,
    speed: SimFixed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Spawned,
    Arrived,
    Stopped,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Event {
    pub frame: u64,
    pub sequence: u64,
    pub entity: u64,
    pub kind: EventKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EntityView {
    pub id: u64,
    pub x: i64,
    pub y: i64,
    pub has_target: bool,
}

/// Owned, read-only projection of the world. Mutating it cannot affect the sim.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WorldView {
    pub tick: u64,
    pub rng: u64,
    pub entities: Vec<EntityView>,
}

pub struct RtsMatch {
    config: MatchConfig,
    clock: FrameClock,
    entities: EntityStore,
    motions: BTreeMap<EntityId, Motion>,
    commands: CommandQueue,
    rng: u64,
    events: Vec<Event>,
    next_event_seq: u64,
}

/// Pure-integer LCG: the same state always produces the same successor.
fn next_rng(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

fn hash_command(hasher: &mut StateHasher, command: &Command) {
    hasher.write_u64(command.execute_frame.value());
    hasher.write_u8(command.player_id);
    hasher.write_u32(command.sequence);
    hasher.write_u64(command.arrival());
    match command.kind {
        CommandKind::Noop => hasher.write_u8(0),
        CommandKind::Spawn => hasher.write_u8(1),
        CommandKind::Move { entity, x, y } => {
            hasher.write_u8(2);
            hasher.write_u64(entity.value());
            hasher.write_i64(x.raw());
            hasher.write_i64(y.raw());
        }
        CommandKind::Stop { entity } => {
            hasher.write_u8(3);
            hasher.write_u64(entity.value());
        }
    }
}

impl RtsMatch {
    pub fn create(config: MatchConfig) -> Self {
        Self {
            config,
            clock: FrameClock::new(config.logic_fps),
            entities: EntityStore::new(),
            motions: BTreeMap::new(),
            commands: CommandQueue::new(),
            rng: config.seed,
            events: Vec::new(),
            next_event_seq: 0,
        }
    }

    pub fn config(&self) -> MatchConfig {
        self.config
    }

    pub fn submit_command(&mut self, command: Command) -> Result<u64, CommandError> {
        if command.execute_frame.value() < self.clock.current().value() {
            return Err(CommandError::FrameInPast);
        }
        self.commands.submit(command)
    }

    /// Complete one logic frame. No caller-supplied delta, no wall clock.
    pub fn step(&mut self) {
        let frame = self.clock.current();

        // 1. Freeze and execute this frame's commands in the stable order.
        let due = self.commands.freeze_frame(frame);
        for command in due {
            self.apply(command, frame);
        }

        // 2. Advance motion and other authoritative systems.
        self.advance_motions(frame);

        // 3. Consume one RNG step per frame; the stream is part of the state.
        self.rng = next_rng(self.rng);

        // 4. Commit the frame.
        self.clock.advance();
    }

    fn apply(&mut self, command: Command, frame: Frame) {
        match command.kind {
            CommandKind::Noop => {}
            CommandKind::Spawn => {
                let id = self.entities.spawn();
                self.motions.insert(
                    id,
                    Motion {
                        x: SimFixed::ZERO,
                        y: SimFixed::ZERO,
                        target: None,
                        speed: SimFixed::ONE,
                    },
                );
                self.push_event(frame, id, EventKind::Spawned);
            }
            CommandKind::Move { entity, x, y } => {
                // Re-validate at execution time: acceptance is not execution.
                if let Some(motion) = self
                    .motions
                    .get_mut(&entity)
                    .filter(|_| self.entities.contains(entity))
                {
                    motion.target = Some((x, y));
                }
            }
            CommandKind::Stop { entity } => {
                if let Some(motion) = self
                    .motions
                    .get_mut(&entity)
                    .filter(|_| self.entities.contains(entity))
                {
                    motion.target = None;
                    self.push_event(frame, entity, EventKind::Stopped);
                }
            }
        }
    }

    fn advance_motions(&mut self, frame: Frame) {
        let mut arrived = Vec::new();
        // BTreeMap iteration is in stable EntityId order.
        for (&id, motion) in self.motions.iter_mut() {
            if let Some((target_x, target_y)) = motion.target {
                let dx = target_x.raw() - motion.x.raw();
                let dy = target_y.raw() - motion.y.raw();
                let step = motion.speed.raw();
                let nx = if dx > step {
                    motion.x.raw() + step
                } else if dx < -step {
                    motion.x.raw() - step
                } else {
                    target_x.raw()
                };
                let ny = if dy > step {
                    motion.y.raw() + step
                } else if dy < -step {
                    motion.y.raw() - step
                } else {
                    target_y.raw()
                };
                motion.x = SimFixed::from_raw(nx);
                motion.y = SimFixed::from_raw(ny);
                if motion.x.raw() == target_x.raw() && motion.y.raw() == target_y.raw() {
                    motion.target = None;
                    arrived.push(id);
                }
            }
        }
        for id in arrived {
            self.push_event(frame, id, EventKind::Arrived);
        }
    }

    fn push_event(&mut self, frame: Frame, entity: EntityId, kind: EventKind) {
        self.events.push(Event {
            frame: frame.value(),
            sequence: self.next_event_seq,
            entity: entity.value(),
            kind,
        });
        self.next_event_seq += 1;
    }

    pub fn read_view(&self) -> WorldView {
        let motions = &self.motions;
        WorldView {
            tick: self.clock.current().value(),
            rng: self.rng,
            entities: self
                .entities
                .iter()
                .map(|id| {
                    let motion = motions.get(&id).expect("every live entity has motion");
                    EntityView {
                        id: id.value(),
                        x: motion.x.raw(),
                        y: motion.y.raw(),
                        has_target: motion.target.is_some(),
                    }
                })
                .collect(),
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        // The sequence counter deliberately does not reset when drained.
        std::mem::take(&mut self.events)
    }

    pub fn compute_state_hash(&self) -> u64 {
        let mut hasher = StateHasher::new();
        hasher.write_u64(self.clock.current().value());
        hasher.write_u64(self.rng);
        hasher.write_u64(self.entities.next_id());
        hasher.write_u64(self.commands.next_arrival());
        hasher.write_u64(self.next_event_seq);
        hasher.write_u64(self.config.seed);
        hasher.write_u64(self.config.content_hash);
        hasher.write_u32(self.config.logic_fps);

        let motions = &self.motions;
        hasher.write_u64(self.entities.len() as u64);
        for id in self.entities.iter() {
            hasher.write_u64(id.value());
            let motion = motions.get(&id).expect("every live entity has motion");
            hasher.write_i64(motion.x.raw());
            hasher.write_i64(motion.y.raw());
            hasher.write_i64(motion.speed.raw());
            match motion.target {
                Some((x, y)) => {
                    hasher.write_u8(1);
                    hasher.write_i64(x.raw());
                    hasher.write_i64(y.raw());
                }
                None => hasher.write_u8(0),
            }
        }

        hasher.write_u64(self.commands.pending().len() as u64);
        for command in self.commands.pending() {
            hash_command(&mut hasher, command);
        }
        hasher.finish()
    }

    /// Name the first field that differs, for desync diagnosis. Stable order,
    /// so the reported field is reproducible.
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.clock.current() != other.clock.current() {
            return Some("tick");
        }
        if self.rng != other.rng {
            return Some("rng");
        }
        if self.entities.next_id() != other.entities.next_id() {
            return Some("next_entity_id");
        }
        if self.commands.next_arrival() != other.commands.next_arrival() {
            return Some("next_arrival");
        }
        if self.next_event_seq != other.next_event_seq {
            return Some("next_event_seq");
        }
        if self.entities.iter().collect::<Vec<_>>() != other.entities.iter().collect::<Vec<_>>() {
            return Some("entities");
        }
        for id in self.entities.iter() {
            let a = self.motions.get(&id).map(|m| (m.x, m.y, m.target, m.speed));
            let b = other
                .motions
                .get(&id)
                .map(|m| (m.x, m.y, m.target, m.speed));
            if a != b {
                return Some("motions");
            }
        }
        if self.commands.pending() != other.commands.pending() {
            return Some("pending_commands");
        }
        None
    }

    pub fn capture_snapshot(&self) -> Snapshot {
        let motions = &self.motions;
        Snapshot {
            version: SNAPSHOT_VERSION,
            sim_version: SIM_VERSION,
            seed: self.config.seed,
            content_hash: self.config.content_hash,
            logic_fps: self.config.logic_fps,
            tick: self.clock.current().value(),
            rng: self.rng,
            next_entity_id: self.entities.next_id(),
            alive_entities: self.entities.iter().map(|id| id.value()).collect(),
            positions: self
                .entities
                .iter()
                .map(|id| {
                    let motion = motions.get(&id).expect("every live entity has motion");
                    PositionRecord {
                        entity: id.value(),
                        x: motion.x.raw(),
                        y: motion.y.raw(),
                        target: motion.target.map(|(x, y)| (x.raw(), y.raw())),
                        speed: motion.speed.raw(),
                    }
                })
                .collect(),
            pending_commands: self.commands.pending().to_vec(),
            next_arrival: self.commands.next_arrival(),
            next_event_seq: self.next_event_seq,
        }
    }

    /// Rebuild a match from a frame-boundary snapshot. Historical events are
    /// not replayed; the event sequence counter is restored instead.
    pub fn restore_snapshot(
        snapshot: Snapshot,
        expected_content_hash: u64,
    ) -> Result<Self, SnapshotError> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::UnsupportedVersion {
                found: snapshot.version,
            });
        }
        if snapshot.sim_version != SIM_VERSION {
            return Err(SnapshotError::SimVersionMismatch {
                found: snapshot.sim_version,
                expected: SIM_VERSION,
            });
        }
        if snapshot.content_hash != expected_content_hash {
            return Err(SnapshotError::ContentMismatch);
        }

        let config = MatchConfig {
            seed: snapshot.seed,
            logic_fps: snapshot.logic_fps,
            content_hash: snapshot.content_hash,
        };
        let entities = EntityStore::restore(snapshot.next_entity_id, &snapshot.alive_entities);

        let mut motions = BTreeMap::new();
        for record in snapshot.positions {
            let id =
                EntityId::new(record.entity).ok_or(SnapshotError::UnknownEntity(record.entity))?;
            if !entities.contains(id) {
                return Err(SnapshotError::UnknownEntity(record.entity));
            }
            motions.insert(
                id,
                Motion {
                    x: SimFixed::from_raw(record.x),
                    y: SimFixed::from_raw(record.y),
                    target: record
                        .target
                        .map(|(x, y)| (SimFixed::from_raw(x), SimFixed::from_raw(y))),
                    speed: SimFixed::from_raw(record.speed),
                },
            );
        }
        if motions.len() != entities.len() {
            return Err(SnapshotError::MissingMotion);
        }

        Ok(Self {
            config,
            clock: FrameClock::at(Frame::new(snapshot.tick), snapshot.logic_fps),
            entities,
            motions,
            commands: CommandQueue::restore(snapshot.next_arrival, snapshot.pending_commands),
            rng: snapshot.rng,
            events: Vec::new(),
            next_event_seq: snapshot.next_event_seq,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_view_is_an_owned_copy() {
        let mut sim = RtsMatch::create(MatchConfig::default());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, CommandKind::Spawn))
            .unwrap();
        sim.step();
        let view = sim.read_view();
        assert_eq!(view.entities.len(), 1);
        assert_eq!(view.tick, 1);
        // The view cannot be used to mutate simulation state.
        assert_eq!(sim.read_view(), view);
    }

    #[test]
    fn draining_events_does_not_reset_the_sequence() {
        let mut sim = RtsMatch::create(MatchConfig::default());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, CommandKind::Spawn))
            .unwrap();
        sim.step();
        let first = sim.drain_events();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].sequence, 0);
        sim.submit_command(Command::new(Frame::new(1), 1, 2, CommandKind::Spawn))
            .unwrap();
        sim.step();
        let second = sim.drain_events();
        assert_eq!(second[0].sequence, 1);
    }
}
