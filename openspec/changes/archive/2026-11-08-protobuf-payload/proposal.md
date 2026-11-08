## Why

P4 的 `W3N1` 线格式由手写 `Buf` / `Reader` 定长编解码，字段增减需同步改两端、无 schema 演进能力，也无法支撑跨语言的观战 / 回放分析 / 独立机器人客户端。[ADR 0005](../../../docs/decisions/0005-protobuf-payload.md) 已冻结决策：采用 protobuf 载荷 + 自有 `W3N1` 帧头的折中方案，并修订 P4 `design.md` 决策一「否决通用序列化框架」一条。本 change 是该 ADR 的实施载体，属 **P4 后续网络层演进**（依附 [ADR 0003](../../../docs/decisions/0003-scope.md) 的 P4 联机阶段，不新设 P 阶段，不改变 P5 玩法范围）。

## What Changes

- 保留 `W3N1` 固定 25B 帧头（`MAGIC` / `version` / `kind` / `sender` / `sequence` / `frame` / `payload_len` / `checksum`，显式小端）及其字节布局不变；`payload_len` 与 FNV-1a 校验继续覆盖 protobuf 编码后的完整 payload。
- **仅把 payload 层**换成 Protocol Buffers：`encode_commands` / `decode_commands` 与 `Welcome` / `Ack` / `Checksum` / `ResyncRequest` / `ResyncReplay` / `Reject` 的 payload 构造与解析改用 `prost` 生成的消息；新增 `.proto` schema（`CommandBatch`、`Command` + `oneof` `CommandKind` 及控制包消息）。
- 落实 [ADR 0005](../../../docs/decisions/0005-protobuf-payload.md) §3 的确定性编码约束：禁用 `map`、字段编号只追加不复用、禁 unknown fields 透传、锁定 `prost` 版本、禁非规范 varint、禁 `float` / `double`（`SimFixed` 用 `raw()` i64）、schema 变更同步 bump `PROTOCOL_VERSION`。
- **BREAKING**：网络线格式的 payload 编码改变，客户端与 relay 须同步发布；版本不匹配继续由现有 `Reject` 明确拒绝，不承诺旧客户端混跑。
- 新增 `prost` / `prost-build` 依赖与 `build.rs` 代码生成；帧编排、稳定排序、状态哈希、快照与断线重连语义不变。

## Capabilities

### New Capabilities

（无。）

### Modified Capabilities

- `lockstep-networking`: 新增「自有帧头 + protobuf 载荷」「确定性 protobuf 编码约束」「模拟确定性边界不变」三条 requirements；现有四条 requirements 的行为语义不变。

## Impact

- **确定性（[ADR 0002](../../../docs/decisions/0002-determinism.md)）**：逐帧 checksum 比对的仍是 `compute_state_hash()` 而非包字节，protobuf 不会直接造成 desync；但 ADR 0002 的显式字节编码契约、包级 replay / 审计与 FNV-1a 校验依赖「字节布局冻结」，须由 canonical 约束补回。`w3-core` 零改动、不新增权威状态字段，状态哈希与快照 v1 字段集合不变；跨平台确定性仍仅 Windows / rustc 1.91 实跑，Linux / macOS 为待验证项，本 change 不声称已达成。
- **资产与法律（[ADR 0001](../../../docs/decisions/0001-license.md)）**：`prost` 为 Apache-2.0，与 GPLv3 兼容；按 CODEBUDDY.md §6 完成许可证检查、`NOTICE.md` 登记与 `Cargo.lock` 提交。导出资产边界不变。
- **代码 / 依赖 / 构建**：`crates/w3-net/src/packet.rs`、`crates/w3-net/Cargo.toml`、`crates/w3-relay` 的控制包解析；打破「零新 crates.io 依赖 / 离线可编译」现状，引入代码生成后 CI Windows / Linux 双平台须生成一致。
- **前置条件**：本机 registry 缓存暂无 `prost` / `protobuf` 相关 crate，且 crates.io 镜像曾 SSL 握手失败；依赖供给须先解决，否则无法编译。
