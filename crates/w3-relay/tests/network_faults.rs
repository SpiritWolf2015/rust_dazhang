//! 任务 3.2：注入丢包 / 乱序 / 短暂断线的回归用例。
//!
//! 通过内存总线的故障注入验证：上下行丢包都由可靠层重传补齐且仍在原定
//! 帧执行；乱序帧按帧号重排；短暂断线后按输入游标追帧，不重复执行命令。

use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld,
};
use w3_net::packet::{Packet, PacketKind};
use w3_net::transport::{Endpoint, LoopbackBus, LoopbackTransport, SharedBus, Transport};
use w3_net::{ClientSession, ReconnectPhase, apply_frame};
use w3_relay::Relay;

fn new_match() -> RtsMatch {
    RtsMatch::create_with_world(
        MatchConfig {
            seed: 7,
            logic_fps: 30,
            content_hash: 0,
        },
        StaticWorld::open_world(16, 16, SimFixed::from_int(8).unwrap()),
    )
}

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
    c1: ClientSession<LoopbackTransport>,
    c2: ClientSession<LoopbackTransport>,
    sim1: RtsMatch,
    sim2: RtsMatch,
}

fn setup() -> Harness {
    let bus = LoopbackBus::shared();
    let mut harness = Harness {
        relay: Relay::new(LoopbackTransport::new(0, &bus), 2, 2),
        c1: ClientSession::new(LoopbackTransport::new(1, &bus), Endpoint::Loopback(0), 1, 0),
        c2: ClientSession::new(LoopbackTransport::new(2, &bus), Endpoint::Loopback(0), 2, 0),
        sim1: new_match(),
        sim2: new_match(),
        bus,
    };
    let hello = Packet::new(PacketKind::Hello, 0, 0, 0, vec![])
        .encode()
        .unwrap();
    harness
        .c1
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .unwrap();
    harness
        .c2
        .transport_mut()
        .send(&Endpoint::Loopback(0), &hello)
        .unwrap();
    harness.relay.poll().unwrap();
    harness.c1.poll().unwrap();
    harness.c2.poll().unwrap();
    harness
}

/// 推进 relay 直到广播出一个新帧（缺失玩家输入时靠超时关闭）。
fn advance_relay_frame(harness: &mut Harness) {
    let before = harness.relay.log().len();
    for _ in 0..4 {
        harness.relay.tick().unwrap();
        if harness.relay.log().len() > before {
            return;
        }
    }
    panic!("relay did not produce a new frame");
}

/// 正常跑一帧（两个客户端都提交输入并应用同一权威帧）。
fn full_frame(harness: &mut Harness, frame: u64) {
    harness
        .c1
        .submit_input(frame, &commands_for(frame, 1))
        .unwrap();
    harness
        .c2
        .submit_input(frame, &commands_for(frame, 2))
        .unwrap();
    harness.relay.poll().unwrap();
    harness.relay.tick().unwrap();
    harness.c1.poll().unwrap();
    harness.c2.poll().unwrap();
    let f1 = harness.c1.pop_ready().unwrap();
    let f2 = harness.c2.pop_ready().unwrap();
    assert_eq!(f1, f2);
    let h1 = apply_frame(&mut harness.sim1, &f1.commands).unwrap();
    let h2 = apply_frame(&mut harness.sim2, &f2.commands).unwrap();
    assert_eq!(h1, h2);
    harness.c1.tick().unwrap();
    harness.c2.tick().unwrap();
}

#[test]
fn dropped_uplink_input_is_retransmitted_and_lands_in_its_original_frame() {
    let mut harness = setup();
    // 丢弃客户端 1 的第一次 Input 发送。
    harness.bus.borrow_mut().drop_next(1);
    harness.c1.submit_input(0, &commands_for(0, 1)).unwrap();
    harness.c2.submit_input(0, &commands_for(0, 2)).unwrap();
    harness.relay.poll().unwrap();

    // 未收齐，帧 0 暂不产出。
    let before = harness.relay.log().len();
    harness.relay.tick().unwrap();
    assert_eq!(harness.relay.log().len(), before);

    // 逻辑 tick 驱动可靠层重传（阈值 3）。
    for _ in 0..3 {
        harness.c1.tick().unwrap();
    }
    harness.relay.poll().unwrap();
    advance_relay_frame(&mut harness);
    assert_eq!(harness.relay.log().len(), before + 1);

    harness.c1.poll().unwrap();
    harness.c2.poll().unwrap();
    let f1 = harness.c1.pop_ready().unwrap();
    let f2 = harness.c2.pop_ready().unwrap();
    assert_eq!(f1, f2);
    assert_eq!(
        f1.frame, 0,
        "retransmitted command stays in its original frame"
    );
    assert_eq!(f1.commands.len(), 2, "both spawn commands recovered");
    let h1 = apply_frame(&mut harness.sim1, &f1.commands).unwrap();
    let h2 = apply_frame(&mut harness.sim2, &f2.commands).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(harness.bus.borrow().faults.dropped, 1);
}

#[test]
fn dropped_downlink_frame_is_retransmitted() {
    let mut harness = setup();
    harness.c1.submit_input(0, &commands_for(0, 1)).unwrap();
    harness.c2.submit_input(0, &commands_for(0, 2)).unwrap();
    harness.relay.poll().unwrap();
    // 丢弃广播给客户端 1 的第一个下行 Frame 包。
    harness.bus.borrow_mut().drop_next(1);
    harness.relay.tick().unwrap();

    harness.c1.poll().unwrap();
    assert!(
        harness.c1.pop_ready().is_none(),
        "client 1 has not received the frame yet"
    );

    // relay 逻辑 tick 驱动下行重传（阈值 3）。
    for _ in 0..3 {
        harness.relay.tick().unwrap();
    }
    harness.c1.poll().unwrap();
    harness.c2.poll().unwrap();
    let f1 = harness
        .c1
        .pop_ready()
        .expect("downlink retransmission missing");
    let f2 = harness.c2.pop_ready().unwrap();
    assert_eq!(f1, f2);
    let h1 = apply_frame(&mut harness.sim1, &f1.commands).unwrap();
    let h2 = apply_frame(&mut harness.sim2, &f2.commands).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn out_of_order_frames_are_reordered_by_frame_number() {
    let mut harness = setup();

    // 帧 0 广播后客户端先不 poll，让 Frame0 留在队列里。
    harness.c1.submit_input(0, &commands_for(0, 1)).unwrap();
    harness.c2.submit_input(0, &commands_for(0, 2)).unwrap();
    harness.relay.poll().unwrap();
    harness.relay.tick().unwrap();

    // 帧 1 的 Input 与 ACK 先走完，再对广播注入乱序。
    harness.c1.submit_input(1, &commands_for(1, 1)).unwrap();
    harness.c2.submit_input(1, &commands_for(1, 2)).unwrap();
    harness.relay.poll().unwrap();
    harness.bus.borrow_mut().reorder_next(2);
    harness.relay.tick().unwrap();

    // 客户端一次 poll 收到两个乱序 Frame，调度器按帧号重排。
    harness.c1.poll().unwrap();
    harness.c2.poll().unwrap();
    let a1 = harness.c1.pop_ready().unwrap();
    let b1 = harness.c1.pop_ready().unwrap();
    let a2 = harness.c2.pop_ready().unwrap();
    let b2 = harness.c2.pop_ready().unwrap();
    assert_eq!((a1.frame, b1.frame), (0, 1));
    assert_eq!((a2.frame, b2.frame), (0, 1));

    for (left, right) in [(a1, a2), (b1, b2)] {
        let h1 = apply_frame(&mut harness.sim1, &left.commands).unwrap();
        let h2 = apply_frame(&mut harness.sim2, &right.commands).unwrap();
        assert_eq!(h1, h2);
    }
    assert_eq!(harness.sim1.read_view(), harness.sim2.read_view());
}

#[test]
fn short_disconnection_catches_up_without_replaying_commands() {
    let mut harness = setup();
    for frame in 0..3 {
        full_frame(&mut harness, frame);
    }
    let aligned = harness.sim1.compute_state_hash();
    assert_eq!(aligned, harness.sim2.compute_state_hash());

    // 断开客户端 2：relay 按超时把缺席玩家视为空输入，继续推进客户端 1。
    harness.bus.borrow_mut().disconnect(2);
    for frame in 3..5 {
        harness
            .c1
            .submit_input(frame, &commands_for(frame, 1))
            .unwrap();
        harness.relay.poll().unwrap();
        advance_relay_frame(&mut harness);
        harness.c1.poll().unwrap();
        let frame_commands = harness.c1.pop_ready().unwrap();
        apply_frame(&mut harness.sim1, &frame_commands.commands).unwrap();
        harness.c1.tick().unwrap();
    }
    assert_ne!(
        harness.sim1.compute_state_hash(),
        harness.sim2.compute_state_hash(),
        "disconnected client should temporarily lag behind"
    );

    // 重连并按输入游标补发错过的帧。
    harness.bus.borrow_mut().reconnect(2);
    harness.c2.poll().unwrap();
    harness.c2.request_resync().unwrap();
    harness.relay.poll().unwrap();
    harness.c2.poll().unwrap();

    let mut replayed = 0u32;
    while let Some(frame_commands) = harness.c2.pop_ready() {
        assert!(frame_commands.frame >= 3, "must not replay applied frames");
        apply_frame(&mut harness.sim2, &frame_commands.commands).unwrap();
        replayed += 1;
    }
    assert_eq!(replayed, 2);
    assert_eq!(
        harness.sim1.compute_state_hash(),
        harness.sim2.compute_state_hash(),
        "state hash must converge after catch-up"
    );
    assert_eq!(harness.c2.reconnect().phase(), ReconnectPhase::Healthy);
}
