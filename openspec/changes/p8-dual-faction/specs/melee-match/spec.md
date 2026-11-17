# Spec Delta

## ADDED Requirements

### Requirement: 四组合双族对局的会话装配与逐帧哈希一致（R2 / R11 / R12.4 / R12.7）

一局 1v1 Melee 会话 SHALL 支持以 `(local_race, ai_race)` 独立双族参数装配 2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）并各打完一局，AI 对手命令 SHALL 按 AI 自身种族产出（R2 / R12.7）。四组合的逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 以「**同一组合两实例一致**」验收，`resolve_victory` MUST 只结算一次、重开 MUST 无残留（R11.1 / R11.4 / R11.5 / R12.4 / ADR 0002）。既有本 capability 8 条 requirement 不改写；双族参数只停装配层、MUST NOT 新增权威状态字段（主「内核零改动与确定性契约」requirement 继续有效）。本 Requirement 只追加、对称于本会话的「一局 Melee 对战可装配并打完」与「确定性 AI 对手」。

#### Scenario: 四组合会话装配并打完一局（R11.1 / R11.4 / R12.7）

- **WHEN** 四种组合各以双族参数装配会话、注入完整一局脚本推进到敌方主城被毁、触发 `Leave`
- **THEN** 四组合均产生唯一胜负结果且只结算一次、按稳定顺序，AI 对手的训练 / 施法 / 研发命令落在 `ai_race` 段，会话全程只经 `w3-core` 既有公开命令面（R11.1 / R11.4 / R12.7 / 主「确定性 AI 对手」）

#### Scenario: 组内逐帧哈希一致且既有两族基线不破坏（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本逐帧推进，并复核既有两族基线 / 回落测试
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等（不只比最终 hash）；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` 仍绿、不静默改其断言；MUST NOT 断言四组合之间或与旧基线哈希数值相等（R12.4 / ADR 0002 / C4）

#### Scenario: 重开干净与快照版本不变（R11.5 / ADR 0002）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重开，并校验 `SNAPSHOT_VERSION`
- **THEN** 新一局 `victory.resolved == false`、视图为空、不残留上一局权威状态，`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变、`git diff -- crates/w3-core/` 为空（R11.5 / ADR 0002 §6-8 / C1）

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5）

- **WHEN** 以退化实现（AI 命令退回本地族 / 四组合装配退化为单参数 `opponent()` / `with_race` 兼容别名退化）替换正式实现
- **THEN** 对应段位 / 基线断言在缺陷态失败、修复态通过，两条证据可复现（CODEBUDDY.md §5 / C5）
