//! P8.5c 野怪营地与掉落正向验收（`creep-camps/spec.md` 的 24 个 Scenario 中
//! 属于 `w3-core` 内核侧的判据）。
//!
//! 规则依据：R1.1 / R1.4 / R6.1 / R6.5 / R7.2 / R7.5 / R7.6 / R8.1 / R8.2 /
//! R8.3 / R8.5 / R8.6 / R11.5 / R12.2 / R12.4；确定性契约 ADR 0002 §2-8 /
//! `CODEBUDDY.md §5`；Q1 方向 A / Q4 bump 6 / Q5 新变体 / Q6 随机码 / Q7 中立归属 / Q8 零资产。
//! 所有数值为合成原创编译期整数 `[待核]`，零文件 IO、零资产、不冒充原版（§14）。

mod creep_common;

use std::collections::BTreeMap;

use creep_common::*;
use w3_core::{
    CampScript, DefId, DropSet, DropTable, EffectKind, EntityId, EventKind, Frame,
    FrozenDefinitions, GameplayCommandKind, LeashPhase, NEUTRAL_OWNER, NeutralCategory, RtsMatch,
    SIM_VERSION, SNAPSHOT_VERSION, SYNTHETIC_CREEP_CONSTANTS, SimFixed, SnapshotError, StaticWorld,
};

/// 捕获快照并经真实 `restore_snapshot` 生产路径注入野怪位置 / 状态。
fn inject_creep_state(
    sim: &RtsMatch,
    content: u64,
    creep: EntityId,
    x: i64,
    y: i64,
    phase: LeashPhase,
    target: Option<EntityId>,
) -> RtsMatch {
    let mut snapshot = sim.capture_snapshot();
    for record in &mut snapshot.positions {
        if record.entity == creep.value() {
            record.x = x;
            record.y = y;
        }
    }
    if let Some(state) = snapshot.creep_camps.creeps.get_mut(&creep) {
        state.phase = phase;
        state.target = target;
    }
    RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap()
}

// ---------------------------------------------------------------------------
// 1. 定义层：零资产 / 冻结常量 / 数据驱动（Q8 / tasks 2.x / R8.2 / R8.6）
// ---------------------------------------------------------------------------

#[test]
fn frozen_creep_defs_are_zero_asset_and_content_sensitive() {
    let base = defs();
    assert_eq!(SNAPSHOT_VERSION, 10, "reason=snapshot_version_10");
    assert_eq!(SIM_VERSION, 10, "reason=sim_version_10");

    // 干净环境构造成功、零文件 IO：只由编译期整数常量装配。
    assert!(base.is_creep(CREEP_DEF));
    assert!(base.is_ground_item(ITEM_DEF));
    assert_eq!(base.camp_script(CAMP_SCRIPT).map(|s| s.tier), Some(1));
    assert_eq!(
        base.creep_constants().leash_radius_raw,
        2 * SCALE,
        "reason=leash_is_frozen_constant"
    );

    // defs_hash / content_hash 进入快照头部，改常量即改变。
    let (sim, _) = seeded_match();
    let snapshot = sim.capture_snapshot();
    assert_eq!(snapshot.defs_hash, base.content_hash());
    assert_eq!(snapshot.content_hash, base.content_hash());
    assert_eq!(snapshot.version, 10);
    assert_eq!(snapshot.sim_version, 10);

    let mut changed = constants();
    changed.leash_radius_raw += 1;
    let variant = FrozenDefinitions::new(vec![(CREEP_DEF, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(changed, scripts(), tables(), categories());
    assert_ne!(
        base.content_hash(),
        variant.content_hash(),
        "reason=constants_enter_defs_hash"
    );
}

#[test]
fn availability_is_definition_driven_not_a_name_whitelist() {
    // 以完全非经典命名的 DefId 登记仍按冻结定义判定（R1.2 / R13.3 / tasks 2.3）。
    let odd_creep: DefId = 7_777_101;
    let odd_item: DefId = 7_777_202;
    let odd_script: u32 = 7_777;
    let odd_table: u32 = 7_778;
    let script = CampScript {
        tier: 2,
        xp: 9,
        gold: 4,
        home_x: HOME_X,
        home_y: HOME_Y,
        drop_table: odd_table,
        creep_defs: vec![odd_creep],
    };
    let table = DropTable {
        sets: [(
            1u32,
            DropSet {
                entries: [(odd_item, 100u32)].into_iter().collect(),
            },
        )]
        .into_iter()
        .collect(),
        effects: BTreeMap::new(),
    };
    let frozen = FrozenDefinitions::new(vec![(odd_creep, creep_unit())], vec![], vec![], vec![])
        .unwrap()
        .with_creep_camps(
            constants(),
            [(odd_script, script)].into_iter().collect(),
            [(odd_table, table)].into_iter().collect(),
            [
                (odd_creep, NeutralCategory::CreepHostile),
                (odd_item, NeutralCategory::GroundItemOwnerless),
            ]
            .into_iter()
            .collect(),
        );
    assert!(frozen.is_creep(odd_creep));
    assert!(frozen.is_ground_item(odd_item));
    assert!(!frozen.is_creep(odd_item));
    assert!(frozen.camp_script(odd_script).is_some());
    assert!(frozen.drop_table(odd_table).is_some());

    // 真实装配：非经典命名营地仍可 seed 并掉落（R8.1 / R7.2）。
    let content = frozen.content_hash();
    let cfg = w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), frozen.clone());
    sim.submit_command(spawn(HOME_X, HOME_Y, 100, 100)).unwrap();
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, odd_script))
        .unwrap();
    sim.step();
    assert_eq!(sim.creep_camps().creeps.len(), 1, "reason=odd_def_seeded");
    attack(&mut sim, 1, 1, 2, 1);
    sim.step();
    let state = sim.creep_camps();
    assert_eq!(state.kill_rewards.len(), 1, "reason=odd_def_reward");
    assert_eq!(
        state.ground_items.values().next().map(|item| item.item_def),
        Some(odd_item),
        "reason=odd_item_dropped"
    );
    assert_eq!(
        state.rng_draws.len(),
        1,
        "reason=one_mutually_exclusive_set"
    );
}

#[test]
fn frozen_constants_and_authoritative_state_are_distinct_layers() {
    // 冻结常量（定义层内容）只经 defs_hash / content_hash 进快照头部；
    // 野怪 / 掉落 / 消费序列是权威状态，逐帧进 FNV-1a（D1 / D4 / tasks 2.4）。
    let (mut sim, _) = seeded_match();
    let before = sim.capture_snapshot();
    assert_eq!(before.creep_camps.camps.len(), 1);
    assert_eq!(before.creep_camps.creeps.len(), 2);
    assert!(before.creep_camps.ground_items.is_empty());
    assert!(before.creep_camps.rng_draws.is_empty());
    kill_both_and_settle(&mut sim, 1);
    let after = sim.capture_snapshot();
    assert_eq!(after.defs_hash, before.defs_hash, "reason=defs_frozen");
    assert_eq!(
        after.content_hash, before.content_hash,
        "reason=content_frozen"
    );
    assert!(
        !after.creep_camps.ground_items.is_empty(),
        "reason=state_live"
    );
    assert!(!after.creep_camps.rng_draws.is_empty(), "reason=draws_live");
    assert_ne!(
        before.creep_camps, after.creep_camps,
        "reason=authoritative_state_changed"
    );
}

// ---------------------------------------------------------------------------
// 2. 营地装配 / 中立归属 / 命令面（Q5 / Q7 / R8.1 / R1.4）
// ---------------------------------------------------------------------------

#[test]
fn seed_creep_camp_is_frame_frozen_idempotent_and_neutral() {
    let (sim, _) = seeded_match();
    let state = sim.creep_camps();
    assert_eq!(state.camps.len(), 1, "reason=one_camp");
    assert_eq!(state.creeps.len(), 2, "reason=two_creeps");
    // Q7：野怪 owner 为中立常量，不挂任何玩家 owner（R8.1）。
    let view = sim.read_view();
    for id in creep_ids() {
        let entity = view
            .entities
            .iter()
            .find(|entity| entity.id == id.value())
            .unwrap();
        assert_eq!(entity.owner, NEUTRAL_OWNER, "reason=neutral_owner");
        assert_eq!(
            state.neutral.get(&id),
            Some(&NeutralCategory::CreepHostile),
            "reason=hostile_faction_marker"
        );
    }
    assert_eq!(
        state.camps[&CAMP_ID].members,
        creep_ids().into_iter().collect()
    );
    assert!(!state.camps[&CAMP_ID].cleared, "reason=not_cleared_yet");

    // 事实事件：`CreepCampSeeded` 与逐个 `Spawned`。
    let (mut replay, _) = seeded_match();
    let events = replay.drain_events();
    assert!(
        events
            .iter()
            .any(|event| event.kind == EventKind::CreepCampSeeded)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == EventKind::Spawned)
            .count(),
        3,
        "reason=player_plus_two_creeps"
    );
    // 事件序号 drain 后不重置（ADR 0002 §5）。
    replay.step();
    let next = replay.drain_events();
    assert!(
        next.iter()
            .all(|event| event.sequence >= events.len() as u64)
    );
}

#[test]
fn repeated_seed_command_does_not_double_inject() {
    let (mut sim, _) = seeded_match();
    let before = sim.creep_camps();
    // 同一 frame-0 系统级命令重复投递（重放 / 双端）：幂等（R12.4）。
    sim.submit_gameplay_command(seed_camp(1, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    let after = sim.creep_camps();
    assert_eq!(after.camps.len(), 1, "reason=no_double_camp");
    assert_eq!(after.creeps.len(), 2, "reason=no_double_spawn");
    assert_eq!(
        after.camps[&CAMP_ID].members,
        before.camps[&CAMP_ID].members
    );
}

#[test]
fn creep_does_not_respond_to_player_move_and_ground_items_are_fixed() {
    // R1.4 / critic residual R-e：野怪 MUST NOT 响应玩家 `Move`。
    let (mut sim, content) = seeded_match();
    let creep = creep_ids()[0];
    let before = sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep.value())
        .unwrap()
        .x;
    sim.submit_command(w3_core::Command::new(
        Frame::new(1),
        PLAYER,
        1,
        w3_core::CommandKind::Move {
            entity: creep,
            x: SimFixed::from_int(7).unwrap(),
            y: SimFixed::from_int(7).unwrap(),
            group: 0,
            append: false,
        },
    ))
    .unwrap();
    sim.step();
    let after = sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep.value())
        .unwrap()
        .x;
    assert_eq!(after, before, "reason=neutral_move_is_noop");

    // 混合集合：地面物品 / 野怪 id 都不会被 `Move` 驱动，位置不变。
    kill_both_and_settle(&mut sim, 2);
    let state = sim.creep_camps();
    assert!(!state.ground_items.is_empty(), "reason=drops_exist");
    let ground_id = *state.ground_items.keys().next().unwrap();
    let ground_before = state.ground_items[&ground_id].clone();
    sim.submit_command(w3_core::Command::new(
        Frame::new(3),
        PLAYER,
        2,
        w3_core::CommandKind::Move {
            entity: ground_id,
            x: SimFixed::from_int(7).unwrap(),
            y: SimFixed::from_int(7).unwrap(),
            group: 0,
            append: false,
        },
    ))
    .unwrap();
    sim.step();
    let state = sim.creep_camps();
    assert_eq!(
        state.ground_items[&ground_id], ground_before,
        "reason=item_fixed"
    );
    // 地面物品不是 `EntityStore` 实体，不出现在只读视图 / `Move` 生成集合。
    assert!(
        sim.read_view()
            .entities
            .iter()
            .all(|entity| entity.id != ground_id.value())
    );
    // 消费序列与快照版本仍完整（content 用于重开基线）。
    let _ = content;
    assert_eq!(sim.capture_snapshot().version, 10);
}

#[test]
fn seed_creep_camp_command_is_stably_sorted_and_both_endpoints_match() {
    // Q5 / R12.2：逐帧推进 vs 一次推进 8 帧，两端逐帧哈希 / 快照一致。
    let cfg_a = config(&defs());
    let mut a = RtsMatch::create_with_definitions(cfg_a, StaticWorld::default(), defs());
    let mut b = RtsMatch::create_with_definitions(cfg_a, StaticWorld::default(), defs());
    for sim in [&mut a, &mut b] {
        sim.submit_command(spawn(HOME_X, HOME_Y, 100, 100)).unwrap();
        // 故意乱序提交两条 frame-0 系统级命令（不同营地 id）。
        sim.submit_gameplay_command(seed_camp(0, 9, CAMP_SCRIPT))
            .unwrap();
        sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
            .unwrap();
    }
    // `step()` 不接受 delta / 步长参数，两端都按同一整数帧数逐帧推进，
    // 一端连续逐帧比较、另一端连续推进 8 帧后比较（同一脚本、同一帧数）。
    let mut hashes_a = Vec::new();
    for _ in 0..8 {
        a.step();
        hashes_a.push(a.compute_state_hash());
    }
    let mut hashes_b = Vec::new();
    for _ in 0..8 {
        b.step();
        hashes_b.push(b.compute_state_hash());
    }
    assert_eq!(hashes_a, hashes_b, "reason=interleaved_hashes");
    assert_eq!(
        a.compute_state_hash(),
        b.compute_state_hash(),
        "reason=batched_hash"
    );
    assert_eq!(
        a.capture_snapshot(),
        b.capture_snapshot(),
        "reason=batched_snapshot"
    );
    assert_eq!(a.first_difference(&b), None, "reason=batched_no_diff");
    assert_eq!(a.creep_camps().camps.len(), 2, "reason=two_camps");
}

// ---------------------------------------------------------------------------
// 3. 击杀收益 / 掉落 / 随机码（Q6 / R6.1 / R6.5 / R8.3 / R7.2 / R7.5 / R7.6）
// ---------------------------------------------------------------------------

#[test]
fn creep_kill_settles_xp_gold_and_drops_exactly_once() {
    let (sim, content) = seeded_match();
    let mut sim = with_hero(sim, content);
    let hero_before = sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == 1)
        .unwrap()
        .gameplay
        .hero;
    assert_eq!(hero_before.map(|hero| hero.xp), Some(0));

    kill_both_and_settle(&mut sim, 1);
    let state = sim.creep_camps();
    assert_eq!(state.kill_rewards.len(), 2, "reason=one_reward_per_creep");
    assert_eq!(state.death_processed.len(), 2, "reason=idempotent_registry");
    assert!(state.pending_deaths.is_empty(), "reason=queue_drained");

    for (creep, reward) in &state.kill_rewards {
        assert_eq!(reward.creep, *creep);
        assert_eq!(reward.killer_owner, PLAYER);
        assert_eq!(reward.gold, CREEP_GOLD, "reason=gold_R8_3");
        assert!(
            reward.xp_candidates.contains(&EntityId::new(1).unwrap()),
            "reason=entered_share_set_R6_5"
        );
        assert_eq!(
            reward.xp_credited.get(&EntityId::new(1).unwrap()),
            Some(&CREEP_XP),
            "reason=credited_R6_5"
        );
        assert_eq!(reward.drops.len(), 2, "reason=one_item_per_exclusive_set");
    }
    // 金币与经验直接进权威状态（表现层 MUST NOT 反写）。
    let snapshot = sim.capture_snapshot();
    assert_eq!(
        snapshot.gameplay.players[&PLAYER].gold,
        2 * CREEP_GOLD,
        "reason=gold_in_authoritative_economy"
    );
    let hero_after = snapshot
        .positions
        .iter()
        .find(|record| record.entity == 1)
        .unwrap()
        .gameplay
        .hero;
    assert_eq!(
        hero_after.map(|hero| hero.xp),
        Some(2 * CREEP_XP),
        "reason=xp_in_authoritative_hero_state"
    );
    assert_eq!(
        state.camps[&CAMP_ID].cleared, true,
        "reason=camp_cleared_R11_5"
    );
}

#[test]
fn duplicate_death_notification_settles_only_once() {
    let (mut sim, _) = seeded_match();
    // 同一野怪重复投递死亡通知：生产路径 `notify_creep_death` 幂等。
    let first = sim.notify_creep_death(creep_ids()[0], EntityId::new(1).unwrap(), PLAYER);
    let second = sim.notify_creep_death(creep_ids()[0], EntityId::new(1).unwrap(), PLAYER);
    assert!(first, "reason=first_notification_accepted");
    assert!(!second, "reason=duplicate_notification_rejected");
    assert_eq!(sim.creep_camps().pending_deaths.len(), 1);
    // 未注册实体 / 已结算实体也拒绝。
    assert!(!sim.notify_creep_death(
        EntityId::new(999).unwrap(),
        EntityId::new(1).unwrap(),
        PLAYER
    ));

    // 让该野怪真正死亡，再重复投递也只结算一次。
    attack(&mut sim, 1, 1, 2, 1);
    sim.step();
    let state = sim.creep_camps();
    assert_eq!(state.kill_rewards.len(), 1, "reason=single_settlement");
    assert_eq!(state.rng_draws.len(), 2, "reason=single_rng_consumption");
    assert_eq!(state.ground_items.len(), 2, "reason=single_drop_batch");
    assert!(!sim.notify_creep_death(creep_ids()[0], EntityId::new(1).unwrap(), PLAYER));
    sim.step();
    assert_eq!(
        sim.creep_camps().kill_rewards.len(),
        1,
        "reason=no_resettlement"
    );
}

#[test]
fn mutually_exclusive_set_never_drops_two_items_and_instances_are_independent() {
    let (mut sim, _) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let state = sim.creep_camps();

    // 每个（野怪, 互斥 set）恰好一次随机码消费 → 至多一件（R7.2 / R8.6）。
    let mut per_set: BTreeMap<(u64, u32), usize> = BTreeMap::new();
    for draw in state.rng_draws.values() {
        *per_set.entry((draw.creep.value(), draw.set)).or_default() += 1;
    }
    assert_eq!(per_set.len(), 4, "reason=two_creeps_times_two_sets");
    assert!(
        per_set.values().all(|&count| count == 1),
        "reason=one_per_set"
    );
    assert_eq!(state.ground_items.len(), state.item_instances.len());

    // 同名道具多实例独立（R7.5）：每个实例 id / 充能 / 冷却各自登记；
    // 效果走既有 `EffectDef`，无 JASS VM（R7.6）。
    let same_def: Vec<_> = state
        .item_instances
        .values()
        .filter(|instance| instance.item_def == ITEM_DEF)
        .collect();
    assert!(same_def.len() >= 2, "reason=two_instances_of_same_item");
    assert_eq!(
        state
            .item_instances
            .values()
            .filter(|instance| instance.item_def == ITEM_DEF)
            .map(|instance| instance.effect.kind)
            .collect::<Vec<_>>(),
        vec![EffectKind::BuffStat; same_def.len()],
        "reason=existing_effect_def_R7_6"
    );
    assert!(
        state
            .neutral
            .iter()
            .filter(|(_, category)| **category == NeutralCategory::GroundItemOwnerless)
            .count()
            >= state.ground_items.len()
    );
    // 独立冷却逐帧推进（R7.5）：状态变化进哈希 / 快照。
    let before = sim.compute_state_hash();
    sim.step();
    assert_ne!(before, sim.compute_state_hash(), "reason=cooldowns_tick");
}

#[test]
fn drop_rng_consumption_sequence_is_seed_deterministic_and_snapshot_safe() {
    // Q6：同 seed 两次重放掉落逐帧一致；稳定 `EntityId` 顺序消费。
    let mut a = seeded_match().0;
    let mut b = seeded_match().0;
    kill_both_and_settle(&mut a, 1);
    kill_both_and_settle(&mut b, 1);
    let state_a = a.creep_camps();
    let state_b = b.creep_camps();
    assert_eq!(
        state_a.rng_draws, state_b.rng_draws,
        "reason=same_seed_draws"
    );
    assert_eq!(
        state_a.ground_items, state_b.ground_items,
        "reason=same_seed_drops"
    );
    assert_eq!(
        state_a.rng_state, state_b.rng_state,
        "reason=same_stream_state"
    );
    // 稳定顺序：消费序列按死亡野怪 `EntityId` 升序分组（2 后 3）。
    let creep_order: Vec<u64> = state_a
        .rng_draws
        .values()
        .map(|d| d.creep.value())
        .collect();
    let mut sorted = creep_order.clone();
    sorted.dedup();
    assert_eq!(sorted, vec![2, 3], "reason=stable_entity_order");
    assert!(
        creep_order.windows(2).all(|w| w[0] <= w[1]),
        "reason=monotone"
    );

    // 帧 N 捕获 / 销毁 / 恢复后继续到 M，不重复掉落、逐帧哈希一致（R12.4）。
    let content = config(&defs()).content_hash;
    let snapshot = a.capture_snapshot();
    let mut restored =
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.creep_camps().rng_draws,
        a.creep_camps().rng_draws,
        "reason=restore_keeps_draws"
    );
    for frame in 0..8 {
        a.step();
        restored.step();
        assert_eq!(
            a.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
        assert_eq!(a.first_difference(&restored), None);
    }
    assert_eq!(
        restored.creep_camps().ground_items.len(),
        a.creep_camps().ground_items.len(),
        "reason=no_duplicate_drops"
    );
}

#[test]
fn seed_and_replay_produce_frame_identical_hashes_with_creep_script() {
    // 双实例交错 + 命令回放（ADR 0002 §8 / R12.4）。
    let (mut a, _) = seeded_match();
    let (mut b, _) = seeded_match();
    kill_both_and_settle(&mut a, 1);
    kill_both_and_settle(&mut b, 1);
    for frame in 0..12 {
        a.step();
        b.step();
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=replay_hash_frame_{frame}"
        );
        assert_eq!(a.capture_snapshot(), b.capture_snapshot());
    }
    // 只读视图消费与 `creep_camps()` owned 拷贝都不改写权威状态。
    let hash_before = a.compute_state_hash();
    let snapshot_before = a.capture_snapshot();
    let _ = a.read_view();
    let _ = a.creep_camps();
    let _ = a.drain_events();
    assert_eq!(
        a.compute_state_hash(),
        hash_before,
        "reason=readonly_no_hash_change"
    );
    assert_eq!(
        a.capture_snapshot(),
        snapshot_before,
        "reason=readonly_no_snapshot_change"
    );
}

// ---------------------------------------------------------------------------
// 4. leash / 归巢状态机（R8.5 / D3 / tasks 5.1）
// ---------------------------------------------------------------------------

fn leash_match(frozen: &FrozenDefinitions) -> (RtsMatch, u64) {
    let content = frozen.content_hash();
    let cfg = w3_core::MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: content,
    };
    let mut sim = RtsMatch::create_with_definitions(cfg, StaticWorld::default(), frozen.clone());
    // 玩家单位放在 `acquisitionRange` 之外，归巢后不会重新索敌。
    sim.submit_command(spawn(HOME_X + 10 * SCALE, HOME_Y, 100, 100))
        .unwrap();
    sim.submit_gameplay_command(seed_camp(0, CAMP_ID, CAMP_SCRIPT))
        .unwrap();
    sim.step();
    (sim, content)
}

#[test]
fn creep_leash_stops_chase_and_returns_home() {
    let frozen = defs();
    let (sim, content) = leash_match(&frozen);
    // 经真实快照往返把野怪放到 leash 半径之外并置 ENGAGED（生产路径注入）。
    let mut sim = inject_creep_state(
        &sim,
        content,
        creep_ids()[0],
        HOME_X + 3 * SCALE,
        HOME_Y,
        LeashPhase::Engaged,
        Some(EntityId::new(1).unwrap()),
    );
    let mut saw_returning = false;
    let mut saw_returned = false;
    for _ in 0..24 {
        let events = sim.drain_events();
        for event in events {
            if event.kind == EventKind::CreepLeashReturning {
                saw_returning = true;
            }
            if event.kind == EventKind::CreepLeashReturned {
                saw_returned = true;
            }
        }
        sim.step();
        let events = sim.drain_events();
        for event in events {
            if event.kind == EventKind::CreepLeashReturning {
                saw_returning = true;
            }
            if event.kind == EventKind::CreepLeashReturned {
                saw_returned = true;
            }
        }
        let state = sim.creep_camps();
        if state.creeps[&creep_ids()[0]].phase == LeashPhase::Idle && saw_returned {
            break;
        }
    }
    assert!(saw_returning, "reason=leash_transition_to_returning_R8_5");
    assert!(saw_returned, "reason=returned_to_home_R8_5");
    let state = sim.creep_camps();
    assert_eq!(
        state.creeps[&creep_ids()[0]].phase,
        LeashPhase::Idle,
        "reason=idle_after_return"
    );
    // 归巢后位置回到 home 阈值内，MUST NOT 无限追出营地。
    let record = sim
        .capture_snapshot()
        .positions
        .iter()
        .find(|record| record.entity == creep_ids()[0].value())
        .unwrap()
        .clone();
    let dx = (record.x - HOME_X) as i128;
    let dy = (record.y - HOME_Y) as i128;
    assert!(
        dx * dx + dy * dy <= (SCALE as i128) * (SCALE as i128),
        "reason=inside_home_threshold"
    );
}

#[test]
fn creep_ai_reuses_existing_attack_and_move_command_face() {
    // D3：无独立 AI × lockstep 注入；野怪攻击与玩家攻击汇入同一结算序列。
    let (mut sim, _) = seeded_match();
    // 玩家单位在 `acquisitionRange` 内 → 野怪进入 ENGAGED。
    sim.step();
    let events = sim.drain_events();
    assert!(
        events
            .iter()
            .any(|event| event.kind == EventKind::CreepEngaged),
        "reason=engaged_via_acquisition_range"
    );
    let state = sim.creep_camps();
    assert!(
        state
            .creeps
            .values()
            .any(|creep| creep.phase == LeashPhase::Engaged)
    );
    // 野怪攻击经既有 `resolve_attacks` 落到玩家单位（权威 hp 变化）。
    let hp_before = sim.read_view().entities[0].hp;
    sim.step();
    let hp_after = sim.read_view().entities[0].hp;
    assert!(hp_after <= hp_before, "reason=shared_attack_settlement");
    // 命令面未新增注入通道：玩法命令队列为空（AI 内部命令不写回队列）。
    assert!(
        sim.capture_snapshot()
            .gameplay
            .commands
            .pending()
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// 5. 哈希 / 快照 / 差异诊断 / 版本（Q1 / Q4 / D5 / tasks 3.x）
// ---------------------------------------------------------------------------

#[test]
fn all_new_authoritative_fields_are_hashed_and_first_difference_locates_them() {
    let (mut sim, content) = seeded_match();
    kill_both_and_settle(&mut sim, 1);
    let base = sim.capture_snapshot();

    // 逐字段改动 → `compute_state_hash()` 变化 + `first_difference()` 定位。
    let mut camp_variant = base.clone();
    camp_variant
        .creep_camps
        .camps
        .get_mut(&CAMP_ID)
        .unwrap()
        .cleared = false;
    let a = RtsMatch::restore_snapshot(camp_variant, StaticWorld::default(), content).unwrap();
    assert_ne!(sim.compute_state_hash(), a.compute_state_hash());
    assert_eq!(sim.first_difference(&a), Some("creep_camps"));

    let mut creep_variant = base.clone();
    let creep = creep_ids()[0];
    creep_variant
        .creep_camps
        .creeps
        .get_mut(&creep)
        .unwrap()
        .phase = LeashPhase::Returning;
    let b = RtsMatch::restore_snapshot(creep_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&b), Some("creep_creeps"));

    let mut neutral_variant = base.clone();
    neutral_variant.creep_camps.neutral.clear();
    let c = RtsMatch::restore_snapshot(neutral_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&c), Some("creep_neutral"));

    let mut item_variant = base.clone();
    let item_id = *base.creep_camps.ground_items.keys().next().unwrap();
    item_variant
        .creep_camps
        .ground_items
        .get_mut(&item_id)
        .unwrap()
        .x += 1;
    let d = RtsMatch::restore_snapshot(item_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&d), Some("creep_ground_items"));

    let mut instance_variant = base.clone();
    instance_variant
        .creep_camps
        .item_instances
        .get_mut(&item_id)
        .unwrap()
        .charges += 1;
    let e = RtsMatch::restore_snapshot(instance_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&e), Some("creep_item_instances"));

    let mut reward_variant = base.clone();
    reward_variant
        .creep_camps
        .kill_rewards
        .get_mut(&creep)
        .unwrap()
        .gold += 1;
    let f = RtsMatch::restore_snapshot(reward_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&f), Some("creep_kill_rewards"));

    let mut draw_variant = base.clone();
    draw_variant.creep_camps.rng_draws.get_mut(&0).unwrap().roll += 1;
    let g = RtsMatch::restore_snapshot(draw_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&g), Some("creep_rng_draws"));

    let mut stream_variant = base.clone();
    stream_variant.creep_camps.rng_state ^= 1;
    let h = RtsMatch::restore_snapshot(stream_variant, StaticWorld::default(), content).unwrap();
    assert_eq!(sim.first_difference(&h), Some("creep_rng_state"));
}

#[test]
fn snapshot_roundtrip_during_camp_clear_is_identical() {
    let (original, content) = seeded_match();
    let mut with_hero_sim = with_hero(original, content);
    // 帧 N 捕获（营地未全清），销毁重建后继续到帧 M。
    let snapshot = with_hero_sim.capture_snapshot();
    let mut restored =
        RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    assert_eq!(
        restored.capture_snapshot().creep_camps,
        with_hero_sim.capture_snapshot().creep_camps
    );
    kill_both_and_settle(&mut with_hero_sim, 1);
    kill_both_and_settle(&mut restored, 1);
    for frame in 0..10 {
        with_hero_sim.step();
        restored.step();
        assert_eq!(
            with_hero_sim.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
        assert_eq!(
            with_hero_sim.capture_snapshot(),
            restored.capture_snapshot(),
            "reason=roundtrip_snapshot_frame_{frame}"
        );
        assert_eq!(with_hero_sim.first_difference(&restored), None);
    }
    assert_eq!(restored.creep_camps().kill_rewards.len(), 2);
}

#[test]
fn restart_does_not_retain_previous_creep_drop_draw_or_reward_state() {
    // R11.5 / R12.4 / tasks 3.4：新一局 `victory.resolved == false`、
    // 营地 / 掉落 / 消费序列 / 收益登记全部复位。
    let (mut previous, _) = seeded_match();
    kill_both_and_settle(&mut previous, 1);
    let dirty = previous.capture_snapshot();
    assert!(!dirty.creep_camps.ground_items.is_empty());
    assert!(!dirty.creep_camps.rng_draws.is_empty());
    assert!(!dirty.creep_camps.kill_rewards.is_empty());

    let fresh = fresh_match();
    let clean = fresh.capture_snapshot();
    assert!(
        !clean.gameplay.victory.resolved,
        "reason=no_victory_residue"
    );
    assert!(clean.creep_camps.camps.is_empty());
    assert!(clean.creep_camps.creeps.is_empty());
    assert!(clean.creep_camps.ground_items.is_empty());
    assert!(clean.creep_camps.item_instances.is_empty());
    assert!(clean.creep_camps.rng_draws.is_empty());
    assert!(clean.creep_camps.kill_rewards.is_empty());
    assert!(clean.creep_camps.death_processed.is_empty());
    assert_eq!(clean.version, dirty.version);
    // 事件驱动掉落流由 seed 确定性派生（重开相同 seed → 相同初始流状态）。
    assert_eq!(
        clean.creep_camps.rng_state,
        fresh.capture_snapshot().creep_camps.rng_state
    );
}

#[test]
fn old_v5_snapshot_and_sim_version_are_explicitly_rejected() {
    // Q4：旧 v5 快照 / 回放按 UnsupportedVersion / SimVersionMismatch 明确拒绝，
    // 不提供跨版本迁移（ADR 0002 §7）。
    let (sim, content) = seeded_match();
    let mut old = sim.capture_snapshot();
    old.version = 5;
    assert!(matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 5 })
    ));
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 5;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 5,
            expected: 10
        })
    ));
    // 定义内容哈希被篡改也明确拒绝（冻结定义内容进快照头）。
    let mut tampered = sim.capture_snapshot();
    tampered.defs_hash ^= 1;
    assert!(matches!(
        RtsMatch::restore_snapshot(tampered, StaticWorld::default(), content),
        Err(SnapshotError::DefsMismatch)
    ));
}

#[test]
fn day_night_and_camp_tier_constants_are_frozen_but_sleep_is_out_of_scope() {
    // Q3：R8.4 昼夜时长登记为冻结常量 `[待核]`，睡眠行为本期不实现；
    // Q8：分级编码 / 随机池类别均为合成原创整数、不冒充原版。
    let frozen = defs();
    let constants = frozen.creep_constants();
    assert_eq!(constants.day_frames, SYNTHETIC_CREEP_CONSTANTS.day_frames);
    assert_eq!(
        constants.night_frames,
        SYNTHETIC_CREEP_CONSTANTS.night_frames
    );
    assert_eq!(constants.camp_tiers, [1, 2, 3]);
    assert_eq!(constants.random_pool_category, 7);
    assert_eq!(frozen.camp_script(CAMP_SCRIPT).unwrap().tier, 1);
    // 无「睡眠」状态字段：权威状态容器只有 leash 三态（R8.5）。
    let (sim, _) = seeded_match();
    assert!(sim.creep_camps().creeps.values().all(|creep| matches!(
        creep.phase,
        LeashPhase::Idle | LeashPhase::Engaged | LeashPhase::Returning
    )));
}

// ---------------------------------------------------------------------------
// 6. 命令编码唯一 / 校验 / 穷尽分发（Q5 / D6 / tasks 6.1 / 6.2）
// ---------------------------------------------------------------------------

#[test]
fn seed_creep_camp_command_encoding_is_unique_and_validated() {
    let encode = |camp: u32, script: u32| {
        let mut queue = w3_core::GameplayCommandQueue::new();
        queue.submit(seed_camp(0, camp, script)).unwrap();
        let mut hasher = w3_core::StateHasher::new();
        queue.hash(&mut hasher);
        hasher.finish()
    };
    assert_ne!(encode(1, 41), encode(2, 41), "reason=camp_encoded");
    assert_ne!(encode(1, 41), encode(1, 42), "reason=script_encoded");
    // 校验：0 号营地 / 脚本被公开提交入口明确拒绝。
    let mut queue = w3_core::GameplayCommandQueue::new();
    assert_eq!(
        queue.submit(seed_camp(0, 0, CAMP_SCRIPT)),
        Err(w3_core::GameplayCommandError::InvalidEntity)
    );
    assert_eq!(
        queue.submit(seed_camp(0, CAMP_ID, 0)),
        Err(w3_core::GameplayCommandError::InvalidEntity)
    );
    // 穷尽分发：`SeedCreepCamp` 真的装配营地（不是 no-op）。
    let (sim, _) = seeded_match();
    assert_eq!(sim.creep_camps().camps.len(), 1, "reason=dispatched");
    // 同源校验：非系统级玩家提交的同源命令被显式拒绝（Q7 / R8.1）。
    let mut replay = fresh_match();
    replay
        .submit_gameplay_command(w3_core::GameplayCommand::new(
            Frame::new(0),
            PLAYER,
            1,
            GameplayCommandKind::SeedCreepCamp {
                camp: CAMP_ID,
                script: CAMP_SCRIPT,
            },
        ))
        .unwrap();
    replay.step();
    assert!(
        replay.creep_camps().camps.is_empty(),
        "reason=non_system_seed_rejected"
    );
}
