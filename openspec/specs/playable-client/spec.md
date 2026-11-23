# playable-client Specification

## Purpose
本规范定义可玩客户端集成层的表现层契约：以冻结的 30 Hz 整数帧累加器独家驱动 `RtsMatch::step()` 并与呈现插值解耦；winit 键鼠事件只经公开提交入口翻译为权威命令，egui HUD 只读派生自模拟视图，交互相机、选中集合、插值余数与窗口尺寸等客户端状态均为表现层专属；本地与联机对局共用同一 `FrameProducer` / `StepDriver` 呈现管线，不引入通用游戏引擎。上述客户端状态一律不进入权威状态、FNV-1a 状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，`w3-core` 保持零改动、不新增权威状态，遵守 ADR 0002 确定性契约。

> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf 载荷折中方案）。

## Requirements

### Requirement: Fixed 30 Hz simulation stepping decoupled from redraw

客户端 SHALL 以冻结的 `logic_fps = 30`（ADR 0002 §2、ADR 0003「规模与帧率」）用整数帧累加器驱动 `RtsMatch::step()`；`step()` MUST NOT 接受 delta、墙钟时间或显示器刷新率；`RedrawRequested`、垂直同步与 GPU 呈现耗时 MUST NOT 决定逻辑帧数。累加器 SHALL 只累计整数帧步长，追帧 SHALL 有显式上限（防止 spiral of death），超限 MUST 明确报告而不静默丢弃帧。

#### Scenario: 高刷新率显示器

- **WHEN** 窗口在 144 Hz 显示器上连续收到 `RedrawRequested`
- **THEN** 每秒仍恰好推进 30 个逻辑帧，帧号按整数序列递增，且与 30 Hz 基准实例的逐帧 `compute_state_hash()` 一致

#### Scenario: 卡顿后的追帧

- **WHEN** 一次呈现耗时超过多个逻辑帧步长
- **THEN** 累加器在显式上限内补齐整数帧，超限以英文结构化日志报告，且 `step()` 从未收到 delta 或墙钟参数

#### Scenario: 不同刷新率的两端重放

- **WHEN** 两端分别以 60 Hz 与 144 Hz 呈现同一命令脚本
- **THEN** 逻辑帧序列、命令执行帧与逐帧状态哈希完全相同

### Requirement: Presentation-only interpolation

客户端 SHALL 使用既有 `Scene::interpolate_all(alpha)` 呈现相邻两个逻辑帧之间的插值；`alpha` SHALL 只来自累加器余数并只存在于表现层（允许 `f32` / `f64`），MUST NOT 写回位置 / 朝向 / 生命，MUST NOT 进入权威状态、FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照（ADR 0002 §1、§5、§6）。

#### Scenario: alpha 变化不改权威状态

- **WHEN** 在同一逻辑帧内以不同 `alpha` 连续插值并渲染多帧
- **THEN** `compute_state_hash()` 与捕获的快照逐字节不变

#### Scenario: 动画丢帧

- **WHEN** 表现层跳过若干插值帧或重复投递事件
- **THEN** 权威状态哈希与无渲染实例一致，且 `crates/w3-render/tests/isolation.rs` 全绿

### Requirement: Input translation to authoritative commands

客户端 SHALL 把 winit 键鼠事件翻译成 `Command` 或 `GameplayCommand`，且 MUST 只经 `submit_command` / `submit_gameplay_command` 提交（ADR 0002 §3、§5）；表现层 MUST NOT 直接读写 `RtsMatch` 的任何字段。输入 SHALL 至少覆盖选中、移动、攻击、施法（`Cast`）、生产（`Train`）、取消生产（`CancelTrain`）以及生产队列的呈现。选中集合 SHALL 是表现层状态，MUST NOT 生成权威命令或进入状态哈希。每条命令 SHALL 携带 `ExecuteFrame` / `PlayerId` / `Sequence`，MUST NOT 用墙钟或呈现帧号推导执行帧。

#### Scenario: 框选单位

- **WHEN** 玩家用左键拖拽框选一组单位
- **THEN** 只更新表现层选中集合，不提交任何命令，权威状态哈希不变

#### Scenario: 右键移动

- **WHEN** 玩家在选中单位后右键点击地面
- **THEN** 生成一条 `Command` 并经 `submit_command` 提交，执行帧由冻结帧号推导，随后按 `ExecuteFrame → PlayerId → Sequence → Arrival` 排序执行

#### Scenario: 玩法命令覆盖

- **WHEN** 玩家触发施法、生产或取消生产
- **THEN** 分别生成 `GameplayCommandKind::Cast` / `Train` / `CancelTrain` 并经 `submit_gameplay_command` 提交，与 P5 命令共用同一校验入口

#### Scenario: 表现层不得反写

- **WHEN** 输入处理或 HUD 回调运行
- **THEN** 不存在对 `RtsMatch` 字段的直接写入路径，`w3-core` 保持零改动

### Requirement: HUD rendered from read-only view

HUD SHALL 只消费只读 `WorldView`（`tick` / `rng` / `entities`）、只读 `Snapshot`（含 `revive` / `town_hall` / `creep_camps` / `item_shop` **与按 Q2 新增的族特色机制顶层字段**）与事实事件拷贝、按 `derive(snapshot, events)` 范式派生只读投影子模型（R13.6 / ADR 0002 §5 / Q2）。本 change P9 把 HUD 只读派生清单按表扩到 **4 族**（Q1' 已拍板双族全量）：选族 `FactionSelection` / `faction_overlay`（`hud.rs:866-871,1215-1271`）、命令卡目录 `CommandCatalog::for_race`、不死族 `3xx` / 暗夜族 `4xx` 段模型键投影、i18n 文案键（按 Q5' / Q6）；Q4 已拍板全量机制（R10.2 / R10.3 / R10.4 / R5.6 / S3 / R8.4）本期纳入，其 HUD 呈现 MUST 仍为表现层只读派生。**P8.5e `FogHudModel` 迷雾只读投影保持表现层只读派生语义不变、fog 可见性 MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`（R10.5 / §0.3 T3 / Q2' 已拍板、不抹平对照差异 D1）**。HUD 只读派生 MUST NOT 反写 `RtsMatch`、进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q3 bump 8）定义的快照或 `first_difference()`；不扩展 `WorldView` / `EntityView` 投影字段（ADR 0002 §5 / P8.4 `runtime-entity-movability` D1 先例）。含地面物品 / 商店（中立建筑）/ 金矿 / 树木 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6）。Q4 已拍板本期含全量机制（不再是后续承接）、Q1' 已拍板双族全量、Q4' 已拍板 1.0 张力七处显式如实登记、不静默窄化 / 不冒充已解决 / 不预设结论；本 change 的 **2 ADDED + 8 MODIFIED = 10 份 delta** 正好不触发 `Consider splitting changes with more than 10 deltas` warning、MUST NOT 新增第 9 份 MODIFIED delta（leader 例行决定 / Q9' / Q10'）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: HUD 只读

- **WHEN** 4 族（人 / 兽 / 不死 / 暗夜）选族 / 命令卡 / 模型键 / Q4 全量机制 HUD / 迷雾连续重绘并校验 `RtsMatch` 写入路径与哈希
- **THEN** `HudModel` 只消费只读视图 / 快照 / 事件、4 族投影与 `FogHudModel` 均为表现层本地只读派生状态，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节和 v8 新基线一致、不进权威状态 / 状态哈希 / 快照 / `first_difference()`；Q2 新增族特色机制顶层权威状态只在 `w3-core` 结算、HUD 只读消费不反写、fog 可见性 MUST NOT 进权威状态（R10.5 / R13.6 / ADR 0002 §5-8 / Q1' / Q2 / Q2' / Q3 / Q4 / Q6）

#### Scenario: 中文本地化文案

- **WHEN** 以不死 / 暗夜选族 overlay / 命令卡 / 生产队列 / Q4 全量机制面板读取 UI 文案并刷新
- **THEN** 4 族文案走 `i18n` 中文键（与既有两族同构）、不进日志 / 权威状态 / 状态哈希，日志与错误信息保持英文结构化 `key=value`、源码注释中文（`CODEBUDDY.md §2 / §3` / R13.6 / Q1' / Q4 / Q5'）

#### Scenario: 资源缺失

- **WHEN** 不死 / 暗夜模型键 / 文案键 / 遮罩资源或 Q4 全量机制呈现资源缺失或段键未命中库
- **THEN** MUST 以英文结构化 `key=value` 日志明确报告并降级为不绘制对应元素、MUST NOT 静默降级或把降级状态写回权威状态；按 Q3 bump 8 的快照契约下逐帧哈希 / 快照字节不变、不改变模拟推进（ADR 0002 §5 / `CODEBUDDY.md §3` / C9 / Q1' / Q2 / Q3 / Q4 / Q5'）

### Requirement: Client state stays non-authoritative

本 change MUST NOT 新增任何权威状态字段；`w3-core` SHALL 保持零改动且不反向依赖 `w3-client` / `w3-render` / `w3-net`（ADR 0002 §5、`config.yaml` 硬红线）。累加器余数、插值 `alpha`、相机位姿、选中集合、HUD 布局与窗口尺寸 MUST NOT 进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照；状态哈希字段集合与快照头部布局 MUST NOT 改变（ADR 0002 §6、§7）。任何将来转为权威的字段 MUST 同时进入状态哈希与快照，并先让 `crates/w3-core/tests/determinism.rs` 通过。

#### Scenario: 客户端状态变化不改哈希与快照

- **WHEN** 玩家改变选中、相机、窗口尺寸或 HUD 布局并推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节与无客户端实例完全一致，`SNAPSHOT_VERSION` 仍为 3

#### Scenario: 权威字段遗漏被门禁拦截

- **WHEN** 某个新字段被加入权威状态却未加入哈希或快照
- **THEN** `cargo test -p w3-core --test determinism` 失败，阻止合并

#### Scenario: 内核零改动

- **WHEN** 校验分层依赖与确定性基线
- **THEN** `w3-core` 无 diff，其依赖不含任何表现层 crate，且 `crates/w3-core/tests/determinism.rs` 全绿

### Requirement: Engine-free stack and dependency license gate

客户端 MUST NOT 引入通用游戏引擎（Bevy 等）或带自动并行调度 / 浮点权威变换的运行时（ADR 0003「明确不做」、ADR 0002 §1、§3）；SHALL 复用既有 `winit 0.30` + `wgpu 30` + `w3-render`，并采用已由用户拍板定案的 `egui` 家族（`egui` + `egui-wgpu` + `egui-winit`，取 MIT 一侧）作为轻量 UI 依赖，用户已按 CODEBUDDY.md §10 授权新增该 MIT 依赖与后续 `NOTICE.md` 登记（交 `docs-owner`，本 change 不自行编写）。版本不兼容时 MUST 明确报告并回退无新依赖的自绘 2D overlay，MUST NOT 静默降级或强行升级 `wgpu` / `winit`。新增依赖前 MUST 检查许可证与 GPLv3 兼容性，`Cargo.lock` MUST 提交，第三方代码 MUST 在 `NOTICE.md` 登记（由 `docs-owner` 跟进，ADR 0001）。后续依赖重大升级与许可证变更 MUST 仍由用户拍板后方可实施（CODEBUDDY.md §10）。

#### Scenario: 否决引擎

- **WHEN** 提案或依赖评审中出现 Bevy / 其他通用引擎
- **THEN** 以其自动并行调度与 `f32` / `f64` 变换同 ADR 0002 冲突为由明确否决，不写入 `Cargo.toml`

#### Scenario: 新增 UI 依赖

- **WHEN** 引入 `egui` / `egui-wgpu` / `egui-winit`
- **THEN** 已核验 MIT 与 GPLv3 兼容、`Cargo.lock` 已更新，并已生成 `NOTICE.md` 登记任务交 `docs-owner`

#### Scenario: 版本不兼容回退

- **WHEN** `egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30` 版本不兼容
- **THEN** 明确报告冲突并切换到自绘 2D overlay（无新依赖），不静默降级、不强行升级 `wgpu` / `winit`

### Requirement: Single presentation pipeline seam

`FrameProducer` SHALL 是本地与联机对局唯一的呈现管线接缝；两种模式 MUST 复用同一 `FrameProducer` / `StepDriver` 接缝与 `Scene::consume` / `Scene::interpolate_all` / `build_vertices` / `GpuContext::render_frame_with_overlay` / `Camera::matrix` 呈现管线，事件循环由 `w3-client::ClientApp<P: StepDriver>` 装配（本地 `FixedStepClient` 与联机 `NetSessionDriver` 都实现同一 `StepDriver: FrameProducer` 接缝；`RendererApp::run()` 仍保留在 `w3-render` 作为简易只读循环，完整客户端不调用它），且 MUST 只有一个 `RtsMatch` 权威写入者、不出现第二套呈现代码或第二份 `RtsMatch`（ADR 0002 §5、ADR 0005）。联机模式 SHALL 由 `ClientSession` 提供已冻结帧的命令，MUST NOT 让网络包到达、抖动或断线重连驱动 30 Hz 累加器或改变逻辑帧序（ADR 0002 §2、§3）。

#### Scenario: 模式切换同管线

- **WHEN** 在本地 `SimProducer` 与联机 session 适配的 `FrameProducer` 之间切换
- **THEN** 使用同一 `Scene` 与 `GpuContext` 代码路径，逐帧 `compute_state_hash()` 在相同命令流下一致

#### Scenario: 网络抖动不改帧序

- **WHEN** 命令包延迟、重传或短暂断线
- **THEN** 累加器仍按冻结 30 Hz 推进，缺输入按帧边界处理（不按墙钟补帧），命令仍按 `ExecuteFrame → PlayerId → Sequence → Arrival` 执行
