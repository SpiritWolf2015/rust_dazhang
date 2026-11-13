# Proposal

## Why

归档的 `playable-window-smoke` 只证明了窗口能开、框选与施法有响应：窗口里只有合成碗状地形、4 个四面体与 ASCII 状态窗，没有资源、建筑、对手或胜负，用户明确判定「根本没法玩」。只读参考项目 `godot_warcraft3` 的可玩水位是「Echo Isles 人族 1v1 可结束（AI + 结算）+ 采矿/建造/训练/清野闭环 + 完整 HUD」。本 change 以用户拍板的「能打完一局」为验收线，把 `w3-core` 已交付但尚未串起来的玩法命令接成完整一局，并补上让它可操作的 HUD 与可辨识呈现。

实施期间用户进一步判定「合成色块 demo 根本不是 War3 地图画面」，要求对齐 `godot_warcraft3` 的视觉水位，遂追加真实地图表现层阶段（阶段 1：真实 Echo Isles 高度场 / 地表分区 / 水体 / 悬崖 / 97 单位 + 2491 装饰物静态摆放；阶段 2：真实 BLP→PNG 地表贴图与 `war3mapMap.png` 小地图底图）。这些全部是**纯表现层数据**，不改 `w3-core`、不进状态哈希 / 快照；真实导出资产仍留本机 gitignored 路径、不入库（ADR 0001）。**CPU 对局逻辑闭环**（合成冻结定义 + 命令脚本，由 `melee_loop.rs` 验证）零资产、可打完一局；**`melee` 可玩 example 的视觉层**依赖 gitignored 的 `config.toml`、缺配置显式失败不回落合成 demo —— `melee-match` spec 已把「零资产环境」Scenario 收敛为 CPU 装配与 `playable` 合成入口，并新增「真实视觉入口依赖本机导出」Scenario。

阶段 2 人工窗口验收后，用户再次判定「EI 地图上仍是基本几何体、未达参考项目制作水平」，遂追加**阶段 3：真实 glTF 模型**——手写 glTF 2.0 子集解析读取本机 922 个真实模型，把 2588 个静态摆放与 6 类动态单位全部替换为真实 WC3 网格 + 真实贴图，并新增 headless GPU 离屏 readback 自测：用**A/B 像素差对照**（仅地形 vs 地形+真实模型）在 CLI 断言真实模型确实改变了大量画面像素、带来多种贴图颜色（地形自身无法伪造）。真实模型 bind pose 静态网格 + 真实贴图颜色已落地；逐像素 GPU 纹理过滤、骨骼动画逐帧蒙皮留后续（见 `design.md` D8 / D9 诚实边界）。

阶段 3 首次真窗口验收后，用户判定「还是基本几何体，没看见 War3 的农民 / 建筑」——日志显示模型顶点确已装配，根因是**渲染观感**（尺度 / 默认相机俯瞰全图 / 特效层被当成实体画成白纸片黑块 / 树冠 billboard 被误删）。遂追加 D9 可视性修复：`wc3FilterMode` 特效层与 UberSplat 地面贴片过滤、保守 alpha 剔除让树成林、表现层放大（静态 2× / 动态 4×）、`ClientApp::with_initial_camera` 拉近到 RTS 局部战场视角（滚轮 / WASD 仍可看全图）。全部用离屏渲染逐版自证，最终画面城堡 / 人形农民清晰可辨、白纸片黑块消失；真窗口 HUD overlay 最终观感仍须人工验收。

D9 后用户第三次真窗口判定「还是残次品」，对照只读参考项目验收图确认差距是**架构性**的：本仓库是 wgpu 手写单管线 + **CPU 逐顶点 UV 采样进顶点色**（每三角形内是插值纯色，出不了贴图细节），天生做不出清晰地表贴图与有树冠的树。用户通过 `AskUserQuestion` 拍板「换真 GPU 纹理管线（推荐）」，遂追加**阶段 4（D10）**：`texture_2d_array` + UV + 逐像素采样 + alpha test，把统一贴图数组（模型贴图 + 6 张地表 tileset + 真实水面贴图）上传 GPU，`SceneVertex` 携带 `uv`/`tex_layer`/`tex_mode`，地形与模型全部走 GPU 逐像素平铺采样；配合双面渲染（树冠 billboard 完整）、季节贴图重定向（Summer 地图用绿树而非上游写死的雪树）、可见水裕量（滤浅滩锯齿）、sRGB framebuffer 修正（离屏自证与真窗口一致）。全部用新增 `render_probe` 离屏整图探针 + `ViewImage` + 像素量化逐版自证，**先用离屏 readback 证明画面、再请人工验收**，不反复开窗；`w3-core` 零改动、零新增依赖、真实资产仍仅留本机（ADR 0001）。残留：单管线 flat 层的水岸格点级锯齿、真窗口 HUD overlay 最终观感仍须人工验收（见 `design.md` D10 诚实边界）。

阶段 4 后用户第四次真窗口判定「模型总算加载出来了，但相机不是正常 War3 开局、农民不在主基地、找不到主基地、没法正常玩一局所以没法开始验收」。核对截图与离线自证确认：画面仍是俯瞰全图远景，玩法单位用合成坐标 `[-96,0]`（岛南浅滩）与真实 EI 出生点 sloc `(-81,46)` 脱节，且离屏探针漏 `step()` 从未渲染过动态单位。遂追加**阶段 5（D11）开局可玩性修复**：按只读参考项目 `godot_warcraft3` 的 `MeleeBootstrap.spawn_at_sloc` / `rts_camera.focus_on_position` 语义，把 `MeleeSetup` 主城 / 农民 / 金矿 / 树重定位到真实 EI 出生区的无树空地（离线量化 12 个落点全部陆地、无树冠遮挡），相机改 RTS 常规近景 `zoom=7 / pan=[-72,34]`（`MAX_ZOOM` 放宽到 20）对准主城——主城完整居中、农民在前方分开可框选、下方开阔地可操作、看不到地图边界虚空；探针修 `step()` 并同步默认开局参数，使离屏能真正自证「打开就能玩」。改的是 `w3-client` 表现层装配脚本常量与相机参数，`w3-core` 零改动、确定性契约不变、闭环测试（含打完一局）全绿。**本轮验收门槛**：用户按 `smoke-checklist.md` B9 确认「能找到基地、框选农民、正常开局」后才开始完整一局验收（见 `design.md` D11 诚实边界）。

## What Changes

- 新增 `melee-match` capability：一局 1v1 Melee 对战会话——开局布置、资源采集、建造、训练、战斗、确定性 AI 对手、胜负结算与重开。
- 新增 `rts-command-hud` capability：命令卡、选择面板、小地图、生产队列可见、中文 UI、编队与群体命令的输入翻译；全部只读派生自模拟视图。
- 新增合成/原创冻结内容定义（农民、主城、兵营、战斗单位、金矿、树木、技能、升级）与开局 `Spawn` 命令脚本；全部为编译期整数常量，零资产读取。
- 新增确定性外部 AI 命令源：按冻结帧号经 `submit_gameplay_command` 提交对手命令，复用既有 `SetAi` / `Leave` / `resolve_victory`，**`w3-core` 零改动**。
- 新增可辨识的实体呈现（按单位类型区分几何 / 队伍色 / 选中环 / 血条），履行 `3d-rendering` 既有契约；阶段 3 起静态摆放与动态单位优先用**真实 glTF/MDX 模型网格 + 真实贴图**（bind pose 静态），程序化几何降级为无本机资产 / 缺键时的回退；逐帧骨骼动画留后续。
- 把 `w3-content` 读取本机真实导出数据的接缝落地为**表现层**装配（gitignored 本机路径 / `load_map` / `textures` PNG 解码 / `gltf` 模型解析 / `model_index` 映射）：`real_map` 缩放装配真实 Echo Isles 地形与静态摆放，`Scene` 在阶段 4 起把地形与模型的真实贴图统一走 GPU `texture_2d_array` 逐像素平铺采样（CPU 顶点色仅在无本机贴图资产的 CI / 旧入口回退），HUD 小地图用真实底图 CPU 降采样颜色网格；`model_assembly` 把 2588 摆放与 6 类动态单位装配成真实 glTF 网格顶点（覆盖 2588 静态摆放 + 开局 12 个动态实体；运行时 `Train`/`Build` 新生成实体因模型缺键回退程序化几何，见 `design.md` D9 诚实边界）。路径缺失或导出数据无效时 `melee` example 显式失败、不回落合成色块 demo；CI / 无本机资产 / 模型缺键环境优雅降级到程序化几何并跳过真实资产 e2e，玩法闭环 CPU 测试不依赖任何资产。
- 新增 `render_headless_rgba` / `render_headless_rgba_with_textures` 离屏 GPU 自测与 `render_probe` 整图探针：CLI 侧用真 adapter 渲染并读回像素。`gpu_headless.rs` 验证管线本身（合成顶点流非黑屏 / 确定性），`real_map_e2e` 用 A/B 对照（仅地形 vs 地形+真实模型、真实贴图 vs 占位白纹理）断言真实模型与 GPU 逐像素采样显著改变画面像素、非黑屏、颜色丰富——破解此前「GPU 呈现只能人工看窗口」的验证盲区；`render_probe` 复用 `melee` 装配 / 相机 / 真实贴图离屏渲整图写本机 PNG（不入库）供 `ViewImage` 逐版自证。真窗口 HUD overlay 观感仍须人工验收。
- 把可玩入口装配成「能打完一局」的 example；既有 `playable-window-entry` 入口保留，不删改。
- **BREAKING：无**。不删除或改写既有公开签名，新增公开项限于新 capability 相关 API；不引入通用游戏引擎、不提交任何资产文件。阶段 2 新增 1 个直接依赖 `image`（仅 png feature，用于本机 PNG 解码）及 13 个传递依赖，许可证逐一核对并登记 `NOTICE.md`、`Cargo.lock` 已提交（用户已授权本轮新增依赖）；阶段 3 真实 glTF 模型**零新增依赖**（仅用已有 `serde_json` / `image`），手写解析器读取本机模型。

## Capabilities

### New Capabilities

- `melee-match`: 一局 1v1 Melee 对战的会话与闭环：开局布置、合成/原创冻结内容定义与真实导出数据接缝、资源采集、建造与训练、战斗、确定性 AI 对手、胜负结算与重开、对局实体可辨识呈现。
- `rts-command-hud`: RTS 命令界面：命令卡、选择面板、小地图、生产队列可见、中文 UI、编队与群体命令的输入翻译，只读派生自模拟视图，不反写权威状态。

### Modified Capabilities

（无。）刻意不产生任何既有 capability 的 MODIFIED delta：现有 `gameplay-systems`（冻结定义 / 统一结算 / AI 命令源 / 胜负）、`playable-client`（30 Hz 累加器 / 输入翻译 / 只读 HUD / 客户端状态非权威）、`3d-rendering`（骨骼动画与队伍色 / 地形 / 交互相机）、`content-import`（导出数据与零资产边界）、`unit-orders`、`playable-window-entry` 的 requirements **无行为变更**；本 change 是它们的行使与组合，不是改写。若实施中确需改写既有 requirement，须先回到对应 artifact 修订再继续，避免主 specs 争用。

## Impact

- **代码面**：`crates/w3-client/`（HUD / 输入 / 场景装配 / `real_map` / `model_assembly` / example）、`crates/w3-render/`（程序化几何回退、`model_geometry` 烘焙、真实贴图 CPU 采样、`render_headless_rgba` 离屏自测）、`crates/w3-content/`（`textures` PNG 解码 / `gltf` 模型解析 / `model_index` 映射）为主；**`crates/w3-core/` 零改动**。
- **API / 依赖**：新增公开项限于 `w3-client` / `w3-render` / `w3-content`；不删改既有公开签名。阶段 2 新增 `image`（png only）直接依赖与 13 个传递依赖（DEFLATE / 色彩管理 / 校验链），均为 MIT / Apache-2.0 / BSD-3-Clause / Zlib / 0BSD / Unlicense 宽松许可、与 GPLv3 兼容，已登记 `NOTICE.md` 新节、`Cargo.lock` 已提交；阶段 3 零新增依赖；未新增任何资产文件。
- **确定性（ADR 0002）**：`w3-core` 零改动 → `SNAPSHOT_VERSION` 仍为 3、FNV-1a 状态哈希字段集合与快照头部布局不变、不新增权威状态字段；`cargo test -p w3-core --test determinism`（26 项）、`crates/w3-render/tests/isolation.rs`（3 项）与 `cargo test --workspace` MUST 全绿；AI 命令经同一校验入口、重放不双重执行。
- **资产与法律（ADR 0001）**：仓库零资产；合成定义全为编译期整数常量；真实导出数据只经 gitignored 本机路径读取、不入库；四个参考项目只读、不复制代码（AGPLv3 组合未经 ADR 复核不得引入）。
- **阶段与范围**：P7（P0–P6 已归档，`playable-window-smoke` 于 2026-11-12 归档）。范围外：JASS/wtg 触发器 VM、自定义地图、战役、战争迷雾 / 隐身、Mod API、联机对战接入本局验收、官方战网、真实 glTF/MDX **骨骼动画逐帧蒙皮**、逐像素水 mask / 独立 shoreline shader（真实模型 bind pose 静态网格 + 真实贴图在阶段 3 落地；**逐像素 GPU 纹理管线 `texture_2d_array` 已在阶段 4 / D10 落地**，留后续的仅骨骼动画蒙皮与水岸逐像素平滑）。
- **人工边界**：真窗口 GPU 冒烟与「打完一局」的人工验收只能由人工在有 GPU / 显示设备的桌面执行；CI 只覆盖编译 + CPU 测试，MUST NOT 声称已验证 GPU 链路。
