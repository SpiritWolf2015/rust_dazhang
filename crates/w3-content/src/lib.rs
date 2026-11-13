//! P1 内容读取层：把上游 `godot_warcraft3` 导出的 JSON 转成中立的 Rust 结构。
//!
//! 范围（ADR 0003 的 P1）：读取 `map-parsed/<slug>/`（地形高度场、单位与装饰物摆放）
//! 以及 `slk-exported/*.json` 表。**本 crate 不含任何原生 MPQ、SLK、MDX 或 W3X
//! 解析器**——该步骤由上游的 Node 工具负责。
//!
//! ## 确定性边界（ADR 0002）
//!
//! 本 crate 中的坐标是上游 JSON 的原始 `f64` 值，属于**冻结输入数据**，
//! 不是权威模拟状态：绝不进入快照或状态哈希，且 `w3-core` 不依赖本 crate。
//! P2 必须先冻结明确的舍入规则，再把这些值转换为 `SimFixed` 送入模拟。
//!
//! ## 冻结定义（P5）
//!
//! `definitions` 模块把 SLK 中立行转换成单位 / 技能 / 物品 / 升级的冻结定义，
//! 并计算确定性的内容哈希。这些结构同样是中立、纯整数的读取结果，不含
//! `SimFixed`、不依赖 `w3-core`；由 app 层在冻结边界转换后注入对局。
//!
//! ## 资产边界（ADR 0001）
//!
//! 本 crate 只包含读取器。它读取的 JSON 派生自玩家本机合法获得的经典客户端，
//! 必须留在本机；其中任何文件都不得提交进本仓库。测试使用合成 fixture。

pub mod definitions;
pub mod gltf;
pub mod heightfield;
pub mod load;
pub mod model_index;
pub mod placements;
pub mod table;
pub mod textures;

pub use definitions::{
    AbilityDef, CONTENT_HASH_SCHEMA, DefinitionError, DefinitionKind, DefinitionTables,
    FrozenDefinitions, ItemDef, UnitDef, UpgradeDef, from_slk_tables,
};
pub use gltf::{GeosetFilter, GltfError, ModelMesh, SubMesh, load_gltf, parse_gltf};
pub use heightfield::{CenterOffset, HeightfieldError, TerrainHeightfield};
pub use load::{ContentError, ParsedMap, load_map, load_table};
pub use model_index::{ENTITY_MODEL_FILES, ModelCatalog, ModelClass, ModelIndexError};
pub use placements::{
    DoodadPlacement, DoodadsFile, PlacementError, Position3, UnitPlacement, UnitsFile,
};
pub use table::{SlkTable, TableError};
pub use textures::{LORDAERON_SUMMER_FILES, RgbaImage, TextureError, load_rgba, load_tileset};
