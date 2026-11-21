//! 一局 1v1 Melee 的端到端 CPU 验收（change `melee-playable-loop` 组 2）。
//!
//! 全部经 `w3-core` 既有公开 API 装配，`w3-core` 零改动；测试断言采集、
//! 建造、训练、战斗、胜负结算与重开闭环可达且可重复。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::melee::defs::{
    ABILITY, BARRACKS, CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_HERO, CATEGORY_RANGED,
    CATEGORY_SIEGE, CATEGORY_SMITHY, HERO_ABILITY, HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO,
    HUMAN_RANGED, HUMAN_SIEGE, HUMAN_SMITHY, ORC_ALTAR, ORC_BARRACKS, ORC_FARM, ORC_HERO,
    ORC_HERO_ABILITY, ORC_RANGED, ORC_SIEGE, ORC_SMITHY, ORC_SOLDIER, ORC_UPGRADE_ARMOR,
    ORC_UPGRADE_WEAPONS, SOLDIER, UPGRADE_ARMOR, UPGRADE_WEAPONS,
};
use w3_client::melee::{
    AI_PLAYER, AiScript, LOCAL_PLAYER, MeleeDriver, MeleeSetup, Race, synthetic_definitions,
    synthetic_world,
};
use w3_client::{
    ClientCommand, FixedStepClient, LOGIC_FPS, MovabilityMap, StepDriver, TICKS_PER_LOGIC_FRAME,
    TICKS_PER_SECOND, filter_movable,
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
    build_driver_with_defs(setup, horizon, synthetic_definitions())
}

/// 以指定冻结定义装配一局（反向验证注入退化定义用，D9 / tasks 6.3）。
/// 全部命令仍只经既有 `submit_command` / `submit_gameplay_command` 提交，
/// `step()` 全程由整数累加器驱动（ADR 0002），退化只发生在冻结定义内容。
fn build_driver_with_defs(
    setup: &MeleeSetup,
    horizon: u64,
    defs: FrozenDefinitions,
) -> MeleeDriver {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim = RtsMatch::create_with_definitions(config, synthetic_world(), defs);
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
/// （design D2：种族选择不引入运行时权威状态，`SNAPSHOT_VERSION` 仍为 3；P8.5a 起为 4、
/// P8.5b 起为 5、P8.5c 起为 6、P8.5d 起为 7；本行属历史沿革叙述，按先例保留并加夹注）。
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
/// P8.3：`category_def_ids()` 保留为**本地份别名**，供本地验收取用；
/// AI 脚本另有 `ai_category_def_ids()`（AI 份），两者 MUST NOT 串用。
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

/// P8.2 兽族新类别装配绑定；显式核对六类槽位仍指向 `2xx` 真实新常量
/// （P8.2 已替换 `ORC_SOLDIER` / `ORC_BARRACKS` 占位）。
/// P8.3：`category_def_ids()` 保留为**本地份别名**，供本地验收取用；
/// AI 脚本另有 `ai_category_def_ids()`（AI 份），两者 MUST NOT 串用。
fn orc_catalog(setup: &MeleeSetup) -> BTreeMap<u64, DefId> {
    assert_eq!(
        setup.race,
        Race::Orc,
        "reason=orc_full_match_requires_orc_setup"
    );
    let catalog = setup.category_def_ids().clone();
    let expected = [
        (CATEGORY_RANGED, ORC_RANGED),
        (CATEGORY_SIEGE, ORC_SIEGE),
        (CATEGORY_FARM, ORC_FARM),
        (CATEGORY_ALTAR, ORC_ALTAR),
        (CATEGORY_SMITHY, ORC_SMITHY),
        (CATEGORY_HERO, ORC_HERO),
    ];
    for (slot, def_id) in expected {
        assert_eq!(
            catalog.get(&slot),
            Some(&def_id),
            "reason=orc_category_binding slot={slot}"
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
/// 快照逐条相等，快照仍为 v7、冻结定义内容哈希仍由 `defs_hash` 承载。
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
        "reason=all_snapshots_are_v7"
    );
    assert_eq!(SNAPSHOT_VERSION, 7, "reason=snapshot_version_unchanged");
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

// ---------------------------------------------------------------------------
// P8.2 兽族完整一局（design D1–D10 / tasks 3.3 / 3.4 / 6.x）
//
// 与 P8.1 人族全阶段一局对称：兽族段 `2xx` 六类真实新定义 + 兽族英雄主动
// 技能 + 兽族攻 / 防升级，`Race` 分派替换占位、技能 / 升级 ID 按族查表。
// `defs_hash` 因新增定义改变，故这里断言「同一兽族脚本两实例逐帧一致」，
// MUST NOT 与改动前旧基线的哈希数值相等（design D6 / 参考对照 G5）。
// ---------------------------------------------------------------------------

/// P8.2 兽族完整一局：采集 → 建造（地洞 / 祭坛 / 战争磨坊）→ 训练（远程 /
/// 反重甲 / 英雄）→ 英雄施法（兽族主动技能）→ 研发（兽族攻 / 防升级）→
/// 战斗 → 摧毁全部建筑结算 → 重开。AI 外部脚本按 300 帧窗口注入，胜负仍由
/// `MeleeDriver` 检测敌方主城消失后提交 `Leave` 触发既有 `resolve_victory`。
/// 技能 / 升级 ID 经 `setup.race` 只读分派取用（design D3 / D5）。
fn play_orc_full_match(setup: &MeleeSetup) -> FullMatchTrace {
    let defs = synthetic_definitions();
    let catalog = orc_catalog(setup);
    // 按族只读分派取技能 / 升级 ID，同时核对其确为兽族新增 ID。
    let hero_ability = setup.race.hero_ability();
    let weapons_upgrade = setup.race.weapons_upgrade();
    let armor_upgrade = setup.race.armor_upgrade();
    assert_eq!(hero_ability, ORC_HERO_ABILITY);
    assert_eq!(weapons_upgrade, ORC_UPGRADE_WEAPONS);
    assert_eq!(armor_upgrade, ORC_UPGRADE_ARMOR);
    let mut driver = build_driver(setup, 300);
    let mut trace = FullMatchTrace {
        hashes: Vec::new(),
        snapshots: Vec::new(),
        final_snapshot: driver.snapshot(),
        victory_frame: 0,
    };
    let mut sequence = 100u32;
    record_steps(&mut driver, &mut trace, 1);

    // 1) 采集：多条 `Gather` 让本地黄金确定增长（R3.1 / R1.5）。
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
        "reason=orc_gather_grows_local_gold"
    );

    // 2) 建造：兵营 + 人口地洞 + 英雄祭坛 + 战争磨坊，队列可见后完成。
    // 三类新增建筑 `speed_raw == 0`，生产出的运行时实体 speed 也必须为 0
    // （R1.4 / R13.6 / design D7）。
    let before_build: BTreeSet<u64> = driver
        .view()
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect();
    let build_defs = [
        ORC_BARRACKS,
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
            "reason=orc_building_queue_visible"
        );
        assert!(
            queue.iter().all(|item| item.remaining_frames > 0),
            "reason=orc_building_queue_remaining_frames_positive"
        );
    }
    record_steps(&mut driver, &mut trace, 55);
    let snapshot = driver.snapshot();
    assert_eq!(
        production_defs(&snapshot, setup.local_hall),
        Vec::<DefId>::new(),
        "reason=orc_building_queue_empty"
    );
    assert!(
        snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 4,
        "reason=orc_building_produced_count"
    );
    assert!(
        driver.view().entities.len() >= before_build.len() + 4,
        "reason=four_orc_buildings_spawned"
    );
    let built_entities: Vec<EntityId> = build_defs
        .iter()
        .map(|&def_id| produced_entity(&snapshot, &before_build, LOCAL_PLAYER, def_id, &defs))
        .collect();
    for entity in &built_entities {
        assert_eq!(
            snapshot_position(&snapshot, *entity).speed,
            0,
            "reason=orc_new_building_runtime_speed_zero"
        );
    }

    // 3) 训练：远程 / 反重甲 / 英雄，逐帧推进至队列清空；英雄带 mana。
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
            "reason=orc_unit_training_queue_visible"
        );
    }
    record_steps(&mut driver, &mut trace, 61);
    let snapshot = driver.snapshot();
    assert_eq!(
        production_defs(&snapshot, setup.local_hall),
        Vec::<DefId>::new(),
        "reason=orc_training_queue_empty"
    );
    assert!(
        snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 7,
        "reason=orc_units_and_buildings_produced"
    );
    let ranged_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[0], &defs);
    let siege_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[1], &defs);
    let hero_id = produced_entity(&snapshot, &before_train, LOCAL_PLAYER, train_defs[2], &defs);
    let hero_def = defs
        .units()
        .get(&train_defs[2])
        .expect("melee: reason=missing_orc_hero_unit_def");
    assert!(hero_def.mana > 0, "reason=orc_hero_definition_has_mana");
    assert_eq!(
        snapshot_position(&snapshot, hero_id).gameplay.max_mana,
        hero_def.mana,
        "reason=trained_orc_hero_has_frozen_mana"
    );

    // 4) 英雄施法：兽族 `AbilityDef` 经既有 `Cast` 结算，法力 / 冷却 / 效果链可见。
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
            ability: hero_ability,
            target: setup.ai_hall,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 4);
    let ability = defs
        .abilities()
        .get(&hero_ability)
        .expect("melee: reason=missing_orc_hero_ability");
    let expected_damage: i64 = ability
        .effects
        .iter()
        .filter(|effect| matches!(effect.kind, EffectKind::Damage))
        .map(|effect| effect.amount)
        .sum();
    assert!(
        expected_damage > 0,
        "reason=orc_hero_ability_has_damage_effect"
    );
    let snapshot = driver.snapshot();
    let hero = snapshot_position(&snapshot, hero_id);
    let target = snapshot_position(&snapshot, setup.ai_hall);
    assert_eq!(
        hero.gameplay.mana,
        hero_mana_before - ability.mana_cost,
        "reason=orc_hero_mana_cost_committed"
    );
    assert!(
        hero.gameplay.cooldowns.contains_key(&hero_ability),
        "reason=orc_hero_ability_cooldown_registered"
    );
    assert_eq!(
        target.hp,
        target_hp_before - expected_damage,
        "reason=orc_hero_ability_damage_resolved"
    );
    assert!(
        target
            .gameplay
            .buffs
            .iter()
            .any(|buff| matches!(buff.kind, EffectKind::Stun)),
        "reason=orc_hero_ability_stun_buff_resolved"
    );
    assert!(
        target
            .gameplay
            .buffs
            .iter()
            .any(|buff| matches!(buff.kind, EffectKind::BuffStat)),
        "reason=orc_hero_ability_stat_buff_resolved"
    );

    // 5) 研发：兽族攻击升级与护甲升级分别经既有 `Research` 完成并作用到单位。
    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Research {
            upgrade: weapons_upgrade,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 25);
    let weapons = defs
        .upgrades()
        .get(&weapons_upgrade)
        .expect("melee: reason=missing_orc_weapons_upgrade");
    assert!(
        weapons.damage_mod > 0,
        "reason=orc_weapons_upgrade_has_damage_mod"
    );
    let snapshot = driver.snapshot();
    let local = &snapshot.gameplay.players[&LOCAL_PLAYER];
    assert_eq!(
        local.research, None,
        "reason=orc_weapons_research_completed"
    );
    assert_eq!(
        local.upgrades.get(&weapons_upgrade),
        Some(&true),
        "reason=orc_weapons_upgrade_recorded"
    );
    assert!(
        snapshot_position(&snapshot, siege_id)
            .gameplay
            .stat_mods
            .damage
            >= weapons.damage_mod,
        "reason=orc_weapons_upgrade_damage_mod_applied"
    );

    frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        sequence,
        frame,
        GameplayCommandKind::Research {
            upgrade: armor_upgrade,
        },
    );
    sequence += 1;
    record_steps(&mut driver, &mut trace, 25);
    let armor = defs
        .upgrades()
        .get(&armor_upgrade)
        .expect("melee: reason=missing_orc_armor_upgrade");
    assert!(armor.hp_mod > 0, "reason=orc_armor_upgrade_has_hp_mod");
    let snapshot = driver.snapshot();
    let local = &snapshot.gameplay.players[&LOCAL_PLAYER];
    assert_eq!(local.research, None, "reason=orc_armor_research_completed");
    assert_eq!(
        local.upgrades.get(&armor_upgrade),
        Some(&true),
        "reason=orc_armor_upgrade_recorded"
    );
    let siege = snapshot_position(&snapshot, siege_id);
    assert!(
        siege.gameplay.stat_mods.max_hp >= armor.hp_mod,
        "reason=orc_armor_upgrade_hp_mod_applied"
    );
    assert!(
        siege.gameplay.stat_mods.damage >= weapons.damage_mod,
        "reason=orc_weapons_upgrade_persists_after_armor"
    );

    // 6) 战斗：三名兽族新单位轮转攻击敌方主城；合成开局 AI 侧 frame-0
    // 唯一建筑就是主城，摧毁它即满足本一局「摧毁全部建筑」判据。
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
        "reason=orc_ai_hall_destroyed"
    );
    let victory = driver.snapshot().gameplay.victory;
    assert!(victory.resolved, "reason=orc_match_victory_resolved");
    assert_eq!(
        victory.winner,
        Some(LOCAL_PLAYER),
        "reason=orc_local_player_wins"
    );
    assert!(
        victory.resolved_frame > 0,
        "reason=orc_victory_frame_recorded"
    );
    record_steps(&mut driver, &mut trace, 10);
    let final_snapshot = driver.snapshot();
    assert_eq!(
        final_snapshot.gameplay.victory, victory,
        "reason=orc_victory_must_resolve_once"
    );

    // 7) 重开：相同冻结定义与开局脚本构造新一局，不残留上一局权威状态。
    let fresh = build_driver(setup, 300);
    assert!(
        !fresh.snapshot().gameplay.victory.resolved,
        "reason=orc_restart_victory_unresolved"
    );
    assert!(
        fresh.view().entities.is_empty(),
        "reason=orc_restart_view_empty"
    );
    let fresh_snapshot = fresh.snapshot();
    assert!(
        fresh_snapshot.positions.is_empty(),
        "reason=orc_restart_positions_empty"
    );
    assert!(
        fresh_snapshot.gameplay.players.is_empty(),
        "reason=orc_restart_players_empty"
    );
    assert_eq!(
        fresh_snapshot.gameplay.victory.resolved_frame, 0,
        "reason=orc_restart_victory_frame_clear"
    );
    assert_eq!(
        fresh_snapshot.version, SNAPSHOT_VERSION,
        "reason=orc_restart_snapshot_version_unchanged"
    );

    trace.final_snapshot = final_snapshot;
    trace.victory_frame = victory.resolved_frame;
    trace
}

/// P8.2 兽族完整一局与基线一致：同一兽族脚本两实例逐帧状态哈希与
/// `capture_snapshot()` 快照逐条相等，快照仍 v7、冻结定义内容哈希仍由
/// `defs_hash` 承载（不依赖改动前的旧哈希数值）。胜负只结算一次、重开干净。
#[test]
fn orc_full_match_matches_baseline_and_restart_is_clean() {
    let baseline = play_orc_full_match(&MeleeSetup::with_race(Race::Orc));
    assert!(
        !baseline.hashes.is_empty(),
        "reason=orc_baseline_trace_not_empty"
    );
    assert_eq!(
        baseline.hashes.len(),
        baseline.snapshots.len(),
        "reason=orc_hash_and_snapshot_counts_match"
    );
    assert!(
        baseline
            .snapshots
            .iter()
            .all(|snapshot| snapshot.version == SNAPSHOT_VERSION),
        "reason=all_orc_snapshots_are_v7"
    );
    assert_eq!(SNAPSHOT_VERSION, 7, "reason=orc_snapshot_version_unchanged");
    assert!(
        baseline.victory_frame > 0,
        "reason=orc_baseline_victory_frame_recorded"
    );
    assert_eq!(
        baseline.final_snapshot.gameplay.victory.resolved_frame, baseline.victory_frame,
        "reason=orc_final_snapshot_uses_recorded_victory_frame"
    );
    // 新增定义是冻结定义内容变化：`defs_hash` 由 `synthetic_definitions()`
    // 的 `content_hash()` 承载，进入 v7 快照头部（C2 / D2）。
    let defs = synthetic_definitions();
    assert_eq!(
        baseline.final_snapshot.defs_hash,
        defs.content_hash(),
        "reason=orc_defs_hash_uses_new_frozen_definitions"
    );
    assert_eq!(
        baseline.final_snapshot.defs, defs,
        "reason=orc_snapshot_defs_match_synthetic_definitions"
    );

    // 两实例注入同一兽族脚本，逐帧字节一致（非与改动前旧基线数值相等）。
    let replay = play_orc_full_match(&MeleeSetup::with_race(Race::Orc));
    assert_eq!(
        replay.hashes, baseline.hashes,
        "reason=orc_frame_hashes_match_baseline"
    );
    assert_eq!(
        replay.snapshots, baseline.snapshots,
        "reason=orc_snapshot_bytes_match_baseline"
    );
    assert_eq!(
        replay.final_snapshot, baseline.final_snapshot,
        "reason=orc_final_snapshot_matches_baseline"
    );
    assert_eq!(
        replay.victory_frame, baseline.victory_frame,
        "reason=orc_victory_frame_matches_baseline"
    );
}

/// 退化冻结定义（缺陷态，D9 / tasks 6.3）：把兽族英雄 `mana` 退化为 0，
/// 模拟「漏用 `unit_with_mana`」。经真实 `FrozenDefinitions::new` 重建后
/// 喂给 `build_driver_with_defs`，退化注入走真实 `Cast` / `apply` 路径，
/// 不是旁路模拟。
fn degenerate_defs_with_mana_less_orc_hero() -> FrozenDefinitions {
    let full = synthetic_definitions();
    let units: Vec<(DefId, UnitDef)> = full
        .units()
        .iter()
        .map(|(&id, &def)| {
            let mut def = def;
            if id == ORC_HERO {
                def.mana = 0;
            }
            (id, def)
        })
        .collect();
    FrozenDefinitions::new(
        units,
        full.abilities()
            .iter()
            .map(|(&id, d)| (id, d.clone()))
            .collect(),
        full.items().iter().map(|(&id, &d)| (id, d)).collect(),
        full.upgrades().iter().map(|(&id, &d)| (id, d)).collect(),
    )
    .expect("melee: reason=degenerate_mana_defs_invalid")
    // P8.5a：同源登记复活段，使缺陷态只差「mana 退化」这一项，
    // 保住 `content_hash()` 元断言的判别力（CODEBUDDY §5 / 不静默降级）。
    // P8.5b：四张定义表均从 `full` 的公开 getter 克隆，新增 8101 / 8102
    // 真实定义自动同源；同理同源登记主城升级段，避免段差异稀释判别力。
    // P8.5c：同源登记野怪营地段，使 mana 退化仍是唯一差异。
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
    // P8.5d：同源登记商店段，避免新增冻结段稀释退化装配的内容哈希比较。
    .with_item_shop(full.item_shop().clone())
}

/// 经真实 `Train` → `Cast` 路径跑一次兽族英雄施法，返回「施法确实结算」
/// （法力扣减 + 冷却登记 + 目标按 `EffectDef` 扣血）是否成立。英雄 mana=0
/// 时 `commit_mana_cost` 失败、施法被内核既有入口拒绝，三项断言不成立。
fn orc_hero_cast_resolves(defs: &FrozenDefinitions) -> bool {
    let setup = MeleeSetup::with_race(Race::Orc);
    let mut driver = build_driver_with_defs(&setup, 0, defs.clone());
    advance(&mut driver, 1);
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
        100,
        train_frame,
        GameplayCommandKind::Train {
            producer: setup.local_hall,
            unit_def: ORC_HERO,
            is_building: false,
        },
    );
    // ORC_HERO `train_frames = 60`，推进到完成帧之后。
    advance(&mut driver, 62);
    let snapshot = driver.snapshot();
    let hero = produced_entity(&snapshot, &before, LOCAL_PLAYER, ORC_HERO, defs);
    let hero_mana_before = snapshot_position(&snapshot, hero).gameplay.mana;
    let target_hp_before = snapshot_position(&snapshot, setup.ai_hall).hp;
    let cast_frame = driver.view().tick + 1;
    submit_gameplay(
        &mut driver,
        LOCAL_PLAYER,
        101,
        cast_frame,
        GameplayCommandKind::Cast {
            caster: hero,
            ability: ORC_HERO_ABILITY,
            target: setup.ai_hall,
        },
    );
    // `cast_frames = 3`，推进到效果链结算之后。
    advance(&mut driver, 4);
    let ability = defs
        .abilities()
        .get(&ORC_HERO_ABILITY)
        .expect("melee: reason=missing_orc_hero_ability");
    let expected_damage: i64 = ability
        .effects
        .iter()
        .filter(|effect| matches!(effect.kind, EffectKind::Damage))
        .map(|effect| effect.amount)
        .sum();
    let after = driver.snapshot();
    let hero_after = snapshot_position(&after, hero);
    let target_after = snapshot_position(&after, setup.ai_hall);
    hero_after.gameplay.mana == hero_mana_before - ability.mana_cost
        && hero_after
            .gameplay
            .cooldowns
            .contains_key(&ORC_HERO_ABILITY)
        && target_after.hp == target_hp_before - expected_damage
}

/// 反向验证·修复态（D9 / tasks 6.3）：正式定义（英雄 `mana > 0`）经真实
/// `Cast` / `apply` 路径，施法确实结算（法力 / 冷却 / 伤害三项成立）。
#[test]
fn reverse_validation_orc_hero_mana_formal_cast_resolves() {
    assert!(
        orc_hero_cast_resolves(&synthetic_definitions()),
        "reason=formal_orc_hero_cast_must_resolve"
    );
}

/// 反向验证·缺陷态（D9 / tasks 6.3）：英雄 `mana == 0` 的退化定义经同一
/// 真实 `Cast` / `apply` 路径必然被拒，三项施法断言失败；同时证明正式
/// `content_hash()` 能抓出该缺陷（元断言防退化）。
#[test]
fn reverse_validation_orc_hero_mana_degenerate_cast_fails_the_same_assertion() {
    let degenerate = degenerate_defs_with_mana_less_orc_hero();
    assert_eq!(degenerate.units().get(&ORC_HERO).unwrap().mana, 0);
    assert!(
        !orc_hero_cast_resolves(&degenerate),
        "reason=degenerate_mana_defs_must_fail_the_same_cast_assertion"
    );
    // 正式定义仍能通过同一断言，且正式 `content_hash()` 与退化定义不同。
    let formal = synthetic_definitions();
    assert_ne!(formal.content_hash(), degenerate.content_hash());
    assert!(orc_hero_cast_resolves(&formal));
}

/// ④ 端到端断言反向验证（D9 / tasks 6.4）：`play_orc_full_match` 使用的
/// `orc_catalog` 断言在「兽族六个新类别仍回落 `ORC_SOLDIER` /
/// `ORC_BARRACKS` 占位」的退化装配绑定下必然失败，正式绑定通过。
/// 退化绑定经真实 `MeleeSetup` 字段注入、喂给同一断言函数（元断言）。
/// P8.3：本注入点显式指向**本地份** `category_def_ids`——`orc_catalog_holds`
/// 是本地验收断言、读 `setup.category_def_ids()`（本地份别名）；AI 脚本另有
/// AI 份 `ai_category_def_ids`，两处退化注入点各指向自身断言读取的正确份。
#[test]
fn reverse_validation_orc_catalog_placeholders_fail_end_to_end_assertion() {
    let formal = MeleeSetup::with_race(Race::Orc);
    assert!(
        orc_catalog_holds(&formal),
        "reason=formal_orc_catalog_passes"
    );

    let mut degenerate = MeleeSetup::with_race(Race::Orc);
    degenerate.category_def_ids = [
        (CATEGORY_RANGED, ORC_SOLDIER),
        (CATEGORY_SIEGE, ORC_SOLDIER),
        (CATEGORY_FARM, ORC_BARRACKS),
        (CATEGORY_ALTAR, ORC_BARRACKS),
        (CATEGORY_SMITHY, ORC_BARRACKS),
        (CATEGORY_HERO, ORC_SOLDIER),
    ]
    .into_iter()
    .collect();
    assert!(
        !orc_catalog_holds(&degenerate),
        "reason=degenerate_placeholder_catalog_must_fail"
    );
    assert!(orc_catalog_holds(&formal), "reason=formal_still_passes");
}

/// 被反向验证的端到端断言：兽族六类槽位绑定到真实 `2xx` 新常量
/// （`orc_catalog` 内断言的布尔化版本，供缺陷态 / 修复态成对取证）。
fn orc_catalog_holds(setup: &MeleeSetup) -> bool {
    let catalog = setup.category_def_ids();
    let expected = [
        (CATEGORY_RANGED, ORC_RANGED),
        (CATEGORY_SIEGE, ORC_SIEGE),
        (CATEGORY_FARM, ORC_FARM),
        (CATEGORY_ALTAR, ORC_ALTAR),
        (CATEGORY_SMITHY, ORC_SMITHY),
        (CATEGORY_HERO, ORC_HERO),
    ];
    expected
        .iter()
        .all(|(slot, def_id)| catalog.get(slot) == Some(def_id))
}

/// 双刷新率逐帧验收（ADR 0002 §2 / 验收）：兽族开局 + 兽族 AI 脚本一次性
/// 帧冻结入队后，60 Hz / 144 Hz 呈现节奏喂入同一整数节拍序列，逐帧帧号与
/// `compute_state_hash()` 完全一致；累加器只数整数帧，`step()` 不接呈现帧率。
fn orc_replay_trace(refresh_hz: u64, seconds: u64) -> Vec<(u64, u64)> {
    let ticks_per_redraw = TICKS_PER_SECOND / refresh_hz;
    assert_eq!(ticks_per_redraw * refresh_hz, TICKS_PER_SECOND);
    let setup = MeleeSetup::with_race(Race::Orc);
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim =
        RtsMatch::create_with_definitions(config, synthetic_world(), synthetic_definitions());
    for command in &setup.commands {
        sim.submit_command(*command)
            .expect("melee: reason=replay_spawn_rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("melee: reason=replay_opening_gameplay_rejected");
    }
    for command in AiScript::new(&setup, 300).commands {
        sim.submit_gameplay_command(command)
            .expect("melee: reason=replay_ai_command_rejected");
    }
    let mut client = FixedStepClient::from_match(sim);
    let mut trace = Vec::new();
    for _ in 0..(refresh_hz * seconds) {
        client.advance_with_observer(ticks_per_redraw, |s| {
            trace.push((s.read_view().tick, s.compute_state_hash()));
        });
    }
    trace
}

#[test]
fn orc_replay_is_refresh_rate_independent_frame_by_frame() {
    let seconds = 2;
    let at_60 = orc_replay_trace(60, seconds);
    let at_144 = orc_replay_trace(144, seconds);
    let expected_frames = u64::from(LOGIC_FPS) * seconds;
    assert_eq!(
        at_60.len() as u64,
        expected_frames,
        "reason=60hz_frame_count"
    );
    assert_eq!(
        at_144.len() as u64,
        expected_frames,
        "reason=144hz_frame_count"
    );
    assert_eq!(at_60, at_144, "reason=60hz_144hz_frame_hash_identical");
    // 逐帧帧号连续递增，防止「状态从未变化」造成的空泛通过。
    for (index, (tick, _)) in at_60.iter().enumerate() {
        assert_eq!(*tick, index as u64 + 1, "reason=orc_frame_number_monotonic");
    }
    assert!(
        at_60.windows(2).any(|pair| pair[0].1 != pair[1].1),
        "reason=orc_simulation_actually_advanced"
    );
}

// ---------------------------------------------------------------------------
// P8.3 双族对抗四组合端到端（design D1 / D2 / D3 / tasks 3.x / 6.1 / C4）
//
// 验收口径（关键）：只断言「同一组合两实例逐帧 compute_state_hash() /
// capture_snapshot() 字节一致」；MUST NOT 断言四组合之间哈希数值相等、
// 也 MUST NOT 断言与改动前旧基线数值相等（AI 命令落各自 ai_race 段，
// 跨组合字节本就不同；本 change 不加新定义，旧基线仍由既有测试单独守护）。
// ---------------------------------------------------------------------------

/// 四组合（本地族, AI 族）：人vs人 / 人vs兽 / 兽vs人 / 兽vs兽。
const DUAL_COMBINATIONS: [(Race, Race); 4] = [
    (Race::Human, Race::Human),
    (Race::Human, Race::Orc),
    (Race::Orc, Race::Human),
    (Race::Orc, Race::Orc),
];

/// 某族 `units` 段位（人族 1xx / 兽族 2xx / 中立资源 9xx）。
fn dual_race_segment(race: Race) -> std::ops::Range<DefId> {
    match race {
        Race::Human => 100..200,
        Race::Orc => 200..300,
    }
}

/// 只读断言：四组合装配的 `def_ids` 按席位落各自族段——本地主城（1）/
/// 农民（2,3,4）落 `local_race` 段，AI 主城（7）/ 农民（8,9,10）落
/// `ai_race` 段，中立资源（5,6,11,12）恒 `9xx`；并被 ② 反向验证复用。
fn seats_land_in_race_segments(setup: &MeleeSetup, local: Race, ai: Race) -> bool {
    let local_segment = dual_race_segment(local);
    let ai_segment = dual_race_segment(ai);
    let neutral: std::ops::Range<DefId> = 900..1000;
    for id in [1u64, 2, 3, 4] {
        if !setup
            .def_ids
            .get(&id)
            .is_some_and(|def_id| local_segment.contains(def_id))
        {
            return false;
        }
    }
    for id in [7u64, 8, 9, 10] {
        if !setup
            .def_ids
            .get(&id)
            .is_some_and(|def_id| ai_segment.contains(def_id))
        {
            return false;
        }
    }
    for id in [5u64, 6, 11, 12] {
        if !setup
            .def_ids
            .get(&id)
            .is_some_and(|def_id| neutral.contains(def_id))
        {
            return false;
        }
    }
    true
}

/// 被四组合正向验收与 ① 反向验证共用的布尔断言：`AiScript::new` 产出的
/// 训练 / 施法 / 研发目标 `DefId` 落在 `ai_race` 段 / 按族分派集合，
/// MUST NOT 泄漏本地或对家族段 ID；两族共用的基线 `SOLDIER`（1）与骚扰
/// 技能 `ABILITY`（10）单独放行，三类命令都须出现以避免平凡通过。
fn ai_commands_land_in_ai_race(setup: &MeleeSetup, ai_race: Race) -> bool {
    let catalog = ai_race.category_def_ids();
    let hero_ability = ai_race.hero_ability();
    let weapons_upgrade = ai_race.weapons_upgrade();
    let armor_upgrade = ai_race.armor_upgrade();
    let segment = dual_race_segment(ai_race);
    let mut saw_train = false;
    let mut saw_cast = false;
    let mut saw_research = false;
    for command in AiScript::new(setup, 1200).commands {
        if command.player_id != AI_PLAYER {
            return false;
        }
        match command.kind {
            GameplayCommandKind::Train { unit_def, .. } => {
                if unit_def == SOLDIER {
                    continue;
                }
                if !segment.contains(&unit_def) || !catalog.values().any(|&id| id == unit_def) {
                    return false;
                }
                saw_train = true;
            }
            GameplayCommandKind::Cast { ability, .. } => {
                if ability == ABILITY {
                    continue;
                }
                if ability != hero_ability {
                    return false;
                }
                saw_cast = true;
            }
            GameplayCommandKind::Research { upgrade } => {
                if upgrade != weapons_upgrade && upgrade != armor_upgrade {
                    return false;
                }
                saw_research = true;
            }
            _ => {}
        }
    }
    saw_train && saw_cast && saw_research
}

/// 按本地族复用既有完整一局脚本（采集 → 建造 → 训练 → 英雄施法 → 研发 →
/// 战斗 → 摧毁敌方主城结算 → 重开）：两族函数均以 `setup.race`（本地族
/// 别名）取本地份绑定与技能 / 升级 ID，AI 外部脚本由 `build_driver` 按
/// `setup.ai_race` 注入。四组合差别只在装配层与 AI 命令，不进内核。
fn play_dual_faction_full_match(setup: &MeleeSetup) -> FullMatchTrace {
    match setup.race {
        Race::Human => play_human_full_match(setup),
        Race::Orc => play_orc_full_match(setup),
    }
}

/// 四组合 CPU 端到端一局与确定性闭环（design D3 / C4 / tasks 3.x）：
/// 各组合装配并打完一局，胜负只结算一次、重开无残留、快照仍 v7；
/// 只断言组内两实例逐帧哈希 / 快照字节一致，不断言跨组合或旧基线相等。
#[test]
fn four_race_combinations_play_full_match_with_within_combo_frame_hashes() {
    assert_eq!(SNAPSHOT_VERSION, 7, "reason=snapshot_version_unchanged");
    for (local, ai) in DUAL_COMBINATIONS {
        let setup = MeleeSetup::with_races(local, ai);
        assert_eq!(setup.race, local, "reason=local_race_alias local={local:?}");
        assert_eq!(setup.ai_race, ai, "reason=ai_race_field ai={ai:?}");
        assert!(
            seats_land_in_race_segments(&setup, local, ai),
            "reason=def_ids_segmented_by_seat local={local:?} ai={ai:?}"
        );
        assert!(
            ai_commands_land_in_ai_race(&setup, ai),
            "reason=ai_commands_in_ai_segment local={local:?} ai={ai:?}"
        );

        // 同一组合两实例：逐帧 `compute_state_hash()` 与 `capture_snapshot()`
        // 字节一致（不只比最终 hash）。两实例共用同一装配与同一 AI 脚本。
        let first = play_dual_faction_full_match(&setup);
        let second = play_dual_faction_full_match(&setup);
        assert!(
            !first.hashes.is_empty(),
            "reason=per_frame_hashes_not_empty local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.hashes.len(),
            first.snapshots.len(),
            "reason=hash_snapshot_counts_match local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.hashes, second.hashes,
            "reason=within_combo_frame_hashes_match local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.snapshots, second.snapshots,
            "reason=within_combo_snapshots_match local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.final_snapshot, second.final_snapshot,
            "reason=within_combo_final_snapshot_match local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.victory_frame, second.victory_frame,
            "reason=within_combo_victory_frame_match local={local:?} ai={ai:?}"
        );

        // 胜负只结算一次（既有脚本已断言）、结算帧进权威状态、快照 v7。
        assert!(
            first.final_snapshot.gameplay.victory.resolved,
            "reason=victory_resolved local={local:?} ai={ai:?}"
        );
        assert_eq!(
            first.final_snapshot.gameplay.victory.winner,
            Some(LOCAL_PLAYER),
            "reason=local_player_wins local={local:?} ai={ai:?}"
        );
        assert!(
            first.victory_frame > 0,
            "reason=victory_frame_recorded local={local:?} ai={ai:?}"
        );
        assert!(
            first
                .snapshots
                .iter()
                .all(|snapshot| snapshot.version == SNAPSHOT_VERSION),
            "reason=all_snapshots_v7 local={local:?} ai={ai:?}"
        );

        // 重开无残留：`victory.resolved == false`、entities / positions /
        // players 视图为空、结算帧清零，且 `SNAPSHOT_VERSION == 7`。
        let fresh = build_driver(&setup, 300);
        assert!(
            !fresh.snapshot().gameplay.victory.resolved,
            "reason=restart_victory_unresolved local={local:?} ai={ai:?}"
        );
        assert!(
            fresh.view().entities.is_empty(),
            "reason=restart_view_empty local={local:?} ai={ai:?}"
        );
        let fresh_snapshot = fresh.snapshot();
        assert!(
            fresh_snapshot.positions.is_empty(),
            "reason=restart_positions_empty local={local:?} ai={ai:?}"
        );
        assert!(
            fresh_snapshot.gameplay.players.is_empty(),
            "reason=restart_players_empty local={local:?} ai={ai:?}"
        );
        assert_eq!(
            fresh_snapshot.gameplay.victory.resolved_frame, 0,
            "reason=restart_victory_frame_clear local={local:?} ai={ai:?}"
        );
        assert_eq!(
            fresh_snapshot.version, SNAPSHOT_VERSION,
            "reason=restart_snapshot_v7 local={local:?} ai={ai:?}"
        );
    }
    // MUST NOT 断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」：
    // 四组合 AI 命令落在各自 ai_race 段、字节本就不同；既有两族基线 /
    // 回落测试单独守护旧口径，不在此处跨组合比较。
}

/// 同一整数节拍序列按不同批次喂入，逐帧采集 `(帧号, compute_state_hash)`。
/// `step()` 不接呈现帧率 / delta，全部命令开局一次性按冻结帧入队，故
/// 批次粒度 MUST NOT 改变逐帧哈希（design D2 / R12.2 / ADR 0002）。
fn dual_faction_replay_trace(
    setup: &MeleeSetup,
    ticks_per_batch: u64,
    batches: u64,
) -> Vec<(u64, u64)> {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim =
        RtsMatch::create_with_definitions(config, synthetic_world(), synthetic_definitions());
    for command in &setup.commands {
        sim.submit_command(*command)
            .expect("melee: reason=dual_replay_spawn_rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("melee: reason=dual_replay_opening_gameplay_rejected");
    }
    for command in AiScript::new(setup, 300).commands {
        sim.submit_gameplay_command(command)
            .expect("melee: reason=dual_replay_ai_command_rejected");
    }
    let mut client = FixedStepClient::from_match(sim);
    let mut trace = Vec::new();
    for _ in 0..batches {
        client.advance_with_observer(ticks_per_batch, |s| {
            trace.push((s.read_view().tick, s.compute_state_hash()));
        });
    }
    trace
}

/// 双刷新率（60 Hz / 144 Hz）与批次推进四组合逐帧验收：同一组合两实例
/// 脚本逐字节相等，一端逐帧、另一端一次推进多帧，两端逐帧哈希一致；
/// 帧号连续递增且状态确有变化（防止「状态从未变化」的空泛通过）。
#[test]
fn four_race_combinations_replay_is_frame_frozen_and_refresh_rate_independent() {
    let seconds = 2;
    let expected_frames = u64::from(LOGIC_FPS) * seconds;
    for (local, ai) in DUAL_COMBINATIONS {
        let setup = MeleeSetup::with_races(local, ai);
        // 同一组合构造两次 AI 外部脚本逐字节相等（帧冻结、可重复）。
        assert_eq!(
            AiScript::new(&setup, 300),
            AiScript::new(&setup, 300),
            "reason=ai_script_byte_equal local={local:?} ai={ai:?}"
        );

        // 一端逐帧 / 另一端一次推进 8 帧（`MAX_CATCHUP_STEPS = 8` 上限
        // 边界），共推进相同整数帧数 `BATched_FRAMES = 64`，两端逐帧
        // `compute_state_hash()` 一致；批次数整除 8，避免两端帧数不等。
        const BATCHED_FRAMES: u64 = 64;
        let per_frame = dual_faction_replay_trace(&setup, TICKS_PER_LOGIC_FRAME, BATCHED_FRAMES);
        let batched =
            dual_faction_replay_trace(&setup, TICKS_PER_LOGIC_FRAME * 8, BATCHED_FRAMES / 8);
        assert_eq!(
            per_frame.len() as u64,
            BATCHED_FRAMES,
            "reason=per_frame_count"
        );
        assert_eq!(batched.len() as u64, BATCHED_FRAMES, "reason=batched_count");
        assert_eq!(
            per_frame, batched,
            "reason=per_frame_vs_batched_frame_hashes_match local={local:?} ai={ai:?}"
        );

        // 60 Hz / 144 Hz 呈现节奏喂入同一整数节拍序列，逐帧帧号与哈希一致。
        let at_60 = dual_faction_replay_trace(&setup, TICKS_PER_SECOND / 60, 60 * seconds);
        let at_144 = dual_faction_replay_trace(&setup, TICKS_PER_SECOND / 144, 144 * seconds);
        assert_eq!(
            at_60.len() as u64,
            expected_frames,
            "reason=60hz_frame_count"
        );
        assert_eq!(
            at_144.len() as u64,
            expected_frames,
            "reason=144hz_frame_count"
        );
        assert_eq!(
            at_60, at_144,
            "reason=60hz_144hz_frame_hash_identical local={local:?} ai={ai:?}"
        );
        for (index, (tick, _)) in at_60.iter().enumerate() {
            assert_eq!(
                *tick,
                index as u64 + 1,
                "reason=frame_number_monotonic local={local:?} ai={ai:?}"
            );
        }
        assert!(
            at_60.windows(2).any(|pair| pair[0].1 != pair[1].1),
            "reason=simulation_actually_advanced local={local:?} ai={ai:?}"
        );
    }
}

/// R1 / R13.6 分层判据沿用（tasks 3.5 / design D6）：四组合装配下 frame-0
/// 混合选中集合（含双方主城 / 农民 / 金矿 / 树木）经装配期
/// `UnitDef.speed_raw` 只读映射过滤后，MUST 只为农民生成 `Move`；建筑与
/// 资源点 MUST NOT 生成 `Move`、位置不变。本 change 不碰输入翻译层、不
/// 回退白名单、不扩权威视图；双族参数只改装配层段位，不改可移动性来源。
#[test]
fn four_race_combinations_keep_immovable_entities_out_of_move() {
    let defs = synthetic_definitions();
    // 混合选中集合：双方主城（1,7）、农民（2,3,4,8,9,10）、中立资源
    // （5,6,11,12）——12 个 frame-0 实体全部纳入。
    let selection: BTreeSet<u64> = (1..=12).collect();
    let movable_expected: BTreeSet<u64> = [2u64, 3, 4, 8, 9, 10].into_iter().collect();
    for (local, ai) in DUAL_COMBINATIONS {
        let setup = MeleeSetup::with_races(local, ai);
        let map: MovabilityMap = setup.movability(&defs);
        assert_eq!(
            map.len(),
            12,
            "reason=one_entry_per_frame0_entity local={local:?}"
        );
        let filtered: BTreeSet<u64> = filter_movable(&selection, &map).into_iter().collect();
        assert_eq!(
            filtered, movable_expected,
            "reason=only_peasants_movable local={local:?} ai={ai:?}"
        );
        // 建筑 / 资源点判据来自冻结定义 speed_raw，不来自运行时视图 speed。
        for id in [1u64, 5, 6, 7, 11, 12] {
            assert_eq!(
                map.get(&id),
                Some(&false),
                "reason=immovable_from_speed_raw id={id} local={local:?} ai={ai:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ① 反向验证（CODEBUDDY.md §5 / C5 / design D5 / tasks 6.1）：
// 「AI 命令族退回本地族」退化 → 异族组合的「AI 命令落 AI 族段」断言失败。
// 退化实现真经 AiScript::new 查表路径注入（替换装配层 ai_race / AI 份
// 绑定），再喂给真实 AiScript::new，不是旁路模拟命令。
// ---------------------------------------------------------------------------

/// 退化装配（缺陷态）：把 AI 命令源退回本地族——令 `ai_race` 与 AI 份
/// 绑定都退回本地族 / 本地份，喂给真实 `AiScript::new`。模拟 P8.3 之前的
/// 错配实现（`AiScript::new` 误用本地族 `setup.race` 与本地份绑定）。
fn degenerate_ai_uses_local_race(setup: &MeleeSetup) -> MeleeSetup {
    let mut degenerate = setup.clone();
    degenerate.ai_race = degenerate.race;
    degenerate.ai_category_def_ids = degenerate.category_def_ids.clone();
    degenerate
}

/// 反向验证·修复态（tasks 6.1）：正式实现四组合全部通过「AI 命令落
/// AI 族段」断言（与 ① 退化态共用的同一个布尔判据）。
#[test]
fn reverse_validation_ai_race_formal_passes_the_assertion() {
    for (local, ai) in DUAL_COMBINATIONS {
        let setup = MeleeSetup::with_races(local, ai);
        assert!(
            ai_commands_land_in_ai_race(&setup, ai),
            "reason=formal_ai_race_passes local={local:?} ai={ai:?}"
        );
    }
}

/// 反向验证·缺陷态（tasks 6.1）：AI 命令族退回本地族后，四组合中 AI 族
/// ≠ 本地族的组（人vs兽 / 兽vs人）必在同一断言下失败；正式实现仍通过
/// （元断言防退化）。同族组合退化前后字节一致，不纳入失败断言。
#[test]
fn reverse_validation_ai_race_local_fallback_degenerate_fails_the_same_assertion() {
    for (local, ai) in [(Race::Human, Race::Orc), (Race::Orc, Race::Human)] {
        let formal = MeleeSetup::with_races(local, ai);
        let degenerate = degenerate_ai_uses_local_race(&formal);
        assert!(
            !ai_commands_land_in_ai_race(&degenerate, ai),
            "reason=ai_race_local_fallback_degenerate_fails local={local:?} ai={ai:?}"
        );
        assert!(
            ai_commands_land_in_ai_race(&formal, ai),
            "reason=formal_still_passes local={local:?} ai={ai:?}"
        );
    }
    // 退化实现 MUST 真经 AiScript::new：退化装配仍产出合法 AI 脚本，
    // 只是目标 ID 落在本地族段（而非空脚本 / 旁路模拟）。
    for (local, ai) in [(Race::Human, Race::Orc), (Race::Orc, Race::Human)] {
        let formal = MeleeSetup::with_races(local, ai);
        let degenerate = degenerate_ai_uses_local_race(&formal);
        assert_eq!(
            AiScript::new(&degenerate, 1200).commands.len(),
            AiScript::new(&formal, 1200).commands.len(),
            "reason=degenerate_uses_real_ai_script_path local={local:?} ai={ai:?}"
        );
    }
}
