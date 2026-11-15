# Tasks

> 按 `design.md` D1–D7 分组；`crates/w3-core/` 零 diff、零资产（ADR 0001 / ADR 0002 / C1）。每组自带验证与证据要求；玩法任务引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 1.0 范围 S1 / S3。

## 1. 人族冻结定义完整集（D1）

- [x] 1.1 在 `crates/w3-client/src/melee/defs.rs` 人族段 `100..200` 新增 6 个 `DefId` 常量与对应 `UnitDef`：远程单位（R5.1 / R5.3）、反重甲第三战斗单位（R5.3）、人口建筑（R3.4）、英雄祭坛（R4.2）、研发建筑（R4.4）、1 名英雄 `mana > 0`（R6）；全为编译期整数常量，所有新增建筑 `unit(..., speed=0, ...)` 使 `speed_raw == 0`（R1.4 / R13.6）；验证：`units` 类别 DefId 全局唯一、落在 `1xx`、不与 `101..104` / `201..204` / `9xx` 冲突，干净环境构造零文件 IO（R2 / ADR 0001）
- [x] 1.2 扩 `synthetic_definitions()`：按 `defs.rs:162-216` 既有结构补齐 6 `UnitDef` 条目；英雄 `mana > 0` 由显式构造写入（既有 `unit()` 固定 `mana: 0`）；验证：枚举定义集含全部新条目、构造可重复、`content_hash()` 随条目增删变化（C3 / `compute_defs_hash` 覆盖 `gameplay.rs:144-181`）
- [x] 1.3 新增 1 个 `AbilityDef` 主动技能：只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` + `EffectDef` 组合（R6 / R7.6），走既有 `Cast` 命令面；**MUST NOT** 新增 `EffectKind` / 命令变体 / JASS VM（N5）；验证：`EffectKind` 与 `GameplayCommandKind` 变体集合不变、`git diff -- crates/w3-core/` 为空
- [x] 1.4 新增 1–2 项 `UpgradeDef`（`damage_mod` / `hp_mod` / `research_frames`），经既有 `Research` 路径接通（R4.4）；验证：`Research` 后单位 `damage` / `hp` 按定义生效、命令排序键不变（ADR 0002）
- [x] 1.5 **反向验证（改坏验红，`CODEBUDDY.md` §5 / C5）**：为 `defs_hash` 内容敏感性写「测试内反例元断言」（漏接某类新定义 → 哈希必须变化），证明退化实现下断言失败、正式实现通过；验证：两条证据（缺陷态失败 / 修复态通过）均可复现，参照 `defs.rs:337-366` 的 `defs_hash_is_fixed_and_content_sensitive` 先例

## 2. Race 分派与开局装配（D3 / D4）

- [x] 2.1 给 `Race`（`defs.rs:74-135`）扩展 `ranged()` / `siege()` / `farm()` / `altar()` / `smithy()` / `hero()` 一类分派，人族返回 `1xx` 新常量；**MUST NOT** 新增 `Player.race` 等运行时权威状态（C1 / 主 `melee-match/spec.md:127-129`）；验证：两族分派落在各自段内、与既有 6 个分派不冲突
- [x] 2.2 扩 `crates/w3-client/src/melee/setup.rs` 人族开局 `def_ids` 关联到新类别；`CommandKind::Spawn` 字段与字节保持 `command.rs:14-21` 现状、逐字节不变（C2 / R4.1）；验证：`race_does_not_change_authoritative_spawn_commands`（`setup.rs:684-700`）仍通过、两族 `Spawn` 与 `gameplay_commands` 字节一致
- [x] 2.3 校验新增建筑 / 资源点 / 主城经 `MeleeSetup::movability`（`setup.rs:434-436`）派生为不可移动、新单位 / 英雄派生为可移动；MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`、MUST NOT 回退建筑类型白名单（C4 / R1.4 / R13.6）；验证：`derive_movability` 对新 `DefId` 的断言全绿

## 3. AI 命令脚本与端到端一局（D5 / D6）

- [x] 3.1 扩 `crates/w3-client/src/melee/ai.rs` 的 `AiScript::new`：新增训练新单位 `Train`、英雄施放主动技能 `Cast`、研发 `Research` 三类命令，沿用 `GameplayCommandKind::{Gather, Train, Cast, Research, SetAi, Leave}`；带固定 `ExecuteFrame` / `PlayerId` / `Sequence` 并按帧冻结排序（R12.7 / R4.3 / R4.4 / R6）；验证：脚本含三类新命令、可重复构造、逐帧推进与一次推进 8 帧两端哈希一致（ADR 0002）
- [x] 3.2 扩 `crates/w3-client/tests/melee_loop.rs` 的 `play_full_match` 或新增 CPU 端到端一局测试，逐阶段断言「采集 → 建造 → 训练 → 英雄施法 → 研发 → 战斗 → 摧毁全部建筑结算 → 重开无残留」（R3.1 / R4.1–4.6 / R5 / R6 / R11.1 / R11.4 / R11.5）；验证：每阶段产生预期权威状态变化、`victory` 只结算一次、重开 `victory.resolved == false` 且视图为空
- [x] 3.3 端到端脚本增「全链路逐帧哈希与基线一致」断言：两实例注入同一脚本，每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、与人族装配基线一致（ADR 0002）；验证：不只比最终 hash，覆盖全部中间帧
- [x] 3.4 **反向验证（改坏验红，`CODEBUDDY.md` §5 / C5）**：为「新增建筑 `speed_raw == 0`」与「混合集合只移动可移动单位」写退化过滤器 / 反例元断言，证明缺陷态（建筑被当可移动 / `speed_raw` 改非 0）失败、修复态通过；验证：两条证据均保留，覆盖 R1.4 / R13.6 与新增建筑路径
- [ ] 3.5 扩 `unit-orders` 相关回归：含新增人口建筑 / 祭坛 / 主城 / 金矿的混合选中集合右键与 `M` 移动 MUST NOT 为不可移动实体生成 `Move`、位置不变、落点按稳定 `EntityId` 顺序（R1.4 / R13.6）；全为不可移动实体时显式反馈、不改哈希；验证：新增 Scenario 对应单测全绿

## 4. 门禁、确定性复核与规格门（C1 / C2 / C3 / C5 / C8）

- [x] 4.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`；验证：退出码 0，UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格（`CODEBUDDY.md` §2）
- [ ] 4.2 **重跑确定性**：`cargo test -p w3-core --test determinism`（26 项）、`cargo test --workspace`；验证：全绿；`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合 / 快照头部布局 / `first_difference()` / 命令排序键不变（C1 / ADR 0002）
- [x] 4.3 校验「新增定义走 `defs_hash` / `content_hash`、不是新权威状态字段」：`content_hash()` 随新增定义变化、快照头部含 `defs_hash` / `content_hash`、`Player` / `RtsMatch` 无新增字段（C3）；验证：单测与 `git diff -- crates/w3-core/` 为空双证据
- [ ] 4.4 `openspec validate p8-human-faction --strict` 与 `python tools/verify_all.py`（含 `decision memo OK` / `spec gate OK`）；验证：`is valid` 与 `Verification OK`；`verify_all.py` 的 `spec_gate_check()` 在 `status: pending` 期间的失败属流程顺序（应先由人批准回填 `status: approved` + `fingerprint:` 再 apply），**不改门禁逻辑、不绕过规格门、不 self-approve**（`CODEBUDDY.md:106-107`）
- [ ] 4.5 由只读 `critic` 按全清单（含第 7 / 10 项四件套一致性、第 12 项参考项目对照、第 13 项 `R-x.y` 符合性、第 11 项反向验证）复审**本 change 全部 diff**；验证：报告区分 blocker / residual、无空泛结论、锚可打开
- [x] 4.6 **规格门由人拍板后再回填**：`decision-memo.md` 的 `status: approved` + `fingerprint:`（`python -B tools/verify_all.py --fingerprint p8-human-faction` 的 sha256）+ `approved_by:` + `date:` 由 `spec-steward` 在人确认后回填；验证：`spec_gate_check()` 不报失配、复选框归一化不触发解锁。本轮成文**不回填**（保持 `status: pending`）
- [ ] 4.7 以中文描述 + Conventional Commits 英文 type 前缀本地 commit；验证：`git log -1` 中文描述首行 ≤ 72 字符（`CODEBUDDY.md` §3 / §4）；`git push` / fetch / pull / clone 由人工执行，AI 不碰远端

## Workflow follow-up

- 规格门由人拍板并回填指纹后进入 `/opsx:apply`（对应 owner 仅 `crates/w3-client/`），再走 `critic` 代码门 → 本地 commit → 人工 push → CI 双平台 `verify_all.py` → `/opsx:archive`。
- P8.2–P8.5（兽族 / 双族对抗 / HUD / 五个深度玩法）各由后续独立 change 承接，本 change 不并（C8）。
