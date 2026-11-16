# 决策纪要 — `runtime-entity-movability`

> 取料纪律：基于仓库 artifacts、只读参考项目与用户 2026-11-16 本会话拍板回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md` §6）。
> 本 change 承接 P8.0 / P8.1 登记的已知限制 **B1**（运行时 `Train` / `Build` 动态实体可移动性无法判定）；规格门已由用户 2026-11-16 本会话指令 + `AskUserQuestion` 方案拍板批准（与 P8.1「用户指令作为批准」处理一致）。`reference-researcher` 已按 `UPSTREAM.md` 矩阵 T1 / T4 出对照报告，锚由 `spec-steward` 逐条打开复核，见 `design.md`「## 参考项目对照」。
> **锚写法**：本轮 `w3-core` / `w3-client` / `w3-render` 代码会被 owner 改动，四件套与本纪要的证据锚一律用「文件路径 + 函数名 / 符号名」、不写 `:NN` 行号；对未改动文件用章节号 / 规则编号 / 矩阵条目。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | **ADR 0002 零冲击**：`w3-core` 首次被动，但只加只读视图字段；`cargo test -p w3-core --test determinism` 的 golden hash 逐字节不变、状态哈希字段集合不变、快照头部布局（`defs_hash` / `content_hash`）不变、`SNAPSHOT_VERSION` 仍 3、`first_difference()` 与 `ExecuteFrame → PlayerId → Sequence → Arrival` 排序键 / 稳定 `EntityId` 顺序不变；不新增命令 / 事件 / 权威状态字段 | `docs/decisions/0002-determinism.md` §5 / §6-8 / `CODEBUDDY.md` §5 / `sim_match.rs::compute_state_hash` / `sim_match.rs::capture_snapshot`（`PositionRecord.speed` 已含 speed）/ `sim_match.rs::read_view` / `proposal.md` Impact |
| C2 | **只读投影非权威状态**：`EntityView` 是 `read_view()` 派生的只读视图、修改不影响模拟、不进哈希 / 快照；`UnitState.speed` 是**已存在**的权威字段且已在哈希 / 快照中，本次仅投影到只读视图；只读投影范式参照 `vera20k` 但结论出处是本项目 ADR 0002 | `sim_match.rs::EntityView` / `sim_match.rs::UnitState` / `sim_match.rs::read_view` / `sim_match.rs::capture_snapshot` / `docs/decisions/0002-determinism.md` §5 / `UPSTREAM.md` §「按任务类型的强制参考矩阵」T1 + `vera20k/src/net/command_exchange.rs::RetainedSendProgress`（只读投影范式，GPLv3 + Section 7 仅借范式、不复制代码）|
| C3 | **R1.4 / R13.6 分层判据**：frame-0 装配实体由装配期 `UnitDef.speed_raw` 派生映射判定（P8.0 D1 权威），运行时动态实体由只读视图 `EntityView.speed == 0` / `> 0` 判定；两条来源不可互替（`Builder::spawn` 把 frame-0 `speed` 抬到 `max(1)`、运行时建筑 `speed == 0`）；建筑 / 金矿 / 主城 MUST NOT 生成 `Move` | `docs/references/WC3-MELEE-RULES.md` R1.4 / R13.6 / §14 / `input.rs::derive_movability` / `setup.rs::Builder::spawn` / `command.rs::CommandQueue::submit`（`InvalidSpawn`）/ `sim_match.rs::complete_production` / `openspec/specs/unit-orders/spec.md`「群体移动只作用于可移动单位」/ `design.md` D2 |
| C4 | **`filter_movable` 签名 / 既有回归零改动**：保持既有签名与对已登记实体行为不变；合并函数在 `InputState::apply` 群体 `Move` 分支把未登记实体按视图 speed 补进有效映射，`apply` 已持有 `view: &WorldView` 无需改签名；`tests/movability.rs` 5 处直接调 `filter_movable` / `degenerate_filter` 的反向验证断言零改动仍成立 | `input.rs::filter_movable`（`unwrap_or(true)`）/ `input.rs::InputState::apply` / `crates/w3-client/tests/movability.rs::degenerate_filter` / `openspec/changes/archive/2026-11-15-p8-faction-foundation/design.md` D1 / D3 / `design.md` D3 |
| C5 | **反向验证（改坏验红）**：新增 / 修改玩法回归 MUST 证明「去掉动态实体兜底、退回旧 `unwrap_or(true)` 默认放行」令「运行时生产建筑不响应 `Move`」断言在缺陷态失败、正式实现通过；仅「全绿」不算完成 | `CODEBUDDY.md` §5 / `openspec/config.yaml` `rules.tasks` 第三条 / `crates/w3-client/tests/movability.rs::degenerate_filter`（P8.0 真 A-B 先例）/ `design.md` D4 |
| C6 | **零资产 / 数值 `[待核]`**：全为合成 / 原创编译期常量，零文件 IO、零 MPQ 派生数据、零真实导出 JSON、零新增依赖 / `Cargo.lock` 改动；具体数值不冻结冒充原版、以本机导出 SLK 核对、不入库 | `docs/decisions/0001-license.md` / `docs/decisions/0004-frozen-coordinates.md` / `CODEBUDDY.md` §6 / `docs/references/WC3-MELEE-RULES.md` §14 / `UPSTREAM.md` §「按任务类型的强制参考矩阵」T5 |
| C7 | **1.0 范围 S1 / S3、N1–N6 不纳入**：落在 S1（局域网联机对战，含真人 VS AI 敌人玩家）/ S3（4 种族做完）的过程分期；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」不是不做项；N5（JASS VM）/ N6（资产入库）尤其不得纳入 | `docs/references/WC3-MELEE-RULES.md` §0.1 S1–S6 / §0.2 N1–N6 / §0.3 / `CODEBUDDY.md` §1 / `proposal.md` Impact |
| C8 | **与 P8.1 / `p8-human-faction` 正交**：P8.1 已归档、`p8-human-faction` 仍 active 且规格锁定；本 change 不碰其四件套正文、只新增 ADDED Requirement 不改写既有 requirement；四件套一次性成文、任一 artifact 改动按 §8 同步回查其余三份 | `CODEBUDDY.md` §8 / §9（规格锁定与重审）/ `openspec/changes/p8-human-faction/decision-memo.md`「规格门」/ `openspec/changes/archive/2026-11-15-p8-faction-foundation/decision-memo.md`「B1 代码门 blocker」/ `design.md` D6 |

**归因声明**：C1 / C2 的「新增权威状态字段须同步进哈希 / 快照 / `first_difference()`」硬门禁出处是**本项目** ADR 0002 §6-8 + `CODEBUDDY.md` §5 + 主 `openspec/specs/melee-match/spec.md`，**不是任何参考项目**；C2 中 `vera20k` 只提供「只读投影」范式、锚指向 `UPSTREAM.md` 矩阵 T1 条目与参考项目内路径，不得冒充参考结论（见 `design.md`「参考项目对照」许可证边界）。

## 拍板决策（含被否备选与理由）

- **P1 用户 2026-11-16 本会话指令 + `AskUserQuestion` 方案拍板（规格门批准依据）**。用户先指令「继续补齐动态实体承接」，紧接的 `AskUserQuestion` 中明确选择「**只读暴露 speed（推荐）**」：给只读 `EntityView` 加 `speed` 字段、投影内核已有权威 `UnitState.speed`；frame-0 用装配期映射、运行时动态实体用视图 speed 判据。这是**人对技术方案与规格的拍板**，等价于规格门批准（与 P8.1「用户指令作为批准」处理一致）。证据形态：**本会话用户消息 / 选项（非仓库文件，由人工核对）**——依 `openspec/decision-memo.template.md` 三分判据第 3 类，仓库内无可核验 `file:line` 锚，**不冒充已落库文档**。**用户拍板的是方案与放行、不是逐条技术细节确认**：D1–D6 的实现细节（字段类型、合并函数落点、构造点清单、反向验证形态）为 `spec-steward` 依根因核实后成文，不写成「用户已逐条确认」。方向落地锚：`proposal.md` Why / `design.md` Decisions。被否方向见「被否方向台账」。
- **P2（重大决策，随 P1 批准采纳）首次放松 `w3-core` 实现层「零改动」约束**。P8.0 / P8.1 把「`crates/w3-core/` 零改动 + MUST NOT 扩展 `WorldView` / `EntityView` + MUST NOT 用运行时 `speed`」写成实现层硬约束（B1 由此无法判定）；本 change 经用户拍板**首次放松该实现层约束**，改为「只加只读视图字段 + 分层判据」，**ADR 0002 确定性契约一条不放松**（golden hash 逐字节不变、不新增权威状态 / 命令 / 事件 / 排序键）。证据形态：同 P1 的用户方案拍板；**用户拍板的是方案与放行、不是逐条技术细节确认**（「零哈希冲击」的技术论证由 `spec-steward` 依 `sim_match.rs::read_view` / `capture_snapshot` 现状核实后成文，见 C1 / C2）。锚：`openspec/changes/archive/2026-11-15-p8-faction-foundation/decision-memo.md` C1 / C5 /「B1 代码门 blocker」/`openspec/changes/p8-human-faction/decision-memo.md` C1 / C4 / `design.md` Context / D1。**被否两备选**：① 让生产事件携带 `DefId` / `speed`（在 `complete_production` push `Event`）→ 改事件序列会动 `next_event_seq` 与逐帧哈希、**golden hash 全变**，且新增事件变体属权威行为；② 坚持内核零改动、纯表现层命令流追踪 → `Train` 命令带 `DefId` 但那是命令不是实体视图，`spawn` 出的 `EntityId` 与命令无稳定关联、联机 / 回放 / 重放下**易错配**（B1 正是此路不可判定）。另有 「`EntityView` 直接暴露 `DefId`」「放开 `Spawn` 校验」「新增运行时权威状态」等被否方向见台账。
- **P3 技术方案随批准采纳进入 apply**。D1–D6（只读投影 / 分层判据 / 合并函数 / 反向验证 / playable 兜底 / 范围正交）与 capability 划分（新增 `runtime-entity-orders`、`unit-orders` 增 ADDED Requirement）随 P1 规格门批准采纳，进入 `/opsx:apply`（`sim-owner` 守 `w3-core`、`client-owner` 守 `w3-client` / `w3-render`）。**用户拍板的是方案与放行、不是逐条技术细节确认**：capability 命名、Scenario 拆分、tasks 分组为 `spec-steward` 依 `CODEBUDDY.md` §8 四件套一致性成文，不写成用户逐条批准。锚：`proposal.md` Capabilities / `specs/` / `design.md` Decisions / `tasks.md`。

## 被否方向台账

- **R1 生产 `Event` 携带 `DefId` / `speed`** → 否决：改事件序列会动 `next_event_seq` 与逐帧哈希、golden hash 全变，且引入事件变体属权威行为（C1 / ADR 0002）。锚 `design.md` D1 备选 1 / `sim_match.rs::complete_production`。若前提后续变化 MUST 整体重估，不沿用旧否决。
- **R2 纯表现层命令流 + diff 追踪 `EntityId`** → 否决：`GameplayCommandKind::Train` 带 `DefId` 但那是命令不是实体视图，运行时 `spawn` 出的 `EntityId` 与命令无稳定关联，联机 / 回放 / 重放易错配；B1 根因即此路不可判定。锚 `design.md` D1 备选 2 / `sim_match.rs::complete_production`。
- **R3 `EntityView` 直接暴露 `DefId`** → 否决：需把 `DefId` 存进 `UnitState`（新增权威状态字段）或把装配层映射塞进内核，都要动哈希 / 快照 / 单一状态所有者边界；且用户拍板方案是「只读暴露 speed」而非 `DefId`。锚 `design.md` D1 备选 3 / `sim_match.rs::UnitState`。
- **R4 改 `CommandKind::Spawn` 校验放开 `speed = 0`**（让 frame-0 建筑也 `speed == 0`）→ 否决：改命令语义 / 线格式 / `CommandQueue::submit` 校验，破坏 P8.0 已定的两族 `Spawn` 字节一致性、frame-0 装配路径全面变动。锚 `design.md` D1 备选 4 / D2 备选 4 / `command.rs::CommandQueue::submit` / `setup.rs::Builder::spawn`。
- **R5 新增 `Player.race` 或任何运行时权威状态字段承载可移动性** → 否决：新增权威状态须同步进哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`（ADR 0002 §6-8）；与「`UnitState.speed` 已存在且已进哈希 / 快照」重复。锚 `design.md` D1 备选 5 / `openspec/changes/p8-human-faction/decision-memo.md` R1。
- **R6 把 P8.5 深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、HUD / 键位接线、联机底座 / 回放重做塞进本 change** → 否决：C7 / C8 正交与单 change 单 owner；用户 2026-11-15「别扯不重要的事」、2026-11-16 只授权「继续补齐动态实体承接」。锚 `design.md` D6 / `proposal.md` Impact / `docs/references/WC3-MELEE-RULES.md` §0.2 N5 / §14。

## 待确认项（如实标注，不冒充已拍板）

- **具体数值 `[待核]`** → 本 change 无新增玩法数值；视图 `speed`、测试构造值全为合成原创 `[待核]` 常量，不冻结冒充原版；须以玩家自备正版客户端经 `godot_warcraft3` 导出的本机 SLK 核对、不入库。锚 `docs/references/WC3-MELEE-RULES.md` §14 / `design.md` D6。
- **真窗口人工验收未执行** → 本轮为规格成文 + CPU 侧验证设计，真窗口人工验收**未执行 / 待补验**，不冒充通过。锚 `proposal.md` Impact / `tasks.md` Workflow follow-up。
- **`file:line` 锚因代码改动会漂移（已用符号名规避）** → 本轮 `w3-core` / `w3-client` / `w3-render` 会被 owner 改动；四件套与本纪要锚已一律用「路径 + 符号名」/ 章节号 / 规则编号 / 矩阵条目，规避 P8.1 演练发现的行号漂移返工；参考项目为只读固定提交、其 `file:line` 锚不漂移（见 `design.md`「参考项目对照」开头声明）。apply 后如需更正按 `CODEBUDDY.md` §9 规格锁定与重审处理。
- **P8.5 深度玩法 / 野怪 / 物品 / 迷雾 / HUD / 键位接线** → 由后续独立 change 承接，本 change 不排期细节、不纳入。锚 `docs/references/WC3-MELEE-RULES.md` §0.1 / §13 R13.7 / `design.md` D6 / R6。
- **`critic` 代码门待后续复核** → 规格门已由用户拍板批准，但**代码门（全清单，含第 7 / 10 项四件套一致性、第 11 项反向验证真 A-B、第 12 项参考对照、第 13 项 `R-x.y`、确定性 / 首次动 `w3-core` 零冲击）尚未执行**，由 apply 完成后委派只读 `critic` 复审全部 diff；结论（blocker / residual）回报前不冒充通过。锚 `CODEBUDDY.md` §9 / `tasks.md` 6.3 / 6.4。
- **playable 默认态处置（client-owner 已回报、显式处置完成）** → `examples/playable.rs`（4 个可移动单位、`speed>0`、无建筑、无装配期 `def_ids` 关联）与 `app.rs::ClientApp::new` 默认空映射：结论为「**合并函数 `effective_movability` 从只读视图现算兜底已覆盖、无需额外注入**」——注入多余且会掩盖兜底生效。已用 `playable_default_empty_map_keeps_all_spawn_units_movable` 与 `default_empty_movability_does_not_release_runtime_building` 两条回归坐实，并在 `app.rs` / `playable.rs` 两处中文注释显式登记。锚 `crates/w3-client/tests/movability.rs` 对应测试 / `design.md` D5 / `tasks.md` 4.1 / 4.2。本条已闭合，不再待确认。
- **活文档字面禁令同步（docs-owner 已完成）/ 主 spec 仍待后续承接** → `WC3-MELEE-RULES.md` R1.4 / R13.6 与主 `openspec/specs/unit-orders/spec.md` 既有 requirement 的「MUST NOT 扩展 `EntityView` / 用运行时 `speed` / `w3-core` 零改动」是 P8.0 / P8.1 阶段实现层禁令；本 change 经拍板放松该实现层约束（ADR 0002 零冲击不变）。**活文档 R1.4（:97）、R13.6 第 2/3 条、§15 变更记录已由 `docs-owner` 按 §14 流程同步修订**：改为分层判据口径、注明 2026-11-16 拍板依据、保留 R1.4 核心语义与 R13.6 第 1 条键盘过滤、编号 / 章节号零破坏、不静默改规则。**主 `openspec/specs/unit-orders/spec.md` 既有 requirement 正文与 `p8-human-faction` 四件套正文仍由后续 change / 归档复核承接**（本 change 只新增 ADDED Requirement、不改写既有正文），此项如实保留为待确认、不冒充已解决。锚 `design.md` Context / D6 / `proposal.md` Impact / `CODEBUDDY.md` §8。
- **归档与人工 push** → 归档后须确认主 `openspec/specs/` 新增 `runtime-entity-orders` capability、`unit-orders` 并入 ADDED Requirement；本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀），push / fetch / pull / clone 及任何远端同步人工独占（`CODEBUDDY.md` §4 / §10）。

## apply 实施结论（2026-11-16，如实登记）

- **首次动 `w3-core`（sim-owner）**：`crates/w3-core/src/sim_match.rs` 仅加 `EntityView.speed: i64` 只读字段 + `read_view()` 投影 `unit.speed.raw()` + 3 类共 359 行测试（投影一致性 / 运行时生产建筑 `speed==0` 可观测 / 确定性零冲击）。`git diff -- crates/w3-core/` 仅此 1 文件 +364 行、0 删除，哈希 / 快照 / 命令 / 事件 / 状态字段 / `SNAPSHOT_VERSION` 全部零改动。
- **golden hash 逐字节不变（C1 验收线）**：`cargo test -p w3-core --test determinism` 26/26；`tests/determinism.rs` 文件 sha256 前后一致（无硬编码 golden 常量，改用 `examples/golden_hash.rs` 整份输出改动前后 `diff -u` 无输出、`final_hash=0x3d7e4312f63844e5` 与 ADR 0002 复核记录逐字吻合）。改坏验红：投影改成 `speed:0` → w3-core lib 2 条测试变红、确定性零冲击测试仍绿（反证视图不进哈希 / 快照）。
- **下游分层判据（client-owner）**：13 处 `EntityView` 构造点补 `speed`；新增纯函数 `effective_movability(base, view)`（已登记项原样保留、未登记运行时实体按 `view.speed != 0` 补），在 `InputState::apply` 群体 `Move` 分支接线；`filter_movable` / `derive_movability` / `apply` 签名零改动、P8.0 既有 5 处反向验证断言零改动仍成立。playable / 默认空映射由兜底覆盖（见待确认项已闭合条目）。
- **真 A-B 反向验证（C5）**：`tests/movability.rs` 用真实 `RtsMatch` + `GameplayCommandKind::Train{is_building:true/false}` + `step()` 生产出建筑 / 单位，经 `read_view()` 得 `speed==0` / `>0`，断言运行时建筑不响应 `Move`、单位与 frame-0 生产者响应、frame-0 混合选中不退化、状态哈希 / 快照 / `SNAPSHOT_VERSION==3` 不变；退化实现（撤回合并函数接线 = 旧 `unwrap_or(true)`）经真实 `apply` 路径令 `reason=runtime_building_must_not_move` 断言失败。**leader 在最终文件上重跑 A-B（修正增量中间态的旧计数）**：缺陷态 `cargo test -p w3-client --test movability` → `19 passed; 4 failed`（4 条失败恰为 `runtime_building_does_not_move_but_unit_and_peasant_do` / `runtime_move_filter_matches_movable_only_script_frame_by_frame` / `default_empty_movability_does_not_release_runtime_building` / `reverse_validation_formal_impl_passes_runtime_building_assertion`），恢复接线后修复态 `23 passed; 0 failed`；元断言证明退化解等价 `unwrap_or(true)` 全放行，`critic` 静态推演「删接线恰好 4 条变红」与实测吻合。
- **门禁（leader 独立复核，非采信转述）**：`cargo fmt --all -- --check`、`determinism 26/26`、`cargo test --workspace` **全绿**、编码 269 文件 / 源码约定 86 Rust 文件 / `git diff --check` 全绿、`openspec validate runtime-entity-movability --strict` `is valid`、指纹 `221e318c…` 不变（规格锁定未解锁）、`decision_memo_check` / `spec_gate_check` 均 True。**GPU 环境条件（与 `p8-human-faction` 纪要口径对齐）**：本机 `real_map_e2e` / `gpu_headless` 的 `0xc0000005` 原生崩溃为间歇性、与改动无关（本机 `config.toml` 指向真实导出资产时触发、CI 无资产优雅跳过）；本轮 `cargo test --workspace` 与完整 `verify_all.py` **未复现、全绿**，但「本轮未复现」不冒充「GPU 永久修复」，两纪要差异仅因本机环境/驱动瞬时状态不同，非结论冲突。
- **`critic` 代码门**：见下「代码门结论」小节（已回报、已回填）。

## 代码门结论（`critic` 2026-11-16 已回报）

- status: passed_no_blocker
- report: 只读 `critic` 全清单复审全部 diff（13 项）→ **blocker 0**，确定性零冲击按从严口径「无疑点」（`w3-core` 仅 `sim_match.rs` 1 文件 +364 行 / 0 删除、视图结构上不进哈希 / 快照字节路径、`hash_unit`/`PositionRecord.speed` HEAD 即已含 speed、`SNAPSHOT_VERSION`/`SIM_VERSION` 仍 3、`tests/determinism.rs` 与 `golden_hash.rs` 零改动）、反向验证真 A-B（静态推演删接线 4 条变红，与 leader 实测 `4 failed` 吻合）、四件套一致性 / 规格锁定（指纹仍 `221e318c…`）、参考对照逐锚、R1.4 / R13.6 / 活文档 §14 修订、编码 / 所有权 / 范围 / Git 边界全部通过。结论：**可进入本地 commit**。
- residual（如实登记、不阻塞）：R1 tasks 勾选口径 → 已勾齐 1.1–1.4 / 6.3–6.4（复选框归一化不改指纹）；R2 A-B 计数 → 已在最终文件重跑取真值（`19 passed / 4 failed` ↔ `23 passed`）；R4 change 目录未跟踪 → commit 前显式 `git add openspec/changes/runtime-entity-movability/`；**R3 主 `openspec/specs/unit-orders/spec.md:39` 字面禁令「MUST NOT 扩展 EntityView / 用运行时 speed」与本 delta 直接矛盾** → 已在 `design.md` Context / D6、`decision-memo.md` 待确认项、`tasks.md` Workflow follow-up 如实登记为「后续 change / 归档时用 `## MODIFIED Requirements` 消解或同步主 spec / `p8-human-faction` C4」，本 change 不改写既有正文、不冒充已解决，**归档前 MUST 处理**；R5 `openspec validate --all` 9 条超长 requirement WARNING 为既有、非本 change 引入；R6 两份纪要 GPU 结论差异 → 本条已补环境条件说明。
- `critic` 复审边界：按分工未跑 `cargo test`（与 leader 避让），determinism / A-B 计数 / workspace 执行结果以 leader 实测为准，静态证明独立成立。

## 停止条件（追加阶段 / 归档前）

1. C1–C8 逐条仍有证据锚、无一条被本轮改动证伪（尤其 C1 golden hash 逐字节不变须以命令输出实证）。
2. 规格门由人拍板、`fingerprint:` 回填真值、`status: approved`，且 `verify_all.py` 的 `spec_gate_check()` 不报失配。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛；playable 处置结论、`critic` 代码门结论回报前不冒充通过。
4. 四件套一致性按 `CODEBUDDY.md` §8 回查：proposal 的 Scope / specs 的 `#### Scenario:` / design 的 alternatives / tasks 的步骤互不打架；任一 artifact 改动 MUST 同步回查其余三份。
5. 归档时：tasks 复选框状态、已知限制承接、delta 合并去向、活文档 / 主 spec 字面禁令同步情况均如实登记。

## 规格门

- status: approved
- fingerprint: 221e318cbff406897e6c0961c230a764b0a88ea6eeebae4041881203f1979e38
- approved_by: 用户 2026-11-16 本会话指令 + AskUserQuestion 方案拍板（非仓库文件，由人工核对）
- date: 2026-11-16
- reason: 用户 2026-11-16 本会话指令「继续补齐动态实体承接」+ 紧接 `AskUserQuestion` 明确选择「只读暴露 speed（推荐）」方案，属**人对技术方案与规格的拍板**、等价于规格门批准（与 P8.1「用户指令作为批准」处理一致），非 pending、非 self-approve、不再向人请求批准；只读 `reference-researcher` 已按 `UPSTREAM.md` 矩阵 T1 / T4 出「参考对照报告」且锚由 `spec-steward` 逐条打开复核（见 `design.md`「## 参考项目对照」），参考缺口随方案拍板放行、不臆造；`critic` 规格门结论随该拍板采纳，**`critic` 代码门待 apply 完成后复核**（待确认项如实登记、不冒充通过）。用户拍板的是方案与放行、不是逐条技术细节确认。指纹真值由 `python -B tools/verify_all.py --fingerprint runtime-entity-movability` 在四件套成文并 `validate --strict` 通过后取，`decision-memo.md` 不在指纹纳入范围内、回填不自指死锁。
