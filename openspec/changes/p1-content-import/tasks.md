## 1. 读取器骨架

- [x] 1.1 新增 `w3-content` crate 与 serde / serde_json 依赖
- [x] 1.2 实现 `TerrainHeightfield` 结构与维度/数组长度校验
- [x] 1.3 实现 `UnitsFile` / `DoodadsFile`（忽略未知字段、count 校验）

## 2. 表格与加载边界

- [x] 2.1 实现 `SlkTable` 与 `find_row`（稳定 BTreeMap 顺序）
- [x] 2.2 实现 `load_map` / `load_table` 与统一 `ContentError`
- [x] 2.3 编写合成 fixture 与单元/集成测试

## 3. 真实数据端到端验证

- [x] 3.1 运行上游 dev-setup 生成 SLK 与地图 JSON（跳过 Godot/bake）
- [x] 3.2 新增 `examples/load_export.rs` 并以真实 Echo Isles 数据加载成功
- [x] 3.3 在 README / UPSTREAM.md 记录路径、绕过方式与合规边界
