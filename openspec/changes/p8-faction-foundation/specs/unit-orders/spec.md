# Spec Delta

## ADDED Requirements

### Requirement: 群体移动只作用于可移动单位

群体右键 `Move` 命令生成时 SHALL 只选中**由 client 侧冻结定义派生的可移动性判据**判定为可移动的实体；建筑、金矿、树木等不可移动实体 MUST NOT 生成 `Move` 命令、MUST NOT 改变位置，且 MUST NOT 影响其余可移动单位的落点分配。该判据 SHALL 在装配期从 `FrozenDefinitions::units()` 的 `UnitDef.speed_raw` 派生为表现层只读映射并注入输入翻译层；过滤 SHALL 在输入翻译层完成。系统 MUST NOT 为传递该判据扩展 `WorldView` / `EntityView`、MUST NOT 用运行时实体 `speed` 替代、MUST NOT 回退到建筑类型白名单；`w3-core` 的命令语义、状态哈希字段集合与快照口径 MUST NOT 改变（ADR 0002）。

#### Scenario: 混合选中集合右键移动

- **WHEN** 框选集合同时含农民、主城与金矿，玩家右键下达移动命令
- **THEN** 只为农民生成 `Move` 命令，主城与金矿位置不变，逐帧状态哈希与「只选中农民」的等效脚本一致

#### Scenario: 全为不可移动实体

- **WHEN** 选中集合只含建筑 / 资源点并右键下达移动命令
- **THEN** 不生成任何 `Move` 命令，权威状态与状态哈希不变，且有明确的无可执行目标反馈

#### Scenario: 落点分配不受过滤影响

- **WHEN** 混合集合中多个农民与若干建筑一起右键移动
- **THEN** 农民的落点分配与集合只含这些农民时逐条相同，按稳定 `EntityId` 顺序
