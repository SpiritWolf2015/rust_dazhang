# Spec Delta

## MODIFIED Requirements

### Requirement: 表现层 HUD 只读派生、零资产、许可证与范围边界

六格背包 / 商店命令卡 / 购买 / 拾取 / 回城进度 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 P8.5a `ReviveHudModel::derive` / P8.5b `derive_upgrade_slot` 范式），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5 / R13.5）。UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。全部数值为合成原创编译期整数 `[待核]`、仓库零资产、许可证边界与范围外承接见下列各 Scenario（ADR 0001 / ADR 0004 / §14 / Q2 / Q3）。**P8.5d 原「P8.5e 迷雾下商店 / 地面物品可见性显式登记为范围外承接」的明文交接现由本 change `fog-of-war` capability 正式承接**：商店（中立建筑）/ 地面物品在迷雾下的可见性 MUST 按「当前可见格」过滤后再喂 `MinimapModel.points` 与 `Scene`，地面物品不是 `EntityStore` 实体、不出现在 `read_view()` 实体集合（`creep.rs:34-38`），其位置 / 类别 SHALL 从只读 `Snapshot.item_shop` / `Snapshot.creep_camps`（`snapshot.rs:139,145`）查表取得，MUST NOT 只靠 `WorldView.entities`（R10.1 / R10.5 / R9.1）。过滤全在表现层只读派生、不反写、不进状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 保持 7 不 bump、`w3-core` 零权威状态改动（R10.5 / ADR 0002 §5 / §6-8 / Q1 / Q4）。本 capability 自身权威状态口径不变。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含「范围边界与后续承接」）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；迷雾交接口径已按 `fog-of-war` 承接写入各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: HUD 只读不改哈希（R10.5 / R13.5 / ADR 0002 §5）

- **WHEN** 连续驱动六格背包 / 商店命令卡 / 购买进度 / 回城施法条 HUD 重绘，以及 P8.5e 迷雾下商店 / 地面物品可见性过滤后的投影重绘，并与无 HUD 实例同脚本比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、HUD 无 `RtsMatch` 字段写入路径、快照不含任何 fog / visibility / explored 字段；`w3-render/tests/isolation.rs` 隔离断言全绿（ADR 0002 §5 / R13.5 / R12.4 / `design.md` D8 / Q9 形态①）；键位 / 命令卡状态为表现层只读派生、不进权威状态（R13.5 / R13.6）

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14 / R7.x）

- **WHEN** 校验本 change 的全部改动与新增常量，含 P8.5e 表现层合成迷雾常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON / PNG / GLB，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine` 仅借鉴语义无代码复制、无 Java / LibGDX 依赖，`rts_kernel_cs` 的 `double` 路线不采用，无新增未登记依赖（ADR 0001 / ADR 0004 / R10.1 / `design.md` 第八节 / §14）

#### Scenario: 范围边界与后续承接（Q2 / Q3 / R10 / R8.4 / R9.3 / §14）

- **WHEN** 校验本 change 的命令面、拾取 / 购买 / 使用闭环与物品 / 商店行为集合，以及 P8.5e 迷雾承接状态
- **THEN** R8.4 昼夜睡眠、AI 玩家清野 / 拾取（R12.7 / §0.3 T5）、R9 中立建筑交互全量仍显式登记为范围外承接、不冒充已解决；**P8.5e 迷雾下商店 / 地面物品可见性（R10 / R10.5）不再属范围外承接**，由 `fog-of-war` capability 承接为表现层只读派生、MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`（R10.1 / R10.5 / Q3）；与本机导出不符的数值标 `[待核]` 并回到人拍板、不静默改规则（§14 / Q2 / Q3 / `design.md` Non-Goals）
