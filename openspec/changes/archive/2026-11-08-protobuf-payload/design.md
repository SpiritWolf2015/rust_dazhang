## Context

P4 已归档（`2026-11-08-p4-lockstep-networking`），网络层使用自研 `W3N1` 线格式（`crates/w3-net/src/packet.rs`）：固定 25B 头 + 显式小端 + FNV-1a 校验，payload 由手写 `Buf` / `Reader` 定长编解码全部 `CommandKind` 变体，`w3-net` 零外部序列化依赖、可 `cargo build --offline`。

[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md) 已冻结决策：采用「自有 `W3N1` 帧头 + protobuf 载荷」的折中方案，并修订 P4 `design.md` 决策一「否决通用序列化框架」一条。本 change 是该 ADR 的实施载体；动机见 `proposal.md`，系统行为见 `specs/lockstep-networking/spec.md`。本 design 只回答「怎么做」。

关键事实：逐帧 checksum 比对的是 `compute_state_hash()`（模拟权威状态哈希）而非网络包字节，protobuf 编码不确定性**不会直接造成 desync**。风险在 [ADR 0002](../../../docs/decisions/0002-determinism.md) 的显式字节编码契约，以及包级 replay / 审计 / FNV-1a 校验 / 版本拒绝所依赖的「字节布局冻结」前提。

## Goals / Non-Goals

**Goals:** 保持 lockstep 帧语义与确定性边界不变的前提下，把 payload 层换成 schema-first 的 protobuf 编解码；获得字段级前后兼容与跨语言能力；落实 ADR 0005 §3 的 canonical 约束。

**Non-Goals:** 不改动 `w3-core` 权威状态、帧编排、排序键、状态哈希与快照；不做像素级 / 体积优化承诺；不承诺旧客户端长期混跑；不引入 gRPC / 服务发现；不解决 20,000 单位 / 30 人规模（[ADR 0003](../../../docs/decisions/0003-scope.md) 的 Non-Goals）。

## Decisions

### 决策一：保留 `W3N1` 帧头，仅替换 payload（ADR 0005 §1）

- **结论**：25B 帧头（`MAGIC` / `version` / `kind` / `sender` / `sequence` / `frame` / `payload_len` / `checksum`，显式小端）与 `Packet::encode` / `decode` 的头处理逻辑完全不变；只把 `encode_commands` / `decode_commands` 与 `Welcome` / `Ack` / `Checksum` / `ResyncRequest` / `ResyncReplay` / `Reject` 的 payload 换成 protobuf 消息；`payload_len` 与 FNV-1a 覆盖编码后的字节。
- **考虑过的备选方案**：
  - 整体替换为 protobuf（连帧头一起）：否决，丢失帧号 / sequence / 版本 / 校验的显式字节语义，与 ADR 0002 契约冲突，可靠层与帧编排需重写。
  - 保持全程手写 `W3N1`：否决，无 schema 演进与跨语言能力，字段增减需同步改两端。
  - serde + bincode / postcard：否决，Rust 专用、无跨语言、schema 演进弱。
  - FlatBuffers / Cap'n Proto：暂缓，零拷贝优势在每帧少量小命令包上不明显，构建成本更高。
- **对跨平台确定性的影响（ADR 0002）**：帧头字节布局仍由本项目冻结，大小端 / 字长平台一致；命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序不变，`SimFixed` Q47.16、30 Hz 整数帧与 FNV-1a 状态哈希不变。ADR 0002 的跨平台确定性目前仅 Windows / rustc 1.91 实跑，Linux / macOS 仍为待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：DIP / ISP——`Transport` trait 与 `RtsMatch` 命令提交边界不变，`sim/` 不读 socket、`w3-core` 不反向依赖 `w3-net`；SRP——`packet.rs`（线格式）与 `reliable.rs` / `frame.rs` 职责不变，仅替换 payload 编解码实现；唯一状态所有者仍是 `RtsMatch`。

### 决策二：`prost` + 代码生成（ADR 0005 §2）

- **结论**：采用 `prost` / `prost-build` 纯 Rust 生成 `#[derive(Message)]` 结构体；`.proto` 定义 `CommandBatch`、`Command`（`oneof` 承载 7 个 `CommandKind` 变体）与各控制包 payload 消息；`build.rs` 在 `w3-net` 内生成代码。
- **考虑过的备选方案**：
  - gRPC / tonic：否决，HTTP/2 请求-响应模型与 UDP 帧同步广播不匹配。
  - 手写 `Message` 实现、不引入 build.rs：否决，放弃代码生成的维护收益，与 ADR 0005 动因冲突。
  - 运行时反射式 protobuf 库：否决，体积与复杂度更高，无必要。
- **对跨平台确定性的影响（ADR 0002）**：命令数值字段用定宽 `int64` / `uint64` / `uint32` / `bool`，禁用 `float` / `double`；`SimFixed` 以 `raw()`（i64）传输，[ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md) 的 `f64` 边界不被突破。锁定 `prost` 版本并要求 Windows / Linux 双平台生成结果字节一致，避免库版本差异影响编码。
- **对确定性契约与分层依赖的影响**：冻结输入边界不变；`w3-core` 零改动。显式失败——解码失败 / 未知字段 / 校验和不匹配返回 `NetError`，不静默降级（CODEBUDDY.md 硬红线）。

### 决策三：canonical 编码约束（ADR 0005 §3）

- **结论**：禁用 `map`（集合用 `repeated` + 显式顺序）、字段编号只追加不复用不改名且删除保留 `reserved`、禁止 unknown fields 透传 / 回显、锁定 `prost` 版本写入 `Cargo.lock`、禁止非规范 varint（必要时做「解码→再编码」canonical 往返校验）、schema 变更同步 bump `PROTOCOL_VERSION`。
- **考虑过的备选方案**：
  - 依赖 protobuf 默认行为、不加约束：否决，map 迭代顺序 / unknown fields / 库版本差异会让「同一命令 → 不同字节」，破坏包级 replay / 审计与 FNV-1a 校验语义。
  - 只对业务命令加约束、控制包放宽：否决，控制包同样进入命令流与日志，约束必须一致。
- **对跨平台确定性的影响（ADR 0002）**：把「字节布局外包给库」的缺口用约束补回，使同一逻辑命令在任何平台 / 库版本下产生相同字节；不新增权威状态字段，状态哈希与快照 v1 字段集合不变。
- **对确定性契约与分层依赖的影响**：LoD / CARP——约束集中在 `.proto` 定义与 `packet.rs` 的编解码入口，不散落到 `reliable.rs` / `frame.rs` / `session.rs`；唯一状态所有者仍是 `RtsMatch`。

### 决策四：版本与同步发布策略

- **结论**：保留「客户端与 relay 同步发布、`Reject` 明确拒绝版本不匹配」；schema 变更 MUST bump `PROTOCOL_VERSION`；protobuf 的前后兼容只用于平滑升级与工具链，不用于承诺旧客户端长期混跑。
- **考虑过的备选方案**：
  - 依赖 protobuf 兼容性长期混跑旧客户端：否决，proposal 与 ADR 0005 已明确不承诺；且 unknown fields 策略会使字节不可控。
  - 双实现并存 + 运行期协商：否决，属于 backwards-compatibility shim，违反 CODEBUDDY.md 「避免过度工程 / 不加兼容垫片」。
- **对跨平台确定性的影响（ADR 0002）**：版本边界显式、失败可见，不含隐式降级；帧序不受版本协商影响。
- **对确定性契约与分层依赖的影响**：唯一状态所有者不变；显式失败红线得到保持。

## Risks / Trade-offs

- [依赖获取是现实前置条件] → 本机 registry 缓存暂无 `prost` / `protobuf` 相关 crate，crates.io 镜像曾 SSL 握手失败；实施前先解决离线 / 联网供给，再锁定版本并检查 Apache-2.0 与 GPLv3 兼容性、登记 `NOTICE.md`、提交 `Cargo.lock`（CODEBUDDY.md §6）。
- [打破零依赖 / 离线可编译现状] → 接受；`build.rs` + 代码生成纳入 CI，Windows / Linux 双平台 `verify_all.py` 必须生成一致。
- [字节布局部分外包给库实现] → 依赖决策三的 canonical 约束；若约束被破坏，按 ADR 0002 复核点重新评估，包级日志 / 回放的字节稳定性不再由本项目单独保证。
- [与 P4 归档 `design.md` 决策一冲突] → 由 ADR 0005 明确修订，本 change 引用 ADR 0005 编号；P4 已归档，不做追溯改写。
- [第一版规模收益有限] → 双客户端、每帧少量命令下体积 / 性能收益有限，本次主要面向未来演进（ADR 0003 Non-Goals）。
- [BREAKING 线格式变更] → 客户端与 relay 同步发布；回滚策略是整体回退到上一发布版本组合，不提供跨版本兼容层。

## Migration Plan

1. 解决 `prost` / `prost-build` 依赖供给并锁定版本；完成许可证检查、`NOTICE.md` 登记、`Cargo.lock` 提交。
2. 新增 `.proto` 与 `build.rs`，确认双平台生成结果一致；先补齐编解码单测，再一次性替换 payload 层，避免手写 / protobuf 双实现长期并存。
3. 保持帧头与 `Packet` 头逻辑不变，替换 `encode_commands` / `decode_commands` 及控制包 payload；落实 canonical 约束。
4. 回归：编解码 roundtrip、`cargo test -p w3-core --test determinism`（w3-core 零改动基线）、双客户端逐帧哈希一致、丢包 / 乱序 / 断线注入、真实 UDP 双进程实跑、CI 双平台门禁。
5. 同步发布客户端与 relay；文档撤销「未使用 protobuf」表述。

## Open Questions

无待决设计问题：ADR 0005 已冻结全部路线决策。实施期需确认的 `prost` 具体版本与 protoc 供给方式属于环境前置（tasks 1.1 / 1.3），不构成设计未决项。
