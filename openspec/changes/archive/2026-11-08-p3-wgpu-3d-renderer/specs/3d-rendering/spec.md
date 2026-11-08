> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## ADDED Requirements

### Requirement: Render from read-only simulation view

渲染层 SHALL 只读取 `read_view()` 与事实事件；MUST NOT 持有或反写任何权威模拟状态；缺帧或未创建模型 MUST NOT 改变模拟推进。

#### Scenario: 模型加载失败

- **WHEN** 某个单位的模型资源缺失
- **THEN** 该单位不显示，但模拟状态哈希与另一无渲染实例一致

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
