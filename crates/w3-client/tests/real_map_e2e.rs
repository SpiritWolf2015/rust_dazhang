//! 真实导出地图的端到端 CPU 自测（真实地图阶段）。
//!
//! 本机存在 `config.toml` 指向的 Echo Isles 导出目录时，断言真实数据装配：
//! 12513 地形顶点 / 97 单位 + 2491 装饰物摆放 / 6 种地表分区 / 静态摆放
//! 进入 GPU 顶点流，且全部是表现层数据，`w3-core` 权威状态零改动。
//!
//! CI / 无本机导出资产时自动跳过（不依赖本机资产、不开 GPU 窗口）。
//! 跳过原因以英文结构化 `key=value` 打印，便于排查。

use std::path::{Path, PathBuf};

use w3_client::real_map::{RealMap, assemble, assemble_from, load_paths};
use w3_core::SNAPSHOT_VERSION;

/// gitignored `config.toml` 位于 workspace 根；`cargo test` 的工作目录是
/// crate 根，用编译期 `CARGO_MANIFEST_DIR` 定位，避免真实资产测试静默跳过。
fn config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config.toml")
}

/// 真实地图存在时返回装配产物；不存在则返回 `None`（测试跳过）。
fn local_real_map() -> Option<RealMap> {
    let config = config_path();
    if !config.exists() {
        eprintln!("skip=real_map_e2e reason=no_local_config");
        return None;
    }
    let paths = match load_paths(&config) {
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

/// 真实贴图资产存在时返回完整装配产物；否则跳过（CI 无本机资产）。
fn local_full_map() -> Option<RealMap> {
    let config = config_path();
    if !config.exists() {
        eprintln!("skip=real_map_textures reason=no_local_config");
        return None;
    }
    let paths = load_paths(&config).ok()?;
    if paths.tileset_dir.is_none() || paths.minimap_image.is_none() {
        eprintln!("skip=real_map_textures reason=no_texture_config");
        return None;
    }
    let (map, _) = assemble(&config).expect("full real map must assemble");
    Some(map)
}

#[test]
fn real_tileset_feeds_scene_vertex_colors() {
    use w3_render::Scene;

    let Some(map) = local_full_map() else {
        return;
    };
    let tiles = map.terrain_tiles.as_ref().expect("6 tileset pngs loaded");
    assert_eq!(tiles.len(), 6, "真实地表贴图必须 6 层");
    let (w, h) = (tiles[0].width, tiles[0].height);
    assert!(w > 0 && h > 0);
    for tile in tiles {
        assert_eq!((tile.width, tile.height), (w, h), "tileset 尺寸一致");
        assert_eq!(tile.pixels.len(), (w * h * 4) as usize, "RGBA8 长度一致");
    }

    // 真实贴图颜色确实改变地形顶点色（而非回退的 6 色调色板色块）。
    let mut scene = Scene::default();
    let palette = scene.build_vertices(&map.terrain);
    scene.set_terrain_tiles(map.terrain_tiles.clone());
    let textured = scene.build_vertices(&map.terrain);
    assert_eq!(palette.len(), textured.len());
    let changed = palette
        .iter()
        .zip(&textured)
        .filter(|(a, b)| a.surface >= 0.0 && a.color != b.color)
        .count();
    assert!(changed > 0, "真实贴图采样必须体现在地形顶点色");
    // 确定性：同一贴图两次生成完全一致的顶点流。
    assert_eq!(textured, scene.build_vertices(&map.terrain));
}

#[test]
fn real_minimap_decodes_into_nonflat_hud_backdrop() {
    let Some(map) = local_full_map() else {
        return;
    };
    let image = map.minimap_image.as_ref().expect("minimap png loaded");
    assert!(image.width > 0 && image.height > 0);
    let backdrop = w3_client::hud::minimap_backdrop(image, 32);
    assert_eq!(backdrop.grid, 32);
    assert_eq!(backdrop.colors.len(), 32 * 32);
    assert!(
        backdrop
            .colors
            .iter()
            .all(|c| c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)))
    );
    // 真实小地图不是单色纯色块：至少呈现多种颜色。
    let mut seen = std::collections::BTreeSet::new();
    for color in &backdrop.colors {
        seen.insert(color.map(|c| (c * 256.0) as i64));
    }
    assert!(seen.len() > 4, "真实小地图底图应含多种颜色");
}

/// 真实模型资产存在时返回完整装配产物；否则跳过（CI 无本机资产）。
fn local_model_paths() -> Option<w3_client::real_map::RealMapPaths> {
    let config = config_path();
    if !config.exists() {
        eprintln!("skip=real_map_models reason=no_local_config");
        return None;
    }
    let paths = load_paths(&config).ok()?;
    if paths.models_dir.is_none() || paths.slk_dir.is_none() {
        eprintln!("skip=real_map_models reason=no_model_config");
        return None;
    }
    Some(paths)
}

fn local_model_map() -> Option<RealMap> {
    local_model_paths()?;
    let (map, _) = assemble(&config_path()).expect("full real map with models must assemble");
    Some(map)
}

#[test]
fn real_gltf_models_replace_all_programmatic_geometry() {
    let Some(map) = local_model_map() else {
        return;
    };
    let stats = &map.model_stats;
    // 真实 Echo Isles：97 单位 + 2491 装饰物全部解析成真实 glTF 模型。
    assert_eq!(stats.placements, 97 + 2491, "全部摆放都参与模型解析");
    assert_eq!(stats.skipped, 0, "所有摆放都应命中真实模型，无几何体残留");
    assert_eq!(stats.budget_truncated, 0);
    assert_eq!(stats.resolved, stats.placements);

    // 顶点流来自真实网格（远超单个程序化标记的 12 顶点），且颜色取自真实贴图。
    assert!(
        map.static_model_vertices.len() > 100_000,
        "真实模型顶点数应远大于程序化几何，实际 {}",
        map.static_model_vertices.len()
    );
    assert!(
        map.static_model_vertices
            .iter()
            .all(|v| v.surface < 0.0 && v.position.iter().all(|c| c.is_finite()))
    );
    // 真 GPU 纹理管线：顶点 color 是调制色，真实贴图颜色来自统一数组纹理。
    // 断言数组纹理非空且像素颜色丰富（贴图确实被收集、非纯色块）。
    let mut colors = std::collections::BTreeSet::new();
    for px in map.textures.pixels.chunks_exact(4) {
        colors.insert([px[0], px[1], px[2]]);
    }
    assert!(!map.textures.is_empty(), "应收集到统一贴图数组");
    assert!(colors.len() > 100, "真实贴图数组应带来丰富颜色，非纯色块");
    // 静态顶点应含 GPU 纹理模式（tex_mode>0 且携带有效层）。
    let textured = map
        .static_model_vertices
        .iter()
        .filter(|v| v.tex_mode > 0.0 && v.tex_layer >= 0.0)
        .count();
    assert!(textured > 0, "真实模型顶点应走 GPU 纹理采样路径");

    // 6 个动态实体类型键全部绑定真实网格。
    assert_eq!(map.entity_models.len(), 6);
    for (key, model) in &map.entity_models {
        assert!(model.triangle_count() > 0, "实体 {key} 模型不能为空");
    }
    // 建筑 / 单位模型比程序化四面体复杂得多。
    assert!(map.entity_models["town-hall"].triangle_count() > 500);
    assert!(map.entity_models["barracks"].triangle_count() > 500);
}

#[test]
fn real_townhall_filters_effects_and_keeps_entity_geometry() {
    // 直接验证真实主城 glTF：特效层 / 地面 UberSplat 被过滤，实体几何保留。
    let Some(paths) = local_model_paths() else {
        return;
    };
    let Some(models_dir) = paths.models_dir.as_ref() else {
        return;
    };
    let townhall = models_dir.join("buildings/Human/TownHall/TownHall.gltf");
    if !townhall.exists() {
        eprintln!("skip=real_townhall_effects reason=no_townhall_model");
        return;
    }
    let mesh = w3_content::load_gltf(&townhall).expect("TownHall.gltf loads");
    assert!(!mesh.submeshes.is_empty());
    // 至少有一个特效层和一个 GroundDecal（TownHall 含 UberSplat / 辉光）。
    let effects = mesh
        .submeshes
        .iter()
        .filter(|s| !s.filter_mode.is_entity())
        .count();
    assert!(effects > 0, "主城应含被识别的特效 / 地面贴片层");
    let ground_decals = mesh
        .submeshes
        .iter()
        .filter(|s| s.filter_mode == w3_content::GeosetFilter::GroundDecal)
        .count();
    assert!(ground_decals > 0, "UberSplat 应被识别为 GroundDecal");
    // 烘焙后只保留实体层三角形（少于原始总数），且非空。
    let baked = w3_render::bake_model(&mesh);
    assert!(!baked.is_empty(), "实体几何不能全被剔除");
    assert!(
        baked.triangle_count() < mesh.triangle_count(),
        "烘焙应剔除特效层三角形"
    );
    // 顶点色不出现纯白异常面片（特效层已过滤，实体贴图着色）。
    assert!(
        baked
            .triangles
            .iter()
            .all(|t| t.color.iter().all(|c| c.iter().all(|v| v.is_finite())))
    );
}

#[test]
fn real_models_enter_scene_vertices_and_are_deterministic() {
    use w3_render::Scene;

    let Some(map) = local_model_map() else {
        return;
    };
    let mut scene = Scene::default();
    scene.set_static_markers(map.markers.clone());
    let programmatic = scene.build_vertices(&map.terrain);
    let programmatic_markers = programmatic.iter().filter(|v| v.surface < 0.0).count();

    scene.set_static_model_vertices(map.static_model_vertices.clone());
    let with_models = scene.build_vertices(&map.terrain);
    let model_markers = with_models.iter().filter(|v| v.surface < 0.0).count();
    // 程序化摆放被真实网格整体替换：摆放顶点数等于真实模型预展开顶点数。
    assert_eq!(model_markers, map.static_model_vertices.len());
    assert!(
        model_markers > programmatic_markers * 3,
        "真实模型顶点应远多于程序化摆放，实际 {} vs {}",
        model_markers,
        programmatic_markers
    );
    assert_eq!(
        scene.static_model_vertex_count(),
        map.static_model_vertices.len()
    );
    // 确定性：同一真实模型流逐帧可复现（无随机源 / 无墙钟）。
    assert_eq!(with_models, scene.build_vertices(&map.terrain));
    assert_eq!(SNAPSHOT_VERSION, 3, "真实模型装配不触碰快照版本");
}

#[test]
fn real_gltf_models_render_on_screen_not_black() {
    use w3_render::{Camera, RenderError, render_headless_rgba_with_textures};

    let Some(map) = local_model_map() else {
        return;
    };

    // 真 GPU 纹理管线：地形与模型顶点都携带 tex_layer，必须上传真实
    // `texture_2d_array`（`Some(&map.textures)`）由片元着色器逐像素采样；
    // 传 `None` 会用 1 层纯白占位纹理，越界层被 clamp 成白色、模型失去贴图
    // 颜色。故 A/B 都注入 `terrain_texture_layers` 并传真实贴图数组。
    let layers = map.terrain_texture_layers;

    // A/B 对照：A = 仅地形（真实地表贴图、无模型）；B = 地形 + 真实 glTF
    // 模型。只有「B 相对 A 出现大量像素变化」才能证明**真实模型确实上屏**，
    // 而非地形自身满足非黑屏 / 颜色丰富（critic B2）。
    let mut terrain_only = w3_render::Scene::default();
    terrain_only.set_terrain_tiles(map.terrain_tiles.clone());
    terrain_only.set_terrain_texture_layers(layers);
    let vertices_a = terrain_only.build_vertices(&map.terrain);
    // A 里 surface>=0 全为地形，surface<0 为空（无静态摆放、无动态单位）。
    assert!(vertices_a.iter().all(|v| v.surface >= 0.0));

    let mut with_models = w3_render::Scene::default();
    with_models.set_terrain_tiles(map.terrain_tiles.clone());
    with_models.set_terrain_texture_layers(layers);
    with_models.set_static_model_vertices(map.static_model_vertices.clone());
    let vertices_b = with_models.build_vertices(&map.terrain);
    assert!(vertices_b.len() > vertices_a.len());
    // B 必须确实含真实模型顶点（surface<0），否则 A/B 差异无从归因于模型。
    assert!(
        vertices_b.iter().any(|v| v.surface < 0.0),
        "B 应含真实模型顶点"
    );

    let view_proj = Camera::new(1.0).matrix(&map.terrain);
    let width = 160u32;
    let height = 120u32;
    // 先渲 A：无 adapter 则整体如实跳过（不冒充通过）。
    let rgba_a = match render_headless_rgba_with_textures(
        &vertices_a,
        &view_proj,
        width,
        height,
        Some(&map.textures),
    ) {
        Ok(rgba) => rgba,
        Err(RenderError::GpuUnavailable(reason)) => {
            eprintln!("skip=real_model_gpu reason=no_adapter detail={reason}");
            return;
        }
        Err(error) => panic!("terrain-only headless render failed: {error}"),
    };
    // A 已成功 ⇒ adapter 可用；B 若再报 GpuUnavailable 属异常，MUST NOT 静默
    // return 显示 passed（critic residual）。
    let rgba_b = match render_headless_rgba_with_textures(
        &vertices_b,
        &view_proj,
        width,
        height,
        Some(&map.textures),
    ) {
        Ok(rgba) => rgba,
        Err(error) => panic!("model headless render failed after terrain-only succeeded: {error}"),
    };
    assert_eq!(rgba_b.len(), (width * height * 4) as usize);

    // ① 全场景非黑屏：大量像素亮于 Clear 背景。
    let lit = rgba_b
        .chunks_exact(4)
        .filter(|p| p[0] > 40 || p[1] > 40 || p[2] > 40)
        .count();
    assert!(lit > 500, "真实地图画面应有大量非背景像素，实际 {lit}");

    // ② 关键：B 相对 A 有大量像素被真实模型改变 —— 地形自身无法伪造。
    let changed = rgba_a
        .chunks_exact(4)
        .zip(rgba_b.chunks_exact(4))
        .filter(|(a, b)| {
            let delta = a[0].abs_diff(b[0]) as u32
                + a[1].abs_diff(b[1]) as u32
                + a[2].abs_diff(b[2]) as u32;
            delta > 24
        })
        .count();
    assert!(
        changed > 200,
        "真实模型应显著改变画面像素（A=仅地形 vs B=地形+模型），实际变化 {changed} 像素"
    );

    // ③ 变化像素里出现多种模型贴图颜色，而非单一色块。
    let mut model_colors = std::collections::BTreeSet::new();
    for (a, b) in rgba_a.chunks_exact(4).zip(rgba_b.chunks_exact(4)) {
        if a[0].abs_diff(b[0]) > 16 || a[1].abs_diff(b[1]) > 16 || a[2].abs_diff(b[2]) > 16 {
            model_colors.insert([b[0] / 16, b[1] / 16, b[2] / 16]);
        }
    }
    assert!(
        model_colors.len() > 8,
        "真实模型应带来多种贴图颜色，实际 {}",
        model_colors.len()
    );
}

/// 真 GPU 纹理管线 headless 自测：同一顶点流，上传真实贴图数组 vs 占位白
/// 纹理，证明片元着色器确实按 `tex_layer` 逐像素采样 `texture_2d_array`
/// （而非用顶点色 / 纯白占位糊弄）。这是 CLI 侧可断言的"贴图上屏"事实。
#[test]
fn gpu_texture_sampling_changes_pixels_vs_placeholder() {
    use w3_render::{Camera, RenderError, render_headless_rgba_with_textures};

    let Some(map) = local_model_map() else {
        return;
    };
    let mut scene = w3_render::Scene::default();
    scene.set_terrain_tiles(map.terrain_tiles.clone());
    scene.set_terrain_texture_layers(map.terrain_texture_layers);
    scene.set_static_model_vertices(map.static_model_vertices.clone());
    let vertices = scene.build_vertices(&map.terrain);
    assert!(
        vertices
            .iter()
            .any(|v| v.tex_mode > 0.0 && v.tex_layer >= 0.0),
        "顶点流应含 GPU 纹理采样顶点"
    );

    let view_proj = Camera::new(1.0).matrix(&map.terrain);
    let width = 160u32;
    let height = 120u32;
    // 真实贴图数组。
    let textured = match render_headless_rgba_with_textures(
        &vertices,
        &view_proj,
        width,
        height,
        Some(&map.textures),
    ) {
        Ok(rgba) => rgba,
        Err(RenderError::GpuUnavailable(reason)) => {
            eprintln!("skip=gpu_texture_sampling reason=no_adapter detail={reason}");
            return;
        }
        Err(error) => panic!("textured headless render failed: {error}"),
    };
    // 占位白纹理（tex_layer 越界被 clamp 到唯一的白层）。
    let placeholder =
        render_headless_rgba_with_textures(&vertices, &view_proj, width, height, None)
            .expect("placeholder render after textured succeeded");

    let changed = textured
        .chunks_exact(4)
        .zip(placeholder.chunks_exact(4))
        .filter(|(a, b)| {
            a[0].abs_diff(b[0]) as u32 + a[1].abs_diff(b[1]) as u32 + a[2].abs_diff(b[2]) as u32
                > 24
        })
        .count();
    assert!(
        changed > 500,
        "真实贴图数组应显著改变像素（相对占位白纹理），证明 GPU 采样生效，实际 {changed}"
    );
}
