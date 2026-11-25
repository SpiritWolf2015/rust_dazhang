//! winit 键鼠 → 权威命令的翻译层（tasks 5.1–5.6、design D3）。
//!
//! 本模块是**纯表现层逻辑**：只消费只读 [`WorldView`] 与屏幕坐标，产出
//! [`ClientCommand`]（普通命令 / 玩法命令）。它不持有 `RtsMatch`、不写入任何
//! 权威字段；命令最终只经 `submit_command` / `submit_gameplay_command` 提交。
//!
//! 选中集合、框选矩形、序号计数器一律是表现层专属状态，MUST NOT 进入
//! FNV-1a 状态哈希或 `SNAPSHOT_VERSION`（当前 9）快照。执行帧由冻结的
//! 整数帧号推导（`WorldView::tick + LEAD_FRAMES`），MUST NOT 用墙钟或呈现帧号。
//!
//! 屏幕 `f64` → 世界 `f64` 的拾取运算只发生在表现层；任何将进入权威状态的
//! 坐标都 MUST 经既有冻结边界 [`w3_app::freeze_f64`]（ADR 0004）转成 `SimFixed`，
//! 不新增浮点到权威状态的路径。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use w3_app::freeze_f64;
use w3_core::{
    Command, CommandKind, DefId, EntityId, Event, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, SCALE, SimFixed, WorldView,
};

use crate::melee::command_card::{CardCommand, CardId, CardKind, CommandCatalog};
use crate::melee::defs::BARRACKS;

/// 表现层只读可移动性映射：`EntityId → 是否可移动`。
///
/// 装配期由 client 侧冻结定义 [`FrozenDefinitions::units`] 的
/// `UnitDef::speed_raw` 派生（design D1），经 [`crate::app::ClientApp`]
/// 注入输入翻译层。该映射是表现层专属数据，MUST NOT 进入 FNV-1a 状态哈希、
/// `SNAPSHOT_VERSION`（当前 9）快照或任何权威状态字段（ADR 0002）。
/// 本映射只登记 frame-0 装配实体；群体 `Move` 前 MUST 经
/// [`effective_movability`] 用只读视图补齐运行时动态实体判据，再把产出的
/// 有效映射交给 [`filter_movable`]，不做类型白名单推断。
pub type MovabilityMap = BTreeMap<u64, bool>;

/// 装配期从 client 侧冻结定义派生 `EntityId → 可移动性` 映射（design D1）。
///
/// 判据是 `UnitDef::speed_raw > 0`：`speed_raw == 0` 的建筑 / 资源点不可移动。
/// `entity_defs` 是装配层记录的 `EntityId → DefId` 关联（不是运行时视图）。
/// 定义缺失的实体按不可移动处理（显式保守，不放行未知类型）。
pub fn derive_movability(
    defs: &FrozenDefinitions,
    entity_defs: &BTreeMap<u64, DefId>,
) -> MovabilityMap {
    entity_defs
        .iter()
        .map(|(&id, &def_id)| {
            let movable = defs
                .units()
                .get(&def_id)
                .is_some_and(|unit| unit.speed_raw > 0);
            (id, movable)
        })
        .collect()
}

/// 按只读可移动性映射从选中集合筛出可生成 `Move` 的实体（design D1）。
///
/// `BTreeSet` 迭代即稳定 `EntityId` 顺序，落点分配只作用于筛后集合；
/// 映射显式登记为 `false` 的建筑 / 金矿 / 树木 MUST NOT 生成 `Move`。
pub fn filter_movable(selection: &BTreeSet<u64>, movability: &MovabilityMap) -> Vec<u64> {
    selection
        .iter()
        .copied()
        .filter(|id| movability.get(id).copied().unwrap_or(true))
        .collect()
}

/// 合并装配期映射与当前只读视图，产出群体 `Move` 使用的有效可移动性映射
/// （design D2 / D3 分层判据，R1.4 / R13.6）。
///
/// 判据按**实体来源分层**，两条来源 MUST NOT 互相替代：
/// - `base` 已登记的 frame-0 装配实体**原样保留**（P8.0 D1 权威判据，
///   来自 `UnitDef.speed_raw`）。`Builder::spawn` 因 `CommandQueue::submit`
///   的 `speed > 0` 校验把 frame-0 建筑的 `speed` 抬到 `max(1)`，其运行时
///   视图 `speed > 0` 无法区分建筑，故 MUST NOT 用视图 speed 覆盖已登记项，
///   否则 R1 bug 从 frame-0 路径复发。
/// - `base` 未登记的运行时动态实体（`Train` / `Build` 产出，不在装配期
///   `def_ids` 关联中）按只读 `EntityView.speed != 0` 补一条判据：内核
///   `complete_production` 以真实 `def.speed_raw` 写入 `UnitState.speed`
///   （建筑 `0`、单位 `> 0`），视图 speed 在此才是可靠判据。
///
/// 纯函数、每次 `apply` 从当前 `view` 现算，不持久化映射、不监听生产事件、
/// 不反写模拟状态；有效映射是表现层只读派生数据，MUST NOT 进入 FNV-1a
/// 状态哈希、`SNAPSHOT_VERSION`（当前 9）快照或任何权威状态字段（ADR 0002）。
/// 已登记项优先（原样保留），仅对未登记项按视图 speed 补齐，因此默认空映射
/// （`ClientApp::new`）下 view 里所有实体也都有判据，不再等价「全部可移动」
/// 缺陷态（design D5）。
pub fn effective_movability(base: &MovabilityMap, view: &WorldView) -> MovabilityMap {
    let mut effective = base.clone();
    for entity in &view.entities {
        effective
            .entry(entity.id)
            .or_insert_with(|| entity.speed != 0);
    }
    effective
}

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
    /// 经典键 / 命令卡点击统一归一化：只携带被按下的字母，由当前卡的
    /// 「键 → action」表在 `apply` 内翻译（R13.2 / D1）。键盘与点击都回传它，
    /// MUST NOT 携带「命令来源标记」进命令面（C2 / R13.6）。
    Hotkey(char),
    /// `M` 移动瞄准：进入表现层瞄准态，不生成命令。
    MoveAim,
    /// `A` 攻击瞄准：进入表现层瞄准态（仅既有 `Attack` 指定目标入口）。
    AttackAim,
    /// 瞄准 / 放置态下的左键定点：复用右键同一命令生成处（D5 / R13.6）。
    AimPoint([f64; 2]),
    /// `Esc`：清空子菜单栈 / 放置态 / 瞄准态 / 队列，不生成命令（R13.3）。
    CancelMenu,
    /// `Tab`：混编同类型子组切换（表现层游标，R13.4）。
    CycleSubgroup,
    /// `Shift` 队列开关：true 时后续 `Move` 经既有 `append` 字段排队。
    SetQueue(bool),
}

/// 表现层放置态内容：待左键定点的既有 `Train { is_building }` 参数。
///
/// 纯表现层局部状态，MUST NOT 进入权威状态 / FNV-1a 状态哈希 /
/// `SNAPSHOT_VERSION`（当前 9）快照（D3 / ADR 0002）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingPlacement {
    pub unit_def: u32,
    pub is_building: bool,
}

/// 表现层瞄准态：`M` 移动 / `A` 攻击的待定点状态。
///
/// 纯表现层局部状态，MUST NOT 进入权威状态 / 状态哈希 / 快照（D3 / ADR 0002）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AimState {
    None,
    Move,
    Attack,
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
    /// 最近一次输入是否因「选中集合全为不可移动实体」而无可执行目标。
    /// 表现层只读反馈位，MUST NOT 进入权威状态 / 状态哈希 / 快照。
    no_movable_target: bool,
    /// 子菜单栈：当前卡 ID 序列。空表示按选中集合类型定主卡；栈顶为
    /// 当前子卡。纯表现层局部状态，MUST NOT 进入权威状态 / FNV-1a 状态
    /// 哈希 / `SNAPSHOT_VERSION`（当前 9）快照（D3 / ADR 0002 / R13.3）。
    submenu_stack: Vec<CardId>,
    /// 放置态：待左键定点的建造命令（既有 `Train { is_building }` 参数）。
    /// 纯表现层局部状态，MUST NOT 进入权威状态 / 状态哈希 / 快照（D3）。
    placement: Option<PendingPlacement>,
    /// 瞄准态：`M` 移动 / `A` 攻击的待定点状态。纯表现层局部状态，
    /// MUST NOT 进入权威状态 / 状态哈希 / 快照（D5 / ADR 0002）。
    aim: AimState,
    /// 当前卡的「键 → action」表；每次 `apply` 由当前卡 `clear()` 后重建，
    /// 使同一字母随卡与选中集合决定含义、旧卡绑定失效（R13.2 / R13.3）。
    /// 纯表现层局部状态，MUST NOT 进入权威状态 / 状态哈希 / 快照。
    key_actions: BTreeMap<char, CardCommand>,
    /// 当前卡 ID（子菜单栈顶或按选中集合类型定的主卡），表现层只读。
    current_card: CardId,
    /// 混编选中集合的同类型子组游标（`Tab`），表现层只读。
    subgroup_cursor: usize,
    /// `Shift` 队列开关：true 时后续 `Move` 命令带 `append: true` 入队，
    /// 逐条经既有命令面、不改 `ExecuteFrame → PlayerId → Sequence →
    /// Arrival` 排序键（R13.4 / ADR 0002）。表现层只读。
    queue_append: bool,
    /// 注入的表现层只读定义表（DIP）：按钮 / 键位 / 格位单一真值源，
    /// 同时作为反向验证注入退化目录的接缝。不进权威状态 / 哈希 / 快照。
    catalog: CommandCatalog,
    /// 注入的装配层 `EntityId → DefId` 只读关联，用于按定义表派生当前卡与
    /// 子组分类（Risks / tasks 1.4）。表现层只读，MUST NOT 进权威状态。
    entity_defs: BTreeMap<u64, u32>,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new(LOCAL_PLAYER)
    }
}

impl InputState {
    /// 创建空选中集合的输入状态。
    pub fn new(local_player: u8) -> Self {
        let catalog = CommandCatalog::standard();
        let key_actions = catalog.key_actions(CardId::Main);
        Self {
            local_player,
            selection: BTreeSet::new(),
            drag_start: None,
            drag_current: None,
            next_sequence: 1,
            groups: BTreeMap::new(),
            next_group: 1,
            no_movable_target: false,
            submenu_stack: Vec::new(),
            placement: None,
            aim: AimState::None,
            key_actions,
            current_card: CardId::Main,
            subgroup_cursor: 0,
            queue_append: false,
            catalog,
            entity_defs: BTreeMap::new(),
        }
    }

    /// 注入装配层 `EntityId → DefId` 只读关联（表现层数据，D2 / tasks 1.4）。
    /// 用于由定义表派生当前命令卡与混编子组分类；不改动既有 `new` 签名、
    /// 未注入时未登记实体按通用类型派生，不进权威状态 / 哈希 / 快照。
    pub fn with_entity_defs(mut self, defs: BTreeMap<u64, u32>) -> Self {
        self.entity_defs = defs;
        self
    }

    /// 就地注入装配层 `EntityId → DefId` 只读关联，并立即按当前选中集合
    /// 重建当前卡 / 键表（`ClientApp::with_entity_defs` 装配接线用，B1）。
    ///
    /// 与 HUD 显示卡的 `entity_defs` 同源、消除两个真值源分叉；只改表现层
    /// 局部状态，MUST NOT 进权威状态 / FNV-1a 状态哈希 / 快照。
    /// `ClientApp::new`（未注入）保持既有空关联行为不变。
    pub fn set_entity_defs(&mut self, defs: BTreeMap<u64, u32>) {
        self.entity_defs = defs;
        self.refresh_card();
    }

    /// 注入表现层只读定义表（DIP）。生产实现用 `CommandCatalog::standard()`；
    /// 本接缝供反向验证注入退化目录（全局静态键表）经真实 `apply` 路径
    /// 证明「同一字母随卡切换」断言失败（C5 / D9）。
    pub fn with_catalog(mut self, catalog: CommandCatalog) -> Self {
        self.catalog = catalog;
        self.key_actions = self.catalog.key_actions(self.current_card);
        self
    }

    /// 就地切换命令卡定义表（Q2 按本地族只读查表刷新）。
    ///
    /// 选族是表现层本地只读状态：切换目录只重建当前卡的「键 → action」
    /// 表，MUST NOT 反写模拟状态、不进 FNV-1a 状态哈希 / 快照（ADR 0002）。
    /// 与 [`InputState::set_entity_defs`] 同源，供 HUD 显示侧与输入翻译侧
    /// 保持一致（B1 接线）。
    pub fn set_catalog(&mut self, catalog: CommandCatalog) {
        self.catalog = catalog;
        self.refresh_card();
    }

    /// 只读当前命令卡定义表（供验收 / 诊断读取按族目录）。
    pub fn catalog(&self) -> &CommandCatalog {
        &self.catalog
    }

    /// 当前卡 ID（表现层只读）。
    pub fn current_card(&self) -> CardId {
        self.current_card
    }

    /// 子菜单栈深度（表现层只读，`Esc` 清空后为 0）。
    pub fn submenu_depth(&self) -> usize {
        self.submenu_stack.len()
    }

    /// 当前放置态（表现层只读）。
    pub fn placement(&self) -> Option<PendingPlacement> {
        self.placement
    }

    /// 当前瞄准态（表现层只读）。
    pub fn aim(&self) -> AimState {
        self.aim
    }

    /// 当前 `Shift` 队列开关（表现层只读）。
    pub fn queue_append(&self) -> bool {
        self.queue_append
    }

    /// 当前混编子组游标（表现层只读）。
    pub fn subgroup_cursor(&self) -> usize {
        self.subgroup_cursor
    }

    /// 当前卡的「键 → action」表（表现层只读，供验收 / 反向验证核对）。
    pub fn key_actions(&self) -> &BTreeMap<char, CardCommand> {
        &self.key_actions
    }

    /// 由子菜单栈 + 选中集合类型定当前卡，并 `clear()` 后重建键表。
    ///
    /// 每次 `apply` 都重建，使同一字母随当前卡与选中集合决定含义、旧卡
    /// 绑定失效（R13.2 / R13.3）。纯表现层，零权威状态写入。
    fn refresh_card(&mut self) {
        let card = self.submenu_stack.last().copied().unwrap_or_else(|| {
            let kinds: Vec<CardKind> = self
                .selection
                .iter()
                .filter_map(|id| {
                    self.entity_defs
                        .get(id)
                        .map(|&def| self.catalog.kind_of(def))
                })
                .collect();
            self.catalog.primary_card(&kinds)
        });
        self.current_card = card;
        self.key_actions.clear();
        self.key_actions = self.catalog.key_actions(card);
    }

    /// 最近一次输入是否因全为不可移动实体而无可执行目标（明确反馈位）。
    ///
    /// 该反馈只在表现层供 HUD / 验收读取，不生成任何命令、不改写权威状态。
    pub fn no_movable_target(&self) -> bool {
        self.no_movable_target
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
    /// `view` / `events` / `movability` 都是只读输入；事件仅用于展示，
    /// 不参与命令生成。`movability` 是装配期由 client 侧冻结定义
    /// `UnitDef.speed_raw` 派生的表现层只读映射，只用于群体 `Move` 过滤
    /// （design D1），MUST NOT 进入权威状态 / 状态哈希 / 快照。
    /// 提交失败由宿主显式报告，本层对不满足前置条件的动作显式返回错误。
    pub fn apply(
        &mut self,
        action: InputAction,
        view: &WorldView,
        mapper: &ScreenMapper,
        _events: &[Event],
        movability: &MovabilityMap,
    ) -> Result<Vec<ClientCommand>, InputError> {
        self.no_movable_target = false;
        // 每次输入先按「当前卡 + 选中集合」重建键→action 表，键盘与
        // 命令卡点击都归一化进本方法、共用唯一命令生成处（D1 / D5）。
        self.refresh_card();
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
                // 右键与键盘 `M` / `A` 瞄准定点共用同一命令生成处：
                // 拾取敌方 → 既有 `Attack`，否则 → 经 R1.4 分层过滤的
                // 既有 `Move`（D5 / R13.6 / C3）。
                self.context_commands(point, view, mapper, movability, false)
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
            InputAction::Gather => self.gather_commands(view),
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
            InputAction::Stop => self.stop_commands(view),
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
            InputAction::Hotkey(key) => {
                // 键盘与命令卡点击都归一化到此：查当前卡键表得既有命令面
                // 动作，未绑定字母显式不生成命令（不静默降级）。
                let command = self.key_actions.get(&key).copied();
                match command {
                    Some(command) => self.apply_card_command(command, view),
                    None => Ok(Vec::new()),
                }
            }
            InputAction::MoveAim => self.apply_card_command(CardCommand::MoveAim, view),
            InputAction::AttackAim => self.apply_card_command(CardCommand::AttackAim, view),
            InputAction::AimPoint(point) => self.aim_point(point, view, mapper, movability),
            InputAction::CancelMenu => {
                self.cancel_menu();
                Ok(Vec::new())
            }
            InputAction::CycleSubgroup => {
                self.cycle_subgroup();
                Ok(Vec::new())
            }
            InputAction::SetQueue(append) => {
                self.queue_append = append;
                Ok(Vec::new())
            }
        }
    }

    // ---- 以下为表现层只读派生辅助方法：命令仍只经既有命令面产出 ----

    /// 右键地面 / 敌方目标与键盘 `M` / `A` 瞄准定点共用的唯一命令生成处。
    ///
    /// `require_enemy` 为 true 时（`A` 瞄准态）只认指定敌方目标入口，点地面
    /// 不生成命令——MUST NOT 化为 attack-move 点地面途中交战语义（R13.2）。
    /// 地面 `Move` 经 `effective_movability` 分层判据 + `filter_movable`
    /// 过滤，建筑 / 金矿 / 树木 / 运行时建筑 MUST NOT 生成 `Move`（R1.4 / C3）。
    fn context_commands(
        &mut self,
        point: [f64; 2],
        view: &WorldView,
        mapper: &ScreenMapper,
        movability: &MovabilityMap,
        require_enemy: bool,
    ) -> Result<Vec<ClientCommand>, InputError> {
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
                .map(|attacker| self.simulation(view, CommandKind::Attack { attacker, target }))
                .collect())
        } else if require_enemy {
            // 指定目标攻击入口点地面：不是 attack-move，不生成任何命令。
            Ok(Vec::new())
        } else {
            let (x, y) = mapper.freeze_point(point)?;
            // 群体 Move 用分层判据产出的有效映射过滤（design D2 / D3）：
            // frame-0 已登记项原样保留，运行时动态实体由只读视图 speed
            // 补齐；建筑 / 金矿 / 树木 / 运行时建筑在此被筛除，落点按
            // 筛后集合的稳定 EntityId 顺序分配，不因过滤改变。有效映射
            // 每次从当前只读视图现算，不持久化、不进权威状态。
            let effective = effective_movability(movability, view);
            let movers: Vec<EntityId> = filter_movable(&self.selection, &effective)
                .into_iter()
                .filter_map(EntityId::new)
                .collect();
            if movers.is_empty() {
                // 全为不可移动实体：不生成空命令、不改权威状态 / 哈希，
                // 以英文结构化日志明确报告并置反馈位（表现层只读）。
                log::warn!(
                    target: "w3_client",
                    "no movable target in selection: reason=all_immovable selected={}",
                    self.selection.len()
                );
                self.no_movable_target = true;
                return Ok(Vec::new());
            }
            // 多选时共用一个非 0 分组号，由内核按稳定实体 ID 分配落点。
            let group = if movers.len() > 1 {
                self.take_group()
            } else {
                0
            };
            // `append` 只来自表现层 `Shift` 队列开关；既有命令面 /
            // 排序键 / golden hash 不变（R13.4 / ADR 0002）。
            let append = self.queue_append;
            Ok(movers
                .into_iter()
                .map(|entity| {
                    self.simulation(
                        view,
                        CommandKind::Move {
                            entity,
                            x,
                            y,
                            group,
                            append,
                        },
                    )
                })
                .collect())
        }
    }

    /// 既有 `Gather` 命令生成（键盘 `G` / 命令卡点击与既有夹具共用）。
    fn gather_commands(&mut self, view: &WorldView) -> Result<Vec<ClientCommand>, InputError> {
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

    /// 既有 `Stop` 命令生成（`S` 只做 Stop，MUST NOT 合并 Hold，R13.2）。
    fn stop_commands(&mut self, view: &WorldView) -> Result<Vec<ClientCommand>, InputError> {
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

    /// 执行一个定义表动作（由 `Hotkey` 翻译或显式经典键到达）。
    ///
    /// 只映射到既有命令面：`Move` / `Attack` / `Stop` 与
    /// `Gather` / `Train` / `CancelTrain`；子菜单 / 放置态只改表现层局部
    /// 状态、不生成权威命令（D1 / D3 / R13.6）。
    fn apply_card_command(
        &mut self,
        command: CardCommand,
        view: &WorldView,
    ) -> Result<Vec<ClientCommand>, InputError> {
        match command {
            CardCommand::MoveAim => {
                self.aim = AimState::Move;
                Ok(Vec::new())
            }
            CardCommand::AttackAim => {
                self.aim = AimState::Attack;
                Ok(Vec::new())
            }
            CardCommand::Stop => self.stop_commands(view),
            CardCommand::Gather => self.gather_commands(view),
            CardCommand::OpenBuildMenu => {
                self.push_submenu(CardId::Build);
                Ok(Vec::new())
            }
            CardCommand::OpenTrainMenu => {
                self.push_submenu(CardId::Train);
                Ok(Vec::new())
            }
            CardCommand::TrainUnit {
                unit_def,
                is_building,
            } => {
                if is_building {
                    // 建造项转放置态，左键定点才产出既有 `Train`（R13.3）。
                    self.placement = Some(PendingPlacement {
                        unit_def,
                        is_building,
                    });
                    Ok(Vec::new())
                } else {
                    // 生产建筑训练键直接产出既有 `Train`。
                    let producer = self.first_selected().ok_or(InputError::NoSelection)?;
                    Ok(vec![self.gameplay(
                        view,
                        GameplayCommandKind::Train {
                            producer,
                            unit_def,
                            is_building: false,
                        },
                    )])
                }
            }
            CardCommand::CancelTrain => {
                let producer = self.first_selected().ok_or(InputError::NoSelection)?;
                Ok(vec![self.gameplay(
                    view,
                    GameplayCommandKind::CancelTrain { producer, index: 0 },
                )])
            }
        }
    }

    /// 子菜单入口入栈，并立即按栈顶重建键表（下一次 `apply` 的
    /// `refresh_card` 也会重建、结果一致）：使同一字母随卡切换、旧卡
    /// 绑定失效，且 getter 能即时反映当前卡（R13.2 / R13.3）。
    fn push_submenu(&mut self, card: CardId) {
        self.submenu_stack.push(card);
        self.current_card = card;
        self.key_actions.clear();
        self.key_actions = self.catalog.key_actions(card);
    }

    /// 瞄准 / 放置态下的左键定点。
    ///
    /// 放置态 → 产出既有 `Train { is_building: true }`；移动瞄准 → 复用
    /// `context_commands`（R1.4 过滤）；攻击瞄准 → 仅既有 `Attack` 指定
    /// 目标入口。定点后清空一次性瞄准 / 放置态，不生成新命令变体。
    fn aim_point(
        &mut self,
        point: [f64; 2],
        view: &WorldView,
        mapper: &ScreenMapper,
        movability: &MovabilityMap,
    ) -> Result<Vec<ClientCommand>, InputError> {
        if let Some(placement) = self.placement.take() {
            self.aim = AimState::None;
            let producer = self.first_selected().ok_or(InputError::NoSelection)?;
            // 既有 `Train` 命令面不携带放置坐标，定点只作表现层触发；
            // 坐标不进权威状态、不改哈希 / 快照（R13.3 / ADR 0002）。
            return Ok(vec![self.gameplay(
                view,
                GameplayCommandKind::Train {
                    producer,
                    unit_def: placement.unit_def,
                    is_building: placement.is_building,
                },
            )]);
        }
        match self.aim {
            AimState::Move => {
                self.aim = AimState::None;
                self.context_commands(point, view, mapper, movability, false)
            }
            AimState::Attack => {
                self.aim = AimState::None;
                self.context_commands(point, view, mapper, movability, true)
            }
            AimState::None => Ok(Vec::new()),
        }
    }

    /// `Esc` 取消：清空子菜单栈 / 放置态 / 瞄准态 / 队列，不生成命令，
    /// 并立即回主卡重建键表（R13.3）。
    fn cancel_menu(&mut self) {
        self.submenu_stack.clear();
        self.placement = None;
        self.aim = AimState::None;
        self.queue_append = false;
        self.refresh_card();
    }

    /// `Tab` 混编同类型子组切换：按装配层 `DefId` 把选中集合分子组，
    /// 游标循环后把表现层 `selection` 收窄到当前子组。命令分发仍按
    /// R1.4 过滤（建筑进入子组也不响应 `Move`）。不改权威状态 / 哈希。
    fn cycle_subgroup(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let mut groups: BTreeMap<u32, BTreeSet<u64>> = BTreeMap::new();
        for &id in &self.selection {
            let key = self.entity_defs.get(&id).copied().unwrap_or(0);
            groups.entry(key).or_default().insert(id);
        }
        let subgroups: Vec<&BTreeSet<u64>> = groups.values().collect();
        if subgroups.len() <= 1 {
            return;
        }
        self.subgroup_cursor = (self.subgroup_cursor + 1) % subgroups.len();
        self.selection = subgroups[self.subgroup_cursor].clone();
    }
}
