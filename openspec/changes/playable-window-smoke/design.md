# Design

## Context

见 `proposal.md` 的 Why（不重述动机）。P0–P6 已全部实施并归档（最新归档 `openspec/changes/archive/2026-11-11-gameplay-command-wire-format/`），主 `openspec/specs/` 共 11 个 capability。用户否决引入 Bevy，拍板「A 方案」：复用既有 `winit 0.30` + `wgpu 30` + `w3-render` + 新 `w3-client` + egui 0.37。

已核实的关键既有事实（见 `proposal.md` 与源码核对）：

- `crates/w3-client/` 只有 `src/lib.rs`（导出 `ClientApp` / `FixedStepClient` / `StepDriver` / `NetSessionDriver` / `HudModel` / `Texts` / `ZH` / `InputState` / `CameraState` 等）与 3 个 CPU 测试（`tests/replay.rs` / `interaction.rs` / `lockstep.rs`）；**无 `main.rs` / example / bin**。`ClientApp::run()`（`app.rs:210`）在全仓库无调用者，唯一构造点 `interaction.rs:63` 是 `ClientApp::new(driver, synthetic_terrain(), ModelLibrary::default(), 0)` 的 CPU 测试。真窗口路径启动不起来。
- `ClientApp<P: StepDriver>::new(driver, terrain: TerrainMesh, library: ModelLibrary, max_frames: u32) -> Self`：**签名不需要** `GpuContext` / `winit::window::Window`；`run()` 自建 `EventLoop::builder().build()`，窗口与 `GpuContext::new(&window, size.width, size.height)` 在 `ApplicationHandler::resumed` 内创建。`about_to_wait` 由 `HostClock` 整数 tick 驱动累加器，`step()` 全程无参，`Instant` 只在宿主层。呈现走 `GpuContext::render_frame_with_overlay`（含 egui HUD pass）。
- `crates/w3-render/examples/render_frame.rs` 是 P3 的只读截图 example：`fallback_terrain()` 用合成高度场、`SimProducer::new()` 用 `StaticWorld::open_world(16, 16, SimFixed::from_int(8))` + 6 条合成 `Spawn`、`RendererApp::run()` 简易只读循环（无累加器 / 输入 / HUD），`main()` 里 `GpuContext::new` 流程可作蓝本。
- `i18n.rs` 文案层已实现；本 change 启用 egui 0.37 `default_fonts` 内置开源字体，`FontDefinitions::default()` 的 `font_data` 非空，HUD 文字层正常绘制（D6）。内置字体无 CJK 字形，故文案为 ASCII，中文上屏留后续独立 change；字体许可证已在 `NOTICE.md` 登记。
- `crates/w3-client/Cargo.toml` 已依赖 `w3-core` / `w3-render` / `w3-content` / `w3-app`（`freeze_f64`）/ `w3-net` / `winit` / `wgpu` / `egui` 家族 / `log` / `thiserror`；含 `[target.'cfg(target_os = "linux")'.dependencies]` 的 `x11` 追加（`linux-winit-platform-backend` 已修好 Linux 平台后端）。
- `crates/w3-core/`：`SNAPSHOT_VERSION = 3`（`snapshot.rs:15`），`RtsMatch::create_with_world` / `submit_command` / `step()` / `compute_state_hash()` / `read_view()` 等公开 API，`w3-app::freeze_f64`（`freeze.rs:34`）。`crates/w3-core/tests/determinism.rs`（26 项）与 `crates/w3-render/tests/isolation.rs`（3 项）是硬门禁。
- 当前仅 1 个 active change（`playable-window-smoke`）；`linux-winit-platform-backend` / `cross-platform-golden-hash` / `gameplay-command-wire-format` 均已归档，其 capability（`linux-platform-backend` / `cross-platform-determinism` / `gameplay-command-wire`）已进入主 `openspec/specs/`。CI 为 `.github/workflows/ci.yml` 的 `ubuntu-latest` / `windows-latest` matrix 跑 `python tools/verify_all.py`（CODEBUDDY.md §2：只验证**工具链**跨平台，不等于模拟内核确定性已达成）。

约束：确定性硬门禁（ADR 0002）、资产法律边界（ADR 0001）、范围与否决引擎（ADR 0003）、冻结坐标（ADR 0004）、线格式分层（ADR 0005）。架构原则（config.yaml context）：SRP / OCP / LSP / DIP / ISP / LoD / CARP，正确性 / 确定性优先；硬红线：唯一状态所有者、`w3-core` 不反向依赖上层、显式失败不静默降级、只读参考不复制代码。规范先行；`/opsx:apply` 已按用户拍板实施（第 2–11 组），并经四轮真窗口反馈逐轮扩展范围。

## Goals / Non-Goals

**Goals:**

- 补一个零资产、零新依赖、可编译可运行的最小入口，把 P6 真窗口路径（输入 / HUD / 相机 / 30 Hz 累加器）真正启动起来。
- 提供可逐项执行的人工 GPU 冒烟清单，并划清「人工验收」与「CI 编译 + CPU 测试」的边界。
- 全部复用既有 `ClientApp` 装配与公开 API，不新增权威状态、不改哈希 / 快照、不改既有 API。
- 新增独立 capability `playable-window-entry`，不产生其它 capability 的 MODIFIED delta（避免与其它 active change 争用）。

**Non-Goals:**

- 引入通用游戏引擎（Bevy 等）、新 crate、`[[bin]]` 或任何新依赖（ADR 0003「明确不做」）。
- 中文 HUD 标签上屏 / 引入字体资产（CODEBUDDY.md §10 人工独占，后续独立项）；`default_fonts` 内置开源字体已在本 change 启用、文案为 ASCII（D6）。
- 读取 `config.toml`、真实导出 JSON / MPQ 派生数据 / `assets-export/` / `slk-exported/` 或任何暴雪资产（ADR 0001）。
- 改 `w3-core` 帧序、命令排序、状态哈希编码、快照字段或 `SNAPSHOT_VERSION`；新增权威状态字段。
- 把 GPU 冒烟写进 CI 自动化、或用 CPU 测试 / headless 软件渲染冒充 GPU 验收。
- 声称全平台 / 全 CPU 跨平台确定性已达成（ADR 0002：目前仅 Windows 与 Debian 12 x86_64 / rustc 1.99.1 已验证，macOS / 其它 CPU 架构 / 其它 rustc 仍待验证）。
- 范围边界：代码改动仅落在 `crates/w3-client/` 与 `crates/w3-render/`（`w3-core` 零 diff），另含 `w3-client/Cargo.toml`、`Cargo.lock`（仅新增传递包）与 `NOTICE.md` 许可证登记；不手写主 `openspec/specs/` / 其它 change 目录 / `docs/` / `.codebuddy/` / `CODEBUDDY.md` / 根配置。本地 commit 由 AI 执行，`push` / `fetch` / `pull` / `clone` 与归档 `/opsx:archive` 一律由人工 / 编排者执行（CODEBUDDY.md §4 / §9）。

## Decisions

### D1：入口落点与形态——cargo example `examples/playable.rs`

- **决策**：新增 `crates/w3-client/examples/playable.rs`，命令 `cargo run -p w3-client --example playable`。example 是 `w3-client` 包内 target，不进 `lib`、不改 `Cargo.toml` 依赖段、不新增 `[[bin]]` 或 `[workspace]` 成员、不引入第二套 crate 边界。`ClientApp::run()` 成为本 example 的唯一真实调用者。
- **备选方案（Alternatives considered）**：
  - *新增 `[[bin]]`（`src/main.rs` 或 `src/bin/playable.rs`）* — **否决**。会把「可玩客户端」语义从 lib 扩成双 target，需在 `Cargo.toml` 增加 bin 段；bin 与 lib 的依赖形状易被误读为「存在第二个可执行入口」，且 P6 的 `ClientApp` 已是 lib 的公开装配点，bin 无额外收益。
  - *扩展 `crates/w3-render/examples/render_frame.rs`* — **否决**。该文件是 P3 的只读截图循环（`RendererApp::run()`），无累加器 / 输入 / HUD / 相机交互；把 P6 的客户端装配塞回 `w3-render` 会让渲染层反向依赖 `w3-client`，破坏分层（D2 / D7）。
  - *新建小 crate（如 `w3-playable`）只放入口* — **否决**。需改 `[workspace].members`、`Cargo.toml`、`Cargo.lock`，新增依赖边与许可证复核面，为一个 `main()` 扩大 owner 边界，违反 CARP / SRP 与「零新依赖」目标。
  - *cargo example（本决策）* — **采纳**。改动面最小（一个文件）、自动被 `cargo build --examples` / CI 编译覆盖、`w3-client` 依赖结构与 `[lib]` 零改动，且 `crates/w3-core/examples/golden_hash.rs` / `crates/w3-render/examples/render_frame.rs` 已是同仓库既有惯例。
- **关联 ADR 0002 与跨平台确定性影响**：入口形态纯属构建产物，不进入权威状态、不新增可比较算术来源；`logic_fps = 30` 整数帧、稳定排序、FNV-1a 哈希、`SNAPSHOT_VERSION` v3 快照契约零改动。example 可编译不等于「模拟内核跨平台确定性已验证」；Linux 侧仅由 `linux-winit-platform-backend` 提供 winit `x11` 后端编译前提，本决策不扩大 `cross-platform-golden-hash` 的已验证平台范围。
- **架构原则与分层依赖**：CARP（复用既有 `ClientApp` / crate 边界，不重写）、SRP（入口只做装配，呈现归 `w3-render`、时钟归宿主）、ISP（example 只见窄公开 API）。依赖方向 `example → w3-client → w3-render / w3-core / w3-app`，`w3-core` 不反向依赖上层硬红线不变。

### D2：装配方式——复用 `ClientApp::new(...).run()` + 本地 `FixedStepClient`

- **决策**：example 只构造 `FixedStepClient::from_match(RtsMatch::create_with_definitions(config, 合成 StaticWorld, 合成 FrozenDefinitions))`（合成命令脚本与冻结玩法定义，世界原点与地形中心对齐；见 D13）、`TerrainMesh`（合成，瓦片边长与世界格距一致）、插入内存占位模型的 `ModelLibrary`（零资产，使 `Scene` 生成单位占位标记，供人工清单框选验收）、`max_frames`，再调用 `ClientApp::new(driver, terrain, library, max_frames).run()`。**不新写宿主循环**。已由源码核实：`ClientApp::new` 的签名不需要 `GpuContext` / `Window`；窗口与 `GpuContext::new(&window, size.width, size.height)` 在 `ClientApp::resumed` 内创建（流程与 `render_frame.rs:190` 一致），example 无需自行构造二者。`max_frames` 是既有「限制呈现帧数便于无人值守验收」的参数，取足够大的值供人工交互，仍属表现层。
- **备选方案（Alternatives considered）**：
  - *在 example 里自建 `EventLoop` / `ApplicationHandler` 复刻 `ClientApp` 循环* — **否决**。出现复制的事件循环与第二套呈现代码，违反 SRP / CARP，且会把「时钟归宿主 / 呈现只读」的边界打散，最容易把 `RedrawRequested` 误接回 `step()`（ADR 0002 §2）。
  - *扩展 `RendererApp::run()`（`w3-render`）承载累加器 / 输入 / HUD* — **否决**。`RendererApp` 是消费 self 的封闭只读循环，无节拍源 / 输入 / 相机 / HUD 注入点；把客户端时钟塞回 `w3-render` 会让渲染层依赖 `w3-client`，破坏分层（P6 design D6 已裁定）。
  - *在 example 内另建一份 `RtsMatch` 直接 `step()` 再喂给自建 Scene* — **否决**。出现第二个权威状态所有者与第二条 `step()` 路径，直接违反 ADR 0002 §5 与「唯一状态所有者」硬红线。
  - *复用 `ClientApp` 既有装配* — **采纳**。全程只有一个 `RtsMatch`、一条无参 `step()` 路径、一套 `Scene` / `GpuContext` / `Camera` 呈现管线。
- **关联 ADR 0002 与跨平台确定性影响**：累加器只做整数 tick → 整数帧，`step()` 不接受 delta / 墙钟 / 刷新率；`about_to_wait` 是唯一节拍源，`Resized` 只改交换链与 aspect。插值 `alpha`、相机、选中、HUD、`Instant` 全是表现层，MUST NOT 进 FNV-1a 哈希或 v3 快照。本决策不新增可比较算术来源，跨平台确定性结论不变（仍为待验证项，不得扩大）。
- **架构原则与分层依赖**：SRP / DIP（example 依赖 `ClientApp` 抽象装配而非内核内部状态）、LoD（只与公开 API 打交道）、LSP（本地 `FixedStepClient` 实现 `StepDriver`）。`w3-core` 零改动、冻结输入边界（`w3-content` 的 `f64` 只经 `w3-app::freeze_f64`）不受影响。

### D3：无资产与冻结边界——合成数据，`freeze_f64` 单一入口

- **决策**：example 用合成高度场（蓝本 `render_frame.rs::fallback_terrain`）与合成 `Spawn` 命令脚本（蓝本 `determinism.rs` 的 `open_world` / `script()`），固定 `MatchConfig { seed, logic_fps: 30, content_hash }` 全部用整数常量。MUST NOT 读 `config.toml`、导出 JSON、`assets-export/`、`slk-exported/`、`models/*.json`、PNG / GLB / 音效或任何暴雪资产 / MPQ 派生数据。任何 `f64 → SimFixed` 一律经 `w3-app::freeze_f64`（ADR 0004）。人工产物（截图 / 日志）落在已 gitignored 的 `target/` 等本机路径，不入库。
- **备选方案（Alternatives considered）**：
  - *复用 `render_frame.rs` 的可选 `<assets-dir>` 参数读本机导出数据* — **否决**。会让 example 依赖玩家本机 `config.toml` / 导出资产，破坏「在任何干净环境可跑」与 CI 编译验证，触碰 ADR 0001 资产红线（即使不入税也混入了资产路径依赖）。
  - *在仓库内嵌一份最小地形 / 单位 JSON 或程序化生成后写盘* — **否决**。任何派生数据文件入库都违反 ADR 0001 与 `.gitignore` / CI 资产扫描；生成逻辑应在内存完成，不落盘。
  - *直接用 `v * 65536.0` / `SimFixed::from_raw` 把合成坐标转成定点* — **否决**。新增第二条 `f64 → SimFixed` 路径，绕过 ADR 0004 §3 的纯整数 IEEE-754 位分解冻结边界，可能引入平台浮点差异。
  - *合成数据 + `w3-app::freeze_f64`* — **采纳**。零资产、零文件 IO、冻结边界唯一、干净环境可跑。
- **关联 ADR 0002 与跨平台确定性影响**：合成数据只是冻结输入的构造方式，`f64` 始终停留在冻结边界之外、不进快照与状态哈希；`MatchConfig` 与命令脚本全为整数常量，相同源码在任何平台产生相同权威状态。跨平台确定性结论不变；本决策不引入新的可比较算术来源。
- **架构原则与分层依赖**：DIP（example 只依赖 `w3-app` 的冻结函数抽象）、唯一状态所有者（数据注入 `RtsMatch::create_with_definitions` 而非反写字段）、显式失败（`freeze_f64` 对 NaN / Inf / 溢出返回 `FrozenError`，不静默截断）。`w3-core` 不依赖 `w3-content` / 上层硬红线不变。

### D4：不改权威状态——只调公开 API，`w3-core` 零改动

- **决策**：example 只调用 `w3-core` / `w3-client` / `w3-render` / `w3-app` 既有公开 API（`create_with_definitions` / `submit_command` / `step()` / `compute_state_hash()` / `read_view()` / `capture_snapshot()` / `freeze_f64` 等）。`crates/w3-core/` 零改动；**不新增权威状态字段**、不 bump `SNAPSHOT_VERSION`（当前 3）、不改 FNV-1a 状态哈希字段集合、不改帧序、不改 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、不改快照头部布局（含 `defs_hash` / `content_hash`）。`step()` 无参、整数帧 30 Hz。客户端表现层状态（累加器余数 / `alpha` / 相机 / 选中 / HUD / 窗口尺寸 / `Instant`）一律不进哈希或快照。
- **备选方案（Alternatives considered）**：
  - *为「冒烟已运行 N 帧」在内核加一个计数器字段* — **否决**。新增权威状态必须同时进哈希与快照并 bump 版本（config.yaml rules.specs），直接违反 ADR 0002 §6 / §7 与本 change「零改动内核」前提。
  - *把 `max_frames` / 窗口尺寸写进 `WorldView` 供 HUD 读取* — **否决**。扩展 `WorldView` 等于改 `w3-core` 公开形状，且让表现层参数进入权威视图。
  - *让 example 持有 `&mut RtsMatch` 直接推进* — **否决**。`RtsMatch` 字段全私有、`read_view()` 返回 owned 数据，类型系统已封死此路；强开会破坏「唯一写入者」（ADR 0002 §5）。
  - *只调公开 API、权威状态零改动* — **采纳**。P6 `interaction.rs` 已用同一公开 API 证明 HUD / 相机 / 输入生命周期不改哈希与快照字节，本 change 沿用。
- **关联 ADR 0002 与跨平台确定性影响**：状态哈希字段集合与 v3 快照头部逐字节不变，逐帧 `compute_state_hash()` 与 CPU 基线一致；若将来把新字段转为权威，MUST 同时进入哈希与快照并先让 `crates/w3-core/tests/determinism.rs` 通过——本 change 不行使该例外。跨平台确定性结论不变。
- **架构原则与分层依赖**：唯一状态所有者 / `w3-core` 不反向依赖上层（硬红线）、OCP（通过既有公开入口扩展而非侵入内核）、LoD（表现层只见 owned 视图与事件拷贝）。`w3-content` 的 `f64` 冻结输入边界不受影响。

### D5：GPU 冒烟验收清单（人工项）与 CI 边界

- **决策**：在 spec 与 tasks 中给出可逐项勾选的**人工** GPU 冒烟清单，至少含：① 窗口能开；② `GpuContext::new` 成功；③ egui HUD overlay pass 正常提交；④ HUD 文字（含按键提示）以 ASCII 正常显示（egui 0.37 `default_fonts`）；⑤ 左键框选 / 右键移动 / Q 施法 / T 生产 / C 取消；⑥ WASD 平移 / 滚轮缩放 / Z-X 旋转；⑦ `Resized` 后画面正常且逻辑帧不跳；⑧ 高刷显示器下逻辑帧恒定 30。清单项 MUST 标注**只能人工在有 GPU 与显示设备的桌面执行**；无 GPU / headless / CI runner MUST 明确报告无法执行、不静默跳过。CI 只覆盖编译（`cargo build --examples`）+ CPU 测试（`determinism.rs` 26 项 / `isolation.rs` 3 项 / `verify_all.py`），**MUST NOT 声称 CI 已验证 GPU 链路**。
- **备选方案（Alternatives considered）**：
  - *把 GPU 冒烟写进 CI：无头窗口 + `capture_rgba` 截图自动比对* — **否决**。CI runner 无稳定 GPU / 显示设备，`GpuContext::new` 可能失败；即便用软件渲染（llvmpipe / lavapipe）也不是目标用户的 GPU 路径，且截图入税触碰 ADR 0001。CODEBUDDY.md §2 明确 CI 双平台只验证工具链。
  - *复用 `render_frame.rs` 的 `readback::write_png` 做「GPU 验收」* — **否决**。那是 P3 只读截图 example，证明的是「能渲染一帧」，不含累加器 / 输入 / HUD / 相机 / 30 Hz 真窗口路径，不能替代人工冒烟。
  - *把 P6 的 CPU 测试（`replay` / `interaction` / `lockstep`）当作 GPU 验收已覆盖* — **否决**。它们不开窗口、不初始化 `GpuContext`、不提交 egui overlay pass，正是本 change 要补的缺口；混同会让「真窗口从未跑过」的缺口被掩盖。
  - *纯人工清单 + 明确 CI 边界* — **采纳**。承认现有验收的边界，把不可自动化的部分显式交人工。
- **关联 ADR 0002 与跨平台确定性影响**：清单⑧（高刷恒定 30）与⑦（Resize 不跳帧）是 ADR 0002 §2 的真窗口复核，属**呈现层观测**，不新增可比较算术来源；逐帧状态哈希一致性仍由 CPU 测试把关。本决策不把 Linux / macOS 列为「GPU 链路已验证」；CI 双平台绿灯不等于模拟内核确定性已达成，也不等于 GPU 路径已跑通。
- **架构原则与分层依赖**：显式失败不静默降级（无 GPU 明确报告）、单一权威（验收结论以人工清单为准，不以 CI 绿灯偷换）、SRP（清单属规范 / 任务，不写进 `crates/`）。owner 写范围见 D7。

### D6：字体与许可证边界——启用内置开源字体，中文上屏留后续

- **决策**：启用 egui 0.37 的 `default_fonts` feature（用户第三轮拍板）。`epaint_default_fonts` 在编译期内嵌开源字体（`(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`），`FontDefinitions::default()` 的 `font_data` 非空，`HudAssets::prepare` 返回 `text_enabled=true`，HUD 文字层（资源 / 人口 / 生产队列 / 按键提示）正常绘制。内置字体无 CJK 字形，故 `i18n` 文案改为 ASCII；中文上屏 MUST 留作后续独立 change。许可证变更由程序员本人拍板（CODEBUDDY.md §10 人工独占），`NOTICE.md` 登记字体许可证，`Cargo.lock` 仅新增 `epaint_default_fonts` 传递包；**不复制任何字体文件进仓库**、不读取本机字体路径（ADR 0001）。
- **备选方案（Alternatives considered）**：
  - *维持 P6 的字体降级（不启用 `default_fonts`）* — **否决**。HUD 完全无文字，用户不知道按什么键操作，人工清单 4.3 / 4.4 无法验收。
  - *内嵌一份 CJK 字体文件进仓库* — **否决**。字体文件是资产，入库违反 ADR 0001 的资产边界且需额外许可证复核；本期以 ASCII 文案先保证可见。
  - *运行时读取本机系统字体* — **否决**。回到 `config.toml` / 本机路径依赖（D3 已否决），且系统字体分发许可不明。
- **关联 ADR 0002 与跨平台确定性影响**：字体 / 文案纯属表现层，UI 文案 MUST NOT 进日志 / 权威状态 / 状态哈希（CODEBUDDY.md §3）；文字绘制不改变模拟推进、不改哈希与 v3 快照。本决策不新增可比较算术来源，跨平台确定性结论不变。
- **架构原则与分层依赖**：ISP（HUD 只读派生）、显式失败不静默降级（字体数据为空仍 `EmptyFontData` 报错 + 英文日志）、OCP（中文上屏以独立 change 扩展）。新增依赖前检查 GPLv3 兼容并提交 `Cargo.lock`（ADR 0001）——本 change 已核对字体许可证并在 `NOTICE.md` 登记。

### D7：owner 归属——单一 `client-owner` + 只读 `critic`

- **决策**：实施 owner = **`client-owner`**（单一 owner）。其 `.codebuddy/agents/client-owner.md` 声明可写 `crates/w3-client/` 与 `crates/w3-render/`，正好覆盖 `crates/w3-client/examples/playable.rs`；配只读 `critic`（审查确定性、所有权、许可证、编码、资产入库）。`spec-steward` 本轮只写 `openspec/changes/playable-window-smoke/`，不写 `.codebuddy/agents/`（不在可写范围，也无需新建 agent）。满足 CODEBUDDY.md §8「一个 change 一个 owner」+ 只读 critic。
- **备选方案（Alternatives considered）**：
  - *多 owner（client-owner 写 example、render-owner 改 `w3-render`、docs-owner 改文档）* — **否决**。本 change 只新增一个 example 文件，`w3-render` / `docs/` / 主 specs 零改动，多 owner 反而制造争用与职责重叠。
  - *由 `spec-steward` 直接写 example* — **否决**。超出 `spec-steward` 可写范围（`openspec/` / `docs/decisions/`），且 Rust 源码需 `cargo fmt --all` 门禁，属 `client-owner` 职责。
  - *交 `net-owner` / `sim-owner`* — **否决**。本 change 不碰 `w3-net` / `w3-core`。
  - *单一 `client-owner` + 只读 `critic`* — **采纳**。改动面与 owner 边界完全对齐。
- **关联 ADR 0002 与跨平台确定性影响**：owner 归属是流程约束，不改变代码与可比较量；`critic` 复审须覆盖「`w3-core` 零 diff、哈希 / 快照不变、无资产入库、无新依赖、编码基线、中文提交信息」。跨平台确定性结论不变。
- **架构原则与分层依赖**：单一权威 / 一个 change 一个 owner；`critic` 只读不写（CODEBUDDY.md §9）。实施期 Rust 源码遵循 rustfmt 默认风格、写完 `cargo fmt --all`、提交前 `cargo fmt --all -- --check`（CODEBUDDY.md §2 / §9）。

### D8：归档顺序与 delta 落点——新增独立 capability，无 MODIFIED delta

- **决策**：本 change 只新增 capability `playable-window-entry`（全部 ADDED Requirements），**不产生任何既有 capability 的 MODIFIED delta**。理由：主 `openspec/specs/` 共 11 个 capability 的需求无行为变更，本 change 是 `playable-client` 既有要求的「行使」而非改写；当前仅 1 个 active change（`playable-window-smoke`），`linux-winit-platform-backend` / `cross-platform-golden-hash` / `gameplay-command-wire-format` 均已归档，其 capability（`linux-platform-backend` / `cross-platform-determinism` / `gameplay-command-wire`）已进入主 specs、不再持有 active delta，归档时无争用。归档顺序：P6 → winit → golden-hash → gameplay-command-wire-format 已按裁定顺序执行归档；本 change 与已归档的 `gameplay-command-wire-format` **无依赖**（不改 `w3-net` / protobuf / `PROTOCOL_VERSION`），互不阻塞，可各自独立推进。
- **备选方案（Alternatives considered）**：
  - *对 `playable-client` 产生 MODIFIED delta，把「存在可运行入口」写进主 spec* — **否决**。主 `playable-client` 已规定 `ClientApp` 装配 / 单一呈现管线 / 状态非权威，本 change 不新增能力面行为；且 MODIFIED 会与任何后续写该 capability 的 change 冲突。
  - *把 requirements 挂到 `3d-rendering` 或 `deterministic-simulation` 下* — **否决**。入口既非渲染能力面行为变更，也不改确定性契约；后者还与 `cross-platform-golden-hash` 的 `cross-platform-determinism` delta 邻接，易生口径冲突。
  - *与 `gameplay-command-wire-format` 合并成一个 change* — **否决**。两者 owner 不同（`client-owner` vs `net-owner`）、改动面不同（`w3-client` example vs `w3-net` / `w3-relay` protobuf）、BREAKING 与否不同，合并违反「一个 change 一个 owner」。
  - *新增独立 capability、P6 之后归档* — **采纳**。与 `gameplay-command-wire-format` 的 (h) / D8 口径（刻意不碰 `lockstep-networking`）一致。
- **关联 ADR 0002 与跨平台确定性影响**：纯规范文档操作，不改变代码、帧序、哈希或快照字段；归档只把 `playable-window-entry` 写入主 `openspec/specs/`，不改 `deterministic-simulation` / `playable-client` 的口径，避免与 `config.yaml` 的 `SNAPSHOT_VERSION` v3 表述打架。跨平台确定性结论不变。
- **架构原则与分层依赖**：单一权威（主 specs 为 source of truth，CODEBUDDY.md §8）、一个 change 一个 owner；归档由编排者在人工 GPU 冒烟通过 + `critic` 复审 + 本地 commit + 人工 push + CI 双平台门禁之后执行（`/opsx:archive`），AI 侧不执行任何远端同步命令（CODEBUDDY.md §4 / §9）。

### D9：拾取映射与呈现投影一致（范围扩展后新增）

- **问题**：首次人工 GPU 冒烟暴露 P6 既有缺陷——`ScreenMapper`（`w3-client/src/input.rs`）是轴对齐线性正交映射，3D 单位却由 `Camera::matrix`（`w3-render/src/gpu/mod.rs`，`look_at_rh` + `perspective_rh` 斜视透视）绘制。同一世界点在两套映射下的屏幕位置差 100–250px，远超 `PICK_RADIUS = 24px`；框选 / 施法必然失败，HUD 血条 / 选中框与 3D 单位分离。合成数据的中心对齐只能让原点重合，除中心点外系统性错位仍在；单位占位标记中心与地面点还差约 50px。
- **决策**：`w3-render` 新增 `ScreenProjection`，由与 `Camera::matrix` 相同的 `Camera` / `TerrainMesh` 纯函数构造 `view_proj` 及逆矩阵；`w3-client::ScreenMapper` 增加可选 `projection` 字段并委托，`ClientApp::mapper()` 注入，`ScreenMapper::new` 签名不变（默认 `None` 保持既有 CPU 测试口径）。`world_to_screen` 以 `scene::UNIT_MARKER_SIZE * 0.8`（单位占位标记中心平面）为基准，`screen_to_world` 以 y=0 地面为基准，各司其职；`scene::UNIT_MARKER_SIZE` 提为常量避免魔法数。合成地形起伏压到很浅（0.15）以消除贴地高度偏差。
- **备选方案（Alternatives considered）**：
  - *改 `Camera::matrix` 为正交俯视以迁就线性映射* — **否决**。视觉退化为 2.5D、Z-X 旋转失效，且改动 P3 / P6 既有呈现基线。
  - *在 `ScreenMapper` 内复刻投影公式* — **否决**。复制 `Camera::matrix` 常量易失同步，违反 CARP。
  - *给 `w3-client` 引入 `glam` 直接算矩阵* — **否决**。新增依赖违反零新增依赖目标，且 `glam` 已由 `w3-render` 封装。
  - *仅放宽 `PICK_RADIUS` 或忽略单位标记高度* — **否决**。掩盖系统性错位而非修复。
- **关联 ADR 0002 与跨平台确定性影响**：投影矩阵与逆矩阵全为表现层 `f32` / `f64`，`w3-core` 零改动、不新增权威状态、不进 FNV-1a 哈希或 `SNAPSHOT_VERSION` v3 快照；`step()` 无参、`Resized` 不推进逻辑帧的边界不变。拾取不引入新的可比较算术来源，跨平台确定性结论不变。
- **架构原则与分层依赖**：CARP（复用 `Camera::matrix` 同源矩阵而非复制）、OCP（通过新增公开项扩展而非改既有签名）、ISP（`ScreenMapper` 只见窄投影接口）、LoD（`ClientApp` 统一构造并注入）。owner 仍为单一 `client-owner`，改动落在 `crates/w3-client/` / `crates/w3-render/`。
- **验收**：`w3-render` 新增 `screen_to_world_inverts_ground_projection` 与 `projection_matches_camera_matrix`；`w3-client` 新增 `selection_uses_the_render_projection`，`interaction.rs` 改用 `ClientApp::screen_mapper()` 统一口径；人工清单 4.4 待用户重跑逐项勾选。

### D10：拾取按单位屏幕包围盒判定（第三轮反馈）

- **问题**：投影同源后 `Cast` 仍报 `no target entity picked`，但错误是 `NoTarget` 而非 `NoSelection`，说明框选已命中己方单位，失败在敌方拾取。`pick_enemy` / `commit_selection` 用 `PICK_RADIUS = 24px` 固定半径，而单位占位四面体（`UNIT_MARKER_SIZE = 24` 世界单位）在屏幕上有 200–400px，用户点击四面体大部分区域都被判为未命中；HUD 选中框又硬编码 40×40（`hud.rs`），与实体尺寸严重失配，视觉上「没结合」。
- **决策**：`ScreenProjection::unit_screen_bounds` 投影四面体 apex 与底面四角（保守覆盖任意朝向）得到屏幕包围盒；`ScreenMapper::unit_screen_bounds` 委托（未注入投影时退化为中心 ± `PICK_RADIUS` 供 CPU 测试）；`commit_selection` / `pick_enemy` 改用包围盒判定，单击 / 拖框阈值与拾取半径解耦；`HudUnit` 增加 `bounds_min` / `bounds_max`，选中框按包围盒绘制。包围盒只存在于表现层，不进权威状态 / 哈希 / 快照。
- **备选方案**：*仅放大 `PICK_RADIUS`* — **否决**，会让「单击 vs 拖框」阈值同步放大，拖小框被误判为点击；*按单位距离排序取最近* — **否决**，仍漏点击边缘。
- **验收**：`interaction.rs` 的空地点用例改取远离包围盒的点以保证右键走 `Move`；`selection_uses_the_render_projection` 与全量 CPU 测试全绿；人工清单 4.4 待用户重跑。

### D11：默认 1920×1080 窗口（第三轮反馈）

- **决策**：`ClientApp::resumed` 创建窗口时用 `with_inner_size(LogicalSize::new(1920.0, 1080.0))`，并输出英文结构化 `window created: width=... height=... scale_factor=...` 诊断日志，便于人工验收与 DPI 排查。尺寸纯属表现层，不进权威状态 / 哈希 / 快照；既有公开签名不变。
- **备选方案**：*新增 `ClientApp::with_window_size` 链式配置* — **否决**，为本 change 唯一的冒烟入口增加配置面，超出「补入口」范围；*依赖 winit 平台默认尺寸* — **否决**，窗口过小不便验收。

### D12：高 DPI 坐标空间统一（第四轮反馈）

- **问题**：用户窗口逻辑 1920×1080、`scale_factor=1.5`、物理 2880×1620。已核对 winit 0.30.13 registry 源码：`CursorMoved.position` 是 `PhysicalPosition<f64>`；`GpuContext` 与 `Camera::matrix` 也在物理空间；但 egui 用逻辑空间（`pixels_per_point=1.5`）。`HudModel::collect` 把物理坐标直接写进 `HudUnit`，`hud::draw` 交给 egui 后整体放大 1.5 倍 → 选中框 / 血条飘在四面体右下方。参考项目（Godot `rts_camera.gd` 的 viewport 投影、Warsmash 的 LibGDX `unproject`、vera20k `entity_pick` 的「拾取基于已绘制对象」）都让输入与 UI 共用同一坐标空间，或显式做屏幕→世界反投影。
- **决策**：`ScreenMapper` 增加 `pixels_per_point` 字段（默认 1.0）与 `with_pixels_per_point` / `to_logical` / `to_logical_bounds` / `logical_size`；`ClientApp::mapper()` 从 `window.scale_factor()` 注入（CPU 测试无窗口回落 1.0，既有断言不变）；`HudModel::collect` 把 `screen` / `bounds_*` 换算为逻辑坐标；`hud::draw` 为所有存活单位绘制队伍色标记与血条、选中单位加绿框，敌方单位不再隐形。`world_to_screen` / `unit_screen_bounds` 仍返回物理坐标供输入拾取使用，二者各司其职。
- **关联 ADR 0002 与跨平台确定性影响**：换算与坐标全为表现层 `f64` / `f32`，`w3-core` 零改动，不进权威状态 / 哈希 / v3 快照。
- **验收**：`logical_coordinates_are_physical_divided_by_scale_factor`（与逻辑视口投影逐位一致）、`hud_marks_all_alive_units_in_logical_coordinates`、`physical_mouse_picks_enemy_under_high_dpi` 三个 CPU 回归。

### D13：合成冻结玩法定义（第四轮反馈）

- **问题**：`FixedStepClient::new` → `RtsMatch::create_with_world` 使用 `FrozenDefinitions::empty()`（sim_match.rs:401）；`execute_cast` 查不到 ability 10 直接 return，`enqueue_production` 查不到 unit_def 1 时 `train_frames = 0 → 1` 帧几乎不可见 → Q / T / C 全部「没效果」。
- **决策**：example 改用 `RtsMatch::create_with_definitions` + `FixedStepClient::from_match`，注入纯内存合成的 `UnitDef`（`unit_def = 1`，`train_frames = 30`）与 `AbilityDef`（`ability = 10`，`cast_frames = 3`），与 `w3_client::input` 的 `DEFAULT_UNIT_DEF` / `DEFAULT_ABILITY` 对齐；仍为零资产、零新增依赖、只调公开 API。
- **关联 ADR 0002 与跨平台确定性影响**：冻结定义进入 `defs_hash` / 快照头部属内核既有行为，本 change 不改 `w3-core`、不新增权威状态字段、不 bump `SNAPSHOT_VERSION`。
- **验收**：`frozen_definitions_make_train_and_cast_effective`（内核层）与 `client_app_train_command_reaches_the_kernel`（ClientApp 输入层）两个 CPU 回归。

## Risks / Trade-offs

- [包围盒过大导致点空地误判为攻击单位] → 包围盒与 3D 四面体视觉范围一致，点中单位即攻击属 RTS 正常语义；空地右键的 CPU 用例守住「远离包围盒仍生成 Move」（D10）。

- [拾取 / 呈现投影再次漂移] → `ScreenProjection` 与 `Camera::matrix` 由同一 `Camera` / `TerrainMesh` 纯函数构造，`projection_matches_camera_matrix` 测试守住同源；`Camera::matrix` 常量若变更，测试 MUST 同步（D9）。

- [无 GPU / 无显示设备环境（headless VM、CI runner、远程桌面无硬件加速）导致清单无法执行] → 清单项显式标注「只能人工在有 GPU 与显示设备的桌面执行」；无此环境 MUST 明确报告无法执行、不静默跳过、不改判为通过；清单未逐项勾选前不得归档（D5 / spec Scenario「人工边界与无 GPU 环境」）。
- [把 CI 双平台绿灯误当 GPU 冒烟已验证] → proposal / spec / tasks 三处写明 CI 只覆盖编译 + CPU 测试（`verify_all.py`），CODEBUDDY.md §2 明确 CI 只验证工具链跨平台；MUST NOT 声称 CI 已验证 GPU 链路，也 MUST NOT 据 CI 声称模拟内核确定性已达成（D5）。
- [字体许可证（OFL-1.1 / Ubuntu-font-1.0）合规边界被突破，或字体 / 图标资产入库] → 启用 `default_fonts` 已由程序员本人拍板（CODEBUDDY.md §10 人工独占），许可证在 `NOTICE.md` 登记、`Cargo.lock` 仅新增 `epaint_default_fonts` 传递包、不复制任何字体文件进仓库；中文上屏留后续独立 change；`.gitignore` + CI 资产扫描继续把关（D3 / D6）。
- [真窗口呈现帧率与逻辑帧混淆：把 `RedrawRequested` / VSync 当成 30 Hz 节拍，或 Resize 触发 `step()`] → 复用 `ClientApp` 的 `HostClock` 整数 tick 累加器，`step()` 无参、`Resized` 只改交换链与 aspect；人工清单⑦ / ⑧ 专测「Resize 不跳帧」「高刷恒定 30」；`determinism.rs` / `interaction.rs` 的 60 Hz / 144 Hz 重放作 CPU 基线（D2 / D4 / D5）。
- [example 反向依赖上层或形成第二套呈现代码 / 第二份 `RtsMatch`] → example 只在 `w3-client` 包内向下依赖，禁止自建 `EventLoop` / `GpuContext::new` / 直接 `&mut RtsMatch`；`critic` 复审「单一装配、单一权威状态、`w3-core` 零 diff」；`w3-core` 不反向依赖上层硬红线（D1 / D2 / D7）。
- [example 装配参数（`seed` / `content_hash` / `max_frames`）被误当成权威状态或外部可变输入] → 全部用源码内整数常量，`MatchConfig` 冻结 `logic_fps = 30`；不读命令行参数 / 环境变量 / `config.toml` / 标准输入，`max_frames` 仅为表现层呈现帧上限（D3 / D4）。
- [人工冒烟产物（截图 / 日志）被 `git add -A` 误入库] → 产物落已 gitignored 的 `target/` 等本机路径；`python tools/verify_all.py` 的资产扫描 + `git status` 复核；代码改动仅落在 `crates/w3-client/` 与 `crates/w3-render/`、`w3-core` 零 diff（D3 / D7）。
- [example 与 `ClientApp::new` 签名演进不同步（`resumed` 内 GPU 初始化流程变化）] → example 只依赖公开签名 `new(driver, terrain, library, max_frames)` 与 `run()`；`GpuContext::new` 保持在 `ClientApp::resumed` 单一位置；若签名变化，由 `client-owner` 在同一 change 内迁移并复跑门禁（D2）。
- [归档顺序与其它 active change 争用主 specs] → 只新增 `playable-window-entry`，零 MODIFIED delta；与 `gameplay-command-wire-format` 无依赖；归档前 `openspec validate playable-window-smoke --strict` 复跑，归档后确认主 `openspec/specs/playable-window-entry/spec.md` 已建立（D8）。
- [T 生产「看不出效果」被误判为功能失效] → 第五轮真窗口用户确认其他交互均正常、仅 T 生产无法从画面确认。经核对为**已知呈现限制，非内核未生效**：`enqueue_production`（sim_match.rs:1073）写入 30 帧队列，30 逻辑帧后 `complete_production`（sim_match.rs:1311-1345）确实 `entities.spawn()` 生成新单位、`produced += 1` 且 `supply_used` / `supply_cap += 1`；但新单位坐标直接取生产者 `(x, y)`（sim_match.rs:1321-1325）与生产者完全重叠，HUD 进度条 `ProgressBar::new(0.0)` 恒 0%、队列仅显示选中单位（hud.rs:157 / 257）、30 帧（1 秒）一闪而过，肉眼无从分辨。用户拍板**本 change 只保证内核生效、呈现可见性留后续独立 change**：不改 `w3-core`（`complete_production` 属既有内核行为，改动会触碰状态哈希 / 快照 / golden hash，超出「补入口」范围与 ADR 0002 硬门禁）、不改 HUD / 合成 `train_frames`。内核生效由 `frozen_definitions_make_train_and_cast_effective` / `client_app_train_command_reaches_the_kernel` 两个 CPU 回归守住（D13）；tasks 4.4 登记为已解释呈现限制。

## Migration Plan

1. **规范侧（本轮，`spec-steward` 可写范围）**：写 `openspec/changes/playable-window-smoke/` 下 proposal / specs / design / tasks 四件套；`openspec validate playable-window-smoke` 与 `--strict` 通过；`python tools/check_encoding.py` 与 `git diff --check` 通过。不碰主 specs / 其它 change 目录 / `crates/` / `docs/` / 根配置。
2. **用户拍板（交付回报项；用户已逐轮拍板确认）**：(a) example 命名与落点（`examples/playable.rs` vs `[[bin]]`）；(b) 确认立即 `/opsx:apply`——前置归档集合已满足（P6、`linux-winit-platform-backend`、`cross-platform-golden-hash`、`gameplay-command-wire-format` 均已归档；当前仅 1 个 active change：`playable-window-smoke`）；(c) 人工 GPU 冒烟由用户本机执行（清单标注「待人工在有 GPU / 显示设备桌面执行」）；(d) 初始裁定「中文上屏 / 启用 `default_fonts`」留后续独立项，第三轮用户改判为启用 `default_fonts` + ASCII 文案（D6 / 第 10 组）。四项改判需回到对应 artifact 修订。
3. **`/opsx:apply`（由 `client-owner` 执行，已实施）**：新增 `crates/w3-client/examples/playable.rs`；合成地形 / 合成单位脚本 → `FixedStepClient` → `ClientApp::new(...).run()`；`w3-core` 始终零改动，但为修拾取 / 高 DPI 一致性扩展了 `w3-render` / `w3-client` 既有实现并启用 `default_fonts`（第 9–11 组，用户逐轮拍板）。写完 `cargo fmt --all`，提交前 `cargo fmt --all -- --check`。
4. **本机自检（`client-owner`，有显示设备机器）**：`cargo build -p w3-client --example playable`、`cargo run -p w3-client --example playable`；确认窗口打开、`GpuContext::new` 成功、英文结构化日志可观测、HUD 文字正常显示（第 10 组）。
5. **人工 GPU 冒烟（人工项，D5 清单逐项勾选）**：由 (c) 指定的人与设备执行，记录可观察证据；无 GPU 环境明确报告不可执行。
6. **门禁**：`cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`、`python tools/verify_all.py`、重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism`）与 `crates/w3-render/tests/isolation.rs`、`openspec validate playable-window-smoke --strict`。
7. **`critic` 复审 → 本地 commit（中文描述）→ 人工 push → CI 双平台门禁 → `/opsx:archive`**：`openspec archive playable-window-smoke -y`，确认主 `openspec/specs/` 新增 `playable-window-entry` 且既有 11 个 capability 未被改写（P6、winit、golden-hash、gameplay-command-wire-format 均已归档，当前 1 个 active change）。本地 commit 由 AI 执行，`push` / CI / 归档动作由人工 / 编排者执行（CODEBUDDY.md §4 / §9）。
8. **回滚**：只需删除 `crates/w3-client/examples/playable.rs`；`w3-core` 零改动、依赖零改动、快照格式不变（不 bump `SNAPSHOT_VERSION`），回滚不影响权威状态或其它 change。

## Open Questions

> 以下为**需要用户拍板 / 确认**的事项，现已全部拍板并登记结论。四件套按用户逐轮拍板写入（`examples/playable.rs`、立即 `/opsx:apply`、人工冒烟由用户本机执行、启用 `default_fonts` 而中文上屏留后续独立项），并在交付回报中明确列出；若用户改判，须回到对应 artifact 与 tasks 修订，不擅自猜测。这些是「流程 / 归属 / 后续范围」决策，不改变 `playable-window-entry` 的行为契约，故不阻塞 `--strict` 校验。

- **（a）example 命名与落点**：用户已确认 `crates/w3-client/examples/playable.rs`（`cargo run -p w3-client --example playable`），否决 `[[bin]]` / 扩展 `render_frame.rs` / 新 crate（D1，第 1 组 1.1）。
- **（b）实施时机**：用户已拍板立即 `/opsx:apply`。`gameplay-command-wire-format` 已于 2026-11-11 归档，前置归档集合已满足（`linux-winit-platform-backend` / `cross-platform-golden-hash` 亦已归档，当前仅 1 个 active change：`playable-window-smoke`）；与已归档的 `gameplay-command-wire-format` 无依赖（D8 结论不变）。
- **（c）人工 GPU 冒烟的执行者与设备**：清单为人工项，只能人工在有 GPU / 显示设备的桌面执行；已登记由用户在本机执行（第 1 组 1.3、tasks 第 4 组导语），Linux 侧 `x11` 后端机器本期不纳入（D5）。
- **（d）中文上屏 / 启用 `default_fonts`**：用户第三轮改判为启用 `default_fonts` 内置开源字体 + ASCII 文案（D6 / 第 10 组），字体许可证已在 `NOTICE.md` 登记；中文上屏（需内嵌 CJK 字体、涉及资产与许可证，CODEBUDDY.md §10 人工独占）仍留后续独立 change。已不属待确认项。
- **（e）`max_frames` 取值**：`ClientApp::new` 的第 4 参用于「无人值守验收限帧」；已定 `MAX_FRAMES = u32::MAX`（不限帧、由人工关闭窗口退出，见 example），不改变 spec / 任务分解（D2）。
