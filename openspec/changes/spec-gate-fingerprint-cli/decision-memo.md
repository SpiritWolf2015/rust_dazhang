# 决策纪要 — `spec-gate-fingerprint-cli`

> 取料纪律：基于仓库 artifacts 回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002）。
> 本 change 是工作流闭环的首次从零演练载体；规格门已于 2026-11-14 由用户拍板批准，演练产物未 commit 前仍可整体丢弃。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `crates/w3-core/` 零改动；不新增权威状态字段，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合与快照头部布局不变 | `CODEBUDDY.md:47-53`（§5）/ `openspec/config.yaml:9-16` / `proposal.md` Impact |
| C2 | 仓库零资产、无新增第三方依赖；子命令只依赖 Python 标准库 | `CODEBUDDY.md:55-60`（§6）/ `tools/verify_all.py:8-15`（import 全标准库） |
| C3 | 规格门批准**由人拍板**，脚本不得自批、不得写回 `decision-memo.md` | `CODEBUDDY.md:102`（§9 串联）/ `design.md` Non-Goals |
| C4 | 子命令 MUST 复用 `spec_artifact_fingerprint()` 原字节口径，不得重构，否则「批准值」与「校验值」漂移 | `tools/verify_all.py:158`（指纹函数）/ `tools/verify_all.py:190`（`spec_gate_check()` 复用） |
| C5 | 全量门禁入口 `main()` 行为不变，子命令严格隔离、不写文件 | `tools/verify_all.py:260`（`main()`）/ `tools/verify_all.py:280`（`__main__` 分发）/ `specs/verification-cli/spec.md` Scenario「全量门禁行为不变」 |
| C6 | 新增回归 MUST 反向验证（改坏验红），仅「全绿」不构成验证；**本 change 属演练性主动加严**，`CODEBUDDY.md` §5 的强制对象是 `w3-core` 确定性回归，本次不计作该规则已跑通 | `CODEBUDDY.md:51`（§5）/ `openspec/config.yaml:61`（`rules.tasks` 第三条） |
| C7 | `critic` 代码门按全清单审**本 change 全部 diff**，含第 10 项证据锚与第 11 项整体调用路径 | `.codebuddy/agents/critic.md:28-29` / `tasks.md` 3.4 |
| C8 | 工具脚本跨平台（Python，平台差异在脚本内判断）；日志 / 错误英文、注释与文档中文 | `CODEBUDDY.md` §2 / §3 / `tools/verify_all.py:60-68`（`openspec_command()` 内 `os.name` 分支） |
| C9 | AI 不执行 `push` / `fetch` / `pull` / `clone`，归档与远端同步由人工 / 编排者执行 | `CODEBUDDY.md:32`（§4 权限矩阵） |

## 拍板决策（含被否备选与理由）

- **P1 用户已明确授权「从零演练完整闭环」**：用户在本会话上一轮选项列表中选择「1」（用下一个新 change 从零演练完整闭环）。证据形态：本会话用户消息（**非仓库文件**，依模板三分判据第 3 类「只在人脑子里的拍板」；仓库内无可核验 `file:line` 锚，**由人工核对**）。该授权针对**演练行为**，不是对本 change 技术方案的批准。
- **P2 已由用户拍板批准**：演练载体 = 给 `verify_all.py` 加 `--fingerprint <change>` 只读子命令。依据 `8093cd9` 引入规格门机械校验后、在**会话交付说明**中暴露的「批准时取指纹需绕 `python -B -c`」缺口（该命令字面**不在 commit message 内，非可打开锚**）；纯工具脚本、零资产、可完整演练反向验证。用户于 2026-11-14 选「1 批准并修正」明确批准。被否备选见 R1–R4。
- **P3 已由用户拍板批准**：技术方案 D1–D3（最小 `sys.argv` 扩展单入口 / 未知输入 fail-loud / 测试内反例元断言）。锚 `design.md` Decisions；同 P2 由用户选项 1 批准。

## 被否方向台账

- **R1** 新增 `tools/fingerprint.py` 独立脚本 → 否决：同一算法两份入口，易与 `spec_gate_check()` 口径漂移（违反单一真值源）。锚 `design.md` D1 备选 1。
- **R2** 保持 `python -B -c` 临时命令、只写进文档 → 否决：未受版本控制、跨平台写法不一、无法被 `critic` 审查。锚 `design.md` D1 备选 2。
- **R3** 引入 `argparse` 子命令框架 → 否决：当前仅一个开关，属过度工程。锚 `design.md` D1 备选 3。
- **R4** 未知 change 静默输出空指纹并以 0 退出 / 自动补齐四件套 → 否决：静默降级 + 脚本越权写 artifact。锚 `design.md` D2 备选 1、2。
- `design.md` D3 的两条备选（只写正向断言、人工改坏代码）属测试实现细节级，按 `openspec/decision-memo.template.md:11`「不写实现细节、不复述 `design.md`」不纳入方向台账，避免被误判为漏记。

## 规格评审报告与 residual 处置（2026-11-14）

`critic` 规格门报告：**无 blocker，8 条 residual**。用户拍板选项 1「批准并修正」后的处置如实登记：

- **R1 错锚（最重要的演练发现）**：`critic` 逐条实际打开证据锚，抓出 C3 `CODEBUDDY.md:101` → 实为 `:102`、C6 `config.yaml:60` → 实为 `:61`、C8 `verify_all.py:56-63` → `os.name` 分支实在 `:60-68`；另 C2 范围 `8-13`→`8-15`、`54-59`→`55-60`。已全部修正，并把 `8093cd9` 的证据形态改注为「会话交付说明、非 commit 可打开内容」。
- **R2 四件套定义不齐**：已在 `spec.md` Req2 / 两个 Scenario、`design.md` D2、`tasks.md` 1.2 统一——完整性以 `proposal.md` / `design.md` / `tasks.md` 为准，`specs/` 为空沿用 `spec_artifact_fingerprint()` 既定空字节口径，无 `.openspec.yaml` 归 `problem=unknown change`。
- **R3 显式验证点**：已在 `tasks.md` 2.5 补「CLI stdout 与 `spec_artifact_fingerprint()` 返回值逐字节相等、退出 0」断言，承接 `spec.md` Scenario「对 active change 取指纹」。
- **R4 / R5 / R6**：R4 见台账末句；R5 已把 `proposal.md` 的 `rules.proposal` 出处更正为 `openspec/config.yaml:48`，P7 与 ADR 0003 分期的措辞滞后登记为演练发现、不改规则文件；R6 已在 C6 注明「主动加严、不计作 §5 跑通」、P1 注明「仓库内无可核验锚、由人工核对」。
- **R7 pending 窗口**：`critic` 实测 `main()` 用 `&=` 不短路，7 类检查全部执行并输出，pending 红不掩盖其他结果；但批准前任何人跑全量门禁恒 FAILED，已列入待确认项作为演练后独立议题。
- **R8 `CODEBUDDY.md:8` §1 速览陈旧**（仍写 2 个 active change，实测为 `melee-playable-loop` 与本 change）：登记为演练发现，交 `docs-owner` 后续同步，不纳入本 change。

**修正窗口**：R1–R3 / R5 涉及四件套的修订全部在回填 `fingerprint:` **之前**完成（四件套在指纹范围内，回填后再改会失配并按 §9 重走规格门）；R1 中仅改 `decision-memo.md` 的项与 R4/R6/R7/R8 不在指纹范围内。`critic` 结论：8 个工作流提交的规则能跑通、未发现互斥打架，不触发 `design.md` 所设「回规则 artifact 修订 + 重走规格门」分支。

## 待确认项（如实标注，不冒充已拍板）

- ~~**规格门是否批准**~~ → 已于 2026-11-14 由用户拍板批准（选项 1「批准并修正」）。
- ~~**四件套指纹回填值**~~ → 已在四件套修订完成后用 `spec_artifact_fingerprint()` 计算并回填（`e5ad65e6…`）。
- **`verify_all.py` 对 `status: pending` 报 `invalid spec gate status`**：语义不够明确（应报「尚未批准」）。是否给 `spec_gate_check()` 增加 `pending` 分支（仍 fail-closed、非零退出，**不得据此让门禁变绿**），作为演练后独立议题，本轮不擅自改脚本。
- **pending 窗口全仓门禁恒红**：批准前其他 change 需先完成本 change 批准；`&=` 不短路已实测不掩盖其他检查。
- **演练结论（完整闭环）**：`critic` 规格门已实测 8 条规则能跑通、无互斥打架；apply / 反向验证 / 代码门 / 门禁 是否跑通待 `tasks.md` 3.5 填写，未跑通的环节 MUST NOT 标为通过。
- **P7 阶段与 ADR 0003 分期措辞滞后 / `CODEBUDDY.md:8` §1 速览陈旧**：交 `spec-steward` / `docs-owner` 后续同步，不纳入本 change 范围。
- **子命令是否需 `--help` / 组合参数**：记为后续，当前一个开关不预建框架。
- **归档与人工 push**：AI 不碰远端（`CODEBUDDY.md` §4）。

## 代码门复审与完整闭环演练结论（2026-11-14）

`critic` 代码门报告：**1 个 blocker（B1）+ 6 条 residual**。

- **B1 已修**：C5 承重约束锚 `tools/verify_all.py:239-256` 在 apply 后漂移（该处现为 `fingerprint_cli()`，`main()` 移至 `:260`、`__main__` 分发在 `:280`）。已按实际行号更正。只改 `decision-memo.md`（不在指纹范围内），不解锁规格门、不重走、不动代码。
- **R1 四件套内同类锚漂移**：`proposal.md` / `spec.md` / `design.md` / `tasks.md` 内引用的 `:158` / `:190` / `:239-256` 在 apply 后同样可能失准；但四件套在指纹范围内，改则失配并按 §9 重走规格门，故**本轮不改**，登记为演练发现。结构性问题：`file:line` 锚随实现代码漂移，后续可考虑改用函数名 / 稳定标识。
- **R2 `memo_section_body()` 前缀匹配**：本纪要曾因「## 规格门审查与 residual 处置」标题先命中前缀，导致 `spec_gate_check()` 读不到 `status`；已改名规避。前缀匹配本身仍有同类风险，作为演练后独立议题，**不为让门禁变绿而改门禁逻辑**。
- **R3 新增 Python 单测未挂进 `verify_all.py` 的 `main()`**：受 C5「全量门禁行为不变」约束，本轮只在本地跑 `python -m unittest discover -s tools -p 'test_*.py'`（8 passed）留证据；是否挂进统一门禁作为后续独立 change。
- **R4 两处 Scenario 缺独立自动化断言**：「不写文件」「与规格门口径一致」由 3.3 / CLI 手动对比覆盖，无独立用例；如实登记。
- **R5 docstring 入口未带 `-B`**：会生成 `__pycache__`，已由主会话清理；`.gitignore` 未忽略 `__pycache__` 属仓库既有问题，交后续处理。
- **R6 已登记遗留**：P7 阶段与 ADR 0003 分期措辞滞后、`CODEBUDDY.md:8` §1 速览陈旧，见待确认项。

**完整闭环演练结论**：决策纪要 → 规格门（`critic` 报告 + 人拍板 + 指纹回填 `e5ad65e6…`） → apply（`--fingerprint` CLI + 8 个单测含反向验证，真实 CLI 输出与回填指纹逐字节一致） → 代码门（全清单复审，B1 已修） → 门禁（`Verification OK`） **全程跑通**；8 个工作流提交（`93cf95c`…`b3b7b64`）的规则**未发现互斥打架**。演练中真实抓到并修正：3 处证据锚错锚、`status` 前缀匹配撞车、apply 后 C5 锚漂移（B1）、`spec.md` Req2 正文超长触发 `--strict` WARNING、spec 与 design 对「四件套完整性」定义不齐、Scenario 1 缺显式验证点。`tasks.md` 全部勾选后，因复选框归一化，指纹与 `status: approved` 均保持不变（已由 `spec_gate_check()` 复核）。未冒充通过的环节均在待确认项如实登记。

## 停止条件（追加阶段 / 归档前）

1. C1–C9 逐条仍有证据锚、无一条被本轮改动证伪。
2. 规格门由人拍板、`fingerprint:` 回填、`status: approved`，且 `verify_all.py` 的 `spec_gate_check()` 不报失配。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛。

## 规格门

- status: approved
- fingerprint: e5ad65e66fe95a4515518730c33ef43770ff88169d339d49575f63cf6001fd6b
- approved_by: 用户（本会话选项 1「批准并修正」）
- date: 2026-11-14
- reason: `critic` 规格门报告无 blocker、8 条 residual 已按上节处置；用户拍板批准 P2 载体与 P3 技术方案 D1–D3 并授权按 residual 修正后回填。指纹由 `spec_artifact_fingerprint()` 在四件套修订完成后计算，`decision-memo.md` 不在其纳入范围内，回填不自指死锁。
