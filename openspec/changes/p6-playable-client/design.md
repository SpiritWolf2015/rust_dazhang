## Context

P0–P5 已交付：`w3-core`（零依赖确定性内核，`RtsMatch` 唯一写入者）、`w3-content`（冻结 `f64` 导出数据）、`w3-app`（注入适配层）、`w3-render`（`winit 0.30` + `wgpu 30` 只读呈现）、`w3-net` / `w3-relay`（lockstep 联机）、`gameplay-systems`（P5 玩法闭环，已 complete 并 push，change `p5-gameplay-systems` 按用户拍板先归档、再 apply 本 change；`openspec archive p5-gameplay-systems -y` 由编排者随后执行，本轮不归档、不预断结果）。

已具备、无需重写：`RendererApp<P: FrameProducer>::run()` 的 winit 事件循环 / 窗口 / 交换链（`crates/w3-render/src/gpu/mod.rs:469` 起）；`FrameProducer::next_frame(&mut self) -> (WorldView, Vec<Event>)`；`examples/render_frame.rs` 的 `SimProducer` 持有唯一 `RtsMatch` 并交出 `read_view()` / `drain_events()` 只读拷贝；`Scene::interpolate_all`、`Scene::consume`、`build_vertices`、`Camera`、`TerrainMesh`、`ModelLibrary`、`GpuContext`；`tests/isolation.rs` 的 3 个隔离验收（表现层消费只读视图、动画丢帧、事件重复投递都不改哈希）。快照当前 `SNAPSHOT_VERSION = 3`，头部含 `defs_hash` / `content_hash`（`crates/w3-core/src/snapshot.rs`）。

缺口（本 change 补齐）：逻辑帧绑死 `RedrawRequested`（`render_frame.rs:190` 附近 `RedrawRequested → render_once() → next_frame() → sim.step()`），违反 ADR 0002 §2；无键鼠输入→命令；无 HUD；`Camera` 仅 `aspect`；联机 `ClientSession` 未接进呈现管线。

约束与相关方：确定性是硬门禁（ADR 0002）；资产与法律边界（ADR 0001）；第一版 Melee 优先、范围见 ADR 0003；冻结坐标见 ADR 0004、线格式见 ADR 0005。用户已明确**否决引入 Bevy**，授权「A 方案」：复用 `winit 0.30` + `wgpu 30` + `w3-render`，加一个轻量 UI 依赖补齐交互层。用户对 a–d 四项待拍板决策已全部拍板（均选推荐）：UI 依赖定案 `egui` 家族（取 MIT 一侧，已按 CODEBUDDY.md §10 授权新增 MIT 依赖与后续 `NOTICE.md` 登记）、`w3-net` 接入纳入本期（不缩减范围）、新 crate 命名定案 `w3-client`、先归档 P5 再 apply P6。本 change 仅提案，代码留到 `/opsx:apply`。实施 owner 已指定 `client-owner`（由编排者新建 agent 定义，不在 `spec-steward` 可写范围）。

## Goals / Non-Goals

**Goals:**
- 把逻辑帧推进从显示器刷新率解耦，固定 30 Hz 整数帧累加器驱动 `step()`，用既有 `Scene::interpolate_all(alpha)` 做呈现插值。
- 建立 winit 键鼠 → `Command` / `GameplayCommand` 的翻译层，只经 `submit_command` / `submit_gameplay_command` 提交。
- 提供只读 HUD、可交互相机，并让本地与联机对局复用同一 `FrameProducer` 呈现管线。
- 全程不新增权威状态字段：`w3-core` 零改动，状态哈希字段集合与快照 v3 不变。

**Non-Goals:**
- 引入通用游戏引擎（Bevy 等）或任何自动并行 ECS 调度（已否决，见 D1）。
- JASS / wtg 触发器 VM、自定义地图、战役、战争迷雾 / 隐身、Mod API、跨版本存档、Godot 宿主、官方战网（ADR 0003「明确不做」）。
- 新增玩法阶段或玩法机制；P6 是 P3/P4/P5 能力面的集成收尾。
- 修改 `w3-core` 的帧序、命令排序、状态哈希编码或快照字段；修改 `logic_fps = 30`。
- 本轮不写 `crates/`、不写 `NOTICE.md`（属 `docs-owner`）、不改 `CODEBUDDY.md`、不 commit、不归档、不执行远端命令。
- 声称 Linux / macOS 跨平台确定性已达成（ADR 0002：目前仅 Windows / rustc 1.91 实跑）。

## Decisions

### D1：引擎选型——采用「A 方案」复用既有栈，否决 Bevy

- **决策**：复用 `winit 0.30` + `wgpu 30` + `w3-render` + 新增 `w3-client`，至多加一个轻量 UI 依赖；不引入通用游戏引擎。ADR 0003「明确不做」段补记此边界。
- **备选方案（Alternatives considered）**：
  - *引入 Bevy（含 Bevy UI / bevy_wgpu）* — **否决**。Bevy 的 ECS 自动并行调度与 `f32` / `f64` `Transform` 会同 ADR 0002 §1（禁止浮点进入权威状态）、§3（禁止异步任务完成顺序影响规则）、§6（稳定字段集合）直接冲突；且强制接管 winit / wgpu 版本，等于重写 P3/P4 的呈现与联机栈，属 CODEBUDDY.md §10 的「依赖重大升级」，人工独占。
  - *引入其它一体式引擎（macroquad / Fyrox / godot-rust）* — **否决**。同样接管窗口与主循环，与已有 `RendererApp::run()` 重复，违反 SRP / CARP，且带来许可证与依赖树复核成本。
  - *纯手写全部 UI 与窗口 glue、不加任何依赖* — **保留为 D4 的备选**（自绘 2D overlay），但作为整体方案范围过大、进度风险高。
  - *A 方案（复用 + 轻量 UI）* — **采纳**。改动面最小，全部新状态留在表现层。
- **关联 ADR 0002 与跨平台确定性影响**：不引入并行调度与浮点权威变换，整数帧 / 稳定排序 / FNV-1a 哈希的语义原样保留；表现层的 `f32` / `f64` 只用于 MVP 与插值，MUST NOT 进入权威状态。跨平台确定性仍是待验证项（仅 Windows / rustc 1.91 实跑），本决策不宣称达成，也不新增需验证的算术来源。
- **架构原则与分层依赖**：CARP（组合复用既有 `RendererApp` / `Scene` / `GpuContext`，而非重写）、SRP（渲染仍只读、客户端拥有输入与时钟）、DIP（客户端依赖 `FrameProducer` 抽象而非具体 GPU 实现）。`w3-core` 不反向依赖任何上层 crate 的硬红线不变。

### D2：固定 30 Hz 累加器 + 已有插值，逻辑帧与重绘解耦（最高优先）

- **决策**：在客户端层用整数帧累加器按冻结 `logic_fps = 30` 驱动 `RtsMatch::step()`；`FrameProducer::next_frame()` 改为只交出当前只读视图与事件拷贝，MUST NOT 调用 `step()`；渲染帧用 `Scene::interpolate_all(alpha)` 呈现，`alpha` 只来自累加器余数。**BREAKING**：`next_frame()` 语义变更，`SimProducer` 等实现者须迁移。
- **备选方案（Alternatives considered）**：
  - *维持现状（`RedrawRequested` 里 `step()`）* — **否决**。直接违反 ADR 0002 §2，且 `step()` 次数随显示器刷新率变化，联机重放必 desync。
  - *把 delta / 墙钟传给 `step()` 做补帧* — **否决**。ADR 0002 §2 明确禁止 delta / 墙钟；ADR 0002 备选方案表已记录「帧率可变 / 按墙钟补帧」为否决项。
  - *把逻辑 tick 放到独立 OS 线程* — **否决**。引入并行 / 异步完成顺序，触碰 ADR 0002 §3 与「引入并行 / 异步仿真」复核点，收益不抵风险。
  - *依赖 VSync 把呈现锁到 30 Hz* — **否决**。显示器相关、不可移植，且呈现卡顿仍会改变逻辑帧数。
  - *整数帧累加器 + 呈现插值* — **采纳**。符合 ADR 0002 §2/§5，且 `interpolate_all` 已存在，无需新渲染能力。
- **关联 ADR 0002 与跨平台确定性影响**：累加器只做整数帧计数，`step()` 仍不接受任何时间参数；帧序列与命令执行帧在 60 Hz / 144 Hz 呈现下逐帧一致。`alpha` 允许 `f32` / `f64` 但仅存在于表现层，MUST NOT 写回位置 / 朝向 / 生命、不进状态哈希与快照。追帧设显式上限（spiral of death 保护），超限 MUST 以英文日志明确报告，遵守「显式失败不静默降级」硬红线。跨平台确定性仍未实测，故不得把插值浮点结果纳入任何可比较的权威量。
- **架构原则与分层依赖**：SRP（时钟归客户端、呈现归 `w3-render`）、OCP（通过改变 `FrameProducer` 契约扩展，不侵入 `RtsMatch`）、LoD（渲染层只与 `WorldView` / `Event` 打交道）。`w3-core` 零改动，冻结输入边界（`w3-content` 的 `f64` 只经 `w3-app::freeze_f64`）不受影响。

### D3：输入 → 命令只经两个提交入口

- **决策**：winit 键鼠事件翻译成 `Command` / `GameplayCommand`，只经 `RtsMatch::submit_command` / `submit_gameplay_command` 提交；覆盖选中、移动、攻击、施法、生产、取消生产等；选中集合是表现层状态，不生成权威命令。
- **备选方案（Alternatives considered）**：
  - *表现层直接改 `RtsMatch` 字段 / 持有 `&mut`* — **否决**。违反 ADR 0002 §5「`RtsMatch` 唯一写入者、字段全私有」，且 `read_view()` 返回 owned 数据已封死此路。
  - *新增第三条「UI 专用」提交 API 到 `w3-core`* — **否决**。扩大内核表面积、需要改 `w3-core`，与「本 change 零改动内核」冲突；P5 已确立玩家与 AI 命令同源校验入口。
  - *把 UI 事件写成一个 `EffectDef` / 玩法命令变体* — **否决**。混淆「输入」与「玩法效果」，且会让 UI 布局参数进入权威状态。
  - *复用既有 `Command` / `GameplayCommand` + 两个提交入口* — **采纳**（P5 的 `GameplayCommandKind::Cast` / `Train` / `CancelTrain` 已存在）。
- **关联 ADR 0002 与跨平台确定性影响**：命令仍携带 `ExecuteFrame` / `PlayerId` / `Sequence`，由冻结帧号推导，MUST NOT 用墙钟或呈现帧号；`freeze_frame` 的 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序与执行时存活 / 归属重校验原样保留。提交失败 MUST 明确返回错误，不静默降级。键鼠坐标到 `SimFixed` 的转换 MUST 走既有冻结边界（ADR 0004），不得引入新的浮点→权威状态路径。
- **架构原则与分层依赖**：DIP（UI 依赖命令抽象而非内核内部状态）、ISP（输入层只见两个窄提交接口）、唯一状态所有者硬红线不变；`w3-core` 不依赖 `w3-client`。

### D4：UI / HUD 依赖选型（**已由用户拍板 a：定案 `egui` 家族**）

- **决策（已定案）**：采用 `egui` + `egui-wgpu` + `egui-winit`，`egui` 取 `MIT OR Apache-2.0` 的 MIT 一侧（与 GPLv3 兼容，CODEBUDDY.md §6）；用户已按 CODEBUDDY.md §10 授权新增该 MIT 依赖与后续 `NOTICE.md` 登记（交 `docs-owner`，本 change 不自行编写）。风险：`egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30`（当前 `default-features = false, features = ["rwh_06"]`）的版本兼容性仍须在 apply 阶段核验；若不兼容，MUST 明确失败并回退自绘 2D overlay，MUST NOT 静默降级或强行升级 `wgpu` / `winit`（重大升级属人工独占，CODEBUDDY.md §10）。
- **备选方案（Alternatives considered）**：
  - *Bevy UI* — **否决**，理由同 D1；随 D1 一并记录为已否决备选。
  - *自绘 2D overlay（沿用现有 wgpu 管线，零新依赖）* — **保留为回退备选**（版本不兼容时启用）。许可证 / 版本风险最低，但 HUD 布局、文字排版、命中测试需自行实现，工作量最大。
  - *`egui` 家族* — **已由用户拍板采纳**。开发效率最高，代价是新增依赖 + 版本耦合风险 + `NOTICE.md` 登记（交 `docs-owner`）。
  - *`iced` / `imgui-rs` 等其它 UI 库* — **暂不采纳**，依赖树与 wgpu 集成成本不低于 egui，且需再次做 GPLv3 兼容复核。
- **关联 ADR 0002 与跨平台确定性影响**：无论哪个方案，HUD MUST 只从 `WorldView` / 事实事件读取，UI 文案走 i18n 且 MUST NOT 进入日志、权威状态或状态哈希（CODEBUDDY.md §3）；字体加载失败以英文日志明确报告并降级为不绘制，MUST NOT 改变模拟推进。UI 库内部的浮点布局与哈希无关，但须严格隔离在 `w3-client`，不得被 `w3-render` 的只读路径或 `w3-core` 引用。
- **架构原则与分层依赖**：SRP / ISP（HUD 只读视图，不参与命令生成以外的写路径）、CARP（优先复用现有 wgpu 管线）。新增依赖前 MUST 检查许可证与 GPLv3 兼容性，`Cargo.lock` MUST 提交，第三方代码 MUST 在 `NOTICE.md` 登记（ADR 0001、CODEBUDDY.md §6）；本 change 不自行编写 `NOTICE.md`。

### D5：相机扩展落在 `w3-render` 的 `Camera`

- **决策**：扩展 `crates/w3-render/src/gpu/mod.rs` 的 `Camera`（当前仅 `aspect`）支持平移 / 缩放 / 旋转，仍由 `matrix()` 生成 MVP；输入映射放在客户端层。**BREAKING**：`Camera { aspect }` 结构体字面量调用点（如 `render_frame.rs`）须迁移，建议用构造器 / `Default` 降低破坏面。
- **备选方案（Alternatives considered）**：
  - *在 `w3-client` 另建独立相机结构并自行算矩阵* — **否决**。重复实现 MVP，违反 CARP / DRY，且 `RendererApp` 已持有 `Camera`。
  - *把相机位姿放进 `w3-core` 作为权威状态* — **否决**。相机是纯表现层 `f32` / `f64` 状态，进权威状态会违反 ADR 0002 §1，并强制扩展状态哈希与快照、重跑全部确定性验收。
  - *只加 `aspect` 之外的常量偏移、不支持交互* — **否决**，不满足可玩性需求。
  - *扩展现有 `Camera` + 新增字段构造器* — **采纳**。
- **关联 ADR 0002 与跨平台确定性影响**：相机位姿 MUST NOT 反写 `RtsMatch`、进状态哈希或快照；`WindowEvent::Resized` 只更新 `aspect` 与交换链，MUST NOT 触发 `step()`。相机矩阵是呈现专用浮点，不参与任何可比较量，故不影响跨平台确定性结论（仍为待验证项）。
- **架构原则与分层依赖**：OCP（扩展 `Camera` 而非替换）、SRP（相机几何在 `w3-render`，输入映射在 `w3-client`）、LoD。`w3-render` 仍不依赖 `w3-net` / `w3-core` 的上层扩展。

### D6：把 `w3-net::ClientSession` 适配为 `FrameProducer`（**已由用户拍板 b：纳入本期**）

- **决策（范围已定：纳入本期，不缩减范围）**：让联机对局通过 `FrameProducer` 适配复用同一 `RendererApp::run()` / `Scene` / `GpuContext` 管线；适配层只把已冻结帧的输入命令经 `submit_command` / `apply_frame` 提交，全程只有一个 `RtsMatch`。用户已拍板保留第 8 组任务与 `lockstep-networking` delta（含 MODIFIED 块与两条 ADDED requirement），不缩减范围。已冻结帧由 `ClientSession` 的 `FrameScheduler` / `FrameAssembler` / `pop_ready` 提供，`stabilize` / `apply_frame` 的排序语义不变。
- **备选方案（Alternatives considered）**：
  - *联机单独写一套渲染循环* — **否决**。出现两套呈现代码与两个状态所有者，违反 SRP / CARP 与 ADR 0002 §5。
  - *让 `w3-net` 直接持有并推进 `RtsMatch`* — **否决**。造成 `w3-core` 被上层反向编排、帧序出现第二个来源，违反分层依赖与 ADR 0002 §2。
  - *本期不接入，仅定义 `FrameProducer` 接缝、把联机留到后续 change* — **未采纳**（用户拍板 b = 是，纳入本期）；曾为「否」分支，此时 `lockstep-networking` delta 可缩减为只保留「网络时序不驱动固定步进」的约束。
  - *`ClientSession` → `FrameProducer` 适配* — **已由用户拍板采纳**。
- **关联 ADR 0002 与跨平台确定性影响**：网络包到达、抖动、重传、乱序、断线恢复 MUST NOT 驱动 30 Hz 累加器或改变 `step()` 次数与命令排序；缺失输入 SHALL 按既定帧边界处理（`lockstep-networking` 既有要求），不按墙钟补帧、不无限阻塞。逐帧 `compute_state_hash()` 比对、`first_difference()` 字段集合、`W3N1` 25 字节帧头与 protobuf 编码约束（ADR 0005）MUST NOT 改变。`w3-core` 零改动且不反向依赖 `w3-net` / `w3-render` / `w3-client`。
- **架构原则与分层依赖**：DIP（呈现依赖 `FrameProducer` 抽象，本地 / 联机两种实现可替换，符合 LSP）、SRP、唯一状态所有者硬红线。冻结帧与本地累加器的职责边界：累加器决定「何时推进一个整数帧」，网络层决定「该帧有哪些已冻结命令」，两者不可互换。

### D7：新 crate 命名与分层依赖（**已由用户拍板 c：定案 `w3-client`**）

- **决策（已定案：`w3-client`）**：新增独立 crate `w3-client` 承载事件循环装配、固定累加器、输入翻译、HUD 与 session 适配；`Camera` 扩展留在 `w3-render`。
- **备选方案（Alternatives considered）**：
  - *全部塞进 `w3-render`* — **否决**。`w3-render` 目前是「只读呈现层」，引入输入 / 权威状态持有者会破坏 SRP 与 ADR 0002 §5 的表现边界，也会让 `w3-render` 依赖 `w3-net`。
  - *复用现有 `w3-app`* — **否决**。`w3-app` 是「内容层 → 内核」的冻结注入适配层（`freeze` / `inject`），语义与交互客户端完全不同，混合会违反 SRP / ISP。
  - *命名 `w3-ui` / `w3-shell` / `w3-game`* — **未采纳**（用户拍板 c = `w3-client`）；语义偏窄或偏宽，`w3-client` 与 `lockstep_client` 示例、`ClientSession` 术语一致。
  - *新建 `w3-client`* — **已由用户拍板采纳**。
- **关联 ADR 0002 与跨平台确定性影响**：新 crate 只依赖 `w3-core` / `w3-render` /（可选）`w3-net` / `w3-content`，`w3-core` 绝不反向依赖；所有新字段都是表现层专属，MUST NOT 进入 FNV-1a 状态哈希或快照 v3。crate 边界本身不改变任何可比较量，也不改变跨平台确定性的待验证状态。
- **架构原则与分层依赖**：SRP / DIP / 唯一状态所有者 / `w3-core` 不反向依赖上层（硬红线）。**owner 归属**：实施 owner 已指定 `client-owner`，由编排者新建该 agent 定义（不在 `spec-steward` 可写范围），满足 CODEBUDDY.md §8「一个 change 一个 owner」；结案记录见 Open Questions。

### D8：归档顺序——先归档 P5，再 apply P6（**已由用户拍板 d**）

- **决策（已定案）**：`p5-gameplay-systems` 已 complete（13/13，commit `d9f5321`，已 push），按用户拍板应**先由编排者执行 `openspec archive p5-gameplay-systems -y` 并确认主 `openspec/specs/deterministic-simulation/spec.md` 已吸收 P5 的 `MODIFIED Requirements: State hashing and snapshot`**，再进入 P6 的 `/opsx:apply` 与最终归档。本 change 不自行归档、不预断 P5 已归档，归档动作由编排者随后执行。
- **备选方案（Alternatives considered）**：
  - *先归档 P6、后归档 P5* — **未采纳**（用户拍板 d = 先 P5 后 P6）。P5 的 delta 会重写主 `deterministic-simulation` 的哈希 / 快照 requirement；若 P6 在 P5 之前归档，P6 归档后主 spec 仍处于「P5 未吸收」的中间态，两个 change 对同一主 spec 的先后写入难以复核。
  - *让 P6 也产生 `deterministic-simulation` delta（例如把「累加器」写进内核 spec）* — **否决**。P6 不新增权威状态、不动 `w3-core`，不该碰该主 spec；且会与 P5 的 MODIFIED block 形成直接冲突。本 change 的三份 delta 落在 `playable-client`（新增）、`3d-rendering`、`lockstep-networking`，**刻意不产生 `deterministic-simulation` delta**，从源头规避冲突。
  - *P5 永不归档、长期保留 active* — **否决**。主 specs 是 source of truth（CODEBUDDY.md §8），长期不归档会让 P5 的 `gameplay-systems` 能力无法进入主 specs。
  - *先 P5 后 P6* — **已由用户拍板采纳**。
- **关联 ADR 0002 与跨平台确定性影响**：归档顺序纯属规范文档操作，不改变代码与可比较量；但 P5 归档会把「技能 / Buff / 生产队列 / 内容哈希进入哈希与快照」写入主 spec，P6 的 specs 引用 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）正是以此为准，顺序颠倒会造成文档口径不一致。
- **架构原则与分层依赖**：单一权威（主 specs 为 source of truth）、一个 change 一个 owner；归档由编排者随后执行，不触碰 `crates/`。

### D9：一致性维护——ADR 0003 分期表与 `config.yaml` 快照措辞

- **决策**：在 `docs/decisions/0003-scope.md` 把 P5 行改为「已完成（commit `d9f5321`）」、新增 P6 行（可玩客户端集成层，明确标注「集成收尾、不新增玩法阶段」）、在「明确不做」补「不引入通用游戏引擎（Bevy 等）」并引用 ADR 0002 的自动并行调度与浮点冲突；把 `openspec/config.yaml` 的过期「快照 v1」措辞同步为由 `SNAPSHOT_VERSION` 定义（当前 3，含 `defs_hash` 头）。
- **备选方案（Alternatives considered）**：
  - *完全不动分期表，只在 design/proposal 说明 P6 属 P3/P4/P5 集成收尾* — **部分采纳**。用户给出的这一 fallback 意在避免「新增玩法阶段」的误读；本设计同时采用两层的写法：ADR 0003 增加 P6 行但显式标注「非玩法阶段」，proposal 也写明落在 P3/P4/P5 能力面收尾，两处口径一致。
  - *把 P6 写成新的玩法阶段* — **否决**。P6 不新增玩法机制，且 `config.yaml` rules.proposal 明确「标明路线图阶段（P0–P5）」。
  - *顺手改写 `config.yaml` rules.proposal 的「（P0-P5）」* — **已由用户拍板保持「P0-P5」**：属规则本身的阶段枚举变更，超出「同步过期措辞」范围；用户按判断直接采纳，proposal 继续双标 P6 与 P3/P4/P5 能力面，不改为「P0–P6」。
  - *只改 `config.yaml` 第 12 行 context，不改 rules.specs 的「快照 v1」* — **否决**。两处是同一过期事实，若只改一处，rules.specs 会继续要求 specs 声明「进入快照 v1」，与本 change 的 specs（v3）打架；故两处一并同步为版本无关表述。
- **关联 ADR 0002 与跨平台确定性影响**：纯文档一致性改动，不改变任何代码、帧序、哈希或快照字段；同步后 spec 与 config 对「快照版本由 `SNAPSHOT_VERSION` 定义」的口径统一，避免与 `crates/w3-core/src/snapshot.rs` 的事实及 P5 归档后的主 spec 冲突。
- **架构原则与分层依赖**：单一权威（ADR 记「为什么决定」、OpenSpec 记「系统该做什么」、spec 引用 ADR 编号）。`CODEBUDDY.md:49`（「快照 v1」）与 `CODEBUDDY.md:8`（「P2–P5 active」「主 specs 仅两个」）同类过期措辞不在本 change 可写范围，仍列为建议交 `docs-owner` 同步。

## Risks / Trade-offs

- [逻辑帧与呈现刷新率耦合是既有实现的隐性 desync 源] → D2 用整数帧累加器解耦，并以 60 Hz / 144 Hz 双刷新率重放场景作验收；追帧设显式上限 + 英文日志，不静默降级。
- [`egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30` 版本不兼容] → 已拍板采用 `egui` 家族，但提案与规范阶段不锁版本；核验证据在进入 `/opsx:apply` 前产出，不兼容即明确报告并回退自绘 2D overlay（零新依赖），MUST NOT 静默降级或强行升级 `wgpu` / `winit`（人工独占）。
- [新增依赖的许可证与 `NOTICE.md` 登记] → 取 `egui` MIT 一侧并复核 GPLv3 兼容；`Cargo.lock` 提交；用户已按 CODEBUDDY.md §10 授权新增 MIT 依赖与后续 `NOTICE.md` 登记（`NOTICE.md` 仍属 `docs-owner`，本 change 不自行编写）；后续 `wgpu` / `winit` 依赖重大升级 / 许可证变更仍属人工独占，须重新拍板。
- [`FrameProducer::next_frame()` 与 `Camera` 的 BREAKING 变更] → 在 proposal 显式标注；迁移 `SimProducer`、`render_frame.rs` 的 `Camera { aspect }` 字面量；用构造器 / `Default` 收窄破坏面；纳入 `cargo test --workspace` 验收。
- [P5 归档与 P6 apply 的顺序] → 本 change 不产生 `deterministic-simulation` delta；按用户拍板先归档 P5 再 apply P6（D8，归档由编排者随后执行，本轮不归档、不预断结果）。
- [表现层浮点（相机 / 插值 / UI 布局）被误纳入权威状态] → D2 / D5 / D4 全部规定 MUST NOT 写回、不进哈希与快照；以 `tests/isolation.rs` 与 `tests/determinism.rs` 作门禁。
- [跨平台确定性被误宣称达成] → 明确：本 change 不引入新的可比较算术来源，但 Linux / macOS 仍属待验证项（ADR 0002）。
- [owner 定义与「一个 change 一个 owner」] → 实施 owner 已指定 `client-owner`（由编排者新建 agent 定义，不在 `spec-steward` 可写范围），满足 CODEBUDDY.md §8，不再阻塞 `/opsx:apply`。
- [UI 字体 / 图标误入库或含暴雪资产] → ADR 0001 边界 + CI 资产扫描；UI 资源自带许可证或程序化生成，仓库不含 MPQ 派生数据。

## Migration Plan

1. **先决（规范侧，本 change 可写范围）**：完成 proposal / specs / design / tasks 四件套，`openspec validate p6-playable-client` 通过；同步 ADR 0003 与 `openspec/config.yaml`；`python tools/check_encoding.py` 通过。
2. **用户拍板（已完成，均选推荐）**：a = `egui` 家族（含 CODEBUDDY.md §10 的 MIT 依赖授权与后续 `NOTICE.md` 登记交 `docs-owner`）、b = `w3-net` 接入纳入本期（不缩减第 8 组任务与 `lockstep-networking` delta）、c = 新 crate 命名 `w3-client`、d = 先归档 P5 再 apply P6；实施 owner = `client-owner`；`config.yaml` 阶段枚举保持「P0-P5」。后续依赖重大升级 / 许可证变更仍由人工确认。
3. **归档 P5（由编排者随后执行，本轮不归档）**：`openspec archive p5-gameplay-systems -y`，确认主 `deterministic-simulation` / `gameplay-systems` specs 已更新（D8）。
4. **`/opsx:apply`（由 `client-owner` 执行，非本轮）**：新建 `w3-client`（命名已定案）；装配固定累加器 + `RendererApp`；迁移 `FrameProducer` / `Camera` 调用点；实现输入翻译、HUD、相机控制；实现 `ClientSession` → `FrameProducer` 适配（b 已拍板纳入本期）。
5. **验收门禁**：`cargo test -p w3-core --test determinism`（重跑 `crates/w3-core/tests/determinism.rs`）、`cargo test -p w3-render`（`tests/isolation.rs`）、`cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`python tools/verify_all.py`。
6. **回滚**：P6 全部改动限定在 `w3-client` / `w3-render` / `w3-net` 适配层，`w3-core` 零改动；回滚只需移除新 crate 与 `FrameProducer` / `Camera` 迁移，不影响权威状态与快照格式（不 bump `SNAPSHOT_VERSION`）。
7. **归档 P6**：`openspec archive p6-playable-client -y`，确认主 `openspec/specs/` 新增 `playable-client` 且 `3d-rendering` / `lockstep-networking` 已吸收 delta。

## Open Questions

> 用户对 a–d 四项待拍板决策已全部拍板（均选推荐），owner 归属、`config.yaml` 阶段枚举与 `CODEBUDDY.md` 措辞亦已结论。以下逐条结案，仅末尾保留真正未定的实现细节。

- **（a）UI 依赖选型 — 已由用户拍板**：定案 `egui` 家族（`egui` + `egui-wgpu` + `egui-winit`，取 MIT 一侧）；用户已按 CODEBUDDY.md §10 授权新增 MIT 依赖与后续 `NOTICE.md` 登记（交 `docs-owner`，本 change 不自行编写）。`wgpu 30` / `winit 0.30` 兼容性仍须在 apply 阶段核验，不兼容则明确报告并回退自绘 2D overlay，MUST NOT 静默降级或强行升级（见 D4）。
- **（b）第 5 项「`w3-net` 接入」是否纳入本期 — 已由用户拍板**：纳入本期，不缩减范围；保留第 8 组任务与 `lockstep-networking` delta（含 MODIFIED 块与两条 ADDED requirement）（见 D6）。
- **（c）新 crate 命名 — 已由用户拍板**：定案 `w3-client`（见 D7）。
- **（d）P5 归档与 P6 apply 的先后顺序 — 已由用户拍板**：先归档 P5、再 apply P6；`openspec archive p5-gameplay-systems -y` 由编排者随后执行，本 change 不自行归档、不预断 P5 已归档（见 D8）。
- **（e）owner 归属 — 已指定 `client-owner`**：由编排者新建该 agent 定义（不在 `spec-steward` 可写范围），满足 CODEBUDDY.md §8「一个 change 一个 owner」。
- **（f）`config.yaml` rules.proposal 的「（P0-P5）」— 已由用户拍板**：保持「P0-P5」不动，不改为「P0–P6」（P6 为集成收尾）；proposal 继续双标 P6 与 P3/P4/P5 能力面（见 D9）。
- **（g）`CODEBUDDY.md:49` / `CODEBUDDY.md:8` 过期措辞 — 已由用户拍板**：仍列为建议交 `docs-owner` 同步（「快照 v1」→ `SNAPSHOT_VERSION`；「P2–P5 active」「主 specs 仅两个」→ 实际状态），本 change 不可写该文件。

**仍未定（实现细节，留给 `client-owner`，不阻塞 apply）：**
- 累加器追帧上限的具体数值，以及超限是否暂停对局（D2 只要求「显式上限 + 明确报告」）。
- UI 字体 / 图标资源的具体来源（自带许可证或程序化生成，须满足 ADR 0001：不入库、不含暴雪资产 / MPQ 派生数据）。
