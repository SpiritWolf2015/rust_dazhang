## Why

P2 的模拟状态需要一个可视化出口才能验证"看起来对不对"，而 WC3 是 3D 骨骼动画游戏，与 `vera20k` 的 2D SHP 精灵管线完全不同。P3 建立全新的 wgpu 3D 渲染层，只消费模拟视图。

## What Changes

- 新增 `w3-render` crate：wgpu/winit 窗口、glTF/MDX 骨骼动画、高度场地形、队伍色、粒子/挂点。
- 建立 `EntityId → 渲染实例` 映射，表现层只读 `read_view()` 与事实事件。
- 复用上游 `godot_warcraft3` 的 MDX→glTF 转换规格（骨骼、Sequence、GeosetAnim、FilterMode、PE2）。

## Capabilities

### New Capabilities

- `3d-rendering`: 从模拟视图渲染 3D 地形与单位，且不影响模拟确定性。

### Modified Capabilities

（无。）

## Impact

- 新增 workspace 成员与 wgpu/winit 依赖；`sim/` 仍 MUST NOT 依赖 `render/`。
- 动画丢帧、资源加载失败 MUST NOT 反写模拟状态。
- 需要本机导出的 PNG/GLB/glTF（留在玩家机器上，不入库）。
