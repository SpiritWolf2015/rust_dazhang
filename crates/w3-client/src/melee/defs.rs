//! 合成 / 原创冻结玩法定义（design D2）。
//!
//! 全部为编译期整数常量，不读取 `config.toml`、导出 JSON 或任何游戏资产
//! （ADR 0001）。定义经 `FrozenDefinitions::new` 冻结后，内容哈希进入快照
//! 头部；对局中无热更新入口。速度与射程经 `SimFixed::from_int` 冻结，
//! 不存在 `f64 → SimFixed` 直转路径。

use w3_core::{
    AbilityDef, DefId, EffectDef, EffectKind, FrozenDefinitions, ItemDef, SimFixed, UnitDef,
    UpgradeDef,
};

/// 单位定义编号。`1` 与 `w3_client::input::DEFAULT_UNIT_DEF` 对齐，
/// 故 `T` 默认训练战斗单位；既有 `playable.rs` 入口注入的 `unit_def=1`
/// 也仍然有效（不破坏兼容）。
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

/// 合成冻结定义：农民、士兵、主城、兵营、金矿、树木、技能、升级、物品。
///
/// 数值全为编译期整数常量；主城生命刻意压低，使一局能在合理帧数内打完。
pub fn synthetic_definitions() -> FrozenDefinitions {
    let units = vec![
        (PEASANT, unit(60, 3, 4, 4, 1, 20)),
        (SOLDIER, unit(120, 4, 30, 200, 1, 30)),
        (TOWN_HALL, unit(120, 0, 0, 0, 0, 60)),
        (BARRACKS, unit(300, 0, 0, 0, 0, 40)),
        (GOLD_MINE, unit(500, 0, 0, 0, 0, 0)),
        (TREE, unit(200, 0, 0, 0, 0, 0)),
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
    use super::*;

    #[test]
    fn definitions_cover_all_kinds_and_are_stable() {
        let defs = synthetic_definitions();
        assert_eq!(defs.units().len(), 6);
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
