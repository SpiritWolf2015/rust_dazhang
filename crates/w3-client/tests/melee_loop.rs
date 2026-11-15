//! 一局 1v1 Melee 的端到端 CPU 验收（change `melee-playable-loop` 组 2）。
//!
//! 全部经 `w3-core` 既有公开 API 装配，`w3-core` 零改动；测试断言采集、
//! 建造、训练、战斗、胜负结算与重开闭环可达且可重复。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::melee::defs::{
    BARRACKS, CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE,
    CATEGORY_SMITHY, HERO_ABILITY, HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO, HUMAN_RANGED, HUMAN_SIEGE,
    HUMAN_SMITHY, SOLDIER, UPGRADE_ARMOR, UPGRADE_WEAPONS,
};
use w3_client::melee::{
    AI_PLAYER, LOCAL_PLAYER, MeleeDriver, MeleeSetup, Race, synthetic_definitions, synthetic_world,
};
use w3_client::{
    ClientCommand, FixedStepClient, LOGIC_FPS, MovabilityMap, StepDriver, TICKS_PER_LOGIC_FRAME,
};
use w3_core::{
    Command, CommandKind, DefId, EffectKind, EntityId, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, PositionRecord, RtsMatch, SNAPSHOT_VERSION, Snapshot,
    UnitDef,
};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;

/// 既有输入翻译回归未注入判据映射：空映射沿用「全部可移动」的旧行为契约
/// （design D1 的 `unwrap_or(true)` 回落），仅适配 `apply` 新签名。
fn legacy_movability() -> MovabilityMap {
    MovabilityMap::new()
}

/// 按开局脚本装配一局；`horizon = 0` 时不注入 AI 脚本，便于隔离玩家操作。
fn build_driver(setup: &MeleeSetup, horizon: u64) -> MeleeDriver {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim =
        RtsMatch::create_with_definitions(config, synthetic_world(), synthetic_definitions());
    for command in &setup.commands {
        sim.submit_command(*command)
            .expect("melee: reason=spawn_command_rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("melee: reason=opening_gameplay_rejected");
    }
    MeleeDriver::new(FixedStepClient::from_match(sim), setup, horizon)
}

/// 逐帧推进恰好 `frames` 个整数逻辑帧。
fn advance(driver: &mut MeleeDriver, frames: u64) {
    for _ in 0..frames {
        driver.advance_ticks(TICKS_PER_LOGIC_FRAME);
    }
}

fn submit_simulation(
    driver: &mut MeleeDriver,
    player: u8,
    sequence: u32,
    frame: u64,
    kind: CommandKind,
) {
    driver
        .enqueue(ClientCommand::Simulation(Command::new(
            Frame::new(frame),
            player,
            sequence,
            kind,
        )))
        .expect("melee: reason=simulation_command_rejected");
}

fn submit_gameplay(
    driver: &mut MeleeDriver,
    player: u8,
    sequence: u32,
    frame: u64,
    kind: GameplayCommandKind,
) {
    driver
        .enqueue(ClientCommand::Gameplay(GameplayCommand::new(
            Frame::new(frame),
            player,
            sequence,
            kind,
        )))
        .expect("melee: reason=gameplay_command_rejected");
}

#[test]
fn setup_spawns_both_sides_and_two_players() {
    let setup = MeleeSetup::new();
    let mut driver = build_driver(&setup, 0);
    advance(&mut driver, 1);

    let view = driver.view();
    assert_eq!(view.entities.len(), 12);
    assert!(
        view.entities
            .iter()
            .any(|e| e.id == setup.local_hall.value())
    );
    assert!(view.entities.iter().any(|e| e.id == setup.ai_hall.value()));

    let snapshot = driver.snapshot();
    assert!(snapshot.gameplay.players.contains_key(&LOCAL_PLAYER));
    assert!(snapshot.gameplay.players.contains_key(&AI_PLAYER));
    assert!(!snapshot.gameplay.victory.resolved);
}

#[test]
fn gather_grows_gold_and_replay_matches_hash_and_snapshot() {
    let setup = MeleeSetup::new();
    let mut a = build_driver(&setup, 0);
    let mut b = build_driver(&setup, 0);
    advance(&mut a, 5);
    advance(&mut b, 5);

    assert!(
        a.snapshot().gameplay.players[&LOCAL_PLAYER].gold >= 1,
        "开局采集必须让黄金增长"
    );
    for _ in 0..20 {
        a.advance_ticks(TICKS_PER_LOGIC_FRAME);
        b.advance_ticks(TICKS_PER_LOGIC_FRAME);
        assert_eq!(a.sim().compute_state_hash(), b.sim().compute_state_hash());
        assert_eq!(a.snapshot(), b.snapshot());
    }
}

#[test]
fn build_and_train_generate_entities_with_visible_queue() {
    let setup = MeleeSetup::new();
    let mut driver = build_driver(&setup, 0);
    advance(&mut driver, 1);

    let frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        10,
        frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: BARRACKS,
            is_building: true,
        },
    );
    advance(&mut driver, 3);

    let snapshot = driver.snapshot();
    let queued: Vec<_> = snapshot
        .positions
        .iter()
        .flat_map(|position| &position.gameplay.production)
        .collect();
    assert_eq!(queued.len(), 1, "生产队列必须在画面数据中可见");
    assert!(queued[0].remaining_frames > 0);

    advance(&mut driver, 45);
    let snapshot = driver.snapshot();
    assert_eq!(
        snapshot
            .positions
            .iter()
            .flat_map(|position| &position.gameplay.production)
            .count(),
        0,
        "队列完成后必须刷新"
    );
    assert!(snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 1);
    assert!(driver.view().entities.len() > 12, "建造完成必须生成新实体");
}

#[test]
fn ai_script_drives_opponent_economy() {
    let setup = MeleeSetup::new();
    let mut driver = build_driver(&setup, 300);
    advance(&mut driver, 200);

    let ai = &driver.snapshot().gameplay.players[&AI_PLAYER];
    assert!(ai.gold > 1, "AI 必须产生采集行为");
    assert!(ai.produced >= 1, "AI 必须产生训练行为");
}

#[test]
fn full_script_plays_one_match_to_victory_and_restart_is_clean() {
    let setup = MeleeSetup::new();
    let mut driver = build_driver(&setup, 0);
    advance(&mut driver, 1);

    let mut sequence = 100u32;
    let mut frame = driver.view().tick + 1;

    // 1) 采集：多条 `Gather` 让资源确定增长。
    for _ in 0..5 {
        submit_gameplay(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame,
            GameplayCommandKind::Gather {
                unit: setup.local_peasants[0],
            },
        );
        sequence += 1;
        frame += 1;
    }
    advance(&mut driver, 6);
    assert!(driver.snapshot().gameplay.players[&LOCAL_PLAYER].gold > 1);

    // 2) 建造兵营，推进至队列完成。
    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: BARRACKS,
            is_building: true,
        },
    );
    sequence += 1;
    advance(&mut driver, 50);

    // 3) 训练士兵；训练完成的实体 ID 由内核稳定分配，从视图差集取得。
    let before: BTreeSet<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    let train_frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        train_frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: SOLDIER,
            is_building: false,
        },
    );
    sequence += 1;
    advance(&mut driver, 35);
    let produced: Vec<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .filter(|id| !before.contains(id))
        .collect();
    assert!(!produced.is_empty(), "训练必须生成新实体");
    let soldier = EntityId::new(produced[0]).expect("melee: reason=invalid_soldier_id");

    // 4) 攻击敌方主城直至摧毁（合成攻城单位射程覆盖双方主城间距）。
    let attack_start = driver.view().tick + 1;
    for offset in 0..8 {
        submit_simulation(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            attack_start + offset,
            CommandKind::Attack {
                attacker: soldier,
                target: setup.ai_hall,
            },
        );
        sequence += 1;
    }
    advance(&mut driver, 20);

    // 5) 主城消失 → `MeleeDriver` 提交 `Leave` → 既有 `resolve_victory` 结算。
    let victory = driver.snapshot().gameplay.victory;
    assert!(victory.resolved, "摧毁敌方主城后必须结算胜负");
    assert_eq!(victory.winner, Some(LOCAL_PLAYER));

    // 6) 重开：相同开局脚本产生干净的新一局，不残留上一局状态。
    let restart = MeleeSetup::new();
    let fresh = build_driver(&restart, 0);
    assert!(!fresh.snapshot().gameplay.victory.resolved);
    assert!(fresh.view().entities.is_empty());
}

#[test]
fn input_translation_covers_melee_actions_and_groups() {
    use w3_client::{InputAction, InputState, ScreenMapper};

    let setup = MeleeSetup::new();
    let mut driver = build_driver(&setup, 0);
    advance(&mut driver, 1);
    let view = driver.view();
    let mapper = ScreenMapper::new(1920.0, 1080.0, 256.0, [0.0, 0.0], 1.0);
    let mut input = InputState::new(LOCAL_PLAYER);

    // 全屏框选：选中集合是表现层状态，不生成任何命令。
    input
        .apply(
            InputAction::SelectStart([0.0, 0.0]),
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([1920.0, 1080.0]),
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    assert!(!input.selection().is_empty());

    let gather = input
        .apply(
            InputAction::Gather,
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    assert!(gather.iter().any(|command| matches!(
        command,
        ClientCommand::Gameplay(gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::Gather { .. })
    )));

    let build = input
        .apply(
            InputAction::Build,
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    assert!(matches!(
        build[0],
        ClientCommand::Gameplay(ref gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::Train { is_building: true, .. })
    ));

    let cancel = input
        .apply(
            InputAction::CancelTrain,
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    assert!(matches!(
        cancel[0],
        ClientCommand::Gameplay(ref gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::CancelTrain { .. })
    ));

    let stop = input
        .apply(InputAction::Stop, &view, &mapper, &[], &legacy_movability())
        .unwrap();
    assert!(stop.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(simulation)
            if matches!(simulation.kind, CommandKind::Stop { .. })
    )));

    // 编组与按组选择：纯表现层动作，不生成命令、不改哈希。
    assert!(
        input
            .apply(
                InputAction::AssignGroup(3),
                &view,
                &mapper,
                &[],
                &legacy_movability()
            )
            .unwrap()
            .is_empty()
    );
    assert!(
        input
            .apply(
                InputAction::SelectGroup(3),
                &view,
                &mapper,
                &[],
                &legacy_movability()
            )
            .unwrap()
            .is_empty()
    );
    assert!(!input.selection().is_empty());

    // 群体移动：多选共用非 0 分组号，落点由内核按稳定实体 ID 分配。
    let moves = input
        .apply(
            InputAction::ContextAction([1900.0, 1000.0]),
            &view,
            &mapper,
            &[],
            &legacy_movability(),
        )
        .unwrap();
    assert!(moves.len() > 1);
    assert!(moves.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(simulation)
            if matches!(simulation.kind, CommandKind::Move { group, .. } if group > 0)
    )));
}

#[test]
fn ai_commands_are_frame_frozen_not_clock_driven() {
    let setup = MeleeSetup::new();
    let mut per_frame = build_driver(&setup, 300);
    let mut batched = build_driver(&setup, 300);

    // 一端逐帧推进，另一端一次推进 8 帧（恰好 `MAX_CATCHUP_STEPS = 8`
    // 上限边界，不触发追帧丢弃），两端共推进 120 帧。批次超过上限会被
    // 宿主层显式丢弃帧、使两端总帧数不同，那不是本 change 的不确定性。
    // AI 外部命令在开局一次性入队并按 ExecuteFrame 冻结执行，故提交
    // 调用粒度 MUST NOT 改变权威状态哈希与快照（ADR 0002）。
    for _ in 0..120 {
        per_frame.advance_ticks(TICKS_PER_LOGIC_FRAME);
    }
    for _ in 0..15 {
        batched.advance_ticks(TICKS_PER_LOGIC_FRAME * 8);
    }
    assert_eq!(
        per_frame.sim().compute_state_hash(),
        batched.sim().compute_state_hash()
    );
    assert_eq!(per_frame.snapshot(), batched.snapshot());
}

/// 按既有完整闭环脚本把一局打到胜负结算，返回逐帧状态哈希与终局快照。
/// 一局内两族对抗共用一个 `RtsMatch` 与一份 `FrozenDefinitions`；种族参数
/// 只写入表现层 `def_ids`，`Spawn` 命令字节完全一致（design D2）。
fn play_full_match(setup: &MeleeSetup) -> (Vec<u64>, Snapshot) {
    let mut driver = build_driver(setup, 0);
    let mut hashes = Vec::new();
    let step = |driver: &mut MeleeDriver, hashes: &mut Vec<u64>, frames: u64| {
        for _ in 0..frames {
            driver.advance_ticks(TICKS_PER_LOGIC_FRAME);
            hashes.push(driver.sim().compute_state_hash());
        }
    };
    step(&mut driver, &mut hashes, 1);

    let mut sequence = 100u32;
    let mut frame = driver.view().tick + 1;
    for _ in 0..5 {
        submit_gameplay(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame,
            GameplayCommandKind::Gather {
                unit: setup.local_peasants[0],
            },
        );
        sequence += 1;
        frame += 1;
    }
    step(&mut driver, &mut hashes, 6);
    assert!(driver.snapshot().gameplay.players[&LOCAL_PLAYER].gold > 1);

    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: BARRACKS,
            is_building: true,
        },
    );
    sequence += 1;
    step(&mut driver, &mut hashes, 50);

    let before: BTreeSet<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    let train_frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        train_frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: SOLDIER,
            is_building: false,
        },
    );
    sequence += 1;
    step(&mut driver, &mut hashes, 35);
    let produced: Vec<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .filter(|id| !before.contains(id))
        .collect();
    assert!(!produced.is_empty(), "训练必须生成新实体");
    let soldier = EntityId::new(produced[0]).expect("melee: reason=invalid_soldier_id");

    let attack_start = driver.view().tick + 1;
    for offset in 0..8 {
        submit_simulation(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            attack_start + offset,
            CommandKind::Attack {
                attacker: soldier,
                target: setup.ai_hall,
            },
        );
        sequence += 1;
    }
    step(&mut driver, &mut hashes, 20);

    let snapshot = driver.snapshot();
    assert!(snapshot.gameplay.victory.resolved, "一局必须打到胜负结算");
    assert_eq!(snapshot.gameplay.victory.winner, Some(LOCAL_PLAYER));
    (hashes, snapshot)
}

/// 两族参数各装配一局并打完，逐帧哈希与快照字节与「未传种族参数」基线一致
/// （design D2：种族选择不引入运行时权威状态，`SNAPSHOT_VERSION` 仍为 3）。
#[test]
fn both_races_play_full_match_with_baseline_frame_hashes() {
    let baseline = play_full_match(&MeleeSetup::new());
    assert!(!baseline.0.is_empty(), "基线必须含逐帧哈希");

    for race in [Race::Human, Race::Orc] {
        let (hashes, snapshot) = play_full_match(&MeleeSetup::with_race(race));
        assert_eq!(
            hashes, baseline.0,
            "reason=frame_hashes_match_baseline race={race:?}"
        );
        assert_eq!(
            snapshot, baseline.1,
            "reason=snapshot_bytes_match_baseline race={race:?}"
        );
    }
}

/// 回落兼容：未传种族参数回落人族段，一局仍能装配并打完，且与显式人族
/// 参数的逐帧哈希 / 快照完全一致。
#[test]
fn race_fallback_matches_human_segment_in_full_match() {
    let fallback = play_full_match(&MeleeSetup::new());
    let human = play_full_match(&MeleeSetup::with_race(Race::Human));
    assert_eq!(MeleeSetup::new().race, Race::Human);
    assert_eq!(
        fallback.0, human.0,
        "reason=fallback_frame_hashes_match_human"
    );
    assert_eq!(
        fallback.1, human.1,
        "reason=fallback_snapshot_bytes_match_human"
    );
}

/// P8.1 完整一局的逐帧证据：状态哈希、快照序列与终局胜负帧。
#[derive(Clone, Debug, PartialEq, Eq)]
struct FullMatchTrace {
    hashes: Vec<u64>,
    snapshots: Vec<Snapshot>,
    final_snapshot: Snapshot,
    victory_frame: u64,
}

/// 逐帧推进并记录权威状态哈希与快照副本；`step()` 全程由整数累加器驱动。
fn record_steps(driver: &mut MeleeDriver, trace: &mut FullMatchTrace, frames: u64) {
    for _ in 0..frames {
        driver.advance_ticks(TICKS_PER_LOGIC_FRAME);
        trace.hashes.push(driver.sim().compute_state_hash());
        trace.snapshots.push(driver.snapshot());
    }
}

/// 从快照中取出指定实体的只读位置记录。
fn snapshot_position<'a>(snapshot: &'a Snapshot, entity: EntityId) -> &'a PositionRecord {
    snapshot
        .positions
        .iter()
        .find(|position| position.entity == entity.value())
        .expect("melee: reason=missing_snapshot_position")
}

/// 位置记录是否仍与冻结单位定义的初值逐字段一致（用于识别刚生产出的实体）。
fn position_matches_def(position: &PositionRecord, def: &UnitDef) -> bool {
    position.hp == def.hp
        && position.max_hp == def.hp
        && position.speed == def.speed_raw
        && position.attack_damage == def.damage
        && position.attack_range == def.range_raw
        && position.gameplay.mana == def.mana
        && position.gameplay.max_mana == def.mana
}

/// 从「推进前实体集合」的差集中找出归属指定玩家、且初值匹配冻结定义的实体。
fn produced_entity(
    snapshot: &Snapshot,
    before: &BTreeSet<u64>,
    owner: u8,
    def_id: DefId,
    defs: &FrozenDefinitions,
) -> EntityId {
    let def = defs
        .units()
        .get(&def_id)
        .expect("melee: reason=missing_frozen_unit_def");
    let position = snapshot
        .positions
        .iter()
        .find(|&position| {
            !before.contains(&position.entity)
                && position.owner == owner
                && position_matches_def(position, def)
        })
        .expect("melee: reason=missing_produced_entity");
    EntityId::new(position.entity).expect("melee: reason=invalid_produced_entity_id")
}

/// 只读某生产者的当前队列定义编号，用于验证队列可见与清空。
fn production_defs(snapshot: &Snapshot, producer: EntityId) -> Vec<DefId> {
    snapshot_position(snapshot, producer)
        .gameplay
        .production
        .iter()
        .map(|item| item.unit_def)
        .collect()
}

/// 人族新类别装配绑定；显式核对六类槽位仍指向 `1xx` 新定义。
fn human_catalog(setup: &MeleeSetup) -> BTreeMap<u64, DefId> {
    assert_eq!(
        setup.race,
        Race::Human,
        "reason=human_full_match_requires_human_setup"
    );
    let catalog = setup.category_def_ids().clone();
    let expected = [
        (CATEGORY_RANGED, HUMAN_RANGED),
        (CATEGORY_SIEGE, HUMAN_SIEGE),
        (CATEGORY_FARM, HUMAN_FARM),
        (CATEGORY_ALTAR, HUMAN_ALTAR),
        (CATEGORY_SMITHY, HUMAN_SMITHY),
        (CATEGORY_HERO, HUMAN_HERO),
    ];
    for (slot, def_id) in expected {
        assert_eq!(
            catalog.get(&slot),
            Some(&def_id),
            "reason=human_category_binding slot={slot}"
        );
    }
    catalog
}

/// P8.1 人族完整一局：采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 →
/// 摧毁敌方主城结算 → 重开。AI 外部脚本按 300 帧窗口注入，其窗口末尾的
/// `Leave` 在 302 帧，本闭环在 216 帧内不会借 AI 离席提前结算；胜负仍由
/// `MeleeDriver` 检测敌方主城消失后提交 `Leave` 触发既有 `resolve_victory`。
fn play_human_full_match(setup: &MeleeSetup) -> FullMatchTrace {
    let defs = synthetic_definitions();
    let catalog = human_catalog(setup);
    let mut driver = build_driver(setup, 300);
    let mut trace = FullMatchTrace {
        hashes: Vec::new(),
        snapshots: Vec::new(),
        final_snapshot: driver.snapshot(),
        victory_frame: 0,
    };
    let mut sequence = 100u32;
    record_steps(&mut driver, &mut trace, 1);

    // 1) 采集：多条 `Gather` 让本地黄金确定增长。
    let mut frame = driver.view().tick + 1;
    for _ in 0..5 {
        submit_gameplay(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame,
            GameplayCommandKind::Gather {
                unit: setup.local_peasants[0],
            },
        );
        sequence += 1;
        frame += 1;
    }
    record_steps(&mut driver, &mut trace, 6);
    assert!(
        driver.snapshot().gameplay.players[&LOCAL_PLAYER].gold > 1,
        "reason=gather_grows_local_gold"
    );

    // 2) 建造：兵营 + 人口建筑 + 英雄祭坛 + 研发建筑，先验证队列可见再完成。
    let before_build: BTreeSet<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    let build_defs = [
        BARRACKS,
        catalog[&CATEGORY_FARM],
        catalog[&CATEGORY_ALTAR],
        catalog[&CATEGORY_SMITHY],
    ];
    frame = driver.view().tick + 1;
    for (offset, &unit_def) in build_defs.iter().enumerate() {
        submit_gameplay(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame + offset as u64,
            GameplayCommandKind::Train {
                producer: setup.local_hall,
                unit_def,
                is_building: true,
            },
        );
        sequence += 1;
    }
    record_steps(&mut driver, &mut trace, 5);
    {
        let snapshot = driver.snapshot();
        let queue = &snapshot_position(&snapshot, setup.local_hall)
            .gameplay
            .production;
        assert_eq!(
            queue
                .iter()
                .map(|item| item.unit_def)
                .collect::<Vec<DefId>>(),
            build_defs.to_vec(),
            "reason=building_queue_visible"
        );
        assert!(
            queue.iter().all(|item| item.remaining_frames > 0),
            "reason=building_queue_remaining_frames_positive"
        );
    }
    record_steps(&mut driver, &mut trace, 55);
    let snapshot = driver.snapshot();
    assert_eq!(
        production_defs(&snapshot, setup.local_hall),
        Vec::<DefId>::new(),
        "reason=local_building_queue_empty"
    );
    assert!(
        snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 4,
        "reason=local_building_produced_count"
    );
    assert!(
        driver.view().entities.len() >= before_build.len() + 4,
        "reason=four_new_buildings_spawned"
    );
    let built_entities: Vec<EntityId> = build_defs
        .iter()
        .map(|&def_id| produced_entity(&snapshot, &before_build, LOCAL_PLAYER, def_id, &defs))
        .collect();
    for entity in &built_entities {
        assert_eq!(
            snapshot_position(&snapshot, *entity).speed,
            0,
            "reason=new_building_runtime_speed_zero"
        );
    }

    // 3) 训练：远程单位 / 反重甲单位 / 英雄，逐帧推进至生产队列清空。
    let before_train: BTreeSet<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    let train_defs = [
        catalog[&CATEGORY_RANGED],
        catalog[&CATEGORY_SIEGE],
        catalog[&CATEGORY_HERO],
    ];
    frame = driver.view().tick + 1;
    for (offset, &unit_def) in train_defs.iter().enumerate() {
        submit_gameplay(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame + offset as u64,
            GameplayCommandKind::Train {
                producer: setup.local_hall,
                unit_def,
                is_building: false,
            },
        );
        sequence += 1;
    }
    record_steps(&mut driver, &mut trace, 4);
    {
        let snapshot = driver.snapshot();
        assert_eq!(
            production_defs(&snapshot, setup.local_hall),
            train_defs.to_vec(),
            "reason=unit_training_queue_visible"
        );
    }
    record_steps(&mut driver, &mut trace, 61);
    let snapshot = driver.snapshot();
    assert_eq!(
        production_defs(&snapshot, setup.local_hall),
        Vec::<DefId>::new(),
        "reason=local_training_queue_empty"
    );
    assert!(
        snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 7,
        "reason=local_units_and_buildings_produced"
    );
    let ranged_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[0], &defs);
    let siege_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[1], &defs);
    let hero_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[2], &defs);
    let hero_def = defs
        .units()
        .get(&train_defs[2])
        .expect("melee: reason=missing_hero_unit_def");
    assert!(hero_def.mana > 0, "reason=hero_definition_has_mana");
    assert_eq!(
        snapshot_position(&snapshot, hero_id).gameplay.max_mana,
        hero_def.mana,
        "reason=trained_hero_has_frozen_mana"
    );

    // 4) 英雄施法：新增 `AbilityDef` 经既有 `Cast` 结算，法力 / 冷却 / 效果链可见。
    let hero_mana_before = snapshot_position(&snapshot, hero_id).gameplay.mana;
    let target_hp_before = snapshot_position(&snapshot, setup.ai_hall).hp;
    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Cast {
            caster: hero_id,
            ability: HERO_ABILITY,
            target: setup.ai_hall,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 4);
    let ability = defs
        .abilities()
        .get(&HERO_ABILITY)
        .expect("melee: reason=missing_hero_ability");
    let expected_damage: i64 = ability
        .effects
        .iter()
        .filter(|effect| matches!(effect.kind, EffectKind::Damage))
        .map(|effect| effect.amount)
        .sum();
    assert!(expected_damage > 0, "reason=hero_ability_has_damage_effect");
    let snapshot = driver.snapshot();
    let hero = snapshot_position(&snapshot, hero_id);
    let target = snapshot_position(&snapshot, setup.ai_hall);
    assert_eq!(
        hero.gameplay.mana,
        hero_mana_before - ability.mana_cost,
        "reason=hero_mana_cost_committed"
    );
    assert!(
        hero.gameplay.cooldowns.contains_key(&HERO_ABILITY),
        "reason=hero_ability_cooldown_registered"
    );
    assert_eq!(
        target.hp,
        target_hp_before - expected_damage,
        "reason=hero_ability_damage_resolved"
    );
    assert!(
        target
            .gameplay
            .buffs
            .iter()
            .any(|buff| matches!(buff.kind, EffectKind::Stun)),
        "reason=hero_ability_stun_buff_resolved"
    );
    assert!(
        target
            .gameplay
            .buffs
            .iter()
            .any(|buff| matches!(buff.kind, EffectKind::BuffStat)),
        "reason=hero_ability_stat_buff_resolved"
    );

    // 5) 研发：攻击升级与护甲升级分别经既有 `Research` 完成并作用到存活单位。
    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Research {
            upgrade: UPGRADE_WEAPONS,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 25);
    let weapons = defs
        .upgrades()
        .get(&UPGRADE_WEAPONS)
        .expect("melee: reason=missing_weapons_upgrade");
    assert!(
        weapons.damage_mod > 0,
        "reason=weapons_upgrade_has_damage_mod"
    );
    let snapshot = driver.snapshot();
    let local = &snapshot.gameplay.players[&LOCAL_PLAYER];
    assert_eq!(local.research, None, "reason=weapons_research_completed");
    assert_eq!(
        local.upgrades.get(&UPGRADE_WEAPONS),
        Some(&true),
        "reason=weapons_upgrade_recorded"
    );
    assert!(
        snapshot_position(&snapshot, siege_id)
            .gameplay
            .stat_mods
            .damage
            >= weapons.damage_mod,
        "reason=weapons_upgrade_damage_mod_applied"
    );

    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Research {
            upgrade: UPGRADE_ARMOR,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 25);
    let armor = defs
        .upgrades()
        .get(&UPGRADE_ARMOR)
        .expect("melee: reason=missing_armor_upgrade");
    assert!(armor.hp_mod > 0, "reason=armor_upgrade_has_hp_mod");
    let snapshot = driver.snapshot();
    let local = &snapshot.gameplay.players[&LOCAL_PLAYER];
    assert_eq!(local.research, None, "reason=armor_research_completed");
    assert_eq!(
        local.upgrades.get(&UPGRADE_ARMOR),
        Some(&true),
        "reason=armor_upgrade_recorded"
    );
    let siege = snapshot_position(&snapshot, siege_id);
    assert!(
        siege.gameplay.stat_mods.max_hp >= armor.hp_mod,
        "reason=armor_upgrade_hp_mod_applied"
    );
    assert!(
        siege.gameplay.stat_mods.damage >= weapons.damage_mod,
        "reason=weapons_upgrade_persists_after_armor"
    );

    // 6) 战斗：三名新单位轮转攻击敌方主城；合成开局 AI 侧 frame-0 唯一建筑
    // 就是主城，摧毁它即满足本一局「摧毁全部建筑」判据。
    let attackers = [siege_id, ranged_id, hero_id];
    frame = driver.view().tick + 1;
    for offset in 0..8u64 {
        let attacker = attackers[(offset as usize) % attackers.len()];
        submit_simulation(
            &mut driver,
            LOCAL_PLAYER,
            sequence,
            frame + offset,
            CommandKind::Attack {
                attacker,
                target: setup.ai_hall,
            },
        );
        sequence += 1;
    }
    record_steps(&mut driver, &mut trace, 20);
    let view = driver.view();
    assert!(
        !view
            .entities
            .iter()
            .any(|entity| entity.id == setup.ai_hall.value() && entity.alive),
        "reason=ai_hall_destroyed"
    );
    let victory = driver.snapshot().gameplay.victory;
    assert!(victory.resolved, "reason=match_victory_resolved");
    assert_eq!(
        victory.winner,
        Some(LOCAL_PLAYER),
        "reason=local_player_wins"
    );
    assert!(victory.resolved_frame > 0, "reason=victory_frame_recorded");
    record_steps(&mut driver, &mut trace, 10);
    let final_snapshot = driver.snapshot();
    assert_eq!(
        final_snapshot.gameplay.victory, victory,
        "reason=victory_must_resolve_once"
    );

    // 7) 重开：相同冻结定义与开局脚本构造新一局，不残留上一局权威状态。
    let fresh = build_driver(setup, 300);
    assert!(
        !fresh.snapshot().gameplay.victory.resolved,
        "reason=restart_victory_unresolved"
    );
    assert!(
        fresh.view().entities.is_empty(),
        "reason=restart_view_empty"
    );
    let fresh_snapshot = fresh.snapshot();
    assert!(
        fresh_snapshot.positions.is_empty(),
        "reason=restart_positions_empty"
    );
    assert!(
        fresh_snapshot.gameplay.players.is_empty(),
        "reason=restart_players_empty"
    );
    assert_eq!(
        fresh_snapshot.gameplay.victory.resolved_frame, 0,
        "reason=restart_victory_frame_clear"
    );
    assert_eq!(
        fresh_snapshot.version, SNAPSHOT_VERSION,
        "reason=restart_snapshot_version_unchanged"
    );

    trace.final_snapshot = final_snapshot;
    trace.victory_frame = victory.resolved_frame;
    trace
}

/// 人族 P8.1 完整一局与基线一致：逐帧状态哈希与 `capture_snapshot()`
/// 快照逐条相等，快照仍为 v3、冻结定义内容哈希仍由 `defs_hash` 承载。
#[test]
fn human_full_match_matches_baseline_and_restart_is_clean() {
    let baseline = play_human_full_match(&MeleeSetup::new());
    assert!(
        !baseline.hashes.is_empty(),
        "reason=baseline_trace_not_empty"
    );
    assert_eq!(
        baseline.hashes.len(),
        baseline.snapshots.len(),
        "reason=hash_and_snapshot_counts_match"
    );
    assert!(
        baseline
            .snapshots
            .iter()
            .all(|snapshot| snapshot.version == SNAPSHOT_VERSION),
        "reason=all_snapshots_are_v3"
    );
    assert_eq!(SNAPSHOT_VERSION, 3, "reason=snapshot_version_unchanged");
    assert!(
        baseline.victory_frame > 0,
        "reason=baseline_victory_frame_recorded"
    );
    assert_eq!(
        baseline.final_snapshot.gameplay.victory.resolved_frame, baseline.victory_frame,
        "reason=final_snapshot_uses_recorded_victory_frame"
    );
    let defs = synthetic_definitions();
    assert_eq!(
        baseline.final_snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_uses_new_frozen_definitions"
    );
    assert_eq!(
        baseline.final_snapshot.defs, defs,
        "reason=snapshot_defs_match_synthetic_definitions"
    );

    let replay = play_human_full_match(&MeleeSetup::with_race(Race::Human));
    assert_eq!(
        replay.hashes, baseline.hashes,
        "reason=frame_hashes_match_human_baseline"
    );
    assert_eq!(
        replay.snapshots, baseline.snapshots,
        "reason=snapshot_bytes_match_human_baseline"
    );
    assert_eq!(
        replay.final_snapshot, baseline.final_snapshot,
        "reason=final_snapshot_matches_human_baseline"
    );
    assert_eq!(
        replay.victory_frame, baseline.victory_frame,
        "reason=victory_frame_matches_human_baseline"
    );
}
