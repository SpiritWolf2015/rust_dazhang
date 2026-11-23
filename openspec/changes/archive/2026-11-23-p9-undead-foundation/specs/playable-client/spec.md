# Spec Delta

## MODIFIED Requirements

### Requirement: HUD rendered from read-only view

HUD SHALL 只消费只读 `WorldView`（`tick` / `rng` / `entities`）、只读 `Snapshot`（含 `revive` / `town_hall` / `creep_camps` / `item_shop` **与按 Q2 新增的族特色机制顶层字段**）与事实事件拷贝、按 `derive(snapshot, events)` 范式派生只读投影子模型（R13.6 / ADR 0002 §5 / Q2）。本 change P9 把 HUD 只读派生清单按表扩到 **4 族**（Q1' 已拍板双族全量）：选族 `FactionSelection` / `faction_overlay`（`hud.rs:866-871,1215-1271`）、命令卡目录 `CommandCatalog::for_race`、不死族 `3xx` / 暗夜族 `4xx` 段模型键投影、i18n 文案键（按 Q5' / Q6）；Q4 已拍板全量机制（R10.2 / R10.3 / R10.4 / R5.6 / S3 / R8.4）本期纳入，其 HUD 呈现 MUST 仍为表现层只读派生。**P8.5e `FogHudModel` 迷雾只读投影保持表现层只读派生语义不变、fog 可见性 MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`（R10.5 / §0.3 T3 / Q2' 已拍板、不抹平对照差异 D1）**。HUD 只读派生 MUST NOT 反写 `RtsMatch`、进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q3 bump 8）定义的快照或 `first_difference()`；不扩展 `WorldView` / `EntityView` 投影字段（ADR 0002 §5 / P8.4 `runtime-entity-movability` D1 先例）。含地面物品 / 商店（中立建筑）/ 金矿 / 树木 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6）。Q4 已拍板本期含全量机制（不再是后续承接）、Q1' 已拍板双族全量、Q4' 已拍板 1.0 张力七处显式如实登记、不静默窄化 / 不冒充已解决 / 不预设结论；本 change 的 **2 ADDED + 8 MODIFIED = 10 份 delta** 正好不触发 `Consider splitting changes with more than 10 deltas` warning、MUST NOT 新增第 9 份 MODIFIED delta（leader 例行决定 / Q9' / Q10'）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: HUD 只读

- **WHEN** 4 族（人 / 兽 / 不死 / 暗夜）选族 / 命令卡 / 模型键 / Q4 全量机制 HUD / 迷雾连续重绘并校验 `RtsMatch` 写入路径与哈希
- **THEN** `HudModel` 只消费只读视图 / 快照 / 事件、4 族投影与 `FogHudModel` 均为表现层本地只读派生状态，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节和 v8 新基线一致、不进权威状态 / 状态哈希 / 快照 / `first_difference()`；Q2 新增族特色机制顶层权威状态只在 `w3-core` 结算、HUD 只读消费不反写、fog 可见性 MUST NOT 进权威状态（R10.5 / R13.6 / ADR 0002 §5-8 / Q1' / Q2 / Q2' / Q3 / Q4 / Q6）

#### Scenario: 中文本地化文案

- **WHEN** 以不死 / 暗夜选族 overlay / 命令卡 / 生产队列 / Q4 全量机制面板读取 UI 文案并刷新
- **THEN** 4 族文案走 `i18n` 中文键（与既有两族同构）、不进日志 / 权威状态 / 状态哈希，日志与错误信息保持英文结构化 `key=value`、源码注释中文（`CODEBUDDY.md §2 / §3` / R13.6 / Q1' / Q4 / Q5'）

#### Scenario: 资源缺失

- **WHEN** 不死 / 暗夜模型键 / 文案键 / 遮罩资源或 Q4 全量机制呈现资源缺失或段键未命中库
- **THEN** MUST 以英文结构化 `key=value` 日志明确报告并降级为不绘制对应元素、MUST NOT 静默降级或把降级状态写回权威状态；按 Q3 bump 8 的快照契约下逐帧哈希 / 快照字节不变、不改变模拟推进（ADR 0002 §5 / `CODEBUDDY.md §3` / C9 / Q1' / Q2 / Q3 / Q4 / Q5'）
