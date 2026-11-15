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

use w3_core::{
    AbilityDef, DefId, EffectDef, EffectKind, FrozenDefinitions, ItemDef, SimFixed, UnitDef,
    UpgradeDef,
};

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
/// 升级定义编号。
pub const UPGRADE: DefId = 20;
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

/// 合成冻结定义：既有 6 个基线单位 + 人族 / 兽族 / 中立资源分段单位 +
/// 技能、升级、物品。
///
/// 数值全为编译期整数常量；主城生命刻意压低，使一局能在合理帧数内打完。
/// 分段单位与基线单位同数值（P8.0 只做地基，不交付可玩的两族差异），
/// 但 `speed_raw` 由单位性质决定：农民 / 士兵 `> 0`，建筑 / 资源点 `== 0`，
/// 供 [`crate::input::derive_movability`] 作为可移动性判据（design D1）。
pub fn synthetic_definitions() -> FrozenDefinitions {
    let units = vec![
        // 既有基线定义（1..=6），既有入口行为不变。
        (PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        (GOLD_MINE, unit(500, 0, 0, 0, 0, 0)),
        (TREE, unit(200, 0, 0, 0, 0, 0)),
        // 人族段 1xx。
        (HUMAN_PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (HUMAN_SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (HUMAN_TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (HUMAN_BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        // 兽族段 2xx。
        (ORC_PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (ORC_SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (ORC_TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (ORC_BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        // 中立 / 资源段 9xx，两族共用。
        (NEUTRAL_GOLD_MINE, unit(500, 0, 0, 0, 0, 0)),
        (NEUTRAL_TREE, unit(200, 0, 0, 0, 0, 0)),
    ];
    let abilities = vec![(
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
    )];
    let items = vec![(
        ITEM,
        ItemDef {
            hp_mod: 10,
            damage_mod: 2,
        },
    )];
    let upgrades = vec![(
        UPGRADE,
        UpgradeDef {
            damage_mod: 4,
            hp_mod: 10,
            research_frames: 30,
        },
    )];
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

    /// 分段单位定义编号全集（`units` 类别，含既有 6 个基线定义）。
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
            ORC_PEASANT,
            ORC_SOLDIER,
            ORC_TOWN_HALL,
            ORC_BARRACKS,
            NEUTRAL_GOLD_MINE,
            NEUTRAL_TREE,
        ]
    }

    #[test]
    fn definitions_cover_all_kinds_and_are_stable() {
        let defs = synthetic_definitions();
        assert_eq!(defs.units().len(), 16);
        assert!(defs.units().contains_key(&PEASANT));
        assert!(defs.units().contains_key(&SOLDIER));
        assert!(defs.units().contains_key(&TOWN_HALL));
        assert!(defs.units().contains_key(&BARRACKS));
        assert!(defs.units().contains_key(&GOLD_MINE));
        assert!(defs.units().contains_key(&TREE));
        assert!(defs.abilities().contains_key(&ABILITY));
        assert!(defs.items().contains_key(&ITEM));
        assert!(defs.upgrades().contains_key(&UPGRADE));
        assert_eq!(defs, synthetic_definitions());
    }

    #[test]
    fn units_def_ids_are_globally_unique_and_segmented() {
        let ids = all_unit_ids();
        let unique: BTreeSet<DefId> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "units 类别 DefId 必须全局唯一");

        // 分段只约束 units 类别：三个非 units 类别沿用类别内唯一 ID。
        assert_eq!(ABILITY, 10);
        assert_eq!(UPGRADE, 20);
        assert_eq!(ITEM, 30);

        // 既有 6 个基线定义保持原值，分段常量不得与之冲突。
        let baseline: BTreeSet<DefId> = [1, 2, 3, 4, 5, 6].into_iter().collect();
        let segmented = [
            HUMAN_PEASANT,
            HUMAN_SOLDIER,
            HUMAN_TOWN_HALL,
            HUMAN_BARRACKS,
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

        // 段位分配：人族 1xx / 兽族 2xx / 中立资源 9xx。
        for id in [
            HUMAN_PEASANT,
            HUMAN_SOLDIER,
            HUMAN_TOWN_HALL,
            HUMAN_BARRACKS,
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
