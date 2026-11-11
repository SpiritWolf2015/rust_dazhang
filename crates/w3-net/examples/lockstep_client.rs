//! 人工联机验收客户端（配合 `w3-relay` bin）。
//!
//! 用法：
//! `cargo run -p w3-net --example lockstep_client -- <relay-addr> <player-id> <frames>`
//! 例（三个独立终端，relay 需保持运行）：
//! `cargo run -p w3-relay -- 127.0.0.1:9000 2`
//! `cargo run -p w3-net --example lockstep_client -- 127.0.0.1:9000 1 30`
//! `cargo run -p w3-net --example lockstep_client -- 127.0.0.1:9000 2 30`
//!
//! 两个客户端应逐帧打印相同的 hash，且最终 `done: ... last_hash=...` 一致。
//! 本 example 是验收演示：`sleep` 只用于节流网络轮询，不进入模拟状态；
//! 帧推进完全由收到的 relay 权威帧命令集驱动。
//! 每帧在普通命令之外额外上行一条玩法命令，按输入帧号轮转覆盖施法 / 生产 /
//! 取消生产三类，用于验证玩法命令经真实 UDP 上行、广播后进入权威帧批（tasks 8.6）。

use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use w3_core::{
    Command, CommandKind, DefId, EntityId, Frame, GameplayCommand, GameplayCommandKind,
    MatchConfig, RtsMatch, SimFixed, StaticWorld,
};
use w3_net::packet::{Packet, PacketKind};
use w3_net::transport::{Endpoint, UdpTransport};
use w3_net::{ClientSession, Transport, apply_frame, init_logging};

/// 命令的 `execute_frame` 基于本地当前帧略微超前，避免过期帧被拒绝。
const INPUT_DELAY: u64 = 4;

/// 演示用技能定义编号，与 `w3-client` 输入层 `DEFAULT_ABILITY` 一致（合成配置，非导出资产）。
const DEMO_ABILITY: DefId = 10;
/// 演示用单位定义编号，与 `w3-client` 输入层 `DEFAULT_UNIT_DEF` 一致。
const DEMO_UNIT_DEF: DefId = 1;

fn command_for(sequence: u64, player: u8, execute_frame: u64) -> Command {
    if sequence == 0 {
        return Command::new(
            Frame::new(execute_frame),
            player,
            1,
            CommandKind::Spawn {
                x: SimFixed::from_raw(65536 * (i64::from(player) + 1)),
                y: SimFixed::from_raw(65536),
                hp: 100,
                speed: SimFixed::from_raw(196608),
                attack_damage: 10,
                attack_range: SimFixed::from_raw(655360),
            },
        );
    }
    Command::new(
        Frame::new(execute_frame),
        player,
        sequence as u32,
        CommandKind::Noop,
    )
}

/// 按输入帧号轮转生成一条玩法命令及其小写 `kind` 标签。
///
/// `validate_gameplay_entities` 只拒绝 `EntityId::NONE`，不要求实体真实存在；
/// 因此这里统一使用固定的非零 `EntityId`，执行期对不存在的实体是确定性空操作，
/// 三类 `Cast` / `Train` / `CancelTrain` 都能进入权威帧批而不会触发 `NetError`。
fn gameplay_for(
    next_input: u64,
    player: u8,
    execute_frame: u64,
) -> (GameplayCommand, &'static str) {
    // 固定非零实体 ID：编码为 uint64 后解码仍非零，避免被同源校验判为非法实体。
    let caster = EntityId::new(1).expect("demo entity id must be non-zero");
    let target = EntityId::new(2).expect("demo entity id must be non-zero");
    let producer = EntityId::new(1).expect("demo entity id must be non-zero");
    let sequence = next_input as u32;
    match next_input % 3 {
        0 => (
            GameplayCommand::new(
                Frame::new(execute_frame),
                player,
                sequence,
                GameplayCommandKind::Cast {
                    caster,
                    ability: DEMO_ABILITY,
                    target,
                },
            ),
            "cast",
        ),
        1 => (
            GameplayCommand::new(
                Frame::new(execute_frame),
                player,
                sequence,
                GameplayCommandKind::Train {
                    producer,
                    unit_def: DEMO_UNIT_DEF,
                    is_building: false,
                },
            ),
            "train",
        ),
        _ => (
            GameplayCommand::new(
                Frame::new(execute_frame),
                player,
                sequence,
                GameplayCommandKind::CancelTrain { producer, index: 0 },
            ),
            "cancel_train",
        ),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let mut args = env::args().skip(1);
    let relay: SocketAddr = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:9000".to_string())
        .parse()?;
    let player_id: u8 = args.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let frames: u64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(30);

    let transport = UdpTransport::bind("127.0.0.1:0")?;
    let mut session = ClientSession::new(transport, Endpoint::Udp(relay), player_id, 0);
    let mut sim = RtsMatch::create_with_world(
        MatchConfig {
            seed: 42,
            logic_fps: 30,
            content_hash: 0,
        },
        StaticWorld::open_world(16, 16, SimFixed::from_int(8).unwrap()),
    );

    let hello = Packet::new(PacketKind::Hello, 0, 0, 0, vec![]).encode()?;
    session
        .transport_mut()
        .send(&Endpoint::Udp(relay), &hello)?;
    log::info!(target: "w3_net", "client started: player_id={} relay={} frames={}", player_id, relay, frames);

    let mut next_input = 0u64;
    let mut applied = 0u64;
    let mut last_hash = 0u64;
    // 累计从 `pop_ready()` 取出的权威帧批中的玩法命令条数，两端应相等且大于 0。
    let mut gameplay_applied = 0u64;
    // 连续缺帧只请求一次重同步，避免 lobby 未就绪时反复刷屏。
    let mut resync_pending = false;
    while applied < frames {
        session.poll()?;
        // 每轮只提交一条尚未提交的普通命令，并在其之外追加一条玩法命令上行广播。
        if next_input < frames {
            let execute_frame = sim.read_view().tick + INPUT_DELAY;
            let (gameplay, kind) = gameplay_for(next_input, player_id, execute_frame);
            session.submit_input(
                next_input,
                &[command_for(next_input, player_id, execute_frame)],
                &[gameplay],
            )?;
            log::info!(target: "w3_net", "gameplay uplink: frame={} kind={}", next_input, kind);
            next_input += 1;
        }
        // 应用所有已就绪的权威帧。
        while let Some(frame_commands) = session.pop_ready() {
            gameplay_applied += frame_commands.gameplay.len() as u64;
            let hash = apply_frame(&mut sim, &frame_commands.commands, &frame_commands.gameplay)?;
            session.report_checksum(frame_commands.frame, hash)?;
            last_hash = hash;
            applied += 1;
            log::info!(target: "w3_net", "frame applied: frame={} hash={:#x}", frame_commands.frame, hash);
        }
        let timed_out = session.tick()?;
        if timed_out && !resync_pending {
            log::warn!(target: "w3_net", "frame timeout: applied={} requesting resync", applied);
            session.request_resync()?;
        }
        resync_pending = timed_out;
        std::thread::sleep(Duration::from_millis(33));
    }
    log::info!(target: "w3_net", "done: frames={} last_hash={:#x} gameplay_applied={}", applied, last_hash, gameplay_applied);
    Ok(())
}
