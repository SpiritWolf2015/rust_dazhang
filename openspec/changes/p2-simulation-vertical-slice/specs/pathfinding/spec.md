> 相关 ADR：ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## ADDED Requirements

### Requirement: Static and dynamic obstacle pathfinding

系统 SHALL 提供基于网格的八方向寻路，考虑静态地形阻挡与对局内动态占地；动态占地的增删 MUST 通过帧命令进行；导航快照 SHALL 校验静态地图身份。

#### Scenario: 阻挡变化触发重规划

- **WHEN** 路径上新增动态障碍
- **THEN** 单位在确定性帧点重规划，且不穿越障碍

### Requirement: Facing and slope scaling

朝向 SHALL 为权威状态，按帧转向并受转向速率限制；坡度对速度的影响 MUST 用 `SimFixed` 计算并进入状态哈希。

#### Scenario: 坡度一致

- **WHEN** 两个实例在相同高度场移动相同单位
- **THEN** 逐帧朝向与位置哈希一致

### Requirement: Group placement and congestion

群体落点 SHALL 使用固定工作量分帧规划与稳定槽位顺序；连续拥堵达到契约时长后 SHALL 判定当前目标失败并继续队列；停住单位 MUST NOT 被推移。

#### Scenario: 编队稳定

- **WHEN** 同一组单位以不同提交顺序下达群体移动
- **THEN** 按稳定排序键得到相同落点分配与逐帧轨迹
