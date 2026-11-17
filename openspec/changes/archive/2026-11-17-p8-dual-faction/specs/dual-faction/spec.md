# Spec Delta

## Purpose

为 P8.3 双族对抗定义行为契约：本地与 AI 两个独立种族参数在装配层解耦、AI 命令源按 AI 自身种族产出、四种族组合 CPU 端到端一局的确定性闭环，全部为装配层只读数据、不新增权威状态字段、`crates/w3-core/` 零 diff、守住 ADR 0002 快照与哈希契约及 1.0 范围 S1 / S3。

## ADDED Requirements

### Requirement: 本地与 AI 双族独立参数装配（R2 / R12.7）

系统 SHALL 提供 `with_races(local_race, ai_race)` 双参数装配入口，把本地 / AI 种族解耦为两个独立参数；`MeleeSetup` SHALL 增 `ai_race` 一类**装配层只读**字段，`race` 继续作本地族别名、不破坏现有 `.race` 断言。系统 MUST NOT 新增 `Player.race` / `RtsMatch.race` 等运行时权威状态字段。四组合的 `def_ids` SHALL 按席位装配：本地席位用 `local_race` 段、AI 席位用 `ai_race` 段、中立资源恒 `9xx`（R2 / R12.7）。细节见各 `#### Scenario:`。

#### Scenario: 四组合装配段位正确（R2）

- **WHEN** 以 `(local_race, ai_race)` 的 2×2 四种组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）分别装配开局
- **THEN** 本地席位实体 `def_ids` 落在 `local_race` 段（`1xx` 或 `2xx`）、AI 席位落在 `ai_race` 段、中立资源恒 `9xx`，且「人vs人」「兽vs兽」两个同族组合的双方段位分别一致、不与 `1..=6` 基线冲突（R2 / C2）

#### Scenario: 保留 with_race 兼容别名与既有入口逐字节不变（R12.7）

- **WHEN** 分别以 `with_race(race)` 与 `with_races(race, race.opponent())`、`MeleeSetup::new()` 装配开局并比对命令字节
- **THEN** 三者 `Spawn` / `gameplay_commands` / `kinds` / `def_ids` 逐字节一致，`setup.rs::race_does_not_change_authoritative_spawn_commands` 与 `melee_loop.rs::race_fallback_matches_human_segment_in_full_match` 仍绿（R12.7 / C2 / 主 `melee-match/spec.md`）

#### Scenario: 双族参数不进权威状态与哈希快照（ADR 0002）

- **WHEN** 校验四组合装配的权威状态字段集合与 `SNAPSHOT_VERSION`
- **THEN** 无 `race` / `ai_race` 或任何新增权威状态字段，`SNAPSHOT_VERSION` 仍为 3、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变，双族参数只停装配层 `EntityId→DefId` 关联（ADR 0002 §6-8 / C1 / C2）

#### Scenario: 四组合装配下建筑与资源点不响应 Move（R1.4 / R13.6）

- **WHEN** 四组合任一装配下框选含主城 / 金矿 / 树木 / 新增建筑的混合选中集合并下达移动命令
- **THEN** 沿用既有 `unit-orders` 的 R1.4 / R13.6 实体来源分层判据、不改写：frame-0 装配实体用装配期 `UnitDef.speed_raw` 只读映射、运行时动态实体用只读 `EntityView.speed == 0` 判不可移动，不可移动实体 MUST NOT 出现在 `Move` 生成集合、位置不变（R1.4 / R13.6 / C4）

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

系统 SHALL 支持 2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）各装配并打完一局 CPU 无渲染对局（采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁敌方主城结算 → 重开），每阶段 MUST 可执行验证；`resolve_victory` MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态（R11.1 / R11.4 / R11.5）。验收口径 MUST 为「**同一组合两实例逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致**」（R12.4 / ADR 0002）。

#### Scenario: 四组合各能打完一局并分出胜负（R11.1 / R11.4）

- **WHEN** 四种组合各注入一份完整一局脚本并推进到敌方主城被摧毁、触发 `Leave`
- **THEN** 四种组合均经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态（R11.1 / R11.4 / C4）

#### Scenario: 组内两实例逐帧哈希与快照一致（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本并逐帧推进全部阶段
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、不只比最终 hash；因双族参数只停装配层、四组合的权威 `Spawn` 初值相同，既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes`（两族参数逐帧哈希与 `MeleeSetup::new()` 基线一致）与 `race_fallback_matches_human_segment_in_full_match` MUST 仍绿、不静默改其断言（R12.4 / ADR 0002 / C4）

#### Scenario: 不断言跨组合或旧基线哈希数值相等（ADR 0002）

- **WHEN** 编写四组合确定性回归断言
- **THEN** MUST NOT 断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」；本 change 不加新定义、`defs_hash` / `content_hash` 不变，故无需调整旧基线数值（若后续 change 新增定义才按「同脚本两实例逐帧一致」口径调整）（ADR 0002 / C1 / C4 / `design.md` Risks）

#### Scenario: 重开无残留（R11.5）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重新构造一局
- **THEN** 新一局 `victory.resolved == false`、entities / positions / players 视图为空、不含上一局任何权威状态；`SNAPSHOT_VERSION == 3`（R11.5 / C4）

#### Scenario: SNAPSHOT_VERSION 与 golden hash 不变（ADR 0002）

- **WHEN** 四组合打完一局后校验内核零 diff 与快照头部
- **THEN** `git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍 3、`cargo test -p w3-core --test determinism`（26 项）全绿、状态哈希字段集合 / 快照头部布局 / `first_difference()` / FNV-1a / golden hash 逐字节不变（ADR 0002 §6-8 / C1）

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

本 change SHALL 只做「入口参数」层的双族装配解耦 + CPU 四组合闭环；HUD 选族可见 / 选族 UI（P8.4）、P8.5 五个深度玩法、不死暗夜（P9+）、中文上屏、运行时新实体真实模型绑定、真窗口人工验收、AI × lockstep 命令接合（`WC3-MELEE-RULES.md` §0.3 T5）MUST NOT 纳入本 change。AI 命令源 MUST 是外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`、非启用内核 `run_ai`。本 change 落在 S1 / S3 的过程分期、不新增范围编号，N1–N6 不纳入（C7 / C8）。

#### Scenario: HUD 选族与 T5 不冒充本 change（S1 / S3 / §0.3 T5）

- **WHEN** 校验本 change 的能力边界与路线图 §3 P8.4 / §0.3 T5 的承接关系
- **THEN** 双族选择仅经装配入口参数表达、无 HUD 选族 UI 交付；不启用内核 `run_ai`、不实现 AI 命令注入 lockstep 帧流的接合；这些如实登记为后续独立 change 承接、不冒充已解决（S1 / S3 / §0.3 T5 / C7 / C8）

#### Scenario: 零资产与数值不冒充经典事实（R2 / ADR 0001）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境运行四组合装配
- **THEN** 零文件 IO、零新增定义 / 依赖 / 资产，两族「更肉 / 更快」差异是既有 `[取舍]` 合成原创常量相对大小、不复制参考项目 rawcode / 单位名、不冒充经典事实（R2 / §14 / ADR 0001 / ADR 0004 / C6）

#### Scenario: N1–N6 不纳入且 1.0 能力不作不做项（§0）

- **WHEN** 核对本 change 的 1.0 范围口径
- **THEN** N1–N6（尤其 N5 JASS / wtg VM、N6 资产入库）不纳入；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项（§0 / R12.7 / C7）
