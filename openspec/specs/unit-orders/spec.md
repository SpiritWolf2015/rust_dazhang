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
