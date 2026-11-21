//! P8.5d 物品 / 商店 / 回城卷轴回归的共享夹具。
//!
//! 全部数值 / DefId 均为合成原创编译期整数常量 `[待核]`，零文件 IO、
//! 零资产、不依赖 `w3-content`，只经 `w3-core` 公开 API 构造（Q8 / §14）。
//! 场景注入一律走 `capture_snapshot()` → 改 public 字段 → `restore_snapshot()`
//! 这一既有确定性生产路径（非旁路手搓权威状态）。

#![allow(dead_code)]

use w3_core::{
    Command, CommandKind, DefId, EffectDef, EffectKind, EntityId, Frame, FrozenDefinitions,
    GROUND_ITEM_ID_BASE, GameplayCommand, GameplayCommandKind, GroundItemState, ItemBag,
    ItemInstanceState, MatchConfig, NEUTRAL_OWNER, NeutralCategory, RtsMatch, SCALE, SimFixed,
    StaticWorld, synthetic_item_shop_definitions,
};

pub const SEED: u64 = 0x5eed;
pub const PLAYER: u8 = 1;
pub const OTHER_PLAYER: u8 = 2;
/// 持包英雄（owner = `PLAYER`）。
pub const HERO_ID: u64 = 1;
/// 移交目标英雄（owner = `PLAYER`，与 HERO_ID 同族）。
pub const PEER_ID: u64 = 2;
/// 友方主城实体（owner = `PLAYER`，冻结定义层登记为主城分类）。
pub const TOWN_HALL_ID: u64 = 3;
pub const TOWN_HALL_DEF: DefId = 8002;
pub const SHOP: u32 = 7;
pub const SHELF_CONSUMABLE: u32 = 101;
pub const SHELF_PORTAL: u32 = 102;
pub const SHELF_PERMANENT: u32 = 103;
pub const ITEM_CONSUMABLE: DefId = 9001;
pub const ITEM_PORTAL: DefId = 9002;
pub const ITEM_PERMANENT: DefId = 9003;
pub const ITEM_DROPPED: DefId = 9201;
pub const PRICE_CONSUMABLE: i64 = 50;
pub const SELL_CONSUMABLE: i64 = 15;
/// 定义层商店表冻结坐标（Q47.16 raw），英雄出生位置与之重合以通过交互半径。
pub const SHOP_X: i64 = 2 * SCALE;
pub const SHOP_Y: i64 = 2 * SCALE;
pub const GROUND_ID: u64 = GROUND_ITEM_ID_BASE + 1;

/// 缩短回城施法帧便于逐帧验证，其余全为合成原创编译期整数 `[待核]`。
pub fn defs() -> FrozenDefinitions {
    let mut item_shop = synthetic_item_shop_definitions();
    item_shop.constants.town_portal_cast_frames = 2;
    FrozenDefinitions::new(vec![], vec![], vec![], vec![])
        .unwrap()
        .with_item_shop(item_shop)
}

pub fn config(frozen: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: frozen.content_hash(),
    }
}

fn spawn_at_shop(sequence: u32) -> Command {
    Command::new(
        Frame::new(0),
        PLAYER,
        sequence,
        CommandKind::Spawn {
            x: SimFixed::from_raw(SHOP_X),
            y: SimFixed::from_raw(SHOP_Y),
            hp: 100,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: 0,
            attack_range: SimFixed::from_int(10).unwrap(),
        },
    )
}

/// 正式对局：frame-0 出生三个实体（id 1 / 2 / 3）→ `step()` → 快照注入
/// 黄金 / `town_halls` 分类登记 / 独立实例 ID 段起点 → `restore_snapshot()`。
pub fn base_match(gold: i64, with_town_hall: bool) -> (RtsMatch, u64) {
    let frozen = defs();
    let content = frozen.content_hash();
    let mut sim =
        RtsMatch::create_with_definitions(config(&frozen), StaticWorld::default(), frozen);
    for sequence in 1..=3u32 {
        sim.submit_command(spawn_at_shop(sequence)).unwrap();
    }
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    snapshot.gameplay.players.entry(PLAYER).or_default().gold = gold;
    // 实例 id 与注入段隔离，保证购买 / 掉落 / 手工注入身份不冲突。
    snapshot.creep_camps.next_ground_item_id = GROUND_ITEM_ID_BASE + 1000;
    // 友方主城放在与商店 / 英雄不同的确定性整数坐标，便于观察单英雄传送。
    if let Some(hall) = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == TOWN_HALL_ID)
    {
        hall.x = 5 * SCALE;
        hall.y = 5 * SCALE;
    }
    if with_town_hall {
        snapshot
            .item_shop
            .town_halls
            .insert(EntityId::new(TOWN_HALL_ID).unwrap(), TOWN_HALL_DEF);
    }
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    (restored, content)
}

/// 经真实 `restore_snapshot` 生产路径注入任意只读快照改动。
pub fn with_state<F>(sim: &RtsMatch, content: u64, mutate: F) -> RtsMatch
where
    F: FnOnce(&mut w3_core::Snapshot),
{
    let mut snapshot = sim.capture_snapshot();
    mutate(&mut snapshot);
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

/// 六格背包注入用的稳定实例 `EntityId`（`0..6` 槽位）。
pub fn bag_instance(slot: u8) -> EntityId {
    EntityId::new(GROUND_ITEM_ID_BASE + 100 + u64::from(slot)).unwrap()
}

pub fn test_effect() -> EffectDef {
    EffectDef {
        kind: EffectKind::Heal,
        amount: 10,
        duration_frames: 0,
    }
}

/// 给英雄注入六格定长实例背包（同名道具多实例独立充能 / 冷却，R7.5）。
pub fn inject_bag(snapshot: &mut w3_core::Snapshot, hero: EntityId, item_defs: &[DefId]) {
    let mut bag = ItemBag::default();
    for (slot, &item_def) in item_defs.iter().enumerate() {
        let instance = bag_instance(slot as u8);
        bag.set(slot as u8, Some(instance));
        snapshot.creep_camps.item_instances.insert(
            instance,
            ItemInstanceState {
                item_def,
                charges: slot as u32 + 1,
                cooldown_frames: 0,
                effect: test_effect(),
                holder: Some(hero),
                slot: Some(slot as u8),
            },
        );
    }
    snapshot.item_shop.bags.insert(hero, bag);
}

/// 注入一件无主地面物品（含独立实例、充能、冷却与中立登记，R1.1 / R7.2）。
pub fn inject_ground(
    snapshot: &mut w3_core::Snapshot,
    id: EntityId,
    item_def: DefId,
    charges: u32,
    cooldown_frames: u32,
) {
    snapshot.creep_camps.ground_items.insert(
        id,
        GroundItemState {
            item_def,
            camp: 0,
            x: SHOP_X,
            y: SHOP_Y,
            instance: id,
        },
    );
    snapshot.creep_camps.item_instances.insert(
        id,
        ItemInstanceState {
            item_def,
            charges,
            cooldown_frames,
            effect: test_effect(),
            holder: None,
            slot: None,
        },
    );
    snapshot
        .creep_camps
        .neutral
        .insert(id, NeutralCategory::GroundItemOwnerless);
}

pub fn submit(
    sim: &mut RtsMatch,
    frame: u64,
    player: u8,
    sequence: u32,
    kind: GameplayCommandKind,
) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        player,
        sequence,
        kind,
    ))
    .unwrap();
}

pub fn step_n(sim: &mut RtsMatch, frames: usize) {
    for _ in 0..frames {
        sim.step();
    }
}

pub fn gold(sim: &RtsMatch) -> i64 {
    sim.capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .map(|economy| economy.gold)
        .unwrap_or(0)
}

pub fn hero() -> EntityId {
    EntityId::new(HERO_ID).unwrap()
}

pub fn peer() -> EntityId {
    EntityId::new(PEER_ID).unwrap()
}

pub fn town_hall() -> EntityId {
    EntityId::new(TOWN_HALL_ID).unwrap()
}

pub fn ground() -> EntityId {
    EntityId::new(GROUND_ID).unwrap()
}

/// 拾取命令（码 15）。
pub fn pickup(frame: u64, sequence: u32, ground_item: EntityId) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        GameplayCommandKind::PickupGroundItem {
            hero: hero(),
            ground_item,
        },
    )
}

/// 购买命令（码 16）。
pub fn buy(frame: u64, sequence: u32, shelf: u32) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        GameplayCommandKind::BuyFromShop {
            hero: hero(),
            shop: SHOP,
            shelf,
        },
    )
}

/// 市场刷新系统命令（码 23，`NEUTRAL_OWNER`）。
pub fn refresh(frame: u64, sequence: u32, batch: u32) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        NEUTRAL_OWNER,
        sequence,
        GameplayCommandKind::RefreshMarket { shop: SHOP, batch },
    )
}

/// 含拾取 / 购买 / 使用 / 售卖 / 回城 / 刷新的确定性脚本（双实例 / 回放共用）。
pub fn script() -> Vec<GameplayCommand> {
    vec![
        pickup(1, 1, ground()),
        buy(2, 1, SHELF_CONSUMABLE),
        GameplayCommand::new(
            Frame::new(4),
            PLAYER,
            1,
            GameplayCommandKind::DropItem {
                hero: hero(),
                instance: bag_instance(0),
            },
        ),
        refresh(6, 1, 41),
        refresh(6, 2, 41), // 同批次重复投递，幂等不重复消费。
    ]
}
