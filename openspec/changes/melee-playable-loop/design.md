# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实，2026-11-12）：主 `openspec/specs/` 共 12 个 capability；`w3-core` 已交付确定性内核与完整玩法命令面——`CommandKind`（`Spawn` / `Move` 含群体分组 / `Stop` / `Attack` / 障碍）与 `GameplayCommandKind`（`Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `SpawnWave` / `SetAi` / `Leave`），以及 `FrozenDefinitions`、`PlayerEconomy`、`VictoryState` + `resolve_victory`（`leave()` 置 `active=false` 后按稳定顺序结算）、`run_ai`（当前只让空闲单位发 `Gather`）。`playable-window-smoke` 已归档并交付 `ClientApp` 装配、只读 HUD、高 DPI 拾取与 `playable-window-entry` 入口，但 example 只有合成碗状地形、4 个四面体与 ASCII 状态窗。用户拍板验收线为「能打完一局」，资产前提为本机具备正版客户端导出条件但本期不依赖。约束：ADR 0001（仓库零资产、只读参考）、ADR 0002（`w3-core` 确定性硬门禁，`SNAPSHOT_VERSION` 当前 3，头部含 `defs_hash` / `content_hash`）、ADR 0004（冻结坐标）、CODEBUDDY.md（中文注释 / 英文日志 / rustfmt / 跨平台脚本 / AI 不碰远端）。

## Goals / Non-Goals

**Goals:**

- 用既有 `w3-core` 公开 API 与命令把一局 1v1 Melee 串成可玩闭环，`w3-core` 零改动。
- 提供能看懂、按得动的命令界面（命令卡 / 选择面板 / 小地图 / 可见生产队列 / 编队）。
- 让农民、战斗单位、建筑、资源点、敌我在画面上可辨识。
- 内容全部为合成 / 原创冻结定义，任何机器零资产可跑；真实导出数据为可插拔接缝。
- 全部呈现与交互只读，不改 FNV-1a 状态哈希、快照字节与 `SNAPSHOT_VERSION`。

**Non-Goals:**

- 改 `w3-core` 帧序、命令排序、状态哈希编码、快照字段或 `SNAPSHOT_VERSION`；新增权威状态字段。
- 引入通用游戏引擎（Bevy 等）、新直接依赖、CJK 字体依赖或任何字体 / 模型 / 纹理资产文件。
- 真实 glTF/MDX 骨骼动画、水体 / 悬崖 / 装饰物全量还原、战争迷雾、JASS/wtg 触发器 VM、战役、Mod API。
- 联机对战接入本局验收（P5 已有 `w3-net` / relay，本 change 只走单机对 AI）。
- 中文上屏（需内嵌 CJK 字体与许可证人工拍板，留后续独立 change）；本期文字层沿用已启用的开源内置字体 ASCII 文案。
- 声称全平台确定性已达成（ADR 0002 目前仅 Windows 与 Debian 12 x86_64 / rustc 1.99.1 经 golden hash 验证）。

## Decisions

### D1：闭环靠「复用既有内核命令 + 场景层装配」，`w3-core` 零改动

- **决策**：`melee-match` 全部行为只经 `w3-core` 既有公开 API 实现——`RtsMatch::create_with_definitions` 注入冻结定义、开局 `Spawn` 命令脚本、`submit_command` / `submit_gameplay_command` 提交采集 / 训练 / 建造 / 战斗命令、`SetAi` 开关、`Leave` 触发既有 `resolve_victory`、`capture_snapshot()` 只读派生 HUD。`crates/w3-core/` MUST 零 diff、不新增权威状态字段、不 bump `SNAPSHOT_VERSION`。
- **备选方案（Alternatives considered）**：
  - *在内核新增「关卡 / 战役状态机」权威字段* — **否决**。新增权威状态必须同时进入状态哈希与快照并 bump 版本（ADR 0002 §6 / §7），会把表现层剧本塞进内核，违反唯一状态所有者与 SRP。
  - *在客户端维护第二份对局状态* — **否决**。违反「表现层不反写」硬红线，且会带来两份真相与哈希漂移。
  - *复用既有命令 + 装配层剧本* — **采纳**。内核只需被「行使」，剧本与呈现留在上层。
- **关联 ADR 0002 与跨平台确定性影响**：`w3-core` 零改动 → 逐帧 `compute_state_hash()`、`capture_snapshot()` 字节、golden hash 与跨平台确定性结论全部不变；命令仍按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序，执行帧由冻结帧号推导、不取墙钟。
- **架构原则与分层依赖**：唯一状态所有者、DIP（装配层只依赖公开 API）、SRP（内核管规则、装配层管开局与剧本）、`w3-core` 不反向依赖上层。
- **实现决策（新增公开项）**：为使 AI 外部命令与逐帧胜负检测不受 `advance_ticks` 墙钟调用粒度影响，`w3-client` 新增 `FixedStepClient::advance_with_hook`——每个逻辑帧 `step()` 之前以 `&mut RtsMatch` 回调。该新增公开项不删改既有签名；唯一调用点 `MeleeDriver::advance_ticks` 在回调内只用 `read_view()` 与 `submit_gameplay_command()`，不写字段、不调用 `step()`、不持有第二份状态。回调暴露可写句柄属设计精度取舍（`critic` residual R8），后续可收窄为「只暴露提交入口」的窄 trait，本 change 不引入该抽象。

### D2：合成 / 原创冻结内容定义，真实导出数据为可插拔接缝

- **决策**：`FrozenDefinitions` 由编译期整数常量构造，至少覆盖农民、主城、兵营、战斗单位、资源点、技能、升级（含 `hp` / `speed_raw` / `damage` / `range_raw` / `supply` / `train_frames` / `mana_cost` / `cast_frames` / 效果）。真实导出数据经一个可选 provider 边界封装（`w3-content::load_map` / SLK 表 / gitignored 本机路径），缺失时回落合成定义、不报错；本期不写实际加载的验收承诺、不读取任何资产、不入库。
- **备选方案（Alternatives considered）**：
  - *本期直接读真实 WC3 导出数据* — **否决**。用户本机虽具备导出条件，但 `config.toml` 与 `assets-export/` 当前为空，dev-setup 需正版 MPQ + Godot + Node，作为主线前置会阻塞；且导出 JSON 的 `f64` 坐标须经 ADR 0004 冻结边界。
  - *把单位数值直接写死在各命令里* — **否决**。无法表达类型 / 造价 / 训练帧，后续接 SLK 需重写。
  - *编译期整数常量冻结定义 + 缺失回落* — **采纳**。零资产、可复现、可平滑接真实数据。
- **关联 ADR 0002 与跨平台确定性影响**：定义开局冻结、内容哈希进入快照头部（既有行为）；`w3-content` 原始 `f64` 坐标 MUST NOT 进入哈希或快照，须经 ADR 0004 冻结边界；本 change 不改 `defs_hash` / `content_hash` 计算方式。
- **架构原则与分层依赖**：ISP（provider 窄接口）、OCP（真实数据以独立 change 扩展，不侵入本 change）、ADR 0001（仓库零资产、只读参考不复制代码）。

### D3：AI 对手用「确定性外部命令源」，不扩展内核 `run_ai`

- **决策**：场景装配层按冻结帧号预生成对手 `GameplayCommand`（采集、训练、周期性骚扰，以及主城被毁时 `Leave { player }`），在开局**一次性**经 `submit_gameplay_command` 入队，由内核按 `ExecuteFrame` 冻结执行；**不开启内核既有 `SetAi` 开关**（避免内部 AI 对玩家空闲单位额外发出采集命令），对手行为只由这一条外部确定性命令源提供。敌方主城被摧毁 → 逐帧 hook 检测后按冻结帧号提交 `Leave` → `leave()` 置 `active=false` → 既有 `resolve_victory` 结算胜负。
- **备选方案（Alternatives considered）**：
  - *扩展 `w3-core::run_ai` 让 AI 会造兵会进攻* — **否决**。改内核、可能新增权威状态，触碰 ADR 0002 硬门禁；且既有 `gameplay-systems` 规定内部 AI 命令不注入重放，调试与验收更困难。
  - *绕过命令队列定时刷兵* — **否决**。绕过公开提交入口，破坏确定性命令流与重放一致性。
  - *外部确定性命令源* — **采纳**。它是外部命令，带 `ExecuteFrame` / `PlayerId` / `Sequence`，重放注入外部命令集即可复现。
- **关联 ADR 0002 与跨平台确定性影响**：AI 命令带固定 `ExecuteFrame` / `PlayerId` / `Sequence`，开局一次性入队、由 `freeze_frame` 按帧冻结执行，提交时机与墙钟调用粒度无关；`Leave` 由每帧 `step()` 前的 hook 按冻结帧号提交。完整重放只注入外部命令、不双重注入内部 AI 命令（符合 `gameplay-systems` 既有 requirement）；`ai_commands_are_frame_frozen_not_clock_driven` 以 1 帧 vs 8 帧批次（`MAX_CATCHUP_STEPS` 上限边界）各推进 120 帧验证状态哈希与快照一致。
- **架构原则与分层依赖**：唯一命令入口、显式失败（非法命令报错不静默）、LoD（AI provider 只依赖公开提交 API）。

### D4：HUD 与输入分层——只读派生 + 命令卡 / 小地图 / 编队

- **决策**：`HudModel` 继续只从只读 `WorldView`、事实事件与只读 `Snapshot` 派生；命令卡、选择面板、小地图、生产队列、编队视图都是纯派生。输入翻译在既有「只经公开提交入口」基础上新增 `Gather`、建造 `Train { is_building: true }`、`CancelTrain`、编组 / 按组选择、群体 `Move`（复用既有 `group` 字段按稳定 ID 分配落点）。UI 文案走 i18n、以现有开源内置字体 ASCII 显示；中文上屏留后续独立 change。
- **备选方案（Alternatives considered）**：
  - *HUD 持有对局状态副本以便交互* — **否决**。表现层反写 / 非权威状态进哈希风险，违反 `playable-client` 既有契约。
  - *本期引入 CJK 字体实现中文上屏* — **否决**。新增字体依赖与许可证变更属 CODEBUDDY.md §10 人工独占，且会把合规问题带进本 change。
  - *只读派生 + ASCII 文案 + 后续中文扩展* — **采纳**。文字层可见、合规边界不破。
- **关联 ADR 0002 与跨平台确定性影响**：HUD 数据与选中集合 MUST NOT 进入哈希或快照；输入执行帧由冻结帧号推导，不用墙钟或呈现帧号；`isolation.rs` 全绿。
- **架构原则与分层依赖**：ISP（HUD 只读派生）、OCP（中文上屏独立扩展）、`w3-core` 不反向依赖 `w3-client`。

### D5：程序化几何按类型区分，履行既有呈现契约

- **决策**：`w3-render` 扩展 `Scene` 按实体类型（农民 / 战斗单位 / 建筑 / 资源点）生成不同程序化几何，配队伍色、选中环与血条；这是对 `3d-rendering` 既有「Skeletal animation and team color」与「Terrain and heightfield presentation」的最小可辨识子集，真实 glTF/MDX 模型与骨骼动画留后续。
- **备选方案（Alternatives considered）**：
  - *本期接入真实 MDX/glTF 模型* — **否决**。需导出资产与模型解析，阻塞主线且资产不入库。
  - *维持四面体占位* — **否决**。用户明确判定「根本没法玩」，无法分辨类型与敌我。
  - *程序化几何 + 队伍色 + 选中环* — **采纳**。零资产、类型可辨、改动集中在表现层。
- **关联 ADR 0002 与跨平台确定性影响**：纯表现层几何与颜色 MUST NOT 反写权威状态、进入哈希或快照；`capture_snapshot()` 字节与只读基线一致。
- **架构原则与分层依赖**：SRP（呈现只读）、ADR 0001（零资产）、`3d-rendering` 既有契约不被改写。

### D6：入口复用 `ClientApp`，不改既有公开签名

- **决策**：新增一个 example（或扩展既有 `playable.rs`）装配「一局」：`FixedStepClient::from_match` → `ClientApp::new(driver, terrain, library, max_frames).run()`，复用 `playable-window-entry` 既有入口与装配；新增公开项限于 `w3-client` / `w3-render`，不删改既有签名。
- **备选方案（Alternatives considered）**：
  - *新写宿主事件循环 / 第二套呈现管线* — **否决**。违反 `playable-client` 的「Single presentation pipeline seam」与 CARP。
  - *把闭环塞进 `playable.rs` 一个巨型 main* — **否决**。SRP；改为「定义 / 装配 / AI provider / HUD」分模块。
- **关联 ADR 0002 与跨平台确定性影响**：`step()` 仍由冻结 30 Hz 整数累加器独家驱动；`Resized` 不触发 `step()`；表现层状态不进哈希或快照。
- **架构原则与分层依赖**：单一装配、单一权威状态、DIP（example 只依赖公开 API）。

## Risks / Trade-offs

- [AI 太弱导致「没有对手感」或打不完一局] → D3 命令源至少覆盖采集、训练与周期性进攻；人工验收清单逐项勾「一局可打完」；若强度仍不足，作为后续独立 change 增强，**不在本 change 扩内核**。
- [合成定义数值不合理导致闭环卡住（资源不够 / 训练过慢）] → CPU 自测用固定命令脚本跑完整一局（采集 → 造兵营 → 训兵 → 摧毁主城 → 结算）验证可达；数值全为编译期常量，便于一次性校准。
- [真实导出接缝被误当成已实现] → proposal / spec / design / tasks 四处均声明本期不依赖、不入库；接缝只保留 provider 边界与缺失回落，不写实际加载的验收承诺。
- [中文未上屏被误认为 HUD 不可见] → ASCII 文案 + 已启用的开源内置字体保证文字层可见；中文上屏与 CJK 字体明确留后续独立 change（CODEBUDDY.md §10 人工独占）。
- [`w3-core` 被无意改动破坏确定性] → `git diff -- crates/w3-core/` 为空、`determinism` 26 项 / `isolation` 3 项 / `cargo test --workspace` 全绿、`SNAPSHOT_VERSION` 仍为 3 作硬门禁；critic 复审逐项勾对。
- [人工 GPU 冒烟被 CI 绿灯冒充] → CI 只覆盖编译 + CPU 测试；真窗口「打完一局」只能人工在有 GPU / 显示设备桌面执行，清单未逐项勾选前不得归档。
- [新增依赖 / 许可证越界] → 本 change 不新增直接依赖；CJK 字体、真实模型、开源资产若后续引入，走 CODEBUDDY.md §10 人工拍板 + `NOTICE.md` 登记 + `Cargo.lock` 提交。
- [参考项目代码被复制或 AGPLv3 组合] → 四个参考项目只读；`WarsmashModEngine`(AGPLv3) 仅作语义参考，未经 ADR 复核不得引入其代码。
- [命令卡 / 小地图交互被误接成权威命令] → 小地图点击只平移表现层相机；选中集合、编组、HUD 布局全部表现层专属，`isolation.rs` 与逐帧哈希比对守住。
- [窗口内「重开」按键未实现] → 本 change 验收线为「打完一局」；spec 的重开 Scenario 已与实现对齐为「以相同冻结定义与开局脚本重新构造一局」（CPU 层 `full_script_plays_one_match_to_victory_and_restart_is_clean` 验证），窗口内按键式重开留后续独立 change（`critic` residual R2）。
- [命令卡 `Move` / `Attack` 以当前光标为目标，与按钮点击位置无关；`pending_input_action` 单槽同帧仅保留最后一个动作] → 属交互可用性限制（`critic` residual R7），人工清单 8.2 重点观察；不改确定性契约，后续 change 可引入「待指定目标」态。
- [AI 脚本开局一次性入队，`pending` 含 `AI_HORIZON = 3600` 帧全部命令，超过后对手命令耗尽] → 属规模 / 性能边界与已知设计限制（`critic` residual R11），不影响确定性；`GameplayCommandQueue` 的哈希与快照序列化 `pending`，HUD 每帧 `snapshot()` 有额外开销，人工 8.4 观察高刷表现。
- [确定性回归只比对终态哈希 / 快照，批次覆盖 ≤ `MAX_CATCHUP_STEPS = 8`] → 终态完整快照相等足以排除分叉；>8 帧批次会触发宿主层显式追帧丢弃（设计保护，非不确定性），MUST NOT 据此声称任意卡顿 / 任意粒度已验证（`critic` residual R10）。
- [新增测试断言消息沿用仓库既有中文惯例] → 运行时日志 / 错误文本一律英文（CODEBUDDY.md §3），测试 `assert!` 中文消息为仓库既有先例（`critic` residual R4），不进日志 / 权威状态 / 哈希；后续可统一改为英文 `key=value`。

## Migration Plan

1. **规范侧（本轮）**：写 `openspec/changes/melee-playable-loop/` 下 proposal / specs / design / tasks 四件套；`openspec validate melee-playable-loop --strict` 通过；`python tools/check_encoding.py` 与 `git diff --check` 通过。不碰主 specs / 其它 change / `crates/` / 根配置。
2. **用户拍板后 `/opsx:apply`**：由对应 owner 实施（`w3-client` / `w3-render` 为主，`w3-core` 零改动），按 tasks 分组推进；写完 `cargo fmt --all`、提交前 `cargo fmt --all -- --check`。
3. **CPU 自测**：固定命令脚本跑完一局；`cargo test -p w3-core --test determinism`（26 项）、`crates/w3-render/tests/isolation.rs`（3 项）、`cargo test --workspace`、`cargo build --examples`、`python tools/verify_all.py` 全绿。
4. **本机自检（有显示设备机器）**：`cargo build -p w3-client --example melee` 与 `cargo run -p w3-client --example melee`，确认窗口打开、HUD 文字层与命令卡 / 小地图可见、英文结构化日志可观测。
5. **人工 GPU 冒烟（人工项）**：由用户在有 GPU / 显示设备桌面逐项执行清单，含「从开局到分出胜负打完一局」；无 GPU 环境明确报告不可执行、不静默跳过、不改判通过。
6. **`critic` 复审 → 本地中文 commit → 人工 push → CI 双平台门禁 → `/opsx:archive`**：归档由编排者执行；AI 不执行任何 `push` / `fetch` / `pull` / `clone`。
7. **回滚**：删除新增 example 与 HUD / 呈现扩展，恢复 `w3-client` / `w3-render` 改动；`w3-core` 零改动、依赖零改动、`SNAPSHOT_VERSION` 不变，回滚不影响权威状态或其它 change。

## Open Questions

> 以下均为**可延后**的后续范围，不影响本 change 的 specs、方案与任务分解；若纳入本期，须回到对应 artifact 修订。

- **真实导出数据接入阶段**：用户本机具备正版客户端导出条件；在哪一阶段用 `w3-content` 真实数据替换合成定义、是否引入 SLK 数值，留后续独立 change（D2）。
- **中文上屏与 CJK 字体**：需内嵌开源 CJK 字体，涉及新增依赖与许可证（CODEBUDDY.md §10 人工独占），留后续独立 change（D4）。
- **真实模型 / 骨骼动画 / 迷雾 / 联机接入本局**：留 P7 后续阶段，不在本期验收线内（D5、D6）。
- **AI 强度与多难度**：本期保证「有对手、可打完、可结算」；更丰富的 AI 行为（编队、科技、多难度）留后续 change（D3）。
