> 相关 ADR：ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## ADDED Requirements

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
