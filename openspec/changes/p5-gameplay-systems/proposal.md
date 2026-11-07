## Why

P2 只覆盖选中/移动/攻击，距离"复刻经典 RTS 玩法"仍缺技能、英雄、物品、科技、经济、采集、建造、生产与胜负。P5 在这些机制上补齐第一版玩法闭环，同时保持确定性契约。

## What Changes

- 在 `w3-core` 增加技能/效果/状态、英雄与物品、库存与人口、生产/建造队列、科技与胜负判定。
- 建立冻结的内容定义与内容哈希，作为对局配置的一部分。
- 单位/玩家 AI 的命令生成走同一校验入口；重放 MUST NOT 重复注入内部 AI 命令。
- 技能采用三层扩展（模板 / 效果组合 / 注册行为），不引入脚本 VM。

## Capabilities

### New Capabilities

- `gameplay-systems`: 技能、英雄、物品、经济、生产、科技、AI 与胜负的确定性规则。

### Modified Capabilities

- `deterministic-simulation`: 内容哈希 MUST 进入快照校验；新增系统状态 MUST 进入状态哈希。

## Impact

- `w3-core` 权威状态显著扩张；`tests/determinism.rs` 需扩展覆盖施法、Buff 到期、同帧死亡与生产取消。
- 需要冻结的内容定义来源（SLK 表 → 定义 → 内容哈希）。
- JASS / 触发器 VM 仍不在第一版范围（ADR 0003）。
