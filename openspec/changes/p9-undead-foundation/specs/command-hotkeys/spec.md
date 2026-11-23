# Spec Delta

## MODIFIED Requirements

### Requirement: 命令卡 12 格由定义层数据派生并随选中类型切换

命令卡 12 格 SHALL 由定义层数据派生并随选中类型切换；`RACE_ROWS`（`command_card.rs:239`）SHALL 按 Q1' 已拍板扩到 **4**、`RACE_TRAIN_TARGETS`（`:254-277`）2 行×9 列扩到 **4 行×9 列**（新增不死族 `3xx` / 暗夜族 `4xx` 段目标），`race_row()`（`:299-304`）仍为唯一 `match Race` 单一查表处、`DEFAULT_COMMAND_TARGETS`（`:283`）缺失回落人族段 + 英文结构化 `key=value` 日志显式报告、MUST NOT 静默降级（`CODEBUDDY.md §3` / R1.2 / R13.1 / R13.3 / D6）。键位与命令解耦、命令卡 / 编队 / 子菜单 MUST 为表现层只读派生、不进权威状态 / 哈希 / 快照（R13.6 / ADR 0002 §5 / R10.5）；族分派 MUST 由定义层数据驱动、MUST NOT 散落 `if race` 白名单（`BUILD_SYSTEM.md:296-298`）。Q2 / Q3 新增 `GameplayCommandKind` 码 24 起的族特色机制命令 MUST 经既有公开命令入口结算、不进快捷键 / 命令卡白名单（由 `specs/undead-faction/spec.md`「新增命令 / 事件变体」Requirement 承接）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 选中农民与选中生产者的命令卡不同（R13.1）

- **WHEN** 在 4 族（人 / 兽 / 不死 / 暗夜）下分别选中农民（工人 / 小精灵）与选中生产者（主城 / 军事建筑 / 古树）并读取命令卡 12 格
- **THEN** 同族内两卡内容不同、各族按 `RACE_TRAIN_TARGETS` 命中本族段目标（含 `3xx` / `4xx`），`race_row()` 单一查表处返回 0 / 1 / 2 / 3，不散落 `if race` 白名单；命令卡为只读派生、按 Q3 bump 8 的 `SNAPSHOT_VERSION` 快照契约下仍不进权威状态 / 哈希 / 快照（R13.1 / R1.2 / R2 / D6 / Q1' / Q3 / Q5' / Q6）

#### Scenario: 键位与命令解耦（R13.1）

- **WHEN** 校验 4 族命令卡格位与键位映射的实现，并枚举 Q2 / Q3 新增 `GameplayCommandKind` 码 24 起
- **THEN** 键位绑定与命令 ID 解耦、由定义层命令卡格位数据驱动，新增不死 / 暗夜目标不引入硬编码族名 / `if race` 白名单；新码 24 起（既有 1–23 不动、校验 / 穷尽分发三处同步）不进快捷键白名单、`DEFAULT_COMMAND_TARGETS` 缺失回落人族段并以英文 `key=value` 日志明确报告（R13.1 / R1.2 / `CODEBUDDY.md §3` / ADR 0005 §3 / Q1' / Q2 / Q3 / Q5'）

#### Scenario: 空选中不显示命令卡且不生成命令（R13.1）

- **WHEN** 在 4 族任一装配下清空选中集合并触发命令卡刷新 / 快捷键
- **THEN** 不显示命令卡、不生成任何权威命令，逐帧 `compute_state_hash()` 与 v8 新基线一致；含金矿 / 树木 / 主城 / 新增不死 / 暗夜建筑（含 rooted 古树）的选中集合 MUST NOT 生成 `Move`、拔根古树可移动性由定义层 `speed_raw` 数据判定（R1.2 / R1.1 / R1.4 / R13.1 / R13.6 / Q3 / `openspec/config.yaml:57-58`）
