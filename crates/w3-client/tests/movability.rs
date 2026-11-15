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
    HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE, NEUTRAL_TREE, ORC_PEASANT, ORC_TOWN_HALL,
};
use w3_client::melee::synthetic_definitions;
use w3_client::{
    ClientApp, ClientCommand, FixedStepClient, InputAction, InputState, MovabilityMap,
    ScreenMapper, TICKS_PER_LOGIC_FRAME, derive_movability, filter_movable,
};
use w3_core::{
    Command, CommandKind, DefId, EntityView, Frame, FrozenDefinitions, SCALE, SNAPSHOT_VERSION,
    SimFixed, StaticWorld, UnitDef, WorldView,
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
    assert_eq!(SNAPSHOT_VERSION, 3, "reason=snapshot_version_unchanged");

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
    assert_eq!(SNAPSHOT_VERSION, 3, "reason=snapshot_version_unchanged");

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
