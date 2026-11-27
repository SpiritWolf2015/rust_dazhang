# Proposal

> 本轮只成文、不实施代码（apply 是后续独立会话）。本 change 为**非玩法 / 工作流治理（门禁与规则治理）**独立 change，用户 2026-11-25 明确拍板「新提独立工作流 change 并落成四件套」。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。两条护栏是本项目自定的工程治理约定（见 `design.md`「## 参考项目对照」与 `decision-memo.md`「承重约束」），不复制任何外部实现、不引入外部技术栈。

## Why

P0–P6 全部实施并归档后，仓库持续由跨会话 AI 协作推进，暴露两类**工作流治理**风险（非玩法缺陷、非内核缺陷），需以仓库权威文件与门禁而非口头惯例收口：① **上下文膨胀**：跨会话上下文随 `crates/**` 规模增长而膨胀，本项目却没有任何机械预算门禁；本项目已有 `check_encoding.py` / `check_source_conventions.py` / `verify_all.py` 的成熟门禁体例，缺一条「代码膨胀预算」护栏。② **临时约束被永久化**：一句「暂时不做 / 当期 change 内约束」会被后续会话当成永久禁令、让后续会话判断摇摆；本项目 P11 apply 真实撞到同类问题——`crates/w3-client/src/melee/defs.rs` 的 P8.1 当期约束「MUST NOT 新增 `EffectKind` 变体」被当作永久门禁，P11 需专门在 `design.md` D1 / 结论 4 澄清「被取代」、`client-owner` 改过期注释（仓库内可核实，锚见 `decision-memo.md`）。两者都只关乎「规则文字与门禁」，与 `w3-core` 权威状态、玩法、资产无关，故单列一个零内核改动的独立 change。

## What Changes

- 新增 capability `ai-workflow-guardrails`：用 SHALL/MUST 固化两条工程治理契约（代码膨胀预算门禁 / 临时约束时效与作用域规范），每条配可执行验证 `#### Scenario:`，并显式声明零确定性影响、零资产、零新增权威状态。
- **护栏 1 · 代码膨胀预算门禁**：apply 阶段新增 `tools/check_code_budget.py`（纯 Python、跨平台、平台差异用 `os.name`、注释中文 / 输出英文、UTF-8 无 BOM / LF），只读统计 `crates/**/*.rs`（及可选 `tools/*.py`）的**总行数 / 单文件行数 / 函数体行数 / 模块文件数**，超阈值报错并以非零退出码退出，挂进 `tools/verify_all.py` 一行。**阈值具体数值进 `decision-memo.md`「待确认项」由人拍板**；先做「基线测量」任务产出当前 `crates/` 规模分布，**不臆造外部项目规模数字**（阈值须以本项目基线测量为准）。硬阈值门禁 vs 仅文档约定、是否挂进 `verify_all.py`、是否纳入 CI 双平台同样列为待确认项。
- **护栏 2 · 临时约束标注时效 / 作用域**：apply 阶段在 `openspec/config.yaml` 的 `rules.*` 与 `CODEBUDDY.md`（§8 附近）增加惯例——凡写 `MUST NOT` /「本期不做」/「暂不」/「当期 change 内约束」类文字，MUST 标注作用范围（哪个 change / 模块）与失效条件；`spec-steward` 归档合并时 MUST 复核是否被后续 change 取代；`critic` 清单第 7 / 10 项加对应复核点。本轮只描述「apply 阶段将改」，不改这些权威文件。
- **真正改为「代码」的部分**：仅限 `tools/check_code_budget.py`（新脚本）与 `tools/verify_all.py` 的一行挂载；`CODEBUDDY.md` / `openspec/config.yaml` / `.codebuddy/agents/critic.md` 是规则文字，不是运行时代码。
- **明确不纳入**：反编译 / REA / MCP / Ghidra 工具链、并行多 agent 写同一 crate、抓帧对齐原版渲染、真窗口人工验收（8.x）、golden hash 双平台、Q1 清单水位、`p12-ability-doc-reconciliation` / `p13-ability-full-roster` 的任何 Scope，以及与本项目无关的外部技术栈或落地结论。

## Capabilities

### New Capabilities
- `ai-workflow-guardrails`: 两条工程治理契约 —— (1) 代码膨胀预算门禁：先做只读基线测量、阈值由人拍板后由 `tools/check_code_budget.py` 只读统计四类规模指标、超阈值显式失败、可挂进 `tools/verify_all.py`；(2) 临时约束时效与作用域规范：`config.yaml` / `CODEBUDDY.md` / `critic.md` 的临时类约束文字须带作用域与失效条件、归档合并复核取代。两条契约均含可执行验证口径与零确定性 / 零资产边界。

### Modified Capabilities
<!-- 无。本 change 只新增一个 capability；护栏 2 的规则文字落在 apply 阶段由 spec-steward / critic 维护，不改任何既有主 spec 的 requirement（不新增 MODIFIED delta）；本轮成文 `git diff -- openspec/specs/` 为空。 -->

## Impact

- **路线图阶段**：P14（P0–P5 及 P6–P11 全部已实施并归档，见 `CODEBUDDY.md §1` 与 `openspec list`）；本 change 是工作流 / 门禁 / 规则治理独立 change，**非玩法 change**，不实现任何 `R-x.y`、不动 1.0 范围 S1–S6、不改 `docs/references/WC3-MELEE-RULES.md`。
- **ADR**：0002（确定性契约：本 change 零 `w3-core` 权威状态改动，脚本只读统计、不进 FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION` 定义的快照、不改 `step()` / 命令排序；不 bump 三版本号，当前权威值 9）、0001（资产 / 法律：零暴雪资产、零 MPQ 派生数据、不新增第三方依赖、护栏为本项目自定约定不复制外部代码）、0003（1.0 范围：不扩范围、不碰玩法）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）；本 change **不新增 / 不改任何 ADR 正文**。
- **确定性契约**：**零 `w3-core` 权威状态改动、零确定性影响**。不新增权威状态字段（故无需进入状态哈希或 `SNAPSHOT_VERSION` 快照）、不进 `first_difference()`、不改帧序 / 命令排序 / 状态哈希编码；不 bump `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`（当前权威值均为 9，锚 `crates/w3-core/src/snapshot.rs:67-68` / `crates/w3-net/src/packet.rs:50`）；`git diff -- crates/w3-core/` 为空。`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用（不触确定性代码、不重跑 `cargo test -p w3-core --test determinism`）；tasks **不放** determinism 重跑任务，但放 `cargo test --workspace` / `python tools/verify_all.py` 保证整体不回归。**门禁脚本「故意造超阈值临时文件→报红→删除→通过」是门禁自身的改坏验红自证，不是 §5 针对 `w3-core` 权威状态回归测试的形态①反向验证，两者不得混淆**（见 `design.md` D4）。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效入库；不新增第三方依赖、`Cargo.lock` 不变；两条护栏为本项目自定的工程治理约定，不复制任何外部代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源、不登记为第六个参考项目（参考项目只读五个，见 `CODEBUDDY.md §1` / `UPSTREAM.md`）。
- **代码 / 系统**：本轮只成文，改动仅落在 `openspec/changes/p14-ai-workflow-guardrails/`；apply 阶段真正改「代码」的仅 `tools/check_code_budget.py` 与 `tools/verify_all.py` 一行挂载，其余为规则文字。不改 `crates/**` / `openspec/specs/**` / `docs/**` / 归档件 / 根配置 / p12 / p13。
- **参考项目矩阵**：`UPSTREAM.md:104-112` 的 T1–T7 **均不命中**（纯工程治理，非游戏格式 / 玩法 / 确定性内核 / 网络传输），见 `design.md`「## 参考项目对照」如实登记「不命中 + 理由 + 无参考锚、无参考缺口、不臆造」。
- **依赖 / CI**：不新增第三方依赖；是否挂进 `verify_all.py`、是否纳入 CI 双平台（`windows-latest` / `ubuntu-latest`）均列为待确认项，本轮不臆造为已定案。
