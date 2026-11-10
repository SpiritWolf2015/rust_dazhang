# Proposal

## Why

> 路线图阶段（与 `gameplay-command-wire-format` 的 (f) 口径一致）：本 change 按 `openspec/config.yaml` rules.proposal 的 P0–P5 阶段枚举，**不新增阶段**，标注为「**P6 归档后的集成收尾补齐**」。`config.yaml` rules.proposal 的「P0-P5」枚举保持不动（用户已多次拍板）。相关 ADR：[ADR 0001](../../../docs/decisions/0001-license.md)（资产与法律）、[ADR 0002](../../../docs/decisions/0002-determinism.md)（确定性契约）、[ADR 0003](../../../docs/decisions/0003-scope.md)（范围与分期 / 否决通用引擎）、[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)（冻结坐标 / 拒绝 `f64` 进权威状态）、[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md)（线格式与分层边界，仅引用）。

P0–P6 已全部实施并归档，但 `crates/w3-client/` 只有 `lib.rs`，**没有 `main.rs` / example / bin**；`ClientApp::run()`（`crates/w3-client/src/app.rs:210`）在全仓库无任何调用者，唯一构造点 `tests/interaction.rs:63` 是 CPU 测试。因此 P6 的输入 / HUD / 相机 / 30 Hz 累加器那条**真实 GPU 窗口路径根本启动不起来**：已有验收全是 CPU 测试（`tests/replay.rs` / `interaction.rs` / `lockstep.rs`），真实 GPU 窗口冒烟从未做过。用户询问「如何人工测试」时发现该缺口，明确拍板「另开 change 补入口」——本 change 即该独立 change。它只补一个零资产、零新依赖、可编译可运行的最小启动入口，让人工能在有 GPU / 显示设备的桌面执行 P6 的窗口冒烟清单；不改任何既有 API、不实施内核改动。本轮仅立规范，代码留到 `/opsx:apply`（后续动作）。

## What Changes

- 新增 cargo example `crates/w3-client/examples/playable.rs`，以 `cargo run -p w3-client --example playable` 启动真实 winit 窗口；复用既有 `ClientApp::new(...).run()` + 本地 `FixedStepClient`，不新写宿主循环。
- example 只用**合成地形 / 合成单位脚本**（参照 `crates/w3-render/examples/render_frame.rs` 的 `fallback_terrain` 与 `crates/w3-core/tests/determinism.rs` 的 `open_world` / `script()`），MUST NOT 读 `config.toml`、导出 JSON、`assets-export/`、`slk-exported/` 或任何暴雪资产 / MPQ 派生数据（ADR 0001）；任何 `f64 → SimFixed` 走 `w3-app::freeze_f64`（ADR 0004）；截图 / 日志等产物不入库。
- 新增**人工 GPU 冒烟验收清单**（窗口能开、`GpuContext::new` 成功、egui HUD overlay pass、中文标签降级、左键框选 / 右键移动 / 施法 / 生产键、WASD / 滚轮相机、`Resized` 后画面正常且逻辑帧不跳、高刷下逻辑帧恒定 30），明确**只能人工在有 GPU / 显示设备的桌面执行**。
- 明确 egui 0.37 不启用 `default_fonts`：中文标签按已裁定不显示、走英文结构化降级日志（`tasks 6.6`）；中文上屏属后续独立项（OFL-1.1 / Ubuntu-font-1.0 属 CODEBUDDY.md §10 人工独占且仍无 CJK 字形）。
- **BREAKING：无**。新增 example，不改既有 API、不改 `w3-client/Cargo.toml` 依赖段、不新增依赖、`Cargo.lock` 不变、`NOTICE.md` 无需登记。
- 新增独立 capability `playable-window-entry`；**不产生其它 capability 的 MODIFIED delta**（避免与其它 active change 争用）。

## Capabilities

### New Capabilities

- `playable-window-entry`: 可玩窗口冒烟入口的行为契约——零资产 / 零新依赖的最小 cargo example 复用既有 `ClientApp` 装配把真实 GPU 窗口路径启动起来；只调公开 API、`w3-core` 零改动、不新增权威状态、状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照不变；人工 GPU 冒烟清单与人工 / CI 边界；字体降级与许可证边界；只新增一个 capability 的 ADDED delta。

### Modified Capabilities

（无。）**刻意不产生任何既有 capability 的 MODIFIED delta**：主 `openspec/specs/` 的 10 个 capability（`3d-rendering` / `content-import` / `cross-platform-determinism` / `deterministic-simulation` / `gameplay-systems` / `linux-platform-backend` / `lockstep-networking` / `pathfinding` / `playable-client` / `unit-orders`）的需求**无行为变更**，本 change 只是新增一个可运行入口供人工验收，是 `playable-client` 既有要求的**行使**而非改写。当前仅 2 个 active change（`gameplay-command-wire-format` / `playable-window-smoke`）；`linux-winit-platform-backend` 与 `cross-platform-golden-hash` 已归档、其 capability 已进入主 specs，本 change 不产生任何 MODIFIED delta，归档时无 capability 争用。归档顺序见 `design.md` D8：本 change 在 P6（已归档）之后，与 `gameplay-command-wire-format` 无依赖。本轮不碰主 `openspec/specs/`、不提前归档。

## Impact

- **确定性契约（ADR 0002）**：`crates/w3-core/` **零改动**；**不新增任何权威状态字段**，故无新字段需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照；状态哈希字段集合、帧序、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、快照头部布局**全部不变，不 bump 快照版本**。`step()` 无参；累加器余数 / 插值 `alpha` / 相机 / 选中 / HUD 等表现层状态不进哈希或快照。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿。跨平台确定性已在 Windows 与 Debian 12 x86_64 / `rustc 1.99.1` 通过离线 golden hash 逐字节比对验证（ADR 0002 复核记录），macOS / 其它 CPU 架构 / 其它 rustc 版本仍待验证，不得据本 change 声称全平台已达成。
- **资产与法律边界（ADR 0001 / ADR 0004）**：零资产 example，不提交任何 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效；产物（截图 / 日志）留本机、不入库。不新增依赖，`Cargo.lock` 不变，`NOTICE.md` 无需登记；不启用 `default_fonts`，避免引入 OFL-1.1 / Ubuntu-font-1.0 字体许可证（§10 人工独占）。只读参考项目（含 AGPLv3 的 `WarsmashModEngine`）不作代码复制。
- **分层依赖**：example 在 `w3-client` 内部向下依赖 `w3-core` / `w3-render` / `w3-app`（`freeze_f64`）/ `w3-net` 既有依赖，`w3-core` 不反向依赖任何上层 crate；不出现第二套呈现代码、第二份 `RtsMatch`、复制的事件循环；表现层不反写模拟状态（ADR 0002 §5）。owner = `client-owner`（可写 `crates/w3-client/` / `crates/w3-render/`，正好覆盖 example），配只读 `critic`，满足「一个 change 一个 owner」。
- **CI 边界**：`.github/workflows/ci.yml` 的 `ubuntu-latest` / `windows-latest` matrix 跑 `python tools/verify_all.py`，**只覆盖编译与 CPU 测试**（含 `w3-core/tests/determinism.rs`、`w3-render/tests/isolation.rs`）；Linux 侧 winit 平台后端已由 `linux-winit-platform-backend` 修好。CI 双平台只验证**工具链**跨平台（CODEBUDDY.md §2），**不得声称 CI 已验证 GPU 链路**，也不等于模拟内核确定性已达成。
- **人工验收**：GPU 冒烟清单为**人工项**，只能在有 GPU / 显示设备的桌面执行；无 GPU / headless 环境 MUST 明确报告无法执行、不静默跳过。清单未逐项勾选前不得归档。
- **规范边界**：本轮只写 `openspec/changes/playable-window-smoke/` 下四件套；不碰主 `openspec/specs/`、其它 change 目录（含归档目录）、`openspec/config.yaml`、`docs/`、`crates/`、`.codebuddy/`、`NOTICE.md`、`CODEBUDDY.md`、根配置；不运行 `cargo`、不 commit、不归档、不执行任何 git 远端命令。
