//! 只读 HUD（tasks 6.3–6.7、design D4）。
//!
//! HUD 只消费 [`HudModel`]——由只读 `WorldView`、事实事件拷贝与只读快照
//! 派生的**纯表现层数据**。本模块不持有 `RtsMatch`、不引用任何权威状态
//! 句柄，因此结构上无法反写模拟（ADR 0002 §5）。
//!
//! - 选中框 / 血条 / 资源 / 生产队列只从 `HudModel` 读取；
//! - UI 文案来自 [`crate::i18n`]，MUST NOT 进入日志、权威状态或状态哈希；
//! - 字体使用 egui 内置开源默认字体（`default_fonts`），不加载任何资产
//!   文件（ADR 0001）；内置字体无 CJK 字形，故文案为 ASCII；
//! - 字体 / 资源不可用时以英文结构化 `key=value` 日志报告并降级为不绘制
//!   文字元素，MUST NOT 改变模拟推进。

use std::collections::BTreeSet;

use w3_core::{Event, SCALE, Snapshot, WorldView};

use crate::i18n::Texts;
use crate::input::ScreenMapper;

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
}

/// 生产队列条目的只读展示数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudQueueItem {
    pub unit_def: u32,
    pub remaining_frames: u32,
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
}

impl HudModel {
    /// 从只读视图、事实事件拷贝与只读快照派生 HUD 数据。
    ///
    /// `WorldView` 提供单位生命 / 位置 / 归属；只读 `Snapshot` 提供资源与
    /// 生产队列（这些字段不在 `WorldView` 内，见交付回报）。两者都是只读
    /// API，本函数 MUST NOT 写回任何权威状态。`ScreenMapper` 的投影输出是
    /// 物理像素，HUD 绘制在 egui 逻辑空间，故 MUST 经 `to_logical` 换算。
    pub fn collect(
        view: &WorldView,
        snapshot: &Snapshot,
        selection: &BTreeSet<u64>,
        events: &[Event],
        mapper: &ScreenMapper,
        local_player: u8,
    ) -> Self {
        let units = view
            .entities
            .iter()
            .map(|entity| {
                let max_hp = snapshot
                    .positions
                    .iter()
                    .find(|position| position.entity == entity.id)
                    .map_or(entity.hp.max(1), |position| position.max_hp.max(1));
                let world = [
                    entity.x as f64 / SCALE as f64,
                    entity.y as f64 / SCALE as f64,
                ];
                let screen = mapper.to_logical(mapper.world_to_screen(world));
                let (bounds_min, bounds_max) =
                    mapper.to_logical_bounds(mapper.unit_screen_bounds(world));
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
        }
    }
}

/// 绘制一帧 HUD：只读 `HudModel` + i18n 文案，MUST NOT 触碰权威状态。
///
/// 所有存活单位都绘制队伍色标记与血条（拾取对象 MUST 可见、可分辨敌我），
/// 选中单位额外绘制绿色选中框；`assets.text_enabled == false` 时只保留
/// 几何元素，不绘制任何文字标签（资源缺失降级路径）。
pub fn draw(ctx: &egui::Context, model: &HudModel, texts: &Texts, assets: HudAssets) {
    egui::Area::new(egui::Id::unique("w3_client_hud_overlay")).show(ctx, |ui| {
        let painter = ui.painter();
        for unit in model.units.iter().filter(|unit| unit.alive) {
            let frame = egui::Rect::from_min_max(
                egui::pos2(unit.bounds_min[0], unit.bounds_min[1]),
                egui::pos2(unit.bounds_max[0], unit.bounds_max[1]),
            );
            let team_color = if unit.owner == model.local_player {
                egui::Color32::from_rgb(90, 170, 255)
            } else {
                egui::Color32::from_rgb(255, 90, 90)
            };
            let stroke = if unit.selected {
                egui::Stroke::new(3.0, egui::Color32::LIGHT_GREEN)
            } else {
                egui::Stroke::new(1.0, team_color)
            };
            painter.rect_stroke(
                frame,
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
            let fraction = (unit.hp as f32 / unit.max_hp as f32).clamp(0.0, 1.0);
            let bar = egui::Rect::from_min_size(
                egui::pos2(frame.min.x, frame.min.y - 8.0),
                egui::vec2(frame.width(), 4.0),
            );
            painter.rect_filled(bar, egui::CornerRadius::ZERO, egui::Color32::DARK_RED);
            let filled =
                egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * fraction, 4.0));
            painter.rect_filled(filled, egui::CornerRadius::ZERO, team_color);
        }
    });

    if !assets.text_enabled {
        return;
    }

    egui::Window::new(texts.hud_title).show(ctx, |ui| {
        ui.label(format!("{}: {}", texts.tick, model.tick));
        ui.separator();
        ui.label(texts.resources);
        ui.label(format!("{}: {}", texts.gold, model.gold));
        ui.label(format!("{}: {}", texts.lumber, model.lumber));
        ui.label(format!(
            "{}: {}/{}",
            texts.supply, model.supply_used, model.supply_cap
        ));
        ui.separator();
        ui.label(format!(
            "{}: {}",
            texts.selection,
            if model.selected_count == 0 {
                texts.no_selection
            } else {
                "" // 具体单位列表见下方血条 / 几何元素
            }
        ));
        if model.selected_count > 0 {
            ui.label(format!("{}", model.selected_count));
        }
        ui.separator();
        ui.label(texts.production);
        if model.queue.is_empty() {
            ui.label(texts.empty_queue);
        } else {
            for item in &model.queue {
                ui.add(egui::ProgressBar::new(0.0).text(format!(
                    "{} #{} ({} {})",
                    texts.production, item.unit_def, item.remaining_frames, texts.tick
                )));
            }
        }
        ui.separator();
        ui.label(texts.action_hint);
    });
}
