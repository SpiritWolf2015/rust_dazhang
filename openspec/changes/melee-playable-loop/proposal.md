# Proposal

## Why

归档的 `playable-window-smoke` 只证明了窗口能开、框选与施法有响应：窗口里只有合成碗状地形、4 个四面体与 ASCII 状态窗，没有资源、建筑、对手或胜负，用户明确判定「根本没法玩」。只读参考项目 `godot_warcraft3` 的可玩水位是「Echo Isles 人族 1v1 可结束（AI + 结算）+ 采矿/建造/训练/清野闭环 + 完整 HUD」。本 change 以用户拍板的「能打完一局」为验收线，把 `w3-core` 已交付但尚未串起来的玩法命令接成完整一局，并补上让它可操作的 HUD 与可辨识呈现。

实施期间用户进一步判定「合成色块 demo 根本不是 War3 地图画面」，要求对齐 `godot_warcraft3` 的视觉水位，遂追加真实地图表现层阶段（阶段 1：真实 Echo Isles 高度场 / 地表分区 / 水体 / 悬崖 / 97 单位 + 2491 装饰物静态摆放；阶段 2：真实 BLP→PNG 地表贴图与 `war3mapMap.png` 小地图底图）。这些全部是**纯表现层数据**，不改 `w3-core`、不进状态哈希 / 快照；真实导出资产仍留本机 gitignored 路径、不入库（ADR 0001）。**CPU 对局逻辑闭环**（合成冻结定义 + 命令脚本，由 `melee_loop.rs` 验证）零资产、可打完一局；**`melee` 可玩 example 的视觉层**依赖 gitignored 的 `config.toml`、缺配置显式失败不回落合成 demo —— `melee-match` spec 已把「零资产环境」Scenario 收敛为 CPU 装配与 `playable` 合成入口，并新增「真实视觉入口依赖本机导出」Scenario。真实 glTF/MDX 骨骼模型与逐像素 GPU 纹理留后续阶段（见 `design.md` 已知限制）。

## What Changes

- 新增 `melee-match` capability：一局 1v1 Melee 对战会话——开局布置、资源采集、建造、训练、战斗、确定性 AI 对手、胜负结算与重开。
- 新增 `rts-command-hud` capability：命令卡、选择面板、小地图、生产队列可见、中文 UI、编队与群体命令的输入翻译；全部只读派生自模拟视图。
- 新增合成/原创冻结内容定义（农民、主城、兵营、战斗单位、金矿、树木、技能、升级）与开局 `Spawn` 命令脚本；全部为编译期整数常量，零资产读取。
- 新增确定性外部 AI 命令源：按冻结帧号经 `submit_gameplay_command` 提交对手命令，复用既有 `SetAi` / `Leave` / `resolve_victory`，**`w3-core` 零改动**。
- 新增可辨识的程序化实体呈现（按单位类型区分几何 / 队伍色 / 选中环 / 血条），履行 `3d-rendering` 既有的「Skeletal animation and team color」与地形要求；真实 glTF/MDX 模型留后续。
- 把 `w3-content` 读取本机真实导出数据的接缝落地为**表现层**装配（gitignored 本机路径 / `load_map` / `textures` PNG 解码）：`real_map` 缩放装配真实 Echo Isles 地形与静态摆放，`Scene` 把真实贴图 CPU 采样进地形顶点色，HUD 小地图用真实底图 CPU 降采样颜色网格。路径缺失或导出数据无效时 `melee` example 显式失败、不回落合成色块 demo；CI / 无本机资产环境自动跳过真实资产 e2e，玩法闭环 CPU 测试不依赖任何资产。
- 把可玩入口装配成「能打完一局」的 example；既有 `playable-window-entry` 入口保留，不删改。
- **BREAKING：无**。不删除或改写既有公开签名，新增公开项限于新 capability 相关 API；不引入通用游戏引擎、不提交任何资产文件。阶段 2 新增 1 个直接依赖 `image`（仅 png feature，用于本机 PNG 解码）及 13 个传递依赖，许可证逐一核对并登记 `NOTICE.md`、`Cargo.lock` 已提交（用户已授权本轮新增依赖）。

## Capabilities

### New Capabilities

- `melee-match`: 一局 1v1 Melee 对战的会话与闭环：开局布置、合成/原创冻结内容定义与真实导出数据接缝、资源采集、建造与训练、战斗、确定性 AI 对手、胜负结算与重开、对局实体可辨识呈现。
- `rts-command-hud`: RTS 命令界面：命令卡、选择面板、小地图、生产队列可见、中文 UI、编队与群体命令的输入翻译，只读派生自模拟视图，不反写权威状态。

### Modified Capabilities

（无。）刻意不产生任何既有 capability 的 MODIFIED delta：现有 `gameplay-systems`（冻结定义 / 统一结算 / AI 命令源 / 胜负）、`playable-client`（30 Hz 累加器 / 输入翻译 / 只读 HUD / 客户端状态非权威）、`3d-rendering`（骨骼动画与队伍色 / 地形 / 交互相机）、`content-import`（导出数据与零资产边界）、`unit-orders`、`playable-window-entry` 的 requirements **无行为变更**；本 change 是它们的行使与组合，不是改写。若实施中确需改写既有 requirement，须先回到对应 artifact 修订再继续，避免主 specs 争用。

## Impact

- **代码面**：`crates/w3-client/`（HUD / 输入 / 场景装配 / `real_map` / example）、`crates/w3-render/`（程序化实体几何、静态摆放与真实贴图 CPU 采样）、`crates/w3-content/`（`textures` PNG 解码）为主；**`crates/w3-core/` 零改动**。
- **API / 依赖**：新增公开项限于 `w3-client` / `w3-render` / `w3-content`；不删改既有公开签名。阶段 2 新增 `image`（png only）直接依赖与 13 个传递依赖（DEFLATE / 色彩管理 / 校验链），均为 MIT / Apache-2.0 / BSD-3-Clause / Zlib / 0BSD / Unlicense 宽松许可、与 GPLv3 兼容，已登记 `NOTICE.md` 新节、`Cargo.lock` 已提交；未新增任何资产文件。
- **确定性（ADR 0002）**：`w3-core` 零改动 → `SNAPSHOT_VERSION` 仍为 3、FNV-1a 状态哈希字段集合与快照头部布局不变、不新增权威状态字段；`cargo test -p w3-core --test determinism`（26 项）、`crates/w3-render/tests/isolation.rs`（3 项）与 `cargo test --workspace` MUST 全绿；AI 命令经同一校验入口、重放不双重执行。
- **资产与法律（ADR 0001）**：仓库零资产；合成定义全为编译期整数常量；真实导出数据只经 gitignored 本机路径读取、不入库；四个参考项目只读、不复制代码（AGPLv3 组合未经 ADR 复核不得引入）。
- **阶段与范围**：P7（P0–P6 已归档，`playable-window-smoke` 于 2026-11-12 归档）。范围外：JASS/wtg 触发器 VM、自定义地图、战役、战争迷雾 / 隐身、Mod API、联机对战接入本局验收、官方战网、真实 glTF/MDX 模型与骨骼动画（留后续独立 change）。
- **人工边界**：真窗口 GPU 冒烟与「打完一局」的人工验收只能由人工在有 GPU / 显示设备的桌面执行；CI 只覆盖编译 + CPU 测试，MUST NOT 声称已验证 GPU 链路。
