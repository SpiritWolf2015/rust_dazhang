## Context

P2 建立了可玩的确定性模拟，但没有可视化。WC3 是 3D 骨骼模型 + 高度场地形；`vera20k` 的 2D SHP 图集管线不可复用，只能借用其 wgpu/winit 工程基建。上游 `godot_warcraft3/docs/design/asset-convert/` 已记录 MDX 骨骼、蒙皮、Sequence、GeosetAnim、FilterMode、PE2 粒子与挂点规格。

## Goals / Non-Goals

**Goals:** 从只读模拟视图渲染地形与单位；保证表现层不影响确定性。

**Non-Goals:** 与 gamemd 的像素级视觉对齐；音频/UI 完成；GPU 性能达 20,000 单位；提交任何导出资产。

## Decisions

- **wgpu/winit 自研**：否决 Godot 宿主（目标为 Rust 自研）与移植 2D SHP 管线。
- **只读视图 + 事件**：`EntityId → 实例` 映射由表现层持有；否决在渲染层缓存权威状态。
- **复用转换规格而非产物**：按上游文档读 glTF/PNG；否决把 GLB/PNG 提交进仓库。

## Risks / Trade-offs

- [wgpu 后端在不同平台的呈现差异] → 用 GPU readback / 截图验收，但不把呈现差异计入模拟确定性。
- [MDX 转换保真度未验证] → 先做静态模型，再做骨骼动画与粒子；残差记录。
- [性能未测] → 20,000 单位目标不在第一版承诺。

## Migration Plan

新增 crate；app 层编排。不改动 `w3-core` 的权威状态。

## Open Questions

- 采用 glTF 还是自定义 MDX 直接读取。
- 队伍色与 FilterMode 的具体材质映射。
