//! 确定性验收：双实例、快照往返、回放与 P2 玩法竖切必须逐帧一致。
//! 这些测试是 ADR 0002 / 0004 与三个 spec delta 的门禁。

use w3_core::{
    Command, CommandKind, EntityId, EventKind, Frame, GroupPlanRecord, MatchConfig, RtsMatch,
    SimFixed, SnapshotError, StaticWorld,
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
