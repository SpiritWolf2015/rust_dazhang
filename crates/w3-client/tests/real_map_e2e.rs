//! 真实导出地图的端到端 CPU 自测（真实地图阶段）。
//!
//! 本机存在 `config.toml` 指向的 Echo Isles 导出目录时，断言真实数据装配：
//! 12513 地形顶点 / 97 单位 + 2491 装饰物摆放 / 6 种地表分区 / 静态摆放
//! 进入 GPU 顶点流，且全部是表现层数据，`w3-core` 权威状态零改动。
//!
//! CI / 无本机导出资产时自动跳过（不依赖本机资产、不开 GPU 窗口）。
//! 跳过原因以英文结构化 `key=value` 打印，便于排查。

use std::path::Path;

use w3_client::real_map::{RealMap, assemble_from, load_paths};
use w3_core::SNAPSHOT_VERSION;

/// 真实地图存在时返回装配产物；不存在则返回 `None`（测试跳过）。
fn local_real_map() -> Option<RealMap> {
    let config = Path::new("config.toml");
    if !config.exists() {
        eprintln!("skip=real_map_e2e reason=no_local_config");
        return None;
    }
    let paths = match load_paths(config) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("skip=real_map_e2e reason=invalid_config error={error}");
            return None;
        }
    };
    if !paths.map_dir.join("terrain-heightfield.json").exists() {
        eprintln!(
            "skip=real_map_e2e reason=no_export_data dir={}",
            paths.map_dir.display()
        );
        return None;
    }
    let parsed = w3_content::load_map(&paths.map_dir).expect("real export must load");
    Some(assemble_from(&parsed).expect("assemble_from real export"))
}

#[test]
fn real_export_assembles_true_echo_isles_topology() {
    let Some(map) = local_real_map() else {
        return;
    };
    // 真实 Echo Isles：129×97 = 12513 顶点，97 单位 + 2491 装饰物 = 2588 摆放。
    assert_eq!(map.terrain.vertices.len(), 12513);
    assert_eq!(map.terrain.columns, 129);
    assert_eq!(map.terrain.rows, 97);
    assert_eq!(map.markers.len(), 97 + 2491);

    // 6 种 Lordaeron Summer 地表分区都出现，且保留水体 / 悬崖层数据。
    let mut surfaces = std::collections::BTreeSet::new();
    let mut has_water = false;
    let mut has_cliff = false;
    for vertex in &map.terrain.vertices {
        surfaces.insert(vertex.surface as i64);
        if vertex.water > vertex.position[1] {
            has_water = true;
        }
        if vertex.cliff < 15.0 {
            has_cliff = true;
        }
    }
    assert_eq!(surfaces.len(), 6, "6 种地表分区");
    assert!(has_water, "真实地图应含水体");
    assert!(has_cliff, "真实地图应含悬崖层");

    // 静态摆放坐标落在缩放后的地形范围内。
    for marker in &map.markers {
        assert!(marker.x.is_finite() && marker.y.is_finite());
        assert!((-140.0..=140.0).contains(&marker.x));
        assert!((-110.0..=110.0).contains(&marker.y));
    }
}

#[test]
fn static_markers_enter_gpu_vertices() {
    use w3_render::Scene;

    let Some(map) = local_real_map() else {
        return;
    };
    let mut scene = Scene::default();
    scene.set_static_markers(map.markers.clone());
    let vertices = scene.build_vertices(&map.terrain);
    assert_eq!(scene.static_marker_count(), 2588);
    assert!(
        vertices.len() > map.terrain.indices.len(),
        "静态摆放必须追加到地形顶点之后"
    );
    // 确定性：同一装配两次生成完全相同的顶点流（无随机源 / 无墙钟）。
    let again = scene.build_vertices(&map.terrain);
    assert_eq!(vertices, again);
    assert_eq!(SNAPSHOT_VERSION, 3, "真实地图装配不触碰快照版本");
}
