## Why

> 路线图阶段：P6（ADR 0003 新增的「集成收尾」阶段，复用 P3 `3d-rendering`、P4 `lockstep-networking`、P5 `gameplay-systems` 的既有能力，**不新增玩法阶段**；按 `config.yaml` 的 P0–P5 阶段划分，本 change 落在 P3/P4/P5 能力面的收尾）。相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期）、ADR 0004（冻结坐标）、ADR 0005（protobuf 载荷）。

P0–P5 已交付确定性内核、内容导入、玩法竖切、wgpu 呈现、lockstep 联机与完整玩法系统，但仓库里唯一的窗口入口 `crates/w3-render/examples/render_frame.rs` 仍是人工截图验收：其 `RedrawRequested → render_once() → next_frame() → sim.step()` 把逻辑帧绑死在显示器刷新率上（违反 ADR 0002 §2「`step()` 不接受 delta、墙钟或渲染帧率」），且没有键鼠输入、HUD、相机控制，`w3-net::ClientSession` 也接不进同一只呈现管线，第一版仍「不可玩」。用户已明确否决引入 Bevy，选择「A 方案」——复用既有 `winit 0.30` + `wgpu 30` + `w3-render`、加一个轻量 UI 依赖补齐交互层。用户对四项待拍板决策已全部拍板（均选推荐）：UI 依赖 = `egui` 家族（取 MIT 一侧，已按 CODEBUDDY.md §10 授权新增 MIT 依赖与后续 `NOTICE.md` 登记）、`w3-net` session 接入纳入本期（不缩减范围）、新 crate 命名定案 `w3-client`、实施 owner 已指定 `client-owner`。本 change 据此把已有能力收口成可玩客户端集成层；按拍板「先归档 P5、再 apply P6」推进，`openspec archive p5-gameplay-systems -y` 由编排者随后执行，本 change 不自行归档、不预断 P5 已归档；本轮仅写规范，代码留到 `/opsx:apply`。

## What Changes

- 新增 `playable-client` capability，描述客户端集成层契约：固定 30 Hz 步进、呈现插值、输入→命令、HUD、交互相机、客户端交互状态非权威、依赖/许可证闸门、本地与联机共用的 `FrameProducer` 接缝。
- **最高优先**：修复逻辑帧与重绘的绑定——改由整数帧累加器驱动 `RtsMatch::step()`，渲染用既有 `Scene::interpolate_all(alpha)` 呈现两帧之间的插值；`alpha` 只影响呈现，不进状态哈希。
- **BREAKING**：`FrameProducer::next_frame()` 语义从「推进一帧并返回只读视图」改为「只交出当前只读视图与事件拷贝」，不再推进模拟；现有 `SimProducer` 等实现者须迁移。
- 把 winit 键鼠事件翻译成 `Command` / `GameplayCommand`，只经 `submit_command` / `submit_gameplay_command` 提交；覆盖选中、移动、攻击、施法、生产 / 取消生产；表现层 MUST NOT 反写 `RtsMatch`（ADR 0002 §5）。
- 新增 HUD：选中框、血条、资源、生产队列，技能栏可选。UI 依赖已由用户拍板定案为 `egui` 家族（`egui` + `egui-wgpu` + `egui-winit`，`egui` 取 `MIT OR Apache-2.0` 的 MIT 一侧），用户已按 CODEBUDDY.md §10 授权新增该 MIT 依赖与后续 `NOTICE.md` 登记（交 `docs-owner`，本 change 不自行编写）；`egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30` 的兼容性仍须在 apply 阶段核验，不兼容时 MUST 明确报告并回退「自绘 2D overlay（无新依赖）」，MUST NOT 静默降级或强行升级 `wgpu` / `winit`；Bevy / Bevy UI 已否决。
- **BREAKING**：扩展 `w3-render::gpu::Camera`（当前仅 `aspect`）以支持平移 / 缩放 / 旋转；`Camera { aspect }` 结构体字面量调用点须迁移。
- 把 `w3-net::ClientSession` 适配成 `FrameProducer`（用户已拍板 b 纳入本期，不缩减范围，保留第 8 组任务与 `lockstep-networking` delta 的 MODIFIED 块及两条 ADDED requirement），令联机对局复用同一 `RendererApp::run()` / `Scene` / `GpuContext` 管线；网络帧到达 MUST NOT 驱动 30 Hz 累加器。
- 一致性维护：ADR 0003 分期表 P5 改为已完成、新增 P6 行、「明确不做」补「不引入通用游戏引擎（Bevy 等）」；`openspec/config.yaml` 的过期「快照 v1」措辞同步为 `SNAPSHOT_VERSION`（当前 3）；`config.yaml` rules.proposal 阶段枚举按用户拍板保持「P0-P5」不变。
- 本 change **不新增任何权威状态字段**，`SNAPSHOT_VERSION` 保持 3（含 `defs_hash` 头），`w3-core` 保持零改动，帧序 / 命令排序 / 状态哈希字段集合不变。

## Capabilities

### New Capabilities
- `playable-client`: 可玩客户端集成层契约——固定 30 Hz 步进与重绘解耦、呈现插值、输入到权威命令的翻译、只读 HUD、交互相机、客户端交互状态非权威、依赖与许可证闸门、本地与联机共用的 `FrameProducer` 接缝。

### Modified Capabilities
- `3d-rendering`: MODIFIED `Render from read-only simulation view`（逻辑帧推进与重绘解耦、`FrameProducer` 不再推进模拟、插值只影响呈现）；ADDED `Interactive camera control`（平移 / 缩放 / 旋转，相机状态为表现层专属）。
- `lockstep-networking`: ADDED 联网 session 接入同一呈现管线、网络帧与本地 30 Hz 累加器边界的要求（已拍板纳入本期，保留 MODIFIED 块与两条 ADDED requirement）；既有 `W3N1` 帧头、protobuf 编码约束与逐帧 checksum 契约不变。

## Impact

- **确定性契约（ADR 0002）**：`step()` 仍 MUST NOT 接受 delta / 墙钟 / 渲染帧率；累加器只数整数帧。`alpha`、相机、选中集合、HUD 状态、累加器余数一律为表现层专属，MUST NOT 进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` 头）定义的快照。本 change 不新增权威状态、不 bump 快照版本、不改帧序与命令排序，`w3-core` 零改动；`crates/w3-core/tests/determinism.rs` 与 `crates/w3-render/tests/isolation.rs` MUST 全绿。跨平台确定性目前仅在 Windows / rustc 1.91 实跑，Linux / macOS 仍属待验证项，本 change 不得声称已达成。
- **资产与法律边界（ADR 0001）**：不提交任何 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效；UI 字体 / 图标须自带许可证或程序化生成，不含暴雪资产。新增依赖前 MUST 检查许可证与 GPLv3 兼容性（`egui` 取 MIT 一侧），`Cargo.lock` MUST 提交，第三方代码须在 `NOTICE.md` 登记（交 `docs-owner`，列为后续任务，本 change 不自行编写）。用户已按 CODEBUDDY.md §10 拍板授权新增 MIT 依赖与后续 `NOTICE.md` 登记；后续 `wgpu` / `winit` 的依赖重大升级 / 许可证变更仍属人工独占，须重新拍板后方可实施。
- **分层依赖**：新增 crate `w3-client`（命名已由用户拍板定案）依赖 `w3-core` / `w3-render` /（可选）`w3-net` / `w3-content`；`w3-core` 绝不反向依赖上层；唯一权威写入者仍是 `RtsMatch`。
- **依赖版本风险**：`egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30`（当前 `default-features = false, features = ["rwh_06"]`）的兼容性仍须在 apply 阶段核验，本提案不锁定版本；不兼容时 MUST 明确报告并回退「自绘 2D overlay（无新依赖）」，MUST NOT 静默降级或强行升级 `wgpu` / `winit`。
- **待同步的一致性（不在本 change 可写范围）**：`CODEBUDDY.md:49` 的「快照 v1」过期措辞，以及 `CODEBUDDY.md:8` 的「P2–P5 为 active」「主 specs 仅两个」，建议交 `docs-owner` 同步（用户已确认保持为建议）。
- **owner 归属（已指定）**：实施 owner 已指定为 `client-owner`，由编排者新建该 agent 定义（不在 `spec-steward` 可写范围）；本 change 按 CODEBUDDY.md §8「一个 change 一个 owner」由 `client-owner` 承接，不再阻塞 `/opsx:apply`（见 `design.md` 的 Open Questions 结案记录）。
- **代码范围**：`crates/w3-render/`、新增 `crates/w3-client/`、`crates/w3-net/` 的适配层；本轮仅写 `openspec/` 与 `docs/decisions/`，不碰 `crates/`、不 commit、不归档、不执行远端命令。
