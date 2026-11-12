# Tasks

> 按 `design.md` D1–D7 决策分组，每组自带其工作所需的测试与验证；`w3-core` 全程零改动。工具 / 命令名保持原样，任务描述一律中文。第 1–7、9 组为用户拍板「能打完一局」的闭环与 HUD；第 10 / 11 组为实施期间用户追加的真实地图表现层阶段（D7）。第 6.3 / 8 组为人工 GPU 冒烟，须用户在有显示设备桌面执行，CI 不覆盖、未勾前不归档。

## 1. 规范基线与合成内容定义

- [x] 1.1 复核四件套 `proposal.md` / `specs/melee-match/spec.md` / `specs/rts-command-hud/spec.md` / `design.md` / `tasks.md` 与 `openspec validate melee-playable-loop --strict` 通过；验证：命令输出 `is valid` 且无 WARNING
- [x] 1.2 新增合成 / 原创冻结定义模块（农民、主城、兵营、战斗单位、资源点、技能、升级，全部编译期整数常量）；验证：单元测试覆盖各 `DefId` 与 `defs_hash` 固定，且无 `f64 → SimFixed` 直转
- [x] 1.3 新增开局 `Spawn` 命令脚本（双方主城 / 农民 / 资源点，冻结整数坐标，同一玩家内序列唯一）；验证：单元测试校验命令条数、坐标冻结与双方实体归属
- [x] 1.4 定义权威冻结定义的零资产边界与真实导出数据的**表现层** provider（gitignored 本机路径、权威定义与 CPU 闭环不读取资产、缺失回落合成定义不报错；表现层 `real_map` / `textures` 按 `config.toml` 读本机资产、不入库，见 D7 / 第 10–11 组）；验证：`export_source_falls_back_without_io` 单测证明权威路径无文件 IO，`melee_loop.rs` 零资产跑完一局
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
- [x] 6.2 `cargo build -p w3-client --example melee` 与 `cargo build --examples`；验证：编译成功（阶段 2 新增 `image` 直接依赖，见第 10 / 11 组与 `NOTICE.md` 登记）
- [ ] 6.3 本机 `cargo run -p w3-client --example melee`（有显示设备机器，须 `config.toml` 指向本机真实导出）；验证：窗口打开、**真实 Echo Isles 地形 / 地表贴图 / 小地图底图可见**、HUD 文字层 / 命令卡可见、英文结构化日志可观测 —— **本轮 CLI 未执行真窗口（`MAX_FRAMES = u32::MAX` 不会自动退出），与第 8 组人工 GPU 冒烟一并留用户本机执行；编译已由 6.2 与 `cargo test --workspace` 双重覆盖**

## 7. 全量门禁与确定性复核（集成检查）

- [x] 7.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`；验证：命令退出码 0
- [x] 7.2 `python tools/check_encoding.py` 与 `git diff --check`；验证：无 BOM / CRLF / 行尾空格
- [x] 7.3 `cargo test --workspace`、`cargo test -p w3-core --test determinism`（26 项）、`isolation`（3 项）；验证：全绿，`git diff -- crates/w3-core/` 为空
- [x] 7.4 `python tools/verify_all.py`（含 `cargo build --examples` 与 CI 资产扫描）；验证：输出 `Verification OK`，无 MPQ 派生数据 / 导出资产入库
- [x] 7.5 复核 `SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合与快照头部布局不变；`Cargo.lock` / `NOTICE.md` 的 diff 为阶段 2 已授权并登记的 `image` 依赖与许可证新节（非意外）；验证：逐项 diff 核对、`asset scan OK`
- [x] 7.6 `openspec validate melee-playable-loop` 与 `openspec validate melee-playable-loop --strict`；验证：均输出 `is valid` 且 `--strict` 无 WARNING

## 8. 人工 GPU 冒烟（人工项，仅能在有 GPU 与显示设备的桌面执行）

> 本组全部为**人工项**：只能由用户在本机逐项执行。**执行者与设备登记：由用户在本机（有 GPU / 显示设备桌面）执行；`melee` example 现依赖 `config.toml` 指向的本机真实 Echo Isles 导出（阶段 1/2，D7），真实资产留本机、不入库；玩法闭环 CPU 测试仍零资产。** **运行记录：本轮 AI 侧已完成 CPU 自测——`melee_loop` 7 项端到端（含「采集→建造→训练→摧毁主城→胜负结算→重开」一局可打完，以及 `ai_commands_are_frame_frozen_not_clock_driven` 确定性回归）、`determinism` 26 项、`isolation` 3 项、`w3-render` 36 项（含真实贴图采样 3 项回归）、`w3-content` 23 项、`real_map_e2e` 4 项（本机真实加载 6 张贴图 / 底图、无 skip）、`interaction` 21 项、`cargo test --workspace` 全绿，`verify_all.py` → `Verification OK`（编码 / fmt / openspec validate / asset scan 全通过）；6.2 编译 example 通过。6.3 真窗口与 8.1–8.5 仍待用户本机逐项勾选（判据见 `smoke-checklist.md`）。** 当前 CLI 会话标注「待人工在有 GPU / 显示设备桌面执行」；CI / headless / 无显示设备环境 MUST 明确报告无法执行、不静默跳过、不改判为通过；**CI 只覆盖编译 + CPU 测试，未验证 GPU 链路**；清单未逐项勾选前不得归档。

- [ ] 8.1 人工项：窗口能开可关；真实 Echo Isles 129×97 地形（非合成浅碗）、真实 BLP→PNG 地表贴图颜色（非调色板色块）、97+2491 静态摆放贴地可见、真实 `war3mapMap.png` 小地图底图（非纯灰底），且 HUD 文字层 / 命令卡 / 选择面板 / 生产队列可见；验证：人工记录可观察证据（对应 `smoke-checklist.md` B1–B6）
- [ ] 8.2 人工项：在真实地形 / 真实贴图上框选拾取不错位；右键移动 / 攻击、采集、建造、训练、取消、编队逐项生效且失败有英文日志；验证：人工记录观察
- [ ] 8.3 人工项：从开局到分出胜负**打完一局**（含 AI 对手行为、胜负显示与重开）；验证：人工记录完整剧本与结果
- [ ] 8.4 人工项：WASD / 滚轮 / Z-X 相机、拖拽 `Resized`、高刷下逻辑帧恒定 30；验证：人工记录画面与帧号
- [ ] 8.5 人工项：汇总清单勾选状态并记录「CI 只覆盖编译 + CPU 测试，未验证 GPU 链路」；验证：逐项有通过判据，无 GPU 环境标注不可执行

## 9. `critic` 复审与本地提交（AI 侧）

- [x] 9.1 由只读 `critic` 复审确定性、所有权、许可证、编码、资产入库、`w3-core` 零 diff 与只读呈现边界；验证：复审结论无阻塞项，且明确「CI 未验证 GPU 链路、人工清单已通过或未执行」—— 复审首轮指出 1 项阻塞（AI 命令按 `advance_ticks` 墙钟调用粒度提交，破坏逐帧确定性，对应 `driver.rs` 的 `submit_due_commands`），已修为「开局一次性入队 + 逐帧 hook 提交 `Leave`」并补 `ai_commands_are_frame_frozen_not_clock_driven` 回归；复复审结论 **无阻塞项**，`w3-core` 零 diff、`SNAPSHOT_VERSION` 仍为 3；明确确认「CI 只覆盖编译 + CPU 测试、未验证 GPU 链路」「人工清单 6.3 / 8.1–8.5 未执行」「`w3-core` 零 diff、SNAPSHOT_VERSION 仍为 3」。12 项非阻塞 residual 中 R1 / R2 / R3 / R5 / R6 / R9 / R10 / R12 已修订或对齐，R4 / R7 / R8 / R11 登记为 `design.md` Risks 已知限制
- [x] 9.2 以中文描述 + Conventional Commits 英文 type 前缀本地 commit（首行 ≤ 72 字符）；验证：`git log -1` 显示中文描述，且包含本 change 全部代码与四件套改动
- [x] 9.3 确认 `git push` 等远端同步命令一律由人工执行；验证：AI 未执行任何 `push` / `fetch` / `pull` / `clone` 命令（人工独占）

## 10. 真实地图表现层（阶段 1，用户追加 / D7）

> 实施期间用户判定合成色块 demo 不是 War3 地图画面，拍板「真实地图 + 保留玩法」。全部为表现层派生数据，`w3-core` 零改动、不进哈希 / 快照；commit `4db13ab`。

- [x] 10.1 `w3-client::real_map` 用 gitignored `config.toml` 定位本机导出，`WORLD_SCALE = 2/128` 缩放装配真实 Echo Isles 129×97 高度场 / 6 地表分区 / 水体 / 崖层；验证：`real_map_e2e.rs` 断言 12513 顶点、拓扑与确定性，缺配置显式失败不回落 demo
- [x] 10.2 真实 97 单位 + 2491 装饰物摆放转表现层 `StaticMarker`，6 种地表调色板 + 程序化几何区分；验证：`static_markers_enter_gpu_vertices` 断言 2588 摆放进顶点流且可复现
- [x] 10.3 `melee` example 改装配真实地图、缺配置 / 导出无效显式失败；验证：`cargo build --examples` 通过，玩法坐标落在真实地形范围内、相机 / 拾取 / `StaticWorld` 零改动
- [x] 10.4 真实导出数据 CPU 自测；验证：`real_map_e2e.rs` 2 项（拓扑 + 静态摆放）全绿，`git diff -- crates/w3-core/` 为空

## 11. 真实视觉资产与合规（阶段 2，用户追加 / D7）

> 用户选择「继续做完阶段 2 再看」：真实 BLP→PNG 地表贴图 + 小地图底图 + 依赖合规，CPU 自测全绿后一次性请人工验收。策略为纯 CPU 可验证路径（零 shader / 零 GPU 黑屏风险）。

- [x] 11.1 `w3-content::textures` 新增 `image`（png only）解码为 `RgbaImage`，`load_rgba` / `load_tileset` 校验 6 张贴图数量与尺寸一致；验证：`w3-content` 单测（解码失败显式报错、6 文件名唯一）全绿
- [x] 11.2 `w3-render::Scene::set_terrain_tiles` 按 `surface` 层 + 世界 XZ 平铺把真实像素采样进地形顶点色，缺贴图回退调色板；验证：`w3-render` 36 项全绿，含 `terrain_color_uses_real_tile_pixels_not_palette` / `sample_tile_is_deterministic_and_oob_safe` / `build_vertices_real_tiles_survive_consume_and_are_stable`
- [x] 11.3 `w3-client::hud::minimap_backdrop` 小地图底图 CPU 降采样颜色网格 + `ClientApp::with_minimap_image` / `with_terrain_tiles` 注入；验证：HUD 单测（纯色 / 退化输入 / 确定性）全绿，egui 既有 `rect_filled` 逐格绘制
- [x] 11.4 `real_map::assemble` 装配贴图与小地图（可选、配置即加载失败显式报错），`melee` example 串联；验证：`real_map_e2e.rs` 新增 `real_tileset_feeds_scene_vertex_colors`（顶点色较调色板改变且可复现）与 `real_minimap_decodes_into_nonflat_hud_backdrop`（底图非纯色）真正加载本机 6 张 512×256 贴图与 256×256 底图、4 项全绿；修复 e2e 的 `config.toml` 定位（`CARGO_MANIFEST_DIR`，此前一直被跳过）
- [x] 11.5 新增依赖许可证核对与登记；验证：`cargo metadata` 逐一提取 13 个新包 license，全为宽松许可与 GPLv3 兼容，`NOTICE.md` 新节登记、`Cargo.lock` 提交、资产扫描 `asset scan OK`
- [x] 11.6 全量门禁；验证：`python tools/verify_all.py` → `Verification OK`（编码 / rustfmt / 全测试 / `openspec validate` / 资产扫描全绿），`crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍为 3

> **阶段 2 已知限制（如实登记，不冒充完成，留阶段 3 独立 change）**：真实 glTF/MDX 骨骼模型未接入（离线无 `gltf` crate、CLI 侧不可验），静态摆放仍为程序化几何；地表为 CPU 采样进顶点色而非逐像素 GPU 纹理；egui 底图与顶点色地形在真 GPU 窗口的最终观感须人工验收（第 6.3 / 8 组，`smoke-checklist.md`）。

## Workflow follow-up

- 人工 `git push` → CI 双平台 `python tools/verify_all.py` 绿灯。
- 由编排者执行 `/opsx:archive`（`openspec archive melee-playable-loop -y`），确认主 `openspec/specs/` 新增 `melee-match` 与 `rts-command-hud`，既有 12 个 capability 未被改写。
- 后续独立 change：真实 SLK 数值进权威 `FrozenDefinitions`（表现层真实地图 / 贴图已在本 change 阶段 1/2 落地）、逐像素 GPU 纹理采样、真实 glTF/MDX 模型与骨骼动画、中文上屏（CJK 字体与许可证人工拍板）、AI 强度与多难度、联机接入本局。
