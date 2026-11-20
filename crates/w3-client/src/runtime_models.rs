//! 运行时动态实体「DefId 段 → 真实 glTF 模型键」的表现层只读派生（Q3 / D3）。
//!
//! ## 为什么只能做到「段级」而不是「精确 DefId 级」
//!
//! 用户 2026-11-18 拍板 Q3「本期就上真实两族 glTF」，规格要求运行时
//! `Train` / `Build` 新实体按 DefId 段（`1xx` 人族 / `2xx` 兽族 / `9xx`
//! 中立）解析真实模型键。但零内核改动（ADR 0002，`w3-core` 零 diff）下
//! **无法可靠拿到运行时实体的精确 DefId**，逐锚核实如下：
//!
//! - `w3-core/src/sim_match.rs:180-192` `EntityView` 字段只有
//!   `id/x/y/facing/speed/hp/owner/alive/has_target`，**无 `def_id`**；
//! - `sim_match.rs:161-177` `EventKind` 只有
//!   `Spawned/Arrived/Stopped/Died/AttackHit/MoveFailed`，`Event` 无
//!   `def_id` 字段；
//! - `sim_match.rs:1314-1348` `complete_production` 只
//!   `entities.spawn()` + `units.insert()`，**不发 `Spawned` 事件**，
//!   且新 `EntityId` 由内核 `EntityId` 分配器递增给出，表现层无从反推
//!   该实体由哪条 `Train { unit_def, .. }` 命令产出。
//!
//! 因此「记录表现层发出的 `Train` / `Build` 命令并关联新 `EntityId`」的路径
//! **不是确定性的**：AI 命令由 `MeleeDriver::new` 在装配期一次性入队、不经
//! `ClientApp::apply_input`；联机帧命令经 `w3_net::apply_frame` 到达，本地
//! 不经过同一提交点；回放重放脚本而不重放输入手势。规格 MUST「回放 / 联机
//! 下同一输入产生同一模型键」，故本 change 不采用命令记录关联。
//!
//! ## 采用的确定性只读派生（`owner` 席位 → 族段 + `speed` 只读特征）
//!
//! - `EntityView.owner`：`w3_client::input::LOCAL_PLAYER`（1）本地席位 =
//!   `FactionSelection::local_race` 段；`melee::AI_PLAYER`（2）AI 席位 =
//!   `ai_race` 段；其它 owner 视为中立 `9xx` 段。选族是表现层本地只读
//!   状态（Q1 / ADR 0002），同一对局内固定，回放 / 联机一致。
//! - `EntityView.speed`：`complete_production` 用 `UnitDef.speed_raw` 写入
//!   `UnitState.speed`（建筑 / 资源 `0`、可移动单位 `> 0`）；这与
//!   [`crate::input::effective_movability`] 的 R1.4 分层判据同源，是只读
//!   视图里可靠的「单位 / 建筑」特征。
//! - 两者组合查编译期整数 `DefId` 段定义表 [`SEGMENT_MODEL_KEYS`]，产出该段
//!   的**代表真实模型键**（键名逐字取自 `w3_content::ENTITY_MODEL_FILES`
//!   的 14 键，不新增表项、不改 `w3-content`）。
//!
//! 结果经 `ModelLibrary::bind_seat` 注入表现层只读绑定，`Scene::consume` 的
//! `resolve_with_reason` 消费它 → `instance.model_key` 命中 `Scene` 注入的
//! 真实 `BakedModel` → `Scene::expand_model` 展开真实 glTF 网格（Q3 链路）。
//! 全程只读、确定性（同一 `WorldView` + 同一选族 → 同一键），MUST NOT 进
//! 权威状态 / FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 6）定义的快照
//! / `first_difference()`（ADR 0002）。
//!
//! ## 未覆盖的部分（如实登记，交 leader 判断是否需回到用户拍板）
//!
//! 运行时实体按段代表键呈现，**无法区分同段内的具体类别**：R-1 已批准
//! 「`ORC_SIEGE`(206) / `ORC_HERO`(210) 及人族六类本期不映射真实网格」，
//! 本模块在精确 `DefId` 查表 [`def_id_to_model_key`] 里显式把这些类别映射为
//! `None`（缺键显式可诊断回退），但运行时因拿不到精确 DefId，这些实体仍会
//! 落到该段代表键。要做到「精确类别级真实 / 缺键」需内核在只读视图或事件
//! 携带 DefId（或新增权威关联），那会触发规格解锁重走门，本 change 不做。
//! 装配期 frame-0 的 12 个实体已有精确 `EntityId → DefId` 关联，可按精确
//! DefId 查表，不受此限制。

use w3_core::DefId;
use w3_render::ModelLibrary;

use crate::faction::FactionSelection;
use crate::melee::Race;
use crate::melee::defs::{
    HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO, HUMAN_RANGED, HUMAN_SIEGE, HUMAN_SMITHY, ORC_HERO,
    ORC_SIEGE,
};

/// 人族 `1xx` 段、兽族 `2xx` 段、中立 `9xx` 段的起始编号（编译期整数常量）。
pub const SEGMENT_HUMAN: DefId = 100;
pub const SEGMENT_ORC: DefId = 200;
pub const SEGMENT_NEUTRAL: DefId = 900;

/// 一个 DefId 段的代表真实模型键（键名逐字取自
/// `w3_content::ENTITY_MODEL_FILES`，不新增表项、不改 `w3-content`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactionModelKeys {
    /// 该段可移动单位的代表真实模型键。
    pub unit: &'static str,
    /// 该段建筑 / 资源的代表真实模型键。
    pub building: &'static str,
}

/// 编译期整数 `DefId` 段 → 真实模型键定义表（Q3 / R2 / R4）。
///
/// 复用 `defs.rs::Race::category_def_ids` 的「按族只读查表」先例：
/// 族段由编译期整数常量索引，键名逐字使用 content-owner 定死的
/// `ENTITY_MODEL_FILES` 14 键。MUST NOT 用建筑名 / `H`/`O` 前缀白名单推断
/// （R1.2），MUST NOT 在 Present 层判玩法规则。
pub const SEGMENT_MODEL_KEYS: &[(DefId, FactionModelKeys)] = &[
    (
        SEGMENT_HUMAN,
        FactionModelKeys {
            unit: "soldier",
            building: "town-hall",
        },
    ),
    (
        SEGMENT_ORC,
        FactionModelKeys {
            unit: "orc-grunt",
            building: "orc-great-hall",
        },
    ),
    (
        SEGMENT_NEUTRAL,
        FactionModelKeys {
            unit: "tree",
            building: "gold-mine",
        },
    ),
];

/// `content-owner` 定死的 14 个动态实体键（逐字使用，不改 `w3-content`）。
/// 6 个人族 / 中立键 + 8 个兽族键，全部有真实 glTF 网格（本机资产就绪时）。
pub const ENTITY_MODEL_KEYS: [&str; 14] = [
    "peasant",
    "soldier",
    "town-hall",
    "barracks",
    "gold-mine",
    "tree",
    "orc-peon",
    "orc-grunt",
    "orc-headhunter",
    "orc-great-hall",
    "orc-barracks",
    "orc-burrow",
    "orc-altar",
    "orc-war-mill",
];

/// R-1 已批准「本期不映射真实网格」的八类精确 DefId。
/// 精确查表 [`def_id_to_model_key`] 对它们返回 `None`，走缺键显式可诊断
/// 回退；运行时实体因拿不到精确 DefId 仍落段代表键（见模块级「未覆盖」）。
pub const R1_UNMAPPED_DEFS: [DefId; 8] = [
    HUMAN_RANGED,
    HUMAN_SIEGE,
    HUMAN_FARM,
    HUMAN_ALTAR,
    HUMAN_SMITHY,
    HUMAN_HERO,
    ORC_SIEGE,
    ORC_HERO,
];

/// 精确 `DefId` 所属段编号（`100` / `200` / `900`）。纯整数运算、只读。
pub fn def_segment(def_id: DefId) -> DefId {
    if def_id >= SEGMENT_NEUTRAL {
        SEGMENT_NEUTRAL
    } else if def_id >= SEGMENT_ORC {
        SEGMENT_ORC
    } else if def_id >= SEGMENT_HUMAN {
        SEGMENT_HUMAN
    } else {
        // 既有 6 个 `UnitDef`（`1..=6`）是人族基线，归人族段。
        SEGMENT_HUMAN
    }
}

/// 按段编号查代表真实模型键（编译期定义表，无 `if race` 散落分支）。
fn segment_keys(segment: DefId) -> Option<&'static FactionModelKeys> {
    SEGMENT_MODEL_KEYS
        .iter()
        .find(|(id, _)| *id == segment)
        .map(|(_, keys)| keys)
}

/// 按本地族 / AI 族反查段模型键（`Race` 两变体只在定义表这一处集中映射）。
fn race_segment(race: Race) -> DefId {
    match race {
        Race::Human => SEGMENT_HUMAN,
        Race::Orc => SEGMENT_ORC,
    }
}

/// 精确 `DefId` → 真实模型键（装配期 frame-0 实体 / CPU 回归用）。
///
/// `is_building` 由装配期 `EntityView.speed == 0` 或 `MeleeSetup::movability`
/// 只读判据给出。R-1 八类返回 `None`（缺键显式可诊断回退）；未登记 DefId
/// 也返回 `None`，MUST NOT 静默产出错误族键。
pub fn def_id_to_model_key(def_id: DefId, is_building: bool) -> Option<&'static str> {
    if R1_UNMAPPED_DEFS.contains(&def_id) {
        return None;
    }
    let keys = segment_keys(def_segment(def_id))?;
    Some(if is_building {
        keys.building
    } else {
        keys.unit
    })
}

/// 席位（`owner`）→ 段代表真实模型键的只读装配计划（Q3）。
///
/// 由 `FactionSelection`（表现层本地只读状态）派生，装配期注入
/// `ModelLibrary`。同一对局内固定，回放 / 联机一致，确定性。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeModelPlan {
    pub local_race: Race,
    pub ai_race: Race,
}

impl RuntimeModelPlan {
    /// 由开局选族派生装配计划。
    pub fn from_faction(faction: FactionSelection) -> Self {
        Self {
            local_race: faction.local_race,
            ai_race: faction.ai_race,
        }
    }

    /// 某席位（`owner`）对应的段代表真实模型键。
    /// `1` 本地席位、`2` AI 席位、其它视为中立 `9xx` 段。
    pub fn seat_keys(self, owner: u8) -> Option<&'static FactionModelKeys> {
        let segment = match owner {
            crate::input::LOCAL_PLAYER => race_segment(self.local_race),
            crate::melee::AI_PLAYER => race_segment(self.ai_race),
            _ => SEGMENT_NEUTRAL,
        };
        segment_keys(segment)
    }

    /// 只读派生某运行时实体的真实模型键（确定性纯函数，供 CPU 回归）。
    /// `speed == 0` → 建筑键，`speed != 0` → 单位键。
    pub fn runtime_model_key(self, owner: u8, speed: i64) -> Option<&'static str> {
        let keys = self.seat_keys(owner)?;
        Some(if speed == 0 { keys.building } else { keys.unit })
    }

    /// 装配期把 14 个真实模型键占位 + 席位段绑定注入 `ModelLibrary`。
    ///
    /// 占位让 `resolve_with_reason` 的段键命中（不回落 `"default"`）；真实
    /// glTF 网格由 `Scene::set_entity_models` 单独注入。纯表现层只读绑定，
    /// MUST NOT 进权威状态 / 状态哈希 / 快照（ADR 0002）。
    pub fn bind_seats(self, library: &mut ModelLibrary) {
        for key in ENTITY_MODEL_KEYS {
            library.insert_placeholder(key);
        }
        for owner in [crate::input::LOCAL_PLAYER, crate::melee::AI_PLAYER] {
            if let Some(keys) = self.seat_keys(owner) {
                library.bind_seat(owner, keys.unit, keys.building);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_model_keys_are_two_factions_plus_neutral() {
        assert_eq!(SEGMENT_MODEL_KEYS.len(), 3);
        assert_eq!(def_segment(101), SEGMENT_HUMAN);
        assert_eq!(def_segment(210), SEGMENT_ORC);
        assert_eq!(def_segment(901), SEGMENT_NEUTRAL);
        assert_eq!(def_segment(1), SEGMENT_HUMAN, "既有 6 基线归人族段");
    }

    #[test]
    fn def_id_to_model_key_maps_segments_and_r1_unmapped() {
        assert_eq!(def_id_to_model_key(102, false), Some("soldier"));
        assert_eq!(def_id_to_model_key(103, true), Some("town-hall"));
        assert_eq!(def_id_to_model_key(202, false), Some("orc-grunt"));
        assert_eq!(def_id_to_model_key(203, true), Some("orc-great-hall"));
        for def_id in R1_UNMAPPED_DEFS {
            assert_eq!(def_id_to_model_key(def_id, false), None);
            assert_eq!(def_id_to_model_key(def_id, true), None);
        }
    }

    #[test]
    fn runtime_plan_is_deterministic_and_faction_segmented() {
        let orc_local =
            RuntimeModelPlan::from_faction(FactionSelection::with_races(Race::Orc, Race::Human));
        assert_eq!(orc_local.runtime_model_key(1, 3), Some("orc-grunt"));
        assert_eq!(orc_local.runtime_model_key(1, 0), Some("orc-great-hall"));
        assert_eq!(orc_local.runtime_model_key(2, 3), Some("soldier"));
        assert_eq!(orc_local.runtime_model_key(2, 0), Some("town-hall"));
        // 同一输入两次派生一致（回放 / 联机确定性）。
        assert_eq!(
            orc_local.runtime_model_key(1, 3),
            orc_local.runtime_model_key(1, 3)
        );
    }

    #[test]
    fn bind_seats_wires_all_keys_and_both_seats() {
        let plan = RuntimeModelPlan::from_faction(FactionSelection::default());
        let mut library = ModelLibrary::default();
        library.insert("default", w3_render::Model::default());
        plan.bind_seats(&mut library);
        assert_eq!(
            library.seat(1).map(|s| s.unit_key.as_str()),
            Some("soldier")
        );
        assert_eq!(
            library.seat(2).map(|s| s.unit_key.as_str()),
            Some("orc-grunt")
        );
        for key in ENTITY_MODEL_KEYS {
            assert!(
                library.get(key).is_some(),
                "reason=missing_placeholder key={key}"
            );
        }
    }
}
