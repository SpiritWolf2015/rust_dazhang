//! `p9-command-hotkeys` CPU 回归（D9 / C5 / `CODEBUDDY.md` §5）。
//!
//! 全部为合成命令脚本 / 合成视图 / 真实 `RtsMatch` 生产夹具，零资产、
//! 零文件 IO、不开 GPU 窗口、不引入墙钟。核心断言：
//! - 经典键 / 命令卡点击汇入同一 `InputState::apply`、只产出既有命令面，
//!   `S`=Stop 与 `H`=Hold 不合并、`A` 不化为 attack-move、无新命令变体；
//! - 两级 chords（B→A）/ 子菜单栈 / 放置态 / 同一字母随卡切换 / Esc 取消；
//! - 编队 / Tab 子组 / Shift 队列（`Move { append: true }`）全为表现层只读；
//! - 键盘 `M` / `A` 复用 R1.4 分层判据（frame-0 装配实体 + 运行时动态实体），
//!   建筑 / 金矿 / 主城 / 运行时建筑 MUST NOT 生成 `Move`；
//! - 反向验证（改坏验红）：退化实现经**真实** `filter_movable` / `apply`
//!   路径注入，缺陷态失败、修复态通过（反例元断言 / A-B 对照）；
//! - 状态哈希 / 快照 / `SNAPSHOT_VERSION=3` / golden hash 逐字节不变。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::melee::defs::{
    HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO, HUMAN_PEASANT, HUMAN_RANGED, HUMAN_SIEGE, HUMAN_SMITHY,
    HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE,
};
use w3_client::melee::{MeleeSetup, synthetic_definitions};
use w3_client::{
    AimState, ButtonDef, CardCommand, CardId, ClientApp, ClientCommand, CommandCard,
    CommandCatalog, FixedStepClient, HudAssets, HudModel, InputAction, InputState, LOGIC_FPS,
    MovabilityMap, PendingPlacement, ScreenMapper, TICKS_PER_LOGIC_FRAME, ZH, derive_movability,
    effective_movability, filter_movable, slot,
};
use w3_core::{
    Command, CommandKind, DefId, EntityId, EntityView, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, RtsMatch, SCALE, SNAPSHOT_VERSION, SimFixed, Snapshot,
    StaticWorld, UnitDef, WorldView,
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

/// 退化过滤器（缺陷态）：不过滤，把所有实体当可移动（复用
/// `tests/movability.rs::degenerate_filter` 先例，仅用于反向验证）。
fn degenerate_filter(selection: &BTreeSet<u64>, _: &MovabilityMap) -> Vec<u64> {
    selection.iter().copied().collect()
}

/// 农民 + 主城（建筑）+ 金矿（资源点）的 frame-0 混合夹具。
///
/// 三个实体视图 `speed` 都被 `max(1)` 抬成 `> 0`，故「frame-0 误用视图
/// speed」的退化实现会把建筑 / 资源点判为可移动（R1 bug）；正式映射由
/// 冻结定义 `speed_raw` 派生（建筑 / 资源点 `== 0`）。
fn mixed_fixture() -> (WorldView, ScreenMapper, MovabilityMap, BTreeMap<u64, DefId>) {
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
    let def_ids: BTreeMap<u64, DefId> = [
        (PEASANT_ID, HUMAN_PEASANT),
        (HALL_ID, HUMAN_TOWN_HALL),
        (MINE_ID, NEUTRAL_GOLD_MINE),
    ]
    .into_iter()
    .collect();
    let map = derive_movability(&synthetic_definitions(), &def_ids);
    (view, mapper, map, def_ids)
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

/// 应用一个归一化动作（省去空事件切片），返回命令流。
fn apply(
    input: &mut InputState,
    action: InputAction,
    view: &WorldView,
    mapper: &ScreenMapper,
    map: &MovabilityMap,
) -> Vec<ClientCommand> {
    input.apply(action, view, mapper, &[], map).unwrap()
}

/// 取出命令流里全部 `Move` 的目标实体 ID（稳定 EntityId 顺序）。
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

/// 被反向验证的单一断言：混合集合只有农民生成 `Move`。
fn only_peasant_moves(commands: &[ClientCommand]) -> bool {
    commands.len() == 1 && move_targets(commands) == vec![PEASANT_ID]
}

/// 键盘 `M`：`MoveAim`（表现层瞄准态、无命令）→ `AimPoint` 定点
/// （复用右键同一命令生成处与 R1.4 分层过滤）。
fn keyboard_move(
    input: &mut InputState,
    view: &WorldView,
    mapper: &ScreenMapper,
    map: &MovabilityMap,
    point: [f64; 2],
) -> Vec<ClientCommand> {
    let aim = apply(input, InputAction::MoveAim, view, mapper, map);
    assert!(aim.is_empty(), "reason=move_aim_is_presentation_only");
    assert_eq!(input.aim(), AimState::Move, "reason=aim_state_set");
    apply(input, InputAction::AimPoint(point), view, mapper, map)
}

/// 穷尽匹配 `CommandKind`：若 `w3-core` 新增 Patrol / Hold / AttackMove
/// 变体，本函数无法编译，构成「不新增命令变体」的编译期守卫（C4 / D7）。
fn command_kind_tag(kind: CommandKind) -> u8 {
    match kind {
        CommandKind::Noop => 0,
        CommandKind::Spawn { .. } => 1,
        CommandKind::Move { .. } => 2,
        CommandKind::Stop { .. } => 3,
        CommandKind::Attack { .. } => 4,
        CommandKind::SetObstacle { .. } => 5,
        CommandKind::RemoveObstacle { .. } => 6,
    }
}

/// 退化定义目录（缺陷态，仅反向验证用）：所有卡共用同一张全局静态键表，
/// 「同一字母随卡切换」与两级 chords 第二键都失效。生产实现 MUST 用
/// `CommandCatalog::standard()`（C5 / D9）。
fn degenerate_static_catalog() -> CommandCatalog {
    let buttons = [
        ButtonDef {
            def_id: 90_001,
            slot: slot(0, 0),
            hotkey: 'm',
            command: CardCommand::MoveAim,
            label: "Move",
        },
        ButtonDef {
            def_id: 90_002,
            slot: slot(1, 0),
            hotkey: 'a',
            command: CardCommand::AttackAim,
            label: "Attack",
        },
        ButtonDef {
            def_id: 90_003,
            slot: slot(2, 0),
            hotkey: 's',
            command: CardCommand::Stop,
            label: "Stop",
        },
        ButtonDef {
            def_id: 90_004,
            slot: slot(3, 0),
            hotkey: 'g',
            command: CardCommand::Gather,
            label: "Gather",
        },
        ButtonDef {
            def_id: 90_005,
            slot: slot(0, 1),
            hotkey: 'b',
            command: CardCommand::OpenBuildMenu,
            label: "Build",
        },
    ];
    let base = CommandCard::new(CardId::Main, &buttons);
    let mut cards = BTreeMap::new();
    for id in [
        CardId::Main,
        CardId::Worker,
        CardId::Producer,
        CardId::Build,
        CardId::Train,
        CardId::Shop,
    ] {
        cards.insert(
            id,
            CommandCard {
                id,
                buttons: base.buttons,
            },
        );
    }
    CommandCatalog::from_cards(cards, BTreeMap::new())
}

/// 「同一字母随卡切换」断言：正式目录为 true、退化目录为 false。
fn same_letter_changes_across_cards(catalog: &CommandCatalog) -> bool {
    catalog.key_actions(CardId::Worker).get(&'g') != catalog.key_actions(CardId::Build).get(&'g')
}

// ---------------------------------------------------------------------------
// 1. 键→命令表 / 键盘与点击等价 / 既有命令面边界（tasks 2.1 / 2.2 / 2.4）
// ---------------------------------------------------------------------------

#[test]
fn classic_hotkeys_normalize_to_existing_command_kinds() {
    let (view, mapper, map, defs) = mixed_fixture();
    // 注入装配关联，使当前卡按选中集合（含农民）派生成 Worker 卡，
    // 经典 `m` / `a` / `s` / `g` 键齐全（主卡不含工人专属采集键）。
    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);

    // `M` → 移动瞄准（表现层，无命令）；`A` → 攻击瞄准（无命令）。
    assert!(apply(&mut input, InputAction::Hotkey('m'), &view, &mapper, &map).is_empty());
    assert_eq!(input.aim(), AimState::Move);
    assert!(apply(&mut input, InputAction::Hotkey('a'), &view, &mapper, &map).is_empty());
    assert_eq!(input.aim(), AimState::Attack);
    // Esc 清空瞄准态、不生成命令。
    assert!(apply(&mut input, InputAction::CancelMenu, &view, &mapper, &map).is_empty());
    assert_eq!(input.aim(), AimState::None);

    // `S` → 既有 `Stop`；`G` → 既有 `Gather`，都经同一 apply。
    let stop = apply(&mut input, InputAction::Hotkey('s'), &view, &mapper, &map);
    assert_eq!(stop.len(), 3, "reason=one_stop_per_selected_entity");
    assert!(stop.iter().all(|command| matches!(
        command,
        ClientCommand::Simulation(Command {
            kind: CommandKind::Stop { .. },
            ..
        })
    )));
    let gather = apply(&mut input, InputAction::Hotkey('g'), &view, &mapper, &map);
    assert_eq!(gather.len(), 3, "reason=one_gather_per_selected_entity");
    assert!(gather.iter().all(|command| matches!(
        command,
        ClientCommand::Gameplay(GameplayCommand {
            kind: GameplayCommandKind::Gather { .. },
            ..
        })
    )));
}

#[test]
fn stop_attack_are_existing_variants_no_hold_or_attack_move() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);

    // `S` 只产出既有 `Stop`（穷尽匹配标签为 3），MUST NOT 合并 Hold。
    let stop = apply(&mut input, InputAction::Hotkey('s'), &view, &mapper, &map);
    for command in &stop {
        if let ClientCommand::Simulation(simulation) = command {
            assert_eq!(command_kind_tag(simulation.kind), 3, "reason=stop_only");
        }
    }
    // `A` 瞄准态点地面：不是 attack-move，MUST NOT 生成任何命令。
    apply(&mut input, InputAction::AttackAim, &view, &mapper, &map);
    let ground = apply(
        &mut input,
        InputAction::AimPoint([900.0, 600.0]),
        &view,
        &mapper,
        &map,
    );
    assert!(ground.is_empty(), "reason=no_attack_move_on_ground");
    // 穷尽匹配守卫：CommandKind 只有既有 7 个变体（无 Patrol/Hold/AttackMove）。
    assert_eq!(command_kind_tag(CommandKind::Noop), 0);
    assert_eq!(
        command_kind_tag(CommandKind::Attack {
            attacker: EntityId::new(1).unwrap(),
            target: EntityId::new(2).unwrap(),
        }),
        4
    );
}

#[test]
fn a_targets_existing_attack_command() {
    // 含一个敌方实体的视图：A 瞄准态点敌方 → 既有 `Attack { attacker, target }`。
    let mut view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![EntityView {
            id: 1,
            x: 0,
            y: 0,
            facing: 0,
            speed: 3 * SCALE,
            hp: 60,
            owner: 1,
            alive: true,
            has_target: false,
        }],
    };
    view.entities.push(EntityView {
        id: 2,
        x: 40 * SCALE,
        y: 0,
        facing: 0,
        speed: 3 * SCALE,
        hp: 60,
        owner: 2,
        alive: true,
        has_target: false,
    });
    // 点敌方实体包围盒中心（注入 box）。
    let mut boxes = BTreeMap::new();
    boxes.insert(1u64, ([100.0, 100.0], [140.0, 140.0]));
    boxes.insert(2u64, ([500.0, 500.0], [540.0, 540.0]));
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes);
    let map = MovabilityMap::new();
    let mut input = InputState::new(1);
    // 只选中己方单位 ID=1。
    input
        .apply(
            InputAction::SelectStart([100.0, 100.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    input
        .apply(
            InputAction::SelectEnd([140.0, 140.0]),
            &view,
            &mapper,
            &[],
            &map,
        )
        .unwrap();
    assert_eq!(
        input.selection(),
        &[1u64].into_iter().collect::<BTreeSet<u64>>()
    );

    apply(&mut input, InputAction::AttackAim, &view, &mapper, &map);
    let attack = apply(
        &mut input,
        InputAction::AimPoint([520.0, 520.0]),
        &view,
        &mapper,
        &map,
    );
    assert_eq!(attack.len(), 1, "reason=one_attack_for_selected_unit");
    assert!(matches!(
        attack[0],
        ClientCommand::Simulation(Command {
            kind: CommandKind::Attack { attacker, target },
            ..
        }) if attacker.value() == 1 && target.value() == 2
    ));
}

#[test]
fn keyboard_and_card_click_share_one_normalized_action() {
    let (view, mapper, map, _) = mixed_fixture();
    // 两个独立 InputState：一个按 `Hotkey('s')`（键盘），一个按「命令卡
    // Stop 格点击」回传的同一个 `Hotkey('s')`（点击），逐字段命令相同。
    let mut keyboard = InputState::new(1);
    let mut click = InputState::new(1);
    select_all(&mut keyboard, &view, &mapper, &map);
    select_all(&mut click, &view, &mapper, &map);
    let from_key = apply(
        &mut keyboard,
        InputAction::Hotkey('s'),
        &view,
        &mapper,
        &map,
    );
    let from_click = apply(&mut click, InputAction::Hotkey('s'), &view, &mapper, &map);
    assert_eq!(from_key, from_click, "reason=one_apply_no_source_marker");
    // 命令卡定义表里「Stop 格」的 hotkey 与 command 一致（点击按 hotkey ≡ 按键）。
    let catalog = CommandCatalog::standard();
    let stop_button = catalog
        .card(CardId::Main)
        .filled()
        .into_iter()
        .find(|button| button.command == CardCommand::Stop)
        .expect("reason=stop_button_exists");
    assert_eq!(stop_button.hotkey, 's');
    assert_eq!(
        catalog.key_actions(CardId::Main).get(&'s'),
        Some(&CardCommand::Stop)
    );
}

// ---------------------------------------------------------------------------
// 2. 两级 chords / 子菜单栈 / 放置态 / Esc / 同一字母随卡切换（tasks 3.x）
// ---------------------------------------------------------------------------

#[test]
fn build_chords_open_pick_place_emits_existing_train() {
    let (view, mapper, map, defs) = mixed_fixture();
    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);

    // `B` 开建造子菜单：入栈、不生成命令、当前卡切到 Build。
    let open = apply(&mut input, InputAction::Hotkey('b'), &view, &mapper, &map);
    assert!(open.is_empty(), "reason=open_menu_emits_no_command");
    assert_eq!(input.submenu_depth(), 1, "reason=stack_pushed");
    assert_eq!(input.current_card(), CardId::Build);

    // 第二键 `A` 选祭坛：转放置态、不生成命令。
    let pick = apply(&mut input, InputAction::Hotkey('a'), &view, &mapper, &map);
    assert!(pick.is_empty(), "reason=placement_emits_no_command");
    assert_eq!(
        input.placement(),
        Some(PendingPlacement {
            unit_def: HUMAN_ALTAR,
            is_building: true,
        })
    );

    // 左键定点才产出既有 `Train { is_building: true }`。
    let place = apply(
        &mut input,
        InputAction::AimPoint([900.0, 600.0]),
        &view,
        &mapper,
        &map,
    );
    assert_eq!(place.len(), 1, "reason=one_train_on_place");
    assert!(matches!(
        place[0],
        ClientCommand::Gameplay(GameplayCommand {
            kind: GameplayCommandKind::Train {
                unit_def,
                is_building: true,
                ..
            },
            ..
        }) if unit_def == HUMAN_ALTAR
    ));
    assert!(input.placement().is_none(), "reason=placement_consumed");
}

#[test]
fn esc_cancels_stack_and_placement_without_commands() {
    let (view, mapper, map, defs) = mixed_fixture();
    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);
    apply(&mut input, InputAction::Hotkey('b'), &view, &mapper, &map);
    apply(&mut input, InputAction::Hotkey('a'), &view, &mapper, &map);
    assert!(input.placement().is_some());
    assert_eq!(input.submenu_depth(), 1);

    // Esc：清空栈 / 放置态、回主卡、不生成命令。
    let cancel = apply(&mut input, InputAction::CancelMenu, &view, &mapper, &map);
    assert!(cancel.is_empty(), "reason=esc_emits_no_command");
    assert_eq!(input.submenu_depth(), 0, "reason=stack_cleared");
    assert!(input.placement().is_none(), "reason=placement_cleared");
    assert_eq!(input.current_card(), CardId::Worker, "reason=back_to_main");
    // 到定点为止全程没有命令；Esc 后定点也不再生成（放置态已清）。
    let after = apply(
        &mut input,
        InputAction::AimPoint([900.0, 600.0]),
        &view,
        &mapper,
        &map,
    );
    assert!(after.is_empty(), "reason=no_command_after_esc");
}

#[test]
fn same_letter_changes_meaning_and_old_binding_invalid() {
    let (view, mapper, map, defs) = mixed_fixture();
    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);

    // 主卡 `g` → 既有 `Gather` 命令。
    let gather = apply(&mut input, InputAction::Hotkey('g'), &view, &mapper, &map);
    assert!(gather.iter().all(|command| matches!(
        command,
        ClientCommand::Gameplay(GameplayCommand {
            kind: GameplayCommandKind::Gather { .. },
            ..
        })
    )));

    // 进建造子卡：旧卡 `m`（MoveAim）绑定失效，同一 `g` 变成建造项。
    apply(&mut input, InputAction::Hotkey('b'), &view, &mapper, &map);
    assert_eq!(input.current_card(), CardId::Build);
    assert!(
        input.key_actions().get(&'m').is_none(),
        "reason=old_binding_invalid"
    );
    assert_eq!(
        input.key_actions().get(&'g'),
        Some(&CardCommand::TrainUnit {
            unit_def: HUMAN_SMITHY,
            is_building: true,
        })
    );
    let build_pick = apply(&mut input, InputAction::Hotkey('g'), &view, &mapper, &map);
    assert!(build_pick.is_empty(), "reason=build_pick_is_placement");
    assert_eq!(
        input.placement(),
        Some(PendingPlacement {
            unit_def: HUMAN_SMITHY,
            is_building: true,
        })
    );
}

#[test]
fn submenu_fields_are_presentation_only() {
    let (view, mapper, map, defs) = mixed_fixture();
    // 子菜单栈 / 放置态 / 瞄准态字段只存在于 InputState；对本局权威状态
    // 无任何写入，compute_state_hash / capture_snapshot 逐字节不变。
    let mut sim = RtsMatch::create(MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    });
    sim.step();
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();

    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);
    apply(&mut input, InputAction::Hotkey('b'), &view, &mapper, &map);
    apply(&mut input, InputAction::Hotkey('a'), &view, &mapper, &map);
    apply(&mut input, InputAction::MoveAim, &view, &mapper, &map);
    assert_eq!(input.submenu_depth(), 1);
    assert!(input.placement().is_some());
    assert_eq!(input.aim(), AimState::Move);

    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=hash_unchanged"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=snapshot_unchanged"
    );
    assert_eq!(SNAPSHOT_VERSION, 3);
}

// ---------------------------------------------------------------------------
// 3. 编队 / Tab 子组 / Shift 队列（tasks 4.x）
// ---------------------------------------------------------------------------

#[test]
fn groups_and_subgroup_cursor_are_presentation_only() {
    let (view, mapper, map, defs) = mixed_fixture();
    let mut input = InputState::new(1).with_entity_defs(defs.clone());
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 3);

    // Ctrl+数字编组 / 数字选组 / Tab 子组切换都只改表现层选中集合。
    assert!(
        apply(
            &mut input,
            InputAction::AssignGroup(3),
            &view,
            &mapper,
            &map
        )
        .is_empty()
    );
    assert!(
        apply(
            &mut input,
            InputAction::SelectGroup(3),
            &view,
            &mapper,
            &map
        )
        .is_empty()
    );
    // 混编集合（农民 / 主城 / 金矿三类 DefId）Tab 应在子组间切换。
    assert!(apply(&mut input, InputAction::CycleSubgroup, &view, &mapper, &map).is_empty());
    let after_first = input.selection().clone();
    assert!(apply(&mut input, InputAction::CycleSubgroup, &view, &mapper, &map).is_empty());
    // 子组游标循环且只收窄表现层集合；编队允许含建筑，但命令分发仍按 R1.4。
    assert_ne!(input.subgroup_cursor(), 0);
    assert!(
        input
            .selection()
            .is_subset(&[PEASANT_ID, HALL_ID, MINE_ID].into_iter().collect())
    );
    assert!(!after_first.is_empty());
}

#[test]
fn shift_queue_emits_append_true_moves_with_stable_order() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    // Shift 开：后续命令逐条经既有 `Move { append: true }` 入队。
    apply(
        &mut input,
        InputAction::SetQueue(true),
        &view,
        &mapper,
        &map,
    );
    assert!(input.queue_append());
    let first = keyboard_move(&mut input, &view, &mapper, &map, [900.0, 600.0]);
    let second = keyboard_move(&mut input, &view, &mapper, &map, [950.0, 650.0]);
    // 每条都是既有 Move 且带 append=true，无新命令变体。
    for command in first.iter().chain(second.iter()) {
        match command {
            ClientCommand::Simulation(simulation) => match simulation.kind {
                CommandKind::Move { append, .. } => {
                    assert!(append, "reason=shift_queue_append_true");
                    assert_eq!(command_kind_tag(simulation.kind), 2);
                }
                other => panic!("unexpected command kind: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }
    // 命令序号按提交顺序稳定递增（不改排序键）。
    let sequences: Vec<u32> = first
        .iter()
        .chain(second.iter())
        .map(|command| match command {
            ClientCommand::Simulation(simulation) => simulation.sequence,
            _ => unreachable!(),
        })
        .collect();
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    // 关 Shift 后恢复 append=false（既有命令面行为不变）。
    apply(
        &mut input,
        InputAction::SetQueue(false),
        &view,
        &mapper,
        &map,
    );
    let plain = keyboard_move(&mut input, &view, &mapper, &map, [970.0, 670.0]);
    assert!(plain.iter().all(|command| matches!(
        command,
        ClientCommand::Simulation(Command {
            kind: CommandKind::Move { append: false, .. },
            ..
        })
    )));
}

// ---------------------------------------------------------------------------
// 4. 键盘 M / A 复用 R1.4 分层过滤（tasks 5.x，最关键）
// ---------------------------------------------------------------------------

#[test]
fn keyboard_m_only_peasant_moves_in_mixed_selection() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    assert_eq!(input.selection().len(), 3, "reason=all_three_selected");

    let commands = keyboard_move(&mut input, &view, &mapper, &map, [900.0, 600.0]);
    assert!(
        only_peasant_moves(&commands),
        "reason=only_peasant_may_move"
    );
    assert_eq!(move_targets(&commands), vec![PEASANT_ID]);
    assert!(!input.no_movable_target(), "reason=movable_target_exists");
    // 主城 / 金矿位置不变、没有任何指向它们的 Move。
    assert_eq!(view.entities[1].x, 10 * SCALE);
    assert_eq!(view.entities[2].x, -10 * SCALE);
    assert!(!move_targets(&commands).contains(&HALL_ID));
    assert!(!move_targets(&commands).contains(&MINE_ID));

    // 落点与「只含农民」的等效脚本逐条相同（R1.4）。
    let peasants_view = WorldView {
        tick: view.tick,
        rng: view.rng,
        entities: vec![view.entities[0].clone()],
    };
    let mut only = InputState::new(1);
    select_all(&mut only, &peasants_view, &mapper, &map);
    let only_cmds = keyboard_move(&mut only, &peasants_view, &mapper, &map, [900.0, 600.0]);
    assert_eq!(
        commands, only_cmds,
        "reason=landing_slots_unchanged_by_filter"
    );
}

#[test]
fn keyboard_m_matches_right_click_command_generation() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut via_key = InputState::new(1);
    let mut via_click = InputState::new(1);
    select_all(&mut via_key, &view, &mapper, &map);
    select_all(&mut via_click, &view, &mapper, &map);
    let keyboard = keyboard_move(&mut via_key, &view, &mapper, &map, [900.0, 600.0]);
    let right_click = apply(
        &mut via_click,
        InputAction::ContextAction([900.0, 600.0]),
        &view,
        &mapper,
        &map,
    );
    // 同一命令生成处 `context_commands`：命令字节逐字段相同。
    assert_eq!(keyboard, right_click, "reason=one_command_generation_site");
}

#[test]
fn all_immovable_keyboard_m_emits_no_move_with_feedback() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut input = InputState::new(1);
    // 只框选主城 + 金矿（两个不可移动实体）。
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
        &[HALL_ID, MINE_ID].into_iter().collect::<BTreeSet<u64>>()
    );
    let commands = keyboard_move(&mut input, &view, &mapper, &map, [900.0, 600.0]);
    assert!(commands.is_empty(), "reason=no_move_commands");
    assert!(input.no_movable_target(), "reason=explicit_feedback_flag");
}

// ---- 运行时动态实体真实生产夹具（复用 `tests/movability.rs` 先例） ----

const RT_BUILDING_DEF: DefId = 1;
const RT_UNIT_DEF: DefId = 2;

struct RuntimeIds {
    producer: u64,
    building: u64,
    unit: u64,
}

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
    .expect("command_hotkeys: reason=runtime_definitions_invalid");
    (defs, building_speed_raw, unit_speed_raw)
}

fn runtime_production_match() -> (RtsMatch, FrozenDefinitions) {
    let (defs, _, _) = runtime_definitions();
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: defs.content_hash(),
    };
    let mut sim = RtsMatch::create_with_definitions(config, StaticWorld::default(), defs.clone());
    sim.submit_command(spawn(1, 0, 100))
        .expect("command_hotkeys: reason=producer_spawn_rejected");
    let producer = EntityId::new(1).expect("command_hotkeys: reason=invalid_producer_id");
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
    .expect("command_hotkeys: reason=building_train_rejected");
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
    .expect("command_hotkeys: reason=unit_train_rejected");
    for _ in 0..3 {
        sim.step();
    }
    (sim, defs)
}

fn runtime_mapper(view: &WorldView) -> ScreenMapper {
    let mut boxes = BTreeMap::new();
    for (index, entity) in view.entities.iter().enumerate() {
        let x0 = 100.0 + 200.0 * index as f64;
        boxes.insert(entity.id, ([x0, 100.0], [x0 + 40.0, 140.0]));
    }
    ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0).with_entity_boxes(boxes)
}

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
    assert_eq!(view.entities.len(), 3, "reason=producer_plus_two_runtime");
    let runtime: Vec<&EntityView> = view.entities.iter().filter(|e| e.id != 1).collect();
    let ids = RuntimeIds {
        producer: 1,
        building: runtime
            .iter()
            .find(|e| e.speed == 0)
            .expect("command_hotkeys: reason=runtime_building_missing")
            .id,
        unit: runtime
            .iter()
            .find(|e| e.speed > 0)
            .expect("command_hotkeys: reason=runtime_unit_missing")
            .id,
    };
    let def_ids: BTreeMap<u64, DefId> = [(ids.producer, RT_UNIT_DEF)].into_iter().collect();
    let assembly = derive_movability(&defs, &def_ids);
    let mapper = runtime_mapper(&view);
    (sim, defs, view, mapper, assembly, ids)
}

#[test]
fn runtime_building_does_not_move_under_keyboard_m() {
    let (sim, _defs, view, mapper, assembly, ids) = runtime_fixture();
    let building = view.entities.iter().find(|e| e.id == ids.building).unwrap();
    let unit = view.entities.iter().find(|e| e.id == ids.unit).unwrap();
    assert_eq!(building.speed, 0, "reason=building_speed_zero");
    assert!(unit.speed > 0, "reason=unit_speed_positive");
    assert_eq!(assembly.len(), 1, "reason=assembly_registers_frame0_only");

    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &assembly);
    assert_eq!(input.selection().len(), 3);
    let commands = keyboard_move(&mut input, &view, &mapper, &assembly, [900.0, 600.0]);
    let targets = move_targets(&commands);

    // 运行时建筑 MUST NOT 生成 Move；frame-0 生产者与运行时战斗单位响应。
    assert!(
        !targets.contains(&ids.building),
        "reason=runtime_building_must_not_move"
    );
    assert!(targets.contains(&ids.unit), "reason=runtime_unit_moves");
    assert!(
        targets.contains(&ids.producer),
        "reason=frame0_producer_moves"
    );
    let mut expected = vec![ids.producer, ids.unit];
    expected.sort_unstable();
    assert_eq!(targets, expected, "reason=stable_entity_id_order");
    assert_eq!(
        view.entities
            .iter()
            .find(|e| e.id == ids.building)
            .unwrap()
            .x,
        building.x
    );

    // 表现层只读：状态哈希 / 快照 / 版本不变。
    assert_eq!(sim.compute_state_hash(), hash_before);
    assert_eq!(sim.capture_snapshot(), snapshot_before);
    assert_eq!(snapshot_before.version, SNAPSHOT_VERSION);
    assert_eq!(SNAPSHOT_VERSION, 3);
}

// ---------------------------------------------------------------------------
// 5. 反向验证（改坏验红，tasks 6.1 / 6.2 / C5 / `CODEBUDDY.md` §5）
// ---------------------------------------------------------------------------

/// 反向验证·修复态：`MoveAim + AimPoint` 键盘路径经正式 R1.4 过滤，
/// 混合集合只有农民生成 `Move`（`CODEBUDDY.md` §5 修复态证据）。
#[test]
fn reverse_validation_keyboard_formal_filters_immovable() {
    let (view, mapper, map, _) = mixed_fixture();
    let mut input = InputState::new(1);
    select_all(&mut input, &view, &mapper, &map);
    let commands = keyboard_move(&mut input, &view, &mapper, &map, [900.0, 600.0]);
    assert!(only_peasant_moves(&commands), "reason=formal_filter_passes");
    // 正式 `filter_movable` 收窄到农民。
    assert_eq!(filter_movable(input.selection(), &map), vec![PEASANT_ID]);
}

/// 反向验证·缺陷态：`frame-0 误用视图 speed / 不过滤` 的退化映射经
/// **真实** `effective_movability` + `filter_movable` + `InputState::apply`
/// 键盘 `M` 路径注入，令「只有农民生成 Move」断言失败（改坏验红）。
#[test]
fn reverse_validation_keyboard_degenerate_fails_the_same_assertion() {
    let (view, mapper, _, _) = mixed_fixture();
    let selection: BTreeSet<u64> = [PEASANT_ID, HALL_ID, MINE_ID].into_iter().collect();

    // 退化映射：空映射 → 合并函数从只读视图 speed 补齐，而 fixture 的
    // frame-0 建筑视图 speed 被 max(1) 抬成 >0，故全判可移动（等价
    // 「frame-0 误用视图 speed / 不过滤 / 把建筑当可移动」缺陷，R1 bug）。
    let degenerate_map = MovabilityMap::new();
    let effective = effective_movability(&degenerate_map, &view);
    assert_eq!(
        filter_movable(&selection, &effective),
        vec![PEASANT_ID, HALL_ID, MINE_ID],
        "reason=degenerate_map_moves_buildings_and_resources"
    );
    // 元断言：退化映射经真实 filter_movable 路径等价 degenerate_filter。
    assert_eq!(
        filter_movable(&selection, &effective),
        degenerate_filter(&selection, &degenerate_map),
        "reason=degenerate_impl_equals_no_filter"
    );

    // 缺陷态经真实 InputState::apply 键盘 `M` 路径：建筑 / 资源点也生成 Move。
    let mut broken = InputState::new(1);
    select_all(&mut broken, &view, &mapper, &degenerate_map);
    let broken_commands =
        keyboard_move(&mut broken, &view, &mapper, &degenerate_map, [900.0, 600.0]);
    assert_eq!(
        move_targets(&broken_commands),
        vec![PEASANT_ID, HALL_ID, MINE_ID],
        "reason=degenerate_generates_move_for_all"
    );
    // 同一断言在退化实现下必然失败（缺陷态被抓出）。
    assert!(
        !only_peasant_moves(&broken_commands),
        "reason=degenerate_must_fail_the_same_assertion"
    );

    // 正式态（修复态）仍通过同一断言，两条证据成对。
    let formal = derive_movability(
        &synthetic_definitions(),
        &[
            (PEASANT_ID, HUMAN_PEASANT),
            (HALL_ID, HUMAN_TOWN_HALL),
            (MINE_ID, NEUTRAL_GOLD_MINE),
        ]
        .into_iter()
        .collect(),
    );
    let mut fixed = InputState::new(1);
    select_all(&mut fixed, &view, &mapper, &formal);
    let fixed_commands = keyboard_move(&mut fixed, &view, &mapper, &formal, [900.0, 600.0]);
    assert!(
        only_peasant_moves(&fixed_commands),
        "reason=formal_impl_passes"
    );
}

/// 反向验证·修复态：正式定义目录经真实 `apply` 完成 B→A chords，
/// 「同一字母随卡切换」成立。
#[test]
fn reverse_validation_chords_formal_passes() {
    let catalog = CommandCatalog::standard();
    assert!(
        same_letter_changes_across_cards(&catalog),
        "reason=formal_catalog_changes_letter_by_card"
    );
    let (view, mapper, map, defs) = mixed_fixture();
    let mut input = InputState::new(1).with_entity_defs(defs);
    select_all(&mut input, &view, &mapper, &map);
    apply(&mut input, InputAction::Hotkey('b'), &view, &mapper, &map);
    assert_eq!(input.current_card(), CardId::Build);
    apply(&mut input, InputAction::Hotkey('a'), &view, &mapper, &map);
    assert_eq!(
        input.placement(),
        Some(PendingPlacement {
            unit_def: HUMAN_ALTAR,
            is_building: true,
        }),
        "reason=formal_chords_place_altar"
    );
}

/// 反向验证·缺陷态：注入退化定义目录（全局静态键表 / 不清栈）到
/// **真实** `InputState::apply` 路径，令 B→A chords 与「同一字母随卡
/// 切换」断言失败（C5 / D9）。
#[test]
fn reverse_validation_chords_degenerate_fails_the_same_assertions() {
    let degenerate = degenerate_static_catalog();
    // 元断言：退化目录把键表退化成全局静态映射（正式目录随卡切换）。
    assert!(!same_letter_changes_across_cards(&degenerate));
    assert!(same_letter_changes_across_cards(&CommandCatalog::standard()));

    let (view, mapper, map, defs) = mixed_fixture();
    // 退化目录经真实 apply 路径注入（不是旁路模拟）。
    let mut broken = InputState::new(1)
        .with_entity_defs(defs.clone())
        .with_catalog(degenerate.clone());
    select_all(&mut broken, &view, &mapper, &map);
    apply(&mut broken, InputAction::Hotkey('b'), &view, &mapper, &map);
    assert_eq!(broken.current_card(), CardId::Build, "reason=stack_pushed");
    // 退化目录里 Build 卡的 `g` 仍是 Gather（静态映射，不随卡切换）；
    // 且没有建造第二键，`a` 退化为 AttackAim 而非进入放置态。
    assert_eq!(
        broken.key_actions().get(&'g'),
        Some(&CardCommand::Gather),
        "reason=degenerate_letter_does_not_change"
    );
    let pick = apply(&mut broken, InputAction::Hotkey('a'), &view, &mapper, &map);
    assert!(pick.is_empty());
    assert!(
        broken.placement().is_none(),
        "reason=degenerate_chords_must_fail"
    );
    assert_eq!(
        broken.aim(),
        AimState::Attack,
        "reason=degenerate_a_is_static_attack_aim"
    );

    // 正式实现（修复态）通过同一 chords 断言：Build 卡 `g` 是建造项、
    // `a` 进放置态、同一字母随卡切换。
    let mut fixed = InputState::new(1).with_entity_defs(defs);
    select_all(&mut fixed, &view, &mapper, &map);
    apply(&mut fixed, InputAction::Hotkey('b'), &view, &mapper, &map);
    assert_eq!(
        fixed.key_actions().get(&'g'),
        Some(&CardCommand::TrainUnit {
            unit_def: HUMAN_SMITHY,
            is_building: true,
        }),
        "reason=formal_letter_changes_by_card"
    );
    apply(&mut fixed, InputAction::Hotkey('a'), &view, &mapper, &map);
    assert_eq!(
        fixed.placement(),
        Some(PendingPlacement {
            unit_def: HUMAN_ALTAR,
            is_building: true,
        }),
        "reason=formal_chords_pass"
    );
}

// ---------------------------------------------------------------------------
// 6. 确定性实证 / i18n / HUD 12 格（tasks 6.3 / 6.4 / 7.1 / 7.2）
// ---------------------------------------------------------------------------

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

fn hash(app: &ClientApp<FixedStepClient>) -> u64 {
    app.driver().sim().compute_state_hash()
}

fn snapshot(app: &ClientApp<FixedStepClient>) -> Snapshot {
    app.driver().sim().capture_snapshot()
}

fn build_app() -> ClientApp<FixedStepClient> {
    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0, 100)).unwrap();
    driver.submit_command(spawn(2, 4, 120)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
        .with_entity_defs(
            [(1u64, HUMAN_PEASANT), (2u64, HUMAN_TOWN_HALL)]
                .into_iter()
                .collect(),
        );
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
    app
}

#[test]
fn hud_twelve_slot_card_and_labels_never_change_state_hash() {
    let mut rendered = build_app();
    let mut plain = build_app();
    let ctx = egui::Context::default();
    for _ in 0..12 {
        // 只读派生含 12 格命令卡的 HUD 并用 egui 绘制（纯 CPU）。
        let model = rendered.hud_model();
        let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            w3_client::draw(ctx, &model, &ZH, HudAssets { text_enabled: true });
        });
        let _shapes = output.shapes.len();
        output.drop_without_applying_deltas();
        rendered.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        assert_eq!(hash(&rendered), hash(&plain), "reason=hud_changed_hash");
        assert_eq!(
            snapshot(&rendered),
            snapshot(&plain),
            "reason=hud_changed_snapshot"
        );
        assert_eq!(snapshot(&rendered).version, SNAPSHOT_VERSION);
    }
    assert_eq!(SNAPSHOT_VERSION, 3, "reason=snapshot_version_3");
}

#[test]
fn melee_setup_def_ids_wire_into_definition_table() {
    // tasks 1.4：装配层 `EntityId → DefId` 关联注入后，每个 frame-0
    // 实体都能在只读定义表查到选中集合类型；P8.1 人族六类新 DefId
    // 各有命令卡条目（由定义表记录驱动，非建筑名白名单）。
    let catalog = CommandCatalog::standard();
    let setup = MeleeSetup::new();
    for &def_id in setup.def_ids().values() {
        let _kind = catalog.kind_of(def_id);
    }
    let trained = catalog.trained_def_ids();
    for id in [
        HUMAN_RANGED,
        HUMAN_SIEGE,
        HUMAN_FARM,
        HUMAN_ALTAR,
        HUMAN_SMITHY,
        HUMAN_HERO,
    ] {
        assert!(
            trained.contains(&id),
            "reason=missing_catalog_entry id={id}"
        );
    }
}

#[test]
fn presentation_state_and_classic_commands_replay_frame_by_frame() {
    // 两同构真实生产对局：连续切换命令卡 / 子菜单 / 编队 / Shift 队列并
    // 下达经典键命令后，逐帧 `compute_state_hash()` / `capture_snapshot()`
    // 字节一致；`SNAPSHOT_VERSION` 仍 3、golden hash 口径不变。
    let (mut a, defs) = runtime_production_match();
    let (mut b, _) = runtime_production_match();
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=start_equal"
    );
    let (_, _, view, mapper, assembly, ids) = runtime_fixture();

    let entity_defs: BTreeMap<u64, DefId> = [
        (ids.producer, RT_UNIT_DEF),
        (ids.building, RT_BUILDING_DEF),
        (ids.unit, RT_UNIT_DEF),
    ]
    .into_iter()
    .collect();
    let mut ia = InputState::new(1).with_entity_defs(entity_defs.clone());
    let mut ib = InputState::new(1).with_entity_defs(entity_defs);
    select_all(&mut ia, &view, &mapper, &assembly);
    select_all(&mut ib, &view, &mapper, &assembly);

    // 纯表现层操作（不生成命令）：子菜单、Esc、编组、Tab、Shift 开关。
    for input in [&mut ia, &mut ib] {
        apply(input, InputAction::Hotkey('b'), &view, &mapper, &assembly);
        apply(input, InputAction::CancelMenu, &view, &mapper, &assembly);
        apply(
            input,
            InputAction::AssignGroup(2),
            &view,
            &mapper,
            &assembly,
        );
        apply(
            input,
            InputAction::SelectGroup(2),
            &view,
            &mapper,
            &assembly,
        );
        apply(
            input,
            InputAction::SetQueue(true),
            &view,
            &mapper,
            &assembly,
        );
    }
    // 下达经典键命令（键盘 M → 既有 `Move { append: true }`），两实例同序列。
    let ca = keyboard_move(&mut ia, &view, &mapper, &assembly, [900.0, 600.0]);
    let cb = keyboard_move(&mut ib, &view, &mapper, &assembly, [900.0, 600.0]);
    assert_eq!(ca, cb, "reason=identical_command_sequence");
    assert!(!ca.is_empty(), "reason=classic_command_actually_issued");

    for command in &ca {
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
    let snapshot = a.capture_snapshot();
    assert_eq!(snapshot.version, SNAPSHOT_VERSION);
    assert_eq!(SNAPSHOT_VERSION, 3, "reason=snapshot_version_3");
    assert_eq!(
        snapshot.defs_hash,
        defs.content_hash(),
        "reason=defs_hash_unchanged"
    );
    assert_eq!(
        snapshot.content_hash,
        defs.content_hash(),
        "reason=content_hash_unchanged"
    );
}

#[test]
fn twelve_slot_card_derives_by_selection_kind_not_whitelist() {
    // 用真实 ClientApp 的只读视图 / 快照派生 HUD：选中农民（Worker）与
    // 只选建筑（Producer）得到不同 12 格卡；空选中 `command_card` 为 None。
    let app = build_app();
    let view = app.driver().sim().read_view();
    let snap = app.driver().sim().capture_snapshot();
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let selection: BTreeSet<u64> = [1u64].into_iter().collect();
    // 当前卡由「EntityId → DefId」关联查定义表派生（非建筑名白名单）。
    let worker = HudModel::collect(&view, &snap, &selection, &[], &mapper, 1).with_entity_defs(
        [(1u64, HUMAN_PEASANT), (2u64, HUMAN_TOWN_HALL)]
            .into_iter()
            .collect(),
    );
    let producer = HudModel::collect(&view, &snap, &selection, &[], &mapper, 1).with_entity_defs(
        [(1u64, HUMAN_TOWN_HALL), (2u64, HUMAN_TOWN_HALL)]
            .into_iter()
            .collect(),
    );
    let worker_card = worker.command_card.expect("reason=worker_card");
    let producer_card = producer.command_card.expect("reason=producer_card");
    assert_eq!(worker_card.buttons.len(), 12);
    assert_eq!(producer_card.buttons.len(), 12);
    assert_ne!(
        worker_card, producer_card,
        "reason=card_changes_with_selection_kind"
    );
    assert_eq!(worker_card.id, CardId::Worker);
    assert_eq!(producer_card.id, CardId::Producer);

    // 子菜单换卡：注入 Build 子卡直接得到建造格（R13.3）。
    let build = HudModel::collect(&view, &snap, &selection, &[], &mapper, 1)
        .with_entity_defs([(1u64, HUMAN_PEASANT)].into_iter().collect())
        .with_active_card(CardId::Build);
    assert_eq!(
        build.command_card.expect("reason=build_card").id,
        CardId::Build
    );

    let empty = BTreeSet::new();
    let none = HudModel::collect(&view, &snap, &empty, &[], &mapper, 1)
        .with_entity_defs([(1u64, HUMAN_PEASANT)].into_iter().collect());
    assert!(
        none.command_card.is_none(),
        "reason=empty_selection_no_card"
    );
}

// ---------------------------------------------------------------------------
// 7. B1 集成接线：经 ClientApp 的完整翻译链（红转绿证据，CODEBUDDY.md §5）
// ---------------------------------------------------------------------------

/// B1 回归夹具：真实 `ClientApp` 装配（对齐 `examples/melee.rs` 接线），
/// `.with_entity_defs(...)` 与 `.with_movability(...)` 都注入。id=1 农民
/// （Worker / 可移动）、id=2 主城（Producer / 不可移动）构成「混合集合含
/// 建筑」以验证 R1.4 过滤；`entity_defs` 同时喂 HUD 显示侧与输入翻译侧。
fn build_wired_app() -> ClientApp<FixedStepClient> {
    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0, 100)).unwrap();
    driver.submit_command(spawn(2, 4, 120)).unwrap();
    let defs: BTreeMap<u64, DefId> = [(1u64, HUMAN_PEASANT), (2u64, HUMAN_TOWN_HALL)]
        .into_iter()
        .collect();
    let movability = derive_movability(&synthetic_definitions(), &defs);
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)
        .with_entity_defs(defs)
        .with_movability(movability);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
    app
}

/// 经 `ClientApp::apply_input` 全屏框选（复用既有范例），断言选中非空。
fn select_all_via_app(app: &mut ClientApp<FixedStepClient>) {
    app.apply_input(InputAction::SelectStart([0.0, 0.0]))
        .unwrap();
    app.apply_input(InputAction::SelectEnd([1280.0, 720.0]))
        .unwrap();
    assert!(
        !app.selection().is_empty(),
        "reason=selection_via_client_app"
    );
}

/// B1-①：Worker 选中态经 ClientApp 的完整装配路径，命令卡键表与 HUD 同源。
/// 修复前 `InputState.entity_defs` 恒空 → `refresh_card` 恒 Main 卡 → 'g' 无
/// 绑定静默丢命令、'b' 打不开子菜单（现状红）；修复后同源注入 → 全绿。
#[test]
fn b1_client_app_worker_hotkeys_wire_entity_defs() {
    let mut app = build_wired_app();
    select_all_via_app(&mut app);

    // `Hotkey('g')` 经当前卡键表翻译产出既有 `Gather`（Worker 卡 'g'）。
    let gather = app.apply_input(InputAction::Hotkey('g')).unwrap();
    assert!(!gather.is_empty(), "reason=worker_gather_via_client_app");
    assert!(gather.iter().all(|command| matches!(
        command,
        ClientCommand::Gameplay(GameplayCommand {
            kind: GameplayCommandKind::Gather { .. },
            ..
        })
    )));

    // `Hotkey('b')` 开建造子菜单：不生成命令、当前卡切到 Build。
    let mut app_b = build_wired_app();
    select_all_via_app(&mut app_b);
    let open = app_b.apply_input(InputAction::Hotkey('b')).unwrap();
    assert!(open.is_empty(), "reason=open_menu_emits_no_command");
    assert_eq!(
        app_b.input().current_card(),
        CardId::Build,
        "reason=build_submenu_via_client_app"
    );
}

/// B1-②：键盘 M → 左键 AimPoint 定点经同一命令生成处产出过滤后的 Move。
/// 修复前 `AimPoint` 无生产事件（左键恒框选）；混合集合含主城（建筑）时
/// 只有农民生成 Move（复用 `effective_movability` + `filter_movable`，R1.4）。
#[test]
fn b1_client_app_left_click_aim_emits_filtered_move() {
    let mut app = build_wired_app();
    select_all_via_app(&mut app);

    // 键盘 M → 表现层移动瞄准态、不生成命令。
    let aim = app.apply_input(InputAction::Hotkey('m')).unwrap();
    assert!(aim.is_empty(), "reason=move_aim_is_presentation_only");
    assert_eq!(app.input().aim(), AimState::Move);

    // 左键定点决策：瞄准态下产出 `AimPoint`（修复前无生产事件 → 红）。
    let mapper = app.screen_mapper();
    let center = mapper.world_to_screen([0.0, 0.0]);
    let point = [center[0] + 320.0, center[1] + 220.0];
    assert_eq!(
        app.left_click_action(true, false, point),
        Some(InputAction::AimPoint(point)),
        "reason=left_click_produces_aim_point"
    );
    // 定点手势的释放无动作，MUST NOT 覆盖瞄准态已定命令或框选集合。
    assert_eq!(app.left_click_action(false, false, point), None);

    // 经同一 apply 产出过滤后的 Move：主城（不可移动）被 R1.4 筛除。
    let commands = app.apply_input(InputAction::AimPoint(point)).unwrap();
    assert_eq!(
        move_targets(&commands),
        vec![PEASANT_ID],
        "reason=only_peasant_moves_via_client_app"
    );
    assert_eq!(app.input().aim(), AimState::None, "reason=aim_consumed");
}

/// B1-③：命令卡 12 格点击回传的 Hotkey 经 ClientApp 都有明确去向，
/// 不静默丢命令；至少一条格直接产命令（修复前 Worker 卡的 'g' / 'b'
/// 在 Main 卡找不到 → 变无响应，现状红）。
#[test]
fn b1_client_app_command_card_clicks_have_clear_destination() {
    let catalog = CommandCatalog::standard();
    let buttons = catalog.card(CardId::Worker).filled();
    assert!(!buttons.is_empty(), "reason=worker_card_has_buttons");

    for button in &buttons {
        let mut app = build_wired_app();
        select_all_via_app(&mut app);
        // 12 格点击由 `HudInteraction.command_hotkey` 归一化为 `Hotkey`。
        let commands = app.apply_input(InputAction::Hotkey(button.hotkey)).unwrap();
        let has_destination = !commands.is_empty()
            || app.input().aim() != AimState::None
            || app.input().placement().is_some()
            || matches!(
                app.input().current_card(),
                CardId::Build | CardId::Train | CardId::Shop
            );
        assert!(
            has_destination,
            "reason=slot_click_has_no_destination hotkey={}",
            button.hotkey
        );
    }

    // 至少一条 12 格点击经 ClientApp 直接产命令（Worker 采集格）。
    let gather_button = buttons
        .iter()
        .find(|button| button.command == CardCommand::Gather)
        .expect("reason=worker_has_gather_button");
    let mut app = build_wired_app();
    select_all_via_app(&mut app);
    let commands = app
        .apply_input(InputAction::Hotkey(gather_button.hotkey))
        .unwrap();
    assert!(!commands.is_empty(), "reason=card_click_emits_command");
}
