# 决策纪要 — `p8-dual-faction`

> 取料纪律：基于仓库 artifacts、只读参考项目对照报告与用户 2026-11-17 本会话拍板回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md` §6）。
> 本 change 是 P8 路线图 `docs/roadmap/P8-human-orc-melee.md` §3 的 **P8.3「双族对抗与 AI」** 立项，对称于已归档 `2026-11-16-p8-human-faction`（P8.1）/ `2026-11-17-p8-orc-faction`（P8.2），站在两族分段定义、按族只读分派、`p9-command-hotkeys`（R13 操作层）/ `runtime-entity-movability`（R1.4 分层判据）与外部帧冻结 `AiScript` 已归档结论之上、不重开。`reference-researcher` 已按 `UPSTREAM.md` 矩阵 T1 / T4 / T5（T2 一句）出「参考对照报告」，结论与锚由 `spec-steward` 逐条打开复核（含三处诚实判断、归因纠正、参考缺口 G1 / G3 / G6 / G7），见 `design.md`「## 参考项目对照」。
> **证据锚写法**：本轮 `w3-client` 代码会被 owner 改动，本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目，**不写精确 `:NN` 行号**（P8.1 行号漂移返工教训）；参考项目为只读固定提交、其 `file:line` 锚不漂移、按对照报告原样保留。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `crates/w3-core/` 零 diff：不新增权威状态字段（`Player.race` / `RtsMatch.race` 等）/ `CommandKind` / `GameplayCommandKind` / `EffectKind` 变体 / 事件；本 change **不加新 `UnitDef` / `AbilityDef` / `UpgradeDef`**，`defs_hash` / `content_hash` 也不变；`SNAPSHOT_VERSION` 仍 3；状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 稳定 `EntityId` 顺序 / FNV-1a / golden hash 逐字节不变；`git diff -- crates/w3-core/` 为空 | `crates/w3-core/src/snapshot.rs::SNAPSHOT_VERSION` / `crates/w3-core/src/command.rs::CommandKind::Spawn` / `crates/w3-core/src/gameplay.rs::GameplayCommandKind` / `crates/w3-core/src/gameplay.rs::compute_defs_hash` / `crates/w3-core/src/gameplay.rs::FrozenDefinitions::content_hash` / `crates/w3-core/src/gameplay.rs::first_difference` / `docs/decisions/0002-determinism.md` §6-8（ADR 0002）/ `CODEBUDDY.md §5` / 主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」/ `proposal.md` Impact / `design.md` D4 |
| C2 | 双族参数落装配层、不新增权威状态：新增 `with_races(local_race, ai_race)` 双参数入口把本地 / AI 种族解耦；**保留 `with_race(race)` 为 `with_races(race, race.opponent())` 兼容别名**，`MeleeSetup::new()`（本地人族 / AI 兽族）与既有入口行为逐字节不变、`Spawn` 命令字节逐字节不变（两族 / 四组合装配字节一致）、`race_does_not_change_authoritative_spawn_commands` 回落测试仍绿；`MeleeSetup` 增 `ai_race` 一类**装配层只读**字段（`race` 继续作本地族别名、不破坏现有 `.race` 断言）；`def_ids` 按席位装配、种族只停装配层 `EntityId→DefId` 关联，MUST NOT 进权威状态 / 哈希 / 快照 | `crates/w3-client/src/melee/setup.rs::MeleeSetup::with_race` / `crates/w3-client/src/melee/setup.rs::MeleeSetup::with_races` / `crates/w3-client/src/melee/setup.rs::Builder::spawn` / `crates/w3-client/src/melee/setup.rs::race_does_not_change_authoritative_spawn_commands` / `crates/w3-client/src/melee/defs.rs::Race::opponent` / `crates/w3-core/src/command.rs::CommandKind::Spawn`（不含 DefId）/ 主 `openspec/specs/melee-match/spec.md` / `proposal.md` Impact / `design.md` D1 |
| C3 | AI 命令源按 AI 自身种族产出：`AiScript::new` 现状用 `setup.race`（本地族）+ 本地族 `category_def_ids()` 生成 `AI_PLAYER` 的命令、与按 `opponent()` 段装配的 AI 实体错配；本 change MUST 改为按 **`ai_race`** 取 `category_def_ids`（训练目标）/ `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade`，使 AI 训练 / 施法 / 研发的 DefId 落在 AI 族段；`category_def_ids` 需区分本地 / AI 两份（本地验收用本地份、AI 脚本用 AI 份）；仍是外部帧冻结命令（`SetAi { enabled: false }` 关内核 `run_ai`）、按 `(execute_frame, player_id, sequence)` 稳定排序、不新增权威状态 | `crates/w3-client/src/melee/ai.rs::AiScript::new` / `crates/w3-client/src/melee/setup.rs::MeleeSetup::category_def_ids` / `crates/w3-client/src/melee/defs.rs::Race` / `crates/w3-client/src/melee/driver.rs::MeleeDriver::new` / `docs/references/WC3-MELEE-RULES.md` R12.7 / R2 / `design.md` D2 |
| C4 | 四组合 CPU 端到端 + 确定性闭环：以 `(local_race, ai_race)` 参数化 2×2 = 人vs人 / 人vs兽 / 兽vs人 / 兽vs兽 四种组合，各能装配并打完一局（采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁敌方主城结算 → 重开），`resolve_victory` 只结算一次、重开无残留、`SNAPSHOT_VERSION == 3`（R11.1 / R11.4 / R11.5）。**验收口径（关键）**：断言「**同一组合两实例逐帧 `compute_state_hash()` / `capture_snapshot()` 字节一致**」——双族参数只停装配层、四组合权威 `Spawn` 初值相同；既有 `both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` MUST 仍绿、不静默改其断言；**不得**断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」（本 change 不加新定义） | `crates/w3-client/tests/melee_loop.rs::play_full_match` / `crates/w3-client/tests/melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` / `crates/w3-client/tests/melee_loop.rs::race_fallback_matches_human_segment_in_full_match` / `crates/w3-core/src/sim_match.rs::resolve_victory` / `crates/w3-core/src/sim_match.rs::compute_state_hash` / 主 `openspec/specs/melee-match/spec.md` / `docs/references/WC3-MELEE-RULES.md` R12.4 / R11 / `docs/decisions/0002-determinism.md` / `design.md` D3 |
| C5 | 反向验证（改坏验红，`CODEBUDDY.md` §5）：新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过，用「测试内反例元断言」或 A-B 对照（**对称复用** `ai.rs::reverse_validation_orc_dispatch_formal_passes_the_assertion` / `reverse_validation_orc_dispatch_degenerate_fails_the_same_assertion` / `ai.rs::placeholder_catalog()`、`defs.rs::reverse_validation_defs_hash_*`、`tests/movability.rs::degenerate_filter` 先例）。至少覆盖：①「AI 命令族退回本地族」→ 四组合中至少 2 组（人vs兽 / 兽vs人、以及同族组合若命令 ID 不同）的「AI 训练 / 施法 / 研发 DefId 落在 AI 族段」断言失败；②「四组合装配退化为单参数 `opponent()`」→「人vs人 / 兽vs兽」两组 `def_ids` 段位断言失败；③「`with_race` 兼容别名退化」→ 既有回落基线测试失败。退化实现真经 `AiScript::new` / `MeleeSetup::with_races` / apply 路径注入、非旁路模拟；仅「全绿」不算完成 | `CODEBUDDY.md §5` / `openspec/config.yaml` 的 `rules.tasks` 第三条 / 已归档 `2026-11-17-p8-orc-faction` 的 `ai.rs::reverse_validation_orc_dispatch_*` / 已归档 `2026-11-16-p8-human-faction` 的 `defs.rs::reverse_validation_defs_hash_*` / 已归档 `2026-11-15-p8-faction-foundation` 的 `tests/movability.rs::degenerate_filter` / `design.md` D5 / `tasks.md` 6.x |
| C6 | 零资产 / 数值 `[待核]` / 不冒充经典事实：本 change 不加新定义、不复制 Warsmash rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 abilId；两族「更肉 / 更快」等差异是既有 `[取舍]` 合成原创常量相对大小、非经典事实；零资产 / 零文件 IO / 零 `config.toml` 或导出 JSON 读取 / 零新增依赖 | `docs/decisions/0001-license.md`（ADR 0001）/ `docs/decisions/0004-frozen-coordinates.md`（ADR 0004）/ `CODEBUDDY.md §6` / `docs/references/WC3-MELEE-RULES.md` R2 / §14 / `docs/references/UPSTREAM.md` 矩阵条目 T4 / T5 边界 / `godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md:3` / `design.md` D7 |
| C7 | 1.0 范围：落在 S1（局域网联机对战，含真人 VS AI 敌人玩家）/ S3（4 种族做完）的**过程分期**（P8.3 两族 CPU 闭环、P9+ 补不死 / 暗夜收敛四族）、不新增范围编号；AI 玩家全功能属 1.0 必做（R12.7）；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项；N1–N6 不纳入（尤其 N5 JASS / wtg VM、N6 资产入库）。**范围边界（防过度承诺）**：路线图 §3 字面「玩家可选族（入口参数 / **HUD**）」中的 HUD 选族可见 / 选族 UI 属 **P8.4**，本 change 只做「入口参数」层；路线图字面「`run_ai` 按种族」在本项目实际是**外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`**（架构取舍、不写成启用内核 `run_ai`）；**AI × lockstep 命令接合（`WC3-MELEE-RULES.md` §0.3 T5 待确认项）不属本 change**、由后续独立 change 承接 | `docs/references/WC3-MELEE-RULES.md` §0（§0.1 S1–S6 / §0.2 N1–N6 / AI 玩家口径 / §0.3 T5）/ R12.7 / `docs/roadmap/P8-human-orc-melee.md` §3（P8.3 / P8.4）/ §6 第 4 条 / `CODEBUDDY.md §1` / `crates/w3-client/src/melee/ai.rs` / `crates/w3-client/src/melee/driver.rs` / `proposal.md` Why / Impact / `design.md` D8 |
| C8 | 单 change 不并后续：HUD / 表现层模型键 / 选族 UI（P8.4）、P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、不死暗夜（P9+）、中文上屏、运行时新实体真实模型绑定（模型缺键回退已知限制）、真窗口人工验收、AI × lockstep 接合（T5）均由后续独立 change 承接，本 change 不扩范围、不冒充已解决 | `docs/roadmap/P8-human-orc-melee.md` §3 / §6 / 已归档 `2026-11-17-p8-orc-faction/decision-memo.md`「归档说明」/ `design.md` Goals / Non-Goals / Risks / `tasks.md` Workflow follow-up |

**归因声明**：C1 的「新增权威状态字段须同步进哈希 / 快照 / `first_difference()`」硬门禁出处是**本项目** ADR 0002 §6-8 + `CODEBUDDY.md §5` + 主 `melee-match/spec.md`「内核零改动与确定性契约」，**不是 `vera20k`**；不得冒充参考项目结论（见 `design.md`「参考项目对照」T1 归因纠正）。C6 的「两族数值差异不是经典事实」是本项目 `WC3-MELEE-RULES.md` R2 `[已核]` 规则性事实 + `[取舍]` 合成原创常量，四参考项目均无该数值 / 结论；来自参考项目边界的锚指向 `UPSTREAM.md` 矩阵条目 T4 / T5 或参考项目内可核验路径（如 `CLASSIC_RULES_BASELINE.md:3`），不指向参考项目路径冒充经典数值。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

## 拍板决策（含被否备选与理由）

- **P1 用户 2026-11-17 本会话指令：归档并继续开发 P8.3 双族对抗**。用户在本会话明确指令「我已执行 git push 完毕。GitHub Action 执行成功（run 44029882988）。现在请你归档并继续开发 P8.3 双族对抗」，是**人对方向（P8.3 双族对抗）与实施的明确拍板**、等价于规格门批准（与 P8.1「用户指令作为批准」、P8.2 / `p9-command-hotkeys`「指令 + 方案拍板」先例一致）：`## 规格门` 直接写 `status: approved`、不 `pending`、不 self-approve 措辞、不再向人请求批准。证据形态：**本会话用户消息（非仓库文件，由人工核对）**——依 `openspec/decision-memo.template.md` 三分判据第 3 类，仓库内无可核验 `file:line` 锚，**不冒充已落库文档**。**用户拍板的是方向与放行、不是逐条技术细节确认**；方向落地锚 `docs/roadmap/P8-human-orc-melee.md` §3（P8.3 水位 / 四组合验收）/ §6 第 4 条（P8.3 在两族 CPU 闭环都绿之后开）/ `docs/references/WC3-MELEE-RULES.md` §0.1（S1 / S3）。被否：把 HUD 选族 UI / P8.5 深度玩法 / 联机底座 / 工具链调参 / AI × lockstep T5 塞进本 change（与「别扯不重要的事」/ C7 / C8 冲突）。
- **P2（AI 推荐、随批准采纳进 apply，MUST NOT 写成用户已逐条确认）P8.3 技术方案 `design.md` D1–D8**：双族独立参数解耦（`with_races` + 保留 `with_race` 兼容别名 + `ai_race` 装配层只读字段、不新增 `Player.race`）+ `AiScript` 改用 AI 自身种族取训练 / 施法 / 研发目标（`category_def_ids` 区分本地 / AI 两份、仍是外部帧冻结 `AiScript` + `SetAi { enabled: false }`）+ 四组合 2×2 CPU 端到端一局（验收口径「组内两实例逐帧哈希一致、既有两族基线不破坏、不断言跨组合 / 旧基线数值相等」）+ 反向验证改坏验红；`w3-core` 零 diff、不加新定义则 `defs_hash` 不变。锚 `design.md` Decisions / `specs/dual-faction/spec.md` / `specs/melee-match/spec.md` / `tasks.md`。**技术方案为 AI 推荐，随用户对方向的批准采纳进入 apply，不是用户逐条技术细节确认**；被否备选见 R1–R10 与 `design.md` 各决策「备选方案」。参考缺口 G6 / G7（四项目均无「独立双族参数 + 四组合逐帧哈希验收」与「外部帧冻结 `AiScript` 按种族查表」精确做法）方向已随 P1 放行，但技术取舍如实标注为 AI 推荐、不冒充用户已逐条确认或参考结论。
- **P3（AI 推荐、随批准采纳进 apply，MUST NOT 写成用户已逐条确认）Capability 划分**：新增 `dual-faction`（双族对战装配与 AI 命令分派、四组合确定性闭环），`melee-match` 增加 1 条 ADDED Requirement（四组合会话装配与逐帧哈希一致）；既有 `melee-match` 8 条 requirement 不改写、`unit-orders` 不改写（R1 分层判据沿用、本 change 不碰输入层）；两 capability 与主 `openspec/specs/` 20 个 capability 均不重名。锚 `proposal.md` Capabilities / `design.md` D1 / D3 / D6。状态同 P2。

## 被否方向台账

- **R1** 改 `w3-core` 加 `Player.race` / `RtsMatch.race`（或任何运行时权威状态字段）承载双族 → 否决：C1 / ADR 0002 §6-8；新增权威状态须同步进状态哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`，违反主 `melee-match/spec.md`「内核零改动与确定性契约」。锚 `design.md` D1 备选 1 / D4。
- **R2** 给 `CommandKind::Spawn` 加 `DefId` 字段以承载种族 → 否决：C2；改 `w3-core` 命令线格式 / 状态哈希 / 快照，两族 / 四组合 `Spawn` 字节不再一致，违反内核零 diff。种族只经装配层 `def_ids` 关联。锚 `design.md` D1 备选 2 / `crates/w3-core/src/command.rs::CommandKind::Spawn`。
- **R3** 删 `with_race` 兼容别名、破坏 `MeleeSetup::new()` 与既有入口 / 两族基线 / 回落测试 → 否决：C2 / C4；既有入口行为逐字节不变、回落测试仍绿是本 change 的回归证据。锚 `design.md` D1 备选 3 / `melee_loop.rs::race_fallback_matches_human_segment_in_full_match`。
- **R4** AI 命令继续用本地族（`AiScript::new` 保持 `setup.race` + 本地族 `category_def_ids()`），或让 AI 实体改用本地族段装配以「对齐」→ 否决：前者保留错配、四组合必然暴露；后者把「AI 可选另一族」退化成两族恒同族、违背 R2 / 路线图 §3。锚 `design.md` D2 / C3。
- **R5** 启用内核 `run_ai` 按种族产出命令（贴合路线图 §3 字面「`run_ai` 按种族」），或借本 change 做 AI × lockstep 命令接合（§0.3 T5）→ 否决：C7 / ADR 0002 / ADR 0005；本项目 AI 命令源是外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`，T5 是明确待确认项、须回到人拍板、由后续独立 change 承接。锚 `design.md` D8 / `WC3-MELEE-RULES.md` §0.3 T5。
- **R6** 把 HUD 选族 UI / 选族可见 / 表现层模型键（P8.4）、P8.5 深度玩法、不死暗夜（P9+）、中文上屏、真窗口人工验收塞进本 change → 否决：C7 / C8 / `docs/roadmap/P8-human-orc-melee.md` §3（HUD 属 P8.4）/ §6 第 4 条串行推进。锚 `design.md` Goals / Non-Goals / D7 / D8 / Risks。
- **R7** 按「参考项目已有四组合验收」臆造对照：`godot_warcraft3` 对手种族硬编码 human（`match_bootstrap_module.gd:157`）、`ACCEPTANCE.md:24` 是**已降级**四族 10 配对历史规划（`:3` 明确不再作为 MVP 门槛）、`game_session.gd:12` 单字段；四项目均无「独立双族参数 + 四组合 2×2 逐帧哈希验收」（G6）与「外部帧冻结 `AiScript` 按种族查表」（G7）→ 否决：如实登记参考缺口、不臆造、不冒充参考结论。锚 `design.md`「参考项目对照」T5 诚实判断① / T4 诚实判断② / G6 / G7。
- **R8** 新增 `EffectKind` / `CommandKind` / `GameplayCommandKind` 变体 / 新 `UnitDef` / `AbilityDef` / `UpgradeDef` / JASS / wtg VM（N5）承载双族，或升 `SNAPSHOT_VERSION` 到 4「容纳」双族参数 → 否决：C1 / C6 / N5 / ADR 0002；双族参数不进权威状态、本 change 不加新定义、快照头部布局不变，升版本是改内核权威契约。锚 `design.md` D4 / D7。
- **R9** 断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」，或静默改写既有两族基线 / 回落测试断言以「迁就」新参数 → 否决：C4 / ADR 0002 / `design.md` D3 备选 1–3；四组合各自 AI 命令落在 `ai_race` 段、字节本就不同，跨组合数值相等不是本 change 契约；本 change 不加新定义、`defs_hash` 不变、无需调整旧基线；既有基线是「种族不进权威状态」的回归证据。锚 `design.md` D3 / `tasks.md` 3.3。
- **R10** 照搬 Warsmash rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 abilId，或把「兽族更肉 / 更快」标成 `[已核]` 经典事实冻结进权威定义 → 否决：C6 / ADR 0001 / §14 活文档流程；四参考项目都无该数值 / 结论（`CLASSIC_RULES_BASELINE.md:3`），AGPLv3 仅借鉴语义不复制代码。锚 `design.md` D7 / `UPSTREAM.md` 矩阵条目 T4 / T5 边界。

## 待确认项（如实标注，不冒充已拍板）

- **规格门批准** → **已由用户 2026-11-17 本会话指令拍板放行（见「规格门」小节与 P1）**：是人对方向与放行的明确拍板、等价规格门批准，`status: approved`、真值指纹回填，非 pending / 非 self-approve / 非沉默当授权。**用户拍板的是方向与放行、不是逐条技术细节确认**；P2 / P3 技术方案为 AI 推荐、随批准采纳进 apply，不写成用户已逐条确认。
- **AI × lockstep T5** → `WC3-MELEE-RULES.md` §0.3 T5 待确认项：AI 决策在哪一侧计算、命令如何注入 lockstep 帧流、如何保证多端确定性重放，架构未定；本 change 明确不承接、由后续独立 change 回到人拍板（遵守 ADR 0002 / ADR 0005）。锚 `WC3-MELEE-RULES.md` §0.3 T5 / `proposal.md` Impact / `design.md` D8。
- **真窗口人工验收未执行 / 待补验** → 本轮 CPU 侧以回归 + 门禁自证，真窗口 / GPU / egui overlay 一律未人工观察，如实标注「未执行 / 待补验」、不补测、不冒充通过（沿用 P8.1 / P8.2 / `p9-command-hotkeys` 归档说明先例）。锚 `tasks.md` 7.1 / `design.md` Risks。
- **HUD 选族属 P8.4** → 路线图 §3 字面「玩家可选族（入口参数 / HUD）」中的 HUD 选族可见 / 选族 UI 属 P8.4，本 change 只做入口参数层；运行时新实体真实模型绑定（模型缺键回退已知限制）、表现层模型键扩充亦属 P8.4。锚 `docs/roadmap/P8-human-orc-melee.md` §3 / `design.md` D8 / C7 / C8。
- **运行时动态实体承接边界** → 已归档 `runtime-entity-movability` 用只读 `EntityView.speed` 承接运行时 `Train` / `Build` 动态实体；本 change 沿用该分层判据、不擅自改 `unwrap_or` 语义 / 不扩权威视图、不改主 `unit-orders/spec.md`。锚 `design.md` D6 / Risks / `WC3-MELEE-RULES.md` R1.4 / R13.6。
- **`file:line` 锚漂移已用符号名规避** → C1–C8 的本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目、不写精确 `:NN` 行号（本轮 `w3-client` 代码会被 owner 改动，P8.1 有行号漂移返工教训）；参考项目为只读固定提交、`file:line` 锚按对照报告原样保留。apply 后四件套任一 artifact 改动即视为解锁、须按 `CODEBUDDY.md §8 / §9` 同步回查其余三份并重走规格门。
- **参考缺口 G1 / G3 / G6 / G7** → 编译期整数 `DefId` 按种族分段且不引入运行时权威状态（G1）、`Spawn` 不含 DefId 用装配层 `def_ids` 关联（G3，P8.0 原创）、**G6（P8.3 特有）** 四项目均无「本地 + AI 两个独立种族参数解耦、四组合 2×2 CPU 端到端逐帧哈希验收」精确做法（godot 对手硬编码 human / `ACCEPTANCE.md:24` 已降级四族 10 配对规划）、**G7（P8.3 特有）** 四项目均无「外部帧冻结 `AiScript` 按种族查表产出训练 / 施法 / 研发命令且不新增权威状态」精确做法（godot 完整 AI profile + 固定人族配方 / Warsmash JASS 且无 melee 按种族 AI 脚本 / `UNIT_AI.md:64` 不做 AI 玩家）：方向已随 P1 放行，但技术取舍为 AI 推荐、不冒充已对齐或已验证。锚 `design.md`「参考项目对照」「参考缺口」。
- **`critic` 规格门 / 代码门** → **规格门已完成（2026-11-17）**：只读 `critic` 规格门复核（清单 7/10/12/13 + 四件套一致性）**blocker 0、可进入 apply**；第 4 点最高优先专项「`AiScript` 改用 `ai_race` 后既有两族基线测试会不会变红」判为**通过**——`play_full_match` 用 `build_driver(setup, 0)`、`AiScript::new(setup, 0)` 因 `while frame(60) <= horizon(0)` 返回空命令集（`ai.rs` 自检测试亦证），既有 `both_races_play_full_match_with_baseline_frame_hashes` / `race_fallback_matches_human_segment_in_full_match` 不受 AI 脚本字节变化影响、不会变红；默认 `MeleeSetup::new()`（本地人族 / AI 兽族）在 `horizon>0` 路径的 AI 命令字节确实会由人族段变兽族段，四件套已在 `design.md` Risks / `tasks.md` 2.4 / `specs/dual-faction/spec.md` 明确「按实际 `ai_race` 更新断言口径、不静默放宽、不冒充默认仍是人族段」的处置。**代码门（全清单）已完成（2026-11-17）：`critic` blocker 0、可进入本地 commit，集成接线真实性 / R1 五测试无静默放宽 / 反向验证真 A-B / 既有基线断言原样未改逐条核验通过**，结论如实登记于「已知限制与待补验」小节「代码门结论」，不冒充 workspace 全绿 / GPU 永久修复 / 真窗口已验收。锚 `CODEBUDDY.md §9` / `tasks.md` 4.5。
- **规格门 residual R1（apply 期 owner 必须显式承接，不解锁规格门）** → `critic` 指出「既有 AI 脚本族断言」枚举不全：四件套只点名 `default_script_keeps_human_ability_and_upgrade_ids` 与 `script_contains_orc_new_category_commands` 两条，但 `ai.rs` 另有 `script_contains_human_new_category_commands`、成对 `reverse_validation_orc_dispatch_formal_passes_the_assertion` / `reverse_validation_orc_dispatch_degenerate_fails_the_same_assertion`（及 `script_trains_orc_new_units` 辅助）也会随 `ai_race` 翻转而失效。owner 在 apply 期 MUST 把这几处一并列入「按实际 `ai_race` 更新断言口径 / 或重命名」作业面，**MUST NOT 静默放宽断言、MUST 保持反向验证仍为真 A-B**（`design.md` Risks 自设禁令）；由 `cargo test --workspace` 与 `critic` 代码门强制暴露并复核。`critic` 另报 R2（保留 `category_def_ids()` 字段/访问器为本地份别名、另增 AI 份以兼容 ≥10 处现存断言与两处退化注入点）、R3（T1 先例锚用了精确 `:NN` 行号 / C2 的 `with_races` 是 apply 后新增符号——归档前折成符号名，改 `design.md` 会解锁指纹、本轮不动）、R4（三处诚实判断编号口径归档说明统一）、R5（规格门结论已回填本条；少数纯确定性 Scenario 未挂 `R-x.y` 属元验证、Requirement 头部已覆盖）——均为不阻塞的承接/措辞项，按「收尾不扩范围」登记、不在规格门阶段改四件套。
- **P8.4 / P8.5 / P9+ / T5 后续 change** → HUD / 五个深度玩法 / 不死暗夜 / AI × lockstep 接合各自单独 change、串行推进；本 change 不承接、不在此时排期细节。锚 `docs/roadmap/P8-human-orc-melee.md` §3 / §6 / `WC3-MELEE-RULES.md` §0.3 T5。
- **人工边界** → 本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀）；push / fetch / pull / clone 及任何远端同步人工独占（`CODEBUDDY.md §4 / §10`）。本轮成文不写 `crates/` 代码、不 commit、不改 `## 规格门` 以外的批准状态。

## 停止条件（追加阶段 / 归档前）

1. C1–C8 逐条仍有证据锚、无一条被本轮改动证伪。
2. 规格门 `status: approved`（用户 2026-11-17 指令、非 self-approve）、`fingerprint:` 真值回填、`approved_by:` / `date:` / `reason:` 齐备，且 `verify_all.py` 的 `spec_gate_check()` 不报失配。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛。
4. 四件套一致性按 `CODEBUDDY.md §8` 回查：proposal 的 Scope / specs 的 `#### Scenario:` / design 的 alternatives / tasks 的步骤互不打架。
5. 归档时：delta 合并去向（新增 `dual-faction` capability、`melee-match` +1 ADDED Requirement）、未勾任务原因、已知限制承接如实登记；主 `openspec/specs/` 是否变化据实说明。

## 规格门

- status: approved
- fingerprint: c1a9af2e1b19b35826e8a6d92fc72632382a64d449207c40633bbf14f6d72a6b
- approved_by: 用户 2026-11-17 本会话指令（非仓库文件，由人工核对）
- date: 2026-11-17
- reason: 用户 2026-11-17 本会话指令「我已执行 git push 完毕。GitHub Action 执行成功（run 44029882988）。现在请你归档并继续开发 P8.3 双族对抗」是人对方向（P8.3 双族对抗）与实施的明确拍板、等价规格门批准（与 P8.1「用户指令作为批准」、P8.2 / `p9-command-hotkeys` 先例一致），非 pending、非 self-approve、不再向人请求批准；只读 `reference-researcher` 已按 `UPSTREAM.md` 矩阵 T1 / T4 / T5（T2 一句）出「参考对照报告」且锚由 `spec-steward` 逐条打开复核（含 T5 / T4 / `ACCEPTANCE.md:24` 降级三处诚实判断、ADR 0002 硬门禁归因纠正、参考缺口 G1 / G3 / G6 / G7，见 `design.md`「## 参考项目对照」）；**`critic` 规格门 / 代码门待复核、如实登记、不冒充通过**；用户拍板的是方向与放行、不是逐条技术细节确认，P2 / P3 技术方案为 AI 推荐、随批准采纳进入 apply，MUST NOT 写成用户已逐条确认。指纹真值由 `python -B tools/verify_all.py --fingerprint p8-dual-faction` 在四件套成文并 `openspec validate p8-dual-faction --strict` 通过后取，`decision-memo.md` 不在指纹纳入范围内（`tools/verify_all.py::spec_artifact_fingerprint`）、回填不自指死锁。

## apply 实施结论（client-owner，2026-11-17）

> 本节为 apply 阶段如实登记，四件套正文零改动、`## 规格门` 小节一字不动；
> 指纹复核仍为 `c1a9af2e…`、`spec_gate_check()` / `decision_memo_check()` 均 True。

- **落地范围**：仅 `crates/w3-client/src/melee/setup.rs`、`ai.rs`、
  `crates/w3-client/tests/melee_loop.rs` 三个文件；`crates/w3-core/` 零 diff
  （`git diff -- crates/w3-core/` 为空），未改 `w3-render` / `w3-net` /
  `w3-relay` / `Cargo.lock` / 主 `openspec/specs/` / 活文档 / 路线图。
- **双族解耦（D1 / C2）**：新增 `MeleeSetup::with_races(local_race, ai_race)`；
  `with_race(race)` 保留为 `with_races(race, race.opponent())` 兼容别名；
  `MeleeSetup::new()` 仍为「本地人族 / AI 兽族」；新增装配层只读字段
  `ai_race`（`race` 继续作本地族别名），`def_ids` 按席位装配，四组合
  `Spawn` / `gameplay_commands` / `kinds` 字节与基线逐字节一致。
- **`category_def_ids` 两份兼容（R2 / D2）**：保留 `category_def_ids` 字段与
  `category_def_ids()` 访问器为**本地份别名**（≥10 处既有断言与 2 处退化
  注入点不破坏）；新增 `ai_category_def_ids` 字段 + `ai_category_def_ids()`
  访问器 + `category_def_ids_for(race)` 只读查表。`AiScript::new` 改由
  `ai_race` + AI 份取训练 / 施法 / 研发目标；`human_catalog` / `orc_catalog` /
  `category_movability` 等本地验收仍用本地份。两处退化注入点分别显式指向
  正确份：`ai.rs` 成对反向验证注入 AI 份、`melee_loop.rs` 端到端断言注入本地份。
- **R1 residual 五处处置**：`script_contains_human_new_category_commands` →
  重命名 `…_human_ai_race_…` 并改按「AI 人族」装配断言；
  `script_contains_orc_new_category_commands` → 重命名 `…_orc_ai_race_…`
  并改按 `MeleeSetup::new()`（AI 兽族）断言；
  `default_script_keeps_human_ability_and_upgrade_ids` → 重命名
  `default_script_uses_ai_race_orc_ability_and_upgrade_ids` 并按实际 AI 兽族
  更新口径（不冒充「默认脚本仍是人族段」）；成对
  `reverse_validation_orc_dispatch_*` 仍为真 A-B（注入点改为 AI 份、正式
  实现改为默认 AI 兽族 setup），`script_trains_orc_new_units` /
  `placeholder_catalog` 保留。均不静默放宽。
- **四组合端到端（D3 / C4）**：`melee_loop.rs` 新增四组合（人vs人 / 人vs兽 /
  兽vs人 / 兽vs兽）各打完一局（采集→建造→训练→英雄施法→研发→战斗→结算→重开），
  只断言「同一组合两实例逐帧 `compute_state_hash()` / `capture_snapshot()`
  字节一致」，MUST NOT 断言跨组合或旧基线数值相等；并断言双方主城 / 农民
  `def_ids` 段位、`AiScript` 命令目标落 AI 族段、胜负只结算一次、重开无残留、
  `SNAPSHOT_VERSION == 3`；另补四组合 R1.4 / R13.6 分层判据与 60 / 144 Hz
  双刷新率逐帧回归。既有 `both_races_play_full_match_with_baseline_frame_hashes`
  / `race_fallback_matches_human_segment_in_full_match` 保持原样全绿。
- **反向验证（D5 / C5）三类退化均有「缺陷态失败 / 修复态通过」成对证据**：
  ① 「AI 命令族退回本地族」、② 「四组合装配退化为单参数 `opponent()`」、
  ③ 「`with_race` 兼容别名退化」，另加 `apply` 路径注入的真 A-B 命令输出
  （临时改坏 `AiScript::new` / `with_races` / `with_race` 后四组合 /
  别名一致性测试 exit 101 FAILED，随即恢复后 exit 0 ok）。退化实现均真经
  `AiScript::new` / `MeleeSetup::with_races` 路径，非旁路模拟。
- **代码门 residual R-1 如实登记（③ 判别力的证据锚口径，`critic` 报告）**：
  ③ 的 spec / tasks 文字原承诺「既有回落基线测试在缺陷态失败、两条证据可复现」，
  但既有四条回落基线（`both_races_play_full_match_with_baseline_frame_hashes`
  / `race_fallback_matches_human_segment_in_full_match` /
  `race_fallback_matches_human_segment_exactly` /
  `race_does_not_change_authoritative_spawn_commands`）按构造**对该退化无判别力**
  ——`play_full_match` 用 `build_driver(setup, 0)`、`AiScript::new(setup, 0)` 返回
  空命令集、`CommandKind::Spawn` 不含 DefId 且四组合 Spawn 字节一致、别名两边
  同退化仍相等。**实际判别力由新增的 `setup.rs::alias_is_consistent` 布尔断言 +
  成对 `reverse_validation_alias_*` + `with_race_alias_matches_dual_entry_and_new_keeps_default`
  承载**：退化候选经真实 `MeleeSetup::with_races` 装配路径构造、比较 `ai_race` /
  `ai_category_def_ids` 等全部装配字段，正式态通过 / 缺陷态失败、元断言防平凡，
  非 trivially-green。**owner 没有悄悄改基线断言**（四条函数体在 diff 中零改动、
  leader `--exact` 单独复跑 ok）——这是「既有基线无判别力 + 验证载体替换」的
  诚实事实，不冒充「既有基线在缺陷态失败」。四件套（`specs/dual-faction/spec.md`
  该 Scenario 与 `tasks.md` 6.3 措辞）属规格锁定、本轮不改（改则解锁指纹须重走
  规格门），**归档时把该 Scenario / 6.3 统一为「别名一致性成对断言」**。
  leader 另外独立复现了 ① 的真 A-B（`AiScript::new` catalog 改回本地份 →
  四组合 `FAILED reason=ai_commands_in_ai_segment`、恢复 → ok）；②③ 的 A-B
  命令输出为 owner 实证、`critic` 以逐行读码 + 静态推演核验判别力、未重跑。
- **`tasks.md` 复选框状态（如实登记，收尾已勾齐 29/29）**：按「仅允许把已完成项
  `- [ ]`→`- [x]`」勾选 24 项由 `client-owner` 实际完成的实现 / 门禁 / 反向验证
  任务（1.1–1.4 / 2.1–2.4 / 3.1–3.5 / 4.1–4.4 / 6.1–6.4 / 7.1–7.3）；
  收尾阶段 leader 据实补勾 5 项：4.5（只读 `critic` 全清单代码门，已执行、
  blocker 0）、4.6（本地 commit 本轮 `feat` commit；push / fetch / pull / clone
  仍人工独占）、5.1（参考对照逐锚 / `critic` 第 12 项）、5.2（范围边界）、
  5.3（许可证边界）——后三项均由本次 `critic` 代码门清单第 7/12/13 项结果
  实质复核承接。复选框归一化不触发规格解锁，四件套指纹复核仍为
  `c1a9af2e…`；真窗口人工验收 / GPU 永久修复 / T5 / P8.4–P8.5 / P9+ 仍为
  「未执行 / 待补验」承接项，不冒充已完成。
- **门禁（本地 CPU 侧）**：`cargo fmt --all` + `cargo fmt --all -- --check`、
  `cargo test -p w3-core --test determinism`（26）、`cargo test -p w3-client --lib`
  （68）、`--test melee_loop`（20）、`--test movability`（31）、
  `--test command_hotkeys`（25）、`--test interaction`（25）、
  `cargo test --workspace`（本轮 exit 0、GPU 测试未复现崩溃）、
  `python tools/check_encoding.py`、`python tools/check_source_conventions.py`、
  `git diff --check` 全部 OK；`openspec validate p8-dual-faction --strict`
  is valid。

## 已知限制与待补验（如实标注、不冒充通过）

- **真窗口人工验收未执行 / 待补验**：本轮 CPU 侧以回归 + 门禁自证，真窗口 /
  GPU / egui overlay 未人工观察，不冒充通过（沿用 P8.1 / P8.2 /
  `p9-command-hotkeys` 归档说明先例）。
- **GPU 环境项**：本机 `cargo test --workspace` 的 `real_map_e2e.rs` /
  `gpu_headless.rs` 两个 GPU 测试此前有间歇性 `0xc0000005` 原生崩溃，与
  本 change 无关（相关文件零改动、对新增符号零引用）；本轮两次 workspace
  全跑均 exit 0、未复现。如后续复现，按「跳过该 2–4 个 GPU 测试跑 CPU 侧
  全绿」+ 如实标注处理，交付结论不冒充「workspace 全绿」。
- **代码门结论（已完成，2026-11-17）**：只读 `critic` 全清单代码门（13 项）
  复核 **blocker 0、可进入本地 commit**。三处最高风险明确判为未重演：
  ① 集成接线真实性（防 p9 首轮 B1）——`AiScript::new` 生产代码真用
  `setup.ai_race` + `setup.ai_category_def_ids()`、四组合 `play_full_match`
  真经 `AiScript::new` 产出 AI 命令并断言段位、leader 独立改坏验红印证；
  ② R1 五测试无静默放宽 / 无删断言（多处反而新增负向泄漏断言）、成对
  `reverse_validation_orc_dispatch_*` 退化注入随生产读取份改打
  `ai_category_def_ids`、仍为真 A-B；③ 反向验证三类退化均真经生产路径、
  既有两族回落基线断言原样未改（R-1 证据锚口径另如实登记）。
  `crates/w3-core/` 与 `defs.rs` 零 diff、`SNAPSHOT_VERSION=3`、golden hash
  不变、规格锁定指纹 `c1a9af2e…` 未失配。`critic` 另报 R-2（极轻：测试内
  `expect` 个别未 `reason=` 结构化，非生产路径、不建议为它动 diff）。
- **归档 / commit / push**：本地 commit 由 leader（中文描述 + Conventional
  Commits 英文 type 前缀，本次 `feat` commit）；push / fetch / pull / clone
  及任何远端同步人工独占（`CODEBUDDY.md §4 / §10`）。归档时须把
  `specs/dual-faction/spec.md` 的 ③ Scenario 与 `tasks.md` 6.3 措辞统一为
  「别名一致性成对断言」（R-1），并统一 `design.md` 两条超长行（规格门 R3）
  与三处诚实判断编号口径（R4）——均为归档说明的纯措辞项、不改行为契约。
- **范围外承接（C7 / C8 / D8）**：HUD 选族 UI / 表现层模型键（P8.4）、
  P8.5 五个深度玩法、不死暗夜（P9+）、中文上屏、运行时新实体真实模型绑定
  （模型缺键回退四面体占位）、AI × lockstep 命令接合（`WC3-MELEE-RULES.md`
  §0.3 T5）、真窗口人工验收均由后续独立 change 承接；联机 / 回放 / AI
  玩家 / 迷雾是 1.0 已在能力或范围、不是不做项。运行时动态实体承接边界
  （`runtime-entity-movability` 已归档）本 change 沿用、不扩权威视图。
- **数值与资产**：本 change 不加任何 `UnitDef` / `AbilityDef` / `UpgradeDef`，
  `defs_hash` / `content_hash` 不变；零资产 / 零文件 IO / 零 `config.toml`
  读取 / 零新增依赖，两族数值差异是既有 `[取舍]` 合成原创常量相对大小、
  非经典事实。不得据本 change 声称跨平台确定性有新验证（macOS / ARM64 /
  其它 rustc 仍待验证，ADR 0002 复核记录）。
