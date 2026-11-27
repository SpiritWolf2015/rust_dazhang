# Tasks

> 本文件为 p14 的实施清单，**本轮只成文、不实施代码**，全部复选框保持 `- [ ]` 未勾，apply 是后续独立会话。本 change 为非玩法 / 工作流治理 change（owner 待 leader 拍板，一个 change 一个 owner），零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不新增依赖；`CODEBUDDY.md §5` 硬门禁与形态①反向验证**不适用**，故本清单**不含** `cargo test -p w3-core --test determinism` 任务（不臆造内核改动），但含 `cargo test --workspace` / `python tools/verify_all.py` 保证整体不回归。**门禁脚本「改坏验红」是门禁自证，不是 §5 形态①**（D4 / D7）。`## 规格门` 保持 `status: pending`、批准记录由人拍板后回填。

## 1. 基线测量（只读，阈值拍板前置）

- [ ] 1.1 对 `crates/**/*.rs`（及可选 `tools/*.py`）做只读基线测量，产出当前规模分布：总行数 / 单文件行数 / 函数体行数 / 模块文件数，函数体行数用轻量启发式并在报告中声明「粗统计」口径。验证：测量报告四项数字可由 `wc -l` 类命令复核、全程只读（`git status --short` 无新增文件）、**不臆造任何外部项目规模数字**。
- [ ] 1.2 把基线分布登记进 `decision-memo.md`「待确认项」，作为阈值具体数值的拍板依据；阈值数值未经人拍板前**不冻结任何数字到脚本 / 权威文件**。验证：`decision-memo.md`「待确认项」含四条阈值项且标注「待规格门拍板」；`git diff -- openspec/config.yaml` / `git diff -- CODEBUDDY.md` 在成文期为空。

## 2. 护栏 1：代码膨胀预算门禁脚本与挂载（apply 阶段）

- [ ] 2.1 按 `decision-memo.md`「待确认项」拍板结论（硬阈值门禁 vs 仅文档约定二选一）新增 `tools/check_code_budget.py`：纯 Python 标准库、跨平台、平台差异用 `os.name` 判断、注释中文 / 输出英文、UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；只读统计四类规模指标、超阈值英文报错 + 非零退出码显式失败；不写中间产物、不写游戏资产 / 本机绝对路径；不新增第三方依赖。验证：`python tools/check_code_budget.py` 在正常态以零退出码通过并输出英文统计摘要；`python tools/check_encoding.py` 覆盖该新文件通过；`critic.md` 清单第 8 项（工具脚本跨平台）口径一致，无 `.ps1` / `.sh` / `.bat` / `.cmd`。
- [ ] 2.2 按拍板结论决定是否把 `tools/check_code_budget.py` 以一行 `run(...)` 挂进 `tools/verify_all.py` 的 `main()`（`:318-348`）；若拍板「仅文档约定」则不挂载、只保留脚本可独立运行，并在 `decision-memo.md` 记录结论。验证：`git diff -- tools/verify_all.py` 与拍板结论一致（最多一行挂载），`python tools/verify_all.py` 其余各步不被该改动破坏；不臆造「已挂载 / 已进 CI」。
- [ ] 2.3 按拍板结论决定是否纳入 CI 双平台（`.github/workflows/ci.yml` 的 `windows-latest` / `ubuntu-latest` matrix）；若拍板不纳入则不在本 change 改 CI 配置并如实登记。验证：`git diff -- .github/` 与拍板结论一致；`decision-memo.md`「待确认项」该项已结案或显式豁免；CI 由人工 push 后触发、本轮不预先声称通过。

## 3. 护栏 2：临时约束时效 / 作用域规范（apply 阶段，规则文字）

- [ ] 3.1 在 `openspec/config.yaml` 的 `rules.proposal/specs/design/tasks`（`:50-72`）增补惯例：凡写 `MUST NOT` /「本期不做」/「暂不」/「当期 change 内约束」类文字 MUST 标注作用范围（哪个 change / 模块）与失效条件；**永久硬红线（ADR 0002 / `CODEBUDDY.md §5`）MUST NOT 被误标为临时约束**。验证：新增文字每条带「作用域 + 失效条件」；全仓 Grep 该类新增文字无一条缺标注；`openspec validate p14-ai-workflow-guardrails --strict` 通过。
- [ ] 3.2 在 `CODEBUDDY.md` §8 附近增补同一惯例（写目标不写手段、边界说清、临时约束标注时间 / 范围），并写明 `spec-steward` 归档合并时 MUST 复核该约束是否已被后续 change 取代。验证：`git diff -- CODEBUDDY.md` 仅落在 §8 附近、不改动 §2 / §5 / §6 硬基线；文字与 `config.yaml` 3.1 口径一致、不另立冲突规则。
- [ ] 3.3 在 `.codebuddy/agents/critic.md` 清单第 7 项（`:25`，规范一致性 / spec 引用 ADR 编号）/ 第 10 项（`:28`，四件套一致性 / 承重约束锚打开核验）加「临时约束是否已标注作用域与失效条件、是否被后续 change 取代」复核点。验证：`git diff -- .codebuddy/agents/critic.md` 仅追加复核点、不改既有清单编号与语义；复核点覆盖 P11 反例（`crates/w3-client/src/melee/defs.rs:22/919/2290-2292` 已澄清 P8.1 当期约束「被取代」）。
- [ ] 3.4 以 P11 返工为反例做一次只读复核演练：核对 `openspec/changes/archive/2026-11-25-p11-ability-system/design.md:11/169-173/236` 与 `defs.rs` 过期注释，确认新惯例能识别「当期约束被取代」场景。验证：演练结论写进 `decision-memo.md`（含证据锚）；归档件 / `defs.rs` 零改动（`git diff` 为空，不追溯改写历史）。

## 4. 门禁自证与全绿验证（apply 阶段）

- [ ] 4.1 **门禁脚本改坏验红自证**：在 `crates/**` 下故意造一个超阈值的临时 `.rs` 文件 → 跑 `python tools/check_code_budget.py` 观察英文报错 + 非零退出码 → 删除该临时文件 → 复跑以零退出码通过。验证：三段输出如实记录（缺陷态报红 / 修复态通过）、`git status --short` 自证临时文件已删除且无残留 / 无资产入库；**措辞 MUST 明确这是「门禁脚本自身的改坏验红自证」，不是 `CODEBUDDY.md §5` 针对 `w3-core` 权威状态回归测试的形态①反向验证，两者不混淆、不据此改内核或重跑 determinism**。
- [ ] 4.2 跑 `python tools/check_encoding.py`、`python tools/check_source_conventions.py`、`cargo fmt --all -- --check`、`cargo test --workspace`。验证：四条命令全部成功；新脚本 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格、注释中文 / 输出英文；`cargo test --workspace` 保证整体不回归（**不重跑** `cargo test -p w3-core --test determinism`，零内核改动、不臆造）。
- [ ] 4.3 跑 `python tools/verify_all.py` 与 `openspec validate p14-ai-workflow-guardrails --strict`。验证：`openspec validate --strict` 零 error（>500 字 requirement 类 WARNING 如有则如实标注、不冒充无告警）；`decision_memo_check()` 为 True；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属**预期 fail-closed 信号**，如实登记、不绕过、不填 `approved` / 指纹、不冒充「统一门禁全绿」；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免。
- [ ] 4.4 脚本跨平台自证：在 Windows / Linux 两种 `os.name` 分支下统计同一份 `crates/**/*.rs`，四类指标结果一致。验证：两平台输出摘要逐项相等；脚本内用 `os.name` 判断、不维护两套脚本、不写 `.ps1` / `.sh` / `.bat` / `.cmd`（`CODEBUDDY.md §2` / §9、`critic.md` 第 8 项）；本条如无双平台环境则如实标「待 VM / 人工」、不冒充已验证。

## 5. 边界自证、规则文档同步核对与 git 边界（成文期 + apply 阶段）

- [ ] 5.1 零内核 / 零确定性边界自证：`git diff -- crates/w3-core/` 为空、`git diff -- crates/w3-net/` 为空；`SNAPSHOT_VERSION` / `SIM_VERSION`（`crates/w3-core/src/snapshot.rs:67-68`）与 `PROTOCOL_VERSION`（`crates/w3-net/src/packet.rs:50`）仍 == 9；不进状态哈希 / 快照 / `first_difference()`、不改 `step()` / 命令排序。验证：三条 `git diff` 输出为空、三版本号常量仍 == 9；本清单无 `determinism` 重跑任务（不臆造内核改动，D7）。
- [ ] 5.2 零资产 / 零依赖边界自证：`git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 导出 JSON / PNG / GLB / 音效；`python tools/verify_all.py` 资产扫描通过；`Cargo.lock` 不变、`NOTICE.md` 无需更新；外部技术栈与外部项目规模数字未进仓库。验证：资产扫描输出 OK、`git diff -- Cargo.lock` / `git diff -- NOTICE.md` 为空、全仓 Grep 无外部项目规模数字作为本项目事实（ADR 0001）。
- [ ] 5.3 **规则文档同步核对**（按 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条如实给结论）：本 change 为非玩法 / 工作流治理 change，不实现任何玩法、不触及玩法规则实现状态 / §13.7 现状 / 附录「规则 → 实现覆盖对照」/ §15 变更记录 / 参考项目数量（仍五个），结论为**无需同步**并写明依据；MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」。验证：`decision-memo.md`「## 规则文档同步」小节非空且结论真实、`git diff -- docs/references/WC3-MELEE-RULES.md` 为空；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免。
- [ ] 5.4 决策纪要 / 规格门结构自证：确认 `decision-memo.md` 三必填小节（承重约束 / 拍板决策 / 待确认项）非空、每条承重约束带 `file:line` / commit / 命令输出证据锚、「## 规格门」**仅** `status: pending`、无 `fingerprint` / `approved_by` / `date`、无「## 省略说明」、不 self-approve。验证：`python -c "import sys;sys.path.insert(0,'tools');import verify_all as v;print(v.decision_memo_check())"` 输出 True；`git diff -- docs/decisions/` 为空（未改 ADR 正文）。
- [ ] 5.5 git 边界自证：确认改动仅落在 `openspec/changes/p14-ai-workflow-guardrails/`，未改 `openspec/specs/**` / `openspec/changes/archive/**` / p12 / p13 / `docs/**` / `crates/**` / 根配置；不 `git add` / `commit` / `push` / `fetch` / `pull` / `clone` / `archive`。验证：`git status --short` 仅显示 p14 新目录为未跟踪（p12 / p13 已有 `??` 保持不变）；`git diff -- openspec/specs/` / `git diff -- docs/` 为空。

## Workflow follow-up

- 规格门：`critic` 按清单第 7 / 10 / 12 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ leader / `spec-steward` 回填 `status: approved` + 四件套指纹（`python tools/verify_all.py --fingerprint p14-ai-workflow-guardrails`）+ `approved_by` + `date`；本轮 `status: pending`、AI 未 self-approve、未填指纹。
- 参考矩阵命中判定（T1–T7 均不命中、本轮未派 `reference-researcher`）待 `critic` 清单第 12 项复核；若改判命中则补对照报告并回到用户拍板，不臆造。
- apply 在后续独立会话实施（一个 change 一个 owner、不同时改多 owner 文件），本轮不执行任一复选框、不勾任一任务。
- 人工 push / CI 双平台 `verify_all.py` / `/opsx:archive` 由人工与 leader 处理；AI 不得 push / fetch / pull / clone、不得 archive、不得本地 commit（本轮统一由 leader 处理）。
