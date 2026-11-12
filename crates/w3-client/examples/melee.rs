//! 真实 Echo Isles 地图上的一局 1v1 Melee（对齐 `godot_warcraft3` 水位）。
//!
//! 用法：`cargo run -p w3-client --example melee`（在本仓库根目录执行）。
//!
//! 本 example 只做**装配**，把三条既有链路串起来：
//!
//! - 真实地图：读取本机 `map-parsed/echoisles/` 导出数据，经
//!   [`w3_client::real_map`] 缩放到玩法世界尺度，生成真实高度场地形
//!   （129×97 网格、Lordaeron Summer 地表分区、悬崖 / 水体、97 个单位摆放
//!   与 2491 个装饰物摆放）。路径由 gitignored 的本机 `config.toml` 提供
//!   （ADR 0001）；缺配置或导出数据无效时**显式失败**，MUST NOT 静默回落到
//!   合成 demo。仓库不含任何导出资产。
//! - 玩法闭环：`w3_client::melee` 的合成冻结定义、开局 `Spawn` 脚本、
//!   确定性外部 AI 命令源与既有 `ClientApp` 原样保留（玩法坐标 -96..96
//!   落在真实地形范围内，相机 / 拾取 / `StaticWorld` 均无需改动）。
//! - `w3-core` 零改动；真实导出坐标、`StaticMarker`、类型标签全部是表现层
//!   数据，MUST NOT 进入权威状态、状态哈希或 `SNAPSHOT_VERSION`（当前 3）
//!   定义的快照（ADR 0002）。
//!
//! 阶段 2 接入真实 BLP→PNG 地表贴图（CPU 采样进地形顶点色）与
//! `war3mapMap.png` 小地图底图（CPU 降采样颜色网格）；阶段 3 把 2588 个
//! 静态摆放与 6 类动态单位全部替换为真实 glTF WC3 模型（bind pose 静态网格
//! + 逐顶点 UV 采样贴图，程序化几何仅在无本机资产 / 缺键时回退）。真实模型
//! 由 [`w3_client::model_assembly`] 装配、经 `ClientApp` builder 注入 `Scene`。
//! 逐帧骨骼动画蒙皮与逐像素 GPU 纹理过滤留后续阶段。以上全部是表现层数据，
//! MUST NOT 进入权威状态、状态哈希或快照（ADR 0002）。

use std::path::Path;

use w3_client::melee::{MeleeDriver, MeleeSetup, synthetic_definitions, synthetic_world};
use w3_client::{ClientApp, FixedStepClient};
use w3_core::{MatchConfig, RtsMatch};
use w3_render::{Model, ModelLibrary};

/// 固定种子，与既有确定性基线一致。
const SEED: u64 = 0x5eed;
/// 固定内容哈希。
const CONTENT_HASH: u64 = 0xC0FFEE;
/// 冻结的逻辑帧率，与 `FixedStepClient` 的 `LOGIC_FPS` 一致。
const LOGIC_FPS: u32 = 30;
/// 呈现帧上限：不限帧数，由人工关闭窗口退出（表现层参数）。
const MAX_FRAMES: u32 = u32::MAX;
/// AI 外部命令脚本的预生成帧数上限。
const AI_HORIZON: u64 = 3600;

/// 纯内存模型库：为每种动态实体类型插入一个空模型键，并按开局实体 ID
/// 绑定类型标签。`Scene` 用该 key 匹配 [`Scene::set_entity_models`] 注入的
/// 真实 glTF 模型；无真实模型时回退到程序化几何。本库本身零资产读取，
/// 真实网格由 `real_map::assemble` 从本机 glTF 装配后单独注入。
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
    // 装配既有 stderr 日志后端，让库内英文结构化 key=value 日志可观测。
    w3_net::init_logging();

    // 真实地图：缺 config.toml / 导出数据无效 → 显式失败，不回落合成 demo。
    let (real_map, _paths) = w3_client::real_map::assemble(Path::new("config.toml")).unwrap_or_else(
        |error| {
            log::error!(
                target: "w3_client",
                "real map unavailable: error={} hint=create gitignored config.toml with map_dir pointing to the local Echo Isles export",
                error
            );
            panic!("melee example: reason=real_map_unavailable error={error}");
        },
    );
    log::info!(
        target: "w3_client",
        "real map loaded: vertices={} static_markers={} columns={} terrain_tiles={} minimap={} model_vertices={} entity_models={} models_resolved={}/{}",
        real_map.terrain.vertices.len(),
        real_map.markers.len(),
        real_map.terrain.columns,
        real_map.terrain_tiles.as_ref().map_or(0, |tiles| tiles.len()),
        real_map.minimap_image.is_some(),
        real_map.static_model_vertices.len(),
        real_map.entity_models.len(),
        real_map.model_stats.resolved,
        real_map.model_stats.placements,
    );

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
            .expect("melee example: reason=spawn_command_rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("melee example: reason=opening_gameplay_rejected");
    }
    // 对手行为全部由 `MeleeDriver` 的外部确定性命令源提供；不开启内核
    // 既有 `SetAi` 开关，避免内部 AI 对玩家空闲单位额外发出采集命令
    // （design D3：单一外部命令源）。
    let driver = MeleeDriver::new(FixedStepClient::from_match(sim), &setup, AI_HORIZON);
    let kinds = setup.kinds.clone();
    let library = synthetic_library(&setup);

    ClientApp::new(driver, real_map.terrain, library, MAX_FRAMES)
        .with_static_markers(real_map.markers)
        .with_terrain_tiles(real_map.terrain_tiles)
        .with_minimap_image(real_map.minimap_image)
        .with_static_model_vertices(real_map.static_model_vertices)
        .with_entity_models(real_map.entity_models)
        .with_entity_kinds(kinds)
        .run()
        .expect("melee example: reason=window_run_failed");
}
