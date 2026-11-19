# Spec Delta

## MODIFIED Requirements

### Requirement: 内核零改动与确定性契约

本 requirement 标题沿用 P6 历史契约名，自 P8.5a 起口径按用户 2026-11-19 拍板修订：本 capability 已交付的历史行为 SHALL 只经 `w3-core` 既有公开 API 与命令实现；P8.5a 英雄复活 change MUST 按 ADR 0002 全流程改 `w3-core` 真复活，新增死亡登记 / 复活队列 / 费用等权威状态 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`（R12.4 / ADR 0002 §6-8）。`GameplayCommandKind` 按 Q2 拍板新增独立 `ReviveHero` / `CancelRevive` 变体；determinism 26 项全绿、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（`CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10`「相关 ADR」写法与本 change `design.md` Context「MODIFIED delta 的身份键说明」）。

#### Scenario: 内核零 diff

- **WHEN** 校验 P8.5a 英雄复活对 `crates/w3-core/` 的全部改动
- **THEN** `git diff -- crates/w3-core/` 非空属预期，新增权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」反向验证，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4，MUST NOT 沿用「零 diff / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 哈希与快照字节不变

- **WHEN** 跑完一局并与 P8.5a 新实现的 CPU 基线比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与新实现基线一致，golden hash 基线按新实现如实更新、不沿用旧值，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），MUST NOT 沿用「仍为 3 / 逐字节不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 四组合双族对局的会话装配与逐帧哈希一致（R2 / R11 / R12.4 / R12.7）

一局 1v1 Melee 会话 SHALL 支持 `(local_race, ai_race)` 双族参数装配 2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）并各打完一局，AI 命令 SHALL 按 AI 自身种族产出（R2 / R12.7）。四组合逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 以「同一组合两实例一致」验收，`resolve_victory` MUST 只结算一次、重开 MUST 无残留（R11.1 / R11.4 / R11.5 / R12.4 / ADR 0002）。双族参数只停装配层；P8.5a 复活 change 改 `w3-core` 后，四组合基线 MUST 按新实现重取、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），`git diff -- crates/w3-core/` 非空属预期，MUST NOT 断言与 P8.5a 前旧基线逐字节相等、MUST NOT 沿用「仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10`「相关 ADR」写法与本 change `design.md` Context「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合会话装配并打完一局（R11.1 / R11.4 / R12.7）

- **WHEN** 四种组合各以双族参数装配会话、注入完整一局脚本推进到敌方主城被毁、触发 `Leave`
- **THEN** 四组合均产生唯一胜负结果且只结算一次、按稳定顺序，AI 对手的训练 / 施法 / 研发命令落在 `ai_race` 段，会话全程只经 `w3-core` 既有公开命令面（R11.1 / R11.4 / R12.7 / 主「确定性 AI 对手」）

#### Scenario: 组内逐帧哈希一致且既有两族基线不破坏（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本逐帧推进，并复核既有两族基线 / 回落测试
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等（不只比最终 hash）；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` 仍绿、不静默改其断言；MUST NOT 断言四组合之间或与 P8.5a 前旧基线哈希数值相等（R12.4 / ADR 0002 / C4）

#### Scenario: 重开干净与快照版本不变（R11.5 / ADR 0002）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重开，并校验 `SNAPSHOT_VERSION`
- **THEN** 新一局 `victory.resolved == false`、视图为空、不残留上一局权威状态与复活登记；`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）、golden hash 基线按新实现如实更新，`git diff -- crates/w3-core/` 非空属 P8.5a 预期，MUST NOT 沿用「仍为 3 / 逐字节不变 / 为空」旧断言（R11.5 / R12.4 / ADR 0002 §6-8 / C1）

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5）

- **WHEN** 以退化实现（AI 命令退回本地族 / 四组合装配退化为单参数 `opponent()` / `with_race` 兼容别名退化 / 复活新增字段漏接哈希快照）替换正式实现
- **THEN** 对应段位 / 基线断言在缺陷态失败、修复态通过，两条证据可复现（`CODEBUDDY.md §5` / C5 / C27）
