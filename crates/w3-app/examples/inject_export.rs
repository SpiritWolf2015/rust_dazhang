//! 针对本机导出地图目录的注入演示。
//!
//! 用法：
//!   cargo run -p w3-app --example inject_export -- <map-dir> [defs-dir]
//!
//! 路径在运行时提供，导出的 JSON 来自玩家本机的经典客户端，绝不提交。
//! 本示例从地图头部冻结 `tileSize` / `centerOffset`，注入地形与单位摆放，
//! 并推进若干帧验证链路可用。
//!
//! 可选的 `defs-dir` 是 SLK 派生定义目录，按 app 层约定读取
//! `units.json` / `abilities.json` / `items.json` / `upgrades.json`
//! （缺失的文件视为空定义集）。提供该目录时走 `inject_map_with_definitions`，
//! 把冻结定义与组合内容哈希注入对局；否则退化为仅地图注入。
//! 第一版列名是内容层的规范化合成列名，真实导出表头的字段映射属后续扩展。

use std::path::Path;

use w3_app::{Injection, create_match, inject_map, inject_map_with_definitions};
use w3_content::{DefinitionTables, ParsedMap, SlkTable, from_slk_tables, load_map, load_table};

/// 读取可选的 SLK 定义表文件；文件不存在视为该类空定义集。
fn optional_table(path: &Path) -> Result<Option<SlkTable>, String> {
    if !path.exists() {
        return Ok(None);
    }
    load_table(path)
        .map(Some)
        .map_err(|error| format!("load_table failed for {}: {error}", path.display()))
}

/// 按是否提供定义目录，选择「仅地图注入」或「地图 + SLK 定义注入」。
fn build_injection(map: &ParsedMap, defs_dir: Option<&String>) -> Result<Injection, String> {
    let Some(defs_dir) = defs_dir else {
        return inject_map(map).map_err(|error| format!("inject_map failed: {error}"));
    };
    let dir = Path::new(defs_dir);
    let units = optional_table(&dir.join("units.json"))?;
    let abilities = optional_table(&dir.join("abilities.json"))?;
    let items = optional_table(&dir.join("items.json"))?;
    let upgrades = optional_table(&dir.join("upgrades.json"))?;
    let defs = from_slk_tables(&DefinitionTables {
        units: units.as_ref(),
        abilities: abilities.as_ref(),
        items: items.as_ref(),
        upgrades: upgrades.as_ref(),
    })
    .map_err(|error| format!("from_slk_tables failed: {error}"))?;
    inject_map_with_definitions(map, &defs)
        .map_err(|error| format!("inject_map_with_definitions failed: {error}"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: inject_export <map-dir> [defs-dir]");
        std::process::exit(2);
    }

    let map = match load_map(Path::new(&args[1])) {
        Ok(map) => map,
        Err(error) => {
            eprintln!("load_map failed: {error}");
            std::process::exit(1);
        }
    };
    let injection = match build_injection(&map, args.get(2)) {
        Ok(injection) => injection,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };

    let mut sim = create_match(&injection, 1);
    for _ in 0..10 {
        sim.step();
    }
    let origin = injection.world.origin();
    println!(
        "injected=1 cells={}x{} origin={},{} cell_size={} spawns={} definitions={} content_hash={:#x} defs_hash={:#x} state_hash={:#x}",
        injection.world.width(),
        injection.world.height(),
        origin.0.raw(),
        origin.1.raw(),
        injection.world.cell_size().raw(),
        injection.spawn_commands.len(),
        injection.definitions.units().len()
            + injection.definitions.abilities().len()
            + injection.definitions.items().len()
            + injection.definitions.upgrades().len(),
        injection.content_hash,
        injection.definitions.content_hash(),
        sim.compute_state_hash()
    );
}
