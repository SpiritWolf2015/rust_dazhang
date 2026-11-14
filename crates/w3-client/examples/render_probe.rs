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
    // 缺省与可玩入口 `melee` 的初始视角一致（`zoom=7`、`pan=[-72,34]`），
    // 保证离屏自证用的就是真窗口开局参数。
    let zoom = args
        .get(2)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(7.0);
    let pan = [
        args.get(3)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(-72.0),
        args.get(4)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(34.0),
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
    // 与真窗口首帧一致：推进一帧让 frame 0 的 `Spawn` 命令生效，否则
    // `read_view()` 返回空实体、动态单位（主城 / 农民）不会上屏。此前探针漏
    // 了这一步，导致所有离屏图只含静态摆放，无法自证开局可玩性。
    sim.step();
    let view = sim.read_view();
    let library = synthetic_library(&setup);

    let mode = args.get(5).map(String::as_str).unwrap_or("");

    // "units" 诊断：只渲动态单位（跳过静态摆放），确认主城 / 农民真实网格
    // 是否加载、是否贴地、彼此间距。仅诊断工具，不入库逻辑。
    let units_only = mode == "units";
    let mut scene = Scene::default();
    scene.set_terrain_tiles(map.terrain_tiles.clone());
    scene.set_terrain_texture_layers(map.terrain_texture_layers);
    if !units_only {
        scene.set_static_markers(map.markers.clone());
        scene.set_static_model_vertices(map.static_model_vertices.clone());
    }
    scene.set_entity_models(map.entity_models.clone());
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &library,
    );
    let mut vertices = scene.build_vertices(&map.terrain);
    if units_only {
        // 地形染成深色，突出动态单位模型。
        for v in &mut vertices {
            if v.surface >= 0.0 {
                v.color = [0.15, 0.18, 0.15, 1.0];
                v.tex_mode = 0.0;
            }
        }
        let dynamic: Vec<_> = vertices.iter().filter(|v| v.surface < 0.0).collect();
        println!(
            "units diag: dynamic_vertices={} total={}",
            dynamic.len(),
            vertices.len()
        );
    }

    // 诊断模式（argv[5]="diag"）：把顶点按类别改纯色 + tex_mode=0，用于定位
    // "深灰锯齿三角"究竟是几何洞（背景）还是水/地表着色。仅诊断工具，不入库逻辑。
    let diag = mode == "diag";
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

    // "picks" 诊断：量化 HUD 投影点 / 框选包围盒 与 真实模型顶点屏幕范围
    // 的错位，定位「框选选不上」根因。仅诊断工具，不入库逻辑。
    if mode == "picks" {
        use w3_render::ScreenProjection;
        let proj = ScreenProjection::new(&view_proj, &map.terrain, width as f32, height as f32);
        let dynamic_world: Vec<[f64; 2]> = vertices
            .iter()
            .filter(|v| v.surface < 0.0)
            .map(|v| [f64::from(v.position[0]), f64::from(v.position[2])])
            .collect();
        for entity in &view.entities {
            let world = [
                f64::from(w3_render::world_to_f32(entity.x)),
                f64::from(w3_render::world_to_f32(entity.y)),
            ];
            let hud = proj.world_to_screen(world);
            let (bmin, bmax) = proj.unit_screen_bounds(world);
            let terrain_y = f64::from(
                map.terrain
                    .height_at_world(world[0] as f32, world[1] as f32),
            );
            let ground = proj.project_world_point(world[0], terrain_y, world[1]);
            // 该单位附近动态模型顶点的屏幕包围盒（最贴近用户看到的位置）。
            let mut dmin = [f64::MAX, f64::MAX];
            let mut dmax = [f64::MIN, f64::MIN];
            let mut count = 0usize;
            for &[wx, wz] in &dynamic_world {
                if (wx - world[0]).abs() < 16.0 && (wz - world[1]).abs() < 16.0 {
                    let p = proj.project_world_point(
                        wx,
                        f64::from(map.terrain.height_at_world(wx as f32, wz as f32)),
                        wz,
                    );
                    for a in 0..2 {
                        dmin[a] = dmin[a].min(p[a]);
                        dmax[a] = dmax[a].max(p[a]);
                    }
                    count += 1;
                }
            }
            println!(
                "pick id={} owner={} world=({:.0},{:.0}) hud=({:.0},{:.0}) ground=({:.0},{:.0}) bounds=[({:.0},{:.0})-({:.0},{:.0})] model_verts={} model_screen=[({:.0},{:.0})-({:.0},{:.0})]",
                entity.id,
                entity.owner,
                world[0],
                world[1],
                hud[0],
                hud[1],
                ground[0],
                ground[1],
                bmin[0],
                bmin[1],
                bmax[0],
                bmax[1],
                count,
                dmin[0],
                dmin[1],
                dmax[0],
                dmax[1],
            );
        }
    }

    let view_proj = view_proj.matrix(&map.terrain);

    let mut rgba = render_headless_rgba_with_textures(
        &vertices,
        &view_proj,
        width,
        height,
        Some(&map.textures),
    )
    .expect("render_probe: reason=headless_render_failed");

    // "boxes" 诊断：把真实模型屏幕包围盒画成绿色描边（模拟 HUD 选中框），
    // 把旧 `world_to_screen` 低空平面锚点画成红点（对比错位）。用于离屏
    // 目视确认「选框贴合模型」这一用户问题是否修复。仅诊断工具。
    if mode == "boxes" {
        use w3_render::ScreenProjection;
        let proj =
            ScreenProjection::new(&state.camera(), &map.terrain, width as f32, height as f32);
        let boxes = scene.entity_screen_boxes(&map.terrain, &proj);
        for (id, (bmin, bmax)) in &boxes {
            println!(
                "box id={} size=({:.0}x{:.0}) at=({:.0},{:.0})",
                id,
                bmax[0] - bmin[0],
                bmax[1] - bmin[1],
                bmin[0],
                bmin[1],
            );
        }
        let set = |rgba: &mut Vec<u8>, x: i64, y: i64, color: [u8; 3]| {
            if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
                return;
            }
            let i = ((y as u32 * width + x as u32) * 4) as usize;
            rgba[i] = color[0];
            rgba[i + 1] = color[1];
            rgba[i + 2] = color[2];
            rgba[i + 3] = 255;
        };
        // 绿框：真实模型 box（粗 2px 描边）。
        for (_, (min, max)) in &boxes {
            let (x0, y0) = (min[0] as i64, min[1] as i64);
            let (x1, y1) = (max[0] as i64, max[1] as i64);
            for t in 0..2i64 {
                for x in x0..=x1 {
                    set(&mut rgba, x, y0 + t, [0, 255, 0]);
                    set(&mut rgba, x, y1 - t, [0, 255, 0]);
                }
                for y in y0..=y1 {
                    set(&mut rgba, x0 + t, y, [0, 255, 0]);
                    set(&mut rgba, x1 - t, y, [0, 255, 0]);
                }
            }
        }
        // 红点：旧低空平面锚点（每个实体 5×5 方块，醒目对比）。
        for entity in &view.entities {
            let world = [
                f64::from(w3_render::world_to_f32(entity.x)),
                f64::from(w3_render::world_to_f32(entity.y)),
            ];
            let p = proj.world_to_screen(world);
            for dx in -2..=2 {
                for dy in -2..=2 {
                    set(&mut rgba, p[0] as i64 + dx, p[1] as i64 + dy, [255, 0, 0]);
                }
            }
        }
        println!("boxes diag: entity_boxes={}", boxes.len());
    }

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
