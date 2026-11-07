## Context

项目为 GPLv3 的 Rust 复刻 RTS，第一版目标含联机帧同步。三个参考项目中，`vera20k` 用定点数并明确要求跨平台确定性；`rts_kernel_cs` 用 `double` 且明确不承诺跨平台确定性。P0 必须先解决数值基础，否则联机与回放无法验证。实现见 commit `d7a4e19`，契约见 ADR 0002。

## Goals / Non-Goals

**Goals:**

- 建立可独立测试、无渲染/UI/音频/网络依赖的确定性内核。
- 提供 desync 可定位到字段的诊断能力。
- 让快照与回放成为可验证的验收门槛。

**Non-Goals:**

- 任何玩法（战斗、经济、技能、寻路）。
- 原生 gamemd 或 War3 浮点/x87 的精确模拟。
- 跨版本存档兼容、并行/异步仿真、ECS 框架。

## Decisions

- **定点数而非 f64**：`SimFixed` Q47.16 on i64，i128 中间量，整数除法向零截断。否决 `double`（`rts_kernel_cs` 路线，无法保证跨平台一致）与 x87 模拟（精确原生算术本身不是理由）。
- **零依赖 crate**：P0 只用标准库，离线可编译，数值契约完全自控。否决提前引入 `fixed` crate 或自建通用数值框架。
- **稳定顺序而非哈希容器**：`BTreeSet` / `BTreeMap` 按 `EntityId` 排序；FNV-1a 显式字节编码。否决 `DefaultHasher`、`HashMap` 迭代与对象身份。
- **唯一状态所有者**：`RtsMatch` 字段全私有，`read_view()` 返回 owned 拷贝。否决暴露可变内部集合。
- **30 Hz 冻结**：见 ADR 0002；变更需新 ADR 与全量重放验证。

## Risks / Trade-offs

- [Q47.16 范围与精度有限，大地图坐标可能溢出] → 溢出显式报错；P2 引入真实坐标时评估换 Q 格式或分段表示。
- [超越函数尚未验证] → 引入时须使用纯 Rust `libm` 并补跨平台边界测试。
- [仅在本机 Windows / rustc 1.91 实跑] → Linux/macOS 与其他 CPU 的确定性列为待验证项，不作已达成结论。
- [30 Hz 冻结后改动成本高] → 通过 ADR 复核点控制。

## Migration Plan

无迁移：P0 为新增 crate。后续阶段若改数值语义，须先让 `tests/determinism.rs` 全绿并重定基线。

## Open Questions

- P2 从 `w3-content` 的 f64 坐标到 `SimFixed` 的舍入规则尚未冻结。
- 20,000 单位规模目标（ADR 0003）与当前对象化实体存储的性能关系未验证。
