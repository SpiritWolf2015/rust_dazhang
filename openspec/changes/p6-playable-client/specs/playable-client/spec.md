> 相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0004（冻结坐标与定点单位）、ADR 0005（protobuf 载荷折中方案）。

## ADDED Requirements

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

HUD SHALL 只从只读 `WorldView`、事实事件与只读 `Snapshot`（`capture_snapshot()` 公开只读 API）派生绘制选中框、血条、资源、生产队列与（可选）技能栏；MUST NOT 反写模拟状态（ADR 0002 §5）。`WorldView` 字段仅 `tick` / `rng` / `entities`，经济 / 人口 / 生产队列 / `max_hp` 等由 `HudModel::collect` 经只读 `Snapshot` 的 `gameplay` 字段派生；此只读派生不扩展 `WorldView`、不改动 `w3-core`（零改动）、不反写权威状态，也不进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照，权威状态哈希与快照字节保持不变（`tests/interaction.rs` 的 `hud_lifecycle_keeps_state_hash_and_snapshot_bytes` 已验证）。UI 文案 SHALL 走 i18n（可用中文），MUST NOT 进入日志、权威状态或状态哈希（CODEBUDDY.md §3）。UI 字体 / 图标 MUST NOT 为 MPQ 派生数据或任何暴雪资产，且 MUST NOT 被提交进仓库（ADR 0001）。资源或字体加载失败 MUST 以英文日志明确报告并降级为不绘制对应元素，MUST NOT 改变模拟推进。

#### Scenario: HUD 只读

- **WHEN** 生产队列、资源或单位生命发生变化
- **THEN** HUD 依据 `WorldView` / 事件刷新显示，且 `compute_state_hash()` 与无 HUD 实例一致

#### Scenario: 中文本地化文案

- **WHEN** HUD 显示中文标签
- **THEN** 文案不写入任何 `log` 记录、不进入权威状态、不进入状态哈希

#### Scenario: 资源缺失

- **WHEN** UI 字体或图标缺失
- **THEN** 以英文结构化日志记录错误、对应元素不绘制，且 `crates/w3-render/tests/isolation.rs` 的状态哈希比对仍全绿

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
