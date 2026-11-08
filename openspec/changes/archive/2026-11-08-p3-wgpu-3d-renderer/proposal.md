## Why

P2 的模拟状态需要一个可视化出口才能验证"看起来对不对"，而 WC3 是 3D 骨骼动画游戏，与 `vera20k` 的 2D SHP 精灵管线完全不同。P3 建立全新的 wgpu 3D 渲染层，只消费模拟视图。本 change 对应 [ADR 0003](../../../docs/decisions/0003-scope.md) 路线图的 **P3 阶段**（wgpu 3D 渲染）；确定性契约见 ADR 0002，资产与法律边界见 ADR 0001。

## What Changes

- 新增 `w3-render` crate：wgpu/winit 窗口、glTF/MDX 骨骼动画、高度场地形、队伍色、粒子/挂点。
- 建立 `EntityId → 渲染实例` 映射，表现层只读 `read_view()` 与事实事件（ADR 0002）。
- 复用上游 `godot_warcraft3` 的 MDX→glTF 转换规格（骨骼、Sequence、GeosetAnim、FilterMode、PE2）；只读规格、不提交资产（ADR 0001）。

## Capabilities

### New Capabilities

- `3d-rendering`: 从模拟视图渲染 3D 地形与单位，且不影响模拟确定性。

### Modified Capabilities

（无。）

## Impact

对资产/法律边界（ADR 0001）：不新增、不提交任何 MPQ 派生的导出资产（含导出 JSON / PNG / GLB / 音效），只按本机 `config.toml` 路径读取；只读参考上游转换规格，不复制其代码。对确定性契约（ADR 0002）：本 change 不新增权威状态、不改动帧序、命令排序、状态哈希与快照 v1；呈现差异与 GPU 后端平台差异不计入模拟确定性，模拟内核跨平台确定性仍为待验证项。冻结坐标与定点单位（ADR 0004）只在视图边界做单向转换，不反写模拟。

- 新增 workspace 成员与 wgpu/winit 依赖；`sim/` 仍 MUST NOT 依赖 `render/`。
- 动画丢帧、资源加载失败 MUST NOT 反写模拟状态。
- 需要本机导出的 PNG/GLB/glTF（留在玩家机器上，不入库）。
