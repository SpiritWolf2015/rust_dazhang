//! P8.5a 英雄复活：只读 HUD 派生与真实装配 CPU 回归（tasks 6.1 / 6.2 / D / ADR 0002 §5）。
//!
//! 全部为合成命令脚本与真实 `synthetic_definitions()` 装配，零资产、零文件 IO、
//! 不开 GPU 窗口、不引入墙钟。核心断言：
//! - 真实装配 `is_altar` / `is_hero` 为真、英雄经既有 `Train` 路径产出
//!   `Some(HeroState)`、`ReviveHero` 命令在真实装配下被接受（Q3 / R1.2 / R13.3）；
//! - 祭坛复活队列进度 / 费用 / 剩余帧的 HUD 派生只从只读 `Snapshot.revive`
//!   与 `HeroDied` / `Revived` 事实事件拷贝出发（ADR 0002 §5 / D7）；
//! - HUD 连续重绘时逐帧 `compute_state_hash()` / `capture_snapshot()` 字节
//!   与无 HUD 实例一致（`isolation.rs` 风格双实例对照）；
//! - 反向验证（改坏验红）：「表现层反写权威状态」的退化实现经真实公开
//!   提交入口注入，必被「双实例逐帧哈希一致」断言拦截（CODEBUDDY §5）。
//!
//! 本文件只覆盖 CPU / 离屏自证；真窗口人工清单 8.x 保持「未执行 / 待人工」。

use std::collections::BTreeSet;

use w3_client::melee::defs::{HUMAN_ALTAR, HUMAN_HERO, ORC_ALTAR, ORC_HERO, synthetic_definitions};
use w3_client::melee::synthetic_world;
use w3_client::{HudAssets, HudModel, ReviveHudModel, ScreenMapper, ZH};
use w3_core::{
    AbilityDef, Command, CommandKind, DefId, EntityId, Event, EventKind, Frame, FrozenDefinitions,
    GameplayCommand, GameplayCommandKind, HeroDeathRecord, HeroState, ItemDef, MatchConfig,
    ReviveConstants, RtsMatch, SNAPSHOT_VERSION, SYNTHETIC_REVIVE_CONSTANTS, SimFixed, Snapshot,
    StaticWorld, UnitDef, UpgradeDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
const HERO_ID: u64 = 1;
const ALTAR_ID: u64 = 2;
const HERO_HP: i64 = 400;
const HERO_LEVEL: u32 = 5;
const HERO_XP: i64 = 4321;

fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("reason=fixed_conversion_failed")
}

/// 由整数字面量构造 `Spawn` 命令；`speed > 0` 满足既有提交校验。
fn spawn(x: i64, y: i64, hp: i64, sequence: u32) -> Command {
    Command::new(
        Frame::new(0),
        PLAYER,
        sequence,
        CommandKind::Spawn {
            x: fixed(x),
            y: fixed(y),
            hp,
            speed: fixed(3),
            attack_damage: 0,
            attack_range: fixed(10),
        },
    )
}

/// 与 `synthetic_definitions()` 同源、但**未**登记复活常量段的退化定义（缺陷态）。
/// 经真实 [`FrozenDefinitions::new`] 重建，非旁路模拟（CODEBUDDY §5 / D8）。
/// P8.5b：四张定义表均从 `full` 的公开 getter 克隆，新增 8101 / 8102
/// 真实定义自动同源；再同源登记主城升级段，使缺陷态仍只差「未登记复活段」
/// 这一项，避免 `content_hash()` 判别力被新增段差异稀释
/// （CODEBUDDY §5 / 不静默降级）。P8.5c：同源登记野怪营地段，保持该
/// 缺陷态仍只差「未登记复活段」，不让新冻结段稀释判别力。
fn degenerate_definitions() -> FrozenDefinitions {
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
        // P8.5d：同源登记商店段，使缺陷态仍只差复活段。
        .with_item_shop(full.item_shop().clone())
}

/// 真实装配 + 快照往返注入「已阵亡英雄 + 死亡登记 + 祭坛 + 充足经济」。
///
/// 注入走 `capture_snapshot()` → 修改只读快照的 public 字段 →
/// `restore_snapshot()` 这条既有确定性生产路径（先例
/// `crates/w3-core/tests/hero_revive.rs::dead_hero_match`），非手搓假结构。
/// 返回 `(对局, 冻结定义, 世界, 内容哈希, 英雄 EntityId, 祭坛 EntityId)`。
fn dead_hero_fixture() -> (
    RtsMatch,
    FrozenDefinitions,
    StaticWorld,
    u64,
    EntityId,
    EntityId,
) {
    let defs = synthetic_definitions();
    let content = defs.content_hash();
    let world = synthetic_world();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, world.clone(), defs.clone());
    sim.submit_command(spawn(0, 0, HERO_HP, 1))
        .expect("reason=hero_spawn_rejected");
    sim.submit_command(spawn(5, 0, 300, 2))
        .expect("reason=altar_spawn_rejected");
    sim.step();

    let mut snapshot = sim.capture_snapshot();
    let hero_pos = snapshot
        .positions
        .iter_mut()
        .find(|position| position.entity == HERO_ID)
        .expect("reason=hero_position_present");
    hero_pos.alive = false;
    hero_pos.hp = 0;
    hero_pos.gameplay.hero = Some(HeroState {
        level: HERO_LEVEL,
        xp: HERO_XP,
    });
    let hero_gameplay = hero_pos.gameplay.clone();
    let hero = EntityId::new(HERO_ID).expect("reason=invalid_hero_id");
    snapshot.revive.deaths.insert(
        hero,
        HeroDeathRecord {
            death_frame: 0,
            owner: PLAYER,
            gameplay: hero_gameplay,
            max_hp: HERO_HP,
        },
    );
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    economy.lumber = 100_000;

    let restored = RtsMatch::restore_snapshot(snapshot, world.clone(), content)
        .expect("reason=dead_hero_restore_failed");
    (
        restored,
        defs,
        world,
        content,
        hero,
        EntityId::new(ALTAR_ID).expect("reason=invalid_altar_id"),
    )
}

/// 提交一条 `ReviveHero` 命令（执行帧由当前只读视图的冻结帧号推导）。
///
/// 用当前 `tick` 而非 `tick + 1`：内核 `step()` 的阶段 7 以
/// `freeze_frame(clock.current())` 取 `execute_frame <= frame` 的命令，故提交到
/// 当前冻结帧可在下一次 `step()` 立即执行（与 `hero_revive.rs` 的 `Frame::new(1)` 同口径）。
fn submit_revive(sim: &mut RtsMatch, hero: EntityId, altar: EntityId, altar_def: DefId) {
    let frame = Frame::new(sim.read_view().tick);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        1,
        GameplayCommandKind::ReviveHero {
            hero,
            altar,
            altar_def,
        },
    ))
    .expect("reason=revive_command_rejected");
}

/// 稳定字节视图：内核未暴露快照字节编码，用 `Debug` 表示作为可观测代理，
/// 配合 `Snapshot: PartialEq` 逐字段相等断言（与 `interaction.rs` 同款）。
fn snapshot_bytes(snapshot: &Snapshot) -> Vec<u8> {
    format!("{snapshot:?}").into_bytes()
}

/// 被反向验证的隔离断言：HUD 派生 / 重绘后，权威状态哈希与快照字节不变。
fn hud_isolation_holds(rendered: &RtsMatch, plain: &RtsMatch) -> bool {
    rendered.compute_state_hash() == plain.compute_state_hash()
        && snapshot_bytes(&rendered.capture_snapshot()) == snapshot_bytes(&plain.capture_snapshot())
}

/// 退化实现（缺陷态）：模拟「表现层反写权威状态」——经真实公开提交入口
/// `submit_command` 提交一条 `Noop`，改变 `pending_commands` / `next_arrival`。
/// 这是确定性生产路径注入，不是旁路改一个假布尔值（CODEBUDDY §5 / D8）。
fn degenerate_writeback(sim: &mut RtsMatch, frame: u64) {
    sim.submit_command(Command::new(
        Frame::new(frame),
        PLAYER,
        9_000,
        CommandKind::Noop,
    ))
    .expect("reason=degenerate_writeback_command_rejected");
}

/// 英雄经既有 `Train` 路径产出后，返回新实体的 `HeroState`（真实装配）。
fn train_probes_hero_state(defs: &FrozenDefinitions) -> Option<HeroState> {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    sim.submit_command(spawn(0, 0, 100, 1))
        .expect("reason=producer_spawn_rejected");
    sim.step();
    let producer = EntityId::new(1).expect("reason=invalid_producer_id");
    let frame = Frame::new(sim.read_view().tick.saturating_add(1));
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        1,
        GameplayCommandKind::Train {
            producer,
            unit_def: HUMAN_HERO,
            is_building: false,
        },
    ))
    .expect("reason=train_command_rejected");
    // `HUMAN_HERO.train_frames = 60`，足额推进到完成帧之后。
    for _ in 0..64 {
        sim.step();
    }
    let snapshot = sim.capture_snapshot();
    let produced = snapshot
        .positions
        .iter()
        .filter(|position| position.entity > 1)
        .max_by_key(|position| position.entity)
        .expect("reason=train_produced_entity");
    produced.gameplay.hero
}

// ---------------------------------------------------------------------------
// 任务 D：真实装配接入 with_hero_revive 的 CPU 证据
// ---------------------------------------------------------------------------

#[test]
fn synthetic_definitions_registers_altar_and_hero_categories() {
    let defs = synthetic_definitions();
    // 两族祭坛 / 英雄类别由定义层 `BTreeSet<DefId>` 驱动，非建筑名白名单。
    assert!(defs.is_altar(HUMAN_ALTAR), "reason=human_altar_registered");
    assert!(defs.is_altar(ORC_ALTAR), "reason=orc_altar_registered");
    assert!(defs.is_hero(HUMAN_HERO), "reason=human_hero_registered");
    assert!(defs.is_hero(ORC_HERO), "reason=orc_hero_registered");
    assert!(!defs.is_hero(HUMAN_ALTAR), "reason=categories_not_crossed");
    assert_eq!(
        defs.altar_defs(),
        &[HUMAN_ALTAR, ORC_ALTAR]
            .into_iter()
            .collect::<BTreeSet<DefId>>(),
        "reason=altar_defs_exact"
    );
    assert_eq!(
        defs.hero_defs(),
        &[HUMAN_HERO, ORC_HERO]
            .into_iter()
            .collect::<BTreeSet<DefId>>(),
        "reason=hero_defs_exact"
    );
    // 冻结复活常量随等级确定性变化，且常量段编入内容哈希（Q3 / R1.2）。
    let constants: ReviveConstants = *defs.revive_constants();
    assert_eq!(constants, SYNTHETIC_REVIVE_CONSTANTS);
    let low = defs.revive_cost(1);
    let high = defs.revive_cost(5);
    assert!(high.gold >= low.gold, "reason=gold_scales_with_level");
    assert!(high.frames >= low.frames, "reason=frames_non_decreasing");

    // Q1 口径：接入 revive 段 + 类别集合后内容哈希必变，不沿用旧值、
    // 不声称逐字节不变；退化定义（未登记）与正式定义同源可对照。
    let degenerate = degenerate_definitions();
    assert!(
        !degenerate.is_altar(HUMAN_ALTAR),
        "reason=degenerate_altar_false"
    );
    assert!(
        !degenerate.is_hero(HUMAN_HERO),
        "reason=degenerate_hero_false"
    );
    assert_ne!(
        defs.content_hash(),
        degenerate.content_hash(),
        "reason=revive_defs_hash_changed"
    );
}

#[test]
fn train_produces_some_hero_state_only_with_hero_registration() {
    // 修复态：真实装配登记了英雄类别，`Train` 产出 `Some(HeroState)`。
    let formal = train_probes_hero_state(&synthetic_definitions());
    assert_eq!(
        formal,
        Some(HeroState { level: 1, xp: 0 }),
        "reason=formal_train_produces_some_hero_state"
    );

    // 缺陷态：未登记英雄类别的退化定义下 `hero = None`，证明接入是复活链路
    // 可达的必要条件（未接入时英雄复活是死代码，A-B 元断言防退化）。
    let degenerate = train_probes_hero_state(&degenerate_definitions());
    assert_eq!(
        degenerate, None,
        "reason=degenerate_train_produces_no_hero_state"
    );
}

#[test]
fn revive_command_is_accepted_in_real_assembly() {
    let (mut sim, defs, _world, _content, hero, altar) = dead_hero_fixture();
    submit_revive(&mut sim, hero, altar, HUMAN_ALTAR);
    sim.step();

    let snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v7");
    assert_eq!(SNAPSHOT_VERSION, 7, "reason=snapshot_version_7");
    let item = snapshot
        .revive
        .queue
        .get(&hero)
        .expect("reason=revive_queue_present");
    let cost = defs.revive_cost(HERO_LEVEL);
    assert_eq!(item.hero, hero, "reason=queue_hero");
    assert_eq!(item.altar, altar, "reason=queue_altar");
    assert_eq!(item.altar_def, HUMAN_ALTAR, "reason=queue_altar_def");
    assert_eq!(item.owner, PLAYER, "reason=queue_owner");
    assert_eq!(item.gold, cost.gold, "reason=queue_gold");
    assert_eq!(item.lumber, cost.lumber, "reason=queue_lumber");
    assert_eq!(item.total_frames, cost.frames, "reason=queue_total_frames");
    assert!(item.remaining_frames > 0, "reason=queue_progress_remaining");
    assert!(
        snapshot.revive.charges.contains_key(&hero),
        "reason=charge_present"
    );
    assert!(
        snapshot.revive.deaths.contains_key(&hero),
        "reason=death_present"
    );
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_matches_real_assembly"
    );

    // 定义层类别驱动：未登记的 `altar_def` 命令被显式拒绝、不免费复活。
    let (mut other, _defs, _world, _content, hero2, altar2) = dead_hero_fixture();
    let gold_before = other
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .map_or(0, |economy| economy.gold);
    let frame = Frame::new(other.read_view().tick);
    other
        .submit_gameplay_command(GameplayCommand::new(
            frame,
            PLAYER,
            1,
            GameplayCommandKind::ReviveHero {
                hero: hero2,
                altar: altar2,
                altar_def: 9_999,
            },
        ))
        .expect("reason=unknown_altar_command_submitted");
    other.step();
    let snap = other.capture_snapshot();
    assert!(
        snap.revive.queue.is_empty(),
        "reason=unknown_altar_rejected"
    );
    assert!(
        snap.revive.charges.is_empty(),
        "reason=unknown_altar_no_charge"
    );
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).map_or(0, |e| e.gold),
        gold_before,
        "reason=unknown_altar_no_deduction"
    );
}

// ---------------------------------------------------------------------------
// 任务 C：只读 HUD 派生、连续重绘隔离回归与反向验证
// ---------------------------------------------------------------------------

#[test]
fn revive_hud_derives_only_from_readonly_snapshot_and_events() {
    let (mut sim, _defs, _world, _content, hero, altar) = dead_hero_fixture();
    submit_revive(&mut sim, hero, altar, HUMAN_ALTAR);
    sim.step();

    let snapshot = sim.capture_snapshot();
    let events = vec![
        Event {
            frame: 1,
            sequence: 10,
            entity: HERO_ID,
            kind: EventKind::HeroDied,
        },
        Event {
            frame: 1,
            sequence: 11,
            entity: HERO_ID,
            kind: EventKind::HeroDied,
        },
        Event {
            frame: 2,
            sequence: 12,
            entity: HERO_ID,
            kind: EventKind::Revived,
        },
        Event {
            frame: 2,
            sequence: 13,
            entity: HERO_ID,
            kind: EventKind::Spawned,
        },
    ];
    let projection = ReviveHudModel::derive(&snapshot, &events);

    // 队列项逐字段来自只读 `Snapshot.revive.queue`，不带权威句柄。
    assert_eq!(projection.queue.len(), snapshot.revive.queue.len());
    let source = snapshot
        .revive
        .queue
        .get(&hero)
        .expect("reason=queue_source");
    let derived = projection
        .queue
        .iter()
        .find(|item| item.hero == HERO_ID)
        .expect("reason=derived_queue_item");
    assert_eq!(derived.altar, ALTAR_ID);
    assert_eq!(derived.altar_def, HUMAN_ALTAR);
    assert_eq!(derived.owner, PLAYER);
    assert_eq!(derived.remaining_frames, source.remaining_frames);
    assert_eq!(derived.total_frames, source.total_frames);
    assert_eq!(derived.gold, source.gold);
    assert_eq!(derived.lumber, source.lumber);
    assert_eq!(projection.deaths, snapshot.revive.deaths.len());
    assert_eq!(projection.charges, snapshot.revive.charges.len());
    assert_eq!(
        projection.hero_died_events, 2,
        "reason=hero_died_event_count"
    );
    assert_eq!(projection.revived_events, 1, "reason=revived_event_count");

    // 纯函数：同一只读输入重复派生结果完全一致，且 `HudModel::collect`
    // 产出的投影与独立派生相同（只读通路同源）。
    assert_eq!(projection, ReviveHudModel::derive(&snapshot, &events));
    let view = sim.read_view();
    let selection = BTreeSet::new();
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let model = HudModel::collect(&view, &snapshot, &selection, &events, &mapper, PLAYER);
    assert_eq!(
        model.revive, projection,
        "reason=hud_collect_revive_matches_derive"
    );

    // 空快照 / 空事件派生空投影，无墙钟、无全局静态字典、无建筑名白名单。
    let empty = ReviveHudModel::derive(
        &RtsMatch::create_with_world(
            MatchConfig {
                seed: SEED,
                logic_fps: 30,
                content_hash: 0,
            },
            StaticWorld::default(),
        )
        .capture_snapshot(),
        &[],
    );
    assert_eq!(empty, ReviveHudModel::default(), "reason=empty_projection");
}

#[test]
fn hud_redraw_keeps_state_hash_and_snapshot_bytes() {
    let (mut rendered, _defs, _world, _content, hero, altar) = dead_hero_fixture();
    let (mut plain, _defs2, _world2, _content2, hero_plain, altar_plain) = dead_hero_fixture();
    submit_revive(&mut rendered, hero, altar, HUMAN_ALTAR);
    submit_revive(&mut plain, hero_plain, altar_plain, HUMAN_ALTAR);

    let ctx = egui::Context::default();
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let selection = BTreeSet::new();

    for frame in 0..16 {
        rendered.step();
        plain.step();

        let view = rendered.read_view();
        let snapshot = rendered.capture_snapshot();
        let events = rendered.drain_events();
        let _ = plain.drain_events();

        // 只有 rendered 一侧反复派生 HUD 数据并用 egui 绘制（纯 CPU，无 GPU）。
        let model = HudModel::collect(&view, &snapshot, &selection, &events, &mapper, PLAYER);
        assert!(
            !model.revive.queue.is_empty(),
            "reason=revive_queue_visible frame={frame}"
        );
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            w3_client::draw(ctx, &model, &ZH, HudAssets { text_enabled: true });
        });
        let _drawn_shapes = output.shapes.len();
        output.drop_without_applying_deltas();

        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "reason=hud_redraw_changed_state_hash frame={frame}"
        );
        assert_eq!(
            snapshot_bytes(&rendered.capture_snapshot()),
            snapshot_bytes(&plain.capture_snapshot()),
            "reason=hud_redraw_changed_snapshot_bytes frame={frame}"
        );
        assert_eq!(
            rendered.capture_snapshot().version,
            SNAPSHOT_VERSION,
            "reason=snapshot_version_7 frame={frame}"
        );
    }
}

#[test]
fn reverse_validation_revive_hud_writeback_fails_isolation_assertion() {
    let (mut rendered, _defs, _world, _content, hero, altar) = dead_hero_fixture();
    let (mut plain, _defs2, _world2, _content2, hero_plain, altar_plain) = dead_hero_fixture();
    submit_revive(&mut rendered, hero, altar, HUMAN_ALTAR);
    submit_revive(&mut plain, hero_plain, altar_plain, HUMAN_ALTAR);
    rendered.step();
    plain.step();

    // 修复态：正式只读派生（只消费 `&Snapshot` / `&[Event]`）不改权威状态，
    // 双实例逐帧哈希 / 快照字节一致，隔离断言通过。
    let snapshot = rendered.capture_snapshot();
    let events = rendered.drain_events();
    let _ = plain.drain_events();
    let _projection = ReviveHudModel::derive(&snapshot, &events);
    assert!(
        hud_isolation_holds(&rendered, &plain),
        "reason=formal_read_only_projection_passes"
    );

    // 缺陷态：退化实现反写权威状态（经真实 `submit_command` 生产入口），
    // 逐帧哈希 / 快照字节漂移，隔离断言必失败 —— 元断言防退化。
    let writeback_frame = rendered.read_view().tick.saturating_add(1);
    degenerate_writeback(&mut rendered, writeback_frame);
    assert!(
        !hud_isolation_holds(&rendered, &plain),
        "reason=degenerate_writeback_rejected_by_isolation_assertion"
    );
}
