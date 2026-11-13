//! 离屏整图探针：用真实 `melee` example 的相机参数渲染一帧并写 PNG。
//!
//! 纯本地诊断工具，供 `ViewImage` 离线自证「真实地图 + 真实模型贴图 + GPU
//! 纹理管线」的画面，避免每轮都开真窗口人工观察。它复用与可玩入口完全
//! 相同的装配（`real_map::assemble`）、冻结定义、开局实体与 `CameraState`
//! 初始视角，并上传真实 `texture_2d_array`。
//!
//! 用法（仓库根目录执行）：
//!   `cargo run -p w3-client --example render_probe -- target/probe.png`
//! 输出 PNG 留在本机 `target/`，绝不入库（ADR 0001）。全部是表现层数据，
//! 不进入权威状态 / 状态哈希 / 快照（ADR 0002），`w3-core` 零改动。

use std::path::Path;

use w3_client::melee::{MeleeSetup, synthetic_definitions, synthetic_world};
use w3_client::{CameraAction, CameraState};
use w3_core::{MatchConfig, RtsMatch};
use w3_render::{
    Camera, FrameInput, Model, ModelLibrary, Scene, render_headless_rgba_with_textures,
};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;
const LOGIC_FPS: u32 = 30;

/// 与可玩入口一致的纯内存模型库：按开局实体 ID 绑定类型键。
fn synthetic_library(setup: &MeleeSetup) -> ModelLibrary {
    let mut library = ModelLibrary::default();
    for key in [
        "default",
        "peasant",
        "soldier",
        "town-hall",
        "barracks",
        "gold-mine",
        "tree",
    ] {
        library.insert(key, Model::default());
    }
    for (id, key) in setup.model_keys() {
        library.bind_entity(id, key);
    }
    library
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "target/render_probe.png".to_string());
    // 可选 zoom / pan_x / pan_y（argv[2..4]），用于离线批量挑相机参数；
    // 缺省与可玩入口 `melee` 的初始视角一致。
    let zoom = args
        .get(2)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(3.5);
    let pan = [
        args.get(3)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(-92.0),
        args.get(4)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0),
    ];
    // 与真窗口一致：16:9 宽高比，1280×720 离屏。
    let width: u32 = 1280;
    let height: u32 = 720;
    let aspect = width as f32 / height as f32;

    let (map, _paths) = w3_client::real_map::assemble(Path::new("config.toml"))
        .expect("render_probe: reason=real_map_unavailable");

    // 与 melee 相同的开局冻结定义 / 命令，令动态单位（主城 / 农民）上屏。
    let setup = MeleeSetup::new();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim =
        RtsMatch::create_with_definitions(config, synthetic_world(), synthetic_definitions());
    for command in &setup.commands {
        sim.submit_command(*command)
            .expect("spawn command rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("gameplay command rejected");
    }
    let view = sim.read_view();
    let library = synthetic_library(&setup);

    let mut scene = Scene::default();
    scene.set_static_markers(map.markers.clone());
    scene.set_terrain_tiles(map.terrain_tiles.clone());
    scene.set_terrain_texture_layers(map.terrain_texture_layers);
    scene.set_static_model_vertices(map.static_model_vertices.clone());
    scene.set_entity_models(map.entity_models.clone());
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &library,
    );
    let mut vertices = scene.build_vertices(&map.terrain);

    // 诊断模式（argv[5]="diag"）：把顶点按类别改纯色 + tex_mode=0，用于定位
    // "深灰锯齿三角"究竟是几何洞（背景）还是水/地表着色。仅诊断工具，不入库逻辑。
    let diag = args.get(5).is_some_and(|s| s == "diag");
    if diag {
        for v in &mut vertices {
            if (0.0..6.0).contains(&v.surface) {
                // 地形陆地：纯绿，tex_mode=0 不采样。
                v.color = [0.0, 1.0, 0.0, 1.0];
                v.tex_mode = 0.0;
            } else if (v.surface - 6.0).abs() < 0.01 {
                // 水面：纯红。
                v.color = [1.0, 0.0, 0.0, 1.0];
                v.tex_mode = 0.0;
            }
        }
    }

    // 与真窗口 `resumed` 一致：Resize 更新 aspect，set_initial_view 设局部视角。
    let mut state = CameraState::new(aspect);
    state.apply(CameraAction::Resize(width, height));
    state.set_initial_view(zoom, pan);
    let view_proj: Camera = state.camera();
    let view_proj = view_proj.matrix(&map.terrain);

    let rgba = render_headless_rgba_with_textures(
        &vertices,
        &view_proj,
        width,
        height,
        Some(&map.textures),
    )
    .expect("render_probe: reason=headless_render_failed");
    w3_render::gpu::readback::write_png(Path::new(&out), &rgba, width, height)
        .expect("render_probe: reason=write_png_failed");
    println!(
        "probe written: path={} vertices={} texture_layers={} terrain_tiles={} zoom={} pan={:?}",
        out,
        vertices.len(),
        map.textures.layer_count,
        map.terrain_tiles.as_ref().map_or(0, |t| t.len()),
        zoom,
        pan,
    );
}
