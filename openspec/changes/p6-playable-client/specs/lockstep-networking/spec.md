> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0005（protobuf 载荷折中方案）。

## ADDED Requirements

### Requirement: Client session feeds the shared presentation pipeline

联机对局 SHALL 通过把 `ClientSession` 适配为 `FrameProducer` 复用本地对局的同一 `RendererApp::run()` / `Scene` / `GpuContext` 呈现管线，MUST NOT 为联机模式建立第二套呈现代码或第二份 `RtsMatch` 权威状态（ADR 0002 §5、ADR 0005）。适配层 SHALL 只把已冻结帧的输入命令经 `submit_command` / `apply_frame` 提交，`w3-core` MUST 保持零改动且不反向依赖 `w3-net` / `w3-render` / `w3-client`。

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
- **THEN** 前 25 字节 `W3N1` 帧头与 protobuf payload 逐字节符合既有规范，`PROTOCOL_VERSION` 不变

## MODIFIED Requirements

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
