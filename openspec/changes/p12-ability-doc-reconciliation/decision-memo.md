# 决策纪要 — `p12-ability-doc-reconciliation`

> 取料纪律：基于仓库 artifacts 回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。
> 合规边界：纯文档，不进状态哈希 / 快照、不含资产、不含本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 阶段：提案成文期（本轮只成文、不实施代码，apply 是后续独立会话）。用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」，本 change 为文档侧；玩法侧见 `p13-ability-full-roster`。**批准状态为 `pending`、由人拍板、AI 未 self-approve**。
> 结构说明：`## 规格门` 置于末尾且只有本节以「规格门」开头（`tools/verify_all.py::memo_section_body()` 按前缀匹配），`status` 字段以本节为准；本纪要不在四件套指纹集合内（`tools/verify_all.py:175-195`），改它不触发 `CODEBUDDY.md §8` 规格解锁。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | 三版本号权威值当前均为 **9**（不是 8）；`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 在源码与 config 一致 | `crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13`；`CODEBUDDY.md §5`；本轮只读 Grep 命令输出 |
| C2 | 主 spec 存在约 11 处历史无条件 `==8` /「保持 8」/「当前 8」/「三版本号 = 8」硬断言，根因是 P9 / P10 / P11 历次 bump 后历史 delta 合并残留 | `openspec/specs/ai-player-lockstep/spec.md:34`、`openspec/specs/deterministic-simulation/spec.md:44,66`、`openspec/specs/dual-faction/spec.md:82`、`openspec/specs/undead-faction/spec.md:104`、`openspec/specs/ability-system/spec.md:4,67`、`openspec/specs/gameplay-systems/spec.md:42`、`openspec/specs/lockstep-networking/spec.md:12`、`openspec/specs/melee-match/spec.md:82`；本轮全仓 Grep 命令输出 |
| C3 | P9 Q9' 已拍板「归档后主 spec 历史 `==8` 字面由独立 change 收口」；主 spec 只在 archive 合并时复核、提案阶段不直接编辑 | `openspec/changes/archive/2026-11-25-p11-ability-system/tasks.md` 7.10、`design.md` D0 /「## Risks / Trade-offs」bump 全仓消费者条、`decision-memo.md`「已知限制」第 2 条；`proposal.md`「Modified Capabilities」注释 |
| C4 | 清单规模登记偏差：P11 `design.md`「清单规模事实锚」登记 `AbilityData.json` 803 条 / **91 列**，content-owner 本机导出实测 **803 条 / 95 列**，其余规模元数据全部一致 | `openspec/changes/archive/2026-11-25-p11-ability-system/design.md:20`（`## 清单规模事实锚`）、`decision-memo.md`「已知限制」第 8 条；95 列为 content-owner 本机导出只读实测，属**仓库文件外证据 / 人工确认项**（ADR 0001，导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库） |
| C5 | tasks 5.2 表述歧义：`compute_state_hash()` 自 P5 起把 `self.config.content_hash`（= `FrozenDefinitions::content_hash()` 冻结内容身份）作为既有字段写入，「不改运行时 `compute_state_hash()`」与 ADR 0002 契约冲突，属成文表述歧义、非实现缺陷 | `openspec/changes/archive/2026-11-25-p11-ability-system/tasks.md:31`（5.2）、`decision-memo.md`「已知限制」第 9 条；`crates/w3-core/src/sim_match.rs:4328`（`hasher.write_u64(self.config.content_hash)`）/ `:4375`（`hasher.write_u64(self.defs.content_hash())`）/ `:4418-4419`（`first_difference` 比较 `content_hash`）；`docs/decisions/0002-determinism.md §6`（`content_hash` 为参与字段） |
| C6 | P11 归档件四件套已 `status: approved` + 指纹锁定，改任一 artifact 即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门由人拍板；订正 MUST NOT 追溯改写归档件 | `openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`「## 规格门」；`CODEBUDDY.md §8` / §9 规格锁定与重审 |
| C7 | 四件套指纹集合仅含 change 目录内 `proposal.md` / `design.md` / `specs/**/*.md` / `tasks.md`，**不含**主 `openspec/specs/**`、不含 `decision-memo.md` | `tools/verify_all.py:175-195`（`spec_artifact_fingerprint()`）；P11 `design.md` D0 备选① 引用 `:182-186` |
| C8 | 决策纪要三必填小节 = 承重约束 / 拍板决策 / 待确认项；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免 | `tools/verify_all.py:29`（`REQUIRED_MEMO_SECTIONS`）/ `:143-172`（`decision_memo_check()`）/ `:207-253`（`spec_gate_check()`）/ `:256-294`（`rule_doc_sync_check()`）；`openspec/decision-memo.template.md:55-61` |
| C9 | 编码基线 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；git 提交信息中文；AI 不得 push / fetch / pull / clone；零资产红线 | `CODEBUDDY.md §2` / §3 / §4 / §6；`openspec/config.yaml:34-40`；`tools/check_encoding.py` |
| C10 | 本 change 经核对 `UPSTREAM.md` 矩阵 **T1–T7 均不命中**（纯文档 / 规格口径订正，不改任何实现层），故承重约束**无一条来自参考项目**、无参考项目锚 / 参考缺口需要双标 | `docs/references/UPSTREAM.md:104-112`（按任务类型的强制参考矩阵）；`design.md`「## 参考项目对照」逐条判定 |

## 拍板决策（含被否备选与理由）

- **P1 建两个独立 change：文档侧 `p12-ability-doc-reconciliation`（本 change，三项成文口径订正）+ 玩法侧 `p13-ability-full-roster`（顺延项全量 + Q1 逐项 SLK 对齐），各自 Scope 严格划分、不互相串味**。锚：用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」（本会话用户消息 / leader 转达，**仓库文件外证据 / 人工确认项**）。被否：① 合并为一个 change（理由：用户明确划分，文档订正零内核改动与玩法全量触确定性内核的门禁 / owner 边界不同）；② 把文档订正塞进玩法 change 或反之（理由：同上，不互相串味）。

> 措辞纪律：P1 为本轮**唯一**已拍板项（证据锚 = 本会话用户消息 / leader 转达，仓库文件外证据 / 人工确认项），MUST NOT 写成 AI 自定 / AI 推荐 / AI 已确认 / 沉默授权。**本轮只拍板了「建两个 change、各自 Scope 划分」**；其余范围 / 方案 / 口径 / 数值 / 验收线一律进「待确认项」，不臆造为已定案。三项订正的具体口径（95 列 vs 删除数字、约 11 处逐处判定、表述最终文字）与规格门批准记录均**未经用户拍板**。

## 被否方向台账

- **R1 追溯改写 P11 归档件四件套或 `decision-memo.md`** → 归档件已 `status: approved` + 指纹锁定（C6），改即触发 §8 规格解锁须重走规格门；用户明确「不追溯改写归档件」。
- **R2 为订正新增第 10 份 MODIFIED delta 凑数** → 用户明确禁止；OpenSpec 要求 MODIFIED 复制整块 requirement，逐份改写会与主 spec 既有口径叠床架屋（P11 `design.md` D0）。
- **R3 全仓正则把字面 `8` 一律替换为 `9`** → 会抹平 P9 / P10 / P11 各 change 的「bump 8」实施期史实、误伤无关数字（C3 / D1）。
- **R4 把 tasks 5.2 表述歧义判定为实现缺陷、去改 `compute_state_hash()` 或 ADR 0002 正文** → 用户明确「成文表述订正、非实现缺陷」；`sim_match.rs:4328` 既有写入正是 ADR 0002 §6 正确实现（C5）。
- **R5 自填 `status: approved` + 指纹，或写「## 省略说明」求门禁变绿** → `CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve；本 change 非小改动、豁免会违反 fail-closed 信号（C8）。

## 待确认项（如实标注，不冒充已拍板）

- **三项订正的具体落地口径** → 待规格门用户拍板：① 清单规模是订正为 **95 列** 还是删除具体列数、统一「粗统计、以人工核对为准」（用户给的是「订正为 95 列（或删除该数字、按粗统计口径统一）」二选一，未定案）；② 约 11 处主 spec 逐处判定「当前版本号消费者」与「历史 bump 8 沿革史实」的最终清单；③ 冻结定义哈希契约表述的最终文字。
- **95 列实测值** → 属 content-owner 本机导出只读实测（仓库文件外证据 / 人工确认项，C4），MUST NOT 写成「用户已确认」；待内容侧 / 人工复核。
- **订正落点文件** → 后续主 spec / 文档用哪几处承载「清单规模」与「冻结定义哈希契约表述」尚未定案，待 apply 阶段由 `spec-steward` 与规格门拍板。
- **规格门批准记录** → 「## 规格门」保持 `status: pending`；四件套指纹（`tools/verify_all.py --fingerprint p12-ability-doc-reconciliation`）、`critic` 规格门报告、`reference-researcher` 对照报告（本 change 判定不命中矩阵、无报告，待 `critic` 第 12 项复核该判定）、`approved_by` / `date` 均由 leader / 用户后续拍板回填，本轮不碰、AI 未 self-approve。
- **是否命中参考矩阵 T1–T7** → 本轮据 `UPSTREAM.md:104-112` 判定均不命中（C10 / design「## 参考项目对照」），待 `critic` 清单第 12 项复核；若改判命中则须补 `reference-researcher` 对照报告并回到用户拍板，不臆造。
- **apply 阶段执行时机与 owner 分工** → 待 leader 安排后续独立会话；本轮不实施代码、不勾任一复选框。
- **验收线口径** → `openspec validate --all` / `python tools/verify_all.py` / `python tools/check_encoding.py` 本轮已跑通的部分如实登记；CI 双平台 `verify_all.py` 由人工 push 后触发、本轮不预先声称通过（C9）。
- **`spec_gate_check()` fail-closed** → `invalid spec gate status 'pending'` 属预期信号，不绕过、不修复、不冒充放行（C8）。

## 已知限制（如实登记，不冒充已解决、不扩范围）

- **p12 不纳入项**：顺延项（中立英雄 / 野怪 / 全部单位技能）与 Q1 逐项 SLK 对齐属 `p13-ability-full-roster`；真窗口 8.x 人工清单、golden hash 双平台、CI 双平台属人工 / VM 项；参考缺口、`vera20k` HEAD 前进、Q-A / 形态①时序澄清属 p13 或历史事实。本 change 一律不处理、不冒充已解决（锚：用户 2026-11-25 Scope 划分 / 本会话用户消息，仓库文件外证据 / 人工确认项）。
- **归档件字面原样保留**：P11 归档件 `design.md` 的 91 列、`tasks.md` 5.2 的歧义表述、主 spec 历史 `==8` 字面在归档件四件套内保持原样（零改动、不追溯改写，C6）；本 change 只在后续主 spec / 文档收口。
- **本轮 `git diff -- openspec/specs/` 为空**：主 spec 只在 archive 合并时复核或 apply 阶段改，proposal 只描述「要做什么」（C3 / C7）。
- **零确定性门禁**：本 change 不触 `w3-core` 确定性代码，不重跑 `cargo test -p w3-core --test determinism`、`CODEBUDDY.md §5` 形态①反向验证不适用，如实登记、不冒充全量门禁（D4）。
- **未新出 `reference-researcher` 对照报告**：本 change 判定不命中矩阵 T1–T7（C10），无报告、无借鉴、无参考缺口；该判定待规格门复核。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C10 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项已清空或显式豁免（本轮为提案成文期，批准记录 / 口径二选一 / 逐处判定等由规格门拍板，未自行收敛）。
3. 不以「预设问题问完」「问了 N 轮」当收敛；批准由用户拍板、不拿沉默当授权。
4. 「## 规格门」保持 `status: pending`、不 self-approve、不写「## 省略说明」。

## 规则文档同步

- 结论：**无需同步**。依据 `WC3-MELEE-RULES.md §14` 第 5 条：本 change 是**纯文档 / 规格口径订正 change**，不实现任何玩法、不触及任何 `R-x.y` 实现状态、不改 §13.7 现状、不改附录「规则 → 实现覆盖对照」任何行、不改 §15 变更记录、不改参考项目数量（仍五个）。三项订正分别是版本号消费者字面（ADR 0002 / ADR 0005）、本机导出表清单规模元数据（ADR 0001 / ADR 0004 / `§14`）、冻结定义哈希契约表述（ADR 0002 / ADR 0004），均非规则实现状态变更。
- MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」（`WC3-MELEE-RULES.md:393` 三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。真窗口 8.x 人工清单不在本 change 范围、不标通过。
- 结构门禁：`tools/verify_all.py::rule_doc_sync_check()` 仅对过规格门 `status: approved` 的玩法 change 校验本小节存在且非空；本 change `status: pending` 且非玩法（design.md 不承接任何 `R-x.y` 实现），豁免。本小节按 §14 第 5 条如实给出「无需同步」结论、保持非空；`decision-memo.md` 不在四件套指纹集合内（C7），补该小节不触发 §8 规格解锁。

## 规格门

- status: approved
- fingerprint: e81736e7fc587a1feba2a0866940a1ef62517a51f4c83ef72fe81ba291e50d31
- approved_by: 用户（2026-11-25 经 `AskUserQuestion` 拍板「批准，p13 先修订再锁定」；`critic` 三份规格门报告 `blocker = 0`、`reference-researcher` p13 对照报告已出，由 leader 按授权序列回填，非 AI self-approve、非沉默授权）
- date: 2026-11-25
- reason: 用户 2026-11-25 规格门拍板批准；`critic` 规格门报告 `blocker = 0`（四件套一致、三必填小节非空且锚可打开、T1–T7 不命中如实登记、P11 归档件锁定不追溯改写、机械门禁全绿）。四件套随本批只订正 `tasks.md:30` git 边界措辞（实为本轮成文范围内的三个 change 目录），按 `CODEBUDDY.md §8` 同步回查其余三份后重算指纹。

规格已锁定；`verify_all.py::spec_gate_check()` 机械校验 `status` 与指纹一致；此后改任一四件套 artifact 即视为解锁，MUST 按 `CODEBUDDY.md §8` 同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）。提案阶段「保持 pending / 由 leader 回填」等措辞为当时流程约束记录（沿用 P11 归档件体例），批准状态一律以本小节 `status: approved` 为准。
