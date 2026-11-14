//! 批次 B 验收：HUD / 相机 / 输入翻译 / 命令提交 / 冻结边界 / 节拍接入。
//!
//! 全部用合成命令脚本与合成地形，不读取任何真实导出资产、不开 GPU 窗口、
//! 不引入墙钟（宿主时钟仅测其整数换算纯度）。核心断言：
//! - HUD 生命周期、相机操作后，`compute_state_hash()` 与快照逐字节不变，
//!   `SNAPSHOT_VERSION` 仍为 3；
//! - 60 Hz / 144 Hz 呈现节奏经宿主整数 tick 喂入后逐帧帧号与哈希一致；
//! - 命令只经 `submit_command` / `submit_gameplay_command`，执行帧由冻结
//!   帧号推导，坐标经 `freeze_f64` 冻结边界；
//! - 选中集合变化不生成命令、不改哈希；提交失败显式返回错误；
//! - `WindowEvent::Resized` 等价动作只改 aspect，MUST NOT 推进逻辑帧。

use w3_client::{
    ClientApp, ClientCommand, ClientError, FixedStepClient, FrameAdvance, HudAssetError, HudAssets,
    HudModel, InputAction, InputError, InputState, LEAD_FRAMES, ScreenMapper, StepDriver,
    TICKS_PER_LOGIC_FRAME, TICKS_PER_SECOND, ZH,
};
use w3_core::{
    AbilityDef, Command, CommandError, CommandKind, EffectDef, EffectKind, EntityId, Event, Frame,
    FrozenDefinitions, GameplayCommand, GameplayCommandKind, MatchConfig, RtsMatch, SCALE,
    SNAPSHOT_VERSION, SimFixed, Snapshot, StaticWorld, UnitDef, WorldView,
};
use w3_render::{Camera, ModelLibrary, ScreenProjection, TerrainMesh, TerrainVertex};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;

fn spawn(x: i64, y: i64, owner: u8) -> Command {
    Command::new(
        Frame::new(0),
        owner,
        1,
        CommandKind::Spawn {
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(y).unwrap(),
            hp: 100,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: 10,
            attack_range: SimFixed::from_int(10).unwrap(),
        },
    )
}

/// 合成地形：手工最小网格，不依赖本机导出资产。
fn synthetic_terrain() -> TerrainMesh {
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

/// 装配一个带两个己方单位与一个敌方单位的客户端应用。
fn build_app() -> ClientApp<FixedStepClient> {
    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(0, 0, 1)).unwrap();
    driver.submit_command(spawn(4, 0, 1)).unwrap();
    driver.submit_command(spawn(-4, 0, 2)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0);
    // 推进一帧让 Spawn 生效（step 无参，由累加器整数 tick 驱动）。
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
    app
}

fn hash(app: &ClientApp<FixedStepClient>) -> u64 {
    app.driver().sim().compute_state_hash()
}

fn snapshot(app: &ClientApp<FixedStepClient>) -> Snapshot {
    app.driver().sim().capture_snapshot()
}

/// 稳定字节视图：内核未暴露快照字节编码，用 `Debug` 表示作为可观测代理，
/// 配合 `Snapshot: PartialEq` 逐字段相等断言。
fn snapshot_bytes(snapshot: &Snapshot) -> Vec<u8> {
    format!("{snapshot:?}").into_bytes()
}

// ---------------------------------------------------------------------------
// B4：HUD 只读，生命周期内权威状态哈希与快照不变
// ---------------------------------------------------------------------------

#[test]
fn hud_lifecycle_keeps_state_hash_and_snapshot_bytes() {
    let mut rendered = build_app();
    let mut plain = build_app();
    let ctx = egui::Context::default();

    for _ in 0..24 {
        // rendered 一侧反复派生 HUD 数据并用 egui 绘制（纯 CPU，无 GPU）。
        let model = rendered.hud_model();
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            w3_client::draw(ctx, &model, &ZH, HudAssets { text_enabled: true });
        });
        // 测试不接 GPU 后端，显式丢弃本帧纹理增量，满足 egui `TexturesDelta`
        // 的 Drop 契约（生产路径在 `ClientApp::render_one` 中真正消费）。
        let _drawn_shapes = output.shapes.len();
        output.drop_without_applying_deltas();

        rendered.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

        assert_eq!(
            hash(&rendered),
            hash(&plain),
            "HUD drawing changed the state hash"
        );
        assert_eq!(
            snapshot(&rendered),
            snapshot(&plain),
            "HUD drawing changed the snapshot"
        );
        assert_eq!(
            snapshot_bytes(&snapshot(&rendered)),
            snapshot_bytes(&snapshot(&plain)),
            "HUD drawing changed the snapshot bytes"
        );
        assert_eq!(snapshot(&rendered).version, SNAPSHOT_VERSION);
        assert_eq!(SNAPSHOT_VERSION, 3);
    }
}

#[test]
fn font_degradation_does_not_change_authoritative_state() {
    let ctx = egui::Context::default();
    // 显式空字体定义 MUST 被判为资源不可用（spec「资源缺失」Scenario）。
    assert_eq!(
        HudAssets::from_fonts(&ctx, egui::FontDefinitions::empty()),
        Err(HudAssetError::EmptyFontData)
    );
    // 启用 `default_fonts` 后内置字体数据非空，HUD 文字元素正常绘制。
    assert_eq!(
        HudAssets::prepare(&ctx),
        Ok(HudAssets { text_enabled: true })
    );

    // 降级路径：文字元素不绘制、几何元素仍呈现，权威状态不变。
    let mut degraded = build_app();
    let mut plain = build_app();
    for _ in 0..8 {
        let model = degraded.hud_model();
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            w3_client::draw(ctx, &model, &ZH, HudAssets::degraded());
        });
        let _drawn_shapes = output.shapes.len();
        output.drop_without_applying_deltas();
        degraded.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        assert_eq!(hash(&degraded), hash(&plain));
        assert_eq!(snapshot(&degraded), snapshot(&plain));
        assert_eq!(snapshot(&degraded).version, 3);
    }
}

#[test]
fn hud_assets_accept_available_font_definitions() {
    let ctx = egui::Context::default();
    // 提供非空字体数据时（例如人工拍板启用 default_fonts 后），资源准备成功。
    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert(
        "synthetic".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(b"")),
    );
    assert_eq!(
        HudAssets::from_fonts(&ctx, fonts),
        Ok(HudAssets { text_enabled: true })
    );
}

#[test]
fn hud_model_is_read_only_projection() {
    let app = build_app();
    let view: WorldView = app.driver().sim().read_view();
    let snap = snapshot(&app);
    let mapper = app.screen_mapper();
    let events: Vec<Event> = Vec::new();
    let model = HudModel::collect(
        &view,
        &snap,
        app.selection(),
        &events,
        &mapper,
        app.selection().len() as u8,
    );
    assert_eq!(model.tick, view.tick);
    assert_eq!(model.units.len(), view.entities.len());
    assert_eq!(model.selected_count, 0);
    assert!(model.queue.is_empty());
    assert_eq!(snap.version, 3);
}

// ---------------------------------------------------------------------------
// B5：相机操作不改哈希与快照
// ---------------------------------------------------------------------------

#[test]
fn camera_operations_keep_state_hash_and_snapshot_v3() {
    use w3_client::CameraAction;

    let mut moved = build_app();
    let mut plain = build_app();
    for step in 0..24 {
        moved.apply_camera(CameraAction::Pan([8.0, -4.0]));
        moved.apply_camera(CameraAction::Zoom(0.05));
        moved.apply_camera(CameraAction::Rotate(0.02));
        if step % 4 == 0 {
            moved.apply_camera(CameraAction::Resize(1600, 900));
        }
        let _matrix = moved.camera();

        moved.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        assert_eq!(
            hash(&moved),
            hash(&plain),
            "camera operations changed the state hash"
        );
        assert_eq!(
            snapshot(&moved),
            snapshot(&plain),
            "camera operations changed the snapshot"
        );
        assert_eq!(snapshot(&moved).version, 3);
        assert_eq!(snapshot(&moved).defs_hash, snapshot(&plain).defs_hash);
    }
}

#[test]
fn with_initial_camera_is_presentation_only() {
    // 初始相机 builder（拉近到局部战场）只改表现层位姿，不推进逻辑帧、
    // 不改状态哈希 / 快照，且 MVP 矩阵与默认相机确实不同（builder 生效）。
    let terrain = synthetic_terrain();

    let default_driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    let default_app = ClientApp::new(default_driver, terrain.clone(), ModelLibrary::default(), 0);
    let default_view_proj = default_app.camera().matrix(&terrain);

    let focused_driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    let focused = ClientApp::new(focused_driver, terrain.clone(), ModelLibrary::default(), 0)
        .with_initial_camera(3.5, [-92.0, 0.0]);
    let focused_view_proj = focused.camera().matrix(&terrain);

    assert_ne!(
        default_view_proj.to_cols_array(),
        focused_view_proj.to_cols_array(),
        "初始相机必须改变 MVP，证明 builder 生效"
    );
    // 未推进逻辑帧、权威状态与全新 app 一致（纯表现层）。
    assert_eq!(focused.driver().sim().read_view().tick, 0);
    assert_eq!(hash(&focused), hash(&default_app), "初始相机不得改状态哈希");
    assert_eq!(
        snapshot(&focused),
        snapshot(&default_app),
        "初始相机不得改快照"
    );
}

#[test]
fn resize_only_changes_aspect_and_never_steps() {
    use w3_client::CameraAction;

    let mut app = build_app();
    let tick_before = app.driver().sim().read_view().tick;
    let hash_before = hash(&app);
    for _ in 0..32 {
        app.apply_camera(CameraAction::Resize(1920, 1080));
    }
    // Resize 是纯表现层动作：帧号与哈希不变（MUST NOT 触发 step()）。
    assert_eq!(app.driver().sim().read_view().tick, tick_before);
    assert_eq!(hash(&app), hash_before);
}

// ---------------------------------------------------------------------------
// B2：宿主整数 tick 节拍接入，双刷新率逐帧一致
// ---------------------------------------------------------------------------

/// 按 `refresh_hz` 呈现节奏喂整数 tick，返回逐帧 `(tick, hash)` 序列。
fn host_replay(refresh_hz: u64, seconds: u64) -> Vec<(u64, u64)> {
    let ticks_per_redraw = TICKS_PER_SECOND / refresh_hz;
    assert_eq!(ticks_per_redraw * refresh_hz, TICKS_PER_SECOND);
    let mut app = build_app();
    let mut trace = Vec::new();
    for _ in 0..(refresh_hz * seconds) {
        let plan: FrameAdvance = app.advance_with_ticks(ticks_per_redraw);
        // 每个整数逻辑帧推进后记录帧号与权威状态哈希。
        let steps = plan.steps;
        if steps > 0 {
            // 累加器在一轮内可能推进多帧；逐帧哈希由「末尾 tick + 对拍实例」保证，
            // 这里记录本轮末尾帧号，并与 30 Hz 基准逐帧比对。
            trace.push((app.driver().sim().read_view().tick, hash(&app)));
        }
    }
    trace
}

#[test]
fn host_tick_source_is_refresh_rate_independent() {
    let at_60 = host_replay(60, 2);
    let at_144 = host_replay(144, 2);
    // 两种呈现节奏在同一整数 tick 总量下推进相同逻辑帧数、逐帧哈希一致。
    assert_eq!(at_60, at_144);
    assert_eq!(at_60.len() as u64, 60);
    for (index, (tick, _)) in at_60.iter().enumerate() {
        assert_eq!(*tick, index as u64 + 2);
    }
}

#[test]
fn step_never_receives_delta_and_advances_integer_frames_only() {
    let mut app = build_app();
    let before = app.driver().sim().read_view().tick;
    // 半个逻辑帧：0 个整数帧。
    let half = app.advance_with_ticks(TICKS_PER_LOGIC_FRAME / 2);
    assert_eq!(half.steps, 0);
    assert_eq!(app.driver().sim().read_view().tick, before);
    // 补齐半帧：恰好 1 个整数帧，余数归零。
    let full = app.advance_with_ticks(TICKS_PER_LOGIC_FRAME / 2);
    assert_eq!(full.steps, 1);
    assert_eq!(app.driver().sim().read_view().tick, before + 1);
    assert!((full.alpha - 0.0).abs() < 1e-6);
}

#[test]
fn host_clock_is_integer_only() {
    use w3_client::HostClock;
    let mut clock = HostClock::new();
    // 首次 poll 为 0（未启动），随后只产出整数 tick，绝不外泄 Instant。
    let first = clock.poll_ticks();
    let second = clock.poll_ticks();
    assert_eq!(first, 0);
    assert!(
        second < TICKS_PER_SECOND,
        "host clock only produces integer ticks"
    );
}

// ---------------------------------------------------------------------------
// B3：输入翻译 → 命令
// ---------------------------------------------------------------------------

#[test]
fn selection_is_presentation_only_and_never_generates_commands() {
    let mut app = build_app();
    let hash_before = hash(&app);
    let tick_before = app.driver().sim().read_view().tick;
    let commands = app
        .apply_input(InputAction::SelectStart([600.0, 340.0]))
        .unwrap();
    assert!(commands.is_empty());
    let commands = app
        .apply_input(InputAction::SelectDrag([700.0, 420.0]))
        .unwrap();
    assert!(commands.is_empty());
    let commands = app
        .apply_input(InputAction::SelectEnd([700.0, 420.0]))
        .unwrap();
    assert!(
        commands.is_empty(),
        "selection MUST NOT generate authoritative commands"
    );
    assert_eq!(hash(&app), hash_before, "selection changed the state hash");
    assert_eq!(app.driver().sim().read_view().tick, tick_before);
}

#[test]
fn context_action_generates_move_with_frozen_frame_and_frozen_coords() {
    let mut app = build_app();
    let mut plain = build_app();
    let view = app.driver().sim().read_view();
    let mapper = app.screen_mapper();
    // 框选全部己方单位（用世界坐标反推的屏幕点覆盖整屏）。
    let world = [0.0, 0.0];
    let center = mapper.world_to_screen(world);
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(!app.selection().is_empty());

    // 取远离单位占位包围盒的空地点，确保右键走 Move 而非 Attack 分支。
    let target_point = [center[0] + 320.0, center[1] + 220.0];
    let commands = app
        .apply_input(InputAction::ContextAction(target_point))
        .unwrap();
    assert!(
        !commands.is_empty(),
        "right click should generate a move command"
    );
    for command in &commands {
        match command {
            ClientCommand::Simulation(Command { kind, .. }) => {
                assert!(matches!(kind, CommandKind::Move { .. }));
            }
            other => panic!("unexpected command: {other:?}"),
        }
        let ClientCommand::Simulation(command) = command else {
            unreachable!()
        };
        // 执行帧由冻结整数帧号推导（MUST NOT 用墙钟 / 呈现帧号）。
        assert_eq!(command.execute_frame.value(), view.tick + LEAD_FRAMES);
        assert_eq!(command.player_id, 1);
        if let CommandKind::Move { x, y, .. } = command.kind {
            // 坐标必须是冻结边界产出的 SimFixed（raw 为 Q47.16 整数）。
            let expected = app.screen_mapper().freeze_point(target_point).unwrap();
            assert_eq!(x, expected.0);
            assert_eq!(y, expected.1);
            let _ = (x.raw(), y.raw());
        }
    }
    // 命令经公开 submit_command 入队会合法改变内核队列计数（参与哈希），
    // 这是权威状态所有者的行为，不是表现层反写；此处只断言命令结构正确。
    // 「表现层只读」由框选 / HUD / 相机三类不改哈希的验收覆盖。
    plain.advance_with_ticks(0);
    assert_eq!(hash(&plain), hash(&build_app()));
}

#[test]
fn gameplay_actions_generate_cast_train_cancel_train() {
    let mut app = build_app();
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();

    let train = app.apply_input(InputAction::Train).unwrap();
    assert_eq!(train.len(), 1);
    assert!(matches!(
        train[0],
        ClientCommand::Gameplay(ref c) if matches!(c.kind, GameplayCommandKind::Train { .. })
    ));

    let cancel = app.apply_input(InputAction::CancelTrain).unwrap();
    assert!(matches!(
        cancel[0],
        ClientCommand::Gameplay(ref c) if matches!(c.kind, GameplayCommandKind::CancelTrain { .. })
    ));

    // 施法：以敌方单位屏幕点为目标的拾取。
    let view = app.driver().sim().read_view();
    let mapper = app.screen_mapper();
    let enemy = view
        .entities
        .iter()
        .find(|e| e.owner == 2)
        .expect("enemy unit exists");
    let enemy_screen =
        mapper.world_to_screen([enemy.x as f64 / SCALE as f64, enemy.y as f64 / SCALE as f64]);
    let cast = app.apply_input(InputAction::Cast(enemy_screen)).unwrap();
    assert!(matches!(
        cast[0],
        ClientCommand::Gameplay(ref c) if matches!(c.kind, GameplayCommandKind::Cast { .. })
    ));
}

#[test]
fn input_errors_are_explicit_not_silent() {
    let mut app = build_app();
    // 无选中单位：需要施法者 / 生产者的动作显式报错。
    assert_eq!(
        app.apply_input(InputAction::Train),
        Err(w3_client::ClientError::Input(InputError::NoSelection))
    );
    assert_eq!(
        app.apply_input(InputAction::CancelTrain),
        Err(w3_client::ClientError::Input(InputError::NoSelection))
    );
    assert_eq!(
        app.apply_input(InputAction::ContextAction([640.0, 360.0])),
        Err(w3_client::ClientError::Input(InputError::NoSelection))
    );
    // 非法屏幕坐标经冻结边界显式拒绝。
    let bad = ScreenMapper::new(-1.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    assert_eq!(
        bad.freeze_point([f64::NAN, 0.0]),
        Err(InputError::InvalidScreenPoint)
    );
}

#[test]
fn frame_in_past_submission_is_an_explicit_error() {
    let mut app = build_app();
    // 构造一条执行帧早于当前冻结帧的命令：内核 MUST 显式拒绝，不静默降级。
    let stale = Command::new(Frame::new(0), 1, 99, CommandKind::Noop);
    let result = app.driver_mut().enqueue(ClientCommand::Simulation(stale));
    assert!(
        result.is_err(),
        "frame in the past must be rejected explicitly"
    );
    assert_eq!(
        result.unwrap_err(),
        ClientError::Command(CommandError::FrameInPast)
    );
    assert!(EntityId::new(1).is_some());
}

#[test]
fn commands_carry_frozen_execute_frame_player_and_sequence() {
    let mut app = build_app();
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    let tick = app.driver().sim().read_view().tick;
    let train = app.apply_input(InputAction::Train).unwrap();
    let ClientCommand::Gameplay(command) = &train[0] else {
        panic!("expected gameplay command")
    };
    assert_eq!(command.execute_frame, Frame::new(tick + LEAD_FRAMES));
    assert_eq!(command.player_id, 1);
    assert!(command.sequence >= 1);
    // 再提交可正常入队（只经公开 API）。
    assert!(
        app.driver_mut()
            .enqueue(ClientCommand::Gameplay(*command))
            .is_ok()
    );
}

// ---------------------------------------------------------------------------
// 拾取映射与呈现投影一致（真实窗口框选 / 命中验收的 CPU 回归）
// ---------------------------------------------------------------------------

#[test]
fn selection_uses_the_render_projection() {
    let mut app = build_app();
    let view = app.driver().sim().read_view();
    let entity = view.entities.first().expect("synthetic unit exists");
    let world = [
        entity.x as f64 / SCALE as f64,
        entity.y as f64 / SCALE as f64,
    ];

    let projected = app.screen_mapper().world_to_screen(world);
    let linear = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).world_to_screen(world);
    assert!(
        (projected[0] - linear[0]).abs() > 1.0 || (projected[1] - linear[1]).abs() > 1.0,
        "注入的精确投影必须与旧线性映射不同"
    );

    // 按「渲染投影算出的单位屏幕点」拖一个小框，必须命中该单位。
    app.apply_input(InputAction::SelectStart([
        projected[0] - 8.0,
        projected[1] - 8.0,
    ]))
    .unwrap();
    app.apply_input(InputAction::SelectEnd([
        projected[0] + 8.0,
        projected[1] + 8.0,
    ]))
    .unwrap();
    assert!(
        app.selection().contains(&entity.id),
        "框选按呈现投影计算的单位屏幕点应命中该单位"
    );
}

// ---------------------------------------------------------------------------
// 高 DPI 坐标空间一致性 / 玩法定义生效（第四轮真窗口反馈的 CPU 回归）
// ---------------------------------------------------------------------------

/// 复现用户环境：物理 2880×1620、逻辑 1920×1080、`scale_factor = 1.5`。
fn dpi_mapper(physical: [f64; 2], logical: [f64; 2]) -> ScreenMapper {
    let terrain = synthetic_terrain();
    let aspect = (physical[0] / physical[1]) as f32;
    let camera = Camera::new(aspect);
    let projection =
        ScreenProjection::new(&camera, &terrain, physical[0] as f32, physical[1] as f32);
    let extent = (terrain.columns.max(terrain.rows) as f64 * terrain.cell_size as f64).max(1.0);
    ScreenMapper::new(physical[0], physical[1], extent, [0.0, 0.0], 1.0)
        .with_projection(projection)
        .with_pixels_per_point(physical[0] / logical[0])
}

#[test]
fn logical_coordinates_are_physical_divided_by_scale_factor() {
    let physical = [2880.0, 1620.0];
    let logical = [1920.0, 1080.0];
    let mapper = dpi_mapper(physical, logical);
    assert_eq!(mapper.logical_size(), logical);

    let world = [8.0, -4.0];
    let physical_point = mapper.world_to_screen(world);
    let logical_point = mapper.to_logical(physical_point);
    assert!((logical_point[0] - physical_point[0] / 1.5).abs() < 1e-9);
    assert!((logical_point[1] - physical_point[1] / 1.5).abs() < 1e-9);

    // 与「直接按逻辑视口尺寸构造的投影」逐位一致：证明 HUD 逻辑坐标
    // 与 3D 呈现在同一逻辑视口内对齐，不再整体偏移 1.5 倍。
    let terrain = synthetic_terrain();
    let camera = Camera::new((logical[0] / logical[1]) as f32);
    let logical_projection =
        ScreenProjection::new(&camera, &terrain, logical[0] as f32, logical[1] as f32);
    let expected = logical_projection.world_to_screen(world);
    assert!((logical_point[0] - expected[0]).abs() < 1e-6);
    assert!((logical_point[1] - expected[1]).abs() < 1e-6);

    let bounds = mapper.unit_screen_bounds(world);
    let (logical_min, logical_max) = mapper.to_logical_bounds(bounds);
    assert!((logical_min[0] - bounds.0[0] / 1.5).abs() < 1e-9);
    assert!((logical_max[1] - bounds.1[1] / 1.5).abs() < 1e-9);
}

#[test]
fn hud_marks_all_alive_units_in_logical_coordinates() {
    let app = build_app();
    let view: WorldView = app.driver().sim().read_view();
    let snap = snapshot(&app);
    let mapper = dpi_mapper([2880.0, 1620.0], [1920.0, 1080.0]);
    let events: Vec<Event> = Vec::new();
    let selection = std::collections::BTreeSet::new();

    let model = HudModel::collect(&view, &snap, &selection, &events, &mapper, 1);
    // 所有存活单位（含未选中的敌方）都必须有 HUD 标记，拾取对象才可见。
    assert_eq!(model.units.len(), view.entities.len());
    assert!(model.units.iter().any(|unit| unit.owner == 2));
    assert_eq!(model.local_player, 1);

    for unit in &model.units {
        let entity = view
            .entities
            .iter()
            .find(|entity| entity.id == unit.id)
            .expect("unit exists in view");
        let world = [
            entity.x as f64 / SCALE as f64,
            entity.y as f64 / SCALE as f64,
        ];
        let (_, expected_max) = mapper.to_logical_bounds(mapper.unit_screen_bounds(world));
        assert!(
            (f64::from(unit.bounds_max[0]) - expected_max[0]).abs() < 1e-3,
            "HUD 包围盒必须是逻辑坐标"
        );
        assert!(
            (f64::from(unit.bounds_max[1]) - expected_max[1]).abs() < 1e-3,
            "HUD 包围盒必须是逻辑坐标"
        );
    }
}

#[test]
fn physical_mouse_picks_enemy_under_high_dpi() {
    let mapper = dpi_mapper([2880.0, 1620.0], [1920.0, 1080.0]);
    let app = build_app();
    let view: WorldView = app.driver().sim().read_view();
    let events: Vec<Event> = Vec::new();
    let mut input = InputState::new(1);

    // 全屏框选（物理像素），选中己方单位。
    input
        .apply(
            InputAction::SelectStart([0.0, 0.0]),
            &view,
            &mapper,
            &events,
        )
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([2880.0, 1620.0]),
            &view,
            &mapper,
            &events,
        )
        .unwrap();
    assert!(!input.selection().is_empty());

    // 点敌方单位包围盒中心（物理像素），Cast 必须生成玩法命令。
    let enemy = view
        .entities
        .iter()
        .find(|entity| entity.owner == 2)
        .expect("enemy exists");
    let world = [enemy.x as f64 / SCALE as f64, enemy.y as f64 / SCALE as f64];
    let (min, max) = mapper.unit_screen_bounds(world);
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let commands = input
        .apply(InputAction::Cast(center), &view, &mapper, &events)
        .unwrap();
    assert!(matches!(
        commands[0],
        ClientCommand::Gameplay(ref command)
            if matches!(command.kind, GameplayCommandKind::Cast { .. })
    ));
}

#[test]
fn frozen_definitions_make_train_and_cast_effective() {
    let unit = UnitDef {
        hp: 100,
        mana: 100,
        speed_raw: SimFixed::from_int(3).unwrap().raw(),
        damage: 10,
        range_raw: SimFixed::from_int(10).unwrap().raw(),
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
    // 与 example 的 `synthetic_defs` 同构：unit_def 1 / ability 10。
    let defs = FrozenDefinitions::new(vec![(1, unit)], vec![(10, ability)], Vec::new(), Vec::new())
        .unwrap();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: CONTENT_HASH,
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs);
    sim.submit_command(spawn(0, 0, 1)).unwrap();
    sim.submit_command(spawn(1, 0, 2)).unwrap();
    sim.step();

    let caster = EntityId::new(1).unwrap();
    let target = EntityId::new(2).unwrap();
    let frame = Frame::new(sim.read_view().tick + 1);
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        1,
        1,
        GameplayCommandKind::Train {
            producer: caster,
            unit_def: 1,
            is_building: false,
        },
    ))
    .unwrap();
    sim.submit_gameplay_command(GameplayCommand::new(
        frame,
        1,
        2,
        GameplayCommandKind::Cast {
            caster,
            ability: 10,
            target,
        },
    ))
    .unwrap();
    // 命令执行帧为 `tick + 1`，需推进两帧才会被内核执行。
    for _ in 0..2 {
        sim.step();
    }

    let snap = sim.capture_snapshot();
    assert!(
        snap.positions
            .iter()
            .any(|position| !position.gameplay.production.is_empty()),
        "注入冻结定义后 T 生产必须写入生产队列"
    );
    assert!(
        snap.positions
            .iter()
            .any(|position| position.gameplay.cast.is_some()),
        "注入冻结定义后 Q 施法必须进入引导运行态"
    );
}

#[test]
fn client_app_train_command_reaches_the_kernel() {
    let unit = UnitDef {
        hp: 100,
        mana: 100,
        speed_raw: SimFixed::from_int(3).unwrap().raw(),
        damage: 10,
        range_raw: SimFixed::from_int(10).unwrap().raw(),
        supply: 1,
        train_frames: 30,
    };
    let defs = FrozenDefinitions::new(vec![(1, unit)], Vec::new(), Vec::new(), Vec::new()).unwrap();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: CONTENT_HASH,
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs);
    sim.submit_command(spawn(0, 0, 1)).unwrap();
    let driver = FixedStepClient::from_match(sim);
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 100);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

    // 真窗口路径：框选 → 按 T → 命令经 ClientApp 提交 → 内核生效。
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    let commands = app.apply_input(InputAction::Train).unwrap();
    assert!(matches!(
        commands[0],
        ClientCommand::Gameplay(ref command)
            if matches!(command.kind, GameplayCommandKind::Train { .. })
    ));
    for _ in 0..3 {
        app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
    }
    let snap = app.driver().sim().capture_snapshot();
    assert!(
        snap.positions
            .iter()
            .any(|position| !position.gameplay.production.is_empty()),
        "T 生产命令必须经 ClientApp 提交并在内核生效"
    );
}

/// 单一真值源回归：注入真实模型屏幕包围盒后，框选命中与 HUD 标记锚点都
/// 以它为准，替代旧 `unit_screen_bounds`（24 世界单位四面体近全屏巨框）。
/// 复现用户「选框不贴合模型、选不上」的问题域：box 中心点应精确命中，
/// box 外的点（即便落在旧巨框内）不应命中。
#[test]
fn entity_screen_boxes_drive_selection_and_hud_center() {
    use w3_core::EntityView;

    // 单个己方实体，逻辑坐标 (0,0)。
    let view = WorldView {
        tick: 0,
        rng: 0,
        entities: vec![EntityView {
            id: 1,
            x: 0,
            y: 0,
            facing: 0,
            hp: 100,
            owner: 1,
            alive: true,
            has_target: false,
        }],
    };
    let events: Vec<Event> = Vec::new();

    // 真实模型投影出的紧凑 box（模拟 Scene::entity_screen_boxes 产物）。
    let model_box: ([f64; 2], [f64; 2]) = ([600.0, 380.0], [640.0, 440.0]);
    let mut boxes = std::collections::BTreeMap::new();
    boxes.insert(1u64, model_box);
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0)
        .with_projection(ScreenProjection::new(
            &Camera::new(1280.0 / 720.0),
            &synthetic_terrain(),
            1280.0,
            720.0,
        ))
        .with_entity_boxes(boxes);

    // ① HUD 标记中心 / 命中包围盒 = 注入的真实 box（非旧口径）。
    let center = mapper.entity_screen_center(1, [0.0, 0.0]);
    assert!((center[0] - 620.0).abs() < 1.0e-6);
    assert!((center[1] - 410.0).abs() < 1.0e-6);
    let (min, max) = mapper.entity_screen_box(1, [0.0, 0.0]);
    assert_eq!(min, model_box.0);
    assert_eq!(max, model_box.1);
    // 旧 HUD 锚点 `world_to_screen`（低空四面体平面）与真实模型 box 中心
    // 明显不同——证明注入后确实换了真值源、不再用错位旧口径。
    let old_anchor = mapper.world_to_screen([0.0, 0.0]);
    assert!(
        (old_anchor[0] - center[0]).hypot(old_anchor[1] - center[1]) > 1.0,
        "真实模型中心不应等于旧低空平面锚点 old={old_anchor:?} new={center:?}"
    );

    // ② 在真实 box 中心单击 → 选中该实体。
    let mut input = InputState::new(1);
    input
        .apply(InputAction::SelectStart(center), &view, &mapper, &events)
        .unwrap();
    input
        .apply(InputAction::SelectEnd(center), &view, &mapper, &events)
        .unwrap();
    assert!(input.selection().contains(&1), "box 中心单击应选中实体");

    // ③ 在真实 box 外的远点单击 → 不应选中（旧四面体巨框会误中，现已消除）。
    let outside = [min[0] - 60.0, min[1] - 60.0];
    assert!(
        outside[0] < min[0] || outside[0] > max[0] || outside[1] < min[1] || outside[1] > max[1],
        "该点必须落在真实 box 之外"
    );
    let mut input2 = InputState::new(1);
    input2
        .apply(InputAction::SelectStart(outside), &view, &mapper, &events)
        .unwrap();
    input2
        .apply(InputAction::SelectEnd(outside), &view, &mapper, &events)
        .unwrap();
    assert!(
        !input2.selection().contains(&1),
        "真实 box 外的点不应选中（旧四面体巨框会误中，现已消除）"
    );
}

/// W1 回归：已注入 boxes 但某实体缺 id（首帧前 scene 空、运行时新实体尚未
/// consume、相机背后被投影过滤）时，`entity_screen_box` 不得回退旧的 24 单位
/// 四面体巨框（否则一框选中全屏、B10③ 复现），应收敛为紧致 box。
#[test]
fn entity_screen_box_missing_id_never_falls_back_to_giant_box() {
    use w3_core::EntityView;

    // 只给 id=2 注入真实 box；id=1 故意缺 id。
    let mut boxes = std::collections::BTreeMap::new();
    boxes.insert(2u64, ([600.0, 380.0], [640.0, 440.0]));
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0)
        .with_projection(ScreenProjection::new(
            &Camera::new(1280.0 / 720.0),
            &synthetic_terrain(),
            1280.0,
            720.0,
        ))
        .with_entity_boxes(boxes);

    // 缺 id 的 box 紧致（宽高 = 2*PICK_RADIUS=48），远小于旧巨框。
    let (min, max) = mapper.entity_screen_box(1, [0.0, 0.0]);
    assert!(
        (max[0] - min[0]) <= 48.0 && (max[1] - min[1]) <= 48.0,
        "缺 id 不得回退旧四面体巨框，实际 {}x{}",
        max[0] - min[0],
        max[1] - min[1]
    );
    // 未注入 boxes（合成 mapper）仍走旧口径，既有单测零改动。
    let bare = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let (bmin, bmax) = bare.entity_screen_box(1, [0.0, 0.0]);
    let (omin, omax) = bare.unit_screen_bounds([0.0, 0.0]);
    assert_eq!(bmin, omin);
    assert_eq!(bmax, omax);

    // 端到端：缺 id 实体不会因巨框被「隔空」拖框误选。
    let view = WorldView {
        tick: 0,
        rng: 0,
        entities: vec![EntityView {
            id: 1,
            x: 0,
            y: 0,
            facing: 0,
            hp: 100,
            owner: 1,
            alive: true,
            has_target: false,
        }],
    };
    let events: Vec<Event> = Vec::new();
    let far = [50.0, 50.0]; // 远离逻辑点中心的屏幕角落
    let mut input = InputState::new(1);
    input
        .apply(InputAction::SelectStart(far), &view, &mapper, &events)
        .unwrap();
    input
        .apply(InputAction::SelectEnd(far), &view, &mapper, &events)
        .unwrap();
    assert!(
        !input.selection().contains(&1),
        "缺 id 实体不得被远处拖框因巨框误选"
    );
}

/// W3 回归：HUD 真的消费注入的 box（端到端），而非仅 mapper 内部自洽。
/// 断言 `HudModel::collect` 产出的单位 bounds_min/max 等于注入 box（经
/// to_logical 换算）；若 HUD 仍读旧锚点 / 旧巨框，此断言失败。
#[test]
fn hud_collect_uses_injected_entity_box_bounds() {
    let app = build_app();
    let view: WorldView = app.driver().sim().read_view();
    let snap = snapshot(&app);
    let entity_id = view.entities[0].id;

    let injected: ([f64; 2], [f64; 2]) = ([300.0, 200.0], [340.0, 260.0]);
    let mut boxes = std::collections::BTreeMap::new();
    boxes.insert(entity_id, injected);
    // pixels_per_point=2.0 → 逻辑坐标为物理坐标的一半。
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0)
        .with_projection(ScreenProjection::new(
            &Camera::new(1280.0 / 720.0),
            &synthetic_terrain(),
            1280.0,
            720.0,
        ))
        .with_pixels_per_point(2.0)
        .with_entity_boxes(boxes);

    let hud = HudModel::collect(&view, &snap, &Default::default(), &[], &mapper, 1);
    let unit = hud.units.iter().find(|u| u.id == entity_id).unwrap();
    assert!(
        (unit.bounds_min[0] - 150.0).abs() < 1.0
            && (unit.bounds_min[1] - 100.0).abs() < 1.0
            && (unit.bounds_max[0] - 170.0).abs() < 1.0
            && (unit.bounds_max[1] - 130.0).abs() < 1.0,
        "HUD bounds 应等于注入 box 的 to_logical，实际 min={:?} max={:?}",
        unit.bounds_min,
        unit.bounds_max
    );
    // 中心同样来自注入 box（物理 [320,230] → 逻辑 [160,115]）。
    assert!((unit.screen[0] - 160.0).abs() < 1.0);
    assert!((unit.screen[1] - 115.0).abs() < 1.0);
}
