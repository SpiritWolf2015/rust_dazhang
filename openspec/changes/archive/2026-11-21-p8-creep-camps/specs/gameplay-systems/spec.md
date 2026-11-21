# Spec Delta

## MODIFIED Requirements

### Requirement: Shared effect settlement

生命/法力、伤害、治疗、Buff、属性修正与目标过滤 SHALL 由统一结算服务处理；技能、物品、攻击 MUST NOT 各自绕过结算改血；费用提交事务与效果执行链 MUST 分离。该分离 MUST 覆盖 P8.5a 复活与 P8.5b 主城升级的 gold / lumber 扣费提交与队列效果执行：取消 / 主城被毁 / 对局失败 MUST 事务性退款并恢复升级登记、MUST NOT 重复扣费或半升级（R3.1 / R4.1 / R6.3 / ADR 0002）。本 change P8.5c 野怪营地与掉落按推荐方向 A（待规格门 Q1 拍板）把该统一结算服务明确覆盖**野怪击杀收益结算**（经验给附近英雄 R6.1 / 金币 R8.3 / 掉落 R7.2，MUST 按稳定 `EntityId` 顺序、只结算一次、经验分摊「进入集合 vs 实际入账」两环节 R6.5）与**掉落随机码消费**（死亡事件幂等、按稳定 `EntityId` 顺序确定性消费、消费序列进状态哈希 / 快照，R8.6）；击杀收益与掉落事实同样 MUST NOT 由表现层反写 `HeroState.xp` / `PlayerEconomy.gold` / 背包（ADR 0002 §5）。P8.5b 新增的「主城等级 / tier」顶层权威状态 / 升级队列 / 费用登记 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`；P8.5c 新增的野怪 / 营地 / leash / home 状态、掉落表消费序列、地面物品 / 物品实例、确定性随机码消费序列 MUST 同样同步进 FNV-1a 状态哈希、快照与 `first_difference()`，遗漏 MUST 被确定性验收反例断言拦截；`SNAPSHOT_VERSION` 按 Q4 推荐（待规格门拍板）bump 5→6（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v5 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）。既有 Purpose 正文不改写。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 目标死亡后的后续效果

- **WHEN** 一组效果中先造成伤害使目标死亡，后续效果再施加眩晕
- **THEN** 后续效果返回 `NoEffect`，已提交的效果不回滚、不重复扣费（主 `openspec/specs/gameplay-systems/spec.md:23-26` / R11.2）

#### Scenario: 复活费用提交与取消退款事务分离

- **WHEN** 提交祭坛复活命令冻结扣费、随后取消队列或队列因祭坛被毁 / 对局失败中止
- **THEN** 扣费提交与队列效果执行链分离：`gold / lumber` 精确回退、死亡登记恢复可再次复活、不生成半复活实体，逐帧 `compute_state_hash()` 与相同脚本基线一致（R6.3 / R3.1 / ADR 0002 §6-8）

#### Scenario: 复活新增权威状态遗漏进哈希被拦截

- **WHEN** 新增死亡登记 / 复活队列 / 费用字段，或 P8.5b 新增 tier 顶层权威状态 / 升级队列 / 费用登记，或 P8.5c 新增野怪 / 营地 / 地面物品 / 物品实例 / 确定性随机码消费序列 / 掉落表消费序列，未加入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败，阻止合并；tier 顶层权威状态与 P8.5c 全部新增权威状态 MUST 完整进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 5→6（历史标题「复活」字面保留作身份键、不代表只覆盖复活，详见本 requirement 下 `>` 身份键说明）（R8.6 / ADR 0002 §8 / `CODEBUDDY.md §5` / 主 `openspec/specs/gameplay-systems/spec.md:4` / Q9）
