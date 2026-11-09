//! SLK 中立行到冻结定义的转换，以及冻结定义的内容哈希。
//!
//! 对应 OpenSpec change `p5-gameplay-systems` 任务 1.1（SLK 表 → 冻结定义与
//! 内容哈希）与任务 1.3 中属于内容层的「重复 ID 定义拒绝」部分；需求见
//! `specs/gameplay-systems/spec.md` 的 `Frozen content definitions`。
//! `design.md` 把「SLK 表到冻结定义的字段映射与默认值口径」列为 Open Question，
//! 本模块的字段映射表与默认值规则即为该问题的第一版答案。
//!
//! ## 定位与边界
//!
//! - 本模块是**纯转换层**：输入是 [`SlkTable`] 的中立行，输出是中立、无
//!   `w3-core` 依赖的冻结定义。字段只用 `String` / `i64` / `bool`，不含
//!   `SimFixed`、不含任何 `f64`。`w3-core` 不依赖本 crate，app 层参照
//!   `inject.rs` 的冻结边界，再把中立定义转换成 `w3-core` 类型并注入对局。
//! - 上游数值列若是浮点（如 `1.5`）会在边界被拒绝，而不是被截断或四舍五入，
//!   以保持「P2 之前不把浮点混入模拟」的硬红线。
//! - 内容哈希在一次转换后作为对局配置的一部分（进快照头，见 spec），不在
//!   运行时读表、不支持热更新；热更新与恢复校验由 `w3-core` 侧负责。
//!
//! ## 输入组织
//!
//! [`DefinitionTables`] 用具名字段显式标注每张表属于哪一类定义，而不是从
//! `SlkTable::source` 猜测类别（导出文件名不可靠、也随上游变化）。第一版
//! **每类定义对应一张规范表**，字段为 `Option`：某类表缺失即视为空定义集，
//! 与 `units.json` / `doodads.json` 可选文件的既有口径一致。将来若需把同一类
//! 的多张上游表合并，只需扩展输入结构与转换函数，公共定义结构保持不变（开闭原则）。
//!
//! ## 字段映射（第一版规范化列名，均为合成口径）
//!
//! 列名是内容层自定义的规范化键，与 `table.rs` 中既有 fixture 的
//! `unitID` / `Name` / `hitPoints` 风格一致；真实导出列名若有差异，属后续
//! 字段映射扩展，不影响本层公共结构。所有未列出的列一律忽略。
//!
//! 单位表（主键列 `unitID`）：
//!
//! | 字段 | 列名 | 类型 | 缺失默认值 |
//! |---|---|---|---|
//! | `id` | `unitID` | `String` | `""`，空值按 [`DefinitionError::MissingId`] 拒绝 |
//! | `name` | `Name` | `String` | `""` |
//! | `max_hp` | `hitPoints` | `i64` | `0` |
//! | `max_mana` | `manaPoints` | `i64` | `0` |
//! | `armor` | `armor` | `i64` | `0` |
//! | `is_hero` | `isHero` | `bool` | `false` |
//!
//! 技能表（主键列 `abilityID`）：`id`←`abilityID`、`name`←`Name`、
//! `mana_cost`←`manaCost`、`cooldown`←`cooldown`、`is_hero_ability`←`hero`。
//!
//! 物品表（主键列 `itemID`）：`id`←`itemID`、`name`←`Name`、
//! `gold_cost`←`goldCost`、`charges`←`charges`、`is_permanent`←`permanent`。
//!
//! 升级表（主键列 `upgradeID`）：`id`←`upgradeID`、`name`←`Name`、
//! `max_level`←`maxLevel`、`gold_cost_base`←`goldBase`、`time_base`←`timeBase`。
//!
//! ## 默认值口径（明确、可复现）
//!
//! 内容层是**纯读取器**，刻意不臆造任何游戏平衡默认值（「升级至少 1 级」
//! 「单位至少 1 点生命」这类语义补全属于 `w3-core` 的规则，不在本层决定）。
//! 因此对所有非主键字段采用唯一、零例外的机械规则：
//!
//! - **键缺失或值为 `null`**：数值字段取 `0`，字符串字段取 `""`，
//!   布尔字段取 `false`。缺失与 `null` 走同一默认值，口径完全一致、可复现。
//! - **数值列**：接受 JSON 整数，或可解析为 `i64` 的十进制字符串（上游导出常把
//!   数字写成字符串）；浮点、非数字字符串、以及 `null` 之外的其它类型按边界
//!   错误拒绝，其中 `null` 归为「取默认值」。布尔列额外接受 `0` / `1` 整数与
//!   `"0"` / `"1"` / `"true"` / `"false"` / 空串。
//! - **主键列**：必须为非空字符串；缺失、空串或 `null` 返回
//!   [`DefinitionError::MissingId`]。ID 是唯一有「必填」语义的字段。
//! - 数值不做取值范围裁剪、不解释游戏语义，全部是原始整数，交由 app / `w3-core`
//!   层在冻结时决定约束。
//! - 本模块唯一的常量是哈希编码版本 [`CONTENT_HASH_SCHEMA`]，它不是字段默认值，
//!   只用于哈希域分隔；没有任何需要维护的「文档化常量默认值」。
//!
//! ## 重复 ID 口径
//!
//! 同一类别内的 ID 必须唯一，重复返回 [`DefinitionError::DuplicateId`]；
//! 每类定义输出时按 `id` 升序排列（用 `BTreeMap` 收集）。不同类别是独立的 ID
//! 命名空间（内容层不解释 FourCC 语义），跨类别是否全局唯一由 `w3-core` 在
//! 冻结配置时决定。
//!
//! ## `content_hash` 编码约定
//!
//! 与 `w3-core::StateHasher` 同精神但本 crate 内独立实现：FNV-1a 64 位，
//! offset basis `0xcbf2_9ce4_8422_2325`、prime `0x0000_0100_0000_01b3`，
//! 逐字节 `hash = (hash ^ byte).wrapping_mul(prime)`，与平台字节序无关。
//!
//! 字节流按以下固定顺序拼接，任何一处都无歧义：
//!
//! 1. 写 [`CONTENT_HASH_SCHEMA`]（`u64` 小端）作版本域分隔；字段集或编码变化时
//!    递增该版本，避免新旧定义哈希空间混淆。
//! 2. 按固定类别顺序 单位 → 技能 → 物品 → 升级 逐段编码；每段先写类别标签
//!    （`u8`：1 / 2 / 3 / 4）与该段定义条数（`u64` 小端），标签与条数避免不同
//!    类别 / 数量的拼接产生相同字节流。
//! 3. 每段内按 `id` 升序排序后逐条编码；因此哈希与输入行顺序、以及
//!    [`FrozenDefinitions`] 各 `Vec` 的存储顺序都无关。
//! 4. 每条记录按字段固定顺序编码：`id`、`name`、各 `i64`、各 `bool`。
//!    `i64` / `u64` 用 `to_le_bytes()`（显式小端），`bool` 写单字节 `0` / `1`，
//!    `String` 写「`u64` 小端字节长度 + UTF-8 字节」，长度前缀消除字段边界歧义
//!    （例如 `"ab"+"c"` 与 `"a"+"bc"` 编码不同）。
//!
//! 相同内容必得相同哈希；任一字段值、ID、定义条数或类别归属变化都会改变字节流，
//! 从而改变哈希。

use std::collections::BTreeMap;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::table::{SlkTable, TableError};

/// FNV-1a 64 位 offset basis，与 `w3-core::StateHasher` 一致。
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64 位 prime，与 `w3-core::StateHasher` 一致。
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 内容哈希编码版本；字段集或编码顺序变化时递增。
pub const CONTENT_HASH_SCHEMA: u64 = 1;

/// 冻结定义的类别，用于错误定位与哈希的类别域分隔。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefinitionKind {
    /// 单位定义。
    Unit,
    /// 技能定义。
    Ability,
    /// 物品定义。
    Item,
    /// 升级定义。
    Upgrade,
}

impl DefinitionKind {
    /// 哈希编码用的固定类别标签，与枚举声明顺序解耦，避免重排影响哈希。
    fn tag(&self) -> u8 {
        match self {
            Self::Unit => 1,
            Self::Ability => 2,
            Self::Item => 3,
            Self::Upgrade => 4,
        }
    }

    /// 英文错误文本用的类别标签。
    fn label(&self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Ability => "ability",
            Self::Item => "item",
            Self::Upgrade => "upgrade",
        }
    }
}

/// 单位冻结定义（中立结构，字段只用 `String` / `i64` / `bool`）。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UnitDef {
    /// 主键（列 `unitID`）；缺失或空值在转换期被拒绝。
    #[serde(default, rename = "unitID", deserialize_with = "de_string")]
    pub id: String,
    /// 名称（列 `Name`），缺失取空串。
    #[serde(default, rename = "Name", deserialize_with = "de_string")]
    pub name: String,
    /// 生命上限（列 `hitPoints`），缺失取 `0`。
    #[serde(default, rename = "hitPoints", deserialize_with = "de_i64")]
    pub max_hp: i64,
    /// 法力上限（列 `manaPoints`），缺失取 `0`。
    #[serde(default, rename = "manaPoints", deserialize_with = "de_i64")]
    pub max_mana: i64,
    /// 护甲（列 `armor`），缺失取 `0`。
    #[serde(default, rename = "armor", deserialize_with = "de_i64")]
    pub armor: i64,
    /// 是否英雄（列 `isHero`），缺失取 `false`。
    #[serde(default, rename = "isHero", deserialize_with = "de_bool")]
    pub is_hero: bool,
}

/// 技能冻结定义。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AbilityDef {
    /// 主键（列 `abilityID`）；缺失或空值在转换期被拒绝。
    #[serde(default, rename = "abilityID", deserialize_with = "de_string")]
    pub id: String,
    /// 名称（列 `Name`），缺失取空串。
    #[serde(default, rename = "Name", deserialize_with = "de_string")]
    pub name: String,
    /// 法力消耗（列 `manaCost`），缺失取 `0`。
    #[serde(default, rename = "manaCost", deserialize_with = "de_i64")]
    pub mana_cost: i64,
    /// 冷却（列 `cooldown`），缺失取 `0`。
    #[serde(default, rename = "cooldown", deserialize_with = "de_i64")]
    pub cooldown: i64,
    /// 是否英雄技能（列 `hero`），缺失取 `false`。
    #[serde(default, rename = "hero", deserialize_with = "de_bool")]
    pub is_hero_ability: bool,
}

/// 物品冻结定义。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ItemDef {
    /// 主键（列 `itemID`）；缺失或空值在转换期被拒绝。
    #[serde(default, rename = "itemID", deserialize_with = "de_string")]
    pub id: String,
    /// 名称（列 `Name`），缺失取空串。
    #[serde(default, rename = "Name", deserialize_with = "de_string")]
    pub name: String,
    /// 金币价格（列 `goldCost`），缺失取 `0`。
    #[serde(default, rename = "goldCost", deserialize_with = "de_i64")]
    pub gold_cost: i64,
    /// 使用次数（列 `charges`），缺失取 `0`。
    #[serde(default, rename = "charges", deserialize_with = "de_i64")]
    pub charges: i64,
    /// 是否永久物品（列 `permanent`），缺失取 `false`。
    #[serde(default, rename = "permanent", deserialize_with = "de_bool")]
    pub is_permanent: bool,
}

/// 升级冻结定义。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UpgradeDef {
    /// 主键（列 `upgradeID`）；缺失或空值在转换期被拒绝。
    #[serde(default, rename = "upgradeID", deserialize_with = "de_string")]
    pub id: String,
    /// 名称（列 `Name`），缺失取空串。
    #[serde(default, rename = "Name", deserialize_with = "de_string")]
    pub name: String,
    /// 最高等级（列 `maxLevel`），缺失取 `0`。
    #[serde(default, rename = "maxLevel", deserialize_with = "de_i64")]
    pub max_level: i64,
    /// 基础金币消耗（列 `goldBase`），缺失取 `0`。
    #[serde(default, rename = "goldBase", deserialize_with = "de_i64")]
    pub gold_cost_base: i64,
    /// 基础研究时间（列 `timeBase`），缺失取 `0`。
    #[serde(default, rename = "timeBase", deserialize_with = "de_i64")]
    pub time_base: i64,
}

/// 四类 SLK 表到冻结定义的输入集合；每类可缺省（`None` 视为空定义集）。
///
/// 用法：用具名字段显式给出每类对应的表，例如只转换单位表时写
/// `DefinitionTables { units: Some(&table), ..Default::default() }`。
#[derive(Clone, Debug, Default)]
pub struct DefinitionTables<'a> {
    /// 单位表（主键列 `unitID`）。
    pub units: Option<&'a SlkTable>,
    /// 技能表（主键列 `abilityID`）。
    pub abilities: Option<&'a SlkTable>,
    /// 物品表（主键列 `itemID`）。
    pub items: Option<&'a SlkTable>,
    /// 升级表（主键列 `upgradeID`）。
    pub upgrades: Option<&'a SlkTable>,
}

/// 冻结定义的集合：四类定义各自按 `id` 升序排列。
///
/// 字段为 `pub`，供 app 层读取后冻结进 `w3-core` 类型；[`content_hash`] 每次
/// 现算且内部按 `id` 规范化排序，因此哈希始终反映当前内容，不依赖存储顺序。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrozenDefinitions {
    /// 单位定义，按 `id` 升序。
    pub units: Vec<UnitDef>,
    /// 技能定义，按 `id` 升序。
    pub abilities: Vec<AbilityDef>,
    /// 物品定义，按 `id` 升序。
    pub items: Vec<ItemDef>,
    /// 升级定义，按 `id` 升序。
    pub upgrades: Vec<UpgradeDef>,
}

/// 转换 / 校验冻结定义时的边界错误（错误文本一律英文）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefinitionError {
    /// 同一类别内出现重复 ID。
    DuplicateId { kind: DefinitionKind, id: String },
    /// 某行主键列缺失、为空或为 `null`。
    MissingId { kind: DefinitionKind, row: usize },
    /// 某行字段类型或取值非法（含浮点数值列）。
    InvalidRow {
        kind: DefinitionKind,
        row: usize,
        message: String,
    },
    /// 输入表的 `recordCount` 与 `records` 长度不一致。
    InvalidTable(TableError),
}

impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId { kind, id } => {
                write!(f, "duplicate {} definition id '{id}'", kind.label())
            }
            Self::MissingId { kind, row } => {
                write!(f, "missing id in {} definition at row {row}", kind.label())
            }
            Self::InvalidRow { kind, row, message } => {
                write!(
                    f,
                    "invalid {} definition at row {row}: {message}",
                    kind.label()
                )
            }
            Self::InvalidTable(error) => write!(f, "invalid SLK table: {error:?}"),
        }
    }
}

impl std::error::Error for DefinitionError {}

/// 把若干 SLK 表转换成冻结定义。
///
/// 逐表校验 `recordCount` 一致性，逐行按字段映射表转换；未知列被 serde 忽略，
/// 因此上游 schema 增长不会破坏读取。缺表视作空定义集，重复 ID、缺失 ID 与
/// 非法取值都会返回明确的 [`DefinitionError`]。输出的每类定义按 `id` 升序。
pub fn from_slk_tables(
    tables: &DefinitionTables<'_>,
) -> Result<FrozenDefinitions, DefinitionError> {
    Ok(FrozenDefinitions {
        units: parse_rows(DefinitionKind::Unit, tables.units)?,
        abilities: parse_rows(DefinitionKind::Ability, tables.abilities)?,
        items: parse_rows(DefinitionKind::Item, tables.items)?,
        upgrades: parse_rows(DefinitionKind::Upgrade, tables.upgrades)?,
    })
}

/// 让泛型转换 / 哈希函数能读取任意类别定义的主键。
trait DefinitionId {
    fn id(&self) -> &str;
}

impl DefinitionId for UnitDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl DefinitionId for AbilityDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl DefinitionId for ItemDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl DefinitionId for UpgradeDef {
    fn id(&self) -> &str {
        &self.id
    }
}

/// 把单张表转换成一类定义：校验、反序列化、主键校验、重复检测、按 ID 排序。
fn parse_rows<T>(kind: DefinitionKind, table: Option<&SlkTable>) -> Result<Vec<T>, DefinitionError>
where
    T: DeserializeOwned + DefinitionId,
{
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    table.validate().map_err(DefinitionError::InvalidTable)?;

    // BTreeMap 既做类别内重复检测，又天然给出按 id 升序的稳定顺序。
    let mut by_id: BTreeMap<String, T> = BTreeMap::new();
    for (row_index, row) in table.records.iter().enumerate() {
        // 中立行是 BTreeMap<String, Value>；转成 Value::Object 交给 serde，
        // 未列出的列由派生的 Deserialize 自动忽略。这里逐项显式 clone，
        // 避免对 &BTreeMap 直接调用 clone() 时返回引用集合的类型歧义。
        let object: Map<String, Value> = row
            .iter()
            .map(|(key, value)| ((*key).clone(), (*value).clone()))
            .collect();
        let def: T = serde_json::from_value(Value::Object(object)).map_err(|error| {
            DefinitionError::InvalidRow {
                kind,
                row: row_index,
                message: error.to_string(),
            }
        })?;

        let id = def.id().to_owned();
        if id.is_empty() {
            return Err(DefinitionError::MissingId {
                kind,
                row: row_index,
            });
        }
        if by_id.contains_key(&id) {
            return Err(DefinitionError::DuplicateId { kind, id });
        }
        by_id.insert(id, def);
    }
    Ok(by_id.into_values().collect())
}

/// 宽松整数反序列化：JSON 整数或可解析的十进制字符串；`null` 取默认 `0`；
/// 浮点与其它类型在边界拒绝，避免 `f64` 混入冻结定义。
fn de_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| <D::Error as serde::de::Error>::custom("integer value out of range")),
        Value::String(text) => text
            .parse::<i64>()
            .map_err(|_| <D::Error as serde::de::Error>::custom("invalid integer string")),
        Value::Null => Ok(0),
        _ => Err(<D::Error as serde::de::Error>::custom(
            "expected an integer value",
        )),
    }
}

/// 宽松字符串反序列化：字符串原样；`null` 取默认空串；其它类型拒绝。
fn de_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => Ok(text),
        Value::Null => Ok(String::new()),
        _ => Err(<D::Error as serde::de::Error>::custom(
            "expected a string value",
        )),
    }
}

/// 宽松布尔反序列化：布尔、`0` / `1` 整数、常见字符串；`null` 取默认 `false`。
fn de_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Bool(flag) => Ok(flag),
        Value::Number(number) => match number.as_i64() {
            Some(0) => Ok(false),
            Some(_) => Ok(true),
            None => Err(<D::Error as serde::de::Error>::custom(
                "expected an integer flag",
            )),
        },
        Value::String(text) => match text.as_str() {
            "1" | "true" | "TRUE" => Ok(true),
            "0" | "false" | "FALSE" | "" => Ok(false),
            _ => Err(<D::Error as serde::de::Error>::custom(
                "invalid boolean string",
            )),
        },
        Value::Null => Ok(false),
        _ => Err(<D::Error as serde::de::Error>::custom(
            "expected a boolean value",
        )),
    }
}

/// 独立的 FNV-1a 64 位哈希器，与 `w3-core::StateHasher` 同精神但不依赖它。
struct ContentHasher(u64);

impl ContentHasher {
    fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    /// 长度前缀 + UTF-8 字节，消除变长字段的边界歧义。
    fn write_str(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn finish(self) -> u64 {
        self.0
    }
}

/// 编码一类定义：类别标签 + 条数 + 按 id 升序的逐条字段。
fn hash_defs<T, F>(hasher: &mut ContentHasher, kind: DefinitionKind, defs: &[T], write_fields: F)
where
    T: DefinitionId,
    F: Fn(&mut ContentHasher, &T),
{
    hasher.write_u8(kind.tag());
    hasher.write_u64(defs.len() as u64);
    let mut ordered: Vec<&T> = defs.iter().collect();
    ordered.sort_by(|a, b| a.id().cmp(b.id()));
    for def in ordered {
        write_fields(hasher, def);
    }
}

impl FrozenDefinitions {
    /// 计算全部冻结定义的内容哈希（FNV-1a 64，显式小端编码）。
    ///
    /// 编码约定见模块文档：版本域分隔 → 四类按固定顺序 → 每类按 `id` 升序 →
    /// 逐字段定长 / 带长度前缀编码。相同内容必得相同哈希；内容变化则哈希变化；
    /// 结果与输入行顺序及各 `Vec` 存储顺序无关。
    pub fn content_hash(&self) -> u64 {
        let mut hasher = ContentHasher::new();
        hasher.write_u64(CONTENT_HASH_SCHEMA);

        hash_defs(
            &mut hasher,
            DefinitionKind::Unit,
            &self.units,
            |hasher, def| {
                hasher.write_str(&def.id);
                hasher.write_str(&def.name);
                hasher.write_i64(def.max_hp);
                hasher.write_i64(def.max_mana);
                hasher.write_i64(def.armor);
                hasher.write_bool(def.is_hero);
            },
        );
        hash_defs(
            &mut hasher,
            DefinitionKind::Ability,
            &self.abilities,
            |hasher, def| {
                hasher.write_str(&def.id);
                hasher.write_str(&def.name);
                hasher.write_i64(def.mana_cost);
                hasher.write_i64(def.cooldown);
                hasher.write_bool(def.is_hero_ability);
            },
        );
        hash_defs(
            &mut hasher,
            DefinitionKind::Item,
            &self.items,
            |hasher, def| {
                hasher.write_str(&def.id);
                hasher.write_str(&def.name);
                hasher.write_i64(def.gold_cost);
                hasher.write_i64(def.charges);
                hasher.write_bool(def.is_permanent);
            },
        );
        hash_defs(
            &mut hasher,
            DefinitionKind::Upgrade,
            &self.upgrades,
            |hasher, def| {
                hasher.write_str(&def.id);
                hasher.write_str(&def.name);
                hasher.write_i64(def.max_level);
                hasher.write_i64(def.gold_cost_base);
                hasher.write_i64(def.time_base);
            },
        );

        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 从合成 JSON 构造一张 SLK 表；所有 fixture 均为内联合成数据。
    fn slk(json: &str) -> SlkTable {
        serde_json::from_str(json).expect("synthetic table must parse")
    }

    /// 覆盖四类定义各一条的合成表集合，供哈希类测试复用。
    fn full_tables() -> (SlkTable, SlkTable, SlkTable, SlkTable) {
        let units = slk(r#"{"recordCount":2,"records":[
                {"unitID":"BBBB","Name":"Synthetic B","hitPoints":200,"manaPoints":50,"armor":3,"isHero":1,"futureColumn":true},
                {"unitID":"AAAA","Name":"Synthetic A","hitPoints":"100","isHero":false}
            ]}"#);
        let abilities = slk(
            r#"{"records":[{"abilityID":"CCCC","Name":"Synthetic C","manaCost":10,"cooldown":5,"hero":true}]}"#,
        );
        let items =
            slk(r#"{"records":[{"itemID":"DDDD","goldCost":250,"charges":2,"permanent":"1"}]}"#);
        let upgrades = slk(
            r#"{"records":[{"upgradeID":"EEEE","Name":"Synthetic E","goldBase":100,"timeBase":60}]}"#,
        );
        (units, abilities, items, upgrades)
    }

    fn definitions(
        tables: &(SlkTable, SlkTable, SlkTable, SlkTable),
    ) -> Result<FrozenDefinitions, DefinitionError> {
        from_slk_tables(&DefinitionTables {
            units: Some(&tables.0),
            abilities: Some(&tables.1),
            items: Some(&tables.2),
            upgrades: Some(&tables.3),
        })
    }

    #[test]
    fn converts_rows_with_documented_defaults_and_ignores_unknown_fields() {
        let defs = definitions(&full_tables()).expect("conversion must succeed");

        // 输出的每类定义都按 id 升序。
        assert_eq!(defs.units.len(), 2);
        assert_eq!(defs.units[0].id, "AAAA");
        assert_eq!(defs.units[1].id, "BBBB");

        // 字符串数值被接受；缺失字段按 0 / 空 / false 取默认；未知列被忽略。
        let a = &defs.units[0];
        assert_eq!(a.name, "Synthetic A");
        assert_eq!(a.max_hp, 100);
        assert_eq!(a.max_mana, 0);
        assert_eq!(a.armor, 0);
        assert!(!a.is_hero);

        let b = &defs.units[1];
        assert_eq!(b.max_hp, 200);
        assert_eq!(b.max_mana, 50);
        assert_eq!(b.armor, 3);
        assert!(b.is_hero);

        // 技能 / 物品 / 升级的映射与宽松取值。
        assert_eq!(defs.abilities[0].mana_cost, 10);
        assert_eq!(defs.abilities[0].cooldown, 5);
        assert!(defs.abilities[0].is_hero_ability);
        assert_eq!(defs.items[0].name, "");
        assert_eq!(defs.items[0].gold_cost, 250);
        assert_eq!(defs.items[0].charges, 2);
        assert!(defs.items[0].is_permanent);
        // maxLevel 缺失时按统一口径取 0，与显式 null / 其它数值字段一致。
        assert_eq!(defs.upgrades[0].max_level, 0);
        assert_eq!(defs.upgrades[0].gold_cost_base, 100);
        assert_eq!(defs.upgrades[0].time_base, 60);
    }

    #[test]
    fn explicit_null_matches_missing_field_defaults() {
        // 缺失与 null 必须走同一默认值，保证口径零例外、可复现。
        let missing = slk(r#"{"records":[{"unitID":"AAAA"}]}"#);
        let nulls =
            slk(r#"{"records":[{"unitID":"AAAA","Name":null,"hitPoints":null,"isHero":null}]}"#);
        let from_missing = from_slk_tables(&DefinitionTables {
            units: Some(&missing),
            ..Default::default()
        })
        .unwrap();
        let from_nulls = from_slk_tables(&DefinitionTables {
            units: Some(&nulls),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(from_missing, from_nulls);
        assert_eq!(from_nulls.units[0].name, "");
        assert_eq!(from_nulls.units[0].max_hp, 0);
        assert!(!from_nulls.units[0].is_hero);
    }

    #[test]
    fn content_hash_is_stable_and_independent_of_input_order() {
        let a = definitions(&full_tables()).unwrap();
        let b = definitions(&full_tables()).unwrap();
        assert_eq!(a.content_hash(), b.content_hash());

        // 输入行顺序颠倒、额外未知列，都不影响内容哈希（按 id 规范化）。
        let reordered = slk(r#"{"records":[
                {"unitID":"AAAA","Name":"Synthetic A","hitPoints":"100","isHero":false,"extra":1},
                {"unitID":"BBBB","Name":"Synthetic B","hitPoints":200,"manaPoints":50,"armor":3,"isHero":1}
            ]}"#);
        let (_, abilities, items, upgrades) = full_tables();
        let c = from_slk_tables(&DefinitionTables {
            units: Some(&reordered),
            abilities: Some(&abilities),
            items: Some(&items),
            upgrades: Some(&upgrades),
        })
        .unwrap();
        assert_eq!(a.content_hash(), c.content_hash());
    }

    #[test]
    fn content_hash_is_independent_of_storage_order() {
        let mut defs = definitions(&full_tables()).unwrap();
        let before = defs.content_hash();
        defs.units.reverse();
        defs.upgrades.reverse();
        assert_eq!(defs.content_hash(), before);
    }

    #[test]
    fn content_hash_changes_when_content_changes() {
        let defs = definitions(&full_tables()).unwrap();
        let original = defs.content_hash();

        let mut changed_value = defs.clone();
        changed_value.units[0].max_hp += 1;
        assert_ne!(changed_value.content_hash(), original);

        let mut changed_name = defs.clone();
        changed_name.abilities[0].name.push('!');
        assert_ne!(changed_name.content_hash(), original);

        let mut added = defs.clone();
        added.items.push(ItemDef {
            id: "FFFF".to_owned(),
            name: String::new(),
            gold_cost: 0,
            charges: 0,
            is_permanent: false,
        });
        assert_ne!(added.content_hash(), original);

        // 类别归属不同也应改变哈希（域分隔标签生效）。
        let mut moved = defs.clone();
        moved.units.clear();
        assert_ne!(moved.content_hash(), original);
    }

    #[test]
    fn all_four_kinds_contribute_to_the_hash() {
        let defs = definitions(&full_tables()).unwrap();
        let original = defs.content_hash();

        let mut no_units = defs.clone();
        no_units.units.clear();
        assert_ne!(no_units.content_hash(), original);

        let mut no_abilities = defs.clone();
        no_abilities.abilities.clear();
        assert_ne!(no_abilities.content_hash(), original);

        let mut no_items = defs.clone();
        no_items.items.clear();
        assert_ne!(no_items.content_hash(), original);

        let mut no_upgrades = defs.clone();
        no_upgrades.upgrades.clear();
        assert_ne!(no_upgrades.content_hash(), original);
    }

    #[test]
    fn duplicate_id_in_same_kind_is_rejected() {
        let units = slk(r#"{"recordCount":2,"records":[{"unitID":"AAAA"},{"unitID":"AAAA"}]}"#);
        let error = from_slk_tables(&DefinitionTables {
            units: Some(&units),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(
            error,
            DefinitionError::DuplicateId {
                kind: DefinitionKind::Unit,
                id: "AAAA".to_owned(),
            }
        );
        // 错误文本为英文且带定位信息。
        assert!(
            error
                .to_string()
                .contains("duplicate unit definition id 'AAAA'")
        );
    }

    #[test]
    fn same_id_in_different_kinds_is_allowed() {
        // 内容层按类别独立命名空间去重；跨类别全局唯一性由 w3-core 决定。
        let units = slk(r#"{"records":[{"unitID":"SAME"}]}"#);
        let items = slk(r#"{"records":[{"itemID":"SAME"}]}"#);
        let defs = from_slk_tables(&DefinitionTables {
            units: Some(&units),
            items: Some(&items),
            ..Default::default()
        })
        .expect("different kinds have independent id spaces");
        assert_eq!(defs.units[0].id, "SAME");
        assert_eq!(defs.items[0].id, "SAME");
    }

    #[test]
    fn missing_id_is_rejected() {
        let units = slk(r#"{"records":[{"unitID":"AAAA"},{"Name":"no id here"}]}"#);
        let error = from_slk_tables(&DefinitionTables {
            units: Some(&units),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(
            error,
            DefinitionError::MissingId {
                kind: DefinitionKind::Unit,
                row: 1,
            }
        );
    }

    #[test]
    fn floating_point_value_is_rejected_at_the_boundary() {
        // 浮点数值列在边界被拒绝，绝不把 f64 带进冻结定义。
        let units = slk(r#"{"records":[{"unitID":"AAAA","hitPoints":1.5}]}"#);
        let error = from_slk_tables(&DefinitionTables {
            units: Some(&units),
            ..Default::default()
        })
        .unwrap_err();
        assert!(matches!(
            error,
            DefinitionError::InvalidRow {
                kind: DefinitionKind::Unit,
                row: 0,
                ..
            }
        ));
    }

    #[test]
    fn invalid_record_count_is_rejected() {
        let units = slk(r#"{"recordCount":5,"records":[{"unitID":"AAAA"}]}"#);
        let error = from_slk_tables(&DefinitionTables {
            units: Some(&units),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(
            error,
            DefinitionError::InvalidTable(TableError::RecordCountMismatch {
                declared: 5,
                found: 1,
            })
        );
    }

    #[test]
    fn empty_tables_produce_empty_definitions_and_stable_hash() {
        let a = from_slk_tables(&DefinitionTables::default()).unwrap();
        let b = from_slk_tables(&DefinitionTables::default()).unwrap();
        assert!(a.units.is_empty());
        assert!(a.abilities.is_empty());
        assert!(a.items.is_empty());
        assert!(a.upgrades.is_empty());
        assert_eq!(a.content_hash(), b.content_hash());
        // 空集与含一条定义的集合哈希必须不同。
        assert_ne!(
            a.content_hash(),
            definitions(&full_tables()).unwrap().content_hash()
        );
    }
}
