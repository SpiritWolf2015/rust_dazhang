## 1. 前置决策与规范基线（用户拍板项，已结案）

- [x] 1.1 用户拍板 a（已完成）：UI 依赖定案 `egui` 家族（`egui` + `egui-wgpu` + `egui-winit`，`egui` 取 MIT 一侧）；`wgpu 30` / `winit 0.30` 版本兼容性仍须在 apply 阶段核验，不兼容则明确报告并回退自绘 2D overlay，MUST NOT 静默降级或强行升级
- [x] 1.2 用户拍板 b（已完成）：第 5 项 `w3-net` session 接入纳入本期，不缩减范围；保留第 8 组任务与 `lockstep-networking` delta（含 MODIFIED 块与两条 ADDED requirement）
- [x] 1.3 用户拍板 c（已完成）：新 crate 命名定案 `w3-client`
- [x] 1.4 用户拍板 d（已完成）：归档顺序为先归档 P5、再 apply P6；`openspec archive p5-gameplay-systems -y` 由编排者随后执行，本 change 不自行归档、不预断 P5 已归档
- [x] 1.5 单一 owner 归属（已完成）：实施 owner 已指定 `client-owner`，由编排者新建该 agent 定义（不在 `spec-steward` 可写范围），满足 CODEBUDDY.md §8「一个 change 一个 owner」
- [x] 1.6 `config.yaml` rules.proposal 阶段枚举（已完成）：按用户拍板保持「P0-P5」不动，不改为「P0–P6」；proposal 继续双标 P6 与 P3/P4/P5 能力面
- [x] 1.7 复核 ADR 0003 分期表 P5 已完成 / P6 行 / 「不引入通用游戏引擎」已同步（本轮提案阶段已写入，已复核）
- [x] 1.8 复核 `openspec/config.yaml` 「快照 v1」已同步为 `SNAPSHOT_VERSION`（当前 3）（本轮提案阶段已写入，已复核）
- [ ] 1.9 建议交 `docs-owner` 同步 `CODEBUDDY.md:49` / `CODEBUDDY.md:8` 过期措辞（本 change 不可写该文件）
- [x] 1.10 新增 UI 依赖授权（已完成）：用户已按 CODEBUDDY.md §10 授权新增该 MIT 依赖与后续 `NOTICE.md` 登记（`NOTICE.md` 仍属 `docs-owner`，本 change 不自行编写）；后续依赖重大升级 / 许可证变更仍属人工独占，须重新拍板

## 2. 归档 P5 与打开 P6 apply

- [ ] 2.1 执行 `openspec archive p5-gameplay-systems -y`
- [ ] 2.2 确认主 `openspec/specs/deterministic-simulation/spec.md` 与 `gameplay-systems/spec.md` 已吸收 P5 delta
- [ ] 2.3 apply 前复跑 `openspec validate p6-playable-client` 与 `python tools/check_encoding.py`

## 3. 新 crate 与依赖合规

- [ ] 3.1 新建 crate（建议 `w3-client`）并在虚拟 workspace 注册，只依赖 `w3-core` / `w3-render` /（可选）`w3-net` / `w3-content`
- [ ] 3.2 新增依赖前检查许可证与 GPLv3 兼容性（`egui` 取 MIT 一侧），并提交 `Cargo.lock`
- [ ] 3.3 把 `NOTICE.md` 第三方代码登记列为 `docs-owner` 后续任务（本 change 不自行编写）
- [ ] 3.4 工具脚本一律用跨平台 Python / Node，不写 `.ps1` / `.sh` / `.bat` / `.cmd`

## 4. 固定 30 Hz 累加器与呈现插值（最高优先，BREAKING）

- [ ] 4.1 实现整数帧累加器（`logic_fps = 30` 冻结），设显式追帧上限，超限以英文结构化日志明确报告
- [ ] 4.2 改 `FrameProducer::next_frame()` 只交出当前只读 `WorldView` / 事件拷贝，MUST NOT 调用 `RtsMatch::step()`；迁移 `SimProducer`
- [ ] 4.3 渲染帧改用 `Scene::interpolate_all(alpha)`，`alpha` 只来自累加器余数且只存在于表现层
- [ ] 4.4 增加 60 Hz / 144 Hz 双刷新率重放验收场景，逐帧帧号与状态哈希一致
- [ ] 4.5 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism`）确认 `w3-core` 零改动且全绿
- [ ] 4.6 重跑 `crates/w3-render/tests/isolation.rs` 三项只读隔离验收

## 5. 输入 → 命令翻译

- [ ] 5.1 实现 winit 键鼠事件 → `Command` / `GameplayCommand` 翻译层
- [ ] 5.2 覆盖选中（表现层状态）、移动、攻击、施法 `Cast`、生产 `Train`、取消生产 `CancelTrain` 与生产队列呈现
- [ ] 5.3 命令只经 `submit_command` / `submit_gameplay_command` 提交，携带冻结帧号推导的 `ExecuteFrame` / `PlayerId` / `Sequence`
- [ ] 5.4 键鼠坐标到 `SimFixed` 的转换走既有冻结边界（ADR 0004 / `w3-app::freeze_f64`），不新增浮点→权威状态路径
- [ ] 5.5 提交失败显式返回错误，不静默降级
- [ ] 5.6 验收：表现层不存在对 `RtsMatch` 字段的直接写入路径

## 6. UI / HUD

- [ ] 6.1 按拍板 a 落地 UI 方案（`egui` + `egui-wgpu` / `egui-winit`，或自绘 2D overlay）
- [ ] 6.2 核验 `egui-wgpu` / `egui-winit` 与 `wgpu 30` / `winit 0.30` 版本兼容；不兼容则明确报告并回退自绘，不强行升级
- [ ] 6.3 HUD 只从 `WorldView` / 事实事件绘制选中框、血条、资源、生产队列（技能栏可选）
- [ ] 6.4 UI 文案走 i18n（可中文），不进日志、权威状态与状态哈希
- [ ] 6.5 UI 字体 / 图标不含暴雪资产或 MPQ 派生数据，且不入库
- [ ] 6.6 字体 / 资源加载失败以英文日志明确报告并降级为不绘制对应元素
- [ ] 6.7 重跑 `crates/w3-core/tests/determinism.rs` 与 `crates/w3-render/tests/isolation.rs` 确认 HUD 不改权威哈希

## 7. 可交互相机

- [ ] 7.1 扩展 `w3-render::gpu::Camera` 支持平移 / 缩放 / 旋转，提供构造器或 `Default` 收窄 BREAKING 破坏面
- [ ] 7.2 迁移 `render_frame.rs` 的 `Camera { aspect }` 结构体字面量调用点
- [ ] 7.3 `WindowEvent::Resized` 只更新 `aspect` 与交换链，MUST NOT 触发 `step()`
- [ ] 7.4 验收：相机操作后逐帧 `compute_state_hash()` 与未操作实例一致，快照仍为 v3

## 8. `w3-net` session 接入（条件性：仅当拍板 b = 是；否则删除本组）

- [ ] 8.1 实现 `ClientSession` → `FrameProducer` 适配，复用同一 `RendererApp::run()` / `Scene` / `GpuContext` 管线
- [ ] 8.2 适配层只提交已冻结帧的输入命令，经 `submit_command` / `apply_frame`，全程只有一个 `RtsMatch`
- [ ] 8.3 网络抖动 / 重传 / 乱序 / 断线恢复 MUST NOT 驱动 30 Hz 累加器、改 `step()` 次数或改命令排序
- [ ] 8.4 校验 `W3N1` 25 字节帧头、protobuf 编码约束、逐帧 checksum 与 `first_difference()` 语义不变（ADR 0005）
- [ ] 8.5 验收：联机接入不新增权威状态，快照仍 v3 且头部含 `defs_hash`
- [ ] 8.6 重跑 `crates/w3-core/tests/determinism.rs` 确认 `w3-core` 零改动且不反向依赖上层 crate

## 9. 编码、格式、确定性与归档验收

- [ ] 9.1 源码注释（含模块级 `//!`、pub rustdoc `///`、行内 `//`）一律中文；日志 / 错误 / `panic!` 一律英文且结构化 `key=value`，无散落 `println!`
- [ ] 9.2 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（官方 `rustfmt` 默认风格，仓库无自定义配置）
- [ ] 9.3 `python tools/check_encoding.py` 通过：UTF-8 无 BOM、LF、文件末尾换行、无行尾空格
- [ ] 9.4 `cargo test --workspace` 与 `python tools/verify_all.py` 通过
- [ ] 9.5 重跑 `crates/w3-core/tests/determinism.rs` 最终门禁全绿，并确认状态哈希字段集合与快照头部布局未变
- [ ] 9.6 CI 资产扫描通过：无 MPQ 派生数据、真实导出 JSON / PNG / GLB / 音效入库
- [ ] 9.7 git 提交信息描述一律中文（Conventional Commits 英文 type 前缀保留），一个改动一个 commit
- [ ] 9.8 `critic` 只读审查确定性、所有权、许可证、编码与资产合规
- [ ] 9.9 `openspec archive p6-playable-client -y` 并确认主 `openspec/specs/` 新增 `playable-client`、`3d-rendering` 与 `lockstep-networking` 已吸收 delta
