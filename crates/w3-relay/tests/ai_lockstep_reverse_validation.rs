//! `p10-ai-player-lockstep` 批次 2：硬门禁 5.2 反向验证（形态①）。
//!
//! 本文件只补齐批次 1 正向回归的「缺陷敏感性」证据，不改
//! `ai_lockstep_match.rs` 的行为，也不改 `Leave` 处理路径。每个测试都在
//! 测试内构造「退化输入 → 公开生产路径 → 元断言」：先用
//! `apply_frame()` / `submit_gameplay_command()` / `step()` /
//! `capture_snapshot()` / `restore_snapshot()` / `compute_state_hash()` /
//! `first_difference()` / `resync_replay_packet()` / `parse_resync_replay()`
//! 等真实入口制造反例，再证明正向测试的等价谓词在正确态为真、在退化态为假。
//! 所有元断言都含「正式实现仍通过同一断言」的防退化分支，不是形态②的
//! 两条命令输出留档，也不是刻意改坏被测代码验红留档；更没有旁路模拟或
//! 手写假哈希结论。
//!
//! 覆盖 tasks 5.2 中可由 `w3-relay` 公开 API 构造的退化态：
//! ① AI 命令本地静默注入不经帧流；② 一端不启用 `run_ai`；
//! ③ `AiState` 四处同步敏感性；④ `run_ai` 内部命令写回命令队列 / 注入重放；
//! ⑦ 联机规则不一致 / 未冻结开局；以及「胜负非 `Leave` 捷径」专项。
//!
//! 不另造缺陷态的清单项：⑤ 哈希映射迭代顺序 / 墙钟 / 双精度浮点 /
//! 独立随机流属于内核长期性质，`w3-relay` 没有可经公开 API 注入的非确定源，
//! 硬造只会变成旁路模拟；该项由既有
//! `cargo test -p w3-core --test determinism`（26 项）与 `design.md` D7
//! 边界登记承接。⑥ 含建筑误生成 `Move` 属于 `w3-client` 输入翻译，
//! `w3-relay` 集成测试既不能依赖该 crate，也不能越过表现层入口伪造输入；
//! 该项由既有 R1 输入翻译回归与 `design.md` D7 边界登记承接。
//!
//! 真窗口人工清单不在本文件触碰；本文件也不改四件套正文、纪要、版本号、
//! 冻结常量、`w3-core` 生产逻辑或 `w3-net` / `w3-relay` 生产代码。

use w3_core::{
    Command, CommandKind, EntityId, Frame, GameplayCommand, GameplayCommandKind, MatchConfig,
    RtsMatch, SimFixed, Snapshot, StateHasher, StaticWorld, VictoryState,
};
use w3_net::packet::{FrameCommands, encode_commands, parse_resync_replay};
use w3_net::{apply_frame, resync_replay_packet};

/// 真人席位编号，与批次 1 正向夹具保持一致。
const LOCAL_PLAYER: u8 = 1;
/// AI 席位编号，与批次 1 正向夹具保持一致。
const AI_PLAYER: u8 = 2;
/// 冻结开局使用的整数随机种子。
const SEED: u64 = 0xA1_10;
/// 合成 / 原创内容身份，供 `restore_snapshot()` 显式校验。
const CONTENT_HASH: u64 = 0xC0_FF_EE;
/// 真人攻击并摧毁 AI 全部建筑的整数帧。
const ATTACK_FRAME: u64 = 9;
/// 既有 `Leave` 结算命令面所在整数帧。
const LEAVE_FRAME: u64 = 10;
/// `SetAi` 的首次决策周期帧，与内核既有整数周期一致。
const AI_DECISION_FRAME: u64 = 10;

/// 把整数字面量冻结为 Q47.16 定点值，显式处理转换失败。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("ai_lockstep_reverse: reason=fixed_conversion_failed")
}

/// 构造本文件专用的冻结世界；`restore_snapshot()` 必须传入同一身份。
fn world() -> StaticWorld {
    StaticWorld::open_world(16, 16, fixed(8))
}

/// 由冻结种子 / 世界 / 内容身份构造对局；不同 seed 即不同冻结开局定义。
fn new_match(seed: u64) -> RtsMatch {
    RtsMatch::create_with_world(
        MatchConfig {
            seed,
            logic_fps: 30,
            content_hash: CONTENT_HASH,
        },
        world(),
    )
}

/// 经公开快照恢复入口重建对局。
fn restored(snapshot: Snapshot) -> RtsMatch {
    RtsMatch::restore_snapshot(snapshot, world(), CONTENT_HASH)
        .expect("ai_lockstep_reverse: reason=restore_snapshot_failed")
}

/// 构造一条合法 `Spawn` 命令；坐标、速度、伤害、射程均为合成整数常量。
fn spawn_command(
    frame: u64,
    player: u8,
    sequence: u32,
    x: i64,
    y: i64,
    hp: i64,
    speed: i64,
    damage: i64,
    range: i64,
) -> Command {
    Command::new(
        Frame::new(frame),
        player,
        sequence,
        CommandKind::Spawn {
            x: fixed(x),
            y: fixed(y),
            hp,
            speed: fixed(speed),
            attack_damage: damage,
            attack_range: fixed(range),
        },
    )
}

/// 冻结开局定义：真人主城 / 农民与两座 AI 席位建筑。
struct Opening {
    commands: Vec<Command>,
    gameplay: Vec<GameplayCommand>,
    local_hall: EntityId,
    local_worker: EntityId,
    ai_buildings: Vec<EntityId>,
}

/// 按稳定顺序登记实体编号；主城带足够伤害 / 射程，供建筑全毁夹具使用。
fn opening() -> Opening {
    let local_hall = EntityId::new(1).expect("ai_lockstep_reverse: reason=invalid_local_hall_id");
    let local_worker =
        EntityId::new(2).expect("ai_lockstep_reverse: reason=invalid_local_worker_id");
    let ai_hall = EntityId::new(3).expect("ai_lockstep_reverse: reason=invalid_ai_hall_id");
    let ai_outpost = EntityId::new(4).expect("ai_lockstep_reverse: reason=invalid_ai_outpost_id");

    let commands = vec![
        spawn_command(0, LOCAL_PLAYER, 1, 2, 2, 100, 1, 80, 20),
        spawn_command(0, LOCAL_PLAYER, 2, 3, 2, 60, 3, 0, 0),
        // 帧 0 的 AI 席位 `Spawn` 是冻结开局定义，不是 AI 决策命令。
        spawn_command(0, AI_PLAYER, 1, 4, 2, 30, 1, 0, 0),
        spawn_command(0, AI_PLAYER, 2, 4, 3, 30, 1, 0, 0),
    ];
    let gameplay = vec![
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            10,
            GameplayCommandKind::SetAi { enabled: true },
        ),
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            11,
            GameplayCommandKind::Gather { unit: local_worker },
        ),
    ];

    Opening {
        commands,
        gameplay,
        local_hall,
        local_worker,
        ai_buildings: vec![ai_hall, ai_outpost],
    }
}

/// 帧 0 的玩法命令批，可切换 `SetAi` 的二值状态而保持序列 / 条数一致。
fn gameplay_with_set_ai(opening: &Opening, enabled: bool) -> Vec<GameplayCommand> {
    vec![
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            10,
            GameplayCommandKind::SetAi { enabled },
        ),
        GameplayCommand::new(
            Frame::new(0),
            LOCAL_PLAYER,
            11,
            GameplayCommandKind::Gather {
                unit: opening.local_worker,
            },
        ),
    ]
}

/// 经 `apply_frame()` 应用开局；这是联机正向测试使用的唯一推进点。
fn apply_opening(sim: &mut RtsMatch, opening: &Opening, gameplay: Vec<GameplayCommand>) -> u64 {
    apply_frame(sim, &opening.commands, &gameplay)
        .expect("ai_lockstep_reverse: reason=apply_opening_failed")
}

/// 经 `apply_frame()` 推进一个空帧，保持「恰好一次 `step()`」语义。
fn step_frame(sim: &mut RtsMatch) -> u64 {
    apply_frame(sim, &[], &[]).expect("ai_lockstep_reverse: reason=empty_apply_frame_failed")
}

/// 由一个权威帧批构造 `FrameCommands`，便于 canonical 字节比对与回放。
fn frame_commands(
    frame: u64,
    commands: Vec<Command>,
    gameplay: Vec<GameplayCommand>,
) -> FrameCommands {
    FrameCommands {
        frame,
        commands,
        gameplay,
    }
}

/// 正式端点不变量：逐帧状态哈希 / 快照 / 首个差异诊断均一致。
fn positive_endpoint_invariants(a: &RtsMatch, b: &RtsMatch) -> bool {
    a.compute_state_hash() == b.compute_state_hash()
        && a.capture_snapshot() == b.capture_snapshot()
        && a.first_difference(b).is_none()
}

/// 正式断言链在正确态绿、在退化态红；元断言防止谓词退化成恒真。
fn assert_formal_passes_and_degenerate_fails(formal: bool, degenerate: bool, case: &str) {
    assert!(formal, "reason=formal_still_passes case={case}");
    assert!(
        !degenerate,
        "reason=degenerate_must_fail_same_assertion case={case}"
    );
    assert!(
        formal && !degenerate,
        "reason=meta_assertion_detects_regression case={case}"
    );
}

/// 由 canonical payload 编码得到权威帧批字节。
fn command_batch_bytes(frame: &FrameCommands) -> Vec<u8> {
    encode_commands(&frame.commands, &frame.gameplay)
        .expect("ai_lockstep_reverse: reason=encode_commands_failed")
}

/// 帧流只承载真人命令：无 AI 席位署名的玩法命令、无针对 AI 建筑的外部采集。
fn no_ai_decision_commands(frame: &FrameCommands, opening: &Opening) -> bool {
    frame.gameplay.iter().all(|command| {
        command.player_id == LOCAL_PLAYER
            && !matches!(
                command.kind,
                GameplayCommandKind::Gather { unit } if opening.ai_buildings.contains(&unit)
            )
    })
}

/// 两端 `CommandBatch` 的 canonical 字节一致且都不含 AI 决策命令。
fn positive_command_batch_invariants(
    local: &FrameCommands,
    remote: &FrameCommands,
    opening: &Opening,
) -> bool {
    command_batch_bytes(local) == command_batch_bytes(remote)
        && no_ai_decision_commands(local, opening)
        && no_ai_decision_commands(remote, opening)
}

/// 退化态①：把内核 `run_ai` 的内部 `Gather` 以 AI 席位署名直接静默注入。
fn ai_silent_gather(opening: &Opening, frame: u64) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        AI_PLAYER,
        900,
        GameplayCommandKind::Gather {
            unit: opening.ai_buildings[0],
        },
    )
}

#[test]
fn reverse_validation_silent_ai_command_injection_is_detected() {
    let opening = opening();
    let frame0 = frame_commands(0, opening.commands.clone(), opening.gameplay.clone());
    let frame1_clean = frame_commands(1, Vec::new(), Vec::new());

    // 正确态：两端都经 `apply_frame()` 应用同一权威帧批，不绕过帧流。
    let mut formal_a = new_match(SEED);
    let mut formal_b = new_match(SEED);
    apply_frame(&mut formal_a, &frame0.commands, &frame0.gameplay)
        .expect("ai_lockstep_reverse: reason=formal_frame0_failed client=1");
    apply_frame(&mut formal_b, &frame0.commands, &frame0.gameplay)
        .expect("ai_lockstep_reverse: reason=formal_frame0_failed client=2");
    apply_frame(
        &mut formal_a,
        &frame1_clean.commands,
        &frame1_clean.gameplay,
    )
    .expect("ai_lockstep_reverse: reason=formal_frame1_failed client=1");
    apply_frame(
        &mut formal_b,
        &frame1_clean.commands,
        &frame1_clean.gameplay,
    )
    .expect("ai_lockstep_reverse: reason=formal_frame1_failed client=2");
    let formal_ok = positive_command_batch_invariants(&frame1_clean, &frame1_clean, &opening)
        && positive_endpoint_invariants(&formal_a, &formal_b);

    // 退化态：仅一端用公开 `submit_gameplay_command()` + `step()` 静默注入
    // AI 席位采集，另一端仍走空帧。注入不经 `apply_frame()` 帧流。
    let mut degenerate_a = new_match(SEED);
    let mut degenerate_b = new_match(SEED);
    apply_frame(&mut degenerate_a, &frame0.commands, &frame0.gameplay)
        .expect("ai_lockstep_reverse: reason=degenerate_frame0_failed client=1");
    apply_frame(&mut degenerate_b, &frame0.commands, &frame0.gameplay)
        .expect("ai_lockstep_reverse: reason=degenerate_frame0_failed client=2");
    apply_frame(
        &mut degenerate_a,
        &frame1_clean.commands,
        &frame1_clean.gameplay,
    )
    .expect("ai_lockstep_reverse: reason=degenerate_frame1_failed client=1");
    let injected = ai_silent_gather(&opening, 1);
    degenerate_b
        .submit_gameplay_command(injected)
        .expect("ai_lockstep_reverse: reason=silent_submit_failed");
    degenerate_b.step();
    let frame1_with_local_injection = frame_commands(1, Vec::new(), vec![injected]);
    let degenerate_ok =
        positive_command_batch_invariants(&frame1_clean, &frame1_with_local_injection, &opening)
            && positive_endpoint_invariants(&degenerate_a, &degenerate_b);

    assert_formal_passes_and_degenerate_fails(
        formal_ok,
        degenerate_ok,
        "silent_ai_command_injection",
    );
    assert_eq!(
        formal_a.compute_state_hash(),
        formal_b.compute_state_hash(),
        "reason=formal_hashes_match"
    );
    assert_ne!(
        degenerate_a.compute_state_hash(),
        degenerate_b.compute_state_hash(),
        "reason=silent_injection_diverges_hash"
    );
    assert!(
        degenerate_a.first_difference(&degenerate_b).is_some(),
        "reason=silent_injection_located_by_first_difference"
    );
    assert!(
        !no_ai_decision_commands(&frame1_with_local_injection, &opening),
        "reason=ai_decision_command_is_visible_in_frame_batch"
    );
}

/// 退化态②：一端不启用 `run_ai`。
#[test]
fn reverse_validation_one_endpoint_without_run_ai_is_detected() {
    let opening = opening();
    let enabled_gameplay = gameplay_with_set_ai(&opening, true);
    let disabled_gameplay = gameplay_with_set_ai(&opening, false);

    // 正确态：两端同样经 `apply_frame()` 注入 `SetAi{enabled:true}`，并推进
    // 到首个 AI 决策帧之后，证明 `run_ai` 状态续算仍一致。
    let mut formal_a = new_match(SEED);
    let mut formal_b = new_match(SEED);
    apply_opening(&mut formal_a, &opening, enabled_gameplay.clone());
    apply_opening(&mut formal_b, &opening, enabled_gameplay);
    for _ in 0..AI_DECISION_FRAME {
        step_frame(&mut formal_a);
        step_frame(&mut formal_b);
    }
    let formal_ok = positive_endpoint_invariants(&formal_a, &formal_b);

    // 退化态：一端经帧流启用 `run_ai`，另一端经帧流显式关闭；其余开局
    // 命令、序列、推进次数完全相同，差异只来自权威 `AiState`。
    let mut degenerate_a = new_match(SEED);
    let mut degenerate_b = new_match(SEED);
    apply_opening(&mut degenerate_a, &opening, opening.gameplay.clone());
    apply_opening(&mut degenerate_b, &opening, disabled_gameplay);
    for _ in 0..AI_DECISION_FRAME {
        step_frame(&mut degenerate_a);
        step_frame(&mut degenerate_b);
    }
    let degenerate_ok = positive_endpoint_invariants(&degenerate_a, &degenerate_b);

    assert_formal_passes_and_degenerate_fails(
        formal_ok,
        degenerate_ok,
        "one_endpoint_without_run_ai",
    );
    assert_ne!(
        degenerate_a.compute_state_hash(),
        degenerate_b.compute_state_hash(),
        "reason=ai_enabled_is_hashed"
    );
    assert_ne!(
        degenerate_a.capture_snapshot().gameplay.ai,
        degenerate_b.capture_snapshot().gameplay.ai,
        "reason=ai_enabled_is_in_snapshot"
    );
    assert!(
        degenerate_a.first_difference(&degenerate_b).is_some(),
        "reason=ai_enabled_is_located_by_first_difference"
    );
    assert_eq!(
        formal_a.first_difference(&formal_b),
        None,
        "reason=formal_ai_state_stays_equal"
    );
}

/// 退化哈希：重放公开 `WorldGameplayState::hash()` 编码，但把 AI 状态清空，
/// 等价于 `AiState` 未进入 `compute_state_hash()`；注入源是真实 owned 快照。
fn degenerate_gameplay_hash_without_ai(snapshot: &Snapshot) -> u64 {
    let mut gameplay = snapshot.gameplay.clone();
    gameplay.ai = Default::default();
    let mut hasher = StateHasher::new();
    gameplay.hash(&mut hasher);
    hasher.finish()
}

/// 退化差异诊断：重放公开 `WorldGameplayState::first_difference()` 编码，
/// 但先把两端 AI 状态抹平，等价于 `AiState` 未进入差异检测；注入源仍是
/// 真实 owned 快照，不是手写假差异结论。
fn degenerate_gameplay_first_difference_without_ai(
    a: &Snapshot,
    b: &Snapshot,
) -> Option<&'static str> {
    let mut lhs = a.gameplay.clone();
    let rhs = b.gameplay.clone();
    lhs.ai = rhs.ai.clone();
    lhs.first_difference(&rhs)
}

/// 正式恢复不变量：两份同源实例经 `restore_snapshot()` 重建后逐帧续算一致。
fn restore_steps_are_consistent(source: &RtsMatch, other: &RtsMatch) -> bool {
    let mut a = restored(source.capture_snapshot());
    let mut b = restored(other.capture_snapshot());
    for _ in 0..AI_DECISION_FRAME {
        a.step();
        b.step();
        if a.compute_state_hash() != b.compute_state_hash() || a.first_difference(&b).is_some() {
            return false;
        }
    }
    true
}

/// 退化态③：`AiState` 对哈希 / 快照 / 恢复 / 差异检测四处的同步敏感性。
#[test]
fn reverse_validation_ai_state_four_sync_points_are_sensitive() {
    let opening = opening();
    let mut enabled = new_match(SEED);
    let mut disabled = new_match(SEED);
    apply_opening(&mut enabled, &opening, gameplay_with_set_ai(&opening, true));
    apply_opening(
        &mut disabled,
        &opening,
        gameplay_with_set_ai(&opening, false),
    );

    let enabled_snapshot = enabled.capture_snapshot();
    let disabled_snapshot = disabled.capture_snapshot();
    let formal_restored = restored(enabled_snapshot.clone());

    // 正式态：`compute_state_hash()`、`capture_snapshot()`、
    // `first_difference()` 都能区分仅 `ai.enabled` 不同的两实例；正式
    // `restore_snapshot()` 恢复后逐帧续算仍一致。
    let formal_hash_detects = enabled.compute_state_hash() != disabled.compute_state_hash();
    let formal_snapshot_detects = enabled_snapshot.gameplay.ai != disabled_snapshot.gameplay.ai;
    let formal_first_difference_detects = enabled.first_difference(&disabled).is_some();
    let formal_restore_detects = enabled.compute_state_hash()
        == formal_restored.compute_state_hash()
        && enabled.first_difference(&formal_restored).is_none()
        && restore_steps_are_consistent(&enabled, &formal_restored);

    // 退化态：把 owned 快照中的 AI 状态清空后，经真实
    // `restore_snapshot()` 重建；再编码与正式快照都不一致。
    let mut dropped_snapshot = enabled_snapshot.clone();
    dropped_snapshot.gameplay.ai = Default::default();
    let dropped_restored = restored(dropped_snapshot);
    let degenerate_hash_still_detects = degenerate_gameplay_hash_without_ai(&enabled_snapshot)
        != degenerate_gameplay_hash_without_ai(&disabled_snapshot);
    let degenerate_snapshot_still_detects =
        dropped_restored.capture_snapshot().gameplay.ai == enabled_snapshot.gameplay.ai;
    let degenerate_first_difference_still_detects =
        degenerate_gameplay_first_difference_without_ai(&enabled_snapshot, &disabled_snapshot)
            .is_some();
    let degenerate_restore_still_consistent =
        restore_steps_are_consistent(&enabled, &dropped_restored);
    assert!(
        !degenerate_hash_still_detects,
        "reason=degenerate_hash_is_blind_to_ai"
    );
    assert!(
        !degenerate_snapshot_still_detects,
        "reason=degenerate_snapshot_loses_ai"
    );
    assert!(
        !degenerate_first_difference_still_detects,
        "reason=degenerate_first_difference_is_blind_to_ai"
    );
    assert!(
        !degenerate_restore_still_consistent,
        "reason=degenerate_restore_diverges_after_steps"
    );
    assert_ne!(
        dropped_restored.compute_state_hash(),
        enabled.compute_state_hash(),
        "reason=degenerate_restore_diverges"
    );
    assert!(
        enabled.first_difference(&dropped_restored).is_some(),
        "reason=first_difference_locates_dropped_ai"
    );
    assert_formal_passes_and_degenerate_fails(
        formal_hash_detects && formal_snapshot_detects,
        degenerate_hash_still_detects && degenerate_snapshot_still_detects,
        "ai_state_hash_and_snapshot",
    );
    assert_formal_passes_and_degenerate_fails(
        formal_first_difference_detects && formal_restore_detects,
        degenerate_first_difference_still_detects && degenerate_restore_still_consistent,
        "ai_state_restore_and_first_difference",
    );

    // `pending_tasks` 敏感性：通过 owned 快照插入一条整数任务，再经
    // `restore_snapshot()` 注入；正式哈希 / 快照 / 差异检测必须改变。
    let mut pending_snapshot = enabled_snapshot.clone();
    pending_snapshot.gameplay.ai.pending_tasks.insert(77, 3);
    let pending_restored = restored(pending_snapshot);
    assert_ne!(
        pending_restored.compute_state_hash(),
        enabled.compute_state_hash(),
        "reason=pending_tasks_is_hashed"
    );
    assert_ne!(
        pending_restored.capture_snapshot(),
        enabled_snapshot,
        "reason=pending_tasks_is_in_snapshot"
    );
    assert_eq!(
        enabled.first_difference(&pending_restored),
        Some("ai"),
        "reason=pending_tasks_is_located_by_first_difference"
    );
    assert!(
        formal_restore_detects,
        "reason=formal_restore_still_consistent"
    );
}

/// 构造从帧 0 到帧 11 的正式权威帧批；帧 10 会触发内核 `run_ai`。
fn formal_frames(opening: &Opening) -> Vec<FrameCommands> {
    let mut frames = vec![frame_commands(
        0,
        opening.commands.clone(),
        opening.gameplay.clone(),
    )];
    for frame in 1..=11 {
        frames.push(frame_commands(frame, Vec::new(), Vec::new()));
    }
    frames
}

/// 逐帧应用权威帧批，返回原局逐帧状态哈希。
fn apply_frames(sim: &mut RtsMatch, frames: &[FrameCommands]) -> Vec<u64> {
    frames
        .iter()
        .map(|frame| {
            apply_frame(sim, &frame.commands, &frame.gameplay)
                .expect("ai_lockstep_reverse: reason=apply_frames_failed")
        })
        .collect()
}

/// 正式回放不变量：经 `ResyncReplay` 编解码后逐帧哈希与原局一致。
fn replay_matches_hashes(frames: &[FrameCommands], hashes: &[u64]) -> bool {
    let packet = resync_replay_packet(0, frames)
        .expect("ai_lockstep_reverse: reason=resync_replay_packet_failed");
    let decoded = parse_resync_replay(&packet.payload)
        .expect("ai_lockstep_reverse: reason=parse_resync_replay_failed");
    if decoded.len() != hashes.len() {
        return false;
    }
    let mut replay = new_match(SEED);
    for (index, frame) in decoded.iter().enumerate() {
        let hash = apply_frame(&mut replay, &frame.commands, &frame.gameplay)
            .expect("ai_lockstep_reverse: reason=replay_apply_frame_failed");
        if hash != hashes[index] {
            return false;
        }
    }
    true
}

/// 退化态④：把 `run_ai` 内部采集命令当作外部命令重复写入帧 11 / 回放。
#[test]
fn reverse_validation_run_ai_command_writeback_is_detected() {
    let opening = opening();
    let frames = formal_frames(&opening);

    // 正确态：两端逐帧应用同一批权威帧，帧 10 的 `run_ai` 内部采集不进入
    // 帧流；`ResyncReplay` 回放逐帧哈希与原局一致。
    let mut formal_a = new_match(SEED);
    let mut formal_b = new_match(SEED);
    let formal_hashes = apply_frames(&mut formal_a, &frames);
    let formal_b_hashes = apply_frames(&mut formal_b, &frames);
    assert_eq!(
        formal_hashes, formal_b_hashes,
        "reason=formal_endpoint_hashes_match"
    );
    let formal_endpoints_ok = positive_endpoint_invariants(&formal_a, &formal_b);
    let formal_replay_ok = replay_matches_hashes(&frames, &formal_hashes);

    // 退化态：只给一端 / 回放帧批在帧 11 注入重复的 `Gather`，模拟把
    // 帧 10 的内部 AI 命令写回命令队列或重放。注入仍经 `apply_frame()`。
    let mut degenerate_a = new_match(SEED);
    let mut degenerate_b = new_match(SEED);
    for frame in &frames[..frames.len() - 1] {
        apply_frame(&mut degenerate_a, &frame.commands, &frame.gameplay)
            .expect("ai_lockstep_reverse: reason=degenerate_prefix_failed client=1");
        apply_frame(&mut degenerate_b, &frame.commands, &frame.gameplay)
            .expect("ai_lockstep_reverse: reason=degenerate_prefix_failed client=2");
    }
    let duplicate = GameplayCommand::new(
        Frame::new(11),
        LOCAL_PLAYER,
        901,
        GameplayCommandKind::Gather {
            unit: opening.local_worker,
        },
    );
    let clean_last = frame_commands(11, Vec::new(), Vec::new());
    let duplicate_last = frame_commands(11, Vec::new(), vec![duplicate]);
    apply_frame(
        &mut degenerate_a,
        &clean_last.commands,
        &clean_last.gameplay,
    )
    .expect("ai_lockstep_reverse: reason=degenerate_last_failed client=1");
    apply_frame(
        &mut degenerate_b,
        &duplicate_last.commands,
        &duplicate_last.gameplay,
    )
    .expect("ai_lockstep_reverse: reason=degenerate_last_failed client=2");

    let mut degenerate_frames = frames.clone();
    let last = degenerate_frames
        .last_mut()
        .expect("ai_lockstep_reverse: reason=degenerate_last_frame_missing");
    last.gameplay = vec![duplicate];
    let degenerate_endpoints_ok = positive_endpoint_invariants(&degenerate_a, &degenerate_b);
    let degenerate_replay_ok = replay_matches_hashes(&degenerate_frames, &formal_hashes);

    assert_formal_passes_and_degenerate_fails(
        formal_endpoints_ok,
        degenerate_endpoints_ok,
        "run_ai_writeback_two_endpoints",
    );
    assert_formal_passes_and_degenerate_fails(
        formal_replay_ok,
        degenerate_replay_ok,
        "run_ai_writeback_replay",
    );
    assert!(
        degenerate_a.first_difference(&degenerate_b).is_some(),
        "reason=duplicate_gather_located_by_first_difference"
    );
    let gold_a = degenerate_a.capture_snapshot().gameplay.players[&LOCAL_PLAYER].gold;
    let gold_b = degenerate_b.capture_snapshot().gameplay.players[&LOCAL_PLAYER].gold;
    assert_eq!(
        gold_b,
        gold_a.saturating_add(1),
        "reason=duplicate_gather_double_executes"
    );
    assert_ne!(
        degenerate_a.compute_state_hash(),
        degenerate_b.compute_state_hash(),
        "reason=duplicate_gather_diverges_hash"
    );
}

/// 退化态⑦：两个端点以不同 seed 构造冻结开局，命令流相同但权威状态分叉。
#[test]
fn reverse_validation_unfrozen_opening_is_detected() {
    let opening = opening();

    // 正确态：相同冻结开局定义 + 相同帧流 → 逐帧哈希 / 快照 / 差异诊断一致。
    let mut formal_a = new_match(SEED);
    let mut formal_b = new_match(SEED);
    apply_opening(&mut formal_a, &opening, opening.gameplay.clone());
    apply_opening(&mut formal_b, &opening, opening.gameplay.clone());
    for _ in 0..3 {
        step_frame(&mut formal_a);
        step_frame(&mut formal_b);
    }
    let formal_ok = positive_endpoint_invariants(&formal_a, &formal_b);

    // 退化态：仅 seed 不同，其余冻结开局命令与推进次数完全相同。
    let mut degenerate_a = new_match(SEED);
    let mut degenerate_b = new_match(SEED + 1);
    apply_opening(&mut degenerate_a, &opening, opening.gameplay.clone());
    apply_opening(&mut degenerate_b, &opening, opening.gameplay.clone());
    for _ in 0..3 {
        step_frame(&mut degenerate_a);
        step_frame(&mut degenerate_b);
    }
    let degenerate_ok = positive_endpoint_invariants(&degenerate_a, &degenerate_b);

    assert_formal_passes_and_degenerate_fails(formal_ok, degenerate_ok, "unfrozen_opening_seed");
    assert_ne!(
        degenerate_a.compute_state_hash(),
        degenerate_b.compute_state_hash(),
        "reason=seed_diverges_hash"
    );
    assert_ne!(
        degenerate_a.capture_snapshot(),
        degenerate_b.capture_snapshot(),
        "reason=seed_diverges_snapshot"
    );
    assert!(
        degenerate_a.first_difference(&degenerate_b).is_some(),
        "reason=seed_located_by_first_difference"
    );
}

/// 真人攻击两座 AI 建筑的正式命令流。
fn attack_commands(opening: &Opening, frame: u64) -> Vec<Command> {
    vec![
        Command::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            400,
            CommandKind::Attack {
                attacker: opening.local_hall,
                target: opening.ai_buildings[0],
            },
        ),
        Command::new(
            Frame::new(frame),
            LOCAL_PLAYER,
            401,
            CommandKind::Attack {
                attacker: opening.local_hall,
                target: opening.ai_buildings[1],
            },
        ),
    ]
}

/// 既有结算命令面：由真人席位提交，但在批次 1 中只允许建筑全毁后出现。
fn leave_command(frame: u64) -> GameplayCommand {
    GameplayCommand::new(
        Frame::new(frame),
        LOCAL_PLAYER,
        500,
        GameplayCommandKind::Leave { player: AI_PLAYER },
    )
}

/// AI 建筑是否全部不再存活；与批次 1 的视图判据一致。
fn buildings_destroyed(sim: &RtsMatch, opening: &Opening) -> bool {
    let view = sim.read_view();
    opening.ai_buildings.iter().all(|building| {
        !view
            .entities
            .iter()
            .any(|entity| entity.id == building.value() && entity.alive)
    })
}

/// 「打完一局」正向断言链的只读证据。
struct MatchOutcome {
    frame9_buildings_destroyed: bool,
    frame9_victory_resolved: bool,
    frame10_buildings_destroyed: bool,
    victory: VictoryState,
    tick: u64,
}

/// 正向断言链：帧 9 建筑全毁且尚未结算，帧 10 才由既有结算命令面产生胜者。
fn positive_full_match_invariant(outcome: &MatchOutcome) -> bool {
    outcome.frame9_buildings_destroyed
        && !outcome.frame9_victory_resolved
        && outcome.frame10_buildings_destroyed
        && outcome.victory.resolved
        && outcome.victory.winner == Some(LOCAL_PLAYER)
        && outcome.victory.resolved_frame == LEAVE_FRAME
        && outcome.tick == LEAVE_FRAME + 1
}

/// 正式夹具：帧 9 攻击摧毁 AI 全部建筑，帧 10 才提交 `Leave`。
fn formal_match_outcome(opening: &Opening) -> MatchOutcome {
    let mut sim = new_match(SEED);
    apply_opening(&mut sim, opening, opening.gameplay.clone());
    for _ in 1..ATTACK_FRAME {
        step_frame(&mut sim);
    }
    let attacks = attack_commands(opening, ATTACK_FRAME);
    apply_frame(&mut sim, &attacks, &[])
        .expect("ai_lockstep_reverse: reason=formal_attack_frame_failed");
    let frame9_buildings_destroyed = buildings_destroyed(&sim, opening);
    let frame9_victory_resolved = sim.capture_snapshot().gameplay.victory.resolved;

    let leave = leave_command(LEAVE_FRAME);
    apply_frame(&mut sim, &[], &[leave])
        .expect("ai_lockstep_reverse: reason=formal_leave_frame_failed");
    let snapshot = sim.capture_snapshot();
    MatchOutcome {
        frame9_buildings_destroyed,
        frame9_victory_resolved,
        frame10_buildings_destroyed: buildings_destroyed(&sim, opening),
        victory: snapshot.gameplay.victory,
        tick: sim.read_view().tick,
    }
}

/// 退化夹具：AI 建筑仍存活时提前提交 `Leave`，不执行任何攻击命令。
fn leave_shortcut_outcome(opening: &Opening) -> MatchOutcome {
    let mut sim = new_match(SEED);
    apply_opening(&mut sim, opening, opening.gameplay.clone());
    let early_leave = leave_command(1);
    apply_frame(&mut sim, &[], &[early_leave])
        .expect("ai_lockstep_reverse: reason=shortcut_leave_frame_failed");
    for _ in 2..=ATTACK_FRAME {
        step_frame(&mut sim);
    }
    let frame9_buildings_destroyed = buildings_destroyed(&sim, opening);
    let frame9_victory_resolved = sim.capture_snapshot().gameplay.victory.resolved;

    step_frame(&mut sim);
    let snapshot = sim.capture_snapshot();
    MatchOutcome {
        frame9_buildings_destroyed,
        frame9_victory_resolved,
        frame10_buildings_destroyed: buildings_destroyed(&sim, opening),
        victory: snapshot.gameplay.victory,
        tick: sim.read_view().tick,
    }
}

/// 专项：证明「未摧毁建筑就 `Leave`」不满足批次 1 的正向断言链。
#[test]
fn reverse_validation_leave_shortcut_is_not_a_full_match() {
    let opening = opening();
    let formal = formal_match_outcome(&opening);
    let shortcut = leave_shortcut_outcome(&opening);
    let formal_ok = positive_full_match_invariant(&formal);
    let shortcut_ok = positive_full_match_invariant(&shortcut);

    assert_formal_passes_and_degenerate_fails(formal_ok, shortcut_ok, "leave_shortcut");
    assert!(
        formal.frame9_buildings_destroyed,
        "reason=formal_buildings_destroyed_before_leave"
    );
    assert!(
        !formal.frame9_victory_resolved,
        "reason=formal_victory_waits_for_leave_frame"
    );
    assert!(
        !shortcut.frame9_buildings_destroyed,
        "reason=shortcut_leaves_buildings_alive"
    );
    assert!(
        shortcut.frame9_victory_resolved,
        "reason=shortcut_resolves_victory_too_early"
    );
    assert_ne!(
        shortcut.victory.resolved_frame, LEAVE_FRAME,
        "reason=shortcut_resolved_frame_is_not_formal_frame"
    );
    assert_eq!(
        formal.victory.resolved_frame, LEAVE_FRAME,
        "reason=formal_resolved_frame_recorded"
    );
}
