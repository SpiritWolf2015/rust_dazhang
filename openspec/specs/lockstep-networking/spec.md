# lockstep-networking Specification

## Purpose
本规范定义多端命令帧编排、可靠传输、逐帧一致性校验与断线恢复的联机契约：只传输玩家输入命令，各端按相同帧序执行相同命令；缺帧、丢包、乱序与断线恢复 MUST NOT 改变模拟确定性。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf 载荷折中方案）。

## Requirements

### Requirement: Transmit only input commands

联机 SHALL 只传输真人玩家输入命令、MUST NOT 传输游戏状态；每个客户端 SHALL 运行相同模拟代码并按相同帧序执行相同命令。AI 席位的决策 MUST NOT 作为独立命令注入联机 `CommandBatch` 或经 relay 广播——AI 决策在 `w3-core` 内核侧由既有 `run_ai` 各端确定性重算、其决策状态为权威状态（`AiState`）并进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 8）定义的快照；各端以相同开局冻结定义 / 相同外部命令流构造 `RtsMatch`，`ClientSession::submit_input` MUST NOT 本地静默 `submit_gameplay_command` 注入 AI 命令（R12.2 / R12.7 / §0.3 T5 / ADR 0002 §5 / ADR 0005 §4）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 change 只修订本条 Requirement，`lockstep-networking` 其余 8 条 Requirement（含历史「`SNAPSHOT_VERSION` 仍 3」口径残留）按 P9 Q9' 已拍板登记为已知限制、由 `docs-owner` 后续独立 change 收口。

#### Scenario: 相同命令流

- **WHEN** 两个客户端收到相同的冻结真人命令帧、AI 席位启用内核 `run_ai` 各端重算
- **THEN** 逐帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`；`CommandBatch` 只含真人玩家命令、无 AI 决策命令、无本地静默注入（R12.2 / R12.7 / R12.4 / ADR 0002 §5 / ADR 0005 §4）

#### Scenario: AI 决策不进帧流（R12.7 / 被否台账 R1 / R2）

- **WHEN** 尝试把 `AiScript` 外部命令经 relay 广播注入联机帧流，或启用「双模式可切换」使两套命令源并存
- **THEN** 联机 session 只保留内核 `run_ai` 单一决策源、该路径显式失败或不被接受、不产生双份 AI 命令、两端 `CommandBatch` 一致、逐帧哈希一致不被破坏（P2 已拍板 / R12.7 / ADR 0002）

### Requirement: Reliable UDP delivery and ordering

系统 SHALL 在 UDP 之上提供可靠发送队列、重传与按帧排序；到达顺序 MUST NOT 决定执行顺序；缺失输入 SHALL 按既定帧边界处理而非阻塞无限等待。

#### Scenario: 丢包重传

- **WHEN** 一个命令包丢失但被重传补齐
- **THEN** 该命令仍在原定帧执行，且不改变后续帧序

### Requirement: Per-frame checksum comparison

多人对局 SHALL 交换并比对逐帧校验值；不一致 MUST 被检测并可定位到首个差异字段。

#### Scenario: 检测到 desync

- **WHEN** 某客户端状态哈希与对端不一致
- **THEN** 报告差异帧与 `first_difference()` 字段名，不静默继续

### Requirement: Reconnection and replay recovery

断线客户端 SHALL 保留身份与输入游标，并在恢复后读取错过的帧；重放 MUST NOT 重复执行已提交的命令或历史事件。

#### Scenario: 短暂断线

- **WHEN** 客户端断线后重新接入
- **THEN** 追上当前帧且状态哈希与健康客户端一致

### Requirement: Own frame envelope with protobuf payload

网络层 SHALL 保留 `W3N1` 固定 25 字节帧头（`MAGIC` / `version` / `kind` / `sender` / `sequence` / `frame` / `payload_len` / `checksum`，全部显式小端）及其字节布局不变，并 SHALL 仅将 payload 层改用 Protocol Buffers 编码。`payload_len` 与 FNV-1a 校验 MUST 覆盖 protobuf 编码后的完整 payload 字节；`Packet::encode` / `Packet::decode` 的帧头处理逻辑 MUST NOT 改变。

#### Scenario: 帧头字节布局不变

- **WHEN** 编码任意一个命令包或控制包
- **THEN** 前 25 字节各字段与 `W3N1` 规范逐字节一致，且 checksum 覆盖 protobuf 编码后的 payload

#### Scenario: 版本不匹配

- **WHEN** 收到 `version` 与 `PROTOCOL_VERSION` 不符的包
- **THEN** 明确返回 `Reject`，不尝试按新 schema 解码 payload

### Requirement: Canonical protobuf encoding constraints

系统 SHALL 对 protobuf 载荷施加确定性编码约束：MUST NOT 使用 `map` 字段；字段编号只追加、不复用、不改名且删除后保留 `reserved`；MUST NOT 透传或回显 unknown fields；MUST 锁定 `prost` / `prost-build` 版本并写入 `Cargo.lock`；MUST NOT 产生非规范 varint 编码。命令数值字段 SHALL 使用定宽 `int64` / `uint64` / `uint32` / `bool`，MUST NOT 使用 `float` / `double`；`SimFixed` SHALL 以 `raw()`（i64）传输。proto schema 变更 MUST 同步 bump `PROTOCOL_VERSION`。

#### Scenario: 禁用不确定编码

- **WHEN** 定义或生成 `.proto` schema 并编码同一批命令
- **THEN** schema 不含 `map` 字段、未知字段被丢弃且不回显、生成代码在 Windows / Linux 双平台编码结果逐字节一致

#### Scenario: 定点数值无浮点

- **WHEN** 编码含坐标 / 速度 / 攻击范围的命令
- **THEN** 该字段为 `int64` 的 `SimFixed::raw()`，无 `f32` / `f64` 参与线格式

#### Scenario: schema 变更同步版本

- **WHEN** `.proto` 字段发生增改
- **THEN** `PROTOCOL_VERSION` 同步递增，旧版本客户端被 `Reject` 明确拒绝

### Requirement: Unchanged simulation determinism boundary

网络层 SHALL 只传输玩家输入命令，MUST NOT 传输游戏状态。帧编排（`FrameScheduler` / `FrameAssembler`）、`stabilize` 稳定排序、`apply_frame` 依序提交使 `arrival` 一致、`compute_state_hash()` / 由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照 / `first_difference()` 的字段集合与语义 MUST NOT 改变；`w3-core` MUST 保持零改动且不反向依赖 `w3-net` / `w3-relay` / `w3-render` / `w3-client`。本 change（及 P6 客户端集成层）不新增权威状态字段，故状态哈希与快照无需扩展；联机 session 接入同一呈现管线时亦 MUST NOT 引入第二份权威状态（ADR 0002 §5、ADR 0005）。

#### Scenario: 双端逐帧一致

- **WHEN** 两个客户端解码出相同命令流并按既定帧序提交
- **THEN** 逐帧 `compute_state_hash()` 相等且 `first_difference()` 返回 `None`

#### Scenario: 内核零改动

- **WHEN** 校验分层依赖与确定性基线
- **THEN** `w3-core` 无 diff，其依赖不含 `w3-net` / `w3-relay`，且 `cargo test -p w3-core --test determinism` 全绿

#### Scenario: 联机接入不新增权威状态

- **WHEN** 把 `ClientSession` 适配为 `FrameProducer` 后运行联机对局
- **THEN** 快照版本仍为 3、头部含 `defs_hash`，且状态哈希字段集合与本地对局一致

### Requirement: Client session feeds the shared presentation pipeline

联机对局 SHALL 通过把 `ClientSession` 适配为 `FrameProducer` / `StepDriver`（`NetSessionDriver`）复用本地对局的同一 `FrameProducer` / `StepDriver` 接缝与 `Scene` / `GpuContext` 呈现管线（事件循环由 `w3-client::ClientApp` 装配；`RendererApp::run()` 仍保留在 `w3-render` 作为简易只读循环，完整客户端不调用它），MUST NOT 为联机模式建立第二套呈现代码或第二份 `RtsMatch` 权威状态（ADR 0002 §5、ADR 0005）。适配层 SHALL 只把已冻结帧的输入命令经 `submit_command` / `apply_frame` 提交，`w3-core` MUST 保持零改动且不反向依赖 `w3-net` / `w3-render` / `w3-client`。

#### Scenario: 联机与本地同管线

- **WHEN** 联机对局启动并接入 `ClientSession`
- **THEN** 走与本地 `SimProducer` 相同的 `FrameProducer → Scene → GpuContext` 路径，全程只有一个 `RtsMatch` 写入者

#### Scenario: 联机适配层内核零改动

- **WHEN** 校验分层依赖与确定性基线
- **THEN** `w3-core` 无 diff，且 `cargo test -p w3-core --test determinism` 全绿

### Requirement: Network timing MUST NOT drive the fixed timestep

网络包到达、抖动、丢包重传、乱序与断线恢复 MUST NOT 驱动客户端 30 Hz 累加器、改变 `step()` 调用次数或改变命令排序（ADR 0002 §2、§3）；缺失输入 SHALL 按既定帧边界处理而非按墙钟补帧或无限阻塞。逐帧 `compute_state_hash()` 交换比对、`W3N1` 帧头字节布局、protobuf 编码约束与 `first_difference()` 的字段集合与语义 MUST NOT 改变（ADR 0005）；本 change 不新增权威状态字段，故状态哈希与由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照无需扩展。

#### Scenario: 网络抖动

- **WHEN** 某帧输入包延迟到达或被重传补齐
- **THEN** 该命令仍在原定 `ExecuteFrame` 执行，累加器帧数与 30 Hz 基准一致，逐帧状态哈希与对端相等

#### Scenario: 断线恢复不重放

- **WHEN** 客户端短暂断线后重新接入
- **THEN** 追上当前帧且不重复执行已提交命令、不重放历史事件，状态哈希与健康客户端一致

#### Scenario: 线格式不变

- **WHEN** 联机适配层编码任意命令包
- **THEN** 前 25 字节 `W3N1` 帧头与 protobuf payload 逐字节符合既有规范，协议版本由 `PROTOCOL_VERSION` 常量定义，P6 联机接入未改动 schema；后续 schema 变更须按 ADR 0005 同步 bump 并明确拒绝旧版本
