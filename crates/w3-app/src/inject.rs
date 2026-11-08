//! 冻结地图数据注入 `w3-core`（见 ADR 0004 与 `design.md`）。
//!
//! `ParsedMap` 的原始 `f64` 在边界经 [`freeze_f64`] 转成 `SimFixed`，
//! 再组装成中立的 `StaticWorld` 与出生命令。`w3-core` 不读取文件、
//! 不反向依赖内容层。

use std::fmt;

use w3_content::{HeightfieldError, ParsedMap};
use w3_core::{Command, CommandKind, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld};

use crate::freeze::{FrozenError, freeze_f64};

/// 上游 `MAP_DATA.md` 给出的 `tileSize` 默认值（缺失时使用）。
const DEFAULT_TILE_SIZE: f64 = 128.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InjectError {
    Frozen(FrozenError),
    Heightfield(HeightfieldError),
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
            Self::InvalidCellSize => f.write_str("tileSize must be positive"),
        }
    }
}

impl std::error::Error for InjectError {}

/// 注入结果：冻结世界、其内容身份与稳定顺序的出生命令。
pub struct Injection {
    pub world: StaticWorld,
    /// 取 `world.identity()`，作为对局 `content_hash` 与恢复校验依据。
    pub content_hash: u64,
    pub spawn_commands: Vec<Command>,
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
    })
}

/// 用注入结果创建对局并提交出生命令。出生发生在第 0 帧。
pub fn create_match(injection: &Injection, seed: u64) -> RtsMatch {
    let config = MatchConfig {
        seed,
        logic_fps: 30,
        content_hash: injection.content_hash,
    };
    let mut sim = RtsMatch::create_with_world(config, injection.world.clone());
    for command in &injection.spawn_commands {
        sim.submit_command(*command)
            .expect("validated spawn command");
    }
    sim
}
