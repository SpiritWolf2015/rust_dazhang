# unit-orders Specification

## Purpose
本规范定义玩家命令的执行校验、单位移动的确定性积分与攻击结算，保证权威状态逐帧一致。

> 相关 ADR：ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: Command validation at execution

每条玩家命令 SHALL 经过 `submit_command` 校验并在目标帧执行时重新校验存活、在场与归属；接收 MUST NOT 等同于执行成功；失败 MUST NOT 留下部分状态变更。

#### Scenario: 目标已死亡

- **WHEN** 执行指向已移除实体的移动命令
- **THEN** 命令被丢弃，实体状态与权威哈希不变

### Requirement: Deterministic movement integration

单位位置 SHALL 以 `SimFixed` 逐帧推进，按稳定 `EntityId` 顺序结算；到达目标后 SHALL 发出一次性事实事件；表现层 MUST NOT 反写位置。

#### Scenario: 两实例移动一致

- **WHEN** 两个实例执行相同移动脚本
- **THEN** 每帧位置与状态哈希一致，且到达事件序号一致

### Requirement: Attack resolution within one frame

攻击结算 SHALL 在同一帧的既定阶段按稳定顺序完成；生命耗尽 MUST 立即置死，后续动作 MUST 再次检查存活。

#### Scenario: 同帧死亡

- **WHEN** 两个攻击在同一帧使同一目标生命归零
- **THEN** 目标只死亡一次，且不产生重复扣血或重复事件

### Requirement: 群体移动只作用于可移动单位

群体 `Move` 命令生成时 SHALL 只选中可移动实体；建筑、金矿、树木等不可移动实体 MUST NOT 生成 `Move`、MUST NOT 改变位置、MUST NOT 影响其余单位落点分配。可移动性判据 SHALL 在表现层输入翻译层完成、MUST NOT 回退建筑类型白名单，并按实体来源分层：frame-0 装配实体用装配期冻结定义派生的只读映射判定（权威），运行时 `Train` / `Build` 动态实体用只读 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动，两条来源 MUST NOT 互相替代。`EntityView.speed` SHALL 投影已有权威 `UnitState.speed`、MUST NOT 新增权威状态字段或改动命令语义 / 状态哈希 / 快照 / `SNAPSHOT_VERSION`（3）/ golden hash（ADR 0002；分层判据与约束放松依据见 `WC3-MELEE-RULES.md` R1.4 / R13.6 / §14）。

#### Scenario: 混合选中集合右键移动

- **WHEN** 框选集合同时含农民、主城与金矿，玩家右键下达移动命令
- **THEN** 只为农民生成 `Move` 命令，主城与金矿位置不变，逐帧状态哈希与「只选中农民」的等效脚本一致

#### Scenario: 全为不可移动实体

- **WHEN** 选中集合只含建筑 / 资源点并右键下达移动命令
- **THEN** 不生成任何 `Move` 命令，权威状态与状态哈希不变，且有明确的无可执行目标反馈

#### Scenario: 落点分配不受过滤影响

- **WHEN** 混合集合中多个农民与若干建筑一起右键移动
- **THEN** 农民的落点分配与集合只含这些农民时逐条相同，按稳定 `EntityId` 顺序

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

### Requirement: 人族新增实体沿用可移动性过滤并覆盖端到端一局

人族新增单位 / 建筑 SHALL 沿用由冻结定义 `UnitDef.speed_raw` 派生的表现层只读可移动性映射（R1.4 / R13.6 / P8.0 D1）；含新增建筑 / 资源点 / 主城的选中集合 MUST NOT 为其生成 `Move` 命令、MUST NOT 改变位置。端到端一局 SHALL 覆盖「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 结算 → 重开」各阶段且逐帧哈希一致（R3.1 / R4 / R5 / R6 / R11 / ADR 0002）。判据按实体来源分层：frame-0 装配实体用装配期 `UnitDef.speed_raw` 派生的只读映射判定（权威），运行时 `Train` / `Build` 动态实体用只读 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动，两条来源 MUST NOT 互相替代；系统 MUST NOT 回退建筑类型白名单（`WC3-MELEE-RULES.md` R1.4 / R13.6 / §14、change `runtime-entity-movability`）。

#### Scenario: 新增实体可移动性由 speed_raw 派生

- **WHEN** 装配期从 `FrozenDefinitions::units()` 对含人族新定义的 `EntityId→DefId` 关联派生可移动性映射
- **THEN** 远程 / 反重甲单位与英雄为可移动、人口建筑 / 祭坛 / 研发建筑与主城 / 金矿 / 树木为不可移动，且判据不来自运行时 `speed` 或类型白名单（R1.4 / R13.6）

#### Scenario: 混合选中集合右键移动排除新增建筑与资源点

- **WHEN** 框选集合同时含农民、远程单位、新增人口建筑 / 祭坛 / 主城与金矿，玩家右键或 `M` 下达移动命令
- **THEN** 只为农民与远程单位生成 `Move` 命令，新增建筑 / 资源点 / 主城 MUST NOT 出现在 `Move` 命令集合中、位置不变，逐帧状态哈希与「只选中可移动单位」的等效脚本一致（R1.4 / R13.6）

#### Scenario: 全为新增建筑时不生成 Move

- **WHEN** 选中集合只含新增建筑 / 主城 / 资源点并下达移动命令
- **THEN** 不生成任何 `Move` 命令、权威状态与状态哈希不变，且有明确的无可执行目标反馈（R1.4 / R13.6）

#### Scenario: 端到端一局各阶段可达且结算重开干净

- **WHEN** 按既有命令面依次执行「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」
- **THEN** 每阶段产生预期权威状态变化、胜负只结算一次、重开无残留，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致（R3.1 / R4 / R5 / R6 / R11.1 / R11.4 / R11.5 / ADR 0002）

#### Scenario: 新增定义不新增权威状态且进入快照头部

- **WHEN** 校验人族新增定义对权威状态与快照的影响
- **THEN** MUST NOT 新增权威状态字段；新增定义是冻结定义内容变化，经 `defs_hash` / `content_hash` 进入由 `SNAPSHOT_VERSION`（当前 3）定义的快照头部，状态哈希字段集合与快照头部布局不变（ADR 0002 §6-8 / C3）

#### Scenario: 反向验证（改坏验红）

- **WHEN** 以退化过滤器（不过滤、把新增建筑也当可移动）替换正式实现
- **THEN** 混合集合断言在缺陷态失败、修复态通过，两条证据均可复现（`CODEBUDDY.md` §5 / C5）
