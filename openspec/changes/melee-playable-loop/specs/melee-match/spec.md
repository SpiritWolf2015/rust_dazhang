# Spec Delta

## Purpose

定义一局 1v1 Melee 对战的可玩会话契约：从开局布置、资源采集、建造训练、战斗到 AI 对手与胜负结算全程可在窗口内完成，把 `w3-core` 已交付的玩法命令串成「能打完一局」的闭环，并让对局实体可辨识。

> 相关 ADR：ADR 0001（合成定义零资产、导出数据不入库）；ADR 0002（内核零改动、状态哈希 / 快照 / `SNAPSHOT_VERSION` 不变、外部命令按帧冻结）；ADR 0003（第一版以 Melee 对战优先）；ADR 0004（冻结坐标与定点单位）。

## ADDED Requirements

### Requirement: 一局 Melee 对战可装配并打完

系统 SHALL 提供一局 1v1 Melee 对战的确定性会话：开局为双方布置主城、农民与资源点，玩家经窗口输入完成采集、建造、训练、进攻，直到分出胜负；全程 MUST NOT 依赖任何本机导出资产或配置文件。

#### Scenario: 开局布置

- **WHEN** 启动可玩入口
- **THEN** 双方各有主城、农民与资源点，资源与人口有确定初值，且全部权威坐标由冻结整数常量构造

#### Scenario: 打完一局

- **WHEN** 玩家依次采集资源、建造兵营、训练战斗单位并摧毁敌方主城
- **THEN** 内核按既有结算阶段产生唯一胜负结果，HUD 显示胜负，且玩家可重开新一局

#### Scenario: 零资产环境

- **WHEN** 在不含 `config.toml` 与任何导出目录的干净环境运行
- **THEN** 一局仍能启动并打完，不读取任何资产文件

### Requirement: 冻结内容定义零资产

对局 SHALL 以编译期整数常量构造的冻结定义启动，至少覆盖农民、主城、兵营、战斗单位、资源点、技能与升级；内容哈希 MUST 进入快照头部；MUST NOT 读取 `config.toml`、导出 JSON 或任何游戏资产。

#### Scenario: 定义冻结

- **WHEN** 一局开始
- **THEN** `FrozenDefinitions` 校验通过、内容哈希固定，对局中定义不被热更新

#### Scenario: 拒绝不匹配恢复

- **WHEN** 用不同内容哈希的定义恢复快照
- **THEN** 明确拒绝恢复，不产生半初始化的对局

### Requirement: 资源采集

系统 SHALL 允许玩家选中农民并指向资源点进行采集，采集命令经公开提交入口产生，资源随整数逻辑帧确定性增长，且 HUD 可见。

#### Scenario: 命令采集

- **WHEN** 选中农民后对资源点下达采集命令
- **THEN** 生成 `Gather` 玩法命令，资源数值随帧确定增长，选中集合本身不生成权威命令

#### Scenario: 采集不改哈希口径

- **WHEN** 采集推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与相同命令脚本的基线一致

### Requirement: 建造与训练

系统 SHALL 允许玩家用主城或兵营建造建筑、训练单位；生产队列的剩余帧数 MUST 随逻辑帧递减，完成后 MUST 生成实体并按定义扣增资源与人口，HUD MUST 显示队列进度。

#### Scenario: 训练完成生成实体

- **WHEN** 对生产者下达训练命令并推进至队列完成
- **THEN** 生成新单位实体、更新资源与人口，HUD 队列项消失且新实体可见

#### Scenario: 取消生产

- **WHEN** 玩家取消某个生产队列条目
- **THEN** 该条目不再推进、不生成实体，资源按既有规则处理

### Requirement: 确定性 AI 对手

系统 SHALL 为对手玩家提供确定性命令源，按冻结帧号经公开提交入口产生采集、训练与进攻命令；AI 命令 MUST NOT 覆盖仍有效的玩家命令，完整重放 MUST NOT 双重注入内部 AI 命令。

#### Scenario: 对手会行动

- **WHEN** 一局推进若干逻辑帧且未分胜负
- **THEN** 对手单位产生采集、训练或进攻行为，玩家可见其状态变化

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部玩家命令并重新运行 AI 命令源
- **THEN** 逐帧状态哈希与原始对局一致

### Requirement: 胜负结算与重开

胜负 SHALL 在既有内核结算阶段按稳定顺序只结算一次；一方主城被毁或离开后 MUST 产生唯一胜负结果并在 HUD 显示；胜负后 MUST 能以相同冻结定义与开局脚本重新构造一局，不残留上一局权威状态。

#### Scenario: 摧毁主城结算

- **WHEN** 敌方主城被摧毁
- **THEN** 经既有 `Leave` / 胜负路径产生唯一胜者与结算帧，HUD 显示结果

#### Scenario: 同帧只结算一次

- **WHEN** 同一帧内多个胜负条件同时满足
- **THEN** 只产生一次结算与一组事件

#### Scenario: 重开

- **WHEN** 胜负已结算后以相同冻结定义与开局脚本重新构造对局
- **THEN** 新一局从初始权威状态开始，不残留上一局胜负 / 实体 / 资源状态

### Requirement: 对局实体可辨识呈现

系统 SHALL 按实体类型以不同几何、队伍色、选中环与血条呈现农民、战斗单位、建筑与资源点，使玩家可分辨类型与敌我；呈现 MUST NOT 反写权威状态、进入状态哈希或快照。

#### Scenario: 类型与敌我可辨

- **WHEN** 观察含农民、战斗单位、建筑、资源点与双方单位的窗口
- **THEN** 各类实体外观可区分，敌我以队伍色区分，选中单位有选中环与血条

#### Scenario: 呈现只读

- **WHEN** 实体外观随逻辑帧变化并连续重绘
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节和只读基线一致

### Requirement: 内核零改动与确定性契约

本 capability 的所有行为 SHALL 只经 `w3-core` 既有公开 API 与命令实现；`crates/w3-core/` MUST 零改动，MUST NOT 新增权威状态字段，`SNAPSHOT_VERSION` MUST 仍为 3，状态哈希字段集合与快照头部布局 MUST 不变。

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的全部改动
- **THEN** `git diff -- crates/w3-core/` 为空，`cargo test -p w3-core --test determinism` 26 项全绿

#### Scenario: 哈希与快照字节不变

- **WHEN** 跑完一局并与 CPU 基线比对
- **THEN** 逐帧状态哈希与 `capture_snapshot()` 字节与基线一致，快照版本仍为 3
