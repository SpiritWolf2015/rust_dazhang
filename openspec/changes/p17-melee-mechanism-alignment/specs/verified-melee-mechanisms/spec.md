# Spec Delta

## Purpose

把本机一手明文数据核对出的 WC3 Melee 业务规则机制性结论（R6.1 / R6.3 / R6.5 / R7.4 / R8.5 / R5.3）正式定义为 `w3-core` 确定性内核的可验证行为契约：只落机制结构 / 布尔开关 / 公式族，数值一律 `[待核]` 合成占位，并以反向验证锁定防退化。

## ADDED Requirements

### Requirement: 满级英雄参与经验分摊且注释与规则一致（R6.5）

系统 SHALL 在野怪击杀结算中让**满级英雄参与经验分摊**（2026-11-26 按 `WC3-MELEE-RULES.md §14` 第 2 条拍板、§15 有该行、commit `9f2d3e7`）；`crates/w3-core/src/sim_match.rs:4035-4036` 的旧 `[待核]` 注释 SHALL 订正为该核对结论，**结算行为保持不变**、不冻结任何 `[待核]` 数值（附录「核实约定」`WC3-MELEE-RULES.md:353` / ADR 0001 / 0004）。

#### Scenario: 满级英雄参与分摊且注释订正（R6.5 / §14 第 2 条）

- **WHEN** 一个满级英雄与一个未满级英雄同处经验分摊范围、己方击杀野怪并推进结算帧，且核对 `crates/w3-core/src/sim_match.rs` 的注释
- **THEN** 满级英雄进入分摊集合并获得均分经验、`state.xp` 确定性增长，逐帧 `compute_state_hash()` 与同脚本基线一致；`Grep pattern="2026-11-26|§14|§15|参与分摊" path="crates/w3-core/src/sim_match.rs"` 命中，`Grep pattern="本期不冻结该规则|是否参与分摊.*待核" path="crates/w3-core/src/sim_match.rs"` 返回零命中；MUST NOT 把满级英雄排除出集合

#### Scenario: 注释订正不改结算行为（R6.5 / ADR 0002）

- **WHEN** 对比注释订正前后同一冻结定义与开局脚本的逐帧状态哈希
- **THEN** 结算输出与 `SNAPSHOT_VERSION` 定义的快照（当前 9）逐字节一致、注释变更不进 FNV-1a 状态哈希（ADR 0002 §6-7）

### Requirement: 经验结算两环节分离、稳定顺序与均分（R6.1 / R6.5 / ADR 0002）

经验分摊 SHALL 区分「进入分摊集合」（范围 + 存活 + 己方英雄）与「实际入账 / 等级因子」两环节（R6.5），集合 SHALL 以稳定 `EntityId` 顺序承载、逐英雄均分、只结算一次；MUST NOT 由表现层反写 `HeroState.xp`（ADR 0002 §5）。分摊范围 / 各档比例 / 范围常量等数值一律 `[待核]` 合成占位、不冻结（`WC3-MELEE-RULES.md:172,353`）。

#### Scenario: 两环节分离与稳定 EntityId 顺序（R6.5 / ADR 0002）

- **WHEN** 多个己方英雄处于分摊范围、击杀野怪，随后以相同冻结定义与开局脚本重开一局
- **THEN** 分摊集合判定与实际入账按稳定 `EntityId` 顺序分离可验证、集合内英雄均分同一 `share`、不重复入账；新一局不残留上一局 xp / 收益登记（R11.5 / R12.4）

#### Scenario: 缺陷态被反向验证拦截（R6.5 / CODEBUDDY.md §5）

- **WHEN** 以退化实现（排除满级英雄 / 均分改为全额 / 集合改为不稳定顺序 / 重复入账）替换正式实现并跑新增回归
- **THEN** 对应反例断言失败、修复后通过，两条证据可复现（`CODEBUDDY.md §5` 形态① / ②），仅「全绿」不构成验证

### Requirement: 布尔与因子机制以合成占位表达且不改字段集合（R6.1 / R6.5）

系统 SHALL 用合成占位值表达三类机制结构：建筑击杀是否授予经验的布尔开关、召唤物击杀经验折减因子、按英雄等级分档递减的经验因子（R6.1 / R6.5，`WarsmashModEngine` 仅借「布尔开关 + 分档表 + 公式族」语义、不借真值，见 `design.md`「## 参考项目对照」G3）；这些机制 MUST NOT 新增 / 修改权威状态字段集合、MUST NOT 写死任何原版数值、MUST NOT 含 `f64`（ADR 0001 / 0002 / 0004）。若实现某子项需改字段集合，SHALL 顺延后续 change 并登记为已知限制，不臆造字段。

#### Scenario: 合成占位且干净环境构造（R6.1 / ADR 0001 / 0004）

- **WHEN** 在不含 `config.toml` 与任何导出目录的干净环境构造含这些机制的冻结定义
- **THEN** 构造成功、不做任何文件 IO、每个值为编译期合成整数常量、`[待核]` 标注齐全，不含 MPQ 派生数据 / 真实导出 JSON / 原版字段名 / rawcode / 技能名 / 具体数值 / `f64`

#### Scenario: 字段集合与三版本号零改动（R6.5 / ADR 0002 / ADR 0005）

- **WHEN** 收尾核对本 change 的权威状态字段集合与三版本号
- **THEN** `git diff` 无新增 / 修改权威状态字段、无新增 `GameplayCommandKind` 变体；`SNAPSHOT_VERSION` / `SIM_VERSION`（`crates/w3-core/src/snapshot.rs:67-68`）与 `PROTOCOL_VERSION`（`crates/w3-net/src/packet.rs:50`）仍为 9，`w3net.proto` 与 `w3-net` / `w3-client` / `w3-relay` 零改动

### Requirement: 复活费用与时间公式族上限生效（R6.3 / ADR 0002）

复活费用（gold / lumber）与复活时间（30 Hz 整数帧）SHALL 读冻结的**合成 / 原创编译期整数常量**、按「基数 + 等级系数 → 绝对上限」表驱动计算，SHALL 用 `i128` 中间量防溢出、整数除法向零截断、MUST NOT 引入浮点；祭坛复活与酒馆买活参数 MUST 分离（R6.3 / R9.3 / `WC3-MELEE-RULES.md:169,353` / ADR 0001 / 0004）。买活半血无蓝未实现、登记为已知限制顺延。

#### Scenario: 上限生效与费用读冻结合成常量（R6.3 / ADR 0002）

- **WHEN** 分别以等级 1 与高等级英雄提交复活、并对比前后 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 费用 / 时间按冻结常量随等级确定变化、在绝对上限处截断（不越界、不出现负值）、经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部、逐帧哈希与同脚本基线一致（R12.4 / ADR 0002 §6-8）

#### Scenario: 公式族缺陷态被反向验证拦截（R6.3 / CODEBUDDY.md §5）

- **WHEN** 以退化实现（去掉绝对上限 / 引入浮点 / 祭坛与买活参数串用）替换正式实现并跑新增回归
- **THEN** 对应反例断言失败、修复后通过（`CODEBUDDY.md §5` 形态① / ②），仅「全绿」不构成验证

### Requirement: 确定性回归、反向验证与 determinism 重跑（ADR 0002 / CODEBUDDY.md §5）

本 change 真改 `w3-core` 确定性代码，SHALL 新增 CPU 确定性回归断言「满级英雄参与分摊」「集合内英雄均分」「稳定 `EntityId` 顺序」「复活费用 / 时间公式族上限生效」，MUST 重跑 `cargo test -p w3-core --test determinism` 全绿；新增 / 修改回归 MUST 证明缺陷态失败、修复态通过（测试内反例 + 元断言防退化，或 A-B 对照），仅「全绿」不构成验证（`CODEBUDDY.md §5` / ADR 0002 §8）。无法构造缺陷态的 MUST 在 `design.md` 登记理由与替代证据。

#### Scenario: determinism 重跑与改坏验红（ADR 0002 §8 / CODEBUDDY.md §5）

- **WHEN** 跑 `cargo test -p w3-core --test determinism`，并以退化实现替换正式实现
- **THEN** 正式态全绿、缺陷态对应反例断言失败且修复后通过，两条证据可复现；`cargo test --workspace` 与 `cargo fmt --all -- --check` 由 apply 阶段复核、不预先声称通过

### Requirement: 证据、数值、权威状态与资产边界（ADR 0001 / 0002 / 0004）

一手明文证据台账（仓库外 / 人工确认项 / 行号可能漂移）与任何反编译产物 MUST NOT 进仓库、权威状态、FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照、`first_difference()`、golden hash、determinism 验收或 CI 门禁（ADR 0001 / 0002 / `CODEBUDDY.md §5-6`）。**本 change 零新增权威状态字段，故无需进入状态哈希或由 `SNAPSHOT_VERSION`（当前 9）定义的快照**；若规格门改判全量方案而新增权威状态字段，MUST 四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）+ bump 三版本号 + golden hash 重取 + 反向验证。数值一律 `[待核]` + 合成占位、不冻结原版真值（附录「核实约定」`:353`）。

#### Scenario: 零资产与证据隔离自证（ADR 0001 / 0002 / 0004）

- **WHEN** 收尾核对本 change 的变更边界与门禁产物
- **THEN** `git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 派生 JSON / PNG / GLB / 音效；`git diff -- crates/w3-net/` / `-- crates/w3-client/` / `-- crates/w3-relay/` 为空；仓库内无本机盘符绝对路径、无凭证、无原版具体数值 / 字段名 / rawcode / 技能名；证据台账不出现在任何仓库 artifact（台账一律「仓库外 / 人工确认项 / 行号可能漂移」，不入库、不冒充已核验）

#### Scenario: 全量方案改判时的四处同步（ADR 0002 §6-7）

- **WHEN** 规格门改判全量方案、实现新增权威状态字段（R6.1 阈值分档表 / R5.3 克制矩阵等）
- **THEN** 该字段 MUST 同时进入状态哈希、`SNAPSHOT_VERSION` 定义的快照、`first_difference()` 与 `restore_snapshot`，`cargo test -p w3-core --test determinism` 全绿并经反向验证；MUST NOT 借本 change「零新增权威状态」口径旁路门禁

### Requirement: 参考缺口如实登记、顺延不臆造（R5.3 / R7.4 / R8.5 / 附录核实约定）

R5.3 克制矩阵倍率、R7.4 回购价公式、R8.5 归巢半径 / 触发阈值等**无权威锚**的数值 SHALL 一律保持 `[待核]` + 合成占位、登记为参考缺口并顺延后续 change；系统 MUST NOT 冻结这些真值、MUST NOT 据参考项目硬编码值臆造结论（`WC3-MELEE-RULES.md:353,374-391` / `UPSTREAM.md:122` / ADR 0001 / 0004）。机制结构（二维克制矩阵 / 货架清单驱动 / 可打断独立 leash 行为）只作对齐登记、不冒充已实现。

#### Scenario: 缺口清单与顺延可核验（R5.3 / R7.4 / R8.5）

- **WHEN** 核对 `design.md`「## 参考项目对照」的参考缺口段与 `decision-memo.md` 已知限制
- **THEN** R5.3 / R7.4 / R8.5 三类缺口逐条标 `[待核]` + 合成占位 + 顺延，无一条被写成「已核实 / 已对齐」；`Grep` 不出现任何原版具体数值；回到规格门由人拍板、不臆造

### Requirement: 规则文档同步与成文表述瑕疵登记（§14 第 5 条 / §15 / 附录核实约定）

玩法 change 收尾 SHALL 按 `WC3-MELEE-RULES.md §14` 第 5 条在 `decision-memo.md`「## 规则文档同步」给出结论，并在 apply / 归档前更新附录「规则 → 实现覆盖对照」对应行与 §15 变更记录（注明日期与依据）。`[待核]` 口径 MUST 引用附录「核实约定」(`WC3-MELEE-RULES.md:353`)，MUST NOT 沿用 `:172` / `:391` 的「§11 核实约定」历史误称（`## 11` `:233` 实为胜负与结束）；成文表述瑕疵只登记、不静默改规则文档正文（§14 第 2 条）。MUST NOT 把「引用 `R-x.y`」冒充「该规则已完整实现 / 已人工验收」（三态判定 `:395`）。

#### Scenario: 待办结论与表述瑕疵登记可核验（§14 第 2 条 / 第 5 条）

- **WHEN** 本轮成文期核对 `decision-memo.md` 与规则文档变更边界
- **THEN** 「## 规则文档同步」小节非空且给出「本轮 pending / 纯成文、暂写待办结论、apply 阶段落实」；`git diff -- docs/references/WC3-MELEE-RULES.md` 为空；本 change 四件套引用「附录核实约定 / `:353`」而非「§11 核实约定」；瑕疵登记在本纪要「已知限制」、不冒充已收口
