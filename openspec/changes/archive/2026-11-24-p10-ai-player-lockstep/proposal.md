# Proposal

## Why

1.0 的唯一对局形态 S1 是「局域网联机对战（含**真人 VS AI 敌人玩家**）」，AI 玩家全部功能属 1.0 必做、回放录像 S2 须逐帧哈希一致（`docs/references/WC3-MELEE-RULES.md:25,26,32`）；但 `§0.3 T5:57` 明确登记「AI 玩家 × 联机 lockstep 接合」架构未定、后续做 AI / 联机 change 时 MUST 先回到人拍板。P0–P9 已归档（P9 双族全量 + bump 8 刚归档），当前无 active change、主 specs 30 份、v8 基线 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`。`R12.7:249` 要求「MUST 复用 `w3-core` 既有 `SetAi` / `run_ai` 命令面产出命令、MUST NOT 旁路写入权威状态」，而 `R12.2:244`（只传玩家输入命令）/ `R12.4:246`（回放逐帧哈希一致）/ `R11.1:233`（摧毁对手全部建筑判胜）/ `R11.4:236`（胜负只结算一次、结算帧进权威状态）尚未在「AI 席位进联机 session」这一形态下被契约化——这是 1.0 头号架构缺口。用户 2026-11-23 两次 `AskUserQuestion` 已把方向、决策侧与范围水位提前拍板（见 `decision-memo.md` P1 / P2 / P3），故按 `CODEBUDDY.md §8` 站稳承重约束与拍板决策后一次成文。相关 ADR：0001（资产 / 法律）、0002（确定性硬门禁）、0003（范围 / 分期）、0004（冻结边界）、0005（protobuf 载荷 + W3N1）。

## What Changes

- **方向 =「AI 玩家 × 联机 lockstep 接合」（用户 P1 已拍板）**：承接 `§0.3 T5` / `R12.7` / S1 / S2，把「AI 席位」接进联机 lockstep session 并打完一局 1v1（真人 VS AI）。
- **决策侧与命令注入 =「内核各端确定性重算」（用户 P2 已拍板，选推荐）**：启用 `w3-core` 既有 `run_ai`；AI 决策状态为**权威状态**、复用既有 `AiState` / `SetAi` 命令面（`gameplay.rs:1286,1355,1986,2028-2031`、`sim_match.rs:737-738,2480-2483,3231-3261` 已进 FNV-1a 状态哈希与 v8 快照）；联机帧流**只承载真人玩家命令**，各端以相同开局冻结定义 / 相同外部命令流构造 `RtsMatch`、由 `run_ai` 每帧确定性重算，靠逐帧状态哈希一致；MUST NOT 旁路写权威状态、MUST NOT 把 AI 决策结果作为独立命令注入 `CommandBatch`（`session.rs:92-93` 明确禁止本地静默 `submit_gameplay_command` 否则 desync）。**被否**：①「外部 `AiScript` 经 relay 广播」②「双模式可切换」——两者 MUST NOT 静默采用，完整否决依据见 `decision-memo.md`「## 被否方向台账」R1 / R2。
- **本期范围 =「最小接合」（用户 P3 已拍板，选推荐）**：只把 AI 席位接进联机 lockstep session 并打完一局 1v1、逐帧哈希一致 + 回放一致。**AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）登记为后续 change 承接**；**C 类待拍板边界**（`R3.5` 资源共享 / `R5.5` 高地加成 / `R11.3` 认输时限 / `R12.5` 观战·断线重连 / `R13.1` grid / `R13.5` 界面键 / `R13.7` 键位别名）本期一律**不定、不扩范围**，登记为已知限制 / 后续 change。**被否**：①「全量 AI 行为本期做完」②「最小接合 + 本期定 C 类边界」——见 `decision-memo.md` R3 / R4。
- **「打完一局」胜负判据 MUST 可执行、不留白**：`run_ai` 现仅「空闲单位采集」语义（`sim_match.rs:3231-3261` 只发 `Gather`、不产生进攻 / `Leave`），故本期用**冻结开局外部命令夹具（真人玩家命令流）→ 摧毁 AI 席位全部建筑 → `resolve_victory` 只结算一次**（`R11.1` / `R11.4`）的可执行验收，各端逐帧 `compute_state_hash()` 一致、`first_difference()` 为 `None`、回放逐帧一致（`R12.4`）；**不注入 `AiScript` 的外部命令 / `Leave`**。完整论证见 `design.md` D4、可执行 `#### Scenario:` 见 `specs/ai-player-lockstep/spec.md`。
- **四件套按单一已拍板口径一次成文**：`design.md` D1–D7 不写「可容纳两种结果」、`tasks.md` 不列两分支、本 Scope 不写待选；架构 / 范围 Q 已由用户 P2 / P3 拍板，**规格门 Q1–Q9 只承载「已拍板架构下的实现层处置」**（版本与权威状态、AI 席位 PlayerId 归属、两套来源归位、胜负判据、随机流、回放恢复、规则文档同步、诚实边界、锁定流程），逐条四要素见 `decision-memo.md`「## 待确认项」，MUST NOT 重新提出范围 / 架构 Q。
- **一个 change 一个 owner**：提案阶段仅 `spec-steward` 写 `openspec/**`；实施期按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。本阶段不改 `crates/**`、不改活文档 `WC3-MELEE-RULES.md` 正文、不改 ADR 正文、不改 `openspec/config.yaml` / `CODEBUDDY.md`、不改主 `openspec/specs/**` 正文（归档才合并 delta）。

## Capabilities

### New Capabilities

- `ai-player-lockstep`: AI 玩家席位 × 联机 lockstep 接合的确定性契约——内核 `run_ai` 各端重算、AI 决策状态为权威状态（复用既有 `AiState`、进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 8）定义的快照）、联机帧流只承载真人命令、席位对称（同一 PlayerId 命名空间）、`submit_input` 禁止本地静默注入、打完一局 1v1 的可执行胜负判据（`R11.1` / `R11.4`）、逐帧哈希一致 + 回放一致（`R12.2` / `R12.4`）、R1 可移动性过滤、确定性与反向验证、范围边界（AI 全量行为 / C 类边界 / N1–N6 登记为后续 change 或不做）。

### Modified Capabilities

- `lockstep-networking`: 「Transmit only input commands」Requirement 补充「AI 决策**不**作为命令注入联机帧流、各端由内核 `run_ai` 确定性重算、`submit_input` MUST NOT 本地静默注入」的确定性边界（其余 8 条 Requirement 不变）。
- `gameplay-systems`: 「Deterministic AI command source」Requirement 补充「联机形态下 AI 决策状态为权威状态、`run_ai` 内部命令不写回命令队列、不注入重放、各端重算逐帧哈希一致」（其余 3 条 Requirement 不变）。
- `melee-match`: 「确定性 AI 对手」Requirement 补充「联机 session 启用内核 `run_ai`、本地 `melee` 窗口 `AiScript` 夹具保留但不列 1.0 验收线、打完一局胜负判据按 `R11.1` 摧毁全部建筑」（其余 8 条 Requirement 不变）。
- `deterministic-simulation`: 「State hashing and snapshot」Requirement 补充「AI 决策状态（`AiState`）已进 FNV-1a 状态哈希与 v8 快照、本期零新增权威状态字段的推荐口径由规格门 Q1 拍板」（其余 3 条 Requirement 不变）。

> delta 数控制：**1 ADDED + 4 MODIFIED = 5 份**，低于 `Consider splitting changes with more than 10 deltas` 上限（沿用 P9 R30 先例）。MODIFIED delta 逐字保留主 spec 历史 `### Requirement:` / `#### Scenario:` 身份键、口径改在 THEN 与 requirement 新正文、每条带 `>` 身份键说明、不写 `## Purpose` 覆盖 delta、delta 顶部不写说明块。其余主 specs（含历史「`SNAPSHOT_VERSION` 仍 3」口径残留）按 P9 Q9' 已拍板登记为已知限制、由 `docs-owner` 后续独立 change 收口，本阶段不改主 `openspec/specs/**` 正文。

## Impact

- **代码面 / owner（实施期、非本阶段）**：`client-owner`（`crates/w3-client/` 联机 session 装配、`AiScript` 本地夹具归位）与 `net-owner`（`crates/w3-net/` / `crates/w3-relay/` 席位语义 / `submit_input` / lobby `player_id` 归属）为主接缝；`sim-owner`（`crates/w3-core/` 复用既有 `run_ai` / `AiState` / `resolve_victory`，零新增权威状态字段为 Q1 推荐口径）配套回归；`docs-owner` 按规格门 Q7 结论收尾活文档 / ADR 复核记录。按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。本阶段只产出 `openspec/changes/p10-ai-player-lockstep/` 下的文档、不改 `crates/**` 任何文件。
- **确定性契约（ADR 0002）**：本期按 P2 已拍板复用既有 `AiState`（`enabled` / `next_decision_frame` / `decision_period` / `pending_tasks.len()` 已进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`，`gameplay.rs:2028-2031`）；**Q1 推荐「零新增权威状态字段 → 保持 8 不 bump」**（P8.1「定义层变化不 bump」/ P9 R19 先例），若规格门 Q1 拍板确需新增字段则按 ADR 0002 全套 bump 8→9（`compute_state_hash` / `Snapshot` 顶层字段 / `first_difference()` / `restore_snapshot` 四处同步、golden hash 按新实现如实重取、proto 只追加 tag 59 起、`reserved 14 to 32` 保持、旧 v8 明确拒绝、ADR 0002 / 0005 复核追加）。命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、30 Hz 整数帧、稳定 `EntityId` 顺序、`SimFixed` Q47.16、FNV-1a、每帧一次主 LCG 一律不变；`w3-core` 不得依赖 `w3-content` / render / ui / audio / net，表现层不得反写模拟状态，`w3-content` 原始 `f64` 坐标不进哈希 / 快照。`cargo test -p w3-core --test determinism`（26 项）重跑、新增 / 修改回归按 `CODEBUDDY.md §5` 形态①做反向验证（改坏验红）。
- **联机 / 回放（R12.2 / R12.4 / ADR 0005 §3-4）**：只传玩家输入命令、MUST NOT 传游戏状态；W3N1 25 字节帧头 / `FrameScheduler` / `stabilize` / `apply_frame` / 断线重连游标不变；`Frame` / `Checksum` / `Welcome` / `Reject` / `ResyncReplay` 处理路径不变；回放按同一命令序列 + 同一冻结定义哈希 / 内容哈希 + v8 快照逐帧一致。AI 席位无真实连接、不贡献命令帧，PlayerId 归属 / 是否算 combatant 由规格门 Q2 拍板。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产，本期不改 `FrozenDefinitions` / 冻结定义、不读 `config.toml` 或本机导出数据、不运行 `tools/mpq-extract|slk-export|map-parse|asset-convert` 导出 / 构建脚本；`AiScript` / `run_ai` 命令面全为既有合成原创编译期整数、`[待核]` 数值不采信不冻结不入库。参考项目只读、不复制代码（详见 `design.md`「## 参考项目对照」许可证核对）；`WarsmashModEngine`（AGPLv3）仅借语义、`vera20k`（GPLv3 + Section 7）按固定提交 `305ea3a75` 零复制、`ConcordRTS`（GPL-3.0-only）RA2/YR 专属规则不作 WC3 事实、MIT godot 借结构不借实现、`rts_kernel_cs` 的 `double` 路线不采用；预判零新增依赖。
- **阶段与 ADR 编号**：路线图阶段 **P10**（P0–P9 已归档、承接 `docs/roadmap/P8-human-orc-melee.md:3,161` 与 `WC3-MELEE-RULES.md §0.3 T5` 所指后续 AI / 联机 change；`openspec/config.yaml` 的 `rules.proposal`「P0–P5」为历史措辞，此处按项目实际阶段标注，沿用 P9 先例）。相关 ADR：0001 / 0002 §3-8 / 0003 / 0004 / 0005 §3-4。
- **规格门与人工边界**：四件套 + `decision-memo.md` 按单一已拍板口径一次成文后 → `openspec validate p10-ai-player-lockstep --strict` / `check_encoding.py` / `verify_all.py --fingerprint`（只读重算、不回填）→ `critic` 规格门报告 → 用户逐条拍板 Q1–Q9 并确认锁定 → leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`。本轮 `status: pending`、不 self-approve、不回填指纹、不拿沉默当授权、不写 `## 省略说明` 豁免。本阶段不 apply、不 commit、不 archive、不 push / fetch / pull / clone。

## Scope

- **做（P2 / P3 已由用户 2026-11-23 明确拍板生效，非 AI 推荐、非静默窄化）**：① 按「内核各端确定性重算」接合——启用 `w3-core` 既有 `run_ai`、复用既有 `AiState` / `SetAi`、联机帧流只承载真人玩家命令、各端相同开局冻结定义 / 相同外部命令流、`submit_input` 禁止本地静默注入、AI 席位与真人席位对称（同一 PlayerId 命名空间）；② 按「最小接合」交付——AI 席位进联机 lockstep session、打完一局 1v1（真人 VS AI），胜负判据按冻结真人命令流摧毁 AI 席位全部建筑 → `resolve_victory` 只结算一次（`R11.1` / `R11.4`）且可执行、逐帧 `compute_state_hash()` 一致 + `first_difference()` 为 `None` + 回放逐帧一致（`R12.4`）、重开干净（`R11.5`）；③ R1 可移动性约束（金矿 / 树木 / 建筑 / 主城 MUST NOT 出现在 `Move` 命令 Scenario）；④ 确定性硬门禁（`determinism` 26 项重跑 + 反向验证形态① + `cargo fmt --all -- --check` + `check_encoding.py`）;⑤ 1 ADDED + 4 MODIFIED = 5 份 delta、其余登记已知限制；⑥ `openspec validate --strict` / `check_encoding.py` / `verify_all.py --fingerprint`（`status: pending` 期 fail-closed 为正确信号、不绕过不改门禁）；⑦ 真窗口人工验收清单 8.x 保持 `- [ ]`「未执行 / 待人工」。
- **不做（已拍板后的如实登记、非待拍板项、不静默窄化）**：**AI 全量行为**（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）本期做完、`最小接合 + 本期定 C 类边界`（R3 / R4 已否）；**C 类边界本期一律不定**（`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7`）；**外部 `AiScript` 经 relay 广播 / 双模式可切换**（R1 / R2 已否）；N1–N6（纯本地单机对局形态 N1 / 地图编辑器 / RPG 自定义地图 / 剧情战役 / JASS VM / 资产入库）、联机底座重做（P4 已交付）、AI 决策作为独立命令注入 lockstep 帧流、`AiScript` 的 `Leave` 结算注入联机 session、`f64` / 墙钟 / 独立随机流入权威状态；不静默改活文档 `WC3-MELEE-RULES.md`（§14 第 2 条、是否本期同步交规格门 Q7 拍板）、不改 ADR 正文 / 主 `openspec/specs/**` 正文 / `config.yaml` / CI / `crates/**`（提案阶段）。
- **跨平台确定性口径（Q8 / C8 已拍板）**：仅按 ADR 0002 复核记录沿用「Windows 11 与 Debian 12 x86_64 / rustc 1.99.1（v3）已逐字节验证」；v4–v8 Debian 12 侧逐字节比对 / macOS / ARM64 / 其它 rustc 待验证，**不得据本 change 声称全平台确定性已达成或有新验证**；CI 双平台 `verify_all.py` 本会话未运行、如实标注；真窗口 8.x 清单不冒充通过。
- **本阶段交付边界**：仅产出 `openspec/changes/p10-ai-player-lockstep/` 下 `proposal.md` / `specs/**`（1 ADDED + 4 MODIFIED）/ `design.md` / `tasks.md` + `decision-memo.md`；主 `openspec/specs/**` **不变化**（归档才合并 delta）。四件套按用户已拍板口径收敛为「内核各端确定性重算 + 最小接合」单一路径一次成文、不写「可容纳两种结果」、不列两分支；`decision-memo.md`「## 拍板决策」只记已拍板项、MUST NOT 写成「AI 推荐」、不 self-approve、不回填指纹、`status: pending` 保持。若规格门出现 blocker MUST 按 `CODEBUDDY.md §8` 同步回查四件套、由 `spec-steward` 修订后重走规格门，不静默沿用、不冒充已锁定。
