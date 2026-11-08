## Context

P2 建立了可玩的确定性模拟，但没有可视化。WC3 是 3D 骨骼模型 + 高度场地形；`vera20k` 的 2D SHP 图集管线不可复用，只能借用其 wgpu/winit 工程基建。上游 `godot_warcraft3/docs/design/asset-convert/` 已记录 MDX 骨骼、蒙皮、Sequence、GeosetAnim、FilterMode、PE2 粒子与挂点规格。

## Goals / Non-Goals

**Goals:** 从只读模拟视图渲染地形与单位；保证表现层不影响确定性。

**Non-Goals:** 与 gamemd 的像素级视觉对齐；音频/UI 完成；GPU 性能达 20,000 单位；提交任何导出资产。

## Decisions

三个决策共同受 [ADR 0002](../../../docs/decisions/0002-determinism.md) 的确定性契约与 config.yaml context 的架构设计原则约束：SRP / OCP / LSP / DIP / ISP / LoD / CARP 按 Rust 语境（模块 / trait / 组合 / 所有权）判定，正确性 / 确定性优先；硬红线为确定性契约、唯一状态所有者、`w3-core` 不反向依赖上层与冻结输入边界、显式失败不静默降级、只读参考不复制代码。渲染层只单向消费只读视图，不新增权威状态、不改动状态哈希与快照 v1；下列每个决策分别说明对跨平台确定性与分层依赖的影响。

### 决策一：wgpu/winit 自研

- **结论**：新增 `w3-render` crate，直接用 wgpu + winit 建立窗口、交换链、单条渲染管线与离屏 readback；否决 Godot 宿主（目标为 Rust 自研，ADR 0003 已明确排除）与移植 `vera20k` 的 2D SHP 图集管线（WC3 是 3D 骨骼模型 + 高度场，语义不可复用）。
- **考虑过的备选方案**：
  - Godot 宿主 / GDScript：否决，ADR 0003 明确不做，且会把模拟状态暴露给外部运行时。
  - 移植 2D SHP 图集管线：否决，2D 精灵无法表达骨骼动画与高度场地形。
  - 引入完整游戏引擎承载渲染：否决，会带来与确定性无关的隐式调度与庞大依赖，越过最小依赖边界。
- **对跨平台确定性的影响（ADR 0002）**：wgpu 的 `f32` 矩阵、着色器与 GPU 后端只作用于表现层；权威状态仍是 `SimFixed` Q47.16 整数、30 Hz 整数帧与 FNV-1a 状态哈希，不因引入 GPU 而改变。Vulkan / DX12 / Metal / GL 的像素级呈现差异用 GPU readback 截图验收，不计入模拟确定性；ADR 0002 的跨平台确定性目前仅 Windows / rustc 1.91 实跑，Linux / macOS 仍是待验证项，本决策不声称已达成。
- **对确定性契约与分层依赖的影响**：DIP / ISP——渲染层通过 `FrameProducer` trait 只接收 owned `WorldView` + `Vec<Event>`，`w3-core` 不反向依赖 `w3-render`，`sim/` MUST NOT 依赖 `render/`；SRP——GPU 基建（`gpu/`）与场景状态（`scene.rs`）分离，唯一状态所有者仍是 `RtsMatch`。显式失败不静默降级：`GpuContext::new` 无适配器 / 设备失败时返回 `RenderError::GpuUnavailable`，不反写模拟。

### 决策二：只读视图 + 事件

- **结论**：`EntityId → 实例` 映射由表现层 `Scene` 独占，只消费 `read_view()` 的 owned `WorldView` 与事实事件拷贝；视觉插值、动画序列切换、模型缺失降级只改表现层 `f32` / 枚举。否决在渲染层缓存或反写权威状态。
- **考虑过的备选方案**：
  - 渲染层持有 `RtsMatch` / 缓存权威状态：否决，破坏唯一状态所有者与 ADR 0002 的表现边界。
  - 按渲染帧率 / 墙钟时间驱动模拟推进：否决，`step()` 不接受 delta 与墙钟，整数逻辑帧 30 Hz 已冻结。
  - 用无序容器迭代实例：否决，`Scene` 用 `BTreeMap` 稳定 `EntityId` 顺序，按 `Event.sequence` 排序去重，保证顶点生成可复现。
- **对跨平台确定性的影响（ADR 0002）**：权威状态的唯一写入者仍是 `RtsMatch`；`compute_state_hash()` 与快照 v1 的字段集合不变，模型加载失败、动画丢帧、两帧间插值 MUST NOT 改变状态哈希（对应 `3d-rendering` spec 的两个 Scenario）。`world_to_f32` / `facing_to_radians` 是视图边界的单向转换，冻结坐标（ADR 0004）与表现层 `f32` 都不进入哈希或快照；稳定迭代带来的呈现可复现性不等同于 ADR 0002 的模拟跨平台确定性，后者仍是待验证项。
- **对确定性契约与分层依赖的影响**：唯一状态所有者 + LoD / CARP——`Scene` 在结构上不接触 `RtsMatch`，无法反写位置、朝向或生命；事件只用于动画序列切换并按 `sequence` 去重，禁止迭代顺序 / 异步完成顺序影响规则；显式失败——资源缺失由 `ModelLibrary::resolve` 返回 `None`、单位不显示，异常数据只记日志或返回 `RenderError`，不改模拟推进。

### 决策三：复用转换规格而非产物

- **结论**：按上游 `godot_warcraft3/docs/design/asset-convert/` 的规格，用 `serde_json` 读取精简 JSON 子集（骨骼 / Sequence / GeosetAnim / FilterMode / 挂点 / tracks，camelCase 字段），由 `Model::from_path` 与 `ModelLibrary::load_dir` 从本机目录加载；否决把 GLB / PNG / MDX 或任何 MPQ 派生资产提交进仓库，也否决复制上游代码。
- **考虑过的备选方案**：
  - 直接解析 MDX 二进制：P3 暂缓，格式复杂且会把资产解析耦合进仓库。
  - 引入 `gltf` crate 解析完整 `.glb`：否决，本机离线缓存无该 crate，且完整二进制解析器超出精简子集所需。
  - 把 GLB / PNG / 导出 JSON 提交进仓库：否决，直接违反 ADR 0001 资产与法律边界。
  - 复制上游 `godot_warcraft3` / `WarsmashModEngine`（AGPLv3）代码：否决，只作只读格式语义参考，不复制代码（ADR 0001 / ADR 0003）。
- **对跨平台确定性的影响（ADR 0002）**：模型 JSON 的原始 `f32` 数值是冻结输入，只在表现层被采样；不进入权威状态、状态哈希与快照 v1。`sample_track` 的 `f32` 插值允许平台间细微差异，但因不回写模拟，不影响 `compute_state_hash()`；ADR 0002 的跨平台确定性结论不因此改变，仍为待验证项。
- **对确定性契约与分层依赖的影响**：冻结输入边界 + 只读参考不复制代码（硬红线）——`w3-render` 依赖 `w3-content` 的 `TerrainHeightfield` 与 `w3-core` 的只读视图，`w3-core` 不反向依赖任何上层；OCP——上游 schema 变化只需扩展 `ModelDocument` 的 `#[serde(default)]` 字段，不改模拟层；显式失败——非法 JSON / 缺文件返回 `RenderError::Model` / `RenderError::Asset`，单个非法文件跳过并记日志，不静默伪造状态。资产留本机、不入库（ADR 0001）。

## Risks / Trade-offs

- [wgpu 后端在不同平台的呈现差异] → 用 GPU readback / 截图验收，但不把呈现差异计入模拟确定性（ADR 0002）。
- [MDX 转换保真度未验证] → 先做静态模型，再做骨骼动画与粒子；残差记录。
- [性能未测] → 20,000 单位目标不在第一版承诺。

## Migration Plan

新增 crate；app 层编排。不改动 `w3-core` 的权威状态（ADR 0002）。

## 已决 Open Questions（P3 实现时冻结）

- **模型输入格式（glTF 还是自定义 MDX 直接读取）**：P3 实现时**两者都不直接做**——不解析 MDX 二进制，也不引入完整 glTF 二进制解析器（本机离线缓存无 `gltf` crate）。`crates/w3-render/src/model.rs` 用 `serde_json` 读取上游 `godot_warcraft3` MDX→glTF 转换规格的精简 JSON 子集（`ModelDocument`：bones / sequences / geosetAnims / attachments / materials / tracks，camelCase 字段）；`Model::from_path` 与 `ModelLibrary::load_dir` 从本机目录加载 `*.json`。**否决**：自定义 MDX 二进制解析器、依赖 `gltf` crate。转换产物（导出 JSON / PNG / GLB）留在本机、不入库（ADR 0001）。
- **队伍色与 FilterMode 的具体材质映射**：12 色 `TEAM_COLORS` 按 `owner % 12` 取（`team_color` 越界回绕、不 panic）；`FilterMode` 为 6 枚枚举（Opaque / Blend / Additive / AddAlpha / Modulate / Modulate2x）并提供 `is_transparent()`；`Material::resolve(owner)` 产出 `ResolvedMaterial { filter_mode, texture, team_color }`；挂点 `Model::attachment_world` 用骨骼全局矩阵变换 offset。**必须如实标注残差**：GPU 侧当前只有一条管线、统一使用 `BlendState::ALPHA_BLENDING`，逐材质 `FilterMode → blend state` 映射**尚未完成**，属已知残差（见 `openspec/changes/p3-wgpu-3d-renderer/presentation-acceptance.md` 的呈现验收与残差记录），不得声称已完成。材质与颜色均为表现层派生数据，不进入权威状态、状态哈希与快照 v1（ADR 0002）。
