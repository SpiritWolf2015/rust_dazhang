# Spec Delta

## ADDED Requirements

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
