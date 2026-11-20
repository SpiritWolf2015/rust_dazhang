# faction-definitions Specification

## Purpose
为两族完整化提供定义层地基：冻结定义的 `DefId` 按种族分段、开局按种族装配，且种族选择不引入新的运行时权威状态，守住 ADR 0002 确定性契约。

## Requirements

### Requirement: 冻结定义按种族分段

`FrozenDefinitions` 中 **`units` 类别**的 `DefId` SHALL 按种族分段（人族段、兽族段、中立 / 资源段），同段内唯一、跨段不冲突；分段只约束 `units` 类别，`abilities` / `upgrades` / `items` 三个类别的既有类别内 ID（`10` / `20` / `30`）沿用各自类别内唯一规则、不纳入种族分段。每段定义仍是编译期整数常量，内容哈希 MUST 进入快照头部（ADR 0002）。分段 MUST NOT 引入 `f64`、MUST NOT 读取 `config.toml` 或任何导出资产（ADR 0001 / ADR 0004）。

#### Scenario: 分段唯一且可复现

- **WHEN** 构造含人族段与兽族段的 `FrozenDefinitions`
- **THEN** `units` 类别各 `DefId` 全局唯一、构造可重复、`defs_hash` 固定，且与既有合成定义的 6 个 `UnitDef` 的 DefId 不冲突；三个非 units 类别的 ID 仍各自类别内唯一

#### Scenario: 零资产构造

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造分段定义
- **THEN** 构造成功且不做任何文件 IO

### Requirement: 开局按种族装配且不新增权威状态

`MeleeSetup` SHALL 依据种族参数生成对应 `DefId` 段的 `Spawn` 开局命令；玩家选族 MUST NOT 以新的运行时权威状态字段表达，未指定种族时 SHALL 回落人族段（R2 / R12.7 / ADR 0002）。选族装配本身不新增权威状态；P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A 新增「主城等级 / tier」顶层权威状态，MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，tier 需求链冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash`；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3 或 4 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 两族分别装配

- **WHEN** 分别以人族与兽族参数构造开局
- **THEN** 双方 `Spawn` 命令的 `DefId` 落在各自段内，资源与人口初值按既有冻结常量确定，选族装配不引入 `race` 权威状态字段（R2）

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的选族装配改动，以及 P8.5b 主城升级的 `w3-core` 改动
- **THEN** 选族装配不引入 `race` 权威状态字段；P8.5b 的 `w3-core` 改动 MUST 把新增 tier 顶层权威状态 / 升级队列 / 费用登记同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」、golden hash 基线按新实现如实更新，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体、`git diff -- crates/w3-core/` 非空属预期，MUST NOT 沿用「`git diff -- crates/w3-core/` 为空 / 仍为 3 或 4 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 回落兼容

- **WHEN** 既有 `playable` 合成入口不传种族参数
- **THEN** 行为与改动前一致，一局仍能装配并打完，且主城升级在 `w3-core` 确定性命令面可用、逐帧哈希与 P8.5b 新实现基线一致（R12.7 / R4.1）
