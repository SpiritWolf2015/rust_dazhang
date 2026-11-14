//! winit 键鼠 → 权威命令的翻译层（tasks 5.1–5.6、design D3）。
//!
//! 本模块是**纯表现层逻辑**：只消费只读 [`WorldView`] 与屏幕坐标，产出
//! [`ClientCommand`]（普通命令 / 玩法命令）。它不持有 `RtsMatch`、不写入任何
//! 权威字段；命令最终只经 `submit_command` / `submit_gameplay_command` 提交。
//!
//! 选中集合、框选矩形、序号计数器一律是表现层专属状态，MUST NOT 进入
//! FNV-1a 状态哈希或 `SNAPSHOT_VERSION`（当前 3）快照。执行帧由冻结的
//! 整数帧号推导（`WorldView::tick + LEAD_FRAMES`），MUST NOT 用墙钟或呈现帧号。
//!
//! 屏幕 `f64` → 世界 `f64` 的拾取运算只发生在表现层；任何将进入权威状态的
//! 坐标都 MUST 经既有冻结边界 [`w3_app::freeze_f64`]（ADR 0004）转成 `SimFixed`，
//! 不新增浮点到权威状态的路径。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use w3_app::freeze_f64;
use w3_core::{
    Command, CommandKind, EntityId, Event, Frame, GameplayCommand, GameplayCommandKind, SCALE,
    SimFixed, WorldView,
};

use crate::melee::defs::BARRACKS;

/// 命令提前量：执行帧 = 当前冻结逻辑帧号 + 该常量（整数帧，ADR 0002 §2）。
/// 固定值，与 `Instant` / 呈现帧号 / 刷新率无关。
pub const LEAD_FRAMES: u64 = 1;

/// 本地玩家编号（表现层配置）。
pub const LOCAL_PLAYER: u8 = 1;

/// 施法默认技能与生产默认单位定义编号；合成配置，不是任何导出资产。
pub const DEFAULT_ABILITY: u32 = 10;
pub const DEFAULT_UNIT_DEF: u32 = 1;

/// 单个选中单位在屏幕上可拾取的半径（像素）。
const PICK_RADIUS: f64 = 24.0;

/// 翻译层产出的命令；宿主据其类型选择两个公开提交入口之一。
#[derive(Clone, Debug, PartialEq)]
pub enum ClientCommand {
    /// 普通命令，经 `RtsMatch::submit_command` 提交。
    Simulation(Command),
    /// 玩法命令，经 `RtsMatch::submit_gameplay_command` 提交。
    Gameplay(GameplayCommand),
}

/// 输入翻译错误：显式返回，MUST NOT 静默降级（tasks 5.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    /// 没有选中单位却请求了需要施法者 / 生产者的动作。
    NoSelection,
    /// 需要敌方目标的动作没有拾取到目标。
    NoTarget,
    /// 屏幕坐标或视口尺寸非法（NaN / 非正）。
    InvalidScreenPoint,
    /// 世界坐标经冻结边界转换失败（ADR 0004）。
    Frozen(w3_app::FrozenError),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSelection => f.write_str("no unit selected for input action"),
            Self::NoTarget => f.write_str("no target entity picked for input action"),
            Self::InvalidScreenPoint => f.write_str("invalid screen point or viewport"),
            Self::Frozen(error) => write!(f, "freeze boundary rejected coordinate: {error}"),
        }
    }
}

/// 屏幕 ↔ 世界（XZ 平面）的拾取映射，纯表现层 `f64`。
///
/// `world_span` 为视口覆盖的世界宽度，`pan` / `zoom` 来自表现层相机状态。
/// 注入 `ScreenProjection` 后，`screen_to_world` 以 y=0 地面为基准；未注入
/// 时退回轴对齐线性映射（供单元测试）。结果 MUST 经 `freeze_f64` 冻结后
/// 才进入权威状态。不使用三角函数或超越函数，跨平台差异与确定性无关。
///
/// **实体屏幕包围盒**：注入 `entity_boxes`（由 `Scene::entity_screen_boxes`
/// 从真实模型逐顶点投影）后，`entity_screen_box` / `entity_screen_center`
/// 是 HUD 标记与框选命中的**单一真值源**，替代旧 `world_to_screen` /
/// `unit_screen_bounds` 那套按程序化四面体低空平面估算的口径（会与放大 4×
/// 且贴地的真实模型错位上百像素）。未注入时回退旧方法，既有单测零改动。
#[derive(Clone, Debug)]
pub struct ScreenMapper {
    pub width: f64,
    pub height: f64,
    pub world_span: f64,
    pub pan: [f64; 2],
    pub zoom: f64,
    /// 与渲染相机同源的精确投影；`None` 时退回线性映射（供单元测试）。
    projection: Option<w3_render::ScreenProjection>,
    /// 物理像素 / 逻辑像素（窗口 `scale_factor`）；默认 1.0 表示无缩放。
    pixels_per_point: f64,
    /// 实体 ID → 真实模型屏幕包围盒（物理像素）；`None` = 回退旧口径。
    entity_boxes: Option<Arc<BTreeMap<u64, ([f64; 2], [f64; 2])>>>,
}

impl ScreenMapper {
    /// 由视口尺寸、地形覆盖范围与相机状态构造映射。
    pub fn new(width: f64, height: f64, world_span: f64, pan: [f64; 2], zoom: f64) -> Self {
        Self {
            width,
            height,
            world_span,
            pan,
            zoom,
            projection: None,
            pixels_per_point: 1.0,
            entity_boxes: None,
        }
    }

    /// 注入与 `Camera::matrix` 同源的精确投影，使拾取与呈现口径一致。
    pub fn with_projection(mut self, projection: w3_render::ScreenProjection) -> Self {
        self.projection = Some(projection);
        self
    }

    /// 注入窗口缩放比：winit 鼠标与 GPU 交换链是物理像素，egui HUD 是
    /// 逻辑像素；HUD 绘制前 MUST 经 `to_logical` 换算，否则高 DPI 下
    /// 选中框 / 血条会整体偏移 `pixels_per_point` 倍。
    pub fn with_pixels_per_point(mut self, pixels_per_point: f64) -> Self {
        self.pixels_per_point = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
            pixels_per_point
        } else {
            1.0
        };
        self
    }

    /// 注入由 `Scene::entity_screen_boxes` 产出的真实模型屏幕包围盒，作为
    /// HUD 标记 / 选中框 / 框选命中的单一真值源。纯表现层，不进权威状态。
    pub fn with_entity_boxes(mut self, boxes: BTreeMap<u64, ([f64; 2], [f64; 2])>) -> Self {
        self.entity_boxes = Some(Arc::new(boxes));
        self
    }

    /// 实体真实模型屏幕包围盒（物理像素）：命中测试 / 框选用。
    ///
    /// - 未注入 boxes（验收用合成 mapper / 线性映射单测）：回退旧
    ///   `unit_screen_bounds`，保证既有单测零改动。
    /// - 已注入但缺该 id（首帧前 scene 空、运行时新实体尚未 `consume`、
    ///   相机背后实体被投影过滤）：**不回退旧 24 单位四面体巨框**（那会在
    ///   zoom=7 下一框选中全屏、正是「选不上 / 乱选」根因），改用一个以
    ///   `world_to_screen` 为中心、`PICK_RADIUS` 为半径的紧致 box，使命中
    ///   自然收敛到单位逻辑点附近、绝不产生近全屏框。该实体在下一帧
    ///   `consume` 后即有真实 box，短窗口内降级可接受。
    pub fn entity_screen_box(&self, id: u64, world: [f64; 2]) -> ([f64; 2], [f64; 2]) {
        if let Some(boxes) = &self.entity_boxes {
            if let Some(&b) = boxes.get(&id) {
                return b;
            }
            let center = self.world_to_screen(world);
            return (
                [center[0] - PICK_RADIUS, center[1] - PICK_RADIUS],
                [center[0] + PICK_RADIUS, center[1] + PICK_RADIUS],
            );
        }
        self.unit_screen_bounds(world)
    }

    /// 实体真实模型屏幕包围盒的中心（物理像素）：HUD 队伍色点 / 选中框
    /// 锚点。未注入 boxes 时回退 `world_to_screen`，既有行为零改动。
    pub fn entity_screen_center(&self, id: u64, world: [f64; 2]) -> [f64; 2] {
        if let Some(boxes) = &self.entity_boxes {
            if let Some(&(min, max)) = boxes.get(&id) {
                return [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
            }
        }
        self.world_to_screen(world)
    }

    /// 物理屏幕坐标 → egui 逻辑坐标。
    pub fn to_logical(&self, point: [f64; 2]) -> [f64; 2] {
        [
            point[0] / self.pixels_per_point,
            point[1] / self.pixels_per_point,
        ]
    }

    /// 物理屏幕包围盒 → egui 逻辑包围盒。
    pub fn to_logical_bounds(&self, bounds: ([f64; 2], [f64; 2])) -> ([f64; 2], [f64; 2]) {
        (self.to_logical(bounds.0), self.to_logical(bounds.1))
    }

    /// 逻辑视口尺寸（物理尺寸 / 缩放比），供 HUD 布局与自测使用。
    pub fn logical_size(&self) -> [f64; 2] {
        [
            self.width / self.pixels_per_point,
            self.height / self.pixels_per_point,
        ]
    }

    /// 屏幕像素坐标 → 世界 `f64` 坐标（尚未冻结，仅表现层使用）。
    pub fn screen_to_world(&self, point: [f64; 2]) -> Result<[f64; 2], InputError> {
        if !point.iter().all(|v| v.is_finite())
            || self.width <= 0.0
            || self.height <= 0.0
            || self.world_span <= 0.0
            || self.zoom <= 0.0
            || !self.zoom.is_finite()
        {
            return Err(InputError::InvalidScreenPoint);
        }
        if let Some(projection) = self.projection {
            return projection
                .screen_to_world(point)
                .ok_or(InputError::InvalidScreenPoint);
        }
        let scale = self.world_span / self.zoom;
        Ok([
            self.pan[0] + (point[0] / self.width - 0.5) * scale,
            self.pan[1] + (point[1] / self.height - 0.5) * scale,
        ])
    }

    /// 世界 `f64` 坐标 → 屏幕像素坐标（用于框选 / 命中测试，不进权威状态）。
    pub fn world_to_screen(&self, world: [f64; 2]) -> [f64; 2] {
        if let Some(projection) = self.projection {
            return projection.world_to_screen(world);
        }
        let scale = (self.world_span / self.zoom).max(f64::MIN_POSITIVE);
        [
            ((world[0] - self.pan[0]) / scale + 0.5) * self.width,
            ((world[1] - self.pan[1]) / scale + 0.5) * self.height,
        ]
    }

    /// 单位占位标记在屏幕上的保守包围盒，供框选 / 命中 / HUD 使用。
    pub fn unit_screen_bounds(&self, world: [f64; 2]) -> ([f64; 2], [f64; 2]) {
        if let Some(projection) = self.projection {
            return projection.unit_screen_bounds(world);
        }
        let center = self.world_to_screen(world);
        (
            [center[0] - PICK_RADIUS, center[1] - PICK_RADIUS],
            [center[0] + PICK_RADIUS, center[1] + PICK_RADIUS],
        )
    }

    /// 屏幕坐标 → 冻结的 `SimFixed` 世界坐标，走既有冻结边界（ADR 0004）。
    pub fn freeze_point(&self, point: [f64; 2]) -> Result<(SimFixed, SimFixed), InputError> {
        let world = self.screen_to_world(point)?;
        let x = freeze_f64(world[0]).map_err(InputError::Frozen)?;
        let y = freeze_f64(world[1]).map_err(InputError::Frozen)?;
        Ok((x, y))
    }
}

/// 中性的输入动作：宿主把 winit 事件归一化为此枚举，翻译层不依赖 winit。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputAction {
    /// 左键按下，开始框选（只改表现层状态）。
    SelectStart([f64; 2]),
    /// 左键拖拽，更新框选矩形。
    SelectDrag([f64; 2]),
    /// 左键释放，提交选中集合；不生成任何命令。
    SelectEnd([f64; 2]),
    /// 右键：拾取到敌方单位则攻击，否则移动到该点。
    ContextAction([f64; 2]),
    /// 施法（`Q`）：以选中首个单位为施法者、该屏幕点拾取的敌方单位为目标。
    Cast([f64; 2]),
    /// 生产（`T`）：以选中首个单位为生产者，训练战斗单位。
    Train,
    /// 取消生产（`C`）：以选中首个单位为生产者，取消队首。
    CancelTrain,
    /// 采集（`G`）：对选中农民逐一下达 `Gather`。
    Gather,
    /// 建造（`B`）：以选中单位为生产者，建造兵营。
    Build,
    /// 停止（`H`）：清空选中单位当前订单。
    Stop,
    /// 编组（F1..F9）：把当前选中集合记入表现层编组。
    AssignGroup(u8),
    /// 按组选择（1..9）：恢复表现层编组选中集合。
    SelectGroup(u8),
}

/// 表现层输入状态：选中集合、框选矩形、命令序号。全部非权威。
#[derive(Clone, Debug)]
pub struct InputState {
    local_player: u8,
    selection: BTreeSet<u64>,
    drag_start: Option<[f64; 2]>,
    drag_current: Option<[f64; 2]>,
    next_sequence: u32,
    /// 编组（表现层专属，不进权威状态 / 哈希 / 快照）。
    groups: BTreeMap<u8, BTreeSet<u64>>,
    /// 群体移动分组号（非 0，按稳定实体 ID 分配落点）。
    next_group: u32,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new(LOCAL_PLAYER)
    }
}

impl InputState {
    /// 创建空选中集合的输入状态。
    pub fn new(local_player: u8) -> Self {
        Self {
            local_player,
            selection: BTreeSet::new(),
            drag_start: None,
            drag_current: None,
            next_sequence: 1,
            groups: BTreeMap::new(),
            next_group: 1,
        }
    }

    /// 取下一个非 0 群体移动分组号；表现层单调递增，不进权威状态。
    fn take_group(&mut self) -> u32 {
        let group = self.next_group.max(1);
        self.next_group = group.wrapping_add(1).max(1);
        group
    }

    /// 只读选中集合（`BTreeSet` 稳定顺序，不引入 `HashMap` 迭代顺序）。
    pub fn selection(&self) -> &BTreeSet<u64> {
        &self.selection
    }

    /// 当前框选矩形（屏幕像素坐标），供 HUD 绘制选中框。
    pub fn drag_rect(&self) -> Option<([f64; 2], [f64; 2])> {
        self.drag_start.zip(self.drag_current)
    }

    /// 本地玩家编号。
    pub fn local_player(&self) -> u8 {
        self.local_player
    }

    /// 取下一个命令序号（表现层单调递增，按冻结帧号提交）。
    fn take_sequence(&mut self) -> u32 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        sequence
    }

    /// 由冻结逻辑帧号推导执行帧；`tick` 来自只读 `WorldView`，不是墙钟 /
    /// 呈现帧号（ADR 0002 §2、§3）。
    fn execute_frame(&self, view: &WorldView) -> Frame {
        Frame::new(view.tick.saturating_add(LEAD_FRAMES))
    }

    fn simulation(&mut self, view: &WorldView, kind: CommandKind) -> ClientCommand {
        ClientCommand::Simulation(Command::new(
            self.execute_frame(view),
            self.local_player,
            self.take_sequence(),
            kind,
        ))
    }

    fn gameplay(&mut self, view: &WorldView, kind: GameplayCommandKind) -> ClientCommand {
        ClientCommand::Gameplay(GameplayCommand::new(
            self.execute_frame(view),
            self.local_player,
            self.take_sequence(),
            kind,
        ))
    }

    /// 提交框选：只更新表现层选中集合，MUST NOT 生成命令、不改状态哈希。
    fn commit_selection(&mut self, view: &WorldView, mapper: &ScreenMapper) {
        self.selection.clear();
        let (start, end) = match self.drag_rect() {
            Some(rect) => rect,
            None => return,
        };
        let min_x = start[0].min(end[0]);
        let max_x = start[0].max(end[0]);
        let min_y = start[1].min(end[1]);
        let max_y = start[1].max(end[1]);
        // 阈值只区分「单击」与「拖框」，命中判定改用单位标记屏幕包围盒。
        let is_click = (max_x - min_x).abs() < 4.0 && (max_y - min_y).abs() < 4.0;

        for entity in &view.entities {
            if !entity.alive {
                continue;
            }
            let world = [
                entity.x as f64 / SCALE as f64,
                entity.y as f64 / SCALE as f64,
            ];
            // 单一真值源：用 Scene 真实模型顶点投影的实体屏幕包围盒判定命中，
            // 替代旧 `unit_screen_bounds`（24 世界单位四面体估算，放大后近乎
            // 全屏、拖框选不准）。未注入 boxes 时回退旧口径，既有测试零改动。
            let (unit_min, unit_max) = mapper.entity_screen_box(entity.id, world);
            let inside = if is_click {
                end[0] >= unit_min[0] - PICK_RADIUS
                    && end[0] <= unit_max[0] + PICK_RADIUS
                    && end[1] >= unit_min[1] - PICK_RADIUS
                    && end[1] <= unit_max[1] + PICK_RADIUS
            } else {
                unit_min[0] <= max_x
                    && unit_max[0] >= min_x
                    && unit_min[1] <= max_y
                    && unit_max[1] >= min_y
            };
            if inside {
                self.selection.insert(entity.id);
            }
        }
        self.drag_start = None;
        self.drag_current = None;
    }

    /// 拾取屏幕点处的敌方单位（`owner != local_player`），返回其 `EntityId`。
    fn pick_enemy(
        &self,
        view: &WorldView,
        mapper: &ScreenMapper,
        point: [f64; 2],
    ) -> Option<EntityId> {
        let mut best: Option<(f64, u64)> = None;
        for entity in &view.entities {
            if !entity.alive || entity.owner == self.local_player {
                continue;
            }
            let world = [
                entity.x as f64 / SCALE as f64,
                entity.y as f64 / SCALE as f64,
            ];
            let (unit_min, unit_max) = mapper.entity_screen_box(entity.id, world);
            let inside = point[0] >= unit_min[0] - PICK_RADIUS
                && point[0] <= unit_max[0] + PICK_RADIUS
                && point[1] >= unit_min[1] - PICK_RADIUS
                && point[1] <= unit_max[1] + PICK_RADIUS;
            if inside {
                let center = mapper.entity_screen_center(entity.id, world);
                let distance = (center[0] - point[0]).hypot(center[1] - point[1]);
                if best.is_none_or(|(d, _)| distance < d) {
                    best = Some((distance, entity.id));
                }
            }
        }
        let (_, id) = best?;
        EntityId::new(id)
    }

    fn first_selected(&self) -> Option<EntityId> {
        self.selection
            .iter()
            .copied()
            .next()
            .and_then(EntityId::new)
    }

    /// 处理一个归一化输入动作，返回需要提交的命令（可能为空）。
    ///
    /// `view` 与 `events` 都是只读输入；事件仅用于展示，不参与命令生成。
    /// 提交失败由宿主显式报告，本层对不满足前置条件的动作显式返回错误。
    pub fn apply(
        &mut self,
        action: InputAction,
        view: &WorldView,
        mapper: &ScreenMapper,
        _events: &[Event],
    ) -> Result<Vec<ClientCommand>, InputError> {
        match action {
            InputAction::SelectStart(point) => {
                self.drag_start = Some(point);
                self.drag_current = Some(point);
                Ok(Vec::new())
            }
            InputAction::SelectDrag(point) => {
                if self.drag_start.is_some() {
                    self.drag_current = Some(point);
                }
                Ok(Vec::new())
            }
            InputAction::SelectEnd(point) => {
                self.drag_current = Some(point);
                self.commit_selection(view, mapper);
                // 选中是表现层状态：不生成命令、哈希不变。
                Ok(Vec::new())
            }
            InputAction::ContextAction(point) => {
                if self.selection.is_empty() {
                    return Err(InputError::NoSelection);
                }
                let selected: Vec<EntityId> = self
                    .selection
                    .iter()
                    .copied()
                    .filter_map(EntityId::new)
                    .collect();
                if let Some(target) = self.pick_enemy(view, mapper, point) {
                    Ok(selected
                        .into_iter()
                        .map(|attacker| {
                            self.simulation(view, CommandKind::Attack { attacker, target })
                        })
                        .collect())
                } else {
                    let (x, y) = mapper.freeze_point(point)?;
                    // 多选时共用一个非 0 分组号，由内核按稳定实体 ID 分配落点。
                    let group = if selected.len() > 1 {
                        self.take_group()
                    } else {
                        0
                    };
                    Ok(selected
                        .into_iter()
                        .map(|entity| {
                            self.simulation(
                                view,
                                CommandKind::Move {
                                    entity,
                                    x,
                                    y,
                                    group,
                                    append: false,
                                },
                            )
                        })
                        .collect())
                }
            }
            InputAction::Cast(point) => {
                let caster = self.first_selected().ok_or(InputError::NoSelection)?;
                let target = self
                    .pick_enemy(view, mapper, point)
                    .ok_or(InputError::NoTarget)?;
                Ok(vec![self.gameplay(
                    view,
                    GameplayCommandKind::Cast {
                        caster,
                        ability: DEFAULT_ABILITY,
                        target,
                    },
                )])
            }
            InputAction::Train => {
                let producer = self.first_selected().ok_or(InputError::NoSelection)?;
                Ok(vec![self.gameplay(
                    view,
                    GameplayCommandKind::Train {
                        producer,
                        unit_def: DEFAULT_UNIT_DEF,
                        is_building: false,
                    },
                )])
            }
            InputAction::CancelTrain => {
                let producer = self.first_selected().ok_or(InputError::NoSelection)?;
                Ok(vec![self.gameplay(
                    view,
                    GameplayCommandKind::CancelTrain { producer, index: 0 },
                )])
            }
            InputAction::Gather => {
                if self.selection.is_empty() {
                    return Err(InputError::NoSelection);
                }
                let selected: Vec<EntityId> = self
                    .selection
                    .iter()
                    .copied()
                    .filter_map(EntityId::new)
                    .collect();
                Ok(selected
                    .into_iter()
                    .map(|unit| self.gameplay(view, GameplayCommandKind::Gather { unit }))
                    .collect())
            }
            InputAction::Build => {
                let producer = self.first_selected().ok_or(InputError::NoSelection)?;
                Ok(vec![self.gameplay(
                    view,
                    GameplayCommandKind::Train {
                        producer,
                        unit_def: BARRACKS,
                        is_building: true,
                    },
                )])
            }
            InputAction::Stop => {
                if self.selection.is_empty() {
                    return Err(InputError::NoSelection);
                }
                let selected: Vec<EntityId> = self
                    .selection
                    .iter()
                    .copied()
                    .filter_map(EntityId::new)
                    .collect();
                Ok(selected
                    .into_iter()
                    .map(|entity| self.simulation(view, CommandKind::Stop { entity }))
                    .collect())
            }
            InputAction::AssignGroup(group) => {
                self.groups.insert(group, self.selection.clone());
                Ok(Vec::new())
            }
            InputAction::SelectGroup(group) => {
                if let Some(members) = self.groups.get(&group) {
                    self.selection = members.clone();
                }
                Ok(Vec::new())
            }
        }
    }
}
