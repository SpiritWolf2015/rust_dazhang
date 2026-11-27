# 决策纪要 — `p13-ability-full-roster`

> 取料纪律：基于仓库 artifacts 与 P11 `reference-researcher` 只读对照报告结论（已落盘于 P11 归档件）回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。
> 合规边界：纯文档，不进状态哈希 / 快照、不含资产、不含本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 阶段：提案成文期（本轮只成文、不实施代码，apply 是后续独立会话）。用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」，本 change 为玩法侧；文档侧见 `p12-ability-doc-reconciliation`。**批准状态为 `pending`、由人拍板、AI 未 self-approve**。
> 结构说明：`## 规格门` 置于末尾且只有本节以「规格门」开头（`tools/verify_all.py::memo_section_body()` 按前缀匹配），`status` 字段以本节为准；本纪要不在四件套指纹集合内（`tools/verify_all.py:175-195`），改它不触发 `CODEBUDDY.md §8` 规格解锁。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | P11 本期范围 = 四族英雄技能全量 + 各族代表性单位 / 道具技能；**中立英雄 / 野怪 / 全部单位技能顺延、标已知限制**，由后续独立 change 承接 | `openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`「已知限制」第 1 条、`design.md` D1 /「Risks / Trade-offs」顺延项条、`tasks.md` 2.1、`spec.md` Req1 Scenario；`openspec/specs/ability-system/spec.md:16-17` |
| C2 | Q1「清单逐项 SLK 对齐」apply 实际落地是**经典结构口径的合成占位表 72 条**，未经本机导出 SLK 逐项核对 | `crates/w3-core/src/ability_system.rs:31`（`ABILITY_CATALOG_LEN = 72`）/ `:121-177`（`SYNTHETIC_ABILITY_CATALOG`）、`crates/w3-client/src/melee/defs.rs:734`（`FOUR_RACE_ABILITY_CATALOG`）/ `:783` / `:1321-1322`（自述「72 条合成原创 `DefId` 占位」）；`openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`「已知限制」第 11 条 |
| C3 | 收尾只读实测本机导出表与整齐结构口径**系统性不一致**，用户 2026-11-25 已拍板「接受水位为已知限制、由后续独立 change 承接」；导出数据是**仓库文件外证据 / 人工确认项**，不入库 | `openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`「已知限制」第 11 条（heroAbilList 非空 85 单位 / 344 技能 ID、每英雄 {3:1,4:79,5:5}、大招 {0:33,1:51,2:1}、四族 hero=1 human16/orc24/undead17/nightelf17、`levels` {0:18,1:668,2:6,3:93,4:14}、大招 44 条）；ADR 0001（gitignored `config.toml`、不入库）；用户 2026-11-25 拍板（仓库文件外证据 / 人工确认项） |
| C4 | 两层张力：对齐的是清单条目与分类，不是照搬经典名称 / 四字符 rawcode / 数值；内部合成原创 `DefId`、不引入四字符 ID → `DefId` 映射层、数值 `[待核]` 不入库 / 不进权威状态 / 哈希 / 快照 | `WC3-MELEE-RULES.md:7,124,351,374`；ADR 0001 / 0002 / 0004；`crates/w3-client/src/melee/defs.rs:1-6`（纯编译期零文件 IO）；P11 `design.md` D1「两层张力」/ D2、`decision-memo.md` P8 / Q1 / Q2 拍板记录 |
| C5 | 玩法规则基线：1.0 范围 S3 / S5 / S6（`WC3-MELEE-RULES.md:27-30`）与 R6（`:165-171`）/ R8（`:188-196`）/ R9（`:200-218`）/ R7.6（`:184`）/ R2（`:113-126`）/ R12.4（`:248`）/ R10.5（`:229`）/ §14（`:418-424`）；N1–N6 明确不做，尤其 N5 JASS·wtg VM（`:43-49`）；规则编号引用 ≠ 已实现（`:393` 三态判定） | `WC3-MELEE-RULES.md` 上列 `file:line`；`docs/decisions/0003-scope.md:15,38`；`CODEBUDDY.md §9` WC3 业务规则基线 |
| C6 | 三版本号权威值当前均为 **9**；若新增权威状态须四处同步 + `WorldGameplayState::hash` + 按 ADR 0002 评估 bump（若拍板则 9→10 全套）；`cargo test -p w3-core --test determinism` + 形态①改坏验红 | `crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`（本轮只读 Grep）；`crates/w3-core/src/sim_match.rs:4328,4375,4418-4419`；`0002-determinism.md:111,115,117,118`（P9 7→8 / P11 8→9 全套先例、跨平台待验证口径）；`0005-protobuf-payload.md:87`；`CODEBUDDY.md §5` |
| C7 | 清单数据来源是本体 MPQ 数据表、与地图脚本分离；定义层 = `AbilityData`（数值）+ `UnitAbilities.abilList` / `heroAbilList`（挂载）+ `*AbilityFunc.txt` + `AbilityBuffData`；Melee 具名技能不依赖 WE（论证性结论） | 矩阵条目 `docs/references/UPSTREAM.md:108`（T3）/ `:109`（T4）；参考项目内路径 `godot_warcraft3/docs/data/WC3_ASSET_PATHS.md:86-107,245,274`、`godot_warcraft3/docs/data/PIPELINE.md:118-121,136-149`、`godot_warcraft3/packages/content/definitions/units/unit_abilities_def.gd:70-71`、`WarsmashModEngine/.../StandardObjectData.java:50,217`、`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:117-126`；P11 `decision-memo.md` C6 / C7 |
| C8 | T1 承重锚一律 `vera20k@305ea3a75:<path>:<line>` 双标、MUST NOT 退化成工作树 HEAD 行号；AGPLv3（Warsmash）只借语义不复制代码、`ConcordRTS` RA2/YR 不作 WC3 事实来源、README 自述不作已验证事实 | 矩阵条目 `docs/references/UPSTREAM.md:106`（T1）/ `:109`（T4）/ `:112`（T7）/ `:114`；参考项目内路径 `vera20k@305ea3a75:src/net/lockstep.rs:1-20,35-39`、`vera20k@305ea3a75:src/net/command_exchange.rs:39,52,80`、`WarsmashModEngine/LICENSE:1-3`；`UPSTREAM.md:11,19-25`（固定提交 + 升级复核先例）/ `:35`（README 自述）；`0001-license.md:17,30,59,66-73`；P11 `design.md` D10 / 结论 9、`decision-memo.md` C9 / C10 |
| C9 | 规则文档同步为正式结构门禁（§14 第 5 条），玩法 change 收尾须在「## 规则文档同步」小节给结论、同步附录「规则 → 实现覆盖对照」R6 技能独立行 + §15；`status: pending` 由 `rule_doc_sync_check()` 豁免 | `WC3-MELEE-RULES.md:393-414,415-421,424,426-437`（三态判定 / 附录 R6 技能独立行 `:405` / §15 `:437`）；`tools/verify_all.py:256-294`（`rule_doc_sync_check()`）；`openspec/decision-memo.template.md:48-53`；`openspec/config.yaml rules.proposal / rules.tasks` |
| C10 | 本轮只成文、不实施代码；规格门须由人拍板、不 self-approve；四件套 `## Purpose` 不被 delta 覆盖、delta 顶部不写说明块、逐字保留历史 `#### Scenario:` 身份键、口径改在 THEN | 用户 2026-11-25 明确拍板（本会话用户消息 / leader 转达，仓库文件外证据 / 人工确认项）；`openspec instructions specs` 的 MODIFIED 整块复制规则；`CODEBUDDY.md §8` 四件套一致性 / 规格锁定与重审；P11 `design.md` D0 |
| C11 | 决策纪要三必填小节 = 承重约束 / 拍板决策 / 待确认项；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed；`rule_doc_sync_check()` 对 pending 豁免；四件套指纹不含 `decision-memo.md` | `tools/verify_all.py:29`（`REQUIRED_MEMO_SECTIONS`）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（`spec_artifact_fingerprint()`）/ `:207-253`（`spec_gate_check()`）/ `:256-294`；`openspec/decision-memo.template.md:55-61` |
| C12 | 编码基线 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；git 提交信息中文；AI 不得 push / fetch / pull / clone；零资产红线 | `CODEBUDDY.md §2` / §3 / §4 / §6；`openspec/config.yaml:34-40`；`tools/check_encoding.py` |

## 拍板决策（含被否备选与理由）

- **P1 建两个独立 change：文档侧 `p12-ability-doc-reconciliation`（三项成文口径订正）+ 玩法侧 `p13-ability-full-roster`（本 change，顺延项全量 + Q1 清单逐项 SLK 对齐），各自 Scope 严格划分、不互相串味**。锚：用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」（本会话用户消息 / leader 转达，**仓库文件外证据 / 人工确认项**）。被否：① 合并为一个 change（理由：用户明确划分，文档订正零内核改动与玩法全量触确定性内核的门禁 / owner 边界不同）；② 把文档订正塞进玩法 change 或反之（理由：同上，不互相串味）。
- **P2 承接 P11 已知限制 1（顺延项全量）与 11（Q1 清单逐项 SLK 对齐）作为本 change 的两项 Scope**。锚：用户 2026-11-25 已拍板「接受水位为已知限制、由后续独立 change 承接」（`openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md:100`，仓库内 artifacts 明确记载的用户拍板）+ 本轮 `AskUserQuestion` 式 Scope 划分（本会话用户消息 / leader 转达，仓库文件外证据 / 人工确认项）。被否：① 继续把顺延项 / 清单水位标已知限制、本期不承接（理由：与用户拍板「由后续独立 change 承接」冲突）；② 扩范围到 p12 的三项文档订正或真窗口 / golden hash 双平台实际执行（理由：超已拍板边界，按「收尾阶段优先记录已知限制而非扩范围」处置）。

> 措辞纪律：P1 / P2 为本轮**仅有的**已拍板项，证据锚为本会话用户消息 / leader 转达（仓库文件外证据 / 人工确认项）与 P11 归档件，MUST NOT 写成 AI 自定 / AI 推荐 / AI 已确认 / 沉默授权。**用户本轮只拍板了「建两个 change、各自 Scope 划分」**；其余范围 / 方案 / 数值 / 验收线一律进「待确认项」，不臆造为已定案。特别是：清单逐项人工核对**尚未执行**、实际条目数**未核实**、是否新增权威状态 / 是否 bump 9→10 **未拍板**、规格门批准记录**未回填**。

## 被否方向台账

- **R1 照搬经典具名技能名称 / 四字符 rawcode / 数值，或引入四字符 ID → `DefId` 映射层** → 违反 ADR 0001 零资产 / `WC3-MELEE-RULES.md:7,351`、P11 Q1↔Q2 调和口径「清单对齐 SLK、标识仍合成」（C4）；两层张力按 D2 显式区分。
- **R2 把「清单逐项人工核对」冒充「已完成」、臆造精确条目数、把 `[待核]` 数值冻结进权威定义** → 用户 2026-11-25 明确「接受水位为已知限制、由后续独立 change 承接」，本轮未做逐项核对（C3）；`WC3-MELEE-RULES.md §14` / ADR 0001 / 0004。
- **R3 把 Warsmash Java / LibGDX 代码、`*.j` / `.jui` / `jassparser/` 代码引入仓库，或用 JASS / wtg 触发器 VM 实现技能** → AGPLv3 只借语义、不复制代码、不引入依赖（C8）；N5 明确不做（`0003-scope.md:15,38` / `WC3-MELEE-RULES.md:43-49`）；R7.6 道具技能只走既有 `EffectDef` 组合。
- **R4 把 `ConcordRTS` 的 RA2/YR 规则 / 数值 / 格式冒充 WC3 事实，或把 `README.md` 自述「逐字节一致」当本项目已验证事实** → `UPSTREAM.md:35,114` / `WC3-MELEE-RULES.md:3,351` 唯一权威（C8）。
- **R5 把 `@305ea3a75` 双标退化成工作树 HEAD 行号** → `UPSTREAM.md:11,19-25` 先例「保留原锚、不据前进改写」（C8 / P11 `design.md` D10）。
- **R6 把「design 引用 `R-x.y`」冒充「规则已实现 / 已人工验收」** → `WC3-MELEE-RULES.md:393` 三态判定 / §14 第 5 条 / `critic.md:34` 清单第 13 项（C5 / C9）；真窗口 8.x 人工清单保持「未执行 / 待人工」。
- **R7 新增权威状态却不四处同步 / 跳过 `determinism` 重跑或形态①改坏验红，或声称 bump 后双平台 / 全平台确定性已验证** → ADR 0002 §6-8 / §8 / `CODEBUDDY.md §5` / `0002-determinism.md:118`（C6）。
- **R8 自填 `status: approved` + 指纹，或写「## 省略说明」求门禁变绿** → `CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve；本 change 非小改动、豁免会违反 fail-closed 信号（C10 / C11）。

## 待确认项（如实标注，不冒充已拍板）

- **是否新增权威状态** → 若全量实现新增权威运行态则须四处同步 + `WorldGameplayState::hash` + 评估 bump（C6）；本轮不臆造为已定案，待规格门由用户拍板。
- **是否 bump 9→10 全套** → 三版本号当前 9，是否随新增权威状态 bump 到 10（含全仓消费者同步 / Grep 旧版本号无残留 / 旧 v9 明确拒绝 / `w3net.proto` 只追加 tag / golden hash 重取 / ADR 复核记录）待规格门拍板（C6 / `0002-determinism.md:117` / `0005-protobuf-payload.md:87`）；不 bump 时须写明「零新增权威状态」依据。**本轮一律不臆造为已定案**。
- **清单实际条目数与人工逐项核对** → 按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData` 人工逐项核对由 `content-owner` 在 apply 阶段执行（仓库文件外证据 / 人工确认项），实际条目数核对后如实登记、不臆造精确数字（C2 / C3）；收尾实测规模数字一律标「粗统计、以人工核对为准」。**MUST NOT 冒充「已逐项核对完成」**。
- **数值 `[待核]` 核实与规则口径** → 数值一律 `[待核]`、以本机导出 SLK 核对、不入库、不静默改规则；规则与本机导出不符时先标 `[待核]` 并回到用户拍板（`WC3-MELEE-RULES.md §14` 第 2 条）。
- **规则文档同步的拍板顺序** → 按 Q-A 类与 P11 同批、但须在规格门批准记录回填后的 apply 阶段由 `docs-owner` 落定附录「规则 → 实现覆盖对照」R6 技能独立行 + §15（C9 / D6）；提案阶段正文零改动、结论为「待 apply 阶段同步」，由用户拍板顺序。
- **规格门批准记录** → 「## 规格门」保持 `status: pending`；四件套指纹（`tools/verify_all.py --fingerprint p13-ability-full-roster`）、`critic` 规格门报告、`reference-researcher` 独立对照报告（已产出、锚已逐条核实、状态文字本批回填，D7）、`approved_by` / `date` 均由 leader / 用户后续拍板回填，本轮不碰、AI 未 self-approve。用户 2026-11-25 经 `AskUserQuestion` 已拍板「批准，p13 先修订再锁定」、`critic` 三份规格门报告 `blocker = 0`，四件套指纹与批准记录由 leader 按授权序列回填、非 AI self-approve、非沉默授权，批准状态一律以「## 规格门」小节 `status: approved` 为准；玩法待确认项（是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数 / 数值 `[待核]` / 验收线）仍由用户在 apply 阶段拍板、不冒充已确认。
- **`reference-researcher` 独立对照报告与参考缺口** → 本 change 命中 T3 / T4 / T1，本轮「## 参考项目对照」小节沿用 P11 归档件已核实结论（C7 / C8）；独立对照报告已产出并逐锚打开核实、状态文字本批回填，报告结论与现有对照小节实质一致、未新增 / 删改任何 Scope / Requirement / Scenario / 决策 / 验证口径；参考缺口 1–6 仍回到用户拍板、不臆造（是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数 / 数值 `[待核]` / 真窗口 8.x / golden hash 双平台 / CI 双平台仍待 apply 阶段由用户拍板或待人工 / 待 VM，不得冒充已确认）。
- **验收线** → 真窗口 8.x 人工清单保持「未执行 / 待人工」；golden hash 双平台（Debian 12 / macOS / ARM64 / 其它 rustc）逐字节比对、CI 双平台 `verify_all.py` 一律登记为待 VM / 人工（人工 push 后触发）、不写成已验证（C6 / D5）。
- **`spec_gate_check()` fail-closed** → `invalid spec gate status 'pending'` 属预期信号，不绕过、不修复、不冒充放行（C11）。
- **apply 阶段执行时机与 owner 分工** → 一个 change 一个 owner、`content-owner` / `sim-owner` / `net-owner` / `client-owner` / `docs-owner` 串行、不同时改多 owner 文件，待 leader 安排后续独立会话；本轮不实施代码、不勾任一复选框。

## 已知限制（如实登记，不冒充已解决、不扩范围）

- **清单逐项 SLK 对齐水位**：P11 apply 落地为合成占位表 72 条、未经本机导出 SLK 逐项核对（C2），本轮承接但**尚未执行逐项人工核对**、实际条目数未核实；`decision-memo.md` 不冒充「已逐项核对完成」、不臆造精确数字（C3）。属用户 2026-11-25 拍板「接受水位为已知限制、由后续独立 change 承接」的承接项，非实现缺陷。
- **真窗口 8.x 人工清单**：全部保持「未执行 / 待人工」、不标通过、不用离屏截图 / CPU 回归冒充（D5 / `CODEBUDDY.md §9`）。
- **golden hash 双平台 / CI 双平台**：Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对、CI 双平台 `verify_all.py` 一律登记为「待 VM / 人工」，不声称全平台 / 双平台确定性已验证（`0002-determinism.md:118` / C6）。
- **参考缺口 1–6 与独立对照报告**：`design.md`「## 参考项目对照」如实登记（无四族全量具名技能参考项目权威清单、具名技能数值 / 快捷键不作权威、无「Melee 不依赖 JASS」直接原文断言、独立 `reference-researcher` 对照报告已产出并回填且结论与现有对照小节实质一致、golden hash 双平台排期、`vera20k` 工作树 HEAD 前进人事项），回到用户拍板、不臆造（D7 / C8）；真窗口 8.x / golden hash 双平台 / CI 双平台仍待 apply 阶段由用户拍板或待人工 / 待 VM，不得冒充已确认。
- **`vera20k` 工作树 HEAD 前进**：按 `UPSTREAM.md:19-25` 先例保留 `305ea3a75` 固定锚、不据前进改写任何 ADR；登记为待 `docs-owner` 独立 change 处理的人事项，本 change 不因此阻塞。
- **本轮 `git diff -- openspec/specs/` 为空**：主 spec 只在 archive 合并时复核或 apply 阶段改，MODIFIED delta 逐字保留 P11 两个历史 Scenario 身份键、口径只改在 THEN（C10）。
- **p12 三项文档订正不纳入**：历史 `==8` 字面 / 清单规模 91→95 列 / tasks 5.2 表述订正由 `p12-ability-doc-reconciliation` 承接，本 change 严格不串味（P1）。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C12 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项已清空或显式豁免（本轮为提案成文期，是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数 / 验收线 / 批准记录等由规格门用户拍板，未自行收敛）。
3. 不以「预设问题问完」「问了 N 轮」当收敛；批准由用户拍板、不拿沉默当授权。
4. 「## 规格门」保持 `status: pending`、不 self-approve、不写「## 省略说明」。
5. 清单逐项人工核对未冒充已完成、真窗口 8.x / 双平台人工清单未标通过。

## 规则文档同步

- 结论：**需同步，但本轮为提案成文阶段（只成文、不实施代码），给出「待 apply 阶段按已拍板顺序同步」的如实结论**。依据 `WC3-MELEE-RULES.md §14` 第 5 条：本 change 是**玩法 change**，`design.md` 承接 S3 / S5 / S6 与 R6 / R8 / R9 / R7.6 / R2 / R12.4 / R10.5 的玩法实现需求（仅作需求依据）；收尾（规格门批准记录回填后的 apply 阶段）须由 `docs-owner` 按 Q-A 已拍板顺序更新附录「规则 → 实现覆盖对照」**R6 技能独立行**（`:405`，现状标「部分落地」、含顺延项 / 清单逐项 SLK 核对待人工）与 §15 变更记录（`:437`），**不新增 S / R 编号、不改既有 `R-x.y` 条文与状态标记、不改 §13.7 现状 / 参考项目数量（仍五个）、按 §14 第 2 条注明日期与依据、不静默改规则**（C5 / C9）。
- **诚实边界**：本轮**未**同步活文档、未实现任何玩法（`git diff -- docs/references/WC3-MELEE-RULES.md` 应为空），MUST NOT 把「design 引用 `R-x.y`」或「代码将来会实现」冒充「该规则已完整实现 / 已人工验收」（`WC3-MELEE-RULES.md:393` 三态判定 / §14 第 5 条 / `critic.md:34` 清单第 13 项）；附录 R6 技能独立行状态在 apply 阶段据实更新、真窗口 8.x 人工清单一律「未执行 / 待人工」、不标通过；清单逐项 SLK 人工核对 / `[待核]` 数值仍待人工、不静默改规则（§14）。
- 结构门禁：`tools/verify_all.py::rule_doc_sync_check()` 仅对过规格门 `status: approved` 的玩法 change 校验本小节存在且非空；本 change `status: pending` 豁免。本小节按 §14 第 5 条如实给出「需同步、待 apply 阶段」的结论、保持非空；`decision-memo.md` 不在四件套指纹集合内（C11），补该小节不触发 §8 规格解锁。

## 规格门

- status: approved
- fingerprint: 7c776a14d8971034a046d0b690b2405a91708bd490b0d61033b477c543580178
- approved_by: 用户（2026-11-25 经 `AskUserQuestion` 拍板「批准，p13 先修订再锁定」；`critic` 三份规格门报告 `blocker = 0`、`reference-researcher` p13 对照报告已出，由 leader 按授权序列回填，非 AI self-approve、非沉默授权）
- date: 2026-11-25
- reason: 用户 2026-11-25 规格门拍板「批准，p13 先修订再锁定」；`critic` 规格门报告 `blocker = 0`、`reference-researcher` 独立对照报告已产出（T3/T4/T1 命中、五条锚按固定提交逐条打开成立、许可证红线守住）。按授权序列先回填「独立报告已出」状态文字、订正 G1–G10 与 git 边界措辞、同步回查四份并重算指纹，再回填本小节 `approved` + 新指纹，避免先锁定后改触发 `CODEBUDDY.md §8` 规格解锁。待确认项（是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数 / 验收线）仍由用户在 apply 阶段拍板，不冒充已确认。

规格已锁定；`verify_all.py::spec_gate_check()` 机械校验 `status` 与指纹一致；此后改任一四件套 artifact 即视为解锁，MUST 按 `CODEBUDDY.md §8` 同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）。提案阶段「保持 pending / 由 leader 回填」等措辞为当时流程约束记录（沿用 P11 归档件体例），批准状态一律以本小节 `status: approved` 为准。
