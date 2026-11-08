//! 任务 3.1：双客户端同图对战跑通并逐帧哈希一致。
//!
//! 两个 `RtsMatch` 经自建 relay 的权威帧命令流推进，只传输入命令、
//! 不传状态；逐帧比对 `compute_state_hash()`、`read_view()` 与双方
//! checksum 交换结果（spec 的「相同命令流」与 desync 检测 Scenario）。
//! 注意：此处两个客户端实例跑在内存回环总线上；真实 UDP 双进程实机
//! 验收另见 `vera20k-mapping.md`，对应 tasks.md 3.1 的人工验收项。

use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld,
};
use w3_net::packet::{Packet, PacketKind};
use w3_net::transport::{Endpoint, LoopbackBus, LoopbackTransport, SharedBus, Transport};
use w3_net::{ClientSession, apply_frame};
use w3_relay::Relay;

const FRAMES: u64 = 24;

fn new_match() -> RtsMatch {
    RtsMatch::create_with_world(
        MatchConfig {
            seed: 42,
            logic_fps: 30,
            content_hash: 0,
        },
        StaticWorld::open_world(16, 16, SimFixed::from_int(8).unwrap()),
    )
}

/// 每个玩家每帧提交一条输入命令；帧 0 出生，之后周期性移动。
fn commands_for(frame: u64, player: u8) -> Vec<Command> {
    if frame == 0 {
        return vec![Command::new(
            Frame::new(0),
            player,
            1,
            CommandKind::Spawn {
                x: SimFixed::from_raw(65536 * (i64::from(player) + 1)),
                y: SimFixed::from_raw(65536),
                hp: 100,
                speed: SimFixed::from_raw(196608),
                attack_damage: 10,
                attack_range: SimFixed::from_raw(655360),
            },
        )];
    }
    if frame % 4 == 0 {
        return vec![Command::new(
            Frame::new(frame),
            player,
            frame as u32,
            CommandKind::Move {
                entity: EntityId::new(u64::from(player)).unwrap(),
                x: SimFixed::from_raw(262144),
                y: SimFixed::from_raw(262144),
                group: 0,
                append: false,
            },
        )];
    }
    vec![Command::new(
        Frame::new(frame),
        player,
        frame as u32,
        CommandKind::Noop,
    )]
}

struct Harness {
    bus: SharedBus,
    relay: Relay<LoopbackTransport>,
    client1: ClientSession<LoopbackTransport>,
    client2: ClientSession<LoopbackTransport>,
    sim1: RtsMatch,
    sim2: RtsMatch,
}

fn setup() -> Harness {
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
    };

    // lobby 加入：relay 分配稳定 player_id 并回 Welcome。
    let hello = Packet::new(PacketKind::Hello, 0, 0, 0, vec![])
        .encode()
        .unwrap();
    harness
        .client1
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .unwrap();
    harness
        .client2
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .unwrap();
    harness.relay.poll().unwrap();
    harness.client1.poll().unwrap();
    harness.client2.poll().unwrap();
    assert_eq!(harness.client1.player_id(), 1);
    assert_eq!(harness.client2.player_id(), 2);
    assert!(harness.relay.lobby().is_started());
    harness
}

/// 跑一个完整逻辑帧，返回两端状态哈希。
fn run_frame(harness: &mut Harness, frame: u64) -> (u64, u64) {
    harness
        .client1
        .submit_input(frame, &commands_for(frame, 1))
        .unwrap();
    harness
        .client2
        .submit_input(frame, &commands_for(frame, 2))
        .unwrap();

    // relay 收 Input 并回 ACK；认证时钟关闭帧并广播权威命令集。
    harness.relay.poll().unwrap();
    harness.relay.tick().unwrap();

    harness.client1.poll().unwrap();
    harness.client2.poll().unwrap();

    let frame1 = harness.client1.pop_ready().unwrap();
    let frame2 = harness.client2.pop_ready().unwrap();
    assert_eq!(frame1, frame2, "both clients must receive the same frame");
    assert_eq!(frame1.frame, frame);

    let hash1 = apply_frame(&mut harness.sim1, &frame1.commands).unwrap();
    let hash2 = apply_frame(&mut harness.sim2, &frame2.commands).unwrap();

    // 逐帧 checksum 交换（relay 转发给对端）。
    harness
        .client1
        .report_checksum(frame1.frame, hash1)
        .unwrap();
    harness
        .client2
        .report_checksum(frame2.frame, hash2)
        .unwrap();
    harness.relay.poll().unwrap();
    harness.client1.poll().unwrap();
    harness.client2.poll().unwrap();

    harness.client1.tick().unwrap();
    harness.client2.tick().unwrap();
    (hash1, hash2)
}

#[test]
fn two_clients_stay_bit_identical_every_frame() {
    let mut harness = setup();
    let mut previous: Option<u64> = None;
    for frame in 0..FRAMES {
        let (hash1, hash2) = run_frame(&mut harness, frame);
        assert_eq!(hash1, hash2, "state hash diverged at frame {frame}");
        assert_eq!(
            harness.sim1.read_view(),
            harness.sim2.read_view(),
            "read_view diverged at frame {frame}"
        );
        assert!(
            !harness.client1.checksums().has_desync(),
            "client 1 reported desync at frame {frame}"
        );
        assert!(
            !harness.client2.checksums().has_desync(),
            "client 2 reported desync at frame {frame}"
        );
        if let Some(prev) = previous {
            assert_ne!(hash1, prev, "simulation must advance every frame");
        }
        previous = Some(hash1);
    }
    assert_eq!(harness.relay.log().len() as u64, FRAMES);
    assert_eq!(harness.sim1.read_view().tick, FRAMES);
    assert_eq!(harness.sim1.drain_events(), harness.sim2.drain_events());
    assert_eq!(harness.bus.borrow().faults.dropped, 0);
}

#[test]
fn only_input_commands_cross_the_wire() {
    let mut harness = setup();
    // 帧 0 的权威命令集只包含输入命令，且不含任何模拟状态字段。
    harness
        .client1
        .submit_input(0, &commands_for(0, 1))
        .unwrap();
    harness
        .client2
        .submit_input(0, &commands_for(0, 2))
        .unwrap();
    harness.relay.poll().unwrap();
    harness.relay.tick().unwrap();
    let recorded = harness.relay.log().from_frame(0);
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].commands.len(), 2);
    for command in &recorded[0].commands {
        assert!(matches!(command.kind, CommandKind::Spawn { .. }));
    }
}
