# playable-window-entry Specification

## Purpose
本 capability 定义「可玩窗口冒烟入口」的行为契约：以零资产、零新依赖的最小 cargo example 复用既有 `ClientApp` 装配，把 P6 已交付的真实 winit 窗口 / wgpu 呈现 / 输入 / HUD / 30 Hz 累加器路径真正启动起来，供人工在有 GPU 与显示设备的桌面执行窗口冒烟验收；入口只调公开 API，`w3-core` 零改动，状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照不变。相关 ADR：ADR 0001（资产与法律）、ADR 0002（确定性契约）、ADR 0003（范围与分期 / 否决通用引擎）、ADR 0004（冻结坐标）、ADR 0005（线格式与分层边界）。本 capability 为新增，全部为 ADDED Requirements；刻意不产生 `playable-client` 等既有 capability 的 MODIFIED delta，理由见 `proposal.md` Capabilities 与 `design.md` D8。

## Requirements

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

### Requirement: 拾取映射与呈现投影一致

系统 SHALL 让输入拾取（框选 / 命中 / HUD 位置）复用与 `Camera::matrix` 同源的投影，MUST NOT 出现「渲染用透视斜视投影、拾取用轴对齐线性映射」两套口径。`world_to_screen` SHALL 以单位占位标记中心平面为基准，`screen_to_world` SHALL 以 y=0 地面为基准；投影矩阵与逆矩阵 MUST NOT 进入权威状态、状态哈希或由 `SNAPSHOT_VERSION`（当前 3）定义的快照。既有公开 API 签名 MUST NOT 改变，新增项限于 `ScreenProjection` / `ScreenMapper::with_projection` / `ClientApp::screen_mapper` / `UNIT_MARKER_SIZE`。

#### Scenario: 框选命中按呈现投影定位的单位

- **WHEN** 在真窗口按屏幕上看到的 3D 单位拖框或单击
- **THEN** 选中集合包含该单位，`ContextAction` / `Cast` 不再因坐标错位报 `no unit selected` / `no target entity picked`

#### Scenario: 相机变换后仍保持一致

- **WHEN** 平移 / 缩放 / 旋转相机或拖拽改变窗口尺寸后再次框选
- **THEN** 拾取仍与当前呈现投影一致，逻辑帧推进不受影响

#### Scenario: CPU 回归守住一致性

- **WHEN** 运行 `w3-render` 投影同源 / 地面反投影测试与 `w3-client` 的 `selection_uses_the_render_projection`
- **THEN** 全部通过，且证明注入的精确投影与旧线性映射结果不同、按呈现投影拖框能命中单位

### Requirement: 拾取按单位屏幕包围盒判定

系统 SHALL 以单位占位标记在屏幕上的保守包围盒判定框选 / 单击 / 敌方拾取，MUST NOT 用固定小半径或硬编码小矩形判定；HUD 选中框 SHALL 使用同一包围盒绘制，使几何与 3D 实体贴合。包围盒 MUST NOT 进入权威状态、状态哈希或快照。

#### Scenario: 点中四面体任意位置即命中

- **WHEN** 在真窗口单击或框选 3D 单位占位四面体的可见区域（含边缘）
- **THEN** 选中集合包含该单位，敌方拾取不再报 `no target entity picked`

#### Scenario: HUD 选中框贴合实体

- **WHEN** 单位被选中
- **THEN** 选中框与血条按该单位包围盒绘制，尺寸与 3D 四面体一致，不使用固定 40×40 矩形

#### Scenario: 空地右键仍生成移动命令

- **WHEN** 在远离所有单位包围盒的空地右键
- **THEN** 生成 `Move` 命令（而非 `Attack`），坐标经既有冻结边界产出

### Requirement: 高 DPI 坐标空间一致

系统 SHALL 在高 DPI 下让输入拾取（winit 物理像素）与 HUD 绘制（egui 逻辑像素）各用正确的坐标空间：`ScreenMapper` SHALL 携带窗口 `pixels_per_point` 并提供物理→逻辑换算，`HudModel` SHALL 以逻辑坐标输出，MUST NOT 把物理坐标直接交给 egui。系统 SHALL 为所有存活单位绘制 HUD 标记并按队伍区分敌我，使拾取对象可见。

#### Scenario: 1.5 倍缩放下 HUD 贴合实体

- **WHEN** 在 `scale_factor = 1.5` 的窗口运行 example 并选中单位
- **THEN** 选中框与血条绘制在该单位 3D 占位标记的逻辑屏幕包围盒上，无 1.5 倍偏移

#### Scenario: 敌方单位可分辨

- **WHEN** 观察含敌方单位的窗口
- **THEN** 未选中的敌方单位也按队伍色绘制标记与血条，用户可据此定位施法目标

#### Scenario: CPU 回归守住坐标换算

- **WHEN** 运行 `logical_coordinates_are_physical_divided_by_scale_factor` 与 `physical_mouse_picks_enemy_under_high_dpi`
- **THEN** 逻辑坐标与逻辑视口投影逐位一致，物理鼠标点击敌方包围盒能生成施法命令

### Requirement: 合成冻结玩法定义

example SHALL 为对局注入纯内存合成的冻结玩法定义（至少 `unit_def = 1` 与 `ability = 10`），使 Q 施法 / T 生产 / C 取消在内核中产生可观测效果；MUST NOT 读取任何资产文件。

#### Scenario: 生产与施法生效

- **WHEN** 在真窗口按 T 生产或选中单位后对敌方目标按 Q 施法
- **THEN** 内核写入生产队列 / 施法引导运行态，命令经既有公开入口提交，不再 NoEffect

#### Scenario: CPU 回归守住生效链路

- **WHEN** 运行 `frozen_definitions_make_train_and_cast_effective` 与 `client_app_train_command_reaches_the_kernel`
- **THEN** 内核层与 `ClientApp` 输入层均证明生产 / 施法命令生效

### Requirement: 30 Hz 整数帧与表现层状态隔离

逻辑帧 SHALL 由冻结 `logic_fps = 30` 的整数帧累加器驱动无参 `step()`（ADR 0002 §2）；`step()` MUST NOT 接受 delta / 墙钟 / 刷新率。累加器余数、插值 `alpha`、相机位姿、选中集合、HUD 布局与窗口尺寸 MUST NOT 进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 3）定义的快照；`Resized` MUST NOT 触发 `step()`。

#### Scenario: 高刷下逻辑帧恒定

- **WHEN** 在 144 Hz 显示器上连续呈现
- **THEN** 每秒仍恰好推进 30 个整数逻辑帧，帧号与逐帧状态哈希与 30 Hz 基准一致

#### Scenario: 表现层状态变化不改哈希与快照

- **WHEN** 在真窗口中改变选中、相机、窗口尺寸并推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节与无窗口实例完全一致

### Requirement: 人工 GPU 冒烟清单与人工边界

系统 SHALL 提供可逐项勾选的人工 GPU 冒烟清单，MUST 至少覆盖：1920×1080 窗口能开、`GpuContext::new` 成功、egui HUD overlay pass 正常提交、HUD 文字（含按键提示）以 ASCII 正常显示、左键框选 / 右键移动 / 施法 / 生产键、WASD / 滚轮相机、`Resized` 后画面正常且逻辑帧不跳、高刷下逻辑帧恒定 30。清单项 MUST 标注为**只能人工在有 GPU 与显示设备的桌面执行**。

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

### Requirement: 内置开源字体与 HUD 文字可见

系统 SHALL 启用 egui 0.37 的 `default_fonts` feature，使用编译期内嵌的开源字体让 HUD 文字层（含按键提示）正常显示；MUST NOT 复制任何字体文件进仓库、MUST NOT 运行时读取本机字体路径（ADR 0001）。内置字体无 CJK 字形，故 i18n 文案 SHALL 为 ASCII；中文上屏 SHALL 留作后续独立 change。字体许可证 SHALL 在 `NOTICE.md` 登记，变更由程序员本人拍板（CODEBUDDY.md §10 人工独占）。字体数据为空时 MUST 走 `HudAssets::degraded()` + 英文结构化 `key=value` 降级日志。

#### Scenario: HUD 文字与按键提示可见

- **WHEN** 在有显示设备的桌面运行 example
- **THEN** HUD 窗口显示资源 / 人口 / 生产队列与 `LMB drag` / `RMB` / `Q` / `T` / `C` / `WASD` / `Wheel` / `Z/X` 操作提示，无 `hud font unavailable` 降级日志

#### Scenario: 资源缺失仍显式降级

- **WHEN** `FontDefinitions::default()` 的 `font_data` 为空
- **THEN** `HudAssets::prepare` 返回 `EmptyFontData`，文字元素不绘制、英文日志明确报告，模拟推进不变

#### Scenario: 字体许可证边界

- **WHEN** 校验 `w3-client/Cargo.toml`、`Cargo.lock` 与 `NOTICE.md`
- **THEN** `default_fonts` 已启用、`Cargo.lock` 仅新增 `epaint_default_fonts` 传递包、`NOTICE.md` 登记 `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`，仓库无任何字体资产文件

### Requirement: 默认 1920×1080 窗口

系统 SHALL 以 1920×1080 逻辑尺寸创建 winit 窗口，并 SHALL 以英文结构化 `key=value` 日志记录实际宽高与 `scale_factor`。该尺寸纯属表现层，MUST NOT 进入权威状态、状态哈希或快照。

#### Scenario: 窗口尺寸与诊断

- **WHEN** 运行 example 并抵达 `resumed`
- **THEN** 窗口按 1920×1080 逻辑尺寸创建，日志含 `window created: width=... height=... scale_factor=...`

### Requirement: 不新增直接依赖且产物不入库

本 change MUST NOT 新增任何直接 Cargo 依赖；`Cargo.lock` SHALL 仅因 `default_fonts` 新增 `epaint_default_fonts` 传递包。人工冒烟产物（截图 / 日志）MUST NOT 提交进仓库，SHALL 落在已 gitignored 的本机路径（如 `target/`）。清单项 MUST NOT 把 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效或任何游戏资产写进仓库（ADR 0001）。

#### Scenario: 依赖与资产扫描

- **WHEN** 跑 `python tools/verify_all.py` 的资产扫描与 `git status`
- **THEN** 无新增直接依赖、`Cargo.lock` 仅新增 `epaint_default_fonts`、无任何 MPQ 派生数据 / 导出资产 / 截图 / 日志被跟踪

#### Scenario: 产物留本机

- **WHEN** 人工冒烟产生截图或日志
- **THEN** 产物落在 gitignored 路径且不入库，`.gitignore` 与 CI 资产扫描继续把关
