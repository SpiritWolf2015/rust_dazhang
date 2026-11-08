## Why

P0 有确定性内核、P1 有地图数据，但两者尚未连通，也没有任何可玩的玩法。P2 用一个最小竖切（选中 → 移动 → 攻击）验证"内容数据进入模拟"与"定点坐标"这条链路，同时把 `rts_kernel_cs` 的移动/寻路语义移植到定点数学上。

## What Changes

- 在 `w3-core` 增加单位订单、移动与攻击的权威状态，并把它们纳入快照与状态哈希。
- 移植八方向寻路、动态障碍、朝向/坡度、群体软编队与拥堵避让语义，改用 `SimFixed`。
- 冻结 `w3-content` 原始 f64 坐标到 `SimFixed` 的转换与舍入规则（ADR 0002 复核点）。
- 由 app 层把冻结的地图数据注入模拟；`w3-core` 仍不依赖 `w3-content`。

## Capabilities

### New Capabilities

- `unit-orders`: 选中、移动、停止、攻击命令的提交、校验与逐帧执行。
- `pathfinding`: 静态/动态障碍寻路、朝向转向、坡度减速、群体落点与拥堵处理。

### Modified Capabilities

- `deterministic-simulation`: 新增权威状态字段 MUST 进入快照与状态哈希（MODIFIED Requirements）。

## Impact

- `w3-core` 的 `compute_state_hash` / `capture_snapshot` / `first_difference` 需扩展；`tests/determinism.rs` 必须重跑全绿。
- 需要新的 ADR（坐标转换规则）或扩充 ADR 0002。
- 表现层仍不可反写模拟状态。
