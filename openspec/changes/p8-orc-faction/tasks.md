# Tasks

> 按 `design.md` D1–D10 分组；`crates/w3-core/` 零 diff、零资产、数值 `[待核]` / `[取舍]`（ADR 0001 / ADR 0002 / C1 / C6）。每组自带验证与证据要求；玩法任务引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 1.0 范围 S1 / S3。本 change 对称于已归档 P8.1，站在 `p9-command-hotkeys`（键盘 `M` / `A` 复用 R1.4 过滤）与 `runtime-entity-movability`（实体来源分层判据）已归档结论之上、不重开。证据锚一律「路径 + 函数名 / 符号名」或章节号 / 规则编号，不写精确 `:NN` 行号（本轮 `w3-client` 代码会被 owner 改动）。

## 1. 兽族冻结定义完整集（D1 / D10）

- [x] 1.1 在 `crates/w3-client/src/melee/defs.rs` 兽族段 `200..300` 新增 6 个 `DefId` 常量与对应 `UnitDef`：远程 / 猎头类（R5.1 / R5.3）、反重甲第三战斗单位（R5.3）、人口 / 防御两用地洞（R2 / R3.4）、英雄祭坛（R4.2）、研发建筑 / 战争磨坊类（R4.4）、1 名英雄 `mana > 0`（R6）；全为编译期整数常量，所有新增建筑 `defs.rs::unit(..., speed=0, ...)` 使 `speed_raw == 0`（R1.4 / R13.6）；验证：`units` 类别 DefId 全局唯一、落在 `2xx`、不与 `201..204` / 人族 `1xx` / 中立 `9xx` / 基线 `1..=6` 冲突，干净环境构造零文件 IO（R2 / ADR 0001 / C6）
- [x] 1.2 扩 `defs.rs::synthetic_definitions`：按既有结构补齐 6 个 `UnitDef` 条目（22 → 28）；英雄 `mana > 0` 由 `defs.rs::unit_with_mana` 显式写入（既有 `defs.rs::unit` 固定 `mana: 0`）；验证：枚举定义集含全部新条目、构造可重复、`content_hash()` 随条目增删变化（`gameplay.rs::compute_defs_hash` 覆盖四类 / C2）
- [x] 1.3 兽族数值与人族差异（更肉 / 更快 / 更低远程稳定度）只以合成整数常量**相对大小**表达、标 `[取舍]`、引用 R2 / R5 / R5.1 / R5.3 的 `[已核]` 框架；MUST NOT 写成参考项目得来的经典事实、MUST NOT 新增 `armor_type` / `attack_type` 权威字段、逻辑层 MUST NOT 写 `if race == Orc { hp += … }`；验证：数值改动集中在 `defs.rs` 的 `2xx` 段常量、`git diff -- crates/w3-core/` 为空（C6 / D10 / `design.md` 归因纠正② / G5）
- [x] 1.4 校验零资产 / 零 rawcode：不复制 Warsmash `AOwk` / `AHwe` / `ASlo` / `Hamg` / `ogre` / `opeo`、不复制 godot 原创三族单位名 / `FACTION_UNIT_ROSTERS.md` 原创机制名、不复制任何暴雪字符串表 / SLK / `Hotkey` / `Buttonpos`；验证：仓库文本搜索无上述字面、无 `.mpq` / `.blp` / `.slk` 等派生后缀（ADR 0001 / ADR 0004 / `UPSTREAM.md` T4 / T5 边界）

## 2. 兽族主动技能、攻防升级与 Race 分派替换占位（D2 / D3 / D10）

- [x] 2.1 新增 1 个 `AbilityDef` 主动技能（类别内唯一 ID）：只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` + `EffectDef` 组合（R6 / R7.6），走既有 `Cast` 命令面；**MUST NOT** 新增 `EffectKind` / `GameplayCommandKind` / `CommandKind` 变体 / JASS VM（N5），**MUST NOT** 借 `orcHeroActives.json` 的 `AOwk` 隐身 / 移速增益这类需新增权威状态的 P8.5 语义；验证：变体集合不变、`git diff -- crates/w3-core/` 为空（`design.md` T4 兽族特有校正①）
- [x] 2.2 新增 1–2 项 `UpgradeDef`（`damage_mod` / `hp_mod` / `research_frames`），经既有 `Research` 路径接通（R4.4）；验证：`Research` 后单位 `damage` / `hp` 按定义生效、命令排序键不变（ADR 0002）
- [x] 2.3 **MUST NOT** 臆造对称 `orcUnitActives.json` 兽族单位技能表（参考项目实际无该文件、与 `UPSTREAM.md` T4 矩阵条目一致）、**MUST NOT** 臆造 `FarSeer` / `Blademaster` / `ShadowHunter`（`Abilities/Orc/` 下仅 `AbilityWarStomp.j`）；验证：对照 `design.md`「## 参考项目对照」兽族特有校正③如实登记（`design.md` T4 / G5）
- [x] 2.4 给 `defs.rs::Race` 的 `ranged()/siege()/farm()/altar()/smithy()/hero()/category_def_ids` 六个新类别对 `Orc` **从 `ORC_SOLDIER` / `ORC_BARRACKS` 占位替换为真实 `2xx` 常量**、人族保持 `1xx` 不变；**MUST NOT** 新增 `Player.race` 等运行时权威状态（C3 / 主 `melee-match/spec.md`「内核零改动与确定性契约」）；验证：两族分派落在各自段内、兽族返回值不再等于占位、与既有 6 个分派不冲突

## 3. 开局装配、AI 脚本与端到端一局（D4 / D5 / D6）

- [x] 3.1 扩 `crates/w3-client/src/melee/setup.rs::MeleeSetup::with_race` 兽族开局 `def_ids` / `category_def_ids` 关联到新类别；`CommandKind::Spawn` 字段与字节保持 `command.rs::CommandKind::Spawn` 现状（不含 `DefId`）、逐字节不变（C2 / R4.1）；验证：`setup.rs::race_does_not_change_authoritative_spawn_commands` 仍通过、两族 `Spawn` 与 `gameplay_commands` 字节一致
- [x] 3.2 扩 `crates/w3-client/src/melee/ai.rs::AiScript::new`：新增训练兽族新单位 `Train`、兽族英雄施放主动技能 `Cast`、研发 `Research` 三类命令，`unit_def` 经 `setup.category_def_ids()` 查表（D3 替换占位后拿到真实 `2xx`），沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`；带固定 `ExecuteFrame` / `PlayerId` / `Sequence` 并按帧冻结排序（R12.7 / R4.3 / R4.4 / R6）；验证：脚本含三类新命令、可重复构造、逐帧推进与一次推进 8 帧两端哈希一致（ADR 0002）
- [x] 3.3 扩 `crates/w3-client/tests/melee_loop.rs` 的 `play_full_match` 或新增 CPU 端到端一局测试，逐阶段断言「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开无残留」（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5）；验证：每阶段产生预期权威状态变化、`victory` 只结算一次、重开 `victory.resolved == false` 且视图为空
- [x] 3.4 端到端脚本增「全链路逐帧哈希与基线一致」断言：两实例注入同一脚本，每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、与兽族装配基线一致，**不只比最终 hash**（ADR 0002 / golden-hash 复核记录）；验证：覆盖全部中间帧、`git diff -- crates/w3-core/` 为空
- [x] 3.5 扩 `unit-orders` 相关回归：含兽族新增人口地洞 / 祭坛 / 战争磨坊 / 主城 / 金矿的混合选中集合右键与键盘 `M` 移动 MUST NOT 为不可移动实体生成 `Move`、位置不变、落点按稳定 `EntityId` 顺序（R1.4 / R13.6 / 承接 `p9-command-hotkeys`）；全为不可移动实体时显式反馈、不改哈希；验证：frame-0 装配映射 + 运行时 `EntityView.speed` 两条来源分层的 Scenario 单测全绿（C4 / D7）

## 4. 门禁、确定性复核与零 diff 验证（C1 / C2 / C3 / C8 / D8）

- [x] 4.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`；验证：退出码 0，UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格（`CODEBUDDY.md` §2）
- [x] 4.2 **重跑确定性**：`cargo test -p w3-core --test determinism`（`tests/determinism.rs`，26 项）、`cargo test --workspace`；验证：全绿；`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 稳定 `EntityId` 顺序 / FNV-1a / golden hash 逐字节不变（C1 / ADR 0002 / D8）
- [x] 4.3 校验「新增定义走 `defs_hash` / `content_hash`、不是新权威状态字段」：每个新增兽族定义漏接都会改变 `content_hash()`、快照头部含 `defs_hash` / `content_hash`、`Player` / `RtsMatch` 无新增字段；验证：单测（`defs.rs::defs_hash_is_fixed_and_content_sensitive` 先例）与 `git diff -- crates/w3-core/` 为空双证据（C2 / D2）
- [x] 4.4 `openspec validate p8-orc-faction --strict` 与 `python tools/verify_all.py`（含 `decision memo OK` / `spec gate OK`）；验证：`is valid` 与 `Verification OK`；规格门 `status: approved`（用户 2026-11-17 指令、非 pending / 非 self-approve）、`fingerprint:` 为真值回填后 `spec_gate_check()` 不报失配、复选框归一化不触发解锁（`CODEBUDDY.md` §8 / §9）
- [x] 4.5 由只读 `critic` 按全清单（含第 7 / 10 项四件套一致性、第 11 项反向验证、第 12 项参考项目对照、第 13 项 `R-x.y` 符合性）复审**本 change 全部 diff**；验证：报告区分 blocker / residual、无空泛结论、锚可打开；`critic` 规格门 / 代码门结论如实登记、不冒充通过（C8 / `decision-memo.md` 待确认项）
- [x] 4.6 以中文描述 + Conventional Commits 英文 type 前缀本地 commit；验证：`git log -1` 中文描述首行 ≤ 72 字符（`CODEBUDDY.md` §3 / §4）；`git push` / fetch / pull / clone 由人工执行，AI 不碰远端（`CODEBUDDY.md` §4 / §10）

## 5. 参考对照与许可证边界复核（C6 / D10）

- [x] 5.1 逐锚复核 `design.md`「## 参考项目对照」：兽族特有四条校正（`orcHeroActives.json` 是 `AOwk` / `windwalk` 非牛头人技能、`AbilityWarStomp.j` 仅 1 个 `.j` 且数据 / 效果分离、`orcUnitActives.json` 不存在 / `Abilities/Orc/` 无 `FarSeer` 等、`MutableObjectData.java` 的 `urac/arac/frac/grac` 是通用字段且 `Grep Orc|ORC|ogru|ogre|opeo` 无匹配）、两处归因纠正（ADR 0002 硬门禁出处不是 `vera20k`、数值差异不是参考项目经典事实）、G1–G5 参考缺口、许可证边界；验证：锚逐条可打开、语义相符（`UPSTREAM.md` 触发时机第 4 / 5 条 / `critic` 清单第 12 项）
- [x] 5.2 许可证边界复核：`WarsmashModEngine`（AGPLv3）仅借语义不复制代码 / rawcode / 单位名 / 数值、不引入 Java / LibGDX 依赖；`godot_warcraft3` / `rts_kernel_cs`（MIT）借结构不借实现 / `double` 路线；`vera20k`（GPLv3 + Section 7）未复制代码故附加条款不进入；验证：四项目均不复制代码进仓库、资产 / 派生数据不入库（ADR 0001 / ADR 0004）

## 6. 反向验证（改坏验红，`CODEBUDDY.md` §5 / C5 / D9）

- [x] 6.1 为 `defs_hash` 内容敏感性写「测试内反例元断言」：每个新增兽族定义（6 `UnitDef` / 1 `AbilityDef` / 1–2 `UpgradeDef`）漏接 → `content_hash()` 必须变化；**对称复用** `defs.rs::rebuild_definitions` / `defs.rs::degenerate_hash_ignoring_new` / 成对 `defs.rs::reverse_validation_defs_hash_formal_passes_the_assertion` / `defs.rs::reverse_validation_defs_hash_degenerate_fails_the_same_assertion` 先例；验证：退化实现下断言失败、正式实现通过（缺陷态 / 修复态两条证据均可复现）
- [x] 6.2 为「新增建筑 `speed_raw == 0`」写退化反例：把某类兽族新增建筑的 `speed_raw` 改为非 0 → R1.4 派生断言 / 混合集合 `Move` 过滤断言在缺陷态失败、修复态通过；验证：覆盖 frame-0 装配映射路径（C4 / R1.4 / R13.6）
- [x] 6.3 为「英雄 `mana > 0`」写退化反例：把兽族英雄 `mana` 退化为 0（漏用 `defs.rs::unit_with_mana`）→ 英雄定义 / 施法断言在缺陷态失败、修复态通过；验证：两条证据均可复现（R6）
- [x] 6.4 为「`Race::category_def_ids` 替换占位」写退化反例：让兽族六个新类别仍回落 `ORC_SOLDIER` / `ORC_BARRACKS` → 分派 / AI 查表 / 端到端断言在缺陷态失败、修复态通过；验证：两条证据均可复现（C3 / D3）
- [x] 6.5 为可移动性过滤写退化过滤器反例（对称 `tests/movability.rs::degenerate_filter`）：不过滤 / 把新增建筑当可移动 / 用运行时视图 `speed` 替代 frame-0 装配映射 → 混合集合断言在缺陷态失败、修复态通过；验证：覆盖 R1.4 / R13.6 两条来源分层、不互替（C4 / C5 / D7）
- [x] 6.6 汇总改坏验红证据：以上四类退化反例的「缺陷态失败 / 修复态通过」命令输出或测试内元断言均保留、不冒充全绿；无法构造缺陷态的情形 MUST 在 `design.md` D9 登记理由与替代证据（仅「全绿」不算完成）

## 7. 收尾、已知限制如实标注与规格锁定（C7 / C8 / D10）

- [x] 7.1 **真窗口人工验收「未执行 / 待补验」如实标注**：本轮 CPU 侧以回归 + 门禁自证，真窗口 / GPU / egui overlay 一律未人工观察，不冒充通过（沿用 P8.1 / `p9-command-hotkeys` 归档说明先例，`decision-memo.md` 待确认项）
- [x] 7.2 已知限制 / 范围外承接如实登记：运行时动态实体承接边界（`runtime-entity-movability` 已归档、本 change 沿用不扩视图）、运行时新实体「模型缺键回退」四面体占位（P8.4）、P8.3 双族对抗与 AI 选族 / P8.4 HUD / P8.5 五个深度玩法 / P9+ 不死暗夜 / 中文上屏（CJK 字体许可证人工拍板）/ 数值 `[待核]` 以本机 SLK 核对、参考缺口 G1–G5；本 change 不扩范围、不补测、不改代码（C7 / C8 / D10）
- [x] 7.3 规格锁定复核：`decision-memo.md`「## 规格门」`status: approved` + 四件套指纹真值（由 `python -B tools/verify_all.py --fingerprint p8-orc-faction` 取、不猜）+ `approved_by:` + `date: 2026-11-17` + `reason:`；apply 由 `client-owner` 执行，四件套正文零改动、`tasks.md` 复选框归一化不解锁；四件套任一 artifact 改动即视为解锁、MUST 按 `CODEBUDDY.md` §8 / §9 同步回查其余三份并重走规格门（`CODEBUDDY.md` §8 / §9）

## Workflow follow-up

- 规格门已由用户 2026-11-17 本会话指令拍板批准（等价规格门批准、`status: approved`、真值指纹回填），进入 `/opsx:apply`（对应 owner 仅 `crates/w3-client/`），再走 `critic` 代码门 → 本地 commit → 人工 push → CI 双平台 `verify_all.py` → `/opsx:archive`。
- P8.3–P8.5（双族对抗与 AI / HUD / 五个深度玩法）与 P9+（不死 / 暗夜）各由后续独立 change 承接，本 change 不并（C8 / `docs/roadmap/P8-human-orc-melee.md` §6 第 4 条 P8.3 在两族 CPU 闭环都绿之后开）。
