//! 可玩窗口冒烟入口（change `playable-window-smoke`）。
//!
//! 用法：`cargo run -p w3-client --example playable`。
//!
//! 本 example 只补一个最小启动入口，把 P6 已交付的真实 winit 窗口 /
//! wgpu 呈现 / 输入 / HUD / 30 Hz 累加器路径启动起来，供人工在有 GPU
//! 与显示设备的桌面执行窗口冒烟验收。全部数据为内存合成：合成高度场
//! （蓝本 `w3-render/examples/render_frame.rs::fallback_terrain`）与合成
//! `Spawn` 命令脚本（蓝本 `w3-core/tests/determinism.rs`）；入口不读取
//! 任何本机导出资产或游戏资产 / MPQ 派生数据（ADR 0001）。
//!
//! 装配只复用 `ClientApp` 的 `new` 与 `run` 两个公开入口：不自建
//! `EventLoop` / `ApplicationHandler` / `GpuContext`，不出现第二份
//! `RtsMatch`。权威坐标全部由 `SimFixed::from_int` 从整数常量构造，
//! 不存在 `f64 → SimFixed` 转换路径（ADR 0004）；合成高度场里的 `f64`
//! 只用于 `TerrainMesh` 表现层几何，不进入权威状态、状态哈希或快照。
//!
//! 逻辑帧由 `FixedStepClient` 内冻结的 `logic_fps = 30` 整数累加器驱动，
//! `step()` 无参；窗口尺寸、相机、选中、HUD、累加器余数等表现层状态
//! 不进入 FNV-1a 状态哈希或 `SNAPSHOT_VERSION`（当前 3）快照（ADR 0002）。
//!
//! 为让人工清单可实际验收，合成数据额外做三处对齐：向模型库插入一个
//! 纯内存的占位模型（不加载任何资产文件），使 `Scene` 生成单位占位
//! 标记；把地形瓦片边长、中心偏移与合成世界的格距 / 原点对齐，使
//! 相机观察中心与拾取映射都落在世界原点；注入纯内存合成的冻结玩法
//! 定义（`unit_def = 1` / `ability = 10`），否则 `create_with_world` 的
//! 空定义会让 Q 施法 / T 生产 / C 取消在内核里 NoEffect。

use w3_client::{ClientApp, FixedStepClient};
use w3_content::CenterOffset;
use w3_core::{
    AbilityDef, Command, CommandKind, EffectDef, EffectKind, Frame, FrozenDefinitions, MatchConfig,
    RtsMatch, SimFixed, StaticWorld, UnitDef,
};
use w3_render::{Model, ModelLibrary, TerrainMesh};

/// 固定种子，与 `determinism.rs::config()` / `golden_hash.rs` 一致。
const SEED: u64 = 0x5eed;
/// 固定内容哈希，与 `determinism.rs::config()` / `golden_hash.rs` 一致。
const CONTENT_HASH: u64 = 0xC0FFEE;
/// 合成世界的格数，与 `render_frame.rs::SimProducer::new` 一致。
const WORLD_WIDTH: u32 = 16;
const WORLD_HEIGHT: u32 = 16;
/// 合成世界与地形的统一格距；两者尺度一致，避免单位被缩到不可见。
const CELL_SIZE: i64 = 8;
/// 呈现帧上限：取 `u32::MAX` 表示不限帧数，由人工关闭窗口退出。
/// 该值仅为表现层呈现帧计数，不进入权威状态。
const MAX_FRAMES: u32 = u32::MAX;
/// 合成高度场的网格边长，与 `render_frame.rs::fallback_terrain` 一致。
const TERRAIN_SIZE: usize = 17;
/// 生产默认单位定义编号，与 `w3_client::input::DEFAULT_UNIT_DEF` 一致。
const UNIT_DEF: u32 = 1;
/// 施法默认技能定义编号，与 `w3_client::input::DEFAULT_ABILITY` 一致。
const ABILITY_DEF: u32 = 10;

/// 由整数字面量构造 Q47.16 定点值；显式处理错误，不静默吞错。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("playable: reason=fixed_conversion_failed")
}

/// 合成世界：以世界原点为中心的开阔地形，不读取任何地图数据。
///
/// `origin` 取负半宽，使模拟世界坐标范围与地形网格对齐，右键移动等
/// 拾取结果落在合法格子内。
fn synthetic_world() -> StaticWorld {
    let half = fixed(-(WORLD_WIDTH as i64) * CELL_SIZE / 2);
    StaticWorld::new(
        WORLD_WIDTH,
        WORLD_HEIGHT,
        fixed(CELL_SIZE),
        (half, half),
        Vec::new(),
        Vec::new(),
    )
}

/// 合成冻结玩法定义：让 `Train`（`UNIT_DEF`）与 `Cast`（`ABILITY_DEF`）
/// 在内核里有可查的定义；全部为编译期整数常量，不读取任何资产。
fn synthetic_defs() -> FrozenDefinitions {
    let unit = UnitDef {
        hp: 100,
        mana: 100,
        speed_raw: fixed(3).raw(),
        damage: 10,
        range_raw: fixed(10).raw(),
        supply: 1,
        train_frames: 30,
    };
    let ability = AbilityDef {
        mana_cost: 10,
        cooldown_frames: 5,
        cast_frames: 3,
        behavior: 0,
        effects: vec![EffectDef {
            kind: EffectKind::Damage,
            amount: 30,
            duration_frames: 0,
        }],
    };
    FrozenDefinitions::new(
        vec![(UNIT_DEF, unit)],
        vec![(ABILITY_DEF, ability)],
        Vec::new(),
        Vec::new(),
    )
    .expect("playable: reason=invalid_synthetic_definitions")
}

/// 构造一条合成 `Spawn` 命令；全部数值为编译期整数常量。
fn spawn(x: i64, y: i64, owner: u8, sequence: u32) -> Command {
    Command::new(
        Frame::new(0),
        owner,
        sequence,
        CommandKind::Spawn {
            x: fixed(x),
            y: fixed(y),
            hp: 100,
            speed: fixed(3),
            attack_damage: 10,
            attack_range: fixed(10),
        },
    )
}

/// 合成 `Spawn` 命令脚本：帧 0 在世界原点附近为玩家 1 放置三个己方
/// 单位、为玩家 2 放置一个敌方单位，分散在屏幕中央供框选 / 右键
/// 移动 / 施法 / 生产验收。执行帧固定为 0，序列在同一玩家内唯一。
fn spawn_script() -> Vec<Command> {
    vec![
        spawn(-32, -16, 1, 1),
        spawn(0, -16, 1, 2),
        spawn(32, -16, 1, 3),
        spawn(0, 32, 2, 1),
    ]
}

/// 无本机导出数据时的合成高度场（中间低、四周高的碗状地形）。
///
/// 这里的 `f64` 只用于 `TerrainHeightfield` → `TerrainMesh` 的表现层几何
/// 构造，不产生 `SimFixed`，也不进入权威状态 / 状态哈希 / 快照。
/// `tile_size` 取模拟世界的格距，`center_offset` 取负半宽，使 `Camera`
/// 的观察中心与 `ScreenMapper` 的屏幕中心都落在世界原点。
fn fallback_terrain() -> TerrainMesh {
    let mut heights = Vec::with_capacity(TERRAIN_SIZE * TERRAIN_SIZE);
    for y in 0..TERRAIN_SIZE {
        for x in 0..TERRAIN_SIZE {
            let dx = x as f64 - (TERRAIN_SIZE / 2) as f64;
            let dy = y as f64 - (TERRAIN_SIZE / 2) as f64;
            // 保持很浅的起伏：拾取以固定高度平面为基准，避免贴地高度
            // 起伏让单位视觉位置与拾取投影错位。
            heights.push((dx * dx + dy * dy).sqrt() * 0.15);
        }
    }
    let half_offset = -(TERRAIN_SIZE as f64) * (CELL_SIZE as f64) * 0.5;
    let hf = w3_content::TerrainHeightfield {
        tilepoint_width: TERRAIN_SIZE,
        tilepoint_height: TERRAIN_SIZE,
        map_width: TERRAIN_SIZE - 1,
        map_height: TERRAIN_SIZE - 1,
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
    TerrainMesh::from_heightfield(&hf).expect("playable: reason=invalid_synthetic_heightfield")
}

fn main() {
    // 装配既有 stderr 日志后端，让库内的英文结构化 key=value 日志
    // （如 `hud font unavailable`、`gpu init failed`）真正可观测。
    w3_net::init_logging();

    // 唯一权威状态：合成对局带冻结玩法定义，命令只经公开入口入队。
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: CONTENT_HASH,
    };
    let mut sim = RtsMatch::create_with_definitions(config, synthetic_world(), synthetic_defs());
    for command in spawn_script() {
        sim.submit_command(command)
            .expect("playable: reason=spawn_command_rejected");
    }
    let driver = FixedStepClient::from_match(sim);

    // 纯内存占位模型：`Scene` 仅在模型键可解析时绘制单位标记，插入
    // 空模型的 `default` 键即可让程序化四面体占位几何上屏，零资产读取。
    let mut library = ModelLibrary::default();
    library.insert("default", Model::default());

    // 单一装配点：窗口 / GpuContext / 输入 / HUD / 30 Hz 累加器全部由
    // `ClientApp` 内部流程创建，example 只提供合成数据与装配参数。
    ClientApp::new(driver, fallback_terrain(), library, MAX_FRAMES)
        .run()
        .expect("playable: reason=window_run_failed");
}
