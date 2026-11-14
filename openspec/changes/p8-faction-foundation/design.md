# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实）：`crates/w3-client/src/melee/defs.rs:16-27` 定义 6 个 `DefId` 常量，`unit()`（`:35`）与 `synthetic_definitions()`（`:50`）构造 6 个 `UnitDef`；`crates/w3-core/src/gameplay.rs:50` 定义 `UnitDef`（含 `speed_raw`）、`:90` 定义 `FrozenDefinitions`；`crates/w3-core/src/snapshot.rs:15` 为 `SNAPSHOT_VERSION = 3`；`crates/w3-client/src/input.rs:369` 的 `commit_selection` 与 `:513` 的 `CommandKind::Move` 生成是群体移动入口；`crates/w3-client/src/melee/setup.rs:141` 定义 `MeleeSetup`。主 `openspec/specs/melee-match/spec.md:127-129` 已把「内核零改动 / 不新增权威状态字段 / `SNAPSHOT_VERSION` 仍为 3」写成 requirement。约束：ADR 0001、ADR 0002、ADR 0004、`CODEBUDDY.md` §5 / §8 / §9。

## Goals / Non-Goals

**Goals:** 修群体移动过滤；引入定义层种族分段与开局按种族装配；`w3-core` 零改动、不新增权威状态；为 P8.1 / P8.2 铺路。

**Non-Goals:** 两族完整单位 / 建筑 / 英雄 / 技能（P8.1 / P8.2）、AI 按种族（P8.3）、表现层模型键（P8.4）、真窗口人工验收（本轮如实标注「未执行 / 待补验」）、`w3-core` 任何改动。**P8.5 深度玩法为必做全量，但由五个独立 change 承接，本 change 不承接**（`docs/roadmap/P8-human-orc-melee.md` §0 / §3）。

## Decisions

### D1 在输入翻译层按 `speed_raw` 过滤，不改 `w3-core` 的 `Move` 语义

**决策**：判据来自 **client 侧冻结定义**（`FrozenDefinitions::units()` 的 `UnitDef.speed_raw`，`crates/w3-core/src/gameplay.rs:50`），而非运行时视图。装配期遍历冻结定义，为每个 `EntityId` 派生一份 `EntityId → 可移动性` 的**表现层只读映射**；经 `crates/w3-client/src/app.rs:199` 的 `entity_kinds` 同类注入通路（`:256` `with_entity_kinds`、`:370` `apply_input`）把映射传入 `InputState`，`crates/w3-client/src/input.rs:458` 的 `apply()` 签名增加该只读参数；`:513` 生成群体 `Move` 前按映射筛出可移动实体，落点分配仍按稳定 `EntityId` 顺序、只作用于筛后集合。

**为什么必须新建这条通路（规格门 Blocker 1 的闭合）**：`EntityView` / `WorldView`（`crates/w3-core/src/sim_match.rs:180` / `:193`）只有 `{tick, rng, entities:{id,x,y,facing,hp,owner,alive,has_target}}`，**没有** `speed_raw` / `DefId` / `EntityKind`；`selected` 只有 `EntityId`；`MeleeSetup` 又把运行时 `speed` 强制 `fixed(speed.max(1))`（`crates/w3-client/src/melee/setup.rs:200`），建筑与资源点也被抬到 1，**不能**用运行时 `speed` 替代定义层判据。因此实现 MUST NOT 去 `WorldView` 里找定义（那会滑向改 `w3-core`），MUST NOT 扩展 `WorldView` / `EntityView`，MUST NOT 用运行时 `speed`，MUST NOT 回退到 `EntityKind` / 建筑类型白名单（见备选 2）。所有接线都在 `crates/w3-client/`，`crates/w3-core/` 零 diff。

**备选方案**：

1. 在 `w3-core::sim_match` 收到 `Move` 时丢弃不可移动实体 —— 否决：改内核权威路径、要动状态哈希与快照字段（ADR 0002），且把表现层选择语义塞进内核。
2. 用「建筑类型」硬编码白名单（`EntityKind::is_resource()` 等） —— 否决：违反「数值读 Def 不写死」的工程纪律（引申自 `godot_warcraft3/docs/roadmap/NEXT.md:152`，原文特指造价 / 伤害；「可移动性判据不写死类型」是本项目引申），新增单位就要改表；且 `EntityKind` 只有 `is_resource()` / `is_producer()`、无 `is_movable()`。归档遗留建议为「按单位类型过滤」（`openspec/changes/archive/2026-11-14-melee-playable-loop/design.md` Risks 末条），本 change 改判为**读定义层 `speed_raw` 派生映射**，理由正是避免写死类型；该方向变更已在 `decision-memo.md` R3 登记。注意 `speed_raw == 0` 是定义层既有事实、但在输入翻译层不可达，故按上述决策派生映射，而非直接读视图。
3. HUD 选中时就不允许选中建筑 —— 否决：改变选择契约（`rts-command-hud`），建筑仍需可选中以查看 / 训练。

**ADR 0002 / 跨平台确定性影响**：只在表现层输入翻译过滤命令集合，`CommandKind::Move` 线格式、命令排序键、`step()`、状态哈希与快照全部不变；同一输入脚本在 Windows / Linux 生成同一命令序列。**不改变任何跨平台确定性结论**，也不得据本 change 声称 ADR 0002 有新验证。

**架构原则对照**：SRP（过滤只在输入翻译一处）、DIP（只读 `WorldView` 定义、不反写内核）、显式失败（全不可移动时给反馈、不静默生成空命令）；不触及分层依赖 / 冻结输入等硬红线。

### D2 种族维度放在**定义层**（DefId 分段），不新增 `Player.race`

**决策**：`crates/w3-client/src/melee/defs.rs` 把 `DefId` 按种族分段（人族 `1xx`、兽族 `2xx`、中立 / 资源 `9xx`），`synthetic_definitions()` 仍返回 `FrozenDefinitions`；`MeleeSetup` 增加种族参数，开局 `Spawn` 命令写对应段的 `DefId`。**不**在 `w3-core` 的 `Player` / `RtsMatch` 上加 `race` 字段。

**备选方案**：

1. 在 `w3-core` 加 `Player.race` 权威状态字段 —— 否决：新增权威状态 MUST 同时进状态哈希、快照与 `first_difference()`（`CODEBUDDY.md:47-53`、ADR 0002），`SNAPSHOT_VERSION` 可能升版，直接违反主 `melee-match/spec.md:127-129`。
2. 每个种族一份独立的 `FrozenDefinitions` / 独立对局 —— 否决：一局内两族对抗需要同一 `RtsMatch`，两份定义破坏单一真值源与 `defs_hash` 口径。
3. 用字符串 / 枚举 tag 挂在实体上 —— 否决：引入 `String` 进权威状态、改哈希编码，且与「DefId 已能区分类型」重复。

**ADR 0002 / 跨平台确定性影响**：`FrozenDefinitions` 内容哈希已进快照头部，分段定义差异被 `defs_hash` 天然覆盖；不新增权威状态字段、不改哈希字段集合与快照头部布局，`SNAPSHOT_VERSION` 仍为 3。开局 `Spawn` 仍是既有命令，排序键不变。跨平台无差异（全为编译期整数常量）。

**架构原则对照**：OCP（扩展 DefId 分段而非改写内核类型）、SRP（种族是定义层属性）、CARP（组合既有 `FrozenDefinitions`）、唯一状态所有者（不新增竞争状态）。

### D3 反向验证用「测试内反例元断言」

**决策**：群体移动回归构造「1 农民 + 1 主城 + 1 金矿」的选中集合，断言只有农民收到 `Move`；再定义一个**退化过滤器**（不过滤、把所有实体都当可移动），证明该断言在缺陷实现下**失败**、正式实现下通过，作为「改坏验红」证据（`CODEBUDDY.md:51`、`openspec/config.yaml:61`）。

**备选方案**：

1. 只写正向断言 —— 否决：正向断言在缺陷实现下也可能全绿。
2. 人工改坏代码验红 —— 否决：不可复现、不进 CI。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图与命令，不触碰 `w3-core` 状态；本 change 仍包含重跑 `determinism` 的任务以确认零影响。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」与 `openspec/config.yaml` 的 `rules.design`。本 change 命中 T1（确定性内核 / 状态哈希 / 命令排序）、T2（移动 / 编队）、T4（玩法语义，仅登记、本 change 不落地）、T5（场景装配 / Melee 初始化）；四个参考项目只读、不复制代码（ADR 0001），本机绝对路径见 `UPSTREAM.md` 开头表格，不在本文重复。本小节为**核验版**：上一轮为未经真实调研写入，已按 `reference-researcher` 报告逐锚重核并纠正归因，再经 `critic` 清单第 12 项聚焦核验无 blocker。资产与派生文件一律不入库（ADR 0001、`UPSTREAM.md` §合规）；MIT（godot / rts_kernel_cs）、GPLv3 + Section 7（vera20k）、AGPLv3（Warsmash）的许可证边界见各条「不借鉴 / 边界」；参考缺口按 `UPSTREAM.md` 触发时机第 5 条回到人拍板（本 change `status: pending`）。

- **T1 确定性内核 / 状态哈希 / 命令排序 → `vera20k`**：锚 `vera20k/src/net/mod.rs:1-15`（纯命令传输与依赖规则）、`vera20k/src/net/lockstep.rs:36-38` / `:648`（`MultiplayerChecksumHistory` / `ChecksumMismatch` 逐帧 checksum 对比）、`vera20k/docs/Networking.md:207`（replay 播放校验已记录 state hashes）。**借鉴**：lockstep 逐帧 checksum 对比的 desync 检测思路，以及 `net/ is a pure command transport ... doesn't know game rules`（`mod.rs:3-5`）的依赖纪律——呼应本项目「表现层只传输入命令、不反写内核」。**不借鉴 / 边界**：RA2 专属的 gamemd 地址 / INI / SHP / VXL / 2D 等距实现；`services/relay-protocol/` 的 "snapshot" 是链路测量 / netsim trace 快照（`services/relay-protocol/src/link.rs:98`），非权威状态快照。**归因纠正**：「新增权威状态字段必须同时进入状态哈希与快照」是**本项目** `CODEBUDDY.md` §5、ADR 0002 §6/§7/§8（`0002-determinism.md:58` 参与字段固定、`:72` 改字段须先过 determinism）与主 `openspec/specs/melee-match/spec.md:127-129` 的硬门禁，**不是 vera20k 的纪律**；D2 否决 `Player.race` 的依据应指向这三者。GPLv3 + Section 7 附加条款仅在真的复制代码时进入（ADR 0001），本 change 不复制。
- **T2 移动 / 编队 → `rts_kernel_cs`**：锚 `rts_kernel_cs/docs/MOVEMENT.md` §「Group placement and formations」`:73-81`（`:77` "stable ideal slots using deterministic minimum squared-distance matching"、`:79` "unchanged distance/Y/X order"）、§「读取数据的所有权」`:109`（`Entities` 按实体 ID 排序）、§「追加与来源优先级」`:36`；`rts_kernel_cs/docs/FRAMEWORK.md` §「Issue #29 验收要点」`:64`（"可变群体输入在入队时冻结" + `ExecuteFrame → PlayerId → Sequence → ArrivalOrder` 稳定排序）、§「Issue #33 验收要点」`:81`（按 ID 稳定 live 集合）。**借鉴**：「群体输入集合冻结后、落点分配按稳定 `EntityId` 顺序、单个成员失败不影响其余成员」（`MOVEMENT.md:77`）的**语义**，支撑 `unit-orders` delta「落点分配不受过滤影响」Scenario（`specs/unit-orders/spec.md:19-22`）。**不借鉴 / 边界**：`docs/NAVIGATION.md`（`:32` / `:36` / `:38`）只讲网格 A\* / 动态占地 / 导航快照身份，**无群体落点分配语义**，不作为该 Scenario 的锚；rts_kernel_cs 的 `GroupMove` 是 C# **内核内**语义，本 change 的过滤发生在**表现层输入翻译层**（D1），不复制 C# 实现；`double` 数值路线与「不承诺跨平台确定性」不采用（`src/Rts.Kernel/SimulationTypes.cs:3`、`src/Rts.Kernel/MatchSnapshot.cs:9-13`、`README.zh.md:7` / `:21`、`README.md:7` / `:21`、`docs/MOVEMENT.md:71`；ADR 0002）。
- **T5 场景装配 / Melee 初始化 → `godot_warcraft3`**：锚 `godot_warcraft3/docs/design/game/ROADMAP.md` §「0. 为什么先不写触发器？」`:12-23`（8 条「固定动作表」，`:19`「创建对战初始单位（主城 + 农民等）」）、§「2. 分阶段清单」下「### C. Melee Bootstrap（对战初始化 · 原生）」`:74-92`（动作表 `:78-86`，`:80-81`「按种族 / 对战默认金木」「需种族→单位 ID 表」）；`godot_warcraft3/docs/roadmap/NEXT.md` §「4. 工程纪律」`:152` "数值读 SLK Def，禁止逻辑里写死造价 / 伤害"；实际装配代码 `godot_warcraft3/packages/gameplay/match/melee_bootstrap.gd:27-77`（`:41-43` 按 `race` 取 `town_hall_id` / `worker_id`）、`godot_warcraft3/packages/gameplay/catalog/melee_race_preview.gd:7-12` / `:52-63` / `:67-79`、`godot_warcraft3/apps/game/app/match/match_bootstrap_module.gd:51-58`。**借鉴**：「种族→单位 ID 表」的开局装配**结构**（支撑 D2 按种族装配 DefId）与「数值读 Def 不写死」的**工程纪律**。**不借鉴 / 边界**：`NEXT.md:152` 原文特指造价 / 伤害，D1 备选 2（`design.md:24`）把它推广到「可移动性判据读 `speed_raw` 而非建筑类型白名单」是**本项目引申**，非 godot 原文；不借 Godot / GDScript 实现、字符串 typeId、`GameSession.local_race` 运行时会话状态（`packages/gameplay/match/game_session.gd:8-12`，与本项目「不新增权威状态」正交）；`docs/design/game/FACTION_UNIT_ROSTERS.md` §「9. 灰盒实现批次」`:205-230`（U0–U3、`:230`「每族至少完成两种开局和两种主力组合」）是**原创三族**（`:1-5`）完整可玩性的验收判据，**不适用于本 change 的地基范围**（`design.md:11` Non-Goals），只借「分批灰盒推进」的结构供 P8.1 / P8.2 排期参考，不借单位名 / 数值、不作本 change 验收判据、不冒充经典人族 / 兽族复刻。
- **T4 玩法语义 → `WarsmashModEngine`（仅登记，本 change 不落地）**：锚 `WarsmashModEngine/core/assets/abilityBehaviors/`（15 个 JSON：`humanUnitActives` / `humanHeroActives` / `orcHeroActives` / `nightElfUnitActives` / `undeadUnitActives` / `neutralUnitActives` 等按种族命名，另有 `auras` / `generalPassives` / `reactions` / `Transformations` / `RaiseDead` / `itemSimple` 等通用文件；兽族只有 `orcHeroActives.json`、无 `orcUnitActives.json`，分组不对称）、`WarsmashModEngine/resources/Scripts/Abilities/{Human,Orc}/`（Human 下 Archmage / MountainKing / Paladin 共 11 个 `.j`，Orc 下 TaurenChieftain 1 个 `.j`）、`WarsmashModEngine/core/src/com/etheller/warsmash/units/manager/MutableObjectData.java:411` / `:423` / `:425` / `:428`（单位 / 技能 / buff / 科技数据表的 `urac` / `arac` / `frac` / `grac` race 分类字段）。P8.1 / P8.2 两族单位与技能将参考其「按种族 / 英雄分组」的**行为数据结构**；本 change 只做地基，不引入玩法语义。**不借鉴 / 边界**：`WarsmashModEngine/resources/Scripts/melee.jui:1-73` 经核验是 **JASS UI 帧定义脚本**（`SKIN = "NightElf"`、`CreateRootFrame`、`LoadTOCFile("UI\\FrameDef\\FrameDef.toc")`、资源栏 / 单位头像文本、`FramePositionBounds`），**不是对战初始化动作表、不含任何「开局按种族生成单位」条目**（`.jui` 定义见 `resources/Scripts/common.jui:3-13`）；该固定提交的 `resources/Scripts/` 下也无 `MeleeStartingUnits*` 或经典 `Blizzard.j` melee 初始化 JASS（`resources/Scripts/Abilities/Human/Archmage/Blizzard.j` 是暴风雪技能脚本，不是初始化 JASS）。故 `melee.jui` 只作 FDF / UI 格式参考。**AGPLv3：只借鉴设计语义、不得复制任何代码**，不引入 Java / LibGDX 依赖；若结论需要代码 MUST 标注「须按 ADR 0001 复核点重新评估」。
- **参考缺口**：四个参考项目均没有「编译期整数 DefId 按种族分段、且种族选择不引入任何运行时权威状态」的现成做法——这是本项目在 ADR 0002 与主 `openspec/specs/melee-match/spec.md:127-129` 约束下的原创取舍，由 D2 备选方案论证，**不冒充参考项目结论**。但「按种族 / 阵营组织单位定义」的近似做法确实存在（Warsmash `MutableObjectData.java:411` 的 `urac` race 字段、种族命名的 `abilityBehaviors/` 与 `Abilities/{Human,Orc}/`；godot `melee_race_preview.gd:52-79` 的种族→单位 ID 表、`FACTION_UNIT_ROSTERS.md` 三族名册）：只借「种族→定义查表」的结构，不照搬其字符串 typeId、运行时 `GameSession.local_race` 状态、AGPLv3 代码与原创单位名 / 数值。

## Risks / Trade-offs

- **判据映射接线**：派生映射会改 `InputState::apply()` 签名（`crates/w3-client/src/input.rs:458`）与 `app.rs::apply_input()` 调用点（`crates/w3-client/src/app.rs:370`）；既有 interaction 单测需按新签名适配，但行为契约不变。该映射是**表现层只读派生**，MUST NOT 进入权威状态 / 状态哈希 / 快照；若实现时发现要改 `WorldView` 才能拿到判据，即视为走错方向，须回到本设计而非改内核。
- `speed_raw == 0` 作为可移动性判据是**约定**：未来若有「可移动但速度为 0」的单位（如被定身），需在定义层显式区分，不能靠运行时状态改写；登记为后续约束。
- DefId 分段一旦定下，两族新增单位 MUST 落在各自段内；段位分配写入 `defs.rs` 常量注释，避免跨段冲突。
- 本 change 是 P8 地基，**不**交付任何可玩的两族差异内容；两族可玩性由 P8.1 / P8.2 承接，避免把范围撑大。
- `file:line` 锚随实现代码漂移（上一轮演练的 R1 / B1 发现）：`decision-memo.md` 的锚在 apply 后可能失准；按 `CODEBUDDY.md:103` 规格锁定规则，四件套内同类锚改动会触发重走规格门，本轮回填前先核实。
