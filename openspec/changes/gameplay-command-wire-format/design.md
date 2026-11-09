## Context

P0–P5 已交付并归档，P6 `p6-playable-client` 已实施完成（54/55 tasks，**尚未归档**，剩归档动作），主 `openspec/specs/` 现有 7 个 capability。P6 的 `crates/w3-client/src/net_driver.rs::StepDriver::enqueue` 对 `ClientCommand::Gameplay(_)` 显式返回 `ClientError::GameplayUnsupportedOnline`（英文日志 `reason=wire_format_command_only`），因为 [ADR 0005](../../../docs/decisions/0005-protobuf-payload.md) 的 protobuf 线格式只承载 `CommandKind`。该边界在 `p6-playable-client/design.md` 的 Risks「已裁定·已知边界」条登记，并被 `critic` 三轮复审定性为「非阻塞的合规显式失败」；编排者裁定「补齐需扩展 schema 并 bump `PROTOCOL_VERSION`，另开独立 change」。本 change 即该独立 change。

源码核对事实（本 design 的依据，均来自 `crates/` 实读，非摘要转述）：

- `w3-net/src/proto/w3net.proto`：`Command.kind` oneof 占用 tag 4–10（7 变体），`reserved 11 to 16`（**仅 6 位**）；`CommandBatch { repeated Command commands = 1; reserved 2 to 8; }`；`FrameCommands { uint64 frame = 1; CommandBatch commands = 2; reserved 3 to 6; }`；`ResyncReplay { repeated FrameCommands frames = 1; reserved 2 to 6; }`；控制包各自 `reserved`。
- `w3-net/src/payload.rs`：`to_pb_kind(&CommandKind)` / `from_pb_kind(pb::command::Kind)` 是**双向穷尽 match**；顶层编解码 `encode_commands` / `decode_commands` 的消息是 `CommandBatch`（`Input` / `Frame` 包直接用它，`FrameCommands.commands` 嵌套它）；Rust 侧 `FrameCommands { frame, commands: Vec<Command> }`；`payload_resync_replay` / `parse_resync_replay`；`MAX_PAYLOAD = u16::MAX`。
- `w3-net/src/packet.rs`：`MAGIC = b"W3N1"`、`HEADER_LEN = 25`、`PROTOCOL_VERSION: u8 = 2`；`decode` 对 `version != PROTOCOL_VERSION` 返回 `NetError::VersionMismatch`；测试断言 `bytes[4] == 2`。
- `w3-net/src/frame.rs`：`stabilize(&mut [Command])` 按 `(execute_frame, player_id, sequence)` 稳定排序；`apply_frame(&mut RtsMatch, &[Command])` = 依序 `submit_command` → **恰好一次** `sim.step()` → `compute_state_hash()`；`FrameScheduler` / `FrameAssembler` / `take_ready` / `poll`。
- `w3-net/src/session.rs`：`ClientSession` 不持有 `RtsMatch`；`submit_input` 构造 `Input` 包走上行广播；`pop_ready`。
- `w3-core/src/gameplay.rs`：`GameplayCommandKind` 10 变体，字段为 `EntityId`(u64) / `DefId`(u32) / `EffectDef { EffectKind 枚举, i64, u32 }` / `bool` / `u8` / `u32`，**全整数无浮点**；`GameplayCommand` 携带 `execute_frame / player_id / sequence / kind / arrival`（`arrival` 私有、提交时分配）；`GameplayCommandQueue` 有**独立** `next_arrival`。
- `w3-core/src/sim_match.rs`：`submit_command` → `CommandQueue`，`submit_gameplay_command` → `validate_gameplay_command` → `gameplay.commands`（`GameplayCommandQueue`）；`step()` 在**阶段 1** 冻结普通命令、**阶段 7** 冻结玩法命令、**阶段 12** `clock.advance()`——一次调用内两类命令都被消费，两条队列的 `arrival` 计数器相互独立。
- `w3-core/src/gameplay.rs::WorldGameplayState::hash` → `GameplayCommandQueue::hash`：`next_arrival` 与 pending 命令（含 `arrival`）**已进入 FNV-1a 状态哈希**；故本 change **无需新增权威状态字段**，只需保证两端提交子序列一致。
- **`w3-relay/src/lib.rs`（编排者摘要未列、源码核对补充）**：`PacketKind::Input` 用 `decode_commands`、`broadcast_frame` 用 `encode_commands`、`CommandLog` 存 `FrameCommands`、`FrameAssembler` 装配帧——本 change 必改点，且 CODEBUDDY.md §9 的 owner 表中**没有任何 agent 拥有 `crates/w3-net/` 或 `crates/w3-relay/`**。

约束：确定性是硬门禁（ADR 0002）；资产与法律（ADR 0001）；第一版 Melee 优先（ADR 0003）；冻结坐标（ADR 0004）；线格式折中（ADR 0005，§3 canonical 约束、§4 红线、末尾「复核点」）。8 项待拍板决策已全部由用户拍板（均采纳推荐），本轮据以定案。本轮写 `openspec/changes/gameplay-command-wire-format/` 四件套并**只在** `docs/decisions/0005-protobuf-payload.md` 追加一条「修订 / 复核」记录；不碰主 `openspec/specs/` / `p6-playable-client/` / `crates/` / `.codebuddy/agents/`、不 commit、不归档、不执行远端命令。

## Goals / Non-Goals

**Goals:**
- 让 10 类 `GameplayCommandKind` 在联机线格式上与 `CommandKind` 平行承载，打通上行广播 → 帧装配 → 权威帧批 → 依序提交 → 一次 `step()` 全链路。
- 按 ADR 0005 §3 落实 schema 变更同步 bump `PROTOCOL_VERSION`（2→3）与「同步发布、不承诺旧客户端混跑」。
- 联机与本地玩法命令行为一致，去掉 `GameplayUnsupportedOnline` 短路，同时保持「显式失败不静默降级」。
- 全程 `w3-core` 零改动、不新增权威状态字段、不 bump `SNAPSHOT_VERSION`（当前 3）、不改状态哈希字段集合。

**Non-Goals:**
- 不改 `w3-core` 的 `CommandKind` / `GameplayCommandKind`、帧序、稳定排序键、状态哈希编码、快照字段、随机流或 `logic_fps = 30`。
- 不修改 `p6-playable-client/` 下任何 spec 的实质（(d) 已拍板：P6 的「`PROTOCOL_VERSION` 不变」Scenario 保持原样、不追溯改写；待 P6 归档后由主 `openspec/specs/lockstep-networking/spec.md` 修订措辞，见 D6 / D7）。
- 不修改 `.codebuddy/agents/`（不在 `spec-steward` 可写范围）；owner 归属已由 (e) 拍板定案，见 D5。
- 本轮按 (g) 已在 ADR 0005 **只追加**一条「修订 / 复核」记录（不改原决策正文、不改既有段落，D4）；不写 `NOTICE.md` / `CODEBUDDY.md` / `config.yaml` / 主 `openspec/specs/`。
- 不引入 `float` / `double` / `map` / gRPC / 新序列化框架；不承诺旧客户端长期混跑；不引入跨版本兼容垫片。
- 本轮不写 `crates/`、不碰主 `openspec/specs/` / `p6-playable-client/`、不运行 `cargo`、不 commit、不归档、不执行任何 git 远端命令；「P6 先归档 + 之后修订主 spec」仅作为已拍板的顺序策略固化待执行，须在人工 push + CI 双平台通过后按 §9 串联执行（见 D6 / D7 / Migration Plan）。
- 不声称 Linux / macOS 跨平台确定性已达成（ADR 0002：目前仅 Windows / rustc 1.91 实跑）；不新增需要跨平台验证的算术来源。

## Decisions

> 共 8 条。D1–D7 为编排者列出的 7 个核心设计问题，D8 为「spec delta 落点」这一必须明确给出的补充项。每条含「决策 / 备选方案（Alternatives considered）/ 关联 ADR 0002 与跨平台确定性影响 / 架构原则与分层依赖影响」四部分。**8 项已全部由用户拍板、均采纳推荐方案，本轮全部定案**；(d) 的执行载体选定 A(ii)，理由见 D7。

### D1：schema 扩展方式与 tag 段规划（**已定案 (a)**）

- **决策（已定案，(a) 用户拍板采纳 D1 建议 + 源码校正）**：采用「**新增独立 `GameplayCommand` 消息 + `GameplayCommandBatch` + 在顶层 `CommandBatch` 追加 tag 9**」，**不动** `Command` 的 `oneof`（tag 4–10）与 `reserved 11 to 16`。
  - 新增 `GameplayCommand`：`execute_frame=1 / player_id=2 / sequence=3`，`oneof kind` 用 **tag 4–13** 放 10 个玩法变体（`Cast / Train / CancelTrain / Research / BuyItem / Gather / RegisterBehavior / SpawnWave / SetAi / Leave`），`reserved 14 to 32`。因为是**全新消息**、tag 空间独立，`Command.reserved 11 to 16` 只有 6 位而玩法有 10 变体的矛盾**根本不适用**——这是对「tag 段如何规划」最干净的回答，且完全满足 ADR 0005 §3「字段编号只追加、不复用、不改名；删除保留 `reserved`」。
  - 新增 `GameplayCommandBatch { repeated GameplayCommand commands = 1; reserved 2 to 8; }`。
  - **关键源码校正（已由 (a) 采纳）**：`encode_commands` / `decode_commands` 的顶层消息是 `CommandBatch`，`FrameCommands.commands` 也是 `CommandBatch`。因此追加字段应落在 `CommandBatch`：`GameplayCommandBatch gameplay = 9;`（tag 9，位于 `reserved 2 to 8` **之后**，只追加，不改该 `reserved`）。这样 `FrameCommands` / `ResyncReplay` **无需新增字段**即自动携带玩法批，Input / Frame / ResyncReplay 三条路径一次覆盖。在 `FrameCommands` 里追加 `repeated` 字段的设想会漏掉 Input / Frame 主路径（它们不经过 `FrameCommands`），需两处都改且语义割裂，故**否决**。
  - `to_pb_kind` / `from_pb_kind`（对 `CommandKind`）的穷尽 match **完全不动**，只新增 `to_pb_gameplay_*` / `from_pb_gameplay_*` 映射；`EffectKind` 用 `effect_kind_code` 的稳定码 `uint32` 并对未知码显式拒绝；`EntityId`→`uint64`、`DefId`→`uint32`、`SimFixed`→`raw()` `int64`、其余 `int64/uint32/bool`。
- **备选方案（Alternatives considered）**：
  - *A1：删 / 扩 `Command.reserved 11 to 16`，用 11–20 放 10 个 gameplay 变体，另设 `reserved 21 to 32`* — **否决**。占用 / 改写既有 `reserved` 声明属「复用保留位」；更致命的是 `pb::Command` 解码目标是 `w3_core::Command { kind: CommandKind }`（7 变体），gameplay 变体无法映射回 `CommandKind`，除非把 `Command` 改成 enum wrapper（改下游契约、违反 ISP/SRP，且 `decode_commands` 返回类型 BREAKING）。
  - *A2：保留 `reserved 11 to 16`，在 `Command.kind` oneof 追加 17–26* — **否决**。仍把两类不同 Rust 类型塞进同一 `oneof`，类型语义污染与穷尽 match 破坏同 A1。
  - *B′：新增独立消息但在 `FrameCommands` 追加 `repeated GameplayCommand`* — **未采纳**。漏 Input / Frame 主路径，需 `CommandBatch` 与 `FrameCommands` 两处扩展，语义重复（CARP 违反）。
  - *B：新增独立消息 + 追加到 `CommandBatch`（顶层 tag 9）* — **已定案采纳 (a)**。改动面最小、`Command` 零触碰、10 变体 tag 段自洽、三条路径一次覆盖。
  - *新增 `PacketKind::GameplayInput` / `GameplayFrame` 独立包类型* — **否决**。帧编排出现两套就绪队列与两个推进来源，易触发双 `step()`，与 D2 及 ADR 0002 §2 冲突；`PacketKind` 还需另行规划 tag。
  - *把玩法命令编码进 `Noop` 扩展字段 / `bytes` 透传* — **否决**。违反 ADR 0005 §3 的 schema-first / canonical / 禁 unknown fields 约束。
- **关联 ADR 0002 与跨平台确定性影响**：字段全定宽整数 / 布尔、禁用 `float` / `double` / `map`、`SimFixed` 用 `raw()`（ADR 0004），生成代码在 Windows / Linux 双平台字节一致；不新增权威状态、不改状态哈希与 `SNAPSHOT_VERSION`（当前 3）。跨平台确定性仍仅在 Windows / rustc 1.91 实跑，本决策不声称已达成，也不新增算术来源。
- **架构原则与分层依赖影响**：SRP（`Command` 与 `GameplayCommand` 各管一类）、OCP（新增消息扩展而非改既有穷尽 match）、ISP（`w3-net` 只见 `w3-core` 的公开类型）、CARP（复用既有 `CommandBatch` 嵌套结构）。`w3-core` 零改动、不反向依赖；唯一状态所有者仍是 `RtsMatch`。

### D2：命令流组织——同一帧批、统一稳定键、一次 `step()`（**已定案 (b)**）

- **决策（已定案，(b) 用户拍板采纳 D2；否决独立批 / 双 `step()`）**：玩法命令与普通命令 SHALL 共用**同一个** `FrameCommands` 帧批、**同一** `stabilize` 稳定排序键语义、**同一个** `apply_frame` 帧。`Command` 与 `GameplayCommand` 是两条独立队列、各自独立 `arrival`，统一键的作用是**规范化归并顺序**而非跨类型共享 `arrival`。
  - Rust 侧 `FrameCommands` 扩展为含 `commands: Vec<Command>` + `gameplay: Vec<GameplayCommand>`（BREAKING，见 proposal Impact）。引入统一枚举 `FrameInput`（或等价归并视图），按 `(execute_frame, player_id, sequence, type_tag)` 稳定排序，`type_tag` 为**固定常量**（`Simulation=0 / Gameplay=1`），仅作并列键的确定性 tiebreaker，使归并序列可复现、可 canonical 编码。
  - `apply_frame` 在**恰好一次** `sim.step()` 之前，按归并序依序提交：普通命令 → `submit_command`，玩法命令 → `submit_gameplay_command`（内部走 `validate_gameplay_command`，与玩家 / AI 同源）；随后仅一次无参 `step()`，返回 `compute_state_hash()`。
  - **源码核对的关键确定性依据**：`Command` 走 `CommandQueue`、`GameplayCommand` 走 `GameplayCommandQueue`，两条队列 `next_arrival` **独立**；`step()` 一次调用内阶段 1 与阶段 7 分别冻结两类命令。因此 `arrival` 按各自队列的提交顺序独立分配，**跨类型交错顺序不影响 `arrival`、也不影响权威状态**；真正的硬要求是「每一类型的提交子序列在两端完全相同」。`type_tag` 正是把归并序规范化、避免实现依赖不确定遍历（如 `HashMap`）的手段，不改变 ADR 0002 §3 各类队列内部 `ExecuteFrame → PlayerId → Sequence → Arrival` 的键语义。
  - `arrival` 仍**不编码**（沿用 `encode_commands` 的现有约定）；`submit_input` 同时上行两类命令；`FrameAssembler` 同时收集两类；`ResyncReplay` 同时补发两类。
  - 联机唯一模拟推进点仍是 `w3_net::apply_frame`（P6 `net_driver.rs` 已确立，其内部恰好一次 `step()`），本 change **不新增第二个推进点**。
- **备选方案（Alternatives considered）**：
  - *两类命令各自独立帧批 + 两次 `apply_frame` / 双 `step()`* — **否决**。直接违反 ADR 0002 §2「`step()` 无参、每帧恰好一次」，双推进必 desync。
  - *同一帧批但按到达顺序提交* — **否决**。违反 ADR 0002 §3「到达顺序不决定执行顺序」。
  - *在 `w3-core` 新增统一命令队列 / 统一命令类型* — **否决**。`w3-core` 零改动硬红线；且会改变权威状态、需扩展状态哈希与快照字段集合。
  - *玩法命令作为 `CommandKind` 新变体经 `submit_command` 提交* — **否决**。不改 `CommandKind`；且玩法命令的校验入口是 `validate_gameplay_command`（含 `validate_gameplay_entities`），与普通命令不同源。
  - *`type_tag` 用枚举哈希 / 运行时值* — **否决**。引入平台相关不确定性，影响 canonical 字节一致性。
  - *同一帧批 + 固定 `type_tag` 统一稳定键 + 一次 `step()`* — **已定案采纳 (b)**。
- **关联 ADR 0002 与跨平台确定性影响**：整数帧 30 Hz、稳定 `EntityId` 顺序、FNV-1a 状态哈希、`arrival` 一致性、状态哈希与 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` / `content_hash`）全部不变；`step()` 次数不变、帧序不变。跨平台确定性仍为待验证项，本决策不新增浮点或超越函数。
- **架构原则与分层依赖影响**：SRP（帧边界单一推进点）、LoD（网络层只与 `w3-core` 的两个公开提交入口交互）、DIP（依赖 `RtsMatch` 抽象而非内部队列）、唯一状态所有者硬红线、`w3-core` 不反向依赖上层。「显式失败不静默降级」：提交失败 MUST 以英文结构化日志明确返回 `NetError`。

### D3：`PROTOCOL_VERSION` 2→3 与兼容策略（**已定案 (c)**）

- **决策（已定案，(c) 用户拍板采纳 D3）**：`PROTOCOL_VERSION` **2→3**（`packet.rs`），`Packet::decode` 对 `version != 3` 明确返回 `VersionMismatch`、relay 侧以 `Reject` 控制包明确拒绝；**不**保留「能解码 v2 但不混跑」的双解码 / 协商垫片；客户端与 relay 同步发布，**无兼容垫片**，前后兼容只用于平滑升级与工具链（ADR 0005 §3）。同步更新 `packet.rs` 的版本注释、`frame_header_unchanged_for_protobuf_payload` 测试中 `bytes[4] == 2` 的断言，以及 `w3-relay/tests/*` 对 `PROTOCOL_VERSION` 的 import。
  - **最强理由（源码推导，已由用户核实采纳）**：protobuf 默认丢弃 unknown fields（`payload.rs` 既有 `unknown_fields_are_dropped_not_echoed` 测试为证）。若 v2 端与 v3 端静默混跑，v2 端会把 `CommandBatch` 里的玩法命令字段当作未知字段**静默忽略**，两端命令流不一致 → 逐帧 `compute_state_hash()` 不等 → desync，且失败不可见。故 version 硬拒绝是确定性硬门禁，不是可选的兼容偏好，也直接呼应 CODEBUDDY.md「显式失败不静默降级」硬红线与 `protobuf-payload` design 决策四对「backwards-compatibility shim」的否决。
- **备选方案（Alternatives considered）**：
  - *bump + 明确拒绝 v2（无兼容垫片）* — **已定案采纳 (c)**。符合 ADR 0005 §3、主 `lockstep-networking` spec「版本不匹配」Scenario、CODEBUDDY.md 硬红线。
  - *保留 v2 可解码但标记 gameplay 不支持、允许短时混跑* — **否决**。这正是要修的「静默降级」边界；且属 ADR 0005 §3 明确不承诺的「旧客户端长期混跑」，并引入 unknown fields 丢弃风险。
  - *运行期版本协商 / 双实现并存* — **否决**。过度工程、违反「不加兼容垫片」（`protobuf-payload` D4 已否决同类方案），且协商结果会进入影响帧序的路径。
  - *不 bump、纯靠 protobuf 前向兼容* — **否决**。直接违反 ADR 0005 §3「proto schema 变更必须同步 bump `PROTOCOL_VERSION`」硬约束，且造成上述静默 desync。
- **关联 ADR 0002 与跨平台确定性影响**：版本边界显式、失败可见，不含隐式降级；25B `W3N1` 帧头布局、显式小端、FNV-1a 覆盖范围、`payload_len` 语义均不变（ADR 0005 §1）；状态哈希与 `SNAPSHOT_VERSION`（当前 3）不变。跨平台确定性仍为待验证项，版本字节是显式 `u8`，无平台歧义。
- **架构原则与分层依赖影响**：SRP（版本判定集中在 `Packet::decode`）、显式失败不静默降级硬红线、单一权威。`w3-core` 不感知协议版本、不反向依赖。

### D4：ADR 0005 复核修订记录（**已定案 (g)，本轮已执行**）

- **决策（已定案，(g) 用户拍板采纳 D4；本轮已执行）**：ADR 0005 末尾「复核点」明确「proto schema 变更或 `prost` 升级触发复核」，本 change 正是 proto schema 变更，已定案在 `docs/decisions/0005-protobuf-payload.md` **只追加**一条「修订 / 复核」记录（新增 `## 修订记录` 段，沿用文件头部 `- **修订**：` 行体例；内容：proto schema 变更触发 §「复核点」、字段编号段扩展〔独立 `GameplayCommand` / `GameplayCommandBatch` + 顶层 `CommandBatch` tag 9〕、`PROTOCOL_VERSION` 2→3、明确拒绝旧版本、canonical 约束继续适用、实施状态以本 change 的 tasks 为准），**不改原决策正文**（§1–§4、备选方案表、后果、复核点原文均不动）、**不改既有段落**（含文件头部既有 `- **修订**：` 行）。本轮已由 `spec-steward` 在可写范围 `docs/decisions/` 内追加完成。
- **备选方案（Alternatives considered）**：
  - *只追加「修订 / 复核」段* — **已定案采纳 (g)**。保留 ADR「记为什么决定」的定位与历史决策原文，满足复核可追溯性。
  - *不改 ADR，仅在 change design 里说明* — **否决**。ADR 0005 复核点要求 schema 变更触发复核，且 spec ↔ ADR 引用关系需可审计。
  - *新开 ADR 0006* — **否决**。决策未变（仍是自有帧头 + protobuf payload），只是复核记录；不应膨胀 ADR 编号。
  - *直接把 ADR 正文的 `PROTOCOL_VERSION` 改写成 3 / 改 §3 约束文字* — **否决**。破坏历史决策记录；ADR 记「为什么」，当前版本值属 change / 代码事实，不回写 ADR 正文。
- **关联 ADR 0002 与跨平台确定性影响**：纯文档一致性操作，不改代码、帧序、哈希、快照或任何可比较量；不影响跨平台确定性的待验证状态。
- **架构原则与分层依赖影响**：单一权威（ADR 记「为什么决定」、OpenSpec 记「系统该做什么」、spec 引用 ADR 编号，CODEBUDDY.md §8）；不追溯改写已归档 / Accepted 决策（与 `protobuf-payload` 对 P4 的处理一致）。

### D5：owner 归属（**已定案 (e)**）

- **决策（已定案，(e) 用户拍板新建 `net-owner`）**：owner **已指定 `net-owner`**（单一 owner，覆盖 `crates/w3-net/` / `crates/w3-relay/`），`w3-client` 侧由 `client-owner` 配合，配只读 `critic`；中文 agent 定义由编排者新建（不在 `spec-steward` 可写范围，本轮不写 `.codebuddy/agents/`）。补充源码核对结论：本 change 必改 `crates/w3-net/` **与 `crates/w3-relay/`**（见 Context），而 CODEBUDDY.md §9 的 owner 表中 `client-owner` 只写 `crates/w3-client/` / `crates/w3-render/`，`sim-owner` 只写 `crates/w3-core/`，**没有任何现有 agent 拥有 `w3-net` 或 `w3-relay`**。故由编排者新建 `net-owner` 落实 agent 定义，且必须满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 配一个只读 `critic`，并与 `client-owner` 在 `w3-client/src/net_driver.rs` 明确切分边界（避免两个 owner 同时改同一文件）。
- **备选方案（Alternatives considered）**：
  - *扩展 `client-owner` 可写范围到 `crates/w3-net/`（并覆盖 `crates/w3-relay/`）* — **未采纳**。沿用 P6 已建立的联机适配经验；但 `client-owner` 名义是「P6 可玩客户端集成层」，扩到线格式 / relay crate 语义偏宽，需同步改其职责描述。
  - *新建 `net-owner`（可写 `crates/w3-net/` + `crates/w3-relay/`，必要时含 `w3-client` 的 net 适配）* — **已定案采纳 (e)**。SRP 最贴合（线格式 / 传输 / relay 是独立关注点）；代价是需编排者新建 agent 定义，且与 `client-owner` 在 `w3-client/src/net_driver.rs` 的边界需明确切分（避免两个 owner 同时改同一文件）。
  - *用户另行指定（含单 change 临时授权）* — **未采纳**。
  - *拆成两个 change（线格式一个 owner、客户端一个 owner）* — **否决**。会把一个帧批内两类命令的实现切成两期，制造中间态 desync 风险，且违反「一个 change 一个 owner」的单 change 口径。
- **关联 ADR 0002 与跨平台确定性影响**：owner 划分不改变任何可比较量，但决定「谁守确定性门禁」的责任闭环；按 (e) 定案后 `critic` 仍只读复审确定性 / 所有权 / 许可证 / 编码 / 资产，`cargo fmt --all -- --check` 与 `tests/determinism.rs` 门禁不变。
- **架构原则与分层依赖影响**：单一权威、一个 change 一个 owner、critic 只读（CODEBUDDY.md §8/§9）；`w3-core` 仍由 `sim-owner` 独占且本 change 零改动，不产生跨 owner 写冲突。

### D6：归档顺序——「P6 先、本 change 后」（**已定案 (d)**）

- **决策（已定案，(d) 用户拍板采纳 D6）**：**先归档 `p6-playable-client`，再归档本 change**；本 change 的 `/opsx:apply` 应在 P6 归档后进行。理由：
  1. P6 已实施完成（54/55），仅剩归档；先归档让主 `openspec/specs/lockstep-networking/spec.md` 吸收 P6 delta、成为稳定 source of truth。
  2. 本 change 是 P6「已裁定·已知边界」的**后续补齐**，逻辑上依赖 P6 的 `GameplayUnsupportedOnline` 已存在；时间线自洽。
  3. 若本 change 先归档，P6 后归档时其 delta 中「`PROTOCOL_VERSION` 不变」Scenario 会**回写**主 `lockstep-networking` spec 到与 `PROTOCOL_VERSION=3` 矛盾的状态（即 D7 冲突后移且更难收口）。先 P6 后本 change，可让冲突在 P6 归档时暴露、再由归档后的主 spec 一致性维护收口。
  4. 本 change 刻意不产生 `lockstep-networking` delta（D8），两者操作不同 capability，故归档顺序**不会**造成 OpenSpec delta 合并冲突，只剩文档口径冲突。
  - **本轮边界（不立即归档）**：本轮**不执行归档、不改主 `openspec/specs/`**——归档是 CODEBUDDY.md §9 串联的末端动作（P6 刚 commit、未 push / CI，须先经人工 push + CI Windows / Linux 双平台 `verify_all.py` 通过）。「P6 先归档 + 之后修订主 spec」仅作为已拍板的**顺序策略**固化进本 design 与 `tasks.md` 2.2 / 2.3 待执行，本轮不碰 P6 change spec、不碰主 specs。
- **备选方案（Alternatives considered）**：
  - *本 change 先归档、P6 后归档* — **否决**。P6 会把「不变」写回主 spec 造成自相矛盾，且让已 complete 的 P6 为新提案让位不合理。
  - *两者同时归档* — **否决**。OpenSpec 逐个归档、无法原子，顺序仍需裁定。
  - *长期不归档 P6* — **否决**。主 specs 是 source of truth（CODEBUDDY.md §8），P6 已实施完成却不归档会让主 spec 缺失 `playable-client` 能力。
  - *P6 先、本 change 后* — **已定案采纳 (d)**（并与 P6 的 D8「先归档 P5 再 apply P6」同类决策一致）。
- **关联 ADR 0002 与跨平台确定性影响**：纯规范文档操作，不改代码、帧序、哈希、快照或可比较量；但顺序影响主 spec 口径与 `PROTOCOL_VERSION=3` 事实的一致性（ADR 0002 的显式版本边界 / ADR 0005 §3）。
- **架构原则与分层依赖影响**：单一权威（主 specs source of truth）、一个 change 一个 owner；归档由编排者按 §9 串联随后执行（人工 push + CI 双平台通过后），本轮不归档、不改主 `openspec/specs/`、不预断结果、不触碰 `crates/`。

### D7：与 P6「`PROTOCOL_VERSION` 不变」Scenario 的冲突（**已定案 (d)，执行载体选定 A(ii)**）

- **决策（已定案，(d) 用户拍板采纳选项 A；执行载体选定 A(ii)）**：本轮**不修改** `openspec/changes/p6-playable-client/` 下任何 spec 文件（用户明确要求「保持原样、不追溯改写」）。冲突事实登记：P6 的 `specs/lockstep-networking/spec.md` 在 `### Requirement: Network timing MUST NOT drive the fixed timestep` 下有 `#### Scenario: 线格式不变`，THEN 写「…`PROTOCOL_VERSION` 不变」；本 change 要 bump 到 3，**直接冲突**。P6 另有 `## MODIFIED Requirements` 的 `Unchanged simulation determinism boundary`。主 `openspec/specs/lockstep-networking/spec.md` 现有 requirements 本身并不写死版本值（只要求「schema 变更同步 bump」与「版本不匹配 `Reject`」，本轮复核确认），因此冲突**仅来自 P6 未归档的 delta**，是文档口径冲突、不是 `openspec validate` 的结构错误。**选项 A/B/C 与三个执行子选项的裁定如下**：
  - **选项 A（已定案采纳）**：P6 归档后，在**主** `openspec/specs/lockstep-networking/spec.md` 把该 Scenario 从「`PROTOCOL_VERSION` 不变」修订为「`PROTOCOL_VERSION` 按 ADR 0005 §3 随 schema 变更同步递增；本能力只约束 25B `W3N1` 帧头布局与 canonical 编码约束不变」。P6 的 change spec 保持原样（已实施完成、不追溯改写），冲突在 source of truth 层收口。
  - **执行载体：选定 A(ii)（归档后由 `spec-steward` 作为一次性一致性维护直接修订主 spec，不走 change delta）**。选它的理由：
    - (h) D8 已定案「新增独立 `gameplay-command-wire` capability、**不产生 `lockstep-networking` delta**」；执行载体必须与「不产生 delta」不互斥。
    - *A(i)（本 change 追加一个最小 `lockstep-networking` MODIFIED delta）* 与 *选项 B* 都**会产生 `lockstep-networking` delta**，与 (h) **互斥**，且需推翻 / 重拍 D8、把 P6「不变」Scenario 提前卷入本 change，故**否决**。
    - *A(iii)（另开一个极小 change 承载该措辞修订）* 仍将产生 `lockstep-networking` delta，且引入新 change / 新阶段 / 新 owner，超出 (f)「不新增阶段」的克制口径，故**否决**。
    - *选项 C* 需改 P6 已实施完成 change 的 spec 实质，用户明确「保持原样、不追溯改写」，故**否决**。
    - **A(ii) 不产生任何 `lockstep-networking` delta**（归档后直接在主 `openspec/specs/lockstep-networking/spec.md` 做一次性措辞维护），既满足选项 A「P6 归档后由主 spec 修订措辞、P6 change spec 保持原样」，又与 (h) D8 完全不互斥，是唯一同时满足 (d) 与 (h) 的执行载体。
  - **选项 B — 否决**：本 change 产出 `lockstep-networking` 的 MODIFIED delta 显式覆盖该 Scenario（等价于推翻 D8，与 (h) 互斥）。
  - **选项 C — 否决**：在 P6 的 change spec 该 Scenario 下加注「注：随后续 `gameplay-command-wire-format` change 演进，`PROTOCOL_VERSION` 按 ADR 0005 §3 bump」；属改 P6 spec 实质，本轮一律不动。
  - **注意（(d) 与 (h) 已一并拍板）**：D8 与 A(i) / B 的互斥关系已随本次一并拍板消解——选定 A(ii) 后本 change 仍「不产生 `lockstep-networking` delta」，措辞修订作为归档后的一次性一致性维护落在主 spec，不需要任何 change delta，也不重拍 D8。
  - **本轮边界**：归档是 CODEBUDDY.md §9 串联的末端动作（P6 刚 commit、未 push / CI），本轮**不执行归档、不改主 `openspec/specs/`、不改 P6 change spec**；顺序策略与 A(ii) 仅固化在 design 与 `tasks.md` 2.2 / 2.3 待执行，须在人工 push + CI 双平台通过后执行。
- **备选方案（Alternatives considered）**：见上选项 A（已定案采纳 + 执行载体 A(ii)）/ A(i) / A(ii) / A(iii) / B / C；*忽略冲突、依赖归档时人工记忆* — **否决**，会让主 spec 长期自相矛盾，违反「单一权威」。
- **关联 ADR 0002 与跨平台确定性影响**：纯文档一致性，不改代码、帧序、哈希、快照；但主 spec 若保留「版本不变」会误导后续 change 违反 ADR 0005 §3 的版本边界，间接危及确定性。
- **架构原则与分层依赖影响**：单一权威（主 specs source of truth）、ADR ↔ spec 引用一致（CODEBUDDY.md §8）、不追溯改写已实施完成 change 的 spec 实质；A(ii) 的一次性一致性维护不走 change delta，避免与 P6 active delta 在同一 capability 冲突。

### D8：spec delta 落点——新增独立 capability（补充「明确给出」项）（**已定案 (h)**）

- **决策（已定案，(h) 用户拍板采纳 D8）**：本 change 的 spec delta **落在新增独立 capability `gameplay-command-wire`**（`specs/gameplay-command-wire/spec.md`，全部 `## ADDED Requirements`），**不产生 `lockstep-networking` delta**，`### Modified Capabilities` 留空。理由：
  1. 避免与 P6 尚未归档的 `lockstep-networking` delta（含 ADDED「线格式不变」+ MODIFIED「Unchanged simulation determinism boundary」）在同一 capability 上冲突。
  2. 主 `lockstep-networking` spec 现有 requirements 已覆盖「版本不匹配 `Reject`」「schema 变更同步 bump `PROTOCOL_VERSION`」「canonical 编码约束」「只传输入命令」「模拟确定性边界不变」——本 change 是**行使**这些要求而非修改其行为；新增玩法命令的契约是自包含的新关注点（SRP）。
  3. MODIFIED delta 需整块复制并改写既有 requirement（`specs` artifact 规则），会把 P6 的「不变」Scenario 提前卷入本 change，放大 D7 冲突。
  - **对主 specs 的影响（回报要点）**：本 change 归档时，主 `openspec/specs/` 将**新增** `gameplay-command-wire/spec.md` 一个 capability；既有 7 个主 spec **不改动**。按 (d) 已定案的选项 A + 执行载体 A(ii)，`lockstep-networking` 主 spec 的「`PROTOCOL_VERSION` 不变」措辞在 P6 归档后由 `spec-steward` 一次性一致性维护单独修订（不走 change delta、不产生 `lockstep-networking` delta，与本 change 归档无关），本轮不执行、不改主 specs。
- **备选方案（Alternatives considered）**：
  - *MODIFIED `lockstep-networking`* — **否决**。与 P6 active delta 直接冲突；MODIFIED 需整块复制既有 requirement；会把 D7 冲突提前。
  - *新增 capability + 同时 MODIFIED `lockstep-networking`* — **否决**。双重冲突面，违反「不擅自改 P6 spec 实质」的克制要求。
  - *塞进已归档的 `gameplay-systems`* — **否决**。那是 `w3-core` 玩法规则能力，线格式不归它；动它会重写已归档主 spec，且 `w3-core` 本 change 零改动。
  - *新增独立 `gameplay-command-wire`* — **已定案采纳 (h)**。
- **关联 ADR 0002 与跨平台确定性影响**：delta 落点纯属规范结构选择，不改代码 / 可比较量；新 capability 内每条 requirement 均声明 `w3-core` 零改动、状态哈希与 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` / `content_hash`）不变。
- **架构原则与分层依赖影响**：SRP（线格式契约独立成 capability）、单一权威（主 specs source of truth）、ADR ↔ spec 引用一致（新 spec 抬头引用 ADR 0001–0005，CODEBUDDY.md §8）。

## Risks / Trade-offs

- [10 个玩法变体 vs `Command.reserved 11 to 16` 仅 6 位] → D1 用独立 `GameplayCommand` 消息新开 tag 4–13 + `reserved 14 to 32`，既有保留位零触碰；tag 段规划已由 (a) 拍板定案。
- [在 `Command.kind` oneof 混装两类命令导致 `CommandKind` 穷尽 match 与返回类型破坏] → D1 否决 A1/A2，只新增平行消息与平行映射函数；`CommandKind` 零改动。
- [顶层编解码消息是 `CommandBatch` 而非 `FrameCommands`，追加字段易落错位置] → D1 源码校正已由 (a) 采纳：追加到顶层 `CommandBatch` tag 9，Input / Frame / ResyncReplay 一次覆盖。
- [v2 / v3 客户端静默混跑导致玩法命令被 unknown fields 丢弃而 desync，且失败不可见] → (c) 已拍板 D3：硬 bump + `VersionMismatch` / `Reject` 明确拒绝，同步发布，不加兼容垫片。
- [新增玩法命令引入第二次 `step()` / 双推进] → (b) 已拍板 D2：同一 `apply_frame` 帧内归并排序、一次 `step()` 前全部提交；联机唯一推进点不变；`tests/determinism.rs` 与双端逐帧哈希作门禁。
- [两类命令独立 `arrival` 计数器，归并顺序不确定会改变 pending 命令哈希] → D2 固定 `type_tag` 统一稳定键 + 稳定排序，保证每类提交子序列两端一致；`arrival` 仍不编码、非跨类型共享。
- [`FrameCommands` / `apply_frame` / `submit_input` 公开形状 BREAKING，波及 `w3-relay` 与 P6 `net_driver.rs` 及 `w3-relay/tests/*`] → tasks 逐项迁移；两处 BREAKING 已由用户拍板确认接受；`cargo test --workspace` 全绿把关。
- [`w3-relay` 是必改点却无任何现有 owner 覆盖；`w3-net` 亦无 owner] → (e) 已拍板 D5：指定 `net-owner`（覆盖 `w3-net` / `w3-relay`），`w3-client` 侧由 `client-owner` 配合、配只读 `critic`；中文 agent 定义由编排者新建，本轮不改 `.codebuddy/agents/`。
- [P6 未归档的「`PROTOCOL_VERSION` 不变」Scenario 与本 change 冲突] → (d) 已拍板：D6「P6 先归档、本 change 后归档」+ D7 选项 A、执行载体 A(ii)（归档后由主 spec 修订措辞，P6 change spec 保持原样）；本轮不改 P6 spec 实质、不改主 specs、不提前归档。
- [本轮误把「P6 先归档 + 修订主 spec」提前执行、越界改主 `openspec/specs/`] → (d) 明确本轮不执行归档、不改主 specs：归档是 §9 串联末端动作（P6 刚 commit、未 push / CI），仅把顺序策略固化进 design 与 `tasks.md` 2.2 / 2.3 待执行，须在人工 push + CI 双平台通过后执行。
- [D8「不产生 `lockstep-networking` delta」与 D7 选项 A(i)/B 互斥] → (d) 与 (h) 已一并拍板：选 A(ii) 一次性一致性维护（不走 change delta、不产生 delta），两决策不互斥，D8 不重拍。
- [ADR 0005 复核点未闭环] → (g) 已拍板 D4：本轮在 ADR 0005 只追加一条「修订 / 复核」记录（不改原决策正文、不改既有段落），复核点已闭环。
- [路线图阶段标注不明（P6 后续 / 独立 P7 / 不新增阶段）] → (f) 已拍板：不新增阶段、标注为「P6 集成收尾的后续补齐」，`config.yaml` rules.proposal 的「P0-P5」枚举保持不动。
- [新增依赖 / 许可证 / 资产入库] → 本 change 预期复用既有 `prost`，不引入新依赖；若确需新增，按 ADR 0001 与 CODEBUDDY.md §6 检查 GPLv3 兼容性、`Cargo.lock`、`NOTICE.md`（交 `docs-owner`）；CI 资产扫描把关。
- [中文注释 / 英文日志 / UTF-8 无 BOM / LF / `git diff --check`] → tasks 列出 `cargo fmt --all` + `--check`、`python tools/check_encoding.py`、`git diff --check`；失败文本与日志一律英文结构化 `key=value`。
- [跨平台确定性被误宣称达成] → 明确本 change 不新增浮点 / 超越函数 / 可比较算术来源，Linux / macOS 仍属待验证项（ADR 0002）。
- [真实 UDP 双进程 / CI 双平台生成一致性] → tasks 列人工实跑与 `verify_all.py` 双平台门禁。

## Migration Plan

1. **规范侧（本轮，已完成）**：按 8 项用户拍板结论定案 proposal / specs / design / tasks 四件套；在 ADR 0005 只追加一条「修订 / 复核」记录；`openspec validate gameplay-command-wire-format` 与 `--strict` 通过；`python tools/check_encoding.py` 与 `git diff --check` 通过。不碰 `crates/`、主 `openspec/specs/` / `p6-playable-client/`。
2. **用户拍板（已完成，8 项均采纳推荐，阻塞解除）**：(a) 独立 `GameplayCommand` / `GameplayCommandBatch` + 顶层 `CommandBatch` tag 9（采纳 D1 源码校正）；(b) 同一帧批 + 固定 `type_tag` + 一次 `step()` 前全部提交，否决独立批 / 双 `step()`；(c) `PROTOCOL_VERSION` 2→3 硬 bump + 明确拒绝 v2 + 同步发布、无兼容垫片；(d) P6 先归档、本 change 后归档，P6 change spec 保持原样，归档后由主 spec 修订措辞（选项 A / 执行载体 A(ii)）；(e) 新建 `net-owner`（`w3-net` / `w3-relay`），`client-owner` 配合、只读 `critic`；(f) 不新增阶段、标为「P6 集成收尾的后续补齐」；(g) ADR 0005 只追加一条修订 / 复核记录（本轮已执行）；(h) 新增独立 `gameplay-command-wire` capability、不产生 `lockstep-networking` delta。
3. **落实 owner 定义（编排者，本轮不执行）**：按 (e) 已拍板结论新建中文 `net-owner` agent 定义（可写 `crates/w3-net/` / `crates/w3-relay/`），`w3-client` 侧由 `client-owner` 配合，配只读 `critic`；确认「一个 change 一个 owner」（不在 `spec-steward` 可写范围，`tasks.md` 2.1）。
4. **归档 P6（编排者，本轮不执行；按 §9 串联须在人工 push + CI Windows / Linux 双平台通过后执行，P6 刚 commit、未 push / CI，不提前归档）**：`openspec archive p6-playable-client -y`，确认主 `openspec/specs/` 已吸收 P6 的 `playable-client` / `3d-rendering` / `lockstep-networking` delta；P6 的「`PROTOCOL_VERSION` 不变」Scenario 保持原样、不追溯改写（(d) 选项 A，`tasks.md` 2.2）。
5. **A(ii) 一次性一致性维护（编排者 / `spec-steward`，本轮不执行，P6 归档后执行）**：在**主** `openspec/specs/lockstep-networking/spec.md` 把「线格式不变」Scenario 从「`PROTOCOL_VERSION` 不变」修订为「`PROTOCOL_VERSION` 按 ADR 0005 §3 随 schema 变更同步递增；本能力只约束 25B `W3N1` 帧头布局与 canonical 编码约束不变」，**不走 change delta、不产生 `lockstep-networking` delta**（与 (h) 不互斥），不追溯改写 P6 change spec（`tasks.md` 2.3）。
6. **`/opsx:apply`（由已指定 `net-owner` 执行）**：`w3net.proto` 扩展 → `PROTOCOL_VERSION` bump → `payload.rs` 双向编解码 → `frame.rs` / `session.rs` / `w3-relay` 命令流 → `w3-client` 联机通路去掉 `GameplayUnsupportedOnline` → 测试与门禁。顺序见 `tasks.md`。
7. **验收门禁**：`cargo test -p w3-core --test determinism`（重跑 `crates/w3-core/tests/determinism.rs`，确认 `w3-core` 零改动且全绿）、`cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`python tools/verify_all.py`、CI Windows / Linux 双平台生成字节一致、双客户端逐帧哈希一致、丢包 / 乱序 / 断线回归、真实 UDP 双进程实跑（人工）。
8. **同步发布**：客户端与 relay 同版本发布；version=2 端点被明确 `Reject`。**回滚**：整体回退到上一发布版本组合（客户端 + relay 同步），不提供跨版本兼容层；`w3-core` 零改动、快照格式不变（不 bump `SNAPSHOT_VERSION`），回滚不影响权威状态与已有快照。
9. **归档本 change（后续，本轮不执行；P6 归档在前，且须在本 change 的 apply / commit / 人工 push / CI 双平台通过后按 §9 串联执行）**：`openspec archive gameplay-command-wire-format -y`，确认主 `openspec/specs/` 新增 `gameplay-command-wire/spec.md`，既有 7 个主 spec 不因本 change 归档改动，并核对 A(ii) 已完成 `lockstep-networking` 主 spec 口径修订。

## Open Questions

> (a)–(h) **已全部由用户拍板、均采纳推荐方案**，结论如下；仅保留真正未定的实现细节（见末尾）。本轮不擅自改 P6 spec 实质 / 主 `openspec/specs/` / `.codebuddy/agents/`；ADR 0005 已按 (g) 只追加一条「修订 / 复核」记录。

- **（a）schema 扩展方式与 tag 段规划 — 已由用户拍板（采纳 D1 建议 + 源码校正）**：独立 `GameplayCommand` 消息（oneof tag 4–13 覆盖 10 变体 + `reserved 14 to 32`）+ `GameplayCommandBatch` + 在**顶层 `CommandBatch` 追加 tag 9**（而非 `FrameCommands`），不动 `Command` 的 `oneof` tag 4–10 与 `reserved 11 to 16`。理由已核实：玩法命令变体无法映射回 `CommandKind` 穷尽 match，且 6 个保留位不够 10 个变体；顶层编解码消息是 `CommandBatch`，Input / Frame / ResyncReplay 三条路径一次覆盖。
- **（b）命令流组织 — 已由用户拍板（采纳 D2）**：同一帧批 + 固定 `type_tag`（`Simulation=0` / `Gameplay=1`）统一稳定键 + 恰好一次 `step()` 前全部提交；否决独立批 / 双 `step()`。`Command` 与 `GameplayCommand` 是两条独立队列、独立 `arrival`，统一键只用于规范化归并顺序、非跨类型共享 `arrival`。
- **（c）`PROTOCOL_VERSION` 兼容策略 — 已由用户拍板（采纳 D3）**：2→3 硬 bump + `VersionMismatch` / `Reject` 明确拒绝 v2 + 客户端与 relay 同步发布、无兼容垫片。核心理由已核实：v2 端会把玩法命令字段当 unknown field 静默丢弃 → 两端命令流不等 → desync 不可见，违反「显式失败不静默降级」。
- **（d）归档顺序与 P6「不变」Scenario — 已由用户拍板（采纳 D6 / D7 选项 A）**：**P6 先归档、本 change 后归档**；P6 的 `p6-playable-client/specs/lockstep-networking/spec.md`「`PROTOCOL_VERSION` 不变」Scenario **保持原样、不追溯改写**；待 P6 归档后由主 `openspec/specs/lockstep-networking/spec.md` 修订措辞。**执行载体已选定 A(ii)**（归档后由 `spec-steward` 作为一次性一致性维护直接修订主 spec、不走 change delta），是唯一与 (h)「不产生 `lockstep-networking` delta」不互斥的载体：A(i) / B 会产 delta（互斥）、A(iii) 需新 change / 新阶段、C 需改 P6 spec 实质，均否决（详见 D7）。本轮**不执行归档、不改主 specs**——归档是 §9 串联末端动作（P6 刚 commit、未 push / CI），仅把顺序策略固化进 design 与 `tasks.md` 2.2 / 2.3 待执行，须在人工 push + CI 双平台通过后执行。
- **（e）owner 归属 — 已由用户拍板（新建 `net-owner`）**：已指定 `net-owner`（单一 owner，覆盖 `crates/w3-net/` / `crates/w3-relay/`），`w3-client` 侧由 `client-owner` 配合，配只读 `critic`；中文 agent 定义由编排者新建（不在 `spec-steward` 可写范围，本轮不改 `.codebuddy/agents/`）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 `critic`。
- **（f）路线图阶段标注 — 已由用户拍板（采纳建议）**：不新增阶段、标注为「P6 集成收尾的后续补齐」；`config.yaml` rules.proposal 的「P0-P5」枚举保持不动（与 P6 的 D9 拍板口径一致）。独立 P7 / 归入 P4 后续网络层演进 / 不标注阶段 均未采纳。
- **（g）是否追加 ADR 0005 修订记录 — 已由用户拍板（采纳 D4，本轮已执行）**：在 `docs/decisions/0005-protobuf-payload.md` **只追加**一条「修订 / 复核」记录（新增 `## 修订记录` 段，沿用 `- **修订**：` 行体例），不改原决策正文、不改既有段落；涵盖字段编号段扩展、`PROTOCOL_VERSION` 2→3、明确拒绝旧版本、canonical 约束继续适用、实施状态以本 change 的 tasks 为准。
- **（h）spec delta 落点 — 已由用户拍板（采纳 D8）**：新增独立 `gameplay-command-wire` capability、**不产生 `lockstep-networking` delta**；`proposal.md` Capabilities 与 `specs/` 布局不变。与 (d) 选项 A 不互斥的执行载体为 A(ii)（同 (d)）。

**仍未定（实现细节，留给已指定 `net-owner`，不阻塞本轮 validate）：** `EffectKind` 稳定码的未知值拒绝方式（在 `w3-net` 侧实现，不改 `w3-core`）；`FrameCommands` 新字段命名与 `FrameInput` 枚举的具体形状；是否保留 `ClientError::GameplayUnsupportedOnline` 变体用于其它显式失败场景。（(d) 的执行载体已选定 A(ii)，不再是开放项。）
