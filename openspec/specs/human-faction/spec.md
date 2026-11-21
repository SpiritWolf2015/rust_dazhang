# human-faction Specification

## Purpose
为 P8.1 人族完整化定义行为契约：人族冻结定义、主动技能与升级、`Race` 分派、开局装配、AI 命令脚本与 CPU 端到端一局，全部为合成 / 原创编译期整数常量、零资产（ADR 0001），且不新增权威状态字段、守住 ADR 0002 确定性契约与 1.0 范围 S1 / S3。

## Requirements

### Requirement: 人族冻结定义完整集

`FrozenDefinitions` 的 `units` 类别 SHALL 在人族段 `100..200` 补齐六类新 `UnitDef`（远程 / 反重甲 / 人口建筑 / 祭坛 / 研发建筑 / 英雄），DefId 全局唯一、新增建筑 `speed_raw == 0`、全为编译期整数常量零文件 IO（R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6 / ADR 0001）。P8.5a 复活 / P8.5b 主城升级 / P8.5c 野怪营地与掉落 change 均按用户拍板改 `w3-core`：新增死亡登记 / 复活队列 / 费用 / tier / 野怪 / 地面物品 / 物品实例 / 随机码消费序列等权威状态 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`。本 change P8.5d 物品 / 商店 / 回城卷轴按推荐方向 A（待规格门 Q1 拍板）继续按 ADR 0002 全流程改 `w3-core`：新增英雄六格背包与物品实例（owner / slot / 充能 / 冷却 / `EffectDef`）、拾取 / 购买 / 使用 / 丢弃 / 售卖事实、商店库存与刷新、回城传送（施法帧 / 目标 / 中断）、确定性随机码消费序列等权威状态（R6.2 / R7.5 / R7.6 / R9.3）全部同步进哈希 / 快照 / `first_difference()`；商店货架 / 价格 / 库存 / 回购价 / 回城施法帧等冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash`，全为合成原创编译期整数 `[待核]`（ADR 0001 / §14）；`SNAPSHOT_VERSION` / `SIM_VERSION` 按 Q4 推荐（待规格门拍板）bump 6→7（连同快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）；按 Q5 推荐（条件项、待规格门拍板）新增拾取 / 购买 / 使用 / 售卖 / 回城命令与事件变体、proto 只追加 tag 37 起、`PROTOCOL_VERSION` 6→7；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R7.5 / R9.3 / R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 六类新定义齐备且分段唯一

- **WHEN** 构造 `synthetic_definitions()` 并枚举 `units` 类别
- **THEN** 六类新 `UnitDef` 均在 `100..200` 段、DefId 全局唯一且与既有定义不冲突，远程单位射程大于近战、英雄 `mana > 0`、所有新增建筑 `speed_raw == 0`（覆盖 R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6）

#### Scenario: 定义值全为合成整数且零资产

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造人族定义与 P8.5d 商店 / 道具冻结常量
- **THEN** 构造成功、不做任何文件 IO、每个字段均为编译期整数常量 `[待核]`，不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名或 rawcode / `ngme` / `nmrk` / 半价 / `goldCost` / `stockMax`（R2 / ADR 0001 / ADR 0004 / 规则 `:382` / `:373`）

#### Scenario: 内容哈希随新增定义变化而非新增权威状态

- **WHEN** 对比补齐人族定义前后两份 `FrozenDefinitions` 的 `content_hash()`，并复核 P8.5a 复活 / P8.5b 主城升级 / P8.5c 野怪营地 / P8.5d 物品商店回城新增权威状态
- **THEN** 哈希不同且差异经 `compute_defs_hash` 的 units / abilities / upgrades / items 段 + P8.5b tier 需求链 / P8.5c 掉落表 / P8.5d 商店货架与道具冻结常量段进入 `defs_hash` / `content_hash` 快照头部（R2 / R8.6 / R7.3 / ADR 0002）；但 P8.5d 按推荐方向 A MUST 新增六格背包 / 实例 owner 与 slot / 商店库存 / 回城传送 / 消费序列等权威状态并同步进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7、按 Q5 推荐新增命令 / 事件变体、`git diff -- crates/w3-core/` 非空属预期、golden hash 基线按新实现如实更新，MUST NOT 沿用「MUST NOT 新增权威状态字段 / `SNAPSHOT_VERSION`（当前 3、4、5 或 6）不变 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / R7.5 / R9.3 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 人族主动技能与攻防升级走既有命令面

系统 SHALL 新增 1 个 `AbilityDef` 主动技能（只用既有 `EffectKind` / `EffectDef` 组合、经 `Cast` 施放）与 1–2 项 `UpgradeDef`（经 `Research` 接通），MUST NOT 新增 `EffectKind` 变体、MUST NOT 引入 JASS / wtg VM（N5 / R6 / R7.6 / R4.4）；P8.1 正向语义不变。但 P8.5a 复活 / P8.5b 主城升级 / P8.5c 野怪营地 change 按用户拍板改 `w3-core`：新增 `ReviveHero` / `CancelRevive` / `UpgradeTownHall` / `SeedCreepCamp` 命令与事件变体（连带 `hash_gameplay_command` / 校验 / 穷尽分发同步、ADR 0005 字段编号只追加 / `reserved 14 to 32` / `PROTOCOL_VERSION` bump 与 `Reject` 旧版本、`net-owner` 按 `CODEBUDDY.md §9` 串行协作）；新增 tier / 野怪 / 掉落权威状态 MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`。本 change P8.5d 按推荐方向 A 继续改 `w3-core`：按 Q5 推荐（条件项、待规格门拍板）新增拾取 / 购买 / 使用 / 售卖 / 回城命令与事件变体、proto 只追加 tag 37 起、`PROTOCOL_VERSION` 6→7 / `Reject` 旧版本，六格背包 / 商店库存 / 回城传送 / 消费序列等权威状态 MUST 同步进哈希 / 快照 / `first_difference()`；道具效果 MUST 走既有 `EffectDef` 组合、不执行 JASS（R7.6）；`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放明确拒绝）；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增 `GameplayCommandKind` / `CommandKind` 变体」旧断言（R7.5 / R9.3 / R12.4 / ADR 0002 §6-8 / ADR 0005 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 英雄施法经既有 Cast 结算

- **WHEN** 对局中英雄实体提交 `GameplayCommandKind::Cast` 施放新增主动技能
- **THEN** 技能效果按 `EffectDef` 组合确定性结算、`mana` / 冷却按冻结定义扣减，全程只经既有 `Cast` 命令面（R6 / R7.6）；道具使用同样只经既有 `EffectDef` 组合、不引入 JASS VM（R7.6 / N5）

#### Scenario: 攻防升级经既有 Research 结算

- **WHEN** 对局中提交 `GameplayCommandKind::Research` 研发新增攻 / 防升级并推进至 `research_frames`
- **THEN** 单位 `damage` / `hp` 按 `UpgradeDef` 生效、全程只经既有 `Research` 路径；要求 T2 / T3 的科技在主城未达 tier 时 MUST 被前置判据显式拒绝（R4.4 / R4.1 / C4）

#### Scenario: 内核命令与效果类型集合不变

- **WHEN** 校验本 capability 的 P8.1 主动技能 / 攻防升级改动，以及 P8.5d 物品 / 商店 / 回城卷轴 change 对 `crates/w3-core/` 的全部改动
- **THEN** P8.1 侧 `EffectKind` 变体集合不变、主动技能 / 升级仍只经既有 `Cast` / `Research` 命令面、不存在任何 JASS / wtg VM 入口（N5 / R6 / R7.6 / R4.4 / ADR 0002）；但 P8.5d 按 Q5 推荐新增拾取 / 购买 / 使用 / 售卖 / 回城 `GameplayCommandKind` 变体 + 事件变体，`hash_gameplay_command` 编码 / 校验 / 穷尽分发同步覆盖、protobuf 字段编号只追加（tag 37 起）且 `PROTOCOL_VERSION` bump 并 `Reject` 旧版本，`git diff -- crates/w3-core/` 非空属预期、变体集合不再「不变」，新增六格背包 / 商店库存 / 回城传送 / 消费序列等权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7，MUST NOT 沿用「变体集合不变 / `git diff -- crates/w3-core/` 为空 / 仍为 3、4、5 或 6 / 逐字节不变」旧断言（R12.4 / R7.5 / R9.3 / ADR 0002 §6-8 / ADR 0005 / `CODEBUDDY.md §5`）

### Requirement: Race 分派与开局装配不新增权威状态

`Race` SHALL 扩展到新类别的定义层 / 装配层分派，返回值 MUST 落在 `100..200` 人族段；`MeleeSetup` SHALL 扩展人族开局 `def_ids` 关联。系统 MUST NOT 新增 `Player.race` 之类**选族**运行时权威状态字段（该约束仅针对选族装配、P8.1 正向语义不变），`CommandKind::Spawn` 字节 MUST 逐字节不变（C2 / 主 `openspec/specs/melee-match/spec.md:127-129`）。但 P8.5b 主城升级 / P8.5c 野怪营地 change 按用户拍板方向 A MUST 新增 tier / 野怪 / 掉落**顶层权威状态**（与选族 `race` 字段无关）并同步进 FNV-1a 状态哈希、快照与 `first_difference()`；本 change P8.5d 物品 / 商店 / 回城卷轴按推荐方向 A 继续 MUST 新增六格背包 / 物品实例 owner 与 slot / 商店库存 / 回城传送 / 消费序列等权威状态（与选族 `race`、tier、野怪均无关）并同步进哈希 / 快照 / `first_difference()`（R6.2 / R7.5 / R9.3）；`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）；按 Q5 推荐新增拾取 / 购买 / 使用 / 售卖 / 回城命令与事件变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R8.1 / R8.6 / R7.5 / R9.3 / R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 新类别分派落在人族段

- **WHEN** 分别以 `Race::Human` / `Race::Orc` 调用新增类别分派方法
- **THEN** 人族返回值全部落在 `100..200` 段、兽族落在 `200..300` 段，与既有 `peasant()` / `soldier()` / `town_hall()` / `barracks()` 一致且不冲突（R2 / R3.4 / R4.2 / R4.4 / R6）

#### Scenario: 两族 Spawn 字节逐字节一致

- **WHEN** 分别以两族参数构造开局并比对 `commands` 与 `gameplay_commands`
- **THEN** `CommandKind::Spawn` 字节逐字节相同、种族差异只存在于装配层 `def_ids` 关联，逐帧状态哈希与 `capture_snapshot()` 字节在同一 P8.5d 新实现基线内一致、MUST NOT 断言与 P8.5d 前旧基线逐字节相等（R2 / R12.4 / R7.5 / R9.3 / C2 / ADR 0002）

#### Scenario: 不新增运行时权威状态

- **WHEN** 校验 `RtsMatch` / `Player` 的选族状态字段、`SNAPSHOT_VERSION`，以及 P8.5d 物品 / 商店 / 回城新增权威状态
- **THEN** 选族装配不引入 `race` 字段、两族 `CommandKind::Spawn` 字节逐字节一致，命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（R2 / R12.7 / R13.6 / ADR 0002 / C1 / C2）；但 P8.5d 按推荐方向 A MUST 新增六格背包 / 实例 owner 与 slot / 商店库存 / 回城传送 / 确定性随机码消费序列等权威状态字段（独立于选族 `race`）并同步进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7，`git diff -- crates/w3-core/` 非空属预期，MUST NOT 沿用「无 `race` 或任何新增权威状态字段 / `SNAPSHOT_VERSION` 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变」旧断言（R8.1 / R8.6 / R7.5 / R9.3 / R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: AI 命令脚本覆盖人族新玩法

`AiScript` SHALL 扩展为会训练新单位、施放英雄技能、研发升级的确定性外部命令脚本，MUST 沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`，按 `(execute_frame, player_id, sequence)` 升序且可重复构造（R12.7 / R4.3 / R4.4 / R6）。命令 MUST 为帧冻结外部命令，不与墙钟调用粒度绑定（ADR 0002）。

#### Scenario: 脚本含新单位训练 / 英雄施法 / 研发

- **WHEN** 以人族开局生成 `AiScript` 并枚举其 `GameplayCommand`
- **THEN** 含指向新增 `UnitDef` 的 `Train`、指向新增 `AbilityDef` 的 `Cast`、指向新增 `UpgradeDef` 的 `Research`，以及 `Gather` / `SetAi` / `Leave`，且全部 `player_id` 为 AI 玩家（R4.3 / R4.4 / R6 / R12.7）

#### Scenario: 帧冻结而非时钟驱动

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致（ADR 0002 / R12.7）

### Requirement: 人族 CPU 端到端一局与确定性闭环

系统 SHALL 支持 CPU 无渲染一局按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」跑完（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5），每阶段 MUST 可执行验证、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 与 P8.5c 新实现基线一致；胜负 MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态。本 change P8.5d 物品 / 商店 / 回城卷轴按推荐方向 A（待规格门 Q1 拍板）改 `w3-core` 后，全链路基线 MUST 按新实现重取、golden hash 基线按新实现如实更新、不沿用 P8.5c v6 值，新增六格背包 / 实例 owner 与 slot / 充能冷却 / 拾取购买使用丢弃售卖事实 / 商店库存与刷新 / 回城传送 / 消费序列等权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`（R6.2 / R7.5 / R9.3 / R12.4）；`SNAPSHOT_VERSION` / `SIM_VERSION` 按 Q4 推荐 bump 6→7（连同快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）；按 Q5 推荐新增拾取 / 购买 / 使用 / 售卖 / 回城命令与事件变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变」旧断言（R7.5 / R9.3 / R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 采集阶段黄金增长

- **WHEN** 对人族农民提交 `Gather` 并推进若干逻辑帧
- **THEN** 玩家黄金确定增长且逐帧哈希与 P8.5d 基线一致（R3.1 / R1.5）

#### Scenario: 建造阶段生成人口 / 祭坛 / 研发建筑

- **WHEN** 提交 `Train { is_building: true }` 建造新增人口建筑 / 英雄祭坛 / 研发建筑并推进至队列完成
- **THEN** 生产队列可见后清空、生成新建筑实体，且这些建筑 `speed_raw == 0` 不响应 `Move`（R3.4 / R4.2 / R4.4 / R1.4 / R13.6）

#### Scenario: 训练阶段生成远程 / 反重甲单位

- **WHEN** 从人族生产建筑提交 `Train` 训练远程单位与反重甲单位并推进至完成
- **THEN** 生成可移动新实体、远程单位射程与反重甲单位伤害符合冻结定义；要求 T2 / T3 的二线单位在主城未达 tier 时 MUST 被显式拒绝（R5.1 / R5.3 / R4.3 / R4.1）

#### Scenario: 英雄施法阶段结算主动技能

- **WHEN** 训练出英雄后对其提交 `Cast` 施放新增主动技能
- **THEN** 英雄 `mana` 扣减、效果按 `EffectDef` 组合确定性结算、逐帧哈希与 P8.5d 基线一致（R6 / R7.6）

#### Scenario: 研发阶段升级生效

- **WHEN** 提交 `Research` 研发攻 / 防升级并推进至完成
- **THEN** 玩家单位 `damage` / `hp` 按 `UpgradeDef` 生效、逐帧哈希与 P8.5d 基线一致；要求 T2 / T3 的科技按冻结 tier 需求链前置校验（R4.4 / R4.1）

#### Scenario: 战斗与摧毁全部建筑结算

- **WHEN** 指挥人族战斗单位 `Attack` 敌方建筑直至摧毁全部建筑、触发 `Leave`
- **THEN** 既有 `resolve_victory` 只结算一次、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态（R5 / R11.1 / R11.4）；英雄阵亡不判负、可经祭坛复活且六格背包按实例身份恢复（R11.2 / R6.3 / R7.5）

#### Scenario: 重开无残留

- **WHEN** 以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态与复活登记 / tier / 升级队列 / 野怪 / 营地 / 掉落 / 随机码消费序列 / 六格背包 / 商店库存 / 回城传送登记（R11.5 / R8.6 / R7.5 / R9.3）

#### Scenario: 全链路逐帧哈希与基线一致

- **WHEN** 两实例注入同一完整一局脚本（含英雄阵亡 / 祭坛复活 / 主城升级 / 二线单位解锁 / 清野 / 掉落 / 拾取 / 购买 / 使用 / 回城）并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等，且与 P8.5d 新实现的人族装配逐帧哈希基线一致；`git diff -- crates/w3-core/` 非空属 P8.5d 预期、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7、按 Q5 推荐新增命令 / 事件变体，MUST NOT 沿用「为空 / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变」旧断言（R7.5 / R9.3 / R12.4 / ADR 0002 / R11.4 / `CODEBUDDY.md §5`）

#### Scenario: 反向验证（改坏验红）

- **WHEN** 以退化实现（漏接某类新定义的 `defs_hash` 敏感性 / 把新增建筑 `speed_raw` 改为非 0 / 复活 `spawn` 新实体 / tier 权威状态漏接哈希快照 / 未升级仍接受未解锁 `Train` / 升级费用不扣 gold / 野怪或掉落或随机码消费序列漏接哈希快照 / 六格背包或实例充能或商店库存或回城传送漏接哈希快照 / 同名道具共用充能 / 购买不扣 gold / 拾取购买不经事务命令面 / 掉落用墙钟或 `HashMap` 迭代顺序）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过，两条证据均可复现（`CODEBUDDY.md §5` / Q9 / R22）
