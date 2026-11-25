//! P8.5d 物品 / 商店 / 回城卷轴正向验收与确定性回归。
//!
//! 全部场景经 `w3-core` 公开 API 构造：`create_with_definitions` →
//! `capture_snapshot()` 注入只读状态 → `restore_snapshot()` →
//! `submit_gameplay_command` → `step()`。零文件 IO、零资产，数值全为
//! 合成原创编译期整数 `[待核]`（Q8 / §14 / ADR 0001 / ADR 0004）。
//!
//! 业务规则：R1.1 / R1.4 / R6.2 / R6.3 / R7.1–R7.6 / R8.6 / R9.1 /
//! R9.3 / R11.5 / R12.2 / R12.4；确定性硬门禁 ADR 0002 §6-8 / `CODEBUDDY.md §5`。

mod item_shop_common;

use item_shop_common::*;
use std::collections::BTreeSet;

use w3_core::{
    Command, CommandKind, EffectKind, EntityId, EventKind, Frame, FrozenDefinitions,
    GameplayCommand, GameplayCommandKind, HeroDeathRecord, HeroState, ItemBag, ItemShopFactKind,
    NEUTRAL_OWNER, NeutralCategory, ReviveCharge, ReviveQueueItem, RtsMatch, SIM_VERSION,
    SNAPSHOT_VERSION, SimFixed, SnapshotError, StaticWorld,
};

fn has_fact(sim: &RtsMatch, kind: ItemShopFactKind) -> bool {
    sim.item_shop_state()
        .facts
        .values()
        .any(|fact| fact.kind == kind)
}

fn has_event(sim: &mut RtsMatch, kind: EventKind) -> bool {
    sim.drain_events().iter().any(|event| event.kind == kind)
}

/// 六格满包（`0..6` 各挂一个独立实例 `EntityId`）。
fn full_bag() -> ItemBag {
    let mut bag = ItemBag::default();
    for slot in 0..6u8 {
        bag.set(slot, Some(bag_instance(slot)));
    }
    bag
}

// ---------------------------------------------------------------------------
// 1. 定义层：零资产 / 冻结常量 / 数据驱动（Q8 / tasks 1.x / R9.1）
// ---------------------------------------------------------------------------

#[test]
fn frozen_item_shop_defs_are_zero_asset_and_content_sensitive() {
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    assert_eq!(SIM_VERSION, 9, "reason=sim_version_9");
    let base = defs();

    // 干净环境构造成功、零文件 IO：只由编译期整数常量装配。
    assert!(base.item_shop().is_shop_def(SHOP));
    assert!(base.item_shop().is_town_hall_def(TOWN_HALL_DEF));
    assert!(base.item_shop().is_town_portal_item(ITEM_PORTAL));
    assert_eq!(
        base.item_shop().shelf(SHELF_CONSUMABLE).unwrap().price,
        PRICE_CONSUMABLE
    );

    // defs_hash / content_hash 进入快照头部。
    let (sim, _) = base_match(1000, true);
    let snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.defs_hash, base.content_hash());
    assert_eq!(snapshot.content_hash, base.content_hash());
    assert_eq!(snapshot.version, 9);
    assert_eq!(snapshot.sim_version, 9);

    // Q8：改任一货架 / 价格 / 库存 / 回城施法帧 / 类别常量即改变 content_hash()。
    let rebuild = |item_shop: w3_core::ItemShopDefinitions| {
        FrozenDefinitions::new(vec![], vec![], vec![], vec![])
            .unwrap()
            .with_item_shop(item_shop)
            .content_hash()
    };
    let base_hash = base.content_hash();

    let mut changed_price = base.item_shop().clone();
    changed_price
        .shelves
        .get_mut(&SHELF_CONSUMABLE)
        .unwrap()
        .price += 1;
    assert_ne!(base_hash, rebuild(changed_price), "reason=price_segment");

    let mut changed_cast = base.item_shop().clone();
    changed_cast.constants.town_portal_cast_frames += 1;
    assert_ne!(
        base_hash,
        rebuild(changed_cast),
        "reason=cast_frames_segment"
    );

    let mut changed_stock = base.item_shop().clone();
    changed_stock
        .shelves
        .get_mut(&SHELF_CONSUMABLE)
        .unwrap()
        .stock_max += 1;
    assert_ne!(base_hash, rebuild(changed_stock), "reason=stock_segment");

    let mut changed_class = base.item_shop().clone();
    changed_class.town_hall_defs.insert(9999);
    assert_ne!(base_hash, rebuild(changed_class), "reason=class_segment");

    let mut changed_ratio = base.item_shop().clone();
    changed_ratio.constants.sell_ratio_percent += 1;
    assert_ne!(base_hash, rebuild(changed_ratio), "reason=ratio_segment");
}

#[test]
fn non_classic_def_ids_are_definition_driven_not_a_name_whitelist() {
    // 非经典命名的商店 / 主城 / 道具 DefId 只要登记即可被判定（R1.2 / R13.3）。
    let mut custom = defs().item_shop().clone();
    custom.shop_defs.insert(4711);
    custom.town_hall_defs.insert(4712);
    custom.town_portal_items.insert(4713);
    custom
        .item_categories
        .insert(4713, w3_core::ItemCategory::Consumable);
    let frozen = FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_item_shop(custom);
    assert!(frozen.item_shop().is_shop_def(4711));
    assert!(frozen.item_shop().is_town_hall_def(4712));
    assert!(frozen.item_shop().is_town_portal_item(4713));
    assert!(!frozen.item_shop().is_shop_def(4712));
    assert!(!frozen.item_shop().is_town_hall_def(4711));
}

#[test]
fn frozen_constants_and_authoritative_state_are_distinct_layers() {
    // 货架清单 / 价格 / 库存上限 / 刷新间隔是冻结定义层（进 defs_hash）；
    // 库存 / 背包 / 传送 / 消费序列是权威状态（进 compute_state_hash）。
    let (sim, content) = base_match(1000, true);
    let base = sim.capture_snapshot();
    let mut changed = base.clone();
    changed.item_shop.stock.insert(SHELF_CONSUMABLE, 1);
    let changed_sim = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();
    assert_ne!(sim.compute_state_hash(), changed_sim.compute_state_hash());
    assert_eq!(sim.first_difference(&changed_sim), Some("item_shop_stock"));
    // 权威库存改动 MUST NOT 改变冻结定义内容哈希。
    assert_eq!(
        sim.capture_snapshot().defs_hash,
        changed_sim.capture_snapshot().defs_hash
    );
}

// ---------------------------------------------------------------------------
// 2. 六格定长实例背包与实例独立充能（R6.2 / R7.5 / tasks 2.1）
// ---------------------------------------------------------------------------

#[test]
fn six_slot_bag_is_fixed_length_and_instances_are_independent() {
    let (sim, content) = base_match(1000, true);
    let injected = with_state(&sim, content, |snapshot| {
        // 含多件同名卷轴：MUST 各有独立 EntityId / charges / cooldown。
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
    });
    let snapshot = injected.capture_snapshot();
    let bag = snapshot.item_shop.bags.get(&hero()).unwrap();
    assert_eq!(bag.slots.len(), 6, "reason=six_fixed_slots");
    assert!(bag.is_full(), "reason=bag_full");
    let ids: Vec<EntityId> = bag.slots.iter().flatten().copied().collect();
    assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), 6);
    let charges: Vec<u32> = ids
        .iter()
        .map(|id| snapshot.creep_camps.item_instances.get(id).unwrap().charges)
        .collect();
    assert!(
        charges.windows(2).any(|pair| pair[0] != pair[1]),
        "reason=independent_charges"
    );
    // 同名道具确有多实例（R7.5）。
    assert_eq!(
        ids.iter()
            .filter(|id| snapshot
                .creep_camps
                .item_instances
                .get(id)
                .unwrap()
                .item_def
                == ITEM_CONSUMABLE)
            .count(),
        6
    );
}

#[test]
fn full_bag_is_explicitly_rejected_not_silently_dropped() {
    let (sim, content) = base_match(1000, true);
    let mut full = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
    });
    let gold_before = gold(&full);
    let stock_before = full.item_shop_state().stock;
    let bag_before = *full.item_shop_state().bags.get(&hero()).unwrap();
    submit(
        &mut full,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut full, 1);
    assert_eq!(gold(&full), gold_before, "reason=full_bag_no_deduction");
    assert_eq!(
        full.item_shop_state().stock,
        stock_before,
        "reason=full_bag_no_stock_change"
    );
    assert_eq!(
        *full.item_shop_state().bags.get(&hero()).unwrap(),
        bag_before,
        "reason=full_bag_unchanged"
    );
    assert!(
        has_fact(&full, ItemShopFactKind::Rejected),
        "reason=explicit_reject"
    );
    assert!(has_event(&mut full, EventKind::ItemShopCommandRejected));
}

// ---------------------------------------------------------------------------
// 3. 拾取闭环（R1.1 / R7.2 / R7.5 / R13.6 / tasks 2.3）
// ---------------------------------------------------------------------------

#[test]
fn pickup_migrates_instance_identity_without_resetting_charges_or_cooldown() {
    let (sim, content) = base_match(1000, true);
    let mut pickup_sim = with_state(&sim, content, |snapshot| {
        inject_ground(snapshot, ground(), ITEM_DROPPED, 2, 7);
    });
    assert_eq!(
        pickup_sim
            .capture_snapshot()
            .creep_camps
            .item_instances
            .get(&ground())
            .unwrap()
            .charges,
        2,
        "reason=charges_consumed_once"
    );
    submit(
        &mut pickup_sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: ground(),
        },
    );
    step_n(&mut pickup_sim, 1);
    let after = pickup_sim.capture_snapshot();
    // 同一 EntityId 进入背包某格，ground_items / neutral 无主标记清除。
    assert!(!after.creep_camps.ground_items.contains_key(&ground()));
    assert!(!after.creep_camps.neutral.contains_key(&ground()));
    let bag = after.item_shop.bags.get(&hero()).unwrap();
    assert_eq!(
        bag.slots[0],
        Some(ground()),
        "reason=same_instance_entity_id"
    );
    let instance = after.creep_camps.item_instances.get(&ground()).unwrap();
    assert_eq!(instance.charges, 2, "reason=charges_not_reset");
    // 冷却按整数帧自然递减，MUST NOT 被拾取刷新。
    assert!(
        instance.cooldown_frames <= 7,
        "reason=cooldown_not_refreshed"
    );
    assert_eq!(instance.holder, Some(hero()));
    assert_eq!(instance.slot, Some(0));
    assert!(has_fact(&pickup_sim, ItemShopFactKind::Pickup));
    assert!(has_event(&mut pickup_sim, EventKind::ItemPickedUp));
}

#[test]
fn pickup_is_idempotent_and_ground_items_never_enter_move_generation() {
    let (sim, content) = base_match(1000, true);
    let mut pickup_sim = with_state(&sim, content, |snapshot| {
        inject_ground(snapshot, ground(), ITEM_DROPPED, 3, 0);
    });
    submit(
        &mut pickup_sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: ground(),
        },
    );
    step_n(&mut pickup_sim, 1);
    // 重复投递同一拾取：第二次被显式拒绝（幂等）。
    submit(
        &mut pickup_sim,
        2,
        PLAYER,
        1,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: ground(),
        },
    );
    step_n(&mut pickup_sim, 1);
    let state = pickup_sim.item_shop_state();
    assert_eq!(
        state
            .bags
            .get(&hero())
            .unwrap()
            .slots
            .iter()
            .flatten()
            .count(),
        1,
        "reason=only_one_pickup"
    );

    // 混合选中集合（英雄 / 地面物品 / 商店）对地面下达 Move：
    // 只有英雄生成 Move，地面物品 / 商店位置不变（R1.1 / R1.4 / R13.6）。
    let before = pickup_sim.capture_snapshot();
    let shop_fixture = EntityId::new(w3_core::shop_fixture_id(SHOP)).unwrap();
    for (index, entity) in [hero(), ground(), shop_fixture].iter().enumerate() {
        pickup_sim
            .submit_command(Command::new(
                Frame::new(3),
                PLAYER,
                index as u32 + 1,
                CommandKind::Move {
                    entity: *entity,
                    x: SimFixed::from_int(7).unwrap(),
                    y: SimFixed::from_int(7).unwrap(),
                    group: 0,
                    append: false,
                },
            ))
            .unwrap();
    }
    step_n(&mut pickup_sim, 1);
    let after = pickup_sim.capture_snapshot();
    assert_eq!(
        after.item_shop, before.item_shop,
        "reason=shop_and_bag_not_written_by_move"
    );
    assert_eq!(
        after.creep_camps.ground_items, before.creep_camps.ground_items,
        "reason=ground_items_fixed"
    );
    assert_eq!(
        after.creep_camps.neutral, before.creep_camps.neutral,
        "reason=neutral_registration_unchanged"
    );
}

// ---------------------------------------------------------------------------
// 4. 购买 / 售卖事务（R7.3 / R7.4 / R9.3 / tasks 2.4）
// ---------------------------------------------------------------------------

#[test]
fn purchase_transaction_three_conditions_and_exact_refund_on_failure() {
    // 黄金不足：显式拒绝、不扣费、不建实例、库存不变。
    let mut poor = base_match(PRICE_CONSUMABLE - 1, true).0;
    submit(
        &mut poor,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut poor, 1);
    assert_eq!(
        gold(&poor),
        PRICE_CONSUMABLE - 1,
        "reason=poor_no_deduction"
    );
    assert!(has_fact(&poor, ItemShopFactKind::Rejected));

    // 库存售罄：显式拒绝、不扣费、不建实例。
    let (sim, content) = base_match(1000, true);
    let mut sold_out = with_state(&sim, content, |snapshot| {
        snapshot.item_shop.stock.insert(SHELF_CONSUMABLE, 0);
    });
    submit(
        &mut sold_out,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut sold_out, 1);
    assert_eq!(gold(&sold_out), 1000, "reason=sold_out_no_deduction");
    assert_eq!(
        sold_out.item_shop_state().stock.get(&SHELF_CONSUMABLE),
        Some(&0)
    );

    // 成功路径：精确扣费 / 建独立实例 / 入六格背包 / 库存递减。
    let mut success = base_match(1000, true).0;
    let stock_before = success
        .item_shop_state()
        .stock
        .get(&SHELF_CONSUMABLE)
        .copied()
        .unwrap();
    submit(
        &mut success,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut success, 1);
    assert_eq!(
        gold(&success),
        1000 - PRICE_CONSUMABLE,
        "reason=exact_deduction"
    );
    assert_eq!(
        success.item_shop_state().stock.get(&SHELF_CONSUMABLE),
        Some(&(stock_before - 1)),
        "reason=stock_decrement"
    );
    let bag = success
        .item_shop_state()
        .bags
        .get(&hero())
        .copied()
        .unwrap();
    let instance = bag.slots[0].unwrap();
    let snapshot = success.capture_snapshot();
    let built = snapshot.creep_camps.item_instances.get(&instance).unwrap();
    assert_eq!(built.item_def, ITEM_CONSUMABLE);
    assert_eq!(built.holder, Some(hero()));
    assert_eq!(built.slot, Some(0));
    assert!(has_fact(&success, ItemShopFactKind::Purchase));
    assert!(has_event(&mut success, EventKind::ItemPurchased));
}

#[test]
fn sell_credits_exact_buyback_price_and_instance_cannot_be_sold_twice() {
    let (sim, content) = base_match(0, true);
    let mut sell_sim = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
    });
    submit(
        &mut sell_sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::SellItem {
            hero: hero(),
            instance: bag_instance(0),
            shop: SHOP,
        },
    );
    step_n(&mut sell_sim, 1);
    assert_eq!(
        gold(&sell_sim),
        SELL_CONSUMABLE,
        "reason=exact_buyback_credit"
    );
    let after = sell_sim.capture_snapshot();
    assert!(
        !after
            .creep_camps
            .item_instances
            .contains_key(&bag_instance(0))
    );
    assert_eq!(after.item_shop.bags.get(&hero()).unwrap().slots[0], None);
    // 重复售卖被显式拒绝、不重复入账。
    submit(
        &mut sell_sim,
        2,
        PLAYER,
        1,
        GameplayCommandKind::SellItem {
            hero: hero(),
            instance: bag_instance(0),
            shop: SHOP,
        },
    );
    step_n(&mut sell_sim, 1);
    assert_eq!(gold(&sell_sim), SELL_CONSUMABLE, "reason=no_double_credit");
    assert!(
        sell_sim.item_shop_state().facts.values().any(|fact| {
            fact.kind == ItemShopFactKind::Sell && fact.gold_delta == SELL_CONSUMABLE
        })
    );
    assert!(has_event(&mut sell_sim, EventKind::ItemSold));
}

#[test]
fn legacy_buyitem_placeholder_is_no_longer_a_bypass_write_path() {
    // R-1：旧 `BuyItem` 变体 / 码 5 保留，但显式拒单，MUST NOT
    // 不扣 gold / 不校验 / 不建实例就写背包（R9.3 / ADR 0005 §3）。
    let mut sim = base_match(1000, true).0;
    let before = sim.capture_snapshot();
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyItem {
            unit: hero(),
            item: ITEM_CONSUMABLE,
        },
    );
    step_n(&mut sim, 1);
    let after = sim.capture_snapshot();
    assert_eq!(
        after.gameplay.players.get(&PLAYER).unwrap().gold,
        before.gameplay.players.get(&PLAYER).unwrap().gold,
        "reason=no_bypass_gold"
    );
    assert!(
        after.item_shop.bags.is_empty(),
        "reason=no_bypass_bag_write"
    );
    assert!(
        after.creep_camps.item_instances.is_empty(),
        "reason=no_bypass_instance"
    );
    assert_eq!(
        after.item_shop.stock, before.item_shop.stock,
        "reason=no_bypass_stock"
    );
    assert!(
        sim.item_shop_state()
            .facts
            .values()
            .any(|fact| fact.kind == ItemShopFactKind::Rejected)
    );
    // 新事务购买入口仍然可用（证明没有把购买整体关掉）。
    submit(
        &mut sim,
        2,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut sim, 1);
    assert!(sim.item_shop_state().bags.get(&hero()).is_some());
}

// ---------------------------------------------------------------------------
// 5. 使用道具四路径 / 共享冷却 / 丢弃 / 移交（R7.5 / R7.6 / R6.2）
// ---------------------------------------------------------------------------

#[test]
fn use_item_four_paths_and_shared_cooldown_by_instance_group() {
    let (sim, content) = base_match(1000, true);

    // 合法路径：充能递减、共享冷却按整数帧置位、效果走既有 EffectDef 组合。
    let mut legal = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE, ITEM_CONSUMABLE]);
        for slot in 0..2u8 {
            snapshot
                .creep_camps
                .item_instances
                .get_mut(&bag_instance(slot))
                .unwrap()
                .charges = 3;
        }
    });
    submit(
        &mut legal,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::NONE,
        },
    );
    step_n(&mut legal, 1);
    let snapshot = legal.capture_snapshot();
    let used = snapshot
        .creep_camps
        .item_instances
        .get(&bag_instance(0))
        .unwrap();
    assert_eq!(used.charges, 2, "reason=charges_decremented");
    assert!(
        used.cooldown_frames > 0,
        "reason=cooldown_set_by_integer_frames"
    );
    assert_eq!(
        used.effect.kind,
        EffectKind::Heal,
        "reason=existing_effect_def_R7_6"
    );
    // 同组第二实例也进入共享冷却（整数帧，无 float / 墙钟）。
    assert!(
        snapshot
            .creep_camps
            .item_instances
            .get(&bag_instance(1))
            .unwrap()
            .cooldown_frames
            > 0
    );
    assert!(has_fact(&legal, ItemShopFactKind::Use));

    // 充能耗尽：显式拒绝、不扣次数。
    let mut exhausted = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        snapshot
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(0))
            .unwrap()
            .charges = 0;
    });
    submit(
        &mut exhausted,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::NONE,
        },
    );
    step_n(&mut exhausted, 1);
    assert_eq!(
        exhausted
            .capture_snapshot()
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .charges,
        0
    );

    // 冷却未到：显式拒绝、不扣次数。
    let mut cooling = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        snapshot
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(0))
            .unwrap()
            .cooldown_frames = 100;
    });
    submit(
        &mut cooling,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::NONE,
        },
    );
    step_n(&mut cooling, 1);
    assert_eq!(
        cooling
            .capture_snapshot()
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .charges,
        1
    );

    // 目标非法（不是存活实体）：显式拒绝、不扣次数。
    let mut invalid = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
    });
    submit(
        &mut invalid,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::new(999999).unwrap(),
        },
    );
    step_n(&mut invalid, 1);
    assert_eq!(
        invalid
            .capture_snapshot()
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .charges,
        1
    );
    assert!(has_event(&mut invalid, EventKind::ItemShopCommandRejected));
}

#[test]
fn drop_keeps_instance_identity_and_remaining_cooldown_then_pickup_again() {
    let (sim, content) = base_match(1000, true);
    let mut drop_sim = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        snapshot
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(0))
            .unwrap()
            .cooldown_frames = 9;
    });
    submit(
        &mut drop_sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::DropItem {
            hero: hero(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut drop_sim, 1);
    let after_drop = drop_sim.capture_snapshot();
    let ground_state = after_drop
        .creep_camps
        .ground_items
        .get(&bag_instance(0))
        .unwrap();
    assert_eq!(
        ground_state.instance,
        bag_instance(0),
        "reason=same_instance_id"
    );
    assert_eq!(
        after_drop.creep_camps.neutral.get(&bag_instance(0)),
        Some(&NeutralCategory::GroundItemOwnerless)
    );
    // 剩余冷却带入且按整数帧继续递减（转交也不能刷新）。
    assert!(
        after_drop
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .cooldown_frames
            <= 9
    );
    assert_eq!(
        after_drop.item_shop.bags.get(&hero()).unwrap().slots[0],
        None
    );
    assert!(has_fact(&drop_sim, ItemShopFactKind::Discard));

    // 重新拾取后仍保持身份与剩余冷却。
    let mut reclaimed = with_state(&drop_sim, content, |_| {});
    submit(
        &mut reclaimed,
        2,
        PLAYER,
        1,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: bag_instance(0),
        },
    );
    step_n(&mut reclaimed, 1);
    let after_pickup = reclaimed.capture_snapshot();
    assert_eq!(
        after_pickup.item_shop.bags.get(&hero()).unwrap().slots[0],
        Some(bag_instance(0))
    );
    assert!(has_event(&mut reclaimed, EventKind::ItemPickedUp));
}

#[test]
fn transfer_keeps_instance_identity_and_full_target_bag_is_rejected() {
    let (sim, content) = base_match(1000, true);
    let mut transfer_sim = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        snapshot
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(0))
            .unwrap()
            .cooldown_frames = 5;
    });
    submit(
        &mut transfer_sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::TransferItem {
            from_hero: hero(),
            to_hero: peer(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut transfer_sim, 1);
    let after = transfer_sim.capture_snapshot();
    assert_eq!(after.item_shop.bags.get(&hero()).unwrap().slots[0], None);
    assert_eq!(
        after.item_shop.bags.get(&peer()).unwrap().slots[0],
        Some(bag_instance(0))
    );
    let instance = after
        .creep_camps
        .item_instances
        .get(&bag_instance(0))
        .unwrap();
    assert_eq!(instance.holder, Some(peer()));
    assert_eq!(instance.slot, Some(0));
    assert!(
        instance.cooldown_frames <= 5,
        "reason=cooldown_not_refreshed"
    );
    assert!(has_fact(&transfer_sim, ItemShopFactKind::Transfer));

    // 目标满包：显式拒绝、源格不变（R6.2）。
    let (full_sim, full_content) = base_match(1000, true);
    let mut full = with_state(&full_sim, full_content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        let mut peer_bag = ItemBag::default();
        for slot in 0..6u8 {
            let id = EntityId::new(w3_core::GROUND_ITEM_ID_BASE + 200 + u64::from(slot)).unwrap();
            peer_bag.set(slot, Some(id));
        }
        snapshot.item_shop.bags.insert(peer(), peer_bag);
    });
    submit(
        &mut full,
        1,
        PLAYER,
        1,
        GameplayCommandKind::TransferItem {
            from_hero: hero(),
            to_hero: peer(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut full, 1);
    let after_full = full.item_shop_state();
    assert_eq!(
        after_full.bags.get(&hero()).unwrap().slots[0],
        Some(bag_instance(0)),
        "reason=source_unchanged"
    );
    assert!(has_fact(&full, ItemShopFactKind::Rejected));
}

// ---------------------------------------------------------------------------
// 6. 回城卷轴三路径（R7.1 / R7.5 / 结论 7）
// ---------------------------------------------------------------------------

fn portal_match(with_town_hall: bool) -> RtsMatch {
    let (sim, content) = base_match(1000, with_town_hall);
    with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_PORTAL]);
    })
}

#[test]
fn town_portal_with_friendly_town_hall_teleports_to_integer_coordinates() {
    let mut sim = portal_match(true);
    let before = sim.capture_snapshot();
    let hero_before = before
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert_ne!(hero_before.x, 5 * w3_core::SCALE);
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut sim, 1);
    // 施法状态（施法帧 / 目标 / 次数）进权威状态。
    let casting = sim.item_shop_state();
    assert!(
        casting.casts.contains_key(&hero()),
        "reason=cast_is_authoritative"
    );
    assert_eq!(casting.portal_uses.get(&hero()), Some(&1));
    assert_eq!(
        casting.casts.get(&hero()).unwrap().target,
        town_hall(),
        "reason=stable_town_hall_target"
    );
    assert!(has_fact(&sim, ItemShopFactKind::TownPortalStart));
    step_n(&mut sim, 3);
    let after = sim.capture_snapshot();
    let hero_after = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert_eq!(
        hero_after.x,
        5 * w3_core::SCALE,
        "reason=teleported_to_integer_x"
    );
    assert_eq!(
        hero_after.y,
        5 * w3_core::SCALE,
        "reason=teleported_to_integer_y"
    );
    assert!(
        !after.item_shop.casts.contains_key(&hero()),
        "reason=cast_cleaned"
    );
    assert!(has_fact(&sim, ItemShopFactKind::TownPortalComplete));
    assert!(has_event(&mut sim, EventKind::TownPortalCompleted));
}

#[test]
fn town_portal_without_friendly_town_hall_rejects_without_consuming_charges() {
    let mut sim = portal_match(false);
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut sim, 1);
    let after = sim.capture_snapshot();
    assert!(
        !after.item_shop.casts.contains_key(&hero()),
        "reason=no_half_cast"
    );
    assert_eq!(
        after.item_shop.portal_uses.get(&hero()),
        None,
        "reason=no_use_consumed"
    );
    assert_eq!(
        after
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .charges,
        1,
        "reason=charge_not_consumed"
    );
    let hero_after = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert_eq!(hero_after.x, SHOP_X, "reason=no_half_teleport");
    assert!(
        sim.item_shop_state()
            .facts
            .values()
            .any(|fact| fact.kind == ItemShopFactKind::TownPortalCancel)
    );
    assert!(has_event(&mut sim, EventKind::TownPortalCancelled));
}

#[test]
fn town_portal_cast_interruption_settles_without_half_teleport() {
    let mut sim = portal_match(true);
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut sim, 1);
    assert!(sim.item_shop_state().casts.contains_key(&hero()));
    // 显式打断 → 取消并结算，不产生半传送位置（R7.5）。
    submit(
        &mut sim,
        2,
        PLAYER,
        1,
        GameplayCommandKind::CancelTownPortal { hero: hero() },
    );
    step_n(&mut sim, 1);
    let after = sim.capture_snapshot();
    assert!(
        !after.item_shop.casts.contains_key(&hero()),
        "reason=cast_cleaned"
    );
    let hero_after = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert_eq!(hero_after.x, SHOP_X, "reason=no_half_teleport");
    assert!(
        sim.item_shop_state()
            .facts
            .values()
            .any(|fact| fact.kind == ItemShopFactKind::TownPortalCancel)
    );
    assert!(has_event(&mut sim, EventKind::TownPortalCancelled));
}

#[test]
fn town_portal_is_single_hero_not_mass_teleport() {
    // 结论 7：单英雄卷轴，MUST NOT 带 channelEndTick / areaOfEffect / 多单位。
    let mut sim = portal_match(true);
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
    );
    step_n(&mut sim, 4);
    let after = sim.capture_snapshot();
    let peer_after = after
        .positions
        .iter()
        .find(|position| position.entity == PEER_ID)
        .unwrap();
    assert_eq!(peer_after.x, SHOP_X, "reason=peer_not_teleported");
    let hero_after = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert_eq!(hero_after.x, 5 * w3_core::SCALE);
}

// ---------------------------------------------------------------------------
// 7. 市场刷新 / 消费序列 / 确定性（R8.6 / Q6 / tasks 2.7）
// ---------------------------------------------------------------------------

#[test]
fn market_refresh_is_idempotent_and_consumption_sequence_is_seed_deterministic() {
    let (sim, content) = base_match(1000, true);
    let build = || {
        let mut a = with_state(&sim, content, |snapshot| {
            snapshot.item_shop.stock.insert(SHELF_CONSUMABLE, 0);
        });
        // 同帧多个批次 + 同一批次重复投递通知。
        a.submit_gameplay_command(refresh(1, 2, 42)).unwrap();
        a.submit_gameplay_command(refresh(1, 1, 41)).unwrap();
        a.submit_gameplay_command(refresh(1, 3, 42)).unwrap();
        step_n(&mut a, 1);
        a
    };
    let mut a = build();
    let b = build();
    let sa = a.capture_snapshot();
    let sb = b.capture_snapshot();
    // 幂等：两个不同批次各消费一次，重复投递不重复消费。
    assert_eq!(
        sa.item_shop.refresh_draws.len(),
        2,
        "reason=two_batches_consumed_once"
    );
    assert_eq!(sa.item_shop.refresh_batches.len(), 2);
    assert_eq!(sa.item_shop.refresh_counts.get(&SHOP), Some(&2));
    // 消费序列 / 随机流状态 / 刷新清单逐帧一致。
    assert_eq!(sa.item_shop.refresh_draws, sb.item_shop.refresh_draws);
    assert_eq!(sa.creep_camps.rng_draws, sb.creep_camps.rng_draws);
    assert_eq!(sa.creep_camps.rng_state, sb.creep_camps.rng_state);
    assert_eq!(sa.creep_camps.next_rng_seq, sb.creep_camps.next_rng_seq);
    assert_eq!(sa.item_shop.stock, sb.item_shop.stock);
    assert_eq!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), None);
    assert!(has_event(&mut a, EventKind::MarketRefreshed));

    // 同批次再投递一次：不重复消费、不改变消费序列。
    let before = a.item_shop_state().refresh_draws.len();
    a.submit_gameplay_command(refresh(2, 4, 41)).unwrap();
    step_n(&mut a, 1);
    assert_eq!(a.item_shop_state().refresh_draws.len(), before);
}

#[test]
fn all_new_authoritative_fields_are_hashed_and_first_difference_locates_them() {
    let (sim, content) = base_match(1000, true);
    let mut prepared = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]);
        snapshot
            .item_shop
            .death_bags
            .insert(hero(), ItemBag::default());
    });
    submit(
        &mut prepared,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_PORTAL,
        },
    );
    step_n(&mut prepared, 1);
    prepared.submit_gameplay_command(refresh(2, 1, 91)).unwrap();
    step_n(&mut prepared, 1);
    let base = prepared.capture_snapshot();

    struct Case {
        field: &'static str,
        mutate: Box<dyn Fn(&mut w3_core::Snapshot)>,
    }
    let cases: Vec<Case> = vec![
        Case {
            field: "item_shop_bags",
            mutate: Box::new(|s| {
                s.item_shop.bags.clear();
            }),
        },
        Case {
            field: "item_shop_death_bags",
            mutate: Box::new(|s| {
                s.item_shop.death_bags.clear();
            }),
        },
        Case {
            field: "item_shop_facts",
            mutate: Box::new(|s| {
                s.item_shop.facts.clear();
            }),
        },
        Case {
            field: "item_shop_next_fact_seq",
            mutate: Box::new(|s| {
                s.item_shop.next_fact_seq += 1;
            }),
        },
        Case {
            field: "item_shop_stock",
            mutate: Box::new(|s| {
                s.item_shop.stock.insert(SHELF_CONSUMABLE, 99);
            }),
        },
        Case {
            field: "item_shop_refresh_counts",
            mutate: Box::new(|s| {
                s.item_shop.refresh_counts.insert(SHOP, 99);
            }),
        },
        Case {
            field: "item_shop_refresh_draws",
            mutate: Box::new(|s| {
                s.item_shop.refresh_draws.clear();
            }),
        },
        Case {
            field: "item_shop_refresh_batches",
            mutate: Box::new(|s| {
                s.item_shop.refresh_batches.clear();
            }),
        },
        Case {
            field: "item_shop_pending_refreshes",
            mutate: Box::new(|s| {
                s.item_shop.pending_refreshes.insert((99, SHOP, 99));
            }),
        },
        Case {
            field: "item_shop_portal_uses",
            mutate: Box::new(|s| {
                s.item_shop.portal_uses.insert(hero(), 9);
            }),
        },
        Case {
            field: "item_shop_town_halls",
            mutate: Box::new(|s| {
                s.item_shop.town_halls.clear();
            }),
        },
        Case {
            field: "item_shop_next_charge_seq",
            mutate: Box::new(|s| {
                s.item_shop.next_charge_seq += 1;
            }),
        },
        Case {
            field: "creep_item_holders",
            mutate: Box::new(|s| {
                if let Some(instance) = s.creep_camps.item_instances.values_mut().next() {
                    instance.holder = Some(peer());
                }
            }),
        },
        Case {
            field: "creep_item_instances",
            mutate: Box::new(|s| {
                if let Some(instance) = s.creep_camps.item_instances.values_mut().next() {
                    instance.charges += 1;
                }
            }),
        },
    ];
    for case in cases {
        let mut changed = base.clone();
        (case.mutate)(&mut changed);
        let restored =
            RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();
        assert_ne!(
            prepared.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=hashed_{}",
            case.field
        );
        assert_eq!(
            prepared.first_difference(&restored),
            Some(case.field),
            "reason=first_difference_locates_{}",
            case.field
        );
    }
}

// ---------------------------------------------------------------------------
// 8. 双实例 / 快照往返 / 回放 / 旧版本拒绝 / 复活恢复 / 重开无残留
// ---------------------------------------------------------------------------

#[test]
fn double_instance_interleaved_and_snapshot_roundtrip_match_frame_by_frame() {
    let (base, content) = base_match(1000, true);
    let prepared = with_state(&base, content, |snapshot| {
        inject_ground(snapshot, ground(), ITEM_DROPPED, 3, 0);
    });
    let build = || {
        let mut sim = RtsMatch::restore_snapshot(
            prepared.capture_snapshot(),
            StaticWorld::default(),
            content,
        )
        .unwrap();
        for command in script() {
            sim.submit_gameplay_command(command).unwrap();
        }
        sim
    };
    let mut a = build();
    let mut b = build();
    let mut hashes = Vec::new();
    for frame in 0..10 {
        if frame % 2 == 0 {
            a.step();
            b.step();
        } else {
            b.step();
            a.step();
        }
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=frame_{frame}"
        );
        assert_eq!(a.first_difference(&b), None);
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=snapshot_{frame}"
        );
        hashes.push(a.compute_state_hash());
    }
    assert!(
        hashes.windows(2).any(|pair| pair[0] != pair[1]),
        "reason=state_advanced"
    );

    // 帧 N 捕获 / 销毁 / 恢复后继续到帧 M 与未中断实例一致。
    let snapshot = a.capture_snapshot();
    let mut restored =
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    for _ in 0..5 {
        a.step();
        restored.step();
        assert_eq!(a.compute_state_hash(), restored.compute_state_hash());
        assert_eq!(a.capture_snapshot(), restored.capture_snapshot());
        assert_eq!(restored.first_difference(&a), None);
    }
}

#[test]
fn command_script_is_frame_frozen_and_replay_does_not_double_inject() {
    // 同一确定性脚本重放：逐帧哈希序列一致、重放不双重注入（R12.2 / R12.4）。
    let (base, content) = base_match(1000, true);
    let prepared = with_state(&base, content, |snapshot| {
        inject_ground(snapshot, ground(), ITEM_DROPPED, 3, 0);
    });
    let mut original =
        RtsMatch::restore_snapshot(prepared.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    let mut replay =
        RtsMatch::restore_snapshot(prepared.capture_snapshot(), StaticWorld::default(), content)
            .unwrap();
    for command in script() {
        // GameplayCommand 是 Copy，两端注入逐字段相同的脚本。
        original.submit_gameplay_command(command).unwrap();
        replay.submit_gameplay_command(command).unwrap();
    }
    let expected: Vec<u64> = (0..10)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.compute_state_hash(),
            *hash,
            "reason=replay_frame_{frame}"
        );
    }
    assert_eq!(original.first_difference(&replay), None);
    // 刷新批次仍只消费一次（重放不双重消费）。
    assert_eq!(replay.item_shop_state().refresh_draws.len(), 1);
}

#[test]
fn old_v6_snapshot_and_sim_version_are_explicitly_rejected() {
    let (sim, content) = base_match(1000, true);
    let mut old = sim.capture_snapshot();
    old.version = 6;
    assert!(matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 6 })
    ));
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 6;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 6,
            expected: 9
        })
    ));
}

#[test]
fn revive_restores_six_slot_instance_bag_by_instance_identity() {
    // R6.3 / tasks 2.8：六格实例背包按实例 `EntityId` 与剩余充能 / 冷却
    // 逐项恢复，MUST NOT 回退为 `bag: Vec<DefId>` 的 DefId 序列。
    let (sim, content) = base_match(1000, true);
    let mut dead = with_state(&sim, content, |snapshot| {
        // 英雄阵亡、登记死亡快照、六格背包移入 death_bags。
        let position = snapshot
            .positions
            .iter_mut()
            .find(|position| position.entity == HERO_ID)
            .unwrap();
        position.alive = false;
        position.hp = 0;
        position.gameplay.hero = Some(HeroState { level: 3, xp: 42 });
        let hero_gameplay = position.gameplay.clone();
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        let bag = *snapshot.item_shop.bags.get(&hero()).unwrap();
        snapshot.item_shop.bags.remove(&hero());
        snapshot.item_shop.death_bags.insert(hero(), bag);
        for slot in 0..6u8 {
            let instance = snapshot
                .creep_camps
                .item_instances
                .get_mut(&bag_instance(slot))
                .unwrap();
            instance.holder = None;
            instance.slot = None;
        }
        snapshot.revive.deaths.insert(
            hero(),
            HeroDeathRecord {
                death_frame: 0,
                owner: PLAYER,
                gameplay: hero_gameplay,
                max_hp: 100,
            },
        );
        // 复活队列项 + 冻结扣费登记，走既有 advance_revive_queue 生产路径。
        snapshot.revive.queue.insert(
            hero(),
            ReviveQueueItem {
                hero: hero(),
                altar: town_hall(),
                altar_def: TOWN_HALL_DEF,
                owner: PLAYER,
                remaining_frames: 1,
                total_frames: 1,
                gold: 0,
                lumber: 0,
            },
        );
        snapshot.revive.charges.insert(
            hero(),
            ReviveCharge {
                owner: PLAYER,
                gold: 0,
                lumber: 0,
            },
        );
    });
    let before = dead.capture_snapshot();
    assert!(before.item_shop.death_bags.contains_key(&hero()));
    assert!(!before.item_shop.bags.contains_key(&hero()));
    step_n(&mut dead, 1);
    let after = dead.capture_snapshot();
    let position = after
        .positions
        .iter()
        .find(|position| position.entity == HERO_ID)
        .unwrap();
    assert!(position.alive, "reason=hero_revived");
    let bag = after
        .item_shop
        .bags
        .get(&hero())
        .expect("reason=bag_restored");
    for (slot, expected) in (0..6u8).map(bag_instance).enumerate() {
        assert_eq!(
            bag.slots[slot],
            Some(expected),
            "reason=instance_identity_{slot}"
        );
        let instance = after.creep_camps.item_instances.get(&expected).unwrap();
        assert_eq!(instance.item_def, ITEM_CONSUMABLE);
        assert_eq!(instance.holder, Some(hero()));
        assert_eq!(instance.slot, Some(slot as u8));
        assert_eq!(
            instance.charges,
            slot as u32 + 1,
            "reason=charges_preserved_{slot}"
        );
    }
    assert!(!after.item_shop.death_bags.contains_key(&hero()));
    assert!(has_event(&mut dead, EventKind::Revived));
}

#[test]
fn restart_and_snapshot_restore_have_no_item_shop_residue() {
    let (sim, content) = base_match(1000, true);
    let mut used = with_state(&sim, content, |snapshot| {
        inject_ground(snapshot, ground(), ITEM_DROPPED, 3, 0);
    });
    for command in script() {
        used.submit_gameplay_command(command).unwrap();
    }
    step_n(&mut used, 8);
    assert!(!used.item_shop_state().bags.is_empty());

    // 新一局：无残留背包 / 传送 / 消费序列 / 事实，胜负未结算。
    let fresh = RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
    let clean = fresh.capture_snapshot();
    assert!(clean.item_shop.bags.is_empty());
    assert!(clean.item_shop.death_bags.is_empty());
    assert!(clean.item_shop.casts.is_empty());
    assert!(clean.item_shop.facts.is_empty());
    assert!(clean.item_shop.refresh_draws.is_empty());
    assert!(clean.item_shop.refresh_batches.is_empty());
    assert!(clean.item_shop.pending_refreshes.is_empty());
    assert!(clean.item_shop.charges.is_empty());
    assert_eq!(clean.item_shop.next_fact_seq, 0);
    assert_eq!(clean.item_shop.next_charge_seq, 0);
    assert!(
        !clean.gameplay.victory.resolved,
        "reason=victory_not_resolved"
    );

    // 快照恢复不残留 / 不重复购买或刷新。
    let snapshot = used.capture_snapshot();
    let restored =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(snapshot.item_shop, restored.capture_snapshot().item_shop);
    assert_eq!(used.compute_state_hash(), restored.compute_state_hash());
}

#[test]
fn new_command_kinds_have_unique_encodings_and_are_exhaustively_dispatched() {
    // Q5：新增变体码 15–23 唯一编码，既有 1–14 不动；校验 / 穷尽分发同步。
    let kinds = vec![
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: ground(),
        },
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::NONE,
        },
        GameplayCommandKind::DropItem {
            hero: hero(),
            instance: bag_instance(0),
        },
        GameplayCommandKind::TransferItem {
            from_hero: hero(),
            to_hero: peer(),
            instance: bag_instance(0),
        },
        GameplayCommandKind::SellItem {
            hero: hero(),
            instance: bag_instance(0),
            shop: SHOP,
        },
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
        GameplayCommandKind::CancelTownPortal { hero: hero() },
        GameplayCommandKind::RefreshMarket {
            shop: SHOP,
            batch: 1,
        },
    ];
    let mut hashes = BTreeSet::new();
    let (sim, content) = base_match(1000, true);
    for (index, kind) in kinds.iter().enumerate() {
        let mut queued = with_state(&sim, content, |_| {});
        queued
            .submit_gameplay_command(GameplayCommand::new(Frame::new(1), PLAYER, 1, *kind))
            .unwrap();
        assert!(
            hashes.insert(queued.compute_state_hash()),
            "reason=unique_encoding_{index}"
        );
        // 穷尽分发：每个变体都能被 step() 消费而不 panic。
        step_n(&mut queued, 1);
        assert!(
            queued
                .capture_snapshot()
                .gameplay
                .commands
                .pending()
                .is_empty()
        );
    }
    assert_eq!(hashes.len(), kinds.len());
    // 同源校验：0 号商店 id / 0 号实体被公开提交入口明确拒绝。
    let mut invalid = with_state(&sim, content, |_| {});
    assert!(
        invalid
            .submit_gameplay_command(GameplayCommand::new(
                Frame::new(1),
                PLAYER,
                1,
                GameplayCommandKind::RefreshMarket { shop: 0, batch: 1 },
            ))
            .is_err()
    );
    assert!(
        invalid
            .submit_gameplay_command(GameplayCommand::new(
                Frame::new(1),
                NEUTRAL_OWNER,
                1,
                GameplayCommandKind::PickupGroundItem {
                    hero: EntityId::NONE,
                    ground_item: ground(),
                },
            ))
            .is_err()
    );
    assert!(full_bag().is_full());
}
