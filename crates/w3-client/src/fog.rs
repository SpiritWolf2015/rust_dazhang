//! 战争迷雾：表现层本地只读派生（P8.5e / R10.1 / R10.5 / §0.3 T3 / ADR 0002 §5）。
//!
//! 本模块是**表现层专属**，MUST NOT 进入 `w3-core` 权威状态、FNV-1a 状态哈希、
//! `SNAPSHOT_VERSION`（当前 7）定义的快照或 `first_difference()`。唯一的权威写入者
//! 仍是 `RtsMatch`；迷雾网格 / 当前可见 / 已探索层 / 雾遮罩全部是
//! [`FogGrid`] 内的表现层本地跨帧累积状态，每个本地视角一份（design D5）。
//!
//! 派生输入固定为：只读 `WorldView`（`tick` / `entities` 的 `x` / `y` /
//! `owner` / `alive`）+ 事实事件拷贝 + 只读 `Snapshot`（`creep_camps` /
//! `item_shop` / `defs`）+ 装配期 `entity_defs`。派生全程使用 `i64` / `i128`
//! 整数运算，MUST NOT 读取墙钟 / `SystemTime` / `Instant` / 渲染帧率 /
//! `HashMap` 迭代顺序 / `f64` / 表现层独立随机源；昼夜相位只由整数帧号
//! `tick % (day_frames + night_frames)` 派生（R10.1 / R10.2）。
//!
//! 视野半径昼 / 夜、昼夜时长、格子分辨率、视野阻挡、隐身触发 / 反隐半径、
//! 雾遮罩不透明度全部为**表现层合成原创编译期整数常量 `[待核]`**（B 类）。
//! 它们 MUST NOT 进 `w3-core::FrozenDefinitions` / `UnitDef` /
//! `compute_defs_hash` / `defs_hash` / `content_hash`：否则定义层内容变化会
//! 改变 golden hash 基线，与「保持 7 不 bump」自相矛盾（design D3 / Q6）。
//! 数值不含任何 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / Warsmash
//! `GRID_STEP` / `PATHING_RATIO` / `DYING_UNIT_VISION_RADIUS`、godot 网页值
//! 或 `melee.jui` JASS UI 帧定义值；不读 `config.toml`、零文件 IO（ADR 0001 / §14）。
//!
//! 参考项目对照（design.md「## 参考项目对照」，只读语义、零代码复制）：
//! 三态 / 单调累积 / 每帧降级重算借 Warsmash `CFogState` 与
//! `CPlayerFogOfWar` 语义（AGPLv3，仅语义）；小地图点「喂入前过滤」借
//! godot `SHARED_MINIMAP.md`（MIT，仅结构）；「派生缓存不进快照、恢复后重建」
//! 借 `rts_kernel_cs` `FRAMEWORK.md` / `MOVEMENT.md`（MIT，仅语义）。
//! `vera20k` 把 fog 进状态哈希 / 快照 / `first_difference()` 是**对照差异、
//! 明确不借鉴**（R10.5 / §0.3 T3）。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{DefId, Event, SCALE, Snapshot, WorldView, shop_fixture_id};

// ---------------------------------------------------------------------------
// 表现层合成原创编译期整数常量 `[待核]`（B 类，MUST NOT 进 FrozenDefinitions）
// ---------------------------------------------------------------------------

/// 迷雾网格每边格数 `[待核]`；合成原创整数，非任何参考项目数值。
pub const FOG_GRID_SIDE: usize = 64;

/// 每个迷雾格覆盖的世界跨度（Q47.16 raw）`[待核]`；合成原创整数。
pub const FOG_CELL_SPAN_RAW: i64 = 4 * SCALE;

/// 网格世界半跨度（Q47.16 raw）；由格子数与格跨度纯整数派生，居中于世界原点。
pub const FOG_HALF_EXTENT_RAW: i64 = (FOG_GRID_SIDE as i64) * FOG_CELL_SPAN_RAW / 2;

/// 昼间视野半径（Q47.16 raw）`[待核]`；合成原创整数，非权威 / 导出事实。
pub const FOG_SIGHT_DAY_RAW: i64 = 12 * SCALE;

/// 夜间视野半径（Q47.16 raw）`[待核]`；合成原创整数，昼值更大（R10.1 / R10.2）。
pub const FOG_SIGHT_NIGHT_RAW: i64 = 7 * SCALE;

/// 飞行单位视野半径（Q47.16 raw）`[待核]`；飞行视野不受地形限制（R5.6）。
pub const FOG_SIGHT_FLYING_RAW: i64 = 16 * SCALE;

/// 昼时长（整数逻辑帧）`[待核]`；参考既有冻结昼夜常量的量级、非权威复制。
pub const FOG_DAY_FRAMES: u64 = 600;

/// 夜时长（整数逻辑帧）`[待核]`；与昼时长合成同一周期，纯整数帧派生。
pub const FOG_NIGHT_FRAMES: u64 = 480;

/// MASKED 格雾遮罩不透明度（0..255）`[待核]`；合成原创整数。
pub const FOG_OVERLAY_OPACITY_MASKED: u8 = 224;

/// FOGGED 格雾遮罩不透明度（0..255）`[待核]`；合成原创整数。
pub const FOG_OVERLAY_OPACITY_FOGGED: u8 = 128;

/// VISIBLE 格雾遮罩不透明度 `[待核]`；完全透明、不遮挡地形底图。
pub const FOG_OVERLAY_OPACITY_VISIBLE: u8 = 0;

/// 视野阻挡半径 `[待核]`（Q2 / Q3 范围外承接：本期不接线到任何玩法行为）。
///
/// 仅登记为表现层合成原创编译期整数常量，MUST NOT 进 `FrozenDefinitions`；
/// R5.6 视野阻挡作为玩法行为、R10.4 隐身反隐、S3 暗夜夜间隐身、R8.4 野怪睡眠
/// 均不在本期最小闭环内（design「## 范围边界」/ Q2），不实现、不静默窄化。
pub const FOG_VISION_BLOCK_RADIUS_RAW: i64 = 5 * SCALE;

/// 隐身触发半径 `[待核]`（R10.4 范围外承接，本期不实现）。
pub const FOG_STEALTH_TRIGGER_RADIUS_RAW: i64 = 3 * SCALE;

/// 反隐半径 `[待核]`（R10.4 范围外承接，本期不实现）。
pub const FOG_DETECT_RADIUS_RAW: i64 = 9 * SCALE;

/// 合成飞行类别 `DefId` 集合 `[待核]`（表现层专用，非经典 rawcode / 权威定义）。
///
/// 只读视图 `EntityView` 没有 `def_id` / `is_flying`（Q5 不扩展投影字段），
/// 故飞行分类走装配期 `entity_defs` 查表 + 表现层合成常量；查不到的运行时
/// 动态实体按地面视野处理（Q5 已知限制：`entity_defs` 只覆盖 frame-0 实体）。
pub const FOG_FLYING_DEF_IDS: &[DefId] = &[97_001, 97_002];

// ---------------------------------------------------------------------------
// 三态网格
// ---------------------------------------------------------------------------

/// 迷雾格三态：未探索 MASKED / 已探索 FOGGED / 当前可见 VISIBLE（R10.1）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FogCell {
    /// 从未进入过视野；地形与实体均不可呈现。
    #[default]
    Masked,
    /// 曾进入过视野、当前不在视野；地形底图保留，敌方变化不可见。
    Fogged,
    /// 当前在己方存活单位视野内；地形与实体按本帧只读视图呈现。
    Visible,
}

impl FogCell {
    /// 稳定单字节编码，供只读投影 / 断言使用，不依赖枚举内存布局。
    pub const fn code(self) -> u8 {
        match self {
            Self::Masked => 0,
            Self::Fogged => 1,
            Self::Visible => 2,
        }
    }

    /// 是否当前可见（R10.1）；小地图单位点过滤的唯一可见性判据。
    pub const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }

    /// 是否「已探索但当前不可见」（FOGGED）；冻结上一已知态用（R10.1）。
    pub const fn is_fogged(self) -> bool {
        matches!(self, Self::Fogged)
    }

    /// 是否已探索（FOGGED / VISIBLE）；单调累积，MUST NOT 回退为 MASKED。
    pub const fn is_explored(self) -> bool {
        !matches!(self, Self::Masked)
    }
}

/// 昼夜相位（R10.2）；只由整数帧号派生，与墙钟 / 太阳时无关。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FogPhase {
    /// 昼：使用 [`FOG_SIGHT_DAY_RAW`]。
    #[default]
    Day,
    /// 夜：使用 [`FOG_SIGHT_NIGHT_RAW`]。
    Night,
}

impl FogPhase {
    /// 是否白昼。
    pub const fn is_day(self) -> bool {
        matches!(self, Self::Day)
    }

    /// 稳定单字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::Day => 0,
            Self::Night => 1,
        }
    }
}

/// 由整数帧号纯派生的昼夜相位：`phase = tick % (day_frames + night_frames)`。
pub const fn day_phase(tick: u64) -> FogPhase {
    if tick % (FOG_DAY_FRAMES + FOG_NIGHT_FRAMES) < FOG_DAY_FRAMES {
        FogPhase::Day
    } else {
        FogPhase::Night
    }
}

/// 视野半径数据驱动：昼 / 夜两值合成常量；飞行单位不受地形限制（R5.6）。
///
/// 本期最小闭环不实现任何地形遮挡（R5.6 视野阻挡作为玩法行为属 Q2 范围外），
/// 因此飞行类别只切换到 [`FOG_SIGHT_FLYING_RAW`]、天然不受地形裁切；
/// 地面单位按昼夜相位取对应半径。纯整数、无 `f64`、无独立随机源。
pub const fn sight_radius_raw(phase: FogPhase, flying: bool) -> i64 {
    if flying {
        FOG_SIGHT_FLYING_RAW
    } else if phase.is_day() {
        FOG_SIGHT_DAY_RAW
    } else {
        FOG_SIGHT_NIGHT_RAW
    }
}

/// 三态网格计数（只读投影用，owned 值拷贝）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FogCounts {
    pub masked: usize,
    pub fogged: usize,
    pub visible: usize,
    /// 已探索格数（FOGGED + VISIBLE），单调不减。
    pub explored: usize,
}

/// 只读 Snapshot 查表得到的中立实体类别（P8.5c / P8.5d 交接，R8.1 / R9.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FogNeutralCategory {
    /// 野怪营地（`Snapshot.creep_camps.camps` 的 home 锚点）。
    CreepCamp,
    /// 野怪单位（`Snapshot.creep_camps.creeps` / `Snapshot.positions`）。
    Creep,
    /// 地面掉落物品（`Snapshot.creep_camps.ground_items`，不是实体、不在 `read_view()`）。
    GroundItem,
    /// 中立商店（`Snapshot.defs.item_shop().shops` 冻结坐标 + `Snapshot.item_shop` 登记）。
    Shop,
}

impl FogNeutralCategory {
    /// 稳定单字节编码。
    pub const fn code(self) -> u8 {
        match self {
            Self::CreepCamp => 0,
            Self::Creep => 1,
            Self::GroundItem => 2,
            Self::Shop => 3,
        }
    }
}

/// 中立实体的只读投影标记（owned 值拷贝，位置来自只读 Snapshot）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FogNeutralMarker {
    pub id: u64,
    /// 世界坐标 x（Q47.16 raw）。
    pub x: i64,
    /// 世界坐标 y（Q47.16 raw）。
    pub y: i64,
    pub category: FogNeutralCategory,
}

/// 表现层本地三态迷雾网格：每个本地视角一份、跨帧累积（design D1 / D5）。
///
/// 结构上只持有 `Vec<FogCell>` 与只读派生的元数据，不持有 `RtsMatch` 句柄，
/// 因此**无法反写**权威状态；也 MUST NOT 被塞进 `Snapshot` /
/// `compute_state_hash()` / `first_difference()`（R10.5 / ADR 0002 §5）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FogGrid {
    cells: Vec<FogCell>,
    local_player: u8,
    last_tick: Option<u64>,
    phase: FogPhase,
    event_count: usize,
    snapshot_tick: u64,
    visible_markers: Vec<FogNeutralMarker>,
}

impl FogGrid {
    /// 干净环境构造：网格初始全 MASKED，零文件 IO、不读 `config.toml`。
    pub fn new(local_player: u8) -> Self {
        Self {
            cells: vec![FogCell::Masked; FOG_GRID_SIDE * FOG_GRID_SIDE],
            local_player,
            last_tick: None,
            phase: FogPhase::Day,
            event_count: 0,
            snapshot_tick: 0,
            visible_markers: Vec::new(),
        }
    }

    /// 网格每边格数（表现层合成常量 `[待核]`）。
    pub const fn side(&self) -> usize {
        FOG_GRID_SIDE
    }

    /// 只读三态格序列（行优先、稳定顺序），供回放逐帧比对与断言。
    pub fn cells(&self) -> &[FogCell] {
        &self.cells
    }

    /// 最近一次重算的整数帧号；未重算时返回 `None`。
    pub const fn last_tick(&self) -> Option<u64> {
        self.last_tick
    }

    /// 本地视角编号。
    pub const fn local_player(&self) -> u8 {
        self.local_player
    }

    /// 当前昼夜相位（纯整数帧派生）。
    pub const fn phase(&self) -> FogPhase {
        self.phase
    }

    /// 本批次事实事件拷贝的条数（只读消费，不改权威状态）。
    pub const fn event_count(&self) -> usize {
        self.event_count
    }

    /// 只读 Snapshot 最近帧号（只读一致性元数据，不进哈希 / 快照）。
    pub const fn snapshot_tick(&self) -> u64 {
        self.snapshot_tick
    }

    /// 当前 VISIBLE 格内的中立实体标记（已从只读 Snapshot 查表并过滤）。
    pub fn visible_markers(&self) -> &[FogNeutralMarker] {
        &self.visible_markers
    }

    /// 三态格计数（owned 值拷贝）。
    pub fn counts(&self) -> FogCounts {
        let mut counts = FogCounts::default();
        for cell in &self.cells {
            match cell {
                FogCell::Masked => counts.masked += 1,
                FogCell::Fogged => counts.fogged += 1,
                FogCell::Visible => counts.visible += 1,
            }
        }
        counts.explored = counts.fogged + counts.visible;
        counts
    }

    /// 世界坐标（Q47.16 raw）→ 行优先格索引；越界钳到边缘格，纯整数、确定性。
    fn cell_index(raw: i64) -> usize {
        let span = FOG_CELL_SPAN_RAW as i128;
        let half = FOG_HALF_EXTENT_RAW as i128;
        let shifted = (raw as i128) + half;
        let index = shifted.div_euclid(span);
        index.clamp(0, (FOG_GRID_SIDE - 1) as i128) as usize
    }

    /// 格中心的世界坐标（Q47.16 raw），用二倍坐标避免中点非整数，纯 `i128`。
    fn cell_center_raw(index: usize) -> i128 {
        let span = FOG_CELL_SPAN_RAW as i128;
        let half = FOG_HALF_EXTENT_RAW as i128;
        -half + (2 * index as i128 + 1) * span / 2
    }

    /// 查询世界坐标所在格的三态；只读、纯整数。
    pub fn cell_at(&self, x: i64, y: i64) -> FogCell {
        let gx = Self::cell_index(x);
        let gy = Self::cell_index(y);
        self.cells[gy * FOG_GRID_SIDE + gx]
    }

    /// 该世界坐标是否当前可见（R10.1）；小地图 / Scene 可见性过滤的统一判据。
    pub fn is_visible(&self, x: i64, y: i64) -> bool {
        self.cell_at(x, y).is_visible()
    }

    /// 当前可见的实体编号集合（供 `Scene` 动态实例过滤，R10.1 / design D6）。
    ///
    /// 己方单位按本地视角恒呈现；敌方 / 中立实体只在 VISIBLE 格内呈现。
    /// 在场性（`alive`）由调用方与可见性分两层处理；本方法只读视图。
    pub fn visible_entity_ids(&self, view: &WorldView) -> BTreeSet<u64> {
        let mut ids = BTreeSet::new();
        for entity in &view.entities {
            if !entity.alive {
                continue;
            }
            if entity.owner == self.local_player || self.is_visible(entity.x, entity.y) {
                ids.insert(entity.id);
            }
        }
        ids
    }

    /// 处于 FOGGED 格的「冻结上一已知态」实体编号集合（R10.1 / design D6）。
    ///
    /// 己方单位恒不属于冻结集合；MASKED 格实体不呈现，VISIBLE 格实体按本帧
    /// 只读 `WorldView` 重建最新状态，故只在 FOGGED 格保留上一已知态。
    /// 本方法只读，`BTreeSet` 稳定顺序、无墙钟 / 渲染帧率 / 随机源。
    pub fn remembered_entity_ids(&self, view: &WorldView) -> BTreeSet<u64> {
        let mut ids = BTreeSet::new();
        for entity in &view.entities {
            if !entity.alive || entity.owner == self.local_player {
                continue;
            }
            if self.cell_at(entity.x, entity.y).is_fogged() {
                ids.insert(entity.id);
            }
        }
        ids
    }

    /// 圆形揭示：把落在 `radius_raw` 半径内的格批量升为 VISIBLE。
    ///
    /// 只改表现层网格；稳定行优先遍历、`i128` 平方距离、无随机源、无 `f64`。
    /// MASKED 与 FOGGED 都升为 VISIBLE，但永不再写回 MASKED（单调累积）。
    fn reveal_circle(&mut self, x: i64, y: i64, radius_raw: i64) {
        let span = FOG_CELL_SPAN_RAW as i128;
        let radius = radius_raw as i128;
        let radius_sq = radius * radius;
        let gx = Self::cell_index(x) as i128;
        let gy = Self::cell_index(y) as i128;
        let reach = radius.div_euclid(span) + 1;
        let side = FOG_GRID_SIDE as i128;
        for dy in -reach..=reach {
            let cy = gy + dy;
            if !(0..side).contains(&cy) {
                continue;
            }
            let center_y = Self::cell_center_raw(cy as usize);
            let ddy = center_y - y as i128;
            for dx in -reach..=reach {
                let cx = gx + dx;
                if !(0..side).contains(&cx) {
                    continue;
                }
                let center_x = Self::cell_center_raw(cx as usize);
                let ddx = center_x - x as i128;
                if ddx * ddx + ddy * ddy <= radius_sq {
                    self.cells[(cy * side + cx) as usize] = FogCell::Visible;
                }
            }
        }
    }

    /// 每逻辑帧重算当前可见（R10.1 / design D1）：先把上一帧全部 VISIBLE
    /// **批量降为 FOGGED**，再按本帧己方存活单位与合成视野半径做圆形揭示。
    ///
    /// 派生输入 = 只读 `WorldView`（`tick` / `entities`）+ 事实事件拷贝 +
    /// 只读 `Snapshot` + 装配期 `entity_defs`；MUST 只读取、无 `RtsMatch`
    /// 字段写入路径、无墙钟 / `SystemTime` / `Instant` / 渲染帧率 /
    /// `HashMap` 迭代顺序 / `f64` / 表现层独立随机源。
    ///
    /// 同一 `tick` 重复调用是**幂等**的（不重复降级 / 揭示），故相机操作、
    /// 重绘、插值帧丢失、网络帧抖动 / 乱序 / 重传都 MUST NOT 驱动本派生
    /// （ADR 0002 §2-3 / ADR 0005 §4）；`tick` 回退同样不重算，回放从帧 0
    /// 按同一权威状态序列逐帧重放得到同一网格（R12.4 / design D5）。
    pub fn observe(
        &mut self,
        view: &WorldView,
        snapshot: &Snapshot,
        events: &[Event],
        entity_defs: &BTreeMap<u64, DefId>,
        local_player: u8,
    ) {
        if let Some(last) = self.last_tick {
            if view.tick <= last {
                return;
            }
        }
        // 1) 批量降级：VISIBLE → FOGGED；FOGGED 单调保留，绝不回退 MASKED。
        for cell in &mut self.cells {
            if *cell == FogCell::Visible {
                *cell = FogCell::Fogged;
            }
        }
        // 2) 昼夜相位只由整数帧号派生。
        self.phase = day_phase(view.tick);
        // 3) 按本帧己方存活单位 `x` / `y` / `owner` / `alive` 做圆形揭示。
        for entity in &view.entities {
            if !entity.alive || entity.owner != local_player {
                continue;
            }
            let flying = entity_defs
                .get(&entity.id)
                .is_some_and(|def_id| FOG_FLYING_DEF_IDS.contains(def_id));
            let radius = sight_radius_raw(self.phase, flying);
            self.reveal_circle(entity.x, entity.y, radius);
        }
        // 4) 承接 P8.5c / P8.5d：中立实体位置 / 类别经只读 Snapshot 查表，
        //    按当前可见格过滤；地面物品不是实体、MUST NOT 只靠 WorldView.entities。
        self.visible_markers = visible_neutral_markers(snapshot, self);
        // 5) 记录只读元数据（事实事件拷贝条数 / 只读快照帧号），不进哈希 / 快照。
        self.event_count = events.len();
        self.snapshot_tick = snapshot.tick;
        self.local_player = local_player;
        self.last_tick = Some(view.tick);
    }
}

/// 从只读 `Snapshot` 查表派生「当前 VISIBLE 格内」的中立实体标记。
///
/// 复用 `sim_match.rs:233-243` 既有「只读投影查表」范式：事实事件的 entity
/// 只是线索，位置 / 类别一律从 owned 只读 `Snapshot` 取得。地面物品 / 商店
/// 不是 `EntityStore` 实体、不出现在 `read_view()`（`creep.rs:34-38`），
/// MUST 走只读 Snapshot（R1.1 / R8.1 / R9.1 / R10.5）。
/// 纯只读、`BTreeMap` 稳定键序、无 `HashMap` 迭代顺序、无 `f64`、无随机源。
pub fn visible_neutral_markers(snapshot: &Snapshot, grid: &FogGrid) -> Vec<FogNeutralMarker> {
    let mut markers = Vec::new();

    // 营地：`Snapshot.creep_camps.camps` 的 home 锚点（R8.1）。
    for (&camp_id, camp) in &snapshot.creep_camps.camps {
        if grid.is_visible(camp.home_x, camp.home_y) {
            markers.push(FogNeutralMarker {
                id: u64::from(camp_id),
                x: camp.home_x,
                y: camp.home_y,
                category: FogNeutralCategory::CreepCamp,
            });
        }
    }

    // 野怪：当前位置取 `Snapshot.positions`，类别取 `Snapshot.creep_camps.creeps`
    // 登记（home 锚点兜底），MUST NOT 把中立敌对野怪误当可移动单位（R8.1 / R1.4）。
    for (&entity, creep) in &snapshot.creep_camps.creeps {
        let (x, y) = snapshot
            .positions
            .iter()
            .find(|record| record.entity == entity.value())
            .map(|record| (record.x, record.y))
            .unwrap_or((creep.home_x, creep.home_y));
        if grid.is_visible(x, y) {
            markers.push(FogNeutralMarker {
                id: entity.value(),
                x,
                y,
                category: FogNeutralCategory::Creep,
            });
        }
    }

    // 地面物品：只在 `Snapshot.creep_camps.ground_items` 登记，不在 read_view()
    // 实体集合中（creep.rs:34-38）；位置 / 类别 MUST 从这里查表（R9.1 / R10.5）。
    for (&entity, item) in &snapshot.creep_camps.ground_items {
        if grid.is_visible(item.x, item.y) {
            markers.push(FogNeutralMarker {
                id: entity.value(),
                x: item.x,
                y: item.y,
                category: FogNeutralCategory::GroundItem,
            });
        }
    }

    // 中立商店：`Snapshot.defs.item_shop().shops` 冻结坐标 +
    // `Snapshot.item_shop.refresh_counts` 活跃登记；商店不是实体（R9.1）。
    for (&shop_id, shop) in &snapshot.defs.item_shop().shops {
        if !snapshot.item_shop.refresh_counts.contains_key(&shop_id) {
            continue;
        }
        if grid.is_visible(shop.x, shop.y) {
            markers.push(FogNeutralMarker {
                id: shop_fixture_id(shop_id),
                x: shop.x,
                y: shop.y,
                category: FogNeutralCategory::Shop,
            });
        }
    }

    markers
}

/// 只读雾遮罩不透明度（纯整数、无资源 IO）；供 HUD / Scene 遮罩呈现。
///
/// MASKED / FOGGED / VISIBLE 三态分别取合成常量 `[待核]`；
/// VISIBLE 完全透明，底图 / 相机视口框独立于迷雾遮罩（R10.1）。
pub const fn overlay_opacity(cell: FogCell) -> u8 {
    match cell {
        FogCell::Masked => FOG_OVERLAY_OPACITY_MASKED,
        FogCell::Fogged => FOG_OVERLAY_OPACITY_FOGGED,
        FogCell::Visible => FOG_OVERLAY_OPACITY_VISIBLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::EntityView;

    /// 合成一个己方存活单位视图；整数坐标、无浮点、无墙钟。
    fn own_entity(id: u64, x: i64, y: i64) -> EntityView {
        EntityView {
            id,
            x,
            y,
            facing: 0,
            speed: 3 * SCALE,
            hp: 100,
            owner: 1,
            alive: true,
            has_target: false,
        }
    }

    #[test]
    fn grid_starts_fully_masked_without_file_io() {
        let grid = FogGrid::new(1);
        assert_eq!(grid.side(), FOG_GRID_SIDE, "reason=synthetic_grid_side");
        assert_eq!(
            grid.cells().len(),
            FOG_GRID_SIDE * FOG_GRID_SIDE,
            "reason=full_grid"
        );
        assert!(
            grid.cells().iter().all(|cell| *cell == FogCell::Masked),
            "reason=initial_all_masked"
        );
        assert_eq!(grid.counts().masked, FOG_GRID_SIDE * FOG_GRID_SIDE);
        assert_eq!(grid.counts().explored, 0);
        assert_eq!(grid.last_tick(), None);
    }

    #[test]
    fn day_phase_is_pure_integer_frame_modulo() {
        assert!(day_phase(0).is_day(), "reason=tick_zero_is_day");
        assert!(
            day_phase(FOG_DAY_FRAMES - 1).is_day(),
            "reason=last_day_frame"
        );
        assert!(
            !day_phase(FOG_DAY_FRAMES).is_day(),
            "reason=first_night_frame"
        );
        assert!(
            day_phase(FOG_DAY_FRAMES + FOG_NIGHT_FRAMES).is_day(),
            "reason=period_wraps"
        );
    }

    #[test]
    fn sight_radius_is_data_driven_by_phase_and_flying() {
        assert_eq!(
            sight_radius_raw(FogPhase::Day, false),
            FOG_SIGHT_DAY_RAW,
            "reason=day_ground"
        );
        assert_eq!(
            sight_radius_raw(FogPhase::Night, false),
            FOG_SIGHT_NIGHT_RAW,
            "reason=night_ground"
        );
        assert_eq!(
            sight_radius_raw(FogPhase::Day, true),
            FOG_SIGHT_FLYING_RAW,
            "reason=flying_ignores_terrain"
        );
        assert_eq!(
            sight_radius_raw(FogPhase::Night, true),
            FOG_SIGHT_FLYING_RAW,
            "reason=flying_night_unchanged"
        );
    }

    /// 合成一个敌方存活单位视图；整数坐标、无墙钟、无渲染帧率。
    fn enemy_entity(id: u64, x: i64, y: i64) -> EntityView {
        EntityView {
            id,
            x,
            y,
            facing: 0,
            speed: 3 * SCALE,
            hp: 100,
            owner: 2,
            alive: true,
            has_target: false,
        }
    }

    #[test]
    fn explored_layer_never_rolls_back_to_masked() {
        let mut grid = FogGrid::new(1);
        grid.reveal_circle(0, 0, FOG_SIGHT_DAY_RAW);
        let before = grid.counts();
        assert!(before.visible > 0, "reason=reveal_creates_visible_cells");
        assert!(before.explored > 0, "reason=visible_implies_explored");
        // 模拟每帧「先降级再揭示」：VISIBLE 批量降为 FOGGED，不得回退 MASKED。
        for cell in &mut grid.cells {
            if *cell == FogCell::Visible {
                *cell = FogCell::Fogged;
            }
        }
        let after = grid.counts();
        assert_eq!(after.visible, 0, "reason=visible_cells_are_downgraded");
        assert_eq!(after.fogged, before.visible, "reason=fogged_is_monotonic");
        assert_eq!(
            after.explored, before.explored,
            "reason=explored_layer_does_not_shrink"
        );
        assert_eq!(
            after.masked, before.masked,
            "reason=masked_cells_do_not_grow"
        );
    }

    #[test]
    fn render_visibility_sets_split_visible_fogged_and_masked_entities() {
        let mut grid = FogGrid::new(1);
        let row = FogGrid::cell_index(0);
        let own_col = FogGrid::cell_index(0);
        let fogged_col = FogGrid::cell_index(10 * FOG_CELL_SPAN_RAW);
        let masked_col = FogGrid::cell_index(20 * FOG_CELL_SPAN_RAW);
        grid.cells[row * FOG_GRID_SIDE + own_col] = FogCell::Visible;
        grid.cells[row * FOG_GRID_SIDE + fogged_col] = FogCell::Fogged;
        grid.cells[row * FOG_GRID_SIDE + masked_col] = FogCell::Masked;
        let view = WorldView {
            tick: 3,
            rng: 0,
            entities: vec![
                own_entity(1, 0, 0),
                enemy_entity(2, 10 * FOG_CELL_SPAN_RAW, 0),
                enemy_entity(3, 20 * FOG_CELL_SPAN_RAW, 0),
            ],
        };
        assert_eq!(
            grid.visible_entity_ids(&view),
            [1u64].into_iter().collect(),
            "reason=own_and_visible_cells_only"
        );
        assert_eq!(
            grid.remembered_entity_ids(&view),
            [2u64].into_iter().collect(),
            "reason=fogged_cells_keep_last_known_state"
        );
        assert!(
            !grid.visible_entity_ids(&view).contains(&3),
            "reason=masked_enemy_is_hidden"
        );
    }
}
