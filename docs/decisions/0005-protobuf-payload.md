# ADR 0005：网络消息采用 protobuf 载荷 + 自有 W3N1 帧头（折中方案）

- **状态**：Accepted（设计决策已定；代码实施由后续独立 OpenSpec change 承载，尚未落地）
- **日期**：2026-11-08
- **相关**：[ADR 0001](0001-license.md)、[ADR 0002](0002-determinism.md)、[ADR 0003](0003-scope.md)、[ADR 0004](0004-frozen-coordinates.md)；`crates/w3-net/`、`crates/w3-relay/`
- **实现对应**：待实施（P4 已归档于 `2026-11-08-p4-lockstep-networking`；实施走后续独立 change）
- **修订**：P4 `design.md` 决策一中「用通用序列化框架（serde / protobuf）承载 Event：否决」一条，修订为「仅载荷层采用 protobuf，帧头仍为自有 `W3N1`」

## 背景（Context）

P4 实现的网络层使用自研 `W3N1` 线格式（`crates/w3-net/src/packet.rs`）：固定 25B 头（`MAGIC` / `version` / `kind` / `sender` / `sequence` / `frame` / `payload_len` / `checksum`）+ 显式小端整数 + FNV-1a 32 校验，payload 由手写 `Buf` / `Reader` 定长编码全部 `CommandKind` 变体。`w3-net` 无序列化框架依赖，仅依赖 `w3-core` / `log` / `thiserror`，可 `cargo build --offline`。

P4 `design.md` 决策一曾明确否决「用通用序列化框架（serde / protobuf）承载 Event」，理由是第一版不引入新 crates.io 依赖，且自定义定长头 + 显式小端 + FNV-1a 更易锁定字节级确定性。该判断在当时（双客户端竖切、客户端与 relay 同步发布）成立。

现决定引入 Protocol Buffers，动因是面向后续演进的**schema 演进能力与跨语言能力**（观战 / 回放分析 / 独立机器人 / 非 Rust 客户端），以及减少手写编解码的维护风险。本 ADR 只回答「为什么这样决定、边界在哪」，系统行为由后续 OpenSpec change 的 spec 描述。

**必须先澄清的一个事实**：P4 的逐帧 checksum 比对的是 `compute_state_hash()` —— 即模拟权威状态哈希，**不是网络包的字节**。因此 protobuf 编码层面的不确定性**不会直接造成 desync**：两端只要解码出相同的 `Command` 并按既定帧序提交，状态就一致（[ADR 0002](0002-determinism.md)）。风险不在「状态哈希」，而在 ADR 0002 的「显式字节编码」契约，以及包级 replay / 审计日志 / FNV-1a 完整性校验 / 版本拒绝都建立在「字节布局由本项目冻结」这一前提上。把字节布局交给库实现，等于把一部分确定性边界外包，必须用下文约束补回。

## 决策（Decision）

### 1. 分层：保留自有 envelope，仅替换 payload

- **`W3N1` 固定头保持不变**，仍为显式小端 25B：`MAGIC` 4B、`version` 1B、`kind` 1B、`sender` 1B、`sequence` 4B、`frame` 8B、`payload_len` 2B、`checksum` 4B。
- 这些字段承载的是 **lockstep 帧语义**（帧号、可靠层 sequence、包类型、版本拒绝、完整性校验、发送者），不是业务数据，不交给通用序列化。保留它们即保留了：字节级可复现的帧边界、ADR 0002 的显式编码契约、现有重传 / 去重 / 帧编排 / 版本拒绝机制，以及「到达顺序不决定执行顺序」的稳定排序键。
- **仅把 payload 层** —— 即 `encode_commands` / `decode_commands` 与 `Welcome` / `Ack` / `Checksum` / `ResyncRequest` / `ResyncReplay` / `Reject` 的 payload 构造与解析 —— 换成 Protocol Buffers 消息。
- `payload_len` 与 FNV-1a 校验继续覆盖 **protobuf 编码后的字节**；`Packet::encode` / `Packet::decode` 的头处理逻辑不变。

### 2. Rust 实现选型

- 采用 `prost` 系（纯 Rust、生成 `#[derive(Message)]` 结构体，适合 UDP 小包）；`.proto` 定义 `CommandBatch`、`Command`（`CommandKind` 用 `oneof`）、以及各控制包 payload 消息。
- 不采用 gRPC / tonic：其基于 HTTP/2 与请求-响应模型，与本项目 UDP + 帧同步广播不匹配。
- 命令数值字段使用定宽 `int64` / `uint64` / `uint32` / `bool`，**禁止 `float` / `double`**：`SimFixed` 以 `raw()`（i64）传输，[ADR 0002](0002-determinism.md) / [ADR 0004](0004-frozen-coordinates.md) 禁止 `f64` 进入权威状态与冻结边界。

### 3. 为确定性必须附加的约束

protobuf 默认的「灵活性」在此场景是风险，实施时 MUST 全部落实：

- **禁用 `map` 字段**（迭代顺序不确定）；集合一律 `repeated` + 显式顺序。
- **字段编号只追加、不复用、不改名**；删除字段保留 `reserved`，避免旧 tag 被重新解释。
- **禁止 unknown fields 透传 / 回显**：解码后的未知字段不得再序列化进 relay 的命令流或日志，避免不同版本客户端产生不同字节。
- **锁定 `prost` / `prost-build` 版本**并写入 `Cargo.lock`；代码生成结果在 CI 的 Windows / Linux 双平台必须字节一致。
- **禁止非规范 varint 编码**；必要时对编码结果做「解码→再编码」canonical 往返校验。
- **proto schema 变更必须同步 bump `PROTOCOL_VERSION`**，配合现有 `Reject` 明确拒绝版本不匹配；「客户端与 relay 同步发布」的策略本 ADR 予以保留，前后兼容只用于**平滑升级与工具链**，不用于承诺旧客户端长期混跑。
- 帧编排（`FrameScheduler` / `FrameAssembler`）、`stabilize` 稳定排序、`apply_frame` 依序提交使 `arrival` 一致、状态哈希、快照语义、断线重连游标**全部不变**。

### 4. 不变的红线

- 网络**只传输玩家输入命令，MUST NOT 传输游戏状态**（`lockstep-networking` spec）。
- `w3-core` 零改动、零依赖，不反向依赖 `w3-net` / `w3-relay`；表现层不反写模拟状态。
- 资产与法律边界（[ADR 0001](0001-license.md)）：导出资产不入库；新增依赖按 CODEBUDDY.md §6 检查许可证、`NOTICE.md` 登记、`Cargo.lock` 提交。

## 备选方案（Alternatives considered）

| 方案 | 结论 | 理由 |
|---|---|---|
| 保持现状：全程手写 `W3N1` | 否决 | 无 schema 演进与跨语言能力；字段增减需同步改编解码两端，维护风险随包类型增长。 |
| **自有 `W3N1` 帧头 + protobuf payload（折中）** | **采纳** | 保留 lockstep 帧语义与确定性边界，仅把业务载荷交给 schema-first 框架；改动面最小、收益最大。 |
| 整体替换为 protobuf（连帧头一起） | 否决 | 丢失帧号 / sequence / 版本 / 校验的显式字节语义，与 ADR 0002 契约冲突，且可靠层与帧编排需重写。 |
| serde + bincode / postcard | 否决 | Rust 专用、无跨语言、schema 演进弱；无法达成引入 protobuf 的动因。 |
| gRPC / tonic | 否决 | HTTP/2 请求-响应模型与 UDP 帧同步广播不匹配。 |
| FlatBuffers / Cap'n Proto | 暂缓 | 零拷贝优势在「每帧少量小命令包」上不明显，生态与构建成本更高；需要时另立 ADR。 |

## 后果（Consequences）

**正面**：schema-first + 代码生成，减少手写编解码 bug；字段级前后兼容；跨语言客户端 / 观战 / 回放分析 / 独立 relay 工具成为可能；锁步帧语义仍由本项目完全掌控。

**负面 / 需明确接受**：

- 打破 P4「零新 crates.io 依赖 / `cargo build --offline` 全绿」的现状。`prost` 为 Apache-2.0，与本项目 GPLv3 兼容，但须按 CODEBUDDY.md §6 完成许可证检查、`NOTICE.md` 登记与 `Cargo.lock` 提交。
- 依赖获取是**现实前置条件**：本机 registry 缓存目前无 `prost` / `protobuf` 相关 crate，且 crates.io 镜像曾出现 SSL 握手失败；实施前必须先解决离线/联网依赖供给。
- 引入 `build.rs` + 代码生成，改变「无构建脚本」现状，CI 双平台（[CODEBUDDY.md](../../CODEBUDDY.md) §4）新增生成工具一致性门槛。
- 与 P4 归档 `design.md` 决策一的否决条冲突 —— 本 ADR 明确修订该条；P4 已归档，不做追溯改写，后续 change 的 design 须引用本 ADR 编号。
- 字节布局部分外包给库实现，包级日志 / 回放的字节稳定性不再由本项目单独保证，依赖第 3 节 canonical 约束；若约束被破坏，需按 ADR 0002 复核点重新评估。
- 第一版规模（双客户端、每帧少量命令）下体积与性能收益有限，本次决策主要面向未来演进，不声称解决 20,000 单位 / 30 人规模（[ADR 0003](0003-scope.md) 的 Non-Goals）。
- **本 ADR 仅记录决策，未实施任何代码**；在后续 change 落地前不得声称网络层已使用 protobuf。

## 复核点

- 实施时：新建独立 OpenSpec change（不得塞进已归档的 P4）；`openspec validate`；`cargo test --workspace` 与 `cargo test -p w3-core --test determinism` 全绿（w3-core 零改动）；双客户端逐帧哈希一致；注入丢包 / 乱序 / 断线回归；真实 UDP 双进程实跑；CI Windows / Linux 双平台 `verify_all.py`。
- 未来触发复核：proto schema 变更或 `prost` 升级、跨语言客户端接入、unknown fields 策略变化、引入加密 / 压缩 / 分片、提升规模目标、在非 Windows 平台首次实跑。
