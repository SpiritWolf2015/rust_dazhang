//! P8.5d 物品 / 商店 / 回城卷轴：真实装配 CPU 回归 + HUD 只读派生 + 输入过滤。
//!
//! 全部为合成命令脚本与 `synthetic_definitions()` 真实装配，零资产、零文件 IO、
//! 不开 GPU 窗口、不引入墙钟。测试经 `create_with_definitions` →
//! `MeleeSetup` 的公开 `Spawn` 命令 → `capture_snapshot()` / `restore_snapshot()`
//! 既有确定性生产路径注入初始只读状态，再经 `submit_gameplay_command` →
//! `step()` 证明拾取 / 购买 / 使用 / 回城 / 刷新命令面可达；双实例同脚本逐帧
//! `compute_state_hash()` / `capture_snapshot()` / `first_difference()` 一致。
//!
//! HUD 侧只从只读 `Snapshot.item_shop` 与十一类事实事件拷贝派生；不持有
//! `RtsMatch` 写句柄，不进状态哈希 / 快照 / `first_difference()`。真窗口 UI
//! 接线待后续 change：当前输入翻译层没有拾取 / 购买 / 使用 / 售卖 / 回城键位
//! 绑定，本文件只验证同一输入翻译链对地面物品 / 商店混合选中集合的 `Move`
//! 过滤，不冒充真窗口验收。

use std::collections::BTreeMap;

use w3_client::melee::defs::{
    HUMAN_HERO, ITEM_SHOP_CONSUMABLE, ITEM_SHOP_DROP_CONSUMABLE, ITEM_SHOP_DROP_PERMANENT,
    ITEM_SHOP_PERMANENT, ITEM_SHOP_X, ITEM_SHOP_Y, SHELF_CONSUMABLE, SHELF_PERMANENT,
    SHELF_TOWN_PORTAL, SHOP_DEF, SHOP_ID, TOWN_HALL_PORTAL_DEF, TOWN_PORTAL_ITEM,
};
use w3_client::melee::{LOCAL_PLAYER, MeleeSetup, synthetic_definitions};
use w3_client::{
    ClientCommand, InputAction, InputState, ItemShopHudModel, MovabilityMap, ScreenMapper,
    derive_movability, effective_movability, filter_movable,
};
use w3_core::{
    AbilityDef, Command, CommandKind, DefId, EffectDef, EffectKind, EntityId, EntityView, Event,
    EventKind, Frame, FrozenDefinitions, GROUND_ITEM_ID_BASE, GameplayCommand, GameplayCommandKind,
    GroundItemState, HeroState, ItemDef, ItemInstanceState, ItemShopFactKind, MatchConfig,
    NEUTRAL_OWNER, NeutralCategory, RtsMatch, SCALE, SIM_VERSION, SNAPSHOT_VERSION, Snapshot,
    StaticWorld, UnitDef, UpgradeDef, WorldView, shop_fixture_id,
};

const SEED: u64 = 0x5eed;
const START_GOLD: i64 = 1000;
const GROUND_INSTANCE_ID: u64 = GROUND_ITEM_ID_BASE + 1;
const BAG_INSTANCE_BASE: u64 = GROUND_ITEM_ID_BASE + 100;
const NEXT_INSTANCE_ID: u64 = GROUND_ITEM_ID_BASE + 1000;
const PORTAL_TARGET_X: i64 = 5 * SCALE;
const PORTAL_TARGET_Y: i64 = 5 * SCALE;
const HERO_VIEW_ID: u64 = 1;

/// 一次装配的固定实体编号与实例编号，纯 CPU 夹具数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fixture {
    hero: EntityId,
    town_hall: EntityId,
    ground_item: EntityId,
    bag_consumable: EntityId,
    bag_portal: EntityId,
    shop_fixture: EntityId,
}

/// 由整数字面量构造稳定实例编号；显式处理非法零值，不静默吞错。
fn instance(id: u64) -> EntityId {
    EntityId::new(id).expect("reason=invalid_instance_id")
}

/// 由槽位推导稳定背包实例编号，同名道具多实例也各有独立身份。
fn bag_instance(slot: u8) -> EntityId {
    instance(BAG_INSTANCE_BASE + u64::from(slot))
}

/// 合成道具效果：只走既有 `EffectDef` / `EffectKind`，不引入 JASS 入口。
fn heal_effect() -> EffectDef {
    EffectDef {
        kind: EffectKind::Heal,
        amount: 5,
        duration_frames: 0,
    }
}

/// 合成回城卷轴效果：同样只用既有 `EffectDef` 组合表达语义。
fn portal_effect() -> EffectDef {
    EffectDef {
        kind: EffectKind::BuffStat,
        amount: 1,
        duration_frames: 8,
    }
}

/// 把一件背包实例注入只读快照；注入路径仍经 `restore_snapshot()` 生效。
fn inject_bag_item(
    snapshot: &mut Snapshot,
    hero: EntityId,
    slot: u8,
    item_def: DefId,
    charges: u32,
    effect: EffectDef,
) -> EntityId {
    let instance_id = bag_instance(slot);
    let mut bag = snapshot
        .item_shop
        .bags
        .get(&hero)
        .copied()
        .unwrap_or_default();
    assert!(
        bag.set(slot, Some(instance_id)),
        "reason=bag_slot_out_of_range"
    );
    snapshot.item_shop.bags.insert(hero, bag);
    snapshot.creep_camps.item_instances.insert(
        instance_id,
        ItemInstanceState {
            item_def,
            charges,
            cooldown_frames: 0,
            effect,
            holder: Some(hero),
            slot: Some(slot),
        },
    );
    instance_id
}

/// 把一件无主地面物品注入只读快照，带独立充能 / 冷却与中立登记。
fn inject_ground_item(snapshot: &mut Snapshot, ground_item: EntityId, item_def: DefId) {
    snapshot.creep_camps.ground_items.insert(
        ground_item,
        GroundItemState {
            item_def,
            camp: 0,
            x: ITEM_SHOP_X,
            y: ITEM_SHOP_Y,
            instance: ground_item,
        },
    );
    snapshot.creep_camps.item_instances.insert(
        ground_item,
        ItemInstanceState {
            item_def,
            charges: 2,
            cooldown_frames: 0,
            effect: heal_effect(),
            holder: None,
            slot: None,
        },
    );
    snapshot
        .creep_camps
        .neutral
        .insert(ground_item, NeutralCategory::GroundItemOwnerless);
}

/// 真实装配 + `capture_snapshot` / `restore_snapshot` 生产路径注入初始状态。
///
/// `MeleeSetup` 只提交 frame-0 的公开 `Spawn` 命令；后续物品命令全部经
/// `submit_gameplay_command` 提交。注入的是只读快照 public 字段，最终仍由
/// `RtsMatch::restore_snapshot` 载入，MUST NOT 视为表现层反写权威状态。
fn prepared_match(defs: &FrozenDefinitions) -> (RtsMatch, u64, Fixture) {
    let content = defs.content_hash();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    let setup = MeleeSetup::new();
    for command in &setup.commands {
        sim.submit_command(command.clone())
            .expect("reason=spawn_command_rejected");
    }
    sim.step();

    let hero = setup.local_peasants[0];
    let town_hall = setup.local_hall;
    let ground_item = instance(GROUND_INSTANCE_ID);
    let shop_fixture = instance(shop_fixture_id(SHOP_ID));
    let mut snapshot = sim.capture_snapshot();
    snapshot
        .gameplay
        .players
        .entry(LOCAL_PLAYER)
        .or_default()
        .gold = START_GOLD;
    snapshot.creep_camps.next_ground_item_id = NEXT_INSTANCE_ID;
    if let Some(position) = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == hero.value())
    {
        position.x = ITEM_SHOP_X;
        position.y = ITEM_SHOP_Y;
        position.gameplay.hero = Some(HeroState { level: 1, xp: 0 });
    }
    if let Some(position) = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == town_hall.value())
    {
        position.x = PORTAL_TARGET_X;
        position.y = PORTAL_TARGET_Y;
    }
    snapshot
        .item_shop
        .town_halls
        .insert(town_hall, TOWN_HALL_PORTAL_DEF);
    inject_ground_item(&mut snapshot, ground_item, ITEM_SHOP_DROP_CONSUMABLE);
    let bag_consumable = inject_bag_item(
        &mut snapshot,
        hero,
        0,
        ITEM_SHOP_CONSUMABLE,
        3,
        heal_effect(),
    );
    let bag_portal = inject_bag_item(&mut snapshot, hero, 1, TOWN_PORTAL_ITEM, 3, portal_effect());
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
        .expect("reason=snapshot_restore_failed");
    (
        restored,
        content,
        Fixture {
            hero,
            town_hall,
            ground_item,
            bag_consumable,
            bag_portal,
            shop_fixture,
        },
    )
}

/// 含拾取 / 购买 / 使用 / 回城 / 市场刷新的确定性脚本（双实例 / 回放共用）。
fn script(fixture: &Fixture) -> Vec<GameplayCommand> {
    vec![
        GameplayCommand::new(
            Frame::new(1),
            LOCAL_PLAYER,
            1,
            GameplayCommandKind::PickupGroundItem {
                hero: fixture.hero,
                ground_item: fixture.ground_item,
            },
        ),
        GameplayCommand::new(
            Frame::new(1),
            LOCAL_PLAYER,
            2,
            GameplayCommandKind::BuyFromShop {
                hero: fixture.hero,
                shop: SHOP_ID,
                shelf: SHELF_CONSUMABLE,
            },
        ),
        GameplayCommand::new(
            Frame::new(2),
            LOCAL_PLAYER,
            1,
            GameplayCommandKind::UseItem {
                hero: fixture.hero,
                instance: fixture.bag_consumable,
                target: EntityId::NONE,
            },
        ),
        GameplayCommand::new(
            Frame::new(2),
            LOCAL_PLAYER,
            2,
            GameplayCommandKind::UseTownPortal {
                hero: fixture.hero,
                instance: fixture.bag_portal,
            },
        ),
        GameplayCommand::new(
            Frame::new(3),
            NEUTRAL_OWNER,
            1,
            GameplayCommandKind::RefreshMarket {
                shop: SHOP_ID,
                batch: 41,
            },
        ),
    ]
}

/// 经公开命令入口提交脚本并推进指定整数帧；`step()` 全程无参。
fn submit_and_step(sim: &mut RtsMatch, fixture: &Fixture, frames: usize) {
    for command in script(fixture) {
        sim.submit_gameplay_command(command)
            .expect("reason=gameplay_command_rejected");
    }
    for _ in 0..frames {
        sim.step();
    }
}

/// 只读快照中是否登记了指定物品 / 商店 / 回城事实。
fn has_fact(snapshot: &Snapshot, kind: ItemShopFactKind) -> bool {
    snapshot
        .item_shop
        .facts
        .values()
        .any(|fact| fact.kind == kind)
}

/// 与正式装配同源、但故意漏接 `with_item_shop` 冻结段的退化定义（缺陷态）。
/// 复活 / 主城升级 / 野怪营地段仍同源登记，使唯一差异就是商店段缺失。
fn defs_without_item_shop_segment() -> FrozenDefinitions {
    let full = synthetic_definitions();
    let units: Vec<(DefId, UnitDef)> = full.units().iter().map(|(&id, &def)| (id, def)).collect();
    let abilities: Vec<(DefId, AbilityDef)> = full
        .abilities()
        .iter()
        .map(|(&id, def)| (id, def.clone()))
        .collect();
    let items: Vec<(DefId, ItemDef)> = full.items().iter().map(|(&id, &def)| (id, def)).collect();
    let upgrades: Vec<(DefId, UpgradeDef)> = full
        .upgrades()
        .iter()
        .map(|(&id, &def)| (id, def))
        .collect();
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("reason=degenerate_definitions_invalid")
        .with_hero_revive(
            *full.revive_constants(),
            full.altar_defs().clone(),
            full.hero_defs().clone(),
        )
        .with_town_hall_upgrade(
            *full.town_hall_constants(),
            full.town_hall_defs().clone(),
            full.tier_requirements().clone(),
        )
        .with_creep_camps(
            *full.creep_constants(),
            full.camp_scripts().clone(),
            full.drop_tables().clone(),
            full.neutral_categories().clone(),
        )
}

/// 十一类新增事实事件拷贝，供 HUD 只读派生回归使用；`Event` 不携带 payload。
fn fact_events() -> Vec<Event> {
    let kinds = [
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
    kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| Event {
            frame: 1,
            sequence: 900_001 + index as u64,
            entity: 1,
            kind,
        })
        .collect()
}

/// 取出命令流里全部 `Move` 的目标实体编号（稳定 `EntityId` 顺序）。
fn move_targets(commands: &[ClientCommand]) -> Vec<u64> {
    commands
        .iter()
        .filter_map(|command| match command {
            ClientCommand::Simulation(Command {
                kind: CommandKind::Move { entity, .. },
                ..
            }) => Some(entity.value()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 任务 A：真实装配接入物品 / 商店 / 回城冻结段的 CPU 证据
// ---------------------------------------------------------------------------

#[test]
fn real_assembly_registers_item_shop_defs_and_snapshot_header() {
    assert_eq!(SNAPSHOT_VERSION, 8, "reason=snapshot_version_8");
    assert_eq!(SIM_VERSION, 8, "reason=sim_version_8");
    let defs = synthetic_definitions();
    assert!(
        defs.units().contains_key(&SHOP_DEF),
        "reason=shop_unit_registered"
    );
    assert!(
        defs.units().contains_key(&TOWN_HALL_PORTAL_DEF),
        "reason=portal_town_hall_unit_registered"
    );
    assert!(
        defs.items().contains_key(&ITEM_SHOP_CONSUMABLE)
            && defs.items().contains_key(&TOWN_PORTAL_ITEM)
            && defs.items().contains_key(&ITEM_SHOP_PERMANENT)
            && defs.items().contains_key(&ITEM_SHOP_DROP_CONSUMABLE)
            && defs.items().contains_key(&ITEM_SHOP_DROP_PERMANENT),
        "reason=item_defs_registered"
    );
    assert!(
        defs.item_shop().shop(SHOP_ID).is_some(),
        "reason=shop_fixture_registered"
    );
    assert!(
        defs.item_shop().shelf(SHELF_CONSUMABLE).is_some()
            && defs.item_shop().shelf(SHELF_TOWN_PORTAL).is_some()
            && defs.item_shop().shelf(SHELF_PERMANENT).is_some(),
        "reason=shelves_registered"
    );
    assert!(
        defs.item_shop().is_shop_def(SHOP_DEF)
            && defs.item_shop().is_town_hall_def(TOWN_HALL_PORTAL_DEF)
            && defs.item_shop().is_town_portal_item(TOWN_PORTAL_ITEM),
        "reason=item_shop_categories_registered"
    );
    assert!(
        defs.is_ground_item(ITEM_SHOP_DROP_CONSUMABLE)
            && defs.is_ground_item(ITEM_SHOP_DROP_PERMANENT),
        "reason=drop_categories_registered"
    );

    let (sim, _, _) = prepared_match(&defs);
    let snapshot = sim.capture_snapshot();
    assert_eq!(
        snapshot.version, SNAPSHOT_VERSION,
        "reason=snapshot_version_header"
    );
    assert_eq!(
        snapshot.sim_version, SIM_VERSION,
        "reason=sim_version_header"
    );
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_header"
    );
    assert_eq!(
        snapshot.content_hash,
        defs.content_hash(),
        "reason=content_hash_header"
    );
    assert!(
        !snapshot.item_shop.stock.is_empty(),
        "reason=stock_initialized_from_shelves"
    );
}

#[test]
fn real_assembly_commands_reach_item_shop_snapshot_and_replay_matches() {
    let defs = synthetic_definitions();
    let (mut a, content, fixture) = prepared_match(&defs);
    let (mut b, _, fixture_b) = prepared_match(&defs);
    assert_eq!(fixture, fixture_b, "reason=fixture_is_deterministic");
    for command in script(&fixture) {
        a.submit_gameplay_command(command.clone())
            .expect("reason=a_command_rejected");
        b.submit_gameplay_command(command)
            .expect("reason=b_command_rejected");
    }

    for frame in 0..16 {
        a.step();
        b.step();
        let events_a = a.drain_events();
        let events_b = b.drain_events();
        assert_eq!(events_a, events_b, "reason=event_copy_frame_{frame}");
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=replay_hash_frame_{frame}"
        );
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=replay_snapshot_frame_{frame}"
        );
        assert_eq!(
            a.first_difference(&b),
            None,
            "reason=first_difference_frame_{frame}"
        );
        assert_eq!(
            a.read_view(),
            b.read_view(),
            "reason=read_view_frame_{frame}"
        );
        if frame == 4 {
            let snapshot = a.capture_snapshot();
            a = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content)
                .expect("reason=mid_run_snapshot_restore_failed");
            assert_eq!(
                a.compute_state_hash(),
                b.compute_state_hash(),
                "reason=restore_keeps_hash"
            );
            assert_eq!(
                a.capture_snapshot(),
                b.capture_snapshot(),
                "reason=restore_keeps_snapshot"
            );
        }
    }

    let after = a.capture_snapshot();
    assert_eq!(after.version, SNAPSHOT_VERSION, "reason=snapshot_stays_v8");
    let price = defs
        .item_shop()
        .shelf(SHELF_CONSUMABLE)
        .expect("reason=missing_consumable_shelf")
        .price;
    assert_eq!(
        after
            .gameplay
            .players
            .get(&LOCAL_PLAYER)
            .map(|economy| economy.gold),
        Some(START_GOLD - price),
        "reason=exact_purchase_deduction"
    );
    assert!(
        has_fact(&after, ItemShopFactKind::Pickup),
        "reason=pickup_fact"
    );
    assert!(
        has_fact(&after, ItemShopFactKind::Purchase),
        "reason=purchase_fact"
    );
    assert!(has_fact(&after, ItemShopFactKind::Use), "reason=use_fact");
    assert!(
        has_fact(&after, ItemShopFactKind::TownPortalStart),
        "reason=town_portal_start_fact"
    );
    assert!(
        has_fact(&after, ItemShopFactKind::TownPortalComplete),
        "reason=town_portal_complete_fact"
    );
    assert!(
        has_fact(&after, ItemShopFactKind::MarketRefresh),
        "reason=market_refresh_fact"
    );
    assert!(
        after.item_shop.casts.is_empty(),
        "reason=town_portal_cast_cleaned"
    );
    assert_eq!(
        after.item_shop.portal_uses.get(&fixture.hero),
        Some(&1),
        "reason=portal_uses_recorded"
    );
    assert_eq!(
        after.item_shop.refresh_counts.get(&SHOP_ID),
        Some(&1),
        "reason=refresh_count_recorded"
    );
    assert_eq!(
        after.item_shop.refresh_draws.len(),
        1,
        "reason=refresh_draw_recorded"
    );
    assert_eq!(
        after.item_shop.refresh_batches.len(),
        1,
        "reason=refresh_batch_recorded"
    );
    let stock_max = defs
        .item_shop()
        .shelf(SHELF_CONSUMABLE)
        .expect("reason=missing_consumable_shelf")
        .stock_max;
    assert_eq!(
        after.item_shop.stock.get(&SHELF_CONSUMABLE),
        Some(&stock_max),
        "reason=refresh_restores_stock"
    );
    let bag = after
        .item_shop
        .bags
        .get(&fixture.hero)
        .expect("reason=hero_bag_missing");
    assert!(
        bag.slots.iter().flatten().count() >= 3,
        "reason=bag_has_pickup_purchase_portal_instances"
    );
    assert!(
        bag.slots.contains(&Some(fixture.ground_item)),
        "reason=ground_instance_migrated_to_bag"
    );
    assert!(
        !after
            .creep_camps
            .ground_items
            .contains_key(&fixture.ground_item),
        "reason=ground_item_removed"
    );
    assert_eq!(
        after
            .creep_camps
            .item_instances
            .get(&fixture.ground_item)
            .and_then(|instance| instance.holder),
        Some(fixture.hero),
        "reason=instance_holder_updated"
    );
    let purchase_instance = after
        .item_shop
        .facts
        .values()
        .find(|fact| fact.kind == ItemShopFactKind::Purchase)
        .expect("reason=missing_purchase_fact")
        .instance;
    assert!(
        bag.slots.contains(&Some(purchase_instance)),
        "reason=purchase_instance_in_bag"
    );
    assert_eq!(
        after
            .creep_camps
            .item_instances
            .get(&purchase_instance)
            .map(|instance| instance.item_def),
        Some(ITEM_SHOP_CONSUMABLE),
        "reason=purchase_instance_def"
    );
    let hero_position = after
        .positions
        .iter()
        .find(|position| position.entity == fixture.hero.value())
        .expect("reason=hero_position_missing");
    assert_eq!(
        (hero_position.x, hero_position.y),
        (PORTAL_TARGET_X, PORTAL_TARGET_Y),
        "reason=single_hero_teleported_to_town_hall"
    );
}

#[test]
fn reverse_validation_missing_item_shop_segment_makes_commands_unreachable() {
    let full = synthetic_definitions();
    let degenerate = defs_without_item_shop_segment();
    assert_ne!(
        full.content_hash(),
        degenerate.content_hash(),
        "reason=item_shop_segment_enters_content_hash"
    );
    assert!(
        full.item_shop().shop(SHOP_ID).is_some()
            && full.item_shop().is_shop_def(SHOP_DEF)
            && full.item_shop().is_town_hall_def(TOWN_HALL_PORTAL_DEF)
            && full.item_shop().is_town_portal_item(TOWN_PORTAL_ITEM),
        "reason=formal_item_shop_categories_available"
    );
    assert!(
        degenerate.item_shop().shop(SHOP_ID).is_none()
            && !degenerate.item_shop().is_shop_def(SHOP_DEF)
            && !degenerate
                .item_shop()
                .is_town_hall_def(TOWN_HALL_PORTAL_DEF)
            && !degenerate.item_shop().is_town_portal_item(TOWN_PORTAL_ITEM),
        "reason=degenerate_item_shop_categories_missing"
    );

    let (mut formal, _, formal_fixture) = prepared_match(&full);
    submit_and_step(&mut formal, &formal_fixture, 4);
    let formal_after = formal.capture_snapshot();
    assert!(
        has_fact(&formal_after, ItemShopFactKind::Purchase),
        "reason=formal_purchase_reachable"
    );
    assert!(
        has_fact(&formal_after, ItemShopFactKind::Use),
        "reason=formal_use_reachable"
    );
    assert!(
        has_fact(&formal_after, ItemShopFactKind::TownPortalStart),
        "reason=formal_town_portal_reachable"
    );
    assert!(
        has_fact(&formal_after, ItemShopFactKind::MarketRefresh),
        "reason=formal_refresh_reachable"
    );

    let (mut broken, _, broken_fixture) = prepared_match(&degenerate);
    submit_and_step(&mut broken, &broken_fixture, 4);
    let broken_after = broken.capture_snapshot();
    assert!(
        !has_fact(&broken_after, ItemShopFactKind::Purchase),
        "reason=degenerate_purchase_unreachable"
    );
    assert!(
        !has_fact(&broken_after, ItemShopFactKind::Use),
        "reason=degenerate_use_unreachable"
    );
    assert!(
        !has_fact(&broken_after, ItemShopFactKind::TownPortalStart),
        "reason=degenerate_town_portal_unreachable"
    );
    assert!(
        !has_fact(&broken_after, ItemShopFactKind::MarketRefresh),
        "reason=degenerate_refresh_unreachable"
    );
    assert!(
        has_fact(&broken_after, ItemShopFactKind::Rejected),
        "reason=degenerate_commands_explicitly_rejected"
    );
    assert!(
        broken_after.item_shop.stock.is_empty(),
        "reason=degenerate_stock_not_initialized"
    );
    assert!(
        broken_after.item_shop.casts.is_empty(),
        "reason=degenerate_no_half_cast"
    );
    assert!(
        broken_after.item_shop.refresh_draws.is_empty()
            && broken_after.item_shop.refresh_batches.is_empty(),
        "reason=degenerate_no_rng_consumption"
    );
}

// ---------------------------------------------------------------------------
// 任务 B：HUD 只读派生与权威状态隔离
// ---------------------------------------------------------------------------

#[test]
fn item_shop_hud_derive_is_read_only_and_snapshot_stays_v8() {
    let defs = synthetic_definitions();
    let (mut sim, _, fixture) = prepared_match(&defs);
    submit_and_step(&mut sim, &fixture, 16);
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let events = fact_events();

    // CPU 侧连续派生 16 帧；几何重绘隔离由 `w3-render/tests/isolation.rs` 覆盖。
    for _ in 0..16 {
        let model = ItemShopHudModel::derive(&snapshot_before, &events);
        assert_eq!(model.event_count, 11, "reason=eleven_event_kinds_derived");
        assert_eq!(model.picked_up_events, 1, "reason=picked_up_event_derived");
        assert_eq!(model.purchased_events, 1, "reason=purchased_event_derived");
        assert_eq!(model.used_events, 1, "reason=used_event_derived");
        assert_eq!(model.discarded_events, 1, "reason=discarded_event_derived");
        assert_eq!(
            model.transferred_events, 1,
            "reason=transferred_event_derived"
        );
        assert_eq!(model.sold_events, 1, "reason=sold_event_derived");
        assert_eq!(
            model.portal_started_events, 1,
            "reason=portal_started_event_derived"
        );
        assert_eq!(
            model.portal_completed_events, 1,
            "reason=portal_completed_event_derived"
        );
        assert_eq!(
            model.portal_cancelled_events, 1,
            "reason=portal_cancelled_event_derived"
        );
        assert_eq!(
            model.market_refreshed_events, 1,
            "reason=market_refreshed_event_derived"
        );
        assert_eq!(model.rejected_events, 1, "reason=rejected_event_derived");
        assert!(!model.bags.is_empty(), "reason=bags_derived");
        assert!(!model.stock.is_empty(), "reason=stock_derived");
        assert!(!model.facts.is_empty(), "reason=facts_derived");
        assert!(
            model.casts.is_empty(),
            "reason=completed_cast_not_in_snapshot"
        );
    }

    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=hud_derive_does_not_change_state_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=hud_derive_does_not_change_snapshot"
    );
    assert_eq!(SNAPSHOT_VERSION, 8, "reason=snapshot_version_8");
    assert_eq!(SIM_VERSION, 8, "reason=sim_version_8");
    assert_eq!(
        snapshot_before.version, SNAPSHOT_VERSION,
        "reason=snapshot_header_v8"
    );
}

// ---------------------------------------------------------------------------
// 任务 C：地面物品 / 商店混合选中集合的 `Move` 过滤（键位接线已知限制）
// ---------------------------------------------------------------------------

#[test]
fn mixed_ground_item_and_shop_selection_never_generates_move() {
    let defs = synthetic_definitions();
    let ground_item_id = GROUND_INSTANCE_ID;
    let shop_fixture_id = shop_fixture_id(SHOP_ID);
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            EntityView {
                id: HERO_VIEW_ID,
                x: 0,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 120,
                owner: LOCAL_PLAYER,
                alive: true,
                has_target: false,
            },
            // 地面物品不是 `EntityStore` 实体；这里作为合成视图标记参与 CPU
            // 回归，速度故意为正，以证明装配期 `speed_raw` 映射优先。
            EntityView {
                id: ground_item_id,
                x: 1 * SCALE,
                y: 1 * SCALE,
                facing: 0,
                speed: 3 * SCALE,
                hp: 1,
                owner: LOCAL_PLAYER,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: shop_fixture_id,
                x: 2 * SCALE,
                y: 2 * SCALE,
                facing: 0,
                speed: 3 * SCALE,
                hp: 300,
                owner: LOCAL_PLAYER,
                alive: true,
                has_target: false,
            },
        ],
    };
    let mut boxes = BTreeMap::new();
    boxes.insert(HERO_VIEW_ID, ([100.0, 100.0], [140.0, 140.0]));
    boxes.insert(ground_item_id, ([600.0, 600.0], [640.0, 640.0]));
    boxes.insert(shop_fixture_id, ([400.0, 400.0], [440.0, 440.0]));
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);
    let def_ids: BTreeMap<u64, DefId> = [
        (HERO_VIEW_ID, HUMAN_HERO),
        (ground_item_id, ITEM_SHOP_DROP_CONSUMABLE),
        (shop_fixture_id, SHOP_DEF),
    ]
    .into_iter()
    .collect();
    let base: MovabilityMap = derive_movability(&defs, &def_ids);
    assert_eq!(base.get(&HERO_VIEW_ID), Some(&true));
    assert_eq!(base.get(&ground_item_id), Some(&false));
    assert_eq!(base.get(&shop_fixture_id), Some(&false));

    let mut input = InputState::new(LOCAL_PLAYER);
    input
        .apply(
            InputAction::SelectStart([0.0, 0.0]),
            &view,
            &mapper,
            &[],
            &base,
        )
        .expect("reason=select_start_failed");
    input
        .apply(
            InputAction::SelectEnd([1280.0, 720.0]),
            &view,
            &mapper,
            &[],
            &base,
        )
        .expect("reason=select_end_failed");
    assert_eq!(input.selection().len(), 3, "reason=mixed_selection");

    let selection = input.selection().clone();
    let effective = effective_movability(&base, &view);
    assert_eq!(
        filter_movable(&selection, &effective),
        vec![HERO_VIEW_ID],
        "reason=filter_movable_keeps_only_hero"
    );
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &base,
        )
        .expect("reason=context_action_failed");
    assert_eq!(
        move_targets(&commands),
        vec![HERO_VIEW_ID],
        "reason=only_hero_receives_move"
    );
    assert!(
        !move_targets(&commands).contains(&ground_item_id),
        "reason=ground_item_never_moves"
    );
    assert!(
        !move_targets(&commands).contains(&shop_fixture_id),
        "reason=shop_fixture_never_moves"
    );
    assert_eq!(commands.len(), 1, "reason=no_extra_commands_generated");
}
