//! `p10-ai-player-lockstep` 批次 1：联机 1v1（真人席位 × AI 席位）CPU 正向回归。
//!
//! 两个 `ClientSession` 作为两个只读模拟端点复用 `two_clients.rs` 的回环
//! relay / Hello→Welcome / 权威帧 / checksum 交换范式；AI 席位不另立
//! `PlayerId` 空间，也没有独立决策连接。第二个端点只上行空输入，用于保持
//! 既有两端点 checksum 路径；AI 席位 PlayerId 由冻结开局定义的静态
//! `Spawn` 命令登记，运行期不贡献任何 AI 决策命令帧。
//!
//! 命令夹具是冻结真人命令流：`SetAi{enabled:true}`、采集、建造、训练、
//! 进攻都经 `ClientSession::submit_input` 上行同一 `CommandBatch`，再由
//! `w3_net::apply_frame` 作为唯一推进点提交；测试不调用本地静默
//! `submit_gameplay_command`，不把内核 `run_ai` 的内部 `Gather` 注入帧流。
//!
//! 胜负判据沿用 P9 / 本地 `MeleeDriver` 先例：真人命令流先用 `Attack`
//! 摧毁 AI 席位全部建筑，随后才在既定点提交既有
//! `Leave{player:AI_PLAYER}` 结算命令面；实际结算路径是内核 `leave()`
//! 把 AI 席位 `active=false`，再由 `resolve_victory` 在同帧阶段产生唯一
//! 胜者。这里不冒充「建筑全毁自动结算」，也不采用 `AiScript` 的提前
//! 认输 / 直接判负。
//!
//! 本文件只新增测试：不改 W3N1 25B 帧头、protobuf payload、`FrameScheduler`
//! / `stabilize` / `apply_frame`、checksum、Hello / Welcome / Frame / Reject
//! / ResyncReplay 处理路径，也不改 `w3-core` 生产逻辑与四件套正文。

use std::collections::BTreeSet;

use w3_core::{
    Command, CommandKind, DefId, EntityId, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, RtsMatch, SNAPSHOT_VERSION, SimFixed, StaticWorld, UnitDef,
};
use w3_net::packet::{
    FrameCommands, HEADER_LEN, Packet, PacketKind, decode_commands, encode_commands,
    parse_resync_replay,
};
use w3_net::transport::{Endpoint, LoopbackBus, LoopbackTransport, SharedBus, Transport};
use w3_net::{ClientSession, MAGIC, PROTOCOL_VERSION, apply_frame, resync_replay_packet};
use w3_relay::Relay;

/// 真人席位编号，与本地 `LOCAL_PLAYER` 保持一致。
const LOCAL_PLAYER: u8 = 1;
/// AI 席位编号，与本地 `AI_PLAYER` 保持一致。
const AI_PLAYER: u8 = 2;
/// 合成 / 原创冻结定义编号，只用于验收夹具，不读取本机导出数据。
const BUILDING_DEF: DefId = 7_001;
/// 合成 / 原创士兵定义编号，射程覆盖本夹具两座 AI 建筑。
const SOLDIER_DEF: DefId = 7_002;
/// 建造命令所在整数帧。
const BUILD_FRAME: u64 = 5;
/// 建造队列完成帧，用来记录训练前实体集合。
const BUILD_COMPLETE_FRAME: u64 = 6;
/// 训练命令所在整数帧。
const TRAIN_FRAME: u64 = 7;
/// 训练队列完成帧，用来取得新士兵实体编号。
const UNIT_COMPLETE_FRAME: u64 = 8;
/// 真人攻击并摧毁 AI 全部建筑的整数帧。
const ATTACK_FRAME: u64 = 9;
/// 既有 `Leave` 结算命令面所在整数帧。
const LEAVE_FRAME: u64 = 10;
/// 首次内核 AI 决策帧，由 `SetAi` 的决策周期派生。
const AI_DECISION_FRAME: u64 = 10;
/// 结算后继续推进的末帧，用于证明只结算一次。
const FINAL_FRAME: u64 = 20;

/// 把整数字面量冻结为 Q47.16 定点值，显式处理转换失败。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("ai_lockstep: reason=fixed_conversion_failed")
}

/// 返回整数坐标 / 速度 / 射程对应的 Q47.16 原始值。
fn scaled(value: i64) -> i64 {
    fixed(value).raw()
}

/// 本测试专用的最小冻结定义集合，全部是编译期整数常量、零资产依赖。
fn locked_definitions() -> FrozenDefinitions {
    let units = vec![
        (
            BUILDING_DEF,
            UnitDef {
                hp: 60,
                mana: 0,
                speed_raw: 0,
                damage: 0,
                range_raw: 0,
                supply: 0,
                train_frames: 2,
            },
        ),
        (
            SOLDIER_DEF,
            UnitDef {
                hp: 100,
                mana: 0,
                speed_raw: scaled(3),
                damage: 80,
                range_raw: scaled(20),
                supply: 1,
                train_frames: 2,
            },
        ),
    ];
    FrozenDefinitions::new(units, Vec::new(), Vec::new(), Vec::new())
        .expect("ai_lockstep: reason=frozen_definitions_invalid")
}

/// 两个端点使用完全相同的冻结开局定义构造 `RtsMatch`。
fn new_match() -> RtsMatch {
    RtsMatch::create_with_definitions(
        MatchConfig {
            seed: 0xA1_10,
            logic_fps: 30,
            content_hash: 0xC0_FF_EE,
        },
        StaticWorld::open_world(16, 16, fixed(8)),
        locked_definitions(),
    )
}

/// 构造一条合法 `Spawn` 命令；建筑 / 资源点也用最小正速度满足既有校验。
fn spawn_command(
    frame: u64,
    player: u8,
    sequence: u32,
    x: i64,
    y: i64,
    hp: i64,
    speed: SimFixed,
    damage: i64,
    range: SimFixed,
) -> Command {
    Command::new(
        Frame::new(frame),
        player,
        sequence,
        CommandKind::Spawn {
            x: fixed(x),
            y: fixed(y),
            hp,
            speed,
            attack_damage: damage,
            attack_range: range,
        },
    )
}

/// 冻结开局定义：真人主城 / 农民、AI 两座建筑，以及帧 0 真人玩法命令。
struct Opening {
    commands: Vec<Command>,
    gameplay: Vec<GameplayCommand>,
    local_hall: EntityId,
    local_worker: EntityId,
    ai_buildings: Vec<EntityId>,
}

/// 按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定顺序登记实体编号。
fn opening() -> Opening {
    let local_hall = EntityId::new(1).expect("ai_lockstep: reason=invalid_local_hall_id");
    let local_worker = EntityId::new(2).expect("ai_lockstep: reason=invalid_local_worker_id");
    let ai_hall = EntityId::new(3).expect("ai_lockstep: reason=invalid_ai_hall_id");
    let ai_outpost = EntityId::new(4).expect("ai_lockstep: reason=invalid_ai_outpost_id");

    let commands = vec![
        spawn_command(0, LOCAL_PLAYER, 1, 2, 2, 120, fixed(1), 0, fixed(0)),
        spawn_command(0, LOCAL_PLAYER, 2, 3, 2, 60, fixed(3), 0, fixed(0)),
        // 帧 0 的 AI 席位 `Spawn` 是冻结开局定义，不是 AI 决策命令。
        spawn_command(0, AI_PLAYER, 1, 4, 2, 30, fixed(1), 0, fixed(0)),
        spawn_command(0, AI_PLAYER, 2, 4, 3, 30, fixed(1), 0, fixed(0)),
    ];
    let gameplay = vec![
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            10,
            GameplayCommandKind::SetAi { enabled: true },
        ),
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            11,
            GameplayCommandKind::Gather { unit: local_worker },
        ),
    ];

    Opening {
        commands,
        gameplay,
        local_hall,
        local_worker,
        ai_buildings: vec![ai_hall, ai_outpost],
    }
}

/// 冻结真人命令流：采集 → 建造 → 训练 → 进攻 → 既有结算命令面。
fn human_script(
    frame: u64,
    opening: &Opening,
    attacker: Option<EntityId>,
) -> (Vec<Command>, Vec<GameplayCommand>) {
    if frame == 0 {
        return (opening.commands.clone(), opening.gameplay.clone());
    }
    if (1..=4).contains(&frame) {
        let gather = GameplayCommand::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            100 + frame as u32,
            GameplayCommandKind::Gather {
                unit: opening.local_worker,
            },
        );
        return (Vec::new(), vec![gather]);
    }
    if frame == BUILD_FRAME {
        let build = GameplayCommand::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            200,
            GameplayCommandKind::Train {
                producer: opening.local_hall,
                unit_def: BUILDING_DEF,
                is_building: true,
            },
        );
        return (Vec::new(), vec![build]);
    }
    if frame == TRAIN_FRAME {
        let train = GameplayCommand::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            300,
            GameplayCommandKind::Train {
                producer: opening.local_hall,
                unit_def: SOLDIER_DEF,
                is_building: false,
            },
        );
        return (Vec::new(), vec![train]);
    }
    if frame == ATTACK_FRAME {
        let attacker = attacker.expect("ai_lockstep: reason=attacker_missing frame=9");
        let attacks = vec![
            Command::new(
                Frame::new(frame),
                LOCAL_PLAYER,
                400,
                CommandKind::Attack {
                    attacker,
                    target: opening.ai_buildings[0],
                },
            ),
            Command::new(
                Frame::new(frame),
                LOCAL_PLAYER,
                401,
                CommandKind::Attack {
                    attacker,
                    target: opening.ai_buildings[1],
                },
            ),
        ];
        return (attacks, Vec::new());
    }
    if frame == LEAVE_FRAME {
        // 既有结算命令面：真人命令流已经在上一帧摧毁全部 AI 建筑后才提交。
        // 胜负实际由 `leave()` 把 AI 席位 `active=false`，再由
        // `resolve_victory` 在同帧阶段结算；不是「建筑全毁自动结算」，也不是
        // `AiScript` 的提前认输 / 直接判负。
        let leave = GameplayCommand::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            500,
            GameplayCommandKind::Leave { player: AI_PLAYER },
        );
        return (Vec::new(), vec![leave]);
    }
    (Vec::new(), Vec::new())
}

/// 双端点 + relay + 两份 `RtsMatch` 的测试夹具。
struct Harness {
    bus: SharedBus,
    relay: Relay<LoopbackTransport>,
    client1: ClientSession<LoopbackTransport>,
    client2: ClientSession<LoopbackTransport>,
    sim1: RtsMatch,
    sim2: RtsMatch,
    opening: Opening,
}

/// 建立 lobby 并核对 W3N1 / 版本 / 稳定 PlayerId 基线。
fn setup() -> Harness {
    assert_eq!(MAGIC, *b"W3N1", "reason=magic_mismatch");
    assert_eq!(HEADER_LEN, 25, "reason=header_len_unchanged");
    assert_eq!(PROTOCOL_VERSION, 10, "reason=protocol_version_expected_10");
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_unchanged");

    let bus = LoopbackBus::shared();
    let relay = Relay::new(LoopbackTransport::new(0, &bus), 2, 2);
    let client1 = ClientSession::new(LoopbackTransport::new(1, &bus), Endpoint::Loopback(0), 1, 0);
    let client2 = ClientSession::new(LoopbackTransport::new(2, &bus), Endpoint::Loopback(0), 2, 0);
    let mut harness = Harness {
        bus,
        relay,
        client1,
        client2,
        sim1: new_match(),
        sim2: new_match(),
        opening: opening(),
    };

    let hello = Packet::new(PacketKind::Hello, 0, 0, 0, vec![])
        .encode()
        .expect("ai_lockstep: reason=hello_encode_failed");
    harness
        .client1
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .expect("ai_lockstep: reason=hello_send_failed client=1");
    harness
        .client2
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .expect("ai_lockstep: reason=hello_send_failed client=2");
    harness
        .relay
        .poll()
        .expect("ai_lockstep: reason=relay_poll_failed stage=hello");
    harness
        .client1
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=1 stage=welcome");
    harness
        .client2
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=2 stage=welcome");

    assert_eq!(
        harness.client1.player_id(),
        LOCAL_PLAYER,
        "reason=client1_player_id_mismatch"
    );
    assert_eq!(
        harness.client2.player_id(),
        AI_PLAYER,
        "reason=client2_player_id_mismatch"
    );
    assert_eq!(
        harness.relay.lobby().players(),
        vec![LOCAL_PLAYER, AI_PLAYER],
        "reason=lobby_players_mismatch"
    );
    assert!(
        harness.relay.lobby().is_started(),
        "reason=lobby_not_started"
    );
    assert_eq!(
        harness.opening.ai_buildings.len(),
        2,
        "reason=ai_building_count_mismatch"
    );
    harness
}

/// 一帧内权威命令集的记录，供回放比对使用。
struct FrameTrace {
    frame_commands: FrameCommands,
    hash: u64,
}

/// 断言权威帧批只含冻结真人命令与帧 0 静态 AI 开局定义。
fn assert_no_ai_decision_commands(frame: &FrameCommands, opening: &Opening) {
    for command in &frame.commands {
        assert_eq!(
            command.execute_frame.value(),
            frame.frame,
            "reason=simulation_execute_frame_mismatch frame={}",
            frame.frame
        );
        let allowed_player = command.player_id == LOCAL_PLAYER
            || (frame.frame == 0
                && command.player_id == AI_PLAYER
                && matches!(command.kind, CommandKind::Spawn { .. }));
        assert!(
            allowed_player,
            "reason=unexpected_simulation_player frame={} player={} kind={:?}",
            frame.frame, command.player_id, command.kind
        );
        assert!(
            !matches!(command.kind, CommandKind::Move { .. }),
            "reason=building_or_resource_move_injected frame={}",
            frame.frame
        );
    }

    for command in &frame.gameplay {
        assert_eq!(
            command.execute_frame.value(),
            frame.frame,
            "reason=gameplay_execute_frame_mismatch frame={}",
            frame.frame
        );
        assert_eq!(
            command.player_id, LOCAL_PLAYER,
            "reason=ai_player_gameplay_command_in_frame frame={}",
            frame.frame
        );
        match command.kind {
            GameplayCommandKind::SetAi { enabled } => assert!(
                enabled,
                "reason=run_ai_must_stay_enabled frame={}",
                frame.frame
            ),
            GameplayCommandKind::Gather { unit } => assert_eq!(
                unit, opening.local_worker,
                "reason=unexpected_external_gather frame={}",
                frame.frame
            ),
            GameplayCommandKind::Train { producer, .. } => assert_eq!(
                producer, opening.local_hall,
                "reason=unexpected_producer frame={}",
                frame.frame
            ),
            GameplayCommandKind::Leave { player } => {
                assert_eq!(
                    player, AI_PLAYER,
                    "reason=leave_target_mismatch frame={}",
                    frame.frame
                );
                assert_eq!(
                    frame.frame, LEAVE_FRAME,
                    "reason=leave_must_follow_building_destruction frame={}",
                    frame.frame
                );
            }
            other => panic!(
                "reason=unexpected_gameplay_kind frame={} kind={other:?}",
                frame.frame
            ),
        }
    }
}

/// 断言 canonical 往返：两端字节一致、两次编码一致、解码再编码一致、
/// unknown field 被丢弃且不回显。
fn assert_canonical_frame(local: &FrameCommands, remote: &FrameCommands, logged: &FrameCommands) {
    assert_eq!(
        local, remote,
        "reason=endpoint_frame_mismatch frame={}",
        local.frame
    );
    let canonical = encode_commands(&local.commands, &local.gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=local");
    let canonical_again = encode_commands(&local.commands, &local.gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=repeat");
    let remote_bytes = encode_commands(&remote.commands, &remote.gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=remote");
    let logged_bytes = encode_commands(&logged.commands, &logged.gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=log");
    assert_eq!(
        canonical, canonical_again,
        "reason=encoding_not_deterministic frame={}",
        local.frame
    );
    assert_eq!(
        canonical, remote_bytes,
        "reason=endpoint_bytes_mismatch frame={}",
        local.frame
    );
    assert_eq!(
        canonical, logged_bytes,
        "reason=log_bytes_mismatch frame={}",
        local.frame
    );

    let (decoded_commands, decoded_gameplay) = decode_commands(&canonical)
        .expect("ai_lockstep: reason=decode_commands_failed stage=canonical");
    assert_eq!(
        decoded_commands, local.commands,
        "reason=decoded_simulation_commands_mismatch frame={}",
        local.frame
    );
    assert_eq!(
        decoded_gameplay, local.gameplay,
        "reason=decoded_gameplay_commands_mismatch frame={}",
        local.frame
    );
    let reencoded = encode_commands(&decoded_commands, &decoded_gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=roundtrip");
    assert_eq!(
        reencoded, canonical,
        "reason=decode_reencode_mismatch frame={}",
        local.frame
    );

    // 追加字段号 1000 的合法 varint unknown field；prost 解码必须丢弃它，
    // 再编码不得回显，从而验证 canonical 约束不依赖 unknown fields 透传。
    let mut with_unknown = canonical.clone();
    with_unknown.extend_from_slice(&[0xC0, 0x3E, 0x2A]);
    let (unknown_commands, unknown_gameplay) = decode_commands(&with_unknown)
        .expect("ai_lockstep: reason=decode_commands_failed stage=unknown_field");
    let unknown_roundtrip = encode_commands(&unknown_commands, &unknown_gameplay)
        .expect("ai_lockstep: reason=encode_commands_failed stage=unknown_roundtrip");
    assert_eq!(
        unknown_roundtrip, canonical,
        "reason=unknown_field_echoed frame={}",
        local.frame
    );
}

/// 跑一个完整逻辑帧：上行真人命令、两端空 / 非空输入经 relay 收齐、
/// 两端应用同一权威帧、逐帧交换 checksum 并返回权威帧与状态哈希。
fn run_frame(harness: &mut Harness, frame: u64, attacker: Option<EntityId>) -> FrameTrace {
    let (commands, gameplay) = human_script(frame, &harness.opening, attacker);
    harness
        .client1
        .submit_input(frame, &commands, &gameplay)
        .expect("ai_lockstep: reason=submit_input_failed client=1");
    // 第二个只读模拟端点保持既有两端点 checksum 路径，固定上行空输入；
    // AI 席位没有独立连接或决策命令源。
    harness
        .client2
        .submit_input(frame, &[], &[])
        .expect("ai_lockstep: reason=submit_input_failed client=2");
    harness
        .relay
        .poll()
        .expect("ai_lockstep: reason=relay_poll_failed stage=input");
    harness
        .relay
        .tick()
        .expect("ai_lockstep: reason=relay_tick_failed");
    harness
        .client1
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=1 stage=frame");
    harness
        .client2
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=2 stage=frame");

    let local = harness
        .client1
        .pop_ready()
        .expect("ai_lockstep: reason=frame_missing client=1");
    let remote = harness
        .client2
        .pop_ready()
        .expect("ai_lockstep: reason=frame_missing client=2");
    assert_eq!(local.frame, frame, "reason=frame_number_mismatch client=1");
    assert_eq!(remote.frame, frame, "reason=frame_number_mismatch client=2");
    let logged = harness
        .relay
        .log()
        .from_frame(frame)
        .into_iter()
        .next()
        .expect("ai_lockstep: reason=log_frame_missing");
    assert_eq!(
        logged.frame, frame,
        "reason=log_frame_mismatch frame={frame}"
    );
    assert_canonical_frame(&local, &remote, &logged);
    assert_no_ai_decision_commands(&local, &harness.opening);
    assert_no_ai_decision_commands(&remote, &harness.opening);

    let hash1 = apply_frame(&mut harness.sim1, &local.commands, &local.gameplay)
        .expect("ai_lockstep: reason=apply_frame_failed client=1");
    let hash2 = apply_frame(&mut harness.sim2, &remote.commands, &remote.gameplay)
        .expect("ai_lockstep: reason=apply_frame_failed client=2");
    assert_eq!(hash1, hash2, "reason=state_hash_diverged frame={frame}");
    let view1 = harness.sim1.read_view();
    let view2 = harness.sim2.read_view();
    assert_eq!(view1, view2, "reason=read_view_diverged frame={frame}");
    let snapshot1 = harness.sim1.capture_snapshot();
    let snapshot2 = harness.sim2.capture_snapshot();
    assert_eq!(
        snapshot1, snapshot2,
        "reason=capture_snapshot_diverged frame={frame}"
    );
    assert_eq!(
        harness.sim1.first_difference(&harness.sim2),
        None,
        "reason=first_difference_not_none frame={frame}"
    );

    harness
        .client1
        .report_checksum(frame, hash1)
        .expect("ai_lockstep: reason=checksum_send_failed client=1");
    harness
        .client2
        .report_checksum(frame, hash2)
        .expect("ai_lockstep: reason=checksum_send_failed client=2");
    harness
        .relay
        .poll()
        .expect("ai_lockstep: reason=relay_poll_failed stage=checksum");
    harness
        .client1
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=1 stage=checksum");
    harness
        .client2
        .poll()
        .expect("ai_lockstep: reason=client_poll_failed client=2 stage=checksum");
    assert!(
        !harness.client1.checksums().has_desync(),
        "reason=client1_desync frame={frame}"
    );
    assert!(
        !harness.client2.checksums().has_desync(),
        "reason=client2_desync frame={frame}"
    );
    harness
        .client1
        .tick()
        .expect("ai_lockstep: reason=client_tick_failed client=1");
    harness
        .client2
        .tick()
        .expect("ai_lockstep: reason=client_tick_failed client=2");

    FrameTrace {
        frame_commands: local,
        hash: hash1,
    }
}

#[test]
fn online_ai_seat_full_match_is_deterministic_replays_and_restarts_clean() {
    let mut harness = setup();
    let mut hashes = Vec::new();
    let mut trace_frames = Vec::new();
    let mut attacker: Option<EntityId> = None;
    let mut before_train: Option<BTreeSet<u64>> = None;

    for frame in 0..=FINAL_FRAME {
        let trace = run_frame(&mut harness, frame, attacker);
        hashes.push(trace.hash);
        trace_frames.push(trace.frame_commands);

        let view = harness.sim1.read_view();
        assert_eq!(
            view.tick,
            frame + 1,
            "reason=step_count_mismatch frame={frame}"
        );
        let snapshot = harness.sim1.capture_snapshot();
        assert_eq!(
            snapshot.version, SNAPSHOT_VERSION,
            "reason=snapshot_version_changed frame={frame}"
        );
        assert!(
            snapshot.gameplay.ai.enabled,
            "reason=run_ai_disabled frame={frame}"
        );
        assert_eq!(
            snapshot.gameplay.ai.decision_period, 10,
            "reason=ai_decision_period_mismatch frame={frame}"
        );
        let other_ai = harness.sim2.capture_snapshot().gameplay.ai;
        assert_eq!(
            snapshot.gameplay.ai, other_ai,
            "reason=ai_state_diverged frame={frame}"
        );

        if frame == 0 {
            assert_eq!(
                view.entities.len(),
                4,
                "reason=opening_entity_count_mismatch"
            );
            let owners: BTreeSet<u8> = view.entities.iter().map(|entity| entity.owner).collect();
            let expected_owners: BTreeSet<u8> = [LOCAL_PLAYER, AI_PLAYER].into_iter().collect();
            assert_eq!(
                owners, expected_owners,
                "reason=seat_owner_namespace_mismatch"
            );
            assert_eq!(
                snapshot.gameplay.ai.next_decision_frame, AI_DECISION_FRAME,
                "reason=ai_initial_next_frame_mismatch"
            );
        }
        if frame == 4 {
            assert!(
                snapshot.gameplay.players[&LOCAL_PLAYER].gold >= 5,
                "reason=gather_did_not_grow_gold frame={frame}"
            );
        }
        if frame == BUILD_COMPLETE_FRAME {
            before_train = Some(
                view.entities
                    .iter()
                    .map(|entity| entity.id)
                    .collect::<BTreeSet<u64>>(),
            );
        }
        if frame == UNIT_COMPLETE_FRAME {
            let before = before_train
                .take()
                .expect("ai_lockstep: reason=training_baseline_missing");
            let produced: Vec<u64> = view
                .entities
                .iter()
                .filter(|entity| {
                    !before.contains(&entity.id) && entity.owner == LOCAL_PLAYER && entity.speed > 0
                })
                .map(|entity| entity.id)
                .collect();
            assert_eq!(
                produced.len(),
                1,
                "reason=trained_entity_count_mismatch frame={frame}"
            );
            attacker = Some(
                EntityId::new(produced[0]).expect("ai_lockstep: reason=invalid_trained_entity_id"),
            );
            assert!(
                snapshot.gameplay.players[&LOCAL_PLAYER].produced >= 2,
                "reason=build_and_train_not_committed frame={frame}"
            );
        }
        if frame == ATTACK_FRAME {
            for building in &harness.opening.ai_buildings {
                assert!(
                    !view
                        .entities
                        .iter()
                        .any(|entity| entity.id == building.value() && entity.alive),
                    "reason=ai_building_still_alive frame={frame} entity={}",
                    building.value()
                );
            }
            assert!(
                !snapshot.gameplay.victory.resolved,
                "reason=victory_must_wait_for_existing_settlement_command frame={frame}"
            );
        }
        if frame == LEAVE_FRAME {
            let victory = snapshot.gameplay.victory;
            assert!(
                victory.resolved,
                "reason=victory_not_resolved frame={frame}"
            );
            assert_eq!(
                victory.winner,
                Some(LOCAL_PLAYER),
                "reason=victory_winner_mismatch frame={frame}"
            );
            assert_eq!(
                victory.resolved_frame, LEAVE_FRAME,
                "reason=victory_frame_mismatch frame={frame}"
            );
            assert!(
                victory.resolved_frame > 0,
                "reason=victory_frame_not_recorded frame={frame}"
            );
            assert_eq!(
                snapshot.gameplay.ai.next_decision_frame,
                AI_DECISION_FRAME + 10,
                "reason=ai_first_decision_not_advanced frame={frame}"
            );
            let players: Vec<u8> = snapshot.gameplay.players.keys().copied().collect();
            assert_eq!(
                players,
                vec![LOCAL_PLAYER, AI_PLAYER],
                "reason=seat_players_mismatch frame={frame}"
            );
            assert!(
                !snapshot.gameplay.players[&AI_PLAYER].active,
                "reason=ai_seat_still_active frame={frame}"
            );
            assert!(
                snapshot.gameplay.players[&LOCAL_PLAYER].active,
                "reason=human_seat_inactive frame={frame}"
            );
        }
        if frame > LEAVE_FRAME {
            assert!(
                snapshot.gameplay.victory.resolved,
                "reason=victory_reset_after_resolution frame={frame}"
            );
            assert_eq!(
                snapshot.gameplay.victory.winner,
                Some(LOCAL_PLAYER),
                "reason=victory_winner_changed frame={frame}"
            );
            assert_eq!(
                snapshot.gameplay.victory.resolved_frame, LEAVE_FRAME,
                "reason=victory_must_resolve_once frame={frame}"
            );
        }
    }

    assert_eq!(
        harness.relay.log().len() as u64,
        FINAL_FRAME + 1,
        "reason=relay_log_len_mismatch"
    );
    assert_eq!(
        harness.sim1.read_view().tick,
        FINAL_FRAME + 1,
        "reason=step_count_mismatch_final"
    );
    assert_eq!(
        harness.bus.borrow().faults.dropped,
        0,
        "reason=unexpected_bus_faults"
    );

    let original_snapshot = harness.sim1.capture_snapshot();
    let logged_frames = harness.relay.log().from_frame(0);
    assert_eq!(
        trace_frames, logged_frames,
        "reason=endpoint_log_frames_mismatch"
    );
    assert_eq!(
        logged_frames.len(),
        hashes.len(),
        "reason=frame_hash_count_mismatch"
    );
    let replay_packet = resync_replay_packet(0, &logged_frames)
        .expect("ai_lockstep: reason=resync_replay_packet_failed");
    assert_eq!(
        replay_packet.kind,
        PacketKind::ResyncReplay,
        "reason=resync_replay_packet_kind_mismatch"
    );
    let decoded = parse_resync_replay(&replay_packet.payload)
        .expect("ai_lockstep: reason=parse_resync_replay_failed");
    assert_eq!(
        decoded, logged_frames,
        "reason=resync_replay_order_mismatch"
    );

    let mut replay1 = new_match();
    let mut replay2 = new_match();
    for (index, frame_commands) in decoded.iter().enumerate() {
        let replay_hash1 = apply_frame(
            &mut replay1,
            &frame_commands.commands,
            &frame_commands.gameplay,
        )
        .expect("ai_lockstep: reason=replay_apply_frame_failed client=1");
        let replay_hash2 = apply_frame(
            &mut replay2,
            &frame_commands.commands,
            &frame_commands.gameplay,
        )
        .expect("ai_lockstep: reason=replay_apply_frame_failed client=2");
        assert_eq!(
            replay_hash1, replay_hash2,
            "reason=replay_hash_diverged frame={}",
            frame_commands.frame
        );
        assert_eq!(
            replay_hash1, hashes[index],
            "reason=replay_hash_mismatch frame={}",
            frame_commands.frame
        );
        assert_eq!(
            replay1.first_difference(&replay2),
            None,
            "reason=replay_first_difference_not_none frame={}",
            frame_commands.frame
        );
    }
    assert_eq!(
        replay1.capture_snapshot(),
        original_snapshot,
        "reason=replay_final_snapshot_mismatch"
    );

    let fresh = new_match();
    let fresh_snapshot = fresh.capture_snapshot();
    assert!(
        !fresh_snapshot.gameplay.victory.resolved,
        "reason=restart_victory_resolved"
    );
    assert_eq!(
        fresh_snapshot.gameplay.victory.resolved_frame, 0,
        "reason=restart_victory_frame_not_clear"
    );
    assert!(
        fresh.read_view().entities.is_empty(),
        "reason=restart_view_not_empty"
    );
    assert!(
        fresh_snapshot.gameplay.players.is_empty(),
        "reason=restart_players_not_empty"
    );
    assert_eq!(
        fresh_snapshot.version, SNAPSHOT_VERSION,
        "reason=restart_snapshot_version_changed"
    );
}
