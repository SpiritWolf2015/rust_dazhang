# 3d-rendering Specification

## Purpose
本规范定义从只读模拟视图渲染 3D 地形与单位的表现层契约，保证呈现层不反写权威状态、不影响模拟确定性，且导出资产不入库。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: Render from read-only simulation view

渲染层 SHALL 只读取 `read_view()`、事实事件与 P8.5e 表现层只读派生的三态迷雾网格 / 当前可见集合；MUST NOT 持有或反写任何权威模拟状态；缺帧或未创建模型 MUST NOT 改变模拟推进。`FrameProducer::next_frame()` SHALL 只交出当前逻辑帧的只读 `WorldView` 与事件拷贝，MUST NOT 调用 `RtsMatch::step()`；模拟推进 SHALL 由客户端的固定 30 Hz 整数帧累加器独家驱动（ADR 0002 §2、§5）。呈现 SHALL 通过 `Scene::interpolate_all(alpha)` 在相邻逻辑帧之间插值，`alpha` 与插值结果 MUST NOT 写回权威状态、进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 7，含 `defs_hash` 头）定义的快照（R10.5 / ADR 0002 §6-8 / Q4）。本 change P8.5e 的雾遮罩呈现与动态实例可见性过滤 SHALL 只在 `build_vertices` 喂入顶点 **之前**按格状态过滤 / 冻结上一已知态（R10.1），是只读表现层派生结果、纯函数稳定顺序无随机源，MUST NOT 反写 `RtsMatch` / 进状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5 / `scene.rs:904-933`）；迷雾遮罩独立于地形底图与权威状态，`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变、`w3-core` 零改动（R10.5 / ADR 0002 §6-8）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。历史 requirement 正文含「`SNAPSHOT_VERSION`（当前 3）」概述口径，按 Q10 推荐如实登记为已知限制、由 `docs-owner` 后续独立 change 统一收口。

#### Scenario: 模型加载失败

- **WHEN** 某个单位的模型资源缺失，或迷雾遮罩纹理 / 网格资源缺失
- **THEN** 该单位 / 该遮罩元素不显示，但模拟状态哈希与另一无渲染实例一致；降级 MUST NOT 把「不可见」写回权威状态或改变模拟推进（R10.1 / R10.5 / ADR 0002 §5）

#### Scenario: 重绘与逻辑帧解耦

- **WHEN** 窗口在 144 Hz 显示器上连续触发 `RedrawRequested`，并在重绘间反复刷新迷雾过滤结果
- **THEN** `next_frame()` 不推进模拟，逻辑帧仍按固定 30 Hz 累加器递增，逐帧状态哈希与 30 Hz 基准实例一致；当前可见每帧重算只依赖整数帧 `tick` 与只读视图、不受渲染帧率 / 墙钟影响（R10.1 / R10.5 / ADR 0002 §2-3）

#### Scenario: 插值不进权威状态

- **WHEN** 以不同 `alpha` 调用 `Scene::interpolate_all` 并重复渲染，且叠加雾遮罩呈现
- **THEN** `compute_state_hash()` 与快照字节和未插值 / 无雾遮罩实例相同，三态网格 / 已探索层为表现层本地只读派生、不进状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5 / Q9 形态①隔离回归 / `crates/w3-render/tests/isolation.rs`）

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
