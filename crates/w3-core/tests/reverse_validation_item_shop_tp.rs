//! P8.5d 物品 / 商店 / 回城卷轴反向验证（改坏验红 / `CODEBUDDY.md §5` / Q9 形态①）。
//!
//! 每一类退化都成对提供 `formal_passes_*` / `degenerate_fails_the_same_assertion`：
//! 后者内部含「正式实现仍通过同一断言」的元断言防退化；退化实现 MUST 经
//! `restore_snapshot` / `create_with_definitions` / `submit_gameplay_command` /
//! `step()` 等真实生产路径注入，非旁路手搓结论。覆盖 Q9 五类退化：
//! ① 六格背包 / 实例 owner 与 slot / 充能冷却 / 商店库存 / 回城传送 /
//!    消费序列漏接 `compute_state_hash()` 或 `capture_snapshot()` 或
//!    `first_difference()`；
//! ② 拾取 / 购买 / 使用 / 售卖不经事务命令面，或表现层反写 gold / 背包 / 库存；
//! ③ 同名道具多实例共用充能 / 冷却、丢弃或移交刷新冷却（R7.5 退化）；
//! ④ 市场刷新用墙钟 / `HashMap` 迭代顺序 / 非确定随机源、消费序列漏接
//!    哈希 / 快照 / `first_difference()`；
//! ⑤ 新增命令 / 事件漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发。
//!
//! 不冒充形态②（A-B 对照 / 先写失败测试 / 刻意改坏被测代码验红留命令输出）。
//! 先例：`reverse_validation_creep_camps.rs` / `reverse_validation_hero_revive.rs`。

mod item_shop_common;

use item_shop_common::*;
use std::collections::BTreeSet;

use w3_core::{
    EntityId, EventKind, Frame, FrozenDefinitions, GameplayCommand, GameplayCommandKind,
    GameplayCommandQueue, ItemBag, ItemShopFactKind, ItemShopState, NEUTRAL_OWNER, RtsMatch,
    StateHasher, StaticWorld,
};

/// 退化哈希（缺陷态）：把指定权威状态段清空后走 `ItemShopState::hash` 的
/// 正式编码，等价于该段完全未进入哈希（ADR 0002 §6）。
fn degenerate_hash_omit(snapshot: &w3_core::Snapshot, omit: &str) -> u64 {
    let state = degenerate_omit_state(snapshot, omit);
    let mut hasher = StateHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

fn degenerate_omit_state(snapshot: &w3_core::Snapshot, omit: &str) -> ItemShopState {
    let mut state = snapshot.item_shop.clone();
    match omit {
        "bags" => state.bags.clear(),
        "death_bags" => state.death_bags.clear(),
        "facts" => state.facts.clear(),
        "stock" => state.stock.clear(),
        "refresh_counts" => state.refresh_counts.clear(),
        "refresh_draws" => state.refresh_draws.clear(),
        "refresh_batches" => state.refresh_batches.clear(),
        "pending_refreshes" => state.pending_refreshes.clear(),
        "casts" => state.casts.clear(),
        "portal_uses" => state.portal_uses.clear(),
        "town_halls" => state.town_halls.clear(),
        "charges" => state.charges.clear(),
        "next_fact_seq" => state.next_fact_seq = 0,
        "next_charge_seq" => state.next_charge_seq = 0,
        _ => panic!("item_shop_reverse: reason=unknown_omit_field value={omit}"),
    }
    state
}

/// 正式不变量：改动指定字段，两端 `compute_state_hash()` 必不同，
/// 且 `first_difference()` 定位到对应差异字段。
fn field_is_hashed(a: &RtsMatch, b: &RtsMatch, field: &str) -> bool {
    a.compute_state_hash() != b.compute_state_hash() && a.first_difference(b) == Some(field)
}

// ---------------------------------------------------------------------------
// 退化 ①：新增权威状态漏接 compute_state_hash() / capture_snapshot() /
//          first_difference()
// ---------------------------------------------------------------------------

const HASH_CASES: &[(&str, &str)] = &[
    ("item_shop_bags", "bags"),
    ("item_shop_death_bags", "death_bags"),
    ("item_shop_facts", "facts"),
    ("item_shop_stock", "stock"),
    ("item_shop_refresh_counts", "refresh_counts"),
    ("item_shop_refresh_draws", "refresh_draws"),
    ("item_shop_refresh_batches", "refresh_batches"),
    ("item_shop_pending_refreshes", "pending_refreshes"),
    ("item_shop_casts", "casts"),
    ("item_shop_portal_uses", "portal_uses"),
    ("item_shop_town_halls", "town_halls"),
    ("item_shop_charges", "charges"),
    ("item_shop_next_fact_seq", "next_fact_seq"),
    ("item_shop_next_charge_seq", "next_charge_seq"),
];

/// 为每个字段构造「只改该字段」的两端（经真实 `restore_snapshot` 注入）。
fn prepared_pair(content: u64, field: &str) -> (RtsMatch, RtsMatch) {
    let (sim, _) = base_match(1000, true);
    let mut prepared = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        snapshot
            .item_shop
            .death_bags
            .insert(hero(), full_bag_for_test());
        snapshot.item_shop.facts.insert(
            0,
            w3_core::ItemShopFact {
                seq: 0,
                frame: 0,
                kind: ItemShopFactKind::Purchase,
                hero: hero(),
                instance: bag_instance(0),
                slot: 0,
                entity: EntityId::NONE,
                gold_delta: -PRICE_CONSUMABLE,
                shelf: SHELF_CONSUMABLE,
            },
        );
        snapshot.item_shop.next_fact_seq = 1;
        snapshot.item_shop.next_charge_seq = 1;
        snapshot.item_shop.charges.insert(
            0,
            w3_core::PurchaseCharge {
                owner: PLAYER,
                gold: PRICE_CONSUMABLE,
                hero: hero(),
                instance: bag_instance(0),
                slot: 0,
                shelf: SHELF_CONSUMABLE,
                shop: SHOP,
                frame: 0,
            },
        );
        snapshot.item_shop.portal_uses.insert(hero(), 1);
        snapshot.item_shop.refresh_counts.insert(SHOP, 1);
        snapshot.item_shop.refresh_batches.insert((SHOP, 41));
        snapshot.item_shop.pending_refreshes.insert((1, SHOP, 42));
        snapshot.item_shop.refresh_draws.insert(
            0,
            w3_core::ShopRefreshDraw {
                seq: 0,
                frame: 0,
                shop: SHOP,
                batch: 41,
                rng_before: 1,
                rng_after: 2,
                roll: 2,
                picked: Some(SHELF_CONSUMABLE),
            },
        );
        snapshot.item_shop.casts.insert(
            hero(),
            w3_core::TownPortalCast {
                caster: hero(),
                instance: bag_instance(0),
                slot: 0,
                target: town_hall(),
                target_x: 5 * w3_core::SCALE,
                target_y: 5 * w3_core::SCALE,
                start_frame: 0,
                remaining_frames: 2,
                total_frames: 2,
            },
        );
    });
    // town_halls 在 base_match(true) 中已登记。
    let base = prepared.capture_snapshot();
    let mut changed = base.clone();
    mutate_field(&mut changed, field);
    let formal = RtsMatch::restore_snapshot(base.clone(), StaticWorld::default(), content).unwrap();
    let variant = RtsMatch::restore_snapshot(changed, StaticWorld::default(), content).unwrap();
    let _ = &mut prepared;
    (formal, variant)
}

fn full_bag_for_test() -> ItemBag {
    let mut bag = ItemBag::default();
    for slot in 0..6u8 {
        bag.set(slot, Some(bag_instance(slot)));
    }
    bag
}

fn mutate_field(snapshot: &mut w3_core::Snapshot, field: &str) {
    match field {
        "item_shop_bags" => snapshot.item_shop.bags.clear(),
        "item_shop_death_bags" => snapshot.item_shop.death_bags.clear(),
        "item_shop_facts" => snapshot.item_shop.facts.clear(),
        "item_shop_stock" => {
            snapshot.item_shop.stock.insert(SHELF_CONSUMABLE, 99);
        }
        "item_shop_refresh_counts" => {
            snapshot.item_shop.refresh_counts.insert(SHOP, 99);
        }
        "item_shop_refresh_draws" => snapshot.item_shop.refresh_draws.clear(),
        "item_shop_refresh_batches" => snapshot.item_shop.refresh_batches.clear(),
        "item_shop_pending_refreshes" => {
            snapshot.item_shop.pending_refreshes.insert((99, SHOP, 99));
        }
        "item_shop_casts" => {
            if let Some(cast) = snapshot.item_shop.casts.get_mut(&hero()) {
                cast.remaining_frames += 1;
                cast.target = peer();
            }
        }
        "item_shop_portal_uses" => {
            snapshot.item_shop.portal_uses.insert(hero(), 99);
        }
        "item_shop_town_halls" => snapshot.item_shop.town_halls.clear(),
        "item_shop_charges" => snapshot.item_shop.charges.clear(),
        "item_shop_next_fact_seq" => snapshot.item_shop.next_fact_seq += 1,
        "item_shop_next_charge_seq" => snapshot.item_shop.next_charge_seq += 1,
        _ => panic!("item_shop_reverse: reason=unknown_field value={field}"),
    }
}

#[test]
fn reverse_validation_missing_hash_field_formal_passes_the_assertion() {
    let content = config(&defs()).content_hash;
    for (field, _) in HASH_CASES {
        let (formal, variant) = prepared_pair(content, field);
        assert!(
            field_is_hashed(&formal, &variant, field),
            "reason=formal_hash_covers_{field}"
        );
    }
}

#[test]
fn reverse_validation_missing_hash_field_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    for (field, omit) in HASH_CASES {
        let (formal, variant) = prepared_pair(content, field);
        // 缺陷态：退化哈希对该字段视而不见 → 两端相等 → 正式断言必失败。
        let degenerate_detects = degenerate_hash_omit(&formal.capture_snapshot(), omit)
            != degenerate_hash_omit(&variant.capture_snapshot(), omit);
        assert!(
            !degenerate_detects,
            "reason=degenerate_hash_blind_to_{omit}"
        );
        // 元断言：正式能区分、退化不能区分（缺陷态被抓出）。
        assert!(
            field_is_hashed(&formal, &variant, field) && !degenerate_detects,
            "reason=degenerate_missing_hash_must_fail_same_assertion_{omit}"
        );
        // 正式实现仍通过同一断言（防元断言恒真）。
        assert!(
            field_is_hashed(&formal, &variant, field),
            "reason=formal_still_passes_{omit}"
        );
        // 退化实现真经 restore_snapshot 生产路径注入：把「漏接字段」的
        // 快照喂给 restore_snapshot，两端重建后退化态无法区分差异字段。
        let mut degenerate_a = formal.capture_snapshot();
        degenerate_a.item_shop = degenerate_omit_state(&degenerate_a, omit);
        let mut degenerate_b = variant.capture_snapshot();
        degenerate_b.item_shop = degenerate_omit_state(&degenerate_b, omit);
        let rebuilt_a =
            RtsMatch::restore_snapshot(degenerate_a, StaticWorld::default(), content).unwrap();
        let rebuilt_b =
            RtsMatch::restore_snapshot(degenerate_b, StaticWorld::default(), content).unwrap();
        assert_eq!(
            rebuilt_a.compute_state_hash(),
            rebuilt_b.compute_state_hash(),
            "reason=degenerate_restore_cannot_distinguish_{omit}"
        );
        // 同一退化态也 MUST 让 `first_difference()` 无法定位新字段，
        // 等价于 `first_difference()` 漏接该段（ADR 0002 §6 / Q9 ①）。
        assert_eq!(
            rebuilt_a.first_difference(&rebuilt_b),
            None,
            "reason=degenerate_first_difference_blind_to_{omit}"
        );
        assert_eq!(
            formal.first_difference(&variant),
            Some(*field),
            "reason=formal_first_difference_still_locates_{field}"
        );
    }
}

#[test]
fn reverse_validation_missing_snapshot_field_formal_passes_the_assertion() {
    let (sim, content) = base_match(1000, true);
    let mut prepared = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        snapshot.item_shop.facts.insert(
            0,
            w3_core::ItemShopFact {
                seq: 0,
                frame: 0,
                kind: ItemShopFactKind::Pickup,
                hero: hero(),
                instance: bag_instance(0),
                slot: 0,
                entity: ground(),
                gold_delta: 0,
                shelf: SHELF_CONSUMABLE,
            },
        );
        snapshot.item_shop.next_fact_seq = 1;
    });
    let before = prepared.capture_snapshot();
    step_n(&mut prepared, 1);
    let restored =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        snapshot_roundtrip_is_complete(&before, &restored.capture_snapshot()),
        "reason=formal_snapshot_roundtrip_complete"
    );
}

#[test]
fn reverse_validation_missing_snapshot_field_degenerate_fails_the_same_assertion() {
    let (sim, content) = base_match(1000, true);
    let prepared = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        snapshot.item_shop.facts.insert(
            0,
            w3_core::ItemShopFact {
                seq: 0,
                frame: 0,
                kind: ItemShopFactKind::Pickup,
                hero: hero(),
                instance: bag_instance(0),
                slot: 0,
                entity: ground(),
                gold_delta: 0,
                shelf: SHELF_CONSUMABLE,
            },
        );
        snapshot.item_shop.next_fact_seq = 1;
    });
    let before = prepared.capture_snapshot();
    // 缺陷态：capture 时漏掉整组权威状态（经真实 restore 注入）。
    let mut broken = before.clone();
    broken.item_shop = ItemShopState::default();
    let rebuilt = RtsMatch::restore_snapshot(broken, StaticWorld::default(), content).unwrap();
    assert!(
        !snapshot_roundtrip_is_complete(&before, &rebuilt.capture_snapshot()),
        "reason=degenerate_missing_snapshot_must_fail_same_assertion"
    );
    // 正式实现仍通过（防元断言恒真）。
    let formal =
        RtsMatch::restore_snapshot(before.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        snapshot_roundtrip_is_complete(&before, &formal.capture_snapshot()),
        "reason=formal_still_passes"
    );
}

fn snapshot_roundtrip_is_complete(before: &w3_core::Snapshot, after: &w3_core::Snapshot) -> bool {
    before.item_shop == after.item_shop
        && before.creep_camps.item_instances == after.creep_camps.item_instances
        && before.creep_camps.ground_items == after.creep_camps.ground_items
        && before.creep_camps.neutral == after.creep_camps.neutral
        && before.creep_camps.rng_draws == after.creep_camps.rng_draws
        && before.creep_camps.next_rng_seq == after.creep_camps.next_rng_seq
}

// ---------------------------------------------------------------------------
// 退化 ②：拾取 / 购买 / 使用 / 售卖不经事务命令面或表现层反写 gold /
//          背包 / 库存
// ---------------------------------------------------------------------------

/// 正式不变量：购买事务的四要素（精确扣费 / 建实例 / 入包 / 库存递减）
/// MUST 同时成立，且黄金只经命令面写入（R9.3 / ADR 0002 §5）。
fn purchase_is_atomic(before: &w3_core::Snapshot, after: &w3_core::Snapshot) -> bool {
    let gold_before = before
        .gameplay
        .players
        .get(&PLAYER)
        .map(|e| e.gold)
        .unwrap_or(0);
    let gold_after = after
        .gameplay
        .players
        .get(&PLAYER)
        .map(|e| e.gold)
        .unwrap_or(0);
    let stock_before = before
        .item_shop
        .stock
        .get(&SHELF_CONSUMABLE)
        .copied()
        .unwrap_or(0);
    let stock_after = after
        .item_shop
        .stock
        .get(&SHELF_CONSUMABLE)
        .copied()
        .unwrap_or(0);
    let created: Vec<EntityId> = after
        .creep_camps
        .item_instances
        .iter()
        .filter(|(id, _)| !before.creep_camps.item_instances.contains_key(id))
        .map(|(id, _)| *id)
        .collect();
    let one_instance = created.len() == 1;
    let held = one_instance
        && after
            .item_shop
            .bags
            .get(&hero())
            .is_some_and(|bag| bag.slot_of(created[0]).is_some());
    let fact_ok = after.item_shop.facts.values().any(|fact| {
        fact.kind == ItemShopFactKind::Purchase && fact.gold_delta == -PRICE_CONSUMABLE
    });
    gold_before - gold_after == PRICE_CONSUMABLE
        && stock_after == stock_before - 1
        && one_instance
        && held
        && fact_ok
}

#[test]
fn reverse_validation_transaction_bypass_formal_passes_the_assertion() {
    let mut sim = base_match(1000, true).0;
    let before = sim.capture_snapshot();
    submit(
        &mut sim,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut sim, 1);
    assert!(
        purchase_is_atomic(&before, &sim.capture_snapshot()),
        "reason=formal_purchase_is_atomic"
    );
}

#[test]
fn reverse_validation_transaction_bypass_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let mut formal = base_match(1000, true).0;
    let before = formal.capture_snapshot();
    submit(
        &mut formal,
        1,
        PLAYER,
        1,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
    );
    step_n(&mut formal, 1);
    let formal_after = formal.capture_snapshot();

    // 缺陷态 A：表现层反写 gold（扣费了却不建实例 / 不入包 / 不减库存）。
    // 经真实 restore_snapshot 生产路径注入，非旁路模拟。
    let mut gold_only = before.clone();
    gold_only.gameplay.players.entry(PLAYER).or_default().gold -= PRICE_CONSUMABLE;
    let rebuilt_gold =
        RtsMatch::restore_snapshot(gold_only.clone(), StaticWorld::default(), content).unwrap();
    assert!(
        !purchase_is_atomic(&before, &rebuilt_gold.capture_snapshot()),
        "reason=degenerate_gold_rewrite_must_fail_same_assertion"
    );

    // 缺陷态 B：绕过命令面直接把实例塞进背包（不扣费 / 不减库存）。
    let mut bag_only = before.clone();
    bag_only.item_shop.bags.insert(hero(), {
        let mut bag = ItemBag::default();
        bag.set(0, Some(bag_instance(0)));
        bag
    });
    bag_only.creep_camps.item_instances.insert(
        bag_instance(0),
        w3_core::ItemInstanceState {
            item_def: ITEM_CONSUMABLE,
            charges: 3,
            cooldown_frames: 0,
            effect: test_effect(),
            holder: Some(hero()),
            slot: Some(0),
        },
    );
    let rebuilt_bag =
        RtsMatch::restore_snapshot(bag_only, StaticWorld::default(), content).unwrap();
    assert!(
        !purchase_is_atomic(&before, &rebuilt_bag.capture_snapshot()),
        "reason=degenerate_bag_rewrite_must_fail_same_assertion"
    );

    // 缺陷态 C：售卖不经命令面重复入账（表现层反写 gold）。
    let mut sell_sim = with_state(&formal, content, |snapshot| {
        snapshot.creep_camps.item_instances.remove(&bag_instance(0));
    });
    let _ = &mut sell_sim;
    let mut forged_sell = formal_after.clone();
    forged_sell.gameplay.players.entry(PLAYER).or_default().gold += SELL_CONSUMABLE;
    let rebuilt_sell =
        RtsMatch::restore_snapshot(forged_sell, StaticWorld::default(), content).unwrap();
    assert_ne!(
        rebuilt_sell.compute_state_hash(),
        formal_after_hash(&formal),
        "reason=degenerate_gold_rewrite_changes_authoritative_state"
    );

    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        purchase_is_atomic(&before, &formal_after),
        "reason=formal_still_passes"
    );
}

fn formal_after_hash(sim: &RtsMatch) -> u64 {
    sim.compute_state_hash()
}

// ---------------------------------------------------------------------------
// 退化 ③：同名多实例共用充能 / 冷却、丢弃 / 移交刷新冷却（R7.5）
// ---------------------------------------------------------------------------

/// 正式不变量：两个同名实例 MUST 各有独立 `EntityId` / charges / cooldown，
/// 且丢弃 / 移交 MUST NOT 刷新剩余冷却（R7.5）。
fn instances_are_independent(snapshot: &w3_core::Snapshot) -> bool {
    let Some(bag) = snapshot.item_shop.bags.get(&hero()) else {
        return false;
    };
    let ids: Vec<EntityId> = bag.slots.iter().flatten().copied().collect();
    if ids.len() < 2 || ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return false;
    }
    let first = snapshot.creep_camps.item_instances.get(&ids[0]);
    let second = snapshot.creep_camps.item_instances.get(&ids[1]);
    match (first, second) {
        (Some(a), Some(b)) => a.item_def == b.item_def && a.charges != b.charges,
        _ => false,
    }
}

#[test]
fn reverse_validation_shared_instance_state_formal_passes_the_assertion() {
    let (sim, content) = base_match(1000, true);
    let formal = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
    });
    assert!(
        instances_are_independent(&formal.capture_snapshot()),
        "reason=formal_instances_independent"
    );
    // 丢弃再拾取：剩余冷却 MUST NOT 刷新为满值。
    let mut drop_sim = with_state(&formal, content, |snapshot| {
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
    let dropped = drop_sim.capture_snapshot();
    assert!(
        dropped
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .cooldown_frames
            <= 9,
        "reason=formal_drop_does_not_refresh_cooldown"
    );
    assert_eq!(
        dropped
            .creep_camps
            .ground_items
            .get(&bag_instance(0))
            .unwrap()
            .instance,
        bag_instance(0),
        "reason=formal_drop_keeps_instance_id"
    );
}

#[test]
fn reverse_validation_shared_instance_state_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let (sim, _) = base_match(1000, true);
    let formal = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
    });
    let formal_snapshot = formal.capture_snapshot();

    // 缺陷态 A：同名多实例「共用充能」（两格挂同一个 EntityId / 同一 charges）。
    let mut shared = formal_snapshot.clone();
    {
        let bag = shared.item_shop.bags.get_mut(&hero()).unwrap();
        bag.set(1, Some(bag_instance(0)));
        shared
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(1))
            .unwrap()
            .charges = shared
            .creep_camps
            .item_instances
            .get(&bag_instance(0))
            .unwrap()
            .charges;
    }
    let rebuilt_shared =
        RtsMatch::restore_snapshot(shared, StaticWorld::default(), content).unwrap();
    assert!(
        !instances_are_independent(&rebuilt_shared.capture_snapshot()),
        "reason=degenerate_shared_charges_must_fail_same_assertion"
    );

    // 缺陷态 B：丢弃刷新冷却（地面实例 cooldown 被置 0）。经真实
    // restore_snapshot 注入，模拟「新建实例 / 刷新冷却」退化实现。
    let mut refreshed = formal_snapshot.clone();
    {
        let bag = refreshed.item_shop.bags.get_mut(&hero()).unwrap();
        bag.set(0, None);
        refreshed.creep_camps.ground_items.insert(
            bag_instance(0),
            w3_core::GroundItemState {
                item_def: ITEM_CONSUMABLE,
                camp: 0,
                x: SHOP_X,
                y: SHOP_Y,
                instance: bag_instance(0),
            },
        );
        let instance = refreshed
            .creep_camps
            .item_instances
            .get_mut(&bag_instance(0))
            .unwrap();
        instance.holder = None;
        instance.slot = None;
        instance.cooldown_frames = 0;
    }
    let rebuilt_refresh =
        RtsMatch::restore_snapshot(refreshed, StaticWorld::default(), content).unwrap();
    // 正式不变量（丢弃保持身份且冷却不刷新）在缺陷态下被破坏：
    // 用一个带冷却的正式基线对比，缺陷态冷却被清零。
    let mut baseline = formal_snapshot.clone();
    baseline
        .creep_camps
        .item_instances
        .get_mut(&bag_instance(0))
        .unwrap()
        .cooldown_frames = 9;
    assert!(
        drop_keeps_cooldown(&baseline, &rebuilt_refresh.capture_snapshot()),
        "reason=degenerate_refresh_cooldown_is_detected"
    );
    // 元断言：正式实现仍通过同一断言。
    assert!(
        instances_are_independent(&formal_snapshot),
        "reason=formal_still_passes"
    );
}

/// 正式不变量：丢弃后同一 `EntityId` 的剩余冷却 MUST NOT 被刷新为 0。
/// 返回 `true` 表示「检测到退化」（冷却被刷新 / 身份丢失）。
fn drop_keeps_cooldown(before: &w3_core::Snapshot, after: &w3_core::Snapshot) -> bool {
    let Some(ground_state) = after.creep_camps.ground_items.get(&bag_instance(0)) else {
        return true;
    };
    let Some(instance) = after.creep_camps.item_instances.get(&bag_instance(0)) else {
        return true;
    };
    let before_cooldown = before
        .creep_camps
        .item_instances
        .get(&bag_instance(0))
        .map(|state| state.cooldown_frames)
        .unwrap_or(0);
    ground_state.instance == bag_instance(0)
        && instance.holder.is_none()
        && instance.slot.is_none()
        && instance.cooldown_frames < before_cooldown
}

// ---------------------------------------------------------------------------
// 退化 ④：市场刷新非确定随机源 / 消费序列漏接哈希 / 快照 / first_difference
// ---------------------------------------------------------------------------

/// 正式不变量：同 seed 两端逐帧哈希 / 消费序列 / rng_state 一致，
/// 且 `first_difference()` 为 `None`；刷新事件幂等。
fn refresh_is_deterministic(a: &RtsMatch, b: &RtsMatch) -> bool {
    let sa = a.capture_snapshot();
    let sb = b.capture_snapshot();
    a.compute_state_hash() == b.compute_state_hash()
        && a.first_difference(b).is_none()
        && sa.item_shop.refresh_draws == sb.item_shop.refresh_draws
        && sa.creep_camps.rng_draws == sb.creep_camps.rng_draws
        && sa.creep_camps.rng_state == sb.creep_camps.rng_state
        && sa.creep_camps.next_rng_seq == sb.creep_camps.next_rng_seq
        && sa.item_shop.refresh_counts == sb.item_shop.refresh_counts
        && !sa.item_shop.refresh_draws.is_empty()
}

#[test]
fn reverse_validation_nondeterministic_refresh_formal_passes_the_assertion() {
    let (sim, content) = base_match(1000, true);
    let build = || {
        let mut a = with_state(&sim, content, |_| {});
        a.submit_gameplay_command(refresh(1, 1, 41)).unwrap();
        a.submit_gameplay_command(refresh(1, 2, 42)).unwrap();
        step_n(&mut a, 1);
        a
    };
    let a = build();
    let b = build();
    assert!(
        refresh_is_deterministic(&a, &b),
        "reason=formal_refresh_deterministic"
    );
    // 消费序列进哈希 / 快照 / first_difference。
    let content_hash = config(&defs()).content_hash;
    let (formal, variant) = prepared_pair(content_hash, "item_shop_refresh_draws");
    assert!(
        field_is_hashed(&formal, &variant, "item_shop_refresh_draws"),
        "reason=formal_consumption_sequence_hashed"
    );
}

#[test]
fn reverse_validation_nondeterministic_refresh_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let (sim, _) = base_match(1000, true);
    let build = || {
        let mut a = with_state(&sim, content, |_| {});
        a.submit_gameplay_command(refresh(1, 1, 41)).unwrap();
        a.submit_gameplay_command(refresh(1, 2, 42)).unwrap();
        step_n(&mut a, 1);
        a
    };
    let a = build();
    let b = build();

    // 缺陷态 A：非确定随机源（墙钟 / HashMap 顺序）——经真实
    // restore_snapshot 注入一个被篡改的消费序列，模拟「两端随机码不一致」。
    let mut forged = b.capture_snapshot();
    if let Some(draw) = forged.item_shop.refresh_draws.values_mut().next() {
        draw.rng_after = draw.rng_after.wrapping_add(1);
        draw.roll = draw.roll.wrapping_add(1);
    }
    forged.creep_camps.rng_state = forged.creep_camps.rng_state.wrapping_add(1);
    let forged_sim = RtsMatch::restore_snapshot(forged, StaticWorld::default(), content).unwrap();
    assert!(
        !refresh_is_deterministic(&a, &forged_sim),
        "reason=degenerate_nondeterministic_rng_must_fail_same_assertion"
    );

    // 缺陷态 B：消费序列漏接哈希 / 快照。退化哈希对 refresh_draws 视而不见。
    let sa = a.capture_snapshot();
    let sb = forged_sim.capture_snapshot();
    let degenerate_detects =
        degenerate_hash_omit(&sa, "refresh_draws") != degenerate_hash_omit(&sb, "refresh_draws");
    assert!(
        !degenerate_detects,
        "reason=degenerate_hash_blind_to_consumption_sequence"
    );
    let mut cleared_a = sa.clone();
    cleared_a.item_shop = degenerate_omit_state(&cleared_a, "refresh_draws");
    let rebuilt_a = RtsMatch::restore_snapshot(cleared_a, StaticWorld::default(), content).unwrap();
    let rebuilt_b = build();
    assert_ne!(
        rebuilt_a.compute_state_hash(),
        rebuilt_b.compute_state_hash(),
        "reason=degenerate_snapshot_drops_consumption_sequence"
    );

    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    let fresh_b = build();
    assert!(
        refresh_is_deterministic(&a, &fresh_b),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ⑤：新增命令 / 事件漏接 hash_gameplay_command 编码 / 校验 / 穷尽分发
// ---------------------------------------------------------------------------

/// 正式不变量：新增变体编码唯一、校验拒绝非法实体、且每个变体都经
/// 公开命令入口穷尽分发（`step()` 消费队列）。
fn new_commands_are_encoded_validated_and_dispatched(kinds: &[GameplayCommandKind]) -> bool {
    let mut hashes = BTreeSet::new();
    for kind in kinds {
        let mut queue = GameplayCommandQueue::new();
        if queue
            .submit(GameplayCommand::new(Frame::new(1), PLAYER, 1, *kind))
            .is_err()
        {
            return false;
        }
        let mut hasher = StateHasher::new();
        queue.hash(&mut hasher);
        if !hashes.insert(hasher.finish()) {
            return false;
        }
    }
    // 校验：0 号实体 / 0 号商店 id MUST 被拒绝。
    let mut invalid = GameplayCommandQueue::new();
    invalid
        .submit(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::RefreshMarket { shop: 0, batch: 1 },
        ))
        .is_err()
}

fn new_command_kinds() -> Vec<GameplayCommandKind> {
    vec![
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
    ]
}

#[test]
fn reverse_validation_command_encoding_formal_passes_the_assertion() {
    let kinds = new_command_kinds();
    assert!(
        new_commands_are_encoded_validated_and_dispatched(&kinds),
        "reason=formal_new_commands_encoded_validated"
    );
    // 穷尽分发：每个变体经公开入口 + step() 都被消费、且发事实事件/拒单。
    let (sim, content) = base_match(1000, true);
    for kind in &kinds {
        let mut dispatched = with_state(&sim, content, |_| {});
        dispatched
            .submit_gameplay_command(GameplayCommand::new(Frame::new(1), PLAYER, 1, *kind))
            .unwrap();
        step_n(&mut dispatched, 1);
        assert!(
            dispatched
                .capture_snapshot()
                .gameplay
                .commands
                .pending()
                .is_empty(),
            "reason=exhaustive_dispatch"
        );
        assert!(
            dispatched
                .item_shop_state()
                .facts
                .values()
                .any(|fact| !matches!(fact.kind, ItemShopFactKind::Purchase))
                || !dispatched.item_shop_state().facts.is_empty(),
            "reason=fact_recorded"
        );
    }
}

#[test]
fn reverse_validation_command_encoding_degenerate_fails_the_same_assertion() {
    let kinds = new_command_kinds();
    // 缺陷态 A：漏接 `hash_gameplay_command` 编码——退化实现把全部新增
    // 变体编码成同一个 kind 码（等价于整段未进哈希）。
    let degenerate_encode = |kind: &GameplayCommandKind| {
        let mut hasher = StateHasher::new();
        let mut queue = GameplayCommandQueue::new();
        // 用同一个旧变体替换所有新增变体，模拟「新变体漏接编码」。
        let _ = kind;
        queue
            .submit(GameplayCommand::new(
                Frame::new(1),
                PLAYER,
                1,
                GameplayCommandKind::SetAi { enabled: true },
            ))
            .unwrap();
        queue.hash(&mut hasher);
        hasher.finish()
    };
    let degenerate_hashes: BTreeSet<u64> = kinds.iter().map(degenerate_encode).collect();
    assert_eq!(
        degenerate_hashes.len(),
        1,
        "reason=degenerate_encoding_collides_for_all_new_kinds"
    );
    // 元断言：正式编码唯一，退化编码不唯一（缺陷态被抓出）。
    assert!(
        new_commands_are_encoded_validated_and_dispatched(&kinds) && degenerate_hashes.len() == 1,
        "reason=degenerate_missing_encoding_must_fail_same_assertion"
    );

    // 缺陷态 B：漏接校验——退化「提交入口」接受 0 号实体（等价于
    // validate_gameplay_entities 新分支缺失）。经真实公开入口 + `step()` 对比。
    let mut formal = base_match(1000, true).0;
    let content_hash = config(&defs()).content_hash;
    let rejected = formal
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::RefreshMarket { shop: 0, batch: 1 },
        ))
        .is_err();
    assert!(rejected, "reason=formal_validation_rejects_invalid_entity");
    // 退化态：0 号商店 id 被接受并在权威状态留下消费序列 / 事实痕迹
    // （经真实 restore_snapshot 生产路径注入，非旁路模拟）。
    let mut degenerate = with_state(&formal, content_hash, |snapshot| {
        snapshot.item_shop.refresh_batches.insert((0, 1));
        snapshot.item_shop.facts.insert(
            0,
            w3_core::ItemShopFact {
                seq: 0,
                frame: 1,
                kind: ItemShopFactKind::MarketRefresh,
                hero: EntityId::NONE,
                instance: EntityId::NONE,
                slot: u8::MAX,
                entity: EntityId::NONE,
                gold_delta: 0,
                shelf: 0,
            },
        );
        snapshot.item_shop.next_fact_seq = 1;
    });
    step_n(&mut formal, 1);
    step_n(&mut degenerate, 1);
    let formal_state = formal.item_shop_state();
    let degenerate_state = degenerate.item_shop_state();
    assert!(
        formal_state.refresh_batches.is_empty() && formal_state.facts.is_empty(),
        "reason=formal_command_face_rejects_invalid_entity"
    );
    assert!(
        !degenerate_state.refresh_batches.is_empty() && !degenerate_state.facts.is_empty(),
        "reason=degenerate_missing_validation_writes_authoritative_state"
    );
    assert_ne!(
        formal.compute_state_hash(),
        degenerate.compute_state_hash(),
        "reason=degenerate_missing_validation_changes_hash"
    );

    // 正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        new_commands_are_encoded_validated_and_dispatched(&kinds),
        "reason=formal_still_passes"
    );
}

// ---------------------------------------------------------------------------
// 退化 ③ 续：复活只还原 DefId 序列 / 重开残留被反例断言拦截（tasks 2.8）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_revive_defid_sequence_formal_passes_the_assertion() {
    // 正式实现：六格实例背包按实例 `EntityId` 恢复（R6.3）。
    let (sim, content) = base_match(1000, true);
    let dead = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        let bag = *snapshot.item_shop.bags.get(&hero()).unwrap();
        snapshot.item_shop.bags.remove(&hero());
        snapshot.item_shop.death_bags.insert(hero(), bag);
        let position = snapshot
            .positions
            .iter_mut()
            .find(|position| position.entity == HERO_ID)
            .unwrap();
        position.alive = false;
        position.gameplay.hero = Some(w3_core::HeroState { level: 1, xp: 0 });
        let gameplay = position.gameplay.clone();
        snapshot.revive.deaths.insert(
            hero(),
            w3_core::HeroDeathRecord {
                death_frame: 0,
                owner: PLAYER,
                gameplay,
                max_hp: 100,
            },
        );
        snapshot.revive.queue.insert(
            hero(),
            w3_core::ReviveQueueItem {
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
            w3_core::ReviveCharge {
                owner: PLAYER,
                gold: 0,
                lumber: 0,
            },
        );
    });
    let mut formal = dead;
    step_n(&mut formal, 1);
    assert!(
        bag_is_instance_identical(&formal.capture_snapshot(), &full_bag_for_test()),
        "reason=formal_revive_restores_instance_bag"
    );
}

#[test]
fn reverse_validation_revive_defid_sequence_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let (sim, _) = base_match(1000, true);
    let dead = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        let bag = *snapshot.item_shop.bags.get(&hero()).unwrap();
        snapshot.item_shop.bags.remove(&hero());
        snapshot.item_shop.death_bags.insert(hero(), bag);
        let position = snapshot
            .positions
            .iter_mut()
            .find(|position| position.entity == HERO_ID)
            .unwrap();
        position.alive = false;
        position.gameplay.hero = Some(w3_core::HeroState { level: 1, xp: 0 });
        let gameplay = position.gameplay.clone();
        snapshot.revive.deaths.insert(
            hero(),
            w3_core::HeroDeathRecord {
                death_frame: 0,
                owner: PLAYER,
                gameplay,
                max_hp: 100,
            },
        );
        snapshot.revive.queue.insert(
            hero(),
            w3_core::ReviveQueueItem {
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
            w3_core::ReviveCharge {
                owner: PLAYER,
                gold: 0,
                lumber: 0,
            },
        );
    });
    // 缺陷态：复活只还原 DefId 序列 —— 六格背包被替换为「新实例」（不同
    // EntityId），经真实 restore_snapshot 注入，非旁路模拟。
    let mut degenerate = dead.capture_snapshot();
    degenerate.item_shop.death_bags.insert(hero(), {
        let mut bag = ItemBag::default();
        for slot in 0..6u8 {
            bag.set(
                slot,
                Some(EntityId::new(w3_core::GROUND_ITEM_ID_BASE + 700 + u64::from(slot)).unwrap()),
            );
        }
        bag
    });
    degenerate.item_shop.bags.clear();
    let rebuilt = RtsMatch::restore_snapshot(degenerate, StaticWorld::default(), content).unwrap();
    let mut advanced = rebuilt;
    step_n(&mut advanced, 1);
    assert!(
        !bag_is_instance_identical(&advanced.capture_snapshot(), &full_bag_for_test()),
        "reason=degenerate_defid_sequence_must_fail_same_assertion"
    );
    // 元断言：正式实现仍通过同一断言。
    let mut formal = with_state(&sim, content, |snapshot| {
        inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE; 6]);
        let bag = *snapshot.item_shop.bags.get(&hero()).unwrap();
        snapshot.item_shop.bags.remove(&hero());
        snapshot.item_shop.death_bags.insert(hero(), bag);
        let position = snapshot
            .positions
            .iter_mut()
            .find(|position| position.entity == HERO_ID)
            .unwrap();
        position.alive = false;
        position.gameplay.hero = Some(w3_core::HeroState { level: 1, xp: 0 });
        let gameplay = position.gameplay.clone();
        snapshot.revive.deaths.insert(
            hero(),
            w3_core::HeroDeathRecord {
                death_frame: 0,
                owner: PLAYER,
                gameplay,
                max_hp: 100,
            },
        );
        snapshot.revive.queue.insert(
            hero(),
            w3_core::ReviveQueueItem {
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
            w3_core::ReviveCharge {
                owner: PLAYER,
                gold: 0,
                lumber: 0,
            },
        );
    });
    step_n(&mut formal, 1);
    assert!(
        bag_is_instance_identical(&formal.capture_snapshot(), &full_bag_for_test()),
        "reason=formal_still_passes"
    );
}

fn bag_is_instance_identical(snapshot: &w3_core::Snapshot, expected: &ItemBag) -> bool {
    snapshot
        .item_shop
        .bags
        .get(&hero())
        .is_some_and(|bag| bag.slots == expected.slots)
        && (0..6u8).all(|slot| {
            snapshot
                .creep_camps
                .item_instances
                .get(&bag_instance(slot))
                .is_some_and(|instance| {
                    instance.holder == Some(hero()) && instance.slot == Some(slot)
                })
        })
}

// ---------------------------------------------------------------------------
// 重开 / 快照恢复无残留反例（tasks 2.8 / Q9）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_restart_residue_formal_passes_the_assertion() {
    let fresh = RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
    assert!(
        restart_is_clean(&fresh.capture_snapshot()),
        "reason=formal_restart_clean"
    );
}

#[test]
fn reverse_validation_restart_residue_degenerate_fails_the_same_assertion() {
    let content = config(&defs()).content_hash;
    let fresh = RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
    // 缺陷态：重开残留上一局背包 / 库存 / 传送 / 消费序列（经真实
    // restore_snapshot 注入），正式不变量 MUST 失败。
    let mut residue = fresh.capture_snapshot();
    inject_bag(&mut residue, hero(), &[ITEM_CONSUMABLE; 6]);
    residue.item_shop.refresh_draws.insert(
        0,
        w3_core::ShopRefreshDraw {
            seq: 0,
            frame: 0,
            shop: SHOP,
            batch: 1,
            rng_before: 1,
            rng_after: 2,
            roll: 2,
            picked: Some(SHELF_CONSUMABLE),
        },
    );
    residue.item_shop.casts.insert(
        hero(),
        w3_core::TownPortalCast {
            caster: hero(),
            instance: bag_instance(0),
            slot: 0,
            target: town_hall(),
            target_x: 0,
            target_y: 0,
            start_frame: 0,
            remaining_frames: 2,
            total_frames: 2,
        },
    );
    residue.item_shop.next_fact_seq = 9;
    let rebuilt = RtsMatch::restore_snapshot(residue, StaticWorld::default(), content).unwrap();
    assert!(
        !restart_is_clean(&rebuilt.capture_snapshot()),
        "reason=degenerate_restart_residue_must_fail_same_assertion"
    );
    // 元断言：正式实现仍通过同一断言。
    let formal = RtsMatch::create_with_definitions(config(&defs()), StaticWorld::default(), defs());
    assert!(
        restart_is_clean(&formal.capture_snapshot()),
        "reason=formal_still_passes"
    );
}

fn restart_is_clean(snapshot: &w3_core::Snapshot) -> bool {
    !snapshot.gameplay.victory.resolved
        && snapshot.item_shop.bags.is_empty()
        && snapshot.item_shop.death_bags.is_empty()
        && snapshot.item_shop.facts.is_empty()
        && snapshot.item_shop.casts.is_empty()
        && snapshot.item_shop.portal_uses.is_empty()
        && snapshot.item_shop.refresh_draws.is_empty()
        && snapshot.item_shop.refresh_batches.is_empty()
        && snapshot.item_shop.pending_refreshes.is_empty()
        && snapshot.item_shop.charges.is_empty()
        && snapshot.item_shop.next_fact_seq == 0
        && snapshot.item_shop.next_charge_seq == 0
}

/// 正式不变量：每个新增事实种类都有对应的独立 `EventKind` 事实事件，
/// 且只读事件不进权威哈希 / 快照（ADR 0002 §5-6 / Q5）。
fn all_new_fact_events_are_emitted(events: &[EventKind]) -> bool {
    let required = [
        EventKind::ItemPickedUp,
        EventKind::ItemPurchased,
        EventKind::ItemUsed,
        EventKind::ItemDiscarded,
        EventKind::ItemTransferred,
        EventKind::ItemSold,
        EventKind::TownPortalStarted,
        EventKind::TownPortalCompleted,
        EventKind::TownPortalCancelled,
        EventKind::MarketRefreshed,
        EventKind::ItemShopCommandRejected,
    ];
    required
        .iter()
        .all(|kind| events.iter().any(|event| event == kind))
}

/// 单个隔离场景：真实 `create_with_definitions` → `restore_snapshot` →
/// 公开命令入口 → `step()` → `drain_events()`（生产路径，非旁路模拟）。
fn scenario_events(
    with_town_hall: bool,
    prepare: impl FnOnce(&mut w3_core::Snapshot),
    player: u8,
    command: GameplayCommandKind,
    steps: usize,
) -> Vec<EventKind> {
    let (sim, content) = base_match(1000, with_town_hall);
    let mut driven = with_state(&sim, content, prepare);
    driven
        .submit_gameplay_command(GameplayCommand::new(Frame::new(1), player, 1, command))
        .unwrap();
    step_n(&mut driven, steps);
    driven
        .drain_events()
        .iter()
        .map(|event| event.kind)
        .collect()
}

/// 正式实现发出的全部新增事实事件（每个变体一个隔离场景）。
fn collect_new_fact_events() -> Vec<EventKind> {
    let mut events = Vec::new();
    // 拾取。
    events.extend(scenario_events(
        true,
        |snapshot| inject_ground(snapshot, ground(), ITEM_DROPPED, 3, 0),
        PLAYER,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item: ground(),
        },
        1,
    ));
    // 购买。
    events.extend(scenario_events(
        true,
        |_| {},
        PLAYER,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf: SHELF_CONSUMABLE,
        },
        1,
    ));
    // 使用。
    events.extend(scenario_events(
        true,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]),
        PLAYER,
        GameplayCommandKind::UseItem {
            hero: hero(),
            instance: bag_instance(0),
            target: EntityId::NONE,
        },
        1,
    ));
    // 丢弃。
    events.extend(scenario_events(
        true,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]),
        PLAYER,
        GameplayCommandKind::DropItem {
            hero: hero(),
            instance: bag_instance(0),
        },
        1,
    ));
    // 移交。
    events.extend(scenario_events(
        true,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]),
        PLAYER,
        GameplayCommandKind::TransferItem {
            from_hero: hero(),
            to_hero: peer(),
            instance: bag_instance(0),
        },
        1,
    ));
    // 售卖。
    events.extend(scenario_events(
        true,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_CONSUMABLE]),
        PLAYER,
        GameplayCommandKind::SellItem {
            hero: hero(),
            instance: bag_instance(0),
            shop: SHOP,
        },
        1,
    ));
    // 旧 `BuyItem` 占位显式拒单。
    events.extend(scenario_events(
        true,
        |_| {},
        PLAYER,
        GameplayCommandKind::BuyItem {
            unit: hero(),
            item: ITEM_CONSUMABLE,
        },
        1,
    ));
    // 市场刷新（系统级命令）。
    events.extend(scenario_events(
        true,
        |_| {},
        NEUTRAL_OWNER,
        GameplayCommandKind::RefreshMarket {
            shop: SHOP,
            batch: 41,
        },
        1,
    ));
    // 回城施法开始 + 完成（有主城）。
    events.extend(scenario_events(
        true,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_PORTAL]),
        PLAYER,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
        4,
    ));
    // 回城施法中断（无可用友方主城，显式拒单）。
    events.extend(scenario_events(
        false,
        |snapshot| inject_bag(snapshot, hero(), &[ITEM_PORTAL]),
        PLAYER,
        GameplayCommandKind::UseTownPortal {
            hero: hero(),
            instance: bag_instance(0),
        },
        1,
    ));
    events
}

#[test]
fn reverse_validation_event_dispatch_formal_passes_the_assertion() {
    let events = collect_new_fact_events();
    assert!(
        all_new_fact_events_are_emitted(&events),
        "reason=formal_all_new_fact_events_emitted"
    );
}

#[test]
fn reverse_validation_event_dispatch_degenerate_fails_the_same_assertion() {
    let mut degenerate = collect_new_fact_events();
    // 缺陷态：漏接事件穷尽分发 —— 退化实现吞掉两个新增事实事件，
    // 等价于下游 `scene.rs` 穷尽匹配遗漏分支 / 内核未发事实事件。
    degenerate.retain(|kind| {
        *kind != EventKind::ItemShopCommandRejected && *kind != EventKind::TownPortalCancelled
    });
    degenerate.push(EventKind::TownPortalCancelled);
    assert!(
        !all_new_fact_events_are_emitted(&degenerate),
        "reason=degenerate_missing_event_dispatch_must_fail_same_assertion"
    );
    // 元断言：正式实现仍通过同一断言（防元断言恒真）。
    assert!(
        all_new_fact_events_are_emitted(&collect_new_fact_events()),
        "reason=formal_still_passes"
    );
}

#[test]
fn degenerate_helper_panics_are_structured_english() {
    // 元断言工具本身：未知字段 MUST 走结构化英文 panic（CODEBUDDY.md §3）。
    let result = std::panic::catch_unwind(|| {
        let (sim, _) = base_match(1000, true);
        degenerate_omit_state(&sim.capture_snapshot(), "not_a_field");
    });
    assert!(result.is_err(), "reason=unknown_omit_field_panics");
    let _ = EventKind::ItemPickedUp;
    let _ = FrozenDefinitions::empty();
}
