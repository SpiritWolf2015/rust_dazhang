# Spec Delta

## MODIFIED Requirements

### Requirement: 开局按种族装配且不新增权威状态

`MeleeSetup` SHALL 依据种族参数生成对应 `DefId` 段的 `Spawn` 开局命令；玩家选族 MUST NOT 以新的运行时权威状态字段表达，未指定种族时 SHALL 回落人族段、既有入口行为不变（R2 / R12.7 / ADR 0002）。选族装配本身不新增权威状态；P8.5a 英雄复活 change 按用户 2026-11-19 拍板改 `w3-core` 真复活，新增死亡登记 / 复活队列 / 费用等权威状态 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，golden hash 基线按新实现如实更新，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`GameplayCommandKind` 按 Q2 拍板新增独立 `ReviveHero` / `CancelRevive` 变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10`「相关 ADR」写法与本 change `design.md` Context「MODIFIED delta 的身份键说明」）。

#### Scenario: 两族分别装配

- **WHEN** 分别以人族与兽族参数构造开局
- **THEN** 双方 `Spawn` 命令的 `DefId` 落在各自段内，资源与人口初值按既有冻结常量确定（R2）

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的选族装配改动，以及 P8.5a 英雄复活的 `w3-core` 改动
- **THEN** 选族装配不引入 `race` 权威状态字段；P8.5a 的 `w3-core` 改动 MUST 把新增权威状态同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、golden hash 基线按新实现如实更新，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），`git diff -- crates/w3-core/` 非空属预期，MUST NOT 沿用「`git diff -- crates/w3-core/` 为空 / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 回落兼容

- **WHEN** 既有 `playable` 合成入口不传种族参数
- **THEN** 行为与改动前一致，一局仍能装配并打完（R12.7）
