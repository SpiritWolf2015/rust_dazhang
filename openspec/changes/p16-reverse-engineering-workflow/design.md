# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「一份反编译使用流程参考说明怎么落盘」。事实来源为**本仓库 artifacts + 已由人拍板落地的合规措辞与 p15 四件套 + 仓库文件外的人工确认项**（逐条带可核验锚，不臆造）：

- **已拍板既成事实**：用户 2026-11-25 拍板「引入 clean-room 反编译证据流程」，`ADR 0003` 第二次修订（`docs/decisions/0003-scope.md:109-123`）、`ADR 0001` 复核记录（`docs/decisions/0001-license.md:77-83`）、`README.md:29` / `CONTRIBUTING.md:11` 合规措辞、`p15-presentation-evidence-workflow` 四件套（`status: approved` + 四件套指纹锁定）均已落地，commit `6bdf3ae` 已 push。用户现在要求「落盘 参考目录 反编译流程说明 并走 规格门」——本轮只成文、不落盘。
- **职责分离**：p15 定义「系统该做什么」（`specs/presentation-evidence-workflow/spec.md:9-61`）、ADR 记「为什么决定」（`CODEBUDDY.md §8`），本 change 的文档提供「怎么用的可照抄流程」三类文档互不替代；p15 四件套已锁定，改任一 artifact 即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门。
- **红线**：`docs/decisions/0003-scope.md:117-120`（零资产 / MPQ 派生数据不入库 / 反编译产物留本机仓库外 / 不绕 EULA · DRM · 登录 · 反作弊 / 仅合法自备经典客户端；证据 MUST NOT 进权威状态 / 状态哈希 / 快照 / golden hash / determinism / CI；视觉 oracle 非业务逻辑 oracle；`w3-core` 零改动、不 bump 三版本号）；`docs/decisions/0001-license.md:80-82`；`docs/decisions/0002-determinism.md:49-53`（§5 状态权威与表现边界）。
- **三版本号**：`crates/w3-core/src/snapshot.rs:67-68`（`SNAPSHOT_VERSION` / `SIM_VERSION` == 9）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION` == 9）；`openspec/config.yaml:13` / `CODEBUDDY.md §5` 亦记「当前 9」。本 change 零新增权威状态、不 bump。
- **文档定位声明块体例**：`docs/references/WC3-MELEE-RULES.md:1-13`（文档定位 / 不是上游参考项目 / 合规 / 确定性 / 状态标记 / 建立日期 / 维护责任）、`docs/references/UPSTREAM.md:1-5`（固定五个只读参考项目 + 区分注记）。
- **仓库文件外证据 / 人工确认项（不冒充已核验）**：`craft-anything`（本机外部工具，MIT OR Apache-2.0，early alpha）工作树各文件行号，行号由用户任务清单给出、可能漂移；取证流程的通用工程环节为本项目自定的流程说明、不附带外部资料行号；沿用 p14 `design.md:112` / p15 `design.md:154` 口径——本机外部工具 **MUST NOT 登记为第六个只读参考项目**，不复制代码、不引入技术栈、不作 WC3 玩法 / 格式 / 数值事实来源。
- **消费侧既有能力**：`crates/w3-render/src/gpu/mod.rs:908-927`（`render_headless_rgba` / `render_headless_rgba_with_textures` 离屏 RGBA 两变体）——文档 Check 步骤只引用、不改其实现。

**参考矩阵命中判定**：本 change 是**纯文档 / 参考说明交付**（与 `p14-ai-workflow-guardrails` 工作流护栏 change 同类），不改解析 / 玩法 / 网络 / 权威内核 / 场景呈现实现。经核对 `docs/references/UPSTREAM.md:104-114` 矩阵：**T1–T7 均不命中**（T7 可选交叉、仅红线）；无参考项目新增借鉴、无新增参考项目锚、无参考缺口，故本轮**不委派** `reference-researcher`、无对照报告（沿用 p14 `design.md:113` 口径），该判定待 `critic` 清单第 12 项复核。逐条判定见下「## 参考项目对照」。

## Goals / Non-Goals

**Goals:**
- 新增单一 capability `reverse-engineering-workflow-reference`，用 SHALL/MUST 把「`docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 唯一目标路径 + 8 个章节（定位声明块 / 取证流程的通用工程环节 / `craft-anything` 产品化 / 逐项对照表 / 端到端 8 步 / 两条反馈循环 / 产物落点矩阵 / 诚实边界）」写成带可执行 Grep / Read / 文件存在性验证的 `#### Scenario:`。
- 把「取证流程的通用工程环节 = 本项目自定流程说明、不附带外部资料行号」「`craft-anything` 文件行号 = 仓库文件外证据 / 人工确认项、可能漂移」「文档章节粒度 / 是否建索引 / apply 时机与 owner 分工 / 规格门批准记录」参数化承接为规格门待确认项（spec 用行为式 SHALL、tasks 用条件式措辞），拍板这些二选一时**不推翻 spec requirement 与 task 结构**。
- 显式声明零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不新增依赖、不改 ADR 正文 / 主 specs / 业务规则；**不重跑** `cargo test -p w3-core --test determinism`（不臆造内核改动）。
- 把 `UPSTREAM.md` 矩阵 T1–T7 逐条判定落进「## 参考项目对照」，红线边界逐条声明、五个只读参考项目不变、缺口如实登记为「无」。

**Non-Goals:**
- 不改 `crates/**` 任何实现（含 `w3-render` / `w3-content` / `w3-core`）；不改 ADR 0001 / 0002 / 0003 正文；不 bump 三版本号（9 不变）；不新增权威状态。
- **本轮只成文**：不新增 / 不改 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`（apply 阶段才落盘）、不改 `tools/**` / `openspec/config.yaml` / `CODEBUDDY.md` / `docs/**` / `openspec/specs/**` / 归档件 / 根配置 / p12 / p13 / p14 / p15 / `README.md` / `CONTRIBUTING.md` / `UPSTREAM.md` / `NOTICE.md` / `WC3-MELEE-RULES.md`。
- **不纳入**：把 `craft-anything` 复制 / 加为 crate 依赖 / 登记 `NOTICE.md` / 接入 CI；把反编译产物或任何 MPQ 派生数据入库或写进文档；把视觉 oracle 当业务逻辑 oracle 去改 `w3-core` / determinism / golden hash 门禁；实现任何玩法规则编号；登记第六个只读参考项目或改 `UPSTREAM.md` 五个参考项目表格；改 p14「明确不纳入」边界；追溯改写 P11 归档件；真窗口人工验收（8.x）、golden hash 双平台、CI 双平台。
- 不 archive / 不 commit / 不 push / 不 fetch / 不 pull / 不 clone / 不勾任一复选框 / 不 self-approve 规格门 / 不写「## 省略说明」。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、**对跨平台确定性的影响（ADR 0002）**、分层依赖 / 架构原则。全部决策均为**工作流 / 文档 / 证据纪律 / 只读对照**，对确定性契约零影响：不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号。

### D1 交付形态：单一新增文档，不改既有权威文件、不建索引

- **决策**：apply 阶段唯一产物 = 在 `docs/references/` 新增**一份** `REVERSE-ENGINEERING-WORKFLOW.md`（clean-room 取证工具 ↔ 本项目 p15 形态 A 对照的可照抄反编译使用流程说明）。不改 `UPSTREAM.md` 五个参考项目表格、不建索引到 `UPSTREAM.md` / `README.md`、不改任何既有权威文件；不写本机真实绝对路径 / 资产 / 派生数据 / 凭证。
- **备选方案（被否）**：① 把流程说明直接写进 p15 四件套或 ADR 0001 / 0002 / 0003 正文（被否：职责分离——p15 记「该做什么」、ADR 记「为什么决定」、本档记「怎么用」；且 p15 四件套已 `status: approved` + 指纹锁定，改即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门）；② 拆成多份文档（如通用工程环节说明 + 工具手册 + p15 复述各一份）（被否：用户明确「新增一份」，clean-room 取证工具 ↔ 本项目 p15 形态 A 对照正是要把两侧串成一条端到端流程，拆开反而失去对照价值）；③ 顺手在 `README.md` / `UPSTREAM.md` 加索引（被否：用户明确「不建索引」，如需索引留待确认项且须先改本 change 四件套触发规格解锁）；④ 本轮直接落盘该文档（被否：用户明确本轮只成文，文档落盘是规格门批准后的 apply 动作）。
- **依据**：用户本会话任务边界（仓库文件外证据 / 人工确认项）；`openspec/changes/p15-presentation-evidence-workflow/decision-memo.md:100-104`（p15 已锁定）；`docs/decisions/0003-scope.md:122`（p15 四件套成文属后续独立任务）；`CODEBUDDY.md §8`（四件套一致性 / 规格锁定与重审）。
- **代价**：文档未在 `README.md` / `UPSTREAM.md` 建索引、可发现性偏低，如实登记为已知限制；索引二选一属规格门待确认项。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。新增一份纯文档、留仓库内且不含资产 / 派生数据，不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()`，不改 `step()` / `ExecuteFrame→PlayerId→Sequence→Arrival` 命令排序；不 bump 三版本号（9 不变）。跨平台确定性本身不受影响；本 change 不据此声称 macOS / ARM64 / 其它 rustc 已达成（ADR 0002 复核记录口径）。
- **分层依赖 / 架构原则**：SRP（p15 / ADR / 本档三类文档职责分离）、诚实边界（文档 ≠ 门禁、工具 ≠ 依赖）、不拿沉默当授权。

### D2 文档结构：定位声明块 + 取证流程的通用工程环节 + 8 步流程 + 落点矩阵 + 诚实边界

- **决策**：文档 8 个章节按「文档定位声明块（沿用 `WC3-MELEE-RULES.md:1-13` / `UPSTREAM.md:1-5` 体例）→ 取证流程的通用工程环节 → `craft-anything` 产品化实现 → 逐项对照表（约 10 行）→ 端到端 8 步流程（每步标「输入 · 工具 · 产物 · 落点 · 边界」）→ 两条反馈循环 → 产物落点矩阵 → 诚实边界」组织；每章节在 spec 里用 SHALL/MUST + 带 Grep / Read / 文件存在性验证的 `#### Scenario:` 表达，不写「应有」二字的空断言。
- **备选方案（被否）**：① 只写 `craft-anything` 的 CLI 命令清单（被否：缺取证流程的通用工程环节与 p15 形态 A 的对照，无法回答「两侧怎么串起来」）；② 只复述 p15 的行为契约（被否：与 p15 spec 重复、不提供「可照抄流程」增量）；③ 把「诚实边界」省略或并入他节（被否：用户明确列为独立章节，且 early alpha / 未 live 验证 / 工具本体尚未运行等缺口必须集中、醒目登记，不冒充已验证）；④ 场景只写「文档应有 X 章节」（被否：`openspec instructions specs` 要求 Scenario 可测试、用户明确要求 THEN 给可执行 Grep / Read / 文件存在性验证）。
- **依据**：用户本会话任务清单（8 章节 + 10 行对照表 + 五要素，仓库文件外证据 / 人工确认项）；`openspec instructions specs --change p16-reverse-engineering-workflow --json`（Scenario 格式规则）；`docs/references/WC3-MELEE-RULES.md:1-13` / `docs/references/UPSTREAM.md:1-5`（定位声明块体例）；`openspec/config.yaml:57-61`（`rules.specs`）。
- **代价**：`craft-anything` 文件行号为仓库文件外证据，Scenario 的 Grep 命中断言只能校验成文形态、不能证明外部工具原文行号正确；取证流程的通用工程环节为本项目自定流程说明、不附带外部资料行号；如实登记为已知限制、不冒充已核验。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。章节结构 / Scenario 全是文档与验证描述，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）；**不触发** `cargo test -p w3-core --test determinism`。
- **分层依赖 / 架构原则**：显式失败（无 THEN 可执行验证即不合格）、诚实边界（可核验成文形态 ≠ 外部原文已核验）、ADR 记「为什么」与 spec 记「该做什么」不打架。

### D3 证据锚与来源定性：本项目自定流程说明 + 工具行号一律标人工确认项

- **决策**：取证流程的通用工程环节为本项目自定的流程说明，MUST 注明「不引用、不转述任何外部内部文章、不附带外部资料行号、产出行为规格 / 证据节点而非代码、不是 WC3 技术事实来源」；`craft-anything` 各文件行号（`README.md:96-105` 等）在文档里一律标注为「仓库文件外证据 / 人工确认项 / 本机外部工具工作树快照 / 行号可能漂移」；MUST NOT 把工具行号写成仓库内 `file:line` 或「已逐行核验」。
- **备选方案（被否）**：① 把工具行号伪装成仓库内 `file:line` 锚（被否：该工具非仓库文件，违反 `CODEBUDDY.md §8` 证据锚诚实边界）；② 省略工具行号只写章节名（被否：用户明确要求带行号锚，且 p15 已有「固定提交双标」先例说明锚的价值）；③ 本轮打开外部工具原文逐行复核行号（被否：用户明确本轮只成文、外部工具工作树 HEAD 可能前进，逐行复核属 apply / 人工动作，不冒充已核验）；④ 把判定须经独立重审写成项目自创结论（被否：那是本项目自定的工程治理约定，如实登记来源）。
- **依据**：p15 `design.md:89`（T1 承重锚固定提交双标先例）/ `:143-149`（`craft-anything` 缺口如实登记）；p14 `design.md:112`（工程治理约定、不登记第六个参考项目）；`CODEBUDDY.md §8`（承重约束逐条带锚、无证据项不写「用户已确认」）；`docs/decisions/0003-scope.md:113`（行为规格、非代码）。
- **代价**：行号漂移进规格门待确认项；文档可复现性靠标注纪律而非门禁。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。证据锚标注是文档文字，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：诚实边界（仓库外证据不当仓库内锚、不拿沉默当授权）、证据锚可核验性。

### D4 合规 / 确定性边界：定位声明块 + 产物落点矩阵双重声明

- **决策**：文档定位声明块逐条写 ADR 0001 合规边界（零资产入库 / MPQ 派生数据不入库 / 反编译产物留本机仓库外 / 不绕 EULA · DRM · 登录 · 反作弊 / 仅合法自备经典客户端）与 ADR 0002 确定性边界（反编译证据是视觉 oracle，不进权威状态 / 状态哈希 / 快照 / golden hash / determinism 验收 / CI 门禁）；「产物落点矩阵」再对伪 C / 调用图 / 行为规格 / 离屏 RGBA / MPQ 派生数据 / 状态哈希六项逐项标「能否入库」「能否进权威状态或门禁」，依据 `docs/decisions/0003-scope.md:118-119` / `README.md:29` / `CONTRIBUTING.md:11`。
- **备选方案（被否）**：① 只在开头写一句「合规见 ADR」（被否：用户明确要求定位声明块逐条 + 落点矩阵逐项，一句话无法照抄、也无法核验）；② 把离屏 RGBA 写成「可入库证据」（被否：`0003-scope.md:119` 明文证据不进 golden hash / determinism / CI，离屏 RGBA 只作消费侧视觉 oracle 的中间产物、不入库）；③ 把行为规格 / 伪 C 写成「可提交进仓库当证据」（被否：`0001-license.md:81`「反编译 / 反汇编产物一律留本机、不入库、不随发行物分发」、`README.md:29` / `CONTRIBUTING.md:11`）；④ 把 MPQ 派生数据标「可入库但脱敏」（被否：ADR 0001 零资产红线、无脱敏例外）。
- **依据**：`docs/decisions/0003-scope.md:118-119`、`README.md:29`、`CONTRIBUTING.md:11`、`docs/decisions/0001-license.md:80-82`、`docs/decisions/0002-determinism.md:49-53`、`docs/references/UPSTREAM.md:63-92`。
- **代价**：矩阵判定与依据分散在多份 ADR / 文档，apply 阶段须逐条对照，不臆造判定。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。合规 / 确定性边界是文档声明，本身不新增权威状态、不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）；矩阵明确「状态哈希」一项也仅作对照项、不被文档改动。
- **分层依赖 / 架构原则**：唯一状态所有者、零资产红线、显式失败（边界 MUST NOT 越界）、诚实边界。

### D5 参考矩阵逐条判定与红线不越界（T1–T7 均不命中）

- **决策**：`docs/references/UPSTREAM.md:104-114` 矩阵 T1–T7 逐条判定**均不命中**（T7 可选交叉、仅红线），故不委派 `reference-researcher`、无对照报告、无参考缺口；五个只读参考项目不变（`vera20k` / `godot_warcraft3` / `rts_kernel_cs` / `WarsmashModEngine` / `ConcordRTS`），`craft-anything` 不登记为第六个；红线边界逐条声明（见「## 参考项目对照」）。
- **备选方案（被否）**：① 把 `craft-anything` 登记为第六个只读参考项目（被否：`CODEBUDDY.md §1` / `UPSTREAM.md:9-15,96,100` 五个只读矩阵，沿用 p14 `design.md:112` / p15 `design.md:154` 口径）；② 判定 T5（场景 / HUD / 3D 呈现）命中并补对照报告（被否：本 change 只产出一份流程说明文档、不改任何呈现实现，与 p15 的 T5 主命中不同，p15 对照报告已覆盖呈现层锚，本轮不重复委派）；③ 以 `ConcordRTS` 作呈现层参考或以其 README「逐字节一致」自述作已验证事实（被否：`UPSTREAM.md:114` 明令 T3 / T4 / T5 MUST NOT 以其为参考、`:35` README 自述不作已验证事实）；④ 凭记忆给 WC3 格式 / 玩法下结论凑「对照」（被否：`UPSTREAM.md:100` 明令不得凭记忆下结论，本 change 非玩法）。
- **依据**：`docs/references/UPSTREAM.md:104-114`（矩阵与交叉注记）/ `:9-15`（五个固定参考版本）/ `:96,100`（五个只读、不复制代码）/ `:35,50,52,53,110,114`（红线）；`openspec/changes/p14-ai-workflow-guardrails/design.md:100-113`（全不命中逐条判定体例）；`CODEBUDDY.md §9`（凡 T1–T7 命中 MUST 有「## 参考项目对照」）；`openspec/config.yaml rules.design`。
- **代价**：命中判定待 `critic` 清单第 12 项复核；若改判命中任一 T1–T7，MUST 补 `reference-researcher` 对照报告并回到用户拍板、不臆造。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。只读对照、不复制代码、不引入依赖，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）；`ConcordRTS` 的「逐字节一致」验收方法论只作红线对照，不作本项目跨平台确定性已达成的证据。
- **分层依赖 / 架构原则**：只读参考不复制代码、许可证红线（AGPLv3 / Section 7 / GPL-3.0-only）、证据锚诚实边界。

### D6 非玩法边界、规则文档同步「无需同步」与规格门 pending

- **决策**：本 change 非玩法 / 纯文档，不实现任何玩法规则编号；`design.md` 内**不出现** `R\d+` 形式的台账编号（避免 `tools/verify_all.py` 的 `RULE_REF_PATTERN` 把非玩法 change 误判为玩法 change），被否方向台账编号只在 `decision-memo.md` 里用；`decision-memo.md`「## 规则文档同步」按 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条给出「无需同步」结论、保持非空；「## 规格门」严格只有 `- status: pending` + 一段说明、不填指纹 / `approved_by` / `date` / `reason`、不写「## 省略说明」、不 self-approve。
- **备选方案（被否）**：① 省略规格门或写省略说明求门禁变绿（被否：用户明确「走规格门」；`spec_gate_check()` 对 `status: pending` 是预期 fail-closed 信号，本 change 非「目标与方向已清楚的小改动」，不 self-approve）；② 把玩法规则编号引用写进 `design.md` 当「未实现」声明（被否：`RULE_REF_PATTERN` 机械扫描 `design.md` 命中即视为玩法 change，会触发 `rule_doc_sync_check()` 结构校验，与「非玩法豁免」相冲突，故改在 `decision-memo.md` 里以否定式声明）；③ 本轮替用户把章节粒度 / 索引 / 行号漂移 / apply 分工拍成定案（被否：用户只拍板「落盘流程说明 + 走规格门」，无证据项 MUST NOT 写「用户已确认」）；④ 自填 `status: approved` + 指纹（被否：`CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve）。
- **依据**：`tools/verify_all.py:40`（`RULE_REF_PATTERN`）/ `:256-294`（`rule_doc_sync_check()` 仅扫 `design.md`）/ `:207-253`（`spec_gate_check()` pending fail-closed）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（四件套指纹不含 `decision-memo.md`）；`openspec/decision-memo.template.md:14,55-64`；p12 / p14 / p15 `decision-memo.md`「## 规格门」体例；`CODEBUDDY.md §8` / §9。
- **代价**：`python tools/verify_all.py` 由 leader 跑、本轮不跑；`spec_gate_check()` 在 `status: pending` 期间报 `invalid spec gate status 'pending'` 属预期 fail-closed，如实登记、不绕过。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。规格门状态 / 规则文档同步结论是纯文档 / 门禁元数据，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：参数与策略分离（OCP）、不拿沉默当授权、批准由人拍板、诚实边界（pending 不冒充放行）。

### D7 确定性 / 资产 / 依赖整体边界：零 `w3-core` 改动、不跑 determinism

- **决策**：本 change 零 `w3-core` 权威状态改动、不进状态哈希 / 快照 / `first_difference()`、不 bump 三版本号（仍 9）、不动资产、不新增第三方依赖、不改 ADR 0001 / 0002 / 0003 正文、不改主 `openspec/specs/**`、不改业务规则、不接 CI；**不重跑** `cargo test -p w3-core --test determinism`（`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用），但跑 `openspec validate p16-reverse-engineering-workflow --strict` / `python tools/check_encoding.py` 保证成文不回归；边界以 `git diff -- crates/w3-core/` 为空、三版本号仍 9、无资产、无新依赖自证。
- **备选方案（被否）**：① 「保险起见」也跑一遍 determinism 并写进 tasks（被否：用户明确零内核改动，放了反而暗示存在内核改动、`CODEBUDDY.md §5` 形态①不适用；沿用 p14 `design.md` D7 / p15 `design.md` D5 诚实边界）；② 借本 change 顺手 bump 版本号 / 改 ADR 正文 / 接 CI（被否：超 Scope，用户明确本轮只成文、不 bump、不改 ADR 正文）；③ 把反编译证据纳入 golden hash / CI 当「可复现验收」（被否：`0003-scope.md:103,119` 明文证据 MUST NOT 进 golden hash / determinism / CI，视觉 oracle 不是机器可判业务逻辑 oracle）。
- **依据**：`CODEBUDDY.md §5` / §6 / §8 / §9；`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`；`docs/decisions/0003-scope.md:117-120`；`docs/decisions/0002-determinism.md:49-53`；`tools/verify_all.py:318-348`（统一门禁体例）。
- **代价**：本轮 `python tools/verify_all.py` 由 leader 跑、本轮不跑；决策纪要三必填小节非空、`rule_doc_sync_check()` 对 pending / 非玩法豁免。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。不新增权威状态（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照）、不改 `step()` / 命令排序 / 状态哈希编码；跨平台确定性待验证项（macOS / ARM64 / 其它 rustc）不因本 change 变化、不声称达成。
- **分层依赖 / 架构原则**：唯一状态所有者、只读参考不复制代码、显式失败、诚实边界（零内核改动不冒充全量门禁）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-114` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。本 change 是**纯文档 / 参考说明交付**（与 `p14-ai-workflow-guardrails` 工作流护栏 change 同类），经核对 **T1–T7 均不命中**（T7 可选交叉、仅红线），故本轮**不委派** `reference-researcher`、无对照报告、无参考项目新增借鉴、无新增参考项目锚、**无参考缺口**；该判定待 `critic` 清单第 12 项复核（沿用 p14 `design.md:113` 口径）。

| 矩阵 | 主参考项目 / 路径 | 命中 | 不命中理由 / 不借鉴（含许可证理由）/ 可核验锚 |
|---|---|---|---|
| T1 确定性内核 / lockstep / 状态哈希 / 命令排序 / 快照（`UPSTREAM.md:106`，`vera20k`：`src/net/`、`services/relay-protocol/`、`docs/`） | `vera20k` `src/net/` | 不命中 | 只新增一份流程说明文档，不改内核实现、不 bump、不新增权威状态；ADR 0002 契约零改动。GPLv3 + Section 7 附加条款**只有真的复制代码时**才进入，本轮只读不复制、`vera20k` Section 7 只读不复制不进入（锚 `UPSTREAM.md:50,106` / `0001-license.md:14,30,66-73`） |
| T2 移动 / 寻路 / 编队 / 帧阶段（`UPSTREAM.md:107`，`rts_kernel_cs`：`docs/FRAMEWORK.md`、`MOVEMENT.md`、`NAVIGATION.md`） | `rts_kernel_cs` `docs/` | 不命中 | 不触及移动 / 寻路 / 帧阶段语义；其 `double` 数值路线与「不承诺跨平台确定性」**不采用**（ADR 0002 / `UPSTREAM.md:52`），C# 实现不复制 |
| T3 WC3 数据格式 / SLK / 地图解析 / 资产管线（`UPSTREAM.md:108`，`godot_warcraft3`：`docs/data/PIPELINE.md`、`WC3_ASSET_PATHS.md`、`CONTENT_PACKS.md` 等） | `godot_warcraft3` `docs/data/` | 不命中 | 不改导出管线 / schema / 资产解析；文档「诚实边界」只引用「WC3 资产与数据表仍靠 `godot_warcraft3` 导出管线」这一既有事实（`UPSTREAM.md:51,108`），不新增借鉴。资产与派生 JSON / PNG / GLB 一律不入库（ADR 0001 / `UPSTREAM.md:63-92`）；MIT、只读不复制代码 |
| T4 玩法语义 / JASS / 技能 / FDF UI（`UPSTREAM.md:109`，`WarsmashModEngine`：`jassparser/`、`fdfparser/`、`resources/Scripts/`） | `WarsmashModEngine` `jassparser/` 等 | 不命中 | 不实现任何技能 / 玩法语义；非玩法 change，不引用任何玩法规则编号作需求依据。**AGPLv3 只借语义、不复制代码**：只核对许可证、不借 JASS 语义、不复制 Java / LibGDX 代码、不引入依赖（ADR 0001 / `UPSTREAM.md:53`）；AGPLv3 与 GPLv3 组合有条件，未经 ADR 0001 复核不得引入其代码 |
| T5 场景装配 / 相机 / HUD / Melee 初始化 / 3D 呈现（`UPSTREAM.md:110`，`godot_warcraft3`：`packages/`、`apps/game/`、`docs/design/game/`） | `godot_warcraft3` `packages/` | 不命中 | 不触及场景装配 / 呈现 / HUD 实现；文档 Check 步骤只**引用** `crates/w3-render/src/gpu/mod.rs:908-927` 离屏能力名称、不改其实现。T5 MUST NOT 以 `ConcordRTS` 为参考（`UPSTREAM.md:114`）；`FACTION_UNIT_ROSTERS.md` 原创三族只借结构 / 批次判据、不借单位名与数值 |
| T6 网络传输 / UDP 可靠层 / relay 广播重传（`UPSTREAM.md:111`，`vera20k`：`src/net/`、`services/relay-protocol/`） | `vera20k` `src/net/` | 不命中 | 不改 W3N1 帧头 / relay / proto schema（ADR 0005）；`PROTOCOL_VERSION` 仍 9（`crates/w3-net/src/packet.rs:50`），无借鉴、无线格式引入 |
| T7 通用 RTS 引擎架构 / 确定性内核与表现层分离 / 平台适配与跨平台逐字节验收方法论（`UPSTREAM.md:112`，`ConcordRTS`：`src/core/determinism/`、`src/network/lockstep/`、`src/network/snapshots/`） | `ConcordRTS` `src/core/determinism/` 等 | 不命中（可选交叉、仅红线） | 不触及确定性内核与表现层分离 / 平台适配实现；只在红线层保留「视觉 oracle MUST NOT 进权威状态 / 哈希 / 快照」的方法论对照，不新增借鉴。**`UPSTREAM.md:114` T5 明令禁止以 `ConcordRTS` 为参考**；其 RA2/YR 专属格式（SHP / VXL / TMP / MIX / INI）、规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染绝不作 WC3 玩法 / 格式 / 数值 / 规则事实来源；README 自述「逐字节一致」**不作本项目已验证事实**（`UPSTREAM.md:35`）；GPL-3.0-only 与 GPLv3 同族、只读不复制代码不引入依赖 |

**红线边界（逐条声明）：**

- `WarsmashModEngine` AGPLv3 只借语义、不复制代码、不引入 Java / LibGDX 依赖（`CODEBUDDY.md §9` / ADR 0001 / `UPSTREAM.md:53`）。
- `vera20k` Section 7 附加条款只读不复制不进入：只有真的复制其代码时才随之进入（`UPSTREAM.md:50,106` / `0001-license.md:14,30,66-73`）。
- `ConcordRTS` RA2/YR 专属格式 / 规则 / 数值绝不作 WC3 事实来源；`UPSTREAM.md:114` T5 禁止以 `ConcordRTS` 为参考；README 自述「逐字节一致」不作本项目已验证事实（`UPSTREAM.md:35`）。
- `rts_kernel_cs` 的 `double` 路线不采用（ADR 0002 / `UPSTREAM.md:52`）。
- 资产与派生 JSON / PNG / GLB 一律不入库（ADR 0001 / `UPSTREAM.md:63-92`）；反编译产物留本机仓库外、不随发行物分发（`0001-license.md:81` / `README.md:29` / `CONTRIBUTING.md:11`）。

**参考项目只读、数量与来源（如实登记）：**

- 五个只读参考项目仍是 `vera20k`（GPLv3 + Section 7）、`godot_warcraft3`（MIT）、`rts_kernel_cs`（MIT）、`WarsmashModEngine`（AGPLv3，仅借鉴语义、禁复制代码）、`ConcordRTS`（GPL-3.0-only，只读、RA2/YR 不作 WC3 事实来源）——全部只读、不复制代码、不引入依赖（`CODEBUDDY.md §1` / §9、`UPSTREAM.md:9-15,96,100`）。
- `craft-anything`（本机外部工具，MIT OR Apache-2.0）**MUST NOT 登记为第六个只读参考项目**（沿用 p14 `design.md:112` / p15 `design.md:154` 口径）；生产侧 / 消费侧只读文档交接、判定与产物同提交 + 独立重审、证据结构化均为本项目自定的工程治理约定，不复制代码、不引入技术栈、不作 WC3 玩法 / 格式 / 数值事实来源。未运行该工具、不冒充已核验。
- 承重约束证据锚全部来自本仓库 artifacts 与仓库文件外的人工确认项（ADR / README / CONTRIBUTING / p15 四件套 / `UPSTREAM.md` 矩阵 / `craft-anything` 文件行号），见「## Context」与 `decision-memo.md`「承重约束」表；**无一条来自参考项目**，故无「矩阵条目 / 参考项目内路径」类锚需要双标。
- **参考缺口：无**（本 change 不命中矩阵、无需 `reference-researcher` 对照报告，故本轮未委派；判定待 `critic` 清单第 12 项复核）。若规格门改判命中任一 T1–T7，MUST 补对照报告并回到用户拍板，不臆造。`craft-anything` 的 early alpha / Clean room 不是 OS 沙箱 / 无 MPQ · BLP · MDX · SLK 专用解析器 / Deep + Clean room 组合无 live 验证先例等缺口，是**工具能力边界**（见 `decision-memo.md`「已知限制」与文档「诚实边界」章节），**不是**参考项目对照缺口，不混为一谈；取证流程为本项目自定流程说明、工具本身的可核实性属人工确认项（`decision-memo.md`「待确认项」），不冒充已核验。

## Risks / Trade-offs

- **[Risk] 把「引入工作流」误读成已引入代码依赖 / 放宽红线 / 过规格门 / 获暴雪授权 / 试点完成 / 已落盘文档** → Mitigation：proposal / specs / D1 / D4 / D7 逐处显式否定式声明；`## 规格门` 保持 `status: pending`、AI 未 self-approve；红线锚 `0001-license.md:80-82` / `0003-scope.md:117-120` / `README.md:29` / `CONTRIBUTING.md:11`。
- **[Risk] `craft-anything` 工具行号被当仓库内可机械核验锚，或工作树 HEAD 前进导致行号漂移** → Mitigation：D3 逐处标「仓库文件外证据 / 人工确认项 / 当时快照 / 可能漂移」；取证流程的通用工程环节为本项目自定流程说明、不附带外部资料行号；工具行号漂移进 `decision-memo.md`「待确认项」；不冒充已逐行核验。
- **[Risk] `design.md` 出现 `R\d+` 形式台账编号，被 `RULE_REF_PATTERN` 误判为玩法 change，触发 `rule_doc_sync_check()` 结构校验** → Mitigation：D6 把被否方向台账编号只放 `decision-memo.md`，`design.md` 内零 `R\d+`；成文后 Grep 自证零命中。
- **[Risk] 把视觉 oracle / 离屏 RGBA 当 ADR 0002 业务逻辑 oracle，据此改 `w3-core` / determinism / golden hash / CI** → Mitigation：D4 / D7 显式区分「视觉 oracle ≠ 可冻结机器可判业务逻辑 oracle」、tasks 不放 determinism 任务、边界自证 `git diff -- crates/w3-core/` 为空、三版本号仍 9。
- **[Risk] 把 `craft-anything` 登记为第六个参考项目，或 T5 误以 `ConcordRTS` 为参考** → Mitigation：「## 参考项目对照」数量与交叉注记逐条声明、`UPSTREAM.md:114` 明令、沿用 p14 口径。
- **[Risk] `spec_gate_check()` 对 `status: pending` fail-closed 被误当失败去 self-approve / 写省略说明** → Mitigation：D6 如实登记为预期信号、不绕过、不填指纹 / `approved_by` / `date`；本轮完整 `verify_all.py` 由 leader 跑。
- **[Risk] 规则编号引用被冒充规则已实现、真窗口 / 双平台人工清单被标通过** → Mitigation：spec「非玩法边界」requirement + `decision-memo.md`「## 规则文档同步」无需同步结论（§14 第 5 条）、`critic` 清单第 13 项内容层复核。
- **[Risk] 文档「诚实边界」被省略或把缺口写成已验证，误导读者直接照抄运行 early alpha 工具** → Mitigation：spec「诚实边界」requirement 的 Grep / Read 逐项命中验证 + D2 备选③被否；工具本体尚未运行 / p15 未 apply / 试点未验证闭环醒目登记。

## Migration Plan

- **本轮（成文期）**：只在 `openspec/changes/p16-reverse-engineering-workflow/` 内产出四件套 + `decision-memo.md`；`git status --short` 仅该目录新增；`## 规格门` 保持 `status: pending`、AI 未 self-approve、不勾任一复选框；文档 `REVERSE-ENGINEERING-WORKFLOW.md` **不落盘**。
- **规格门**：`critic` 按第 7 / 10 / 12 / 13 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ 由 leader / `spec-steward` 在人确认后回填 `status: approved` + 四件套指纹 + `approved_by` + `date`（本轮不填）；此后规格锁定，改任一 artifact MUST 按 §8 同步回查其余三份并重走规格门。
- **apply（后续独立会话，时机 / owner 分工待 leader 拍板）**：按 D1–D7 落地，一个 change 一个 owner、不同时改多 owner 文件；先收敛规格门待确认项（章节粒度 / 索引 / 行号漂移 / apply 分工），再按拍板结论在 `docs/references/` 新增唯一一份 `REVERSE-ENGINEERING-WORKFLOW.md`（8 章节逐条对照 spec 的 Scenario 验证）；不碰 p12 / p13 / p14 / p15 / 归档件 / 主 spec / `crates/**` / `tools/**` / 根配置 / ADR / README / CONTRIBUTING / UPSTREAM / NOTICE / WC3-MELEE-RULES。
- **回滚**：本 change 成文产物为纯文档、apply 侧无运行时代码 / 权威状态 / 快照 / 版本号变更，删除 / 回退文档即可；反编译证据留仓库外本机、不随发行物分发，**无确定性回滚面**（ADR 0002）。
- **人工项**：人工 push → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` → `/opsx:archive`；AI 不 push / fetch / pull / clone、不 archive、不本地 commit（本轮由 leader 统一处理）。真窗口 8.x 人工验收、golden hash v9 双平台逐字节比对、CI 双平台一律「待人工 / 待 VM」。

## Open Questions

> 以下均为「可安全延后而不改变 specs / approach / task breakdown」的未知项，已由 D1 / D3 / D6 参数化承接并登记在 `decision-memo.md`「待确认项」，由规格门 / leader 拍板；本轮**不臆造为已定案**：

- 文档章节粒度（8 章节详略、逐项对照表确切行数、8 步「输入 · 工具 · 产物 · 落点 · 边界」字段口径）→ 待规格门用户拍板。
- 是否在 `UPSTREAM.md` / `README.md` 加索引 → 本轮明确不建；若后续要索引须先改本 change 四件套（触发 §8 规格解锁重走规格门），待规格门拍板。
- `craft-anything` 证据锚行号漂移 → 待规格门拍板「apply 阶段是否重新只读核对 / 是否用章节标题 + 行号双标」；取证流程的通用工程环节为本项目自定流程说明、不附带外部资料行号。
- apply 阶段执行时机与 owner 分工（一个 change 一个 owner）→ 待 leader 安排后续独立会话。
- 规格门批准记录回填（四件套指纹 / `critic` 报告 / `approved_by` / `date`）→ 待用户拍板后由 leader / `spec-steward` 回填，本轮一律不填。
- 参考矩阵 T1–T7 命中判定复核 → 本轮据 `UPSTREAM.md:104-114` 判定均不命中，待 `critic` 清单第 12 项复核；若改判命中须补对照报告并回到用户拍板。
