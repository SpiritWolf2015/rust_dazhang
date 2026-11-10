//! 断线重连、追帧与重放不重复执行。
//!
//! 客户端保留 `player_id` 与输入游标 `next_frame`（下一个待应用帧号）；
//! 重连后只补 `next_frame` 及之后的帧，且同一帧只应用一次，因此已提交
//! 的命令与历史事件都不会重复产生（spec Scenario）。

use std::collections::{BTreeMap, BTreeSet};

use crate::error::NetError;
use crate::packet::{
    FrameCommands, Packet, PacketKind, payload_resync_replay, payload_resync_request,
};

/// 客户端持久身份与输入游标。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionCursor {
    pub player_id: u8,
    /// 下一个待应用的帧号；重连补发从此帧开始。
    pub next_frame: u64,
}

impl SessionCursor {
    pub fn new(player_id: u8, next_frame: u64) -> Self {
        Self {
            player_id,
            next_frame,
        }
    }

    pub fn next_expected_frame(&self) -> u64 {
        self.next_frame
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconnectPhase {
    Healthy,
    Disconnected,
    CatchingUp,
}

/// 重连状态机 + 已应用帧集合。
pub struct ReconnectController {
    cursor: SessionCursor,
    applied_frames: BTreeSet<u64>,
    phase: ReconnectPhase,
}

impl ReconnectController {
    pub fn new(cursor: SessionCursor) -> Self {
        Self {
            cursor,
            applied_frames: BTreeSet::new(),
            phase: ReconnectPhase::Healthy,
        }
    }

    pub fn cursor(&self) -> SessionCursor {
        self.cursor
    }

    pub fn phase(&self) -> ReconnectPhase {
        self.phase
    }

    pub fn disconnect(&mut self) {
        self.phase = ReconnectPhase::Disconnected;
    }

    pub fn begin_catch_up(&mut self) {
        self.phase = ReconnectPhase::CatchingUp;
    }

    pub fn finish_catch_up(&mut self) {
        self.phase = ReconnectPhase::Healthy;
    }

    /// 从补发帧流中筛出尚未应用、按帧号排序且去重的帧计划。
    /// 不改状态；实际 `apply_frame` 成功后才调用 `commit`。
    pub fn plan_replay(&self, frames: &[FrameCommands]) -> Vec<FrameCommands> {
        let mut grouped: BTreeMap<u64, FrameCommands> = BTreeMap::new();
        for frame in frames {
            if frame.frame < self.cursor.next_frame || self.applied_frames.contains(&frame.frame) {
                continue;
            }
            // 同一帧只保留第一份，避免重复执行。
            grouped.entry(frame.frame).or_insert_with(|| frame.clone());
        }
        grouped.into_values().collect()
    }

    /// 标记某帧已成功应用并推进游标。
    pub fn commit(&mut self, frame: u64) {
        self.applied_frames.insert(frame);
        self.cursor.next_frame = self.cursor.next_frame.max(frame.saturating_add(1));
    }

    pub fn applied_count(&self) -> usize {
        self.applied_frames.len()
    }
}

/// 构造重连请求；wire 上携带的是补发起始帧号。
pub fn resync_request_packet(sender: u8, cursor: SessionCursor) -> Packet {
    Packet::new(
        PacketKind::ResyncRequest,
        sender,
        0,
        cursor.next_frame,
        payload_resync_request(cursor.player_id, cursor.next_frame),
    )
}

/// 构造补发命令流包。
pub fn resync_replay_packet(sender: u8, frames: &[FrameCommands]) -> Result<Packet, NetError> {
    Ok(Packet::new(
        PacketKind::ResyncReplay,
        sender,
        0,
        0,
        payload_resync_replay(frames)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{parse_resync_replay, parse_resync_request};
    use w3_core::{Command, CommandKind, Frame};

    fn command(frame: u64, player: u8, sequence: u32) -> Command {
        Command::new(Frame::new(frame), player, sequence, CommandKind::Noop)
    }

    fn frame_commands(frame: u64, sequence: u32) -> FrameCommands {
        FrameCommands {
            frame,
            commands: vec![command(frame, 1, sequence)],
            gameplay: vec![],
        }
    }

    #[test]
    fn cursor_points_to_next_frame() {
        let cursor = SessionCursor::new(2, 7);
        assert_eq!(cursor.next_expected_frame(), 7);
    }

    #[test]
    fn replay_plan_filters_applied_frames_sorts_and_deduplicates() {
        // 已应用到帧 3，下一个待应用帧为 4。
        let mut controller = ReconnectController::new(SessionCursor::new(1, 4));
        // 含已应用帧、乱序、重复帧。
        let frames = vec![
            frame_commands(6, 1),
            frame_commands(2, 2),
            frame_commands(5, 3),
            frame_commands(5, 99),
            frame_commands(4, 4),
        ];
        let plan = controller.plan_replay(&frames);
        let frame_numbers: Vec<u64> = plan.iter().map(|f| f.frame).collect();
        assert_eq!(frame_numbers, vec![4, 5, 6]);
        // 同一帧只保留第一份。
        assert_eq!(plan[1].commands[0].sequence, 3);

        for frame in &plan {
            controller.commit(frame.frame);
        }
        assert_eq!(controller.cursor().next_frame, 7);
        assert_eq!(controller.applied_count(), 3);
        // 再次计划不会重复产出。
        assert!(controller.plan_replay(&frames).is_empty());
    }

    #[test]
    fn phase_transitions() {
        let mut controller = ReconnectController::new(SessionCursor::new(1, 0));
        assert_eq!(controller.phase(), ReconnectPhase::Healthy);
        controller.disconnect();
        assert_eq!(controller.phase(), ReconnectPhase::Disconnected);
        controller.begin_catch_up();
        assert_eq!(controller.phase(), ReconnectPhase::CatchingUp);
        controller.finish_catch_up();
        assert_eq!(controller.phase(), ReconnectPhase::Healthy);
    }

    #[test]
    fn resync_packets_roundtrip() {
        let cursor = SessionCursor::new(2, 11);
        let request = resync_request_packet(2, cursor);
        assert_eq!(request.kind, PacketKind::ResyncRequest);
        assert_eq!(parse_resync_request(&request.payload).unwrap(), (2, 11));

        let frames = vec![frame_commands(12, 1), frame_commands(13, 2)];
        let replay = resync_replay_packet(0, &frames).unwrap();
        assert_eq!(replay.kind, PacketKind::ResyncReplay);
        assert_eq!(parse_resync_replay(&replay.payload).unwrap(), frames);
    }
}
