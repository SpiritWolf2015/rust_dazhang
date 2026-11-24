# Spec Delta

## MODIFIED Requirements

### Requirement: 确定性 AI 对手

系统 SHALL 为联机对局的 AI 对手席位提供确定性命令源：AI 席位决策在 `w3-core` 内核侧由既有 `run_ai` 各端确定性重算、复用既有 `SetAi` / `AiState` 命令面（决策状态为权威状态、进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 8）定义的快照），联机 `CommandBatch` 只承载真人玩家命令、MUST NOT 注入外部 `AiScript` 命令（R12.7 / R12.2 / §0.3 T5）。AI 命令 MUST NOT 覆盖仍有效的玩家命令，完整重放 MUST NOT 双重注入内部 AI 命令。本地 `melee` 窗口 `AiScript`（`SetAi{enabled:false}`）保留为开发夹具、不列 1.0 验收线（N1 / Q3）。本期 `run_ai` 仅空闲单位采集语义，AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）登记为后续 change 承接（P3 已拍板 / R12.7）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 change 只修订本条 Requirement，`melee-match` 其余 8 条 Requirement（含历史「`SNAPSHOT_VERSION` 仍 3 / 内核零改动」口径残留）按 P9 Q9' 已拍板登记为已知限制、由 `docs-owner` 后续独立 change 收口。

#### Scenario: 对手会行动

- **WHEN** 联机 1v1 一局推进若干逻辑帧且未分胜负、AI 席位由内核 `run_ai` 各端重算
- **THEN** 对手空闲单位按整数帧周期产生采集行为（`Gather`）、两端可见相同的状态变化且逐帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`；本期不要求 AI 建造 / 训练 / 进攻 / 结算等全量行为（R12.7 / R12.4 / ADR 0002 §3 / §4 / P3 已拍板 / `sim_match.rs:3231-3261`）

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部真人玩家命令并在各端重新运行确定性 `run_ai`、不注入 `AiScript` 外部命令 / `Leave`
- **THEN** 逐帧状态哈希与原始联机对局一致、`run_ai` 内部命令不写回命令队列、不在重放中二次注入（R12.4 / R12.7 / ADR 0002 §3 / §4 / P2 已拍板）

#### Scenario: 联机一局打完的胜负判据可执行（R11.1 / R11.4 / R12.4 / R11.5）

- **WHEN** 联机 1v1 各端以相同冻结开局定义构造 `RtsMatch`（真人席位 + 启用 `run_ai` 的 AI 席位），注入冻结真人命令流摧毁 AI 席位全部建筑、逐帧推进到建筑全毁，并回放同一命令序列
- **THEN** 经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为真人玩家、只结算一次且按稳定顺序（R11.1 / R11.4）；两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`、回放逐帧哈希一致（R12.4）；重开后 `victory.resolved == false` 且视图为空（R11.5）；胜负不依赖 `AiScript` 的 `Leave`（R11.3 属本期不定的 C 类边界）（P2 / P3 已拍板 / Q4 / §0.3 T5 / ADR 0002 §5）
