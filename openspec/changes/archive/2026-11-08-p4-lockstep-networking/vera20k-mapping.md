# P4 与 vera20k 的设计对应关系及残差

## 对应关系

`vera20k`（GPLv3 + Section 7）的 `src/net/` 与 `services/relay-protocol`
提供生产级 lockstep / UDP 可靠层 / 认证数据报 / 排序时钟 / 自适应冗余 /
断线重连与回放设计。本项目**只借鉴架构，重写线格式**，未复制其代码
（ADR 0001、CODEBUDDY.md §1）。

| vera20k 设计 | 本项目落地 | 对应 spec / 任务 |
|---|---|---|
| lockstep：只传输入命令，各端跑相同模拟 | `FrameScheduler` 按帧缓存、`stabilize` 稳定排序、`apply_frame` 依序 `submit_command` 后恰好 `step()` 一帧 | Transmit only input commands；1.3 |
| UDP 可靠层：ACK / 重传 / 去重 | `ReliableSender`（按确定性逻辑 tick 重传）、`ReliableReceiver`（sequence 去重）、`Transport` 抽象；上行 Input 与下行 Frame **双向**都走可靠队列 | Reliable UDP delivery and ordering；1.1 |
| 排序时钟：帧边界关闭 | `FrameAssembler` 收齐或超时关闭帧，缺席玩家视为空输入，不无限等待 | 缺失输入按既定帧边界处理；1.3 |
| relay 认证时钟 + 不可变命令流 | `w3-relay` 的 `Lobby`（稳定 `player_id`）、`CommandLog`（每帧只记录一次）、`Relay::tick` 广播 | 2.2 |
| 逐帧 checksum 与 desync 定位 | `ChecksumExchange` 交换比对，不一致返回 `NetError::Desync` 并保留 `DesyncReport`；`first_difference_field` 包装 `first_difference()` | Per-frame checksum comparison；2.1 |
| 断线重连 / 回放恢复 | `SessionCursor`（`next_frame` 游标）、`ReconnectController::plan_replay`（过滤已应用帧）、`ResyncReplay` 补发 | Reconnection and replay recovery；2.3 |
| Event 线格式 | **不复用** RA2 `FRAMEINFO` / `FRAMESYNC`，自定义 `W3N1`：固定 25B 头、显式小端、FNV-1a 校验、全 `CommandKind` 编码 | 1.2 |
| 自建服务 | `w3-relay` lib + bin，UDP 绑定地址可配；版本不匹配发 `Reject` 明确拒绝 | 2.2；design「自建服务」 |

## 本会话验证（自动化）

- `w3-net` / `w3-relay` 单元测试：线格式 roundtrip、真实 UDP loopback
  收发、丢包重传、乱序、断线、checksum desync、帧编排确定性等。
- 集成测试 `crates/w3-relay/tests/two_clients.rs`：两个独立 `RtsMatch`
  + `ClientSession` 经内存 relay 同图同命令流跑 24 帧，逐帧比对
  `compute_state_hash()`、`read_view()` 与双方 checksum 交换结果。
- 集成测试 `crates/w3-relay/tests/network_faults.rs`：注入上行 Input 丢包
  （重传补齐后仍在原定帧执行）、下行 Frame 丢包（下行重传补齐）、乱序帧
  （按帧号重排）、短暂断线（按输入游标追帧且不重复执行已提交命令）。
- `cargo fmt --all -- --check`、`cargo test --workspace`、
  `python tools/verify_all.py`、`openspec validate` 全部通过。

## 实机验收（3.1 已通过）

`tasks.md` 的 3.1「双客户端同图对战跑通并逐帧哈希一致」已勾选。可执行
入口为三个独立终端（relay 是常驻循环，不会自行退出，需保持运行）：

```bash
# 终端 1：relay
cargo run -p w3-relay -- 127.0.0.1:9000 2
# 终端 2：客户端 1
cargo run -p w3-net --example lockstep_client -- 127.0.0.1:9000 1 30
# 终端 3：客户端 2
cargo run -p w3-net --example lockstep_client -- 127.0.0.1:9000 2 30
```

`w3-net::init_logging()` 把英文结构化日志写到 stderr：每个客户端逐帧打印
`frame applied: frame=N hash=0x...`，结束时打印
`done: frames=30 last_hash=0x...`。

2026-11-08 两次真实 UDP 双进程实跑（本会话以独立 `CARGO_TARGET_DIR` +
端口 9100；用户按上述三终端命令在端口 9000 复跑）：两端各 30 帧逐帧 hash
完全一致，最终 `last_hash` 一致（分别为 `0x909369450b20f177` 与
`0xb97ecae4ddc5b7b8`）。**hash 绝对值取决于两个客户端的启动时序与命令提交
节奏，验收判据是两端逐帧与最终完全一致，而非某个固定值。**

实跑共暴露并修复四个问题：

1. relay 在 lobby 未收齐玩家前就推进帧时钟，后加入的客户端从帧 0 起会死等
   → `Relay::tick()` 在 `lobby.is_started()` 前不推进，并加回归测试。
2. `log` 后端未初始化导致验收静默 → `w3-net::logging` 提供极简 stderr 后端。
3. Windows UDP 在对端 socket 关闭后 `recv_from` 返回 `WSAECONNRESET(10054)`，
   relay 误判为致命错误而退出 → `UdpTransport::recv` 与 `WouldBlock` 一并
   吞掉该瞬时错误。
4. lobby 未就绪期间客户端每轮 `request_resync` 刷屏 → example 改为连续缺帧
   只请求一次（边沿触发）。

## 已知残差

- **线格式不兼容**：`W3N1` 与 RA2 `FRAMEINFO` / `FRAMESYNC`、gamemd/IPX、
  官方战网均不兼容（design 的 Non-Goals）。客户端与 relay 必须同步发布，
  版本不匹配明确拒绝，不承诺旧客户端兼容。
- **无压缩缓存**：线格式未做 Event 编解码压缩 / 字典缓存；第一版以正确性
  优先，命令量小，压缩留待后续（对应 design 的 Open Question 之一）。
- **无自适应调速**：未引入 relay 自适应冗余 / 调速（对应 design 的 Open
  Question 之二）；第一版沿用「降到最慢玩家」——`FrameAssembler` 在固定
  `timeout_ticks` 后按空输入关闭帧，保证所有客户端收到相同权威帧命令集。
- **relay / 客户端节拍**：`w3-relay/src/main.rs` 与 `lockstep_client.rs`
  的 `sleep(33ms)` 只用于避免忙等、节流网络轮询；帧号来自
  `FrameAssembler` 的逻辑计数，客户端只按收到的帧包推进本地模拟，
  墙钟不进入 `compute_state_hash()` / 快照。`reliable.rs` 的「不读墙钟」
  特指重传由逻辑 tick 驱动。宿主负载会改变帧分组的实时速率，但不改变
  帧序与内容，因此不影响 ADR 0002 的确定性结论。
- **唯一提交路径未被类型强制**：`ClientSession` 不持有 `RtsMatch`，
  app 层若旁路 `apply_frame` 直接 `submit_command` 会单端推进
  `next_arrival` 而 desync；「只经 `apply_frame` 提交」目前是分层约定，
  未由类型系统强制，留待后续冻结。
- **无预测回滚 / 反作弊 / 认证数据报加密**：均在设计 Non-Goals 内。
- **规模未验证**：第一版以双客户端与小规模验收为准；30 人 / 20,000 单位
  目标未验证（design 的 Risks）。
- **日志后端**：`w3-net::logging` 提供极简 stderr 后端，由 `w3-relay`
  bin 与 `lockstep_client` example 调用 `init_logging()` 装配，验收输出
  默认可见；当前固定输出 Info 及以上级别，未实现 `RUST_LOG` 级别过滤，
  且 P3 的 `render_frame` example 仍未初始化后端（与本 change 无关）。
- **跨平台确定性**：模拟内核跨平台确定性仍仅 Windows / rustc 1.91 实跑
  （ADR 0002），Linux / macOS 属待验证项；本 change 不声称已达成。

## 资产与法律

本 change 未引入新的 crates.io 依赖（传输层使用 `std::net`），未复制
`vera20k` 代码；导出资产与密钥仍留在本机，绝不入库（ADR 0001）。
`NOTICE.md` 的「第三方 crate 依赖（P3 w3-render）」一节所列依赖同样覆盖
本 change 使用的 `log` / `thiserror`，无需新增条目。

> **后续演进注记（2026-11-08）**：本文件记录的是 P4 归档时的状态，不追溯改写。ADR 0005 之后，独立 change `protobuf-payload` 已把 payload 层改用 Protocol Buffers（`prost`），新增 crates.io 依赖并打破「零新依赖 / 离线可编译」；`W3N1` 帧头保持不变。当前依赖登记见 `NOTICE.md` 的「第三方 crate 依赖（P4 / protobuf-payload）」一节。
