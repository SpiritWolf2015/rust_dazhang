# melee-match Specification

## Purpose
定义一局 1v1 Melee 对战的可玩会话契约：从开局布置、资源采集、建造训练、战斗到 AI 对手与胜负结算全程可在窗口内完成，把 `w3-core` 已交付的玩法命令串成「能打完一局」的闭环，并让对局实体可辨识。

> 相关 ADR：ADR 0001（合成定义零资产、导出数据不入库）；ADR 0002（内核零改动、状态哈希 / 快照 / `SNAPSHOT_VERSION` 不变、外部命令按帧冻结）；ADR 0003（第一版以 Melee 对战优先）；ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: 一局 Melee 对战可装配并打完

系统 SHALL 提供一局 1v1 Melee 对战的确定性会话：开局为双方布置主城、农民与资源点，玩家经窗口输入完成采集、建造、训练、进攻，直到分出胜负。对局的**权威逻辑闭环**（`FrozenDefinitions`、开局命令脚本、内核推进与胜负结算）MUST NOT 依赖任何本机导出资产或配置文件；真实地图表现层（地形 / 地表贴图 / 小地图底图）经 gitignored 的本机 `config.toml` 可选接入，属只读表现层接缝，MUST NOT 进入权威状态、状态哈希或快照（ADR 0001 / ADR 0002）。

#### Scenario: 开局布置

- **WHEN** 启动 `melee` 可玩入口（需 gitignored `config.toml` 指向本机 Echo Isles 导出）
- **THEN** 双方各有主城、农民与资源点，资源与人口有确定初值，且全部权威坐标由冻结整数常量构造

#### Scenario: 打完一局

- **WHEN** 玩家依次采集资源、建造兵营、训练战斗单位并摧毁敌方主城
- **THEN** 内核按既有结算阶段产生唯一胜负结果，HUD 显示胜负，且玩家可重开新一局

#### Scenario: CPU 对局装配零资产

- **WHEN** 在不含 `config.toml` 与任何导出目录的干净环境运行 CPU 对局装配（`MeleeDriver` / `melee_loop.rs`）或既有 `playable` 合成入口
- **THEN** 一局仍能启动并打完，权威逻辑不读取任何资产文件

#### Scenario: 真实视觉入口依赖本机导出

- **WHEN** 启动 `melee` 可玩 example，且已提供 gitignored 的 `config.toml` 指向本机 Echo Isles 导出
- **THEN** 真实地形 / 地表贴图 / 小地图底图作为只读表现层呈现，权威状态、状态哈希与快照不变
- **WHEN** 启动 `melee` 可玩 example 但 `config.toml` 缺失或导出数据无效
- **THEN** 显式失败、不回落合成色块 demo（与 `proposal.md` / `design.md` D7 一致）

> 该入口对 gitignored 本机 `config.toml` 的依赖属表现层接缝，MUST NOT 进入权威状态、状态哈希或快照（ADR 0001 / ADR 0002）。

### Requirement: 冻结内容定义零资产

对局 SHALL 以编译期整数常量构造的冻结定义启动，至少覆盖农民、主城、兵营、战斗单位、资源点、技能与升级；内容哈希 MUST 进入快照头部；**`FrozenDefinitions` 构造路径** MUST NOT 读取 `config.toml`、导出 JSON 或任何游戏资产（真实导出数据仅用于只读表现层，见「一局 Melee 对战可装配并打完」的边界澄清）。

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

本 requirement 标题沿用 P6 历史契约名，自 P8.5a 起口径按用户 2026-11-19 拍板修订：本 capability 已交付的历史行为 SHALL 只经 `w3-core` 既有公开 API 与命令实现；P8.5a 英雄复活 change MUST 按 ADR 0002 全流程改 `w3-core` 真复活，新增死亡登记 / 复活队列 / 费用等权威状态 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`（R12.4 / ADR 0002 §6-8）。`GameplayCommandKind` 按 Q2 拍板新增独立 `ReviveHero` / `CancelRevive` 变体；determinism 26 项全绿、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（`CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10`「相关 ADR」写法与本 change `design.md` Context「MODIFIED delta 的身份键说明」）。

#### Scenario: 内核零 diff

- **WHEN** 校验 P8.5a 英雄复活对 `crates/w3-core/` 的全部改动
- **THEN** `git diff -- crates/w3-core/` 非空属预期，新增权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」反向验证，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4，MUST NOT 沿用「零 diff / 仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 哈希与快照字节不变

- **WHEN** 跑完一局并与 P8.5a 新实现的 CPU 基线比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与新实现基线一致，golden hash 基线按新实现如实更新、不沿用旧值，`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），MUST NOT 沿用「仍为 3 / 逐字节不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 四组合双族对局的会话装配与逐帧哈希一致（R2 / R11 / R12.4 / R12.7）

一局 1v1 Melee 会话 SHALL 支持 `(local_race, ai_race)` 双族参数装配 2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）并各打完一局，AI 命令 SHALL 按 AI 自身种族产出（R2 / R12.7）。四组合逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 以「同一组合两实例一致」验收，`resolve_victory` MUST 只结算一次、重开 MUST 无残留（R11.1 / R11.4 / R11.5 / R12.4 / ADR 0002）。双族参数只停装配层；P8.5a 复活 change 改 `w3-core` 后，四组合基线 MUST 按新实现重取、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），`git diff -- crates/w3-core/` 非空属预期，MUST NOT 断言与 P8.5a 前旧基线逐字节相等、MUST NOT 沿用「仍为 3 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10`「相关 ADR」写法与本 change `design.md` Context「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合会话装配并打完一局（R11.1 / R11.4 / R12.7）

- **WHEN** 四种组合各以双族参数装配会话、注入完整一局脚本推进到敌方主城被毁、触发 `Leave`
- **THEN** 四组合均产生唯一胜负结果且只结算一次、按稳定顺序，AI 对手的训练 / 施法 / 研发命令落在 `ai_race` 段，会话全程只经 `w3-core` 既有公开命令面（R11.1 / R11.4 / R12.7 / 主「确定性 AI 对手」）

#### Scenario: 组内逐帧哈希一致且既有两族基线不破坏（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本逐帧推进，并复核既有两族基线 / 回落测试
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等（不只比最终 hash）；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` 仍绿、不静默改其断言；MUST NOT 断言四组合之间或与 P8.5a 前旧基线哈希数值相等（R12.4 / ADR 0002 / C4）

#### Scenario: 重开干净与快照版本不变（R11.5 / ADR 0002）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重开，并校验 `SNAPSHOT_VERSION`
- **THEN** 新一局 `victory.resolved == false`、视图为空、不残留上一局权威状态与复活登记；`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）、golden hash 基线按新实现如实更新，`git diff -- crates/w3-core/` 非空属 P8.5a 预期，MUST NOT 沿用「仍为 3 / 逐字节不变 / 为空」旧断言（R11.5 / R12.4 / ADR 0002 §6-8 / C1）

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5）

- **WHEN** 以退化实现（AI 命令退回本地族 / 四组合装配退化为单参数 `opponent()` / `with_race` 兼容别名退化 / 复活新增字段漏接哈希快照）替换正式实现
- **THEN** 对应段位 / 基线断言在缺陷态失败、修复态通过，两条证据可复现（`CODEBUDDY.md §5` / C5 / C27）
