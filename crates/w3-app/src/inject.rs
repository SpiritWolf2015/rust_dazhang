//! 冻结地图数据注入 `w3-core`（见 ADR 0004 与 `design.md`）。
//!
//! `ParsedMap` 的原始 `f64` 在边界经 [`freeze_f64`] 转成 `SimFixed`，
//! 再组装成中立的 `StaticWorld` 与出生命令。`w3-core` 不读取文件、
//! 不反向依赖内容层。
//!
//! P5 追加：把 `w3-content` 从 SLK 表转换出的中立冻结定义，经本模块转换成
//! `w3-core` 的冻结定义并注入对局配置，令「SLK 表 → 内容层定义 → 内核定义 →
//! 内容哈希进快照头」的链路在真实 app 入口成立（见 `design.md`）。

use std::fmt;

use w3_content::{FrozenDefinitions as ContentDefinitions, HeightfieldError, ParsedMap};
use w3_core::{
    AbilityDef as CoreAbilityDef, Command, CommandKind, DefId,
    DefinitionError as CoreDefinitionError, EffectDef, EffectKind, Frame,
    FrozenDefinitions as CoreDefinitions, ItemDef as CoreItemDef, MatchConfig, RtsMatch, SCALE,
    SimFixed, StaticWorld, UnitDef as CoreUnitDef, UpgradeDef as CoreUpgradeDef,
};

use crate::freeze::{FrozenError, freeze_f64};

/// 上游 `MAP_DATA.md` 给出的 `tileSize` 默认值（缺失时使用）。
const DEFAULT_TILE_SIZE: f64 = 128.0;

/// 第一版 app 层字段映射的冻结常量（内容层 SLK 列未提供对应字段时使用）。
///
/// 口径单一、可复现，全部为编译期常量，不读取任何运行时配置；新增 SLK 列时
/// 只需扩展 [`freeze_definitions`] 的映射，不改公共结构。
const DEFAULT_SPEED_RAW: i64 = 3 * SCALE;
const DEFAULT_DAMAGE: i64 = 10;
const DEFAULT_RANGE_RAW: i64 = 10 * SCALE;
const DEFAULT_SUPPLY: i64 = 1;
const DEFAULT_TRAIN_FRAMES: u32 = 5;
const DEFAULT_ABILITY_DAMAGE: i64 = 10;
const DEFAULT_ITEM_HP_MOD: i64 = 5;
const DEFAULT_ITEM_DAMAGE_MOD: i64 = 1;
const DEFAULT_UPGRADE_DAMAGE_MOD: i64 = 1;
const DEFAULT_UPGRADE_HP_MOD: i64 = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InjectError {
    Frozen(FrozenError),
    Heightfield(HeightfieldError),
    /// 冻结定义转换失败（目前仅 `String` ID 编码碰撞导致的重复 ID）。
    Definition(CoreDefinitionError),
    InvalidCellSize,
}

impl From<FrozenError> for InjectError {
    fn from(error: FrozenError) -> Self {
        Self::Frozen(error)
    }
}

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frozen(error) => write!(f, "coordinate freeze failed: {error}"),
            Self::Heightfield(error) => write!(f, "invalid heightfield: {error:?}"),
            Self::Definition(error) => write!(f, "definition freeze failed: {error}"),
            Self::InvalidCellSize => f.write_str("tileSize must be positive"),
        }
    }
}

impl std::error::Error for InjectError {}

/// 内容层 `String` 主键 → 内核 `DefId(u32)` 的稳定编码：FNV-1a 32 位 over UTF-8。
///
/// 显式、平台无关；若两个不同 ID 碰撞，`CoreDefinitions::new` 会以
/// `DuplicateId` 明确拒绝，不静默合并定义。
pub fn definition_id(id: &str) -> DefId {
    const OFFSET: u32 = 0x811c_9dc5;
    const PRIME: u32 = 0x0100_0193;
    let mut hash = OFFSET;
    for byte in id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// 把静态地图身份与冻结定义内容哈希组合成对局内容身份。
///
/// 显式小端 FNV-1a，平台无关；任一输入变化都会改变结果。app 层把它写进
/// `MatchConfig.content_hash`，恢复校验时作为 `expected_content_hash` 传入，
/// 从而同时锚定地图与内容定义。
pub fn combine_content_hash(world_identity: u64, defs_hash: u64) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in world_identity
        .to_le_bytes()
        .into_iter()
        .chain(defs_hash.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// 把内容层中立冻结定义转换成内核冻结定义（任务 1.1 的 app 层注入边界）。
///
/// 第一版字段映射口径（内容层 SLK 列集合见 `w3-content::definitions` 模块文档）：
/// - 主键：[`definition_id`]（FNV-1a 32 位）；碰撞由 `new` 明确拒绝。
/// - 单位：`hp ← max_hp`、`mana ← max_mana`；`speed_raw/damage/range_raw/
///   supply/train_frames` 取冻结常量；`armor/is_hero/name` 第一版无对应内核
///   字段，显式忽略。
/// - 技能：`mana_cost ← mana_cost`、`cooldown_frames ← cooldown`（非负截断）；
///   `cast_frames/behavior` 取 0；`effects` 取单条冻结默认伤害效果，使第一版
///   技能可结算；`hero/name` 显式忽略。
/// - 物品：`hp_mod/damage_mod` 取冻结常量；`gold_cost/charges/permanent/name`
///   第一版无对应内核字段，显式忽略。
/// - 升级：`damage_mod/hp_mod` 取冻结常量、`research_frames ← time_base`
///   （非负截断）；`max_level/gold_cost_base/name` 显式忽略。
///
/// 「内容层有值、内核无对应字段」一律用显式常量或显式忽略，不臆造游戏语义；
/// 后续扩展 SLK 列时只改本函数。
pub fn freeze_definitions(defs: &ContentDefinitions) -> Result<CoreDefinitions, InjectError> {
    let units: Vec<(DefId, CoreUnitDef)> = defs
        .units
        .iter()
        .map(|def| {
            (
                definition_id(&def.id),
                CoreUnitDef {
                    hp: def.max_hp,
                    mana: def.max_mana,
                    speed_raw: DEFAULT_SPEED_RAW,
                    damage: DEFAULT_DAMAGE,
                    range_raw: DEFAULT_RANGE_RAW,
                    supply: DEFAULT_SUPPLY,
                    train_frames: DEFAULT_TRAIN_FRAMES,
                },
            )
        })
        .collect();
    let abilities: Vec<(DefId, CoreAbilityDef)> = defs
        .abilities
        .iter()
        .map(|def| {
            (
                definition_id(&def.id),
                CoreAbilityDef {
                    mana_cost: def.mana_cost,
                    cooldown_frames: def.cooldown.clamp(0, u32::MAX as i64) as u32,
                    cast_frames: 0,
                    behavior: 0,
                    effects: vec![EffectDef {
                        kind: EffectKind::Damage,
                        amount: DEFAULT_ABILITY_DAMAGE,
                        duration_frames: 0,
                    }],
                },
            )
        })
        .collect();
    let items: Vec<(DefId, CoreItemDef)> = defs
        .items
        .iter()
        .map(|def| {
            (
                definition_id(&def.id),
                CoreItemDef {
                    hp_mod: DEFAULT_ITEM_HP_MOD,
                    damage_mod: DEFAULT_ITEM_DAMAGE_MOD,
                },
            )
        })
        .collect();
    let upgrades: Vec<(DefId, CoreUpgradeDef)> = defs
        .upgrades
        .iter()
        .map(|def| {
            (
                definition_id(&def.id),
                CoreUpgradeDef {
                    damage_mod: DEFAULT_UPGRADE_DAMAGE_MOD,
                    hp_mod: DEFAULT_UPGRADE_HP_MOD,
                    research_frames: def.time_base.clamp(0, u32::MAX as i64) as u32,
                },
            )
        })
        .collect();
    CoreDefinitions::new(units, abilities, items, upgrades).map_err(InjectError::Definition)
}

/// 注入结果：冻结世界、其内容身份、稳定顺序的出生命令与冻结内核定义。
pub struct Injection {
    pub world: StaticWorld,
    /// 取 `world.identity()`；注入定义后取 [`combine_content_hash`] 的结果。
    /// 作为对局 `content_hash` 与恢复校验依据。
    pub content_hash: u64,
    pub spawn_commands: Vec<Command>,
    /// SLK 派生、经 app 层冻结的内核定义；未注入定义时为空集。
    pub definitions: CoreDefinitions,
}

fn owner_to_player(owner: i64) -> u8 {
    owner.clamp(0, u8::MAX as i64) as u8
}

/// 把一张已解析的地图转成可注入的冻结数据。
///
/// - `cell_size` 由 `tileSize` 冻结（缺失取 128），`origin` 由 `centerOffset`
///   冻结（缺失取 0,0）；单位世界坐标原样 1:1 冻结，不平移。
/// - 格子数取 `tilepoint - 1`；每格高度取对应瓦片左上角顶点。
/// - 非正 `tileSize` 在冻结边界显式拒绝，避免 `world_to_cell` 与
///   `cell_center` 的格距口径不自洽。
/// - P2 竖切**不读 `pathing.json`（WPM）**：静态阻挡保持全可走，动态阻挡
///   由帧命令增删（见 `design.md` 的 Open Questions 结论）。
/// - 未注入 SLK 定义时 `definitions` 为空集、`content_hash` 为地图身份。
pub fn inject_map(map: &ParsedMap) -> Result<Injection, InjectError> {
    let heightfield = &map.heightfield;
    heightfield.validate().map_err(InjectError::Heightfield)?;

    let width = heightfield.tilepoint_width.saturating_sub(1).max(1) as u32;
    let height = heightfield.tilepoint_height.saturating_sub(1).max(1) as u32;
    let cell_size = freeze_f64(heightfield.tile_size.unwrap_or(DEFAULT_TILE_SIZE))?;
    if cell_size.raw() <= 0 {
        return Err(InjectError::InvalidCellSize);
    }
    let origin = match heightfield.center_offset {
        Some(offset) => (freeze_f64(offset.x)?, freeze_f64(offset.y)?),
        None => (SimFixed::ZERO, SimFixed::ZERO),
    };

    let cells = (width as usize).saturating_mul(height as usize);
    let mut heights = Vec::with_capacity(cells);
    let mut blocked = Vec::with_capacity(cells);
    for y in 0..height as usize {
        for x in 0..width as usize {
            let vertex = heightfield.height_at(x, y).unwrap_or(0.0);
            heights.push(freeze_f64(vertex)?);
            blocked.push(false);
        }
    }

    let world = StaticWorld::new(width, height, cell_size, origin, blocked, heights);
    let content_hash = world.identity();

    let spawn_commands = map
        .units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            let x = freeze_f64(unit.position.x)?;
            let y = freeze_f64(unit.position.y)?;
            Ok(Command::new(
                Frame::new(0),
                owner_to_player(unit.owner),
                (index + 1) as u32,
                CommandKind::Spawn {
                    x,
                    y,
                    hp: 100,
                    speed: SimFixed::from_int(3).expect("bounded spawn speed"),
                    attack_damage: 10,
                    attack_range: SimFixed::from_int(10).expect("bounded spawn range"),
                },
            ))
        })
        .collect::<Result<Vec<Command>, InjectError>>()?;

    Ok(Injection {
        world,
        content_hash,
        spawn_commands,
        definitions: CoreDefinitions::empty(),
    })
}

/// 在 [`inject_map`] 基础上把 SLK 派生的冻结定义注入对局配置。
///
/// 内容身份由「静态地图身份 + 内核定义内容哈希」组合而成，与
/// `Snapshot.defs_hash` 一起构成恢复校验的双重锚点。
pub fn inject_map_with_definitions(
    map: &ParsedMap,
    defs: &ContentDefinitions,
) -> Result<Injection, InjectError> {
    let mut injection = inject_map(map)?;
    let core_defs = freeze_definitions(defs)?;
    injection.content_hash = combine_content_hash(injection.content_hash, core_defs.content_hash());
    injection.definitions = core_defs;
    Ok(injection)
}

/// 用注入结果创建对局并提交出生命令。出生发生在第 0 帧。
///
/// 冻结定义（含 SLK 派生）经 `create_with_definitions` 注入；定义为空集时
/// 与旧行为一致。
pub fn create_match(injection: &Injection, seed: u64) -> RtsMatch {
    let config = MatchConfig {
        seed,
        logic_fps: 30,
        content_hash: injection.content_hash,
    };
    let mut sim = RtsMatch::create_with_definitions(
        config,
        injection.world.clone(),
        injection.definitions.clone(),
    );
    for command in &injection.spawn_commands {
        sim.submit_command(*command)
            .expect("validated spawn command");
    }
    sim
}
