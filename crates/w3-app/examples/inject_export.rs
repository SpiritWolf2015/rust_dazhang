//! 针对本机导出地图目录的注入演示。
//!
//! 用法：
//!   cargo run -p w3-app --example inject_export -- <map-dir>
//!
//! 路径在运行时提供，导出的 JSON 来自玩家本机的经典客户端，绝不提交。
//! 本示例从地图头部冻结 `tileSize` / `centerOffset`，注入地形与单位摆放，
//! 并推进若干帧验证链路可用。

use std::path::Path;

use w3_app::{create_match, inject_map};
use w3_content::load_map;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: inject_export <map-dir>");
        std::process::exit(2);
    }

    let map = match load_map(Path::new(&args[1])) {
        Ok(map) => map,
        Err(error) => {
            eprintln!("load_map failed: {error}");
            std::process::exit(1);
        }
    };
    let injection = match inject_map(&map) {
        Ok(injection) => injection,
        Err(error) => {
            eprintln!("inject_map failed: {error}");
            std::process::exit(1);
        }
    };

    let mut sim = create_match(&injection, 1);
    for _ in 0..10 {
        sim.step();
    }
    let origin = injection.world.origin();
    println!(
        "injected=1 cells={}x{} origin={},{} cell_size={} spawns={} identity={:#x} state_hash={:#x}",
        injection.world.width(),
        injection.world.height(),
        origin.0.raw(),
        origin.1.raw(),
        injection.world.cell_size().raw(),
        injection.spawn_commands.len(),
        injection.content_hash,
        sim.compute_state_hash()
    );
}
