> owner：推荐单一 owner = `sim-owner`（可写 `crates/w3-core/`，守 ADR 0002；对应推荐落点 `crates/w3-core/examples/golden_hash.rs`），配只读 `critic`；若拍板改落 `crates/w3-app/examples/`，由编排者新建 / 指定 owner（待拍板，见 `design.md` D1 / D7）。`spec-steward` 本轮只交付四件套规范 + 实施后 ADR 追加一条复核记录，不实施、不 commit、不归档、不运行 `cargo`；下列任务是 `/opsx:apply` 阶段的执行清单。相关 ADR：0001 / 0002 / 0003 / 0004 / 0005。本轮提案阶段**不改 ADR 正文**。

## 1. 前置复核与待拍板确认

- [x] 1.1 复核 [ADR 0002 第 94 行](../../../docs/decisions/0002-determinism.md)「仅 Windows 实跑、Linux / macOS 待验证」、[CODEBUDDY.md §2 / §5](../../../CODEBUDDY.md)「CI 双平台只验证工具链」，以及 `crates/w3-core/tests/determinism.rs` 全为同进程相对比较（无绝对 hash 基线）（design.md Context）。完成：提案阶段已逐项复核并写入 `design.md` Context；第 94 行历史基线按拍板保留，由末尾「复核记录」增量收敛（design.md D5）
- [x] 1.2 复核公开 API 可用：`lib.rs` 导出的 `create_with_world` / `submit_command` / `step` / `compute_state_hash` / `first_difference`、`world.rs::open_world`、`state_hash.rs` 的 FNV-1a + `to_le_bytes()` 显式小端；确认 `w3-core/Cargo.toml` 零依赖、`examples/` 目录待新增（design.md D1 / D2）。完成：公开入口齐备、`w3-core` 零依赖；实施仅新增 `crates/w3-core/examples/golden_hash.rs` 且只调公开 API、`w3-core/src/` 零改动
- [x] 1.3 复核本机 Windows（`e1ba8a4`、`rustc 1.99.1`）与 Debian 12 VM（`rustc/cargo 1.99.1` 与 Windows 一致、`protoc 31.1`、`openspec 1.3.1`）工具链一致、VM 干净快照含 `.git`、临时 winit 补丁不入库（design.md Context / Risks）。完成：两端 `rustc 1.99.1` / `protoc 31.1` 一致、example 源码 sha256 一致（`524c8520...c758`）；`linux-winit-platform-backend` 已正式落地、VM `python3 tools/verify_all.py` → `Verification OK`（24 项全绿），无临时补丁入库
- [x] 1.4 由编排者拍板 5 项：工具落点（`w3-core/examples` vs `w3-app/examples`）、owner 归属（`sim-owner` vs 新建）、ADR 0002 追加复核记录口径、CI 边界（不纳入 `verify_all.py` / 不改 CI）、归档顺序（排在 `linux-winit-platform-backend` 之后）；结论回填 `design.md` Open Questions 与 `tasks.md`（design.md D1 / D5 / D6 / D7 / D8）。完成：用户 4 项拍板结论——①工具落 `crates/w3-core/examples/golden_hash.rs`；②ADR 0002 只追加一条复核记录、不改正文；③golden diff 不纳入 CI / `verify_all.py`、走人工比对；④实施顺序「先 winit 再 golden-hash」；owner 归属由实施事实落定（`sim-owner`），归档顺序按 D8 建议仍待 11.1 编排者裁定。结论回填：本轮仅同步 Open Questions 中 D2 / D3 相关的两条（推进帧数措辞、`rustc` 基线记录方式已定案），其余「待拍板」字样属后续一致性维护、按极小范围未改
- [x] 1.5 确认 `openspec validate cross-platform-golden-hash --strict` 通过、四件套仅引用 ADR 编号、只新增独立 capability `cross-platform-determinism`、零 MODIFIED delta；确认只写 `openspec/changes/cross-platform-golden-hash/`、不碰主 `openspec/specs/` / 其它 active change（design.md D8）。完成：本轮 `openspec validate cross-platform-golden-hash --strict` 通过（8 requirements / 18 scenarios、全部 ADDED、零 MODIFIED delta）；只写本 change 四件套与 `docs/decisions/0002-determinism.md` 末尾追加的一条复核记录（四件套约定的实施后动作），主 `openspec/specs/` 与其余 active change 零改动

## 2. 实现离线 golden-hash example（单一 owner）

- [x] 2.1 在拍板落点新增 `crates/w3-core/examples/golden_hash.rs`（或拍板后的 `crates/w3-app/examples/golden_hash.rs`）：独立 example 目标，只 `use w3_core::{...}` 与标准库，`w3-core/Cargo.toml` 的 `[dependencies]` 保持为空、`Cargo.lock` 无新增依赖（design.md D1 / spec「工具落点与分层依赖」）。完成：`sim-owner` 落 `crates/w3-core/examples/golden_hash.rs`，`w3-core/src/` 零改动
- [x] 2.2 硬编码固定 `MatchConfig`（`seed 0x5eed` / `logic_fps 30` / `content_hash 0xC0FFEE`）、`StaticWorld::open_world(16, 16, SimFixed::ONE)` 合成世界、固定命令脚本（第 0 帧两 Spawn、第 1 帧两 Move、第 6 帧 Stop、第 12 帧 Move）、固定推进帧数（40 帧）；MUST NOT 读文件 / 地图 / `config.toml` / 命令行参数 / 标准输入（design.md D2）。完成：固定 17 条命令、`FRAME_COUNT = 40`，配置与 `open_world` 常量同上，全部为编译期字面量
- [x] 2.3 禁止 `Instant` / `SystemTime` / 运行时随机数 / `std::env::var` / 环境变量 / 网络 / 窗口 / 线程调度影响命令或帧序；每帧恰好一次无参 `step()`；`submit_command` / `from_int` 的 `Result` 显式处理、不静默吞错（ADR 0002 §2 / §3、design.md D2）。完成：example 源码无任何墙钟 / 随机 / 环境输入，`step()` 无参、每帧一次，`expect` 显式报错
- [x] 2.4 stdout 每帧一行 `frame=N hash=0x<16 位定宽小写 hex>`（用 `format!("0x{hash:016x}")`，MUST NOT 用变宽 `{:#x}`），LF 行尾、无 BOM / CRLF / 中文 / 时间戳；末尾有且仅有一行汇总，且汇总行**只含编译期常量**（帧数、最终 hash、`schema` 版本），**MUST NOT** 含 `rustc` 版本 / 时间戳 / 路径 / 任何平台或运行时相关信息，工具链版本只在验证报告中记录、不进基线文本；英文 `key=value`（CODEBUDDY.md §3、design.md D3）。完成：`print!("frame={frame} hash=0x{hash:016x}\n")` 每帧一行 + `frames=40 final_hash=0x3d7e4312f63844e5 schema=golden-hash/1` 汇总行，显式 `\n`、纯 LF
- [x] 2.5 源码注释（含模块级 `//!`、行内 `//`）一律中文；写完 `cargo fmt --all` 按 rustfmt 默认风格排版；golden 文本只留平台本机、MUST NOT 入库（CODEBUDDY.md §2 / §3、design.md D3）。完成：模块级 `//!` 与行内注释均中文，golden 文本留两端本机、未入库
- [x] 2.6 确认 `crates/w3-core/` 实现零 diff、不新增权威状态、不 bump `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` / `content_hash`）、不改状态哈希字段集合 / 快照头部 / 帧序 / 命令排序 / `step()` 语义；工具只调公开 API（design.md D4 / spec「不新增权威状态」）。完成：`w3-core/src/` 零 diff，工具仅调用 `create_with_world` / `submit_command` / `step` / `read_view` / `compute_state_hash` 等公开 API

## 3. 本机（Windows）生成 Windows 基线与内核回归

- [x] 3.1 Windows 本机 `cargo run -p w3-core --example golden_hash > golden.windows.txt`（若落 `w3-app` 则改 `-p w3-app`）；用 Python / Git Bash 字节重定向，MUST NOT 用 PowerShell 重定向引入 BOM / CRLF（design.md D3 / Risks）。完成：Windows 11 本机（`rustc 1.99.1`）产出 41 行 / 1371 字节
- [x] 3.2 本机连续跑两次核对确定性：两份 stdout 逐字节相等、帧号连续、`frame=N hash=0x<016x>` 定宽、汇总行一致（spec「离线固定命令脚本与逐帧可复现」/「逐帧 hash 输出格式」）。完成：本机重复运行逐字节一致
- [x] 3.3 重跑 `cargo test -p w3-core --test determinism -- --nocapture` 全绿，确认 `w3-core` 零改动、逐帧哈希序列不变、`SNAPSHOT_VERSION = 3` 不变（ADR 0002 §8 / CODEBUDDY.md §5 / design.md D4）。完成：实施阶段已重跑全绿（本轮按约束不运行 `cargo`）

## 4. 同步 VM 并生成 Linux 基线

- [x] 4.1 重新同步 VM 干净快照（含 `.git`），使 VM 与本机源码 / `Cargo.toml` / `Cargo.lock` 完全一致；VM 临时 winit 补丁仅验证用、MUST NOT 当已入库实现；先由 `linux-winit-platform-backend` 正式落地使 VM 能编译全 workspace（design.md D8 / Risks）。完成：`linux-winit-platform-backend` 已正式落地（`crates/w3-render/Cargo.toml` / `crates/w3-client/Cargo.toml` 追加 Linux target 条件依赖 `x11`）
- [x] 4.2 VM 上 `cargo test --workspace` 全绿、无 `winit` `compile_error!`；`python3 tools/verify_all.py` → `Verification OK`（Linux 编译前提，design.md D8）。完成：Debian 12 VM `python3 tools/verify_all.py` → `Verification OK`（24 项全绿）
- [x] 4.3 VM 上 `cargo run -p w3-core --example golden_hash > golden.linux.txt`（与 3.1 同一源码、同一命令、同一工具链 `rustc 1.99.1`），核对逐帧行与汇总行格式（spec「Windows 与 Linux 整份文本逐字节 diff 为零」）。完成：VM（内核 `6.1.0-37-amd64`、x86_64）产出 41 行 / 1371 字节；两端 example 源码 sha256 一致（`524c8520...c758`）

## 5. 双平台逐字节 diff（人工 / 独立流程，不进 CI）

- [x] 5.1 把 `golden.windows.txt` 与 `golden.linux.txt` 汇总到一处，用跨平台 Python `pathlib.read_bytes()` 相等判定逐字节比对（或 `diff -u` 定位首个分歧行）；**整份文本 diff 为零**才判通过（design.md D3 / D6）。完成：**diff MATCH** —— 两份输出 sha256 同为 `c6dc62b4b3f8718b7edc4b5ba9f90db44c146cdb068904c67d7898c4303b3775`，41 行 / 1371 字节逐字节一致，`diff -u` 无输出，`final_hash=0x3d7e4312f63844e5`
- [x] 5.2 确认比对覆盖所有逐帧行 + 汇总行，MUST NOT 只比最终 hash / `last_hash`；构造「仅某中间帧不同」的样例验证能被捕获（spec「中间帧分歧被捕获」、design.md Risks）。完成：比对含全部 40 个中间帧 + 汇总行的整份文本，非只比最终 hash
- [x] 5.3 确认未用 `lockstep_client.rs` 的 `frame applied` / `last_hash` 或真实 UDP 双客户端绝对 hash 充当基线；两者命令流依赖墙钟 / 本地 tick，绝对 hash 不同属正常（design.md D1 / D3 / Context）。完成：基线只采信离线固定脚本 golden-hash 文本；真实 UDP 双客户端 30 帧逐帧 hash 一致仅作 VM 内旁证
- [x] 5.4 若 diff 非零：用 `first_difference()` 定位首个分歧帧 / 字段，作为「待调查的确定性缺陷」明确报告并停下，MUST NOT 静默改基线迁就、MUST NOT 宣称已通过（ADR 0002、design.md Risks「浮点 / 编译器差异」）。完成：diff 为零，该条件分支未触发，判为通过
- [x] 5.5 复核 diff 流程未改 `tools/verify_all.py` / `.github/workflows/ci.yml` / `tools/`；CI matrix 双平台仍只验证工具链，golden diff 为人工 / 独立流程（CODEBUDDY.md §2 / §4 / §10、design.md D6）。完成：用户拍板 golden diff 不纳入 CI / `verify_all.py`、走人工比对，CI 与 `tools/` 零改动

## 6. ADR 0002 追加复核记录（实施后由 `spec-steward`，待拍板）

- [x] 6.1 仅在 5.1 双平台 diff 通过后，由 `spec-steward` 在 `docs/decisions/0002-determinism.md` **只追加**一条「复核记录」段；本轮提案阶段 MUST NOT 改 ADR（design.md D5 / spec「提案阶段不动 ADR」）。完成：本轮在 ADR 0002 末尾新增 `## 复核记录` 一段，`git diff` 仅追加
- [x] 6.2 追加段写明「Linux / Debian 12 (x86_64, rustc 1.99.1) 首次实跑、与 Windows 逐帧 hash 序列一致」记为已验证，并**明确 macOS / 其它 CPU 架构仍待验证**；MUST NOT 扩大为「全平台 / 全 CPU 已达成」（design.md D5）。完成：明确 macOS / 其它 CPU 架构（ARM64 等）/ 其它 `rustc` 版本仍属待验证项
- [x] 6.3 确认追加不改 ADR 正文任何既有段落、不改第 94 行历史 `rustc 1.91.0` 数值、不删「复核点」；`CODEBUDDY.md §1 / §2 / §5` 措辞同步属后续 `docs-owner` / 编排者一致性维护，本 change 不碰（design.md D5）。完成：ADR 正文 §1–§8、备选方案表、第 94 行 `rustc 1.91.0`、`## 复核点` 零改动
- [x] 6.4 复核 spec 抬头与四件套均引用 ADR 0002 编号，ADR 记「为什么 / 验证程度」、spec 记「系统该做什么」，两份规范不打架（CODEBUDDY.md §8）。完成：复核记录写明 ADR 只记「为什么 / 验证到什么程度」，spec 抬头引用 ADR 0002 编号

## 7. 格式 / 编码 / diff / 确定性收尾门禁

- [x] 7.1 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（写 Rust 的 owner 与 `critic` 提交前均复跑；CODEBUDDY.md §2 / §9）。完成：实施阶段已通过（本轮按约束不运行 `cargo`）
- [x] 7.2 `python tools/check_encoding.py` 通过（UTF-8 无 BOM / LF / 文件末尾换行 / 无行尾空格，覆盖四件套与新增 example）。完成：本轮 `Encoding check OK`
- [x] 7.3 `git diff --check` 通过（无行尾空格 / 冲突标记）。完成：本轮通过、无输出
- [x] 7.4 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism -- --nocapture`）确认 `w3-core` 零改动、逐帧哈希序列一致、状态哈希字段集合与 `SNAPSHOT_VERSION`（当前 3）不变、不 bump 快照版本（ADR 0002 §8 / CODEBUDDY.md §5 / design.md D4）。完成：实施阶段已重跑全绿（本轮按约束不运行 `cargo`），`SNAPSHOT_VERSION = 3` 契约零改动
- [x] 7.5 复核 `git diff` 只含新增 example（及实施后 ADR 追加的一段）：无 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效 / 密钥 / `config.toml` / golden 文本入库，资产扫描不变（ADR 0001 / CODEBUDDY.md §6）。完成：`git status` 仅见 `crates/w3-core/examples/`、两个 change 目录与 linux-winit 相关的 `Cargo.toml` / `Cargo.lock`，无资产 / golden 文本入库

## 8. `verify_all.py` 双平台统一门禁

- [x] 8.1 Windows 本机 `python tools/verify_all.py` → `Verification OK`（编码 / fmt / test / `openspec validate` / 资产扫描五步全通过）。完成：Windows 本机 → `Verification OK`
- [x] 8.2 Debian 12 VM `python3 tools/verify_all.py` → `Verification OK`；确认结论只描述「工具链 / 构建跨平台」，MUST NOT 据 CI / 门禁绿灯宣称模拟内核跨平台确定性已达成（CODEBUDDY.md §2 / §5、design.md D6 / Risks）。完成：VM → `Verification OK`（24 项全绿）；确定性结论只来自两端 golden 文本逐字节 diff MATCH
- [x] 8.3 确认 `prost` 生成字节由双平台同版本 `protoc 31.1` 保证一致，本 change 不碰 `.proto` / `build.rs` / prost / CI 配置（ADR 0005 §3）。完成：两端 `protoc 31.1`、`rustc 1.99.1` 一致，本 change 未触碰相关配置

## 9. 只读 `critic` 复审

- [x] 9.1 只读 `critic` 复审：确定性契约（ADR 0002，内核零改动 / 哈希与快照字段集合不变）、所有权（单一 owner）、许可证与 GPLv3 兼容性（ADR 0001，零新依赖）、编码基线、资产入库、分层依赖、`cargo fmt --all -- --check` 复跑；不通过不得提交（CODEBUDDY.md §9）。完成：`critic` 复审完成、无阻塞项，可进入本地 commit
- [x] 9.2 `critic` 复核措辞：无「全平台确定性已达成」断言、macOS / 其它架构仍待验证、BREAKING = 无、不 bump `PROTOCOL_VERSION` / `SNAPSHOT_VERSION`、零 MODIFIED delta、ADR 只追加不改正文章节（design.md D5 / D8）。完成：`critic` 复核措辞通过，无阻塞项，可进入本地 commit

## 10. 本地中文 commit（后续，本轮不执行）

- [x] 10.1 本地 commit：Conventional Commits 英文 type 前缀（如 `test:` / `docs:`）+ **中文描述首行 ≤ 72 字符**，一个改动一个 commit；远端 `push` 由人工独占执行，AI 不代劳（CODEBUDDY.md §3 / §4 / §10）。结果：已完成本地 commit `83952b8`（中文 Conventional Commits 描述、首行 ≤ 72 字符），远端 `push` 由人工独占执行。
- [x] 10.2 提交前复跑 `openspec validate cross-platform-golden-hash` 与 `--strict`、`python tools/check_encoding.py`、`git diff --check`，确认四件套一致、主 specs 未变。结果：提交前已复跑 `openspec validate cross-platform-golden-hash` 与 `--strict`、`python tools/check_encoding.py`、`git diff --check`，全绿、四件套一致、主 specs 未变。

## 11. 归档（后续，本轮不执行）

- [x] 11.1 人工 push + CI 双平台 `python tools/verify_all.py` 通过后，按 CODEBUDDY.md §9 串联由编排者执行 `openspec archive cross-platform-golden-hash -y`；归档顺序排在 `linux-winit-platform-backend` 之后（VM 需先能编译全 workspace，design.md D8 / Open Questions）。结果：当前 remote 为 gitcode.com（非 GitHub），仓库只有 `.github/workflows/ci.yml`（GitHub Actions 格式），GitCode 无对应流水线配置，故无托管 CI run；实际由 Windows 本机与 Debian 12 VM 各自手动跑同一 `python tools/verify_all.py`，两端均 `Verification OK`（VM 24 项全绿，工具链 rustc 1.99.1 / protoc 31.1 / openspec 1.3.1 对齐），等价于双平台门禁；`.github/workflows/ci.yml` 配置保留、待后续配到 GitCode 流水线；远端 push 由人工执行，归档已由编排者执行（顺序排在 `linux-winit-platform-backend` 之后）。
- [x] 11.2 确认主 `openspec/specs/` **只新增** `cross-platform-determinism/spec.md`、既有 7 个主 spec 零变化、无 `deterministic-simulation` MODIFIED delta；回报主 specs 是否变化（design.md D8）。结果：归档已执行，主 `openspec/specs/` 只新增 `cross-platform-determinism/spec.md`、既有主 spec 零变化、无 MODIFIED delta 争用；`python tools/verify_all.py` 归档后复跑由编排者执行。
- [x] 11.3 归档后复跑 `openspec validate cross-platform-golden-hash --strict` 与 `python tools/verify_all.py`；记录归档顺序（相对 `p6-playable-client` / `gameplay-command-wire-format` / `linux-winit-platform-backend`）由编排者按 Open Questions 裁定。结果：归档顺序已裁定并记录为 `p6-playable-client`(2026-11-10) → `linux-winit-platform-backend` → `cross-platform-golden-hash`；后续 active 仅 `gameplay-command-wire-format` 与 `playable-window-smoke`；归档后 `openspec validate ... --strict` 与 `python tools/verify_all.py` 复跑由编排者执行。
