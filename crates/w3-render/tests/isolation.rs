//! 确定性隔离验收（`3d-rendering` spec）。
//!
//! 表现层消费只读视图、模型加载失败、动画丢帧、位置插值与事件重复投递，
//! 都 MUST NOT 改变 `RtsMatch` 的权威状态哈希。测试建立两个完全相同的
//! 对局，只有一个额外跑完整表现层管线，逐帧比对 `compute_state_hash()`
//! 与 `read_view()`。

use w3_core::{
    Command, CommandKind, EntityId, Event, EventKind, Frame, MatchConfig, RtsMatch,
    SNAPSHOT_VERSION, SimFixed, StaticWorld,
};
use w3_render::{
    FrameInput, Model, ModelLibrary, RenderFogVisibility, Scene, TerrainMesh, TerrainVertex,
};

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

/// P8.5a：英雄阵亡 / 复活完成事实事件是纯表现层副作用。
///
/// `HeroDied` 切死亡序列、`Revived` 切站立序列，找不到序列时 no-op；
/// 连续重绘、重复投递、事件重复消费都 MUST NOT 反写权威状态、不改
/// `compute_state_hash()` / `capture_snapshot()` 字节（ADR 0002 §5 / tasks 6.1）。
#[test]
fn hero_revive_events_do_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);
    let mut library = ModelLibrary::default();
    library.insert("default", Model::from_json(MODEL_DOC).unwrap());
    let mut scene = Scene::default();
    let terrain = simple_terrain();

    for _ in 0..16 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let _events = rendered.drain_events();
        let _ = plain.drain_events();

        // 只读视图先建实例；随后投递英雄阵亡 / 复活完成事实事件，
        // 并重复投递同一批次（幂等：按 sequence 去重）。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let revive_events = vec![
            Event {
                frame: view.tick,
                sequence: 10_001,
                entity: 1,
                kind: EventKind::HeroDied,
            },
            Event {
                frame: view.tick,
                sequence: 10_002,
                entity: 1,
                kind: EventKind::Revived,
            },
        ];
        for _ in 0..3 {
            scene.consume(
                FrameInput {
                    view: &view,
                    events: &revive_events,
                },
                &library,
            );
        }
        scene.interpolate_all(0.5);
        let _vertices = scene.build_vertices(&terrain);

        assert_eq!(
            scene.applied_events(),
            revive_events.len() as u64,
            "英雄复活事实事件应被 sequence 去重"
        );
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "英雄复活事件消费改变了状态哈希"
        );
        assert_eq!(
            rendered.capture_snapshot(),
            plain.capture_snapshot(),
            "英雄复活事件消费改变了快照字节"
        );
        assert_eq!(view, plain.read_view());
    }
}

/// P8.5b：主城升级开始 / 完成 / 取消三个事实事件是纯表现层副作用。
///
/// 三个事件在 `apply_event` 里都是幂等 no-op（升级进度 / tier / 费用由
/// `w3-core` 权威状态写入，HUD 只读派生），重复投递按 sequence 去重；
/// 连续重绘、事件重复消费 MUST NOT 反写权威状态、不改
/// `compute_state_hash()` / `capture_snapshot()` 字节（ADR 0002 §5 / tasks 6.1）。
#[test]
fn town_hall_upgrade_events_do_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);
    let mut library = ModelLibrary::default();
    library.insert("default", Model::from_json(MODEL_DOC).unwrap());
    let mut scene = Scene::default();
    let terrain = simple_terrain();

    for _ in 0..16 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let _events = rendered.drain_events();
        let _ = plain.drain_events();

        // 只读视图先建实例；随后投递三个主城升级事实事件，并重复投递
        // 同一批次（幂等：按 sequence 去重）。三个分支均为纯表现层 no-op。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let upgrade_events = vec![
            Event {
                frame: view.tick,
                sequence: 20_001,
                entity: 1,
                kind: EventKind::TownHallUpgradeStarted,
            },
            Event {
                frame: view.tick,
                sequence: 20_002,
                entity: 1,
                kind: EventKind::TownHallUpgradeCompleted,
            },
            Event {
                frame: view.tick,
                sequence: 20_003,
                entity: 1,
                kind: EventKind::TownHallUpgradeCancelled,
            },
        ];
        for _ in 0..3 {
            scene.consume(
                FrameInput {
                    view: &view,
                    events: &upgrade_events,
                },
                &library,
            );
        }
        scene.interpolate_all(0.5);
        let _vertices = scene.build_vertices(&terrain);

        assert_eq!(
            scene.applied_events(),
            upgrade_events.len() as u64,
            "主城升级事实事件应被 sequence 去重"
        );
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "主城升级事件消费改变了状态哈希"
        );
        assert_eq!(
            rendered.capture_snapshot(),
            plain.capture_snapshot(),
            "主城升级事件消费改变了快照字节"
        );
        assert_eq!(view, plain.read_view());
    }
}

/// P8.5c：野怪营地 / leash / 死亡 / 掉落六类事实事件是纯表现层副作用。
///
/// 六个事件在 `apply_event` 里都是幂等 no-op（营地 / 掉落 / 收益 / 随机码
/// 由 `w3-core` 权威状态写入，HUD 只读派生），重复投递按 sequence 去重；
/// 连续 16 帧重绘、事件重复消费 MUST NOT 反写权威状态、不改
/// `compute_state_hash()` / `capture_snapshot()` 字节（ADR 0002 §5 / tasks 7.1）。
#[test]
fn creep_camp_events_do_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);
    let mut library = ModelLibrary::default();
    library.insert("default", Model::from_json(MODEL_DOC).unwrap());
    let mut scene = Scene::default();
    let terrain = simple_terrain();

    for _ in 0..16 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let _events = rendered.drain_events();
        let _ = plain.drain_events();

        // 只读视图先建实例；随后投递六类野怪事实事件，并重复投递同一批次
        // （幂等：按 sequence 去重）。六个分支均为纯表现层 no-op。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let creep_events = vec![
            Event {
                frame: view.tick,
                sequence: 30_001,
                entity: 1,
                kind: EventKind::CreepCampSeeded,
            },
            Event {
                frame: view.tick,
                sequence: 30_002,
                entity: 1,
                kind: EventKind::CreepEngaged,
            },
            Event {
                frame: view.tick,
                sequence: 30_003,
                entity: 1,
                kind: EventKind::CreepLeashReturning,
            },
            Event {
                frame: view.tick,
                sequence: 30_004,
                entity: 1,
                kind: EventKind::CreepLeashReturned,
            },
            Event {
                frame: view.tick,
                sequence: 30_005,
                entity: 1,
                kind: EventKind::CreepDied,
            },
            Event {
                frame: view.tick,
                sequence: 30_006,
                entity: 1,
                kind: EventKind::ItemDropped,
            },
        ];
        for _ in 0..3 {
            scene.consume(
                FrameInput {
                    view: &view,
                    events: &creep_events,
                },
                &library,
            );
        }
        scene.interpolate_all(0.5);
        let _vertices = scene.build_vertices(&terrain);

        assert_eq!(
            scene.applied_events(),
            creep_events.len() as u64,
            "reason=creep_events_are_deduplicated"
        );
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "reason=creep_event_consumption_keeps_state_hash"
        );
        assert_eq!(
            rendered.capture_snapshot(),
            plain.capture_snapshot(),
            "reason=creep_event_consumption_keeps_snapshot"
        );
        assert_eq!(
            view,
            plain.read_view(),
            "reason=creep_event_consumption_keeps_read_view"
        );
    }
}

/// P8.5d：物品 / 商店 / 回城十一类事实事件是纯表现层副作用。
///
/// 十一个事件在 `apply_event` 里都是幂等 no-op（六格背包 / 货架库存 /
/// 回城施法 / 市场刷新 / 消费序列由 `w3-core` 权威状态写入，HUD 只读派生），
/// 重复投递按 sequence 去重；连续 16 帧重绘、插值与事件重复消费 MUST NOT
/// 反写权威状态、不改 `compute_state_hash()` / `capture_snapshot()` 字节，
/// 快照版本仍为 v9（ADR 0002 §5 / tasks 6.2 / R10.5）。
#[test]
fn item_shop_events_do_not_change_state_hash() {
    assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);
    let mut library = ModelLibrary::default();
    library.insert("default", Model::from_json(MODEL_DOC).unwrap());
    let mut scene = Scene::default();
    let terrain = simple_terrain();

    for _ in 0..16 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let _events = rendered.drain_events();
        let _ = plain.drain_events();

        // 只读视图先建实例；随后投递十一类物品 / 商店 / 回城事实事件，
        // 并重复投递同一批次（幂等：按 sequence 去重）。
        scene.consume(
            FrameInput {
                view: &view,
                events: &[],
            },
            &library,
        );
        let item_shop_events = vec![
            Event {
                frame: view.tick,
                sequence: 40_001,
                entity: 1,
                kind: EventKind::ItemPickedUp,
            },
            Event {
                frame: view.tick,
                sequence: 40_002,
                entity: 1,
                kind: EventKind::ItemPurchased,
            },
            Event {
                frame: view.tick,
                sequence: 40_003,
                entity: 1,
                kind: EventKind::ItemUsed,
            },
            Event {
                frame: view.tick,
                sequence: 40_004,
                entity: 1,
                kind: EventKind::ItemDiscarded,
            },
            Event {
                frame: view.tick,
                sequence: 40_005,
                entity: 1,
                kind: EventKind::ItemTransferred,
            },
            Event {
                frame: view.tick,
                sequence: 40_006,
                entity: 1,
                kind: EventKind::ItemSold,
            },
            Event {
                frame: view.tick,
                sequence: 40_007,
                entity: 1,
                kind: EventKind::TownPortalStarted,
            },
            Event {
                frame: view.tick,
                sequence: 40_008,
                entity: 1,
                kind: EventKind::TownPortalCompleted,
            },
            Event {
                frame: view.tick,
                sequence: 40_009,
                entity: 1,
                kind: EventKind::TownPortalCancelled,
            },
            Event {
                frame: view.tick,
                sequence: 40_010,
                entity: 1,
                kind: EventKind::MarketRefreshed,
            },
            Event {
                frame: view.tick,
                sequence: 40_011,
                entity: 1,
                kind: EventKind::ItemShopCommandRejected,
            },
        ];
        for _ in 0..3 {
            scene.consume(
                FrameInput {
                    view: &view,
                    events: &item_shop_events,
                },
                &library,
            );
        }
        scene.interpolate_all(0.5);
        let _vertices = scene.build_vertices(&terrain);

        assert_eq!(
            scene.applied_events(),
            item_shop_events.len() as u64,
            "reason=item_shop_events_are_deduplicated"
        );
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "reason=item_shop_event_consumption_keeps_state_hash"
        );
        assert_eq!(
            rendered.capture_snapshot(),
            plain.capture_snapshot(),
            "reason=item_shop_event_consumption_keeps_snapshot"
        );
        assert_eq!(
            rendered.capture_snapshot().version,
            SNAPSHOT_VERSION,
            "reason=item_shop_snapshot_stays_v9"
        );
        assert_eq!(
            view,
            plain.read_view(),
            "reason=item_shop_event_consumption_keeps_read_view"
        );
    }
}

/// P8.5e 战争迷雾：可见性过滤 / 重复事实事件投递 / 多帧重绘只读隔离。
///
/// 本测试不新增任何 `EventKind`：重复投递 P8.5c / P8.5d 既有事实事件，
/// 并在 `Scene` 上切换只读迷雾可见性；MUST NOT 改变内核哈希、v9 快照、
/// `read_view()` / `drain_events()` 输出或 `first_difference()`。
#[test]
fn fog_of_war_events_do_not_change_state_hash() {
    let mut rendered = build_match();
    let mut plain = build_match();
    submit_move(&mut rendered);
    submit_move(&mut plain);
    let mut scene = Scene::default();
    let library = ModelLibrary::default();
    let terrain = simple_terrain();
    let fog_events = vec![
        Event {
            frame: 0,
            sequence: 50_001,
            entity: 1,
            kind: EventKind::CreepCampSeeded,
        },
        Event {
            frame: 0,
            sequence: 50_002,
            entity: 1,
            kind: EventKind::ItemDropped,
        },
        Event {
            frame: 0,
            sequence: 50_003,
            entity: 1,
            kind: EventKind::MarketRefreshed,
        },
    ];
    let snapshot_bytes = |snapshot: &w3_core::Snapshot| format!("{snapshot:?}").into_bytes();

    for _ in 0..16 {
        rendered.step();
        plain.step();
        let view_a = rendered.read_view();
        let events_a = rendered.drain_events();
        let view_b = plain.read_view();
        let events_b = plain.drain_events();
        let visibility = RenderFogVisibility {
            visible_entity_ids: view_a
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.owner == 0)
                .map(|entity| entity.id)
                .collect(),
            remembered_entity_ids: view_a
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.owner == 1)
                .map(|entity| entity.id)
                .collect(),
        };
        scene.set_fog_visibility(Some(visibility));
        for _ in 0..3 {
            scene.consume(
                FrameInput {
                    view: &view_a,
                    events: &fog_events,
                },
                &library,
            );
        }
        scene.interpolate_all(0.5);
        let _vertices = scene.build_vertices(&terrain);
        assert_eq!(
            scene.applied_events(),
            fog_events.len() as u64,
            "reason=fog_related_events_are_deduplicated"
        );
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "reason=fog_visibility_keeps_state_hash"
        );
        assert_eq!(
            rendered.capture_snapshot(),
            plain.capture_snapshot(),
            "reason=fog_visibility_keeps_snapshot"
        );
        assert_eq!(
            snapshot_bytes(&rendered.capture_snapshot()),
            snapshot_bytes(&plain.capture_snapshot()),
            "reason=fog_visibility_keeps_snapshot_bytes"
        );
        assert_eq!(
            rendered.first_difference(&plain),
            None,
            "reason=fog_visibility_keeps_first_difference_none"
        );
        assert_eq!(view_a, view_b, "reason=fog_visibility_keeps_read_view");
        assert_eq!(
            events_a, events_b,
            "reason=fog_visibility_keeps_drained_events"
        );
        assert_eq!(
            rendered.capture_snapshot().version,
            SNAPSHOT_VERSION,
            "reason=fog_snapshot_stays_v9"
        );
        assert_eq!(SNAPSHOT_VERSION, 9, "reason=snapshot_version_9");
    }
}
