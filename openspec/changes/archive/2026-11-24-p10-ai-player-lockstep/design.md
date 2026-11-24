# Design

## Context

见 `proposal.md` 的 Why / What Changes：本 change 是 `WC3-MELEE-RULES.md §0.3 T5:57` / `R12.7:249` / S1 / S2 所指「AI 玩家 × 联机 lockstep 接合」的后续 AI / 联机 change。P0–P9 已实施并归档（P9 双族全量 + bump 8 刚归档，归档目录 `openspec/changes/archive/2026-11-23-p9-undead-foundation/`），当前 `openspec list` = No active changes、主 specs 30 份、`SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`、golden hash `final_hash=0x0b53a047b1f8e5e0`、proto tag 46–58 / `reserved 14 to 32` 保持、`GameplayCommandKind` 码 1–36、determinism 26 项、`verify_all.py` 全绿（本会话 `openspec list --specs` / Grep 逐锚复核，C5）。

**本轮授权口径 = 用户 2026-11-23 两次 `AskUserQuestion` 提前拍板架构 Q 与范围 Q**：P1 方向「AI 玩家 × 联机 lockstep 接合」、P2 决策侧「内核各端确定性重算」（被否「外部 `AiScript` 经 relay 广播」「双模式可切换」）、P3 范围「最小接合」（被否「全量 AI 行为本期做完」「最小接合 + 本期定 C 类边界」）。按 `CODEBUDDY.md §8`「承重约束与拍板决策逐条站稳后才成文」，四件套按该**单一已拍板口径一次成文**：本文件 D1–D7 不写「可容纳两种结果」、`tasks.md` 不列两分支、`proposal.md` Scope 不写待选。规格门 Q1–Q9 只承载「已拍板架构下的实现层处置」、逐条四要素见 `decision-memo.md`「## 待确认项」，MUST NOT 重新提出范围 / 架构 Q。

**现状逐点核实（本会话 Read / Grep 逐锚核对，对应 `decision-memo.md` 承重约束 C1–C8）**：

| # | 现状 | 证据锚 |
|---|---|---|
| S1 | 内核 `run_ai` 已存在且已进哈希 / 快照：step 第 10 阶段每帧重算、内部命令不写回队列 / 不注入重放；`fn run_ai` 只作用 `Order::Idle` 空闲单位、经与玩家相同的 `validate_gameplay_command` 校验入口、只发 `Gather` | `crates/w3-core/src/sim_match.rs:737-738,3231-3261` / `gameplay.rs:1286,1355,1986,2028-2031` / `sim_match.rs:2480-2483`（`set_ai`） |
| S2 | `AiState`（`enabled` / `next_decision_frame` / `decision_period` / `pending_tasks`）已进 FNV-1a 状态哈希（`gameplay.rs:2028-2031`）与 v8 快照（`deterministic-simulation/spec.md:42` 字面「AI 状态与待执行定时任务」）；`SetAi { enabled: bool }` 是既有命令面 | `gameplay.rs:1286-1290,1355,1986,2028-2031` / `crates/w3-core/src/snapshot.rs:56-57` |
| S3 | client `AiScript` 是本地 `melee` 窗口开发夹具：帧冻结外部命令、`SetAi{enabled:false}` 关内核 `run_ai`、`Leave` 结算、`AI_PLAYER` 署名、按 `(execute_frame, player_id, sequence)` 稳定排序、16 组合四族回归 | `crates/w3-client/src/melee/ai.rs:1-32,48-71,188-217,238,556` |
| S4 | 联机 `ClientSession` 持 `player_id`；`submit_input` 把普通 / 玩法命令上行同一 `CommandBatch`、明确「MUST NOT 本地静默 `submit_gameplay_command`（否则两端命令流不一致导致 desync）」；relay lobby 按连接分配稳定 `player_id`（0 留 relay、从 1 起）、`assembler.submit` / `broadcast_frame` 向 `lobby.players()` 广播；`Frame` / `Checksum` / `Welcome` / `Reject` / `ResyncReplay` 处理齐备 | `crates/w3-net/src/session.rs:22-33,91-109,124-183` / `crates/w3-relay/src/lib.rs:31-60,205-208,279-288` |
| S5 | `resolve_victory` / `VictoryState` 既有能力（R11.1 摧毁对手全部建筑 / R11.4 只结算一次、结算帧进权威状态 / R11.5 重开干净）；P9 的 CPU 一局脚本与逐帧哈希回归是先例 | `WC3-MELEE-RULES.md:233,236,237` / `openspec/specs/melee-match/spec.md:94-111,145-169` / `openspec/changes/archive/2026-11-23-p9-undead-foundation/tasks.md:32-35` |
| S6 | 主 specs 历史「`SNAPSHOT_VERSION` 仍 3 / 内核零改动 / golden hash 逐字节不变」口径残留；P9 Q9' 已按用户拍板登记为已知限制、由 `docs-owner` 后续独立 change 收口 | `lockstep-networking/spec.md:79,94,112` / `melee-match/spec.md:115,131,140` / `openspec/changes/archive/2026-11-23-p9-undead-foundation/decision-memo.md:110,137` |
| S7 | 参考对照命中矩阵 T1 / T4 / T5 / T6 / T7（对话内产出、未落盘、列人工确认项）；五个参考项目均**无**「AI 决策结果作为独立命令注入联机 lockstep 帧流」的实现（结论 9） | `docs/references/UPSTREAM.md:98-127` / `decision-memo.md` C6 / 本文件「## 参考项目对照」 |

**MODIFIED delta 的身份键说明**：4 份 `## MODIFIED Requirements` delta 逐字保留主 spec 历史 `### Requirement:` / `#### Scenario:` 标题作归档合并身份键（含「Transmit only input commands」「重放不双重执行」「对手会行动」「快照往返」等历史措辞），行为口径一律改写在各 Scenario 的 THEN 与 requirement 新正文，每条带 `>` 身份键说明；delta 顶部不写说明块、不写 `## Purpose` 覆盖（已存在主 spec 写 Purpose 会无效并触发首个 `##` 前文本 warning）。归档时才合并，本阶段不改主 `openspec/specs/**` 正文。共 **5 份 delta = 1 ADDED（`ai-player-lockstep`）+ 4 MODIFIED**，低于 `Consider splitting changes with more than 10 deltas` 上限（P9 R30 先例）。

## Goals / Non-Goals

**Goals:**

- 按用户 P2 / P3 已拍板单一口径一次成文四件套 + 纪要：收敛为「**内核各端确定性重算 + 最小接合**」单一路径，不写「可容纳两种结果」、不列两分支、不把 P2 / P3 写成 AI 推荐、不静默采用被否方向。
- **P2 接合**：启用 `w3-core` 既有 `run_ai`、复用既有 `AiState` / `SetAi`，联机帧流只承载真人玩家命令，各端相同开局冻结定义 / 相同外部命令流、`run_ai` 自然各端重算，靠逐帧状态哈希一致；AI 席位与真人席位对称（同一 PlayerId 命名空间 / 校验入口 / 结算），`submit_input` MUST NOT 本地静默注入。
- **P3 交付**：AI 席位进联机 lockstep session、打完一局 1v1（真人 VS AI），逐帧 `compute_state_hash()` 一致 + `first_difference()` 为 `None` + 回放逐帧一致 + 重开干净；「打完一局」胜负判据 D4 显式可执行、不留白。
- 确定性硬门禁：复用既有 `AiState`（已进哈希 / v8 快照）、Q1 推荐零新增权威状态字段保持 8 不 bump、determinism 26 项重跑 + 反向验证形态①、golden hash 口径按 Q1 结果如实处置、不声称全平台达成。
- 1 ADDED + 4 MODIFIED = 5 份 delta、其余登记已知限制；R1 可移动性过滤；四件套一致性（proposal Scope / specs `#### Scenario:` / 本文件 alternatives / tasks 步骤同口径）。

**Non-Goals:**

- 不重新提出范围 / 架构 Q、不写「交用户拍板 / 不预设结论 / 3 候选待选」作为范围待选、不自称规格已锁定、不 self-approve、不回填指纹、不拿沉默当授权。
- 不静默窄化用户 2026-11-14 已拍板的 1.0 范围、不静默改 `WC3-MELEE-RULES.md`（§14 第 2 条、是否本期同步交规格门 Q7 拍板）、不把参考项目原创设定 / 自述验收写成经典 WC3 事实或已验证事实、不声称跨平台确定性已达成。
- 不做 AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）本期完成、不定 C 类边界（`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7`）、不重做 P4 联机底座、不改 W3N1 25 字节帧头 / `FrameScheduler` / `stabilize` / `apply_frame`、不把 AI 决策作为独立命令注入 lockstep 帧流、不注入 `AiScript` 的 `Leave`、不引入 `f64` / 墙钟 / 独立随机流入权威状态。
- 不做 N1–N6（纯本地单机对局形态 N1 / 地图编辑器 / RPG 自定义地图 / 剧情战役 / JASS VM / 资产入库）；不全量刷新 17+ 份主 specs 历史「仍 3」口径残留 / 所有 `## Purpose` 概述文字（P9 Q9' 已拍板按已知限制收口）；不改主 `openspec/specs/**` 正文（归档才合并 delta）、不改 ADR 正文、不改 `crates/**`（提案阶段）、不 archive / commit / push。

## Decisions

> 每个决策关联 ADR 0002 编号、列出 alternatives considered、说明对跨平台确定性（ADR 0002 复核记录仅 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证；v4–v8 侧仅 Windows 重取基线；macOS / ARM64 / 其它 rustc 待验证）与分层依赖（SRP / OCP / LSP / DIP / ISP / LoD / CARP + 唯一状态所有者 / `w3-core` 不反向依赖上层 / 显式失败不静默降级）的影响；玩法决策声明所依据的业务规则编号 `R-x.y` 与核实状态（`[已核]` / `[待核]` / `[取舍]`），`[待核]` 数值不得冻结进权威定义。本 change 不冻结任何 `[待核]` 数值。

### D1 决策侧与命令注入 = 内核各端确定性重算（P2 已拍板；R12.7 `[取舍]` / R12.2 `[取舍]` / §0.3 T5 `[取舍]` / S1 `[已核]`）

**决定**：联机 lockstep session 启用 `w3-core` 既有 `run_ai` 作为 AI 席位**唯一**决策源；AI 决策状态（`AiState`）为权威状态、复用既有 `SetAi` 命令面；联机 `CommandBatch` **只承载真人玩家命令**，AI 决策结果 MUST NOT 作为独立命令注入帧流或经 relay 广播；各端以相同开局冻结定义 / 相同外部命令流构造 `RtsMatch`、`run_ai` 每帧确定性重算，靠逐帧 `compute_state_hash()` 一致（S1 地基已存在且已进哈希 / 快照）。**依据**：用户 2026-11-23 第 2 次 `AskUserQuestion` P2（仓库文件外证据、人工确认项）、`R12.7:249`「MUST 复用既有 `SetAi` / `run_ai` 命令面、MUST NOT 旁路写入权威状态」、`R12.2:244`「只传输玩家输入命令」、`§0.3 T5:57`「多端确定性重放」、ADR 0002 §3/§5/§6-8、ADR 0005 §4 红线、参考对照结论 1 / 3 / 9（AI 决策 = 内核权威状态、各端重算、进哈希；联机帧流只承载玩家意图；五个项目均无「AI 结果注入帧流」实现）。**alternatives considered**：① **外部 `AiScript` 经 relay 广播**（P2 已否：把权威 AI 决策降级为可丢失 / 可乱序的网络命令、违反 `R12.2` / ADR 0005 §4、需新增 proto tag / 版本 bump、参考结论 9 无先例）；② **双模式可切换**（P2 已否：两套命令源并存令 `SetAi` 二值语义失去单一真值、易双重发命令破坏逐帧哈希、违反「一个 change 一个 owner / 不列两分支」）；③ **relay 侧跑 AI 并把结果作为权威命令下行**（否决：引入第二份权威状态 / 状态所有者不唯一、违反 ADR 0002 §5）；④ **沿用 `AiScript` 本地外部命令源直接塞进联机帧流**（否决：与 P2 已拍板「内核重算」正面冲突、N1 开发夹具不列验收线）。**跨平台确定性影响**（关联 ADR 0002 §3/§4/§5/§6-8）：AI 决策只依赖整数帧 + 稳定 `EntityId` 顺序 + 既有 `AiState`（已进哈希 / v8 快照），不引入宿主墙钟 / `f64` / `HashMap` 迭代 / 异步任务完成顺序，各端同输入同输出；但**不声称全平台达成**——仅沿用 v3 已验证结论，v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证。**分层影响**：唯一状态所有者 `RtsMatch`（`run_ai` 内部结算、不写回队列）、`w3-core` 不反向依赖 `w3-net` / `w3-relay`、表现层只读不反写、显式失败不静默降级（`submit_input` 禁止本地静默注入）；SRP（AI 决策集中内核）/ DIP（只读冻结定义）/ CARP（组合既有命令面）。

### D2 本期范围 = 最小接合（P3 已拍板；R12.7 `[取舍]` / §0.3 T5 `[取舍]` / C 类边界 `[待核 / 待拍板]`）

**决定**：本期只把「AI 席位」接进联机 lockstep session 并打完一局 1v1（真人 VS AI）、逐帧哈希一致 + 回放一致；S1 / `R12.7` 的 AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）登记为后续 change 承接；C 类边界（`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7`）本期**一律不定、不扩范围**，登记为已知限制 / 后续 change。**依据**：用户 2026-11-23 第 2 次 `AskUserQuestion` P3（人工确认项）、`WC3-MELEE-RULES.md:387`（C 类「待拍板范围 / 架构，参考项目不能核实、回到人拍板」）、`UPSTREAM.md:122` 第 5 条（参考缺口回到用户拍板不臆造）、P9 Q7' 已把 AI × lockstep 接合登记为后续独立 change。**alternatives considered**：① **全量 AI 行为本期做完**（P3 已否：超出最小接合、把本期扩成四族全量 AI 重做、与 C 类边界不定叠加）；② **最小接合 + 本期定 C 类边界**（P3 已否：`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 属 `[待核 / 待拍板]`、参考项目不能核实、定边界即扩范围）；③ **只做规格文档不改任何契约**（否决：假闭环、无「打完一局」可执行验收）；④ **按参考项目 Warsmash / godot 原创 A0–A4 / 1Hz 墙钟 AI 批次设本期水位**（否决：原创设定 / 单机设计不作经典 WC3 事实、参考缺口⑥）。**跨平台确定性影响**（关联 ADR 0002 §2）：本期不新增帧阶段 / 不改 `logic_fps=30`、AI 决策周期仍由整数帧 `next_decision_frame` / `decision_period` 驱动（既有权威状态），无墙钟 / 1Hz 驱动；范围收窄降低引入跨平台不确定源的面、但不构成新验证。**分层影响**：一个 change 一个 owner 协调（提案阶段仅 `spec-steward` 写 `openspec/**`）、不扩范围改 `crates/**`、不为呈现问题扩范围（收尾已知限制优先）。

### D3 AI 席位对称 / PlayerId 归属 / submit_input 边界（Q2 实现层处置，按席位对称单一口径；R12.7 `[取舍]` / R11.4 `[取舍]` / ADR 0002 §3）

**决定**：AI 席位与真人席位**对称**——同一 PlayerId 命名空间、同一 `validate_gameplay_command` 校验入口、同一套经济 / 胜负结算，差别只是席位标志；不另立 PlayerId 空间。各端 `RtsMatch` 以相同开局冻结定义 / 相同外部命令流构造，`run_ai` 自然各端重算。`ClientSession::submit_input` SHALL 把真人玩家普通 / 玩法命令上行同一 `CommandBatch`，MUST NOT 本地静默 `submit_gameplay_command` 注入 AI 命令（`session.rs:92-93`）。AI 席位无真实连接、不贡献命令帧；其 PlayerId 由开局冻结定义登记（Q2 推荐），relay `expected_player_count` / `lobby.players()` 语义是否排除 AI 席位、是否算 combatant 属 Q2 规格门拍板范围（不预设结论、不扩范围）。**依据**：参考对照结论 7（四项目 AI 与真人席位对称：同一 PlayerId / owner / house_id 命名空间、同一命令归属校验入口、同一结算，差别只是 `is_human` / `COMPUTER` 标志）、C4（relay 按连接分配 `player_id`、0 留 relay、AI 无连接）、S4（`submit_input` 禁止本地静默注入）、`R12.7:249`。**alternatives considered**：① **relay 为 AI 席位预留虚拟 `player_id`**（Q2 备选：改动 lobby 稳定分配逻辑、可能破坏 `player_id` 契约）；② **AI 席位完全不进 PlayerId 空间、只作 `run_ai` 的 owner 参数**（Q2 备选：与结论 7 席位对称冲突、且 `run_ai` 命令以 owner 署名需 PlayerId）；③ **AI 席位贡献命令帧并由 relay 署名**（否决：与 P2「帧流只承载真人命令」正面冲突、ADR 0005 结论 3「主机从已认证连接推导发送槽位」在 AI 无连接时无答案、参考缺口③）。**跨平台确定性影响**（关联 ADR 0002 §3/§6-8）：席位对称保证「同一命令归属校验入口 / 同一结算」在各端一致；`submit_input` 单一入口避免一端静默注入造成两端命令流不等 → desync 不可见（显式失败不静默降级）；PlayerId 归属由冻结定义登记而非连接分配，避免不同端连接顺序差异进入权威状态。**分层影响**：`w3-net` / `w3-relay` 改装配语义而不改 W3N1 线格式 / `FrameScheduler` / `stabilize`，`w3-core` 不反向依赖上层；SRP（席位归属集中）/ LSP（AI 与真人席位对同一命令面行为一致）。

### D4 「打完一局」胜负判据（本期最关键设计张力，MUST 显式回答、不留白；R11.1 `[已核]` / R11.4 `[取舍]` / R11.5 `[取舍]` / R12.4 `[取舍]`）

> **本节正面回答第五节第 4 条的设计张力**：`run_ai` 现仅「空闲单位采集」语义（`sim_match.rs:3231-3261` 只发 `Gather`、不产生进攻 / `Leave`），若不注入 `AiScript` 的外部命令（含 `Leave` 结算），一局**如何分出胜负**？MUST NOT 留白、MUST NOT 靠 `AiScript` 的 `Leave` 静默绕过。

**决定（可执行验收）**：联机 1v1 各端以相同冻结开局定义构造 `RtsMatch`（真人席位 + 启用 `run_ai` 的 AI 席位），注入**冻结开局外部命令夹具 = 真人玩家命令流**（采集 → 建造 → 训练 → 进攻 → 摧毁 AI 席位全部建筑），逐帧推进到 AI 建筑全毁 → 由既有 `VictoryState::resolve_victory` 按 `R11.1`「摧毁对手全部建筑」判胜、`R11.4`「只结算一次、结算帧进权威状态」产生唯一胜者与结算帧（`victory.winner` 为真人玩家、`victory.resolved == true`）；以同一命令序列回放，两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`、回放逐帧哈希与原局一致（`R12.4`）；重开后 `victory.resolved == false` 且视图为空（`R11.5`）。AI 席位靠 `run_ai` 每帧确定性重算（只作用空闲单位采集），**不注入 `AiScript` 的外部命令 / `Leave`**——`R11.3` 认输 / 时限属 P3 已拍板本期不定的 C 类边界，不被本期验收依赖。该判据已在 `specs/ai-player-lockstep/spec.md`「打完一局 1v1 的可执行胜负判据」与 `specs/melee-match/spec.md`「联机一局打完的胜负判据可执行」写成可执行 `#### Scenario:`（WHEN 指明注入冻结真人命令流 / THEN 指明 `resolve_victory` 唯一胜者 + 逐帧哈希 + 回放 + 重开），MUST NOT 只写「能打完一局」而无触发条件与断言。Q4 规格门只拍板「冻结开局命令夹具的具体形态 / 是否接受该推荐」，不重新提出范围 / 架构 Q。**依据**：S1（`run_ai` 语义）、S5（`resolve_victory` 既有能力 / P9 一局脚本先例）、`WC3-MELEE-RULES.md:233,236,237`、参考对照结论 6（`ConcordRTS` 回放恢复先核对 tick 与完整运行时哈希再事务提交）、`R12.4:246`。**alternatives considered**：① **冻结夹具里带 `Leave` 结算命令（`R11.3`）直接判负**（Q4 备选、不推荐：`Leave` 是 `AiScript` 外部命令语义、与「联机 session 不注入 `AiScript`」冲突、认输时限属 C 类边界）；② **靠 `run_ai` 自行产出进攻 / 结算命令分胜负**（否决：现 `run_ai` 无此语义、属 AI 全量行为后续 change、本期不扩范围）；③ **靠帧数上限 / 墙钟判定结束**（否决：违反 ADR 0002 §2 整数帧 / 无墙钟、`R11.1` 才是经典胜负条件）；④ **只断言「无 desync」而不触发结算**（否决：假闭环、不能称「打完一局」）。**跨平台确定性影响**（关联 ADR 0002 §2 / §6-8 / §8）：胜负由权威命令流 + 既有 `resolve_victory` 结算、结算帧进 `Snapshot`，逐帧哈希布局不因 AI 席位而分叉；验收只比「两端同脚本逐帧一致 + 回放一致」、MUST NOT 断言与改动前 v7 / 其它组合的哈希数值相等；golden hash 口径按 Q1 结果如实处置、不声称全平台达成。**分层影响**：命令夹具是**验收输入**（外部命令、经 `submit_input` 帧流），不是第二份权威状态；`RtsMatch` 仍是唯一写入者、`w3-core` 不被旁路、表现层只读；显式失败（未触发结算即断言失败、不静默通过）。

### D5 权威状态与版本处置（Q1 实现层处置，按「零新增权威状态字段、保持 8 不 bump」推荐单一口径；ADR 0002 §6-8 / R12.4 `[取舍]`）

**决定**：本期复用既有 `AiState`（`enabled` / `next_decision_frame` / `decision_period` / `pending_tasks.len()` 已进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`，`gameplay.rs:2028-2031`），**零新增权威状态字段** → Q1 推荐保持 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8` 不 bump，golden hash `0x0b53a047b1f8e5e0` / proto tag 46–58 / `reserved 14 to 32` / 命令码 1–36 / determinism 26 项保持。若规格门 Q1 拍板确需新增字段，则按 ADR 0002 全套 bump 8→9（四处同步 `compute_state_hash` / `Snapshot` 顶层 / `first_difference()` / `restore_snapshot`、golden hash 按新实现如实重取、proto 只追加 tag 59 起、旧 v8 明确拒绝、ADR 0002 / 0005 复核追加、反向验证形态①）。**依据**：S1 / S2（`AiState` 已在哈希 / 快照）、P8.1「定义层变化不 bump」/ P9 R19 先例、ADR 0002 §7「不支持跨版本迁移」、`decision-memo.md` R19「两种惯性不采用」、参考对照结论 1 / 6（AI 决策状态进哈希 / 快照 / 恢复校验）。**alternatives considered**：① **沿用 P8.5e「零内核改动 / 版本不 bump」惯性作背书**（否决：本期是 AI × 联机接合、不是纯表现层改动，惯性不采用）；② **沿用 P8.5a–d「逐期 bump」惯性反推本期必 bump 9**（否决：无新增权威状态字段前提、R19）；③ **保持 8 但偷偷塞新字段进快照**（否决：ADR 0002 §7 禁止同版本号静默错读、显式失败不静默降级）；④ **重做 P4 联机底座 / 改 W3N1 线格式**（否决：ADR 0005 §3 只追加、超出最小接合）。**跨平台确定性影响**（关联 ADR 0002 §6-8）：零新增字段时状态哈希字段集合 / 快照布局 / golden hash 逐字节不变、跨平台逐帧一致口径不变；若 Q1 选 bump 9 则哈希布局实质变化、golden hash 按新实现如实重取、v9 Debian 12 侧逐字节比对待 VM / 人工、不声称全平台达成或有新验证。**分层影响**：`w3-core` 不反向依赖上层、唯一状态所有者；`w3-net` / `w3-relay` 改动只限装配语义 / 版本拒绝（若 bump）、W3N1 帧头 / `FrameScheduler` / `stabilize` / `apply_frame` 不变。

### D6 两套 AI 来源归位与本地夹具（Q3 实现层处置，按「联机 run_ai + 本地夹具保留」推荐单一口径；R12.7 `[取舍]` / N1）

**决定**：联机 session 启用内核 `run_ai`（`SetAi{enabled:true}`）；本地 `melee` 窗口 `AiScript`（`SetAi{enabled:false}`）**保留**为开发夹具、不列 1.0 验收线、不改其既有 16 组合四族回归；两者 `SetAi` 二值互斥、MUST NOT 双重发命令。联机 session MUST NOT 注入 `AiScript` 外部命令 / `Leave`。**依据**：用户 P2 已拍板「内核各端确定性重算」、S3（`AiScript` 本地夹具现状）、N1（纯本地单机对局形态不作验收线）、参考缺口②已由 P2 消解、`WC3-MELEE-RULES.md:32`（本地 `melee` 窗口保留为开发夹具）。**alternatives considered**：① **统一收口到 `run_ai`、删除 `AiScript`**（Q3 备选、不推荐：改动面大、破坏 P8 / P9 已归档 16 组合回归与本地开发夹具）；② **联机也用 `AiScript`**（否决：P2 已否「外部脚本广播」）；③ **双模式可切换**（否决：P2 已否、两套来源并存破坏确定性）。**跨平台确定性影响**（关联 ADR 0002 §3/§5）：`SetAi` 二值互斥保证任一形态只有一个 AI 命令源，避免双份 `Gather` / 决策周期错乱改变逐帧哈希；本地夹具是表现层 / 装配层外部命令、不进联机权威状态、不影响各端重算。**分层影响**：SRP（联机 / 本地夹具各自单一职责）、OCP（保留既有夹具不破坏回归）、不扩范围改代码。

### D7 回放 / 随机流 / 反向验证与门禁（Q5 / Q6 / Q8 实现层处置；R12.4 `[取舍]` / ADR 0002 §4 / §8 / CODEBUDDY.md §5）

**决定**：`run_ai` **不消费主 LCG、不分独立随机流**（现只读 `units` 稳定顺序、经 `validate_gameplay_command` 校验、发 `Gather`，无随机抽取，参考对照结论 5「No random draw is spent」），AI 决策周期由整数帧 `next_decision_frame` / `decision_period` 驱动（已进 v8 快照 / 哈希）；回放按同一外部命令序列 + 同一冻结定义逐帧重放 `run_ai` 即一致，`AiState` 已进 v8 快照、恢复先核对版本与完整哈希再事务提交（结论 6）；断线重连 / 观战里 AI 席位决策状态恢复、是否算 combatant 属 `R12.5` C 类边界（P3 已拍板本期不定），按 Q6 登记已知限制 / 后续 change、不扩范围。`cargo test -p w3-core --test determinism`（26 项）重跑、新增 / 修改回归按 `CODEBUDDY.md §5` **证据形态①**（测试内构造反例 + 元断言防退化）做反向验证（改坏验红）：退化实现（AI 命令本地静默注入不经帧流 / 一端不启用 `run_ai` / `AiState` 漏进哈希或快照 / `run_ai` 内部命令写回队列或注入重放 / `HashMap` 迭代 / 墙钟 / 含建筑误生成 `Move`）MUST 真经 `capture_snapshot()` / `compute_state_hash()` / `restore_snapshot()` / 公开命令入口 / `step()` 生产路径注入、非旁路模拟、不冒充形态②；`cargo test --workspace` / `cargo fmt --all -- --check` / `check_source_conventions.py` / `check_encoding.py` / `openspec validate --strict` / `verify_all.py --fingerprint`（`status: pending` 期 fail-closed 为正确信号）全跑。**依据**：ADR 0002 §4（每帧一次主 LCG）/ §8（determinism 门禁）、`CODEBUDDY.md §5`、`openspec/config.yaml:70-71`、参考对照结论 5 / 6 / 8（陷阱清单 → 反向验证清单）、P8.5a–e / P9 形态①先例。**alternatives considered**：① **为 AI 决策分独立 LCG 流**（Q5 备选、不推荐：改 ADR 0002 §4 主 LCG 契约、`Snapshot` 新增随机流字段 → 触发 Q1 bump 分支与 §8 规格解锁）；② **形态② A-B 对照 / 刻意改坏被测代码验红**（工作量更大、交 Q 按推荐形态①）；③ **仅求回归全绿**（否决：仅全绿不构成验证、`CODEBUDDY.md §5`）；④ **本期实现断线重连里 AI 席位恢复**（否决：P3 已否 C 类边界、不扩范围）。**跨平台确定性影响**（关联 ADR 0002 §4/§8）：不分独立流 / 不引入 `f64` / 墙钟 / 无序容器，逐帧哈希布局稳定；反向验证对象为「AI 命令是否经帧流 / `AiState` 四处同步 / 内部命令不写回 / 回放一致」，证据形态适用边界在此登记、由 `critic` 清单第 11 项③复核；v8 Debian 侧逐字节比对仍待 VM / 人工、不声称全平台达成或有新验证。**分层影响**：反向验证真经生产路径、不建旁路模拟；显式失败不静默降级；真窗口 / CI / 跨平台按 Q8 如实标注（C8）。

## 参考项目对照

> 本节是 `design.md` 的强制小节（`UPSTREAM.md:118-121` 触发时机第 2 条 + `openspec/config.yaml:67` `rules.design`），完整转写 `reference-researcher` 按 `docs/references/UPSTREAM.md` 矩阵（`:98-127`）对五个**只读**参考项目产出的「参考对照报告」。**命中任务类型 T1 / T4 / T5 / T6 / T7**（T4 / T5 由「可能」上调为确认命中、用途收窄为「席位 / 命令语义」，其非 lockstep 属性本身构成参考缺口；T2 / T3 不命中）。**报告未落盘、为对话内产出 / 本会话用户消息 / leader 转达、列为人工确认项；逐锚全开由 `critic` 清单第 12 项复核**。五个参考项目**只读、不复制任何代码**，`WarsmashModEngine`（AGPLv3）仅借鉴语义、禁止复制代码（`openspec/config.yaml` 的 `rules.design` 硬要求），`ConcordRTS`（GPL-3.0-only）RA2/YR 专属格式 / 规则不作 WC3 事实来源、README 自述验收不作已验证事实。参考结论有「缺口」时回到用户拍板、不臆造（`UPSTREAM.md:122` 第 5 条）。**结论 9 的「参考项目没有」MUST NOT 推论成「本项目不该做」**：P2 架构由用户明确拍板，参考项目只锚机制语义、给不了范围 / 架构答案（对照差异 D7）。

### 一、T1–T7 命中判定（矩阵锚 `UPSTREAM.md:104-114`）

| 类型 | 项目 | 判定 | 矩阵锚 | 命中理由 / 用途 |
|---|---|---|---|---|
| T1 确定性内核 / lockstep / 状态哈希 / 命令排序 / 快照 | `vera20k` | **命中** | `UPSTREAM.md:106` | AI 决策状态进全状态哈希 / 快照 / 逐帧 checksum、整数帧周期重算、各端跑同一份 AI 决策（结论 1 / 4 / 5） |
| T2 移动 / 寻路 / 编队 | `rts_kernel_cs` | 不命中 | `UPSTREAM.md:107` | 本报告仅以 `docs/architecture/RTS_KERNEL.md:85,149` 作同类陷阱佐证；`double` 路线不采用 |
| T3 WC3 数据格式 / SLK / 资产管线 | `godot_warcraft3` | 不命中 | `UPSTREAM.md:108` | 本 change 不触及资产管线 / 导出；零资产边界仍适用 |
| T4 玩法语义 / JASS / 单位行为 | `WarsmashModEngine` | **确认命中（用途收窄）** | `UPSTREAM.md:109` | 仅借「AI 席位 / 命令归属语义」（结论 7 席位对称）；其单机 / 无联机帧流属性构成参考缺口；AGPLv3 只借语义 |
| T5 场景装配 / HUD / Melee 初始化 | `godot_warcraft3` | **确认命中（用途收窄）** | `UPSTREAM.md:110` | 仅借「AI 席位标志 / 单机 AI 语义」（结论 7 / 9）；原创三族 / 1Hz 墙钟 / 单机 A0–A4 不作经典 WC3 事实 |
| T6 网络传输 / relay 广播 | `vera20k` | **命中** | `UPSTREAM.md:111` | 命令队列归模拟所有、`house_id` + frame stamp 稳定派发、单人 + AI 用 `NetworkSendPolicy::Local` 不广播（结论 4）；本项目自有 W3N1 / ADR 0005 为准、不照搬线格式 |
| T7 通用 RTS 引擎架构 / 确定性内核与表现层分离 / 跨平台验收方法论 | `ConcordRTS` | **命中** | `UPSTREAM.md:112` | 共享生产会话 / `computerControlled` 席位标志、联机帧流只承载玩家意图、AI 状态进权威哈希 / 保存 / 回放恢复（结论 2 / 3 / 6 / 7）；RA2/YR 专属规则不作 WC3 事实 |

### 二、结论 1–9（逐条：项目 / 路径 / 借鉴 / 不借鉴 / 许可证 / 锚）

- **结论 1（T1）**：`vera20k` 电脑玩家 AI 决策在**确定性模拟内核侧**、是 `HouseState` 权威状态一部分、按整数帧周期重算、决策状态全进全状态哈希与逐帧 checksum、各端跑同一份 AI 决策。**借鉴**：「AI 决策 = 内核权威状态、各端重算、进哈希 / 快照」（D1 / D5 / `specs/ai-player-lockstep/spec.md`）。**不借鉴**：RA2 专属 `House` / `BasePlan` 实现、gamem d 原生地址 / INI。**许可证**：GPLv3 + Section 7 附加条款、固定提交 `305ea3a75` 只读、零复制（附加条款只在真复制代码时进入）。**锚**：`vera20k/src/sim/ai_base_building.rs:1-11`、`src/sim/world/world_hash.rs:1-5,792-995,952-956,875,977-993`、`src/headless_scenario.rs:1625-1657`、`docs/sim-thread.md:11,63-64`、`docs/phobos-parallel-ai-queues.md:42-55`（矩阵条目 `UPSTREAM.md:106`）。
- **结论 2（T7）**：`ConcordRTS` 把 AI 放进「共享生产会话」——完整游戏与无窗口入口编译同一套模拟 / 命令 / 保存恢复，AI 是 owner 上的 `computerControlled` 值标志、不是独立进程 / 服务端。**借鉴**：AI 席位标志、不是独立进程（D3 席位对称 / D1 内核重算）。**不借鉴**：`README.md:24-27,646` 与 `API.md:848` 自述验收不作已验证事实；RA2/YR 规则不作 WC3 来源。**许可证**：GPL-3.0-only 只读、不复制、不引入依赖、不作子模块。**锚**：`ConcordRTS/README.md:22`、`API.md:310-314,533-537`、`src/simulation/ai/RuntimeComputerTacticsCodec.h:6-13`（矩阵条目 `UPSTREAM.md:112`）。
- **结论 3（T7/T6）**：`ConcordRTS` 联机 lockstep 帧流**只承载玩家意图**——命令 payload 故意不带归属、由权威主机从已认证连接推导发送槽位，canonicalize → 单一 capture → 先校验再事务提交 → 固定 tick，按 `peerId → 帧内 authored order` 稳定排序；`src/network/lockstep/` 全目录**没有** AI / computer / bot 生成命令证据。**借鉴**：稳定排序键 / 单一入口 / 本地 UI 不入命令流（D1 / D3 / D7）。**不借鉴**：线格式（ADR 0005 自有 W3N1 25 字节帧头 + 命令含 PlayerId）、「主机从已认证连接推导」在 AI 无连接时无答案（参考缺口③）。**许可证**：GPL-3.0-only 只读。**锚**：`RuntimeMultiplayerGameplayCommand.h:11-14,271,338-357`、`RuntimeMultiplayerPlayerIntent.h:79-81,177-184`、`RuntimeMultiplayerLockstep.h:13-33,79-98,150-167`、`RuntimeMultiplayerRuntimeBridge.h:40-48`（矩阵条目 `UPSTREAM.md:111-112`）。
- **结论 4（T1/T6）**：`vera20k` 命令队列归模拟所有、命令带 `house_id` + frame stamp、按 house → FIFO 派发；AI 席位在同一 house 命名空间、可标 ineligible 且**不重排后续 house id**；「单人 + AI」用 `NetworkSendPolicy::Local` 不广播（no other human needs a synchronization window）。**借鉴**：命令队列归模拟所有、席位在同一命名空间、稳定派发（D1 / D3）。**不借鉴**：`Queue_AI` 是 RA2 原生帧边界术语（命令交换 + checksum）、**MUST NOT 误读为「AI 命令经网络注入」**（`docs/sim-thread.md:32-33`、`src/sim/command_queue/checksum.rs:173-184`）；不照搬线格式。**许可证**：GPLv3 + Section 7、零复制、固定提交。**锚**：`vera20k/src/net/lockstep.rs:1-6,654-655,932-976,1180-1221,175-178`、`src/net/command_exchange.rs:1-6`、`src/net/live_commands.rs:267-278`、`src/net/live_commands/startup.rs:94`、`src/net/lan_messages/cooperative.rs:85-112`、`src/architecture_guards.rs:2330-2335`（矩阵条目 `UPSTREAM.md:106,111`）。
- **结论 5（T1）**：多端确定性重放做法——AI 决策状态进全状态哈希、决策周期整数帧、稳定顺序容器（`BTreeMap` / `keys_sorted` / `entries.sort()`）、全场单一同步 RNG、表现层 RNG 与 UI 时钟隔离、AI 选敌「No random draw is spent」、用稳定顺序打破平局。**借鉴**：整数帧周期 / 稳定容器 / 单一 RNG / 不消费随机（D5 / D7 / 反向验证清单）。**不借鉴**：RA2 专属 `House` 状态布局、`HashMap` 迭代顺序 / wall-clock 本地 UI 状态 / 浮点位模式 / 异步任务完成顺序（结论 8 陷阱清单）。**许可证**：GPLv3 + Section 7 零复制。**锚**：`world_hash.rs:242-246,289-312,303-312,151-152,1139-1147`、`docs/sim-thread.md:58-62,293-306`、`docs/ai-nearest-enemy.md:11-12,44-51`、`src/sim/command_queue/checksum.rs:173-184`（矩阵条目 `UPSTREAM.md:106`）。
- **结论 6（T7）**：`ConcordRTS` AI 状态（ComputerMining / ComputerTactics / AI 触发定义）明确进权威哈希、保存与回放 / 快照恢复校验；回放封包 `Initial/Command/Step/Reset/Finish/Selection` 六类，恢复先核对 tick 与完整运行时哈希再事务提交；UI 墙钟 / 随机 / 可变缓存排除在权威状态之外。**借鉴**：AI 状态进哈希 / 快照 / 恢复校验、回放恢复先核对哈希（D4 / D5 / D7）。**不借鉴**：`RuntimeAuthorityHashPrimitives.h:60-71` 的 `double` 位模式规范化属 RA2 路线、本项目禁 `f64` 进权威状态、不采用。**许可证**：GPL-3.0-only 只读。**锚**：`API.md:296-304,539-558,386-392,729-733,492-498`、`src/simulation/ai/RuntimeComputerAiTriggerHash.cpp:4-19`、`RuntimeComputerTacticsCodec.cpp:15-30`、`src/core/determinism/RuntimeAuthorityHashPrimitives.h:21-28`、`RuntimeMultiplayerRuntimeBridge.h:50-58`（矩阵条目 `UPSTREAM.md:112`）。
- **结论 7（席位对称，T1/T4/T5/T7）**：四个有 AI 席位语义的参考项目里 **AI 与真人席位对称**——同一 PlayerId / owner / house_id 命名空间、同一命令归属校验入口、同一套经济 / 胜负 / 计分结算；差别只是席位上的 `is_human` / `human` / `COMPUTER` / `difficulty` 标志。**借鉴**：席位对称、不另立 PlayerId 空间、同一校验入口 / 结算（D3 / `specs/ai-player-lockstep/spec.md`）。**不借鉴**：`WarsmashModEngine` AGPLv3 只借语义、不复制代码、若将来引入其 AI / JASS 代码 MUST 按 ADR 0001 复核点重评；godot 原创三族 / 1Hz 墙钟 / 单机 A0–A4 不作经典 WC3 事实。**许可证**：`vera20k` GPLv3 + Section 7 / `ConcordRTS` GPL-3.0-only / `WarsmashModEngine` AGPLv3 仅语义 / `godot_warcraft3` MIT 借结构。**锚**：`vera20k/src/sim/world/world_hash.rs:792-995,589-605`、`src/net/lockstep.rs:1188-1201`、`src/net/lan_messages/cooperative.rs:85-112`、`docs/ai-nearest-enemy.md:20-22`；`ConcordRTS/API.md:504-529,533-537`；`WarsmashModEngine/.../config/CBasePlayer.java:29-30,167-199`、`.../ai/AIDifficulty.java:5-9`、`.../ui/MenuUI.java:2358-2376`、`.../simulation/players/CPlayerUnitOrderExecutor.java:17-50`、`.../CSimulation.java:510-543`、`.../parsers/jass/JassAIEnvironment.java:24-45,165-167`；`godot_warcraft3/docs/design/game/MELEE_AI.md:7,15-23`、`docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md:63,137,143`、`docs/design/game/ROADMAP.md:14-24,85`（矩阵条目 `UPSTREAM.md:106,109-110,112`）。
- **结论 8（陷阱清单 → 反向验证清单）**：未冻结列表 / house 顺序（真实原生缺陷案例 `docs/ai-nearest-enemy.md:16-27`）、`HashMap` 迭代顺序、wall-clock / 本地 UI 状态、浮点位模式、异步任务完成顺序、联机规则不一致（`docs/ai-nearest-enemy.md:11-12`）。**借鉴**：逐条转为 D7 / `tasks.md` 反向验证（改坏验红）退化实现清单。**不借鉴**：`godot_warcraft3/docs/architecture/RTS_KERNEL.md:85,149` 仅作同类陷阱佐证、`rts_kernel_cs` `double` 路线不采用。**许可证**：同结论 1 / 5 / 6。**锚**：结论 5 / 6 全部锚 + `godot_warcraft3/docs/architecture/RTS_KERNEL.md:85,149`（矩阵条目 `UPSTREAM.md:106-107,112`）。
- **结论 9（关键参考缺口的证据）**：**五个项目均无「AI 决策结果作为独立命令注入联机 lockstep 命令帧流」的实现**——`vera20k` / `ConcordRTS` 的 AI 是内核权威状态（各端重算、进哈希）、联机帧流只承载玩家意图；`WarsmashModEngine` / `godot_warcraft3` 是单机、无联机帧流；`vera20k` 单人 + AI 用 `NetworkSendPolicy::Local` 不广播。**借鉴**：作为 P2「内核各端重算」的机制佐证、并登记「参考项目给不了范围 / 架构答案」的缺口（D1 / 对照差异 D7）。**不借鉴 / MUST NOT**：**不得把「参考项目没有」推论成「本项目不该做」**——P2 架构由用户明确拍板、不是参考项目反推；也不得把「参考项目没有 AI 注入帧流」当成「外部脚本广播方向被参考项目否证」（其否决依据是 `R12.2` / ADR 0005 / ADR 0002，见 `decision-memo.md` R1）。**许可证**：五个项目均只读、零复制。**锚**：`WarsmashModEngine/.../JassAIEnvironment.java:165-167`、`.../CSimulation.java:510-543`、`godot_warcraft3/docs/verification/rts_kernel/M0/R00.5_asbuilt.md:47-60`、`ConcordRTS/.../RuntimeMultiplayerPlayerIntent.h:79-81`、`vera20k/src/net/live_commands.rs:267-278`、`src/net/lockstep.rs:175-178`（矩阵条目 `UPSTREAM.md:106,109-112`）。

### 三、参考缺口（7 项，如实登记、回到人拍板、不臆造）

- **G1** AI 决策在哪侧 + 命令如何注入联机帧流（`§0.3 T5` 正是此空白）→ **本期已由用户 P2 明确拍板「内核各端确定性重算」消解、按已拍板登记**（`decision-memo.md` P2 / C1）。
- **G2** 本项目两套 AI 命令源（`run_ai` / `AiScript`）如何取舍 → **本期已由用户 P2 归位内核 `run_ai`**（`AiScript` 本地夹具保留属 Q3 实现层处置）。
- **G3** AI 命令署名 / PlayerId 归属（`ConcordRTS`「主机从已认证连接推导」链在 AI 无连接时无答案）→ **规格门 Q2**（D3 按席位对称单一口径论证、PlayerId 由冻结定义登记为推荐，是否算 combatant 交人拍板）。
- **G4** AI 决策随机流是否分独立流（ADR 0002 §4 每帧一次主 LCG）→ **规格门 Q5**（D7 推荐不分独立流、现 `run_ai` 无随机抽取）。
- **G5** 断线重连 / 观战里 AI 席位决策状态恢复、AI 席位是否算 combatant → **规格门 Q6**（P3 已拍板 C 类边界本期不定，此项只作「已知限制 / 后续 change」登记、不扩范围）。
- **G6** AI 难度档 / 决策频率 / 1Hz 墙钟 / A0–A4 批次均为 Warsmash / godot 原创或单机设计 → **不作经典 WC3 事实**（D2 / 对照差异 D6）；本期不冻结任何 `[待核]` 数值。
- **G7** 不得声称跨平台确定性已验证（ADR 0002 待验证项：v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc）→ **Q8 / C8**（D1 / D5 / D7 / 「## 已知限制 / 边界」如实标注）。

### 四、对照差异（显著登记、不抹平）

- **D-a** `vera20k` / `ConcordRTS` 的「AI 席位」在**各自 RA2 语义**里（`HouseState` / `computerControlled`）与本项目 WC3 席位语义不逐字对应 → 只借「席位对称 / 内核权威状态」机制，不借 House / RA2 规则（`WC3-MELEE-RULES.md` 仍是唯一业务规则权威）。
- **D-b** `ConcordRTS` 命令 payload「故意不带归属、由主机从已认证连接推导」vs 本项目 ADR 0005「命令含 PlayerId + 自有 W3N1 帧头」→ 只借稳定排序 / 单一入口，**不照搬线格式**；且 AI 无连接使「主机推导」链无答案（G3）。
- **D-c** `WarsmashModEngine` `float` 昼夜相位 / 1Hz 墙钟 / `CSimulation` 单机模拟、`godot_warcraft3` 原创三族 / A0–A4 vs 本项目 `SimFixed` Q47.16 整数帧 / 经典 WC3 四族 → B 类 `[待核]` 数值 / 原创设定不采信不冻结不入库、不作经典事实（D2 / G6）。
- **D-d** `ConcordRTS` `RuntimeAuthorityHashPrimitives.h:60-71` 的 `double` 位模式规范化 vs ADR 0002「禁 `f32` / `f64` 进权威状态」→ 明确不采用（结论 6）。
- **D-e（最关键）** 结论 9「五个项目均无 AI 结果注入联机帧流」vs 本项目 `§0.3 T5` 待确认空白 → **参考项目给不了范围 / 架构答案**（`UPSTREAM.md:122` 第 5 条）；P2 由**用户明确拍板**给出、不是 AI 推荐、不是参考项目反推；MUST NOT 把「参考项目没有」推论成「本项目不该做」、MUST NOT 把「参考项目没有注入」当成外部脚本方向被参考项目否证（其否决依据见 `decision-memo.md` R1）。

### 五、许可证核对

- **AGPLv3 `WarsmashModEngine`**（结论 7 / 9）：仅 `WarsmashModEngine` 是 AGPLv3，**只借语义、不复制代码、不引入依赖**；若将来引入其 AI / JASS 代码 MUST 按 ADR 0001 复核点重评（`UPSTREAM.md:109` / `ADR 0001:66-73`）。
- **GPLv3 + Section 7 `vera20k`**（结论 1 / 4 / 5 / 8）：固定提交 `305ea3a75` 只读、零复制；Section 7 附加条款只在真复制代码时进入、本项目不复制。
- **`ConcordRTS` GPL-3.0-only**（结论 2 / 3 / 6 / 7）：与本项目 GPLv3 同许可族、只读、不复制代码、不引入依赖、不作子模块；RA2/YR 专属不作 WC3 来源、`README.md` 自述验收不作已验证事实（`UPSTREAM.md:27-37,112`）。
- **MIT `godot_warcraft3` / `rts_kernel_cs`**：只借结构 / 同类陷阱佐证，不借实现、`double` 路线不采用；Godot / GDScript / C# 不复制。
- **零资产 / 零新增依赖**：全程未打开资产 / 派生文件、未跑构建导出脚本、未执行远端同步；本 change 不改 `FrozenDefinitions` / 冻结定义、不读 `config.toml` 或本机导出数据、预判零新增依赖（若引入须核许可 + `NOTICE.md` 登记 + `Cargo.lock` 提交）。防误用清单 `WC3-MELEE-RULES.md:373-384` 全项适用。

## 范围边界（P3 已拍板后的如实登记、非待拍板项）

本节把「本期做什么 / 不做什么」单独列明（与 `proposal.md` Scope / What Changes、`decision-memo.md`「## 已知限制 / 边界」、`tasks.md` 第 8 组一致）。

- **本期（P2 / P3 已拍板生效）**：内核各端确定性重算（`run_ai` / `AiState` / `SetAi`、帧流只承载真人命令、席位对称、`submit_input` 单一入口）+ 最小接合（AI 席位进联机 lockstep session、打完一局 1v1、逐帧哈希一致 + 回放一致 + 重开干净）+ R1 可移动性过滤 + determinism 26 项 / 反向验证形态① + 1 ADDED + 4 MODIFIED = 5 份 delta（其余登记已知限制）+ 真窗口清单保持未勾。**Q1–Q9 是已拍板架构下的实现层处置、由规格门逐条拍板**，不重新提出范围 / 架构 Q。
- **1.0 范围张力（如实登记、不静默窄化、不冒充已解决）**：S1「AI 玩家全部功能属 1.0 必做」（`WC3-MELEE-RULES.md:25,32`）与 P3「本期最小接合、AI 全量行为后续 change」的关系如实登记——本期只接合席位 + 打完一局，AI 全量行为属 1.0 必做但由后续 change 承接，不静默窄化用户 2026-11-14 已拍板的 1.0 范围、不冒充已完成（`decision-memo.md` L-1）。
- **C 类边界本期一律不定**：`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 登记为已知限制 / 后续 change（P3 已拍板、不扩范围）。
- **实施期核对 / 已知限制 / 待人工（非规格门范围 / 架构 Q）**：`[待核]` 数值本期不冻结（G6）；真窗口 8.x 保持 `- [ ]`「未执行 / 待人工」（Q8）；CI 双平台本会话未运行、v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成（G7）；主 specs 历史「仍 3」残留按 P9 Q9' 登记已知限制由 `docs-owner` 后续独立 change 收口；Q9' residual 文档（`§0.3 T5` 状态措辞 / 附录 R12 行 / §15 / 附录「28」「p9 在飞」过时措辞 / 主 specs「仍 3」/「四个·四者」）是否本期收口由规格门 Q7 拍板、不预设结论、不静默改活文档（§14 第 2 条）。
- **范围外**：AI 全量行为本期完成、C 类边界本期定、`N1–N6`、联机底座重做（P4 已交付）、JASS VM（N5）、AI 决策作为独立命令注入 lockstep 帧流、`AiScript` 的 `Leave` 注入联机 session、`f64` / 墙钟 / 独立随机流入权威状态。
- **本 change 不触及**：主 `openspec/specs/**` 正文（归档才合并 delta）、`docs/references/WC3-MELEE-RULES.md` 活文档（§14）、ADR 正文、`crates/**`（提案阶段）、任何资产 / 导出数据入库。

## Risks / Trade-offs

- **[Risk] 1.0 范围张力（AI 全量行为后续 change）** → Mitigation：`proposal.md` Scope / 本文件「## 范围边界」/ `specs/ai-player-lockstep/spec.md`「范围边界与后续承接」/`decision-memo.md` L-1 显式如实登记、不静默窄化用户 2026-11-14 已拍板的 1.0 范围、不冒充已解决；`critic` 清单第 13 项复核。
- **[Risk] 「打完一局」判据被误读为可留白 / 靠 `AiScript` 的 `Leave` 静默绕过** → Mitigation：D4 显式可执行验收（冻结真人命令流 → `R11.1` 摧毁全部建筑 → `resolve_victory` 唯一结算 → 逐帧哈希 + 回放 + 重开）、写进 `specs/ai-player-lockstep/spec.md` 与 `specs/melee-match/spec.md` 的可执行 `#### Scenario:`；Q4 只拍板夹具形态、不重提范围 / 架构 Q；`R11.3` 认输时限属本期不定的 C 类边界、不被验收依赖。
- **[Risk] AI 席位 PlayerId 归属 / 是否算 combatant 未定（G3 / Q2）** → Mitigation：D3 按席位对称单一口径论证（同一 PlayerId 命名空间、冻结定义登记）、Q2 规格门交人拍板、不预设结论；`submit_input` 禁止本地静默注入、relay 按连接分配 `player_id` 的现状（C4）在纪要 / 规格门如实登记；`critic` 清单第 7 / 10 项复核四件套一致性。
- **[Risk] 参考对照报告未落盘 / 锚未逐锚全开 / 结论 9 被误推为「本项目不该做」** → Mitigation：本文件「## 参考项目对照」完整转写结论 1–9 + 7 项缺口 + 许可证核对 + 对照差异 D-e，`decision-memo.md` C6 转写关键锚；**列为人工确认项**、逐锚全开由 `critic` 清单第 12 项复核；MUST NOT 把「参考项目没有」推论成「本项目不该做」、P2 由用户拍板不是参考反推。
- **[Risk] 版本 / golden hash 口径被惯性套用（Q1）** → Mitigation：D5 按「零新增权威状态字段、保持 8 不 bump」推荐单一口径、R19 两种惯性不采用、Q1 逐项写依据与代价交人拍板；`decision-memo.md`「被否方向台账」显式登记；`critic` 清单第 7 / 10 项复核。
- **[Risk] 参考项目原创设定 / 自述验收被冒充经典 WC3 事实或已验证事实** → Mitigation：「## 参考项目对照」逐条「不借鉴 / 许可证」+ 对照差异 D-a / D-c / D-d、`WarsmashModEngine`（AGPLv3）只借语义、`ConcordRTS` RA2/YR 专属不作 WC3 来源 / README 自述不作已验证事实、G6 / G7 如实登记；`critic` 清单第 12 项复核。
- **[Risk] 反向验证形态误冒充形态② / 陷阱清单遗漏** → Mitigation：D7 / `tasks.md` 按 `CODEBUDDY.md §5` 形态①（测试内构造反例 + 元断言防退化、退化实现真经生产路径注入），退化清单逐条覆盖参考对照结论 8（未冻结列表 / `HashMap` / wall-clock / `f64` / 异步顺序 / 联机规则不一致）；由 `critic` 清单第 11 项③复核。
- **[Risk] R1 可移动性 bug 从联机 / 键盘路径复发** → Mitigation：`specs/ai-player-lockstep/spec.md`「R1 可移动性过滤与表现层只读」Scenario 明确「含建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`」、`tasks.md` 反向验证含「含建筑误生成 `Move`」退化项（`CODEBUDDY.md §9:112` / `openspec/config.yaml:61`）。
- **[Risk] 所有 Q 待规格门拍板、`status: pending` 令 `verify_all.py` 变红** → Mitigation：`tools/verify_all.py::spec_gate_check()` 判 `invalid spec gate status` 是 fail-closed 正确信号，本阶段不绕过 / 不改门禁 / 不 self-approve / 不回填指纹 / 不写 `## 省略说明` 豁免；`python tools/verify_all.py --fingerprint p10-ai-player-lockstep` 只读重算如实记录、不回填。四件套按已拍板单一口径一次成文、不列两分支。
- **[Risk] 真窗口 / CI / 跨平台不冒称（Q8 / C8）** → Mitigation：`tasks.md` 真窗口清单逐项 `- [ ]`「未执行 / 待人工」、不提供替代、不冒充通过；仅沿用 ADR 0002 复核记录 v3 双平台已验证结论，v8 Debian 侧 / macOS / ARM64 / 其它 rustc 待验证、本 change 不声称全平台达成或有新验证；CI 双平台 `verify_all.py` 本会话未运行、不冒充已绿。
- **[Risk] Warsmash AGPLv3 / `rts_kernel_cs` `double` / 资产入库 / 本机 GPU 崩溃** → Mitigation：「## 参考项目对照」第五节硬红线（只借语义禁复制代码、`double` 不采用、零资产、`f64` 不进哈希快照）；本机 `config.toml` 存在时 GPU 原生崩溃为 HEAD 固有、CI 干净环境全绿，如实登记为已知限制、本阶段不改 `crates/**`。

## 已知限制 / 边界（如实登记，不冒充已解决、不扩范围）

与 `decision-memo.md`「## 已知限制 / 边界」L-1–L-6、`proposal.md` Scope、`tasks.md` 第 8 组逐条对应。本节只登记、不扩范围改主 specs 正文 / 活文档 / ADR / `crates/**`。

- **L-1（AI 全量行为后续 change）**：S1 / `R12.7` 的建造 / 训练 / 进攻 / 战斗 / 清野 / 结算等 AI 全量行为属 1.0 必做、本期按 P3 已拍板「最小接合」登记为后续 change 承接；本期 `run_ai` 只作用空闲单位采集。
- **L-2（C 类边界本期一律不定）**：`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 按 P3 已拍板本期不定、不扩范围；Q6（断线重连 / 观战里 AI 席位恢复、是否算 combatant）按已知限制登记、不冒充已解决。
- **L-3（Q9' residual 文档）**：活文档 `§0.3 T5` 状态措辞 / 附录 R12 行 / §15 / 附录「28」「p9 在飞」过时措辞 / 主 specs「仍 3」残留 / 「四个·四者」残留按 P9 Q9' 登记为后续独立 change 待办；本期是否收口交规格门 Q7 拍板、不预设结论、不静默改活文档（§14 第 2 条）。
- **L-4（真窗口 / CI / 跨平台）**：真窗口 8.x 保持 `- [ ]`「未执行 / 待人工」、不用离屏 / CPU 回归冒充；CI 双平台本会话未运行；v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成或有新验证；本机 `config.toml` 存在时 GPU 原生崩溃为 HEAD 固有、CI 干净环境全绿（C8 / Q8）。
- **L-5（参考对照报告未落盘 / 锚未逐锚全开 / 参考缺口）**：C6 报告为对话内产出、未落盘、列人工确认项，结论 1–9 / 7 项缺口 / 许可证核对完整转写进本文件「## 参考项目对照」、承重锚指向 `UPSTREAM.md:98-127` 矩阵条目与参考项目内路径；逐锚全开由 `critic` 清单第 12 项复核；参考缺口 G1 / G2 已由用户 P2 拍板消解、按已拍板登记，G3–G7 如实登记、回到人拍板、不臆造。
- **L-6（主 specs 历史「仍 3」口径残留 / `## Purpose` 概述文字）**：`lockstep-networking/spec.md:79,94,112` / `melee-match/spec.md:115,131,140` 等字面「`SNAPSHOT_VERSION` 仍 3 / 内核零改动 / golden hash 逐字节不变」与当前 v8 事实不符，按 P9 Q9' 已拍板「登记为已知限制、由 `docs-owner` 后续独立 change 统一收口」处置、不冒充已全量对齐、不为凑数全量改主 specs；本 change 只修订 4 份承重 delta 的 4 条 Requirement、其余登记已知限制。
- **L-7（规格门 fail-closed / §8 规格解锁）**：`status: pending` 时 `tools/verify_all.py::spec_gate_check()` 判 `invalid spec gate status` 令门禁变红是**正确信号**；本阶段不绕过 / 不改门禁 / 不冒充 approved / 不回填新指纹 / 不 self-approve / 不由实现方自行放行；`--fingerprint` 只读重算如实记录、不回填。「目标与方向已清楚的小改动可显式声明省略规格门」**不适用本 change**，不写 `## 省略说明` 豁免。Q1–Q9 由人逐条拍板、不拿沉默当授权。

## Migration Plan

- **部署 / 回滚**：按 Q1 推荐（零新增权威状态字段、保持 8 不 bump）本 change 是**联机 session 装配 + 客户端夹具归位 + 回归**的接缝改动，**无快照 / 回放版本契约变更**：`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 保持 8、`Snapshot` 顶层字段集合 / `defs_hash` / `content_hash` / golden hash `0x0b53a047b1f8e5e0` / proto tag 46–58 / `reserved 14 to 32` / 命令码 1–36 不变、不提供跨版本迁移（ADR 0002 §7）。若规格门 Q1 拍板 bump 8→9，则按 ADR 0002 §7 / ADR 0005 §3 全套：旧 v8 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` / `VersionMismatch` + relay `Reject` 明确拒绝、golden hash 按新实现如实重取、ADR 0002 / 0005 复核记录追加、不删改 §1–§8 正文。回滚即还原 client / net / relay 装配与回归、主 specs MODIFIED 合并结果；本机导出资产留 gitignored、不随代码回滚。
- **顺序（`CODEBUDDY.md §9`，本阶段止于 ①）**：① 本轮：`spec-steward` 按 §8 一次成文四件套 + `decision-memo.md` → `openspec validate p10-ai-player-lockstep --strict` 零 error/warning + `check_encoding.py` 全绿 + `--fingerprint` 只读重算四件套指纹如实记录（不回填）→ ② **规格门**：`critic` 规格门报告 → 用户逐条拍板 Q1–Q9 并确认「批准并锁定」（拍板结果若与已拍板口径不同须按 §8 同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户确认锁定）→ leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`（本轮 `status: pending` 不变、不 self-approve / 不回填）→ ③ `/opsx:apply`（`client-owner` / `net-owner` / `sim-owner` / `docs-owner` 按 §9 串行协作、不并行改同一 owner 文件）→ ④ `critic` 代码门（全清单）→ ⑤ 本地 commit（中文描述 + Conventional Commits 英文 type 前缀）→ ⑥ 人工 push → CI 双平台 `verify_all.py` → ⑦ 真窗口人工清单逐项勾选 → ⑧ `/opsx:archive` 并确认主 specs 合并（新增 `ai-player-lockstep` + 4 份 MODIFIED）。**指纹失配即视为规格已解锁**，MUST 按 §8 同步回查其余三份、由 `spec-steward` 修订后重走规格门，不得由实现方自行放行。
- **归档 delta 去向**：新增 `ai-player-lockstep` 主 capability（`## Purpose` + 7 条 ADDED Requirements / 12 条 Scenario）+ 4 份 `## MODIFIED Requirements` 命中 `lockstep-networking`（1 条 / 2 Scenario）/ `gameplay-systems`（1 条 / 2 Scenario）/ `melee-match`（1 条 / 3 Scenario）/ `deterministic-simulation`（1 条 / 4 Scenario），共 **5 份 delta**（计数以交付报告实测为准）。主 `openspec/specs/**` 本阶段**不变化**（归档时才合并）。`status: pending` fail-closed 为正确信号、不绕过。
