//! 合成 / 原创冻结玩法定义（design D2）。
//!
//! 全部为编译期整数常量，不读取 `config.toml`、导出 JSON 或任何游戏资产
//! （ADR 0001）。定义经 `FrozenDefinitions::new` 冻结后，内容哈希进入快照
//! 头部；对局中无热更新入口。速度与射程经 `SimFixed::from_int` 冻结，
//! 不存在 `f64 → SimFixed` 直转路径。
//!
//! P8 地基：`units` 类别的 `DefId` 按种族分段（人族 `1xx`、兽族 `2xx`、
//! 中立 / 资源 `9xx`）。分段**只约束 `units` 类别**；`abilities` /
//! `upgrades` / `items` 三个类别沿用类别内唯一 ID（`10` / `20` / `30`），
//! 不纳入分段。既有 6 个 `UnitDef` 的 `DefId`（`1..=6`）保持原值，确保
//! `DEFAULT_UNIT_DEF` / `SOLDIER` / `BARRACKS` 等既有入口行为不变。
//! 分段差异只经既有 `defs_hash` 字段天然覆盖，不新增权威状态字段、
//! 不改 `SNAPSHOT_VERSION`（当前 3）与快照头部布局（ADR 0002）。
//!
//! P8.1 人族完整化：在人族段 `1xx` 追加远程 / 反重甲 / 人口 / 祭坛 /
//! 研发 / 英雄六类 `UnitDef`，以及类别内唯一 ID 的英雄主动技能与攻防升级。
//! 全部是冻结定义**内容变化**，经既有 `compute_defs_hash` 进 `defs_hash` /
//! `content_hash` 快照头部；不新增权威状态字段、不改状态哈希字段集合、
//! 不新增 `EffectKind` / 命令变体 / JASS VM（ADR 0002 / design D1–D7）。

use std::collections::BTreeMap;

use w3_core::{
    AbilityDef, DefId, EffectDef, EffectKind, FrozenDefinitions, ItemDef, SimFixed, UnitDef,
    UpgradeDef,
};

// 新类别槽位编号（表现层装配查表键，**不是** `EntityId`，MUST NOT 进入
// 权威状态 / 状态哈希 / 快照）。取值刻意避开 frame-0 的实体 ID（1..=12），
// 使 `category_def_ids` 与 `def_ids` 合并时也不会把不存在的实体混进
// 可移动性派生（design D4 / P8.0 B1）。
/// 槽位基准：新类别装配绑定专用命名空间。
pub const CATEGORY_SLOT_BASE: u64 = 1_000_000;
/// 槽位：远程单位。
pub const CATEGORY_RANGED: u64 = CATEGORY_SLOT_BASE + 1;
/// 槽位：反重甲第三战斗单位。
pub const CATEGORY_SIEGE: u64 = CATEGORY_SLOT_BASE + 2;
/// 槽位：人口建筑。
pub const CATEGORY_FARM: u64 = CATEGORY_SLOT_BASE + 3;
/// 槽位：英雄祭坛。
pub const CATEGORY_ALTAR: u64 = CATEGORY_SLOT_BASE + 4;
/// 槽位：研发 / 科技建筑。
pub const CATEGORY_SMITHY: u64 = CATEGORY_SLOT_BASE + 5;
/// 槽位：英雄。
pub const CATEGORY_HERO: u64 = CATEGORY_SLOT_BASE + 6;

/// 单位定义编号。`1` 与 `w3_client::input::DEFAULT_UNIT_DEF` 对齐，
/// 故 `T` 默认训练战斗单位；既有 `playable.rs` 入口注入的 `unit_def=1`
/// 也仍然有效（不破坏兼容）。这 6 个是既有基线定义，不参与种族分段。
pub const SOLDIER: DefId = 1;
pub const PEASANT: DefId = 2;
pub const TOWN_HALL: DefId = 3;
pub const BARRACKS: DefId = 4;
pub const GOLD_MINE: DefId = 5;
pub const TREE: DefId = 6;
/// 技能定义编号（与 `w3_client::input::DEFAULT_ABILITY` 对齐）。
pub const ABILITY: DefId = 10;
/// 英雄主动技能定义编号（类别内唯一 ID，不与 `ABILITY`（10）冲突）。
pub const HERO_ABILITY: DefId = 11;
/// 升级定义编号（地基既有攻防合一升级）。
pub const UPGRADE: DefId = 20;
/// 攻击升级定义编号（类别内唯一 ID，不与 `UPGRADE`（20）冲突）。
pub const UPGRADE_WEAPONS: DefId = 21;
/// 护甲升级定义编号（类别内唯一 ID，不与 `UPGRADE`（20）冲突）。
pub const UPGRADE_ARMOR: DefId = 22;
/// 物品定义编号。
pub const ITEM: DefId = 30;

// ---------------------------------------------------------------------------
// 种族分段 DefId（`units` 类别专用，全为编译期整数常量）
//
// 段位分配一旦定下，P8.1 / P8.2 新增单位 MUST 落在所属段内，避免跨段冲突：
//   - `100..=199`：人族（Human）单位 / 建筑；
//   - `200..=299`：兽族（Orc）单位 / 建筑；
//   - `900..=999`：中立 / 资源（金矿、树木），两族共用。
// `ABILITIES` / `UPGRADES` / `ITEMS` 三个类别仍是类别内唯一 ID，不占段位。
// ---------------------------------------------------------------------------

/// 人族段：农民。
pub const HUMAN_PEASANT: DefId = 101;
/// 人族段：士兵。
pub const HUMAN_SOLDIER: DefId = 102;
/// 人族段：主城。
pub const HUMAN_TOWN_HALL: DefId = 103;
/// 人族段：兵营。
pub const HUMAN_BARRACKS: DefId = 104;
// P8.1 人族新增六类（design D1）：全部为合成 / 原创编译期整数常量，
// 命名不照搬暴雪 rawcode 或只读参考项目的原创单位名 / 数值（ADR 0001 / C6）。
// 具体数值 `[待核]`：以本机导出 SLK 核对、不入库、不冒充原版。
/// 人族段：远程单位（穿刺 / 长射程，R5.1 / R5.3 框架），`speed_raw > 0`。
pub const HUMAN_RANGED: DefId = 105;
/// 人族段：反重甲第三战斗单位（R5.3），以较高单发伤害表达克制，`speed_raw > 0`。
pub const HUMAN_SIEGE: DefId = 106;
/// 人族段：人口建筑（R3.4），`speed_raw == 0`。
pub const HUMAN_FARM: DefId = 107;
/// 人族段：英雄祭坛（R4.2），`speed_raw == 0`。
pub const HUMAN_ALTAR: DefId = 108;
/// 人族段：研发 / 科技建筑（铁匠铺类，R4.4），`speed_raw == 0`。
pub const HUMAN_SMITHY: DefId = 109;
/// 人族段：英雄（R6），`mana > 0` 且 `speed_raw > 0`。
pub const HUMAN_HERO: DefId = 110;
/// 兽族段：农民。
pub const ORC_PEASANT: DefId = 201;
/// 兽族段：士兵。
pub const ORC_SOLDIER: DefId = 202;
/// 兽族段：主城。
pub const ORC_TOWN_HALL: DefId = 203;
/// 兽族段：兵营。
pub const ORC_BARRACKS: DefId = 204;
/// 中立 / 资源段：金矿（两族共用）。
pub const NEUTRAL_GOLD_MINE: DefId = 901;
/// 中立 / 资源段：树木（两族共用）。
pub const NEUTRAL_TREE: DefId = 902;

/// 开局选择的种族（定义层参数，纯表现层 / 装配层数据）。
///
/// 该枚举只用于选择开局 `Spawn` 关联的 `DefId` 段，MUST NOT 写入
/// `w3-core` 的 `Player` / `RtsMatch`（不新增 `race` 权威状态字段，
/// ADR 0002 / 主 `melee-match/spec.md:127-129`）。未指定时回落人族段。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Race {
    /// 人族段（`1xx`）；默认回落值。
    #[default]
    Human,
    /// 兽族段（`2xx`）。
    Orc,
}

impl Race {
    /// 对战对手的种族（一局内两族对抗共用一个 `RtsMatch`、
    /// 一份 `FrozenDefinitions`；此处只用于选择开局装配的段位）。
    pub fn opponent(self) -> Self {
        match self {
            Self::Human => Self::Orc,
            Self::Orc => Self::Human,
        }
    }

    /// 该种族农民定义编号。
    pub fn peasant(self) -> DefId {
        match self {
            Self::Human => HUMAN_PEASANT,
            Self::Orc => ORC_PEASANT,
        }
    }

    /// 该种族士兵定义编号。
    pub fn soldier(self) -> DefId {
        match self {
            Self::Human => HUMAN_SOLDIER,
            Self::Orc => ORC_SOLDIER,
        }
    }

    /// 该种族主城定义编号。
    pub fn town_hall(self) -> DefId {
        match self {
            Self::Human => HUMAN_TOWN_HALL,
            Self::Orc => ORC_TOWN_HALL,
        }
    }

    /// 该种族兵营定义编号。
    pub fn barracks(self) -> DefId {
        match self {
            Self::Human => HUMAN_BARRACKS,
            Self::Orc => ORC_BARRACKS,
        }
    }

    /// 中立金矿定义编号（两族共用 `9xx` 段）。
    pub fn gold_mine(self) -> DefId {
        let _ = self;
        NEUTRAL_GOLD_MINE
    }

    /// 中立树木定义编号（两族共用 `9xx` 段）。
    pub fn tree(self) -> DefId {
        let _ = self;
        NEUTRAL_TREE
    }

    // P8.1 新类别分派（design D3）：仍是**定义层 / 装配层**的参数查表，
    // MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`（不新增 `race` 权威状态）。
    // 兽族本 change 不补新类别，按 design D3「沿用既有 2xx、返回既有对应项
    // 或占位」回落到既有 `ORC_SOLDIER` / `ORC_BARRACKS`；P8.2 完整化时替换，
    // 返回值仍恒落在 `200..300` 段、不破坏分段测试。

    /// 该种族远程单位定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn ranged(self) -> DefId {
        match self {
            Self::Human => HUMAN_RANGED,
            Self::Orc => ORC_SOLDIER,
        }
    }

    /// 该种族反重甲第三战斗单位定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn siege(self) -> DefId {
        match self {
            Self::Human => HUMAN_SIEGE,
            Self::Orc => ORC_SOLDIER,
        }
    }

    /// 该种族人口建筑定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn farm(self) -> DefId {
        match self {
            Self::Human => HUMAN_FARM,
            Self::Orc => ORC_BARRACKS,
        }
    }

    /// 该种族英雄祭坛定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn altar(self) -> DefId {
        match self {
            Self::Human => HUMAN_ALTAR,
            Self::Orc => ORC_BARRACKS,
        }
    }

    /// 该种族研发 / 科技建筑定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn smithy(self) -> DefId {
        match self {
            Self::Human => HUMAN_SMITHY,
            Self::Orc => ORC_BARRACKS,
        }
    }

    /// 该种族英雄定义编号（人族 `1xx` 新常量 / 兽族 `2xx` 占位）。
    pub fn hero(self) -> DefId {
        match self {
            Self::Human => HUMAN_HERO,
            Self::Orc => ORC_SOLDIER,
        }
    }

    /// 装配层新类别绑定（design D3 / D4）：把六类人族 / 兽族分派结果
    /// 聚合成 `槽位 → DefId` 只读表，供 `MeleeSetup::with_race` 与
    /// `AiScript` 查表。键是 [`CATEGORY_RANGED`] 一类槽位常量，不是
    /// `EntityId`；全部为编译期整数常量，MUST NOT 写入 `w3-core` 的
    /// `Player` / `RtsMatch`。技能 / 升级是类别内唯一 ID、不占种族段，
    /// 由调用方直接用 [`HERO_ABILITY`] 一类常量，不进本分派表。
    pub fn category_def_ids(self) -> BTreeMap<u64, DefId> {
        [
            (CATEGORY_RANGED, self.ranged()),
            (CATEGORY_SIEGE, self.siege()),
            (CATEGORY_FARM, self.farm()),
            (CATEGORY_ALTAR, self.altar()),
            (CATEGORY_SMITHY, self.smithy()),
            (CATEGORY_HERO, self.hero()),
        ]
        .into_iter()
        .collect()
    }
}

/// 由整数字面量构造 Q47.16 定点值；显式处理错误，不静默吞错。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("melee defs: reason=fixed_conversion_failed")
}

/// 构造单位定义；速度与射程走冻结整数，不引入浮点直转。
fn unit(hp: i64, speed: i64, damage: i64, range: i64, supply: i64, train_frames: u32) -> UnitDef {
    UnitDef {
        hp,
        mana: 0,
        speed_raw: fixed(speed).raw(),
        damage,
        range_raw: fixed(range).raw(),
        supply,
        train_frames,
    }
}

/// 构造英雄单位定义；与 [`unit`] 同口径，但显式写入 `mana > 0`（design D1）。
///
/// 既有 `unit()` 固定 `mana: 0`，英雄是唯一带法力的类别（R6）；不改
/// `w3-core::UnitDef` 结构、不新增权威状态字段，只填既有冻结字段。
fn unit_with_mana(
    hp: i64,
    speed: i64,
    damage: i64,
    range: i64,
    supply: i64,
    train_frames: u32,
    mana: i64,
) -> UnitDef {
    UnitDef {
        mana,
        ..unit(hp, speed, damage, range, supply, train_frames)
    }
}

/// 合成冻结定义：既有 6 个基线单位 + 人族 / 兽族 / 中立资源分段单位 +
/// P8.1 人族六类新单位 + 技能 / 升级 / 物品。
///
/// 数值全为编译期整数常量、`[待核]`（以本机导出 SLK 核对、不入库）；主城
/// 生命刻意压低，使一局能在合理帧数内打完。分段单位与基线单位同数值
/// （P8.0 只做地基，不交付可玩的两族差异），但 `speed_raw` 由单位性质决定：
/// 农民 / 士兵 / 远程 / 反重甲 / 英雄 `> 0`，建筑 / 资源点 `== 0`，
/// 供 [`crate::input::derive_movability`] 作为可移动性判据（design D1 / R1.4）。
/// 新增定义只改变 `FrozenDefinitions` 的内容哈希（`defs_hash` / `content_hash`），
/// 不改 `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合或命令排序键（ADR 0002）。
pub fn synthetic_definitions() -> FrozenDefinitions {
    let units = vec![
        // 既有基线定义（1..=6），既有入口行为不变。
        (PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        (GOLD_MINE, unit(500, 0, 0, 0, 0, 0)),
        (TREE, unit(200, 0, 0, 0, 0, 0)),
        // 人族段 1xx（P8.0 地基四件）。
        (HUMAN_PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (HUMAN_SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (HUMAN_TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (HUMAN_BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        // P8.1 人族六类新定义（design D1）：全部合成 / 原创编译期整数常量，
        // 数值 `[待核]`、不冒充原版；远程射程大于近战、英雄 `mana > 0`、
        // 三类新增建筑 `speed == 0` 使 `speed_raw == 0`（R1.4 / R13.6）。
        (HUMAN_RANGED, unit(90, 3, 18, 420, 2, 24)),
        (HUMAN_SIEGE, unit(180, 2, 48, 260, 3, 40)),
        (HUMAN_FARM, unit(200, 0, 0, 0, 0, 20)),
        (HUMAN_ALTAR, unit(300, 0, 0, 0, 0, 50)),
        (HUMAN_SMITHY, unit(300, 0, 0, 0, 0, 35)),
        (HUMAN_HERO, unit_with_mana(400, 4, 42, 200, 5, 60, 100)),
        // 兽族段 2xx。
        (ORC_PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (ORC_SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (ORC_TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (ORC_BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        // 中立 / 资源段 9xx，两族共用。
        (NEUTRAL_GOLD_MINE, unit(500, 0, 0, 0, 0, 0)),
        (NEUTRAL_TREE, unit(200, 0, 0, 0, 0, 0)),
    ];
    let abilities = vec![
        (
            ABILITY,
            AbilityDef {
                mana_cost: 0,
                cooldown_frames: 5,
                cast_frames: 3,
                behavior: 0,
                effects: vec![EffectDef {
                    kind: EffectKind::Damage,
                    amount: 20,
                    duration_frames: 0,
                }],
            },
        ),
        // P8.1 英雄主动技能（design D2）：只用既有 `EffectKind` +
        // `EffectDef` 组合表达，走既有 `Cast` 命令面；MUST NOT 新增
        // `EffectKind` 变体 / 命令变体 / JASS VM（N5）。伤害 + 眩晕 +
        // 自身增益三段组合，数值 `[待核]`。
        (
            HERO_ABILITY,
            AbilityDef {
                mana_cost: 20,
                cooldown_frames: 30,
                cast_frames: 3,
                behavior: 0,
                effects: vec![
                    EffectDef {
                        kind: EffectKind::Damage,
                        amount: 60,
                        duration_frames: 0,
                    },
                    EffectDef {
                        kind: EffectKind::Stun,
                        amount: 0,
                        duration_frames: 5,
                    },
                    EffectDef {
                        kind: EffectKind::BuffStat,
                        amount: 4,
                        duration_frames: 10,
                    },
                ],
            },
        ),
    ];
    let items = vec![(
        ITEM,
        ItemDef {
            hp_mod: 10,
            damage_mod: 2,
        },
    )];
    let upgrades = vec![
        (
            UPGRADE,
            UpgradeDef {
                damage_mod: 4,
                hp_mod: 10,
                research_frames: 30,
            },
        ),
        // P8.1 攻 / 防两项升级（design D2 / R4.4）：经既有 `Research`
        // 路径接通，`research_frames` 到期后由内核 `apply_upgrade` 把
        // `damage_mod` / `hp_mod` 结算到我方存活单位。数值 `[待核]`。
        (
            UPGRADE_WEAPONS,
            UpgradeDef {
                damage_mod: 8,
                hp_mod: 0,
                research_frames: 24,
            },
        ),
        (
            UPGRADE_ARMOR,
            UpgradeDef {
                damage_mod: 0,
                hp_mod: 20,
                research_frames: 24,
            },
        ),
    ];
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("melee defs: reason=invalid_synthetic_definitions")
}

/// 本机导出数据的可选来源（ADR 0001 / design D2）。
///
/// 只保留 provider 边界：本期不读取、不入库；缺失或未提供时回落到合成定义，
/// 零文件 IO。后续独立 change 可在此处接入 `w3-content::load_map` 与 SLK 表，
/// 但导出 JSON 的 `f64` 坐标 MUST 经 ADR 0004 冻结边界、MUST NOT 进哈希或快照。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportSource {
    /// gitignored 本机导出地图目录（可选，本期不读取）。
    pub map_dir: Option<String>,
    /// SLK 表路径（可选，本期不读取）。
    pub table_path: Option<String>,
}

impl ExportSource {
    /// 两个路径都提供时才视为可用；本期即使可用也只作为接缝存在。
    pub fn is_available(&self) -> bool {
        self.map_dir.is_some() && self.table_path.is_some()
    }

    /// 解析冻结定义。本期无论是否提供导出路径都回落到合成定义，
    /// 不做任何文件 IO、不读取 `config.toml`、不向仓库写入资产。
    pub fn definitions(&self) -> FrozenDefinitions {
        synthetic_definitions()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::input::derive_movability;

    /// `units` 类别定义编号全集（既有 6 个基线 + 分段 + P8.1 人族六类）。
    fn all_unit_ids() -> Vec<DefId> {
        vec![
            PEASANT,
            SOLDIER,
            TOWN_HALL,
            BARRACKS,
            GOLD_MINE,
            TREE,
            HUMAN_PEASANT,
            HUMAN_SOLDIER,
            HUMAN_TOWN_HALL,
            HUMAN_BARRACKS,
            HUMAN_RANGED,
            HUMAN_SIEGE,
            HUMAN_FARM,
            HUMAN_ALTAR,
            HUMAN_SMITHY,
            HUMAN_HERO,
            ORC_PEASANT,
            ORC_SOLDIER,
            ORC_TOWN_HALL,
            ORC_BARRACKS,
            NEUTRAL_GOLD_MINE,
            NEUTRAL_TREE,
        ]
    }

    /// P8.1 人族新类别 `DefId`（供分段 / 唯一性 / 内容敏感性回归复用）。
    const HUMAN_NEW_UNIT_IDS: [DefId; 6] = [
        HUMAN_RANGED,
        HUMAN_SIEGE,
        HUMAN_FARM,
        HUMAN_ALTAR,
        HUMAN_SMITHY,
        HUMAN_HERO,
    ];

    /// P8.1 新增建筑（MUST `speed_raw == 0`，R1.4 / R13.6）。
    const HUMAN_NEW_BUILDING_IDS: [DefId; 3] = [HUMAN_FARM, HUMAN_ALTAR, HUMAN_SMITHY];

    #[test]
    fn definitions_cover_all_kinds_and_are_stable() {
        let defs = synthetic_definitions();
        assert_eq!(defs.units().len(), 22, "reason=definition_count");
        assert!(defs.units().contains_key(&PEASANT));
        assert!(defs.units().contains_key(&SOLDIER));
        assert!(defs.units().contains_key(&TOWN_HALL));
        assert!(defs.units().contains_key(&BARRACKS));
        assert!(defs.units().contains_key(&GOLD_MINE));
        assert!(defs.units().contains_key(&TREE));
        for id in HUMAN_NEW_UNIT_IDS {
            assert!(
                defs.units().contains_key(&id),
                "reason=missing_human_new_unit id={id}"
            );
        }
        assert!(defs.abilities().contains_key(&ABILITY));
        assert!(defs.abilities().contains_key(&HERO_ABILITY));
        assert!(defs.items().contains_key(&ITEM));
        assert!(defs.upgrades().contains_key(&UPGRADE));
        assert!(defs.upgrades().contains_key(&UPGRADE_WEAPONS));
        assert!(defs.upgrades().contains_key(&UPGRADE_ARMOR));
        assert_eq!(defs.abilities().len(), 2);
        assert_eq!(defs.upgrades().len(), 3);
        assert_eq!(defs, synthetic_definitions());
    }

    #[test]
    fn units_def_ids_are_globally_unique_and_segmented() {
        let ids = all_unit_ids();
        let unique: BTreeSet<DefId> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "units 类别 DefId 必须全局唯一");

        // 分段只约束 units 类别：三个非 units 类别沿用类别内唯一 ID；
        // P8.1 新增技能 / 升级取类别内唯一 ID，不占 `1xx` / `2xx` 段位。
        assert_eq!(ABILITY, 10);
        assert_eq!(HERO_ABILITY, 11);
        assert_eq!(UPGRADE, 20);
        assert_eq!(UPGRADE_WEAPONS, 21);
        assert_eq!(UPGRADE_ARMOR, 22);
        assert_eq!(ITEM, 30);
        let category_ids = [
            ABILITY,
            HERO_ABILITY,
            UPGRADE,
            UPGRADE_WEAPONS,
            UPGRADE_ARMOR,
            ITEM,
        ];
        let category_unique: BTreeSet<DefId> = category_ids.into_iter().collect();
        assert_eq!(
            category_unique.len(),
            category_ids.len(),
            "reason=category_internal_ids_must_be_unique"
        );

        // 既有 6 个基线定义保持原值，分段常量不得与之冲突。
        let baseline: BTreeSet<DefId> = [1, 2, 3, 4, 5, 6].into_iter().collect();
        let segmented = [
            HUMAN_PEASANT,
            HUMAN_SOLDIER,
            HUMAN_TOWN_HALL,
            HUMAN_BARRACKS,
            HUMAN_RANGED,
            HUMAN_SIEGE,
            HUMAN_FARM,
            HUMAN_ALTAR,
            HUMAN_SMITHY,
            HUMAN_HERO,
            ORC_PEASANT,
            ORC_SOLDIER,
            ORC_TOWN_HALL,
            ORC_BARRACKS,
            NEUTRAL_GOLD_MINE,
            NEUTRAL_TREE,
        ];
        assert!(
            segmented.iter().all(|id| !baseline.contains(id)),
            "分段 DefId 不得与既有 6 个 UnitDef 冲突"
        );

        // 段位分配：人族 1xx（含 P8.1 六类）/ 兽族 2xx / 中立资源 9xx。
        for id in [
            HUMAN_PEASANT,
            HUMAN_SOLDIER,
            HUMAN_TOWN_HALL,
            HUMAN_BARRACKS,
            HUMAN_RANGED,
            HUMAN_SIEGE,
            HUMAN_FARM,
            HUMAN_ALTAR,
            HUMAN_SMITHY,
            HUMAN_HERO,
        ] {
            assert!((100..200).contains(&id), "人族段应落在 1xx: {id}");
        }
        for id in [ORC_PEASANT, ORC_SOLDIER, ORC_TOWN_HALL, ORC_BARRACKS] {
            assert!((200..300).contains(&id), "兽族段应落在 2xx: {id}");
        }
        for id in [NEUTRAL_GOLD_MINE, NEUTRAL_TREE] {
            assert!((900..1000).contains(&id), "中立资源段应落在 9xx: {id}");
        }
    }

    #[test]
    fn defs_hash_is_fixed_and_content_sensitive() {
        let a = synthetic_definitions();
        let b = synthetic_definitions();
        assert_eq!(
            a.content_hash(),
            b.content_hash(),
            "构造可重复、defs_hash 固定"
        );
        assert_eq!(a, b);

        // 少一个分段单位即改变内容哈希（天然覆盖分段定义差异，不新增字段）。
        let mut units: Vec<(DefId, UnitDef)> = a
            .units()
            .iter()
            .map(|(&id, &def)| (id, def))
            .filter(|(id, _)| *id != ORC_PEASANT)
            .collect();
        units.sort_by_key(|(id, _)| *id);
        let smaller = FrozenDefinitions::new(
            units,
            a.abilities()
                .iter()
                .map(|(&id, d)| (id, d.clone()))
                .collect(),
            a.items().iter().map(|(&id, &d)| (id, d)).collect(),
            a.upgrades().iter().map(|(&id, &d)| (id, d)).collect(),
        )
        .unwrap();
        assert_ne!(a.content_hash(), smaller.content_hash());
    }

    /// 反例辅助：从完整定义集删除指定条目，模拟「漏接某类新定义」的缺陷态。
    /// 用于 `defs_hash` 内容敏感性反向验证（CODEBUDDY.md §5 / tasks 1.5）。
    fn rebuild_definitions(
        drop_unit: Option<DefId>,
        drop_ability: Option<DefId>,
        drop_upgrade: Option<DefId>,
    ) -> FrozenDefinitions {
        let full = synthetic_definitions();
        let units = full
            .units()
            .iter()
            .filter(|&(&id, _)| Some(id) != drop_unit)
            .map(|(&id, &d)| (id, d))
            .collect();
        let abilities = full
            .abilities()
            .iter()
            .filter(|&(&id, _)| Some(id) != drop_ability)
            .map(|(&id, d)| (id, d.clone()))
            .collect();
        let items = full.items().iter().map(|(&id, &d)| (id, d)).collect();
        let upgrades = full
            .upgrades()
            .iter()
            .filter(|&(&id, _)| Some(id) != drop_upgrade)
            .map(|(&id, &d)| (id, d))
            .collect();
        FrozenDefinitions::new(units, abilities, items, upgrades)
            .expect("melee defs: reason=rebuild_definitions_failed")
    }

    /// 退化哈希（缺陷态）：只散列 P8.0 基线 16 个 `UnitDef`，忽略全部
    /// P8.1 新增单位 / 技能 / 升级，模拟 `compute_defs_hash` 漏接新定义的
    /// 退化实现。仅用于反向验证，生产实现 MUST 用 `content_hash()`。
    fn degenerate_hash_ignoring_new(defs: &FrozenDefinitions) -> u64 {
        const BASELINE: [DefId; 16] = [
            1, 2, 3, 4, 5, 6, 101, 102, 103, 104, 201, 202, 203, 204, 901, 902,
        ];
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for id in BASELINE {
            if let Some(def) = defs.units().get(&id) {
                hash = (hash ^ id as u64).wrapping_mul(0x0000_0100_0000_01b3);
                hash = (hash ^ def.hp as u64).wrapping_mul(0x0000_0100_0000_01b3);
                hash = (hash ^ def.speed_raw as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }

    /// 正向回归：正式 `content_hash()` 对漏接的每一类新定义都敏感，
    /// 证明新增定义经既有 `compute_defs_hash` 进 `defs_hash` / `content_hash`
    /// 快照头部，而不是新增权威状态字段（ADR 0002 / C3）。
    #[test]
    fn defs_hash_detects_each_new_definition() {
        let full = synthetic_definitions();
        for id in HUMAN_NEW_UNIT_IDS {
            let variant = rebuild_definitions(Some(id), None, None);
            assert_ne!(
                full.content_hash(),
                variant.content_hash(),
                "reason=content_hash_must_cover_new_unit id={id}"
            );
        }
        let no_ability = rebuild_definitions(None, Some(HERO_ABILITY), None);
        assert_ne!(
            full.content_hash(),
            no_ability.content_hash(),
            "reason=content_hash_must_cover_new_ability"
        );
        for id in [UPGRADE_WEAPONS, UPGRADE_ARMOR] {
            let variant = rebuild_definitions(None, None, Some(id));
            assert_ne!(
                full.content_hash(),
                variant.content_hash(),
                "reason=content_hash_must_cover_new_upgrade id={id}"
            );
        }
    }

    /// 反向验证·修复态：正式实现（`content_hash()`）通过「少一个定义即改哈希」
    /// 断言。与下一条「缺陷态」构成 `CODEBUDDY.md` §5 的反例元断言证据对。
    #[test]
    fn reverse_validation_defs_hash_formal_passes_the_assertion() {
        let full = synthetic_definitions();
        for id in HUMAN_NEW_UNIT_IDS {
            let variant = rebuild_definitions(Some(id), None, None);
            assert!(
                full.content_hash() != variant.content_hash(),
                "reason=formal_hash_must_detect_missing_unit id={id}"
            );
        }
        let no_ability = rebuild_definitions(None, Some(HERO_ABILITY), None);
        assert!(full.content_hash() != no_ability.content_hash());
        let no_upgrade = rebuild_definitions(None, None, Some(UPGRADE_WEAPONS));
        assert!(full.content_hash() != no_upgrade.content_hash());
    }

    /// 反向验证·缺陷态：退化哈希（只覆盖基线 16 个定义）对漏接的任一
    /// 新定义**不敏感**，故「少一个定义即改哈希」断言在退化实现下必然失败；
    /// 同时证明正式 `content_hash()` 能抓出同一缺陷（元断言防退化）。
    #[test]
    fn reverse_validation_defs_hash_degenerate_fails_the_same_assertion() {
        let full = synthetic_definitions();
        for id in HUMAN_NEW_UNIT_IDS {
            let variant = rebuild_definitions(Some(id), None, None);
            assert_eq!(
                degenerate_hash_ignoring_new(&full),
                degenerate_hash_ignoring_new(&variant),
                "reason=degenerate_hash_must_not_detect_missing_unit id={id}"
            );
            assert!(
                full.content_hash() != variant.content_hash(),
                "reason=formal_hash_must_catch_the_defect id={id}"
            );
        }
        let no_ability = rebuild_definitions(None, Some(HERO_ABILITY), None);
        assert_eq!(
            degenerate_hash_ignoring_new(&full),
            degenerate_hash_ignoring_new(&no_ability),
            "reason=degenerate_hash_must_not_detect_missing_ability"
        );
        assert!(full.content_hash() != no_ability.content_hash());
        let no_upgrade = rebuild_definitions(None, None, Some(UPGRADE_WEAPONS));
        assert_eq!(
            degenerate_hash_ignoring_new(&full),
            degenerate_hash_ignoring_new(&no_upgrade),
            "reason=degenerate_hash_must_not_detect_missing_upgrade"
        );
        assert!(full.content_hash() != no_upgrade.content_hash());
    }

    /// P8.1 定义属性（design D1）：三类新增建筑 `speed_raw == 0`、远程射程
    /// 大于近战、英雄 `mana > 0`。均为冻结定义内容，不进运行时权威状态。
    #[test]
    fn new_definitions_carry_required_frozen_properties() {
        let defs = synthetic_definitions();
        for id in HUMAN_NEW_BUILDING_IDS {
            let def = defs.units().get(&id).expect("reason=missing_new_building");
            assert_eq!(def.speed_raw, 0, "reason=building_speed_raw_zero id={id}");
        }
        let ranged = defs.units().get(&HUMAN_RANGED).unwrap();
        let soldier = defs.units().get(&HUMAN_SOLDIER).unwrap();
        assert!(
            ranged.range_raw > soldier.range_raw,
            "reason=ranged_out_ranges_melee"
        );
        let hero = defs.units().get(&HUMAN_HERO).unwrap();
        assert!(hero.mana > 0, "reason=hero_has_mana");
        assert!(hero.speed_raw > 0, "reason=hero_is_movable");
    }

    /// 英雄主动技能只用既有 `EffectKind` 组合表达（design D2 / N5）。
    /// 穷尽匹配 `EffectKind` 的四个既有变体：若 `w3-core` 新增变体，
    /// 本测试将无法编译，构成「不新增 EffectKind」的编译期守卫。
    #[test]
    fn hero_ability_uses_only_existing_effect_kinds() {
        fn kind_tag(kind: EffectKind) -> u8 {
            match kind {
                EffectKind::Damage => 0,
                EffectKind::Heal => 1,
                EffectKind::BuffStat => 2,
                EffectKind::Stun => 3,
            }
        }
        let defs = synthetic_definitions();
        let ability = defs
            .abilities()
            .get(&HERO_ABILITY)
            .expect("reason=missing_hero_ability");
        assert!(
            ability.effects.len() >= 2,
            "reason=ability_must_compose_multiple_effects"
        );
        let mut tags: Vec<u8> = ability
            .effects
            .iter()
            .map(|effect| kind_tag(effect.kind))
            .collect();
        tags.sort_unstable();
        assert!(tags.windows(2).all(|w| w[0] <= w[1]));
        assert!(tags.iter().all(|tag| *tag <= 3));
    }

    /// 新增类别分派（design D3）：人族六类落在 `1xx`、兽族占位落在 `2xx`，
    /// 与既有四个分派一致且不冲突；技能 / 升级是类别内唯一 ID、不分段。
    #[test]
    fn new_category_dispatch_is_segmented() {
        let human_ids = [
            Race::Human.ranged(),
            Race::Human.siege(),
            Race::Human.farm(),
            Race::Human.altar(),
            Race::Human.smithy(),
            Race::Human.hero(),
        ];
        assert!(human_ids.iter().all(|id| (100..200).contains(id)));
        assert_eq!(human_ids, HUMAN_NEW_UNIT_IDS);
        for id in [
            Race::Orc.ranged(),
            Race::Orc.siege(),
            Race::Orc.farm(),
            Race::Orc.altar(),
            Race::Orc.smithy(),
            Race::Orc.hero(),
        ] {
            assert!(
                (200..300).contains(&id),
                "reason=orc_placeholder_in_2xx id={id}"
            );
        }
        let human_binding = Race::Human.category_def_ids();
        let orc_binding = Race::Orc.category_def_ids();
        assert_eq!(human_binding.len(), 6);
        assert_eq!(orc_binding.len(), 6);
        assert_ne!(human_binding, orc_binding);
    }

    #[test]
    fn race_selects_the_matching_def_id_segment() {
        assert_eq!(Race::default(), Race::Human);
        for race in [Race::Human, Race::Orc] {
            let (lo, hi) = match race {
                Race::Human => (100, 200),
                Race::Orc => (200, 300),
            };
            for id in [
                race.peasant(),
                race.soldier(),
                race.town_hall(),
                race.barracks(),
            ] {
                assert!((lo..hi).contains(&id), "种族单位应落在对应段: {id}");
            }
            assert_eq!(race.gold_mine(), NEUTRAL_GOLD_MINE);
            assert_eq!(race.tree(), NEUTRAL_TREE);
        }
        assert_ne!(Race::Human.peasant(), Race::Orc.peasant());
        assert_ne!(Race::Human.town_hall(), Race::Orc.town_hall());
    }

    #[test]
    fn derive_movability_reads_speed_raw_not_type_names() {
        let defs = synthetic_definitions();
        let entity_defs: BTreeMap<u64, DefId> = [
            (1, HUMAN_PEASANT),
            (2, HUMAN_TOWN_HALL),
            (3, NEUTRAL_GOLD_MINE),
            (4, NEUTRAL_TREE),
            (5, ORC_SOLDIER),
            (6, ORC_BARRACKS),
            (7, HUMAN_RANGED),
            (8, HUMAN_SIEGE),
            (9, HUMAN_HERO),
            (10, HUMAN_FARM),
            (11, HUMAN_ALTAR),
            (12, HUMAN_SMITHY),
        ]
        .into_iter()
        .collect();
        let map = derive_movability(&defs, &entity_defs);
        assert_eq!(map.get(&1), Some(&true), "农民速度 > 0 可移动");
        assert_eq!(map.get(&2), Some(&false), "主城速度 0 不可移动");
        assert_eq!(map.get(&3), Some(&false), "金矿速度 0 不可移动");
        assert_eq!(map.get(&4), Some(&false), "树木速度 0 不可移动");
        assert_eq!(map.get(&5), Some(&true), "兽族士兵速度 > 0 可移动");
        assert_eq!(map.get(&6), Some(&false), "兽族兵营速度 0 不可移动");
        // P8.1 新类别：判据仍只读 `speed_raw`，不来自类型名 / 运行时速度。
        for id in [7u64, 8, 9] {
            assert_eq!(map.get(&id), Some(&true), "reason=movable_new_unit id={id}");
        }
        for id in [10u64, 11, 12] {
            assert_eq!(
                map.get(&id),
                Some(&false),
                "reason=immovable_new_building id={id}"
            );
        }
        // 定义缺失的实体按不可移动处理（显式保守）。
        let missing = derive_movability(&defs, &[(99, 999_999)].into_iter().collect());
        assert_eq!(missing.get(&99), Some(&false));
    }

    #[test]
    fn export_source_falls_back_without_io() {
        let missing = ExportSource::default();
        assert!(!missing.is_available());
        assert_eq!(missing.definitions(), synthetic_definitions());

        let provided = ExportSource {
            map_dir: Some("C:/export/echoisles".to_string()),
            table_path: Some("C:/export/UnitBalance.json".to_string()),
        };
        assert!(provided.is_available());
        assert_eq!(provided.definitions(), synthetic_definitions());
    }
}
