//! P1 验收：读取本机导出的地图目录与一张 SLK 表。
//! 所有 fixture 均为合成数据，不提交任何游戏派生内容。

use std::path::PathBuf;

use w3_content::{ContentError, HeightfieldError, load_map, load_table};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn load_map_reads_terrain_and_placements() {
    let map = load_map(&fixture("minimal_map")).unwrap();

    assert_eq!(map.heightfield.tilepoint_width, 3);
    assert_eq!(map.heightfield.tilepoint_height, 3);
    assert_eq!(map.heightfield.map_width, 2);
    assert_eq!(map.heightfield.vertex_count(), Some(9));
    assert_eq!(map.heightfield.height_at(0, 0), Some(0.0));
    assert_eq!(map.heightfield.height_at(2, 1), Some(5.0));
    assert_eq!(map.heightfield.main_tileset.as_deref(), Some("Z"));
    assert_eq!(map.heightfield.ground_tilesets, vec!["g0", "g1"]);
    assert_eq!(map.heightfield.ground_textures.len(), 9);

    assert_eq!(map.units.len(), 2);
    assert_eq!(map.units[0].type_id, "AAAA");
    assert_eq!(map.units[0].position.x, 128.0);
    assert_eq!(map.units[0].owner, 0);
    assert_eq!(map.units[1].type_id, "BBBB");
    assert_eq!(map.units[1].position.y, 512.25);
    assert_eq!(map.units[1].owner, 1);

    assert_eq!(map.doodads.len(), 1);
    assert_eq!(map.doodads[0].id, "DDDD");
    assert_eq!(map.doodads[0].creation_number, 7);
}

#[test]
fn optional_placement_files_may_be_missing() {
    let map = load_map(&fixture("terrain_only")).unwrap();
    assert_eq!(map.heightfield.vertex_count(), Some(4));
    assert!(map.units.is_empty());
    assert!(map.doodads.is_empty());
}

#[test]
fn invalid_heightfield_is_rejected_at_the_boundary() {
    let error = load_map(&fixture("invalid_map")).unwrap_err();
    assert!(matches!(
        error,
        ContentError::Heightfield(HeightfieldError::WrongArrayLength { .. })
    ));
}

#[test]
fn missing_heightfield_is_reported() {
    let error = load_map(&fixture("does_not_exist")).unwrap_err();
    assert!(matches!(error, ContentError::MissingHeightfield(_)));
}

#[test]
fn slk_table_loads_records() {
    let table = load_table(&fixture("tables/units.json")).unwrap();
    assert_eq!(table.headers, vec!["unitID", "Name", "hitPoints"]);
    assert_eq!(table.records.len(), 2);
    let row = table.find_row("unitID", "BBBB").unwrap();
    assert_eq!(
        row.get("hitPoints").and_then(serde_json::Value::as_i64),
        Some(200)
    );
    assert!(table.find_row("unitID", "ZZZZ").is_none());
}

#[test]
fn missing_table_is_reported() {
    let error = load_table(&fixture("tables/nope.json")).unwrap_err();
    assert!(matches!(error, ContentError::MissingTable(_)));
}
