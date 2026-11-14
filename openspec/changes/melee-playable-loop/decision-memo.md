# 决策纪要 — `melee-playable-loop`（首个实例）

> **推广状态**：本文件是「决策纪要」工作流的**首个实例**；该工作流已固化为项目规则（`CODEBUDDY.md` §8、`openspec/config.yaml` 的 `rules.proposal`、`spec-steward.md`），可复用模板见 `openspec/decision-memo.template.md`。
>
> **取料边界（如实标注）**：本 change 已 77/89、四件套早已成文，本纪要属**回填式实例**，只验证格式与证据锚纪律，**不验证**「拷问结构性前置于成文」——那要等下一个新 change 从提案阶段从零试。
>
> **取料纪律**：本纪要基于仓库已有 artifacts 回填。所有「拍板」项均能在 artifacts 内找到明确记载；**找不到记载的一律进「待确认项」，MUST NOT 把 AI 事后归纳写成「用户已确认」**（这正是「规格本身也是 AI 写的」要防的病）。
>
> **不改的东西**：不改 `.codebuddy/skills/`（OpenSpec init 生成、会被覆盖）、不改任何 Rust 源码；规则落在版本控制的权威文件里，不依赖本机全局状态。
>
> **合规边界**：纯文档；不进入 FNV-1a 状态哈希、不进入 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002 / CODEBUDDY.md §6）。

## 格式约定（已提取为 `openspec/decision-memo.template.md`，此处保留实例说明）

1. **三分判据**：能从仓库 / 本机 / 只读参考项目查到的事实，自己查并给结论，不占用户注意力；查得到候选与依据的，先给「推荐 + 依据 + 代价 + 备选」再请用户拍板；只在用户脑子里的（偏好、验收线、拍板权）才问。
2. **证据锚**：每条记录 MUST 带可核验锚点 —— `file:line`、`git` commit、或 artifacts 中明确记载的用户拍板。**无锚点记作「待确认」，MUST NOT 写成「用户已确认」**。
3. **不拿沉默当授权**：用户未明确表态 = 未拍板；「采纳推荐」须有明确记录。
4. **只记三类**：拍板决策（含被否备选与理由）、承重约束（支撑多个决策的前提，逐条带锚）、待确认项。**不写实现细节、不复述 `design.md`、不做流水账**。
5. **停止条件**：承重约束逐条站稳（有锚）且待确认项清空或显式豁免；**不以「问完预设问题 / 问了 N 轮」当收敛**。
6. **可裁剪**：目标与方向已清楚的小改动可显式标注省略本纪要；范围 / 方案澄清、事实核对、决定确认三件事不可省。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `w3-core` 零改动；`SNAPSHOT_VERSION` 仍为 3；不新增权威状态字段、不改哈希 / 快照布局 | `design.md:34,39` / `proposal.md:44,46` / `tasks.md:11,52,74` |
| C2 | 仓库零资产；真实导出数据只经 gitignored `config.toml` 进表现层，缺配置显式失败、不回落合成 demo | `proposal.md:7,26,47` / `design.md:45,106,128` / ADR 0001 |
| C3 | `w3-content` 原始 `f64` 坐标、PNG 像素、glTF 顶点 MUST NOT 进权威定义 / 状态哈希 / 快照 | `design.md:50,105,127,169` / ADR 0004 |
| C4 | 验收线是「能打完一局 1v1 Melee」，不以 CI 绿灯或「窗口能开」为准 | `proposal.md:5` / `tasks.md:57-65` / `smoke-checklist.md:57-65` |
| C5 | 人工 GPU 冒烟清单未逐项勾选前不归档；CI / headless 只覆盖编译 + CPU + 离屏 3D，不覆盖真窗口 egui overlay | `smoke-checklist.md:4-6,94-95` / `tasks.md:59` / `design.md:236,241,257` |
| C6 | 呈现 / GUI 类修复先离线查根因 + 离屏 readback 自证 + CPU 回归全绿，再请人工验收，不反复开窗 | `proposal.md:13,15,17` / `design.md:154,205` / `tasks.md:143,157` |
| C7 | 已知限制如实登记；人工清单无证据项标「未执行 / 待补验」，不冒充通过 | `smoke-checklist.md:6,94` / `design.md:122,147,171,196,222` / `tasks.md:139,153,168` |
| C8 | 新增依赖须经人工授权 + 许可证核对 + `NOTICE.md` 登记 + `Cargo.lock` 提交 | `proposal.md:29,45` / `design.md:28,106,237` / `tasks.md:90` |
| C9 | AI 不执行任何 `push` / `fetch` / `pull` / `clone`；归档与远端同步由人工 / 编排者执行 | `CODEBUDDY.md:32` / `tasks.md:71,137,151,167` / `design.md:257` |

## 拍板决策（用户明确拍板，含被否备选）

- **P1 验收线「能打完一局」**：采集 → 建造 → 训练 → 战斗 → AI 对手 → 胜负结算 + 完整 HUD。锚 `proposal.md:5`。被否：`playable-window-smoke` 的「窗口能开 / 框选有响应」——用户判定「根本没法玩」（`proposal.md:5`）。
- **P2 视觉水位对齐只读参考 `godot_warcraft3`，逐阶段追加**：真实 EI 地形 / 贴图（阶段 1/2）、真实 glTF 模型（阶段 3）。锚 `proposal.md:7,9` / `design.md:94,111`。被否：合成色块 demo、基本几何体占位（`tasks.md:75,97`）。
- **P3 换真 GPU 纹理管线**（`texture_2d_array` + UV + 逐像素采样 + alpha test）：第三次真窗口否定后，用户经 `AskUserQuestion` 明确选择。锚 `design.md:154,165-168` / `proposal.md:13` / commit `7b5622d`。被否：① 只修刺眼处、保留 CPU 顶点色；② 暂停画面先验收玩法（`design.md:154,166` 记为放弃选项）。
- **P4 阶段 2 继续做完再一次性请人工验收，并接受 `image`（png only）依赖**。锚 `tasks.md:84` / `design.md:28,106`。被否：阶段 2 即停、把真实贴图留后续（`tasks.md:84`）。

## 被否方向台账（`design.md` 按决策分散记录，此处单列以暴露「前提翻转」）

- **R1** 阶段 2 收尾曾登记「真实 glTF 模型留阶段 3 独立 change」→ 用户否定，在**同一 change** 内追加完成（`tasks.md:93`）。
- **R2** D9 首版「只修尺度 / 相机 / 特效层」→ 第三次否定后改判为**架构性**差距，整体换向 D10（`proposal.md:13` / `design.md:154`）。
- **R3** D11 三个方向全否：只改相机不动玩法坐标、主城放字面 sloc `(-81,46)`、清场过滤静态树（`design.md:190-193`）。
- **R4** D12 三个方向全否：只调 `world_to_screen` 固定高度、缩小 `PICK_RADIUS`、把 boxes 塞进 `ScreenProjection`（`design.md:217-219`）。
- **R5** D10 两个方向经实测 / 离线量化否决：独立半透明水面网格（锯齿无改善且多 6 万顶点）、把 `water>ground` 判水改成「消除 77% 误判」（`design.md:167-168`）。
- **R6** D9「面积过滤去伪影」量化后否决（会误删建筑大墙面，`design.md:145`）。
- **R7 ★ 前提翻转**：D7 阶段 2 曾以「shader / GPU 在 CLI 侧无法验证、是反复黑屏返工根源」**否决** GPU `texture_2d_array`（`design.md:102`）；D8 补上 headless GPU 自测后该前提被证伪，D10 由用户拍板**翻转为采纳**（`design.md:104,154-163`）。记录价值：否决理由是有条件的，前提变化 MUST 整体重估，不能沿用旧否决。

## 待确认项（本期未决，如实标注，不冒充已拍板）

- **人工 GPU 冒烟未执行**：`6.3` 与 `8.1–8.5`（`tasks.md:46,61-65`）、`14.10`（`tasks.md:136`）、`15.6`（`tasks.md:150`）、`16.7`（`tasks.md:165`）均未勾选；`smoke-checklist.md:82-90` 验收记录表全为未填。→ 待用户按清单逐项执行。
- **B9 + B10 门槛未确认**：「打开能找到基地、框住农民能选中、选中框贴合模型」（`smoke-checklist.md:36-37`）；未通过即反馈、不勉强往下勾 C/D/E 组。
- **已知限制是否接受并推进归档**：未在本 change 内拍板。用户在上一个 change（`playable-window-smoke`）的偏好是「记为已知限制直接归档、不扩范围」，但**不能据此推定本 change 已同意**。
- **`critic` 复审覆盖范围**：`9.1` 记录的是早期复审（`tasks.md:69`）；针对 D10–D12 的 `14.11` / `15.7` / `16.8` 尚未勾选（`tasks.md:137,151,167`）。→ 待确认是否需补复审。
- **「同一方向被否定 ≥2 次即停下重定方向」是否固化为项目规则**：本 change 有 6 轮否定记录（`proposal.md:9-17`），但仓库内无成文规则；属**待用户拍板的工作流改动**，不擅自写入本纪要的承重约束。

## 停止条件（后续在本 change 追加阶段前）

1. C1–C9 逐条仍有证据锚、无一条被本轮改动证伪。
2. B9 / B10 门槛通过，且人工清单无证据项已如实标注「未执行 / 待补验」。
3. 不以「预设问题问完」「问了 N 轮」当收敛。

## 推广结论（2026-11-14，用户拍板「推广」）

- 已提取为 `openspec/decision-memo.template.md`，并写入 `CODEBUDDY.md` §8、`openspec/config.yaml` 的 `rules.proposal`、`spec-steward.md` 职责。
- 本实例证明的增量：`design.md` 按决策分散记录时看不见「前提翻转」（R7），单列承重约束与证据锚后才暴露；这正是工作流要固化的部分。
- **仍未验证**：「拷问结构性前置于成文」——本实例是回填式，须在下一个新 change 从提案阶段从零试点。
- **仍为提示层约束**：`openspec validate` / `verify_all.py` 当前都不机械校验 `decision-memo.md` 是否存在与小节非空；如需机械门禁须另开一步。
