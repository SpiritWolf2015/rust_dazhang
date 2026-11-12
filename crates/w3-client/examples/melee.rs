//! 可玩的一局 1v1 Melee 入口（change `melee-playable-loop`）。
//!
//! 用法：`cargo run -p w3-client --example melee`。
//!
//! 本 example 只做**装配**：把 `w3-client::melee` 提供的合成冻结定义、
//! 开局 `Spawn` 脚本、确定性外部 AI 命令源与既有 `ClientApp` 串成一局，
//! 从开局采集、建造、训练、进攻到分出胜负全程可在窗口内完成。
//!
//! 全部数据为内存合成：不读取 `config.toml`、导出 JSON 或任何游戏资产
//! （ADR 0001）；`w3-core` 零改动，权威状态、状态哈希与
//! `SNAPSHOT_VERSION`（当前 3）定义的快照不受影响（ADR 0002）。
//!
//! 呈现层通过 `ModelLibrary::bind_entity` 把实体类型标签映射为不同的
//! 程序化几何（农民 / 士兵 / 建筑 / 资源点），全部为表现层数据，
//! MUST NOT 进入权威状态、状态哈希或快照。

use w3_client::melee::{MeleeDriver, MeleeSetup, synthetic_definitions, synthetic_world};
use w3_client::{ClientApp, FixedStepClient};
use w3_content::CenterOffset;
use w3_core::{MatchConfig, RtsMatch};
use w3_render::{Model, ModelLibrary, TerrainMesh};

/// 固定种子，与既有确定性基线一致。
const SEED: u64 = 0x5eed;
/// 固定内容哈希。
const CONTENT_HASH: u64 = 0xC0FFEE;
/// 冻结的逻辑帧率，与 `FixedStepClient` 的 `LOGIC_FPS` 一致。
const LOGIC_FPS: u32 = 30;
/// 合成世界格数与格距（与 `melee::setup::synthetic_world` 一致）。
const WORLD_WIDTH: u32 = 32;
const CELL_SIZE: i64 = 8;
/// 呈现帧上限：不限帧数，由人工关闭窗口退出（表现层参数）。
const MAX_FRAMES: u32 = u32::MAX;
/// AI 外部命令脚本的预生成帧数上限。
const AI_HORIZON: u64 = 3600;
/// 合成高度场网格边长 = 世界格数 + 1。
const TERRAIN_SIZE: usize = (WORLD_WIDTH + 1) as usize;

/// 无本机导出数据时的合成高度场：很浅的碗状地形，尺度与 `synthetic_world`
/// 对齐，使相机观察中心、拾取映射与单位坐标都落在世界原点附近。
///
/// 这里的 `f64` 只用于 `TerrainHeightfield` → `TerrainMesh` 的表现层几何，
/// 不产生 `SimFixed`，也不进入权威状态 / 状态哈希 / 快照。
fn fallback_terrain() -> TerrainMesh {
    let mut heights = Vec::with_capacity(TERRAIN_SIZE * TERRAIN_SIZE);
    for y in 0..TERRAIN_SIZE {
        for x in 0..TERRAIN_SIZE {
            let dx = x as f64 - (TERRAIN_SIZE / 2) as f64;
            let dy = y as f64 - (TERRAIN_SIZE / 2) as f64;
            heights.push((dx * dx + dy * dy).sqrt() * 0.15);
        }
    }
    let half_offset = -(WORLD_WIDTH as f64) * (CELL_SIZE as f64) * 0.5;
    let hf = w3_content::TerrainHeightfield {
        tilepoint_width: TERRAIN_SIZE,
        tilepoint_height: TERRAIN_SIZE,
        map_width: WORLD_WIDTH as usize,
        map_height: WORLD_WIDTH as usize,
        center_offset: Some(CenterOffset {
            x: half_offset,
            y: half_offset,
        }),
        main_tileset: None,
        main_tileset_name: None,
        ground_tilesets: Vec::new(),
        cliff_tilesets: Vec::new(),
        tile_size: Some(CELL_SIZE as f64),
        heights,
        ground_textures: Vec::new(),
        ground_variations: Vec::new(),
        cliff_textures: Vec::new(),
        cliff_variations: Vec::new(),
        layer_heights: Vec::new(),
        water_heights: Vec::new(),
        flags_packed: Vec::new(),
    };
    TerrainMesh::from_heightfield(&hf).expect("melee example: reason=invalid_synthetic_heightfield")
}

/// 纯内存模型库：为每种实体类型插入一个空模型键，并按开局实体 ID 绑定
/// 类型标签，使 `Scene` 生成对应程序化几何（零资产读取）。
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

    ClientApp::new(driver, fallback_terrain(), library, MAX_FRAMES)
        .with_entity_kinds(kinds)
        .run()
        .expect("melee example: reason=window_run_failed");
}
