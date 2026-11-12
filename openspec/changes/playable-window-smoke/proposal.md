# Proposal

## Why

> 路线图阶段（与 `gameplay-command-wire-format` 的 (f) 口径一致）：本 change 按 `openspec/config.yaml` rules.proposal 的 P0–P5 阶段枚举，**不新增阶段**，标注为「**P6 归档后的集成收尾补齐**」。`config.yaml` rules.proposal 的「P0-P5」枚举保持不动（用户已多次拍板）。相关 ADR：[ADR 0001](../../../docs/decisions/0001-license.md)（资产与法律）、[ADR 0002](../../../docs/decisions/0002-determinism.md)（确定性契约）、[ADR 0003](../../../docs/decisions/0003-scope.md)（范围与分期 / 否决通用引擎）、[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)（冻结坐标 / 拒绝 `f64` 进权威状态）、[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md)（线格式与分层边界，仅引用）。

P0–P6 已全部实施并归档，但 `crates/w3-client/` 只有 `lib.rs`，**没有 `main.rs` / example / bin**；`ClientApp::run()`（`crates/w3-client/src/app.rs:210`）在全仓库无任何调用者，唯一构造点 `tests/interaction.rs:63` 是 CPU 测试。因此 P6 的输入 / HUD / 相机 / 30 Hz 累加器那条**真实 GPU 窗口路径根本启动不起来**：已有验收全是 CPU 测试（`tests/replay.rs` / `interaction.rs` / `lockstep.rs`），真实 GPU 窗口冒烟从未做过。用户询问「如何人工测试」时发现该缺口，明确拍板「另开 change 补入口」——本 change 即该独立 change。它只补一个零资产、零新依赖、可编译可运行的最小启动入口，让人工能在有 GPU / 显示设备的桌面执行 P6 的窗口冒烟清单；不改任何既有 API、不实施内核改动。规范四件套已立，`/opsx:apply` 已实施。

首轮人工 GPU 冒烟进一步暴露 P6 既有缺陷：输入拾取用的 `ScreenMapper` 是轴对齐线性正交映射，而 3D 单位由 `Camera::matrix` 的透视斜视投影绘制，同一世界点在两套映射下的屏幕位置差 100–250px，远超 24px 拾取半径，导致框选 / 施法必然失败、HUD 血条 / 选中框与 3D 单位分离。用户拍板**扩展本 change 范围直接修复**（见 What Changes 与 `design.md` D9）。

第三轮真窗口运行再暴露三点：① 投影同源后仍按 24px 固定半径拾取，而单位占位四面体在屏幕上有 200–400px，单击 / 选中框与实体尺寸严重失配；② 窗口由 winit 默认尺寸创建，过小不便验收；③ HUD 因字体降级完全无文字，用户不知道按什么键操作。用户再次拍板：拾取改用单位屏幕包围盒、窗口默认 1920×1080、启用 egui 内置开源字体 `default_fonts` 并保证 UI 可见（文案改 ASCII，中文上屏留后续独立 change）；许可证变更由程序员本人授权并在 `NOTICE.md` 登记。

第四轮真窗口运行（物理 2880×1620 / 逻辑 1920×1080 / `scale_factor=1.5`）再暴露两个根因：HUD 把 winit 物理像素坐标直接当 egui 逻辑坐标绘制，选中框 / 血条整体偏移 1.5 倍；example 的 `create_with_world` 用空冻结玩法定义，Q 施法 / T 生产 / C 取消在内核里 NoEffect。用户要求先核对参考开源项目、写足 CPU 自测再人工验收，本轮据此处理。

## What Changes

- 新增 cargo example `crates/w3-client/examples/playable.rs`，以 `cargo run -p w3-client --example playable` 启动真实 winit 窗口；复用既有 `ClientApp::new(...).run()` + 本地 `FixedStepClient`，不新写宿主循环。
- example 只用**合成地形 / 合成单位脚本**（参照 `crates/w3-render/examples/render_frame.rs` 的 `fallback_terrain` 与 `crates/w3-core/tests/determinism.rs` 的 `open_world` / `script()`），MUST NOT 读 `config.toml`、导出 JSON、`assets-export/`、`slk-exported/` 或任何暴雪资产 / MPQ 派生数据（ADR 0001）；任何 `f64 → SimFixed` 走 `w3-app::freeze_f64`（ADR 0004）；截图 / 日志等产物不入库。
- 新增**人工 GPU 冒烟验收清单**（窗口能开、`GpuContext::new` 成功、egui HUD overlay pass、HUD 文字可见、左键框选 / 右键移动 / 施法 / 生产键、WASD / 滚轮相机、`Resized` 后画面正常且逻辑帧不跳、高刷下逻辑帧恒定 30），明确**只能人工在有 GPU / 显示设备的桌面执行**。
- **启用 `egui/default_fonts` 内置开源字体**：HUD 文字层（资源 / 人口 / 生产队列 / 按键提示）正常显示；内置字体无 CJK 字形，故 `i18n` 文案为 ASCII，中文上屏留后续独立 change（许可证见 `NOTICE.md`）。英文结构化降级日志仅在字体数据为空时出现。
- **修复 P6 拾取 / 呈现投影不一致**：`w3-render` 新增 `ScreenProjection`（与 `Camera::matrix` 同源的 `view_proj` 及逆矩阵），`w3-client::ScreenMapper` 增加可选投影并委托，`ClientApp::mapper()` 注入；`scene::UNIT_MARKER_SIZE` 提为常量供拾取平面推导；`interaction.rs` 与新 CPU 回归测试验证框选 / 命中按呈现投影定位。既有公开签名不变。
- **拾取按单位占位标记的屏幕包围盒判定**（`ScreenProjection::unit_screen_bounds`）：框选 / 单击 / 敌方拾取与 HUD 选中框共用同一包围盒，替换 24px 固定半径与硬编码 40×40 选中框。
- **窗口默认 1920×1080**：`ClientApp::resumed` 创建窗口时用 `with_inner_size(LogicalSize::new(1920.0, 1080.0))`，并输出英文结构化窗口诊断日志。
- **统一高 DPI 坐标空间**：winit 鼠标与 GPU 交换链是物理像素、egui HUD 是逻辑像素；`ScreenMapper` 增加 `pixels_per_point`（窗口 `scale_factor`）与 `to_logical` / `to_logical_bounds`，`HudModel::collect` 把 HUD 坐标换算到逻辑空间，选中框 / 血条不再整体偏移；HUD 为所有存活单位绘制队伍色标记，敌方单位可分辨、可拾取。
- **注入合成冻结玩法定义**：example 用 `RtsMatch::create_with_definitions` + `FixedStepClient::from_match` 注入 `unit_def = 1` / `ability = 10`，修复 Q 施法 / T 生产 / C 取消因 `create_with_world` 空定义而 NoEffect 的问题；纯内存合成、零资产读取。
- **BREAKING：无既有签名变更**。新增 example 与 `ScreenProjection` / `ScreenMapper::with_projection` / `unit_screen_bounds` / `ClientApp::screen_mapper` / `UNIT_MARKER_SIZE` / `HudUnit.bounds_*` 公开项；`w3-client/Cargo.toml` 的 `egui` 启用 `default_fonts` feature，`Cargo.lock` 仅新增 `epaint_default_fonts` 传递包，`NOTICE.md` 登记字体许可证；不新增直接依赖、不新增仓库资产文件。
- 新增独立 capability `playable-window-entry`；**不产生其它 capability 的 MODIFIED delta**（避免与其它 active change 争用）。

## Capabilities

### New Capabilities

- `playable-window-entry`: 可玩窗口冒烟入口的行为契约——零暴雪资产的最小 cargo example 复用既有 `ClientApp` 装配把真实 GPU 窗口路径启动起来；只调公开 API、`w3-core` 零改动、不新增权威状态、状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照不变；拾取映射与呈现投影一致且按单位屏幕包围盒判定；默认 1920×1080 窗口；启用内置开源字体、HUD 文字（含按键提示）可见、中文上屏留后续；人工 GPU 冒烟清单与人工 / CI 边界；只新增一个 capability 的 ADDED delta。

### Modified Capabilities

（无。）**刻意不产生任何既有 capability 的 MODIFIED delta**：主 `openspec/specs/` 的 11 个 capability（`3d-rendering` / `content-import` / `cross-platform-determinism` / `deterministic-simulation` / `gameplay-command-wire` / `gameplay-systems` / `linux-platform-backend` / `lockstep-networking` / `pathfinding` / `playable-client` / `unit-orders`）的需求**无行为变更**，本 change 只是新增一个可运行入口并修复其拾取一致性，是 `playable-client` 既有要求的**行使**而非改写。当前仅 1 个 active change（`playable-window-smoke`）；`linux-winit-platform-backend` / `cross-platform-golden-hash` / `gameplay-command-wire-format` 均已归档、其 capability 已进入主 specs，本 change 不产生任何 MODIFIED delta，归档时无 capability 争用。归档顺序见 `design.md` D8：本 change 在 P6（已归档）之后，与已归档的 `gameplay-command-wire-format` 无依赖。本轮不碰主 `openspec/specs/`、不提前归档。

## Impact

- **确定性契约（ADR 0002）**：`crates/w3-core/` **零改动**；**不新增任何权威状态字段**，故无新字段需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照；状态哈希字段集合、帧序、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、快照头部布局**全部不变，不 bump 快照版本**。`step()` 无参；累加器余数 / 插值 `alpha` / 相机 / 选中 / HUD / 投影矩阵等表现层状态不进哈希或快照。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿。跨平台确定性已在 Windows 与 Debian 12 x86_64 / `rustc 1.99.1` 通过离线 golden hash 逐字节比对验证（ADR 0002 复核记录），macOS / 其它 CPU 架构 / 其它 rustc 版本仍待验证，不得据本 change 声称全平台已达成。
- **资产与法律边界（ADR 0001 / ADR 0004）**：零暴雪资产 example，不提交任何 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效；产物（截图 / 日志）留本机、不入库。不新增直接依赖；`egui/default_fonts` 在编译期内嵌开源字体（`(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`），`Cargo.lock` 新增 `epaint_default_fonts` 传递包，`NOTICE.md` 已登记；许可证变更由程序员本人拍板（CODEBUDDY.md §10 人工独占）。不复制任何字体文件进仓库、不读取本机字体。只读参考项目（含 AGPLv3 的 `WarsmashModEngine`）不作代码复制。
- **分层依赖**：example 在 `w3-client` 内部向下依赖 `w3-core` / `w3-render` / `w3-app`（`freeze_f64`）/ `w3-net` 既有依赖，`w3-core` 不反向依赖任何上层 crate；不出现第二套呈现代码、第二份 `RtsMatch`、复制的事件循环；表现层不反写模拟状态（ADR 0002 §5）。拾取一致性修复只新增公开项、不改既有签名。owner = `client-owner`（可写 `crates/w3-client/` / `crates/w3-render/`，正好覆盖全部代码改动），配只读 `critic`，满足「一个 change 一个 owner」。
- **拾取一致性边界**：`ScreenProjection` 与 `Camera::matrix` 由同一 `Camera` / `TerrainMesh` 纯函数构造；`world_to_screen` 以单位占位标记中心平面（`UNIT_MARKER_SIZE * 0.8`）为基准、`screen_to_world` 以 y=0 地面为基准，`unit_screen_bounds` 给出单位四面体的保守屏幕包围盒；全部为表现层 `f32` / `f64`，不进权威状态 / 状态哈希 / 快照。合成地形保持很浅的起伏以消除贴地高度偏差；真实起伏地图的精确贴地拾取留后续项。
- **HUD 与字体边界**：启用内置开源字体后 `HudAssets::prepare` 返回 `text_enabled=true`，HUD 文字层（资源 / 人口 / 生产队列 / 按键提示）正常绘制；内置字体无 CJK 字形，故文案为 ASCII，中文上屏留后续独立 change。UI 文案仍走 i18n、MUST NOT 进入日志 / 权威状态 / 状态哈希；字体数据为空时才走英文结构化降级日志并隐藏文字元素。
- **CI 边界**：`.github/workflows/ci.yml` 的 `ubuntu-latest` / `windows-latest` matrix 跑 `python tools/verify_all.py`，**只覆盖编译与 CPU 测试**（含 `w3-core/tests/determinism.rs`、`w3-render/tests/isolation.rs` 与新增投影 / 交互回归）；Linux 侧 winit 平台后端已由 `linux-winit-platform-backend` 修好。CI 双平台只验证**工具链**跨平台（CODEBUDDY.md §2），**不得声称 CI 已验证 GPU 链路**，也不等于模拟内核确定性已达成。
- **人工验收**：GPU 冒烟清单为**人工项**，只能在有 GPU / 显示设备的桌面执行；无 GPU / headless 环境 MUST 明确报告无法执行、不静默跳过。清单未逐项勾选前不得归档。
- **规范边界（范围扩展后）**：本轮写 `openspec/changes/playable-window-smoke/` 下四件套、`crates/w3-client/` / `crates/w3-render/` 实现、`w3-client/Cargo.toml`、`Cargo.lock`（仅新增传递包）与 `NOTICE.md` 的许可证登记；不碰主 `openspec/specs/`、其它 change 目录（含归档目录）、`openspec/config.yaml`、`docs/`、`.codebuddy/`、`CODEBUDDY.md`、根配置；本地 commit 由 AI 执行（中文描述），归档（`/opsx:archive`）、`push` / `fetch` / `pull` / `clone` 及任何 git 远端命令一律由人工 / 编排者执行（CODEBUDDY.md §4 / §9）。
