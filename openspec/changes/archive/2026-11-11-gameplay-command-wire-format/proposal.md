## Why

> 路线图阶段（(f) 已拍板定稿）：本 change 按 `openspec/config.yaml` rules.proposal 的 P0–P5 阶段枚举，依附 P4 `lockstep-networking` 与 P5 `gameplay-systems` 两个能力面的补齐，不新增玩法机制。相对 [ADR 0003](../../../docs/decisions/0003-scope.md) 分期表已有的 P6「集成收尾」行，**已定案：不新增阶段、标注为「P6 集成收尾的后续补齐」**，不单列独立 P7；`config.yaml` rules.proposal 的「P0-P5」枚举保持不动（见 `design.md` 的 D6 与 Open Questions）。相关 ADR：[ADR 0001](../../../docs/decisions/0001-license.md)（资产与法律）、[ADR 0002](../../../docs/decisions/0002-determinism.md)（确定性契约）、[ADR 0003](../../../docs/decisions/0003-scope.md)（范围与分期）、[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)（冻结坐标 / 拒绝 `f64`）、[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md)（protobuf 载荷折中方案）。

P5 引入的 `GameplayCommandKind`（10 个变体：`Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `RegisterBehavior` / `SpawnWave` / `SetAi` / `Leave`）只在本地模式可用：`w3-net` 的 protobuf 线格式（`w3net.proto` 的 `Command.kind` oneof、`payload.rs::to_pb_kind` / `from_pb_kind` 的穷尽 match）只承载 `CommandKind`，`FrameCommands` / `apply_frame` / `submit_input` / `FrameAssembler` 全链路也只处理 `Command`。P6 的 `crates/w3-client/src/net_driver.rs::StepDriver::enqueue` 对 `ClientCommand::Gameplay(_)` 显式返回 `ClientError::GameplayUnsupportedOnline` 并记英文日志 `gameplay command unsupported: reason=wire_format_command_only`，绝不静默本地 `submit_gameplay_command`（否则两端命令流不一致将 desync，违反 ADR 0002）。该边界已被 `critic` 三轮复审定性为「非阻塞的合规显式失败」，并登记在 `p6-playable-client/design.md` 的 Risks「已裁定·已知边界」条。编排者裁定「补齐需扩展 schema 并 bump `PROTOCOL_VERSION`（ADR 0005），另开独立 change 承载」——该结论已落实，本 change 即该独立 change，只补齐**联机线格式与命令流**，使施法 / 生产 / 取消生产等玩法命令在本地与联机行为一致；不新增玩法机制、不新增权威状态。本轮仅写规范，代码留到 `/opsx:apply`。

## What Changes

- 扩展 `w3net.proto`（(a) 已拍板）：新增独立 `GameplayCommand` 消息（`oneof kind` 用 tag 4–13 覆盖 10 个玩法变体 + `reserved 14 to 32`）与 `GameplayCommandBatch`，并在**顶层 `CommandBatch`** 的 `reserved 2 to 8` 之后**只追加** `GameplayCommandBatch gameplay = 9;`（tag 9，而非 `FrameCommands`；`FrameCommands` / `ResyncReplay` 因此无需新增字段）；既有 `Command` 的 `oneof`（tag 4–10）与 `reserved 11 to 16` 保持不变。理由已由源码核实：玩法命令变体无法映射回 `CommandKind` 的穷尽 match，且 6 个保留位不够 10 个变体（见 `design.md` D1）。
- **`PROTOCOL_VERSION` 由 2 bump 到 3**（(c) 已拍板；ADR 0005 §3「proto schema 变更必须同步 bump」）；`Packet::decode` 对 `version=2` 明确返回 `VersionMismatch` / 由 relay 回 `Reject`，**不静默按旧 schema 解码**；客户端与 relay 同步发布、**无兼容垫片**。核心理由已由源码核实：v2 端会把玩法命令字段当 unknown field 静默丢弃 → 两端命令流不等 → desync 不可见，违反「显式失败不静默降级」（见 `design.md` D3）。
- 扩展 `payload.rs` 的双向编解码：新增玩法命令的 `to_pb_*` / `from_pb_*` 穷尽映射，全部字段用定宽 `int64` / `uint64` / `uint32` / `bool`（`EntityId`→`uint64`、`DefId`→`uint32`、`EffectKind`→稳定码 `uint32`；若命令字段为 `SimFixed` 则用 `raw()` 的 `int64`，该约束仅适用于同批 `CommandBatch` 内的既有普通命令，本 change 的 10 个玩法变体无 `SimFixed` 字段），**禁用 `float` / `double`、禁用 `map`**；`ResyncReplay` 同步承载玩法命令（ADR 0005 §2/§3、ADR 0004）。
- 扩展 `frame.rs` 的命令流（(b) 已拍板）：一帧内两类命令归入同一帧批，按**统一稳定键**（`execute_frame → player_id → sequence → type_tag`，`type_tag` 固定 `Simulation=0` / `Gameplay=1`）归并排序，在**恰好一次 `step()` 之前**全部提交；否决独立批 / 双 `step()`。`Command` 与 `GameplayCommand` 是两条独立队列、各自独立 `arrival`，统一键的作用是规范化归并顺序而非跨类型共享 `arrival`。玩法命令只经 `submit_gameplay_command`（与玩家 / AI 同源校验），MUST NOT 绕过、MUST NOT 双推进（见 `design.md` D2）。
- 打通 `w3-client` 联机通路：去掉 `ClientCommand::Gameplay(_)` 的 `GameplayUnsupportedOnline` 短路，改走 `ClientSession::submit_input` 上行广播；校验失败仍显式失败、不静默本地提交。
- **BREAKING（协议，已获用户确认）**：`PROTOCOL_VERSION` 2→3，客户端与 relay 必须同步发布，不承诺旧客户端混跑（ADR 0005 §3，(c) 已拍板）。
- **BREAKING（API，已获用户确认）**：`FrameCommands` 需携带玩法命令集合，`apply_frame` / `submit_input` / `FrameAssembler` / `CommandLog` 等 `w3-net` / `w3-relay` / `w3-client` 调用点须迁移。
- 一致性（(g) 已拍板，本轮已执行）：在 [ADR 0005](../../../docs/decisions/0005-protobuf-payload.md) **只追加**一条「修订 / 复核」记录（字段编号段扩展、`PROTOCOL_VERSION` 2→3、明确拒绝旧版本、canonical 约束继续适用、实施状态以本 change 的 tasks 为准），不改原决策正文、不改既有段落（见 `design.md` D4）；`docs/decisions/` 本轮仅追加该文件这一条记录。

## Capabilities

### New Capabilities

- `gameplay-command-wire`: 联机玩法命令线格式契约——10 类玩法命令的完整承载、字段编号只追加与 canonical 编码约束、`PROTOCOL_VERSION` bump 与版本拒绝、同源校验与一次 `step()` 前提交、`w3-core` 零改动与哈希 / 快照不变、双端逐帧哈希一致与断线重连、本地与联机行为一致、网络只传输入命令。

### Modified Capabilities

（无。）**刻意不产生 `lockstep-networking` delta（(h) 已拍板，采纳 D8）**：主 `openspec/specs/lockstep-networking/spec.md` 现有 requirements 已覆盖「版本不匹配 `Reject`」「schema 变更同步 bump `PROTOCOL_VERSION`」「canonical 编码约束」「模拟确定性边界不变」，本 change 是这些要求的**行使**而非行为变更；提案阶段 `p6-playable-client` 曾持有该 capability 的 active delta（含 ADDED「线格式不变」与 MODIFIED「Unchanged simulation determinism boundary」），两条 change 同时写同一 capability 会在归档时冲突；P6 现已归档、delta 已进入主 spec 且「线格式不变」措辞已按 A(ii) 修订，本 change 仍不产生 `lockstep-networking` delta。归档顺序（(d) 已拍板：P6 先归档、本 change 后归档）、P6「`PROTOCOL_VERSION` 不变」Scenario 的处理（选项 A：P6 的 change spec 保持原样、不追溯改写；待 P6 归档后由主 `openspec/specs/lockstep-networking/spec.md` 修订措辞，执行载体 A(ii) 一次性一致性维护，不走 change delta）与落点结论见 `design.md` 的 D6 / D7 / D8，均已拍板；本轮不擅自改 P6 的 spec 实质、不碰主 `openspec/specs/`、不提前归档。

## Impact

- **确定性契约（ADR 0002）**：本 change **不新增任何权威状态字段**；玩法命令作为「未执行命令」本就经 `WorldGameplayState::hash → GameplayCommandQueue::hash` 进入 FNV-1a 状态哈希，无需扩展字段集合。状态哈希字段集合、`first_difference()` 语义、帧序、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、30 Hz 整数帧、由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照**全部不变，不 bump 快照版本**。`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿。跨平台确定性已在 Windows 与 Debian 12 x86_64 / rustc 1.99.1 通过离线 golden hash 验证（ADR 0002 复核记录），macOS / 其它 CPU 架构 / 其它 rustc 仍待验证，本 change 不声称全平台已达成。
- **资产与法律边界（ADR 0001）**：不提交任何 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效；线格式只传命令参数（`DefId` / `EffectDef` 是定义 ID 与整数参数，不是游戏资产）。本 change **预期不引入新依赖**（复用既有 `prost` 0.13 / `prost-build` 0.13）；若实施期确需新增，须先检查许可证与 GPLv3 兼容性、提交 `Cargo.lock`、由 `docs-owner` 在 `NOTICE.md` 登记（本 change 不自行编写 `NOTICE.md`）。只读参考项目（含 AGPLv3 的 `WarsmashModEngine`）不作代码复制。
- **分层依赖**：`w3-core` **零改动、零依赖**，不反向依赖 `w3-net` / `w3-relay` / `w3-client`；唯一权威写入者仍是 `RtsMatch`；网络只传玩家输入命令，MUST NOT 传游戏状态（ADR 0005 §4）；表现层不反写模拟状态；`w3-content` 的 `f64` 冻结边界（ADR 0004）不受影响。
- **影响面（`w3-net`）**：`src/proto/w3net.proto`、`packet.rs`（版本常量与测试断言）、`payload.rs`（`FrameCommands` / 双向编解码 / resync replay）、`frame.rs`（`stabilize` / `apply_frame` / `FrameScheduler` / `FrameAssembler`）、`session.rs`（`submit_input` / `pop_ready`）、`checksum.rs` 的诊断路径（字段集合不变）。
- **影响面（`w3-client`）**：`net_driver.rs::StepDriver::enqueue`、`app.rs` 的 `ClientError` 变体、`input.rs` 的 `ClientCommand` 上行路径；去掉 `GameplayUnsupportedOnline` 短路后仍保持「显式失败不静默降级」。
- **影响面（`w3-relay`，源码核对补充）**：`src/lib.rs` 的 `PacketKind::Input` 用 `decode_commands`、`broadcast_frame` 用 `encode_commands`、`CommandLog` 存 `FrameCommands`、`FrameAssembler` 装配帧，均为本 change 的必改点；这一 crate 超出「`w3-net` + `w3-client`」的原有表述，其 owner 归属见下方「owner 归属」条。
- **owner 归属（(e) 已拍板）**：已指定 `net-owner`（单一 owner，覆盖 `crates/w3-net/` / `crates/w3-relay/`），`w3-client` 侧由 `client-owner` 配合，配只读 `critic`；中文 agent 定义由编排者新建（不在 `spec-steward` 可写范围，本轮不写 `.codebuddy/agents/`）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 `critic`（见 `design.md` D5）。
- **BREAKING（两处，均已获用户确认）**：①协议版本 `PROTOCOL_VERSION` 2→3（bump + `Reject` 拒绝旧版本，ADR 0005 §3，(c) 已拍板）；②`FrameCommands` / `apply_frame` / `submit_input` 的公开形状变化需迁移所有调用点（含 `w3-relay/tests/two_clients.rs`、`network_faults.rs` 与 P6 的 `net_driver.rs`，(a)/(b) 已拍板）。两处 BREAKING 已由用户拍板确认接受。
- **规范边界**：本轮写 `openspec/changes/gameplay-command-wire-format/` 下四件套，并在 `docs/decisions/0005-protobuf-payload.md` 只追加一条「修订 / 复核」记录；不碰主 `openspec/specs/` / `p6-playable-client/` / `crates/`、不写 `NOTICE.md` / `CODEBUDDY.md` / `.codebuddy/` / `tools/` / 根配置、不运行 `cargo`、不 commit、不归档、不执行任何 git 远端命令。
