## 1. 前置决策与规范基线（8 项已由用户拍板、均采纳推荐；阻塞解除）

- [x] 1.1 拍板 (a) schema 扩展方式与 tag 段规划——**已拍板，采纳 D1 建议 + 源码校正**：独立 `GameplayCommand` 消息（oneof tag 4–13 覆盖 10 变体 + `reserved 14 to 32`）+ `GameplayCommandBatch` + 在**顶层 `CommandBatch` 追加 tag 9**（而非 `FrameCommands`），不动 `Command` 的 `reserved 11 to 16`；理由已核实：玩法命令变体无法映射回 `CommandKind` 穷尽 match，且 6 个保留位不够 10 个变体（design.md D1）
- [x] 1.2 拍板 (b) 命令流组织——**已拍板，采纳 D2**：同一帧批 + 固定 `type_tag`（`Simulation=0` / `Gameplay=1`）统一稳定键 + 恰好一次 `step()` 前全部提交；否决独立批 / 双 `step()`；`Command` 与 `GameplayCommand` 是两条独立队列、独立 `arrival`，统一键只用于规范化归并顺序、非跨类型共享 arrival（design.md D2）
- [x] 1.3 拍板 (c) `PROTOCOL_VERSION` 兼容策略——**已拍板，采纳 D3**：2→3 硬 bump + `VersionMismatch` / `Reject` 明确拒绝 v2 + 客户端与 relay 同步发布、无兼容垫片；核心理由：v2 端把玩法命令字段当 unknown field 静默丢弃 → 两端命令流不等 → desync 不可见，违反「显式失败不静默降级」（design.md D3）
- [x] 1.4 拍板 (d) 归档顺序与 P6「`PROTOCOL_VERSION` 不变」Scenario 处理——**已拍板，采纳 D6 / D7 选项 A**：P6 先归档、本 change 后归档；P6 的 change spec 该 Scenario 保持原样、不追溯改写；待 P6 归档后由主 `openspec/specs/lockstep-networking/spec.md` 修订措辞；**执行载体选定 A(ii)**（归档后由 `spec-steward` 一次性一致性维护直接修订主 spec、不走 change delta），与 (h)「不产生 `lockstep-networking` delta」不互斥；A(i)/B 会产 delta（互斥）、A(iii) 需新 change、C 需改 P6 spec 实质，均否决（design.md D6 / D7）
- [x] 1.5 拍板 (e) owner 归属——**已拍板，新建 `net-owner`**：已指定 `net-owner`（单一 owner，覆盖 `crates/w3-net/` / `crates/w3-relay/`），`w3-client` 侧由 `client-owner` 配合，配只读 `critic`；中文 agent 定义由编排者新建（不在 `spec-steward` 可写范围，本 change 不修改 `.codebuddy/agents/`），满足 CODEBUDDY.md §8「一个 change 一个 owner」（design.md D5）
- [x] 1.6 拍板 (f) 路线图阶段标注——**已拍板，采纳建议**：不新增阶段、标注为「P6 集成收尾的后续补齐」，`config.yaml` rules.proposal 的「P0-P5」枚举保持不动（design.md Open Questions (f)）
- [x] 1.7 拍板 (g) 是否追加 ADR 0005 修订记录——**已拍板，采纳 D4，本轮已执行**：在 `docs/decisions/0005-protobuf-payload.md` **只追加**一条「修订 / 复核」记录（新增 `## 修订记录` 段：字段编号段扩展、`PROTOCOL_VERSION` 2→3、明确拒绝旧版本、canonical 约束继续适用、实施状态以本 change 的 tasks 为准），不改原决策正文、不改既有段落（design.md D4）
- [x] 1.8 拍板 (h) spec delta 落点——**已拍板，采纳 D8**：新增独立 `gameplay-command-wire` capability、不产生 `lockstep-networking` delta；`proposal.md` Capabilities 与 `specs/` 布局不变；与 (d) 选项 A 不互斥的执行载体为 A(ii)（design.md D8）
- [x] 1.9 复核 `openspec validate gameplay-command-wire-format` 与 `--strict` 通过、`python tools/check_encoding.py` 与 `git diff --check` 通过（本轮规范侧基线，已复核通过）

## 2. owner 定义与 P6 归档前置（编排者执行，本轮不做）

- [ ] 2.1 **由编排者执行**：按 1.5 已拍板结论新建中文 `net-owner` agent 定义，声明可写 `crates/w3-net/` / `crates/w3-relay/`（及 `w3-client` 的 net 适配切分），继承 `permissions.deny`；`w3-client` 侧由 `client-owner` 配合，配只读 `critic`（不在 `spec-steward` 可写范围）
- [ ] 2.2 先执行 `openspec archive p6-playable-client -y`，确认主 `openspec/specs/` 已吸收 P6 的 `playable-client` / `3d-rendering` / `lockstep-networking` delta；P6 的「`PROTOCOL_VERSION` 不变」Scenario 保持原样、不追溯改写（(d) 已拍板「P6 先归档、本 change 后归档」，design.md D6）。**按 §9 串联，须在人工 push + CI 双平台通过后执行，本轮不提前归档**（P6 刚 commit、未 push / CI，归档是 §9 串联的末端动作）
- [ ] 2.3 按 1.4 已拍板结论（选项 A / 执行载体 **A(ii)**）处理主 `lockstep-networking/spec.md` 中 P6 遗留的「`PROTOCOL_VERSION` 不变」Scenario 措辞：P6 归档后由 `spec-steward` 以一次性一致性维护**直接修订主 spec**、**不走 change delta、不产生 `lockstep-networking` delta**（与 (h) 不互斥），不追溯改写已实施完成 change 的 spec 实质（design.md D7）。**按 §9 串联，须在人工 push + CI 双平台通过后执行，本轮不提前归档、不改主 specs**

## 3. `w3net.proto` 扩展（只追加，ADR 0005 §3）

- [ ] 3.1 新增 `GameplayCommand` 消息：`execute_frame=1` / `player_id=2` / `sequence=3`，`oneof kind` 用 tag 4–13 覆盖 `Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `RegisterBehavior` / `SpawnWave` / `SetAi` / `Leave` 共 10 变体，并写 `reserved 14 to 32`
- [ ] 3.2 为 10 个玩法变体各定义消息，字段全用定宽 `int64` / `uint64` / `uint32` / `bool`（`EntityId`→`uint64`、`DefId`→`uint32`、`EffectDef.amount`→`int64`、`EffectKind`→稳定码 `uint32`、`SimFixed`→`raw()` `int64`）；每消息显式 `reserved`，MUST NOT 出现 `float` / `double` / `map`
- [ ] 3.3 新增 `GameplayCommandBatch { repeated GameplayCommand commands = 1; reserved 2 to 8; }`
- [ ] 3.4 在既有 `CommandBatch` 的 `reserved 2 to 8` 之后追加 `GameplayCommandBatch gameplay = 9;`；确认 `Command` 的 `oneof` tag 4–10、`reserved 11 to 16`、`FrameCommands` 的 `reserved 3 to 6`、`ResyncReplay` 与全部控制包的字段编号 / `reserved` 零改动
- [ ] 3.5 确认 `build.rs` 无需改动（`prost-build` 仍编译同一 `w3net.proto`），`prost` / `prost-build` 版本锁定且 `Cargo.lock` 不变；`cargo:rerun-if-changed` 已覆盖 schema

## 4. `PROTOCOL_VERSION` bump 与版本拒绝（BREAKING）

- [ ] 4.1 在 `crates/w3-net/src/packet.rs` 把 `PROTOCOL_VERSION: u8 = 2` 改为 `3`，更新版本注释（追加「3 = 玩法命令线格式，ADR 0005 §3」），`MAGIC` / `HEADER_LEN = 25` / `encode` / `decode` 头逻辑与 FNV-1a 覆盖范围不变
- [ ] 4.2 确认 `Packet::decode` 对 `version != 3` 返回 `NetError::VersionMismatch`、relay 侧以 `Reject` 控制包明确拒绝，MUST NOT 按新 schema 解码旧版 payload、不保留 v2 解码垫片
- [ ] 4.3 更新 `packet.rs` 测试中 `bytes[4] == 2` 断言与版本相关用例；更新 `w3-relay/src/lib.rs` 测试对 `PROTOCOL_VERSION` 的 import 断言
- [ ] 4.4 补一条「version=2 包被明确拒绝、玩法命令字段不被静默当作 unknown field」回归测试（对应 spec「旧版本包被明确拒绝」Scenario）

## 5. `payload.rs` 双向编解码与 canonical 约束

- [ ] 5.1 新增 `to_pb_gameplay_kind` / `from_pb_gameplay_kind` 的**双向穷尽**映射，覆盖 10 个 `GameplayCommandKind` 变体；`to_pb_kind` / `from_pb_kind`（针对 `CommandKind`）保持原样、不改 `CommandKind`
- [ ] 5.2 新增玩法命令批的 `to_pb_gameplay_batch` / `from_pb_gameplay_batch`；`EffectKind` 用稳定码 `uint32` 并对未知码显式返回 `NetError::Malformed`，`EffectDef` 的 `i64` / `u32` 逐字段映射
- [ ] 5.3 扩展 `FrameCommands`（Rust）为携带 `commands: Vec<Command>` + `gameplay: Vec<GameplayCommand>`（BREAKING），`arrival` 仍不编码、由各端提交时本地分配
- [ ] 5.4 扩展 `encode_commands` / `decode_commands` 同时编解码两类命令；`MAX_PAYLOAD` 对两类命令条数均作上限校验；解码失败 / 缺 kind / 溢出显式返回 `NetError`，不静默降级
- [ ] 5.5 扩展 `payload_resync_replay` / `parse_resync_replay`，使每帧 `FrameCommands` 同时补发两类命令且顺序与原始权威帧批一致
- [ ] 5.6 补 canonical 往返测试：同批命令两次编码逐字节一致、解码→再编码一致、未知字段丢弃且不回显、10 变体逐个往返、无浮点 / 无 `map`（对应 `schema_forbids_map_and_floating_point` 测试扩展）；验证 Windows / Linux 双平台生成字节一致

## 6. `frame.rs` / `session.rs` / `w3-relay` 命令流（一次 `step()` 前全部提交）

- [ ] 6.1 在 `w3-net/src/frame.rs` 引入统一 `FrameInput` 归并视图与固定 `type_tag`（`Simulation=0` / `Gameplay=1`），按 `(execute_frame, player_id, sequence, type_tag)` 稳定排序；扩展 `stabilize` 使其对两类命令生效且不改 `Command` 现有排序键语义
- [ ] 6.2 扩展 `apply_frame` 接收同帧批两类命令：归并序内普通命令经 `submit_command`、玩法命令经 `submit_gameplay_command`（同源 `validate_gameplay_command`）**全部提交**后，**恰好一次**无参 `sim.step()`，返回 `compute_state_hash()`；MUST NOT 双推进、MUST NOT 绕过校验
- [ ] 6.3 扩展 `FrameScheduler` / `FrameAssembler`：缓存 / 装配 / `take_ready` / `poll` 同时处理两类命令，到达顺序不决定执行顺序、缺帧超时语义、`arrival` 一致性不变
- [ ] 6.4 扩展 `w3-net/src/session.rs` 的 `submit_input` 同时上行两类命令、`pop_ready` 交出含两类命令的 `FrameCommands`；`ClientSession` 仍不持有 `RtsMatch`
- [ ] 6.5 扩展 `w3-relay/src/lib.rs`：`PacketKind::Input` 解码两类命令并 `assembler.submit`；`broadcast_frame` 编码两类命令；`CommandLog` 存含两类命令的 `FrameCommands`；`ResyncReplay` 补发路径核对
- [ ] 6.6 补帧编排单测：统一稳定键排序、一次 `step()` 前全部提交、玩法命令 `FrameInPast` / `InvalidEntity` 显式拒绝、断线重连不重复执行（对应 spec「一次 step 前全部提交」「玩法命令同源校验」Scenario）

## 7. `w3-client` 联机通路（去掉 `GameplayUnsupportedOnline`）

- [ ] 7.1 修改 `crates/w3-client/src/net_driver.rs::StepDriver::enqueue`：去掉 `ClientCommand::Gameplay(_)` 的 `GameplayUnsupportedOnline` 短路与 `reason=wire_format_command_only` 日志，改走 `ClientSession::submit_input` 上行广播；MUST NOT 本地静默 `submit_gameplay_command`
- [ ] 7.2 核对 `crates/w3-client/src/app.rs` 的 `ClientError` 变体与 `crates/w3-client/src/input.rs` 的 `ClientCommand` 上行路径；失败仍以英文结构化 `key=value` 日志明确报告并显式返回 `ClientError`（CODEBUDDY.md §3「显式失败不静默降级」）
- [ ] 7.3 核对 P6 的 `NetSessionDriver::consume_ready_frames` 唯一模拟推进点仍为 `w3_net::apply_frame`，累加器 / 网络 tick 不新增 `step()` 次数；本地与联机玩法命令行为一致
- [ ] 7.4 补客户端联机通路单测：施法 / 生产 / 取消生产在联机可用、本地与联机同一命令产生同一权威状态、上行失败显式返回错误

## 8. 测试、确定性与门禁

- [ ] 8.1 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism -- --nocapture`），确认 `w3-core` 零改动、状态哈希字段集合与 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` / `content_hash`）不变、不 bump 快照版本
- [ ] 8.2 补双客户端逐帧哈希一致用例：含 `Cast` / `Train` / `CancelTrain` 等玩法命令的权威帧批，逐帧 `compute_state_hash()` 相等且 `first_difference()` 返回 `None`、无 `Desync`
- [ ] 8.3 注入丢包 / 乱序 / 短暂断线回归（扩展 `w3-relay/tests/network_faults.rs`、`two_clients.rs`），确认玩法命令仍在原定 `ExecuteFrame` 执行一次、帧序与 `step()` 次数不变、追帧后哈希与健康客户端一致
- [ ] 8.4 `ResyncReplay` 回归：补发多帧同时含两类命令、顺序与原始权威帧批一致、恢复后逐帧哈希一致
- [ ] 8.5 `cargo test --workspace` 全绿（含 `w3-net` / `w3-relay` / `w3-client` / `w3-core` 全部单测）
- [ ] 8.6 真实 UDP 双进程实跑（**人工**）：两客户端联机触发施法 / 生产 / 取消生产，30 帧两端 `compute_state_hash()` 一致，确认无 `GameplayUnsupportedOnline`、无静默降级
- [ ] 8.7 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（写 Rust 的 owner 与 `critic` Review 前均复跑）
- [ ] 8.8 源码注释（含 `pub` rustdoc / 模块级 `//!` / 行内 `//`）一律中文，日志与错误信息一律英文结构化 `key=value`，无散落 `println!`（CODEBUDDY.md §3）
- [ ] 8.9 `python tools/check_encoding.py` 通过（UTF-8 无 BOM / LF / 文件末尾换行 / 无行尾空格）与 `git diff --check` 通过
- [ ] 8.10 `python tools/verify_all.py` 通过；CI Windows / Linux 双平台 `verify_all.py` 与 prost 生成字节一致门禁通过
- [ ] 8.11 依赖与许可证复核：确认本 change 未引入新依赖（复用既有 `prost` / `prost-build`）；若确需新增，按 CODEBUDDY.md §6 检查 GPLv3 兼容性、提交 `Cargo.lock`、由 `docs-owner` 在 `NOTICE.md` 登记（本 change 不自行编写 `NOTICE.md`）
- [ ] 8.12 资产合规复核（ADR 0001）：确认无 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效入库，冻结定义留玩家本机，线格式不含资产字节
- [ ] 8.13 只读 `critic` 复审：确定性 / 所有权 / 许可证 / 编码 / 资产入库 / rustfmt，复跑 `cargo fmt --all -- --check`，不通过不得提交
- [ ] 8.14 本地 commit（Conventional Commits 英文 type 前缀 + **中文描述首行 ≤ 72 字符**，一个改动一个 commit）；远端 `push` 由人工独占执行，AI 不代劳

## 9. 归档（后续，本轮不执行）

- [ ] 9.1 复跑 `openspec validate gameplay-command-wire-format` 与 `--strict`，确认全部 tasks 完成、四件套一致
- [ ] 9.2 由编排者执行 `openspec archive gameplay-command-wire-format -y`，确认主 `openspec/specs/` 新增 `gameplay-command-wire/spec.md`，既有 7 个主 spec 按 1.4 / D7 结论核对 `lockstep-networking` 口径
- [ ] 9.3 按 1.7 结论在 `docs/decisions/0005-protobuf-payload.md` 追加「修订 / 复核」记录并复核 `openspec validate --all`（仅追加段、不改原决策正文）
