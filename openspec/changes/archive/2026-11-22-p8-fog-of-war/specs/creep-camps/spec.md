# Spec Delta

## MODIFIED Requirements

### Requirement: 表现层 HUD 只读派生

野怪营地状态 / 掉落事实 / 击杀收益 / 地面物品 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` 与 P8.5b `derive_upgrade_slot` 范式），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5）。只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单 / `f64`；UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。**P8.5c 原「迷雾下野怪可见性留 P8.5e」的明文交接现由本 change `fog-of-war` capability 正式承接**：野怪 / 营地 / 地面物品在迷雾下的可见性 MUST 按「当前可见格」过滤后再喂 `MinimapModel.points` 与 `Scene`，位置 / 类别经只读 `Snapshot.creep_camps` 查表（R10.1 / R10.5 / R8.1）；过滤全在表现层只读派生、不反写、不进状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 保持 7 不 bump、`w3-core` 零权威状态改动（R10.5 / ADR 0002 §5 / §6-8 / Q1 / Q4）。本 capability 自身权威状态口径不变。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含「只读派生边界与迷雾留后续」）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；迷雾交接口径已按 `fog-of-war` 承接写入各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: HUD 只读不改哈希（R10.5 / ADR 0002 §5）

- **WHEN** 连续驱动野怪血条 / 掉落标记 / 击杀收益 HUD 重绘，以及 P8.5e 迷雾下野怪可见性过滤后的投影重绘，并与无 HUD 实例同脚本比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、HUD 无 `RtsMatch` 字段写入路径、快照不含任何 fog / visibility / explored 字段；`w3-render/tests/isolation.rs` 隔离断言全绿（ADR 0002 §5 / R10.5 / Q9 形态① / `design.md` D6）

#### Scenario: 只读派生边界与迷雾留后续（R10.5 / Q3）

- **WHEN** 校验本 change 的只读视图字段与野怪可见性路径
- **THEN** 只读投影不含墙钟 / `static var` 全局字典 / 建筑名白名单 / `f64`；历史标题「迷雾留后续」字面保留作身份键、**迷雾下野怪可见性不再留后续**，现由 `fog-of-war` capability 承接为表现层只读派生、MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`（R10.5 / §0.3 T3 / R8.1 / Q3 / `fog-of-war/spec.md`「P8.5c / P8.5d 野怪、商店与地面物品可见性承接」）

### Requirement: 零资产、许可证与范围边界

野怪 DefId / 单位名 / 营地分级 / 掉落清单与概率 / 经验金币 / leash 半径 / `acquisitionRange` / 昼夜时长 / 随机池类别编码全为合成原创编译期整数常量或 `[待核]`（B 类），MUST NOT 含 MPQ 派生 / 真实导出 JSON / 参考项目单位名 / rawcode / 原创数值、MUST NOT 写进 `WC3-MELEE-RULES.md` 当经典规则（§14 / ADR 0001 / ADR 0004）。`w3-content` 原始 `f64` 坐标不进哈希 / 快照；`WarsmashModEngine`（AGPLv3）**仅借语义、禁复制 Java / LibGDX 代码**，`rts_kernel_cs` 的 `double` 路线不采用（ADR 0001 / ADR 0002）。拾取 / 背包 / 商店 / 回城卷轴（P8.5d）、昼夜（R8.4）、AI 清野（§0.3 T5）、中立建筑（R9.3）登记为范围外承接（Q2 / Q3）；**迷雾（P8.5e）不再登记为范围外承接——由本 change `fog-of-war` capability 正式承接**，其视野半径 / 格子分辨率 / 雾遮罩不透明度等全为表现层合成原创编译期常量 `[待核]`、不进 `FrozenDefinitions` / `defs_hash`（R10.1 / R10.5 / ADR 0001 / §14 / Q6）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；范围边界口径以各 Scenario 的 THEN 为准。

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14）

- **WHEN** 校验本 change 的全部改动与新增常量，含 P8.5e 表现层合成迷雾常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine` 仅借鉴语义无代码复制、无 Java / LibGDX 依赖，`rts_kernel_cs` 的 `double` 路线不采用，无新增未登记依赖（ADR 0001 / ADR 0004 / R10.1 / `design.md` 结论 4 / 结论 6 / 第八节）

#### Scenario: 范围边界与后续承接（Q2 / Q3 / R8.4 / R9.3）

- **WHEN** 校验本 change 的命令面、掉落闭环与野怪行为集合，以及 P8.5e 迷雾承接状态
- **THEN** 拾取 / 六格背包 / 商店购买 / 回城卷轴（P8.5d）、昼夜睡眠（R8.4）、AI 玩家清野（R12.7 / §0.3 T5）、中立建筑交互（R9.3）仍显式登记为范围外承接、不冒充已解决；**迷雾下野怪可见性（P8.5e）不再属范围外承接**，由 `fog-of-war` capability 承接为表现层只读派生（R10.1 / R10.5 / Q3）；与本机导出不符的数值标 `[待核]` 并回到人拍板、不静默改规则（§14 / Q2 / Q3 / `design.md` Non-Goals）
