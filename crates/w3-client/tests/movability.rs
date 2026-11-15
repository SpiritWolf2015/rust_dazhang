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
    HUMAN_PEASANT, HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE, ORC_PEASANT, ORC_TOWN_HALL,
};
use w3_client::melee::synthetic_definitions;
use w3_client::{
    ClientApp, ClientCommand, FixedStepClient, InputAction, InputState, MovabilityMap,
    ScreenMapper, TICKS_PER_LOGIC_FRAME, derive_movability, filter_movable,
};
use w3_core::{
    Command, CommandKind, DefId, EntityView, Frame, SCALE, SNAPSHOT_VERSION, SimFixed, StaticWorld,
    WorldView,
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
