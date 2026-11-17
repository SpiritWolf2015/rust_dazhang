# Spec Delta

## ADDED Requirements

### Requirement: 键盘 M / A 路径复用 R1.4 分层可移动性过滤

键盘 `M` → `Move` / `A` → `Attack` 生成命令时 SHALL 与 `R1.3` 右键路径走同一输入翻译与命令生成处（`input.rs::InputState::apply`），MUST 由 `input.rs::effective_movability` 实体来源分层判据过滤：frame-0 装配实体用装配期 `UnitDef.speed_raw` 只读映射（权威），运行时动态实体用 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动，两条 MUST NOT 互替。金矿 / 树木 / 主城 / 建筑 MUST NOT 出现在键盘 `Move` 生成集合、MUST NOT 生成 `Move`、MUST NOT 改变位置；落点按稳定 `EntityId` 顺序、过滤后集合与「只含可移动单位」逐条相同（`R1.4` / `R13.6` / §14）。只读派生数据 MUST NOT 进权威状态 / 状态哈希 / `SNAPSHOT_VERSION`（3）快照（ADR 0002）。

#### Scenario: frame-0 混合选中集合键盘 M 排除建筑与资源点（R1.4 / R13.6）

- **WHEN** frame-0 装配的农民 + 主城 + 金矿混合选中集合按 `M` 并指定地面落点
- **THEN** 只为农民生成 `Move`；主城（建筑）与金矿（资源点）MUST NOT 出现在 `Move` 生成集合中、位置不变；落点分配与「只含这些农民」时逐条相同、按稳定 `EntityId` 顺序（R1.4 / P8.0 回归不退化）

#### Scenario: 框选含建筑时键盘 M 只为可移动单位生成 Move（R1.4 / R13.6）

- **WHEN** 左键框选同时选中单位与建筑 / 资源点后按 `M` 指定地面落点
- **THEN** 框选允许建筑进入选中集合，但命令生成集合 MUST 只含可移动单位；任何建筑 / 金矿 / 树木 MUST NOT 出现在 `Move` 生成集合中、位置不变（R1.4）

#### Scenario: 运行时动态实体建筑键盘 M 不响应 Move（R1.4 / R13.6）

- **WHEN** 运行时 `Train` / `Build` 产出的建筑（`EntityView.speed == 0`）与农民一起框选并对地面按 `M`
- **THEN** 该运行时建筑 MUST NOT 出现在 `Move` 生成集合中、位置不变；产出的战斗单位（`speed > 0`）与农民正常生成 `Move`（R13.6 / §14）

#### Scenario: 全为不可移动实体键盘 M 不生成 Move（R1.4 / R13.6）

- **WHEN** 选中集合只含建筑 / 资源点并按 `M` 指定地面落点
- **THEN** 不生成任何 `Move` 命令，权威状态与状态哈希不变、有明确的「无可执行目标」反馈位（表现层只读、不进哈希 / 快照）

#### Scenario: 键盘 A 生成既有 Attack 且复用同一过滤（R13.2 / R13.6）

- **WHEN** 混合选中集合按 `A` 指定敌方目标
- **THEN** 生成既有 `CommandKind::Attack { attacker, target }`、与右键敌方目标路径走同一 `apply`；不出现 attack-move 点地面途中交战语义（`[待核]`、后续 change 承接），不新增命令变体（R13.2 / R13.6）

#### Scenario: 状态哈希 / 快照 / SNAPSHOT_VERSION 不变（ADR 0002 / R13.6）

- **WHEN** 键盘 `M` / `A` 路径接线后重跑 `cargo test -p w3-core --test determinism`（26 项）并与基线比对
- **THEN** golden hash 逐字节不变、`SNAPSHOT_VERSION` 仍 3，状态哈希字段集合、快照头部布局（`defs_hash` / `content_hash`）、`first_difference()`、命令 / 事件、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键均不变；`git diff -- crates/w3-core/` 为空（ADR 0002 §6-8）

#### Scenario: 分层判据边界与只读派生（R1.2 / R1.4 / R13.6 / ADR 0002）

- **WHEN** 校验键盘 `M` / `A` 路径的分层判据实现与改动面
- **THEN** MUST NOT 扩展 `WorldView` / `EntityView`（已是只读视图字段）、MUST NOT 用运行时实体 `speed` 替代 frame-0 装配映射、MUST NOT 回退建筑类型白名单（`R1.2`）；frame-0 与运行时两条判据 MUST NOT 互相替代（`R1.4` / §14）；只读派生数据 MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照，`crates/w3-core/` 零 diff（ADR 0002 / `R13.6`）

#### Scenario: 反向验证（改坏验红）（CODEBUDDY.md §5 / R1.4 / R13.6）

- **WHEN** 以退化键盘路径（不过滤、把建筑当可移动、或 frame-0 误用视图 `speed`）替换正式实现，对含建筑 / 金矿 / 主城的混合选中集合按 `M`
- **THEN** 「不可移动实体 MUST NOT 出现在 `Move` 生成集合」断言在缺陷态失败、修复态通过，两条证据均可复现（C5）
