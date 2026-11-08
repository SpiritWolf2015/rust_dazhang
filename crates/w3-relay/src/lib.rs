//! w3-relay：自建 lobby / 认证时钟 / 不可变命令流。
//!
//! relay 只广播玩家输入命令与帧号，不持有、不计算游戏状态；`w3-core`
//! 不依赖本 crate。帧包由 relay 的认证时钟关闭后广播，客户端按收到的
//! 帧包推进本地模拟，因此 relay 的节拍不进入客户端的确定性状态。
//! 版本不匹配明确拒绝（design 的 Risks，见 ADR 0001 自建服务边界）。
//! 上下行都走可靠发送队列：下行 Frame 按玩家独立确认与重传。

use std::collections::BTreeMap;

use thiserror::Error;
use w3_core::Command;
use w3_net::frame::FrameAssembler;
use w3_net::packet::{
    FrameCommands, Packet, PacketKind, decode_commands, encode_commands, parse_ack,
    parse_resync_request, payload_reject, payload_welcome,
};
use w3_net::transport::{Endpoint, Transport};
use w3_net::{NetError, ReliableSender, ack_packet, resync_replay_packet};

/// relay 侧错误；错误文本一律英文。
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("net error: {0}")]
    Net(#[from] NetError),
    #[error("lobby error: {0}")]
    Lobby(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// lobby：登记端点、分配稳定的 `player_id`（0 保留给 relay）。
#[derive(Default)]
pub struct Lobby {
    next_player_id: u8,
    players: BTreeMap<u8, Endpoint>,
    endpoints: BTreeMap<Endpoint, u8>,
    started: bool,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            next_player_id: 1,
            ..Self::default()
        }
    }

    /// 加入或重连（同一端点幂等，保持身份不变）。
    pub fn join(&mut self, endpoint: Endpoint) -> Result<u8, RelayError> {
        if let Some(&player_id) = self.endpoints.get(&endpoint) {
            return Ok(player_id);
        }
        let player_id = self.next_player_id;
        self.next_player_id = self
            .next_player_id
            .checked_add(1)
            .ok_or_else(|| RelayError::Lobby("player id exhausted".into()))?;
        self.players.insert(player_id, endpoint.clone());
        self.endpoints.insert(endpoint, player_id);
        Ok(player_id)
    }

    pub fn player_for(&self, endpoint: &Endpoint) -> Option<u8> {
        self.endpoints.get(endpoint).copied()
    }

    pub fn endpoint_for(&self, player_id: u8) -> Option<&Endpoint> {
        self.players.get(&player_id)
    }

    pub fn players(&self) -> Vec<u8> {
        self.players.keys().copied().collect()
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}

/// 不可变命令流：每帧只记录一次，供断线重连补发。
#[derive(Default)]
pub struct CommandLog {
    frames: BTreeMap<u64, FrameCommands>,
}

impl CommandLog {
    /// 已存在同帧则忽略，保证命令流不可变。
    pub fn record(&mut self, frame: FrameCommands) {
        self.frames.entry(frame.frame).or_insert(frame);
    }

    /// 从 `from_frame` 起（含）按帧号顺序返回补发帧。
    pub fn from_frame(&self, from_frame: u64) -> Vec<FrameCommands> {
        self.frames
            .range(from_frame..)
            .map(|(_, frame)| frame.clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// relay 主体：驱动 lobby、帧装配、上下行可靠广播与重连补发。
pub struct Relay<T: Transport> {
    transport: T,
    lobby: Lobby,
    assembler: FrameAssembler,
    log: CommandLog,
    /// 每个玩家独立的下行可靠发送器（Frame 广播按玩家确认与重传）。
    downlink: BTreeMap<u8, ReliableSender>,
    expected_player_count: usize,
    tick: u64,
}

impl<T: Transport> Relay<T> {
    /// `player_count` 为预期玩家数（用于帧装配的收齐判定）。
    pub fn new(transport: T, player_count: u8, timeout_ticks: u64) -> Self {
        let expected: Vec<u8> = (1..=player_count).collect();
        Self {
            transport,
            lobby: Lobby::new(),
            assembler: FrameAssembler::new(0, &expected, timeout_ticks),
            log: CommandLog::default(),
            downlink: BTreeMap::new(),
            expected_player_count: expected.len(),
            tick: 0,
        }
    }

    pub fn lobby(&self) -> &Lobby {
        &self.lobby
    }

    pub fn log(&self) -> &CommandLog {
        &self.log
    }

    pub fn current_frame(&self) -> u64 {
        self.assembler.current_frame()
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// 处理所有待收包；返回已处理包数。
    pub fn poll(&mut self) -> Result<usize, RelayError> {
        let mut handled = 0usize;
        while let Some((from, bytes)) = self.transport.recv()? {
            match Packet::decode(&bytes) {
                Ok(packet) => self.handle_packet(packet, from, &bytes)?,
                Err(NetError::VersionMismatch { .. }) => {
                    let reject = Packet::new(
                        PacketKind::Reject,
                        0,
                        0,
                        0,
                        payload_reject("protocol version mismatch"),
                    );
                    self.transport.send(&from, &reject.encode()?)?;
                }
                Err(error) => return Err(RelayError::Net(error)),
            }
            handled += 1;
        }
        Ok(handled)
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        from: Endpoint,
        raw: &[u8],
    ) -> Result<(), RelayError> {
        match packet.kind {
            PacketKind::Hello => {
                let player_id = self.lobby.join(from.clone())?;
                self.downlink
                    .entry(player_id)
                    .or_insert_with(|| ReliableSender::new(3));
                let current = self.assembler.current_frame();
                let welcome = Packet::new(
                    PacketKind::Welcome,
                    0,
                    0,
                    current,
                    payload_welcome(player_id, current),
                );
                self.transport.send(&from, &welcome.encode()?)?;
                if self.lobby.players().len() >= self.expected_player_count {
                    self.lobby.start();
                }
            }
            PacketKind::Input => {
                if let Some(player_id) = self.lobby.player_for(&from) {
                    let commands: Vec<Command> = decode_commands(&packet.payload)?;
                    self.assembler.submit(player_id, commands);
                    let ack = ack_packet(0, packet.sequence, packet.frame);
                    self.transport.send(&from, &ack.encode()?)?;
                } else {
                    // 显式拒绝未登记端点的输入，仍回 ACK 避免发送端无限重传。
                    log::warn!(target: "w3_relay", "input from unregistered endpoint: {:?}", from);
                    let ack = ack_packet(0, packet.sequence, packet.frame);
                    self.transport.send(&from, &ack.encode()?)?;
                }
            }
            PacketKind::Ack => {
                let sequence = parse_ack(&packet.payload)?;
                match self.lobby.player_for(&from) {
                    Some(player_id) => {
                        if let Some(sender) = self.downlink.get_mut(&player_id) {
                            sender.on_ack(sequence);
                        }
                    }
                    None => {
                        log::warn!(target: "w3_relay", "ack from unregistered endpoint: {:?}", from)
                    }
                }
            }
            PacketKind::Checksum => {
                // 转发给除发送者外的所有已登记玩家，供逐帧比对。
                for player_id in self.lobby.players() {
                    if let Some(endpoint) = self.lobby.endpoint_for(player_id) {
                        if endpoint != &from {
                            self.transport.send(endpoint, raw)?;
                        }
                    }
                }
            }
            PacketKind::ResyncRequest => {
                let (_, from_frame) = parse_resync_request(&packet.payload)?;
                let frames = self.log.from_frame(from_frame);
                let replay = resync_replay_packet(0, &frames)?;
                self.transport.send(&from, &replay.encode()?)?;
            }
            PacketKind::Frame
            | PacketKind::Welcome
            | PacketKind::ResyncReplay
            | PacketKind::Reject => {
                log::warn!(target: "w3_relay", "unexpected downlink packet: kind={:?}", packet.kind);
            }
        }
        Ok(())
    }

    /// 推进一个认证时钟 tick：收齐或超时则关闭帧并广播；重传未确认下行帧。
    /// lobby 未收齐预期玩家前不启动帧时钟，避免帧号先行推进，导致后加入
    /// 的客户端从帧 0 起等待一个永远不会到达的帧。
    pub fn tick(&mut self) -> Result<(), RelayError> {
        if !self.lobby.is_started() {
            return Ok(());
        }
        self.tick += 1;
        if let Some(frame) = self.assembler.tick() {
            self.broadcast_frame(frame)?;
        }
        for (player_id, sender) in &mut self.downlink {
            let Some(endpoint) = self.lobby.endpoint_for(*player_id).cloned() else {
                continue;
            };
            for bytes in sender.tick(self.tick) {
                self.transport.send(&endpoint, &bytes)?;
            }
        }
        Ok(())
    }

    fn broadcast_frame(&mut self, frame: FrameCommands) -> Result<(), RelayError> {
        self.log.record(frame.clone());
        let commands = encode_commands(&frame.commands)?;
        for player_id in self.lobby.players() {
            let Some(endpoint) = self.lobby.endpoint_for(player_id).cloned() else {
                continue;
            };
            let Some(sender) = self.downlink.get_mut(&player_id) else {
                continue;
            };
            // sequence 由各玩家独立的 ReliableSender 分配。
            let packet = Packet::new(PacketKind::Frame, 0, 0, frame.frame, commands.clone());
            let (_, bytes) = sender.queue(packet)?;
            self.transport.send(&endpoint, &bytes)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_net::LoopbackBus;
    use w3_net::LoopbackTransport;
    use w3_net::PROTOCOL_VERSION;
    use w3_net::Transport;
    use w3_net::packet::{parse_resync_replay, parse_welcome, payload_resync_request};

    fn hello() -> Vec<u8> {
        Packet::new(PacketKind::Hello, 0, 0, 0, vec![])
            .encode()
            .unwrap()
    }

    #[test]
    fn lobby_assigns_stable_ids_and_is_idempotent() {
        let mut lobby = Lobby::new();
        let a = Endpoint::Loopback(10);
        let b = Endpoint::Loopback(11);
        assert_eq!(lobby.join(a.clone()).unwrap(), 1);
        assert_eq!(lobby.join(b.clone()).unwrap(), 2);
        assert_eq!(lobby.join(a.clone()).unwrap(), 1);
        assert_eq!(lobby.player_for(&b), Some(2));
        assert_eq!(lobby.players(), vec![1, 2]);
    }

    #[test]
    fn command_log_is_immutable_and_ordered() {
        let mut log = CommandLog::default();
        log.record(FrameCommands {
            frame: 2,
            commands: vec![],
        });
        log.record(FrameCommands {
            frame: 1,
            commands: vec![],
        });
        log.record(FrameCommands {
            frame: 1,
            commands: vec![w3_core::Command::new(
                w3_core::Frame::new(1),
                1,
                1,
                w3_core::CommandKind::Noop,
            )],
        });
        assert_eq!(log.len(), 2);
        let replay = log.from_frame(1);
        assert_eq!(
            replay.iter().map(|f| f.frame).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(replay[0].commands.is_empty());
    }

    #[test]
    fn relay_welcomes_players_and_rejects_bad_version() {
        let bus = LoopbackBus::shared();
        let mut relay = Relay::new(LoopbackTransport::new(0, &bus), 2, 2);
        let mut client = LoopbackTransport::new(1, &bus);
        client.send(&Endpoint::Loopback(0), &hello()).unwrap();
        assert_eq!(relay.poll().unwrap(), 1);
        let (_, bytes) = client.recv().unwrap().unwrap();
        let welcome = Packet::decode(&bytes).unwrap();
        assert_eq!(welcome.kind, PacketKind::Welcome);
        assert_eq!(parse_welcome(&welcome.payload).unwrap().0, 1);

        let mut bad = hello();
        bad[4] = PROTOCOL_VERSION + 1;
        client.send(&Endpoint::Loopback(0), &bad).unwrap();
        relay.poll().unwrap();
        let (_, bytes) = client.recv().unwrap().unwrap();
        let reject = Packet::decode(&bytes).unwrap();
        assert_eq!(reject.kind, PacketKind::Reject);
    }

    #[test]
    fn relay_broadcasts_frames_and_replays_from_cursor() {
        let bus = LoopbackBus::shared();
        let mut relay = Relay::new(LoopbackTransport::new(0, &bus), 2, 2);
        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        a.send(&Endpoint::Loopback(0), &hello()).unwrap();
        b.send(&Endpoint::Loopback(0), &hello()).unwrap();
        relay.poll().unwrap();
        a.recv().unwrap();
        b.recv().unwrap();

        let input = Packet::new(
            PacketKind::Input,
            1,
            7,
            0,
            encode_commands(&[w3_core::Command::new(
                w3_core::Frame::new(0),
                1,
                1,
                w3_core::CommandKind::Noop,
            )])
            .unwrap(),
        )
        .encode()
        .unwrap();
        a.send(&Endpoint::Loopback(0), &input).unwrap();
        b.send(&Endpoint::Loopback(0), &input).unwrap();
        relay.poll().unwrap();
        relay.tick().unwrap();

        // 队列里可能先有 Input 的 ACK，过滤出 Frame 广播。
        let mut frame = None;
        while let Some((_, bytes)) = a.recv().unwrap() {
            let packet = Packet::decode(&bytes).unwrap();
            if packet.kind == PacketKind::Frame {
                frame = Some(packet);
                break;
            }
        }
        let frame = frame.expect("frame broadcast missing");
        assert_eq!(frame.frame, 0);
        assert_eq!(relay.log().len(), 1);

        let request = Packet::new(
            PacketKind::ResyncRequest,
            1,
            0,
            0,
            payload_resync_request(1, 0),
        )
        .encode()
        .unwrap();
        a.send(&Endpoint::Loopback(0), &request).unwrap();
        relay.poll().unwrap();
        let mut found = false;
        while let Some((_, bytes)) = a.recv().unwrap() {
            if Packet::decode(&bytes).unwrap().kind == PacketKind::ResyncReplay {
                let replay = Packet::decode(&bytes).unwrap();
                assert_eq!(parse_resync_replay(&replay.payload).unwrap().len(), 1);
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn frame_clock_does_not_advance_before_lobby_is_full() {
        let bus = LoopbackBus::shared();
        let mut relay = Relay::new(LoopbackTransport::new(0, &bus), 2, 1);
        // 无玩家时反复 tick，帧时钟必须停在 0、命令流为空。
        for _ in 0..10 {
            relay.tick().unwrap();
        }
        assert_eq!(relay.current_frame(), 0);
        assert!(relay.log().is_empty());

        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        a.send(&Endpoint::Loopback(0), &hello()).unwrap();
        b.send(&Endpoint::Loopback(0), &hello()).unwrap();
        relay.poll().unwrap();
        assert!(relay.lobby().is_started());

        // 收齐玩家后第一个权威帧必须是帧 0。
        relay.tick().unwrap();
        assert_eq!(
            relay.log().from_frame(0).first().map(|frame| frame.frame),
            Some(0)
        );
    }
}
