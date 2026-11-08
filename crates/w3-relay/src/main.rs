//! w3-relay 可执行入口：自建 lobby / 认证时钟服务。
//!
//! 用法：`cargo run -p w3-relay -- <bind-address> <player-count>`
//! 例：`cargo run -p w3-relay -- 127.0.0.1:9000 2`

use std::env;
use std::time::Duration;

use w3_net::{UdpTransport, init_logging};
use w3_relay::Relay;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let mut args = env::args().skip(1);
    let address = args.next().unwrap_or_else(|| "127.0.0.1:9000".to_string());
    let player_count: u8 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2);

    let transport = UdpTransport::bind(&address)?;
    // relay 是帧时钟权威：这里的墙钟只决定帧包广播节拍，不进入客户端
    // 的确定性模拟状态（客户端只按收到的帧包推进，见 ADR 0002）。
    let mut relay = Relay::new(transport, player_count, 2);
    log::info!(target: "w3_relay", "listening: address={} players={}", address, player_count);
    loop {
        relay.poll()?;
        relay.tick()?;
        std::thread::sleep(Duration::from_millis(33));
    }
}
