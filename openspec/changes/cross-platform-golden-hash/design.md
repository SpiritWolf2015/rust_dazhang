## Context

`rust_dazhang` 是 GPLv3、Rust edition 2024 虚拟 workspace（ADR 0003）。P0–P5 已归档、P6 `p6-playable-client` 实施中；主 `openspec/specs/` 共 7 个 capability；当前 4 个 active change：`p6-playable-client`（54/55）、`gameplay-command-wire-format`（9/54）、`linux-winit-platform-backend`（0/30）、`cross-platform-golden-hash`（本 change）。路线图阶段与 `gameplay-command-wire-format` (f) / `linux-winit-platform-backend` 口径一致：**「P6 集成收尾的后续补齐」、不新增阶段**。

当前状态（已由源码 / ADR / 测试逐项复核）：

- [ADR 0002 第 94 行](../../../docs/decisions/0002-determinism.md)：「目前仅在本机 Windows 11 / rustc 1.91.0 实跑通过；Linux、macOS 与其他 CPU 架构的跨平台确定性尚未实测，属于待验证项，而非已达成结论」；[CODEBUDDY.md §2 / §5](../../../CODEBUDDY.md) 明确 CI 双平台跑同一 `python tools/verify_all.py` 只验证**工具链**跨平台，不等于模拟内核确定性已达成。ADR 0002 §8「复核点」含「在非 Windows 平台首次实跑」。
- `crates/w3-core/tests/determinism.rs` 全部是**相对比较**：`two_instances_interleaved_match_frame_hashes` 等测试用 `assert_eq!(a.compute_state_hash(), b.compute_state_hash())`、`assert_eq!(a.first_difference(&b), None)`、`original` vs `restored`；双实例在**同一进程、同一 OS、同一 rustc、同一 CPU** 上交错推进。`config()` = `seed 0x5eed / logic_fps 30 / content_hash 0xC0FFEE`，`script()` = 第 0 帧两个 Spawn（玩家 1/2，分配 `EntityId` 1/2）、第 1 帧两个 Move、第 6 帧 Stop、第 12 帧 Move；`StaticWorld::open_world(16, 16, SimFixed::ONE)` 在快照测试中已使用。Linux 上该测试全绿只证明「Linux 自身自洽」，不构成 Windows ↔ Linux 绝对 hash 基线。
- `crates/w3-core/src/lib.rs` 第 16–18 行列明公开入口：`create` / `submit_command` / `step` / `read_view` / `drain_events` / `capture_snapshot` / `restore_snapshot` / `compute_state_hash` / `first_difference`；`sim_match.rs` 另有 `create_with_world(config, world)`（第 388 行）、`compute_state_hash()` 返回 `u64`（第 1447 行）；`world.rs` 有 `open_world(width, height, cell_size)`（第 109 行）。工具只调公开 API 即可完成逐帧 hash。
- `crates/w3-core/src/state_hash.rs`：FNV-1a（`OFFSET_BASIS` / `PRIME`）+ `to_le_bytes()` 显式小端字节编码；`crates/w3-core/Cargo.toml` 的 `[dependencies]` 为空（注释「刻意不引入任何依赖：P0 只使用标准库，保证离线可编译」），`crates/w3-core/examples/` **当前不存在**。
- `crates/w3-app/examples/inject_export.rs:80` 只打印**一个**最终 `state_hash`，且经 `load_map` 读本机导出地图 JSON（ADR 0001 不入库），不适合做基线；`crates/w3-app/Cargo.toml` 依赖 `w3-core` + `w3-content`。
- `crates/w3-net/examples/lockstep_client.rs:99` 逐帧打印 `frame applied: frame=N hash={:#x}`，但 `execute_frame` 由 `sim.read_view().tick + INPUT_DELAY` 本地 tick 推导（第 86 行）、客户端 `sleep(33ms)`（第 107 行）、`w3-relay` 用墙钟节拍；不同机器 / 不同次运行命令流时序不同，绝对 hash 不同属正常。编排者实测 Windows `last_hash=0x24bab69f54062e59` 与 VM `0xa89ca94078604bff` 不可直接比对。
- 已具备的验证环境（编排者本轮实测）：Windows 本机 `c:/open_rts/rust_dazhang`（HEAD `e1ba8a4`，`rustc 1.99.1`、`protoc 31.1`、`openspec 1.3.1`，`python tools/verify_all.py` → `Verification OK`）；Debian 12 VM `192.168.56.12`，`/srv/rust_dazhang` 为本机干净工作区 tar 快照（含 `.git`），`rustc/cargo 1.99.1`（与 Windows **完全一致**）、`rustfmt 1.10.0-stable`、`protoc 31.1`、`openspec 1.3.1`、`python3 3.11.2`、`node 18`、`git 2.39.5`、磁盘 79G 可用；提案阶段 VM 在临时 winit 补丁（当时 `linux-winit-platform-backend` 正在正式修、本机仓库尚未改）后 `cargo test --workspace` 24 项全绿、`verify_all.py` → `Verification OK`、真实 UDP 双客户端 30 帧逐帧 hash 序列完全一致。**注意最后一项是同一 VM 内两进程一致，不是跨平台结论**。**实施后状态**：`linux-winit-platform-backend` 已由 `client-owner` 在本机 `crates/w3-render/Cargo.toml` / `crates/w3-client/Cargo.toml` / `Cargo.lock` 正式落地（非零改动），VM 重新同步后 `python3 tools/verify_all.py` → `Verification OK`（24 项全绿）；本 change 的 golden-hash example 已由 `sim-owner` 落地、双平台逐字节 MATCH，`critic` 复审完成、无阻塞项。

干系人：实现 owner（推荐 `sim-owner`，待拍板）、只读 `critic`、`spec-steward`（规范 + 实施后 ADR 追加）、编排者（同步 VM / 人工跑双平台 diff / 人工 push / 归档）。相关 ADR：0001 / 0002 / 0003 / 0004 / 0005。

本轮（`spec-steward`）只交付四件套规范，实施走 `/opsx:apply`；不改 ADR 正文、不碰 `crates/` / 主 `openspec/specs/` / 其它 change / `tools/` / CI 配置。

## Goals / Non-Goals

**Goals:**

- 提供一个**离线、固定命令脚本、无墙钟 / 网络 / 窗口 / 文件 / 资产、零新依赖**的可执行入口，产出绝对逐帧 hash 文本，作为 ADR 0002 跨平台确定性的 golden 基线。
- 以「Windows 跑一次、VM 跑一次、整份 stdout 逐字节 diff 为零」为唯一通过条件，捕获**中间帧**分歧而非只比最终 hash。
- 把 [ADR 0002](../../../docs/decisions/0002-determinism.md) 的 Linux (x86_64) 待验证项收敛为「已验证」，且**明确 macOS / 其它 CPU 架构仍待验证**，不扩大结论。
- 保持 `w3-core` 确定性契约**零改动**：不新增权威状态、不 bump `SNAPSHOT_VERSION`（当前 3）、不改状态哈希字段集合 / 快照头部 / 帧序 / 命令排序 / `step()` 语义。
- 只新增独立 capability `cross-platform-determinism`，不产生 `deterministic-simulation` 的 MODIFIED delta，避免与 active change 争用；理清 CI 边界与归档顺序。

**Non-Goals:**

- **不声称全平台 / 全 CPU 确定性已达成**：macOS / ARM / 其它架构仍待验证；`rustc` 版本漂移需重新生成基线。
- 不改 `w3-core` 实现、不改公开 API、不改状态哈希 / 快照 / `PROTOCOL_VERSION`（ADR 0005）/ `SNAPSHOT_VERSION`。
- 不把 golden-hash diff 塞进 `tools/verify_all.py` 或 CI；不改 `.github/workflows/ci.yml` / `tools/`（CODEBUDDY.md §10 人工独占）。
- 不提交 golden 文本 / 游戏资产 / 导出 JSON / `config.toml` / 密钥；golden 文本只留平台本机。
- 不引入新依赖、不新建独立 crate、不写 `.ps1` / `.sh` / `.bat` / `.cmd`（CODEBUDDY.md §2）；工具用 Rust example，比对用跨平台 Python / `diff`。
- 不新增路线图阶段（沿用「P6 集成收尾的后续补齐」）；不改 `config.yaml` rules.proposal 的「P0-P5」枚举。
- 本轮不改 ADR 0002 正文、不改 `CODEBUDDY.md`、不归档、不 commit、不执行任何 git 远端命令、不运行 `cargo`。

## Decisions

每个决策统一给出：**决策** / **备选方案（Alternatives considered）** / **关联 ADR 0002 与跨平台确定性影响** / **架构原则与分层依赖**。

### D1：工具形态与落点——`w3-core` 的独立 example（推荐，待拍板）

**决策**：新增一个**离线可执行 example**（推荐路径 `crates/w3-core/examples/golden_hash.rs`），只 `use w3_core::{...}` 与标准库，不读文件 / 网络 / 墙钟 / 环境变量 / `config.toml`，不需 GPU / 窗口，不含任何游戏资产（ADR 0001），**不引入新依赖**。它是 `[[example]]` 独立编译目标，不进 `w3-core` 的 lib、不进 `[dependencies]`、`Cargo.lock` 无新增条目。之所以推荐 `w3-core` 而非 `w3-app`：golden hash 验证的对象是 `w3-core` 的确定性契约，工具与 `crates/w3-core/tests/determinism.rs` 同属内核验收面（SRP），且 `w3-app` 依赖 `w3-content`（读导出 JSON）、语义是「注入边界」、`inject_export.rs` 依赖本机地图，与本工具「不读文件 / 不需要内容层」错配，更关键的是 **`w3-app` 没有专属 owner**（见 D7）。最终落点为**待拍板项**（见 Open Questions）。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **`crates/w3-core/examples/golden_hash.rs`（独立 Rust example）** | **采纳（推荐）** | 与被测对象同 crate、同 `determinism.rs` 验收面；零新依赖、无需窗口 / 网络 / 文件；`w3-core` 零依赖与离线可编译不被破坏；example 是独立目标，不污染 lib；`cargo run -p w3-core --example golden_hash` 一处运行，两端同命令。 |
| `crates/w3-app/examples/golden_hash.rs` | 备选（待拍板） | `w3-app` 已有 example 目录与「注入边界」示例，但它依赖 `w3-content`（读导出 JSON / ADR 0001 本机地图）、语义错配；且 **无专属 owner**，需编排者新建 / 指定 agent（不在 `spec-steward` 可写范围）。若拍板选它，须在 tasks 里同步改 owner 与运行命令。 |
| Python 工具脚本（`tools/*.py`）调 Rust 库 | 否决 | Python 无法直接链接 Rust `w3-core` 的 `compute_state_hash()`；要么再包一层 FFI / CLI，等于绕回 Rust 入口，还引入 `ctypes` / 子进程时序与编码（BOM / CRLF）风险；且工具本身应是 Rust 才能证明「同一份 Rust 代码在两平台一致」。 |
| 新建独立 crate（如 `w3-goldenhash`） | 否决 | 为一个验证入口新增 workspace 成员 + `Cargo.toml` + 维护面，违反最小改动（YAGNI / SRP）；且新 crate 同样要依赖 `w3-core`，与 example 等价但更重；`Cargo.lock` / 资产扫描 / owner 边界都要扩。 |
| 扩展 `crates/w3-app/examples/inject_export.rs` | 否决 | 它 `:80` 只打印一个最终 `state_hash`、依赖 `load_map` 读本机导出地图（ADR 0001 不入库），与「离线固定脚本 / 不读文件 / 逐帧输出」三条硬要求直接冲突。 |
| 复用 `crates/w3-net/examples/lockstep_client.rs` 的 `frame applied` 日志 | 否决 | 依赖 UDP + relay 墙钟 `sleep(33ms)` + 本地 tick 推导 `execute_frame`，命令流时序随机器 / 运行变化，绝对 hash 不同属正常；不能做 golden 基线（见 Context）。 |
| 写进 `crates/w3-core/tests/determinism.rs`（加一个打印绝对 hash 的测试） | 否决 | 测试是相对比较、由 libtest 管理输出（含进度 / 计时噪声）、`--nocapture` 才可见，不是稳定的 stdout 文本；且会把「验证基线」与「相对门禁」混在一个文件（SRP）。本 change 要求 `determinism.rs` **零改动**。 |

**关联 ADR 0002 与跨平台确定性影响**：这是整个 change 的载体。工具只**消费** `w3-core` 公开 API（`create_with_world` / `submit_command` / `step` / `compute_state_hash`），不写入、不扩展任何权威状态；FNV-1a 显式小端编码（`state_hash.rs`）原样保留，因此两端输出的绝对 hash 具备可比性。零新依赖意味着不存在第三方数值库 / `libm` 版本差异干扰，正好把 ADR 0002 §1「纯整数数学、离线可编译」的跨平台前提暴露给实测验证。落 `w3-app` 会把 `w3-content` 的冻结 `f64` 输入（ADR 0004）带进依赖图，虽不进哈希但徒增变量，故推荐 `w3-core`。本决策本身**不产生也不否定**确定性，只是提供可观测通道。

**架构原则与分层依赖**：SRP——验证入口只做「固定脚本 → 逐帧 hash 文本」一件事；ISP / DIP——只依赖 `w3-core` 公开契约，不依赖 `w3-content` / `net` / `render`；CARP / LoD——example 组合内核而非继承 / 改写；OCP——新增 example 而不改 `determinism.rs` 或 lib；`w3-core` 不反向依赖上层（硬红线），唯一权威写入者仍是 `RtsMatch`（ADR 0002 §5）。

### D2：固定命令脚本——全硬编码、相同字节输入 → 相同字节输出

**决策**：在 example 源码里**硬编码**确定性输入，复刻 `determinism.rs` 的可复现蓝本但独立成文（不改测试）：`MatchConfig { seed: 0x5eed, logic_fps: 30, content_hash: 0xC0FFEE }`、`StaticWorld::open_world(16, 16, SimFixed::ONE)` 合成世界（不读地图）、固定命令脚本（第 0 帧玩家 1/2 各一个 Spawn，第 1 帧两个 Move，第 6 帧一个 Stop，第 12 帧一个 Move；`EntityId` 1/2 由内核「ID 不复用 + 玩家编号排序」契约确定），固定推进帧数（如 40 帧，与 `determinism.rs` 交错测试窗口一致）。全部经 `submit_command` 一次性提交，按整数帧每帧一次 `step()`。MUST NOT 使用 `Instant` / `SystemTime` / 运行时随机数 / `std::env::var` / 命令行参数 / 标准输入 / 文件 / 网络 / 线程调度顺序决定命令内容、顺序或帧序。`rustc` 等工具链版本只进验证报告（`tasks.md` 关键结果与 ADR 0002 复核记录），MUST NOT 进入输出文本；汇总行只含编译期常量（帧数、最终 hash、`schema=golden-hash/1`），MUST NOT 含 `rustc` 版本 / 时间戳 / 路径 / 平台或运行时相关信息，且不参与命令或帧序。验收语义：**相同字节输入 → 相同字节输出**——同源码、同脚本、同帧数下任何平台 / 任何次运行 stdout 逐字节一致。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **全硬编码常量脚本 + `open_world` 合成世界** | **采纳** | 输入完全自足、无外部时序 / 资产，两端字节一致；复用 `determinism.rs::script()` 的既有契约（Spawn 顺序 → EntityId），可读性强。 |
| 从文件 / JSON 读命令脚本 | 否决 | 引入文件 I/O 与「两端脚本文件是否逐字节相同」的新变量，违反「不读文件 / 相同字节输入」；且易被误当成导出资产（ADR 0001）。 |
| 从 `config.toml` 读 `seed` / `logic_fps` / `content_hash` | 否决 | `config.toml` 被 gitignore、玩家本机各异，直接破坏绝对值可比性（ADR 0001 / ADR 0002 §2 冻结）。 |
| 用 `inject_export.rs` 的地图注入 / `w3-content` 导出 JSON | 否决 | 依赖本机导出地图（不入库）、只产出一个最终 hash；与「离线固定输入 / 逐帧」冲突（Context）。 |
| 复用 `lockstep_client.rs` 的 `INPUT_DELAY` + 本地 tick 推导 `execute_frame` | 否决 | 命令 `execute_frame` 随本地推进变化，不同机器命令流时序不同；绝对 hash 不可比（D1 已否决）。 |
| 用运行时 `rand` / 墙钟 / `HashMap` 迭代顺序生成命令 | 否决 | 直接违反 ADR 0002 §3 / §4（稳定排序、独占 RNG 流、禁 `HashMap` 迭代顺序），会制造假 desync。 |
| 帧数由命令行参数 / 环境变量控制 | 否决 | 引入外部输入，两端可能不一致；固定帧数常量即可，且汇总行需稳定。 |

**关联 ADR 0002 与跨平台确定性影响**：本决策把 ADR 0002 的所有确定性要素固定为常量：`SimFixed` Q47.16（`from_int` 构造）、30 Hz 整数帧、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、独占 RNG 流（由固定 `seed` 与内核自管，工具不触碰）、FNV-1a 显式小端哈希。正因为所有输入都是整数常量且无宿主相关量，Windows / Linux 若产出不同 hash，即可**定位到编译器 / 平台对整数或字节编码的差异**（这正是本 change 要验证的东西），而不是被墙钟 / 网络 / 浮点噪声污染。`SNAPSHOT_VERSION`（当前 3）与快照头部布局不被工具使用也不被改动。

**架构原则与分层依赖**：DIP——工具依赖 `w3-core` 的公开 `Command` / `MatchConfig` / `StaticWorld` 构造，不碰私有状态；SRP——脚本是「输入夹具」，与「输出格式」（D3）分离；显式失败不静默降级——`submit_command` / `from_int` 的 `Result` MUST 显式 `unwrap` / 报错退出，不吞错；「正确性 / 确定性优先」硬红线在本决策体现为拒绝一切非确定性输入。

### D3：输出格式与比对——逐帧定宽十六进制 + 逐字节 diff 全序列

**决策**：stdout 从推进的第 0 帧起**每帧一行** `frame=N hash=0x...`：`N` 为十进制整数帧，`0x` 后为 `compute_state_hash()` 的 `u64`（FNV-1a + 显式小端字节编码）的 **16 位定宽小写十六进制**（用 `format!("0x{hash:016x}")`，MUST NOT 用变宽 `{:#x}`，避免前导零省略造成的字节差异）；所有行以 LF 结束，MUST NOT 输出 CRLF / BOM / 彩色转义 / 中文 / 时间戳 / 日志前缀，保持英文 `key=value`（CODEBUDDY.md §3）。末尾**有且仅有一行汇总**：`frames=<帧数> final_hash=0x<16 位定宽小写 hex> schema=<编译期常量>`，且只含**编译期常量**——总帧数、最终 hash（= 最后一帧 hash）与 `schema` 版本（编译期常量字面量，如 `const GOLDEN_SCHEMA: &str = "golden-hash/1"`）；两端源码相同即字节相同，且**不参与命令或帧序**（D2）。汇总行 **MUST NOT** 包含 `rustc` 版本、时间戳、路径、主机名、OS 名或任何平台 / 运行时相关信息，MUST NOT 含任何运行时探测值（如当前时间），亦 MUST NOT 借 `env!("...")` / `std::env::var` 注入工具链探测值。`rustc` 等工具链版本 SHALL 只在验证报告（本 change 的 `tasks.md` 关键结果与 ADR 0002 复核记录）中记录，MUST NOT 进入 golden 基线文本——基线文本与平台相关信息彻底解耦，是保证同一份源码两端逐字节可 diff 的前提。golden 文本只留平台本机、MUST NOT 入库。

比对方式：Windows 跑一次、VM 跑一次，**整份文本逐字节 diff 为零**才算通过。用跨平台 Python `pathlib.read_bytes()` 相等判定（或 `diff -u` 定位首个分歧行），MUST NOT 只比最终 hash / `last_hash`：**中间帧分歧也要捕获**（两平台可能在某帧短暂分歧后重新收敛，只比最终 hash 会漏掉）。MUST NOT 用 `lockstep_client.rs` 的联机 `frame applied` 日志或同机双实例相对比较代替。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **逐帧 `frame=N hash=0x<016x>` + 汇总行 + 整份逐字节 diff** | **采纳** | 绝对基线、可定位到具体帧、两端同源码同格式；定宽避免前导零 / CRLF 差异；全序列覆盖中间帧分歧。 |
| 只打印 / 只比对最终 `state_hash`（如 `inject_export.rs:80`） | 否决 | **中间帧分歧会漏掉**；`inject_export.rs` 还依赖本机地图、只给一个 hash。 |
| 变宽 `{:#x}` 十六进制 | 否决 | 前导零省略使不同量级 hash 的文本宽度不同；虽然同值两端一致，但定宽 `{:016x}` 更明确、能杜绝格式化实现差异，且便于人工核对。 |
| JSON / 二进制输出 | 否决 | 引入序列化库（新依赖）或手写二进制字节序风险；纯文本行更利于 `diff`、人工定位与编码基线（UTF-8 无 BOM / LF）。 |
| 只比最终 hash 一致的「宽松通过」 | 否决 | 见 Risks「只比最终 hash 漏中间帧」；与 ADR 0002 §8「逐帧哈希一致」验收语义不符。 |
| 用 `lockstep_client.rs` 联机日志 / 真实 UDP 双客户端 hash 比对 | 否决 | 命令流依赖墙钟与本地 tick；两端绝对 hash 不同属正常（Context），会误判为 desync。 |
| 把 `rustc` / OS / 主机名 / 时间戳 / 路径用运行时 API 探测或编译期常量打印进汇总 | 否决 | 两端这些值可能不同或绑定特定工具链 → 汇总行 diff 非零 / 基线绑定平台相关信息 → 假失败，且违反「无运行时环境变量 / 墙钟」与「输出不绑定平台相关信息」约束；工具链版本只在验证报告中记录、不进汇总行（实施按编排者明确指令，汇总行只有 `schema=golden-hash/1` 一个编译期常量）。 |
| 把 Windows 的 golden 文本提交进仓库当基线 | 否决 | 验证产物入库会与「不提交资产 / 导出数据」边界混淆，且工具链升级后基线过期；本 change 采用「两端各跑一次、逐字节比对」的对称方式。 |

**关联 ADR 0002 与跨平台确定性影响**：输出就是 ADR 0002 §6 FNV-1a 全状态哈希的**绝对值序列**，而非 `determinism.rs` 的相对相等断言——这是本 change 补齐的根本缺口。定宽小写 hex 直接对应 `to_le_bytes()` 显式小端编码后的 `u64`，任何平台若对整数除法向零截断、`i128` 中间量、字节序或 `BTreeMap` 迭代产生差异，都会在某一帧的 16 位 hex 上暴露。汇总行不含 `rustc` 基线常量等平台相关信息，使基线文本只由固定输入与编译期常量决定：两端「工具链一致性」（`rustc 1.99.1` / `protoc 31.1`）只在验证报告（`tasks.md` 关键结果）与 ADR 0002 复核记录中承载，MUST NOT 进入基线文本；若将来 rustc 漂移导致 hash 变化，基线必须重生成（见 Risks）。本决策不改哈希算法 / 字段集合 / 快照，只读 `compute_state_hash()`。

**架构原则与分层依赖**：SRP——「采集」（D2）与「呈现 / 比对」（D3）分离；ISP——工具不暴露新 API，只产出文本；LoD——不触碰内核内部表示，只消费公开返回值；「显式失败不静默降级」——diff 非零 MUST 明确报不通过并定位到帧，不静默忽略；工具输出英文 `key=value`、注释中文（CODEBUDDY.md §3）。比对脚本用跨平台 Python / `diff`，不写 `.sh` / `.ps1`（CODEBUDDY.md §2）。

### D4：快照与哈希字段不变——验证而非改动，零权威状态新增

**决策**：本 change **不新增任何权威状态字段**、不 bump `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）与 `SIM_VERSION`、不改状态哈希字段集合、不改快照头部布局、不改帧序 / `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序 / 稳定 `EntityId` 顺序 / 30 Hz 整数帧 / 无参 `RtsMatch::step()` / `first_difference()` 语义。golden-hash 工具 MUST 只调用 `w3-core` 公开 API、MUST NOT 修改 `crates/w3-core/` 任何实现或依赖——`w3-core` 的确定性契约**零改动**。`config.yaml` rules.specs 要求「新增权威状态必须声明进入状态哈希与 `SNAPSHOT_VERSION` 定义的快照（当前 3）」：本 change **无新增权威状态**，故无字段需要该声明；spec 的「不新增权威状态、状态哈希与快照字段不变」requirement 已显式写明这一点。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（确认零改动）。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **工具只调公开 API、内核与测试零改动** | **采纳** | 这是「验证」而非「改动」：现有相对比较测试继续守门禁，新工具补绝对基线；改动面最小、风险最低。 |
| 为 golden hash 在 `w3-core` 新增专用哈希 API / 字段 | 否决 | 会改变公开 API 与权威状态面、触发 ADR 0002 §8 全量重验证、可能 bump 快照；`compute_state_hash()` 已足够。 |
| 给状态哈希加「平台标签」字段辅助比对 | 否决 | 新增权威状态字段 → 必须同时进哈希与快照（config.yaml rules.specs）、bump `SNAPSHOT_VERSION`，彻底偏离「验证」定位且制造平台相关状态。 |
| bump `SNAPSHOT_VERSION` / 改快照头部布局 | 否决 | 与 golden 验证无关，破坏跨版本快照契约（ADR 0002 §7「不支持跨版本迁移」）。 |
| 顺手把 `determinism.rs` 改成打印绝对 hash | 否决 | 会改动确定性门禁测试本体、引入 libtest 输出噪声；且违反本 change「`determinism.rs` 零改动、只重跑」的约束。 |

**关联 ADR 0002 与跨平台确定性影响**：本决策是「确定性契约零改动」的护栏。正因为哈希字段集合、字节编码、快照布局完全不动，Windows / Linux 的 hash 序列差异才**只可能**来自平台 / 编译器对既有整数运算与数据结构的实现差异，而不会混入「工具自己改了状态」的噪声；这是 golden 比对有效的逻辑前提。若 diff 非零，调查方向明确（ADR 0002 §1 超越函数未用、`libm` 未引入、无浮点进权威状态），而非先怀疑基线工具。ADR 0002 §8「任何改动权威状态 / 帧序 / 命令排序 / 哈希编码 / 快照字段的改动，必须先让测试通过」——本 change 无此类改动，`determinism.rs` 重跑只是回归确认。

**架构原则与分层依赖**：唯一状态所有者仍是 `RtsMatch`（ADR 0002 §5）；DIP / 表现层只读边界——工具是只读观察者，不反写模拟状态；OCP——新增 example 扩展验证能力而不修改内核；`w3-core` 零依赖、不反向依赖上层（硬红线）；正确性 / 确定性优先于便利。

### D5：ADR 0002 复核记录——实施后由 `spec-steward` 只追加，不扩大结论（待拍板）

**决策**：双平台逐字节 diff **通过后**，由 `spec-steward` 在 `docs/decisions/0002-determinism.md` **只追加**一条「复核记录」段：把「Linux / Debian 12 (x86_64, rustc 1.99.1) 首次实跑、与 Windows 逐帧 hash 序列一致」记为**已验证**；MUST 同时明确 **macOS / 其它 CPU 架构仍是待验证项**，MUST NOT 把结论扩大为「全平台 / 全 CPU 确定性已达成」。追加 MUST NOT 修改 ADR 正文任何既有段落、MUST NOT 改写第 94 行历史 `rustc 1.91.0` 数值、MUST NOT 删「复核点」。分工保持：ADR 记「为什么决定 / 验证到什么程度」，spec 记「系统该做什么」，本 capability 抬头已引用 ADR 0002 编号（CODEBUDDY.md §8）。**本轮提案阶段不改 ADR**：只把「实施后由 `spec-steward` 追加」写进本 D5 与 `tasks.md` 第 6 组，列为**待拍板项**（见 Open Questions）。`CODEBUDDY.md §1 / §2 / §5` 的「待验证 / 仅 Windows 实跑」措辞同步属后续 `docs-owner` / 编排者一致性维护，**不在本轮可写范围、本 change 不碰**。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **实施后只追加一条复核记录、明确 macOS / 其它架构待验证** | **采纳（推荐）** | 兑现 ADR 0002 §8 复核点「在非 Windows 平台首次实跑」；只追加不改正文，历史数值与决策上下文完整；结论严格限定到已实测的 Windows / Linux x86_64 + rustc 1.99.1。 |
| 本轮提案阶段就改 ADR | 否决 | 双平台 diff 尚未在本 change 内实跑（本轮不运行 `cargo`），提前把结论写成「已验证」是无证据断言；用户明确「本轮提案阶段先不改 ADR」。 |
| 改 ADR 正文第 94 行措辞 / 把 `rustc 1.91.0` 改成 `1.99.1` | 否决 | 那是历史基线数值，改写等于篡改决策记录；新工具链应以「复核记录」增量追加，不回改正文。 |
| 把结论写成「跨平台确定性已达成」 | 否决 | 直接违反 ADR 0002 第 94 行与 CODEBUDDY.md §2 / §5；macOS / 其它 CPU / rustc 漂移均未实测。 |
| 追加记录但不提 macOS / 其它架构仍待验证 | 否决 | 沉默会被误读为「全部已验证」；必须显式保留未验证边界。 |
| 不改 ADR、只在 change 里记结论 | 否决 | ADR 0002 是「待验证项」的权威出处，不追加则两份规范长期打架；`spec-steward` 职责正是维护 ADR 与 spec 一致。 |
| 顺手同步 `CODEBUDDY.md` 措辞 | 否决（本轮） | 超出本轮可写范围（用户明确不碰 `CODEBUDDY.md`）；列为后续 `docs-owner` 一致性维护 / Open Questions。 |

**关联 ADR 0002 与跨平台确定性影响**：这是把「验证结果」回写到「为什么决定」的唯一落点。追加内容严格绑定实测条件（Windows 11 + Debian 12 x86_64、`rustc 1.99.1`、同一份固定脚本、逐字节 diff 为零），不把 ADR 0002 §1 的定点数学 / §6 哈希字段 / §7 快照契约作任何修改；反过来，正因为 hash 序列一致，才有证据把「Linux 待验证」升级为「已验证」，同时保留 macOS / 其它 CPU 作为 ADR 0002 复核点的后续项。ADR 与 spec 不打架：spec 记「系统该产出可逐字节比对的逐帧 hash 文本」，ADR 记「验证到什么程度」。

**架构原则与分层依赖**：单一权威（ADR 0002 是确定性结论的唯一出处、本文件不另立结论）；SRP——ADR 管「为什么 / 验证程度」，spec 管「做什么」，change 管「怎么改」；显式失败不静默降级——diff 未通过 MUST NOT 追加「已验证」；`docs/decisions/` 是 `spec-steward` 可写范围，`.codebuddy/agents/` / `CODEBUDDY.md` 不是。

### D6：CI 集成边界——不纳入 `verify_all.py` / CI，列为人工 / 独立流程

**决策**：**不**把 golden-hash 双平台 diff 纳入 `tools/verify_all.py` 或 `.github/workflows/ci.yml`。理由：`verify_all.py` 是**单次单平台**门禁（编码 / `cargo fmt --all -- --check` / `cargo test --workspace` / `openspec validate` / 资产扫描），一次运行只在一个 OS 上产出一个结果；跨平台 diff 必须先在 Windows 与 VM 各跑一次、再把**两端产物汇总**后比对，属「跨机器编排」，塞进单平台门禁要么拿不到对端产物、要么退化成「只跑一端」。CI matrix（`ubuntu-latest` / `windows-latest`）也是各自独立 job、无产物汇总与交叉比对步骤，且改 CI 配置属 CODEBUDDY.md §10「人工独占」、超出本轮可写范围。因此 golden-hash diff SHALL 作为**人工 / 独立脚本流程**（编排者在 Windows + VM 各跑一次，用跨平台 Python 逐字节比对），在 `tasks.md` 第 5 组列清单；`verify_all.py` 双平台仍只验证工具链跨平台、MUST NOT 被表述为「已验证模拟内核确定性」（CODEBUDDY.md §2 / §4）。本 change 自身仍 MUST 通过 `openspec validate --strict`（`verify_all.py` 遍历所有 active change）。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **不改 CI / `tools/`，golden diff 走人工 / 独立流程** | **采纳** | 与「单平台门禁」语义一致；不碰根配置 / CI（§10 人工独占）；跨平台 diff 本就需要两端人工汇总。 |
| 把 golden diff 塞进 `verify_all.py` | 否决 | `verify_all.py` 单次单平台、拿不到对端产物；硬塞会引入「跨机器」假设、破坏跨平台单一入口与 `os.name` 内部判平台的纪律（CODEBUDDY.md §2）。 |
| 给 CI 加一个「汇总两端产物并 diff」的 job / artifact | 否决 | 改 `.github/workflows/ci.yml` 属人工独占、超出本轮范围；CI 两个 matrix job 无共享产物语义，改动大且与 ADR 0002 结论更新节奏（一次性验证）不匹配。 |
| 新增 `tools/golden_hash_diff.py` 独立工具脚本 | 备选（不在本轮范围） | 可作为将来人工流程的辅助，但本 change 明确「只写四件套、不碰 `tools/`」；比对用 Python 单行 / `diff` 即可，无需新脚本文件。列为 Open Questions。 |
| 借「CI 双平台 `verify_all.py` 全绿」直接宣称确定性已验证 | 否决 | 直接违反 CODEBUDDY.md §2 / §5 与 ADR 0002；工具链跨平台 ≠ 模拟内核逐帧绝对 hash 一致（见 Risks）。 |

**关联 ADR 0002 与跨平台确定性影响**：ADR 0002 的结论升级**只能**来自 golden-hash 两端逐字节 diff，不能来自 CI 全绿；明确这条边界可防止「Linux `cargo test --workspace` 全绿 = 确定性已达成」的误读（那只是 Linux 自身相对比较自洽）。`verify_all.py` 里的 `cargo test --workspace` 仍包含 `determinism.rs`，但它只跑相对比较；本决策把「绝对基线」与「相对门禁」两类验收清晰分离。CI 配置零改动 ⇒ 无新增生成字节 / 平台差异风险（ADR 0005 §3 的 `protoc 31.1` 纪律不受影响）。

**架构原则与分层依赖**：SRP——门禁入口单一职责、不背负跨机器编排；OCP——不改 `verify_all.py` / CI 扩展验证能力，靠人工流程；DIP——ADR 结论依赖实测证据而非门禁绿灯；CODEBUDDY.md §10 AI / 人工责任边界（CI / 发布 / 远端由人工独占）。

### D7：owner 归属——推荐 `sim-owner` 单 owner + 只读 `critic`（待拍板）

**决策**：推荐单一实现 owner = **`sim-owner`**（CODEBUDDY.md §9 可写 `crates/w3-core/`，守 ADR 0002），正好覆盖推荐落点 `crates/w3-core/examples/golden_hash.rs`；配只读 **`critic`**（复审确定性 / 所有权 / 许可证 / 编码 / 资产入库 / rustfmt）。`spec-steward` 只写 `openspec/changes/cross-platform-golden-hash/` 四件套 + 实施后 `docs/decisions/0002-determinism.md` 追加一条复核记录（D5）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 `critic`，`mode` 不使用 `bypassPermissions`。**待拍板项**：若 D1 改判落 `crates/w3-app/examples/`，`w3-app` **没有专属 owner**（`sim-owner` 管 `w3-core`、`content-owner` 管 `w3-content`、`client-owner` 管 `w3-client` / `w3-render`、`net-owner` 管 `w3-net` / `w3-relay`），须由编排者新建 / 指定一个 owner（中文 agent 定义在 `.codebuddy/agents/`，**不在 `spec-steward` 可写范围**），并在 `tasks.md` 同步改运行命令与 owner 说明；这会成为 `/opsx:apply` 的前置阻塞，故 D1 推荐 `w3-core` 落点。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **`sim-owner` 单 owner + 只读 `critic`（工具落 `w3-core/examples/`）** | **采纳（推荐）** | 落点与可写范围完全对齐、天然守 ADR 0002、无需新建 agent；一个 change 一个 owner。 |
| 工具落 `w3-app/examples/` 并由编排者新建 owner | 备选（待拍板） | `w3-app` 无专属 owner；新建中文 agent 定义超出 `spec-steward` 可写范围，会成为 apply 前置阻塞。 |
| `net-owner` 承接（因为有 `lockstep_client.rs` 逐帧打印先例） | 否决 | 本 change 明确否决复用联机日志、不碰 `w3-net` / `w3-relay`；介入即越权。 |
| `content-owner` 承接（因为 `w3-app` 注入边界） | 否决 | 本工具不读 `w3-content` / 导出 JSON，且不碰 `crates/w3-content/`。 |
| 由 `spec-steward` 直接写 `crates/` 下的 example | 否决 | 超出可写范围（`openspec/` / `docs/decisions/`），且用户明确「不要碰 `crates/`」。 |
| 多 owner 协作（如 `sim-owner` + `client-owner`） | 否决 | 违反「一个 change 一个 owner」，且会并发写同一 change 的文件。 |
| 无 `critic` 直接提交 | 否决 | 违反 CODEBUDDY.md §9 串联（`/opsx:apply` → `critic` → 本地 commit）。 |

**关联 ADR 0002 与跨平台确定性影响**：`sim-owner` 是 ADR 0002 的守门人，天然会核验「`w3-core` 零改动、`determinism.rs` 全绿、哈希 / 快照字段集合不变、不误报全平台达成」；`critic` 复跑 `cargo fmt --all -- --check` 与确定性 / 措辞审查。owner 划分本身不产生任何运行时状态、不进哈希 / 快照。把工具放在 `sim-owner` 范围内，也让「实施后 ADR 追加由 `spec-steward`」这条跨角色交接最清晰。

**架构原则与分层依赖**：SRP / 所有权边界——每个 crate 唯一实现所有者、避免并发写冲突；`critic` 只读（CODEBUDDY.md §9）；中文 agent 定义若需新建由编排者处理，本轮不碰 `.codebuddy/agents/`。

### D8：归档顺序与 delta 落点——独立 capability、`linux-winit-platform-backend` 之后归档（待拍板）

**决策**：只新增 `specs/cross-platform-determinism/spec.md`（全部 ADDED Requirements），MUST NOT 产生 `deterministic-simulation` 或其它主 spec 的 **MODIFIED delta**。理由：本 change 是「验证」而非行为变更，既有主 spec 的 requirement 行为全部不变（甚至零改动 `w3-core` 实现）；且 4 个 active change 中 `p6-playable-client` 持有 `3d-rendering` / `lockstep-networking` 的 active delta、`gameplay-command-wire-format` 与 `linux-winit-platform-backend` 同为 active，两条 change 同写同一 capability 会在归档时冲突。与 `gameplay-command-wire-format` (h) / `linux-winit-platform-backend` D7 口径一致。**归档顺序建议排在本 change 的 `/opsx:apply` + 双平台 diff 通过 + ADR 追加 + 人工 push + CI 通过之后，且在 `linux-winit-platform-backend` 之后**：VM 必须先能编译全 workspace（含呈现层平台后端）才跑得动 `cargo run -p w3-core --example golden_hash`（`cargo test --workspace` / `verify_all.py` 也依赖此前提）；提案阶段 `linux-winit-platform-backend` 的 VM 临时补丁只存在于 VM 副本、本机仓库零改动，须先正式落地；**实施后状态**：该 change 已由 `client-owner` 在本机正式落地、VM 已重新同步并能编译全 workspace，此归档前提已满足。因零 MODIFIED delta，与其余 3 个 active change 无 capability 争用；具体先后由编排者裁定（Open Questions）。归档后主 `openspec/specs/` 应只新增 `cross-platform-determinism/spec.md`，既有 7 个主 spec 零变化。

**备选方案（Alternatives considered）**：

| 方案 | 结论 | 理由 |
|---|---|---|
| **新增独立 capability `cross-platform-determinism`、零 MODIFIED delta、排在 `linux-winit-platform-backend` 之后归档** | **采纳（推荐）** | 契约边界清晰、归档时新增主 spec、与 active change 零争用；归档顺序尊重 VM 编译前提。 |
| 给 `deterministic-simulation` 写 MODIFIED delta（声明「确定性不变」） | 否决 | requirement 行为未变，用 MODIFIED 属于「无变化却改写」；且易与 `cross-platform-golden-hash` 之外的 change / 归档冲突；「不变」用 ADDED 里的「零改动」requirement 表达即可（同 `linux-winit-platform-backend` D7）。 |
| 复用 `deterministic-simulation` 只加 ADDED requirement | 否决 | 会与 `p6-playable-client` 等 active change 的既有 capability 争用；独立 capability 更贴「跨平台验证」这一新关注点（ISP）。 |
| 先于 `linux-winit-platform-backend` 归档 | 否决 | VM 尚不能编译全 workspace（winit 缺 Linux 后端 → `compile_error!`），golden-hash 工具与双平台门禁跑不起来，证据链不成立。 |
| 与 `p6-playable-client` / `gameplay-command-wire-format` 争用 capability | 否决 | 归档冲突；本 change 刻意独立。 |
| 不写 spec、只写 design / tasks | 否决 | 违反 schema `spec-driven` 的 artifact 顺序与 `openspec validate` 要求。 |

**关联 ADR 0002 与跨平台确定性影响**：把「确定性边界零改动」写进 ADDED requirement（spec「不新增权威状态、状态哈希与快照字段不变」），使 ADR 0002 的约束在 spec 层可验收，而非靠 MODIFIED 改写既有 requirement；不引入任何新权威状态概念，故无状态哈希 / 快照字段集合变化、不 bump `SNAPSHOT_VERSION`（当前 3）。归档顺序不影响运行时确定性，但影响「ADR 0002 结论升级」的证据是否成立——只有 VM 能编译 + 双平台 diff 为零，才有理由追加复核记录。

**架构原则与分层依赖**：SRP / ISP——一个 capability 只描述「跨平台 golden-hash 验证」这一关注点；主 specs 是 source of truth（CODEBUDDY.md §8），本轮不碰主 `openspec/specs/`，归档时才由工具吸收新增 capability；显式失败不静默降级——归档前 MUST 复核主 specs 收敛结果。

## Risks / Trade-offs

- **[Risk]** 浮点 / 编译器差异：不同 OS、rustc 后端或 CPU 上，即便 `SimFixed` 是整数数学，也可能因 `i128` 中间量、整数除法向零截断、`BTreeMap` 迭代、结构体布局或优化级别差异导致某帧 hash 不同 → **Mitigation**：这正是本 change 要暴露的问题；工具用 16 位定宽 hex 逐帧输出，diff 非零时定位到具体帧并用 `first_difference()`（公开 API）定位字段；ADR 0002 §1 未用超越函数 / `libm`、权威状态无 `f32` / `f64`，理论上应一致；若不一致，MUST 明确报告为「待调查的确定性缺陷」，MUST NOT 静默忽略或改基线迁就（显式失败不静默降级）。
- **[Risk]** `rustc` 版本漂移：本机 / VM 现为同一 `rustc 1.99.1`，ADR 0002 正文历史基线是 `1.91.0`；将来 stable 升级可能改变优化 / 布局，使旧 golden 文本失效 → **Mitigation**：汇总行只含编译期常量（帧数、最终 hash、`schema=golden-hash/1`），不含实跑工具链信息；两端 `rustc` 版本只在验证报告（`tasks.md` 关键结果与 ADR 0002 复核记录）中记录、不进基线文本，版本不同即视为不可比、需重生成基线；`rustc` 升级触发 ADR 0002 复核点「在非 Windows 平台首次实跑」的重新评估，列入 Open Questions；不改写 ADR 正文历史数值（D5）。
- **[Risk]** 只比最终 hash 漏掉中间帧：两平台可能某帧分歧后又收敛到相同最终 hash → **Mitigation**：D3 强制整份文本逐字节 diff、每帧一行、spec 有「中间帧分歧被捕获」Scenario；MUST NOT 用 `last_hash` / `inject_export.rs` 单 hash 口径。
- **[Risk]** 把 CI 误当跨平台验证：CODEBUDDY.md §2 / §5 明确 CI 双平台 `verify_all.py` 只验证工具链；有人可能据「Linux 全绿」宣称确定性已达成 → **Mitigation**：D6 明确 golden diff 不进 CI、ADR 结论只来自两端逐字节 diff；proposal / spec / design / tasks / commit 措辞一律限定「Windows 与 Linux (x86_64, rustc 1.99.1) 逐帧一致」，`critic` 复核措辞，不出现「全平台已达成」。
- **[Risk]** 误用联机 / 墙钟日志当基线：`lockstep_client.rs` 的 `last_hash` 在 Windows 与 VM 不同属正常（命令 `execute_frame` 依赖本地 tick、relay `sleep(33ms)`） → **Mitigation**：Context / D1 / D3 / spec 多处明确否决；工具离线、固定脚本、无墙钟；验收只采信 golden-hash stdout 文本。
- **[Risk]** 编码 / 换行造成假 diff：Windows 上 PowerShell 重定向会引入 UTF-16 / BOM / CRLF，导致逐字节比对假失败 → **Mitigation**：工具显式输出 LF、无 BOM、`0x<016x>` 定宽（D3）；比对用跨平台 Python `read_bytes()`；采集产物用 `cargo run ... > golden.txt` 时走 Git Bash / Python 写字节而非 PowerShell；`python tools/check_encoding.py` 覆盖四件套（CODEBUDDY.md §2）。
- **[Risk]** 提案阶段 VM 环境漂移 / 临时 winit 补丁误当已入库实现：当时 VM 的 `M Cargo.lock` + 两个 `Cargo.toml` 补丁只用于验证、本机仓库零改动 → **Mitigation**：归档顺序排在 `linux-winit-platform-backend` 之后（D8）；tasks 含「重新同步 VM 干净快照」步骤；先让本机与 VM 改动一致再跑门禁，绝不把 VM 补丁当实现；**实施后状态**：`linux-winit-platform-backend` 已由 `client-owner` 在本机正式落地、VM 已重新同步并 `Verification OK`（24 项全绿），此风险已关闭。
- **[Risk]** ADR 结论扩大到 macOS / 其它 CPU 架构 → **Mitigation**：D5 + spec「结论不扩大」Scenario 显式保留未验证边界；追加只写 Linux x86_64 + rustc 1.99.1 已实测；`critic` 复核措辞。
- **[Risk]** 工具落 `w3-app` 时 owner 缺位阻塞 `/opsx:apply` → **Mitigation**：D1 / D7 推荐 `w3-core/examples/` + `sim-owner`；若拍板改落 `w3-app`，由编排者先新建 / 指定 owner（超出 `spec-steward` 可写范围），列为 Open Questions 待拍板。
- **[Risk]** 归档时与其它 active change 争用 capability → **Mitigation**：D8 零 MODIFIED delta、只新增独立 capability；归档顺序与 `linux-winit-platform-backend` 解耦后由编排者裁定。
- **[Risk]** 把 golden 文本 / 导出数据 / 密钥写进仓库 → **Mitigation**：spec「无墙钟、网络、窗口、文件与资产依赖」requirement + 资产扫描不变（ADR 0001）；golden 文本只留平台本机、不入库；`critic` 复核 `git status`。

## Migration Plan

纯新增验证入口、无数据迁移、无协议 / 状态 / 快照迁移、无兼容垫片（BREAKING = 无）。步骤（与 `tasks.md` 一致，实施走 `/opsx:apply`）：

1. 前置：确认本机干净工作区 `e1ba8a4`、4 个 active change 状态、`openspec validate cross-platform-golden-hash --strict` 通过；由编排者拍板 D1 落点 / D7 owner / D5 ADR 追加 / D6 CI 边界 / D8 归档顺序。
2. 由单一 owner 在推荐落点新增 `golden_hash.rs` example：硬编码脚本 + 配置 + `open_world`、逐帧定宽 hex + 汇总行（D1 / D2 / D3），中文注释、`cargo fmt --all`。
3. 本机（Windows）`cargo run -p w3-core --example golden_hash > golden.windows.txt` 生成 Windows 基线；`cargo test -p w3-core --test determinism -- --nocapture` 确认内核零改动 / 全绿（D4）。
4. **重新同步 VM 干净快照**（含 `.git`，剔除临时 winit 补丁或待 `linux-winit-platform-backend` 正式落地后同步）；VM `cargo test --workspace` 全绿、`python3 tools/verify_all.py` → `Verification OK`。
5. VM `cargo run -p w3-core --example golden_hash > golden.linux.txt` 生成 Linux 基线；把两份文本汇总、用跨平台 Python 逐字节比对，diff 为零才通过（D3）。
6. diff 通过后由 `spec-steward` 在 `docs/decisions/0002-determinism.md` **只追加**一条复核记录（D5），不改正文。
7. 收尾门禁：`cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`、`python tools/verify_all.py`（Windows / VM 双平台）、只读 `critic` 复审。
8. 本地 commit（Conventional Commits 英文 type + **中文描述首行 ≤ 72 字符**），远端 `push` 由人工独占；人工 push + CI 双平台通过后由编排者 `openspec archive cross-platform-golden-hash -y`，确认主 `openspec/specs/` 仅新增 `cross-platform-determinism/spec.md`、既有 7 个主 spec 零变化（D8）。

**回滚策略**：`git revert` 掉新增的 example 文件（及实施后 ADR 追加的那一段）即可完全回到改动前状态；无运行时状态、无存档、无协议 / 快照版本需要回滚（未 bump `PROTOCOL_VERSION` / `SNAPSHOT_VERSION`）。golden 文本留在本机、不入库，无需清理仓库。ADR 复核记录若因后续发现 diff 非零需撤回，按「只追加一条更正记录」处理，不回改历史段落。

## Open Questions

- **工具落点（D1，待拍板）**：推荐 `crates/w3-core/examples/golden_hash.rs`（与被测对象同 crate、零新依赖、`sim-owner` 覆盖）；备选 `crates/w3-app/examples/golden_hash.rs`（`w3-app` 语义是注入边界、依赖 `w3-content`、且无专属 owner）。请编排者拍板，落点变则 `tasks.md` 的运行命令与 owner 说明同步调整。
- **owner 归属（D7，待拍板）**：推荐 `sim-owner` 单 owner + 只读 `critic`；若落 `w3-app`，须由编排者新建 / 指定 owner（`.codebuddy/agents/` 不在 `spec-steward` 可写范围），否则 `/opsx:apply` 阻塞。
- **ADR 0002 追加复核记录（D5，待拍板）**：本轮提案阶段不改 ADR；双平台 diff 通过后是否按建议「只追加一条、明确 macOS / 其它架构仍待验证」执行，请确认；`CODEBUDDY.md §1 / §2 / §5` 的「待验证」措辞同步时机由编排者 / `docs-owner` 裁定（本 change 不碰）。
- **CI 集成边界（D6，待拍板）**：判断为「不纳入 `verify_all.py` / CI、走人工 / 独立流程、不改 CI 配置」。若编排者希望将来新增独立 `tools/golden_hash_diff.py` 辅助脚本，属另一 change / 人工范围，本轮不写 `tools/`。
- **归档顺序（D8，待拍板）**：建议在 `linux-winit-platform-backend` 之后（VM 需能编译全 workspace）；相对 `p6-playable-client` / `gameplay-command-wire-format` 的先后由编排者按 CODEBUDDY.md §9 串联裁定。因零 MODIFIED delta，无 capability 争用。
- **推进帧数与脚本细节（D2，实施期确认）**：建议 40 帧（与 `determinism.rs` 交错测试窗口一致）、`seed 0x5eed / logic_fps 30 / content_hash 0xC0FFEE`、`StaticWorld::open_world(16, 16, SimFixed::ONE)`；具体常量在 `/opsx:apply` 由 owner 定稿，汇总行只含编译期常量（帧数、最终 hash、`schema=golden-hash/1`），脚本内容 MUST 在两端源码中逐字节相同。
- **`rustc` 基线记录方式（D3，已结案）**：已定案——`rustc` 基线不进基线文本，只在 ADR 0002 复核记录与验证报告（`tasks.md` 关键结果）中记录（实测两端 `rustc 1.99.1`）；汇总行只含编译期常量（帧数、最终 hash、`schema=golden-hash/1`），MUST NOT 含 `rustc` 版本 / 时间戳 / 路径 / 平台或运行时相关信息。
- **rustc / 平台漂移后续（ADR 0002 复核点）**：将来 macOS / ARM / 其它 CPU 架构或 rustc 升级时的重新验证，属 ADR 0002 复核点的后续 change，不在本 change 范围；本 change 不锁定 rustc 版本、不承诺跨运行时兼容。
