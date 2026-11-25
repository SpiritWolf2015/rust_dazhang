# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「怎么实现」。当前 `w3-core` 技能层现状（已逐锚复核）：`EffectKind` 仅 4 变体（`gameplay.rs:283-288`）、`effect_kind_code` 仅码 0–3（`:416-421`）、`AbilityDef` = 模板参数 + `EffectDef` 组合 + 注册行为 ID（`:312-318`）、`Cast { caster, ability, target: EntityId }` 仅单体实体目标（`:1323-1327`）、`execute_cast` 一次性结算（`sim_match.rs:1316-1358`）、`Vitals` 无坐标 / 范围 / 随机 / 召唤槽（`:842-849`）、`Settlement` 只有 damage / heal / buff / stat_mod / filter_targets（`:851-941`）、`BuffInstance` 无分类标签（`:950-954`）、`AuraState` 无半径 / 受益集 / 订阅（`:1295-1299`）、`WorldGameplayState` 的 auras / behaviors（`:1983-1991`）、`resolve_effects` 链尾追加一条 `extra_effect`（`:2090-2105`）。这四类原语不足以表达经典技能原语（对照结论 G4）。

版本号现状：`SNAPSHOT_VERSION = SIM_VERSION = 8`（`snapshot.rs:56-57`）、`PROTOCOL_VERSION = 8`（`packet.rs:45`）；`8→9` 是 P11 **待执行动作、非既成事实**。四处同步在 `sim_match.rs:3784`（`compute_state_hash`）/ `:3861`（`first_difference`）/ `:3941`（`capture_snapshot`）/ `:4038`（`restore_snapshot`），定义层在 `gameplay.rs:434`（`compute_defs_hash`）/ `:1994-2059`（`WorldGameplayState::hash`）/ `:2061-2077`（其 `first_difference`）/ `:4043-4064`（restore 五重校验）/ `:2318`（掉落表 EffectDef 段进定义哈希）。golden hash v8 值 `0x0b53a047b1f8e5e0`（`golden_hash.rs:75-77` / `0002-determinism.md:118`）**不沿用**。

命令面事实订正（结论 G8）：`GameplayCommandKind` 全 36 变体（`gameplay.rs:1322-1546`）**确无 `Attack`、确无 `Build`**；建造 = `Train { producer, unit_def, is_building: bool }`（`:1328-1332` / `sim_match.rs:1360-1389`）；攻击 / 移动 / 停止 = `CommandKind::{Attack, Move, Stop}`（`command.rs:11-49`，排序键 `:3`）。本 change 施法走 `GameplayCommandKind::Cast`，MUST NOT 写成 `GameplayCommandKind::Attack` / `::Build`（不存在，`critic.md` 清单第 10 项①会挑）。

**关键约束取代**：`crates/w3-client/src/melee/defs.rs:794-797` 的「只用既有 `EffectKind` + `EffectDef` 组合…MUST NOT 新增 `EffectKind` 变体 / 命令变体 / JASS VM（N5）」是 **P8.1 当期 change 内约束、不是永久门禁**；P11 已由用户 2026-11-24 拍板「四族全量 + bump 9 全套」+ Q-B 立项顺序、规格门已逐条拍板定案，扩展 `EffectKind` / 命令变体属本期授权范围。此约束在本 change 被取代及依据见 D1；JASS VM（N5）仍不引入、不随之解禁。

参考对照命中 `UPSTREAM.md:106-112` 矩阵 **T4**（WarsmashModEngine）/ **T3**（godot_warcraft3）/ **T1**（vera20k），可选交叉 **T5**（godot 场景 / Present）、**T7**（ConcordRTS 仅作 T1 的 C++ 实现对照）。**T2 不命中**：P11 不改移动 / 寻路 / 帧阶段，`rts_kernel_cs` 的 `double` 路线被 ADR 0002 否决（`0002-determinism.md:15,18,78`）。**T6 不命中**：不改 W3N1 帧头 / UDP 可靠层 / relay 重传，版本号与 proto tag 属 ADR 0002/0005（`0005-protobuf-payload.md:23-27,43`）。

## 清单规模事实锚（本机导出表规模元数据，leader 已只读核对）

> 本节只借清单规模作事实锚，**只登记规模与分类**，不登记任何具体技能名 / 四字符 rawcode / 数值；导出数据留在玩家本机、由 gitignored 的 `config.toml` 提供路径、不入库（ADR 0001）。规模元数据为**本机导出表的规模统计**（仓库文件外证据、人工确认项），不是参考项目结论、不作 WC3 玩法事实来源；清单逐项固化由 apply 阶段人工核对、数值 `[待核]`、不静默改规则（`WC3-MELEE-RULES.md §14`）。

- `UnitAbilities.json` **836** 条（列 `abilList` / `heroAbilList` / `unitAbilID` / `auto` / `sortAbil` / `InBeta`）：`heroAbilList` 非空 **85** 单位、英雄技能 ID 合计 **344** 条；`abilList` 非空 **586** 单位、单位技能 ID 合计 **1139** 条。
- `AbilityData.json` **803** 条 / **91** 列（含 `code` / `race` / `hero` / `levels` / `reqLevel`）；按 `race` 分 human **52** / orc **69** / undead **70** / nightelf **72**（四族合计 **263**）、creeps **236** / other **289** / naga **11** / None **4**。
- `AbilityBuffData.json` **237** 条。
- 以上计数一律标注为「按逗号 / 分号分隔 ID 的**粗统计、以人工核对为准**」；**不臆造「四族全量 = 某精确数字」**，实际清单条目数在 apply 阶段核对后如实登记。「每族 4 英雄 × 4 技能」是经典清单结构口径（R6 / `WC3-MELEE-RULES.md:163-171`）、不是从上述粗统计推得的精确条目数。
- 顺延项（中立英雄 / 野怪 / 全部单位技能）见本文件「## Risks / Trade-offs」、`decision-memo.md`「已知限制」、`tasks.md` 如实登记。
- 与 Q1 的关系：本节为 D1「清单按本机导出表逐项对齐」提供规模事实锚，MUST NOT 被读成「照搬经典具名技能名称 / rawcode / 数值」的授权；内部标识仍合成原创、SLK 四字符 ID 只作核对依据、不进权威状态 / 状态哈希 / 快照 / 不入库（见 D1 / D2）。

## Goals / Non-Goals

**Goals:**
- 四族技能清单按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）逐项对齐并人工固化成编译期常量表（内部仍用合成原创 `DefId` 作键、不引入四字符 ID → `DefId` 映射层），本期范围 = 四族英雄技能全量 + 各族代表性单位 / 道具技能，建立「全量能力 + 行为原语注册表 + 权威运行态 + 确定性验收」；中立英雄 / 野怪 / 全部单位技能顺延标已知限制。
- 补齐经典技能原语并落在单一 `BehaviorCatalog` 注册表（OCP），每个新增权威运行态都进哈希 / 快照 / `first_difference` / restore。
- 完成 ADR 0002 的 bump 9 全套与 ADR 0005 的 proto 版本纪律，守确定性契约与表现层只读边界。

**Non-Goals:**
- 不做 N2 地图编辑器 / N3 RPG 自定义地图 / N5 JASS·wtg 触发器 VM（`WC3-MELEE-RULES.md:43-46` / `0003-scope.md:15,38`）；不做 P10 被否的外部 AiScript 经 relay 注入 / 双模式可切换 / AI 决策作为独立命令注入帧流。
- 本期不含 AI 自主施法；AI 全量行为顺延 P12、基于 v9 再评估（Q7 已拍板）。不改 `AiScript` 现有取 `Cast` 目标 ID 的只读夹具。
- 不引入 Warsmash Java / LibGDX 代码或 `*.j` / `.jui` / `jassparser/` 代码；不引入 ConcordRTS 的 RA2/YR 数值 / 规则；不把原创三族 / 合成原创技能冒充经典具名技能（按 Q1 两层口径：清单条目与分类逐项对齐经典具名技能 ≠ 照搬经典名称 / rawcode / 数值）；不把本机导出的具体技能名 / 四字符 rawcode / 数值写进四件套或仓库；不把 README 自述当已验证事实。
- 本期**不改活文档** `docs/references/WC3-MELEE-RULES.md`（Q-A 由 docs-owner 在规格门批准记录回填后的 apply 阶段落定，见 D9）；不改 `crates/**`；本期顺延项（中立英雄 / 野怪 / 全部单位技能）如实标已知限制、不扩范围；不 archive / 不 commit / 不 push / 不 self-approve 规格门。

## Decisions

> 每个决策统一给出：决策（**已由用户 2026-11-24 规格门逐条拍板定案**；Q1 改按备选「SLK 清单逐项对齐」、Q1↔Q2 调和「清单对齐 SLK、标识仍合成」、Q2 / Q3 / Q4 / Q6 / Q7 / Q8 / Q10 按推荐、Q5 三者同步 8→9、Q9 / Q-A 已拍板流程约束）、备选方案（alternatives considered，含被否理由）、依据（`R-x.y` + 核实状态 `[已核]` / `[待核]` / `[取舍]` + ADR）、代价、对跨平台确定性的影响（ADR 0002）、分层依赖 / 架构原则（SRP / OCP / DIP）。`[待核]` 数值一律不冻结进权威定义、回到人拍板。`critic` 规格门报告 9 条 B 类 residual 按用户拍板「批准前修订再锁定」逐条吸收（R-1 / R-2 / R-3 / R-5 落本文件，R-4 / R-6 / R-9 / R-10 落 `decision-memo.md`，R-4 与 R-7 另落 `tasks.md` / `specs/`）。

### D0 只写 ADDED delta、不写 MODIFIED delta（历史主 spec 口径）

- **决策**：新增单一 capability `ability-system`，不写 `deterministic-simulation` / `gameplay-command-wire` 的 MODIFIED delta。
- **备选方案**：① 把 `deterministic-simulation/spec.md:44` 的「本期零新增权威状态字段、保持 8 不 bump」改写为 bump 9 口径（被否：那是当期 change 实施期口径；**OpenSpec 主 spec 在 archive 阶段合并、提案阶段不直接编辑；历期同类残留按 P9 Q9' 已知限制收口、由后续独立 change 统一收口**。「改写会与已锁定指纹冲突」不成立——依据 `tools/verify_all.py:182-186`：四件套指纹集合仅 change 目录内 `proposal.md` / `design.md` / `specs/**/*.md` / `tasks.md`，**不含主 `openspec/specs/**`**）；② 写 MODIFIED delta 但只改局部（被否：OpenSpec 要求复制整块 requirement，局部改写会在归档时丢细节）；③ 写完整 MODIFIED delta（复制整块 requirement + 逐字保留全部历史 `#### Scenario:` 身份键 + `>` 身份键说明）——历期实际采用的做法（先例 `archive/2026-11-23-p9-undead-foundation/tasks.md:48`、`archive/2026-11-24-p10-ai-player-lockstep/specs/deterministic-simulation/spec.md` 头部），**考虑过、因本 change 只新增 capability、不改既有 requirement 的行为契约而不取**：P11 只对 `ability-system` 写 ADDED delta、单一 change 单一份 delta 即可表达「四族全量 + bump 9 全套」，写 MODIFIED delta 会令主 spec `:44` 历史口径与本期新增 capability 的 ADDED requirement 并列、徒增归档合并冲突面，且 P11 授权不依赖该转义条款（见下）。
- **依据**：`deterministic-simulation/spec.md:44` 已自带转义条款「若 Q1 拍板确需新增字段 MUST 四处同步并按 ADR 0002 bump 8→9 全套」；`command-hotkeys/spec.md:34` 先例「历史『MUST NOT 新增变体』是 change 的实施期约束、不代表禁止后续 change 按用户拍板改内核」；`faction-definitions/spec.md:26`「删除历史断言…是各期实施期口径、不是耐久禁令」。`[已核]` / R12.4 / ADR 0002。**编号复用澄清（R-3）**：此处 Q1 为 **P10 规格门 Q1**（版本与权威状态处置），与 P11 Q1（技能清单来源）编号复用含义不同；P11 授权来自用户 2026-11-24 拍板 P4『四族全量 + bump 9 全套』，**不依赖该转义条款**。
- **代价**：归档后主 spec `:44` 仍留「保持 8」历史字面，靠本 capability 的 ADDED requirement 与身份键说明消解；`critic` 第 7 / 10 项须复核两者不打架；归档后主 spec 其它无条件 `==8` 硬断言字面按 P9 Q9' 已知限制、由后续独立 change 收口（见 `decision-memo.md`「已知限制」与 `tasks.md` 7.10）。
- **跨平台确定性影响**：无（纯 spec 组织决策，不触权威状态）。
- **分层依赖 / 架构原则**：SRP（一个 capability 只拥有技能系统及其确定性契约）、避免重复定义。

### D1（Q1）四族全量的清单来源与范围

- **决策（用户 2026-11-24 规格门改拍备选、已定案）**：**按本机导出 SLK 经典具名清单逐项对齐**——四族技能**清单**逐项按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）核对后，**人工固化成编译期常量表**进 `FrozenDefinitions`；本期范围 = 四族英雄技能全量（经典清单结构每族 4 英雄 × 4 技能 = 英雄 3 普通 + 1 个 6 级大招，含技能点 / 等级 / 学习 / 升级）+ 各族代表性单位 / 道具技能（道具技能经既有 `EffectDef` 组合走 R7.6），中立英雄 / 野怪 / 全部单位技能顺延、标已知限制。内部仍用合成原创 `DefId` 作键（沿用 `defs.rs:120,122,124,279,281` 的 `ABILITY=10` / 四族英雄 `11–14` 并扩四族）、**不引入「四字符 ID / 具名技能名 → `DefId`」映射层**（Q1↔Q2 调和口径），数值一律 `[待核]`、名称合成原创不冒充经典具名技能、SLK 四字符 ID 只作核对依据、**不进权威状态 / 状态哈希 / 快照 / 不入库**（`WC3-MELEE-RULES.md:124,351`、ADR 0001 / 0002 / 0004、`crates/w3-client/src/melee/defs.rs:1-6` 纯编译期零文件 IO）。建立「技能系统全量能力 + 行为原语注册表 + 权威运行态 + 确定性验收」。清单规模事实锚见本文件「## 清单规模事实锚」；清单逐项固化由 apply 阶段人工核对（数值 `[待核]`、不静默改规则，`WC3-MELEE-RULES.md §14`），实际清单条目数核对后如实登记、不臆造「四族全量 = 某精确数字」。
- **两层张力（必须写清、避免 `critic` 判自相矛盾）**：「清单结构 / 条目逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」是**两个层次**——对齐的是**清单条目与分类**（哪些技能、每族几个、英雄 3 普通 + 1 大招、代表性单位 / 道具技能），**不是照搬经典名称 / 四字符 rawcode / 数值**；内部标识仍合成原创。原推荐「四族各一套完整原创合成技能体系」**降为被否备选**（理由：用户改选按 SLK 清单逐项对齐、要真实清单结构），原备选「严格按本机导出 SLK 的经典具名清单逐项复刻」**升为决策**、并按 Q1↔Q2 调和口径细化为上述准确表述（清单对齐、标识仍合成）。
- **备选方案（被否）**：四族各一套**完整的原创合成技能体系**（原 Q1【推荐】）——参考项目无逐项权威表、且会失去真实清单结构；用户 2026-11-24 规格门明确改选「按 SLK 清单逐项对齐」，要清单条目 / 分类与经典一致、成本转由 apply 阶段人工核对承担。「若规格门改拍此备选须同步回查四件套」的触发已发生，本轮即按 `CODEBUDDY.md §8` 四份 + `decision-memo.md` 同步回查、一次成文。`critic` 第 7 / 10 项复核两层张力不被读成矛盾。
- **依据**：S3（四族功能做完，`WC3-MELEE-RULES.md:27`）`[已核]`；R6（`:163-171`）`[已核]` 机制 / `[待核]` 数值；R7.6（`:182`）`[取舍]`；R2（`:111`）`[已核]`；`WC3-MELEE-RULES.md:124,351` / §14 `[已核]` 边界；ADR 0001 / 0002 / 0003 / 0004。Q-A 已拍板「S3 下细化英雄 / 单位技能全量、不新增 S 编号」。
- **代价**：清单逐项人工核对与数值 `[待核]` 排期落到 apply 阶段（`content-owner`）；另写全量技能定义表与 CPU 回归；中立英雄 / 野怪 / 全部单位技能顺延标已知限制（见「## Risks / Trade-offs」与 `decision-memo.md`「已知限制」）。**不产出「照搬经典名称 / rawcode / 数值」的承诺**。
- **跨平台确定性影响**：技能清单定义与数值常量为 `FrozenDefinitions` 冻结常量，经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部（`0002-determinism.md:111,115,117` 定义层内容变化、与新增权威运行态严格区分）；全部编译期整数常量、零文件 IO、零 `f64`，SLK 四字符 ID / 导出数据不进权威状态 / 哈希 / 快照 / 仓库，不引入平台差异（ADR 0002 §6 / ADR 0004）。
- **分层依赖 / 架构原则**：DIP（表现层不读 SLK、只消费冻结定义）、SRP（定义层与运行态分离）、OCP（新技能 = 新定义条目而非散落分支）。

### D2（Q2）具名技能 ID / order / 数值映射

- **决策（Q1↔Q2 调和口径，已拍板定案）**：继续用合成原创 `DefId`（沿用 `defs.rs:120,122,124,279,281` 的 `ABILITY=10` / 四族英雄 `11–14` 并扩四族），**不引入**「按具名技能名 / 四字符 ID → DefId」映射层；数值一律 `[待核]`、不冻结进权威定义。**调和口径**：Q1 已拍板「清单逐项对齐 SLK」是本机的**清单条目与分类**对齐（核对依据），但标识仍合成原创、不引入映射层——「清单对齐 SLK、标识仍合成」两件事并存，取真实清单结构而不取经典 rawcode 兼容层。
- **备选方案（被否）**：引入四字符 ID → `DefId` 映射层 —— 会引入一层无本期收益的间接；Q1 已改拍「逐项清单对齐」后经 Q1↔Q2 调和口径明确「清单对齐 SLK、标识仍合成」，仍不重估映射层。
- **依据**：`WC3-MELEE-RULES.md:124,351`（数值以本机导出 SLK 为准、不入库、不进权威状态）/ R6 / R7.6 / R13.3（`WC3-MELEE-RULES.md:287`）`[待核]` 键位 / 数值不锁定、不入库；`defs.rs:1-6,43,54,77,160,183,208,219,325` 自述合成原创、纯编译期零文件 IO；ADR 0001 / ADR 0002 / ADR 0004。
- **代价**：无经典四字符 ID 兼容层；后续若要做地图 / 存档互操作须另立 change。
- **跨平台确定性影响**：`DefId` 为定宽整数、按稳定 ID 顺序进哈希，无字符串比较 / 哈希迭代顺序差异（ADR 0002 §6）。
- **分层依赖 / 架构原则**：ISP（客户端只依赖 `DefId`，不依赖映射表）、避免 DIP 倒置。

### D3（Q3）EffectKind 扩展粒度与行为表形态

- **决策（已拍板定案、按推荐）**：新增**最小必要原语集**——持续 AOE / zone、召唤、光环受益集、被动 proc 订阅、变身 / 状态切换、点目标、buff 分类移除、链接弹射、随机消费；配一个 `BehaviorCatalog` 式注册表（OCP：扩展非散落分支），每个权威运行态都进哈希 / 快照 / `first_difference` / restore。
- **备选方案（被否）**：每个具名技能一个原生行为、无统一注册表 —— 会散落 `if behavior == ...` 分支、违反 OCP 且使穷尽分发 / 哈希四处同步难以收敛（对照 `ABILITY_SYSTEM.md:117-126` 建议用 `AbilityBehaviorCatalog` 替代散落分支）。
- **依据**：R6 / R5（`:151`）`[已核]` 机制 / `[待核]` 数值；R10.3（`:220-225`）`[取舍]`；参照 `godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:7-14,16-60,117-126,214-219` 的 Data / Logic / Present 分层与 catalog 注册表（只借结构、不复制 GDScript，MIT）；`WarsmashModEngine/core/assets/abilityBehaviors/*.json`（15 个，按种族 / 英雄分组、兽族仅 `orcHeroActives.json`、分组不对称）只借行为分类语义（AGPLv3，不复制）；`gameplay.rs:851-853` `Settlement`「单一结算所有者」。ADR 0002。
- **代价**：`EffectKind` 稳定码从 0–3 扩到更多（只追加、旧码不动）；新增 proto effect 字段须按 ADR 0005 只追加 tag + 保留 `reserved 4 to 8`。
- **跨平台确定性影响**：所有新原语运行态用 `BTreeMap` / `BTreeSet` 稳定序、按稳定 `EntityId` 顺序遍历；zone / 召唤 / 光环受益集 / `rng_draws` 全部四处同步进 FNV-1a 哈希与 v9 快照；无 `f32` / `f64`、无 wall-clock、无宿主 locale 依赖（ADR 0002 §6-8）。
- **分层依赖 / 架构原则**：SRP（catalog 单一职责）、OCP（注册新行为 = 扩展）、DIP（数值门面委托抽象注册表）、LoD（结算只经 `Settlement`）、CARP（效果组合而非继承）。

### D4（Q4）随机流消费契约

- **决策（已拍板定案、按推荐）**：沿用单条 match-owned 整数 LCG + 显式 `rng_draws` 编码（P8.5c 先例 `0002-determinism.md:113`），消费顺序进快照 / `first_difference`；**不新增独立流**。
- **备选方案（被否）**：per-skill 独立流 —— 会引入「哪个流先消费」的帧内顺序问题、破坏「每帧恰好消费一次主 LCG」契约（`0002-determinism.md:44-47`），且参考项目侧随机流实现本次未打开核实（缺口④），不臆造。
- **依据**：R12.4（`WC3-MELEE-RULES.md:241-247`）`[已核]`；R6 / R5 proc `[待核]` 数值；ADR 0002 §4 / §6-8。
- **代价**：技能 proc 的随机消费必须与主帧序对齐，需显式 `rng_draws` 编码而非隐式宿主随机。
- **跨平台确定性影响**：LCG 为整数运算、`seed` 与消费序列随快照，恢复后按 `rng_draws` 续算不重复；表现层使用独立随机源（ADR 0002 §4-5）。
- **分层依赖 / 架构原则**：唯一状态所有者（`RtsMatch` 独占流）、表现层不反写。

### D5（Q5）PROTOCOL_VERSION 是否随 8→9 同步 bump + bump 9 全套

- **决策（已由用户规格门拍板定案：三者同步 8→9）**：三者同步 8→9（`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`，沿用 P9 先例 `0005-protobuf-payload.md:87` / `0002-determinism.md:117`）；proto 只追加 tag、`reserved 14 to 32` 保持、旧 v8 明确拒绝。全套执行步骤：四处同步（`compute_state_hash` / `Snapshot` / `first_difference()` / `restore_snapshot`）+ `compute_defs_hash`（定义层冻结常量）+ `cargo test -p w3-core --test determinism` + golden hash 如实重取（不沿用 `0x0b53a047b1f8e5e0`）+ `w3net.proto` 只追加 tag（`reserved 14 to 32` / `EffectDef reserved 4 to 8` 保持）+ **bump 8→9 全仓版本号消费者同步 + 全仓 Grep 旧版本号无残留验证（`tasks.md` 7.10）** + `CODEBUDDY.md §5` 形态①反向验证 + ADR 0002 / 0005 各追加一条「复核记录 / 修订记录」（不改写正文与历史基线数值）+ `cargo test --workspace` + `cargo fmt --all -- --check` + `python tools/verify_all.py`。
- **备选方案（被否）**：因 `EffectDef.effect_kind` 是 `uint32`、若本期不新增玩法命令变体则不 bump `PROTOCOL_VERSION` —— 须与 net-owner / ADR 0005 §3「schema 变更必须同步 bump PROTOCOL_VERSION」一致；Q3 已拍板新增命令 / 效果字段，且用户 Q5 已拍板三者同步，故取同步 bump。
- **依据**：R12.4 / R12.2 `[已核]`；ADR 0002 §6-8 / §7（不支持跨版本迁移）、ADR 0005 §3（禁 map / float / double、删字段保留 reserved、canonical、未知字段不透传）。`8→9` 为**待执行**、不得写成既成事实（结论 G5）。
- **代价**：客户端与 relay 同步发布、无兼容垫片、旧 v8 全部拒绝；golden hash 新值须如实重取、Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对登记为待 VM / 人工，**不得声称 bump 9 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118` / Q6）。
- **跨平台确定性影响**：定宽整数 + 显式小端 + 稳定 ID 顺序不变；proto 生成代码在 Windows / Linux 双平台字节一致（ADR 0005 §3）；W3N1 25 字节帧头不动。
- **分层依赖 / 架构原则**：`w3-core` 不反向依赖 `w3-net`；`w3-net` / `w3-relay` 改动只限 payload 编解码 + 版本拒绝；唯一状态所有者。

### D6 Q3 / Q5 的命令面与 proto 纪律（结论 G8）

- **决策**：施法走 `GameplayCommandKind::Cast`；建造走 `GameplayCommandKind::Train { is_building: true }`；攻击 / 移动 / 停止走 `CommandKind::{Attack, Move, Stop}`。新增技能命令（Q3 / Q5 已由用户规格门拍板定案）按「只追加 kind 码 37 起 / proto tag 59 起、`reserved 14 to 32` 保持」纪律，校验 / 哈希编码 / 穷尽分发三处同步。
- **备选方案（被否）**：复用 `RegisterBehavior` 承载技能命令（被否：`RegisterBehavior { id, effect }`（`gameplay.rs:1347-1350`）是「注册一条 `extra_effect`」语义，无法承载点目标 / 范围 / 方向）；写成 `GameplayCommandKind::Attack` / `::Build`（被否：两个变体不存在）。
- **依据**：R1 / R1.4 / R12.2 `[已核]`；R12.7 `[取舍]`；ADR 0002 §2-3 / §6、ADR 0005 §3；`command.rs:3,11-49`、`gameplay.rs:1322-1546`。
- **代价**：需同步 `hash_gameplay_command` 编码 / 校验 / 穷尽分发、`w3-render` 穷尽分支、`net-owner` / `client-owner` 串行协作。
- **跨平台确定性影响**：命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、提交时冻结到达序号；R1.4 过滤保证金矿 / 树木 / 建筑 / 资源点 MUST NOT 响应 `Move`（ADR 0002 §2-3）。
- **分层依赖 / 架构原则**：SRP（命令只登记输入、逐帧结算由 `step()` 完成）、显式失败不静默降级。

### D7（Q7）AI 施法边界（T5 编号澄清）

- **决策（已拍板定案、按推荐）**：本期**不含** AI 自主施法；AI 全量行为顺延 P12；本期只保证技能可被真人命令 / 回放 / 联机 lockstep 确定性驱动（含 `AiScript` 现有取 `Cast` 目标 ID 的只读夹具不顺延改动）。
- **备选方案（被否）**：本期纳入 AI 自主施法 —— 与用户拍板「AI 全量行为顺延 P12」冲突，且 `WC3-MELEE-RULES.md:57` §0.3 T5（AI 玩家 × 联机 lockstep 接合）架构未定、回到人拍板；不借 ConcordRTS 推导（T7 只作通用架构 / T1 C++ 实现对照）。
- **依据**：S1（`WC3-MELEE-RULES.md:24`）/ §0.3 T5（`:57`）`[待核 / 待拍板]`（**规则文档 §0.3 T5 待确认项、非本 change 规格门 Q；本期不含 AI 自主施法已拍板**）；R12.7（`:241-247`）`[取舍]`；P10 被否台账。**注意**：`UPSTREAM.md` 矩阵 **T5**（godot 场景 / HUD / Melee 初始化，`:110`）与 `WC3-MELEE-RULES.md:57` §0.3 **T5 待确认项**（AI×lockstep 接合）不是同一件事，MUST NOT 混用。
- **代价**：AI 敌人玩家不会主动施法，属 P12 范围；本期验收线只到「三路径确定性驱动」。
- **跨平台确定性影响**：`run_ai` 既有决策状态仍各端确定性重算、进哈希 / 快照（`deterministic-simulation/spec.md:44`）；本期新增字段为零（AI 侧）。
- **分层依赖 / 架构原则**：SRP（AI 决策不改技能结算）、LoD。

### D8（Q8）验收判据与真窗口人工清单

- **决策（已拍板定案、按推荐）**：CPU 侧 = `cargo test -p w3-core --test determinism` + 新增形态①反向验证（测试内反例 + 元断言防退化：缺陷态失败 / 修复态通过）+ 四族技能逐帧哈希一致 + 回放一致 + 旧 v8 拒绝；真窗口人工项（`tasks.md` 第 8 组 `8.x`）保持 `- [ ]` 未勾、如实标「待人工」，不用离屏截图 / CPU 回归冒充。
- **备选方案（被否）**：本期即要求真窗口打完一局 —— 成本高、与 Q8' 边界冲突，且 `feedback_gui_fix_selfcheck` 约定 CLI 不可观测项先自测再请人工验收，不重复请人工反复试错。
- **依据**：R12.4 / R12.2 `[已核]`；`CODEBUDDY.md §5`（反向验证形态①）/ §9（人工清单如实标注）/ `critic.md:29` 清单第 11 项③。
- **代价**：真窗口人工验收留缺口、如实登记为已知限制，不冒充通过。
- **跨平台确定性影响**：CPU 验收在 Windows 侧跑；CI 双平台 `verify_all.py` 由**人工 push 后**触发（AI 不得 push / fetch / pull / clone），不预先声称通过。
- **分层依赖 / 架构原则**：显式失败不静默降级、测试即契约。

### D9（Q9 / Q-A）规则文档同步（流程约束，已拍板形态与顺序）

- **决策**：收尾在 `decision-memo.md` 的「## 规则文档同步」小节给出结论，Q-A 活文档细化（`WC3-MELEE-RULES.md` **S3 下细化「英雄 / 单位技能全量」** + 附录「规则 → 实现覆盖对照」新增 **R6 技能独立行** + §15 变更记录追加一条，**不新增 S 编号**）与 P11 同批、但**必须在规格门批准记录回填后的 apply 阶段**由 `docs-owner` 落定；本期正文零改动。按 `WC3-MELEE-RULES.md §14` 第 2 条注明 **2026-11-24** 日期与依据、不静默改规则；`tasks.md` 以一条复选框任务承接该步骤。
- **备选方案（被否）**：提案阶段即改活文档 —— 越权、且规格未拍板时改活文档会与四件套打架（用户已明确拍板前置顺序）；把「design 引用 `R-x.y`」冒充「规则已实现」（被否：`WC3-MELEE-RULES.md:393` 三态判定 / §14 第 5 条 / `critic.md:34` 清单第 13 项）。
- **依据**：§14 第 2 / 5 条（`WC3-MELEE-RULES.md:415-421`）、附录覆盖对照（`:391-411`，现有 R6 行 `:402` 不含技能）、§15 变更记录（`:423-433`）、`openspec/config.yaml rules.proposal / rules.tasks`、`critic.md:34` 第 13 项。`[已核]` 流程 / Q-A 已拍板、非待拍板项。
- **代价**：活文档本期不改，覆盖对照 R6 技能行在 apply 阶段才更新；`rule_doc_sync_check()` 对 `pending` 豁免、结构门禁待 `approved` 后生效。
- **跨平台确定性影响**：无（纯文档，不进哈希 / 快照）。
- **分层依赖 / 架构原则**：ADR 记「为什么」、spec 记「系统该做什么」、活文档记「规则基线」，三者不互相改写（`CODEBUDDY.md §8`）。

### D10（Q10）规格门验锚

- **决策（已拍板定案、按推荐）**：转写参考锚一律保留 `@305ea3a75` 双标（`vera20k@305ea3a75:src/net/lockstep.rs:1-20,35-39` / `.../command_exchange.rs:39,52,80` / `docs/Networking.md`），MUST NOT 退化成工作树 HEAD 行号；凡涉及 T1–T7 却无「## 参考项目对照」按 blocker（`critic.md:31` 清单第 12 项）。
- **备选方案（被否）**：按工作树 HEAD（`926a036e`）改写行号 —— 会与 `UPSTREAM.md:11` 固定提交不一致；按 `UPSTREAM.md:19-25` 先例保留原锚、不据前进改写任何 ADR。
- **依据**：`UPSTREAM.md:11,19-25` 升级复核先例；命令输出 `git merge-base --is-ancestor 305ea3a75 HEAD`（exit 0 / 正常前进、非换分支）；`critic.md:31`。`[已核]` 人事项。
- **代价**：`vera20k` 工作树 HEAD 前进登记为待 `docs-owner` 独立 change 处理的人事项，P11 不因此阻塞。
- **跨平台确定性影响**：无（T1 只作确定性内核 / 哈希 / 排序 / 快照方法论对照，不复制代码）。
- **分层依赖 / 架构原则**：只读参考不复制代码（ADR 0001）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡矩阵 T1–T7 命中的 change MUST 有本小节，逐条列「本次参考的项目 / 路径 / 借鉴了什么 / 明确不借鉴什么（含许可证理由）/ 可核验锚」。五个参考项目只读、不复制代码（ADR 0001）。本次命中 **T4 / T3 / T1**，可选交叉 **T5 / T7**；**T2 / T6 不命中**（原因见 Context）。

### 结论 1（G1，已核实）具名技能数据来源是本体 MPQ 数据表、与地图脚本分离

- **参考项目 / 路径**：`godot_warcraft3`（MIT）`docs/data/WC3_ASSET_PATHS.md:86-107,245,274`、`:25,32`；`docs/data/PIPELINE.md:136-149,118-121`；`apps/game/config/development_asset_request.source:88-142`；`packages/content/definitions/units/unit_abilities_def.gd:70-71`；`tools/asset-convert/src/development-references.js:17`；`WarsmashModEngine`（AGPLv3）`.../CUnitData.java:164`、.../StandardObjectData.java:50,217。
- **借鉴**：定义层形态 = `AbilityData`(数值) + `UnitAbilities.abilList` / `heroAbilList`(挂载) + `*AbilityFunc.txt`(order/art) + `AbilityBuffData`(buff)，映射为 `FrozenDefinitions` 内冻结定义（`crates/w3-core/src/gameplay.rs:367-369`）；Q1 已拍板按本机导出表**清单条目与分类逐项对齐**即沿用此数据分层（规模事实锚见本文件「## 清单规模事实锚」），数值 `[待核]`、以本机导出 SLK 人工核对、不入库、不照搬经典名称 / rawcode / 数值。
- **不借鉴**：导出的 `assets/slk-exported/**` 内容不入库（ADR 0001）；Godot / GDScript 实现不复制（MIT 允许但本项目不用其宿主）。
- **锚**：`WC3_ASSET_PATHS.md:86-107`（`UnitAbilities.slk` / `AbilityData.slk` / `*AbilityFunc.txt` / `ItemData.slk`）、`:274`（「地图自带脚本在地图包内的 `war3map.j`，不在此目录」）、`PIPELINE.md:118-121`（输入 `.cache/wc3-assets/**/*.slk` → 输出 `assets/slk-exported/<逻辑路径>.json`）、`unit_abilities_def.gd:70-71`（`abilList` / `heroAbilList` 是 SLK 记录字段）、`StandardObjectData.java:50,217`（启动读本体 SLK）。

### 结论 2（G1 核心，**论证性结论**）具名技能 = 本体数据 + 引擎原生行为，Melee 不依赖 WE

- **诚实边界（防 `critic` 第 12 项判臆造）**：这是由「本体表 vs 地图脚本路径分离」+「Data / Logic / Present 分层」+「Melee Bootstrap 原生实现」+「ADR 0003 范围决策」共同推出的**推论**；参考项目**没有一句原文直书**「Melee 具名技能不是 `war3map.j`」。Warsmash 复刻引擎**确实并存三形态**（42 个 Java 原生类如 `.../orc/taurenchieftain/CAbilityWarStomp.java`、`.../orc/farseer/CAbilityChainLightning.java`；12 个 `resources/Scripts/Abilities/**/*.j` 如 `Human/Archmage/Blizzard.j:1-45` vJASS；15 个 `core/assets/abilityBehaviors/*.json`）——这是 **Warsmash 的实现选择 ≠ 经典 Melee 依赖地图脚本**；本项目 N5 不做 JASS VM，**只借数据分层 / 行为分类 / 效果组合语义，明确不借鉴 JASS 脚本实现路径**。
- **参考项目 / 路径**：`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:12-13,117-126`（Data→Logic→Present、Logic 按行为类型分、行为表 `summon_point` / `channel_aoe_damage` / `mass_teleport` / `aura_regen_mana`）、`:38-48`（每个具名技能独立 GDScript 类）、`:63-74`（被动 = Func 无 Order）；`docs/design/game/ROADMAP.md:12,23-24,39,42,74-76`（Melee 对战初始化 = 固定动作表、可先做 `MeleeBootstrap` 硬编码原生实现、触发器 VM 属远期 E）；`docs/roadmap/NEXT.md:35`（N∞ 远期才做触发器 VM）。
- **借鉴**：Data / Logic / Present 分层、按行为类型分类、效果组合语义。
- **不借鉴**：`war3map.j` / vJASS / `*.j` 脚本实现路径（N5）；Warsmash 为 AGPLv3 只借语义、不复制代码。
- **锚**：`WarsmashModEngine/.../WarsmashConstants.java:60`（`JASS_FILE_LIST` 含 `war3map.j`）、`docs/decisions/0003-scope.md:12,15,38`、`WC3-MELEE-RULES.md:43-46`（N2 / N3 / N5）。

### 结论 3（G2 / G3，已核实）边界合规与许可证

- **参考项目 / 路径**：`WarsmashModEngine/LICENSE:1-3`（实开为 **AGPLv3**）、`jassparser/`（含 `antlr-src/` / `src/` 子目录的解析模块）、`fdfparser/`、`resources/Scripts/melee.jui:1-18`（JASS UI 帧定义，仅作 FDF / UI 格式参考）、`resources/Scripts/common.jui`（实开存在）；`docs/references/UPSTREAM.md:14,53,109`。
- **借鉴**：仅 FDF / UI 格式语义与「行为分类」语义。`docs/decisions/0003-scope.md:32`（P5 已完成并归档、契约层 = 三层模板 + 效果组合）。
- **不借鉴（含许可证理由）**：**MUST NOT** 引入其 Java / LibGDX 代码或依赖、不复制 `*.j` / `.jui` / `jassparser/` 代码；AGPLv3 与本项目 GPLv3 组合有条件，引入代码须按 `docs/decisions/0001-license.md:17,59,66-73` 复核点重评（本次未触发）；`0003-scope.md:15,38,61,75,89`（JASS / wtg VM 不做，其余「明确不做」条目不变）。
- **锚**：`WarsmashModEngine/LICENSE:1-3`、`UPSTREAM.md:14,53,109`、`0001-license.md:17,59,66-73`、`0003-scope.md:15,32,38,61,75,89`。

### 结论 4（G4，已核实）现有四类效果原语不足以表达经典技能原语 + **defs.rs:794-797 约束被取代**

- **现状锚**：`crates/w3-core/src/gameplay.rs:283-288`（`EffectKind{Damage,Heal,BuffStat,Stun}`）、`:292-296`（`EffectDef`）、`:312-318`（`AbilityDef`）、`:416-421`（`effect_kind_code` 仅码 0–3）、`:1323-1327`（`Cast` 仅单体实体目标、无点目标 / 范围 / 方向）、`:1347-1350`（`RegisterBehavior`）、`:1311-1314`（`BehaviorSpec`）、`:2090-2105`（`resolve_effects` 链尾追加一条 `extra_effect`）、`:842-849`（`Vitals` 无坐标 / 范围 / 随机 / 召唤槽）、`:851-941`（`Settlement` 只有 damage / heal / buff / stat_mod / apply_stat_mod / filter_targets）、`:950-954`（`BuffInstance`）、`:1295-1299`（`AuraState` 无半径 / 受益集 / 订阅）、`:1983-1991`（`WorldGameplayState` 的 auras / behaviors）；`crates/w3-core/src/sim_match.rs:1316-1358`（`execute_cast` 一次性结算）、`:2501`（`Cast` 分发）；`crates/w3-client/src/melee/defs.rs:120,122,124,279,281`（合成技能 ID `ABILITY=10` / 四族英雄 `11–14`）、`:779-823`（合成 `AbilityDef` 实例）、`:43,54,77,160,183,208,219,325`（`[待核]` / 合成原创 / 刻意非经典命名）。
- **「不足」逐项理由（对照经典原语，锚参考侧）**：持续 AOE 多波（`ABILITY_SYSTEM.md:147,205-208`；`WaveState` 绑定刷怪非技能 zone）、召唤（`Cast` 无生成实体语义；`ABILITY_SYSTEM.md:146,207-208`；`WarsmashModEngine/core/assets/abilityBehaviors/RaiseDead.json` 的 `createUnit` / `createTimedLifeBuff`）、光环 / 被动 / 闪避 / 暴击（`AuraState` 无事件订阅 / 受益集；`ABILITY_SYSTEM.md:47,72-74,148,157`；`generalPassives.json` 的 `addEvasionListener` / `addAttackPreDamageListener`、`reactions.json`、`auras.json` 11 类）、变身（`ABILITY_SYSTEM.md:158`；`Transformations.json`）、击退 / 点目标位移（`Vitals` 无坐标、`Cast.target` 是 `EntityId`；`ABILITY_SYSTEM.md:67,149,184`）、驱散 / buff 分类移除 / 链接弹射（`Settlement::buff` 只 push 无分类标签；`CAbilityChainLightning.java`）、随机流（`0002-determinism.md:44-47` 单 LCG 每帧恰好一次；`EffectDef` 无概率字段）。
- **关键（约束取代）**：`crates/w3-client/src/melee/defs.rs:794-797` 的「只用既有 `EffectKind` + `EffectDef` 组合…MUST NOT 新增 `EffectKind` 变体 / 命令变体 / JASS VM」是 **P8.1 当时 change 内约束、不是永久门禁**；P11 已由用户拍板「四族全量 + bump 9 全套」，扩展属本期授权范围 —— **该约束在本 change 被取代及依据** = 用户 2026-11-24 `AskUserQuestion` 明确拍板『四族全量 + bump 9 全套』（本会话用户消息 / leader 转达，仓库文件外证据、人工确认项）+ Q-B 立项顺序（见 D1 / Context）；JASS VM（N5）仍不引入。
- **不借鉴**：Warsmash 的 Java 原生类 / `*.j` / JSON DSL **不复制**，只借「原语不足」的分类语义作论证（AGPLv3）。
- **锚**：上列全部 `file:line`；`WarsmashModEngine/core/assets/abilityBehaviors/RaiseDead.json`、`generalPassives.json`、`humanHeroActives.json:1-50`（`abilityList` + `onBeginCasting` 动作 DSL）。

### 结论 5（G5，已核实 + 设计约束）新增权威状态触发 ADR 0002 全套；`8→9` 待执行

- **参考项目 / 路径**：`vera20k@305ea3a75:src/net/lockstep.rs:1-20,35-39`、`vera20k@305ea3a75:src/net/command_exchange.rs:39,52,80`、`vera20k@305ea3a75:docs/Networking.md`（**T1 双标**，见结论 9）；本项目 `crates/w3-core/src/snapshot.rs:56-57`、`crates/w3-net/src/packet.rs:45` / `:154-156`、`crates/w3-net/src/proto/w3net.proto:138-154,160-196`、四处 `sim_match.rs:3784,3861,3941,4038`、`gameplay.rs:434,1994-2059,2061-2077,2318,4043-4064`、`crates/w3-core/examples/golden_hash.rs:75-77`。
- **借鉴**：`0002-determinism.md:55-60,62-67,69-72`（哈希 / 快照 / 验收门槛）、`:117`（**P9 7→8 全套先例**：新增顶层权威态、`BTreeMap` / `BTreeSet` 稳定序、13 个 `GameplayCommandKind` kind 码 24–36、22 个 `EventKind`、proto 只追加 tag 46–58、`reserved 14 to 32` 保持、旧 v7 按 `UnsupportedVersion` / `SimVersionMismatch` / relay `Reject` 拒绝、`cargo test --workspace` 全绿）、`:113`（P8.5c `rng_draws` 先例）、`:44-47`（单 LCG）、`:49-53`（表现层只读）；`0005-protobuf-payload.md:38-43,87`（禁 map / float / double、删字段保留 reserved、schema 变更同步 bump `PROTOCOL_VERSION`、P9 三者同步 7→8）；`CODEBUDDY.md:48-54`、`.codebuddy/agents/critic.md:29`。
- **明确不借鉴（含许可证理由）**：`vera20k` 为 GPLv3 + Section 7 附加条款，**只有真的复制其代码时**才进入（`0001-license.md:30` / `UPSTREAM.md:50`）；本次只借确定性内核 / 哈希 / 排序 / 快照方法论，不复制代码、不引入其 crate 依赖。
- **边界（务必）**：当前版本号 **8**，`8→9` 是 P11 **待执行动作非既成事实**；golden hash v8 值 `0x0b53a047b1f8e5e0`（`:118`）**不沿用**、须按 v9 如实重取；**不得声称 bump 9 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118`「Debian 12 侧 v4–v8 逐字节离线比对仍待 VM / 人工完成」口径，Q6）。
- **锚**：上列全部 `file:line`；`w3net.proto:145`（`EffectDef reserved 4 to 8`）、`:198`（`reserved 14 to 32`）、`:160-196`（oneof tag 4–13 + 33–58）。

### 结论 6（G6，已核实）五参考项目组织方式 + 红线

- **参考项目 / 路径**：`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:7-14,16-60`（`ability_catalog.gd` 数值门面 / `ability_behavior_catalog.gd` order-behavior-target-passive 唯一注册表 / `ability_executor.gd` 按 target_kind + behavior 路由）、`:117-126`（建议用 `AbilityBehaviorCatalog` 替代散落分支）、`:214-219`、`:51-56`（Present / Logic 分离）、`:44,46,49,203-208`（buff / 计时器）；`WarsmashModEngine/core/assets/abilityBehaviors/`（15 个 JSON 按种族 / 英雄分组，`humanHeroActives.json:1-50` 的 `abilityList` + `onBeginCasting` 动作 DSL；**分组不对称已核实：兽族仅 `orcHeroActives.json` 无 `orcUnitActives.json`**，与 `UPSTREAM.md:109` 一致）；`.../MutableObjectData.java:411,423,425,428`（`urac` / `arac` / `frac` / `grac` race 分类字段）。
- **借鉴（架构原则）**：catalog 单一职责（SRP）、注册新行为 = 扩展非散落分支（OCP）、数值门面委托抽象注册表（DIP）、Present / Logic 分离；对应本项目 `Settlement`（`gameplay.rs:851-853`「单一结算所有者」）、`FrozenDefinitions`、ADR 0002 §5（`0002-determinism.md:49-53`）、`CODEBUDDY.md:53`。
- **明确不借鉴 / 红线**：`ConcordRTS` 是 C++20 的 RA2/YR 引擎（`UPSTREAM.md:32-36`），SHP / VXL / TMP / MIX / INI 格式、RA2 规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染**绝不作 WC3 事实来源**（`UPSTREAM.md:114` 明令 T3 / T4 / T5 MUST NOT 以 ConcordRTS 为参考；`WC3-MELEE-RULES.md:3,351` 是唯一权威）；`ConcordRTS/README.md:25` 自述「逐字节一致」不得当本项目已验证事实（`UPSTREAM.md:35`）；`godot_warcraft3/docs/design/game/FACTION_UNIT_ROSTERS.md:1-9` 自述「原创三族」「不逐项镜像经典 RTS 单位」只借结构与批次判据；`rts_kernel_cs` 的 `double` 路线不采用（ADR 0002：`0002-determinism.md:15,18,78`）；`WarsmashModEngine` AGPLv3 只借语义；`vera20k` GPLv3 + Section 7 只有真的复制其代码时才进入（`0001-license.md:30` / `UPSTREAM.md:50`）。

### 结论 7（G7，已核实）合规边界 / R7.6

- **参考项目 / 路径**：`godot_warcraft3/docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md:49,67`（道具 abilList 仅白名单 `AIhe/AIma/AIde`、不能直接走 `AbilityExecutor`）；`WC3-MELEE-RULES.md:7`（数值一律不锁定、以本机导出 SLK 为唯一数值来源、不入库）、`:124`（四族定义全为合成 / 原创编译期整数常量、数值以本机导出 SLK 为准、不进权威状态）、`:351`（不得把公开攻略数值冻结进权威定义）、`:374`（B 类纯数值 / 键位绑定，参考项目不作权威）、`:376-385`（防误用清单）；`0001-license.md:21,55,63`（许可证只覆盖代码，MPQ 资产 / 派生数据不入库）；合成原创自述 `crates/w3-client/src/melee/defs.rs:1,27,76-77,160,183,208,219,325,794-797`。
- **借鉴**：道具 abilList 白名单式边界语义（只借结构）。
- **不借鉴（含许可证理由）**：**MUST NOT** 把合成原创编译期常量命名为经典具名技能或冒充其数值；按 Q1 两层口径，**清单条目与分类逐项对齐经典具名技能 ≠ 名称 / rawcode / 数值照搬**，合成原创命名 / 数值 MUST NOT 冒充经典具名技能、不照搬经典名称 / 四字符 rawcode / 数值；任何具名技能数值写实须 `[待核]` 并回到人拍板；本机导出表的具体技能名 / rawcode / 数值不写进四件套或仓库、只登记规模与分类（见「## 清单规模事实锚」）；资产与派生 JSON / PNG 一律不入库（ADR 0001）。**R7.6**（`WC3-MELEE-RULES.md:182`）：道具 abilList / 效果 MUST 走既有 `EffectDef` 组合、MUST NOT 直接执行任意 JASS 技能；`gameplay.rs:2318` 掉落表 EffectDef 段进定义哈希。
- **锚**：上列 `file:line` + `ITEMS_AND_MELEE_AI_PLAN.md:49,67`。

### 结论 8（G8，已核实）命令面事实订正

- **参考项目 / 路径**：本项目 `crates/w3-core/src/gameplay.rs:1322-1546`（`GameplayCommandKind` 全 **36** 变体）、`crates/w3-core/src/command.rs:11-49`（`CommandKind`）、`:3`（排序键 `ExecuteFrame -> PlayerId -> Sequence -> ArrivalOrder`）；`crates/w3-core/src/sim_match.rs:1360-1389`。
- **借鉴 / 决策**：施法走 `GameplayCommandKind::Cast`；建造走 `GameplayCommandKind::Train { is_building: true }`；攻击 / 移动 / 停止走 `CommandKind::{Attack, Move, Stop}`；**MUST NOT** 写成 `GameplayCommandKind::Attack` / `::Build`（不存在，`critic` 清单第 10 项①会挑）；新增技能命令（Q3 / Q5 已由用户规格门拍板定案）须按「只追加 kind 码 / proto tag」纪律。
- **不借鉴**：不照搬任何参考项目的命令面命名（T2 / T6 不命中）。
- **锚**：`gameplay.rs:1322-1546`（确无 `Attack`、确无 `Build`）、`:1328-1332`、`command.rs:3,11-49`。

### 结论 9（G9，已核实；人事项）vera20k 工作树 HEAD 前进

- **事实**：`vera20k` 工作树 HEAD `926a036e`（2026-11-24）≠ `UPSTREAM.md:11` 固定提交 `305ea3a75`（2026-11-17）；`git merge-base --is-ancestor 305ea3a75 HEAD` 返回 0（正常前进、非换分支）；其余四个项目 HEAD 与固定提交一致。按 `UPSTREAM.md:19-25` 先例**保留原锚、不据前进改写任何 ADR**。
- **借鉴**：无（纯人事项登记）。
- **明确不借鉴**：**T1 承重锚一律写 `vera20k@305ea3a75:<path>:<line>` 双标**（`src/net/lockstep.rs:1-20,35-39`、`src/net/command_exchange.rs:39,52,80`、`docs/Networking.md`）；实测同文件 blob 不同、工作树 HEAD 侧行号漂移为 `:38` / `:50` / `:74`，故 MUST NOT 退化成工作树 HEAD 行号。
- **锚**：`UPSTREAM.md:11,19-25`；命令输出 `git rev-parse HEAD` / `git log -1 305ea3a75` / `git merge-base --is-ancestor 305ea3a75 HEAD`（exit 0）。登记为待 `docs-owner` 独立 change 处理的人事项；P11 design 不因此阻塞。

### 结论 10（G9，规格门拍板已定案）规格门 Q 清单

- 见本文件 D1–D10、`decision-memo.md`「## 规格门拍板记录（已定案）」：Q1–Q8 / Q10 **已由用户 2026-11-24 规格门逐条拍板定案**（Q1 改按备选「按 SLK 清单逐项对齐」+ Q1↔Q2 调和「清单对齐 SLK、标识仍合成」+ 全量边界「四族英雄技能全量 + 各族代表性单位 / 道具技能、顺延项标已知限制」、Q5 三者同步 8→9、Q2 / Q3 / Q4 / Q6 / Q7 / Q8 / Q10 按推荐），Q9 / Q-A 为已拍板流程约束、非待拍板项；`critic` 规格门报告 9 条 B 类 residual 按用户拍板「批准前修订再锁定」逐条吸收。四件套按最终口径定案成文、MUST NOT 再写「按【推荐】成文 / 待规格门拍板」；`## 规格门` 保持 `status: pending`、由 leader 回填批准记录。参考缺口回到人拍板、不臆造；清单规模事实锚见「## 清单规模事实锚」、`decision-memo.md`「人工确认项」。

### 许可证核对结论

- **AGPLv3 触及**（`WarsmashModEngine`）但只借语义、未复制代码、未引入 Java / LibGDX 依赖；若任何结论需引用其代码须按 `0001-license.md` 复核点重评（**本次未触发**）。
- **Section 7**（`vera20k` GPLv3 + 附加条款）只读不复制、不进入（`0001-license.md:30` / `UPSTREAM.md:50`）。
- **资产派生**（`godot_warcraft3` `assets/slk-exported/**`）只引用文档路径与文件名作 schema 证据、不引用导出数值、不入库；MIT 许可允许但本项目按 ADR 0001 零资产。
- **ConcordRTS**（GPL-3.0-only）与本项目同许可族、只读不复制、无需重评（`UPSTREAM.md:33`）。
- **rts_kernel_cs**（MIT）T2 不命中、不借鉴其 `double` 路线。

### 参考缺口（如实登记、回到人拍板、不臆造）

1. **四族全量具名技能无参考项目权威清单**（对应 Q1）——参考项目无逐项权威表；用户 2026-11-24 已拍板改按**本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData` 清单逐项人工核对**（规模元数据见「## 清单规模事实锚」、粗统计以人工核对为准、不臆造精确数字），清单逐项固化由 apply 阶段人工核对、数值 `[待核]`、不静默改规则（`WC3-MELEE-RULES.md §14`）。
2. **具名技能数值 / 快捷键参考项目不作权威**（对应 Q2）——`WC3-MELEE-RULES.md:7,351,374` 一律 `[待核]`、以本机导出 SLK 核对、不入库。
3. **无「Melee 不依赖 JASS」直接原文断言**（结论 2 为多锚推论）——已标「论证性结论」并写明诚实边界，防 `critic` 第 12 项判臆造。
4. **参考项目侧随机流实现本次未打开核实**（对应 Q4）——不臆造 per-skill 流，沿用 ADR 0002 单 LCG + `rng_draws` 先例。
5. **`EffectKind` 该扩到何种粒度是设计决策非事实**（对应 Q3）——Q3 已由用户规格门拍板定案、按推荐取最小必要原语集 + `BehaviorCatalog` 注册表，不臆造为参考项目事实。
6. **golden hash 双平台比对排期**（对应 Q6）——Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对登记为待 VM / 人工、不写成已验证（Q6 已拍板按推荐）。

## Risks / Trade-offs

- **[Risk] `8→9` 被误写成既成事实** → Mitigation：四件套统一写「当前 8、本期 bump 9 待执行」；golden hash v8 值标注「不沿用」；ADR 复核记录待 apply 追加（`0002-determinism.md:118` / Q6）。
- **[Risk] 把 `defs.rs:794-797` 的 P8.1 约束当永久门禁、导致四件套自相矛盾** → Mitigation：D1 / 结论 4 显式写「该约束在本 change 被取代及依据」，但 N5 JASS VM 不随之解禁。
- **[Risk] 把 Warsmash 三形态（Java 原生类 / `*.j` / JSON）写成「经典 Melee 依赖地图脚本」** → Mitigation：结论 2 标「论证性结论」+ 诚实边界；只借数据分层 / 行为分类 / 效果组合语义。
- **[Risk] 把 ConcordRTS README 自述「逐字节一致」或原创三族 / RA2/YR 数值当 WC3 事实** → Mitigation：结论 6 红线 + `UPSTREAM.md:35,114`；WC3 业务规则唯一权威 `WC3-MELEE-RULES.md`。
- **[Risk] 新增权威运行态漏进哈希 / 快照 / `first_difference` / restore 之一** → Mitigation：`tasks.md` 形态①反向验证（缺陷态失败 / 修复态通过）+ 四处同步清单 + `critic` 第 11 项③。
- **[Risk] 把 `@305ea3a75` 双标退化成工作树 HEAD 行号** → Mitigation：结论 9 / D10 + `critic` 第 12 项验锚。
- **[Risk] 提案阶段误改活文档 / 把「引用 `R-x.y`」冒充「已实现」** → Mitigation：D9 + `tasks.md` 单条复选框 + `decision-memo.md`「## 规则文档同步」诚实边界 + `WC3-MELEE-RULES.md:393` 三态判定。
- **[Risk] 真窗口 8.x 人工项被 CPU 回归 / 离屏截图冒充通过** → Mitigation：`tasks.md` 第 8 组全部 `- [ ]` 未勾、标「待人工」（Q8 / `CODEBUDDY.md §9`）。
- **[Risk] Q1 两层张力被判自相矛盾**：「清单结构 / 条目逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」→ Mitigation：D1 / 结论 7 / `spec.md` Req1 显式区分两个层次——对齐的是清单条目与分类（哪些技能、每族几个、英雄 3 普通 + 1 大招、单位 / 道具技能），不是照搬经典名称 / 四字符 rawcode / 数值；内部标识仍合成原创、不引入映射层。
- **[Risk] 清单规模粗统计被误当精确数字 / 具体技能名 / rawcode / 数值入库** → Mitigation：「## 清单规模事实锚」只登记规模与分类、标注「粗统计、以人工核对为准」、不臆造「四族全量 = 某精确数字」、实际条目数 apply 阶段核对后如实登记；具体技能名 / rawcode / 数值 MUST NOT 写进四件套或仓库（ADR 0001 / `WC3-MELEE-RULES.md §14`）。
- **[Risk] 顺延项被误读成本期已全量** → Mitigation：本期范围止于「四族英雄技能全量 + 各族代表性单位 / 道具技能」；**中立英雄 / 野怪 / 全部单位技能顺延、如实标已知限制**（`decision-memo.md`「已知限制」/ `tasks.md` 2.1 / `spec.md` Req1 Scenario），不扩范围、不冒充已解决。
- **[Risk] bump 8→9 全仓版本号消费者漏同步** → Mitigation：`tasks.md` 7.10 全仓版本号消费者同步 + 全仓 Grep 旧版本号无残留验证（先例 P9 `tasks.md:39,53`）；归档后主 spec 历史 `==8` 字面按 P9 Q9' 已知限制、后续独立 change 收口（`decision-memo.md`「已知限制」）。
- **[Risk] 规格门 `pending` 被绕过或填 `approved` / fingerprint** → Mitigation：`decision-memo.md`「## 规格门」只写 `status: pending`，由 leader 在用户拍板 + 9 条 residual 批准前修订后回填。
- **[Trade-off] 按 SLK 清单逐项对齐 vs 四族各一套完整原创合成体系** → Q1 已由用户 2026-11-24 规格门**改拍前者（原备选升为决策）**、要真实清单结构；代价 = 清单逐项人工核对 + 数值 `[待核]` 排期落到 apply 阶段 + 顺延项（中立英雄 / 野怪 / 全部单位技能）标已知限制。按 Q1↔Q2 调和口径，**不因此引入四字符 ID → `DefId` 映射层、不照搬经典名称 / rawcode / 数值**，内部标识仍合成原创；原推荐「四族各一套完整原创合成技能体系」降为被否备选。

## Migration Plan

1. **已批准规格态修订（本轮）**：按用户 2026-11-24 规格门逐条拍板定案口径同步回查四件套 + `decision-memo.md`、一次成文（Q1 改备选 + 调和 + 边界、Q5 三者同步 8→9、其余 7 项按推荐、Q9 / Q-A 已拍板、9 条 B 类 residual 批准前修订）；`openspec validate p11-ability-system --strict` + `python tools/check_encoding.py`；规格门 `status: pending`，不改 `crates/**` / 活文档 / CI / 权限配置，不 archive / commit / push。
2. **规格门**：`critic` 按清单第 7 / 10 / 12 / 13 项出报告 `blocker: 0` → **用户 2026-11-24 已逐条拍板定案 Q1–Q8 / Q10**（不拿沉默当授权）→ 9 条 B 类 residual 按用户拍板批准前修订 → **leader 回填 `status: approved` + 四件套指纹 + `approved_by` + `date`** → 规格锁定。
3. **apply 阶段**：先由 `docs-owner` 落定 Q-A 活文档细化（S3 细化 + 附录 R6 技能独立行 + §15 一条，§14 第 2 条注明 2026-11-24 日期与依据）；再由 `sim-owner` / `net-owner` / `client-owner` / `content-owner` 各自目录实施 bump 9 全套（D5 步骤、含全仓版本号消费者同步与 Grep 验证）与四族技能清单逐项人工核对；`critic` 代码门 → 本地 commit（中文描述）→ **人工 push** → CI 双平台 `verify_all.py` → `/opsx:archive`。
4. **回滚**：`8→9` 不提供跨版本迁移（ADR 0002 §7）；旧 v8 快照 / 回放 / 包明确拒绝（`UnsupportedVersion` / `SimVersionMismatch` / `VersionMismatch` + relay `Reject`），回滚到 v8 实现须整组 revert + golden hash 按 v8 重取，不在本期承诺。
5. **待 VM / 人工**：golden hash v9 值的 Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对、CI 双平台门禁、真窗口 8.x 人工清单一律如实登记为「待」，不冒充通过。

## Open Questions

Q1–Q8 / Q10 **已由用户 2026-11-24 规格门逐条拍板定案**（本文件 D1–D10 给出决策 + 备选 + 依据 + 代价，`decision-memo.md`「## 规格门拍板记录（已定案）」逐条登记）；Q9 / Q-A 已由用户拍板形态与顺序，属流程约束、非待拍板项。**本 change 不再有「待规格门拍板」的 Q**，不在 tasks 里烘焙未拍板假设。剩余为已知限制 / 人工确认项（清单逐项人工核对、顺延项、规模元数据粗统计、golden hash 双平台待 VM / 人工、真窗口 8.x 待人工、参考缺口①–⑤、归档后主 spec 历史 `==8` 字面按 P9 Q9' 已知限制），如实登记、不臆造；`## 规格门` 保持 `status: pending`，由 leader 回填批准记录。
