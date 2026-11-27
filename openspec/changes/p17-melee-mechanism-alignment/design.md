# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「一手明文数据核对出的机制性结论怎么以确定性方式正式落进 `w3-core` 的需求规格与回归锁定」。事实来源为**本仓库 artifacts + 用户 2026-11-26 已拍板的规则核对结论 + `reference-researcher` 已产出并核实的参考对照报告 + 仓库外一手明文证据台账（人工确认项）**（逐条带可核验锚，不臆造）：

- **已拍板既成事实**：用户已引入 clean-room 反编译 / 一手明文数据取证流程（ADR 0003 第二次修订 `docs/decisions/0003-scope.md:109-123`、ADR 0001 复核记录 `docs/decisions/0001-license.md:77-83`、p15 / p16 change）；此前产出都停在「流程规范 / 证据脚本 / 文档」、一行游戏代码没动，用户明确不满并要求「真改 `w3-core`」、授权「规格门 → 真改 `w3-core`」走完整 `CODEBUDDY.md §9` 串联。2026-11-26 已按 `WC3-MELEE-RULES.md §14` 第 2 条把 R6.5「满级英雄参与经验分摊」写进规则文档（commit `9f2d3e7`，§15 有该行 `:438`）。本 change 由 `spec-steward` 做 ADR 0003 第三次修订（纯追加，明确范围扩展 / 红线不变 / 1.0 范围不变）。
- **一手明文证据台账（仓库外 / 人工确认项 / 行号可能漂移）**：2026-11-26 两份证据台账（R6 样板 + 推广批）与明文 JASS（`common.j` / `Blizzard.j`）、MiscGame / MiscData / SLK 导出表均**留本机仓库外、不入库、不分发、未逐行核验、不冒充已核验**；事实源为用户合法自备经典客户端经 `godot_warcraft3` 导出。四件套 MUST NOT 写本机绝对路径；「原版具体数值 / 字段名 / rawcode / 技能名」禁令按来源**二分**（critic 规格门 residual-3：把只读参考项目源码佐证锚与本机一手明文取证真值分开，避免代码门 Grep 误伤 `design.md`「## 参考项目对照」G3/G4/G5 表格）：**禁止入库** —— 本机一手明文取证字段名 / 四字符 rawcode / 技能名 / 具体数值（留仓库外、不进 `crates/w3-core` 冻结定义，与 `WC3-MELEE-RULES.md:368,369,382-387` 既有体例一致）；**允许引用** —— 只读参考项目源码佐证锚与防误用清单登记值（仅可在 `design.md`「## 参考项目对照」引用，不作 WC3 数值事实源、不冻结进权威定义、不随发行物分发；G3/G4/G5 表格锚保留、不删）。本二分统摄四件套（`proposal.md` 明确不做 / 本文件 / `spec.md` 各 Scenario / `tasks.md`）所有同类「不写 / 不含 / Grep 不出现」禁令措辞，语义不变；6.2 的 Grep 验证范围据此收敛为「一手明文取证产物 + `crates/w3-core` 冻结定义 + 仓库内新增代码」、不扫对照表。证据只作**机制锚**，不是 ADR 0002 口径「可冻结、机器可判、可进 CI 的业务逻辑 oracle」，MUST NOT 进权威状态 / 哈希 / 快照 / golden hash / determinism / CI（ADR 0001 / 0002 / `CODEBUDDY.md §5-6`）。
- **需求源与口径**：`docs/references/WC3-MELEE-RULES.md` 的 `R-x.y`（R6.1 经验 `:167` / R6.3 复活 `:169` / R6.5 分摊 `:172` / R7.4 售卖 `:182` / R8.5 leash `:194` / R5.3 克制矩阵 `:157`）、附录「核实约定」`:353`、§14 第 2 条 `:421`、§15 `:426-438`、附录「参考缺口与防误用清单」`:374-391`。**`## 11`（`:233`）是「胜负与结束」章节，不是核实约定**；`:172` / `:391` 已有的「§11 核实约定」属历史误称（成文表述瑕疵），本 change 只在 `decision-memo.md` 登记、不静默改规则文档正文（§14 第 2 条），四件套一律引用「附录核实约定 / `:353`」。
- **代码现状（本轮只读核实，逐条带锚）**：`crates/w3-core/src/sim_match.rs:4021-4050` R6.5 两环节已实现（第一环节范围 + 存活 + 己方英雄进 `BTreeSet` 稳定序 `xp_candidates`；第二环节 `total = script.xp * hero_level_xp_factor / 100`、`share = total / count`、逐英雄 `state.xp += share` 写入 `xp_credited`），但 `:4035-4036` 注释仍是旧 `[待核]` 措辞「本期不冻结该规则，所有集合内英雄统一入账」——**代码行为已与拍板一致、注释过时、无回归锁定**。`creep.rs:126-155` `CreepConstants`（`xp_share_radius_raw: 10 * SCALE` / `hero_level_xp_factor: 100` 为合成占位 `[待核]`）、`:171-181` 进 `hash()`、`:348-363` `KillReward` 的 `xp_candidates` / `xp_credited`、`:528-547` 进 `hash()`、`:576` `first_difference()`（`:611` 为 `kill_rewards` 差异分支）。`gameplay.rs:46-71` `ReviveConstants`、`:74-87` `SYNTHETIC_REVIVE_CONSTANTS` 合成占位、`:104-123` `cost()` **已实现**「基数 + 等级系数 → 绝对上限」公式族（`i128` 中间量、整数截断、无浮点）、`:125-139` 进 `hash()`、`:43` 注释「酒馆按 Q4 不在本期范围」（买活半血无蓝未实现）。
- **三版本号与四处同步**：`crates/w3-core/src/snapshot.rs:67-68`（`SNAPSHOT_VERSION` / `SIM_VERSION` == 9）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION` == 9）；`crates/w3-core/src/sim_match.rs:4320` `compute_state_hash` / `:4399` `first_difference` / `:4481` `capture_snapshot` / `:4581` `restore_snapshot` 四处同步（ADR 0002 §6 / §7）。本 change 首期零新增权威状态、不 bump。
- **Windows 写文件纪律**：本轮用 Write 工具字节级写入（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格），禁止 `Path.write_text()` / `open(..., "w")` 默认模式（会把 LF 转 CRLF、破坏四件套指纹，`CODEBUDDY.md §2` / 历史 `p9-undead-foundation` CRLF 漂移教训）。

**参考矩阵命中判定**：本 change 命中 `docs/references/UPSTREAM.md:104-114` 矩阵 **T1（主：机制落 `w3-core` + 权威状态四处同步）、T4（主：R6.5 / R6.1 / R6.3 / R7.4 / R8.5 / R5.3 玩法语义）**，T7 仅对 T1 架构方法论可选交叉（对 T4 玩法语义 MUST NOT 参考），T2 边缘（leash 只对齐机制结构），T3 / T5 / T6 不命中。`reference-researcher` 已产出并核实的「参考对照报告」（G1–G7，五个固定提交存在、锚按固定提交 `git grep` 核实、非逐行全量核验、`vera20k` / `ConcordRTS` 工作区行号可能漂移）由用户本会话提供，本轮**直接逐条引用**到「## 参考项目对照」，不再重复委派；判定待 `critic` 清单第 12 项复核。

## Goals / Non-Goals

**Goals:**
- 新增单一 capability `verified-melee-mechanisms`，用 SHALL/MUST + 带可执行 Grep / 测试验证的 `#### Scenario:`（WHEN/THEN）定义 8 条 requirement：R6.5 满级参与分摊 + 注释订正、经验两环节 / 稳定序 / 均分、布尔与因子机制合成占位、R6.3 复活公式族上限、确定性回归 + 反向验证 + determinism 重跑、证据 / 数值 / 权威状态 / 资产边界、参考缺口顺延、规则文档同步与表述瑕疵登记。
- 把「首期范围 = 推荐最小闭环 vs 全量方案」「布尔 / 因子机制是否首期落地」「五类参考缺口处置」「反向验证形态」「批准记录」参数化承接为规格门待确认项（spec 用行为式 SHALL、tasks 用条件式措辞），拍板二选一时**不推翻 spec requirement 与 task 结构**。
- 显式声明首期**零新增权威状态字段 / 零新增 `GameplayCommandKind` / 不 bump 三版本号 / 不连锁 `w3-net` / `w3-client` / `w3-relay`**；若规格门改判全量方案动权威状态字段，MUST 四处同步 + bump + golden hash 重取 + `cargo test -p w3-core --test determinism` + 反向验证。
- 把 G1–G7 逐条（项目 / 路径 / 借鉴 / 不借鉴含许可证理由 / 锚）落进「## 参考项目对照」，红线边界逐条声明、五个只读参考项目不变、参考缺口如实登记回到人拍板。

**Non-Goals:**
- **本轮只成文**：不改 `crates/**`（含 `w3-core` / `w3-net` / `w3-client` / `w3-relay`）、不改 `docs/references/WC3-MELEE-RULES.md` 正文、不改主 `openspec/specs/**`、不改 `tools/**` / CI / 权限配置 / p12–p16 / 归档件 / ADR 0001 / 0002 / 0004 / 0005 正文（ADR 0003 仅文末纯追加第三次修订）。
- **不纳入（顺延 p18 / 已知限制）**：R6.1 经验阈值分档表改 `HeroState` 字段、R5.3 克制矩阵新建 capability 与倍率、R7.4 回购价公式、R8.5 三级 leash 与归巢半径 / 触发阈值、R6.3 买活半血无蓝、L2 反编译补 R3.2 / R3.3 upkeep；冻结任何原版真值；新增权威状态 / 命令 / 事件；bump 三版本号；golden hash 重取；接 CI；把证据当业务逻辑 oracle；登记第六个参考项目。
- 不 archive / 不 commit / 不 push / 不 fetch / 不 pull / 不 clone / 不勾任一复选框 / 不 self-approve 规格门 / 不写「## 省略说明」 / 不填 `status: approved` / 指纹到任何 decision-memo；真窗口人工验收 8.x、golden hash 双平台、CI 双平台一律「待人工 / 待 VM」。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、**对跨平台确定性的影响（ADR 0002）**、分层依赖 / 架构原则、玩法决策所依据的 `R-x.y` 与核实状态（`[已核机制]` / `[待核]` / `[取舍]`）。全部决策对 ADR 0002 契约的影响逐条判定；`[待核]` 数值一律不冻结进权威定义。

### D1 首期范围：推荐最小闭环，全量方案顺延 p18（规格门待确认项）

- **决策**：按「推荐 + 依据 + 代价 + 备选」给规格门拍板。**推荐最小可验收闭环**：只改 `sim_match.rs` 结算逻辑与注释、给 `gameplay.rs` / `creep.rs` 已实现机制加确定性回归；不新增 / 不修改权威状态字段集合、不新增 `GameplayCommandKind`、不 bump 三版本号、不连锁 `w3-net` / `w3-client` / `w3-relay`。
- **备选方案（被否）**：① **全量方案**（R6.1 经验阈值分档表 + 递推公式改 `HeroState` 字段、R5.3 克制矩阵新建 capability、R7.4 回购、R8.5 三级 leash、R6.3 买活半血无蓝、L2 反编译补 R3.2 / R3.3 upkeep）——被否：改权威状态结构、需 bump 三版本号与 golden hash 重取、连锁 `w3-net` / `w3-client` / `w3-relay`，风险与工作量显著更大；② **本轮直接改 `crates/` 代码**（被否：任务边界明确本轮只成文、apply 是规格门批准后的 `sim-owner` 独立会话，一个 change 一个 owner）；③ **继续只做流程 / 证据脚本 / 文档**（被否：用户明确不满「一行游戏代码没动」、要求真改 `w3-core`）；④ **直接把证据台账 / 明文一手数据写进仓库当 oracle**（被否：ADR 0001 零资产 + ADR 0002 证据不进门禁，且台账为仓库外人工确认项）。
- **依据**：C1（用户本会话授权，`decision-memo.md`）/ `CODEBUDDY.md §9`（串联顺序）/ `docs/references/WC3-MELEE-RULES.md:353,374-391`（`[待核]` 不冻结、参考缺口）/ ADR 0001 / 0002 / 0004。
- **代价**：首期不覆盖 R6.1 阈值分档、R5.3 克制矩阵、R7.4 回购、R8.5 三级 leash、R6.3 买活等，如实登记为已知限制 / 顺延，不冒充已对齐。
- **规则编号与核实状态**：R6.1 `[待核]`、R6.3 `[待核数值 / 已核机制]`、R6.5 `[已核机制 / 待核数值]`、R7.4 `[待核]`、R8.5 `[待核]`、R5.3 `[待核]`（`WC3-MELEE-RULES.md:157,167,169,172,182,194`）。
- **对跨平台确定性的影响（ADR 0002）**：**首期零影响**——零新增权威状态、不改命令排序 / 状态哈希编码 / 帧序、不 bump 三版本号（仍 9）；但真改确定性代码，MUST 重跑 `cargo test -p w3-core --test determinism` + 反向验证。全量方案若被规格门改判，MUST 四处同步 + bump + golden hash 重取；macOS / ARM64 / 其它 rustc 仍属待验证项、不声称达成。
- **分层依赖 / 架构原则**：正确性 / 确定性优先、唯一状态所有者、不拿沉默当授权、参数与策略分离（OCP，二选一不推翻四件套结构）。

### D2 R6.5 注释订正与结算行为锁定（代码行为不变）

- **决策**：把 `crates/w3-core/src/sim_match.rs:4035-4036` 注释订正为「2026-11-26 按 §14 核对：满级英雄参与分摊（见 §15）」，**结算行为保持不变**（范围 + 存活 + 己方英雄进集合、`total / count` 均分、逐英雄入账 `xp_credited`）；新增 CPU 回归把「满级英雄参与分摊」锁死防退化。
- **备选方案（被否）**：① **继续标 `[待核]` 不冻结**（被否：2026-11-26 已拍板参与分摊，commit `9f2d3e7` / §15 `:438`）；② **改结算代码把满级英雄排除**（被否：与拍板结论相反、会令回归失败）；③ **静默改写 `WC3-MELEE-RULES.md` 正文**（被否：§14 第 2 条「不静默改规则」，用户明确本 change 不改正文）；④ **只订正注释不加回归**（被否：`CODEBUDDY.md §5` 要求反向验证、仅注释订正无防退化价值）。
- **依据**：`docs/references/WC3-MELEE-RULES.md:172`（R6.5）/ `:438`（§15）/ `:421`（§14 第 2 条）；commit `9f2d3e7`；`crates/w3-core/src/sim_match.rs:4021-4050`；`docs/decisions/0002-determinism.md:55-72`。
- **代价**：注释变更不进状态哈希，回归测试须构造满级英雄缺陷态（排除 / 均分改全额），证据形态① / ② 由 apply 阶段 `sim-owner` 按 §5 定。
- **规则编号与核实状态**：R6.5 `[已核机制 / 待核数值]`——只有「参与分摊」机制结论已核，分摊范围 / 各档比例 / 范围常量全 `[待核]`、用合成占位（`WC3-MELEE-RULES.md:172,353`）。
- **对跨平台确定性的影响（ADR 0002）**：**零状态影响**——注释不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()`；回归断言逐帧 `compute_state_hash()` 与同脚本基线一致，不改命令排序键 / 30 Hz 整数帧 / `SimFixed` Q47.16。
- **分层依赖 / 架构原则**：SRP（注释订正与行为锁定分离）、显式失败（回归反例必失败）、ADR 记「为什么」与 spec 记「该做什么」不打架。

### D3 布尔与因子机制用合成占位表达、不改字段集合

- **决策**：用合成占位值表达三类机制结构（建筑击杀是否给经验的布尔开关、召唤物击杀经验折减因子、按等级分档递减的经验因子），借 G3 的「布尔开关 + 分档表 + 公式族」语义与「进入分摊集合 / 经验入账」两环节分离，**不借真值**；MUST NOT 新增 / 修改权威状态字段集合、不写死原版数值、不含 `f64`。若实现某子项需改字段集合，SHALL 顺延后续 change 并登记已知限制、不臆造字段。
- **备选方案（被否）**：① **冻结参考项目 / 公开攻略的具体数值**（被否：附录核实约定 `:353`、防误用清单 `:374-391`、ADR 0001 / 0004 禁止把 `[待核]` 数值冻结进权威定义）；② **为三类机制新建权威状态字段**（被否：触发 ADR 0002 四处同步 + `SNAPSHOT_VERSION` bump + proto / relay 连锁，超出最小闭环，D1）；③ **用 `f64` 表达折减因子**（被否：ADR 0002 §1 禁止浮点进权威状态 / 快照 / 状态哈希）；④ **照抄 `WarsmashModEngine` 的 Java 常量 / 代码**（被否：AGPLv3 只借语义、禁复制代码，`openspec/config.yaml rules.design` 硬要求）。
- **依据**：G3（`CGameplayConstants.java:220-222,238,426-436` / `CUnit.java:3041,3083,3106-3109`，见「## 参考项目对照」）；`crates/w3-core/src/creep.rs:126-155`；`docs/decisions/0002-determinism.md:25`（禁 `f32` / `f64`）；`docs/decisions/0001-license.md:80-82`。
- **代价**：`[待核]` 合成占位值与原版数值不等价，不冒充经典规则事实（`[取舍]`）；能否在「不改字段集合」前提下落地由规格门待确认项拍板，不预先许诺三子项全落。
- **规则编号与核实状态**：R6.1 `[待核]` / R6.5 `[待核数值]`——机制结构属本项目 `[取舍]`，数值一律 `[待核]`、不冻结。
- **对跨平台确定性的影响（ADR 0002）**：**零影响（前提成立时）**——合成占位编译期整数常量经既有 `CreepConstants::hash()` 进 `defs_hash` / `content_hash`（定义层内容，与新增权威运行态严格区分），不新增权威状态字段即无需进入逐帧状态哈希 / 快照 / `first_difference()`。若改判需新增字段，MUST 按 D7 四处同步。
- **分层依赖 / 架构原则**：冻结输入边界（定义层常量 ≠ 权威运行态）、DIP（机制结构由定义层数据驱动、不写死）、诚实边界（合成占位不冒充原版）。

### D4 复活公式族上限与表驱动（R6.3）只加回归、不改字段

- **决策**：`crates/w3-core/src/gameplay.rs:104-123` 的 `ReviveConstants::cost()` 已实现「基数 + 等级系数 → 绝对上限」（`i128` 中间量、整数除法向零截断、无浮点），本 change 只**补确定性回归**断言上限生效、费用 / 时间随等级确定变化、祭坛与买活参数分离；买活半血无蓝未实现（`:43` 注释「酒馆按 Q4 不在本期范围」）登记为已知限制顺延。
- **备选方案（被否）**：① **本 change 一并实现买活半血无蓝**（被否：超出最小闭环、涉及新参数 / 命令面，D1 顺延）；② **把复活费用改成读本机导出表**（被否：ADR 0001 零资产 / ADR 0002 权威状态只认冻结合成常量）；③ **去掉绝对上限改为线性增长**（被否：与 G4「基数 + 等级系数 + 上限」公式族语义及既有实现相反）；④ **只加正向全绿测试、不做反向验证**（被否：`CODEBUDDY.md §5` 要求缺陷态会失败）。
- **依据**：`crates/w3-core/src/gameplay.rs:43,46-71,74-87,104-123,125-139`；G4（`CGameplayConstants.java:224-246,16,158-160,172,324`）；`docs/references/WC3-MELEE-RULES.md:169,367`（R6.3 / 附录佐证锚）；ADR 0001 / 0002 / 0004。
- **代价**：高等级英雄的截断断言须构造「越过上限」的缺陷态，不改 `cost()` 主体。
- **规则编号与核实状态**：R6.3 `[已核机制 / 待核数值]`——公式族结构（表驱动 / 参数分离 / 上限）属机制锚，具体金额 / 秒数 / 倍率一律 `[待核]`、合成占位（`WC3-MELEE-RULES.md:169,353,367`）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——只新增 CPU 回归，不改 `cost()` 逻辑、不新增权威状态、费用常量继续经 `hash()` 进 `defs_hash` / `content_hash` 快照头部；跨平台确定性待验证项（macOS / ARM64 / 其它 rustc）不因本 change 变化。
- **分层依赖 / 架构原则**：SRP（结算逻辑与回归分离）、冻结定义层驱动、显式失败（反例必红）。

### D5 确定性回归、反向验证与 determinism 重跑

- **决策**：新增 CPU 确定性回归断言「满级英雄参与分摊」「集合内英雄均分」「稳定 `EntityId` 顺序」「复活费用 / 时间公式族上限生效」；MUST 重跑 `cargo test -p w3-core --test determinism` 全绿；MUST 按 `CODEBUDDY.md §5` 形态①（测试内反例 + 元断言防退化）或②（A-B 对照 / 先写失败测试 / 刻意改坏被测代码验红）证明缺陷态失败、修复后通过。无法构造缺陷态的 MUST 在 `design.md` 登记理由与替代证据（本 change 四类断言均可构造缺陷态，故不豁免）。
- **备选方案（被否）**：① **仅「全绿」当验证**（被否：`CODEBUDDY.md §5` 明确仅全绿不构成验证）；② **把反向验证顺延 / 只在 design 写理由**（被否：四类断言缺陷态可构造，不满足「无法构造」豁免条件）；③ **把证据台账 / 离屏对照接进 determinism / CI 门禁**（被否：ADR 0002 证据不进门禁、视觉 oracle 非业务逻辑 oracle）；④ **不重跑 determinism**（被否：`CODEBUDDY.md §5` 任何改动 `crates/w3-core/` 实现 MUST 通过，本 change 真改确定性代码）。
- **依据**：`CODEBUDDY.md §5`；`docs/decisions/0002-determinism.md:69-72`（§8 验收门槛）；`crates/w3-core/tests/determinism.rs`；`crates/w3-core/tests/reverse_validation_*.rs`（既有形态①先例）；`.codebuddy/agents/critic.md:29`（清单第 11 项③）。
- **代价**：新增回归须覆盖四类缺陷态，apply 阶段工作量落在 `sim-owner`；具体形态与缺陷态构造由规格门 / apply 拍板（`decision-memo.md` 待确认项）。
- **规则编号与核实状态**：R6.5 `[已核机制]` / R6.3 `[已核机制]`（回归锁定的是机制结构，不是 `[待核]` 数值）；ADR 0002 契约 `[取舍]`（本项目确定性门禁）。
- **对跨平台确定性的影响（ADR 0002）**：**正向锁定**——回归把 30 Hz 整数帧 / 稳定 `EntityId` 顺序 / 逐帧 `compute_state_hash()` 一致性显式断言，防止未来改动退化；不新增权威状态、不 bump、不改 golden hash 基线（首期零新增字段，无需重取）。
- **分层依赖 / 架构原则**：显式失败、测试本身也要审（`critic` 清单第 11 项③）、诚实边界（不冒充全平台确定性达成）。

### D6 参考缺口一律 `[待核]` + 合成占位、顺延不臆造

- **决策**：R6.1 经验阈值分档、R6.3 复活费用 / 时间、R5.3 克制倍率、R7.4 回购价公式、R8.5 归巢半径 / 触发阈值**均无权威锚**，一律 `[待核]` + 合成占位，登记为参考缺口并顺延后续 change；机制结构（二维克制矩阵 / 货架清单驱动 / 可打断独立 leash 行为）只作对齐登记、不冒充已实现。回到规格门由人拍板（不臆造，`UPSTREAM.md:122` 第 5 条）。
- **备选方案（被否）**：① **用 `WarsmashModEngine` 硬编码常量 / 公开攻略数值补缺口**（被否：G5 / G7 明确「缺真值」「硬编码 `distance<=16f` 带 TODO 不得借鉴」、`WC3-MELEE-RULES.md:378-387` 防误用清单、ADR 0001 / 0004）；② **把缺口写成「已核实」凑完整度**（被否：`CODEBUDDY.md §8` 无证据项 MUST NOT 写成「用户已确认」、参考缺口如实登记）；③ **顺手补 L2 反编译取证真值**（被否：超出本 change 范围、需单独立项走规格门，ADR 0003 第三次修订复核点）；④ **据 `ConcordRTS` 给 T4 玩法语义下结论**（被否：`UPSTREAM.md:114` T3 / T4 / T5 MUST NOT 以其为参考）。
- **依据**：`docs/references/WC3-MELEE-RULES.md:353,374-391`；参考对照报告「参考缺口」段；`docs/references/UPSTREAM.md:114,122`；ADR 0001 / 0004。
- **代价**：首期不承诺这些数值 / 子机制「已对齐」，如实标注为已知限制（沿用「收尾阶段优先记录已知限制而非扩范围」口径）。
- **规则编号与核实状态**：R6.1 / R6.3 / R5.3 / R7.4 / R8.5 全部 `[待核]`（数值侧）；机制结构 `[取舍]`。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——缺口保持 `[待核]`、不写入权威定义、不进 `defs_hash` / `content_hash` / 状态哈希 / 快照；未来若补真值仍只走冻结合成常量路径。
- **分层依赖 / 架构原则**：诚实边界、不拿沉默当授权、冻结输入边界。

### D7 零新增权威状态 / 零新增命令 / 不 bump / 不连锁，明确不新增 `GameplayCommandKind`

- **决策**：首期**不新增 / 不修改权威状态字段集合**（故无需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，当前 9）、**不新增 `GameplayCommandKind`**（`crates/w3-core/src/gameplay.rs:1417`）、不新增事件变体、不 bump 三版本号、不连锁 `w3-net` / `w3-client` / `w3-relay` / `w3net.proto`。**关键理由**：新增 `GameplayCommandKind` 会连带触发 T6 / ADR 0005 的 proto schema 追加 tag 与 `PROTOCOL_VERSION` bump，超出最小闭环；本 change 明确**不新增命令**。若规格门改判全量方案而动权威状态字段，MUST 四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）+ bump 三版本号 + golden hash 重取 + `cargo test -p w3-core --test determinism` + 反向验证，MUST NOT 借本决策旁路门禁。
- **备选方案（被否）**：① **为布尔 / 因子机制新增权威状态字段**（被否：D3、触发四处同步 + bump + 连锁，D1 最小闭环）；② **新增 `GameplayCommandKind` 表达建筑击杀 / 召唤物击杀经验**（被否：连带 T6 / ADR 0005 proto schema 与 `PROTOCOL_VERSION` bump、`w3-net` / `w3-relay` / `w3-client` 连锁，用户明确「请 design.md 明确本 change 不新增命令」）；③ **顺手 bump 9→10「保险起见」**（被否：无新增字段 / 命令，bump 会让旧 v9 快照 / 回放按 `UnsupportedVersion` 拒绝，属无谓破坏）；④ **把证据台账 / `[待核]` 数值写进 `FrozenDefinitions` 进快照头部**（被否：ADR 0001 / 0002 证据不进门禁、数值不冻结）。
- **依据**：`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`、`crates/w3-core/src/gameplay.rs:1417`、`crates/w3-core/src/sim_match.rs:4320,4399,4481,4581`；`docs/decisions/0002-determinism.md:62-72`（§7 / §8）；`docs/decisions/0005-protobuf-payload.md`；`docs/references/UPSTREAM.md:111`（T6）；`openspec/config.yaml:13`。
- **代价**：全量方案的所有改动顺延 p18；本决策须在 `critic` 代码门以 `git diff -- crates/w3-net/` / `-- crates/w3-client/` / `-- crates/w3-relay/` 为空、三版本号仍 9 自证。
- **规则编号与核实状态**：ADR 0002 / ADR 0005 契约 `[取舍]`（确定性 / 版本门禁）；不直接承接 `R-x.y` 实现，仅约束其落地方式。
- **对跨平台确定性的影响（ADR 0002）**：**零影响且为硬边界**——不改 `step()` / `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序 / 状态哈希编码 / 快照字段 / 单 LCG；跨平台确定性待验证项（macOS / ARM64 / 其它 rustc）不因本 change 变化、不声称达成。
- **分层依赖 / 架构原则**：唯一状态所有者、w3-core 不反向依赖上层、冻结输入边界、显式失败、ADR 0005 canonical 约束。

### D8 规则文档同步待办、成文表述瑕疵只登记、规格门 pending

- **决策**：玩法 change 收尾按 `WC3-MELEE-RULES.md §14` 第 5 条在 `decision-memo.md`「## 规则文档同步」给结论、apply / 归档前更新附录「规则 → 实现覆盖对照」对应行与 §15；本轮 `status: pending` / 纯成文，暂写「本轮不需同步、不改正文、apply 阶段落实」待办结论、保持非空。`:172` / `:391` 的「§11 核实约定」历史误称（`## 11` `:233` 实为胜负与结束、准确锚是附录「核实约定」`:353`）**只登记为成文表述瑕疵、不静默改规则文档正文**（§14 第 2 条）；四件套一律引用「附录核实约定 / `:353`」。「## 规格门」严格只有 `- status: pending` + 说明、不填指纹 / `approved_by` / `date` / `reason`、不写「## 省略说明」、不 self-approve。
- **备选方案（被否）**：① **本轮就改 `WC3-MELEE-RULES.md` 正文订正瑕疵 / 覆盖对照**（被否：用户明确本 change 只登记瑕疵、不改正文，§14 第 2 条不静默改规则）；② **省略规格门或写「## 省略说明」求门禁变绿**（被否：用户明确走完整规格门、本 change 非小改动豁免场景）；③ **自填 `status: approved` + 指纹**（被否：`CODEBUDDY.md §8` / §9 由人拍板、AI 不得 self-approve）；④ **把「引用 `R-x.y`」写成「规则已实现」**（被否：`WC3-MELEE-RULES.md:395` 三态判定 / §14 第 5 条 / `critic.md:34` 清单第 13 项）。
- **依据**：`docs/references/WC3-MELEE-RULES.md:172,233,353,391,395,421,424,426-438`；`CODEBUDDY.md §8 / §9`；`tools/verify_all.py:29,40,143-195,207-294,297-315`；`openspec/decision-memo.template.md:14,48-64`；`.codebuddy/agents/critic.md:34`。
- **代价**：`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属**预期 fail-closed 信号**，如实登记、不绕过；完整 `python tools/verify_all.py` 由 leader 跑、本轮不跑；`rule_doc_sync_check()` 对 pending 玩法 change 豁免。
- **规则编号与核实状态**：§14 / §15 / 附录核实约定属活文档维护流程 `[取舍]`；R6.5 已由 commit `9f2d3e7` 拍板 `[已核机制 / 待核数值]`，本 change 不重复改规则。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——纪要 / 规格门 / 同步结论是纯文档 / 门禁元数据，`decision-memo.md` 不在四件套指纹集合内（`tools/verify_all.py:175-195`），不进状态哈希 / 快照、不改命令排序、不 bump。
- **分层依赖 / 架构原则**：参数与策略分离（OCP）、不拿沉默当授权、批准由人拍板、诚实边界（pending 不冒充放行）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-114` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。本 change 命中 **T1（主）、T4（主）**，T7 对 T1 架构方法论可选交叉、T2 边缘、T3 / T5 / T6 不命中。`reference-researcher` 已产出并核实对照报告（用户本会话提供，**仓库外证据 / 人工确认项**），五个固定提交均存在、锚按固定提交 `git grep` 核实（**非逐行全量核验**；`vera20k` / `ConcordRTS` 工作区行号可能漂移），本轮直接逐条引用、不再重复委派；判定待 `critic` 清单第 12 项复核。

| 报告条目 | 项目 / 路径（相对 `UPSTREAM.md:9-15` 各自项目根） | 借鉴 | 不借鉴（含许可证理由）/ 锚 |
|---|---|---|---|
| **G1 [T1 主]** | `vera20k`（GPLv3 + Section 7 附加条款）：`docs/Networking.md:253,599`、`src/net/command_exchange.rs:248`、`src/net/lockstep.rs:353` | 全状态哈希 + 逐帧 checksum + 帧编排方法论：新字段须同进 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 四处（ADR 0002 §6 / §7）；首期零新增字段，该结论只作 D7 硬边界依据 | 不借鉴 RA2 专属格式（`gamemd` 原生地址 / INI / SHP / VXL）/ 地址 / 2D 等距渲染；GPLv3 + Section 7 附加条款**只有真的复制代码时**才进入，本轮只读不复制（`UPSTREAM.md:50,106` / `0001-license.md:14,30,66-73`）。锚：`docs/references/UPSTREAM.md:106`（矩阵 T1 条目） |
| **G2 [T1→T7 可选交叉]** | `ConcordRTS`（GPL-3.0-only，与 GPLv3 同族）：`README.md:22`、`src/core/determinism/RuntimeAuthorityHashPrimitives.h:21-33`、`src/network/snapshots/RuntimeMultiplayerSnapshotSession.h:24`、`API.md:296,310` | 内核 / 表现分离、权威哈希原语、快照恢复契约**方法论**；仅对 D7「四处同步 / 分层边界」作通用 RTS 架构交叉 | **仅方法论，边界最严**：RA2 / YR 专属格式（SHP / VXL / TMP / MIX / INI）、规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染绝不作 WC3 玩法 / 格式 / 数值 / 规则事实来源；README 自述「逐字节一致 / 开发验收完成 N 项」不作本项目已验证事实（`UPSTREAM.md:35,114`）；对 **T4 玩法语义 MUST NOT 参考**；GPL-3.0-only 只读不复制、不引依赖、不作子模块。锚：`docs/references/UPSTREAM.md:112,114`（矩阵 T7 / 交叉注记） |
| **G3 [T4 主]** | `WarsmashModEngine`（AGPLv3）：`core/src/com/etheller/warsmash/viewer5/handlers/w3x/simulation/CGameplayConstants.java:220-222,238,426-436`、`CUnit.java:3041,3083,3106-3109`（`xpReceivingHeroes`、`1f/size`） | 经验 R6.5 / 等级 R6.1 的「**布尔开关 + 分档表 + 公式族**」最强锚：`GlobalExperience` / `MaxLevelHeroesDrainExp` / `BuildingKillsGiveExp` 三开关语义、「进入分摊集合」与「经验入账 / 等级系数」**两环节分离**、Misc 表字段驱动；支撑 D2 / D3 | **不借鉴真值**（分摊范围常量 / 野怪经验比例 / 等级倍率一律 `[待核]` 合成占位，不写任何原版字段名 / rawcode / 具体数值）；**AGPLv3 只借 JASS / 玩法语义、禁复制代码、不引 Java / LibGDX 依赖**，与 GPLv3 组合有条件、未经 ADR 0001 复核不得引入其代码（`openspec/config.yaml rules.design` 硬要求 / `UPSTREAM.md:53,109`）；若结论需代码须按 `0001-license.md:66-73` 复核点重评。锚：`docs/references/UPSTREAM.md:109`（矩阵 T4 条目） |
| **G4 [T4 主]** | `WarsmashModEngine`（AGPLv3）：`CGameplayConstants.java:224-246,16,158-160,172,324`（`revive*` / `awaken*` 参数分离、`float[][] damageBonusTable`）、`combat/CAttackType.java:5-13`、`CDefenseType.java:5`、`units/manager/MutableObjectData.java:411-428` | 复活 R6.3 / 克制 R5.3 的「**基数 + 等级系数 + 上限**」表驱动公式族与**二维矩阵结构**、定义层分层（D4 / D6 结构登记） | 不借鉴金额 / 秒数 / 倍率真值（一律 `[待核]`）；矩阵倍率无权威锚 → D6 缺口顺延；AGPLv3 只借语义、禁复制代码、不引依赖（同 G3）。锚：`docs/references/UPSTREAM.md:109`（T4）/ `docs/references/WC3-MELEE-RULES.md:366-367`（附录佐证锚） |
| **G5 [T4 主，有缺口]** | `WarsmashModEngine`（AGPLv3）：`players/CPlayerState.java:42`（`NO_CREEP_SLEEP`）、`abilities/item/shop/CAbilitySellItems.java:25,28` | leash R8.5 / 售卖 R7.4 的**机制锚**：leash 为可打断独立行为、睡眠开关化、货架清单驱动（D6 只作结构登记） | **缺真值**：硬编码 `distance<=16f`（带 TODO）不得借鉴；归巢半径 / 触发阈值 / 回购比例 / 回购价公式**无锚 → 缺口**，一律 `[待核]` + 合成占位、顺延不臆造；AGPLv3 只借语义、禁复制代码。锚：`docs/references/UPSTREAM.md:109` / `docs/references/WC3-MELEE-RULES.md:369,385,387`（附录佐证锚 / 防误用清单） |
| **G6 [T3 / T5 / T2 边界，不作主参考]** | `godot_warcraft3`（MIT）：`docs/design/game/FACTION_UNIT_ROSTERS.md:4,5`；`rts_kernel_cs`（MIT）：`README.zh.md:7` | 无（只核边界）：确认三族单位名 / 数值为原创种子、`double` 路线不采用、资产不入库 | 不借鉴原创三族名 / 数值（`UPSTREAM.md:79,110` / `WC3-MELEE-RULES.md:386`）、`double` 数值路线（ADR 0002 / `UPSTREAM.md:52,107`）、资产入库 / Godot 宿主（ADR 0001）；T3 / T5 本 change 不命中。锚：`docs/references/UPSTREAM.md:107,108,110`（矩阵 T2 / T3 / T5 条目） |
| **G7 [事实源 / 许可证]** | 事实唯一来源 = 本机一手明文 JASS / SLK / MiscGame 数据（仓库外）+ `docs/references/WC3-MELEE-RULES.md` 的 `R-x.y`；G1–G6 皆机制锚 / 方法论，**不作数值事实源** | 「系统该做什么」以 `R-x.y` 为需求源、机制结论以一手明文核对为准；`[待核]` 数值以附录核实约定 `:353` 处置 | 参考项目 / 公开攻略 / 网页值一律不作真值来源（`:11,353,357,376,391` / `UPSTREAM.md:122` 第 5 条 / ADR 0001 / 0004）；`WarsmashModEngine` AGPLv3 只借 JASS / 玩法语义、不复制代码、不引 Java / LibGDX；若结论需代码须按 ADR 0001 复核点重评。锚：`docs/references/UPSTREAM.md:114,122`（交叉注记 / 触发时机） |

**矩阵命中逐条判定（T1–T7）：**

| 矩阵 | 命中 | 理由 / 边界 |
|---|---|---|
| T1 确定性内核 / 状态哈希 / 命令排序 / 快照（`UPSTREAM.md:106`） | **主命中** | 机制结论落 `w3-core`、D7 明确「若动权威状态字段须四处同步」；首期零新增字段。G1 主锚、G2 可选交叉 |
| T2 移动 / 寻路 / 编队 / 帧阶段（`UPSTREAM.md:107`） | 边缘 | R8.5 leash 只对齐「可打断独立行为」机制结构、不触移动积分 / 寻路；`rts_kernel_cs` 的 `double` 路线不采用（G6） |
| T3 WC3 数据格式 / SLK / 地图解析 / 资产管线（`UPSTREAM.md:108`） | 不命中 | 不改导出管线 / schema / 资产解析；一手明文数据留仓库外、只作机制锚；资产与派生 JSON / PNG / GLB 一律不入库（ADR 0001） |
| T4 玩法语义 / JASS / 技能 / 经典规则（`UPSTREAM.md:109`） | **主命中** | R6.5 / R6.1 / R6.3 / R7.4 / R8.5 / R5.3 玩法语义；G3 / G4 / G5 主锚，`WarsmashModEngine` AGPLv3 只借语义、禁复制代码 |
| T5 场景装配 / 相机 / HUD / 3D 呈现（`UPSTREAM.md:110`） | 不命中 | 不触呈现实现；G6 只核「原创三族 / `double`」边界 |
| T6 网络传输 / UDP 可靠层 / relay（`UPSTREAM.md:111`） | 不命中 | **本 change 明确不新增 `GameplayCommandKind`**（D7）——新增命令会连带 T6 / ADR 0005 proto schema 追加 tag 与 `PROTOCOL_VERSION` bump；`w3net.proto` / `w3-net` / `w3-relay` / `w3-client` 零改动 |
| T7 通用 RTS 引擎架构 / 确定性内核与表现层分离 / 跨平台逐字节验收方法论（`UPSTREAM.md:112`） | 可选交叉（仅 T1 方法论） | G2 仅对 D7「四处同步 / 分层边界」作通用架构方法论交叉；**对 T4 玩法语义 MUST NOT 参考**（`UPSTREAM.md:114`）；RA2 / YR 专属格式 / 规则 / 数值绝不作 WC3 事实来源；README 自述验收不作已验证事实 |

**红线边界（逐条声明）：**

- `WarsmashModEngine` **AGPLv3 只借 JASS / 玩法语义、禁止复制代码**、不引 Java / LibGDX 依赖、不作为 WC3 数值事实源（`CODEBUDDY.md §9` / `openspec/config.yaml rules.design` 硬要求 / ADR 0001 / `UPSTREAM.md:53,109`）；若结论需代码须按 `0001-license.md:66-73` 复核点重评。
- `ConcordRTS` GPL-3.0-only 只读、不复制代码、不引依赖、不作子模块；RA2 / YR 专属格式 / 规则 / 数值 / 键位不作 WC3 玩法 / 格式 / 数值 / 规则事实来源；README「逐字节一致」自述不作本项目已验证事实（`UPSTREAM.md:35,112,114`）；T4 玩法语义 MUST NOT 以其为参考。
- `vera20k` GPLv3 + Section 7 附加条款只读不复制不进入：只有真的复制其代码时才随之进入（`UPSTREAM.md:50,106` / `0001-license.md:14,30,66-73`）。
- `rts_kernel_cs` 的 `double` 数值路线不采用（ADR 0002 / `UPSTREAM.md:52`）；`godot_warcraft3` 原创三族名 / 数值不借鉴（`UPSTREAM.md:79,110`）。
- 资产与派生 JSON / PNG / GLB / 音效一律不入库（ADR 0001 / `UPSTREAM.md:63-92`）；一手明文证据台账与反编译产物留本机仓库外、不入库、不随发行物分发（`0001-license.md:81` / `0003-scope.md:117-120`）。
- 五个只读参考项目仍是 `vera20k` / `godot_warcraft3` / `rts_kernel_cs` / `WarsmashModEngine` / `ConcordRTS`（`CODEBUDDY.md §1` / `UPSTREAM.md:9-15,96,100`），不登记第六个；证据台账 / 外部工具不冒充参考项目。

**参考缺口（如实登记，回到用户拍板、不臆造）：**

- R6.1 经验阈值分档、R6.3 复活费用 / 时间、R5.3 克制倍率、R7.4 回购价公式、R8.5 归巢半径 / 触发阈值**均无权威锚** → 一律 `[待核]` + 合成占位值，不冻结进权威定义（附录核实约定 `:353` / 防误用清单 `:374-391` / ADR 0001 / 0004），逐条进 `decision-memo.md`「待确认项」与「已知限制」，由规格门人拍板。
- G5 的 leash / 售卖只有机制锚、缺真值（`distance<=16f` 带 TODO 不得借鉴）；G3 / G4 的分档表 / 矩阵倍率缺真值。**新增 `GameplayCommandKind` 会连带 T6 / ADR 0005 proto schema 与 `PROTOCOL_VERSION` bump**，本 change 明确**不新增命令**（D7）。
- 若规格门改判「参考项目能补真值」或「补 L2 反编译取证」，MUST 回到用户拍板并按 ADR 0001 / 0003 复核点重评，不臆造、不静默改规则。

## Risks / Trade-offs

- **[Risk] 把「授权规格门 → 真改 `w3-core`」误读成已过规格门 / 已改代码 / 已冻结原版真值 / 证据已接进 CI** → Mitigation：proposal / spec / D1 / D7 / D8 逐处显式否定；`## 规格门` 保持 `status: pending`、AI 未 self-approve；红线锚 `0001-license.md:80-82` / `0003-scope.md:117-120`。
- **[Risk] 满级英雄分摊注释订正被误读成改结算行为、或 `[待核]` 数值被冻结进权威定义** → Mitigation：D2 明确「行为不变」+ spec 逐帧哈希一致 Scenario；D3 / D6 逐处 `[待核]` + 合成占位、不写原版具体数值 / 字段名 / rawcode / 技能名；锚 `WC3-MELEE-RULES.md:353,374-391`。
- **[Risk] 借用 `WarsmashModEngine`（AGPLv3）具体常量 / 复制 Java 代码，或把 `ConcordRTS` 当 T4 玩法语义事实源** → Mitigation：「## 参考项目对照」G3 / G4 / G5 / G7 逐条「不借鉴真值 / 禁复制代码」、`UPSTREAM.md:114` T4 禁止以 `ConcordRTS` 为参考、`openspec/config.yaml rules.design` 硬要求，待 `critic` 清单第 12 项复核。
- **[Risk] 新增 `GameplayCommandKind` 连锁 T6 / ADR 0005 proto schema 与 `PROTOCOL_VERSION` bump、跨 `w3-net` / `w3-relay` / `w3-client` 多 owner** → Mitigation：D7 明确不新增命令 / 不新增权威状态 / 不 bump / 不连锁；spec 字段集合零改动 Scenario；`critic` 代码门以 `git diff` 自证。
- **[Risk] 把视觉 / 机制 oracle / 离屏对照当 ADR 0002 业务逻辑 oracle，据此改 determinism / golden hash / CI** → Mitigation：D5 / D7 显式区分「证据是机制锚、不是可冻结机器可判业务逻辑 oracle」、证据 MUST NOT 进门禁；重跑 determinism 只因真改 `w3-core` 代码、不是因为接证据。
- **[Risk] 「§11 核实约定」历史误称被沿用、或本 change 静默改规则文档正文** → Mitigation：Context / D8 / spec Scenario 逐处引用「附录核实约定 `:353`」、瑕疵只在 `decision-memo.md` 登记、`git diff -- docs/references/WC3-MELEE-RULES.md` 为空（§14 第 2 条）。
- **[Risk] 参考对照锚行号漂移 / 被当成「已逐行核验」，或仓库外台账被写成「已入库」** → Mitigation：G1–G7 与 C4 逐处标「按固定提交 `git grep` 核实 / 非逐行全量核验 / `vera20k` / `ConcordRTS` 行号可能漂移」「仓库外 / 人工确认项」；不写本机绝对路径。
- **[Risk] `spec_gate_check()` 对 `status: pending` fail-closed 被误当失败去 self-approve / 写省略说明** → Mitigation：D8 如实登记为预期信号、不绕过、不填指纹 / `approved_by` / `date`；完整 `verify_all.py` 由 leader 跑。
- **[Risk] 规则编号引用被冒充规则已实现、真窗口 / 双平台人工清单被标通过** → Mitigation：spec「规则文档同步」requirement + `decision-memo.md`「## 规则文档同步」待办结论（§14 第 5 条 / 三态判定 `:395`）、`critic` 清单第 13 项内容层复核；四件套一律引用 `R-x.y` 作需求依据、不冒充已实现。

## Migration Plan

- **本轮（成文期）**：只在 `openspec/changes/p17-melee-mechanism-alignment/` 内产出四件套 + `decision-memo.md`，外加 `docs/decisions/0003-scope.md` 文末纯追加第三次修订；`git status --short` 仅该目录新增 + 该 ADR 文件 modified；`## 规格门` 保持 `status: pending`、AI 未 self-approve、不勾任一复选框；不碰 `crates/**` / 主 specs / 规则文档正文 / `tools/**` / CI / 权限配置 / p12–p16 / 归档件。
- **规格门**：`critic` 按清单第 7 / 10 / 11 / 12 / 13 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ 由 leader / `spec-steward` 在人确认后回填 `status: approved` + 四件套指纹（`python tools/verify_all.py --fingerprint p17-melee-mechanism-alignment`）+ `approved_by` + `date`（本轮不填）；批准即收敛「首期范围 = 推荐最小闭环 vs 全量」等待确认项。此后规格锁定，改任一 artifact MUST 按 §8 同步回查其余三份并重走规格门。
- **apply（后续独立会话，`sim-owner` 唯一 owner）**：按 D1–D8 落地，先收敛规格门拍板的首期范围，再改 `crates/w3-core/src/sim_match.rs`（注释订正、行为不变）/ 补布尔 / 因子机制（不改字段集合）/ 补 `gameplay.rs` / `creep.rs` 机制回归 / 新增 CPU 确定性回归 + 反向验证；重跑 `cargo test -p w3-core --test determinism`、`cargo test --workspace`、`cargo fmt --all -- --check`；不碰 `w3-net` / `w3-client` / `w3-relay` / 主 specs / `tools/**` / ADR 其余正文；一个 change 一个 owner、不同时改多 owner 文件。
- **收尾 / 规则文档同步**：按 §14 第 5 条在 `decision-memo.md`「## 规则文档同步」给结论、更新附录「规则 → 实现覆盖对照」对应行与 §15（注明日期与依据、不静默改规则），`critic` 清单第 13 项内容层复核；真窗口 8.x 人工清单、golden hash v9 双平台逐字节比对、CI 双平台一律「待人工 / 待 VM」。
- **回滚**：本 change 成文产物为纯文档 + 后续 `w3-core` 注释 / 回归 / 合成占位机制（首期零新增权威状态 / 零 bump / 零资产 / 零依赖），删除 / 回退文档与代码即可；证据台账留仓库外本机、不随发行物分发；若规格门改判全量方案动了权威状态 / bump，回滚面另按 ADR 0002 复核。
- **人工项**：人工 push → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` → `/opsx:archive`；AI 不 push / fetch / pull / clone、不 archive、不本地 commit（本轮由 leader 统一处理）。

## Open Questions

> 以下均为「可安全延后而不改变 specs / approach / task breakdown」的未知项，已由 D1 / D3 / D5 / D6 / D8 参数化承接并登记在 `decision-memo.md`「待确认项」，由规格门 / leader 拍板；本轮**不臆造为已定案**：

- **首期范围 = 推荐最小闭环 vs 全量方案**（任务点名）→ 待规格门用户拍板；推荐最小闭环（真改代码、风险可控）、全量方案（改权威状态结构 / bump 三版本号 / 连锁网络客户端层）建议顺延 p18（`decision-memo.md` 第一待确认项 / D1）。
- 布尔 / 因子机制（建筑击杀经验开关 / 召唤物经验折减 / 等级因子分档递减）是否首期落地、若需改字段则顺延 → 待规格门拍板（D3）。
- 五类参考缺口（R6.1 阈值 / R6.3 费用时间 / R5.3 倍率 / R7.4 回购价 / R8.5 归巢半径）的 `[待核]` 处置、是否补 L2 反编译取证 → 待规格门拍板，不臆造（D6）。
- 反向验证具体形态（§5 形态① 测试内反例 + 元断言 / 形态② A-B 对照）与缺陷态构造 → 待规格门 / apply 阶段 `sim-owner` 按 §5 定（D5）。
- 规则文档同步（附录覆盖对照 / §15）的收尾落实、成文表述瑕疵（「§11 核实约定」历史误称）的收口时机 → 待规格门拍板（D8）。
- 规格门批准记录回填（四件套指纹 / `critic` 报告 / `approved_by` / `date`）→ 待用户拍板后由 leader / `spec-steward` 回填，本轮一律不填、AI 未 self-approve（D8）。
- 参考矩阵 T1–T7 命中判定与锚漂移复核 → 本轮据 `UPSTREAM.md:104-114` 判定 T1 / T4 主命中、T7 可选交叉、T2 边缘、T3 / T5 / T6 不命中，待 `critic` 清单第 12 项复核；若改判命中须补对照报告并回到用户拍板。
