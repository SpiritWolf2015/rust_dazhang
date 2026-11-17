# Spec Delta

## Purpose

为 P8.2 兽族完整化定义行为契约：兽族冻结定义、主动技能与攻防升级、`Race` 分派替换占位、开局装配、AI 命令脚本与 CPU 端到端一局，全部为合成 / 原创编译期整数常量、零资产（ADR 0001），数值差异是 `[取舍]` 合成原创、不是参考项目得来的经典事实（R2 / R5），且不新增权威状态字段、守住 ADR 0002 确定性契约与 1.0 范围 S1 / S3。对称于已归档 `human-faction` capability。

## ADDED Requirements

### Requirement: 兽族冻结定义完整集

`FrozenDefinitions` 的 `units` 类别 SHALL 在兽族段 `200..300` 补齐六类新 `UnitDef`（R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6）：远程 / 猎头、反重甲、人口地洞、祭坛、研发建筑、英雄（mana > 0）。DefId MUST 全局唯一、不与既有段冲突；新增建筑 `speed_raw` MUST 为 0（R1.4 / R13.6）；全字段 MUST 为编译期整数常量、零资产（R2 / ADR 0001）。数值差异 SHALL 只以合成常量相对大小表达、标 `[取舍]`，MUST NOT 冒充经典事实或新增 `armor_type` / `attack_type` 权威字段（C6）。细节见各 `#### Scenario:`。

#### Scenario: 六类新定义齐备且分段唯一

- **WHEN** 构造 `synthetic_definitions()` 并枚举 `units` 类别
- **THEN** 六类新 `UnitDef` 均在 `200..300` 段、DefId 全局唯一且与 `201..204` / `1xx` / `9xx` / `1..=6` 不冲突，远程单位射程大于近战、英雄 `mana > 0`、所有新增建筑 `speed_raw == 0`，且 `Race::{ranged,siege,farm,altar,smithy,hero,category_def_ids}` 对 Orc 不再回落 `ORC_SOLDIER` / `ORC_BARRACKS` 占位（覆盖 R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6）

#### Scenario: 定义值全为合成整数且零资产

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造兽族定义
- **THEN** 构造成功、不做任何文件 IO、每个字段均为编译期整数常量，不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名 / 暴雪 rawcode（`AOwk` / `AHwe` / `ASlo` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名（R2 / ADR 0001 / ADR 0004 / C6）

#### Scenario: 数值差异是合成原创取舍而非经典事实

- **WHEN** 对比兽族与对应人族六类定义的 `hp` / `speed_raw` / `damage` / `range_raw` / `supply` / `train_frames`
- **THEN** 兽族「更肉 / 更快 / 更低远程稳定度」仅体现为合成整数常量相对大小与 `[取舍]` 标注，规则依据为本项目 `WC3-MELEE-RULES.md` R2 / R5 / R5.1 / R5.3 的 `[已核]` 框架，MUST NOT 声称与某一经典 WC3 补丁数值一致、MUST NOT 冒充参考项目结论（`godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md:3` / 参考缺口 G5）

#### Scenario: 内容哈希随新增定义变化而非新增权威状态

- **WHEN** 对比补齐兽族定义前后两份 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 哈希不同且差异经 `compute_defs_hash` 的 units / abilities / upgrades 段进入 `defs_hash` / `content_hash` 快照头部；每个新增兽族定义漏接都会改变 `content_hash()`；MUST NOT 新增权威状态字段，`SNAPSHOT_VERSION`（当前 3）不变（ADR 0002 §6-8 / C2）

### Requirement: 兽族主动技能与攻防升级走既有命令面

系统 SHALL 新增 1 个 `AbilityDef` 主动技能，MUST 只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` 与 `EffectDef` 组合表达、经既有 `Cast` 命令面施放（R6 / R7.6）；SHALL 新增 1–2 项 `UpgradeDef`（`damage_mod` / `hp_mod` / `research_frames`）并经既有 `Research` 路径接通（R4.4）。系统 MUST NOT 新增 `EffectKind` 变体、MUST NOT 新增 `GameplayCommandKind` / `CommandKind` 变体、MUST NOT 引入 JASS / wtg VM（N5）。兽族 MUST NOT 据「存在对称 `orcUnitActives.json`」臆造单位技能表（参考项目无该文件，见 `design.md`「## 参考项目对照」兽族特有校正）。

#### Scenario: 英雄施法经既有 Cast 结算

- **WHEN** 对局中兽族英雄实体提交 `GameplayCommandKind::Cast` 施放新增主动技能
- **THEN** 技能效果按 `EffectDef` 组合确定性结算（伤害 / 眩晕 / 增益走既有 `EffectKind`，不借 `orcHeroActives.json` 的 `AOwk` 隐身 / 移速增益这类需新增权威状态的 P8.5 语义）、`mana` / 冷却按冻结定义扣减，全程只经既有 `Cast` 命令面（R6 / R7.6）

#### Scenario: 攻防升级经既有 Research 结算

- **WHEN** 对局中提交 `GameplayCommandKind::Research` 研发新增攻 / 防升级并推进至 `research_frames`
- **THEN** 单位 `damage` / `hp` 按 `UpgradeDef` 的 `damage_mod` / `hp_mod` 确定性生效，全程只经既有 `Research` 路径（R4.4）

#### Scenario: 内核命令与效果类型集合不变

- **WHEN** 校验本 capability 的全部改动对 `crates/w3-core/` 的影响
- **THEN** `EffectKind` / `GameplayCommandKind` / `CommandKind` 变体集合不变、`git diff -- crates/w3-core/` 为空，且不存在任何 JASS / wtg VM 入口（N5 / ADR 0002）

### Requirement: Race 分派替换占位与开局装配不新增权威状态

`Race` 的六个新类别分派（含 `category_def_ids`）SHALL 对 `Orc` 从 `ORC_SOLDIER` / `ORC_BARRACKS` 占位替换为真实 `2xx` 定义常量，返回值 MUST 落在 `200..300` 兽族段；`MeleeSetup` SHALL 扩展兽族开局 `def_ids` / `category_def_ids` 关联。系统 MUST NOT 新增 `Player.race` 之类运行时权威状态字段，`CommandKind::Spawn` 字节 MUST 逐字节不变、两族装配的 `Spawn` 字节一致（C2 / C3 / 主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」）。

#### Scenario: 新类别分派替换占位且落在兽族段

- **WHEN** 分别以 `Race::Human` / `Race::Orc` 调用新增类别分派方法与 `category_def_ids`
- **THEN** 人族返回值全部落在 `100..200` 段、兽族返回值全部落在 `200..300` 段且**不再等于** `ORC_SOLDIER` / `ORC_BARRACKS` 占位，与既有 `peasant()` / `soldier()` / `town_hall()` / `barracks()` 一致且不冲突（R2 / R3.4 / R4.2 / R4.4 / R6）

#### Scenario: 两族分段不冲突且 Spawn 字节逐字节一致

- **WHEN** 分别以两族参数构造开局并比对 `commands` 与 `gameplay_commands`
- **THEN** `CommandKind::Spawn` 字节逐字节相同、兽族新 DefId 不与人族 / 中立 / 基线段冲突，种族差异只存在于装配层 `def_ids` / `category_def_ids` 关联，逐帧状态哈希与 `capture_snapshot()` 字节与未传种族参数的基线一致（C2 / ADR 0002）

#### Scenario: 不新增运行时权威状态

- **WHEN** 校验 `RtsMatch` / `Player` 的状态字段与 `SNAPSHOT_VERSION`
- **THEN** 无 `race` 或任何新增权威状态字段、`SNAPSHOT_VERSION` 仍为 3，命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（ADR 0002 / C1 / C3）

### Requirement: AI 命令脚本覆盖兽族新玩法

`AiScript` SHALL 扩展为会训练兽族新单位、施放兽族英雄技能、研发兽族升级的确定性外部命令脚本，MUST 沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`，按 `(execute_frame, player_id, sequence)` 升序且可重复构造（R12.7 / R4.3 / R4.4 / R6）。命令 MUST 为帧冻结外部命令，不与墙钟调用粒度绑定（ADR 0002）。

#### Scenario: 脚本含兽族新单位训练 / 英雄施法 / 研发

- **WHEN** 以兽族开局生成 `AiScript` 并枚举其 `GameplayCommand`
- **THEN** 含指向新增 `UnitDef` 的 `Train`、指向新增 `AbilityDef` 的 `Cast`、指向新增 `UpgradeDef` 的 `Research`，以及 `Gather` / `SetAi` / `Leave`，且全部 `player_id` 为 AI 玩家（R4.3 / R4.4 / R6 / R12.7）

#### Scenario: 帧冻结而非时钟驱动

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致（ADR 0002 / R12.7）

### Requirement: 兽族 CPU 端到端一局与确定性闭环

系统 SHALL 支持 CPU 无渲染一局按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」跑完（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5），每阶段 MUST 可执行验证、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 与基线一致（ADR 0002）；胜负 MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态（R11.4 / R11.5）。含新增建筑 / 资源点 / 主城的选中集合 MUST NOT 出现在 `Move` 的 Scenario 生成集合（R1.4 / R13.6）。

#### Scenario: 采集阶段黄金增长

- **WHEN** 对兽族苦工提交 `Gather` 并推进若干逻辑帧
- **THEN** 玩家黄金确定增长且逐帧哈希与基线一致（R3.1 / R1.5）

#### Scenario: 建造阶段生成地洞 / 祭坛 / 战争磨坊且不响应 Move

- **WHEN** 提交 `Train { is_building: true }` 建造新增人口地洞 / 英雄祭坛 / 战争磨坊并推进至队列完成
- **THEN** 生产队列可见后清空、生成新建筑实体，且这些建筑 `speed_raw == 0` 派生为不可移动、运行时 `EntityView.speed == 0` 判不可移动，MUST NOT 出现在 `Move` 命令生成集合、位置不变（R2 / R3.4 / R4.2 / R4.4 / R1.4 / R13.6）

#### Scenario: 训练阶段生成远程 / 反重甲单位

- **WHEN** 从兽族生产建筑提交 `Train` 训练远程 / 猎头类单位与反重甲单位并推进至完成
- **THEN** 生成可移动新实体（`EntityView.speed > 0`）、远程单位射程与反重甲单位伤害符合冻结定义的 `[取舍]` 合成数值，逐帧哈希与基线一致（R5.1 / R5.3 / R4.3 / R1.4）

#### Scenario: 英雄施法阶段结算主动技能

- **WHEN** 训练出兽族英雄后对其提交 `Cast` 施放新增主动技能
- **THEN** 英雄 `mana` 扣减、效果按 `EffectDef` 组合确定性结算、逐帧哈希与基线一致（R6 / R7.6）

#### Scenario: 研发阶段升级生效

- **WHEN** 提交 `Research` 研发兽族攻 / 防升级并推进至完成
- **THEN** 玩家单位 `damage` / `hp` 按 `UpgradeDef` 生效、逐帧哈希与基线一致（R4.4）

#### Scenario: 战斗与摧毁全部建筑结算

- **WHEN** 指挥兽族战斗单位 `Attack` 敌方建筑直至摧毁全部建筑、触发 `Leave`
- **THEN** 既有 `resolve_victory` 只结算一次、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态，战斗结算只经 `w3-core` 既有 `Attack` / `Cast`（R5 / R11.1 / R11.4）

#### Scenario: 重开无残留

- **WHEN** 以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态（R11.5）

#### Scenario: 全链路逐帧哈希与基线一致

- **WHEN** 两实例注入同一完整一局脚本并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等，且与兽族装配的逐帧哈希基线一致，不只比最终 hash，`git diff -- crates/w3-core/` 为空（ADR 0002 / R11.4）

#### Scenario: 反向验证（改坏验红）

- **WHEN** 以退化实现（漏接某类新增兽族定义的 `defs_hash` 敏感性 / 把新增建筑 `speed_raw` 改为非 0 / 把英雄 `mana` 改为 0 / 让兽族 `category_def_ids` 仍回落占位）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过，两条证据均可复现（`CODEBUDDY.md` §5 / C5）
