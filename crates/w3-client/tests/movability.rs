//! 群体移动可移动性过滤的 CPU 回归（change `p8-faction-foundation` D1 / D3）。
//!
//! 全部为合成命令脚本 / 合成视图，不读取任何导出资产、不开 GPU 窗口、不引入
//! 墙钟。核心断言：
//! - 表现层只读映射由 client 侧冻结定义 `UnitDef.speed_raw` 派生，MUST NOT
//!   用运行时 `speed` 或建筑类型白名单（design D1 / R1）；
//! - 「1 农民 + 1 主城 + 1 金矿」混合选中集合右键，只有农民收到 `Move`；
//! - 落点分配只作用于筛后集合，与「只含这些农民」的脚本逐条一致；
//! - 全为不可移动实体时命令数为 0、有明确反馈、权威状态哈希与快照不变；
//! - 反向验证（改坏验红）：同一断言在退化过滤器下必然失败、正式实现下通过。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::melee::defs::{
    HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO, HUMAN_PEASANT, HUMAN_RANGED, HUMAN_SIEGE, HUMAN_SMITHY,
    HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE, NEUTRAL_TREE, ORC_ALTAR, ORC_BARRACKS, ORC_FARM, ORC_HERO,
    ORC_PEASANT, ORC_RANGED, ORC_SIEGE, ORC_SMITHY, ORC_SOLDIER, ORC_TOWN_HALL,
};
use w3_client::melee::synthetic_definitions;
use w3_client::{
    ClientApp, ClientCommand, FixedStepClient, InputAction, InputState, LOGIC_FPS, MovabilityMap,
    ScreenMapper, TICKS_PER_LOGIC_FRAME, TICKS_PER_SECOND, derive_movability, effective_movability,
    filter_movable,
};
use w3_core::{
    Command, CommandKind, DefId, EntityId, EntityView, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, RtsMatch, SCALE, SNAPSHOT_VERSION, SimFixed, StaticWorld,
    UnitDef, WorldView,
};
use w3_render::{ModelLibrary, TerrainMesh, TerrainVertex};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;

/// 农民（可移动）实体 ID。
const PEASANT_ID: u64 = 1;
/// 主城（不可移动）实体 ID。
const HALL_ID: u64 = 2;
/// 金矿（不可移动）实体 ID。
const MINE_ID: u64 = 3;

/// 退化过滤器（缺陷态）：不过滤，把所有实体都当可移动。
///
/// 仅用于反向验证；生产实现 MUST 用 [`filter_movable`]（design D3）。
fn degenerate_filter(selection: &BTreeSet<u64>, _: &MovabilityMap) -> Vec<u64> {
    selection.iter().copied().collect()
}

/// 由视图形参生成带注入包围盒的拾取映射：三个实体的屏幕包围盒完全可控，
/// 全屏拖框即可同时选中，且右键点不落在任何实体盒内（走 `Move` 分支）。
fn mixed_fixture() -> (WorldView, ScreenMapper, MovabilityMap) {
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            EntityView {
                id: PEASANT_ID,
                x: 0,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 60,
                owner: 1,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: HALL_ID,
                x: 10 * SCALE,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 120,
                owner: 1,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: MINE_ID,
                x: -10 * SCALE,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 500,
                owner: 1,
                alive: true,
                has_target: false,
            },
        ],
    };
    let mut boxes = BTreeMap::new();
    boxes.insert(PEASANT_ID, ([100.0, 100.0], [140.0, 140.0]));
    boxes.insert(HALL_ID, ([400.0, 400.0], [440.0, 440.0]));
    boxes.insert(MINE_ID, ([600.0, 600.0], [640.0, 640.0]));
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);

    // 判据来自 client 侧冻结定义 `UnitDef.speed_raw`，而非运行时视图。
    let defs = synthetic_definitions();
    let def_ids: BTreeMap<u64, DefId> = [
        (PEASANT_ID, HUMAN_PEASANT),
        (HALL_ID, HUMAN_TOWN_HALL),
        (MINE_ID, NEUTRAL_GOLD_MINE),
    ]
    .into_iter()
    .collect();
    let movability = derive_movability(&defs, &def_ids);
    (view, mapper, movability)
}

/// 全屏框选三个实体（农民 + 主城 + 金矿）。
fn select_all(
    input: &mut InputState,
    view: &WorldView,
    mapper: &ScreenMapper,
    map: &MovabilityMap,
) {
    input
        .apply(InputAction::SelectStart([0.0, 0.0]), view, mapper, &[], map)
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([1280.0, 720.0]),
            view,
            mapper,
            &[],
            map,
        )
        .unwrap();
}

/// 取出命令流里全部 `Move` 的目标实体 ID（按生成顺序，即稳定 EntityId 顺序）。
fn move_targets(commands: &[ClientCommand]) -> Vec<u64> {
    commands
        .iter()
        .filter_map(|command| match command {
            ClientCommand::Simulation(Command {
                kind: CommandKind::Move { entity, .. },
                ..
            }) => Some(entity.value()),
            _ => None,
        })
        .collect()
}

/// 1.2 的验收断言：只有农民收到 `Move`，主城 / 金矿 MUST NOT 收到。
fn only_peasant_moves(commands: &[ClientCommand]) -> bool {
    commands.len() == 1 && move_targets(commands) == vec![PEASANT_ID]
}

#[test]
fn derive_movability_uses_definition_speed_raw_not_runtime_view() {
    let defs = synthetic_definitions();
    let def_ids: BTreeMap<u64, DefId> = [
        (1, HUMAN_PEASANT),
        (2, HUMAN_TOWN_HALL),
        (3, NEUTRAL_GOLD_MINE),
        (4, ORC_PEASANT),
        (5, ORC_TOWN_HALL),
    ]
    .into_iter()
    .collect();
    let map = derive_movability(&defs, &def_ids);
    // 农民 / 兽族农民 speed_raw > 0 → 可移动；建筑 / 资源点 == 0 → 不可移动。
    assert_eq!(map.get(&1), Some(&true));
    assert_eq!(map.get(&2), Some(&false));
    assert_eq!(map.get(&3), Some(&false));
    assert_eq!(map.get(&4), Some(&true));
    assert_eq!(map.get(&5), Some(&false));
    // 定义缺失的实体按不可移动处理（显式保守，不放行未知类型）。
    let missing = derive_movability(&defs, &[(9, 999_999)].into_iter().collect());
    assert_eq!(missing.get(&9), Some(&false));
    assert_eq!(map.len(), 5, "reason=one_entry_per_registered_entity");
}

#[test]
fn mixed_selection_only_peasant_receives_move() {
    let (view, mapper, map) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 3, "reason=all_three_selected");

    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();

    // 正式实现通过：只有农民收到 Move。
    assert!(
        only_peasant_moves(&commands),
        "reason=only_peasant_may_move"
    );
    assert_eq!(move_targets(&commands), vec![PEASANT_ID]);
    assert!(!input.no_movable_target(), "reason=movable_target_exists");
    // 建筑 / 金矿位置不变：输入翻译层没有为它们生成任何命令，也没有反写视图。
    assert_eq!(view.entities[1].x, 10 * SCALE);
    assert_eq!(view.entities[2].x, -10 * SCALE);
    assert!(!commands.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(Command {
            kind: CommandKind::Move { entity, .. },
            ..
        }) if entity.value() == HALL_ID || entity.value() == MINE_ID
    )));
}

#[test]
fn landing_slots_match_peasants_only_equivalent_script() {
    let (view, mapper, map) = mixed_fixture();

    // 混合集合（农民 + 主城 + 金矿），正式过滤后只有农民移动。
    let mut mixed = InputState::new(1);
    select_all(&mut mixed, &view, &mapper, &map);
    let mixed_cmds = mixed
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();

    // 等效脚本：世界视图里只有农民（EntityId 不变），同样右键。
    let peasants_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: vec![view.entities[0].clone()],
    };
    let mut only = InputState::new(1);
    select_all(&mut only, &peasants_view, &mapper, &map);
    let only_cmds = only
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &peasants_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();

    // 落点分配逐条相同：命令字节完全一致（实体 / x / y / group / 帧号 / 序号）。
    assert_eq!(
        mixed_cmds, only_cmds,
        "reason=filter_must_not_change_landing_slots"
    );
    assert_eq!(move_targets(&mixed_cmds), vec![PEASANT_ID]);
}

/// 多农民混合集合：族内落点分配按稳定 `EntityId` 顺序、分组号非 0，
/// 且与「只含这两个农民」的脚本逐条相同（spec「落点分配不受过滤影响」）。
#[test]
fn multi_peasant_group_landing_slots_match_peasants_only_script() {
    // 2 名农民（ID 1、4）与建筑 / 金矿（ID 2、3）交错，检验稳定 ID 顺序。
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            EntityView {
                id: 1,
                x: 0,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 60,
                owner: 1,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: 2,
                x: 10 * SCALE,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 120,
                owner: 1,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: 3,
                x: -10 * SCALE,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 500,
                owner: 1,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: 4,
                x: 20 * SCALE,
                y: 0,
                facing: 0,
                speed: 3 * SCALE,
                hp: 60,
                owner: 1,
                alive: true,
                has_target: false,
            },
        ],
    };
    let mut boxes = BTreeMap::new();
    boxes.insert(1u64, ([100.0, 100.0], [140.0, 140.0]));
    boxes.insert(2u64, ([400.0, 400.0], [440.0, 440.0]));
    boxes.insert(3u64, ([600.0, 600.0], [640.0, 640.0]));
    boxes.insert(4u64, ([200.0, 100.0], [240.0, 140.0]));
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);
    let defs = synthetic_definitions();
    let def_ids: BTreeMap<u64, DefId> = [
        (1, HUMAN_PEASANT),
        (2, HUMAN_TOWN_HALL),
        (3, NEUTRAL_GOLD_MINE),
        (4, HUMAN_PEASANT),
    ]
    .into_iter()
    .collect();
    let map = derive_movability(&defs, &def_ids);

    let mut mixed = InputState::new(1);
    select_all(&mut mixed, &view, &mapper, &map);
    assert_eq!(mixed.selection().len(), 4, "reason=all_four_selected");
    let mixed_cmds = mixed
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    // 只有两个农民收到 Move，且按稳定 EntityId 顺序 (1, 4)。
    assert_eq!(move_targets(&mixed_cmds), vec![1, 4]);
    let group = mixed_cmds.iter().find_map(|command| match command {
        ClientCommand::Simulation(Command {
            kind: CommandKind::Move { group, .. },
            ..
        }) => Some(*group),
        _ => None,
    });
    assert!(group.is_some_and(|value| value > 0), "reason=group_nonzero");

    // 等效脚本：只含两个农民。
    let peasants_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: vec![view.entities[0].clone(), view.entities[3].clone()],
    };
    let mut only = InputState::new(1);
    select_all(&mut only, &peasants_view, &mapper, &map);
    let only_cmds = only
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &peasants_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        mixed_cmds, only_cmds,
        "reason=multi_peasant_landing_slots_unchanged"
    );
}

#[test]
fn all_immovable_emits_no_commands_with_explicit_feedback() {
    let (view, mapper, map) = mixed_fixture();
    // 只框选主城 + 金矿（两个不可移动实体）：注入的包围盒让选择性完全可控。
    let mut input = InputState::new(1);
    input
        .apply(
            InputAction::SelectStart([380.0, 380.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([660.0, 660.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        input.selection(),
        &[HALL_ID, MINE_ID].into_iter().collect::<BTreeSet<u64>>(),
        "reason=only_immovable_selected"
    );

    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    // 命令数为 0、不生成空命令；权威状态与状态哈希由下面的 app 级回归核对。
    assert!(commands.is_empty(), "reason=no_movable_target_no_commands");
    assert!(input.no_movable_target(), "reason=explicit_feedback_flag");
}

/// 合成高度场，供 `ClientApp` 的拾取投影使用（不加载任何资产）。
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

/// 合成 `Spawn` 命令；两个实体都归属本地玩家，避免走 Attack 分支。
fn spawn(sequence: u32, x: i64, hp: i64) -> Command {
    Command::new(
        Frame::new(0),
        1,
        sequence,
        CommandKind::Spawn {
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(0).unwrap(),
            hp,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: 0,
            attack_range: SimFixed::from_int(1).unwrap(),
        },
    )
}

#[test]
fn client_app_all_immovable_keeps_state_hash_and_snapshot() {
    // 两个实体全部映射为不可移动（表现层只读判据，注入经 `with_movability`）。
    let mut movability = MovabilityMap::new();
    movability.insert(1, false);
    movability.insert(2, false);
    assert_eq!(SNAPSHOT_VERSION, 4, "reason=snapshot_version_unchanged");

    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0, 120)).unwrap();
    driver.submit_command(spawn(2, 4, 500)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
        .with_movability(movability);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

    let hash_before = app.driver().sim().compute_state_hash();
    let snapshot_before = app.driver().sim().capture_snapshot();
    let mapper = app.screen_mapper();
    let center = mapper.world_to_screen([0.0, 0.0]);
    let target = [center[0] + 320.0, center[1] + 220.0];

    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(
        !app.selection().is_empty(),
        "reason=selection_is_presentation_only"
    );
    let commands = app.apply_input(InputAction::ContextAction(target)).unwrap();

    // 命令数为 0、明确反馈、权威状态哈希与快照字节不变。
    assert!(commands.is_empty(), "reason=no_commands_generated");
    assert!(app.no_movable_target(), "reason=explicit_feedback_surface");
    assert_eq!(
        app.driver().sim().compute_state_hash(),
        hash_before,
        "reason=all_immovable_must_not_change_state_hash"
    );
    assert_eq!(
        app.driver().sim().capture_snapshot(),
        snapshot_before,
        "reason=all_immovable_must_not_change_snapshot"
    );
    assert!(app.last_error().is_none(), "reason=no_input_error");
}

/// 反向验证（设计 D3 / `CODEBUDDY.md` §5 改坏验红）：
/// 同一条 1.2 断言在正式实现下通过、在退化实现下必然失败。
/// 两条证据分别由下面两个 `#[test]` 保留，命令输出可用 `--nocapture` 观察。
#[test]
fn reverse_validation_formal_implementation_passes_the_assertion() {
    let (view, mapper, map) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    // 正式实现（`filter_movable`）：断言成立。
    assert!(
        only_peasant_moves(&commands),
        "reason=formal_filter_must_pass_the_assertion"
    );
    assert_eq!(filter_movable(input.selection(), &map), vec![PEASANT_ID]);
}

#[test]
fn reverse_validation_degenerate_filter_fails_the_same_assertion() {
    let (view, mapper, map) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    let selection = input.selection().clone();

    // 退化过滤器：不过滤，主城 / 金矿也被当作可移动（缺陷态）。
    let formal = filter_movable(&selection, &map);
    let degenerate = degenerate_filter(&selection, &map);
    assert_eq!(formal, vec![PEASANT_ID], "reason=formal_filter_narrows_set");
    assert_eq!(
        degenerate,
        vec![PEASANT_ID, HALL_ID, MINE_ID],
        "reason=degenerate_filter_moves_buildings_and_resources"
    );

    // 把退化过滤器等价于的「空映射」喂给真实 apply 路径（`unwrap_or(true)`
    // 即「把所有实体都当可移动」），证明缺陷态会给建筑 / 金矿生成 Move。
    let degenerate_map = MovabilityMap::new();
    // A/B 使用同一选中集合：空映射下的生产过滤器与退化过滤器输出完全一致，
    // 即「退化实现」确实通过真实 `filter_movable` 路径注入，而非旁路模拟。
    assert_eq!(
        filter_movable(&selection, &degenerate_map),
        degenerate,
        "reason=degenerate_filter_equals_degenerate_map_on_same_selection"
    );
    let mut broken = InputState::new(1);
    select_all(&mut broken, &view, &mapper, &degenerate_map);
    let broken_commands = broken
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &degenerate_map,
        )
        .unwrap();
    assert_eq!(
        move_targets(&broken_commands),
        vec![PEASANT_ID, HALL_ID, MINE_ID],
        "reason=degenerate_impl_generates_move_for_all"
    );

    // 元断言：1.2 的同一条断言在退化实现下必然失败（缺陷态被抓出）。
    assert!(
        !only_peasant_moves(&broken_commands),
        "reason=degenerate_filter_must_fail_the_same_assertion"
    );
}

// ---------------------------------------------------------------------------
// P8.1 人族新增实体 R1.4 回归（design D1 / D7、unit-orders delta）
//
// 键盘 `M` / `A` 移动按 R13.6 MUST 与右键共用同一输入翻译与命令生成处；
// 当前 `InputState::apply` 的 `ContextAction`（右键 + 命令卡 Move）即该
// 单一接缝，`filter_movable` 是唯一的 `Move` 生成点。R13.7 已登记「`KeyM`
// 键位接线差距由后续 change 承接」，本 change 不新增 `InputAction` 变体、
// 不改键位；以下回归覆盖共享接缝与 `filter_movable` 纯函数本身，不冒充已解决运行时动态实体（P8.0 B1）。
// ---------------------------------------------------------------------------

/// 人族新增实体的 frame-0 静态装配槽位 ID（判据来自冻结定义）。
const H_PEASANT: u64 = 1;
const H_RANGED: u64 = 2;
const H_SIEGE: u64 = 3;
const H_HERO: u64 = 4;
const H_FARM: u64 = 5;
const H_ALTAR: u64 = 6;
const H_SMITHY: u64 = 7;
const H_HALL: u64 = 8;
const H_MINE: u64 = 9;
const H_TREE: u64 = 10;

fn human_entity(id: u64, x_world: i64, hp: i64) -> EntityView {
    EntityView {
        id,
        x: x_world * SCALE,
        y: 0,
        facing: 0,
        speed: 3 * SCALE,
        hp,
        owner: 1,
        alive: true,
        has_target: false,
    }
}

/// 含人族新增六类 + 主城 / 金矿 / 树木的 frame-0 静态装配夹具：
/// 视图实体、`EntityId→DefId` 关联、由冻结定义 `speed_raw` 派生的只读映射。
fn human_fixture() -> (WorldView, ScreenMapper, MovabilityMap, BTreeMap<u64, DefId>) {
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            human_entity(H_PEASANT, 0, 60),
            human_entity(H_RANGED, 10, 90),
            human_entity(H_SIEGE, 20, 180),
            human_entity(H_HERO, 30, 400),
            human_entity(H_FARM, 40, 200),
            human_entity(H_ALTAR, 50, 300),
            human_entity(H_SMITHY, 60, 300),
            human_entity(H_HALL, 70, 120),
            human_entity(H_MINE, 80, 500),
            human_entity(H_TREE, 90, 200),
        ],
    };
    // 两行网格，注入真实模型屏幕包围盒；右键点 [1100,600] 不落在任何盒内。
    let mut boxes = BTreeMap::new();
    let row_one = [H_PEASANT, H_RANGED, H_SIEGE, H_HERO, H_FARM];
    let row_two = [H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE];
    for (index, &id) in row_one.iter().enumerate() {
        let x0 = 100.0 + 180.0 * index as f64;
        boxes.insert(id, ([x0, 100.0], [x0 + 40.0, 140.0]));
    }
    for (index, &id) in row_two.iter().enumerate() {
        let x0 = 100.0 + 180.0 * index as f64;
        boxes.insert(id, ([x0, 300.0], [x0 + 40.0, 340.0]));
    }
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);

    let def_ids: BTreeMap<u64, DefId> = [
        (H_PEASANT, HUMAN_PEASANT),
        (H_RANGED, HUMAN_RANGED),
        (H_SIEGE, HUMAN_SIEGE),
        (H_HERO, HUMAN_HERO),
        (H_FARM, HUMAN_FARM),
        (H_ALTAR, HUMAN_ALTAR),
        (H_SMITHY, HUMAN_SMITHY),
        (H_HALL, HUMAN_TOWN_HALL),
        (H_MINE, NEUTRAL_GOLD_MINE),
        (H_TREE, NEUTRAL_TREE),
    ]
    .into_iter()
    .collect();
    let movability = derive_movability(&synthetic_definitions(), &def_ids);
    (view, mapper, movability, def_ids)
}

/// 退化定义（缺陷态）：把新增人口建筑 / 祭坛 / 研发建筑的 `speed_raw`
/// 改成非 0，模拟「新定义未遵循 `speed_raw == 0`」的缺陷实现。
/// 仅用于反向验证；生产实现 MUST 用 `synthetic_definitions()`。
fn degenerate_defs_with_movable_buildings() -> FrozenDefinitions {
    let defs = synthetic_definitions();
    let units: Vec<(DefId, UnitDef)> = defs
        .units()
        .iter()
        .map(|(&id, &def)| {
            let mut def = def;
            if matches!(id, HUMAN_FARM | HUMAN_ALTAR | HUMAN_SMITHY) {
                def.speed_raw = 3 * SCALE;
            }
            (id, def)
        })
        .collect();
    let abilities = defs
        .abilities()
        .iter()
        .map(|(&id, d)| (id, d.clone()))
        .collect();
    let items = defs.items().iter().map(|(&id, &d)| (id, d)).collect();
    let upgrades = defs.upgrades().iter().map(|(&id, &d)| (id, d)).collect();
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("movability: reason=degenerate_defs_rebuild_failed")
}

/// 被反向验证的断言：混合集合只为可移动单位生成 `Move`（稳定 EntityId 顺序）。
fn only_movable_units_move(commands: &[ClientCommand]) -> bool {
    !commands.is_empty() && move_targets(commands) == vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]
}

/// 反向验证·修复态：正式定义（`speed_raw == 0`）派生的映射通过
/// 「只移动可移动单位」断言。与下一条缺陷态构成 `CODEBUDDY.md` §5 反例证据对。
#[test]
fn human_reverse_validation_formal_defs_pass_the_assertion() {
    let (view, mapper, map, _) = human_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 10, "reason=all_ten_selected");
    let commands = input
        .apply(
            InputAction::ContextAction([1100.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert!(
        only_movable_units_move(&commands),
        "reason=formal_defs_must_pass_the_assertion"
    );
    assert_eq!(
        filter_movable(input.selection(), &map),
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]
    );
}

/// 反向验证·缺陷态：退化定义（建筑 `speed_raw != 0`）下
/// `derive_movability` 会把新增建筑判为可移动，同一条「只移动可移动单位」
/// 断言必然失败；元断言证明缺陷被抓出（改坏验红）。
#[test]
fn human_reverse_validation_degenerate_defs_fail_the_same_assertion() {
    let (view, mapper, _, def_ids) = human_fixture();
    let degenerate_map = derive_movability(&degenerate_defs_with_movable_buildings(), &def_ids);
    assert_eq!(degenerate_map.get(&H_FARM), Some(&true));
    assert_eq!(degenerate_map.get(&H_ALTAR), Some(&true));
    assert_eq!(degenerate_map.get(&H_SMITHY), Some(&true));

    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &degenerate_map);
    let broken_commands = input
        .apply(
            InputAction::ContextAction([1100.0, 600.0]),
            &view,
            &mapper,
            &[],
            &degenerate_map,
        )
        .unwrap();
    assert_eq!(
        move_targets(&broken_commands),
        vec![
            H_PEASANT, H_RANGED, H_SIEGE, H_HERO, H_FARM, H_ALTAR, H_SMITHY
        ],
        "reason=degenerate_defs_move_new_buildings"
    );
    assert!(
        !only_movable_units_move(&broken_commands),
        "reason=degenerate_defs_must_fail_the_same_assertion"
    );
    // 正式定义仍能通过同一断言（修复态）。
    let formal = derive_movability(&synthetic_definitions(), &def_ids);
    let mut fixed_input = InputState::new(1);
    select_all(&mut fixed_input, &view, &mapper, &formal);
    let fixed_commands = fixed_input
        .apply(
            InputAction::ContextAction([1100.0, 600.0]),
            &view,
            &mapper,
            &[],
            &formal,
        )
        .unwrap();
    assert!(only_movable_units_move(&fixed_commands));
}

/// 新增实体可移动性由冻结定义 `speed_raw` 派生（design D1 / D7）：
/// 远程 / 反重甲 / 英雄可移动；人口建筑 / 祭坛 / 研发建筑 / 主城 /
/// 金矿 / 树木不可移动；MUST NOT 用运行时 `speed` 或建筑类型白名单。
#[test]
fn human_new_defs_movability_is_derived_from_speed_raw() {
    let (_, _, map, _) = human_fixture();
    assert_eq!(map.len(), 10, "reason=one_entry_per_registered_entity");
    for id in [H_PEASANT, H_RANGED, H_SIEGE, H_HERO] {
        assert_eq!(map.get(&id), Some(&true), "reason=movable id={id}");
    }
    for id in [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE] {
        assert_eq!(map.get(&id), Some(&false), "reason=immovable id={id}");
    }
    // 判据确为定义层 `speed_raw`：三类新增建筑速度必须是 0。
    let defs = synthetic_definitions();
    for def_id in [HUMAN_FARM, HUMAN_ALTAR, HUMAN_SMITHY] {
        assert_eq!(
            defs.units().get(&def_id).unwrap().speed_raw,
            0,
            "reason=new_building_speed_raw_zero def={def_id}"
        );
    }
}

/// 混合框选集合（农民 + 远程 + 新增人口建筑 / 祭坛 / 研发建筑 + 主城 +
/// 金矿 + 树木）右键移动：只为可移动单位生成 `Move`，位置不变、落点按
/// 稳定 `EntityId` 顺序，且与「只选中可移动单位」等效脚本逐条一致（R1.4）。
#[test]
fn human_mixed_selection_only_moves_movable_units() {
    let (view, mapper, map, _) = human_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 10, "reason=all_ten_selected");

    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    let targets = move_targets(&commands);
    assert_eq!(
        targets,
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO],
        "reason=stable_entity_id_order"
    );
    assert!(!input.no_movable_target(), "reason=movable_targets_exist");
    // 新增建筑 / 资源点 / 主城 MUST NOT 收到 Move、位置不变。
    for id in [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE] {
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                ClientCommand::Simulation(Command {
                    kind: CommandKind::Move { entity, .. },
                    ..
                }) if entity.value() == id
            )),
            "reason=immovable_must_not_move id={id}"
        );
    }
    assert_eq!(view.entities[4].x, 40 * SCALE);
    assert_eq!(view.entities[7].x, 70 * SCALE);

    // 等效脚本：只含四个可移动单位，命令字节逐条相同（落点不受过滤影响）。
    let movers_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: view
            .entities
            .iter()
            .filter(|entity| targets.contains(&entity.id))
            .cloned()
            .collect(),
    };
    let mut only = InputState::new(1);
    select_all(&mut only, &movers_view, &mapper, &map);
    let only_commands = only
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &movers_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        commands, only_commands,
        "reason=landing_slots_unchanged_by_filter"
    );
    assert!(commands.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(Command {
            kind: CommandKind::Move { group, .. },
            ..
        }) if *group > 0
    )));
}

/// 全为新增建筑 / 主城 / 资源点时不生成任何 `Move`，有明确无可执行目标
/// 反馈；权威状态哈希与快照不变（R1.4 / R13.6）。
#[test]
fn human_all_immovable_selection_emits_no_move_with_feedback() {
    let (view, mapper, map, _) = human_fixture();
    let immovable_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: view
            .entities
            .iter()
            .filter(|entity| {
                [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE].contains(&entity.id)
            })
            .cloned()
            .collect(),
    };
    let mut input = InputState::new(1);
    select_all(&mut input, &immovable_view, &mapper, &map);
    assert_eq!(input.selection().len(), 6, "reason=only_immovable_selected");
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &immovable_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert!(commands.is_empty(), "reason=no_move_commands");
    assert!(input.no_movable_target(), "reason=explicit_feedback_flag");
}

/// 全为新增建筑时 `ClientApp` 权威状态哈希 / 快照不变：映射来自冻结定义
/// `speed_raw`，而非手填或运行时 `speed`（R1.4 / ADR 0002）。
#[test]
fn human_new_buildings_all_immovable_keep_state_hash_and_snapshot() {
    let defs = synthetic_definitions();
    let entity_defs: BTreeMap<u64, DefId> = [(1u64, HUMAN_FARM), (2u64, HUMAN_ALTAR)]
        .into_iter()
        .collect();
    let movability = derive_movability(&defs, &entity_defs);
    assert_eq!(movability.get(&1), Some(&false));
    assert_eq!(movability.get(&2), Some(&false));
    assert_eq!(SNAPSHOT_VERSION, 4, "reason=snapshot_version_unchanged");

    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0, 120)).unwrap();
    driver.submit_command(spawn(2, 4, 500)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
        .with_movability(movability);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

    let hash_before = app.driver().sim().compute_state_hash();
    let snapshot_before = app.driver().sim().capture_snapshot();
    let mapper = app.screen_mapper();
    let center = mapper.world_to_screen([0.0, 0.0]);
    let target = [center[0] + 320.0, center[1] + 220.0];

    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(!app.selection().is_empty());
    let commands = app.apply_input(InputAction::ContextAction(target)).unwrap();
    assert!(commands.is_empty(), "reason=no_commands_generated");
    assert!(app.no_movable_target(), "reason=explicit_feedback_surface");
    assert_eq!(
        app.driver().sim().compute_state_hash(),
        hash_before,
        "reason=all_immovable_must_not_change_state_hash"
    );
    assert_eq!(
        app.driver().sim().capture_snapshot(),
        snapshot_before,
        "reason=all_immovable_must_not_change_snapshot"
    );
    assert!(app.last_error().is_none(), "reason=no_input_error");
}

// ---------------------------------------------------------------------------
// change `runtime-entity-movability`（承接已知限制 B1）
//
// 运行时 Train / Build 动态实体不在装配期 def_ids 关联里。分层判据：frame-0
// 装配实体沿用 derive_movability 派生的映射（其运行时视图 speed 被 max(1)
// 抬成 >0，不可用视图 speed 区分建筑）；运行时动态实体由只读视图
// EntityView.speed 判定（complete_production 按真实 def.speed_raw 写入，
// 建筑 0、单位 >0）。两条来源 MUST NOT 互相替代（design D2 / D3 / D4 / D5）。
// ---------------------------------------------------------------------------

/// 运行时动态实体的冻结定义编号（合成原创常量，不读取任何资产）。
const RT_BUILDING_DEF: DefId = 1;
const RT_UNIT_DEF: DefId = 2;

/// 运行时生产对局的三类实体 ID。
struct RuntimeIds {
    producer: u64,
    building: u64,
    unit: u64,
}

/// 构造「建筑 speed_raw == 0 / 战斗单位 speed_raw > 0」两类冻结定义。
/// 全部为编译期整数常量，train_frames 取 2 便于推进到完成帧。
fn runtime_definitions() -> (FrozenDefinitions, i64, i64) {
    let building_speed_raw = 0;
    let unit_speed_raw = SimFixed::from_int(5).unwrap().raw();
    let building = UnitDef {
        hp: 50,
        mana: 0,
        speed_raw: building_speed_raw,
        damage: 0,
        range_raw: 0,
        supply: 0,
        train_frames: 2,
    };
    let unit = UnitDef {
        hp: 80,
        mana: 0,
        speed_raw: unit_speed_raw,
        damage: 10,
        range_raw: SimFixed::from_int(10).unwrap().raw(),
        supply: 1,
        train_frames: 2,
    };
    let defs = FrozenDefinitions::new(
        vec![(RT_BUILDING_DEF, building), (RT_UNIT_DEF, unit)],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("movability: reason=runtime_definitions_invalid");
    (defs, building_speed_raw, unit_speed_raw)
}

/// 经 `RtsMatch` 既有公开通路真实生产运行时建筑与战斗单位，推进到完成帧。
/// 帧 0 出生、帧 1 入队并递减、帧 2 生产完成；`step()` 全程无参。
fn runtime_production_match() -> (RtsMatch, FrozenDefinitions) {
    let (defs, _, _) = runtime_definitions();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    sim.submit_command(spawn(1, 0, 100))
        .expect("movability: reason=producer_spawn_rejected");
    let producer = EntityId::new(1).expect("movability: reason=invalid_producer_id");
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::Train {
            producer,
            unit_def: RT_BUILDING_DEF,
            is_building: true,
        },
    ))
    .expect("movability: reason=building_train_rejected");
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        2,
        GameplayCommandKind::Train {
            producer,
            unit_def: RT_UNIT_DEF,
            is_building: false,
        },
    ))
    .expect("movability: reason=unit_train_rejected");
    for _ in 0..3 {
        sim.step();
    }
    (sim, defs)
}

/// 由真实生产视图注入每个实体的可控屏幕包围盒：全屏拖框选中全部，
/// 右键点落在所有盒之外（走群体 `Move` 分支而非 `Attack` 分支）。
fn runtime_mapper(view: &WorldView) -> ScreenMapper {
    let mut boxes = BTreeMap::new();
    for (index, entity) in view.entities.iter().enumerate() {
        let x0 = 100.0 + 200.0 * index as f64;
        boxes.insert(entity.id, ([x0, 100.0], [x0 + 40.0, 140.0]));
    }
    ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes)
}

/// 完整夹具：真实生产对局、只读视图、拾取映射、装配期映射与三类实体 ID。
/// 装配期映射只登记 frame-0 生产者（可移动单位定义），运行时 Train 产出的
/// 实体刻意不在关联中——精确复现 B1「映射未登记动态实体」的场景。
fn runtime_fixture() -> (
    RtsMatch,
    FrozenDefinitions,
    WorldView,
    ScreenMapper,
    MovabilityMap,
    RuntimeIds,
) {
    let (sim, defs) = runtime_production_match();
    let view = sim.read_view();
    assert_eq!(
        view.entities.len(),
        3,
        "reason=producer_plus_two_runtime_entities"
    );
    let runtime: Vec<&EntityView> = view.entities.iter().filter(|e| e.id != 1).collect();
    let ids = RuntimeIds {
        producer: 1,
        building: runtime
            .iter()
            .find(|e| e.speed == 0)
            .expect("movability: reason=runtime_building_missing")
            .id,
        unit: runtime
            .iter()
            .find(|e| e.speed > 0)
            .expect("movability: reason=runtime_unit_missing")
            .id,
    };
    let def_ids: BTreeMap<u64, DefId> = [(ids.producer, RT_UNIT_DEF)].into_iter().collect();
    let assembly = derive_movability(&defs, &def_ids);
    let mapper = runtime_mapper(&view);
    (sim, defs, view, mapper, assembly, ids)
}

/// 被反向验证的单一断言：运行时生产建筑 MUST NOT 出现在 `Move` 生成集合中。
fn runtime_building_does_not_move(commands: &[ClientCommand], building_id: u64) -> bool {
    !move_targets(commands).contains(&building_id)
}

/// 全屏框选并右键，走真实 `InputState::apply` 群体 `Move` 路径。
fn runtime_group_move(
    view: &WorldView,
    mapper: &ScreenMapper,
    movability: &MovabilityMap,
) -> Vec<ClientCommand> {
    let mut input = InputState::new(1);
    select_all(&mut input, view, mapper, movability);
    assert_eq!(
        input.selection().len(),
        view.entities.len(),
        "reason=all_entities_selected"
    );
    input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            view,
            mapper,
            &[],
            movability,
        )
        .unwrap()
}

/// 合并函数纯函数断言（tasks 2.1 / D2 / D3）：四类输入的分层判据。
/// frame-0 已登记项原样保留（即使视图 speed 被 max(1) 抬成 >0 也不覆盖），
/// 未登记的运行时实体按 `EntityView.speed != 0` 补判据；两条来源不互相替代。
#[test]
fn effective_movability_merges_registered_and_runtime_entities() {
    let base: MovabilityMap = [(1, true), (2, false), (3, false)].into_iter().collect();
    let positive = 3 * SCALE;
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            EntityView {
                id: 1,
                x: 0,
                y: 0,
                facing: 0,
                speed: positive,
                hp: 60,
                owner: 1,
                alive: true,
                has_target: false,
            },
            // frame-0 主城：视图 speed 被 max(1) 抬成 >0，但已登记 false。
            EntityView {
                id: 2,
                x: 10 * SCALE,
                y: 0,
                facing: 0,
                speed: positive,
                hp: 120,
                owner: 1,
                alive: true,
                has_target: false,
            },
            // frame-0 金矿：同上。
            EntityView {
                id: 3,
                x: -10 * SCALE,
                y: 0,
                facing: 0,
                speed: positive,
                hp: 500,
                owner: 1,
                alive: true,
                has_target: false,
            },
            // 运行时建筑：未登记、speed == 0。
            EntityView {
                id: 4,
                x: 0,
                y: 0,
                facing: 0,
                speed: 0,
                hp: 50,
                owner: 1,
                alive: true,
                has_target: false,
            },
            // 运行时战斗单位：未登记、speed > 0。
            EntityView {
                id: 5,
                x: 0,
                y: 0,
                facing: 0,
                speed: 5 * SCALE,
                hp: 80,
                owner: 1,
                alive: true,
                has_target: false,
            },
        ],
    };
    let selection: BTreeSet<u64> = [1, 2, 3, 4, 5].into_iter().collect();

    let effective = effective_movability(&base, &view);
    assert_eq!(effective.len(), 5, "reason=all_entities_have_criterion");
    // 已登记项原样保留：frame-0 建筑 / 金矿的视图 speed > 0 不得覆盖 false。
    assert_eq!(effective.get(&1), Some(&true), "reason=registered_peasant");
    assert_eq!(
        effective.get(&2),
        Some(&false),
        "reason=registered_hall_kept"
    );
    assert_eq!(
        effective.get(&3),
        Some(&false),
        "reason=registered_mine_kept"
    );
    // 未登记运行时实体按视图 speed 补判据。
    assert_eq!(
        effective.get(&4),
        Some(&false),
        "reason=runtime_building_immovable"
    );
    assert_eq!(
        effective.get(&5),
        Some(&true),
        "reason=runtime_unit_movable"
    );
    // 纯函数：输入映射未被改写。
    assert_eq!(base.len(), 3, "reason=input_map_not_mutated");
    assert_eq!(base.get(&2), Some(&false), "reason=input_entry_not_mutated");
    // 经真实 filter_movable：只为可移动单位生成 Move，稳定 EntityId 顺序。
    assert_eq!(
        filter_movable(&selection, &effective),
        vec![1, 5],
        "reason=only_movable_entities"
    );
}

/// 核心回归（tasks 3.1 / R1.4 / R13.6）：真实生产出的运行时建筑
/// （view.speed == 0）在含它的混合选中集合里 MUST NOT 生成 Move、位置不变；
/// 运行时战斗单位（view.speed > 0）与 frame-0 生产者正常响应 Move。
#[test]
fn runtime_building_does_not_move_but_unit_and_peasant_do() {
    let (sim, defs, view, mapper, assembly, ids) = runtime_fixture();
    let (_, building_speed_raw, unit_speed_raw) = runtime_definitions();

    // 只读视图 speed 如实投影内核真实 def.speed_raw。
    let building = view.entities.iter().find(|e| e.id == ids.building).unwrap();
    let unit = view.entities.iter().find(|e| e.id == ids.unit).unwrap();
    let producer = view.entities.iter().find(|e| e.id == ids.producer).unwrap();
    assert_eq!(
        building.speed, building_speed_raw,
        "reason=building_speed_zero"
    );
    assert_eq!(building.speed, 0, "reason=building_speed_exactly_zero");
    assert_eq!(unit.speed, unit_speed_raw, "reason=unit_speed_matches_def");
    assert!(unit.speed > 0, "reason=unit_speed_positive");
    // frame-0 装配实体运行时视图 speed 被 max(1) 抬成 >0。
    assert!(producer.speed > 0, "reason=frame0_speed_lifted");
    assert_ne!(ids.building, ids.unit, "reason=distinct_runtime_entities");

    // 装配期映射只登记 frame-0 生产者，运行时实体未登记（B1）。
    assert_eq!(assembly.len(), 1, "reason=assembly_registers_frame0_only");
    assert_eq!(assembly.get(&ids.producer), Some(&true));
    assert!(
        !assembly.contains_key(&ids.building),
        "reason=building_unregistered"
    );
    assert!(
        !assembly.contains_key(&ids.unit),
        "reason=unit_unregistered"
    );
    // 合并后：已登记项原样保留，运行时建筑补 false、单位补 true。
    let effective = effective_movability(&assembly, &view);
    assert_eq!(effective.get(&ids.building), Some(&false));
    assert_eq!(effective.get(&ids.unit), Some(&true));

    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let commands = runtime_group_move(&view, &mapper, &assembly);
    let targets = move_targets(&commands);

    // 运行时建筑 MUST NOT 生成 Move。
    assert!(
        runtime_building_does_not_move(&commands, ids.building),
        "reason=runtime_building_must_not_move"
    );
    assert!(
        !targets.contains(&ids.building),
        "reason=no_move_for_building"
    );
    // 运行时战斗单位与 frame-0 生产者正常响应 Move，按稳定 EntityId 顺序。
    let mut expected = vec![ids.producer, ids.unit];
    expected.sort_unstable();
    assert_eq!(targets, expected, "reason=stable_entity_id_order");
    assert!(targets.contains(&ids.unit), "reason=runtime_unit_moves");
    assert!(
        targets.contains(&ids.producer),
        "reason=frame0_peasant_moves"
    );
    // 位置不变：没有任何指向运行时建筑的 Move 命令，视图坐标不变。
    assert!(!commands.iter().any(|command| matches!(
        command,
        ClientCommand::Simulation(Command {
            kind: CommandKind::Move { entity, .. },
            ..
        }) if entity.value() == ids.building
    )));
    let (building_x, building_y) = (building.x, building.y);
    let building_after = view.entities.iter().find(|e| e.id == ids.building).unwrap();
    assert_eq!(building_after.x, building_x, "reason=building_x_unchanged");
    assert_eq!(building_after.y, building_y, "reason=building_y_unchanged");

    // 表现层只读：apply 不反写权威状态，状态哈希 / 快照 / 版本不变。
    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=apply_must_not_change_state_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=apply_must_not_change_snapshot"
    );
    assert_eq!(SNAPSHOT_VERSION, 4, "reason=snapshot_version_4");
    assert_eq!(snapshot_before.version, SNAPSHOT_VERSION);
    assert_eq!(snapshot_before.defs_hash, defs.content_hash());
    assert_eq!(snapshot_before.content_hash, defs.content_hash());
}

/// 等效脚本逐帧验收（tasks 5.1 / ADR 0002）：混合选中（含运行时建筑）过滤后
/// 的命令与「只选中可移动单位」的等效脚本逐条相同；两个同构实例提交同一
/// 脚本后逐帧状态哈希 / 快照 / 只读视图一致，运行时建筑位置不变，快照仍 v4。
#[test]
fn runtime_move_filter_matches_movable_only_script_frame_by_frame() {
    let (mut a, defs, view, mapper, assembly, ids) = runtime_fixture();
    let (mut b, _) = runtime_production_match();
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=isomorphic_instances_start_equal"
    );

    // 混合选中（含运行时建筑）走真实 apply 群体 Move 路径。
    let mixed = runtime_group_move(&view, &mapper, &assembly);
    // 等效脚本：视图只保留可移动单位，同一拾取映射与装配期映射。
    let movers_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: view
            .entities
            .iter()
            .filter(|e| e.id != ids.building)
            .cloned()
            .collect(),
    };
    let only = runtime_group_move(&movers_view, &mapper, &assembly);
    assert_eq!(mixed, only, "reason=filter_must_not_change_landing_slots");
    assert!(move_targets(&mixed).contains(&ids.unit));
    assert!(!move_targets(&mixed).contains(&ids.building));

    let building_before = view.entities.iter().find(|e| e.id == ids.building).unwrap();
    let (building_x, building_y) = (building_before.x, building_before.y);

    // 命令经公开入口提交给两同构实例（权威状态所有者的合法行为）。
    for command in &mixed {
        if let ClientCommand::Simulation(cmd) = command {
            a.submit_command(*cmd).unwrap();
            b.submit_command(*cmd).unwrap();
        }
    }
    for frame in 0..12 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=hash_diverged frame={frame}"
        );
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=snapshot_diverged frame={frame}"
        );
        assert_eq!(
            a.read_view(),
            b.read_view(),
            "reason=view_diverged frame={frame}"
        );
    }
    // 运行时建筑未收到 Move，位置不变；快照仍为 v4。
    let after = a.read_view();
    let building_after = after
        .entities
        .iter()
        .find(|e| e.id == ids.building)
        .unwrap();
    assert_eq!(building_after.x, building_x, "reason=building_x_unchanged");
    assert_eq!(building_after.y, building_y, "reason=building_y_unchanged");
    assert_eq!(SNAPSHOT_VERSION, 4, "reason=snapshot_version_4");
    let snapshot = a.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION);
    assert_eq!(snapshot.defs_hash, defs.content_hash());
    assert_eq!(snapshot.content_hash, defs.content_hash());
}

/// frame-0 回归不退化（P8.0 / D2 铁律）：frame-0 主城 / 金矿的视图 speed
/// 被 max(1) 抬成 >0，但已登记 false 必须原样保留；分层判据不得把
/// frame-0 建筑误放行（否则 R1 bug 从 frame-0 路径复发）。
#[test]
fn frame0_mixed_selection_not_regressed_by_effective_movability() {
    let (view, mapper, map) = mixed_fixture();
    // 既有夹具的 frame-0 实体（含建筑 / 资源点）运行时视图 speed 全 >0。
    assert!(
        view.entities.iter().all(|e| e.speed > 0),
        "reason=frame0_view_speed_lifted"
    );
    // 三个实体都已登记，合并函数不新增也不覆盖任何项。
    let effective = effective_movability(&map, &view);
    assert_eq!(effective, map, "reason=registered_frame0_entries_preserved");
    assert_eq!(effective.get(&HALL_ID), Some(&false));
    assert_eq!(effective.get(&MINE_ID), Some(&false));

    let commands = runtime_group_move(&view, &mapper, &map);
    assert_eq!(
        move_targets(&commands),
        vec![PEASANT_ID],
        "reason=only_frame0_peasant_moves"
    );
    assert_eq!(
        view.entities[1].x,
        10 * SCALE,
        "reason=hall_position_unchanged"
    );
    assert_eq!(
        view.entities[2].x,
        -10 * SCALE,
        "reason=mine_position_unchanged"
    );
}

/// 默认空映射兜底（D5 / tasks 4.1 / B1 另一半）：`ClientApp::new` 默认空映射
/// 与 `examples/playable.rs` 未注入态，view 含运行时生产建筑（speed == 0）
/// 时 MUST NOT 生成 Move，效果与「显式注入正确映射」一致，空映射不再触发
/// `unwrap_or(true)` 全放行。
#[test]
fn default_empty_movability_does_not_release_runtime_building() {
    let (_, _, view, mapper, _, ids) = runtime_fixture();
    let empty = MovabilityMap::new();

    // 空映射经合并函数兜底后，view 里每个实体都有 speed 判据。
    let effective = effective_movability(&empty, &view);
    assert_eq!(
        effective.len(),
        view.entities.len(),
        "reason=every_entity_has_criterion"
    );
    assert_eq!(
        effective.get(&ids.building),
        Some(&false),
        "reason=building_immovable"
    );
    assert_eq!(effective.get(&ids.unit), Some(&true), "reason=unit_movable");
    assert_eq!(
        effective.get(&ids.producer),
        Some(&true),
        "reason=producer_movable"
    );

    let commands = runtime_group_move(&view, &mapper, &empty);
    assert!(
        runtime_building_does_not_move(&commands, ids.building),
        "reason=empty_map_must_not_release_building"
    );
    assert!(
        move_targets(&commands).contains(&ids.unit),
        "reason=unit_still_moves"
    );

    // 与「显式注入正确映射」效果一致：按视图 speed 逐条登记的正确映射。
    let injected: MovabilityMap = view.entities.iter().map(|e| (e.id, e.speed != 0)).collect();
    let injected_commands = runtime_group_move(&view, &mapper, &injected);
    assert_eq!(
        move_targets(&commands),
        move_targets(&injected_commands),
        "reason=default_matches_explicit_injection"
    );
}

/// 反向验证·正式态（`CODEBUDDY.md` §5 / D4）：合并函数在场时，
/// 「运行时建筑不响应 Move」断言经真实 `InputState::apply` 路径通过。
#[test]
fn reverse_validation_formal_impl_passes_runtime_building_assertion() {
    let (_, _, view, mapper, assembly, ids) = runtime_fixture();
    let commands = runtime_group_move(&view, &mapper, &assembly);
    assert!(
        runtime_building_does_not_move(&commands, ids.building),
        "reason=formal_impl_passes_the_assertion"
    );
    // 同一断言经真实 filter_movable 路径：有效映射过滤掉运行时建筑。
    let selection: BTreeSet<u64> = view.entities.iter().map(|e| e.id).collect();
    let formal = filter_movable(&selection, &effective_movability(&assembly, &view));
    assert!(
        !formal.contains(&ids.building),
        "reason=formal_filter_narrows_set"
    );
}

/// 退化合并函数（缺陷态，仅用于反向验证）：模拟「合并函数缺席」，
/// 直接把装配期映射原样交回 `filter_movable`，未登记的运行时实体
/// 回到 `unwrap_or(true)` 默认放行（即 B1 缺陷态）。生产实现 MUST 用
/// [`effective_movability`]（design D3 / D4）。
fn degenerate_effective_movability(base: &MovabilityMap, _view: &WorldView) -> MovabilityMap {
    base.clone()
}

/// 反向验证·缺陷态（`CODEBUDDY.md` §5 / D4）：去掉动态实体兜底、退回旧
/// `unwrap_or(true)` 默认放行，令「运行时建筑不响应 Move」断言失败。
/// 退化实现经真实 `filter_movable` 与 `InputState::apply` 路径注入，不是旁路模拟。
#[test]
fn reverse_validation_degenerate_impl_fails_the_same_assertion() {
    let (_, _, view, mapper, assembly, ids) = runtime_fixture();
    let selection: BTreeSet<u64> = view.entities.iter().map(|e| e.id).collect();

    // A/B 纯函数对照：合并函数在场过滤建筑，缺席（退化）则默认放行。
    let formal = filter_movable(&selection, &effective_movability(&assembly, &view));
    let degenerate = filter_movable(
        &selection,
        &degenerate_effective_movability(&assembly, &view),
    );
    assert!(
        !formal.contains(&ids.building),
        "reason=formal_merger_filters_building"
    );
    assert!(
        degenerate.contains(&ids.building),
        "reason=absent_merger_releases_building"
    );

    // 退化映射：把「旧 unwrap_or(true) 默认放行」的结果显式化（所有实体 true）。
    // 合并函数「已登记项原样保留」不会覆盖它，因此可经真实 apply 路径复现
    // 缺陷态；元断言证明它等价于空映射的 unwrap_or(true) 全放行。
    let degenerate_map: MovabilityMap = view.entities.iter().map(|e| (e.id, true)).collect();
    assert_eq!(
        filter_movable(&selection, &degenerate_map),
        filter_movable(&selection, &MovabilityMap::new()),
        "reason=degenerate_map_equals_unwrap_or_true"
    );
    assert_eq!(
        filter_movable(&selection, &degenerate_map),
        degenerate,
        "reason=degenerate_impl_injected_via_real_filter_movable"
    );

    // 缺陷态经真实 InputState::apply 群体 Move 路径：运行时建筑被放行。
    let broken_commands = runtime_group_move(&view, &mapper, &degenerate_map);
    assert!(
        move_targets(&broken_commands).contains(&ids.building),
        "reason=degenerate_impl_generates_move_for_building"
    );
    // 元断言：同一「建筑不 Move」断言在退化实现下必然失败（改坏验红）。
    assert!(
        !runtime_building_does_not_move(&broken_commands, ids.building),
        "reason=degenerate_impl_fails_the_same_assertion"
    );
}

/// 双刷新率逐帧验收（ADR 0002 §2）：含运行时动态实体的真实生产对局，
/// 60 Hz / 144 Hz 呈现节奏喂入同一整数节拍序列后，逐帧帧号与
/// `compute_state_hash()` 完全一致；累加器只数整数帧，`step()` 不接呈现帧率。
fn runtime_replay_trace(refresh_hz: u64, seconds: u64) -> Vec<(u64, u64)> {
    let ticks_per_redraw = TICKS_PER_SECOND / refresh_hz;
    assert_eq!(ticks_per_redraw * refresh_hz, TICKS_PER_SECOND);
    let (sim, _) = runtime_production_match();
    let mut client = FixedStepClient::from_match(sim);
    let mut trace = Vec::new();
    for _ in 0..(refresh_hz * seconds) {
        client.advance_with_observer(ticks_per_redraw, |s| {
            trace.push((s.read_view().tick, s.compute_state_hash()));
        });
    }
    trace
}

#[test]
fn runtime_dynamic_entities_replay_is_refresh_rate_independent() {
    let seconds = 2;
    let at_60 = runtime_replay_trace(60, seconds);
    let at_144 = runtime_replay_trace(144, seconds);
    let expected_frames = u64::from(LOGIC_FPS) * seconds;

    assert_eq!(
        at_60.len() as u64,
        expected_frames,
        "reason=60hz_frame_count"
    );
    assert_eq!(
        at_144.len() as u64,
        expected_frames,
        "reason=144hz_frame_count"
    );
    assert_eq!(at_60, at_144, "reason=60hz_144hz_frame_hash_identical");
    // 逐帧帧号连续递增，防止「状态从未变化」造成的空泛通过。
    for (index, (tick, _)) in at_60.iter().enumerate() {
        // 生产夹具已推进 3 帧，累加器从当前冻结帧号继续。
        assert_eq!(*tick, 3 + index as u64 + 1, "reason=frame_number_monotonic");
    }
    assert!(
        at_60.windows(2).any(|pair| pair[0].1 != pair[1].1),
        "reason=simulation_actually_advanced"
    );
}

/// playable / 默认态显式处置（D5 / tasks 4.2）：`examples/playable.rs` 场景为
/// 4 个 speed > 0 的 `Spawn` 单位、无建筑，`ClientApp::new` 默认空映射未注入
/// `with_movability`。合并函数从只读视图 speed 现算兜底后，四个单位都有判据
/// 且全部可移动，行为正确——如实登记「兜底已覆盖」，无需补一个明确注入：
/// playable 是纯 `Spawn` 脚本、没有装配期 `def_ids` 关联可供派生，注入映射
/// 反而多余且会掩盖兜底是否生效。
#[test]
fn playable_default_empty_map_keeps_all_spawn_units_movable() {
    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    // 复刻 playable 的四个分散 spawn 单位：speed 均为 3 * SCALE > 0。
    for (sequence, x) in [(1u32, -32i64), (2, 0), (3, 32), (4, 0)] {
        driver.submit_command(spawn(sequence, x, 100)).unwrap();
    }
    driver.advance_one_frame();
    let view = driver.sim().read_view();
    assert_eq!(view.entities.len(), 4, "reason=four_playable_spawn_units");
    assert!(
        view.entities.iter().all(|e| e.speed > 0),
        "reason=all_playable_units_have_positive_speed"
    );

    // 默认空映射经合并函数兜底后，四个单位都按 speed 补为可移动。
    let empty = MovabilityMap::new();
    let effective = effective_movability(&empty, &view);
    assert_eq!(effective.len(), 4, "reason=empty_map_fully_backfilled");
    assert!(
        effective.values().all(|&movable| movable),
        "reason=all_units_movable_under_default_map"
    );

    // 走真实 apply 群体 Move 路径：默认态不再等价「全部可移动」缺陷态，
    // 而是每个单位都有 speed 判据、正常响应 Move。
    let commands = runtime_group_move(&view, &runtime_mapper(&view), &empty);
    let mut expected: Vec<u64> = view.entities.iter().map(|e| e.id).collect();
    expected.sort_unstable();
    assert_eq!(move_targets(&commands), expected, "reason=all_units_move");
}

// ---------------------------------------------------------------------------
// P8.2 兽族新增实体 R1.4 / R13.6 分层可移动性回归（C4 / D7 / tasks 3.5 / 6.x）
//
// 与 P8.1 人族新增实体回归对称：兽族远程 / 反重甲 / 英雄 / 苦工可移动，
// 人口地洞 / 祭坛 / 战争磨坊 / 主城 / 金矿 / 树木不可移动。判据由装配期
// `UnitDef.speed_raw` 派生的只读映射提供（权威），运行时动态实体由只读
// `EntityView.speed == 0` 补齐，两条来源 MUST NOT 互替、MUST NOT 回退白名单、
// MUST NOT 扩视图。本 change 不擅自闭合 P8.0 已知限制 B1。
// ---------------------------------------------------------------------------

/// 兽族混合 frame-0 静态装配夹具：兽族六类新实体 + 主城 / 金矿 / 树木，
/// 判据来自 `synthetic_definitions()` 的 `UnitDef.speed_raw`；视图 speed
/// 一律给 3*SCALE（模拟 `Spawn` 的 `max(1)` 抬升），证明 frame-0 判据
/// 不来自运行时视图 `speed`。
fn orc_fixture() -> (WorldView, ScreenMapper, MovabilityMap, BTreeMap<u64, DefId>) {
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![
            human_entity(H_PEASANT, 0, 60),
            human_entity(H_RANGED, 10, 110),
            human_entity(H_SIEGE, 20, 210),
            human_entity(H_HERO, 30, 460),
            human_entity(H_FARM, 40, 240),
            human_entity(H_ALTAR, 50, 340),
            human_entity(H_SMITHY, 60, 320),
            human_entity(H_HALL, 70, 120),
            human_entity(H_MINE, 80, 500),
            human_entity(H_TREE, 90, 200),
        ],
    };
    let mut boxes = BTreeMap::new();
    let row_one = [H_PEASANT, H_RANGED, H_SIEGE, H_HERO, H_FARM];
    let row_two = [H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE];
    for (index, &id) in row_one.iter().enumerate() {
        let x0 = 100.0 + 180.0 * index as f64;
        boxes.insert(id, ([x0, 100.0], [x0 + 40.0, 140.0]));
    }
    for (index, &id) in row_two.iter().enumerate() {
        let x0 = 100.0 + 180.0 * index as f64;
        boxes.insert(id, ([x0, 300.0], [x0 + 40.0, 340.0]));
    }
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);
    let def_ids: BTreeMap<u64, DefId> = [
        (H_PEASANT, ORC_PEASANT),
        (H_RANGED, ORC_RANGED),
        (H_SIEGE, ORC_SIEGE),
        (H_HERO, ORC_HERO),
        (H_FARM, ORC_FARM),
        (H_ALTAR, ORC_ALTAR),
        (H_SMITHY, ORC_SMITHY),
        (H_HALL, ORC_TOWN_HALL),
        (H_MINE, NEUTRAL_GOLD_MINE),
        (H_TREE, NEUTRAL_TREE),
    ]
    .into_iter()
    .collect();
    let movability = derive_movability(&synthetic_definitions(), &def_ids);
    (view, mapper, movability, def_ids)
}

/// 被反向验证的断言：混合集合只为兽族可移动单位（苦工 / 远程 / 反重甲 /
/// 英雄）生成 `Move`，新增建筑 / 主城 / 金矿 / 树木 MUST NOT 生成 `Move`。
fn only_movable_orc_units_move(commands: &[ClientCommand]) -> bool {
    !commands.is_empty() && move_targets(commands) == vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]
}

/// 退化定义（缺陷态）：把三类兽族新增建筑的 `speed_raw` 改成非 0，模拟
/// 「新定义未遵循 `speed_raw == 0`」的缺陷实现。经真实
/// `FrozenDefinitions::new` 重建、`derive_movability` 派生，不是旁路模拟。
fn degenerate_defs_with_movable_orc_buildings() -> FrozenDefinitions {
    let defs = synthetic_definitions();
    let units: Vec<(DefId, UnitDef)> = defs
        .units()
        .iter()
        .map(|(&id, &def)| {
            let mut def = def;
            if matches!(id, ORC_FARM | ORC_ALTAR | ORC_SMITHY) {
                def.speed_raw = 3 * SCALE;
            }
            (id, def)
        })
        .collect();
    let abilities = defs
        .abilities()
        .iter()
        .map(|(&id, d)| (id, d.clone()))
        .collect();
    let items = defs.items().iter().map(|(&id, &d)| (id, d)).collect();
    let upgrades = defs.upgrades().iter().map(|(&id, &d)| (id, d)).collect();
    FrozenDefinitions::new(units, abilities, items, upgrades)
        .expect("movability: reason=degenerate_orc_defs_rebuild_failed")
}

/// 兽族新增实体可移动性由定义层 `speed_raw` 派生（design D7 / R1.4）：远程 /
/// 反重甲 / 英雄 / 苦工可移动；人口地洞 / 祭坛 / 战争磨坊 / 主城 / 金矿 /
/// 树木不可移动；MUST NOT 用运行时 `speed`（视图里它们也都是 3*SCALE）。
#[test]
fn orc_new_defs_movability_is_derived_from_speed_raw() {
    let (_, _, map, _) = orc_fixture();
    assert_eq!(map.len(), 10, "reason=one_entry_per_orc_registered_entity");
    for id in [H_PEASANT, H_RANGED, H_SIEGE, H_HERO] {
        assert_eq!(map.get(&id), Some(&true), "reason=movable_orc id={id}");
    }
    for id in [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE] {
        assert_eq!(map.get(&id), Some(&false), "reason=immovable_orc id={id}");
    }
    let defs = synthetic_definitions();
    for def_id in [ORC_FARM, ORC_ALTAR, ORC_SMITHY] {
        assert_eq!(
            defs.units().get(&def_id).unwrap().speed_raw,
            0,
            "reason=orc_new_building_speed_raw_zero def={def_id}"
        );
    }
    // 既有兽族地基单位也遵循同一 `speed_raw` 判据（不回退类型名 / 白名单）：
    // 士兵可移动、兵营不可移动。
    let legacy = derive_movability(
        &defs,
        &[(90u64, ORC_SOLDIER), (91u64, ORC_BARRACKS)]
            .into_iter()
            .collect(),
    );
    assert_eq!(
        legacy.get(&90),
        Some(&true),
        "reason=orc_soldier_derived_movable"
    );
    assert_eq!(
        legacy.get(&91),
        Some(&false),
        "reason=orc_barracks_derived_immovable"
    );
}

/// 混合框选集合（兽族苦工 / 远程 / 反重甲 / 英雄 + 新增人口地洞 / 祭坛 /
/// 战争磨坊 + 主城 / 金矿 / 树木）右键与键盘 `M`：只为可移动单位生成
/// `Move`，建筑 / 资源点 / 主城 MUST NOT 收到 `Move`、位置不变，落点按
/// 稳定 `EntityId` 顺序，且与「只选中可移动单位」等效脚本逐条一致。
#[test]
fn orc_mixed_selection_only_moves_movable_units() {
    let (view, mapper, map, _) = orc_fixture();

    // 右键 `ContextAction` 路径。
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 10, "reason=all_ten_orc_selected");
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    let targets = move_targets(&commands);
    assert_eq!(
        targets,
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO],
        "reason=orc_stable_entity_id_order"
    );
    assert!(
        !input.no_movable_target(),
        "reason=orc_movable_targets_exist"
    );
    for id in [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE] {
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                ClientCommand::Simulation(Command {
                    kind: CommandKind::Move { entity, .. },
                    ..
                }) if entity.value() == id
            )),
            "reason=orc_immovable_must_not_move id={id}"
        );
    }
    assert_eq!(view.entities[4].x, 40 * SCALE, "reason=orc_farm_position");
    assert_eq!(view.entities[7].x, 70 * SCALE, "reason=orc_hall_position");

    // 键盘 `M` 路径（`MoveAim` 进入瞄准态 + `AimPoint` 复用同一命令生成处），
    // R13.6 要求键盘 M 与右键共用 R1.4 分层过滤。
    let mut keyboard = InputState::new(1);
    select_all(&mut keyboard, &view, &mapper, &map);
    let aim = keyboard
        .apply(InputAction::MoveAim, &view, &mapper, &[], &map)
        .unwrap();
    assert!(aim.is_empty(), "reason=orc_move_aim_is_presentation_only");
    assert_eq!(
        keyboard.aim(),
        w3_client::AimState::Move,
        "reason=orc_keyboard_m_sets_move_aim"
    );
    let keyboard_commands = keyboard
        .apply(
            InputAction::AimPoint([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        move_targets(&keyboard_commands),
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO],
        "reason=orc_keyboard_m_matches_right_click"
    );

    // 等效脚本：只含四个可移动单位，右键命令字节逐条相同（落点不受过滤影响）。
    let movers_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: view
            .entities
            .iter()
            .filter(|entity| targets.contains(&entity.id))
            .cloned()
            .collect(),
    };
    let mut only = InputState::new(1);
    select_all(&mut only, &movers_view, &mapper, &map);
    let only_commands = only
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &movers_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        commands, only_commands,
        "reason=orc_landing_slots_unchanged_by_filter"
    );
}

/// 全为兽族新增建筑 / 主城 / 资源点时右键与键盘 `M` 都不生成任何 `Move`，
/// 有明确无可执行目标反馈，权威状态哈希与快照不变（R1.4 / R13.6）。
#[test]
fn orc_all_immovable_selection_emits_no_move_with_feedback() {
    let (view, mapper, map, _) = orc_fixture();
    let immovable_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: view
            .entities
            .iter()
            .filter(|entity| {
                [H_FARM, H_ALTAR, H_SMITHY, H_HALL, H_MINE, H_TREE].contains(&entity.id)
            })
            .cloned()
            .collect(),
    };
    // 右键路径。
    let mut input = InputState::new(1);
    select_all(&mut input, &immovable_view, &mapper, &map);
    assert_eq!(
        input.selection().len(),
        6,
        "reason=only_orc_immovable_selected"
    );
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &immovable_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert!(commands.is_empty(), "reason=orc_no_move_commands");
    assert!(
        input.no_movable_target(),
        "reason=orc_explicit_feedback_flag"
    );

    // 键盘 `M` 路径：同样是全不可移动集合，无 `Move`、有反馈。
    let mut keyboard = InputState::new(1);
    select_all(&mut keyboard, &immovable_view, &mapper, &map);
    keyboard
        .apply(InputAction::MoveAim, &immovable_view, &mapper, &[], &map)
        .unwrap();
    let keyboard_commands = keyboard
        .apply(
            InputAction::AimPoint([900.0, 600.0]),
            &immovable_view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert!(
        keyboard_commands.is_empty(),
        "reason=orc_keyboard_m_no_move_commands"
    );
    assert!(
        keyboard.no_movable_target(),
        "reason=orc_keyboard_m_explicit_feedback_flag"
    );
}

/// 全为兽族新增建筑时 `ClientApp` 权威状态哈希 / 快照不变：判据来自冻结
/// 定义 `speed_raw`，而非运行时 `speed`（R1.4 / ADR 0002）。
#[test]
fn orc_new_buildings_all_immovable_keep_state_hash_and_snapshot() {
    let defs = synthetic_definitions();
    let entity_defs: BTreeMap<u64, DefId> =
        [(1u64, ORC_FARM), (2u64, ORC_ALTAR), (3u64, ORC_SMITHY)]
            .into_iter()
            .collect();
    let movability = derive_movability(&defs, &entity_defs);
    for id in [1u64, 2, 3] {
        assert_eq!(movability.get(&id), Some(&false));
    }
    assert_eq!(SNAPSHOT_VERSION, 4, "reason=snapshot_version_unchanged");

    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0, 240)).unwrap();
    driver.submit_command(spawn(2, 4, 340)).unwrap();
    driver.submit_command(spawn(3, 8, 320)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
        .with_movability(movability);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);

    let hash_before = app.driver().sim().compute_state_hash();
    let snapshot_before = app.driver().sim().capture_snapshot();
    let mapper = app.screen_mapper();
    let center = mapper.world_to_screen([0.0, 0.0]);
    let target = [center[0] + 320.0, center[1] + 220.0];
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(!app.selection().is_empty());
    let commands = app.apply_input(InputAction::ContextAction(target)).unwrap();
    assert!(commands.is_empty(), "reason=orc_no_commands_generated");
    assert!(app.no_movable_target(), "reason=orc_explicit_feedback");
    assert_eq!(
        app.driver().sim().compute_state_hash(),
        hash_before,
        "reason=orc_all_immovable_must_not_change_state_hash"
    );
    assert_eq!(
        app.driver().sim().capture_snapshot(),
        snapshot_before,
        "reason=orc_all_immovable_must_not_change_snapshot"
    );
    assert!(app.last_error().is_none(), "reason=orc_no_input_error");
}

/// 反向验证·修复态（D9 / tasks 6.2）：正式定义（`speed_raw == 0`）派生的
/// 映射经真实 `InputState::apply` 右键路径通过「只移动可移动单位」断言。
#[test]
fn orc_reverse_validation_formal_defs_pass_the_assertion() {
    let (view, mapper, map, _) = orc_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 10, "reason=all_ten_orc_selected");
    let commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert!(
        only_movable_orc_units_move(&commands),
        "reason=formal_orc_defs_must_pass_the_assertion"
    );
    assert_eq!(
        filter_movable(input.selection(), &map),
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]
    );
}

/// 反向验证·缺陷态（D9 / tasks 6.2）：把兽族新增建筑 `speed_raw` 改为非 0
/// 后，真实 `derive_movability` 把它们判为可移动，同一「只移动可移动单位」
/// 断言经真实 `InputState::apply` 必然失败；正式定义仍通过（元断言防退化）。
#[test]
fn orc_reverse_validation_degenerate_defs_fail_the_same_assertion() {
    let (view, mapper, _, def_ids) = orc_fixture();
    let degenerate_map = derive_movability(&degenerate_defs_with_movable_orc_buildings(), &def_ids);
    for id in [H_FARM, H_ALTAR, H_SMITHY] {
        assert_eq!(
            degenerate_map.get(&id),
            Some(&true),
            "reason=degenerate_orc_building_movable id={id}"
        );
    }
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &degenerate_map);
    let broken_commands = input
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &degenerate_map,
        )
        .unwrap();
    assert_eq!(
        move_targets(&broken_commands),
        vec![
            H_PEASANT, H_RANGED, H_SIEGE, H_HERO, H_FARM, H_ALTAR, H_SMITHY
        ],
        "reason=degenerate_orc_defs_move_new_buildings"
    );
    assert!(
        !only_movable_orc_units_move(&broken_commands),
        "reason=degenerate_orc_defs_must_fail_the_same_assertion"
    );
    // 正式定义仍能通过同一断言（修复态），两条证据成对。
    let formal = derive_movability(&synthetic_definitions(), &def_ids);
    let mut fixed = InputState::new(1);
    select_all(&mut fixed, &view, &mapper, &formal);
    let fixed_commands = fixed
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &formal,
        )
        .unwrap();
    assert!(only_movable_orc_units_move(&fixed_commands));
}

/// 反向验证·缺陷态（D9 / tasks 6.5）：退化过滤器（不过滤 / 把新增建筑当
/// 可移动）与「用运行时视图 `speed` 替代 frame-0 装配映射」都经真实
/// `filter_movable` / `InputState::apply` 路径注入，混合集合断言必然失败；
/// 正式实现仍通过。两条来源分层、不互替（R1.4 / R13.6）。
#[test]
fn orc_reverse_validation_degenerate_filter_fails_the_same_assertion() {
    let (view, mapper, map, _) = orc_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    let selection = input.selection().clone();

    let formal = filter_movable(&selection, &map);
    let degenerate = degenerate_filter(&selection, &map);
    assert_eq!(formal, vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]);
    assert_eq!(
        degenerate.len(),
        10,
        "reason=degenerate_filter_moves_orc_buildings_and_resources"
    );

    // 退化 = 空映射 `unwrap_or(true)` 全放行（等价于不过滤），且不会覆盖
    // frame-0 已登记判据；证明退化实现真经 `filter_movable` 注入。
    let degenerate_map = MovabilityMap::new();
    assert_eq!(
        filter_movable(&selection, &degenerate_map),
        degenerate,
        "reason=degenerate_filter_equals_degenerate_map_on_same_selection"
    );
    let mut broken = InputState::new(1);
    select_all(&mut broken, &view, &mapper, &degenerate_map);
    let broken_commands = broken
        .apply(
            InputAction::ContextAction([900.0, 600.0]),
            &view,
            &mapper,
            &[],
            &degenerate_map,
        )
        .unwrap();
    assert_eq!(
        move_targets(&broken_commands).len(),
        10,
        "reason=degenerate_impl_generates_move_for_all_orc"
    );
    assert!(
        !only_movable_orc_units_move(&broken_commands),
        "reason=degenerate_orc_filter_must_fail_the_same_assertion"
    );
    // 正式实现 `effective_movability` 合并后的分层判据仍通过同一断言。
    assert_eq!(
        filter_movable(&selection, &effective_movability(&map, &view)),
        vec![H_PEASANT, H_RANGED, H_SIEGE, H_HERO]
    );
}

/// 运行时 `Train` / `Build` 产出的兽族建筑（`speed_raw == 0`、运行时视图
/// `EntityView.speed == 0`、不在装配期 `def_ids` 关联中）经
/// `effective_movability` 判为不可移动；产出的兽族战斗单位正常响应 `Move`。
/// 本 change 沿用 `runtime-entity-movability` 承接、不擅自改 `unwrap_or`
/// 语义 / 不扩视图（P8.0 已知限制 B1）。
#[test]
fn orc_runtime_building_does_not_move_but_unit_does() {
    let defs = synthetic_definitions();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    sim.submit_command(spawn(1, 0, 100))
        .expect("movability: reason=orc_producer_spawn_rejected");
    let producer = EntityId::new(1).expect("movability: reason=invalid_orc_producer_id");
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        1,
        GameplayCommandKind::Train {
            producer,
            unit_def: ORC_FARM,
            is_building: true,
        },
    ))
    .expect("movability: reason=orc_building_train_rejected");
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(1),
        1,
        2,
        GameplayCommandKind::Train {
            producer,
            unit_def: ORC_RANGED,
            is_building: false,
        },
    ))
    .expect("movability: reason=orc_unit_train_rejected");
    // ORC_FARM train_frames=20、ORC_RANGED=24，推进到完成帧后视图含
    // 生产者 + 建筑 + 战斗单位三个实体。
    for _ in 0..28 {
        sim.step();
    }
    let view = sim.read_view();
    assert_eq!(
        view.entities.len(),
        3,
        "reason=orc_producer_plus_two_runtime"
    );
    let runtime: Vec<&EntityView> = view.entities.iter().filter(|e| e.id != 1).collect();
    let building = runtime
        .iter()
        .find(|e| e.speed == 0)
        .expect("movability: reason=orc_runtime_building_missing")
        .id;
    let unit = runtime
        .iter()
        .find(|e| e.speed > 0)
        .expect("movability: reason=orc_runtime_unit_missing")
        .id;
    // 装配期映射只登记 frame-0 生产者（ORC_PEASANT 可移动），运行时实体
    // 刻意不在关联中——精确复现 B1 场景。
    let assembly = derive_movability(&defs, &[(1u64, ORC_PEASANT)].into_iter().collect());
    let effective = effective_movability(&assembly, &view);
    assert_eq!(effective.get(&building), Some(&false));
    assert_eq!(effective.get(&unit), Some(&true));
    let commands = runtime_group_move(&view, &runtime_mapper(&view), &assembly);
    let targets = move_targets(&commands);
    assert!(
        !targets.contains(&building),
        "reason=orc_runtime_building_not_moved"
    );
    assert!(targets.contains(&unit), "reason=orc_runtime_unit_moved");
    assert!(
        runtime_building_does_not_move(&commands, building),
        "reason=orc_runtime_formal_assertion_passes"
    );
    // 退化实现（合并函数缺席、退回 `unwrap_or(true)` 默认放行）必然失败。
    let degenerate = filter_movable(
        &view.entities.iter().map(|e| e.id).collect(),
        &degenerate_effective_movability(&assembly, &view),
    );
    assert!(
        degenerate.contains(&building),
        "reason=degenerate_orc_merger_releases_runtime_building"
    );
    let degenerate_map: MovabilityMap = view.entities.iter().map(|e| (e.id, true)).collect();
    let broken = runtime_group_move(&view, &runtime_mapper(&view), &degenerate_map);
    assert!(
        !runtime_building_does_not_move(&broken, building),
        "reason=degenerate_orc_impl_fails_the_same_assertion"
    );
}
