## Why

> 路线图阶段（与 `gameplay-command-wire-format` 的 (f) 口径一致）：本 change 按 `openspec/config.yaml` rules.proposal 的 P0–P5 阶段枚举核对，**不新增玩法阶段**；相对 [ADR 0003](../../../docs/decisions/0003-scope.md) 分期表已有的 P6「集成收尾」行，标注为 **「P6 集成收尾的后续补齐」**，不单列独立 P7，`config.yaml` rules.proposal 的「P0-P5」枚举保持不动。相关 ADR：[ADR 0001](../../../docs/decisions/0001-license.md)（资产与法律）、[ADR 0002](../../../docs/decisions/0002-determinism.md)（确定性契约）、[ADR 0003](../../../docs/decisions/0003-scope.md)（范围与分期）、[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)（冻结坐标 / 拒绝 `f64`）、[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md)（CI 双平台生成字节一致）。

P0–P5 已归档、P6 可玩客户端集成层实施中。编排者在 Debian 12 VM（`rustc/cargo 1.99.1`、`protoc 31.1`、`openspec 1.3.1`）上验证 [CODEBUDDY.md §2](../../../CODEBUDDY.md) 的「工具链跨平台」时发现一个**真实编译阻塞**：`crates/w3-render/Cargo.toml:18` 与 `crates/w3-client/Cargo.toml:23` 均为 `winit = { version = "0.30", default-features = false, features = ["rwh_06"] }`（当初为规避 Windows 本机离线缓存缺失的 `ab_glyph` / wayland / x11 装饰依赖而设，`egui-winit` 同样 `default-features = false`）。Windows 上 win32 后端由 winit 的 target cfg 自动生效，故不暴露；但在 Linux 上 `winit 0.30.13` 关闭 default 后**没有任何平台后端**，`src/platform_impl/mod.rs` 末尾的 `compile_error!("The platform you're compiling for is not supported by winit")` 被触发，VM 上 `python3 tools/verify_all.py` 的 `cargo test --workspace` 因此退出 101。最小修法（给两个 crate 各追加一段 `[target.'cfg(target_os = "linux")'.dependencies]` 的 `winit = { version = "0.30", default-features = false, features = ["rwh_06", "x11"] }`）已在 VM 副本验证：`cargo test --workspace` 24 项全绿、`python3 tools/verify_all.py` → `Verification OK`、真实 UDP 双客户端 30 帧逐帧 hash 序列完全一致。提案阶段该补丁**只存在于 VM 副本**（`M Cargo.lock` 与两个 `Cargo.toml`）、本机仓库零改动；**实施后状态**：现已由 `client-owner` 在本机 `crates/w3-render/Cargo.toml` / `crates/w3-client/Cargo.toml` / `Cargo.lock` 正式落地（非零改动），VM 重新同步后 `python3 tools/verify_all.py` → `Verification OK`（24 项全绿）、真实 UDP 双客户端 30 帧逐帧 hash 一致，本机 `python tools/verify_all.py` → `Verification OK`（4 个 active change 全 valid）。本 change 已把这一「构建 / 呈现层平台适配」固化为规范，`/opsx:apply` 已完成。

## What Changes

- 在 `crates/w3-render/Cargo.toml` 与 `crates/w3-client/Cargo.toml` **各追加一段** `[target.'cfg(target_os = "linux")'.dependencies]`：`winit = { version = "0.30", default-features = false, features = ["rwh_06", "x11"] }`；既有 `[dependencies]` 的 `winit` 行保持不变（Cargo feature 合并语义见 `design.md` D2）。
- 选择 `x11` 单一 Linux 后端：MUST NOT 启用 `wayland` / `wayland-dlopen` / `wayland-csd-adwaita`，避免重新拉入 `ab_glyph` 等装饰依赖（见 `design.md` D1）。
- `egui-winit` 的 `default-features = false` **保持不动**（见 `design.md` D3）。
- `Cargo.lock` MUST 随变更提交：新增 Linux target 条件依赖（`x11-dl` 等），`winit` 版本仍为 `0.30.13`、**非主版本升级**（对 CODEBUDDY.md §10 的定性见 `design.md` D3）。
- **BREAKING = 无（纯构建配置）**：不改任何 Rust 源码、不改公开 API、不改协议、不 bump `PROTOCOL_VERSION`、不 bump `SNAPSHOT_VERSION`（当前 **3**）、不新增权威状态字段；表现层只读边界（ADR 0002 §5）不变。

## Capabilities

### New Capabilities

- `linux-platform-backend`: Linux 平台窗口后端适配契约——两个 crate 的 target 条件依赖与最小 winit feature 集、Windows 行为零影响、`verify_all.py` 双平台门禁、`w3-core` 与确定性契约零改动、`Cargo.lock` 提交与资产合规。

### Modified Capabilities

（无。）**刻意只新增独立 capability、不产生任何 MODIFIED delta**：本 change 是纯构建配置适配，`3d-rendering` / `deterministic-simulation` 等既有主 spec 的 requirement 行为均不变；且 `p6-playable-client` 仍持有 `3d-rendering` / `lockstep-networking` 的 active delta，`gameplay-command-wire-format` 与 `cross-platform-golden-hash` 同为 active change，两条 change 同时写同一 capability 会在归档时冲突。与 (h) 口径一致，理由见 `design.md` D7。本轮不碰主 `openspec/specs/`、不改其它 change 的 spec 实质。

## Impact

- **确定性契约（ADR 0002）**：构建 / 呈现层平台适配，**不碰 `w3-core`、不新增权威状态、不动状态哈希 / 快照 / 帧序**；`RtsMatch::step()` 无参语义不变；状态哈希字段集合、`first_difference()` 语义、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照全部不变、不 bump。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（确认零改动）。**不声称跨平台确定性已达成**：那属于 `cross-platform-golden-hash` change；[CODEBUDDY.md §2](../../../CODEBUDDY.md) 明确 CI 双平台只验证工具链跨平台，ADR 0002 的 Linux / macOS 确定性仍属待验证项（见 `design.md` D4）。
- **资产与法律（ADR 0001）**：不提交任何 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效；`verify_all.py::asset_scan` 不变。新增 target 条件依赖按 [CODEBUDDY.md §6](../../../CODEBUDDY.md) 检查许可证与 GPLv3 兼容性；若需第三方登记由 `docs-owner` 在 `NOTICE.md` 处理（本 change 不自行编写 `NOTICE.md`）。四个只读参考项目（含 AGPLv3 的 `WarsmashModEngine`）不作代码复制。
- **分层依赖**：`w3-core` 零改动、零依赖，不反向依赖 `w3-render` / `w3-client`；表现层不反写模拟状态；`w3-content` 的 `f64` 冻结边界（ADR 0004）不受影响；`w3-render` / `w3-client` 的向下依赖与无环结构不变。DIP / ISP / OCP：仅追加 target 条件依赖，既有 `[dependencies]` 不动，Windows 侧依赖图零变化（见 `design.md` D2 / D3）。
- **`Cargo.lock` 与 CI 影响**：`Cargo.lock` 新增 Linux target 条件依赖（`x11-dl` 等）并 MUST 提交；`.github/workflows/ci.yml` 的 matrix（`ubuntu-latest` / `windows-latest`）与单行 `python tools/verify_all.py` 入口不变，Linux 由编译失败转为全绿、Windows 仍全绿；`prost` 生成字节由双平台 `protoc 31.1` 保证一致（ADR 0005 §3）；`openspec validate` 覆盖新增 active change（见 `design.md` D5）。
- **owner 归属**：单一 owner = **`client-owner`**（可写 `crates/w3-client/` / `crates/w3-render/`，正好覆盖两个改动文件），配只读 **`critic`**；`sim-owner` 管 `w3-core`，本 change 不碰（CODEBUDDY.md §8 / §9，见 `design.md` D6）。
- **规范边界**：本轮只写 `openspec/changes/linux-winit-platform-backend/` 下四件套；不碰主 `openspec/specs/`、其它 change 目录、`crates/`、`.codebuddy/agents/`、`NOTICE.md`、`CODEBUDDY.md`、根配置；不运行 `cargo`、不 commit、不归档、不执行任何 git 远端命令。
