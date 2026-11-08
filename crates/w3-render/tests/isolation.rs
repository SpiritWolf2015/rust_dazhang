//! 确定性隔离验收（`3d-rendering` spec）。
//!
//! 表现层消费只读视图、模型加载失败、动画丢帧、位置插值与事件重复投递，
//! 都 MUST NOT 改变 `RtsMatch` 的权威状态哈希。测试建立两个完全相同的
//! 对局，只有一个额外跑完整表现层管线，逐帧比对 `compute_state_hash()`
//! 与 `read_view()`。

use w3_core::{
    Command, CommandKind, EntityId, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld,
};
use w3_render::{FrameInput, Model, ModelLibrary, Scene, TerrainMesh, TerrainVertex};

/// 带一个动画序列与一条平移轨道的最小模型（上游转换规格 JSON）。
const MODEL_DOC: &str = r#"{
    "bones": [{"name": "root"}],
    "sequences": [{"name": "stand", "start": 0, "end": 30, "fps": 30, "looping": true}],
    "tracks": [{"bone": 0, "times": [0, 30], "translations": [[0,0,0],[1,0,0]]}]
}"#;

fn build_match() -> RtsMatch {
    let mut sim = RtsMatch::create_with_world(
        MatchConfig {
            seed: 7,
            logic_fps: 30,
            content_hash: 0,
        },
        StaticWorld::open_world(8, 8, SimFixed::from_int(4).unwrap()),
    );
    for i in 0..4u32 {
        sim.submit_command(Command::new(
            Frame::new(0),
            (i % 2) as u8,
            i + 1,
            CommandKind::Spawn {
                x: SimFixed::from_int(2 + i64::from(i)).unwrap(),
                y: SimFixed::from_int(3).unwrap(),
                hp: 100,
                speed: SimFixed::from_int(3).unwrap(),
                attack_damage: 10,
                attack_range: SimFixed::from_int(10).unwrap(),
            },
        ))
        .unwrap();
    }
    sim
}

fn submit_move(sim: &mut RtsMatch) {
    sim.submit_command(Command::new(
        Frame::new(1),
        0,
        10,
        CommandKind::Move {
            entity: EntityId::new(1).unwrap(),
            x: SimFixed::from_int(20).unwrap(),
            y: SimFixed::from_int(20).unwrap(),
            group: 0,
            append: false,
        },
    ))
    .unwrap();
}

/// 手写一个最小地形，避免测试依赖本机导出资产。
fn simple_terrain() -> TerrainMesh {
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

#[test]
fn presentation_consumption_does_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);

    let mut scene = Scene::default();
    // 空模型库：所有单位模型资源缺失，降级为不显示。
    let library = ModelLibrary::default();
    let terrain = simple_terrain();

    for _ in 0..48 {
        rendered.step();
        plain.step();

        let view_a = rendered.read_view();
        let events_a = rendered.drain_events();
        let view_b = plain.read_view();
        let events_b = plain.drain_events();

        // 只有 rendered 一侧跑表现层：消费、插值、生成 GPU 顶点。
        scene.consume(
            FrameInput {
                view: &view_a,
                events: &events_a,
            },
            &library,
        );
        scene.interpolate_all(0.5);
        let vertices = scene.build_vertices(&terrain);
        assert_eq!(
            vertices.len(),
            terrain.indices.len(),
            "模型缺失时不应生成单位标记"
        );

        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "表现层消费改变了状态哈希"
        );
        assert_eq!(view_a, view_b);
        assert_eq!(events_a, events_b);
    }
}

#[test]
fn animation_sampling_with_dropped_frames_does_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    let mut library = ModelLibrary::default();
    library.insert("default", Model::from_json(MODEL_DOC).unwrap());
    let mut scene = Scene::default();

    for frame in 0..48u32 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let events = rendered.drain_events();
        let _ = plain.drain_events();

        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        // 表现层动画故意丢帧：每 3 帧才采样一次骨骼。
        if frame % 3 == 0 {
            let matrices = library
                .get("default")
                .unwrap()
                .sample(0, frame as f32 / 30.0);
            assert_eq!(matrices.len(), 1);
        }
        scene.interpolate_all((frame % 10) as f32 / 10.0);

        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "动画采样/丢帧改变了状态哈希"
        );
    }
}

#[test]
fn duplicate_event_delivery_does_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    let mut scene = Scene::default();
    let library = ModelLibrary::default();

    rendered.step();
    plain.step();
    let view = rendered.read_view();
    let events = rendered.drain_events();
    let _ = plain.drain_events();

    // 同一批事实事件重复投递（模拟重传 / 重复 drain）。
    for _ in 0..5 {
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
    }
    assert_eq!(
        scene.applied_events() as usize,
        events.len(),
        "重复事件应被 sequence 去重"
    );
    assert_eq!(
        rendered.compute_state_hash(),
        plain.compute_state_hash(),
        "事件去重改变了状态哈希"
    );
}
