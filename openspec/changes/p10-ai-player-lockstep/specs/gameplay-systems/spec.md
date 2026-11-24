# Spec Delta

## MODIFIED Requirements

### Requirement: Deterministic AI command source

玩家与玩家 AI SHALL 经相同命令校验入口；内部 AI 命令在完整重放中 MUST NOT 被再次注入；单位自动索敌 MUST NOT 覆盖仍有效的玩家命令。联机形态下 AI 席位 SHALL 由 `w3-core` 既有 `run_ai` 唯一决策、各端确定性重算，AI 决策状态（`AiState`）为权威状态并进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`；`run_ai` 内部命令 MUST NOT 写回命令队列、MUST NOT 注入重放；联机 `CommandBatch` 只承载真人玩家命令（R12.7 / R12.2 / §0.3 T5 / ADR 0002 §3 / §4 / §6-8）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 change 只修订本条 Requirement，`gameplay-systems` 其余 3 条 Requirement 不变。

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部真人玩家命令并在各端重新运行确定性 `run_ai`（AI 席位启用）、捕获 v8 快照后恢复续跑
- **THEN** 逐帧状态哈希与原始联机对局一致、`first_difference()` 为 `None`、`run_ai` 内部 `Gather` 命令不写回命令队列、不在重放中二次注入、AI 决策按 `next_decision_frame` 整数帧周期续算不重复（`sim_match.rs:3231-3261` / `gameplay.rs:2028-2031` / R12.4 / R12.7 / ADR 0002 §3 / §4 / §6-8）

#### Scenario: 联机各端 AI 决策状态一致（R12.7 / ADR 0002 §5）

- **WHEN** 两个联机端点以相同开局冻结定义构造 `RtsMatch`、各端 `run_ai` 对空闲单位发 `Gather` 并由 `SetAi{enabled:true}` 启用
- **THEN** 两端 `AiState`（`enabled` / `next_decision_frame` / `decision_period` / `pending_tasks`）逐帧一致、进状态哈希与 v8 快照、无墙钟 / `f64` / `HashMap` 迭代顺序 / 独立随机流、无本地静默 `submit_gameplay_command`（R12.7 / R12.2 / ADR 0002 §3-§8 / C3 / C4）
