# Proposal

> 本轮只成文、不实施代码（apply 是后续独立会话）。本 change 是承接 `ADR 0003`（2026-11-25 第二次修订）复核点「决定引入反编译 / 抓帧 / REA 类逆向流程 → 单独立项、走规格门、由人拍板试点边界」的**独立试点 change**，为**非玩法 / 呈现层证据工作流** change。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。用户 2026-11-25 明确拍板「已决定本项目引入反编译 War3 的工作流」并选择「ADR 修订 + 形态 A 试点」；`ADR 0003` 第二次修订、`ADR 0001` 复核记录、`README.md` / `CONTRIBUTING.md` 合规措辞已由人拍板落地（已在工作区、未提交），本 change 不碰这些文件。本 change 只规范「呈现层证据流程」与单个特性的试点：**不是**已引入代码依赖、**不是**已放宽红线、**不是**已过规格门、**不是**已获暴雪授权。

## Why

长期目标是**像素级复刻 War3**；呈现层行为（水面绘制顺序、透明排序、遮挡剔除、粒子、动画播放混合、UI 观感）靠猜对不上，需要 clean-room 反编译证据流程产出**行为规格**，作为呈现层像素对齐的证据来源（依据 `docs/decisions/0003-scope.md:94-100,109-123`）。1.0（局域网联机 Melee 对战）只是短期目标；本 change 不扩 1.0 范围、不改玩法。

## What Changes

- 新增「呈现层证据流程」规范与**单个特性的试点**，形态 A（依据 `0003-scope.md:113-116`）：
  - **生产侧**：`craft-anything`（MIT OR Apache-2.0，纯 Rust clean-room 逆向 + AI 重建引擎、early alpha）作为**本机外部工具**（CLI / MCP）运行，取 WC3 原生二进制的伪 C / 调用图 / 行为规格；产出**行为规格文档**（不是代码、不是业务逻辑 oracle）。
  - **交接**：行为规格与证据记录存**仓库外本机证据目录**，主仓库**只读文档交接**（生产侧与消费侧只通过只读文档交接）；证据纪律按本项目自定约定（判定须经独立重审）→ 判定与产物同提交 + 独立重审。
  - **消费侧**：用 `crates/w3-render` 离屏 `render_headless_rgba` / `render_headless_rgba_with_textures` + ViewImage + 真窗口人工验收做**视觉 oracle**。
  - **WC3 资产与数据表仍以 `godot_warcraft3` 导出管线为准**（外部工具无 MPQ / BLP / MDX / SLK 专用解析器）。
- **明确不做**：不入库 / 不复制上游代码 / 不新增依赖 / 不登记 `NOTICE.md` / 不改 `w3-core` 权威状态 / 不 bump `SNAPSHOT_VERSION`·`SIM_VERSION`·`PROTOCOL_VERSION` / 不改 determinism 与 golden hash 门禁 / 不接 CI / 不改业务规则 / 不登记第六个只读参考项目 / 不改 p14 的「明确不纳入」边界 / 不改 `openspec/specs/**` 主 specs / 不追溯改写 P11 归档件。
- **红线（ADR 0001 / 0002）**：零资产入库、MPQ 派生数据不入库、不绕过 EULA / DRM / 登录 / 反作弊、仅用于用户合法自备的经典客户端；反编译证据 MUST NOT 进权威状态 / 状态哈希 / 快照 / golden hash / determinism 验收 / CI 门禁；表现层只读派生；`w3-core` 零改动（`ADR 0001` 复核记录 `0001-license.md:77-83`、`0003-scope.md:117-120`、`0002-determinism.md:49-53`）。
- **本 change 是非玩法 change**：不实现任何 `R-x.y`，`docs/references/WC3-MELEE-RULES.md` 无需同步（`decision-memo.md`「## 规则文档同步」小节按 §14 第 5 条给「无需同步」结论、pending / 非玩法豁免）。
- **试点特性选择、5 项参考缺口、`craft-anything` 的 Deep 模式 + Clean room 组合缺口**一律为**规格门待确认项**、由人拍板，AI 不 self-approve（见 `decision-memo.md`「待确认项」与 `design.md`「## 参考项目对照」）。

## Capabilities

### New Capabilities
- `presentation-evidence-workflow`: 呈现层证据流程的行为契约 —— 以本机外部工具在仓库外产出行为规格文档（不是代码 / 不是业务逻辑 oracle）、经只读文档交接到主仓库、判定与产物同提交 + 独立重审、由 `w3-render` 离屏 / 真窗口视觉 oracle 消费；显式声明零资产 / 零新增权威状态 / 零确定性影响 / 不 bump / 不接 CI / 五个只读参考项目不变 / 单特性试点由规格门拍板。

### Modified Capabilities
<!-- 无。本 change 只新增一个 capability；本轮成文不改任何既有主 spec 的 requirement（不新增 MODIFIED delta）；`git diff -- openspec/specs/` 为空。 -->

## Impact

- **路线图阶段**：P15（P0–P5 及 P6–P14 全部已实施并归档，见 `CODEBUDDY.md §1` 与 `openspec list`）；本 change 是工作流 / 呈现层证据独立 change，**非玩法 change**，不实现任何 `R-x.y`、不动 1.0 范围 S1–S6、不改 `docs/references/WC3-MELEE-RULES.md`。
- **ADR**：0003（第二次修订：承接复核点、形态 A 试点、单特性由规格门拍板）、0001（复核记录：零资产、不复制代码、不绕 EULA / DRM、产物留本机不入库、不登记 `NOTICE.md`）、0002（表现层只读派生、反编译证据不进权威状态 / 哈希 / 快照 / 门禁）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）；本 change **不新增 / 不改任何 ADR 正文**、不碰 `README.md` / `CONTRIBUTING.md`。
- **确定性契约**：**零 `w3-core` 权威状态改动、零确定性影响**。不新增权威状态字段（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，当前 9）、不进 `first_difference()`、不改帧序 / 命令排序 / 状态哈希编码；不 bump 三版本号（当前权威值均为 9，锚 `crates/w3-core/src/snapshot.rs:67-68` / `crates/w3-net/src/packet.rs:50`）；不改 determinism 与 golden hash 门禁。`CODEBUDDY.md §5` 硬门禁与形态①反向验证均不适用（不触确定性代码、不重跑 `cargo test -p w3-core --test determinism`）；tasks **不放** determinism 重跑任务。反编译证据属视觉 oracle / 行为规格，**不是** ADR 0002 口径「可冻结、机器可判、可进 CI 的业务逻辑 oracle」。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效入库；不新增第三方依赖、`Cargo.lock` 不变、不登记 `NOTICE.md`；仅用于用户合法自备的经典客户端、不绕过 EULA / DRM / 登录 / 反作弊；不声称获暴雪授权。`craft-anything` 是**本机外部工具 / 人工确认项**，MUST NOT 登记为第六个只读参考项目；五个只读参考项目仍是 `vera20k` / `godot_warcraft3` / `rts_kernel_cs` / `WarsmashModEngine` / `ConcordRTS`（`CODEBUDDY.md §1`、`UPSTREAM.md:9-15,96,100`）。
- **代码 / 系统**：本轮只成文，改动仅落在 `openspec/changes/p15-presentation-evidence-workflow/`；不改 `crates/**` / `openspec/specs/**` / `docs/**` / 归档件 / 根配置 / p12 / p13 / p14。消费侧只读复用既有 `w3-render` 离屏能力（`crates/w3-render/src/gpu/mod.rs:908-927`），不改其实现。
- **参考项目矩阵**：`UPSTREAM.md:104-114` 的 T5（场景 / 相机 / HUD / 3D 呈现，`UPSTREAM.md:110`）**主命中**；T1 / T3 / T7 **可选交叉（仅红线）**；T2 / T4 / T6 **不命中**。`reference-researcher` 对照报告**本轮已产出**，结论逐条落在 `design.md`「## 参考项目对照」；T1 承重锚一律 `vera20k@305ea3a75:<path>:<line>` 固定提交双标。
- **依赖 / CI**：不新增第三方依赖、不接 CI；`craft-anything` early alpha / Clean room 不是 OS 沙箱 / Deep + Clean room 组合无 live 验证先例等缺口如实登记为规格门待确认项。
