## Context

P0 建立确定性基线（ADR 0002；跨平台确定性目前仅在 Windows / rustc 1.91 实跑，Linux / macOS 为待验证项），P2 有玩法。`vera20k` 的 `src/net/` 与 `services/relay-protocol` 提供了生产级的 lockstep、UDP 可靠层、认证数据报、排序时钟、自适应冗余、断线重连与回放设计，但其 Event 线格式复刻 RA2 `FRAMEINFO` / `FRAMESYNC`（`64B4D0` 等），不能照搬。本 change 按 [ADR 0003](../../../docs/decisions/0003-scope.md) 路线图的 **P4 阶段**推进；资产 / 法律边界见 [ADR 0001](../../../docs/decisions/0001-license.md)，确定性契约见 [ADR 0002](../../../docs/decisions/0002-determinism.md)，冻结坐标与定点单位见 ADR 0004。

## Goals / Non-Goals

**Goals:** 双客户端同图对战、逐帧 checksum 对比、断线恢复；与官方服务器无关。

**Non-Goals:** gamemd/IPX 线兼容；官方战网/BNET；预测回滚；反作弊；第一版 30 人规模承诺。

## Decisions

四个决策共同受 [ADR 0002](../../../docs/decisions/0002-determinism.md) 的确定性契约与 config.yaml context 的架构设计原则约束：SRP / OCP / LSP / DIP / ISP / LoD / CARP 按 Rust 语境（模块 / trait / 组合 / 所有权）判定，正确性 / 确定性优先；硬红线为确定性契约、唯一状态所有者、`w3-core` 不反向依赖上层与冻结输入边界、显式失败不静默降级、只读参考不复制代码。网络层只编排输入命令、不反写权威状态，不新增权威状态、不改动状态哈希与快照 v1；下列每个决策分别列出考虑过的备选方案，并说明对跨平台确定性与分层依赖的影响。

### 决策一：借鉴架构、重写线格式

- **结论**：移植 lockstep / reliable UDP / relay 时钟的设计，定义自己的 Event 包。否决复用 RA2 原生包布局。
- **考虑过的备选方案**：
  - 直接复用 RA2 `FRAMEINFO` / `FRAMESYNC` 包布局：否决，与 `gamemd.exe` 私有协议绑定、语义不可移植，且与 ADR 0003「原生 Event 线格式不复用」相冲突。
  - 复制 `vera20k` 的 `src/net/` / `services/relay-protocol` 代码：否决，其 GPLv3 + Section 7 附加条款会随代码引入（ADR 0001 §2），且硬红线要求只读参考不复制代码。
  - 用通用序列化框架（serde / protobuf）承载 Event：否决，第一版不引入新 crates.io 依赖（传输层用 `std::net`），自定义定长头 + 显式小端 + FNV-1a 校验更易锁定字节级确定性。
  - 引入 Event 压缩 / 字典缓存：暂缓，第一版以正确性优先，作为已知残差（见「已决 Open Questions」）。
- **对跨平台确定性的影响（ADR 0002）**：`W3N1` 线格式（`crates/w3-net/src/packet.rs`）用固定 25B 头、显式小端整数与 FNV-1a 校验，字节编码在大小端 / 不同字长平台上一致；命令帧交给 `w3-core` 后仍按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序，`SimFixed` Q47.16 整数数学、30 Hz 整数帧与 FNV-1a 状态哈希均不变。ADR 0002 的跨平台确定性目前仅 Windows / rustc 1.91 实跑，Linux / macOS 仍是待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：DIP / ISP——`w3-net` 通过 `Transport` trait 与既定队列相位向 `w3-core` 提交命令，`sim/` 不读 socket、`w3-core` 不反向依赖 `w3-net`；SRP——线格式（`packet.rs`）、可靠传输（`reliable.rs`）、帧编排（`frame.rs`）分离，唯一状态所有者仍是 `RtsMatch`。显式失败——校验和不匹配、版本不匹配明确返回 `NetError` / `Reject`，不静默降级。冻结坐标（ADR 0004）只在冻结边界转换，命令参数沿用定点 / 整数表示，不引入 `f64` 进权威状态。

### 决策二：relay 持有时钟

- **结论**：2 人以上对局用 relay 认证时钟与不可变命令流；LAN 直连走独立 UDP。否决"纯 P2P 无权威时钟"。
- **考虑过的备选方案**：
  - 纯 P2P、各端自行关帧且不设权威时钟：否决，缺席输入 / 网络延迟会让各端帧边界与命令集分叉，破坏 ADR 0002「相同输入逐帧一致」的前提。
  - 每客户端独立按墙钟 / 渲染帧率推进：否决，`step()` 不接受 delta 与墙钟，30 Hz 整数帧已冻结（ADR 0002）。
  - relay 自适应冗余 / 调速替代「降到最慢玩家」：暂缓，第一版不引入，作为已知残差（见「已决 Open Questions」）。
  - 复用官方战网 / BNET 或第三方托管服务：否决，ADR 0003 明确不做，且涉及资产 / 法律边界（ADR 0001）。
- **对跨平台确定性的影响（ADR 0002）**：`FrameAssembler`（`crates/w3-net/src/frame.rs`）在固定 `timeout_ticks` 后按缺席玩家空输入关闭帧，`Lobby` 分配稳定 `player_id`、`CommandLog` 每帧只记录一次、`Relay::tick` 广播不可变命令流，保证所有客户端收到相同的权威帧命令集；帧号来自逻辑计数，墙钟与 `sleep(33ms)` 只用于节流网络轮询，不进入 `compute_state_hash()` / 快照 v1。ADR 0002 的跨平台确定性仍仅 Windows / rustc 1.91 实跑，Linux / macOS 为待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：唯一状态所有者 + SRP——relay 只持有 lobby / 命令日志 / 时钟，不持有也不反写 `RtsMatch`；LoD / CARP——`ClientSession` 不持有 `RtsMatch`，命令只经 `apply_frame` 提交（此单一提交路径目前是分层约定、未由类型系统强制，属已知残差，见 `vera20k-mapping.md`）。显式失败——版本不匹配发 `Reject` 明确拒绝、缺失输入按空输入关闭帧而非无限等待。资产 / 法律（ADR 0001）：自建 relay 不引入新许可证依赖，SaaS 化风险见 Risks。

### 决策三：只传命令

- **结论**：由既定队列相位把命令交给模拟；`sim/` 不读 socket。否决传输层持有第二份命令队列。
- **考虑过的备选方案**：
  - 传输层缓存第二份命令队列 / 直接写 `RtsMatch`：否决，破坏唯一状态所有者与 ADR 0002 的状态权威边界。
  - 传输权威状态快照 / 增量而非输入命令：否决，会让 `f64` 与平台相关编码进入权威状态、状态哈希与快照 v1，且与 30 Hz 整数帧冻结冲突。
  - 预测回滚（rollback）：否决，第一版 Non-Goals，需额外的确定性重放基础设施。
  - 按网络到达顺序立即执行：否决，到达顺序 MUST NOT 决定执行顺序（ADR 0002）。
- **对跨平台确定性的影响（ADR 0002）**：只传输入命令，各端运行相同模拟代码、按相同帧序执行相同命令，逐帧 `compute_state_hash()` 相等（对应 spec「相同命令流」Scenario）；`FrameScheduler` 按帧缓存、`stabilize` 稳定排序、`apply_frame` 依序 `submit_command` 后恰好 `step()` 一帧，丢包重传补齐后仍在原定帧执行（对应「丢包重传」Scenario）；不新增权威状态，状态哈希与快照 v1 的字段集合不变。ADR 0002 的跨平台确定性仍仅 Windows / rustc 1.91 实跑，Linux / macOS 为待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：DIP / ISP——`w3-core` 只暴露命令提交与只读视图，`w3-net` / `w3-relay` 单向依赖它，`sim/` MUST NOT 依赖 `net/`；冻结输入边界（ADR 0004）——命令参数在冻结边界转为 `SimFixed` / 整数，`f64` 不进权威状态。显式失败——`ChecksumExchange` 比对不一致返回 `NetError::Desync` 并保留 `DesyncReport` / `first_difference_field`，不静默继续；断线恢复按 `SessionCursor` 过滤已应用帧，不重复执行已提交命令或历史事件。

### 决策四：自建服务

- **结论**：按 ADR 0001，自建 lobby/relay 是本项目基于资产 / 法律边界的独立决定，不依赖官方服务器；「不给官方 lobby/relay 访问权」是 `vera20k` 在 GPLv3 之上**自行附加**的 Section 7 条款、并非 GPLv3 固有限制，仅在真的引入其代码时才随之进入。
- **考虑过的备选方案**：
  - 接入官方战网 / BNET 或依赖 `vera20k` 官方服务器：否决，ADR 0003 明确不做，且商标 / 资产合规与第三方服务授权不在本项目许可证覆盖内（ADR 0001）。
  - fork `vera20k` 的 relay 服务并改造：否决，约 97% 代码绑死 RA2，且复制代码会带入其 Section 7 附加条款（ADR 0001 §2）；只读参考不复制代码是硬红线。
  - 用通用消息中间件 / 云托管队列承载命令流：否决，越界引入庞大依赖与隐式调度，且 SaaS 化会触发 GPLv3 / AGPLv3 评估（ADR 0001）。
- **对跨平台确定性的影响（ADR 0002）**：relay 是 lib + bin 的自建 UDP 服务，只负责 lobby、认证时钟、不可变命令流与版本拒绝，不参与 `compute_state_hash()` / 快照 v1；服务部署平台（Windows / Linux）不改变帧序与命令内容，各端仍以 `w3-core` 的定点 / 整数帧语义推进。ADR 0002 的跨平台确定性仍仅 Windows / rustc 1.91 实跑，Linux / macOS 为待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：SRP / OCP——`w3-relay`（服务编排）与 `w3-net`（客户端传输）分离，app 层接入，`w3-core` 不反向依赖任何上层；唯一状态所有者仍是 `RtsMatch`。资产 / 法律边界（ADR 0001）与范围（ADR 0003）：仓库不含任何游戏资产，导出数据 / 密钥 / relay ticket 留本机，自建 lobby/relay 是本项目基于该边界的独立决定；四个上游只读参考（`WarsmashModEngine` 为 AGPLv3）、不复制代码。显式失败——版本不匹配发 `Reject` 明确拒绝，客户端与 relay 同步发布、不承诺旧客户端兼容。

## Risks / Trade-offs

- [客户端与 relay 需同步发布] → 不承诺旧版本兼容，版本不匹配明确拒绝。
- [在线对局的 SaaS 化不在 GPLv3 覆盖内] → 见 ADR 0001；必要时评估 AGPLv3。
- [30 人 / 20,000 单位规模未验证] → 第一版以双客户端与小规模验收为准。

## Migration Plan

新增 `w3-net` / `w3-relay`；app 层接入。不改动 `w3-core` 权威状态语义（ADR 0002）。

## 已决 Open Questions（P4 实现时冻结）

- **Event 编解码与压缩缓存的具体格式**：P4 实现时**已决**采用自定义 `W3N1` 线格式（`crates/w3-net/src/packet.rs`）：固定 25B 头 + 显式小端整数 + FNV-1a 校验 + 全 `CommandKind` 编码，**不复用** RA2 `FRAMEINFO` / `FRAMESYNC`；线格式只承载输入命令，不进入权威状态、状态哈希与快照 v1（ADR 0002）。**必须如实标注残差**：Event 编解码的压缩 / 字典缓存**未实现**，第一版以正确性优先、命令量小，压缩留待后续，属已知残差（见 `openspec/changes/p4-lockstep-networking/vera20k-mapping.md` 的残差记录），不得声称已完成。
- **是否引入 relay 自适应调速（替代原生「降到最慢玩家」）**：P4 实现时**已决第一版不引入**，沿用「降到最慢玩家」——`FrameAssembler`（`crates/w3-net/src/frame.rs`）在固定 `timeout_ticks` 后按缺席玩家空输入关闭帧，保证所有客户端收到相同的权威帧命令集；帧号来自逻辑计数，墙钟不进入 `compute_state_hash()` / 快照 v1（ADR 0002）。自适应冗余 / 调速留待后续，作为残差记入 `vera20k-mapping.md`，不得声称已完成。
