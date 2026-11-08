//! 帧边界命令编排。
//!
//! 客户端只按 relay 的权威帧命令集推进；乱序到达先按帧号缓存，再按
//! `(execute_frame, player_id, sequence)` 稳定排序后**依序** `submit_command`，
//! 使本地分配的 `Command.arrival` 在每个客户端一致（ADR 0002）。
//! relay 侧在收齐或超时后关闭帧，未提交玩家视为空输入，不无限等待。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{Command, CommandError, RtsMatch};

use crate::error::NetError;
use crate::packet::FrameCommands;

/// 缺失输入的默认等待上限（逻辑 tick）。
pub const DEFAULT_INPUT_TIMEOUT_FRAMES: u64 = 2;

/// 按稳定键排序命令；`arrival` 不参与（由本地依序提交时分配）。
pub fn stabilize(commands: &mut [Command]) {
    commands.sort_by_key(|command| {
        (
            command.execute_frame.value(),
            command.player_id,
            command.sequence,
        )
    });
}

/// 把一帧命令应用到对局：依序提交 → 恰好推进一帧 → 返回状态哈希。
pub fn apply_frame(sim: &mut RtsMatch, commands: &[Command]) -> Result<u64, NetError> {
    let mut sorted = commands.to_vec();
    stabilize(&mut sorted);
    for command in &sorted {
        if let Err(error) = sim.submit_command(*command) {
            let current = sim.read_view().tick;
            if matches!(error, CommandError::FrameInPast) {
                return Err(NetError::FrameInPast {
                    frame: command.execute_frame.value(),
                    current,
                });
            }
            return Err(NetError::Session(format!("command rejected: {error:?}")));
        }
    }
    sim.step();
    Ok(sim.compute_state_hash())
}

/// 客户端侧的帧调度结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameStep {
    /// 本帧命令已就绪（已稳定排序）。
    Ready { frame: u64, commands: Vec<Command> },
    /// 仍缺该帧输入；`waited` 为累计等待 tick。
    Waiting { frame: u64, waited: u64 },
}

/// 客户端帧调度器：按帧号排序、去重，避免重放重复执行。
pub struct FrameScheduler {
    next_frame: u64,
    buffered: BTreeMap<u64, Vec<Command>>,
    applied: BTreeSet<u64>,
    waited: u64,
    pub timeout_frames: u64,
}

impl FrameScheduler {
    pub fn new(start_frame: u64, timeout_frames: u64) -> Self {
        Self {
            next_frame: start_frame,
            buffered: BTreeMap::new(),
            applied: BTreeSet::new(),
            waited: 0,
            timeout_frames: timeout_frames.max(1),
        }
    }

    pub fn next_frame(&self) -> u64 {
        self.next_frame
    }

    /// 把下一帧基准前移到 `frame`（中途加入 / relay 已推进时对齐）。
    /// 只前进不后退，避免重放已应用帧。
    pub fn rebase(&mut self, frame: u64) {
        if frame > self.next_frame {
            self.next_frame = frame;
            self.buffered.retain(|candidate, _| *candidate >= frame);
            self.waited = 0;
        }
    }

    /// 缓存来自 relay 的权威帧命令集；已应用过的帧直接忽略（重放不重复执行）。
    pub fn offer(&mut self, frames: impl IntoIterator<Item = FrameCommands>) {
        for frame in frames {
            if self.applied.contains(&frame.frame) || frame.frame < self.next_frame {
                continue;
            }
            let mut commands = frame.commands;
            stabilize(&mut commands);
            self.buffered.insert(frame.frame, commands);
        }
    }

    /// 取出下一帧权威命令集（不累计等待）；供网络收包路径使用。
    pub fn take_ready(&mut self) -> Option<FrameCommands> {
        let commands = self.buffered.remove(&self.next_frame)?;
        let frame = self.next_frame;
        self.applied.insert(frame);
        self.next_frame += 1;
        self.waited = 0;
        Some(FrameCommands { frame, commands })
    }

    /// 累计一次缺帧等待（由确定性逻辑 tick 驱动，不读墙钟）。
    pub fn note_wait(&mut self) {
        self.waited += 1;
    }

    /// 尝试取帧；缺失则累计等待。
    pub fn poll(&mut self) -> FrameStep {
        if let Some(FrameCommands { frame, commands }) = self.take_ready() {
            return FrameStep::Ready { frame, commands };
        }
        self.note_wait();
        FrameStep::Waiting {
            frame: self.next_frame,
            waited: self.waited,
        }
    }

    pub fn has_timed_out(&self) -> bool {
        self.waited >= self.timeout_frames
    }
}

/// relay 侧的帧装配器：收集各玩家输入，收齐或超时后产出该帧权威命令集。
pub struct FrameAssembler {
    frame: u64,
    inputs: BTreeMap<u8, Vec<Command>>,
    expected_players: BTreeSet<u8>,
    waited: u64,
    pub timeout_ticks: u64,
}

impl FrameAssembler {
    pub fn new(start_frame: u64, expected_players: &[u8], timeout_ticks: u64) -> Self {
        Self {
            frame: start_frame,
            inputs: BTreeMap::new(),
            expected_players: expected_players.iter().copied().collect(),
            waited: 0,
            timeout_ticks: timeout_ticks.max(1),
        }
    }

    pub fn current_frame(&self) -> u64 {
        self.frame
    }

    /// 记录某玩家本帧提交的命令（同一玩家重复提交会覆盖，取最后一次）。
    pub fn submit(&mut self, player: u8, commands: Vec<Command>) {
        self.inputs.insert(player, commands);
    }

    /// 推进一个逻辑 tick；收齐全部玩家或超时则关闭并产出该帧。
    pub fn tick(&mut self) -> Option<FrameCommands> {
        let complete = self
            .expected_players
            .iter()
            .all(|player| self.inputs.contains_key(player));
        self.waited += 1;
        if !complete && self.waited < self.timeout_ticks {
            return None;
        }
        let mut commands = Vec::new();
        for (_, mut player_commands) in std::mem::take(&mut self.inputs) {
            commands.append(&mut player_commands);
        }
        stabilize(&mut commands);
        let produced = FrameCommands {
            frame: self.frame,
            commands,
        };
        self.frame += 1;
        self.waited = 0;
        Some(produced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::{CommandKind, Frame, MatchConfig, SimFixed, StaticWorld};

    fn command(frame: u64, player: u8, sequence: u32) -> Command {
        Command::new(Frame::new(frame), player, sequence, CommandKind::Noop)
    }

    fn frame_commands(frame: u64, commands: Vec<Command>) -> FrameCommands {
        FrameCommands { frame, commands }
    }

    fn new_match() -> RtsMatch {
        RtsMatch::create_with_world(
            MatchConfig::default(),
            StaticWorld::open_world(8, 8, SimFixed::from_int(4).unwrap()),
        )
    }

    #[test]
    fn stabilize_uses_frame_player_sequence_key() {
        let mut commands = vec![
            command(2, 1, 9),
            command(1, 3, 1),
            command(1, 1, 5),
            command(1, 1, 2),
        ];
        stabilize(&mut commands);
        let keys: Vec<_> = commands
            .iter()
            .map(|c| (c.execute_frame.value(), c.player_id, c.sequence))
            .collect();
        assert_eq!(keys, vec![(1, 1, 2), (1, 1, 5), (1, 3, 1), (2, 1, 9)]);
    }

    #[test]
    fn scheduler_applies_frames_in_order_and_ignores_replays() {
        let mut scheduler = FrameScheduler::new(1, 2);
        // 故意先给帧 2，再给帧 1。
        scheduler.offer([frame_commands(2, vec![command(2, 1, 1)])]);
        assert!(scheduler.take_ready().is_none());
        scheduler.offer([frame_commands(1, vec![command(1, 2, 1)])]);
        assert_eq!(
            scheduler.take_ready(),
            Some(frame_commands(1, vec![command(1, 2, 1)]))
        );
        assert_eq!(
            scheduler.take_ready(),
            Some(frame_commands(2, vec![command(2, 1, 1)]))
        );
        // 重放已应用的帧不会再次产出。
        scheduler.offer([frame_commands(1, vec![command(1, 2, 1)])]);
        assert!(scheduler.take_ready().is_none());
        assert_eq!(scheduler.next_frame(), 3);
    }

    #[test]
    fn missing_input_times_out_deterministically() {
        let mut scheduler = FrameScheduler::new(1, 2);
        assert!(!scheduler.has_timed_out());
        assert!(matches!(
            scheduler.poll(),
            FrameStep::Waiting { waited: 1, .. }
        ));
        assert!(matches!(
            scheduler.poll(),
            FrameStep::Waiting { waited: 2, .. }
        ));
        assert!(scheduler.has_timed_out());
    }

    #[test]
    fn poll_returns_frame_number_with_ready_commands() {
        let mut scheduler = FrameScheduler::new(4, 2);
        scheduler.offer([frame_commands(4, vec![command(4, 1, 1)])]);
        assert_eq!(
            scheduler.poll(),
            FrameStep::Ready {
                frame: 4,
                commands: vec![command(4, 1, 1)]
            }
        );
    }

    #[test]
    fn apply_frame_is_deterministic_across_instances() {
        let spawn = Command::new(
            Frame::new(0),
            0,
            1,
            CommandKind::Spawn {
                x: SimFixed::from_raw(65536),
                y: SimFixed::from_raw(65536),
                hp: 100,
                speed: SimFixed::from_raw(196608),
                attack_damage: 10,
                attack_range: SimFixed::from_raw(655360),
            },
        );
        // 乱序到达的命令。
        let commands = vec![command(2, 1, 5), spawn, command(1, 0, 3)];
        let mut a = new_match();
        let mut b = new_match();
        let hash_a = apply_frame(&mut a, &commands).unwrap();
        let hash_b = apply_frame(&mut b, &commands).unwrap();
        assert_eq!(hash_a, hash_b);
        assert_eq!(a.read_view(), b.read_view());
    }

    #[test]
    fn assembler_closes_frame_on_timeout_with_missing_player() {
        let mut assembler = FrameAssembler::new(0, &[1, 2], 2);
        assembler.submit(1, vec![command(0, 1, 1)]);
        assert!(assembler.tick().is_none());
        // 玩家 2 始终没提交；超时后按空输入关闭帧 0。
        let produced = assembler.tick().unwrap();
        assert_eq!(produced.frame, 0);
        assert_eq!(produced.commands.len(), 1);
        assert_eq!(assembler.current_frame(), 1);
    }

    #[test]
    fn assembler_closes_immediately_when_all_players_submitted() {
        let mut assembler = FrameAssembler::new(5, &[1, 2], 10);
        assembler.submit(2, vec![command(5, 2, 1)]);
        assembler.submit(1, vec![command(5, 1, 1)]);
        let produced = assembler.tick().unwrap();
        assert_eq!(produced.frame, 5);
        // 稳定排序：player 1 在前。
        assert_eq!(produced.commands[0].player_id, 1);
        assert_eq!(assembler.current_frame(), 6);
    }
}
