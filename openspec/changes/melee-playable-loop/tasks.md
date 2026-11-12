# Tasks

> 本轮只立规范、不实施代码（`/opsx:apply` 为后续动作），故以下全部不预勾。按 `design.md` D1–D6 决策分组，每组自带其工作所需的测试与验证；`w3-core` 全程零改动。工具 / 命令名保持原样，任务描述一律中文。

## 1. 规范基线与合成内容定义

- [x] 1.1 复核四件套 `proposal.md` / `specs/melee-match/spec.md` / `specs/rts-command-hud/spec.md` / `design.md` / `tasks.md` 与 `openspec validate melee-playable-loop --strict` 通过；验证：命令输出 `is valid` 且无 WARNING
- [x] 1.2 新增合成 / 原创冻结定义模块（农民、主城、兵营、战斗单位、资源点、技能、升级，全部编译期整数常量）；验证：单元测试覆盖各 `DefId` 与 `defs_hash` 固定，且无 `f64 → SimFixed` 直转
- [x] 1.3 新增开局 `Spawn` 命令脚本（双方主城 / 农民 / 资源点，冻结整数坐标，同一玩家内序列唯一）；验证：单元测试校验命令条数、坐标冻结与双方实体归属
- [x] 1.4 定义真实导出数据 provider 边界（缺失回落合成定义、gitignored 本机路径、本期不读取不入库）；验证：单元测试验证缺失路径回落且无文件 IO
- [x] 1.5 复核 `crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍为 3、不新增权威状态字段；验证：`git diff -- crates/w3-core/` 为空

## 2. 对局装配与闭环 CPU 自测

- [x] 2.1 实现 `melee` 场景装配（`RtsMatch::create_with_definitions` + 开局脚本 + `FixedStepClient::from_match`）；验证：单元测试可构造对局并推进若干整数帧
- [x] 2.2 实现确定性外部 AI 命令源（按冻结帧号经 `submit_gameplay_command` 提交采集 / 训练 / 周期性进攻，主城被毁提交 `Leave`）；验证：单元测试证明命令带 `ExecuteFrame` / `PlayerId` / `Sequence` 且经同一校验入口
- [x] 2.3 接通资源采集 / 建造 / 训练 / 战斗的既有命令路径；验证：CPU 测试断言资源增长、队列剩余帧递减、完成后生成实体
- [x] 2.4 接通胜负结算与重开（复用既有 `resolve_victory`）；验证：CPU 测试断言唯一胜者、结算帧与重开后无上一局残留
- [x] 2.5 固定命令脚本跑完一局的端到端 CPU 测试（采集 → 造兵营 → 训兵 → 摧毁主城 → 结算）；验证：测试通过且可重复，逐帧哈希与基线一致
- [x] 2.6 重跑 `cargo test -p w3-core --test determinism`（26 项）确认内核零改动；验证：全绿，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致

## 3. 可辨识呈现（`w3-render`）

- [x] 3.1 按实体类型生成不同程序化几何（农民 / 战斗单位 / 建筑 / 资源点）；验证：`w3-render` 单元测试断言各类型几何输出不同
- [x] 3.2 队伍色、选中环、血条按既有 `3d-rendering` 契约呈现；验证：单元测试断言颜色与选中环随只读状态派生
- [x] 3.3 呈现只读：实体外观变化不改哈希与快照；验证：`crates/w3-render/tests/isolation.rs` 3 项全绿、逐帧哈希比对一致

## 4. 命令 HUD（`w3-client`）

- [x] 4.1 命令卡与选择面板（按选中类型显示可用动作与类型 / 生命 / 归属 / 资源）；验证：HUD 单元测试覆盖农民、生产者、空选中三态
- [x] 4.2 生产队列可见（条目、单位类型、剩余帧递减、完成 / 取消刷新）；验证：HUD 单元测试断言队列派生与刷新，不出现「生效但不可见」
- [x] 4.3 小地图（地形轮廓、实体点、相机视口框，点击只平移相机）；验证：单元测试断言小地图派生且点击不生成权威命令
- [x] 4.4 文字层与 i18n（现有开源内置字体 ASCII 文案，中文上屏留后续）；验证：启动无文字层缺失降级，文案不进日志 / 权威状态 / 状态哈希
- [x] 4.5 HUD 只读派生不改确定性；验证：`tests/interaction.rs` 的哈希 / 快照字节比对全绿

## 5. 输入翻译

- [x] 5.1 新增 `Gather` / 建造 `Train { is_building: true }` / `CancelTrain` 输入动作，只经公开提交入口；验证：交互测试断言生成对应玩法命令
- [x] 5.2 编队（编组 / 按组选择）与群体 `Move`（按稳定实体 ID 分配落点）；验证：交互测试断言选中集合不改哈希、群体命令分组字段正确
- [x] 5.3 命令卡点击与小地图点击接入（前者翻译为命令，后者仅平移相机）；验证：交互测试断言小地图点击不生成命令

## 6. 可玩入口 `melee`

- [x] 6.1 新增 `crates/w3-client/examples/melee.rs`，复用 `ClientApp::new(driver, terrain, library, max_frames).run()` 单一装配；验证：源码检索仅 1 处 `ClientApp::new` 与 1 处 `.run()`，无自建 `EventLoop` / `GpuContext` / 第二份 `RtsMatch`
- [x] 6.2 `cargo build -p w3-client --example melee` 与 `cargo build --examples`；验证：编译成功、无新增直接依赖
- [ ] 6.3 本机 `cargo run -p w3-client --example melee`（有显示设备机器）；验证：窗口打开、HUD 文字层 / 命令卡 / 小地图可见、英文结构化日志可观测 —— **本轮 CLI 未执行真窗口（`MAX_FRAMES = u32::MAX` 不会自动退出），与第 8 组人工 GPU 冒烟一并留用户本机执行；编译已由 6.2 与 `cargo test --workspace` 双重覆盖**

## 7. 全量门禁与确定性复核（集成检查）

- [x] 7.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`；验证：命令退出码 0
- [x] 7.2 `python tools/check_encoding.py` 与 `git diff --check`；验证：无 BOM / CRLF / 行尾空格
- [x] 7.3 `cargo test --workspace`、`cargo test -p w3-core --test determinism`（26 项）、`isolation`（3 项）；验证：全绿，`git diff -- crates/w3-core/` 为空
- [x] 7.4 `python tools/verify_all.py`（含 `cargo build --examples` 与 CI 资产扫描）；验证：输出 `Verification OK`，无 MPQ 派生数据 / 导出资产入库
- [x] 7.5 复核 `SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合与快照头部布局不变、`Cargo.lock` / `NOTICE.md` 无意外 diff；验证：逐项 diff 核对
- [x] 7.6 `openspec validate melee-playable-loop` 与 `openspec validate melee-playable-loop --strict`；验证：均输出 `is valid` 且 `--strict` 无 WARNING

## 8. 人工 GPU 冒烟（人工项，仅能在有 GPU 与显示设备的桌面执行）

> 本组全部为**人工项**：只能由用户在本机逐项执行。**执行者与设备登记：由用户在本机（有 GPU / 显示设备桌面）执行；用户本机具备正版经典 WC3 客户端可导出真实资产，但本 change 零资产依赖、不读取本机导出目录。** **运行记录：本轮 AI 侧已完成 CPU 自测——`w3-client` 11 项单测、`melee_loop` 7 项端到端（含「采集→建造→训练→摧毁主城→胜负结算→重开」一局可打完，以及新增 `ai_commands_are_frame_frozen_not_clock_driven` 确定性回归）、`interaction` 21 项、`w3-render` 28 项、`isolation` 3 项、`determinism` 26 项、`cargo test --workspace` 全绿，`verify_all.py` → `Verification OK`；6.2 编译 example 通过。6.3 真窗口与 8.1–8.5 仍待用户本机逐项勾选。** 当前 CLI 会话标注「待人工在有 GPU / 显示设备桌面执行」；CI / headless / 无显示设备环境 MUST 明确报告无法执行、不静默跳过、不改判为通过；**CI 只覆盖编译 + CPU 测试，未验证 GPU 链路**；清单未逐项勾选前不得归档。

- [ ] 8.1 人工项：窗口能开可关，HUD 文字层 / 命令卡 / 选择面板 / 小地图 / 生产队列可见；验证：人工记录可观察证据
- [ ] 8.2 人工项：框选、右键移动 / 攻击、采集、建造、训练、取消、编队逐项生效且失败有英文日志；验证：人工记录观察
- [ ] 8.3 人工项：从开局到分出胜负**打完一局**（含 AI 对手行为、胜负显示与重开）；验证：人工记录完整剧本与结果
- [ ] 8.4 人工项：WASD / 滚轮 / Z-X 相机、拖拽 `Resized`、高刷下逻辑帧恒定 30；验证：人工记录画面与帧号
- [ ] 8.5 人工项：汇总清单勾选状态并记录「CI 只覆盖编译 + CPU 测试，未验证 GPU 链路」；验证：逐项有通过判据，无 GPU 环境标注不可执行

## 9. `critic` 复审与本地提交（AI 侧）

- [x] 9.1 由只读 `critic` 复审确定性、所有权、许可证、编码、资产入库、`w3-core` 零 diff 与只读呈现边界；验证：复审结论无阻塞项，且明确「CI 未验证 GPU 链路、人工清单已通过或未执行」—— 复审首轮指出 1 项阻塞（AI 命令按 `advance_ticks` 墙钟调用粒度提交，破坏逐帧确定性，对应 `driver.rs` 的 `submit_due_commands`），已修为「开局一次性入队 + 逐帧 hook 提交 `Leave`」并补 `ai_commands_are_frame_frozen_not_clock_driven` 回归；复复审结论 **无阻塞项**，`w3-core` 零 diff、`SNAPSHOT_VERSION` 仍为 3；明确确认「CI 只覆盖编译 + CPU 测试、未验证 GPU 链路」「人工清单 6.3 / 8.1–8.5 未执行」「`w3-core` 零 diff、SNAPSHOT_VERSION 仍为 3」。12 项非阻塞 residual 中 R1 / R2 / R3 / R5 / R6 / R9 / R10 / R12 已修订或对齐，R4 / R7 / R8 / R11 登记为 `design.md` Risks 已知限制
- [x] 9.2 以中文描述 + Conventional Commits 英文 type 前缀本地 commit（首行 ≤ 72 字符）；验证：`git log -1` 显示中文描述，且包含本 change 全部代码与四件套改动
- [x] 9.3 确认 `git push` 等远端同步命令一律由人工执行；验证：AI 未执行任何 `push` / `fetch` / `pull` / `clone` 命令（人工独占）

## Workflow follow-up

- 人工 `git push` → CI 双平台 `python tools/verify_all.py` 绿灯。
- 由编排者执行 `/opsx:archive`（`openspec archive melee-playable-loop -y`），确认主 `openspec/specs/` 新增 `melee-match` 与 `rts-command-hud`，既有 12 个 capability 未被改写。
- 后续独立 change：真实导出数据接入（`w3-content` + 正版客户端导出）、中文上屏（CJK 字体与许可证人工拍板）、真实 glTF/MDX 模型与骨骼动画、AI 强度与多难度、联机接入本局。
