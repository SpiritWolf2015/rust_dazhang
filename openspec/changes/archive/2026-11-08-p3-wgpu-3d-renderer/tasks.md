## 1. 窗口与 GPU 基建

- [x] 1.1 新增 `w3-render` crate 与 wgpu/winit 依赖
- [x] 1.2 建立窗口、交换链与只读 `read_view()` 消费边界

## 2. 地形与模型

- [x] 2.1 从 `TerrainHeightfield` 构建 3D 地形网格
- [x] 2.2 加载 glTF/MDX 模型、骨骼动画与动画序列
- [x] 2.3 实现队伍色、FilterMode 材质映射与挂点

## 3. 隔离验收

- [x] 3.1 建立 `EntityId → 实例` 映射与事件消费去重
- [x] 3.2 验证模型加载失败/动画丢帧不改变状态哈希
- [x] 3.3 用截图与 GPU readback 做一次呈现验收并记录残差
