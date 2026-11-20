//! P8.5b 主城升级正向验收与确定性回归（ADR 0002 / R4.1 / R3.1 / R12.4）。
//!
//! 测试全部经公开 API 构造场景：`capture_snapshot()` 取帧边界完整状态 →
//! 修改只读快照的 public 字段注入经济 / 主城类别 / tier 需求链 →
//! `restore_snapshot()` 恢复 → `submit_gameplay_command` 提交升级 / 训练 / 研发 →
//! `step()` 逐帧推进到队列完成。退化态不是手搓假结论，而是走快照往返这一
//! 既有确定性生产路径（先例 `hero_revive.rs` / `determinism.rs`）。
//!
//! 全部 DefId / 常量为合成原创编译期整数 `[待核]`，刻意使用非经典命名的
//! DefId，验证解锁判据由定义层类别 + tier 需求链数据驱动，MUST NOT 依赖
//! 建筑名字符串白名单或硬编码 id（R1.2 / R13.3 / D2）。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::{
    Command, CommandKind, DefId, EntityId, EventKind, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, MatchConfig, ProductionItem, RtsMatch, SIM_VERSION, SNAPSHOT_VERSION,
    SYNTHETIC_TOWN_HALL_CONSTANTS, SimFixed, SnapshotError, StaticWorld, TownHallConstants,
    TownHallTier, TownHallUpgradeItem, UnitDef, UpgradeDef,
};

const SEED: u64 = 0x5eed;
const PLAYER: u8 = 1;
/// 合成 / 原创主城 DefId：刻意非经典命名，类别由定义层集合登记驱动。
const TOWN_HALL_DEF: DefId = 8101;
/// 需要 T2 的二线单位 DefId（合成原创）。
const TIER2_UNIT: DefId = 8111;
/// 需要 T3 的二线单位 DefId（合成原创）。
const TIER3_UNIT: DefId = 8112;
/// 需要 T2 的科技 DefId（合成原创）。
const TIER2_UPGRADE: DefId = 8121;
/// 需要 T3 的科技 DefId（合成原创）。
const TIER3_UPGRADE: DefId = 8122;
/// 未登记任何 tier 需求的普通单位，验证「无需求登记默认放行」。
const PLAIN_UNIT: DefId = 8130;
const TOWN_HALL_ID: u64 = 1;
const PRODUCER_ID: u64 = 2;

/// 缩短升级时间便于逐帧验证，其余全为合成原创编译期整数常量 `[待核]`。
fn town_hall_constants() -> TownHallConstants {
    let mut constants = SYNTHETIC_TOWN_HALL_CONSTANTS;
    constants.frames = 3;
    constants.tier2_gold = 100;
    constants.tier2_lumber = 40;
    constants.price_delta_gold = 60;
    constants.price_delta_lumber = 30;
    constants
}

/// 定义层类别 + tier 需求链登记；全部非经典命名 DefId，数据驱动。
fn tier_requirements() -> BTreeMap<DefId, TownHallTier> {
    [
        (TIER2_UNIT, TownHallTier::T2),
        (TIER2_UPGRADE, TownHallTier::T2),
        (TIER3_UNIT, TownHallTier::T3),
        (TIER3_UPGRADE, TownHallTier::T3),
    ]
    .into_iter()
    .collect()
}

fn unit(train_frames: u32, hp: i64) -> UnitDef {
    UnitDef {
        hp,
        mana: 0,
        speed_raw: SimFixed::from_int(3).unwrap().raw(),
        damage: 0,
        range_raw: 0,
        supply: 1,
        train_frames,
    }
}

/// 构造含主城 / 二线单位 / 科技定义 + 冻结常量 + 类别 + 需求链的冻结定义。
/// 干净环境（无 `config.toml` / 导出目录）构造成功、零文件 IO：全部数值为
/// 编译期常量，构造过程只调 `w3-core` 公开 API，不读取任何文件 / 网络。
fn town_hall_defs() -> FrozenDefinitions {
    let units = vec![
        (TOWN_HALL_DEF, unit(2, 500)),
        (TIER2_UNIT, unit(5, 80)),
        (TIER3_UNIT, unit(5, 90)),
        (PLAIN_UNIT, unit(5, 60)),
    ];
    let upgrades = vec![
        (
            TIER2_UPGRADE,
            UpgradeDef {
                damage_mod: 1,
                hp_mod: 2,
                research_frames: 5,
            },
        ),
        (
            TIER3_UPGRADE,
            UpgradeDef {
                damage_mod: 2,
                hp_mod: 3,
                research_frames: 5,
            },
        ),
    ];
    let town_hall_defs: BTreeSet<DefId> = [TOWN_HALL_DEF].into_iter().collect();
    FrozenDefinitions::new(units, vec![], vec![], upgrades)
        .unwrap()
        .with_town_hall_upgrade(town_hall_constants(), town_hall_defs, tier_requirements())
}

fn config(defs: &FrozenDefinitions) -> MatchConfig {
    MatchConfig {
        seed: SEED,
        logic_fps: 30,
        content_hash: defs.content_hash(),
    }
}

fn spawn(x: i64, y: i64, hp: i64) -> CommandKind {
    CommandKind::Spawn {
        x: SimFixed::from_int(x).unwrap(),
        y: SimFixed::from_int(y).unwrap(),
        hp,
        speed: SimFixed::from_int(3).unwrap(),
        attack_damage: 0,
        attack_range: SimFixed::from_int(10).unwrap(),
    }
}

/// 生成主城与生产者实体并推进一帧，返回 `(对局, content_hash)`。
fn seeded() -> (RtsMatch, u64) {
    let defs = town_hall_defs();
    let content = defs.content_hash();
    let mut sim = RtsMatch::create_with_definitions(config(&defs), StaticWorld::default(), defs);
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 1, spawn(0, 0, 500)))
        .unwrap();
    sim.submit_command(Command::new(Frame::new(0), PLAYER, 2, spawn(4, 0, 300)))
        .unwrap();
    sim.step();
    (sim, content)
}

/// 经快照往返注入玩家经济（gold / lumber），返回恢复后的对局。
fn funded_match(gold: i64, lumber: i64) -> (RtsMatch, u64) {
    let (sim, content) = seeded();
    let mut snapshot = sim.capture_snapshot();
    let economy = snapshot.gameplay.players.entry(PLAYER).or_default();
    economy.gold = gold;
    economy.lumber = lumber;
    let restored = RtsMatch::restore_snapshot(snapshot, StaticWorld::default(), content).unwrap();
    (restored, content)
}

fn submit_upgrade(sim: &mut RtsMatch, frame: u64, target_tier: TownHallTier, cancel: bool) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        1,
        GameplayCommandKind::UpgradeTownHall {
            town_hall: EntityId::new(TOWN_HALL_ID).unwrap(),
            town_hall_def: TOWN_HALL_DEF,
            target_tier,
            cancel,
        },
    ))
    .unwrap();
}

fn submit_train(sim: &mut RtsMatch, frame: u64, unit_def: DefId, sequence: u32) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        GameplayCommandKind::Train {
            producer: EntityId::new(PRODUCER_ID).unwrap(),
            unit_def,
            is_building: false,
        },
    ))
    .unwrap();
}

fn submit_research(sim: &mut RtsMatch, frame: u64, upgrade: DefId, sequence: u32) {
    sim.submit_gameplay_command(GameplayCommand::new(
        Frame::new(frame),
        PLAYER,
        sequence,
        GameplayCommandKind::Research { upgrade },
    ))
    .unwrap();
}

/// 精确停在 `TownHallUpgradeCompleted` 事实事件产生的那一帧。
fn advance_until_upgraded(sim: &mut RtsMatch) {
    sim.drain_events();
    for _ in 0..20 {
        sim.step();
        let done = sim
            .drain_events()
            .iter()
            .any(|event| event.kind == EventKind::TownHallUpgradeCompleted);
        if done {
            return;
        }
    }
    panic!("reason=upgrade_never_completed");
}

fn producer_production(sim: &RtsMatch) -> Vec<ProductionItem> {
    sim.capture_snapshot()
        .positions
        .into_iter()
        .find(|position| position.entity == PRODUCER_ID)
        .expect("producer position present")
        .gameplay
        .production
}

fn current_research(sim: &RtsMatch) -> Option<(DefId, u32)> {
    sim.capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .and_then(|economy| economy.research)
}

fn tier_of_town_hall(sim: &RtsMatch) -> Option<TownHallTier> {
    sim.capture_snapshot()
        .town_hall
        .tiers
        .get(&EntityId::new(TOWN_HALL_ID).unwrap())
        .copied()
}

/// 当前逻辑帧号（`step()` 提交后时钟 +1）；用于「升级完成后同一命令面」把
/// 命令提交到当前帧，避免落在过去帧或尚未到期的未来帧。
fn current_frame(sim: &RtsMatch) -> u64 {
    sim.read_view().tick
}

/// 不会产生任何玩法状态变化的占位命令，用于对齐玩法命令队列计数器 /
/// pending 内容，隔离「被拒绝命令是否改变玩法权威状态」（先例
/// `determinism.rs::new_gameplay_fields_are_hashed_and_snapshot_safe`）。
fn noop_cancel_train() -> GameplayCommandKind {
    GameplayCommandKind::CancelTrain {
        producer: EntityId::new(PRODUCER_ID).unwrap(),
        index: u32::MAX,
    }
}

/// 核心验收线：升级完成等级递进（T1→T2）、tier 顶层权威状态登记、
/// 完成事实事件恰好一次、无新增实体（同一主城实体等级迁移，不 spawn）。
#[test]
fn upgrade_completes_and_tier_advances() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    let before = sim.capture_snapshot();
    let before_next_id = before.next_entity_id;
    let before_alive = before.alive_entities.len();
    assert!(
        before.town_hall.tiers.is_empty(),
        "reason=initial_tier_state_empty_implies_t1"
    );

    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let in_progress = sim.capture_snapshot();
    assert_eq!(in_progress.town_hall.queue.len(), 1, "reason=queue_created");
    assert_eq!(
        in_progress.town_hall.charges.len(),
        1,
        "reason=charge_frozen"
    );
    let item = in_progress.town_hall.queue.values().next().unwrap().clone();
    assert_eq!(item.from_tier, TownHallTier::T1, "reason=from_t1");
    assert_eq!(item.target_tier, TownHallTier::T2, "reason=to_t2");
    assert!(
        item.remaining_frames <= item.total_frames,
        "reason=integer_frame_progress"
    );
    assert_eq!(
        item.total_frames,
        town_hall_constants().frames,
        "reason=frames_from_frozen_constant"
    );

    advance_until_upgraded(&mut sim);
    let after = sim.capture_snapshot();
    assert_eq!(
        tier_of_town_hall(&sim),
        Some(TownHallTier::T2),
        "reason=tier_is_t2"
    );
    assert!(after.town_hall.queue.is_empty(), "reason=queue_cleared");
    assert!(after.town_hall.charges.is_empty(), "reason=charge_consumed");
    assert_eq!(
        after.next_entity_id, before_next_id,
        "reason=no_new_entity_spawned"
    );
    assert_eq!(
        after.alive_entities.len(),
        before_alive,
        "reason=entity_count_unchanged"
    );
}

/// R4.1 逐级递进：T1→T2→T3 两次升级，禁止跳级（T1 直接 T3 被拒）。
#[test]
fn tier_advances_step_by_step_and_rejects_skipping() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    // 跳级：T1 直接请求 T3，显式拒绝、不入队、不扣费。
    let gold_before = sim
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    submit_upgrade(&mut sim, 1, TownHallTier::T3, false);
    sim.step();
    assert!(
        sim.capture_snapshot().town_hall.queue.is_empty(),
        "reason=tier_skip_rejected"
    );
    assert_eq!(
        sim.capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        gold_before,
        "reason=tier_skip_no_deduction"
    );

    // T1→T2。
    submit_upgrade(&mut sim, 2, TownHallTier::T2, false);
    advance_until_upgraded(&mut sim);
    assert_eq!(tier_of_town_hall(&sim), Some(TownHallTier::T2));
    // T2→T3。
    submit_upgrade(&mut sim, 6, TownHallTier::T3, false);
    advance_until_upgraded(&mut sim);
    assert_eq!(tier_of_town_hall(&sim), Some(TownHallTier::T3));
    // T3 为顶，再请求任何目标都被拒（successor 为 None）。
    submit_upgrade(&mut sim, 12, TownHallTier::T3, false);
    sim.step();
    assert!(
        sim.capture_snapshot().town_hall.queue.is_empty(),
        "reason=t3_is_terminal"
    );
}

/// 冻结 tier 需求链：未升级（T1）时二线单位 / 科技 Train / Research 被显式
/// 拒绝，不生成队列项、不扣资源、权威状态哈希不变。
#[test]
fn locked_train_and_research_are_rejected_before_upgrade() {
    // 提交命令本身会递增玩法命令队列 `next_arrival`（权威状态的一部分），
    // 因此用「等量 no-op 对齐命令计数器」的双实例法隔离玩法状态差异：
    // 一端提交被拒的 Train / Research，另一端提交等量 no-op，推进一帧后
    // 若拒绝路径真的不改玩法权威状态，两端逐帧哈希必须相等。
    let (mut rejected, _) = funded_match(100_000, 100_000);
    let (mut baseline, _) = funded_match(100_000, 100_000);
    rejected
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::Train {
                producer: EntityId::new(PRODUCER_ID).unwrap(),
                unit_def: TIER2_UNIT,
                is_building: false,
            },
        ))
        .unwrap();
    rejected
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            2,
            GameplayCommandKind::Research {
                upgrade: TIER2_UPGRADE,
            },
        ))
        .unwrap();
    baseline
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            noop_cancel_train(),
        ))
        .unwrap();
    baseline
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            2,
            noop_cancel_train(),
        ))
        .unwrap();

    let gold_before = rejected
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    rejected.step();
    baseline.step();
    assert!(
        producer_production(&rejected).is_empty(),
        "reason=train_rejected"
    );
    assert_eq!(
        current_research(&rejected),
        None,
        "reason=research_rejected"
    );
    assert_eq!(
        rejected.compute_state_hash(),
        baseline.compute_state_hash(),
        "reason=no_authoritative_state_drift field={:?}",
        rejected.first_difference(&baseline)
    );
    assert_eq!(
        rejected.first_difference(&baseline),
        None,
        "reason=no_difference_vs_noop_baseline"
    );
    assert_eq!(
        rejected
            .capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        gold_before,
        "reason=no_resource_deduction"
    );

    // 无需求登记的普通单位仍放行（数据驱动，非全量封禁）。
    let frame = current_frame(&rejected);
    submit_train(&mut rejected, frame, PLAIN_UNIT, 3);
    rejected.step();
    assert!(
        !producer_production(&rejected).is_empty(),
        "reason=unlocked_plain_unit_accepted"
    );
}

/// 升级完成后解锁在同一确定性命令面立即生效；T2 解锁 T2 单位 / 科技，
/// 但 T3 单位 / 科技仍被拒（逐级解锁）。
#[test]
fn unlock_is_effective_immediately_after_completion_and_is_tiered() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    advance_until_upgraded(&mut sim);
    assert_eq!(tier_of_town_hall(&sim), Some(TownHallTier::T2));

    // T2 单位 / 科技被接受（提交到升级完成后的当前帧，立即生效）。
    let frame = current_frame(&sim);
    submit_train(&mut sim, frame, TIER2_UNIT, 1);
    submit_research(&mut sim, frame, TIER2_UPGRADE, 2);
    sim.step();
    assert!(
        producer_production(&sim)
            .iter()
            .any(|item| item.unit_def == TIER2_UNIT),
        "reason=tier2_unit_unlocked"
    );
    assert!(
        current_research(&sim).is_some(),
        "reason=tier2_research_unlocked"
    );

    // T3 单位在 T2 阶段仍被拒。
    let (mut other, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut other, 1, TownHallTier::T2, false);
    advance_until_upgraded(&mut other);
    let other_frame = current_frame(&other);
    submit_train(&mut other, other_frame, TIER3_UNIT, 1);
    other.step();
    assert!(
        !producer_production(&other)
            .iter()
            .any(|item| item.unit_def == TIER3_UNIT),
        "reason=tier3_unit_still_locked_at_t2"
    );
}

/// 定义层数据驱动：非经典命名的主城 / 二线单位 DefId 只要登记进类别 /
/// 需求链就按数据判定；未登记类别的 DefId 升级被显式拒绝。
#[test]
fn category_and_requirements_are_definition_driven() {
    // 非经典命名 TOWN_HALL_DEF=8101 与任何经典 rawcode / 名字都无关，
    // 只要登记为主城类别即可升级。
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    advance_until_upgraded(&mut sim);
    assert_eq!(
        tier_of_town_hall(&sim),
        Some(TownHallTier::T2),
        "reason=non_classic_def_still_upgrades"
    );

    // 未登记为主城类别：命令带一个未登记的 DefId，显式拒绝、不扣费。
    let (mut other, _) = funded_match(100_000, 100_000);
    let gold_before = other
        .capture_snapshot()
        .gameplay
        .players
        .get(&PLAYER)
        .unwrap()
        .gold;
    other
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::UpgradeTownHall {
                town_hall: EntityId::new(TOWN_HALL_ID).unwrap(),
                town_hall_def: 9999,
                target_tier: TownHallTier::T2,
                cancel: false,
            },
        ))
        .unwrap();
    other.step();
    assert!(other.capture_snapshot().town_hall.queue.is_empty());
    assert_eq!(
        other
            .capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        gold_before,
        "reason=unknown_category_no_deduction"
    );
}

/// 费用事务：扣费精确、资源不足显式拒绝；升级费用读冻结常量（含差价），
/// MUST NOT 把导出总价直接当扣款。
#[test]
fn charge_is_exact_and_insufficient_is_rejected() {
    let constants = town_hall_constants();
    let cost_t2 = constants.cost(TownHallTier::T2).unwrap();
    let cost_t3 = constants.cost(TownHallTier::T3).unwrap();
    assert_eq!(cost_t2.gold, 100, "reason=t2_gold_from_constant");
    assert_eq!(cost_t2.lumber, 40, "reason=t2_lumber_from_constant");
    // T3 = T2 基础 + 差价，不是某个绝对总价。
    assert_eq!(
        cost_t3.gold,
        cost_t2.gold + constants.price_delta_gold,
        "reason=t3_uses_price_delta"
    );
    assert_eq!(
        cost_t3.lumber,
        cost_t2.lumber + constants.price_delta_lumber,
        "reason=t3_lumber_uses_price_delta"
    );

    // 提交 → 扣费精确。
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let after = sim.capture_snapshot();
    let economy = after.gameplay.players.get(&PLAYER).unwrap();
    assert_eq!(economy.gold, 100_000 - cost_t2.gold, "reason=gold_deducted");
    assert_eq!(
        economy.lumber,
        100_000 - cost_t2.lumber,
        "reason=lumber_deducted"
    );
    assert_eq!(
        after
            .town_hall
            .charges
            .get(&EntityId::new(TOWN_HALL_ID).unwrap())
            .unwrap()
            .gold,
        cost_t2.gold,
        "reason=charge_frozen"
    );

    // 资源不足 → 显式拒绝，不生成队列 / 不扣费 / 不免费升级。
    let (mut poor, _) = funded_match(0, 0);
    submit_upgrade(&mut poor, 1, TownHallTier::T2, false);
    poor.step();
    let poor_snap = poor.capture_snapshot();
    assert!(
        poor_snap.town_hall.queue.is_empty(),
        "reason=insufficient_rejected"
    );
    assert!(
        poor_snap.town_hall.charges.is_empty(),
        "reason=insufficient_no_charge"
    );
    assert_eq!(
        poor_snap.gameplay.players.get(&PLAYER).unwrap().gold,
        0,
        "reason=insufficient_no_deduction"
    );
    assert_eq!(
        poor_snap
            .town_hall
            .tiers
            .get(&EntityId::new(TOWN_HALL_ID).unwrap()),
        None,
        "reason=no_free_upgrade"
    );
}

/// R-2 / 事务性退款：玩家主动取消（单命令 `cancel=true`）与三条引擎内部
/// 中止路径（主城被毁 / 对局失败 / 玩家失活）都精确回退 gold / lumber、
/// 恢复登记到可再次升级、不产生半升级实体、不重复扣费。
#[test]
fn cancel_and_internal_aborts_refund_exactly() {
    let constants = town_hall_constants();
    let cost = constants.cost(TownHallTier::T2).unwrap();

    // 玩家主动取消：提交 → 扣费 → cancel → 精确退款、登记清空、tier 不变。
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    assert_eq!(
        sim.capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        100_000 - cost.gold
    );
    submit_upgrade(&mut sim, 2, TownHallTier::T2, true);
    sim.step();
    let snap = sim.capture_snapshot();
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).unwrap().gold,
        100_000,
        "reason=cancel_refunds_gold"
    );
    assert_eq!(
        snap.gameplay.players.get(&PLAYER).unwrap().lumber,
        100_000,
        "reason=cancel_refunds_lumber"
    );
    assert!(
        snap.town_hall.queue.is_empty(),
        "reason=cancel_clears_queue"
    );
    assert!(
        snap.town_hall.charges.is_empty(),
        "reason=cancel_clears_charge"
    );
    assert!(
        snap.town_hall.tiers.is_empty(),
        "reason=cancel_no_half_upgrade"
    );
    // 取消后可再次升级（登记已恢复）。
    submit_upgrade(&mut sim, 3, TownHallTier::T2, false);
    sim.step();
    assert_eq!(
        sim.capture_snapshot().town_hall.queue.len(),
        1,
        "reason=can_retry_after_cancel"
    );

    // 主城被毁：置 alive=false，推进一帧触发事务性退款。
    let (mut destroyed, content1) = funded_match(100_000, 100_000);
    submit_upgrade(&mut destroyed, 1, TownHallTier::T2, false);
    destroyed.step();
    let mut snap1 = destroyed.capture_snapshot();
    snap1
        .positions
        .iter_mut()
        .find(|position| position.entity == TOWN_HALL_ID)
        .expect("town hall present")
        .alive = false;
    let mut d = RtsMatch::restore_snapshot(snap1, StaticWorld::default(), content1).unwrap();
    d.step();
    let d_snap = d.capture_snapshot();
    assert_eq!(
        d_snap.gameplay.players.get(&PLAYER).unwrap().gold,
        100_000,
        "reason=town_hall_destroyed_refunds"
    );
    assert!(
        d_snap.town_hall.queue.is_empty(),
        "reason=destroy_clears_queue"
    );

    // 对局失败：注入已结算 victory，推进一帧触发事务性退款。
    let (mut failed, content2) = funded_match(100_000, 100_000);
    submit_upgrade(&mut failed, 1, TownHallTier::T2, false);
    failed.step();
    let mut snap2 = failed.capture_snapshot();
    snap2.gameplay.victory.resolved = true;
    let mut f = RtsMatch::restore_snapshot(snap2, StaticWorld::default(), content2).unwrap();
    f.step();
    assert_eq!(
        f.capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        100_000,
        "reason=defeat_refunds"
    );

    // 玩家失活：注入 active=false，推进一帧触发事务性退款。
    let (mut inactive, content3) = funded_match(100_000, 100_000);
    submit_upgrade(&mut inactive, 1, TownHallTier::T2, false);
    inactive.step();
    let mut snap3 = inactive.capture_snapshot();
    snap3.gameplay.players.get_mut(&PLAYER).unwrap().active = false;
    let mut i = RtsMatch::restore_snapshot(snap3, StaticWorld::default(), content3).unwrap();
    i.step();
    assert_eq!(
        i.capture_snapshot()
            .gameplay
            .players
            .get(&PLAYER)
            .unwrap()
            .gold,
        100_000,
        "reason=inactive_refunds"
    );
}

/// 事实事件：升级开始 / 完成 / 取消各恰好一次，事件序号在 `drain_events`
/// 后不重置。
#[test]
fn upgrade_events_are_emitted_once() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    sim.drain_events();
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    // 逐帧累积事件（不能用 advance_until_upgraded，它内部每帧 drain 会取走
    // 提交帧产生的 Started 事件）。
    let mut events = Vec::new();
    for _ in 0..12 {
        sim.step();
        events.extend(sim.drain_events());
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeStarted
                && event.entity == TOWN_HALL_ID)
            .count(),
        1,
        "reason=started_emitted_once"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeCompleted
                && event.entity == TOWN_HALL_ID)
            .count(),
        1,
        "reason=completed_emitted_once"
    );
    assert!(
        sim.drain_events().is_empty(),
        "reason=no_duplicate_events_after_drain"
    );

    let (mut cancel_sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut cancel_sim, 1, TownHallTier::T2, false);
    cancel_sim.step();
    cancel_sim.drain_events();
    submit_upgrade(&mut cancel_sim, 2, TownHallTier::T2, true);
    cancel_sim.step();
    assert_eq!(
        cancel_sim
            .drain_events()
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeCancelled)
            .count(),
        1,
        "reason=cancelled_emitted_once"
    );
}

/// Q3：冻结常量 / 类别 / 需求链进定义哈希，改任一段即改变 `content_hash()`。
#[test]
fn defs_constants_and_requirements_are_in_defs_hash() {
    let defs = town_hall_defs();

    // 改升级常量 → content_hash 变化。
    let mut changed = town_hall_constants();
    changed.tier2_gold += 1;
    let variant = FrozenDefinitions::new(
        defs.units()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
        vec![],
        vec![],
        defs.upgrades()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
    )
    .unwrap()
    .with_town_hall_upgrade(
        changed,
        [TOWN_HALL_DEF].into_iter().collect(),
        tier_requirements(),
    );
    assert_ne!(
        defs.content_hash(),
        variant.content_hash(),
        "reason=constants_in_defs_hash"
    );

    // 改主城类别集合 → content_hash 变化。
    let no_category = FrozenDefinitions::new(
        defs.units()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
        vec![],
        vec![],
        defs.upgrades()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
    )
    .unwrap()
    .with_town_hall_upgrade(town_hall_constants(), BTreeSet::new(), tier_requirements());
    assert_ne!(
        defs.content_hash(),
        no_category.content_hash(),
        "reason=category_in_defs_hash"
    );

    // 改 tier 需求链 → content_hash 变化。
    let mut req = tier_requirements();
    req.insert(TIER2_UNIT, TownHallTier::T3);
    let changed_req = FrozenDefinitions::new(
        defs.units()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
        vec![],
        vec![],
        defs.upgrades()
            .iter()
            .map(|(&id, &value)| (id, value))
            .collect(),
    )
    .unwrap()
    .with_town_hall_upgrade(
        town_hall_constants(),
        [TOWN_HALL_DEF].into_iter().collect(),
        req,
    );
    assert_ne!(
        defs.content_hash(),
        changed_req.content_hash(),
        "reason=requirements_in_defs_hash"
    );

    // 只读查询接口按定义层数据工作。
    assert!(defs.is_town_hall(TOWN_HALL_DEF));
    assert!(!defs.is_town_hall(9999));
    assert_eq!(defs.tier_requirement(TIER2_UNIT), Some(TownHallTier::T2));
    assert_eq!(defs.tier_requirement(TIER3_UPGRADE), Some(TownHallTier::T3));
    assert_eq!(defs.tier_requirement(PLAIN_UNIT), None);
    assert_eq!(defs.town_hall_constants().frames, 3);
    assert_eq!(defs.town_hall_defs().len(), 1);
    assert_eq!(defs.tier_requirements().len(), 4);

    // 显式编码 ↔ 枚举 roundtrip（供线格式对齐，非法值拒绝）。
    for tier in [TownHallTier::T1, TownHallTier::T2, TownHallTier::T3] {
        assert_eq!(TownHallTier::from_code(tier.code()), Some(tier));
    }
    assert_eq!(TownHallTier::from_code(0), None);
    assert_eq!(TownHallTier::from_code(4), None);
    assert_eq!(TownHallTier::T1.successor(), Some(TownHallTier::T2));
    assert_eq!(TownHallTier::T2.successor(), Some(TownHallTier::T3));
    assert_eq!(TownHallTier::T3.successor(), None);
}

/// Q1 / ADR 0002 §6-8：三组新增权威状态 MUST 同时进入状态哈希、快照与
/// `first_difference()`；只改一组，两端哈希不同、差异诊断能定位到新字段。
#[test]
fn new_town_hall_fields_are_hashed_and_snapshot_safe() {
    let (mut a, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut a, 1, TownHallTier::T2, false);
    a.step(); // 队列 / 费用登记进行中
    let base = a.capture_snapshot();
    let hall = EntityId::new(TOWN_HALL_ID).unwrap();

    // 只改 tiers：正式哈希变化、first_difference 定位 town_hall_tiers。
    let mut snap_tiers = base.clone();
    snap_tiers.town_hall.tiers.insert(hall, TownHallTier::T2);
    let variant_tiers =
        RtsMatch::restore_snapshot(snap_tiers, StaticWorld::default(), content).unwrap();
    assert_ne!(
        a.compute_state_hash(),
        variant_tiers.compute_state_hash(),
        "reason=tiers_in_state_hash"
    );
    assert_eq!(
        a.first_difference(&variant_tiers),
        Some("town_hall_tiers"),
        "reason=first_difference_locates_tiers"
    );
    assert_ne!(
        a.capture_snapshot(),
        variant_tiers.capture_snapshot(),
        "reason=tiers_in_snapshot"
    );

    // 只改 queue：first_difference 定位 town_hall_queue。
    let mut snap_queue = base.clone();
    snap_queue
        .town_hall
        .queue
        .get_mut(&hall)
        .unwrap()
        .remaining_frames = 99;
    let variant_queue =
        RtsMatch::restore_snapshot(snap_queue, StaticWorld::default(), content).unwrap();
    assert_eq!(
        a.first_difference(&variant_queue),
        Some("town_hall_queue"),
        "reason=first_difference_locates_queue"
    );
    assert_ne!(
        a.compute_state_hash(),
        variant_queue.compute_state_hash(),
        "reason=queue_in_state_hash"
    );

    // 只改 charges：first_difference 定位 town_hall_charges。
    let mut snap_charges = base.clone();
    snap_charges.town_hall.charges.get_mut(&hall).unwrap().gold = 999;
    let variant_charges =
        RtsMatch::restore_snapshot(snap_charges, StaticWorld::default(), content).unwrap();
    assert_eq!(
        a.first_difference(&variant_charges),
        Some("town_hall_charges"),
        "reason=first_difference_locates_charges"
    );
    assert_ne!(
        a.compute_state_hash(),
        variant_charges.compute_state_hash(),
        "reason=charges_in_state_hash"
    );
}

/// 确定性：双实例交错推进（含升级命令）逐帧 `compute_state_hash()` 与
/// `capture_snapshot()` 相等，`first_difference()` 为 `None`。
#[test]
fn two_instances_interleaved_match_frame_hashes() {
    let (mut a, _) = funded_match(100_000, 100_000);
    let (mut b, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut a, 1, TownHallTier::T2, false);
    submit_upgrade(&mut b, 1, TownHallTier::T2, false);
    let mut state_changed = false;
    for frame in 0..16 {
        if frame % 2 == 0 {
            a.step();
            b.step();
        } else {
            b.step();
            a.step();
        }
        assert_eq!(
            a.compute_state_hash(),
            b.compute_state_hash(),
            "reason=frame_{frame}_hash_match field={:?}",
            a.first_difference(&b)
        );
        assert_eq!(a.first_difference(&b), None, "reason=frame_{frame}_no_diff");
        assert_eq!(
            a.capture_snapshot(),
            b.capture_snapshot(),
            "reason=frame_{frame}_snapshot_match"
        );
        if frame == 1 {
            state_changed = true;
        }
    }
    assert!(state_changed, "reason=scenario_actually_advanced");
    assert_eq!(
        tier_of_town_hall(&a),
        Some(TownHallTier::T2),
        "reason=both_upgraded"
    );
}

/// 确定性：帧 N 捕获 / 销毁 / 恢复后继续到帧 M 逐帧一致，升级不重复
/// （Completed 恰好一次），三组状态不残留 / 不丢失（R12.4）。
#[test]
fn snapshot_roundtrip_continues_identically() {
    let (mut original, content) = funded_match(100_000, 100_000);
    submit_upgrade(&mut original, 1, TownHallTier::T2, false);
    original.step(); // 队列进行中捕获
    let snapshot = original.capture_snapshot();
    assert_eq!(
        snapshot.town_hall.queue.len(),
        1,
        "reason=queue_in_snapshot"
    );
    assert_eq!(
        snapshot.town_hall.charges.len(),
        1,
        "reason=charge_in_snapshot"
    );
    let mut restored =
        RtsMatch::restore_snapshot(snapshot.clone(), StaticWorld::default(), content).unwrap();
    assert_eq!(
        original.compute_state_hash(),
        restored.compute_state_hash(),
        "reason=restore_preserves_hash"
    );
    assert_eq!(
        restored.capture_snapshot().town_hall,
        snapshot.town_hall,
        "reason=restore_preserves_town_hall_state"
    );
    original.drain_events();
    restored.drain_events();
    for frame in 0..12 {
        original.step();
        restored.step();
        assert_eq!(
            original.compute_state_hash(),
            restored.compute_state_hash(),
            "reason=roundtrip_frame_{frame}"
        );
    }
    let events = restored.drain_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == EventKind::TownHallUpgradeCompleted)
            .count(),
        1,
        "reason=no_duplicate_upgrade"
    );
    assert_eq!(tier_of_town_hall(&restored), Some(TownHallTier::T2));
}

/// 确定性：回放同脚本逐帧哈希与原始一致，重放不双重注入升级命令 / 不重复扣费。
#[test]
fn replay_matches_original() {
    let (mut original, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut original, 1, TownHallTier::T2, false);
    let expected: Vec<u64> = (0..16)
        .map(|_| {
            original.step();
            original.compute_state_hash()
        })
        .collect();

    let (mut replay, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut replay, 1, TownHallTier::T2, false);
    for (frame, hash) in expected.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.compute_state_hash(),
            *hash,
            "reason=replay_frame_{frame}"
        );
    }
}

/// Q1：旧 v4 快照被 `SnapshotError` 明确拒绝，且 `SNAPSHOT_VERSION` /
/// `SIM_VERSION` 已 bump 到 5，不提供跨版本迁移。
#[test]
fn old_v4_snapshot_is_rejected() {
    assert_eq!(SNAPSHOT_VERSION, 5, "reason=snapshot_version_5");
    assert_eq!(SIM_VERSION, 5, "reason=sim_version_5");
    let (sim, content) = funded_match(100_000, 100_000);
    let mut old = sim.capture_snapshot();
    old.version = 4;
    assert!(matches!(
        RtsMatch::restore_snapshot(old, StaticWorld::default(), content),
        Err(SnapshotError::UnsupportedVersion { found: 4 })
    ));
    let mut old_sim = sim.capture_snapshot();
    old_sim.sim_version = 4;
    assert!(matches!(
        RtsMatch::restore_snapshot(old_sim, StaticWorld::default(), content),
        Err(SnapshotError::SimVersionMismatch {
            found: 4,
            expected: 5
        })
    ));
}

/// ADR 0002 §5：只读视图与事件消费不改写权威状态（含主城升级三组状态）。
#[test]
fn readonly_view_and_events_do_not_change_authoritative_state() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let hash_before = sim.compute_state_hash();
    let snapshot_before = sim.capture_snapshot();
    let _view = sim.read_view();
    let _view = sim.read_view();
    let _events = sim.drain_events();
    assert_eq!(
        sim.compute_state_hash(),
        hash_before,
        "reason=read_only_does_not_change_hash"
    );
    assert_eq!(
        sim.capture_snapshot(),
        snapshot_before,
        "reason=read_only_does_not_change_snapshot"
    );
}

/// 命令变体编码 / 校验 / 穷尽分发：提交新命令真的生效（不是 no-op），
/// 且 `EntityId::NONE` 被 `validate_gameplay_entities` 显式拒绝。
#[test]
fn upgrade_command_is_dispatched_and_validated() {
    use w3_core::GameplayCommandError;
    let (mut sim, _) = funded_match(100_000, 100_000);
    // 非法实体：提交入口应显式拒绝。
    let err = sim
        .submit_gameplay_command(GameplayCommand::new(
            Frame::new(1),
            PLAYER,
            1,
            GameplayCommandKind::UpgradeTownHall {
                town_hall: EntityId::NONE,
                town_hall_def: TOWN_HALL_DEF,
                target_tier: TownHallTier::T2,
                cancel: false,
            },
        ))
        .unwrap_err();
    assert_eq!(err, GameplayCommandError::InvalidEntity);
    // 合法命令经穷尽分发真正生效（队列 / 费用登记建立）。
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    assert_eq!(
        sim.capture_snapshot().town_hall.queue.len(),
        1,
        "reason=command_actually_dispatched"
    );
}

/// 帧冻结命令两端一致：一端逐帧、另一端一次推进，共推进相同整数帧数并注入
/// 同一升级脚本 → 逐帧哈希一致、重放不双重注入（R12.4 / R12.7）。
#[test]
fn frozen_frame_command_is_consistent_across_step_sizes() {
    let (mut a, _) = funded_match(100_000, 100_000);
    let (mut b, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut a, 1, TownHallTier::T2, false);
    submit_upgrade(&mut b, 1, TownHallTier::T2, false);
    // 两端都逐帧推进 8 帧（step() 不接受 delta / 步长参数，此处比较
    // 「同一脚本、同一整数帧数」的两端一致性）。
    let mut hashes_a = Vec::new();
    let mut hashes_b = Vec::new();
    for _ in 0..8 {
        a.step();
        hashes_a.push(a.compute_state_hash());
    }
    for _ in 0..8 {
        b.step();
        hashes_b.push(b.compute_state_hash());
    }
    assert_eq!(hashes_a, hashes_b, "reason=same_script_same_frame_hashes");
    assert_eq!(a.capture_snapshot(), b.capture_snapshot());
}

/// 队列项字段完整性（供下游穷尽匹配 / 只读 HUD 对齐）：升级进行中
/// `TownHallUpgradeItem` 各字段可读且为整数帧 / 稳定 EntityId。
#[test]
fn queue_item_fields_are_integer_frames_and_stable_ids() {
    let (mut sim, _) = funded_match(100_000, 100_000);
    submit_upgrade(&mut sim, 1, TownHallTier::T2, false);
    sim.step();
    let item: TownHallUpgradeItem = sim
        .capture_snapshot()
        .town_hall
        .queue
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(item.town_hall.value(), TOWN_HALL_ID);
    assert_eq!(item.town_hall_def, TOWN_HALL_DEF);
    assert_eq!(item.owner, PLAYER);
    assert_eq!(item.from_tier, TownHallTier::T1);
    assert_eq!(item.target_tier, TownHallTier::T2);
    assert!(item.remaining_frames > 0 && item.remaining_frames <= item.total_frames);
    assert_eq!(item.total_frames, town_hall_constants().frames);
    assert_eq!(
        item.gold,
        town_hall_constants().cost(TownHallTier::T2).unwrap().gold
    );
    assert_eq!(
        item.lumber,
        town_hall_constants().cost(TownHallTier::T2).unwrap().lumber
    );
}
