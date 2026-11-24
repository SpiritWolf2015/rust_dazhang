# Tasks

> 授权边界（本阶段）：本文件是 `p10-ai-player-lockstep` 按 `CODEBUDDY.md §8`「承重约束与拍板决策逐条站稳后一次成文」的实施清单，**本阶段只产出四件套 + `decision-memo.md` 文档**；不 apply、不 commit、不 archive、不 self-approve、不回填规格门指纹、不改 `crates/**` / 主 `openspec/specs/**` 正文 / `docs/references/WC3-MELEE-RULES.md` / ADR / `docs/decisions/` / `openspec/config.yaml` / `CODEBUDDY.md` / CI 配置 / `tools/`。
> 口径：用户 2026-11-23 两次 `AskUserQuestion` 已把**架构 Q（P2 内核各端确定性重算）与范围 Q（P3 最小接合）提前拍板**；四件套按该单一已拍板口径一次成文，本 tasks **不列两分支**、不写「可容纳两种结果」、不静默采用被否方向（外部 `AiScript` 广播 / 双模式 / 全量 AI 本期 / 最小接合 + 定 C 类边界）。规格门 Q1–Q9 只承载「已拍板架构下的实现层处置」、逐条四要素见 `decision-memo.md`「## 待确认项」，由人拍板、不拿沉默当授权；`status: pending`、不回填指纹、不写 `## 省略说明` 豁免。
> 提案阶段交付前自检（由 `spec-steward` 在交付报告给出命令输出，不占实施期复选框任务）：`openspec validate p10-ai-player-lockstep --strict` 零 error 零 warning → `python tools/check_encoding.py` 全绿 → `python tools/verify_all.py --fingerprint p10-ai-player-lockstep` 只读重算四件套 sha256 指纹如实记录、不回填 → `git status --short` 仅 `?? openspec/changes/p10-ai-player-lockstep/` → `critic` 规格门报告 + 用户逐条拍板 Q1–Q9 + 批准锁定。`status: pending` 令全量门禁 spec gate 变红是 fail-closed 正确信号，不绕过 / 不改门禁 / 不 self-approve / 不为了变绿写 `approved`。
> 每条任务带可核验完成判据（命令 / 测试 / 产物）。规则依据 `docs/references/WC3-MELEE-RULES.md` 正文当前行号与规则编号 `R-x.y`（附录历史旧行号不沿用）；确定性硬门禁 ADR 0002 / `CODEBUDDY.md §5`；参考结论见 `design.md`「## 参考项目对照」；Q1–Q9 见 `decision-memo.md`「## 待确认项」。一个 change 一个 owner 协调：`client-owner` / `net-owner` 为主接缝、`sim-owner` / `docs-owner` 配套，按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。全 `- [ ]` 未执行。

## 1. 规格门 Q 拍板前的实施准备与单一口径核对（P2 / P3 已拍板、Q1–Q9 待规格门）

- [x] 1.1 核对四件套 + `decision-memo.md` 按「内核各端确定性重算 + 最小接合」单一已拍板口径一致：`proposal.md` Scope / `specs/**` 的 `#### Scenario:` / `design.md` D1–D7 alternatives / 本 tasks 步骤互不打架，无「可容纳两种结果」/ 两分支 / 待选范围；验证：`critic` 清单第 7 / 10 项复核、`openspec validate p10-ai-player-lockstep --strict` 零 warning、`decision-memo.md` 的 P2 / P3 只记已拍板项且不写成「AI 推荐」（§8 / P1 / P2 / P3）
- [x] 1.2 逐条复核承重约束 C1–C8 证据锚仍成立、无一条被本期文档改动证伪：`WC3-MELEE-RULES.md:25,32,34,42,57,249`（C1）/ `CODEBUDDY.md §5` + `config.yaml:9-14,26-27,59,70-71`（C2）/ 内核 `run_ai` / `AiState` / `AiScript` 源码锚（C3）/ `session.rs` / `relay/lib.rs`（C4）/ v8 版本号 / golden hash / proto tag / determinism 26 项（C5）/ `UPSTREAM.md:98-127` 矩阵（C6）/ §14 第 5 条（C7）/ P9 纪要 `:109,:172`（C8）；验证：逐锚 Grep / Read 复核表、`critic` 清单第 12 项复核参考锚可打开性（C1–C8）
- [x] 1.3 核对被否方向台账 R1–R4 与两种惯性未被静默采用：外部 `AiScript` 广播 / 双模式 / 全量 AI 本期 / 最小接合 + 定 C 类边界、P8.5e「零内核改动背书」/ P8.5a–d「逐期 bump 反推」两种惯性；验证：`decision-memo.md`「## 被否方向台账」与 `design.md` D1 / D2 / D5 alternatives 逐条对照、`critic` 清单第 7 项复核（P2 / P3 / R19）

## 2. 联机 session AI 席位装配（内核 run_ai 各端重算，P2 已拍板；R12.7 / R12.2 / §0.3 T5 / S1）

- [x] 2.1 由 `net-owner` / `client-owner` 在联机 lockstep session 装配 AI 席位：启用 `w3-core` 既有 `run_ai`（`SetAi{enabled:true}`）、复用既有 `AiState` / `SetAi` 命令面，各端以相同开局冻结定义 / 相同外部命令流构造 `RtsMatch`、由 `run_ai` 每帧确定性重算；验证：两端 `run_ai` 决策状态逐帧一致、CPU 回归绿、`git diff -- crates/w3-core/` 按 Q1 结果处置（R12.7 / R12.2 / §0.3 T5 / ADR 0002 §5 / S1）
- [x] 2.2 保证联机 `CommandBatch` **只承载真人玩家命令**：`ClientSession::submit_input` 把普通 / 玩法命令上行同一 `CommandBatch`、AI 决策 MUST NOT 作为独立命令注入帧流或经 relay 广播、MUST NOT 本地静默 `submit_gameplay_command` 注入 AI 命令；验证：两端 `CommandBatch` 逐帧字节一致、无 AI 决策命令、无本地静默注入路径、`first_difference()` 为 `None`（R12.2 / R12.7 / ADR 0005 §4 / `session.rs:92-93` / 结论 3 / 9）
- [x] 2.3 实现 AI 席位与真人席位对称：同一 PlayerId 命名空间、同一 `validate_gameplay_command` 校验入口、同一套经济 / 胜负结算，差别只是席位标志；AI 席位无真实连接、不贡献命令帧、PlayerId 由开局冻结定义登记（Q2 推荐口径）；验证：席位对称 CPU 回归绿、`run_ai` 命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、不另立 PlayerId 空间、relay `player_id` 稳定分配契约不被破坏（R12.7 / R11.4 / ADR 0002 §3 / 结论 7 / C4 / Q2）
- [x] 2.4 由 `client-owner` 把本地 `melee` 窗口 `AiScript`（`SetAi{enabled:false}`）保留为开发夹具、不列 1.0 验收线、不改其既有 16 组合四族回归，并保证联机 session 与本地夹具 `SetAi` 二值互斥、不双重发命令、联机 session 不注入 `AiScript` 外部命令 / `Leave`；验证：既有 `ai.rs:238,556` 16 组合回归不改动仍绿、二值互斥断言绿、联机 session 无 `AiScript` 注入（P2 已拍板 / Q3 推荐 / N1 / R12.7 / `ai.rs:188-217`）

## 3. 权威状态 / 状态哈希 / v8 快照 / 版本处置（Q1 推荐单一口径：零新增权威状态字段、保持 8 不 bump）

- [x] 3.1 由 `sim-owner` 复核既有 `AiState`（`ai.enabled` / `next_decision_frame` / `decision_period` / `pending_tasks.len()`）已进 FNV-1a 状态哈希（`gameplay.rs:2028-2031`）、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`，本期**零新增权威状态字段**、不改 `Snapshot` 顶层字段集合 / `defs_hash` / `content_hash`；验证：状态哈希字段集合与 v8 基线一致、`cargo test -p w3-core` 绿、`git diff -- crates/w3-core/src/snapshot.rs` 为空（ADR 0002 §6-8 / R12.4 / Q1 / C2 / C3）
- [x] 3.2 保持 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`、golden hash `final_hash=0x0b53a047b1f8e5e0`、proto tag 46–58 / `reserved 14 to 32` / `GameplayCommandKind` 码 1–36 / determinism 26 项不因本期接缝改动变化（Q1 推荐口径）；验证：全仓 Grep 版本号 / golden hash 无残留改动、`cargo test --workspace` 绿、旧 v8 快照 / 回放仍按既有契约恢复、不提供跨版本迁移（ADR 0002 §7 / ADR 0005 §3 / Q1 / C5）；若规格门 Q1 拍板改为 bump 8→9，MUST 按 `CODEBUDDY.md §8` 同步修订四件套 + 重算指纹 + 重走规格门后由对应 owner 落地，不在本 tasks 另列分支
- [x] 3.3 复核 `run_ai` 决策周期由整数帧 `next_decision_frame` / `decision_period` 驱动、不消费主 LCG / 不分独立随机流（Q5 推荐口径），`run_ai` 内部 `Gather` 命令不写回命令队列、不注入重放；验证：每帧一次主 LCG 不变、`HashMap` 迭代 / 墙钟 / `f64` / 异步任务完成顺序不进权威状态、重放只注入外部真人命令、`cargo test -p w3-core` 绿（ADR 0002 §2-§4 / §6-8 / R12.4 / 结论 5 / Q5 / `sim_match.rs:3231-3261`）

## 4. 「打完一局 1v1」可胜负判据 + 逐帧哈希 + 回放一致（R11.1 / R11.4 / R12.4 / R11.5 / Q4）

- [x] 4.1 构造**冻结开局外部命令夹具 = 真人玩家命令流**（采集 → 建造 → 训练 → 进攻 → 摧毁 AI 席位全部建筑），各端以相同冻结开局定义构造 `RtsMatch`（真人席位 + 启用 `run_ai` 的 AI 席位）并逐帧推进到 AI 建筑全毁；验证：一局可打完、夹具是验收输入（外部命令、经 `submit_input` 帧流）而非第二份权威状态、不注入 `AiScript` 的 `Leave`（R11.1 / §0.3 T5 / R12.7 / D4 / Q4）
- [x] 4.2 断言胜负结算：经既有 `resolve_victory` 按 `R11.1` 摧毁对手全部建筑判胜、`R11.4` 只结算一次且结算帧进权威状态、`victory.winner` 为真人玩家、`victory.resolved == true`；重开后 `victory.resolved == false` 且视图为空（`R11.5`）；`R11.3` 认输 / 时限属本期不定的 C 类边界、不被验收依赖；验证：CPU 端到端一局回归绿、结算只一次（`WC3-MELEE-RULES.md:233,236,237` / P9 一局脚本先例 / Q4）
- [x] 4.3 新增「两端逐帧一致 + 回放一致」CPU 回归：同一命令脚本两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等（不只比最终 hash）、`first_difference()` 为 `None`、回放逐帧哈希与原局一致、`Frame` / `Checksum` / `Welcome` / `Reject` / `ResyncReplay` 处理路径不变；验证：`cargo test -p w3-net` / `cargo test -p w3-client` 绿、W3N1 25 字节帧头 / `FrameScheduler` / `stabilize` / `apply_frame` 不变（R12.2 / R12.4 / ADR 0002 §6-8 / ADR 0005 §3-4 / 结论 6）

## 5. determinism 26 项重跑与反向验证（改坏验红）（CODEBUDDY.md §5 形态① / ADR 0002 §8）

- [x] 5.1 重跑 `cargo test -p w3-core --test determinism -- --nocapture`（26 项）全绿，证明命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、30 Hz 整数帧、`SimFixed` Q47.16、稳定 `EntityId` 顺序、FNV-1a、每帧一次主 LCG 不变；验证：命令输出留档、26 项全绿（`CODEBUDDY.md §5` / `openspec/config.yaml:70` / ADR 0002 §2-§8）
- [x] 5.2 按 `CODEBUDDY.md §5` **证据形态①**（测试内构造反例 + 元断言防退化）做反向验证（改坏验红）：退化实现（① AI 命令本地静默注入不经帧流 ② 一端不启用 `run_ai` ③ `AiState` 漏进 `compute_state_hash` / `Snapshot` / `first_difference()` / `restore_snapshot` 之一 ④ `run_ai` 内部命令写回命令队列或注入重放 ⑤ 用 `HashMap` 迭代顺序 / 墙钟 / `f64` / 独立随机流 ⑥ 含建筑 / 资源点 / 主城选中集合误生成 `Move` ⑦ 联机规则不一致 / 未冻结 house 顺序）MUST 真经 `capture_snapshot()` / `compute_state_hash()` / `restore_snapshot()` / `first_difference()` / 公开命令入口 / `step()` 生产路径注入、非旁路模拟；验证：每组「缺陷态失败 / 修复态通过」元断言证据可复现、MUST NOT 冒充形态②「两条命令输出留档」、无法构造缺陷态的项在 `design.md` D7 登记理由与替代证据（`CODEBUDDY.md §5` / `openspec/config.yaml:71` / 结论 8 陷阱清单 / `critic` 清单第 11 项③）

## 6. 规则文档同步核对（§14 第 5 条 / config.yaml:72 / Q7）

- [x] 6.1 收尾回答「本期是否需要同步 `docs/references/WC3-MELEE-RULES.md`」：本期正是 `§0.3 T5:57` / 附录 R12 行（`:408`）所指「后续 AI / 联机 change」；按规格门 Q7 拍板结论处置——需同步则更新附录「规则 → 实现覆盖对照」R12 行与 §15 变更记录（并复核 `§0.3 T5` 状态措辞 / 附录「28」「p9 在飞」过时措辞 / 主 specs「仍 3」残留 / 「四个·四者」残留），无需同步则写明依据；`decision-memo.md`「## 规则文档同步」非空结论、`tasks.md` 本条复选框口径对齐；验证：`critic` 清单第 13 项复核「不静默改规则（§14 第 2 条）」「不把 design 引用 `R-x.y` 冒充已实现」「真窗口清单未标通过」、活文档 diff 与 Q7 拍板一致（`WC3-MELEE-RULES.md:408,421` / `openspec/config.yaml:56,72` / `decision-memo.template.md:48-53` / `verify_all.py::rule_doc_sync_check()`）

## 7. 全量回归、格式门禁、四件套一致性与编码基线（§8 / Q9）

- [x] 7.1 全量回归与格式门禁：`cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_source_conventions.py` / `python tools/check_encoding.py` 全绿、`openspec validate p10-ai-player-lockstep --strict` 零 warning；验证：各命令输出全绿、所有文件 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格、无新增未登记依赖、`Cargo.lock` / `NOTICE.md` 无 diff（`CODEBUDDY.md §2 / §3 / §5 / §7` / Q8）
- [x] 7.2 主 specs **5 份 delta（1 ADDED `ai-player-lockstep` + 4 MODIFIED）**归档前复核：逐字保留全部历史 `#### Scenario:` 身份键、口径改在 THEN、requirement 正文给完整新正文、每条带 `>` 身份键说明、MUST NOT 写 `## Purpose` 覆盖 delta、delta 顶部不写说明块、消除与已拍板方向（内核重算 + 最小接合）的正面冲突；其余主 specs 历史「仍 3」口径残留按 P9 Q9' 已拍板登记已知限制、不冒充已全量对齐、不为凑数全量改主 specs；验证：`critic` 清单第 7 / 10 项复核 `blocker: 0`、delta 数 5 < `Consider splitting changes with more than 10 deltas` 上限（P9 R30 先例）
- [x] 7.3 确认四件套一致性复核：proposal 的 Scope / specs 的 `#### Scenario:` / design 的 alternatives / tasks 的步骤按同一已拍板单一口径一致（内核各端确定性重算 + 最小接合 / Q1–Q9 待规格门 / 1.0 张力如实登记 / 形态① / 参考项目对照逐条）；若实施期任一 artifact 改动 MUST 按 `CODEBUDDY.md §8` 同步回查其余三份、由 `spec-steward` 修订后重走规格门；验证：`critic` 清单第 7 / 10 项复核、`decision-memo.md` 规格门指纹与四件套一致（须 `critic` 报告 + 用户确认锁定后由 leader 回填，本阶段 `status: pending`、不回填 / 不 self-approve / 不写 `## 省略说明` 豁免）

## 8. 真窗口人工验收清单（Q8 / C8：真窗口项未执行 / 待人工，不提供替代、不冒充通过）

> 本组为人工桌面验收项，CLI / 离屏自证无法覆盖 egui overlay 与真实 GPU 画面。**全部保持 `- [ ]` 与「未执行 / 待人工」、不用离屏 / CPU 回归冒充真窗口验收、CI MUST NOT 声称已验证 GPU**；GUI 类修复先自测再请人工验收（不反复试错）。Q8 已拍板要求本组保持未勾、无证据项如实标注、不冒充通过。

- [ ] 8.1 真窗口启动联机 1v1（真人 VS AI 席位）肉眼确认 AI 席位接入 lockstep session、两端画面一致、AI 空闲单位按整数帧采集、真人命令流可见 —— 状态：未执行 / 待人工（R12.7 / S1 / §0.3 T5 / P2）
- [ ] 8.2 真窗口肉眼确认一局能打完：真人采集 → 建造 → 训练 → 进攻 → 摧毁 AI 席位全部建筑 → 显示 `VICTORY`（唯一胜者）、重开干净无残留、两端逐帧确定性无异常 —— 状态：未执行 / 待人工（R11.1 / R11.4 / R11.5 / R12.4 / Q4）
- [ ] 8.3 真窗口肉眼确认含农民 + 主城 + 金矿 / 树木 / 建筑的选中集合右键移动或按 `M` 键时只移动可移动单位、建筑 / 资源点位置不变、命令卡 / 小地图 / HUD 只读呈现 —— 状态：未执行 / 待人工（R1.1 / R1.4 / R9.1 / R13.6 / ADR 0002 §5）
- [ ] 8.4 真窗口与离屏 `render_headless_rgba` + `ViewImage` 自证画面一致、随 30 Hz 逻辑帧稳定推进（先离屏自证 + CPU 回归全绿再请人工验收）—— 状态：未执行 / 待人工（R12.4 / Q8 / GUI 自测纪律）
- [ ] 8.5 `Resized` 窗口缩放 / 高刷 30 Hz 下 HUD 投影 / 命令卡 / 小地图与逐帧确定性无异常 —— 状态：未执行 / 待人工（无证据项如实标注，不冒充通过）
- [ ] 8.6 清单项保持 `- [ ]` 与「未执行 / 待人工」、无证据项如实标注、不用离屏截图 / CPU 回归冒充真窗口验收、CI 双平台 `verify_all.py` 本会话未运行如实说明、v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成 —— 状态：未执行 / 待人工（Q8 / C8 / ADR 0002 复核记录 / `CODEBUDDY.md §5 / §10`）

## Workflow follow-up

- 规格门（**本轮按 §8 一次成文后待规格门**）：四件套 + 纪要按 P2 / P3 已拍板单一口径一次成文（本轮已完成、`status: pending`）→ leader 用 `python tools/verify_all.py --fingerprint p10-ai-player-lockstep` 只读重算四件套指纹如实记录（不回填门禁）→ **`critic` 规格门报告** → **用户经 `AskUserQuestion` 逐条拍板 Q1–Q9 并确认「批准并锁定」**（拍板结果若与已拍板口径不同须按 §8 同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户确认锁定）→ **leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`** → `/opsx:apply`（`client-owner` / `net-owner` 为主接缝、`sim-owner` / `docs-owner` 配套，按 §9 串行协作）→ `critic` 代码门 → 本地 commit（中文描述 + Conventional Commits 英文 type 前缀）→ 人工 push → CI 双平台 `verify_all.py` → 真窗口清单逐项勾选 → `/opsx:archive` 并确认主 specs 合并（新增 `ai-player-lockstep` + 4 份 MODIFIED）。**本轮 `status: pending` 不变、不回填指纹、不 self-approve、不拿沉默当授权、不把 P2 / P3 写成 AI 推荐、不写 `## 省略说明` 豁免**。
- 已知限制承接（后续独立 change，不改本 change 四件套）：AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）（L-1）；C 类边界 `R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 与 Q6 断线重连 / 观战里 AI 席位恢复（L-2）；Q9' residual 文档（活文档 `§0.3 T5` 状态措辞 / 附录 R12 行 / §15 / 附录「28」「p9 在飞」过时措辞 / 主 specs「仍 3」残留 / 「四个·四者」残留，按规格门 Q7 拍板结论收口）（L-3）；主 specs 历史「仍 3」残留与所有 `## Purpose` 概述文字由 `docs-owner` 统一收口（L-6）；v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、真窗口 8.x 清单（L-4）；参考对照报告未落盘、逐锚全开由 `critic` 清单第 12 项复核、参考缺口 G3–G7（L-5）；`model_index` 固定临时目录并发 FAILED 需串行单实例门禁。
- 归档时逐条登记未勾任务原因、已知限制承接、主 `openspec/specs/` 是否变化据实说明；`status: pending` 令 `tools/verify_all.py::spec_gate_check()` 变红是 fail-closed 正确信号（`decision-memo.md` L-7），本阶段不绕过 / 不改门禁；`python tools/verify_all.py --fingerprint p10-ai-player-lockstep` 只读重算指纹如实记录、不回填门禁。
- 人工边界（非待拍板，仅登记）：本地 `git add` / `commit` 由 AI（中文描述 + Conventional Commits 英文 type 前缀），四件套 + `decision-memo.md` MUST 一并纳入 commit；`push` / `fetch` / `pull` / `clone` 及任何远端同步、CI、真窗口人工验收、`/opsx:archive` 归档全部人工独占且本轮未执行。
