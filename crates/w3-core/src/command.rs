//! Player commands and the queue that freezes them into deterministic frames.
//!
//! Execution order is `ExecuteFrame -> PlayerId -> Sequence -> ArrivalOrder`.
//! A command is frozen (including its arrival number) at submission time; the
//! queue never reorders by when a packet happened to arrive.

use crate::entity::EntityId;
use crate::fixed::SimFixed;
use crate::frame::Frame;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandKind {
    Noop,
    Spawn,
    Move {
        entity: EntityId,
        x: SimFixed,
        y: SimFixed,
    },
    Stop {
        entity: EntityId,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Command {
    pub execute_frame: Frame,
    pub player_id: u8,
    pub sequence: u32,
    pub kind: CommandKind,
    /// Assigned by the queue at submission, so it cannot change afterwards.
    arrival: u64,
}

impl Command {
    pub fn new(execute_frame: Frame, player_id: u8, sequence: u32, kind: CommandKind) -> Self {
        Self {
            execute_frame,
            player_id,
            sequence,
            kind,
            arrival: 0,
        }
    }

    pub const fn arrival(&self) -> u64 {
        self.arrival
    }

    fn with_arrival(mut self, arrival: u64) -> Self {
        self.arrival = arrival;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandError {
    InvalidEntity,
    FrameInPast,
}

#[derive(Clone, Debug, Default)]
pub struct CommandQueue {
    pending: Vec<Command>,
    next_arrival: u64,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate, freeze the arrival number and enqueue. Acceptance does not
    /// mean the command will execute successfully; `step()` re-validates.
    pub fn submit(&mut self, command: Command) -> Result<u64, CommandError> {
        let invalid_entity = match command.kind {
            CommandKind::Move { entity, .. } | CommandKind::Stop { entity } => {
                entity == EntityId::NONE
            }
            CommandKind::Noop | CommandKind::Spawn => false,
        };
        if invalid_entity {
            return Err(CommandError::InvalidEntity);
        }

        let arrival = self.next_arrival;
        self.next_arrival += 1;
        self.pending.push(command.with_arrival(arrival));
        Ok(arrival)
    }

    /// Take every command due at or before `frame`, in the stable order.
    pub fn freeze_frame(&mut self, frame: Frame) -> Vec<Command> {
        let (mut due, rest): (Vec<Command>, Vec<Command>) = self
            .pending
            .drain(..)
            .partition(|c| c.execute_frame <= frame);
        self.pending = rest;
        due.sort_by_key(|c| (c.execute_frame, c.player_id, c.sequence, c.arrival));
        due
    }

    pub fn pending(&self) -> &[Command] {
        &self.pending
    }

    pub fn next_arrival(&self) -> u64 {
        self.next_arrival
    }

    /// Snapshot restore only.
    pub(crate) fn restore(next_arrival: u64, pending: Vec<Command>) -> Self {
        Self {
            pending,
            next_arrival,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeze_frame_uses_the_stable_sort_key() {
        let mut queue = CommandQueue::new();
        // Deliberately out of order.
        queue
            .submit(Command::new(Frame::new(2), 1, 9, CommandKind::Noop))
            .unwrap();
        queue
            .submit(Command::new(Frame::new(1), 3, 1, CommandKind::Noop))
            .unwrap();
        queue
            .submit(Command::new(Frame::new(1), 1, 5, CommandKind::Noop))
            .unwrap();
        queue
            .submit(Command::new(Frame::new(1), 1, 2, CommandKind::Noop))
            .unwrap();
        queue
            .submit(Command::new(Frame::new(1), 2, 1, CommandKind::Noop))
            .unwrap();

        let due = queue.freeze_frame(Frame::new(2));
        let keys: Vec<_> = due
            .iter()
            .map(|c| (c.execute_frame.value(), c.player_id, c.sequence))
            .collect();
        assert_eq!(
            keys,
            vec![(1, 1, 2), (1, 1, 5), (1, 2, 1), (1, 3, 1), (2, 1, 9)]
        );
        assert!(queue.pending().is_empty());
    }

    #[test]
    fn future_commands_are_left_pending() {
        let mut queue = CommandQueue::new();
        queue
            .submit(Command::new(Frame::new(5), 1, 1, CommandKind::Noop))
            .unwrap();
        assert!(queue.freeze_frame(Frame::new(0)).is_empty());
        assert_eq!(queue.pending().len(), 1);
        assert_eq!(queue.freeze_frame(Frame::new(5)).len(), 1);
    }

    #[test]
    fn arrival_numbers_are_frozen_at_submission() {
        let mut queue = CommandQueue::new();
        queue
            .submit(Command::new(Frame::new(0), 1, 1, CommandKind::Noop))
            .unwrap();
        let second = queue
            .submit(Command::new(Frame::new(0), 1, 2, CommandKind::Noop))
            .unwrap();
        assert_eq!(second, 1);
        // Re-submitting copies cannot change the recorded arrival order.
        assert_eq!(queue.next_arrival(), 2);
    }

    #[test]
    fn invalid_entity_is_rejected() {
        let mut queue = CommandQueue::new();
        let bad = Command::new(
            Frame::new(0),
            1,
            1,
            CommandKind::Stop {
                entity: EntityId::NONE,
            },
        );
        assert_eq!(queue.submit(bad), Err(CommandError::InvalidEntity));
    }
}
