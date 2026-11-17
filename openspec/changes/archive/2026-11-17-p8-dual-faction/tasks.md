# Tasks

> 按 `design.md` D1–D8 分组；`crates/w3-core/` 零 diff、零资产、不加新定义则 `defs_hash` / `content_hash` 不变、`SNAPSHOT_VERSION` 仍 3（ADR 0001 / ADR 0002 / C1 / C6）。每组自带验证与证据要求；玩法任务引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 1.0 范围 S1 / S3。本 change 对称于已归档 P8.1 / P8.2，站在两族分段定义、按族只读分派、R1.4 分层判据与外部帧冻结 `AiScript` 已归档结论之上、不重开。证据锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号，不写精确 `:NN` 行号（本轮 `w3-client` 代码会被 owner 改动）。

## 1. 双族独立参数装配解耦（D1 / C2）

- [x] 1.1 在 `crates/w3-client/src/melee/setup.rs::MeleeSetup` 新增 `with_races(local_race, ai_race)` 双参数装配入口、增 `ai_race` 一类装配层只读字段（`race` 继续作本地族别名、不破坏现有 `.race` 断言）；`def_ids` 按席位装配（本地 `local_race` 段、AI `ai_race` 段、中立恒 `9xx`）；验证：2×2 四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）各装配成功、本地 / AI 席位段位断言全绿、同族组合双方段位一致（R2 / R12.7 / C2 / ADR 0002）
- [x] 1.2 保留 `with_race(race)` 为 `with_races(race, race.opponent())` 兼容别名、`MeleeSetup::new()`（本地人族 / AI 兽族）与既有入口逐字节不变；验证：`setup.rs::race_does_not_change_authoritative_spawn_commands`、`setup.rs::race_fallback_matches_human_segment_exactly`、`melee_loop.rs::race_fallback_matches_human_segment_in_full_match` 仍全绿、既有入口行为不改（R12.7 / C2 / C4）
- [x] 1.3 校验 `CommandKind::Spawn` 不含 DefId、四组合装配的 `Spawn` / `gameplay_commands` / `kinds` 字节与基线逐字节一致，双族参数只停装配层 `EntityId→DefId` 关联；MUST NOT 新增 `Player.race` / `RtsMatch.race` 等权威状态字段；验证：`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍 3、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变（ADR 0002 §6-8 / C1 / C2 / 主 `melee-match/spec.md`「内核零改动与确定性契约」）
- [x] 1.4 校验零新增定义 / 零资产 / 零文件 IO：本 change 不加 `UnitDef` / `AbilityDef` / `UpgradeDef` / `EffectKind` / `GameplayCommandKind` / `CommandKind` 变体，不复制 Warsmash rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 abilId，不读 `config.toml` / 导出 JSON、不新增依赖；验证：`synthetic_definitions()` 条目数不变（28 `UnitDef` / 3 `AbilityDef` / 5 `UpgradeDef`）、`defs_hash` / `content_hash` 不变、仓库文本搜索无上述 rawcode 字面、无 `.mpq` / `.blp` / `.slk` 派生后缀（R2 / §14 / ADR 0001 / ADR 0004 / C1 / C6）

## 2. AI 命令源按 AI 自身种族产出（D2 / C3）

- [x] 2.1 改 `crates/w3-client/src/melee/ai.rs::AiScript::new`：把训练目标 `category_def_ids`、`hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade` 全部改由 **`ai_race`** 取用（不再是 `setup.race` 本地族），复用 `defs.rs::Race` 按族只读分派、不新增定义层语义；验证：四组合各枚举 `AI_PLAYER` 的 `Train` / `Cast` / `Research` 命令，其 DefId 落在 `ai_race` 段、MUST NOT 泄漏本地族段目标 ID（R12.7 / R2 / R4.3 / R4.4 / R6 / C3）
- [x] 2.2 把 `setup.rs::MeleeSetup::category_def_ids` 区分为本地 / AI 两份（如 `local_category_def_ids` / `ai_category_def_ids` 或 `category_def_ids_for(race)`）：本地验收与 `category_movability` 用本地份、`AiScript` 用 AI 份，两份 MUST NOT 串用；验证：异族组合两份互不相交、同族组合段位一致、`setup.rs::category_bindings_are_segmented_without_touching_spawn_bytes` 仍绿（R2 / C2 / C3）
- [x] 2.3 保持 AI 命令为外部帧冻结命令：`SetAi { enabled: false }` 关内核 `run_ai`、沿用 `GameplayCommandKind::{Gather,Train,Cast,Research,SetAi,Leave}`、按 `(execute_frame, player_id, sequence)` 升序且序列唯一、可重复构造、不启用内核 `run_ai`、不实现 AI × lockstep 接合（§0.3 T5）；验证：同一组合构造两次脚本逐字节相等、一端逐帧另一端一次推进多帧两端逐帧 `compute_state_hash()` / `capture_snapshot()` 一致（ADR 0002 / R12.2 / R12.7 / C3 / C7）
- [x] 2.4 按实际 `ai_race` 更新既有 AI 脚本族断言（`ai.rs::default_script_keeps_human_ability_and_upgrade_ids` / `ai.rs::script_contains_orc_new_category_commands`）：`MeleeSetup::new()`（本地人族 / AI 兽族）的 AI 命令落兽族段、`with_race(Race::Orc)`（本地兽族 / AI 人族）落人族段，不静默放宽断言、不冒充「默认脚本仍是人族段」；验证：更新后的断言在正式实现通过、在「AI 命令退回本地族」退化实现失败（R12.7 / C3 / C4 / `design.md` Risks）

## 3. 四组合 CPU 端到端一局与确定性闭环（D3 / C4）

- [x] 3.1 扩 `crates/w3-client/tests/melee_loop.rs` 的 `play_full_match` / `build_driver` 骨架或新增测试，以 `(local_race, ai_race)` 参数化 2×2 四组合，逐阶段断言「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁敌方主城结算 → 重开」；验证：四组合各能装配并打完一局、每阶段产生预期权威状态变化（R3.1 / R4 / R5 / R6 / R11.1 / R11.4 / R11.5 / C4）
- [x] 3.2 四组合确定性回归断言「**同一组合两实例逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致**」（不只比最终 hash）；验证：覆盖全部中间帧、四组合各有一对实例逐条相等（R12.4 / ADR 0002 / C4）
- [x] 3.3 保持既有两族基线 / 回落测试全绿、不静默改其断言：`melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes`（两族参数逐帧哈希与 `MeleeSetup::new()` 基线一致）、`race_fallback_matches_human_segment_in_full_match`、60 / 144 Hz 逐帧哈希回归；MUST NOT 断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」；验证：上述既有测试原样全绿、新回归无跨组合 / 跨基线数值相等断言（R12.4 / ADR 0002 / C4 / `design.md` Risks）
- [x] 3.4 四组合胜负结算与重开干净：`resolve_victory` 只结算一次、按稳定顺序、`victory.winner` 为本地玩家、结算帧进权威状态；重开 `victory.resolved == false` 且 entities / positions / players 视图为空、无上一局残留；验证：四组合各跑一次结算与重开、`SNAPSHOT_VERSION == 3`（R11.1 / R11.4 / R11.5 / C4）
- [x] 3.5 R1 / R13.6 分层判据沿用、本 change 不碰输入层：四组合装配下含新增建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`、位置不变，frame-0 用装配期 `UnitDef.speed_raw` 只读映射、运行时动态实体用只读 `EntityView.speed == 0`，两条不互替；验证：沿用主 `unit-orders/spec.md` 分层判据、不改写其 8 条 requirement、不回退白名单 / 不扩权威视图（R1.4 / R13.6 / C4 / D6）

## 4. 门禁、确定性复核与零 diff 验证（C1 / C2 / C3 / C8 / D4）

- [x] 4.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`；验证：退出码 0，UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格（`CODEBUDDY.md` §2）
- [x] 4.2 **重跑确定性**：`cargo test -p w3-core --test determinism`（`tests/determinism.rs`，26 项）、`cargo test --workspace`；验证：全绿；`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍 3、状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 稳定 `EntityId` 顺序 / FNV-1a / golden hash 逐字节不变（C1 / ADR 0002 / D4）
- [x] 4.3 校验「本 change 不加新定义则 `defs_hash` / `content_hash` 不变」：冻结定义集条目数 / 内容不变、`compute_defs_hash` 输出与改动前一致、`Player` / `RtsMatch` 无新增字段；验证：`defs.rs::definitions_cover_all_kinds_and_are_stable` / `defs.rs::defs_hash_is_fixed_and_content_sensitive` 仍绿 + `git diff -- crates/w3-core/` 为空双证据（C1 / D4 / 与 P8.1 / P8.2「新增定义走 defs_hash 内容变化」的差异）
- [x] 4.4 `openspec validate p8-dual-faction --strict` 与 `python tools/verify_all.py`（含 `decision memo OK` / `spec gate OK`）；验证：`is valid` 与 `Verification OK`；规格门 `status: approved`（用户 2026-11-17 指令、非 pending / 非 self-approve）、`fingerprint:` 为真值回填后 `spec_gate_check()` 不报失配、复选框归一化不触发解锁（`CODEBUDDY.md` §8 / §9）
- [x] 4.5 由只读 `critic` 按全清单（含第 7 / 10 项四件套一致性、第 11 项反向验证、第 12 项参考项目对照、第 13 项 `R-x.y` 符合性）复审**本 change 全部 diff**；验证：报告区分 blocker / residual、无空泛结论、锚可打开；`critic` 规格门 / 代码门结论如实登记、不冒充通过（C8 / `decision-memo.md` 待确认项）
- [x] 4.6 以中文描述 + Conventional Commits 英文 type 前缀本地 commit；验证：`git log -1` 中文描述首行 ≤ 72 字符（`CODEBUDDY.md` §3 / §4）；`git push` / fetch / pull / clone 由人工执行，AI 不碰远端（`CODEBUDDY.md` §4 / §10）

## 5. 参考对照与许可证边界复核（C6 / C7 / D7 / D8）

- [x] 5.1 逐锚复核 `design.md`「## 参考项目对照」：T5 三处诚实判断（godot 只有每席位 race 查表、无「本地 + 对手两个独立种族参数」；`match_bootstrap_module.gd` 对手硬编码 human；`ACCEPTANCE.md:24` / `:3` 四族 10 配对是已降级历史规划）、T4 诚实判断（godot 单一人族固定配方 AI、无「AI 按 race 查表产出不同训练 / 施法 / 研发命令」；Warsmash 无 `common.j` / `*AI*.j` / melee 按种族 AI 脚本）、T1 归因纠正（ADR 0002 硬门禁出处不是 `vera20k`）、参考缺口 G1 / G3 / **G6 / G7**、许可证边界；验证：锚逐条可打开、语义相符、`critic` 清单第 12 项复核（`UPSTREAM.md` 触发时机第 4 / 5 条）
- [x] 5.2 范围边界复核：HUD 选族可见 / 选族 UI（P8.4）、AI × lockstep 接合（§0.3 T5）、外部 `AiScript` 非内核 `run_ai`、P8.5 五个深度玩法 / 不死暗夜（P9+）/ 中文上屏 / 运行时新实体真实模型绑定 / 真窗口人工验收均不属本 change、如实登记为后续承接；验证：`proposal.md` Impact / `design.md` Goals / Non-Goals / D8 / Risks 与 `decision-memo.md` 待确认项一致、不冒充已解决（C7 / C8）
- [x] 5.3 许可证边界复核：`WarsmashModEngine`（AGPLv3）仅借语义不复制代码 / rawcode / 单位名 / 数值、不引 Java / LibGDX 依赖；`godot_warcraft3` / `rts_kernel_cs`（MIT）借结构不借实现 / `double` 路线；`vera20k`（GPLv3 + Section 7）未复制代码故附加条款不进入；验证：四项目均不复制代码进仓库、资产 / 派生数据不入库（ADR 0001 / ADR 0004）

## 6. 反向验证（改坏验红，CODEBUDDY.md §5 / C5 / D5）

- [x] 6.1 为「AI 命令族退回本地族」写退化反例：把 `AiScript::new` 退化为继续用本地族 `race` 生成 AI 命令 → 四组合中至少 2 组（人vs兽 / 兽vs人、以及同族组合若命令 ID 不同）的「AI 训练 / 施法 / 研发 DefId 落在 AI 族段」断言在缺陷态失败、正式实现通过；**对称复用** `ai.rs::reverse_validation_orc_dispatch_formal_passes_the_assertion` / `reverse_validation_orc_dispatch_degenerate_fails_the_same_assertion` / `ai.rs::placeholder_catalog()` 退化注入先例；验证：退化实现真经 `AiScript::new` 查表路径注入、两条证据可复现（R12.7 / C3 / C5）
- [x] 6.2 为「四组合装配退化为单参数 `opponent()`」写退化反例：把 `with_races` 退化为单参数 `race.opponent()` → 「人vs人 / 兽vs兽」两组 `def_ids` 段位断言在缺陷态失败、正式实现通过；验证：退化实现真经 `MeleeSetup::with_races` 装配路径注入（R2 / C2 / C5）
- [x] 6.3 为「`with_race` 兼容别名退化」写退化反例：删除别名或使其不再等于 `with_races(race, race.opponent())` → 既有回落基线测试在缺陷态失败、正式实现通过；验证：`race_fallback_matches_human_segment_in_full_match` / `race_does_not_change_authoritative_spawn_commands` 两条证据可复现（R12.7 / C2 / C5）
- [x] 6.4 汇总改坏验红证据：三类退化反例的「缺陷态失败 / 修复态通过」命令输出或测试内元断言均保留、不冒充全绿；无法构造缺陷态的情形 MUST 在 `design.md` D5 登记理由与替代证据（仅「全绿」不算完成）；由 `critic` 清单第 11 项③ 复核（`CODEBUDDY.md` §5 / C5）

## 7. 收尾、已知限制如实标注与规格锁定（C7 / C8 / D8）

- [x] 7.1 **真窗口人工验收「未执行 / 待补验」如实标注**：本轮 CPU 侧以回归 + 门禁自证，真窗口 / GPU / egui overlay 一律未人工观察，不冒充通过（沿用 P8.1 / P8.2 / `p9-command-hotkeys` 归档说明先例，`decision-memo.md` 待确认项）
- [x] 7.2 已知限制 / 范围外承接如实登记：运行时动态实体承接边界（`runtime-entity-movability` 已归档、本 change 沿用不扩视图）、运行时新实体「模型缺键回退」四面体占位（P8.4）、HUD 选族 / 双族模型键（P8.4）、P8.5 五个深度玩法 / 不死暗夜（P9+）/ 中文上屏 / AI × lockstep 接合（T5）/ 参考缺口 G1 / G3 / G6 / G7、数值 `[待核]` 沿用既有 `[取舍]`；本 change 不扩范围、不补测、不改代码（C7 / C8 / D7 / D8）
- [x] 7.3 规格锁定复核：`decision-memo.md`「## 规格门」`status: approved` + 四件套指纹真值（由 `python -B tools/verify_all.py --fingerprint p8-dual-faction` 取、不猜）+ `approved_by:` + `date: 2026-11-17` + `reason:`；apply 由 `client-owner` 执行，四件套正文零改动、`tasks.md` 复选框归一化不解锁；四件套任一 artifact 改动即视为解锁、MUST 按 `CODEBUDDY.md` §8 / §9 同步回查其余三份并重走规格门（`CODEBUDDY.md` §8 / §9）

## Workflow follow-up

- 规格门已由用户 2026-11-17 本会话指令拍板批准（等价规格门批准、`status: approved`、真值指纹回填），进入 `/opsx:apply`（对应 owner 仅 `crates/w3-client/`），再走 `critic` 代码门 → 本地 commit → 人工 push → CI 双平台 `verify_all.py` → `/opsx:archive`。
- P8.4（HUD / 表现层模型键 / 选族 UI）、P8.5 五个深度玩法与 P9+（不死 / 暗夜）、AI × lockstep 接合（§0.3 T5）各由后续独立 change 承接，本 change 不并（C8 / `docs/roadmap/P8-human-orc-melee.md` §3 / §6）。
