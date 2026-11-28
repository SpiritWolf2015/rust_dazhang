//! 开局选族（P8.4 / Q1）：表现层本地只读状态与装配期传参接缝。
//!
//! 本模块只持有 [`FactionSelection`] 这一表现层状态与一个把选族结果交给
//! 既有 [`MeleeSetup::with_races`] 的装配期注入点。窗口内 egui overlay 的
//! 绘制与交互在 [`crate::hud`]，状态机与按本地族刷新命令卡在
//! [`crate::app::ClientApp`]；权威装配只发生在 example 的启动期。
//!
//! 硬红线（ADR 0002 / 主 `dual-faction/spec.md:25` / design D1）：
//! - 选族 MUST NOT 新增 `Player.race` / `RtsMatch.race` 或任何权威状态字段；
//!   MUST NOT 进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 10）定义的
//!   快照或 `first_difference()`；`w3-core` 零改动。
//! - 选族只体现在开局装配层 `EntityId → DefId` 关联的段位（人族 `1xx` /
//!   兽族 `2xx` / 中立 `9xx`）与按族只读查表的命令卡目标上；
//!   `CommandKind::Spawn` 不携带 `DefId`，十六组合 `Spawn` / `gameplay_commands`
//!   字节逐字节不变（P8.0 决策 / P8.3 已证）。
//! - 默认（未确认 / 未选）回落本地人族 / AI 兽族，与
//!   [`MeleeSetup::new`] 完全一致；`MeleeSetup::new` / `with_race`
//!   兼容别名与既有回落基线测试一律不动。

use crate::melee::{MeleeSetup, Race};

/// 开局前可见的本地族 / AI 族选择（表现层本地只读状态）。
///
/// 该结构只存在于 `w3-client`，不是权威状态；`confirmed` 只是窗口内
/// egui overlay 的状态机标志（是否已点确认），MUST NOT 进入权威状态、
/// 状态哈希或快照。UI 文案与字体另走 [`crate::i18n`] / [`crate::hud`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactionSelection {
    /// 本地席位种族（决定本地席位 `def_ids` 段与命令卡目标段）。
    pub local_race: Race,
    /// AI 席位种族（决定 AI 席位 `def_ids` 段与 AI 命令源目标段）。
    pub ai_race: Race,
    /// overlay 状态机：选族是否已确认（纯表现层，不进权威状态）。
    pub confirmed: bool,
}

impl Default for FactionSelection {
    /// 未选族回落本地人族 / AI 兽族，与 `MeleeSetup::new()` 一致（Q1）。
    fn default() -> Self {
        Self {
            local_race: Race::Human,
            ai_race: Race::Orc,
            confirmed: false,
        }
    }
}

impl FactionSelection {
    /// 构造默认选族（本地人族 / AI 兽族、未确认）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 由两个独立种族参数构造已确认的选族（P9 / Q1' 4×4 十六组合可装配）。
    pub fn with_races(local_race: Race, ai_race: Race) -> Self {
        Self {
            local_race,
            ai_race,
            confirmed: true,
        }
    }

    /// 切换本地族（overlay 交互，纯表现层）。
    pub fn with_local_race(mut self, race: Race) -> Self {
        self.local_race = race;
        self
    }

    /// 切换 AI 族（overlay 交互，纯表现层）。
    pub fn with_ai_race(mut self, race: Race) -> Self {
        self.ai_race = race;
        self
    }

    /// 确认选族（overlay 状态机，纯表现层、不生成任何命令）。
    pub fn confirm(mut self) -> Self {
        self.confirmed = true;
        self
    }

    /// 装配期把 `(local_race, ai_race)` 交给既有 [`MeleeSetup::with_races`]。
    ///
    /// 这是生产唯一路径：默认（Human / Orc）结果与 `MeleeSetup::new()`
    /// 逐字节一致，故未确认时天然回落既有基线，无需特判 `confirmed`。
    /// 本函数不提交任何命令、不触碰权威状态；`Spawn` 字节族无关。
    pub fn assemble_setup(&self) -> MeleeSetup {
        assemble_from_selection(*self, MeleeSetup::with_races)
    }
}

/// 装配期注入点：把选族结果的两个 `Race` 交给 `assemble` 并返回装配层数据。
///
/// 生产实现传 [`MeleeSetup::with_races`] 本身；反向验证（改坏验红 Q1）
/// 注入忽略选族的退化实现，二者都经本函数与 `with_races` 的真实装配
/// 路径、非旁路模拟（先例 `p8-dual-faction` 的成对装配函数）。
/// 退化实现若继续用硬编码 `MeleeSetup::new()` 等价参数，异族组合的
/// 「本地席位落所选族段」断言必然失败。
pub fn assemble_from_selection<F>(selection: FactionSelection, assemble: F) -> MeleeSetup
where
    F: Fn(Race, Race) -> MeleeSetup,
{
    assemble(selection.local_race, selection.ai_race)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认选族与 `MeleeSetup::new()` 逐字段一致（Q1 回落基线）。
    #[test]
    fn default_selection_falls_back_to_melee_setup_new() {
        let selection = FactionSelection::default();
        assert_eq!(selection.local_race, Race::Human);
        assert_eq!(selection.ai_race, Race::Orc);
        assert!(!selection.confirmed);
        let assembled = selection.assemble_setup();
        let baseline = MeleeSetup::new();
        assert_eq!(assembled.commands, baseline.commands);
        assert_eq!(assembled.gameplay_commands, baseline.gameplay_commands);
        assert_eq!(assembled.def_ids, baseline.def_ids);
        assert_eq!(assembled.race, baseline.race);
        assert_eq!(assembled.ai_race, baseline.ai_race);
    }

    /// overlay 切换 / 确认只改表现层字段，不改变装配函数的确定性。
    #[test]
    fn overlay_transitions_are_presentation_only() {
        let selection = FactionSelection::new()
            .with_local_race(Race::Orc)
            .with_ai_race(Race::Human)
            .confirm();
        assert_eq!(selection.local_race, Race::Orc);
        assert_eq!(selection.ai_race, Race::Human);
        assert!(selection.confirmed);
        let setup = selection.assemble_setup();
        assert_eq!(setup.race, Race::Orc);
        assert_eq!(setup.ai_race, Race::Human);
        // 种族只停装配层关联，权威 Spawn 字节与默认基线逐字节一致。
        assert_eq!(setup.commands, MeleeSetup::new().commands);
    }
}
