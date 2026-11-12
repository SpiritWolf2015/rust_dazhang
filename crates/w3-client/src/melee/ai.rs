//! 确定性外部 AI 命令源（design D3）。
//!
//! 按冻结帧号预生成对手的玩法命令，全部经 `submit_gameplay_command` 提交；
//! 不扩展 `w3-core::run_ai`、不新增权威状态。这些是**外部**命令，带固定的
//! `ExecuteFrame` / `PlayerId` / `Sequence`，完整重放注入同一脚本即可复现，
//! 不会被当作内部 AI 命令二次注入。

use w3_core::{Frame, GameplayCommand, GameplayCommandKind};

use crate::melee::defs::{ABILITY, SOLDIER};
use crate::melee::setup::{AI_PLAYER, MeleeSetup};

/// AI 决策周期（整数逻辑帧）。
const DECISION_PERIOD: u64 = 60;
/// 骚扰周期：每若干个决策周期让主城施放一次技能，形成周期性进攻。
const HARASS_PERIOD: u64 = 4;

/// 一局中预生成的 AI 外部命令脚本。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiScript {
    /// 按 `(execute_frame, player_id, sequence)` 升序排列。
    pub commands: Vec<GameplayCommand>,
}

impl AiScript {
    /// 预生成对手命令直到 `horizon_frames`。
    ///
    /// 每个决策周期让 AI 农民采集、主城训练士兵；每隔若干周期让主城对
    /// 玩家主城施放技能形成骚扰。全部 ID 来自开局布置，均为固定值。
    pub fn new(setup: &MeleeSetup, horizon_frames: u64) -> Self {
        let mut commands = Vec::new();
        // 序列 1 已由开局 `Gather` 占用。
        let mut sequence = 2u32;
        let mut frame = DECISION_PERIOD;
        let mut period_index = 0u64;

        while frame <= horizon_frames {
            for &unit in &setup.ai_peasants {
                commands.push(GameplayCommand::new(
                    Frame::new(frame),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Gather { unit },
                ));
                sequence += 1;
            }
            commands.push(GameplayCommand::new(
                Frame::new(frame + 1),
                AI_PLAYER,
                sequence,
                GameplayCommandKind::Train {
                    producer: setup.ai_hall,
                    unit_def: SOLDIER,
                    is_building: false,
                },
            ));
            sequence += 1;

            if period_index > 0 && period_index % HARASS_PERIOD == 0 {
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 2),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Cast {
                        caster: setup.ai_hall,
                        ability: ABILITY,
                        target: setup.local_hall,
                    },
                ));
                sequence += 1;
            }

            frame += DECISION_PERIOD;
            period_index += 1;
        }

        commands.sort_by_key(|command| {
            (
                command.execute_frame.value(),
                command.player_id,
                command.sequence,
            )
        });
        Self { commands }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_deterministic_and_ordered() {
        let setup = MeleeSetup::new();
        let script = AiScript::new(&setup, 600);
        assert!(!script.commands.is_empty());
        assert_eq!(script, AiScript::new(&setup, 600));
        assert!(script.commands.windows(2).all(|pair| {
            (
                pair[0].execute_frame.value(),
                pair[0].player_id,
                pair[0].sequence,
            ) <= (
                pair[1].execute_frame.value(),
                pair[1].player_id,
                pair[1].sequence,
            )
        }));
        assert!(script.commands.iter().all(|c| c.player_id == AI_PLAYER));
        assert!(
            script
                .commands
                .iter()
                .all(|c| c.execute_frame.value() <= 602)
        );
    }

    #[test]
    fn script_contains_gather_train_and_harass() {
        let setup = MeleeSetup::new();
        let script = AiScript::new(&setup, 1200);
        assert!(
            script
                .commands
                .iter()
                .any(|c| matches!(c.kind, GameplayCommandKind::Gather { .. }))
        );
        assert!(
            script
                .commands
                .iter()
                .any(|c| matches!(c.kind, GameplayCommandKind::Train { .. }))
        );
        assert!(
            script
                .commands
                .iter()
                .any(|c| matches!(c.kind, GameplayCommandKind::Cast { .. }))
        );
    }
}
