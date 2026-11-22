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

use w3_content::RgbaImage;
use w3_core::{
    BAG_SLOTS, DefId, Event, EventKind, LeashPhase, NEUTRAL_OWNER, SCALE, Snapshot, TownHallTier,
    WorldView,
};
use w3_render::{display_hints, team_color};

use crate::faction::FactionSelection;
use crate::fog::{FogCounts, FogGrid, FogNeutralCategory, FogNeutralMarker, overlay_opacity};
use crate::i18n::Texts;
use crate::input::ScreenMapper;
use crate::melee::EntityKind;
use crate::melee::Race;
use crate::melee::command_card::{
    CardCommand, CardId, CardKind, CommandCard, CommandCatalog, GRID_HEIGHT, GRID_WIDTH, slot,
};

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

/// 单个祭坛复活队列项的只读 HUD 投影（P8.5a / tasks 6.1）。
///
/// 全部字段来自只读 `Snapshot.revive.queue` 的值拷贝，owned、纯表现层，
/// MUST NOT 反写 `RtsMatch` 权威状态、进状态哈希 / 快照 / `first_difference()`。
/// 进度 / 费用 / 剩余帧均由冻结权威状态只读派生，不引入墙钟、全局静态字典
/// 或建筑名白名单（ADR 0002 §5 / D7 / R10.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudReviveItem {
    pub hero: u64,
    pub altar: u64,
    pub altar_def: DefId,
    pub owner: u8,
    pub remaining_frames: u32,
    pub total_frames: u32,
    pub gold: i64,
    pub lumber: i64,
}

/// 祭坛复活 HUD 的只读投影模型（一帧）。
///
/// 只从只读 `Snapshot.revive.{queue,deaths,charges}` 与 `drain_events()` 的
/// `HeroDied` / `Revived` 事实事件拷贝派生；本结构不持有 `RtsMatch`，结构上
/// 无法反写模拟状态，也不进入 FNV-1a 状态哈希 / 快照 / `first_difference()`。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReviveHudModel {
    /// 复活队列项（按 `BTreeMap` 稳定 `EntityId` 顺序）。
    pub queue: Vec<HudReviveItem>,
    /// 死亡登记条数（只读派生，仅用于 HUD 展示）。
    pub deaths: usize,
    /// 冻结扣费登记条数（只读派生，仅用于 HUD 展示）。
    pub charges: usize,
    /// 本批次事实事件中的 `HeroDied` 计数。
    pub hero_died_events: usize,
    /// 本批次事实事件中的 `Revived` 计数。
    pub revived_events: usize,
}

impl ReviveHudModel {
    /// 从只读快照拷贝与事实事件拷贝派生 HUD 数据；纯只读、无副作用。
    ///
    /// `snapshot` 是 [`w3_core::Snapshot`] 的所有权副本（由 `StepDriver::snapshot`
    /// 提供），`events` 是 `drain_events()` 的事实事件拷贝。本函数只读取两者，
    /// 不调用 `step()`、不提交命令、不改写任何权威状态字段。
    pub fn derive(snapshot: &Snapshot, events: &[Event]) -> Self {
        let queue = snapshot
            .revive
            .queue
            .values()
            .map(|item| HudReviveItem {
                hero: item.hero.value(),
                altar: item.altar.value(),
                altar_def: item.altar_def,
                owner: item.owner,
                remaining_frames: item.remaining_frames,
                total_frames: item.total_frames,
                gold: item.gold,
                lumber: item.lumber,
            })
            .collect();
        let hero_died_events = events
            .iter()
            .filter(|event| event.kind == EventKind::HeroDied)
            .count();
        let revived_events = events
            .iter()
            .filter(|event| event.kind == EventKind::Revived)
            .count();
        Self {
            queue,
            deaths: snapshot.revive.deaths.len(),
            charges: snapshot.revive.charges.len(),
            hero_died_events,
            revived_events,
        }
    }
}

/// 单个主城升级队列项的只读 HUD 投影（P8.5b / tasks 6.1-6.2）。
///
/// 全部字段来自只读 [`w3_core::Snapshot::town_hall`] 的 `queue` 值拷贝，
/// owned、纯表现层，MUST NOT 反写 `RtsMatch` 权威状态、进状态哈希 / 快照 /
/// `first_difference()`。当前 tier、升级进度 / 剩余帧 / 费用均由冻结权威状态
/// 只读派生，不引入墙钟、全局静态字典或建筑名白名单（ADR 0002 §5 / D6 / D7）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudUpgradeItem {
    pub town_hall: u64,
    pub town_hall_def: DefId,
    pub owner: u8,
    pub from_tier: TownHallTier,
    pub target_tier: TownHallTier,
    pub remaining_frames: u32,
    pub total_frames: u32,
    pub gold: i64,
    pub lumber: i64,
}

/// 主城升级 HUD 的只读投影模型（一帧）。
///
/// 只从只读 `Snapshot.town_hall.{tiers,queue,charges}` 与 `drain_events()`
/// 的 `TownHallUpgradeStarted` / `TownHallUpgradeCompleted` /
/// `TownHallUpgradeCancelled` 事实事件拷贝派生；本结构不持有 `RtsMatch`，结构上
/// 无法反写模拟状态，也不进入 FNV-1a 状态哈希 / 快照 / `first_difference()`
/// （ADR 0002 §5 / C6 / D6）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TownHallUpgradeHudModel {
    /// 升级队列项（按 `BTreeMap` 稳定 `EntityId` 顺序）。
    pub queue: Vec<HudUpgradeItem>,
    /// 当前各主城等级（按稳定 `EntityId` 顺序）。
    pub tiers: Vec<(u64, TownHallTier)>,
    /// 冻结扣费登记条数（只读派生，仅用于 HUD 展示）。
    pub charges: usize,
    /// 本批次 `TownHallUpgradeStarted` 事实事件计数。
    pub started_events: usize,
    /// 本批次 `TownHallUpgradeCompleted` 事实事件计数。
    pub completed_events: usize,
    /// 本批次 `TownHallUpgradeCancelled` 事实事件计数。
    pub cancelled_events: usize,
}

impl TownHallUpgradeHudModel {
    /// 从只读快照拷贝与事实事件拷贝派生 HUD 数据；纯只读、无副作用。
    ///
    /// `snapshot` 是 [`w3_core::Snapshot`] 的所有权副本（由
    /// `StepDriver::snapshot` 提供），`events` 是 `drain_events()` 的事实事件
    /// 拷贝。本函数只读取两者，不调用 `step()`、不提交命令、不改写任何权威
    /// 状态字段；队列 / 等级 / 费用均走 `BTreeMap` 稳定顺序，无墙钟 / `HashMap`
    /// 迭代顺序 / 全局静态字典 / 建筑名白名单（R13 / R13.7 / ADR 0002 §5）。
    pub fn derive(snapshot: &Snapshot, events: &[Event]) -> Self {
        let queue = snapshot
            .town_hall
            .queue
            .values()
            .map(|item| HudUpgradeItem {
                town_hall: item.town_hall.value(),
                town_hall_def: item.town_hall_def,
                owner: item.owner,
                from_tier: item.from_tier,
                target_tier: item.target_tier,
                remaining_frames: item.remaining_frames,
                total_frames: item.total_frames,
                gold: item.gold,
                lumber: item.lumber,
            })
            .collect();
        let tiers = snapshot
            .town_hall
            .tiers
            .iter()
            .map(|(&entity, &tier)| (entity.value(), tier))
            .collect();
        let started_events = events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeStarted)
            .count();
        let completed_events = events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeCompleted)
            .count();
        let cancelled_events = events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeCancelled)
            .count();
        Self {
            queue,
            tiers,
            charges: snapshot.town_hall.charges.len(),
            started_events,
            completed_events,
            cancelled_events,
        }
    }
}

/// 单个野怪的只读血条投影（P8.5c / tasks 7.1）。
///
/// 全部字段来自只读 `Snapshot.creep_camps.creeps` 与 `Snapshot.positions`
/// 的值拷贝，owned、纯表现层，MUST NOT 反写 `RtsMatch` 权威状态、进状态哈希 /
/// 快照 / `first_difference()`。距离与生命均为整数，不引入 `f64`
/// （ADR 0002 §5 / D7）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreepHudBar {
    pub entity: u64,
    pub camp: u32,
    pub hp: i64,
    pub max_hp: i64,
    pub alive: bool,
    pub phase: LeashPhase,
    pub home_x: i64,
    pub home_y: i64,
}

/// 野怪营地 HUD 的只读投影模型（一帧，P8.5c / tasks 7.1）。
///
/// 只从只读 [`w3_core::Snapshot::creep_camps`] 与 `drain_events()` 的六类
/// 事实事件拷贝派生：营地 / 野怪 / 地面物品 / 物品实例 / 击杀收益 / 随机码
/// 消费序列都只是值拷贝，本结构不持有 `RtsMatch`，结构上无法反写模拟状态，
/// 也不进入 FNV-1a 状态哈希 / 快照 / `first_difference()`。派生只走
/// `BTreeMap` / `BTreeSet` 稳定顺序，不引入墙钟、全局静态字典、建筑名
/// 白名单或 `f64`（ADR 0002 §5 / D7）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreepCampHudModel {
    /// 营地登记条数。
    pub camp_count: usize,
    /// 已全清营地条数。
    pub cleared_camps: usize,
    /// 野怪登记总数。
    pub total_creeps: usize,
    /// 只读视图中仍存活的野怪条数。
    pub alive_creeps: usize,
    /// 野怪血条（按稳定 `EntityId` 顺序）。
    pub creep_bars: BTreeMap<u64, CreepHudBar>,
    /// 地面掉落标记（按稳定 `EntityId` 顺序）。
    pub drop_markers: BTreeSet<u64>,
    /// 物品实例条数。
    pub item_instances: usize,
    /// 击杀收益登记条数。
    pub reward_count: usize,
    /// 经验入账累计（只读派生，仅用于 HUD 展示）。
    pub reward_xp: i64,
    /// 金币入账累计（只读派生，仅用于 HUD 展示）。
    pub reward_gold: i64,
    /// 确定性随机码消费登记条数。
    pub rng_draws: usize,
    /// 本批次 `CreepCampSeeded` 事实事件计数。
    pub seeded_events: usize,
    /// 本批次 `CreepEngaged` 事实事件计数。
    pub engaged_events: usize,
    /// 本批次 `CreepLeashReturning` 事实事件计数。
    pub leash_returning_events: usize,
    /// 本批次 `CreepLeashReturned` 事实事件计数。
    pub leash_returned_events: usize,
    /// 本批次 `CreepDied` 事实事件计数。
    pub creep_died_events: usize,
    /// 本批次 `ItemDropped` 事实事件计数。
    pub item_dropped_events: usize,
}

impl CreepCampHudModel {
    /// 从只读快照拷贝与事实事件拷贝派生 HUD 数据；纯只读、无副作用。
    ///
    /// `snapshot` 是 [`w3_core::Snapshot`] 的所有权副本（由
    /// `StepDriver::snapshot` 提供），`events` 是 `drain_events()` 的事实事件
    /// 拷贝。本函数只读取两者，不调用 `step()`、不提交命令、不改写任何权威
    /// 状态字段；所有累计 / 计数均为整数，MUST NOT 引入墙钟或浮点
    /// （ADR 0002 §5 / D7）。
    pub fn derive(snapshot: &Snapshot, events: &[Event]) -> Self {
        let creep_bars: BTreeMap<u64, CreepHudBar> = snapshot
            .creep_camps
            .creeps
            .iter()
            .map(|(&entity, creep)| {
                let (hp, max_hp, alive) = snapshot
                    .positions
                    .iter()
                    .find(|record| record.entity == entity.value())
                    .map_or((0, 1, false), |record| {
                        (record.hp, record.max_hp.max(1), record.alive)
                    });
                (
                    entity.value(),
                    CreepHudBar {
                        entity: entity.value(),
                        camp: creep.camp,
                        hp,
                        max_hp,
                        alive,
                        phase: creep.phase,
                        home_x: creep.home_x,
                        home_y: creep.home_y,
                    },
                )
            })
            .collect();
        let drop_markers: BTreeSet<u64> = snapshot
            .creep_camps
            .ground_items
            .keys()
            .map(|entity| entity.value())
            .collect();
        let reward_xp = snapshot
            .creep_camps
            .kill_rewards
            .values()
            .flat_map(|reward| reward.xp_credited.values())
            .copied()
            .sum::<i64>();
        let reward_gold = snapshot
            .creep_camps
            .kill_rewards
            .values()
            .map(|reward| reward.gold)
            .sum::<i64>();
        let mut model = Self {
            camp_count: snapshot.creep_camps.camps.len(),
            cleared_camps: snapshot
                .creep_camps
                .camps
                .values()
                .filter(|camp| camp.cleared)
                .count(),
            total_creeps: creep_bars.len(),
            alive_creeps: creep_bars.values().filter(|bar| bar.alive).count(),
            creep_bars,
            drop_markers,
            item_instances: snapshot.creep_camps.item_instances.len(),
            reward_count: snapshot.creep_camps.kill_rewards.len(),
            reward_xp,
            reward_gold,
            rng_draws: snapshot.creep_camps.rng_draws.len(),
            seeded_events: 0,
            engaged_events: 0,
            leash_returning_events: 0,
            leash_returned_events: 0,
            creep_died_events: 0,
            item_dropped_events: 0,
        };
        for event in events {
            match event.kind {
                EventKind::CreepCampSeeded => model.seeded_events += 1,
                EventKind::CreepEngaged => model.engaged_events += 1,
                EventKind::CreepLeashReturning => model.leash_returning_events += 1,
                EventKind::CreepLeashReturned => model.leash_returned_events += 1,
                EventKind::CreepDied => model.creep_died_events += 1,
                EventKind::ItemDropped => model.item_dropped_events += 1,
                _ => {}
            }
        }
        model
    }
}

/// 单个英雄六格实例背包的只读 HUD 投影（P8.5d / tasks 6.2）。
///
/// 槽位保存物品实例 `EntityId` 的值拷贝，不保存 `RtsMatch` 句柄，也不解析
/// 定义层名称；同名道具多实例仍各有独立编号，纯表现层派生。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudItemBag {
    /// 持包英雄的实体编号（只读派生）。
    pub hero: u64,
    /// 定长六格；`None` 表示空槽，`Some` 是物品实例编号值拷贝。
    pub slots: [Option<u64>; BAG_SLOTS],
}

/// 单个回城施法条的只读 HUD 投影（P8.5d / tasks 6.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudTownPortalCast {
    /// 施法英雄实体编号。
    pub caster: u64,
    /// 友方主城目标实体编号。
    pub target: u64,
    /// 剩余施法整数帧。
    pub remaining_frames: u32,
    /// 施法总整数帧。
    pub total_frames: u32,
}

/// 单条物品 / 商店 / 回城事实的只读 HUD 投影（P8.5d / tasks 6.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudItemShopFact {
    /// 稳定事实序号。
    pub seq: u64,
    /// 事实登记整数帧。
    pub frame: u64,
    /// 事实种类稳定编码，来自 `ItemShopFactKind::code()`。
    pub kind_code: u8,
    /// 持包英雄 / 施法者实体编号。
    pub hero: u64,
    /// 物品实例实体编号。
    pub instance: u64,
    /// 参与槽位；`u8::MAX` 表示无槽位。
    pub slot: u8,
    /// 关联实体编号。
    pub entity: u64,
    /// 精确黄金变动。
    pub gold_delta: i64,
    /// 货架 / 商店 / 道具编号。
    pub shelf: u32,
}

/// 物品 / 商店 / 回城 HUD 的只读投影模型（一帧，P8.5d / tasks 6.2）。
///
/// 只从只读 [`w3_core::Snapshot::item_shop`] 的 `bags` / `stock` / `casts` /
/// `facts` 与 `drain_events()` 的十一类事实事件拷贝派生：六格背包、货架库存、
/// 回城施法进度、购买 / 拾取 / 使用 / 拒单事实都只是整数值拷贝。本结构不持有
/// `RtsMatch`，结构上无法反写模拟状态，也不进入 FNV-1a 状态哈希、v7 快照或
/// `first_difference()`（ADR 0002 §5 / R10.5 / R13.5）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemShopHudModel {
    /// 六格背包投影（按稳定 `EntityId` 顺序）。
    pub bags: Vec<HudItemBag>,
    /// 货架库存投影（按稳定货架 id 顺序）。
    pub stock: BTreeMap<u32, u32>,
    /// 回城施法条（按稳定施法者 `EntityId` 顺序）。
    pub casts: Vec<HudTownPortalCast>,
    /// 权威事实投影（按稳定 `seq` 顺序）。
    pub facts: Vec<HudItemShopFact>,
    /// 本批次事实事件总数。
    pub event_count: usize,
    /// 本批次 `ItemPickedUp` 计数。
    pub picked_up_events: usize,
    /// 本批次 `ItemPurchased` 计数。
    pub purchased_events: usize,
    /// 本批次 `ItemUsed` 计数。
    pub used_events: usize,
    /// 本批次 `ItemDiscarded` 计数。
    pub discarded_events: usize,
    /// 本批次 `ItemTransferred` 计数。
    pub transferred_events: usize,
    /// 本批次 `ItemSold` 计数。
    pub sold_events: usize,
    /// 本批次 `TownPortalStarted` 计数。
    pub portal_started_events: usize,
    /// 本批次 `TownPortalCompleted` 计数。
    pub portal_completed_events: usize,
    /// 本批次 `TownPortalCancelled` 计数。
    pub portal_cancelled_events: usize,
    /// 本批次 `MarketRefreshed` 计数。
    pub market_refreshed_events: usize,
    /// 本批次 `ItemShopCommandRejected` 计数。
    pub rejected_events: usize,
}

impl ItemShopHudModel {
    /// 从只读快照拷贝与事实事件拷贝派生 HUD 数据；纯只读、无副作用。
    ///
    /// `snapshot` 是 [`w3_core::Snapshot`] 的所有权副本（由 `StepDriver::snapshot`
    /// 提供），`events` 是 `drain_events()` 的事实事件拷贝。本函数只读取两者，
    /// 不调用 `step()`、不提交命令、不改写任何权威状态字段；所有集合都按
    /// `BTreeMap` 稳定键序遍历，不引入墙钟、浮点、全局静态字典或 `HashMap`
    /// 迭代顺序（ADR 0002 §5 / D7）。
    pub fn derive(snapshot: &Snapshot, events: &[Event]) -> Self {
        let bags = snapshot
            .item_shop
            .bags
            .iter()
            .map(|(&hero, bag)| HudItemBag {
                hero: hero.value(),
                slots: bag.slots.map(|slot| slot.map(|instance| instance.value())),
            })
            .collect();
        let casts = snapshot
            .item_shop
            .casts
            .values()
            .map(|cast| HudTownPortalCast {
                caster: cast.caster.value(),
                target: cast.target.value(),
                remaining_frames: cast.remaining_frames,
                total_frames: cast.total_frames,
            })
            .collect();
        let facts = snapshot
            .item_shop
            .facts
            .values()
            .map(|fact| HudItemShopFact {
                seq: fact.seq,
                frame: fact.frame,
                kind_code: fact.kind.code(),
                hero: fact.hero.value(),
                instance: fact.instance.value(),
                slot: fact.slot,
                entity: fact.entity.value(),
                gold_delta: fact.gold_delta,
                shelf: fact.shelf,
            })
            .collect();
        let mut model = Self {
            bags,
            stock: snapshot.item_shop.stock.clone(),
            casts,
            facts,
            event_count: events.len(),
            picked_up_events: 0,
            purchased_events: 0,
            used_events: 0,
            discarded_events: 0,
            transferred_events: 0,
            sold_events: 0,
            portal_started_events: 0,
            portal_completed_events: 0,
            portal_cancelled_events: 0,
            market_refreshed_events: 0,
            rejected_events: 0,
        };
        // 十一类新增事实事件只参与表现层计数，重复投递由事件消费者的
        // sequence 去重吸收，MUST NOT 改写权威状态或进入状态哈希。
        for event in events {
            match event.kind {
                EventKind::ItemPickedUp => model.picked_up_events += 1,
                EventKind::ItemPurchased => model.purchased_events += 1,
                EventKind::ItemUsed => model.used_events += 1,
                EventKind::ItemDiscarded => model.discarded_events += 1,
                EventKind::ItemTransferred => model.transferred_events += 1,
                EventKind::ItemSold => model.sold_events += 1,
                EventKind::TownPortalStarted => model.portal_started_events += 1,
                EventKind::TownPortalCompleted => model.portal_completed_events += 1,
                EventKind::TownPortalCancelled => model.portal_cancelled_events += 1,
                EventKind::MarketRefreshed => model.market_refreshed_events += 1,
                EventKind::ItemShopCommandRejected => model.rejected_events += 1,
                _ => {}
            }
        }
        model
    }
}

/// 战争迷雾 HUD 的只读投影模型（P8.5e / R10.1 / R10.5 / tasks 2.1）。
///
/// 与 `ReviveHudModel` / `TownHallUpgradeHudModel` / `CreepCampHudModel` /
/// `ItemShopHudModel` 同构：只消费只读 `Snapshot`、事实事件拷贝与表现层本地
/// [`FogGrid`] 的 owned 值拷贝，不持有 `RtsMatch`，结构上无法反写模拟状态。
/// 本结构 MUST NOT 进 FNV-1a 状态哈希、v7 快照或 `first_difference()`。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FogHudModel {
    /// 最近一次只读快照帧号。
    pub snapshot_tick: u64,
    /// 最近一次迷雾重算帧号；`None` 表示该 HUD 数据未接迷雾派生。
    pub last_tick: Option<u64>,
    /// 本地视角编号（表现层专属，不进权威状态）。
    pub local_player: u8,
    /// 昼夜相位稳定编码（0 昼 / 1 夜；R10.2）。
    pub phase_code: u8,
    /// 三态格计数；`explored` 单调不减。
    pub counts: FogCounts,
    /// 本批次事实事件拷贝条数（只读消费）。
    pub event_count: usize,
    /// 当前 VISIBLE 格内的野怪 / 商店 / 地面物品标记（只读 Snapshot 查表）。
    pub visible_markers: Vec<FogNeutralMarker>,
}

impl FogHudModel {
    /// 从只读快照、事实事件拷贝与表现层迷雾网格派生 HUD 数据；纯只读。
    ///
    /// `derive(snapshot, events)` 范式在本期多接一个表现层网格参数：网格
    /// 本身由只读 `WorldView` / `Snapshot` 派生，仍不携带权威写句柄。
    /// 全部集合走 `Vec` / `BTreeMap` 稳定顺序，无墙钟 / `HashMap` / `f64`。
    pub fn derive(snapshot: &Snapshot, events: &[Event], grid: &FogGrid) -> Self {
        Self {
            snapshot_tick: snapshot.tick,
            last_tick: grid.last_tick(),
            local_player: grid.local_player(),
            phase_code: grid.phase().code(),
            counts: grid.counts(),
            event_count: events.len(),
            visible_markers: grid.visible_markers().to_vec(),
        }
    }
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

/// 小地图只读点的来源类别（R10.1 / R8.1 / R9.1）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MinimapPointCategory {
    /// `WorldView.entities` 的在场实体。
    #[default]
    Entity,
    /// 野怪营地 home 锚点（`Snapshot.creep_camps.camps`）。
    CreepCamp,
    /// 野怪单位（`Snapshot.creep_camps.creeps`）。
    Creep,
    /// 地面掉落物品（不是 `EntityStore` 实体、不在 `read_view()`）。
    GroundItem,
    /// 中立商店（冻结定义表 + `Snapshot.item_shop` 活跃登记）。
    Shop,
}

impl MinimapPointCategory {
    /// 由迷雾中立标记类别映射成小地图点类别；稳定单值、无字符串白名单。
    pub const fn from_fog_category(category: FogNeutralCategory) -> Self {
        match category {
            FogNeutralCategory::CreepCamp => Self::CreepCamp,
            FogNeutralCategory::Creep => Self::Creep,
            FogNeutralCategory::GroundItem => Self::GroundItem,
            FogNeutralCategory::Shop => Self::Shop,
        }
    }
}

/// 小地图上的只读实体点（归一化到 `[0, 1]`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapPoint {
    pub nx: f32,
    pub ny: f32,
    pub owner: u8,
    pub selected: bool,
    /// 来源类别；实体点与 Snapshot 中立标记在喂入前分别做可见性过滤。
    pub category: MinimapPointCategory,
}

/// 小地图底图的 CPU 降采样颜色网格（纯表现层数据，无 GPU 纹理句柄）。
///
/// 把真实 `war3mapMap.png` 在每个网格单元内做 RGBA 均值，转成 `[0,1]` 的
/// 归一化颜色；HUD 用 egui 既有 `rect_filled` 逐格绘制，避免引入 GPU 纹理
/// 上传 / shader 等 CLI 侧不可验证的画面路径（与地表贴图同一策略）。
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapBackdrop {
    /// 每边网格数；`colors.len() == grid * grid`，行优先。
    pub grid: usize,
    pub colors: Vec<[f32; 4]>,
}

/// 把一张 RGBA8 图像降采样成 `grid × grid` 颜色网格；纯函数、确定性。
/// 空图像 / `grid == 0` 时返回空网格，绝不 panic。
pub fn minimap_backdrop(image: &RgbaImage, grid: usize) -> MinimapBackdrop {
    if grid == 0 || image.width == 0 || image.height == 0 || image.pixels.len() < 4 {
        return MinimapBackdrop {
            grid,
            colors: Vec::new(),
        };
    }
    let gw = image.width as usize;
    let gh = image.height as usize;
    let mut colors = Vec::with_capacity(grid * grid);
    for gy in 0..grid {
        let y0 = gy * gh / grid;
        let y1 = ((gy + 1) * gh / grid).max(y0 + 1).min(gh);
        for gx in 0..grid {
            let x0 = gx * gw / grid;
            let x1 = ((gx + 1) * gw / grid).max(x0 + 1).min(gw);
            let mut sum = [0u64; 4];
            let mut count = 0u64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let index = (y * gw + x) * 4;
                    if let Some(pixel) = image.pixels.get(index..index + 4) {
                        for channel in 0..4 {
                            sum[channel] += pixel[channel] as u64;
                        }
                        count += 1;
                    }
                }
            }
            let divisor = (count.max(1) * 255) as f32;
            colors.push([
                sum[0] as f32 / divisor,
                sum[1] as f32 / divisor,
                sum[2] as f32 / divisor,
                sum[3] as f32 / divisor,
            ]);
        }
    }
    MinimapBackdrop { grid, colors }
}

/// 小地图雾遮罩的 CPU 网格（纯表现层，无 GPU 纹理 / 文件 IO）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinimapFog {
    /// 每边网格数；`opacities.len() == grid * grid`，行优先。
    pub grid: usize,
    /// 三态格透明度（0..255）；VISIBLE 为 0、FOGGED / MASKED 取合成常量。
    pub opacities: Vec<u8>,
}

/// 小地图只读模型。
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapModel {
    pub points: Vec<MinimapPoint>,
    /// 世界跨度，供把点击换算回相机平移（表现层参数）。
    pub extent: f32,
    /// 真实小地图底图（CPU 降采样颜色网格）；`None` 时用纯色底。
    pub backdrop: Option<MinimapBackdrop>,
    /// 可选雾遮罩；底图 / 相机视口框独立于迷雾遮罩（R10.1）。
    pub fog: Option<MinimapFog>,
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
    /// 祭坛复活队列的只读 HUD 投影（P8.5a / tasks 6.1）；纯表现层派生，
    /// MUST NOT 反写权威状态、进状态哈希 / 快照 / `first_difference()`。
    pub revive: ReviveHudModel,
    /// 主城升级等级 / 队列 / 费用的只读 HUD 投影（P8.5b / tasks 6.1-6.2）；
    /// 纯表现层派生，MUST NOT 反写权威状态、进状态哈希 / 快照 /
    /// `first_difference()`（ADR 0002 §5 / D6）。
    pub town_hall: TownHallUpgradeHudModel,
    /// 野怪营地 / 掉落 / 击杀收益的只读 HUD 投影（P8.5c / tasks 7.1）；
    /// 纯表现层派生，MUST NOT 反写权威状态、进状态哈希 / 快照 /
    /// `first_difference()`（ADR 0002 §5 / D7）。
    pub creep: CreepCampHudModel,
    /// 六格背包 / 货架库存 / 回城施法 / 事实事件的只读 HUD 投影
    /// （P8.5d / tasks 6.2）；纯表现层派生，MUST NOT 反写权威状态、
    /// 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5）。
    pub item_shop: ItemShopHudModel,
    /// 三态迷雾网格 / 当前可见 / 已探索层 / 中立标记的只读 HUD 投影
    /// （P8.5e / tasks 2.1）；纯表现层派生，MUST NOT 进权威状态 / 哈希 /
    /// 快照 / `first_difference()`（R10.5 / ADR 0002 §5）。
    pub fog: FogHudModel,
    pub selected_count: usize,
    pub event_count: usize,
    /// 本地玩家编号，用于按队伍区分 HUD 标记颜色。
    pub local_player: u8,
    /// 装配层注入的表现层类型标签（不进权威状态 / 哈希 / 快照）。
    pub kinds: BTreeMap<u64, EntityKind>,
    /// 命令卡动作（只读派生，既有兼容字段）。
    pub command_actions: Vec<CommandAction>,
    /// 12 格命令卡（由定义表按选中集合类型派生，含空格位）；
    /// `None` 表示空选中、不显示命令卡（R13.1）。纯表现层只读派生，
    /// MUST NOT 进权威状态 / 状态哈希 / 快照（ADR 0002）。
    pub command_card: Option<CommandCard>,
    /// 装配层注入的 `EntityId → DefId` 只读关联，供命令卡按定义表派生
    /// 当前卡（Risks / tasks 1.4）。表现层只读，不进权威状态。
    pub entity_defs: BTreeMap<u64, DefId>,
    /// 当前卡 ID（子菜单栈顶或按选中类型定的主卡），由输入层注入，
    /// 供子菜单切换时命令卡重建（R13.3）。表现层只读。
    pub active_card: CardId,
    /// 主选择面板（只读派生）。
    pub selection_panel: Option<SelectionPanel>,
    /// 小地图（只读派生）。
    pub minimap: MinimapModel,
    /// 胜负结果（只读派生，来自 `VictoryState`）。
    pub winner: Option<u8>,
    /// 开局前选族（表现层本地只读状态，Q1）。HUD 只读取它绘制 overlay，
    /// MUST NOT 进权威状态 / 状态哈希 / 快照（ADR 0002）。
    pub faction: FactionSelection,
    /// 是否绘制开局前选族 overlay（由 [`ClientApp::with_faction_selection`]
    /// 注入；`playable.rs` 冒烟窗口不注入、保持既有 HUD 行为，Q1 / design D8）。
    pub faction_overlay: bool,
}

/// 把世界坐标归一化到小地图 `[0, 1]` 区间。
fn normalize_world(raw: i64, extent: f32) -> f32 {
    let world = raw as f64 / SCALE as f64;
    (world / extent.max(1.0) as f64 + 0.5).clamp(0.0, 1.0) as f32
}

/// 从只读视图派生小地图数据；在喂入 `points` 前完成迷雾可见性过滤。
///
/// 「在场性」先看 `EntityView.alive`，「可见性」再看本地视角迷雾网格：
/// 己方单位恒呈现，敌方 / 中立实体只在 VISIBLE 格进入 `points`；
/// MASKED / FOGGED 格敌方单位点 MUST NOT 泄露（R10.1 / R10.5）。
/// P8.5c / P8.5d 的野怪 / 营地 / 地面物品 / 商店位置来自只读 Snapshot
/// 查表后写入 `FogGrid.visible_markers()`，这里只做 owned 值拷贝，
/// 底图与相机视口框独立于雾遮罩，点击仍只平移表现层相机（R13.6）。
fn derive_minimap(
    view: &WorldView,
    selection: &BTreeSet<u64>,
    extent: f32,
    fog: Option<&FogGrid>,
    local_player: u8,
) -> MinimapModel {
    let visible_ids = fog
        .map(|grid| grid.visible_entity_ids(view))
        .unwrap_or_default();
    let mut points = Vec::new();
    for entity in &view.entities {
        if !entity.alive {
            continue;
        }
        let visible =
            fog.is_none() || entity.owner == local_player || visible_ids.contains(&entity.id);
        if !visible {
            continue;
        }
        points.push(MinimapPoint {
            nx: normalize_world(entity.x, extent),
            ny: normalize_world(entity.y, extent),
            owner: entity.owner,
            selected: selection.contains(&entity.id),
            category: MinimapPointCategory::Entity,
        });
    }
    if let Some(grid) = fog {
        for marker in grid.visible_markers() {
            points.push(MinimapPoint {
                nx: normalize_world(marker.x, extent),
                ny: normalize_world(marker.y, extent),
                owner: NEUTRAL_OWNER,
                selected: false,
                category: MinimapPointCategory::from_fog_category(marker.category),
            });
        }
    }
    let fog_overlay = fog.map(|grid| MinimapFog {
        grid: grid.side(),
        opacities: grid.cells().iter().copied().map(overlay_opacity).collect(),
    });
    MinimapModel {
        points,
        extent,
        backdrop: None,
        fog: fog_overlay,
    }
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

/// 由「选中集合类型 + 装配层 `EntityId → DefId` 关联」查定义表产出当前
/// 12 格命令卡（含空格位）。判据来自定义表，MUST NOT 用 `EntityKind`
/// match 或建筑名字符串白名单（R13.1 / R1.2 / D2 / tasks 1.2）。
/// 空选中返回 `None`（不显示命令卡）；运行时动态实体缺关联时按定义表
/// 回退通用类型，仍能按只读查表派生（Risks / tasks 1.4）。
fn derive_command_card(
    units: &[HudUnit],
    entity_defs: &BTreeMap<u64, DefId>,
    active_card: CardId,
    local_race: Race,
) -> Option<CommandCard> {
    let selected: Vec<&HudUnit> = units
        .iter()
        .filter(|unit| unit.selected && unit.alive)
        .collect();
    if selected.is_empty() {
        return None;
    }
    // 按本地族只读查表构造命令卡目录：人族 `1xx` / 兽族 `2xx`（Q2）。
    // 默认人族与既有 `standard()` 逐字节一致，既有入口行为不变。
    let catalog = CommandCatalog::for_race(local_race);
    let card_id = match active_card {
        // 子卡来自输入层注入的子菜单栈顶（建造 / 训练 / 商店）。
        CardId::Build | CardId::Train | CardId::Shop => active_card,
        // 主卡按选中集合类型重算：查定义表 `DefId → CardKind`。
        _ => {
            let kinds: Vec<CardKind> = selected
                .iter()
                .map(|unit| {
                    entity_defs
                        .get(&unit.id)
                        .map(|&def| catalog.kind_of(def))
                        .unwrap_or(CardKind::Combat)
                })
                .collect();
            catalog.primary_card(&kinds)
        }
    };
    Some(catalog.card(card_id))
}

/// 把定义表动作投射成既有 [`CommandAction`]（兼容字段，信息有损；
/// 真实命令以 `command_hotkey` 经同一 `apply` 产出，D1）。
fn compat_action(command: CardCommand) -> CommandAction {
    match command {
        CardCommand::MoveAim => CommandAction::Move,
        CardCommand::AttackAim => CommandAction::Attack,
        CardCommand::Stop => CommandAction::Stop,
        CardCommand::Gather => CommandAction::Gather,
        CardCommand::OpenBuildMenu => CommandAction::Build,
        CardCommand::OpenTrainMenu | CardCommand::TrainUnit { .. } => CommandAction::Train,
        CardCommand::CancelTrain => CommandAction::Cancel,
    }
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
    /// 从只读视图、事实事件拷贝与只读快照派生 HUD 数据（兼容旧入口）。
    ///
    /// 旧入口不注入表现层迷雾网格，`fog` 投影保持 `Default`、小地图不做
    /// 可见性过滤；真实装配 MUST 用 [`HudModel::collect_with_fog`]。
    pub fn collect(
        view: &WorldView,
        snapshot: &Snapshot,
        selection: &BTreeSet<u64>,
        events: &[Event],
        mapper: &ScreenMapper,
        local_player: u8,
    ) -> Self {
        Self::collect_impl(
            view,
            snapshot,
            selection,
            events,
            mapper,
            local_player,
            None,
        )
    }

    /// 真实装配入口：附带表现层本地迷雾网格的只读 owned 值拷贝。
    ///
    /// 迷雾网格只来自 `FogGrid::observe()` 的表现层派生，MUST NOT 反写
    /// `RtsMatch`、进状态哈希 / 快照 / `first_difference()`（R10.5）。
    pub fn collect_with_fog(
        view: &WorldView,
        snapshot: &Snapshot,
        selection: &BTreeSet<u64>,
        events: &[Event],
        mapper: &ScreenMapper,
        local_player: u8,
        fog: &FogGrid,
    ) -> Self {
        Self::collect_impl(
            view,
            snapshot,
            selection,
            events,
            mapper,
            local_player,
            Some(fog),
        )
    }

    /// 单一派生真值源：`WorldView` 提供单位生命 / 位置 / 归属；只读
    /// `Snapshot` 提供资源与生产队列（这些字段不在 `WorldView` 内）。
    /// `ScreenMapper` 的投影输出是物理像素，HUD 绘制在 egui 逻辑空间，
    /// 故 MUST 经 `to_logical` 换算。全程只读取、无权威状态写入路径。
    fn collect_impl(
        view: &WorldView,
        snapshot: &Snapshot,
        selection: &BTreeSet<u64>,
        events: &[Event],
        mapper: &ScreenMapper,
        local_player: u8,
        fog_grid: Option<&FogGrid>,
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
                // 单一真值源：HUD 标记中心与选中框都用 `Scene` 真实模型顶点
                // 投影出的实体屏幕包围盒（注入 boxes 时）；未注入回退旧口径。
                // 旧 `world_to_screen`（低空平面 y=19.2）/ `unit_screen_bounds`
                // （24 世界单位四面体）会与放大 4×、贴地的真实模型错位上百像素。
                let screen = mapper.to_logical(mapper.entity_screen_center(entity.id, world_f64));
                let (bounds_min, bounds_max) =
                    mapper.to_logical_bounds(mapper.entity_screen_box(entity.id, world_f64));
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
        let fog_hud = match fog_grid {
            Some(grid) => FogHudModel::derive(snapshot, events, grid),
            None => FogHudModel::default(),
        };

        Self {
            tick: view.tick,
            units,
            gold: economy.map_or(0, |e| e.gold),
            lumber: economy.map_or(0, |e| e.lumber),
            supply_used: economy.map_or(0, |e| e.supply_used),
            supply_cap: economy.map_or(0, |e| e.supply_cap),
            queue,
            // 只读派生：只消费 `snapshot.revive` 与事实事件拷贝，不写回模拟状态。
            revive: ReviveHudModel::derive(snapshot, events),
            // 只读派生：只消费 `snapshot.town_hall` 与事实事件拷贝，不写回模拟状态。
            town_hall: TownHallUpgradeHudModel::derive(snapshot, events),
            // 只读派生：只消费 `snapshot.creep_camps` 与事实事件拷贝，不写回模拟状态。
            creep: CreepCampHudModel::derive(snapshot, events),
            // 只读派生：只消费 `snapshot.item_shop` 与十一类事实事件拷贝，
            // 不写回模拟状态，也不进入状态哈希 / 快照。
            item_shop: ItemShopHudModel::derive(snapshot, events),
            // 只读派生：只消费表现层迷雾网格的 owned 值拷贝（R10.5）。
            fog: fog_hud,
            selected_count: selection.len(),
            event_count: events.len(),
            local_player,
            command_actions: Vec::new(),
            command_card: None,
            entity_defs: BTreeMap::new(),
            active_card: CardId::Main,
            selection_panel: None,
            minimap: derive_minimap(view, selection, extent, fog_grid, local_player),
            winner: snapshot.gameplay.victory.winner,
            faction: FactionSelection::default(),
            faction_overlay: false,
            kinds,
        }
    }

    /// 注入装配层的实体类型标签并重新派生命令卡与选择面板。
    /// 类型标签是表现层数据，MUST NOT 进入权威状态、状态哈希或快照。
    pub fn with_kinds(mut self, kinds: BTreeMap<u64, EntityKind>) -> Self {
        self.command_actions = derive_commands(&self.units, &kinds);
        self.selection_panel = derive_selection_panel(&self.units, &kinds);
        self.kinds = kinds;
        self.refresh_command_card();
        self
    }

    /// 注入装配层 `EntityId → DefId` 只读关联并重派生 12 格命令卡（tasks 1.4）。
    /// 表现层只读，MUST NOT 进权威状态 / 状态哈希 / 快照。
    pub fn with_entity_defs(mut self, defs: BTreeMap<u64, DefId>) -> Self {
        self.entity_defs = defs;
        self.refresh_command_card();
        self
    }

    /// 注入当前卡 ID（子菜单栈顶或按选中类型定的主卡）并重派生命令卡
    /// （R13.3 子菜单换卡）。表现层只读。
    pub fn with_active_card(mut self, card: CardId) -> Self {
        self.active_card = card;
        self.refresh_command_card();
        self
    }

    /// 注入开局前选族（表现层本地只读状态，Q1）并按本地族重派生命令卡
    /// （Q2 按族查表）。选族 MUST NOT 进权威状态 / 状态哈希 / 快照；
    /// HUD 只用它绘制 overlay 与挑选按族命令卡目录（ADR 0002 / design D1）。
    pub fn with_faction(mut self, faction: FactionSelection) -> Self {
        self.faction = faction;
        self.refresh_command_card();
        self
    }

    /// 开关开局前选族 overlay 的绘制（表现层，默认关闭）。
    /// `playable.rs` 冒烟窗口不注入，保持既有 HUD（design D8 / Q1）。
    pub fn with_faction_overlay(mut self, visible: bool) -> Self {
        self.faction_overlay = visible;
        self
    }

    /// 由当前 `entity_defs` / `active_card` / 选中集合 / 本地族按定义表
    /// 重派生 12 格。纯只读派生，不触碰任何权威状态。
    fn refresh_command_card(&mut self) {
        self.command_card = derive_command_card(
            &self.units,
            &self.entity_defs,
            self.active_card,
            self.faction.local_race,
        );
    }

    /// 注入真实小地图底图（纯表现层 CPU 降采样数据）；不改变任何权威状态。
    pub fn with_minimap_backdrop(mut self, backdrop: Option<MinimapBackdrop>) -> Self {
        self.minimap.backdrop = backdrop;
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
    /// 命令卡被点击的动作（既有兼容字段，投射成 [`CommandAction`]）。
    pub command_action: Option<CommandAction>,
    /// 12 格命令卡被点击格绑定的经典键字母（`[待核]` 合成键位）。
    ///
    /// 键盘与命令卡点击都归一化成同一个 `InputAction::Hotkey`、汇入
    /// 同一 `InputState::apply`（D1 / R13.6）；不携带「命令来源标记」。
    pub command_hotkey: Option<char>,
    /// 开局前选族 overlay 的本地族按钮点击（表现层本地只读状态，Q1）。
    pub faction_local: Option<Race>,
    /// 开局前选族 overlay 的 AI 族按钮点击（表现层本地只读状态，Q1）。
    pub faction_ai: Option<Race>,
    /// 开局前选族 overlay 的确认按钮点击（不生成任何权威命令）。
    pub faction_confirm: bool,
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
    // 几何层：`unit.screen` / `bounds_*` 由 `Scene::entity_screen_boxes` 从真实
    // 模型逐顶点投影得到（单一真值源），故队伍色点、选中框、血条都贴合用户
    // 看到的模型，不再用旧的低空平面 / 24 单位四面体口径（会错位上百像素）。
    // 选中框按真实包围盒绘制（贴合模型），并夹一个最小尺寸保证小单位可见。
    egui::Area::new(egui::Id::unique("w3_client_hud_overlay")).show(ctx, |ui| {
        let painter = ui.painter();
        for unit in model.units.iter().filter(|unit| unit.alive) {
            let hints = display_hints(unit.owner, unit.hp, unit.max_hp, unit.selected);
            let team_color = to_color(hints.team_color);
            let center = egui::pos2(unit.screen[0], unit.screen[1]);
            // 所有单位：小队伍色点，标示位置与敌我。
            painter.circle_filled(center, 2.5, team_color);
            // 仅选中单位：贴合真实模型包围盒的选中框 + 血条。
            if let Some(ring) = hints.selection_ring {
                let mut frame = egui::Rect::from_min_max(
                    egui::pos2(unit.bounds_min[0], unit.bounds_min[1]),
                    egui::pos2(unit.bounds_max[0], unit.bounds_max[1]),
                );
                // 最小尺寸 12×12：极小 / 远处模型也能看到选中框。
                if frame.width() < 12.0 || frame.height() < 12.0 {
                    frame = egui::Rect::from_center_size(center, egui::vec2(12.0, 12.0));
                }
                painter.rect_stroke(
                    frame,
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(1.5, to_color(ring)),
                    egui::StrokeKind::Outside,
                );
                let bar = egui::Rect::from_min_size(
                    egui::pos2(frame.min.x, frame.min.y - 6.0),
                    egui::vec2(frame.width().max(12.0), 3.0),
                );
                painter.rect_filled(bar, egui::CornerRadius::ZERO, egui::Color32::DARK_RED);
                let filled = egui::Rect::from_min_size(
                    bar.min,
                    egui::vec2(bar.width() * hints.hp_fraction, 3.0),
                );
                painter.rect_filled(filled, egui::CornerRadius::ZERO, team_color);
            }
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

        // 命令卡：4 列 × 3 行 = 12 格，由定义表按选中集合类型只读派生
        // （含空格位）。点击格与按该格绑定的经典键等效，二者都归一化成
        // 同一个 `InputAction::Hotkey` 汇入同一 `apply`（R13.1 / R13.6）。
        ui.label(texts.commands);
        match &model.command_card {
            None => {
                ui.label(texts.no_selection);
            }
            Some(card) => {
                for y in 0..GRID_HEIGHT {
                    ui.horizontal(|ui| {
                        for x in 0..GRID_WIDTH {
                            let position = slot(x, y);
                            match card.at(position) {
                                Some(button) => {
                                    // 标签走 i18n 边界：静态 ASCII 文案，
                                    // MUST NOT 进日志 / 权威状态 / 哈希。
                                    let text = format!("[{}] {}", button.hotkey, button.label);
                                    if ui.button(text).clicked() {
                                        interaction.command_hotkey = Some(button.hotkey);
                                        interaction.command_action =
                                            Some(compat_action(button.command));
                                    }
                                }
                                None => {
                                    // 空格位：占位，不响应点击（12 格含空格）。
                                    ui.add_enabled(false, egui::Button::new(" - "));
                                }
                            }
                        }
                    });
                }
            }
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

        // 祭坛复活队列：只读投影的进度 / 费用 / 剩余帧（P8.5a / tasks 6.1）。
        // 文案全部走 i18n，MUST NOT 进日志 / 权威状态 / 状态哈希（ADR 0002 §5）。
        ui.label(texts.revive_queue);
        if model.revive.queue.is_empty() {
            ui.label(texts.revive_empty);
        } else {
            for item in &model.revive.queue {
                ui.label(format!(
                    "{} #{}: {} {}/{} | {} {} {}",
                    texts.revive_hero,
                    item.hero,
                    texts.revive_remaining,
                    item.remaining_frames,
                    item.total_frames,
                    texts.revive_cost,
                    item.gold,
                    item.lumber,
                ));
            }
        }
        ui.label(format!(
            "{}: {} | {}: {} | {}: {} | {}: {}",
            texts.revive_deaths,
            model.revive.deaths,
            texts.revive_charges,
            model.revive.charges,
            texts.hero_died,
            model.revive.hero_died_events,
            texts.hero_revived,
            model.revive.revived_events,
        ));
        ui.separator();

        // 主城升级队列：只读投影的当前 tier、进度 / 剩余帧 / 费用
        // （P8.5b / tasks 6.1-6.2）。文案全部走 i18n，MUST NOT 进日志 /
        // 权威状态 / 状态哈希（ADR 0002 §5 / D6）。
        ui.label(texts.town_hall_queue);
        if model.town_hall.queue.is_empty() {
            ui.label(texts.town_hall_empty);
        } else {
            for item in &model.town_hall.queue {
                ui.label(format!(
                    "{} #{}: T{} -> T{} | {} {}/{} | {} {} {}",
                    texts.town_hall,
                    item.town_hall,
                    item.from_tier.code(),
                    item.target_tier.code(),
                    texts.town_hall_remaining,
                    item.remaining_frames,
                    item.total_frames,
                    texts.town_hall_cost,
                    item.gold,
                    item.lumber,
                ));
            }
        }
        for (entity, tier) in &model.town_hall.tiers {
            ui.label(format!(
                "{} #{}: {} T{}",
                texts.town_hall,
                entity,
                texts.town_hall_tier,
                tier.code(),
            ));
        }
        ui.label(format!(
            "{}: {} | {}: {} | {}: {} | {}: {} | {}: {}",
            texts.town_hall_charges,
            model.town_hall.charges,
            texts.town_hall_started,
            model.town_hall.started_events,
            texts.town_hall_completed,
            model.town_hall.completed_events,
            texts.town_hall_cancelled,
            model.town_hall.cancelled_events,
            texts.town_hall_tier_count,
            model.town_hall.tiers.len(),
        ));
        ui.separator();

        // 野怪营地 HUD：血条 / 掉落标记 / 击杀收益 / 事实事件计数全部来自
        // `CreepCampHudModel` 只读投影；文案全部走 i18n，MUST NOT 进日志 /
        // 权威状态 / 状态哈希（ADR 0002 §5 / D7）。
        ui.label(texts.creep_camps);
        if model.creep.camp_count == 0
            && model.creep.creep_bars.is_empty()
            && model.creep.drop_markers.is_empty()
        {
            ui.label(texts.creep_empty);
        } else {
            ui.label(format!(
                "{}: {} | {}: {} | {}: {}/{}",
                texts.creep_camps,
                model.creep.camp_count,
                texts.creep_cleared,
                model.creep.cleared_camps,
                texts.creep_alive,
                model.creep.alive_creeps,
                model.creep.total_creeps,
            ));
            for bar in model.creep.creep_bars.values().take(4) {
                ui.label(format!(
                    "{} #{}: {} {}/{} | {} {}",
                    texts.creep_hp,
                    bar.entity,
                    texts.hp,
                    bar.hp,
                    bar.max_hp,
                    texts.creep_phase,
                    bar.phase.code(),
                ));
            }
            ui.label(format!(
                "{}: {} | {}: {} {} / {} {} | {}: {}",
                texts.creep_drops,
                model.creep.drop_markers.len(),
                texts.creep_rewards,
                model.creep.reward_xp,
                texts.creep_xp,
                model.creep.reward_gold,
                texts.creep_gold,
                texts.creep_events,
                model.creep.rng_draws,
            ));
            ui.label(format!(
                "{}: {} | {}: {} | {}: {} | {}: {} | {}: {} | {}: {}",
                texts.creep_seeded,
                model.creep.seeded_events,
                texts.creep_engaged,
                model.creep.engaged_events,
                texts.creep_leash_returning,
                model.creep.leash_returning_events,
                texts.creep_leash_returned,
                model.creep.leash_returned_events,
                texts.creep_died,
                model.creep.creep_died_events,
                texts.item_dropped,
                model.creep.item_dropped_events,
            ));
        }
        ui.separator();

        // 物品 / 商店 / 回城 HUD：六格背包 / 库存 / 施法条 / 事实计数全部来自
        // `ItemShopHudModel` 只读投影；文案全部走 i18n，MUST NOT 进日志 /
        // 权威状态 / 状态哈希（ADR 0002 §5 / R10.5 / R13.5）。
        ui.label(texts.item_shop);
        if model.item_shop.bags.is_empty()
            && model.item_shop.stock.is_empty()
            && model.item_shop.casts.is_empty()
            && model.item_shop.facts.is_empty()
        {
            ui.label(texts.item_shop_empty);
        } else {
            for bag in model.item_shop.bags.iter().take(2) {
                let slots: Vec<String> = bag
                    .slots
                    .iter()
                    .enumerate()
                    .map(|(slot, instance)| match instance {
                        Some(instance) => {
                            format!("{}#{}:{}", texts.item_shop_slot, slot, instance)
                        }
                        None => format!("{}#{}:-", texts.item_shop_slot, slot),
                    })
                    .collect();
                ui.label(format!(
                    "{} #{}: {}",
                    texts.item_shop_bag,
                    bag.hero,
                    slots.join(" ")
                ));
            }
            for (&shelf, &remaining) in model.item_shop.stock.iter().take(3) {
                ui.label(format!(
                    "{} #{}: {}",
                    texts.item_shop_stock, shelf, remaining
                ));
            }
            for cast in model.item_shop.casts.iter().take(2) {
                ui.label(format!(
                    "{} #{} -> #{}: {}/{}",
                    texts.item_shop_casts,
                    cast.caster,
                    cast.target,
                    cast.remaining_frames,
                    cast.total_frames
                ));
            }
            ui.label(format!(
                "{}: {} | {}: {}",
                texts.item_shop_facts,
                model.item_shop.facts.len(),
                texts.item_shop_events,
                model.item_shop.event_count
            ));
            ui.label(format!(
                "{} {} | {} {} | {} {} | {} {} | {} {} | {} {}",
                texts.item_shop_picked_up,
                model.item_shop.picked_up_events,
                texts.item_shop_purchased,
                model.item_shop.purchased_events,
                texts.item_shop_used,
                model.item_shop.used_events,
                texts.item_shop_discarded,
                model.item_shop.discarded_events,
                texts.item_shop_transferred,
                model.item_shop.transferred_events,
                texts.item_shop_sold,
                model.item_shop.sold_events,
            ));
            ui.label(format!(
                "{} {} | {} {} | {} {} | {} {} | {} {}",
                texts.item_shop_portal_started,
                model.item_shop.portal_started_events,
                texts.item_shop_portal_completed,
                model.item_shop.portal_completed_events,
                texts.item_shop_portal_cancelled,
                model.item_shop.portal_cancelled_events,
                texts.item_shop_refreshed,
                model.item_shop.market_refreshed_events,
                texts.item_shop_rejected,
                model.item_shop.rejected_events,
            ));
        }
        ui.separator();

        // 战争迷雾 HUD：三态计数 / 昼夜相位 / 中立标记都只来自 `FogHudModel`
        // 的 owned 值拷贝；文案走 i18n，不进日志 / 权威状态 / 状态哈希。
        if model.fog.last_tick.is_some() {
            let phase = if model.fog.phase_code == 0 {
                texts.fog_day
            } else {
                texts.fog_night
            };
            ui.label(texts.fog_of_war);
            ui.label(format!(
                "{} {} | {} {} | {} {} | {} {}",
                texts.fog_masked,
                model.fog.counts.masked,
                texts.fog_fogged,
                model.fog.counts.fogged,
                texts.fog_visible,
                model.fog.counts.visible,
                texts.fog_explored,
                model.fog.counts.explored,
            ));
            ui.label(format!(
                "{}: {} {}: {} | {}: {} | {}: {}",
                texts.tick,
                model.fog.last_tick.unwrap_or(0),
                texts.fog_phase,
                phase,
                texts.fog_markers,
                model.fog.visible_markers.len(),
                texts.item_shop_events,
                model.fog.event_count,
            ));
            ui.separator();
        }

        // 小地图：只读实体点 + 相机视口框；点击只返回归一化坐标。
        ui.label(texts.minimap);
        let size = egui::vec2(160.0, 160.0);
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
        let rect = response.rect;
        if let Some(backdrop) = &model.minimap.backdrop {
            if !backdrop.colors.is_empty() && backdrop.grid > 0 {
                let cell_w = rect.width() / backdrop.grid as f32;
                let cell_h = rect.height() / backdrop.grid as f32;
                for (index, color) in backdrop.colors.iter().enumerate() {
                    let gx = (index % backdrop.grid) as f32;
                    let gy = (index / backdrop.grid) as f32;
                    let cell = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + gx * cell_w, rect.min.y + gy * cell_h),
                        egui::vec2(cell_w.ceil(), cell_h.ceil()),
                    );
                    painter.rect_filled(cell, egui::CornerRadius::ZERO, to_color(*color));
                }
            } else {
                painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(24));
            }
        } else {
            painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_gray(24));
        }
        // 雾遮罩叠在底图之上、单位点之下；相机视口框在后面单独绘制，
        // 与遮罩互不影响。遮罩网格缺失时显式报告并跳过对应元素。
        if let Some(fog) = &model.minimap.fog {
            if fog.grid > 0 && fog.opacities.len() == fog.grid * fog.grid {
                let cell_w = rect.width() / fog.grid as f32;
                let cell_h = rect.height() / fog.grid as f32;
                for (index, opacity) in fog.opacities.iter().enumerate() {
                    if *opacity == 0 {
                        continue;
                    }
                    let gx = (index % fog.grid) as f32;
                    let gy = (index / fog.grid) as f32;
                    let cell = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + gx * cell_w, rect.min.y + gy * cell_h),
                        egui::vec2(cell_w.ceil(), cell_h.ceil()),
                    );
                    painter.rect_filled(
                        cell,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_black_alpha(*opacity),
                    );
                }
            } else {
                log::warn!(
                    target: "w3_client",
                    "fog overlay unavailable: reason=invalid_grid scope=presentation_only action=skip_fog_overlay grid={} cells={}",
                    fog.grid,
                    fog.opacities.len()
                );
            }
        }
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

    draw_faction_overlay(
        ctx,
        &model.faction,
        model.faction_overlay,
        texts,
        &mut interaction,
    );
    interaction
}

/// 绘制开局前可见的选族 overlay（窗口内 egui，Q1 用户 2026-11-18 拍板）。
///
/// 只读取表现层本地只读状态 [`FactionSelection`]，回传按钮点击到
/// [`HudInteraction`]；状态机与确认动作落在 [`crate::app::ClientApp`]
/// 装配链内部。未注入 overlay 或 `confirmed == true` 后不绘制，MUST NOT
/// 生成任何权威命令、不进日志 / 权威状态 / 状态哈希 / 快照（ADR 0002）。
/// UI 文案全部来自 [`Texts`]（ASCII，走 i18n 边界，不写死中文名 / 图标表）。
fn draw_faction_overlay(
    ctx: &egui::Context,
    faction: &FactionSelection,
    visible: bool,
    texts: &Texts,
    interaction: &mut HudInteraction,
) {
    if !visible || faction.confirmed {
        return;
    }
    egui::Window::new(texts.faction_title)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // 只读状态提示（ASCII 文案，MUST NOT 进日志 / 权威状态 / 哈希）。
            ui.label(format!(
                "{}: {} {} / {} {}",
                texts.faction_status,
                texts.faction_local,
                race_label(faction.local_race, texts),
                texts.faction_ai,
                race_label(faction.ai_race, texts),
            ));
            ui.separator();

            ui.label(texts.faction_local);
            ui.horizontal(|ui| {
                if ui.button(texts.faction_human).clicked() {
                    interaction.faction_local = Some(Race::Human);
                }
                if ui.button(texts.faction_orc).clicked() {
                    interaction.faction_local = Some(Race::Orc);
                }
            });

            ui.label(texts.faction_ai);
            ui.horizontal(|ui| {
                if ui.button(texts.faction_human).clicked() {
                    interaction.faction_ai = Some(Race::Human);
                }
                if ui.button(texts.faction_orc).clicked() {
                    interaction.faction_ai = Some(Race::Orc);
                }
            });

            ui.separator();
            if ui.button(texts.faction_confirm).clicked() {
                interaction.faction_confirm = true;
            }
        });
}

/// 选族的只读 ASCII 展示标签（走 i18n 边界，不进日志 / 权威状态 / 哈希）。
fn race_label(race: Race, texts: &Texts) -> &'static str {
    match race {
        Race::Human => texts.faction_human,
        Race::Orc => texts.faction_orc,
    }
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
                speed: 3 * SCALE,
                hp: 100,
                owner: 1,
                alive: true,
                has_target: false,
            }],
        };
        let mut selection = BTreeSet::new();
        selection.insert(1u64);
        let minimap = derive_minimap(&view, &selection, 256.0, None, 1);
        assert_eq!(minimap.points.len(), 1);
        assert!((minimap.points[0].nx - 0.5).abs() < 1.0e-6);
        assert!(minimap.points[0].selected);
        assert_eq!(
            minimap.points[0].category,
            MinimapPointCategory::Entity,
            "reason=entity_point_category"
        );
        assert_eq!(minimap.extent, 256.0);
        // 纯派生路径不注入底图 / 雾遮罩；两者由装配层显式注入。
        assert!(minimap.backdrop.is_none());
        assert!(minimap.fog.is_none());
    }

    fn solid_rgba(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..(width as usize * height as usize) {
            pixels.extend_from_slice(&color);
        }
        RgbaImage {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn minimap_backdrop_downsamples_and_is_deterministic() {
        let image = solid_rgba(8, 8, [100, 150, 200, 255]);
        let a = minimap_backdrop(&image, 4);
        let b = minimap_backdrop(&image, 4);
        assert_eq!(a, b);
        assert_eq!(a.grid, 4);
        assert_eq!(a.colors.len(), 16);
        // 纯色图每个网格都应等于该颜色的归一化值。
        for color in &a.colors {
            assert!((color[0] - 100.0 / 255.0).abs() < 1.0e-6);
            assert!((color[1] - 150.0 / 255.0).abs() < 1.0e-6);
            assert!((color[2] - 200.0 / 255.0).abs() < 1.0e-6);
            assert!((color[3] - 1.0).abs() < 1.0e-6);
        }
    }

    #[test]
    fn minimap_backdrop_handles_degenerate_inputs_without_panic() {
        let empty = RgbaImage {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
        assert!(minimap_backdrop(&empty, 8).colors.is_empty());
        let image = solid_rgba(2, 2, [10, 20, 30, 255]);
        assert!(minimap_backdrop(&image, 0).colors.is_empty());
        // 网格数大于像素数时仍不 panic、尺寸正确。
        assert_eq!(minimap_backdrop(&image, 16).colors.len(), 256);
    }
}
