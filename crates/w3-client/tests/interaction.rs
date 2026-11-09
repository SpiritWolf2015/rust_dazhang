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
    HudModel, InputAction, InputError, LEAD_FRAMES, ScreenMapper, StepDriver,
    TICKS_PER_LOGIC_FRAME, TICKS_PER_SECOND, ZH,
};
use w3_core::{
    Command, CommandError, CommandKind, EntityId, Event, Frame, GameplayCommandKind, SCALE,
    SNAPSHOT_VERSION, SimFixed, Snapshot, StaticWorld, WorldView,
};
use w3_render::{ModelLibrary, TerrainMesh, TerrainVertex};

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
    // 当前 egui 依赖树未启用 `default_fonts`，内置字体数据为空：这正是
    // 真实部署下字体资源不可用的路径，宿主会记录英文日志并降级。
    // （是否启用带 OFL/Ubuntu 字体许可证的 default_fonts 属人工独占，见回报。）
    assert_eq!(HudAssets::prepare(&ctx), Err(HudAssetError::EmptyFontData));

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
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
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
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    // 框选全部己方单位（用世界坐标反推的屏幕点覆盖整屏）。
    let world = [0.0, 0.0];
    let center = mapper.world_to_screen(world);
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(!app.selection().is_empty());

    let target_point = [center[0] + 60.0, center[1] + 30.0];
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
            let expected = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0)
                .freeze_point(target_point)
                .unwrap();
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
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
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
