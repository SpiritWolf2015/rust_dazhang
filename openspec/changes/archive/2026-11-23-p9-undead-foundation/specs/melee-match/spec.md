# Spec Delta

## MODIFIED Requirements

### Requirement: 四组合双族对局的会话装配与逐帧哈希一致（R2 / R11 / R12.4 / R12.7）

系统 SHALL 支持由 2×2 四组合扩到 **4×4 十六组合**（人 / 兽 / 不死 / 暗夜 4 族各对本族、异族与同族；Q1' 已拍板「不死 + 暗夜双族全量」+ P8 两族不重做）的会话装配并各打完一局 CPU 无渲染对局（采集 → 建造 → 训练 → 英雄施法 → 研发 → **Q2 / Q4 全量机制触发** → 战斗 → 摧毁敌方主城结算 → 重开），沿用既有 `MeleeSetup::with_races` / `resolve_victory` / `AiScript` 命令面与 `EffectDef` 组合（既有 `GameplayCommandKind` 码 1–23 不动、Q2 / Q3 新增码 24 起另见 `specs/undead-faction/spec.md`）、不引入 JASS / wtg VM（N5 / R7.6 / ADR 0002）（`docs/roadmap/P8-human-orc-melee.md:148`）。`resolve_victory` MUST 只结算一次、按稳定顺序、结算帧进入权威状态（R11.1 / R11.4）。验收口径 MUST 为「同一组合两实例逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致」（R12.4 / ADR 0002）；因按 Q2 新增权威状态 + 定义层内容变化 + Q3 bump 8，MUST NOT 断言与改动前 v7 旧基线数值相等、MUST NOT 断言跨组合哈希数值相等。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「四组合」「双族」「快照版本不变」等历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合会话装配并打完一局（R11.1 / R11.4 / R12.7）

- **WHEN** 十六组合（含原四组合）各注入一份完整一局脚本并推进到敌方主城被摧毁、触发 `Leave`
- **THEN** 全部组合经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态；本地 / AI 席位 `def_ids` 落在各自族段（`1xx` / `2xx` / `3xx` / `4xx`）、中立恒 `9xx`，腐地 / 古树 / 昼夜 / 隐身目标判定 / 月亮井 / 缠绕金矿等 Q2 / Q4 机制按整数帧确定性结算、新增不死 / 暗夜组合不破坏原四组合基线（R11.1 / R11.4 / R12.7 / R2 / R10.2-4 / R1.5 / Q1' / Q2 / Q4 / Q5'）

#### Scenario: 组内逐帧哈希一致且既有两族基线不破坏（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本并逐帧推进全部阶段，同时复核既有人 / 兽两族基线测试
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、不只比最终 hash；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` MUST 按 v8 新基线如实更新后仍绿、不静默改断言、不断言跨组合或 v7 旧基线数值相等（R12.4 / ADR 0002 / Q3 / C4）

#### Scenario: 重开干净与快照版本不变（R11.5 / ADR 0002）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重新构造一局并校验快照头部
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态；**历史标题字面「快照版本不变」是身份键、不作本期行为断言**：按用户 Q2 / Q3 已明确拍板（偏离 AI 推荐、MUST NOT 写成 AI 推荐）`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` bump 7→8、`Snapshot` 新增族特色机制顶层字段并四处同步、旧 v7 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、golden hash 按 v8 新实现如实重取（R11.5 / ADR 0002 §6-8 / R6 / Q2 / Q3 / Q5'）

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5）

- **WHEN** 以退化实现（漏接不死 / 暗夜段 `defs_hash` 敏感性 / `3xx` / `4xx` 段混入 `8xxx` / `with_races` 退回单参数 `opponent()` / `race_row()` 散落白名单 / 新增建筑 `speed_raw` 改非 0 / 框选含建筑误生成 `Move` / 新增权威状态漏进 `compute_state_hash` / `Snapshot` / `first_difference()` 之一）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过、两条证据可复现（按 `CODEBUDDY.md §5` 形态①测试内反例 + 元断言防退化、退化实现真经 `capture_snapshot()` / `compute_state_hash()` / 公开命令入口生产路径注入、不冒充形态②）；含建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6 / ADR 0002 §8 / Q1' / Q2 / Q3 / `openspec/config.yaml:57-58`）
