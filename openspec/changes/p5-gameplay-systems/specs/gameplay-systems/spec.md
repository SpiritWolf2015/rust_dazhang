## ADDED Requirements

### Requirement: Frozen content definitions

对局开始前 SHALL 校验并冻结单位/技能/物品/升级定义；内容哈希 MUST 进入快照头部；内容不匹配 MUST 被拒绝恢复；对局中 MUST NOT 热更新定义。

#### Scenario: 内容变更

- **WHEN** 用不同内容哈希的定义恢复快照
- **THEN** 明确拒绝，不产生半初始化的对局

### Requirement: Shared effect settlement

生命/法力、伤害、治疗、Buff、属性修正与目标过滤 SHALL 由统一结算服务处理；技能、物品、攻击 MUST NOT 各自绕过结算改血；费用提交事务与效果执行链 MUST 分离。

#### Scenario: 目标死亡后的后续效果

- **WHEN** 一组效果中先造成伤害使目标死亡，后续效果再施加眩晕
- **THEN** 后续效果返回 `NoEffect`，已提交的效果不回滚、不重复扣费

### Requirement: Deterministic AI command source

玩家与玩家 AI SHALL 经相同命令校验入口；内部 AI 命令在完整重放中 MUST NOT 被再次注入；单位自动索敌 MUST NOT 覆盖仍有效的玩家命令。

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部玩家命令并重新运行确定性 AI
- **THEN** 逐帧状态哈希与原始对局一致

### Requirement: Victory and cleanup

胜负判定 SHALL 在既定帧阶段完成；玩家离开、House 销毁与清理 MUST 通过唯一所有者，不产生竞争状态。

#### Scenario: 同帧胜负

- **WHEN** 同一帧内多个胜负条件同时满足
- **THEN** 按稳定顺序只产生一次结算与一组事件
