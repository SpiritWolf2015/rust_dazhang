# gameplay-command-wire Specification

## Purpose
本规范定义 P5 的 10 类玩法命令（`Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `RegisterBehavior` / `SpawnWave` / `SetAi` / `Leave`）在联机线格式上的承载契约：字段编号只追加、canonical 编码、协议版本随 schema 变更同步递增并明确拒绝旧版本、玩法命令经同源校验后在恰好一次 `step()` 前全部提交，以及本地与联机行为一致；`w3-core` 零改动、状态哈希与快照不变。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf 载荷折中方案）。

## Requirements

### Requirement: Wire format carries all gameplay command kinds

联机线格式 SHALL 承载 P5 `GameplayCommandKind` 的**全部 10 个变体**及其全部字段，且 MUST 与 `w3_core::GameplayCommand` 的 `execute_frame` / `player_id` / `sequence` / `kind` 一一对应、双向可逆。字段 SHALL 使用定宽 `int64` / `uint64` / `uint32` / `bool`，MUST NOT 使用 `float` / `double`（ADR 0005 §2、ADR 0004）；若命令字段为 `SimFixed` 则用 `raw()` 的 `int64`，本 change 的 10 个玩法变体无 `SimFixed` 字段（该约束仅适用于同批 `CommandBatch` 内的既有普通命令）。`arrival` MUST NOT 编码，仍由各端按稳定序提交时本地分配（ADR 0005 §3、ADR 0002 §3）。相关 ADR：ADR 0001 / 0002 / 0003 / 0004 / 0005。

#### Scenario: 10 类玩法命令编解码往返

- **WHEN** 依次编码并解码 `Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `RegisterBehavior` / `SpawnWave` / `SetAi` / `Leave` 各一条玩法命令
- **THEN** 解码得到的 `GameplayCommand` 与原命令的 `execute_frame` / `player_id` / `sequence` / `kind` 逐字段相等，且无变体丢失或歧义映射

#### Scenario: 无浮点进入线格式

- **WHEN** 检查新增的玩法命令消息定义
- **THEN** 不含 `float` / `double` / `map` 字段，且 `EffectKind` 以稳定码而非枚举索引隐式传递；若命令字段为 `SimFixed` 则其定点数值用 `raw()` 的 `int64`，本 change 的 10 个玩法变体实际字段为 `EntityId` / `DefId` / `u32` / `bool` / `u8` / `EffectDef`、无 `SimFixed` 字段（该约束仅适用于同批 `CommandBatch` 内的既有普通命令）

#### Scenario: 10 变体字段定宽映射

- **WHEN** 逐字段检查 10 个玩法变体消息的字段类型
- **THEN** `EntityId` 用 `uint64` 的 `value()`、`DefId` 用 `uint32`、`EffectKind` 用稳定码 `uint32`、`EffectDef.amount` 用 `int64`，全部为定宽整数 / 布尔，无 `float` / `double` / `map` 字段

### Requirement: Append-only field numbers and canonical protobuf encoding

既有 `.proto` 消息的字段编号与语义 SHALL 保持不变；新增字段 MUST 只追加在既有 `reserved` 段之后已规划的新 tag 段，MUST NOT 复用、改名或重排既有字段编号；删除一律保留 `reserved`（ADR 0005 §3）。系统 MUST NOT 使用 `map` 字段（集合一律 `repeated` + 显式顺序），MUST NOT 透传或回显 unknown fields，MUST 锁定 `prost` / `prost-build` 版本并写入 `Cargo.lock`，MUST NOT 产生非规范 varint，且生成代码在 Windows / Linux 双平台编码结果 MUST 逐字节一致（ADR 0005 §3）。10 个玩法变体与现有仅 6 位的 `reserved 11 to 16` 的 tag 段规划 SHALL 在 `design.md` 的 D1 显式给出并经拍板，MUST NOT 挤压或复用既有 `Command` 的保留位。

#### Scenario: 只追加不复用

- **WHEN** 为玩法命令扩展 `w3net.proto`
- **THEN** `Command` 的 7 个既有 `oneof` 变体 tag（4–10）与 `reserved 11 to 16` 保持不变、既有控制包字段编号不变，新增内容只出现在新规划 tag 段，且任一保留位未被占用

#### Scenario: canonical 往返与未知字段丢弃

- **WHEN** 对同一批（普通 + 玩法）命令编码两次，或解码带有合法但未知字段的字节后再编码
- **THEN** 两次编码逐字节相同、解码→再编码结果相同，未知字段被丢弃且不回显，双平台生成代码字节一致

#### Scenario: schema 变更同步版本

- **WHEN** 玩法命令相关的 `.proto` 字段发生增改
- **THEN** `PROTOCOL_VERSION` 同步递增，旧版本客户端被明确拒绝（见「Protocol version bump and rejection」requirement）

#### Scenario: 既有字段编号清单保持不变

- **WHEN** 为玩法命令扩展 `w3net.proto` 前核对既有消息定义
- **THEN** `Command` 的 `oneof` tag 4–10 与 `reserved 11 to 16`、`CommandBatch` 的 `reserved 2 to 8`、`FrameCommands` 的 `reserved 3 to 6`、各控制包 `reserved` 的字段编号与语义均保持不变，且新增字段只追加在已规划的新 tag 段

### Requirement: Protocol version bump and rejection

`PROTOCOL_VERSION` SHALL 由 **2 bump 到 3**（ADR 0005 §3「proto schema 变更必须同步 bump」）；`Packet::decode` MUST 对 `version != PROTOCOL_VERSION` 明确返回 `VersionMismatch`，relay 侧 MUST 以 `Reject` 明确拒绝，MUST NOT 尝试按新 schema 解码旧版本 payload（ADR 0005 §3、主 `lockstep-networking` spec「版本不匹配」Scenario）。客户端与 relay SHALL 同步发布，不承诺旧客户端长期混跑；前后兼容只用于平滑升级与工具链（ADR 0005 §3）。**version=2 客户端绝不可与 version=3 端点静默混跑**：protobuf 默认丢弃 unknown fields，v2 端会把携带玩法命令的新字段静默忽略，造成两端命令流不一致进而 desync（ADR 0002、CODEBUDDY.md「显式失败不静默降级」硬红线）。

#### Scenario: 旧版本包被明确拒绝

- **WHEN** 一个 `version=2` 的包到达 `version=3` 的端点
- **THEN** `Packet::decode` 返回 `VersionMismatch` 或 relay 回 `Reject`，且不按新 schema 解码其 payload、不进入权威命令流

#### Scenario: version=3 端点编码

- **WHEN** 编码任意一个命令包或控制包
- **THEN** 前 25 字节 `W3N1` 帧头布局与 FNV-1a 覆盖范围不变，仅 version 字节为 3（ADR 0005 §1）

### Requirement: Same-source validation and single step before submission

一帧内的普通命令与玩法命令 SHALL 归入同一权威帧批，按统一稳定键（`ExecuteFrame → PlayerId → Sequence → type_tag`，`type_tag` 为区分两类命令的固定判别，不改 ADR 0002 §3 的排序语义）稳定排序，并在**恰好一次** `RtsMatch::step()` 之前**全部提交**（ADR 0002 §2/§3、ADR 0005 §3）。玩法命令 MUST 只经 `RtsMatch::submit_gameplay_command`（内部走 `validate_gameplay_command`，与玩家 / AI 命令同源校验）提交，MUST NOT 绕过校验直接写权威状态、MUST NOT 因新增玩法命令而引入第二次 `step()` 或任何额外推进（ADR 0002 §2）。联机键鼠玩法命令 MUST 经 `ClientSession::submit_input` 走上行广播，MUST NOT 本地静默 `submit_gameplay_command`（否则两端命令流不一致导致 desync）。

#### Scenario: 一次 step 前全部提交

- **WHEN** 一帧批内同时含普通 `Command` 与玩法 `GameplayCommand`
- **THEN** 两类命令按统一稳定键排序后，普通命令经 `submit_command`、玩法命令经 `submit_gameplay_command` 全部提交，随后仅调用一次无参 `step()`，返回 `compute_state_hash()`

#### Scenario: 玩法命令同源校验

- **WHEN** 一条玩法命令的 `execute_frame` 早于当前帧或含 `EntityId::NONE` 实体
- **THEN** `submit_gameplay_command` 经 `validate_gameplay_command` 明确返回 `FrameInPast` / `InvalidEntity`，该命令不被执行且不静默降级

#### Scenario: 不本地静默提交

- **WHEN** 联机模式下产生一条 `ClientCommand::Gameplay`
- **THEN** 该命令只进入上行广播的输入流，本地不直接 `submit_gameplay_command`，两端收到相同权威帧批后逐帧哈希一致

### Requirement: Unchanged w3-core, state hash and snapshot

`w3-core` MUST 保持**零改动、零依赖**，且不反向依赖 `w3-net` / `w3-relay` / `w3-client`（ADR 0002 §5、ADR 0005 §4）。本 change MUST NOT 新增任何权威状态字段；状态哈希字段集合、`first_difference()` 语义与由 `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）定义的快照 SHALL 全部**不变、不 bump 快照版本**；`crates/w3-core/tests/determinism.rs` MUST 重跑且全绿（ADR 0002 §6/§7/§8、CODEBUDDY.md §5）。`CommandKind` 与 `payload.rs` 对 `CommandKind` 的穷尽 match MUST 保持原样，本 change 只新增玩法命令的编解码映射。

#### Scenario: 内核零改动与确定性基线

- **WHEN** 校验分层依赖与确定性基线
- **THEN** `crates/w3-core/` 无 diff，其依赖不含 `w3-net` / `w3-relay` / `w3-client`，且 `cargo test -p w3-core --test determinism` 全绿

#### Scenario: 哈希与快照字段集合不变

- **WHEN** 在联机对局中提交玩法命令后比对状态哈希与快照
- **THEN** `SNAPSHOT_VERSION` 仍为 3、头部含 `defs_hash` / `content_hash`，状态哈希字段集合与本地对局一致，未新增权威状态字段

#### Scenario: 玩法命令经既有队列进入状态哈希

- **WHEN** 在联机对局中提交玩法命令后计算权威状态哈希
- **THEN** 玩法命令作为「未执行命令」本就经 `WorldGameplayState::hash → GameplayCommandQueue::hash` 进入 FNV-1a 状态哈希，因此无需新增权威状态字段，状态哈希字段集合与本地对局一致

### Requirement: Per-frame hash equality and invariant frame ordering

两个客户端 SHALL 在解码出相同（普通 + 玩法）命令流并按既定帧序提交后，逐帧 `compute_state_hash()` 相等且 `first_difference()` 返回 `None`（ADR 0002 §6、ADR 0005）。帧编排（`FrameScheduler` / `FrameAssembler`）、`stabilize` 稳定排序、`apply_frame` 依序提交使 `arrival` 一致、逐帧 checksum 比对、`ResyncReplay` 补发与断线重连游标 MUST NOT 因新增玩法命令而改变语义（ADR 0005 §3）。网络抖动、丢包重传、乱序到达与断线恢复 MUST NOT 改变命令执行帧序、改变 `step()` 次数或造成玩法命令重复执行 / 历史事件重放（ADR 0002 §3、主 `lockstep-networking` spec）。

#### Scenario: 双端逐帧哈希一致（含玩法命令）

- **WHEN** 两个客户端逐帧应用同一批含 `Cast` / `Train` / `CancelTrain` 的权威帧命令集
- **THEN** 每帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`，且逐帧 checksum 交换无 `Desync`

#### Scenario: 乱序抖动断线不改帧序

- **WHEN** 含玩法命令的帧包被延迟、重传补齐或乱序到达，或客户端短暂断线后追帧
- **THEN** 玩法命令仍在原定 `ExecuteFrame` 执行一次，帧序与 `step()` 次数不变，追帧后状态哈希与健康客户端一致且不重放已提交命令

#### Scenario: ResyncReplay 携带玩法命令

- **WHEN** relay 为断线客户端补发多帧 `ResyncReplay`
- **THEN** 每帧 `FrameCommands` 同时含普通与玩法命令且顺序与原始权威帧批一致，恢复后逐帧哈希与健康客户端一致

### Requirement: Local and online gameplay command parity

联机模式 SHALL 支持与本地模式**同一组** 10 类玩法命令（施法 / 生产 / 取消生产 / 研究 / 购买物品 / 采集 / 注册行为 / 波次 / AI 设置 / 离开），`StepDriver::enqueue` MUST NOT 再对 `ClientCommand::Gameplay(_)` 返回 `GameplayUnsupportedOnline` 或以 `reason=wire_format_command_only` 拒绝（P6 的「已裁定·已知边界」由本 change 解除）。本地与联机对局对同一条玩法命令 SHALL 产生相同的校验结果与权威状态变更；命令被内核拒绝或网络上行失败时 MUST 仍以英文结构化 `key=value` 日志明确报告并显式返回错误，MUST NOT 静默降级或本地代跑（CODEBUDDY.md §3、ADR 0002「显式失败不静默降级」）。

#### Scenario: 联机玩法命令可用

- **WHEN** 联机对局中玩家触发施法、生产、取消生产
- **THEN** 对应 `GameplayCommand` 经上行广播进入权威帧批并在两端执行，不再返回 `GameplayUnsupportedOnline`，本地与联机结果一致

#### Scenario: 失败仍显式报告

- **WHEN** 一条联机玩法命令上行失败或被 `validate_gameplay_command` 拒绝
- **THEN** 以英文结构化日志明确记录 `frame` / `player` / `error`，显式返回 `ClientError`，不静默本地提交、不改变两端命令流

### Requirement: Network transmits only input commands

网络层 SHALL 只传输玩家输入命令（普通 `Command` 与玩法 `GameplayCommand`），MUST NOT 传输游戏状态、冻结定义内容、`WorldGameplayState` 或任何表现层数据（ADR 0005 §4、主 `lockstep-networking` spec「Transmit only input commands」）。玩法命令消息中的 `DefId` / `EffectDef` MUST 仅作为命令参数（ID 与整数值）传递，冻结定义本身仍由本机 `config.toml` 路径读取、不入库、不进线格式（ADR 0001、ADR 0003、ADR 0004）。表现层 MUST NOT 反写 `RtsMatch` 权威状态（ADR 0002 §5）。

#### Scenario: 只传输入不传状态

- **WHEN** 检查联机玩法命令包的 payload 内容
- **THEN** 仅含 `execute_frame` / `player_id` / `sequence` / `kind` 及定宽整数参数，不含生命值、经济、实体全量状态或冻结定义表

#### Scenario: 资产不入线格式与仓库

- **WHEN** 实施与评审本 change 的全部产物
- **THEN** 仓库不含 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效，冻结定义仍留玩家本机，线格式不含资产字节
