//! 传输抽象：真实 UDP socket 与内存回环（可注入故障）。
//!
//! 可靠层只依赖 [`Transport`] trait，因此丢包 / 乱序 / 短暂断线可以在
//! 单元测试里精确注入，不依赖真实网络或墙钟。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use std::rc::Rc;

use crate::error::NetError;
use crate::packet::HEADER_LEN;

/// 端点标识：真实 UDP 用地址，内存回环用逻辑编号。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Endpoint {
    Udp(SocketAddr),
    Loopback(u8),
}

/// 传输层契约；`recv` 非阻塞，无数据返回 `Ok(None)`。
pub trait Transport {
    fn send(&mut self, destination: &Endpoint, bytes: &[u8]) -> Result<(), NetError>;
    fn recv(&mut self) -> Result<Option<(Endpoint, Vec<u8>)>, NetError>;
    fn local(&self) -> Endpoint;
}

/// 真实 UDP 端点（非阻塞）。
pub struct UdpTransport {
    socket: UdpSocket,
    local: Endpoint,
    buffer: Vec<u8>,
}

impl UdpTransport {
    pub fn bind(address: &str) -> Result<Self, NetError> {
        let socket = UdpSocket::bind(address)?;
        socket.set_nonblocking(true)?;
        let local = Endpoint::Udp(socket.local_addr()?);
        Ok(Self {
            socket,
            local,
            // 最大合法包长 = 固定包头 + u16 上限 payload。
            buffer: vec![0u8; HEADER_LEN + u16::MAX as usize],
        })
    }

    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}

impl Transport for UdpTransport {
    fn send(&mut self, destination: &Endpoint, bytes: &[u8]) -> Result<(), NetError> {
        let Endpoint::Udp(address) = destination else {
            return Err(NetError::Transport(
                "UdpTransport needs Udp endpoint".into(),
            ));
        };
        self.socket.send_to(bytes, address)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<(Endpoint, Vec<u8>)>, NetError> {
        match self.socket.recv_from(&mut self.buffer) {
            Ok((count, address)) => Ok(Some((
                Endpoint::Udp(address),
                self.buffer[..count].to_vec(),
            ))),
            // Windows UDP：对端 socket 关闭后，ICMP port unreachable 会让
            // 下一次 recv_from 返回 WSAECONNRESET(10054)。这是可忽略的瞬时
            // 错误，不应让 relay 退出；std 无法设置 SIO_UDP_CONNRESET，
            // 故在此与 WouldBlock 一并吞掉。
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::ConnectionReset
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(NetError::Io(error)),
        }
    }

    fn local(&self) -> Endpoint {
        self.local.clone()
    }
}

/// 内存回环总线的故障注入配置。
#[derive(Clone, Debug, Default)]
pub struct FaultInjection {
    /// 丢弃接下来 N 次发送（一次性，重传可补齐）。
    pub drop_next: u64,
    /// 接下来 N 次发送插到队首（制造乱序）。
    pub reorder_next: u64,
    /// 处于断线状态的端点（其收发均被丢弃）。
    pub disconnected: BTreeSet<u8>,
    /// 累计丢弃计数，便于测试断言。
    pub dropped: u64,
}

/// 所有回环端点共享的内存总线。
#[derive(Default)]
pub struct LoopbackBus {
    queues: BTreeMap<u8, VecDeque<(u8, Vec<u8>)>>,
    pub faults: FaultInjection,
}

/// 多端点共享句柄（单线程测试用）。
pub type SharedBus = Rc<RefCell<LoopbackBus>>;

impl LoopbackBus {
    pub fn shared() -> SharedBus {
        Rc::new(RefCell::new(Self::default()))
    }

    pub fn register(&mut self, id: u8) {
        self.queues.entry(id).or_default();
    }

    /// 丢弃接下来 `count` 次发送，用于模拟单个丢包。
    pub fn drop_next(&mut self, count: u64) {
        self.faults.drop_next = count;
    }

    /// 让接下来 `count` 次发送插队，模拟乱序到达。
    pub fn reorder_next(&mut self, count: u64) {
        self.faults.reorder_next = count;
    }

    pub fn disconnect(&mut self, id: u8) {
        self.faults.disconnected.insert(id);
    }

    pub fn reconnect(&mut self, id: u8) {
        self.faults.disconnected.remove(&id);
    }

    fn send(&mut self, from: u8, to: u8, bytes: Vec<u8>) {
        if self.faults.disconnected.contains(&from) || self.faults.disconnected.contains(&to) {
            self.faults.dropped += 1;
            return;
        }
        if self.faults.drop_next > 0 {
            self.faults.drop_next -= 1;
            self.faults.dropped += 1;
            return;
        }
        let reorder = self.faults.reorder_next > 0;
        if reorder {
            self.faults.reorder_next -= 1;
        }
        if let Some(queue) = self.queues.get_mut(&to) {
            if reorder {
                queue.push_front((from, bytes));
            } else {
                queue.push_back((from, bytes));
            }
        }
    }

    fn recv(&mut self, id: u8) -> Option<(u8, Vec<u8>)> {
        self.queues.get_mut(&id)?.pop_front()
    }

    #[cfg(test)]
    fn queued(&self, id: u8) -> usize {
        self.queues.get(&id).map_or(0, VecDeque::len)
    }
}

/// 绑定到内存总线的端点。
pub struct LoopbackTransport {
    id: u8,
    bus: SharedBus,
}

impl LoopbackTransport {
    pub fn new(id: u8, bus: &SharedBus) -> Self {
        bus.borrow_mut().register(id);
        Self {
            id,
            bus: Rc::clone(bus),
        }
    }

    pub fn id(&self) -> u8 {
        self.id
    }
}

impl Transport for LoopbackTransport {
    fn send(&mut self, destination: &Endpoint, bytes: &[u8]) -> Result<(), NetError> {
        let Endpoint::Loopback(to) = *destination else {
            return Err(NetError::Transport(
                "LoopbackTransport needs Loopback endpoint".into(),
            ));
        };
        self.bus.borrow_mut().send(self.id, to, bytes.to_vec());
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<(Endpoint, Vec<u8>)>, NetError> {
        Ok(self
            .bus
            .borrow_mut()
            .recv(self.id)
            .map(|(from, bytes)| (Endpoint::Loopback(from), bytes)))
    }

    fn local(&self) -> Endpoint {
        Endpoint::Loopback(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_delivers_between_registered_endpoints() {
        let bus = LoopbackBus::shared();
        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        a.send(&Endpoint::Loopback(2), b"hello").unwrap();
        assert_eq!(
            b.recv().unwrap(),
            Some((Endpoint::Loopback(1), b"hello".to_vec()))
        );
        assert_eq!(bus.borrow().queued(2), 0);
    }

    #[test]
    fn injected_single_loss_is_recoverable_by_retransmission() {
        let bus = LoopbackBus::shared();
        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        bus.borrow_mut().drop_next(1);
        a.send(&Endpoint::Loopback(2), b"first").unwrap();
        assert_eq!(b.recv().unwrap(), None, "first send should be dropped");
        a.send(&Endpoint::Loopback(2), b"first").unwrap();
        assert_eq!(
            b.recv().unwrap(),
            Some((Endpoint::Loopback(1), b"first".to_vec())),
            "重传应到达"
        );
        assert_eq!(bus.borrow().faults.dropped, 1);
    }

    #[test]
    fn injected_reorder_changes_arrival_order() {
        let bus = LoopbackBus::shared();
        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        a.send(&Endpoint::Loopback(2), b"one").unwrap();
        bus.borrow_mut().reorder_next(1);
        a.send(&Endpoint::Loopback(2), b"two").unwrap();
        let first = b.recv().unwrap().unwrap().1;
        let second = b.recv().unwrap().unwrap().1;
        assert_eq!(first, b"two");
        assert_eq!(second, b"one");
    }

    #[test]
    fn disconnected_endpoint_drops_and_reconnect_restores() {
        let bus = LoopbackBus::shared();
        let mut a = LoopbackTransport::new(1, &bus);
        let mut b = LoopbackTransport::new(2, &bus);
        bus.borrow_mut().disconnect(2);
        a.send(&Endpoint::Loopback(2), b"lost").unwrap();
        assert_eq!(b.recv().unwrap(), None);
        bus.borrow_mut().reconnect(2);
        a.send(&Endpoint::Loopback(2), b"restored").unwrap();
        assert!(b.recv().unwrap().is_some());
    }

    #[test]
    fn udp_transport_roundtrip_on_loopback() {
        let mut a = UdpTransport::bind("127.0.0.1:0").unwrap();
        let mut b = UdpTransport::bind("127.0.0.1:0").unwrap();
        let b_address = match b.local() {
            Endpoint::Udp(address) => address,
            _ => panic!("expected Udp endpoint"),
        };
        a.send(&Endpoint::Udp(b_address), b"ping").unwrap();
        // 非阻塞 socket 在 loopback 上通常立即到达，给少量调度机会。
        let mut received = None;
        for _ in 0..200 {
            if let Some(packet) = b.recv().unwrap() {
                received = Some(packet);
                break;
            }
            std::thread::yield_now();
        }
        let (from, bytes) = received.expect("udp loopback delivery failed");
        assert_eq!(bytes, b"ping");
        assert_eq!(from, a.local());
    }
}
