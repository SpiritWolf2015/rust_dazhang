# Tasks

> 本文件为 p15 的实施清单，**本轮只成文、不实施代码**，全部复选框保持 `- [ ]` 未勾，apply 是后续独立会话。本 change 为非玩法 / 呈现层证据工作流 change（owner 待 leader 拍板，一个 change 一个 owner），零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不新增依赖；`CODEBUDDY.md §5` 硬门禁与形态①反向验证**不适用**，故本清单**不含** `cargo test -p w3-core --test determinism` 任务（不臆造内核改动）。`## 规格门` 保持 `status: pending`、批准记录由人拍板后回填。所有试点特性 / 合规判定 / 交接格式 / 验收判据 / Deep + Clean room 组合均为**规格门待确认项**，须先拍板再执行；不勾任一复选框、不 self-approve。

## 1. 规格门待确认项收敛（前置，apply 阶段由人拍板）

- [x] 1.1 由规格门用户拍板**试点特性**：在 `decision-memo.md`「待确认项」④ 的推荐（单 PE2 / 飞弹特效「混合模式 + 透明排序」视觉 oracle 对照，样本 `FireBallMissile` 或 `PriestMissile` 的 `Stand` / `Death` 两态、固定机位 / 动作 / 时间点）与备选 A（MDX 动画混合，管线未核实）/ B（水面绘制顺序，离屏是否覆盖水面未核实）/ C（HUD / UI 观感，不推荐）之间选定，记录「推荐 + 依据 + 代价 + 备选」的拍板结论。验证：`decision-memo.md`「待确认项」④ 有明确拍板记录（含证据锚），MUST NOT 写成 AI 自定 / 沉默授权；判据收敛到「单个特效 / 固定机位」，不宣称解决全局透明排序。
- [x] 1.2 由规格门用户收敛其余 **5 项参考缺口**：① 合法自备 / 不绕 EULA · DRM · 登录 · 反作弊的逐条合规判定；② 行为规格文档字段 / 粒度 / 版本化与「仓库外证据目录 → 主仓库只读文档交接」格式；③ 视觉 oracle 验收判据（动作 / 时间点 / 机位固定、像素差异阈值、人眼判定与机器断言分界）；⑤ 「Deep 模式 + Clean room 角色隔离」组合（两次运行？角色分工？是否覆盖本机经典客户端）。验证：`decision-memo.md`「待确认项」①②③⑤ 逐条有拍板结论或显式豁免；`craft-anything/README.md:143-147,240-248` / `docs/claude-code.md:63` / `crates/formats/README.md:1-75` 的缺口如实登记、不臆造答案。

## 2. 生产侧 clean-room 行为规格产出（仓库外本机，apply 阶段）

- [ ] 2.1 按拍板结论在**仓库外本机证据目录**配置 `craft-anything` 作为本机外部工具（CLI / MCP）运行，逐条核对合规前提：仅用户合法自备的经典客户端、不绕 EULA / DRM / 登录 / 反作弊、Clean room 不是 OS 沙箱的额外约束。验证：合规逐条判定记录留证据锚（ADR 0001 复核记录 `0001-license.md:80-82`）；`git status --short` 无新增代码 / 依赖 / 资产，`Cargo.lock` / `NOTICE.md` 零改动，工具本体不入库、不复制上游代码、不接 CI。
- [ ] 2.2 以该工具从原生二进制产出**行为规格文档**（伪 C / 调用图 / 呈现层行为描述），覆盖拍板选定的单特性（如混合模式 + 透明排序的观察字段）。验证：产物是行为规格文档而非可编译代码 / 业务逻辑 oracle；伪 C / 抓帧二进制 / 证据原始产物 MUST 留在仓库外本机证据目录、不入库；MUST NOT 用 `craft-anything` 解析 MPQ / BLP / MDX / SLK（`crates/formats/README.md:1-75` 无专用解析器），资产侧走 `godot_warcraft3` 导出管线。
- [ ] 2.3 把参考项目 AlphaKey 阈值 / FilterMode 等数值登记为 `[待核]` 候选观察字段，不冻结进权威定义；WC3 数值以本机导出 SLK 为唯一来源。验证：行为规格文档数值条目带 `[待核]` 标注（`WC3-MELEE-RULES.md:7,126` / §14 第 2 条）；`git diff -- crates/w3-core/` 为空、不进状态哈希 / 快照。

## 3. 交接与证据纪律（只读文档交接，apply 阶段）

- [ ] 3.1 按拍板的交接格式，把行为规格与证据记录经**仓库外本机证据目录 → 主仓库只读文档交接**落地（生产侧与消费侧只通过只读文档交接）。验证：主仓库只新增只读文档交接件；`git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 派生 JSON / PNG / GLB / 音效；原始伪 C / 抓帧产物不入库。
- [ ] 3.2 落实证据纪律「判定须经独立重审」：每条判定与其依据产物同提交、并留**独立重审**记录。验证：判定记录与产物同批、含独立重审锚；MUST NOT 由产出方自审即标通过；文字纪律无机械强校验、如实登记为结构性改善（`critic` 内容层复核），不冒充门禁。

## 4. 消费侧视觉 oracle 单特性试点（apply 阶段）

- [ ] 4.1 复用 `crates/w3-render` 离屏 `render_headless_rgba` / `render_headless_rgba_with_textures`（`crates/w3-render/src/gpu/mod.rs:908-927`）+ ViewImage，对照拍板选定的单特性，固定机位 / 动作 / 时间点、以模拟帧编号命名截图（依据 `godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:48,14,10`）。验证：离屏 RGBA 输出与截图命名可复核；`git diff -- crates/` 为空（零代码改动、只读复用）；无 GPU 适配器时按 `mod.rs:915` 返回 `GpuUnavailable`、不冒充通过。
- [x] 4.2 按 `FX_BAKE_FIDELITY.md:50,59` 明确「Headless 检查不证明 GPU 画面正确；截图验证也不等于原作保真验收」，把真窗口人工验收与「相同动作 / 时间点 / 机位的原作参考帧」列为人工项。验证：试点结论 MUST NOT 写成「GPU 画面正确 / 原作保真验收已通过」；真窗口 8.x 人工清单保持「未执行 / 待人工」、不标通过；判据收敛到单特效 / 固定机位、不宣称解决全局透明排序。
- [x] 4.3 视觉 oracle 证据与权威状态隔离自证：证据不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()` / golden hash / `determinism` 验收 / CI 门禁。验证：`git diff -- crates/w3-core/` / `-- crates/w3-net/` 为空；`SNAPSHOT_VERSION` / `SIM_VERSION`（`snapshot.rs:67-68`）与 `PROTOCOL_VERSION`（`packet.rs:50`）仍 == 9；不接 CI、不改 `tools/**`。

## 5. 成文与收尾的结构自证（成文期 + apply 阶段）

- [x] 5.1 成文期校验：`openspec validate p15-presentation-evidence-workflow --strict` 与 `python tools/check_encoding.py`；完整 `python tools/verify_all.py` 由 leader 跑（本轮不跑）。验证：`openspec validate --strict` 零 error（>500 字 requirement 类 WARNING 如有则如实标注、不冒充零告警）；编码 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；`decision_memo_check()` 为 True；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属**预期 fail-closed 信号**，如实登记、不绕过、不填 `approved` / 指纹、不冒充「统一门禁全绿」；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免。
- [x] 5.2 **规则文档同步核对**（按 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条如实给结论）：本 change 为非玩法 / 呈现层证据工作流 change，不实现任何玩法、不触及 `R-x.y` 实现状态 / §13.7 现状 / 附录「规则 → 实现覆盖对照」/ §15 变更记录 / 参考项目数量（仍五个），结论为**无需同步**并写明依据；MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」。验证：`decision-memo.md`「## 规则文档同步」小节非空且结论真实、`git diff -- docs/references/WC3-MELEE-RULES.md` 为空；`rule_doc_sync_check()` 对 pending / 非玩法 change 豁免。
- [x] 5.3 参考项目对照与红线自证：核对 `design.md`「## 参考项目对照」逐条登记 T5 主命中、T1 / T3 / T7 可选交叉（仅红线）、T2 / T4 / T6 不命中，T1 承重锚为 `vera20k@305ea3a75:<path>:<line>` 固定提交双标（不退化成工作树 HEAD 行号）；五个只读参考项目不变、`craft-anything` 不登记为第六个；AGPLv3 / Section 7 / `ConcordRTS` RA2/YR 边界逐条成立。验证：`git -C vera20k show 305ea3a75:src/net/lockstep.rs` 可核对 `:10` 原文；对照报告本轮已产出、结论落 `design.md`；参考缺口 ①–⑤ 仍在「待确认项」、不臆造；由 `critic` 清单第 12 项复核。
- [x] 5.4 决策纪要 / 规格门结构自证：确认 `decision-memo.md` 三必填小节（承重约束 / 拍板决策 / 待确认项）非空、每条承重约束带 `file:line` / commit / 命令输出证据锚、「## 已知限制」「## 规则文档同步」非空、「## 规格门」**仅** `- status: pending` + 一段说明、无 `fingerprint` / `approved_by` / `date` / `reason` 字段行、无省略说明、不 self-approve。验证：`python -c "import sys;sys.path.insert(0,'tools');import verify_all as v;print(v.decision_memo_check())"` 输出 True；`## 规格门` 置于末尾且只有本节以「规格门」开头；`decision-memo.md` 不在四件套指纹集合内。
- [x] 5.5 git 边界自证：确认改动仅落在 `openspec/changes/p15-presentation-evidence-workflow/`，未改 `openspec/specs/**` / `openspec/changes/archive/**` / p12 / p13 / p14 / `docs/**` / `README.md` / `CONTRIBUTING.md` / `crates/**` / `tools/**`；不 `git add` / `commit` / `push` / `fetch` / `pull` / `clone` / `archive`、不勾任一复选框。验证：`git status --short` 仅显示 p15 新目录为未跟踪（工作区既有 ADR / README / CONTRIBUTING 修改与 p12 / p13 / p14 保持原样、不被本 change 碰）；`git diff -- openspec/specs/` / `-- docs/` / `-- crates/` / `-- tools/` 为空；不追溯改写 P11 归档件、不改 p14「明确不纳入」边界。

## Workflow follow-up

- 规格门：`critic` 按清单第 7 / 10 / 12 / 13 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ leader / `spec-steward` 回填 `status: approved` + 四件套指纹（`python tools/verify_all.py --fingerprint p15-presentation-evidence-workflow`）+ `approved_by` + `date`；本轮 `status: pending`、AI 未 self-approve、未填指纹。
- `reference-researcher` 对照报告本轮已产出、结论落在 `design.md`「## 参考项目对照」；T5 主命中判定与承重锚待 `critic` 清单第 12 项复核；参考缺口一律回到用户拍板、不臆造。
- apply 在后续独立会话实施（一个 change 一个 owner、不同时改多 owner 文件），本轮不执行任一复选框、不勾任一任务；试点特性 / 5 项参考缺口须先经规格门拍板。
- 真窗口 8.x 人工验收、golden hash v9 双平台逐字节比对、CI 双平台一律「待人工 / 待 VM」、不冒充通过；不得据对照报告声称跨平台确定性已达成。
- 人工 push / CI 双平台 `verify_all.py` / `/opsx:archive` 由人工与 leader 处理；AI 不得 push / fetch / pull / clone、不得 archive、不得本地 commit（本轮统一由 leader 处理）。
