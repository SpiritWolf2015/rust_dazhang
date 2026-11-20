# gameplay-systems Specification

## Purpose
本规范定义第一版玩法闭环的确定性规则：技能与效果由统一结算服务处理（伤害、治疗、Buff 与属性修正收敛到唯一所有者），费用提交事务与效果执行链分离；单位/技能/物品/升级定义在开局冻结，内容哈希进入快照头部并拒绝不匹配的恢复；玩家与玩家 AI 经同一命令校验入口，重放只注入外部命令、不重复注入内部 AI 命令；胜负判定与玩家离开、House 销毁、清理由唯一所有者在既定帧阶段按稳定顺序完成。玩法闭环在 `w3-core` 内以定点整数与稳定顺序实现，新增权威状态同步进入状态哈希与快照，保持 ADR 0002 确定性契约。

## Requirements

### Requirement: Frozen content definitions

对局开始前 SHALL 校验并冻结单位/技能/物品/升级定义；内容哈希 MUST 进入快照头部；内容不匹配 MUST 被拒绝恢复；对局中 MUST NOT 热更新定义。

> 相关 ADR：ADR 0002（内容哈希进入快照头部、定义冻结不热更新）；ADR 0001（定义源自玩家本机导出数据，资产不入库）；ADR 0004（冻结坐标 / 浮点拒绝边界）。

#### Scenario: 内容变更

- **WHEN** 用不同内容哈希的定义恢复快照
- **THEN** 明确拒绝，不产生半初始化的对局

### Requirement: Shared effect settlement

生命/法力、伤害、治疗、Buff、属性修正与目标过滤 SHALL 由统一结算服务处理；技能、物品、攻击 MUST NOT 各自绕过结算改血；费用提交事务与效果执行链 MUST 分离。该分离 MUST 覆盖 P8.5a 复活与 P8.5b 主城升级的 gold / lumber 扣费提交与队列效果执行：取消 / 主城被毁 / 对局失败 MUST 事务性退款并恢复升级登记、MUST NOT 重复扣费或半升级（R3.1 / R4.1 / R6.3 / ADR 0002）。P8.5b 新增的「主城等级 / tier」顶层权威状态 / 升级队列 / 费用登记 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q1 推荐 bump 4→5；连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）定义的快照与 `first_difference()`（ADR 0002 §6-8 / `CODEBUDDY.md §5`）。既有 Purpose 正文不改写。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 目标死亡后的后续效果

- **WHEN** 一组效果中先造成伤害使目标死亡，后续效果再施加眩晕
- **THEN** 后续效果返回 `NoEffect`，已提交的效果不回滚、不重复扣费（主 `openspec/specs/gameplay-systems/spec.md:23-26` / R11.2）

#### Scenario: 复活费用提交与取消退款事务分离

- **WHEN** 提交祭坛复活命令冻结扣费、随后取消队列或队列因祭坛被毁 / 对局失败中止
- **THEN** 扣费提交与队列效果执行链分离：`gold / lumber` 精确回退、死亡登记恢复可再次复活、不生成半复活实体，逐帧 `compute_state_hash()` 与相同脚本基线一致（R6.3 / R3.1 / ADR 0002 §6-8）

#### Scenario: 复活新增权威状态遗漏进哈希被拦截

- **WHEN** 新增死亡登记 / 复活队列 / 费用字段，或 P8.5b 新增 tier 顶层权威状态 / 升级队列 / 费用登记，未加入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败，阻止合并；tier 顶层权威状态 MUST 完整进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（历史标题「复活」字面保留作身份键、不代表只覆盖复活，详见本 requirement 下 `>` 身份键说明）（ADR 0002 §8 / `CODEBUDDY.md §5` / 主 `openspec/specs/gameplay-systems/spec.md:4` / Q8）

### Requirement: Deterministic AI command source

玩家与玩家 AI SHALL 经相同命令校验入口；内部 AI 命令在完整重放中 MUST NOT 被再次注入；单位自动索敌 MUST NOT 覆盖仍有效的玩家命令。

> 相关 ADR：ADR 0002（命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；同源校验与重放不双重执行保证逐帧哈希一致）。

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部玩家命令并重新运行确定性 AI
- **THEN** 逐帧状态哈希与原始对局一致

### Requirement: Victory and cleanup

胜负判定 SHALL 在既定帧阶段完成；玩家离开、House 销毁与清理 MUST 通过唯一所有者，不产生竞争状态。

> 相关 ADR：ADR 0002（既定帧阶段与稳定顺序结算；唯一所有者避免竞争状态，保持逐帧哈希一致）。

#### Scenario: 同帧胜负

- **WHEN** 同一帧内多个胜负条件同时满足
- **THEN** 按稳定顺序只产生一次结算与一组事件
