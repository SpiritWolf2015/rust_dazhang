# Design

## Context

动机与现状见 `proposal.md` Why，不在此重述。现状（只读核实，锚一律「路径 + 函数名 / 符号名」，不写精确 `:NN` 行号——本轮 `w3-client` 代码会被 owner 改动，P8.1 有行号漂移返工教训）：`crates/w3-client/src/melee/defs.rs::Race` 已具备 `Race::{Human,Orc}`、`opponent()`、`peasant/soldier/town_hall/barracks/gold_mine/tree`、`ranged/siege/farm/altar/smithy/hero`、`category_def_ids`、`hero_ability/harass_ability/weapons_upgrade/armor_upgrade` 按族只读分派，`synthetic_definitions()` 构造 28 `UnitDef` / 3 `AbilityDef` / 5 `UpgradeDef`；`crates/w3-client/src/melee/setup.rs` 的 `MeleeSetup::{new,with_race}` 只存 `race` 一个字段、`ai_race = race.opponent()` 单参数派生、`category_def_ids: local_race.category_def_ids()` 只由本地族派生一份、`Builder::spawn` 的 `CommandKind::Spawn` 不含 DefId、`def_ids: BTreeMap<u64, DefId>` 按席位装配、`MeleeSetup::movability` / `category_movability`；既有测试 `race_does_not_change_authoritative_spawn_commands` / `category_bindings_are_segmented_without_touching_spawn_bytes`。`crates/w3-client/src/melee/ai.rs::AiScript::new` 现状 `catalog = setup.category_def_ids()`（本地族绑定）+ `let race = setup.race`（本地族）取 `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade` 与 `Train` 目标，生成 `AI_PLAYER` 命令；`SetAi { enabled: false }` 关内核 `run_ai`；命令按 `(execute_frame, player_id, sequence)` 稳定排序；既有 `default_script_keeps_human_ability_and_upgrade_ids` / `script_contains_orc_new_category_commands` / 成对 `reverse_validation_orc_dispatch_*`。`crates/w3-client/src/melee/driver.rs::MeleeDriver::new` 一次性提交 `AiScript::new` 外部帧冻结命令、`advance_ticks` 逐帧检测敌方主城存活 → `Leave` → 既有 `resolve_victory`。`crates/w3-client/tests/melee_loop.rs` 的 `play_full_match` / `play_human_full_match` / `play_orc_full_match` / `build_driver` / `build_driver_with_defs` 骨架（用 `build_driver(setup, 0)`、horizon=0 不注入 AI 脚本，故逐帧哈希只反映权威 `Spawn` 初值与玩家命令）、`both_races_play_full_match_with_baseline_frame_hashes`（两族参数逐帧哈希与 `MeleeSetup::new()` 基线一致）、`race_fallback_matches_human_segment_in_full_match`、`full_script_plays_one_match_to_victory_and_restart_is_clean`、60 / 144 Hz 逐帧哈希回归、`human_catalog` / `orc_catalog`。

内核只读事实（锚用符号名）：`crates/w3-core/src/command.rs::CommandKind::Spawn`（不含 `DefId`）、`crates/w3-core/src/gameplay.rs` 的 `GameplayCommandKind::{Train,Cast,Research,Gather,SetAi,Leave}` / `compute_defs_hash` / `FrozenDefinitions::content_hash` / `first_difference()`、`crates/w3-core/src/snapshot.rs::SNAPSHOT_VERSION`（= 3）、`crates/w3-core/src/sim_match.rs` 的 `resolve_victory` / `compute_state_hash` / `EntityView.speed`（只读投影 `UnitState.speed`）。主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」/「确定性 AI 对手」/「胜负结算与重开」、主 `openspec/specs/unit-orders/spec.md`（8 条，含 R1.4 / R13.6 分层判据）、主 `openspec/specs/human-faction/spec.md` / `orc-faction/spec.md`。约束：ADR 0001 / 0002 / 0004、`CODEBUDDY.md §5 / §8 / §9`、`docs/references/WC3-MELEE-RULES.md` R1.4 / R2 / R11 / R12.2 / R12.4 / R12.7 / R13.6 / §0 / §0.3 / §14。

## Goals / Non-Goals

**Goals:** 在 `crates/w3-client/src/melee/` 装配层把本地 / AI 种族解耦为两个独立参数（保留 `with_race` 兼容别名）、新增 `ai_race` 装配层只读字段、`AiScript` 改用 AI 自身种族取命令目标、`category_def_ids` 区分本地 / AI 两份；以 2×2 四组合 CPU 端到端一局验证「各能打完一局、组内两实例逐帧哈希一致、胜负只结算一次 / 重开干净」且既有两族基线 / 回落测试不破坏；`crates/w3-core/` 零 diff、零资产、`SNAPSHOT_VERSION` 仍 3、不加新定义则 `defs_hash` / `content_hash` 不变。

**Non-Goals:** HUD 选族可见 / 选族 UI 与表现层模型键（P8.4）、P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、不死暗夜（P9+）、中文上屏（CJK 字体许可证人工拍板）、运行时新实体真实模型绑定（模型缺键回退已知限制）、联机底座 / 回放（已在能力 S1 / S2）、真窗口人工验收（本轮如实标注「未执行 / 待补验」）、AI × lockstep 命令接合（`WC3-MELEE-RULES.md` §0.3 T5）、`w3-core` 任何改动、任何新增 `UnitDef` / `AbilityDef` / `UpgradeDef` / `EffectKind` / `CommandKind` / `GameplayCommandKind` 变体 / JASS / wtg VM（N5）、启用内核 `run_ai`、新增 `Player.race` 之类权威状态。

## Decisions

> 玩法决策的规则依据与核实状态按 `openspec/config.yaml` 的 `rules.design` 标注：`[已核]` 规则性事实、`[待核]` 数值（不冻结进权威定义）、`[取舍]` 本项目实现决定。本 change **不加任何新定义、不引入新数值**；两族既有「更肉 / 更快」差异沿用 P8.1 / P8.2 已登记的 `[取舍]` 合成原创常量相对大小、非经典事实（C6）。R2 四族不对称是 `[已核]` 规则性事实；R1.4 / R13.6 分层判据、R12.7 AI 全功能、R11 / R12.2 / R12.4 会话与确定性契约均为本项目 `[已核]` / `[取舍]`；双族独立参数、「同一组合两实例逐帧哈希一致」验收口径、`SetAi { enabled: false }` 外部命令源是 `[取舍]`。

### D1 双族独立参数落装配层、不新增权威状态、保留 with_race 兼容别名（写透 C2）

**决策**：新增 `with_races(local_race, ai_race)`（或等价双参数入口）把本地 / AI 种族解耦；`MeleeSetup` 增 `ai_race` 一类**装配层只读**字段，`race` 继续作本地族别名、不破坏既有 `.race` 断言；`with_race(race)` 保留为 `with_races(race, race.opponent())` 兼容别名，`MeleeSetup::new()`（本地人族 / AI 兽族）与既有入口行为逐字节不变；`def_ids` 按席位装配（本地用 `local_race` 段、AI 用 `ai_race` 段、中立 `9xx`）。`CommandKind::Spawn` 不含 DefId、字节逐字节不变（两族 / 四组合装配字节一致）（R2 `[已核]`、装配分层 `[取舍]`）。**MUST NOT** 新增 `Player.race` / `RtsMatch.race` 等运行时权威状态字段；种族只停装配层 `EntityId→DefId` 关联，不进权威状态 / 状态哈希 / 快照（C1 / C2）。

**备选方案**：
1. 在 `w3-core` 加 `Player.race` / `RtsMatch.race` 承载双族 —— 否决：改内核权威状态字段，MUST 同步进状态哈希 / 快照 / `first_difference()` / 可能升 `SNAPSHOT_VERSION`，违反主 `melee-match/spec.md`「内核零改动与确定性契约」（ADR 0002 / C1）。
2. 给 `CommandKind::Spawn` 加 `DefId` 字段承载种族 —— 否决：改内核命令线格式 / 状态哈希 / 快照，两族 `Spawn` 字节不再一致（C2 / 主 `melee-match/spec.md`）。
3. 删 `with_race` 只留双参数入口 —— 否决：破坏既有 `MeleeSetup::new()` / `with_race` 入口与 `race_fallback_matches_human_segment_in_full_match` / `race_does_not_change_authoritative_spawn_commands` 基线测试、既有 tests 逐字节口径（C2 / C4）。
4. 用 `String` / 枚举标签挂实体或每族一份 `FrozenDefinitions` —— 否决：引入 `String` 进权威状态 / 改哈希编码，或一局内两份定义破坏单一真值源与 `defs_hash` 口径（P8.0 D2 备选 2 / 3 同一否决理由）。
5. 保持 `ai_race = local_race.opponent()` 单参数、只在外层「翻转」参数伪造四组合 —— 否决：无法表达同族组合（人vs人 / 兽vs兽）的真实装配，`def_ids` 段位断言无意义（C4 / G6）。

**理由**：`MeleeSetup::def_ids` 已是「种族维度只停在这一层」的既定接缝；`defs.rs::Race` 注释明确「只用于选择开局 `Spawn` 关联的 `DefId` 段，MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`」。加一个装配层只读字段 + 双参数入口即可表达 2×2 四组合，零内核改动、零新定义。

**ADR 0002 / 跨平台确定性影响**：双族参数是编译期 / 装配期只读数据，不进权威状态 / 状态哈希 / 快照；`Spawn` 命令字节、排序键、`SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合、快照头部布局、`first_difference()`、FNV-1a / golden hash 全部不变；本 change 不加新定义，`compute_defs_hash` / `content_hash` 也不变。全为编译期整数常量，Windows / Linux 无差异；**不改变任何跨平台确定性结论**，不得据本 change 声称 ADR 0002 有新验证（macOS / ARM64 / 其它 rustc 仍待验证）。

**架构原则对照**：OCP（扩装配入口而不改内核类型）、SRP（种族是装配层属性）、CARP（复用既有 `MeleeSetup` / `Race`）、唯一状态所有者（装配关联不反写内核）、LSP（新入口与既有入口对同一构造行为一致）。

### D2 AI 命令源按 AI 自身种族产出、category_def_ids 区分两份（写透 C3）

**决策**：`ai.rs::AiScript::new` 现状用 `setup.race`（本地族）+ 本地族 `category_def_ids()` 生成 `AI_PLAYER` 命令、与按 `opponent()` 段装配的 AI 实体错配（P8.1 / P8.2 单族对称闭环未暴露，P8.3 四组合必然暴露）；本 change MUST 改为按 **`ai_race`** 取 `category_def_ids`（训练目标）/ `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade`，使 AI 训练 / 施法 / 研发的 DefId 落在 AI 族段。系统 SHALL 把 `category_def_ids` 区分为本地 / AI 两份（如 `local_category_def_ids` / `ai_category_def_ids` 或 `category_def_ids_for(race)`）：本地验收 / `category_movability` 用本地份、AI 脚本用 AI 份，两份 MUST NOT 串用。复用 `defs.rs::Race` 按族只读分派、不新增定义层语义；仍是外部帧冻结命令（`SetAi { enabled: false }` 关内核 `run_ai`）、按 `(execute_frame, player_id, sequence)` 稳定排序、沿用 `GameplayCommandKind::{Train,Cast,Research,Gather,SetAi,Leave}`、不新增权威状态（R12.7 `[已核]`、按族命令 `[取舍]`、R2 / R4.3 / R4.4 / R6 `[已核]`）。

**备选方案**：
1. AI 命令继续用本地族 / 让 AI 实体改用本地族段装配以「对齐」—— 否决：前者保留错配、四组合仍错；后者把「AI 可选另一族」退化成两族恒同族、违背路线图 §3 与 R2 不对称，且 AI 实体的段位装配是 P8.1 / P8.2 既有行为（C3 / C4）。
2. 改 `w3-core::run_ai` 按种族产出命令 —— 否决：改内核 AI 路径 / 权威状态，且 AI × lockstep 接合架构（§0.3 T5）未拍板，须另走 ADR 0002 / 0005 流程（C7）。
3. 把 `ai_race` 写进 `Player` / 快照「方便 AI 决策」—— 否决：新增权威状态、改哈希 / 快照（C1 / ADR 0002）。
4. 用墙钟 / 随机数 / 会话 `String` 状态驱动 AI 决策 —— 否决：破坏 30 Hz 整数帧与重放确定性（ADR 0002 §2 / §4）；`godot_warcraft3` 的 `GameSession.local_race: String` 是参考项目运行时会话语义、不借鉴（见「## 参考项目对照」T5 诚实判断①）。
5. 让 AI 直接改玩家经济 / 单位字段 —— 否决：旁路写入权威状态、破坏唯一状态所有者与 `first_difference()`。

**理由**：`AiScript` 已是「确定性外部命令源」；`MeleeDriver` 一次性入队并按帧冻结执行、与刷新率 / 卡顿无关。切换查表所用的种族参数即可修错配、不改机制、不碰内核。

**ADR 0002 / 跨平台确定性影响**：命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、外部重放注入同一脚本即可复现；不新增权威状态、不改状态哈希 / 快照；切换的是装配层查表参数，四组合各自命令字节确定、跨平台一致。`SNAPSHOT_VERSION` 仍 3。**不得据 CPU 全绿声称跨平台确定性有新验证**。

**架构原则对照**：SRP（AI 只产命令、种族是装配层数据）、DIP（经既有 `submit_gameplay_command` / `Race` 只读分派）、显式失败（命令被拒时英文日志告警、不静默降级）、LoD（AI 脚本经 `MeleeSetup` 查表、不直接触内核）。

### D3 四组合 CPU 端到端 + 验收口径「组内两实例逐帧哈希一致」、既有基线不破坏（写透 C4）

**决策**：以 `(local_race, ai_race)` 参数化 2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽），各装配并打完一局（采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁敌方主城结算 → 重开），`resolve_victory` 只结算一次、按稳定顺序，重开无残留，`SNAPSHOT_VERSION == 3`（R11.1 / R11.4 / R11.5 `[已核]`、一局帧数 `[取舍]`）。**验收口径（关键）**：断言「**同一组合两实例逐帧 `compute_state_hash()` / `capture_snapshot()` 字节一致**」——因双族参数只停装配层、四组合的权威 `Spawn` 初值相同；既有 `both_races_play_full_match_with_baseline_frame_hashes`（两族参数逐帧哈希与 `MeleeSetup::new()` 基线一致）与 `race_fallback_matches_human_segment_in_full_match` MUST 仍绿、**不静默改其断言**；**不得**断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」（后者是新增定义 / `defs_hash` 变化时才需调整，本 change 不加新定义）（R12.4 `[取舍]` / ADR 0002）。「既有 AI 脚本族断言」（`default_script_keeps_human_ability_and_upgrade_ids` / `script_contains_orc_new_category_commands`）随 D2 切换按实际 `ai_race` 更新断言口径、不静默放宽、不冒充「默认脚本仍是人族段」（见 Risks）。反向验证另见 D5 / `tasks.md` 第 6 组。

**备选方案**：
1. 断言「四组合之间哈希数值相等」—— 否决：四组合各自 AI 命令落在 `ai_race` 段、命令字节本就不同，跨组合哈希数值相等不是本 change 的契约；把它写成断言是过度约束、会在合法实现下假绿 / 假红（C4）。
2. 断言「与改动前旧基线数值相等」—— 否决：本 change 不加新定义、`defs_hash` / `content_hash` 不变，本就无需改旧基线；把旧基线数值写死是把「实现细节」当契约，新增定义时反而要返工（P8.1 / P8.2 已澄清）。
3. 静默改写 `both_races_play_full_match_with_baseline_frame_hashes` / 回落测试断言以「迁就」新参数 —— 否决：这两个是「种族不进权威状态」的既有回归证据，改写即掩盖回归（C4）。
4. 只加正向断言、不比逐帧 —— 否决：正向断言在缺陷实现下也可能全绿、且最终 hash 相同可能掩盖中间帧 desync（ADR 0002 golden-hash 复核记录要求逐帧文本 / 字节比对）。

**理由**：`play_full_match` / `build_driver` 骨架已用 horizon=0 隔离 AI 脚本、逐帧哈希只反映权威 `Spawn` 初值与玩家命令；四组合脚本复用该骨架即可在「组内两实例一致」口径下验收，既不改既有基线、又不臆造跨组合数值相等。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图与命令、不触碰 `w3-core` 状态；仍包含重跑 `cargo test -p w3-core --test determinism` 任务以确认零影响；四组合装配的 `Spawn` 字节相同、权威哈希口径不变。不得据 CPU 全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（每阶段断言、不留静默）、CARP（复用既有 `play_full_match` / `build_driver` / `MeleeDriver`）。

### D4 w3-core 零 diff 与确定性契约不放松（写透 C1）

**决策**：本 change **不新增**权威状态字段 / `CommandKind` / `GameplayCommandKind` / `EffectKind` 变体 / 事件 / `UnitDef` / `AbilityDef` / `UpgradeDef`；`snapshot.rs::SNAPSHOT_VERSION` 仍 3；状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 稳定 `EntityId` 顺序 / FNV-1a / golden hash 逐字节不变；本 change 不加新定义，故 `compute_defs_hash` / `FrozenDefinitions::content_hash` 也不变；`git diff -- crates/w3-core/` 为空。**归因纠正（必须保留）**：「新增权威状态字段须同步进状态哈希 / 快照 / `first_difference()`」硬门禁出处是**本项目** `CODEBUDDY.md §5` + `docs/decisions/0002-determinism.md` §6-8（ADR 0002）+ 主 `melee-match/spec.md`「内核零改动与确定性契约」，**不是 `vera20k`**；`decision-memo.md` 的 C1 锚指向本项目权威文件或 `UPSTREAM.md` 矩阵条目、**不指向参考项目路径**（见「## 参考项目对照」）。

**备选方案**：
1. 借「双族」名义新增 `Player.race` 权威字段并声称「反正还没放数值」—— 否决：只要新增字段就须同步哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`（C1 / ADR 0002）。
2. 升 `SNAPSHOT_VERSION` 到 4 以「容纳」双族参数 —— 否决：双族参数不进权威状态、快照头部布局不变，升版本是改内核权威契约、无必要且违反 ADR 0002。
3. 借 `runtime-entity-movability`「只读视图投影已有权威状态」的放松，把 `ai_race` 做成运行时权威字段 —— 否决：该放松仅适用于复用既有 `Move` / `Attack` 路径 + R1.4 分层过滤，MUST NOT 被借用于新增任何权威状态 / 命令 / 事件（R13.6 范围限定、P8.1 已澄清的常设硬门禁）。

**理由**：主 `melee-match/spec.md`「内核零改动与确定性契约」已是 requirement；P8.0 / P8.1 / P8.2 / `p9-command-hotkeys` 均以 `w3-core` 零 diff 通过，本 change 对称沿用且更彻底（连 `defs_hash` 都不变）。

**ADR 0002 / 跨平台确定性影响**：内核零 diff，`determinism` 26 项与 `cargo test --workspace` 全绿、`git diff -- crates/w3-core/` 为空；逐帧状态哈希与 `capture_snapshot()` 字节按 D3 组内口径一致。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

**架构原则对照**：唯一状态所有者、确定性契约硬红线、`w3-core` 不反向依赖上层与冻结输入边界、显式失败不静默降级。

### D5 反向验证（改坏验红）对称复用 P8.1 / P8.2 先例（写透 C5）

**决策**：新增 / 修改玩法回归 MUST 证明**缺陷态失败、修复态通过**（`CODEBUDDY.md §5` / `config.yaml rules.tasks` 第三条），证据形态用「测试内反例元断言」或 A-B 对照，**对称复用** `defs.rs::rebuild_definitions` / `defs.rs::degenerate_hash_ignoring_new` / 成对 `defs.rs::reverse_validation_defs_hash_*`、`ai.rs::reverse_validation_orc_dispatch_*`（含 `placeholder_catalog()` 退化注入）、`tests/movability.rs::degenerate_filter` 先例。至少覆盖三类退化：①「AI 命令族退回本地族」→ 四组合中至少 2 组（人vs兽 / 兽vs人、以及同族组合若命令 ID 不同）的「AI 训练 / 施法 / 研发 DefId 落在 AI 族段」断言失败、正式实现通过；②「四组合装配退化为单参数 `opponent()`」→「人vs人 / 兽vs兽」两组的 `def_ids` 段位断言失败；③「`with_race` 兼容别名退化」→ 既有回落基线测试失败。退化实现 MUST 真经 `AiScript::new` / `MeleeSetup::with_races` / apply 路径注入、非旁路模拟；仅「全绿」不算完成。

**备选方案**：
1. 只加正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，不构成反向验证（C5）。
2. 人工改坏代码验红、不进 CI —— 否决：不可复现；改用测试内构造退化 `MeleeSetup` / 退化查表并加元断言防退化（`ai.rs::placeholder_catalog()` 已是「把装配层绑定替换回占位表喂给真实 `AiScript::new`」的成熟先例）。
3. 只比最终 hash、不覆盖段位 / 别名退化 —— 否决：会掩盖中间帧 desync 与 AI 命令族错配（C3 / C4）。

**理由**：P8.2 的成对 `reverse_validation_orc_dispatch_formal_passes_the_assertion` / `..._degenerate_fails_the_same_assertion` 已是「测试内反例元断言 + 正式实现元断言防退化」的成熟先例，P8.3 只需对称扩到双族参数与四组合。

**ADR 0002 / 跨平台确定性影响**：退化实现只在测试内构造、不触碰正式 `w3-core` 状态；缺陷态 / 修复态证据均为 CPU 侧编译期整数常量，跨平台一致。不得据测试全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（退化实现必失败、不留静默）、CARP（复用既有退化注入辅助函数）。

### D6 R1 / R13.6 分层判据沿用、本 change 不碰输入层（写透 C4 边界）

**决策**：本 change 不新增定义、不改输入翻译层；四组合装配下 frame-0 主城 / 金矿 / 树木 / 既有新增建筑的可移动性 MUST 沿用主 `unit-orders/spec.md` 的 R1.4 / R13.6 实体来源分层判据（装配期 `UnitDef.speed_raw` 派生只读映射为权威、运行时动态实体用只读 `EntityView.speed == 0` 判不可移动，两条不互替），不改写该 capability 的任何 requirement。四组合装配的 `def_ids` 段位变化只影响表现层关联、不影响可移动性判据来源；含建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`（R1.4 / R13.6 `[已核]`、分层判据 `[取舍]`）。

**备选方案**：
1. 为四组合另写一套可移动性白名单 —— 否决：违反「判据读定义层、不写死类型」（R1.4 / P8.0 D1），且本 change 不加新实体、无需新判据。
2. 在 `w3-core` 收到 `Move` 时丢弃不可移动实体 —— 否决：改内核权威路径、要动状态哈希与快照（ADR 0002 / P8.0 D1 备选 1）。
3. 用运行时 `EntityView.speed` 统一替代装配期映射 —— 否决：frame-0 建筑视图 speed 被 `speed.max(1)` 抬成 `> 0`，会误判可移动、令 R1 bug 复发（R1.4 / R13.6 / §14）。

**理由**：P8.0 已把 frame-0 判据接线闭合、`runtime-entity-movability` 已把运行时动态实体承接闭合；四组合只换装配层段位、不引入新实体种类，分层判据自动沿用、零接线改动。

**ADR 0002 / 跨平台确定性影响**：过滤只在表现层输入翻译、`CommandKind::Move` 线格式 / 排序键 / 哈希 / 快照全部不变；只读 `EntityView.speed` 是投影已有权威 `UnitState.speed`、不新增权威状态字段、`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变。运行时 `Train` / `Build` 动态实体承接边界是已归档 `runtime-entity-movability` 的已知限制，本 change 沿用、不扩视图。

**架构原则对照**：SRP（过滤只在输入翻译一处）、DIP（只读冻结定义）、LSP（四组合装配与既有装配对同一派生函数行为一致）、显式失败（全不可移动给反馈）。

### D7 零资产 / 数值不冒充经典事实 / 单 change 不并后续（写透 C6 / C8）

**决策**：本 change 不加新定义、不复制 Warsmash rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 abilId；两族「更肉 / 更快」等差异是 P8.1 / P8.2 既有 `[取舍]` 合成原创常量相对大小、非经典事实（`CLASSIC_RULES_BASELINE.md:3` / R2 / §14）。零资产 / 零文件 IO / 零 `config.toml` 或导出 JSON 读取 / 零新增依赖（ADR 0001 / ADR 0004）。**一个 change 不并后续**：HUD 选族 UI / 表现层模型键（P8.4）、P8.5 五个深度玩法、不死暗夜（P9+）、中文上屏、运行时新实体真实模型绑定、真窗口人工验收、AI × lockstep 接合（T5）均由后续独立 change 承接（C8）。

**备选方案**：
1. 顺手把 HUD 选族 UI / 选族可见做进本 change 以「对齐路线图 §3 字面」—— 否决：路线图 §3 的 HUD 属 P8.4（§3 P8.4 明确「选族入口可见」）、§6 第 4 条串行推进；本 change 只做「入口参数」层（C7 / C8）。
2. 把 AI × lockstep 接合（T5）塞进本 change —— 否决：T5 是 §0.3 明确待确认项、须回到人拍板，另走 ADR 0002 / 0005 流程（C7）。
3. 把「兽族数值差异」标成 `[已核]` 经典事实 —— 否决：四参考项目都无该数值 / 结论、违反 C6 与 §14 活文档流程「不静默改规则」。

**理由**：ADR 0001 / 0004 与活文档 §0 / §14 是单一权威；本 change 的价值在装配层解耦而非扩内容，不携带任何数值 / rawcode / 字符串表。

**ADR 0002 / 跨平台确定性影响**：不新增权威状态 / 命令 / 事件、不加新定义，`SNAPSHOT_VERSION` 仍 3、`defs_hash` / `content_hash` 不变，跨平台无差异。不改变任何跨平台确定性结论。

**架构原则对照**：DIP（数值读定义、逻辑不写死）、SRP（数值是定义层属性）、只读参考不复制代码硬红线、显式失败（`[待核]` 不冒充原版）。

### D8 范围边界：HUD 属 P8.4、run_ai 是外部 AiScript、T5 后续承接（写透 C7）

**决策**：本 change 落在 1.0 范围 S1（局域网联机对战，含真人 VS AI 敌人玩家）/ S3（4 种族做完）的**过程分期**（P8.3 两族 CPU 闭环、P9+ 补不死 / 暗夜收敛四族）、不新增范围编号（S1–S6 / N1–N6 不变）；AI 玩家全功能属 1.0 必做（R12.7）。**范围边界（防过度承诺）**：路线图 §3 字面「玩家可选族（入口参数 / **HUD**）」中的 HUD 选族可见 / 选族 UI 属 **P8.4**，本 change 只做「入口参数」层；路线图字面「`run_ai` 按种族」在本项目实际是**外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`**（架构取舍，如实登记、不写成启用内核 `run_ai`）；**AI × lockstep 命令接合（`WC3-MELEE-RULES.md` §0.3 T5 待确认项）不属本 change**、由后续独立 change 承接。联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项；N1–N6 不纳入（尤其 N5 JASS / wtg VM、N6 资产入库）。

**备选方案**：
1. 把本 change 写成「启用内核 `run_ai` 按种族产出命令」以贴合路线图字面 —— 否决：本项目 AI 命令源是已交付的外部帧冻结 `AiScript` + `SetAi { enabled: false }`，启用内核 `run_ai` 改内核路径 / 权威状态、且 T5 未拍板（C3 / C7 / `ai.rs` / `driver.rs` 现状）。
2. 把 HUD 选族 UI、P8.5 深度玩法、不死暗夜塞进来「一次做完」—— 否决：C8 / 路线图 §6 第 4 条串行推进、各阶段单独 change。
3. 把 N1–N6 或「联机 / 回放 / 迷雾」写成不做项 —— 否决：§0 已拍板 1.0 范围，联机 / 回放 / AI 玩家 / 迷雾属 1.0；N1 仅指纯本地单机对局形态（C7）。

**理由**：路线图是过程性制作安排，`WC3-MELEE-RULES.md` §0 是 1.0 范围单一权威；P8.1 / P8.2 归档说明已建立「范围边界如实登记、不冒充已解决」的先例，本 change 对称沿用。

**ADR 0002 / 跨平台确定性影响**：范围边界不改变任何权威状态 / 命令 / 哈希 / 快照；T5 后续 change 若涉及命令注入 lockstep 帧流，MUST 另按 ADR 0002 / 0005 复核。不改变跨平台结论。

**架构原则对照**：显式失败（不静默把范围边界写成已完成）、单一状态所有者、DIP（AI 命令经既有帧冻结入口）。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」（T1–T6 表、触发时机第 1–5 条）与 `openspec/config.yaml` 的 `rules.design`。本 change 命中 **T1**（确定性内核 / 状态哈希 / 命令排序 / 快照，C1 / C4 / D4）、**T4**（玩法语义 / AI 按种族，C3 / D2，重点）、**T5**（双方独立选族 / 场景装配，C2 / D1，重点）；**T2 一句、不展开**：R1.4 可移动性沿用 `rts_kernel_cs/docs/MOVEMENT.md:77`「稳定槽位」与 `FRAMEWORK.md:109`「战斗 / 经济 / 技能迁移不纳入该内核契约」的语义，本 change 不碰输入层、不借 `double` 路线。结论由只读 `reference-researcher` 出「参考对照报告」（leader 本会话已接收），下列所有锚已由 `spec-steward` **逐条实际打开核实存在、语义相符**（打开的是 `C:/open_rts/<项目>` 固定提交，见 `UPSTREAM.md` 开头固定版本表，不在本文重复本机绝对路径）。四个参考项目均为**只读**、代码不进入本仓库（ADR 0001）；本机绝对路径 / 资产 / 派生数据一律不入库。许可证：`vera20k` GPLv3 + Section 7 附加条款（仅真的复制代码时才进入）、`godot_warcraft3` / `rts_kernel_cs` MIT、`WarsmashModEngine` **AGPLv3 仅借鉴语义、不得复制任何代码**。参考缺口按 `UPSTREAM.md` 触发时机第 5 条如实登记、回到人拍板、不臆造。**参考项目 `file:line` 锚不漂移、按对照报告原样保留；本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目，不写精确 `:NN` 行号。**

- **T5 双方独立选族（重点，三处诚实判断之一）→ `godot_warcraft3`（MIT）**。锚（逐条打开核实）：`godot_warcraft3/docs/design/game/ROADMAP.md:12-23`（WE 对战初始化是「固定动作表」8 条、先做 `MeleeBootstrap` 硬编码模块、不解析 wtg）、`:74-92`（C 阶段「创建对战初始单位（主城 + 工人于 sloc，**需种族→单位 ID 表**）」、输出「要生成 / 删除的单位指令」且「**不**解析 wtg」）、`:103-123`（F0–F10 人族竖切）；`packages/gameplay/match/melee_bootstrap.gd:27-77`（`spawn_at_sloc(map_loader, sloc, race, player_owner, hf_dict)`、`:41-43` 经 `MeleeRacePreview.town_hall_id(race)` / `worker_id(race)` 查表、按席位 `player_owner` 落点）；`packages/gameplay/catalog/melee_race_preview.gd:7-12`（`enum Race` 四族 HUMAN / ORC / UNDEAD / NIGHT_ELF）、`:51-78`（`town_hall_id` / `worker_id` 按 race 返回 typeId 的查表结构）。**借鉴**：「动作表 + 种族→单位 ID 查表 + 按席位 owner 落点 + 输出指令」的**装配结构**，对 `setup.rs::MeleeSetup::with_races` 双族参数按席位装配可借。**关键如实判断①**：godot 只有「每席位一个 race 查表」泛化结构，**没有**「本地 + 对手两个独立种族参数」——`apps/game/app/match/match_bootstrap_module.gd:50-55` 的 `bootstrap_melee(options)` 只有 `preview_race` 一个入参、`:57` `race = MeleeRacePreview.race_from_string(preview_race)` 用于本地席位 `:103`；对手席位走 `:135-136` `spawn_opponent_base(...)`、`:157` `var race: int = MeleeRacePreview.race_from_string("human")` **把对手种族硬编码为 human**；`packages/gameplay/match/game_session.gd:12` 只有 `var local_race: String = "human"` 单字段、`:70-88` `from_melee_bootstrap(...)` 无 `opponent_race`。四组合 2×2 CPU 逐帧哈希验收**无先例**：`docs/roadmap/ACCEPTANCE.md:24` 的「全部 10 种无序种族配对（含同族）均完成双向出生位对局」是**四族**双向出生位玩法对局草案，且 `:3` 明确「当前范围已按用户最新目标调整为 EI 1v1 MVP …… 以下完整编辑器/四族/战役路线图保留作历史规划，**不再作为当前 MVP 的完成门槛**」；`docs/roadmap/EI_MVP_ACCEPTANCE.md:7`（「先用当前人族实现验证十个系统的协作；种族覆盖必须随版本明确列出，**不能据此称为四族完成**」）、`docs/roadmap/NEXT.md:18`（「Echo Isles 人族 1v1 可结束」）、`:121`（N4-C 第二种族最小集仍是「择一」扩展）。只借「配对覆盖要有明确验收判据、单族 MVP 不冒充四族完成」的**排期 / 如实标注纪律**，**严禁写成「参考项目已有四组合逐帧哈希验收」**。**不借鉴 / 边界**：Godot / GDScript 实现、字符串 typeId / rawcode（`htow` / `ogre` / `opeo`）、运行时 `local_race: String` 会话状态、`CommandKind::Spawn` 字节 MUST 不变（不含 DefId，P8.0 原创）。许可证理由：MIT 借结构、不复制实现代码。

- **T4 AI 按种族（重点，三处诚实判断之二）→ `godot_warcraft3`（MIT）+ `WarsmashModEngine`（AGPLv3，仅语义）**。锚（逐条打开核实）：`godot_warcraft3/docs/design/game/MELEE_AI.md:1`（标题「**人族**电脑对手机制」）、`:7`（单机 EI「人类玩家 vs 人族电脑」）、`:8`（不做科技树 / 全阵营决策）、`:79`（本版电脑**固定人类开局配方** 4 工人 + 主城）、`:86`（「AI 可造子集」= 全部人族基础建筑）；`docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md:9,13`（人族电脑经营进攻）、`:29-37`（A0–A4 闭环）、`:137`（A0 才「**建议**」GameSession 增加参与者配置含种族）、`:147`（A1 固定开局 `human_build_plan.gd`「以小型状态机 / 规则表驱动固定开局」）、`:176-182,195`；`docs/design/game/UNIT_AI.md:13`（「与 UNIT_AI 区别：本组件做基地经营 + 进攻时机」）、`:64`（明确不做「AI 玩家（选点、科技树、多线）」）、`:163`（「若将来 AI 玩家要自动派农民，应是 Melee/Player AI 发 Harvest Order，仍进 `HarvestController`，不扩 `UnitAI.Profile`」）、`:199-206`（`PASSIVE` / `CAMP_CREEP` / `TEAM_PLAYER` / `REACTIVE` Profile **按 owner / 野怪分、不按种族**）；`docs/design/game/COMBAT_SYSTEM.md:61`（「AI 玩家宏观策略（**明确不做**，见 UNIT_AI 非目标）」）；`docs/design/game/FACTION_UNIT_ROSTERS.md:1-9`（**原创三族**、名称 / 造价 / 数值是种子、不逐项镜像经典 RTS 单位）、`:205-230`（U0–U3 灰盒批次、U3 `:230`「每族至少两种开局和两种主力组合」）。**关键如实判断②**：godot **无任何**「AI 按 `race` 查表产出不同训练 / 施法 / 研发命令」语义，是**单一人族固定配方 AI**（`MELEE_AI.md` / `ITEMS_AND_MELEE_AI_PLAN.md` A1 / `UNIT_AI.md:64` / `COMBAT_SYSTEM.md:61`）；本项目 AI 命令源是**外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`**（`ai.rs` / `driver.rs`），与参考项目脚本 VM / 完整 AI profile **不同源**。Warsmash：`core/assets/abilityBehaviors/` Glob **15 个 json**（`humanUnitActives.json` / `humanHeroActives.json` / `orcHeroActives.json` 存在、`orcUnitActives.json` **不存在**；本轮打开 `orcHeroActives.json:2-4` 证实是 `AOwk` / `windwalk` 疾风步、非牛头人）；`resources/Scripts/**` Glob 仅 Human / Orc 技能 `.j`（`Abilities/Orc/` 下仅 `AbilityWarStomp.j`）+ `abilitiesCommon*.j` / `common.jui` / `melee.jui`，**无 `common.j`、无 `*AI*.j`、无 melee / 按种族 AI 脚本**（本轮 `**/*AI*.j` 与 `**/common.j` 均 No files found、`**/melee.jui` 存在且仅为 JASS UI 帧定义），如实登记。**借鉴**：「AI 决策走正常命令入口、按 owner 隔离、策略 / 执行分层」的**结构语义**（`UNIT_AI.md:163` / `MELEE_AI.md:13`）+ U3「每族多组合能结束比赛」**批次判据措辞**（不作本 change 验收判据）；Warsmash 仅借「按种族 / 英雄分组组织数据」**结构**。**不借鉴 / 边界**：完整 AI profile / 固定人族配方 / `String` 会话状态 / `double` 路线 / 原创三族单位名；不复制 AGPLv3 代码 / rawcode / JASS / wtg VM（N5）、不引 Java · LibGDX。`ACCEPTANCE.md:24` 降级事实见上，**为三处诚实判断之三**（四族 10 配对是已降级历史规划、不是当前完成门槛）。

- **T1 确定性 / 逐帧哈希 / 重开干净 → `vera20k`（GPLv3 + Section 7，仅范式）**。锚（逐条打开核实）：`vera20k/src/net/lockstep.rs:35-39`（`CHECKSUM_HISTORY_LEN` / `MultiplayerChecksumHistory` / `ChecksumMismatch` / `FrameInfo` / `FrameInfoCompareGate` 等纯命令传输导出）、`:264-269`（`SynchronizedCommand` 以字节为权威）、`:424-435`（`issue()` / `admit()` FIFO 不区分输入设备）、`:462-488`（`FrameInfo` 带 checksum）、`:539-570`、`:631-649`（`frame_info()` 用 `checksum_history.compare(frame_info)?` 逐帧比对、`:648` 该调用）；本项目先例主 `melee-match/spec.md:80-92,94-111,127-139`（确定性 AI 对手 / 胜负只结算一次 / 重开 Scenario / 内核零改动 / `SNAPSHOT_VERSION` 3 / 哈希快照字节不变）、`tests/melee_loop.rs:206-305,563-579,1127,1599,1846-1871`（`play_full_match` 逐帧哈希 / 两族基线 / 60 / 144 Hz 回归）。**借鉴**：纯命令传输 + 逐帧 checksum 的 desync 检测思路。**不借鉴 / 边界**：RA2 专属 gamemd 地址 / INI / SHP / VXL / 2D 等距实现不借；未复制任何代码，故 **GPLv3 + Section 7 附加条款不进入本项目**（ADR 0001）。**归因纠正（必须保留）**：「新增权威状态字段须同步进状态哈希 / 快照 / `first_difference()`」硬门禁出处是**本项目** `ADR 0002 §6-8`（`docs/decisions/0002-determinism.md:55-72`）+ `CODEBUDDY.md §5` + 主 `melee-match/spec.md`「内核零改动与确定性契约」，**不是 `vera20k`**；`decision-memo.md` 该锚指向本项目权威文件或 `UPSTREAM.md` 矩阵条目，**不指向参考项目路径**。P8.3 双族参数是**装配层只读数据**、不进权威状态 / 哈希 / 快照，`w3-core` 零 diff；本 change 不加新定义则连 `defs_hash` 都不变。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录 `:107`）。

- **与 P8.1 / P8.2 对称复用的锚（已核验仍存在、沿用不重复铺开）**：`ROADMAP.md:12-23,74-92,103-123`、`melee_bootstrap.gd:27-77`、`melee_race_preview.gd:51-78`、`NEXT.md:152-153`（「数值读 SLK Def，禁止逻辑里写死造价 / 伤害」「新技能优先拼 Effect + 注册 Behavior」）、`ABILITY_SYSTEM.md:12-13`（权威数据来自 WC3 Def + **Data → Logic → Present** 分层）、`CLASSIC_RULES_BASELINE.md:3,52`（`:3`「未锁定具体经典补丁；不能宣称当前数值与某一完整原版版本一致」、`:52`「不能把数据行完整性等同于四族玩法完成」）、`FACTION_UNIT_ROSTERS.md:1-9,205-230`、`MOVEMENT.md:77`、`FRAMEWORK.md:109`、`COMBAT_SYSTEM.md:48-52`、`UNIT_AI.md:199-206`、`vera20k/src/net/lockstep.rs:35-39,648`；兽族四条校正（`orcHeroActives.json` 是 `AOwk` / `windwalk` 非牛头人、`AbilityWarStomp.j` 是 `Orc/` 唯一 `.j`、无 `orcUnitActives.json`、无兽族专用 race 字段）沿用、不重复铺开。

- **参考缺口（如实登记、不臆造、不冒充参考结论）**：**G1** 编译期整数 `DefId` 按种族分段且**不引入运行时权威状态**——godot `game_session.gd:12` 是运行时 `local_race: String`、Warsmash / godot 带 `String` typeId 会话语义，四项目均无本项目 C2 式纯装配层双族参数；本项目原创取舍。**G3** `Spawn` 不含 DefId、用装配层 `def_ids` 关联承载种族——P8.0 原创（`command.rs::CommandKind::Spawn`），参考项目均带 typeId / 会话状态。**G6（P8.3 特有）** 四项目均无「本地 + AI 两个独立种族参数解耦、四组合 2×2 CPU 端到端逐帧哈希验收」精确做法：godot 对手硬编码 human（`match_bootstrap_module.gd:157`）、`ACCEPTANCE.md:24` 是已降级四族 10 配对历史规划、`game_session.gd:12` 单字段；本项目原创取舍。**G7（P8.3 特有）** 四项目均无「外部帧冻结 `AiScript` 按种族查表产出训练 / 施法 / 研发命令、且不新增权威状态」精确做法：godot 完整 AI profile + 固定人族配方（`MELEE_AI.md` / `ITEMS_AND_MELEE_AI_PLAN.md`）、Warsmash JASS 且固定提交无 melee / 按种族 AI 脚本、`UNIT_AI.md:64` 明确不做 AI 玩家；本项目原创取舍。数值差异非经典事实（`CLASSIC_RULES_BASELINE.md:3` / `FACTION_UNIT_ROSTERS.md:4`）、`defs_hash` 变化只比「同脚本两实例逐帧一致」非旧基线数值 / 非跨组合相等。**G6 / G7 须在规格门由人拍板**（用户 2026-11-17 本会话指令已放行方向、但技术取舍仍如实标注为 AI 推荐、随批准采纳进 apply，MUST NOT 写成用户已逐条确认；见 `decision-memo.md` P2 / P3 / 待确认项）。

- **许可证边界**：`WarsmashModEngine`（AGPLv3）仅借语义、不复制代码 / rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ 单位名 / 数值、不引 Java / LibGDX 依赖（若需代码 MUST 按 ADR 0001 复核点重评许可证）；`godot_warcraft3` / `rts_kernel_cs`（MIT）借结构、不借实现 / 原创三族名 / `double` 路线；`vera20k`（GPLv3 + Section 7）未复制代码故附加条款不进入、RA2 线格式不借。SLK / 地图 JSON / PNG / GLB 一律不入库（ADR 0001 / ADR 0004）。四项目均不复制代码进仓库。

## Risks / Trade-offs

- **既有 `both_races_play_full_match_with_baseline_frame_hashes` / `race_fallback_matches_human_segment_in_full_match` 不被破坏** → 这两条是「双族参数不进权威状态」的既有回归证据（`play_full_match` 用 `build_driver(setup, 0)`、horizon=0 不注入 AI 脚本，逐帧哈希只反映权威 `Spawn` 初值与玩家命令）。缓解：D1 保留 `with_race` 兼容别名且 `Spawn` 字节逐字节不变、D3 组内两实例逐帧一致口径、不静默改这两条断言（C4）。
- **`ai_race` 装配层只读、不进权威状态** → 新增字段若误写入 `w3-core` 的 `Player` / `RtsMatch` 或进状态哈希 / 快照，会破坏 ADR 0002 与 golden hash。缓解：D1 / D4 明确字段归属装配层、`git diff -- crates/w3-core/` 为空、`determinism` 26 项全绿、不加新定义则 `defs_hash` / `content_hash` 不变（C1 / C2）。
- **AI 命令族切换对既有 `default_script_keeps_human_ability_and_upgrade_ids` / `script_contains_orc_new_category_commands` 的影响与处置** → 这两条断言的是「默认 / `with_race(Orc)` 入口的脚本技能与升级 ID」，现状按本地族生成（错配的产物）。D2 改用 `ai_race` 后，`MeleeSetup::new()`（本地人族 / AI 兽族）的 AI 脚本将落兽族段、`with_race(Race::Orc)`（本地兽族 / AI 人族）落人族段，断言口径 MUST 按实际 `ai_race` 更新、**不静默放宽、不冒充「默认脚本仍是人族段」**；这不是「改既有两族逐帧哈希基线」（那两条属 CPU 会话层、horizon=0），而是 AI 命令脚本单测的语义修正。缓解：`specs/dual-faction/spec.md`「既有 AI 脚本族断言处置」Scenario、`tasks.md` 第 2 组显式任务、由 `critic` 代码门复核（C3 / C4）。
- **四组合命令字节与哈希口径** → 四组合各自 AI 命令落在 `ai_race` 段、字节本就不同；若误断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」会把实现细节当契约、在合法实现下假绿 / 假红。缓解：D3 只断言组内两实例逐帧一致、不静默改既有两族基线、`specs` 与 `decision-memo.md` 被否台账显式登记为被否方向（C4）。
- **`defs_hash` / `content_hash` 不变是本 change 与 P8.1 / P8.2 的关键差异** → P8.1 / P8.2 新增定义是「冻结定义内容变化」、`defs_hash` 随之改变；本 change 不加任何新定义、只改装配层关联与 AI 查表参数，故连 `defs_hash` 都不变、`SNAPSHOT_VERSION` 仍 3、无需调整旧基线数值。缓解：D4 / `proposal.md` Impact / `tasks.md` 第 4 组显式校验 `git diff -- crates/w3-core/` 为空且定义集无新增条目（C1）。
- **真窗口人工验收未执行 / 待补验** → 本轮 CPU 侧以回归 + 门禁自证，真窗口 / GPU / egui overlay 一律未人工观察，如实标注「未执行 / 待补验」、不冒充通过（沿用 P8.1 / P8.2 / `p9-command-hotkeys` 归档说明先例）。HUD 选族可见 / 选族 UI 属 P8.4、本 change 不做（C7 / C8）。
- **`file:line` 锚漂移已用符号名规避** → 本项目 `w3-client` 代码会被 owner 改动，C1–C8 / `design.md` 的本项目代码锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号 / 矩阵条目、不写精确 `:NN` 行号（P8.1 行号漂移返工教训）；参考项目为只读固定提交、其 `file:line` 锚不漂移、按对照报告原样保留。apply 后四件套任一 artifact 改动即视为规格解锁、MUST 按 `CODEBUDDY.md §8 / §9` 同步回查其余三份并重走规格门，不得只改一份。
- **范围外承接** → HUD / 表现层模型键（P8.4）/ P8.5 五个深度玩法 / P9+ 不死暗夜 / 中文上屏 / 运行时新实体真实模型绑定 / 真窗口人工验收 / AI × lockstep 接合（T5）由后续独立 change 承接；联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项（C7 / C8）。
