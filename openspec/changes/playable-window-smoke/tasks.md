# Tasks

> 规范四件套已立，初始全部不预勾。第 1 组是 `design.md` Open Questions 的拍板项：在拍板确认前不得把它们当作既定假设直接实施，避免把未定决策硬编码进任务清单。用户已拍板确认 (a)(c)(d) 推荐默认值、(b) 前置归档集合已满足并立即 `/opsx:apply`；首轮人工 GPU 冒烟暴露 P6 拾取 / 呈现投影不一致后拍板扩展范围直接修复（第 9 组）；第三轮真窗口运行再暴露拾取半径 / 窗口尺寸 / HUD 无文字，拍板包围盒拾取、1920×1080 窗口、启用内置开源字体（第 10 组）；第四轮真窗口运行（物理 2880×1620 / 逻辑 1920×1080 / `scale_factor=1.5`）再暴露 HUD 物理 / 逻辑坐标错位与空冻结玩法定义导致 Q / T / C NoEffect，用户要求先核对参考开源项目、写足 CPU 自测再人工验收，据此新增第 11 组。中文上屏留后续独立 change。第 4 组人工 GPU 冒烟标注「待人工在有 GPU / 显示设备桌面执行」。工具 / 命令名保持原样，任务描述一律中文。**第五轮（修复后）用户确认「基本都正确了……其他都没有问题了，可以先归档了」：本次启动日志无任何 `input action rejected`，框选 / 右键移动 / Q 施法 / WASD / Z-X 相机均正常；T 生产「看不出效果」经核对为已知呈现限制（非功能失效，内核确实 `entities.spawn()` 生成新单位），用户拍板记为已知限制、不改代码（详见 `design.md` Risks），4.4 登记为已解释限制。按用户拍板补勾第 4 组有证据项、走 `critic` 复审与本地中文 commit（第 6 / 7 组），归档动作（第 8 组）与 `push` / CI 留人工。**

## 1. 前置拍板与规范基线（用户拍板项，apply 前确认）

- [x] 1.1 用户拍板 (a)：确认 example 命名与落点为 `crates/w3-client/examples/playable.rs`（`cargo run -p w3-client --example playable`），否决 `[[bin]]` / 扩展 `render_frame.rs` / 新 crate；验证：回报记录明确命名与形态，且 `design.md` D1 与 `specs/playable-window-entry/spec.md` 无需改判
- [x] 1.2 用户拍板 (b)：确认实施时机——前置归档集合已满足（`gameplay-command-wire-format` 已于 2026-11-11 归档，`linux-winit-platform-backend` / `cross-platform-golden-hash` 亦已归档，当前仅 1 个 active change：`playable-window-smoke`），用户已拍板立即 `/opsx:apply`；验证：回报记录明确 apply 顺序与前置归档集合
- [x] 1.3 用户拍板 (c)：确认人工 GPU 冒烟由谁、在何种 GPU 与显示设备执行（含 Linux 侧 `x11` 后端机器是否纳入）；验证：冒烟清单登记执行者与设备，无 GPU 环境明确标注不可执行
- [x] 1.4 用户拍板 (d)：初始裁定「中文上屏 / 启用 `default_fonts`」留后续独立 change；第三轮用户改判为启用 egui `default_fonts` 内置开源字体 + ASCII 文案（第 10 组 / D6），中文上屏仍留后续独立 change；验证：回报记录确认改判，`w3-client/Cargo.toml` 已启用 `default_fonts`、`NOTICE.md` 登记字体许可证
- [x] 1.5 复核本轮规范四件套 `proposal.md` / `specs/playable-window-entry/spec.md` / `design.md` / `tasks.md` 与 `openspec validate playable-window-smoke --strict` 通过；验证：命令输出 `is valid` 且无 WARNING

## 2. 实现 example（由 `client-owner` 执行，单一 owner）

- [x] 2.1 新增 `crates/w3-client/examples/playable.rs`：合成高度场（蓝本 `render_frame.rs::fallback_terrain`）与合成 `Spawn` 命令脚本（蓝本 `determinism.rs` 的 `open_world` / `script()`），固定整数常量 `MatchConfig { seed, logic_fps: 30, content_hash }`；验证：`cargo build -p w3-client --example playable` 可编译，源码无 `config.toml` / 导出 JSON / `assets-export/` / `slk-exported/` / 暴雪资产读取
- [x] 2.2 装配路径只复用 `FixedStepClient::from_match(sim)` + `ClientApp::new(driver, terrain, library, max_frames).run()`，不自建 `EventLoop` / `ApplicationHandler` / `GpuContext::new` / 第二份 `RtsMatch`；验证：源码检索唯有 1 处 `ClientApp::new` 与 1 处 `.run()`，无 `EventLoop::builder()` / `run_app()` 复制
- [x] 2.3 任何 `f64 → SimFixed` 经 `w3-app::freeze_f64`（ADR 0004），不新增浮点乘法直转；验证：源码检索无 `v * 65536.0` 之类路径，`w3-core` 的 `from_int` / 冻结边界保持唯一
- [x] 2.4 校验 `crates/w3-core/` 零 diff、不新增权威状态字段、不 bump `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合与快照头部布局不变；验证：`git diff -- crates/w3-core/` 为空，`grep SNAPSHOT_VERSION crates/w3-core/src/snapshot.rs` 仍为 3
- [x] 2.5 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism`）确认内核零改动且 26 项全绿；验证：测试输出全绿，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致
- [x] 2.6 确认 example 注释（模块级 `//!`、行内 `//`）一律中文、日志 / 错误文本一律英文结构化 `key=value`、无散落 `println!`、无新增 `.ps1` / `.sh` / `.bat` / `.cmd` 脚本；验证：`cargo fmt --all` 后 `git diff --check` 无行尾空格，源码复核无英文注释

## 3. 本机 cargo build/run（有显示设备机器，CPU 自检）

- [x] 3.1 执行 `cargo build -p w3-client --example playable` 与 `cargo run -p w3-client --example playable`；验证：编译成功且打开一个真实 winit 窗口进入事件循环，依赖段仅按第 10 组启用 egui `default_fonts`（无新增直接依赖）、既有公开签名不变（仅新增）
- [x] 3.2 核对启动日志：`GpuContext::new` 成功、英文结构化 `key=value` 日志可观测；验证：窗口诊断日志与 HUD 文字按第 10 组裁定正常显示
- [x] 3.3 核对不新增直接依赖与产物不入库；验证：`Cargo.lock` 仅新增 `epaint_default_fonts` 传递包、`NOTICE.md` 仅登记字体许可证，`git status` 无 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效 / 截图 / 日志被跟踪

## 4. 人工 GPU 冒烟（人工项，仅能在有 GPU 与显示设备的桌面执行）

> 本组全部为**人工项**：只能由第 1.3 项拍板指定的人与设备逐项执行。**执行者与设备登记：由用户在本机执行（具体 GPU 型号 / 显示设备由用户在执行时补记；Linux 侧 `x11` 后端机器本期不纳入）。** **运行记录：第一次（修复前）窗口可开、英文 `hud font unavailable` 降级日志出现，但单位占位标记未生成（`ModelLibrary` 为空键）；第二次单位可见但框选 / 施法报 `no unit selected` / `no target entity picked`（拾取 / 呈现投影不一致，差 100–250px）；第三次投影同源后 `Cast` 仍报 `no target entity picked`（24px 固定拾取半径相对 200–400px 四面体过小），且窗口过小、HUD 无文字；第四次（物理 2880×1620 / 逻辑 1920×1080 / `scale_factor=1.5`）选中框仍不贴合、`Cast` 报 `NoTarget`、Q / T / C 无效果——根因为 HUD 物理坐标直接喂给 egui 逻辑空间偏移 1.5 倍、且 example 用空冻结玩法定义；第五轮（修复后）用户确认「基本都正确了……其他都没有问题了，可以先归档了」，启动日志 `window created: width=2880 height=1620 scale_factor=1.5`，**全程无任何 `input action rejected`**，框选 / 右键移动 / Q 施法 / WASD / Z-X 相机均正常。四处缺陷均已在本 change 修复（第 9 / 10 / 11 组），CPU 自测已覆盖。** **第五轮遗留：T 生产「看不出效果」经核对为已知呈现限制、非功能失效（内核确实 `entities.spawn()` 生成新单位，但与生产者坐标完全重叠、HUD 进度条恒 0%、队列仅 1 秒），用户拍板记为已知限制、不改代码，详见 `design.md` Risks 与 4.4。** 当前 CLI 会话标注「CI / headless / 无显示设备环境 MUST 明确报告无法执行、不静默跳过、不改判为通过」；**CI 只覆盖编译 + CPU 测试，未验证 GPU 链路**；**清单未逐项勾选前不得归档**。**4.6（`Resized` 拖拽改窗口尺寸）与 4.7（高刷 144 Hz 逐帧恒定 30）本轮人工会话未执行、保持未勾，CPU 侧 `determinism.rs` / `interaction.rs` 重放有基线覆盖但不冒充人工通过；归档前由编排者按用户拍板确认或补验。**

- [x] 4.1 人工项：确认窗口能正常打开且可关闭；验证：第五轮日志 `window created: width=2880 height=1620 scale_factor=1.5`（物理 2880×1620 = 逻辑 1920×1080 × 1.5），多轮 `cargo run` 均可重开窗口，无 `window creation failed`
- [x] 4.2 人工项：确认 `GpuContext::new` 成功、egui HUD overlay pass 正常提交；验证：第五轮启动日志出现 `wgpu_hal::vulkan::adapter Found 6 cooperative matrix configurations supported by wgpu`、无 `render failed` / `gpu init failed`，用户总体确认「基本都正确了」
- [x] 4.3 人工项：确认 HUD 文字（资源 / 人口 / 生产队列 / 按键提示）以 ASCII 正常显示，且所有存活单位都有队伍色标记与血条（蓝色己方 / 红色敌方）；验证：字体启用后无 `hud font unavailable` 降级日志，第五轮用户总体确认无其它问题；未逐项截图存档
- [x] 4.4 人工项：依次执行左键框选、右键移动、Q 施法 / T 生产 / C 取消生产；验证：第五轮全程无 `input action rejected`，框选 / 右键 / Q / C 经既有 `ClientApp` 路径生效；**T 生产内核生效但呈现不可见（已知限制，见 `design.md` Risks 与本组导语），按用户拍板记为已解释限制、不改判功能失败**
- [x] 4.5 人工项：执行 WASD 平移、滚轮缩放、Z-X 旋转相机；验证：第四轮用户明确「右键移动、WSAD 移动相机、XZ 旋转相机成功」，第五轮无回归
- [ ] 4.6 人工项：拖拽改变窗口尺寸（`Resized`）；验证：画面正常、逻辑帧不跳、累加器与呈现刷新率解耦 —— **本轮人工会话未执行，CPU 侧 `interaction.rs` Resize 基线覆盖，待编排者补验 / 拍板**
- [ ] 4.7 人工项：在高刷（如 144 Hz）显示器持续呈现；验证：逻辑帧恒定 30，逐帧帧号与 30 Hz 基准一致 —— **本轮人工会话未执行，CPU 侧 `determinism.rs` / `interaction.rs` 60 Hz / 144 Hz 重放基线覆盖，待编排者补验 / 拍板**
- [x] 4.8 人工项：汇总清单勾选状态并明确记录「CI 只覆盖编译 + CPU 测试，未验证 GPU 链路」；验证：4.1–4.5 / 4.8 按证据勾选，4.6 / 4.7 如实标注未执行，T 生产记为已知限制；CI 边界已在导语与本项记录

## 5. 门禁（编译 + CPU 测试，含确定性重跑）

- [x] 5.1 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（官方 `rustfmt` 默认风格）；验证：命令退出码 0
- [x] 5.2 `python tools/check_encoding.py` 通过：UTF-8 无 BOM、LF、文件末尾换行、无行尾空格；验证：输出无 `BOM detected` / `CRLF line ending detected` / `invalid UTF-8`
- [x] 5.3 `git diff --check` 通过；验证：输出无 whitespace error
- [x] 5.4 重跑 `crates/w3-core/tests/determinism.rs`（26 项）、`crates/w3-render/tests/isolation.rs`（3 项）、`cargo test -p w3-render --lib`（含投影回归）与 `cargo test -p w3-client --test interaction`（含框选 / DPI / 玩法定义回归）全绿；验证：全部通过，`w3-core` 零 diff
- [x] 5.5 `python tools/verify_all.py` 通过（含 `cargo test --workspace`、`cargo build --examples` 与 CI 资产扫描）；验证：输出 `Verification OK`，无 MPQ 派生数据 / 导出资产入库
- [x] 5.6 复核不新增权威状态、`SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合与快照头部布局不变、表现层状态（累加器余数 / `alpha` / 相机 / 选中 / HUD / 窗口尺寸 / 投影矩阵 / 包围盒 / 缩放比）不进哈希或快照；验证：`w3-core` 无 diff，`interaction.rs` 的哈希 / 快照字节比对仍全绿
- [x] 5.7 复核 `openspec validate playable-window-smoke` 与 `openspec validate playable-window-smoke --strict` 仍通过；验证：两条命令均输出 `is valid` 且 `--strict` 无 WARNING

## 6. `critic` 只读复审

- [x] 6.1 由只读 `critic` 复审确定性、所有权、许可证、编码、资产入库、「单一装配 / 单一权威状态 / `w3-core` 零 diff」与「拾取 / 呈现投影同源、高 DPI 坐标一致、既有公开签名不变、字体许可证登记」；验证：复审结论记录无阻塞项，且明确「CI 未验证 GPU 链路、人工清单已通过或未执行」—— 复审结论：第 1–6 / 8 项无阻塞，B1 / B2 文档矛盾已修，B3（4.6 / 4.7 未执行）为归档前置而非 AI 侧阻塞；明确确认「CI 只覆盖编译 + CPU 测试、未验证 GPU 链路」「人工清单 4.6 / 4.7 未执行」「4.4 的 T 生产呈现限制已按用户拍板登记为已知限制」
- [x] 6.2 复核无新增直接依赖、`Cargo.lock` 仅新增 `epaint_default_fonts`、`NOTICE.md` 登记字体许可证、工具脚本跨平台；验证：复审清单逐项勾对 `design.md` D6 / D7 / D9 / D10 / D11 / D12 / D13，`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3

## 7. 中文 commit（本地写入，人工独占 push）

- [x] 7.1 以中文描述 + Conventional Commits 英文 type 前缀提交，一个改动一个 commit，首行 ≤ 72 字符；验证：`git log -1` 显示中文描述，且包含 example 与拾取 / 字体 / 窗口 / DPI / 玩法定义修复及四件套的全部改动
- [x] 7.2 确认 `git push` 等远端同步命令一律由人工执行；验证：AI 未执行任何 `push` / `fetch` / `pull` / `clone` 命令（此为人工独占，非 AI 任务）

## 8. 归档

- [ ] 8.1 人工 GPU 冒烟清单（第 4 组）逐项勾选、`critic` 复审无阻塞、CI 双平台门禁通过后，由编排者执行 `openspec archive playable-window-smoke -y`；验证：命令成功且归档目录带日期前缀
- [ ] 8.2 确认主 `openspec/specs/` 新增 `playable-window-entry/spec.md`，既有 11 个 capability 未被改写、无其它 change 争用（当前仅 1 个 active change：`playable-window-smoke`；`gameplay-command-wire-format` 已归档）；验证：`openspec list --specs` 列出新 capability，`git diff -- openspec/specs/` 只新增该目录

## 9. 修复拾取 / 呈现投影一致性（范围扩展后新增，已实施）

- [x] 9.1 `w3-render` 新增 `ScreenProjection`（与 `Camera::matrix` 同源的 `view_proj` 及逆矩阵），`world_to_screen` 以 `UNIT_MARKER_SIZE * 0.8` 单位中心平面为基准、`screen_to_world` 以 y=0 地面为基准；验证：新增投影同源 / 地面反投影回归测试全绿
- [x] 9.2 `w3-client::ScreenMapper` 增加可选 `projection` 并委托；`ScreenMapper::new` 签名不变（默认旧线性映射）；`ClientApp::mapper()` 注入精确投影，新增 `ClientApp::screen_mapper()`；验证：`interaction.rs` 改用 `app.screen_mapper()` 统一口径，既有断言全绿
- [x] 9.3 新增 `selection_uses_the_render_projection` CPU 回归：证明注入的精确投影与旧线性映射不同，且按呈现投影拖框能命中单位；验证：测试全绿
- [x] 9.4 `scene::UNIT_MARKER_SIZE` 提为常量供拾取平面推导；合成地形起伏压到很浅（系数 0.15），消除贴地高度与拾取平面的二次偏差；验证：`cargo build -p w3-client --example playable` 可编译
- [x] 9.5 复核 `w3-core` 零 diff、`SNAPSHOT_VERSION` 仍为 3、不新增直接依赖（`Cargo.lock` 仅在第 10 组新增 `epaint_default_fonts` 传递包）、既有公开签名不变；验证：`git diff -- crates/w3-core/` 为空，源码复核无签名删除
- [x] 9.6 四件套（proposal / spec / design / tasks）同步范围扩展，`openspec validate playable-window-smoke --strict` 通过；验证：命令输出 `is valid` 且无 WARNING

## 10. 拾取包围盒 / 窗口尺寸 / HUD 字体（第三轮反馈，已实施）

- [x] 10.1 `ScreenProjection::unit_screen_bounds` 投影单位占位标记的保守屏幕包围盒，`ScreenMapper::unit_screen_bounds` 委托；验证：`cargo test -p w3-render --lib` 全绿
- [x] 10.2 `commit_selection` / `pick_enemy` 改用包围盒判定，单击阈值与拾取半径解耦；`interaction.rs` 空地点用例同步；验证：`cargo test -p w3-client --test interaction` 全绿
- [x] 10.3 `HudUnit` 增加 `bounds_min` / `bounds_max`，HUD 选中框按包围盒绘制（替换硬编码 40×40）；验证：CPU 测试全绿
- [x] 10.4 `ClientApp::resumed` 默认 1920×1080 逻辑尺寸，输出英文 `window created: width=... height=... scale_factor=...` 诊断日志；验证：`cargo build -p w3-client --example playable` 可编译
- [x] 10.5 启用 `egui/default_fonts`，`i18n` 文案改 ASCII（含完整按键提示），`HudAssets::prepare` 断言更新、`NOTICE.md` 登记字体许可证；验证：`cargo test -p w3-client --test interaction` 全绿，`Cargo.lock` 仅新增 `epaint_default_fonts`
- [x] 10.6 四件套同步范围扩展与前几组判据（3.3 / 9.5 的 `Cargo.lock` / `NOTICE.md` 口径），`openspec validate playable-window-smoke --strict` 通过；验证：命令输出 `is valid` 且无 WARNING

## 11. 高 DPI 坐标一致性与玩法定义（第四轮反馈，已实施并 CPU 自测）

- [x] 11.1 核对 winit 0.30.13 `CursorMoved.position` 为 `PhysicalPosition`、egui 为逻辑像素；`ScreenMapper` 增加 `pixels_per_point` / `to_logical` / `to_logical_bounds` / `logical_size`，`ClientApp::mapper()` 从 `window.scale_factor()` 注入；验证：`cargo test -p w3-client --test interaction` 全绿，CPU 测试（无窗口）回落 1.0
- [x] 11.2 `HudModel::collect` 以逻辑坐标输出 `screen` / `bounds_*`，`hud::draw` 为所有存活单位绘制队伍色标记与血条（含未选中敌方）、选中单位加绿框；验证：`hud_marks_all_alive_units_in_logical_coordinates` 全绿
- [x] 11.3 example 用 `RtsMatch::create_with_definitions` + `FixedStepClient::from_match` 注入合成 `unit_def = 1` / `ability = 10`；验证：`frozen_definitions_make_train_and_cast_effective` 与 `client_app_train_command_reaches_the_kernel` 全绿
- [x] 11.4 补齐针对性 CPU 回归（DPI 换算 / HUD 逻辑坐标 / 物理鼠标拾取敌方 / 内核 Train+Cast / ClientApp T 链路 / 既有框选回归），全部通过；验证：`cargo test --workspace` 全绿
- [x] 11.5 四件套同步第四轮范围，`openspec validate playable-window-smoke --strict` 通过；验证：命令输出 `is valid` 且无 WARNING
