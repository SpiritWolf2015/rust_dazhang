# 桌面冒烟验收清单 — `melee-playable-loop`

> 关联 change：`openspec/changes/melee-playable-loop/`（对应 `tasks.md` 的 6.3 与 8.1–8.5）。
> 本清单全部为**人工项**：只能人工在有 GPU / 显示设备的桌面执行；CI 只覆盖编译 + CPU 测试，MUST NOT 声称已验证 GPU 链路。
> 截图 / 日志等冒烟产物 MUST NOT 提交进仓库，SHALL 落在 gitignored 的本机路径（ADR 0001、CODEBUDDY.md §6）。
> 勾选 `- [x]` 仅当该项**实际观察通过**；无证据项如实标注未执行，不冒充通过。

## 验收前准备

- 入口：`cargo run -p w3-client --example melee`
- 窗口：`MAX_FRAMES = u32::MAX`，不会自动退出，验收完手动关闭窗口
- 环境：有 GPU / 显示设备的桌面；预期 1920×1080 逻辑尺寸，本机物理 2880×1620 / `scale_factor=1.5`
- 数据（阶段 1/2/3 真实地图 + 真实模型）：`melee` 入口读取 gitignored 本机 `config.toml` 指向的 Echo Isles 导出——`terrain-heightfield.json` / 6 张 BLP→PNG 地表贴图 / `war3mapMap.png` 小地图底图 / **922 个真实 glTF 模型（`.bin` + 贴图）与 SLK 映射表**；**资产仅留本机、绝不入库**（ADR 0001）。缺 `config.toml` 或导出无效时 example 显式失败、不回落合成 demo（启动日志 `real map unavailable` + panic）。玩法定义与 CPU 测试仍零资产
- 失败定位：贴回 `window created: ... scale_factor=...`、`real map loaded: ...` 与 `input action rejected: ...` 日志

## A. 启动与日志（对应 6.3 / 8.1）

- [ ] **A1** 执行入口命令 → 弹出 `w3-client` 窗口，进程进入事件循环不退出
- [ ] **A2** 启动日志出现 `INFO target=w3_client window created: width=... height=... scale_factor=...`，宽高与本机一致
- [ ] **A3** 无 `render failed` / `gpu init failed` / `window creation failed`；无 `hud font unavailable`（字体已启用，文字层不应降级）
- [ ] **A4** 以下为**已知无害**告警，不算失败：`wgpu_hal::vulkan::instance` 的 layer manifest 提示、`egui_wgpu` 的 `Bgra8UnormSrgb` 偏好提示、启动瞬间 `catch-up limit exceeded: cap=8 dropped_ticks=...`（GPU 初始化耗时的追帧保护，纯表现层，不改权威状态）
- [ ] **A5** 关闭窗口可正常退出，无 panic / 卡死

## B. 真实地图画面与 HUD（对应 8.1）

> 阶段 1/2 已接入真实 Echo Isles 视觉，本组判据由「合成浅碗 + 12 四面体」同步为真实地形 / 贴图 / 摆放 / 底图。

- [ ] **B1** 可见真实 Echo Isles 地形：129×97 高度场（地表分区、水体、崖层可辨），**非合成浅碗**；6 种地表呈现真实 BLP→PNG 贴图颜色（CPU 采样进顶点色上屏），**非 6 色调色板色块**
- [ ] **B2** 97 单位 + 2491 装饰物静态摆放呈现为**真实 glTF WC3 模型**：成片真实树木（非竖直细杆 / 圆锥）、真实岩石、真实金矿 / 中立建筑（有细节的立体网格 + 真实贴图颜色），与地形贴合、无整体错位或悬空；**不得出现四面体 / 圆锥 / 长方体等基本几何体占位**
- [ ] **B3** 动态单位呈现真实模型：农民（Peasant）/ 士兵（Footman）/ 主城（TownHall）/ 兵营 / 金矿 / 树均为真实 glTF 网格 + 真实贴图，敌我按队伍色 / 模型外观区分；模型为 bind pose 静态站姿（无走路 / 攻击动画属已知限制），拉近相机（`W` / 滚轮）可辨模型细节
- [ ] **B4** `Status` 窗口含：Tick、Resources（Gold / Lumber / Supply）、Selection 面板、Commands 命令卡、Production 队列、Minimap 小地图、底部按键提示；**文字全部可读、无缺字方块**
- [ ] **B5** 所有存活单位有血条；选中单位额外有绿色选中环/框
- [ ] **B6** 小地图显示真实 `war3mapMap.png` 的 CPU 降采样底图（32×32 颜色网格，多色、非纯灰底），蓝/红实体点与相机视口框叠加在底图之上、位置与 3D 画面对应

## C. 选择、命令与失败反馈（对应 8.2）

- [ ] **C1** 左键拖框 / 单击选中己方单位 → 选中环出现，HUD 的 Selection 显示类型与 HP；命令卡随选中类型变化
- [ ] **C2** 空选中时按 `G`/`B`/`T`/`C`/`H`/`Q` → 日志显式 `input action rejected: action=... error=no unit selected for input action`（**设计行为，非崩溃**）
- [ ] **C3** 右键空地 → 单位移动；多选后右键 → 群体移动（落点分散、不叠一起）
- [ ] **C4** 右键敌方单位 → 生成 Attack；`Q` 施法需先选中且点中敌方目标，否则 `no target entity picked`
- [ ] **C5** `G`（采集）：选中农民 → Gold 数值增长
- [ ] **C6** `B`（建造）：选中主城 → Production 出现 `Production #4: 40 Tick` 并逐帧递减 → 完成后队列消失、画面新增兵营（长方体）
- [ ] **C7** `T`（训练）：选中生产者 → `Production #1: 30 Tick` 递减 → 完成后生成士兵（**新单位可见，不再是「队列生效但看不见」**）
- [ ] **C8** `C`（取消）：队列项立即消失、不生成实体；`H`（停止）：单位停止当前动作
- [ ] **C9** `F1`–`F9` 编组、`1`–`9` 按组选择：选中集合恢复，期间不产生命令
- [ ] **C10** 命令卡按钮 `[Move]` / `[Attack]` / `[Stop]` / `[Gather]` / `[Train]` / `[Build]` / `[Cancel]` 可点击并触发对应动作

## D. 小地图与相机（对应 8.2 / 8.4）

- [ ] **D1** 点击小地图某处 → 相机平移到该处、3D 画面随之变化；**不生成任何命令**（无 `Attack` / `Move` 日志、选中集合不变）
- [ ] **D2** `W` / `A` / `S` / `D` 平移、滚轮缩放、`Z` / `X` 旋转，画面随之变化（注意 `S` 是相机后移，**停止是 `H`**）

## E. 打完一局（对应 8.3，核心验收线）

- [ ] **E1** 开局双方各 1 主城 + 3 农民 + 金矿 + 树，HUD 显示 Gold / Supply 初值
- [ ] **E2** 观察 AI 对手会行动：约每 60 逻辑帧（2 秒）采集/训练，敌方单位数量 / 资源有可见变化
- [ ] **E3** 按 `G` 采集几次 → `B` 造兵营 → `T` 训练士兵
- [ ] **E4** 选中士兵，**连续多次右键点击敌方主城**（合成攻城单位单次 Attack 造成 30 伤害、主城 120 HP，约需 4–5 次；每次右键一条 Attack 命令）
- [ ] **E5** 敌方主城 HP 归零消失后 → HUD 出现 `VICTORY: Winner 1`，且只结算一次（无重复刷屏）
- [ ] **E6** 全程无 panic / 无 `render failed`；胜负显示后窗口仍稳定

> 已知限制：窗口内**暂无重开按键**，`design.md` Risks 已登记；重开以「关闭窗口后重新运行入口」替代，CPU 层已验证以相同冻结定义与开局脚本重新构造一局。

## F. 窗口尺寸与刷新率（对应 8.4）

- [ ] **F1** 拖拽改变窗口尺寸 → 画面正常、无拉伸错位，选中框仍贴合实体
- [ ] **F2** 高刷（144 Hz）持续呈现 → 逻辑帧恒定 30（Tick 每秒约 +30），无随刷新率加速
- [ ] **F3** `Resized` 不触发逻辑帧跳变（Tick 连续递增，无突跳）

## G. 日志与合规（对应 8.5）

- [ ] **G1** 所有日志为英文结构化 `key=value`；无任何中文文案进日志；启动含 `real map loaded: ... terrain_tiles=6 minimap=true model_vertices=... entity_models=6 models_resolved=2588/2588` 与 `models assembled: placements=2588 resolved=2588 ...`（真实 glTF 模型全量命中）
- [ ] **G2** `config.toml` 与导出目录仅位于本机且被 `.gitignore` 忽略；`git status` 无新增地形 / 贴图 / 底图 / MPQ 派生资产文件；日志不写回资产内容；**仓库零资产**（ADR 0001）——注意 `melee` 入口**确实读取** gitignored 本机路径，这是合规行为，不得据此放宽资产入库禁令
- [ ] **G3** 记录「CI 只覆盖编译 + CPU 测试，未验证 GPU 链路」，不以 CI 绿灯代替本清单

## 验收记录

| 分组 | 结论 | 证据（截图 / 日志片段） |
|---|---|---|
| A1–A5 启动与日志 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| B1–B6 真实地图画面与 HUD | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| C1–C10 选择与命令 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| D1–D2 小地图与相机 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| E1–E6 打完一局 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| F1–F3 尺寸与刷新率 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |
| G1–G3 日志与合规 | ☐ 通过 ☐ 不通过 ☐ 未执行 | |

## 验收后回填

1. 把各分组结论同步到 `openspec/changes/melee-playable-loop/tasks.md` 的 `6.3` 与 `8.1`–`8.5`，逐项勾 `- [x]`；无证据项如实标注「未执行 / 待补验」，**不冒充通过**。
2. 若某组不通过，把 `window created: ... scale_factor=...`、`real map loaded: ...` 与 `input action rejected: ...` 日志贴回给实施方定位；清单未逐项勾选前不得归档。
3. 已知限制（窗口内重开按键、命令卡 Move/Attack 以光标为目标、AI 脚本 horizon 3600 帧、中文上屏，以及**后续待办**：逐像素 GPU 纹理过滤、真实 glTF 骨骼动画逐帧蒙皮、少数非 EI 子集装饰物用同类真实模型兜底）见 `design.md` Risks / D7 / D8，不计为本清单失败项。真实地形 / 地表贴图 / 小地图底图（阶段 1/2）与**真实 glTF 模型 bind pose 网格 + 真实贴图（阶段 3）本期均已接入**，不再是「留后续 change」；基本几何体仅在本机无模型资产时作为回退出现。
