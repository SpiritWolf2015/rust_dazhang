//! 确定性验收：双实例、快照往返、回放与 P2 玩法竖切必须逐帧一致。
//! 这些测试是 ADR 0002 / 0004 与三个 spec delta 的门禁。

use w3_core::{
    AbilityDef, Command, CommandKind, EffectDef, EffectKind, EntityId, EventKind, Frame,
    FrozenDefinitions, GameplayCommand, GameplayCommandKind, GroupPlanRecord, ItemDef, MatchConfig,
    RtsMatch, SimFixed, SnapshotError, StaticWorld, UnitDef, UpgradeDef,
};

fn config() -> MatchConfig {
    MatchConfig {
        seed: 0x5eed,
        logic_fps: 30,
        content_hash: 0xC0FFEE,
    }
}

fn spawn(x: i64, y: i64) -> CommandKind {
    CommandKind::Spawn {
        x: SimFixed::from_int(x).unwrap(),
        y: SimFixed::from_int(y).unwrap(),
        hp: 100,
        speed: SimFixed::from_int(3).unwrap(),
        attack_damage: 100,
        attack_range: SimFixed::from_int(100).unwrap(),
    }
}

fn move_cmd(entity: EntityId, x: i64, y: i64, frame: u64, player: u8, seq: u32) -> Command {
    Command::new(
        Frame::new(frame),
        player,
        seq,
        CommandKind::Move {
            entity,
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(y).unwrap(),
            group: 0,
            append: false,
        },
    )
}

fn group_cmd(
    entity: EntityId,
    x: i64,
    y: i64,
    frame: u64,
    player: u8,
    seq: u32,
    group: u32,
) -> Command {
    Command::new(
        Frame::new(frame),
        player,
        seq,
        CommandKind::Move {
            entity,
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(y).unwrap(),
            group,
            append: false,
        },
    )
}

/// 固定的命令脚本。第 0 帧的两个 Spawn 按玩家编号（先 1 后 2）排序，
/// 因此内核"ID 绝不复用"的计数器会分配 1 与 2。刻意依赖这一点，
/// 因为它本身就是内核契约的一部分。
fn script() -> Vec<Command> {
    let first = EntityId::new(1).unwrap();
    let second = EntityId::new(2).unwrap();
    vec![
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        Command::new(Frame::new(0), 2, 1, spawn(0, 0)),
        move_cmd(first, 5, 3, 1, 1, 2),
        move_cmd(second, -4, 2, 1, 2, 2),
        Command::new(Frame::new(6), 1, 3, CommandKind::Stop { entity: first }),
        move_cmd(second, 9, 9, 12, 2, 3),
    ]
}

fn seeded(script: &[Command]) -> RtsMatch {
    let mut sim = RtsMatch::create(config());
    for command in script {
        sim.submit_command(*command).unwrap();
    }
    sim
}

#[test]
fn two_instances_interleaved_match_frame_hashes() {
    let script = script();
    let mut a = seeded(&script);
    let mut b = seeded(&script);
    let mut hashes = Vec::new();

    for frame in 0..40 {
        // 交错推进两个实例，确保二者无法共享隐藏状态。
        if frame % 2 == 0 {
            a.step();
            b.step();
        } else {
            b.step();
            a.step();
        }
        let hash_a = a.compute_state_hash();
        let hash_b = b.compute_state_hash();
        assert_eq!(
            hash_a,
            hash_b,
            "frame {frame} diverged at {:?}",
            a.first_difference(&b)
        );
        assert_eq!(a.first_difference(&b), None);
        hashes.push(hash_a);
    }

    // 防止「状态从未变化」造成的空泛通过。
    assert!(
        hashes.windows(2).any(|pair| pair[0] != pair[1]),
        "the simulation did not actually advance"
    );
}

#[test]
fn snapshot_roundtrip_continues_identically() {
    let script = script();
    let mut original = seeded(&script);
    for _ in 0..10 {
        original.step();
    }

    let snapshot = original.capture_snapshot();
    let mut restored = RtsMatch::restore_snapshot(
        snapshot.clone(),
        StaticWorld::default(),
        config().content_hash,
    )
    .unwrap();
    assert_eq!(original.compute_state_hash(), restored.compute_state_hash());
    assert_eq!(original.read_view(), restored.read_view());

    let mut reference = seeded(&script);
    for _ in 0..20 {
        reference.step();
    }
    for _ in 0..10 {
        original.step();
        restored.step();
    }
    assert_eq!(
        reference.compute_state_hash(),
        original.compute_state_hash()
    );
    assert_eq!(
        reference.compute_state_hash(),
        restored.compute_state_hash()
    );
}

#[test]
fn replay_of_recorded_commands_matches_original() {
    let recorded = script();
    let mut original = seeded(&recorded);
    let expected: Vec<u64> = (0..30)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();

    let mut replay = seeded(&recorded);
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.compute_state_hash(),
            *hash,
            "replay diverged at frame {frame}"
        );
    }
}

#[test]
fn first_difference_names_the_changed_field() {
    let mut a = RtsMatch::create(config());
    let b = RtsMatch::create(config());
    a.step();
    assert_eq!(a.first_difference(&b), Some("tick"));
}

#[test]
fn first_difference_reports_config_fields() {
    let a = RtsMatch::create(config());
    let mut other = config();
    other.content_hash = 0xDEAD;
    let b = RtsMatch::create(other);
    assert_eq!(a.first_difference(&b), Some("content_hash"));
}

#[test]
fn snapshot_rejects_mismatched_content() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let snapshot = sim.capture_snapshot();
    assert!(matches!(
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), 0xDEAD_BEEF),
        Err(SnapshotError::ContentMismatch)
    ));
}

#[test]
fn snapshot_rejects_mismatched_world() {
    let world = StaticWorld::open_world(16, 16, SimFixed::ONE);
    let mut sim = RtsMatch::create_with_world(config(), world);
    sim.step();
    let snapshot = sim.capture_snapshot();
    // 恢复时提供身份不同的世界必须被拒绝。
    assert!(matches!(
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), config().content_hash),
        Err(SnapshotError::WorldMismatch)
    ));
}

#[test]
fn snapshot_rejects_malformed_group_plan() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    // 成员与槽位数量不一致，恢复时必须拒绝而不是在规划时越界。
    snapshot.group_plans.push(GroupPlanRecord {
        group: 1,
        goal: (0, 0),
        members: vec![1],
        slots: Vec::new(),
        progress: 0,
    });
    assert!(matches!(
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), config().content_hash),
        Err(SnapshotError::MalformedGroupPlan)
    ));
}

#[test]
fn snapshot_rejects_old_version() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let mut snapshot = sim.capture_snapshot();
    snapshot.version = 1;
    assert!(matches!(
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), config().content_hash),
        Err(SnapshotError::UnsupportedVersion { found: 1 })
    ));
}

#[test]
fn two_instances_movement_and_events_match() {
    // 短距离专用脚本：确保在观测窗口内产生 Arrived 事件。
    let commands = vec![
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        move_cmd(EntityId::new(1).unwrap(), 2, 0, 1, 1, 2),
    ];
    let mut a = seeded(&commands);
    let mut b = seeded(&commands);
    let mut arrivals = 0;
    for frame in 0..80 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "frame {frame} diverged at {:?}",
            a.first_difference(&b)
        );
        let events_a = a.drain_events();
        let events_b = b.drain_events();
        assert_eq!(events_a, events_b, "frame {frame} event sequence diverged");
        arrivals += events_a
            .iter()
            .filter(|event| event.kind == EventKind::Arrived)
            .count();
    }
    assert!(arrivals > 0, "the movement scenario must produce arrivals");
}

#[test]
fn group_move_is_stable_across_submission_orders() {
    fn group_script(reverse: bool) -> Vec<Command> {
        let mut commands = vec![
            Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
            Command::new(Frame::new(0), 1, 2, spawn(1, 0)),
            Command::new(Frame::new(0), 1, 3, spawn(2, 0)),
        ];
        let moves = vec![
            group_cmd(EntityId::new(1).unwrap(), 6, 6, 1, 1, 1, 7),
            group_cmd(EntityId::new(2).unwrap(), 6, 6, 1, 1, 2, 7),
            group_cmd(EntityId::new(3).unwrap(), 6, 6, 1, 1, 3, 7),
        ];
        if reverse {
            for command in moves.into_iter().rev() {
                commands.push(command);
            }
        } else {
            commands.extend(moves);
        }
        commands
    }

    // 两种提交顺序只改变到达序号；排序键由 (frame, player, sequence) 决定。
    // 帧 0 尚未执行的命令携带不同 arrival，属于输入差异，故只比较轨迹；
    // 帧 1 起待执行命令已清空，权威哈希必须一致。
    let mut a = seeded(&group_script(false));
    let mut b = seeded(&group_script(true));
    for frame in 0..150 {
        a.step();
        b.step();
        let track = |sim: &RtsMatch| {
            sim.read_view()
                .entities
                .iter()
                .map(|entity| (entity.id, entity.x, entity.y, entity.facing))
                .collect::<Vec<_>>()
        };
        assert_eq!(track(&a), track(&b), "frame {frame} trajectory diverged");
        if frame >= 1 {
            assert_eq!(
                a.compute_state_hash(),
                b.compute_state_hash(),
                "frame {frame} diverged at {:?}",
                a.first_difference(&b)
            );
        }
    }
    assert_eq!(
        a.read_view(),
        b.read_view(),
        "formation slots must be stable"
    );
}

#[test]
fn dynamic_obstacle_triggers_deterministic_replan() {
    let unit = EntityId::new(1).unwrap();
    let commands = vec![
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        move_cmd(unit, 7, 0, 1, 1, 2),
        Command::new(
            Frame::new(10),
            1,
            3,
            CommandKind::SetObstacle {
                id: 1,
                x: 3,
                y: 0,
                w: 1,
                h: 4,
            },
        ),
    ];
    let mut a = seeded(&commands);
    let mut b = seeded(&commands);
    for frame in 0..80 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "frame {frame} diverged at {:?}",
            a.first_difference(&b)
        );
        assert_eq!(a.drain_events(), b.drain_events());
        // 单位不得出现在动态障碍占据的格子里。
        let view = a.read_view();
        let position = &view.entities[0];
        let cell = StaticWorld::default().world_to_cell(
            SimFixed::from_raw(position.x),
            SimFixed::from_raw(position.y),
        );
        assert!(
            !(cell.x == 3 && (0..4).contains(&cell.y)),
            "unit crossed the dynamic obstacle at frame {frame}"
        );
    }
}

#[test]
fn same_frame_double_kill_is_deterministic() {
    let target = EntityId::new(3).unwrap();
    let commands = vec![
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        Command::new(Frame::new(0), 2, 1, spawn(0, 0)),
        Command::new(Frame::new(0), 3, 1, spawn(0, 0)),
        Command::new(
            Frame::new(1),
            1,
            2,
            CommandKind::Attack {
                attacker: EntityId::new(1).unwrap(),
                target,
            },
        ),
        Command::new(
            Frame::new(1),
            2,
            2,
            CommandKind::Attack {
                attacker: EntityId::new(2).unwrap(),
                target,
            },
        ),
    ];
    let mut a = seeded(&commands);
    let mut b = seeded(&commands);
    let mut deaths = 0;
    for _ in 0..4 {
        a.step();
        b.step();
        assert_eq!(a.compute_state_hash(), b.compute_state_hash());
        let events = a.drain_events();
        assert_eq!(events, b.drain_events());
        deaths += events
            .iter()
            .filter(|event| event.kind == EventKind::Died)
            .count();
    }
    assert_eq!(deaths, 1, "target must die exactly once");
}

#[test]
fn new_authoritative_fields_are_hashed() {
    // 动态障碍字段进入哈希与差异诊断。
    let mut a = RtsMatch::create(config());
    let mut b = RtsMatch::create(config());
    a.submit_command(Command::new(
        Frame::new(0),
        1,
        1,
        CommandKind::SetObstacle {
            id: 1,
            x: 2,
            y: 2,
            w: 2,
            h: 2,
        },
    ))
    .unwrap();
    // 等量 Noop 对齐队列计数器，隔离出障碍字段本身的差异。
    b.submit_command(Command::new(Frame::new(0), 1, 1, CommandKind::Noop))
        .unwrap();
    a.step();
    b.step();
    assert_ne!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), Some("obstacles"));

    // 单位订单字段进入哈希与差异诊断；用等量 Noop 对齐队列计数器。
    let mut c = RtsMatch::create(config());
    let mut d = RtsMatch::create(config());
    let spawn_command = Command::new(Frame::new(0), 1, 1, spawn(0, 0));
    c.submit_command(spawn_command).unwrap();
    d.submit_command(spawn_command).unwrap();
    // Move 改变订单状态但不产生事件，便于隔离出 units 字段差异。
    c.submit_command(move_cmd(EntityId::new(1).unwrap(), 3, 0, 1, 1, 2))
        .unwrap();
    d.submit_command(Command::new(Frame::new(1), 1, 2, CommandKind::Noop))
        .unwrap();
    c.step();
    d.step();
    c.step();
    d.step();
    assert_ne!(c.compute_state_hash(), d.compute_state_hash());
    assert_eq!(c.first_difference(&d), Some("units"));
}

#[test]
fn snapshot_roundtrip_during_group_planning() {
    // 70 名成员 > 单帧规划预算，捕获进行中的分帧规划作业。
    let mut commands = Vec::new();
    for seq in 1..=70u32 {
        commands.push(Command::new(Frame::new(0), 1, seq, spawn(0, 0)));
    }
    for seq in 1..=70u32 {
        commands.push(group_cmd(
            EntityId::new(seq as u64).unwrap(),
            6,
            6,
            1,
            1,
            seq,
            7,
        ));
    }
    let mut original = seeded(&commands);
    original.step(); // 帧 0：出生
    original.step(); // 帧 1：群体规划推进但未提交
    let snapshot = original.capture_snapshot();
    assert_eq!(snapshot.group_plans.len(), 1);

    let mut restored = RtsMatch::restore_snapshot(
        snapshot.clone(),
        StaticWorld::default(),
        config().content_hash,
    )
    .unwrap();
    assert_eq!(original.compute_state_hash(), restored.compute_state_hash());
    for _ in 0..8 {
        original.step();
        restored.step();
        assert_eq!(original.compute_state_hash(), restored.compute_state_hash());
    }
}

// ---------------------------------------------------------------------------
// P5 玩法系统确定性场景（施法 / Buff 到期 / 同帧死亡 / 生产取消 / AI 重放）
// ---------------------------------------------------------------------------

fn unit_def() -> UnitDef {
    UnitDef {
        hp: 100,
        mana: 100,
        speed_raw: SimFixed::from_int(3).unwrap().raw(),
        damage: 10,
        range_raw: SimFixed::from_int(10).unwrap().raw(),
        supply: 1,
        train_frames: 4,
    }
}

fn gameplay_defs() -> FrozenDefinitions {
    FrozenDefinitions::new(
        vec![(1, unit_def())],
        vec![
            (
                10,
                AbilityDef {
                    mana_cost: 10,
                    cooldown_frames: 5,
                    cast_frames: 3,
                    behavior: 0,
                    effects: vec![
                        EffectDef {
                            kind: EffectKind::Damage,
                            amount: 30,
                            duration_frames: 0,
                        },
                        EffectDef {
                            kind: EffectKind::BuffStat,
                            amount: 5,
                            duration_frames: 3,
                        },
                    ],
                },
            ),
            (
                11,
                AbilityDef {
                    mana_cost: 5,
                    cooldown_frames: 0,
                    cast_frames: 0,
                    behavior: 99,
                    effects: vec![EffectDef {
                        kind: EffectKind::Heal,
                        amount: 20,
                        duration_frames: 0,
                    }],
                },
            ),
            (
                12,
                AbilityDef {
                    mana_cost: 0,
                    cooldown_frames: 0,
                    cast_frames: 0,
                    behavior: 0,
                    effects: vec![
                        EffectDef {
                            kind: EffectKind::Damage,
                            amount: 200,
                            duration_frames: 0,
                        },
                        EffectDef {
                            kind: EffectKind::BuffStat,
                            amount: 99,
                            duration_frames: 5,
                        },
                    ],
                },
            ),
            (
                13,
                AbilityDef {
                    mana_cost: 0,
                    cooldown_frames: 0,
                    cast_frames: 0,
                    behavior: 0,
                    effects: vec![EffectDef {
                        kind: EffectKind::Stun,
                        amount: 1,
                        duration_frames: 10,
                    }],
                },
            ),
        ],
        vec![(
            20,
            ItemDef {
                hp_mod: 10,
                damage_mod: 5,
            },
        )],
        vec![(
            30,
            UpgradeDef {
                damage_mod: 3,
                hp_mod: 5,
                research_frames: 4,
            },
        )],
    )
    .unwrap()
}

fn gameplay_config(defs: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: 0x5eed,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    }
}

fn gp(frame: u64, player: u8, sequence: u32, kind: GameplayCommandKind) -> GameplayCommand {
    GameplayCommand::new(Frame::new(frame), player, sequence, kind)
}

#[test]
fn casting_buff_expiry_and_restore_are_deterministic() {
    let defs = gameplay_defs();
    let content = defs.content_hash();
    let cfg = gameplay_config(&defs);
    let spawn_a = Command::new(Frame::new(0), 1, 1, spawn(0, 0));
    let spawn_b = Command::new(Frame::new(0), 2, 1, spawn(0, 0));
    let cast = gp(
        1,
        1,
        1,
        GameplayCommandKind::Cast {
            caster: EntityId::new(1).unwrap(),
            ability: 10,
            target: EntityId::new(2).unwrap(),
        },
    );

    let build = || {
        let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
        sim.submit_command(spawn_a).unwrap();
        sim.submit_command(spawn_b).unwrap();
        sim.submit_gameplay_command(cast).unwrap();
        sim
    };
    let mut a = build();
    let mut b = build();

    // 推进到引导进行中（帧 2），捕获含施法运行态的快照。
    for _ in 0..2 {
        a.step();
        b.step();
        assert_eq!(a.compute_state_hash(), b.compute_state_hash());
    }
    let snap = a.capture_snapshot();
    let caster = snap.positions.iter().find(|p| p.entity == 1).unwrap();
    assert!(
        caster.gameplay.cast.is_some(),
        "cast runtime must enter snapshot"
    );
    let mana_at_capture = caster.gameplay.mana;

    let mut restored = RtsMatch::restore_snapshot(snap, StaticWorld::default(), content).unwrap();
    assert_eq!(a.compute_state_hash(), restored.compute_state_hash());
    let restored_snap = restored.capture_snapshot();
    let restored_caster = restored_snap
        .positions
        .iter()
        .find(|p| p.entity == 1)
        .unwrap();
    assert_eq!(
        restored_caster.gameplay.mana, mana_at_capture,
        "restore must not re-charge mana or re-cast"
    );

    // 施法完成、冷却递减、Buff 到期后三者仍逐帧一致。
    for frame in 2..24 {
        a.step();
        b.step();
        restored.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "frame {frame}"
        );
        assert_eq!(
            a.compute_state_hash(),
            restored.compute_state_hash(),
            "restore frame {frame}"
        );
        assert_eq!(a.first_difference(&b), None);
        assert_eq!(a.first_difference(&restored), None);
    }
}

#[test]
fn effects_after_target_death_return_no_effect() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let cast = gp(
        1,
        1,
        1,
        GameplayCommandKind::Cast {
            caster: EntityId::new(1).unwrap(),
            ability: 12,
            target: EntityId::new(2).unwrap(),
        },
    );
    let build = || {
        let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0)))
            .unwrap();
        sim.submit_command(Command::new(Frame::new(0), 2, 1, spawn(0, 0)))
            .unwrap();
        sim.submit_gameplay_command(cast).unwrap();
        sim
    };
    let mut a = build();
    let mut b = build();
    for _ in 0..3 {
        a.step();
        b.step();
    }
    assert_eq!(a.compute_state_hash(), b.compute_state_hash());
    let snap = a.capture_snapshot();
    let target = snap.positions.iter().find(|p| p.entity == 2).unwrap();
    assert!(!target.alive, "lethal damage must kill the target");
    assert!(
        target.gameplay.buffs.is_empty(),
        "subsequent effects must be NoEffect after death"
    );
}

#[test]
fn production_cancel_is_deterministic() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let producer = EntityId::new(1).unwrap();
    let train = gp(
        1,
        1,
        1,
        GameplayCommandKind::Train {
            producer,
            unit_def: 1,
            is_building: false,
        },
    );
    let cancel = gp(
        2,
        1,
        2,
        GameplayCommandKind::CancelTrain { producer, index: 0 },
    );
    let spawn_cmd = Command::new(Frame::new(0), 1, 1, spawn(0, 0));

    let build = |with_cancel: bool| {
        let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
        sim.submit_command(spawn_cmd).unwrap();
        sim.submit_gameplay_command(train).unwrap();
        if with_cancel {
            sim.submit_gameplay_command(cancel).unwrap();
        }
        sim
    };
    let mut a = build(true);
    let mut b = build(true);
    let mut uncancelled = build(false);

    for _ in 0..14 {
        a.step();
        b.step();
        uncancelled.step();
    }
    assert_eq!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), None);
    let snap_a = a.capture_snapshot();
    let snap_c = uncancelled.capture_snapshot();
    assert!(
        snap_a
            .positions
            .iter()
            .find(|p| p.entity == 1)
            .unwrap()
            .gameplay
            .production
            .is_empty()
    );
    assert_eq!(
        snap_a.alive_entities.len(),
        1,
        "cancelled production must not spawn"
    );
    assert_eq!(
        snap_c.alive_entities.len(),
        2,
        "completed production must spawn a unit"
    );
}

#[test]
fn ai_replay_does_not_double_execute() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let set_ai = gp(1, 1, 1, GameplayCommandKind::SetAi { enabled: true });
    let external = [
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        Command::new(Frame::new(0), 2, 1, spawn(1, 0)),
    ];

    let build = |with_ai: bool| {
        let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
        for command in &external {
            sim.submit_command(*command).unwrap();
        }
        if with_ai {
            sim.submit_gameplay_command(set_ai).unwrap();
        }
        sim
    };
    let mut original = build(true);
    let mut replay = build(true);
    let mut no_ai = build(false);

    // 重放只注入相同的外部命令，AI 每帧确定性重算。
    let expected: Vec<u64> = (0..30)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(replay.compute_state_hash(), *hash, "frame {frame}");
    }
    for _ in 0..30 {
        no_ai.step();
    }
    assert_ne!(
        original.compute_state_hash(),
        no_ai.compute_state_hash(),
        "AI must affect authoritative state"
    );
    let snap = replay.capture_snapshot();
    assert!(
        snap.gameplay.commands.pending().is_empty(),
        "internal AI commands must never enter the command queue"
    );
}

#[test]
fn new_gameplay_fields_are_hashed_and_snapshot_safe() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let spawn_cmd = Command::new(Frame::new(0), 1, 1, spawn(0, 0));
    let producer = EntityId::new(1).unwrap();
    let research = gp(0, 1, 1, GameplayCommandKind::Research { upgrade: 30 });

    let mut a = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
    let mut b = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
    a.submit_command(spawn_cmd).unwrap();
    b.submit_command(spawn_cmd).unwrap();
    a.submit_gameplay_command(research).unwrap();
    // 等量不改变状态的命令对齐玩法命令计数器，隔离出世界玩法状态差异。
    let noop_kind = GameplayCommandKind::CancelTrain {
        producer,
        index: u32::MAX,
    };
    a.submit_gameplay_command(gp(0, 1, 2, noop_kind)).unwrap();
    b.submit_gameplay_command(gp(0, 1, 1, noop_kind)).unwrap();
    b.submit_gameplay_command(gp(0, 1, 2, noop_kind)).unwrap();

    a.step();
    b.step();
    assert_ne!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), Some("gameplay_players"));

    let snap = a.capture_snapshot();
    let restored =
        RtsMatch::restore_snapshot(snap, StaticWorld::default(), defs.content_hash()).unwrap();
    assert_eq!(a.compute_state_hash(), restored.compute_state_hash());
    assert_eq!(a.first_difference(&restored), None);
}

#[test]
fn frozen_definitions_are_hashed_and_content_validated() {
    let full = gameplay_defs();
    let empty = FrozenDefinitions::empty();
    // 两份配置 content_hash 相同，只让冻结定义本身不同。
    let cfg = MatchConfig {
        seed: 1,
        logic_fps: 30,
        content_hash: 0xABCD,
    };
    let a = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), full.clone());
    let b = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), empty);
    assert_ne!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), Some("frozen_definitions"));
    // 只读视图：没有运行期热更新入口。
    assert_eq!(a.definitions().content_hash(), full.content_hash());

    let mut sim = RtsMatch::create_with_definitions(
        MatchConfig {
            seed: 1,
            logic_fps: 30,
            content_hash: full.content_hash(),
        },
        StaticWorld::default(),
        full.clone(),
    );
    sim.step();
    let snap = sim.capture_snapshot();
    assert_eq!(snap.content_hash, full.content_hash());
    assert_eq!(snap.defs.content_hash(), full.content_hash());
    assert!(matches!(
        RtsMatch::restore_snapshot(
            snap,
            StaticWorld::default(),
            full.content_hash().wrapping_add(1)
        ),
        Err(SnapshotError::ContentMismatch)
    ));
}

#[test]
fn registered_behavior_extends_the_effect_chain() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let target = EntityId::new(2).unwrap();
    let register = gp(
        0,
        1,
        1,
        GameplayCommandKind::RegisterBehavior {
            id: 99,
            effect: EffectDef {
                kind: EffectKind::BuffStat,
                amount: 1,
                duration_frames: 4,
            },
        },
    );
    let cast = gp(
        1,
        1,
        2,
        GameplayCommandKind::Cast {
            caster: EntityId::new(1).unwrap(),
            ability: 11,
            target,
        },
    );
    let build = |with_register: bool| {
        let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
        sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0)))
            .unwrap();
        sim.submit_command(Command::new(Frame::new(0), 2, 1, spawn(0, 0)))
            .unwrap();
        if with_register {
            sim.submit_gameplay_command(register).unwrap();
        }
        sim.submit_gameplay_command(cast).unwrap();
        sim
    };
    let mut a = build(true);
    let mut b = build(false);
    for _ in 0..3 {
        a.step();
        b.step();
    }
    let snap_a = a.capture_snapshot();
    let snap_b = b.capture_snapshot();
    let ta = snap_a.positions.iter().find(|p| p.entity == 2).unwrap();
    let tb = snap_b.positions.iter().find(|p| p.entity == 2).unwrap();
    assert_eq!(
        ta.gameplay.buffs.len(),
        1,
        "registered behavior adds an effect"
    );
    assert!(
        tb.gameplay.buffs.is_empty(),
        "no registration means no extra effect"
    );
    assert_eq!(
        ta.gameplay.stat_mods.damage, 1,
        "BuffStat must actually modify authoritative stats"
    );
    assert_ne!(a.compute_state_hash(), b.compute_state_hash());
}

#[test]
fn snapshot_rejects_tampered_definitions() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs.clone());
    sim.step();
    let mut snap = sim.capture_snapshot();
    // 替换冻结定义但保留 defs_hash 头部，恢复必须明确拒绝。
    snap.defs = FrozenDefinitions::empty();
    assert!(matches!(
        RtsMatch::restore_snapshot(snap, StaticWorld::default(), defs.content_hash()),
        Err(SnapshotError::DefsMismatch)
    ));
}

#[test]
fn snapshot_rejects_pre_p5_versions() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let base = sim.capture_snapshot();

    let mut old_version = base.clone();
    old_version.version = 2;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_version, StaticWorld::default(), config().content_hash),
        Err(SnapshotError::UnsupportedVersion { found: 2 })
    ));

    let mut old_sim = base;
    old_sim.sim_version = 2;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), config().content_hash),
        Err(SnapshotError::SimVersionMismatch {
            found: 2,
            expected: 3
        })
    ));
}

#[test]
fn victory_winner_option_is_hashed_with_a_tag() {
    let mut sim = RtsMatch::create(config());
    sim.step();
    let base = sim.capture_snapshot();

    let mut none_winner = base.clone();
    none_winner.gameplay.victory.resolved = true;
    none_winner.gameplay.victory.winner = None;
    let mut zero_winner = base;
    zero_winner.gameplay.victory.resolved = true;
    zero_winner.gameplay.victory.winner = Some(0);

    let a = RtsMatch::restore_snapshot(none_winner, StaticWorld::default(), config().content_hash)
        .unwrap();
    let b = RtsMatch::restore_snapshot(zero_winner, StaticWorld::default(), config().content_hash)
        .unwrap();
    // `None` 与 `Some(0)` 必须编码不同，避免哈希相同而差异诊断报不一致。
    assert_ne!(a.compute_state_hash(), b.compute_state_hash());
    assert_eq!(a.first_difference(&b), Some("victory"));
}

#[test]
fn stun_buff_blocks_player_actions() {
    let defs = gameplay_defs();
    let cfg = gameplay_config(&defs);
    let stun = gp(
        1,
        1,
        1,
        GameplayCommandKind::Cast {
            caster: EntityId::new(1).unwrap(),
            ability: 13,
            target: EntityId::new(2).unwrap(),
        },
    );
    let move_cmd = Command::new(
        Frame::new(2),
        2,
        2,
        CommandKind::Move {
            entity: EntityId::new(2).unwrap(),
            x: SimFixed::from_int(5).unwrap(),
            y: SimFixed::ZERO,
            group: 0,
            append: false,
        },
    );
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), defs);
    sim.submit_command(Command::new(Frame::new(0), 1, 1, spawn(0, 0)))
        .unwrap();
    sim.submit_command(Command::new(Frame::new(0), 2, 1, spawn(0, 0)))
        .unwrap();
    sim.submit_gameplay_command(stun).unwrap();
    sim.submit_command(move_cmd).unwrap();
    for _ in 0..3 {
        sim.step();
    }
    let snap = sim.capture_snapshot();
    let target = snap.positions.iter().find(|p| p.entity == 2).unwrap();
    assert!(
        target
            .gameplay
            .buffs
            .iter()
            .any(|buff| buff.kind == EffectKind::Stun),
        "stun buff must be present"
    );
    assert!(
        target.goal.is_none(),
        "stunned unit must reject the player Move command"
    );
}
