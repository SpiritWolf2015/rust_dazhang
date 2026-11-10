# Spec Delta

## Purpose

本 capability 定义「可玩窗口冒烟入口」的行为契约：以零资产、零新依赖的最小 cargo example 复用既有 `ClientApp` 装配，把 P6 已交付的真实 winit 窗口 / wgpu 呈现 / 输入 / HUD / 30 Hz 累加器路径真正启动起来，供人工在有 GPU 与显示设备的桌面执行窗口冒烟验收；入口只调公开 API，`w3-core` 零改动，状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照不变。相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期 / 否决通用引擎）、ADR 0004（冻结坐标）、ADR 0005（线格式与分层边界）。本 capability 为新增，全部为 ADDED Requirements；刻意不产生 `playable-client` 等既有 capability 的 MODIFIED delta，理由见 `proposal.md` Capabilities 与 `design.md` D8。

## ADDED Requirements

### Requirement: 最小可编译可运行的窗口入口

系统 SHALL 提供 `crates/w3-client/examples/playable.rs` 作为 cargo example，且 MUST 能以 `cargo run -p w3-client --example playable` 编译并启动。入口 MUST NOT 以 `[[bin]]`、新 crate 或修改 `w3-client/Cargo.toml` 依赖段的形式引入，MUST NOT 改动任何既有公开 API（BREAKING 无）。example 自身 SHALL 是 `ClientApp::run()` 在全仓库的唯一真实调用者。

#### Scenario: 编译与启动

- **WHEN** 在有显示设备的本机执行 `cargo build -p w3-client --example playable` 与 `cargo run -p w3-client --example playable`
- **THEN** 编译成功且打开一个真实 winit 窗口进入事件循环，无 `w3-client` 依赖段或既有 API 的 diff

#### Scenario: 不引入新入口形态

- **WHEN** 校验本 change 的文件清单
- **THEN** 只新增 `examples/playable.rs` 与其规范文件，不含 `main.rs`、`[[bin]]`、新 crate 或 `Cargo.toml` 依赖变更

### Requirement: 复用既有 ClientApp 装配，不出现第二套呈现管线

example SHALL 复用 `ClientApp::new(...).run()` 与本地 `FixedStepClient`，MUST NOT 新写宿主事件循环、第二套呈现代码、第二份 `RtsMatch` 或复制 `EventLoop`。`GpuContext` / `Window` 的构造 SHALL 沿用 `ClientApp` 内部流程（参照 `render_frame.rs` 的 `GpuContext::new`），example 只负责合成数据与装配参数。

#### Scenario: 单一装配与单一权威状态

- **WHEN** 阅读 example 的装配路径
- **THEN** 只有一个 `ClientApp::new(driver, terrain, library, max_frames).run()` 调用点、一个 `FixedStepClient`、一个 `RtsMatch`，且无自建 `EventLoop::builder()` / `run_app()`

#### Scenario: GPU 上下文由既有流程创建

- **WHEN** 运行 example 并抵达 `resumed`
- **THEN** `GpuContext::new(&window, width, height)` 由 `ClientApp` 内部调用并成功，example 未另写一套 GPU 初始化

### Requirement: 合成数据零资产与冻结边界

example SHALL 只用合成地形与合成单位脚本装配对局（参照 `render_frame.rs::fallback_terrain` 与 `determinism.rs` 的 `open_world` / `script()`）；MUST NOT 读 `config.toml`、导出 JSON、`assets-export/`、`slk-exported/` 或任何暴雪资产 / MPQ 派生数据（ADR 0001）。任何 `f64 → SimFixed` 转换 MUST 经 `w3-app::freeze_f64`（ADR 0004），固定 `MatchConfig` 用整数常量。

#### Scenario: 无本机导出数据仍可启动

- **WHEN** 在不含任何导出资产与 `config.toml` 的干净环境运行 example
- **THEN** 用合成高度场、合成 `Spawn` 命令与固定 `MatchConfig` 正常启动，不读取任何资产文件

#### Scenario: 冻结边界不被绕过

- **WHEN** 检查 example 的 `f64 → SimFixed` 转换
- **THEN** 全部经 `w3-app::freeze_f64`，不存在 `v * 65536.0` 之类的浮点乘法直转路径

### Requirement: 只调公开 API 且不改权威状态

example SHALL 只调用 `w3-core` / `w3-client` / `w3-render` / `w3-app` 既有公开 API；`crates/w3-core/` MUST 零改动。本 change MUST NOT 新增任何权威状态字段，MUST NOT bump `SNAPSHOT_VERSION`（当前 **3**，头部含 `defs_hash` / `content_hash`）；FNV-1a 状态哈希字段集合、帧序与快照头部布局 MUST 保持不变。

#### Scenario: 内核零改动与哈希快照不变

- **WHEN** 运行 example 若干整数帧并与 CPU 基线比对
- **THEN** `w3-core` 无 diff，`SNAPSHOT_VERSION` 仍为 3，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致

#### Scenario: 权威状态遗漏被门禁拦截

- **WHEN** 假设将来把某字段转为权威状态却未加入哈希或由 `SNAPSHOT_VERSION` 定义的快照
- **THEN** `cargo test -p w3-core --test determinism` 失败；本 change 不新增权威字段，故无需扩展哈希或快照字段集合

### Requirement: 30 Hz 整数帧与表现层状态隔离

逻辑帧 SHALL 由冻结 `logic_fps = 30` 的整数帧累加器驱动无参 `step()`（ADR 0002 §2）；`step()` MUST NOT 接受 delta / 墙钟 / 刷新率。累加器余数、插值 `alpha`、相机位姿、选中集合、HUD 布局与窗口尺寸 MUST NOT 进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3）定义的快照；`Resized` MUST NOT 触发 `step()`。

#### Scenario: 高刷下逻辑帧恒定

- **WHEN** 在 144 Hz 显示器上连续呈现
- **THEN** 每秒仍恰好推进 30 个整数逻辑帧，帧号与逐帧状态哈希与 30 Hz 基准一致

#### Scenario: 表现层状态变化不改哈希与快照

- **WHEN** 在真窗口中改变选中、相机、窗口尺寸并推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节与无窗口实例完全一致

### Requirement: 人工 GPU 冒烟清单与人工边界

系统 SHALL 提供可逐项勾选的人工 GPU 冒烟清单，MUST 至少覆盖：窗口能开、`GpuContext::new` 成功、egui HUD overlay pass 正常提交、中文标签按已裁定不显示且有英文降级日志、左键框选 / 右键移动 / 施法 / 生产键、WASD / 滚轮相机、`Resized` 后画面正常且逻辑帧不跳、高刷下逻辑帧恒定 30。清单项 MUST 标注为**只能人工在有 GPU 与显示设备的桌面执行**。

#### Scenario: 窗口与 HUD 呈现

- **WHEN** 人工运行 example 并观察首帧
- **THEN** 窗口正常打开、`GpuContext::new` 成功、egui HUD overlay pass 正常提交，几何元素（框选 / 血条）可见

#### Scenario: 键鼠与相机交互

- **WHEN** 人工依次执行左键框选、右键移动、Q 施法 / T 生产 / C 取消、WASD 平移与滚轮缩放
- **THEN** 各项输入经既有 `ClientApp` 路径生效，命令只经公开提交入口，失败以英文结构化日志明确报告

#### Scenario: Resize 与刷新率

- **WHEN** 人工拖拽改变窗口尺寸并在高刷显示器持续呈现
- **THEN** 画面正常、逻辑帧不跳且恒为 30，累加器与呈现刷新率解耦

#### Scenario: 人工边界与无 GPU 环境

- **WHEN** 在无 GPU / 无显示设备的 headless 或 CI runner 上尝试 GPU 冒烟
- **THEN** 明确报告无法执行、不静默跳过、不改判为通过；清单未逐项勾选前不得归档

#### Scenario: CI 边界

- **WHEN** CI 在 `ubuntu-latest` / `windows-latest` 跑 `python tools/verify_all.py`
- **THEN** 只覆盖编译与 CPU 测试（含 `w3-core/tests/determinism.rs`、`w3-render/tests/isolation.rs`），且 MUST NOT 声称 CI 已验证 GPU 窗口链路

### Requirement: 字体降级与许可证边界

系统 SHALL 保持 egui 0.37 不启用 `default_fonts`；`FontDefinitions::default()` 返回空 `font_data` 时，中文标签 MUST NOT 上屏，且 MUST 走 `HudAssets::degraded()` + 英文结构化 `key=value` 降级日志（tasks 6.6）。启用 `default_fonts` 会引入 OFL-1.1 / Ubuntu-font-1.0 字体许可证，属 CODEBUDDY.md §10 人工独占且仍无 CJK 字形，故 MUST NOT 在本 change 启用；中文上屏 SHALL 留作后续独立 change。

#### Scenario: 中文字体降级

- **WHEN** 运行 example 且 egui 内置字体数据为空
- **THEN** `HudAssets::prepare` 显式返回 `EmptyFontData`，中文文字元素不绘制，英文日志 `hud font unavailable` 明确报告，模拟推进不变

#### Scenario: 许可证边界不被突破

- **WHEN** 校验 `w3-client/Cargo.toml` 与依赖图
- **THEN** `default_fonts` 未启用、`Cargo.lock` 无变化、`NOTICE.md` 无需登记，OFL-1.1 / Ubuntu-font-1.0 许可证未进入依赖树

### Requirement: 不新增依赖且产物不入库

本 change MUST NOT 新增任何 Cargo 依赖或修改 `Cargo.lock`；人工冒烟产物（截图 / 日志）MUST NOT 提交进仓库，SHALL 落在已 gitignored 的本机路径（如 `target/`）。清单项 MUST NOT 把 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效或任何游戏资产写进仓库（ADR 0001）。

#### Scenario: 依赖与资产扫描

- **WHEN** 跑 `python tools/verify_all.py` 的资产扫描与 `git status`
- **THEN** 无新增依赖、无 `Cargo.lock` diff、无任何 MPQ 派生数据 / 导出资产 / 截图 / 日志被跟踪

#### Scenario: 产物留本机

- **WHEN** 人工冒烟产生截图或日志
- **THEN** 产物落在 gitignored 路径且不入库，`.gitignore` 与 CI 资产扫描继续把关
