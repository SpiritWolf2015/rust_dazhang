# Spec Delta

## Purpose

本 capability 定义反编译证据流程参考说明的行为契约：以单一新增文档 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 提供「clean-room 取证工具 ↔ 本项目 p15 形态 A」对照、可照抄的端到端使用流程，带证据锚、合规与确定性边界及诚实边界；文档定位声明不引用、不转述任何外部内部文章；产出是纯文档而非代码或业务逻辑 oracle，零资产、零新增权威状态、零确定性影响，非玩法 change。

## ADDED Requirements

### Requirement: 唯一目标路径与文档定位声明块

系统 SHALL 在 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 提供**唯一一份**反编译使用流程参考说明；apply 阶段为本 change 唯一产物，本轮成文期不落盘。文档 SHALL 沿用 `WC3-MELEE-RULES.md:1-13` / `UPSTREAM.md:1-5` 体例，以「文档定位声明块」开头：不是上游参考项目、五个只读参考项目仍在 `UPSTREAM.md`、合规边界（ADR 0001）、确定性边界（ADR 0002）、状态标记、建立日期 2026-11-25、维护责任 `docs-owner`。

#### Scenario: 目标路径与定位声明块可核验

- **WHEN** 规格门批准后的 apply 阶段落盘该文档后核对开头声明块
- **THEN** 文件存在性验证 `Glob pattern="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中 1 个文件；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md:1-13` 显示「文档定位」标题与「不是上游参考项目」「五个只读参考项目」「合规边界」「确定性边界」「状态标记」「建立日期 2026-11-25」「维护责任 `docs-owner`」逐条；`Grep pattern="不是上游参考项目|UPSTREAM.md|ADR 0001|ADR 0002|2026-11-25|docs-owner" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中上述 7 项

#### Scenario: 合规与确定性边界逐条命中

- **WHEN** 核对定位声明块的合规边界与确定性边界两段
- **THEN** `Grep pattern="零资产入库|MPQ 派生数据不入库|留本机仓库外|EULA|DRM|登录|反作弊|合法自备经典客户端" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中 ADR 0001 合规段（锚 `docs/decisions/0003-scope.md:118` / `README.md:29` / `CONTRIBUTING.md:11`）；`Grep pattern="视觉 oracle|不进权威状态|状态哈希|快照|golden hash|determinism|CI 门禁|ADR 0002" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中 ADR 0002 确定性段（锚 `docs/decisions/0003-scope.md:119`）

### Requirement: 取证流程的通用工程环节

文档 SHALL 以一节写取证流程的通用工程环节，含六个小节：触发与目标收敛、生产侧 / 消费侧分离与文档交接、长程任务分类、并行与调度、证据结构化与独立重审、工程护栏。文档定位声明 MUST 注明「不引用、不转述任何外部内部文章」；取证产出的是**行为规格 / 证据节点，不是代码**、不是 WC3 技术事实来源。本节为本项目自定的流程说明，不附带任何外部资料行号。

#### Scenario: 通用工程环节六小节可核验

- **WHEN** 核对取证流程的通用工程环节一节
- **THEN** `Grep pattern="取证流程的通用工程环节|触发与目标收敛|生产侧 / 消费侧分离与文档交接|长程任务分类|并行与调度|证据结构化与独立重审|工程护栏" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中全部锚词；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 该节含「生产侧 / 消费侧」「只读文档交接」「判定与产物同提交」「独立重审」逐条文字

#### Scenario: 不引用外部文章与产出定性可核验

- **WHEN** 核对文档定位声明与产出定性说明
- **THEN** `Grep pattern="不引用、不转述任何外部内部文章|通用工程实践|行为规格|证据节点|不是代码|独立重审" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中；文档 MUST NOT 附带任何外部资料行号，也 MUST NOT 把取证产出写成代码

### Requirement: craft-anything 产品化实现与证据锚

文档 SHALL 以一节写 `craft-anything` 的产品化实现，覆盖主循环 `Study→Build→Check→Ready`、Clean room / Deep 双模式、角色隔离、证据工作记忆、反编译工具组、对照 / Check 工具组、项目目录与入口、合规八类要素；每类 MUST 带对应文件行号锚（逐类清单见下方 Scenario 的可执行 Grep）。文档 MUST 标注该工具为本机外部工具（MIT OR Apache-2.0、early alpha）、行号为当时工作树快照 / 人工确认项、可能漂移，MUST NOT 把行号写成仓库内 `file:line`。

#### Scenario: 主循环 / 双模式 / 角色隔离 / 工作记忆锚可核验

- **WHEN** 核对 `craft-anything` 产品化实现一节前半段
- **THEN** `Grep pattern="Study→Build→Check→Ready|96-105|Clean room|Deep|132-150|clean_room.rs:14-158|AGENTS.md:23|claude-code.md:18-25|46-60" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中；`Grep pattern="skill.md:38-58|61-74|107-108|tools.md:7-44|brief|plan|hypothesis|finding|handoff|journal|ev_|observed|derived|inferred|Absence is not proof|先预测再验证" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中证据工作记忆全部要素

#### Scenario: 工具组 / 目录入口 / 合规锚可核验

- **WHEN** 核对 `craft-anything` 产品化实现一节后半段
- **THEN** `Grep pattern="tools.md:46-102|tools.md:228-313|file-formats.md:15-16|README.md:226-229|README.md:176-187|210-224|README.md:259-262|README.md:143-147|skill.md:89-95" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中反编译工具组 / 对照 Check 工具组 / 项目目录 / 入口 / 合规全部行号锚；文档 MUST 标注 `craft-anything` 为本机外部工具（MIT OR Apache-2.0、early alpha）、行号为当时工作树快照 / 人工确认项、可能漂移

### Requirement: 逐项对照表与端到端 8 步流程

文档 SHALL 含「逐项对照表」（通用工程实践 → `craft-anything` 对应 → 本项目 p15 形态 A，约 10 行）与「端到端 8 步流程」：立目标 / Deep 取证 / 证据结构化 / 独立重审（判定与产物同提交）/ 文档交接（仓库外证据目录 → 主仓库只读）/ Build（隔离 agent 不接触原版安装）/ Check（离屏 `render_headless_rgba` 或 `render_headless_rgba_with_textures` + ViewImage + 真窗口人工验收）/ 收敛或回炉（不达标转新抓取需求）。8 步中每一步 MUST 标「输入 · 工具 · 产物 · 落点 · 边界」五要素。

#### Scenario: 对照表三列与数据行可核验

- **WHEN** 核对「逐项对照表」一节
- **THEN** `Grep pattern="通用工程实践|craft-anything 对应|p15 形态 A|形态 A" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中三列表头；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 该表数据行 ≥ 9 且 ≤ 14（实际 10 行），每行三列分别落在通用工程实践 / 工具 / 本项目 p15 三侧

#### Scenario: 8 步流程与五要素可核验

- **WHEN** 核对「端到端 8 步流程」一节
- **THEN** `Grep pattern="立目标|Deep 取证|证据结构化|独立重审|判定与产物同提交|文档交接|Build|Check|收敛或回炉|不达标转新抓取需求" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中 8 步名称；`Grep pattern="render_headless_rgba|render_headless_rgba_with_textures|ViewImage|真窗口人工验收|隔离 agent|不接触原版安装|输入|工具|产物|落点|边界" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中 Check / Build 细节与「输入 · 工具 · 产物 · 落点 · 边界」五要素表头，且每步都带该五要素

### Requirement: 两条反馈循环

文档 SHALL 写两条反馈循环：① 探索 ↔ 实现（Study / Build 之间反复）；② Check → 新抓取需求（不达标回炉，用 `README.md:222` 的 `--continue-in` / `--rounds` 再取证）。两条循环 MUST 标明「不达标转新抓取需求」而非「标记通过」。

#### Scenario: 两条反馈循环可核验

- **WHEN** 核对「两条反馈循环」一节
- **THEN** `Grep pattern="探索|实现|Check|新抓取需求|--continue-in|--rounds|README.md:222|不达标转新抓取需求" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中两条循环及其工具入口；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 该节明确区分「探索↔实现」与「Check→新抓取需求」两条回路，MUST NOT 写成「Check 不达标即标记通过」

### Requirement: 产物落点矩阵

文档 SHALL 含「产物落点矩阵」，对伪 C / 调用图 / 行为规格 / 离屏 RGBA / MPQ 派生数据 / 状态哈希逐项标「能否入库」「能否进权威状态或门禁」，并给出依据（`docs/decisions/0003-scope.md:118-119`、`README.md:29`、`CONTRIBUTING.md:11`）。矩阵 MUST 给出「反编译产物与 MPQ 派生数据不入库、不进权威状态 / 哈希 / 快照 / golden hash / determinism / CI 门禁」的结论。

#### Scenario: 六项产物两列判定与依据可核验

- **WHEN** 核对「产物落点矩阵」一节
- **THEN** `Grep pattern="伪 C|调用图|行为规格|离屏 RGBA|MPQ 派生数据|状态哈希|能否入库|能否进权威状态或门禁" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中六项产物与两列判定表头；`Grep pattern="0003-scope.md:118-119|README.md:29|CONTRIBUTING.md:11|ADR 0001|ADR 0002" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中依据；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 该矩阵中伪 C / 调用图 / 行为规格 / 离屏 RGBA / MPQ 派生数据 / 状态哈希六行均给出「不入库 / 不进权威状态或门禁」结论

### Requirement: 诚实边界

文档 SHALL 含「诚实边界」一节，逐条登记 `craft-anything` early alpha（`README.md:240-248`）、`docs/claude-code.md:63` 未 live 验证、Deep + Clean room 组合无先例（p15 待确认项⑤）、Clean room 不是 OS 沙箱（`README.md:143-147`）、无 MPQ / BLP / MDX / SLK 专用解析器（`crates/formats/README.md:1-75`）→ WC3 资产与数据表仍靠 `godot_warcraft3` 导出管线；反编译证据是视觉 oracle、不进 ADR 0002 门禁；`reference-researcher` 对照报告按 §9 只读体例未落盘为仓库文件；真窗口 8.x / golden hash v9 双平台 / CI 双平台待人工或 VM；**工具本体尚未运行 / p15 未 apply / 试点未验证闭环**。

#### Scenario: 诚实边界各缺口可核验

- **WHEN** 核对「诚实边界」一节
- **THEN** `Grep pattern="early alpha|240-248|claude-code.md:63|Deep \+ Clean room|p15 待确认项|不是 OS 沙箱|143-147|MPQ|BLP|MDX|SLK|formats/README.md:1-75|godot_warcraft3|视觉 oracle|reference-researcher|真窗口|golden hash v9|CI 双平台|工具本体尚未运行|p15 未 apply|试点未验证闭环" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中全部缺口；`Read docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 该节 MUST NOT 把上述缺口写成「已验证可用 / 已试点完成 / 已落盘 / 已获授权」

### Requirement: 明确不做边界与确定性 / 资产零影响

本 change SHALL 只在 `docs/references/` 新增该一份文档；MUST NOT 改 `UPSTREAM.md` 五个参考项目表格、不登记第六个只读参考项目、不写本机真实绝对路径 / 资产 / 派生数据 / 凭证、不改 ADR / `CODEBUDDY.md` / `README.md` / `CONTRIBUTING.md` / `NOTICE.md` / `WC3-MELEE-RULES.md` / `openspec/specs/**` / `crates/**` / `tools/**` / p15 四件套 / p12–p14 / P11 归档件、不改 `w3-core`、不 bump 三版本号、不接 CI、不改 determinism / golden hash 门禁、不改业务规则。零新增权威状态（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，当前 9），不重跑 `cargo test -p w3-core --test determinism`。

#### Scenario: 明确不做清单与零改动可核验

- **WHEN** 收尾核对本 change 的变更边界
- **THEN** `Grep pattern="UPSTREAM.md|第六个|本机真实绝对路径|NOTICE.md|WC3-MELEE-RULES.md|openspec/specs/|crates/|tools/|w3-core|不 bump|不接 CI|golden hash|业务规则" path="docs/references/REVERSE-ENGINEERING-WORKFLOW.md"` 命中「明确不做」清单；`git status --short` 仅新增 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 一个文件；`git diff -- crates/w3-core/` / `-- openspec/specs/` / `-- tools/` 为空，三版本号仍 == 9（锚 `crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`）；`git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 派生 JSON / PNG / GLB

### Requirement: 非玩法边界与规则文档同步结论

本 capability SHALL 声明为**非玩法 / 纯文档（反编译使用流程参考说明）**能力：不实现任何玩法规则编号 `R-x.y`、不碰 1.0 范围 S1–S6。`decision-memo.md`「## 规则文档同步」SHALL 按 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条给出「无需同步」结论并写明依据，保持非空；MUST NOT 把规则编号引用冒充规则已实现。

#### Scenario: 无需同步结论与豁免可核验

- **WHEN** 收尾运行规则文档同步结构门禁与内容复核
- **THEN** `Read openspec/changes/p16-reverse-engineering-workflow/decision-memo.md` 的「## 规则文档同步」小节非空且给出「无需同步」结论（非玩法 / 不承接玩法规则编号 / 不改 §13.7 / §15 / 附录覆盖对照 / 参考项目数量仍五个）；`git diff -- docs/references/WC3-MELEE-RULES.md` 为空；`design.md` 不含 `R\d+` 形式的台账编号（避免 `RULE_REF_PATTERN` 误判）；`rule_doc_sync_check()` 对 `status: pending` / 非玩法 change 豁免
