# Proposal

## Why

P8 路线图 `docs/roadmap/P8-human-orc-melee.md` §3（用户 2026-11-14 拍板「人族 + 兽族做完」且含 P8.5 全量；2026-11-15 指令「做 1.0 游戏业务，别扯不重要的事」）把 P8.1 定为人族完整化。P8.0 地基（`p8-faction-foundation`，已归档 2026-11-15）只交付了人族段 `101..104` 四个 `UnitDef`（农民 / 士兵 / 主城 / 兵营）、兽族段 `201..204`、中立 `9xx`，以及各 1 个 `AbilityDef` / `UpgradeDef` / `ItemDef`；人族尚缺远程单位（R5.1 穿刺 / R5.3 克制）、第三战斗单位、人口建筑（R3.4）、英雄祭坛（R4.2）、研发建筑（R4.4）与英雄（R6），无法打出一局含「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 结算 → 重开」的完整 Melee（R11.1 / R11.4 / R11.5）。本 change 落在 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/**S3**（4 个种族功能做完）的过程分期，按 `docs/references/WC3-MELEE-RULES.md` 的 R1.4 / R2 / R3.4 / R4.1–4.6 / R5 / R5.3 / R6 / R11 / R13.6 补齐**合成 / 原创编译期整数常量**的人族最小可玩集，且 `crates/w3-core/` 零 diff、零资产（ADR 0001 / ADR 0002）。**N1–N6 一律不纳入**；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」，不是不做项（C7）。

## What Changes

- `crates/w3-client/src/melee/defs.rs` 人族段 `100..200` 新增 `UnitDef`（全部为合成整数常量）：**远程单位**（穿刺 / 长射程，R5.1 / R5.3）、**反重甲第三战斗单位**（R5.3）、**人口建筑**（R3.4）、**英雄祭坛**（R4.2）、**研发 / 科技建筑**（铁匠铺类，R4.4）、**1 名英雄**（`mana > 0`，R6）。所有新增建筑 `speed_raw == 0`，供 `derive_movability` 派生 R1.4 / R13.6 可移动性过滤。DefId 落在 `100..200`、不与 `101..104` / `201..204` / `9xx` 冲突。
- 新增 **1 个 `AbilityDef` 主动技能**：只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` + `EffectDef` 组合表达（R6 / R7.6），走既有 `Cast` 命令面；**不新增 `EffectKind`、不新增命令变体、不引入 JASS / wtg VM（N5）**。新增 **1–2 项 `UpgradeDef` 攻 / 防升级**（`damage_mod` / `hp_mod` / `research_frames`），经既有 `Research` 路径接通（R4.4）。
- `Race` 分派扩展到新类别（如 `ranged()` / `siege()` / `farm()` / `altar()` / `smithy()` / `hero()`，命名由实现定），MUST 是**定义层 / 装配层**分派；**MUST NOT 新增 `Player.race` 之类运行时权威状态**（C1 / 主 `melee-match/spec.md:127-129`）。
- `crates/w3-client/src/melee/setup.rs` 扩展人族开局的 `def_ids` 关联；**`CommandKind::Spawn` 字节逐字节不变**（P8.0 已定 `Spawn` 不含 DefId，`command.rs:14-21`），种族只经装配层 `EntityId→DefId` 关联（C2）。
- `crates/w3-client/src/melee/ai.rs` 扩展 `AiScript`：会训练新单位 / 施放英雄技能 / 研发升级，沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`（R12.7 / R4.3 / R4.4 / R6），仍为外部帧冻结命令、不新增权威状态。
- `crates/w3-client/tests/melee_loop.rs`（或新测试）：CPU 端到端一局覆盖「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开无残留」各阶段（R3.1 / R4 / R5 / R6 / R11.1 / R11.4 / R11.5），逐帧哈希与基线一致（ADR 0002）；并补 **R1.4 可移动性回归**与**反向验证（改坏验红）**（C4 / C5）。
- **BREAKING：无**。既有 6 个基线定义 `1..=6`、人族 `101..104` / 兽族 `201..204` / 中立 `9xx` 的 DefId 与既有入口行为不变；新增定义只改变 `FrozenDefinitions` 的内容哈希（`defs_hash` / `content_hash`），不改 `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合、快照头部布局、`first_difference()` 与命令排序键（C3）。

## Capabilities

### New Capabilities

- `human-faction`: 人族完整化的冻结定义（远程 / 反重甲 / 人口 / 祭坛 / 研发 / 英雄单位、1 主动技能、攻防升级、`Race` 分派）、开局 `def_ids` 装配、AI 命令脚本扩展，以及 CPU 端到端一局的确定性闭环；含「新增定义是冻结定义内容变化、不是新权威状态字段」的 ADR 0002 边界。

### Modified Capabilities

- `unit-orders`: 增加一条 **ADDED Requirement**——人族新单位 / 建筑的 R1.4 可移动性过滤（新增建筑 / 资源点 / 主城 MUST NOT 出现在 `Move` 的 Scenario）与端到端一局各阶段的可执行验证；既有「命令校验 / 确定性移动积分 / 同帧攻击结算 / 群体移动只作用于可移动单位」四条 requirement 不改写。

## Impact

- **代码面**：仅 `crates/w3-client/src/melee/{defs,setup,ai}.rs` 与 `crates/w3-client/tests/melee_loop.rs`（或新测试文件）；**`crates/w3-core/` 零 diff**（`UnitDef` / `FrozenDefinitions` / `WorldView` / `EntityView` / `CommandKind` / `GameplayCommandKind` / `snapshot.rs` 均不修改），`w3-render` / `w3-net` / `w3-relay` / `Cargo.lock` 零改动。
- **确定性契约（ADR 0002）**：不新增权威状态字段；新增定义是**冻结定义内容变化**，经 `compute_defs_hash`（`crates/w3-core/src/gameplay.rs:144-181`）进入 `defs_hash` / `content_hash`（`:212-215`），写入由 `SNAPSHOT_VERSION`（当前 3，`snapshot.rs:15`）定义的快照头部。状态哈希字段集合、快照头部布局、`first_difference()`、30 Hz 整数帧、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键、稳定 `EntityId` 顺序、FNV-1a 哈希一律不变；`cargo test -p w3-core --test determinism`（26 项）与 `cargo test --workspace` MUST 全绿，`git diff -- crates/w3-core/` 为空，逐帧状态哈希与 `capture_snapshot()` 字节与基线一致。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产、零 MPQ 派生数据、零真实导出 JSON；全部为合成 / 原创编译期整数常量，数值 `[待核]`、以本机导出 SLK 核对、不入库、不冒充原版（C6）。四个参考项目只读、不复制代码：`WarsmashModEngine`（AGPLv3）仅借鉴按种族 / 英雄分组的**语义**，`godot_warcraft3` / `rts_kernel_cs`（MIT）不借原创单位名 / 暴雪 rawcode，`vera20k`（GPLv3 + Section 7）逐帧 checksum 思路仅作参照；见 `design.md`「## 参考项目对照」。
- **ADR 与阶段**：相关 ADR 0001 / 0002 / 0003（范围与分期）/ 0004 / 0005。阶段 **P8.1**（`docs/roadmap/P8-human-orc-melee.md` §3）；`openspec/config.yaml:50` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 一致）。本 change 落在 S1 / S3 的过程分期；S2（回放）/ S4（道具）/ S5（野怪）/ S6（中立建筑）不在本 change 交付但属 1.0 范围。
- **范围外承接（A / 后续独立 change，C8）**：兽族完整化（P8.2）、双族对抗与 AI 选族（P8.3）、HUD / 表现层模型键（P8.4）、**P8.5 五个深度玩法**（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）。**联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项**（C7）；N1–N6 不纳入，尤其 N5（JASS / wtg VM）与 N6（资产入库）。会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）本 change 不做，属 P8.5，MUST 另走 ADR 0002 流程、不得借「零改动」旁路。
- **人工边界**：规格门由人拍板（`decision-memo.md` 的 `status: pending`，不 self-approve）；真窗口人工验收不在本轮执行（CPU 侧以回归 + 门禁自证），如实标注「未执行 / 待补验」；push / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。
