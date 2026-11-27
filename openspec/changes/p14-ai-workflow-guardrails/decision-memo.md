# 决策纪要 — `p14-ai-workflow-guardrails`

> 取料纪律：基于仓库 artifacts 回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。
> 合规边界：纯文档，不进状态哈希 / 快照、不含资产、不含本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 阶段：提案成文期（本轮只成文、不实施代码，apply 是后续独立会话）。用户 2026-11-25 明确拍板「新提独立工作流 change 并落成四件套」，本 change 为**非玩法 / 工作流治理（门禁与规则治理）**独立 change，不串味 p12 / p13。**批准状态为 `pending`、由人拍板、AI 未 self-approve**。
> 结构说明：`## 规格门` 置于末尾且只有本节以「规格门」开头（`tools/verify_all.py::memo_section_body()` 按前缀匹配），`status` 字段以本节为准；本纪要不在四件套指纹集合内（`tools/verify_all.py:175-195`），改它不触发 `CODEBUDDY.md §8` 规格解锁。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | 三版本号权威值当前均为 **9**（`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`）；本 change 不 bump、不新增权威状态 | `crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13` / `CODEBUDDY.md §5`；本轮只读 Grep 命令输出 |
| C2 | 现有门禁体例与挂载点：`verify_all.py` 的 `main()` 是 `run(...)` 链 + 资产扫描 / 决策纪要 / 规格门 / 规则文档同步；`openspec_command()` 用 `os.name` 判平台差异是跨平台脚本先例 | `tools/verify_all.py:318-348`（`main()`）/ `:66-74`（`os.name` 分支）/ `:88-108`（`asset_scan()`）；`tools/check_encoding.py:15-25,37-51`（UTF-8/LF/未跟踪文件覆盖）；`tools/check_source_conventions.py`（本轮 `ls tools/*.py` 输出）；`CODEBUDDY.md §2` / §9 |
| C3 | 临时约束被永久化的本项目真实实例：P8.1 当期「MUST NOT 新增 `EffectKind` 变体」被当永久门禁，P11 需专门澄清「被取代」并改过期注释 | `crates/w3-client/src/melee/defs.rs:22`（模块头「当期…是 P8.1 change 内约束、非永久」）/ `:919`（「已被 P11 授权扩展取代」）/ `:2290-2292`（「穷尽匹配四个既有变体…非永久门禁」）；`openspec/changes/archive/2026-11-25-p11-ability-system/design.md:11`（关键约束取代）/ `:169-173`（结论 4）/ `:236`（Risk）；本轮只读 Grep 命令输出 |
| C4 | 两条护栏是**本项目自定的工程治理约定**（代码膨胀预算门禁 / 临时约束时效与作用域 / 判定须经独立重审 / 显式失败），不复制任何外部代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源；阈值 MUST 由本项目基线测量得出、不冻结任何外部项目数字 | 用户 2026-11-25 任务边界（本会话用户消息 / leader 转达，**仓库文件外证据 / 人工确认项**）；`CODEBUDDY.md §1`（参考项目只读五个）/ §6（零资产）/ ADR 0001；`design.md` Context 护栏工程依据 |
| C5 | 护栏 2 的规则文字落点：`config.yaml` 的 `rules.*`、`CODEBUDDY.md §8`、`critic` 清单第 7/10 项；本轮只描述「apply 阶段将改」，不改这些权威文件 | `openspec/config.yaml:50-72`（`rules.proposal/specs/design/tasks`）；`CODEBUDDY.md:80-90`（§8）；`.codebuddy/agents/critic.md:25`（第 7 项规范一致性 / spec 引用 ADR 编号）/ `:28`（第 10 项 spec 与决策纪要质量）；本轮 `git status --short` 命令输出 |
| C6 | 规则文档同步是正式结构门禁（§14 第 5 条），`rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 生效；pending / 非玩法 change 豁免 | `docs/references/WC3-MELEE-RULES.md:424`（§14 第 5 条）/ `:393-395`（三态判定 / 覆盖对照）；`tools/verify_all.py:256-294`（`rule_doc_sync_check()`）/ `:38-40`（`RULE_REF_PATTERN`）；`openspec/decision-memo.template.md:48-53`；`CODEBUDDY.md §9` |
| C7 | 决策纪要三必填小节 = 承重约束 / 拍板决策 / 待确认项；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed；`rule_doc_sync_check()` 对 pending / 非玩法豁免；四件套指纹不含 `decision-memo.md` | `tools/verify_all.py:29`（`REQUIRED_MEMO_SECTIONS`）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（`spec_artifact_fingerprint()`）/ `:207-253`（`spec_gate_check()`）/ `:256-294`；`openspec/decision-memo.template.md:55-64` |
| C8 | 编码基线 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；工具脚本跨平台（Python/Node、`os.name`、不写 `.ps1`/`.sh`/`.bat`/`.cmd`）、注释中文 / 输出英文；git 提交信息中文；AI 不得 push/fetch/pull/clone；零资产红线 | `CODEBUDDY.md §2` / §3 / §4 / §6；`openspec/config.yaml:34-40`；`tools/check_encoding.py:15-25,69-87`；`.codebuddy/agents/critic.md:21,26` |
| C9 | 本 change 经核对 `UPSTREAM.md` 矩阵 **T1–T7 均不命中**（纯工程治理，不改内核 / 格式 / 玩法 / 场景 / 网络 / 通用架构实现），故承重约束**无一条来自参考项目**、无参考项目锚 / 参考缺口需要双标、不派 `reference-researcher` | `docs/references/UPSTREAM.md:104-112`（按任务类型的强制参考矩阵）/ `:96,100`（五个只读参考项目 / 不复制代码）；`design.md`「## 参考项目对照」逐条判定；该判定待 `critic` 清单第 12 项复核 |
| C10 | 规格门由人拍板、AI 不得 self-approve；pending 体例沿用 p12 / p13；一个 change 一个 owner、`spec-steward` 归档合并复核约束是否被后续 change 取代 | `CODEBUDDY.md §8`（成文顺序 / 四件套一致性 / 规格门）/ §9（串联 / 规格锁定与重审 / 一个 change 一个 owner）；p12 `decision-memo.md:69-73` / p13 `decision-memo.md:80-84` 的「## 规格门」pending 体例；`openspec/decision-memo.template.md:14` |
| C11 | 四件套指纹集合仅含 change 目录内 `proposal.md` / `design.md` / `specs/**/*.md` / `tasks.md`，**不含**主 `openspec/specs/**`、不含 `decision-memo.md`；`tasks.md` 复选框翻转被归一化、不触发解锁 | `tools/verify_all.py:175-195`（`spec_artifact_fingerprint()` 的 `paths` 与 `re.sub` 归一化）；p12 `decision-memo.md` C7；`CODEBUDDY.md §8` 规格锁定 |
| C12 | 本轮只成文、不实施代码；可写范围仅 `openspec/changes/p14-ai-workflow-guardrails/`；不碰 p12 / p13 / 归档件 / 主 spec / 其它目录；阈值数值与二选一未拍板 | 用户 2026-11-25 任务边界（本会话用户消息 / leader 转达，**仓库文件外证据 / 人工确认项**）；本轮 `git status --short` 输出（p12/p13/p14 三个 `??`）；`openspec status --change p14-ai-workflow-guardrails --json` 输出 |

## 拍板决策（含被否备选与理由）

- **P1 新提独立工作流 change `p14-ai-workflow-guardrails` 并落成完整四件套 + `decision-memo.md`，Scope 严格单一职责（代码膨胀预算门禁 + 临时约束时效规范两条护栏）、本轮只成文不实施代码**。锚：用户 2026-11-25 明确拍板「新提独立工作流 change 并落成四件套」（本会话用户消息 / leader 转达，**仓库文件外证据 / 人工确认项**）。被否：① 把两条护栏并入 p12 / p13（理由：用户明确「独立 change」，且本 change 零内核改动、非玩法，与 p12 文档订正 / p13 玩法全量的 owner 与门禁边界不同，不串味）；② 本轮直接改 `CODEBUDDY.md` / `config.yaml` / `critic.md` / `tools/**` 落地规则（理由：本轮只成文、可写范围仅 p14 目录，规则文字是 apply 阶段动作）；③ 把任何外部资料登记成第六个参考项目或引入其技术栈（理由：`CODEBUDDY.md §1` / `UPSTREAM.md` 参考项目只读五个、ADR 0001 零资产、护栏为本项目自定约定，C4）。

> 措辞纪律：P1 为本轮**唯一**已拍板项（证据锚 = 本会话用户消息 / leader 转达，仓库文件外证据 / 人工确认项），MUST NOT 写成 AI 自定 / AI 推荐 / AI 已确认 / 沉默授权。**本轮只拍板了「新提独立工作流 change 并落成四件套」**；其余范围 / 方案 / 阈值数值 / 门禁严格度 / 挂载 / CI / 验收线 / owner 分工 / 规格门批准记录一律进「待确认项」，不臆造为已定案。

## 被否方向台账

> 编号说明：本小节的 `R1`–`R6` 是「被否方向」台账编号（沿用 p12 / p13 `decision-memo.md`「## 被否方向台账」体例），**不是** `docs/references/WC3-MELEE-RULES.md` 的玩法规则编号 `R-x.y`；本 change 为非玩法 change，`design.md` 无 `RULE_REF_PATTERN` 命中、未引用任何玩法规则编号，proposal / specs / tasks 中出现的 `R-x.y` 均为「不引用 / 不实现任何玩法规则」的否定式声明，不作需求依据。

- **R1 把任何外部项目规模数字冻结成本项目阈值或写进 spec / 权威文件** → 阈值须由本项目基线测量得出、非外部实测（C4）；用户明确「先做基线测量、阈值进待确认项、不臆造外部数字」。若前提后续变化 MUST 整体重估，不沿用旧否决。
- **R2 把门禁脚本「改坏验红自证」措辞混淆为 `CODEBUDDY.md §5` 形态①反向验证，进而改 `w3-core` / 重跑 determinism** → §5 形态①专指 `w3-core` 权威状态 / 帧序 / 命令排序 / 状态哈希 / 快照的回归测试；本 change 零内核改动（C1）、`CODEBUDDY.md §5` 不适用（`design.md` D4 / D7），不臆造内核改动。
- **R3 自填 `status: approved` + 指纹，或写「## 省略说明」求门禁变绿** → `CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve；本 change 非「目标与方向已清楚的小改动」，豁免会违反 fail-closed 信号（C7 / C10）。
- **R4 把外部资料当 WC3 玩法 / 格式 / 数值事实来源，或复制其代码 / 引入外部技术栈** → C4 / ADR 0001 / `UPSTREAM.md` 五个只读参考项目；本 change 只落自定护栏。
- **R5 全局给所有 `MUST NOT` 加失效条件、把 ADR 0002 / §5 永久硬红线误标为临时约束** → `design.md` D5 备选④被否：惯例只针对「当期 change 内约束」，永久硬红线 MUST NOT 被误标。
- **R6 顺手 bump 三版本号 / 改 ADR 正文 / 新增第三方依赖 / 纳入真窗口人工验收或 golden hash 双平台** → 超 Scope、用户明确不改 ADR 正文、不 bump、不新增依赖；真窗口 8.x / golden hash / CI 双平台属人工 / VM 项（C1 / C8，`design.md` Non-Goals）。

## 待确认项（如实标注，不冒充已拍板）

- **阈值具体数值（四类指标：总行数 / 单文件行数 / 函数体行数 / 模块文件数）** → 待 apply 阶段基线测量（task 1.1）产出当前 `crates/` 规模分布后，由规格门用户拍板；不臆造外部项目数字（C4 / D2）。
- **硬阈值门禁 vs 仅文档约定二选一** → 待规格门用户拍板；spec 用行为式 SHALL、tasks 用条件式措辞，二选一不推翻 spec requirement 与 task 结构（`design.md` D6）。
- **是否挂进 `tools/verify_all.py`**（一行 `run(...)` 挂载） → 待规格门用户拍板；成文只写「SHALL 可挂进」（能力），不臆造「已挂载」（D6 / task 2.2）。
- **是否纳入 CI 双平台**（`.github/workflows/ci.yml` 的 `windows-latest` / `ubuntu-latest` matrix） → 待规格门用户拍板；CI 由人工 push 后触发，本轮不预先声称通过（C8 / task 2.3 / 4.4）。
- **护栏为本项目自定的工程治理约定** → 阈值与适用范围由规格门用户拍板（不臆造、不冒充已核验）；MUST NOT 登记为第六个参考项目（C4 / C9）。
- **apply 阶段执行时机与 owner 分工** → 一个 change 一个 owner、不同时改多 owner 文件；`tools/**`（脚本与挂载）/ `config.yaml` + `CODEBUDDY.md` + `critic.md`（规则文字）/ CI 各自 owner 由 leader 安排后续独立会话（C10）；本轮不实施代码、不勾任一复选框。
- **规格门批准记录回填** → 「## 规格门」保持 `status: pending`；四件套指纹（`python tools/verify_all.py --fingerprint p14-ai-workflow-guardrails`）、`critic` 规格门报告、`reference-researcher` 对照报告（本 change 判定不命中矩阵、无报告，待 `critic` 第 12 项复核该判定）、`approved_by` / `date` 均由 leader / 用户后续拍板回填，本轮不碰、AI 未 self-approve（C7 / C10 / C11）。
- **函数体行数统计口径与是否纳入 `tools/*.py`** → 轻量启发式「粗统计」口径与统计范围待规格门拍板，可能有宏 / 多行签名误报，如实登记为已知限制、不冒充精确（`design.md` D1 / D3）。
- **`openspec validate --strict` 的 >500 字 WARNING** → 本轮已控制两条 requirement 描述为 332 / 346 字符；若 CLI 仍报 WARNING 则如实标注、不冒充零告警（验收项）。
- **`spec_gate_check()` fail-closed** → `invalid spec gate status 'pending'` 属预期信号，不绕过、不修复、不冒充放行（C7）。
- **验收线口径** → `openspec validate p14-ai-workflow-guardrails --strict` / `python tools/check_encoding.py` / `python -c "...decision_memo_check()"` 本轮已跑通的部分如实登记；脚本跨平台自证（4.4）/ `cargo test --workspace` / `python tools/verify_all.py` 全绿 / CI 双平台属 apply 阶段与人工 / VM 项，不预先声称通过（C8）。

## 已知限制（如实登记，不冒充已解决、不扩范围）

- **阈值与二选一未定案**：本轮只成文，阈值数值 / 硬门禁 vs 文档约定 / 是否挂载 / 是否进 CI 一律在「待确认项」，由规格门拍板；脚本在阈值拍板前只能测量 / 报告、不硬失败（`design.md` D2 / D6）。
- **规则文字惯例无机械强校验**：护栏 2 是文字约定，机械门禁无法强校验「是否标注作用域与失效条件」，只能靠 `critic` 第 7 / 10 项内容层复核（`design.md` D5 / Risks），如实登记为结构性改善、非机械保证（判定须经独立重审）。
- **函数体行数为启发式粗统计**：不引第三方 AST 解析（零新依赖），宏 / 多行签名可能误报，脚本注释声明口径、不冒充精确（D3）。
- **零确定性门禁**：本 change 不触 `w3-core` 确定性代码，不重跑 `cargo test -p w3-core --test determinism`、`CODEBUDDY.md §5` 硬门禁与形态①反向验证不适用，如实登记、不冒充全量门禁（`design.md` D7 / C1）。
- **未新出 `reference-researcher` 对照报告**：本 change 判定不命中矩阵 T1–T7（C9），无报告、无借鉴、无参考缺口；该判定待规格门 / `critic` 第 12 项复核。
- **本轮 `git diff -- openspec/specs/` / `-- docs/` / `-- crates/**` / `-- tools/**` 为空**：本 change 只新增一个 ADDED capability delta、不改主 spec；apply 阶段才动 `tools/**` / 规则文字（C12 / `proposal.md` Capabilities 注释）。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C12 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项已清空或显式豁免（本轮为提案成文期，阈值 / 二选一 / 挂载 / CI / 人工确认项 / owner 分工 / 批准记录由规格门用户拍板，未自行收敛）。
3. 不以「预设问题问完」「问了 N 轮」当收敛；批准由用户拍板、不拿沉默当授权。
4. 「## 规格门」保持 `status: pending`、不 self-approve、不写「## 省略说明」、不填指纹 / `approved_by` / `date`。
5. 不把外部项目数字冒充本项目阈值、不把门禁改坏验红自证冒充 §5 形态①、不把引用编号冒充规则已实现。

## 规则文档同步

- 结论：**无需同步**。依据 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条：本 change 是**非玩法 / 工作流治理（门禁与规则治理）change**，不实现任何玩法、不触及任何 `R-x.y` 实现状态、不改 §13.7 现状、不改附录「规则 → 实现覆盖对照」任何行、不改 §15 变更记录、不改参考项目数量（仍五个）。两条护栏分别是代码膨胀预算门禁（`tools/*.py` 只读统计）与临时约束时效规范（规则文字），均非玩法规则实现状态变更；proposal / specs / tasks 不承接任何 `R-x.y` 实现需求（C6 / C9）。
- MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」（`WC3-MELEE-RULES.md:393-395` 三态判定 / §14 第 5 条 / `critic.md:32,34` 清单第 13 项）。真窗口 8.x 人工清单、golden hash 双平台、Q1 清单水位不在本 change 范围、不标通过。
- 结构门禁：`tools/verify_all.py::rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 校验本小节存在且非空；本 change `status: pending` 且非玩法（`design.md` 不承接任何 `R-x.y` 实现），豁免。本小节按 §14 第 5 条如实给出「无需同步」结论、保持非空；`decision-memo.md` 不在四件套指纹集合内（C11），补该小节不触发 §8 规格解锁。结论语义真实性由 `critic` 清单第 13 项复核，机械门禁只校验结构、不冒充语义验证。

## 规格门

- status: approved
- fingerprint: e49889a1621b335ee03179a325f250737f1ec42f2d762850316bf1d21245a856
- approved_by: 用户（2026-11-25 经 `AskUserQuestion` 拍板「批准，p13 先修订再锁定」；`critic` 三份规格门报告 `blocker = 0`、`reference-researcher` p13 对照报告已出，由 leader 按授权序列回填，非 AI self-approve、非沉默授权）
- date: 2026-11-25
- reason: 用户 2026-11-25 规格门拍板批准；`critic` 规格门报告 `blocker = 0`（四件套一致、T1–T7 不命中如实登记含五个只读参考项目不变、零 `w3-core` / 不 bump / 不新增依赖、机械门禁全绿）。四件套无待回填项、未改，指纹为成文态值。 2026-11-26 按用户拍板移除对内涉密外部文章内容，四件套做最小清除改写后重算指纹并重新锁定；本次为合规处置、非玩法 / 非范围变更，由用户 2026-11-26 明确授权，未重走完整 §9 规格门（如实标注、不冒充已过规格门）

规格已锁定；`verify_all.py::spec_gate_check()` 机械校验 `status` 与指纹一致；此后改任一四件套 artifact 即视为解锁，MUST 按 `CODEBUDDY.md §8` 同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）。提案阶段「保持 pending / 由 leader 回填」等措辞为当时流程约束记录（沿用 P11 归档件体例），批准状态一律以本小节 `status: approved` 为准。
