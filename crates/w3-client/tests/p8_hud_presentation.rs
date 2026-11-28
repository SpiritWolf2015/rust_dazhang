//! P8.4 `p8-hud-presentation` Q1 + Q2 + Q3 CPU 回归（design D1 / D2 /
//! D3 / D6、`CODEBUDDY.md §5`）。全部为合成命令脚本 / 真实 `RtsMatch` /
//! 真实 `CommandCatalog` / 真实 `Scene` 生产夹具，零资产、零文件 IO、
//! 不开 GPU 窗口、不引入墙钟。核心断言：
//! - Q1 窗口内选族是表现层本地只读状态：确认后经既有
//!   `MeleeSetup::with_races` 派生开局，本地 / AI 席位 `def_ids` 落所选族
//!   段、中立恒 `9xx`，`Spawn` / `gameplay_commands` 逐字节不变，
//!   `SNAPSHOT_VERSION == 10`、逐帧 `compute_state_hash()` /
//!   `capture_snapshot()` 不变，AI 命令源按所选 `ai_race` 产出；
//! - Q2 命令卡按本地族只读查表分派：兽族 Producer / Build / Train 卡
//!   `TrainUnit { unit_def, .. }` 全落 `2xx`、无 `HUMAN_*`，人族装配
//!   `for_race(Human) == standard()` 逐字节不变，12 格 / chords 契约不变，
//!   `def_kinds` 补齐两族登记，未知族 / 缺表项明确回落；
//! - Q3 运行时动态实体按 `owner` 席位段 + 只读 `speed` 派生真实两族
//!   glTF 模型键（零内核改动：`EntityView` 无 `def_id`），命中真实
//!   `BakedModel` 而非四面体；14 键表与 `ENTITY_MODEL_FILES` 逐字一致、
//!   R-1 八类精确查表缺键回退、干净环境缺资产显式可诊断回退、模型键 /
//!   几何不进权威状态 / 哈希 / 快照、静态摆放链路不变；
//! - R1.4 / R13.6 可移动性分层判据不因 HUD 改动退化；
//! - 反向验证（改坏验红）：成对 `reverse_validation_*` A-B 对照，退化
//!   实现真经 `assemble_from_selection` / `for_race_with_targets` /
//!   `Scene::consume` / `ModelLibrary::resolve_with_reason` 生产路径注入、
//!   非旁路模拟（先例 `p8-dual-faction`）。

use std::collections::{BTreeMap, BTreeSet};

use w3_client::faction::{FactionSelection, assemble_from_selection};
use w3_client::melee::command_card::{
    CARD_SLOTS, CardCommand, CardId, CommandCatalog, DEFAULT_COMMAND_TARGETS, RACE_ROWS,
    RACE_TRAIN_TARGETS, RaceTargetTable, TrainCategory, slot,
};
use w3_client::melee::defs::{
    ABILITY, HUMAN_ALTAR, HUMAN_BARRACKS, HUMAN_FARM, HUMAN_HERO, HUMAN_PEASANT, HUMAN_RANGED,
    HUMAN_SIEGE, HUMAN_SMITHY, HUMAN_SOLDIER, HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE, NEUTRAL_TREE,
    NIGHT_ELF_SOLDIER, ORC_ALTAR, ORC_BARRACKS, ORC_FARM, ORC_HERO, ORC_RANGED, ORC_SIEGE,
    ORC_SMITHY, ORC_SOLDIER, ORC_TOWN_HALL, Race, UNDEAD_SOLDIER,
};
use w3_client::melee::{
    AI_PLAYER, AiScript, LOCAL_PLAYER, MeleeSetup, synthetic_definitions, synthetic_world,
};
use w3_client::runtime_models::{
    ENTITY_MODEL_KEYS, R1_UNMAPPED_DEFS, RuntimeModelPlan, SEGMENT_HUMAN, SEGMENT_MODEL_KEYS,
    SEGMENT_NEUTRAL, SEGMENT_NIGHT_ELF, SEGMENT_ORC, SEGMENT_UNDEAD, def_id_to_model_key,
    def_segment,
};
use w3_client::{
    ClientApp, ClientCommand, FixedStepClient, HudModel, InputAction, InputState, MovabilityMap,
    ScreenMapper, TICKS_PER_LOGIC_FRAME,
};
use w3_core::{
    Command, CommandKind, DefId, EntityView, Frame, GameplayCommand, GameplayCommandKind,
    MatchConfig, RtsMatch, SNAPSHOT_VERSION, SimFixed, StaticWorld, WorldView,
};
use w3_render::{
    BakedModel, BakedTriangle, FrameInput, Model, ModelLibrary, REASON_ASSET_MISSING, Scene,
    TerrainMesh, TerrainVertex,
};

const SEED: u64 = 0x5eed;
const CONTENT_HASH: u64 = 0xC0FFEE;
const LOGIC_FPS: u32 = 30;

/// P9 / Q1' 十六组合（本地族, AI 族）：四族 4×4 笛卡尔积，含同族组合。
const COMBINATIONS: [(Race, Race); 16] = [
    (Race::Human, Race::Human),
    (Race::Human, Race::Orc),
    (Race::Human, Race::Undead),
    (Race::Human, Race::NightElf),
    (Race::Orc, Race::Human),
    (Race::Orc, Race::Orc),
    (Race::Orc, Race::Undead),
    (Race::Orc, Race::NightElf),
    (Race::Undead, Race::Human),
    (Race::Undead, Race::Orc),
    (Race::Undead, Race::Undead),
    (Race::Undead, Race::NightElf),
    (Race::NightElf, Race::Human),
    (Race::NightElf, Race::Orc),
    (Race::NightElf, Race::Undead),
    (Race::NightElf, Race::NightElf),
];

/// 该种族 `units` 段的 `DefId` 区间（P9 五段中的四族段：
/// 人族 `1xx` / 兽族 `2xx` / 不死 `3xx` / 暗夜 `4xx`）。
fn race_segment(race: Race) -> std::ops::Range<DefId> {
    match race {
        Race::Human => 100..200,
        Race::Orc => 200..300,
        Race::Undead => 300..400,
        Race::NightElf => 400..500,
    }
}

/// 人族段全部命令卡目标常量（Q2「MUST NOT 出现 `HUMAN_*`」判据）。
fn human_target_ids() -> BTreeSet<DefId> {
    [
        HUMAN_SOLDIER,
        HUMAN_RANGED,
        HUMAN_SIEGE,
        HUMAN_HERO,
        HUMAN_ALTAR,
        HUMAN_FARM,
        HUMAN_BARRACKS,
        HUMAN_SMITHY,
        HUMAN_TOWN_HALL,
    ]
    .into_iter()
    .collect()
}

/// 从选族结果经既有 `with_races` 派生开局（Q1 生产唯一路径）。
fn assembled(selection: FactionSelection) -> MeleeSetup {
    assemble_from_selection(selection, MeleeSetup::with_races)
}

// ---------------------------------------------------------------------------
// Q1 选族入口：开局 DefId 分段 / 表现层只读 / AI 命令源（tasks 3.2 / 3.3）
// ---------------------------------------------------------------------------

/// 选族后开局 `def_ids` 分段正确：本地席位落 `local_race` 段、AI 席位落
/// `ai_race` 段、中立金矿 / 树木恒 `9xx`（R2 / R12.7）。
#[test]
fn q1_selection_assembles_segmented_opening_def_ids() {
    for (local, ai) in COMBINATIONS {
        let selection = FactionSelection::with_races(local, ai);
        let setup = assembled(selection);
        assert_eq!(setup.race, local, "reason=local_race_alias local={local:?}");
        assert_eq!(setup.ai_race, ai, "reason=ai_race_field ai={ai:?}");
        // 本地席位主城 + 三农民（ID 1..=4）落本地族段。
        for id in [1u64, 2, 3, 4] {
            let def_id = *setup
                .def_ids()
                .get(&id)
                .expect("reason=missing_local_def_id");
            assert!(
                race_segment(local).contains(&def_id),
                "reason=local_seat_in_local_segment id={id} def={def_id} local={local:?}"
            );
        }
        // AI 席位主城 + 三农民（ID 7..=10）落 AI 族段。
        for id in [7u64, 8, 9, 10] {
            let def_id = *setup.def_ids().get(&id).expect("reason=missing_ai_def_id");
            assert!(
                race_segment(ai).contains(&def_id),
                "reason=ai_seat_in_ai_segment id={id} def={def_id} ai={ai:?}"
            );
        }
        // 中立资源（ID 5,6,11,12）恒 9xx。
        for id in [5u64, 6, 11, 12] {
            let def_id = *setup
                .def_ids()
                .get(&id)
                .expect("reason=missing_neutral_def_id");
            assert!(
                (900..1000).contains(&def_id),
                "reason=neutral_in_9xx id={id} def={def_id}"
            );
        }
        assert_eq!(setup.def_ids().get(&5), Some(&NEUTRAL_GOLD_MINE));
        assert_eq!(setup.def_ids().get(&6), Some(&NEUTRAL_TREE));
    }
}

/// `Spawn` / `gameplay_commands` 既有入口逐字节不变：四组合与
/// `MeleeSetup::new()` 完全一致；`new` / `with_race` 兼容别名保持。
#[test]
fn q1_spawn_and_gameplay_bytes_are_race_independent() {
    let baseline = MeleeSetup::new();
    for (local, ai) in COMBINATIONS {
        let setup = assembled(FactionSelection::with_races(local, ai));
        assert_eq!(
            setup.commands, baseline.commands,
            "reason=spawn_bytes_equal"
        );
        assert_eq!(
            setup.gameplay_commands, baseline.gameplay_commands,
            "reason=opening_gameplay_bytes_equal"
        );
        assert_eq!(setup.kinds, baseline.kinds, "reason=kinds_bytes_equal");
        assert_eq!(setup.commands.len(), 12, "reason=opening_spawn_count");
        for command in &setup.commands {
            assert_eq!(command.execute_frame, Frame::new(0));
            assert!(matches!(command.kind, CommandKind::Spawn { .. }));
        }
    }
    // 兼容别名与既有回落基线逐字节一致（不静默放宽断言）。
    for race in [Race::Human, Race::Orc] {
        let alias = MeleeSetup::with_race(race);
        let dual = MeleeSetup::with_races(race, race.opponent());
        assert_eq!(alias.commands, dual.commands, "reason=alias_spawn_bytes");
        assert_eq!(alias.def_ids, dual.def_ids, "reason=alias_def_ids");
    }
    assert_eq!(MeleeSetup::new().commands, baseline.commands);
    assert_ne!(
        MeleeSetup::with_race(Race::Human).def_ids,
        MeleeSetup::with_race(Race::Orc).def_ids,
        "reason=def_ids_are_race_specific"
    );
}

/// 默认选族回落 `MeleeSetup::new()`（本地人族 / AI 兽族，Q1 未选族回落）。
#[test]
fn q1_default_selection_falls_back_to_new() {
    let selection = FactionSelection::default();
    assert_eq!(selection.local_race, Race::Human);
    assert_eq!(selection.ai_race, Race::Orc);
    assert!(!selection.confirmed);
    let setup = assembled(selection);
    let baseline = MeleeSetup::new();
    assert_eq!(setup.commands, baseline.commands);
    assert_eq!(setup.gameplay_commands, baseline.gameplay_commands);
    assert_eq!(setup.def_ids, baseline.def_ids);
    assert_eq!(setup.race, baseline.race);
    assert_eq!(setup.ai_race, baseline.ai_race);
    assert_eq!(
        setup.ai_category_def_ids(),
        baseline.ai_category_def_ids(),
        "reason=ai_category_binding_fallback"
    );
}

/// 选族是表现层本地只读状态：无新增权威状态字段、`SNAPSHOT_VERSION` 仍 10、
/// 逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与基线一致
/// （ADR 0002 / C1 / C2）。
#[test]
fn q1_selection_is_presentation_only_and_never_changes_hash_or_snapshot() {
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    let defs = synthetic_definitions();

    // 两个不同选族装配的真实对局（Spawn 字节相同）逐帧哈希 / 快照一致。
    let human = assembled(FactionSelection::with_races(Race::Human, Race::Orc));
    let orc = assembled(FactionSelection::with_races(Race::Orc, Race::Human));
    let mut sim_h = build_sim(&human, &defs);
    let mut sim_o = build_sim(&orc, &defs);
    for frame in 0..8 {
        assert_eq!(
            sim_h.compute_state_hash(),
            sim_o.compute_state_hash(),
            "reason=hash_selection_independent frame={frame}"
        );
        let snap_h = sim_h.capture_snapshot();
        let snap_o = sim_o.capture_snapshot();
        assert_eq!(
            snap_h, snap_o,
            "reason=snapshot_selection_independent frame={frame}"
        );
        assert_eq!(
            snap_h.version, SNAPSHOT_VERSION,
            "reason=snapshot_version_10"
        );
        assert_eq!(snap_h.defs_hash, defs.content_hash(), "reason=defs_hash");
        assert_eq!(snap_h.content_hash, CONTENT_HASH, "reason=content_hash");
        sim_h.step();
        sim_o.step();
    }
}

/// tasks 3.5：HUD 只读投影在 v10 新基线下复核（批次 A 四族投影已落地）。
///
/// - 不选族回落人族段：`FactionSelection::default()` 与 `MeleeSetup::new()`
///   的 `Spawn` / `gameplay_commands` / `def_ids` 逐字节一致；
/// - `derive(snapshot, events)` 只读派生范式不变：反复用 `HudModel::collect`
///   与 `FogGrid::observe` + `FogHudModel::derive`（含重复事件投递 / 迷雾
///   网格重算）派生后，真实 `RtsMatch` 的 `compute_state_hash()` /
///   `capture_snapshot()` 与派生前逐字节一致（HUD / 选中 / 相机 / 迷雾
///   MUST NOT 反写权威状态，`FogHudModel` 只读语义不变）；
/// - 十六组合四族投影逐帧哈希 / 快照与不选族 v10 基线一致，快照 v10 且头部
///   `defs_hash` / `content_hash` 一致；
/// - 不扩展 `WorldView` / `EntityView`（沿用 [`runtime_view`] 既有字段）。
#[test]
fn q1_hud_projection_is_read_only_and_matches_unselected_v10_baseline() {
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    let defs = synthetic_definitions();

    // 1) 不选族回落人族段（Q1 未选族回落）。
    let unselected = FactionSelection::default();
    assert_eq!(
        unselected.local_race,
        Race::Human,
        "reason=unselected_falls_back_to_human_segment"
    );
    assert_eq!(unselected.ai_race, Race::Orc, "reason=unselected_ai_is_orc");
    let baseline = MeleeSetup::new();
    let unselected_setup = assembled(unselected);
    assert_eq!(
        unselected_setup.commands, baseline.commands,
        "reason=fallback_spawn_bytes"
    );
    assert_eq!(
        unselected_setup.gameplay_commands, baseline.gameplay_commands,
        "reason=fallback_gameplay_bytes"
    );
    assert_eq!(
        unselected_setup.def_ids, baseline.def_ids,
        "reason=fallback_def_ids"
    );

    // 2) HUD / FogHud 只读派生不改权威哈希与快照（重复投递 / 网格重算）。
    let (_, _, mapper, entity_defs, selection) = hud_fixture();
    let mut sim = build_sim(&baseline, &defs);
    sim.step();
    let view = sim.read_view();
    let snap = sim.capture_snapshot();
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    assert_eq!(
        snapshot_before.version, SNAPSHOT_VERSION,
        "reason=baseline_snapshot_v10"
    );
    let mut fog_grid = w3_client::FogGrid::new(LOCAL_PLAYER);
    for _ in 0..8 {
        let _hud = HudModel::collect(&view, &snap, &selection, &[], &mapper, LOCAL_PLAYER);
        fog_grid.observe(&view, &snap, &[], &entity_defs, LOCAL_PLAYER);
        let _fog_hud = w3_client::FogHudModel::derive(&snap, &[], &fog_grid);
    }
    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=hud_derive_does_not_change_state_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=hud_derive_does_not_change_snapshot"
    );
    // `FogHudModel::derive` 只读消费快照 / 事件拷贝，不携带权威写句柄。
    let fog_hud = w3_client::FogHudModel::derive(&snap, &[], &fog_grid);
    assert_eq!(
        fog_hud.snapshot_tick, snap.tick,
        "reason=fog_hud_derives_snapshot_tick"
    );

    // 3) 十六组合四族投影与不选族 v10 基线逐帧一致（选族只停装配层）。
    for (local, ai) in COMBINATIONS {
        let setup = assembled(FactionSelection::with_races(local, ai));
        let mut selected = build_sim(&setup, &defs);
        let mut base = build_sim(&baseline, &defs);
        for frame in 0..8 {
            assert_eq!(
                selected.compute_state_hash(),
                base.compute_state_hash(),
                "reason=hud_projection_hash_matches_unselected_baseline frame={frame} local={local:?} ai={ai:?}"
            );
            let selected_snapshot = selected.capture_snapshot();
            let base_snapshot = base.capture_snapshot();
            assert_eq!(
                selected_snapshot, base_snapshot,
                "reason=hud_projection_snapshot_matches_unselected_baseline frame={frame} local={local:?} ai={ai:?}"
            );
            assert_eq!(
                selected_snapshot.version, SNAPSHOT_VERSION,
                "reason=hud_projection_snapshot_v10 frame={frame}"
            );
            assert_eq!(
                selected_snapshot.defs_hash,
                defs.content_hash(),
                "reason=hud_projection_defs_hash frame={frame}"
            );
            assert_eq!(
                selected_snapshot.content_hash, CONTENT_HASH,
                "reason=hud_projection_content_hash frame={frame}"
            );
            selected.step();
            base.step();
        }
    }
}

/// 命令卡按族刷新不改哈希与快照（ADR 0002 / R13.6）：真实 `ClientApp`
/// 连续切换选族 / 刷新命令卡并推进整数逻辑帧，逐帧哈希与快照与
/// 不切族的对照实例一致。
#[test]
fn q1_command_card_race_refresh_never_changes_hash_or_snapshot() {
    let mut rendered = build_app();
    let mut plain = build_app();
    let selections = [
        FactionSelection::with_races(Race::Orc, Race::Human),
        FactionSelection::with_races(Race::Human, Race::Human),
        FactionSelection::with_races(Race::Orc, Race::Orc),
    ];
    for frame in 0..12 {
        let selection = selections[frame % selections.len()];
        rendered.apply_faction_selection(selection);
        // 只读派生 HUD（含 12 格命令卡），不写回任何权威状态。
        let model = rendered.hud_model();
        assert_eq!(
            model.faction, selection,
            "reason=faction_is_read_only_state"
        );
        rendered.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        plain.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
        assert_eq!(
            hash(&rendered),
            hash(&plain),
            "reason=faction_refresh_changed_hash frame={frame}"
        );
        assert_eq!(
            snapshot(&rendered),
            snapshot(&plain),
            "reason=faction_refresh_changed_snapshot frame={frame}"
        );
        assert_eq!(snapshot(&rendered).version, SNAPSHOT_VERSION);
    }
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
}

/// 选族后 AI 命令源按所选 `ai_race` 产出（P8.3 回归接新入口）：
/// `AiScript::new` 的训练目标落 AI 族段，MUST NOT 泄漏对家族段；技能 /
/// 升级 ID 按 `ai_race` 只读分派。
#[test]
fn q1_ai_command_source_follows_selected_ai_race() {
    for (local, ai) in COMBINATIONS {
        let setup = assembled(FactionSelection::with_races(local, ai));
        let script = AiScript::new(&setup, 1200);
        let segment = race_segment(ai);
        let foreign = race_segment(ai.opponent());
        let mut saw_train = false;
        let mut saw_cast = false;
        let mut saw_research = false;
        for command in &script.commands {
            assert_eq!(
                command.player_id, 2,
                "reason=ai_player local={local:?} ai={ai:?}"
            );
            match command.kind {
                GameplayCommandKind::Train { unit_def, .. } => {
                    if unit_def == 1 {
                        // 基线士兵（1）两族共用、不分段。
                        continue;
                    }
                    assert!(
                        segment.contains(&unit_def),
                        "reason=ai_train_in_ai_segment id={unit_def} ai={ai:?}"
                    );
                    assert!(
                        !foreign.contains(&unit_def),
                        "reason=foreign_train_leaked id={unit_def} ai={ai:?}"
                    );
                    saw_train = true;
                }
                GameplayCommandKind::Cast { ability, .. } => {
                    if ability == ABILITY {
                        // 两族共用骚扰技能。
                        continue;
                    }
                    assert_eq!(
                        ability,
                        ai.hero_ability(),
                        "reason=ai_hero_ability ai={ai:?}"
                    );
                    saw_cast = true;
                }
                GameplayCommandKind::Research { upgrade } => {
                    assert!(
                        upgrade == ai.weapons_upgrade() || upgrade == ai.armor_upgrade(),
                        "reason=ai_research_matches_race upgrade={upgrade} ai={ai:?}"
                    );
                    saw_research = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_train && saw_cast && saw_research,
            "reason=ai_script_has_three_kinds local={local:?} ai={ai:?}"
        );
    }
}

/// 窗口内 egui overlay 装配链（Q1）：`with_faction_selection` 启用 overlay、
/// 默认本地人族 / AI 兽族、未确认；确认后状态机落定且按本地族只读查表
/// 刷新命令卡目录（Q2 联动），HUD 绘制不 panic、哈希 / 快照不变。
#[test]
fn q1_faction_overlay_state_machine_and_command_card_link() {
    let selection = FactionSelection::default();
    let mut app = build_app().with_faction_selection(selection);
    assert_eq!(
        app.faction_selection(),
        selection,
        "reason=builder_injects_selection"
    );
    assert_eq!(app.faction_selection().local_race, Race::Human);
    assert_eq!(app.faction_selection().ai_race, Race::Orc);
    assert!(
        !app.faction_selection().confirmed,
        "reason=pre_match_unconfirmed"
    );
    let model = app.hud_model();
    assert!(
        model.faction_overlay,
        "reason=overlay_visible_before_confirm"
    );
    // 人族目录：训练目标全落 1xx。
    assert!(
        app.input()
            .catalog()
            .trained_def_ids()
            .iter()
            .all(|def_id| (100..200).contains(def_id))
    );

    // 用真实 egui 上下文绘制一帧（含选族 overlay），只产生形状、不写回状态。
    let hash_before = hash(&app);
    let snapshot_before = snapshot(&app);
    let ctx = egui::Context::default();
    let output = ctx.run_pass(egui::RawInput::default(), |ctx| {
        w3_client::draw(
            ctx,
            &model,
            &w3_client::ZH,
            w3_client::HudAssets { text_enabled: true },
        );
    });
    assert!(!output.shapes.is_empty(), "reason=overlay_shapes_rendered");
    output.drop_without_applying_deltas();
    assert_eq!(hash(&app), hash_before, "reason=overlay_draw_changed_hash");

    // 确认切族：状态机落定、按族目录联动刷新、权威状态零改动。
    let confirmed = FactionSelection::with_races(Race::Orc, Race::Human);
    app.apply_faction_selection(confirmed);
    assert_eq!(app.faction_selection(), confirmed);
    assert!(app.faction_selection().confirmed, "reason=confirmed");
    assert!(
        app.input()
            .catalog()
            .trained_def_ids()
            .iter()
            .all(|def_id| (200..300).contains(def_id))
    );
    assert_eq!(hash(&app), hash_before, "reason=faction_changed_hash");
    assert_eq!(
        snapshot(&app),
        snapshot_before,
        "reason=faction_changed_snapshot"
    );
    assert_eq!(snapshot(&app).version, SNAPSHOT_VERSION);
}

/// R1.4 / R13.6 分层判据不因 HUD 改动退化：四组合装配下主城 / 金矿 /
/// 树木不可移动、农民可移动（判据来自冻结定义 `speed_raw`，非类型名）。
#[test]
fn q1_selection_preserves_r14_movability_filter() {
    let defs = synthetic_definitions();
    for (local, ai) in COMBINATIONS {
        let setup = assembled(FactionSelection::with_races(local, ai));
        let map = setup.movability(&defs);
        for id in [2u64, 3, 4, 8, 9, 10] {
            assert_eq!(
                map.get(&id),
                Some(&true),
                "reason=peasant_movable id={id} local={local:?} ai={ai:?}"
            );
        }
        for id in [1u64, 5, 6, 7, 11, 12] {
            assert_eq!(
                map.get(&id),
                Some(&false),
                "reason=building_or_resource_immovable id={id} local={local:?} ai={ai:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Q2 命令卡按本地族查表分派（tasks 4.1 / 4.2 / 4.3 / 4.4）
// ---------------------------------------------------------------------------

/// 全部 `TrainUnit { unit_def, .. }` 目标（按值，稳定 `BTreeSet` 顺序）。
fn trained_targets(catalog: &CommandCatalog) -> BTreeSet<DefId> {
    catalog.trained_def_ids()
}

/// Producer / Build / Train 卡的所有训练 / 建造目标（逐卡枚举）。
fn card_train_targets(catalog: &CommandCatalog, id: CardId) -> Vec<DefId> {
    catalog
        .card(id)
        .filled()
        .into_iter()
        .filter_map(|button| match button.command {
            CardCommand::TrainUnit { unit_def, .. } => Some(unit_def),
            _ => None,
        })
        .collect()
}

/// 兽族建筑点训练产兽族目标：`TrainUnit { unit_def, .. }` 落 `2xx` 段、
/// MUST NOT 出现 `HUMAN_*`（R2 / R4 / R13.3）。
#[test]
fn q2_orc_catalog_trains_orc_targets_and_never_human() {
    let catalog = CommandCatalog::for_race(Race::Orc);
    let human = human_target_ids();
    for id in [CardId::Producer, CardId::Build, CardId::Train] {
        let targets = card_train_targets(&catalog, id);
        assert!(
            !targets.is_empty(),
            "reason=card_has_train_targets id={id:?}"
        );
        for unit_def in &targets {
            assert!(
                (200..300).contains(unit_def),
                "reason=orc_target_in_2xx id={id:?} def={unit_def}"
            );
            assert!(
                !human.contains(unit_def),
                "reason=human_target_must_not_appear id={id:?} def={unit_def}"
            );
        }
    }
    // 全部训练目标集合也只落 2xx。
    for unit_def in trained_targets(&catalog) {
        assert!(
            (200..300).contains(&unit_def),
            "reason=trained_set_in_2xx def={unit_def}"
        );
    }
    // 兽族六类新类别目标都在表中。
    for def_id in [
        ORC_RANGED, ORC_SIEGE, ORC_FARM, ORC_ALTAR, ORC_SMITHY, ORC_HERO,
    ] {
        assert!(
            trained_targets(&catalog).contains(&def_id),
            "reason=missing_orc_new_category def={def_id}"
        );
    }
}

/// 人族装配行为逐字节不变：`for_race(Human) == standard()`，训练目标
/// 全落 `1xx`，12 格 / `slot = y*4+x` / 键位 / chords 契约原样保留。
#[test]
fn q2_human_catalog_is_byte_identical_to_standard() {
    let standard = CommandCatalog::standard();
    let human = CommandCatalog::for_race(Race::Human);
    assert_eq!(standard, human, "reason=human_catalog_unchanged");
    assert_eq!(CARD_SLOTS, 12);
    assert_eq!(slot(0, 0), 0);
    assert_eq!(slot(3, 2), 11);
    assert_eq!(slot(2, 2), 10);
    for id in standard.card_ids() {
        let card = standard.card(id);
        assert_eq!(card.buttons.len(), 12, "reason=twelve_slots id={id:?}");
        for (index, button) in card.buttons.iter().enumerate() {
            if let Some(button) = button {
                assert_eq!(button.slot as usize, index, "reason=slot_matches_index");
            }
        }
    }
    // 既有键位语义：主卡 `s`=Stop、工人卡 `b`=OpenBuildMenu、建造卡
    // `g`=兽族/人族 Smithy 建造项（按族查表，人族这里是 HUMAN_SMITHY）。
    assert_eq!(
        standard.key_actions(CardId::Main).get(&'s'),
        Some(&CardCommand::Stop)
    );
    assert_eq!(
        standard.key_actions(CardId::Worker).get(&'b'),
        Some(&CardCommand::OpenBuildMenu)
    );
    assert_eq!(
        standard.key_actions(CardId::Build).get(&'g'),
        Some(&CardCommand::TrainUnit {
            unit_def: HUMAN_SMITHY,
            is_building: true,
        })
    );
    for unit_def in trained_targets(&standard) {
        assert!(
            (100..200).contains(&unit_def),
            "reason=human_trained_set_in_1xx def={unit_def}"
        );
    }
    // 定义表人族行与人族冻结常量逐项一致（编译期整数表驱动）。
    let row = RACE_TRAIN_TARGETS[0];
    assert_eq!(row[TrainCategory::Soldier.index()], Some(HUMAN_SOLDIER));
    assert_eq!(row[TrainCategory::TownHall.index()], Some(HUMAN_TOWN_HALL));
    assert_eq!(
        DEFAULT_COMMAND_TARGETS[TrainCategory::Hero.index()],
        HUMAN_HERO
    );
}

/// 兽族定义表行与兽族冻结常量逐项一致；两族目标不重名、不相同。
#[test]
fn q2_race_target_table_drives_dispatch_without_if_race_or_whitelist() {
    let human_row = RACE_TRAIN_TARGETS[0];
    let orc_row = RACE_TRAIN_TARGETS[1];
    let pairs = [
        (TrainCategory::Soldier, HUMAN_SOLDIER, ORC_SOLDIER),
        (TrainCategory::Ranged, HUMAN_RANGED, ORC_RANGED),
        (TrainCategory::Siege, HUMAN_SIEGE, ORC_SIEGE),
        (TrainCategory::Hero, HUMAN_HERO, ORC_HERO),
        (TrainCategory::Altar, HUMAN_ALTAR, ORC_ALTAR),
        (TrainCategory::Farm, HUMAN_FARM, ORC_FARM),
        (TrainCategory::Barracks, HUMAN_BARRACKS, ORC_BARRACKS),
        (TrainCategory::Smithy, HUMAN_SMITHY, ORC_SMITHY),
        (TrainCategory::TownHall, HUMAN_TOWN_HALL, ORC_TOWN_HALL),
    ];
    for (category, human_def, orc_def) in pairs {
        let index = category.index();
        assert_eq!(
            human_row[index],
            Some(human_def),
            "reason=human_row category={category:?}"
        );
        assert_eq!(
            orc_row[index],
            Some(orc_def),
            "reason=orc_row category={category:?}"
        );
        assert!((100..200).contains(&human_def));
        assert!((200..300).contains(&orc_def));
        assert_ne!(human_def, orc_def, "reason=distinct_race_targets");
    }
    // P9 / Q1'：定义表四行，不死 `3xx` / 暗夜 `4xx` 两行整行落在
    // 各自段、9 列全 `Some`，且与既有两族行不重叠（4 行 × 9 列）。
    assert_eq!(RACE_ROWS, 4);
    assert_eq!(RACE_TRAIN_TARGETS.len(), 4);
    for (row_index, segment) in [(2usize, 300..400), (3, 400..500)] {
        for entry in RACE_TRAIN_TARGETS[row_index] {
            assert!(
                entry.is_some_and(|id| segment.contains(&id)),
                "reason=p9_row_in_segment row={row_index}"
            );
        }
    }
    assert_eq!(
        RACE_TRAIN_TARGETS[2][TrainCategory::Soldier.index()],
        Some(UNDEAD_SOLDIER)
    );
    assert_eq!(
        RACE_TRAIN_TARGETS[3][TrainCategory::Soldier.index()],
        Some(NIGHT_ELF_SOLDIER)
    );
}

/// `def_kinds` 补齐两族登记（含兽族六类新类别），`kind_of` 通用回落保持。
#[test]
fn q2_def_kinds_registers_both_factions_and_keeps_generic_fallback() {
    let catalog = CommandCatalog::for_race(Race::Orc);
    // 兽族六类新类别不再是「未登记 → 通用 Combat」回落。
    assert_eq!(catalog.kind_of(ORC_RANGED), w3_client::CardKind::Combat);
    assert_eq!(catalog.kind_of(ORC_SIEGE), w3_client::CardKind::Combat);
    assert_eq!(catalog.kind_of(ORC_FARM), w3_client::CardKind::Producer);
    assert_eq!(catalog.kind_of(ORC_ALTAR), w3_client::CardKind::Producer);
    assert_eq!(catalog.kind_of(ORC_SMITHY), w3_client::CardKind::Producer);
    assert_eq!(catalog.kind_of(ORC_HERO), w3_client::CardKind::Hero);
    assert_eq!(
        catalog.kind_of(ORC_TOWN_HALL),
        w3_client::CardKind::Producer
    );
    assert_eq!(catalog.kind_of(ORC_BARRACKS), w3_client::CardKind::Producer);
    // 人族六类登记保持；中立资源登记保持；未登记 DefId 通用回落保持。
    assert_eq!(catalog.kind_of(HUMAN_FARM), w3_client::CardKind::Producer);
    assert_eq!(catalog.kind_of(HUMAN_HERO), w3_client::CardKind::Hero);
    assert_eq!(catalog.kind_of(HUMAN_PEASANT), w3_client::CardKind::Worker);
    assert_eq!(
        catalog.kind_of(NEUTRAL_GOLD_MINE),
        w3_client::CardKind::Resource
    );
    assert_eq!(catalog.kind_of(NEUTRAL_TREE), w3_client::CardKind::Resource);
    assert_eq!(catalog.kind_of(999_999), w3_client::CardKind::Combat);
    assert_eq!(catalog.primary_card(&[]), CardId::Main);
}

/// 未知族 / 缺表项明确默认回落且可诊断（C8 / C9）：`for_optional_race(None)`
/// 回落到人族段；缺表项表回落到 `DEFAULT_COMMAND_TARGETS`，不 panic、
/// 不静默产出错误族目标。
#[test]
fn q2_unknown_race_and_missing_table_entry_fall_back_explicitly() {
    // 未知族：回落到人族段（与 standard 逐字节一致）。
    let unknown = CommandCatalog::for_optional_race(None);
    assert_eq!(
        unknown,
        CommandCatalog::standard(),
        "reason=unknown_race_falls_back_human"
    );
    assert!(
        trained_targets(&unknown)
            .iter()
            .all(|def_id| (100..200).contains(def_id))
    );

    // 缺表项：把兽族行某几项置 None，经真实 for_race_with_targets 构造
    // 路径产出目录，缺项回落到人族段默认（不 panic、可诊断）。
    let mut table: RaceTargetTable = RACE_TRAIN_TARGETS;
    table[1][TrainCategory::Soldier.index()] = None;
    table[1][TrainCategory::TownHall.index()] = None;
    let partial = CommandCatalog::for_race_with_targets(Race::Orc, table);
    let producer = card_train_targets(&partial, CardId::Producer);
    assert!(
        producer.contains(&HUMAN_SOLDIER),
        "reason=missing_soldier_falls_back_to_default"
    );
    let build = card_train_targets(&partial, CardId::Build);
    assert!(
        build.contains(&HUMAN_TOWN_HALL),
        "reason=missing_town_hall_falls_back_to_default"
    );
    // 未缺项仍然是兽族段目标（回落只作用于缺项，不污染整表）。
    assert!(build.contains(&ORC_ALTAR), "reason=other_entries_still_orc");
}

/// 命令面回归（Q2 / R13.3）：兽族 Producer 卡训练格经真实
/// `InputState::apply` 产出的 `Train { unit_def, .. }` 落 `2xx` 且
/// MUST NOT 是 `HUMAN_*`；人族装配仍产 `1xx`（逐字节行为不变）。
/// `Train` 命令只经既有 `GameplayCommandKind` 变体、不改排序键。
#[test]
fn q2_producer_train_card_emits_race_segment_train_command() {
    let view = WorldView {
        tick: 4,
        rng: 0,
        entities: vec![EntityView {
            id: 1,
            x: 0,
            y: 0,
            facing: 0,
            speed: 0,
            hp: 300,
            owner: 1,
            alive: true,
            has_target: false,
        }],
    };
    let mapper = ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let movability = MovabilityMap::new();

    // 从 Producer 卡取出「训练格」的经典键字母（键位与命令解耦）。
    let catalog = CommandCatalog::for_race(Race::Orc);
    let train_button = catalog
        .card(CardId::Producer)
        .filled()
        .into_iter()
        .find(|button| {
            matches!(
                button.command,
                CardCommand::TrainUnit {
                    is_building: false,
                    ..
                }
            )
        })
        .expect("reason=producer_train_button_exists");

    // P9 / Q1'：四族 Producer 卡训练格各产本族段 `Train`（3xx / 4xx）。
    for (race, segment) in [
        (Race::Orc, 200..300),
        (Race::Human, 100..200),
        (Race::Undead, 300..400),
        (Race::NightElf, 400..500),
    ] {
        let mut input = InputState::new(1)
            .with_entity_defs([(1u64, race.town_hall())].into_iter().collect())
            .with_catalog(CommandCatalog::for_race(race));
        input
            .apply(
                InputAction::SelectStart([0.0, 0.0]),
                &view,
                &mapper,
                &[],
                &movability,
            )
            .unwrap();
        input
            .apply(
                InputAction::SelectEnd([1280.0, 720.0]),
                &view,
                &mapper,
                &[],
                &movability,
            )
            .unwrap();
        assert!(!input.selection().is_empty(), "reason=producer_selected");
        let commands = input
            .apply(
                InputAction::Hotkey(train_button.hotkey),
                &view,
                &mapper,
                &[],
                &movability,
            )
            .unwrap();
        assert_eq!(commands.len(), 1, "reason=one_train_command race={race:?}");
        match &commands[0] {
            ClientCommand::Gameplay(GameplayCommand {
                kind: GameplayCommandKind::Train { unit_def, .. },
                ..
            }) => {
                assert!(
                    segment.contains(unit_def),
                    "reason=train_command_in_race_segment race={race:?} def={unit_def}"
                );
                assert!(
                    !human_target_ids().contains(unit_def) || race == Race::Human,
                    "reason=orc_train_must_not_be_human race={race:?} def={unit_def}"
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}

/// HUD 命令卡按本地族只读派生：选中生产者时人族 / 兽族 `HudModel`
/// 的 12 格卡目标分别落 `1xx` / `2xx`，空选中不显示命令卡。
#[test]
fn q2_hud_command_card_derives_targets_by_local_race() {
    let (view, snap, mapper, defs, selection) = hud_fixture();
    let human = HudModel::collect(&view, &snap, &selection, &[], &mapper, 1)
        .with_entity_defs(defs.clone())
        .with_active_card(CardId::Producer)
        .with_faction(FactionSelection::with_races(Race::Human, Race::Orc));
    let orc = HudModel::collect(&view, &snap, &selection, &[], &mapper, 1)
        .with_entity_defs(defs)
        .with_active_card(CardId::Producer)
        .with_faction(FactionSelection::with_races(Race::Orc, Race::Human));
    let human_card = human.command_card.expect("reason=human_card");
    let orc_card = orc.command_card.expect("reason=orc_card");
    assert_eq!(human_card.buttons.len(), 12);
    assert_eq!(orc_card.buttons.len(), 12);
    let human_targets = collect_card_targets(&human_card);
    let orc_targets = collect_card_targets(&orc_card);
    assert!(!human_targets.is_empty());
    assert!(human_targets.iter().all(|d| (100..200).contains(d)));
    assert!(orc_targets.iter().all(|d| (200..300).contains(d)));
    assert_ne!(human_card, orc_card, "reason=card_changes_with_local_race");
}

// ---------------------------------------------------------------------------
// 反向验证（改坏验红，CODEBUDDY.md §5 / tasks 6.1 / design D6）
// ---------------------------------------------------------------------------

/// 被反向验证的 Q1 断言：本地席位 `def_ids` 全部落在选族 `local_race` 段。
fn local_seat_in_selected_segment(setup: &MeleeSetup, selection: FactionSelection) -> bool {
    let segment = race_segment(selection.local_race);
    [1u64, 2, 3, 4].iter().all(|id| {
        setup
            .def_ids()
            .get(id)
            .is_some_and(|def_id| segment.contains(def_id))
    })
}

/// 退化装配（缺陷态，仅反向验证用）：忽略 `(local_race, ai_race)`、
/// 硬编码回落 `MeleeSetup::new()`（本地人族 / AI 兽族）。仍经真实
/// [`assemble_from_selection`] 生产路径注入，非旁路模拟（先例
/// `p8-dual-faction::degenerate_opponent_assembly`）。
fn degenerate_default_assembly(selection: FactionSelection) -> MeleeSetup {
    assemble_from_selection(selection, |_, _| MeleeSetup::new())
}

/// 反向验证·Q1 修复态：正式实现四组合全通过「本地席位落所选族段」断言。
#[test]
fn reverse_validation_q1_formal_passes_selected_segment_assertion() {
    for (local, ai) in COMBINATIONS {
        let selection = FactionSelection::with_races(local, ai);
        assert!(
            local_seat_in_selected_segment(&assembled(selection), selection),
            "reason=formal_q1_passes local={local:?} ai={ai:?}"
        );
    }
}

/// 反向验证·Q1 缺陷态：选族传参退化回硬编码 `MeleeSetup::new()` 时，
/// 「本地席位落所选族段」断言在「本地兽族」组合必失败；正式实现仍通过
/// （元断言防退化，两条证据成对）。
#[test]
fn reverse_validation_q1_degenerate_fails_the_same_assertion() {
    // 本地兽族：退化实现产出人族段，段位断言失败。
    let selection = FactionSelection::with_races(Race::Orc, Race::Human);
    let degenerate = degenerate_default_assembly(selection);
    assert!(
        !local_seat_in_selected_segment(&degenerate, selection),
        "reason=degenerate_q1_must_fail local=Orc"
    );
    // 元断言：退化实现确实等于硬编码 new()，且正式实现仍通过。
    assert_eq!(degenerate.def_ids, MeleeSetup::new().def_ids);
    assert!(local_seat_in_selected_segment(
        &assembled(selection),
        selection
    ));
    // 本地人族组合恰好不暴露缺陷，证明判据依赖「所选族」而非恒真。
    let human = FactionSelection::with_races(Race::Human, Race::Orc);
    assert!(local_seat_in_selected_segment(
        &degenerate_default_assembly(human),
        human
    ));
}

/// 被反向验证的 Q2 断言：目录的全部训练 / 建造目标都落兽族 `2xx` 段、
/// 且不含任何 `HUMAN_*` 目标。
fn catalog_trains_only_orc_targets(catalog: &CommandCatalog) -> bool {
    let targets = trained_targets(catalog);
    !targets.is_empty()
        && targets.iter().all(|def_id| (200..300).contains(def_id))
        && targets
            .iter()
            .all(|def_id| !human_target_ids().contains(def_id))
}

/// 退化定义表（缺陷态，仅反向验证用）：四行都写死 `HUMAN_*` 目标
/// （即 P8.4 之前 `command_card.rs:235-360` 的旧实现，模拟散落
/// `if race` 白名单 / 新族行静默回落的退化实现），仍经真实
/// [`CommandCatalog::for_race_with_targets`] 生产路径注入、非旁路模拟。
fn degenerate_human_only_table() -> RaceTargetTable {
    [RACE_TRAIN_TARGETS[0]; RACE_ROWS]
}

/// 反向验证·Q2 修复态：正式按族查表目录通过「兽族训练产兽族目标」断言。
#[test]
fn reverse_validation_q2_formal_passes_orc_target_assertion() {
    assert!(
        catalog_trains_only_orc_targets(&CommandCatalog::for_race(Race::Orc)),
        "reason=formal_q2_passes"
    );
}

/// 反向验证·Q2 缺陷态：命令卡退化回写死 `HUMAN_*` 时，「兽族训练产兽族
/// 目标」断言必失败；正式实现仍通过（元断言防退化，两条证据成对）。
#[test]
fn reverse_validation_q2_degenerate_fails_the_same_assertion() {
    let degenerate =
        CommandCatalog::for_race_with_targets(Race::Orc, degenerate_human_only_table());
    assert!(
        !catalog_trains_only_orc_targets(&degenerate),
        "reason=degenerate_q2_must_fail"
    );
    // 缺陷态确实产出人族目标（旧实现），正式实现产出兽族目标。
    assert!(
        trained_targets(&degenerate)
            .iter()
            .all(|def_id| (100..200).contains(def_id))
    );
    assert!(catalog_trains_only_orc_targets(&CommandCatalog::for_race(
        Race::Orc
    )));
    // 人族正式目录逐字节不变，证明退化只破坏按族分派。
    assert_eq!(
        CommandCatalog::for_race(Race::Human),
        CommandCatalog::standard()
    );
}

// ---------------------------------------------------------------------------
// 只读夹具（合成数据 / 真实 RtsMatch / 真实 ClientApp）
// ---------------------------------------------------------------------------

/// 装配层选族结果 → 真实权威对局（只经两个公开提交入口）。
fn build_sim(setup: &MeleeSetup, defs: &w3_core::FrozenDefinitions) -> RtsMatch {
    let config = MatchConfig {
        seed: SEED,
        logic_fps: LOGIC_FPS,
        content_hash: CONTENT_HASH,
    };
    let mut sim = RtsMatch::create_with_definitions(config, synthetic_world(), defs.clone());
    for command in &setup.commands {
        sim.submit_command(*command)
            .expect("p8_hud: reason=spawn_command_rejected");
    }
    for command in &setup.gameplay_commands {
        sim.submit_gameplay_command(*command)
            .expect("p8_hud: reason=opening_gameplay_rejected");
    }
    sim
}

/// 合成地形（纯表现层几何，不进权威状态 / 哈希 / 快照）。
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

fn spawn(sequence: u32, x: i64) -> Command {
    Command::new(
        Frame::new(0),
        1,
        sequence,
        CommandKind::Spawn {
            x: SimFixed::from_int(x).unwrap(),
            y: SimFixed::from_int(0).unwrap(),
            hp: 100,
            speed: SimFixed::from_int(3).unwrap(),
            attack_damage: 0,
            attack_range: SimFixed::from_int(1).unwrap(),
        },
    )
}

fn build_app() -> ClientApp<FixedStepClient> {
    let mut driver = FixedStepClient::new(SEED, CONTENT_HASH, StaticWorld::default());
    driver.submit_command(spawn(1, 0)).unwrap();
    driver.submit_command(spawn(2, 4)).unwrap();
    let mut app = ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0);
    app.advance_with_ticks(TICKS_PER_LOGIC_FRAME);
    app
}

fn hash(app: &ClientApp<FixedStepClient>) -> u64 {
    app.driver().sim().compute_state_hash()
}

fn snapshot(app: &ClientApp<FixedStepClient>) -> w3_core::Snapshot {
    app.driver().sim().capture_snapshot()
}

/// 供 HUD 命令卡派生用的最小只读夹具（一个生产者实体被选中）。
fn hud_fixture() -> (
    w3_core::WorldView,
    w3_core::Snapshot,
    w3_client::ScreenMapper,
    BTreeMap<u64, DefId>,
    BTreeSet<u64>,
) {
    let defs = synthetic_definitions();
    let setup = MeleeSetup::new();
    let mut sim = build_sim(&setup, &defs);
    sim.step();
    let view = sim.read_view();
    let snap = sim.capture_snapshot();
    let mapper = w3_client::ScreenMapper::new(1280.0, 720.0, 256.0, [0.0, 0.0], 1.0);
    let selection: BTreeSet<u64> = [1u64].into_iter().collect();
    let entity_defs: BTreeMap<u64, DefId> = [(1u64, HUMAN_TOWN_HALL)].into_iter().collect();
    (view, snap, mapper, entity_defs, selection)
}

fn collect_card_targets(card: &w3_client::CommandCard) -> Vec<DefId> {
    card.buttons
        .iter()
        .flatten()
        .filter_map(|button| match button.command {
            CardCommand::TrainUnit { unit_def, .. } => Some(unit_def),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Q3 运行时动态实体真实两族 glTF 模型键（tasks 5.1 / 5.3 / 5.4 / 6.1）
// ---------------------------------------------------------------------------

/// 一个运行时 `Train` / `Build` 新实体的只读视图（不在装配期 12 个 frame-0
/// `EntityId → key` 绑定内，`EntityView` 无 `def_id`，Q3 按 owner + speed 派生）。
fn runtime_view(id: u64, owner: u8, speed: i64) -> WorldView {
    WorldView {
        tick: 60,
        rng: 0,
        entities: vec![EntityView {
            id,
            x: 0,
            y: 0,
            facing: 0,
            speed,
            hp: 100,
            owner,
            alive: true,
            has_target: false,
        }],
    }
}

/// 带 5 个三角形的合成 `BakedModel`（真实 glTF 网格的 CPU 替身）。
/// `expand_model` 每个三角形展开 3 个顶点，故该模型共 15 顶点；
/// 程序化四面体 `append_unit_marker` 固定 12 顶点，二者可 CPU 区分。
fn real_mesh() -> BakedModel {
    let triangles = (0..5)
        .map(|index| {
            let z = index as f32;
            BakedTriangle::colored(
                [[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]],
                [[0.0, 0.0, 1.0]; 3],
                [[0.85, 0.25, 0.15, 1.0]; 3],
            )
        })
        .collect();
    BakedModel { triangles }
}

/// 按选族装配 14 键占位 + 两席位段绑定的 `ModelLibrary`（Q3 生产路径）。
fn faction_library(faction: FactionSelection) -> ModelLibrary {
    let mut library = ModelLibrary::default();
    library.insert("default", Model::default());
    RuntimeModelPlan::from_faction(faction).bind_seats(&mut library);
    library
}

/// 把 14 个真实键各绑定一个合成真实网格（模拟本机 glTF 全部就绪）。
fn all_real_meshes() -> BTreeMap<String, BakedModel> {
    ENTITY_MODEL_KEYS
        .iter()
        .map(|key| ((*key).to_string(), real_mesh()))
        .collect()
}

/// 统计 `Scene::build_vertices` 中动态实体贡献的顶点数（扣除地形 / 静态摆放）。
fn dynamic_vertex_count(scene: &Scene) -> usize {
    let terrain = synthetic_terrain();
    let total = scene.build_vertices(&terrain).len();
    total - terrain.indices.len()
}

/// 兽族本地席位（`owner=1`）运行时新实体按 `2xx` 段解析真实 glTF 网格，
/// 人族 AI 席位（`owner=2`）按 `1xx` 段；两者模型键不同、顶点流均为真实
/// 网格（15 顶点）而非四面体（12 顶点），可区分（R2 / R4 / C9 / Q3 拍板）。
#[test]
fn q3_orc_and_human_runtime_entities_render_distinct_real_meshes() {
    // 本地兽族 / AI 人族（Q1 选族装配）；owner 1=本地、2=AI。
    let faction = FactionSelection::with_races(Race::Orc, Race::Human);
    let library = faction_library(faction);
    let mut scene = Scene::default();
    scene.set_entity_models(all_real_meshes());

    // 两个运行时新实体（单位 speed>0）：本地兽族 id=90 / AI 人族 id=91。
    let view = WorldView {
        tick: 60,
        rng: 0,
        entities: vec![
            EntityView {
                id: 90,
                x: 0,
                y: 0,
                facing: 0,
                speed: 3 * 65536,
                hp: 100,
                owner: LOCAL_PLAYER,
                alive: true,
                has_target: false,
            },
            EntityView {
                id: 91,
                x: 4,
                y: 0,
                facing: 0,
                speed: 3 * 65536,
                hp: 100,
                owner: AI_PLAYER,
                alive: true,
                has_target: false,
            },
        ],
    };
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &library,
    );

    let keys: BTreeMap<u64, String> = scene
        .instances()
        .map(|instance| {
            (
                instance.id,
                instance
                    .model_key
                    .clone()
                    .expect("reason=segment_key_resolved"),
            )
        })
        .collect();
    assert_eq!(
        keys.get(&90).map(String::as_str),
        Some("orc-grunt"),
        "本地兽族落 2xx 段真实键"
    );
    assert_eq!(
        keys.get(&91).map(String::as_str),
        Some("soldier"),
        "AI 人族落 1xx 段真实键"
    );
    assert_ne!(
        keys.get(&90),
        keys.get(&91),
        "两族运行时实体模型键 MUST 可区分"
    );
    assert!(
        !keys.values().any(|key| key == "default"),
        "MUST NOT 回落默认四面体"
    );
    // 两个实例各 15 顶点（真实网格），合计 30，而非两个四面体 24。
    assert_eq!(
        dynamic_vertex_count(&scene),
        2 * real_mesh().triangle_count() * 3,
        "真实网格而非四面体"
    );
    assert!(
        scene.fallback_diagnostics().is_empty(),
        "真实网格就位时无回退诊断"
    );
}

/// 运行时建筑（`speed == 0`）按段解析建筑真实键（`orc-great-hall` /
/// `town-hall`），与单位键不同；R1.4 分层判据同源、不生成 `Move`。
#[test]
fn q3_runtime_buildings_resolve_segment_building_keys() {
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    let library = faction_library(faction);
    let mut scene = Scene::default();
    scene.set_entity_models(all_real_meshes());
    // speed==0 建筑（本地兽族席位）。
    let view = runtime_view(92, LOCAL_PLAYER, 0);
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &library,
    );
    let key = scene
        .instances()
        .next()
        .and_then(|instance| instance.model_key.as_deref());
    assert_eq!(key, Some("orc-great-hall"), "兽族建筑落 2xx 段建筑真实键");
    assert_eq!(
        dynamic_vertex_count(&scene),
        real_mesh().triangle_count() * 3
    );
}

/// DefId 段 → 真实模型键的编译期表（Q3 / R2 / C16–C18 / P9 Q1'）：
/// 五段覆盖 `1xx` / `2xx` / `3xx` / `4xx` / `9xx`；18 键与
/// `ENTITY_MODEL_FILES` 逐字一致（既有 14 个真实网格键 + P9 由
/// content-owner 落盘的 4 个合成段代表键）；兽族 8 键与新增 4 键均为
/// 小写归一逻辑路径（`normalize()` 双侧归一，C18 不是待修缺陷）；键名
/// MUST 全部来自 content-owner 接口，不改解析逻辑 / 静态摆放链路
/// （ADR 0004）。新段沿用「段代表键 + 显式可诊断回退」，不要求给每个
/// 新 `DefId` 加精确映射（沿用 R-1 口径，避免扩范围）。
#[test]
fn q3_def_id_segment_table_matches_entity_model_files() {
    assert_eq!(SEGMENT_MODEL_KEYS.len(), 5);
    assert_eq!(def_segment(101), SEGMENT_HUMAN);
    assert_eq!(def_segment(210), SEGMENT_ORC);
    assert_eq!(def_segment(301), SEGMENT_UNDEAD);
    assert_eq!(def_segment(401), SEGMENT_NIGHT_ELF);
    assert_eq!(def_segment(901), SEGMENT_NEUTRAL);
    // 18 键逐字等于 `w3-content::ENTITY_MODEL_FILES` 的键（两个 owner
    // 并行改不同文件，必须一致；既有 14 键前缀逐字不变）。
    let content_keys: Vec<&str> = w3_content::ENTITY_MODEL_FILES
        .iter()
        .map(|(key, _)| *key)
        .collect();
    assert_eq!(
        ENTITY_MODEL_KEYS.len(),
        18,
        "6 人族/中立 + 8 兽族 + 4 P9 段代表键"
    );
    assert_eq!(ENTITY_MODEL_KEYS.as_slice(), content_keys.as_slice());
    assert_eq!(
        &content_keys[..14],
        &[
            "peasant",
            "soldier",
            "town-hall",
            "barracks",
            "gold-mine",
            "tree",
            "orc-peon",
            "orc-grunt",
            "orc-headhunter",
            "orc-great-hall",
            "orc-barracks",
            "orc-burrow",
            "orc-altar",
            "orc-war-mill"
        ]
    );
    // 兽族 8 键（索引 6..14）的 value 全为小写归一逻辑路径（C18）。
    for (key, path) in w3_content::ENTITY_MODEL_FILES.iter().skip(6).take(8) {
        assert!(key.starts_with("orc-"), "reason=orc_key_prefix key={key}");
        assert_eq!(
            *path,
            path.to_ascii_lowercase(),
            "reason=path_not_normalized key={key}"
        );
    }
    // P9 新增 4 键（索引 14..18）：合成全小写逻辑路径占位串，
    // 非资产内容、不指向真实导出资产，沿用大小写归一机制。
    for (key, path) in w3_content::ENTITY_MODEL_FILES.iter().skip(14) {
        assert!(
            [
                "undead-unit",
                "undead-building",
                "nightelf-unit",
                "nightelf-building",
            ]
            .contains(key),
            "reason=p9_segment_key_name key={key}"
        );
        assert_eq!(
            *path,
            path.to_ascii_lowercase(),
            "reason=p9_path_not_normalized key={key}"
        );
    }
    // 段键表五段所有键都在 18 键内（含新段代表键，不引入表外键）。
    for (_, keys) in SEGMENT_MODEL_KEYS {
        assert!(ENTITY_MODEL_KEYS.contains(&keys.unit));
        assert!(ENTITY_MODEL_KEYS.contains(&keys.building));
    }
}

/// R-1 已知限制（用户批准、不得擅自补齐）：`ORC_SIEGE`(206) /
/// `ORC_HERO`(210) 及人族六类精确 DefId 查表返回 `None`，
/// 走「缺键显式可诊断回退」，不映射真实网格。
#[test]
fn q3_r1_eight_defs_have_no_precise_real_mesh() {
    assert_eq!(R1_UNMAPPED_DEFS.len(), 8);
    for def_id in R1_UNMAPPED_DEFS {
        assert_eq!(def_id_to_model_key(def_id, false), None);
        assert_eq!(def_id_to_model_key(def_id, true), None);
    }
    // 而基础单位 / 建筑精确 DefId 按段命中真实键（逐字节不变）。
    assert_eq!(def_id_to_model_key(HUMAN_SOLDIER, false), Some("soldier"));
    assert_eq!(
        def_id_to_model_key(ORC_TOWN_HALL, true),
        Some("orc-great-hall")
    );
}

/// 本机资产缺失（干净环境 / CI，无 `config.toml`）时显式、可诊断回退：
/// 段键解析成功但 `entity_models` 为空 → `reason=asset_missing` 记录 +
/// 明确默认几何（四面体，12 顶点，可辨），MUST NOT 静默、
/// MUST NOT 只在「无本机资产」时才可诊断（注入无键 DefId 同理）。
#[test]
fn q3_missing_asset_is_explicitly_diagnosable_with_clear_fallback() {
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    let library = faction_library(faction);
    let mut scene = Scene::default(); // 不注入任何 entity_models = 干净环境。
    let view = runtime_view(93, LOCAL_PLAYER, 3 * 65536);
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &library,
    );
    assert_eq!(
        scene.fallback_diagnostics().get(&93),
        Some(&REASON_ASSET_MISSING),
        "reason=asset_missing_must_be_recorded"
    );
    // 回退几何仍可辨（四面体 12 顶点），不是空 / 黑屏。
    assert_eq!(dynamic_vertex_count(&scene), 12, "明确默认几何可辨");
    // 模型键仍是段真实键（不是 "default"），只缺真实网格、不缺键。
    let key = scene
        .instances()
        .next()
        .and_then(|instance| instance.model_key.as_deref());
    assert_eq!(key, Some("orc-grunt"));
}

/// 模型键 / 几何只读，不进权威状态 / FNV-1a 状态哈希 / 快照：
/// 真实 `RtsMatch` 在「跑完整 Q3 呈现管线」与「不跑」两侧逐帧一致，
/// `SNAPSHOT_VERSION == 10`（ADR 0002 / C1 / tasks 5.4）。
#[test]
fn q3_model_key_resolution_never_changes_hash_or_snapshot() {
    assert_eq!(SNAPSHOT_VERSION, 10);
    let defs = synthetic_definitions();
    let setup = MeleeSetup::new();
    let mut rendered = build_sim(&setup, &defs);
    let mut plain = build_sim(&setup, &defs);

    let faction = FactionSelection::default();
    let library = faction_library(faction);
    let mut scene = Scene::default();
    scene.set_entity_models(all_real_meshes());
    let terrain = synthetic_terrain();

    for frame in 0..8 {
        rendered.step();
        plain.step();
        let view = rendered.read_view();
        let events = rendered.drain_events();
        let _ = plain.drain_events();
        // 只有 rendered 一侧跑 Q3 呈现管线（consume + 插值 + 生成顶点）。
        scene.consume(
            FrameInput {
                view: &view,
                events: &events,
            },
            &library,
        );
        scene.interpolate_all(0.5);
        let _ = scene.build_vertices(&terrain);
        assert_eq!(
            rendered.compute_state_hash(),
            plain.compute_state_hash(),
            "reason=q3_presentation_changed_hash frame={frame}"
        );
        let snap_a = rendered.capture_snapshot();
        let snap_b = plain.capture_snapshot();
        assert_eq!(
            snap_a, snap_b,
            "reason=q3_presentation_changed_snapshot frame={frame}"
        );
        assert_eq!(snap_a.version, SNAPSHOT_VERSION);
        assert_eq!(snap_a.defs_hash, defs.content_hash(), "reason=defs_hash");
    }
}

// ---------------------------------------------------------------------------
// 反向验证·改坏验红 ③④（CODEBUDDY.md §5 / tasks 6.1 / design D6）
// ---------------------------------------------------------------------------

/// 被反向验证的 Q3 断言：兽族运行时新实体按 `2xx` 段解析**真实 glTF 网格**
/// （模型键为段真实键 `orc-grunt`，且顶点流 = 真实网格 15 顶点）。
fn orc_runtime_entity_renders_real_mesh(library: &ModelLibrary) -> bool {
    let mut scene = Scene::default();
    scene.set_entity_models(all_real_meshes());
    let view = runtime_view(94, LOCAL_PLAYER, 3 * 65536);
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        library,
    );
    let real_key = scene
        .instances()
        .next()
        .and_then(|instance| instance.model_key.as_deref())
        == Some("orc-grunt");
    let real_geometry = dynamic_vertex_count(&scene) == real_mesh().triangle_count() * 3;
    real_key && real_geometry
}

/// 退化装配（缺陷态，仅反向验证用）：忽略选族、不注入段绑定，
/// 一律回落 `"default"` 四面体（Q3 之前的旧实现）。仍经真实
/// [`ModelLibrary::resolve_with_reason`] + [`Scene::consume`] 生产路径
/// 注入，非旁路模拟。
fn degenerate_default_library() -> ModelLibrary {
    let mut library = ModelLibrary::default();
    library.insert("default", Model::default());
    library
}

/// 反向验证·③ 修复态：正式段绑定装配通过「兽族新实体按段解析真实网格」断言。
#[test]
fn reverse_validation_q3_formal_passes_real_mesh_assertion() {
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    assert!(
        orc_runtime_entity_renders_real_mesh(&faction_library(faction)),
        "reason=formal_q3_passes"
    );
}

/// 反向验证·③ 缺陷态：退化回一律 `"default"` 四面体 → 同一断言必失败；
/// 正式实现仍通过（元断言防退化，两条证据成对）。
#[test]
fn reverse_validation_q3_degenerate_fails_the_same_assertion() {
    let degenerate = degenerate_default_library();
    assert!(
        !orc_runtime_entity_renders_real_mesh(&degenerate),
        "reason=degenerate_q3_must_fail"
    );
    // 元断言：退化态确实产 "default" 四面体，正式态确实产真实网格。
    let mut scene = Scene::default();
    scene.set_entity_models(all_real_meshes());
    let view = runtime_view(94, LOCAL_PLAYER, 3 * 65536);
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        &degenerate,
    );
    assert_eq!(
        scene
            .instances()
            .next()
            .and_then(|i| i.model_key.as_deref()),
        Some("default")
    );
    assert_eq!(dynamic_vertex_count(&scene), 12, "四面体占位");
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    assert!(orc_runtime_entity_renders_real_mesh(&faction_library(
        faction
    )));
}

/// 被反向验证的 ④ 断言：干净环境缺资产时「显式可诊断回退」=
/// `Scene::fallback_diagnostics` 记录 `reason=asset_missing`。
fn missing_asset_is_explicitly_diagnosable(scene: &mut Scene, library: &ModelLibrary) -> bool {
    let view = runtime_view(95, LOCAL_PLAYER, 3 * 65536);
    scene.consume(
        FrameInput {
            view: &view,
            events: &[],
        },
        library,
    );
    scene.fallback_diagnostics().get(&95) == Some(&REASON_ASSET_MISSING)
}

/// 反向验证·④ 修复态：正式 `Scene`（显式诊断）通过「缺资产显式可诊断」断言。
#[test]
fn reverse_validation_q4_formal_passes_diagnosable_assertion() {
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    let mut scene = Scene::default();
    assert!(missing_asset_is_explicitly_diagnosable(
        &mut scene,
        &faction_library(faction)
    ));
}

/// 反向验证·④ 缺陷态：`Scene::with_silent_fallback(true)` 静默吞掉日志与
/// 诊断记录 → 同一断言必失败；正式实现仍通过（元断言防退化）。
/// 退化真经 [`Scene::consume`] 生产路径注入、非旁路模拟。
#[test]
fn reverse_validation_q4_degenerate_fails_the_same_assertion() {
    let faction = FactionSelection::with_races(Race::Orc, Race::Orc);
    let library = faction_library(faction);
    let mut degenerate = Scene::default().with_silent_fallback(true);
    assert!(
        !missing_asset_is_explicitly_diagnosable(&mut degenerate, &library),
        "reason=degenerate_q4_must_fail"
    );
    // 元断言：退化态静默（无诊断），正式态显式（有诊断）。
    assert!(degenerate.fallback_diagnostics().is_empty());
    let mut formal = Scene::default();
    assert!(missing_asset_is_explicitly_diagnosable(
        &mut formal,
        &library
    ));
}
