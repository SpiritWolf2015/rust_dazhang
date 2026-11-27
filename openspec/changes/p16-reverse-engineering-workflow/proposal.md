# Proposal

> 本轮只成文、不实施代码、不把文档落到 `docs/references/`（apply 是规格门批准后的后续独立会话）。本 change 是**纯文档交付 change**，承接 `p15-presentation-evidence-workflow`（已 `status: approved` + 四件套指纹锁定）：p15 定义「系统该做什么」，本 change apply 阶段的**唯一产物** `docs/references/REVERSE-ENGINEERING-WORKFLOW.md` 提供「怎么用的可照抄流程」——一份「clean-room 取证工具 ↔ 本项目 p15 形态 A」对照的反编译使用流程说明。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。本 change 不引入代码依赖、不放宽红线、未过规格门、未试点完成、未获暴雪授权；本轮文档尚未落盘。

## Why

p15 已定义呈现层证据流程规范，但团队缺一份带证据锚、带诚实边界的可照抄**流程参考说明**：clean-room 取证工具（`craft-anything`，本机外部工具，MIT OR Apache-2.0，early alpha）与本项目 p15 形态 A 怎么串成端到端流程需要成文（依据 `docs/decisions/0003-scope.md:109-123` 第二次修订与 `p15-presentation-evidence-workflow` 四件套）。本 change 只补齐「怎么用」的可照抄流程，不复制外部工具代码、不引入其技术栈、不作 WC3 玩法 / 格式 / 数值事实来源。

## What Changes

- `docs/references/` **新增一份** `REVERSE-ENGINEERING-WORKFLOW.md`（apply 阶段唯一产物）：文档定位声明块 + 取证流程的通用工程环节 + `craft-anything` 产品化实现 + 逐项对照表（约 10 行）+ 端到端 8 步流程 + 两条反馈循环 + 产物落点矩阵 + 诚实边界。
- **不改任何既有权威文件、不建索引到 `UPSTREAM.md` / `README.md`**（如需索引留待确认项，见 `decision-memo.md`「待确认项」）。
- 文档内容全部带可核验证据锚：仓库内锚为 `file:line` / ADR 编号；`craft-anything` 文件行号一律标注为「仓库文件外证据 / 人工确认项」并注明可能漂移；取证流程的通用工程环节为本项目自定的流程说明，不引用、不转述任何外部内部文章、不附带外部资料行号。文档 MUST 显式点明取证产出的是**行为规格 / 证据节点，不是代码**、不是 WC3 技术事实来源。
- **明确不做**：不改 `UPSTREAM.md` 五个参考项目表格；不把 `craft-anything` 登记为第六个只读参考项目；不写本机真实绝对路径、不含资产 / 派生数据 / 凭证；不改 ADR / `CODEBUDDY.md` / `README.md` / `CONTRIBUTING.md` / `NOTICE.md` / `WC3-MELEE-RULES.md` / `openspec/specs/**` / `crates/**` / `tools/**` / p15 四件套 / p12–p14 / P11 归档件；不改 `w3-core`、不 bump 三版本号、不接 CI、不改 determinism / golden hash 门禁；不改业务规则。
- **本 change 是非玩法 change**：不实现任何玩法规则编号 `R-x.y`；`decision-memo.md`「## 规则文档同步」小节按 `WC3-MELEE-RULES.md §14` 第 5 条给出「无需同步」结论（pending / 非玩法豁免），保持非空。

## Capabilities

### New Capabilities
- `reverse-engineering-workflow-reference`: 反编译证据流程参考说明的行为契约 —— 以单一新增文档落到 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`，含定位声明块 / 取证流程的通用工程环节 / 逐项对照表 / 8 步流程 / 两条反馈循环 / 产物落点矩阵 / 诚实边界；每个章节用 SHALL/MUST 与可执行的 Grep / Read / 文件存在性验证表达；显式声明零资产 / 零新增权威状态 / 零确定性影响 / 不 bump / 不接 CI / 五个只读参考项目不变 / 文档目标路径与章节可核验。

### Modified Capabilities
<!-- 无。本 change 只新增一个 capability；本轮成文不改任何既有主 spec 的 requirement（不新增 MODIFIED delta）；`git diff -- openspec/specs/` 为空。 -->

## Impact

- **路线图阶段**：P16（P0–P5 及 P6–P15 全部已实施并归档 / 成文，见 `CODEBUDDY.md §1` 与 `openspec list`）；本 change 是纯文档 / 参考说明交付 change，**非玩法 change**，不实现任何玩法规则编号 `R-x.y`、不动 1.0 范围 S1–S6、不改 `docs/references/WC3-MELEE-RULES.md`。
- **ADR**：0003（第二次修订，承接复核点、形态 A 试点；`:118-119` 红线）、0001（复核记录，零资产 / 不复制代码 / 不绕 EULA · DRM · 登录 · 反作弊 / 产物留本机不入库）、0002（表现层只读派生、反编译证据不进权威状态 / 哈希 / 快照 / golden hash / determinism / CI）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）；本 change **不新增 / 不改任何 ADR 正文**。合规措辞已由人拍板落地于 `README.md:29` / `CONTRIBUTING.md:11`（commit `6bdf3ae`），本 change 不碰。
- **确定性契约**：**零 `w3-core` 权威状态改动、零确定性影响**。不新增权威状态字段（故无需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，当前 9，锚 `crates/w3-core/src/snapshot.rs:67`）、不进 `first_difference()`、不改帧序 / 命令排序 / 状态哈希编码；不 bump 三版本号（当前 9）；不改 determinism 与 golden hash 门禁。`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用，tasks **不放** determinism 重跑任务。反编译证据属视觉 oracle / 行为规格，**不是** ADR 0002 口径「可冻结、机器可判、可进 CI 的业务逻辑 oracle」。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效入库；不新增第三方依赖、`Cargo.lock` 不变、不登记 `NOTICE.md`；仅用于用户合法自备的经典客户端、不绕过 EULA / DRM / 登录 / 反作弊；不声称获暴雪授权；不写本机真实绝对路径 / 凭证。`craft-anything`（本机外部工具）MUST NOT 登记为第六个只读参考项目；五个只读参考项目仍是 `vera20k` / `godot_warcraft3` / `rts_kernel_cs` / `WarsmashModEngine` / `ConcordRTS`（`CODEBUDDY.md §1`、`UPSTREAM.md:9-15,96,100`）。
- **代码 / 系统**：本轮只成文，改动仅落在 `openspec/changes/p16-reverse-engineering-workflow/`；不改 `crates/**` / `openspec/specs/**` / `docs/**` / `tools/**` / 归档件 / 根配置 / p12 / p13 / p14 / p15。apply 阶段唯一产物为 `docs/references/REVERSE-ENGINEERING-WORKFLOW.md`，本轮不落盘。
- **参考项目矩阵**：`docs/references/UPSTREAM.md:104-114` 的 T1–T7 经核对**均不命中**（T7 可选交叉、仅红线）；本 change 与 `p14-ai-workflow-guardrails` 同属纯文档 / 参考说明交付，无参考项目新增借鉴、无新增参考项目锚、无参考缺口，逐条判定见 `design.md`「## 参考项目对照」，待 `critic` 清单第 12 项复核。
- **依赖 / CI**：不新增第三方依赖、不接 CI；`craft-anything` early alpha / Clean room 不是 OS 沙箱 / Deep + Clean room 组合无 live 验证先例 / 工具本体尚未运行 / p15 未 apply / 试点未验证闭环等缺口如实登记为「诚实边界」与「待确认项」。
