# 决策纪要 — `p10-ai-player-lockstep`

> 取料纪律：基于仓库 artifacts、只读参考项目结论与用户 `AskUserQuestion` 明确拍板记录回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进由 `SNAPSHOT_VERSION`（当前 8）定义的快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 本轮性质（务必先理解）：本 change 是「AI 玩家 × 联机 lockstep 接合」，承接 `WC3-MELEE-RULES.md §0.3 T5:57` / `R12.7:249` / S1 / S2。用户 2026-11-23 两次 `AskUserQuestion` 已把**架构 Q（决策侧）与范围 Q（本期水位）提前拍板**；按 `CODEBUDDY.md §8`「承重约束与拍板决策逐条站稳后才成文」，四件套按该单一已拍板口径一次成文（`proposal → specs → design → tasks`），不写「可容纳两种结果」、不列两分支、Scope 不写待选。下「规格门 Q1–Q9」只承载**已拍板架构下的实现层处置**，本轮不做规格门、不拍板、不 self-approve、不回填指纹、不拿沉默当授权。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 / 类别 |
|---|---|---|
| C1 | **1.0 范围与 §0.3 T5**：S1 是 1.0 唯一对局形态、含「真人 VS AI 敌人玩家」，AI 玩家全部功能（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）属 1.0 必做；`§0.3 T5:57` 字面「架构未定、后续做 AI / 联机 change 时 MUST 先回到人拍板」；`R12.7:249`「MUST 复用既有 `SetAi` / `run_ai` 命令面、MUST NOT 旁路写入权威状态」；N1 不做纯本地单机对局形态；AI 命令如何注入联机 lockstep 帧流登记为 §0.3 T5 待确认 | `docs/references/WC3-MELEE-RULES.md:25,32,34,42,57,249`；A 类 |
| C2 | **确定性硬门禁 ADR 0002**：新增权威状态 MUST 四处同步（`compute_state_hash` / `Snapshot` 顶层字段 / `first_difference()` / `restore_snapshot`）；改 `w3-core` MUST 跑 `cargo test -p w3-core --test determinism`、反向验证形态①、`cargo fmt --all -- --check`；30 Hz 整数帧、排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、稳定 `EntityId`、FNV-1a、每帧一次主 LCG、`logic_fps=30` 冻结 | `CODEBUDDY.md:50-54`（§5）/ `openspec/config.yaml:9-14,26-27,59,70-71` / `docs/decisions/0002-determinism.md:36-72`（§3/§4/§6-8）；A 类 |
| C3 | **两套 AI 来源现状**（本会话只读逐锚复核）：内核 `run_ai` — `sim_match.rs:737-738`（step 第 10 阶段「确定性 AI：每帧重算，内部命令不写回命令队列、不注入重放」）、`:3231-3261`（`fn run_ai`：只作用 `Order::Idle` 空闲单位、经与玩家相同的 `validate_gameplay_command` 校验入口、绝不写回命令队列，只发 `Gather`）、`:2480-2483`（`set_ai` 切 `self.gameplay.ai.enabled` / `decision_period` / `next_decision_frame`）；`gameplay.rs:1286`（`pub struct AiState`）、`:1986`（`GameplayState.ai: AiState`）、`:2028-2031`（`ai.enabled` / `next_decision_frame` / `decision_period` / `pending_tasks.len()` **已进 FNV-1a 状态哈希**）、`:1355`（`SetAi { enabled: bool }`）。client `AiScript` — `crates/w3-client/src/melee/ai.rs:1-32`（帧冻结外部命令、不扩展 `run_ai`、不新增权威状态）、`:48-71`（预生成到 horizon、按 `(execute_frame, player_id, sequence)` 稳定排序）、`:188-217`（`SetAi{enabled:false}` 关内核 `run_ai`、`Leave` 结算、`AI_PLAYER` 署名）、`:238,556`（16 组合四族回归）。**「内核各端确定性重算」的地基已存在且已进哈希 / 快照；`AiScript` 是本地 `melee` 窗口开发夹具（N1 不作 1.0 验收线）** | `crates/w3-core/src/sim_match.rs:737-738,2480-2483,3231-3261` / `crates/w3-core/src/gameplay.rs:1286,1355,1986,2028-2031` / `crates/w3-client/src/melee/ai.rs:1-32,48-71,188-217,238,556`（本会话 Grep / Read 逐锚复核）；A 类 |
| C4 | **联机 / 回放既有底座**：`ClientSession` 持 `player_id`；`submit_input` 把普通 / 玩法命令上行同一 `CommandBatch`、明确「MUST NOT 本地静默 `submit_gameplay_command`（否则两端命令流不一致导致 desync）」；relay lobby 按连接分配稳定 `player_id`（0 留 relay、从 1 起）、`assembler.submit(player_id, commands, gameplay)`、`broadcast_frame` 向 `lobby.players()` 广播；ADR 0005 W3N1 25 字节帧头 + protobuf payload、只传输入命令、回放逐帧哈希、`first_difference()`。**AI 席位无真实连接 → 其 `player_id` 归属 / 是否贡献帧 / 是否算 combatant 是规格门 Q2 的具体缺口** | `crates/w3-net/src/session.rs:22-33,91-109,124-183` / `crates/w3-relay/src/lib.rs:31-60,205-208,279-288` / `docs/decisions/0005-protobuf-payload.md:21-50`（本会话 Grep 逐锚复核）；A 类 |
| C5 | **v8 基线**：`SNAPSHOT_VERSION = SIM_VERSION = 8`、`PROTOCOL_VERSION = 8`；golden hash `final_hash=0x0b53a047b1f8e5e0`；proto tag 46–58、`reserved 14 to 32` 保持；`GameplayCommandKind` 码 1–36；determinism 26 项；主 specs 30 份；`verify_all.py` 全绿；当前无 active change。P9 归档纪要 R6/R7/R11/R19（bump 8 全套、P8.1「定义层变化不 bump」先例、两种惯性不采用） | `crates/w3-core/src/snapshot.rs:56-57` / `crates/w3-net/src/packet.rs:45` / `crates/w3-core/examples/golden_hash.rs:75-77` / `crates/w3-net/src/proto/w3net.proto:184,196,198` / `crates/w3-core/tests/determinism.rs`（26 个 `#[test]`）/ `openspec/changes/archive/2026-11-23-p9-undead-foundation/decision-memo.md:28,30,33,41` / `openspec list --specs`（30 份，本会话命令输出）；A 类 |
| C6 | **参考对照报告**（`reference-researcher` 对话内产出、未落盘、列人工确认项）：命中矩阵 **T1 / T4 / T5 / T6 / T7**（T4/T5 用途收窄为「席位 / 命令语义」，其非 lockstep 属性本身构成参考缺口；T2 / T3 不命中）；结论 1–9（AI 决策 = 内核权威状态 / 各端重算 / 进哈希与快照、联机帧流只承载玩家意图、席位对称、陷阱清单、**五个项目均无「AI 决策结果作为独立命令注入联机 lockstep 帧流」的实现**）、7 项参考缺口、许可证核对（AGPLv3 仅 Warsmash、Section 7 仅 vera20k、ConcordRTS GPL-3.0-only、MIT godot）。完整转写见 `design.md`「## 参考项目对照」；承重锚指向矩阵条目 `docs/references/UPSTREAM.md:98-127` 与参考项目内路径；逐锚全开由 `critic` 清单第 12 项复核 | `docs/references/UPSTREAM.md:98-127`（矩阵 T1–T7 / 触发时机 / 维护）/ 本会话用户消息 / leader 转达（仓库文件外证据、人工确认项）；A 类（机制语义）/ B 类（原创数值与自述验收不作 WC3 事实） |
| C7 | **规则文档同步（正式规则，§14 第 5 条）**：玩法 change MUST 在 `decision-memo.md` 含非空「## 规则文档同步」结论小节、tasks 含「规则文档同步核对」复选框、收尾更新附录「规则 → 实现覆盖对照」R12 行（`:408`）与 §15；`tools/verify_all.py::rule_doc_sync_check()` 结构门禁（仅对过规格门 `status: approved` 的玩法 change 生效，`pending` 豁免）、`critic` 清单第 13 项内容层复核。**本期正是 `§0.3 T5:57` / 附录 R12 行所指「后续 AI / 联机 change」** | `docs/references/WC3-MELEE-RULES.md:408,421` / `openspec/config.yaml:56,72` / `openspec/decision-memo.template.md:48-53` / `tools/verify_all.py:256-294`；A 类 |
| C8 | **Q8' 诚实边界**：真窗口 8.x 保持 `- [ ]`「未执行 / 待人工」、不用离屏 / CPU 回归冒充真窗口验收；CI 双平台 `verify_all.py` 本会话未运行如实标注；v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成；本机 `config.toml` 存在时 GPU 原生崩溃为 HEAD 固有、CI 干净环境全绿 | `openspec/changes/archive/2026-11-23-p9-undead-foundation/decision-memo.md:109,172` / `docs/decisions/0002-determinism.md:102-118`（复核记录）/ `CODEBUDDY.md §5:54,§10`；A 类 |

## 拍板决策（含被否备选与理由）

- **P1（第 1 次拍板，方向；用户明确拍板、非 AI 归纳）**：下一个 change = **「AI 玩家 × 联机 lockstep 接合」**，承接 `WC3-MELEE-RULES.md §0.3 T5:57` / `R12.7:249` / S1 / S2；这是 1.0 唯一对局形态里「真人 VS AI 敌人玩家」必做但架构未定的头号缺口。锚：本会话用户消息 / leader 转达（仓库文件外证据、人工确认项）+ `WC3-MELEE-RULES.md:25,32,57,249`。被否：无（用户点名方向，非选项否决）。
- **P2（第 2 次拍板 Q1，决策侧与命令注入；用户选推荐、明确拍板、非 AI 归纳）**：**「内核各端确定性重算」**——启用 `w3-core` 既有 `run_ai`，AI 决策状态为权威状态，联机帧流只承载真人玩家命令，各端靠状态哈希一致（C3 地基已存在且已进哈希 / 快照）。**被否**：①「外部 `AiScript` 经 relay 广播」（与「帧流只承载真人命令」/ ADR 0002 / ADR 0005 冲突、把 AI 决策当命令注入帧流，参考结论 9 佐证五个项目均无此实现）②「双模式可切换」（两套命令源并存会双重发命令破坏确定性、与「一个 change 一个 owner / 不列两分支」冲突）。MUST NOT 静默采用被否项。锚：本会话用户 2026-11-23 第 2 次 `AskUserQuestion`（仓库文件外证据、人工确认项）+ C3 / C4。
- **P3（第 2 次拍板 Q2，本期范围水位；用户选推荐、明确拍板、非 AI 归纳）**：**「最小接合」**——本期只把「AI 席位」接进联机 lockstep session 并打完一局 1v1（真人 VS AI），逐帧哈希一致 + 回放一致；S1 / `R12.7` 的 AI **全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）登记为后续 change 承接**。**被否**：①「全量 AI 行为本期做完」②「最小接合 + 本期定 C 类边界」。因此 **C 类待拍板边界（`R3.5` 资源共享 / `R5.5` 高地加成 / `R11.3` 认输时限 / `R12.5` 观战·断线重连 / `R13.1` grid / `R13.5` 界面键 / `R13.7` 键位别名）本期一律不定、不扩范围，登记为已知限制 / 后续 change**；N1–N6 不纳入。锚：本会话用户 2026-11-23 第 2 次 `AskUserQuestion`（仓库文件外证据、人工确认项）+ `WC3-MELEE-RULES.md:135,158,235,247,261,305,324,42-47`。
- **P0（本轮例行决定，非范围 / 方向拍板、如实登记）**：change 命名 `p10-ai-player-lockstep`（小写连字符、阶段前缀 P10；取「AI 玩家 × 联机 lockstep 接合」语义，区别于 `lockstep-networking` 传输底座与 `melee-match` 本地闭环），按 §8 用 `openspec new change` 建目录；一个 change 一个 owner 协调——提案阶段仅 `spec-steward` 写 `openspec/**`，实施期按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。锚：`openspec status --change p10-ai-player-lockstep --json`（本会话命令输出）+ `CODEBUDDY.md §8 / §9`。

## 被否方向台账（MUST NOT 静默采用；前提后续变化须整体重估）

- **R1「外部 `AiScript` 经 relay 广播 AI 决策命令」** → 用户 P2 明确否决。依据：与「联机帧流只承载真人玩家命令」（`WC3-MELEE-RULES.md:244` R12.2 / ADR 0005 §4 红线）冲突、把 AI 权威决策降级为可丢失 / 可乱序的网络命令、参考结论 9 五个项目均无此实现、`AiScript` 是 N1 本地开发夹具。锚：用户 2026-11-23 Q1 + C3 / C4 / `design.md`「## 参考项目对照」结论 9。
- **R2「双模式可切换（内核 `run_ai` / 外部脚本二选一）」** → 用户 P2 明确否决。依据：两套命令源并存令 `SetAi` 二值语义失去单一真值、易双重发命令破坏逐帧哈希、`tasks.md` 不得列两分支。锚：用户 2026-11-23 Q1 + `CODEBUDDY.md §8`。
- **R3「AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）本期做完」** → 用户 P3 明确否决。依据：超出「最小接合」、与 C 类边界本期不定叠加会把本期扩成四族全量 AI 重做。锚：用户 2026-11-23 Q2 + S1 / `R12.7`。
- **R4「最小接合 + 本期定 C 类边界」** → 用户 P3 明确否决。依据：`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 均属 `[待核 / 待拍板]`、参考项目不能「核实」（`WC3-MELEE-RULES.md:387` C 类），本期定边界即扩范围。锚：用户 2026-11-23 Q2 + `WC3-MELEE-RULES.md:387`。
- **两种惯性不采用（沿用 P9 R19）**：① 不沿用 P8.5e「零内核改动 / 版本不 bump」口径作本期背书；② 不沿用 P8.5a–d「逐期 bump」惯性反推本期必 bump 9。本期版本处置是规格门 Q1 由人拍板的实现层结论，不是惯性。锚：`openspec/changes/archive/2026-11-23-p9-undead-foundation/decision-memo.md:41`（R19）+ `docs/decisions/0002-determinism.md:109-118`。
- **N1–N6 范围**（纯本地单机对局形态 / 地图编辑器 / RPG 自定义地图 / 剧情战役 / JASS VM / 资产入库）不纳入（C1）；联机底座重做（P4 已交付、`R12.2` / ADR 0005）不做。

## 待确认项（规格门 Q 清单：已拍板架构下的实现层处置，本轮不拍板、不冒充已确认）

> 本节为 `tools/verify_all.py::decision_memo_check()` 三必填小节之一，并如实登记**未决事项**。下列 Q1–Q9 均为用户 2026-11-23 两次拍板**之后**、在「内核各端确定性重算 + 最小接合」单一已拍板口径下的**实现层处置**，MUST NOT 重新提出范围 / 架构 Q；每条按 P9 先例给「推荐 + 依据 + 代价 + 备选」四要素，交用户在规格门逐条拍板。`status: pending`、不 self-approve、不拿沉默当授权。

- **Q1 版本与权威状态处置**：本期是否保持 8 不 bump，还是 bump 8→9 全套。
  - 推荐：复用既有 `AiState` / `SetAi`、**零新增权威状态字段**（只改装配与 session 接缝 + 回归）→ 保持 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8` 不 bump。
  - 依据：C3 已确认 `ai.enabled` / `next_decision_frame` / `decision_period` / `pending_tasks` 已进 FNV-1a 状态哈希与 v8 快照；C5 基线；P8.1「定义层变化不 bump」/ P9 R19 先例；ADR 0002 §7「不支持跨版本迁移」。
  - 代价：保持 8 即 v8 快照 / 回放字节布局与 golden hash 不变、无需旧版本拒绝、改动面最小；但若实施期发现确需新增字段，须切到备选分支 → 按 §8 触发规格解锁、重算四件套指纹、重走规格门。
  - 备选：若确需新增字段则 bump 8→9 全套——四处同步（`compute_state_hash` / `Snapshot` 顶层字段 / `first_difference()` / `restore_snapshot`）、golden hash 按新实现如实重取、proto 只追加 tag 59 起 / `reserved 14 to 32` 保持、旧 v8 明确拒绝、ADR 0002 / 0005 复核追加、determinism 26 项、反向验证形态①。
- **Q2 AI 席位 `player_id` / 署名 / 贡献帧 / lobby 语义**：AI 席位由谁分配 PlayerId、是否贡献帧、是否算 combatant、`submit_input` 禁止本地静默注入如何满足。
  - 推荐：AI 席位与真人席位**对称**（同一 PlayerId 命名空间、同一命令校验入口、同一套经济 / 胜负结算，差别只是席位标志）；各端 `RtsMatch` 以相同开局定义 / 相同外部命令流构造、`run_ai` 自然各端重算；AI 席位无真实连接、不贡献命令帧、不另立 PlayerId 空间。
  - 依据：参考对照结论 7（四项目 AI 与真人席位对称）+ C4（relay 按连接分配 `player_id`、0 留 relay、AI 无连接）+ `R12.7` + `session.rs:92-93`「MUST NOT 本地静默注入」。
  - 代价：需在开局冻结定义里登记 AI 席位 PlayerId（而非 lobby 连接分配），`w3-relay` 的 `expected_player_count` / `lobby.players()` 语义可能须显式排除 AI 席位；「是否算 combatant」影响胜负结算计数。
  - 备选：① 由 relay 为 AI 席位预留虚拟 `player_id`（改动 lobby 分配逻辑、可能破坏稳定 `player_id` 契约）；② AI 席位完全不进 PlayerId 空间、只作 `run_ai` 的 owner 参数（与「席位对称」结论冲突、且 `run_ai` 命令以 owner 署名需 PlayerId）。
- **Q3 `run_ai` 与 `AiScript` 两套来源归位**：联机 session 启用内核 `run_ai`，本地 `melee` 窗口夹具是否保留 `AiScript`。
  - 推荐：联机 session 启用内核 `run_ai`（`SetAi{enabled:true}`）；本地 `melee` 窗口夹具**保留** `AiScript`（`SetAi{enabled:false}`）作开发夹具、不列 1.0 验收线、不改其既有 16 组合回归。
  - 依据：用户 P2 已拍板「内核各端确定性重算」+ C3 两套来源现状 + N1 纯本地单机形态不作验收线；参考缺口②已由 P2 消解。
  - 代价：须显式保证联机 session 不注入 `AiScript`、本地夹具不启用 `run_ai`（`SetAi` 二值互斥），否则两套来源并存双重发命令破坏确定性；保留两套代码面有维护成本。
  - 备选：统一收口到 `run_ai`、删除 `AiScript`（改动面大、破坏 P8 / P9 已归档 16 组合回归与本地开发夹具、与「最小接合 / 不重做」不符）。
- **Q4 最小接合下「打完一局」的胜负判据（本期最关键设计张力）**：`run_ai` 现仅「空闲单位采集」语义、不产生进攻 / `Leave`，一局如何分出胜负。
  - 推荐：冻结开局外部命令夹具（**真人玩家命令流**）→ 各端按相同帧序执行 → 摧毁 AI 席位全部建筑（`R11.1`）→ `resolve_victory` 只结算一次（`R11.4`）→ 逐帧哈希一致 + 回放一致；AI 席位靠 `run_ai` 每帧确定性重算（只作用空闲单位采集），**不注入 `AiScript` 的外部命令 / `Leave`**。该可执行验收已在 `design.md` D4 显式论证并写进 `specs/ai-player-lockstep/spec.md` 的可执行 `#### Scenario:`，MUST NOT 留白。
  - 依据：C3 `run_ai` 语义（`sim_match.rs:3231-3261` 只发 `Gather`）+ `VictoryState::resolve_victory` 既有能力（`WC3-MELEE-RULES.md:236`）+ `R11.1:233` + `R12.4:246` + §0.3 T5「多端确定性重放」。
  - 代价：验收夹具须构造一份能摧毁 AI 全部建筑的冻结真人命令流（不改内核、不注入 AI 脚本）；「最小接合」下 AI 席位行为极简（只采集），胜负完全由真人命令流驱动，不演示 AI 全量行为（登记为后续 change）。
  - 备选：冻结开局外部命令夹具里带 `Leave` 结算命令（`R11.3`）直接判负——但 `Leave` 是 `AiScript` 外部命令语义、与「联机 session 不注入 `AiScript`」冲突，且认输时限属 P3 已拍板不定的 C 类边界 `R11.3`，故不推荐。
- **Q5 AI 决策随机流**：`run_ai` 是否消费主 LCG、是否分独立流、如何进快照 / 哈希 / 回放。
  - 推荐：`run_ai` **不消费主 LCG、不分独立随机流**（现只读 `units` 稳定顺序、经 `validate_gameplay_command` 校验、发 `Gather`，无随机抽取）；AI 决策周期由整数帧 `next_decision_frame` / `decision_period` 驱动，二者已进 v8 快照与哈希。
  - 依据：ADR 0002 §4「每场对局独占一条 `u64` LCG 流、每帧恰好消耗一次」+ 参考对照结论 5「AI 选敌 No random draw is spent、用稳定顺序打破平局」+ C3 `run_ai` 无随机。
  - 代价：后续 AI 全量行为（建造 / 进攻 / 清野）若引入随机，须按 ADR 0002 §4 复核「独立流 vs 主 LCG」并进快照 / 哈希 / 回放；本期登记为后续 change 承接。
  - 备选：为 AI 决策分独立 LCG 流（改动 ADR 0002 §4 主 LCG 契约、`Snapshot` 新增随机流字段 → 触发 Q1 bump 分支与 §8 规格解锁，与「最小接合」不符）。
- **Q6 回放 / 断线重连里 AI 席位状态恢复**：`AiState` 已进快照，重连快照如何恢复 AI 决策状态。
  - 推荐：按「已知限制 / 后续 change」如实登记、**本期不扩范围**；回放按同一外部命令序列 + 同一冻结定义逐帧重放 `run_ai` 即一致；断线重连里 AI 席位决策状态恢复、AI 席位是否算 combatant 属 `R12.5` 观战·断线重连 C 类边界（P3 已拍板本期不定）。
  - 依据：P3 已拍板 C 类边界本期不定 + `R12.5:247` `[待核 / 待拍板]` + C3 `AiState` 已进快照 + 用户第五节 Q6「此项按已知限制 / 后续 change 如实登记、不扩范围」。
  - 代价：本期不覆盖断线重连 / 观战下的 AI 席位恢复，登记为已知限制；真窗口人工清单保持未勾。
  - 备选：本期一并实现 AI 席位断线重连恢复（超出「最小接合」、P3 已拍板不定的 C 类边界，MUST NOT 扩范围）。
- **Q7 规则文档同步（§14 第 5 条）**：本期是否实质同步活文档 `WC3-MELEE-RULES.md`。
  - 推荐：`decision-memo.md`「## 规则文档同步」给出明确结论——本期正是附录 R12 行（`:408`）与 `§0.3 T5:57` 所指「后续 AI / 联机 change」；**「是否本期实质同步活文档」交规格门由人拍板**，本轮（止于成文 + validate）不改活文档正文、不静默改规则（§14 第 2 条），实质同步交规格门拍板后收尾。
  - 依据：C7 + P9 Q9' 已把 `§0.3 T5` 状态措辞 / 附录 R12 行 / §15 / 附录「28」与「p9 在飞」过时措辞 / 主 specs「仍 3」残留 / 「四个·四者」残留登记为「后续独立 change 待办」；MUST NOT 静默改规则、MUST NOT 把 design 引用 `R-x.y` 冒充已实现。
  - 代价：本期不收口这些 residual 措辞（登记为已知限制）；若规格门拍板「本期同步」，须按 §14 由 `docs-owner` 更新活文档 + 附录 R12 行 + §15，并核对不冒充已实现。
  - 备选：本期直接收口全部 residual 文档措辞（超出「最小接合」、涉及活文档正文与多份主 specs、§14 要求先由人拍板，本轮不做；是否本期做交规格门 Q7 拍板，不预设结论）。
- **Q8 Q8' 诚实边界（沿用 C8）**：真窗口 / CI / 跨平台如实标注。
  - 推荐：真窗口 8.x 人工清单保持 `- [ ]`「未执行 / 待人工」、不用离屏 / CPU 回归冒充真窗口验收；CI 双平台 `verify_all.py` 本会话未运行如实标注；v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成；本机 `config.toml` 存在时 GPU 原生崩溃为 HEAD 固有、CI 干净环境全绿。
  - 依据：C8 + `CODEBUDDY.md §5:54,§10` + P9 纪要 Q8'。
  - 代价：本期不产出真窗口验收证据、不声称跨平台达成，如实标注为已知限制 / 待人工。
  - 备选：不适用——把真窗口清单标通过、声称全平台确定性已达成、把 CI 未运行写成全绿均违反 §10 人工边界与诚实边界，是**被否做法**，不列为可选分支（`critic` 清单第 13 项复核）。
- **Q9 规格门锁定流程**：本 change 由「目标与方向已清楚的小改动」豁免规格门吗、如何锁定。
  - 推荐：不豁免——`critic` 规格门报告 → 用户逐条拍板 Q1–Q9 + 批准并锁定 → leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`；本轮 `status: pending`、不 self-approve、不回填、不拿沉默当授权、**不写 `## 省略说明` 豁免**。
  - 依据：`CODEBUDDY.md §8 / §9:106-107` + `tools/verify_all.py::spec_gate_check():207-253` / `spec_artifact_fingerprint():175-195` + `decision-memo.template.md:14,55-62`；本 change 是架构接合、非「目标与方向已清楚的小改动」。
  - 代价：`status: pending` 令 `spec_gate_check()` 判 `invalid spec gate status` 变红是 **fail-closed 正确信号**，全量门禁本阶段不全绿；须走完整规格门才能 `/opsx:apply`。
  - 备选：写 `## 省略说明` 声明小改动豁免 / self-approve / 回填伪造指纹（均违反 §8 / §9 与本 change 性质，MUST NOT；备选栏记为被否做法）。

**人工确认项（仓库文件外证据、列人工确认项、不冒充「用户已确认」）**：
- 用户 2026-11-23 两次 `AskUserQuestion` 明确拍板（P1 / P2 / P3）由本会话用户消息 / leader 转达，无仓库内 artifacts 锚；按 P9 先例列人工确认项。
- `reference-researcher` 参考对照报告为对话内产出、**未落盘**，结论 1–9 / 参考缺口 / 许可证核对由本会话用户消息转达；逐锚全开由 `critic` 清单第 12 项复核（C6 / `design.md`「## 参考项目对照」）。
- CI 双平台 `verify_all.py`、真窗口 8.x 人工清单、v8 Debian 12 逐字节比对：本会话**未执行 / 未验证**，如实标注、不冒充通过（Q8 / C8）。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C8 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项（规格门 Q1–Q9）显式登记为「待规格门由人拍板」，不冒充已拍板、不拿沉默当授权；人工确认项如实标注仓库文件外证据。
3. 不以「预设问题问完 / 问了 N 轮」当收敛；规格门由人拍板，批准记录（`status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`）由 leader 在人确认后回填，本纪要不回填、不 self-approve。

## 规则文档同步

> 本节按 `WC3-MELEE-RULES.md §14` 第 5 条（`:421`）、`openspec/config.yaml:56`（`rules.proposal`）、`openspec/decision-memo.template.md:48-53` 给出非空结论；`tools/verify_all.py::rule_doc_sync_check():256-294` 仅对过规格门 `status: approved` 的玩法 change 做「存在且非空」结构门禁，本 change `status: pending` 豁免，结论真实性由 `critic` 清单第 13 项内容层复核。

**结论：本轮（止于「一次成文四件套 + decision-memo.md + validate 通过」）不同步活文档 `docs/references/WC3-MELEE-RULES.md` 正文；「是否本期实质同步」作为规格门 Q7 交用户拍板、不预设结论、不静默改规则（§14 第 2 条）。依据：**

- **本期正是活文档所指的「后续 AI / 联机 change」**：`§0.3 T5:57`「AI 玩家 × 联机 lockstep 接合（待确认，登记为后续 change）」与附录「规则 → 实现覆盖对照」R12 行（`:408`）「AI 玩家 × 联机 lockstep 接合见 §0.3 T5 待确认、由后续 AI / 联机 change 承接」均指向本 change。本轮若实质同步，须更新 `§0.3 T5` 状态措辞、附录 R12 行实现状态、§15 变更记录，并复核附录「28」capability 与「p9 在飞」过时措辞、主 specs「仍 3」残留、「四个·四者」残留——这些 P9 Q9' 已拍板登记为「后续独立 change 待办」（`openspec/changes/archive/2026-11-23-p9-undead-foundation/decision-memo.md:110,169-174`）。**本期是否收口由规格门 Q7 由人拍板，不预设结论**（用户第五节第 7 条明确「不预设结论」）。
- **MUST NOT 静默改规则（§14 第 2 条）/ MUST NOT 把 design 引用 `R-x.y` 冒充已实现**：`design.md` / `specs/**` / `proposal.md` / `tasks.md` 引用的 `R12.7` / `R12.2` / `R12.4` / `R11.1` / `R11.4` / `R1.1` / `R1.4` / `§0.3 T5` 是需求 / 契约约束引用，不是「该规则已实现」的判定（`WC3-MELEE-RULES.md:393` 三态判定）。本期不新增 / 不重排任何 `R-x.y` 编号、不改状态标记、不冻结 `[待核]` 数值。
- **§13.7 现状 / 参考项目数量不变**：本 change 未触及 §13 命令卡 / 键位现状补记；只读参考项目仍为五个（`UPSTREAM.md:9-15` / `CODEBUDDY.md §1`），未增减。
- **§15 变更记录不改既有行**：按 §15 维护惯例「仅追加本行、不改既有行」，历史记录不因本 change 状态转换改写；是否追加一行由规格门 Q7 拍板后收尾。
- **tasks 承接**：`tasks.md` 含「规则文档同步核对」复选框任务（第 6 组），收尾回答是否需要同步活文档；需同步则更新附录 R12 行与 §15，无需同步则写明依据（`openspec/config.yaml:72`）。
- **Q8' 诚实边界**：真窗口 8.x 人工清单保持「未执行 / 待人工」、不用离屏 / CPU 回归冒充通过；CI 双平台 `verify_all.py` 本会话未运行、如实标注；跨平台仅沿用 ADR 0002 复核记录 Windows / Debian 12 x86_64（v3）已逐字节验证结论，v8 Debian 侧 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成。

**apply 收尾结论（tasks 6.1 / 规格门 Q7 已拍板，2026-11-23）**：Q7 由用户明确拍板「不实质同步活文档 `WC3-MELEE-RULES.md`、按 Q9' 登记已知限制」（见「## 规格门」reason）。据此本期**不修改**活文档正文：不改 `§0.3 T5` 状态措辞、附录 R12 行（`:408`）、§15 变更记录，不动附录「28」「p9 在飞」/ 主 specs「仍 3」/「四个·四者」等 Q9' residual 措辞，全部按 L-3 登记为后续独立 change 待办（§14 第 2 条「不静默改规则」）。`design.md` / `specs/**` / `proposal.md` / `tasks.md` 引用的 `R12.7` / `R12.2` / `R12.4` / `R11.1` / `R11.4` / `R1.1` / `R1.4` / `§0.3 T5` 仅为需求 / 契约约束引用，MUST NOT 作「该规则已实现」判定（`WC3-MELEE-RULES.md:393` 三态判定）；本期未新增 / 重排 `R-x.y` 编号、未改状态标记、未冻结 `[待核]` 数值。真窗口 8.x 清单保持 `- [ ]`「未执行 / 待人工」、未标通过；CI 双平台本会话未运行如实标注。tasks 6.1 据此结论打勾；`critic` 清单第 13 项复核「不静默改规则 / 不把 design 引用 `R-x.y` 冒充已实现 / 真窗口清单未标通过」三项，活文档 diff 为空与 Q7 拍板一致。本结论写在 `decision-memo.md`（不在四件套指纹集合 `spec_artifact_fingerprint()`），不触发 §8 规格解锁、不改四件套正文；上文「交规格门 Q7 由人拍板」为规格门锁定前的历史陈述，按 §8 决策链保留。

## 已知限制 / 边界（如实登记，不冒充已解决、不扩范围）

与 `design.md`「## 已知限制 / 边界」 / `proposal.md` Scope / `tasks.md` 第 8 组逐条对应。

- **L-1（AI 全量行为后续 change）**：S1 / `R12.7` 的建造 / 训练 / 进攻 / 战斗 / 清野 / 结算等 AI 全量行为属 1.0 必做、本期按 P3 已拍板「最小接合」登记为后续 change 承接；本期 `run_ai` 只作用空闲单位采集，不演示全量 AI 行为。
- **L-2（C 类边界本期一律不定）**：`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7` 按 P3 已拍板本期不定、不扩范围；Q6（断线重连 / 观战里 AI 席位恢复、是否算 combatant）按已知限制登记。
- **L-3（Q9' residual 文档）**：活文档 `§0.3 T5` 状态措辞 / 附录 R12 行 / §15 / 附录「28」「p9 在飞」过时措辞 / 主 specs「仍 3」残留 / 「四个·四者」残留按 P9 Q9' 登记为后续独立 change 待办；本期是否收口交规格门 Q7 拍板。
- **L-4（真窗口 / CI / 跨平台）**：真窗口 8.x 保持 `- [ ]`「未执行 / 待人工」；CI 双平台本会话未运行；v8 Debian 12 逐字节比对 / macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成；本机 `config.toml` 存在时 GPU 原生崩溃为 HEAD 固有、CI 干净环境全绿（C8 / Q8）。
- **L-5（参考对照报告未落盘）**：C6 报告为对话内产出、未落盘、列人工确认项，结论 1–9 / 参考缺口 / 许可证核对完整转写进 `design.md`「## 参考项目对照」、承重锚指向 `UPSTREAM.md:98-127` 矩阵条目与参考项目内路径；逐锚全开由 `critic` 清单第 12 项复核。参考缺口 ①②已由用户 P2 拍板消解、按已拍板登记；缺口 ③–⑦如实登记、回到人拍板、不臆造。
- **L-6（规格门 fail-closed）**：`status: pending` 令 `tools/verify_all.py::spec_gate_check()` 判 `invalid spec gate status` 变红是正确信号；本阶段不绕过 / 不改门禁 / 不 self-approve / 不回填指纹 / 不由实现方自行放行；`--fingerprint` 只读重算如实记录、不回填。不写 `## 省略说明` 豁免。
- **L-7（5.2 反向验证 ⑤⑥ 承接，apply 登记）**：tasks 5.2 的退化态 ① AI 命令本地静默注入 / ② 一端不启用 `run_ai` / ③ `AiState` 四处同步 / ④ `run_ai` 内部命令写回 / ⑦ 未冻结开局，以及「胜负非 `Leave` 捷径」专项，已由 `crates/w3-relay/tests/ai_lockstep_reverse_validation.rs` 按 `CODEBUDDY.md §5` 形态①（测试内反例 + 元断言防退化、真经 `apply_frame` / `submit_gameplay_command` / `step` / `restore_snapshot` / `first_difference` / `ResyncReplay` 公开生产路径注入，非旁路模拟、非形态②）覆盖。退化态 ⑤ `HashMap` 迭代 / 墙钟 / `f64` / 独立随机流、⑥ 含建筑误生成 `Move` 经 `w3-relay` 公开 API 无法在不旁路模拟或越界改 `w3-client` 的前提下构造，按 `CODEBUDDY.md §5`「无法构造缺陷态的 MUST 登记理由与替代证据」与 `design.md` D7 边界，由既有 `cargo test -p w3-core --test determinism`（26 项，本期重跑全绿）与既有 R1 可移动性输入翻译回归承接，理由写在该文件模块注释；由 `critic` 清单第 11 项③复核、不冒充已全量覆盖。
- **L-8（胜负结算命令面张力，apply 如实登记）**：现有内核 `resolve_victory`（`sim_match.rs:3211`）仅在 `players[].active` 剩 1 个时结算，`active=false` 唯一来自 `leave()`（`GameplayCommandKind::Leave`），「摧毁建筑」本身不触发结算；`design.md` D4 字面「建筑全毁 → `resolve_victory` 按 `R11.1` 判胜」与该内核事实存在字面张力。批次 1 正向回归（`crates/w3-relay/tests/ai_lockstep_match.rs`）沿用主 `openspec/specs/melee-match/spec.md:98-101`「敌方主城被摧毁 → 经既有 `Leave` / 胜负路径产生唯一胜者」与 P9 / 本地 `MeleeDriver` 先例：帧 9 真人命令流 `Attack` 摧毁 AI 席位全部建筑并断言 `!victory.resolved`，帧 10 才提交既有 `Leave{player:AI_PLAYER}` 结算命令面 → `resolve_victory` 产生唯一胜者（`winner=真人`、`resolved_frame` 进权威状态、只结算一次、重开干净）；该 `Leave` 是冻结真人命令流的既有结算命令面，非 `AiScript` 提前认输 / 直接判负、非本地静默注入；`ai_lockstep_reverse_validation.rs::reverse_validation_leave_shortcut_is_not_a_full_match` 证明「未摧毁建筑提前 `Leave`」会被正向断言链拒绝。这是零内核改动（Q1 保持 8 不 bump）下唯一可落的既有命令面，落在已锁定规格契约内，但 D4 措辞张力由 `critic` 代码门清单第 7 / 10 / 13 项复核、不静默冒充「建筑全毁自动结算」；本期不因此改四件套正文（`w3-core` 零 diff、不触发 §8 解锁）。另：批次 1 harness 保留第二个 `ClientSession`（`player_id=AI_PLAYER`、**只读且恒空输入**，见 `ai_lockstep_match.rs:299-300,342-351,520-523`）以复用既有两端点 checksum 路径；该端点**有真实连接、只是不贡献任何 AI 决策命令帧**，与 Q2「席位对称 / 帧流只承载真人命令」实质口径一致，但与 Q2 字面「AI 无真实连接」存在**测试夹具保真度 + 本纪要措辞 overclaim**（代码门 B-3）：这是测试夹具实现、非产品行为、零生产 diff，`Q2` 字面「无连接」不据本 harness 声称产品层已达成，同交 `critic` 清单第 7 / 10 项复核、按「功能生效仅呈现不足时登记已知限制、不扩范围」惯例登记，不改四件套正文。

## 规格门

- status: approved
- fingerprint: 64532a8c6237c45fd1ec351d33756ba058d14d53f24723d35b68d1e5d8552adf
- approved_by: 用户（2026-11-23 规格门 `AskUserQuestion` 逐条明确拍板「全部按推荐 + 记已知限制 + 批准并锁定」；`critic` 规格门报告 `blocker: 0` 提交后由人拍板，非 AI self-approve、非沉默授权）
- date: 2026-11-23
- reason: `critic` 规格门报告 `blocker: 0`（A 类 0 / B 类 0）；四件套按「内核各端确定性重算 + 最小接合」单一已拍板口径一致、Q1–Q9 四要素齐备且不越界重提范围 / 架构 Q、参考对照三条特别核（`Queue_AI` 不误读 / `ConcordRTS` 自述验收与 RA2 边界 / Warsmash·godot 原创设定不冒充经典）通过、规则文档同步结论真实、机械复核全绿（`openspec validate --strict` / `check_encoding.py` / 指纹 `64532a8c…5adf` 一致 / 主 `openspec/specs/` 零 diff）。用户规格门逐条拍板：**Q1 保持 8 不 bump**、**Q7 不实质同步活文档·按 Q9' 登记已知限制**、**Q2–Q6 / Q8–Q9 全部按推荐**；`critic` residual **R1**（`design.md:169` ADDED Scenario 计数 12→13 笔误）/**R2**（3 个 `vera20k` 参考锚行号精度，`NetworkSendPolicy::Local` 实际在 `live_commands.rs:207`）按用户拍板登记为**已知限制 / 人工确认项**、不改四件套正文；**R4**（拍板轮次措辞）经核对「2 次 `AskUserQuestion` 调用 / 3 条结论（P1 / P2 / P3）」准确、四件套「两次」无矛盾、无需改（系 leader 转述措辞歧义）。三者均不影响行为契约 / 确定性 / 范围 / 验收线，按「功能生效仅呈现不足时登记已知限制、不扩范围」的既有惯例直接推进锁定；四件套正文零改动、指纹保持 `64532a8c…5adf`。R3（参考对照报告未落盘 / 锚未逐锚全开）、R5（附录 R12 行 / §15 按 Q7 结论后续收口）、R6（Q1 / Q5 / Q6 备选联动 §8 解锁）、R7（规格门阶段 `cargo fmt` / `cargo test` / determinism 未运行、如实标注）按 `critic` 报告登记为人工确认项 / 时序提醒，交后续环节。依据 `CODEBUDDY.md §8 / §9:106-107` + Q9（不豁免规格门）。

> 本 change 是架构接合、非「目标与方向已清楚的小改动」；规格门由人拍板批准后规格锁定，由 `tools/verify_all.py::spec_gate_check()` 机械校验本指纹与四件套一致。指纹集合为 `proposal.md` / `design.md` / `specs/**/*.md` / `tasks.md`，**不含 `decision-memo.md`**（`tools/verify_all.py::spec_artifact_fingerprint():175-195`）；补写本纪要小节不触发 §8 规格解锁。本纪要其余小节中「`status: pending`」「须规格门确认锁定」「本阶段不 apply」等表述为规格门锁定**前**的流程状态记录（历史陈述、按 §8 决策链保留），**当前状态以本节 `status: approved` 为准**。规格锁定后实现期若改动四件套任一 artifact 即视为解锁（指纹失配由 `verify_all.py` 判定），MUST 按 §8 同步回查其余三份、由 `spec-steward` 修订后重走规格门。锁定后按 §9 进入 `/opsx:apply`；`push` / `fetch` / `pull` / `clone` 及任何远端同步仍由人工独占。
