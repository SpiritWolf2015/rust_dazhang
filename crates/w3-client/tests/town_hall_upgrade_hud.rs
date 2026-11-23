//! P8.5b 主城升级：只读 HUD 派生与真实装配 CPU 回归（tasks 6.1 / 6.2 / D6 / ADR 0002 §5）。
//!
//! 全部为合成命令脚本与真实 `synthetic_definitions()` 装配，零资产、零文件 IO、
//! 不开 GPU 窗口、不引入墙钟。核心断言：
//! - 真实装配 `is_town_hall` / `tier_requirement` / `TownHallUpgradeState` 与
//!   `UpgradeTownHall` 命令经既有提交入口被接受（Q3 / Q4 / R1.2 / R13.3）；
//! - 真实装配把 `8xxx` 两条 T2 需求链 DefId 注册为真实 `UnitDef` /
//!   `UpgradeDef`；T1 拒绝 `Train` / `Research`，T2 完成后生成实体、结算科技，
//!   双实例同脚本逐帧 `compute_state_hash()` / `capture_snapshot()` 一致；
//! - 主城升级队列进度 / 费用 / 剩余帧 / tier 的 HUD 派生只从只读
//!   `Snapshot.town_hall` 与三个事实事件拷贝出发（ADR 0002 §5 / D7）；
//! - HUD 连续重绘 16 帧逐帧 `compute_state_hash()` / `capture_snapshot()` 字节
//!   与无 HUD 实例一致（`isolation.rs` 风格双实例对照）；
//! - 升级命令卡格位由冻结定义数据驱动，以非经典命名 DefId 仍按需求链显示，
//!   建筑名 / DefId 白名单退化实现令回归失败（R13.3 / C15 / Q6）；
//! - 反向验证（改坏验红）：「表现层反写权威状态」退化实现经真实公开提交入口
//!   注入，必被「双实例逐帧哈希一致」断言拦截（CODEBUDDY §5）。
//!
//! 本文件只覆盖 CPU / 离屏自证；真窗口人工清单 8.x 保持「未执行 / 待人工」。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::melee::defs::{
    HUMAN_BARRACKS, HUMAN_TOWN_HALL, ORC_TOWN_HALL, SOLDIER, TIER2_TECH, TIER2_UNIT, TOWN_HALL,
    TOWN_HALL_PORTAL_DEF, synthetic_definitions,
};
use w3_client::melee::synthetic_world;
use w3_client::{
    HudAssets, HudModel, HudUpgradeItem, ScreenMapper, TownHallUpgradeHudModel, UpgradeSlot, ZH,
    derive_upgrade_slot, draw,
};
use w3_core::{
    AbilityDef, Command, CommandKind, DefId, EntityId, Event, EventKind, Frame, FrozenDefinitions,
    GameplayCommand, GameplayCommandKind, ItemDef, MatchConfig, ProductionItem, RtsMatch,
    SNAPSHOT_VERSION, SYNTHETIC_TOWN_HALL_CONSTANTS, SimFixed, Snapshot, StaticWorld,
    TownHallConstants, TownHallTier, TownHallUpgradeState, UnitDef, UpgradeDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
const TOWN_HALL_ID: u64 = 1;
const PRODUCER_ID: u64 = 2;

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

/// 与 `synthetic_definitions()` 同源、但**未**登记主城升级段的退化定义（缺陷态）。
/// 经真实 [`FrozenDefinitions::new`] 重建，非旁路模拟（CODEBUDDY §5 / D8）。
/// P8.5b：四张定义表均从 `full` 的公开 getter 克隆，新增 8101 / 8102
/// 真实定义自动同源；缺陷态仍**故意不登记**主城升级段，使 A-B 对照只差
/// 该段（`with_town_hall_upgrade`），不误伤反向验证设计意图。
/// P8.5c：同源登记野怪营地段，使缺陷态仍只差「主城升级段」。
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
        // 同源登记复活段，使缺陷态只差「主城升级段」这一项。
        .with_hero_revive(
            *full.revive_constants(),
            full.altar_defs().clone(),
            full.hero_defs().clone(),
        )
        .with_creep_camps(
            *full.creep_constants(),
            full.camp_scripts().clone(),
            full.drop_tables().clone(),
            full.neutral_categories().clone(),
        )
        // P8.5d：同源登记商店段，使缺陷态仍只差主城升级段。
        .with_item_shop(full.item_shop().clone())
}

/// 真实装配 + 快照往返注入「主城实体 + 充足经济」。
///
/// 注入走 `capture_snapshot()` → 修改只读快照的 public 字段 →
/// `restore_snapshot()` 这条既有确定性生产路径（先例
/// `crates/w3-core/tests/hero_revive.rs::dead_hero_fixture`），非手搓假结构。
/// 返回 `(对局, 冻结定义, 世界, 内容哈希, 主城 EntityId)`。
fn upgrade_fixture() -> (RtsMatch, FrozenDefinitions, StaticWorld, u64, EntityId) {
    let defs = synthetic_definitions();
    let content = defs.content_hash();
    let world = synthetic_world();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, world.clone(), defs.clone());
    sim.submit_command(spawn(0, 0, 500, 1))
        .expect("reason=town_hall_spawn_rejected");
    sim.step();

    let mut snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v8");
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    economy.lumber = 100_000;

    let restored = RtsMatch::restore_snapshot(snapshot, world.clone(), content)
        .expect("reason=upgrade_fixture_restore_failed");
    (
        restored,
        defs,
        world,
        content,
        EntityId::new(TOWN_HALL_ID).expect("reason=invalid_town_hall_id"),
    )
}

/// 用指定冻结定义装配 T2 门槛验收夹具：主城实体、合法生产者实体与充足经济。
///
/// `CommandKind::Spawn` 只携带冻结初值、不携带 `DefId`，故生产者合法性由
/// `is_actionable` 的存活 / 归属 / 无眩晕三项判据直接核实；`HUMAN_BARRACKS`
/// 真实冻结定义只用于确认装配层建筑定义存在且不可移动，MUST NOT 写入权威状态。
fn tier_gate_fixture_with_defs(
    defs: FrozenDefinitions,
) -> (
    RtsMatch,
    FrozenDefinitions,
    StaticWorld,
    u64,
    EntityId,
    EntityId,
) {
    let barracks = defs
        .units()
        .get(&HUMAN_BARRACKS)
        .expect("reason=real_barracks_def_present");
    assert_eq!(barracks.speed_raw, 0, "reason=barracks_def_immovable");
    assert!(
        defs.units().contains_key(&TIER2_UNIT),
        "reason=tier2_unit_def_present"
    );
    assert!(
        defs.upgrades().contains_key(&TIER2_TECH),
        "reason=tier2_tech_def_present"
    );

    let content = defs.content_hash();
    let world = synthetic_world();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, world.clone(), defs.clone());
    sim.submit_command(spawn(0, 0, 500, 1))
        .expect("reason=town_hall_spawn_rejected");
    sim.submit_command(spawn(4, 0, 300, 2))
        .expect("reason=producer_spawn_rejected");
    sim.step();

    let mut snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v8");
    {
        let hall = snapshot
            .positions
            .iter()
            .find(|position| position.entity == TOWN_HALL_ID)
            .expect("reason=town_hall_position_present");
        assert!(
            hall.alive && hall.owner == PLAYER && hall.gameplay.buffs.is_empty(),
            "reason=town_hall_is_actionable"
        );
        let producer = snapshot
            .positions
            .iter()
            .find(|position| position.entity == PRODUCER_ID)
            .expect("reason=producer_position_present");
        assert!(
            producer.alive && producer.owner == PLAYER && producer.gameplay.buffs.is_empty(),
            "reason=producer_is_actionable"
        );
    }
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    economy.lumber = 100_000;

    let restored = RtsMatch::restore_snapshot(snapshot, world.clone(), content)
        .expect("reason=tier_gate_fixture_restore_failed");
    (
        restored,
        defs,
        world,
        content,
        EntityId::new(TOWN_HALL_ID).expect("reason=invalid_town_hall_id"),
        EntityId::new(PRODUCER_ID).expect("reason=invalid_producer_id"),
    )
}

/// 正式真实装配的 T2 门槛夹具；主城类别必须已登记。
fn tier_gate_fixture() -> (
    RtsMatch,
    FrozenDefinitions,
    StaticWorld,
    u64,
    EntityId,
    EntityId,
) {
    let defs = synthetic_definitions();
    assert!(
        defs.is_town_hall(HUMAN_TOWN_HALL),
        "reason=real_hall_registered"
    );
    tier_gate_fixture_with_defs(defs)
}

/// 提交一条 `Train` 命令；执行帧由当前只读视图的冻结帧号推导。
fn submit_train(sim: &mut RtsMatch, producer: EntityId, unit_def: DefId, sequence: u32) {
    let frame = Frame::new(sim.read_view().tick);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        sequence,
        GameplayCommandKind::Train {
            producer,
            unit_def,
            is_building: false,
        },
    ))
    .expect("reason=train_command_rejected");
}

/// 提交一条 `Research` 命令；执行帧由当前只读视图的冻结帧号推导。
fn submit_research(sim: &mut RtsMatch, upgrade: DefId, sequence: u32) {
    let frame = Frame::new(sim.read_view().tick);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        sequence,
        GameplayCommandKind::Research { upgrade },
    ))
    .expect("reason=research_command_rejected");
}

/// 等量 no-op 玩法命令：用于对齐玩法命令队列到达序号，不产生权威状态变化。
fn submit_noop_cancel_train(sim: &mut RtsMatch, producer: EntityId, sequence: u32) {
    let frame = Frame::new(sim.read_view().tick);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        sequence,
        GameplayCommandKind::CancelTrain {
            producer,
            index: u32::MAX,
        },
    ))
    .expect("reason=noop_command_rejected");
}

/// 同一 T2 解锁命令脚本：训练 8101 单位 + 研发 8102 科技。
fn submit_unlock_script(sim: &mut RtsMatch, producer: EntityId, start_sequence: u32) {
    submit_train(sim, producer, TIER2_UNIT, start_sequence);
    submit_research(sim, TIER2_TECH, start_sequence.saturating_add(1));
}

/// 生产者当前生产队列（`Snapshot.positions` 的只读值拷贝）。
fn producer_production(sim: &RtsMatch, producer: EntityId) -> Vec<ProductionItem> {
    let id = producer.value();
    sim.capture_snapshot()
        .positions
        .into_iter()
        .find(|position| position.entity == id)
        .expect("reason=producer_position_present")
        .gameplay
        .production
}

/// 玩家当前研发队列（`PlayerEconomy.research` 的只读值拷贝）。
fn current_research(sim: &RtsMatch) -> Option<(DefId, u32)> {
    sim.capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .and_then(|economy| economy.research)
}

/// 主城当前权威 tier；`None` 表示尚未登记升级（等价 T1 基线）。
fn tier_of_town_hall(sim: &RtsMatch, hall: EntityId) -> Option<TownHallTier> {
    sim.capture_snapshot().town_hall.tiers.get(&hall).copied()
}

/// 同脚本双实例逐帧哈希 / 快照 / 首差异字段诊断一致。
fn assert_dual_state_matches(a: &RtsMatch, b: &RtsMatch, phase: &str, frame: usize) {
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason={phase}_frame_{frame}_hash_match field={:?}",
        a.first_difference(&b)
    );
    assert_eq!(
        a.first_difference(&b),
        None,
        "reason={phase}_frame_{frame}_no_diff"
    );
    assert_eq!(
        a.capture_snapshot(),
        b.capture_snapshot(),
        "reason={phase}_frame_{frame}_snapshot_match"
    );
}

/// 双实例同时推进到 `TownHallUpgradeCompleted`，逐帧比对同一升级脚本。
fn advance_until_upgraded_dual(primary: &mut RtsMatch, replay: &mut RtsMatch, hall: EntityId) {
    let frames = primary
        .definitions()
        .town_hall_constants()
        .cost(TownHallTier::T2)
        .expect("reason=tier2_cost_present")
        .frames;
    let window = frames.saturating_add(16) as usize;
    primary.drain_events();
    replay.drain_events();
    let mut done = false;
    for frame in 0..window {
        primary.step();
        replay.step();
        assert_dual_state_matches(primary, replay, "upgrade", frame);
        let events_a = primary.drain_events();
        let events_b = replay.drain_events();
        assert_eq!(
            events_a, events_b,
            "reason=upgrade_frame_{frame}_events_match"
        );
        if events_a
            .iter()
            .any(|event| event.kind == EventKind::TownHallUpgradeCompleted)
        {
            done = true;
            break;
        }
    }
    assert!(done, "reason=upgrade_never_completed");
    assert_eq!(
        tier_of_town_hall(primary, hall),
        Some(TownHallTier::T2),
        "reason=primary_tier_is_t2"
    );
    assert_eq!(
        tier_of_town_hall(replay, hall),
        Some(TownHallTier::T2),
        "reason=replay_tier_is_t2"
    );
}

/// T2 解锁效果是否全部结算：科技旗标、实体生成、队列清空。
fn tier2_effects_settled(sim: &RtsMatch, producer: EntityId, before_alive: usize) -> bool {
    let snapshot = sim.capture_snapshot();
    let upgrade_done = snapshot
        .gameplay
        .players
        .get(&PLAYER)
        .and_then(|economy| economy.upgrades.get(&TIER2_TECH))
        .copied()
        == Some(true);
    upgrade_done
        && snapshot.alive_entities.len() > before_alive
        && current_research(sim).is_none()
        && producer_production(sim, producer).is_empty()
}

/// 双实例继续推进到生产 / 研发完成，逐帧比对同一解锁脚本。
fn advance_until_tier2_effects_dual(
    primary: &mut RtsMatch,
    replay: &mut RtsMatch,
    producer: EntityId,
    before_alive: usize,
) {
    let (unit_frames, tech_frames) = {
        let defs = primary.definitions();
        (
            defs.units()
                .get(&TIER2_UNIT)
                .expect("reason=tier2_unit_def_present")
                .train_frames,
            defs.upgrades()
                .get(&TIER2_TECH)
                .expect("reason=tier2_tech_def_present")
                .research_frames,
        )
    };
    let window = unit_frames.max(tech_frames).saturating_add(4) as usize;
    let mut done = false;
    for frame in 0..window {
        primary.step();
        replay.step();
        assert_dual_state_matches(primary, replay, "effects", frame);
        let events_a = primary.drain_events();
        let events_b = replay.drain_events();
        assert_eq!(
            events_a, events_b,
            "reason=effects_frame_{frame}_events_match"
        );
        if tier2_effects_settled(primary, producer, before_alive) {
            done = true;
            break;
        }
    }
    assert!(done, "reason=tier2_effects_never_settled");
    assert!(
        tier2_effects_settled(replay, producer, before_alive),
        "reason=replay_tier2_effects_settled"
    );
}

/// 提交一条 `UpgradeTownHall` 命令（执行帧由当前只读视图的冻结帧号推导）。
///
/// 用当前 `tick` 而非 `tick + 1`：内核 `step()` 阶段 7 以
/// `freeze_frame(clock.current())` 取 `execute_frame <= frame` 的命令，故提交到
/// 当前冻结帧可在下一次 `step()` 立即执行（与 P8.5a `submit_revive` 同口径）。
fn submit_upgrade(sim: &mut RtsMatch, town_hall: EntityId, town_hall_def: DefId) {
    let frame = Frame::new(sim.read_view().tick);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        PLAYER,
        1,
        GameplayCommandKind::UpgradeTownHall {
            town_hall,
            town_hall_def,
            target_tier: TownHallTier::T2,
            cancel: false,
        },
    ))
    .expect("reason=upgrade_command_rejected");
}

/// 真实装配下提交 `UpgradeTownHall` 后返回 `Some(TownHallUpgradeState)`：
/// 仅当主城类别登记进冻结定义、升级队列确实建立时成立；缺陷态（未登记）
/// 返回 `None`。仿 P8.5a `train_probes_hero_state` 的 A-B 对照，走既有确定性
/// 生产路径（提交 → `step()` → `capture_snapshot()`），不是旁路模拟
/// （CODEBUDDY §5 / D8 / ADR 0002）。
fn upgrade_probes_state(defs: &FrozenDefinitions) -> Option<TownHallUpgradeState> {
    let content = defs.content_hash();
    let world = synthetic_world();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, world.clone(), defs.clone());
    sim.submit_command(spawn(0, 0, 500, 1))
        .expect("reason=probe_spawn_rejected");
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    economy.lumber = 100_000;
    let mut sim =
        RtsMatch::restore_snapshot(snapshot, world, content).expect("reason=probe_restore_failed");
    let town_hall = EntityId::new(TOWN_HALL_ID).expect("reason=invalid_probe_hall_id");
    submit_upgrade(&mut sim, town_hall, HUMAN_TOWN_HALL);
    sim.step();
    let state = sim.capture_snapshot().town_hall;
    state.queue.contains_key(&town_hall).then_some(state)
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

// ---------------------------------------------------------------------------
// 任务 D：真实装配接入 with_town_hall_upgrade 的 CPU 证据
// ---------------------------------------------------------------------------

#[test]
fn synthetic_definitions_registers_town_hall_categories_and_tier_chain() {
    let defs = synthetic_definitions();

    // 主城类别由定义层 `BTreeSet<DefId>` 驱动，非建筑名白名单。
    assert!(
        defs.is_town_hall(TOWN_HALL),
        "reason=baseline_hall_registered"
    );
    assert!(
        defs.is_town_hall(HUMAN_TOWN_HALL),
        "reason=human_hall_registered"
    );
    assert!(
        defs.is_town_hall(ORC_TOWN_HALL),
        "reason=orc_hall_registered"
    );
    assert!(!defs.is_town_hall(SOLDIER), "reason=categories_not_crossed");
    // P8.5d：回城目标也登记为主城类别，使数据驱动目标与升级类别共用同一集合。
    assert!(
        defs.is_town_hall(TOWN_HALL_PORTAL_DEF),
        "reason=portal_town_hall_registered"
    );
    assert_eq!(
        defs.town_hall_defs(),
        &[
            TOWN_HALL,
            HUMAN_TOWN_HALL,
            ORC_TOWN_HALL,
            TOWN_HALL_PORTAL_DEF,
        ]
        .into_iter()
        .collect::<BTreeSet<DefId>>(),
        "reason=town_hall_defs_exact"
    );

    // P8.5b：8101 / 8102 既是需求链 DefId，也是真实定义表条目，
    // 使 `complete_production` / `apply_upgrade` 升级后可查表结算。
    assert!(
        defs.units().contains_key(&TIER2_UNIT),
        "reason=tier2_unit_def_registered"
    );
    let tier2_unit = defs
        .units()
        .get(&TIER2_UNIT)
        .expect("reason=tier2_unit_def_present");
    assert!(
        tier2_unit.train_frames > 1,
        "reason=tier2_unit_train_frames"
    );
    assert!(
        defs.upgrades().contains_key(&TIER2_TECH),
        "reason=tier2_tech_def_registered"
    );
    let tier2_tech = defs
        .upgrades()
        .get(&TIER2_TECH)
        .expect("reason=tier2_tech_def_present");
    assert!(
        tier2_tech.research_frames > 1 && tier2_tech.damage_mod > 0 && tier2_tech.hp_mod > 0,
        "reason=tier2_tech_settles"
    );

    // tier 需求链只登记合成原创 `8xxx` 真实定义 DefId → T2；既有可训练 / 可研发
    // DefId 一律无需求，保证既有装配脚本的命令行为不变（R1.2 / R13.3 / Q4）。
    assert_eq!(
        defs.tier_requirement(TIER2_UNIT),
        Some(TownHallTier::T2),
        "reason=tier2_unit_requires_t2"
    );
    assert_eq!(
        defs.tier_requirement(TIER2_TECH),
        Some(TownHallTier::T2),
        "reason=tier2_tech_requires_t2"
    );
    assert_eq!(
        defs.tier_requirement(SOLDIER),
        None,
        "reason=existing_def_not_gated"
    );
    assert_eq!(
        defs.tier_requirement(HUMAN_BARRACKS),
        None,
        "reason=existing_barracks_not_gated"
    );
    assert_eq!(
        defs.tier_requirements(),
        &[
            (TIER2_UNIT, TownHallTier::T2),
            (TIER2_TECH, TownHallTier::T2)
        ]
        .into_iter()
        .collect::<BTreeMap<DefId, TownHallTier>>(),
        "reason=tier_requirements_exact"
    );

    // 冻结常量段编入内容哈希；退化定义（未登记）与正式定义同源可对照。
    let constants: TownHallConstants = *defs.town_hall_constants();
    assert_eq!(constants, SYNTHETIC_TOWN_HALL_CONSTANTS);
    let cost = constants.cost(TownHallTier::T2);
    assert!(cost.is_some(), "reason=tier2_cost_some");
    assert_eq!(constants.cost(TownHallTier::T1), None, "reason=t1_no_cost");
    let degenerate = degenerate_definitions();
    assert!(
        !degenerate.is_town_hall(HUMAN_TOWN_HALL),
        "reason=degenerate_hall_false"
    );
    assert_eq!(
        degenerate.tier_requirement(TIER2_UNIT),
        None,
        "reason=degenerate_chain_false"
    );
    assert_ne!(
        defs.content_hash(),
        degenerate.content_hash(),
        "reason=town_hall_defs_hash_changed"
    );
}

#[test]
fn upgrade_command_is_accepted_in_real_assembly_and_builds_state() {
    let (mut sim, defs, _world, _content, town_hall) = upgrade_fixture();
    submit_upgrade(&mut sim, town_hall, HUMAN_TOWN_HALL);
    sim.step();

    let snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v8");
    assert_eq!(SNAPSHOT_VERSION, 8, "reason=snapshot_version_8");
    // `Some(TownHallUpgradeState)`：v8 快照携带已填充的顶层权威状态。
    let state: TownHallUpgradeState = snapshot.town_hall.clone();
    let item = state
        .queue
        .get(&town_hall)
        .expect("reason=upgrade_queue_present");
    let cost = defs
        .town_hall_constants()
        .cost(TownHallTier::T2)
        .expect("reason=tier2_cost_present");
    assert_eq!(item.town_hall, town_hall, "reason=queue_town_hall");
    assert_eq!(item.town_hall_def, HUMAN_TOWN_HALL, "reason=queue_def");
    assert_eq!(item.owner, PLAYER, "reason=queue_owner");
    assert_eq!(item.from_tier, TownHallTier::T1, "reason=queue_from_t1");
    assert_eq!(item.target_tier, TownHallTier::T2, "reason=queue_to_t2");
    assert_eq!(item.gold, cost.gold, "reason=queue_gold");
    assert_eq!(item.lumber, cost.lumber, "reason=queue_lumber");
    assert_eq!(item.total_frames, cost.frames, "reason=queue_total_frames");
    assert!(item.remaining_frames > 0, "reason=queue_progress_remaining");
    assert!(
        state.charges.contains_key(&town_hall),
        "reason=charge_present"
    );
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_matches_real_assembly"
    );

    // 定义层类别驱动：未登记 `town_hall_def` 的命令被显式拒绝、不免费升级。
    let (mut other, _defs, _world, _content, hall2) = upgrade_fixture();
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
            GameplayCommandKind::UpgradeTownHall {
                town_hall: hall2,
                town_hall_def: 9_999,
                target_tier: TownHallTier::T2,
                cancel: false,
            },
        ))
        .expect("reason=unknown_hall_command_submitted");
    other.step();
    let snap = other.capture_snapshot();
    assert!(
        snap.town_hall.queue.is_empty(),
        "reason=unknown_hall_rejected"
    );
    assert!(
        snap.town_hall.charges.is_empty(),
        "reason=unknown_hall_no_charge"
    );
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).map_or(0, |e| e.gold),
        gold_before,
        "reason=unknown_hall_no_deduction"
    );
}

/// 真实装配的 `Some(TownHallUpgradeState)` A-B 对照（仿 P8.5a
/// `train_produces_some_hero_state_only_with_hero_registration`）：
/// 正式定义登记主城类别 → `Some`（队列建立、费用扣减）；退化定义未登记 →
/// `None`（显式拒绝、不免费升级）。证明接入是升级链路可达的必要条件。
#[test]
fn upgrade_produces_some_town_hall_state_only_with_hall_registration() {
    let formal = upgrade_probes_state(&synthetic_definitions());
    assert!(formal.is_some(), "reason=formal_hall_produces_some_state");
    let state = formal.expect("reason=formal_state_present");
    assert_eq!(state.queue.len(), 1, "reason=formal_queue_len");
    assert_eq!(state.charges.len(), 1, "reason=formal_charges_len");

    // 缺陷态：未登记主城升级段的退化定义 → `None`（反例元断言防退化）。
    let degenerate = upgrade_probes_state(&degenerate_definitions());
    assert!(
        degenerate.is_none(),
        "reason=degenerate_hall_produces_no_state"
    );
}

/// 反向验证·缺陷态：未登记主城升级段的退化定义拒绝 `UpgradeTownHall`。
/// 与上一修复态构成 `CODEBUDDY.md §5` 的反例元断言证据对。
#[test]
fn reverse_validation_unregistered_town_hall_rejects_upgrade() {
    let defs = degenerate_definitions();
    let content = defs.content_hash();
    let world = StaticWorld::default();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(config, world.clone(), defs.clone());
    sim.submit_command(spawn(0, 0, 500, 1))
        .expect("reason=spawn_rejected");
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = 100_000;
    economy.lumber = 100_000;
    let mut sim = RtsMatch::restore_snapshot(snapshot, world, content)
        .expect("reason=degenerate_restore_failed");
    let hall = EntityId::new(TOWN_HALL_ID).expect("reason=invalid_hall_id");
    submit_upgrade(&mut sim, hall, HUMAN_TOWN_HALL);
    sim.step();
    let snap = sim.capture_snapshot();
    assert!(
        snap.town_hall.queue.is_empty(),
        "reason=degenerate_defs_must_reject_upgrade"
    );
}

// ---------------------------------------------------------------------------
// 任务 C：只读 HUD 派生、连续重绘隔离回归与反向验证
// ---------------------------------------------------------------------------

#[test]
fn town_hall_upgrade_hud_derives_only_from_readonly_snapshot_and_events() {
    let (mut sim, _defs, _world, _content, town_hall) = upgrade_fixture();
    submit_upgrade(&mut sim, town_hall, HUMAN_TOWN_HALL);
    sim.step();

    let snapshot = sim.capture_snapshot();
    let events = vec![
        Event {
            frame: 1,
            sequence: 10,
            entity: TOWN_HALL_ID,
            kind: EventKind::TownHallUpgradeStarted,
        },
        Event {
            frame: 1,
            sequence: 11,
            entity: TOWN_HALL_ID,
            kind: EventKind::TownHallUpgradeStarted,
        },
        Event {
            frame: 2,
            sequence: 12,
            entity: TOWN_HALL_ID,
            kind: EventKind::TownHallUpgradeCompleted,
        },
        Event {
            frame: 2,
            sequence: 13,
            entity: TOWN_HALL_ID,
            kind: EventKind::TownHallUpgradeCancelled,
        },
        Event {
            frame: 2,
            sequence: 14,
            entity: TOWN_HALL_ID,
            kind: EventKind::Spawned,
        },
    ];
    let projection = TownHallUpgradeHudModel::derive(&snapshot, &events);

    // 队列项逐字段来自只读 `Snapshot.town_hall.queue`，不带权威句柄。
    assert_eq!(projection.queue.len(), snapshot.town_hall.queue.len());
    let source = snapshot
        .town_hall
        .queue
        .get(&town_hall)
        .expect("reason=queue_source");
    let derived = projection
        .queue
        .iter()
        .find(|item| item.town_hall == TOWN_HALL_ID)
        .expect("reason=derived_queue_item");
    assert_eq!(derived.town_hall, source.town_hall.value());
    assert_eq!(derived.town_hall_def, source.town_hall_def);
    assert_eq!(derived.owner, source.owner);
    assert_eq!(derived.from_tier, source.from_tier);
    assert_eq!(derived.target_tier, source.target_tier);
    assert_eq!(derived.remaining_frames, source.remaining_frames);
    assert_eq!(derived.total_frames, source.total_frames);
    assert_eq!(derived.gold, source.gold);
    assert_eq!(derived.lumber, source.lumber);
    // `HudUpgradeItem` 全部字段为 owned 值拷贝，纯表现层。
    let _: &HudUpgradeItem = derived;
    assert_eq!(projection.charges, snapshot.town_hall.charges.len());
    assert_eq!(projection.tiers.len(), snapshot.town_hall.tiers.len());
    assert_eq!(projection.started_events, 2, "reason=started_event_count");
    assert_eq!(
        projection.completed_events, 1,
        "reason=completed_event_count"
    );
    assert_eq!(
        projection.cancelled_events, 1,
        "reason=cancelled_event_count"
    );

    // 纯函数：同一只读输入重复派生结果完全一致，且 `HudModel::collect`
    // 产出的投影与独立派生相同（只读通路同源）。
    assert_eq!(
        projection,
        TownHallUpgradeHudModel::derive(&snapshot, &events)
    );
    let view = sim.read_view();
    let selection = BTreeSet::new();
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let model = HudModel::collect(&view, &snapshot, &selection, &events, &mapper, PLAYER);
    assert_eq!(
        model.town_hall, projection,
        "reason=hud_collect_town_hall_matches_derive"
    );

    // 空快照 / 空事件派生空投影，无墙钟、无全局静态字典、无建筑名白名单。
    let empty = TownHallUpgradeHudModel::derive(
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
    assert_eq!(
        empty,
        TownHallUpgradeHudModel::default(),
        "reason=empty_projection"
    );
}

#[test]
fn hud_redraw_keeps_state_hash_and_snapshot_bytes() {
    let (mut rendered, _defs, _world, _content, hall_a) = upgrade_fixture();
    let (mut plain, _defs2, _world2, _content2, hall_b) = upgrade_fixture();
    submit_upgrade(&mut rendered, hall_a, HUMAN_TOWN_HALL);
    submit_upgrade(&mut plain, hall_b, HUMAN_TOWN_HALL);

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
            !model.town_hall.queue.is_empty(),
            "reason=upgrade_queue_visible frame={frame}"
        );
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            draw(ctx, &model, &ZH, HudAssets { text_enabled: true });
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
            "reason=snapshot_version_8 frame={frame}"
        );
    }
}

#[test]
fn reverse_validation_town_hall_hud_writeback_fails_isolation_assertion() {
    let (mut rendered, _defs, _world, _content, hall_a) = upgrade_fixture();
    let (mut plain, _defs2, _world2, _content2, hall_b) = upgrade_fixture();
    submit_upgrade(&mut rendered, hall_a, HUMAN_TOWN_HALL);
    submit_upgrade(&mut plain, hall_b, HUMAN_TOWN_HALL);
    rendered.step();
    plain.step();

    // 修复态：正式只读派生（只消费 `&Snapshot` / `&[Event]`）不改权威状态，
    // 双实例逐帧哈希 / 快照字节一致，隔离断言通过。
    let snapshot = rendered.capture_snapshot();
    let events = rendered.drain_events();
    let _ = plain.drain_events();
    let _projection = TownHallUpgradeHudModel::derive(&snapshot, &events);
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

// ---------------------------------------------------------------------------
// 任务 B：命令卡升级格位由定义层数据驱动（R13.3 / C15 / Q6 `[待核]`）
// ---------------------------------------------------------------------------

#[test]
fn upgrade_slot_is_definition_driven_not_name_whitelist() {
    let defs = synthetic_definitions();
    // 已登记主城 + 需求链存在 T2 需求 → 显示升级格位、目标 T2。
    let slot = derive_upgrade_slot(&defs, HUMAN_TOWN_HALL);
    assert_eq!(
        slot,
        Some(UpgradeSlot {
            town_hall_def: HUMAN_TOWN_HALL,
            target_tier: TownHallTier::T2,
        }),
        "reason=registered_hall_shows_slot"
    );
    // 未登记类别 → 无格位。
    assert_eq!(
        derive_upgrade_slot(&defs, SOLDIER),
        None,
        "reason=combat_unit_has_no_upgrade_slot"
    );

    // 以非经典命名的主城 DefId 登记进冻结定义后，仍按需求链显示格位：
    // 判据只查 `is_town_hall` 类别登记与 `tier_requirements` 需求链。
    let non_classic: DefId = 9_101;
    let units: Vec<(DefId, UnitDef)> = defs.units().iter().map(|(&id, &d)| (id, d)).collect();
    let abilities: Vec<(DefId, AbilityDef)> = defs
        .abilities()
        .iter()
        .map(|(&id, d)| (id, d.clone()))
        .collect();
    let items: Vec<(DefId, ItemDef)> = defs.items().iter().map(|(&id, &d)| (id, d)).collect();
    let upgrades: Vec<(DefId, UpgradeDef)> =
        defs.upgrades().iter().map(|(&id, &d)| (id, d)).collect();
    let alt_defs = FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("reason=non_classic_defs_invalid")
        .with_hero_revive(
            *defs.revive_constants(),
            defs.altar_defs().clone(),
            defs.hero_defs().clone(),
        )
        .with_town_hall_upgrade(
            SYNTHETIC_TOWN_HALL_CONSTANTS,
            [non_classic].into_iter().collect::<BTreeSet<DefId>>(),
            [(TIER2_UNIT, TownHallTier::T2)]
                .into_iter()
                .collect::<BTreeMap<DefId, TownHallTier>>(),
        )
        .with_creep_camps(
            *defs.creep_constants(),
            defs.camp_scripts().clone(),
            defs.drop_tables().clone(),
            defs.neutral_categories().clone(),
        )
        // P8.5d：同源登记商店段，使非经典主城对照只差类别集合。
        .with_item_shop(defs.item_shop().clone());
    assert!(
        alt_defs.is_town_hall(non_classic),
        "reason=non_classic_hall_registered"
    );
    assert_eq!(
        derive_upgrade_slot(&alt_defs, non_classic),
        Some(UpgradeSlot {
            town_hall_def: non_classic,
            target_tier: TownHallTier::T2,
        }),
        "reason=non_classic_hall_shows_slot_data_driven"
    );

    // 白名单退化实现（缺陷态）：按硬编码经典 DefId 判格位，对非经典命名
    // DefId 判不出格位；正式数据驱动实现仍能判出 —— 反例元断言防退化。
    fn whitelist_slot(def_id: DefId) -> bool {
        def_id == HUMAN_TOWN_HALL || def_id == ORC_TOWN_HALL || def_id == TOWN_HALL
    }
    assert!(
        !whitelist_slot(non_classic),
        "reason=whitelist_degenerate_fails_for_non_classic"
    );
    assert!(
        derive_upgrade_slot(&alt_defs, non_classic).is_some(),
        "reason=formal_definition_chain_passes"
    );
}

// ---------------------------------------------------------------------------
// critic blocker 1：真实装配「升级后训练 / 解锁生效」CPU 验收
// ---------------------------------------------------------------------------

/// T1 阶段：8101 / 8102 受冻结 T2 需求链约束，被显式拒绝；用等量 no-op
/// 玩法命令对齐到达序号后，逐帧哈希 / 快照 / `first_difference()` 与基线一致。
#[test]
fn real_assembly_t1_rejects_tier2_train_and_research_without_state_drift() {
    let (mut rejected, defs, _world, _content, hall, producer) = tier_gate_fixture();
    let (mut baseline, _defs2, _world2, _content2, _hall2, producer2) = tier_gate_fixture();
    assert_eq!(
        defs.tier_requirement(TIER2_UNIT),
        Some(TownHallTier::T2),
        "reason=tier2_unit_requires_t2"
    );
    assert_eq!(
        defs.tier_requirement(TIER2_TECH),
        Some(TownHallTier::T2),
        "reason=tier2_tech_requires_t2"
    );
    assert_eq!(
        defs.tier_requirement(HUMAN_BARRACKS),
        None,
        "reason=existing_barracks_not_gated"
    );

    submit_unlock_script(&mut rejected, producer, 1);
    submit_noop_cancel_train(&mut baseline, producer2, 1);
    submit_noop_cancel_train(&mut baseline, producer2, 2);
    let gold_before = rejected
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .map_or(0, |economy| economy.gold);
    rejected.step();
    baseline.step();

    assert!(
        producer_production(&rejected, producer).is_empty(),
        "reason=t1_train_rejected"
    );
    assert_eq!(
        current_research(&rejected),
        None,
        "reason=t1_research_rejected"
    );
    assert_eq!(
        rejected.compute_state_hash(),
        baseline.compute_state_hash(),
        "reason=no_authoritative_state_drift field={:?}",
        rejected.first_difference(&baseline)
    );
    assert_eq!(
        rejected.first_difference(&baseline),
        None,
        "reason=no_difference_vs_noop_baseline"
    );
    assert_eq!(
        rejected.capture_snapshot(),
        baseline.capture_snapshot(),
        "reason=snapshot_matches_noop_baseline"
    );
    assert_eq!(
        rejected
            .capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .map_or(0, |economy| economy.gold),
        gold_before,
        "reason=no_resource_deduction"
    );
    assert_eq!(
        tier_of_town_hall(&rejected, hall),
        None,
        "reason=t1_has_no_t2_tier"
    );
}

/// T2 完成后：同一 8101 / 8102 命令脚本被接受，生产生成新实体、科技旗标
/// 与属性修正结算；双实例同脚本逐帧哈希 / 快照 / 首差异字段诊断一致。
#[test]
fn real_assembly_t2_accepts_tier2_train_research_and_settles_effects() {
    let (mut primary, defs, _world, _content, hall_a, producer_a) = tier_gate_fixture();
    let (mut replay, _defs2, _world2, _content2, hall_b, producer_b) = tier_gate_fixture();
    assert_eq!(
        hall_a.value(),
        hall_b.value(),
        "reason=identical_hall_entity"
    );
    assert_eq!(
        producer_a.value(),
        producer_b.value(),
        "reason=identical_producer_entity"
    );

    submit_upgrade(&mut primary, hall_a, HUMAN_TOWN_HALL);
    submit_upgrade(&mut replay, hall_b, HUMAN_TOWN_HALL);
    advance_until_upgraded_dual(&mut primary, &mut replay, hall_a);

    let frame = Frame::new(primary.read_view().tick);
    assert_eq!(
        frame.value(),
        replay.read_view().tick,
        "reason=same_tick_before_unlock_commands"
    );
    submit_unlock_script(&mut primary, producer_a, 1);
    submit_unlock_script(&mut replay, producer_b, 1);
    primary.step();
    replay.step();
    assert_dual_state_matches(&primary, &replay, "accept", 0);
    assert!(
        producer_production(&primary, producer_a)
            .iter()
            .any(|item| item.unit_def == TIER2_UNIT),
        "reason=t2_unit_unlocked"
    );
    assert_eq!(
        current_research(&primary).map(|(upgrade, _)| upgrade),
        Some(TIER2_TECH),
        "reason=t2_research_unlocked"
    );

    let before = primary.capture_snapshot();
    let before_alive = before.alive_entities.len();
    let before_next_id = before.next_entity_id;
    let before_produced = before
        .gameplay
        .players
        .get(&PLAYER)
        .map_or(0, |economy| economy.produced);
    advance_until_tier2_effects_dual(&mut primary, &mut replay, producer_a, before_alive);

    let snapshot = primary.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION, "reason=snapshot_v8");
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_matches_real_assembly"
    );
    assert_eq!(
        snapshot.alive_entities.len(),
        before_alive.saturating_add(1),
        "reason=complete_production_generated_entity"
    );
    assert_eq!(
        snapshot.next_entity_id,
        before_next_id.saturating_add(1),
        "reason=next_entity_id_advanced_once"
    );
    let economy = snapshot
        .gameplay
        .players
        .get(&PLAYER)
        .expect("reason=player_economy_present");
    assert_eq!(
        economy.upgrades.get(&TIER2_TECH),
        Some(&true),
        "reason=tier2_tech_flag_set"
    );
    assert_eq!(
        economy.produced,
        before_produced.saturating_add(1),
        "reason=produced_counter_advanced_once"
    );
    let tech = *defs
        .upgrades()
        .get(&TIER2_TECH)
        .expect("reason=tier2_tech_def_present");
    assert!(
        tech.damage_mod > 0 && tech.hp_mod > 0,
        "reason=tier2_tech_has_settlement_effect"
    );
    let hall = snapshot
        .positions
        .iter()
        .find(|position| position.entity == hall_a.value())
        .expect("reason=town_hall_position_present");
    assert_eq!(
        hall.gameplay.stat_mods.damage, tech.damage_mod,
        "reason=apply_upgrade_settled_damage_mod"
    );
    assert_eq!(
        hall.gameplay.stat_mods.max_hp, tech.hp_mod,
        "reason=apply_upgrade_settled_hp_mod"
    );
    assert_eq!(
        current_research(&primary),
        None,
        "reason=research_queue_cleared"
    );
    assert!(
        producer_production(&primary, producer_a).is_empty(),
        "reason=production_queue_cleared"
    );
    assert_dual_state_matches(&primary, &replay, "final", 0);
}

/// 同一 T2 命令脚本在 T1 / T2 产生可观测差异，不是「所有 DefId 恒放行」。
/// T1 用等量 no-op 对齐早期升级命令占用的到达序号，随后两端在同一冻结帧
/// 提交同一 `Train` / `Research` 脚本，比较队列行为与权威状态哈希。
#[test]
fn real_assembly_t1_and_t2_unlock_behavior_differs_with_same_command_script() {
    let (mut t1, _defs1, _world1, _content1, _hall1, producer1) = tier_gate_fixture();
    let (mut t2, _defs2, _world2, _content2, hall2, producer2) = tier_gate_fixture();
    t1.drain_events();
    t2.drain_events();
    let upgrade_window = {
        let constants = t2.definitions().town_hall_constants();
        constants
            .cost(TownHallTier::T2)
            .expect("reason=tier2_cost_present")
            .frames
            .saturating_add(16) as usize
    };
    submit_upgrade(&mut t2, hall2, HUMAN_TOWN_HALL);
    let mut t2_done = false;
    for _ in 0..upgrade_window {
        t2.step();
        let events = t2.drain_events();
        if events
            .iter()
            .any(|event| event.kind == EventKind::TownHallUpgradeCompleted)
        {
            t2_done = true;
            break;
        }
    }
    assert!(t2_done, "reason=t2_upgrade_never_completed");

    while t1.read_view().tick < t2.read_view().tick {
        t1.step();
        t1.drain_events();
    }
    assert_eq!(
        t1.read_view().tick,
        t2.read_view().tick,
        "reason=same_tick_before_unlock_commands"
    );

    // 对齐玩法命令队列计数器：T2 早期升级命令占一个到达序号，T1 用等量
    // `CancelTrain` no-op 对齐，随后两端提交同一 `Train` / `Research` 脚本。
    submit_noop_cancel_train(&mut t1, producer1, 1);
    submit_unlock_script(&mut t1, producer1, 2);
    submit_unlock_script(&mut t2, producer2, 2);
    t1.step();
    t2.step();

    assert!(
        producer_production(&t1, producer1).is_empty(),
        "reason=t1_still_rejects_same_script"
    );
    assert_eq!(
        current_research(&t1),
        None,
        "reason=t1_research_still_rejected"
    );
    assert!(
        producer_production(&t2, producer2)
            .iter()
            .any(|item| item.unit_def == TIER2_UNIT),
        "reason=t2_accepts_same_script"
    );
    assert_eq!(
        current_research(&t2).map(|(upgrade, _)| upgrade),
        Some(TIER2_TECH),
        "reason=t2_research_accepts_same_script"
    );
    assert_eq!(
        t1.read_view().tick,
        t2.read_view().tick,
        "reason=same_tick_after_unlock_commands"
    );
    assert_ne!(
        t1.compute_state_hash(),
        t2.compute_state_hash(),
        "reason=t1_t2_authoritative_state_must_differ"
    );
    assert_ne!(
        t1.capture_snapshot(),
        t2.capture_snapshot(),
        "reason=t1_t2_snapshot_must_differ"
    );
    assert!(
        t1.first_difference(&t2).is_some(),
        "reason=t1_t2_first_difference_present"
    );
}

/// 反向验证·缺陷态：故意不登记主城升级段（需求链缺失）时，8101 / 8102
/// 在 T1 会退化成「无需求恒放行」；正式真实装配仍显式拒绝。元断言证明
/// 新增回归在缺陷态会失败，不是只靠全绿（CODEBUDDY.md §5）。
#[test]
fn real_assembly_tier_gate_reverse_validation_missing_chain_accepts_at_t1() {
    let (mut formal, defs, _world, _content, _hall, producer) = tier_gate_fixture();
    let degenerate_defs = degenerate_definitions();
    assert_eq!(
        degenerate_defs.tier_requirement(TIER2_UNIT),
        None,
        "reason=degenerate_unit_chain_absent"
    );
    assert_eq!(
        degenerate_defs.tier_requirement(TIER2_TECH),
        None,
        "reason=degenerate_tech_chain_absent"
    );
    assert!(
        defs.tier_requirement(TIER2_UNIT).is_some() && defs.tier_requirement(TIER2_TECH).is_some(),
        "reason=formal_chain_present"
    );
    let (mut degenerate, _ddefs, _world2, _content2, _hall2, producer2) =
        tier_gate_fixture_with_defs(degenerate_defs);

    submit_unlock_script(&mut formal, producer, 1);
    submit_unlock_script(&mut degenerate, producer2, 1);
    formal.step();
    degenerate.step();

    assert!(
        producer_production(&formal, producer).is_empty(),
        "reason=formal_t1_train_rejected"
    );
    assert_eq!(
        current_research(&formal),
        None,
        "reason=formal_t1_research_rejected"
    );
    assert!(
        producer_production(&degenerate, producer2)
            .iter()
            .any(|item| item.unit_def == TIER2_UNIT),
        "reason=degenerate_t1_train_wrongly_accepted"
    );
    assert_eq!(
        current_research(&degenerate).map(|(upgrade, _)| upgrade),
        Some(TIER2_TECH),
        "reason=degenerate_t1_research_wrongly_accepted"
    );
    assert_ne!(
        formal.compute_state_hash(),
        degenerate.compute_state_hash(),
        "reason=defect_state_detected_by_hash"
    );
    assert!(
        formal.first_difference(&degenerate).is_some(),
        "reason=defect_state_located_by_first_difference"
    );
}
