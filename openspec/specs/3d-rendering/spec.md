# 3d-rendering Specification

## Purpose
本规范定义从只读模拟视图渲染 3D 地形与单位的表现层契约，保证呈现层不反写权威状态、不影响模拟确定性，且导出资产不入库。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。
## Requirements
### Requirement: Render from read-only simulation view

渲染层 SHALL 只读取 `read_view()` 与事实事件；MUST NOT 持有或反写任何权威模拟状态；缺帧或未创建模型 MUST NOT 改变模拟推进。`FrameProducer::next_frame()` SHALL 只交出当前逻辑帧的只读 `WorldView` 与事件拷贝，MUST NOT 调用 `RtsMatch::step()`；模拟推进 SHALL 由客户端的固定 30 Hz 整数帧累加器独家驱动（ADR 0002 §2、§5）。呈现 SHALL 通过 `Scene::interpolate_all(alpha)` 在相邻逻辑帧之间插值，`alpha` 与插值结果 MUST NOT 写回权威状态、进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照。

#### Scenario: 模型加载失败

- **WHEN** 某个单位的模型资源缺失
- **THEN** 该单位不显示，但模拟状态哈希与另一无渲染实例一致

#### Scenario: 重绘与逻辑帧解耦

- **WHEN** 窗口在 144 Hz 显示器上连续触发 `RedrawRequested`
- **THEN** `next_frame()` 不推进模拟，逻辑帧仍按固定 30 Hz 累加器递增，逐帧状态哈希与 30 Hz 基准实例一致

#### Scenario: 插值不进权威状态

- **WHEN** 以不同 `alpha` 调用 `Scene::interpolate_all` 并重复渲染
- **THEN** `compute_state_hash()` 与快照字节和未插值实例相同

### Requirement: Skeletal animation and team color

系统 SHALL 渲染 glTF/MDX 骨骼动画与动画序列，并应用队伍色材质层；视觉插值 MUST NOT 写回位置或朝向。

#### Scenario: 动画与逻辑分离

- **WHEN** 表现层在两帧之间插值单位位置
- **THEN** `compute_state_hash()` 与未插值实例相同

### Requirement: Terrain and heightfield presentation

系统 SHALL 用 `TerrainHeightfield` 的顶点高度与地表索引渲染 3D 地形；悬崖层与水体 MUST 按导出数据呈现。

#### Scenario: 地形只读

- **WHEN** 渲染层加载高度场
- **THEN** 不修改导出的 JSON 或模拟的权威地形状态

### Requirement: Interactive camera control

系统 SHALL 扩展现有 `Camera`（当前仅 `aspect`）以支持平移、缩放与旋转，并由输入层按表现层参数驱动；相机位姿 SHALL 是 `f32` / `f64` 表现层状态，MUST NOT 反写 `RtsMatch`、进入状态哈希或由 `SNAPSHOT_VERSION`（当前 3）定义的快照（ADR 0002 §1、§5）。窗口 `Resized` SHALL 只更新 `aspect` 与交换链，MUST NOT 触发 `step()`。

#### Scenario: 相机操作不改确定性

- **WHEN** 玩家连续平移 / 缩放 / 旋转相机并推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与未操作相机的实例一致

#### Scenario: 窗口尺寸变化

- **WHEN** `WindowEvent::Resized` 到达
- **THEN** 仅更新 `aspect` 与交换链尺寸，帧号与状态哈希不变

