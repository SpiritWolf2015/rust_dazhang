//! 只读 HUD（P6 批次 B + `melee-playable-loop` 组 4 / design D4）。
//!
//! HUD 只消费 [`HudModel`]——由只读 `WorldView`、事实事件拷贝与只读快照
//! 派生的**纯表现层数据**。本模块不持有 `RtsMatch`、不引用任何权威状态
//! 句柄，因此结构上无法反写模拟（ADR 0002 §5）。
//!
//! - 选中框 / 血条 / 资源 / 生产队列 / 命令卡 / 选择面板 / 小地图都只从
//!   `HudModel` 读取；
//! - 实体类型标签由装配层注入，是表现层数据，MUST NOT 进入权威状态或哈希；
//! - UI 文案来自 [`crate::i18n`]，MUST NOT 进入日志、权威状态或状态哈希；
//! - 字体使用 egui 内置开源默认字体（`default_fonts`），不加载任何资产
//!   文件（ADR 0001）；内置字体无 CJK 字形，故文案为 ASCII；
//! - 字体 / 资源不可用时以英文结构化 `key=value` 日志报告并降级为不绘制
//!   文字元素，MUST NOT 改变模拟推进。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{Event, SCALE, Snapshot, WorldView};
use w3_render::{display_hints, team_color};

use crate::i18n::Texts;
use crate::input::ScreenMapper;
use crate::melee::EntityKind;

/// HUD 字体 / 资源；`text_enabled` 为 false 时降级为不绘制文字元素。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudAssets {
    pub text_enabled: bool,
}

/// HUD 资源准备失败；错误文本一律英文（CODEBUDDY.md §3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudAssetError {
    /// 内置字体数据为空，无法绘制任何文字标签。
    EmptyFontData,
}

impl std::fmt::Display for HudAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFontData => f.write_str("hud font data is empty"),
        }
    }
}

impl HudAssets {
    /// 用给定的字体定义准备 HUD。字体数据为空即视为资源不可用，显式报错。
    ///
    /// 字体数据来自 crate 内编译的定义（`FontDefinitions`），不读取任何文件、
    /// 不引入仓库资产；本 change 启用 `egui/default_fonts` 内置开源字体，
    /// 故默认定义非空、文字元素正常绘制。
    pub fn from_fonts(
        ctx: &egui::Context,
        fonts: egui::FontDefinitions,
    ) -> Result<Self, HudAssetError> {
        if fonts.font_data.is_empty() {
            return Err(HudAssetError::EmptyFontData);
        }
        ctx.set_fonts(fonts);
        Ok(Self { text_enabled: true })
    }

    /// 用 egui 内置默认字体准备 HUD：不加载任何文件、不引入资产。
    pub fn prepare(ctx: &egui::Context) -> Result<Self, HudAssetError> {
        Self::from_fonts(ctx, egui::FontDefinitions::default())
    }

    /// 降级实例：文字元素不绘制，几何元素（框选 / 血条）仍可呈现。
    pub fn degraded() -> Self {
        Self {
            text_enabled: false,
        }
    }
}

/// 单个单位的只读 HUD 展示数据。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudUnit {
    pub id: u64,
    pub hp: i64,
    pub max_hp: i64,
    pub owner: u8,
    pub alive: bool,
    pub selected: bool,
    /// 屏幕像素坐标（表现层投影，不进权威状态）。
    pub screen: [f32; 2],
    /// 单位占位标记的屏幕包围盒，选中框按它绘制以贴合 3D 实体。
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
    /// 世界坐标（表现层，供小地图派生命中，不进权威状态）。
    pub world: [f32; 2],
}

/// 生产队列条目的只读展示数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudQueueItem {
    pub unit_def: u32,
    pub remaining_frames: u32,
}

/// 命令卡动作（由选中集合与实体类型只读派生）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAction {
    Move,
    Attack,
    Stop,
    Gather,
    Train,
    Build,
    Cancel,
}

impl CommandAction {
    /// 命令卡 ASCII 标签（走 i18n 边界，不进日志 / 权威状态 / 哈希）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Attack => "Attack",
            Self::Stop => "Stop",
            Self::Gather => "Gather",
            Self::Train => "Train",
            Self::Build => "Build",
            Self::Cancel => "Cancel",
        }
    }
}

/// 主选择面板（只读派生）。
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionPanel {
    pub id: u64,
    pub label: &'static str,
    pub hp: i64,
    pub max_hp: i64,
    pub owner: u8,
}

/// 小地图上的只读实体点（归一化到 `[0, 1]`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapPoint {
    pub nx: f32,
    pub ny: f32,
    pub owner: u8,
    pub selected: bool,
}

/// 小地图只读模型。
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapModel {
    pub points: Vec<MinimapPoint>,
    /// 世界跨度，供把点击换算回相机平移（表现层参数）。
    pub extent: f32,
}

/// 一帧 HUD 的全部只读展示数据；纯派生、owned，不持有权威状态句柄。
#[derive(Clone, Debug, PartialEq)]
pub struct HudModel {
    pub tick: u64,
    pub units: Vec<HudUnit>,
    pub gold: i64,
    pub lumber: i64,
    pub supply_used: i64,
    pub supply_cap: i64,
    pub queue: Vec<HudQueueItem>,
    pub selected_count: usize,
    pub event_count: usize,
    /// 本地玩家编号，用于按队伍区分 HUD 标记颜色。
    pub local_player: u8,
    /// 装配层注入的表现层类型标签（不进权威状态 / 哈希 / 快照）。
    pub kinds: BTreeMap<u64, EntityKind>,
    /// 命令卡动作（只读派生）。
    pub command_actions: Vec<CommandAction>,
    /// 主选择面板（只读派生）。
    pub selection_panel: Option<SelectionPanel>,
    /// 小地图（只读派生）。
    pub minimap: MinimapModel,
    /// 胜负结果（只读派生，来自 `VictoryState`）。
    pub winner: Option<u8>,
}

/// 把世界坐标归一化到小地图 `[0, 1]` 区间。
fn normalize_world(raw: i64, extent: f32) -> f32 {
    let world = raw as f64 / SCALE as f64;
    (world / extent.max(1.0) as f64 + 0.5).clamp(0.0, 1.0) as f32
}

/// 从只读视图派生小地图数据；不写回任何权威状态。
fn derive_minimap(view: &WorldView, selection: &BTreeSet<u64>, extent: f32) -> MinimapModel {
    let points = view
        .entities
        .iter()
        .filter(|entity| entity.alive)
        .map(|entity| MinimapPoint {
            nx: normalize_world(entity.x, extent),
            ny: normalize_world(entity.y, extent),
            owner: entity.owner,
            selected: selection.contains(&entity.id),
        })
        .collect();
    MinimapModel { points, extent }
}

/// 由选中集合与类型标签派生命令卡；空选中返回空列表。
fn derive_commands(units: &[HudUnit], kinds: &BTreeMap<u64, EntityKind>) -> Vec<CommandAction> {
    let selected: Vec<&HudUnit> = units
        .iter()
        .filter(|unit| unit.selected && unit.alive)
        .collect();
    if selected.is_empty() {
        return Vec::new();
    }
    let mut actions = vec![
        CommandAction::Move,
        CommandAction::Attack,
        CommandAction::Stop,
    ];
    if selected
        .iter()
        .any(|unit| kinds.get(&unit.id) == Some(&EntityKind::Peasant))
    {
        actions.push(CommandAction::Gather);
        actions.push(CommandAction::Build);
    }
    if selected
        .iter()
        .any(|unit| kinds.get(&unit.id).is_some_and(|kind| kind.is_producer()))
    {
        actions.push(CommandAction::Train);
        actions.push(CommandAction::Cancel);
    }
    actions
}

/// 由选中集合派生主选择面板。
fn derive_selection_panel(
    units: &[HudUnit],
    kinds: &BTreeMap<u64, EntityKind>,
) -> Option<SelectionPanel> {
    let unit = units.iter().find(|unit| unit.selected && unit.alive)?;
    Some(SelectionPanel {
        id: unit.id,
        label: kinds.get(&unit.id).map_or("Unit", |kind| kind.label()),
        hp: unit.hp,
        max_hp: unit.max_hp,
        owner: unit.owner,
    })
}

impl HudModel {
    /// 从只读视图、事实事件拷贝与只读快照派生 HUD 数据。
    ///
    /// `WorldView` 提供单位生命 / 位置 / 归属；只读 `Snapshot` 提供资源与
    /// 生产队列（这些字段不在 `WorldView` 内）。两者都是只读 API，本函数
    /// MUST NOT 写回任何权威状态。`ScreenMapper` 的投影输出是物理像素，
    /// HUD 绘制在 egui 逻辑空间，故 MUST 经 `to_logical` 换算。
    pub fn collect(
        view: &WorldView,
        snapshot: &Snapshot,
        selection: &BTreeSet<u64>,
        events: &[Event],
        mapper: &ScreenMapper,
        local_player: u8,
    ) -> Self {
        let units: Vec<HudUnit> = view
            .entities
            .iter()
            .map(|entity| {
                let max_hp = snapshot
                    .positions
                    .iter()
                    .find(|position| position.entity == entity.id)
                    .map_or(entity.hp.max(1), |position| position.max_hp.max(1));
                let world_f64 = [
                    entity.x as f64 / SCALE as f64,
                    entity.y as f64 / SCALE as f64,
                ];
                let screen = mapper.to_logical(mapper.world_to_screen(world_f64));
                let (bounds_min, bounds_max) =
                    mapper.to_logical_bounds(mapper.unit_screen_bounds(world_f64));
                HudUnit {
                    id: entity.id,
                    hp: entity.hp,
                    max_hp,
                    owner: entity.owner,
                    alive: entity.alive,
                    selected: selection.contains(&entity.id),
                    screen: [screen[0] as f32, screen[1] as f32],
                    bounds_min: [bounds_min[0] as f32, bounds_min[1] as f32],
                    bounds_max: [bounds_max[0] as f32, bounds_max[1] as f32],
                    world: [world_f64[0] as f32, world_f64[1] as f32],
                }
            })
            .collect();

        let economy = snapshot.gameplay.players.get(&local_player);
        let queue = snapshot
            .positions
            .iter()
            .filter(|position| selection.contains(&position.entity))
            .flat_map(|position| &position.gameplay.production)
            .map(|item| HudQueueItem {
                unit_def: item.unit_def,
                remaining_frames: item.remaining_frames,
            })
            .collect();
        let extent = mapper.world_span as f32;
        let kinds = BTreeMap::new();

        Self {
            tick: view.tick,
            units,
            gold: economy.map_or(0, |e| e.gold),
            lumber: economy.map_or(0, |e| e.lumber),
            supply_used: economy.map_or(0, |e| e.supply_used),
            supply_cap: economy.map_or(0, |e| e.supply_cap),
            queue,
            selected_count: selection.len(),
            event_count: events.len(),
            local_player,
            command_actions: Vec::new(),
            selection_panel: None,
            minimap: derive_minimap(view, selection, extent),
            winner: snapshot.gameplay.victory.winner,
            kinds,
        }
    }

    /// 注入装配层的实体类型标签并重新派生命令卡与选择面板。
    /// 类型标签是表现层数据，MUST NOT 进入权威状态、状态哈希或快照。
    pub fn with_kinds(mut self, kinds: BTreeMap<u64, EntityKind>) -> Self {
        self.command_actions = derive_commands(&self.units, &kinds);
        self.selection_panel = derive_selection_panel(&self.units, &kinds);
        self.kinds = kinds;
        self
    }
}

/// 把线性 RGBA 转成 egui 颜色。
fn to_color(color: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (color[3].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// 一帧 HUD 的交互结果（纯表现层，MUST NOT 生成权威命令或进入哈希）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HudInteraction {
    /// 小地图点击的归一化坐标（仅平移相机）。
    pub minimap_click: Option<[f32; 2]>,
    /// 命令卡被点击的动作。
    pub command_action: Option<CommandAction>,
}

/// 绘制一帧 HUD：只读 `HudModel` + i18n 文案，MUST NOT 触碰权威状态。
///
/// 保留既有签名；小地图点击与命令卡动作由 [`draw_with_minimap`] 返回，
/// 供需要处理相机平移 / 输入翻译的调用方使用。
pub fn draw(ctx: &egui::Context, model: &HudModel, texts: &Texts, assets: HudAssets) {
    let _ = draw_with_minimap(ctx, model, texts, assets);
}

/// 绘制一帧 HUD 并返回交互结果（小地图点击 / 命令卡动作）。
pub fn draw_with_minimap(
    ctx: &egui::Context,
    model: &HudModel,
    texts: &Texts,
    assets: HudAssets,
) -> HudInteraction {
    // 几何层：所有存活单位都绘制队伍色标记、选中环与血条。
    egui::Area::new(egui::Id::unique("w3_client_hud_overlay")).show(ctx, |ui| {
        let painter = ui.painter();
        for unit in model.units.iter().filter(|unit| unit.alive) {
            let hints = display_hints(unit.owner, unit.hp, unit.max_hp, unit.selected);
            let frame = egui::Rect::from_min_max(
                egui::pos2(unit.bounds_min[0], unit.bounds_min[1]),
                egui::pos2(unit.bounds_max[0], unit.bounds_max[1]),
            );
            let team_color = to_color(hints.team_color);
            let stroke = match hints.selection_ring {
                Some(ring) => egui::Stroke::new(3.0, to_color(ring)),
                None => egui::Stroke::new(1.0, team_color),
            };
            painter.rect_stroke(
                frame,
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
            let bar = egui::Rect::from_min_size(
                egui::pos2(frame.min.x, frame.min.y - 8.0),
                egui::vec2(frame.width(), 4.0),
            );
            painter.rect_filled(bar, egui::CornerRadius::ZERO, egui::Color32::DARK_RED);
            let filled = egui::Rect::from_min_size(
                bar.min,
                egui::vec2(bar.width() * hints.hp_fraction, 4.0),
            );
            painter.rect_filled(filled, egui::CornerRadius::ZERO, team_color);
        }
    });

    if !assets.text_enabled {
        return HudInteraction::default();
    }

    let mut interaction = HudInteraction::default();
    egui::Window::new(texts.hud_title).show(ctx, |ui| {
        ui.label(format!("{}: {}", texts.tick, model.tick));
        if let Some(winner) = model.winner {
            ui.heading(format!("{}: {} {}", texts.victory, texts.winner, winner));
        }
        ui.separator();

        ui.label(texts.resources);
        ui.label(format!("{}: {}", texts.gold, model.gold));
        ui.label(format!("{}: {}", texts.lumber, model.lumber));
        ui.label(format!(
            "{}: {}/{}",
            texts.supply, model.supply_used, model.supply_cap
        ));
        ui.separator();

        // 选择面板。
        ui.label(texts.selection_panel);
        if let Some(panel) = &model.selection_panel {
            ui.label(format!("{} #{}", panel.label, panel.id));
            ui.label(format!("{}: {}/{}", texts.hp, panel.hp, panel.max_hp));
        } else {
            ui.label(texts.no_selection);
        }
        ui.separator();

        // 命令卡：动作按选中集合与实体类型只读派生。
        ui.label(texts.commands);
        if model.command_actions.is_empty() {
            ui.label(texts.no_selection);
        } else {
            ui.horizontal_wrapped(|ui| {
                for action in &model.command_actions {
                    if ui.button(action.label()).clicked() {
                        interaction.command_action = Some(*action);
                    }
                }
            });
        }
        ui.separator();

        // 生产队列：剩余帧数随逻辑帧递减，完成 / 取消后条目立即刷新。
        ui.label(texts.production);
        if model.queue.is_empty() {
            ui.label(texts.empty_queue);
        } else {
            for item in &model.queue {
                ui.label(format!(
                    "{} #{}: {} {}",
                    texts.production, item.unit_def, item.remaining_frames, texts.tick
                ));
            }
        }
        ui.separator();

        // 小地图：只读实体点 + 相机视口框；点击只返回归一化坐标。
        ui.label(texts.minimap);
        let size = egui::vec2(160.0, 160.0);
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
        let rect = response.rect;
        painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(24));
        for point in &model.minimap.points {
            let center = egui::pos2(
                rect.min.x + point.nx * rect.width(),
                rect.min.y + point.ny * rect.height(),
            );
            // 与几何层统一使用 `w3-render` 的 12 色队伍色表。
            let color = to_color(team_color(point.owner));
            let radius = if point.selected { 3.5 } else { 2.0 };
            painter.circle_filled(center, radius, color);
        }
        painter.rect_stroke(
            rect.shrink(24.0),
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
            egui::StrokeKind::Outside,
        );
        if response.clicked() {
            let position = response.interact_pointer_pos().unwrap_or(rect.center());
            interaction.minimap_click = Some([
                ((position.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
                ((position.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
            ]);
        }
        ui.separator();
        ui.label(texts.action_hint);
    });
    interaction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hud_unit(id: u64, selected: bool, owner: u8) -> HudUnit {
        HudUnit {
            id,
            hp: 60,
            max_hp: 60,
            owner,
            alive: true,
            selected,
            screen: [0.0, 0.0],
            bounds_min: [0.0, 0.0],
            bounds_max: [10.0, 10.0],
            world: [0.0, 0.0],
        }
    }

    #[test]
    fn command_card_depends_on_selected_kinds() {
        let units = vec![hud_unit(1, true, 1)];

        let mut peasant = BTreeMap::new();
        peasant.insert(1u64, EntityKind::Peasant);
        let actions = derive_commands(&units, &peasant);
        assert!(actions.contains(&CommandAction::Gather));
        assert!(actions.contains(&CommandAction::Build));
        assert!(!actions.contains(&CommandAction::Train));

        let mut producer = BTreeMap::new();
        producer.insert(1u64, EntityKind::TownHall);
        let actions = derive_commands(&units, &producer);
        assert!(actions.contains(&CommandAction::Train));
        assert!(actions.contains(&CommandAction::Cancel));
        assert!(!actions.contains(&CommandAction::Gather));
    }

    #[test]
    fn empty_selection_has_no_command_card_or_panel() {
        let units = vec![hud_unit(1, false, 1)];
        let kinds = BTreeMap::new();
        assert!(derive_commands(&units, &kinds).is_empty());
        assert!(derive_selection_panel(&units, &kinds).is_none());
    }

    #[test]
    fn selection_panel_uses_kind_label() {
        let units = vec![hud_unit(1, true, 1)];
        let mut kinds = BTreeMap::new();
        kinds.insert(1u64, EntityKind::Barracks);
        let panel = derive_selection_panel(&units, &kinds).expect("panel");
        assert_eq!(panel.label, "Barracks");
        assert_eq!(panel.hp, 60);
    }

    #[test]
    fn minimap_points_are_normalized_and_read_only() {
        use w3_core::EntityView;
        let view = WorldView {
            tick: 1,
            rng: 0,
            entities: vec![EntityView {
                id: 1,
                x: 0,
                y: 0,
                facing: 0,
                hp: 100,
                owner: 1,
                alive: true,
                has_target: false,
            }],
        };
        let mut selection = BTreeSet::new();
        selection.insert(1u64);
        let minimap = derive_minimap(&view, &selection, 256.0);
        assert_eq!(minimap.points.len(), 1);
        assert!((minimap.points[0].nx - 0.5).abs() < 1.0e-6);
        assert!(minimap.points[0].selected);
        assert_eq!(minimap.extent, 256.0);
    }
}
