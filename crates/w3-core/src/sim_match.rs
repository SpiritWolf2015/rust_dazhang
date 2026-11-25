//! 权威对局状态的唯一所有者。
//!
//! `RtsMatch` 是唯一写入者；所有公开读取都返回所有权拷贝，
//! 绝不暴露可变的内部集合。`step()` 恰好推进一个逻辑帧，不接受 delta。
//!
//! P2 竖切：订单、寻路、动态占地与群体规划全部由本模块独占，并同时进入
//! 状态哈希、快照与 `first_difference()`（见 ADR 0002 / 0004）。

use std::collections::{BTreeMap, BTreeSet};

use crate::ability_system::{
    AbilityCategory, AbilityRngDraw, AbilitySystemState, AbilityTargetKind, AuraBeneficiarySet,
    BehaviorKind, BuffCategory, ChainState, PassiveProcState, PendingAbilityCast, PointTargetState,
    SummonState, TransformState, ZoneState,
};
use crate::angle::{Angle, slope_speed_scale};
use crate::command::{Command, CommandError, CommandKind, CommandQueue};
use crate::creep::{
    CREEP_RNG_SALT, CampState, CreepCampState, CreepDeathNotice, CreepState, GROUND_ITEM_ID_BASE,
    GroundItemState, ItemInstanceState, KillReward, LeashPhase, NEUTRAL_OWNER, NeutralCategory,
    RngDraw,
};
use crate::entity::{EntityId, EntityStore};
use crate::faction_mechanics::{
    FactionFactKind, FactionMechanicsState, MineKind, MineRecord, RegenCondition, RegenRecord,
};
use crate::fixed::{SCALE, SimFixed, isqrt};
use crate::frame::{Frame, FrameClock};
use crate::gameplay::{
    AuraState, BehaviorSpec, CastRuntime, DefId, EffectDef, EffectKind, EffectOutcome,
    FrozenDefinitions, GameplayCommand, GameplayCommandError, GameplayCommandKind, HeroDeathRecord,
    HeroReviveState, HeroState, ProductionItem, ReviveCharge, ReviveQueueItem, Settlement,
    TownHallTier, TownHallUpgradeCharge, TownHallUpgradeItem, TownHallUpgradeState,
    UnitGameplayState, VictoryState, Vitals, WaveState, WorldGameplayState, resolve_effects,
    restore_by_percent, validate_gameplay_entities,
};
use crate::item_shop::{
    ItemBag, ItemCategory, ItemShopFact, ItemShopFactKind, ItemShopState, PurchaseCharge,
    ShopRefreshDraw, TownPortalCast, shop_fixture_id,
};
use crate::nav::{self, DEFAULT_NODE_BUDGET, DynamicObstacles, GridArea};
use crate::snapshot::{
    GroupPlanRecord, ObstacleRecord, OrderRecord, PositionRecord, SIM_VERSION, SNAPSHOT_VERSION,
    Snapshot, SnapshotError,
};
use crate::state_hash::StateHasher;
use crate::world::StaticWorld;

/// 每帧群体规划推进的槽位预算（固定工作量，见 `pathfinding` spec）。
const PLAN_BUDGET: usize = 64;
/// 连续拥堵判失败的帧数（30 Hz 下 3 秒）。
const CONGESTION_TIMEOUT_FRAMES: u32 = 90;
/// 每单位待执行订单上限。
const MAX_PENDING_ORDERS: usize = 64;
/// 出生单位的冻结初始法力（`Spawn` 命令不携带法力，见 P2 内联初值口径）。
const DEFAULT_UNIT_MANA: i64 = 100;
/// 默认每帧转向增量（1/16 圈）。
const DEFAULT_TURN_RATE: i64 = 1 << 12;
/// 英雄出生等级（合成原创编译期整数常量 `[待核]`）。
const HERO_SPAWN_LEVEL: u32 = 1;
/// 英雄出生经验（合成原创编译期整数常量 `[待核]`）。
const HERO_SPAWN_XP: i64 = 0;

/// 群体落点的稳定偏移表，按 `EntityId` 排序后依次分配。
const FORMATION_OFFSETS: [(i64, i64); 16] = [
    (0, 0),
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
    (2, 0),
    (0, 2),
    (-2, 0),
    (0, -2),
    (2, 2),
    (-2, 2),
    (2, -2),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MatchConfig {
    pub seed: u64,
    pub logic_fps: u32,
    pub content_hash: u64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            logic_fps: 30,
            content_hash: 0,
        }
    }
}

/// 单个玩家的权威意图。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Order {
    Idle,
    Stop,
    Move {
        x: SimFixed,
        y: SimFixed,
        group: u32,
    },
    Attack {
        target: EntityId,
    },
}

/// 单位的全部权威状态，由 `RtsMatch` 独占写入。
#[derive(Clone, PartialEq, Eq, Debug)]
struct UnitState {
    x: SimFixed,
    y: SimFixed,
    facing: Angle,
    speed: SimFixed,
    hp: i64,
    max_hp: i64,
    owner: u8,
    attack_damage: i64,
    attack_range: SimFixed,
    turn_rate: i64,
    alive: bool,
    order: Order,
    pending: Vec<Order>,
    goal: Option<(SimFixed, SimFixed)>,
    path: Vec<(SimFixed, SimFixed)>,
    path_index: usize,
    blocked_frames: u32,
    group: u32,
    /// P5 玩法运行态（法力 / Buff / 施法 / 冷却 / 英雄 / 背包 / 生产队列）。
    gameplay: UnitGameplayState,
}

impl UnitState {
    fn new(
        x: SimFixed,
        y: SimFixed,
        owner: u8,
        hp: i64,
        speed: SimFixed,
        attack_damage: i64,
        attack_range: SimFixed,
    ) -> Self {
        Self {
            x,
            y,
            facing: Angle::ZERO,
            speed,
            hp,
            max_hp: hp,
            owner,
            attack_damage,
            attack_range,
            turn_rate: DEFAULT_TURN_RATE,
            alive: true,
            order: Order::Idle,
            pending: Vec::new(),
            goal: None,
            path: Vec::new(),
            path_index: 0,
            blocked_frames: 0,
            group: 0,
            gameplay: UnitGameplayState::default(),
        }
    }
}

/// 固定工作量分帧规划的作业状态。
#[derive(Clone, PartialEq, Eq, Debug)]
struct GroupPlan {
    goal: (SimFixed, SimFixed),
    members: Vec<EntityId>,
    slots: Vec<(SimFixed, SimFixed)>,
    progress: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Spawned,
    Arrived,
    Stopped,
    Died,
    AttackHit,
    MoveFailed,
    /// P8.5a 英雄阵亡事实事件（供只读表现层消费，不进权威状态）。
    HeroDied,
    /// P8.5a 英雄复活完成事实事件。
    Revived,
    /// P8.5b 主城升级开始事实事件（已扣费、队列项建立，供只读表现层消费）。
    TownHallUpgradeStarted,
    /// P8.5b 主城升级完成事实事件（tier 递进、解锁立即生效）。
    TownHallUpgradeCompleted,
    /// P8.5b 主城升级取消 / 中止事实事件（事务性退款，供只读表现层消费）。
    TownHallUpgradeCancelled,
    /// P8.5c 野怪营地按冻结脚本初始化完成（frame-0 系统级命令的事实事件）。
    /// `Event.entity` 携带营地实例 id；野怪实体逐个发既有 `Spawned`。
    CreepCampSeeded,
    /// P8.5c 野怪进入 ENGAGED 追击（供只读表现层消费，不进权威状态）。
    CreepEngaged,
    /// P8.5c 野怪切 RETURNING 归巢（leash 越界，MUST NOT 无限追出营地）。
    CreepLeashReturning,
    /// P8.5c 野怪归巢到 home 阈值内恢复 IDLE（R8.5）。
    CreepLeashReturned,
    /// P8.5c 野怪被击杀（共享结算服务的幂等死亡通知，Q6 / R8.3 / R8.6）。
    CreepDied,
    /// P8.5c 地面物品掉落事实事件（无主，`Event.entity` 为地面物品 id）。
    ItemDropped,
    /// P8.5d 拾取地面物品事实事件（`Event.entity` 为英雄 id，按只读
    /// `Snapshot.item_shop.facts` / `Snapshot.item_shop.bags` 查表）。
    ItemPickedUp,
    /// P8.5d 商店购买事实事件（精确扣费 / 建实例 / 库存递减）。
    ItemPurchased,
    /// P8.5d 使用道具事实事件（充能 / 冷却 / 既有 `EffectDef` 组合）。
    ItemUsed,
    /// P8.5d 丢弃道具回地面事实事件（保持实例身份与剩余冷却）。
    ItemDiscarded,
    /// P8.5d 两英雄间移交道具事实事件。
    ItemTransferred,
    /// P8.5d 售卖道具事实事件（精确入账冻结回购价）。
    ItemSold,
    /// P8.5d 回城施法开始事实事件（目标按友方主城分类数据驱动）。
    TownPortalStarted,
    /// P8.5d 回城传送完成事实事件（单英雄整数坐标传送）。
    TownPortalCompleted,
    /// P8.5d 回城施法中断 / 拒单事实事件（不产生半传送位置）。
    TownPortalCancelled,
    /// P8.5d 市场刷新事实事件（幂等，复用 P8.5c 事件驱动随机流）。
    MarketRefreshed,
    /// P8.5d 命令被显式拒绝事实事件（满包 / 售罄 / 黄金不足 / 非法目标 /
    /// 冷却未到 / 无友方主城 / 旧 `BuyItem` 占位旁路被拒）。
    ItemShopCommandRejected,
    // -----------------------------------------------------------------------
    // P9 / Q2 / Q4 族特色机制只读事实事件（`Event` 结构不变、无 payload）。
    // 表现层由 `Event.entity` 查只读 `Snapshot.faction_mechanics` 得到
    // 全部事实；历史事件 MUST NOT 进权威哈希 / 快照（ADR 0002 §5-6）。
    // fog 可见性 MUST NOT 由任何事实事件反写权威状态（R10.5 / §0.3 T3）。
    // -----------------------------------------------------------------------
    /// 腐地扩张源与初始格登记（R4.6）。
    BlightSeeded,
    /// 尸体登记（`:119`）。
    CorpseRaised,
    /// 食尸回血 / 召唤骷髅（`:119`）。
    CorpseConsumed,
    /// 诅咒金矿绑定（R1.5）。
    GoldMineHaunted,
    /// 缠绕金矿绑定（R1.5）。
    GoldMineEntangled,
    /// 古树扎根 / 拔根状态切换。
    AncientRootChanged,
    /// 月亮井登记（R10.3）。
    MoonWellBound,
    /// 隐身目标判定登记（R10.4 / S3，玩法机制，非 fog 可见性）。
    StealthTargetChanged,
    /// 反隐单位登记（R10.4）。
    DetectorRegistered,
    /// 夜间隐身候选登记（S3）。
    NightCloakChanged,
    /// 视线阻挡机制输入登记（R5.6，非 fog 可见性派生结果）。
    SightBlockerChanged,
    /// 野怪睡眠登记（R8.4）。
    CreepSleepRegistered,
    /// 回血条件定义层枚举登记（R10.3）。
    RegenConditionChanged,
    /// 昼夜相位切换（R10.2 逐 tick 结算事实）。
    DayNightPhaseChanged,
    /// 隐身目标被反隐揭示（R10.4 逐 tick 结算事实，玩法判定）。
    StealthRevealed,
    /// 腐地扩张一格（逐 tick 结算事实）。
    BlightExpanded,
    /// 尸体衰减完成移除（逐 tick 结算事实）。
    CorpseDecayed,
    /// 金矿完成一次采集结算（逐 tick 结算事实）。
    MineSettled,
    /// 回血条件首次生效（逐 tick 结算事实）。
    RegenApplied,
    /// 野怪睡眠状态切换（逐 tick 结算事实）。
    CreepSleepChanged,
    /// 夜间隐身生效 / 解除（逐 tick 结算事实）。
    NightCloakApplied,
    /// 族特色机制命令被显式拒绝（非法实体 / 未登记尸体等，显式失败不静默降级）。
    FactionMechanicRejected,
}

/// 只读事实事件（`drain_events()` 返回 owned 拷贝，序号不重置）。
///
/// P8.5c 不新增 payload 字段：按 D6「扩展只读事实事件」，新增
/// `EventKind` 变体即可；`Event.entity` 携带营地 id / 野怪 `EntityId` /
/// 地面物品 id，表现层由 owned `Snapshot.creep_camps` 与
/// [`RtsMatch::creep_camps`] 只读投影查表得到「掉什么 / 给谁经验」
/// 等全部事实（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` 范式）。
/// P8.5d 同样不新增 payload：拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖 /
/// 回城 / 刷新事实由 `Event.entity` 查只读 `Snapshot.item_shop.facts` /
/// `Snapshot.item_shop.bags` / `Snapshot.item_shop.stock` 得到（Q5 / D6）。
/// 历史事件不进入状态哈希 / 快照 / `first_difference()`（ADR 0002 §5-6）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Event {
    pub frame: u64,
    pub sequence: u64,
    pub entity: u64,
    pub kind: EventKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EntityView {
    pub id: u64,
    pub x: i64,
    pub y: i64,
    pub facing: i64,
    /// 投影自 `UnitState.speed` 的只读视图字段，供表现层 R1.4 分层判据；
    /// 不进入权威状态 / 状态哈希 / 快照。
    pub speed: i64,
    pub hp: i64,
    pub owner: u8,
    pub alive: bool,
    pub has_target: bool,
}

/// 具备所有权的只读世界视图，修改它不会影响模拟。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WorldView {
    pub tick: u64,
    pub rng: u64,
    pub entities: Vec<EntityView>,
}

pub struct RtsMatch {
    config: MatchConfig,
    clock: FrameClock,
    entities: EntityStore,
    units: BTreeMap<EntityId, UnitState>,
    commands: CommandQueue,
    world: StaticWorld,
    obstacles: DynamicObstacles,
    group_plans: BTreeMap<u32, GroupPlan>,
    rng: u64,
    events: Vec<Event>,
    next_event_seq: u64,
    /// 冻结内容定义（创建后无可变 API，拒绝运行期热更新）。
    defs: FrozenDefinitions,
    /// 全局玩法运行态（经济 / 胜负 / AI / 光环 / 波次 / 行为注册 / 玩法命令）。
    gameplay: WorldGameplayState,
    /// P8.5a 英雄复活三组权威状态（死亡登记 / 复活队列 / 费用登记）。
    /// 字段私有，仅本结构可写；同时进入状态哈希、快照与差异诊断。
    revive: HeroReviveState,
    /// P8.5b 主城升级三组权威状态（等级 / 升级队列 / 费用登记）。
    /// 独立字段，MUST NOT 复用 `PlayerEconomy.upgrades` 布尔旗标；
    /// 同时进入状态哈希、快照与差异诊断（ADR 0002 §6-8 / C5）。
    town_hall: TownHallUpgradeState,
    /// P8.5c 野怪营地与掉落全部权威状态（Q1 / D1 / D5）。
    /// 字段私有，仅本结构可写；全部落 `BTreeMap` / `BTreeSet`
    /// 确定性有序容器，同时进入状态哈希、快照与差异诊断。
    creep_camps: CreepCampState,
    /// P8.5d 物品 / 商店 / 回城卷轴全部权威状态（Q6 / Q7 / D1 / D5）。
    /// 字段私有，仅本结构可写；六格定长实例背包与 `creep_camps`
    /// 的 `item_instances` 按稳定 `EntityId` 构成单一实例身份口径。
    /// 同时进入状态哈希、由 `SNAPSHOT_VERSION`（当前 9）定义的快照与差异诊断。
    item_shop: ItemShopState,
    /// P9 / Q2 / Q4 族特色机制顶层权威状态（腐地 / 尸体 / 金矿 / 古树 /
    /// 昼夜相位 / 隐身目标判定 / 月亮井 / 缠绕金矿 / 夜间隐身 + Q4 全量）。
    /// 字段私有，仅本结构可写；全部落 `BTreeMap` / `BTreeSet`
    /// 确定性有序容器、按稳定 `EntityId` 顺序遍历；四处同步进 FNV-1a
    /// 状态哈希、由 `SNAPSHOT_VERSION`（9）定义的 `Snapshot` 顶层字段、
    /// `first_difference()` 与 `restore_snapshot`（ADR 0002 §6-8）。
    /// 迷雾 / 视野可见性 MUST 表现层只读派生，MUST NOT 进本字段 /
    /// 哈希 / 快照 / `first_difference()`（R10.5 / §0.3 T3 / Q2'）。
    faction_mechanics: FactionMechanicsState,
}

/// 技能效果结算上下文：全部为冻结整数输入，不携带闭包或宿主随机源。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct AbilityEffectContext {
    caster: EntityId,
    ability: DefId,
    target: AbilityTargetKind,
    player: u8,
    frame: u64,
}

/// 纯整数 LCG：相同状态总是产生相同的后继状态。
fn next_rng(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// 从本帧唯一 LCG 后继状态派生逻辑随机码；不推进主 LCG、不新增独立流。
fn mix_ability_rng(state: u64, salt: u64, seq: u64, ability: DefId) -> u64 {
    let mixed = state
        .wrapping_add(salt)
        .wrapping_add((u64::from(ability) << 32) ^ seq.rotate_left(17));
    mixed ^ (mixed >> 33)
}

/// P9 / Q4 把逐 tick 结算事实类别映射为只读 `EventKind` 事实事件。
/// 穷尽匹配，新增事实类别 MUST 同步登记事件（无遗漏默认放行）；
/// 事件无 payload，表现层由 `Event.entity` 查只读
/// `Snapshot.faction_mechanics.facts` 得到全部细节（ADR 0002 §5）。
fn fact_event_kind(kind: FactionFactKind) -> EventKind {
    match kind {
        FactionFactKind::BlightExpanded => EventKind::BlightExpanded,
        FactionFactKind::CorpseDecayed => EventKind::CorpseDecayed,
        FactionFactKind::MineSettled => EventKind::MineSettled,
        FactionFactKind::PhaseChanged => EventKind::DayNightPhaseChanged,
        FactionFactKind::StealthRevealed => EventKind::StealthRevealed,
        FactionFactKind::NightCloakApplied => EventKind::NightCloakApplied,
        FactionFactKind::RegenApplied => EventKind::RegenApplied,
        FactionFactKind::CreepSleepChanged => EventKind::CreepSleepChanged,
    }
}

fn formation_slot(
    index: usize,
    goal: (SimFixed, SimFixed),
    cell_size: SimFixed,
) -> (SimFixed, SimFixed) {
    let ring = (index / FORMATION_OFFSETS.len()) as i64 + 1;
    let (ox, oy) = FORMATION_OFFSETS[index % FORMATION_OFFSETS.len()];
    let spacing = cell_size.raw().saturating_mul(2).saturating_mul(ring);
    let x = goal.0.raw().saturating_add(ox.saturating_mul(spacing));
    let y = goal.1.raw().saturating_add(oy.saturating_mul(spacing));
    (SimFixed::from_raw(x), SimFixed::from_raw(y))
}

fn order_record(order: Order) -> OrderRecord {
    match order {
        Order::Idle => OrderRecord::Idle,
        Order::Stop => OrderRecord::Stop,
        Order::Move { x, y, group } => OrderRecord::Move {
            x: x.raw(),
            y: y.raw(),
            group,
        },
        Order::Attack { target } => OrderRecord::Attack {
            target: target.value(),
        },
    }
}

fn order_from_record(record: OrderRecord) -> Order {
    match record {
        OrderRecord::Idle => Order::Idle,
        OrderRecord::Stop => Order::Stop,
        OrderRecord::Move { x, y, group } => Order::Move {
            x: SimFixed::from_raw(x),
            y: SimFixed::from_raw(y),
            group,
        },
        OrderRecord::Attack { target } => Order::Attack {
            target: EntityId::new(target).unwrap_or(EntityId::NONE),
        },
    }
}

fn hash_command(hasher: &mut StateHasher, command: &Command) {
    hasher.write_u64(command.execute_frame.value());
    hasher.write_u8(command.player_id);
    hasher.write_u32(command.sequence);
    hasher.write_u64(command.arrival());
    match command.kind {
        CommandKind::Noop => hasher.write_u8(0),
        CommandKind::Spawn {
            x,
            y,
            hp,
            speed,
            attack_damage,
            attack_range,
        } => {
            hasher.write_u8(1);
            hasher.write_i64(x.raw());
            hasher.write_i64(y.raw());
            hasher.write_i64(hp);
            hasher.write_i64(speed.raw());
            hasher.write_i64(attack_damage);
            hasher.write_i64(attack_range.raw());
        }
        CommandKind::Move {
            entity,
            x,
            y,
            group,
            append,
        } => {
            hasher.write_u8(2);
            hasher.write_u64(entity.value());
            hasher.write_i64(x.raw());
            hasher.write_i64(y.raw());
            hasher.write_u32(group);
            hasher.write_u8(u8::from(append));
        }
        CommandKind::Stop { entity } => {
            hasher.write_u8(3);
            hasher.write_u64(entity.value());
        }
        CommandKind::Attack { attacker, target } => {
            hasher.write_u8(4);
            hasher.write_u64(attacker.value());
            hasher.write_u64(target.value());
        }
        CommandKind::SetObstacle { id, x, y, w, h } => {
            hasher.write_u8(5);
            hasher.write_u64(id);
            hasher.write_i64(x as i64);
            hasher.write_i64(y as i64);
            hasher.write_i64(w as i64);
            hasher.write_i64(h as i64);
        }
        CommandKind::RemoveObstacle { id } => {
            hasher.write_u8(6);
            hasher.write_u64(id);
        }
    }
}

fn hash_order(hasher: &mut StateHasher, order: &Order) {
    match order {
        Order::Idle => hasher.write_u8(0),
        Order::Stop => hasher.write_u8(1),
        Order::Move { x, y, group } => {
            hasher.write_u8(2);
            hasher.write_i64(x.raw());
            hasher.write_i64(y.raw());
            hasher.write_u32(*group);
        }
        Order::Attack { target } => {
            hasher.write_u8(3);
            hasher.write_u64(target.value());
        }
    }
}

fn hash_unit(hasher: &mut StateHasher, unit: &UnitState) {
    hasher.write_i64(unit.x.raw());
    hasher.write_i64(unit.y.raw());
    hasher.write_i64(unit.facing.raw());
    hasher.write_i64(unit.speed.raw());
    hasher.write_i64(unit.hp);
    hasher.write_i64(unit.max_hp);
    hasher.write_u8(unit.owner);
    hasher.write_i64(unit.attack_damage);
    hasher.write_i64(unit.attack_range.raw());
    hasher.write_i64(unit.turn_rate);
    hasher.write_u8(u8::from(unit.alive));
    hash_order(hasher, &unit.order);
    hasher.write_u64(unit.pending.len() as u64);
    for order in &unit.pending {
        hash_order(hasher, order);
    }
    match unit.goal {
        Some((x, y)) => {
            hasher.write_u8(1);
            hasher.write_i64(x.raw());
            hasher.write_i64(y.raw());
        }
        None => hasher.write_u8(0),
    }
    hasher.write_u64(unit.path.len() as u64);
    for &(x, y) in &unit.path {
        hasher.write_i64(x.raw());
        hasher.write_i64(y.raw());
    }
    hasher.write_u64(unit.path_index as u64);
    hasher.write_u32(unit.blocked_frames);
    hasher.write_u32(unit.group);
    unit.gameplay.hash(hasher);
}

impl RtsMatch {
    /// 用默认开放世界创建对局（无导航地图的轻量入口）。
    pub fn create(config: MatchConfig) -> Self {
        Self::create_with_world(config, StaticWorld::default())
    }

    /// 由 app 层注入冻结地图数据；`w3-core` 不读取文件、不依赖内容层。
    pub fn create_with_world(config: MatchConfig, world: StaticWorld) -> Self {
        // P8.5c：事件驱动掉落流由 `seed` 经同一整数 LCG 域分隔派生（Q6）。
        // 主流 `rng` 仍每帧恰好消耗一次（ADR 0002 §4），掉落流只在死亡
        // 事件按稳定 `EntityId` 顺序消费，流状态随快照恢复。
        let mut creep_camps = CreepCampState::default();
        creep_camps.rng_state = next_rng(config.seed ^ CREEP_RNG_SALT);
        creep_camps.next_ground_item_id = GROUND_ITEM_ID_BASE;
        Self {
            config,
            clock: FrameClock::new(config.logic_fps),
            entities: EntityStore::new(),
            units: BTreeMap::new(),
            commands: CommandQueue::new(),
            world,
            obstacles: BTreeMap::new(),
            group_plans: BTreeMap::new(),
            rng: config.seed,
            events: Vec::new(),
            next_event_seq: 0,
            defs: FrozenDefinitions::empty(),
            gameplay: WorldGameplayState::default(),
            revive: HeroReviveState::default(),
            town_hall: TownHallUpgradeState::default(),
            creep_camps,
            item_shop: ItemShopState::default(),
            faction_mechanics: FactionMechanicsState::default(),
        }
    }

    /// 注入冻结内容定义；创建后不提供任何运行期热更新入口。
    pub fn create_with_definitions(
        config: MatchConfig,
        world: StaticWorld,
        defs: FrozenDefinitions,
    ) -> Self {
        let mut sim = Self::create_with_world(config, world);
        // P8.5d：商店库存初值来自冻结货架清单（权威状态初值，非定义层本身）。
        let item_shop_defs = defs.item_shop().clone();
        sim.item_shop.initialize_stock(&item_shop_defs);
        sim.defs = defs;
        sim
    }

    /// 冻结定义的只读视图。
    pub fn definitions(&self) -> &FrozenDefinitions {
        &self.defs
    }

    /// P8.5d 物品 / 商店 / 回城权威状态的只读 owned 拷贝（只读投影）。
    pub fn item_shop_state(&self) -> ItemShopState {
        self.item_shop.clone()
    }

    /// P9 / Q2 / Q4 族特色机制权威状态的只读 owned 拷贝（只读投影）。
    /// 表现层只能消费 owned 投影，MUST NOT 反写权威状态（ADR 0002 §5）；
    /// fog 可见性 MUST 由表现层只读派生，MUST NOT 进本投影
    /// （R10.5 / §0.3 T3）。
    pub fn faction_mechanics_state(&self) -> FactionMechanicsState {
        self.faction_mechanics.clone()
    }

    /// P11 技能系统权威运行态的只读 owned 拷贝（只读投影）。
    /// 表现层只能消费 owned 投影，MUST NOT 反写权威状态（ADR 0002 §5）。
    pub fn ability_system_state(&self) -> AbilitySystemState {
        self.gameplay.abilities.clone()
    }

    pub fn config(&self) -> MatchConfig {
        self.config
    }

    pub fn submit_command(&mut self, command: Command) -> Result<u64, CommandError> {
        if command.execute_frame.value() < self.clock.current().value() {
            return Err(CommandError::FrameInPast);
        }
        self.commands.submit(command)
    }

    /// 玩家与 AI 命令共用的校验入口（帧窗口 + 实体合法性）。
    fn validate_gameplay_command(
        &self,
        command: &GameplayCommand,
    ) -> Result<(), GameplayCommandError> {
        if command.execute_frame.value() < self.clock.current().value() {
            return Err(GameplayCommandError::FrameInPast);
        }
        validate_gameplay_entities(&command.kind)
    }

    /// 玩家与 AI 共用的玩法命令提交入口（与 `submit_command` 同源校验）。
    pub fn submit_gameplay_command(
        &mut self,
        command: GameplayCommand,
    ) -> Result<u64, GameplayCommandError> {
        self.validate_gameplay_command(&command)?;
        self.gameplay.commands.submit(command)
    }

    /// 完整推进一个逻辑帧；不接受调用方传入的 delta，也不读取墙钟。
    pub fn step(&mut self) {
        let frame = self.clock.current();

        // 1. 冻结本帧命令，按稳定顺序执行。
        let due = self.commands.freeze_frame(frame);
        let mut attacks = Vec::new();
        let mut group_requests: BTreeMap<u32, Vec<(EntityId, SimFixed, SimFixed)>> =
            BTreeMap::new();
        let mut obstacles_changed = false;
        for command in due {
            self.apply(
                command,
                frame,
                &mut attacks,
                &mut group_requests,
                &mut obstacles_changed,
            );
        }

        // 2. 同帧群体移动聚类，按稳定 ID 排序后进入规划队列。
        for (group, mut members) in group_requests {
            members.sort_by_key(|(id, _, _)| *id);
            self.enqueue_group(group, members);
        }

        // 3. 动态占地变化 → 在途单位确定性重规划。
        if obstacles_changed {
            self.replan_all();
        }

        // 4. 群体规划按固定工作量推进，每帧最多提交一个作业。
        self.advance_group_plans();

        // 5. 移动积分（转向 / 坡度 / 到达 / 拥堵）。
        self.advance_motions(frame);

        // 5b. P8.5c 野怪 leash / 归巢 AI：复用既有 `Attack` / `Move` 命令面与
        //     `run_ai` 确定性重算框架，内部命令不写回队列、不注入重放，
        //     产出的攻击对汇入本帧同一结算序列（R8.5 / D3）。
        self.run_creep_ai(frame, &mut attacks);

        // 6. 同帧攻击结算，按稳定顺序、带存活复核。
        self.resolve_attacks(frame, &attacks);

        // 6b. P8.5c 共享结算服务：按稳定 `EntityId` 顺序只结算一次击杀收益
        //     与掉落（R6.1 / R6.5 / R8.3 / R7.2 / R8.6）。
        self.settle_creep_deaths(frame);

        // 7. 冻结本帧玩法命令并执行（费用事务提交 + 效果链，统一结算）。
        let due_gameplay = self.gameplay.commands.freeze_frame(frame);
        for command in due_gameplay {
            self.apply_gameplay(command, frame);
        }

        // 8. 推进玩法运行态（施法 / 引导、冷却、Buff 到期、光环 / 波次、生产 / 研究）。
        self.advance_gameplay_timers(frame);

        // 8b. 推进祭坛复活队列效果链（费用提交事务在阶段 7 已完成，此处仅执行
        //     剩余帧递减 / 事务性退款 / 恢复本体，两者分离见主 gameplay-systems/spec.md:19）。
        self.advance_revive_queue(frame);

        // 8c. 推进主城升级队列效果链（费用提交事务同样在阶段 7 完成，此处仅执行
        //     剩余帧递减 / 事务性退款 / tier 递进，两者分离见主 gameplay-systems/spec.md:19）。
        self.advance_town_hall_queue(frame);

        // 8d. P8.5c 地面物品实例冷却逐帧推进（R7.5 独立状态，位置不变）。
        //     P8.5d 起持有实例的共享冷却按同一整数帧推进（R7.5）。
        self.advance_ground_items();

        // 8e. P8.5d 物品 / 商店 / 回城卷轴帧阶段：购买事务提交 / 回城施法 /
        //     市场刷新，按稳定顺序执行；费用提交事务在阶段 7 已完成（D2 / D4）。
        self.advance_item_shop(frame);

        // 8f. P9 / Q4 族特色机制帧阶段：按 30 Hz 整数帧逐 tick 查表结算
        //     昼夜循环全量 / 腐地扩张 / 尸体衰减 / 金矿采集 / R10.3 回血 /
        //     隐身反隐 / 夜间隐身 / 野怪睡眠（Q2 / Q4 / ADR 0002 §2-3）。
        //     结算由冻结常量 + 整数帧 + 稳定 EntityId 顺序确定性派生，
        //     无墙钟 / 无 f64 / 无 HashMap 迭代顺序 / 无表现层独立随机源。
        //     fog 可见性 MUST 表现层只读派生（R10.5 / §0.3 T3），本阶段
        //     MUST NOT 把 fog 可见性反写权威状态。
        self.advance_faction_mechanics(frame);

        // 8g. P11 技能系统帧阶段：先取本帧唯一主 LCG 后继状态，供 zone /
        //     召唤 / 光环 / proc / 变身 / 链接 / `rng_draws` 按冻结常量
        //     与稳定 EntityId 顺序结算。此处不推进 `self.rng`，阶段 11
        //     才提交唯一一次主 LCG 消费；技能随机只从该后继状态派生，
        //     MUST NOT 新增 per-skill 独立流（ADR 0002 §4）。
        let rng_before = self.rng;
        let rng_after = next_rng(rng_before);
        self.advance_ability_system(frame, rng_before, rng_after);

        // 9. 胜负判定：既定帧阶段、稳定顺序、只结算一次。
        self.resolve_victory(frame);

        // 10. 确定性 AI：每帧重算，内部命令不写回命令队列、不注入重放。
        self.run_ai(frame);

        // 11. 每帧恰好消费一次随机流；该流属于权威状态（主 LCG 主流，
        //     与事件驱动掉落流 `creep_camps.rng_state` 分离，ADR 0002 §4）。
        self.rng = rng_after;

        // 12. 提交本帧。
        self.clock.advance();
    }

    fn apply(
        &mut self,
        command: Command,
        frame: Frame,
        attacks: &mut Vec<(EntityId, EntityId)>,
        group_requests: &mut BTreeMap<u32, Vec<(EntityId, SimFixed, SimFixed)>>,
        obstacles_changed: &mut bool,
    ) {
        match command.kind {
            CommandKind::Noop => {}
            CommandKind::Spawn {
                x,
                y,
                hp,
                speed,
                attack_damage,
                attack_range,
            } => {
                let id = self.entities.spawn();
                let mut unit = UnitState::new(
                    x,
                    y,
                    command.player_id,
                    hp,
                    speed,
                    attack_damage,
                    attack_range,
                );
                unit.gameplay.mana = DEFAULT_UNIT_MANA;
                unit.gameplay.max_mana = DEFAULT_UNIT_MANA;
                self.units.insert(id, unit);
                self.push_event(frame, id, EventKind::Spawned);
            }
            CommandKind::Move {
                entity,
                x,
                y,
                group,
                append,
            } => {
                // 执行时重新校验：被接收不等于可执行。
                if !self.is_actionable(entity, command.player_id) {
                    return;
                }
                if append {
                    if let Some(unit) = self.units.get_mut(&entity) {
                        if unit.pending.len() < MAX_PENDING_ORDERS {
                            unit.pending.push(Order::Move { x, y, group });
                        }
                    }
                    return;
                }
                if group != 0 {
                    // 旧订单继续，直到规划提交；不立即改变路径。
                    group_requests
                        .entry(group)
                        .or_default()
                        .push((entity, x, y));
                } else {
                    self.assign_move(entity, x, y, 0);
                }
            }
            CommandKind::Stop { entity } => {
                if !self.is_actionable(entity, command.player_id) {
                    return;
                }
                self.stop_unit(entity);
                self.push_event(frame, entity, EventKind::Stopped);
            }
            CommandKind::Attack { attacker, target } => {
                if !self.is_actionable(attacker, command.player_id) || !self.is_alive(target) {
                    return;
                }
                // P8.5c：玩家命令与野怪 AI 共用同一订单赋值路径（复用既有
                // `Attack` 命令面，不新设 AI × lockstep 架构，D3）。
                self.order_attack(attacker, target, attacks);
            }
            CommandKind::SetObstacle { id, x, y, w, h } => {
                let area = GridArea { x, y, w, h };
                if !self.obstacle_in_bounds(area) {
                    return;
                }
                self.obstacles.insert(id, area);
                *obstacles_changed = true;
            }
            CommandKind::RemoveObstacle { id } => {
                if self.obstacles.remove(&id).is_some() {
                    *obstacles_changed = true;
                }
            }
        }
    }

    fn is_alive(&self, entity: EntityId) -> bool {
        self.entities.contains(entity) && self.units.get(&entity).is_some_and(|unit| unit.alive)
    }

    /// 可操作校验：存活、归属匹配，且未被眩晕（`Stun` Buff 仍在生效）。
    ///
    /// P8.5c：中立登记的野怪 / 地面物品 MUST NOT 响应任何玩家命令
    /// （R1.4 / R8.1 / Q7，消除 `player_id == 0` 旁路）。野怪 AI 走
    /// `run_creep_ai` 复用的 `order_attack` / `assign_move` 内部路径，
    /// 不经 `is_actionable`。地面物品不是 `EntityStore` 实体，天然不
    /// 出现在 `Move` 生成集合、位置不变。
    /// P8.5d：商店是定义层货架表（非中立建筑实体），其稳定标识与地面
    /// 物品同属 `GROUND_ITEM_ID_BASE` 以上的保留段，MUST NOT 生成 `Move`
    /// （R1.1 / R1.4 / R9.1 / R13.6），不借只读视图投影放松旁路门禁。
    fn is_actionable(&self, entity: EntityId, player_id: u8) -> bool {
        if self.creep_camps.neutral.contains_key(&entity) {
            return false;
        }
        if entity.value() >= GROUND_ITEM_ID_BASE {
            return false;
        }
        self.entities.contains(entity)
            && self.units.get(&entity).is_some_and(|unit| {
                unit.alive
                    && unit.owner == player_id
                    && !unit
                        .gameplay
                        .buffs
                        .iter()
                        .any(|buff| buff.kind == EffectKind::Stun && buff.remaining > 0)
            })
    }

    fn obstacle_in_bounds(&self, area: GridArea) -> bool {
        let width = self.world.width() as i64;
        let height = self.world.height() as i64;
        area.x >= 0
            && area.y >= 0
            && area.x as i64 + area.w as i64 <= width
            && area.y as i64 + area.h as i64 <= height
    }

    fn stop_unit(&mut self, entity: EntityId) {
        if let Some(unit) = self.units.get_mut(&entity) {
            unit.order = Order::Stop;
            unit.pending.clear();
            unit.goal = None;
            unit.path.clear();
            unit.path_index = 0;
            unit.blocked_frames = 0;
            unit.group = 0;
            // 停住单位不参与移动积分，也不会被其它单位推移。
        }
    }

    /// 设定移动意图并尝试寻路；失败时保留空路径，由拥堵逻辑累计。
    fn assign_move(&mut self, entity: EntityId, x: SimFixed, y: SimFixed, group: u32) {
        if !self.is_alive(entity) {
            return;
        }
        if let Some(unit) = self.units.get_mut(&entity) {
            unit.pending.clear();
            unit.order = Order::Move { x, y, group };
            unit.goal = Some((x, y));
            unit.path.clear();
            unit.path_index = 0;
            unit.blocked_frames = 0;
            unit.group = group;
        }
        self.prepare_move(entity, x, y, group);
    }

    /// 从当前位置到目标重新寻路，转成世界坐标路点（末段为精确目标）。
    fn prepare_move(&mut self, entity: EntityId, x: SimFixed, y: SimFixed, _group: u32) {
        let Some((start_cell, goal_cell)) = self.units.get(&entity).map(|unit| {
            (
                self.world.world_to_cell(unit.x, unit.y),
                self.world.world_to_cell(x, y),
            )
        }) else {
            return;
        };
        let Some(cells) = nav::find_path(
            &self.world,
            &self.obstacles,
            start_cell,
            goal_cell,
            DEFAULT_NODE_BUDGET,
        ) else {
            let unit = self.units.get_mut(&entity).expect("checked above");
            unit.path.clear();
            unit.path_index = 0;
            return;
        };
        let mut waypoints: Vec<(SimFixed, SimFixed)> = cells
            .iter()
            .skip(1)
            .map(|&cell| self.world.cell_center(cell))
            .collect();
        waypoints.push((x, y));
        let unit = self.units.get_mut(&entity).expect("checked above");
        unit.path = waypoints;
        unit.path_index = 0;
    }

    fn enqueue_group(&mut self, group: u32, members: Vec<(EntityId, SimFixed, SimFixed)>) {
        if members.is_empty() {
            return;
        }
        let goal = (members[0].1, members[0].2);
        let ids: Vec<EntityId> = members.iter().map(|(id, _, _)| *id).collect();
        let cell_size = self.world.cell_size();
        let slots: Vec<(SimFixed, SimFixed)> = (0..ids.len())
            .map(|index| formation_slot(index, goal, cell_size))
            .collect();
        self.group_plans.insert(
            group,
            GroupPlan {
                goal,
                members: ids,
                slots,
                progress: 0,
            },
        );
    }

    fn advance_group_plans(&mut self) {
        let mut committed = None;
        let group_ids: Vec<u32> = self.group_plans.keys().copied().collect();
        for group in group_ids {
            let plan = self.group_plans.get_mut(&group).expect("key from keys()");
            let remaining = plan.members.len().saturating_sub(plan.progress);
            plan.progress += remaining.min(PLAN_BUDGET);
            if plan.progress >= plan.members.len() && committed.is_none() {
                committed = Some(group);
            }
        }
        if let Some(group) = committed {
            let plan = self.group_plans.remove(&group).expect("committed key");
            for (index, &id) in plan.members.iter().enumerate() {
                let (x, y) = plan.slots[index];
                self.assign_move(id, x, y, group);
            }
        }
    }

    fn replan_all(&mut self) {
        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        for id in ids {
            let Some(unit) = self.units.get(&id) else {
                continue;
            };
            if !unit.alive {
                continue;
            }
            let Order::Move { x, y, group } = unit.order else {
                continue;
            };
            self.prepare_move(id, x, y, group);
        }
    }

    fn advance_motions(&mut self, frame: Frame) {
        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        let fps = SimFixed::from_int(self.config.logic_fps as i64).unwrap_or(SimFixed::ONE);

        // 阶段 A：每单位每帧最多激活一个待执行订单（拥堵失败续队列）。
        for &id in &ids {
            let next = {
                let Some(unit) = self.units.get_mut(&id) else {
                    continue;
                };
                if unit.alive && unit.order == Order::Idle && !unit.pending.is_empty() {
                    Some(unit.pending.remove(0))
                } else {
                    None
                }
            };
            if let Some(Order::Move { x, y, group }) = next {
                self.assign_move(id, x, y, group);
            }
        }

        // 阶段 B：按稳定 EntityId 顺序逐帧积分。
        let mut finished: Vec<(EntityId, bool)> = Vec::new();
        for &id in &ids {
            let snapshot = {
                let Some(unit) = self.units.get(&id) else {
                    continue;
                };
                if !unit.alive || !matches!(unit.order, Order::Move { .. }) {
                    continue;
                }
                (
                    unit.x,
                    unit.y,
                    unit.speed,
                    unit.turn_rate,
                    unit.facing,
                    unit.path.clone(),
                    unit.path_index,
                    unit.blocked_frames,
                )
            };
            let (mut x, mut y, speed, turn_rate, facing, path, mut path_index, blocked_frames) =
                snapshot;

            // 拥堵检测：当前格被动态障碍覆盖或无有效路径 → 停下累计。
            let current_cell = self.world.world_to_cell(x, y);
            let obstructed = self
                .obstacles
                .values()
                .any(|area| area.contains(current_cell));
            if obstructed || path.is_empty() {
                let blocked = blocked_frames.saturating_add(1);
                if let Some(unit) = self.units.get_mut(&id) {
                    unit.blocked_frames = blocked;
                }
                if blocked >= CONGESTION_TIMEOUT_FRAMES {
                    finished.push((id, false));
                }
                continue;
            }
            if let Some(unit) = self.units.get_mut(&id) {
                unit.blocked_frames = 0;
            }

            if path_index >= path.len() {
                finished.push((id, true));
                continue;
            }

            // 本帧只转向一次；多段共享同一位移预算与倍率。
            let (tx, ty) = path[path_index];
            let dx = tx.raw().saturating_sub(x.raw());
            let dy = ty.raw().saturating_sub(y.raw());
            let dist = isqrt((dx as i128) * (dx as i128) + (dy as i128) * (dy as i128));
            if dist == 0 {
                path_index += 1;
                if let Some(unit) = self.units.get_mut(&id) {
                    unit.path_index = path_index;
                }
                if path_index >= path.len() {
                    finished.push((id, true));
                }
                continue;
            }
            let desired = Angle::from_direction(dx, dy);
            let new_facing = facing.turn_towards(desired, turn_rate);
            let turn_scale = new_facing.turn_speed_scale(desired);
            let from_cell = self.world.world_to_cell(x, y);
            let to_cell = self.world.world_to_cell(tx, ty);
            let from_height = self.world.height_at(from_cell).raw();
            let to_height = self.world.height_at(to_cell).raw();
            let rise = to_height.saturating_sub(from_height);
            let slope_scale =
                slope_speed_scale(SimFixed::from_raw(rise), SimFixed::from_raw(dist as i64));
            let scale = turn_scale.checked_mul(slope_scale).unwrap_or(SimFixed::ONE);
            let per_frame = speed.checked_div(fps).unwrap_or(SimFixed::ZERO);
            let mut budget = per_frame.checked_mul(scale).unwrap_or(SimFixed::ZERO).raw() as i128;

            while budget > 0 && path_index < path.len() {
                let (tx, ty) = path[path_index];
                let dx = tx.raw().saturating_sub(x.raw());
                let dy = ty.raw().saturating_sub(y.raw());
                let d = isqrt((dx as i128) * (dx as i128) + (dy as i128) * (dy as i128));
                if d == 0 {
                    path_index += 1;
                    continue;
                }
                if budget >= d {
                    x = SimFixed::from_raw(tx.raw());
                    y = SimFixed::from_raw(ty.raw());
                    path_index += 1;
                    budget -= d;
                } else {
                    let nx = x.raw().saturating_add(((dx as i128) * budget / d) as i64);
                    let ny = y.raw().saturating_add(((dy as i128) * budget / d) as i64);
                    x = SimFixed::from_raw(nx);
                    y = SimFixed::from_raw(ny);
                    budget = 0;
                }
            }

            if let Some(unit) = self.units.get_mut(&id) {
                unit.x = x;
                unit.y = y;
                unit.facing = new_facing;
                unit.path_index = path_index;
            }
            if path_index >= path.len() {
                finished.push((id, true));
            }
        }

        for (id, success) in finished {
            self.finish_order(id, frame, success);
        }
    }

    fn finish_order(&mut self, entity: EntityId, frame: Frame, success: bool) {
        if let Some(unit) = self.units.get_mut(&entity) {
            unit.order = Order::Idle;
            unit.goal = None;
            unit.path.clear();
            unit.path_index = 0;
            unit.blocked_frames = 0;
            unit.group = 0;
        }
        let kind = if success {
            EventKind::Arrived
        } else {
            EventKind::MoveFailed
        };
        self.push_event(frame, entity, kind);
        // 待执行订单保留，下一帧阶段 A 激活，避免同帧重复发放运动预算。
    }

    fn resolve_attacks(&mut self, frame: Frame, attacks: &[(EntityId, EntityId)]) {
        for &(attacker, target) in attacks {
            // 存活复核：同帧先死者使后续针对它的攻击失效。
            if !self.is_alive(attacker) || !self.is_alive(target) {
                continue;
            }
            let (damage, range, ax, ay) = {
                let unit = self.units.get(&attacker).expect("checked above");
                (
                    unit.attack_damage
                        .saturating_add(unit.gameplay.stat_mods.damage),
                    unit.attack_range.raw(),
                    unit.x.raw(),
                    unit.y.raw(),
                )
            };
            let (tx, ty) = {
                let unit = self.units.get(&target).expect("checked above");
                (unit.x.raw(), unit.y.raw())
            };
            let dx = tx.saturating_sub(ax);
            let dy = ty.saturating_sub(ay);
            if isqrt((dx as i128) * (dx as i128) + (dy as i128) * (dy as i128)) > range as i128 {
                continue;
            }
            // 单一结算所有者：攻击不得绕过 Settlement 直接改权威生命值。
            let outcome = self.apply_effect(
                target,
                &EffectDef {
                    kind: EffectKind::Damage,
                    amount: damage,
                    duration_frames: 0,
                },
                None,
            );
            let died = !self.is_alive(target);
            if died {
                // 沿用现状语义：只清订单 / 路径、不删实体、不判负。
                if let Some(unit) = self.units.get_mut(&target) {
                    unit.order = Order::Idle;
                    unit.pending.clear();
                    unit.goal = None;
                    unit.path.clear();
                    unit.path_index = 0;
                    unit.blocked_frames = 0;
                    unit.group = 0;
                }
                // 英雄阵亡登记：整数帧号 + 完整玩法运行态快照（等级 / 经验 /
                // 背包 / 技能登记 / 法力），不删实体、不判负（R6.3 / R11.2）。
                let hero_snapshot = self.units.get(&target).and_then(|unit| {
                    unit.gameplay
                        .hero
                        .map(|_| (unit.gameplay.clone(), unit.owner, unit.max_hp))
                });
                if let Some((gameplay, owner, max_hp)) = hero_snapshot {
                    self.register_hero_death(target, frame, gameplay, owner, max_hp);
                }
                // P8.5c：野怪死亡通知进共享结算服务（幂等，Q6 / tasks 4.2）。
                // 击杀方 owner 用于 R8.3 金币结算；MUST NOT 由表现层反写。
                if self.creep_camps.creeps.contains_key(&target) {
                    let killer_owner = self
                        .units
                        .get(&attacker)
                        .map(|unit| unit.owner)
                        .unwrap_or(NEUTRAL_OWNER);
                    self.notify_creep_death(target, attacker, killer_owner);
                }
            }
            if outcome == EffectOutcome::Applied {
                self.push_event(frame, target, EventKind::AttackHit);
            }
            if died {
                self.push_event(frame, target, EventKind::Died);
            }
        }
    }

    /// 既有 `Attack` 命令面的内部订单赋值，供 `apply` 与野怪 AI 共用（D3）。
    fn order_attack(
        &mut self,
        attacker: EntityId,
        target: EntityId,
        attacks: &mut Vec<(EntityId, EntityId)>,
    ) {
        if let Some(unit) = self.units.get_mut(&attacker) {
            unit.pending.clear();
            unit.order = Order::Attack { target };
            unit.goal = None;
            unit.path.clear();
            unit.path_index = 0;
            unit.blocked_frames = 0;
            unit.group = 0;
        }
        attacks.push((attacker, target));
    }

    fn push_event(&mut self, frame: Frame, entity: EntityId, kind: EventKind) {
        self.push_event_raw(frame, entity.value(), kind);
    }

    /// 供营地 id / 地面物品 id 这类非 `EntityStore` 实体发只读事实事件
    /// （`Event.entity` 是 `u64`，营地实例 id 与地面物品稳定 ID 段直接用）。
    fn push_event_raw(&mut self, frame: Frame, entity: u64, kind: EventKind) {
        self.events.push(Event {
            frame: frame.value(),
            sequence: self.next_event_seq,
            entity,
            kind,
        });
        self.next_event_seq += 1;
    }

    /// 从 `UnitState` 构造结算视图；上限字段先复制，避免借用冲突。
    fn make_vitals(unit: &mut UnitState) -> Vitals<'_> {
        let max_hp = unit.max_hp;
        Vitals {
            hp: &mut unit.hp,
            max_hp,
            alive: &mut unit.alive,
            mana: &mut unit.gameplay.mana,
            buffs: &mut unit.gameplay.buffs,
            stat_mods: &mut unit.gameplay.stat_mods,
        }
    }

    /// 单个效果经统一结算服务落到目标；目标死亡后后续效果返回 `NoEffect`。
    /// P11 新增效果原语转入技能权威运行态目录，旧效果仍只经 `Settlement`。
    fn apply_effect(
        &mut self,
        target: EntityId,
        effect: &EffectDef,
        context: Option<AbilityEffectContext>,
    ) -> EffectOutcome {
        if !matches!(
            effect.kind,
            EffectKind::Damage | EffectKind::Heal | EffectKind::BuffStat | EffectKind::Stun
        ) {
            let fallback = AbilityEffectContext {
                caster: target,
                ability: 0,
                target: AbilityTargetKind::Entity(target),
                player: 0,
                frame: 0,
            };
            return self.apply_ability_effect(target, effect, context.unwrap_or(fallback));
        }
        let Self {
            units, gameplay, ..
        } = self;
        let Some(unit) = units.get_mut(&target) else {
            return EffectOutcome::NoEffect;
        };
        let mut vitals = Self::make_vitals(unit);
        let outcome = match effect.kind {
            EffectKind::Damage => Settlement::damage(&mut vitals, effect.amount),
            EffectKind::Heal => Settlement::heal(&mut vitals, effect.amount),
            // BuffStat 经结算同时改属性修正并登记短期 Buff；Stun 登记眩晕 Buff。
            EffectKind::BuffStat => Settlement::stat_mod(&mut vitals, effect),
            EffectKind::Stun => Settlement::buff(&mut vitals, effect),
            _ => EffectOutcome::NoEffect,
        };
        if outcome == EffectOutcome::Applied
            && matches!(effect.kind, EffectKind::BuffStat | EffectKind::Stun)
        {
            gameplay
                .abilities
                .buff_categories
                .entry(target)
                .or_default()
                .insert(BuffCategory::from_effect_kind(effect.kind));
        }
        outcome
    }

    /// P11 技能效果原语：按单一行为目录查表分派，只写技能权威运行态；
    /// 生命 / Buff 改写仍经 `Settlement`，不新增独立随机流。
    fn apply_ability_effect(
        &mut self,
        target: EntityId,
        effect: &EffectDef,
        context: AbilityEffectContext,
    ) -> EffectOutcome {
        let Self {
            units,
            gameplay,
            defs,
            entities,
            ..
        } = self;
        let behavior_id = defs
            .abilities()
            .get(&context.ability)
            .map_or(0, |def| def.behavior);
        let Some(behavior) = defs
            .ability_system()
            .behaviors
            .dispatch_by_effect(effect.kind)
            .or_else(|| defs.ability_system().behaviors.dispatch_by_id(behavior_id))
        else {
            return EffectOutcome::NoEffect;
        };
        let constants = defs.ability_system().constants;
        let runtime = &mut gameplay.abilities;
        let beneficiary_ids: BTreeSet<EntityId> = units
            .iter()
            .filter_map(|(&id, unit)| unit.alive.then_some(id))
            .collect();
        let caster_state = units.get(&context.caster).cloned();

        if behavior == BehaviorKind::Summon {
            let Some(mut summoned) = caster_state else {
                return EffectOutcome::NoEffect;
            };
            let id = entities.spawn();
            summoned.owner = context.player;
            summoned.hp = constants.summon_hp.max(1);
            summoned.max_hp = summoned.hp;
            summoned.alive = true;
            summoned.order = Order::Idle;
            summoned.pending.clear();
            summoned.goal = None;
            summoned.path.clear();
            summoned.path_index = 0;
            summoned.blocked_frames = 0;
            summoned.group = 0;
            summoned.gameplay = UnitGameplayState::default();
            units.insert(id, summoned);
            runtime.summons.insert(
                id,
                SummonState {
                    summon: id,
                    owner: context.caster,
                    player: context.player,
                    ability: context.ability,
                    remaining: constants.summon_frames.max(1),
                },
            );
            return EffectOutcome::Applied;
        }

        let Some(unit) = units.get_mut(&target) else {
            return EffectOutcome::NoEffect;
        };
        let mut vitals = Self::make_vitals(unit);
        match behavior {
            BehaviorKind::Zone => {
                let id = EntityId::new(runtime.next_zone_id).unwrap_or(EntityId::NONE);
                runtime.next_zone_id = runtime.next_zone_id.saturating_add(1);
                if id == EntityId::NONE {
                    return EffectOutcome::NoEffect;
                }
                let (x, y) = context.target.point();
                let radius_raw = context.target.radius_raw(constants.point_radius_raw);
                let beneficiaries = beneficiary_ids.iter().copied().take(8).collect();
                runtime.zones.insert(
                    id,
                    ZoneState {
                        id,
                        source: context.caster,
                        ability: context.ability,
                        owner: context.player,
                        x,
                        y,
                        radius_raw,
                        remaining: constants.zone_frames.max(1),
                        beneficiaries,
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::Aura => {
                runtime.aura_beneficiaries.insert(
                    context.caster,
                    AuraBeneficiarySet {
                        source: context.caster,
                        ability: context.ability,
                        beneficiaries: beneficiary_ids,
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::PassiveProc => {
                runtime.passive_procs.insert(
                    (target, context.ability),
                    PassiveProcState {
                        subscribed_frame: context.frame,
                        remaining: u32::MAX,
                        random: false,
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::Transform => {
                runtime.transformations.insert(
                    target,
                    TransformState {
                        ability: context.ability,
                        state_code: effect.duration_frames as u8,
                        remaining: constants.transform_frames.max(1),
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::PointTarget => {
                runtime.point_targets.insert(
                    context.caster,
                    PointTargetState {
                        caster: context.caster,
                        ability: context.ability,
                        target: context.target,
                        frame: context.frame,
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::RemoveBuffCategory => {
                let Some(category) = BuffCategory::from_code(effect.amount as u8) else {
                    return EffectOutcome::NoEffect;
                };
                if category == BuffCategory::None {
                    return EffectOutcome::NoEffect;
                }
                let mut empty = false;
                if let Some(categories) = runtime.buff_categories.get_mut(&target) {
                    categories.remove(&category);
                    empty = categories.is_empty();
                }
                if empty {
                    runtime.buff_categories.remove(&target);
                }
                Settlement::remove_buff_category(&mut vitals, category)
            }
            BehaviorKind::ChainBounce => {
                let id = EntityId::new(runtime.next_chain_id).unwrap_or(EntityId::NONE);
                runtime.next_chain_id = runtime.next_chain_id.saturating_add(1);
                if id == EntityId::NONE {
                    return EffectOutcome::NoEffect;
                }
                runtime.chains.insert(
                    id,
                    ChainState {
                        id,
                        source: context.caster,
                        target: context.target.entity_or(target),
                        ability: context.ability,
                        bounces: constants.chain_bounces.max(1),
                        remaining: constants.zone_frames.max(1),
                    },
                );
                Settlement::damage(&mut vitals, constants.chain_damage)
            }
            BehaviorKind::RandomDraw => {
                runtime.passive_procs.insert(
                    (target, context.ability),
                    PassiveProcState {
                        subscribed_frame: context.frame,
                        remaining: u32::MAX,
                        random: true,
                    },
                );
                EffectOutcome::Applied
            }
            BehaviorKind::Summon => EffectOutcome::NoEffect,
        }
    }

    /// 法力费用提交事务：与效果执行链分离，失败不产生任何改动。
    fn commit_mana_cost(&mut self, caster: EntityId, cost: i64) -> bool {
        let Some(unit) = self.units.get_mut(&caster) else {
            return false;
        };
        Settlement::commit_cost(&mut Self::make_vitals(unit), cost)
    }

    /// 技能效果链：顺序执行；目标中途死亡后由 `Settlement` 自然返回 `NoEffect`，
    /// 已提交的效果不回滚、不重复扣费。
    fn apply_ability_effects(&mut self, context: AbilityEffectContext) {
        let effects = resolve_effects(&self.defs, &self.gameplay, context.ability);
        let target = context.target.entity_or(context.caster);
        for effect in effects {
            self.apply_effect(target, &effect, Some(context));
        }
    }

    /// 英雄技能学习 / 升级：技能点、等级与 6 级大招由冻结清单与运行态共同校验。
    fn learn_ability(&mut self, hero: EntityId, ability: DefId, player: u8) {
        if !self.is_actionable(hero, player) {
            return;
        }
        let Some(entry) = self.defs.ability_system().abilities.get(ability).copied() else {
            return;
        };
        let Some(level) = self
            .units
            .get(&hero)
            .and_then(|unit| unit.gameplay.hero.map(|state| state.level))
        else {
            return;
        };
        if entry.category == AbilityCategory::HeroUltimate && level < 6 {
            return;
        }
        let key = (hero, ability);
        let learned = self
            .gameplay
            .abilities
            .ability_levels
            .get(&key)
            .copied()
            .unwrap_or(0);
        if learned >= u32::from(entry.levels) {
            return;
        }
        let points = self
            .gameplay
            .abilities
            .hero_skill_points
            .entry(hero)
            .or_insert(level.max(1));
        if *points == 0 {
            return;
        }
        *points -= 1;
        self.gameplay
            .abilities
            .ability_levels
            .insert(key, learned.saturating_add(1));
    }

    /// 英雄升级并累积技能点；本期不含 AI 自主升级 / 施法路径。
    fn level_up_hero(&mut self, hero: EntityId, player: u8) {
        if !self.is_actionable(hero, player) {
            return;
        }
        let Self {
            units, gameplay, ..
        } = self;
        let Some(unit) = units.get_mut(&hero) else {
            return;
        };
        let Some(state) = unit.gameplay.hero.as_mut() else {
            return;
        };
        state.level = state.level.saturating_add(1);
        let points = gameplay
            .abilities
            .hero_skill_points
            .entry(hero)
            .or_insert(1);
        *points = points.saturating_add(1);
    }

    /// 施法（玩家命令与 AI 共用入口）：同源校验 → 费用事务 → 运行态 / 效果链。
    /// 点 / 范围 / 方向目标与实体目标共用同一命令面，不新增 AI 决策源。
    fn execute_cast(
        &mut self,
        caster: EntityId,
        ability: DefId,
        target: AbilityTargetKind,
        player: u8,
        frame: Frame,
    ) {
        if !self.is_actionable(caster, player) {
            return;
        }
        if let AbilityTargetKind::Entity(entity) = target {
            if !self.is_alive(entity) {
                return;
            }
        }
        let Some(def) = self.defs.abilities().get(&ability).cloned() else {
            return;
        };
        let on_cooldown = self
            .units
            .get(&caster)
            .and_then(|unit| unit.gameplay.cooldowns.get(&ability))
            .is_some();
        if on_cooldown || !self.commit_mana_cost(caster, def.mana_cost) {
            return;
        }
        if def.cooldown_frames > 0 {
            if let Some(unit) = self.units.get_mut(&caster) {
                unit.gameplay.cooldowns.insert(ability, def.cooldown_frames);
            }
        }
        let context = AbilityEffectContext {
            caster,
            ability,
            target,
            player,
            frame: frame.value(),
        };
        if def.cast_frames > 0 {
            if let Some(unit) = self.units.get_mut(&caster) {
                unit.gameplay.cast = Some(CastRuntime {
                    ability,
                    target: target.entity_or(caster),
                    remaining_frames: def.cast_frames,
                    channel: true,
                });
            }
            self.gameplay.abilities.pending_casts.insert(
                caster,
                PendingAbilityCast {
                    caster,
                    ability,
                    target,
                    player,
                    frame: frame.value(),
                },
            );
            if def.behavior != 0 {
                self.gameplay.auras.insert(
                    caster.value() as u32,
                    AuraState {
                        source: caster,
                        ability,
                        remaining: def.cast_frames,
                    },
                );
            }
        } else {
            self.apply_ability_effects(context);
        }
    }

    fn enqueue_production(
        &mut self,
        producer: EntityId,
        unit_def: DefId,
        is_building: bool,
        player: u8,
    ) {
        if !self.is_actionable(producer, player) {
            return;
        }
        // P8.5b：冻结 tier 需求链前置判据，未达 tier 显式拒绝、不生成队列项、
        // 不扣资源、不改变权威状态（MUST NOT 沿用「全量可训练」旧行为，C4）。
        if !self.tier_requirement_met(player, unit_def) {
            return;
        }
        let frames = self
            .defs
            .units()
            .get(&unit_def)
            .map(|def| def.train_frames)
            .unwrap_or(0);
        if let Some(unit) = self.units.get_mut(&producer) {
            unit.gameplay.production.push(ProductionItem {
                unit_def,
                remaining_frames: frames.max(1),
                is_building,
            });
        }
        self.gameplay.players.entry(player).or_default();
    }

    fn cancel_train(&mut self, producer: EntityId, index: u32) {
        if let Some(unit) = self.units.get_mut(&producer) {
            let index = index as usize;
            if index < unit.gameplay.production.len() {
                unit.gameplay.production.remove(index);
            }
        }
    }

    fn research(&mut self, player: u8, upgrade: DefId) {
        // P8.5b：冻结 tier 需求链前置判据，未达 tier 显式拒绝、不生成研究队列项、
        // 不改变权威状态（R4.4 / C4 / D4）。
        if !self.tier_requirement_met(player, upgrade) {
            return;
        }
        let frames = self
            .defs
            .upgrades()
            .get(&upgrade)
            .map(|def| def.research_frames)
            .unwrap_or(0);
        let economy = self.gameplay.players.entry(player).or_default();
        economy.research = Some((upgrade, frames.max(1)));
    }

    /// P5 遗留 `BuyItem` 占位的收敛（R-1 / R9.3 / ADR 0005 §3）。
    ///
    /// 变体编码与 `hash_gameplay_command` 码 5 保留不动，但语义重写为
    /// **显式拒单**：MUST NOT 再写 `PlayerEconomy.gold` / 六格实例背包 /
    /// 商店库存或凭空建实例。购买的唯一事务入口是
    /// [`GameplayCommandKind::BuyFromShop`]（码 16）→ `submit_purchase`。
    /// 只登记一条 `Rejected` 事实，供只读表现层按 `Event.entity` 查表。
    fn buy_item(&mut self, unit: EntityId, item: DefId, frame: Frame) {
        self.record_item_fact(
            frame,
            ItemShopFactKind::Rejected,
            unit,
            EntityId::NONE,
            u8::MAX,
            EntityId::NONE,
            0,
            item,
        );
        self.push_event(frame, unit, EventKind::ItemShopCommandRejected);
    }

    // -----------------------------------------------------------------------
    // P8.5d 物品 / 商店 / 回城卷轴：唯一事务命令面（R7 / R9.3 / Q5 / Q7）
    // -----------------------------------------------------------------------

    /// 登记一条确定性事实（稳定 `seq`；只读表现层按 `Event.entity` 查表）。
    fn record_item_fact(
        &mut self,
        frame: Frame,
        kind: ItemShopFactKind,
        hero: EntityId,
        instance: EntityId,
        slot: u8,
        entity: EntityId,
        gold_delta: i64,
        shelf: u32,
    ) {
        let mut fact = ItemShopFact {
            seq: 0,
            frame: frame.value(),
            kind,
            hero,
            instance,
            slot,
            entity,
            gold_delta,
            shelf,
        };
        self.item_shop.push_fact(&mut fact);
    }

    /// 显式拒单：不扣费 / 不建实例 / 不改库存 / 不改背包，只登记拒单事实。
    fn reject_item_command(&mut self, frame: Frame, hero: EntityId, shelf: u32) {
        self.record_item_fact(
            frame,
            ItemShopFactKind::Rejected,
            hero,
            EntityId::NONE,
            u8::MAX,
            EntityId::NONE,
            0,
            shelf,
        );
        self.push_event(frame, hero, EventKind::ItemShopCommandRejected);
    }

    /// 六格实例背包的只读 owned 拷贝；未登记视为空包。
    fn item_bag(&self, hero: EntityId) -> ItemBag {
        self.item_shop.bags.get(&hero).copied().unwrap_or_default()
    }

    /// 整数坐标半径判据（`isqrt` 整数数学，MUST NOT 用 `f64` / 墙钟，ADR 0002 §1）。
    fn within_radius(&self, entity: EntityId, x: i64, y: i64, radius: i64) -> bool {
        let Some(unit) = self.units.get(&entity) else {
            return false;
        };
        let dx = unit.x.raw().saturating_sub(x) as i128;
        let dy = unit.y.raw().saturating_sub(y) as i128;
        isqrt(dx * dx + dy * dy) <= i128::from(radius.max(0))
    }

    /// 拾取地面物品闭环（R1.1 / R7.2 / R7.5 / R13.6）。
    ///
    /// 实例以同一 `EntityId` 从 `ground_items` / `neutral` 迁移进六格背包，
    /// MUST NOT 新建实例或重置充能 / 冷却；第二次拾取因登记已移除而显式拒绝
    /// （幂等）。地面物品不是 `EntityStore` 实体，MUST NOT 进 `Move` 生成集合。
    fn submit_pickup(&mut self, hero: EntityId, ground_item: EntityId, player: u8, frame: Frame) {
        if !self.is_actionable(hero, player) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let Some(ground) = self.creep_camps.ground_items.get(&ground_item).cloned() else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let ownerless = self.creep_camps.neutral.get(&ground_item)
            == Some(&NeutralCategory::GroundItemOwnerless);
        let instance_ok = self
            .creep_camps
            .item_instances
            .get(&ground.instance)
            .is_some_and(|instance| instance.holder.is_none() && instance.slot.is_none());
        let radius = self.defs.item_shop().constants.pickup_radius_raw;
        let bag = self.item_bag(hero);
        let Some(slot) = bag.first_free_slot() else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        if !ownerless || !instance_ok || !self.within_radius(hero, ground.x, ground.y, radius) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        // 迁移（不建新实例、不刷新充能冷却）。
        self.creep_camps.ground_items.remove(&ground_item);
        self.creep_camps.neutral.remove(&ground_item);
        if let Some(instance) = self.creep_camps.item_instances.get_mut(&ground.instance) {
            instance.holder = Some(hero);
            instance.slot = Some(slot);
        }
        let mut bag = bag;
        bag.set(slot, Some(ground.instance));
        self.item_shop.bags.insert(hero, bag);
        self.record_item_fact(
            frame,
            ItemShopFactKind::Pickup,
            hero,
            ground.instance,
            slot,
            ground_item,
            0,
            ground.item_def,
        );
        self.push_event(frame, hero, EventKind::ItemPickedUp);
    }

    /// 商店购买费用提交事务（复用 `submit_*` 先例，R7.3 / R9.3）。
    ///
    /// 校验「距离 / 解冻 / 库存 / 黄金 / 背包空位」→ 精确扣费 → 冻结扣费
    /// 登记（`PurchaseCharge`）→ 帧阶段 `advance_purchases` 建实例 / 入包 /
    /// 库存递减；任一后续失败由 `refund_purchase` 精确退款、恢复库存、
    /// 不生成半实例。黄金只经本命令面写 `PlayerEconomy.gold`。
    fn submit_purchase(
        &mut self,
        hero: EntityId,
        shop: u32,
        shelf_id: u32,
        player: u8,
        frame: Frame,
    ) {
        let Some(shop_def) = self.defs.item_shop().shop(shop).cloned() else {
            self.reject_item_command(frame, hero, shelf_id);
            return;
        };
        let Some(shelf) = self.defs.item_shop().shelf(shelf_id).copied() else {
            self.reject_item_command(frame, hero, shelf_id);
            return;
        };
        let shop_registered = self.defs.item_shop().is_shop_def(shop);
        let in_shop = shop_def.shelf_ids.contains(&shelf_id);
        let unlocked = frame.value() >= shelf.unlock_frame;
        let radius_ok =
            self.within_radius(hero, shop_def.x, shop_def.y, shop_def.interact_radius_raw);
        let gold = self
            .gameplay
            .players
            .get(&player)
            .map(|economy| economy.gold)
            .unwrap_or(0);
        let stock = self.item_shop.stock.get(&shelf_id).copied().unwrap_or(0);
        let bag = self.item_bag(hero);
        let Some(slot) = bag.first_free_slot() else {
            self.reject_item_command(frame, hero, shelf_id);
            return;
        };
        if !self.is_actionable(hero, player)
            || !shop_registered
            || !in_shop
            || !unlocked
            || !radius_ok
            || gold < shelf.price
            || stock == 0
        {
            self.reject_item_command(frame, hero, shelf_id);
            return;
        }
        // 精确扣费（唯一写入路径）并冻结扣费登记。
        {
            let economy = self.gameplay.players.entry(player).or_default();
            economy.gold -= shelf.price;
        }
        let instance =
            EntityId::new(self.creep_camps.next_ground_item_id).unwrap_or(EntityId::NONE);
        self.creep_camps.next_ground_item_id =
            self.creep_camps.next_ground_item_id.saturating_add(1);
        let seq = self.item_shop.next_charge_seq;
        self.item_shop.next_charge_seq = seq.saturating_add(1);
        self.item_shop.charges.insert(
            seq,
            PurchaseCharge {
                owner: player,
                gold: shelf.price,
                hero,
                instance,
                slot,
                shelf: shelf_id,
                shop,
                frame: frame.value(),
            },
        );
    }

    /// 购买事务性退款：把冻结扣费精确回退到玩家经济，不生成半实例。
    fn refund_purchase(&mut self, seq: u64, frame: Frame) {
        let Some(charge) = self.item_shop.charges.remove(&seq) else {
            return;
        };
        let economy = self.gameplay.players.entry(charge.owner).or_default();
        economy.gold = economy.gold.saturating_add(charge.gold);
        self.record_item_fact(
            frame,
            ItemShopFactKind::Rejected,
            charge.hero,
            EntityId::NONE,
            u8::MAX,
            EntityId::NONE,
            0,
            charge.shelf,
        );
        self.push_event(frame, charge.hero, EventKind::ItemShopCommandRejected);
    }

    /// 购买事务帧阶段：按稳定 `seq` 顺序提交或退款（R9.3 / ADR 0002 §3）。
    fn advance_purchases(&mut self, frame: Frame) {
        let seqs: Vec<u64> = self.item_shop.charges.keys().copied().collect();
        for seq in seqs {
            let Some(charge) = self.item_shop.charges.get(&seq).copied() else {
                continue;
            };
            let shelf = self.defs.item_shop().shelf(charge.shelf).copied();
            let stock = self
                .item_shop
                .stock
                .get(&charge.shelf)
                .copied()
                .unwrap_or(0);
            let bag = self.item_bag(charge.hero);
            let slot_free = bag.slots.get(charge.slot as usize) == Some(&None);
            let instance_free = !self
                .creep_camps
                .item_instances
                .contains_key(&charge.instance);
            let hero_ok = self.is_actionable(charge.hero, charge.owner);
            let Some(shelf) = shelf else {
                self.refund_purchase(seq, frame);
                continue;
            };
            if !hero_ok || !slot_free || !instance_free || stock == 0 {
                self.refund_purchase(seq, frame);
                continue;
            }
            // 提交：建独立实例、入六格背包、库存递减（三者与黄金同帧生效）。
            self.creep_camps.item_instances.insert(
                charge.instance,
                ItemInstanceState {
                    item_def: shelf.item_def,
                    charges: shelf.charges,
                    cooldown_frames: shelf.cooldown_frames,
                    effect: shelf.effect,
                    holder: Some(charge.hero),
                    slot: Some(charge.slot),
                },
            );
            let mut bag = bag;
            bag.set(charge.slot, Some(charge.instance));
            self.item_shop.bags.insert(charge.hero, bag);
            self.item_shop.stock.insert(charge.shelf, stock - 1);
            self.item_shop.charges.remove(&seq);
            self.record_item_fact(
                frame,
                ItemShopFactKind::Purchase,
                charge.hero,
                charge.instance,
                charge.slot,
                EntityId::new(shop_fixture_id(charge.shop)).unwrap_or(EntityId::NONE),
                -charge.gold,
                charge.shelf,
            );
            self.push_event(frame, charge.instance, EventKind::ItemPurchased);
        }
    }

    /// 使用道具「先验证有效再提交」（R7.5 / R7.6 / N5）。
    ///
    /// 充能 > 0 / 冷却到期 / 目标合法 / 定义层类别已登记；失败不扣次数。
    /// 效果只经既有 `EffectDef` 组合走共享结算服务，
    /// MUST NOT 执行 JASS / wtg VM（R7.6 / N5）。
    fn submit_use_item(
        &mut self,
        hero: EntityId,
        instance_id: EntityId,
        target: EntityId,
        player: u8,
        frame: Frame,
    ) {
        if !self.is_actionable(hero, player) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let bag = self.item_bag(hero);
        let Some(slot) = bag.slot_of(instance_id) else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let Some(instance) = self.creep_camps.item_instances.get(&instance_id).cloned() else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let target_ok = target == EntityId::NONE || self.is_alive(target);
        let category = self.defs.item_shop().item_category(instance.item_def);
        let portal_item = self.defs.item_shop().is_town_portal_item(instance.item_def);
        if instance.holder != Some(hero)
            || instance.charges == 0
            || instance.cooldown_frames != 0
            || !target_ok
            || category.is_none()
            || portal_item
        {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        // 提交：道具效果链走既有结算服务，不携带技能运行态上下文。
        let effect_target = if target == EntityId::NONE {
            hero
        } else {
            target
        };
        self.apply_effect(effect_target, &instance.effect, None);
        // 共享冷却按定义层实例组整数帧推进（同名多实例不共用充能）。
        let group = self.defs.item_shop().item_cooldown_group(instance.item_def);
        let group_frames = self
            .defs
            .item_shop()
            .constants
            .default_cooldown_frames
            .max(1);
        let held: Vec<EntityId> = bag.slots.iter().flatten().copied().collect();
        for held_id in held {
            let Some(held_instance) = self.creep_camps.item_instances.get_mut(&held_id) else {
                continue;
            };
            if self
                .defs
                .item_shop()
                .item_cooldown_group(held_instance.item_def)
                != group
            {
                continue;
            }
            held_instance.cooldown_frames = held_instance.cooldown_frames.max(group_frames);
        }
        let charges = instance.charges.saturating_sub(1);
        let consumable = category == Some(ItemCategory::Consumable);
        if charges == 0 && consumable {
            self.creep_camps.item_instances.remove(&instance_id);
            let mut bag = bag;
            bag.set(slot, None);
            self.item_shop.bags.insert(hero, bag);
        } else if let Some(state) = self.creep_camps.item_instances.get_mut(&instance_id) {
            state.charges = charges;
        }
        self.record_item_fact(
            frame,
            ItemShopFactKind::Use,
            hero,
            instance_id,
            slot,
            effect_target,
            0,
            instance.item_def,
        );
        self.push_event(frame, hero, EventKind::ItemUsed);
    }

    /// 丢弃道具回无主地面物品（R1.1 / R6.2 / R7.5）。
    ///
    /// 同一 `EntityId` 实例身份与剩余充能 / 冷却「转交也不能刷新」，
    /// 位置由英雄当前确定性整数坐标给出，重新登记 `ground_items` / `neutral`。
    fn submit_drop(&mut self, hero: EntityId, instance_id: EntityId, player: u8, frame: Frame) {
        if !self.is_actionable(hero, player) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let bag = self.item_bag(hero);
        let Some(slot) = bag.slot_of(instance_id) else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let Some(instance) = self.creep_camps.item_instances.get(&instance_id).cloned() else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        if instance.holder != Some(hero) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let (x, y) = self
            .units
            .get(&hero)
            .map(|unit| (unit.x.raw(), unit.y.raw()))
            .unwrap_or((0, 0));
        if let Some(state) = self.creep_camps.item_instances.get_mut(&instance_id) {
            state.holder = None;
            state.slot = None;
        }
        let mut bag = bag;
        bag.set(slot, None);
        self.item_shop.bags.insert(hero, bag);
        self.creep_camps.ground_items.insert(
            instance_id,
            GroundItemState {
                item_def: instance.item_def,
                camp: 0,
                x,
                y,
                instance: instance_id,
            },
        );
        self.creep_camps
            .neutral
            .insert(instance_id, NeutralCategory::GroundItemOwnerless);
        self.record_item_fact(
            frame,
            ItemShopFactKind::Discard,
            hero,
            instance_id,
            slot,
            instance_id,
            0,
            instance.item_def,
        );
        self.push_event(frame, hero, EventKind::ItemDiscarded);
    }

    /// 两英雄间移交同一实例；目标满包或源格不符显式拒绝（R6.2 / R7.5）。
    fn submit_transfer(
        &mut self,
        from_hero: EntityId,
        to_hero: EntityId,
        instance_id: EntityId,
        player: u8,
        frame: Frame,
    ) {
        if !self.is_actionable(from_hero, player) || !self.is_actionable(to_hero, player) {
            self.reject_item_command(frame, from_hero, 0);
            return;
        }
        let source = self.item_bag(from_hero);
        let Some(source_slot) = source.slot_of(instance_id) else {
            self.reject_item_command(frame, from_hero, 0);
            return;
        };
        let Some(instance) = self.creep_camps.item_instances.get(&instance_id).cloned() else {
            self.reject_item_command(frame, from_hero, 0);
            return;
        };
        let target = self.item_bag(to_hero);
        let Some(target_slot) = target.first_free_slot() else {
            self.reject_item_command(frame, from_hero, 0);
            return;
        };
        if instance.holder != Some(from_hero) || from_hero == to_hero {
            self.reject_item_command(frame, from_hero, 0);
            return;
        }
        let mut source = source;
        let mut target = target;
        source.set(source_slot, None);
        target.set(target_slot, Some(instance_id));
        self.item_shop.bags.insert(from_hero, source);
        self.item_shop.bags.insert(to_hero, target);
        if let Some(state) = self.creep_camps.item_instances.get_mut(&instance_id) {
            state.holder = Some(to_hero);
            state.slot = Some(target_slot);
        }
        self.record_item_fact(
            frame,
            ItemShopFactKind::Transfer,
            from_hero,
            instance_id,
            target_slot,
            to_hero,
            0,
            instance.item_def,
        );
        self.push_event(frame, from_hero, EventKind::ItemTransferred);
    }

    /// 售卖道具（R7.4 / R9.3）：按实例移出六格背包，精确入账冻结回购价。
    fn submit_sell(
        &mut self,
        hero: EntityId,
        instance_id: EntityId,
        shop: u32,
        player: u8,
        frame: Frame,
    ) {
        if !self.is_actionable(hero, player) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let shop_ok =
            self.defs.item_shop().shop(shop).is_some() && self.defs.item_shop().is_shop_def(shop);
        let bag = self.item_bag(hero);
        let Some(slot) = bag.slot_of(instance_id) else {
            self.reject_item_command(frame, hero, shop);
            return;
        };
        let Some(instance) = self.creep_camps.item_instances.get(&instance_id).cloned() else {
            self.reject_item_command(frame, hero, shop);
            return;
        };
        let Some(price) = self
            .defs
            .item_shop()
            .effective_sell_price(instance.item_def)
        else {
            self.reject_item_command(frame, hero, shop);
            return;
        };
        if !shop_ok || instance.holder != Some(hero) {
            self.reject_item_command(frame, hero, shop);
            return;
        }
        // 实例移出背包并销毁（不残留可再售卖），黄金精确入账。
        self.creep_camps.item_instances.remove(&instance_id);
        let mut bag = bag;
        bag.set(slot, None);
        self.item_shop.bags.insert(hero, bag);
        let economy = self.gameplay.players.entry(player).or_default();
        economy.gold = economy.gold.saturating_add(price);
        self.record_item_fact(
            frame,
            ItemShopFactKind::Sell,
            hero,
            instance_id,
            slot,
            EntityId::new(shop_fixture_id(shop)).unwrap_or(EntityId::NONE),
            price,
            instance.item_def,
        );
        self.push_event(frame, hero, EventKind::ItemSold);
    }

    /// 按「友方主城分类」定义层数据驱动选稳定目标（结论 7 / R1.2）。
    /// 按稳定 `EntityId` 顺序取第一个存活、归属本族、被登记为主城分类的实体。
    fn friendly_town_hall_target(&self, player: u8) -> Option<EntityId> {
        for (&entity, &def_id) in &self.item_shop.town_halls {
            if !self.defs.item_shop().is_town_hall_def(def_id) {
                continue;
            }
            if self
                .units
                .get(&entity)
                .is_some_and(|unit| unit.alive && unit.owner == player)
            {
                return Some(entity);
            }
        }
        None
    }

    /// 回城卷轴独立命令（R7.1 / 结论 7）：目标按友方主城分类选出，
    /// 无可用友方主城显式拒单、不扣次数；施法帧 / 目标 / 次数作独立
    /// 权威状态。单英雄传送，MUST NOT 与大法师 MassTeleport 群体传送混淆。
    fn submit_town_portal(
        &mut self,
        hero: EntityId,
        instance_id: EntityId,
        player: u8,
        frame: Frame,
    ) {
        if !self.is_actionable(hero, player) {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let bag = self.item_bag(hero);
        let Some(slot) = bag.slot_of(instance_id) else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let Some(instance) = self.creep_camps.item_instances.get(&instance_id).cloned() else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let constants = self.defs.item_shop().constants;
        let uses = self.item_shop.portal_uses.get(&hero).copied().unwrap_or(0);
        let portal_item = self.defs.item_shop().is_town_portal_item(instance.item_def);
        if instance.holder != Some(hero)
            || !portal_item
            || instance.charges < constants.town_portal_charge_cost
            || instance.cooldown_frames != 0
            || uses >= constants.town_portal_max_uses
            || self.item_shop.casts.contains_key(&hero)
        {
            self.reject_item_command(frame, hero, 0);
            return;
        }
        let Some(target) = self.friendly_town_hall_target(player) else {
            // 无可用友方主城：显式拒单，不扣次数、不生成半传送状态。
            self.record_item_fact(
                frame,
                ItemShopFactKind::TownPortalCancel,
                hero,
                instance_id,
                slot,
                EntityId::NONE,
                0,
                instance.item_def,
            );
            self.push_event(frame, hero, EventKind::TownPortalCancelled);
            return;
        };
        let (target_x, target_y) = self
            .units
            .get(&target)
            .map(|unit| (unit.x.raw(), unit.y.raw()))
            .unwrap_or((0, 0));
        self.item_shop.casts.insert(
            hero,
            TownPortalCast {
                caster: hero,
                instance: instance_id,
                slot,
                target,
                target_x,
                target_y,
                start_frame: frame.value(),
                remaining_frames: constants.town_portal_cast_frames.max(1),
                total_frames: constants.town_portal_cast_frames.max(1),
            },
        );
        self.item_shop
            .portal_uses
            .insert(hero, uses.saturating_add(1));
        self.record_item_fact(
            frame,
            ItemShopFactKind::TownPortalStart,
            hero,
            instance_id,
            slot,
            target,
            0,
            instance.item_def,
        );
        self.push_event(frame, hero, EventKind::TownPortalStarted);
    }

    /// 回城施法取消 / 中断结算（R7.5）：清理施法状态、按冻结定义结算
    /// 次数与充能、不产生半传送位置。
    fn cancel_town_portal(&mut self, hero: EntityId, frame: Frame) {
        let Some(cast) = self.item_shop.casts.remove(&hero) else {
            self.reject_item_command(frame, hero, 0);
            return;
        };
        let cost = self.defs.item_shop().constants.town_portal_charge_cost;
        self.consume_portal_charge(hero, &cast, cost);
        self.record_item_fact(
            frame,
            ItemShopFactKind::TownPortalCancel,
            hero,
            cast.instance,
            cast.slot,
            cast.target,
            0,
            0,
        );
        self.push_event(frame, hero, EventKind::TownPortalCancelled);
    }

    /// 按冻结定义扣回城卷轴充能；消耗品用完移除实例（R7.1 / R7.5）。
    fn consume_portal_charge(&mut self, hero: EntityId, cast: &TownPortalCast, cost: u32) {
        let Some(instance) = self.creep_camps.item_instances.get(&cast.instance).cloned() else {
            return;
        };
        let charges = instance.charges.saturating_sub(cost);
        let consumable = self.defs.item_shop().item_category(instance.item_def)
            == Some(ItemCategory::Consumable);
        if charges == 0 && consumable {
            self.creep_camps.item_instances.remove(&cast.instance);
            let mut bag = self.item_bag(hero);
            bag.set(cast.slot, None);
            self.item_shop.bags.insert(hero, bag);
        } else if let Some(state) = self.creep_camps.item_instances.get_mut(&cast.instance) {
            state.charges = charges;
        }
    }

    /// 回城施法帧阶段：按稳定 `EntityId` 顺序递减整数帧；主城被毁 /
    /// 持有者被打断（阵亡 / 眩晕）显式取消；到 0 完成单英雄整数坐标传送。
    fn advance_town_portals(&mut self, frame: Frame) {
        let heroes: Vec<EntityId> = self.item_shop.casts.keys().copied().collect();
        for hero in heroes {
            let Some(cast) = self.item_shop.casts.get(&hero).copied() else {
                continue;
            };
            let owner = self.units.get(&hero).map(|unit| unit.owner).unwrap_or(0);
            let target_ok = self
                .friendly_town_hall_target(owner)
                .is_some_and(|target| target == cast.target);
            let holder_ok = self.is_actionable(hero, owner);
            if !target_ok || !holder_ok {
                self.cancel_town_portal(hero, frame);
                continue;
            }
            let remaining = cast.remaining_frames.saturating_sub(1);
            if remaining != 0 {
                if let Some(state) = self.item_shop.casts.get_mut(&hero) {
                    state.remaining_frames = remaining;
                }
                continue;
            }
            // 完成：只在完成时写整数坐标，不产生半传送位置（R7.5）。
            self.item_shop.casts.remove(&hero);
            if let Some(unit) = self.units.get_mut(&hero) {
                unit.x = SimFixed::from_raw(cast.target_x);
                unit.y = SimFixed::from_raw(cast.target_y);
                unit.order = Order::Idle;
                unit.pending.clear();
                unit.goal = None;
                unit.path.clear();
                unit.path_index = 0;
                unit.blocked_frames = 0;
                unit.group = 0;
            }
            let cost = self.defs.item_shop().constants.town_portal_charge_cost;
            self.consume_portal_charge(hero, &cast, cost);
            self.record_item_fact(
                frame,
                ItemShopFactKind::TownPortalComplete,
                hero,
                cast.instance,
                cast.slot,
                cast.target,
                0,
                0,
            );
            self.push_event(frame, hero, EventKind::TownPortalCompleted);
        }
    }

    /// 系统级市场刷新通知入队（幂等由 `refresh_batches` 保证，R8.6 / Q6）。
    fn submit_refresh_market(&mut self, shop: u32, batch: u32, player: u8, frame: Frame) {
        let registered =
            self.defs.item_shop().shop(shop).is_some() && self.defs.item_shop().is_shop_def(shop);
        if player != NEUTRAL_OWNER || !registered {
            self.reject_item_command(
                frame,
                EntityId::new(shop_fixture_id(shop)).unwrap_or(EntityId::NONE),
                shop,
            );
            return;
        }
        // 只入队，RNG 消费推迟到帧阶段按稳定顺序执行（ADR 0002 §3-4）。
        self.item_shop
            .pending_refreshes
            .insert((frame.value(), shop, batch));
    }

    /// 市场刷新帧阶段：复用 P8.5c match-owned 整数 LCG 事件驱动流
    /// （`rng_state` / `rng_draws` / `next_rng_seq`）+ 显式消费序列契约，
    /// 按稳定 `(frame, shop, batch)` 顺序消费，同一批次幂等不重复消费。
    fn advance_market_refresh(&mut self, frame: Frame) {
        let due: Vec<(u64, u32, u32)> = self
            .item_shop
            .pending_refreshes
            .iter()
            .copied()
            .filter(|(scheduled, _, _)| *scheduled <= frame.value())
            .collect();
        for key in due {
            self.item_shop.pending_refreshes.remove(&key);
            let (scheduled, shop, batch) = key;
            if self.item_shop.refresh_batches.contains(&(shop, batch)) {
                continue; // 幂等：同批次已消费，不再消耗随机码。
            }
            let Some(shop_def) = self.defs.item_shop().shop(shop).cloned() else {
                continue;
            };
            // 复用同一 match-owned 整数 LCG 事件驱动流（seed 随快照）。
            let rng_before = self.creep_camps.rng_state;
            let rng_after = next_rng(rng_before);
            self.creep_camps.rng_state = rng_after;
            let seq = self.creep_camps.next_rng_seq;
            self.creep_camps.next_rng_seq = seq.saturating_add(1);
            let shelf_ids: Vec<u32> = shop_def.shelf_ids.iter().copied().collect();
            let picked = if shelf_ids.is_empty() {
                None
            } else {
                Some(shelf_ids[(rng_after % shelf_ids.len() as u64) as usize])
            };
            let picked_def =
                picked.and_then(|id| self.defs.item_shop().shelf(id).map(|s| s.item_def));
            self.creep_camps.rng_draws.insert(
                seq,
                RngDraw {
                    seq,
                    frame: scheduled,
                    creep: EntityId::new(shop_fixture_id(shop)).unwrap_or(EntityId::NONE),
                    table: shop,
                    set: batch,
                    rng_before,
                    rng_after,
                    roll: rng_after,
                    picked: picked_def,
                },
            );
            self.item_shop.refresh_draws.insert(
                seq,
                ShopRefreshDraw {
                    seq,
                    frame: scheduled,
                    shop,
                    batch,
                    rng_before,
                    rng_after,
                    roll: rng_after,
                    picked,
                },
            );
            self.item_shop.refresh_batches.insert((shop, batch));
            let count = self
                .item_shop
                .refresh_counts
                .get(&shop)
                .copied()
                .unwrap_or(0);
            self.item_shop
                .refresh_counts
                .insert(shop, count.saturating_add(1));
            // 刷新清单：按稳定货架 id 顺序把库存补到冻结上限（确定性整数）。
            for shelf_id in shelf_ids {
                if let Some(shelf) = self.defs.item_shop().shelf(shelf_id).copied() {
                    self.item_shop.stock.insert(shelf_id, shelf.stock_max);
                }
            }
            self.record_item_fact(
                frame,
                ItemShopFactKind::MarketRefresh,
                EntityId::NONE,
                EntityId::NONE,
                u8::MAX,
                EntityId::new(shop_fixture_id(shop)).unwrap_or(EntityId::NONE),
                0,
                shop,
            );
            self.push_event_raw(frame, shop_fixture_id(shop), EventKind::MarketRefreshed);
        }
    }

    /// P8.5d 帧阶段总入口：购买事务提交 / 回城施法 / 市场刷新按稳定顺序推进。
    fn advance_item_shop(&mut self, frame: Frame) {
        // 阶段顺序固定：所有命令面在阶段 7 已完成提交，此处只执行效果链。
        self.advance_purchases(frame);
        self.advance_town_portals(frame);
        self.advance_market_refresh(frame);
    }

    /// P9 / Q4 族特色机制逐 tick 确定性结算（30 Hz 整数帧）。
    ///
    /// 结算前记录事实序号，结算后把新增事实按稳定 `seq` 顺序转成只读
    /// `EventKind` 事实事件供表现层消费；历史事件 MUST NOT 进权威哈希
    /// （ADR 0002 §5-6）。fog 可见性 MUST 表现层只读派生，本方法
    /// MUST NOT 把 fog 可见性写进 `faction_mechanics`（R10.5 / §0.3 T3）。
    fn advance_faction_mechanics(&mut self, frame: Frame) {
        let before = self.faction_mechanics.next_fact_seq;
        let constants = *self.defs.faction_mechanics();
        self.faction_mechanics.settle(frame.value(), &constants);
        let after = self.faction_mechanics.next_fact_seq;
        let new_facts: Vec<(EntityId, EventKind)> = self
            .faction_mechanics
            .facts
            .range(before..after)
            .map(|(_, fact)| (fact.entity, fact_event_kind(fact.kind)))
            .collect();
        for (entity, kind) in new_facts {
            self.push_event(frame, entity, kind);
        }
    }

    /// 腐地扩张源登记（R4.6）：登记初始腐地格与扩张进度基准。
    /// 命令入口重新校验实体合法性（ADR 0002 §3 执行时复核）。
    fn faction_seed_blight(&mut self, source: EntityId, cell_x: i32, cell_y: i32, frame: Frame) {
        self.faction_mechanics.blight_cells.insert((cell_x, cell_y));
        self.faction_mechanics.blight_progress.insert(source, 0);
        self.push_event(frame, source, EventKind::BlightSeeded);
    }

    /// 尸体登记（`:119`）：登记剩余衰减整数帧。命令未指定（0）时回退到
    /// 冻结常量的尸体衰减时长（合成 `[待核]` 值，数据驱动非白名单）。
    fn faction_raise_corpse(&mut self, corpse: EntityId, decay_frames: u32, frame: Frame) {
        let fallback = self.defs.faction_mechanics().corpse_decay_frames.max(1);
        let frames = if decay_frames == 0 {
            fallback
        } else {
            decay_frames
        };
        self.faction_mechanics.corpses.insert(corpse, frames);
        self.push_event(frame, corpse, EventKind::CorpseRaised);
    }

    /// 食尸回血 / 召唤骷髅（`:119`）：消费已登记尸体并累计次数；
    /// 尸体未登记则显式拒绝、不静默降级。
    fn faction_consume_corpse(&mut self, consumer: EntityId, corpse: EntityId, frame: Frame) {
        if self.faction_mechanics.corpses.remove(&corpse).is_some() {
            let counter = self
                .faction_mechanics
                .corpse_consumptions
                .entry(consumer)
                .or_insert(0);
            *counter = counter.saturating_add(1);
            self.push_event(frame, consumer, EventKind::CorpseConsumed);
        } else {
            self.push_event(frame, consumer, EventKind::FactionMechanicRejected);
        }
    }

    /// 诅咒金矿绑定（R1.5）。
    fn faction_haunt_mine(&mut self, mine: EntityId, frame: Frame) {
        self.faction_mechanics.mines.insert(
            mine,
            MineRecord {
                kind: MineKind::Haunted,
                progress_frames: 0,
            },
        );
        self.push_event(frame, mine, EventKind::GoldMineHaunted);
    }

    /// 缠绕金矿绑定（R1.5）。
    fn faction_entangle_mine(&mut self, mine: EntityId, frame: Frame) {
        self.faction_mechanics.mines.insert(
            mine,
            MineRecord {
                kind: MineKind::Entangled,
                progress_frames: 0,
            },
        );
        self.push_event(frame, mine, EventKind::GoldMineEntangled);
    }

    /// 古树扎根 / 拔根状态切换（R1.2 定义层数据驱动）。
    fn faction_set_ancient_root(&mut self, ancient: EntityId, rooted: bool, frame: Frame) {
        self.faction_mechanics
            .rooted_ancients
            .insert(ancient, rooted);
        self.push_event(frame, ancient, EventKind::AncientRootChanged);
    }

    /// 月亮井登记（R10.3）。命令未指定（0）时回退到冻结常量的月亮井
    /// 单次恢复量（合成 `[待核]` 值，数据驱动非白名单）。
    fn faction_bind_moon_well(&mut self, well: EntityId, restore: i64, frame: Frame) {
        let fallback = self.defs.faction_mechanics().moon_well_restore;
        let amount = if restore == 0 { fallback } else { restore };
        self.faction_mechanics.moon_wells.insert(well, amount);
        self.push_event(frame, well, EventKind::MoonWellBound);
    }

    /// 隐身目标判定登记（R10.4 / S3，玩法机制，非 fog 可见性）。
    fn faction_set_stealth(&mut self, unit: EntityId, cloaked: bool, frame: Frame) {
        let record = self.faction_mechanics.stealth.entry(unit).or_default();
        record.cloaked = cloaked;
        record.night_only = false;
        self.push_event(frame, unit, EventKind::StealthTargetChanged);
    }

    /// 反隐单位登记（R10.4）。
    fn faction_register_detector(&mut self, unit: EntityId, frame: Frame) {
        self.faction_mechanics.detectors.insert(unit);
        self.push_event(frame, unit, EventKind::DetectorRegistered);
    }

    /// 夜间隐身候选登记 / 移除（S3）。
    fn faction_set_night_cloak(&mut self, unit: EntityId, enabled: bool, frame: Frame) {
        if enabled {
            self.faction_mechanics.night_cloak.insert(unit);
            let record = self.faction_mechanics.stealth.entry(unit).or_default();
            record.night_only = true;
        } else {
            self.faction_mechanics.night_cloak.remove(&unit);
            if let Some(record) = self.faction_mechanics.stealth.get_mut(&unit) {
                record.night_only = false;
            }
        }
        self.push_event(frame, unit, EventKind::NightCloakChanged);
    }

    /// 视线阻挡机制输入登记（R5.6，MUST NOT 存 fog 可见性派生结果）。
    fn faction_mark_sight_blocker(&mut self, blocker: EntityId, blocking: bool, frame: Frame) {
        if blocking {
            self.faction_mechanics.sight_blockers.insert(blocker);
        } else {
            self.faction_mechanics.sight_blockers.remove(&blocker);
        }
        self.push_event(frame, blocker, EventKind::SightBlockerChanged);
    }

    /// 野怪睡眠登记（R8.4）。
    fn faction_set_creep_sleep(&mut self, creep: EntityId, sleeps_at_night: bool, frame: Frame) {
        self.faction_mechanics
            .creep_sleep
            .insert(creep, sleeps_at_night);
        self.push_event(frame, creep, EventKind::CreepSleepRegistered);
    }

    /// 回血条件定义层枚举登记（R10.3，非族名硬编码）。
    fn faction_set_regen_condition(
        &mut self,
        unit: EntityId,
        condition: RegenCondition,
        frame: Frame,
    ) {
        self.faction_mechanics.regen.insert(
            unit,
            RegenRecord {
                condition,
                accumulated: 0,
            },
        );
        self.push_event(frame, unit, EventKind::RegenConditionChanged);
    }

    fn gather(&mut self, unit: EntityId, player: u8) {
        if !self.is_actionable(unit, player) {
            return;
        }
        if let Some(state) = self.units.get_mut(&unit) {
            state.gameplay.gathered = state.gameplay.gathered.saturating_add(1);
        }
        let economy = self.gameplay.players.entry(player).or_default();
        economy.gold = economy.gold.saturating_add(1);
    }

    fn register_behavior(&mut self, id: DefId, effect: EffectDef) {
        self.gameplay.behaviors.insert(
            id,
            BehaviorSpec {
                id,
                extra_effect: effect,
            },
        );
    }

    fn spawn_wave(&mut self, id: u32, period: u32) {
        let period = period.max(1);
        self.gameplay.waves.insert(
            id,
            WaveState {
                period,
                remaining: period,
                count: 0,
            },
        );
    }

    fn set_ai(&mut self, enabled: bool, frame: Frame) {
        self.gameplay.ai.enabled = enabled;
        self.gameplay.ai.decision_period = 10;
        self.gameplay.ai.next_decision_frame = frame.value().saturating_add(10);
    }

    /// 离开 / House 销毁清理的唯一所有者，不产生竞争状态。
    fn leave(&mut self, player: u8) {
        let economy = self.gameplay.players.entry(player).or_default();
        economy.active = false;
        economy.research = None;
    }

    /// 队列命令与 AI 共用的执行入口。
    fn apply_gameplay(&mut self, command: GameplayCommand, frame: Frame) {
        let player = command.player_id;
        match command.kind {
            GameplayCommandKind::Cast {
                caster,
                ability,
                target,
            } => self.execute_cast(
                caster,
                ability,
                AbilityTargetKind::Entity(target),
                player,
                frame,
            ),
            GameplayCommandKind::Train {
                producer,
                unit_def,
                is_building,
            } => self.enqueue_production(producer, unit_def, is_building, player),
            GameplayCommandKind::CancelTrain { producer, index } => {
                self.cancel_train(producer, index)
            }
            GameplayCommandKind::Research { upgrade } => self.research(player, upgrade),
            // P8.5d：旧 `BuyItem` 占位保留码 5，但显式拒单、不写权威购买状态。
            GameplayCommandKind::BuyItem { unit, item } => self.buy_item(unit, item, frame),
            GameplayCommandKind::Gather { unit } => self.gather(unit, player),
            GameplayCommandKind::RegisterBehavior { id, effect } => {
                self.register_behavior(id, effect)
            }
            GameplayCommandKind::SpawnWave { id, period } => self.spawn_wave(id, period),
            GameplayCommandKind::SetAi { enabled } => self.set_ai(enabled, frame),
            GameplayCommandKind::Leave { player } => self.leave(player),
            GameplayCommandKind::ReviveHero {
                hero,
                altar,
                altar_def,
            } => self.submit_revive(hero, altar, altar_def, player),
            GameplayCommandKind::CancelRevive { hero } => self.cancel_revive(hero, player),
            GameplayCommandKind::UpgradeTownHall {
                town_hall,
                town_hall_def,
                target_tier,
                cancel,
            } => self.apply_town_hall_upgrade(
                town_hall,
                town_hall_def,
                target_tier,
                cancel,
                player,
                frame,
            ),
            // P8.5c：frame-0 系统级营地装配命令（Q5 / D6），与击杀 / 掉落
            // 的 `step()` 内部被动系统解耦；同源校验在 `seed_creep_camp`。
            GameplayCommandKind::SeedCreepCamp { camp, script } => {
                self.seed_creep_camp(camp, script, player, frame)
            }
            // P8.5d：拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖 / 回城 / 刷新。
            // 全部经同一公开命令面按冻结整数帧号入队、同源校验、稳定排序。
            GameplayCommandKind::PickupGroundItem { hero, ground_item } => {
                self.submit_pickup(hero, ground_item, player, frame)
            }
            GameplayCommandKind::BuyFromShop { hero, shop, shelf } => {
                self.submit_purchase(hero, shop, shelf, player, frame)
            }
            GameplayCommandKind::UseItem {
                hero,
                instance,
                target,
            } => self.submit_use_item(hero, instance, target, player, frame),
            GameplayCommandKind::DropItem { hero, instance } => {
                self.submit_drop(hero, instance, player, frame)
            }
            GameplayCommandKind::TransferItem {
                from_hero,
                to_hero,
                instance,
            } => self.submit_transfer(from_hero, to_hero, instance, player, frame),
            GameplayCommandKind::SellItem {
                hero,
                instance,
                shop,
            } => self.submit_sell(hero, instance, shop, player, frame),
            GameplayCommandKind::UseTownPortal { hero, instance } => {
                self.submit_town_portal(hero, instance, player, frame)
            }
            GameplayCommandKind::CancelTownPortal { hero } => self.cancel_town_portal(hero, frame),
            GameplayCommandKind::RefreshMarket { shop, batch } => {
                self.submit_refresh_market(shop, batch, player, frame)
            }
            // P9 / Q2 / Q4 新增变体穷尽分发：每个变体 MUST 有独立分支，
            // 无遗漏默认放行（ADR 0002 §3 / Q5 / 反向验证形态①）。
            // 全部经同一公开命令面按冻结整数帧号稳定排序执行。
            GameplayCommandKind::SeedBlight {
                source,
                cell_x,
                cell_y,
            } => self.faction_seed_blight(source, cell_x, cell_y, frame),
            GameplayCommandKind::RaiseCorpse {
                corpse,
                decay_frames,
            } => self.faction_raise_corpse(corpse, decay_frames, frame),
            GameplayCommandKind::ConsumeCorpse { consumer, corpse } => {
                self.faction_consume_corpse(consumer, corpse, frame)
            }
            GameplayCommandKind::HauntGoldMine { mine } => self.faction_haunt_mine(mine, frame),
            GameplayCommandKind::EntangleGoldMine { mine } => {
                self.faction_entangle_mine(mine, frame)
            }
            GameplayCommandKind::SetAncientRoot { ancient, rooted } => {
                self.faction_set_ancient_root(ancient, rooted, frame)
            }
            GameplayCommandKind::BindMoonWell { well, restore } => {
                self.faction_bind_moon_well(well, restore, frame)
            }
            GameplayCommandKind::SetStealth { unit, cloaked } => {
                self.faction_set_stealth(unit, cloaked, frame)
            }
            GameplayCommandKind::RegisterDetector { unit } => {
                self.faction_register_detector(unit, frame)
            }
            GameplayCommandKind::SetNightCloak { unit, enabled } => {
                self.faction_set_night_cloak(unit, enabled, frame)
            }
            GameplayCommandKind::MarkSightBlocker { blocker, blocking } => {
                self.faction_mark_sight_blocker(blocker, blocking, frame)
            }
            GameplayCommandKind::SetCreepSleep {
                creep,
                sleeps_at_night,
            } => self.faction_set_creep_sleep(creep, sleeps_at_night, frame),
            GameplayCommandKind::SetRegenCondition { unit, condition } => {
                self.faction_set_regen_condition(unit, condition, frame)
            }
            // P11 新增变体穷尽分发：每个变体 MUST 有独立分支，无默认放行。
            // 全部经同一公开命令面按冻结整数帧号稳定排序执行。
            GameplayCommandKind::CastPoint {
                caster,
                ability,
                x,
                y,
            } => self.execute_cast(
                caster,
                ability,
                AbilityTargetKind::Point { x, y },
                player,
                frame,
            ),
            GameplayCommandKind::CastArea {
                caster,
                ability,
                x,
                y,
                radius_raw,
            } => self.execute_cast(
                caster,
                ability,
                AbilityTargetKind::Area { x, y, radius_raw },
                player,
                frame,
            ),
            GameplayCommandKind::CastDirection {
                caster,
                ability,
                dx,
                dy,
            } => self.execute_cast(
                caster,
                ability,
                AbilityTargetKind::Direction { dx, dy },
                player,
                frame,
            ),
            GameplayCommandKind::LearnAbility { hero, ability } => {
                self.learn_ability(hero, ability, player)
            }
            GameplayCommandKind::LevelUpHero { hero } => self.level_up_hero(hero, player),
        }
    }

    /// 事务性退款：把冻结扣费登记精确回退到玩家经济，移除队列项与费用登记，
    /// 死亡登记保留（恢复到「可再次复活」），不生成半复活实体、不重复扣费。
    /// refund 与 charge 同样通过 `PlayerEconomy` 与三组状态进入哈希 / 快照。
    fn refund_revive(&mut self, hero: EntityId) {
        if let Some(charge) = self.revive.charges.remove(&hero) {
            let economy = self.gameplay.players.entry(charge.owner).or_default();
            economy.gold = economy.gold.saturating_add(charge.gold);
            economy.lumber = economy.lumber.saturating_add(charge.lumber);
        }
        self.revive.queue.remove(&hero);
    }

    /// 英雄阵亡登记：写入整数帧号 + 状态快照，并发事实事件。
    /// 若该英雄仍在复活队列中（再次死亡），先事务性退款再重新登记。
    fn register_hero_death(
        &mut self,
        hero: EntityId,
        frame: Frame,
        gameplay: UnitGameplayState,
        owner: u8,
        max_hp: i64,
    ) {
        self.refund_revive(hero);
        // P8.5d：英雄阵亡时把六格定长实例背包移入 `death_bags`，
        // 按实例 `EntityId` 身份保留（充能 / 冷却不刷新），复活时逐项恢复
        // （R6.3 / R7.5）；MUST NOT 回退为 DefId 序列。
        if let Some(bag) = self.item_shop.bags.remove(&hero) {
            self.item_shop.death_bags.insert(hero, bag);
        }
        self.revive.deaths.insert(
            hero,
            HeroDeathRecord {
                death_frame: frame.value(),
                owner,
                gameplay,
                max_hp,
            },
        );
        self.push_event(frame, hero, EventKind::HeroDied);
    }

    /// 费用提交事务（与队列效果执行链分离）：校验定义层祭坛类别、实体存活 /
    /// 归属、死亡登记与资源充足，确定性扣减 gold / lumber，冻结扣费登记并入队。
    /// 任一校验不通过即显式拒绝（no-op），MUST NOT 静默降级为免费复活。
    fn submit_revive(&mut self, hero: EntityId, altar: EntityId, altar_def: DefId, player: u8) {
        // 定义层类别驱动：非建筑名字符串白名单，任何被登记的 DefId 都可复活。
        if !self.defs.is_altar(altar_def) {
            return;
        }
        // 祭坛实体存活且归属本族。
        if !self
            .units
            .get(&altar)
            .is_some_and(|unit| unit.alive && unit.owner == player)
        {
            return;
        }
        // 英雄实体阵亡、归属本族、已登记死亡，且不在复活队列（不重复扣费）。
        let Some(unit) = self.units.get(&hero) else {
            return;
        };
        if unit.alive || unit.owner != player || unit.gameplay.hero.is_none() {
            return;
        }
        if self.revive.queue.contains_key(&hero) || !self.revive.deaths.contains_key(&hero) {
            return;
        }
        let level = self
            .revive
            .deaths
            .get(&hero)
            .and_then(|record| record.gameplay.hero.map(|state| state.level))
            .unwrap_or(1);
        let cost = self.defs.revive_cost(level);
        {
            let economy = self.gameplay.players.entry(player).or_default();
            // 资源不足显式拒绝：不改变任何权威状态，不免费复活。
            if economy.gold < cost.gold || economy.lumber < cost.lumber {
                return;
            }
            if economy.supply_used.saturating_add(cost.food) > economy.supply_cap {
                return;
            }
            // 冻结扣费：确定性扣减，退款以 charges 登记为精确逆运算依据。
            economy.gold -= cost.gold;
            economy.lumber -= cost.lumber;
        }
        self.revive.charges.insert(
            hero,
            ReviveCharge {
                owner: player,
                gold: cost.gold,
                lumber: cost.lumber,
            },
        );
        let frames = cost.frames.max(1);
        self.revive.queue.insert(
            hero,
            ReviveQueueItem {
                hero,
                altar,
                altar_def,
                owner: player,
                remaining_frames: frames,
                total_frames: frames,
                gold: cost.gold,
                lumber: cost.lumber,
            },
        );
    }

    /// 取消复活队列项并事务性退款（同源归属校验）。
    fn cancel_revive(&mut self, hero: EntityId, player: u8) {
        let owner_matches = self
            .revive
            .queue
            .get(&hero)
            .is_some_and(|item| item.owner == player);
        if owner_matches {
            self.refund_revive(hero);
        }
    }

    /// 队列效果执行链（与费用提交事务分离）：按稳定 `EntityId` 顺序递减剩余
    /// 帧数；祭坛被毁 / 对局失败 / 登记丢失则事务性退款；到 0 恢复本体。
    fn advance_revive_queue(&mut self, frame: Frame) {
        let ids: Vec<EntityId> = self.revive.queue.keys().copied().collect();
        for hero in ids {
            let Some(item) = self.revive.queue.get(&hero).cloned() else {
                continue;
            };
            let altar_ok = self
                .units
                .get(&item.altar)
                .is_some_and(|unit| unit.alive && unit.owner == item.owner);
            let player_active = self
                .gameplay
                .players
                .get(&item.owner)
                .map(|economy| economy.active)
                .unwrap_or(true);
            let registered = self.revive.deaths.contains_key(&hero)
                && self.units.get(&hero).is_some_and(|unit| !unit.alive);
            if !altar_ok || !registered || !player_active || self.gameplay.victory.resolved {
                self.refund_revive(hero);
                continue;
            }
            let remaining = item.remaining_frames.saturating_sub(1);
            if remaining == 0 {
                self.complete_revive(hero, &item, frame);
            } else if let Some(queue_item) = self.revive.queue.get_mut(&hero) {
                queue_item.remaining_frames = remaining;
            }
        }
    }

    /// 复活完成：恢复同一个英雄实体（同一 `EntityId`），MUST NOT `spawn()`
    /// 新实体、MUST NOT 得到 `hero=None` 空背包。等级 / 经验 / 技能登记
    /// 来自死亡登记快照，六格定长实例背包由 `ItemShopState.death_bags`
    /// 按实例 `EntityId` 逐项恢复（含剩余充能 / 冷却，R6.3 / R7.5）；
    /// 按冻结常量恢复生命 / 法力，改到祭坛位置。
    fn complete_revive(&mut self, hero: EntityId, item: &ReviveQueueItem, frame: Frame) {
        let Some(death) = self.revive.deaths.remove(&hero) else {
            return;
        };
        self.revive.queue.remove(&hero);
        // 费用已消耗：移除扣费登记，不退款、不重复扣费。
        self.revive.charges.remove(&hero);
        let Some((altar_x, altar_y)) = self.units.get(&item.altar).map(|unit| (unit.x, unit.y))
        else {
            return;
        };
        let constants = *self.defs.revive_constants();
        let Some(unit) = self.units.get_mut(&hero) else {
            return;
        };
        unit.alive = true;
        unit.hp = restore_by_percent(death.max_hp, constants.hp_restore_percent);
        // 逐项恢复：hero.level / hero.xp、cooldowns（技能登记）、Buff 等。
        unit.gameplay = death.gameplay;
        unit.gameplay.mana =
            restore_by_percent(unit.gameplay.max_mana, constants.mana_restore_percent);
        // P8.5d：六格实例背包按实例 EntityId 逐项恢复，MUST NOT 回退为
        // `bag: Vec<DefId>` 的 DefId 序列；实例 holder / slot 同步回填。
        if let Some(bag) = self.item_shop.death_bags.remove(&hero) {
            for (slot, slot_instance) in bag.slots.iter().enumerate() {
                if let Some(instance_id) = slot_instance {
                    if let Some(instance) = self.creep_camps.item_instances.get_mut(instance_id) {
                        instance.holder = Some(hero);
                        instance.slot = Some(slot as u8);
                    }
                }
            }
            self.item_shop.bags.insert(hero, bag);
        }
        unit.x = altar_x;
        unit.y = altar_y;
        // 沿用阵亡时的订单 / 路径清理语义，不删实体、不判负。
        unit.order = Order::Idle;
        unit.pending.clear();
        unit.goal = None;
        unit.path.clear();
        unit.path_index = 0;
        unit.blocked_frames = 0;
        unit.group = 0;
        self.push_event(frame, hero, EventKind::Revived);
    }

    // -----------------------------------------------------------------------
    // P8.5b 主城升级（D1 / D4 / Q2 / R-2）
    // -----------------------------------------------------------------------

    /// 玩家当前主城等级：按稳定 `EntityId` 顺序取本人存活主城实体的最高 tier，
    /// 未登记任何主城 tiers 时视为 T1。只读定义层类别 + tier 权威状态，
    /// 不查建筑名字符串 / 硬编码 DefId（R1.2 / R13.3 / D2）。
    fn current_town_hall_tier(&self, player: u8) -> TownHallTier {
        let mut best = TownHallTier::T1;
        for (&entity, &tier) in &self.town_hall.tiers {
            if tier > best
                && self
                    .units
                    .get(&entity)
                    .is_some_and(|unit| unit.alive && unit.owner == player)
            {
                best = tier;
            }
        }
        best
    }

    /// 定义层 tier 需求链前置判据：某单位 / 科技 DefId 是否被当前主城等级解锁。
    /// 无需求登记（`None`）视为 T1 即可训练 / 研发；有需求则须当前 tier ≥ 需求。
    fn tier_requirement_met(&self, player: u8, def_id: DefId) -> bool {
        match self.defs.tier_requirement(def_id) {
            Some(required) => self.current_town_hall_tier(player) >= required,
            None => true,
        }
    }

    /// 事务性退款：把冻结扣费登记精确回退到玩家经济，移除升级队列项与登记，
    /// 不产生半升级实体、不重复扣费。退款与扣费同样经 `PlayerEconomy` 与
    /// 三组权威状态进入哈希 / 快照（D4）。
    fn refund_town_hall_upgrade(&mut self, town_hall: EntityId) {
        if let Some(charge) = self.town_hall.charges.remove(&town_hall) {
            let economy = self.gameplay.players.entry(charge.owner).or_default();
            economy.gold = economy.gold.saturating_add(charge.gold);
            economy.lumber = economy.lumber.saturating_add(charge.lumber);
        }
        self.town_hall.queue.remove(&town_hall);
    }

    /// 升级 / 取消升级命令面：按 `cancel` 分流两种动作（R-2 单命令参数表达）。
    /// 费用提交事务在本阶段完成，队列效果执行链在 `advance_town_hall_queue`
    /// 分离处理（主 gameplay-systems/spec.md:19）。
    fn apply_town_hall_upgrade(
        &mut self,
        town_hall: EntityId,
        town_hall_def: DefId,
        target_tier: TownHallTier,
        cancel: bool,
        player: u8,
        frame: Frame,
    ) {
        if cancel {
            // 玩家主动取消：校验队列项归属后事务性退款并发事实事件。
            let owner_matches = self
                .town_hall
                .queue
                .get(&town_hall)
                .is_some_and(|item| item.owner == player);
            if owner_matches {
                self.refund_town_hall_upgrade(town_hall);
                self.push_event(frame, town_hall, EventKind::TownHallUpgradeCancelled);
            }
            return;
        }
        self.submit_town_hall_upgrade(town_hall, town_hall_def, target_tier, player, frame);
    }

    /// 升级费用提交事务：校验定义层主城类别、实体存活 / 归属、逐级递进、
    /// 无进行中升级、资源充足；确定性扣减 gold / lumber、冻结扣费登记并入队。
    /// 任一校验不通过即显式拒绝（no-op），MUST NOT 静默降级为免费升级。
    fn submit_town_hall_upgrade(
        &mut self,
        town_hall: EntityId,
        town_hall_def: DefId,
        target_tier: TownHallTier,
        player: u8,
        frame: Frame,
    ) {
        // 定义层类别驱动：只查登记的类别集合，MUST NOT 用名字白名单 / 硬编码 id。
        if !self.defs.is_town_hall(town_hall_def) {
            return;
        }
        // 主城实体存活且归属本族。
        if !self
            .units
            .get(&town_hall)
            .is_some_and(|unit| unit.alive && unit.owner == player)
        {
            return;
        }
        // 无进行中升级（不重复扣费）。
        if self.town_hall.queue.contains_key(&town_hall) {
            return;
        }
        // 逐级递进：目标必须是当前登记 tier 的下一级（T1→T2→T3，禁止跳级）。
        let current = self
            .town_hall
            .tiers
            .get(&town_hall)
            .copied()
            .unwrap_or(TownHallTier::T1);
        if current.successor() != Some(target_tier) {
            return;
        }
        let Some(cost) = self.defs.town_hall_constants().cost(target_tier) else {
            return;
        };
        // 资源不足显式拒绝：先用只读 get 校验，绝不 `entry().or_default()`
        // 隐式创建玩家记录，确保「显式失败不产生权威状态漂移」。
        let affordable = self
            .gameplay
            .players
            .get(&player)
            .is_some_and(|economy| economy.gold >= cost.gold && economy.lumber >= cost.lumber);
        if !affordable {
            return;
        }
        {
            // 冻结扣费：确定性扣减，退款以 charges 登记为精确逆运算依据。
            let economy = self
                .gameplay
                .players
                .get_mut(&player)
                .expect("town_hall: reason=player_present_after_affordability_check");
            economy.gold -= cost.gold;
            economy.lumber -= cost.lumber;
        }
        self.town_hall.charges.insert(
            town_hall,
            TownHallUpgradeCharge {
                owner: player,
                gold: cost.gold,
                lumber: cost.lumber,
            },
        );
        let frames = cost.frames.max(1);
        self.town_hall.queue.insert(
            town_hall,
            TownHallUpgradeItem {
                town_hall,
                town_hall_def,
                owner: player,
                from_tier: current,
                target_tier,
                remaining_frames: frames,
                total_frames: frames,
                gold: cost.gold,
                lumber: cost.lumber,
            },
        );
        self.push_event(frame, town_hall, EventKind::TownHallUpgradeStarted);
    }

    /// 升级队列效果执行链（与费用提交事务分离）：按稳定 `EntityId` 顺序递减
    /// 剩余整数帧；主城被毁 / 对局失败 / 玩家失活 / 登记丢失则事务性退款；
    /// 到 0 完成 tier 递进，不生成半升级实体（D4）。
    fn advance_town_hall_queue(&mut self, frame: Frame) {
        let ids: Vec<EntityId> = self.town_hall.queue.keys().copied().collect();
        for town_hall in ids {
            let Some(item) = self.town_hall.queue.get(&town_hall).cloned() else {
                continue;
            };
            let town_hall_ok = self
                .units
                .get(&town_hall)
                .is_some_and(|unit| unit.alive && unit.owner == item.owner);
            let player_active = self
                .gameplay
                .players
                .get(&item.owner)
                .map(|economy| economy.active)
                .unwrap_or(true);
            let charged = self.town_hall.charges.contains_key(&town_hall);
            if !town_hall_ok || !player_active || !charged || self.gameplay.victory.resolved {
                self.refund_town_hall_upgrade(town_hall);
                self.push_event(frame, town_hall, EventKind::TownHallUpgradeCancelled);
                continue;
            }
            let remaining = item.remaining_frames.saturating_sub(1);
            if remaining == 0 {
                self.complete_town_hall_upgrade(town_hall, &item, frame);
            } else if let Some(queue_item) = self.town_hall.queue.get_mut(&town_hall) {
                queue_item.remaining_frames = remaining;
            }
        }
    }

    /// 升级完成：同一主城实体的 tier 递进（MUST NOT `spawn` 新实体），
    /// 费用已消耗故移除扣费登记（不退款、不重复扣费），发完成事实事件。
    /// 解锁在同一确定性命令面立即生效（下一次 `Train` / `Research` 读新 tier）。
    fn complete_town_hall_upgrade(
        &mut self,
        town_hall: EntityId,
        item: &TownHallUpgradeItem,
        frame: Frame,
    ) {
        self.town_hall.queue.remove(&town_hall);
        self.town_hall.charges.remove(&town_hall);
        self.town_hall.tiers.insert(town_hall, item.target_tier);
        self.push_event(frame, town_hall, EventKind::TownHallUpgradeCompleted);
    }

    /// 推进全部可保存运行态；完成项在稳定顺序下统一提交。
    fn advance_gameplay_timers(&mut self, frame: Frame) {
        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        let mut finished_casts: Vec<(EntityId, DefId, EntityId)> = Vec::new();
        let mut finished_production: Vec<(EntityId, u8, DefId, bool)> = Vec::new();

        for &id in &ids {
            let Some(unit) = self.units.get_mut(&id) else {
                continue;
            };
            // 施法 / 引导完成。
            let cast_done = match unit.gameplay.cast.as_mut() {
                Some(cast) => {
                    cast.remaining_frames = cast.remaining_frames.saturating_sub(1);
                    (cast.remaining_frames == 0).then_some((cast.ability, cast.target))
                }
                None => None,
            };
            if let Some((ability, target)) = cast_done {
                unit.gameplay.cast = None;
                finished_casts.push((id, ability, target));
            }
            // 冷却递减。
            let cooldowns: Vec<DefId> = unit.gameplay.cooldowns.keys().copied().collect();
            for ability in cooldowns {
                let expired = unit
                    .gameplay
                    .cooldowns
                    .get_mut(&ability)
                    .is_some_and(|remaining| {
                        *remaining = remaining.saturating_sub(1);
                        *remaining == 0
                    });
                if expired {
                    unit.gameplay.cooldowns.remove(&ability);
                }
            }
            // Buff 到期。
            for buff in &mut unit.gameplay.buffs {
                buff.remaining = buff.remaining.saturating_sub(1);
            }
            unit.gameplay.buffs.retain(|buff| buff.remaining > 0);
            // 生产 / 建造队列。
            let owner = unit.owner;
            let mut produced = Vec::new();
            for item in &mut unit.gameplay.production {
                item.remaining_frames = item.remaining_frames.saturating_sub(1);
                if item.remaining_frames == 0 {
                    produced.push((item.unit_def, item.is_building));
                }
            }
            unit.gameplay
                .production
                .retain(|item| item.remaining_frames > 0);
            for (unit_def, is_building) in produced {
                finished_production.push((id, owner, unit_def, is_building));
            }
        }

        // 施法完成 → 效果链（统一结算，不重复扣费）。待定施法优先取
        // `pending_casts` 中的冻结目标形态，恢复快照后仍不重复施放。
        for (caster, ability, target) in finished_casts {
            let pending = self.gameplay.abilities.pending_casts.remove(&caster);
            let owner = self.units.get(&caster).map(|unit| unit.owner).unwrap_or(0);
            let context = match pending {
                Some(pending) => AbilityEffectContext {
                    caster,
                    ability: pending.ability,
                    target: pending.target,
                    player: pending.player,
                    frame: frame.value(),
                },
                None => AbilityEffectContext {
                    caster,
                    ability,
                    target: AbilityTargetKind::Entity(target),
                    player: owner,
                    frame: frame.value(),
                },
            };
            self.apply_ability_effects(context);
        }
        // 生产完成 → 生成单位（稳定 ID 顺序）。
        for (producer, owner, unit_def, is_building) in finished_production {
            self.complete_production(producer, owner, unit_def, is_building);
        }

        // 科技研究推进（按稳定玩家顺序）。
        let players: Vec<u8> = self.gameplay.players.keys().copied().collect();
        let mut completed_upgrades: Vec<(u8, DefId)> = Vec::new();
        for player in players {
            if let Some(economy) = self.gameplay.players.get_mut(&player) {
                if let Some((upgrade, remaining)) = economy.research {
                    let remaining = remaining.saturating_sub(1);
                    if remaining == 0 {
                        economy.research = None;
                        economy.upgrades.insert(upgrade, true);
                        completed_upgrades.push((player, upgrade));
                    } else {
                        economy.research = Some((upgrade, remaining));
                    }
                }
            }
        }
        for (player, upgrade) in completed_upgrades {
            self.apply_upgrade(player, upgrade);
        }

        // 波次与光环（稳定 ID 顺序）。
        let wave_ids: Vec<u32> = self.gameplay.waves.keys().copied().collect();
        for id in wave_ids {
            if let Some(wave) = self.gameplay.waves.get_mut(&id) {
                wave.remaining = wave.remaining.saturating_sub(1);
                if wave.remaining == 0 {
                    wave.remaining = wave.period;
                    wave.count = wave.count.saturating_add(1);
                }
            }
        }
        self.gameplay.auras.retain(|_, aura| {
            aura.remaining = aura.remaining.saturating_sub(1);
            aura.remaining > 0
        });
    }

    fn complete_production(
        &mut self,
        producer: EntityId,
        owner: u8,
        unit_def: DefId,
        is_building: bool,
    ) {
        let Some(def) = self.defs.units().get(&unit_def).copied() else {
            return;
        };
        let (x, y) = self
            .units
            .get(&producer)
            .map(|unit| (unit.x, unit.y))
            .unwrap_or((SimFixed::ZERO, SimFixed::ZERO));
        let id = self.entities.spawn();
        let mut unit = UnitState::new(
            x,
            y,
            owner,
            def.hp,
            SimFixed::from_raw(def.speed_raw),
            def.damage,
            SimFixed::from_raw(def.range_raw),
        );
        unit.gameplay.max_mana = def.mana;
        unit.gameplay.mana = def.mana;
        // 定义层英雄类别驱动：让英雄经既有 `Train` 生产路径诞生，
        // `HeroState` 不再是死字段。未登记英雄类别的既有定义行为不变。
        if self.defs.is_hero(unit_def) {
            unit.gameplay.hero = Some(HeroState {
                level: HERO_SPAWN_LEVEL,
                xp: HERO_SPAWN_XP,
            });
            // P11：英雄初始技能点进入权威运行态，恢复后不重复发放。
            self.gameplay
                .abilities
                .hero_skill_points
                .insert(id, HERO_SPAWN_LEVEL.max(1));
        }
        self.units.insert(id, unit);
        // P8.5d：友方主城分类登记由定义层数据驱动（结论 7 / R1.2），
        // 只按冻结 DefId 类别判定，MUST NOT 用建筑名白名单或 rawcode。
        if self.defs.item_shop().is_town_hall_def(unit_def) {
            self.item_shop.town_halls.insert(id, unit_def);
        }
        let economy = self.gameplay.players.entry(owner).or_default();
        economy.produced = economy.produced.saturating_add(1);
        economy.supply_used = economy.supply_used.saturating_add(def.supply);
        if !is_building {
            economy.supply_cap = economy.supply_cap.saturating_add(def.supply);
        }
    }

    fn apply_upgrade(&mut self, player: u8, upgrade: DefId) {
        let Some(def) = self.defs.upgrades().get(&upgrade).copied() else {
            return;
        };
        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        for id in ids {
            let Some(unit) = self.units.get_mut(&id) else {
                continue;
            };
            if unit.owner != player || !unit.alive {
                continue;
            }
            // 属性修正统一走结算服务，不直接改权威字段。
            Settlement::apply_stat_mod(&mut Self::make_vitals(unit), def.damage_mod, def.hp_mod);
        }
    }

    /// 胜负在既定帧阶段按稳定顺序只结算一次。
    fn resolve_victory(&mut self, frame: Frame) {
        if self.gameplay.victory.resolved || self.gameplay.players.len() < 2 {
            return;
        }
        let active: Vec<u8> = self
            .gameplay
            .players
            .iter()
            .filter(|(_, economy)| economy.active)
            .map(|(player, _)| *player)
            .collect();
        if active.len() == 1 {
            self.gameplay.victory = VictoryState {
                resolved: true,
                winner: Some(active[0]),
                resolved_frame: frame.value(),
            };
        }
    }

    /// 确定性 AI：只作用于仍空闲单位，不覆盖仍有效的玩家命令；
    /// AI 命令经与玩家相同的 `validate_gameplay_command` 校验入口，但绝不写回
    /// 命令队列，因此完整重放只注入外部命令、不会双重执行内部 AI 命令。
    fn run_ai(&mut self, frame: Frame) {
        if !self.gameplay.ai.enabled || frame.value() < self.gameplay.ai.next_decision_frame {
            return;
        }
        let period = self.gameplay.ai.decision_period.max(1) as u64;
        self.gameplay.ai.next_decision_frame = frame.value().saturating_add(period);

        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        // 目标过滤同样走统一结算服务。
        let alive = Settlement::filter_targets(&ids, |id| self.is_alive(id));
        let idle: Vec<(EntityId, u8)> = alive
            .into_iter()
            .filter_map(|id| {
                self.units
                    .get(&id)
                    .filter(|unit| unit.order == Order::Idle)
                    .map(|unit| (id, unit.owner))
            })
            .collect();
        for (id, owner) in idle {
            let command =
                GameplayCommand::new(frame, owner, 0, GameplayCommandKind::Gather { unit: id });
            // 与 `submit_gameplay_command` 共用同一校验入口，只是不入队、不注入重放。
            if self.validate_gameplay_command(&command).is_ok() {
                self.apply_gameplay(command, frame);
            }
        }
    }

    // -----------------------------------------------------------------------
    // P8.5c 野怪营地与掉落（Q1 / Q6 / Q7 / D1–D4 / R8.1–R8.6）
    // -----------------------------------------------------------------------

    /// 只读 owned 拷贝：野怪营地与掉落的全部权威状态（ADR 0002 §5）。
    /// 表现层可由本视图 + 事实事件 + `Snapshot.creep_camps` 派生 HUD，
    /// MUST NOT 反写。`Event` 不新增 payload 字段以免破坏下游字面量构造，
    /// `Event.entity` 携带营地 / 野怪 / 地面物品 id 供查表（Q5 / D6 / C9）。
    pub fn creep_camps(&self) -> CreepCampState {
        self.creep_camps.clone()
    }

    /// 共享结算服务的死亡通知入口（Q6 / tasks 4.2，幂等）。
    /// 同一野怪重复投递只登记一次，返回是否首次登记；使用当前整数逻辑
    /// 帧号，MUST NOT 接受外部帧号 / 墙钟 / 表现层随机源（ADR 0002 §2-4）。
    pub fn notify_creep_death(
        &mut self,
        creep: EntityId,
        killer: EntityId,
        killer_owner: u8,
    ) -> bool {
        if !self.creep_camps.creeps.contains_key(&creep) {
            return false;
        }
        if self.creep_camps.death_processed.contains(&creep)
            || self.creep_camps.pending_deaths.contains_key(&creep)
        {
            return false;
        }
        let frame = self.clock.current();
        self.creep_camps.pending_deaths.insert(
            creep,
            CreepDeathNotice {
                killer,
                killer_owner,
                frame: frame.value(),
            },
        );
        self.push_event(frame, creep, EventKind::CreepDied);
        true
    }

    /// frame-0 系统级确定性命令：按冻结 `CampScript` 装配营地与野怪（Q5 / Q7）。
    /// 同源校验：只接受 `NEUTRAL_OWNER`；冻结脚本 / 中立类别 / 掉落表数据
    /// 驱动，MUST NOT 用名字白名单或硬编码 DefId（R1.2 / R13.3 / tasks 2.3）。
    /// 野怪 MUST NOT 挂现有玩家 owner 下 spawn（R8.1 / Q7）。
    fn seed_creep_camp(&mut self, camp: u32, script_id: u32, player: u8, frame: Frame) {
        if player != NEUTRAL_OWNER || camp == 0 || script_id == 0 {
            return;
        }
        // 幂等：重放 / 重复提交 MUST NOT 双重注入（R12.4 / tasks 6.1）。
        if self.creep_camps.camps.contains_key(&camp) {
            return;
        }
        let Some(script) = self.defs.camp_script(script_id).cloned() else {
            return;
        };
        if self.defs.drop_table(script.drop_table).is_none() {
            return;
        }
        // 定义层类别驱动：每个野怪 DefId 都必须登记为中立敌对且有单位定义。
        let mut spawn_defs = Vec::new();
        for &def_id in &script.creep_defs {
            let Some(def) = self.defs.units().get(&def_id).copied() else {
                return;
            };
            if !self.defs.is_creep(def_id) {
                return;
            }
            spawn_defs.push((def_id, def));
        }

        let mut members = BTreeSet::new();
        for (index, &(def_id, def)) in spawn_defs.iter().enumerate() {
            let (ox, oy) = FORMATION_OFFSETS[index % FORMATION_OFFSETS.len()];
            let x = SimFixed::from_raw(script.home_x.saturating_add(ox.saturating_mul(SCALE)));
            let y = SimFixed::from_raw(script.home_y.saturating_add(oy.saturating_mul(SCALE)));
            let id = self.entities.spawn();
            let mut unit = UnitState::new(
                x,
                y,
                NEUTRAL_OWNER,
                def.hp,
                SimFixed::from_raw(def.speed_raw),
                def.damage,
                SimFixed::from_raw(def.range_raw),
            );
            unit.gameplay.mana = def.mana;
            unit.gameplay.max_mana = def.mana;
            self.units.insert(id, unit);
            self.creep_camps.creeps.insert(
                id,
                CreepState {
                    camp,
                    def_id,
                    home_x: script.home_x,
                    home_y: script.home_y,
                    phase: LeashPhase::Idle,
                    target: None,
                },
            );
            self.creep_camps
                .neutral
                .insert(id, NeutralCategory::CreepHostile);
            members.insert(id);
            self.push_event(frame, id, EventKind::Spawned);
        }
        self.creep_camps.camps.insert(
            camp,
            CampState {
                script: script_id,
                tier: script.tier,
                home_x: script.home_x,
                home_y: script.home_y,
                members,
                cleared: false,
            },
        );
        self.push_event_raw(frame, u64::from(camp), EventKind::CreepCampSeeded);
    }

    /// 按稳定 `EntityId` 顺序找 home 锚点 `acquisitionRange` 内最近的玩家
    /// 单位（R8.1 对所有玩家敌对）。中性登记实体（其它野怪 / 地面物品）
    /// 不是目标，MUST NOT 依赖名字白名单（R1.2 / R13.3）。
    fn nearest_creep_target(
        &self,
        creep: EntityId,
        home_x: i64,
        home_y: i64,
        range: i64,
    ) -> Option<EntityId> {
        let mut best: Option<(EntityId, i128)> = None;
        for (&id, unit) in &self.units {
            if id == creep || !unit.alive || self.creep_camps.neutral.contains_key(&id) {
                continue;
            }
            let dx = unit.x.raw().saturating_sub(home_x) as i128;
            let dy = unit.y.raw().saturating_sub(home_y) as i128;
            let distance = isqrt(dx * dx + dy * dy);
            if distance > range as i128 {
                continue;
            }
            let better = match best {
                None => true,
                Some((_, best_distance)) => distance < best_distance,
            };
            if better {
                best = Some((id, distance));
            }
        }
        best.map(|(id, _)| id)
    }

    /// 野怪 leash / 归巢 `IDLE / ENGAGED / RETURNING` 确定性重算（R8.5 / D3）。
    /// 复用既有 `Attack` 订单面（`order_attack`）与 `Move` 订单面（`assign_move`）
    /// 及 `run_ai` 每帧重算框架：内部命令不写回队列、不注入重放；
    /// `RETURNING` 途中不索敌 / 不反击，MUST NOT 无限追出营地（R8.5）。
    fn run_creep_ai(&mut self, frame: Frame, attacks: &mut Vec<(EntityId, EntityId)>) {
        let constants = *self.defs.creep_constants();
        let period = u64::from(constants.ai_decision_period.max(1));
        // 稳定 `EntityId` 顺序重算（BTreeMap 键序，ADR 0002 §3）。
        let ids: Vec<EntityId> = self.creep_camps.creeps.keys().copied().collect();
        for id in ids {
            let Some(state) = self.creep_camps.creeps.get(&id).cloned() else {
                continue;
            };
            if !self.is_alive(id) {
                continue;
            }
            let Some((cx, cy)) = self.units.get(&id).map(|unit| (unit.x.raw(), unit.y.raw()))
            else {
                continue;
            };
            let dx = cx.saturating_sub(state.home_x) as i128;
            let dy = cy.saturating_sub(state.home_y) as i128;
            let home_distance = isqrt(dx * dx + dy * dy);

            // 决策点：与 `run_ai` 同构，只在整数帧周期重算路径，
            // 其余帧沿用既有订单，不写回命令队列、不注入重放。
            let decision = frame.value() % period == 0;
            match state.phase {
                LeashPhase::Idle => {
                    let found = self.nearest_creep_target(
                        id,
                        state.home_x,
                        state.home_y,
                        constants.acquisition_range_raw,
                    );
                    if let Some(target) = found {
                        if let Some(creep) = self.creep_camps.creeps.get_mut(&id) {
                            creep.phase = LeashPhase::Engaged;
                            creep.target = Some(target);
                        }
                        // 追击复用既有 `Move` 命令面，攻击复用既有
                        // `resolve_attacks` 结算面（`attacks` 队列）。
                        if decision {
                            self.chase_target(id, target);
                        }
                        attacks.push((id, target));
                        self.push_event(frame, id, EventKind::CreepEngaged);
                    }
                }
                LeashPhase::Engaged => {
                    let leash_exceeded = home_distance > constants.leash_radius_raw as i128;
                    let target_ok = state.target.is_some_and(|target| {
                        self.is_alive(target) && !self.creep_camps.neutral.contains_key(&target)
                    });
                    if leash_exceeded || !target_ok {
                        // 越界或目标失效：停止追击，切 RETURNING 归巢。
                        if let Some(creep) = self.creep_camps.creeps.get_mut(&id) {
                            creep.phase = LeashPhase::Returning;
                            creep.target = None;
                        }
                        self.assign_move(
                            id,
                            SimFixed::from_raw(state.home_x),
                            SimFixed::from_raw(state.home_y),
                            0,
                        );
                        self.push_event(frame, id, EventKind::CreepLeashReturning);
                    } else if let Some(target) = state.target {
                        // 每帧确定性重算：决策点重算追击路径，攻击对汇入
                        // 本帧同一结算序列（不新设 AI × lockstep 架构）。
                        if decision {
                            self.chase_target(id, target);
                        }
                        attacks.push((id, target));
                    }
                }
                LeashPhase::Returning => {
                    if home_distance <= constants.return_threshold_raw as i128 {
                        if let Some(creep) = self.creep_camps.creeps.get_mut(&id) {
                            creep.phase = LeashPhase::Idle;
                            creep.target = None;
                        }
                        self.stop_unit(id);
                        self.push_event(frame, id, EventKind::CreepLeashReturned);
                    } else if frame.value() % period == 0 {
                        // 归巢途中不索敌 / 不反击，只确定性重算路径。
                        self.assign_move(
                            id,
                            SimFixed::from_raw(state.home_x),
                            SimFixed::from_raw(state.home_y),
                            0,
                        );
                    }
                }
            }
        }
    }

    /// 追击：复用既有 `Move` 命令面（`assign_move`）向目标重算路径（D3）。
    fn chase_target(&mut self, creep: EntityId, target: EntityId) {
        let Some((x, y)) = self.units.get(&target).map(|unit| (unit.x, unit.y)) else {
            return;
        };
        self.assign_move(creep, x, y, 0);
    }

    /// 共享结算服务：按稳定 `EntityId` 顺序只结算一次击杀收益与掉落
    /// （R6.1 经验 / R6.5 分摊两环节 / R8.3 金币 / R7.2 掉落 / R8.6 随机码）。
    /// 掉落随机码复用 match-owned 整数 LCG（独立事件驱动流，由 `seed` 派生），
    /// 每个互斥 set 恰好消费一次、至多掉一件；消费序列显式进权威状态
    /// （Q6 / tasks 4.1-4.3 / D2 / D4）。
    fn settle_creep_deaths(&mut self, frame: Frame) {
        // 自动巡检：法术击杀 / 直接置死的野怪也进幂等登记（稳定 ID 顺序）。
        let creep_ids: Vec<EntityId> = self.creep_camps.creeps.keys().copied().collect();
        for id in creep_ids {
            if !self.is_alive(id)
                && !self.creep_camps.death_processed.contains(&id)
                && !self.creep_camps.pending_deaths.contains_key(&id)
            {
                let current = self.clock.current();
                self.creep_camps.pending_deaths.insert(
                    id,
                    CreepDeathNotice {
                        killer: EntityId::NONE,
                        killer_owner: NEUTRAL_OWNER,
                        frame: current.value(),
                    },
                );
                // 法术击杀 / 生命值直接归零也发同一事实事件，供只读表现层
                // 消费（以 `CreepDied` 解耦掉落服务，tasks 4.2）。
                self.push_event(frame, id, EventKind::CreepDied);
            }
        }

        // 按稳定 `EntityId` 顺序消费（BTreeMap 键序）。
        let pending: Vec<(EntityId, CreepDeathNotice)> = self
            .creep_camps
            .pending_deaths
            .iter()
            .map(|(&id, &notice)| (id, notice))
            .collect();
        for (creep, notice) in pending {
            if self.creep_camps.death_processed.contains(&creep) {
                self.creep_camps.pending_deaths.remove(&creep);
                continue;
            }
            self.creep_camps.death_processed.insert(creep);

            let Some(state) = self.creep_camps.creeps.get(&creep).cloned() else {
                self.creep_camps.pending_deaths.remove(&creep);
                continue;
            };
            let Some(camp) = self.creep_camps.camps.get(&state.camp).cloned() else {
                self.creep_camps.pending_deaths.remove(&creep);
                continue;
            };
            let Some(script) = self.defs.camp_script(camp.script).cloned() else {
                self.creep_camps.pending_deaths.remove(&creep);
                continue;
            };
            let constants = *self.defs.creep_constants();
            let table = self.defs.drop_table(script.drop_table).cloned();
            let (death_x, death_y) = self
                .units
                .get(&creep)
                .map(|unit| (unit.x.raw(), unit.y.raw()))
                .unwrap_or((camp.home_x, camp.home_y));

            // R6.5 第一环节：进入分摊集合（范围 + 存活 + 己方英雄，稳定顺序）。
            let mut xp_candidates = BTreeSet::new();
            for (&id, unit) in &self.units {
                if !unit.alive || unit.owner != notice.killer_owner || unit.gameplay.hero.is_none()
                {
                    continue;
                }
                let dx = unit.x.raw().saturating_sub(death_x) as i128;
                let dy = unit.y.raw().saturating_sub(death_y) as i128;
                if isqrt(dx * dx + dy * dy) <= constants.xp_share_radius_raw as i128 {
                    xp_candidates.insert(id);
                }
            }
            // R6.5 第二环节：实际入账（整数分摊 + 等级因子，不反写表现层）。
            // 满级英雄是否参与分摊 `[待核]`：本期不冻结该规则，所有集合内
            // 英雄统一入账（§14 / R6.5）。
            let mut xp_credited = BTreeMap::new();
            let count = xp_candidates.len() as i64;
            if count > 0 {
                let total = script.xp.saturating_mul(constants.hero_level_xp_factor) / 100;
                let share = total / count;
                for &hero in &xp_candidates {
                    if let Some(unit) = self.units.get_mut(&hero) {
                        if let Some(state) = unit.gameplay.hero.as_mut() {
                            state.xp = state.xp.saturating_add(share);
                        }
                    }
                    xp_credited.insert(hero, share);
                }
            }

            // R8.3 金币给击杀方（MUST NOT 由表现层反写 `PlayerEconomy.gold`）。
            let gold = if notice.killer_owner == NEUTRAL_OWNER {
                0
            } else {
                script.gold
            };
            if gold != 0 {
                let economy = self
                    .gameplay
                    .players
                    .entry(notice.killer_owner)
                    .or_default();
                economy.gold = economy.gold.saturating_add(gold);
            }

            // R7.2 / R8.6 掉落：每个互斥 set 恰好一次随机码消费、至多一件。
            let mut drops = Vec::new();
            if let Some(table) = table {
                for (&set_id, set) in &table.sets {
                    let total_weight = set.total_weight();
                    let rng_before = self.creep_camps.rng_state;
                    let rng_after = next_rng(rng_before);
                    self.creep_camps.rng_state = rng_after;
                    let roll = if total_weight == 0 {
                        0
                    } else {
                        rng_after % total_weight
                    };
                    let mut picked = None;
                    let mut cumulative = 0u64;
                    for (&item_def, &weight) in &set.entries {
                        cumulative = cumulative.saturating_add(u64::from(weight));
                        if roll < cumulative && self.defs.is_ground_item(item_def) {
                            picked = Some(item_def);
                            break;
                        }
                    }
                    let seq = self.creep_camps.next_rng_seq;
                    self.creep_camps.next_rng_seq = seq.saturating_add(1);
                    if let Some(item_def) = picked {
                        let ground_id = self.creep_camps.next_ground_item_id;
                        self.creep_camps.next_ground_item_id = ground_id.saturating_add(1);
                        let instance = EntityId::new(ground_id).unwrap_or(EntityId::NONE);
                        let effect = table.effects.get(&item_def).copied().unwrap_or(EffectDef {
                            kind: EffectKind::BuffStat,
                            amount: 0,
                            duration_frames: 0,
                        });
                        self.creep_camps.ground_items.insert(
                            instance,
                            GroundItemState {
                                item_def,
                                camp: state.camp,
                                x: death_x,
                                y: death_y,
                                instance,
                            },
                        );
                        // R7.5：同名道具每个实例独立充能 / 冷却；R7.6：效果
                        // 走既有 `EffectDef`，MUST NOT 引入 JASS VM。
                        self.creep_camps.item_instances.insert(
                            instance,
                            ItemInstanceState {
                                item_def,
                                charges: constants.item_charges,
                                cooldown_frames: constants
                                    .item_cooldown_frames
                                    .saturating_add((roll % 4) as u32),
                                effect,
                                // P8.5d：掉落实例初始无主、无槽位（R1.1 / Q7）。
                                holder: None,
                                slot: None,
                            },
                        );
                        self.creep_camps
                            .neutral
                            .insert(instance, NeutralCategory::GroundItemOwnerless);
                        self.push_event_raw(frame, ground_id, EventKind::ItemDropped);
                        drops.push(instance);
                    }
                    self.creep_camps.rng_draws.insert(
                        seq,
                        RngDraw {
                            seq,
                            frame: frame.value(),
                            creep,
                            table: script.drop_table,
                            set: set_id,
                            rng_before,
                            rng_after,
                            roll,
                            picked,
                        },
                    );
                }
            }

            self.creep_camps.kill_rewards.insert(
                creep,
                KillReward {
                    creep,
                    camp: state.camp,
                    frame: frame.value(),
                    killer: notice.killer,
                    killer_owner: notice.killer_owner,
                    xp_candidates,
                    xp_credited,
                    gold,
                    drops,
                },
            );
            self.creep_camps.pending_deaths.remove(&creep);
        }

        // 营地全清标记（R11.5 / R12.4 重开无残留的正向判据）。
        let camp_ids: Vec<u32> = self.creep_camps.camps.keys().copied().collect();
        for camp_id in camp_ids {
            let all_dead = self
                .creep_camps
                .camps
                .get(&camp_id)
                .is_some_and(|camp| camp.members.iter().all(|id| !self.is_alive(*id)));
            if let Some(camp) = self.creep_camps.camps.get_mut(&camp_id) {
                camp.cleared = all_dead;
            }
        }
    }

    /// 物品实例共享冷却按稳定 `EntityId` 顺序逐帧推进（R7.5 独立状态）。
    /// P8.5c 起覆盖地面物品，P8.5d 起同样覆盖六格背包内的持有实例
    /// （整数帧、无 `float` / 墙钟）；地面位置永不变化（R1.4），
    /// 且地面物品不是 `EntityStore` 实体。
    fn advance_ground_items(&mut self) {
        for instance in self.creep_camps.item_instances.values_mut() {
            instance.cooldown_frames = instance.cooldown_frames.saturating_sub(1);
        }
    }

    /// P11 技能系统逐帧结算：按稳定 ID 顺序递减运行态，并从本帧唯一主 LCG
    /// 后继状态派生随机消费记录。此函数不推进 `self.rng`，不新增独立随机流。
    fn advance_ability_system(&mut self, frame: Frame, rng_before: u64, rng_after: u64) {
        let Self { defs, gameplay, .. } = self;
        let constants = defs.ability_system().constants;
        let runtime = &mut gameplay.abilities;

        let zone_ids: Vec<EntityId> = runtime.zones.keys().copied().collect();
        for id in zone_ids {
            let expired = runtime.zones.get_mut(&id).is_some_and(|zone| {
                zone.remaining = zone.remaining.saturating_sub(1);
                zone.remaining == 0
            });
            if expired {
                runtime.zones.remove(&id);
            }
        }

        let summon_ids: Vec<EntityId> = runtime.summons.keys().copied().collect();
        for id in summon_ids {
            let expired = runtime.summons.get_mut(&id).is_some_and(|summon| {
                summon.remaining = summon.remaining.saturating_sub(1);
                summon.remaining == 0
            });
            if expired {
                runtime.summons.remove(&id);
            }
        }

        let transform_ids: Vec<EntityId> = runtime.transformations.keys().copied().collect();
        for id in transform_ids {
            let expired = runtime.transformations.get_mut(&id).is_some_and(|state| {
                state.remaining = state.remaining.saturating_sub(1);
                state.remaining == 0
            });
            if expired {
                runtime.transformations.remove(&id);
            }
        }

        let chain_ids: Vec<EntityId> = runtime.chains.keys().copied().collect();
        for id in chain_ids {
            let expired = runtime.chains.get_mut(&id).is_some_and(|chain| {
                chain.remaining = chain.remaining.saturating_sub(1);
                chain.remaining == 0
            });
            if expired {
                runtime.chains.remove(&id);
            }
        }

        let proc_keys: Vec<(EntityId, DefId)> = runtime.passive_procs.keys().copied().collect();
        let mut random_requests = Vec::new();
        for key in proc_keys {
            let mut expired = false;
            let mut random = false;
            if let Some(state) = runtime.passive_procs.get_mut(&key) {
                if state.remaining != u32::MAX {
                    state.remaining = state.remaining.saturating_sub(1);
                    expired = state.remaining == 0;
                }
                random = state.random;
            }
            if expired {
                runtime.passive_procs.remove(&key);
            } else if random {
                random_requests.push(key);
            }
        }

        for (unit, ability) in random_requests {
            let has_random = defs.abilities().get(&ability).is_some_and(|def| {
                def.effects
                    .iter()
                    .any(|effect| effect.kind == EffectKind::RandomDraw)
            });
            if !has_random {
                continue;
            }
            let seq = runtime.next_rng_seq;
            let mixed = mix_ability_rng(rng_after, constants.rng_mix_salt, seq, ability);
            let roll = mixed % constants.proc_modulus.max(1);
            runtime.rng_draws.insert(
                seq,
                AbilityRngDraw {
                    seq,
                    frame: frame.value(),
                    unit,
                    ability,
                    rng_before,
                    rng_after,
                    roll,
                    behavior: BehaviorKind::RandomDraw,
                },
            );
            runtime.next_rng_seq = seq.saturating_add(1);
        }
    }

    pub fn read_view(&self) -> WorldView {
        let units = &self.units;
        WorldView {
            tick: self.clock.current().value(),
            rng: self.rng,
            entities: self
                .entities
                .iter()
                .map(|id| {
                    let unit = units.get(&id).expect("every live entity has state");
                    EntityView {
                        id: id.value(),
                        x: unit.x.raw(),
                        y: unit.y.raw(),
                        facing: unit.facing.raw(),
                        speed: unit.speed.raw(),
                        hp: unit.hp,
                        owner: unit.owner,
                        alive: unit.alive,
                        has_target: unit.goal.is_some(),
                    }
                })
                .collect(),
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        // 事件序号在取出后刻意不重置。
        std::mem::take(&mut self.events)
    }

    pub fn compute_state_hash(&self) -> u64 {
        let mut hasher = StateHasher::new();
        hasher.write_u64(self.clock.current().value());
        hasher.write_u64(self.rng);
        hasher.write_u64(self.entities.next_id());
        hasher.write_u64(self.commands.next_arrival());
        hasher.write_u64(self.next_event_seq);
        hasher.write_u64(self.config.seed);
        hasher.write_u64(self.config.content_hash);
        hasher.write_u32(self.config.logic_fps);
        hasher.write_u64(self.world.identity());

        let units = &self.units;
        hasher.write_u64(self.entities.len() as u64);
        for id in self.entities.iter() {
            hasher.write_u64(id.value());
            hash_unit(
                &mut hasher,
                units.get(&id).expect("every live entity has state"),
            );
        }

        hasher.write_u64(self.obstacles.len() as u64);
        for (&id, area) in &self.obstacles {
            hasher.write_u64(id);
            hasher.write_i64(area.x as i64);
            hasher.write_i64(area.y as i64);
            hasher.write_i64(area.w as i64);
            hasher.write_i64(area.h as i64);
        }

        hasher.write_u64(self.group_plans.len() as u64);
        for (&group, plan) in &self.group_plans {
            hasher.write_u32(group);
            hasher.write_i64(plan.goal.0.raw());
            hasher.write_i64(plan.goal.1.raw());
            hasher.write_u64(plan.members.len() as u64);
            for id in &plan.members {
                hasher.write_u64(id.value());
            }
            hasher.write_u64(plan.slots.len() as u64);
            for &(x, y) in &plan.slots {
                hasher.write_i64(x.raw());
                hasher.write_i64(y.raw());
            }
            hasher.write_u64(plan.progress as u64);
        }

        hasher.write_u64(self.commands.pending().len() as u64);
        for command in self.commands.pending() {
            hash_command(&mut hasher, command);
        }
        // P5：冻结定义的内容哈希与全部玩法运行态进入权威状态哈希。
        // P11：技能系统全部权威运行态经 WorldGameplayState::hash 进入；
        // 定义层技能常量只在此处通过既有 defs.content_hash() 身份出现。
        hasher.write_u64(self.defs.content_hash());
        self.gameplay.hash(&mut hasher);
        // P8.5a：英雄复活三组权威状态（死亡登记 / 队列 / 费用）必须进入哈希。
        self.revive.hash(&mut hasher);
        // P8.5b：主城升级三组权威状态（等级 / 升级队列 / 费用登记）必须进入哈希。
        self.town_hall.hash(&mut hasher);
        // P8.5c：野怪 / 营地 / 中立归属 / 地面物品 / 实例 / 收益 /
        // 确定性随机码消费序列全部权威状态必须进入哈希（Q1 / D5）。
        self.creep_camps.hash(&mut hasher);
        // P8.5d：六格定长实例背包 / 复活死亡背包 / 事实 / 库存 / 刷新 /
        // 回城传送 / 消费序列 / 购买扣费登记全部权威状态必须进入哈希
        // （Q4 / Q7 / D1 / D5 / ADR 0002 §6）。
        self.item_shop.hash(&mut hasher);
        // P9 / Q2 / Q4：族特色机制顶层权威状态（腐地 / 尸体 / 金矿 /
        // 古树 / 昼夜相位 / 隐身目标判定 / 月亮井 / 缠绕金矿 / 夜间隐身
        // + Q4 全量结算态）必须进入哈希；全落 BTreeMap / BTreeSet、
        // 按稳定 EntityId 顺序遍历（ADR 0002 §6 / CODEBUDDY.md §5）。
        // fog 可见性 MUST NOT 进入哈希（R10.5 / §0.3 T3）。
        self.faction_mechanics.hash(&mut hasher);
        hasher.finish()
    }

    /// 返回首个出现差异的字段名，用于 desync 诊断。
    /// 比较顺序稳定，因此报告结果可复现。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.clock.current() != other.clock.current() {
            return Some("tick");
        }
        if self.rng != other.rng {
            return Some("rng");
        }
        if self.entities.next_id() != other.entities.next_id() {
            return Some("next_entity_id");
        }
        if self.commands.next_arrival() != other.commands.next_arrival() {
            return Some("next_arrival");
        }
        if self.next_event_seq != other.next_event_seq {
            return Some("next_event_seq");
        }
        if self.config.seed != other.config.seed {
            return Some("seed");
        }
        if self.config.content_hash != other.config.content_hash {
            return Some("content_hash");
        }
        if self.config.logic_fps != other.config.logic_fps {
            return Some("logic_fps");
        }
        if self.world.identity() != other.world.identity() {
            return Some("world_identity");
        }
        if self.entities.iter().collect::<Vec<_>>() != other.entities.iter().collect::<Vec<_>>() {
            return Some("entities");
        }
        for id in self.entities.iter() {
            if self.units.get(&id) != other.units.get(&id) {
                return Some("units");
            }
        }
        if self.obstacles != other.obstacles {
            return Some("obstacles");
        }
        if self.group_plans != other.group_plans {
            return Some("group_plans");
        }
        if self.commands.pending() != other.commands.pending() {
            return Some("pending_commands");
        }
        if self.defs != other.defs {
            return Some("frozen_definitions");
        }
        // P11：技能系统字段由 WorldGameplayState::first_difference 按同一
        // 稳定顺序返回，漏接任一新权威字段 MUST 在反向验证中验红。
        if let Some(field) = self.gameplay.first_difference(&other.gameplay) {
            return Some(field);
        }
        // P8.5a：三组复活权威状态按同一稳定顺序定位首个差异字段。
        if let Some(field) = self.revive.first_difference(&other.revive) {
            return Some(field);
        }
        // P8.5b：三组主城升级权威状态按同一稳定顺序定位首个差异字段。
        if let Some(field) = self.town_hall.first_difference(&other.town_hall) {
            return Some(field);
        }
        // P8.5c：野怪营地与掉落全部权威状态按同一稳定顺序定位首个差异字段。
        if let Some(field) = self.creep_camps.first_difference(&other.creep_camps) {
            return Some(field);
        }
        // P8.5d：物品 / 商店 / 回城全部权威状态按同一稳定顺序定位首个差异字段。
        if let Some(field) = self.item_shop.first_difference(&other.item_shop) {
            return Some(field);
        }
        // P9 / Q2 / Q4：族特色机制顶层权威状态按同一稳定顺序定位首个差异
        // 字段（ADR 0002 §6）。新增字段 MUST 同步接入本处 / 哈希 / 快照 /
        // 恢复（CODEBUDDY.md §5 硬门禁）。fog 可见性 MUST NOT 进入
        // first_difference()（R10.5 / §0.3 T3）。
        if let Some(field) = self
            .faction_mechanics
            .first_difference(&other.faction_mechanics)
        {
            return Some(field);
        }
        None
    }

    pub fn capture_snapshot(&self) -> Snapshot {
        let units = &self.units;
        Snapshot {
            version: SNAPSHOT_VERSION,
            sim_version: SIM_VERSION,
            seed: self.config.seed,
            content_hash: self.config.content_hash,
            defs_hash: self.defs.content_hash(),
            logic_fps: self.config.logic_fps,
            tick: self.clock.current().value(),
            rng: self.rng,
            next_entity_id: self.entities.next_id(),
            alive_entities: self.entities.iter().map(|id| id.value()).collect(),
            positions: self
                .entities
                .iter()
                .map(|id| {
                    let unit = units.get(&id).expect("every live entity has state");
                    PositionRecord {
                        entity: id.value(),
                        x: unit.x.raw(),
                        y: unit.y.raw(),
                        facing: unit.facing.raw(),
                        speed: unit.speed.raw(),
                        hp: unit.hp,
                        max_hp: unit.max_hp,
                        owner: unit.owner,
                        attack_damage: unit.attack_damage,
                        attack_range: unit.attack_range.raw(),
                        turn_rate: unit.turn_rate,
                        alive: unit.alive,
                        order: order_record(unit.order),
                        pending: unit
                            .pending
                            .iter()
                            .map(|order| order_record(*order))
                            .collect(),
                        goal: unit.goal.map(|(x, y)| (x.raw(), y.raw())),
                        path: unit.path.iter().map(|&(x, y)| (x.raw(), y.raw())).collect(),
                        path_index: unit.path_index as u32,
                        blocked_frames: unit.blocked_frames,
                        group: unit.group,
                        gameplay: unit.gameplay.clone(),
                    }
                })
                .collect(),
            pending_commands: self.commands.pending().to_vec(),
            next_arrival: self.commands.next_arrival(),
            next_event_seq: self.next_event_seq,
            world_identity: self.world.identity(),
            obstacles: self
                .obstacles
                .iter()
                .map(|(&id, area)| ObstacleRecord {
                    id,
                    x: area.x,
                    y: area.y,
                    w: area.w,
                    h: area.h,
                })
                .collect(),
            group_plans: self
                .group_plans
                .iter()
                .map(|(&group, plan)| GroupPlanRecord {
                    group,
                    goal: (plan.goal.0.raw(), plan.goal.1.raw()),
                    members: plan.members.iter().map(|id| id.value()).collect(),
                    slots: plan
                        .slots
                        .iter()
                        .map(|&(x, y)| (x.raw(), y.raw()))
                        .collect(),
                    progress: plan.progress as u32,
                })
                .collect(),
            defs: self.defs.clone(),
            // P11：帧 N 提交后的 Snapshot.gameplay 必须含技能权威运行态
            // （zone / 召唤 / 光环受益集 / 被动 proc / 变身 / buff 分类 /
            // 链接 / `rng_draws` / 进行中施法），恢复后从 N+1 继续。
            gameplay: self.gameplay.clone(),
            revive: self.revive.clone(),
            town_hall: self.town_hall.clone(),
            // P8.5c：帧 N 提交后的完整状态必须含野怪营地与掉落权威状态
            // （含事件驱动随机流状态与消费序列，ADR 0002 §6-8 / Q6）。
            creep_camps: self.creep_camps.clone(),
            // P8.5d：帧 N 提交后的完整状态必须含六格实例背包 / 复活死亡背包 /
            // 事实 / 库存 / 刷新 / 回城传送 / 消费序列 / 购买扣费登记，
            // 恢复后从 N+1 继续不残留、不重复购买或刷新（ADR 0002 §6-8 / R11.5）。
            item_shop: self.item_shop.clone(),
            // P9 / Q2 / Q4：帧 N 提交后的完整状态 MUST 含族特色机制顶层
            // 权威状态（由 SNAPSHOT_VERSION = 9 定义的 Snapshot 顶层字段），
            // 恢复后从 N+1 继续不残留、不重复结算（ADR 0002 §6-8）。
            // fog 可见性 MUST NOT 进入快照（R10.5 / §0.3 T3）。
            faction_mechanics: self.faction_mechanics.clone(),
        }
    }

    /// 从帧边界快照重建对局。历史事件不会被重放，只恢复事件序号计数器。
    /// 调用方须提供与快照身份一致的冻结世界。
    pub fn restore_snapshot(
        snapshot: Snapshot,
        world: StaticWorld,
        expected_content_hash: u64,
    ) -> Result<Self, SnapshotError> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::UnsupportedVersion {
                found: snapshot.version,
            });
        }
        if snapshot.sim_version != SIM_VERSION {
            return Err(SnapshotError::SimVersionMismatch {
                found: snapshot.sim_version,
                expected: SIM_VERSION,
            });
        }
        if snapshot.content_hash != expected_content_hash {
            return Err(SnapshotError::ContentMismatch);
        }
        // 冻结定义的内容哈希进快照头；定义被替换或篡改时明确拒绝，
        // 不产生「哈希相同而定义不同」的半初始化对局。
        if snapshot.defs_hash != snapshot.defs.content_hash() {
            return Err(SnapshotError::DefsMismatch);
        }
        if snapshot.world_identity != world.identity() {
            return Err(SnapshotError::WorldMismatch);
        }

        let config = MatchConfig {
            seed: snapshot.seed,
            logic_fps: snapshot.logic_fps,
            content_hash: snapshot.content_hash,
        };
        let entities = EntityStore::restore(snapshot.next_entity_id, &snapshot.alive_entities);

        let mut units = BTreeMap::new();
        for record in snapshot.positions {
            let id =
                EntityId::new(record.entity).ok_or(SnapshotError::UnknownEntity(record.entity))?;
            if !entities.contains(id) {
                return Err(SnapshotError::UnknownEntity(record.entity));
            }
            units.insert(
                id,
                UnitState {
                    x: SimFixed::from_raw(record.x),
                    y: SimFixed::from_raw(record.y),
                    facing: Angle::from_raw(record.facing),
                    speed: SimFixed::from_raw(record.speed),
                    hp: record.hp,
                    max_hp: record.max_hp,
                    owner: record.owner,
                    attack_damage: record.attack_damage,
                    attack_range: SimFixed::from_raw(record.attack_range),
                    turn_rate: record.turn_rate,
                    alive: record.alive,
                    order: order_from_record(record.order),
                    pending: record
                        .pending
                        .iter()
                        .map(|order| order_from_record(*order))
                        .collect(),
                    goal: record
                        .goal
                        .map(|(x, y)| (SimFixed::from_raw(x), SimFixed::from_raw(y))),
                    path: record
                        .path
                        .iter()
                        .map(|&(x, y)| (SimFixed::from_raw(x), SimFixed::from_raw(y)))
                        .collect(),
                    path_index: record.path_index as usize,
                    blocked_frames: record.blocked_frames,
                    group: record.group,
                    gameplay: record.gameplay.clone(),
                },
            );
        }
        if units.len() != entities.len() {
            return Err(SnapshotError::MissingMotion);
        }

        let obstacles: DynamicObstacles = snapshot
            .obstacles
            .iter()
            .map(|record| {
                (
                    record.id,
                    GridArea {
                        x: record.x,
                        y: record.y,
                        w: record.w,
                        h: record.h,
                    },
                )
            })
            .collect();
        let mut group_plans = BTreeMap::new();
        for record in &snapshot.group_plans {
            // 槽位与成员必须一一对应，进度不得越界，否则后续规划会越界访问。
            if record.slots.len() != record.members.len()
                || record.progress as usize > record.members.len()
            {
                return Err(SnapshotError::MalformedGroupPlan);
            }
            let mut members = Vec::with_capacity(record.members.len());
            for &value in &record.members {
                let id = EntityId::new(value).ok_or(SnapshotError::UnknownEntity(value))?;
                if !entities.contains(id) {
                    return Err(SnapshotError::UnknownEntity(value));
                }
                members.push(id);
            }
            group_plans.insert(
                record.group,
                GroupPlan {
                    goal: (
                        SimFixed::from_raw(record.goal.0),
                        SimFixed::from_raw(record.goal.1),
                    ),
                    members,
                    slots: record
                        .slots
                        .iter()
                        .map(|&(x, y)| (SimFixed::from_raw(x), SimFixed::from_raw(y)))
                        .collect(),
                    progress: record.progress as usize,
                },
            );
        }

        Ok(Self {
            config,
            clock: FrameClock::at(Frame::new(snapshot.tick), snapshot.logic_fps),
            entities,
            units,
            commands: CommandQueue::restore(snapshot.next_arrival, snapshot.pending_commands),
            world,
            obstacles,
            group_plans,
            rng: snapshot.rng,
            events: Vec::new(),
            next_event_seq: snapshot.next_event_seq,
            defs: snapshot.defs.clone(),
            // P11：从 v9 快照恢复技能权威运行态；五重头部校验通过后，
            // 克隆完整 `WorldGameplayState` 即可逐帧哈希与销毁前一致。
            gameplay: snapshot.gameplay.clone(),
            revive: snapshot.revive.clone(),
            town_hall: snapshot.town_hall.clone(),
            creep_camps: snapshot.creep_camps.clone(),
            item_shop: snapshot.item_shop.clone(),
            // P9 / Q2 / Q4：从 v9 快照恢复族特色机制顶层权威状态，
            // 逐帧哈希与销毁前一致（ADR 0002 §6-8 / Q3）。
            faction_mechanics: snapshot.faction_mechanics.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::UnitDef;

    fn spawn(x: i64, y: i64, hp: i64, damage: i64) -> CommandKind {
        CommandKind::Spawn {
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(y).unwrap(),
            hp,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: damage,
            attack_range: SimFixed::from_int(100).unwrap(),
        }
    }

    #[test]
    fn read_view_is_an_owned_copy() {
        let mut sim = RtsMatch::create(MatchConfig::default());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0, 100, 10)))
            .unwrap();
        sim.step();
        let view = sim.read_view();
        assert_eq!(view.entities.len(), 1);
        assert_eq!(view.tick, 1);
        // 该视图无法用于修改模拟状态。
        assert_eq!(sim.read_view(), view);
    }

    #[test]
    fn draining_events_does_not_reset_the_sequence() {
        let mut sim = RtsMatch::create(MatchConfig::default());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0, 100, 10)))
            .unwrap();
        sim.step();
        let first = sim.drain_events();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].sequence, 0);
        sim.submit_command(Command::new(Frame::new(1), 1, 2, spawn(1, 1, 100, 10)))
            .unwrap();
        sim.step();
        let second = sim.drain_events();
        assert_eq!(second[0].sequence, 1);
    }

    #[test]
    fn same_frame_double_kill_only_kills_once() {
        let mut sim = RtsMatch::create(MatchConfig::default());
        // 三名单位同点；攻击者单发伤害足以致命。
        for player in 1..=3 {
            sim.submit_command(Command::new(
                Frame::new(0),
                player,
                1,
                spawn(0, 0, 100, 100),
            ))
            .unwrap();
        }
        sim.step();
        sim.drain_events();

        let target = EntityId::new(3).unwrap();
        let first = EntityId::new(1).unwrap();
        let second = EntityId::new(2).unwrap();
        // player_id 必须匹配攻击者归属；两发攻击在同一帧按稳定顺序结算。
        sim.submit_command(Command::new(
            Frame::new(1),
            1,
            2,
            CommandKind::Attack {
                attacker: first,
                target,
            },
        ))
        .unwrap();
        sim.submit_command(Command::new(
            Frame::new(1),
            2,
            2,
            CommandKind::Attack {
                attacker: second,
                target,
            },
        ))
        .unwrap();
        sim.step();

        let events = sim.drain_events();
        let died = events.iter().filter(|e| e.kind == EventKind::Died).count();
        let hits = events
            .iter()
            .filter(|e| e.kind == EventKind::AttackHit)
            .count();
        assert_eq!(died, 1, "target must die exactly once");
        assert_eq!(hits, 1, "no duplicate damage");
        let view = sim.read_view();
        let target_view = view.entities.iter().find(|e| e.id == 3).unwrap();
        assert!(!target_view.alive);
        assert_eq!(target_view.hp, 0);
    }

    /// 构造含「建筑 speed_raw == 0 / 单位 speed_raw > 0」两类冻结定义的对局。
    /// 全部数值为合成原创常量，仅用于只读投影验证，不冻结冒充原版。
    fn movability_defs() -> (FrozenDefinitions, i64, i64) {
        let building_speed_raw = 0;
        let unit_speed_raw = SimFixed::from_int(5).unwrap().raw();
        let building = UnitDef {
            hp: 50,
            mana: 0,
            speed_raw: building_speed_raw,
            damage: 0,
            range_raw: 0,
            supply: 0,
            train_frames: 2,
        };
        let unit = UnitDef {
            hp: 80,
            mana: 0,
            speed_raw: unit_speed_raw,
            damage: 10,
            range_raw: SimFixed::from_int(10).unwrap().raw(),
            supply: 1,
            train_frames: 2,
        };
        let defs = FrozenDefinitions::new(
            vec![(1, building), (2, unit)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        (defs, building_speed_raw, unit_speed_raw)
    }

    /// 经既有公开入口与 `Train` 生产路径产出建筑与普通单位，推进到完成帧。
    fn production_match() -> (RtsMatch, i64, i64) {
        let (defs, building_speed_raw, unit_speed_raw) = movability_defs();
        let config = MatchConfig {
            seed: 1,
            logic_fps: 30,
            content_hash: defs.content_hash(),
        };
        let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs);
        sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0, 100, 0)))
            .unwrap();
        let producer = EntityId::new(1).unwrap();
        sim.submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            1,
            1,
            GameplayCommandKind::Train {
                producer,
                unit_def: 1,
                is_building: true,
            },
        ))
        .unwrap();
        sim.submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            1,
            2,
            GameplayCommandKind::Train {
                producer,
                unit_def: 2,
                is_building: false,
            },
        ))
        .unwrap();
        // 帧 0 出生，帧 1 入队并递减，帧 2 生产完成。
        for _ in 0..3 {
            sim.step();
        }
        (sim, building_speed_raw, unit_speed_raw)
    }

    /// 只读投影一致性：视图 speed 等于 spawn 命令的 speed.raw()，两实体不串值，
    /// 且与权威快照记录一致；消费只读视图不改写权威状态。
    #[test]
    fn read_view_projects_authoritative_speed_without_crossover() {
        let slow = SimFixed::from_int(3).unwrap();
        let fast = SimFixed::from_int(7).unwrap();
        assert_ne!(slow.raw(), fast.raw(), "two distinct speeds are required");

        let mut sim = RtsMatch::create(MatchConfig::default());
        for (sequence, speed) in [(1, slow), (2, fast)] {
            sim.submit_command(Command::new(
                Frame::new(0),
                1,
                sequence,
                CommandKind::Spawn {
                    x: SimFixed::ZERO,
                    y: SimFixed::ZERO,
                    hp: 100,
                    speed,
                    attack_damage: 0,
                    attack_range: SimFixed::ZERO,
                },
            ))
            .unwrap();
        }
        sim.step();

        let view = sim.read_view();
        assert_eq!(
            view.entities.len(),
            2,
            "both spawned entities must be visible"
        );
        let first = view.entities.iter().find(|e| e.id == 1).unwrap();
        let second = view.entities.iter().find(|e| e.id == 2).unwrap();
        assert_eq!(
            first.speed,
            slow.raw(),
            "view.speed must equal spawn speed.raw() for entity 1"
        );
        assert_eq!(
            second.speed,
            fast.raw(),
            "view.speed must equal spawn speed.raw() for entity 2"
        );
        assert_ne!(
            first.speed, second.speed,
            "projection must not cross values between entities"
        );

        // 视图 speed 投影的是权威 UnitState.speed，与快照的 PositionRecord.speed 逐条一致。
        let snapshot = sim.capture_snapshot();
        for entity in &view.entities {
            let record = snapshot
                .positions
                .iter()
                .find(|p| p.entity == entity.id)
                .unwrap();
            assert_eq!(
                entity.speed, record.speed,
                "read-only projection must mirror authoritative PositionRecord.speed"
            );
        }

        // 只读视图消费前后，同一实例的哈希与快照必须完全一致。
        let hash_before = sim.compute_state_hash();
        let snapshot_before = sim.capture_snapshot();
        let _ = sim.read_view();
        let _ = sim.read_view();
        assert_eq!(
            sim.compute_state_hash(),
            hash_before,
            "read_view must not mutate state hash"
        );
        assert_eq!(
            sim.capture_snapshot(),
            snapshot_before,
            "read_view must not mutate snapshot"
        );
    }

    /// 运行时动态实体可观测性：生产出的建筑 view.speed == 0、
    /// 普通单位 view.speed > 0，表现层可由只读视图判定可移动性。
    #[test]
    fn runtime_production_building_and_unit_speeds_are_observable() {
        let (sim, building_speed_raw, unit_speed_raw) = production_match();
        assert_eq!(building_speed_raw, 0, "building def speed_raw must be zero");
        assert!(unit_speed_raw > 0, "unit def speed_raw must be positive");

        let view = sim.read_view();
        assert_eq!(
            view.entities.len(),
            3,
            "producer plus two runtime entities must exist"
        );
        // 生产者来自 frame-0 Spawn（speed 被抬过），本测试只判定运行时动态实体。
        let runtime: Vec<EntityView> = view
            .entities
            .iter()
            .filter(|e| e.id != 1)
            .cloned()
            .collect();
        assert_eq!(
            runtime.len(),
            2,
            "exactly two runtime entities must be produced"
        );

        let zeros: Vec<&EntityView> = runtime.iter().filter(|e| e.speed == 0).collect();
        let positives: Vec<&EntityView> = runtime.iter().filter(|e| e.speed > 0).collect();
        assert_eq!(
            zeros.len(),
            1,
            "exactly one runtime entity must be immovable (building)"
        );
        assert_eq!(
            positives.len(),
            1,
            "exactly one runtime entity must be movable (unit)"
        );
        assert_eq!(
            zeros[0].speed, building_speed_raw,
            "runtime building view.speed must equal def.speed_raw == 0"
        );
        assert_eq!(
            zeros[0].speed, 0,
            "runtime building view.speed must be exactly zero"
        );
        assert_eq!(
            positives[0].speed, unit_speed_raw,
            "runtime unit view.speed must equal def.speed_raw"
        );
        assert!(
            positives[0].speed > 0,
            "runtime unit view.speed must be positive"
        );
        assert_ne!(
            zeros[0].id, positives[0].id,
            "building and unit must be distinct entities"
        );

        // 权威快照记录同时证明 complete_production 写入的是真实 def.speed_raw。
        let snapshot = sim.capture_snapshot();
        for entity in &runtime {
            let record = snapshot
                .positions
                .iter()
                .find(|p| p.entity == entity.id)
                .unwrap();
            assert_eq!(
                entity.speed, record.speed,
                "view.speed must mirror authoritative PositionRecord.speed"
            );
        }
    }

    /// 确定性零冲击：同一对局序列的两个同构实例逐帧哈希 / 快照 / 只读视图一致，
    /// 快照版本为 9（P9 新增族特色机制顶层权威状态），只读视图消费与快照往返均不
    /// 改变权威状态。
    #[test]
    fn readonly_speed_projection_has_zero_hash_and_snapshot_impact() {
        assert_eq!(SNAPSHOT_VERSION, 9, "snapshot version must be 9");
        assert_eq!(SIM_VERSION, 9, "sim version must be 9");

        let (defs, _, _) = movability_defs();
        let config = MatchConfig {
            seed: 0x5eed,
            logic_fps: 30,
            content_hash: defs.content_hash(),
        };

        // 同一对局序列：frame-0 Spawn、运行时 Train、普通 Move。
        let build = || {
            let mut sim =
                RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
            sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0, 100, 0)))
                .unwrap();
            sim.submit_gameplay_command(GameplayCommand::new(
                Frame::new(1),
                1,
                1,
                GameplayCommandKind::Train {
                    producer: EntityId::new(1).unwrap(),
                    unit_def: 2,
                    is_building: false,
                },
            ))
            .unwrap();
            sim.submit_command(Command::new(
                Frame::new(2),
                1,
                2,
                CommandKind::Move {
                    entity: EntityId::new(1).unwrap(),
                    x: SimFixed::from_int(4).unwrap(),
                    y: SimFixed::from_int(3).unwrap(),
                    group: 0,
                    append: false,
                },
            ))
            .unwrap();
            sim
        };

        let mut a = build();
        let mut b = build();
        for frame in 0..12 {
            a.step();
            b.step();
            assert_eq!(
                a.compute_state_hash(),
                b.compute_state_hash(),
                "frame {frame}: state hash diverged at {:?}",
                a.first_difference(&b)
            );
            assert_eq!(
                a.capture_snapshot(),
                b.capture_snapshot(),
                "frame {frame}: snapshot diverged"
            );
            assert_eq!(
                a.read_view(),
                b.read_view(),
                "frame {frame}: read-only view diverged"
            );
            assert_eq!(
                a.first_difference(&b),
                None,
                "frame {frame}: no differences expected"
            );
        }

        // 快照头部版本与内容身份保持稳定。
        let snapshot = a.capture_snapshot();
        assert_eq!(
            snapshot.version, SNAPSHOT_VERSION,
            "snapshot version must stay 9"
        );
        assert_eq!(snapshot.sim_version, SIM_VERSION, "sim version must stay 9");
        assert_eq!(
            snapshot.defs_hash,
            defs.content_hash(),
            "defs_hash must stay unchanged"
        );
        assert_eq!(
            snapshot.content_hash, config.content_hash,
            "content_hash must stay unchanged"
        );

        // 只读视图消费前后，同一实例的哈希与快照完全一致。
        let hash_before = a.compute_state_hash();
        let snapshot_before = a.capture_snapshot();
        let view = a.read_view();
        assert_eq!(
            a.compute_state_hash(),
            hash_before,
            "read_view must not affect state hash"
        );
        assert_eq!(
            a.capture_snapshot(),
            snapshot_before,
            "read_view must not affect snapshot"
        );
        assert!(!view.entities.is_empty(), "view must still be populated");

        // 快照往返继续一致（含新视图字段的只读取值）。
        let restored =
            RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), config.content_hash)
                .unwrap();
        assert_eq!(
            a.compute_state_hash(),
            restored.compute_state_hash(),
            "restore must preserve hash"
        );
        assert_eq!(
            a.read_view(),
            restored.read_view(),
            "restore must preserve read-only view"
        );
        assert_eq!(
            a.first_difference(&restored),
            None,
            "restore must not introduce differences"
        );
    }
}
