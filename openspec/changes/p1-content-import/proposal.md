## Why

P0 的确定性内核需要真实地图与单位数据才能进入玩法阶段，但 WC3 的 MPQ / SLK / MDX / W3X 原生解析工作量巨大。上游 `godot_warcraft3` 已有成熟导出管线，P1 以最小成本对接其 JSON 产物，让核心先跑通。

## What Changes

- 新增 `w3-content` crate（serde / serde_json）：读取 `terrain-heightfield.json`、`units.json`、`doodads.json`、`slk-exported/*.json`。
- 新增维度、平行数组长度、`count` 一致性校验与统一 `ContentError`。
- 新增 `load_map` / `load_table` 与 `examples/load_export.rs` 端到端校验入口。
- 新增合成 JSON fixture（不含任何游戏派生资产）。

## Capabilities

### New Capabilities

- `content-import`: 从本机导出的 JSON 读取地形、摆放与表格数据，并在边界校验一致性。

### Modified Capabilities

（无。P0 的 `deterministic-simulation` 需求不变。）

## Impact

- 新增 workspace 成员 `crates/w3-content`；`w3-core` 不依赖它。
- 导出 JSON 来自玩家本机正版客户端，MUST 留在仓库外；`.gitignore` 已排除相关路径。
- 坐标以原始 f64 作为冻结输入；P2 进入模拟前 MUST 冻结 `SimFixed` 转换与舍入规则。
- 已用真实 Echo Isles 数据端到端验证：129×97 顶点、97 单位、2491 装饰物、UnitBalance 836 行。
