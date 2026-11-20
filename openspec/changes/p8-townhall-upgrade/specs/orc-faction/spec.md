# Spec Delta

## MODIFIED Requirements

### Requirement: 兽族冻结定义完整集

`FrozenDefinitions` 的 `units` 类别 SHALL 在兽族段 `200..300` 补齐六类新 `UnitDef`（远程 / 猎头、反重甲、人口地洞、祭坛、研发建筑、英雄），DefId 全局唯一、新增建筑 `speed_raw == 0`、全字段为编译期整数常量、零资产（R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6 / ADR 0001）。数值差异 SHALL 只以合成常量相对大小表达、标 `[取舍]`、MUST NOT 冒充经典事实（R2 / R5 / C6）。P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A 改 `w3-core` 新增「主城等级 / tier」顶层权威状态 / 升级队列 / 费用登记，MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`；tier 需求链冻结常量按 Q3 推荐经 `compute_defs_hash` 进 `defs_hash` / `content_hash`；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「MUST NOT 新增权威状态字段 / `SNAPSHOT_VERSION`（当前 3 或 4）不变 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 六类新定义齐备且分段唯一

- **WHEN** 构造 `synthetic_definitions()` 并枚举 `units` 类别
- **THEN** 六类新 `UnitDef` 均在 `200..300` 段、DefId 全局唯一且与 `201..204` / `1xx` / `9xx` / `1..=6` 不冲突，远程单位射程大于近战、英雄 `mana > 0`、所有新增建筑 `speed_raw == 0`，且 `Race` 对 Orc 不再回落占位（R2 / R3.4 / R4.2 / R4.4 / R5.1 / R5.3 / R6）

#### Scenario: 定义值全为合成整数且零资产

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造兽族定义
- **THEN** 构造成功、不做任何文件 IO、每个字段均为编译期整数常量，不含 MPQ 派生 / 真实导出 JSON / 参考项目单位名 / 暴雪 rawcode / godot 原创三族单位名（R2 / ADR 0001 / ADR 0004 / C6）

#### Scenario: 数值差异是合成原创取舍而非经典事实

- **WHEN** 对比兽族与对应人族六类定义的 `hp` / `speed_raw` / `damage` / `range_raw` / `supply` / `train_frames`
- **THEN** 兽族「更肉 / 更快 / 更低远程稳定度」仅体现为合成整数常量相对大小与 `[取舍]` 标注，MUST NOT 声称与某一经典 WC3 补丁数值一致、MUST NOT 冒充参考项目结论（R2 / R5 / `CLASSIC_RULES_BASELINE.md:3` / §14）

#### Scenario: 内容哈希随新增定义变化而非新增权威状态

- **WHEN** 对比补齐兽族定义前后两份 `FrozenDefinitions` 的 `content_hash()`，并复核 P8.5b 主城升级新增权威状态
- **THEN** 哈希不同且差异经 `compute_defs_hash` 的 units / abilities / upgrades 段 + P8.5b tier 需求链冻结常量段进入 `defs_hash` / `content_hash` 快照头部、每个新增兽族定义漏接都会改变 `content_hash()`（R2 / ADR 0002）；但 P8.5b 按用户 2026-11-20 拍板方向 A MUST 新增「主城等级 / tier」顶层权威状态并同步进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5、`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体、`git diff -- crates/w3-core/` 非空属预期、golden hash 基线按新实现如实更新，MUST NOT 沿用「MUST NOT 新增权威状态字段 / `SNAPSHOT_VERSION`（当前 3 或 4）不变 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 兽族主动技能与攻防升级走既有命令面

系统 SHALL 新增 1 个 `AbilityDef` 主动技能（只用既有 `EffectKind` / `EffectDef` 组合、经 `Cast` 施放）与 1–2 项 `UpgradeDef`（经 `Research` 接通），MUST NOT 新增 `EffectKind` 变体、MUST NOT 引入 JASS / wtg VM（N5 / R6 / R7.6 / R4.4）；兽族 MUST NOT 据臆造的对称 `orcUnitActives.json` 臆造技能表。P8.1 / P8.2 正向语义不变。但 P8.5b 主城升级 change 按用户拍板改 `w3-core`：`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 命令变体 + 升级事件变体（连带 `hash_gameplay_command` 追加码 13 / 校验 / 穷尽分发同步、ADR 0005 字段编号只追加 / `reserved 14 to 32` / `PROTOCOL_VERSION` bump 与 `Reject` 旧版本、`net-owner` 按 `CODEBUDDY.md §9` 串行协作）；新增 tier 顶层权威状态 / 升级队列 / 费用登记 MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「`EffectKind` / `GameplayCommandKind` / `CommandKind` 变体集合不变 / `git diff` 为空 / 仍为 3 或 4 / 逐字节不变 / MUST NOT 新增权威状态」旧断言（R12.4 / ADR 0002 §6-8 / ADR 0005 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 英雄施法经既有 Cast 结算

- **WHEN** 对局中兽族英雄实体提交 `GameplayCommandKind::Cast` 施放新增主动技能
- **THEN** 技能效果按 `EffectDef` 组合确定性结算（伤害 / 眩晕 / 增益走既有 `EffectKind`，不借 `orcHeroActives.json` 的 `AOwk` 隐身 / 移速增益这类需新增权威状态的 P8.5 语义）、`mana` / 冷却按冻结定义扣减，全程只经既有 `Cast` 命令面（R6 / R7.6）；本 change 承接的这类「需新增权威状态」P8.5 语义正是主城升级 tier（R4.1）

#### Scenario: 攻防升级经既有 Research 结算

- **WHEN** 对局中提交 `GameplayCommandKind::Research` 研发新增攻 / 防升级并推进至 `research_frames`
- **THEN** 单位 `damage` / `hp` 按 `UpgradeDef` 生效、全程只经既有 `Research` 路径；要求 T2 / T3 的科技在主城未达 tier 时 MUST 被冻结需求链前置判据显式拒绝（R4.4 / R4.1 / C4）

#### Scenario: 内核命令与效果类型集合不变

- **WHEN** 校验本 capability 的全部改动对 `crates/w3-core/` 的影响，以及 P8.5b 主城升级 change 的全部改动
- **THEN** P8.2 侧 `EffectKind` 变体集合不变、不存在任何 JASS / wtg VM 入口（N5 / ADR 0002 / R6）；但 P8.5b 按 Q2 推荐新增独立 `UpgradeTownHall` `GameplayCommandKind` 变体 + 升级事件变体，`hash_gameplay_command` 编码 / 校验 / 穷尽分发同步覆盖、protobuf 字段编号只追加且 `PROTOCOL_VERSION` bump 并 `Reject` 旧版本，新增 tier 顶层权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5，`git diff -- crates/w3-core/` 非空属预期、变体集合不再「不变」，MUST NOT 沿用「变体集合不变 / `git diff` 为空 / 仍为 3 或 4 / 逐字节不变」旧断言（R12.4 / ADR 0002 §6-8 / ADR 0005 / `CODEBUDDY.md §5`）

### Requirement: Race 分派替换占位与开局装配不新增权威状态

`Race` 的六个新类别分派（含 `category_def_ids`）SHALL 对 `Orc` 从占位替换为真实 `2xx` 定义常量，返回值 MUST 落在 `200..300` 兽族段；`MeleeSetup` SHALL 扩展兽族开局关联。系统 MUST NOT 新增 `Player.race` 之类**选族**运行时权威状态字段（该约束仅针对选族装配、P8.2 正向语义不变），`CommandKind::Spawn` 字节 MUST 逐字节不变（C2 / C3）。但 P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A MUST 新增「主城等级 / tier」**顶层权威状态**（与选族 `race` 字段无关）并同步进 FNV-1a 状态哈希、快照与 `first_difference()`；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「无 `race` 或任何新增权威状态字段 / `SNAPSHOT_VERSION` 仍为 3 或 4 / 逐字节不变 / 变体集合不变 / `git diff` 为空」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 新类别分派替换占位且落在兽族段

- **WHEN** 分别以 `Race::Human` / `Race::Orc` 调用新增类别分派方法与 `category_def_ids`
- **THEN** 人族返回值全部落在 `100..200` 段、兽族返回值全部落在 `200..300` 段且不再等于 `ORC_SOLDIER` / `ORC_BARRACKS` 占位（R2 / R3.4 / R4.2 / R4.4 / R6）

#### Scenario: 两族分段不冲突且 Spawn 字节逐字节一致

- **WHEN** 分别以两族参数构造开局并比对 `commands` 与 `gameplay_commands`
- **THEN** `CommandKind::Spawn` 字节逐字节相同、兽族新 DefId 不与人族 / 中立 / 基线段冲突，种族差异只存在于装配层关联，逐帧状态哈希与 `capture_snapshot()` 字节在同一 P8.5b 新实现基线内一致、MUST NOT 断言与 P8.5b 前旧基线逐字节相等（R2 / R12.4 / C2 / ADR 0002）

#### Scenario: 不新增运行时权威状态

- **WHEN** 校验 `RtsMatch` / `Player` 的状态字段、`SNAPSHOT_VERSION`，以及 P8.5b 主城升级新增权威状态
- **THEN** 选族装配不引入 `race` 字段、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（R2 / R12.7 / R13.6 / ADR 0002 / C1 / C3）；但 P8.5b 按用户 2026-11-20 拍板方向 A MUST 新增「主城等级 / tier」顶层权威状态字段（独立于选族 `race`）并同步进 FNV-1a 状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5，`git diff -- crates/w3-core/` 非空属预期，MUST NOT 沿用「无 `race` 或任何新增权威状态字段 / `SNAPSHOT_VERSION` 仍为 3 或 4 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 兽族 CPU 端到端一局与确定性闭环

系统 SHALL 支持 CPU 无渲染一局按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」跑完（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5），每阶段 MUST 可执行验证、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 与 P8.5b 新实现基线一致；含新增建筑 / 资源点 / 主城的选中集合 MUST NOT 出现在 `Move` 的 Scenario 生成集合（R1.4 / R13.6）。P8.5b 主城升级 change 按用户拍板改 `w3-core` 后，全链路基线 MUST 按新实现重取、golden hash 基线按新实现如实更新、不沿用 P8.5a v4 值，新增 tier 顶层权威状态 / 升级队列 / 费用登记 MUST 同步进状态哈希 / 快照 / `first_difference()`；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3 或 4 / 逐字节不变 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 采集阶段黄金增长

- **WHEN** 对兽族苦工提交 `Gather` 并推进若干逻辑帧
- **THEN** 玩家黄金确定增长且逐帧哈希与 P8.5b 基线一致（R3.1 / R1.5）

#### Scenario: 建造阶段生成地洞 / 祭坛 / 战争磨坊且不响应 Move

- **WHEN** 提交 `Train { is_building: true }` 建造新增人口地洞 / 英雄祭坛 / 战争磨坊并推进至队列完成
- **THEN** 生产队列可见后清空、生成新建筑实体，且这些建筑 `speed_raw == 0` 派生为不可移动、运行时 `EntityView.speed == 0` 判不可移动，MUST NOT 出现在 `Move` 命令生成集合、位置不变（R2 / R3.4 / R4.2 / R4.4 / R1.4 / R13.6）

#### Scenario: 训练阶段生成远程 / 反重甲单位

- **WHEN** 从兽族生产建筑提交 `Train` 训练远程 / 猎头类单位与反重甲单位并推进至完成
- **THEN** 生成可移动新实体（`EntityView.speed > 0`）、远程单位射程与反重甲单位伤害符合冻结定义的 `[取舍]` 合成数值；要求 T2 / T3 的二线单位在主城未达 tier 时 MUST 被显式拒绝（R5.1 / R5.3 / R4.3 / R4.1 / R1.4）

#### Scenario: 英雄施法阶段结算主动技能

- **WHEN** 训练出兽族英雄后对其提交 `Cast` 施放新增主动技能
- **THEN** 英雄 `mana` 扣减、效果按 `EffectDef` 组合确定性结算、逐帧哈希与 P8.5b 基线一致（R6 / R7.6）

#### Scenario: 研发阶段升级生效

- **WHEN** 提交 `Research` 研发兽族攻 / 防升级并推进至完成
- **THEN** 玩家单位 `damage` / `hp` 按 `UpgradeDef` 生效、逐帧哈希与 P8.5b 基线一致；要求 T2 / T3 的科技按冻结 tier 需求链前置校验（R4.4 / R4.1）

#### Scenario: 战斗与摧毁全部建筑结算

- **WHEN** 指挥兽族战斗单位 `Attack` 敌方建筑直至摧毁全部建筑、触发 `Leave`
- **THEN** 既有 `resolve_victory` 只结算一次、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态，战斗结算只经 `w3-core` 既有 `Attack` / `Cast`（R5 / R11.1 / R11.4）

#### Scenario: 重开无残留

- **WHEN** 以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态与 tier / 升级队列登记（R11.5）

#### Scenario: 全链路逐帧哈希与基线一致

- **WHEN** 两实例注入同一完整一局脚本（含主城升级 / 二线单位解锁）并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等，且与 P8.5b 兽族装配的逐帧哈希基线一致、不只比最终 hash；`git diff -- crates/w3-core/` 非空属 P8.5b 预期、golden hash 基线按新实现如实更新、`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5，`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体，MUST NOT 沿用「为空 / 仍为 3 或 4 / 逐字节不变 / 变体集合不变」旧断言（ADR 0002 / R11.4 / R12.4 / `CODEBUDDY.md §5`）

#### Scenario: 反向验证（改坏验红）

- **WHEN** 以退化实现（漏接某类新增兽族定义的 `defs_hash` 敏感性 / 把新增建筑 `speed_raw` 改为非 0 / 把英雄 `mana` 改为 0 / 让兽族 `category_def_ids` 仍回落占位 / tier 权威状态漏接哈希快照 / 未升级仍接受未解锁 `Train`）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过，两条证据均可复现（`CODEBUDDY.md §5` / Q8 / C23）
