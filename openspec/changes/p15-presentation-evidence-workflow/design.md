# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「呈现层证据流程（形态 A 试点）怎么落地」。事实来源为**本仓库 artifacts + 五个只读参考项目只读对照 + 仓库文件外的人工确认项**（逐条带可核验锚，不臆造）：

- **ADR 承接**：`docs/decisions/0003-scope.md:109-123`（2026-11-25 第二次修订）把「反编译 / REA 整套工作流截至第一次修订仍未引入」翻转为引入 clean-room 反编译证据流程试点，形态 A = `craft-anything` 本机外部工具 + 仓库外证据目录 + 只读文档交接 + 消费侧既有呈现层路径；`:116` 明文「具体特性选择由后续试点 change 的规格门拍板」「本修订不固化任何模板 / 规则」；`:121-122` 补充复核点并写明「p15 试点 change 的四件套成文属后续独立任务」。`docs/decisions/0001-license.md:77-83` 复核记录结论「本次不触发许可证变更」（形态 A 不入库 / 不复制代码 / 不加依赖 / 不登记 `NOTICE.md`），并新增「反编译 / 反汇编产物一律留本机、不入库、不随发行物分发」「不绕过任何 EULA / DRM / 登录 / 反作弊」。
- **长期目标与红线**：`0003-scope.md:94-100`（第一次修订已撤回「目标错配」否决、明确像素级复刻为长期目标）/ `:102-104` / `:117-120`（红线不变、表现层只读派生、证据不进权威状态 / 状态哈希 / 快照 / golden hash / determinism / CI；不 bump 三版本号；`w3-core` 零改动）。`docs/decisions/0002-determinism.md:49-53`（§5 状态权威与表现边界：`RtsMatch` 唯一写入者、表现层只读视图、不得反写）。
- **消费侧既有能力**：`crates/w3-render/src/gpu/mod.rs:908-927`（`render_headless_rgba` / `render_headless_rgba_with_textures` 离屏 RGBA 两变体，`:915` 无 GPU 适配器时返回 `GpuUnavailable`、不冒充通过）、`:593`（`wgpu::BlendState::ALPHA_BLENDING`）、`:579-582`（depth write / compare）。`0003-scope.md:28,30,55`（P1 不写原生解析器、P3 转换规格取自 `godot_warcraft3`、合规模型）。
- **三版本号**：`crates/w3-core/src/snapshot.rs:67-68`（`SNAPSHOT_VERSION` / `SIM_VERSION` == 9）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION` == 9）；`openspec/config.yaml:13` / `CODEBUDDY.md §5` 亦记「当前 9」。本 change 零新增权威状态、不 bump。
- **工程治理约定（本项目自定）**：生产侧与消费侧只通过只读文档交接、判定须经独立重审（判定与产物同提交 + 独立重审）是本项目自定的工程治理约定，不复制任何外部代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源、不登记为第六个参考项目。`craft-anything`（MIT OR Apache-2.0，纯 Rust clean-room 逆向 + AI 重建引擎）是**本机外部工具**，不是参考项目。

**参考矩阵命中判定**：本 change 是**工作流 + 呈现层证据**（新增流程规范 + 单特性试点），不改解析 / 玩法 / 网络 / 权威内核实现。经核对 `docs/references/UPSTREAM.md:104-114` 矩阵：**T5（场景 / 相机 / HUD / 3D 呈现，`UPSTREAM.md:110`）主命中**（视觉 oracle 的方法论与呈现层保真验收锚来自 `godot_warcraft3` FX / PE2 / HUD / PIPELINE 文档）；**T1 / T3 / T7 可选交叉（仅红线）**（T1 `vera20k` 权威边界、T3 `godot_warcraft3` 资产管线不入库、T7 `ConcordRTS` 确定性内核与表现层分离方法论红线）；**T2 / T4 / T6 不命中**（不改移动 / 寻路、不实现 JASS / 玩法语义、不改网络传输）。`reference-researcher` 对照报告**本轮已产出**，逐条结论见下「## 参考项目对照」。

## Goals / Non-Goals

**Goals:**
- 新增单一 capability `presentation-evidence-workflow`，用 SHALL/MUST 把「生产侧行为规格 → 仓库外证据目录 → 主仓库只读文档交接 → 消费侧视觉 oracle」的行为契约、红线与可执行验证一次成文。
- 让**试点特性、5 项参考缺口、合规逐条判定、行为规格字段 / 交接格式、视觉 oracle 验收判据、Deep 模式 + Clean room 角色隔离组合**被参数化承接为规格门待确认项（spec 用行为式 SHALL、tasks 用条件式措辞），规格门拍板这些二选一时**不推翻 spec requirement 与 task 结构**。
- 显式声明零 `w3-core` 权威状态改动、零确定性影响、不 bump、不动资产、不新增依赖、不改 ADR 正文 / 主 specs / 业务规则；**不重跑** `cargo test -p w3-core --test determinism`（不臆造内核改动）。
- 把 `reference-researcher` 对照报告结论逐条落进「## 参考项目对照」，T1 承重锚保持 `vera20k@305ea3a75:<path>:<line>` 固定提交双标，红线边界逐条声明、缺口如实登记。

**Non-Goals:**
- 不改 `crates/**` 任何实现（含 `w3-render` / `w3-content` / `w3-core`）；不改 ADR 0001 / 0002 / 0003 正文；不 bump 三版本号（9 不变）；不新增权威状态。
- **本轮只成文**：不新增 / 不改 `tools/**` / `openspec/config.yaml` / `CODEBUDDY.md` / `docs/**` / `openspec/specs/**` / 归档件 / 根配置 / p12 / p13 / p14 / `README.md` / `CONTRIBUTING.md`；apply 阶段才谈落地。
- **不纳入**：把 `craft-anything` 复制 / 加为 crate 依赖 / 登记 `NOTICE.md` / 接入 CI；把反编译产物或任何 MPQ 派生数据入库；把视觉 oracle 当业务逻辑 oracle 去改 `w3-core` / determinism / golden hash 门禁；实现任何玩法规则 `R-x.y`；登记第六个只读参考项目；改 p14「明确不纳入」边界；追溯改写 P11 归档件；多特性一次性推广或固化模板 / 规则；真窗口人工验收（8.x）、golden hash 双平台、Q1 清单水位。
- 不 archive / 不 commit / 不 push / 不 fetch / 不 pull / 不 clone / 不勾任一复选框 / 不 self-approve 规格门。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、**对跨平台确定性的影响（ADR 0002）**、分层依赖 / 架构原则。全部决策均为**工作流 / 文档 / 证据纪律 / 只读对照**，对确定性契约零影响：不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号。

### D1 流程形态：形态 A（本机外部工具 + 仓库外证据目录 + 只读文档交接）

- **决策**：生产侧以 `craft-anything`（本机外部工具，CLI / MCP）从合法自备经典客户端原生二进制取得伪 C / 调用图 / 行为规格，产出**行为规格文档**（不是代码、不是业务逻辑 oracle）；行为规格与证据记录存**仓库外本机证据目录**，主仓库只接收**只读文档交接**（生产侧与消费侧只通过只读文档交接）。工具不入库 / 不复制代码 / 不加 crate 依赖 / 不登记 `NOTICE.md`。
- **备选方案（被否）**：① 把反编译工具链复制进仓库 / 加为依赖 / 接 CI（被否：`0003-scope.md:114,121` 明定形态 A「不入库 / 不作为 crate 依赖」，且「决定引入外部逆向工具的代码 / 依赖 / 接入 CI」须回 ADR 0001 复核点重评许可证并登记 `NOTICE.md`，超试点边界）；② 把伪 C / 抓帧产物直接提交进仓库当「证据」（被否：ADR 0001 复核记录 `0001-license.md:81`「反编译 / 反汇编产物一律留本机、不入库、不随发行物分发」）；③ 不设证据目录、模型直接读原生二进制给结论（被否：违反只读文档交接纪律，且判定须经独立重审）；④ 本轮一次性推广到全部呈现特性并固化模板 / 规则（被否：用户选「形态 A 试点」，`0003-scope.md:116` 明文「本修订不固化任何模板 / 规则」，工作流改进先试点再固化）。
- **依据**：`0003-scope.md:113-116`（形态 A 决议）/ `:104`（既定路径）/ `:121-122`（复核点补充）；`0001-license.md:79-82`；`UPSTREAM.md:63-92`（导出数据不入库的既有合规模型）。
- **代价**：交接格式 / 字段 / 粒度 / 版本化未定义，属规格门待确认项；Clean room 不是 OS 沙箱（`craft-anything/README.md:143-147`），合规姿态仍靠红线 + 人工逐条判定；仓库外证据目录不进版本控制，可复现性靠文档纪律而非门禁。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。行为规格与证据记录是纯文档、留仓库外，不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()`，不改 `step()` / `ExecuteFrame→PlayerId→Sequence→Arrival` 命令排序；不 bump 三版本号（9 不变）。跨平台确定性本身不受影响；本 change 不据此声称 macOS / ARM64 / 其它 rustc 已达成（ADR 0002 复核记录口径）。
- **分层依赖 / 架构原则**：DIP（消费侧只依赖只读文档契约，不依赖外部工具运行时）、单一状态所有者（外部工具不写模拟状态）、SRP（生产 / 交接 / 消费三段职责分离）、诚实边界（文档 ≠ 门禁、工具 ≠ 依赖）。

### D2 证据纪律：判定与产物同提交 + 独立重审

- **决策**：每条「行为规格 → 视觉对照」判定 SHALL 与该判定所依据的产物同批提交，并经受**独立重审**（判定须经独立重审）；重审 MUST NOT 由产出方自己宣布通过。判定与产物同提交、独立重审只作用于仓库外证据 / 只读交接文档，不触碰任何确定性门禁。
- **备选方案（被否）**：① 只交判定结论、不交产物（被否：无法独立重审，与判定须经独立重审相悖）；② 由产出模型自审即标通过（被否：同上，「终审判定没随产物提交、需独立重审」）；③ 把重审做成 `tools/*.py` 机械门禁挂 `verify_all.py`（被否：本轮只成文、且重审是人工证据纪律而非可机械判定的字节断言，机械门禁无法校验证据真伪，不冒充门禁）。
- **依据**：本项目自定的工程治理约定（判定须经独立重审，p14 `decision-memo.md` 同类口径）；`CODEBUDDY.md §8` 规格门由人拍板、不拿沉默当授权。
- **代价**：重审方式 / 判定粒度 / 验收判据属规格门待确认项；文字纪律无机械强校验，只能靠 `critic` 内容层复核，如实登记为结构性改善、非机械保证。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。判定 / 产物 / 重审记录均为文档，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）；**不触发** `cargo test -p w3-core --test determinism`。
- **分层依赖 / 架构原则**：显式失败（判定未同提交即不成立）、诚实边界（不把自审当通过）、ADR 记「为什么」与 spec 记「该做什么」不打架。

### D3 消费侧：视觉 oracle 复用既有离屏能力，不改 `w3-render`

- **决策**：消费侧沿用 `crates/w3-render` 离屏 `render_headless_rgba` / `render_headless_rgba_with_textures`（`mod.rs:908-927`）+ ViewImage + 真窗口人工验收作**视觉 oracle**，零代码改动复用既有能力。判据 MUST 收敛到「相同动作 / 时间点 / 机位的原作参考」（`FX_BAKE_FIDELITY.md:50`）；Headless 检查与截图验证 MUST NOT 冒充「GPU 画面正确」或「原作保真验收已通过」。证据是视觉 oracle，不是 ADR 0002 口径「可冻结、机器可判、可进 CI 的业务逻辑 oracle」。
- **备选方案（被否）**：① 为试点新增 `w3-render` 接口 / 离屏场景 / 像素比对脚本（被否：本轮只成文、零 `crates/**` 改动；且离屏场景是否覆盖水面 / 动画混合**未核实**，贸然改动会牵动 `w3-content` / 动画管线，属备选 A / B 的未核实风险）；② 把离屏 RGBA 像素差异阈值当可进 CI 的机器断言（被否：`FX_BAKE_FIDELITY.md:50,59` 明示 Headless 不证明 GPU 画面正确、跨透明排序仍受 Godot 限制；阈值 / 人眼与机器分界属规格门待确认项，不臆造数值）；③ 用真窗口截图取代离屏、跳过人工验收（被否：真窗口 8.x 人工清单不在本轮执行、不标通过，且原作参考帧需合法自备客户端）。
- **依据**：`crates/w3-render/src/gpu/mod.rs:908-927` / `:593` / `:579-582`；`0003-scope.md:103-104,115`（既定路径：离屏像素对照 + 真窗口人工验收 + 视觉 oracle 非业务逻辑 oracle）；`godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:48,50,59`。
- **代价**：需合法自备经典客户端 + 相同动作 / 时间点 / 机位的原作参考帧；无 GPU 适配器时离屏返回 `GpuUnavailable`、测试跳过（`mod.rs:915`，不冒充通过）；试点判据须收敛到「单个特效 / 固定机位」，不宣称解决全局透明排序。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。离屏渲染是表现层只读派生、不反写模拟状态（`0002-determinism.md:49-53`）；不进状态哈希 / 快照 / `first_difference()`、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）。GPU 画面在 Windows / Linux 是否逐字节一致属 ADR 0002 待验证项，本 change 不声称达成、不把离屏 RGBA 当跨平台确定性证据。
- **分层依赖 / 架构原则**：表现层只读派生、唯一状态所有者、不复制代码、诚实边界（视觉 oracle ≠ 业务逻辑 oracle、离屏 ≠ 真窗口）。

### D4 资产 / 数值来源不迁移：仍以 `godot_warcraft3` 导出管线为准

- **决策**：WC3 资产与数据表仍走 `godot_warcraft3` 三车道（MPQ→SLK / BLP / MDX→地图 JSON、gitignore 不入库，`UPSTREAM.md:51,63-92` + `PIPELINE.md`）；`craft-anything` 无 MPQ / BLP / MDX / SLK 专用解析器（缺口）。参考项目内 AlphaKey 阈值 / FilterMode 等数值只作 `[待核]` 候选观察字段，WC3 数值唯一来源为本机导出 SLK（`WC3-MELEE-RULES.md:7,126`），MUST NOT 冻结进权威定义。
- **备选方案（被否）**：① 用 `craft-anything` 解析 MPQ / MDX 取资产（被否：`crates/formats/README.md:1-75` 格式清单无专用解析器、全文 grep 无匹配，工具 early alpha、无 live 验证先例）；② 把 `FX_BAKE_FIDELITY.md:10` / `PIPELINE.md:99-101` 的 AlphaKey 阈值 0.83 / FilterMode cutoff 0.75 / 0.08 等 Godot 映射数值当 WC3 权威数值冻结（被否：那是 Godot 渲染器映射值、非 WC3 原值，`WC3-MELEE-RULES.md §14` 第 2 条要求 `[待核]` 不静默改规则、数值以本机导出 SLK 为准）；③ 把 MPQ 派生 JSON / PNG / GLB 提交进仓库便于对照（被否：ADR 0001 零资产红线）。
- **依据**：`docs/references/UPSTREAM.md:51,63-92,108`（T3）；`godot_warcraft3@18b7a70:docs/data/PIPELINE.md:10-31,69-101,99-101,103-134,140-143,152-180,229-239` / `docs/architecture/ASSET_LANES.md:43,45-49`；`craft-anything/crates/formats/README.md:1-75`；`WC3-MELEE-RULES.md:7,126`；`0003-scope.md:28,55,115`。
- **代价**：资产侧能力完全依赖既有导出管线，外部工具只补行为规格；`[待核]` 数值不得冻结，试点判据里的数值口径属规格门待确认项。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。导出数据是冻结输入、不进快照 / 状态哈希（`CODEBUDDY.md §5`）；`[待核]` 观察字段不进权威定义、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：冻结输入边界、不静默改规则、零资产红线、`w3-core` 不反向依赖内容层。

### D5 确定性 / 资产 / 依赖整体边界：零 `w3-core` 改动、不跑 determinism

- **决策**：本 change 零 `w3-core` 权威状态改动、不进状态哈希 / 快照 / `first_difference()`、不 bump 三版本号（仍 9）、不动资产、不新增第三方依赖、不改 ADR 0001 / 0002 / 0003 正文、不改主 `openspec/specs/**`、不改业务规则、不接 CI；**不重跑** `cargo test -p w3-core --test determinism`（`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用），但跑 `openspec validate p15-presentation-evidence-workflow --strict` / `python tools/check_encoding.py` 保证成文不回归；边界以 `git diff -- crates/w3-core/` 为空、三版本号仍 9、无资产、无新依赖自证。
- **备选方案（被否）**：① 「保险起见」也跑一遍 determinism 并写进 tasks（被否：用户明确零内核改动，放了反而暗示存在内核改动、`CODEBUDDY.md §5` 形态①不适用；沿用 p14 `design.md` D4 / D7 诚实边界）；② 借本 change 顺手 bump 版本号 / 改 ADR 正文 / 接 CI（被否：超 Scope，用户明确本轮只成文、不 bump、不改 ADR 正文）；③ 把反编译证据纳入 golden hash / CI 当「可复现验收」（被否：`0003-scope.md:103,119` 明文证据 MUST NOT 进 golden hash / determinism / CI，视觉 oracle 不是机器可判业务逻辑 oracle）。
- **依据**：`CODEBUDDY.md §5` / §6 / §8 / §9；`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`；`0003-scope.md:117-120`；`0002-determinism.md:49-53`；`tools/verify_all.py:318-348`（统一门禁体例）。
- **代价**：本轮 `python tools/verify_all.py` 由 leader 跑、本轮不跑；`spec_gate_check()` 在 `status: pending` 期间属预期 fail-closed，如实登记、不绕过；`decision_memo_check()` 三必填小节非空、`rule_doc_sync_check()` 对 pending / 非玩法豁免。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。不新增权威状态（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照）、不改 `step()` / 命令排序 / 状态哈希编码；跨平台确定性待验证项（macOS / ARM64 / 其它 rustc）不因本 change 变化、不声称达成。
- **分层依赖 / 架构原则**：唯一状态所有者、只读参考不复制代码、显式失败、诚实边界（零内核改动不冒充全量门禁、不把「引入工作流」写成已放宽红线）。

### D6 待确认项的参数化承接：拍板不改变 specs / approach / task 结构

- **决策**：把「试点特性」「合规逐条判定」「行为规格字段 / 粒度 / 版本化 + 交接格式」「视觉 oracle 验收判据」「Deep 模式 + Clean room 角色隔离组合」及 5 项参考缺口全部登记进 `decision-memo.md`「待确认项」，由规格门 / leader 拍板。四件套以**中性口径**承接：spec 写「SHALL 限单个呈现特性、由规格门用户拍板」「SHALL 只接收只读文档交接」、tasks 用「按规格门拍板结论」条件式措辞、试点特性只给「推荐 + 依据 + 代价 + 备选」不锁死。因此这些项属「可安全延后而不改变 specs / approach / task breakdown」的未知项，符合 `openspec instructions design` 对 Open Questions 的定义，**不是**被跳过的决策。
- **备选方案（被否）**：① 本轮替用户把试点特性 / 5 项参考缺口 / 合规判定拍成定案（被否：用户只拍板「引入工作流 + ADR 修订 + 形态 A 试点 + 单独立项」，`0003-scope.md:116` 明文「具体特性选择由后续试点 change 的规格门拍板」，无证据项 MUST NOT 写「用户已确认」）；② 写省略说明求门禁变绿（被否：`spec_gate_check()` 对 `status: pending` 是预期 fail-closed 信号，本 change 非「目标与方向已清楚的小改动」，不 self-approve、不写省略说明）；③ 把参考缺口臆造成已有答案（被否：`openspec/config.yaml rules.design` / `CODEBUDDY.md §9` 要求「参考缺口如实登记并回到人拍板，不臆造」）。
- **依据**：`tools/verify_all.py:207-253`（`spec_gate_check()` pending fail-closed）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（四件套指纹不含 `decision-memo.md`）；`openspec/decision-memo.template.md:14,55-61`；p12 / p13 `decision-memo.md`「## 规格门」pending 体例；`CODEBUDDY.md §8` / §9（规格门由人拍板）。
- **代价**：apply 阶段须先等规格门拍板才能确定试点特性 / 合规判定 / 判据；本轮成文不阻塞，但 `python tools/verify_all.py` 会因 pending 报 fail-closed（如实登记、不绕过、由 leader 跑）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。全部是流程 / 参数 / 文档决策，不进状态哈希 / 快照、不改 `step()` / 命令排序、不 bump 三版本号（9 不变）。
- **分层依赖 / 架构原则**：参数与策略分离（OCP）、不拿沉默当授权、批准由人拍板、诚实边界。

### D7 参考项目对照与红线不越界（T5 主命中、T1 固定提交双标）

- **决策**：`reference-researcher` 对照报告本轮已产出，逐条结论落在「## 参考项目对照」。T1 承重锚一律 `vera20k@305ea3a75:<path>:<line>` **固定提交双标**（`vera20k` 工作树 HEAD 已前进至 `b17d5282…`，按 `UPSTREAM.md:19-25` 先例保留原锚、不据前进改写任何 ADR）；T5 MUST NOT 以 `ConcordRTS` 为参考（`UPSTREAM.md:114`）；五个只读参考项目不变。
- **备选方案（被否）**：① 把 T1 锚退化成工作树 HEAD 行号（被否：`UPSTREAM.md:11,19-25` 先例「保留原锚、不据前进改写」，p13 `decision-memo.md` C8 与「被否方向台账」固定提交双标条 同口径；本轮 `git -C vera20k show 305ea3a75:src/net/lockstep.rs` 核对 `:10` 原文成立、工作树 `sed -n 10p` 已不同，退化会丢证据）；② 把 `craft-anything` 登记为第六个只读参考项目（被否：`CODEBUDDY.md §1` / `UPSTREAM.md:9-15,96,100` 参考项目只读五个，沿用 p14 `design.md:112` 口径）；③ 以 `ConcordRTS` 作 T5 呈现层参考或以其 README「逐字节一致」自述作已验证事实（被否：`UPSTREAM.md:114` 明令 T3 / T4 / T5 MUST NOT 以其为参考、`:35` README 自述不作已验证事实；其 RA2/YR 专属格式 / 规则 / 数值绝不作 WC3 事实来源）。
- **依据**：`docs/references/UPSTREAM.md:104-114`（矩阵与交叉注记）/ `:19-25`（固定提交升级复核先例）/ `:35`（README 自述）/ `:96,100`（五个只读、不复制代码）；`CODEBUDDY.md §9`（凡 T1–T7 命中 MUST 有「## 参考项目对照」）；`openspec/config.yaml rules.design`。
- **代价**：本轮只读抽查打开承重锚（未逐行复核报告全部行号），报告其余锚的逐条核实留 `critic` 清单第 12 项复核；如实登记为已知限制、不冒充全量复核。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**。参考项目只读、不复制代码、不引入依赖，不进状态哈希 / 快照、不改命令排序、不 bump 三版本号（9 不变）；`ConcordRTS` 的「逐字节一致」验收方法论只作红线对照，不作本项目跨平台确定性已达成的证据。
- **分层依赖 / 架构原则**：只读参考不复制代码、许可证红线（AGPLv3 / Section 7 / GPL-3.0-only）、证据锚诚实边界（固定提交双标、不据 HEAD 前进改写）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-114` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。本 change **T5 主命中**、T1 / T3 / T7 可选交叉（仅红线）、T2 / T4 / T6 不命中；`reference-researcher` 对照报告本轮已产出，以下逐条引用其结论与承重锚（均为本轮报告内实际打开、本设计只读抽查复核）。

| 矩阵 | 主参考项目 / 路径 | 命中 | 借鉴 / 不借鉴（含许可证理由） |
|---|---|---|---|
| T1 确定性内核 / lockstep / 状态哈希 / 命令排序 / 快照（`UPSTREAM.md:106`，`vera20k`） | `vera20k@305ea3a75:src/net/lockstep.rs:1-14`、`vera20k@305ea3a75:docs/Networking.md:3-6,599-600,615` | 可选交叉（仅红线） | 借「字节先权威、解码仅在 dispatcher 准入事件后发生」的**权威边界红线**（`lockstep.rs:10` "Bytes stay authoritative there; decoding happens only after the dispatcher admits an event."），用来说明「反编译证据 / 解码产物 MUST NOT 反向进入权威状态」；**不借**其 RA2 实现 / 线格式，GPLv3 + Section 7 附加条款**不触及**（只读不复制，`ADR 0001:14,30,66-73`） |
| T2 移动 / 寻路 / 编队 / 帧阶段（`UPSTREAM.md:107`，`rts_kernel_cs`） | `rts_kernel_cs` `docs/` | 不命中 | 本 change 不改移动 / 寻路；其 `double` 路线本就不采用（ADR 0002），无借鉴、无缺口 |
| T3 WC3 数据格式 / SLK / 资产管线（`UPSTREAM.md:108`，`godot_warcraft3`） | `godot_warcraft3@18b7a70:docs/data/PIPELINE.md:10-31,69-101,99-101,103-134,140-143,152-180,229-239`、`docs/architecture/ASSET_LANES.md:43,45-49` | 可选交叉（仅红线） | 借「MPQ→SLK / BLP / MDX→地图 JSON 三车道 + gitignore 派生数据不入库」的**合规红线**（D4）；**不借** Godot / GDScript 实现、不借 AlphaKey 阈值 / FilterMode 数值当 WC3 事实（只作 `[待核]` 候选观察字段，`WC3-MELEE-RULES.md:7,126`）；MIT、只读不复制代码 |
| T4 玩法语义 / JASS / FDF UI（`UPSTREAM.md:109`，`WarsmashModEngine`） | `WarsmashModEngine/LICENSE:1`、`WarsmashModEngine/README.md:130` | 不命中 | **AGPLv3 不触及**：只核对许可证、不借 JASS 语义、不复制 Java / LibGDX 代码、不引入依赖（ADR 0001 / `UPSTREAM.md:53`）；本 change 非玩法、不实现任何 `R-x.y`；`HUD.md:13` 与 N5 不做 JASS / FDF VM 的冲突使「HUD / UI 观感」为不推荐备选 C |
| T5 场景 / 相机 / HUD / 3D 呈现（`UPSTREAM.md:110`，`godot_warcraft3`） | `godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:10,14,33,48,50,59`、`FX_SAMPLE_ACCEPTANCE.md:3,44,72`、`PE2_GODOT.md:1-34,36`、`MDX_SKINNING_GODOT.md:11-28`、`docs/architecture/ASSET_LANES.md:43,45-49`、`docs/design/game/HUD.md:13,149` | **主命中** | 借呈现层保真**验收方法论**（见下逐条锚）：固定种子 / 相机 / 背景 / 时间步、模拟帧编号命名截图、先校对原作轮廓 / 运动 / 时序、Headless / 截图不等于原作保真验收、PE2 五类混合分开、排序读 SortFarZ / priority plane、金样 `FireBallMissile`；**不借**其原创三族单位名 / 数值（`FACTION_UNIT_ROSTERS.md` 只借结构）、不借 `HUD.md:13`「不复刻 WC3 Console 贴图外壳」当经典 UI 观感事实、不借 Godot 排序限制结论当本项目已验证事实；MIT、只读不复制代码；T5 MUST NOT 以 `ConcordRTS` 为参考（`UPSTREAM.md:114`） |
| T6 网络传输 / relay（`UPSTREAM.md:111`，`vera20k`） | `vera20k` `src/net/` | 不命中 | 本 change 不改 W3N1 帧头 / relay / proto schema（ADR 0005）；`PROTOCOL_VERSION` 仍 9，无借鉴 |
| T7 通用 RTS 架构 / 确定性内核与表现层分离 / 平台适配与跨平台逐字节验收方法论（`UPSTREAM.md:112`，`ConcordRTS`） | `ConcordRTS@32ab245:README.md:3-5,11-13,21-27`、`ConcordRTS@32ab245:API.md:296-312`、`ConcordRTS@32ab245:src/core/determinism/RuntimeAuthorityHashPrimitives.h:1-10` | 可选交叉（仅方法论红线） | 只借「确定性内核与表现层分离 / 平台适配方法论」**红线**，用来说明「视觉 oracle MUST NOT 进权威状态 / 哈希 / 快照」；**不借** RA2/YR 专属格式（SHP / VXL / TMP / MIX / INI）、规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染，绝不作 WC3 玩法 / 格式 / 数值 / 规则事实来源；README「逐字节一致」自述**不作本项目已验证事实**（`UPSTREAM.md:35`）；GPL-3.0-only 与 GPLv3 同族、只读不复制代码不引入依赖 |

**T5 主命中承重锚（逐条引用，`godot_warcraft3@18b7a70` 固定提交）：**

- `docs/design/asset-convert/FX_BAKE_FIDELITY.md:48`：`tests/integration/review_fx_bake.gd` 必须使用真实渲染器、不加 `--headless`，固定种子 / 相机 / 背景与时间步，图片名 020/060/120 是**模拟帧编号** → 本 change 借「固定动作 / 时间点 / 机位」的对照纪律（`decision-memo.md` 待确认项③）。
- `FX_BAKE_FIDELITY.md:50`：原文「Headless 检查不证明 GPU 画面正确；截图验证也不等于原作保真验收。正式验收需加入相同动作、时间点、机位的原作参考。」 → D3 的核心红线（离屏 / 截图不冒充通过）。
- `FX_BAKE_FIDELITY.md:33`：「调整前先校对原作的轮廓、运动和时序，再调整明暗」 → 试点判据先对齐行为时序、再谈明暗 / 像素。
- `FX_BAKE_FIDELITY.md:59`：「跨透明物体的排序仍受 Godot 限制」 → 试点 MUST 收敛到「单个特效 / 固定机位」，不宣称解决全局透明排序（推荐特性代价）。
- `FX_BAKE_FIDELITY.md:10`：「Blend / Additive / Modulate / Modulate2x / AlphaKey 分开映射」「AlphaKey 为 alpha 阈值 0.83 的加法混合」 → 混合模式分类语义借鉴；**0.83 数值只作 `[待核]` 候选观察字段**，不作 WC3 权威数值（`WC3-MELEE-RULES.md:7,126`）。
- `FX_BAKE_FIDELITY.md:14`：「排序读 SortFarZ 与 priority plane」 → 透明排序观察字段借鉴。
- `FX_SAMPLE_ACCEPTANCE.md:3,44,72` 与 `:28-34`：`PriestMissile` / `FireBallMissile` 的 `Stand` / `Death` 两态、蓝色发光体 / 拖尾 / 黑色贴图方块、深 / 浅背景 / 正面 / 顶面视角 / 暂停 / 重播 → 推荐试点特性的样本与两态依据；「帧定位不能反向还原已消散的粒子」「静止预览无法验证移动中的拖尾距离」同为判据边界。
- `PE2_GODOT.md:1-34,36`：特效四类分开、**金样 `FireBallMissile`**（`:36`）→ 推荐样本锚。
- `MDX_SKINNING_GODOT.md:11-28`：MDX 骨骼 / 蒙皮语义 → 备选 A（动画播放 / 混合）的参考存在性；本项目动画混合管线**未核实**、不臆造能力。
- `docs/architecture/ASSET_LANES.md:43,45-49`：视觉 / 数据 / 地图三车道分工 → D4 资产来源不迁移的依据。
- `docs/design/game/HUD.md:13,149`：`:13`「现代底栏三分栏（AOE4 向布局），**不**复刻 WC3 Console 贴图外壳」→ 该自述**不得当经典 UI 观感事实来源**，备选 C（HUD / UI 观感）不推荐。
- `docs/data/PIPELINE.md:10-31,69-101,99-101,103-134,140-143,152-180,229-239`：MPQ→SLK / BLP / MDX→地图 JSON 三车道、gitignore 不入库；`:99-101` 的 `FilterMode=1` cutoff 0.75 / `FilterMode=2` `ALPHA_SCISSOR` 阈值 0.08 / `FilterMode=3/4` ADD → 只作 `[待核]` 候选观察字段、WC3 数值唯一来源为本机导出 SLK。

**本项目 / ADR 锚（对照结论的承重）：**

- `crates/w3-render/src/gpu/mod.rs:593`（`ALPHA_BLENDING`）、`:579-582`（depth write / compare）、`:908-927`（离屏 RGBA 两变体）→ D3 消费侧既有能力锚。
- `docs/references/UPSTREAM.md:51`（`godot_warcraft3` 数据 / 资产格式规格）、`:63-92`（导出数据不入库）、`:104-114`（矩阵 T1–T7 与交叉注记）、`:19-25`（`vera20k` 固定提交升级复核先例）。
- `docs/decisions/0003-scope.md:28,30,55`（P1 不写原生解析器 / P3 转换规格 / 合规模型）、`:102-105` / `:117-120`（红线 / 既定路径 / 复核点）。
- `docs/decisions/0002-determinism.md:49-53`（§5 状态权威与表现边界）。

**红线边界（逐条声明）：**

- AGPLv3 不触及：`WarsmashModEngine/LICENSE:1` / `README.md:130` 仅核对许可证；只借语义、不复制 Java / LibGDX 代码、不引入依赖（`CODEBUDDY.md §9` / ADR 0001）。
- `vera20k` Section 7 附加条款不触及：只读不复制代码，只有真的复制其代码时才随之进入（`UPSTREAM.md:50,106` / `0001-license.md:14,30,66-73`）。
- T5 MUST NOT 以 `ConcordRTS` 为参考（`UPSTREAM.md:114`）；其 RA2/YR 专属格式 / 规则 / 数值绝不作 WC3 事实来源；README「逐字节一致」自述不作本项目已验证事实（`UPSTREAM.md:35`）。
- `rts_kernel_cs` 的 `double` 路线不采用（ADR 0002 / `UPSTREAM.md:52`）。
- `FACTION_UNIT_ROSTERS.md` 原创三族只借结构 / 批次判据，不借单位名与数值（`UPSTREAM.md:110`）。
- `HUD.md:13` 自述「不复刻 WC3 Console 贴图外壳」不得当经典 UI 观感事实。
- `godot_warcraft3` 的 AlphaKey 阈值 / FilterMode 数值只作 `[待核]` 候选观察字段，WC3 数值唯一来源为本机导出 SLK（`WC3-MELEE-RULES.md:7,126`）。
- 资产与派生 JSON / PNG / GLB 一律不入库（ADR 0001 / `UPSTREAM.md:63-92`）。

**`craft-anything` 缺口（如实登记，不臆造答案）：**

- `crates/formats/README.md:1-75`：格式清单**无** MPQ / BLP / MDX / SLK 专用解析器（本轮全文 grep 无匹配，输出 `0`），资产侧仍靠既有 `godot_warcraft3` 导出管线（D4）。
- `README.md:143-147`：Clean room「is not an operating-system sandbox: it doesn't isolate your computer」——工具自己不隔离本机，合规姿态仍靠 ADR 红线 + 人工逐条判定。
- `README.md:240-248`：early alpha、behaviour checks for apps and games「partly there」、部分引擎工具只测过 stand-ins。
- `docs/claude-code.md:63`：「Not yet checked live against Claude Code or an ACP agent」。
- → **「Deep 模式 + Clean room 角色隔离」如何组合落地（两次运行？角色分工？）、是否覆盖本机经典客户端，工具文档无 live 验证先例，属规格门待确认项**（`decision-memo.md` 待确认项⑤），回到人拍板、不臆造。

**参考项目只读、数量与来源（如实登记）：**

- 五个只读参考项目：`vera20k`（GPLv3 + Section 7）、`godot_warcraft3`（MIT）、`rts_kernel_cs`（MIT）、`WarsmashModEngine`（AGPLv3，仅借鉴语义、禁复制代码）、`ConcordRTS`（GPL-3.0-only，只读、RA2/YR 不作 WC3 事实来源）——全部只读、不复制代码、不引入依赖（`CODEBUDDY.md §1` / §9、`UPSTREAM.md:96,100`）。
- `craft-anything`（本机外部工具，MIT OR Apache-2.0）**MUST NOT 登记为第六个只读参考项目**（沿用 p14 `design.md:112` / `decision-memo.md:16` 口径）；生产侧 / 消费侧只读文档交接、判定与产物同提交 + 独立重审均为本项目自定的工程治理约定，不复制代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源。
- 承重约束若来自参考项目，其锚指向矩阵条目（`UPSTREAM.md:104-114`）或参考项目内路径（见 `decision-memo.md` C7 / C10 / C11）。
- **参考缺口 ①–⑤ 如实登记、回到人拍板**（合规逐条判定 / 行为规格字段与交接格式 / 视觉 oracle 验收判据 / 试点特性 / Deep + Clean room 组合，`decision-memo.md` 待确认项），不臆造答案。真窗口 8.x、golden hash v9 双平台、CI 双平台一律「待人工 / 待 VM」，不冒充通过；**不得据本对照报告声称跨平台确定性已达成**。

## Risks / Trade-offs

- **[Risk] 把「引入反编译工作流」误读成已引入代码依赖 / 放宽红线 / 过规格门 / 获暴雪授权** → Mitigation：proposal / specs / D1 / D5 逐处显式否定式声明；`## 规格门` 保持 `status: pending`、AI 未 self-approve；红线锚 `0001-license.md:80-82` / `0003-scope.md:117-120`。
- **[Risk] Clean room 不是 OS 沙箱，工具 early alpha、Deep + Clean room 组合无 live 验证先例，被误当可直接用** → Mitigation：缺口逐条登记在「## 参考项目对照」与 `decision-memo.md`「已知限制」/「待确认项⑤」，由规格门用户拍板；不臆造组合方式、不冒充已验证。
- **[Risk] 把 Godot 的 AlphaKey 阈值 / FilterMode 数值冻结为 WC3 权威数值** → Mitigation：D4 + 红线边界逐条声明「只作 `[待核]` 候选观察字段」、WC3 数值唯一来源本机导出 SLK（`WC3-MELEE-RULES.md:7,126` / §14 第 2 条）。
- **[Risk] 把视觉 oracle / 离屏 RGBA 当 ADR 0002 业务逻辑 oracle，据此改 `w3-core` / determinism / golden hash / CI** → Mitigation：D3 / D5 显式区分「视觉 oracle ≠ 可冻结机器可判业务逻辑 oracle」、tasks 不放 determinism 任务、边界自证 `git diff -- crates/w3-core/` 为空、三版本号仍 9。
- **[Risk] 把 Headless 检查 / 截图验证冒充「GPU 画面正确 / 原作保真验收已通过」** → Mitigation：`FX_BAKE_FIDELITY.md:50,59` 红线、判据收敛「单特效 / 固定机位 / 相同动作时间点机位原作参考」、真窗口人工清单不标通过。
- **[Risk] T1 承重锚退化成工作树 HEAD 行号（`vera20k` HEAD 已前进）** → Mitigation：D7 固定提交双标 `vera20k@305ea3a75:<path>:<line>`，本轮 `git show 305ea3a75:src/net/lockstep.rs` 核对 `:10` 原文成立、工作树第 10 行已不同；按 `UPSTREAM.md:19-25` 先例不据前进改写。
- **[Risk] 把 `craft-anything` 登记为第六个参考项目，或 T5 误以 `ConcordRTS` 为参考** → Mitigation：「## 参考项目对照」数量与交叉注记逐条声明、`UPSTREAM.md:114` 明令、沿用 p14 口径。
- **[Risk] `spec_gate_check()` 对 `status: pending` fail-closed 被误当失败去 self-approve / 写省略说明** → Mitigation：D6 如实登记为预期信号、不绕过、不填指纹 / `approved_by` / `date`；本轮完整 `verify_all.py` 由 leader 跑。
- **[Risk] 规则编号引用被冒充规则已实现、真窗口 / 双平台人工清单被标通过** → Mitigation：spec「非玩法边界」requirement + `decision-memo.md`「## 规则文档同步」无需同步结论（§14 第 5 条）、`critic` 清单第 13 项内容层复核。

## Migration Plan

- **本轮（成文期）**：只在 `openspec/changes/p15-presentation-evidence-workflow/` 内产出四件套 + `decision-memo.md`；`git status --short` 仅该目录新增（另有 p12 / p13 / p14 与工作区已有的 ADR / README / CONTRIBUTING 修改，均不碰）；`## 规格门` 保持 `status: pending`、AI 未 self-approve、不勾任一复选框。
- **规格门**：`critic` 按第 7 / 10 / 12 / 13 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ 由 leader / `spec-steward` 在人确认后回填 `status: approved` + 四件套指纹 + `approved_by` + `date`（本轮不填）；此后规格锁定，改任一 artifact MUST 按 §8 同步回查其余三份并重走规格门。
- **apply（后续独立会话，时机 / owner 分工待 leader 拍板）**：按 D1–D7 落地，一个 change 一个 owner、不同时改多 owner 文件；先收敛规格门待确认项（试点特性 / 合规判定 / 交接格式 / 验收判据 / Deep + Clean room 组合），再按拍板结论执行单特性试点；仓库外证据目录与主仓库只读文档交接分别在各自侧落地，不碰 p12 / p13 / p14 / 归档件 / 主 spec / `crates/**` / `tools/**`。
- **回滚**：本 change 成文产物为纯文档、apply 侧无运行时代码 / 权威状态 / 快照 / 版本号变更，删除 / 回退文档即可；证据留仓库外本机、不随发行物分发，**无确定性回滚面**（ADR 0002）。
- **人工项**：人工 push → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` → `/opsx:archive`；AI 不 push / fetch / pull / clone、不 archive、不本地 commit（本轮由 leader 统一处理）。真窗口 8.x 人工验收、golden hash v9 双平台逐字节比对、CI 双平台一律「待人工 / 待 VM」。

## Open Questions

> 以下均为「可安全延后而不改变 specs / approach / task breakdown」的未知项，已由 D6 参数化承接并登记在 `decision-memo.md`「待确认项」，由规格门 / leader 拍板；本轮**不臆造为已定案**：

- 试点特性（推荐单 PE2 / 飞弹特效「混合模式 + 透明排序」对照，含依据 / 代价 / 备选 A / B / C）→ 待规格门用户拍板（`0003-scope.md:116` 明文由试点 change 规格门拍板）。
- 反编译 / 抓帧证据与「合法自备、不绕过 EULA / DRM / 登录 / 反作弊」的逐条合规判定 → 待规格门 + 人工确认，不臆造。
- 行为规格文档字段 / 粒度 / 版本化与「仓库外证据目录 → 主仓库只读文档交接」格式 → 待规格门拍板。
- 视觉 oracle 验收判据（动作 / 时间点 / 机位如何固定、像素差异阈值、人眼判定与机器断言分界）→ 待规格门拍板，不臆造数值。
- 「Deep 模式 + Clean room 角色隔离」组合（两次运行？角色分工？是否覆盖本机经典客户端）→ 待规格门用户拍板；工具文档无 live 验证先例（缺口）。
- 规格门批准记录回填（四件套指纹 / `critic` 报告 / `approved_by` / `date`）→ 待用户拍板后由 leader / `spec-steward` 回填，本轮一律不填。
