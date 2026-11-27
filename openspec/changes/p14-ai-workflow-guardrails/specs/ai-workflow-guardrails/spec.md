# Spec Delta

## Purpose

本 capability 定义跨会话 AI 协作所需的工程治理契约：以只读、可机械校验的代码膨胀预算门禁抑制上下文膨胀，以临时约束的时效 / 作用域规范防止当期禁令被后续会话误当永久门禁。两条契约均为规则文字与 `tools/*.py` 门禁脚本，零新增权威状态、零确定性影响、不 bump 三版本号、零资产、不新增第三方依赖（ADR 0001 / ADR 0002 / ADR 0003）；不实现任何玩法、不碰 1.0 范围 S1–S6。

## ADDED Requirements

### Requirement: 代码膨胀预算门禁

系统 SHALL 提供跨平台纯 Python 门禁脚本 `tools/check_code_budget.py`，只读统计 `crates/**/*.rs`（及可选 `tools/*.py`）的总行数、单文件行数、函数体行数、模块文件数；超阈值时 MUST 以英文输出报错、以非零退出码失败，并 SHALL 可挂进 `tools/verify_all.py` 统一门禁。阈值具体数值 MUST 由人在基线测量后拍板，MUST NOT 臆造任何外部项目规模数字。脚本 MUST 用 `os.name` 判平台差异、注释中文 / 输出英文、UTF-8 无 BOM / LF，且不写任何游戏资产或本机绝对路径（ADR 0001 / ADR 0002）。

#### Scenario: 基线测量先于阈值、不臆造外部数字

- **WHEN** apply 阶段先跑只读基线测量，产出当前 `crates/` 规模分布（总行数 / 单文件行数 / 函数体行数 / 模块文件数）
- **THEN** 分布结果作为阈值拍板依据写入 `decision-memo.md`「待确认项」；阈值数值未经人拍板前脚本 MUST NOT 冻结任何外部项目数字；`git diff -- openspec/specs/` 与 `git diff -- crates/w3-core/` 均为空（基线测量只读、不写权威状态）

#### Scenario: 超阈值显式失败（门禁脚本改坏验红自证）

- **WHEN** 在 `crates/**` 下故意造一个超阈值的临时 `.rs` 文件后运行 `python tools/check_code_budget.py`
- **THEN** 脚本以非零退出码退出、输出英文报错（含超标维度与实测值）；删除该临时文件后复跑 MUST 以零退出码通过；该自证是**门禁脚本自身的改坏验红**，不是 `CODEBUDDY.md §5` 针对 `w3-core` 权威状态回归测试的形态①反向验证，两者 MUST NOT 混淆；`git status --short` 自证临时文件已删除、无残留

#### Scenario: 门禁零确定性与编码合规

- **WHEN** 跑 `python tools/check_encoding.py`、`cargo fmt --all -- --check` 与 `python tools/verify_all.py`（若拍板挂载）
- **THEN** 新脚本 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格、注释中文 / 输出英文；`crates/w3-core/` 差异为空、`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 仍 == 9、不进状态哈希 / 快照 / `first_difference()`；无需重跑 `cargo test -p w3-core --test determinism`（不触确定性代码，ADR 0002）；`openspec validate p14-ai-workflow-guardrails --strict` 通过

### Requirement: 临时约束时效与作用域规范

`openspec/config.yaml` 的 `rules.*`、`CODEBUDDY.md`（§8 附近）、change 四件套与 `.codebuddy/agents/critic.md` 中凡出现 `MUST NOT` /「本期不做」/「暂不」/「当期 change 内约束」类文字时，系统 SHALL 要求其标注作用范围（哪个 change / 模块）与失效条件，并 MUST NOT 把无作用域标注的当期约束写成永久门禁。`spec-steward` 归档合并时 SHALL 复核该约束是否已被后续 change 取代；`critic` 清单第 7 / 10 项 SHALL 加对应复核点。规则文字为纯文档，不进状态哈希 / 快照、不改 `w3-core`（ADR 0002）。

#### Scenario: 临时类约束带作用域与失效条件

- **WHEN** apply 阶段在 `openspec/config.yaml` 的 `rules.*` 与 `CODEBUDDY.md` §8 增补时效惯例，并全仓 Grep `MUST NOT` /「本期不做」/「暂不」/「当期」类文字
- **THEN** 新增的临时类约束逐条带「作用范围（change / 模块）+ 失效条件」标注；以 P11 返工为反例锚：`crates/w3-client/src/melee/defs.rs:22/919/2290-2292` 与 `openspec/changes/archive/2026-11-25-p11-ability-system/design.md:11/169-173/236` 已澄清 P8.1 当期约束「被取代」；`openspec validate p14-ai-workflow-guardrails --strict` 通过

#### Scenario: 归档合并复核取代与 critic 复核点

- **WHEN** `spec-steward` 归档合并一个含当期约束的 change，或 `critic` 按清单第 7 / 10 项审规格
- **THEN** 复核结论写明「该约束是否被后续 change 取代」并留证据锚；`critic` 第 7 项（ADR 与 spec 一致性、spec 引用 ADR 编号）/ 第 10 项（四件套一致性、承重约束锚打开核验）带时效复核点；被取代的约束 MUST NOT 继续当永久门禁；规则文字改动不进状态哈希 / 快照、`git diff -- crates/w3-core/` 为空（ADR 0002）

#### Scenario: 规则文字边界与非玩法 change 自证

- **WHEN** 收尾核对本 change 的 `design.md` 与 `decision-memo.md`
- **THEN** `design.md`「## 参考项目对照」如实登记 `UPSTREAM.md` 矩阵 T1–T7 均不命中；proposal / specs / tasks 不引用任何 `R-x.y`、不把规则编号引用冒充已实现；`decision-memo.md`「## 规则文档同步」按 `WC3-MELEE-RULES.md §14` 第 5 条给出「无需同步」结论并写明依据（不触及任何 `R-x.y` 实现状态 / §13.7 现状 / 附录覆盖对照 / §15 变更记录 / 参考项目数量）；`git diff -- docs/references/WC3-MELEE-RULES.md` 为空
