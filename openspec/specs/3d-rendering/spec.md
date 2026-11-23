# 3d-rendering Specification

## Purpose
本规范定义从只读模拟视图渲染 3D 地形与单位的表现层契约，保证呈现层不反写权威状态、不影响模拟确定性，且导出资产不入库。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: Render from read-only simulation view

渲染 SHALL 只读取只读 `WorldView` / 只读 `Snapshot`（含按 Q2 新增的族特色机制顶层字段）与表现层只读派生结果构建顶点与动态实例（`Scene::build_vertices` 纯函数、稳定顺序、无随机源），MUST NOT 反写 `RtsMatch`、进 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（按 Q3 bump 8）定义的快照（ADR 0002 §5 / R13.6）。本 change P9 把 **4 族**（Q1' 已拍板双族全量）模型键 / 几何呈现按表接入：人 `1xx` / 兽 `2xx` / 不死 `3xx` / 暗夜 `4xx` / 中立 `9xx` 共 5 段与 `SEGMENT_MODEL_KEYS` / `ENTITY_MODEL_FILES` 逻辑路径字符串合成键表同构、按 Q5' 推荐；**仅逻辑路径键表、非资产内容、零 MPQ 派生数据 / 真实导出 JSON / PNG / GLB 入库**（R2 / R4 / ADR 0001 / ADR 0004）。Q4 已拍板全量机制本期纳入，其呈现 MUST 仍只读（4 族 / Q4 口径并入本 Requirement 与既有 Scenario，MUST NOT 新增第 9 份 MODIFIED delta、2 ADDED + 8 MODIFIED = 10 份）。**P8.5e 迷雾遮罩 / 可见性过滤保持表现层只读呈现、fog 可见性 MUST NOT 进权威状态（R10.5 / §0.3 T3 / Q2' 已拍板、不抹平 vera20k fog 进哈希 D1 对照差异）**；1.0 字面「一并做完」与本期双族全量的张力按 Q4' 已拍板七处显式如实登记、不静默窄化、不冒充已解决。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 模型加载失败

- **WHEN** 不死 `3xx` / 暗夜 `4xx` 段模型键未命中库 / 本机无导出资产 / glTF 加载失败
- **THEN** MUST 以英文结构化 `key=value` 日志明确报告并降级为不绘制该元素（或既有四面体占位）、MUST NOT 静默降级或反写权威状态；CI 无本机资产不冒充验证真实网格、按 Q3 bump 8 的快照契约下逐帧 `compute_state_hash()` / `capture_snapshot()` 字节不变（ADR 0001 / ADR 0004 / ADR 0002 §5 / Q1' / Q3 / C9 / C12 / Q5'）

#### Scenario: 重绘与逻辑帧解耦

- **WHEN** 高刷新率 / 卡顿追帧下连续重绘含 4 族实体、Q4 全量机制呈现与迷雾遮罩的画面
- **THEN** 渲染帧率与 30 Hz 整数逻辑帧解耦、动态实例按只读视图稳定顺序展开、4 族段键派生无墙钟 / `HashMap` 迭代顺序 / `f64` / 表现层独立随机源，不改变模拟推进或逐帧哈希（ADR 0002 §2-3 / §5 / R12.4 / Q1' / Q2 / Q4 / Q5'）

#### Scenario: 插值不进权威状态

- **WHEN** 以表现层插值 alpha 呈现 4 族单位 / 建筑动画、Q4 全量机制效果并叠加迷雾遮罩
- **THEN** 插值 / 动画 / 4 族模型键 / 雾遮罩全是表现层只读派生、不进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（8）定义的快照 / `first_difference()`，逐帧 `compute_state_hash()` 与捕获的快照字节和 v8 新基线一致；Q2 族特色机制权威状态只在 `w3-core` 结算、fog 可见性 MUST NOT 进权威状态（R10.5 / R13.6 / ADR 0002 §5-8 / Q1' / Q2 / Q2' / Q3 / Q4 / Q5'）

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
