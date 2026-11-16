# human-faction Specification

## Purpose
为 P8.1 人族完整化定义行为契约：人族冻结定义、主动技能与升级、`Race` 分派、开局装配、AI 命令脚本与 CPU 端到端一局，全部为合成 / 原创编译期整数常量、零资产（ADR 0001），且不新增权威状态字段、守住 ADR 0002 确定性契约与 1.0 范围 S1 / S3。

## Requirements

### Requirement: 人族冻结定义完整集

`FrozenDefinitions` 的 `units` 类别 SHALL 在人族段 `100..200` 补齐六类新 `UnitDef`：远程单位（穿刺 / 长射程，R5.1 / R5.3）、反重甲第三战斗单位（R5.3）、人口建筑（R3.4）、英雄祭坛（R4.2）、研发建筑（R4.4）、1 名英雄（mana > 0，R6）。DefId MUST 全局唯一、不与 `101..104` / `201..204` / `9xx` 冲突；所有新增建筑 `speed_raw` MUST 为 0（R1.4 / R13.6）；全部字段 MUST 是编译期整数常量、零资产、零文件 IO（R2 / ADR 0001）。

#### Scenario: 六类新定义齐备且分段唯一

- **WHEN** 构造 `synthetic_definitions()` 并枚举 `units` 类别
- **THEN** 六类新 `UnitDef` 均在 `100..200` 段、DefId 全局唯一且与既有定义不冲突，远程单位射程大于近战、英雄 `mana > 0`、所有新增建筑 `speed_raw == 0`（覆盖 R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6）

#### Scenario: 定义值全为合成整数且零资产

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造人族定义
- **THEN** 构造成功、不做任何文件 IO、每个字段均为编译期整数常量，不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名或 rawcode（R2 / ADR 0001 / ADR 0004）

#### Scenario: 内容哈希随新增定义变化而非新增权威状态

- **WHEN** 对比补齐人族定义前后两份 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 哈希不同且差异经 `compute_defs_hash` 的 units / abilities / upgrades 段进入 `defs_hash` / `content_hash` 快照头部；MUST NOT 新增权威状态字段，`SNAPSHOT_VERSION`（当前 3）不变（ADR 0002 §6-8）

### Requirement: 人族主动技能与攻防升级走既有命令面

系统 SHALL 新增 1 个 `AbilityDef` 主动技能，MUST 只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` 与 `EffectDef` 组合表达、经既有 `Cast` 命令面施放（R6 / R7.6）；SHALL 新增 1–2 项 `UpgradeDef`（`damage_mod` / `hp_mod` / `research_frames`）并经既有 `Research` 路径接通（R4.4）。系统 MUST NOT 新增 `EffectKind` 变体、MUST NOT 新增 `GameplayCommandKind` / `CommandKind` 变体、MUST NOT 引入 JASS / wtg VM（N5）。

#### Scenario: 英雄施法经既有 Cast 结算

- **WHEN** 对局中英雄实体提交 `GameplayCommandKind::Cast` 施放新增主动技能
- **THEN** 技能效果按 `EffectDef` 组合确定性结算、`mana` / 冷却按冻结定义扣减，全程只经既有 `Cast` 命令面（R6 / R7.6）

#### Scenario: 攻防升级经既有 Research 结算

- **WHEN** 对局中提交 `GameplayCommandKind::Research` 研发新增攻 / 防升级并推进至 `research_frames`
- **THEN** 单位 `damage` / `hp` 按 `UpgradeDef` 的 `damage_mod` / `hp_mod` 确定性生效，全程只经既有 `Research` 路径（R4.4）

#### Scenario: 内核命令与效果类型集合不变

- **WHEN** 校验本 capability 的全部改动对 `crates/w3-core/` 的影响
- **THEN** `EffectKind` / `GameplayCommandKind` / `CommandKind` 变体集合不变、`git diff -- crates/w3-core/` 为空，且不存在任何 JASS / wtg VM 入口（N5 / ADR 0002）

### Requirement: Race 分派与开局装配不新增权威状态

`Race` SHALL 扩展到新类别的定义层 / 装配层分派（如 `ranged()` / `siege()` / `farm()` / `altar()` / `smithy()` / `hero()`），返回值 MUST 落在 `100..200` 人族段；`MeleeSetup` SHALL 扩展人族开局 `def_ids` 关联。系统 MUST NOT 新增 `Player.race` 之类运行时权威状态字段，`CommandKind::Spawn` 字节 MUST 逐字节不变、两族装配的 `Spawn` 字节一致（C2 / 主 `openspec/specs/melee-match/spec.md:127-129`）。

#### Scenario: 新类别分派落在人族段

- **WHEN** 分别以 `Race::Human` / `Race::Orc` 调用新增类别分派方法
- **THEN** 人族返回值全部落在 `100..200` 段、兽族落在 `200..300` 段，与既有 `peasant()` / `soldier()` / `town_hall()` / `barracks()` 一致且不冲突（R2 / R3.4 / R4.2 / R4.4 / R6）

#### Scenario: 两族 Spawn 字节逐字节一致

- **WHEN** 分别以两族参数构造开局并比对 `commands` 与 `gameplay_commands`
- **THEN** `CommandKind::Spawn` 字节逐字节相同、种族差异只存在于装配层 `def_ids` 关联，逐帧状态哈希与 `capture_snapshot()` 字节与未传种族参数的基线一致（C2 / ADR 0002）

#### Scenario: 不新增运行时权威状态

- **WHEN** 校验 `RtsMatch` / `Player` 的状态字段与 `SNAPSHOT_VERSION`
- **THEN** 无 `race` 或任何新增权威状态字段、`SNAPSHOT_VERSION` 仍为 3，命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（ADR 0002 / C1）

### Requirement: AI 命令脚本覆盖人族新玩法

`AiScript` SHALL 扩展为会训练新单位、施放英雄技能、研发升级的确定性外部命令脚本，MUST 沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`，按 `(execute_frame, player_id, sequence)` 升序且可重复构造（R12.7 / R4.3 / R4.4 / R6）。命令 MUST 为帧冻结外部命令，不与墙钟调用粒度绑定（ADR 0002）。

#### Scenario: 脚本含新单位训练 / 英雄施法 / 研发

- **WHEN** 以人族开局生成 `AiScript` 并枚举其 `GameplayCommand`
- **THEN** 含指向新增 `UnitDef` 的 `Train`、指向新增 `AbilityDef` 的 `Cast`、指向新增 `UpgradeDef` 的 `Research`，以及 `Gather` / `SetAi` / `Leave`，且全部 `player_id` 为 AI 玩家（R4.3 / R4.4 / R6 / R12.7）

#### Scenario: 帧冻结而非时钟驱动

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致（ADR 0002 / R12.7）

### Requirement: 人族 CPU 端到端一局与确定性闭环

系统 SHALL 支持 CPU 无渲染一局按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」跑完（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5），每阶段 MUST 可执行验证、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 与基线一致（ADR 0002）；胜负 MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态（R11.4 / R11.5）。

#### Scenario: 采集阶段黄金增长

- **WHEN** 对人族农民提交 `Gather` 并推进若干逻辑帧
- **THEN** 玩家黄金确定增长且逐帧哈希与基线一致（R3.1 / R1.5）

#### Scenario: 建造阶段生成人口 / 祭坛 / 研发建筑

- **WHEN** 提交 `Train { is_building: true }` 建造新增人口建筑 / 英雄祭坛 / 研发建筑并推进至队列完成
- **THEN** 生产队列可见后清空、生成新建筑实体，且这些建筑 `speed_raw == 0` 不响应 `Move`（R3.4 / R4.2 / R4.4 / R1.4 / R13.6）

#### Scenario: 训练阶段生成远程 / 反重甲单位

- **WHEN** 从人族生产建筑提交 `Train` 训练远程单位与反重甲单位并推进至完成
- **THEN** 生成可移动新实体、远程单位射程与反重甲单位伤害符合冻结定义，逐帧哈希与基线一致（R5.1 / R5.3 / R4.3）

#### Scenario: 英雄施法阶段结算主动技能

- **WHEN** 训练出英雄后对其提交 `Cast` 施放新增主动技能
- **THEN** 英雄 `mana` 扣减、效果按 `EffectDef` 组合确定性结算、逐帧哈希与基线一致（R6 / R7.6）

#### Scenario: 研发阶段升级生效

- **WHEN** 提交 `Research` 研发攻 / 防升级并推进至完成
- **THEN** 玩家单位 `damage` / `hp` 按 `UpgradeDef` 生效、逐帧哈希与基线一致（R4.4）

#### Scenario: 战斗与摧毁全部建筑结算

- **WHEN** 指挥人族战斗单位 `Attack` 敌方建筑直至摧毁全部建筑、触发 `Leave`
- **THEN** 既有 `resolve_victory` 只结算一次、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态（R5 / R11.1 / R11.4）

#### Scenario: 重开无残留

- **WHEN** 以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态（R11.5）

#### Scenario: 全链路逐帧哈希与基线一致

- **WHEN** 两实例注入同一完整一局脚本并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等，且与人族装配的逐帧哈希基线一致，`git diff -- crates/w3-core/` 为空（ADR 0002 / R11.4）

#### Scenario: 反向验证（改坏验红）

- **WHEN** 以退化实现（如漏接某类新定义的 `defs_hash` 敏感性、或把新增建筑 `speed_raw` 改为非 0）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过，两条证据均可复现（`CODEBUDDY.md` §5 / C5）
