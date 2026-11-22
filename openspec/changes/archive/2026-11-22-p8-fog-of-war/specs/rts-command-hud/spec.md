# Spec Delta

## MODIFIED Requirements

### Requirement: 小地图

HUD SHALL 呈现只读小地图，显示地形轮廓、己方/敌方单位、建筑、资源点位置与相机视口框；点击小地图 SHALL 只平移表现层相机，MUST NOT 生成权威命令。本 change P8.5e 承接 `fog-of-war` capability：`derive_minimap` MUST 在把单位点喂入 `MinimapModel.points` **之前**按三态迷雾网格完成可见性过滤，把「在场性」（`EntityView.alive`）与「可见性」（迷雾派生）分两层（R10.1 / R10.5，参考对照结论 R2），只有 VISIBLE 格的敌方单位点 SHALL 呈现、MASKED / FOGGED 格敌方单位点 MUST NOT 呈现；野怪 / 商店 / 地面物品的位置 MUST 从只读 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（P8.5c / P8.5d 交接，R8.1 / R9.1）。过滤全为表现层只读派生，MUST NOT 反写 `RtsMatch` / 进状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5）；三态网格 / 已探索层 / 当前可见为表现层本地状态，`SNAPSHOT_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变、`w3-core` 零权威状态改动（R10.5 / ADR 0002 §6-8 / Q1 / Q4）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 delta 新增一条 `#### Scenario:`（喂入前可见性过滤），历史两条逐字保留。

#### Scenario: 小地图显示实体

- **WHEN** 一局进行中且存在多个实体
- **THEN** 小地图按队伍色标出实体位置并绘制当前相机视口框；P8.5e 起该显示 MUST 先经迷雾可见性过滤，MASKED / FOGGED 格敌方单位点不呈现、只有 VISIBLE 格敌方单位点进入 `MinimapModel.points`（R10.1 / R10.5 / `fog-of-war/spec.md`「小地图单位点可见性过滤」），己方单位按本地视角呈现、底图 / 视口框独立于迷雾遮罩，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节不变（ADR 0002 §5）

#### Scenario: 点击小地图平移

- **WHEN** 玩家点击小地图某处
- **THEN** 相机平移到该处，逐帧状态哈希与快照字节不变；点击小地图 MUST NOT 生成 `Move` / 任何权威命令，迷雾过滤不改变该只读语义（R10.1 / R10.5 / ADR 0002 §5 / R1.1 / R1.4）

#### Scenario: 单位点喂入前按迷雾可见性过滤（R10.1 / R10.5 / P8.5c / P8.5d）

- **WHEN** 一局中同时存在 VISIBLE / FOGGED / MASKED 格内的己方、敌方单位、野怪、中立商店与掉落地面物品并刷新小地图
- **THEN** `derive_minimap` 在喂入 `MinimapModel.points` 前完成可见性过滤（R10.1 / R10.5 / `hud.rs:653-724,791-808`），野怪 / 商店 / 地面物品位置经只读 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（`creep.rs:34-38` 地面物品不在 `read_view()` 实体集合，R8.1 / R9.1 / P8.5c / P8.5d 交接）；「不做过滤、直接 `view.entities.iter().filter(alive)` 全量喂入」或「把迷雾网格塞进 `Snapshot`」的退化实现令隔离回归断言失败（Q9 形态① / R10.5 / ADR 0002 §5）
