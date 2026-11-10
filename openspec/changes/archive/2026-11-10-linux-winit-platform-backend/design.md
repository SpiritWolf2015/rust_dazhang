## Context

`rust_dazhang` 是 GPLv3、Rust edition 2024 虚拟 workspace（ADR 0003）。P0–P5 已归档、P6 `p6-playable-client` 实施中；主 `openspec/specs/` 共 7 个 capability。CODEBUDDY.md §2 / §4 规定 CI 以 `.github/workflows/ci.yml` 的 matrix（`ubuntu-latest` / `windows-latest`）跑同一单行入口 `python tools/verify_all.py`，并明确「CI 双平台只验证**工具链**跨平台，不等于 ADR 0002 模拟内核确定性已达成」。

当前状态（已由源码 / 锁文件复核）：

- `crates/w3-render/Cargo.toml:18`、`crates/w3-client/Cargo.toml:23` 均为 `winit = { version = "0.30", default-features = false, features = ["rwh_06"] }`；`w3-client` 的 `egui-winit = { version = "0.37", default-features = false }`。注释说明关闭 default 是为规避 Windows 本机离线缓存缺失的 `ab_glyph` / wayland / x11 装饰依赖。
- `Cargo.lock` 中 `winit = 0.30.13`；当前**无** `x11-dl` / `ab_glyph` / `wayland-csd-adwaita` 条目（Linux target 依赖将是新增），`xkbcommon-dl` 已在锁文件内。
- winit 0.30 的 `default = ["rwh_06", "x11", "wayland", "wayland-dlopen", "wayland-csd-adwaita"]`；`src/platform_impl/mod.rs` 末尾对「未命中任一平台 cfg」触发 `compile_error!("The platform you're compiling for is not supported by winit")`。Windows 的 win32 后端由 target cfg 自动生效，故关闭 default 在 Windows 无碍；Linux 关闭 default 且不加任何后端则必然编译失败。
- 编排者在 Debian 12 VM（本机 `e1ba8a4` 干净工作区的 tar 快照，`rustc/cargo 1.99.1`、`protoc 31.1`、`openspec 1.3.1`、`python3 3.11.2`、`node 18`）上实测 `python3 tools/verify_all.py` 的 `cargo test` 退出 101。最小修法（两份 `Cargo.toml` 各追加 `[target.'cfg(target_os = "linux")'.dependencies]` 的 `winit` `["rwh_06", "x11"]`）在 VM 副本上：`cargo test --workspace` 24 项全绿、`verify_all.py` → `Verification OK`、真实 UDP 双客户端 30 帧逐帧 hash 序列完全一致。
- 提案阶段**该补丁只在 VM 副本**（`M Cargo.lock` + 两个 `Cargo.toml`）、本机仓库零改动；**实施后状态**：现已由 `client-owner` 在本机两份 `Cargo.toml` 与 `Cargo.lock` 正式落地（非零改动），VM 重新同步后 `python3 tools/verify_all.py` → `Verification OK`（24 项全绿）、真实 UDP 双客户端 30 帧逐帧 hash 一致，`/opsx:apply` 已完成。

干系人：`client-owner`（唯一实现 owner）、只读 `critic`、`spec-steward`（规范）、编排者（同步 VM / 人工 push / 归档）。相关 ADR：0001 / 0002 / 0003 / 0004 / 0005。

## Goals / Non-Goals

**Goals:**

- 消除 Linux 上 `winit 0.30.13` 因「零平台后端」导致的编译阻塞，使 `cargo test --workspace` 与 `python tools/verify_all.py` 在 Linux 全绿。
- 保持 Windows 行为与依赖图**零影响**（win32 后端 continue 生效、feature 集仍为 `rwh_06`）。
- 只做**构建 / 呈现层平台适配**：不碰 `w3-core`、不新增权威状态、不动状态哈希 / 快照 / 帧序、`step()` 无参语义不变。
- 新增独立 capability `linux-platform-backend`，不产生 `3d-rendering` / `deterministic-simulation` 等 MODIFIED delta，避免与 active change 争用。
- 明确 `Cargo.lock` 变更的定性（新增 target 条件依赖、非主版本升级）、提交与许可证合规。

**Non-Goals:**

- **不声称 ADR 0002 跨平台确定性已达成**：Linux / macOS 仍属待验证项，由 `cross-platform-golden-hash` change 承载（CODEBUDDY.md §2 / §5）。
- 不启用 `wayland` 后端、不引入 `ab_glyph` / `wayland-csd-adwaita` 装饰依赖；纯 Wayland / 无 DISPLAY 的 headless 会话不在本 change 范围。
- 不改 `egui-winit`、不改 `w3-render` / `w3-client` 的 Rust 源码、不改 `rwh_06` 全平台 feature、不改 CI 配置 / `tools/` / 根配置 / `NOTICE.md` / `CODEBUDDY.md`。
- 不 bump `PROTOCOL_VERSION`（ADR 0005）、不 bump `SNAPSHOT_VERSION`（当前 3）、不改 protobuf schema / prost 生成逻辑。
- 不新增路线图阶段（沿用 (f) 口径「P6 集成收尾的后续补齐」）。

## Decisions

每个决策统一给出：**决策** / **备选方案（Alternatives considered）** / **关联 ADR 0002 与跨平台确定性影响** / **架构原则与分层依赖**。

### D1：Linux 平台后端选 `x11`（单后端）

**决策**：在 `[target.'cfg(target_os = "linux")'.dependencies]` 只启用 `winit` 的 `x11` feature（与全平台 `rwh_06` 取并集），MUST NOT 启用 `wayland` / `wayland-dlopen` / `wayland-csd-adwaita`。理由：winit 的 `x11` 后端通过 `x11-dl` 在运行时动态加载系统库，编译期依赖面最小；而 `wayland` 组会拉入 `wayland-csd-adwaita` → `ab_glyph` 等装饰依赖，正是两份 `Cargo.toml` 注释当初规避的对象。`x11` 在 X11 与 XWayland 会话下均可用，足以让 `cargo test --workspace` 编译与 `verify_all.py` 通过。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **`x11` 单后端** | **采纳** | 依赖面最小（`x11-dl` 动态加载）、不触装饰依赖、VM 已实测 `cargo test --workspace` 24 项全绿 + UDP 双客户端逐帧 hash 一致。 |
| `wayland` 单后端 | 否决 | 会拉 `wayland-csd-adwaita` / `ab_glyph`（正是规避对象），Windows 离线缓存与依赖合规成本上升；纯 Wayland 原生支持应另开 change 与 ADR 复核。 |
| `x11` + `wayland` 双后端 | 否决 | 叠加两类依赖面、重新引入装饰依赖；第一版只需「能编译」，双后端属过度设计（YAGNI / ISP）。 |
| 恢复 `winit` `default` feature 组 | 否决 | `default` 含 `wayland` / `wayland-csd-adwaita`，等于撤销当初规避决策，Windows 亦可能受牵连。 |
| 维持现状、不改 | 否决 | Linux 必然触发 `compile_error!`，CI `ubuntu-latest` 恒红，与 CODEBUDDY.md §4 双平台门禁直接冲突。 |

**关联 ADR 0002 与跨平台确定性影响**：本决策只改变**编译期平台后端**，不改变 `w3-core` 的任何字节：权威状态、`SimFixed`、整数帧、命令排序、FNV-1a 状态哈希与 `SNAPSHOT_VERSION`（当前 3）快照一律不变（ADR 0002 §2/§3/§6/§7）。`x11-dl` 动态加载发生在呈现层进程启动阶段，不进入模拟循环、不进入 `step()`、不进权威状态。故对「跨平台确定性」既无正向也无负向贡献——**不因 Linux 能编译就声称 ADR 0002 已达成**；Linux / macOS 逐帧哈希 golden 验证由 `cross-platform-golden-hash` change 承担。

**架构原则与分层依赖**：DIP / ISP——只为 `w3-render` / `w3-client` 两个上层 crate 追加最小平台接口依赖，`w3-core` 零依赖、不反向依赖上层；CARP / OCP——用「追加条件依赖」而非改动既有 `[dependencies]` 扩展平台支持；SRP——后端选择是构建配置关注点，不混入渲染逻辑。表现层只读边界（ADR 0002 §5）不变。

### D2：用 `[target.'cfg(target_os = "linux")'.dependencies]` 条件依赖

**决策**：把 `x11` **只**放进 `[target.'cfg(target_os = "linux")'.dependencies]`，与既有 `[dependencies]` 的 `winit`（`rwh_06`）保持同一版本 `0.30`。Cargo 对同 crate 同版本在 target 匹配时 features 取**并集**：Linux 得 `rwh_06` + `x11`，Windows 因 cfg 不匹配只得 `rwh_06`（win32 后端由 winit 自身 target cfg 自动生效）。这样 Windows 零影响，且无需条件编译宏或平台分支源码。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **target 条件依赖（`cfg(target_os = "linux")`）** | **采纳** | Cargo 原生语义、features 取并集、Windows 完全不解析该段；改动只在两份 `Cargo.toml`，无源码分支。 |
| 把 `x11` 直接加进 `[dependencies]` 的 features | 否决 | Windows 也会被强制启用 `x11` 并可能拉 `x11-dl` / 相关依赖，破坏「规避装饰依赖」初衷、Windows 行为非零影响。 |
| 自定义 Cargo `features` + 源码 `#[cfg]` 门控 | 否决 | 需要写 Rust 源码、扩大改动面、引入条件编译分支，违背「纯构建配置」与 OCP。 |
| 用 `cfg!` 宏 / 运行时探测后端 | 否决 | 编译期 `compile_error!` 无法靠运行时逻辑规避；且属表现层运行时逻辑，超出范围。 |
| 分平台维护两套 `Cargo.toml` / 生成脚本 | 否决 | 违反 CODEBUDDY.md §2 单一跨平台入口、增加维护成本与漂移风险。 |

**关联 ADR 0002 与跨平台确定性影响**：feature 并集是 Cargo 依赖解析期行为，产物只影响链接到 `w3-render` / `w3-client` 的平台库，不进入 `w3-core` 编译单元、不进入权威状态或状态哈希（ADR 0002 §5/§6）。不在模拟路径引入任何 `cfg` 分支，因此不会造成「同输入不同平台不同状态」的风险；但同样**不构成跨平台确定性的验证**——它只是让 Linux 具备编译前提。

**架构原则与分层依赖**：OCP / DIP——通过声明式 target 条件扩展平台支持，既有依赖声明与上层代码零修改；LoD——`w3-core` 不感知平台后端；分层依赖方向不变（上层依赖内核与呈现库，内核零依赖）。无环结构（ADR 0004 的 `w3-app` 冻结边界）不受影响。

### D3：影响面——两个 crate、`egui-winit` 不动、`Cargo.lock` 与 §10 定性

**决策**：

- 改且只改 `crates/w3-render/Cargo.toml` 与 `crates/w3-client/Cargo.toml`（各追加一段 Linux target 条件依赖，中文注释说明来由）；不改任何 `.rs` 源码、不改 `rwh_06` 全平台 feature。
- **`egui-winit` 的 `default-features = false` 不动**：其 default（`clipboard` / `wayland` / `x11` / `links`）经 feature 统一会重新开启 `winit` 的 default feature 组。保持关闭即可维持「`rwh_06`（全平台）+ `x11`（仅 Linux target）」这一最小集；本 change 的 `x11` 由我们显式在 target 段声明，无需也不应借 `egui-winit` 打开。
- `Cargo.lock` 会新增 Linux 平台依赖（`x11-dl` 等，当前锁文件确认缺失）；`winit` 版本保持 `0.30.13`。定性：**新增 target 条件依赖、非主版本升级**，不属于 CODEBUDDY.md §10「依赖重大升级」（人工独占）范畴；但按 CODEBUDDY.md §6 / ADR 0001 MUST 完成许可证与 GPLv3 兼容性检查、**`Cargo.lock` MUST 提交**；第三方登记如需则由 `docs-owner` 在 `NOTICE.md` 处理（本 change 不写 `NOTICE.md`）。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **只改两个 `Cargo.toml` + 提交 `Cargo.lock`** | **采纳** | 最小改动面、单一 owner（`client-owner`）即可覆盖、VM 已验证有效。 |
| 同步把 `egui-winit` 改为 `default-features = true` 或补 `x11` | 否决 | 会经 feature 统一重新拉入 `wayland` / 装饰依赖，撤销当初规避决策；Linux 后端已由 target 段显式提供，无需动它。 |
| 三个 crate 都加（含 `w3-app` / `w3-net`） | 否决 | 其它 crate 不依赖 `winit`，无编译阻塞；扩大改动面违反 SRP / 最小影响原则。 |
| 把新增依赖判为 §10「依赖重大升级」交人工独占 | 否决 | 无版本号主 / 次升级、只是既有 `winit 0.30.13` 在 Linux target 下解析出条件依赖；按 §6 常规流程即可，过度定级会阻塞工具链修复。若编排者坚持人工复核，见 Open Questions。 |
| 不提交 `Cargo.lock` | 否决 | 违反 CODEBUDDY.md §6「`Cargo.lock` 提交」与 ADR 0001；会导致双平台解析版本漂移。 |

**关联 ADR 0002 与跨平台确定性影响**：`Cargo.lock` 变更只影响依赖解析，不进入 `w3-core` 编译或模拟运行时；`w3-core` 零改动、零依赖，确定性验收 `crates/w3-core/tests/determinism.rs` MUST 重跑且全绿。Linux 平台库是呈现层依赖，不进状态哈希 / 快照（ADR 0002 §5/§6/§7）。对跨平台确定性无影响，也不据此声称达成。

**架构原则与分层依赖**：SRP——构建配置改动集中在两个上层 crate；ISP——不为不需要 `winit` 的 crate 追加依赖；CARP / LoD——依赖方向保持 `w3-client → w3-render → w3-core`，`w3-core` 不反向依赖（ADR 0002 §5 硬红线）。锁定 `Cargo.lock` 保证双平台解析可复现，呼应 ADR 0005 §3「锁定版本并写入 `Cargo.lock`」的纪律（虽本 change 不涉及 prost）。

### D4：与 ADR 0002 的关系——构建 / 呈现层平台适配，确定性边界零改动

**决策**：明确本 change 是**构建 / 呈现层平台适配**，不是模拟内核变更：`w3-core` 零改动、零依赖；不新增权威状态字段（故无字段需进入状态哈希或快照）；状态哈希字段集合、`first_difference()` 语义、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、无参 `RtsMatch::step()`、由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照全部不变、不 bump。表现层不反写模拟状态，`w3-content` 的 `f64` 冻结边界（ADR 0004）不变。**不声称跨平台确定性已达成**：CODEBUDDY.md §2 的「CI 双平台」只验证工具链，ADR 0002 的 Linux / macOS 逐帧 golden hash 是待验证项，归 `cross-platform-golden-hash` change；VM 上「真实 UDP 双客户端 30 帧逐帧 hash 一致」是同一 VM 内两进程一致，不是跨平台结论。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **显式声明「确定性边界零改动 + 不声称达成」** | **采纳** | 与 ADR 0002 §8 复核点「在非 Windows 平台首次实跑」一致地留白，避免两份规范打架；`spec-steward` 职责要求 spec 引用 ADR 编号、ADR 记「为什么」spec 记「做什么」。 |
| 借本 change 顺带落地跨平台 golden hash | 否决 | 属不同 capability / 不同 owner（`sim-owner`），已有 `cross-platform-golden-hash` change；混写会违反「一个 change 一个 owner」并扩大范围。 |
| 在 ADR 0002 追加复核记录 | 否决 | 本 change 未改变 ADR 0002 的任何决策，只行使「呈现层只读边界」；追加记录由真正的 golden hash 验证 change 承担，避免 ADR 与 spec 重复维护。 |
| 声称「Linux 全绿 = 确定性已验证」 | 否决 | 直接违反 CODEBUDDY.md §2 / §5 与 ADR 0002 §8；工具链跨平台 ≠ 模拟内核确定性。 |

**关联 ADR 0002 与跨平台确定性影响**：这是本决策的本体。本 change 对 ADR 0002 的所有硬红线（定点数学、整数帧、命令排序、状态哈希、快照、状态所有权、显式失败）**零触碰**；对「跨平台确定性」既不推进也不否定，仅移除 Linux 的编译前提阻塞，使后续 `cross-platform-golden-hash` 得以在同一 VM 上实跑。

**架构原则与分层依赖**：唯一状态所有者仍是 `RtsMatch`（ADR 0002 §5）；DIP——表现层依赖内核视图、内核不依赖呈现层；「显式失败不静默降级」硬红线保持（winit 编译期 `compile_error!` 是显式失败，本 change 是补齐后端而非加降级垫片）。正确性 / 确定性优先于便利性。

### D5：CI 与门禁——双平台 `verify_all.py`、protoc 31.1、资产扫描不变

**决策**：`.github/workflows/ci.yml` 与 `tools/verify_all.py` **零改动**；matrix `ubuntu-latest` / `windows-latest` 继续跑同一单行 `python tools/verify_all.py`。验收口径：Linux 由 `cargo test` 退出 101 转为全绿、Windows 保持全绿；两端 `Verification OK`。`prost` 生成字节一致由 ADR 0005 §3 的双平台 `arduino/setup-protoc@v3 version 31.1` 保证（本 change 不碰 `.proto` / `build.rs` / prost 版本，故无新增生成差异风险）；`verify_all.py::asset_scan` 的 `BANNED_SUFFIXES` / `BANNED_DIRS` 不变，资产合规边界（ADR 0001）不变。`verify_all.py` 会遍历所有 active change 跑 `openspec validate`，因此本 change 自身 MUST 通过 `openspec validate --strict`。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **CI / 门禁零改动，仅修依赖声明** | **采纳** | 阻塞根因在 `Cargo.toml`；CI 已具备双平台能力，改 CI 属错配且违反「不碰根配置」边界。 |
| 给 Linux job 单独加 `x11` feature 参数或改 `verify_all.py` | 否决 | 会把平台差异塞进入口脚本，违反 CODEBUDDY.md §2「平台差异由 `verify_all.py` 内部 `os.name` 判断、不维护两套」；且 target cfg 已自动处理。 |
| 改 protoc 版本 / 加生成字节校验脚本 | 否决 | 与本 change 无关，ADR 0005 §3 已用 31.1 锁定；扩面无收益。 |
| 放宽资产扫描以容纳新依赖 | 否决 | 违反 ADR 0001；新增依赖后缀不在禁用清单，无需放宽。 |

**关联 ADR 0002 与跨平台确定性影响**：门禁里的 `cargo test --workspace` 包含 `crates/w3-core/tests/determinism.rs`（ADR 0002 §8）；Linux 首次跑通该测试只是让既有验收在 Linux 可执行，**不等于 ADR 0002 跨平台确定性结论**——同机同 rustc 的通过不构成跨 CPU / 跨 OS golden hash。措辞 MUST 保持「工具链跨平台」口径。

**架构原则与分层依赖**：SRP——门禁入口单一、`os.name` 内部判平台；不引入平台专属 `.sh` / `.ps1`（CODEBUDDY.md §2）；分层依赖不变。`prost` 生成字节一致属 ADR 0005 §3 的既有纪律，本 change 行使而非修改。

### D6：owner 归属——`client-owner` 单一 owner + 只读 `critic`

**决策**：单一 owner = **`client-owner`**（CODEBUDDY.md §9 可写 `crates/w3-client/` / `crates/w3-render/`，正好覆盖两个改动文件），配只读 **`critic`**（复审确定性 / 所有权 / 许可证 / 编码 / 资产入库 / rustfmt）。`sim-owner` 管 `w3-core`，本 change 不碰，故不介入；`spec-steward` 只写 `openspec/changes/linux-winit-platform-backend/` 四件套（必要时 `docs/decisions/`，本 change 判定无需改 ADR，见 D4）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 `critic`；`mode` 不使用 `bypassPermissions`。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **`client-owner` 单 owner + 只读 `critic`** | **采纳** | 改动文件全部落在其可写范围，边界清晰，符合 §8 / §9。 |
| `net-owner` / `content-owner` / `sim-owner` 介入 | 否决 | 本 change 不碰 `crates/w3-net` / `crates/w3-relay` / `crates/w3-content` / `crates/w3-core`，介入即越权、违反「一个 change 一个 owner」。 |
| 由 `spec-steward` 直接改 `Cargo.toml` | 否决 | 超出 `spec-steward` 可写范围（`openspec/` / `docs/decisions/`），且本轮明确「不要碰 `crates/`」。 |
| 无 `critic` 直接提交 | 否决 | 违反 §9 串联（`/opsx:apply` → `critic` → 本地 commit）。 |

**关联 ADR 0002 与跨平台确定性影响**：`critic` 的职责包含复跑 `cargo fmt --all -- --check` 与确定性审查（ADR 0002）；本 change 无 Rust 源码改动，`critic` 重点核验「`w3-core` 零 diff、`determinism.rs` 全绿、无权威状态 / 哈希 / 快照变化、不误报跨平台确定性达成」。owner 划分本身不影响任何运行时状态。

**架构原则与分层依赖**：SRP / 所有权边界——每个 crate 有唯一实现所有者，避免并发写冲突；`critic` 只读，符合 CODEBUDDY.md §9；中文 agent 定义不在本轮可写范围，不新建 / 不改动 `.codebuddy/agents/`。

### D7：spec delta 落点——只新增独立 capability，不产生 MODIFIED delta

**决策**：只新增 `specs/linux-platform-backend/spec.md`（全部 ADDED Requirements），MUST NOT 产生 `3d-rendering` / `deterministic-simulation` / 其它主 spec 的 MODIFIED delta。理由：本 change 是纯构建配置适配，既有主 spec 的 requirement 行为不变；`p6-playable-client` 仍持有 `3d-rendering` / `lockstep-networking` 的 active delta，`gameplay-command-wire-format` 与 `cross-platform-golden-hash` 同为 active change，两条 change 同写同一 capability 会在归档时冲突。与 `gameplay-command-wire-format` 的 (h) 口径一致。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **新增独立 capability `linux-platform-backend`** | **采纳** | 契约边界清晰、归档时新增主 spec、与 active change 零争用。 |
| 给 `3d-rendering` 写 MODIFIED delta | 否决 | requirement 行为未变；且与 P6 的 active delta 争用，归档冲突。 |
| 给 `deterministic-simulation` 写 MODIFIED delta（声明「不变」） | 否决 | 同上，且与 `cross-platform-golden-hash` 潜在冲突；「不变」用 ADDED 里的「零改动」requirement 表达即可。 |
| 不写 spec、只写 design / tasks | 否决 | 违反 schema `spec-driven` 的 artifact 顺序与 `openspec validate` 要求。 |

**关联 ADR 0002 与跨平台确定性影响**：把「确定性边界零改动」写进 ADDED requirement（spec「不新增权威状态、不改状态哈希与快照」），使 ADR 0002 的约束在 spec 层可验收，而非靠 MODIFIED 改写既有 requirement；不引入任何新的权威状态概念，故无状态哈希 / 快照字段集合变化、不 bump `SNAPSHOT_VERSION`（当前 3）。对跨平台确定性无影响、不声称达成。

**架构原则与分层依赖**：SRP / ISP——一个 capability 只描述「Linux 平台后端适配」这一关注点；主 specs 是 source of truth（CODEBUDDY.md §8），本轮不碰主 `openspec/specs/`，归档时才由工具吸收新增 capability。

## Risks / Trade-offs

- **[Risk]** `x11` + `x11-dl` 纯动态加载，在无 X server / 无 DISPLAY 的 headless Linux 上运行时创建窗口会失败 → **Mitigation**：本 change 的验收只要求 `cargo test --workspace` 编译与逻辑测试（测试不创建真实窗口）；运行时开窗能力属 P6 集成层 / 后续 change，列入 Non-Goals 与 Open Questions，由 CODEBUDDY.md §3 英文结构化日志显式报告、不静默降级。
- **[Risk]** feature 统一若被 `egui-winit` 或其它依赖重新开启 `winit` default，会重新拉入 `ab_glyph` / `wayland-csd-adwaita` → **Mitigation**：D3 明确 `egui-winit` 不动；tasks 包含 `cargo tree -e features -i winit` / 依赖图复核，`critic` 复审最小 feature 集。
- **[Risk]** `Cargo.lock` 新增 Linux 平台依赖后，Windows 离线缓存解析差异或双平台版本漂移 → **Mitigation**：`winit` 锁定 `0.30.13`、`Cargo.lock` MUST 提交、CI 双平台 `verify_all.py` 复核；按 CODEBUDDY.md §6 检查许可证与 GPLv3 兼容性。
- **[Risk]** 把「Linux 全绿 + 同一 VM 内 UDP 双客户端 30 帧 hash 一致」误读为 ADR 0002 跨平台确定性已达成 → **Mitigation**：D4 / spec 明确「工具链跨平台」口径、不误报；`cross-platform-golden-hash` 为独立 change，`critic` 复核措辞。
- **[Risk]** 提案阶段 VM 副本补丁与规范不同步（当时本机仓库零改动）→ **Mitigation**：tasks 含「重新同步 VM 干净快照」步骤，先让本机与 VM 改动一致再复跑门禁，绝不把 VM 补丁当已入库实现；**实施后状态**：该修复已由 `client-owner` 在本机正式落地、VM 已重新同步并复跑 `python3 tools/verify_all.py` → `Verification OK`（24 项全绿），风险已关闭。
- **[Risk]** `prost` 生成字节在双平台不一致（间接影响 ADR 0005）→ **Mitigation**：CI 固定 `protoc 31.1`，本 change 不碰 `.proto` / `build.rs` / prost 版本；如出现差异属既有门禁范畴，另立 change 处理。
- **[Risk]** 归档时与 `p6-playable-client` / `gameplay-command-wire-format` / `cross-platform-golden-hash` 争用 capability → **Mitigation**：D7 只写新增 capability 的 ADDED，零 MODIFIED delta，归档冲突面为零；归档顺序见 Open Questions。
- **[Risk]** 纯构建配置改动被误当作「依赖重大升级」触发 CODEBUDDY.md §10 人工独占流程 → **Mitigation**：D3 给出定性（新增 target 条件依赖、非主版本升级）；若编排者坚持人工复核，见 Open Questions，AI 不擅自执行远端 / 升级动作。

## Migration Plan

纯构建配置变更，无数据迁移、无协议 / 状态 / 快照迁移、无兼容垫片（BREAKING = 无）。步骤（与 `tasks.md` 一致）：

1. 本机干净工作区（`e1ba8a4`）由 `client-owner` 在两份 `Cargo.toml` 追加 Linux target 条件依赖 + 中文注释（D1 / D2 / D3）。
2. 本机 `cargo build --workspace` / `cargo test --workspace` 复核 Windows 零影响；由 Cargo 生成 / 更新 `Cargo.lock`（新增 `x11-dl` 等，`winit` 保持 `0.30.13`），按 CODEBUDDY.md §6 核验许可证。
3. **重新同步 VM**（VM 现有补丁仅验证用、不入库），使 VM 与本机改动一致；VM 复跑 `python3 tools/verify_all.py` → `Verification OK`，并做真实 UDP 双客户端 30 帧逐帧 hash 实跑（人工）。
4. Windows 复跑 `python tools/verify_all.py` → `Verification OK`；CI matrix 双平台以 `verify_all.py` 为最终门禁。
5. 门禁收尾：`cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`、重跑 `crates/w3-core/tests/determinism.rs`（确认 `w3-core` 零改动）；只读 `critic` 复审。
6. 本地 commit（Conventional Commits 英文 type + **中文描述首行 ≤ 72 字符**），远端 `push` 由人工独占；人工 push + CI 双平台通过后由编排者 `openspec archive linux-winit-platform-backend -y`，确认主 `openspec/specs/` 新增 `linux-platform-backend/spec.md`、既有 7 个主 spec 不变。

**回滚策略**：`git revert` 掉两份 `Cargo.toml` 的追加段与 `Cargo.lock` 变更即可完全回到改动前状态；无运行时状态、无存档、无协议版本需要回滚（未 bump `PROTOCOL_VERSION` / `SNAPSHOT_VERSION`）。回滚后 Linux 会重新触发 `compile_error!`（已知基线）。

## Open Questions

- **纯 Wayland / 无 DISPLAY 会话**：本 change 只启用 `x11`（含 XWayland 场景）。若将来需原生 Wayland 后端或无头运行，须另开 change 并触发 ADR 0001 依赖合规 / ADR 0002 复核点复核（D1 否决理由）。
- **归档顺序**：本 change 相对 `p6-playable-client`（实施中）、`gameplay-command-wire-format`、`cross-platform-golden-hash` 的归档先后由编排者裁定；因 D7 零 MODIFIED delta，无 capability 争用，但建议在本 change 的 `/opsx:apply` + 人工 push + CI 双平台通过后再归档（CODEBUDDY.md §9 串联）。
- **§10 人工复核**：D3 定性为「新增 target 条件依赖、非主版本升级」。若编排者仍希望按 CODEBUDDY.md §10「依赖重大升级」由人工独占复核 `Cargo.lock`，请明确；AI 不擅自执行任何远端 / 依赖升级动作。
- **`NOTICE.md` 登记**：`x11-dl` 等新增 Linux 平台依赖是否需要登记，由 `docs-owner` 在许可证核验后判定（不在 `spec-steward` / `client-owner` 可写范围）。
- **toolchain 漂移**：VM 实测 `rustc 1.99.1` / `python3 3.11.2` / `node 18`，CI 用 `dtolnay/rust-toolchain@stable` / Python 3.12 / Node 20；若 stable 未来升级导致 winit feature 解析变化，按 ADR 0002 复核点重新评估（本 change 不锁定 rustc 版本）。
