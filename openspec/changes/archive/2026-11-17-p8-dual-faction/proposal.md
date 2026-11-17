# Proposal

## Why

P8 路线图 `docs/roadmap/P8-human-orc-melee.md` §3 把 **P8.3「双族对抗与 AI」** 定为两族 CPU 闭环都绿之后的串行下一步（§6 第 4 条「P8.3 在两族 CPU 闭环都绿之后开」）；用户 2026-11-17 本会话指令「我已执行 git push 完毕。GitHub Action 执行成功（run 44029882988）。现在请你归档并继续开发 P8.3 双族对抗」是**人对方向（P8.3 双族对抗）与实施的明确拍板**，等价规格门批准（与 P8.1「用户指令作为批准」、P8.2 / `p9-command-hotkeys` 先例一致），如实登记见 `decision-memo.md`「## 规格门」。本 change 对称于已归档 `2026-11-16-p8-human-faction`（P8.1 人族完整化）与 `2026-11-17-p8-orc-faction`（P8.2 兽族完整化），站在两族 `Race::{Human,Orc}` 分段定义、按族只读分派与 CPU 一局闭环之上，不重开、不复活已消解结论。

现状（只读核实，锚一律「路径 + 函数名 / 符号名」，不写精确 `:NN` 行号——本轮 `w3-client` 代码会被 owner 改动，P8.1 有行号漂移返工教训）：`crates/w3-client/src/melee/setup.rs::MeleeSetup::with_race` 仍是**单参数**入口（`local_race = race`、`ai_race = race.opponent()`）、`MeleeSetup` 只存 `race` 一个字段（= 本地族）、`category_def_ids` 只由本地族派生一份，故只能表达「人 vs 兽 / 兽 vs 人」两组合，路线图 §3 要求的**四组合（人vs人 / 人vs兽 / 兽vs人 / 兽vs兽）** 无法装配。更关键：`crates/w3-client/src/melee/ai.rs::AiScript::new` 以 `setup.race`（本地族）+ 本地族 `category_def_ids()` 取训练目标 / `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade`，生成的是 `AI_PLAYER`（player 2）的命令；而 AI 实体却按 `ai_race = race.opponent()` 段装配——**AI 命令族与 AI 实体族错配**。P8.1 / P8.2 单族对称闭环未暴露，P8.3 四组合（尤其同族组合）必然暴露。本 change 修此错配、把双族独立参数落到装配层（R12.7 AI 玩家全功能属 1.0 必做 / R2 四族不对称 / R1.4 可移动性分层判据 / R11.1 / R11.4 / R11.5 胜负结算与重开 / R12.2 / R12.4 命令帧与回放逐帧哈希 / R13.6 确定性契约）。

本 change 落在 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 个种族功能做完）的**过程分期**（P8.3 两族 CPU 闭环、P9+ 补不死 / 暗夜收敛四族）、不新增范围编号（C7）。按 `docs/references/WC3-MELEE-RULES.md` 的 R1.4 / R2 / R11 / R12.2 / R12.4 / R12.7 / R13.6 落地；**N1–N6 一律不纳入**（尤其 N5 JASS / wtg VM、N6 资产入库）；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项。`crates/w3-core/` 零 diff、零资产（ADR 0001 / ADR 0002）。**四组合验收口径**：断言「同一组合两实例逐帧 `compute_state_hash()` / `capture_snapshot()` 字节一致」——双族参数只停装配层、四组合的权威 `Spawn` 初值相同；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` / `melee_loop.rs::race_fallback_matches_human_segment_in_full_match` MUST 仍绿、不静默改其断言；MUST NOT 断言「四组合之间哈希数值相等」或「与改动前旧基线数值相等」（本 change 不加新定义，`defs_hash` / `content_hash` 不变）。

## What Changes

- **双族独立参数解耦（装配层、不新增权威状态）**：新增 `with_races(local_race, ai_race)`（或等价双参数入口）把本地 / AI 种族解耦；**保留 `with_race(race)` 为 `with_races(race, race.opponent())` 兼容别名**，`MeleeSetup::new()`（本地人族 / AI 兽族）与既有入口行为逐字节不变。`MeleeSetup` 增 `ai_race` 一类**装配层只读**字段（`race` 继续作本地族别名、不破坏现有 `.race` 断言）；`def_ids` 按席位装配（本地用 `local_race` 段、AI 用 `ai_race` 段、中立资源恒 `9xx`）。`CommandKind::Spawn` 不含 DefId、字节逐字节不变（两族 / 四组合装配字节一致），`setup.rs::race_does_not_change_authoritative_spawn_commands` 回落测试仍绿（C2）。
- **AI 命令源按 AI 自身种族产出**：`ai.rs::AiScript::new` 现状用本地族生成 `AI_PLAYER` 命令、与按 `opponent()` 段装配的 AI 实体错配；本 change MUST 改为按 **`ai_race`** 取 `category_def_ids`（训练目标）/ `hero_ability` / `harass_ability` / `weapons_upgrade` / `armor_upgrade`，使 AI 训练 / 施法 / 研发的 DefId 落在 AI 族段；复用既有 `Race` 按族只读分派、不新增定义层语义；`category_def_ids` 需区分本地 / AI 两份（如 `local_category_def_ids` / `ai_category_def_ids` 或 `category_def_ids_for(race)`），本地验收用本地份、AI 脚本用 AI 份。仍是外部帧冻结命令（`SetAi { enabled: false }` 关内核 `run_ai`）、按 `(execute_frame, player_id, sequence)` 稳定排序、不新增权威状态（C3 / R12.7）。
- **四组合 CPU 端到端 + 确定性闭环**：以 `(local_race, ai_race)` 参数化 2×2 = 人vs人 / 人vs兽 / 兽vs人 / 兽vs兽 四种组合，各能装配并打完一局（采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁敌方主城结算 → 重开），`resolve_victory` 只结算一次、重开无残留、`SNAPSHOT_VERSION == 3`（R11.1 / R11.4 / R11.5）。验收口径见 Why：只断言**同一组合两实例逐帧哈希 / 快照字节一致**，既有两族基线 / 回落测试保持全绿、不静默改其断言，不断言四组合之间或与旧基线数值相等（C4 / R12.4）。
- **反向验证（改坏验红，`CODEBUDDY.md` §5）**：至少覆盖 ①「AI 命令族退回本地族」、②「四组合装配退化为单参数 `opponent()`」（人vs人 / 兽vs兽 段位断言失败）、③「`with_race` 兼容别名退化」（既有回落基线测试失败）；退化实现真经 `AiScript::new` / `MeleeSetup::with_races` / apply 路径注入、非旁路模拟；仅「全绿」不算完成（C5）。
- **三条硬边界**：`crates/w3-core/` 零 diff、`Spawn` 字节不变、本 change **不加新 `UnitDef` / `AbilityDef` / `UpgradeDef`** 故 `defs_hash` / `content_hash` 也不变（区别于 P8.1 / P8.2 的「新增定义是冻结定义内容变化」）；`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变（C1）。
- **范围边界（防过度承诺）**：路线图 §3 字面「玩家可选族（入口参数 / HUD）」中的 **HUD 选族可见 / 选族 UI 属 P8.4**，本 change 只做「入口参数」层的双族装配解耦 + CPU 四组合闭环；路线图字面「`run_ai` 按种族」在本项目实际是**外部帧冻结 `AiScript` + `SetAi { enabled: false }` 关内核 `run_ai`**（架构取舍如实登记、不写成启用内核 `run_ai`）；**AI × lockstep 命令接合（`WC3-MELEE-RULES.md` §0.3 T5 待确认项）不属本 change**、由后续独立 change 承接（C7 / C8）。
- **BREAKING：无**。既有 6 个基线定义 `1..=6`、人族 `1xx` / 兽族 `2xx` / 中立 `9xx` 的 DefId 与既有入口行为不变；本 change 不改任何 `FrozenDefinitions` 内容、不改 `SNAPSHOT_VERSION`、状态哈希字段集合、快照头部布局、`first_difference()` 与命令排序键（C1 / C2）。

## Capabilities

### New Capabilities

- `dual-faction`: 双族对战的装配层独立参数解耦（本地 / AI 两个独立种族、兼容别名、`ai_race` 装配层只读字段）、AI 命令源按 AI 自身种族分派（`category_def_ids` 两份）、四组合 2×2 CPU 端到端一局与确定性闭环、反向验证（改坏验红），以及「双族参数不进权威状态 / 哈希 / 快照、不加新定义则 `defs_hash` 不变」「HUD 选族属 P8.4、AI × lockstep T5 承接、外部 `AiScript` 非内核 `run_ai`」的 ADR 0002 / ADR 0001 / 1.0 范围边界。对称于已归档 `human-faction` / `orc-faction` capability，但主体是**装配层参数解耦**而非新增定义。

### Modified Capabilities

- `melee-match`: 增加一条 **ADDED Requirement**——四组合双族对局的会话装配与逐帧哈希一致（含「组内两实例逐帧哈希一致、既有两族基线不破坏、胜负只结算一次 / 重开干净」的可执行验证）；**既有 8 条 requirement 不改写**。选它而非 `unit-orders`：本 change 不碰输入翻译层，R1 可移动性沿用既有 `unit-orders` 分层判据、不改写（`specs/dual-faction/spec.md` 的 Scenario 显式声明），把「一局会话」的四组合契约补在 `melee-match` 更贴合主 specs 现状、且与新 capability 不重名。

## Impact

- **代码面**：仅 `crates/w3-client/src/melee/{setup,ai}.rs` 与 `crates/w3-client/tests/melee_loop.rs`（或新测试文件）；**`crates/w3-core/` 零 diff**（`UnitDef` / `FrozenDefinitions` / `CommandKind` / `GameplayCommandKind` / `EffectKind` / `EntityView` / `snapshot.rs` 均不修改），`w3-render` / `w3-net` / `w3-relay` / `Cargo.lock` 零改动。`defs.rs::Race` 已具备按族只读分派，本 change 复用、不改定义层；四组合装配只改 `MeleeSetup` 的装配层只读字段与 `def_ids` 席位关联。
- **确定性契约（ADR 0002）**：不新增权威状态字段（`Player.race` / `RtsMatch.race` 等）/ `CommandKind` / `GameplayCommandKind` / `EffectKind` 变体 / 事件；双族参数是**装配层只读数据**、不进权威状态 / 状态哈希 / 快照；`SNAPSHOT_VERSION`（`snapshot.rs::SNAPSHOT_VERSION`）仍 3；状态哈希字段集合、快照头部布局、`first_difference()`、30 Hz 整数帧、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键、稳定 `EntityId` 顺序、FNV-1a / golden hash 逐字节不变；本 change 不加新定义，故 `compute_defs_hash` / `content_hash` 也不变。`cargo test -p w3-core --test determinism` 与 `cargo test --workspace` MUST 全绿、`git diff -- crates/w3-core/` 为空。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产、零 MPQ 派生数据、零真实导出 JSON、零文件 IO、零 `config.toml` 读取、零新增依赖；本 change 不复制 Warsmash rawcode（`AOwk` / `Hamg` / `ogre` / `opeo`）/ godot 原创三族单位名 / 暴雪 abilId；两族「更肉 / 更快」等差异是既有 `[取舍]` 合成原创常量相对大小、非经典事实。四个参考项目只读、不复制代码（见 `design.md`「## 参考项目对照」）。
- **ADR 与阶段**：相关 ADR 0001 / 0002 / 0003（范围与分期）/ 0004 / 0005。阶段 **P8.3**（`docs/roadmap/P8-human-orc-melee.md` §3）；`openspec/config.yaml` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 / P8.1 / P8.2 一致）。本 change 落在 S1 / S3 的过程分期；S2（回放）/ S4（道具）/ S5（野怪）/ S6（中立建筑）不在本 change 交付但属 1.0 范围。
- **范围外承接（后续独立 change，C8）**：HUD / 选族 UI / 表现层模型键（P8.4）、P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、不死暗夜（P9+）、中文上屏（CJK 字体许可证人工拍板）、运行时新实体真实模型绑定（模型缺键回退已知限制）、真窗口人工验收、AI × lockstep 命令接合（§0.3 T5）。**联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项**（C7）；N1–N6 不纳入，尤其 N5（JASS / wtg VM）与 N6（资产入库）。
- **人工边界**：真窗口人工验收不在本轮执行（CPU 侧以回归 + 门禁自证），如实标注「未执行 / 待补验」；`critic` 规格门 / 代码门待复核（见 `decision-memo.md`）；push / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。本轮成文不写 `crates/` 代码、不 commit、不改 `## 规格门` 以外的批准状态。
