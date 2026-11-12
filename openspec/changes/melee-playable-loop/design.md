# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实，2026-11-12）：主 `openspec/specs/` 共 12 个 capability；`w3-core` 已交付确定性内核与完整玩法命令面——`CommandKind`（`Spawn` / `Move` 含群体分组 / `Stop` / `Attack` / 障碍）与 `GameplayCommandKind`（`Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `SpawnWave` / `SetAi` / `Leave`），以及 `FrozenDefinitions`、`PlayerEconomy`、`VictoryState` + `resolve_victory`（`leave()` 置 `active=false` 后按稳定顺序结算）、`run_ai`（当前只让空闲单位发 `Gather`）。`playable-window-smoke` 已归档并交付 `ClientApp` 装配、只读 HUD、高 DPI 拾取与 `playable-window-entry` 入口，但 example 只有合成碗状地形、4 个四面体与 ASCII 状态窗。用户拍板验收线为「能打完一局」，随后进一步判定合成色块 demo 不是 War3 地图画面、要求对齐 `godot_warcraft3` 视觉水位，追加真实地图表现层阶段（阶段 1 真实 Echo Isles 地形 / 静态摆放、阶段 2 真实地表贴图 / 小地图底图），详见 D7。约束：ADR 0001（仓库零资产、只读参考）、ADR 0002（`w3-core` 确定性硬门禁，`SNAPSHOT_VERSION` 当前 3，头部含 `defs_hash` / `content_hash`）、ADR 0004（冻结坐标）、CODEBUDDY.md（中文注释 / 英文日志 / rustfmt / 跨平台脚本 / AI 不碰远端）。

## Goals / Non-Goals

**Goals:**

- 用既有 `w3-core` 公开 API 与命令把一局 1v1 Melee 串成可玩闭环，`w3-core` 零改动。
- 提供能看懂、按得动的命令界面（命令卡 / 选择面板 / 小地图 / 可见生产队列 / 编队）。
- 让农民、战斗单位、建筑、资源点、敌我在画面上可辨识。
- 玩法闭环全部为合成 / 原创冻结定义，任何机器零资产可跑；真实导出数据仅作**表现层**可插拔接缝。
- 在真实 Echo Isles 地图上呈现一局（真实高度场 / 地表分区 / 水体 / 悬崖 / 静态摆放 / BLP→PNG 地表贴图 / 真实小地图底图），对齐 `godot_warcraft3` 视觉水位。
- 全部呈现与交互只读，不改 FNV-1a 状态哈希、快照字节与 `SNAPSHOT_VERSION`。

**Non-Goals:**

- 改 `w3-core` 帧序、命令排序、状态哈希编码、快照字段或 `SNAPSHOT_VERSION`；新增权威状态字段。
- 引入通用游戏引擎（Bevy 等）、CJK 字体依赖，或向仓库提交任何字体 / 模型 / 纹理 / 导出资产文件（ADR 0001）。
- 把真实 SLK 数值灌进 `FrozenDefinitions` 等权威内容定义（本期真实数据只进表现层）；改动 `defs_hash` / `content_hash` 计算方式。
- 真实 glTF/MDX 骨骼模型、逐像素 GPU 纹理采样、战争迷雾、JASS/wtg 触发器 VM、战役、Mod API（均为后续阶段已知限制，见 D7 / Risks）。
- 联机对战接入本局验收（P5 已有 `w3-net` / relay，本 change 只走单机对 AI）。
- 中文上屏（需内嵌 CJK 字体与许可证人工拍板，留后续独立 change）；本期文字层沿用已启用的开源内置字体 ASCII 文案。
- 声称全平台确定性已达成（ADR 0002 目前仅 Windows 与 Debian 12 x86_64 / rustc 1.99.1 经 golden hash 验证）。

> **依赖口径说明**：初版 Goals 写「不新增直接依赖」，指玩法闭环与 HUD；阶段 2 为解码本机 BLP→PNG 贴图，经用户授权新增 1 个直接依赖 `image`（png only）及 13 个传递依赖，许可证逐一核对并登记 `NOTICE.md`（详见 D7）。此处如实同步，不冒充零依赖。

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

- **决策**：`FrozenDefinitions` 由编译期整数常量构造，至少覆盖农民、主城、兵营、战斗单位、资源点、技能、升级（含 `hp` / `speed_raw` / `damage` / `range_raw` / `supply` / `train_frames` / `mana_cost` / `cast_frames` / 效果）。**权威内容定义保持合成 / 零资产**，不受真实导出影响。真实导出数据另有两条只读通道：`w3-content::load_map` / `textures` 读取本机 gitignored 的地形 / 摆放 / PNG 贴图，仅经 `real_map` 装配成表现层 `TerrainMesh` / `StaticMarker` / 顶点色 / 小地图底图（D7）；把真实 SLK 数值灌进 `FrozenDefinitions` 仍属后续范围、未实施。**CPU 对局装配（`FrozenDefinitions` 构造 + `MeleeDriver` / `melee_loop.rs`）不读取任何资产、缺资产不报错**；`melee` 可玩 example 的表现层按 D7 依赖 gitignored `config.toml`，缺失则显式失败、不回落合成 demo。
- **备选方案（Alternatives considered）**：
  - *本期把真实 WC3 导出数值灌进权威定义* — **否决**。导出 `f64` 坐标 / 单位数值须经 ADR 0004 冻结边界，且会改 `defs_hash` / `content_hash`、触碰确定性门禁；作为主线前置会阻塞「能打完一局」。
  - *把单位数值直接写死在各命令里* — **否决**。无法表达类型 / 造价 / 训练帧，后续接 SLK 需重写。
  - *编译期整数常量冻结定义 + 真实数据只进表现层* — **采纳**。权威状态零资产、可复现；视觉层用真实导出对齐 `godot_warcraft3` 水位，两者分层互不污染。
- **关联 ADR 0002 与跨平台确定性影响**：定义开局冻结、内容哈希进入快照头部（既有行为）；`w3-content` 原始 `f64` 坐标、PNG 像素、静态摆放 MUST NOT 进入哈希或快照，只存在于表现层 `f32` / `RgbaImage` / 顶点色；本 change 不改 `defs_hash` / `content_hash` 计算方式。
- **架构原则与分层依赖**：ISP（provider 窄接口）、OCP（真实数据以独立通道扩展，不侵入权威定义）、ADR 0001（仓库零资产、只读参考不复制代码）、唯一状态所有者。

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

### D7：真实地图表现层分两阶段接入，`w3-core` 与权威契约零改动

- **背景**：实施期间用户判定「合成色块窗口不是 War3 地图画面」，要求对齐 `godot_warcraft3` 视觉水位（真实 Echo Isles 地形 + 真实贴图）。这是**表现层范围追加**，不改 `melee-match` / `rts-command-hud` 的行为契约，也不改 `w3-core`。
- **决策（阶段 1，commit `4db13ab`）**：`w3-client::real_map` 读取本机 `map-parsed/echoisles/` 导出 JSON，按 `WORLD_SCALE = 2/128` 把 129×97 高度场（12513 顶点）、6 地表分区、水体 / 崖层、97 单位 + 2491 装饰物摆放缩放装配成表现层 `TerrainMesh` + `StaticMarker`。缩放目标让 `columns × cell_size` 与既有玩法世界对齐，故相机 / 拾取 / `StaticWorld` / 玩法单位坐标**零改动**。`melee` example 缺 `config.toml` 或导出数据无效时**显式失败、不回落合成 demo**；`real_map_e2e.rs` 用真实导出数据断言 12513 / 97+2491 拓扑与确定性（无本机资产时跳过）。
- **决策（阶段 2，本轮）**：为避 CLI 侧无法验证 shader / GPU 黑屏（前几轮反复返工的根因），真实 BLP→PNG 贴图与小地图底图一律走**纯 CPU 可验证**路径：
  - `w3-content::textures` 用 `image`（png only）解码 6 张 512×256 地表贴图与 256×256 `war3mapMap.png` 为 `RgbaImage`；
  - `w3-render::Scene::set_terrain_tiles` 按 `surface` 层索引 + 世界 XZ 平铺把真实像素采样进地形**顶点色**（`sample_tile` / `terrain_color`，确定性纯函数，缺贴图回退 6 色调色板）；
  - `w3-client::hud::minimap_backdrop` 把小地图底图 CPU 降采样成 `grid×grid` 颜色网格，用 egui 既有 `rect_filled` 逐格绘制；
  - `ClientApp::with_terrain_tiles` / `with_minimap_image` 注入，example 串联 `real_map::assemble` 产物。
- **备选方案（Alternatives considered）**：
  - *GPU `texture_2d_array` + UV 纹理采样 shader* — **否决**。shader / GPU 管线在 CLI / headless 侧无法验证，正是此前反复黑屏 / 错位返工的根源；改动 GPU 顶点布局会牵动 `isolation.rs` 确定性 readback 契约。
  - *真实贴图 CPU 采样进顶点色 + 小地图降采样颜色网格* — **采纳**。零 shader 改动、零 GPU 黑屏风险，真实像素确实上屏，且 `cargo test --workspace` 全绿、e2e 断言顶点色较调色板路径改变且逐帧可复现。
  - *接入真实 glTF/MDX 骨骼模型* — **否决（留阶段 3）**。离线缓存无 `gltf` crate，外链贴图 / 骨骼动画解析工程量大且 CLI 侧不可验，如实登记为已知限制，不在本轮假装完成。
- **关联 ADR 0002 与跨平台确定性影响**：`RgbaImage` / 顶点色 / 颜色网格 / `StaticMarker` 全部是表现层派生数据，MUST NOT 进入权威状态、FNV-1a 状态哈希或 `SNAPSHOT_VERSION`（当前 3）快照；`scene.rs` 新增字段不参与 `Scene` 的权威哈希（Scene 本就不进哈希）；`git diff -- crates/w3-core/` 仍为空，`determinism` 26 项 / `isolation` 3 项全绿。
- **资产与许可证（ADR 0001）**：6 张贴图与底图来自玩家本机合法导出的经典客户端，路径由 gitignored `config.toml` 提供、仓库零资产、绝不入库；`image` 及 13 个传递依赖许可证逐一核对（全为 MIT / Apache-2.0 / BSD-3-Clause / Zlib / 0BSD / Unlicense 宽松许可、与 GPLv3 兼容），登记 `NOTICE.md` 新节、`Cargo.lock` 提交；`asset scan OK`。
- **架构原则与分层依赖**：SRP（真实数据装配独立 `real_map` / `textures` 模块）、OCP（`ClientApp` 用 builder 注入，既有 `playable` 入口零改动）、只读呈现边界、ADR 0004（导出 `f64` 只经表现层缩放，不进冻结边界 / 权威状态）。

## Risks / Trade-offs

- [AI 太弱导致「没有对手感」或打不完一局] → D3 命令源至少覆盖采集、训练与周期性进攻；人工验收清单逐项勾「一局可打完」；若强度仍不足，作为后续独立 change 增强，**不在本 change 扩内核**。
- [合成定义数值不合理导致闭环卡住（资源不够 / 训练过慢）] → CPU 自测用固定命令脚本跑完整一局（采集 → 造兵营 → 训兵 → 摧毁主城 → 结算）验证可达；数值全为编译期常量，便于一次性校准。
- [真实导出数据越过表现层污染权威状态] → 真实地图 / 贴图 / 底图全部只在 `real_map` / `textures` / `Scene` / `HudModel` 的表现层派生数据中流转；`w3-content` 原始 `f64` 与 PNG 像素 MUST NOT 进入 `FrozenDefinitions` / 状态哈希 / 快照（D2 / D7）；`w3-core` 零 diff、`SNAPSHOT_VERSION` 仍为 3 作硬门禁。
- [中文未上屏被误认为 HUD 不可见] → ASCII 文案 + 已启用的开源内置字体保证文字层可见；中文上屏与 CJK 字体明确留后续独立 change（CODEBUDDY.md §10 人工独占）。
- [`w3-core` 被无意改动破坏确定性] → `git diff -- crates/w3-core/` 为空、`determinism` 26 项 / `isolation` 3 项 / `cargo test --workspace` 全绿、`SNAPSHOT_VERSION` 仍为 3 作硬门禁；critic 复审逐项勾对。
- [人工 GPU 冒烟被 CI 绿灯冒充] → CI 只覆盖编译 + CPU 测试；真窗口「打完一局」只能人工在有 GPU / 显示设备桌面执行，清单未逐项勾选前不得归档。
- [新增依赖 / 许可证越界] → 阶段 2 经用户授权新增 `image`（png only）及 13 个传递依赖，`cargo metadata` 逐一核对 license 均为宽松许可、与 GPLv3 兼容，已登记 `NOTICE.md` / `Cargo.lock`；CJK 字体、真实模型、开源资产若后续引入，仍走 CODEBUDDY.md §10 人工拍板 + `NOTICE.md` 登记 + `Cargo.lock` 提交。
- [真实 glTF/MDX 骨骼模型 / 逐像素 GPU 纹理被误当成已实现] → 如实登记为**阶段 3 已知限制**：本轮静态摆放仍用程序化几何（四面体 / 长方体 / 圆锥 / 八面体），地表是 CPU 采样进顶点色（非逐像素纹理），离线无 `gltf` crate 且 CLI 侧不可验；不冒充已完成（D7）。
- [真实视觉最终上屏效果未经人工验收] → CPU 侧只验证贴图加载 / 顶点色改变 / 颜色网格非纯色 / 确定性；egui 底图与顶点色地形在真 GPU 窗口的观感、以及阶段 1/2 之前的全部人工 GPU 冒烟项，仍须用户在有显示设备桌面按 `smoke-checklist.md` 逐项执行（第 6.3 / 8 组），CI 不覆盖 GPU 链路，清单未勾前不归档。
- [参考项目代码被复制或 AGPLv3 组合] → 四个参考项目只读；`WarsmashModEngine`(AGPLv3) 仅作语义参考，未经 ADR 复核不得引入其代码。
- [命令卡 / 小地图交互被误接成权威命令] → 小地图点击只平移表现层相机；选中集合、编组、HUD 布局全部表现层专属，`isolation.rs` 与逐帧哈希比对守住。
- [窗口内「重开」按键未实现] → 本 change 验收线为「打完一局」；spec 的重开 Scenario 已与实现对齐为「以相同冻结定义与开局脚本重新构造一局」（CPU 层 `full_script_plays_one_match_to_victory_and_restart_is_clean` 验证），窗口内按键式重开留后续独立 change（`critic` residual R2）。
- [命令卡 `Move` / `Attack` 以当前光标为目标，与按钮点击位置无关；`pending_input_action` 单槽同帧仅保留最后一个动作] → 属交互可用性限制（`critic` residual R7），人工清单 8.2 重点观察；不改确定性契约，后续 change 可引入「待指定目标」态。
- [AI 脚本开局一次性入队，`pending` 含 `AI_HORIZON = 3600` 帧全部命令，超过后对手命令耗尽] → 属规模 / 性能边界与已知设计限制（`critic` residual R11），不影响确定性；`GameplayCommandQueue` 的哈希与快照序列化 `pending`，HUD 每帧 `snapshot()` 有额外开销，人工 8.4 观察高刷表现。
- [确定性回归只比对终态哈希 / 快照，批次覆盖 ≤ `MAX_CATCHUP_STEPS = 8`] → 终态完整快照相等足以排除分叉；>8 帧批次会触发宿主层显式追帧丢弃（设计保护，非不确定性），MUST NOT 据此声称任意卡顿 / 任意粒度已验证（`critic` residual R10）。
- [新增测试断言消息沿用仓库既有中文惯例] → 运行时日志 / 错误文本一律英文（CODEBUDDY.md §3），测试 `assert!` 中文消息为仓库既有先例（`critic` residual R4），不进日志 / 权威状态 / 哈希；后续可统一改为英文 `key=value`。

## Migration Plan

1. **规范侧（本轮）**：写 `openspec/changes/melee-playable-loop/` 下 proposal / specs / design / tasks 四件套；`openspec validate melee-playable-loop --strict` 通过；`python tools/check_encoding.py` 与 `git diff --check` 通过。不碰主 specs / 其它 change / `crates/w3-core/` / 根配置；实施阶段按用户拍板追加了 `crates/w3-client` / `w3-render` / `w3-content` 的表现层改动与 `NOTICE.md` / `Cargo.lock` 依赖登记（D7）。
2. **用户拍板后 `/opsx:apply`**：由对应 owner 实施（`w3-client` / `w3-render` 为主，`w3-core` 零改动），按 tasks 分组推进；写完 `cargo fmt --all`、提交前 `cargo fmt --all -- --check`。
3. **CPU 自测**：固定命令脚本跑完一局；`cargo test -p w3-core --test determinism`（26 项）、`crates/w3-render/tests/isolation.rs`（3 项）、`cargo test --workspace`、`cargo build --examples`、`python tools/verify_all.py` 全绿。
4. **本机自检（有显示设备机器）**：`cargo build -p w3-client --example melee` 与 `cargo run -p w3-client --example melee`，确认窗口打开、HUD 文字层与命令卡 / 小地图可见、英文结构化日志可观测。
5. **人工 GPU 冒烟（人工项）**：由用户在有 GPU / 显示设备桌面逐项执行清单，含「从开局到分出胜负打完一局」；无 GPU 环境明确报告不可执行、不静默跳过、不改判通过。
6. **`critic` 复审 → 本地中文 commit → 人工 push → CI 双平台门禁 → `/opsx:archive`**：归档由编排者执行；AI 不执行任何 `push` / `fetch` / `pull` / `clone`。
7. **回滚**：删除新增 example 与 HUD / 呈现扩展，恢复 `w3-client` / `w3-render` 改动；`w3-core` 零改动、依赖零改动、`SNAPSHOT_VERSION` 不变，回滚不影响权威状态或其它 change。

## Open Questions

> 以下均为**可延后**的后续范围，不影响本 change 的 specs、方案与任务分解；若纳入本期，须回到对应 artifact 修订。

- **真实数据接入权威定义**：表现层真实地图 / 贴图 / 小地图底图已在阶段 1/2 落地（D7）；是否进一步用真实 SLK 数值替换合成 `FrozenDefinitions`、引入真实单位数值，仍留后续独立 change（D2）。
- **中文上屏与 CJK 字体**：需内嵌开源 CJK 字体，涉及新增依赖与许可证（CODEBUDDY.md §10 人工独占），留后续独立 change（D4）。
- **真实模型 / 骨骼动画 / 迷雾 / 联机接入本局**：留 P7 后续阶段，不在本期验收线内（D5、D6）。
- **AI 强度与多难度**：本期保证「有对手、可打完、可结算」；更丰富的 AI 行为（编队、科技、多难度）留后续 change（D3）。
