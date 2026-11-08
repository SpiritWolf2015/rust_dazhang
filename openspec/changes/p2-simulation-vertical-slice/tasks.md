## 1. 坐标契约

- [x] 1.1 新增 ADR：冻结 `w3-content` f64 坐标到 `SimFixed` 的缩放、舍入与单位
- [x] 1.2 在 app 层实现冻结地图数据注入 `w3-core`（内核仍不依赖内容层）
- [x] 1.3 用 Echo Isles 坐标量级（合成常量，资产不入库）边界测试验证无溢出

## 2. 订单与移动

- [x] 2.1 增加单位订单、位置、朝向、速度的权威状态与唯一写入所有者
- [x] 2.2 实现八方向寻路、动态障碍与帧命令增删占地
- [x] 2.3 实现按帧转向、坡度减速与确定性移动积分
- [x] 2.4 实现群体落点、固定工作量分帧规划与拥堵失败续队列

## 3. 攻击竖切与验收

- [x] 3.1 实现同帧攻击结算、生命归零即时置死与存活复核
- [x] 3.2 扩展 `compute_state_hash` / `capture_snapshot` / `first_difference`
- [x] 3.3 扩展 `tests/determinism.rs`：移动、编队、同帧死亡场景
- [x] 3.4 通过 `cargo test --workspace` 与 `cargo fmt --all -- --check`
