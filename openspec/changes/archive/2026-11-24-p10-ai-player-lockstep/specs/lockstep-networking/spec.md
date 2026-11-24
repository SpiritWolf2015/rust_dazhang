# Spec Delta

## MODIFIED Requirements

### Requirement: Transmit only input commands

联机 SHALL 只传输真人玩家输入命令、MUST NOT 传输游戏状态；每个客户端 SHALL 运行相同模拟代码并按相同帧序执行相同命令。AI 席位的决策 MUST NOT 作为独立命令注入联机 `CommandBatch` 或经 relay 广播——AI 决策在 `w3-core` 内核侧由既有 `run_ai` 各端确定性重算、其决策状态为权威状态（`AiState`）并进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 8）定义的快照；各端以相同开局冻结定义 / 相同外部命令流构造 `RtsMatch`，`ClientSession::submit_input` MUST NOT 本地静默 `submit_gameplay_command` 注入 AI 命令（R12.2 / R12.7 / §0.3 T5 / ADR 0002 §5 / ADR 0005 §4）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 change 只修订本条 Requirement，`lockstep-networking` 其余 8 条 Requirement（含历史「`SNAPSHOT_VERSION` 仍 3」口径残留）按 P9 Q9' 已拍板登记为已知限制、由 `docs-owner` 后续独立 change 收口。

#### Scenario: 相同命令流

- **WHEN** 两个客户端收到相同的冻结真人命令帧、AI 席位启用内核 `run_ai` 各端重算
- **THEN** 逐帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`；`CommandBatch` 只含真人玩家命令、无 AI 决策命令、无本地静默注入（R12.2 / R12.7 / R12.4 / ADR 0002 §5 / ADR 0005 §4）

#### Scenario: AI 决策不进帧流（R12.7 / 被否台账 R1 / R2）

- **WHEN** 尝试把 `AiScript` 外部命令经 relay 广播注入联机帧流，或启用「双模式可切换」使两套命令源并存
- **THEN** 联机 session 只保留内核 `run_ai` 单一决策源、该路径显式失败或不被接受、不产生双份 AI 命令、两端 `CommandBatch` 一致、逐帧哈希一致不被破坏（P2 已拍板 / R12.7 / ADR 0002）
