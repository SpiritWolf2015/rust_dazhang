# 决策纪要 — `p16-reverse-engineering-workflow`

> 取料纪律：基于仓库 artifacts / 已由人拍板落地的合规措辞与 p15 四件套回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。取证流程的通用工程环节为本项目自定的流程说明、不附带外部资料行号；`craft-anything` 各文件行号均为**本机外部工具工作树快照 / 人工确认项 / 可能漂移**，不冒充已逐行核验。
> 合规边界：纯文档，不进状态哈希 / 快照、不含资产、不含本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 阶段：提案成文期（本轮只成文、不实施代码、不把文档落到 `docs/references/`，apply 是规格门批准后的后续独立会话）。用户 2026-11-25 已拍板「引入 clean-room 反编译证据流程」并落地 ADR 0003 第二次修订 / ADR 0001 复核记录 / `README.md` / `CONTRIBUTING.md` 合规措辞 / p15 四件套（commit `6bdf3ae`，已 push）；用户现在要求「落盘 参考目录 反编译流程说明 并走 规格门」。本 change 为**非玩法 / 纯文档交付（流程参考说明）** change，不串味 p12 / p13 / p14 / p15。**批准状态为 `pending`、由人拍板、AI 未 self-approve**。
> 结构说明：`## 规格门` 置于末尾且只有本节以「规格门」开头（`tools/verify_all.py::memo_section_body()` 按前缀匹配），`status` 字段以本节为准；本纪要不在四件套指纹集合内（`tools/verify_all.py:175-195`），改它不触发 `CODEBUDDY.md §8` 规格解锁。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | 用户 2026-11-25 已拍板「引入 clean-room 反编译证据流程」并已落地：`ADR 0003` 第二次修订、`ADR 0001` 复核记录、`README.md:29` / `CONTRIBUTING.md:11` 合规措辞、`p15-presentation-evidence-workflow` 四件套（`status: approved` + 四件套指纹锁定）；commit `6bdf3ae` 已 push | `docs/decisions/0003-scope.md:109-123`（第二次修订全文）/ `:111`（用户拍板）/ `:114`（形态 A）/ `:122`（p15 四件套成文属后续独立任务）；`docs/decisions/0001-license.md:77-83`（复核记录）；`README.md:29`（反编译产物与 MPQ 解包同等对待）、`CONTRIBUTING.md:11`；`openspec/changes/p15-presentation-evidence-workflow/decision-memo.md:100-104`（`status: approved` + 指纹）；用户本会话背景说明（**仓库文件外证据 / 人工确认项**） |
| C2 | 用户「现在要求」=「落盘 参考目录 反编译流程说明 并走 规格门」；本 change 是纯文档交付，apply 阶段**唯一产物** `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`，本轮不落盘；流程对照 = clean-room 取证工具（`craft-anything`，本机外部工具，MIT OR Apache-2.0，early alpha）↔ 本项目 p15 形态 A | 用户本会话任务边界（**仓库文件外证据 / 人工确认项**）；`openspec status --change p16-reverse-engineering-workflow --json` 输出；`openspec/changes/p15-presentation-evidence-workflow/proposal.md:11-16`（形态 A 定义）/ `specs/presentation-evidence-workflow/spec.md:3-5` |
| C3 | p15 定义「系统该做什么」，本 change 文档提供「怎么用的可照抄流程」：取证流程的通用工程环节 + `craft-anything` 产品化实现 + 逐项对照表 + 端到端 8 步流程 + 两条反馈循环 + 产物落点矩阵 + 诚实边界；取证产出的是**行为规格 / 证据节点，不是代码**，不是 WC3 技术事实来源 | `openspec/changes/p15-presentation-evidence-workflow/specs/presentation-evidence-workflow/spec.md:9-61`（ADDED Requirements 全文）/ `proposal.md:11-16`；`docs/decisions/0003-scope.md:113-116`（行为规格 / 形态 A / 单特性试点）；本项目自定的流程说明（不附带外部资料行号） |
| C4 | 红线（ADR 0001 / 0002 / 0003）：零资产入库、MPQ 派生数据不入库、反编译产物留本机仓库外、不绕 EULA / DRM / 登录 / 反作弊、仅合法自备经典客户端；反编译证据 MUST NOT 进权威状态 / 状态哈希 / 快照 / golden hash / determinism 验收 / CI 门禁（视觉 oracle，非业务逻辑 oracle）；表现层只读派生、`w3-core` 零改动、不 bump 三版本号、不新增依赖、不登记 `NOTICE.md` | `docs/decisions/0003-scope.md:117-120`（红线不变）/ `:103,113`（视觉 oracle）；`docs/decisions/0001-license.md:80-82`（复核记录「合规边界不变并新增一条」）；`README.md:29` / `CONTRIBUTING.md:11`；`docs/decisions/0002-determinism.md:49-53`（§5 状态权威与表现边界）；`openspec/config.yaml:9-25` |
| C5 | 三版本号权威值当前均为 **9**；本 change 零新增权威状态、不 bump、不重跑 `cargo test -p w3-core --test determinism`（`CODEBUDDY.md §5` 形态①不适用） | `crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13` / `CODEBUDDY.md §5`；本轮只读 `grep -n` 命令输出 |
| C6 | 取证流程的通用工程环节六小节：触发与目标收敛 / 生产侧 / 消费侧分离与文档交接 / 长程任务分类 / 并行与调度 / 证据结构化与独立重审 / 工程护栏；本节为本项目自定的流程说明，不引用、不转述任何外部内部文章、不附带外部资料行号，取证产出的是行为规格 / 证据节点而非代码 | `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`（第 1 节六小节标题 / 文档定位声明块「不引用、不转述任何外部内部文章」）；本项目工程治理约定（生产侧 / 消费侧只读文档交接 / 判定须经独立重审） |
| C7 | `craft-anything` 产品化实现证据锚：主循环 `Study→Build→Check→Ready`（`README.md:96-105`）；Clean room / Deep 双模式（`README.md:132-150`、`crates/engine/src/clean_room.rs:14-158`）；角色隔离（`AGENTS.md:23`、`docs/claude-code.md:18-25,46-60`）；证据工作记忆（`crates/setup/src/skill.md:38-58,61-74,107-108`、`docs/tools.md:7-44`：brief / plan / hypothesis / finding / handoff / journal、claim 挂 `ev_…` id、observed / derived / inferred、Absence is not proof、先预测再验证）；反编译工具组（`docs/tools.md:46-102`）；对照 / Check 工具组（`docs/tools.md:228-313`、`docs/file-formats.md:15-16`）；项目目录（`README.md:226-229`）与入口（`README.md:176-187,210-224`）；合规（`README.md:259-262,143-147`、`crates/setup/src/skill.md:89-95`）；反馈循环 `--continue-in` / `--rounds`（`README.md:222`） | `craft-anything` 工作树内路径行号（**本机外部工具 / 仓库文件外证据 / 人工确认项**，行号由用户任务清单给出、本轮未逐行打开，工作树 HEAD 可能前进、行号可能漂移）；p15 `design.md:143-149` 已只读抽查同类文件（`crates/formats/README.md:1-75` / `README.md:143-147,240-248` / `docs/claude-code.md:63`）；`p15-presentation-evidence-workflow/decision-memo.md:62,74-75`（待确认项⑤ / 已知限制） |
| C8 | `UPSTREAM.md` 矩阵 T1–T7 逐条核对，本 change 判定**均不命中**（T7 可写「可选交叉、仅红线」），与 `p14-ai-workflow-guardrails` 工作流护栏 change 同类；故承重约束**无一条来自参考项目**、无参考项目新增借鉴、无新增参考项目锚、无参考缺口 | `docs/references/UPSTREAM.md:104-114`（按任务类型的强制参考矩阵与交叉注记）/ `:9-15`（五个固定参考版本）/ `:96,100`（五个只读、不复制代码）；`openspec/changes/p14-ai-workflow-guardrails/design.md:100-113`（T1–T7 全不命中逐条判定体例）/ `decision-memo.md:20`（C9）；本 change `design.md`「## 参考项目对照」逐条判定；该判定待 `critic` 清单第 12 项复核 |
| C9 | 文档定位声明块体例沿用 `WC3-MELEE-RULES.md:1-13` / `UPSTREAM.md:1-5`：不是上游参考项目 / 五个只读参考项目仍在 `UPSTREAM.md` / 合规边界（ADR 0001）/ 确定性边界（ADR 0002）/ 状态标记 / 建立日期 2026-11-25 / 维护责任 `docs-owner` | `docs/references/WC3-MELEE-RULES.md:1-13`（文档定位 / 不是上游参考项目 / 合规 / 确定性 / 状态标记 / 建立日期 / 维护责任）；`docs/references/UPSTREAM.md:1-5`（本文件固定五个只读参考项目 + 区分注记）；`CODEBUDDY.md §9`（`docs-owner` 维护参考矩阵） |
| C10 | 产物落点矩阵依据：伪 C / 调用图 / 行为规格 / 离屏 RGBA / MPQ 派生数据 / 状态哈希逐项标「能否入库」「能否进权威状态或门禁」，依据 `ADR 0003:118-119` / `README.md:29` / `CONTRIBUTING.md:11` | `docs/decisions/0003-scope.md:118-119`（ADR 0001 / 0002 红线原文）；`README.md:29` / `CONTRIBUTING.md:11`；`docs/references/UPSTREAM.md:63-92`（导出数据不入库）；`docs/decisions/0002-determinism.md:49-53` |
| C11 | 诚实边界待登记项：`craft-anything` early alpha（`README.md:240-248`）、`docs/claude-code.md:63` 未 live 验证、Deep + Clean room 组合无先例（p15 待确认项⑤）、Clean room 不是 OS 沙箱（`README.md:143-147`）、无 MPQ / BLP / MDX / SLK 专用解析器（`crates/formats/README.md:1-75`）→ WC3 资产与数据表仍靠 `godot_warcraft3` 导出管线；反编译证据是视觉 oracle 不进 ADR 0002 门禁；`reference-researcher` 对照报告按 §9 只读体例未落盘为仓库文件；真窗口 8.x / golden hash v9 双平台 / CI 双平台待人工或 VM；**工具本体尚未运行 / p15 未 apply / 试点未验证闭环** | `craft-anything/README.md:143-147,240-248` / `docs/claude-code.md:63` / `crates/formats/README.md:1-75`（**仓库文件外证据 / 人工确认项**）；p15 `design.md:143-149`（缺口如实登记）/ `decision-memo.md:62,73-81`（待确认项⑤ / 已知限制）；`docs/references/UPSTREAM.md:108`（T3）/ `:51`（`godot_warcraft3` 资产管线）；`CODEBUDDY.md §9`（`reference-researcher` 只读、不写任何文件）/ §5（golden hash 双平台待验证） |
| C12 | 决策纪要三必填小节 = 承重约束 / 拍板决策 / 待确认项；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免；四件套指纹不含 `decision-memo.md`、不含主 `openspec/specs/**`、`RULE_REF_PATTERN` 只扫 `design.md` | `tools/verify_all.py:29`（`REQUIRED_MEMO_SECTIONS`）/ `:40`（`RULE_REF_PATTERN`）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（`spec_artifact_fingerprint()` 的 `paths` 与复选框归一化）/ `:207-253`（`spec_gate_check()`）/ `:256-294`（`rule_doc_sync_check()`，`design.md` 命中 `RULE_REF_PATTERN` 才视为玩法 change）/ `:297-315`（`fingerprint_cli()`）；`openspec/decision-memo.template.md:14,55-64`；p12 `decision-memo.md:69-77` / p14 `:79-87` / p15 `:98-106` 的「## 规格门」体例 |
| C13 | 规则文档同步是正式结构门禁（§14 第 5 条），`rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 生效；pending / 非玩法 change 豁免 | `docs/references/WC3-MELEE-RULES.md:424`（§14 第 5 条）/ `:426-437`（§15 变更记录）；`tools/verify_all.py:256-294` / `:38-40`；`openspec/decision-memo.template.md:48-53`；`CODEBUDDY.md §9`（规则文档同步正式规则）；`openspec/config.yaml:56,72` |
| C14 | 编码基线 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；中文文档；git 提交信息中文；AI 不得 push / fetch / pull / clone；零资产红线；不写本机真实绝对路径 / 资产 / 凭证；Windows 上禁止 `Path.write_text()` / `open(..., "w")` 默认模式（会把 LF 转 CRLF、破坏四件套指纹） | `CODEBUDDY.md §2` / §3 / §4 / §6；`openspec/config.yaml:34-40`；`tools/check_encoding.py:15-25,69-87`；`.codebuddy/agents/critic.md:21,26`；历史会话 `p9-undead-foundation` CRLF 漂移教训 |
| C15 | 本轮只成文、不实施代码；可写范围仅 `openspec/changes/p16-reverse-engineering-workflow/`；不碰 p12 / p13 / p14 / p15 / P11 归档件 / 主 spec / `docs/**` / `README.md` / `CONTRIBUTING.md` / `crates/**` / `tools/**` / `UPSTREAM.md` / `NOTICE.md` / `WC3-MELEE-RULES.md` / `openspec/specs/**`；不 commit / push / archive / 勾复选框 / self-approve / 写「## 省略说明」/ 写 `fingerprint` · `approved_by` · `date` · `reason` 字段行 | 用户本会话任务边界（**仓库文件外证据 / 人工确认项**）；本轮 `git status --short` 输出（工作区干净，仅新增 p16 目录）；`openspec new change p16-reverse-engineering-workflow` 与 `openspec status --change p16-reverse-engineering-workflow --json` 命令输出；`openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`（已 `status: approved` 锁定，改即触发 §8 规格解锁）；p15 四件套已锁定（C1） |

## 拍板决策（含被否备选与理由）

- **P1 新建独立纯文档 change `p16-reverse-engineering-workflow` 并落成完整四件套 + `decision-memo.md`，apply 阶段唯一产物为 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`（clean-room 取证工具 ↔ 本项目 p15 形态 A 对照的可照抄反编译使用流程说明），规格门保持 `status: pending`、本轮只成文不落盘**。锚：用户本会话明确要求「落盘 参考目录 反编译流程说明 并走 规格门」（**仓库文件外证据 / 人工确认项**）+ C1 已拍板已落地的合规措辞 / p15 四件套 / commit `6bdf3ae`；`openspec new change` / `openspec status --change ... --json` 命令输出。被否：① **把文档直接写进 p15 或 ADR 正文**（否决理由：职责分离——p15 定义「系统该做什么」、ADR 记「为什么决定」，本 change 提供「怎么用」的可照抄流程；且 p15 四件套已 `status: approved` + 指纹锁定，改任一 artifact 即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门，C1 / C15）；② **登记第六个只读参考项目**（否决理由：`UPSTREAM.md:9-15,96,100` + `CODEBUDDY.md §1` 固定五个只读参考项目矩阵；`craft-anything` 是本机外部工具，沿用 p14 `design.md:112` / p15 `design.md:154` 口径，C8 / C9）；③ **省略规格门**（否决理由：用户明确「走 规格门」，本 change 非「目标与方向已清楚的小改动」，且 `CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve，C12）。

> 措辞纪律：P1 为本轮**唯一**已拍板项（证据锚 = 用户本会话消息这一仓库文件外证据 / 人工确认项 + C1 仓库内 artifacts 明确记载的既成事实），MUST NOT 写成 AI 自定 / AI 推荐 / AI 已确认 / 沉默授权。**用户本轮拍板的是「落盘参考目录流程说明 + 走规格门」这一交付形态**；其余文档章节粒度 / 是否建索引 / 证据锚行号漂移 / apply 时机与 owner 分工 / 规格门批准记录一律进「待确认项」，不臆造为已定案。特别是：MUST NOT 把「引入工作流」写成「已引入代码依赖 / 已放宽红线 / 已过规格门 / 已获暴雪授权 / 已试点完成 / 已落盘文档」；MUST NOT 把 `craft-anything` early alpha / Clean room 不是 OS 沙箱 / Deep + Clean room 组合无 live 验证先例写成「已验证可用」；MUST NOT 把 `craft-anything` 工具行号写成「已逐行核验」；通用工程环节不附带外部资料行号。

## 被否方向台账

> 编号说明：本小节 `R1`–`R6` 是「被否方向」台账编号（沿用 p12 / p14 / p15 `decision-memo.md`「## 被否方向台账」体例），**不是** `docs/references/WC3-MELEE-RULES.md` 的玩法规则编号；本 change 为非玩法 change，`design.md` 内**不出现**该 `R\d+` 台账编号（避免 `RULE_REF_PATTERN` 误判，C12），`design.md` 不承接任何玩法规则编号的实现需求。

- **R1 把流程说明直接写进 p15 四件套或 ADR 0001 / 0002 / 0003 正文** → p15 已 `status: approved` 指纹锁定、改即触发 §8 规格解锁；ADR 记「为什么」、spec 记「该做什么」，流程说明是「怎么用」的第三类文档（C1 / C15 / P1 被否①）。
- **R2 把 `craft-anything` 登记为第六个只读参考项目，或改 `UPSTREAM.md` 五个参考项目表格 / `NOTICE.md`** → `UPSTREAM.md:9-15,96,100` 五个只读矩阵；沿用 p14 / p15 口径（C8 / C9 / P1 被否②）。
- **R3 省略规格门、写「## 省略说明」求门禁变绿，或自填 `status: approved` + 指纹 / `approved_by` / `date` / `reason`** → 用户明确「走规格门」；`CODEBUDDY.md §8` / §9 由人拍板、AI 不得 self-approve；本 change 非小改动豁免场景（C12 / C15 / P1 被否③）。
- **R4 本轮直接把 `REVERSE-ENGINEERING-WORKFLOW.md` 落到 `docs/references/`，或顺手在 `UPSTREAM.md` / `README.md` 加索引** → 用户明确本轮只成文、文档落盘是规格门批准后的 apply 动作；是否建索引属待确认项（C2 / C15）。
- **R5 把反编译产物（伪 C / 调用图 / 行为规格 / 抓帧证据）、MPQ 派生 JSON / PNG / GLB、资产或凭证写进文档 / 仓库，或把 `craft-anything` 加为依赖 / 接 CI / 登记 `NOTICE.md`** → C4 / C10 / ADR 0001 复核记录 `0001-license.md:80-82`；形态 A 明文「不入库 / 不复制代码 / 不作为 crate 依赖」（`0003-scope.md:114`）。
- **R6 把视觉 oracle / 行为规格当 ADR 0002 口径的业务逻辑 oracle，据此改 `w3-core` 权威状态 / 状态哈希 / 快照 / golden hash / determinism 门禁 / CI，或 bump 三版本号 / 改业务规则 / 追溯改写 P11 归档件** → C4 / C5 / `0003-scope.md:103,119`；`CODEBUDDY.md §5` 形态①不适用、不臆造内核改动；真窗口 8.x / golden hash v9 双平台 / CI 双平台一律「待人工 / 待 VM」、不冒充通过。

## 待确认项（如实标注，不冒充已拍板）

> 以下全部由规格门用户拍板（不拿沉默当授权）；无证据项一律标「待确认」，MUST NOT 写成「用户已确认」。本节非空。

- **文档章节粒度** → 8 个章节（定位声明块 / 取证流程的通用工程环节 / `craft-anything` 产品化 / 逐项对照表约 10 行 / 端到端 8 步 / 两条反馈循环 / 产物落点矩阵 / 诚实边界）的详略、逐项对照表确切行数、8 步「输入 · 工具 · 产物 · 落点 · 边界」五要素的字段口径由规格门用户拍板；spec 用行为式 SHALL、tasks 用条件式措辞，粒度二选一不推翻 spec requirement 与 task 结构。
- **是否在 `UPSTREAM.md` 或 `README.md` 加索引** → 本轮 Why / What Changes 明确「不建索引」；若用户后续要索引，属 apply 阶段动作且须先改本 change 四件套（触发 §8 规格解锁重走规格门），待规格门用户拍板；不臆造为已定案。
- **`craft-anything` 证据锚行号漂移** → C7 行号基于本机外部工具工作树（非仓库文件 / 人工确认项），HEAD 可能前进、行号可能漂移；取证流程的通用工程环节为本项目自定流程说明、不附带外部资料行号；是否需要在 apply 阶段重新只读核对工具行号一遍，待规格门用户拍板。
- **apply 阶段执行时机与 owner 分工** → 一个 change 一个 owner（纯文档交付，落点在 `docs/references/`，疑属 `docs-owner` / `spec-steward` 边界，未拍板）；由 leader 安排后续独立会话；本轮不实施代码、不落盘文档、不勾任一复选框（C15）。
- **规格门批准记录回填** → 「## 规格门」保持 `status: pending`；四件套指纹（`python tools/verify_all.py --fingerprint p16-reverse-engineering-workflow`）、`critic` 规格门报告、批准人 / 日期由 leader 在用户拍板后回填，本轮不碰、AI 未 self-approve（C12）。
- **参考矩阵命中判定复核** → 本轮据 `UPSTREAM.md:104-114` 判定 T1–T7 均不命中（C8 / `design.md`「## 参考项目对照」），无 `reference-researcher` 对照报告、无参考缺口；该判定待 `critic` 清单第 12 项复核；若改判命中则须补对照报告并回到用户拍板，不臆造。
- **`spec_gate_check()` fail-closed** → `invalid spec gate status 'pending'` 属预期信号，不绕过、不修复、不冒充放行（C12）。
- **验收线口径** → `openspec validate p16-reverse-engineering-workflow --strict` / `python tools/check_encoding.py` 本轮已跑通的部分如实登记；完整 `python tools/verify_all.py` 由 leader 跑、本轮不跑；`cargo test --workspace` / CI 双平台属 apply 阶段与人工 / VM 项，不预先声称通过（C14）。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C15 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项已清空或显式豁免（本轮为提案成文期，章节粒度 / 索引 / 行号漂移 / apply 分工 / 批准记录由规格门用户拍板，未自行收敛）。
3. 不以「预设问题问完」「问了 N 轮」当收敛；批准由用户拍板、不拿沉默当授权。
4. 「## 规格门」保持 `status: pending`、仅 `- status: pending` + 一段说明、不 self-approve、不写「## 省略说明」、不填指纹 / `approved_by` / `date` / `reason` 字段行。
5. 不把「引入工作流」写成已引入代码依赖 / 已放宽红线 / 已过规格门 / 已获暴雪授权 / 已试点完成 / 已落盘文档；不把参考缺口写成已有答案；不把 `craft-anything` 工具行号写成已核验。

## 已知限制（如实登记，不冒充已解决、不扩范围）

- **取证流程的通用工程环节为本项目自定的流程说明**：不引用、不转述任何外部内部文章、不附带外部资料行号；生产侧 / 消费侧只读文档交接、判定与产物同提交 + 独立重审、证据结构化均为本项目自定的工程治理约定，不复制代码、不引入其技术栈、不作 WC3 玩法 / 格式 / 数值事实来源、不登记为第六个参考项目（C6 / C9）。
- **`craft-anything` 工作树 HEAD 可能前进**：C7 各 `README.md` / `docs/*.md` / `crates/**` 行号是当时工作树的快照（本机外部工具 / 仓库文件外证据 / 人工确认项），非仓库内可机械核验的 `file:line`；行号漂移进「待确认项」，不冒充固定锚。
- **early alpha 与无 live 验证先例**：`README.md:240-248` 自述 early alpha、部分引擎工具只测过 stand-ins；`docs/claude-code.md:63`「Not yet checked live against Claude Code or an ACP agent」；「Deep 模式 + Clean room 角色隔离」组合、是否覆盖本机经典客户端无 live 验证先例（p15 待确认项⑤）；Clean room 不是 OS 沙箱（`README.md:143-147`），合规姿态仍靠 ADR 0001 / 0002 红线与人工逐条判定（C11）。
- **无 MPQ / BLP / MDX / SLK 专用解析器**：`crates/formats/README.md:1-75` 格式清单无专用解析器，WC3 资产与数据表仍靠 `godot_warcraft3` 导出管线（`UPSTREAM.md:51,108`）；反编译证据是视觉 oracle、不进 ADR 0002 门禁（C4 / C11）。
- **`reference-researcher` 对照报告未落盘**：按 `CODEBUDDY.md §9` 只读体例，对照报告为对话内文本、未落盘为仓库文件；本 change 判定 T1–T7 均不命中、本轮未委派、无报告、无借鉴、无参考缺口（C8）；该判定待 `critic` 清单第 12 项复核。
- **工具本体尚未运行 / p15 未 apply / 试点未验证闭环**：p15 四件套虽已 `status: approved` 锁定，但 apply 未执行、clean-room 反编译证据流程未端到端跑通，本 change 只是「怎么用」的流程参考说明，不冒充工具已验证可用 / 试点已闭环（C1 / C11）。
- **真窗口 / 双平台 / CI 待人工或 VM**：真窗口 8.x 人工清单、golden hash v9 双平台逐字节比对（Debian 12 / macOS / ARM64 / 其它 rustc）、CI 双平台 `verify_all.py` 一律「待人工 / 待 VM」，不冒充通过、不得据本流程说明声称跨平台确定性已达成（ADR 0002 待验证项，C5 / C11）。
- **文档未在 `README.md` / `UPSTREAM.md` 建索引**：本轮 Why / What Changes 明确不建索引、本轮文档尚未落盘；是否在两处加索引属待确认项，如实登记为已知限制、不冒充已对齐（C2 / C9）。
- **零确定性门禁**：本 change 不触 `w3-core` 确定性代码，不重跑 `cargo test -p w3-core --test determinism`、`CODEBUDDY.md §5` 硬门禁与形态①反向验证不适用，如实登记、不冒充全量门禁（C5 / `design.md` D5）。
- **本轮 `git diff` 边界**：`-- openspec/specs/` / `-- docs/` / `-- crates/` / `-- tools/` / `-- README.md` / `-- CONTRIBUTING.md` 为空；只新增 p16 目录内文件；主 specs 是否变化以 leader 归档合并时复核（C15 / `proposal.md` Capabilities 注释）。

## 规则文档同步

- 结论：**无需同步**。依据 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条：本 change 是**非玩法 / 纯文档（反编译使用流程参考说明）change**，不实现任何玩法、不触及任何玩法规则编号的实现状态、不改 §13.7 现状、不改附录「规则 → 实现覆盖对照」任何行、不改 §15 变更记录、不改参考项目数量（仍五个）。本 change 只新增 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 一份流程说明（取证流程的通用工程环节 / 8 步流程 / 产物落点矩阵 / 诚实边界），均非玩法规则实现状态变更；proposal / specs / tasks 不承接任何玩法规则编号的实现需求（C4 / C8 / C13）。
- MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」（`WC3-MELEE-RULES.md:393-395` 三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。`design.md` 内不出现 `R\d+` 形式的台账编号（C12），避免 `RULE_REF_PATTERN` 把本非玩法 change 误判为玩法 change。真窗口 8.x 人工清单、golden hash 双平台、CI 双平台不在本 change 范围、不标通过。
- 结构门禁：`tools/verify_all.py::rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 校验本小节存在且非空；本 change `status: pending` 且非玩法（`design.md` 不承接任何规则编号实现、不含 `R\d+` 命中），豁免。本小节按 §14 第 5 条如实给出「无需同步」结论、保持非空；`decision-memo.md` 不在四件套指纹集合内（C12），补该小节不触发 §8 规格解锁。结论语义真实性由 `critic` 清单第 13 项复核，机械门禁只校验结构、不冒充语义验证。

## 规格门

- status: approved
- fingerprint: 4281332461e1dba3f9f8c7ac27232e2ebef3ae6a7b5245c98d643f993c5ed07e
- approved_by: 用户（2026-11-25 经 `AskUserQuestion` 拍板「批准并落盘 + 代码门 + 提交」；`critic` 规格门报告 `blocker = 0`、7 项全通过，判定 A/B/C/D 均成立，由 leader 按授权序列回填，非 AI self-approve、非沉默授权）
- date: 2026-11-25
- reason: 用户 2026-11-25 要求「落盘参考目录反编译流程说明并走规格门」，明确选择完整规格门串联而非 §8 省略。`critic` 规格门报告 7 项全通过、`blocker = 0`：A（文档本轮未落盘、仅以 9 条 requirement / 13 个可执行 Scenario 定义章节与验证，属「批准 → apply」的正常形态，非 blocker）、B（T1–T7 均不命中、无独立 `reference-researcher` 报告符合 §9 条件式要求，p12/p14 先例）、C（拍板前置齐备）、D（完整串联与 §8/§9 自洽、与 p15 已锁定指纹 `d5ab6b5e…` 无冲突）。批准即收敛 1.1 / 1.2 / 1.3 三项规格门待确认项：章节粒度按 spec 的 SHALL requirement 与 8 章节定案（不另立口径）；索引按 proposal「不建索引」取舍、后续若需索引 MUST 先改四件套并重走规格门；取证流程的通用工程环节为本项目自定流程说明、不附带外部资料行号；`craft-anything` 证据锚一律标「仓库文件外证据 / 人工确认项 / 可能漂移」，不写成仓库内 `file:line` 或已逐行核验；T1–T7 判定已经 `critic` 清单第 12 项复核、五个只读参考项目不变。四件套自批准起锁定、指纹如上；提案阶段「保持 pending / 由 leader 回填 / apply 属后续独立会话」等措辞为当时的流程约束记录（沿用 P11 归档件 / p12 / p14 / p15 体例），批准状态一律以本小节 `status: approved` 为准；此后改任一四件套 artifact 即视为解锁，MUST 按 `CODEBUDDY.md §8` 同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）。真窗口 8.x / golden hash v9 双平台 / CI 双平台仍属「待人工 / 待 VM」，不冒充通过；工具本体尚未运行、p15 未 apply、试点未验证闭环。 2026-11-26 按用户拍板移除对内涉密外部文章内容，四件套做最小清除改写后重算指纹并重新锁定；本次为合规处置、非玩法 / 非范围变更，由用户 2026-11-26 明确授权，未重走完整 §9 规格门（如实标注、不冒充已过规格门）

规格已锁定；`verify_all.py::spec_gate_check()` 机械校验 `status` 与四件套指纹一致。本 change 为非玩法 change，`rule_doc_sync_check()` 对非玩法 change 豁免。文档 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 为本小节批准后的 apply 产物，由 `docs-owner` 落盘并经 `critic` 代码门按 A 判定清单复核。
