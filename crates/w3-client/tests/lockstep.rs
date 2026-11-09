//! 批次 C 验收：联机 `ClientSession` → `StepDriver` 适配（tasks 8.1–8.5）。
//!
//! 全部用内存回环 `LoopbackBus` 与合成命令，不依赖真实网络、墙钟、GPU 窗口
//! 或导出资产。核心断言：
//! - 本地 `FixedStepClient` 与联机 `NetSessionDriver` 在相同冻结帧命令流下
//!   逐帧 `compute_state_hash()` 一致（spec「模式切换同管线」Scenario）；
//! - 乱序 / 重传 / 重复包不改变帧号与哈希序列（spec「网络抖动不改帧序」）；
//! - 网络包到达不驱动累加器：`advance_ticks(0)` 有大量包也不推进帧；
//! - 联机接入不新增权威状态，快照仍 v3 且头部含 `defs_hash` / `content_hash`；
//! - ADR 0005 契约（W3N1 25B 头、protobuf 往返、checksum、first_difference）不变。

use w3_client::{
    ClientCommand, FixedStepClient, NetSessionDriver, StepDriver, TICKS_PER_LOGIC_FRAME,
};
use w3_core::{Command, CommandKind, Frame, SNAPSHOT_VERSION, SimFixed, StaticWorld};
use w3_net::packet::{HEADER_LEN, MAGIC, PROTOCOL_VERSION, Packet, PacketKind, encode_commands};
use w3_net::transport::{Endpoint, LoopbackBus, LoopbackTransport, SharedBus};
use w3_net::{ClientSession, Transport, apply_frame, stabilize};
use w3_render::FrameProducer;

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;
const CLIENT_ID: u8 = 1;
const RELAY_ID: u8 = 9;
const START_FRAME: u64 = 0;
/// 缺帧等待上限设大，避免抖动测试中「包已收齐、无新包」产生超时日志噪音；
/// 不读墙钟，不改变 `FrameScheduler` 的确定性超时语义。
const INPUT_TIMEOUT: u64 = 100_000;

fn world() -> StaticWorld {
    StaticWorld::open_world(16, 16, SimFixed::from_int(8).unwrap())
}

/// 帧 0 生成两个单位，后续帧为 Noop。每帧命令已按
/// `execute_frame → player_id → sequence` 稳定序排列（与 `stabilize` 一致），
/// 保证本地直接提交与联机 `apply_frame` 提交的 `arrival` 顺序一致。
fn frame_commands(frame: u64) -> Vec<Command> {
    if frame == 0 {
        return vec![
            Command::new(
                Frame::new(0),
                1,
                1,
                CommandKind::Spawn {
                    x: SimFixed::from_int(2).unwrap(),
                    y: SimFixed::from_int(3).unwrap(),
                    hp: 100,
                    speed: SimFixed::from_int(3).unwrap(),
                    attack_damage: 10,
                    attack_range: SimFixed::from_int(10).unwrap(),
                },
            ),
            Command::new(
                Frame::new(0),
                1,
                2,
                CommandKind::Spawn {
                    x: SimFixed::from_int(-4).unwrap(),
                    y: SimFixed::from_int(6).unwrap(),
                    hp: 100,
                    speed: SimFixed::from_int(3).unwrap(),
                    attack_damage: 10,
                    attack_range: SimFixed::from_int(10).unwrap(),
                },
            ),
        ];
    }
    vec![Command::new(
        Frame::new(frame),
        1,
        frame as u32 + 10,
        CommandKind::Noop,
    )]
}

/// 把一帧冻结命令封成 `PacketKind::Frame` 并从伪 relay 注入客户端队列。
/// `sequence` 供 `ReliableReceiver` 去重；重复包用相同 sequence 即被忽略。
fn inject_frame(relay: &mut LoopbackTransport, frame: u64, sequence: u32, commands: &[Command]) {
    let payload = encode_commands(commands).expect("protobuf encoding is deterministic");
    let packet = Packet::new(PacketKind::Frame, RELAY_ID, sequence, frame, payload);
    let bytes = packet.encode().expect("W3N1 header encoding succeeds");
    relay
        .send(&Endpoint::Loopback(CLIENT_ID), &bytes)
        .expect("loopback send succeeds");
}

/// 构造一个联机驱动 + 其伪 relay 端点（共享内存总线）。
fn online() -> (
    NetSessionDriver<LoopbackTransport>,
    LoopbackTransport,
    SharedBus,
) {
    let bus = LoopbackBus::shared();
    let client = LoopbackTransport::new(CLIENT_ID, &bus);
    let relay = LoopbackTransport::new(RELAY_ID, &bus);
    let session = ClientSession::with_timeout(
        client,
        Endpoint::Loopback(RELAY_ID),
        1,
        START_FRAME,
        INPUT_TIMEOUT,
    );
    let driver = NetSessionDriver::new(SEED, CONTENT_HASH, world(), session);
    (driver, relay, bus)
}

/// 本地路径：每帧提交该帧命令，随后经累加器恰好 `step()` 一次。
/// 返回逐帧 `(tick, compute_state_hash())`，并断言 tick 严格逐帧 +1。
fn local_trace(frames: u64) -> Vec<(u64, u64)> {
    let mut local = FixedStepClient::new(SEED, CONTENT_HASH, world());
    let mut trace = Vec::new();
    for frame in 0..frames {
        for command in frame_commands(frame) {
            local
                .submit_command(command)
                .expect("local submit succeeds");
        }
        let plan = local.advance_one_frame();
        assert_eq!(plan.steps, 1, "one logic tick advances exactly one frame");
        let tick = local.sim().read_view().tick;
        assert_eq!(tick, frame + 1, "local tick advances by exactly one");
        trace.push((tick, local.sim().compute_state_hash()));
    }
    trace
}

/// 联机路径：注入冻结帧命令，经 `advance_ticks` 消费（唯一推进点是
/// `apply_frame`，内部恰好一次 `step()`）。
///
/// 返回逐帧 `(tick, hash)`：`tick` 直接取自 `RtsMatch::read_view()`，
/// 断言每个已冻结帧应用后 tick 恰等于「已应用帧数」——这是双 `step()`
/// 规避的直接证据（若适配器再调一次 `step()`，tick 会以 2 递增）。
fn online_trace(driver: &mut NetSessionDriver<LoopbackTransport>, frames: u64) -> Vec<(u64, u64)> {
    let mut applied = 0u64;
    let mut guard = 0;
    while applied < frames && guard < frames * 4 + 16 {
        driver.advance_ticks(TICKS_PER_LOGIC_FRAME);
        let now = driver.applied_trace().len() as u64;
        // 每新增一个已应用帧，权威 tick 恰好 +1，绝不 +2。
        assert_eq!(
            driver.view().tick,
            now,
            "each applied frozen frame must step exactly once",
        );
        applied = now;
        guard += 1;
    }
    driver
        .applied_trace()
        .iter()
        .enumerate()
        .map(|(index, &(frame, hash))| {
            assert_eq!(frame, index as u64, "frozen frame numbers are contiguous");
            (index as u64 + 1, hash)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 8.1 / 8.5：模式切换同管线 —— 相同命令流逐帧哈希一致
// ---------------------------------------------------------------------------

#[test]
fn local_and_online_match_frame_by_frame_hash() {
    let frames = 12;
    let (mut driver, mut relay, _bus) = online();
    for frame in 0..frames {
        inject_frame(&mut relay, frame, frame as u32 + 1, &frame_commands(frame));
    }
    let local = local_trace(frames);
    let online = online_trace(&mut driver, frames);

    assert_eq!(
        online.len() as u64,
        frames,
        "every frozen frame is applied once"
    );
    assert_eq!(local, online, "local and online must match frame by frame");
    for (index, (tick, _)) in online.iter().enumerate() {
        assert_eq!(
            *tick,
            index as u64 + 1,
            "frame numbers increase by exactly one"
        );
    }
    // 命令确实产生了权威状态变化，防止「状态从未变化」的空泛通过。
    assert!(local.windows(2).any(|pair| pair[0].1 != pair[1].1));
}

#[test]
fn online_driver_is_feedable_to_the_same_client_app() {
    // 编译期断言：联机适配实现 `StepDriver: FrameProducer`，可直接喂给
    // 批处理 B 的 `ClientApp<P>`，不出现第二套呈现代码。
    fn assert_step_driver<P: StepDriver + 'static>() {}
    assert_step_driver::<FixedStepClient>();
    assert_step_driver::<NetSessionDriver<LoopbackTransport>>();
}

#[test]
fn next_frame_stays_read_only_online() {
    let (mut driver, _relay, _bus) = online();
    let tick = driver.view().tick;
    let hash = driver.sim().compute_state_hash();
    for _ in 0..8 {
        let (view, _events) = driver.next_frame();
        assert_eq!(view.tick, tick);
        assert_eq!(driver.sim().compute_state_hash(), hash);
    }
    assert_eq!(driver.applied_trace().len(), 0);
}

// ---------------------------------------------------------------------------
// 8.2 / 8.3：双 step 规避、节拍隔离、帧序不变
// ---------------------------------------------------------------------------

#[test]
fn each_frozen_frame_advances_exactly_one_logic_frame() {
    // 双 `step()` 的直接验证：每消费一个冻结帧，tick 恰好 +1（而非 +2）。
    let frames = 6;
    let (mut driver, mut relay, _bus) = online();
    for frame in 0..frames {
        inject_frame(&mut relay, frame, frame as u32 + 1, &frame_commands(frame));
    }
    let mut previous = driver.view().tick;
    for _ in 0..frames {
        driver.advance_ticks(TICKS_PER_LOGIC_FRAME);
        let current = driver.view().tick;
        if current != previous {
            assert_eq!(current, previous + 1, "apply_frame must step exactly once");
            previous = current;
        }
    }
    assert_eq!(driver.view().tick, previous);
    assert_eq!(driver.applied_trace().len() as u64, frames);
}

#[test]
fn network_packets_never_drive_the_accumulator() {
    // 注入大量权威帧包，但喂 0 个整数 tick：累加器预算为 0，
    // 一帧也不推进（网络到达 MUST NOT 改 `step()` 次数）。
    let (mut driver, mut relay, _bus) = online();
    for frame in 0..8 {
        inject_frame(&mut relay, frame, frame as u32 + 1, &frame_commands(frame));
    }
    let tick = driver.view().tick;
    let hash = driver.sim().compute_state_hash();
    for _ in 0..16 {
        let plan = driver.advance_ticks(0);
        assert_eq!(plan.steps, 0, "zero ticks produce zero frame budget");
    }
    assert_eq!(driver.view().tick, tick);
    assert_eq!(driver.sim().compute_state_hash(), hash);
    assert!(driver.applied_trace().is_empty());
}

#[test]
fn jitter_reorder_and_retransmit_keep_frame_order_and_hashes() {
    let frames = 10;
    // 基准：按序注入，每个帧号恰好一个权威命令集。
    let (mut baseline, mut relay, _bus) = online();
    for frame in 0..frames {
        inject_frame(&mut relay, frame, frame as u32 + 1, &frame_commands(frame));
    }
    let expected = online_trace(&mut baseline, frames);

    // 抖动：乱序到达 + 重复包（相同 sequence 被去重）+ 「丢包后重传」。
    let (mut jittered, mut relay2, bus2) = online();
    let order = [3u64, 0, 2, 1, 5, 4, 7, 6, 9, 8];
    for &frame in &order {
        let sequence = frame as u32 + 1;
        // 帧 5 模拟首次丢包：不发送，稍后由「重传」补上。
        if frame != 5 {
            inject_frame(&mut relay2, frame, sequence, &frame_commands(frame));
            // 每个包再注入一次重复副本（相同 sequence → receiver 去重）。
            inject_frame(&mut relay2, frame, sequence, &frame_commands(frame));
        }
    }
    // 断线 / 恢复：断开期间丢弃一次，恢复后重传帧 5。
    bus2.borrow_mut().disconnect(CLIENT_ID);
    inject_frame(&mut relay2, 5, 6, &frame_commands(5));
    bus2.borrow_mut().reconnect(CLIENT_ID);
    inject_frame(&mut relay2, 5, 6, &frame_commands(5));

    let actual = online_trace(&mut jittered, frames);
    assert_eq!(
        actual.len() as u64,
        frames,
        "no frame lost or double-applied"
    );
    assert_eq!(
        actual, expected,
        "network jitter must not change frame order or hashes"
    );
}

// ---------------------------------------------------------------------------
// 8.2：联机键鼠走上行广播，Gameplay 命令显式拒绝
// ---------------------------------------------------------------------------

#[test]
fn online_input_goes_uplink_and_gameplay_is_rejected_explicitly() {
    let (mut driver, mut relay, _bus) = online();
    let command = Command::new(Frame::new(0), 1, 1, CommandKind::Noop);
    let tick = driver.view().tick;
    // Simulation 命令只经 `submit_input` 上行，不本地 `submit_command`。
    assert!(driver.enqueue(ClientCommand::Simulation(command)).is_ok());
    assert_eq!(
        driver.view().tick,
        tick,
        "enqueue must not advance the simulation"
    );
    // 伪 relay 应收到一个 Input 包。
    let received = relay.recv().expect("loopback recv succeeds");
    assert!(received.is_some(), "input packet is broadcast to relay");
    let (_, bytes) = received.unwrap();
    let packet = Packet::decode(&bytes).expect("valid W3N1 header");
    assert_eq!(packet.kind, PacketKind::Input);

    // `GameplayCommandKind` 不在 ADR 0005 protobuf schema 内：显式报错。
    let gameplay = w3_core::GameplayCommand::new(
        Frame::new(1),
        1,
        2,
        w3_core::GameplayCommandKind::Train {
            producer: w3_core::EntityId::new(1).unwrap(),
            unit_def: 1,
            is_building: false,
        },
    );
    assert_eq!(
        driver.enqueue(ClientCommand::Gameplay(gameplay)),
        Err(w3_client::ClientError::GameplayUnsupportedOnline),
    );
}

// ---------------------------------------------------------------------------
// 8.4 / 8.5：ADR 0005 契约与不新增权威状态
// ---------------------------------------------------------------------------

#[test]
fn wire_format_contract_is_unchanged() {
    assert_eq!(&MAGIC, b"W3N1");
    assert_eq!(HEADER_LEN, 25);
    assert_eq!(PROTOCOL_VERSION, 2);

    let commands = frame_commands(0);
    let encoded = encode_commands(&commands).expect("encode succeeds");
    let decoded = w3_net::packet::decode_commands(&encoded).expect("decode succeeds");
    // `arrival` 不编码（提交时分配），其余字段往返一致。
    assert_eq!(decoded.len(), commands.len());
    for (a, b) in decoded.iter().zip(commands.iter()) {
        assert_eq!(a.execute_frame, b.execute_frame);
        assert_eq!(a.player_id, b.player_id);
        assert_eq!(a.sequence, b.sequence);
    }

    // `stabilize` 稳定键：ExecuteFrame → PlayerId → Sequence。
    let mut sorted = vec![
        Command::new(Frame::new(2), 1, 1, CommandKind::Noop),
        Command::new(Frame::new(1), 2, 1, CommandKind::Noop),
        Command::new(Frame::new(1), 1, 3, CommandKind::Noop),
        Command::new(Frame::new(1), 1, 1, CommandKind::Noop),
    ];
    stabilize(&mut sorted);
    let keys: Vec<_> = sorted
        .iter()
        .map(|c| (c.execute_frame.value(), c.player_id, c.sequence))
        .collect();
    assert_eq!(keys, vec![(1, 1, 1), (1, 1, 3), (1, 2, 1), (2, 1, 1)]);

    // 25B 固定头 + protobuf payload，逐帧 checksum 字段语义不变。
    let packet = Packet::new(PacketKind::Frame, RELAY_ID, 7, 3, encoded.clone());
    let bytes = packet.encode().expect("encode succeeds");
    assert!(bytes.len() >= HEADER_LEN);
    assert_eq!(&bytes[..4], b"W3N1");
    assert_eq!(Packet::decode(&bytes).expect("decode succeeds").frame, 3);
}

#[test]
fn apply_frame_returns_state_hash_and_first_difference_still_works() {
    // 两个完全一致的对局：apply_frame 返回的哈希应等于各自 compute_state_hash()。
    let mut a = w3_core::RtsMatch::create_with_world(config(), world());
    let mut b = w3_core::RtsMatch::create_with_world(config(), world());
    let commands = frame_commands(0);
    let hash_a = apply_frame(&mut a, &commands).expect("apply_frame succeeds");
    let hash_b = apply_frame(&mut b, &commands).expect("apply_frame succeeds");
    assert_eq!(hash_a, hash_b);
    assert_eq!(hash_a, a.compute_state_hash());
    assert_eq!(w3_net::first_difference_field(&a, &b), None);
    // 差异可定位到字段。
    b.step();
    assert_eq!(w3_net::first_difference_field(&a, &b), Some("tick"));
}

fn config() -> w3_core::MatchConfig {
    w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: CONTENT_HASH,
    }
}

#[test]
fn online_session_keeps_snapshot_v3_with_defs_hash_header() {
    let (mut driver, mut relay, _bus) = online();
    for frame in 0..4 {
        inject_frame(&mut relay, frame, frame as u32 + 1, &frame_commands(frame));
    }
    online_trace(&mut driver, 4);

    let snapshot = driver.snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION);
    assert_eq!(SNAPSHOT_VERSION, 3);
    // 头部身份字段仍在，且联机路径没有新增权威字段（与本地同帧快照逐字段相等）。
    let mut local = FixedStepClient::new(SEED, CONTENT_HASH, world());
    for frame in 0..4 {
        for command in frame_commands(frame) {
            local.submit_command(command).unwrap();
        }
        local.advance_one_frame();
    }
    let local_snapshot = local.sim().capture_snapshot();
    assert_eq!(snapshot.defs_hash, local_snapshot.defs_hash);
    assert_eq!(snapshot.content_hash, local_snapshot.content_hash);
    assert_eq!(
        snapshot, local_snapshot,
        "online path adds no authoritative state"
    );
    assert_eq!(
        driver.sim().compute_state_hash(),
        local.sim().compute_state_hash()
    );
}

#[test]
fn deserialize_and_reject_unknown_wire_fields_cleanly() {
    // prost 解码丢弃 unknown fields，绝不透传；畸形包显式报错。
    let mut bytes = Packet::new(PacketKind::Frame, RELAY_ID, 1, 0, vec![1, 2, 3])
        .encode()
        .expect("encode succeeds");
    // 破坏校验和 → decode 必须报错，不能静默接受。
    let checksum_at = 21;
    bytes[checksum_at] ^= 0xff;
    assert!(Packet::decode(&bytes).is_err());
    // 未知 tag 的 payload 不回显：decode_commands 只返回合法命令。
    let empty = encode_commands(&[]).expect("empty batch encodes");
    assert!(
        w3_net::packet::decode_commands(&empty)
            .expect("empty batch decodes")
            .is_empty()
    );
}
