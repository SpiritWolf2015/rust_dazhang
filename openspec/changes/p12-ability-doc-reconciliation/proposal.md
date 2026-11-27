# Proposal

> 本轮只成文、不实施代码（apply 是后续独立会话）。本 change 为承接已归档 `2026-11-25-p11-ability-system` 决策纪要「## 已知限制」的**纯文档 / 规格口径订正**独立 change，用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」，本 change 为文档侧，玩法侧见 `p13-ability-full-roster`。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。

## Why

P11 归档件「## 已知限制」登记了三项**成文口径偏差**（非实现缺陷），按 P9 Q9' 已拍板「归档后由独立 change 收口」一直挂着：① 多份主 spec 的 THEN / 正文仍无条件写 `==8` /「保持 8」/「当前 8」/「`SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`」，而三版本号权威值当前均为 **9**；② 「清单规模事实锚」登记 `AbilityData.json` 803 条 / 91 列，content-owner 本机导出实测为 803 条 / 95 列；③ `tasks.md` 5.2 字面「不改运行时 `compute_state_hash()`」与 P5 以来 ADR 0002 既有契约（`config.content_hash` 作为既有字段写入）冲突。三者若长期不收口，会令 spec 读者与 reviewer 得到错误的版本号 / 规模 / 哈希契约口径，故单列一个零代码改动的文档 change 收口。

## What Changes

- 新增 capability `doc-version-reconciliation`：用 SHALL/MUST 固化三项成文口径契约（主 spec 版本号消费者与三版本号权威值一致 / 清单规模元数据口径如实 / 冻结定义哈希契约表述准确），每条配可执行验证 Scenario。
- **口径订正 1（主 spec 历史 `==8` 字面）**：apply 阶段由 `spec-steward` 在主 `openspec/specs/**` 的 THEN / 正文把约 11 处无条件 `==8` /「保持 8」/「当前 8」/ 三版本号 = 8 字面订正为权威值 9；**逐字保留历史 requirement 与 Scenario 身份键、口径只改在 THEN**；**不新增第 10 份 MODIFIED delta 凑数**；**不抹平历史沿革叙述里的 bump 8 史实**（P9 / P10 / P11 各 change 实施期口径属既成史实）。本轮成文阶段 `git diff -- openspec/specs/` 为空（主 spec 在 archive 合并时复核、或本 change 的 apply 阶段改；proposal 只描述「要做什么」）。
- **口径订正 2（清单规模 91 → 95 列）**：把后续主 spec / 文档登记的 `AbilityData.json` 列数订正为 95（或删除具体列数、按「粗统计、以人工核对为准」口径统一）；**只做文档订正、不改四件套已锁定的历史 artifact**（P11 归档件已 `status: approved` + 指纹锁定，订正落在后续主 spec / 文档、不追溯改写归档件）。
- **口径订正 3（tasks 5.2 表述歧义）**：把「改任一冻结常量…不改运行时 `compute_state_hash()`」订正为「会经既有 `config.content_hash` / `defs.content_hash()` 身份传导改变 `compute_state_hash()`；冻结定义未作为独立运行态字段二次混入」；属**成文表述订正、非实现缺陷**，不改 ADR 0002 §1–§8 正文、不改归档件四件套。
- **明确不纳入**（分属 p13 / 人工 / VM / 历史事实）：顺延项、Q1 逐项 SLK 对齐、真窗口 8.x 人工清单、golden hash 双平台、CI 双平台、参考缺口、`vera20k` HEAD 前进、Q-A / 形态①时序澄清。

## Capabilities

### New Capabilities
- `doc-version-reconciliation`: 主 spec 版本号消费者与三版本号权威值一致的成文契约、本机导出清单规模元数据口径如实登记契约、冻结定义哈希契约表述准确契约，以及各自的 Grep / `openspec validate --all` / `python tools/verify_all.py` 可执行验证口径。

### Modified Capabilities
<!-- 无。本 change 只新增一个 capability；主 spec 历史 `==8` 字面的订正落在 apply 阶段由 `spec-steward` 直接改 THEN（`openspec/specs/**` 是 source of truth）或归档合并时复核，不在提案阶段新增 MODIFIED delta、不新增第 10 份凑数；本轮成文 `git diff -- openspec/specs/` 为空。依据 P9 Q9' 已拍板「归档后由独立 change 收口」与 P11 `design.md` D0「主 spec 在 archive 阶段合并、提案阶段不直接编辑」。 -->

## Impact

- **路线图阶段**：P12（承接 P11 已归档的文档订正独立 change）；P0–P5 及 P6–P11 均已实施并归档，见 `CODEBUDDY.md §1`。用户 2026-11-25 拍板「建两个 change（文档 + 玩法各一）」，本 change 为文档侧、非玩法 change，不实现任何 `R-x.y`、不动 1.0 范围 S1–S6。
- **ADR**：0002（确定性契约：`config.content_hash` 既有字段口径，仅订正表述、不改 §1–§8 正文与运行时哈希字段集合）、0001（资产 / 法律：清单规模元数据口径不涉及任何具体技能名 / rawcode / 数值入库）、0004（冻结定义：`compute_defs_hash` → `defs_hash` / `content_hash` 快照头部的定义层身份，仅订正表述）、0005（三版本号含 `PROTOCOL_VERSION`，仅订正字面、不改 proto schema）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）；本 change 不新增 / 不改任何 ADR。
- **确定性契约**：**零 `w3-core` 权威状态改动、零确定性影响**；不新增权威状态字段、不进状态哈希 / 快照 / `first_difference()`；不 bump `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`（当前权威值均为 9，本 change 不改其值）；`git diff -- crates/w3-core/` 为空，`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用（本 change 不触确定性代码、不重跑 `cargo test -p w3-core --test determinism`）。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效；本机导出表规模元数据只登记规模与分类、不登记任何具体技能名 / 四字符 rawcode / 数值；导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库（ADR 0001）。
- **代码 / 系统**：本轮只成文，改动仅落在 `openspec/changes/p12-ability-doc-reconciliation/`；apply 阶段由 `spec-steward` 改主 `openspec/specs/**` 的 THEN 字面 + 后续主 spec / 文档的清单规模与哈希契约表述（不改归档件、不改 `crates/**`、不改 `docs/**`、不改 `tools/**`、不改根配置、不改 `CODEBUDDY.md`）。
- **依赖**：不新增第三方依赖；`Cargo.lock` 不变。
