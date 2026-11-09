## MODIFIED Requirements

### Requirement: State hashing and snapshot

系统 SHALL 用显式小端字节编码与稳定 `EntityId` 顺序计算全状态哈希；快照 SHALL 是帧 N 提交完成后的完整状态，含 RNG 流、下一 ID 计数器、已接受但未执行的命令、单位订单与导航状态、**技能施法与冷却、Buff 实例、库存与人口、生产/建造队列、科技进度、AI 状态与待执行定时任务**；头部 SHALL 含冻结定义的内容哈希；版本、`sim_version` 或内容哈希不匹配 MUST 被拒绝。

> 相关 ADR：ADR 0002（确定性契约——FNV-1a 全状态哈希、帧提交后的完整快照与权威状态集合）；ADR 0001（内容哈希取自玩家本机导出数据，资产不入库）。

#### Scenario: 双实例一致性

- **WHEN** 两个实例喂相同命令脚本并交错推进
- **THEN** 每帧 `compute_state_hash()` 相等，且 `first_difference()` 返回 `None`

#### Scenario: 快照往返

- **WHEN** 在帧 N 捕获快照、销毁实例、恢复后继续跑到帧 M
- **THEN** 与未中断直接跑到帧 M 的实例逐帧哈希一致

#### Scenario: 恢复不重复施放

- **WHEN** 从含有进行中施法与 Buff 的快照恢复
- **THEN** 不重新扣费、不重新触发出生副作用、不再次施放技能
