//! 确定性、与引擎无关的 RTS 模拟内核（P0 骨架 + P2 竖切）。
//!
//! 本 crate 独占权威模拟状态，不依赖渲染、UI、音频、网络、文件加载或墙钟时间。
//!
//! 确定性契约（见 `docs/decisions/0002-determinism.md`）：
//! - 所有权威状态数学只使用 [`fixed::SimFixed`] 整数运算。
//! - 对局按整数逻辑帧推进，调用方不能传入 delta。
//! - 命令按帧冻结后以稳定顺序执行。
//! - 状态哈希使用显式字节编码与稳定 ID 顺序。
//! - 随机流由对局独占，并写入每一份快照。
//!
//! P2 追加（见 `docs/decisions/0004-frozen-coordinates.md`）：冻结的静态
//! 世界由 app 层注入；朝向 / 坡度 / 寻路全部使用整数数学，不引入浮点
//! 超越函数或外部依赖。
//!
//! 公共入口对应内核契约：`create`、`submit_command`、`step`、`read_view`、
//! `drain_events`、`capture_snapshot`、`restore_snapshot`、
//! `compute_state_hash`、`first_difference`。

pub mod angle;
pub mod command;
pub mod entity;
pub mod fixed;
pub mod frame;
pub mod gameplay;
pub mod nav;
pub mod sim_match;
pub mod snapshot;
pub mod state_hash;
pub mod world;

pub use angle::{ANGLE_SCALE, Angle, slope_speed_scale};
pub use command::{Command, CommandError, CommandKind, CommandQueue};
pub use entity::{EntityId, EntityStore};
pub use fixed::{FRACTIONAL_BITS, FixedError, SCALE, SimFixed, isqrt};
pub use frame::{Frame, FrameClock};
pub use gameplay::{
    AbilityDef, AiState, AuraState, BehaviorSpec, BuffInstance, CONTENT_HASH_SCHEMA, CastRuntime,
    DefId, DefinitionError, EffectDef, EffectKind, EffectOutcome, FrozenDefinitions,
    GameplayCommand, GameplayCommandError, GameplayCommandKind, GameplayCommandQueue, HeroState,
    ItemDef, PlayerEconomy, ProductionItem, StatMods, UnitDef, UnitGameplayState, UpgradeDef,
    VictoryState, WaveState, WorldGameplayState, resolve_effects,
};
pub use nav::{DEFAULT_NODE_BUDGET, DynamicObstacles, GridArea};
pub use sim_match::{EntityView, Event, EventKind, MatchConfig, RtsMatch, WorldView};
pub use snapshot::{
    GroupPlanRecord, ObstacleRecord, OrderRecord, PositionRecord, SIM_VERSION, SNAPSHOT_VERSION,
    Snapshot, SnapshotError,
};
pub use state_hash::StateHasher;
pub use world::{GridCell, StaticWorld};
