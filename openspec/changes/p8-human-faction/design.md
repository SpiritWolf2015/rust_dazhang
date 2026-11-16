# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实）：`crates/w3-client/src/melee/defs.rs:47-66` 定义人族 `101..104` / 兽族 `201..204` / 中立 `9xx` 分段常量，`:74-135` 定义 `Race` 枚举与 `peasant()` / `soldier()` / `town_hall()` / `barracks()` / `gold_mine()` / `tree()` 分派，`:143-153` 的 `unit()` 签名为 `unit(hp, speed, damage, range, supply, train_frames)` 且写 `UnitDef.speed_raw`（建筑传 `speed == 0`），`:162-216` 的 `synthetic_definitions()` 构造 16 个 `UnitDef` + 1 `AbilityDef(10)` + 1 `UpgradeDef(20)` + 1 `ItemDef(30)`。`crates/w3-client/src/melee/setup.rs:143-165` 的 `MeleeSetup` 含装配层 `def_ids: BTreeMap<u64, DefId>`，`:684-700` 的 `race_does_not_change_authoritative_spawn_commands` 已证明两族 `Spawn` 字节一致。`crates/w3-client/src/melee/ai.rs:30-85` 的 `AiScript::new` 预生成 `Gather` / `Train` / `Cast`，`crates/w3-client/src/melee/driver.rs:55-88` 的 `advance_ticks` 做「敌方主城消失 → `Leave`」结算。`crates/w3-client/tests/melee_loop.rs:446-560` 已有 `play_full_match` 逐帧哈希与两族基线比对。

内核只读事实：`crates/w3-core/src/command.rs:14-21` 的 `CommandKind::Spawn` 无 `DefId`；`crates/w3-core/src/gameplay.rs:31-83` 定义 `EffectKind::{Damage, Heal, BuffStat, Stun}` / `EffectDef` / `UnitDef` / `AbilityDef` / `UpgradeDef`，`:144-181` 的 `compute_defs_hash` 覆盖四类定义、`:212-215` 的 `content_hash()` 进快照头，`:539-578` 的 `GameplayCommandKind` 含 `Cast` / `Train` / `Research` / `Gather` / `SetAi` / `Leave` 等既有变体；`crates/w3-core/src/snapshot.rs:15` 为 `SNAPSHOT_VERSION = 3`。主 `openspec/specs/melee-match/spec.md:127-129` 已把「内核零改动 / 不新增权威状态字段 / `SNAPSHOT_VERSION` 仍为 3」写成 requirement。约束：ADR 0001 / 0002 / 0004、`CODEBUDDY.md` §5 / §8 / §9、`docs/references/WC3-MELEE-RULES.md` R1–R13。

## Goals / Non-Goals

**Goals:** 在 `crates/w3-client/src/melee/` 定义层与装配层补齐人族远程 / 反重甲 / 人口 / 祭坛 / 研发 / 英雄 6 类 `UnitDef`、1 主动技能、1–2 项升级、`Race` 分派、开局 `def_ids` 关联、AI 脚本；以 CPU 端到端一局验证「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 结算 → 重开」与逐帧哈希基线一致；`crates/w3-core/` 零 diff、零资产。

**Non-Goals:** 兽族完整化（P8.2）、双族对抗与 AI 选族（P8.3）、HUD / 表现层模型键（P8.4）、P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、联机底座 / 回放（已在能力 S1 / S2）、真窗口人工验收（本轮如实标注「未执行 / 待补验」）、`w3-core` 任何改动、任何新增 `EffectKind` / 命令变体 / JASS VM（N5）。**会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）本 change 不做，属 P8.5，MUST 另走 ADR 0002 流程，不得借「零改动」旁路。**

## Decisions

> 玩法决策的规则依据与核实状态按 `openspec/config.yaml:63` 的 `rules.design` 标注：`[已核]` 规则性事实、`[待核]` 数值（不冻结进权威定义）、`[取舍]` 本项目实现决定。具体数值（hp / damage / range / supply / train_frames / mana / 冷却 / research_frames）一律 `[待核]`，全为合成原创编译期整数常量，以本机导出 SLK 核对、不入库（ADR 0001 / ADR 0004 / C6）。

### D1 人族新定义全部落在 `100..200` 段，建筑 `speed_raw == 0`

**决策**：在 `defs.rs` 人族段新增 6 个 `DefId` 常量与对应 `UnitDef`——远程单位（穿刺 / 长射程 `[已核]` R5.1 / `[已核]` R5.3 框架、`[待核]` 数值）、反重甲第三战斗单位（`[已核]` R5.3）、人口建筑（`[已核]` R3.4）、英雄祭坛（`[已核]` R4.2）、研发建筑（`[已核]` R4.4）、1 名英雄（`mana > 0`，`[已核]` R6）。所有新增建筑经 `unit(..., speed=0, ...)` 构造，使 `speed_raw == 0`，供 `derive_movability` 派生 R1.4 / R13.6 过滤（C4）；英雄 `mana > 0` 由新增重载或 `UnitDef` 构造处显式写入（既有 `unit()` 固定 `mana: 0`）。规则依据：R1.4 `[已核]`、R2 `[已核]`、R3.4 `[已核]`、R4.2 / R4.4 `[已核]`、R5.1 / R5.3 `[已核]` 框架、R6 `[已核]`、R13.6 `[已核]`；数值 `[待核]`、分段 `[取舍]`。

**备选方案**：
1. 给 `UnitDef` 新增 `armor_type` / `attack_type` / `is_building` 字段以表达 R5 克制矩阵 —— 否决：改 `w3-core` 的权威定义结构、`FrozenDefinitions` 哈希编码与快照头部布局（ADR 0002），且 R5.3 倍率 `[待核]`、无本机导出数据，本 change 只以合成数值的相对大小表达「远程 / 反重甲」语义，不引入新字段。
2. 用「建筑类型」白名单判可移动性 —— 否决：违反 P8.0 D1「判据读定义层 `speed_raw`、不写死类型」的取舍（C4 / R1.4），新增 6 类定义就要改表。
3. 把新定义散落到 `3xx` 或复用既有 `1..=6` —— 否决：破坏 P8.0 已定的 `1xx` / `2xx` / `9xx` 分段（`defs.rs:40-45`），两族 `def_ids` 关联与 `defs_hash` 分段测试失效。

**理由**：`unit()` 已把 `speed_raw` 与 `mana` 全部冻结进 `UnitDef`，`compute_defs_hash`（`gameplay.rs:144-157`）已逐字段覆盖；在定义层加常量即可同时满足 R1.4 派生、R6 英雄 mana、`defs_hash` 内容敏感性，无需碰内核。

**ADR 0002 / 跨平台确定性影响**：新增定义是**冻结定义内容变化**，经 `compute_defs_hash` 进 `defs_hash` / `content_hash`、写入 `SNAPSHOT_VERSION`（当前 3）快照头部；不新增权威状态字段、不改状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键。全为编译期整数常量，Windows / Linux 无差异；**不改变任何跨平台确定性结论**，不得据本 change 声称 ADR 0002 有新验证（macOS / ARM64 / 其它 rustc 仍待验证）。

**架构原则对照**：OCP（扩 DefId 段而非改内核类型）、SRP（种族 / 单位性质是定义层属性）、CARP（组合既有 `FrozenDefinitions`）、唯一状态所有者（不新增竞争状态）。

### D2 主动技能与升级只经既有 `EffectDef` / `Cast` / `Research`，不新增类型

**决策**：新增 1 个 `AbilityDef`（类别内新 ID，不占种族段），用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` 与 `EffectDef { kind, amount, duration_frames }` 组合表达主动技能，经既有 `GameplayCommandKind::Cast { caster, ability, target }` 施放；新增 1–2 项 `UpgradeDef { damage_mod, hp_mod, research_frames }`，经既有 `GameplayCommandKind::Research { upgrade }` 接通（`[已核]` R6 / R7.6 / R4.4、`[取舍]` 用既有组合而非脚本、`[待核]` 数值）。**MUST NOT** 新增 `EffectKind` 变体、`GameplayCommandKind` / `CommandKind` 变体或 JASS / wtg VM（N5）。

**备选方案**：
1. 新增 `EffectKind::Summon` / 召唤物命令变体表达英雄技能 —— 否决：新增命令变体 / 权威状态（召唤物生命周期）须同步进状态哈希 / 快照 / `first_difference()`，MUST 另走 ADR 0002 流程（R13.6 范围限定 `WC3-MELEE-RULES.md:304`），且属 P8.5 深度玩法。
2. 引入 JASS 行为脚本 / `abilityBehaviors` 式注册结构 —— 否决：N5 明确不做 JASS / wtg VM；Warsmash 的 JASS 脚本仅作语义参考、AGPLv3 不得复制代码（见「参考项目对照」T4）。
3. 复用既有 `ABILITY(10)` / `UPGRADE(20)` 不新增 ID —— 否决：无法表达「英雄技能」与「攻 / 防两项升级」的独立内容，`defs_hash` 也不体现新增内容；新增类别内唯一 ID 无冲突（`defs.rs:30-35`）。

**理由**：`gameplay.rs:10` 已明确「技能采用模板 / 效果组合 / 按行为 ID 注册三层扩展，不引入脚本 VM」；既有 `Cast` / `Research` 已覆盖本 change 所需全部命令面（`gameplay.rs:539-578`）。

**ADR 0002 / 跨平台确定性影响**：新增 `AbilityDef` / `UpgradeDef` 走 `compute_defs_hash`（`gameplay.rs:158-179`）进 `defs_hash`；命令仍是既有变体、排序键不变；效果结算走既有 `Settlement` 单所有者、整数帧确定。不新增权威状态、不改变跨平台结论。

**架构原则对照**：OCP（用既有扩展点组合新技能）、SRP（`Settlement` 单一结算所有者）、DIP（效果定义与执行分离）、显式失败（不静默降级到脚本 VM）。

### D3 `Race` 分派扩展到新类别，仍是定义层 / 装配层参数

**决策**：给 `Race` 增加 `ranged()` / `siege()` / `farm()` / `altar()` / `smithy()` / `hero()` 一类方法（命名实现定），人族返回 `1xx` 新常量、兽族沿用既有 `2xx`（本 change 不补兽族新类别，返回既有对应项或占位由实现定，MUST 不破坏 P8.2）。分派只用于 `MeleeSetup::with_race` 装配层 `def_ids` 关联与 `AiScript` 查表（R2 `[已核]`、`[取舍]` 分段）。**MUST NOT** 新增 `Player.race` / `RtsMatch.race` 等运行时权威状态字段。

**备选方案**：
1. 在 `w3-core` 加 `Player.race` —— 否决：C1 / 主 `melee-match/spec.md:127-129`；新增权威状态须同步进哈希 / 快照 / `first_difference()`，可能升 `SNAPSHOT_VERSION`。
2. 每个种族一份独立 `FrozenDefinitions` / 独立对局 —— 否决：一局内两族对抗需同一 `RtsMatch`，两份定义破坏单一真值源与 `defs_hash` 口径（P8.0 D2 备选 2）。
3. 用 `String` / 枚举 tag 挂实体 —— 否决：引入 `String` 进权威状态、改哈希编码，与「DefId 已能区分类型」重复（P8.0 D2 备选 3）。

**理由**：`Race`（`defs.rs:68-80`）注释已明确「只用于选择开局 `Spawn` 关联的 `DefId` 段，MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`」；扩展方法与既有 6 个分派同构，零内核改动。

**ADR 0002 / 跨平台确定性影响**：分派是定义层纯函数（编译期常量查表），不进权威状态、不进哈希 / 快照；`SNAPSHOT_VERSION` 仍 3。跨平台无差异。

**架构原则对照**：SRP（种族是定义层属性）、OCP（加分派方法不改内核）、ISP（按类别暴露窄方法）、LoD（装配层经 `Race` 查表、不直接触内核）。

### D4 开局装配只扩 `def_ids`，`Spawn` 字节逐字节不变

**决策**：`setup.rs` 的 `MeleeSetup::with_race` 扩人族开局 `def_ids` 关联到新增类别（或新增装配期绑定，由实现定）；`CommandKind::Spawn` 的字段与字节 MUST 保持 P8.0 现状（`command.rs:14-21` 不含 `DefId`），两族 `Spawn` 字节一致（`setup.rs:684-700` 已证）。规则依据：R4.1 主城 / R1.4 可移动性派生 `[已核]`、装配层关联 `[取舍]`。

**备选方案**：
1. 给 `CommandKind::Spawn` 加 `DefId` 字段 —— 否决：C2 / R2；改内核命令线格式、状态哈希与快照，两族 `Spawn` 字节不再一致。
2. 在运行时视图里补 `DefId` —— 否决：MUST NOT 扩展 `WorldView` / `EntityView`（C4 / P8.0 D1），且会滑向改 `w3-core`。（注：该「不扩展视图 / 不用运行时 speed」为 P8.0 / P8.1 阶段性实现层约束，2026-11-16 经用户拍板由 change `runtime-entity-movability` 放松为只读视图分层判据；本条历史决策记录保留）
3. 不做装配关联、靠 DefId 数值约定 —— 否决：AI 脚本 / 端到端脚本需要稳定 `EntityId→DefId` 查表，散落约定不可验证。

**理由**：`MeleeSetup` 的 `def_ids`（`setup.rs:152-156`）已是「种族维度只停在这一层」的既定接缝，扩展它不影响任何权威命令。

**ADR 0002 / 跨平台确定性影响**：`Spawn` 命令字节、命令排序键、状态哈希与快照完全不变；`def_ids` 是表现层只读关联、不进哈希 / 快照。两族逐帧哈希与基线一致（`melee_loop.rs:544-560` 已有先例）。

**架构原则对照**：DIP（装配层依赖只读定义）、唯一状态所有者（装配关联不反写内核）、CARP（复用既有 `MeleeSetup`）。

### D5 AI 脚本扩展沿用既有玩法命令，帧冻结外部命令

**决策**：`AiScript::new` 在既有 `Gather` / `Train` / 周期性 `Cast` 骚扰（`ai.rs:30-85`）之外，增加训练新单位、以英雄实体施放主动技能、研发升级三类命令，全部为 `GameplayCommandKind` 既有变体、带固定 `ExecuteFrame` / `PlayerId` / `Sequence`（R12.7 `[已核]`、R4.3 / R4.4 / R6 `[已核]`、帧冻结 `[取舍]`）。**MUST NOT** 扩展 `w3-core::run_ai`、MUST NOT 新增权威状态、MUST NOT 旁路写入。

**备选方案**：
1. 改 `w3-core::run_ai` 按种族产出命令 —— 否决：改内核 AI 路径与权威状态，且 AI × lockstep 接合架构（`WC3-MELEE-RULES.md` §0.3 T5）未拍板，须另走 ADR 0002 / 0005 流程。
2. 用墙钟 / 随机数驱动 AI 决策 —— 否决：破坏 30 Hz 整数帧与重放确定性（ADR 0002 §2 / §4）。
3. 让 AI 直接改玩家经济 / 单位字段 —— 否决：旁路写入权威状态、破坏唯一状态所有者与 `first_difference()`。

**理由**：`AiScript` 已是「确定性外部命令源（`ai.rs:1-6` 注释）」；`MeleeDriver::new`（`driver.rs:35-47`）一次性入队并按帧冻结执行，与刷新率 / 卡顿无关（`melee_loop.rs:419-441` 已证）。扩命令种类不改机制。

**ADR 0002 / 跨平台确定性影响**：命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、外部重放注入同一脚本即可复现；不新增权威状态、不改哈希 / 快照。跨平台无差异。

**架构原则对照**：SRP（AI 只产命令）、DIP（经既有 `submit_gameplay_command`）、显式失败（命令被拒时英文日志告警、不静默降级）。

### D6 CPU 端到端一局覆盖全阶段 + 逐帧哈希基线 + 反向验证

**决策**：扩 `crates/w3-client/tests/melee_loop.rs` 的既有 `play_full_match` 或新增测试，按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开」逐阶段断言（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5 `[已核]`、`[取舍]` 用合成数值让一局在合理帧数内打完），并断言两实例逐帧 `compute_state_hash()` / `capture_snapshot()` 与基线一致（ADR 0002）。新增 / 修改玩法回归 MUST 做**反向验证（改坏验红）**：以「测试内反例元断言」（退化过滤器 / 漏接 `defs_hash` 敏感性 / 把建筑 `speed_raw` 改非 0）或 A-B 对照证明缺陷态失败、修复态通过（C5 / `CODEBUDDY.md` §5）。

**备选方案**：
1. 只加正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，不构成反向验证。
2. 人工改坏代码验红 —— 否决：不可复现、不进 CI（P8.0 D3 备选 2）。
3. 只比最终 hash、不比逐帧 —— 否决：`golden-hash` 复核记录（ADR 0002）要求逐帧文本 / 字节比对，最终 hash 相同可能掩盖中间帧 desync。

**理由**：`melee_loop.rs:446-560` 已有 `play_full_match` 与两族逐帧哈希基线，扩阶段比另起炉灶更稳；`defs.rs:337-366` 的 `defs_hash_is_fixed_and_content_sensitive` 已是「反例元断言」先例。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图与命令、不触碰 `w3-core` 状态；仍包含重跑 `cargo test -p w3-core --test determinism`（26 项）任务以确认零影响。不得据 CPU 全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（每阶段断言、不留静默）、CARP（复用既有 driver / setup）。

### D7 R1.4 / R13.6 过滤沿用 P8.0 只读映射，不重开内核口子

**决策**：新增建筑 `speed_raw == 0` 由 `MeleeSetup::movability`（`setup.rs:434-436`）经 `derive_movability` 派生为表现层只读映射；含新增建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`（R1.4 / R13.6 `[已核]`、派生映射 `[取舍]` / P8.0 D1）。MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`、MUST NOT 回退建筑类型白名单（C4）。（注：该「不扩展视图 / 不用运行时 speed」为 P8.0 / P8.1 阶段性实现层约束，2026-11-16 经用户拍板由 change `runtime-entity-movability` 放松为只读视图分层判据；本条历史决策记录保留）

**备选方案**：
1. 在 `w3-core` 收到 `Move` 时丢弃不可移动实体 —— 否决：改内核权威路径、要动状态哈希与快照（ADR 0002 / P8.0 D1 备选 1）。
2. 对每个新增建筑在输入层硬编码排除 —— 否决：违反「判据读定义层、不写死类型」（C4），新增 6 类就要改表。
3. 用 `EntityKind::is_resource()` 等标签判 —— 否决：`EntityKind`（`setup.rs:80-123`）无可移动性方法，且标签是表现层模型键、不是可移动性权威来源。

**理由**：P8.0 已把判据接线闭合（`derive_movability` 读 `speed_raw`），本 change 只需保证新定义遵循 `speed_raw == 0` 约定即可自动被过滤，零接线改动。

**ADR 0002 / 跨平台确定性影响**：过滤只在表现层输入翻译、`CommandKind::Move` 线格式 / 排序键 / 哈希 / 快照全部不变；跨平台生成同一命令序列。**P8.0 已知限制（运行时 `Train` / `Build` 新实体不在装配期 `def_ids` 关联中）本 change 不擅自闭合**，登记为待确认项由人拍板（见 `decision-memo.md` / Risks）。

**架构原则对照**：SRP（过滤只在输入翻译一处）、DIP（只读冻结定义）、LSP（新 `UnitDef` 与既有定义对同一派生函数行为一致）、显式失败（全不可移动给反馈）。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」（`:67-88`）与 `openspec/config.yaml:64` 的 `rules.design`。本 change 命中 **T1**（确定性内核 / 状态哈希 / 命令排序，C1/C3 需要）、**T2**（移动 / 编队 / 战斗结算语义，R5 / R1.4）、**T4**（玩法语义 / 技能 / 单位行为，P8.1 重点）、**T5**（场景装配 / Melee 初始化，开局 `def_ids`）。四条结论由只读 `reference-researcher` 出报告、`spec-steward` 成文前**逐锚打开复核**（本轮已实际打开，见各条锚）。四个参考项目只读、不复制代码（ADR 0001）；本机绝对路径见 `UPSTREAM.md:9-14` 固定版本表，不在本文重复。许可证：`vera20k` GPLv3 + Section 7（仅复制代码时附加条款才进入）、`godot_warcraft3` / `rts_kernel_cs` MIT、`WarsmashModEngine` **AGPLv3 仅借鉴语义、不得复制任何代码**。资产与派生文件一律不入库（ADR 0001）。参考缺口按 `UPSTREAM.md:88` 触发时机第 5 条回到人拍板，不臆造。

- **T1 确定性内核 / 状态哈希 / 命令排序 → `vera20k`（GPLv3 + Section 7）**：锚 `vera20k/src/net/lockstep.rs:35-39`（`CHECKSUM_HISTORY_LEN` / `MultiplayerChecksumHistory` / `ChecksumMismatch` 等纯命令传输导出）、`:648`（`checksum_history.compare(frame_info)?` 逐帧 checksum 对比）。**借鉴**：lockstep 逐帧 checksum 的 desync 检测思路、命令传输不携带游戏规则（呼应「表现层只传输入命令、不反写内核」）。**不借鉴 / 边界**：RA2 专属 gamemd 地址 / INI / SHP / VXL / 2D 等距实现不借；GPLv3 + Section 7 附加条款仅在真的复制代码时进入（ADR 0001），本 change 不复制。**归因纠正（重要）**：「新增权威状态字段必须同时进入状态哈希与快照」这条硬门禁的出处是**本项目** `CODEBUDDY.md` §5、`docs/decisions/0002-determinism.md:55-72`（ADR 0002 §6/§8）与主 `openspec/specs/melee-match/spec.md:127-129`，**不是 vera20k**；C1 与 D1/D3 否决 `Player.race` 的依据指向这三者，不得冒充参考结论。本 change 新增 `UnitDef` / `AbilityDef` / `UpgradeDef` 是**冻结定义内容变化**（`compute_defs_hash` 覆盖四类，`gameplay.rs:144-181`），`w3-core` 可零 diff；会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）本 change 不做、属 P8.5，MUST 另走 ADR 0002 流程，不得借「零改动」旁路。

- **T2 移动 / 编队 / 战斗结算语义 → `rts_kernel_cs`（MIT）+ `godot_warcraft3`（MIT）**：锚 `rts_kernel_cs/docs/MOVEMENT.md:73-79`（「Group placement and formations」、`:77` "stable ideal slots using deterministic minimum squared-distance matching"、`:79` 未变的 distance/Y/X 枚举顺序）、`rts_kernel_cs/docs/FRAMEWORK.md:109`（明确「不纳入：…战斗 / 经济 / 技能迁移」）；`godot_warcraft3/docs/design/game/COMBAT_SYSTEM.md:48-52`（`ATTACK` / `ATTACK_MOVE` / `HOLD` 索敌 - 追击分类表）、`:365-373`（`atk_type1`: normal / pierce / siege / magic / chaos / hero 攻击类型字段、`range_n1` / `min_range`）、`:411-423`（敌对关系与 `is_valid_attack_target` 过滤）、`godot_warcraft3/docs/design/game/UNIT_AI.md:198-210`（`PASSIVE` / `CAMP_CREEP` / `TEAM_PLAYER` / `REACTIVE` profile 与 idle 索敌 / 受击反击优先级）。**借鉴**：「前排 / 远程 / 反重甲」的**语义分类**与稳定 ID 落点 / 稳定集合枚举顺序，支撑 R5.1 / R5.3 的新单位分类与 R1.4 落点分配。**不借鉴 / 边界**：战斗结算 MUST 走 `w3-core` 既有 `Attack` / `Cast`（`command.rs:34-37` / `gameplay.rs:540-544`），**不移植** godot `DamagePipeline`（`COMBAT_SYSTEM.md:369`）/`CombatRng`（`:409`）或 Godot / GDScript 实现；rts_kernel_cs 的 `double` 数值路线与「不承诺跨平台确定性」（`README.zh.md:7,21`）**不采用**（ADR 0002）；其 C# 内核内 `GroupMove` 语义不复制 —— 本项目的可移动性过滤在表现层输入翻译层（D7）；`FRAMEWORK.md:109` 明确把战斗 / 经济 / 技能迁移排除在该内核契约之外，故 R5 战斗结算的**实现依据仍是本项目 `w3-core` + 业务规则 R5**，不是 rts_kernel_cs。

- **T4 玩法语义 / 技能 / 单位行为 → `WarsmashModEngine`（AGPLv3，仅语义）+ `godot_warcraft3`（MIT）**：锚 `WarsmashModEngine/core/assets/abilityBehaviors/humanUnitActives.json:2-5`（`abilityList` / `ids: Aslo` / `NORMAL_UNITTARGET` / `extraAutoTargetConditions` 数据行）、`humanHeroActives.json:2-5`（`abilityList` / `ids: AHfs` / `NORMAL_POINTTARGET` / `onBeginCasting`）、`WarsmashModEngine/core/src/com/etheller/warsmash/units/manager/MutableObjectData.java:411 / :423 / :425 / :428`（`urac` race / `arac` ability race / `frac` buff race / `grac` upgrade race 分类字段，逐行已核）、`WarsmashModEngine/resources/Scripts/Abilities/Human/Archmage/SummonWaterElemental.j:1-38`（`populateData` 从 editorData 读 `summonUnitId` / `summonUnitCount` / `buffId`、`doEffect` 执行效果、`initializer register` 注册结构 —— 数据 / 效果分离）；`godot_warcraft3/docs/roadmap/NEXT.md:149-155`（工程纪律：`:152`「数值读 SLK Def，禁止逻辑里写死造价 / 伤害」、`:153`「新技能优先拼 Effect + 注册 Behavior」）、`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:11-29`（`:13` **Data → Logic → Present** 分层、`:11` thin 层 `Order + Cooldown + Mana + Behavior`）、`godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md:3 / :52`（`:3`「未锁定具体经典补丁；不能宣称当前数值与某一完整原版版本一致」、`:52`「不能把数据行完整性等同于四族玩法完成」）、`godot_warcraft3/docs/design/game/FACTION_UNIT_ROSTERS.md:1-9 / :205-230`（`:1-5` **原创三族**且「名称 / 造价 / 数值是种子、不是最终平衡承诺」、`:205-230` U0–U3 灰盒批次、`:230`「每族至少完成两种开局和两种主力组合」）。**借鉴**：**只借「按种族 / 英雄分组组织数据」的结构**（race 字段分组、`abilityList` 数据行、技能注册结构与 `Data → Logic → Present` 分层），以及「先核来源再写数值 / 数据行完整 ≠ 玩法完成」的工程纪律（支撑 D2 用既有 `EffectDef` 组合、数值 `[待核]`）。**不借鉴 / 边界**：**不借单位名 / rawcode / 数值** —— `humanUnitActives.json` 的 `Aslo`、`humanHeroActives.json` 的 `AHfs`、`GAMEPLAY_VERTICAL.md:39-49` 的 `Hamg` / `hfoo` / `hlum` / `hrif`、`melee_race_preview.gd:51-78` 的 `htow` / `hpea` 均为暴雪 rawcode，一律不借（C6 / `UPSTREAM.md:79` 矩阵 T5 边界）；`FACTION_UNIT_ROSTERS.md` 是**原创三族**世界观、不是经典人族 / 兽族复刻，只借「分批灰盒 / 结束比赛判据」的排期结构、不作本 change 验收判据；`SummonWaterElemental.j` 是 JASS 脚本，**AGPLv3 只借鉴设计语义、不得复制任何代码**，不引入 Java / LibGDX 依赖，若结论需要代码 MUST 按 ADR 0001 复核点重新评估许可证；NEXT.md 的 `Behavior` 注册 / `CombatRng` 不移植（本 change 不新增 `EffectKind` / 命令变体 / JASS VM，见 D2）。

- **T5 场景装配 / Melee 初始化 → `godot_warcraft3`（MIT）**：锚 `godot_warcraft3/docs/design/game/ROADMAP.md:10-24`（`:12`「WE 对战初始化本质是一张**固定动作表**」、`:14-21` 8 条动作：初始资源 / 创建开局单位 / AI 脚本 / 胜负条件等、`:23` 先做 `MeleeBootstrap` 硬编码模块）、`:74-92`（「C. Melee Bootstrap」动作表，`:81`「创建对战初始单位（主城 + 工人于 sloc，**需种族→单位 ID 表**）」、`:88-90` 输出「要生成 / 删除的单位指令」且「**不**解析 wtg」）、`godot_warcraft3/packages/gameplay/match/melee_bootstrap.gd:27-77 / :41-43`（`spawn_at_sloc(... race ...)` 经 `MeleeRacePreview.town_hall_id(race)` / `worker_id(race)` 查表生成开局单位）、`godot_warcraft3/packages/gameplay/catalog/melee_race_preview.gd:51-78`（`town_hall_id` / `worker_id` 按 race 返回 typeId 的查表结构）。**借鉴**：「动作表 + 种族→单位 ID 查表 + 输出指令、不解析 wtg」的**装配结构**（支撑 D3 `Race` 分派与 D4 `def_ids` 装配）。**不借鉴 / 边界**：不借 Godot / GDScript 实现、字符串 typeId（本项目是编译期整数 `DefId`）、运行时 `GameSession.local_race` 会话状态（与本项目「不新增权威状态」正交，`UPSTREAM.md:79`）；**`CommandKind::Spawn` 字节不变**（P8.0 原创，锚 `crates/w3-core/src/command.rs:14-21` + `setup.rs:684-700`），godot 的「输出单位指令」是编辑器 / 表现层语义，不等于本项目权威命令字节；`melee_race_preview.gd` 返回的 `htow` / `hpea` 是暴雪 rawcode、不借（C6）。

- **锚点校正（如实登记）**：① `godot_warcraft3` 固定提交内**无源码级物理 `Human/` 目录** —— 本轮实际 `find` 核实：`Human/` 目录只存在于 `assets/.staging/wc3-assets/...` 与 `assets/asset-converted/...`，且 `git check-ignore` 确认 `assets/asset-converted/Units/Human` 被忽略（MPQ 派生资产、非源码、不入库）；可核验的等价物是 F0–F10 竖切文档 `godot_warcraft3/docs/design/game/ROADMAP.md:103-123`（`:103`「F. 人族游玩竖切」、`:112-123` F0–F10 表）、`GAMEPLAY_VERTICAL.md:33-56`（`:33-54` F0–F10 竖切总览）、`packages/gameplay/features/build/logic/human_construction_strategy.gd:1-16`（`:1` `class_name HumanConstructionStrategy`、`:4`「F2-B Human ConstructionStrategy」）。② `WarsmashModEngine/resources/Scripts/melee.jui` 实为 **JASS UI 帧定义**（本轮打开首段为 `globals` / `SKIN = "NightElf"` / `framehandle ROOT_FRAME` / `RESOURCE_BAR_*` 等），**不是**对战初始化动作表；该语义见 `UPSTREAM.md:78` 矩阵 T4 行与 `docs/roadmap/P8-human-orc-melee.md:62`。Melee 初始化动作表语义只在 godot `ROADMAP.md` §0 / §2-C，不在 Warsmash 固定提交内。③ T4 的 Warsmash `abilityBehaviors/` 按种族命名**分组不对称**（`UPSTREAM.md:78`：「兽族仅 `orcHeroActives.json`」），只借分组结构、不借分组数量或命名。

- **参考缺口（如实登记、回到人拍板、不臆造）**：以下四项在四个参考项目中**均无现成做法**，属本项目在 ADR 0002 / 主 `melee-match/spec.md:127-129` / ADR 0001 零资产约束下的**原创取舍**，不冒充参考结论：① 编译期整数 `DefId` 按种族分段且**不引入任何运行时权威状态**（Warsmash `urac` / godot `melee_race_preview` 都带运行时会话 / 字符串 typeId 语义）；② 「1 英雄 + 1 主动技能」最小集用既有 `EffectDef` 组合表达（godot F3/F10 走完整技能系统、Warsmash 走 JASS）；③ `Spawn` 不含 `DefId`、用装配层 `def_ids` 关联承载种族（P8.0 原创，`command.rs:14-21`）；④ CPU 无渲染逐帧哈希「一局」验收脚本（`melee_loop.rs:446-560` 本项目已有先例）。**这些缺口待规格门由人拍板**（`decision-memo.md` 的 `status: pending`），AI 不自行放行。

## Risks / Trade-offs

- **新增定义数量与 `defs_hash` 变化** → `compute_defs_hash`（`gameplay.rs:144-181`）对四类定义的条目数与每字段敏感，新增 6 `UnitDef` + 1 `AbilityDef` + 1–2 `UpgradeDef` 必然改变 `content_hash` / `defs_hash`；这是**预期内的冻结定义内容变化**、不是权威状态字段新增（C3）。缓解：D6 的端到端测试比对「相同脚本两实例逐帧哈希一致」而非与改动前的旧基线数值相等；`defs.rs:337-366` 的 `defs_hash_is_fixed_and_content_sensitive` 元断言复核内容敏感性，快照头部 `defs_hash` / `content_hash` 随之更新、`SNAPSHOT_VERSION` 仍 3。
- **`speed_raw == 0` 语义** → 作为可移动性判据是 P8.0 约定；新增 3 类建筑 MUST 一致传 `speed == 0`，否则 `derive_movability` 误判、R1.4 bug 从新建筑路径复发。未来「可移动但速度为 0」（被定身）须在定义层显式区分、不能靠运行时状态改写（P8.0 `design.md` Risks 已登记）。缓解：D1 / D7 与 `unit-orders` delta 的 `speed_raw` 派生 Scenario + 反向验证。
- **P8.1 不碰权威状态的边界** → 本 change 的「零改动 / `SNAPSHOT_VERSION` 仍 3」**仅适用于**复用既有 `Cast` / `Train` / `Research` / `Attack` / `Move` 路径 + 定义层内容扩展 + R1.4 过滤。英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体属 P8.5，会新增权威状态或命令面，MUST 另走 ADR 0002（状态哈希 + 快照 + `first_difference()` + determinism），不得借本条「零改动」旁路（`WC3-MELEE-RULES.md:304` R13.6 范围限定）。
- **运行时动态实体承接** → P8.0 已知限制（B1）：`CommandKind::Spawn` 不含 `DefId`、运行时 `Train` / `Build` 新实体不在装配期 `def_ids` 关联中，其可移动性在「内核零改动 + 不扩展视图」下无法判定。本 change 新增大量可训练 / 可建造实体，**不擅自改 `unwrap_or` 语义或扩视图**，登记为 `decision-memo.md` 待确认项由人拍板、由承接 change 处理；CPU 端到端脚本对 frame-0 静态集合与训练后实体的验证边界如实标注。（注：该「不扩展视图 / 不用运行时 speed」为 P8.0 / P8.1 阶段性实现层约束，2026-11-16 经用户拍板由 change `runtime-entity-movability` 放松为只读视图分层判据并完成承接；本条历史决策记录保留）
- **数值 `[待核]` 不冒充原版** → 6 类新定义的数值全为合成原创常量，R5.3 克制倍率 / R6 经验 / R3.4 人口上限等具体数值 `[待核]`，以本机导出 SLK 核对、不入库；`CLASSIC_RULES_BASELINE.md:52` 明确「数据行完整 ≠ 玩法完成」，本 change 的验收线是「一局能按各阶段打完且逐帧哈希一致」，不是「数值对齐原版」。
- **`file:line` 锚漂移** → 本文与 `decision-memo.md` 的 `file:line` 锚在成文前逐条打开核实；apply 后实现代码行号会漂移，按 `CODEBUDDY.md:107` 规格锁定与重审规则：四件套任一 artifact 改动即视为解锁，MUST 同步回查其余三份并重走规格门，不得只改一份。
- **范围外承接** → 兽族 / 双族对抗 / HUD / P8.5 五个深度玩法由后续独立 change 承接（C8），本 change 不因「顺手」扩范围；联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项（C7）。
