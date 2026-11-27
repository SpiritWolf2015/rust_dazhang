# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「怎么实现这三项成文口径订正」。事实来源均为已归档 `openspec/changes/archive/2026-11-25-p11-ability-system/` 的「## 已知限制」与本仓库 artifacts（可核验锚，非参考项目结论）：

- **三版本号权威值**：`crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13` / `CODEBUDDY.md §5` 亦记「当前 9」。P11 已 bump 8→9（`docs/decisions/0002-determinism.md` P11 复核记录 / `docs/decisions/0005-protobuf-payload.md` P11 修订记录）。
- **主 spec 历史 `==8` 残留**：散布 `openspec/specs/ai-player-lockstep/spec.md:34`、`openspec/specs/deterministic-simulation/spec.md:44,66`、`openspec/specs/dual-faction/spec.md:82`、`openspec/specs/undead-faction/spec.md:104`、`openspec/specs/ability-system/spec.md:4,67`、`openspec/specs/gameplay-systems/spec.md:42`、`openspec/specs/lockstep-networking/spec.md:12`、`openspec/specs/melee-match/spec.md:82` 等的 THEN / 正文（约 11 处）；根因是 P9 / P10 / P11 历次 bump 后历史 delta 合并残留，按 P9 Q9' 已拍板「归档后由独立 change 收口」（P11 `tasks.md` 7.10 / `decision-memo.md`「已知限制」）。
- **清单规模登记偏差**：P11 `design.md`「清单规模事实锚」登记 `AbilityData.json` 803 条 / **91 列**（`:20`），content-owner 本机导出实测 **803 条 / 95 列**，其余规模元数据全部一致（P11 `decision-memo.md`「已知限制」第 8 条）。
- **tasks 5.2 表述歧义**：P11 `tasks.md` 5.2（`:31`）字面「改任一冻结常量即改变 `content_hash()`、**不改运行时 `compute_state_hash()`**」；而 `crates/w3-core/src/sim_match.rs:4328`（`hasher.write_u64(self.config.content_hash)`）/ `:4375`（`hasher.write_u64(self.defs.content_hash())`）/ `:4418-4419`（`first_difference` 比较 `content_hash`）与 `docs/decisions/0002-determinism.md §6`「参与字段固定：… / `content_hash` / …」证明 `compute_state_hash()` 自 P5 起把冻结内容身份作为**既有字段**写入（P11 `decision-memo.md`「已知限制」第 9 条）。属成文表述与既有契约的歧义、非实现缺陷。
- **归档件锁定**：P11 四件套已 `status: approved` + 指纹锁定（`decision-memo.md`「## 规格门」），改任一即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门；`tools/verify_all.py:175-195` 的四件套指纹集合仅含 change 目录内 `proposal.md` / `design.md` / `specs/**/*.md` / `tasks.md`，**不含**主 `openspec/specs/**`、不含 `decision-memo.md`。

**参考矩阵命中判定**：本 change 为纯文档 / 规格口径订正，不改确定性内核实现（T1）、不改 WC3 数据格式 / 导出管线（T3）、不改玩法语义（T4）、不改场景 / 网络 / 通用架构实现（T5 / T6 / T7）、不改移动（T2）；经核对 `docs/references/UPSTREAM.md:104-112` 矩阵 T1–T7 **均不命中**。「## 参考项目对照」小节据实登记该判定，不臆造参考结论、不引入参考项目锚。

## Goals / Non-Goals

**Goals:**
- 新增单一 capability `doc-version-reconciliation`，用 SHALL/MUST 把三项成文口径契约与可执行验证（全仓 Grep 残留为 0 / `openspec validate --all` / `python tools/verify_all.py` / `python tools/check_encoding.py`）一次成文。
- 明确 apply 阶段的订正落点（改主 spec 的 THEN 口径、后续主 spec / 文档的规模与表述口径），逐字保留历史 requirement 与 Scenario 身份键、不抹平 bump 8 沿革史实、不追溯改写归档件。
- 显式声明零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不跑 `determinism` 硬门禁。

**Non-Goals:**
- 不改 `crates/w3-core/` / `crates/w3-net/` / `crates/w3-client/` / `crates/w3-content/` 任何实现；不改 ADR 0002 §1–§8 正文与运行时哈希字段集合；不 bump 三版本号（权威值 9 不变）；不新增权威状态。
- 不改 P11 归档件四件套与 `decision-memo.md`（已锁定、不追溯改写）；不改 `docs/decisions/**` / `docs/references/**` / `tools/**` / 根配置 / `CODEBUDDY.md`。
- **不纳入**：顺延项（中立英雄 / 野怪 / 全部单位技能）与 Q1 逐项 SLK 对齐（属 `p13-ability-full-roster`）、真窗口 8.x 人工清单、golden hash 双平台、CI 双平台、参考缺口、`vera20k` HEAD 前进、Q-A / 形态①时序澄清（分属人工 / VM 项或历史事实）。
- 本轮只成文、不实施代码；不 archive / 不 commit / 不 push / 不 self-approve 规格门。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、对跨平台确定性的影响（ADR 0002）、分层依赖 / 架构原则。全部决策均为**纯文档 / 规格口径订正**，对确定性契约零影响。

### D1 订正落点：apply 阶段改主 spec 的 THEN、不在本 change 内新增 MODIFIED delta

- **决策**：主 spec 历史 `==8` 无条件硬断言的订正落在 **apply 阶段由 `spec-steward` 直接改主 `openspec/specs/**` 的 THEN / 正文**（或 archive 合并时复核）；本 change 只新增一个 ADDED capability，**不新增第 10 份 MODIFIED delta 凑数**。订正逐字保留 `### Requirement:` 标题与其下各 `#### Scenario:` 标题（身份键）、口径只改在 THEN；历史沿革叙述里 P9 / P10 / P11 各 change 的「bump 8」实施期史实原样保留、不抹平。提案 / 成文阶段 `git diff -- openspec/specs/` 为空。
- **备选方案（被否）**：① 为本 change 写 10+ 份 MODIFIED delta 逐份复制整块 requirement（被否：用户明确「不新增第 10 份 MODIFIED delta 凑数」；且 OpenSpec 要求 MODIFIED 复制整块 requirement，逐份改写会在归档时与主 spec 既有口径叠床架屋）；② 全仓正则把字面 `8` 一律替换为 `9`（被否：会抹平历史 bump 8 沿革史实、误伤无关数字 `8` 与非版本上下文）；③ 提案阶段即直接编辑主 spec（被否：P11 `design.md` D0「主 spec 在 archive 阶段合并、提案阶段不直接编辑」＋用户明确「proposal 只描述要做什么」、承诺 `git diff -- openspec/specs/` 成文阶段为空）。
- **依据**：P9 Q9'「归档后由独立 change 收口」（P11 `tasks.md` 7.10 / `decision-memo.md`「已知限制」）；P11 `design.md` D0 / `proposal.md`「Modified Capabilities」注释；`tools/verify_all.py:182-186`（指纹集合不含主 `openspec/specs/**`）；`openspec instructions specs` 的 MODIFIED 整块复制规则。ADR 0002（版本号是确定性契约的一部分，字面须与权威定义一致）。
- **代价**：apply 阶段需逐份核对约 11 处、区分「当前版本号消费者」与「历史 bump 8 史实」，人工核对量落到 apply。
- **对跨平台确定性的影响**：无。仅改 Markdown THEN 字面，不进 FNV-1a 状态哈希 / 快照 / `first_difference()`；三版本号常量不变（ADR 0002 §6-8 零改动）。
- **分层依赖 / 架构原则**：SRP（一个 capability 只拥有文档口径一致性契约）、显式失败不静默降级、避免重复定义。

### D2 清单规模订正口径：95 列或删除具体列数统一粗统计

- **决策**：后续主 spec / 文档把 `AbilityData.json` 列数由 91 订正为**与实测一致的 95 列**，或删除具体列数、统一为「粗统计、以人工核对为准」表述（两者均合规，最终选哪一口径列入 `decision-memo.md`「待确认项」、由规格门拍板，不臆造为已定案）。其余规模元数据原样一致；只登记规模与分类、不登记任何具体技能名 / 四字符 rawcode / 数值。
- **备选方案（被否）**：① 追溯改写 P11 `design.md` 的 91 列（被否：归档件已 `status: approved` + 指纹锁定，改任一 artifact 即触发 `CODEBUDDY.md §8` 规格解锁须重走规格门由人拍板；用户明确「不追溯改写归档件」）；② 保留 91 列不改（被否：数字过时、与 content-owner 实测矛盾）；③ 把 803 / 95 等数字写成精确结论（被否：`WC3-MELEE-RULES.md §14` 与本仓库历期口径要求「粗统计、以人工核对为准、不臆造精确数字」）。
- **依据**：P11 `design.md`「清单规模事实锚」`:20` / `decision-memo.md`「已知限制」第 8 条；ADR 0001（零资产、只借规模语义）/ ADR 0004（冻结定义、导出数据不入库）；`WC3-MELEE-RULES.md §14`。95 列为 content-owner 本机导出只读实测（仓库文件外证据、人工确认项）。
- **代价**：口径二选一未拍板，apply 阶段按拍板结论落地；规模数字仍标注为粗统计、不冒充精确。
- **对跨平台确定性的影响**：无。规模元数据不进权威状态 / 状态哈希 / 快照；本机导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库（ADR 0001 / ADR 0002 §6）。
- **分层依赖 / 架构原则**：DIP（文档只消费规模语义、不读导出文件）、零资产红线。

### D3 冻结定义哈希契约表述：身份传导口径、非实现缺陷

- **决策**：把 P11 `tasks.md` 5.2「不改运行时 `compute_state_hash()`」订正为**准确口径**——「改任一冻结常量会经既有 `config.content_hash` / `defs.content_hash()` 身份传导改变 `compute_state_hash()`；冻结定义未作为独立运行态字段二次混入」。订正落在后续主 spec / 文档表述，**不追溯改写 P11 归档件、不改 ADR 0002 §1–§8 正文、不改运行时哈希字段集合**；明确标注为「成文表述订正、非实现缺陷」。
- **备选方案（被否）**：① 判定为实现缺陷并去改 `compute_state_hash()`（被否：用户明确「成文表述订正、非实现缺陷」；`sim_match.rs:4328` / `:4375` 既有写入正是 ADR 0002 §6 契约的正确实现，改它反而破坏确定性硬门禁）；② 改写 ADR 0002 §1–§8 正文以迁就旧表述（被否：ADR 记「为什么决定」、正文基线不删改，历期复核记录只追加，见 `0002-determinism.md`「复核记录」纪律）；③ 直接删除该条不解释（被否：会让 spec 与实现脱节、留隐性歧义）。
- **依据**：`crates/w3-core/src/sim_match.rs:4328,4375,4418-4419`；`docs/decisions/0002-determinism.md §6`（`content_hash` 为参与字段）/ §8 验收门槛；P11 `tasks.md` 5.2 / `decision-memo.md`「已知限制」第 9 条；ADR 0002 / ADR 0004。
- **代价**：需向 reviewer 解释「定义层冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部」与「`config.content_hash` 作为既有运行态字段」两条路径的区别，避免再被读成矛盾。
- **对跨平台确定性的影响**：无。表述订正不改 `compute_state_hash()` 的字段集合与编码顺序，定宽整数 + 显式小端 + FNV-1a 不变（ADR 0002 §6）；不新增 / 不删除任何权威状态字段。
- **分层依赖 / 架构原则**：ADR 记「为什么」、spec 记「系统该做什么」、文档表述不得与实现契约打架（`CODEBUDDY.md §8`）；显式失败不静默降级。

### D4 边界：零权威状态 / 零确定性影响 / 不 bump / 不动资产 / 不重跑 determinism

- **决策**：本 capability 全部订正为纯文档 / 规格口径表述；MUST NOT 改 `crates/w3-core/` 任何权威状态、哈希、快照、`first_difference()`；不新增权威状态（故无需声明进入状态哈希或由 `SNAPSHOT_VERSION`（当前 9）定义的快照）、不 bump；`CODEBUDDY.md §5` 硬门禁与形态①反向验证**不适用**（不触确定性代码），tasks 不含重跑 `cargo test -p w3-core --test determinism` 的任务；资产扫描与 `python tools/check_encoding.py` 仍执行。
- **备选方案（被否）**：① 为「保险」也跑一遍 `determinism`（被否：本 change 零内核改动、无需触发 ADR 0002 硬门禁，用户明确 p12「不跑 determinism 硬门禁」；写成任务反而暗示存在内核改动、与 proposal / spec 打架）；② 顺手把三版本号再 bump 一次（被否：越权、与权威值 9 不变矛盾，属 p13 或后续版本处置）。
- **依据**：`CODEBUDDY.md §5` / §6；`openspec/config.yaml rules.specs`（新增权威状态才需声明哈希 / 快照）；P11 归档件已锁定。ADR 0001 / ADR 0002 / ADR 0004 / ADR 0005。
- **代价**：验收线只到 Grep / `openspec validate --all` / `verify_all.py` / `check_encoding.py`，不含内核测试；如实登记、不冒充全量门禁。
- **对跨平台确定性的影响**：无（零字段改动、零编码变动）；不改变 ADR 0002 已记录的「Windows / Debian 12 x86_64 逐字节比对已验证、macOS / ARM64 待验证」口径。
- **分层依赖 / 架构原则**：唯一状态所有者不受影响、零资产红线、显式不冒充已验证。

### D5 规格门：保持 pending、不 self-approve、不写「## 省略说明」

- **决策**：`decision-memo.md`「## 规格门」小节只写 `status: pending`，不填 `fingerprint` / `approved_by` / `date`，不写「## 省略说明」（本 change 非「目标与方向已清楚的小改动」，豁免会令 `spec_gate_check()` 变绿、违反 fail-closed 信号）。四件套指纹、`critic` 规格门报告、`reference-researcher` 对照报告（本 change 不命中矩阵、无报告）、批准记录均由 leader / 用户后续拍板回填。
- **备选方案（被否）**：① 自填 `status: approved` + 指纹（被否：`CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve）；② 写「## 省略说明」求门禁变绿（被否：用户明确禁止、该豁免只对小改动）。
- **依据**：`tools/verify_all.py:207-253`（`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'`）/ `:143-172`（`decision_memo_check()` 三必填小节）/ `:256-294`（`rule_doc_sync_check()` 对 pending 豁免）；`openspec/decision-memo.template.md:55-61`；`CODEBUDDY.md §8` / §9。
- **代价**：`python tools/verify_all.py` 的 `spec_gate_check()` 对 pending 报 fail-closed 属**预期信号**，本轮不绕过、不修复、不冒充放行。
- **对跨平台确定性的影响**：无（纯流程，不进哈希 / 快照）。
- **分层依赖 / 架构原则**：批准由人拍板、不拿沉默当授权。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-112` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。**本 change 经核对 T1–T7 均不命中**（纯文档 / 规格口径订正，不改任何参考矩阵所列的实现层），故如实登记「无参考项目借鉴、无参考项目锚、无参考缺口」，不臆造对照结论：

| 矩阵 | 命中 | 判定依据 |
|---|---|---|
| T1 确定性内核 / 状态哈希 / 快照（`UPSTREAM.md:106`，`vera20k`） | 不命中 | D3 只订正 ADR 0002 既有 `content_hash` 身份字段的**成文表述**（`sim_match.rs:4328` 等本仓库锚），不改内核实现、不 bump、不新增权威状态 |
| T2 移动 / 寻路（`UPSTREAM.md:107`，`rts_kernel_cs`） | 不命中 | 不触及移动语义，`double` 路线本就不采用（ADR 0002） |
| T3 WC3 数据格式 / SLK（`UPSTREAM.md:108`，`godot_warcraft3`） | 不命中 | 清单规模 95 列是 content-owner 本机导出实测（仓库外证据），不是参考项目结论；不改导出管线 / schema |
| T4 玩法语义 / 技能（`UPSTREAM.md:109`，`WarsmashModEngine`） | 不命中 | 不实现任何技能 / 玩法语义；顺延项与逐项 SLK 对齐属 `p13`，本 change 明确排除 |
| T5 场景 / HUD（`UPSTREAM.md:110`，`godot_warcraft3`） | 不命中 | 不触及场景装配 / 呈现 |
| T6 网络传输（`UPSTREAM.md:111`，`vera20k`） | 不命中 | 只订正 `PROTOCOL_VERSION` 的 spec 字面，不改 W3N1 帧头 / relay / proto schema（ADR 0005） |
| T7 通用 RTS 架构（`UPSTREAM.md:112`，`ConcordRTS`） | 不命中 | 不触及确定性内核与表现层分离 / 平台适配；且其 RA2/YR 专属格式 MUST NOT 作 WC3 事实来源 |

- **五个参考项目只读、不复制代码（ADR 0001）**：`vera20k`（GPLv3 + Section 7）、`godot_warcraft3`（MIT）、`rts_kernel_cs`（MIT）、`WarsmashModEngine`（AGPLv3，仅借鉴语义、禁复制代码）、`ConcordRTS`（GPL-3.0-only，其 RA2/YR 规则不作 WC3 事实来源）——本 change 全部**不借鉴、不引入依赖**。
- **承重约束证据锚全部来自本仓库 artifacts**（归档件 `decision-memo.md` / 主 `openspec/specs/**` / `crates/**` 源码常量 / ADR / 命令输出），见「## Context」与 `decision-memo.md`「承重约束」表；无一条来自参考项目，故无「矩阵条目 / 参考项目内路径」类锚需要双标。
- **参考缺口：无**（本 change 不命中矩阵、不需 `reference-researcher` 报告）；p13 命中 T3 / T4 / T1 的对照与缺口由 `p13-ability-full-roster/design.md` 承接、回到用户拍板、不臆造。

## Risks / Trade-offs

- **[Risk] apply 阶段把历史 bump 8 沿革史实一并改成 9** → Mitigation：`spec.md` Req1 / D1 明确逐字保留 requirement 与 Scenario 身份键、口径只改 THEN、沿革叙述原样保留；全仓 Grep 只针对「当前版本号消费者」硬断言。
- **[Risk] 追溯改写 P11 归档件触发 `CODEBUDDY.md §8` 规格解锁** → Mitigation：D2 / D3 / spec Scenario 明确归档件四件套与 `decision-memo.md` 零改动，`git diff -- openspec/changes/archive/` 为空。
- **[Risk] 把 tasks 5.2 表述订正误读为实现缺陷、去改 `compute_state_hash()`** → Mitigation：D3 明确「成文表述订正、非实现缺陷」、ADR 0002 §6 / `sim_match.rs:4328` 既有字段为正确实现、`git diff -- crates/w3-core/` 为空。
- **[Risk] 清单规模二选一口径未拍板就烘焙进 tasks** → Mitigation：D2 与 spec Req3 用「95 列或删除具体列数」并列、`decision-memo.md` 列入「待确认项」，tasks 写「按拍板结论」、不臆造为已定案。
- **[Risk] 规格门 `pending` 被绕过或填 `approved` / 指纹 / 省略说明** → Mitigation：D5 + `decision-memo.md`「## 规格门」只写 `status: pending`，由 leader 在用户拍板后回填；`spec_gate_check()` fail-closed 属预期。
- **[Trade-off] 只改文档 vs 顺带 bump / 补跑 determinism** → 取纯文档订正（用户拍板 p12 边界），代价是验收线不含内核测试，如实登记、不冒充全量门禁。

## Migration Plan

1. **本轮（成文、不实施）**：按 `openspec/decision-memo.template.md` 站稳承重约束与拍板决策后，一次成文四件套 + `decision-memo.md`；`openspec validate p12-ability-doc-reconciliation --strict` + `python tools/check_encoding.py`；`git diff -- openspec/specs/` 为空；规格门 `status: pending`。
2. **规格门**：`critic` 按清单第 7 / 10 / 12 / 13 项出规格门报告 → **用户拍板**（不拿沉默当授权）→ leader 回填 `status: approved` + 四件套指纹 + `approved_by` + `date` → 规格锁定。
3. **apply 阶段（后续独立会话）**：`spec-steward` 按 D1 改主 `openspec/specs/**` 的 THEN 字面（逐字保留身份键、不抹平 bump 8 史实）、按 D2 拍板结论订正清单规模、按 D3 订正冻结定义哈希契约表述；逐项跑 spec 各 Scenario 的可执行验证（全仓 Grep 残留为 0 / `openspec validate --all` / `python tools/verify_all.py` / `python tools/check_encoding.py`）；归档件零改动。
4. **回滚**：纯文档订正，回滚 = 还原对应 Markdown 字面；无跨版本迁移、无状态 / 快照兼容性问题（ADR 0002 §7 不适用）。
5. **待人工 / VM**：CI 双平台 `verify_all.py` 由人工 push 后触发（AI 不得 push / fetch / pull / clone），本轮不预先声称通过。

## Open Questions

Q1–Q3 等范围 / 口径未决项一律列入 `decision-memo.md`「待确认项」（清单规模 95 列 vs 删除数字、约 11 处主 spec 的逐处判定、规格门批准记录回填、apply 阶段口径选择），由规格门用户拍板，不在 tasks 里烘焙未拍板假设。本 change 无「可以安全延后而不改变 spec / 方案 / task 拆解」的 Open Question。
