# Spec Delta

## MODIFIED Requirements

### Requirement: Shared effect settlement

生命/法力、伤害、治疗、Buff、属性修正与目标过滤 SHALL 由统一结算服务处理；技能、物品、攻击 MUST NOT 各自绕过结算改血；费用提交事务与效果执行链 MUST 分离。该分离 MUST 覆盖 P8.5a 复活 / P8.5b 主城升级的 gold / lumber 扣费提交与队列效果执行、P8.5c 野怪击杀收益结算与掉落随机码消费，以及本 change P8.5d 的**商店购买扣费提交与入包 / 库存递减效果执行**、**售卖回购价入账**、**使用道具的 `EffectDef` 效果链**：购买 MUST 为「距离 / 库存 / 黄金 / 背包空位校验 → 扣费 → 建实例 → 入六格背包 → 库存递减」事务，满包 / 资源不足 / 库存售罄 / 使用失败 MUST 事务性退款或恢复登记、MUST NOT 重复扣费或半入包或误扣次数（R7.3 / R7.4 / R7.5 / R7.6 / R9.3）。本 change P8.5d 按推荐方向 A（待规格门 Q1 拍板）把该统一结算服务明确覆盖拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖 / 商店库存刷新 / 回城传送结算与确定性随机码消费（复用 P8.5c `rng_state` / `rng_draws` / `next_rng_seq` 事件驱动流 + 显式消费序列，R8.6）；这些事实 MUST NOT 由表现层反写 `HeroState.xp` / `PlayerEconomy.gold` / 六格背包 / 商店库存（ADR 0002 §5 / R9.3）。P8.5c 的野怪 / 营地 / 掉落 / 地面物品 / 物品实例权威状态、本 change P8.5d 的六格背包 / 实例 owner 与 slot / 商店库存 / 回城传送 / 消费序列 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，遗漏 MUST 被确定性验收反例断言拦截；`SNAPSHOT_VERSION` 按 Q4 推荐（待规格门拍板）bump 6→7（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）。既有 Purpose 正文不改写。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 目标死亡后的后续效果

- **WHEN** 一组效果中先造成伤害使目标死亡，后续效果再施加眩晕
- **THEN** 后续效果返回 `NoEffect`，已提交的效果不回滚、不重复扣费（主 `openspec/specs/gameplay-systems/spec.md:23-26` / R11.2）；道具使用效果同样只经既有 `EffectDef` 组合、失败不扣 `charges`（R7.6 / R7.5）

#### Scenario: 复活费用提交与取消退款事务分离

- **WHEN** 提交祭坛复活命令冻结扣费、随后取消队列或队列因祭坛被毁 / 对局失败中止，并对比商店购买满包 / 黄金不足 / 库存售罄的失败路径
- **THEN** 扣费提交与队列效果执行链 / 入包效果执行链分离：`gold / lumber` 精确回退、死亡登记或商店库存恢复、不生成半复活 / 半入包实体、使用失败不误扣 `charges`（R6.3 / R3.1 / R7.3 / R7.5 / R9.3），逐帧 `compute_state_hash()` 与相同脚本基线一致（R12.4 / ADR 0002 §6-8）

#### Scenario: 复活新增权威状态遗漏进哈希被拦截

- **WHEN** 新增死亡登记 / 复活队列 / 费用字段，或 P8.5b tier / P8.5c 野怪 / 营地 / 地面物品 / 物品实例 / 确定性随机码消费序列，或 P8.5d 六格背包 / 实例 owner 与 slot / 充能冷却 / 商店库存与刷新 / 回城传送（施法帧 / 目标 / 中断）/ 消费序列，未加入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败，阻止合并；tier 与 P8.5c / P8.5d 全部新增权威状态 MUST 完整进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7（历史标题「复活」字面保留作身份键、不代表只覆盖复活，详见本 requirement 下 `>` 身份键说明）（R8.6 / R7.5 / R9.3 / R12.4 / ADR 0002 §8 / `CODEBUDDY.md §5` / 主 `openspec/specs/gameplay-systems/spec.md:4` / Q9）
