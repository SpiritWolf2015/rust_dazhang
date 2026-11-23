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
//! - 两者组合查编译期整数 `DefId` 段定义表 [`SEGMENT_MODEL_KEYS`]（P9 扩为
//!   五段 `1xx` / `2xx` / `3xx` / `4xx` / `9xx`），产出该段的**代表真实模型键**
//!   （键名逐字取自 `w3_content::ENTITY_MODEL_FILES` 的 18 键：既有 14 个
//!   真实网格键 + P9 由 content-owner 落盘的 4 个合成段代表键，不改
//!   `w3-content`）。新段沿用「段代表键 + 显式可诊断回退」，不要求给每个
//!   新 `DefId` 加精确映射（沿用 R-1 已批准口径，避免扩范围）。
//!
//! 结果经 `ModelLibrary::bind_seat` 注入表现层只读绑定，`Scene::consume` 的
//! `resolve_with_reason` 消费它 → `instance.model_key` 命中 `Scene` 注入的
//! 真实 `BakedModel` → `Scene::expand_model` 展开真实 glTF 网格（Q3 链路）。
//! 全程只读、确定性（同一 `WorldView` + 同一选族 → 同一键），MUST NOT 进
//! 权威状态 / FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 8）定义的快照
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

/// 五段的起始编号（编译期整数常量）：人族 `1xx`、兽族 `2xx`、
/// 不死 `3xx`（P9）、暗夜 `4xx`（P9）、中立 `9xx`。
pub const SEGMENT_HUMAN: DefId = 100;
pub const SEGMENT_ORC: DefId = 200;
pub const SEGMENT_UNDEAD: DefId = 300;
pub const SEGMENT_NIGHT_ELF: DefId = 400;
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

/// 编译期整数 `DefId` 段 → 真实模型键定义表（Q3 / R2 / R4 / P9 Q1'）。
///
/// 复用 `defs.rs::Race::category_def_ids` 的「按族只读查表」先例：
/// 族段由编译期整数常量索引，键名逐字使用 content-owner 定死的
/// `ENTITY_MODEL_FILES` 键表。P9 新增不死 `3xx` / 暗夜 `4xx` 两段，
/// 沿用「段代表键 + 显式可诊断回退」：新段键是逻辑路径字符串合成键
/// （非资产内容），本机无对应真实 glTF 时由 `ModelLibrary` 以
/// `reason=asset_missing` 显式记录并回落，不静默产出错误族键。
/// MUST NOT 用建筑名 / `H`/`O`/`U`/`N` 前缀白名单推断机制（R1.2），
/// MUST NOT 在 Present 层判玩法规则、不反写权威状态 / 哈希 / 快照。
/// 新段不要求给每个 `3xx` / `4xx` DefId 加精确映射（沿用 R-1 已批准的
/// 不映射真实网格口径，避免扩范围）。
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
        SEGMENT_UNDEAD,
        FactionModelKeys {
            unit: "undead-unit",
            building: "undead-building",
        },
    ),
    (
        SEGMENT_NIGHT_ELF,
        FactionModelKeys {
            unit: "nightelf-unit",
            building: "nightelf-building",
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

/// `content-owner` 定死的动态实体键表（逐字使用，不改 `w3-content`）：
/// 既有 14 键（6 个人族 / 中立键 + 8 个兽族键，本机资产就绪时有真实
/// glTF 网格）+ P9 新增 4 个不死 / 暗夜段代表键（纯合成全小写逻辑路径
/// 占位串、非资产内容，沿用「段代表键 + 显式可诊断回退」）。
/// 与 `w3_content::ENTITY_MODEL_FILES` 的 18 键逐字一致（两个 owner
/// 并行改不同文件，必须一致）。
pub const ENTITY_MODEL_KEYS: [&str; 18] = [
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
    "undead-unit",
    "undead-building",
    "nightelf-unit",
    "nightelf-building",
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

/// 精确 `DefId` 所属段编号（`100` / `200` / `300` / `400` / `900`）。
/// 纯整数运算、只读；判定顺序固定为 `900 → 400 → 300 → 200 → 100`
/// （P9 五段，段号降序比较，避免 `3xx` / `4xx` 被误判成人族基线）。
pub fn def_segment(def_id: DefId) -> DefId {
    if def_id >= SEGMENT_NEUTRAL {
        SEGMENT_NEUTRAL
    } else if def_id >= SEGMENT_NIGHT_ELF {
        SEGMENT_NIGHT_ELF
    } else if def_id >= SEGMENT_UNDEAD {
        SEGMENT_UNDEAD
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

/// 按本地族 / AI 族反查段模型键（`Race` 四变体只在定义表这一处集中映射，
/// P9 / Q1' 扩到五段中的四族段；中立 `9xx` 由 owner 非本地 / AI 时兜底）。
fn race_segment(race: Race) -> DefId {
    match race {
        Race::Human => SEGMENT_HUMAN,
        Race::Orc => SEGMENT_ORC,
        Race::Undead => SEGMENT_UNDEAD,
        Race::NightElf => SEGMENT_NIGHT_ELF,
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

    /// 装配期把 18 个动态实体键占位（14 个真实网格键 + 4 个 P9 段代表键）
    /// + 席位段绑定注入 `ModelLibrary`。
    ///
    /// 占位让 `resolve_with_reason` 的段键命中（不回落 `"default"`）；真实
    /// glTF 网格由 `Scene::set_entity_models` 单独注入。P9 新段代表键是
    /// 合成占位串，本机无对应真实网格时由 `Scene` 以 `reason=asset_missing`
    /// 显式记录并回落，不静默降级。纯表现层只读绑定，MUST NOT 进权威
    /// 状态 / 状态哈希 / 快照（ADR 0002）。
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

    /// P9 Q1'：五段映射（人 100 / 兽 200 / 不死 300 / 暗夜 400 / 中立 900），
    /// `def_segment` 判定顺序 900→400→300→200→100 全部覆盖。
    #[test]
    fn segment_model_keys_are_four_factions_plus_neutral() {
        assert_eq!(SEGMENT_MODEL_KEYS.len(), 5);
        assert_eq!(ENTITY_MODEL_KEYS.len(), 18);
        assert_eq!(def_segment(101), SEGMENT_HUMAN);
        assert_eq!(def_segment(210), SEGMENT_ORC);
        assert_eq!(def_segment(301), SEGMENT_UNDEAD);
        assert_eq!(def_segment(399), SEGMENT_UNDEAD);
        assert_eq!(def_segment(401), SEGMENT_NIGHT_ELF);
        assert_eq!(def_segment(499), SEGMENT_NIGHT_ELF);
        assert_eq!(def_segment(901), SEGMENT_NEUTRAL);
        assert_eq!(def_segment(1), SEGMENT_HUMAN, "既有 6 基线归人族段");
        // 五段各自段代表键都在 18 键表内（段代表键 + 显式可诊断回退）。
        for (_, keys) in SEGMENT_MODEL_KEYS {
            assert!(ENTITY_MODEL_KEYS.contains(&keys.unit));
            assert!(ENTITY_MODEL_KEYS.contains(&keys.building));
        }
        // 4 个段代表键逐字（与 content-owner 落盘键名一致）。
        for key in [
            "undead-unit",
            "undead-building",
            "nightelf-unit",
            "nightelf-building",
        ] {
            assert!(ENTITY_MODEL_KEYS.contains(&key));
        }
    }

    #[test]
    fn def_id_to_model_key_maps_segments_and_r1_unmapped() {
        assert_eq!(def_id_to_model_key(102, false), Some("soldier"));
        assert_eq!(def_id_to_model_key(103, true), Some("town-hall"));
        assert_eq!(def_id_to_model_key(202, false), Some("orc-grunt"));
        assert_eq!(def_id_to_model_key(203, true), Some("orc-great-hall"));
        // P9 不死 / 暗夜段精确 DefId 按段代表键命中（Q1'），
        // 不用每个新 DefId 加精确映射（沿用 R-1 口径）。
        assert_eq!(def_id_to_model_key(301, false), Some("undead-unit"));
        assert_eq!(def_id_to_model_key(303, true), Some("undead-building"));
        assert_eq!(def_id_to_model_key(401, false), Some("nightelf-unit"));
        assert_eq!(def_id_to_model_key(403, true), Some("nightelf-building"));
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
        // P9 Q1'：不死 / 暗夜席位按段代表键解析（4 族五段）。
        let undead_local = RuntimeModelPlan::from_faction(FactionSelection::with_races(
            Race::Undead,
            Race::NightElf,
        ));
        assert_eq!(undead_local.runtime_model_key(1, 3), Some("undead-unit"));
        assert_eq!(
            undead_local.runtime_model_key(1, 0),
            Some("undead-building")
        );
        assert_eq!(undead_local.runtime_model_key(2, 3), Some("nightelf-unit"));
        assert_eq!(
            undead_local.runtime_model_key(2, 0),
            Some("nightelf-building")
        );
        // 中立 owner 恒 `9xx` 段代表键。
        let neutral = RuntimeModelPlan::from_faction(FactionSelection::default());
        assert_eq!(neutral.runtime_model_key(3, 3), Some("tree"));
        assert_eq!(neutral.runtime_model_key(3, 0), Some("gold-mine"));
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
