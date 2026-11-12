//! 一局 1v1 Melee 的端到端 CPU 验收（change `melee-playable-loop` 组 2）。
//!
//! 全部经 `w3-core` 既有公开 API 装配，`w3-core` 零改动；测试断言采集、
//! 建造、训练、战斗、胜负结算与重开闭环可达且可重复。

use std::collections::BTreeSet;

use w3_client::melee::defs::{BARRACKS, SOLDIER};
use w3_client::melee::{
    AI_PLAYER, LOCAL_PLAYER, MeleeDriver, MeleeSetup, synthetic_definitions, synthetic_world,
};
use w3_client::{ClientCommand, FixedStepClient, LOGIC_FPS, StepDriver, TICKS_PER_LOGIC_FRAME};
use w3_core::{
    Command, CommandKind, EntityId, Frame, GameplayCommand, GameplayCommandKind, MatchConfig,
    RtsMatch,
};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;

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
        .apply(InputAction::SelectStart([0.0, 0.0]), &view, &mapper, &[])
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([1920.0, 1080.0]),
            &view,
            &mapper,
            &[],
        )
        .unwrap();
    assert!(!input.selection().is_empty());

    let gather = input
        .apply(InputAction::Gather, &view, &mapper, &[])
        .unwrap();
    assert!(gather.iter().any(|command| matches!(
        command,
        ClientCommand::Gameplay(gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::Gather { .. })
    )));

    let build = input
        .apply(InputAction::Build, &view, &mapper, &[])
        .unwrap();
    assert!(matches!(
        build[0],
        ClientCommand::Gameplay(ref gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::Train { is_building: true, .. })
    ));

    let cancel = input
        .apply(InputAction::CancelTrain, &view, &mapper, &[])
        .unwrap();
    assert!(matches!(
        cancel[0],
        ClientCommand::Gameplay(ref gameplay)
            if matches!(gameplay.kind, GameplayCommandKind::CancelTrain { .. })
    ));

    let stop = input.apply(InputAction::Stop, &view, &mapper, &[]).unwrap();
    assert!(stop.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(simulation)
            if matches!(simulation.kind, CommandKind::Stop { .. })
    )));

    // 编组与按组选择：纯表现层动作，不生成命令、不改哈希。
    assert!(
        input
            .apply(InputAction::AssignGroup(3), &view, &mapper, &[])
            .unwrap()
            .is_empty()
    );
    assert!(
        input
            .apply(InputAction::SelectGroup(3), &view, &mapper, &[])
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
