## Why

> 路线图阶段（与 `gameplay-command-wire-format` (f) / `linux-winit-platform-backend` 口径一致）：按 `openspec/config.yaml` rules.proposal 的 P0–P5 阶段枚举核对，本 change **不新增玩法阶段**；相对 [ADR 0003](../../../docs/decisions/0003-scope.md) 分期表已有的 P6「集成收尾」行，标注为 **「P6 集成收尾的后续补齐」**，不单列独立 P7，`config.yaml` rules.proposal 的「P0-P5」枚举保持不动。相关 ADR：[ADR 0001](../../../docs/decisions/0001-license.md)（资产与法律）、[ADR 0002](../../../docs/decisions/0002-determinism.md)（确定性契约）、[ADR 0003](../../../docs/decisions/0003-scope.md)（范围与分期）、[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)（冻结坐标 / 拒绝 `f64`）、[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md)（CI 双平台生成字节一致）。

[ADR 0002](../../../docs/decisions/0002-determinism.md) 第 94 行明确：「目前仅在本机 Windows 11 / rustc 1.91.0 实跑通过；Linux、macOS 与其他 CPU 架构的跨平台确定性尚未实测，属于待验证项，而非已达成结论」；[CODEBUDDY.md §2 / §5](../../../CODEBUDDY.md) 同样规定 CI 双平台跑同一 `python tools/verify_all.py` 只验证**工具链**跨平台，**不等于**模拟内核跨平台确定性已达成。根本缺口在验收手段：现有 `crates/w3-core/tests/determinism.rs` 全部是**相对比较**（双实例 `compute_state_hash()` 相等、`first_difference()` 返回 `None`、`original` vs `restored`），两个实例始终在**同一进程、同一 OS、同一 rustc、同一 CPU** 上运行。因此 Linux 上 `cargo test --workspace` 全绿只证明「Linux 自身自洽」，无法证明 Windows 与 Linux 对**同一固定命令脚本**产出逐帧相同的 hash 序列——仓库里没有任何「绝对 hash 基线 / golden hash」可跨平台 diff。`crates/w3-app/examples/inject_export.rs:80` 只打印一个最终 `state_hash` 且依赖不入库的本机导出地图（ADR 0001）；`crates/w3-net/examples/lockstep_client.rs` 逐帧打印 `hash=0x...` 但依赖 UDP + relay 墙钟节拍与本地 tick 推导 `execute_frame`，不同机器 / 不同次运行命令流时序不同，绝对 hash 不同属正常，不能拿来做基线。

编排者本轮已具备双平台实跑环境：Windows 本机 `c:/open_rts/rust_dazhang`（HEAD `e1ba8a4`，`rustc 1.99.1`、`protoc 31.1`、`openspec 1.3.1`，`python tools/verify_all.py` → `Verification OK`）与 Debian 12 VM（`/srv/rust_dazhang` 为本机干净工作区 tar 快照、含 `.git`，`rustc/cargo 1.99.1` 与 Windows **完全一致**、`protoc 31.1`、`openspec 1.3.1`）；提案阶段 VM 在临时 winit 补丁（`linux-winit-platform-backend` change 正在正式修、本机仓库尚未改）后 `cargo test --workspace` 24 项全绿、`verify_all.py` → `Verification OK`、真实 UDP 双客户端 30 帧逐帧 hash 序列完全一致。本 change 据此新增一个**离线、固定命令脚本、无墙钟、无网络、无窗口、零新依赖**的 golden-hash 可执行入口，产出逐帧 hash 文本供 Windows / Linux **逐字节 diff**，把 ADR 0002 的 Linux 待验证项收敛为「已验证」；这是**对确定性契约的验证而非改动**。**实施后状态**：`sim-owner` 已新增 `crates/w3-core/examples/golden_hash.rs`，Windows 与 VM 两端输出 sha256 同为 `c6dc62b4b3f8718b7edc4b5ba9f90db44c146cdb068904c67d7898c4303b3775`、41 行 / 1371 字节逐字节 MATCH，ADR 0002 已只追加复核记录；`linux-winit-platform-backend` 已由 `client-owner` 正式落地、`critic` 复审完成、无阻塞项，`/opsx:apply` 已完成。

## What Changes

- 新增离线可执行入口（推荐 `crates/w3-core/examples/golden_hash.rs`，落点为待拍板项，见 `design.md` D1）：只依赖 `w3-core` 公开 API 与标准库，不读文件 / 网络 / 墙钟 / 环境变量、不需 GPU / 窗口、不读 `config.toml`、不含任何游戏资产（ADR 0001）。
- **硬编码**确定性输入：固定命令脚本（参照 `determinism.rs::script()`：第 0 帧两个 Spawn、第 1 帧 Move、第 6 帧 Stop、第 12 帧 Move）、固定 `seed`、`logic_fps = 30`、`content_hash`、用 `StaticWorld::open_world(...)` 合成的静态世界（不读地图）。相同字节输入 → 相同字节输出。
- 输出到 stdout：每帧一行 `frame=N hash=0x...`（`0x` + 16 位定宽小写十六进制，FNV-1a 结果），末尾一行汇总，只含编译期常量（帧数 + 最终 hash + `schema` 版本），不含 `rustc` 版本 / 时间戳 / 路径等平台相关信息（工具链版本只在验证报告中记录、不进基线文本），显式 LF 行尾，便于 `diff` / 逐字节比对。
- 验收口径：Windows 跑一次、VM 跑一次、**整份文本逐字节 diff 为零**才算通过；必须比对完整逐帧序列（中间帧分歧也要捕获），不能只比最终 hash。
- **BREAKING = 无**：不新增权威状态字段、不 bump `SNAPSHOT_VERSION`（当前 **3**）、不改状态哈希字段集合与快照头部布局、不改帧序 / 命令排序 / `step()` 语义；`w3-core` 的确定性契约**零改动**（工具只调用公开 API）。
- ADR 0002 结论更新（实施后、由 `spec-steward`）：双平台 diff 通过后在 `docs/decisions/0002-determinism.md` **只追加**一条「复核记录」段，把「Linux / Debian 12 (x86_64, rustc 1.99.1) 首次实跑、与 Windows 逐帧 hash 序列一致」记为已验证，同时明确 **macOS / 其它 CPU 架构仍是待验证项**（不得扩大结论）。**本轮提案阶段不改 ADR 正文**，只把「实施后追加」写进 `design.md` D5 与 `tasks.md`（待拍板项）。
- CI 集成边界（判断见 `design.md` D6）：**不**把 golden-hash diff 纳入 `tools/verify_all.py` / CI 配置；跨平台 diff 需两端产物汇总，属人工 / 独立流程，不改 `tools/`、不改 `.github/workflows/ci.yml`（CI 改动属 CODEBUDDY.md §10 人工独占）。

## Capabilities

### New Capabilities

- `cross-platform-determinism`: 跨平台确定性 golden-hash 验证契约——离线固定脚本可复现（相同字节输入 → 相同字节输出）、逐帧 hash 输出格式与汇总行、Windows / Linux 逐字节 diff 为零、无墙钟 / 网络 / 窗口 / 资产依赖、不新增权威状态且状态哈希 / 快照字段集合不变、ADR 0002 结论只收敛到已实测平台不扩大到 macOS / 其它架构。

### Modified Capabilities

（无。）**刻意只新增独立 capability、不产生 `deterministic-simulation` 的 MODIFIED delta**：本 change 是「验证」而非行为变更，既有主 `openspec/specs/deterministic-simulation/spec.md` 的 requirement 行为全部不变（本 change 甚至零改动 `w3-core` 实现）；且 `p6-playable-client` / `gameplay-command-wire-format` / `linux-winit-platform-backend` 同为 active change，两条 change 同写同一 capability 会在归档时争用。与 `gameplay-command-wire-format` (h) / `linux-winit-platform-backend` D7 口径一致，理由见 `design.md` D8。本轮不碰主 `openspec/specs/`、不改任何既有 change 的 spec 实质。

## Impact

- **确定性契约（ADR 0002）——这是验证而非改动**：不新增任何权威状态字段，故无字段需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照；状态哈希字段集合、`first_difference()` 语义、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、30 Hz 整数帧、无参 `RtsMatch::step()` 全部不变、不 bump 快照版本。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（确认零改动）。本 change **首次**把 ADR 0002 的 Linux (x86_64) 待验证项收敛为「已验证」，但 **macOS / 其它 CPU 架构仍属待验证项**，不得扩大结论；`rustc` 版本漂移属 ADR 0002 复核点（见 `design.md` Risks）。
- **资产与法律（ADR 0001）**：工具不读 `config.toml`、不读本机导出地图 / 任何 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效；命令脚本是硬编码整数参数而非游戏资产；golden 文本为验证产物、留在平台本机、**不入库**。本 change **不引入新依赖**（复用 `w3-core` 标准库）；四个只读参考项目（含 AGPLv3 的 `WarsmashModEngine`）不作代码复制。
- **分层依赖**：推荐落 `crates/w3-core/examples/`（example 是独立可执行目标，不进 lib、不增加 `[dependencies]`，保持 `w3-core` 零依赖与离线可编译；不反向依赖上层）；若拍板改落 `crates/w3-app/examples/`，须注意 `w3-app` 依赖 `w3-content`（读导出 JSON）、语义是「注入边界」，与本工具「不读文件 / 不需要内容层」错配（ISP / SRP），且 `w3-app` **无专属 owner**（见 `design.md` D1 / D7）。表现层不反写模拟状态，唯一权威写入者仍是 `RtsMatch`；`w3-content` 的 `f64` 冻结边界（ADR 0004）不受影响。
- **CI 边界（CODEBUDDY.md §2 / §4 / §10）**：`tools/verify_all.py` 是单次单平台门禁；跨平台 diff 需要两端产物汇总后比对，不适合塞进单平台门禁或 CI matrix。本 change **不改 CI 配置 / 不改 `tools/`**；`verify_all.py` 双平台仍只验证工具链跨平台，golden-hash diff 列为人工 / 独立流程（见 `design.md` D6）。`openspec validate` 会遍历所有 active change，本 change 自身 MUST 通过 `openspec validate --strict`。
- **ADR 0002 结论更新**：实施后由 `spec-steward` 在 `docs/decisions/0002-determinism.md` **只追加**一条「复核记录」段（不改正文章节、不改第 94 行历史 `rustc 1.91.0` 数值）；`CODEBUDDY.md §1 / §2 / §5` 与 ADR 正文的「待验证」措辞同步属后续 `docs-owner` / 编排者一致性维护，**不在本轮可写范围、本 change 不碰**。待拍板项见 `design.md` D5 / Open Questions。
- **owner 归属（待拍板）**：建议单一 owner = **`sim-owner`**（若工具落 `crates/w3-core/examples/`，正好在其可写 `crates/w3-core/` 范围、守 ADR 0002），配只读 **`critic`**；若落 `w3-app/examples/` 则需编排者新建 / 指定 owner（`w3-app` 无专属 agent 定义，且不在 `spec-steward` 可写范围）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 `critic`（见 `design.md` D7）。
- **归档顺序（待拍板）**：本 change 建议在 `linux-winit-platform-backend` **之后**归档——VM 需先能编译全 workspace（含呈现层平台后端）才跑得动 golden-hash 工具；因 D8 零 MODIFIED delta，与其余 3 个 active change 无 capability 争用（见 `design.md` D8 / Open Questions）。
- **规范边界**：本轮只写 `openspec/changes/cross-platform-golden-hash/` 下四件套；不碰主 `openspec/specs/`、其它 change 目录、`crates/`、`.codebuddy/agents/`、`NOTICE.md`、`CODEBUDDY.md`、根配置、`tools/`、ADR 正文；不运行 `cargo`、不 commit、不归档、不执行任何 git 远端命令。
