# dual-faction Specification

## Purpose
为 P8.3 双族对抗定义行为契约：本地与 AI 两个独立种族参数在装配层解耦、AI 命令源按 AI 自身种族产出、四种族组合 CPU 端到端一局的确定性闭环，全部为装配层只读数据、不新增权威状态字段、`crates/w3-core/` 零 diff、守住 ADR 0002 快照与哈希契约及 1.0 范围 S1 / S3。

## Requirements

### Requirement: 本地与 AI 双族独立参数装配（R2 / R12.7）

系统 SHALL 提供 `with_races(local_race, ai_race)` 双参数装配入口，把本地 / AI 种族解耦为两个独立参数；`MeleeSetup` SHALL 保留 `ai_race` 装配层只读字段，`race` 继续作本地族别名、不破坏现有 `.race` 断言。`Race` SHALL 扩到 4 变体 `Human` / `Orc` / `Undead` / `NightElf`（Q1' 已拍板不死 + 暗夜双族全量 + P8 两族不重做），`COMBINATIONS` 由 2×2 四组合扩到 4×4 十六组合（4 族 4×4、含同族组合）。系统 MUST NOT 新增 `Player.race` / `RtsMatch.race` 分类字段、`CommandKind::Spawn` 字节 MUST 逐字节不变；**Q2 已拍板的族特色机制以独立顶层权威状态段表达、不复用 race 分类字段**。十六组合的 `def_ids` SHALL 按席位装配：本地席位用 `local_race` 段、AI 席位用 `ai_race` 段、中立资源恒 `9xx`（R2 / R12.7 / Q1' / Q5' / Q6 / Q2 / ADR 0002）。细节见各 `#### Scenario:`。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「四组合」「双族」「`SNAPSHOT_VERSION` 仍为 3」等历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合装配段位正确（R2）

- **WHEN** 以 `(local_race, ai_race)` 的 4×4 十六组合（人 / 兽 / 不死 / 暗夜各对本族、异族与同族）分别装配开局，并保留原 2×2 四组合作子集验证
- **THEN** 本地席位实体 `def_ids` 落在 `local_race` 段（`1xx` / `2xx` / `3xx` / `4xx`）、AI 席位落在 `ai_race` 段、中立资源恒 `9xx`，同族组合双方段位一致、不与 `1..=6` 基线或 `8xxx` / `9xxx` 合成段冲突；原四组合段位断言仍绿、不静默改（R2 / Q1' / Q5' / C2）

#### Scenario: 保留 with_race 兼容别名与既有入口逐字节不变（R12.7）

- **WHEN** 分别以 `with_race(race)` 与 `with_races(race, race.opponent())`、`MeleeSetup::new()` 装配开局并比对命令字节
- **THEN** 三者 `Spawn` / `gameplay_commands` / `kinds` / `def_ids` 逐字节一致，`setup.rs::race_does_not_change_authoritative_spawn_commands` 与 `melee_loop.rs::race_fallback_matches_human_segment_in_full_match` 仍绿（R12.7 / C2 / 主 `melee-match/spec.md`）

#### Scenario: 双族参数不进权威状态与哈希快照（ADR 0002）

- **WHEN** 校验十六组合装配的权威状态字段集合与 `SNAPSHOT_VERSION`
- **THEN** 无 `race` / `ai_race` 分类字段或任何以族别为名的权威状态字段，双族参数只停装配层 `EntityId→DefId` 关联；**历史标题字面「仍为 3」是身份键、不作行为断言**：按用户 Q2 / Q3 已拍板，族特色机制新增 `w3-core` 顶层权威状态段（`BTreeMap` / `BTreeSet`、稳定 `EntityId` 顺序）并四处同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（bump 8）定义的 `Snapshot` 顶层字段与 `first_difference()`，`SNAPSHOT_VERSION` 7→8、旧 v7 明确拒绝、golden hash 按 v8 新实现如实重取、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（ADR 0002 §6-8 / `openspec/config.yaml:56` / Q2 / Q3 / C1 / C2）

#### Scenario: 四组合装配下建筑与资源点不响应 Move（R1.4 / R13.6）

- **WHEN** 十六组合（含原四组合）任一装配下框选含主城 / 金矿 / 树木 / 新增不死 / 暗夜建筑的混合选中集合并下达移动命令
- **THEN** 沿用既有 `unit-orders` 的 R1.4 / R13.6 实体来源分层判据、不改写：frame-0 装配实体用装配期 `UnitDef.speed_raw` 只读映射、运行时动态实体用只读 `EntityView.speed == 0` 判不可移动，不可移动实体 MUST NOT 出现在 `Move` 生成集合、位置不变；新增不死 / 暗夜建筑 `speed_raw == 0`，暗夜可拔根古树的扎根态 `speed_raw == 0` 判不可移动、拔根态由定义层 `speed_raw > 0` 判可移动（R1.2 `WC3-MELEE-RULES.md:77-79`、MUST NOT 用名字 / 段位 / 白名单）（R1.4 / R13.6 / R1.2 / C4 / `openspec/config.yaml:57-58` / Q1'）

### Requirement: AI 命令源按 AI 自身种族产出（R12.7 / R2）

`AiScript::new` SHALL 按 **`ai_race`**（而非本地族 `race`）取训练目标 `category_def_ids` 与 `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade`，使 AI 训练 / 施法 / 研发的 DefId 落在 **AI 族段**；系统 SHALL 把 `category_def_ids` 区分为本地份与 AI 份两份（本地验收用本地份、AI 脚本用 AI 份）。命令 MUST 仍是外部帧冻结命令（`SetAi { enabled: false }` 关内核 `run_ai`）、按 `(execute_frame, player_id, sequence)` 稳定排序、经既有 `GameplayCommandKind` 变体、不新增权威状态（R12.7 / R2 / ADR 0002）。

#### Scenario: AI 训练施法研发 DefId 落在 AI 族段（R12.7 / R2）

- **WHEN** 以四组合各生成 `AiScript` 并枚举 `AI_PLAYER` 的 `Train` / `Cast` / `Research` 命令
- **THEN** 训练 `unit_def`、施法 `ability`、研发 `upgrade` 的 DefId 均落在 `ai_race` 段（技能 / 升级为类别内唯一 ID、按族分派），MUST NOT 出现本地族段的目标 ID；人vs兽 与 兽vs兽 的 AI 命令落在兽族段、人vs人 与 兽vs人 落在人族段（R12.7 / R2 / C3 / R4.3 / R4.4 / R6）

#### Scenario: 本地与 AI 两份 category 绑定互不串用（R2）

- **WHEN** 同族 / 异族组合分别读取本地验收用的 `category_def_ids` 与 AI 脚本用的那份
- **THEN** 本地份落在 `local_race` 段、AI 份落在 `ai_race` 段，异族组合两份互不相交、同族组合两份段位一致；复用 `defs.rs::Race` 按族只读分派、不新增定义层语义（R2 / C3）

#### Scenario: 帧冻结外部命令且按稳定键排序（ADR 0002 / R12.2）

- **WHEN** 同一组合构造两次 `AiScript` 并在一端逐帧、另一端一次推进多帧后共推进相同整数帧数
- **THEN** 两次脚本命令逐字节相等、按 `(execute_frame, player_id, sequence)` 升序且序列唯一，两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致；脚本含 `SetAi { enabled: false }` / `Leave`，不启用内核 `run_ai`（ADR 0002 / R12.2 / R12.7 / C3）

#### Scenario: 既有 AI 脚本族断言处置（R12.7 / R2）

- **WHEN** 切换 AI 命令源到 `ai_race` 后复核 `default_script_keeps_human_ability_and_upgrade_ids` 与 `script_contains_orc_new_category_commands`
- **THEN** 按 `MeleeSetup::new()`（本地人族 / AI 兽族）与 `with_race(Race::Orc)`（本地兽族 / AI 人族）的实际 `ai_race` 更新断言口径：默认入口的 AI 命令落在兽族段、`with_race(Orc)` 的 AI 命令落在人族段，不静默放宽既有测试、不冒充「默认脚本仍是人族段」（R12.7 / C4 / `design.md` Risks）

### Requirement: 四组合 CPU 端到端一局与确定性闭环（R11 / R12.4）

系统 SHALL 支持 4×4 十六组合（含原 2×2 四组合）各装配并打完一局 CPU 无渲染对局（采集 → 建造 → 训练 → 英雄施法 → 研发 → **Q2 / Q4 全量机制触发** → 战斗 → 摧毁敌方主城结算 → 重开），每阶段 MUST 可执行验证；`resolve_victory` MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态（R11.1 / R11.4 / R11.5 / `docs/roadmap/P8-human-orc-melee.md:148`）。验收口径 MUST 为「**同一组合两实例逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致**」（R12.4 / ADR 0002）；因新增权威状态 + 定义层内容变化，MUST NOT 断言与改动前 v7 旧基线数值相等、MUST NOT 断言跨组合哈希数值相等，golden hash 按 v8 新实现如实重取。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「四组合」「`SNAPSHOT_VERSION` 仍 3」「golden hash 不变」等历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合各能打完一局并分出胜负（R11.1 / R11.4）

- **WHEN** 十六组合（含原四组合）各注入一份完整一局脚本并推进到敌方主城被摧毁、触发 `Leave`
- **THEN** 全部组合均经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态；腐地 / 古树 / 昼夜 / 隐身目标判定 / 月亮井 / 缠绕金矿等 Q2 / Q4 机制按整数帧确定性结算、新增不死 / 暗夜组合不破坏原四组合基线（R11.1 / R11.4 / R10.2-4 / R1.5 / Q2 / Q4 / Q1' / C4）

#### Scenario: 组内两实例逐帧哈希与快照一致（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、不只比最终 hash、`first_difference()` 为 `None`；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` MUST 按 v8 新基线如实更新后仍绿、不静默改其断言（R12.4 / ADR 0002 / Q3 / C4）

#### Scenario: 不断言跨组合或旧基线哈希数值相等（ADR 0002）

- **WHEN** 编写十六组合确定性回归断言
- **THEN** MUST NOT 断言「十六组合之间哈希数值相等」或「与改动前 v7 旧基线数值相等」；本 change 按 Q2 新增权威状态 + 定义层冻结常量变化 → `defs_hash` / `content_hash` / golden hash 按 v8 新实现如实重取，按「同脚本两实例逐帧一致」口径验收、旧基线数值不写死（ADR 0002 / C1 / C4 / Q3 / `design.md` Risks）

#### Scenario: 重开无残留（R11.5）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、entities / positions / players 视图为空、不含上一局任何权威状态；按 Q3 `SNAPSHOT_VERSION == 8`、旧 v7 快照明确拒绝（R11.5 / C4 / Q3）

#### Scenario: SNAPSHOT_VERSION 与 golden hash 不变（ADR 0002）

- **WHEN** 十六组合打完一局后校验内核 diff 与快照头部
- **THEN** **历史标题字面「不变」是身份键、不作本期行为断言**：按用户 Q2 / Q3 已明确拍板（偏离 AI 推荐、MUST NOT 写成 AI 推荐），`git diff -- crates/w3-core/` **非空属预期**、`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` **bump 7→8**、`Snapshot` 新增族特色机制顶层字段、四处同步、`cargo test -p w3-core --test determinism`（26 项）全绿含「改坏验红」、旧 v7 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、**golden hash 按 v8 新实现如实重取、不沿用 `0x940cb016f6630f1e`、不声称逐字节不变**；v8 Debian 12 侧逐字节比对待 VM / 人工、不声称全平台达成（ADR 0002 §6-8 / R6 / Q2 / Q3 / Q5' / C1）

### Requirement: 反向验证改坏验红（CODEBUDDY.md §5）

新增 / 修改玩法回归 MUST 证明**缺陷态失败、修复态通过**（`CODEBUDDY.md §5`），证据形态用「测试内反例元断言」或 A-B 对照，退化实现 MUST 真经 `AiScript::new` / `MeleeSetup::with_races` / apply 路径注入、非旁路模拟；仅「全绿」不算完成。至少覆盖下列三类退化（C5）。

#### Scenario: AI 命令族退回本地族即失败（R12.7 / C3）

- **WHEN** 把 `AiScript::new` 退化为继续用本地族 `race` 生成 AI 命令
- **THEN** 四组合中至少 2 组（人vs兽 / 兽vs人，以及同族组合若命令 ID 不同）的「AI 训练 / 施法 / 研发 DefId 落在 AI 族段」断言在缺陷态失败、正式实现通过，两条证据可复现（R12.7 / C3 / C5）

#### Scenario: 四组合装配退化为单参数 opponent 即失败（R2 / C2）

- **WHEN** 把 `with_races` 退化为单参数 `race.opponent()` 装配
- **THEN** 「人vs人 / 兽vs兽」两个同族组合的 `def_ids` 段位断言在缺陷态失败、正式实现通过（R2 / C2 / C5）

#### Scenario: with_race 兼容别名退化即失败（R12.7 / C2）

- **WHEN** 删除 `with_race` 兼容别名或使其不再等于 `with_races(race, race.opponent())`
- **THEN** 既有回落基线测试（`race_fallback_matches_human_segment_in_full_match` / `race_does_not_change_authoritative_spawn_commands`）在缺陷态失败、正式实现通过（R12.7 / C2 / C5）

### Requirement: 范围边界与承接（1.0 范围 S1 / S3 / N1–N6）

本 change SHALL 按用户已拍板口径做**不死 + 暗夜双族全量 + 十六组合 CPU 闭环 + Q2 族特色机制纳入 `w3-core` 顶层权威状态 + Q4 全量机制本期纳入 + Q3 bump 8 全套**；**1.0 范围张力 MUST 如实呈现、不静默窄化、不冒充已解决**：`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226`（R10.2 / R10.3 / R10.4）字面「暗夜夜间隐身 / 昼夜 / 隐身反隐随迷雾纳入 1.0 一并做完」，而 P8.5e 规格门 Q2「最小闭环」曾把它们登记为「后续 change / P9+ 四族承接」（`2026-11-22-p8-fog-of-war/decision-memo.md:80,128`）。**Q4 已由用户 2026-11-22 明确拍板本期含全量（不是 AI 推荐、不是静默窄化）、Q1' 已拍板双族全量、Q4' 已拍板张力保留显式**——这是已拍板后的如实登记、不是待拍板项。AI×lockstep 接合（§0.3 T5）、R9 中立建筑交互全量、R13 grid 布局、R3.5 资源共享、R5.5 高地加成、R12.5 观战 / 断线重连 MUST NOT 纳入本 change、不重做 P4 底座（Q7' 已拍板登记已知限制）；N1–N6 不纳入，联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项（C7 / C8）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: HUD 选族与 T5 不冒充本 change（S1 / S3 / §0.3 T5）

- **WHEN** 校验本 change 的能力边界与路线图 §3 / §0.3 T5 的承接关系
- **THEN** 4 族选择仅经装配入口参数与只读 HUD 派生表达、无独立 HUD 玩法交付；不启用内核 `run_ai`、不实现 AI 命令注入 lockstep 帧流的接合；**Q4 已拍板本期含全量、Q1' 已拍板不死 + 暗夜双族全量（暗夜夜间隐身 / 昼夜 / 隐身反隐等按 Q2 / Q4 本期纳入，不再写「后续 change 承接」）**，1.0 字面「一并做完」与本期双族全量的张力按 Q4' 已拍板七处显式如实登记、不预设结论、不静默窄化用户 2026-11-14 已拍板的 1.0 范围（S1 / S3 / §0.3 T5 / R10.2-5 / Q1' / Q4' / C7 / C8）

#### Scenario: 零资产与数值不冒充经典事实（R2 / ADR 0001）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境运行十六组合装配
- **THEN** 零文件 IO、零资产 / 依赖入库；不死 / 暗夜「更肉 / 更快」等差异是 `[取舍]` 合成原创 `[待核]` 常量相对大小、不复制参考项目 rawcode / 单位名 / 数值、不冒充经典事实；两套昼夜常量口径未统一与全量机制 B 类 `[待核]` 数值按 Q3' 已拍板由人核对（实施期数值核对、非规格门待拍板范围 / 架构 Q）、不静默改 `WC3-MELEE-RULES.md`（R2 / §14 / ADR 0001 / ADR 0004 / Q3' / C6）

#### Scenario: N1–N6 不纳入且 1.0 能力不作不做项（§0）

- **WHEN** 核对本 change 的 1.0 范围口径
- **THEN** N1–N6（尤其 N5 JASS / wtg VM、N6 资产入库）不纳入；联机 / 回放 / AI 玩家 / 迷雾 / 暗夜夜间隐身（作为 1.0 已拍板范围、本期按 Q4 纳入而非删减；Q4' 张力保留）不是不做项（§0 / R12.7 / R10.2-5 / Q4' / C7）
