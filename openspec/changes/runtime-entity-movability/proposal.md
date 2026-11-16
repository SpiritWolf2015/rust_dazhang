# Proposal

## Why

承接 P8.0 `p8-faction-foundation` 与 P8.1 `p8-human-faction` 的 `decision-memo.md` 均登记的已知限制 **B1**：运行时 `Train` / `Build` 动态实体不在装配期 `def_ids` 关联中，其可移动性在「`w3-core` 零改动 + 不扩展 `EntityView`」约束下无法判定，`input.rs::filter_movable` 对映射未登记实体 `unwrap_or(true)` 默认放行，会把运行时生产出的建筑也判为可移动，违反 `docs/references/WC3-MELEE-RULES.md` 的 **R1.4**（框选 / 编队与命令过滤：建筑 / 金矿 / 主城 MUST NOT 生成 `Move`）与 **R13.6**（快捷键 → 命令确定性契约，复用 R1.4 过滤）；`app.rs::ClientApp::new` 默认空映射、`examples/playable.rs` 未注入是 B1 的另一半。本 change 落在 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 个种族功能做完）的过程分期，补齐动态实体承接；**N1–N6 一律不纳入**。依据用户 2026-11-16 本会话指令「继续补齐动态实体承接」及紧接 `AskUserQuestion` 明确拍板的「**只读暴露 speed（推荐）**」方案：给只读 `EntityView` 加 `speed` 字段、投影内核已有权威 `UnitState.speed`，frame-0 用装配期映射、运行时动态实体用视图 speed 判据。

## What Changes

- **`crates/w3-core/`（首次动，放松 P8.0 / P8.1 实现层「零改动」约束；ADR 0002 确定性契约一条不放松）**：`sim_match.rs::EntityView` 加只读 `pub speed: i64`；`sim_match.rs::read_view` 投影**已存在**的 `UnitState.speed.raw()`。**不新增权威状态字段**、不改 `compute_state_hash` / `first_difference()` / 快照 / `SNAPSHOT_VERSION`（仍 3）/ 命令 / 事件 / 排序键；`cargo test -p w3-core --test determinism` 的 golden hash MUST 逐字节不变。论证：`EntityView` 是 `read_view()` 派生的只读视图、不进哈希 / 快照，`UnitState.speed` 早已存在且已在哈希 / 快照中，本次仅把它投影到只读视图。
- **表现层分层判据（统一为 speed 语义、两条来源不可互相替代）**：frame-0 装配实体 → 装配期 `speed_raw` 派生映射（P8.0 D1 不变，权威，因 `setup.rs::Builder::spawn` 把 frame-0 `speed` 抬到 `max(1)`、不可用视图 speed 区分）；运行时动态实体（不在装配映射里）→ 只读视图 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动（`sim_match.rs::complete_production` 已按 `def.speed_raw` 写入，建筑 `0`、单位 `> 0`）。
- **实现落点（最小改动、保护既有回归）**：`input.rs::filter_movable` 保持既有签名与对已登记实体的行为不变；新增合并函数把 `WorldView` 里「装配映射未登记」的实体按 `speed != 0` 补进一张有效映射，在 `input.rs::InputState::apply` 的群体 `Move` 分支调用（`apply` 已持有 `view: &WorldView`，无需改签名）。`tests/movability.rs` 里 5 处直接调 `filter_movable` / `degenerate_filter` 的反向验证断言零改动仍成立。
- **反向验证（改坏验红，`CODEBUDDY.md` §5）**：新增回归 MUST 证明「去掉动态实体兜底、退回旧 `unwrap_or(true)` 默认放行」会让「运行时生产出的建筑不响应 `Move`」断言失败、正式实现通过；证据用测试内反例元断言或 A-B 对照（参照归档 P8.0 `tests/movability.rs::degenerate_filter` 先例）。仅「全绿」不算完成。
- **playable / 默认空映射（B1 另一半）**：合并函数兜底后，view 里所有实体（含默认态）都有 speed 判据、空映射不再等价「全部可移动」缺陷态；`examples/playable.rs`（场景 4 个可移动单位、无建筑）与 `app.rs::ClientApp::new` 默认态由 client-owner 评估并显式处置（补明确注入或如实登记兜底已覆盖），**不得**假装处理而实际没做。
- **BREAKING：无**。既有 frame-0 混合选中过滤、稳定 `EntityId` 落点、命令排序键、状态哈希、快照口径、`SNAPSHOT_VERSION`=3 均不变；零资产、零文件 IO，数值为合成原创 `[待核]`。

## Capabilities

### New Capabilities

- `runtime-entity-orders`: 运行时动态实体可移动性的**只读派生**——`EntityView` 只读投影已有权威 `UnitState.speed`、frame-0 装配映射与运行时视图 speed 的分层判据、默认空映射兜底，以及「不新增权威状态、golden hash / 哈希字段集合 / 快照头布局 / `SNAPSHOT_VERSION`=3 / 排序键逐字节不变」的 ADR 0002 零冲击边界。

### Modified Capabilities

- `unit-orders`: 增加一条 **ADDED Requirement**——把 R1.4 / R13.6 的群体 `Move` 可移动性过滤从 frame-0 装配实体扩展覆盖到运行时 `Train` / `Build` 产出的动态实体（建筑 `speed == 0` 不生成 `Move`、战斗单位 `speed > 0` 正常生成；frame-0 混合选中回归不退化；反向验证改坏验红）；既有「命令校验 / 确定性移动积分 / 同帧攻击结算 / 群体移动只作用于可移动单位」四条 requirement 正文不改写。

## Impact

- **代码面**：`crates/w3-core/src/sim_match.rs` 仅 `EntityView` 加只读 `pub speed: i64` + `read_view` 投影 `UnitState.speed.raw()`；`crates/w3-client/src/input.rs` 新增合并函数 + `InputState::apply` 群体 `Move` 分支接线；`crates/w3-client/src/app.rs` / `examples/playable.rs` 默认态处置（client-owner 回报）；`EntityView` 的全部构造点（`hud.rs` / `tests/interaction.rs` / `tests/movability.rs` / `w3-render/src/model.rs` / `w3-render/src/scene.rs` 等）须同步补字段。`w3-net` / `w3-relay` / `Cargo.lock` 零改动。
- **确定性契约（ADR 0002）**：**只加只读视图字段、零哈希 / 快照影响**。`UnitState.speed` 早已存在且已在状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照中（`sim_match.rs::capture_snapshot` 的 `PositionRecord.speed`），`EntityView` 是 `read_view()` 派生的只读视图、MUST NOT 进哈希 / 快照；不改 `compute_state_hash` / `first_difference()` / 命令 / 事件 / `ExecuteFrame → PlayerId → Sequence → Arrival` 排序键 / 稳定 `EntityId` 顺序。`cargo test -p w3-core --test determinism` 全绿且 golden hash 逐字节比对不变，`git diff -- crates/w3-core/` 范围核验限于视图字段与投影。不得据本 change 声称跨平台确定性有新验证（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产、零 MPQ 派生数据、零真实导出 JSON、零文件 IO；全部数值为合成 / 原创 `[待核]`、不入库、不冒充原版。四个参考项目只读、不复制代码；`WarsmashModEngine`（AGPLv3）仅借鉴「类型 / 可移动性随实体走 + `speed == 0`」语义、不引入 Java / LibGDX 依赖，见 `design.md`「## 参考项目对照」。
- **ADR 与阶段**：相关 ADR 0001（资产）/ 0002（确定性）/ 0003（范围，1.0 已修订）/ 0004（冻结输入）/ 0005（canonical 约束，本 change 不涉网络传输）。阶段 **P8.2（B1 承接 change）**，紧随 P8.1 `p8-human-faction`；`openspec/config.yaml` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 / P8.1 一致）。落在 S1 / S3 的过程分期；S2 / S4 / S5 / S6 不在本 change 交付但属 1.0 范围。
- **与并行 change 正交**：P8.1 已归档、`p8-human-faction` 仍 active 且规格锁定，本 change **不碰它们的四件套正文**。`WC3-MELEE-RULES.md`（活文档）R1.4 / R13.6 与主 `openspec/specs/unit-orders/spec.md` 既有 requirement 的「MUST NOT 扩展 `EntityView` / 用运行时 `speed` / `w3-core` 零改动」是 **P8.0 / P8.1 阶段的实现层字面禁令**：本 change 经用户 2026-11-16 拍板首次放松该「实现层」约束（ADR 0002 零冲击不变），活文档由并行 owner 按 `WC3-MELEE-RULES.md` §14 流程同步修订，`docs/references/` 不在本 change 可写范围，本 change 不冒充已解决、如实登记于 `decision-memo.md` 待确认项。
- **人工边界**：规格门已由用户 2026-11-16 本会话指令 + `AskUserQuestion` 方案拍板批准（`decision-memo.md` `status: approved`，与 P8.1「用户指令作为批准」处理一致）；`critic` 代码门、真窗口人工验收待后续复核 / 执行、如实标注；本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀），push / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。
