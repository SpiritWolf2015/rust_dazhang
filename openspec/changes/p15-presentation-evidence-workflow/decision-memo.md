# 决策纪要 — `p15-presentation-evidence-workflow`

> 取料纪律：基于仓库 artifacts、本轮 `reference-researcher` 对照报告结论（已落 `design.md`「## 参考项目对照」）与用户明确拍板记录回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，不把 AI 事后归纳写成「用户已确认」。
> 合规边界：纯文档，不进状态哈希 / 快照、不含资产、不含本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md §6`）。
> 阶段：提案成文期（本轮只成文、不实施代码，apply 是后续独立会话）。用户 2026-11-25 明确拍板「已决定本项目引入反编译 War3 的工作流」并选择「ADR 修订 + 形态 A 试点」，本 change 承接 `ADR 0003` 第二次修订的独立试点 change，为**非玩法 / 呈现层证据工作流** change，不串味 p12 / p13 / p14。**批准状态为 `pending`、由人拍板、AI 未 self-approve**。
> 结构说明：`## 规格门` 置于末尾且只有本节以「规格门」开头（`tools/verify_all.py::memo_section_body()` 按前缀匹配），`status` 字段以本节为准；本纪要不在四件套指纹集合内（`tools/verify_all.py:175-195`），改它不触发 `CODEBUDDY.md §8` 规格解锁。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | 用户 2026-11-25 明确拍板「已决定引入反编译 War3 的工作流」并选择「ADR 修订 + 形态 A 试点」；同日 `ADR 0003` 第二次修订、`ADR 0001` 复核记录、`README.md` / `CONTRIBUTING.md` 合规措辞已由人拍板落地（工作区可见、未提交） | `docs/decisions/0003-scope.md:109-123`（第二次修订全文）/ `:111`（「用户 2026-11-25 明确…」）/ `:114`（形态 A 试点）/ `:116`（单特性由规格门拍板）；`docs/decisions/0001-license.md:77-83`（复核记录）；本轮 `git status --short` 输出（` M README.md` / ` M CONTRIBUTING.md` / ` M docs/decisions/0001-license.md` / ` M docs/decisions/0003-scope.md`）；用户本会话消息（**仓库文件外证据 / 人工确认项**） |
| C2 | 本 change 承接 `ADR 0003` 第二次修订补充复核点「决定引入反编译 / 抓帧 / REA 类逆向流程 → 单独立项、走规格门、由人拍板试点边界」，是独立试点 change、不并入 p12 / p13 / p14 | `docs/decisions/0003-scope.md:104-105`（第一次修订复核点补充）/ `:121-122`（第二次修订复核点补充 + 流程说明「p15 试点 change 的四件套成文属后续独立任务」）；本轮 `openspec new change p15-presentation-evidence-workflow` 与 `openspec status --change p15-presentation-evidence-workflow --json` 命令输出 |
| C3 | 长期目标为「像素级复刻 War3」，1.0 只是短期目标；第一次修订已撤回「目标错配」否决理由，呈现层行为对齐（绘制顺序 / 透明排序 / 遮挡剔除 / 粒子 / 动画混合 / UI 观感）与长期目标对位 | `docs/decisions/0003-scope.md:94-100`（「明确长期目标为像素级复刻 War3」修订记录）/ `:96`（撤回「目标错配」）/ `:99`（新增长期方向）/ `:102-104`（红线不变、既定路径）；用户 2026-11-25 拍板（仓库文件外证据 / 人工确认项） |
| C4 | 红线（ADR 0001 / 0002）：零资产入库、MPQ 派生数据不入库、不复制上游代码、不新增依赖、不登记 `NOTICE.md`、不绕 EULA / DRM / 登录 / 反作弊、仅用于合法自备经典客户端；反编译证据 MUST NOT 进权威状态 / 状态哈希 / 快照 / golden hash / determinism 验收 / CI 门禁；表现层只读派生、`w3-core` 零改动、不 bump 三版本号 | `docs/decisions/0001-license.md:80-82`（复核记录「合规边界不变并新增一条」）/ `:81`（反编译产物留本机不入库）/ `:82`（复核点补充）；`docs/decisions/0003-scope.md:117-120`（红线不变）/ `:102-103`（第一次修订红线）/ `:55`（合规）；`docs/decisions/0002-determinism.md:49-53`（§5 状态权威与表现边界）；`openspec/config.yaml:9-15,18-25` |
| C5 | 三版本号权威值当前均为 **9**；本 change 零新增权威状态，不 bump、不重跑 `cargo test -p w3-core --test determinism`（`CODEBUDDY.md §5` 形态①不适用） | `crates/w3-core/src/snapshot.rs:67`（`SNAPSHOT_VERSION: u32 = 9`）/ `:68`（`SIM_VERSION: u32 = 9`）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION: u8 = 9`）；`openspec/config.yaml:13` / `CODEBUDDY.md §5`；本轮只读 `grep -n` 命令输出 |
| C6 | 消费侧视觉 oracle 的既有能力：`w3-render` 离屏 `render_headless_rgba` / `render_headless_rgba_with_textures` 两变体、alpha blending 与 depth 状态 | `crates/w3-render/src/gpu/mod.rs:908-927`（两变体 + 离屏 RGBA 读回）/ `:593`（`wgpu::BlendState::ALPHA_BLENDING`）/ `:579-582`（depth write / compare）；`docs/decisions/0003-scope.md:104,115`（既定路径）；本轮只读 `sed -n` 命令输出 |
| C7 | WC3 资产与数据表仍以 `godot_warcraft3` 导出管线为准（MPQ→SLK / BLP / MDX→地图 JSON 三车道、gitignore 不入库）；`craft-anything` 格式清单**无** MPQ / BLP / MDX / SLK 专用解析器（缺口） | 矩阵条目 `docs/references/UPSTREAM.md:108`（T3）/ `:51`（`godot_warcraft3` 数据 / 资产格式规格）/ `:63-92`（导出数据不入库）；参考项目内路径 `godot_warcraft3@18b7a70:docs/data/PIPELINE.md:10-31,69-101,99-101,103-134,140-143,152-180,229-239`、`godot_warcraft3@18b7a70:docs/architecture/ASSET_LANES.md:43,45-49`；`craft-anything/crates/formats/README.md:1-75`（全文 grep 无 MPQ / BLP / MDX / SLK 匹配，本轮只读命令输出 `0`）；`docs/references/WC3-MELEE-RULES.md:7,126`（WC3 数值唯一来源为本机导出 SLK） |
| C8 | 参考矩阵命中判定：T5（场景 / 相机 / HUD / 3D 呈现）**主命中**；T1 / T3 / T7 **可选交叉（仅红线）**；T2 / T4 / T6 **不命中**。`reference-researcher` 对照报告**本轮已产出**、结论落 `design.md`「## 参考项目对照」 | 矩阵条目 `docs/references/UPSTREAM.md:104-114`（T5 在 `:110`、交叉注记 `:114`）/ `:106`（T1）/ `:108`（T3）/ `:112`（T7）；`design.md`「## 参考项目对照」逐条；本轮 `reference-researcher` 对照报告（仓库文件外证据 / 人工确认项，锚已只读抽查打开）；T1 承重锚 `vera20k@305ea3a75:src/net/lockstep.rs:1-14`（`:10` "Bytes stay authoritative there; decoding happens only after the dispatcher admits an event."，本轮 `git -C vera20k show 305ea3a75:src/net/lockstep.rs` 命令输出核对） |
| C9 | 五个只读参考项目仍是 `vera20k` / `godot_warcraft3` / `rts_kernel_cs` / `WarsmashModEngine` / `ConcordRTS`；`craft-anything`（本机外部工具）**MUST NOT 登记为第六个只读参考项目**（沿用 p14 口径） | `docs/references/UPSTREAM.md:9-15`（固定参考版本表，五个）/ `:96,100`（合规与矩阵「五个只读、不复制代码」）；`CODEBUDDY.md §1`；`openspec/changes/p14-ai-workflow-guardrails/design.md:112`（不登记第六个参考项目）/ `decision-memo.md:16`（C4）；`craft-anything/README.md:143-147,240-248`（本轮只读 `sed -n` 输出，early alpha）；用户本会话任务边界（仓库文件外证据 / 人工确认项） |
| C10 | T5 主命中承重锚（呈现层保真验收方法论）：固定种子 / 相机 / 背景 / 时间步、以模拟帧编号命名截图；「Headless 检查不证明 GPU 画面正确；截图验证也不等于原作保真验收。正式验收需加入相同动作、时间点、机位的原作参考。」；先校对原作轮廓 / 运动 / 时序；跨透明排序仍受 Godot 限制；PE2 五类混合分开映射、排序读 SortFarZ 与 priority plane；金样 `FireBallMissile` | `godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:48`（固定种子 / 相机 / 背景 / 时间步、图片名 020/060/120 为模拟帧编号）/ `:50`（上引原文）/ `:33`（先校对原作的轮廓、运动和时序）/ `:59`（跨透明物体的排序仍受 Godot 限制）/ `:10`（Blend / Additive / Modulate / Modulate2x / AlphaKey 分开映射）/ `:14`（排序读 SortFarZ 与 priority plane）；`FX_SAMPLE_ACCEPTANCE.md:3,44,72`、`PE2_GODOT.md:1-34,36`（金样 `FireBallMissile`）、`MDX_SKINNING_GODOT.md:11-28`、`docs/design/game/HUD.md:13,149`、`docs/data/PIPELINE.md:99-101`（FilterMode 数值）；本轮只读 `sed -n` 命令输出逐条核对 |
| C11 | 许可证 / 红线边界：AGPLv3 只借语义不复制代码、不引入 Java / LibGDX；`vera20k` Section 7 附加条款不触及（只读不复制）；T5 MUST NOT 以 `ConcordRTS` 为参考、其 RA2/YR 专属格式 / 规则 / 数值绝不作 WC3 事实来源、README「逐字节一致」自述不作本项目已验证事实；`rts_kernel_cs` 的 `double` 路线不采用；原创三族只借结构不借单位名 / 数值；`HUD.md:13`「不复刻 WC3 Console 贴图外壳」不得当经典 UI 观感事实；AlphaKey 阈值 / FilterMode 数值只作 `[待核]` 候选观察字段 | `WarsmashModEngine/LICENSE:1`、`WarsmashModEngine/README.md:130`（AGPLv3 核对，T4 不命中）；`docs/references/UPSTREAM.md:114`（T3 / T4 / T5 MUST NOT 以 `ConcordRTS` 为参考）/ `:35`（README 自述不作已验证事实）/ `:52`（`double` 路线不采用）/ `:110`（`FACTION_UNIT_ROSTERS.md` 原创三族只借结构与批次判据）；`ConcordRTS@32ab245:README.md:3-5,11-13,21-27`、`API.md:296-312`、`src/core/determinism/RuntimeAuthorityHashPrimitives.h:1-10`（T7 仅方法论红线，本轮只读打开）；`vera20k@305ea3a75:docs/Networking.md:3-6,599-600,615`；`godot_warcraft3@18b7a70:docs/design/game/HUD.md:13`；`docs/references/WC3-MELEE-RULES.md:7,126` |
| C12 | 决策纪要三必填小节 = 承重约束 / 拍板决策 / 待确认项；`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed；`rule_doc_sync_check()` 对 pending / 非玩法豁免；四件套指纹不含 `decision-memo.md`、不含主 `openspec/specs/**` | `tools/verify_all.py:29`（`REQUIRED_MEMO_SECTIONS`）/ `:143-172`（`decision_memo_check()`）/ `:175-195`（`spec_artifact_fingerprint()` 的 `paths` 与复选框归一化）/ `:207-253`（`spec_gate_check()`）/ `:256-294`（`rule_doc_sync_check()`）/ `:38-40`（`RULE_REF_PATTERN`）；`openspec/decision-memo.template.md:14,55-64`；p12 `decision-memo.md:69-73` / p13 `:80-84` / p14 `:79-87` 的「## 规格门」体例 |
| C13 | 规则文档同步是正式结构门禁（§14 第 5 条），`rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 生效；pending / 非玩法 change 豁免 | `docs/references/WC3-MELEE-RULES.md:424`（§14 第 5 条）/ `:426-437`（§15 变更记录）；`tools/verify_all.py:256-294` / `:38-40`；`openspec/decision-memo.template.md:48-53`；`CODEBUDDY.md §9`（规则文档同步正式规则） |
| C14 | 编码基线 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格；中文文档；git 提交信息中文；AI 不得 push / fetch / pull / clone；零资产红线；不写本机真实绝对路径 / 资产 / 凭证 | `CODEBUDDY.md §2` / §3 / §4 / §6；`openspec/config.yaml:34-40`；`tools/check_encoding.py:15-25,69-87`；`.codebuddy/agents/critic.md:21,26` |
| C15 | 本轮只成文、不实施代码；可写范围仅 `openspec/changes/p15-presentation-evidence-workflow/`；不碰 p12 / p13 / p14 / 归档件 / 主 spec / `docs/**` / `README.md` / `CONTRIBUTING.md` / `crates/**` / `tools/**`；不追溯改写 P11 归档件；不改 p14「明确不纳入」边界；不 commit / push / archive / 勾复选框 / self-approve | 用户本会话任务边界（仓库文件外证据 / 人工确认项）；本轮 `git status --short` 输出（p15 初始仅 `.openspec.yaml`、p12/p13/p14 为既有 change）；`openspec status --change p15-presentation-evidence-workflow --json` 输出；`openspec/changes/archive/2026-11-25-p11-ability-system/decision-memo.md`（已 `status: approved` 锁定，改即触发 §8 规格解锁） |

## 拍板决策（含被否备选与理由）

- **P1 新建独立试点 change `p15-presentation-evidence-workflow` 并落成完整四件套 + `decision-memo.md`，承接 `ADR 0003` 第二次修订的「形态 A 试点」，Scope 严格限于「呈现层证据流程规范 + 单个特性的试点」，本轮只成文不实施代码**。锚：用户 2026-11-25 明确拍板「已决定本项目引入反编译 War3 的工作流」并选择「ADR 修订 + 形态 A 试点」（用户本会话消息 / leader 转达，**仓库文件外证据 / 人工确认项**）+ `docs/decisions/0003-scope.md:109-123`（第二次修订，仓库内 artifacts 明确记载的用户拍板）/ `:121-122`（复核点补充 + 「p15 试点 change 的四件套成文属后续独立任务」）。被否：① 把试点并入 p12 / p13 / p14 或直接改已归档 P11 四件套（理由：用户选择「单独立项」，且本 change 为非玩法 / 呈现层证据工作流 change，与 p12 文档订正 / p13 玩法全量 / p14 门禁治理的 owner 与门禁边界不同，不串味；P11 已 `status: approved` 锁定、改即触发 §8 规格解锁，C15）；② 本轮直接改 `README.md` / `CONTRIBUTING.md` / ADR 正文 / `crates/**` / `tools/**` 落地工作流（理由：同批合规措辞已由人拍板落地且在工作区未提交，本轮只成文、可写范围仅 p15 目录，C1 / C15）；③ 把 `craft-anything` 登记为第六个只读参考项目或引入其代码 / 依赖 / 技术栈（理由：`CODEBUDDY.md §1` / `UPSTREAM.md:9-15` 参考项目只读五个、ADR 0001 零资产 / 形态 A 本机外部工具不入库，C9）；④ 一次性推广到全部呈现特性或固化模板 / 规则（理由：用户选「形态 A 试点」、按「工作流改进先试点再固化」先验证闭环，`:116` 明文「本修订不固化任何模板 / 规则」）。

> 措辞纪律：P1 为本轮**唯一**已拍板项（证据锚 = 用户本会话消息 / leader 转达这一仓库文件外证据 / 人工确认项，及 `ADR 0003` 第二次修订这一仓库内 artifacts 明确记载的用户拍板），MUST NOT 写成 AI 自定 / AI 推荐 / AI 已确认 / 沉默授权。**用户本轮只拍板了「引入工作流 + ADR 修订 + 形态 A 试点 + 单独立项」**；其余范围 / 方案 / 试点特性 / 5 项参考缺口 / 合规逐条判定 / 交接格式 / 验收判据 / Deep + Clean room 组合 / 规格门批准记录一律进「待确认项」，不臆造为已定案。特别是：MUST NOT 把「引入工作流」写成「已引入代码依赖 / 已放宽红线 / 已过规格门 / 已获暴雪授权」；MUST NOT 把 `craft-anything` early alpha / Clean room 不是 OS 沙箱 / Deep + Clean room 组合无 live 验证先例写成「已验证可用」；MUST NOT 把 `reference-researcher` 对照报告里的「参考缺口」写成已有答案。

## 被否方向台账

> 编号说明：本小节 `R1`–`R7` 是「被否方向」台账编号（沿用 p12 / p13 / p14 `decision-memo.md`「## 被否方向台账」体例），**不是** `docs/references/WC3-MELEE-RULES.md` 的玩法规则编号；本 change 为非玩法 change，`design.md` 不承接任何 `R-x.y` 实现需求，proposal / specs / tasks 中出现的 `R-x.y` 均为「不引用 / 不实现任何玩法规则」的否定式声明，不作需求依据。

- **R1 把反编译产物（伪 C / 调用图 / 行为规格 / 抓帧证据）或 MPQ 派生 JSON / PNG / GLB 写进仓库、或把 `craft-anything` 复制 / 加为 crate 依赖 / 登记 `NOTICE.md`** → C4 / C9 / ADR 0001 复核记录 `0001-license.md:80-82`；形态 A 明文「不入库 / 不复制代码 / 不作为 crate 依赖 / 不登记 `NOTICE.md`」（`0003-scope.md:114`）。若前提后续变化 MUST 整体重估，不沿用旧否决。
- **R2 把视觉 oracle / 行为规格当 ADR 0002 口径的业务逻辑 oracle，据此改 `w3-core` 权威状态 / 状态哈希 / 快照 / golden hash / determinism 门禁 / CI，或 bump 三版本号** → C4 / C5 / `0003-scope.md:103,119`；反编译产出的是「视觉 oracle」不是「可冻结、机器可判、可进 CI 的业务逻辑 oracle」；`CODEBUDDY.md §5` 形态①不适用、不臆造内核改动。
- **R3 用 `craft-anything` 替代 `godot_warcraft3` 导出管线取 WC3 资产 / 数据表，或把 Godot 的 AlphaKey 阈值 / FilterMode 数值冻结为 WC3 权威数值** → C7 / C11；`craft-anything/crates/formats/README.md:1-75` 无 MPQ / BLP / MDX / SLK 解析器（缺口），WC3 数值唯一来源为本机导出 SLK（`WC3-MELEE-RULES.md:7,126`）、数值只作 `[待核]` 候选观察字段。
- **R4 把 `craft-anything` 登记为第六个只读参考项目，或引入 AGPLv3 Java / LibGDX 代码、`vera20k` Section 7 附加条款、`ConcordRTS` RA2/YR 格式 / 规则 / 数值、`rts_kernel_cs` 的 `double` 路线** → C9 / C11 / `UPSTREAM.md:96,100,114`；五个只读参考项目不变、AGPLv3 只借语义、T5 MUST NOT 以 `ConcordRTS` 为参考、README「逐字节一致」自述不作已验证事实。
- **R5 本轮替用户把试点特性 / 5 项参考缺口 / 合规逐条判定 / 交接格式 / 验收判据 / Deep + Clean room 组合拍成定案，或 AI self-approve 规格门** → 用户 2026-11-25 只拍板「引入工作流 + 形态 A 试点 + 单独立项」（C1 / P1），`0003-scope.md:116` 明文「具体特性选择由后续试点 change 的规格门拍板」；无证据项 MUST NOT 写「用户已确认」、不拿沉默当授权。
- **R6 自填 `status: approved` + 指纹 / `approved_by` / `date`，或写省略说明求门禁变绿** → C12 / `CODEBUDDY.md §8` / §9 规格门由人拍板、AI 不得 self-approve；本 change 非「目标与方向已清楚的小改动」，豁免会违反 fail-closed 信号（待确认项与 5 项参考缺口均未收敛）。
- **R7 顺手 bump 三版本号 / 改 ADR 正文 / 接 CI / 改业务规则 / 改 p14「明确不纳入」边界 / 追溯改写 P11 归档件 / 扩大试点到多特性 / 纳入真窗口人工验收或 golden hash 双平台为已完成** → C1 / C4 / C5 / C15、超 Scope、用户明确本轮只成文；真窗口 8.x / golden hash v9 双平台 / CI 双平台一律「待人工 / 待 VM」，不冒充通过。

## 待确认项（如实标注，不冒充已拍板）

> 以下全部由规格门用户拍板（不拿沉默当授权）；无证据项一律标「待确认」，MUST NOT 写成「用户已确认」。
>
> **规格门拍板结论（2026-11-25）**：④ 试点特性已由用户经 `AskUserQuestion` 拍板采用下述「推荐」项（单个 PE2 / 飞弹特效的「混合模式 + 透明排序」视觉 oracle 对照），推荐 / 依据 / 代价 / 备选内容作为拍板依据保留；① 反编译合规逐条判定 / ② 行为规格文档格式 / ③ 视觉 oracle 验收判据 / ⑤ Deep + Clean room 组合及其余参考缺口，按用户拍板**延后到 apply 阶段由人逐条拍板**，不冒充已确认、不预设定案。批准状态一律以「## 规格门」小节 `status: approved` 为准。
>
> **apply 前置收敛结论（2026-11-25，承接上段「延后到 apply 阶段由人逐条拍板」；用户本会话经 `AskUserQuestion` 逐条明确拍板，属仓库文件外证据 / 人工确认项，非 AI 自定、非沉默授权）**：本收敛会话一次拍板 ①②③⑤ 四项，结论如下。**前置收敛 ≠ apply 已启动 ≠ 工具已就绪**——试点仍未开始：`craft-anything-cli` 未安装、仓库外证据目录 / 合法自备经典客户端 / 原作参考帧均由用户本机准备；本轮不装工具、不碰原版客户端、不实施任何代码。
> - **① 合规逐条判定 → 用户本人「确认，按红线执行」**：仅用合法自备的经典客户端、不绕过 EULA / DRM / 登录 / 反作弊；伪 C / 调用图 / 行为规格 / 抓帧证据全留仓库外本机证据目录、不入库不分发；Clean room 不是 OS 沙箱（`craft-anything/README.md:143-147`），故额外靠人工逐条把关。依据 `ADR 0001` 复核记录 `0001-license.md:80-82` / `ADR 0003:118` / `README.md:29` / `CONTRIBUTING.md:11`。**AI 不代判合规、不写成法律意见、不冒充已获暴雪授权**；后续每次取证仍须用户本人逐条确认，不因本次授权自动放行。
> - **② 行为规格格式 / 证据目录 / 交接 / owner →「轻量文档 + 固定字段」**：中文 Markdown 固定字段（样本标识 / 动作·时间点·机位 / 混合模式与排序观察 / 判定 + `ev_` evidence id / 独立重审锚 / `[待核]` 数值 / 已知限制）；仓库外单一证据目录；主仓库只放一份**只读交接摘要**，原始伪 C / 抓帧产物不入库；owner 分工：apply 由 leader 编排、**生产侧（操作原版程序）由用户本人在本机执行**、消费侧 `crates/w3-render` 只读复核归 `client-owner`，AI 不碰原版客户端。代价：无机械 schema 校验，靠文档纪律；`CODEBUDDY.md §5` 形态①不适用、不臆造内核改动。
> - **③ 视觉 oracle 验收判据 →「机器判出图·人眼判保真」**：固定机位 / 动作 / 时间点、以模拟帧编号命名截图；离屏 `render_headless_rgba` / `render_headless_rgba_with_textures` 只机器断言「链路出图 / 确定性」，**不设像素差异阈值**；原作保真由真窗口人工对照「相同动作 / 时间点 / 机位的原作参考帧」裁决。依据 `FX_BAKE_FIDELITY.md:48,50,59` 与 `crates/w3-render/src/gpu/mod.rs:908-927`（无 GPU 时 `GpuUnavailable` 跳过、不冒充通过）。代价：结论带人工判定色彩、不进 CI；判据收敛到单特效 / 固定机位，不宣称解决全局透明排序。
> - **⑤ Deep + Clean room 组合 →「两次运行 + 双角色」**：生产侧一次 Deep 运行只产行为规格（操作原版程序的 agent 不写实现代码），消费侧另一次不接触原版安装、只按规格改 `w3-render`；两次之间只靠文档交接（生产侧与消费侧只通过只读文档交接）。代价：工具文档无 live 验证先例（`craft-anything/docs/claude-code.md:63`、`README.md:240-248`），需先在单特性小范围人工确认可行性；隔离靠角色纪律而非系统沙箱。
> - **其余参考缺口（非拍板项，已知限制沿用登记）**：`craft-anything/crates/formats/README.md:1-75` 无 MPQ / BLP / MDX / SLK 专用解析器、资产侧仍靠 `godot_warcraft3` 导出管线；`HUD.md:13` 不得当经典 UI 观感事实；`FACTION_UNIT_ROSTERS.md` 原创三族只借结构。不臆造答案。
> - **不变项**：④ 试点特性沿用规格门拍板；真窗口 8.x 人工清单 / golden hash v9 双平台 / CI 双平台一律「待人工 / 待 VM」，不冒充通过；不得据本收敛声称跨平台确定性已达成或工具已验证可用。`## 规格门` 的 `status: approved` 与四件套指纹 `d5ab6b5e…` 不变；本次只改 `decision-memo.md`（不在四件套指纹集合内），**不解锁规格、不改四件套正文、不勾任何复选框、不 commit / push / archive**。提案阶段「保持 pending / 由 leader 回填 / 未自行收敛」等措辞为当时的流程约束记录（沿用 P11 归档件体例），收敛状态一律以本段为准。

- **④ 试点特性（推荐 + 依据 + 代价 + 备选，AI 不 self-approve）**：
  - **推荐**：单个 PE2 / 飞弹特效的「混合模式 + 透明排序」视觉 oracle 对照（样本 `FireBallMissile` 或 `PriestMissile` 的 `Stand` / `Death` 两态，固定机位 / 动作 / 时间点）。
  - **依据**：`godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:10`（五类混合分开映射）/ `:14`（排序读 SortFarZ 与 priority plane）/ `:48`（固定种子 / 相机 / 背景 / 时间步、模拟帧编号命名截图）/ `:50`（正式验收需相同动作 / 时间点 / 机位的原作参考）/ `:59`（跨透明排序仍受 Godot 限制）；`docs/data/PIPELINE.md:99-101`（FilterMode 候选观察字段）；`FX_SAMPLE_ACCEPTANCE.md:28-34,44`（`PriestMissile` / `FireBallMissile` 的 `Stand` / `Death` 两态）；`PE2_GODOT.md:14-34,36`（特效四类分开、金样 `FireBallMissile`）；本项目 `crates/w3-render/src/gpu/mod.rs:908-927` 离屏能力、`ADR 0003:104,115` 既定路径（C6 / C10）。
  - **代价**：需合法自备经典客户端 + 相同动作 / 时间点 / 机位的原作参考帧；Clean room 不是 OS 沙箱（`craft-anything/README.md:143-147`）；工具 early alpha（`README.md:240-248`）；判据 MUST 收敛到「单个特效 / 固定机位」，**不宣称解决全局透明排序**（`FX_BAKE_FIDELITY.md:59`）。
  - **备选 A**：MDX 动画播放 / 混合（`MDX_SKINNING_GODOT.md:11-28`）——本项目动画混合管线**未核实**，可能超试点边界，待规格门拍板前不臆造能力。
  - **备选 B**：水面 / 地形绘制顺序 + 遮挡剔除——本项目离屏场景是否覆盖水面**未核实**，易牵动 `w3-content`，待规格门拍板。
  - **备选 C（不推荐）**：HUD / UI 观感——`godot_warcraft3@18b7a70:docs/design/game/HUD.md:13` 自述「不复刻 WC3 Console 贴图外壳」，与 `WC3-MELEE-RULES.md` N5 不做 JASS / FDF VM 冲突（`UPSTREAM.md:109` T4 边界），不作经典 UI 观感事实来源。
- **① 反编译 / 抓帧证据与「合法自备、不绕过 EULA / DRM / 登录 / 反作弊」的逐条合规判定** → 待规格门用户拍板 + 人工确认；工具 / 用法 / 产物三层合规（`0001-license.md:81`）如何逐条举证无现成先例，不臆造答案。
- **② 行为规格文档的字段 / 粒度 / 版本化与「仓库外本机证据目录 → 主仓库只读文档交接」格式** → 待规格门用户拍板；生产侧与消费侧只通过只读文档交接为本项目自定约定，字段与版本化未定义，不臆造。
- **③ 视觉 oracle 验收判据（动作 / 时间点 / 机位如何固定、像素差异阈值、人眼判定与机器断言分界）** → 待规格门用户拍板；`FX_BAKE_FIDELITY.md:50` 明示「Headless 检查不证明 GPU 画面正确；截图验证也不等于原作保真验收」，离屏断言与真窗口人眼判定的分界、像素阈值均无现成数值，不臆造。
- **⑤ 「Deep 模式 + Clean room 角色隔离」如何组合落地（两次运行？角色分工？）、是否覆盖本机经典客户端** → 待规格门用户拍板；`craft-anything/README.md:143-147`（Clean room「不是 OS 沙箱，不隔离你的计算机」）/ `:240-248`（early alpha、behaviour checks for apps and games「partly there」、部分引擎工具只测过 stand-ins）/ `docs/claude-code.md:63`（「Not yet checked live against Claude Code or an ACP agent」）——工具文档**无 live 验证先例**，不臆造组合方式（C10 / 已知限制）。
- **其余参考缺口如实登记、不臆造答案**：`craft-anything/crates/formats/README.md:1-75` 格式清单无 MPQ / BLP / MDX / SLK 专用解析器（本轮全文 grep 无匹配），资产侧仍靠既有 `godot_warcraft3` 导出管线（C7）；`HUD.md:13` 不得当经典 UI 观感事实；`FACTION_UNIT_ROSTERS.md` 原创三族只借结构、不借单位名与数值（C11）。
- **规格门批准记录回填** → 「## 规格门」保持 `status: pending`；四件套指纹（`python tools/verify_all.py --fingerprint p15-presentation-evidence-workflow`）、`critic` 规格门报告、批准人 / 日期由 leader 在用户拍板后回填，本轮不碰、AI 未 self-approve（C12）。
- **`reference-researcher` 对照报告** → **本轮已产出**、结论落在 `design.md`「## 参考项目对照」（T5 主命中、T1 / T3 / T7 可选交叉仅红线、T2 / T4 / T6 不命中；T1 承重锚 `vera20k@305ea3a75` 固定提交双标），锚已只读抽查打开；报告中的「参考缺口」一律回到用户拍板、不臆造（C8 / C10）。
- **真窗口 8.x / golden hash v9 双平台 / CI 双平台** → 一律「待人工 / 待 VM」，不冒充通过；`ADR 0002` 跨平台确定性（macOS / ARM64 / 其它 rustc）仍属待验证项，**不得据本对照报告声称跨平台确定性已达成**（C4 / `0002-determinism.md:118` 复核记录口径）。
- **`spec_gate_check()` fail-closed** → `invalid spec gate status 'pending'` 属预期信号，不绕过、不修复、不冒充放行（C12）。
- **apply 阶段执行时机与 owner 分工** → 一个 change 一个 owner、不同时改多 owner 文件；由 leader 安排后续独立会话；本轮不实施代码、不勾任一复选框（C15）。
- **验收线口径** → `openspec validate p15-presentation-evidence-workflow --strict` / `python tools/check_encoding.py` 本轮已跑通的部分如实登记；完整 `python tools/verify_all.py` 由 leader 跑、本轮不跑；`cargo test --workspace` / CI 双平台属 apply 阶段与人工 / VM 项，不预先声称通过（C14）。

## 已知限制（如实登记，不冒充已解决、不扩范围）

- **`craft-anything` 格式缺口**：`crates/formats/README.md:1-75` 格式清单无 MPQ / BLP / MDX / SLK 专用解析器（本轮全文 grep 无匹配），资产侧仍靠既有 `godot_warcraft3` 导出管线（C7）；外部工具不能替代资产 / 数据表来源。
- **Clean room 不是 OS 沙箱**：`craft-anything/README.md:143-147` 原文「Clean room is a rule about what Craft Anything offers and refuses. It is not an operating-system sandbox: it doesn't isolate your computer.」；工具自己不隔离本机，合规姿态仍靠 ADR 0001 / 0002 红线与人工逐条判定（C4）。
- **early alpha 与无 live 验证先例**：`README.md:240-248` 自述 early alpha、behaviour checks for apps and games「partly there」、部分引擎工具只测过 stand-ins；`docs/claude-code.md:63`「Not yet checked live against Claude Code or an ACP agent」。「Deep 模式 + Clean room 角色隔离」组合、是否覆盖本机经典客户端无 live 验证先例，属规格门待确认项（C10 / 待确认项⑤）。
- **交接与证据纪律为本项目自定的工程治理约定**：生产侧与消费侧只通过只读文档交接、判定须经独立重审（判定与产物同提交 + 独立重审），不复制任何外部代码、不引入外部技术栈、不作 WC3 玩法 / 格式 / 数值事实来源、不登记为第六个参考项目（C9）。
- **试点特性 / 5 项参考缺口未定案**：推荐方案只是推荐，合规逐条判定 / 行为规格字段与交接格式 / 视觉 oracle 判据 / Deep + Clean room 组合 / 特性选择全部由规格门用户拍板，不臆造（待确认项）；本项目动画混合管线、离屏场景是否覆盖水面**未核实**（备选 A / B）。
- **视觉 oracle 不等于原作保真验收**：`FX_BAKE_FIDELITY.md:50` 明示 Headless 检查不证明 GPU 画面正确、截图验证也不等于原作保真验收；`w3-render` 离屏 + ViewImage 是机器侧自测，真窗口人工验收与相同动作 / 时间点 / 机位的原作参考帧属人工项，不标通过（C6 / C10）。
- **零确定性门禁**：本 change 不触 `w3-core` 确定性代码，不重跑 `cargo test -p w3-core --test determinism`、`CODEBUDDY.md §5` 硬门禁与形态①反向验证不适用，如实登记、不冒充全量门禁（C4 / C5）。
- **真窗口 / 双平台 / CI 待人工或 VM**：真窗口 8.x 人工清单、golden hash v9 双平台逐字节比对（Debian 12 / macOS / ARM64 / 其它 rustc）、CI 双平台 `verify_all.py` 一律「待人工 / 待 VM」，不冒充通过、不据本报告声称跨平台确定性已达成（C4 / 待确认项）。
- **本轮 `git diff` 边界**：`-- openspec/specs/` / `-- docs/` / `-- crates/` / `-- tools/` / `-- README.md` / `-- CONTRIBUTING.md` 为空；只新增 p15 目录内文件；主 specs 是否变化以 leader 归档合并时复核（C15 / `proposal.md` Capabilities 注释）。
- **`spec_gate_check()` fail-closed 与「## 规格门」pending**：`invalid spec gate status 'pending'` 属预期信号；本纪要「## 规格门」严格只有 `- status: pending` + 一段说明，无 `fingerprint` / `approved_by` / `date` / `reason` 字段行、不写省略说明、不 self-approve（C12）。

## 停止条件（追加阶段 / 归档前）

1. 承重约束 C1–C15 逐条仍有证据锚、无一条被本轮改动证伪。
2. 待确认项已清空或显式豁免（本轮为提案成文期，试点特性 / 5 项参考缺口 / 合规判定 / 交接格式 / 验收判据 / Deep + Clean room 组合 / 批准记录由规格门用户拍板，未自行收敛）。
3. 不以「预设问题问完」「问了 N 轮」当收敛；批准由用户拍板、不拿沉默当授权。
4. 「## 规格门」保持 `status: pending`、不 self-approve、不写省略说明、不填指纹 / `approved_by` / `date` / `reason`。
5. 不把「引入工作流」写成已引入代码依赖 / 已放宽红线 / 已过规格门 / 已获暴雪授权；不把参考缺口写成已有答案；不把规则编号引用冒充规则已实现；真窗口 / 双平台人工清单未标通过。

## 规则文档同步

- 结论：**无需同步**。依据 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条：本 change 是**非玩法 / 呈现层证据工作流 change**，不实现任何玩法、不触及任何 `R-x.y` 实现状态、不改 §13.7 现状、不改附录「规则 → 实现覆盖对照」任何行、不改 §15 变更记录、不改参考项目数量（仍五个）。本 change 只新增「呈现层证据流程」规范与单特性试点（生产侧行为规格 / 只读文档交接 / 消费侧视觉 oracle），均非玩法规则实现状态变更；proposal / specs / tasks 不承接任何 `R-x.y` 实现需求（C8 / C13）。
- MUST NOT 把「引用某编号 / 某 §」冒充「规则已实现」（`WC3-MELEE-RULES.md:393-395` 三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。AlphaKey 阈值 / FilterMode 等参考项目数值只作 `[待核]` 候选观察字段，WC3 数值唯一来源为本机导出 SLK（`WC3-MELEE-RULES.md:7,126`），不冻结进权威定义（C11）。真窗口 8.x 人工清单、golden hash 双平台、CI 双平台不在本 change 范围、不标通过。
- 结构门禁：`tools/verify_all.py::rule_doc_sync_check()` 仅对过规格门 `status: approved` 且 `design.md` 含规则编号引用的玩法 change 校验本小节存在且非空；本 change `status: pending` 且非玩法（`design.md` 不承接任何 `R-x.y` 实现），豁免。本小节按 §14 第 5 条如实给出「无需同步」结论、保持非空；`decision-memo.md` 不在四件套指纹集合内（C12），补该小节不触发 §8 规格解锁。结论语义真实性由 `critic` 清单第 13 项复核，机械门禁只校验结构、不冒充语义验证。

## apply 自证登记（2026-11-26，AI 侧第 1 段）

> 规格门 `approved`、四件套指纹 `d5ab6b5e…` 锁定后的第一个 apply 会话：用户 2026-11-26 经 `AskUserQuestion` 拍板「先做 AI 侧自证登记」（仓库文件外证据 / 人工确认项，非 AI 自定、非沉默授权）。生产侧 clean-room 取证（2.x）、仓库外文档交接（3.x）、原作参考帧与真窗口人眼验收（4.1）按 ②⑤ 角色隔离归**用户本人本机**，AI 不碰原版客户端、不代判合规、不冒充这些步骤已发生。本小节不在四件套指纹集合内（`tools/verify_all.py:175-195`），追加不触发 `CODEBUDDY.md §8` 规格解锁。

- **已勾 9 项（AI 侧可诚实自证，逐项证据锚）**：
  - `1.1` / `1.2`：核对 `decision-memo.md`「待确认项」④ 规格门拍板结论（单 PE2 / 飞弹特效「混合模式 + 透明排序」视觉 oracle）与 ①②③⑤「apply 前置收敛结论」段（用户逐条 `AskUserQuestion` 拍板）均已记录；判据收敛到「单个特效 / 固定机位」、不宣称解决全局透明排序；非重新拍板、非 AI 自定。
  - `4.2`：明确「Headless 检查不证明 GPU 画面正确；截图验证也不等于原作保真验收」（`FX_BAKE_FIDELITY.md:50,59`）；真窗口 8.x 人工清单与「相同动作 / 时间点 / 机位的原作参考帧」保持「未执行 / 待人工」、不标通过；试点结论不写成「GPU 画面正确 / 原作保真验收已通过」。
  - `4.3`：本会话 `git diff --stat -- crates/w3-core crates/w3-net crates/w3-render crates/w3-client crates/w3-content` 为空；`SNAPSHOT_VERSION` / `SIM_VERSION`（`crates/w3-core/src/snapshot.rs:67-68`）与 `PROTOCOL_VERSION`（`crates/w3-net/src/packet.rs:50`）实读仍 == 9；未接 CI、未改 `tools/**`。
  - `5.1`：`openspec validate p15-presentation-evidence-workflow --strict` → `is valid`；`python tools/check_encoding.py` → `Encoding check OK: 468 text files checked`；`decision_memo_check()` / `spec_gate_check()` / `rule_doc_sync_check()` 实跑均 True。成文期措辞「`spec_gate_check()` 对 pending 报 fail-closed」为当时流程约束记录，规格门现 `approved` 且指纹一致故该函数现为 True，不冒充统一门禁代表试点闭环。
  - `5.2`：「## 规则文档同步」小节非空、结论「无需同步」真实（非玩法 / 不承接 `R-x.y` / 不改 §13.7 · §15 · 附录覆盖对照 / 参考项目仍五个）；`git diff -- docs/references/WC3-MELEE-RULES.md`（含于上面边界 diff）为空；`rule_doc_sync_check()` 对非玩法 change 豁免。
  - `5.3`：`design.md`「## 参考项目对照」逐条登记 T5 主命中 / T1 · T3 · T7 可选交叉（仅红线）/ T2 · T4 · T6 不命中；T1 承重锚 `vera20k@305ea3a75:src/net/lockstep.rs` 经只读 `git show 305ea3a75:src/net/lockstep.rs` 实读，引用句落在第 10–11 行（`:10` 为范围锚内近似行、该句跨两行，如实标注不冒充逐字单行命中）、保持固定提交双标未退化成工作树 HEAD；五个只读参考项目不变、`craft-anything` 不登记为第六个；AGPLv3 / Section 7 / `ConcordRTS` RA2/YR 边界逐条成立。
  - `5.4`：三必填小节（承重约束 / 拍板决策 / 待确认项）非空、每条承重约束带 `file:line` / commit / 命令输出证据锚、「## 已知限制」「## 规则文档同步」非空；「## 规格门」以 `approved` 小节为控制值（成文期「仅 pending」措辞为当时约束记录）；`decision_memo_check()` 实跑 True；「## 规格门」置于末尾且只有本节以「规格门」开头、`decision-memo.md` 不在四件套指纹集合内。
  - `5.5`：`git status --short` 仅本 change 目录内 `tasks.md` / `decision-memo.md` 被改；对 `openspec/specs/` · `openspec/changes/archive/` · p12 / p13 / p14 · `docs/` · `crates/**` · `tools/` · `README.md` · `CONTRIBUTING.md` 的 `git diff --stat` 均为空；未 `git add` / `commit` / `push` / `fetch` / `pull` / `clone` / `archive`；不追溯改写 P11 归档件、不改 p14「明确不纳入」边界。
- **保持未勾 6 项（如实登记，非 AI 可做）**：`2.1`（配置 `craft-anything` + 逐条合规核对）、`2.2`（`--deep` 产行为规格文档）、`2.3`（AlphaKey / FilterMode 登记为 `[待核]` 候选观察字段）、`3.1`（仓库外证据目录 → 主仓库只读文档交接）、`3.2`（判定与产物同批 + 独立重审）、`4.1`（离屏 `render_headless_rgba` / `_with_textures` + ViewImage 对照单特性、需相同动作 / 时间点 / 机位的原作参考帧）。按 ②⑤ 角色隔离（「apply 前置收敛结论」段 ②⑤）归用户本人本机生产侧 / 仓库外证据目录 / 真窗口人眼验收；AI 不碰原版客户端、不开 `--deep` 取证、不代判合规、不写法律意见、不冒充上述步骤已发生。消费侧 `4.1` 的「离屏出图 / 确定性」机器断言依赖生产侧行为规格与 `[待核]` 样本，本段亦未执行、一并保持未勾。
- **诚实边界（不冒充完成）**：本段只完成「AI 侧可诚实自证」的 9 项，**不是试点闭环**；三个门禁函数 True / `validate` valid / 编码 OK 只证明规格与纪要结构一致，不证明 clean-room 取证已发生、行为规格已交接、视觉 oracle 已对照、原作保真已验收。真窗口 8.x / golden hash v9 双平台逐字节比对 / CI 双平台一律「待人工 / 待 VM」，不标通过；不据此声称跨平台确定性已达成或工具已验证可用。
- **成文期措辞字面失效（沿用 P11 归档件 / p16 residual R1 体例，不订正正文）**：`tasks.md` 头部、`5.5`、Workflow follow-up 的「本轮只成文、不实施代码」「全部复选框保持未勾」「apply 是后续独立会话」「`## 规格门` 保持 pending、不填指纹 / `approved_by` / `date` / `reason`」「不勾任一复选框」等，均为提案成文期的流程约束记录；规格门现已 `approved` + 回填指纹（本纪要「## 规格门」小节），批准状态一律以该小节为控制值。订正 `tasks.md` 或其余三件套任一 artifact 正文会改变四件套指纹并触发 `CODEBUDDY.md §8` 规格解锁、须同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）；本段**不动 `tasks.md` 正文（仅翻 9 个复选框）、不改四件套、不解锁规格**，字面失效留后续独立会话按 §8 处置。
- **指纹不变自证**：本会话 `python tools/verify_all.py --fingerprint p15-presentation-evidence-workflow` = `d5ab6b5ec0ac806f1f1f7a13e6d93578c4e130a36e1b788f2846d56d18bfc721`，与「## 规格门」记录一致；复选框归一化（`verify_all.py:191-193`）+ `decision-memo.md` 不在指纹集合内，故勾选与追加登记均不解锁规格。
- **全量统一门禁（leader 实跑，闭合 `critic` R2）**：本会话后台由 leader 实跑 `python tools/verify_all.py` → `Verification OK`（含 `cargo test --workspace` 全量、determinism 30 项、ability system 8 项与各形态①反向验证、`cargo fmt --all -- --check`、`openspec validate` 各 change 与 `--all` 37 items、asset scan、三个门禁函数全绿）；`openspec validate --all`（非 strict）输出 43 条 >500 字 requirement 类 WARNING，属历史主 spec 已知项、如实标注、不冒充零告警。**全量门禁全绿只证明仓库门禁与规格结构一致，不证明 clean-room 取证 / 仓库外文档交接 / 视觉 oracle 对照 / 原作保真验收已发生**，诚实边界不变；本轮零代码改动，此全绿不蕴含任何确定性门禁回归被本次触发。

## apply 判据可操作性澄清（2026-11-26，登记为范围外收尾）

> 用户 2026-11-26 经 `AskUserQuestion` 拍板「登记为范围外收尾」（仓库文件外证据 / 人工确认项，非 AI 自定、非沉默授权）：针对「相同动作 / 时间点 / 机位的原作参考帧 + 真窗口人眼保真」这一验收判据，明确其在本项目当前阶段**不可操作**，登记为本阶段范围外 + 长期目标后续项，不改四件套正文、不解锁规格。本小节不在四件套指纹集合内（`tools/verify_all.py:175-195`），追加不触发 `CODEBUDDY.md §8` 规格解锁；「## 规格门」仍置于末尾且只有本节以「规格门」开头。

- **不可操作的事实根因（三层，逐条带锚，非「用户还没做」）**：
  - **原版客户端夹具缺失**：1.0 是自建引擎 Melee，不是驱动原版 War3 客户端跑「某特效 + 固定机位 + 固定帧 + 暂停抓帧」；p15 形态 A 明确不入库 / 不复制上游代码 / 不加 crate 依赖（`proposal.md:11-16`、`docs/decisions/0003-scope.md:113-116`）。
  - **PE2 / MDX 渲染管线缺失**：仓库**没有** PE2 粒子特效 / MDX 渲染实现；`crates/w3-render/tests/gpu_headless.rs:1-10` 明写离屏自测是「合成顶点流 NDC 三角形、不证明真实 WC3 模型上屏」，`w3-client/tests/real_map_e2e.rs` 是 glTF 真实模型 A/B 断言、非粒子特效；资产走 `godot_warcraft3` 导出管线且仓库不含资产（`design.md` D4 / `UPSTREAM.md:63-92`）。
  - **跨渲染器无共同帧基准**：「相同动作 / 时间点 / 机位」要在原版（自有粒子 + 固定管线）与本项目（wgpu 自建）之间对齐，两套渲染器无共同帧基准，物理上无从产出可比参考帧。
  - **判据来源与借用缺陷**：该判据借自 `godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:48,50`（那个项目做资产重烘焙、手上有原作截图），借到 p15 时**未核对前置条件是否成立**，于是 ③ 判据拍板成「机器判出图·人眼判保真」、`spec.md:41-44` 的 `Scenario` 写成 MUST 收敛到原作参考——但「原作参考帧」本阶段产不出来。这是规格设计缺陷，不是执行缺口。
- **`spec.md:44` 的硬约束是「MUST NOT 冒充原作保真验收已通过」**，不是「必须真的完成保真验收」。拿不到原作参考时，按 `3.2` 的正确处置就是如实回炉 + 登记为已知限制、不标通过；本轮证据侧文档写的「按 3.2 未定案回炉、不冒充完成」本就是合规分支。
- **处置结论（本阶段口径）**：把「原作保真 / 像素级对齐」**移出 p15 本阶段验收**，登记为长期目标「像素级复刻 War3」（`docs/decisions/0003-scope.md:94-100`）的后续 change 前提——待将来真有 PE2 / MDX 渲染管线 + 原作夹具再立项；不挂在 p15 上、不标通过、不冒充完成。p15 交付的验收收敛到 **「clean-room 证据流程走通一圈」的流程闭环自证**：仓库外行为规格（已有 16 条真实 `ev_` id 证据）→ 主仓库只读文档交接（`3.1`）→ 判定与产物同提交 + 独立重审（`3.2`）→ 消费侧离屏 `render_headless_rgba` / `_with_textures` 出图 + 逐字节确定性机器断言（`gpu_headless.rs` 已有、`git diff -- crates/` 为空的只读复用）。
- **不改四件套正文（不解锁规格）**：`spec.md` 是四件套之一，改其 `Scenario` 正文会改变四件套指纹 → 触发 `CODEBUDDY.md §8` 规格解锁 → 须同步回查 proposal / design / tasks + 重走 `critic` 规格门 + 重新由人拍板。按本次拍板**不改四件套、不动 `tasks.md` 正文、不勾任何新复选框**，仅在本纪要登记澄清；四件套指纹 `d5ab6b5ec0ac806f1f1f7a13e6d93578c4e130a36e1b788f2846d56d18bfc721` 不变、`spec_gate_check()` 保持 True。成文期 `spec.md:41-44` 措辞与本次口径的字面张力，沿用 P11 归档件 / p16 residual R1 体例如实登记、留后续正式修订 change 按 §8 处置。
- **范围边界（不混淆、不扩范围）**：真窗口 8.x 人工清单 / golden hash v9 双平台逐字节比对 / CI 双平台是 P9 / P10 遗留「待人工 / 待 VM」项，与本判据无关，仍不标通过；本 change 非玩法，`rule_doc_sync_check()` 豁免、`docs/references/WC3-MELEE-RULES.md` 零改动；零代码 / 零资产 / 零 `w3-core` 权威状态改动、不 bump 三版本号、不接 CI。`2.x / 3.x / 4.1` 复选框状态不变（仍 9/15），生产侧取证、判定重填、独立重审、只读交接仍归用户本人本机，AI 不碰原版客户端、不代判合规、不冒充完成。

## 规格门

- status: approved
- fingerprint: 34399bed5f512d58ca0a73ff833812c8c09887e4999e529e3d085d0e704cde2a
- approved_by: 用户（2026-11-25 经 `AskUserQuestion` 拍板「批准 + 回填 + 一并提交」与「特性按推荐，缺口延后 apply」；`critic` 规格门报告 `blocker = 0`、`reference-researcher` 对照报告已产出，由 leader 按授权序列回填，非 AI self-approve、非沉默授权）
- date: 2026-11-25
- reason: 用户 2026-11-25 拍板引入 clean-room 反编译证据流程试点（形态 A），`ADR 0003` 第二次修订 / `ADR 0001` 复核记录 / `README.md` / `CONTRIBUTING.md` 合规措辞已同步落地；`critic` 规格门报告 7 项全通过、`blocker = 0`，判定 A（对照报告未落盘非 blocker）/ B（无 `RULE_REF_PATTERN` 误判隐患）/ C（前置齐备）/ D（ADR 省略规格门自洽、无需重走）均成立；`reference-researcher` 独立对照报告已产出（按 §9 只读体例仅对话内文本、未落盘为仓库文件，结论落在 `design.md`「## 参考项目对照」）。规格门同时拍板：④ 试点特性采用推荐项（单个 PE2 / 飞弹特效的「混合模式 + 透明排序」视觉 oracle 对照）；其余 4 项参考缺口与 Deep + Clean room 组合**延后到 apply 阶段由人逐条拍板**，不冒充已确认。四件套自批准起锁定、指纹如上；提案阶段「保持 `status: pending` / 由 leader 回填 / 待确认项」等措辞为当时的流程约束记录（沿用 P11 归档件体例），批准状态一律以本小节 `status: approved` 为准；此后改任一四件套 artifact 即视为解锁，MUST 按 `CODEBUDDY.md §8` 同步回查其余三份并重走规格门（重新出 `critic` 报告、重新由人拍板）。 2026-11-26 按用户拍板移除对内涉密外部文章内容，四件套做最小清除改写后重算指纹并重新锁定；本次为合规处置、非玩法 / 非范围变更，由用户 2026-11-26 明确授权，未重走完整 §9 规格门（如实标注、不冒充已过规格门）

规格已锁定；`verify_all.py::spec_gate_check()` 机械校验 `status` 与四件套指纹一致。本 change 为非玩法 change，`rule_doc_sync_check()` 对非玩法 change 豁免；真窗口 8.x / golden hash v9 双平台 / CI 双平台仍属「待人工 / 待 VM」，不冒充通过。
