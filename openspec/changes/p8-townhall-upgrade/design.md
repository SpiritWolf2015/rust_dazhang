# Design

## Context

见 `proposal.md` 的 Why / Scope：本 change 是路线图 `docs/roadmap/P8-human-orc-melee.md §3 P8.5`（:119-131）的 **P8.5b「主城升级」**（:126 表行「主城 Keep / Stronghold 升级、解锁二线单位与科技」、验收「升级后训练 / 解锁生效，快照哈希稳定」）；P8.5 全量已由用户 2026-11-14 拍板必做（:28 / :151），五个子系统各单独 change、串行推进、不塞回 P8.0–P8.4（:121 / :30），P8.5a `p8-hero-revive` 已于 2026-11-20 归档、本 change 紧接。**用户 2026-11-20 经 `AskUserQuestion` 明确拍板方向 A「独立 tier 状态」**（锚：本会话用户消息 / leader 转达，非仓库文件、由人工核对；按 `openspec/decision-memo.template.md` 三分判据第 3 类标注，不冒充已落库文档）：新增「主城等级 / tier」顶层权威状态，按 ADR 0002 全流程进 FNV-1a 状态哈希 + 由 `SNAPSHOT_VERSION` 定义的快照 + `first_difference()`，升级与解锁在 `w3-core` 确定性命令面生效，HUD 只读派生；同步修订主 specs「零 diff / 仍 3 或 4 / golden hash 逐字节不变」历史口径，golden hash 基线必变。被否备选 A′「复用 `PlayerEconomy.upgrades` 布尔旗标 + 静态 tier 表」/ B「表现层只读派生假升级」见 `decision-memo.md` P1 与「## 被否方向台账」；A′/B 均达不到验收线「升级后训练 / 解锁生效、快照哈希稳定」。

**现状差距（逐锚为真，C 类承重约束见 `decision-memo.md` C1–C24，本会话逐锚 / 抽样打开核实；P8.5a 刚改过 `w3-core`，行号已按当前代码复核）**：

1. **无主城等级 / tier 权威状态**：`RtsMatch` 顶层权威字段含 `revive: HeroReviveState`（P8.5a v4 新增，`crates/w3-core/src/sim_match.rs:229` / `:418`），`Snapshot` 顶层字段只到 `revive`（`crates/w3-core/src/snapshot.rs:79-107`，尤其 `:104-107`），`FrozenDefinitions` 只有四类定义 `units/abilities/items/upgrades` + 复活常量段 `revive` + 祭坛 / 英雄类别 `altar_defs` / `hero_defs` + `hash`（`crates/w3-core/src/gameplay.rs:210-220`）；全库 Grep「tier / TownHall / Keep / Stronghold / upgrade_town」在 `w3-core/src` 零命中。
2. **无升级命令**：`GameplayCommandKind` 现为 **12 变体**（10 基础 + P8.5a `ReviveHero` / `CancelRevive`，`gameplay.rs:828-877`），`hash_gameplay_command` 码 **1–12**（`gameplay.rs:932-1011`，`:1001` / `:1007` 为 11/12），校验 `validate_gameplay_entities`（`gameplay.rs:1013-1037`）/ 穷尽分发（`sim_match.rs:1207-1239`）按这 12 类；`Train{producer,unit_def,is_building}` / `Research{upgrade}` 不能表达「指定主城实体 + 目标 tier」；线格式 `oneof` tag 4–13 + 33/34、`reserved 14 to 32`（`crates/w3-net/src/proto/w3net.proto:128-151`）。
3. **训练 / 解锁当前全量可训练**：`enqueue_production` 只校验 `is_actionable` 与 `def.train_frames`（`sim_match.rs:1104-1128`），`complete_production` 生成实体（`sim_match.rs:1535-1577`），均**无 tier / requires 前置判据**；`UnitDef` 只有 7 个整数初值字段（`gameplay.rs:162-172`）、`UpgradeDef` 只有 `damage_mod` / `hp_mod` / `research_frames`（`gameplay.rs:191-197`），`compute_defs_hash` 只编 units / abilities / items / upgrades / revive / altar_defs / hero_defs（`gameplay.rs:268-316`）；`w3-client/src/melee/command_card.rs:214,233,240-293,481` 的 `TownHall` 只是 `TrainCategory`、无升级格位。
4. **研发 / 攻防升级是另一套**：`research` 写 `PlayerEconomy.research`（`sim_match.rs:1139-1148`），逐帧推进完成后写 `PlayerEconomy.upgrades: BTreeMap<DefId, bool>`（`gameplay.rs:763`）并 `apply_upgrade`（`sim_match.rs:1497-1516` / `:1579-1594`）——这是布尔科技旗标，正是 A′ 想复用而方向 A 否决的字段（C5）。
5. **HUD 无 `U` 主基地升级格位**：`w3-client/src/melee/command_card.rs:214,233,481`、`crates/w3-client/src/app.rs:819-883`、`crates/w3-client/src/hud.rs:125-150,467-540` 无升级进度只读派生；P8.5a `ReviveHudModel`（`hud.rs:131` / `derive` :144 / `ReviveHudModel::derive(snapshot, events)` :540）可作只读派生范式（C6）。
6. **确定性契约现状**：`SNAPSHOT_VERSION = SIM_VERSION = 4`（`snapshot.rs:12-21`，`sim_match.rs:2368-2369` 硬断言 4），`Snapshot.revive` 是 v4 顶层权威状态，`compute_state_hash` / `first_difference` / `capture_snapshot` 已含 `defs` / `gameplay` / `revive`（`sim_match.rs:1680-1740` 尤其 `:1736`、`:1742-1798` 尤其 `:1794`、`:1800-1897` 尤其 `:1878`），determinism 26 项（`crates/w3-core/tests/determinism.rs`，本会话 `grep -c "#\[test\]"` = 26），golden hash 已按 P8.5a 重取（Windows 侧 `final_hash=0xada6506e1da36c11`，`crates/w3-core/examples/golden_hash.rs:12-23`）但 **Debian 12 侧 v4 逐字节比对仍待人工 / VM**（`ADR 0002:109-110`）。路线图 :138「`SNAPSHOT_VERSION` 变更须 ADR 复核」、`ADR 0002:62-72`（§7 无跨版本迁移）。

**`reference-researcher` 只读对照报告**（命中 T1 / T2 / T4 / T5；**Q2 条件命中 T6、T3 仍不命中**——见「## 参考项目对照」；矩阵 `docs/references/UPSTREAM.md:81-88`、触发时机 :90-96）已产出但**未落盘**（本会话 leader 转达、列为人工确认项，见「## 已知限制 / 边界」），本文件「## 参考项目对照」逐条转写其全部结论与证据锚（这是证据链唯一留存机会），`decision-memo.md` C12–C20 转写关键锚。四个参考项目根与 9 个代表锚本会话已确认 `EXISTS`（见「## 参考项目对照」末「锚可打开性」），全部只读、不复制代码（ADR 0001）；`WarsmashModEngine`（AGPLv3）仅借鉴语义、禁止复制 Java / LibGDX 代码（`openspec/config.yaml` 的 `rules.design` 硬要求）。**本机 `vera20k` HEAD `590e87068` 偏离 `UPSTREAM.md:11` 固定提交 `305ea3a75`（固定提交是祖先、领先 530 commits）**，T1 锚按固定提交 `git show` 只读核对、未改矩阵、如实登记（C19）。

**MODIFIED delta 的身份键说明（按 warning 从 delta 顶部移入本节；按 critic R-1 先例另在各 MODIFIED requirement 正文下加 `>` 注记，使说明随归档并入主 spec）**：`openspec validate --strict` 要求 `## MODIFIED Requirements` 块保留主 specs 既有 `#### Scenario:` 标题作为归档合并身份键、不得重命名（重命名被解析为「删除旧 + 新增」而拒绝）。因此 6 份 MODIFIED delta（`faction-definitions` 1 条 / `melee-match` 2 条 / `human-faction` 4 条 / `orc-faction` 4 条 / `gameplay-systems` 1 条 / `command-hotkeys` 2 条，共 **14 条 MODIFIED requirement**）的「内核零 diff」「哈希与快照字节不变」「重开干净与快照版本不变」「内容哈希随新增定义变化而非新增权威状态」「内核命令与效果类型集合不变」「不新增运行时权威状态」「状态哈希 / 快照 / golden hash 不变」「复活新增权威状态遗漏进哈希被拦截」等**历史标题字面保留**，其口径已在 WHEN/THEN 内按用户 2026-11-20 拍板方向 A 改为「P8.5b 按 ADR 0002 全流程改 `w3-core`、新增 tier 顶层权威状态 / 升级队列 / 费用登记同步进 FNV-1a 状态哈希 / 快照 / `first_difference()`、determinism 26 项、golden hash 基线按新实现更新、`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）、`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体、`git diff -- crates/w3-core/` 非空属预期、MUST NOT 沿用旧断言」；历史标题字面**不代表**本 change 的行为断言，口径以 THEN 为准。已存在 spec 的 `## Purpose` 经实测 OpenSpec 1.17.0 delta 机制**不合并**（`specs-apply.js`「A delta Purpose only seeds a spec that does not exist yet」，P8.5a 归档纪要 B / R-A 条实测结论），故本 change 不写 `## Purpose` 覆盖、不假装改了主 spec 正文，历史概述措辞登记为已知限制由 `docs-owner` 后续独立 change 收口（见「## 已知限制 / 边界」）。

**四件套一致性（按 `CODEBUDDY.md §8` 一次成文）**：`proposal.md` 的 Scope / Capabilities（新增 `town-hall-upgrade` + 6 个 Modified Capabilities）↔ `specs/**` 的 9 条 ADDED + 14 条 MODIFIED `#### Scenario:` ↔ 本文件 D1–D8 alternatives /「## 参考项目对照」↔ `tasks.md` 的分组步骤互不打架；Q1–Q8 按「推荐」收敛为 P1=A 单一口径一次成文（**不写「可容纳两种结果」**），但推荐值本身尚未获规格门拍板，若拍板与推荐不同 MUST 按 §8 同步回查四件套并重走规格门，不静默沿用、不冒充已锁定（见「## Open Questions」）。

## Goals / Non-Goals

**Goals:**

- G1 按用户拍板方向 A 改 `w3-core`：新增「主城等级 / tier」**顶层权威状态**（独立、离散 T1/T2/T3、MUST NOT 复用 `PlayerEconomy.upgrades` 布尔旗标或表现层假升级），主城升级后二线单位 / 科技按冻结需求链在确定性命令面解锁生效（R4.1 / R4.3 / R4.4 / 验收 :126）。
- G2 新增 tier / 升级队列 / 费用登记三组权威状态，全部同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q1 推荐 bump 4→5）定义的快照与 `first_difference()`，跑通 determinism 26 项 +「改坏验红」四类退化（Q8），golden hash 基线按新实现如实更新（ADR 0002 / `CODEBUDDY.md §5`）。
- G3 升级费用 / 时间（整数帧）/ 差价 / tier 需求链放 `FrozenDefinitions` 冻结常量、经 `compute_defs_hash` 进 `defs_hash` / `content_hash`，全为合成原创编译期整数常量 `[待核]`、定义层数据驱动、MUST NOT 建筑名白名单（R1.2 / R13.3 / ADR 0001 / ADR 0004 / §14）。
- G4 升级扣费 / 退款事务与队列效果执行链分离、取消 / 失败事务性退款、资源不足显式失败（主 `gameplay-systems/spec.md:19` / R3.1）；整数帧 `remaining_frames: u32` + `BTreeMap` 确定性有序容器，不用墙钟 / `float` / 全局静态字典 / `HashMap` 迭代顺序（ADR 0002 §2-3）。
- G5 四件套一次成文同步修订主 specs 口径（不改主 specs 正文、用 MODIFIED delta 表达）、表现层 HUD `U` 键 / 命令卡升级格位只读派生、与 P8.5a 复活 / P8.2 `orc-faction:39` 边界衔接、参考项目对照结论与锚完整落盘、Q2 条件命中 ADR 0005 / T6 连带。

**Non-Goals:**

- 方向 A′「复用 `upgrades` 布尔旗标 + 静态 tier 表」、B「表现层只读派生假升级」、复活 `spawn` 新实体式的假升级、墙钟 / `float` 升级登记、写死经典数值 / 冒充原版。
- T3 全量升级链、两族完整二线单位与科技清单、差价扣费、命令卡第二键与 `U` 绑定等 B 类 `[待核]` 值（Q4 推荐按 P8.5a Q4「只做祭坛复活最小闭环」先例留后续独立 change，参考项目无权威答案，显式登记为范围外承接，见 `tasks.md` / `decision-memo.md`「## 已知限制 / 边界」）。
- JASS / wtg 触发器 VM（N5）、全科技树、地图编辑器 / RPG 自定义地图 / 剧情战役（N2–N4）、联机底座重做（P4 已交付）、AI × lockstep 新注入架构（Q6 / §0.3 T5）。
- 改活文档 `docs/references/WC3-MELEE-RULES.md`（附录 R4.1 机制锚 / 防误用补锚登记为 `docs-owner` 后续独立 change，Q5 / §14）、改 ADR 正文、改主 `openspec/specs/**` 正文（归档才合并）、改其余 15 个含同类历史口径的主 specs（登记为已知限制）。
- 本阶段不 apply / 不 commit / 不 archive / 不 self-approve / 不回填指纹 / 不改 `crates/` 任何文件；`w3-content` 原始 `f64` 坐标不进哈希 / 快照（ADR 0004）。
- 不据本 change 声称跨平台确定性有新验证或全平台达成（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录 :107-110；Debian 12 v4 / v5 侧逐字节比对仍待人工 / VM，Q7）。

## Decisions

> 每个决策统一列出：决策 / 备选方案（含否决理由）/ **ADR 0002 关联与对跨平台确定性的影响** / 架构原则对照 / 业务规则编号与核实状态（`[已核]` / `[待核]` / `[取舍]`）。**Q1–Q8 按「推荐」收敛为 P1=A 单一口径一次成文**（不写「可容纳两种结果」、不列两分支 tasks），但推荐值本身**尚未获规格门拍板**（`decision-memo.md`「## 待确认项」保留逐条「推荐 + 依据 + 代价 + 备选」原文、规格门 `status: pending`）；若规格门拍板结果与推荐不同，MUST 按 `CODEBUDDY.md §8` 同步回查四件套、由 `spec-steward` 修订后重走规格门。跨平台确定性现状：仅 Windows 11 与 Debian 12 x86_64 / rustc 1.99.1（v3）经离线 golden hash 逐字节比对验证（`ADR 0002` 复核记录 / 主 `cross-platform-determinism/spec.md`）；**v4 / v5 的 Debian 侧比对仍待人工 / VM**，macOS / ARM64 / 其它 rustc 仍待验证，**本 change 不得声称有新验证或全平台达成**。

### D1：主城等级 = 独立 tier 顶层权威状态（承重决策，收敛方向 A）

- **决策**：主城等级 / tier SHALL 作为 `w3-core` 顶层权威状态，按玩家主城实体以稳定 `EntityId` / `PlayerId` 顺序的确定性有序容器（如 `BTreeMap`）登记离散等级 T1 / T2 / T3（R4.1 可升级两次）；升级 / 解锁 MUST 在 `w3-core` 确定性命令面生效，MUST NOT 是表现层只读派生假升级。tier **权威状态**与定义层 tier **需求链冻结常量** MUST 明确区分（后者属定义层内容变化、非新增权威状态字段）。
- **备选方案**：① **A′ 复用 `PlayerEconomy.upgrades: BTreeMap<DefId, bool>` + `Research` 旗标 + 定义层静态 tier 表**（P1 被否）→ **否决**：把离散等级塞进布尔科技旗标语义耦合、后续读取绕、把「主城实体升级」混进「研发布尔集合」，仍改 `defs_hash` / 快照字节，且不能清晰表达逐级解锁前置（C5 / `gameplay.rs:763` / `sim_match.rs:1497-1516`）。② **B 表现层只读派生假升级**（P1 被否）→ **否决**：`w3-core` 仍接受未解锁 `Train`（C4 全量可训练）、无权威 tier 可进哈希 / 快照（C2）、表现层不得反写权威状态（ADR 0002 §5 / C9），降级验收 MUST NOT 静默采用；P8.5a「假复活」同类方向已否（归档 `2026-11-20-p8-hero-revive/decision-memo.md` P1 被否 B）。③ 复用 `complete_production` 的 `entities.spawn()` 生成「新主城实体」表达升级 → **否决**：升级是同一主城实体的等级迁移、不是新实体，`spawn` 会改变 `EntityId` 稳定顺序 / 胜负判定（R11）且与 Warsmash / godot 定义层 tier 语义不符（C13 / C14）。
- **ADR 0002 关联与跨平台确定性影响**：这是**新增权威状态**的内核改动，不是零 diff 型 change；tier 全为整数枚举 / `BTreeMap` 稳定顺序，MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`（见 D5）；不引入浮点 / 墙钟。命令排序键、30 Hz 整数帧、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。**跨平台确定性不因本 change 有新验证或新承诺**：新增状态全走整数编码，与已验证的 Windows / Debian 12 x86_64（v3）同机制；v4 / v5 Debian 侧比对、macOS / ARM64 / 其它 rustc 仍待验证。
- **架构原则**：唯一状态所有者（`RtsMatch` 仍是唯一写入者）、SRP（tier 等级与布尔科技旗标分离）、显式失败不静默降级、正确性 / 确定性优先。
- **规则编号与核实状态**：R4.1（T1→T2→T3 逐级解锁）`[已核]` 机制、R4.3 / R4.4（生产 / 研发建筑）`[已核]`、R11（主基地摧毁影响胜负）`[已核]`、R12.4（回放哈希一致）`[取舍]`；tier 名称 / 链清单 / 数值 `[待核]`、`[取舍]` 合成原创；Warsmash / godot 定义层 tier 语义 `[已核]`（只读对照报告、仅锚机制）。

### D2：定义层 tier 需求链与冻结常量数据驱动（承重决策）

- **决策**：「tier → 解锁二线单位与科技需求链」、升级费用（gold / lumber）/ 时间（整数帧 `remaining_frames: u32`）/ 差价 SHALL 放 `FrozenDefinitions` 的冻结常量（Q3 推荐），经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部（C4 / `gameplay.rs:268-316`）；需求链 MUST 由定义层类别与 requires / upgrade 链数据驱动、按 tier 取需求（借 Warsmash `List<List<CUnitTypeRequirement>>` 语义，C13），MUST NOT 用建筑名字符串白名单 / 硬编码 DefId（R1.2 / R13.3）。全部为合成 / 原创编译期整数常量 `[待核]`、零资产、不含 `f64` / MPQ 派生 / 导出 JSON（ADR 0001 / ADR 0004 / §14）。
- **备选方案**：① 把升级常量放 client 侧合成编译期常量表 → **Q3 被否备选**：不动内核定义与 `defs_hash`，但破坏「权威结算读取同一冻结常量」单一来源、双端一致性风险、`w3-core` 不得依赖 `w3-client`（`CODEBUDDY.md §5`）。② 写死 Warsmash `CGameplayConstants.java` 的 Misc 表值或 godot `CLASSIC_RULES_BASELINE.md:46` 的 `320→705` 升级总价 / 网页与本地表不一致值 / `number(key, fallback)` 猜测值 → **否决**：全部 B 类（值在数据表、代码里没有权威值、`CLASSIC_RULES_BASELINE.md:46` 明令不可把导出总价直接当升级扣款），不得当经典规则 / 冻结进权威定义（§14 / ADR 0001 / ADR 0004 / C17）。③ 用建筑名字符串 `"halt"` / `"keep"` 白名单或硬编码 DefId 判解锁 → **否决**：违反 R1.2 / R13.3、与 P8.0 D1 取舍冲突（C15）。
- **ADR 0002 关联与跨平台确定性影响**：冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部，两端定义不同则明确拒绝恢复（主 `gameplay-systems/spec.md:8,12-15`）；改常量即改 `content_hash()`、golden hash 基线按新实现如实更新。全整数、无浮点舍入的平台差异。定义层内容变化属「定义变化」而非「新增权威状态字段」，须与 D1 的 tier 权威状态在 specs 明确区分。
- **架构原则**：冻结输入边界、DIP（数值读定义层）、数据驱动、零资产、CARP。
- **规则编号与核实状态**：R1.2 / R13.3 `[已核]` + `[取舍]`（定义层类别驱动）、R4.1 / R4.3 / R4.4 `[已核]` 机制、R2 / §14 `[取舍]`、ADR 0001 / ADR 0004 `[已核]`；升级费用 / 时间 / 差价 / 链清单 `[待核]`、Warsmash 表驱动机制 `[已核]`（仅锚机制、不锚金额）。

### D3：升级命令面按 Q2 推荐新增独立 `UpgradeTownHall` 变体（承重决策）

- **决策（按 Q2 推荐定案）**：升级 / 取消升级 SHALL 经 `w3-core` 公开提交入口按冻结整数帧号入队、同源校验、按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；按 Q2 推荐新增独立 `GameplayCommandKind::UpgradeTownHall`（指定主城实体 + 目标 tier）命令变体 + 升级开始 / 完成 / 取消事实事件变体，MUST 同步 `hash_gameplay_command` 追加码 13（`gameplay.rs:932-1011`）/ 校验（`:1013-1037`）/ 穷尽分发（`sim_match.rs:1207-1239`）/ `EventKind` 供只读表现层消费。**连带 ADR 0005 / `PROTOCOL_VERSION` / `net-owner**（条件命中矩阵 T6）：protobuf 字段编号只追加（现 tag 33/34 之后、`reserved 14 to 32` 不变）、bump `PROTOCOL_VERSION` 并 `Reject` 明确拒绝旧版本、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、CI 双平台生成代码字节一致（`docs/decisions/0005-protobuf-payload.md:34-44,84` / C21）；`net-owner` 按 `CODEBUDDY.md §9` 由 leader 编排与 `sim-owner` 串行协作、不并行改同一 owner 文件；按 ADR 0005 自有 `W3N1` + protobuf 契约执行、不借鉴 `vera20k` 线格式（`UPSTREAM.md:88`）。
- **备选方案**：① **复用 `Research` + 扩展 `UpgradeDef`** → **Q2 被否备选**：线格式不变 / 不 bump `PROTOCOL_VERSION` / T6 不命中，但需给 `Research{upgrade}` 塞「主城实体 + 目标 tier」异构语义、与 C5「布尔科技旗标」现状冲突面更大，字段扩张仍可能触发 ADR 0005 schema 变更；对照 P8.5a Q2 拍板「新增独立 `ReviveHero` / `CancelRevive` 变体」先例选独立变体。② 表现层直接发命令绕过 `w3-core` 命令面 → **否决**：违反唯一状态所有者 / ADR 0002 §3 / R12.7。③ 复用 `Train{is_building:true}` 伪装升级 → **否决**：`complete_production` 生成新实体（D1 ③），语义不符。
- **ADR 0002 关联与跨平台确定性影响**：命令稳定排序键、30 Hz 整数帧、同源校验、重放不双重注入全部不变；新变体编码 / 校验 / 分发三处同步保证「接收不等于执行成功」语义一致；ADR 0005 canonical 编码继续全部适用、双端解码出相同 `Command` 即状态一致。新增 `hash_gameplay_command` 码 13 会改变未执行命令哈希 → golden hash 基线按新实现如实更新。跨平台确定性不因本 change 有新验证。
- **架构原则**：SRP（升级命令与训练 / 研发命令语义分离）、OCP、唯一状态所有者、分层依赖（`w3-core` 不反向依赖 `w3-net`）。
- **规则编号与核实状态**：R12.4 / R12.7 `[取舍]`、R4.1 `[已核]`、ADR 0002 §3-5 `[已核]`、ADR 0005 §3 `[已核]`；Q2 推荐「新增独立变体」`[取舍]`、待规格门拍板（非仓库文件锚、人工核对）。

### D4：训练 / 研发按冻结 tier 需求链前置解锁 + 费用事务分离（承重决策）

- **决策**：`enqueue_production` / `research` MUST 在读冻结定义的 `train_frames` / `research_frames` 之外，按冻结 tier 需求链校验当前主城等级：要求 T2 / T3 的二线单位 / 科技在当前 tier 未达时 MUST 显式拒绝、不静默放行、不生成队列项、不改变权威状态（R4.1 / R4.3 / R4.4 / 验收 :126）；升级完成后解锁 MUST 在同一确定性命令面立即生效（C4）。升级扣费提交与队列效果执行 MUST 分离（主 `gameplay-systems/spec.md:19`）：提交前校验 gold / lumber 充足再冻结扣费登记、从 `PlayerEconomy` 确定性扣减；取消队列 / 主城被毁 / 对局失败 MUST 事务性退款并恢复升级登记、MUST NOT 静默丢队列 / 重复扣费 / 生成半升级实体；资源不足 MUST 显式拒绝、不静默免费升级。退款 MUST 与扣费同样进状态哈希 / 快照。
- **备选方案**：① 就地扣费、取消不退款 → **否决**：违反主 `gameplay-systems/spec.md:19` 事务分离与 Warsmash / godot 取消退款语义（C13 / C14）。② 提交与执行合并进同一函数 → **否决**：失去「已提交效果不回滚」的确定性边界、取消时无法精确回退。③ 资源不足静默免费升级 / 未解锁 `Train` 静默放行 → **否决**：显式失败不静默降级（config.yaml context 硬红线）、违反验收线「升级后训练 / 解锁生效」。
- **ADR 0002 关联与跨平台确定性影响**：扣费 / 退款改动 `PlayerEconomy`（已进哈希），退款金额必须是扣费登记的精确逆运算、逐帧一致、MUST 进状态哈希 / 快照；解锁前置判据只读冻结定义 + tier 权威状态，按稳定 `EntityId` 顺序、不引入异步 / 墙钟；共享结算服务仍是唯一写入者（主 `gameplay-systems/spec.md:17-21`）。跨平台确定性与 D1 同机制，不声称新验证。
- **架构原则**：唯一状态所有者、SRP（共享结算服务处理费用 / 效果）、显式失败不静默降级、CARP。
- **规则编号与核实状态**：R4.1 / R4.3 / R4.4 / R3.1 `[已核]`、主 `gameplay-systems/spec.md:19` `[取舍]`（项目契约）、Warsmash / godot 退款语义 `[已核]`（只读对照、仅锚机制）；升级数值 `[待核]`。

### D5：新增权威状态进哈希 / 快照 / `first_difference()` + `SNAPSHOT_VERSION` 处置（承重决策）

- **决策**：tier 顶层权威状态 / 升级队列 / 费用登记三组新增权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()` 差异字段集合；遗漏 MUST 被确定性验收反例断言拦截（ADR 0002 §6-8 / `CODEBUDDY.md §5`）。恢复快照后 SHALL 从帧 N+1 继续、不残留 / 重复升级（R12.4）。MUST 跑 `cargo test -p w3-core --test determinism`（26 项）+「改坏验红」四类退化（Q8）；golden hash 基线按新实现如实更新、不沿用 P8.5a v4 值。**`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5**（连同 `SIM_VERSION`、`snapshot.rs` 常量、`sim_match.rs:2368-2369` 单测硬断言、快照头部校验、golden hash 基线、ADR 0002 复核记录、回放 / 联机双端同步发布同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），依据 `snapshot.rs:8-15`「不提供跨版本迁移」与 `ADR 0002:62-72`（§7 不匹配明确拒绝）、路线图 :138「变更须 ADR 复核」、P8.5a bump 3→4 先例；新增字段 MUST 完整进哈希 / 快照 / `first_difference()`、determinism 26 项全绿。
- **备选方案**：① 新增字段只进哈希不进快照（或反之）→ **否决**：ADR 0002 §6-8 / `CODEBUDDY.md §5` 明确「新增权威状态字段必须同时进入状态哈希与快照」，否则快照销毁重建后 desync。② 保持 `SNAPSHOT_VERSION=4` 不改常量 / 断言「省事」→ **Q1 被否备选**：新增字段在同一版本号下的快照被静默错读、与 ADR 0002 §7「明确拒绝」张力。③ bump 5 但只改常量不改 `SIM_VERSION` / 单测断言 / 回放版本拒绝 → **否决**：`sim_match.rs:2368-2369` 硬断言会红、回放 / 联机版本不匹配不拒绝 → desync 不可见。④ 借 R13.6「只读视图投影」实现层约束放松旁路门禁 → **否决**：`WC3-MELEE-RULES.md:314` 明确该放松只适用于复用既有 `Move` / `Attack` + R1.4 分层判据，MUST NOT 借用于新增权威状态 / 命令 / 事件。
- **ADR 0002 关联与跨平台确定性影响**：本决策即是 ADR 0002 §6-8 的行使。FNV-1a + 显式小端字节编码、稳定 `EntityId` 顺序、`first_difference()` 同序返回首个差异字段全部不变；新增字段按稳定顺序编码。golden hash 离线比对基线（Windows / Debian 12 x86_64 v3 已验证）须按新实现重取、不得沿用 P8.5a v4 值；**v4 / v5 的 Debian 侧逐字节比对仍待 VM / 人工**，macOS / ARM64 / 其它 rustc 仍待验证，不声称全平台达成（Q7）。ADR 0002 复核记录按 Q1 推荐 bump 追加（不删改 §1–§8 正文）。
- **架构原则**：确定性契约（最高硬红线）、唯一状态所有者、显式失败不静默降级。
- **规则编号与核实状态**：R12.4 / R11.5 `[取舍]`、ADR 0002 §6-8 / `CODEBUDDY.md §5` `[已核]`、路线图 :138 `[已核]`、Q1 推荐 bump 5 `[取舍]`、待规格门拍板（非仓库文件锚、人工核对）。

### D6：表现层 HUD 只读派生与 `U` 键 / 命令卡格位（承重决策）

- **决策**：主城升级进度 / 费用 / 剩余帧与 `U` 键、命令卡升级格位 SHALL 只从只读 `WorldView`（必要时扩展只读投影字段）/ 事实事件 / 只读 `Snapshot` 派生（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` 范式，C6），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5）；只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单；格位与键位 MUST 由定义层数据驱动（R13 / R13.2 / R13.3 / R13.7 / R1.2）、具体绑定 `[待核]` 不入库；UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。
- **备选方案**：① 本阶段直接改主 `openspec/specs/**` 正文 → **否决**：任务授权边界、`CODEBUDDY.md §8`（归档才合并 delta）、单 owner 约束。② 给 `EntityView` 加可写字段让 HUD 反推升级状态 → **否决**：表现层不得反写模拟状态（ADR 0002 §5）。③ 照搬 godot 墙钟 `Time.get_ticks_msec()` / `static var` 全局字典 / `float remaining_sec` + `_process(delta)` / `"halt"` 建筑名白名单 → **否决**：C15 / ADR 0002 §2-3 / R1.2 / R13.3，须整数帧 `remaining_frames: u32` + `BTreeMap` + 定义层类别驱动；P8.5a C19 已否同类方向。
- **ADR 0002 关联与跨平台确定性影响**：HUD 全为只读派生、不进权威状态 / 哈希 / 快照，逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与无 HUD 实例一致；只读投影字段不新增权威状态。键位 / 命令卡状态表现层只读，具体绑定 `[待核]` 不冻结进权威定义。跨平台确定性零冲击。
- **架构原则**：DIP / LoD（表现层只读投影）、不另立冲突规则（CODEBUDDY.md 单一权威）、一个 change 一个 owner。
- **规则编号与核实状态**：R10.5 / R1.4 / R13.6 `[取舍]`（只读派生边界）、R13 / R13.2 / R13.3 / R13.7 `[已核]` 框架 + `[待核]` 键位、ADR 0002 §5 `[已核]`、§14 `[取舍]`（活文档流程）。

### D7：主 specs 口径同步修订（B-1 教训）与跨平台口径

- **决策**：主 specs 的「`w3-core` 零 diff / `SNAPSHOT_VERSION` 仍 3 或 4 / golden hash 逐字节不变 / 命令与效果类型集合不变 / MUST NOT 新增权威状态」口径用 6 份 `## MODIFIED Requirements` delta（14 条 requirement）同步修订（归档才合并，本阶段不改主 specs 正文）；Scenario 标题按 strict 要求逐字保留身份键、口径改在 THEN、每条给完整新正文 + `>` 身份键说明（见 Context 末「身份键说明」）。**消除 `orc-faction` / `command-hotkeys` 的正面冲突**（P8.5a 未修订它们、登记为已知限制，B-1 教训）：`orc-faction:30,34,49,68,126` 的「MUST NOT 新增权威状态 / `SNAPSHOT_VERSION` 仍 3 / 变体集合不变 / `git diff` 为空」、`command-hotkeys:32,121,126,131` 的「MUST NOT 新增权威状态字段 / 变体 / 事件、零改动、`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变」硬断言在归档后 requirement / Scenario 规范性层面消除。**其余 15 个含同类历史口径的主 specs + 所有 `## Purpose` 概述文字**（实测 delta 不合并 Purpose）如实登记为已知限制、由 `docs-owner` 后续独立 change 收口，不冒充已全量对齐（Q5）。**与 P8.5a / P8.2 边界衔接**：`orc-faction/spec.md:39` 明确 P8.2「不借 `AOwk` 隐身 / 移速增益这类需新增权威状态的 P8.5 语义」——本 change 与 P8.5a 同属承接这类语义、串行推进，不重做 / 不改写 P8.2 兽族定义与技能契约。
- **备选方案**：① 本阶段直接改主 `openspec/specs/**` 正文 → **否决**：任务授权边界、`CODEBUDDY.md §8`、单 owner。② 只修订 Scenario 不给 requirement 完整新正文 → **否决**：B-1 教训（归档后 requirement 正文残留正面冲突硬断言会被 critic 规格门判 blocker）。③ 重命名历史 `#### Scenario:` 标题以「修正措辞」→ **否决**：strict 判「删除旧 + 新增」而拒（Context 末身份键说明）。④ 本 change 把其余 15 个主 specs 一并全量修订 → **Q5 备选**：登记为待用户拍板项，不预设结论（改动面显著扩大、可能违反单 owner / 不扩范围；见「## 已知限制 / 边界」）。
- **ADR 0002 关联与跨平台确定性影响**：主 specs 口径修订是文档层（纯 spec 文本），不改运行时字节、零冲击；只如实登记跨平台现状（Windows / Debian 12 x86_64 v3 已验证、v4 / v5 Debian 侧待比对、macOS / ARM64 / 其它 rustc 待验证），不据本 change 声称全平台达成。
- **架构原则**：不另立冲突规则、一个 change 一个 owner、四件套一致性（`CODEBUDDY.md §8`）。
- **规则编号与核实状态**：§14 `[取舍]`（活文档流程）、R12.4 `[取舍]`、ADR 0002 §6-8 `[已核]`、Q5 修订范围 `[待确认]`（待规格门拍板）。

### D8：零资产 / 许可证 / 反向验证 / 范围边界（贯穿全部决策）

- **决策**：升级数值 / tier 名称 / 两族升级链 / 二线单位与科技清单 / 键位绑定全为合成原创编译期整数常量或 `[待核]`（B 类，C17）、零资产入库、不含 MPQ 派生 / 真实导出 JSON / 参考项目单位名 / rawcode / 原创数值、MUST NOT 写进 `WC3-MELEE-RULES.md` 当经典规则；`w3-content` 原始 `f64` 坐标不进哈希 / 快照（ADR 0001 / ADR 0004）；`WarsmashModEngine`（AGPLv3）**仅借鉴语义、禁止复制任何 Java / LibGDX 代码**（config.yaml `rules.design` 硬要求，与 GPLv3 组合有条件、引入代码须按 ADR 0001 复核点重评）；`rts_kernel_cs` 的 `double` 路线与「不承诺跨平台确定性」不采用（ADR 0002:12-18 / `UPSTREAM.md:30,84`）；`vera20k`（GPLv3 + Section 7）只读、线格式以 ADR 0005 自有 `W3N1` 为准。新增 / 修改回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照，`CODEBUDDY.md §5`），退化类至少覆盖 Q8 四类（tier 漏接哈希 / 快照、未升级仍接受未解锁 `Train` 的假升级、费用不扣 gold / 取消不退款、升级命令漏接编码 / 穷尽分发）；涉及确定性代码改动 MUST 重跑 `cargo test -p w3-core --test determinism`（26 项）。AI 升级命令复用既有 `SetAi` / `run_ai` 确定性外部命令面、AI × lockstep 注入架构按 §0.3 T5 留后续 change（Q6 / R12.7）；日志 / 错误英文结构化 `key=value`、源码注释中文、提交描述中文（本阶段不 commit）。
- **备选方案**：① 只求回归「全绿」不做反向验证 → **否决**：`CODEBUDDY.md §5` 明确仅全绿不构成验证；② 退化实现旁路模拟 → **否决**：失去判别力（先例 `p8-dual-faction` / `p8-hero-revive` C27）；③ 借「经典数值」名义把 Warsmash / godot 值入库 → **否决**：ADR 0001 / §14 / B 类防误用（C17）；④ 用离屏截图 / 自动化断言替代真窗口人工验收 → **否决**：主 `playable-window-entry/spec.md:168-176` CI 边界、Q4 真窗口清单如实标注未执行；⑤ 本期定义新 AI 注入架构 → **否决**：超范围、T5 未拍板、牵动 `w3-net` / `w3-relay`，违反单 owner 串行约束。
- **ADR 0002 关联与跨平台确定性影响**：零新增依赖 / 冻结定义全整数 → 不引入平台浮点差异；`f64` 停留在冻结边界之外。回归测试只读权威状态、不写内核。跨平台确定性现状沿用 ADR 0002 复核记录，不据本 change 声称全平台达成。
- **架构原则**：只读参考不复制代码、显式失败不静默降级、正确性 / 确定性优先、许可证硬红线、资产零入库。
- **规则编号与核实状态**：R2 / §14 `[取舍]`、ADR 0001 / ADR 0004 `[已核]`、升级数值 `[待核]`、Q4 真窗口人工验收「未执行 / 待人工」`[取舍]`。

## 参考项目对照

> 逐条转写 `reference-researcher` 只读对照报告的**全部结论与证据锚**（报告未落盘，本节是唯一留存机会）。矩阵条目 `docs/references/UPSTREAM.md:81-88`、触发时机 :90-96；四个参考项目固定提交见 `UPSTREAM.md:9-14`，全部只读、不复制代码（ADR 0001）；`WarsmashModEngine` 为 **AGPLv3，仅借鉴设计语义、禁止复制任何代码**（`openspec/config.yaml` 的 `rules.design` 硬要求）。**结论分类**：A 类＝机制 / 语义（可借鉴设计语义，只锚机制不锚数值）；B 类＝纯数值 / 具体键位绑定（四个参考项目不是权威来源，**不得写进 `WC3-MELEE-RULES.md` 当经典规则**）；C 类＝范围 / 架构取舍（参考项目不能「核实」，回到用户拍板，Q1–Q8 待规格门）。分类口径沿用 `docs/references/WC3-MELEE-RULES.md` 附录「参考缺口与防误用清单」（:371-388）与 §14。每条按「项目 / 路径 / 借鉴 / 不借鉴 / 许可证 / 锚」列出。

**命中 / 不命中**：本次命中 **T1**（确定性内核 / 状态哈希 / 命令排序 / 快照，主参考 `vera20k`）、**T2**（帧阶段 / 状态所有权语义，主参考 `rts_kernel_cs`）、**T4**（玩法语义 / 主城升级 / tier / 经典规则，主参考 `WarsmashModEngine`）、**T5**（HUD / 队列 / 数值读 Def，主参考 `godot_warcraft3`）；**T3 仍不命中**（不涉资产格式 / SLK / 地图解析 / 资产管线）；**T6 条件命中**（若 Q2 拍板新增 `GameplayCommandKind::UpgradeTownHall` 变体则按 ADR 0005 只追加 protobuf oneof tag、bump `PROTOCOL_VERSION`、`Reject` 旧版本、`net-owner` 串行协作，牵动矩阵 T6「网络传输 / relay」边界；若 Q2 拍板「复用 `Research`」则线格式不变、T6 不命中。矩阵 `UPSTREAM.md:81-88` 尤其 `:88` T6 行、触发时机 :90-96）。**按 ADR 0005 自有 `W3N1` + protobuf 契约执行、不借鉴 `vera20k` 线格式**（`UPSTREAM.md:88` / `docs/decisions/0005-protobuf-payload.md:34-44,84` / C21）。按 `CODEBUDDY.md §9`「凡 T1–T6 命中的 change 写 `design.md` 之前 MUST 委派 `reference-researcher` 出对照报告」，本报告已产出（T1 / T2 / T4 / T5 结论与锚逐条转写于本节，T6 条件命中按本项目 ADR 0005 自有契约收口、未落盘报告列为人工确认项）。锚 `UPSTREAM.md:83-88`。

### T1 主参考 `vera20k`（GPLv3 + Section 7，只有真复制代码才触发附加条款，本项目不复制）

| # | 路径（`file:line`） | 借鉴（分类） | 不借鉴 / 边界（许可证） | 锚 |
|---|---|---|---|---|
| T1-1 | `src/net/lockstep.rs:1-5,207-235`（lockstep 全状态 checksum）、`docs/Networking.md:478-487` —— **均按固定提交 `305ea3a75` `git show` 只读核对** | **A 类**：逐帧全状态 checksum 是 lockstep 门槛 → 新增权威状态必进哈希（D5 佐证，**门禁立法来源仍是本项目 ADR 0002，见 `decision-memo.md` 归因声明**） | **不借鉴** RA2 专属部分（gamemd 原生地址、INI 规则、SHP/VXL 资产、2D 等距渲染）；线格式以本项目 ADR 0005 自有 `W3N1` / protobuf 契约为准（`UPSTREAM.md:88` T6 边界）；GPLv3 + Section 7 附加条款**只有真的复制其代码时**才随之进入，本项目不复制（ADR 0001 / `UPSTREAM.md:22,28`） | `UPSTREAM.md:83`（矩阵 T1）/ 本会话已确认两路径 `EXISTS` / `decision-memo.md` C12 |

### T4 主参考 `WarsmashModEngine`（AGPLv3，只借语义、禁复制代码、不引 Java / LibGDX）

| # | 路径（`file:line`，相对项目根） | 借鉴（分类） | 不借鉴 / 边界（许可证） | 锚 |
|---|---|---|---|---|
| T4-1 | `core/src/com/etheller/warsmash/viewer5/handlers/w3x/simulation/CUnitType.java:64,80,97,125,130,176,192-193,370-371,438-441`（`upgradesTo` / `requirements` / `requirementTiers` / `List<List<CUnitTypeRequirement>>`） | **A 类**：把升级目标 / 需求 / `requirementTiers` / 分层需求列表放**定义层按 tier 取需求**，解锁需求按 tier 分层存于定义层数据（`List<List<...>>`），非建筑名白名单 → D2 定义层 tier 需求链数据驱动依据 | **AGPLv3 与 GPLv3 组合有条件，只借鉴设计语义、禁止复制任何 Java / LibGDX 代码 / rawcode / 队列对象模型**；引入代码须按 ADR 0001 复核点重评；`melee.jui` 不作对战初始化语义 | `UPSTREAM.md:86`（矩阵 T4）/ 本会话已确认 `CUnitType.java` `EXISTS` / `WC3-MELEE-RULES.md:362`（附录 R4.1 行）/ `decision-memo.md` C13 |
| T4-2 | `core/src/.../CGameplayConstants.java:133-141,224-245`（全部 `miscData.getField*`） | **A 类**：升级 / 复活常量是**表驱动**、代码里没有权威值、值在 Misc 表 | **B 类**：Misc 表里的具体金额 / 秒数 / 倍率全部不得当经典规则、不得冻结进权威定义；`WarsmashModEngine` **AGPLv3 禁复制代码** | `UPSTREAM.md:86` / `WC3-MELEE-RULES.md:373`（B 类缺口）/ 本会话已确认 `CGameplayConstants.java` `EXISTS` |

### T5 主参考 `godot_warcraft3`（MIT）

| # | 路径（`file:line`，相对项目根） | 借鉴（分类） | 不借鉴 / 边界（许可证） | 锚 |
|---|---|---|---|---|
| T5-1 | `docs/roadmap/NEXT.md:66,78,120,143,152`（N1 / F7 主城 Keep、二线单位需 Keep + 科技、`building_upgrade`、生产队列 HUD、:152「数值读 SLK Def，禁止逻辑里写死造价 / 伤害」）；`docs/design/game/ROADMAP.md:118-120`；`packages/gameplay/features/technology/logic/tech_presence.gd:10-15,47,176-240`（定义层 Requires / Upgrade 链、按 Keep / tier 取科技可用性）；`packages/gameplay/features/production/logic/train_queue.gd:15-16,233-247`（差价扣费、HUD 只读队列） | **A 类**：主城 Keep 升级 + 二线单位 / 科技需 Keep + 科技前置（D1 / D4 依据）、`building_upgrade` 语义、定义层 Requires / Upgrade 链（D2 依据）、差价扣费与费用事务（D4 依据）、HUD 只读队列（D6 依据）、数值读 Def 不写死（D2 / D8 依据） | Godot / GDScript / `Array` / `Dictionary` / 节点树实现不复制；**MIT 只读、零资产**（ADR 0001）；其 `Producer` / `TechPresence` 对象模型只借结构；**原创三族单位名 / 数值**（`FACTION_UNIT_ROSTERS.md`）不是经典复刻、只借结构与批次判据 | `UPSTREAM.md:87`（矩阵 T5）/ 本会话已确认 `NEXT.md` / `tech_presence.gd` / `train_queue.gd` / `CLASSIC_RULES_BASELINE.md` 四路径 `EXISTS` / `decision-memo.md` C14 |
| T5-2 | `docs/roadmap/CLASSIC_RULES_BASELINE.md:34,46`（:46 明令「不能把表中总价直接用作升级扣款」） | **A 类（仅登记）**：升级差价须按差额计算、不把导出总价当扣款（D4 依据） | **明确不借鉴**：godot 墙钟 `Time.get_ticks_msec()`、`static var` 全局字典、`float remaining_sec` + `_process(delta)` 直接违反 ADR 0002 §2-3（`step()` 不接受 delta / 墙钟 / 渲染帧率）、本项目须用整数帧 `remaining_frames: u32` + `BTreeMap`；`"halt"` 等建筑名字符串白名单违反 R1.2 / R13.3、须由定义层类别驱动；原创三族名与数值、把导出总价直接当升级扣款（`CLASSIC_RULES_BASELINE.md:46` 明令不可）不借鉴；参照 P8.5a 已否同类方向（归档纪要 C19 / C20） | `UPSTREAM.md:87` / `docs/decisions/0002-determinism.md:30-41` / `WC3-MELEE-RULES.md:79,287` / `decision-memo.md` C15 / 本会话已确认 `CLASSIC_RULES_BASELINE.md` `EXISTS` |

### T2 主参考 `rts_kernel_cs`（MIT）

| # | 路径（`file:line`） | 借鉴（分类） | 不借鉴 / 边界（许可证） | 锚 |
|---|---|---|---|---|
| T2-1 | `docs/FRAMEWORK.md:7,62-65,81-83,111`（状态所有权 / 只读视图 / 帧序 / 首差异字段） | **A 类**：状态所有权单一、只读进度视图只暴露拷贝、快照带内容身份并拒绝旧版本、帧阶段与首差异字段（与 ADR 0002 §5 / §7 一致，D5 / D6 佐证） | **不采用**其 `double` 路线与「不承诺跨平台确定性」（ADR 0002:15,18,78 / `UPSTREAM.md:30,84`）；C# 实现不复制；本项目无其单机语义，此处只借状态所有权 / 只读视图契约 | `UPSTREAM.md:84`（矩阵 T2）/ 本会话已确认 `docs/FRAMEWORK.md` `EXISTS` / `decision-memo.md` C16 |

### 内核现状差距（逐锚 `file:line`，本会话自行打开核实，P8.5a 刚改过 `w3-core`）

| # | 差距 | 证据锚（本项目代码） | 归属决策 |
|---|---|---|---|
| ① | 无主城等级 / tier 权威状态：`RtsMatch` 顶层只到 `revive`、`Snapshot` 顶层只到 `revive`、`FrozenDefinitions` 只有四类定义 + 复活 / 祭坛 / 英雄集合 | `crates/w3-core/src/gameplay.rs:618-679`（`HeroReviveState`）/ `crates/w3-core/src/sim_match.rs:98-121`（构造）/ `crates/w3-core/src/snapshot.rs:79-107`（`Snapshot`）；实测 `sim_match.rs:229` / `gameplay.rs:210-220` / `snapshot.rs:104-107` | D1 |
| ② | `GameplayCommandKind` **12 变体**无升级命令、`hash_gameplay_command` 码 **1–12**、`Train{...}` / `Research{upgrade}` 不能表达主城实体升级 | `gameplay.rs:828-877,932-1036` / `sim_match.rs:1207-1239` / `crates/w3-net/src/proto/w3net.proto:136-150`（`oneof` tag 4–13 + 33/34、`reserved 14 to 32`） | D3 |
| ③ | 训练 / 解锁当前全量可训练：`enqueue_production` / `complete_production` 仅校验可执行与 `train_frames`，`UnitDef` / `UpgradeDef` 无 tier / requires，`FrozenDefinitions` 只有四类定义 + P8.5a 复活 / 祭坛 / 英雄集合 | `gameplay.rs:162-219,268-314` / `sim_match.rs:1104-1128,1535-1577` / `crates/w3-client/src/melee/command_card.rs:240-293` | D2 / D4 |
| ④ | 研发 / 攻防升级是另一套：`Research` 写 `PlayerEconomy.upgrades` 并 `apply_upgrade` | `sim_match.rs:1139-1148,1497-1516,1579-1594` / `gameplay.rs:763`（`BTreeMap<DefId, bool>`） | D1（A′ 否决依据） |
| ⑤ | HUD 无 `U` 主基地升级格位（R13.2 / R13.3 / R13.7）；P8.5a `ReviveHudModel` 只读 Snapshot + events 派生范式可仿照 | `crates/w3-client/src/app.rs:819-883` / `crates/w3-client/src/hud.rs:125-150,467-540`（`ReviveHudModel` :131 / `derive` :144 / :540） | D6 |
| ⑥ | 确定性契约现状 `SNAPSHOT_VERSION=SIM_VERSION=4`、`Snapshot.revive` 顶层权威状态、`compute_state_hash` / `first_difference` 已含 `defs/gameplay/revive`、determinism 26 项、golden hash 已按 P8.5a 重取但 Debian 12 v4 逐字节比对仍待人工 / VM | `snapshot.rs:12-21,104-107` / `sim_match.rs:1680-1897` / `crates/w3-core/examples/golden_hash.rs:12-23` / `ADR 0002` 追加复核记录 :109-110 | D5 |

### A / B / A′ 路径代价与 A / B / C 分类

- **A（独立 tier，已拍板方向）**：满足验收线「升级后训练 / 解锁生效、快照哈希稳定」；代价是新增顶层权威状态 → 按 Q1 推荐 bump `SNAPSHOT_VERSION` 4→5 + 重取 golden hash + determinism 26 项 + 改坏验红 + 修订主 specs「零 diff / 仍 3 或 4 / 逐字节不变」历史口径（`faction-definitions:33-36` / `melee-match:127-141` / `human-faction:50-69` / `orc-faction:30,67-68` / `gameplay-systems:35-37` / `command-hotkeys:119-131`）+ ADR 0002 复核记录追加；若按 Q2 推荐新增命令变体还条件命中 T6 / ADR 0005。**A′（复用 `PlayerEconomy.upgrades` + `Research` 旗标 + 定义层静态 tier 表）/ B（表现层只读派生假升级）为被否备选**（P1），代价与理由见 D1 / `decision-memo.md` P1 /「## 被否方向台账」。
- **A 类（机制 / 语义，可借）**：定义层 tier / Requires / Upgrade 链、按 tier 取需求、读 Def 不写死、差价扣费、HUD 只读队列、整数帧 / 哈希 / 快照 / 首差异字段 / 状态所有权（T1 / T2 / T4 / T5 逐条见上表）。
- **B 类（纯数值 / 键位，`[待核]`，参考项目不能冒充）**：Keep / Stronghold / T3 名称、两族完整升级链、二线单位 / 科技清单、升级费用 / 时间 / 差价、命令卡第二键与 `U` 绑定 —— 全 `[待核]`、只能经本机导出 SLK / MiscGame / 原版客户端核对（`WC3-MELEE-RULES.md:350,373-386` / R13.1 / R13.3 / R13.7 / §14），参考项目不得冒充；本节 T5-2 / T4-2 的「明确不借鉴」逐条对应。
- **C 类（待用户拍板，见 `decision-memo.md` Q1–Q8）**：Q1 版本 bump、Q2 命令面、Q3 冻结常量归属、Q4 本期范围、Q5 活文档 / 主 specs 修订范围、Q6 表现层 / AI、Q7 golden hash / 跨平台边界、Q8 改坏验红四类退化 —— 参考项目不能「核实」，按 `UPSTREAM.md:96` 第 5 条回到用户拍板、不臆造。

### 附录锚复核与防误读

- `WC3-MELEE-RULES.md:362` 的 Warsmash `CUnitType.java:97,130,193,438-441` 与 godot `NEXT.md:78,120,143` **仍成立**（本会话已确认两文件 `EXISTS`），但 **`NEXT.md` 本身只证 N1 / Keep 收尾节奏、不证 tier 数据结构**（由 Warsmash `CUnitType.java` + `tech_presence.gd` 补证）；附录表写 `R4.1 :137`，当前正文在 **`:141`**，本 change 的 design / spec **MUST 引用规则编号 R4.1、不引用 `:137` 旧行号**（C18）。该差异只在本节 / `decision-memo.md` 如实登记、本轮不改活文档（Q5 / §14）。
- **vera20k HEAD 偏离如实登记（C19）**：本机 `vera20k` HEAD `590e87068` 偏离 `UPSTREAM.md:11` 固定提交 `305ea3a75`（本会话命令实测：固定提交是祖先、领先 **530 commits**）；T1 锚按固定提交 `git show` 只读核对、未改矩阵、如实登记为已知限制；`UPSTREAM.md` 的 2026-11-17 升级复核小节结论（T1 / T6 路径在新 HEAD 下仍存在、许可证复核不影响本项目合规姿态）不因本 change 改写。
- **`decision-memo.md` 归因声明**：C1「新增权威状态须同步进哈希 / 快照 / `first_difference()` / determinism 26 项 / 改坏验红」硬门禁出处是**本项目** ADR 0002 §6-8 + `CODEBUDDY.md §5` + 主 `gameplay-systems/spec.md:4` / `melee-match/spec.md:129`，**不是 `vera20k` 或任何参考项目**；T1-1 的 vera20k「全状态 checksum」是 lockstep 语义佐证、不是本项目门禁的立法来源。参考项目来源锚均指向 `UPSTREAM.md` 矩阵条目或参考项目内可核验路径。

**锚可打开性（本会话抽样；不写本机绝对路径，改引 `docs/references/UPSTREAM.md:11-14` 矩阵条目，ADR 0001）**：`UPSTREAM.md:11-14` 登记的四个参考项目根（相对路径见该矩阵条目）全部存在；`vera20k/src/net/lockstep.rs` / `vera20k/docs/Networking.md` / `godot_warcraft3/docs/roadmap/NEXT.md` / `godot_warcraft3/packages/gameplay/features/technology/logic/tech_presence.gd` / `godot_warcraft3/packages/gameplay/features/production/logic/train_queue.gd` / `godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md` / `rts_kernel_cs/docs/FRAMEWORK.md` / `WarsmashModEngine/.../CUnitType.java` / `WarsmashModEngine/.../CGameplayConstants.java` 共 9 个代表锚逐个 `EXISTS`（本会话只读核实）；`reference-researcher` 报告未落盘（人工确认项），逐锚全开由 `critic` 按清单第 12 项复核；行号基于 `UPSTREAM.md:9-14` 固定提交，可能微偏移、语义按报告核对。

**参考缺口处置（按 `UPSTREAM.md:96` 第 5 条如实登记、回到用户拍板、不臆造）**：四个参考项目**都没有**「独立 tier 顶层权威状态 + FNV-1a + 版本化快照 + 确定性命令面升级锁」的完整答案——`WarsmashModEngine` 是 Java / LibGDX 单机（无本项目 ADR 0002 权威内核契约）、`godot_warcraft3` 无独立权威内核（ADR 0002:16）、`vera20k` 是 RA2 无 WC3 主城 tier 语义、`rts_kernel_cs` 明确不承诺跨平台确定性；**B 类经典数值 / 键位无权威来源**（Warsmash 值在 Misc 表、godot 网页与本地表不一致、`CLASSIC_RULES_BASELINE.md:46` 明令不可把总价当扣款、本项目 `[待核]` 须本机导出 / 原版客户端核对，§14 / ADR 0001 / ADR 0004）；参考项目自己也有未完成项（`NEXT.md` 只证收尾节奏、`CLASSIC_RULES_BASELINE.md` 自列未完成清单）。因此本 change 的确定性契约、快照版本、命令变体、常量归属、范围取舍、活文档修订、AI × lockstep 架构、反向验证设计全部作为 **C 类 Q1–Q8** 于规格门经用户拍板（`decision-memo.md`「## 待确认项」保留逐条「推荐 + 依据 + 代价 + 备选」）；参考结论不替代拍板、拍板结果由人工核对（非仓库文件锚）、不臆造「已落库文档」。

## Risks / Trade-offs

- **[Risk] golden hash 基线必变、`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5** → Mitigation：按新实现如实更新、不沿用 P8.5a v4 值；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言 `sim_match.rs:2368-2369`、golden hash 基线、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）并按路线图 :138 走 ADR 复核；新增字段 MUST 完整进哈希 / 快照 / `first_difference()`、determinism 26 项全绿。主 specs 6 份 MODIFIED delta 同步修订旧口径（归档才合并）。
- **[Risk] `w3-core` 改动的确定性风险（新增权威状态 / 命令 / 事件漏接）** → Mitigation：D5 全流程（FNV-1a + 显式小端 + 稳定 `EntityId` 顺序 + `first_difference()`）、determinism 26 项 +「改坏验红」四类退化（tier 漏接哈希 / 快照、未升级仍接受未解锁 `Train` 的假升级、费用不扣 gold / 取消不退款、升级命令漏接编码 / 穷尽分发）、共享结算服务唯一写入者；`CODEBUDDY.md §5` 反向验证证据（反例元断言或 A-B 对照）留档（Q8）。
- **[Risk] ADR 0005 protobuf / `PROTOCOL_VERSION` 连带（Q2 条件命中 T6）** → Mitigation：按 Q2 推荐新增独立 `UpgradeTownHall` 变体，字段编号只追加、`reserved 14 to 32`、bump `PROTOCOL_VERSION`、`Reject` 明确拒绝旧版本、canonical 约束继续适用、CI 双平台生成代码字节一致（`docs/decisions/0005-protobuf-payload.md:34-44,84` / C21）；owner 牵动 `sim-owner` + `net-owner` 按 `CODEBUDDY.md §9` 串行协作、不并行改同一文件；按 ADR 0005 自有 `W3N1` + protobuf 契约执行、不借鉴 `vera20k` 线格式（`UPSTREAM.md:88`）。
- **[Risk] 主 specs 同步修订的口径张力（B-1 教训：`orc-faction` / `command-hotkeys` 正面冲突已用 MODIFIED delta 消除；其余 15 个主 specs 与所有 `## Purpose` 仍为已知限制）** → Mitigation：本轮 6 份 MODIFIED delta（14 条 requirement）逐字保留全部历史 `#### Scenario:` 身份键、口径改在 THEN、requirement 正文给完整新正文，归档后在规范性层面消除「必须新增 tier 权威状态」与「禁止新增权威状态 / 变体集合不变 / `git diff` 为空 / `SNAPSHOT_VERSION` 仍 3 / golden hash 逐字节不变」并存（见 `specs/orc-faction/spec.md` / `specs/command-hotkeys/spec.md`）；已存在 spec 的 `## Purpose` 经实测 OpenSpec 1.17.0 delta 机制**不合并**（P8.5a B / R-A 条实测结论），概述性文字历史措辞**如实登记为已知限制**，由后续 `docs-owner` 独立 change 收口，不静默当作已消除、不假装改了主 spec 正文（见「## 已知限制 / 边界」）。其余 15 个含同类历史口径的主 specs 未纳入本次修订、按 P8.5a 先例登记为已知限制、不冒充已全量对齐；是否本 change 一并全量修订属 Q5 待用户拍板项、不预设结论。请 `critic` 按清单第 7 / 10 项复核。
- **[Risk] B 类经典数值 `[待核]` 与活文档附录补锚** → Mitigation：升级费用 / 时间 / 差价 / tier 名称 / 两族升级链 / 二线单位与科技清单 / `U` 键绑定全为合成原创编译期整数常量 `[待核]`、不写死、不冒充原版、不入库（D2 / D8 / B 类防误用 C17）；`WC3-MELEE-RULES.md` 附录 R4.1 机制锚 / 防误用清单补锚、`R4.1 :137` 旧行号与正文 :141 的差异按 Q5 推荐登记为 `docs-owner` 后续独立 change 待办（§14 / C18），本轮不改活文档。
- **[Risk] 参考项目对照报告未落盘、锚未逐锚全开、vera20k HEAD 偏离** → Mitigation：本文件「## 参考项目对照」完整转写全部结论与锚、`decision-memo.md` C12–C20 转写关键锚；本会话抽样打开 9 个代表锚 + 4 个参考项目根，逐锚全开由 `critic` 清单第 12 项复核；参考缺口（C24）不臆造、回到用户拍板；本机 `vera20k` HEAD 偏离固定提交 530 commits 如实登记、T1 锚按固定提交 `git show` 只读核对、未改矩阵（C19）。
- **[Risk] Warsmash AGPLv3 / `rts_kernel_cs` double / 资产入库误用** → Mitigation：D8 硬红线（只借语义禁复制代码、`double` 不采用、零资产、`f64` 不进哈希快照）；`WarsmashModEngine` 引入代码须按 ADR 0001 复核点重评，本 change 不引入。
- **[Risk] T3 全量链 / 多队列 / AI × lockstep 范围蔓延** → Mitigation：按 Q4 / Q6 推荐留后续独立 change、显式登记为后续 change 待办（T3 全量链 / 两族完整清单 / 差价 / `U` 键绑定 / AI × lockstep 升级命令注入架构，见 `tasks.md` 与「## 已知限制 / 边界」）；路线图 :121 串行推进、单 owner 约束、不冒充本期已解决。
- **[Risk] 真窗口人工验收 / GUI 未验证项** → Mitigation：`tasks.md` 真窗口清单逐项 `- [ ]`「未执行 / 待人工」、不提供替代、不冒充通过（C23 / Q4 先例）；先离屏自证 + CPU 回归全绿再请人工验收（GUI 类修复纪律）；CI MUST NOT 声称已验证 GPU。
- **[Risk] 规格门 `status: pending` 令 `verify_all.py` 变红** → Mitigation：`tools/verify_all.py::spec_gate_check()` 判 `invalid spec gate status` 是 fail-closed 正确信号（C22），本阶段不绕过 / 不改门禁 / 不 self-approve / 不回填指纹；`python tools/verify_all.py --fingerprint p8-townhall-upgrade` 只读重算指纹如实记录、不回填门禁。Q1–Q8 待规格门拍板，四件套按推荐收敛为单一口径但不冒充已锁定。

## 已知限制 / 边界（如实登记，不冒充已解决、不扩范围）

与 `decision-memo.md`「## 已知限制 / 边界」逐条对应。本节只登记、不扩范围改主 specs 正文 / 活文档 / ADR / `crates/`。

- **R-1（`## Purpose` 概述文字与其余 15 个主 specs 历史口径）**：主 `openspec/specs/**` 的 `## Purpose` 经实测 OpenSpec 1.17.0 delta 机制不合并（P8.5a B / R-A 条：`openspec archive --json` 输出 `warnings: ["... - delta Purpose ignored; ... already has one"]`、`specs-apply.js:245-263`「A delta Purpose only seeds a spec that does not exist yet」）。故 `faction-definitions:4` / `melee-match:6` ADR blockquote / `human-faction:4` / `orc-faction:4` / `command-hotkeys:4,6` / `gameplay-systems:4` 等概述性措辞逐字残留、归档后与 MODIFIED requirement 正文并存，属概述性文字、非 `### Requirement` / `#### Scenario` 规范性断言，不影响 `openspec validate --strict` 零 warning / 归档 / Q1–Q8 落地 / 确定性契约；**如实登记为已知限制**，由后续 `docs-owner` 独立 change 收口，不静默当作已消除、不冒充「归档后主 spec 全文不再并存」。其余 15 个含同类历史口径的主 specs（`dual-faction` / `unit-orders` / `rts-command-hud` / `playable-client` / `playable-window-entry` / `cross-platform-determinism` / `gameplay-command-wire` / `lockstep-networking` / `linux-platform-backend` / `runtime-entity-orders` / `runtime-entity-model-binding` / `hud-faction-selection` / `3d-rendering` / `verification-cli` 等，本会话 Grep 枚举）**未纳入本次修订**，是各自 change 的历史验收断言，按 P8.5a 先例 +「不改主 specs 正文 / 一个 change 一个 owner / 不扩范围」登记为已知限制、不冒充已全量对齐。
- **R-2（Q1–Q8 待规格门拍板、四件套按推荐单一口径）**：四件套按 Q1–Q8「推荐」收敛为 P1=A 单一口径一次成文（D1–D8 不写「可容纳两种结果」、`tasks.md` 不列两分支），但推荐值本身**尚未获规格门拍板**；`decision-memo.md`「## 待确认项」保留逐条「推荐 + 依据 + 代价 + 备选」原文，规格门 `status: pending`、不 self-approve、不回填指纹。若拍板结果与推荐不同（如 Q1 保持 4 / Q2 复用 `Research` / Q4 全量 T3 / Q5 全量修订其余主 specs），MUST 按 `CODEBUDDY.md §8` 同步回查四件套、由 `spec-steward` 修订后重走规格门（其中 Q2 变更会影响 T6 是否命中 / ADR 0005 连带）。不静默沿用、不冒充已锁定。
- **R-3（部分 Scenario 未带 `R-x.y`）**：`specs/town-hall-upgrade/spec.md` 及 MODIFIED delta 中偏确定性 / 反向验证 / 零资产边界的 Scenario 主要锚 ADR 0002 §6-8 / `CODEBUDDY.md §5` / Cx，未逐条补 `R-x.y`——登记豁免理由：这类是确定性契约 / 门禁 Scenario，非玩法数值规则，规则编号 `R-x.y` 的业务规则基线（`WC3-MELEE-RULES.md`）不覆盖；玩法类 Scenario（主城升级 / 训练 / 研发解锁 / 费用事务 / 胜负 / 命令分派）均已带 `R-x.y`。不臆造编号、不冒充全部覆盖。
- **R-4（参考对照报告未落盘 / 锚未逐锚全开）**：`reference-researcher` 报告为本会话 leader 转达、非仓库文件，结论与锚完整转写进「## 参考项目对照」、`decision-memo.md` C12–C20 转写关键锚；**列为人工确认项**（逐锚全开由 `critic` 清单第 12 项复核），不冒充已落盘文档。Warsmash / godot 行号基于 `UPSTREAM.md:9-14` 固定提交、可能微偏移、语义按报告核对。
- **R-5（vera20k HEAD 偏离固定提交）**：本机 `vera20k` HEAD `590e87068` 偏离固定提交 `305ea3a75`（固定提交是祖先、领先 530 commits，本会话命令实测），T1 锚按固定提交 `git show` 只读核对、未改 `UPSTREAM.md` 矩阵、如实登记为已知限制（C19）；`docs-owner` 后续升级复核时显式更新本表并重新核对相关文档。
- **R-6（活文档附录补锚 / 旧行号差异未修）**：`WC3-MELEE-RULES.md` 附录 R4.1 机制锚 / 防误用清单补锚、`R4.1 :137` 旧行号与正文 :141 差异按 Q5 推荐登记为 `docs-owner` 后续独立 change 待办、本轮不改活文档（§14 / C18）；本 change 的 design / spec 只引用规则编号 R4.1、不引用 `:137` 旧行号。
- **R-7（被否台账为 design alternatives 子集）**：`decision-memo.md`「## 被否方向台账」R1–R7 是本文件 D1–D8 备选方案与「## 参考项目对照」的子集 / 交叉引用，非独立新增决策——登记为已知限制（决策留痕用途），不冒充另立独立证据链。
- **R-8（锚精度与交叉引用）**：部分证据锚为范围锚（如 `gameplay.rs:618-679`）或内部交叉引用（C1–C24 / D1–D8 / Q1–Q8 / 表格），未逐条收敛到单行 `file:line`——登记为已知限制，由 `critic` 清单第 12 项在规格门复核实测可打开性；不冒充逐锚单点可核。
- **R-9（ADR 0005 §4「`w3-core` 零改动」）**：`docs/decisions/0005-protobuf-payload.md:49` §4「`w3-core` 零改动、零依赖，不反向依赖 `w3-net` / `w3-relay`」——「`w3-core` 零改动」是 `gameplay-command-wire-format` 那个 change 的**实施期约束**，耐久红线是「不反向依赖上层」的分层边界；本 change 按用户拍板改 `w3-core` 不违反该耐久约束，仍 MUST NOT 反向依赖 `w3-net` / `w3-relay`。登记为范围澄清，不改 ADR 正文（本轮授权边界，P8.5a R-14 先例）。
- **R-10（Q4 / Q6 范围外承接）**：T3 全量升级链 / 两族完整二线单位与科技清单 / 差价扣费 / 命令卡第二键与 `U` 绑定（B 类 `[待核]`）/ AI × lockstep 升级命令注入架构（§0.3 T5）——按 Q4 / Q6 推荐留后续独立 change、`tasks.md` 与 `decision-memo.md` 同步登记，不冒充本期已解决。
- **R-11（跨平台确定性口径）**：仅沿用 ADR 0002 复核记录 Windows / Debian 12 x86_64（v3）已验证结论；**v4 / v5 的 Debian 侧逐字节离线比对仍待 VM / 人工**（`ADR 0002:110` / `golden_hash.rs:12-23`），macOS / ARM64 / 其它 rustc 待验证，本 change 不声称全平台达成、不声称有新验证（Q7）。

## Migration Plan

- **部署 / 回滚**：本 change 是**新增权威状态 + `w3-core` 实现改动**（非纯表现层增量）。无外部数据迁移，但有**快照 / 回放版本契约变更**：`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新）；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝、不提供跨版本迁移（`snapshot.rs:8-15` / ADR 0002 §7 / ADR 0005 §3）；按 Q2 推荐新增命令变体牵动 `PROTOCOL_VERSION` bump 与双端同步发布、无兼容垫片（`0005-protobuf-payload.md:43`）。回滚即还原 `crates/w3-core/` / `crates/w3-client/` / `crates/w3-net/` 改动 + 主 specs MODIFIED 合并结果，golden hash 基线随之回退到拍板前状态；本机导出资产留 gitignored、不随代码回滚。ADR 0002 复核记录按 Q1 推荐 bump 追加（不删改 §1–§8 正文）。
- **顺序（`CODEBUDDY.md §9`，本阶段止于 ①）**：① 提案阶段：`reference-researcher` 对照报告（已产出、未落盘、结论已转写）→ `decision-memo.md` 站稳承重约束 / 拍板决策 / 待确认项 → 四件套一次成文（proposal → specs → design → tasks）→ `openspec validate p8-townhall-upgrade --strict` 零 warning + `check_encoding.py` 全绿 + `--fingerprint` 重算指纹如实记录（不回填）→ ② **规格门**：`critic` 规格门报告 → **用户经 `AskUserQuestion` 逐条拍板 Q1–Q8 并批准**（拍板结果若与推荐不同须先同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户第二轮确认锁定）→ leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`（本轮 `status: pending` 不变、不 self-approve / 不回填）→ ③ `/opsx:apply`（`sim-owner` 为主 owner，`client-owner` / `net-owner` 按 §9 串行协作）→ ④ `critic` 代码门（全清单）→ ⑤ 本地 commit（中文描述）→ ⑥ 人工 push → CI 双平台 `verify_all.py` → ⑦ 真窗口人工清单逐项勾选 → ⑧ `/opsx:archive` 并确认主 specs 合并。
- **归档 delta 去向**：新增 `town-hall-upgrade` 主 capability（`## Purpose` + 9 条 ADDED Requirements）、6 份 `## MODIFIED Requirements` 命中 `faction-definitions`（1 条）/ `melee-match`（2 条）/ `human-faction`（4 条）/ `orc-faction`（4 条）/ `gameplay-systems`（1 条）/ `command-hotkeys`（2 条），共 **14 条 MODIFIED requirement**；主 `openspec/specs/` 本阶段**不变化**（归档时才合并）。归档时逐条登记未勾任务原因、已知限制承接（活文档待办 / 其余 15 个主 specs 历史口径 / `## Purpose` 概述文字 / T3 全量链 / AI×lockstep / 跨平台待验证），主 specs 是否变化据实说明。

## Open Questions

> **本节登记的不是「可安全延后、不改 specs / 方案 / 任务拆解」的未知项**，而是按项目工作流（`CODEBUDDY.md §8 / §9` 规格门由人拍板）**C 类待拍板项 Q1–Q8**（见 `decision-memo.md`「## 待确认项」逐条「推荐 + 依据 + 代价 + 备选」原文）。**四件套据此从「可容纳两种拍板结果」收敛为 P1=A 单一口径**（`specs` 统一写 Q1 bump 4→5 / Q2 新增独立 `UpgradeTownHall` 变体 / Q3 `FrozenDefinitions` / Q4 T1→T2 最小闭环 / Q5 修订 6 个 capability + 其余登记已知限制 / Q6 只读 HUD + 复用 `SetAi`·`run_ai` / Q7 跨平台边界如实登记 / Q8 改坏验红四类退化，`tasks` 不列两分支、不双路径并行），**不再声称四件套可容纳两种结果、不臆造拍板结果、不冒充规格门已锁定**（P8.5a critic R-2 失实声明教训）。Q1–Q8 **尚未拍板**、`status: pending`、不拿沉默当授权。

- **Q1 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 4→5**（推荐）、**Q2 新增独立 `GameplayCommandKind::UpgradeTownHall` 命令 + 升级事件变体**（推荐，条件命中 T6 / ADR 0005）、**Q3 升级费用 / 时间 / tier 需求链放 `FrozenDefinitions` 冻结常量**（推荐）、**Q4 本期 T1→T2 最小闭环 + 训练 / 研发解锁生效范围**（推荐，T3 全量链 / B 类 `[待核]` 值留后续 change）、**Q5 活文档修订登记为 `docs-owner` 后续独立 change 待办 + 主 specs 修订 6 个 capability、其余 15 个登记已知限制**（推荐；是否本 change 一并全量修订其余 15 个属本 Q 的核心张力、交用户拍板、不预设结论）、**Q6 `U` 键 / 命令卡格位 + HUD 只读派生 + AI 升级命令复用既有 `SetAi` / `run_ai` 外部命令面**（推荐，AI × lockstep 注入架构按 §0.3 T5 留后续 change）、**Q7 golden hash 重取与跨平台边界如实登记**（推荐，不声称全平台）、**Q8 改坏验红四类退化设计**（推荐，反例元断言为主）—— 逐条「推荐 + 依据 + 代价 + 备选」原文留痕见 `decision-memo.md`「## 待确认项」、关键锚见 C1–C24；**因规格门 `status: pending` 且 Q1–Q8 未拍板，本轮不回填 `approved` / 指纹、不 self-approve**（C22）。
- 真窗口人工验收「未执行 / 待人工」、GPU 端画面 / 跨平台确定性（v4 / v5 Debian 侧比对 / macOS / ARM64 / 其它 rustc）未验证项如实登记为已知限制（`decision-memo.md`「## 已知限制 / 边界」/ 本文件「## 已知限制 / 边界」），不冒充通过、不提供替代。
