# Spec Delta

## MODIFIED Requirements

### Requirement: 表现层选族入口与装配期传参

系统 SHALL 提供开局前选族 overlay（**4 族：人 / 兽 / 不死 / 暗夜**，Q1' 已拍板不死 + 暗夜双族全量 + P8 两族不重做；`3xx` / `4xx` 段）与 `FactionSelection` 纯表现层只读状态，经 `MeleeSetup::with_races` / `with_faction_selection` / `apply_faction_selection` 派生开局与 `CommandCatalog::for_race` 注入（`faction.rs:28-35,86-103`、`app.rs:330-360`、R2 / S1 / S3 / R13.1）。`i18n`（`i18n.rs:38-50,205-211`）与 overlay / 命令卡目录 SHALL 按表扩展 4 族文案键；选族 MUST 只停表现层本地只读状态、不新增 `w3-core` 分类权威字段、不进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / Q6 / Q2 / R13.6）。未选族时 SHALL 回落既有默认人族段（R2 / R12.7）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 选族后开局 DefId 分段正确（R2 / S1 / S3）

- **WHEN** 分别以 4 族（人 / 兽 / 不死 / 暗夜）选族 overlay 确认并装配开局，且未选族时走默认入口
- **THEN** 双方 `Spawn` 关联的 `def_ids` 落在所选族段（`1xx` / `2xx` / `3xx` / `4xx`）、中立恒 `9xx`，4 族命令卡目录 / i18n 中文键 / 模型键段按表命中，`Race` 4 变体不进 `w3-core` 分类字段（Q2 族特色机制以独立顶层权威状态段表达、不复用）；未选族回落人族段（R2 / S1 / S3 / R13.1 / Q1' / Q5' / Q6 / Q2）

#### Scenario: 选族是表现层只读状态、不新增权威字段（ADR 0002）

- **WHEN** 连续切换 4 族选族 / 相机 / HUD 并校验 `RtsMatch` / `Player` 字段集合与 `SNAPSHOT_VERSION`
- **THEN** `FactionSelection` / `faction_overlay` 只在 `crates/w3-client/` 本地只读状态、无 `race` 分类权威字段、选族动作本身逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与不选族基线一致；**历史口径「`SNAPSHOT_VERSION` 仍为 3」不作行为断言**：按用户 Q2 / Q3 已拍板 `SNAPSHOT_VERSION` bump 8、`w3-core` 新增族特色机制顶层权威状态段且四处同步（非空 diff 属预期），选族 / HUD 只读派生不反写（ADR 0002 §5-8 / Q2 / Q3 / Q6 / R13.6）

#### Scenario: 未选族时回落既有默认（R2 / R12.7）

- **WHEN** 既有 `playable` 入口不传种族参数或不触发 overlay 确认
- **THEN** 行为与改动前一致、回落人族段且一局仍能装配并打完（逐帧哈希与 v8 新基线一致），4 族扩展与 Q2 / Q4 机制不改变既有回落路径、不静默改既有测试（R2 / R12.7 / Q3 / Q5' / Q1'）

### Requirement: 范围边界与承接 P8.3 遗留

本 change SHALL 按用户已拍板口径做 4 族选族入口与按表扩展、不重做 R13 命令卡 / 快捷键全集；**1.0 范围张力 MUST 如实呈现、不静默窄化、不冒充已解决**：`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226`（R10.2 / R10.3 / R10.4）字面「暗夜夜间隐身 / 昼夜 / 隐身反隐随迷雾纳入 1.0 一并做完」，而 P8.5e 规格门 Q2「最小闭环」曾把它们登记为「后续 change / P9+ 四族承接」（`2026-11-22-p8-fog-of-war/decision-memo.md:80,128`）。**Q4 已由用户 2026-11-22 明确拍板本期含全量（不是 AI 推荐、不是静默窄化）、Q1' 已拍板双族全量、Q4' 已拍板张力保留显式**——暗夜 overlay / 夜间隐身 / 昼夜全量按 Q2 / Q4 本期纳入、不再写「后续 change 承接」；这是已拍板后的如实登记、不是待拍板项。零资产 / `[待核]` 数值不冒充经典（R2 / ADR 0001 / §14 / Q3'）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 承接关系清晰、不重做 R13 命令卡（S1 / S3 / R13.1）

- **WHEN** 校验本 change 的能力边界与 R13 命令卡 / 快捷键的承接关系
- **THEN** 4 族只经按表扩展命令卡目录 / i18n / overlay，不重写 R13 全集、不把 Q2 / Q3 新增 `GameplayCommandKind` 码 24 起塞进快捷键白名单；Q4 已拍板本期含全量机制（R10.2 / R10.3 / R10.4 / R5.6 / S3 / R8.4）按 Q2 权威状态 / Q1' 双族全量纳入、不再登记为「后续 change 承接」，但 1.0 字面「一并做完」与本期双族全量的张力按 Q4' 已拍板七处显式如实登记、不预设结论、不静默窄化 1.0 范围（S1 / S3 / R13.1 / R10.2-5 / Q1' / Q4'）

#### Scenario: 零资产入库与 `[待核]` 不冒充经典（R2 / ADR 0001 / §14 / Q3 拍板）

- **WHEN** 在干净环境运行 4 族选族装配并核对常量落点
- **THEN** 零文件 IO / 零资产入库，4 族文案 / 数值为 `[待核]` 合成原创常量、不冒充原版；按 Q2 / Q3 已拍板 `SNAPSHOT_VERSION` bump 8、族特色机制冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部，两套昼夜常量口径未统一与全量机制 B 类 `[待核]` 数值按 Q3' 已拍板由人核对（实施期数值核对、非规格门待拍板范围 / 架构 Q）、不静默改规则（R2 / ADR 0001 / ADR 0004 / §14 / Q2 / Q3 / Q3'）
