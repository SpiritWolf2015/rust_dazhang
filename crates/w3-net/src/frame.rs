//! 帧边界命令编排。
//!
//! 客户端只按 relay 的权威帧命令集推进；乱序到达先按帧号缓存，再按
//! `(execute_frame, player_id, sequence)` 稳定排序后**依序**提交，
//! 使本地分配的 `Command.arrival` 在每个客户端一致（ADR 0002）。
//! relay 侧在收齐或超时后关闭帧，未提交玩家视为空输入，不无限等待。
//!
//! 自 `PROTOCOL_VERSION = 3` 起一帧同时含普通 `Command` 与玩法
//! `GameplayCommand`：两类命令是两条独立队列、各自独立 `arrival`，
//! 由统一归并视图 [`FrameInput`] 按 `(execute_frame, player_id,
//! sequence, type_tag)` 稳定排序（固定 `type_tag`：`Simulation=0` /
//! `Gameplay=1`），在**恰好一次**无参 `RtsMatch::step()` 之前全部提交。
//! 统一键只用于规范化归并顺序，不改变各类队列内部 `arrival` 语义。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{Command, CommandError, GameplayCommand, GameplayCommandError, RtsMatch};

use crate::error::NetError;
use crate::packet::FrameCommands;

/// 缺失输入的默认等待上限（逻辑 tick）。
pub const DEFAULT_INPUT_TIMEOUT_FRAMES: u64 = 2;

/// 统一稳定键中普通模拟命令的固定判别值（平台无关常量）。
pub const TYPE_TAG_SIMULATION: u8 = 0;
/// 统一稳定键中玩法命令的固定判别值（平台无关常量）。
pub const TYPE_TAG_GAMEPLAY: u8 = 1;

/// 一帧内两类命令的统一归并视图。
///
/// 仅用于规范化归并顺序：`type_tag` 是固定常量，作为 `(execute_frame,
/// player_id, sequence)` 完全相同两条命令的确定性 tiebreaker，避免依赖
/// 不确定遍历（如 `HashMap`）。两类命令仍分别经 `submit_command` /
/// `submit_gameplay_command` 提交到各自独立队列，互不共享 `arrival`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameInput<'a> {
    /// 普通模拟命令（`Simulation`，`type_tag = 0`）。
    Simulation(&'a Command),
    /// 玩法命令（`Gameplay`，`type_tag = 1`）。
    Gameplay(&'a GameplayCommand),
}

impl FrameInput<'_> {
    /// 固定类型判别值；不依赖哈希或运行期值。
    pub const fn type_tag(&self) -> u8 {
        match self {
            FrameInput::Simulation(_) => TYPE_TAG_SIMULATION,
            FrameInput::Gameplay(_) => TYPE_TAG_GAMEPLAY,
        }
    }

    /// 统一稳定键 `(execute_frame, player_id, sequence, type_tag)`。
    pub fn stable_key(&self) -> (u64, u8, u32, u8) {
        match *self {
            FrameInput::Simulation(command) => (
                command.execute_frame.value(),
                command.player_id,
                command.sequence,
                TYPE_TAG_SIMULATION,
            ),
            FrameInput::Gameplay(command) => (
                command.execute_frame.value(),
                command.player_id,
                command.sequence,
                TYPE_TAG_GAMEPLAY,
            ),
        }
    }
}

/// 按稳定键排序普通命令；`arrival` 不参与（由本地依序提交时分配）。
/// 该排序键语义保持原样（`ExecuteFrame → PlayerId → Sequence`）。
pub fn stabilize(commands: &mut [Command]) {
    commands.sort_by_key(|command| {
        (
            command.execute_frame.value(),
            command.player_id,
            command.sequence,
        )
    });
}

/// 按与普通命令相同的稳定键排序玩法命令；玩法队列 `arrival` 独立、
/// 不参与排序键。
pub fn stabilize_gameplay(gameplay: &mut [GameplayCommand]) {
    gameplay.sort_by_key(|command| {
        (
            command.execute_frame.value(),
            command.player_id,
            command.sequence,
        )
    });
}

/// 按 `(execute_frame, player_id, sequence, type_tag)` 稳定排序两类命令的归并视图。
/// `sort_by_key` 是稳定排序，相同键保持输入相对顺序，归并序列可复现。
pub fn stabilize_frame_inputs(inputs: &mut [FrameInput<'_>]) {
    inputs.sort_by_key(FrameInput::stable_key);
}

/// 把一帧的两类命令应用到对局：统一稳定键归并 → 依序全部提交 →
/// **恰好一次**无参 `sim.step()` → 返回 `compute_state_hash()`。
///
/// 普通命令经 `submit_command`、玩法命令经 `submit_gameplay_command`
/// （内部走 `validate_gameplay_command`，与玩家 / AI 同源校验）提交；
/// 任一提交失败显式返回 `NetError`，不跳过、不静默降级、不额外推进。
pub fn apply_frame(
    sim: &mut RtsMatch,
    commands: &[Command],
    gameplay: &[GameplayCommand],
) -> Result<u64, NetError> {
    let mut inputs: Vec<FrameInput<'_>> = commands
        .iter()
        .map(FrameInput::Simulation)
        .chain(gameplay.iter().map(FrameInput::Gameplay))
        .collect();
    stabilize_frame_inputs(&mut inputs);

    for input in inputs {
        match input {
            FrameInput::Simulation(command) => {
                if let Err(error) = sim.submit_command(*command) {
                    let current = sim.read_view().tick;
                    if matches!(error, CommandError::FrameInPast) {
                        return Err(NetError::FrameInPast {
                            frame: command.execute_frame.value(),
                            current,
                        });
                    }
                    return Err(NetError::Session(format!(
                        "command rejected: error={error:?} frame={} player={}",
                        command.execute_frame.value(),
                        command.player_id
                    )));
                }
            }
            FrameInput::Gameplay(command) => {
                if let Err(error) = sim.submit_gameplay_command(*command) {
                    let current = sim.read_view().tick;
                    if matches!(error, GameplayCommandError::FrameInPast) {
                        return Err(NetError::FrameInPast {
                            frame: command.execute_frame.value(),
                            current,
                        });
                    }
                    return Err(NetError::Session(format!(
                        "gameplay command rejected: error={error:?} frame={} player={}",
                        command.execute_frame.value(),
                        command.player_id
                    )));
                }
            }
        }
    }
    sim.step();
    Ok(sim.compute_state_hash())
}

/// 客户端侧的帧调度结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameStep {
    /// 本帧命令已就绪（两类命令均已稳定排序）。
    Ready {
        frame: u64,
        commands: Vec<Command>,
        gameplay: Vec<GameplayCommand>,
    },
    /// 仍缺该帧输入；`waited` 为累计等待 tick。
    Waiting { frame: u64, waited: u64 },
}

/// 客户端帧调度器：按帧号排序、去重，避免重放重复执行。
pub struct FrameScheduler {
    next_frame: u64,
    buffered: BTreeMap<u64, FrameCommands>,
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

    /// 缓存来自 relay 的权威帧命令集；两类命令分别稳定排序，
    /// 到达顺序不决定执行顺序。已应用过的帧直接忽略（重放不重复执行）。
    pub fn offer(&mut self, frames: impl IntoIterator<Item = FrameCommands>) {
        for mut frame in frames {
            if self.applied.contains(&frame.frame) || frame.frame < self.next_frame {
                continue;
            }
            stabilize(&mut frame.commands);
            stabilize_gameplay(&mut frame.gameplay);
            self.buffered.insert(frame.frame, frame);
        }
    }

    /// 取出下一帧权威命令集（不累计等待）；供网络收包路径使用。
    pub fn take_ready(&mut self) -> Option<FrameCommands> {
        let mut frame = self.buffered.remove(&self.next_frame)?;
        let ready_frame = self.next_frame;
        self.applied.insert(ready_frame);
        self.next_frame += 1;
        self.waited = 0;
        frame.frame = ready_frame;
        Some(frame)
    }

    /// 累计一次缺帧等待（由确定性逻辑 tick 驱动，不读墙钟）。
    pub fn note_wait(&mut self) {
        self.waited += 1;
    }

    /// 尝试取帧；缺失则累计等待。
    pub fn poll(&mut self) -> FrameStep {
        if let Some(FrameCommands {
            frame,
            commands,
            gameplay,
        }) = self.take_ready()
        {
            return FrameStep::Ready {
                frame,
                commands,
                gameplay,
            };
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

/// 单玩家本帧提交的两类命令（装配器内部缓存）。
#[derive(Clone, Debug, Default)]
struct PlayerInput {
    commands: Vec<Command>,
    gameplay: Vec<GameplayCommand>,
}

/// relay 侧的帧装配器：收集各玩家输入，收齐或超时后产出该帧权威命令集。
pub struct FrameAssembler {
    frame: u64,
    inputs: BTreeMap<u8, PlayerInput>,
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

    /// 记录某玩家本帧提交的两类命令（同一玩家重复提交会覆盖，取最后一次）。
    pub fn submit(&mut self, player: u8, commands: Vec<Command>, gameplay: Vec<GameplayCommand>) {
        self.inputs
            .insert(player, PlayerInput { commands, gameplay });
    }

    /// 推进一个逻辑 tick；收齐全部玩家或超时则关闭并产出该帧。
    /// 两类命令分别按稳定键排序，使各队列的 `arrival` 在两端一致。
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
        let mut gameplay = Vec::new();
        for (_, mut input) in std::mem::take(&mut self.inputs) {
            commands.append(&mut input.commands);
            gameplay.append(&mut input.gameplay);
        }
        stabilize(&mut commands);
        stabilize_gameplay(&mut gameplay);
        let produced = FrameCommands {
            frame: self.frame,
            commands,
            gameplay,
        };
        self.frame += 1;
        self.waited = 0;
        Some(produced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::{
        CommandKind, EntityId, Frame, GameplayCommandKind, MatchConfig, SimFixed, StaticWorld,
    };

    fn command(frame: u64, player: u8, sequence: u32) -> Command {
        Command::new(Frame::new(frame), player, sequence, CommandKind::Noop)
    }

    fn gameplay(frame: u64, player: u8, sequence: u32) -> GameplayCommand {
        GameplayCommand::new(
            Frame::new(frame),
            player,
            sequence,
            GameplayCommandKind::SetAi { enabled: false },
        )
    }

    fn frame_commands(frame: u64, commands: Vec<Command>) -> FrameCommands {
        FrameCommands {
            frame,
            commands,
            gameplay: vec![],
        }
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
    fn stabilize_gameplay_uses_the_same_stable_key() {
        let mut gameplay_cmds = vec![
            gameplay(2, 1, 9),
            gameplay(1, 3, 1),
            gameplay(1, 1, 5),
            gameplay(1, 1, 2),
        ];
        stabilize_gameplay(&mut gameplay_cmds);
        let keys: Vec<_> = gameplay_cmds
            .iter()
            .map(|g| (g.execute_frame.value(), g.player_id, g.sequence))
            .collect();
        assert_eq!(keys, vec![(1, 1, 2), (1, 1, 5), (1, 3, 1), (2, 1, 9)]);
    }

    #[test]
    fn frame_input_stable_key_uses_fixed_type_tag() {
        assert_eq!(TYPE_TAG_SIMULATION, 0);
        assert_eq!(TYPE_TAG_GAMEPLAY, 1);
        let c = command(1, 1, 1);
        let g = gameplay(1, 1, 1);
        // 同 (frame, player, sequence)：Simulation(0) 必须排在 Gameplay(1) 前。
        let mut inputs = vec![FrameInput::Gameplay(&g), FrameInput::Simulation(&c)];
        stabilize_frame_inputs(&mut inputs);
        assert!(matches!(inputs[0], FrameInput::Simulation(_)));
        assert!(matches!(inputs[1], FrameInput::Gameplay(_)));
        assert_eq!(inputs[0].stable_key(), (1, 1, 1, TYPE_TAG_SIMULATION));
        assert_eq!(inputs[1].stable_key(), (1, 1, 1, TYPE_TAG_GAMEPLAY));
        // 稳定键按 frame 优先，frame 小的永远在前。
        let c0 = command(0, 9, 9);
        let mut mixed = vec![FrameInput::Gameplay(&g), FrameInput::Simulation(&c0)];
        stabilize_frame_inputs(&mut mixed);
        assert_eq!(mixed[0].stable_key().0, 0);
        assert_eq!(mixed[1].stable_key().0, 1);
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
    fn scheduler_buffers_both_command_types_and_skips_replays() {
        let mut scheduler = FrameScheduler::new(0, 2);
        let frame = FrameCommands {
            frame: 0,
            commands: vec![command(0, 2, 2), command(0, 1, 1)],
            gameplay: vec![gameplay(0, 2, 8), gameplay(0, 1, 7)],
        };
        scheduler.offer([frame]);
        let ready = scheduler.take_ready().unwrap();
        // 两类命令分别稳定排序，到达顺序不决定执行顺序。
        assert_eq!(ready.commands[0].player_id, 1);
        assert_eq!(ready.gameplay[0].player_id, 1);
        // 重放（含玩法命令）不重复执行。
        scheduler.offer([FrameCommands {
            frame: 0,
            commands: vec![command(0, 1, 1)],
            gameplay: vec![gameplay(0, 1, 7)],
        }]);
        assert!(scheduler.take_ready().is_none(), "replay must not repeat");
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
                commands: vec![command(4, 1, 1)],
                gameplay: vec![],
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
        // 乱序到达的两类命令。
        let commands = vec![command(2, 1, 5), spawn, command(1, 0, 3)];
        let gameplay_cmds = vec![gameplay(2, 1, 6), gameplay(1, 0, 4)];
        let mut a = new_match();
        let mut b = new_match();
        let hash_a = apply_frame(&mut a, &commands, &gameplay_cmds).unwrap();
        let hash_b = apply_frame(&mut b, &commands, &gameplay_cmds).unwrap();
        assert_eq!(hash_a, hash_b);
        assert_eq!(a.read_view(), b.read_view());
        assert_eq!(a.first_difference(&b), None);
    }

    #[test]
    fn apply_frame_submits_both_types_before_exactly_one_step() {
        let mut sim = new_match();
        let before = sim.read_view().tick;
        let commands = vec![command(0, 1, 1)];
        let gameplay_cmds = vec![gameplay(0, 1, 2)];
        let hash = apply_frame(&mut sim, &commands, &gameplay_cmds).unwrap();
        assert_eq!(sim.read_view().tick, before + 1, "exactly one step");
        assert_eq!(hash, sim.compute_state_hash());
        // 与只含普通命令的基线哈希不同，证明玩法命令确实进入权威提交路径。
        let mut baseline = new_match();
        let baseline_hash = apply_frame(&mut baseline, &commands, &[]).unwrap();
        assert_ne!(hash, baseline_hash, "gameplay commands must be submitted");
    }

    #[test]
    fn apply_frame_rejects_gameplay_frame_in_past() {
        let mut sim = new_match();
        sim.step();
        let current = sim.read_view().tick;
        let past = vec![gameplay(0, 1, 1)];
        assert!(matches!(
            apply_frame(&mut sim, &[], &past),
            Err(NetError::FrameInPast { .. })
        ));
        assert_eq!(sim.read_view().tick, current, "rejection must not step");
    }

    #[test]
    fn apply_frame_rejects_gameplay_invalid_entity() {
        let mut sim = new_match();
        let before = sim.read_view().tick;
        let invalid = GameplayCommand::new(
            Frame::new(0),
            1,
            1,
            GameplayCommandKind::Gather {
                unit: EntityId::NONE,
            },
        );
        assert!(matches!(
            apply_frame(&mut sim, &[], &[invalid]),
            Err(NetError::Session(_))
        ));
        assert_eq!(sim.read_view().tick, before, "rejection must not step");
    }

    #[test]
    fn assembler_closes_frame_on_timeout_with_missing_player() {
        let mut assembler = FrameAssembler::new(0, &[1, 2], 2);
        assembler.submit(1, vec![command(0, 1, 1)], vec![]);
        assert!(assembler.tick().is_none());
        // 玩家 2 始终没提交；超时后按空输入关闭帧 0。
        let produced = assembler.tick().unwrap();
        assert_eq!(produced.frame, 0);
        assert_eq!(produced.commands.len(), 1);
        assert!(produced.gameplay.is_empty());
        assert_eq!(assembler.current_frame(), 1);
    }

    #[test]
    fn assembler_closes_immediately_when_all_players_submitted() {
        let mut assembler = FrameAssembler::new(5, &[1, 2], 10);
        assembler.submit(2, vec![command(5, 2, 1)], vec![]);
        assembler.submit(1, vec![command(5, 1, 1)], vec![]);
        let produced = assembler.tick().unwrap();
        assert_eq!(produced.frame, 5);
        // 稳定排序：player 1 在前。
        assert_eq!(produced.commands[0].player_id, 1);
        assert_eq!(assembler.current_frame(), 6);
    }

    #[test]
    fn assembler_merges_both_command_types_in_stable_order() {
        let mut assembler = FrameAssembler::new(5, &[1, 2], 10);
        assembler.submit(
            2,
            vec![command(5, 2, 1)],
            vec![gameplay(5, 2, 2), gameplay(5, 2, 0)],
        );
        assembler.submit(1, vec![command(5, 1, 1)], vec![gameplay(5, 1, 3)]);
        let produced = assembler.tick().unwrap();
        assert_eq!(produced.commands.len(), 2);
        assert_eq!(produced.gameplay.len(), 3);
        // 两类命令各自按 (execute_frame, player_id, sequence) 稳定排序。
        let gameplay_keys: Vec<_> = produced
            .gameplay
            .iter()
            .map(|g| (g.player_id, g.sequence))
            .collect();
        assert_eq!(gameplay_keys, vec![(1, 3), (2, 0), (2, 2)]);
        assert_eq!(assembler.current_frame(), 6);
    }
}
