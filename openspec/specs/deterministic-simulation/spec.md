# deterministic-simulation Specification

## Purpose
本规范定义确定性模拟内核的权威契约：权威状态一律用 `SimFixed`（Q47.16）整数运算，禁止 `f32` / `f64` 进入权威状态、快照与状态哈希；以整数帧推进且帧率冻结，到期命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；用 FNV-1a 与稳定 `EntityId` 顺序计算全状态哈希，快照为帧提交后的完整状态且头部含内容哈希，版本或内容哈希不匹配须拒绝恢复，守住 ADR 0002 确定性契约。

> 相关 ADR：ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。
## Requirements
### Requirement: Fixed-point arithmetic in authoritative state

所有权威模拟状态 SHALL 使用 `SimFixed`（i64 上的 Q47.16）整数运算；乘除 SHALL 使用 i128 中间量；溢出与除零 MUST 显式返回错误而非静默回绕。`f32` / `f64` MUST NOT 进入权威状态、快照或状态哈希。

#### Scenario: 重复运算位级一致

- **WHEN** 对同一对 `SimFixed` 值重复执行 `checked_mul` / `checked_div`
- **THEN** 每次得到完全相同的 raw i64，且结果不依赖宿主平台或 CPU

#### Scenario: 溢出被拒绝

- **WHEN** 对接近 `i64::MAX` 的值加一
- **THEN** 返回 `FixedError::Overflow`，不产生回绕后的状态

### Requirement: Integer logic frames

模拟 SHALL 以整数帧推进；`step()` MUST NOT 接受 delta、墙钟或渲染帧率；对局创建后 `logic_fps` MUST 冻结不变。

#### Scenario: 固定步长

- **WHEN** 连续调用 `step()` 任意次数
- **THEN** 帧号每次恰好加一，且状态哈希按同一序列演进

### Requirement: Stable command ordering

每帧到期的命令 SHALL 按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序后执行；提交时 MUST 冻结到达序号；执行时 MUST 重新校验实体存活与归属。

#### Scenario: 乱序提交

- **WHEN** 以任意顺序提交同一帧的多条命令
- **THEN** `freeze_frame` 返回的顺序恒由四元组排序键决定，与到达顺序无关

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
