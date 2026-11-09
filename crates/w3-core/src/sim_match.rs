//! 权威对局状态的唯一所有者。
//!
//! `RtsMatch` 是唯一写入者；所有公开读取都返回所有权拷贝，
//! 绝不暴露可变的内部集合。`step()` 恰好推进一个逻辑帧，不接受 delta。
//!
//! P2 竖切：订单、寻路、动态占地与群体规划全部由本模块独占，并同时进入
//! 状态哈希、快照与 `first_difference()`（见 ADR 0002 / 0004）。

use std::collections::BTreeMap;

use crate::angle::{Angle, slope_speed_scale};
use crate::command::{Command, CommandError, CommandKind, CommandQueue};
use crate::entity::{EntityId, EntityStore};
use crate::fixed::{SimFixed, isqrt};
use crate::frame::{Frame, FrameClock};
use crate::gameplay::{
    AuraState, BehaviorSpec, CastRuntime, DefId, EffectDef, EffectKind, EffectOutcome,
    FrozenDefinitions, GameplayCommand, GameplayCommandError, GameplayCommandKind, ProductionItem,
    Settlement, UnitGameplayState, VictoryState, Vitals, WaveState, WorldGameplayState,
    resolve_effects, validate_gameplay_entities,
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
}

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
}

/// 纯整数 LCG：相同状态总是产生相同的后继状态。
fn next_rng(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
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
        }
    }

    /// 注入冻结内容定义；创建后不提供任何运行期热更新入口。
    pub fn create_with_definitions(
        config: MatchConfig,
        world: StaticWorld,
        defs: FrozenDefinitions,
    ) -> Self {
        let mut sim = Self::create_with_world(config, world);
        sim.defs = defs;
        sim
    }

    /// 冻结定义的只读视图。
    pub fn definitions(&self) -> &FrozenDefinitions {
        &self.defs
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

        // 6. 同帧攻击结算，按稳定顺序、带存活复核。
        self.resolve_attacks(frame, &attacks);

        // 7. 冻结本帧玩法命令并执行（费用事务提交 + 效果链，统一结算）。
        let due_gameplay = self.gameplay.commands.freeze_frame(frame);
        for command in due_gameplay {
            self.apply_gameplay(command, frame);
        }

        // 8. 推进玩法运行态（施法 / 引导、冷却、Buff 到期、光环 / 波次、生产 / 研究）。
        self.advance_gameplay_timers();

        // 9. 胜负判定：既定帧阶段、稳定顺序、只结算一次。
        self.resolve_victory(frame);

        // 10. 确定性 AI：每帧重算，内部命令不写回命令队列、不注入重放。
        self.run_ai(frame);

        // 11. 每帧消耗一次随机流；该流属于权威状态。
        self.rng = next_rng(self.rng);

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
    fn is_actionable(&self, entity: EntityId, player_id: u8) -> bool {
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
            );
            let died = !self.is_alive(target);
            if died {
                if let Some(unit) = self.units.get_mut(&target) {
                    unit.order = Order::Idle;
                    unit.pending.clear();
                    unit.goal = None;
                    unit.path.clear();
                    unit.path_index = 0;
                    unit.blocked_frames = 0;
                    unit.group = 0;
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

    fn push_event(&mut self, frame: Frame, entity: EntityId, kind: EventKind) {
        self.events.push(Event {
            frame: frame.value(),
            sequence: self.next_event_seq,
            entity: entity.value(),
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
    fn apply_effect(&mut self, target: EntityId, effect: &EffectDef) -> EffectOutcome {
        let Some(unit) = self.units.get_mut(&target) else {
            return EffectOutcome::NoEffect;
        };
        let mut vitals = Self::make_vitals(unit);
        match effect.kind {
            EffectKind::Damage => Settlement::damage(&mut vitals, effect.amount),
            EffectKind::Heal => Settlement::heal(&mut vitals, effect.amount),
            // BuffStat 经结算同时改属性修正并登记短期 Buff；Stun 登记眩晕 Buff。
            EffectKind::BuffStat => Settlement::stat_mod(&mut vitals, effect),
            EffectKind::Stun => Settlement::buff(&mut vitals, effect),
        }
    }

    /// 法力费用提交事务：与效果执行链分离，失败不产生任何改动。
    fn commit_mana_cost(&mut self, caster: EntityId, cost: i64) -> bool {
        let Some(unit) = self.units.get_mut(&caster) else {
            return false;
        };
        Settlement::commit_cost(&mut Self::make_vitals(unit), cost)
    }

    /// 效果链：顺序执行；目标中途死亡后由 `Settlement` 自然返回 `NoEffect`，
    /// 已提交的效果不回滚、不重复扣费。
    fn apply_effects(&mut self, target: EntityId, ability: DefId) {
        let effects = resolve_effects(&self.defs, &self.gameplay, ability);
        for effect in effects {
            self.apply_effect(target, &effect);
        }
    }

    /// 施法（玩家命令与 AI 共用入口）：同源校验 → 费用事务 → 运行态 / 效果链。
    fn execute_cast(&mut self, caster: EntityId, ability: DefId, target: EntityId, player: u8) {
        if !self.is_actionable(caster, player) || !self.is_alive(target) {
            return;
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
        if def.cast_frames > 0 {
            if let Some(unit) = self.units.get_mut(&caster) {
                unit.gameplay.cast = Some(CastRuntime {
                    ability,
                    target,
                    remaining_frames: def.cast_frames,
                    channel: true,
                });
            }
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
            self.apply_effects(target, ability);
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
        let frames = self
            .defs
            .upgrades()
            .get(&upgrade)
            .map(|def| def.research_frames)
            .unwrap_or(0);
        let economy = self.gameplay.players.entry(player).or_default();
        economy.research = Some((upgrade, frames.max(1)));
    }

    fn buy_item(&mut self, unit: EntityId, item: DefId) {
        let Some(def) = self.defs.items().get(&item).copied() else {
            return;
        };
        if let Some(state) = self.units.get_mut(&unit) {
            state.gameplay.bag.push(item);
            // 属性修正统一走结算服务，不直接改权威字段。
            Settlement::apply_stat_mod(&mut Self::make_vitals(state), def.damage_mod, def.hp_mod);
        }
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
            } => self.execute_cast(caster, ability, target, player),
            GameplayCommandKind::Train {
                producer,
                unit_def,
                is_building,
            } => self.enqueue_production(producer, unit_def, is_building, player),
            GameplayCommandKind::CancelTrain { producer, index } => {
                self.cancel_train(producer, index)
            }
            GameplayCommandKind::Research { upgrade } => self.research(player, upgrade),
            GameplayCommandKind::BuyItem { unit, item } => self.buy_item(unit, item),
            GameplayCommandKind::Gather { unit } => self.gather(unit, player),
            GameplayCommandKind::RegisterBehavior { id, effect } => {
                self.register_behavior(id, effect)
            }
            GameplayCommandKind::SpawnWave { id, period } => self.spawn_wave(id, period),
            GameplayCommandKind::SetAi { enabled } => self.set_ai(enabled, frame),
            GameplayCommandKind::Leave { player } => self.leave(player),
        }
    }

    /// 推进全部可保存运行态；完成项在稳定顺序下统一提交。
    fn advance_gameplay_timers(&mut self) {
        let ids: Vec<EntityId> = self.units.keys().copied().collect();
        let mut finished_casts: Vec<(DefId, EntityId)> = Vec::new();
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
                finished_casts.push((ability, target));
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

        // 施法完成 → 效果链（统一结算，不重复扣费）。
        for (ability, target) in finished_casts {
            self.apply_effects(target, ability);
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
        self.units.insert(id, unit);
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
        hasher.write_u64(self.defs.content_hash());
        self.gameplay.hash(&mut hasher);
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
        if let Some(field) = self.gameplay.first_difference(&other.gameplay) {
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
            gameplay: self.gameplay.clone(),
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
            gameplay: snapshot.gameplay.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
