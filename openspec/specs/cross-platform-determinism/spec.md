# cross-platform-determinism Specification

## Purpose
本规范定义跨平台确定性的离线验证契约：以硬编码的离线固定命令脚本逐帧打印 `compute_state_hash()` 的 golden hash 基线，Windows 本机与 Debian 12 (x86_64, `rustc 1.99.1`) VM 以同一源码、同一工具链各运行一次，两份 stdout 逐字节一致时，据此把「Windows + Linux x86_64 / rustc 1.99.1」这一组合记为 ADR 0002 跨平台确定性已验证；结论不扩大，macOS、其它 CPU 架构（ARM64）与其它 `rustc` 版本仍属待验证项。本 capability 是验证而非改动：工具只调用 `w3-core` 公开 API，不新增权威状态、不 bump `SNAPSHOT_VERSION`（当前 3）、不改状态哈希字段集合与快照头部布局，遵守 ADR 0002 确定性契约。跨平台逐字节 diff 走人工 / 独立流程、不纳入 `tools/verify_all.py` 或 CI，CI 双平台跑同一 `python tools/verify_all.py` 只验证工具链跨平台。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: 离线固定命令脚本与逐帧可复现

golden-hash 工具 SHALL 在源码中**硬编码**一份确定性命令脚本（参照 `crates/w3-core/tests/determinism.rs::script()`：第 0 帧两个 Spawn、第 1 帧 Move、第 6 帧 Stop、第 12 帧 Move）与固定 `MatchConfig`（固定 `seed`、`logic_fps = 30`、固定 `content_hash`），并用 `StaticWorld::open_world(...)` 合成静态世界，MUST NOT 读取地图 / 导出 JSON / `config.toml` / 命令行参数 / 标准输入。工具 MUST 通过 `RtsMatch::create_with_world`（或等价公开入口）提交全部固定命令后按整数帧推进，每帧恰好一次 `step()`、`step()` MUST NOT 接受任何 delta / 墙钟 / 渲染帧率（ADR 0002 §2）。相同字节输入 SHALL 产出相同字节输出：相同源码、相同脚本、相同帧数下，任何平台、任何次运行的 stdout MUST 逐字节一致。工具 MUST NOT 使用 `Instant` / `SystemTime` / 运行时随机数 / `std::env::var` / 环境变量 / 线程调度顺序来决定命令内容、命令顺序或帧序。

#### Scenario: 同一平台重复运行一致

- **WHEN** 在同一平台对 golden-hash 工具连续运行两次并把 stdout 分别重定向到两个文件
- **THEN** 两份文本逐字节相等，逐帧 `frame=N hash=0x...` 序列与末尾汇总行完全相同

#### Scenario: 固定输入不依赖外部时序

- **WHEN** 在有无网络、不同系统时间、不同环境变量、不同工作目录下运行该工具
- **THEN** 输出逐字节不变，命令脚本、`seed`、`logic_fps`、`content_hash`、静态世界与帧序均来自硬编码常量

### Requirement: 逐帧 hash 输出格式与汇总行

golden-hash 工具 SHALL 把结果输出到 stdout：从推进的第 0 帧起，**每帧一行** `frame=N hash=0x...`，其中 `N` 为十进制整数帧、`0x` 后为 `compute_state_hash()` 返回的 `u64`（FNV-1a + 显式小端字节编码，ADR 0002 §6）的 **16 位定宽小写十六进制**（MUST 用 `{:016x}` 之类定宽格式，MUST NOT 使用变宽 `{:#x}` 省略前导零，保证跨平台字节稳定）。所有行 MUST 以 LF（`\n`）结束，MUST NOT 输出 CRLF / BOM / 彩色转义 / 中文 / 时间戳 / 散落诊断文本；每行输出 MUST 为英文 `key=value` 结构（CODEBUDDY.md §3 工具输出英文）。末尾 SHALL 有且仅有一行汇总，且汇总行 SHALL **只含编译期常量**——总帧数、最终 hash（等于最后一帧的 hash）与 `schema` 版本（如 `schema=golden-hash/1`）；汇总行 **MUST NOT** 包含 `rustc` 版本、时间戳、路径或任何平台 / 运行时相关信息，MUST NOT 引入运行时探测或任何随平台变化的值。`rustc` 等工具链版本 SHALL 只在验证报告（本 change 的 `tasks.md` 关键结果与 ADR 0002 复核记录）中记录，MUST NOT 进入 golden 基线文本，以保证两端输出逐字节可 diff。golden 文本 SHALL 仅作为验证产物留在各平台本机，MUST NOT 提交进仓库。

#### Scenario: 逐帧行格式稳定

- **WHEN** 运行 golden-hash 工具并查看 stdout
- **THEN** 每一帧恰好一行 `frame=N hash=0x<16 位小写 hex>`、帧号连续无缺漏，行尾为 LF、无 BOM / CRLF / 前导零省略差异

#### Scenario: 汇总行可复现

- **WHEN** 读取 stdout 最后一行
- **THEN** 汇总行只含编译期常量（帧数、与最后一帧相同的最终 hash、`schema` 版本），MUST NOT 含 `rustc` 版本 / 时间戳 / 路径或任何平台 / 运行时相关信息，工具链版本只在验证报告中记录、不进基线文本，且两次运行该行逐字节一致

### Requirement: Windows 与 Linux 整份文本逐字节 diff 为零

系统 SHALL 以「Windows 本机跑一次、Debian 12 VM 跑一次、两份 stdout 文本**逐字节 diff 为零**」作为跨平台确定性验收通过条件（ADR 0002 复核点「在非 Windows 平台首次实跑」）。两端 MUST 使用**同一份源码、同一工具链版本**（本机与 VM 均为 `rustc/cargo 1.99.1`、`protoc 31.1`、`openspec 1.3.1`），MUST 先由 `linux-winit-platform-backend` 使 VM 能编译全 workspace。比对 MUST 覆盖**整份文本**——所有逐帧行与末尾汇总行，MUST NOT 只比较最终 hash 或 `last_hash`：中间帧分歧同样 MUST 被捕获并判为不通过。比对 SHALL 用逐字节方式（跨平台 Python `pathlib.read_bytes()` 相等判定，或 `diff -u` 定位首个分歧行），MUST NOT 用「同机两实例相对比较」或真实 UDP 联机时序的 `frame applied` 日志代替（后者命令流依赖墙钟 / 本地 tick，绝对 hash 不同属正常）。

#### Scenario: 双平台逐字节一致

- **WHEN** 在 Windows 本机与 Debian 12 VM 以同一源码各运行一次 golden-hash 工具并逐字节比对两份 stdout
- **THEN** diff 为零、退出码为通过，判定 Linux (x86_64, rustc 1.99.1) 与 Windows 逐帧 hash 序列一致

#### Scenario: 中间帧分歧被捕获

- **WHEN** 人为构造两份文本，仅某一中间帧的 hash 不同而最终 hash 与汇总行恰好相同
- **THEN** 逐字节比对判为不通过并定位到该帧行，绝不因最终 hash 相同而放行

#### Scenario: 拒绝用联机日志充当基线

- **WHEN** 有人提议直接比对 Windows 与 VM 的 `lockstep_client.rs` `last_hash` / `frame applied` 绝对 hash
- **THEN** 该做法被明确否决（命令 `execute_frame` 依赖本地 tick、relay 依赖 `sleep(33ms)` 墙钟），验收只采信离线固定脚本的 golden-hash 文本

### Requirement: 不新增权威状态、状态哈希与快照字段不变

本 change MUST NOT 新增任何权威状态字段——因此**无**新字段需要声明进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照；MUST NOT bump `SNAPSHOT_VERSION` / `SIM_VERSION`、MUST NOT 改状态哈希字段集合、MUST NOT 改快照头部布局。golden-hash 工具 MUST 只调用 `w3-core` 的**公开 API**（`create_with_world` / `submit_command` / `step` / `compute_state_hash` / `first_difference`），MUST NOT 修改 `crates/w3-core/` 的任何实现、MUST NOT 暴露或注入任何新的权威状态。`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、30 Hz 整数帧、无参 `step()`、`first_difference()` 语义 SHALL 全部不变。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（确认零改动）；若将来需要新增权威状态，MUST 另开 change 并同时进入状态哈希与 `SNAPSHOT_VERSION` 定义的快照（ADR 0002 §6 / §7、CODEBUDDY.md §5）。

#### Scenario: 内核零改动与确定性验收

- **WHEN** 复核 `git diff` 并执行 `cargo test -p w3-core --test determinism -- --nocapture`
- **THEN** `crates/w3-core/` 实现零 diff、确定性测试全绿、状态哈希字段集合与 `SNAPSHOT_VERSION = 3`（含 `defs_hash` / `content_hash`）不变、未 bump 快照版本

#### Scenario: 工具只经公开 API

- **WHEN** 审查 golden-hash 工具的源码
- **THEN** 只出现 `w3_core` 公开 API 调用，无私有状态访问、无新增字段、无对 `compute_state_hash()` 编码方式的改写

### Requirement: 无墙钟、网络、窗口、文件与资产依赖，零新依赖

golden-hash 工具 SHALL 是**离线、无头（headless）**的可执行入口：MUST NOT 读取任何文件 / 网络 / 墙钟时间 / 环境变量 / `config.toml`；MUST NOT 依赖 UDP / relay / `w3-net` / `w3-relay`；MUST NOT 需要 GPU / 窗口 / `winit` / `w3-render` / `w3-client`；MUST NOT 读取本机导出地图或任何游戏资产（ADR 0001）。工具 MUST NOT 引入任何新依赖，SHALL 只依赖 `w3-core` 既有公开 API 与 Rust 标准库（保持 `w3-core` 零依赖、离线可编译）。工具 MUST NOT 把 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效、密钥或 `config.toml` 写进仓库；golden 文本作为验证产物 MUST 留在平台本机、不入库。

#### Scenario: 断网无窗口可运行

- **WHEN** 在断网、无 DISPLAY / 无窗口服务的环境中运行 golden-hash 工具
- **THEN** 正常输出完整逐帧 hash 文本，退出码为 0，不尝试打开文件 / 套接字 / 窗口 / `config.toml`

#### Scenario: 依赖面与资产合规

- **WHEN** 检查工具所在 crate 的 `Cargo.toml` / `Cargo.lock` 与 `git status`
- **THEN** 无任何新增依赖条目、无游戏资产 / 导出 JSON / 密钥 / golden 文本文件入库，资产扫描（`verify_all.py::asset_scan`）不变（ADR 0001）

### Requirement: 工具落点与分层依赖

golden-hash 工具 SHALL 落在**独立可执行目标（example）**而非库代码路径（推荐 `crates/w3-core/examples/golden_hash.rs`，最终落点为待拍板项，见 `design.md` D1 / D7）：该 example MUST 是独立编译目标，MUST NOT 在 `w3-core` 的 `[dependencies]` 中新增任何条目、MUST NOT 破坏 `w3-core` 零依赖与离线可编译（ADR 0002 §1）。工具 SHALL 只向下依赖 `w3-core`，MUST NOT 反向依赖 `w3-content` / `render` / `ui` / `audio` / `net`，`w3-core` MUST NOT 反向依赖上层 crate；唯一权威写入者仍是 `RtsMatch`，表现层 MUST NOT 反写模拟状态（ADR 0002 §5）。若拍板改落 `crates/w3-app/examples/`，MUST 注意 `w3-app` 依赖 `w3-content`（读导出 JSON）与「注入边界」语义、且 `w3-app` 无专属 owner，并在 `design.md` / `tasks.md` 同步记录。单 change 单 owner：SHALL 指定一个实现 owner（推荐 `sim-owner`，配只读 `critic`），MUST NOT 由多个 owner 并发写同一 change 的文件（CODEBUDDY.md §8 / §9）。

#### Scenario: example 不污染内核依赖

- **WHEN** 在 `w3-core` 新增 golden-hash example 后检查 `crates/w3-core/Cargo.toml` 与依赖图
- **THEN** `[dependencies]` 仍为空、无新依赖，example 仅链接 `w3_core` 与标准库，`w3-core` 不反向依赖任何上层 crate

#### Scenario: owner 与 critic 边界

- **WHEN** 进入 `/opsx:apply` 分派实现任务
- **THEN** 只有一个实现 owner 承接工具代码，只读 `critic` 复审确定性 / 所有权 / 许可证 / 编码 / 资产入库，`mode` 不使用 `bypassPermissions`

### Requirement: ADR 0002 结论收敛范围（只追加、不扩大）

双平台逐字节 diff 通过后，系统 SHALL 由 `spec-steward` 在 `docs/decisions/0002-determinism.md` **只追加**一条「复核记录」段：把「Linux / Debian 12 (x86_64, rustc 1.99.1) 首次实跑、与 Windows 逐帧 hash 序列一致」记为**已验证**；MUST 同时明确 **macOS / 其它 CPU 架构仍是待验证项**，MUST NOT 把结论扩大为「所有平台 / 所有 CPU 已确定」。追加 MUST NOT 修改 ADR 0002 正文任何既有段落、MUST NOT 改写第 94 行历史 `rustc 1.91.0` 数值、MUST NOT 删除「复核点」。规范分工 MUST 保持：ADR 记「为什么决定 / 验证到什么程度」，spec 记「系统该做什么」，本 capability MUST 引用 ADR 0002 编号（CODEBUDDY.md §8）。**本轮提案阶段 MUST NOT 改 ADR**：只把「实施后由 `spec-steward` 追加」写进 `design.md` D5 与 `tasks.md`，作为待拍板项。`CODEBUDDY.md §1 / §2 / §5` 的「待验证」措辞同步属后续 `docs-owner` / 编排者一致性维护，本 change MUST NOT 触碰。

#### Scenario: 只追加且明确边界

- **WHEN** 双平台 diff 通过后向 `docs/decisions/0002-determinism.md` 追加复核记录
- **THEN** 改动仅为新增一段、既有正文零变化，段内写明 Linux 已验证且 macOS / 其它 CPU 架构仍待验证

#### Scenario: 结论不扩大

- **WHEN** 起草任何对外验收结论或 commit / PR 描述
- **THEN** 只表述「Windows 与 Linux (x86_64, rustc 1.99.1) 逐帧一致」，不出现「全平台确定性已达成」之类断言

#### Scenario: 提案阶段不动 ADR

- **WHEN** 本轮 `spec-steward` 交付四件套时检查 `git status`
- **THEN** `docs/decisions/` 无任何改动，ADR 追加只在实施后 tasks 中列出

### Requirement: CI 集成边界与门禁

golden-hash 的跨平台 diff MUST NOT 纳入 `tools/verify_all.py` 或 CI 配置：`verify_all.py` 是**单次单平台**门禁，跨平台 diff 需要两端产物汇总后比对，不适合塞进单平台门禁；本 change MUST NOT 修改 `.github/workflows/ci.yml` / `tools/`（CI 改动属 CODEBUDDY.md §10 人工独占），golden-hash diff SHALL 列为人工 / 独立脚本流程。CI matrix（`ubuntu-latest` / `windows-latest`）跑同一单行 `python tools/verify_all.py` 的既有口径 SHALL 不变，且 MUST NOT 被表述为「已验证模拟内核跨平台确定性」（CODEBUDDY.md §2 / §4）。本 change 自身 MUST 通过 `openspec validate cross-platform-golden-hash` 与 `--strict`（`verify_all.py` 遍历所有 active change）。归档前 SHALL 复核主 `openspec/specs/` 只新增 `cross-platform-determinism/spec.md`、既有 7 个主 spec 零变化；归档顺序 SHALL 排在 `linux-winit-platform-backend` 之后（见 `design.md` D8 / Open Questions）。

#### Scenario: CI 配置零改动

- **WHEN** 复核 `git diff` 中的 `.github/workflows/ci.yml` 与 `tools/`
- **THEN** 二者零变化，`verify_all.py` 双平台仍只验证工具链跨平台，golden-hash diff 由人工 / 独立流程执行

#### Scenario: validate 与主 specs 收敛

- **WHEN** 执行 `openspec validate cross-platform-golden-hash --strict` 并在归档后检查主 `openspec/specs/`
- **THEN** validate 通过，主 specs 仅新增 `cross-platform-determinism/spec.md`，既有 `deterministic-simulation` 等 7 个主 spec 无 MODIFIED 变化
