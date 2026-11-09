//! app 层：把 `w3-content` 的冻结 `f64` 数据注入 `w3-core`（见 ADR 0004）。
//!
//! 本 crate 是内容层与模拟内核之间的唯一适配边界：
//! - `freeze`：用纯整数 IEEE-754 位分解把 `f64` 冻结成 `SimFixed`；
//! - `inject`：把 `ParsedMap` 转成 `StaticWorld` 与出生命令，并把 SLK 派生的
//!   内容层定义转换成内核冻结定义、把内容哈希写进对局配置。
//!
//! `w3-core` 不依赖本 crate，也不依赖 `w3-content`；导出 JSON 留在本机，
//! 由运行时路径提供，绝不提交进仓库。

pub mod freeze;
pub mod inject;

pub use freeze::{FrozenError, freeze_f64};
pub use inject::{
    InjectError, Injection, combine_content_hash, create_match, definition_id, freeze_definitions,
    inject_map, inject_map_with_definitions,
};
