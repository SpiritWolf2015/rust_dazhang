# Spec Delta

## ADDED Requirements

### Requirement: 群体移动过滤覆盖运行时动态实体

群体右键 / `M` 键 `Move` 命令生成 SHALL 把 R1.4 / R13.6 的可移动性过滤从 frame-0 装配实体扩展到运行时 `Train` / `Build` 产出的动态实体：装配期映射未登记的实体由只读视图 `EntityView.speed` 判定（`== 0` 不生成 `Move`、`> 0` 生成）。frame-0 建筑 / 金矿 / 主城与任何不可移动实体 MUST NOT 出现在 `Move` 生成集合中；既有 frame-0 过滤行为与落点分配 MUST NOT 退化（R1.4 / R13.6 / ADR 0002）。

#### Scenario: 运行时 Train/Build 产出的建筑 speed==0 不响应 Move

- **WHEN** 运行时经 `Train` / `Build` 产出建筑（`speed == 0`、不在装配期 `def_ids` 关联中），与农民一起框选并对地面右键或按 `M` 下达移动命令
- **THEN** 该建筑 MUST NOT 出现在 `Move` 命令生成集合中、位置不变，只为农民生成 `Move`，逐帧状态哈希与「只含可移动单位」的等效脚本一致（R1.4 / R13.6）

#### Scenario: 产出的战斗单位 speed>0 正常响应 Move

- **WHEN** 运行时 `Train` 产出战斗单位（`speed > 0`、不在装配期映射中）并框选下达移动命令
- **THEN** 该单位 SHALL 生成 `Move`、按稳定 `EntityId` 顺序分配落点，功能不因「映射未登记」被拦截（R1.4 / R13.6 / ADR 0002）

#### Scenario: frame-0 混合选中仍只为可移动单位生成 Move

- **WHEN** frame-0 装配的农民 + 主城 + 金矿混合选中集合对地面右键或按 `M` 下达移动命令（P8.0 回归场景）
- **THEN** 只为农民生成 `Move`，主城（建筑）与金矿（资源点）MUST NOT 出现在 `Move` 生成集合中、位置不变，落点分配与「只含这些农民」时逐条相同、按稳定 `EntityId` 顺序（R1.4 / R13.6 / P8.0 回归不退化）

#### Scenario: 状态哈希、快照与 SNAPSHOT_VERSION 不变

- **WHEN** 为支持动态实体判据而在 `sim_match.rs::EntityView` 加只读 `speed` 字段、在 `read_view` 投影 `UnitState.speed.raw()`、在 `input.rs::InputState::apply` 加合并函数后，重跑 `cargo test -p w3-core --test determinism` 并与基线比对
- **THEN** golden hash 逐字节不变，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合、快照头部布局（`defs_hash` / `content_hash`）、`first_difference()`、命令 / 事件、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键均不变（ADR 0002 §6-8 / `CODEBUDDY.md` §5 / R13.6）

#### Scenario: 默认空映射不再等价缺陷态

- **WHEN** `app.rs::ClientApp::new` 默认空映射或 `examples/playable.rs` 未注入映射，`view` 含运行时生产建筑（`EntityView.speed == 0`）并框选下达移动命令
- **THEN** 合并函数按视图 `speed` 补进有效映射，该建筑被判不可移动、MUST NOT 生成 `Move`，空映射不再触发 `unwrap_or(true)` 全放行，效果与「显式注入正确映射」一致（R1.4 / R13.6 / `input.rs::filter_movable`）
