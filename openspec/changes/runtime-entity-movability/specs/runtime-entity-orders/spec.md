# Spec Delta

## Purpose

定义运行时 `Train` / `Build` 动态实体可移动性的只读派生契约：从内核已有权威 `UnitState.speed` 投影只读视图字段，按实体来源分层判定，使表现层在不新增权威状态、不改变状态哈希 / 快照 / `SNAPSHOT_VERSION` 的前提下，能对装配期映射未登记的动态实体正确过滤 `Move` 命令。合规边界见 ADR 0002 / `CODEBUDDY.md` §5。

> 相关 ADR：ADR 0002（确定性契约）、ADR 0001（资产与法律）。玩法规则基线：`docs/references/WC3-MELEE-RULES.md` R1.4 / R13.6 / §14。

## ADDED Requirements

### Requirement: 只读实体视图投影已有权威 speed

`sim_match.rs::EntityView` SHALL 暴露只读 `pub speed: i64`，由 `sim_match.rs::read_view` 投影**已存在**的 `UnitState.speed.raw()`；该字段 MUST 是 `read_view()` 派生的只读视图值，MUST NOT 新增权威状态字段，MUST NOT 进入 `compute_state_hash` / `first_difference()` / 快照。系统 MUST NOT 改动 `SNAPSHOT_VERSION`（当前 3）、命令、事件、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键；`cargo test -p w3-core --test determinism` 的 golden hash MUST 逐字节不变（ADR 0002 / `CODEBUDDY.md` §5）。

#### Scenario: read_view 投影 UnitState.speed.raw

- **WHEN** 对含运行时生产实体（建筑 `speed == 0`、战斗单位 `speed > 0`）的 `RtsMatch` 调用 `read_view`
- **THEN** 每个 `EntityView.speed` 等于对应 `UnitState.speed.raw()`，且 `EntityView` 其余字段、`WorldView` 结构与该视图「修改不影响模拟」的只读语义不变（`sim_match.rs::read_view` / ADR 0002 §5）

#### Scenario: 状态哈希、快照与 SNAPSHOT_VERSION 不变

- **WHEN** 加入 `EntityView.speed` 字段与其全部构造点后重跑 `cargo test -p w3-core --test determinism` 并比对 golden hash / `compute_state_hash()` / `capture_snapshot()` 字节
- **THEN** golden hash 逐字节不变，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合、快照头部布局（`defs_hash` / `content_hash`）、`first_difference()`、命令排序键均不变，且 `git diff -- crates/w3-core/` 只含只读视图字段与投影（ADR 0002 §6-8 / `CODEBUDDY.md` §5）

### Requirement: 动态实体可移动性的分层只读派生

系统 SHALL 按实体来源分层判定可移动性：frame-0 装配实体由装配期 `FrozenDefinitions::units()` 的 `UnitDef.speed_raw` 派生映射判定（P8.0 D1，权威）；装配映射未登记的运行时动态实体由只读 `EntityView.speed` 判定（`== 0` 不可移动、`> 0` 可移动）。两条来源 MUST NOT 互相替代，判定 MUST 在表现层输入翻译层完成、MUST NOT 反写模拟状态（R1.4 / R13.6 / ADR 0002）。

#### Scenario: 运行时 Train/Build 产出的建筑不响应 Move

- **WHEN** 运行时经 `GameplayCommandKind::Train` / `Build` 产出建筑（`sim_match.rs::complete_production` 写入 `speed == 0`、不在装配期 `def_ids` 关联中），将其与可移动单位一起框选并对地面右键或按 `M` 下达移动命令
- **THEN** 该建筑 MUST NOT 出现在 `Move` 命令生成集合中、位置不变，只有可移动单位生成 `Move`，落点按稳定 `EntityId` 顺序分配（R1.4 / R13.6）

#### Scenario: 运行时产出的战斗单位正常响应 Move

- **WHEN** 运行时 `Train` 产出战斗单位（`speed > 0`、不在装配期映射中）并框选下达移动命令
- **THEN** 该单位 SHALL 正常生成 `Move` 命令、功能不因「映射未登记」被拦截，逐帧状态哈希与等效脚本一致（R1.4 / R13.6 / ADR 0002）

#### Scenario: frame-0 混合选中仍只为可移动单位生成 Move

- **WHEN** frame-0 装配的农民 + 主城 + 金矿混合选中集合对地面右键或按 `M` 下达移动命令（P8.0 既有回归）
- **THEN** 只为农民生成 `Move`，主城（建筑）与金矿（资源点）MUST NOT 出现在 `Move` 生成集合中、位置不变，过滤后集合与「只含可移动单位」的落点逐条相同（R1.4 / R13.6 / P8.0 回归不退化）

#### Scenario: frame-0 speed 被抬过、视图 speed 不可替代装配映射

- **WHEN** `setup.rs::Builder::spawn` 因 `CommandQueue::submit` 的 `speed.raw() <= 0 → InvalidSpawn` 校验而把 frame-0 实体 `speed` 强制为 `max(1)`，其运行时视图 `speed > 0` 无法区分建筑
- **THEN** 系统 MUST 以装配期 `UnitDef.speed_raw` 派生映射为 frame-0 唯一权威判据，MUST NOT 以 `EntityView.speed` 取代它；运行时动态实体 MUST NOT 回查装配期映射（`command.rs::CommandKind::Spawn` / `command.rs::CommandQueue::submit` / `setup.rs::Builder::spawn`）

### Requirement: 默认空映射经合并函数兜底

`input.rs::InputState::apply` 的群体 `Move` 分支 SHALL 在调用 `filter_movable` 前，经新增合并函数把 `WorldView` 中「装配映射未登记」的实体按 `EntityView.speed != 0` 补进有效映射；`filter_movable` 的签名与对已登记实体的行为 MUST NOT 改变（`apply` 已持有 `view: &WorldView`，MUST NOT 改其签名）。`app.rs::ClientApp::new` 默认 `MovabilityMap::new()` 空映射与 `examples/playable.rs` 未注入映射的场景，MUST NOT 再等价「全部可移动」缺陷态（R1.4 / R13.6 / ADR 0002）。

#### Scenario: 默认空映射不再等价缺陷态

- **WHEN** `ClientApp::new` 默认空映射（或 `examples/playable.rs` 未注入）下，`view` 含运行时生产建筑且其 `EntityView.speed == 0`
- **THEN** 合并函数产出有效映射，该建筑被判不可移动、MUST NOT 生成 `Move`，效果与「显式注入正确映射」一致；空映射不再触发 `unwrap_or(true)` 全放行（R1.4 / R13.6 / `input.rs::filter_movable`）

#### Scenario: filter_movable 既有行为与签名零改动

- **WHEN** 运行 `tests/movability.rs` 里直接调用 `filter_movable` 与 `degenerate_filter` 的既有反向验证断言
- **THEN** 全部断言零改动仍成立；缺陷态（空 / 退化映射下 frame-0 建筑因 `max(1)` 的 `speed > 0` 被误判可移动）仍令断言失败、正式实现令断言通过（`input.rs::filter_movable` / `CODEBUDDY.md` §5）

### Requirement: 动态实体兜底的反向验证

新增回归 MUST 证明「去掉动态实体兜底、退回旧 `unwrap_or(true)` 默认放行」会让「运行时生产出的建筑不响应 `Move`」断言**失败**、正式实现**通过**；证据形态为测试内反例元断言或 A-B 对照。仅「全绿」MUST NOT 视为完成（`CODEBUDDY.md` §5 / `openspec/config.yaml` `rules.tasks` 第三条）。

#### Scenario: 改坏验红

- **WHEN** 以退化实现（合并函数缺席、映射未登记实体回到 `unwrap_or(true)` 默认放行）替换正式实现并运行同一「运行时生产建筑不响应 `Move`」断言
- **THEN** 退化实现下断言失败、正式实现下断言通过，两条证据均可复现，且断言经真实 `input.rs::InputState::apply` 群体 `Move` 路径触发而非旁路模拟（R1.4 / R13.6 / `CODEBUDDY.md` §5）
