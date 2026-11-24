# Spec Delta

## MODIFIED Requirements

### Requirement: State hashing and snapshot

系统 SHALL 用显式小端字节编码与稳定 `EntityId` 顺序计算全状态哈希；快照 SHALL 是帧 N 提交完成后的完整状态，含 RNG 流、下一 ID 计数器、已接受但未执行的命令、单位订单与导航状态、技能施法与冷却、Buff 实例、库存与人口、生产/建造队列、科技进度、**AI 状态（`AiState`）与待执行定时任务**；头部 SHALL 含冻结定义的内容哈希；版本、`sim_version` 或内容哈希不匹配 MUST 被拒绝。联机形态下 AI 席位决策状态（`ai.enabled` / `next_decision_frame` / `decision_period` / `pending_tasks`）由各端 `run_ai` 确定性重算、 MUST 进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`；本期零新增权威状态字段为规格门 Q1 推荐口径、保持 8 不 bump（P8.1 / P9 R19 先例），若 Q1 拍板确需新增字段 MUST 四处同步并按 ADR 0002 bump 8→9 全套（ADR 0002 §6-8 / R12.4 / R12.7）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 change 只修订本条 Requirement，`deterministic-simulation` 其余 3 条 Requirement 不变。

#### Scenario: 双实例一致性

- **WHEN** 两个联机端点喂相同真人命令帧流并交错推进、AI 席位各端由 `run_ai` 重算
- **THEN** 每帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`，`AiState` 逐帧一致、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 30 Hz 整数帧 / 稳定 `EntityId` 顺序 / FNV-1a 不变（R12.2 / R12.4 / R12.7 / ADR 0002 §2-§8）

#### Scenario: 快照往返

- **WHEN** 在帧 N 捕获 v8 快照（含 `AiState`）、销毁实例、恢复后跑到帧 M，并以同一命令序列回放
- **THEN** 与未中断直接跑到帧 M 的实例逐帧哈希一致、`run_ai` 按 `next_decision_frame` 整数帧周期续算、不重复决策、每帧只消费一次主 LCG（R12.4 / ADR 0002 §4 / §6-8 / Q5）

#### Scenario: 恢复不重复施放

- **WHEN** 从含有进行中施法与 Buff、且 AI 席位 `run_ai` 已登记的 v8 快照恢复
- **THEN** 不重新扣费、不重新触发出生副作用、不再次施放技能、不重复注入内部 AI 命令、AI 决策状态与原始对局逐帧一致（R12.4 / R12.7 / ADR 0002 §5 / §7）

#### Scenario: 零新增权威状态字段与反向验证（ADR 0002 §8 / Q1 / CODEBUDDY.md §5）

- **WHEN** 按规格门 Q1 结果若确需新增权威状态字段却漏进 `compute_state_hash` / `Snapshot` 顶层 / `first_difference()` / `restore_snapshot` 之一；或零新增口径下误沿用惯性改动版本号 / golden hash
- **THEN** `cargo test -p w3-core --test determinism`（26 项）对应反例断言失败阻止合并；零新增时 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`、golden hash `0x0b53a047b1f8e5e0` 保持、不声称逐字节变化（ADR 0002 §6-8 / `CODEBUDDY.md §5` 形态① / Q1 / C5）
