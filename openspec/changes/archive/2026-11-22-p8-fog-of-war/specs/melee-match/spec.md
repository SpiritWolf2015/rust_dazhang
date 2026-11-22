# Spec Delta

## MODIFIED Requirements

### Requirement: 对局实体可辨识呈现

系统 SHALL 按实体类型以不同几何、队伍色、选中环与血条呈现农民、战斗单位、建筑与资源点，使玩家可分辨类型与敌我；呈现 MUST NOT 反写权威状态、进入状态哈希或快照。本 change P8.5e 把该呈现明确纳入 `fog-of-war` capability 的可见性过滤：已方 / 敌方单位、建筑、资源点、野怪、中立商店与地面物品在 MASKED / FOGGED 格内的变化 SHALL 对本地视角不可见（R10.1），只有 VISIBLE 格的敌方实体变化进入 `Scene` 动态实例 / `MinimapModel.points` / HUD 只读投影（R10.5）。迷雾呈现 / 雾遮罩 MUST NOT 反写 `RtsMatch`、进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 7）定义的快照或 `first_difference()`（R10.5 / ADR 0002 §5 / §6-8）；`w3-core` 保持零权威状态改动、`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变（R10.5 / Q1 / Q4）。含地面物品 / 商店（中立建筑）/ 金矿 / 树木 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 类型与敌我可辨

- **WHEN** 观察含农民、战斗单位、建筑、资源点与双方单位的窗口，且同时存在 VISIBLE / FOGGED / MASKED 格
- **THEN** 各类实体外观可区分、敌我以队伍色区分、选中单位有选中环与血条；P8.5e 起敌方实体只在 VISIBLE 格呈现、MASKED / FOGGED 格敌方单位 / 建筑变化不呈现（R10.1 / R10.5），该可见性过滤是表现层只读派生、不改变实体类型 / 队伍色的既有呈现语义（ADR 0002 §5）

#### Scenario: 呈现只读

- **WHEN** 实体外观随逻辑帧变化并连续重绘，且叠加迷雾可见性过滤与雾遮罩
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节和只读基线一致、迷雾网格 / 已探索层 / 当前可见为表现层本地只读派生状态、不进权威状态 / 状态哈希 / 快照 / `first_difference()`，`crates/w3-render/tests/isolation.rs` 全绿（R10.5 / R12.4 / ADR 0002 §5 / Q9 形态①）
