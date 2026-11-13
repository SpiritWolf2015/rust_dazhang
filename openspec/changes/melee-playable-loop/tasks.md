# Tasks

> 按 `design.md` D1–D9 决策分组，每组自带其工作所需的测试与验证；`w3-core` 全程零改动。工具 / 命令名保持原样，任务描述一律中文。第 1–7、9 组为用户拍板「能打完一局」的闭环与 HUD；第 10 / 11 组为实施期间用户追加的真实地图表现层阶段 1/2（D7）；第 12 组为用户再次判定「EI 地图上仍是基本几何体」后追加的真实 glTF 模型阶段 3（D8）；第 13 组为阶段 3 真窗口验收后「装配了却看不见」的可视性修复（D9）。第 6.3 / 8 组为人工 GPU 冒烟，须用户在有显示设备桌面执行，CI 不覆盖、未勾前不归档。

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

> 本组全部为**人工项**：只能由用户在本机逐项执行。**执行者与设备登记：由用户在本机（有 GPU / 显示设备桌面）执行；`melee` example 现依赖 `config.toml` 指向的本机真实 Echo Isles 导出 + 922 个真实 glTF 模型（阶段 1/2/3，D7/D8），真实资产留本机、不入库；玩法闭环 CPU 测试仍零资产。** **运行记录：本轮 AI 侧已完成 CPU + 离屏 GPU 自测——`melee_loop` 7 项端到端（含「采集→建造→训练→摧毁主城→胜负结算→重开」一局可打完，以及 `ai_commands_are_frame_frozen_not_clock_driven` 确定性回归）、`determinism` 26 项、`isolation` 3 项、`w3-render` 49 项（含 GeosetFilter / 特效过滤 / 树冠保守剔除）、`gpu_headless` 3 项（真 adapter 离屏渲染非黑屏、逐字节确定）、`w3-content` 39 项（gltf 解析 / filterMode 透传 / model_index 映射）、`real_map_e2e` 8 项（本机真实加载贴图 / 底图 / 2588 真实 glTF 模型、特效层过滤、无 skip、真实模型离屏渲染非黑屏）、`interaction` 22 项（含 with_initial_camera 表现层专属）、`cargo test --workspace` 全绿，`verify_all.py` → `Verification OK`（编码 / fmt / openspec validate / asset scan 全通过）；6.2 编译 example 通过。6.3 真窗口与 8.1–8.5 仍待用户本机逐项勾选（判据见 `smoke-checklist.md`）。** 当前 CLI 会话标注「待人工在有 GPU / 显示设备桌面执行」；CI / headless / 无显示设备环境 MUST 明确报告无法执行、不静默跳过、不改判为通过；**CI 只覆盖编译 + CPU 测试 + 本机 headless 离屏 3D 场景，未验证真窗口 GPU 链路与 HUD overlay**；清单未逐项勾选前不得归档。

- [ ] 8.1 人工项：窗口能开可关；真实 Echo Isles 129×97 地形（非合成浅碗）、真实 BLP→PNG 地表贴图颜色（非调色板色块）、**2588 个静态摆放与开局 12 个动态实体呈现为真实 glTF WC3 模型（树 / 岩石 / 建筑 / 农民，非四面体 / 圆锥 / 长方体等基本几何体；运行时 Train/Build 新实体走模型缺键回退、属已知限制）**、真实 `war3mapMap.png` 小地图底图（非纯灰底），且 HUD 文字层 / 命令卡 / 选择面板 / 生产队列可见；验证：人工记录可观察证据（对应 `smoke-checklist.md` B1–B7）
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

> **阶段 2 收尾说明**：当时登记「真实 glTF 模型留阶段 3 独立 change」。用户随后在真窗口判定「EI 地图上仍是基本几何体、未达参考项目制作水平」，故**阶段 3 在同一 change 内追加并完成**（见第 12 组 / `design.md` D8）——2588 摆放 + 6 类动态单位全部替换为真实 glTF 模型。仍成立的已知限制：逐像素 GPU 纹理过滤、骨骼动画逐帧蒙皮、真 GPU 窗口 HUD overlay 观感须人工验收（第 6.3 / 8 组）。

## 12. 真实 glTF 模型与 headless GPU 自测（阶段 3，用户追加 / D8）

> 用户判定「EI 地图上使用基本几何体，不是达到参考项目的制作水平」，要求全量替换。根因：`w3-render::Model` 只有骨骼 / 动画 / 材质元数据、无网格顶点；922 个真实 glTF 一直在本机未被读取。策略：手写 glTF 2.0 子集解析（零新依赖）+ CPU 烘焙进顶点色（零 shader / 零 GPU 黑屏风险）+ headless 离屏 readback 自测（破解「GPU 只能人工看窗口」盲区）。

- [x] 12.1 `w3-content::gltf` 手写 glTF 2.0 子集解析（`.gltf` JSON + `.bin` 交错顶点 `POSITION`/`NORMAL`/`TEXCOORD_0` + 索引 + 相对路径 PNG 贴图 → `ModelMesh`），零新依赖（仅用已有 `serde_json`/`image`）；验证：`gltf.rs` 单测（交错 stride、u16/u32 索引、材质 alpha/doubleSided、越界 / 短 buffer / 坏 JSON 显式报错、确定性）全绿；真实 Corn / 树 / Footman / 兵营 glTF 探针确认 136/53/372/1671 三角形、贴图与 `node_scale=0.01` 正确解析
- [x] 12.2 `w3-content::model_index` 的 `ModelCatalog` 扫描 922 个 glTF + SLK `file` 字段建立 `typeId`/`doodadID`→模型映射（大小写不敏感、变体前缀、树 `LTlt` 等 `file` 空者按类别兜底、候选列表跳过上游空占位）；验证：`model_index.rs` 单测（合成目录 / 变体 / 树兜底 / 确定性）全绿；真实目录探针 922 索引、单位 25/26（`sloc` 无模型）、装饰物 21/21、6 个实体键全命中
- [x] 12.3 `w3-render::model_geometry` 烘焙：逐顶点 UV 采样真实贴图像素写入顶点色、树叶 `MASK` 镂空剔除（**注：D9 起改为「三顶点 UV 全透明才删」保守剔除，早期「重心单点」剔除会误删树冠，见 13.3**）、`wc3ToGltfVec3` 坐标展开成 `SceneVertex`（布局 / shader / 单管线零改动）；验证：`model_geometry.rs` 单测（贴图采样、alpha 剔除、坐标翻转、退化 UV、展开确定性）全绿
- [x] 12.4 `Scene` 增加 `set_static_model_vertices` / `set_entity_models`，静态摆放与动态单位优先真实网格、缺键回退程序化几何；验证：`scene.rs` 新增 3 项单测（静态替换、动态替换、缺键回退）全绿，`w3-render` 45 项全绿
- [x] 12.5 `w3-client::model_assembly` 按摆放位姿（缩放 / 贴地 / `angle`）预展开静态模型、6 类动态实体烘焙、glTF 解码缓存、顶点预算保护；`config.toml` 加 `models_dir`/`slk_dir`，`real_map::assemble` / `ClientApp` builder / `melee` example 串联；验证：真实装配探针 **2588/2588 摆放命中、0 skip、37.4 万真实模型顶点、3970 种真实贴图颜色、6 实体模型（主城 3212 / 兵营 1667 / 步兵 369 三角形）**
- [x] 12.6 `w3-render::render_headless_rgba` 离屏 GPU 渲染 + readback（真 adapter、无窗口、复用 `build_pipeline`/`record_scene`）；验证：`gpu_headless.rs` 3 项（合成顶点流验管线非黑屏 / 逐字节确定 / 空流纯背景，明确不声称证明真实模型上屏）全绿
- [x] 12.7 真实资产 e2e：`real_gltf_models_replace_all_programmatic_geometry`（2588 全命中、顶点数远超程序化、真实贴图颜色丰富、6 实体模型）、`real_models_enter_scene_vertices_and_are_deterministic`（程序化被真实网格整体替换、逐帧可复现）、`real_gltf_models_render_on_screen_not_black`（**A/B 像素差对照**：仅地形 vs 地形+真实模型两帧，断言真实模型显著改变大量画面像素 + 变化像素含多种模型贴图颜色 + 全场景非黑屏，地形自身无法伪造）；验证：`real_map_e2e.rs` 8 项全绿、无 skip（含第 13 组新增特效过滤测试）
- [x] 12.8 全量门禁与合规；验证：`python tools/verify_all.py` → `Verification OK`（编码 / rustfmt / 全 workspace 测试 / `openspec validate` / 资产扫描全绿），`crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍为 3、**零新增 Cargo 依赖**、`NOTICE.md`/`Cargo.lock` 无新变化

> **阶段 3 诚实边界（如实登记，不冒充完成）**：真实模型为 bind pose 静态网格（逐帧骨骼动画蒙皮留后续）；贴图为逐顶点 UV 采样进顶点色（逐像素 GPU 纹理过滤留后续，稀疏网格细节有限）；少数非 EI convert 子集装饰物用同类真实模型语义兜底；真 GPU 窗口 HUD overlay 叠加观感仍须人工验收（第 6.3 / 8 组，`smoke-checklist.md`）；headless 自测已用 A/B 像素差对照证明真实模型在离屏 3D 场景贡献画面，但不覆盖 HUD overlay 与真窗口最终观感。

## 13. 真实模型可视性修复（阶段 3 真窗口验收后，用户追加 / D9）

> 用户真窗口判定「还是基本几何体，没看见 War3 农民 / 建筑」。日志证明模型顶点已装配（`static_vertices=373773`、`entity_models=6`），正视特写确认模型完整——问题在渲染观感。全部用 `render_headless_rgba` 离屏渲染 + `ViewImage` 离线逐版自证，不反复开窗打扰用户。

- [x] 13.1 `w3-content::gltf` 透传 `extras.wc3FilterMode` / `alphaMode` 到 `SubMesh.filter_mode`（`GeosetFilter` 枚举：Opaque/Mask/Blend/Additive/AddAlpha/Modulate/Modulate2x/GroundDecal），按贴图 URI 的「`Splats/` 目录或 `ubersplat` 文件名」路径级特征识别 `GroundDecal`（收紧后用 `CastleShadow.png`/`Shockwave1.png` 负向单测防误判）；验证：`gltf.rs` 单测（7 种 filterMode 分类、GroundDecal URI 正负向识别、实体/特效判定）全绿
- [x] 13.2 `w3-render::model_geometry::bake_submesh` 只烘焙 `is_entity()` geoset（Opaque/Mask），过滤特效层与地面贴片（消白色纸片 / 大黑三角）；验证：单测 `effect_geosets_are_filtered_out`（5 种特效全剔除、只留 1 实体）、`mask_entity_geoset_is_kept` 全绿；真实主城特效层正视特写对比，黑块 / 白纸片消失
- [x] 13.3 alpha 镂空改**保守剔除**（仅三顶点 UV 全透明才删整片），保留树冠 billboard 面片；验证：单测 `bake_keeps_tree_canopy_when_centroid_is_transparent_but_a_vertex_is_opaque`（UV 构造让旧重心剔除必删、新保守剔除必留，含元断言防退化）+ `bake_still_drops_triangle_with_all_vertices_transparent`（防退化成永不删）全绿，离屏渲染树从细杆变为有树干 / 树冠的林
- [x] 13.4 表现层显示放大 `MODEL_DISPLAY_SCALE=2`（静态）/ `ENTITY_DISPLAY_SCALE=2`（动态额外，合计 4×），静态 / 动态两处装配统一应用；验证：离线离屏渲染对比 2/3/4/6 倍 + 多 zoom 组合，选定可辨城堡 / 人形农民且不过大糊屏
- [x] 13.5 `ClientApp::with_initial_camera(zoom,pan)` builder（经 `CameraState::set_initial_view` 显式赋值 + clamp、幂等，纯表现层）+ `MAX_ZOOM` 4→8（全局 clamp 上限放宽，`default()` 未改、既有入口默认位姿不变）；`melee` example 用 `zoom=3.5`/`pan=[-92,0]` 对准本地出生点的 RTS 局部视角；验证：`interaction.rs::with_initial_camera_is_presentation_only`（MVP 改变、不改哈希 / 快照、不推进 tick）全绿
- [x] 13.6 最终 example 参数离屏渲染自证（`FINAL2`）+ 真实资产 e2e；验证：城堡 / 农民可辨、白纸片黑块消失、树成林；新增 `real_map_e2e::real_townhall_filters_effects_and_keeps_entity_geometry`（真实主城特效层被过滤、实体几何保留），`real_map_e2e.rs` 8 项全绿无 skip

> **D9 诚实边界（如实登记）**：树 billboard 平面在斜俯视角下仍偏细长（WC3 原版靠面向相机 billboard 补齐，静态 bind pose 未做）；个别装饰物残留细长深色伪影（面积量化后否决高风险「面积过滤」，避免误删建筑大墙面）；出生点临水为 EI 真实水域地形特征；真窗口 HUD overlay 叠加观感仍须人工验收（第 6.3 / 8 组）。

## 14. 真 GPU 纹理管线（阶段 4，第三次真窗口验收后用户拍板 / D10）

> 用户第三次真窗口判定「还是残次品」，对照参考项目验收图确认差距是架构性的（CPU 逐顶点采样进顶点色，出不来清晰贴图 / 树冠），通过 `AskUserQuestion` 拍板「换真 GPU 纹理管线（推荐）」，要求先离屏 readback 自证、再请人工验收。全部用 `render_probe` 离屏渲染 + `ViewImage` + 像素量化逐版自证，不反复开窗打扰用户。`w3-core` 零改动、零新增 Cargo 依赖。

- [x] 14.1 `w3-content::texture_array`：`TextureArrayBuilder`（URI 归一化去重、最近邻 resize 到统一 256² 层、层上限 512）、`TextureArray::push_image`（模型贴图数组末尾追加地表 / 水贴图层，层索引不冲突）；验证：单测 8 项（resize / 退化输入 / URI 去重 / 拼接顺序 / 确定性 / 层上限 / 追加层 / 空数组合法）全绿
- [x] 14.2 `w3-render` 顶点与 shader 真 GPU 采样：`SceneVertex` 扩展 `uv`/`tex_layer`/`tex_mode`/`surface`；`scene.wgsl` 用 `texture_2d_array` + `textureSampleLevel` + `discard` alpha test；`build_pipeline` 上传真实贴图 / 纯白占位、`Repeat` sampler；`GpuContext::new_with_textures` / `render_headless_rgba_with_textures`；验证：w3-render 单测 49 + GPU headless 3 + isolation 3 全绿
- [x] 14.3 装配层收集贴图并给烘焙三角形写 UV / 层：`ModelCache` 用 `TextureArrayBuilder` 按子网格 URI 分配层、`bake_model_with_layers` 写 GPU 纹理顶点，`RealMap.textures` 注入 `ClientApp::with_textures`；验证：`cargo test -p w3-client` 全绿、`real_map_e2e` 无 skip
- [x] 14.4 地形纳入 GPU 平铺：`set_terrain_texture_layers` 注入 6 tileset + 1 水面贴图层，`build_terrain_vertex` 输出 `textured` 顶点按世界 XZ 逐像素平铺、水体走水波纹贴图；无层索引回退 CPU 顶点色（既有测试零改动）；验证：`scene.rs` 新增 5 项 CPU 回归（可见水裕量、水陆分流层 + UV 平铺、无 layers 回退、层缺失回退、确定性）全绿
- [x] 14.5 可见水裕量 `WATER_VISIBLE_MARGIN`：离线量化海床 / 浅滩高度间隙（缩放后 1.70~2.48 vs 0~1.31）定阈值 1.6，CPU 回退与 GPU 路径共用 `is_water_vertex`；验证：裕量边界单测 + 离屏整图中央区水岸锯齿显著减少
- [x] 14.6 可视性根因修复：双面渲染 `cull_mode: None`（树冠 billboard 不再被剔成细杆）；季节贴图重定向 `resolve_seasonal_texture` + `load_gltf_seasonal`（Summer 地图用绿树而非上游写死的雪树）；离屏 sRGB framebuffer 修正（`Rgba8UnormSrgb` 与真窗口一致）；验证：`gltf.rs` 季节重定向 3 项单测全绿，离屏图树冠立体 / 绿色 / 亮度正常
- [x] 14.7 离屏整图探针与自证：新增 `crates/w3-client/examples/render_probe.rs`（复用 `melee` 装配 / 相机 / 真实贴图，写 PNG 到本机 `target/` 不入库，支持 zoom / pan / 诊断染色）；逐版 `ViewImage` + 像素量化定位「深灰锯齿」为水层顶点着色而非几何洞（全地形染纯绿后中央区背景色 7.48%→0.01%），否决独立半透明水面网格（同视角锯齿无改善、多 6 万顶点）后回退
- [x] 14.8 真实资产 e2e 升级到真 GPU 管线：`real_gltf_models_render_on_screen_not_black` 改传 `Some(&map.textures)` + 注入 layers；新增 `gpu_texture_sampling_changes_pixels_vs_placeholder`（真实贴图 vs 占位白纹理 A/B 像素差，证明 GPU 逐像素采样生效）；验证：`real_map_e2e` 9 项全绿无 skip
- [x] 14.9 全量门禁与合规；验证：`cargo test --workspace` 全绿、`cargo fmt --all -- --check` 通过、`python tools/verify_all.py` → `Verification OK`（编码 / rustfmt / 测试 / openspec validate / 资产扫描）、`python tools/check_encoding.py` 通过、`crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍为 3、**零新增 Cargo 依赖**、`config.toml.example` 同步新增 `water_texture` 键、`w3-render::lib` 导出 `TerrainTextureLayers`
- [ ] 14.10 人工 GPU 冒烟（人工项）：用户在有 GPU / 显示设备桌面 `cargo run -p w3-client --example melee`，按 `smoke-checklist.md` 逐项验收真窗口观感（清晰地表贴图 / 立体绿树 / 波纹水面 / 可辨建筑与单位 / 完整 HUD / 打完一局）；离屏自证不覆盖真窗口 egui overlay，清单未勾前不归档
- [ ] 14.11 `critic` 复审 → 本地中文 commit；`git push` / CI 双平台门禁 / `/opsx:archive` 由人工或编排者执行（AI 不执行任何远端命令）

> **阶段 4 诚实边界（如实登记，不冒充完成）**：单管线 `@interpolate(flat)` 使跨水陆三角形取 provoking 顶点层，水岸呈**格点级锯齿**（诊断确认非几何洞；参考项目用逐像素水 mask / 独立 shoreline shader 平滑，本仓库未做），可见水裕量已显著减少但不冒充完全消除；`cull_mode: None` 双面渲染属正确性优先的性能权衡；逐帧骨骼动画蒙皮（bind pose 静态）、运行时新实体模型缺键回退（D9）、中文上屏、真窗口 HUD overlay 最终观感仍留后续 / 人工验收。离屏 3D 链路画面已达标并自证，但**不覆盖真窗口最终观感**（第 6.3 / 8 组人工项未勾前不归档）。

## 15. 开局可玩性修复（阶段 5，第四次真窗口验收后 / D11）

> 用户第四次真窗口判定「模型加载出来了但相机不是正常 War3 开局、农民不在主基地、找不到主基地、没法正常玩一局、没法开始验收」。按强反馈先离线查根因（相机语义 + 两套坐标系 + 参考项目开局 + 探针自证漏洞）、批量离屏自证、写足 CPU 回归，全绿后再请人工验收。`w3-core` 零改动、零新增依赖。

- [x] 15.1 根因查实：`Camera::matrix` 的 `zoom=3.5` 视野约 201×113 世界单位（俯瞰全图）、`pan=[-92,0]` 看向 `(-91,1)` 为岛南浅滩；`MeleeSetup` 合成坐标 `[-96,0]` 与真实 EI sloc `(-81,46)` 脱节；真实地图静态单位无主城/农民（主城农民靠 12 个动态实体生成，对齐参考项目 `spawn_at_sloc`）；`render_probe` 漏 `step()` 致 `dynamic_vertices=0`、此前离屏图从未含动态单位。只读核实参考项目 `MeleeBootstrap.spawn_at_sloc` / `focus_on_position` / `camera_initial_distance=16.5` / `pitch=-56°`
- [x] 15.2 `MeleeSetup` 开局重定位：命名常量 `LOCAL_START=(-74,38)`（sloc 起始区天然无树空地，距 sloc 11、半径 14 内 0 棵树）/ `AI_START=(74,48)`；`opening_layout(start,facing,spread)` 农民簇朝地图中心排在主城前方、金矿/树在背离侧，`LOCAL_SPREAD=10` / `AI_SPREAD=4`；离线量化两侧 12 落点全部 `ground>water`、无树冠遮挡、不压地图边界；验证：`melee_loop` 7 项（含打完一局 / 重放确定性）全绿
- [x] 15.3 相机改 RTS 开局近景：`MAX_ZOOM` 8→20；`melee` 默认 `zoom=7 / pan=[-72,34]`（主城完整居中、农民分开可框选、下方开阔、无全图虚空）；`render_probe` 默认参数同步 + `sim.step()` 修动态单位上屏 + `units` 诊断模式；验证：批量 zoom/pan + `ViewImage` 逐版自证主城/农民/金矿可辨、正常近景
- [x] 15.4 针对性 CPU 回归：`setup.rs` 3 项（主城对齐真实起始空地 / 农民簇拉开在前方 / 两侧布局对称确定性）、`camera.rs` 1 项（开局 zoom 在新上限内不被 clamp、越界仍 clamp）全绿；`cargo test --workspace` 全绿
- [x] 15.5 文档同步：`design.md` D11、`proposal.md` 阶段 5、`smoke-checklist.md` B 组口径 + 新增 B9 开局可玩性门槛项（本轮核心验收项）、本组任务记录
- [ ] 15.6 人工 GPU 冒烟（人工项，**门槛**）：用户 `cargo run -p w3-client --example melee`，先按 `smoke-checklist.md` **B9** 逐项确认「打开就能找到主基地、框选农民、正常开局」，通过后再按 C/D/E 组玩完一局；离屏自证不覆盖真窗口 egui overlay，B9 未通过即反馈、不勉强往下勾；清单未勾前不归档
- [ ] 15.7 `critic` 复审 → 本地中文 commit；`git push` / CI 双平台 / `/opsx:archive` 由人工或编排者执行（AI 不执行任何远端命令）

> **阶段 5 诚实边界（如实登记）**：默认开局参数已用真窗口同参 `render_probe` 离屏逐版自证（主城完整居中、农民分开、金矿/树林/水面环绕、无全图虚空），但**不覆盖真窗口 egui overlay / HUD 与最终观感**；真窗口首帧 `step` 前极短空窗理论存在（稳定运行后立即可见，用户截图 HUD 已能选中 Peasant #2 佐证）。AI 侧陆块窄且多树、开局不可见（相距约 155），不影响本地可玩性（胜负靠命令脚本 / 实体 ID 结算）。`render_probe` 输出 PNG 留 gitignored `target/`、不入库。水岸格点级锯齿（D10）、树 billboard 斜俯视偏细、运行时新实体模型缺键回退（D9）、中文上屏仍留后续 / 人工验收。

## Workflow follow-up

- 人工 `git push` → CI 双平台 `python tools/verify_all.py` 绿灯。
- 由编排者执行 `/opsx:archive`（`openspec archive melee-playable-loop -y`），确认主 `openspec/specs/` 新增 `melee-match` 与 `rts-command-hud`，既有 12 个 capability 未被改写。
- 后续独立 change：真实 SLK 数值进权威 `FrozenDefinitions`、逐像素 GPU 纹理采样、真实 glTF 骨骼动画逐帧蒙皮、中文上屏（CJK 字体与许可证人工拍板）、AI 强度与多难度、联机接入本局（表现层真实地图 / 贴图 / 小地图底图已在阶段 1/2、真实 glTF 模型 bind pose 已在阶段 3 落地、可视性修复已在 D9 落地）。
