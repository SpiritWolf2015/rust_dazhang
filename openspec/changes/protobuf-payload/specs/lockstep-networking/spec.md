> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf 载荷折中方案）。

## ADDED Requirements

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
- **THEN** schema 不含 `map` 字段、未知字段被拒绝、生成代码在 Windows / Linux 双平台编码结果逐字节一致

#### Scenario: 定点数值无浮点

- **WHEN** 编码含坐标 / 速度 / 攻击范围的命令
- **THEN** 该字段为 `int64` 的 `SimFixed::raw()`，无 `f32` / `f64` 参与线格式

#### Scenario: schema 变更同步版本

- **WHEN** `.proto` 字段发生增改
- **THEN** `PROTOCOL_VERSION` 同步递增，旧版本客户端被 `Reject` 明确拒绝

### Requirement: Unchanged simulation determinism boundary

网络层 SHALL 只传输玩家输入命令，MUST NOT 传输游戏状态。帧编排（`FrameScheduler` / `FrameAssembler`）、`stabilize` 稳定排序、`apply_frame` 依序提交使 `arrival` 一致、`compute_state_hash()` / 快照 v1 / `first_difference()` 的字段集合与语义 MUST NOT 改变；`w3-core` MUST 保持零改动且不反向依赖 `w3-net` / `w3-relay`。本 change 不新增权威状态字段，故状态哈希与快照 v1 无需扩展。

#### Scenario: 双端逐帧一致

- **WHEN** 两个客户端解码出相同命令流并按既定帧序提交
- **THEN** 逐帧 `compute_state_hash()` 相等且 `first_difference()` 返回 `None`

#### Scenario: 内核零改动

- **WHEN** 校验分层依赖与确定性基线
- **THEN** `w3-core` 无 diff，其依赖不含 `w3-net` / `w3-relay`，且 `cargo test -p w3-core --test determinism` 全绿
