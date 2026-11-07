## ADDED Requirements

### Requirement: Load terrain heightfield with validation

系统 SHALL 从 `terrain-heightfield.json` 读取 header 与顶点平行数组；`heights` MUST 恰好有 `tilepointWidth × tilepointHeight` 项；任何存在的可选平行数组 MUST 与顶点数同长；`tilepointWidth` / `tilepointHeight` MUST 等于 `mapWidth` / `mapHeight` 加一。

#### Scenario: 合法高度场

- **WHEN** 加载尺寸与数组长度一致的导出
- **THEN** 返回 `ParsedMap`，且 `height_at(ix, iy)` 按 `iy * width + ix` 取值

#### Scenario: 数组长度不符

- **WHEN** 某个平行数组长度与顶点数不一致
- **THEN** 返回 `WrongArrayLength` 错误，不返回部分数据

### Requirement: Load placements tolerantly

系统 SHALL 从 `units.json` / `doodads.json` 读取所需字段；上游其余字段 MUST 被忽略；声明的 `count` 与条目数不一致 MUST 被拒绝；文件缺失 SHALL 视为空列表。

#### Scenario: 上游新增字段

- **WHEN** JSON 中出现未知字段（inventory、abilities、scale 等）
- **THEN** 读取器仍成功，且已声明字段的值不变

#### Scenario: 仅有地形

- **WHEN** 目录不含 `units.json` / `doodads.json`
- **THEN** `ParsedMap` 的 units 与 doodads 为空，不报错

### Requirement: Keep exported assets outside the repository

导出 JSON 及其派生的 PNG / GLB / 音效 MUST NOT 被提交进仓库；读取路径 SHALL 由本机配置提供。

#### Scenario: 运行时读取

- **WHEN** `load_export` 收到本机导出目录与 SLK 表路径
- **THEN** 打印统计，且仓库内不新增任何资产文件
