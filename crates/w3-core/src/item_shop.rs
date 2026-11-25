//! P8.5d 物品 / 商店 / 回城卷轴：冻结定义层与顶层权威状态容器。
//!
//! 本模块只提供纯整数的数据结构与确定性编码，不读取文件、不依赖
//! `w3-content` / 渲染 / 网络，也不持有闭包、活动对象、墙钟或浮点。
//! 分两层，MUST NOT 混淆（ADR 0002 §6-8 / Q6 / Q8）：
//! - **冻结定义层**：[`ItemShopConstants`] / [`ShopShelf`] / [`ShopFixture`] /
//!   [`ItemShopDefinitions`]。全部为合成原创编译期整数常量 `[待核]`，经
//!   `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部；
//!   改任一常量即改变 `content_hash()`，属「定义层内容变化」。
//! - **权威状态层**：[`ItemShopState`] 及其成员。每帧由 `RtsMatch` 独占
//!   写入，MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`。
//!   六格背包每格挂稳定 `EntityId` 指向 `CreepCampState.item_instances`，
//!   `ItemInstanceState` 的 `holder` / `slot` 是同一事实的反向索引，
//!   实例身份唯一、无 DefId 序列（R6.2 / R6.3 / R7.5）。
//!
//! 业务规则：R1.1 / R1.4 / R6.2 / R6.3 / R7.1–R7.6 / R8.6 / R9.1 / R9.3 /
//! R12.2 / R12.4。数值全部 `[待核]`，不冒充原版（§14 / ADR 0001 / ADR 0004）。

use std::collections::{BTreeMap, BTreeSet};

use crate::entity::EntityId;
use crate::fixed::SCALE;
use crate::gameplay::{DefId, EffectDef, EffectKind, effect_kind_code};
use crate::state_hash::StateHasher;

/// 六格定长实例背包的槽位数（R6.2，合成原创 `[待核]`）。
pub const BAG_SLOTS: usize = 6;

/// 商店定义层货架 / 主城分类用的稳定标识段，与 `EntityStore` 单位 ID、
/// `GROUND_ITEM_ID_BASE` 段不重叠；商店是定义层货架表，不是 R9.1 中立建筑实体。
pub const SHOP_FIXTURE_ID_BASE: u64 = 1u64 << 40;

/// 由定义层商店 id 派生稳定标识（供命令 / 事实事件按 `Event.entity` 查表）。
pub const fn shop_fixture_id(shop: u32) -> u64 {
    SHOP_FIXTURE_ID_BASE + shop as u64
}

/// 道具类别（冻结定义层，用整数编码表达，Q8）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ItemCategory {
    /// 消耗品：充能耗尽后移除实例（R7.1 / R7.5）。
    Consumable,
    /// 永久宝物：充能耗尽仍持有。
    Permanent,
}

impl ItemCategory {
    /// 稳定的单字节编码，不依赖枚举内存布局（ADR 0002 §6）。
    pub const fn code(self) -> u8 {
        match self {
            ItemCategory::Consumable => 1,
            ItemCategory::Permanent => 2,
        }
    }

    /// 从显式编码还原；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(ItemCategory::Consumable),
            2 => Some(ItemCategory::Permanent),
            _ => None,
        }
    }
}

impl Default for ItemCategory {
    fn default() -> Self {
        ItemCategory::Consumable
    }
}

/// 商店 / 道具 / 回城冻结常量集合（全为合成 / 原创编译期整数 `[待核]`）。
///
/// 距离字段是 Q47.16 原始值（与 `SimFixed::raw()` 同域），时间字段是
/// 30 Hz 整数帧。改任一常量即改变 `content_hash()`（Q8 / §14）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemShopConstants {
    /// 六格背包槽位数（R6.2）。
    pub bag_slots: u8,
    /// 地面物品拾取半径（Q47.16 raw，R7.2 / R1.1）。
    pub pickup_radius_raw: i64,
    /// 商店默认交互半径（Q47.16 raw，R9.1，定义层数据驱动）。
    pub shop_interact_radius_raw: i64,
    /// 售卖比例百分比：按购入价整数缩放（R7.4，`[待核]`）。
    pub sell_ratio_percent: i64,
    /// 实例默认初始充能（R7.5）。
    pub default_charges: u32,
    /// 实例默认共享冷却组帧数（R7.5，整数帧非 `float`）。
    pub default_cooldown_frames: u32,
    /// 回城施法帧数（30 Hz 整数帧，R7.1 / 结论 7）。
    pub town_portal_cast_frames: u32,
    /// 回城卷轴每英雄可用次数上限（定义层 `[待核]`）。
    pub town_portal_max_uses: u32,
    /// 回城卷轴一次施法消耗的充能（定义层 `[待核]`）。
    pub town_portal_charge_cost: u32,
    /// 货架默认库存上限（R9.1）。
    pub default_stock_max: u32,
    /// 货架默认刷新间隔（30 Hz 整数帧，R8.6）。
    pub default_replenish_interval_frames: u32,
    /// 友方主城分类标记（定义层整数类别码，MUST NOT 用建筑名白名单 / rawcode）。
    pub town_hall_class: u32,
    /// 商店分类标记（定义层整数类别码，MUST NOT 用建筑名白名单 / rawcode）。
    pub shop_class: u32,
}

/// 合成 / 原创的物品 / 商店 / 回城冻结常量 `[待核]`；不来自任何资产或参考项目数值。
pub const SYNTHETIC_ITEM_SHOP_CONSTANTS: ItemShopConstants = ItemShopConstants {
    bag_slots: BAG_SLOTS as u8,
    pickup_radius_raw: 4 * SCALE,
    shop_interact_radius_raw: 8 * SCALE,
    sell_ratio_percent: 40,
    default_charges: 3,
    default_cooldown_frames: 10,
    town_portal_cast_frames: 12,
    town_portal_max_uses: 2,
    town_portal_charge_cost: 1,
    default_stock_max: 3,
    default_replenish_interval_frames: 30,
    town_hall_class: 71,
    shop_class: 72,
};

impl Default for ItemShopConstants {
    fn default() -> Self {
        SYNTHETIC_ITEM_SHOP_CONSTANTS
    }
}

impl ItemShopConstants {
    /// 显式小端编码进定义哈希；任一常量变化都改变结果（tasks 1.2）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u8(self.bag_slots);
        hasher.write_i64(self.pickup_radius_raw);
        hasher.write_i64(self.shop_interact_radius_raw);
        hasher.write_i64(self.sell_ratio_percent);
        hasher.write_u32(self.default_charges);
        hasher.write_u32(self.default_cooldown_frames);
        hasher.write_u32(self.town_portal_cast_frames);
        hasher.write_u32(self.town_portal_max_uses);
        hasher.write_u32(self.town_portal_charge_cost);
        hasher.write_u32(self.default_stock_max);
        hasher.write_u32(self.default_replenish_interval_frames);
        hasher.write_u32(self.town_hall_class);
        hasher.write_u32(self.shop_class);
    }
}

/// 单个货架条目（冻结定义层，Q6 / Q8）。
///
/// `effect` 走既有 [`EffectDef`] 组合、MUST NOT 引入 JASS VM 或新
/// `EffectKind` 变体（R7.6 / N5）。`town_portal` 是定义层类别标记，
/// MUST NOT 用 `852286` / `AItp` / `AIte` rawcode 或建筑名白名单。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShopShelf {
    /// 货架售卖的道具定义 id（数据驱动，非硬编码 rawcode）。
    pub item_def: DefId,
    /// 道具类别（消耗品 / 永久宝物）。
    pub category: ItemCategory,
    /// 购入价（精确整数黄金，`[待核]`）。
    pub price: i64,
    /// 回购价：显著低于 `price`（R7.4，`[待核]`）。
    pub sell_price: i64,
    /// 库存上限（冻结常量，R9.1）。
    pub stock_max: u32,
    /// 刷新间隔（30 Hz 整数帧，R8.6）。
    pub replenish_interval_frames: u32,
    /// 解锁时间（整数帧，R9.1）。
    pub unlock_frame: u64,
    /// 新建实例初始充能（R7.5）。
    pub charges: u32,
    /// 新建实例初始共享冷却组帧数（R7.5）。
    pub cooldown_frames: u32,
    /// 共享冷却组整数 id（定义层，R7.5）。
    pub cooldown_group: u32,
    /// 使用效果（既有 `EffectDef` 组合）。
    pub effect: EffectDef,
    /// 回城卷轴类别标记（独立单英雄传送，非 MassTeleport 群体传送）。
    pub town_portal: bool,
    /// 是否可主动使用（R7.6）。
    pub usable: bool,
    /// 是否可售卖（R7.4）。
    pub saleable: bool,
}

impl ShopShelf {
    /// 显式小端编码进定义哈希；任一字段变化都改变 `content_hash()`。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.item_def);
        hasher.write_u8(self.category.code());
        hasher.write_i64(self.price);
        hasher.write_i64(self.sell_price);
        hasher.write_u32(self.stock_max);
        hasher.write_u32(self.replenish_interval_frames);
        hasher.write_u64(self.unlock_frame);
        hasher.write_u32(self.charges);
        hasher.write_u32(self.cooldown_frames);
        hasher.write_u32(self.cooldown_group);
        hasher.write_u8(effect_kind_code(self.effect.kind));
        hasher.write_i64(self.effect.amount);
        hasher.write_u32(self.effect.duration_frames);
        hasher.write_u8(u8::from(self.town_portal));
        hasher.write_u8(u8::from(self.usable));
        hasher.write_u8(u8::from(self.saleable));
    }
}

/// 定义层商店表条目：冻结整数坐标 + 交互半径（Q6 最小建模，非中立建筑实体）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShopFixture {
    /// 商店稳定 id（同时是权威库存 / 刷新计数的键）。
    pub shop: u32,
    /// 冻结坐标 x（Q47.16 raw）。
    pub x: i64,
    /// 冻结坐标 y（Q47.16 raw）。
    pub y: i64,
    /// 交互半径（Q47.16 raw，定义层数据驱动）。
    pub interact_radius_raw: i64,
    /// 该店货架 id 列表（稳定顺序，数据驱动）。
    pub shelf_ids: BTreeSet<u32>,
}

impl ShopFixture {
    /// 显式小端编码进定义哈希。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.shop);
        hasher.write_i64(self.x);
        hasher.write_i64(self.y);
        hasher.write_i64(self.interact_radius_raw);
        hasher.write_u64(self.shelf_ids.len() as u64);
        for &shelf in &self.shelf_ids {
            hasher.write_u32(shelf);
        }
    }
}

/// 物品 / 商店 / 回城冻结定义段（注入 `FrozenDefinitions`，Q6 / Q8）。
///
/// 全部用 `BTreeMap` / `BTreeSet` 确定性有序容器，按稳定键序编入
/// `compute_defs_hash`；改任一段即改变 `content_hash()`。
/// 类别判定只查定义层登记集合，MUST NOT 用名字白名单或硬编码 DefId（R1.2 / R13.3）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ItemShopDefinitions {
    /// 冻结常量段。
    pub constants: ItemShopConstants,
    /// 货架清单（稳定货架 id）。
    pub shelves: BTreeMap<u32, ShopShelf>,
    /// 定义层商店表（稳定商店 id）。
    pub shops: BTreeMap<u32, ShopFixture>,
    /// 被登记为「商店」类别的商店 id 集合（数据驱动，与 `shops` 键一致）；
    /// 非经典命名的商店 id 只要登记即可被判定（R1.2 / R13.3）。
    pub shop_defs: BTreeSet<DefId>,
    /// 被登记为「友方主城」类别的单位 DefId 集合（数据驱动，R7.1）。
    pub town_hall_defs: BTreeSet<DefId>,
    /// 道具类别登记（含 P8.5c 掉落道具，R7.5 / R7.6）。
    pub item_categories: BTreeMap<DefId, ItemCategory>,
    /// 道具共享冷却组登记（定义层整数 id，R7.5）。
    pub item_cooldown_groups: BTreeMap<DefId, u32>,
    /// 道具效果登记（既有 `EffectDef` 组合，R7.6 / N5）。
    pub item_effects: BTreeMap<DefId, EffectDef>,
    /// 道具回购价登记（是否可售与精确入账由定义层数据驱动，R7.4 / R9.3）。
    pub item_sell_prices: BTreeMap<DefId, i64>,
    /// 回城卷轴道具 DefId 集合（定义层类别，非 rawcode）。
    pub town_portal_items: BTreeSet<DefId>,
}

impl ItemShopDefinitions {
    /// 显式小端编码进定义哈希（tasks 1.2 / Q8）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        self.constants.hash(hasher);
        hasher.write_u64(self.shelves.len() as u64);
        for (&shelf_id, shelf) in &self.shelves {
            hasher.write_u32(shelf_id);
            shelf.hash(hasher);
        }
        hasher.write_u64(self.shops.len() as u64);
        for (&shop_id, shop) in &self.shops {
            hasher.write_u32(shop_id);
            shop.hash(hasher);
        }
        hasher.write_u64(self.shop_defs.len() as u64);
        for &def in &self.shop_defs {
            hasher.write_u32(def);
        }
        hasher.write_u64(self.town_hall_defs.len() as u64);
        for &def in &self.town_hall_defs {
            hasher.write_u32(def);
        }
        hasher.write_u64(self.item_categories.len() as u64);
        for (&def, &category) in &self.item_categories {
            hasher.write_u32(def);
            hasher.write_u8(category.code());
        }
        hasher.write_u64(self.item_cooldown_groups.len() as u64);
        for (&def, &group) in &self.item_cooldown_groups {
            hasher.write_u32(def);
            hasher.write_u32(group);
        }
        hasher.write_u64(self.item_effects.len() as u64);
        for (&def, effect) in &self.item_effects {
            hasher.write_u32(def);
            hasher.write_u8(effect_kind_code(effect.kind));
            hasher.write_i64(effect.amount);
            hasher.write_u32(effect.duration_frames);
        }
        hasher.write_u64(self.item_sell_prices.len() as u64);
        for (&def, &price) in &self.item_sell_prices {
            hasher.write_u32(def);
            hasher.write_i64(price);
        }
        hasher.write_u64(self.town_portal_items.len() as u64);
        for &def in &self.town_portal_items {
            hasher.write_u32(def);
        }
    }

    /// 按货架 id 查冻结货架（数据驱动）。
    pub fn shelf(&self, shelf_id: u32) -> Option<&ShopShelf> {
        self.shelves.get(&shelf_id)
    }

    /// 按商店 id 查冻结商店表（数据驱动）。
    pub fn shop(&self, shop_id: u32) -> Option<&ShopFixture> {
        self.shops.get(&shop_id)
    }

    /// 该 DefId 是否被登记为商店类别（MUST NOT 用名字白名单）。
    pub fn is_shop_def(&self, def_id: DefId) -> bool {
        self.shop_defs.contains(&def_id)
    }

    /// 该 DefId 是否被登记为友方主城类别（结论 7 / R1.2）。
    pub fn is_town_hall_def(&self, def_id: DefId) -> bool {
        self.town_hall_defs.contains(&def_id)
    }

    /// 该道具是否为回城卷轴类别（数据驱动，非 rawcode）。
    pub fn is_town_portal_item(&self, def_id: DefId) -> bool {
        self.town_portal_items.contains(&def_id)
    }

    /// 该道具的冻结类别；未登记返回 `None`（道具可用性数据驱动）。
    pub fn item_category(&self, def_id: DefId) -> Option<ItemCategory> {
        self.item_categories.get(&def_id).copied()
    }

    /// 该道具的共享冷却组；未登记返回 0（定义层字段，R7.5）。
    pub fn item_cooldown_group(&self, def_id: DefId) -> u32 {
        self.item_cooldown_groups.get(&def_id).copied().unwrap_or(0)
    }

    /// 该道具的冻结效果组合；未登记返回 None（R7.6）。
    pub fn item_effect(&self, def_id: DefId) -> Option<EffectDef> {
        self.item_effects.get(&def_id).copied()
    }

    /// 该道具的冻结回购价；未登记返回 `None` 表示不可售卖（R7.4 / R9.3）。
    pub fn item_sell_price(&self, def_id: DefId) -> Option<i64> {
        self.item_sell_prices.get(&def_id).copied()
    }

    /// 按冻结货架的购入价与售卖比例确定性推导回购价（`i128` 中间量，
    /// 整数除法向零截断，ADR 0002 §1）；未登记货架返回 `None`。
    pub fn shelf_sell_price(&self, shelf_id: u32) -> Option<i64> {
        let shelf = self.shelves.get(&shelf_id)?;
        let ratio = i128::from(self.constants.sell_ratio_percent.max(0));
        let scaled = i128::from(shelf.price.max(0)) * ratio / 100;
        Some(scaled.min(i128::from(i64::MAX)) as i64)
    }

    /// 精确回购价：登记回购价与「购入价 × 售卖比例」取较小者（R7.4 / Q8）。
    /// 按稳定货架 id 顺序查找关联货架，全整数、确定性；未登记返回 `None`
    /// 表示不可售卖。改任一冻结常量 / 货架 / 回购价即改变结算结果。
    pub fn effective_sell_price(&self, def_id: DefId) -> Option<i64> {
        let registered = self.item_sell_prices.get(&def_id).copied()?;
        let ratio_cap = self
            .shelves
            .iter()
            .find(|(_, shelf)| shelf.item_def == def_id)
            .and_then(|(&shelf_id, _)| self.shelf_sell_price(shelf_id))
            .unwrap_or(i64::MAX);
        Some(registered.min(ratio_cap.max(0)))
    }
}

/// 干净环境可用的合成原创定义段（零文件 IO、零资产、全编译期整数 `[待核]`）。
///
/// 供 `w3-core` 内部回归与下游装配层复用；所有 DefId 均为非经典命名的
/// 合成整数，登记进定义层集合后即可被判定（R1.2 / tasks 1.3）。
pub fn synthetic_item_shop_definitions() -> ItemShopDefinitions {
    let consumable = EffectDef {
        kind: EffectKind::Heal,
        amount: 20,
        duration_frames: 0,
    };
    let portal = EffectDef {
        kind: EffectKind::BuffStat,
        amount: 1,
        duration_frames: 8,
    };
    let permanent = EffectDef {
        kind: EffectKind::BuffStat,
        amount: 2,
        duration_frames: 0,
    };
    let shelf = |item_def: DefId,
                 category: ItemCategory,
                 price: i64,
                 sell_price: i64,
                 cooldown_group: u32,
                 effect: EffectDef,
                 town_portal: bool| ShopShelf {
        item_def,
        category,
        price,
        sell_price,
        stock_max: 3,
        replenish_interval_frames: 30,
        unlock_frame: 0,
        charges: 3,
        cooldown_frames: 10,
        cooldown_group,
        effect,
        town_portal,
        usable: true,
        saleable: true,
    };
    ItemShopDefinitions {
        constants: SYNTHETIC_ITEM_SHOP_CONSTANTS,
        shelves: [
            (
                101u32,
                shelf(9001, ItemCategory::Consumable, 50, 15, 1, consumable, false),
            ),
            (
                102u32,
                shelf(9002, ItemCategory::Consumable, 80, 25, 2, portal, true),
            ),
            (
                103u32,
                shelf(9003, ItemCategory::Permanent, 120, 40, 3, permanent, false),
            ),
        ]
        .into_iter()
        .collect(),
        shops: [(
            7u32,
            ShopFixture {
                shop: 7,
                x: 2 * SCALE,
                y: 2 * SCALE,
                interact_radius_raw: 8 * SCALE,
                shelf_ids: [101u32, 102, 103].into_iter().collect(),
            },
        )]
        .into_iter()
        .collect(),
        shop_defs: [7u32].into_iter().collect(),
        town_hall_defs: [8002u32].into_iter().collect(),
        item_categories: [
            (9001u32, ItemCategory::Consumable),
            (9002, ItemCategory::Consumable),
            (9003, ItemCategory::Permanent),
            (9201, ItemCategory::Consumable),
            (9202, ItemCategory::Consumable),
        ]
        .into_iter()
        .collect(),
        item_cooldown_groups: [(9001u32, 1u32), (9002, 2), (9003, 3), (9201, 1), (9202, 2)]
            .into_iter()
            .collect(),
        item_effects: [
            (9001u32, consumable),
            (9002, portal),
            (9003, permanent),
            (9201, consumable),
            (9202, permanent),
        ]
        .into_iter()
        .collect(),
        item_sell_prices: [
            (9001u32, 15i64),
            (9002, 25),
            (9003, 40),
            (9201, 9),
            (9202, 11),
        ]
        .into_iter()
        .collect(),
        town_portal_items: [9002u32].into_iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// 权威状态层（Q6 / Q7 / D1 / D5）
// ---------------------------------------------------------------------------

/// 六格定长实例背包：每格挂稳定 `EntityId` 指向 `item_instances`（R6.2 / R7.5）。
///
/// MUST NOT 回退为 `Vec<DefId>` 的 DefId 序列；同名道具多实例各有独立
/// `EntityId`，充能 / 冷却 / effect 挂实例（R7.5）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemBag {
    /// 定长 6 格；`None` 表示空槽。
    pub slots: [Option<EntityId>; BAG_SLOTS],
}

impl Default for ItemBag {
    fn default() -> Self {
        Self {
            slots: [None; BAG_SLOTS],
        }
    }
}

impl ItemBag {
    /// 第一个空槽（`0..6`）；满包返回 `None`（显式拒绝、不静默丢弃）。
    pub fn first_free_slot(&self) -> Option<u8> {
        self.slots
            .iter()
            .position(|slot| slot.is_none())
            .map(|index| index as u8)
    }

    /// 是否满包。
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(Option::is_some)
    }

    /// 按实例 `EntityId` 查槽位（拾取幂等 / 丢弃 / 移交 / 售卖用）。
    pub fn slot_of(&self, instance: EntityId) -> Option<u8> {
        self.slots
            .iter()
            .position(|slot| *slot == Some(instance))
            .map(|index| index as u8)
    }

    /// 写入槽位；越界返回 `false`，不产生改动。
    pub fn set(&mut self, slot: u8, instance: Option<EntityId>) -> bool {
        let Some(target) = self.slots.get_mut(slot as usize) else {
            return false;
        };
        *target = instance;
        true
    }

    /// 显式小端编码进 FNV-1a；`Option` 带存在标记避免哈希碰撞。
    pub fn hash(&self, hasher: &mut StateHasher) {
        for slot in &self.slots {
            match slot {
                Some(id) => {
                    hasher.write_u8(1);
                    hasher.write_u64(id.value());
                }
                None => hasher.write_u8(0),
            }
        }
    }
}

/// 物品 / 商店 / 回城事实种类（稳定整数编码，Q5 / R12.2）。
///
/// 事实本身是权威状态（进哈希 / 快照 / `first_difference()`）；只读
/// `Event` 不新增 payload，表现层按 `Event.entity` 查只读 `Snapshot` 表。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ItemShopFactKind {
    /// 拾取地面物品。
    Pickup,
    /// 商店购买。
    Purchase,
    /// 使用道具。
    Use,
    /// 丢弃道具回地面。
    Discard,
    /// 两英雄间移交。
    Transfer,
    /// 售卖道具。
    Sell,
    /// 回城施法开始。
    TownPortalStart,
    /// 回城传送完成。
    TownPortalComplete,
    /// 回城施法中断 / 拒单。
    TownPortalCancel,
    /// 市场刷新。
    MarketRefresh,
    /// 命令被显式拒绝（拒单 / 满包 / 售罄 / 黄金不足 / 非法目标）。
    Rejected,
}

impl ItemShopFactKind {
    /// 稳定的单字节编码。
    pub const fn code(self) -> u8 {
        match self {
            ItemShopFactKind::Pickup => 1,
            ItemShopFactKind::Purchase => 2,
            ItemShopFactKind::Use => 3,
            ItemShopFactKind::Discard => 4,
            ItemShopFactKind::Transfer => 5,
            ItemShopFactKind::Sell => 6,
            ItemShopFactKind::TownPortalStart => 7,
            ItemShopFactKind::TownPortalComplete => 8,
            ItemShopFactKind::TownPortalCancel => 9,
            ItemShopFactKind::MarketRefresh => 10,
            ItemShopFactKind::Rejected => 11,
        }
    }

    /// 从显式编码还原；非法值返回 `None`。
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(ItemShopFactKind::Pickup),
            2 => Some(ItemShopFactKind::Purchase),
            3 => Some(ItemShopFactKind::Use),
            4 => Some(ItemShopFactKind::Discard),
            5 => Some(ItemShopFactKind::Transfer),
            6 => Some(ItemShopFactKind::Sell),
            7 => Some(ItemShopFactKind::TownPortalStart),
            8 => Some(ItemShopFactKind::TownPortalComplete),
            9 => Some(ItemShopFactKind::TownPortalCancel),
            10 => Some(ItemShopFactKind::MarketRefresh),
            11 => Some(ItemShopFactKind::Rejected),
            _ => None,
        }
    }
}

/// 一条确定性事实记录（稳定 `seq` 键，按 `EntityId` 顺序登记）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemShopFact {
    pub seq: u64,
    pub frame: u64,
    pub kind: ItemShopFactKind,
    /// 主要实体（持包英雄 / 施法者）。
    pub hero: EntityId,
    /// 物品实例（无则 `EntityId::NONE`）。
    pub instance: EntityId,
    /// 参与槽位 `0..6`；`u8::MAX` 表示无槽位。
    pub slot: u8,
    /// 关联实体（地面物品 / 来源或目标英雄 / 友方主城 / 商店标识）。
    pub entity: EntityId,
    /// 精确黄金变动（扣费为负、入账为正，R9.3）。
    pub gold_delta: i64,
    /// 货架 / 商店 id（无则 0）。
    pub shelf: u32,
}

impl ItemShopFact {
    /// 显式小端编码进 FNV-1a。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.seq);
        hasher.write_u64(self.frame);
        hasher.write_u8(self.kind.code());
        hasher.write_u64(self.hero.value());
        hasher.write_u64(self.instance.value());
        hasher.write_u8(self.slot);
        hasher.write_u64(self.entity.value());
        hasher.write_i64(self.gold_delta);
        hasher.write_u32(self.shelf);
    }
}

/// 回城传送施法权威状态（施法帧 / 目标 / 次数，R7.1 / 结论 7）。
///
/// MUST NOT 与大法师 MassTeleport 群体传送混淆：单英雄、无读条 AoE、
/// 无多单位、`channelEndTick` 或 `areaOfEffect` 字段。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TownPortalCast {
    /// 施法英雄。
    pub caster: EntityId,
    /// 使用的卷轴实例。
    pub instance: EntityId,
    /// 卷轴所在槽位。
    pub slot: u8,
    /// 友方主城目标实体（定义层分类数据驱动）。
    pub target: EntityId,
    /// 目标整数坐标 x（Q47.16 raw）。
    pub target_x: i64,
    /// 目标整数坐标 y（Q47.16 raw）。
    pub target_y: i64,
    /// 开始施法的整数逻辑帧。
    pub start_frame: u64,
    /// 剩余施法帧（30 Hz 整数帧，MUST NOT 用墙钟 / `f64`）。
    pub remaining_frames: u32,
    /// 总施法帧。
    pub total_frames: u32,
}

impl TownPortalCast {
    /// 显式小端编码进 FNV-1a。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.caster.value());
        hasher.write_u64(self.instance.value());
        hasher.write_u8(self.slot);
        hasher.write_u64(self.target.value());
        hasher.write_i64(self.target_x);
        hasher.write_i64(self.target_y);
        hasher.write_u64(self.start_frame);
        hasher.write_u32(self.remaining_frames);
        hasher.write_u32(self.total_frames);
    }
}

/// 市场刷新随机码消费记录（复用 P8.5c 事件驱动流 + 显式消费序列，R8.6）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShopRefreshDraw {
    /// 与 `CreepCampState.next_rng_seq` 共用的全局消费序号。
    pub seq: u64,
    pub frame: u64,
    /// 商店 id（稳定 `EntityId` 顺序消费）。
    pub shop: u32,
    /// 刷新批次 id（幂等判据）。
    pub batch: u32,
    /// 同一 match-owned 整数 LCG 消费前状态。
    pub rng_before: u64,
    /// 同一 match-owned 整数 LCG 消费后状态。
    pub rng_after: u64,
    /// 本次抽取码。
    pub roll: u64,
    /// 命中货架（幂等补齐用）。
    pub picked: Option<DefId>,
}

impl ShopRefreshDraw {
    /// 显式小端编码进 FNV-1a。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.seq);
        hasher.write_u64(self.frame);
        hasher.write_u32(self.shop);
        hasher.write_u32(self.batch);
        hasher.write_u64(self.rng_before);
        hasher.write_u64(self.rng_after);
        hasher.write_u64(self.roll);
        match self.picked {
            Some(def) => {
                hasher.write_u8(1);
                hasher.write_u32(def);
            }
            None => hasher.write_u8(0),
        }
    }
}

/// 购买事务冻结扣费登记（失败回滚退款的精确逆运算依据，R9.3）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PurchaseCharge {
    pub owner: u8,
    pub gold: i64,
    pub hero: EntityId,
    /// 预分配的实例 id（提交时才真正建实例）。
    pub instance: EntityId,
    pub slot: u8,
    pub shelf: u32,
    pub shop: u32,
    pub frame: u64,
}

impl PurchaseCharge {
    /// 显式小端编码进 FNV-1a。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u8(self.owner);
        hasher.write_i64(self.gold);
        hasher.write_u64(self.hero.value());
        hasher.write_u64(self.instance.value());
        hasher.write_u8(self.slot);
        hasher.write_u32(self.shelf);
        hasher.write_u32(self.shop);
        hasher.write_u64(self.frame);
    }
}

/// P8.5d 物品 / 商店 / 回城卷轴全部权威状态的唯一顶层容器。
///
/// 全部落 `BTreeMap` / `BTreeSet` / 定长数组，按稳定 `EntityId` / `u32` /
/// `u64` 键序遍历；MUST 同时进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`
/// （当前 9）定义的快照与 `first_difference()`
/// （ADR 0002 §6-8 / `CODEBUDDY.md §5`）。
///
/// 随机码消费复用 P8.5c `CreepCampState.rng_state` / `rng_draws` /
/// `next_rng_seq` 事件驱动流（同一 match-owned 整数 LCG、`seed` 随快照），
/// 本结构只登记商店侧的显式消费序列与幂等批次；MUST NOT 用墙钟 /
/// `f64` / 表现层随机源 / `HashMap` 迭代顺序（Q6 / R8.6）。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ItemShopState {
    /// 每英雄六格定长实例背包（稳定 `EntityId`，R6.2）。
    pub bags: BTreeMap<EntityId, ItemBag>,
    /// 英雄阵亡时移入的背包快照，`complete_revive()` 按实例身份恢复（R6.3）。
    pub death_bags: BTreeMap<EntityId, ItemBag>,
    /// 拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖 / 回城 / 刷新事实（稳定 `seq`）。
    pub facts: BTreeMap<u64, ItemShopFact>,
    /// 下一个事实序号。
    pub next_fact_seq: u64,
    /// 商店库存：货架 id → 剩余数量（权威状态，R9.1 / R9.3）。
    pub stock: BTreeMap<u32, u32>,
    /// 商店刷新计数：商店 id → 累计刷新次数（权威状态）。
    pub refresh_counts: BTreeMap<u32, u32>,
    /// 确定性随机码消费序列（稳定全局 `seq`）。
    pub refresh_draws: BTreeMap<u64, ShopRefreshDraw>,
    /// 已处理刷新批次 `(shop, batch)`；同一批次不重复消费（幂等，R8.6）。
    /// 与帧号解耦，重复投递通知 MUST NOT 二次消耗随机码。
    pub refresh_batches: BTreeSet<(u32, u32)>,
    /// 待在帧阶段按稳定顺序处理的刷新请求 `(frame, shop, batch)`。
    pub pending_refreshes: BTreeSet<(u64, u32, u32)>,
    /// 回城施法状态（稳定施法者 `EntityId`）。
    pub casts: BTreeMap<EntityId, TownPortalCast>,
    /// 回城卷轴累计次数（稳定施法者 `EntityId`）。
    pub portal_uses: BTreeMap<EntityId, u32>,
    /// 友方主城分类登记：实体 → 冻结单位 DefId（数据驱动目标，结论 7）。
    pub town_halls: BTreeMap<EntityId, DefId>,
    /// 购买事务冻结扣费登记（稳定 `seq`）。
    pub charges: BTreeMap<u64, PurchaseCharge>,
    /// 下一个扣费登记序号。
    pub next_charge_seq: u64,
}

impl ItemShopState {
    /// 按冻结定义初始化商店库存（权威状态来自定义层，Q6 / Q8）。
    ///
    /// 只在创建对局时调用；库存运行态与冻结货架清单严格分离。
    pub fn initialize_stock(&mut self, defs: &ItemShopDefinitions) {
        for (&shelf_id, shelf) in &defs.shelves {
            self.stock.insert(shelf_id, shelf.stock_max);
        }
        for (&shop_id, _) in &defs.shops {
            self.refresh_counts.entry(shop_id).or_insert(0);
        }
    }

    /// 登记一条确定性事实；返回序号（供只读投影按序号查表）。
    pub fn push_fact(&mut self, fact: &mut ItemShopFact) -> u64 {
        let seq = self.next_fact_seq;
        self.next_fact_seq = seq.saturating_add(1);
        fact.seq = seq;
        self.facts.insert(seq, *fact);
        seq
    }

    /// 全部权威状态显式小端编码进 FNV-1a；按稳定键序遍历。
    /// 新增字段 MUST 同步接入本方法、`first_difference()` 与快照
    /// （`CODEBUDDY.md §5` 硬门禁）。
    pub fn hash(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.bags.len() as u64);
        for (&hero, bag) in &self.bags {
            hasher.write_u64(hero.value());
            bag.hash(hasher);
        }

        hasher.write_u64(self.death_bags.len() as u64);
        for (&hero, bag) in &self.death_bags {
            hasher.write_u64(hero.value());
            bag.hash(hasher);
        }

        hasher.write_u64(self.facts.len() as u64);
        for (&seq, fact) in &self.facts {
            hasher.write_u64(seq);
            fact.hash(hasher);
        }
        hasher.write_u64(self.next_fact_seq);

        hasher.write_u64(self.stock.len() as u64);
        for (&shelf_id, &remaining) in &self.stock {
            hasher.write_u32(shelf_id);
            hasher.write_u32(remaining);
        }

        hasher.write_u64(self.refresh_counts.len() as u64);
        for (&shop_id, &count) in &self.refresh_counts {
            hasher.write_u32(shop_id);
            hasher.write_u32(count);
        }

        hasher.write_u64(self.refresh_draws.len() as u64);
        for (&seq, draw) in &self.refresh_draws {
            hasher.write_u64(seq);
            draw.hash(hasher);
        }

        hasher.write_u64(self.refresh_batches.len() as u64);
        for &(shop, batch) in &self.refresh_batches {
            hasher.write_u32(shop);
            hasher.write_u32(batch);
        }

        hasher.write_u64(self.pending_refreshes.len() as u64);
        for &(frame, shop, batch) in &self.pending_refreshes {
            hasher.write_u64(frame);
            hasher.write_u32(shop);
            hasher.write_u32(batch);
        }

        hasher.write_u64(self.casts.len() as u64);
        for (&hero, cast) in &self.casts {
            hasher.write_u64(hero.value());
            cast.hash(hasher);
        }

        hasher.write_u64(self.portal_uses.len() as u64);
        for (&hero, &uses) in &self.portal_uses {
            hasher.write_u64(hero.value());
            hasher.write_u32(uses);
        }

        hasher.write_u64(self.town_halls.len() as u64);
        for (&entity, &def_id) in &self.town_halls {
            hasher.write_u64(entity.value());
            hasher.write_u32(def_id);
        }

        hasher.write_u64(self.charges.len() as u64);
        for (&seq, charge) in &self.charges {
            hasher.write_u64(seq);
            charge.hash(hasher);
        }
        hasher.write_u64(self.next_charge_seq);
    }

    /// 按同一稳定顺序返回首个差异字段名，便于 desync 诊断（ADR 0002 §6）。
    pub fn first_difference(&self, other: &Self) -> Option<&'static str> {
        if self.bags != other.bags {
            return Some("item_shop_bags");
        }
        if self.death_bags != other.death_bags {
            return Some("item_shop_death_bags");
        }
        if self.facts != other.facts {
            return Some("item_shop_facts");
        }
        if self.next_fact_seq != other.next_fact_seq {
            return Some("item_shop_next_fact_seq");
        }
        if self.stock != other.stock {
            return Some("item_shop_stock");
        }
        if self.refresh_counts != other.refresh_counts {
            return Some("item_shop_refresh_counts");
        }
        if self.refresh_draws != other.refresh_draws {
            return Some("item_shop_refresh_draws");
        }
        if self.refresh_batches != other.refresh_batches {
            return Some("item_shop_refresh_batches");
        }
        if self.pending_refreshes != other.pending_refreshes {
            return Some("item_shop_pending_refreshes");
        }
        if self.casts != other.casts {
            return Some("item_shop_casts");
        }
        if self.portal_uses != other.portal_uses {
            return Some("item_shop_portal_uses");
        }
        if self.town_halls != other.town_halls {
            return Some("item_shop_town_halls");
        }
        if self.charges != other.charges {
            return Some("item_shop_charges");
        }
        if self.next_charge_seq != other.next_charge_seq {
            return Some("item_shop_next_charge_seq");
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_bag_is_fixed_six_slots_and_rejects_overflow() {
        let mut bag = ItemBag::default();
        assert_eq!(bag.slots.len(), BAG_SLOTS);
        assert_eq!(bag.first_free_slot(), Some(0));
        for slot in 0..BAG_SLOTS as u8 {
            assert!(bag.set(slot, EntityId::new(u64::from(slot) + 1)));
        }
        assert!(bag.is_full());
        assert_eq!(bag.first_free_slot(), None);
        assert!(!bag.set(BAG_SLOTS as u8, EntityId::new(99)));
        assert_eq!(bag.slot_of(EntityId::new(3).unwrap()), Some(2));
    }

    #[test]
    fn category_and_fact_codes_are_stable_and_round_trip() {
        assert_eq!(ItemCategory::Consumable.code(), 1);
        assert_eq!(ItemCategory::Permanent.code(), 2);
        assert_eq!(ItemCategory::from_code(1), Some(ItemCategory::Consumable));
        assert_eq!(ItemCategory::from_code(3), None);
        assert_eq!(ItemShopFactKind::TownPortalComplete.code(), 8);
        assert_eq!(
            ItemShopFactKind::from_code(11),
            Some(ItemShopFactKind::Rejected)
        );
        assert_eq!(ItemShopFactKind::from_code(12), None);
    }

    #[test]
    fn frozen_definition_segments_all_change_content_hash() {
        let base = synthetic_item_shop_definitions();
        let hash = |defs: &ItemShopDefinitions| {
            let mut hasher = StateHasher::new();
            defs.hash(&mut hasher);
            hasher.finish()
        };
        let mut changed_price = base.clone();
        changed_price.shelves.get_mut(&101).unwrap().price += 1;
        assert_ne!(hash(&base), hash(&changed_price), "reason=price_segment");

        let mut changed_cast = base.clone();
        changed_cast.constants.town_portal_cast_frames += 1;
        assert_ne!(
            hash(&base),
            hash(&changed_cast),
            "reason=cast_frames_segment"
        );

        let mut changed_class = base.clone();
        changed_class.town_hall_defs.insert(9999);
        assert_ne!(hash(&base), hash(&changed_class), "reason=class_segment");

        let mut changed_effect = base.clone();
        changed_effect.item_effects.get_mut(&9001).unwrap().amount += 1;
        assert_ne!(hash(&base), hash(&changed_effect), "reason=effect_segment");
    }

    #[test]
    fn all_authoritative_groups_are_hashed() {
        let mut a = ItemShopState::default();
        let mut b = ItemShopState::default();
        b.bags.insert(EntityId::new(1).unwrap(), ItemBag::default());
        assert_ne!(a.hash_value(), b.hash_value());
        assert_eq!(a.first_difference(&b), Some("item_shop_bags"));

        a = ItemShopState::default();
        b = ItemShopState::default();
        b.stock.insert(101, 2);
        assert_ne!(a.hash_value(), b.hash_value());
        assert_eq!(a.first_difference(&b), Some("item_shop_stock"));

        a = ItemShopState::default();
        b = ItemShopState::default();
        b.refresh_counts.insert(7, 1);
        assert_ne!(a.hash_value(), b.hash_value());
        assert_eq!(a.first_difference(&b), Some("item_shop_refresh_counts"));

        a = ItemShopState::default();
        b = ItemShopState::default();
        b.portal_uses.insert(EntityId::new(1).unwrap(), 1);
        assert_ne!(a.hash_value(), b.hash_value());
        assert_eq!(a.first_difference(&b), Some("item_shop_portal_uses"));
    }

    impl ItemShopState {
        fn hash_value(&self) -> u64 {
            let mut hasher = StateHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }
}
