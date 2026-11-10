# Tasks

> 本轮只立规范、不实施代码（`/opsx:apply` 为后续动作），故以下全部不预勾。第 1 组是 `design.md` Open Questions 的拍板项：在拍板确认前不得把它们当作既定假设直接实施，避免把未定决策硬编码进任务清单。工具 / 命令名保持原样，任务描述一律中文。

## 1. 前置拍板与规范基线（用户拍板项，apply 前确认）

- [ ] 1.1 用户拍板 (a)：确认 example 命名与落点为 `crates/w3-client/examples/playable.rs`（`cargo run -p w3-client --example playable`），否决 `[[bin]]` / 扩展 `render_frame.rs` / 新 crate；验证：回报记录明确命名与形态，且 `design.md` D1 与 `specs/playable-window-entry/spec.md` 无需改判
- [ ] 1.2 用户拍板 (b)：确认实施时机——本轮只立规范，是否立即 `/opsx:apply`，还是等仍 active 且未实施的 `gameplay-command-wire-format` 归档后再实施（`linux-winit-platform-backend` / `cross-platform-golden-hash` 已归档）；验证：回报记录明确 apply 顺序与前置归档集合
- [ ] 1.3 用户拍板 (c)：确认人工 GPU 冒烟由谁、在何种 GPU 与显示设备执行（含 Linux 侧 `x11` 后端机器是否纳入）；验证：冒烟清单登记执行者与设备，无 GPU 环境明确标注不可执行
- [ ] 1.4 用户拍板 (d)：确认「中文上屏 / 启用 `default_fonts`」留作后续独立 change，本期维持降级 + 英文结构化日志；验证：回报记录确认，且 `w3-client/Cargo.toml` 的 egui feature 无需改动
- [ ] 1.5 复核本轮规范四件套 `proposal.md` / `specs/playable-window-entry/spec.md` / `design.md` / `tasks.md` 与 `openspec validate playable-window-smoke --strict` 通过；验证：命令输出 `is valid` 且无 WARNING

## 2. 实现 example（由 `client-owner` 执行，单一 owner）

- [ ] 2.1 新增 `crates/w3-client/examples/playable.rs`：合成高度场（蓝本 `render_frame.rs::fallback_terrain`）与合成 `Spawn` 命令脚本（蓝本 `determinism.rs` 的 `open_world` / `script()`），固定整数常量 `MatchConfig { seed, logic_fps: 30, content_hash }`；验证：`cargo build -p w3-client --example playable` 可编译，源码无 `config.toml` / 导出 JSON / `assets-export/` / `slk-exported/` / 暴雪资产读取
- [ ] 2.2 装配路径只复用 `FixedStepClient::new(...)` + `ClientApp::new(driver, terrain, library, max_frames).run()`，不自建 `EventLoop` / `ApplicationHandler` / `GpuContext::new` / 第二份 `RtsMatch`；验证：源码检索唯有 1 处 `ClientApp::new` 与 1 处 `.run()`，无 `EventLoop::builder()` / `run_app()` 复制
- [ ] 2.3 任何 `f64 → SimFixed` 经 `w3-app::freeze_f64`（ADR 0004），不新增浮点乘法直转；验证：源码检索无 `v * 65536.0` 之类路径，`w3-core` 的 `from_int` / 冻结边界保持唯一
- [ ] 2.4 校验 `crates/w3-core/` 零 diff、不新增权威状态字段、不 bump `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合与快照头部布局不变；验证：`git diff -- crates/w3-core/` 为空，`grep SNAPSHOT_VERSION crates/w3-core/src/snapshot.rs` 仍为 3
- [ ] 2.5 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism`）确认内核零改动且 26 项全绿；验证：测试输出全绿，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致
- [ ] 2.6 确认 example 注释（模块级 `//!`、行内 `//`）一律中文、日志 / 错误文本一律英文结构化 `key=value`、无散落 `println!`、无新增 `.ps1` / `.sh` / `.bat` / `.cmd` 脚本；验证：`cargo fmt --all` 后 `git diff --check` 无行尾空格，源码复核无英文注释

## 3. 本机 cargo build/run（有显示设备机器，CPU 自检）

- [ ] 3.1 执行 `cargo build -p w3-client --example playable` 与 `cargo run -p w3-client --example playable`；验证：编译成功且打开一个真实 winit 窗口进入事件循环，无 `Cargo.toml` 依赖段或既有 API diff
- [ ] 3.2 核对启动日志：`GpuContext::new` 成功、`hud font unavailable` 英文降级日志按已裁定出现、中文标签不上屏；验证：英文结构化 `key=value` 日志可观测，模拟推进未因降级改变
- [ ] 3.3 核对不新增依赖与产物不入库；验证：`Cargo.lock` / `NOTICE.md` 无 diff，`git status` 无 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效 / 截图 / 日志被跟踪

## 4. 人工 GPU 冒烟（人工项，仅能在有 GPU 与显示设备的桌面执行）

> 本组全部为**人工项**：只能由第 1.3 项拍板指定的人与设备逐项执行，CI / headless / 无显示设备环境 MUST 明确报告无法执行、不静默跳过、不改判为通过；清单未逐项勾选前不得归档。

- [ ] 4.1 人工项：确认窗口能正常打开且可关闭；验证：桌面出现 `w3-client` 窗口，人工记录可观察证据
- [ ] 4.2 人工项：确认 `GpuContext::new` 成功、egui HUD overlay pass 正常提交（框选 / 血条等几何元素可见）；验证：首帧画面与英文结构化日志无 `render failed` / `gpu init failed`
- [ ] 4.3 人工项：确认中文标签按已裁定不显示、走英文结构化降级日志（egui 0.37 无 `default_fonts`）；验证：HUD 文字元素不绘制且模拟推进不变
- [ ] 4.4 人工项：依次执行左键框选、右键移动、Q 施法 / T 生产 / C 取消生产；验证：各输入经既有 `ClientApp` 路径生效，命令只经公开提交入口，失败有英文日志明确报告
- [ ] 4.5 人工项：执行 WASD 平移、滚轮缩放、Z-X 旋转相机；验证：画面视角随之变化且逻辑帧推进不受影响
- [ ] 4.6 人工项：拖拽改变窗口尺寸（`Resized`）；验证：画面正常、逻辑帧不跳、累加器与呈现刷新率解耦
- [ ] 4.7 人工项：在高刷（如 144 Hz）显示器持续呈现；验证：逻辑帧恒定 30，逐帧帧号与 30 Hz 基准一致
- [ ] 4.8 人工项：汇总清单勾选状态并明确记录「CI 只覆盖编译 + CPU 测试，未验证 GPU 链路」；验证：清单逐项有通过判据记录，无 GPU 环境条目标注不可执行

## 5. 门禁（编译 + CPU 测试，含确定性重跑）

- [ ] 5.1 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（官方 `rustfmt` 默认风格）；验证：命令退出码 0
- [ ] 5.2 `python tools/check_encoding.py` 通过：UTF-8 无 BOM、LF、文件末尾换行、无行尾空格；验证：输出无 `BOM detected` / `CRLF line ending detected` / `invalid UTF-8`
- [ ] 5.3 `git diff --check` 通过；验证：输出无 whitespace error
- [ ] 5.4 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism`）与 `crates/w3-render/tests/isolation.rs`（`cargo test -p w3-render --test isolation`）全绿；验证：26 项 + 3 项全绿，`w3-core` 零 diff
- [ ] 5.5 `python tools/verify_all.py` 通过（含 `cargo test --workspace`、`cargo build --examples` 与 CI 资产扫描）；验证：输出 `Verification OK`，无 MPQ 派生数据 / 导出资产入库
- [ ] 5.6 复核不新增权威状态、`SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合与快照头部布局不变、表现层状态（累加器余数 / `alpha` / 相机 / 选中 / HUD / 窗口尺寸）不进哈希或快照；验证：`w3-core` 无 diff，`interaction.rs` 的哈希 / 快照字节比对仍全绿
- [ ] 5.7 复核 `openspec validate playable-window-smoke` 与 `openspec validate playable-window-smoke --strict` 仍通过；验证：两条命令均输出 `is valid` 且 `--strict` 无 WARNING

## 6. `critic` 只读复审

- [ ] 6.1 由只读 `critic` 复审确定性、所有权、许可证、编码、资产入库与「单一装配 / 单一权威状态 / `w3-core` 零 diff」；验证：复审结论记录无阻塞项，且明确「CI 未验证 GPU 链路、人工清单已通过或未执行」
- [ ] 6.2 复核无新增依赖、`default_fonts` 未启用、`NOTICE.md` 无需登记、工具脚本跨平台；验证：`Cargo.lock` 无 diff，复审清单逐项勾对 `design.md` D6 / D7

## 7. 中文 commit（本地写入，人工独占 push）

- [ ] 7.1 以中文描述 + Conventional Commits 英文 type 前缀提交，一个改动一个 commit（例：`feat: 新增可玩窗口冒烟 example 入口`），首行 ≤ 72 字符；验证：`git log -1` 显示中文描述，且只含 `crates/w3-client/examples/playable.rs` 一处代码改动
- [ ] 7.2 确认 `git push` 等远端同步命令一律由人工执行；验证：AI 未执行任何 `push` / `fetch` / `pull` / `clone` 命令（此为人工独占，非 AI 任务）

## 8. 归档

- [ ] 8.1 人工 GPU 冒烟清单（第 4 组）逐项勾选、`critic` 复审无阻塞、CI 双平台门禁通过后，由编排者执行 `openspec archive playable-window-smoke -y`；验证：命令成功且归档目录带日期前缀
- [ ] 8.2 确认主 `openspec/specs/` 新增 `playable-window-entry/spec.md`，既有 10 个 capability 未被改写、无其它 change 争用（当前仅 2 个 active change：`gameplay-command-wire-format` 与 `playable-window-smoke`）；验证：`openspec list --specs` 列出新 capability，`git diff -- openspec/specs/` 只新增该目录
