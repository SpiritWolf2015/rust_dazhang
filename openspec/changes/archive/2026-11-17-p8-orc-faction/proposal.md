# Proposal

## Why

P8 路线图 `docs/roadmap/P8-human-orc-melee.md` §3 把 **P8.2「兽族完整化（比 `godot_warcraft3` N4-C 更完整）」** 定为 P8.1 的串行下一步（用户 2026-11-14 拍板「人族 + 兽族做完」、§6 第 4 条「P8.1 与 P8.2 串行」）；用户 2026-11-17 本会话指令「我已执行 git push 完毕。GitHub Action 执行成功…… 现在请你归档并继续开发下一步 P8.2 兽族」是**人对方向（P8.2 兽族完整化）与实施的明确拍板**，等价于规格门批准（与 P8.1「用户指令作为批准」、`runtime-entity-movability` / `p9-command-hotkeys`「指令 + 方案拍板」先例一致），如实登记见 `decision-memo.md`「## 规格门」。

现状（只读核实）：P8.0 地基（`p8-faction-foundation`，已归档）只交付人族段 `101..104` / 兽族段 `201..204` / 中立 `9xx` 的分段占位；P8.1（`p8-human-faction`，已归档）已把人族段补齐到 `101..110`（新增 `HUMAN_RANGED/SIEGE/FARM/ALTAR/SMITHY/HERO` 六类 `105..110`）、1 个英雄主动技能（`HERO_ABILITY 11`）、2 项攻防升级（`UPGRADE_WEAPONS 21` / `UPGRADE_ARMOR 22`）、`Race` 六类分派、开局 `def_ids` 装配与 CPU 端到端一局。**兽族段当前仅 `ORC_PEASANT 201` / `ORC_SOLDIER 202` / `ORC_TOWN_HALL 203` / `ORC_BARRACKS 204` 四个占位**；`Race::{ranged,siege,farm,altar,smithy,hero,category_def_ids}` 对 `Orc` 六个新类别方法仍回落到 `ORC_SOLDIER` / `ORC_BARRACKS` 占位（`defs.rs` 注释明确「P8.2 完整化时替换」），兽族无英雄、无主动技能、无攻防升级差异，无法打出一局含「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 结算 → 重开」的完整 Melee（R11.1 / R11.4 / R11.5）。本 change 站在 `p9-command-hotkeys`（R13 操作层，键盘 `M` / `A` 命令复用 R1.4 过滤）与 `runtime-entity-movability`（R1.4 / R13.6 **实体来源分层判据**：frame-0 装配期 `UnitDef.speed_raw` 派生只读映射为权威、运行时 `Train` / `Build` 动态实体用只读 `EntityView.speed == 0` 判不可移动）两条已归档结论之上，不重开、不复活已消解的过时禁令。

本 change 落在 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 个种族功能做完）的**过程分期**（P8.2 兽族竖切是四族过程前半段，P9+ 补不死 / 暗夜才收敛四族，C7），按 `docs/references/WC3-MELEE-RULES.md` 的 R1.4 / R2 / R3.4 / R4.1–4.6 / R5 / R5.1 / R5.3 / R6 / R11 / R13.6 补齐**合成 / 原创编译期整数常量**的兽族最小可玩集，且 `crates/w3-core/` 零 diff、零资产（ADR 0001 / ADR 0002）。**「兽族与人族数值有差异（更肉 / 更快 / 更低远程稳定度）」是 `[取舍]` 合成原创常量相对大小表达、不是经典事实**：四项目都**没有**给出「经典 WC3 兽族 vs 人族」的数值或结论，「两族不对称、兽族重甲肉搏 / 正面强 / 地洞兼人口与防御」是本项目业务规则 R2 / R3.4 / R4.4 的 `[已核]` 规则性事实，具体数值差异是 `[取舍]`（见 `design.md`「## 参考项目对照」两处归因纠正 / G5）。**N1–N6 一律不纳入**（尤其 N5 JASS / wtg VM、N6 资产入库）；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项（C7）。

## What Changes

- `crates/w3-client/src/melee/defs.rs` 兽族段 `200..300` 新增六类 `UnitDef`（全部为合成整数常量）：**远程 / 猎头类单位**（R5.1 / R5.3）、**反重甲第三战斗单位**（R5.3）、**人口 / 防御两用地洞**（R2 / R3.4）、**英雄祭坛**（R4.2）、**研发 / 科技建筑（战争磨坊类）**（R4.4）、**1 名英雄**（`mana > 0`，R6）。所有新增建筑 `speed_raw == 0`，供 `derive_movability` 派生 R1.4 / R13.6 可移动性过滤；DefId 落在 `2xx`、不与 `201..204` / 人族 `1xx` / 中立 `9xx` 冲突。数值与人族有差异（更肉 / 更快 / 更低远程稳定度）只以编译期整数常量**相对大小**表达、标 `[取舍]`、引用 R2 / R5，MUST NOT 写成参考项目得来的经典事实、MUST NOT 新增 `armor_type` / `attack_type` 权威字段（C6）。
- 新增 **1 个 `AbilityDef` 主动技能**：只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` + `EffectDef` 组合表达（R6 / R7.6），走既有 `Cast` 命令面；**不新增 `EffectKind`、不新增 `GameplayCommandKind` / `CommandKind` 变体、不引入 JASS / wtg VM（N5）**。新增 **1–2 项 `UpgradeDef` 攻 / 防升级**（`damage_mod` / `hp_mod` / `research_frames`），经既有 `Research` 路径接通（R4.4）。兽族**不臆造对称单位技能表**（参考项目无 `orcUnitActives.json`，见 `design.md`「## 参考项目对照」兽族特有校正）。
- `Race` 六个新类别方法（含 `category_def_ids`）对 `Orc` **从占位替换为真实 `2xx` 常量**，`MeleeSetup::with_race` 扩兽族 `def_ids` / `category_def_ids` 关联；**MUST NOT 新增 `Player.race` 之类运行时权威状态**（C3 / 主 `melee-match/spec.md`「内核零改动与确定性契约」）。分派仍是**定义层 / 装配层**参数查表。
- `crates/w3-client/src/melee/setup.rs` 扩展兽族开局关联；**`CommandKind::Spawn` 字节逐字节不变**（`Spawn` 不含 `DefId`，P8.0 原创），两族装配的 `Spawn` 字节一致（C2）。
- `crates/w3-client/src/melee/ai.rs` 扩展 `AiScript`：会训练兽族新单位 / 施放英雄技能 / 研发升级，沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`，仍为外部帧冻结命令、不新增权威状态。
- `crates/w3-client/tests/melee_loop.rs`（或新测试）：CPU 端到端一局覆盖「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开无残留」各阶段（R3.1 / R4 / R5 / R6 / R11.1 / R11.4 / R11.5），逐帧哈希与基线一致（ADR 0002）；并补 **R1.4 分层判据回归**（新增建筑 / 资源点 / 主城 MUST NOT 出现在 `Move` 生成集合）与**反向验证（改坏验红）**（C4 / C5）。
- **明确三条硬边界**：`w3-core` 零 diff、`Spawn` 字节不变、新增定义走 `defs_hash` / `content_hash`（冻结定义**内容变化**、不是新权威状态字段）。
- **BREAKING：无**。既有 6 个基线定义 `1..=6`、人族 `101..110` / 兽族 `201..204` / 中立 `9xx` 的 DefId 与既有入口行为不变；新增定义只改变 `FrozenDefinitions` 的内容哈希，不改 `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合、快照头部布局、`first_difference()` 与命令排序键（C1 / C2）。

## Capabilities

### New Capabilities

- `orc-faction`: 兽族完整化的冻结定义（远程 / 反重甲 / 人口地洞 / 祭坛 / 战争磨坊 / 英雄单位、1 主动技能、攻防升级、`Race` 分派替换占位）、开局 `def_ids` / `category_def_ids` 装配、AI 命令脚本扩展，以及 CPU 端到端一局的确定性闭环；含「新增定义是冻结定义内容变化、不是新权威状态字段」「数值差异是 `[取舍]` 合成原创、不是经典事实」的 ADR 0002 / ADR 0001 边界。对称于已归档 `human-faction` capability。

### Modified Capabilities

- `unit-orders`: 增加一条 **ADDED Requirement**——兽族新单位 / 建筑的 R1.4 / R13.6 实体来源分层可移动性过滤（新增建筑 / 资源点 / 主城 MUST NOT 出现在 `Move` 的 Scenario 生成集合）与端到端一局各阶段的可执行验证；**既有 7 条 requirement 不改写**。

## Impact

- **代码面**：仅 `crates/w3-client/src/melee/{defs,setup,ai}.rs` 与 `crates/w3-client/tests/melee_loop.rs`（或新测试文件）；**`crates/w3-core/` 零 diff**（`UnitDef` / `FrozenDefinitions` / `CommandKind` / `GameplayCommandKind` / `EffectKind` / `EntityView` / `snapshot.rs` 均不修改），`w3-render` / `w3-net` / `w3-relay` / `Cargo.lock` 零改动。分层判据只读 `EntityView.speed` 是已归档 `runtime-entity-movability` 的既有投影、本 change 不扩展视图、不新增权威状态字段。
- **确定性契约（ADR 0002）**：不新增权威状态字段；新增定义是**冻结定义内容变化**，经 `compute_defs_hash`（`gameplay.rs::compute_defs_hash`）进入 `defs_hash` / `content_hash`（`gameplay.rs::FrozenDefinitions::content_hash`），写入由 `SNAPSHOT_VERSION`（当前 3，`snapshot.rs::SNAPSHOT_VERSION`）定义的快照头部。状态哈希字段集合、快照头部布局、`first_difference()`、30 Hz 整数帧、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键、稳定 `EntityId` 顺序、FNV-1a 哈希一律不变；`cargo test -p w3-core --test determinism` 与 `cargo test --workspace` MUST 全绿，`git diff -- crates/w3-core/` 为空，逐帧状态哈希与 `capture_snapshot()` 字节与基线一致。**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产、零 MPQ 派生数据、零真实导出 JSON；全部为合成 / 原创编译期整数常量，数值 `[待核]`、以本机导出 SLK 核对、不入库、不冒充原版（C6）。四个参考项目只读、不复制代码：`WarsmashModEngine`（AGPLv3）仅借鉴按种族 / 英雄分组的**结构语义**、`godot_warcraft3` / `rts_kernel_cs`（MIT）不借原创单位名 / 暴雪 rawcode / `double` 路线、`vera20k`（GPLv3 + Section 7）逐帧 checksum 范式仅作参照；见 `design.md`「## 参考项目对照」。
- **ADR 与阶段**：相关 ADR 0001 / 0002 / 0003（范围与分期）/ 0004 / 0005。阶段 **P8.2**（`docs/roadmap/P8-human-orc-melee.md` §3）；`openspec/config.yaml` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 / P8.1 一致）。本 change 落在 S1 / S3 的过程分期；S2（回放）/ S4（道具）/ S5（野怪）/ S6（中立建筑）不在本 change 交付但属 1.0 范围。
- **范围外承接（后续独立 change，C8）**：双族对抗与 AI 选族（P8.3）、HUD / 表现层模型键（P8.4）、**P8.5 五个深度玩法**（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、不死暗夜（P9+）、中文上屏（CJK 字体许可证人工拍板）、运行时新实体真实模型绑定（模型缺键回退已知限制）、真窗口人工验收。**联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项**（C7）；N1–N6 不纳入，尤其 N5（JASS / wtg VM）与 N6（资产入库）。会碰权威状态的例外（英雄等级 / 复活 / 掉落 / 迷雾 / attack-move 新命令变体）本 change 不做、属 P8.5，MUST 另走 ADR 0002 流程（状态哈希 + 快照 + `first_difference()` + determinism），**不得借「零改动」旁路**；此为 P8.1 已澄清的常设硬门禁、至今有效、未放松。
- **人工边界**：真窗口人工验收不在本轮执行（CPU 侧以回归 + 门禁自证），如实标注「未执行 / 待补验」；`critic` 规格门 / 代码门待复核（见 `decision-memo.md`）；push / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。
