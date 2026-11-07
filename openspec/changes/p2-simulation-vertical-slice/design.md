## Context

P0 提供定点数、整数帧、命令队列、状态哈希与快照；P1 提供本机导出的地图与摆放数据（原始 f64 坐标）。`rts_kernel_cs` 已有可用的移动/寻路/编队语义，但用 `double` 且不承诺跨平台确定性。P2 要把两者接通，并换成本项目的定点数学。

## Goals / Non-Goals

**Goals:** 一个可玩的竖切（选中 → 移动 → 攻击）；内容数据到模拟的转换规则冻结；所有新增状态进入哈希与快照。

**Non-Goals:** 3D 渲染、联机、技能/英雄/经济、JASS 触发器、完整科技树。

## Decisions

- **语义移植而非代码移植**：借鉴 `rts_kernel_cs` 的八方向寻路、朝向/坡度、群体规划语义，用 Rust + `SimFixed` 重写。否决直接调用其 double 数学。
- **冻结坐标转换**：`w3-content` 的 f64 → `SimFixed` 的缩放、舍入、单位 MUST 在新 ADR 中冻结，并在 `docs/decisions/` 记录。否决在运行时隐式转换。
- **app 层注入**：由 app 把冻结地图数据交给 `w3-core`；`w3-core` 不依赖 `w3-content`。否决让内核读取文件。
- **状态扩展走同一所有者**：订单、导航、动态障碍由 `RtsMatch` 内的模块独占，并进入哈希/快照。否决新增竞争状态或旁路写入。

## Risks / Trade-offs

- [Q47.16 在真实地图坐标（约 ±8192）下的精度/溢出] → 先核算范围，必要时换 Q 格式；溢出显式报错。
- [对象化实体存储在 300 单位以上的性能] → P0-P2 以 300 单位为验证规模；20,000 单位目标需另行评估 data-oriented 改造。
- [移植语义与原作行为差异] → 记录为残差，用逐帧哈希与重放测试约束，不声称完全等价。

## Migration Plan

需新增 ADR 冻结坐标转换；扩展 `compute_state_hash` / `capture_snapshot` 字段；`tests/determinism.rs` 重跑并扩展移动/攻击场景。

## Open Questions

- 坐标单位与 tileSize 的换算口径。
- 是否需要 `pathing.json`（WPM）作为寻路输入，还是仅用 heightfield 推导。
