# Tasks

> 本文件为 p12 的实施清单，**本轮只成文、不实施代码**，全部复选框保持 `- [ ]` 未勾，apply 是后续独立会话。本 change 为纯文档 / 规格口径订正（owner：`spec-steward`），零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产；`CODEBUDDY.md §5` 硬门禁与形态①反向验证**不适用**，故本清单**不含**重跑 `cargo test -p w3-core --test determinism` 的任务（不臆造内核改动）。`## 规格门` 保持 `status: pending`、批准记录由 leader 回填。

## 1. 口径订正 1 —— 主 spec 历史 `==8` 字面（apply 阶段）

- [ ] 1.1 全仓枚举遗留：对 `openspec/specs/**` Grep 无条件 `==8` /「保持 8」/「当前 8」/「`SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`」字面，逐处区分「当前版本号消费者」与「历史 bump 8 沿革史实」并列出清单（约 11 处，含 `ai-player-lockstep/spec.md:34`、`deterministic-simulation/spec.md:44,66`、`dual-faction/spec.md:82`、`undead-faction/spec.md:104`、`ability-system/spec.md:4,67` 等），不臆造精确条数。验证：枚举清单与本轮 Grep 输出一致、三版本号权威值核对为 `crates/w3-core/src/snapshot.rs:67-68` 与 `crates/w3-net/src/packet.rs:50` 的 9。
- [ ] 1.2 按 D1 订正 THEN：把 1.1 中判定为「当前版本号消费者」的无条件硬断言字面改为 9，逐字保留每个 `### Requirement:` 标题与其下各 `#### Scenario:` 标题（身份键）、口径只改在 THEN；历史沿革叙述里 P9 / P10 / P11 的「bump 8」实施期史实原样保留、不抹平。验证：`git diff -- openspec/specs/` 中无任一 Scenario / Requirement 标题被改写、仅 THEN 内字面变化。
- [ ] 1.3 不新增 MODIFIED delta 凑数：确认本 change 只含 `specs/doc-version-reconciliation/spec.md` 一份 ADDED delta，未为订正新增第 10 份 MODIFIED delta。验证：`ls openspec/changes/p12-ability-doc-reconciliation/specs/**/*.md` 只列出该一份、`openspec validate p12-ability-doc-reconciliation --strict` 通过。
- [ ] 1.4 残留清零验证：重跑全仓 Grep，确认「当前版本号消费者」侧旧版本号硬断言残留数为 0、三处权威定义 == 9 全仓一致、历史 bump 8 沿革叙述仍在。验证：Grep 输出为空（沿革段除外）+ `openspec validate --all` 通过（主 specs 不被 delta 覆盖、Purpose 保留）。

## 2. 口径订正 2 —— 清单规模 91 → 95 列（apply 阶段）

- [ ] 2.1 按 `decision-memo.md`「待确认项」的拍板结论，把后续主 spec / 文档登记的 `AbilityData.json` 列数订正为与实测一致的 **95 列**，或删除具体列数、统一为「粗统计、以人工核对为准」表述；其余规模元数据原样一致、只登记规模与分类、不登记任何具体技能名 / 四字符 rawcode / 数值。验证：全仓 Grep 无「91 列」作为既成事实残留，且口径符合 `WC3-MELEE-RULES.md §14` 与 ADR 0001 / ADR 0004。
- [ ] 2.2 归档件零改动：确认 P11 归档件四件套与 `decision-memo.md` 未被追溯改写（已 `status: approved` + 指纹锁定）。验证：`git diff -- openspec/changes/archive/2026-11-25-p11-ability-system/` 为空，`python tools/verify_all.py --fingerprint` 不因本 change 触发 P11 规格解锁。
- [ ] 2.3 零资产验证：确认本机导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库。验证：`python tools/verify_all.py` 资产扫描通过、`git ls-files` 无 `*.slk` / 导出 JSON / PNG / GLB / 音效（ADR 0001）。

## 3. 口径订正 3 —— 冻结定义哈希契约表述（apply 阶段）

- [ ] 3.1 按 D3 把「改任一冻结常量…不改运行时 `compute_state_hash()`」的歧义表述订正为准确口径：「会经既有 `config.content_hash` / `defs.content_hash()` 身份传导改变 `compute_state_hash()`；冻结定义未作为独立运行态字段二次混入」，并显式标注「成文表述订正、非实现缺陷」。验证：订正文字与 `crates/w3-core/src/sim_match.rs:4328`（`hasher.write_u64(self.config.content_hash)`）/ `:4375`（`hasher.write_u64(self.defs.content_hash())`）/ `docs/decisions/0002-determinism.md §6` 的 `content_hash` 既有参与字段一致。
- [ ] 3.2 实现与 ADR 正文零改动：确认 ADR 0002 §1–§8 正文、运行时哈希字段集合、P11 归档件 `tasks.md` 5.2 均未被改写。验证：`git diff -- docs/decisions/0002-determinism.md` 为空、`git diff -- crates/w3-core/` 为空、`git diff -- openspec/changes/archive/2026-11-25-p11-ability-system/tasks.md` 为空。

## 4. 收尾验证与边界自证（本轮成文 + apply 阶段）

- [ ] 4.1 零内核改动自证：确认本 change 不新增权威状态、不进状态哈希 / 快照 / `first_difference()`、不 bump 三版本号（权威值 9 不变）、`CODEBUDDY.md §5` 硬门禁与形态①反向验证不适用。验证：`git diff -- crates/w3-core/` 与 `git diff -- crates/w3-net/` 为空、三版本号常量仍 == 9；本清单无 `determinism` 重跑任务（不臆造内核改动）。
- [ ] 4.2 编码与四件套校验：跑 `python tools/check_encoding.py`（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格）与 `openspec validate p12-ability-doc-reconciliation --strict`。验证：两条命令输出成功、零 error（>500 字 requirement 类 WARNING 如实标注、不冒充无告警）。
- [ ] 4.3 决策纪要结构自证：确认 `decision-memo.md` 三必填小节（承重约束 / 拍板决策 / 待确认项）非空、「## 规则文档同步」小节按 `WC3-MELEE-RULES.md §14` 第 5 条给出如实结论、「## 规格门」只写 `status: pending`、无 `fingerprint` / `approved_by` / `date`、无「## 省略说明」。验证：`python -c "import sys;sys.path.insert(0,'tools');import verify_all as v;print(v.decision_memo_check())"` 输出 True。
- [ ] 4.4 fail-closed 信号如实登记：确认 `python tools/verify_all.py` 的 `spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期、`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免。验证：如实记录该输出，不绕过、不修复、不填 `approved` / 指纹、不冒充放行。
- [ ] 4.5 **规则文档同步核对**（按 `WC3-MELEE-RULES.md §14` 第 5 条如实给结论）：本 change 为纯文档 / 规格口径订正，不实现任何玩法、不触及任何 `R-x.y` 实现状态 / §13.7 现状 / 附录「规则 → 实现覆盖对照」/ §15 变更记录 / 参考项目数量，结论为**无需同步**并写明依据；MUST NOT 把「引用某编号」冒充「规则已实现」。验证：「## 规则文档同步」小节非空且结论真实，`git diff -- docs/references/WC3-MELEE-RULES.md` 为空。
- [ ] 4.6 git 边界自证：确认改动仅落在 `openspec/changes/p12-ability-doc-reconciliation/` 与 `openspec/changes/p13-ability-full-roster/`（后者由同一轮成文），未改 `openspec/specs/**` / `openspec/changes/archive/**` / `docs/**` / `crates/**` / `tools/**` / 根配置 / `CODEBUDDY.md`；不 `git add` / `commit` / `push` / `fetch` / `pull` / `clone` / `archive`。验证：`git status --short` 仅显示本轮成文范围内的 change 目录为未跟踪。

## Workflow follow-up

- 规格门：`critic` 按清单第 7 / 10 / 12 / 13 项出报告 → 用户拍板（不拿沉默当授权）→ leader 回填 `status: approved` + 四件套指纹 + `approved_by` + `date`；本轮 `status: pending`、AI 未 self-approve。
- apply 由 `spec-steward` 在后续独立会话实施（改主 spec THEN 口径 / 清单规模 / 哈希契约表述），本轮不执行任一任务。
- 人工 push / CI 双平台 `verify_all.py` / `/opsx:archive` 由人工与 leader 处理；AI 不得 push / fetch / pull / clone、不得 archive、不得本地 commit（本轮统一由 leader 处理）。
