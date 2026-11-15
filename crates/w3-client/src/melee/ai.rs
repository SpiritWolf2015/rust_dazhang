//! 确定性外部 AI 命令源（design D3 / D5）。
//!
//! 按冻结帧号预生成对手的玩法命令，全部经 `submit_gameplay_command` 提交；
//! 不扩展 `w3-core::run_ai`、不新增权威状态。这些是**外部**命令，带固定的
//! `ExecuteFrame` / `PlayerId` / `Sequence`，完整重放注入同一脚本即可复现，
//! 不会被当作内部 AI 命令二次注入。
//!
//! P8.1（design D5）：在既有采集 / 训练 / 骚扰之外，增加训练人族新单位、
//! 施放英雄主动技能、研发攻防升级三类命令，全部是 `GameplayCommandKind`
//! 既有变体；新类别 `DefId` 经装配层 [`MeleeSetup::category_def_ids`]
//! 查表得到，MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`。

use w3_core::{Frame, GameplayCommand, GameplayCommandKind};

use crate::melee::defs::{
    ABILITY, CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE, HERO_ABILITY, SOLDIER, UPGRADE_ARMOR,
    UPGRADE_WEAPONS,
};
use crate::melee::setup::{AI_PLAYER, MeleeSetup};

/// AI 决策周期（整数逻辑帧）。
const DECISION_PERIOD: u64 = 60;
/// 骚扰周期：每若干个决策周期让主城施放一次技能，形成周期性进攻。
const HARASS_PERIOD: u64 = 4;
/// 英雄主动技能周期：每若干个决策周期施放一次新增技能（R6 / R7.6）。
const HERO_CAST_PERIOD: u64 = 2;
/// 研发周期：每若干个决策周期研发一项攻 / 防升级（R4.4）。
const RESEARCH_PERIOD: u64 = 3;

/// 一局中预生成的 AI 外部命令脚本。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiScript {
    /// 按 `(execute_frame, player_id, sequence)` 升序排列。
    pub commands: Vec<GameplayCommand>,
}

impl AiScript {
    /// 预生成对手命令直到 `horizon_frames`。
    ///
    /// 每个决策周期：既有采集 / 训练士兵 / 周期性骚扰，叠加 P8.1 三类新命令
    /// （训练新单位、施放英雄主动技能、研发攻防升级）；`SetAi` 显式声明该
    /// 席位由外部帧冻结脚本接管（`enabled: false`，不启用内核内部 `run_ai`，
    /// 保持 design D5 的单一外部命令源）；`Leave` 落在规划窗口末尾作为确定性
    /// 结算命令（R11.3 / R12.7）。全部 ID 来自开局布置或装配层新类别绑定，
    /// 均为固定值；`horizon_frames < DECISION_PERIOD` 时脚本为空，便于隔离。
    pub fn new(setup: &MeleeSetup, horizon_frames: u64) -> Self {
        let mut commands = Vec::new();
        // 序列 1 已由开局 `Gather` 占用。
        let mut sequence = 2u32;
        let mut frame = DECISION_PERIOD;
        let mut period_index = 0u64;
        // 新类别装配绑定（槽位 → DefId）：定义层 / 装配层查表，不进权威状态。
        let catalog = setup.category_def_ids();

        while frame <= horizon_frames {
            // 既有：AI 农民采集。
            for &unit in &setup.ai_peasants {
                commands.push(GameplayCommand::new(
                    Frame::new(frame),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Gather { unit },
                ));
                sequence += 1;
            }
            // 既有：主城训练士兵。
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

            // 新增：训练人族新单位（远程 / 反重甲 / 英雄按周期轮转），
            // `unit_def` 指向新增 `UnitDef`，走既有 `Train` 命令面（R4.3）。
            let new_unit = match period_index % 3 {
                0 => catalog[&CATEGORY_RANGED],
                1 => catalog[&CATEGORY_SIEGE],
                _ => catalog[&CATEGORY_HERO],
            };
            commands.push(GameplayCommand::new(
                Frame::new(frame + 1),
                AI_PLAYER,
                sequence,
                GameplayCommandKind::Train {
                    producer: setup.ai_hall,
                    unit_def: new_unit,
                    is_building: false,
                },
            ));
            sequence += 1;

            // 新增：研发攻 / 防升级（交替），走既有 `Research` 路径（R4.4）。
            if period_index % RESEARCH_PERIOD == 0 {
                let upgrade = if (period_index / RESEARCH_PERIOD) % 2 == 0 {
                    UPGRADE_WEAPONS
                } else {
                    UPGRADE_ARMOR
                };
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 1),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Research { upgrade },
                ));
                sequence += 1;
            }

            // 既有：周期性以既有技能骚扰玩家主城。
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

            // 新增：周期性施放英雄主动技能（`ability` 指向新增 `AbilityDef`），
            // 走既有 `Cast` 命令面（R6 / R7.6）；施法者用 frame-0 固定实体 ID，
            // 运行时训练出的英雄实体在端到端一局中另行施放（design D5）。
            if period_index > 0 && period_index % HERO_CAST_PERIOD == 0 {
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 2),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Cast {
                        caster: setup.ai_hall,
                        ability: HERO_ABILITY,
                        target: setup.local_hall,
                    },
                ));
                sequence += 1;
            }

            frame += DECISION_PERIOD;
            period_index += 1;
        }

        if period_index > 0 {
            // `SetAi`：复用既有命令面声明 AI 席位由外部帧冻结脚本驱动，
            // `enabled: false` 显式关闭内核内部 `run_ai`（design D5 单一命令源，
            // 避免内部 AI 对空闲单位额外发采集命令）；不新增权威状态。
            commands.push(GameplayCommand::new(
                Frame::new(1),
                AI_PLAYER,
                sequence,
                GameplayCommandKind::SetAi { enabled: false },
            ));
            sequence += 1;
            // `Leave`：规划窗口末尾的确定性离席 / 结算命令（R11.3 / R12.7），
            // 帧号落在最后一个决策周期之后，不干预窗口内的经济 / 战斗。
            let settle_frame = frame - DECISION_PERIOD + 2;
            commands.push(GameplayCommand::new(
                Frame::new(settle_frame),
                AI_PLAYER,
                sequence,
                GameplayCommandKind::Leave { player: AI_PLAYER },
            ));
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

    /// P8.1（design D5 / tasks 3.1）：脚本含指向新增 `UnitDef` 的 `Train`、
    /// 指向新增 `AbilityDef` 的 `Cast`、指向新增 `UpgradeDef` 的 `Research`，
    /// 以及 `Gather` / `SetAi` / `Leave`，且全部 `player_id` 为 AI 玩家。
    #[test]
    fn script_contains_human_new_category_commands() {
        let setup = MeleeSetup::new();
        let script = AiScript::new(&setup, 1200);
        let catalog = setup.category_def_ids();
        let new_units = [
            catalog[&CATEGORY_RANGED],
            catalog[&CATEGORY_SIEGE],
            catalog[&CATEGORY_HERO],
        ];

        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Train { unit_def, .. } if new_units.contains(&unit_def)
            )),
            "reason=missing_train_pointing_to_new_unit_def"
        );
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Cast { ability, .. } if ability == HERO_ABILITY
            )),
            "reason=missing_cast_pointing_to_new_ability_def"
        );
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Research { upgrade }
                    if upgrade == UPGRADE_WEAPONS || upgrade == UPGRADE_ARMOR
            )),
            "reason=missing_research_pointing_to_new_upgrade_def"
        );
        assert!(
            script
                .commands
                .iter()
                .any(|c| matches!(c.kind, GameplayCommandKind::SetAi { .. }))
        );
        assert!(
            script
                .commands
                .iter()
                .any(|c| matches!(c.kind, GameplayCommandKind::Leave { .. }))
        );
        assert!(
            script.commands.iter().all(|c| c.player_id == AI_PLAYER),
            "reason=all_commands_belong_to_ai_player"
        );
    }

    /// 帧冻结外部命令：同一脚本构造可重复，且 `ExecuteFrame` 与构造次数无关。
    #[test]
    fn script_is_frame_frozen_not_clock_driven() {
        let setup = MeleeSetup::new();
        let a = AiScript::new(&setup, 300);
        let b = AiScript::new(&setup, 300);
        assert_eq!(a, b);
        assert_eq!(AiScript::new(&setup, 0).commands.len(), 0);
        // 序列在同一 `(execute_frame, player_id)` 内唯一，排序键稳定。
        let mut seen = std::collections::BTreeSet::new();
        for command in &a.commands {
            assert!(
                seen.insert((
                    command.execute_frame.value(),
                    command.player_id,
                    command.sequence,
                )),
                "reason=sort_key_must_be_unique"
            );
        }
    }
}
