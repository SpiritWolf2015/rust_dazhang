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
//!
//! P8.2（design D5 / D3）：技能 / 升级 ID 不再硬编码，改由
//! `defs.rs::Race` 的只读分派方法（`hero_ability` / `harass_ability` /
//! `weapons_upgrade` / `armor_upgrade`）按 `setup.race` 取用。默认
//! [`MeleeSetup::new`] 回落人族段、命令字节与 P8.1 完全一致；以
//! `MeleeSetup::with_race(Race::Orc)` 构造兽族开局时，脚本会训练兽族
//! 新单位、施放兽族英雄技能、研发兽族升级。仍是帧冻结外部命令、不新增
//! 权威状态 / 命令变体。

use w3_core::{Frame, GameplayCommand, GameplayCommandKind};

use crate::melee::defs::{CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE, SOLDIER};
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
    ///
    /// P8.2：训练目标 `DefId` 经 [`MeleeSetup::category_def_ids`] 查表（已随
    /// `Race` 分派替换占位而自动跟随种族）；技能 / 升级 ID 经 `setup.race` 的
    /// 只读分派方法取用，默认人族段与 P8.1 字节一致、兽族开局产出兽族内容，
    /// 不再硬编码任何技能 / 升级 ID（design D5）。
    pub fn new(setup: &MeleeSetup, horizon_frames: u64) -> Self {
        let mut commands = Vec::new();
        // 序列 1 已由开局 `Gather` 占用。
        let mut sequence = 2u32;
        let mut frame = DECISION_PERIOD;
        let mut period_index = 0u64;
        // 新类别装配绑定（槽位 → DefId）：定义层 / 装配层查表，不进权威状态。
        let catalog = setup.category_def_ids();
        // 技能 / 升级 ID 按族只读分派：默认人族段与 P8.1 完全一致，兽族
        // 开局指向 P8.2 新增的 `AbilityDef` / `UpgradeDef`（design D3 / D5）。
        let race = setup.race;
        let hero_ability = race.hero_ability();
        let harass_ability = race.harass_ability();
        let weapons_upgrade = race.weapons_upgrade();
        let armor_upgrade = race.armor_upgrade();

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
            // `upgrade` 经 `Race::weapons_upgrade` / `armor_upgrade` 按族查表：
            // 人族段与 P8.1 一致，兽族开局指向 P8.2 新增 `UpgradeDef`。
            if period_index % RESEARCH_PERIOD == 0 {
                let upgrade = if (period_index / RESEARCH_PERIOD) % 2 == 0 {
                    weapons_upgrade
                } else {
                    armor_upgrade
                };
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 1),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Research { upgrade },
                ));
                sequence += 1;
            }

            // 既有：周期性以既有技能骚扰玩家主城。`ABILITY`（10）两族共用、
            // 不占种族段，经 `harass_ability` 查表返回（design D5）。
            if period_index > 0 && period_index % HARASS_PERIOD == 0 {
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 2),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Cast {
                        caster: setup.ai_hall,
                        ability: harass_ability,
                        target: setup.local_hall,
                    },
                ));
                sequence += 1;
            }

            // 新增：周期性施放英雄主动技能（`ability` 指向新增 `AbilityDef`），
            // 走既有 `Cast` 命令面（R6 / R7.6）；施法者用 frame-0 固定实体 ID，
            // 运行时训练出的英雄实体在端到端一局中另行施放（design D5）。
            // `hero_ability` 按族查表：兽族开局施放 P8.2 新增主动技能。
            if period_index > 0 && period_index % HERO_CAST_PERIOD == 0 {
                commands.push(GameplayCommand::new(
                    Frame::new(frame + 2),
                    AI_PLAYER,
                    sequence,
                    GameplayCommandKind::Cast {
                        caster: setup.ai_hall,
                        ability: hero_ability,
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
    use std::collections::BTreeMap;

    use w3_core::DefId;

    use super::*;
    use crate::melee::defs::{
        CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_SMITHY, HERO_ABILITY, ORC_ALTAR, ORC_BARRACKS,
        ORC_FARM, ORC_HERO, ORC_HERO_ABILITY, ORC_RANGED, ORC_SIEGE, ORC_SMITHY, ORC_SOLDIER,
        ORC_UPGRADE_ARMOR, ORC_UPGRADE_WEAPONS, Race, UPGRADE_ARMOR, UPGRADE_WEAPONS,
    };

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

    /// P8.2（design D5 / tasks 3.2）：兽族开局脚本会训练兽族新单位 / 施放
    /// 兽族英雄技能 / 研发兽族升级，全部由 `setup.race` 的只读分派与装配层
    /// 新类别绑定查表产出，MUST NOT 出现人族段的技能 / 升级 ID。
    #[test]
    fn script_contains_orc_new_category_commands() {
        let setup = MeleeSetup::with_race(Race::Orc);
        let script = AiScript::new(&setup, 1200);
        let catalog = setup.category_def_ids();
        let orc_new_units = [ORC_RANGED, ORC_SIEGE, ORC_HERO];

        // 训练目标来自装配层绑定、且已替换占位（不再是 ORC_SOLDIER / ORC_BARRACKS）。
        assert_eq!(catalog[&CATEGORY_RANGED], ORC_RANGED);
        assert_eq!(catalog[&CATEGORY_SIEGE], ORC_SIEGE);
        assert_eq!(catalog[&CATEGORY_FARM], ORC_FARM);
        assert_eq!(catalog[&CATEGORY_ALTAR], ORC_ALTAR);
        assert_eq!(catalog[&CATEGORY_SMITHY], ORC_SMITHY);
        assert_eq!(catalog[&CATEGORY_HERO], ORC_HERO);
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Train { unit_def, .. } if orc_new_units.contains(&unit_def)
            )),
            "reason=missing_train_pointing_to_orc_new_unit_def"
        );
        // 施放兽族英雄技能、研发兽族升级。
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Cast { ability, .. } if ability == ORC_HERO_ABILITY
            )),
            "reason=missing_cast_pointing_to_orc_ability_def"
        );
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Research { upgrade }
                    if upgrade == ORC_UPGRADE_WEAPONS || upgrade == ORC_UPGRADE_ARMOR
            )),
            "reason=missing_research_pointing_to_orc_upgrade_def"
        );
        // 兽族脚本 MUST NOT 泄漏人族段的技能 / 升级 ID（按族分派生效）。
        assert!(
            !script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Cast { ability, .. } if ability == HERO_ABILITY
            )),
            "reason=human_ability_leaked_into_orc_script"
        );
        assert!(
            !script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Research { upgrade }
                    if upgrade == UPGRADE_WEAPONS || upgrade == UPGRADE_ARMOR
            )),
            "reason=human_upgrade_leaked_into_orc_script"
        );
        assert!(
            script.commands.iter().all(|c| c.player_id == AI_PLAYER),
            "reason=all_commands_belong_to_ai_player"
        );
    }

    /// 默认人族回落（design D5 / tasks 3.2）：`MeleeSetup::new` 的脚本技能 /
    /// 升级 ID 与 P8.1 完全一致，不因兽族分派而改变既有入口行为。
    #[test]
    fn default_script_keeps_human_ability_and_upgrade_ids() {
        let setup = MeleeSetup::new();
        let script = AiScript::new(&setup, 1200);
        assert_eq!(setup.race, Race::Human);
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Cast { ability, .. } if ability == HERO_ABILITY
            )),
            "reason=default_hero_ability_unchanged"
        );
        assert!(
            script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Research { upgrade }
                    if upgrade == UPGRADE_WEAPONS || upgrade == UPGRADE_ARMOR
            )),
            "reason=default_upgrade_ids_unchanged"
        );
        assert!(
            !script.commands.iter().any(|c| matches!(
                c.kind,
                GameplayCommandKind::Cast { ability, .. } if ability == ORC_HERO_ABILITY
            )),
            "reason=orc_ability_must_not_leak_into_default_script"
        );
    }

    /// 反向验证（D9 / tasks 6.4）：退化装配层绑定（兽族六个新类别仍回落
    /// ORC_SOLDIER / ORC_BARRACKS 占位）会让「脚本训练兽族新单位」断言失败，
    /// 正式实现 `Race::Orc.category_def_ids()` 通过同一断言。退化实现经真实
    /// `AiScript::new` 查表路径注入，不是旁路模拟。
    #[test]
    fn reverse_validation_orc_dispatch_formal_passes_the_assertion() {
        let setup = MeleeSetup::with_race(Race::Orc);
        assert!(
            script_trains_orc_new_units(&setup),
            "reason=formal_dispatch_must_pass_the_assertion"
        );
    }

    #[test]
    fn reverse_validation_orc_dispatch_degenerate_fails_the_same_assertion() {
        // 退化实现：把装配层绑定替换回 P8.1 时代的占位表，喂给真实
        // `AiScript::new`，其 `unit_def` 全部落到 ORC_SOLDIER / ORC_BARRACKS。
        let mut degenerate = MeleeSetup::with_race(Race::Orc);
        degenerate.category_def_ids = placeholder_catalog();
        assert!(
            !script_trains_orc_new_units(&degenerate),
            "reason=degenerate_dispatch_must_fail_the_same_assertion"
        );
        // 元断言：正式实现仍能通过，证明缺陷被抓出而非断言失效。
        let formal = MeleeSetup::with_race(Race::Orc);
        assert!(script_trains_orc_new_units(&formal));
    }

    /// 被反向验证的断言：脚本含指向兽族新增 `UnitDef` 的 `Train`。
    fn script_trains_orc_new_units(setup: &MeleeSetup) -> bool {
        let orc_new_units = [ORC_RANGED, ORC_SIEGE, ORC_HERO];
        AiScript::new(setup, 1200).commands.iter().any(|c| {
            matches!(
                c.kind,
                GameplayCommandKind::Train { unit_def, .. }
                    if orc_new_units.contains(&unit_def)
            )
        })
    }

    /// P8.1 时代的兽族占位绑定（缺陷态，仅为反向验证注入用）。
    fn placeholder_catalog() -> BTreeMap<u64, DefId> {
        [
            (CATEGORY_RANGED, ORC_SOLDIER),
            (CATEGORY_SIEGE, ORC_SOLDIER),
            (CATEGORY_FARM, ORC_BARRACKS),
            (CATEGORY_ALTAR, ORC_BARRACKS),
            (CATEGORY_SMITHY, ORC_BARRACKS),
            (CATEGORY_HERO, ORC_SOLDIER),
        ]
        .into_iter()
        .collect()
    }
}
