# NOTICE — rust_dazhang

本文件记录本项目的版权、许可证与上游致谢。**本仓库不包含任何暴雪游戏资产**（见 `README.md` 免责声明与 [ADR 0001](docs/decisions/0001-license.md)）。

## 本项目

- 版权：Copyright (C) 2026 rust_dazhang contributors
- 许可证：GNU General Public License v3.0（见 `LICENSE`）
- 本项目为独立从零编写，**未复制任何上游仓库的源代码**。

## 上游参考项目

下表四个项目为**只读参考**，用于设计借鉴与格式规格；它们的代码不进入本仓库，也不作为 git 子模块引入。

| 项目 | 许可证 / 版权 | 借鉴内容 | 本地参考路径 |
|---|---|---|---|
| `vera20k` | GPLv3 + Section 7 附加条款；Copyright © 2026 Halvor Nikolai Rusten and VERA20K contributors | 确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律 | `C:/open_rts/vera20k` |
| `godot_warcraft3` | MIT；Copyright (c) 2026 李维民 | WC3 MPQ / SLK / BLP / MDX / W3X 格式规格、资产三车道与"工具入库、资产留本机"合规模型；P1 起读取其本机导出 JSON | `C:/open_rts/godot_warcraft3` |
| `rts_kernel_cs` | MIT；Copyright (c) 2026 李维民 | 模拟内核 API 契约、唯一状态所有者、帧阶段顺序、命令稳定排序、快照与状态哈希设计 | `C:/open_rts/rts_kernel_cs` |
| `WarsmashModEngine` | AGPLv3；作者 Retera（见其 `README.md`），根目录 `LICENSE` 为 AGPLv3 全文 | Java / LibGDX 的 WC3 模拟器：JASS / 触发器语义、MPQ 与 CASC 数据源、单位与技能玩法行为（只读参考，不复制代码） | `C:/open_rts/WarsmashModEngine` |

固定参考的提交与各自用途详见 [`docs/references/UPSTREAM.md`](docs/references/UPSTREAM.md)。

MIT 条款要求保留版权与许可声明；`WarsmashModEngine` 为 AGPLv3，与本项目 GPLv3 组合有条件，未经 ADR 0001 复核不得引入其代码。若将来引入其中任一项目的代码片段，须在对应文件保留其原始版权头，并在此处登记来源与许可证。

## 第三方 crate 依赖（P3 w3-render）

P3 新增 `crates/w3-render`（wgpu 3D 渲染层）引入以下 crates.io 依赖；许可证结论已逐一核对各 crate 的 `Cargo.toml`：

| crate | 版本 | 许可证 |
|---|---|---|
| `wgpu` | 30.0.1 | MIT OR Apache-2.0 |
| `winit` | 0.30.13 | Apache-2.0 |
| `glam` | 0.29.3 | MIT OR Apache-2.0 |
| `bytemuck` | 1.25.0 | Zlib OR Apache-2.0 OR MIT |
| `serde` / `serde_json` | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 |
| `thiserror` | 2.0.18 | MIT OR Apache-2.0 |
| `log` | 0.4.35 | MIT OR Apache-2.0 |
| `naga`（`wgpu` 传递依赖） | 30.0.1 | MIT OR Apache-2.0 |

- 上述均为 MIT / Apache-2.0 / Zlib 宽松许可，与本项目 GPLv3 兼容；均为 crates.io 依赖，版本由 `Cargo.lock` 锁定，**未复制任何上游源码进仓库**，沿用「本项目为独立从零编写」的声明。
- `winit` 在本项目禁用默认 wayland / x11 features（本机离线缓存缺 `ab_glyph`），仅启用 `rwh_06`；不影响许可证结论。
- 其余传递依赖未逐一核验，`Cargo.lock` 为依赖图权威清单。
- `WarsmashModEngine`（AGPLv3）仍仅为只读参考，**未引入其代码**。
- 资产边界不变：导出 JSON / PNG / GLB 等资产留在本机，绝不入库（见 [ADR 0001](docs/decisions/0001-license.md)）。

## 第三方 crate 依赖（P4 / protobuf-payload）

[ADR 0005](docs/decisions/0005-protobuf-payload.md) 与 `protobuf-payload` change 把 `W3N1` 包的 payload 层改用 Protocol Buffers，新增以下 crates.io 依赖；许可证结论已逐一核对各 crate 的 `Cargo.toml`：

| crate | 版本 | 许可证 |
|---|---|---|
| `prost` | 0.13.5 | Apache-2.0 |
| `prost-build`（build-dependency） | 0.13.5 | Apache-2.0 |
| `prost-derive` / `prost-types`（传递依赖） | 0.13.5 | Apache-2.0 |
| `prettyplease` / `petgraph` / `multimap` / `heck` / `itertools` / `tempfile` / `regex`（`prost-build` 传递依赖） | 见 `Cargo.lock` | MIT OR Apache-2.0 |

- 上述均为 Apache-2.0 或 MIT OR Apache-2.0 宽松许可，与本项目 GPLv3 兼容；均为 crates.io 依赖，版本由 `Cargo.lock` 锁定，**未复制任何上游源码进仓库**，沿用「本项目为独立从零编写」的声明。
- 生成代码由 `build.rs` 调用 `protoc` 在构建期产生，不属于提交进仓库的第三方源码；CI 双平台须安装同版本 protoc 以保证生成结果一致。
- `W3N1` 帧头仍为本项目自有实现（显式小端 + FNV-1a），未引入第三方线格式代码（见 [ADR 0002](docs/decisions/0002-determinism.md)）。
- 其余传递依赖未逐一核验，`Cargo.lock` 为依赖图权威清单。

## 第三方 crate 依赖（P6 / playable-client）

P6 `p6-playable-client` change 新增 `crates/w3-client`（可玩客户端集成层），引入以下 crates.io 依赖；许可证结论已由 `client-owner` 逐一核对各 crate 的 `Cargo.toml`：

| crate | 版本 | 许可证 |
|---|---|---|
| `egui`（直接依赖） | 0.37.0 | MIT OR Apache-2.0（本项目取 MIT 一侧） |
| `egui-wgpu`（直接依赖） | 0.37.0 | MIT OR Apache-2.0（本项目取 MIT 一侧） |
| `egui-winit`（直接依赖） | 0.37.0 | MIT OR Apache-2.0（本项目取 MIT 一侧） |
| `self_cell`（传递依赖） | 1.3.0 | Apache-2.0 OR GPL-2.0-only（本项目取 Apache-2.0 一侧） |

- 兼容性结论：`egui` 家族明确取 MIT 一侧、`self_cell` 明确取 Apache-2.0 一侧，均与本项目 GPLv3 兼容；均为 crates.io 依赖，版本由 `Cargo.lock` 锁定，**未复制任何上游源码进仓库**，沿用「本项目为独立从零编写」的声明。
- `self_cell` 为双许可（`Apache-2.0 OR GPL-2.0-only`）：`GPL-2.0-only` 一侧与 GPLv3 不兼容，本项目明确选择 `Apache-2.0` 一侧以消除合规争议。它是 egui / epaint 文本栅格化链路（`vello_cpu` / `skrifa` / `read-fonts`）的传递依赖，非直接依赖。
- `egui-winit` 以 `default-features = false` 引入，避免拉入 `ab_glyph` / wayland / x11 等平台装饰依赖，与 `w3-render` 保持最小 `winit` feature 集（`rwh_06`）一致；不影响许可证结论。
- `egui` 的 `default_fonts` feature **已启用**（`playable-window-smoke` change，经用户拍板）：引入 `epaint_default_fonts`，其许可证为 `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`。字体许可证登记见下节；属 `CODEBUDDY.md` 第 10 节人工独占的许可证变更，已由程序员本人授权。内置字体为 egui/epaint 编译时内嵌的开源字体，**仓库不新增任何字体资产文件**。
- 待复核：另有 4 个新增包（`objc2-app-kit` / `objc2-ui-kit` / `wgpu-core-deps-wasm` 属其他平台 target，另一个为本地 `w3-client` crate 自身）本机未逐一核验许可证，建议 CI 双平台时由 `docs-owner` 补齐；本轮不编造其许可证。
- 其余传递依赖未逐一核验，`Cargo.lock` 为依赖图权威清单。
- 资产边界不变：导出 JSON / PNG / GLB 等资产留在本机，绝不入库（见 [ADR 0001](docs/decisions/0001-license.md)）。

## 内置开源字体（playable-window-smoke）

`w3-client` 启用 `egui/default_fonts` 后，`epaint_default_fonts` 在编译期把开源字体内嵌进二进制，用于 HUD 文字渲染；运行时不读取本机字体文件、不加载仓库资产（ADR 0001）：

| 字体 / crate | 许可证 | 版权 |
|---|---|---|
| `epaint_default_fonts`（Ubuntu Font 与若干 emoji 字体集合） | `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0` | 字体作者与 Canonical Ltd.（Ubuntu Font Licence 1.0）、各 OFL 字体作者 |

- 兼容性结论：crate 代码为 MIT OR Apache-2.0，字体数据为 OFL-1.1 / Ubuntu-font-1.0；OFL-1.1 与 GPLv3 兼容，Ubuntu Font Licence 1.0 允许随应用分发且不要求应用改用同许可证，故与本项目 GPLv3 组合可接受。许可证变更已由程序员本人拍板（`CODEBUDDY.md` 第 10 节人工独占）。
- 内置字体不含 CJK 字形，故 `w3-client` 的 HUD 文案为 ASCII；中文上屏需另内嵌 CJK 字体，留后续独立 change。
- 字体数据由 crates.io 依赖在构建期内嵌，**未复制任何字体文件进仓库**；`Cargo.lock` 为依赖图权威清单。

## 第三方 crate 依赖（真实地图视觉 / PNG 解码）

`melee-playable-loop` 阶段 2 在 `w3-content` 新增 `image`（仅启用 `png` feature，`default-features = false`），用于把本机导出的 BLP→PNG 地表贴图与 `war3mapMap.png` 小地图底图解码为 RGBA8。许可证结论由 `cargo metadata` 的 `license` 字段逐一核对；均为 crates.io 依赖、版本由 `Cargo.lock` 锁定，**未复制任何上游源码进仓库**：

| crate | 版本 | 许可证 |
|---|---|---|
| `image`（直接依赖） | 0.25.10 | MIT OR Apache-2.0 |
| `png`（传递依赖） | 0.18.1 | MIT OR Apache-2.0 |
| `flate2` / `fdeflate`（DEFLATE 解码） | 1.1.11 / 0.3.7 | MIT OR Apache-2.0 |
| `miniz_oxide`（DEFLATE 解码，两个版本并存） | 0.8.9 / 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| `crc32fast` / `adler2` / `simd-adler32`（校验链） | 1.5.4 / 2.0.1 / 0.3.10 | MIT OR Apache-2.0 / 0BSD OR MIT OR Apache-2.0 / MIT |
| `zlib-rs` | 0.6.8 | Zlib |
| `byteorder-lite` | 0.1.0 | Unlicense OR MIT |
| `moxcms` / `pxfm`（色彩管理） | 0.8.1 / 0.1.30 | BSD-3-Clause OR Apache-2.0 |

- 兼容性结论：上述均为 MIT / Apache-2.0 / BSD-3-Clause / Zlib / 0BSD / Unlicense 宽松许可，与本项目 GPLv3 兼容；`miniz_oxide` 取 MIT 或 Apache-2.0 一侧、`byteorder-lite` 取 MIT 一侧以消除争议。
- `image` 仅启用 `png` feature，未启用其它编解码器 / 默认 feature，依赖面最小；`naga` 已是 P3 `wgpu` 的既有传递依赖，本节不重复登记。
- 资产边界不变：解码的 PNG 派生自玩家本机合法获得的经典客户端，留在本机、绝不入库（见 [ADR 0001](docs/decisions/0001-license.md)）；本 crate 只含读取器，测试用合成 fixture。
- 其余传递依赖未逐一核验，`Cargo.lock` 为依赖图权威清单。

## 第三方资产与商标

Warcraft、Warcraft III、The Frozen Throne 及相关名称、美术、音乐与游戏内容归其各自权利人所有。本项目与暴雪娱乐无关联、未获其授权或背书。玩家须自备合法获得的经典客户端；仓库不分发任何资产或由资产派生的文件。

## 许可与署名问题

请通过仓库 Issue 提出。
