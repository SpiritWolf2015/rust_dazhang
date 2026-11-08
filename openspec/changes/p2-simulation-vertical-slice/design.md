## Context

P0 提供定点数、整数帧、命令队列、状态哈希与快照；P1 提供本机导出的地图与摆放数据（原始 f64 坐标）。`rts_kernel_cs` 已有可用的移动/寻路/编队语义，但用 `double` 且不承诺跨平台确定性。P2 要把两者接通，并换成本项目的定点数学。

## Goals / Non-Goals

**Goals:** 一个可玩的竖切（选中 → 移动 → 攻击）；内容数据到模拟的转换规则冻结；所有新增状态进入哈希与快照。

**Non-Goals:** 3D 渲染、联机、技能/英雄/经济、JASS 触发器、完整科技树。

## Decisions

- **语义移植而非代码移植**：借鉴 `rts_kernel_cs` 的八方向寻路、朝向/坡度、群体规划语义，用 Rust + `SimFixed` 重写。否决直接调用其 double 数学。
- **冻结坐标转换**：`w3-content` 的 f64 → `SimFixed` 的缩放、舍入、单位 MUST 在新 ADR 中冻结，并在 `docs/decisions/` 记录。否决在运行时隐式转换。实现落在 [ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md)：纯整数 IEEE-754 位分解、round half away from zero、离散角度与整数 `isqrt`。
- **app 层注入**：由 app 把冻结地图数据交给 `w3-core`；`w3-core` 不依赖 `w3-content`。否决让内核读取文件。
- **状态扩展走同一所有者**：订单、导航、动态障碍由 `RtsMatch` 内的模块独占，并进入哈希/快照。否决新增竞争状态或旁路写入。

## Risks / Trade-offs

- [Q47.16 在真实地图坐标（约 ±8192）下的精度/溢出] → 先核算范围，必要时换 Q 格式；溢出显式报错。
- [对象化实体存储在 300 单位以上的性能] → P0-P2 以 300 单位为验证规模；20,000 单位目标需另行评估 data-oriented 改造。
- [移植语义与原作行为差异] → 记录为残差，用逐帧哈希与重放测试约束，不声称完全等价。
- [单位战斗与运动参数为诊断默认值] → `w3-app` 注入的 hp / speed / attack_damage / attack_range 是竖切诊断常量；正式玩法须由权威单位定义（SLK）约束，不信任联网客户端。

## Migration Plan

- 新增 [ADR 0004](../../../docs/decisions/0004-frozen-coordinates.md) 冻结坐标缩放 / 舍入 / 单位，以及离散角度、整数 `isqrt` 与整数斜率的定点表示。
- 新增 `crates/w3-app` 作为唯一注入边界；`w3-core` 增加 `StaticWorld`、`create_with_world` 与 `restore_snapshot(snapshot, world, expected_content_hash)`。
- 扩展 `compute_state_hash` / `capture_snapshot` / `first_difference` 覆盖订单、寻路、朝向、动态占地、群体规划与生命；快照版本升到 v2 并拒绝旧版本与身份不符的世界。
- `tests/determinism.rs` 重跑并扩展移动、编队、障碍重规划、同帧死亡、新字段哈希与分帧规划快照场景。

## 已决 Open Questions（P2 实现时冻结）

- **坐标单位与 tileSize 换算口径**：上游 `f64` 世界坐标与 `SimFixed` 1:1（`raw = round(value × 2^16)`），模拟内不做 tile 换算；`tileSize` 由 app 层冻结成 `StaticWorld.cell_size`（世界单位/格），`centerOffset` 冻结成 `StaticWorld.origin`，`world_to_cell` 先减原点再换算。单位坐标原样冻结、不平移，heightfield 顶点取每格左上角。真实 Echo Isles 头部为 `tileSize=128`、`centerOffset=(-8192,-6144)`。详见 ADR 0004。
- **是否需要 `pathing.json`（WPM）作为寻路输入**：P2 竖切**不读 WPM**，静态阻挡保持全可走，动态阻挡只经 `SetObstacle` / `RemoveObstacle` 帧命令增删；需要静态阻挡的测试直接构造 `StaticWorld` 显式提供。完整 WPM 解析留待后续阶段。
- **朝向 / 坡度的超越函数**：用离散角度单位、整数 `isqrt`、整数斜率与整数 smoothstep 重写，不引入 `libm` 或浮点，满足 ADR 0002 零依赖约束。
