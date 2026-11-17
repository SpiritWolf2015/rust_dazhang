# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实，锚一律「路径 + 函数名 / 符号名」，不写精确 `:NN` 行号——本轮 `w3-client` 代码会被 owner 改动，P8.1 有行号漂移返工教训）：`crates/w3-client/src/melee/defs.rs` 定义 `SOLDIER/PEASANT/TOWN_HALL/BARRACKS/GOLD_MINE/TREE`（`1..=6`）、`CATEGORY_*` 六槽位常量、`HERO_ABILITY 11` / `UPGRADE 20` / `UPGRADE_WEAPONS 21` / `UPGRADE_ARMOR 22` / `ITEM 30`，人族段 `HUMAN_PEASANT 101..HUMAN_HERO 110`、兽族段 `ORC_PEASANT 201..ORC_BARRACKS 204`、中立 `NEUTRAL_GOLD_MINE/NEUTRAL_TREE 9xx`；`defs.rs::Race` 的 `peasant()/soldier()/town_hall()/barracks()/gold_mine()/tree()` 与 `ranged()/siege()/farm()/altar()/smithy()/hero()/category_def_ids` 中，**六个新类别对 `Orc` 当前回落到 `ORC_SOLDIER` / `ORC_BARRACKS` 占位**（注释明确「P8.2 完整化时替换」）；`defs.rs::unit` 固定 `mana: 0`、`defs.rs::unit_with_mana` 写英雄 `mana > 0`；`defs.rs::synthetic_definitions` 现构造 22 `UnitDef` / 2 `AbilityDef` / 3 `UpgradeDef` / 1 `ItemDef`；`defs.rs::defs_hash_is_fixed_and_content_sensitive` / `defs.rs::rebuild_definitions` / `defs.rs::degenerate_hash_ignoring_new` / 成对 `defs.rs::reverse_validation_defs_hash_*` 是反向验证先例。`crates/w3-client/src/melee/setup.rs` 的 `MeleeSetup::{new,with_race}`、`Builder::spawn`（`speed.max(1)` 校验）、`def_ids: BTreeMap<u64,DefId>`、`category_def_ids`、`EntityKind` 6 类、`MeleeSetup::movability` / `MeleeSetup::category_def_ids` 与 `race_does_not_change_authoritative_spawn_commands` 先例。`crates/w3-client/src/melee/ai.rs::AiScript::new` 预生成外部冻结命令（既有采集 / 训练 / 骚扰 + P8.1 训练新单位 / 施放英雄技能 / 研发升级 / `SetAi{enabled:false}` / `Leave`），`unit_def` 经 `setup.category_def_ids()` 查表。`crates/w3-client/src/melee/driver.rs::MeleeDriver` / `Leave` 结算、`crates/w3-client/tests/melee_loop.rs` CPU 端到端一局 + 两实例逐帧哈希基线、`tests/movability.rs` R1.4 分层判据 + 反向验证先例。

内核只读事实（锚用符号名）：`crates/w3-core/src/command.rs::CommandKind::Spawn`（不含 `DefId`）、`crates/w3-core/src/gameplay.rs` 的 `EffectKind::{Damage,Heal,BuffStat,Stun}` / `EffectDef` / `UnitDef` / `AbilityDef` / `UpgradeDef` / `GameplayCommandKind`、`compute_defs_hash`（覆盖四类定义）、`FrozenDefinitions::content_hash`（进快照头部）、`crates/w3-core/src/snapshot.rs::SNAPSHOT_VERSION`（= 3）、`crates/w3-core/src/sim_match.rs::EntityView.speed`（只读投影 `UnitState.speed`，已归档 `runtime-entity-movability`）。主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」、主 `openspec/specs/unit-orders/spec.md`「群体移动只作用于可移动单位」/「群体移动过滤覆盖运行时动态实体」/「人族新增实体沿用可移动性过滤并覆盖端到端一局」/「键盘 M / A 路径复用 R1.4 分层可移动性过滤」已把上述边界写成 requirement。约束：ADR 0001 / 0002 / 0004、`CODEBUDDY.md §5 / §8 / §9`、`docs/references/WC3-MELEE-RULES.md` R1–R13（尤其 R1.4 / R13.6 分层判据与 §14 活文档流程）。

## Goals / Non-Goals

**Goals:** 在 `crates/w3-client/src/melee/` 定义层与装配层把兽族六类新 `UnitDef`、1 主动技能、1–2 项升级、`Race` 六类分派**从占位替换为真实 `2xx` 常量**、开局 `def_ids` / `category_def_ids` 关联、AI 脚本补齐；以 CPU 端到端一局验证「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 结算 → 重开」与逐帧哈希基线一致；`crates/w3-core/` 零 diff、零资产、`SNAPSHOT_VERSION` 仍 3。

**Non-Goals:** 双族对抗与 AI 选族（P8.3）、HUD / 表现层模型键（P8.4）、P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、不死暗夜（P9+）、中文上屏（CJK 字体许可证人工拍板）、运行时新实体真实模型绑定（模型缺键回退已知限制）、联机底座 / 回放（已在能力 S1 / S2）、真窗口人工验收（本轮如实标注「未执行 / 待补验」）、`w3-core` 任何改动、任何新增 `EffectKind` / `CommandKind` / `GameplayCommandKind` 变体 / JASS / wtg VM（N5）、不新增 `armor_type` / `attack_type` 权威字段。**会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）本 change 不做、属 P8.5，MUST 另走 ADR 0002 流程，不得借「零改动」旁路**（P8.1 已澄清的常设硬门禁、至今有效、未放松）。

## Decisions

> 玩法决策的规则依据与核实状态按 `openspec/config.yaml` 的 `rules.design` 标注：`[已核]` 规则性事实、`[待核]` 数值（不冻结进权威定义）、`[取舍]` 本项目实现决定。具体数值（hp / damage / range / supply / train_frames / mana / 冷却 / research_frames）一律 `[待核]`，全为合成原创编译期整数常量，以本机导出 SLK 核对、不入库（ADR 0001 / ADR 0004 / C6）。**「两族不对称、兽族重甲肉搏 / 正面强 / 地洞兼人口与防御」是本项目 `WC3-MELEE-RULES.md` R2 / R3.4 / R4.4 的 `[已核]` 规则性事实；「兽族与人族数值差异（更肉 / 更快 / 更低远程稳定度）」是 `[取舍]` 合成原创常量相对大小，四参考项目都没有给出该数值或结论，不得写成参考项目得来的经典事实（见「## 参考项目对照」归因纠正②）。**

### D1 兽族新定义全部落在 `2xx` 段，建筑 `speed_raw == 0`，数值差异只以常量相对大小表达

**决策**：在 `defs.rs` 兽族段新增 6 个 `DefId` 常量与对应 `UnitDef`——远程 / 猎头类单位（`[已核]` R5.1 / R5.3 框架、`[待核]` 数值）、反重甲第三战斗单位（`[已核]` R5.3）、人口 / 防御两用地洞（`[已核]` R2 / R3.4）、英雄祭坛（`[已核]` R4.2）、研发建筑（战争磨坊类，`[已核]` R4.4）、1 名英雄（`mana > 0`，`[已核]` R6）。所有新增建筑经 `defs.rs::unit(..., speed=0, ...)` 构造使 `speed_raw == 0`，供 `input.rs::derive_movability` 派生 R1.4 / R13.6 过滤（C4）；英雄 `mana > 0` 由 `defs.rs::unit_with_mana` 显式写入。数值差异（更肉 / 更快 / 更低远程稳定度）只以合成整数常量**相对大小**表达、标 `[取舍]`、引用 R2 / R5 / R5.1 / R5.3，`[待核]` 数值不冻结冒充原版。规则依据：R1.4 `[已核]`、R2 `[已核]`、R3.4 `[已核]`、R4.2 / R4.4 `[已核]`、R5 / R5.1 / R5.3 `[已核]` 框架、R6 `[已核]`、R13.6 `[已核]`；分段 `[取舍]`。

**备选方案**：
1. 给 `UnitDef` 新增 `armor_type` / `attack_type` / `is_building` 字段以表达 R5 克制矩阵 —— 否决：改 `w3-core` 权威定义结构、`FrozenDefinitions` 哈希编码与快照头部布局（ADR 0002 / C1），且 R5.3 倍率 `[待核]`、无本机导出数据；本 change 只以合成数值相对大小表达「远程 / 反重甲」语义，与 P8.1 design D1 备选 1 同一否决理由。
2. 用「建筑类型」白名单判可移动性 —— 否决：违反 P8.0 D1「判据读定义层 `speed_raw`、不写死类型」的取舍（C4 / R1.4），新增 6 类定义就要改表。
3. 把新定义散落到 `3xx` 或复用既有 `1..=6` / 人族 `1xx` —— 否决：破坏 P8.0 已定的 `1xx` / `2xx` / `9xx` 分段，两族 `def_ids` 关联与 `defs_hash` 分段测试失效。
4. 把「兽族更肉 / 更快 / 更低远程稳定度」写成从参考项目查得的经典事实并冻结具体倍率 —— 否决：四项目都无该数值 / 结论（`CLASSIC_RULES_BASELINE.md:3`），违反 C6 / `design.md` 归因纠正②，且 `[待核]` 数值不得冻结进权威定义。

**理由**：`unit()` 已把 `speed_raw` 与 `mana` 全部冻结进 `UnitDef`，`compute_defs_hash` 已逐字段覆盖；在定义层加常量即可同时满足 R1.4 派生、R6 英雄 mana、`defs_hash` 内容敏感性，无需碰内核。

**ADR 0002 / 跨平台确定性影响**：新增定义是**冻结定义内容变化**，经 `compute_defs_hash` 进 `defs_hash` / `content_hash`、写入 `SNAPSHOT_VERSION`（当前 3）快照头部；不新增权威状态字段、不改状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键。全为编译期整数常量，Windows / Linux 无差异；**不改变任何跨平台确定性结论**，不得据本 change 声称 ADR 0002 有新验证（macOS / ARM64 / 其它 rustc 仍待验证）。

**架构原则对照**：OCP（扩 DefId 段而非改内核类型）、SRP（种族 / 单位性质是定义层属性）、CARP（组合既有 `FrozenDefinitions`）、唯一状态所有者（不新增竞争状态）、显式失败（不静默把 `[待核]` 冒充原版）。

### D2 主动技能与升级只经既有 `EffectDef` / `Cast` / `Research`，新增定义走 `defs_hash` 内容变化（写透 C2）

**决策**：新增 1 个 `AbilityDef`（类别内新 ID，不占种族段），用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` 与 `EffectDef { kind, amount, duration_frames }` 组合表达主动技能，经既有 `GameplayCommandKind::Cast { caster, ability, target }` 施放；新增 1–2 项 `UpgradeDef { damage_mod, hp_mod, research_frames }`，经既有 `GameplayCommandKind::Research { upgrade }` 接通（`[已核]` R6 / R7.6 / R4.4、`[取舍]` 用既有组合而非脚本、`[待核]` 数值）。**MUST NOT** 新增 `EffectKind` 变体、`GameplayCommandKind` / `CommandKind` 变体或 JASS / wtg VM（N5）。**明确区分**：六类 `UnitDef` + `AbilityDef` + `UpgradeDef` 是**冻结定义内容变化**——经既有 `gameplay.rs::compute_defs_hash` 进 `defs_hash` / `gameplay.rs::FrozenDefinitions::content_hash` 快照头部，**不是新增权威状态字段**（C2）：不改 `Player` / `RtsMatch` 字段集合、不改状态哈希字段集合 / 快照头部布局 / `first_difference()`、`SNAPSHOT_VERSION` 仍 3、`CommandKind::Spawn` 字节逐字节不变（两族装配字节一致测试仍绿）。

**备选方案**：
1. 新增 `EffectKind::Summon` / 召唤物命令变体表达英雄技能 —— 否决：新增命令变体 / 权威状态（召唤物生命周期）须同步进状态哈希 / 快照 / `first_difference()`，MUST 另走 ADR 0002 流程（R13.6 范围限定），且属 P8.5 深度玩法。
2. 借 `orcHeroActives.json` 的 `AOwk` / `windwalk` 隐身 + 移速增益表达兽族英雄技能 —— 否决：隐身 / 无碰撞属需新增权威状态的 P8.5 语义，`AOwk` 是暴雪 rawcode、AGPLv3 仅借结构不借效果（见「## 参考项目对照」T4 兽族特有校正①）；本 change 只用既有 `EffectKind` 组合。
3. 引入 JASS 行为脚本 / `abilityBehaviors` 式注册结构 —— 否决：N5 明确不做 JASS / wtg VM；Warsmash 的 JASS 脚本仅作语义参考、AGPLv3 不得复制代码。
4. 复用既有 `ABILITY(10)` / `UPGRADE(20)` 不新增 ID —— 否决：无法表达「兽族英雄技能」与「攻 / 防两项升级」的独立内容，`defs_hash` 也不体现新增内容；新增类别内唯一 ID 无冲突。
5. 把新增定义做成对局运行时字段（而非冻结定义条目） —— 否决：那是新权威状态字段，须同步哈希 / 快照 / `first_difference()`（C1 / C2），与 P8.0 原创「`Spawn` 不含 DefId、用装配层关联」边界冲突。

**理由**：`gameplay.rs` 既定「技能采用模板 / 效果组合 / 按行为 ID 注册三层扩展，不引入脚本 VM」；既有 `Cast` / `Research` 已覆盖本 change 所需全部命令面；`compute_defs_hash` 覆盖 units / abilities / upgrades 四类，内容变化天然进 `defs_hash`。

**ADR 0002 / 跨平台确定性影响**：新增定义走 `compute_defs_hash` 进 `defs_hash` / `content_hash`；命令仍是既有变体、排序键不变；效果结算走既有 `Settlement` 单所有者、整数帧确定。不新增权威状态、不改状态哈希字段集合、`SNAPSHOT_VERSION` 仍 3、不改变跨平台结论。

**架构原则对照**：OCP（用既有扩展点组合新技能）、SRP（`Settlement` 单一结算所有者）、DIP（效果定义与执行分离）、显式失败（不静默降级到脚本 VM）。

### D3 `Race` 分派从占位替换为真实 `2xx` 常量，仍是定义层 / 装配层参数（写透 C3）

**决策**：给 `Race` 的 `ranged()/siege()/farm()/altar()/smithy()/hero()/category_def_ids` 六个新类别对 `Orc` **从 `ORC_SOLDIER` / `ORC_BARRACKS` 占位替换为 P8.2 新增的 `2xx` 真实常量**，人族保持 `1xx` 不变（不破坏 P8.1）；分派只用于 `MeleeSetup::with_race` 装配层 `def_ids` / `category_def_ids` 关联与 `AiScript` 查表（R2 `[已核]`、`[取舍]` 分段）。**MUST NOT** 新增 `Player.race` / `RtsMatch.race` 等运行时权威状态字段（C3）。种族只经装配层 `EntityId→DefId` 关联。

**备选方案**：
1. 在 `w3-core` 加 `Player.race` —— 否决：C1 / 主 `melee-match/spec.md`「内核零改动与确定性契约」；新增权威状态须同步进哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`。
2. 每个种族一份独立 `FrozenDefinitions` / 独立对局 —— 否决：一局内两族对抗需同一 `RtsMatch`，两份定义破坏单一真值源与 `defs_hash` 口径（P8.0 D2 备选 2）。
3. 用 `String` / 枚举 tag 挂实体 —— 否决：引入 `String` 进权威状态、改哈希编码，与「DefId 已能区分类型」重复（P8.0 D2 备选 3）。
4. 兽族继续沿用占位、只补 `UnitDef` 不改分派 —— 否决：AI 脚本经 `category_def_ids()` 查表会拿到 `ORC_SOLDIER` / `ORC_BARRACKS`，无法训练 / 施法 / 研发兽族新内容，端到端一局不成立。

**理由**：`defs.rs::Race` 注释已明确「只用于选择开局 `Spawn` 关联的 `DefId` 段，MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`」；替换六个方法与既有 6 个分派同构，零内核改动。

**ADR 0002 / 跨平台确定性影响**：分派是定义层纯函数（编译期常量查表），不进权威状态、不进哈希 / 快照；`SNAPSHOT_VERSION` 仍 3。跨平台无差异。

**架构原则对照**：SRP（种族是定义层属性）、OCP（改分派返回值不改内核）、ISP（按类别暴露窄方法）、LoD（装配层经 `Race` 查表、不直接触内核）。

### D4 开局装配只扩 `def_ids` / `category_def_ids`，`Spawn` 字节逐字节不变

**决策**：`setup.rs::MeleeSetup::with_race` 扩兽族开局 `def_ids` / `category_def_ids` 关联到新增类别；`CommandKind::Spawn` 的字段与字节 MUST 保持 P8.0 现状（不含 `DefId`），两族 `Spawn` 字节一致（`setup.rs::race_does_not_change_authoritative_spawn_commands` 已证）。规则依据：R4.1 主城 / R1.4 可移动性派生 `[已核]`、装配层关联 `[取舍]`。

**备选方案**：
1. 给 `CommandKind::Spawn` 加 `DefId` 字段 —— 否决：C2 / R2；改内核命令线格式、状态哈希与快照，两族 `Spawn` 字节不再一致。
2. 在运行时视图里补 `DefId` / 扩权威视图 —— 否决：会滑向改 `w3-core`；`runtime-entity-movability` 只加只读 `EntityView.speed`、未新增权威状态，本 change 不扩视图（C4）。
3. 不做装配关联、靠 DefId 数值约定 —— 否决：AI 脚本 / 端到端脚本需要稳定 `EntityId→DefId` 查表，散落约定不可验证。

**理由**：`MeleeSetup::def_ids` 已是「种族维度只停在这一层」的既定接缝，扩展它不影响任何权威命令。

**ADR 0002 / 跨平台确定性影响**：`Spawn` 命令字节、命令排序键、状态哈希与快照完全不变；`def_ids` / `category_def_ids` 是表现层只读关联、不进哈希 / 快照。两族逐帧哈希与基线一致（`melee_loop.rs` 已有先例）。

**架构原则对照**：DIP（装配层依赖只读定义）、唯一状态所有者（装配关联不反写内核）、CARP（复用既有 `MeleeSetup`）。

### D5 AI 脚本扩展沿用既有玩法命令，帧冻结外部命令

**决策**：`ai.rs::AiScript::new` 在既有 `Gather` / `Train` / 周期性 `Cast` 骚扰之外，增加训练兽族新单位、以兽族英雄实体施放主动技能、研发兽族升级三类命令，全部为 `GameplayCommandKind` 既有变体、带固定 `ExecuteFrame` / `PlayerId` / `Sequence`（R12.7 `[已核]`、R4.3 / R4.4 / R6 `[已核]`、帧冻结 `[取舍]`）。`unit_def` 经 `setup.category_def_ids()` 查表（D3 替换占位后拿到真实 `2xx`）。**MUST NOT** 扩展 `w3-core::run_ai`、MUST NOT 新增权威状态、MUST NOT 旁路写入。

**备选方案**：
1. 改 `w3-core::run_ai` 按种族产出命令 —— 否决：改内核 AI 路径与权威状态，且 AI × lockstep 接合架构（`WC3-MELEE-RULES.md` §0.3 T5）未拍板，须另走 ADR 0002 / 0005 流程。
2. 用墙钟 / 随机数驱动 AI 决策 —— 否决：破坏 30 Hz 整数帧与重放确定性（ADR 0002 §2 / §4）。
3. 让 AI 直接改玩家经济 / 单位字段 —— 否决：旁路写入权威状态、破坏唯一状态所有者与 `first_difference()`。

**理由**：`AiScript` 已是「确定性外部命令源」；`MeleeDriver` 一次性入队并按帧冻结执行，与刷新率 / 卡顿无关。扩命令种类不改机制。

**ADR 0002 / 跨平台确定性影响**：命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、外部重放注入同一脚本即可复现；不新增权威状态、不改哈希 / 快照。跨平台无差异。

**架构原则对照**：SRP（AI 只产命令）、DIP（经既有 `submit_gameplay_command`）、显式失败（命令被拒时英文日志告警、不静默降级）。

### D6 CPU 端到端一局覆盖全阶段 + 逐帧哈希基线

**决策**：扩 `crates/w3-client/tests/melee_loop.rs` 既有 `play_full_match` 或新增测试，按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」逐阶段断言（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5 `[已核]`、`[取舍]` 用合成数值让一局在合理帧数内打完），并断言两实例逐帧 `compute_state_hash()` / `capture_snapshot()` 与基线一致（ADR 0002）。**不只比最终 hash**（golden-hash 复核记录要求逐帧文本 / 字节比对，最终 hash 相同可能掩盖中间帧 desync）。反向验证（改坏验红）另见 D9 / `tasks.md` 第 6 组。

**备选方案**：
1. 只加正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，不构成反向验证（C5）。
2. 人工改坏代码验红 —— 否决：不可复现、不进 CI（P8.0 D3 备选 2）。
3. 只比最终 hash、不比逐帧 —— 否决：ADR 0002 golden-hash 复核记录要求逐帧比对。

**理由**：`melee_loop.rs` 已有 `play_full_match` 与两族逐帧哈希基线，扩阶段比另起炉灶更稳；`defs.rs::defs_hash_is_fixed_and_content_sensitive` 已是「反例元断言」先例。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图与命令、不触碰 `w3-core` 状态；仍包含重跑 `cargo test -p w3-core --test determinism` 任务以确认零影响。不得据 CPU 全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（每阶段断言、不留静默）、CARP（复用既有 driver / setup）。

### D7 R1.4 / R13.6 分层判据沿用 `runtime-entity-movability`，新增建筑 `speed_raw == 0`（写透 C4）

**决策**：新增建筑（人口地洞 / 祭坛 / 战争磨坊）`speed_raw == 0`，由 `setup.rs::MeleeSetup::movability` 经 `input.rs::derive_movability` 在**装配期**派生为 frame-0 权威只读映射；运行时 `Train` / `Build` 动态实体用只读 `input.rs::effective_movability` 的 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动（内核 `complete_production` 以真实 `def.speed_raw`、建筑为 0 写入 `UnitState.speed`，视图 speed 才可靠）。含新增建筑 / 资源点 / 主城的选中集合经 `input.rs::filter_movable` MUST NOT 生成 `Move`（R1.4 / R13.6 `[已核]`、分层判据 `[取舍]`、承接 `p9-command-hotkeys` 键盘 `M` / `A` 路径）。**两条来源 MUST NOT 互相替代**（frame-0 误用视图 speed 会因 `CommandQueue::submit` 的 `speed.max(1)` 把建筑判为可移动、令 R1 bug 复发；运行时误用装配映射则无条目、默认放行令运行时建筑响应 `Move`）；MUST NOT 回退建筑类型白名单 / 扩权威视图 / 用运行时 `speed` 替代装配映射（C4）。

**备选方案**：
1. 在 `w3-core` 收到 `Move` 时丢弃不可移动实体 —— 否决：改内核权威路径、要动状态哈希与快照（ADR 0002 / P8.0 D1 备选 1）。
2. 对每个新增建筑在输入层硬编码排除 —— 否决：违反「判据读定义层、不写死类型」（C4 / R1.4），新增 6 类就要改表。
3. 用 `EntityKind::is_resource()` 等标签判 —— 否决：`EntityKind` 是表现层模型键、不是可移动性权威来源，标签无分层判据方法。
4. 用运行时 `EntityView.speed` 统一替代装配期映射 —— 否决：frame-0 建筑视图 speed 被 `speed.max(1)` 抬成 `> 0`，会误判可移动、令 R1 bug 复发（R1.4 / R13.6 / §14）。

**理由**：P8.0 已把 frame-0 判据接线闭合、`runtime-entity-movability` 已把运行时动态实体承接闭合（只读 `EntityView.speed`）；本 change 只需保证新定义遵循 `speed_raw == 0` 约定即可自动被分层过滤，零接线改动、不重开视图口子。

**ADR 0002 / 跨平台确定性影响**：过滤只在表现层输入翻译、`CommandKind::Move` 线格式 / 排序键 / 哈希 / 快照全部不变；只读 `EntityView.speed` 是投影已有权威 `UnitState.speed`、不新增权威状态字段、`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变。跨平台生成同一命令序列。**运行时 `Train` / `Build` 动态实体承接边界**是已归档 `runtime-entity-movability` 的已知限制，本 change 沿用、不擅自改 `unwrap_or` 语义、不扩视图；残余真窗口人工验收待补验如实标注（见 Risks / `decision-memo.md` 待确认项）。

**架构原则对照**：SRP（过滤只在输入翻译一处）、DIP（只读冻结定义）、LSP（新 `UnitDef` 与既有定义对同一派生函数行为一致）、显式失败（全不可移动给反馈）。

### D8 `w3-core` 零 diff 与确定性契约不放松（写透 C1）

**决策**：本 change **不新增**权威状态字段 / `CommandKind` / `GameplayCommandKind` / `EffectKind` 变体 / 事件；`snapshot.rs::SNAPSHOT_VERSION` 仍 3；状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 稳定 `EntityId` 顺序 / FNV-1a / golden hash 逐字节不变；`git diff -- crates/w3-core/` 为空。会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）属 P8.5（`docs/roadmap/P8-human-orc-melee.md` §3 P8.5 / `WC3-MELEE-RULES.md` R13.6 范围限定），本 change 不做、**不得借「零改动」旁路**，MUST 另走 ADR 0002（状态哈希 + 快照 + `first_difference()` + determinism 26 项）。

**备选方案**：
1. 借 `runtime-entity-movability`「只读视图投影已有权威状态」的放松，把兽族数值差异 / 技能效果做成运行时权威字段 —— 否决：该放松仅适用于复用既有 `Move` / `Attack` 路径 + R1.4 分层过滤，MUST NOT 被借用于新增任何权威状态 / 命令 / 事件（R13.6 范围限定、P8.1 已澄清的常设硬门禁）。
2. 升 `SNAPSHOT_VERSION` 到 4 以「容纳」新定义 —— 否决：新增定义走 `defs_hash` 内容变化、快照头部布局不变，升版本是改内核权威契约、无必要且违反 ADR 0002。
3. 把「新增权威状态字段须同步进哈希 / 快照 / `first_difference()`」硬门禁归因给 `vera20k` 参考项目 —— 否决：出处是本项目 `CODEBUDDY.md §5` + `ADR 0002 §6-8` + 主 `melee-match/spec.md`「内核零改动与确定性契约」，不是参考项目（见「## 参考项目对照」归因纠正①）。

**理由**：主 `melee-match/spec.md`「内核零改动与确定性契约」已是 requirement；P8.0 / P8.1 / `runtime-entity-movability` / `p9-command-hotkeys` 四轮均以 `w3-core` 零 diff 通过，本 change 对称沿用。

**ADR 0002 / 跨平台确定性影响**：内核零 diff，`determinism` 26 项与 `cargo test --workspace` 全绿、`git diff -- crates/w3-core/` 为空；逐帧状态哈希与 `capture_snapshot()` 字节与基线一致。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

**架构原则对照**：唯一状态所有者、确定性契约硬红线、`w3-core` 不反向依赖上层与冻结输入边界、显式失败不静默降级。

### D9 反向验证（改坏验红）对称复用 P8.1 先例（写透 C5）

**决策**：新增 / 修改玩法回归 MUST 证明**缺陷态失败、修复态通过**（`CODEBUDDY.md §5` / `config.yaml rules.tasks` 第三条），证据形态用「测试内反例元断言」或 A-B 对照，**对称复用** `defs.rs::rebuild_definitions` / `defs.rs::degenerate_hash_ignoring_new` / 成对 `defs.rs::reverse_validation_defs_hash_*` 与 `tests/movability.rs::degenerate_filter` 先例。至少覆盖四类退化反例：① 每个新增兽族定义漏接即改 `content_hash()`；② 新增建筑 `speed_raw != 0`；③ 英雄 `mana == 0`；④ 兽族 `Race::category_def_ids` 仍回落 `ORC_SOLDIER` / `ORC_BARRACKS` 占位。仅「全绿」不算完成。

**备选方案**：
1. 只加正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，不构成反向验证。
2. 人工改坏代码验红 —— 否决：不可复现、不进 CI；改用测试内构造退化定义 / 退化过滤器并加元断言防退化。
3. 只比最终 hash、不比逐帧 / 不覆盖新增定义漏接 —— 否决：会掩盖中间帧 desync 与 `defs_hash` 漏接。

**理由**：P8.1 `defs.rs::reverse_validation_defs_hash_formal_passes_the_assertion` / `reverse_validation_defs_hash_degenerate_fails_the_same_assertion` 已是「成对反例元断言」的成熟先例，兽族只需对称扩条目。

**ADR 0002 / 跨平台确定性影响**：退化实现只在测试内构造、不触碰正式 `w3-core` 状态；缺陷态 / 修复态证据均为 CPU 侧编译期整数常量，跨平台一致。不得据测试全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（退化实现必失败、不留静默）、CARP（复用既有 `rebuild_definitions` / `degenerate_hash_ignoring_new`）。

### D10 数值 `[取舍]` / 零资产 / 参考归因纠正 / 单 change 范围边界（写透 C6 / C8）

**决策**：全部为合成原创编译期整数常量；**不复制** Warsmash rawcode（`AOwk` / `AHwe` / `ASlo` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 `FACTION_UNIT_ROSTERS.md` 原创机制名（`架弩` / `勒步射击` 等）；兽族「更肉 / 更快 / 更低远程稳定度」只以常量相对大小表达、标 `[取舍]`、引用 R2 / R5 / R5.1 / R5.3，**不得写成参考项目得来的经典事实**（`[已核]` 框架是本项目业务规则）；数值 `[待核]` 以本机导出 SLK 核对、不入库（ADR 0001 / ADR 0004 / R2 / R5 / R6 / §14）。`AbilityDef` 只用既有 `EffectKind` + `EffectDef` 组合、走既有 `Cast`、不新增 `EffectKind` / 命令变体 / JASS VM（N5）；`UpgradeDef` 经既有 `Research`。**一个 change 不并后续**：P8.3 双族对抗与 AI 选族 / P8.4 HUD 表现层模型键 / P8.5 五个深度玩法 / P9+ 不死暗夜 / 中文上屏 / 运行时新实体真实模型绑定 / 真窗口人工验收均由后续独立 change 承接（`docs/roadmap/P8-human-orc-melee.md` §3 / §6、已归档 `p8-human-faction/decision-memo.md`「归档说明」）。

**备选方案**：
1. 照搬 Warsmash `AOwk` / `AHfs` / `Hamg` 等 rawcode 或 godot `FACTION_UNIT_ROSTERS.md` 原创三族单位名 / 数值 —— 否决：C6 / ADR 0001；AGPLv3 仅借鉴结构语义不复制代码 / rawcode / 单位名 / 数值，原创三族不是经典兽族复刻，暴雪 rawcode 属资产派生标识不得入库。
2. 按「人族有 `humanUnitActives.json` 故兽族应有对称 `orcUnitActives.json`」臆造兽族单位技能表 —— 否决：参考项目实际无该文件（`UPSTREAM.md` T4 矩阵条目「兽族仅 `orcHeroActives.json`」），不借分组数量或命名、不臆造（见「## 参考项目对照」兽族特有校正③）。
3. 把 P8.3–P8.5 深度玩法顺手塞进 P8.2 以「一次做完兽族」 —— 否决：C8 / `docs/roadmap/P8-human-orc-melee.md` §6 第 4 条 P8.1 与 P8.2 串行、P8.3 在两族 CPU 闭环都绿之后开；深度玩法各单独 change、串行推进。
4. 把「兽族数值差异」标成 `[已核]` 经典事实并冻结进权威定义 —— 否决：四参考项目都无该数值 / 结论（`CLASSIC_RULES_BASELINE.md:3`「未锁定具体经典补丁」/ `:52`「数据行完整 ≠ 玩法完成」），违反 C6 与 §14 活文档流程「不静默改规则」。

**理由**：ADR 0001 / 0004 与活文档 §0 / §14 是单一权威；参考项目只借「按种族定义表分组 + 数值读 Def 不写死 + Data→Logic→Present 分层」的**工程纪律**（支撑把兽族数值放进 `defs.rs` 的 `2xx` 段常量、逻辑层不写 `if race == Orc { hp += … }`），不携带任何数值 / rawcode / 字符串表。

**ADR 0002 / 跨平台确定性影响**：数值为编译期整数常量、逻辑层不按种族分支改写权威状态，跨平台无差异；不新增权威状态 / 命令 / 事件，`SNAPSHOT_VERSION` 仍 3。不改变任何跨平台确定性结论。

**架构原则对照**：DIP（数值读定义、逻辑不写死）、SRP（数值是定义层属性）、只读参考不复制代码硬红线、显式失败（`[待核]` 不冒充原版）。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」（T1–T6 表、触发时机第 1–5 条）与 `openspec/config.yaml` 的 `rules.design`。本 change 命中 **T1**（确定性内核 / 状态哈希 / 命令排序，C1 / C8 / D8 需要）、**T2**（移动 / 编队 / 战斗结算语义，R5 / R1.4 / D7）、**T4**（玩法语义 / 技能 / 单位行为，**兽族特有重点**）、**T5**（场景装配 / Melee 初始化，开局 `def_ids` / `category_def_ids`）。结论由只读 `reference-researcher` 出「参考对照报告」（leader 本会话已接收），下列所有锚已由 `spec-steward` **逐条实际打开核实存在、语义相符**（本轮打开的是 `C:/open_rts/<项目>` 固定提交，见 `UPSTREAM.md` 开头固定版本表，不在本文重复本机绝对路径）。四个参考项目均为**只读**、代码不进入本仓库（ADR 0001）；本机绝对路径 / 资产 / 派生数据一律不入库。许可证：`vera20k` GPLv3 + Section 7 附加条款（仅真的复制代码时才进入）、`godot_warcraft3` / `rts_kernel_cs` MIT、`WarsmashModEngine` **AGPLv3 仅借鉴语义、不得复制任何代码**。参考缺口按 `UPSTREAM.md` 触发时机第 5 条如实登记、回到人拍板、不臆造。**参考项目 `file:line` 锚不漂移、按对照报告原样保留；本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目，不写精确 `:NN` 行号。**

- **T4 兽族特有（重点，如实校正，不得写成对称表）→ `WarsmashModEngine`（AGPLv3，仅语义）**。锚（逐条打开核实）：`WarsmashModEngine/core/assets/abilityBehaviors/orcHeroActives.json:2-4,11-41,42-221`（**存在**，223 显示行、末行无换行故 `wc -l` 记 222；`:2-4` 为 `abilityList` / `{"id":"AOwk","castId":"windwalk"}` / `NORMAL_NOTARGET`，`:11-41` / `:42-221` 是 `INVISIBLE` + `MVSPDPCT` 移速增益等 buff 数据行）——**里面是 `AOwk` / `windwalk` 疾风步隐身 + 移速增益，不是牛头人技能**；隐身 / 无碰撞属需新增权威状态的 P8.5 语义，本 change **不借该效果、只借「按种族分组组织数据 + 技能拼 Effect + Behavior + 数据 / 效果分离」的结构**。`WarsmashModEngine/resources/Scripts/Abilities/Orc/TaurenChieftain/AbilityWarStomp.j:1-39`（**存在，39 行；`Abilities/Orc/` 下仅此 1 个 `.j`**）：`:20-23` `populateData` 从 `editorData` 读伤害 `ABILITY_FIELD_DATA_A` + `buffId`、**数值不写死**，`:33-36` `doEffect` 无目标 AoE `GroupEnumUnitsInRangeOfUnit` 伤害 + 控制 → 语义可映射到既有 `EffectKind::{Damage,Stun}` + `Cast`（借「数据 / 效果分离」结构，不借 JASS 代码 / rawcode / 数值）。**`abilityBehaviors/orcUnitActives.json` 不存在**（Glob 核实该目录共 15 个 json，含 `humanUnitActives` / `nightElfUnitActives` / `undeadUnitActives` / `neutralUnitActives`，**唯独无 orc 单位技能表**；与 `UPSTREAM.md` T4 矩阵条目「兽族仅 `orcHeroActives.json`」一致）；**`Abilities/Orc/` 下无 `FarSeer` / `Blademaster` / `ShadowHunter`**（`find` 仅 `TaurenChieftain/AbilityWarStomp.j`，如实登记、不臆造）。`WarsmashModEngine/core/src/com/etheller/warsmash/units/manager/MutableObjectData.java:411,423,425,428`（`urac` race / `arac` ability race / `frac` buff race / `grac` upgrade race）是**按种族分类的通用字段、兽族为取值之一，无「兽族专用字段」**；`Grep Orc|ORC|ogru|ogre|opeo` 于该文件 **No matches**（本轮实测）。P8.1 对称锚仍存在：`humanUnitActives.json:2-5`（`Aslo` / `slow` / `NORMAL_UNITTARGET` / `extraAutoTargetConditions`）、`humanHeroActives.json:2-5`（`AHfs` / `flamestrike` / `NORMAL_POINTTARGET` / `onBeginCasting`）、`resources/Scripts/Abilities/Human/Archmage/SummonWaterElemental.j:13-17,19-37,40-42`（`populateData` 数据 / `doEffect` 效果 / `register` 注册三段结构）。**借鉴**：只借「按种族分组组织数据 + 技能拼 Effect + Behavior + 数据 / 效果分离」**结构**。**不借鉴 / 边界**：**不借** `AOwk` / `AHwe` / `Aslo` / `AHfs` 等 rawcode、单位名 / 英雄名 / 数值、原创世界观、Java / LibGDX 代码；**不借分组数量或命名**（不得据「无 `orcUnitActives.json`」推断兽族应有一张对称单位技能表、不得臆造 `FarSeer` / `Blademaster` / `ShadowHunter`）；`orcHeroActives.json` 的隐身 / 移速增益效果不借（P8.5 语义）。许可证理由：AGPLv3 与本项目 GPLv3 组合有条件、未经 ADR 0001 复核不得引入其代码，故只借结构语义。

- **T4 / T2 数值差异语义（关键澄清，不得写成经典事实）→ `godot_warcraft3`（MIT）**。锚（逐条打开核实）：`godot_warcraft3/docs/roadmap/NEXT.md:152-153`（`:152`「数值读 SLK Def，禁止逻辑里写死造价 / 伤害」、`:153`「新技能优先拼 Effect + 注册 Behavior」）、`docs/design/game/ABILITY_SYSTEM.md:12-13`（`:12` 权威数据来自 WC3 Def + `AbilityDataDef` / `UnitAbilitiesDef` / `HumanAbilityFunc.txt`、`:13` **Data → Logic → Present** 分层）、`docs/design/game/FACTION_UNIT_ROSTERS.md:1-5,9-12,205-230`（`:1-5` **原创三族**且「名称 / 造价 / 数值是种子、不是最终平衡承诺」、`:9`「不逐项镜像经典 RTS 单位」、`:215-217` 原创机制名「架弩 / 协同施工 / 仓站连接」「勒步射击 / 可迁兵营 / 角矛突进」「扎根 / 远枝信标 / 生境节点」、U0 数值是每族数据行、U2「同成本 / 同人口 / 同生产时间下接战结果、不凭单挑定价」、U3「每族至少两种开局和两种主力组合」）、`docs/roadmap/CLASSIC_RULES_BASELINE.md:3,52`（`:3`「未锁定具体经典补丁；不能宣称当前数值与某一完整原版版本一致」、`:52`「不能把数据行完整性等同于四族玩法完成」）。`godot_warcraft3/docs/design/game/GAMEPLAY_VERTICAL.md:39-48`（`Hamg` / `hfoo` / `htow` / `hrif` 等暴雪 rawcode）与 `packages/gameplay/catalog/melee_race_preview.gd:55-76`（`htow` / `ogre` / `unpl` / `opeo`）是暴雪 rawcode（**`ogru` 字面在二者中零命中、不得写成锚**，本轮 `grep` 实测）。**关键澄清**：四项目都**没有**给出「经典 WC3 兽族 vs 人族 = 更肉 / 更快 / 更低远程稳定度」的数值或结论；「两族不对称、兽族重甲肉搏 / 正面强 / 地洞兼人口与防御」是**本项目业务规则** `WC3-MELEE-RULES.md` R2 / R3.4 / R4.4 的 `[已核]` 规则性事实，具体数值差异是 `[取舍]` 的合成原创编译期整数常量，**不得写成参考项目得来的经典事实**。**借鉴**：「按种族定义表分组 + 数值读 Def 不写死 + Data / Catalog / Logic / Present 分层」的工程纪律（支撑把兽族数值放进 `defs.rs` 的 `2xx` 段常量、逻辑层不写 `if race == Orc { hp += … }`）。**不借鉴 / 边界**：Godot / GDScript 实现、原创三族单位名 / 机制名、暴雪 rawcode、任何数值；`FACTION_UNIT_ROSTERS.md` 是原创世界观、不是经典兽族复刻，只借排期结构 / 批次判据、不作本 change 验收判据。许可证理由：MIT 借结构、不复制实现代码。

- **T2 移动 / 战斗（复用、不铺开）→ `rts_kernel_cs`（MIT）+ `godot_warcraft3`（MIT）**。锚（逐条打开核实）：`rts_kernel_cs/docs/MOVEMENT.md:77`（「stable ideal slots using deterministic minimum squared-distance matching」稳定槽位）、`rts_kernel_cs/docs/FRAMEWORK.md:109`（明确「不纳入：…战斗 / 经济 / 技能迁移」）、`godot_warcraft3/docs/design/game/COMBAT_SYSTEM.md:48-52,371,409-423`（`:48-52` `ATTACK` / `ATTACK_MOVE` / `HOLD` 索敌分类表、`:371` `atk_type1` normal / pierce / siege / magic / chaos / hero、`:409` `CombatRng`、`:409-423` 伤害结算旁证）、`godot_warcraft3/docs/design/game/UNIT_AI.md:201-206`（`PASSIVE` / `CAMP_CREEP` / `TEAM_PLAYER` / `REACTIVE` profile）。**借鉴**：「前排 / 远程 / 反重甲语义分类 + 稳定 ID 落点」的**语义分类**。**不借鉴 / 边界**：战斗结算 MUST 走 `w3-core` 既有 `Attack` / `Cast`（`command.rs::CommandKind` / `gameplay.rs::GameplayCommandKind`），**不移植** godot `DamagePipeline` / `CombatRng`（GDScript 实现）、**不采用** rts_kernel_cs `double` 路线（ADR 0002）；兽族「更低远程稳定度」只以合成整数相对大小表达，**不新增 `armor_type` / `attack_type` 权威字段**（P8.1 design D1 备选 1 同一否决理由 / C1）。R5 依据仍是本项目 `w3-core` + `WC3-MELEE-RULES.md` R5.1 / R5.3，`FRAMEWORK.md:109` 明确把战斗 / 经济 / 技能迁移排除在该内核契约之外。许可证理由：MIT 借语义分类、不复制 C# / GDScript 实现代码。

- **T5 场景装配 → `godot_warcraft3`（MIT）**。锚（逐条打开核实）：`godot_warcraft3/docs/design/game/ROADMAP.md:12-23,81,88-90`（`:12`「WE 对战初始化本质是一张**固定动作表**」、`:14-21` 8 条动作、`:23` 先做 `MeleeBootstrap` 硬编码模块、`:81`「创建对战初始单位（主城 + 工人于 sloc，**需种族→单位 ID 表**）」、`:88-90` 输出「要生成 / 删除的单位指令」且「**不**解析 wtg」）、`packages/gameplay/match/melee_bootstrap.gd:27-43`（`spawn_at_sloc(... race ...)` 经 `MeleeRacePreview.town_hall_id(race)` / `worker_id(race)` 查表生成开局单位）、`packages/gameplay/catalog/melee_race_preview.gd:51-78`（`town_hall_id` / `worker_id` 按 race 返回 typeId 的查表结构）。**借鉴**：「动作表 + 种族→单位 ID 查表 + 输出指令、不解析 wtg」的**装配结构**，对 `setup.rs::MeleeSetup::with_race` 兽族 `def_ids` / `category_def_ids` 关联可借。**不借鉴 / 边界**：**`CommandKind::Spawn` 字节 MUST 不变**（`command.rs::CommandKind::Spawn` 不含 DefId，P8.0 原创、`setup.rs::race_does_not_change_authoritative_spawn_commands` 两族字节一致测试）；不借 Godot / GDScript、字符串 typeId、运行时会话状态、`melee_race_preview.gd` 的 rawcode（C6 / `UPSTREAM.md` T5 边界）。`WarsmashModEngine/resources/Scripts/melee.jui` 仅为 JASS UI 帧定义、不作初始化动作表（`UPSTREAM.md` T4 矩阵条目）。许可证理由：MIT 借结构、不复制实现代码。

- **T1 确定性内核 + 归因纠正① → `vera20k`（GPLv3 + Section 7，仅范式）**。锚（逐条打开核实）：`vera20k/src/net/lockstep.rs:35-39`（`CHECKSUM_HISTORY_LEN` / `MultiplayerChecksumHistory` / `ChecksumMismatch` 等纯命令传输导出）、`:648`（`checksum_history.compare(frame_info)?` 逐帧 checksum 对比）。**借鉴**：lockstep 逐帧 checksum 的 desync 检测思路、命令传输不携带游戏规则（呼应「表现层只传输入命令、不反写内核」）。**不借鉴 / 边界**：RA2 专属 gamemd 地址 / INI / SHP / VXL / 2D 等距实现不借；未复制任何代码，故 **GPLv3 + Section 7 附加条款不进入本项目**（ADR 0001）。**归因纠正①（重要）**：「新增权威状态字段须同步进状态哈希 / 快照 / `first_difference()`」硬门禁的出处是**本项目** `CODEBUDDY.md §5` + `docs/decisions/0002-determinism.md` §6-8（ADR 0002）+ 主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」，**不是 `vera20k`**；C1 / D8 的依据指向这三者，`decision-memo.md` 该承重约束锚指向本项目权威文件 / `UPSTREAM.md` 矩阵条目，**不指向参考项目路径**。本 change 新增 `UnitDef` / `AbilityDef` / `UpgradeDef` 是**冻结定义内容变化**（`compute_defs_hash` 覆盖四类），`w3-core` 可零 diff、`SNAPSHOT_VERSION` 仍 3；会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）属 P8.5（`docs/roadmap/P8-human-orc-melee.md` §3 P8.5 / `WC3-MELEE-RULES.md` R13.6），本 change 不做、不得借「零改动」旁路。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

- **两处归因纠正（如实登记、不得误挂参考项目）**：① 见上 T1 条——「新增权威状态字段须同步进哈希 / 快照 / `first_difference()`」是本项目 ADR 0002 / `CODEBUDDY.md §5` / 主 `melee-match/spec.md` 的硬门禁，**不是 `vera20k`**，`decision-memo.md` 的 C1 锚指向本项目权威文件 / `UPSTREAM.md` 矩阵条目。②「兽族与人族数值差异（更肉 / 更快 / 更低远程稳定度）」**不是参考项目得来的经典事实**：四项目都无该数值 / 结论（`CLASSIC_RULES_BASELINE.md:3`）；「两族不对称、兽族重甲肉搏 / 正面强 / 地洞兼人口与防御」是本项目 `WC3-MELEE-RULES.md` R2 / R3.4 / R4.4 的 `[已核]` 规则性事实，具体数值差异是 `[取舍]` 合成原创编译期整数常量（C6 / D1 / D10）。`decision-memo.md` 该承重约束锚指向 `WC3-MELEE-RULES.md` R2 / R5 / §14 与 `UPSTREAM.md` T4 / T5 边界，**不指向参考项目路径冒充经典结论**。

- **参考缺口 G1–G5（如实登记、不臆造、不冒充参考结论）**：**G1** 编译期整数 `DefId` 按种族分段且**不引入运行时权威状态**——Warsmash `urac` / godot `melee_race_preview.gd` 都带运行时会话 / 字符串 typeId 语义，四项目均无本项目 C3 式纯定义层分派；本项目原创取舍。**G2**「1 英雄 + 1 主动技能最小集用既有 `EffectDef` 组合表达」——godot F3 / F10 走完整技能系统、Warsmash 走 JASS，无「既有 `EffectKind` 四变体组合」的最小集；本项目原创取舍。**G3** `Spawn` 不含 `DefId`、用装配层 `def_ids` 关联承载种族——P8.0 原创（`command.rs::CommandKind::Spawn`），参考项目均带 typeId / 会话状态。**G4** CPU 无渲染逐帧哈希「一局」验收脚本——`crates/w3-client/tests/melee_loop.rs` 本项目先例，参考项目无对应做法。**G5（兽族特有新增缺口）**：Warsmash 固定提交内**没有** `orcUnitActives.json`、`Abilities/Orc/` 只有 `AbilityWarStomp.j`，故「远程猎头 / 反重甲 / 人口地洞 / 祭坛 / 战争磨坊」五类兽族定义在参考项目中**没有任何兽族专属数据 / 行为文件可对照**，「兽族与人族数值差异」也无经典数值来源（`CLASSIC_RULES_BASELINE.md:3`）；这五类 + 数值差异 MUST 登记为本项目在 `WC3-MELEE-RULES.md` R2 / R3.4 / R4.2 / R4.4 / R5 / R6 下的**合成原创 `[取舍]`**、不作参考结论。以上缺口**方向已由用户 2026-11-17 指令拍板放行（等价规格门批准）**，但具体数值 `[待核]`、不冒充已对齐或已验证（`decision-memo.md` 待确认项 / 停止条件）。

- **许可证边界**：`WarsmashModEngine`（AGPLv3）仅借语义、不复制代码 / rawcode / 单位名 / 数值、不引入 Java / LibGDX 依赖（若需代码 MUST 按 `ADR 0001` 复核点重评许可证）；`godot_warcraft3` / `rts_kernel_cs`（MIT）借结构、不借实现 / 原创三族名 / `double` 路线；`vera20k`（GPLv3 + Section 7）未复制代码故附加条款不进入。`CommandFunc.txt` / `CommandStrings.txt` / `Hotkey` / `Buttonpos` / SLK 一律不入库（ADR 0001 / ADR 0004）。四项目均不复制代码进仓库。

## Risks / Trade-offs

- **新增定义数量与 `defs_hash` 变化** → `gameplay.rs::compute_defs_hash` 对四类定义的条目数与每字段敏感，新增 6 `UnitDef` + 1 `AbilityDef` + 1–2 `UpgradeDef` 必然改变 `content_hash` / `defs_hash`；这是**预期内的冻结定义内容变化**、不是权威状态字段新增（C2 / D2）。缓解：D6 端到端测试比对「相同脚本两实例逐帧哈希一致」而非与改动前旧基线数值相等；`defs.rs::defs_hash_is_fixed_and_content_sensitive` 元断言复核内容敏感性，快照头部 `defs_hash` / `content_hash` 随之更新、`SNAPSHOT_VERSION` 仍 3。
- **`speed_raw == 0` 语义** → 作为可移动性判据是 P8.0 约定；新增 3 类建筑 MUST 一致传 `speed == 0`，否则 `input.rs::derive_movability` 误判、R1.4 bug 从新建筑路径复发。未来「可移动但速度为 0」（被定身）须在定义层显式区分、不能靠运行时状态改写。缓解：D1 / D7 与 `unit-orders` delta 的 `speed_raw` 派生 Scenario + 反向验证（D9）。
- **`Spawn` 不含 `DefId`、用装配层关联** → 兽族新定义只经 `setup.rs::MeleeSetup::with_race` 的 `def_ids` / `category_def_ids` 关联，AI 脚本 / 端到端脚本依赖稳定 `EntityId→DefId` 查表；D3 必须把占位替换为真实常量，否则查表拿到 `ORC_SOLDIER` / `ORC_BARRACKS`、一局不成立。缓解：D4 两族 `Spawn` 字节一致测试 + D9 退化反例④。
- **运行时动态实体承接边界** → 已归档 `runtime-entity-movability` 用只读 `EntityView.speed` 承接运行时 `Train` / `Build` 动态实体；本 change 新增大量可训练 / 可建造实体，沿用该分层判据、**不擅自改 `unwrap_or` 语义 / 不扩权威视图**。frame-0 静态集合与运行时动态实体的验证边界如实标注；残留真窗口人工验收待补验（`decision-memo.md` 待确认项）。
- **数值差异不是经典事实** → 6 类新定义的数值全为合成原创常量，兽族「更肉 / 更快 / 更低远程稳定度」只以常量相对大小表达、标 `[取舍]`；R5.3 克制倍率 / R6 经验 / R3.4 人口上限等具体数值 `[待核]`，以本机导出 SLK 核对、不入库；`godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md:52` 明确「数据行完整 ≠ 玩法完成」，本 change 验收线是「一局能按各阶段打完且逐帧哈希一致」，不是「数值对齐原版」或「参考项目得来的经典结论」（C6 / 归因纠正② / G5）。
- **真窗口人工验收未执行 / 待补验** → 本轮 CPU 侧以回归 + 门禁自证，真窗口 / GPU / egui overlay 一律未人工观察，如实标注「未执行 / 待补验」、不冒充通过（沿用 P8.1 / `p9-command-hotkeys` 归档说明先例）。
- **`file:line` 锚漂移已用符号名规避** → 本项目 `w3-client` 代码会被 owner 改动，C1–C8 / `design.md` 的本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目、不写精确 `:NN` 行号（P8.1 行号漂移返工教训）；参考项目为只读固定提交、其 `file:line` 锚不漂移、按对照报告原样保留。apply 后四件套任一 artifact 改动即视为规格解锁、MUST 按 `CODEBUDDY.md §8 / §9` 同步回查其余三份并重走规格门，不得只改一份。
- **范围外承接** → 双族对抗 / HUD / P8.5 深度玩法 / P9+ 不死暗夜由后续独立 change 承接（C8），本 change 不因「顺手」扩范围；联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项（C7）。
