# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「两条护栏怎么落地」。事实来源全部是**本仓库 artifacts**（可核验锚，非参考项目结论）：

- **现有门禁体例**：`tools/check_encoding.py`（编码基线，UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格，覆盖未跟踪文件）、`tools/check_source_conventions.py`、`tools/verify_all.py:318-348`（`main()` 的 `run(...)` 链 + `asset_scan()` / `decision_memo_check()` / `spec_gate_check()` / `rule_doc_sync_check()`）。护栏 1 的挂载点就是 `main()` 的一行 `run(...)`；`verify_all.py:66-74` 的 `openspec_command()` 用 `os.name` 判平台差异，正是跨平台脚本体例先例（`CODEBUDDY.md §2`）。
- **三版本号权威值**：`crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13` / `CODEBUDDY.md §5` 亦记「当前 9」。本 change 不 bump、不新增权威状态。
- **临时约束永久化的本项目实例**：`crates/w3-client/src/melee/defs.rs:22`（模块头「当期『不新增 `EffectKind` / 命令变体』是 P8.1 change 内约束、非永久」）/ `:919`（「已被 P11 授权扩展取代」）/ `:2290-2292`（「穷尽匹配四个既有 `EffectKind` 变体…该约束是 P8.1 当期 change 内约束、非永久门禁」）；`openspec/changes/archive/2026-11-25-p11-ability-system/design.md:11`（关键约束取代）/ `:169-173`（结论 4）/ `:236`（把该约束当永久门禁的 Risk）。这是本项目 P11 apply 真实发生的「当期约束被后续会话误当永久禁令」返工案例。
- **护栏 2 的落点**：`openspec/config.yaml:50-72`（`rules.proposal/specs/design/tasks`）、`CODEBUDDY.md:80-90`（§8 四件套一致性与规格门）、`.codebuddy/agents/critic.md:25`（第 7 项规范一致性）/ `:28`（第 10 项 spec 与决策纪要质量）。
- **规则文档同步基线**：`docs/references/WC3-MELEE-RULES.md:424`（§14 第 5 条）；`tools/verify_all.py:256-294`（`rule_doc_sync_check()` 仅对过规格门且 `design.md` 含规则编号引用的玩法 change 生效，pending / 非玩法豁免）。
- **护栏的工程依据（本项目自定）**：本项目 P0–P6 已实施并归档后，跨会话 AI 协作暴露两类可核实的工程治理风险（本仓库 artifacts，见 `crates/w3-client/src/melee/defs.rs` P11 返工实例与 `tools/verify_all.py` 门禁体例）：① 上下文随 `crates/**` 规模增长而膨胀，需一条机械预算门禁抑制冗余；② 当期约束被后续会话误当永久门禁，需为临时类约束标注作用范围与失效条件；③ 判定须经独立重审，不由产出方自审即标通过；④ 显式失败、有问题直接报错，不写防御性代码。**两条护栏是本项目自定的工程治理约定**，不复制任何外部实现、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源；阈值 MUST 由本项目基线测量得出，**MUST NOT 冻结任何外部项目数字**。

**参考矩阵命中判定**：本 change 为纯工程治理（工作流 / 门禁 / 规则文字），不改确定性内核实现（T1）、不改移动 / 寻路（T2）、不改 WC3 数据格式 / 导出管线（T3）、不改玩法语义 / 技能 / JASS（T4）、不改场景 / HUD / Melee 初始化（T5）、不改网络传输（T6）、不改通用 RTS 引擎架构 / 确定性内核与表现层分离 / 平台适配实现（T7）；经核对 `docs/references/UPSTREAM.md:104-112` 矩阵 **T1–T7 均不命中**。「## 参考项目对照」据实登记该判定，不臆造参考结论、不引入参考项目锚、不派 `reference-researcher`。

## Goals / Non-Goals

**Goals:**
- 新增单一 capability `ai-workflow-guardrails`，用 SHALL/MUST 把两条工程治理契约与可执行验证（基线测量 / 改坏验红自证 / `openspec validate --strict` / `python tools/verify_all.py` / `python tools/check_encoding.py` / `cargo test --workspace`）一次成文。
- 让「阈值数值」「硬阈值门禁 vs 仅文档约定」「是否挂 `verify_all.py`」「是否纳入 CI 双平台」等**待确认项被参数化承接**：spec 用行为式 SHALL、脚本用阈值参数 / 常量、tasks 用条件式措辞，使规格门拍板这些二选一时**不推翻 spec requirement 与 task 结构**。
- 显式声明零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不新增依赖、不改 ADR 正文；**不重跑** `cargo test -p w3-core --test determinism`（不臆造内核改动），但跑 `cargo test --workspace` / `python tools/verify_all.py` 保证整体不回归。

**Non-Goals:**
- 不改 `crates/**` 任何实现；不改 ADR 0001 / 0002 / 0003 正文；不 bump 三版本号（9 不变）；不新增权威状态。
- **本轮只成文**：不新增 / 不改 `tools/check_code_budget.py` / `tools/verify_all.py` / `openspec/config.yaml` / `CODEBUDDY.md` / `.codebuddy/agents/critic.md` / `docs/**` / `openspec/specs/**` / 归档件 / 根配置 / p12 / p13；apply 阶段才动这些文件。
- **不纳入**：反编译 / REA / MCP / Ghidra 工具链、并行多 agent 写同一 crate、抓帧对齐原版渲染、真窗口人工验收（8.x）、golden hash 双平台、Q1 清单水位、p12/p13 任何 Scope、与本项目无关的外部技术栈或落地结论。
- 不 archive / 不 commit / 不 push / 不 fetch / 不 pull / 不 clone / 不 self-approve 规格门。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、**对跨平台确定性的影响（ADR 0002）**、分层依赖 / 架构原则。全部决策均为**纯工程治理 / 规则文字 / 只读门禁脚本**，对确定性契约零影响：脚本只读统计、不进状态哈希 / 快照、不改 `step()` / 命令排序。

### D1 护栏 1 形态：独立只读门禁脚本 `tools/check_code_budget.py`

- **决策**：新增独立纯 Python 脚本，只读统计 `crates/**/*.rs`（及可选 `tools/*.py`）的总行数 / 单文件行数 / 函数体行数 / 模块文件数，超阈值以英文报错 + 非零退出码**显式失败**（本项目既有硬红线「显式失败不静默降级」）；不写中间产物、不写游戏资产、不写本机绝对路径。
- **备选方案（被否）**：① 并入现有 `check_source_conventions.py`（被否：SRP，职责混淆、编码校验与规模预算是两类关注点，且该文件已覆盖源码约定语言检查）；② 引入第三方 `cloc` / `tokei` / `tree-sitter`（被否：用户明确「不新增第三方依赖」，`Cargo.lock` 不变；AST 解析对「函数体行数」过重）；③ 仅文档约定、不落脚本（列为「待确认项」的「硬阈值门禁 vs 仅文档约定」二选一，本决策不替用户拍板——见 D6）。
- **依据**：`tools/check_encoding.py` / `tools/check_source_conventions.py` / `tools/verify_all.py:318-348` 门禁体例；`CODEBUDDY.md §2`（跨平台 Python 脚本、`os.name`、UTF-8 无 BOM / LF、中文注释 / 英文输出）；`CODEBUDDY.md §9` 工具脚本跨平台。ADR 0001（零资产、零新依赖）、ADR 0002（脚本不属权威状态）。
- **代价**：四类指标里的「函数体行数」只能用轻量启发式（缩进 / 括号计数）而非 AST，可能有误报；阈值未拍板前脚本不能启用硬失败（故先做基线测量，D2）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。脚本只读 `crates/**/*.rs`、不写任何文件，不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()`，不改 `step()` / `ExecuteFrame→PlayerId→Sequence→Arrival` 命令排序；不 bump 三版本号（9 不变）。跨平台确定性本身不受影响，脚本自身按 `os.name` 判平台差异以在 Windows / Linux 一致地枚举文件。
- **分层依赖 / 架构原则**：SRP（一个脚本只做规模统计）、显式失败不静默降级、`w3-core` 不反向依赖上层（脚本在 `tools/`，不被任何 crate 引用）。

### D2 阈值治理：基线测量先行、数值进待确认项、不臆造外部项目数字

- **决策**：第一步任务只做**只读基线测量**，产出当前 `crates/` 规模分布（不设阈值、不改文件）；阈值具体数值由人在规格门拍板后填入脚本参数 / 常量。spec 的 requirement 用**行为式 SHALL**（「超阈值 MUST 报错退出」「阈值 MUST 由人拍板」）而非写死某个数字，使阈值拍板不推翻规格。`decision-memo.md`「待确认项」逐条登记。
- **备选方案（被否）**：① 直接把外部项目的规模数字冻结成本项目阈值（被否：那是外部项目数字、非本项目实测；用户明确阈值须由基线测量得出、不臆造外部数字）；② 把阈值数字写进 spec 正文 / `CODEBUDDY.md`（被否：阈值是会随仓库增长调参的工程参数，写进权威 spec 会让每次调参触发 `CODEBUDDY.md §8` 规格解锁重走规格门）；③ 用「相对百分比增长」代替绝对阈值（列为备选但仍需基线，未被否、但与「四类指标」口径不同，交规格门二选一）。
- **依据**：`proposal.md` Scope 护栏 1「阈值具体数值进待确认项由人拍板，先做基线测量」；`openspec/config.yaml:57-61`（`rules.specs`）；`tools/verify_all.py:175-195`（四件套指纹含 spec，改数字会解锁）。
- **代价**：apply 阶段阈值未拍板前门禁只能以「仅测量 / 报告」模式跑，不能硬失败；拍板动作落在规格门，由人承担。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。基线测量是只读统计，阈值是脚本参数，两者都不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：参数与策略分离（OCP：调阈值不改脚本逻辑）、避免重复定义、不拿仓库外数字冒充本项目事实（诚实边界）。

### D3 统计口径与跨平台：纯标准库、`os.name`、UTF-8 无 BOM / LF、注释中文 / 输出英文

- **决策**：用 Python 标准库（`pathlib` / `os`）枚举与统计，平台差异在脚本内用 `os.name` 判断（不维护两套脚本、不写 `.ps1` / `.sh` / `.bat` / `.cmd`）；注释中文、运行输出英文、UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；统计范围 `crates/**/*.rs`，`tools/*.py` 为可选（是否纳入亦列待确认项）。
- **备选方案（被否）**：① 按字节 / 按字符统计（被否：口径应为「行数」以对齐「每多一行，下一个会话就得多读一行」，字节数不可比）；② 按 `git ls-files` 过滤（被否：未跟踪的新文件也应被计入预算，否则新增大文件可绕过；可复用思路但不作为唯一来源）；③ 平台分叉写两套脚本（被否：`CODEBUDDY.md §2` / §9 明令禁止）。
- **依据**：`tools/verify_all.py:66-74`（`os.name` 分支先例）、`tools/check_encoding.py:15-25`（UTF-8 / LF / 未跟踪文件覆盖）、`CODEBUDDY.md §2` / §3 / §9；`critic.md:26`（清单第 8 项工具脚本跨平台）。
- **代价**：启发式函数体行数对宏 / 多行签名可能偏差，需在脚本注释声明口径、以「粗统计」看待，不冒充精确。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。脚本只读、不写、不进状态哈希 / 快照、不改 `step()` / 命令排序；其 `os.name` 分支只影响文件枚举方式，使 Windows / Linux 上统计结果一致，不触及模拟内核跨平台确定性（该待验证项见 ADR 0002 复核记录，本 change 不声称达成）。
- **分层依赖 / 架构原则**：DIP（脚本只依赖标准库与文件系统，不依赖任何 crate）、跨平台脚本单一实现、中文注释 / 英文输出约定。

### D4 门禁自证：改坏验红（门禁脚本自证）≠ `CODEBUDDY.md §5` 形态①反向验证

- **决策**：apply 阶段做一次「故意造超阈值临时 `.rs` 文件 → 跑 `check_code_budget.py` 报红（非零退出 + 英文报错）→ 删除临时文件 → 复跑通过（零退出）」的**门禁脚本改坏验红自证**，并显式登记这是**门禁脚本对自身阈值判定有效性的自证**，证明门禁在缺陷态会失败、修复态通过。
- **备选方案（被否）**：① 只跑正常态「全绿」不自证（被否：无法证明门禁真的会拦，判定须经独立重审）；② 把该自证措辞写成 `CODEBUDDY.md §5` 的「形态①反向验证」或据此去改 `w3-core` / 重跑 `determinism`（**被否，且 MUST NOT 混淆**：§5 形态①专指「涉及 `w3-core` 权威状态 / 帧序 / 命令排序 / 状态哈希 / 快照的新增或修改回归测试」；本 change 零内核改动、不新增 `w3-core` 回归测试，§5 硬门禁与形态①**不适用**）；③ 用真实源码改坏来代替临时文件（被否：会污染工作树、可能触发指纹 / 编码门禁，临时文件用完即删更安全）。
- **依据**：`CODEBUDDY.md §5` 形态①适用范围（`crates/w3-core/` 权威状态回归测试）；`critic.md:29` 清单第 11 项③（测试本身要审、断言真能失败）；`proposal.md` 与本 change `decision-memo.md` 明确「零内核改动、不重跑 determinism」；判定须经独立重审。
- **代价**：自证要造 / 删临时文件，须确保 `git status --short` 无残留（`CODEBUDDY.md §6` 零资产、临时文件不得入库）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。自证只针对门禁脚本，不改 `step()` / 命令排序 / 状态哈希 / 快照、不进 `first_difference()`、不 bump 三版本号（9 不变）；**不触发** `cargo test -p w3-core --test determinism`。
- **分层依赖 / 架构原则**：显式失败、测试自身可信度、诚实边界（不把门禁自证冒充内核确定性验证）。

### D5 护栏 2 落点：`config.yaml` `rules.*` + `CODEBUDDY.md §8` + `critic` 第 7/10 项 + 归档合并复核

- **决策**：apply 阶段在 `openspec/config.yaml` 的 `rules.proposal/specs/design/tasks` 与 `CODEBUDDY.md` §8 附近增加惯例——凡写 `MUST NOT` /「本期不做」/「暂不」/「当期 change 内约束」类文字 MUST 标注作用范围（哪个 change / 模块）与失效条件（依本项目工程治理约定）；`spec-steward` 归档合并时 MUST 复核是否被后续 change 取代；`critic` 清单第 7 项（规范一致性、spec 引用 ADR 编号）/ 第 10 项（四件套一致性、承重约束锚打开核验）加对应复核点。以 P11 `defs.rs` 过期注释返工为反例锚，验证惯例能覆盖该场景。
- **备选方案（被否）**：① 新建独立治理文档（被否：用户明确「单一权威约定 `CODEBUDDY.md`」，另立文档会与之打架）；② 写进 ADR 正文（被否：用户明确「不改 ADR 正文」，且 ADR 记「为什么」、spec / 规则记「系统该做什么」）；③ 只加 `critic` 复核点、不动 `config.yaml` / `CODEBUDDY.md`（被否：复核缺权威文字依据，会变成 reviewer 自由裁量）；④ 全局正则把所有 `MUST NOT` 一律加时间戳（被否：会误伤 ADR 0002 / §5 等**本就永久**的硬红线，必须区分「当期约束」与「永久约束」）。
- **依据**：`openspec/config.yaml:50-72`、`CODEBUDDY.md:80-90`（§8）、`.codebuddy/agents/critic.md:25,28`、`CODEBUDDY.md §8`（ADR 与 spec 一致性、四件套一致性、规格门）；P11 返工锚见 Context。
- **代价**：惯例是文字约定，机械门禁无法强校验「是否标注作用域」，只能靠 `critic` 第 7/10 项复核；如实登记为已知限制，不冒充机械保证（判定须经独立重审）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。规则文字是纯文档，不进 FNV-1a 状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）；`decision-memo.md` / 规则文字均不在四件套指纹之外触发内核改动。
- **分层依赖 / 架构原则**：单一权威（不另立文档）、ADR 与 spec 不打架、spec 引用 ADR 编号（`critic` 第 7 项）、诚实边界（文字约定不等同机械门禁）。

### D6 待确认项的参数化承接：拍板不改变 specs / approach / task 结构

- **决策**：把「阈值数值」「硬阈值门禁 vs 仅文档约定」「是否挂进 `verify_all.py`」「是否纳入 CI 双平台」「治理约定适用范围」「apply 时机与 owner 分工」「规格门批准记录回填」全部登记进 `decision-memo.md`「待确认项」，由规格门 / leader 拍板。四件套以**中性口径**承接：spec 写「SHALL 可挂进 `verify_all.py`」（能力，不锁死「必须挂」）、tasks 用「按规格门拍板结论挂载」条件式措辞、脚本阈值取参数。因此这些二选一属于「可安全延后而不改变 specs / approach / task breakdown」的未知项，符合 `openspec instructions design` 对 Open Questions 的定义，**不是**被跳过的决策。
- **备选方案（被否）**：① 本轮替用户把二选一拍成「硬门禁 + 挂 verify_all.py + 进 CI 双平台」（被否：用户明确「本轮只拍板了『新提独立工作流 change 并落成四件套』，其余一律进待确认项」，不臆造为已定案、无证据项不写「用户已确认」）；② 把这些项写进 `## 省略说明` 求门禁变绿（被否：`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 是预期 fail-closed 信号，本 change 非「目标与方向已清楚的小改动」，不 self-approve、不写省略说明）。
- **依据**：`tools/verify_all.py:207-253`（`spec_gate_check()` 对 pending fail-closed）/ `:143-172`（`decision_memo_check()` 三必填小节）/ `:175-195`（四件套指纹不含 `decision-memo.md`）；`openspec/decision-memo.template.md:14,55-61`；p12 / p13 `decision-memo.md`「## 规格门」pending 体例；`CODEBUDDY.md §8` / §9（规格门由人拍板）。
- **代价**：apply 阶段须先等规格门拍板才能确定门禁严格度与挂载方式；本轮成文不阻塞，但 `python tools/verify_all.py` 会因 pending 报 fail-closed（如实登记、不绕过）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。全部是流程 / 参数 / 规则文字决策，不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：参数与策略分离（OCP）、不拿沉默当授权、批准由人拍板（`CODEBUDDY.md §8`）。

### D7 整体边界：零确定性 / 零资产 / 零依赖 / 不改 ADR 正文 / 不跑 determinism

- **决策**：本 change 零 `w3-core` 权威状态改动、不进状态哈希 / 快照 / `first_difference()`、不 bump 三版本号（仍 9）、不动资产、不新增第三方依赖、不改 ADR 0001/0002/0003 正文；**不重跑** `cargo test -p w3-core --test determinism`（`CODEBUDDY.md §5` 硬门禁与形态①不适用），但跑 `cargo test --workspace` / `python tools/verify_all.py` / `python tools/check_encoding.py` / `cargo fmt --all -- --check` / `openspec validate p14-ai-workflow-guardrails --strict` 保证整体不回归；边界以 `git diff -- crates/w3-core/` 为空、三版本号仍 9、无资产、无新依赖自证。
- **备选方案（被否）**：① 「保险起见」也跑一遍 determinism 并写进 tasks（被否：用户明确「因零内核改动，tasks 里不要放 determinism 任务，不臆造内核改动」；放了反而暗示存在内核改动）；② 借本 change 顺手 bump 版本号 / 改 ADR（被否：超 Scope、用户明确不改 ADR 正文、不 bump）。
- **依据**：`CODEBUDDY.md §5` / §6 / §8 / §9；`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`；`tools/verify_all.py:318-348`（`cargo fmt` / `cargo test --workspace` / 资产扫描 / 决策纪要 / 规格门 / 规则文档同步全在统一门禁）。
- **代价**：`python tools/verify_all.py` 在 `status: pending` 期间会于 `spec_gate_check()` 失败（预期 fail-closed），本轮不能声称「统一门禁全绿」，只能如实登记 `decision_memo_check()` 为 True、`spec_gate_check()` 为预期失败。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。不新增权威状态（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照）、不改 `step()` / 命令排序 / 状态哈希编码；跨平台确定性待验证项（macOS / ARM64 / 其它 rustc）不因本 change 变化、不声称达成。
- **分层依赖 / 架构原则**：唯一状态所有者、只读参考不复制代码、显式失败、诚实边界（零内核改动不冒充全量门禁）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-112` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。**本 change 经核对 T1–T7 均不命中**（纯工作流 / 门禁 / 规则文字治理，不改任何参考矩阵所列的实现层），故如实登记「无参考项目借鉴、无参考项目锚、无参考缺口」，不臆造对照结论：

| 矩阵 | 主参考项目 / 路径 | 命中 | 判定依据（借鉴 / 不借鉴） |
|---|---|---|---|
| T1 确定性内核 / 状态哈希 / 快照（`UPSTREAM.md:106`，`vera20k`） | `vera20k` `src/net/` | 不命中 | 只新增只读门禁脚本 / 规则文字，不改内核实现、不 bump、不新增权威状态；ADR 0002 契约零改动 |
| T2 移动 / 寻路 / 帧阶段（`UPSTREAM.md:107`，`rts_kernel_cs`） | `rts_kernel_cs` `docs/` | 不命中 | 不触及移动 / 寻路语义；其 `double` 路线本就不采用（ADR 0002） |
| T3 WC3 数据格式 / SLK / 资产管线（`UPSTREAM.md:108`，`godot_warcraft3`） | `godot_warcraft3` `docs/data/` | 不命中 | 不改导出管线 / schema；基线测量统计的是本仓库 `.rs` 行数，非参考项目结论 |
| T4 玩法语义 / JASS / 技能（`UPSTREAM.md:109`，`WarsmashModEngine`） | `WarsmashModEngine` `jassparser/` 等 | 不命中 | 不实现任何技能 / 玩法语义；非玩法 change，不引用任何规则编号；AGPLv3 只借语义、禁复制代码，本 change 连语义也不借 |
| T5 场景 / 相机 / HUD / Melee 初始化（`UPSTREAM.md:110`，`godot_warcraft3`） | `godot_warcraft3` `packages/` | 不命中 | 不触及场景装配 / 呈现 / HUD |
| T6 网络传输 / relay（`UPSTREAM.md:111`，`vera20k`） | `vera20k` `src/net/` | 不命中 | 不改 W3N1 帧头 / relay / proto schema（ADR 0005）；`PROTOCOL_VERSION` 仍 9 |
| T7 通用 RTS 架构 / 确定性内核与表现层分离 / 跨平台验收方法论（`UPSTREAM.md:112`，`ConcordRTS`） | `ConcordRTS` `src/core/determinism/` 等 | 不命中 | 不触及确定性内核与表现层分离 / 平台适配实现；其 RA2/YR 专属格式 / 规则 / 数值 MUST NOT 作 WC3 事实来源；README 自述验收不作本项目已验证事实 |

- **五个参考项目只读、不复制代码（ADR 0001）**：`vera20k`（GPLv3 + Section 7）、`godot_warcraft3`（MIT）、`rts_kernel_cs`（MIT）、`WarsmashModEngine`（AGPLv3，仅借鉴语义、禁复制代码）、`ConcordRTS`（GPL-3.0-only，其 RA2/YR 规则不作 WC3 事实来源）——本 change **全部不借鉴、不引入依赖、不复制代码**（见 `CODEBUDDY.md §1` / §9、`UPSTREAM.md:96,100`）。
- **承重约束证据锚全部来自本仓库 artifacts**（`tools/*.py` / `crates/w3-client/src/melee/defs.rs` / P11 归档件 / `openspec/config.yaml` / `CODEBUDDY.md` / `critic.md`），见「## Context」与 `decision-memo.md`「承重约束」表；**无一条来自参考项目**，故无「矩阵条目 / 参考项目内路径」类锚需要双标。
- **护栏依据不是参考项目（如实登记）**：两条护栏是**本项目自定的工程治理约定**（代码膨胀预算门禁 / 临时约束时效与作用域 / 判定须经独立重审 / 显式失败），**不复制任何外部代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源**；阈值 MUST NOT 冻结任何外部项目数字。`CODEBUDDY.md §1` / `UPSTREAM.md` 的**只读参考项目共五个且仅五个**，本 change **MUST NOT 登记第六个参考项目**（不改 `UPSTREAM.md` / `NOTICE.md` / 参考项目数量）。
- **参考缺口：无**（本 change 不命中矩阵、无需 `reference-researcher` 对照报告，故本轮未委派；该判定待 `critic` 清单第 12 项复核）。若规格门改判命中任一 T1–T7，MUST 补 `reference-researcher` 对照报告并回到用户拍板，不臆造。护栏适用范围与阈值拍板登记在 `decision-memo.md`「待确认项」，不冒充已核验。

## Risks / Trade-offs

- **[Risk] 把外部项目规模数字误当成本项目阈值** → Mitigation：D2 基线测量先行、数值进待确认项由人拍板，spec 用行为式 SHALL、不写死数字；`design.md` Context 与「参考项目对照」显式标「阈值须以本项目实测为准」。
- **[Risk] 「暂时不做 / MUST NOT」文字被后续会话永久化（P11 已发生过）** → Mitigation：D5 惯例要求标注作用域与失效条件 + `spec-steward` 归档复核 + `critic` 第 7/10 项；但惯例是文字约定、无机械强校验，如实登记为已知限制，不冒充门禁（判定须经独立重审）。
- **[Risk] 函数体行数启发式误报、阈值拍板前门禁形同虚设** → Mitigation：脚本注释声明「粗统计」口径；阈值未拍板前只做测量 / 报告、不硬失败；改坏验红自证（D4）证明门禁在缺陷态会拦。
- **[Risk] 把门禁脚本改坏验红自证混淆为 `CODEBUDDY.md §5` 形态①反向验证，进而臆造 `w3-core` 改动或重跑 determinism** → Mitigation：D4 / D7 显式区分两者、tasks 不放 determinism 任务、边界自证 `git diff -- crates/w3-core/` 为空。
- **[Risk] 临时类约束文字与永久硬红线（ADR 0002 / §5）被一律加失效条件而互相打架** → Mitigation：D5 备选④被否，惯例只针对「当期 change 内约束」，永久硬红线 MUST NOT 被误标为临时。
- **[Risk] `spec_gate_check()` 对 `status: pending` 报 fail-closed，被误当失败去 self-approve / 写省略说明** → Mitigation：D6 如实登记为预期信号、不绕过、不填指纹 / `approved_by` / `date`；本轮交付说明明确该 fail-closed。
- **[Risk] 规则文字惯例无法机械校验、reviewer 自由裁量** → Mitigation：`critic` 第 7/10 项内容层复核 + `decision-memo.md` 证据锚，诚实边界登记为结构性改善、非机械保证。

## Migration Plan

- **本轮（成文期）**：只在 `openspec/changes/p14-ai-workflow-guardrails/` 内产出四件套 + `decision-memo.md`；`git status --short` 仅该目录新增；`## 规格门` 保持 `status: pending`、AI 未 self-approve。
- **规格门**：`critic` 按第 7 / 10 / 12 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ 由 `spec-steward` 在人确认后回填 `status: approved` + 四件套指纹 + `approved_by` + `date`（本轮不填）；此后规格锁定，改任一 artifact MUST 按 §8 同步回查其余三份并重走规格门。
- **apply（后续独立会话，时机 / owner 分工待 leader 拍板）**：按 D1–D7 落地，一个 change 一个 owner、不同时改多 owner 文件；先做基线测量，再按拍板结论定阈值 / 门禁严格度 / 挂载方式 / CI；规则文字由 `spec-steward` / `critic` 维护；不碰 p12 / p13 / 归档件 / 主 spec / `crates/**`。
- **回滚**：门禁脚本为新增文件、删除即可；`verify_all.py` 挂载为一行 `run(...)`、撤销该行即可；规则文字为纯文档、回退文字即可；无运行时迁移、无权威状态 / 快照 / 版本号变更，故**无确定性回滚面**（ADR 0002）。
- **人工项**：人工 push → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` → `/opsx:archive`；AI 不 push / fetch / pull / clone、不 archive、不本地 commit（本轮由 leader 统一处理）。

## Open Questions

> 以下均为「可安全延后而不改变 specs / approach / task breakdown」的未知项，已由 D6 参数化承接并登记在 `decision-memo.md`「待确认项」，由规格门 / leader 拍板；本轮**不臆造为已定案**：

- 四类规模指标（总行数 / 单文件行数 / 函数体行数 / 模块文件数）各自的阈值具体数值 → 待基线测量后由人拍板，不臆造外部项目规模数字。
- 硬阈值门禁 vs 仅文档约定二选一 → 待规格门拍板；spec 用行为式 SHALL、tasks 用条件式措辞，二选一不推翻规格。
- 是否挂进 `tools/verify_all.py`、是否纳入 CI 双平台 → 待规格门拍板；spec 写「SHALL 可挂进」（能力）。
- 护栏适用范围与阈值 → 由规格门用户拍板、不臆造、不冒充已核验、不登记为第六个参考项目。
- apply 阶段执行时机与 owner 分工（一个 change 一个 owner）→ 待 leader 安排后续独立会话。
- 规格门批准记录回填（`fingerprint` / `approved_by` / `date` / `critic` 报告 / 参考矩阵命中判定复核）→ 待用户拍板后由 `spec-steward` 回填，本轮一律不填。
