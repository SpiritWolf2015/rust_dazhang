//! End-to-end check against a locally exported map directory.
//!
//! Usage:
//!   cargo run -p w3-content --example load_export -- <map-dir> [slk-table.json]
//!
//! Paths are supplied at runtime and never committed. The exported JSON comes
//! from the player's own classic client and must stay outside this repository.

use std::path::Path;

use w3_content::{load_map, load_table};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: load_export <map-dir> [slk-table.json]");
        std::process::exit(2);
    }

    let map_dir = Path::new(&args[1]);
    match load_map(map_dir) {
        Ok(map) => {
            let hf = &map.heightfield;
            println!(
                "heightfield: {}x{} tilepoints = {:?} vertices, map {}x{}",
                hf.tilepoint_width,
                hf.tilepoint_height,
                hf.vertex_count(),
                hf.map_width,
                hf.map_height
            );
            println!(
                "  heights={} groundTextures={} layerHeights={} waterHeights={} flagsPacked={}",
                hf.heights.len(),
                hf.ground_textures.len(),
                hf.layer_heights.len(),
                hf.water_heights.len(),
                hf.flags_packed.len()
            );
            println!("  height_at(0,0)={:?} mainTileset={:?}", hf.height_at(0, 0), hf.main_tileset);
            println!("units: {}", map.units.len());
            println!("doodads: {}", map.doodads.len());
            if let Some(unit) = map.units.first() {
                println!(
                    "  first unit type={:?} pos=({},{},{}) owner={}",
                    unit.type_id, unit.position.x, unit.position.y, unit.position.z, unit.owner
                );
            }
        }
        Err(error) => {
            eprintln!("load_map failed: {error}");
            std::process::exit(1);
        }
    }

    if let Some(table_path) = args.get(2) {
        match load_table(Path::new(table_path)) {
            Ok(table) => println!(
                "slk table: source={:?} headers={} rows={} recordCount={}",
                table.source,
                table.headers.len(),
                table.records.len(),
                table.record_count
            ),
            Err(error) => {
                eprintln!("load_table failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
