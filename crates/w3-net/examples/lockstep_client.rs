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

use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use w3_core::{Command, CommandKind, Frame, MatchConfig, RtsMatch, SimFixed, StaticWorld};
use w3_net::packet::{Packet, PacketKind};
use w3_net::transport::{Endpoint, UdpTransport};
use w3_net::{ClientSession, Transport, apply_frame, init_logging};

/// 命令的 `execute_frame` 基于本地当前帧略微超前，避免过期帧被拒绝。
const INPUT_DELAY: u64 = 4;

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
    // 连续缺帧只请求一次重同步，避免 lobby 未就绪时反复刷屏。
    let mut resync_pending = false;
    while applied < frames {
        session.poll()?;
        // 每轮只提交一条尚未提交的输入命令。
        if next_input < frames {
            let execute_frame = sim.read_view().tick + INPUT_DELAY;
            session.submit_input(
                next_input,
                &[command_for(next_input, player_id, execute_frame)],
            )?;
            next_input += 1;
        }
        // 应用所有已就绪的权威帧。
        while let Some(frame_commands) = session.pop_ready() {
            let hash = apply_frame(&mut sim, &frame_commands.commands)?;
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
    log::info!(target: "w3_net", "done: frames={} last_hash={:#x}", applied, last_hash);
    Ok(())
}
