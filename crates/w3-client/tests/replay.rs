//! 固定 30 Hz 客户端的确定性与只读边界验收（tasks 4.4 / A3）。
//!
//! 全部用合成命令脚本与合成地形，不读取任何真实导出 JSON / PNG / GLB / 音效，
//! 不引入墙钟、delta 或异步完成顺序。核心断言：
//! - 60 Hz 与 144 Hz 两种重绘节奏喂同一命令脚本，逐帧帧号与 `compute_state_hash()`
//!   完全一致，且与「每帧直接 step」的 30 Hz 基准实例一致；
//! - 不同 `alpha` 连续插值、事件重复投递后，权威状态哈希与快照字节不变，
//!   快照版本仍为 3（头部含 `defs_hash` / `content_hash`）；
//! - `FrameProducer::next_frame()` 只交出当前只读视图，MUST NOT 推进模拟；
//! - 追帧有显式上限且超限被明确报告，累加器只数整数帧步长。

use w3_client::{
    FixedStepClient, FrameAccumulator, LOGIC_FPS, MAX_CATCHUP_STEPS, TICKS_PER_LOGIC_FRAME,
    TICKS_PER_SECOND, present,
};
use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SNAPSHOT_VERSION, SimFixed,
    Snapshot, StaticWorld,
};
use w3_render::{FrameProducer, ModelLibrary, Scene, TerrainMesh, TerrainVertex};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;

fn spawn(x: i64, y: i64) -> CommandKind {
    CommandKind::Spawn {
        x: SimFixed::from_int(x).unwrap(),
        y: SimFixed::from_int(y).unwrap(),
        hp: 100,
        speed: SimFixed::from_int(3).unwrap(),
        attack_damage: 10,
        attack_range: SimFixed::from_int(10).unwrap(),
    }
}

/// 固定的合成命令脚本。每条命令的 `ExecuteFrame` 取冻结的整数帧号，
/// `PlayerId` / `Sequence` 齐全，绝不来自墙钟或呈现帧号（ADR 0002 §3）。
fn command_script() -> Vec<Command> {
    let first = EntityId::new(1).unwrap();
    let second = EntityId::new(2).unwrap();
    vec![
        Command::new(Frame::new(0), 1, 1, spawn(0, 0)),
        Command::new(Frame::new(0), 2, 1, spawn(4, 0)),
        Command::new(
            Frame::new(1),
            1,
            2,
            CommandKind::Move {
                entity: first,
                x: SimFixed::from_int(12).unwrap(),
                y: SimFixed::from_int(9).unwrap(),
                group: 0,
                append: false,
            },
        ),
        Command::new(Frame::new(6), 1, 3, CommandKind::Stop { entity: first }),
        Command::new(
            Frame::new(12),
            2,
            2,
            CommandKind::Move {
                entity: second,
                x: SimFixed::from_int(-6).unwrap(),
                y: SimFixed::from_int(7).unwrap(),
                group: 0,
                append: false,
            },
        ),
    ]
}

fn config() -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    }
}

fn seeded_client() -> FixedStepClient {
    let mut client = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    for command in command_script() {
        client.submit_command(command).unwrap();
    }
    client
}

/// 合成地形：手工构造最小网格，避免依赖本机导出资产。
fn synthetic_terrain() -> TerrainMesh {
    TerrainMesh {
        vertices: vec![TerrainVertex::default(); 4],
        indices: vec![0, 2, 1, 1, 2, 3],
        columns: 2,
        rows: 2,
        cell_size: 128.0,
        origin: [0.0, 0.0],
        cliff_layers: 0,
        water_level: 0.0,
    }
}

/// 稳定字节视图：内核未暴露快照字节编码，用派生的 `Debug` 表示作为
/// 「快照字节」的可观测代理，配合 `Snapshot: PartialEq` 逐字段相等断言。
fn snapshot_bytes(snapshot: &Snapshot) -> Vec<u8> {
    format!("{snapshot:?}").into_bytes()
}

/// 按 `refresh_hz` 节奏模拟 `seconds` 秒的重绘，每次重绘喂入等分的整数节拍。
/// 返回每个逻辑帧推进后的 `(tick, compute_state_hash())` 序列。
fn replay(refresh_hz: u64, seconds: u64) -> Vec<(u64, u64)> {
    let ticks_per_redraw = TICKS_PER_SECOND / refresh_hz;
    assert_eq!(ticks_per_redraw * refresh_hz, TICKS_PER_SECOND);
    let mut client = seeded_client();
    let mut trace = Vec::new();
    for _ in 0..(refresh_hz * seconds) {
        client.advance_with_observer(ticks_per_redraw, |sim| {
            trace.push((sim.read_view().tick, sim.compute_state_hash()));
        });
    }
    trace
}

/// 30 Hz 基准：不经累加器，直接每帧 `step()` 一次。
fn baseline(frames: u64) -> Vec<(u64, u64)> {
    let mut sim = RtsMatch::create_with_world(config(), StaticWorld::default());
    for command in command_script() {
        sim.submit_command(command).unwrap();
    }
    let mut trace = Vec::new();
    for _ in 0..frames {
        sim.step();
        trace.push((sim.read_view().tick, sim.compute_state_hash()));
    }
    trace
}

#[test]
fn dual_refresh_rate_replay_matches_frame_by_frame() {
    let seconds = 3;
    let expected_frames = u64::from(LOGIC_FPS) * seconds;

    let at_60 = replay(60, seconds);
    let at_144 = replay(144, seconds);
    let reference = baseline(expected_frames);

    assert_eq!(at_60.len() as u64, expected_frames);
    assert_eq!(at_144.len() as u64, expected_frames);
    assert_eq!(
        at_60, at_144,
        "60 Hz and 144 Hz must produce identical frame/hash sequence"
    );
    assert_eq!(
        at_60, reference,
        "replay must match the plain 30 Hz step baseline"
    );
    // 逐帧帧号连续递增，防止「状态从未变化」造成的空泛通过。
    for (index, (tick, _)) in at_60.iter().enumerate() {
        assert_eq!(*tick, index as u64 + 1);
    }
    assert!(
        at_60.windows(2).any(|pair| pair[0].1 != pair[1].1),
        "the simulation did not actually advance"
    );
}

#[test]
fn interpolation_alpha_keeps_state_hash_and_snapshot_bytes() {
    let mut client = seeded_client();
    for _ in 0..20 {
        client.advance_one_frame();
    }

    // 先按新语义只读取一次视图与事件拷贝，再取权威基线。
    let (view, events) = client.next_frame();
    let hash_before = client.sim().compute_state_hash();
    let snapshot_before = client.sim().capture_snapshot();
    let bytes_before = snapshot_bytes(&snapshot_before);
    assert_eq!(snapshot_before.version, SNAPSHOT_VERSION);
    assert_eq!(SNAPSHOT_VERSION, 3);
    assert_eq!(snapshot_before.content_hash, CONTENT_HASH);

    let mut scene = Scene::default();
    let library = ModelLibrary::default();
    let terrain = synthetic_terrain();

    // 同一逻辑帧内以不同 alpha 连续插值并生成 GPU 顶点，且反复投递同一批事件。
    for step in 0..=10 {
        let alpha = step as f32 / 10.0;
        let _vertices = present(&mut scene, &view, &events, alpha, &library, &terrain);
        let _again = present(&mut scene, &view, &events, 1.0 - alpha, &library, &terrain);

        assert_eq!(
            client.sim().compute_state_hash(),
            hash_before,
            "alpha interpolation changed the authoritative state hash"
        );
        let snapshot_now = client.sim().capture_snapshot();
        assert_eq!(snapshot_now, snapshot_before, "alpha changed the snapshot");
        assert_eq!(
            snapshot_bytes(&snapshot_now),
            bytes_before,
            "alpha changed the snapshot bytes"
        );
        assert_eq!(snapshot_now.defs_hash, snapshot_before.defs_hash);
        assert_eq!(snapshot_now.content_hash, snapshot_before.content_hash);
        assert_eq!(snapshot_now.version, 3);
    }
}

#[test]
fn frame_producer_next_frame_never_steps_simulation() {
    let mut client = seeded_client();
    let tick_before = client.sim().read_view().tick;
    let hash_before = client.sim().compute_state_hash();

    // next_frame() 只交出当前只读视图与事件拷贝，反复调用也不推进模拟。
    for _ in 0..8 {
        let (view, _events) = client.next_frame();
        assert_eq!(view.tick, tick_before);
    }
    assert_eq!(client.sim().compute_state_hash(), hash_before);

    // 只有显式累加器推进才改变逻辑帧号。
    let plan = client.advance_one_frame();
    assert_eq!(plan.steps, 1);
    assert_eq!(client.sim().read_view().tick, tick_before + 1);
}

#[test]
fn accumulator_advances_only_whole_logic_frames() {
    let mut accumulator = FrameAccumulator::new();
    let half = accumulator.advance(TICKS_PER_LOGIC_FRAME / 2);
    assert_eq!(half.steps, 0);
    assert!((half.alpha - 0.5).abs() < 1e-6);
    assert_eq!(half.dropped_ticks, 0);

    let full = accumulator.advance(TICKS_PER_LOGIC_FRAME / 2);
    assert_eq!(full.steps, 1);
    assert!(full.alpha < 1e-6);
    assert_eq!(accumulator.pending_ticks(), 0);
}

#[test]
fn catch_up_frames_are_capped_and_reported() {
    let mut accumulator = FrameAccumulator::new();
    let huge = TICKS_PER_LOGIC_FRAME * (MAX_CATCHUP_STEPS as u64 + 5);
    let plan = accumulator.advance(huge);
    assert_eq!(plan.steps, MAX_CATCHUP_STEPS, "catch-up must be capped");
    assert!(plan.dropped_ticks > 0, "excess ticks must be reported");
    assert!(
        plan.alpha >= 0.0 && plan.alpha < 1.0,
        "alpha must be a remainder"
    );
    assert!(
        accumulator.pending_ticks() < TICKS_PER_LOGIC_FRAME,
        "remainder must converge below one frame"
    );
}
