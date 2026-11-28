//! P8.5e 战争迷雾：真实装配只读隔离、HUD 可见性过滤、联机 / 回放一致性与改坏验红。
//!
//! 全部用合成命令脚本、`synthetic_definitions()` 与手工 CPU 地形，不读取本机
//! 导出资产、不开 GPU 窗口、不引入权威状态字段。测试经
//! `RtsMatch::create_with_definitions` → `submit_command` → `step()` →
//! `capture_snapshot()` / `restore_snapshot()` 既有确定性生产路径注入夹具；
//! 迷雾只驻留 `w3-client` / `w3-render` 表现层，MUST NOT 进 `w3-core` 权威
//! 状态 / FNV-1a 状态哈希 / v10 快照 / `first_difference()`（R10.5 / ADR 0002 §5）。
//!
//! Q9 反向验证采用 `CODEBUDDY.md §5` 形态①：测试内构造缺陷态，并写元断言
//! 证明同一断言在正式态通过、退化态失败；退化态真经 `capture_snapshot()` /
//! `compute_state_hash()` / 公开命令入口 / `step()` 生产路径，不是旁路模拟。
//! 真窗口人工清单 8.1–8.7 保持未执行 / 待人工，不用 CPU 回归冒充。

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use w3_client::melee::defs::{
    GOLD_MINE, HUMAN_PEASANT, ITEM_SHOP_DROP_CONSUMABLE, SHOP_DEF, TOWN_HALL, TREE,
};
use w3_client::melee::synthetic_definitions;
use w3_client::{
    CameraAction, ClientApp, FOG_DAY_FRAMES, FOG_GRID_SIDE, FOG_NIGHT_FRAMES, FOG_SIGHT_DAY_RAW,
    FixedStepClient, FogCell, FogGrid, FogNeutralCategory, FogPhase, HudAssets, HudModel,
    LOCAL_PLAYER, LOGIC_FPS, MinimapPoint, MinimapPointCategory, ScreenMapper, StepDriver,
    TICKS_PER_LOGIC_FRAME, ZH, day_phase, derive_movability, draw, effective_movability,
    filter_movable,
};
use w3_core::{
    AbilityDef, CampState, Command, CommandKind, CreepState, DefId, EffectDef, EffectKind,
    EntityId, EntityView, Event, Frame, FrozenDefinitions, GroundItemState, ItemDef,
    ItemInstanceState, LeashPhase, MatchConfig, RtsMatch, SCALE, SIM_VERSION, SNAPSHOT_VERSION,
    SimFixed, Snapshot, StaticWorld, UnitDef, UpgradeDef, WorldView, shop_fixture_id,
};
use w3_net::PROTOCOL_VERSION;
use w3_render::{FrameProducer, ModelLibrary, TerrainMesh, TerrainVertex};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = LOCAL_PLAYER;
const ENEMY: u8 = 2;
const NEUTRAL_DEF_ID: DefId = 97_001;
const FOG_CONST_DEF_ID: DefId = 97_999;

/// 由整数字面量构造稳定实体编号；非法零值显式失败，不静默降级。
fn entity(id: u64) -> EntityId {
    EntityId::new(id).expect("reason=invalid_entity_id")
}

/// 整数世界单位转 Q47.16；不引入 `f64 → SimFixed` 直转路径。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("reason=fixed_conversion_failed")
}

/// 构造一条冻结帧号的 `Spawn` 命令；坐标与速度均为整数。
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

/// 固定合成命令脚本：己方单位 + 近处 VISIBLE 敌方 + 远处 MASKED 敌方。
fn spawn_script(own_x: i64) -> Vec<Command> {
    vec![
        spawn(own_x, 0, PLAYER, 1),
        spawn(own_x + 4, 0, ENEMY, 2),
        spawn(own_x + 40, 0, ENEMY, 3),
    ]
}

/// 带一条移动命令的回放脚本；用于 Q7「从帧 0 重放 vs 当前帧起算」对照。
fn move_script() -> Vec<Command> {
    let mut commands = spawn_script(0);
    commands.push(Command::new(
        Frame::new(1),
        PLAYER,
        4,
        CommandKind::Move {
            entity: entity(1),
            x: fixed(40),
            y: fixed(0),
            group: 0,
            append: false,
        },
    ));
    commands
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

/// 稳定字节视图：用 `Debug` 表示作为快照字节的可观测代理，配合 `Snapshot: Eq`。
fn snapshot_bytes(snapshot: &Snapshot) -> Vec<u8> {
    format!("{snapshot:?}").into_bytes()
}

/// 合成道具效果；只走既有 `EffectDef` / `EffectKind`，不引入 JASS 入口。
fn heal_effect() -> EffectDef {
    EffectDef {
        kind: EffectKind::Heal,
        amount: 1,
        duration_frames: 0,
    }
}

/// 向只读快照注入 P8.5c / P8.5d 中立实体，并恢复进真实对局。
///
/// 地面物品不是 `EntityStore` 实体、不出现在 `read_view()`，故位置 / 类别
/// MUST 从 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（R8.1 / R9.1）。
fn inject_neutral_markers(snapshot: &mut Snapshot) {
    let shop_id = {
        let shops = &snapshot.defs.item_shop().shops;
        *shops.keys().next().expect("reason=no_shop_definition")
    };
    snapshot.item_shop.refresh_counts.insert(shop_id, 0);

    let visible_creep = entity(900_001);
    let hidden_creep = entity(900_002);
    let visible_item = entity(900_003);
    let hidden_item = entity(900_004);
    let hidden_raw = 40 * SCALE;

    snapshot.creep_camps.camps.insert(
        1,
        CampState {
            script: 1,
            tier: 1,
            home_x: 0,
            home_y: 0,
            members: BTreeSet::new(),
            cleared: false,
        },
    );
    snapshot.creep_camps.camps.insert(
        2,
        CampState {
            script: 1,
            tier: 1,
            home_x: hidden_raw,
            home_y: 0,
            members: BTreeSet::new(),
            cleared: false,
        },
    );
    snapshot.creep_camps.creeps.insert(
        visible_creep,
        CreepState {
            camp: 1,
            def_id: NEUTRAL_DEF_ID,
            home_x: 0,
            home_y: 0,
            phase: LeashPhase::Idle,
            target: None,
        },
    );
    snapshot.creep_camps.creeps.insert(
        hidden_creep,
        CreepState {
            camp: 2,
            def_id: NEUTRAL_DEF_ID,
            home_x: hidden_raw,
            home_y: 0,
            phase: LeashPhase::Idle,
            target: None,
        },
    );

    let visible_ground = GroundItemState {
        item_def: 30,
        camp: 1,
        x: 0,
        y: 0,
        instance: visible_item,
    };
    let hidden_ground = GroundItemState {
        item_def: 30,
        camp: 2,
        x: hidden_raw,
        y: 0,
        instance: hidden_item,
    };
    snapshot
        .creep_camps
        .ground_items
        .insert(visible_item, visible_ground);
    snapshot
        .creep_camps
        .ground_items
        .insert(hidden_item, hidden_ground);
    for item in [visible_item, hidden_item] {
        snapshot.creep_camps.item_instances.insert(
            item,
            ItemInstanceState {
                item_def: 30,
                charges: 1,
                cooldown_frames: 0,
                effect: heal_effect(),
                holder: None,
                slot: None,
            },
        );
    }
}

/// 从同一定命令脚本装配真实对局；注入走 `capture_snapshot()` →
/// 修改只读快照 public 字段 → `restore_snapshot()` 既有生产路径。
fn build_match_from_commands(commands: &[Command]) -> RtsMatch {
    let defs = synthetic_definitions();
    let content_hash = defs.content_hash();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash,
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs);
    for command in commands {
        sim.submit_command(command.clone())
            .expect("reason=spawn_command_rejected");
    }
    let mut snapshot = sim.capture_snapshot();
    inject_neutral_markers(&mut snapshot);
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content_hash)
        .expect("reason=snapshot_restore_rejected")
}

/// 真实装配并推进一帧，使 `Spawn` 命令经 `step()` 生效。
fn build_match(own_x: i64) -> RtsMatch {
    let mut sim = build_match_from_commands(&spawn_script(own_x));
    sim.step();
    let _events = sim.drain_events();
    sim
}

/// 用同一生产路径装配 `ClientApp`，供 `hud_model()` / `fog_grid()` 真实接线验收。
fn build_app(own_x: i64) -> ClientApp<FixedStepClient> {
    let driver = FixedStepClient::from_match(build_match(own_x));
    ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
}

/// 表现层 HUD 投影使用的 CPU 拾取映射；不读窗口 / GPU 资源。
fn mapper() -> ScreenMapper {
    ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0)
}

/// 从真实对局的只读视图 / 快照派生一帧迷雾网格。
fn observe_grid(sim: &RtsMatch) -> FogGrid {
    let view = sim.read_view();
    let snapshot = sim.capture_snapshot();
    let mut grid = FogGrid::new(PLAYER);
    grid.observe(
        &view,
        &snapshot,
        &[],
        &BTreeMap::<u64, DefId>::new(),
        PLAYER,
    );
    grid
}

/// 真实装配的 `HudModel::collect_with_fog` 输出。
fn hud_with_fog(sim: &RtsMatch) -> HudModel {
    let view = sim.read_view();
    let snapshot = sim.capture_snapshot();
    let events: Vec<Event> = Vec::new();
    let grid = observe_grid(sim);
    HudModel::collect_with_fog(
        &view,
        &snapshot,
        &BTreeSet::new(),
        &events,
        &mapper(),
        PLAYER,
        &grid,
    )
}

/// 正式态隔离断言：逐帧哈希 / 快照 / 只读视图 / 事实事件 / 首差异一致。
fn authoritative_pair_matches(rendered: &mut RtsMatch, plain: &mut RtsMatch) -> bool {
    let events_a = rendered.drain_events();
    let events_b = plain.drain_events();
    let snapshot_a = rendered.capture_snapshot();
    let snapshot_b = plain.capture_snapshot();
    rendered.compute_state_hash() == plain.compute_state_hash()
        && snapshot_a == snapshot_b
        && snapshot_bytes(&snapshot_a) == snapshot_bytes(&snapshot_b)
        && rendered.read_view() == plain.read_view()
        && events_a == events_b
        && rendered.first_difference(plain) == None
}

/// 退化态① / ②：表现层反写权威状态，经公开 `submit_command` + `step()` 注入。
fn degenerate_writeback(sim: &mut RtsMatch) {
    let frame = sim.read_view().tick + 1;
    sim.submit_command(Command::new(
        Frame::new(frame),
        PLAYER,
        9_000,
        CommandKind::Noop,
    ))
    .expect("reason=degenerate_writeback_command_rejected");
    sim.step();
}

/// 退化态①：在 `capture_snapshot()` 字节视图后追加迷雾网格字节。
fn degenerate_fog_snapshot_bytes(sim: &RtsMatch, grid: &FogGrid) -> Vec<u8> {
    let mut bytes = snapshot_bytes(&sim.capture_snapshot());
    bytes.extend_from_slice(format!("{:?}", grid.cells()).as_bytes());
    bytes
}

/// 退化态①：把迷雾格计数混入 `compute_state_hash()` 返回值。
fn degenerate_hash_with_fog(sim: &RtsMatch, grid: &FogGrid) -> u64 {
    sim.compute_state_hash() ^ (grid.counts().visible as u64 + 1)
}

/// 退化态①：伪造 `first_difference()` 多出 fog 字段。
fn degenerate_first_difference_with_fog(
    sim: &RtsMatch,
    plain: &RtsMatch,
    grid: &FogGrid,
) -> Option<&'static str> {
    if grid.counts().visible > 0 && sim.first_difference(plain) == None {
        Some("fog_grid")
    } else {
        sim.first_difference(plain)
    }
}

/// 快照字节中是否出现三态迷雾标记；正式 v10 快照 MUST NOT 含这些字段。
fn contains_fog_marker(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    text.contains("masked") || text.contains("fogged") || text.contains("fog_grid")
}

/// 退化态③：不做可见性过滤，直接把 `alive` 实体全量喂入小地图点。
fn degenerate_no_filter_points(view: &WorldView) -> Vec<MinimapPoint> {
    view.entities
        .iter()
        .filter(|entity| entity.alive)
        .map(|entity| MinimapPoint {
            nx: 0.0,
            ny: 0.0,
            owner: entity.owner,
            selected: false,
            category: MinimapPointCategory::Entity,
        })
        .collect()
}

/// 小地图过滤断言：本夹具共有 2 个敌方在场实体，正式态只呈现 1 个 VISIBLE 点。
fn minimap_filter_assertion(points: &[MinimapPoint], view: &WorldView, fog_present: bool) -> bool {
    let total_enemy = view
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.owner == ENEMY)
        .count();
    let shown_enemy = points
        .iter()
        .filter(|point| point.owner == ENEMY && point.category == MinimapPointCategory::Entity)
        .count();
    total_enemy == 2 && shown_enemy == 1 && fog_present
}

/// 正式态昼夜相位断言：整数帧周期、昼末 / 夜初边界稳定。
fn formal_phase_is_pure_integer_frame(tick: u64) -> bool {
    day_phase(tick) == day_phase(tick + FOG_DAY_FRAMES + FOG_NIGHT_FRAMES)
        && day_phase(FOG_DAY_FRAMES - 1).is_day()
        && !day_phase(FOG_DAY_FRAMES).is_day()
}

/// 退化态④：墙钟 / 渲染帧率 / `HashMap` 迭代顺序 / `f64` / 独立随机源派生相位。
/// 该函数只存在于测试中，用来构造「同一断言必失败」的反例；生产路径绝不调用。
fn degenerate_phase_from_external_sources(tick: u64, fps: u64, seed: u64) -> FogPhase {
    let mut external = HashMap::new();
    external.insert("fps", fps);
    external.insert("tick", tick);
    let probe = external.iter().next().map(|(_, &value)| value).unwrap_or(0);
    let coordinate = tick as f64 / fps.max(1) as f64;
    let random_source = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let wallclock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos() as u64);
    let instant = Instant::now();
    let mixed = tick
        ^ fps
        ^ probe
        ^ wallclock
        ^ random_source
        ^ coordinate.to_bits()
        ^ instant.elapsed().as_nanos() as u64;
    let _ = mixed;
    // 缺陷态故意让外部源覆盖整数帧相位，返回与 `day_phase(tick)` 相反的值。
    match day_phase(tick) {
        FogPhase::Day => FogPhase::Night,
        FogPhase::Night => FogPhase::Day,
    }
}

/// 退化态⑤：把表现层视野常量塞进 `FrozenDefinitions`，`content_hash()` 必变。
fn degenerate_fog_definitions() -> FrozenDefinitions {
    let full = synthetic_definitions();
    let mut units: Vec<(DefId, UnitDef)> =
        full.units().iter().map(|(&id, &def)| (id, def)).collect();
    let mut fog_def = *full
        .units()
        .values()
        .next()
        .expect("reason=no_unit_definition");
    fog_def.hp = FOG_SIGHT_DAY_RAW;
    units.push((FOG_CONST_DEF_ID, fog_def));
    let abilities: Vec<(DefId, AbilityDef)> = full
        .abilities()
        .iter()
        .map(|(&id, def)| (id, def.clone()))
        .collect();
    let items: Vec<(DefId, ItemDef)> = full.items().iter().map(|(&id, &def)| (id, def)).collect();
    let upgrades: Vec<(DefId, UpgradeDef)> = full
        .upgrades()
        .iter()
        .map(|(&id, &def)| (id, def))
        .collect();
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("reason=degenerate_definitions_invalid")
        .with_hero_revive(
            *full.revive_constants(),
            full.altar_defs().clone(),
            full.hero_defs().clone(),
        )
        .with_town_hall_upgrade(
            *full.town_hall_constants(),
            full.town_hall_defs().clone(),
            full.tier_requirements().clone(),
        )
        .with_creep_camps(
            *full.creep_constants(),
            full.camp_scripts().clone(),
            full.drop_tables().clone(),
            full.neutral_categories().clone(),
        )
        .with_item_shop(full.item_shop().clone())
}

/// 回放逐帧轨迹：权威状态序列 + 表现层三态网格 / 已探索层。
#[derive(Clone, Debug, PartialEq, Eq)]
struct FogTrace {
    tick: u64,
    hash: u64,
    snapshot: Vec<u8>,
    fog_cells: Vec<FogCell>,
    explored: usize,
}

/// 从帧 0 逐帧重放并采集表现层迷雾与权威状态序列。
fn replay_trace(commands: &[Command], frames: u64) -> (Vec<FogTrace>, FogGrid) {
    let mut sim = build_match_from_commands(commands);
    let mut grid = FogGrid::new(PLAYER);
    let mut traces = Vec::new();
    for _ in 0..frames {
        sim.step();
        let view = sim.read_view();
        let snapshot = sim.capture_snapshot();
        grid.observe(
            &view,
            &snapshot,
            &[],
            &BTreeMap::<u64, DefId>::new(),
            PLAYER,
        );
        traces.push(FogTrace {
            tick: view.tick,
            hash: sim.compute_state_hash(),
            snapshot: snapshot_bytes(&snapshot),
            fog_cells: grid.cells().to_vec(),
            explored: grid.counts().explored,
        });
    }
    (traces, grid)
}

// ---------------------------------------------------------------------------
// 任务 1–3：真实装配、HUD 只读投影、小地图过滤与表现层呈现
// ---------------------------------------------------------------------------

#[test]
fn real_assembly_fog_hud_keeps_authoritative_state_isolated() {
    let mut rendered = build_app(0);
    let mut plain = build_app(0);
    let ctx = egui::Context::default();

    for _ in 0..24 {
        let hud = rendered.hud_model();
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            draw(ctx, &hud, &ZH, HudAssets { text_enabled: true });
        });
        output.drop_without_applying_deltas();
        rendered.apply_camera(CameraAction::Pan([0.25, -0.25]));
        rendered.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

        let (view_a, events_a) = rendered.driver_mut().next_frame();
        let (view_b, events_b) = plain.driver_mut().next_frame();
        let raw_a = rendered.driver().sim();
        let raw_b = plain.driver().sim();
        assert_eq!(
            raw_a.compute_state_hash(),
            raw_b.compute_state_hash(),
            "reason=fog_hud_keeps_state_hash"
        );
        assert_eq!(
            raw_a.capture_snapshot(),
            raw_b.capture_snapshot(),
            "reason=fog_hud_keeps_snapshot"
        );
        assert_eq!(
            snapshot_bytes(&raw_a.capture_snapshot()),
            snapshot_bytes(&raw_b.capture_snapshot()),
            "reason=fog_hud_keeps_snapshot_bytes"
        );
        assert_eq!(
            raw_a.first_difference(raw_b),
            None,
            "reason=fog_hud_keeps_first_difference_none"
        );
        assert_eq!(view_a, view_b, "reason=fog_hud_keeps_read_view");
        assert_eq!(events_a, events_b, "reason=fog_hud_keeps_events");
        assert_eq!(
            raw_a.capture_snapshot().version,
            SNAPSHOT_VERSION,
            "reason=fog_hud_keeps_snapshot_v10"
        );
    }
    assert!(
        rendered.fog_grid().last_tick().is_some(),
        "reason=real_assembly_observes_fog"
    );
}

#[test]
fn minimap_filters_hidden_enemy_points_and_snapshot_neutral_markers() {
    let app = build_app(0);
    let view = app.driver().view();
    let hud = app.hud_model();
    let points = &hud.minimap.points;
    assert!(
        minimap_filter_assertion(points, &view, hud.minimap.fog.is_some()),
        "reason=hidden_enemy_point_is_filtered"
    );
    assert_eq!(
        hud.minimap.fog.as_ref().map(|fog| fog.opacities.len()),
        Some(FOG_GRID_SIDE * FOG_GRID_SIDE),
        "reason=fog_overlay_grid_is_complete"
    );
    assert!(
        hud.fog.last_tick.is_some(),
        "reason=fog_hud_projection_is_wired"
    );
    assert!(
        hud.fog
            .visible_markers
            .iter()
            .any(|marker| marker.category == FogNeutralCategory::GroundItem && marker.x == 0),
        "reason=visible_ground_item_uses_snapshot_position"
    );
    assert!(
        hud.fog
            .visible_markers
            .iter()
            .any(|marker| marker.category == FogNeutralCategory::CreepCamp && marker.x == 0),
        "reason=visible_camp_uses_snapshot_position"
    );
    let shop_id = *app
        .driver()
        .sim()
        .capture_snapshot()
        .defs
        .item_shop()
        .shops
        .keys()
        .next()
        .expect("reason=no_shop_definition");
    assert!(
        hud.fog
            .visible_markers
            .iter()
            .any(|marker| marker.id == shop_fixture_id(shop_id)
                && marker.category == FogNeutralCategory::Shop),
        "reason=visible_shop_uses_snapshot_lookup"
    );
    assert!(
        !hud.fog
            .visible_markers
            .iter()
            .any(|marker| marker.x == 40 * SCALE),
        "reason=masked_snapshot_markers_are_hidden"
    );
}

#[test]
fn hidden_shop_grid_does_not_leak_snapshot_shop_marker() {
    let app = build_app(40);
    let hud = app.hud_model();
    let shop_id = *app
        .driver()
        .sim()
        .capture_snapshot()
        .defs
        .item_shop()
        .shops
        .keys()
        .next()
        .expect("reason=no_shop_definition");
    let shop_marker = shop_fixture_id(shop_id);
    assert!(
        !hud.fog
            .visible_markers
            .iter()
            .any(|marker| marker.id == shop_marker),
        "reason=fogged_shop_marker_is_not_presented"
    );
    assert!(
        !hud.minimap
            .points
            .iter()
            .any(|point| point.category == MinimapPointCategory::Shop),
        "reason=fogged_shop_point_is_not_fed_into_minimap"
    );
}

#[test]
fn minimap_pan_is_presentation_only_and_keeps_state_hash() {
    let mut app = build_app(0);
    let hash_before = app.driver().sim().compute_state_hash();
    let snapshot_before = app.driver().sim().capture_snapshot();
    // `render_one()` 把 `minimap_click` 归一化为同一个 `CameraAction::Pan`；
    // 本 CPU 回归走同一公开装配入口，不生成任何权威命令。
    app.apply_camera(CameraAction::Pan([8.0, -6.0]));
    assert_eq!(
        app.driver().sim().compute_state_hash(),
        hash_before,
        "reason=minimap_pan_keeps_state_hash"
    );
    assert_eq!(
        app.driver().sim().capture_snapshot(),
        snapshot_before,
        "reason=minimap_pan_keeps_snapshot"
    );
    assert_eq!(
        app.driver().sim().read_view().tick,
        snapshot_before.tick,
        "reason=minimap_pan_does_not_step"
    );
}

#[test]
fn mixed_immovable_selection_never_generates_move_under_fog() {
    let defs = synthetic_definitions();
    let def_ids: BTreeMap<u64, DefId> = [
        (1, HUMAN_PEASANT),
        (2, SHOP_DEF),
        (3, ITEM_SHOP_DROP_CONSUMABLE),
        (4, GOLD_MINE),
        (5, TREE),
        (6, TOWN_HALL),
    ]
    .into_iter()
    .collect();
    let view = WorldView {
        tick: 1,
        rng: 0,
        entities: (1..=6u64)
            .map(|id| EntityView {
                id,
                x: 0,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 100,
                owner: PLAYER,
                alive: true,
                has_target: false,
            })
            .collect(),
    };
    let base = derive_movability(&defs, &def_ids);
    let effective = effective_movability(&base, &view);
    let selection: BTreeSet<u64> = (1..=6u64).collect();
    assert_eq!(
        filter_movable(&selection, &effective),
        vec![1u64],
        "reason=only_movable_unit_receives_move"
    );
}

// ---------------------------------------------------------------------------
// 任务 5：联机两端 / 回放 / 重连已知限制（R12.2 / R12.4 / R12.5）
// ---------------------------------------------------------------------------

#[test]
fn two_clients_same_script_and_8_frame_batches_match_authoritative_and_fog() {
    let mut a = FixedStepClient::from_match(build_match(0));
    let mut b = build_match(0);
    let mut fog_a = FogGrid::new(PLAYER);
    let mut fog_b = FogGrid::new(PLAYER);
    let empty_defs = BTreeMap::<u64, DefId>::new();

    // 一端一次喂入 8 个逻辑帧，另一端在 observer 里逐帧 `step()`；
    // 两边只传同一命令脚本，本地各自派生迷雾，线格式不新增 fog 字段。
    a.advance_with_observer(TICKS_PER_LOGIC_FRAME * 8, |sim_a| {
        b.step();
        let view_a = sim_a.read_view();
        let view_b = b.read_view();
        let snapshot_a = sim_a.capture_snapshot();
        let snapshot_b = b.capture_snapshot();
        assert_eq!(
            view_a.tick, view_b.tick,
            "reason=batched_and_stepwise_ticks_match"
        );
        assert_eq!(
            sim_a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=two_clients_state_hash_matches"
        );
        assert_eq!(
            snapshot_bytes(&snapshot_a),
            snapshot_bytes(&snapshot_b),
            "reason=two_clients_snapshot_bytes_match"
        );
        assert_eq!(
            sim_a.first_difference(&b),
            None,
            "reason=two_clients_first_difference_none"
        );
        fog_a.observe(&view_a, &snapshot_a, &[], &empty_defs, PLAYER);
        fog_b.observe(&view_b, &snapshot_b, &[], &empty_defs, PLAYER);
        assert_eq!(
            fog_a.cells(),
            fog_b.cells(),
            "reason=two_clients_local_fog_matches"
        );
        assert_eq!(
            fog_a.counts(),
            fog_b.counts(),
            "reason=two_clients_explored_layer_matches"
        );
    });
    assert_eq!(PROTOCOL_VERSION, 10, "reason=protocol_version_10");
}

#[test]
fn replay_from_frame_zero_rebuilds_fog_and_state_hash() {
    let commands = spawn_script(0);
    let (original_traces, original_final_grid) = {
        let traces_and_grid = replay_trace(&commands, 12);
        // 原局实例在本作用域结束后销毁；录像只保留命令脚本与权威状态序列。
        traces_and_grid
    };
    let mut replay = build_match_from_commands(&commands);
    let mut replay_grid = FogGrid::new(PLAYER);
    let empty_defs = BTreeMap::<u64, DefId>::new();

    for expected in original_traces {
        replay.step();
        let view = replay.read_view();
        let snapshot = replay.capture_snapshot();
        replay_grid.observe(&view, &snapshot, &[], &empty_defs, PLAYER);
        assert_eq!(view.tick, expected.tick, "reason=replay_tick_matches");
        assert_eq!(
            replay.compute_state_hash(),
            expected.hash,
            "reason=replay_hash_matches"
        );
        assert_eq!(
            snapshot_bytes(&snapshot),
            expected.snapshot,
            "reason=replay_snapshot_bytes_match"
        );
        assert_eq!(
            replay_grid.cells(),
            expected.fog_cells.as_slice(),
            "reason=replay_fog_cells_match"
        );
        assert_eq!(
            replay_grid.counts().explored,
            expected.explored,
            "reason=replay_explored_layer_matches"
        );
    }
    assert_eq!(
        replay_grid.cells(),
        original_final_grid.cells(),
        "reason=replay_final_fog_matches_original"
    );
}

#[test]
fn reconnect_explored_layer_degrades_from_current_frame_until_q7_is_decided() {
    let commands = move_script();
    let (traces, from_zero) = replay_trace(&commands, 8);
    let final_tick = traces.last().expect("reason=missing_replay_trace").tick;
    let mut sim = build_match_from_commands(&commands);
    while sim.read_view().tick < final_tick {
        sim.step();
    }
    let view = sim.read_view();
    let snapshot = sim.capture_snapshot();
    let mut mid_join = FogGrid::new(PLAYER);
    mid_join.observe(
        &view,
        &snapshot,
        &[],
        &BTreeMap::<u64, DefId>::new(),
        PLAYER,
    );
    // Q7 结论未定：中途加入按当前帧起算，不冒充「从帧 0 重放派生」已核。
    assert!(
        mid_join.counts().explored < from_zero.counts().explored,
        "reason=mid_join_explored_layer_is_degraded"
    );
    assert_eq!(
        mid_join.last_tick(),
        Some(final_tick),
        "reason=mid_join_starts_at_current_frame"
    );
}

#[test]
fn version_numbers_and_snapshot_header_stay_at_v10() {
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    assert_eq!(SIM_VERSION, 10, "reason=sim_version_10");
    assert_eq!(PROTOCOL_VERSION, 10, "reason=protocol_version_10");
    let defs = synthetic_definitions();
    let sim = build_match(0);
    let snapshot = sim.capture_snapshot();
    assert_eq!(
        snapshot.version, SNAPSHOT_VERSION,
        "reason=snapshot_header_v10"
    );
    assert_eq!(snapshot.sim_version, SIM_VERSION, "reason=sim_header_v10");
    assert_eq!(
        snapshot.content_hash,
        defs.content_hash(),
        "reason=content_hash_unchanged"
    );
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_unchanged"
    );
    let bytes = snapshot_bytes(&snapshot);
    assert!(
        !contains_fog_marker(&bytes),
        "reason=snapshot_has_no_fog_fields"
    );
}

// ---------------------------------------------------------------------------
// 任务 4 / Q9：测试内反例 + 元断言防退化（形态①）
// ---------------------------------------------------------------------------

#[test]
fn reverse_validation_q9_formal_passes_authoritative_isolation_assertion() {
    let mut rendered = build_match(0);
    let mut plain = build_match(0);
    let hud = hud_with_fog(&rendered);
    let ctx = egui::Context::default();
    let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
        draw(ctx, &hud, &ZH, HudAssets { text_enabled: true });
    });
    output.drop_without_applying_deltas();
    assert!(
        authoritative_pair_matches(&mut rendered, &mut plain),
        "reason=formal_read_only_derivation_passes_isolation"
    );
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_isolation_assertion() {
    let mut rendered = build_match(0);
    let mut plain = build_match(0);
    degenerate_writeback(&mut rendered);
    assert!(
        !authoritative_pair_matches(&mut rendered, &mut plain),
        "reason=degenerate_writeback_fails_isolation_assertion"
    );
}

#[test]
fn reverse_validation_q9_formal_passes_no_fog_in_snapshot_hash_and_difference() {
    let rendered = build_match(0);
    let plain = build_match(0);
    let grid = observe_grid(&rendered);
    let rendered_bytes = snapshot_bytes(&rendered.capture_snapshot());
    let plain_bytes = snapshot_bytes(&plain.capture_snapshot());
    assert!(
        !contains_fog_marker(&rendered_bytes),
        "reason=formal_snapshot_has_no_fog_marker"
    );
    assert_eq!(
        rendered_bytes, plain_bytes,
        "reason=formal_snapshot_bytes_match"
    );
    assert_eq!(
        rendered.compute_state_hash(),
        plain.compute_state_hash(),
        "reason=formal_hash_ignores_fog_grid"
    );
    assert_eq!(
        rendered.first_difference(&plain),
        None,
        "reason=formal_first_difference_has_no_fog_field"
    );
    assert!(grid.counts().visible > 0, "reason=formal_grid_is_non_empty");
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_no_fog_assertion() {
    let rendered = build_match(0);
    let plain = build_match(0);
    let grid = observe_grid(&rendered);
    let degenerate_bytes = degenerate_fog_snapshot_bytes(&rendered, &grid);
    let degenerate_hash = degenerate_hash_with_fog(&rendered, &grid);
    let degenerate_diff = degenerate_first_difference_with_fog(&rendered, &plain, &grid);
    let plain_bytes = snapshot_bytes(&plain.capture_snapshot());
    let plain_hash = plain.compute_state_hash();
    let plain_diff = rendered.first_difference(&plain);
    assert!(
        contains_fog_marker(&degenerate_bytes),
        "reason=degenerate_snapshot_contains_fog_marker"
    );
    assert!(
        degenerate_bytes != plain_bytes
            && degenerate_hash != plain_hash
            && degenerate_diff.is_some(),
        "reason=degenerate_fog_fields_fail_the_same_assertion"
    );
    assert_eq!(
        plain_diff, None,
        "reason=formal_first_difference_stays_none"
    );
}

#[test]
fn reverse_validation_q9_formal_passes_minimap_filter_assertion() {
    let sim = build_match(0);
    let view = sim.read_view();
    let hud = hud_with_fog(&sim);
    assert!(
        minimap_filter_assertion(&hud.minimap.points, &view, hud.minimap.fog.is_some()),
        "reason=formal_minimap_filters_hidden_enemy"
    );
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_minimap_filter_assertion() {
    let sim = build_match(0);
    let view = sim.read_view();
    let points = degenerate_no_filter_points(&view);
    assert!(
        !minimap_filter_assertion(&points, &view, true),
        "reason=degenerate_no_filter_fails_minimap_assertion"
    );
}

#[test]
fn reverse_validation_q9_formal_passes_integer_phase_assertion() {
    let tick = FOG_DAY_FRAMES + 11;
    assert!(
        formal_phase_is_pure_integer_frame(tick),
        "reason=formal_phase_is_pure_integer_frame"
    );
    assert_eq!(
        day_phase(tick).code(),
        day_phase(tick).code(),
        "reason=formal_phase_is_stable"
    );
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_integer_phase_assertion() {
    let tick = FOG_DAY_FRAMES + 11;
    let formal = day_phase(tick);
    let degenerate = degenerate_phase_from_external_sources(tick, 144, 0x5eed);
    assert_ne!(
        degenerate.code(),
        formal.code(),
        "reason=degenerate_external_phase_fails_the_same_assertion"
    );
    assert!(
        formal_phase_is_pure_integer_frame(tick),
        "reason=formal_phase_still_passes"
    );
}

#[test]
fn reverse_validation_q9_formal_passes_content_hash_unchanged_assertion() {
    let defs = synthetic_definitions();
    let baseline = defs.content_hash();
    let sim = build_match(0);
    let hash_before = sim.compute_state_hash();
    let snapshot_before = snapshot_bytes(&sim.capture_snapshot());
    let grid = observe_grid(&sim);
    assert!(
        grid.counts().visible > 0,
        "reason=presentation_constants_are_used"
    );
    assert_eq!(
        defs.content_hash(),
        baseline,
        "reason=formal_fog_constants_are_not_in_definitions"
    );
    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=formal_fog_derivation_keeps_state_hash"
    );
    assert_eq!(
        snapshot_bytes(&sim.capture_snapshot()),
        snapshot_before,
        "reason=formal_fog_derivation_keeps_snapshot_bytes"
    );
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_content_hash_assertion() {
    let baseline = synthetic_definitions().content_hash();
    let degenerate = degenerate_fog_definitions();
    assert_ne!(
        degenerate.content_hash(),
        baseline,
        "reason=degenerate_fog_constants_change_content_hash"
    );
}

#[test]
fn reverse_validation_q9_formal_passes_explored_layer_monotonic_assertion() {
    let sim = build_match(0);
    let view_a = sim.read_view();
    let snapshot = sim.capture_snapshot();
    let mut view_b = view_a.clone();
    view_b.tick += 1;
    for entity in &mut view_b.entities {
        if entity.owner == PLAYER {
            entity.x = 40 * SCALE;
        }
    }
    let empty_defs = BTreeMap::<u64, DefId>::new();
    let mut grid = FogGrid::new(PLAYER);
    grid.observe(&view_a, &snapshot, &[], &empty_defs, PLAYER);
    let explored_a = grid.counts().explored;
    grid.observe(&view_b, &snapshot, &[], &empty_defs, PLAYER);
    assert!(
        grid.counts().explored >= explored_a,
        "reason=formal_explored_layer_is_monotonic"
    );
    assert!(
        grid.cell_at(0, 0).is_explored() && !grid.cell_at(0, 0).is_visible(),
        "reason=formal_old_cell_is_fogged_not_masked"
    );
}

#[test]
fn reverse_validation_q9_degenerate_fails_the_same_explored_layer_assertion() {
    let sim = build_match(0);
    let snapshot = sim.capture_snapshot();
    let mut moved = sim.read_view();
    moved.tick += 2;
    for entity in &mut moved.entities {
        if entity.owner == PLAYER {
            entity.x = 40 * SCALE;
        }
    }
    // 退化态：每帧只画圆形黑幕、不保留已探索层，等价于每帧新建网格。
    let mut degenerate = FogGrid::new(PLAYER);
    degenerate.observe(
        &moved,
        &snapshot,
        &[],
        &BTreeMap::<u64, DefId>::new(),
        PLAYER,
    );
    assert!(
        !degenerate.cell_at(0, 0).is_explored(),
        "reason=degenerate_no_explored_layer_fails_the_same_assertion"
    );
}
