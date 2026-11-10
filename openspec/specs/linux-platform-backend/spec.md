# linux-platform-backend Specification

## Purpose
本规范定义 Linux 平台后端的构建配置契约：为 `crates/w3-render/Cargo.toml` 与 `crates/w3-client/Cargo.toml` 追加 `[target.'cfg(target_os = "linux")'.dependencies]` 条件依赖，仅在 `target_os = "linux"` 下启用 winit 的 `x11` 平台后端、解除 Linux 全工作空间的编译阻塞；Windows 侧既有 `[dependencies]` 的 `winit`（`rwh_06`）与 `egui-winit`（`default-features = false`）配置保持零影响，win32 后端仍由 winit 的 target cfg 自动生效。本 capability 是纯构建配置改动，不触碰任何 `.rs` 逻辑；`w3-core` 保持零改动、不新增权威状态，状态哈希字段集合、由 `SNAPSHOT_VERSION`（当前 3）定义的快照与帧序均不变，遵守 ADR 0002 确定性契约。CI 双平台跑同一 `python tools/verify_all.py` 只验证工具链跨平台，不等于 ADR 0002 模拟内核确定性已达成——该结论归 `cross-platform-golden-hash` change。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf / CI 双平台生成字节一致）。

## Requirements

### Requirement: Linux 目标声明 x11 winit 平台后端

`crates/w3-render/Cargo.toml` 与 `crates/w3-client/Cargo.toml` SHALL 各追加一段 `[target.'cfg(target_os = "linux")'.dependencies]`，其中 `winit = { version = "0.30", default-features = false, features = ["rwh_06", "x11"] }`；两份声明 MUST 与既有 `[dependencies]` 的 `winit` 保持同一版本约束（`0.30`），使 Cargo 在 `target_os = "linux"` 时对同 crate 同版本的 features 取并集得到 `rwh_06` + `x11`。该条件依赖 MUST NOT 出现在 `[dependencies]` 中，MUST NOT 启用 `wayland` / `wayland-dlopen` / `wayland-csd-adwaita` 及其装饰依赖（ADR 0001 依赖面约束、`design.md` D1 / D2）。

#### Scenario: Linux 上解析出 x11 后端

- **WHEN** 在 `target_os = "linux"` 上解析 `w3-render` 与 `w3-client` 的依赖图
- **THEN** 两者 `winit` 的 feature 并集为 `rwh_06` + `x11`，`default-features` 仍为 false，平台实现不再是「无任何后端」

#### Scenario: 不启用 wayland 装饰依赖

- **WHEN** 检查 Linux target 条件依赖的 features
- **THEN** 不含 `wayland` / `wayland-dlopen` / `wayland-csd-adwaita`，且未引入 `ab_glyph` 等服务端装饰依赖

### Requirement: Linux 工作空间可编译且测试全绿

在 `target_os = "linux"` 上，系统 SHALL 使 `cargo test --workspace` 成功编译并执行全部测试，MUST NOT 触发 `winit` 的 `compile_error!("The platform you're compiling for is not supported by winit")`；`python tools/verify_all.py` SHALL 在 Linux 输出 `Verification OK`（编码 / `cargo fmt --all -- --check` / `cargo test --workspace` / `openspec validate` / 资产扫描全部通过）。VM 实测基线：`rustc/cargo 1.99.1`、`protoc 31.1`、`openspec 1.3.1` 下 24 项测试全绿（CODEBUDDY.md §2 / §7）。

#### Scenario: Linux 编译不再阻塞

- **WHEN** 在 Linux VM 上执行 `cargo build --workspace` 与 `cargo test --workspace`
- **THEN** `winit 0.30.13` 成功编译、无 `compile_error!`、退出码为 0，全部测试通过

#### Scenario: Linux 统一门禁通过

- **WHEN** 在 Linux VM 上执行 `python3 tools/verify_all.py`
- **THEN** 五个门禁步骤全部 `[check]` 通过并输出 `Verification OK`，`cargo test` 不再退出 101

### Requirement: Windows 行为零影响

在 `target_os = "windows"` 上，`[target.'cfg(target_os = "linux")'.dependencies]` MUST 不生效；`w3-render` / `w3-client` 的 `[dependencies]` 中 `winit = { version = "0.30", default-features = false, features = ["rwh_06"] }` SHALL 保持原样，win32 后端仍由 winit 的 target cfg 自动生效（ADR 0003 P6 集成收尾）。Windows 的 winit feature 集 MUST 仍为 `rwh_06`、MUST NOT 被强制启用 `x11` 或拉入 `x11-dl` / wayland 相关依赖；`cargo test --workspace` 与 `python tools/verify_all.py` 在 Windows SHALL 保持与改动前一致的结果。

#### Scenario: Windows feature 不变化

- **WHEN** 在 Windows 上解析并构建 `w3-render` / `w3-client`
- **THEN** winit feature 集仍为 `["rwh_06"]`、win32 后端可用，依赖图未新增任何 Linux 平台后端条目

#### Scenario: Windows 门禁不回归

- **WHEN** 在 Windows 上执行 `python tools/verify_all.py`
- **THEN** 与改动前一样输出 `Verification OK`，`cargo fmt` / `cargo test` / `openspec validate` / 资产扫描无回归

### Requirement: 最小 feature 集与 egui-winit 不动

`egui-winit = { version = "0.37", default-features = false }` SHALL 保持不动；关闭其 default features 的理由是防止其 `clipboard` / `wayland` / `x11` / `links` 经 feature 统一重新开启 `winit` 的默认 feature（ADR 0005 §3 的 canonical 约束、`crates/w3-client/Cargo.toml` 现有注释）。系统 MUST NOT 把 `x11` 直接写进 `[dependencies]` 的 features（否则 Windows 亦被强制启用），也 MUST NOT 重新开启 `winit` 的 `default` feature 组；全仓 winit 依赖面 SHALL 维持「`rwh_06`（全平台）+ `x11`（仅 Linux target）」这一最小集合（`design.md` D2 / D3）。

#### Scenario: egui-winit 未触发 feature 回流

- **WHEN** 检查 `w3-client/Cargo.toml` 与 `cargo tree` 的 winit feature 解析结果
- **THEN** `egui-winit` 仍为 `default-features = false`，`winit` 的 `default` / `wayland` / `wayland-csd-adwaita` 均未被重新开启

#### Scenario: x11 不进全平台依赖

- **WHEN** 审阅两份 `Cargo.toml` 的 `[dependencies]` 段
- **THEN** `x11` 只出现在 `[target.'cfg(target_os = "linux")'.dependencies]` 中，`[dependencies]` 的 features 仍为 `["rwh_06"]`

### Requirement: 不新增权威状态、不改状态哈希与快照

本 change MUST NOT 修改 `crates/w3-core/` 的任何实现或依赖（ADR 0002 §5 / §8）；MUST NOT 新增任何权威状态字段——因此无需把任何新字段声明进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照。状态哈希字段集合、`first_difference()` 语义、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、无参 `RtsMatch::step()` 语义与快照 SHALL 全部不变、不 bump `SNAPSHOT_VERSION`。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（确认零改动）。本 change MUST NOT 声称「模拟内核跨平台确定性已达成」：Linux / macOS 仍属待验证项（CODEBUDDY.md §2 / §5、ADR 0002），该结论归 `cross-platform-golden-hash` change；`verify_all.py` 双平台只验证工具链跨平台。

#### Scenario: 内核零改动与确定性验收

- **WHEN** 复核分层依赖并执行 `cargo test -p w3-core --test determinism -- --nocapture`
- **THEN** `crates/w3-core/` 无 diff、不依赖上层 crate，确定性测试全绿，状态哈希字段集合与 `SNAPSHOT_VERSION = 3`（含 `defs_hash` / `content_hash`）不变

#### Scenario: 逐帧哈希序列不变

- **WHEN** 在应用本补丁前后对同一输入序列运行固定 30 Hz 步进
- **THEN** 逐帧 `compute_state_hash()` 序列逐位相同、`first_difference()` 仍返回 `None`，无新增权威状态字段进入哈希或快照

#### Scenario: 不误报跨平台确定性达成

- **WHEN** 记录 Linux 全绿与真实 UDP 双客户端 30 帧 hash 一致的验收结论
- **THEN** 结论只描述「工具链 / 构建跨平台」与「同一 VM 内两客户端逐帧一致」，不出现 ADR 0002 跨平台确定性已达成之表述

### Requirement: Cargo.lock 随变更提交且非主版本升级

`Cargo.lock` SHALL 随两份 `Cargo.toml` 一并提交，包含 Cargo 解析出的 Linux target 条件依赖（如 `x11-dl` 等）；`winit` 版本 MUST 保持 `0.30.13`（`Cargo.lock` 既有条目），MUST NOT 发生主版本升级。该变更定性为「新增 target 条件依赖、非主版本升级」，不属于 CODEBUDDY.md §10 的「依赖重大升级」（人工独占）范畴，但按 CODEBUDDY.md §6 / ADR 0001 SHALL 完成许可证与 GPLv3 兼容性检查、`Cargo.lock` 提交；若需第三方登记，由 `docs-owner` 在 `NOTICE.md` 处理（`design.md` D3）。

#### Scenario: 锁文件纳入提交

- **WHEN** 提交本 change 的实现
- **THEN** `git status` 显示 `Cargo.lock` 与两份 `Cargo.toml` 一并纳入，其新增条目仅为 Linux target 条件依赖，`winit` 版本仍为 `0.30.13`

#### Scenario: 依赖合规复核

- **WHEN** 检查新增 Linux 平台依赖
- **THEN** 已按 CODEBUDDY.md §6 核验许可证与 GPLv3 兼容、`Cargo.lock` 已提交，且无游戏资产 / 真实导出 JSON / 密钥入库

### Requirement: 双平台统一门禁与资产扫描不变

CI SHALL 继续以 `.github/workflows/ci.yml` 的 matrix（`ubuntu-latest` / `windows-latest`）跑同一单行入口 `python tools/verify_all.py`；`dtolnay/rust-toolchain@stable` + `rustfmt`、`arduino/setup-protoc@v3 version 31.1`、Python 3.12、Node 20、`openspec 1.3.1` 的工具链配置 MUST 保持不变。`prost` 生成代码在 Windows / Linux 双平台 SHALL 由同版本 `protoc 31.1` 保证字节一致（ADR 0005 §3、CODEBUDDY.md §4）。资产扫描（`verify_all.py::asset_scan` 的 BANNED_SUFFIXES / BANNED_DIRS）SHALL 不变，仓库 MUST NOT 因本 change 引入 MPQ 派生数据 / 导出 JSON / PNG / GLB / 音效（ADR 0001）。

#### Scenario: 双平台 CI 全绿

- **WHEN** CI matrix 在 `ubuntu-latest` 与 `windows-latest` 各执行 `python tools/verify_all.py`
- **THEN** 两端均输出 `Verification OK`，Linux 由原先 `cargo test` 退出 101 转为通过，Windows 无回归

#### Scenario: prost 生成字节与资产扫描

- **WHEN** 比较双平台构建产物并运行资产扫描
- **THEN** `prost` 生成代码字节一致（`protoc 31.1`），资产扫描仍拒绝 `*.mpq` / `*.blp` / `*.mdx` 等后缀与导出目录，输出 `[check] asset scan OK`
