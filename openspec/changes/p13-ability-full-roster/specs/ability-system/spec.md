# Spec Delta

## MODIFIED Requirements

### Requirement: 四族技能清单按 SLK 逐项对齐并人工固化

系统 SHALL 把四族技能清单逐项按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）人工核对后固化成编译期常量表进 `FrozenDefinitions`；对齐的是清单条目与分类、MUST NOT 照搬经典名称 / rawcode / 数值。内部仍用合成原创 `DefId` 作键、MUST NOT 引入四字符 ID → `DefId` 映射层、MUST NOT 冒充经典具名技能，数值一律 `[待核]` 且 MUST NOT 冻结进权威定义（S3 / R2 / R6 / R7.6 / ADR 0001 / 0002 / 0004）。定义层冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部，与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）。本 change 把本期范围由「顺延标已知限制 + 合成占位表」改为「顺延项全量 + 清单逐项 SLK 人工核对后固化」（S3 / S5 / S6 / R8 / R9 / §14）。

#### Scenario: 四族技能清单全量对齐与冻结常量

- **WHEN** 按 S3 / R6 以本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）逐项核对四族技能清单并人工固化成 `FrozenDefinitions` 编译期常量表
- **THEN** 清单条目与分类逐项对齐经典具名技能的**真实结构与分类**（哪些技能、每族几个、英雄普通技能 + 6 级大招、单位技能、道具技能），替换 P11 合成占位表 72 条的整齐结构口径；**中立英雄 / 野怪 / 全部单位技能纳入全量实现、不再顺延标已知限制**（S3 / S5 / S6 / R6 / R8 / R9 / R7.6 / §14）；实际条目数经人工逐项核对后如实登记、MUST NOT 臆造「四族全量 = 某精确数字」（`WC3-MELEE-RULES.md §14`、仓库文件外证据 / 人工确认项）；`DefId` 类别内唯一且仍为合成原创（沿用 `ABILITY=10` / 英雄 `11–14` 并扩四族）、无四字符 ID → `DefId` 映射层、SLK 四字符 ID 只作核对依据不进权威状态 / 状态哈希 / 快照 / 不入库；冻结常量经 `compute_defs_hash`（`gameplay.rs:434`）进入 `defs_hash` / `content_hash` 快照头部、与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）；`cargo test -p w3-core --test determinism` 全绿（S3 / R6 / R2 / ADR 0002）

#### Scenario: 两层张力与零资产合规

- **WHEN** 核对技能定义来源、清单对齐层次与仓库跟踪文件
- **THEN** 「清单结构 / 条目逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」被显式区分为两个层次——对齐的是清单条目与分类、MUST NOT 照搬经典名称 / 四字符 rawcode / 数值；命名非经典具名、数值均 `[待核]`、以本机导出 SLK 人工核对、不静默改规则（`WC3-MELEE-RULES.md §14`）；SLK 四字符 ID 只作核对依据、不进权威状态 / 状态哈希 / 快照；无 `*.slk` / MPQ 派生 JSON / PNG / GLB 入库（`python tools/verify_all.py` 资产扫描通过），本机导出表规模元数据只登记规模与分类、不登记任何具体技能名 / rawcode / 数值（导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库，ADR 0001）（R6 / R7.6 / ADR 0001 / 0002 / 0004 / `WC3-MELEE-RULES.md:7,124,351,374`）

## ADDED Requirements

### Requirement: 顺延项技能全量纳入

系统 SHALL 把中立英雄 / 野怪 / 全部单位技能纳入技能系统全量实现，覆盖 1.0 范围 S3 / S5 / S6 与业务规则 R6 / R8 / R9 / R7.6（`WC3-MELEE-RULES.md:27-30,165-171,184,188-218`）。规则编号引用只作需求依据，MUST NOT 把「引用 `R-x.y`」冒充「该规则已实现」（附录三态判定 / §14 第 5 条）。

#### Scenario: 顺延项全量范围与规则引用核对

- **WHEN** 核对 P11 归档件「已知限制」第 1 条（顺延项标已知限制）与本 change 的全量范围，并核对 `design.md` 引用的 S3 / S5 / S6 / R6 / R8 / R9 / R7.6
- **THEN** 中立英雄 / 野怪 / 全部单位技能均在全量实现范围内、无「顺延 / 已知限制」残留于行为契约；每个规则引用可在 `WC3-MELEE-RULES.md` 打开且标注 `[已核]` / `[待核]` / `[取舍]` 核实状态；`openspec validate p13-ability-full-roster --strict` 通过（S3 / R8 / R9 / §14）

### Requirement: 清单逐项 SLK 人工核对的人工确认项边界

清单条目与分类 SHALL 经本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData` 人工逐项核对后固化；该核对为**仓库文件外证据、人工确认项**，系统 MUST NOT 冒充「已逐项核对完成」，实际条目数 MUST 核对后如实登记、MUST NOT 臆造精确数字、数值 `[待核]` 不静默改规则（`WC3-MELEE-RULES.md §14` / ADR 0001）。

#### Scenario: 人工逐项核对与真实清单固化

- **WHEN** apply 阶段由 `content-owner` 对本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）人工逐项核对，替换 `SYNTHETIC_ABILITY_CATALOG`（`crates/w3-core/src/ability_system.rs:31,121-177`）与 `FOUR_RACE_ABILITY_CATALOG`（`crates/w3-client/src/melee/defs.rs:734`）的合成占位口径
- **THEN** 真实清单条目与分类经人工确认后固化成编译期常量表；实际条目数核对后如实登记、不臆造精确数字；`decision-memo.md`「待确认项」如实登记该人工确认项边界、proposal / design 无「已逐项核对完成」措辞；`python tools/verify_all.py` 资产扫描通过、`git ls-files` 无导出数据入库（ADR 0001 / `WC3-MELEE-RULES.md §14`）

#### Scenario: 两层张力不被读成矛盾

- **WHEN** 核对「清单条目与分类逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」两种表述
- **THEN** 两者被显式区分为两个层次：对齐的是清单条目与分类（哪些技能、每族几个、英雄 3 普通 + 1 大招、单位 / 道具技能），MUST NOT 照搬经典名称 / 四字符 rawcode / 数值；内部仍用合成原创 `DefId`、不引入四字符 ID → `DefId` 映射层、数值 `[待核]`（S3 / R6 / R7.6 / ADR 0001 / 0004）

### Requirement: 全量清单的两层张力与零资产边界

对齐清单 MUST NOT 照搬经典名称 / 四字符 rawcode / 数值；内部 MUST 用合成原创 `DefId` 作键、MUST NOT 引入四字符 ID → `DefId` 映射层、MUST NOT 冒充经典具名技能；数值 `[待核]` MUST NOT 冻结进权威定义或入库（ADR 0001 / 0002 / 0004 / `WC3-MELEE-RULES.md:7,124,351`）。

#### Scenario: 合成原创与零资产合规

- **WHEN** 核对 `crates/w3-client/src/melee/defs.rs` 与 `crates/w3-core/src/ability_system.rs` 的清单定义与仓库跟踪文件
- **THEN** 无四字符 ID 常量与映射层、命名非经典具名、数值均 `[待核]`；无 `*.slk` / MPQ 派生 JSON / PNG / GLB / `*.j` / `.jui` 入库（`python tools/verify_all.py` 资产扫描通过）；`WarsmashModEngine`（AGPLv3）只借语义、不复制代码、`ConcordRTS`（GPL-3.0-only）RA2/YR 数值 / 规则不作 WC3 事实来源（ADR 0001 / `docs/references/UPSTREAM.md:109,112,114`）

### Requirement: 新增权威状态的确定性契约与 bump 评估

若全量实现新增权威状态，系统 SHALL 令其同步进入 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 四处与 `WorldGameplayState::hash`，并进入 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 9；若 bump 则 10）定义的快照；MUST 按 ADR 0002 评估是否 bump `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`，若拍板 9→10 则全套同步（R12.4 / ADR 0002 §6-8）。

#### Scenario: 四处同步与形态①改坏验红

- **WHEN** 对任一新增权威运行态字段故意漏写哈希 / 快照 / `first_difference` / restore 之一，再以完整实现重跑 `cargo test -p w3-core --test determinism`
- **THEN** `CODEBUDDY.md §5` 形态①反例元断言在缺陷态失败、修复态通过；完整实现下四处同步 + `WorldGameplayState::hash`、双实例逐帧哈希一致、快照销毁重建后从 N+1 续算一致；`cargo test -p w3-core --test determinism` 全绿（R12.4 / ADR 0002 §6-8 / `critic.md` 清单第 11 项③）

#### Scenario: bump 9→10 全套评估与旧版本拒绝

- **WHEN** 按 ADR 0002 评估新增权威状态是否触发版本 bump，且若规格门拍板 9→10 时核对三版本号消费者与 golden hash
- **THEN** 结论由规格门拍板、不臆造为已定案（进 `decision-memo.md`「待确认项」）；若 bump，则 `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 三者同步 9→10、全仓 Grep 旧版本号无残留、旧 v9 快照 / 回放 / 包按 `UnsupportedVersion` / `SimVersionMismatch` / `VersionMismatch` + relay `Reject` 明确拒绝、`w3net.proto` 只追加 tag 并保留 `reserved`、golden hash 如实重取（R12.4 / ADR 0002 §7-8 / ADR 0005 §3）；无 bump 时写明「零新增权威状态」依据

### Requirement: 验收线与人工清单边界

验收 SHALL 含 `cargo test -p w3-core --test determinism`、`cargo test --workspace`、`cargo fmt --all -- --check` 与形态①反向验证；真窗口 8.x 人工清单 MUST 保持未勾「待人工」、不冒充通过；golden hash 双平台 / CI 双平台 / 参考缺口 MUST 如实登记为待 VM / 人工，不写成已验证；N1–N6（尤其 N5 JASS / wtg 触发器 VM）明确不做（R12.4 / `CODEBUDDY.md §5` / §9 / `WC3-MELEE-RULES.md:43-49`）。

#### Scenario: CPU 验收全绿且人工清单不冒充

- **WHEN** 执行 apply 阶段的 CPU 验收命令并核对 `tasks.md` 人工清单组与 `decision-memo.md`「已知限制」
- **THEN** determinism / workspace / fmt / verify_all 的对应检查全绿、形态①缺陷态失败修复态通过；真窗口 8.x 人工项全部 `- [ ]` 未勾标「待人工」、golden hash 双平台 / CI 双平台 / 参考缺口如实标「待 VM / 人工」、无「已验证全平台」措辞（R12.4 / `CODEBUDDY.md §5` / §9）

#### Scenario: N1–N6 明确不做

- **WHEN** 核对本 change 的实现边界与 `crates/` 内引用
- **THEN** 无 JASS / wtg 触发器 VM（N5）、地图编辑器（N2）、RPG 自定义地图（N3）、剧情战役（N4）、纯本地单机对局形态（N1）、暴雪资产入库（N6）；道具技能经既有 `EffectDef` 组合走 R7.6、MUST NOT 直接执行任意 JASS（`WC3-MELEE-RULES.md:43-49,184` / `0003-scope.md:15,38`）

### Requirement: 规则文档同步核对

玩法 change 收尾 SHALL 按 `WC3-MELEE-RULES.md §14` 第 5 条在 `decision-memo.md`「## 规则文档同步」小节给出结论，需同步则更新附录「规则 → 实现覆盖对照」对应行与 §15 变更记录、无需同步则写明依据；提案阶段给「待 apply 阶段同步」的如实结论，MUST NOT 把「引用 `R-x.y`」冒充「规则已实现」（三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。

#### Scenario: 规则文档同步结论不冒充已实现

- **WHEN** 核对 `decision-memo.md`「## 规则文档同步」小节与 `tasks.md` 规则文档同步核对复选框
- **THEN** 小节非空且结论如实（提案阶段为「待 apply 阶段按已拍板顺序同步附录 R6 技能独立行 / §15」）、未把「design 引用 R6 / R8 / R9」冒充「规则已实现」；真窗口 8.x 人工清单仍「未执行 / 待人工」、不标通过；`tools/verify_all.py::rule_doc_sync_check()` 对 `status: pending` 豁免（§14 第 5 条 / `critic.md` 清单第 13 项）
