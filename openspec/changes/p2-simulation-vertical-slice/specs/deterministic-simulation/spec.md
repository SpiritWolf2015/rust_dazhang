## MODIFIED Requirements

### Requirement: State hashing and snapshot

系统 SHALL 用显式小端字节编码与稳定 `EntityId` 顺序计算全状态哈希；快照 SHALL 是帧 N 提交完成后的完整状态，含 RNG 流、下一 ID 计数器、已接受但未执行的命令、**所有单位订单、移动进度、朝向、寻路状态与动态障碍预约**；版本、`sim_version` 或内容哈希不匹配 MUST 被拒绝。

#### Scenario: 双实例一致性

- **WHEN** 两个实例喂相同命令脚本并交错推进
- **THEN** 每帧 `compute_state_hash()` 相等，且 `first_difference()` 返回 `None`

#### Scenario: 快照往返

- **WHEN** 在帧 N 捕获快照、销毁实例、恢复后继续跑到帧 M
- **THEN** 与未中断直接跑到帧 M 的实例逐帧哈希一致

#### Scenario: 新增状态遗漏进入哈希

- **WHEN** 新增一个权威状态字段但未加入哈希或快照
- **THEN** 确定性验收测试失败，阻止合并
