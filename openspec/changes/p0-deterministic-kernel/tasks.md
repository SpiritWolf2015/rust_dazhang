## 1. Workspace 与数值基础

- [x] 1.1 建立虚拟 workspace 与 `w3-core` crate（edition 2024，零依赖）
- [x] 1.2 实现 `SimFixed` Q47.16、i128 中间量、溢出/除零显式错误
- [x] 1.3 实现整数 `Frame` / `FrameClock`，`step()` 不接受 delta

## 2. 状态所有权与命令

- [x] 2.1 实现 `EntityId` / `EntityStore`（ID 不复用、稳定 ID 顺序）
- [x] 2.2 实现 `CommandQueue` 冻结到达序号与四元组稳定排序
- [x] 2.3 实现 `RtsMatch` 私有状态、`read_view()` / `drain_events()`

## 3. 哈希、快照与验收

- [x] 3.1 实现 FNV-1a 状态哈希与 `first_difference()` 字段定位
- [x] 3.2 实现快照 v1 捕获/恢复与版本/内容哈希校验
- [x] 3.3 编写双实例交错、快照往返、回放等确定性验收测试
- [x] 3.4 记录 ADR 0002（确定性契约）与 ADR 0003（范围分期）
