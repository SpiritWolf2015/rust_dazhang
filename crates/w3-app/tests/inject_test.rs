//! app 层端到端注入测试：合成 fixture，不含任何真实导出数据。

use w3_app::{create_match, freeze_f64, inject_map};
use w3_content::{ParsedMap, TerrainHeightfield, UnitsFile};
use w3_core::{Command, CommandKind, EntityId, Frame, GridCell, RtsMatch, SimFixed};

fn synthetic_map() -> ParsedMap {
    let heightfield: TerrainHeightfield = serde_json::from_str(
        r#"{
            "tilepointWidth": 3, "tilepointHeight": 3,
            "mapWidth": 2, "mapHeight": 2,
            "tileSize": 1, "centerOffset": {"x": 0, "y": 0},
            "heights": [0,0,0,0,1,1,0,0,0]
        }"#,
    )
    .unwrap();
    let units: UnitsFile = serde_json::from_str(
        r#"{
            "count": 2,
            "units": [
                {"typeId":"AAAA","position":{"x":1.0,"y":1.0,"z":0.0},"owner":1},
                {"typeId":"BBBB","position":{"x":2.5,"y":0.5,"z":0.0},"owner":2}
            ]
        }"#,
    )
    .unwrap();
    ParsedMap {
        heightfield,
        units: units.units,
        doodads: Vec::new(),
    }
}

/// 形状与 Echo Isles 头部同量级：tileSize=128、centerOffset=(-8192,-6144)。
fn echo_isles_shape_map() -> ParsedMap {
    let heightfield: TerrainHeightfield = serde_json::from_str(
        r#"{
            "tilepointWidth": 5, "tilepointHeight": 5,
            "mapWidth": 4, "mapHeight": 4,
            "tileSize": 128, "centerOffset": {"x": -8192, "y": -6144},
            "heights": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
        }"#,
    )
    .unwrap();
    let units: UnitsFile = serde_json::from_str(
        r#"{
            "count": 1,
            "units": [{"typeId":"AAAA","position":{"x":-7936.0,"y":-5888.0,"z":0.0},"owner":1}]
        }"#,
    )
    .unwrap();
    ParsedMap {
        heightfield,
        units: units.units,
        doodads: Vec::new(),
    }
}

#[test]
fn inject_synthetic_map_and_run_frames() {
    let map = synthetic_map();
    let injection = inject_map(&map).unwrap();
    assert_eq!(injection.world.width(), 2);
    assert_eq!(injection.world.height(), 2);
    assert_eq!(injection.world.cell_size(), SimFixed::ONE);
    assert_eq!(injection.spawn_commands.len(), 2);
    assert_eq!(injection.content_hash, injection.world.identity());

    let mut sim = create_match(&injection, 7);
    sim.step(); // 第 0 帧执行出生命令。
    let view = sim.read_view();
    assert_eq!(view.entities.len(), 2);
    assert_eq!(view.entities[0].x, freeze_f64(1.0).unwrap().raw());
    assert_eq!(view.entities[1].x, freeze_f64(2.5).unwrap().raw());

    // 注入的世界身份进入快照，恢复必须接受同一世界。
    let snapshot = sim.capture_snapshot();
    let restored =
        RtsMatch::restore_snapshot(snapshot, injection.world.clone(), injection.content_hash)
            .unwrap();
    assert_eq!(sim.compute_state_hash(), restored.compute_state_hash());
}

#[test]
fn injection_is_deterministic() {
    let a = inject_map(&synthetic_map()).unwrap();
    let b = inject_map(&synthetic_map()).unwrap();
    assert_eq!(a.content_hash, b.content_hash);
    assert_eq!(a.spawn_commands, b.spawn_commands);
}

#[test]
fn invalid_heightfield_is_rejected() {
    let mut map = synthetic_map();
    map.heightfield.heights.truncate(2);
    assert!(inject_map(&map).is_err());
}

#[test]
fn echo_isles_header_drives_cell_size_and_origin() {
    let injection = inject_map(&echo_isles_shape_map()).unwrap();
    assert_eq!(injection.world.width(), 4);
    assert_eq!(injection.world.height(), 4);
    assert_eq!(injection.world.cell_size(), freeze_f64(128.0).unwrap());
    assert_eq!(injection.world.origin().0, freeze_f64(-8192.0).unwrap());
    assert_eq!(injection.world.origin().1, freeze_f64(-6144.0).unwrap());
    assert_eq!(injection.spawn_commands.len(), 1);

    let mut sim = create_match(&injection, 7);
    sim.step();
    let view = sim.read_view();
    assert_eq!(view.entities.len(), 1);
    // 单位世界坐标原样冻结；(x - origin)/cell_size = 256/128 = 2 → 格 (2,2)。
    let cell = injection.world.world_to_cell(
        SimFixed::from_raw(view.entities[0].x),
        SimFixed::from_raw(view.entities[0].y),
    );
    assert_eq!(cell, GridCell::new(2, 2));
}

#[test]
fn injected_world_movement_is_deterministic() {
    let injection = inject_map(&echo_isles_shape_map()).unwrap();
    let mut a = create_match(&injection, 7);
    let mut b = create_match(&injection, 7);
    // 目标为格 (3,3) 中心：origin + (3*128+64, 3*128+64)。
    let command = Command::new(
        Frame::new(1),
        1,
        2,
        CommandKind::Move {
            entity: EntityId::new(1).unwrap(),
            x: freeze_f64(-7744.0).unwrap(),
            y: freeze_f64(-5696.0).unwrap(),
            group: 0,
            append: false,
        },
    );
    a.submit_command(command).unwrap();
    b.submit_command(command).unwrap();
    // 第 0 帧执行出生，之后才可取到单位视图。
    a.step();
    b.step();
    let before = a.read_view().entities[0].x;
    for frame in 0..30 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "frame {frame} diverged on the injected world"
        );
    }
    let after = a.read_view();
    assert_eq!(after, b.read_view());
    assert_ne!(
        after.entities[0].x, before,
        "unit must actually move on the injected world"
    );
}

#[test]
fn non_positive_tile_size_is_rejected() {
    let mut map = synthetic_map();
    map.heightfield.tile_size = Some(0.0);
    assert!(matches!(
        inject_map(&map),
        Err(w3_app::InjectError::InvalidCellSize)
    ));
}

#[test]
fn slk_definitions_are_injected_and_settle_in_a_real_match() {
    use w3_app::{combine_content_hash, definition_id, inject_map_with_definitions};
    use w3_content::{DefinitionTables, SlkTable, from_slk_tables};
    use w3_core::{GameplayCommand, GameplayCommandKind};

    // 合成 SLK 表（不含任何真实导出数据）。
    let units: SlkTable = serde_json::from_str(
        r#"{
            "columns": 2, "rows": 1, "headers": ["unitID", "hitPoints"],
            "recordCount": 1,
            "records": [{"unitID": "H001", "hitPoints": 100, "manaPoints": 20}]
        }"#,
    )
    .unwrap();
    let abilities: SlkTable = serde_json::from_str(
        r#"{
            "columns": 3, "rows": 1, "headers": ["abilityID", "manaCost", "cooldown"],
            "recordCount": 1,
            "records": [{"abilityID": "A001", "manaCost": 5, "cooldown": 2}]
        }"#,
    )
    .unwrap();
    let content_defs = from_slk_tables(&DefinitionTables {
        units: Some(&units),
        abilities: Some(&abilities),
        ..Default::default()
    })
    .unwrap();

    let injection = inject_map_with_definitions(&synthetic_map(), &content_defs).unwrap();
    let expected = combine_content_hash(
        injection.world.identity(),
        injection.definitions.content_hash(),
    );
    assert_eq!(injection.content_hash, expected);
    assert_eq!(injection.definitions.units().len(), 1);
    assert_eq!(injection.definitions.abilities().len(), 1);

    let mut sim = create_match(&injection, 7);
    sim.step(); // 第 0 帧出生。
    let before = sim.capture_snapshot();
    // 冻结定义的内容哈希进入快照头，且定义确实来自 SLK 转换。
    assert_eq!(before.defs_hash, injection.definitions.content_hash());
    assert_eq!(before.defs.abilities().len(), 1);
    assert_eq!(sim.definitions().content_hash(), before.defs_hash);

    // 注入的技能在真实对局中可结算：施法后目标生命下降。
    let ability = definition_id("A001");
    let cast = GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::Cast {
            caster: EntityId::new(1).unwrap(),
            ability,
            target: EntityId::new(2).unwrap(),
        },
    );
    sim.submit_gameplay_command(cast).unwrap();
    sim.step(); // 第 1 帧执行施法。
    let after = sim.capture_snapshot();
    let target = after.positions.iter().find(|p| p.entity == 2).unwrap();
    assert!(target.hp < 100, "injected ability must settle damage");

    // 快照往返：内容身份同时锚定地图与定义，恢复逐帧哈希一致。
    let mut restored =
        RtsMatch::restore_snapshot(after, injection.world.clone(), injection.content_hash).unwrap();
    assert_eq!(sim.compute_state_hash(), restored.compute_state_hash());
    sim.step();
    restored.step();
    assert_eq!(sim.compute_state_hash(), restored.compute_state_hash());

    // 换一套内容定义（内容哈希变化）必须被恢复拒绝。
    let other_defs = from_slk_tables(&DefinitionTables {
        units: Some(&units),
        ..Default::default()
    })
    .unwrap();
    let other_injection = inject_map_with_definitions(&synthetic_map(), &other_defs).unwrap();
    assert_ne!(other_injection.content_hash, injection.content_hash);
    let mismatched = sim.capture_snapshot();
    assert!(
        RtsMatch::restore_snapshot(
            mismatched,
            injection.world.clone(),
            other_injection.content_hash,
        )
        .is_err()
    );
}
