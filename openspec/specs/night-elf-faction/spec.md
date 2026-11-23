# night-elf-faction Specification

## Purpose
定义 P9「暗夜精灵全量」在用户 2026-11-22 第二轮规格门 Q1' 已拍板「不死 + 暗夜双族全量」后的行为契约：暗夜 `4xx` 段冻结定义与零资产、古树拔根 / 扎根可移动建筑由定义层数据决定（R1.2）、月亮井回血回蓝、小精灵缠绕金矿、夜间隐身 / 昼夜 / 回血与英雄 / 单位 / 建筑最小集、4 族 16 组合中的暗夜席位；族特色机制纳入 `w3-core` 顶层权威状态（Q2 已拍板）、全量机制本期纳入（Q4 已拍板）、bump 8（Q3 / Q5' 已拍板）、R10.5 边界（fog 可见性只读派生，Q2' 已拍板）、零资产防误用清单、真窗口清单未勾；守住 ADR 0001 / 0002 / 0004 / 0005 与 `WC3-MELEE-RULES.md` S3 / R1–R13 / R10.5 / §14。

## Requirements

### Requirement: 暗夜 `4xx` 冻结定义分段与零资产

系统 SHALL 在 client `synthetic_definitions()` 新增暗夜 `4xx` 段（400–499）定义：小精灵、生命之树、月亮井、战争古树、猎手大厅、1–2 战斗单位、1 英雄（`mana > 0`）+ 1 主动技能，对齐 `docs/roadmap/P8-human-orc-melee.md:17-26` 每族最小集（R2 / R4.1–4.6 / R5 / R6 / R7.6）。DefId MUST 全局唯一、与 `1xx` / `2xx` / `3xx` / `9xx` / `8xxx` / `70_000` / `1_000_000` 命名空间逐一核对（Q1' 已拍板）；全部字段 MUST 为合成原创编译期整数 `[待核]`、零资产零文件 IO（ADR 0001 / ADR 0004）；**新增建筑 `speed_raw == 0`**（R1.4 / R13.6）。

#### Scenario: 五段分段唯一且零资产构造

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造含 `1xx` / `2xx` / `3xx` / `4xx` / `9xx` 五段的 `FrozenDefinitions` 并枚举 `units` 类别
- **THEN** 暗夜定义全部落在 `4xx` 段、DefId 全局唯一且不与 `1xx` / `2xx` / `3xx` / `8xxx`（`TIER2_UNIT=8101` / `TOWN_HALL_PORTAL_DEF=8002`）/ `9xxx`（`CREEP_UNIT=9301` / 道具 9001–9402）冲突，所有新增建筑 `speed_raw == 0`，构造不做任何文件 IO、不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名或 rawcode（R2 / R1.4 / ADR 0001 / ADR 0004 / Q1' / Q5'）

#### Scenario: 内容哈希随新增定义变化而非新增权威分类字段

- **WHEN** 对比补齐暗夜族定义前后两份 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 哈希不同且差异经 `compute_defs_hash`（`gameplay.rs:421-504`）的 units / abilities / upgrades / items 与机制冻结常量段进入 `defs_hash` / `content_hash` 快照头部；该变化是定义层内容变化，与 Q2 新增 `w3-core` 顶层权威状态严格区分、MUST NOT 复用 `Player.race` / `RtsMatch.race` 分类字段（ADR 0002 §6-8 / R12.4 / Q2 / Q6）

### Requirement: 古树拔根 / 扎根可移动建筑由定义层数据决定（R1.2）

暗夜古树 SHALL 支持拔根 / 扎根状态切换，扎根态 `speed_raw == 0` 判不可移动、拔根态 `speed_raw > 0` 判可移动（R1.2 `WC3-MELEE-RULES.md:77-79,358-360` `[已核]`）；护甲类型 / 结构标志 / 移动行为 MUST 由定义层 level data 传入（参考对照 R7，**AGPLv3 只借语义零 Java 复制**）。**可移动性 MUST 由定义层 `UnitDef.speed_raw` / 只读 `EntityView.speed` 分层数据判定、MUST NOT 用单位名 / `DefId` 段位（`4xx`）/ 白名单推断**（R1.4 / R13.6 / `openspec/config.yaml:57-58`）；含生命之树 / 月亮井 / 战争古树 / 金矿 / 主城等不可移动实体的混合选中集合 MUST NOT 生成 `Move`（R1.1 / R9.1 / R13.6）。

#### Scenario: 定义层数据驱动且不用名字 / 段位 / 白名单

- **WHEN** 以 rooted / uprooted 两态古树 `UnitDef.speed_raw` 与只读 `EntityView.speed` 分别构造装配期只读映射与运行时动态实体判据
- **THEN** 扎根态 `speed_raw == 0` 判不可移动、拔根态 `speed_raw > 0` 判可移动，判据 100% 来自定义层数据、全库 Grep 无「按名字 / `4xx` 段位 / 建筑白名单」推断；frame-0 装配实体用装配期 `UnitDef.speed_raw`、运行时动态实体用只读 `EntityView.speed == 0` 两条来源 MUST NOT 互相替代（R1.2 / R1.4 / R13.6 / `defs.rs:466-475,1533-1556` 先例 / Q2' 已拍板 R10.5 边界）

#### Scenario: 混合选中集合只为可移动单位生成 Move 且反向验证

- **WHEN** 框选含生命之树 / 月亮井 / 战争古树（rooted）/ 金矿 / 树木 / 主城与可移动单位的混合集合并下达移动命令，再以退化实现（把 rooted 古树 `speed_raw` 改非 0 或用段位白名单）替换正式实现
- **THEN** 不可移动实体 MUST NOT 出现在 `Move` 生成集合、位置不变、只为可移动单位按稳定 `EntityId` 顺序生成 `Move`；退化实现令「建筑不响应 Move」断言在缺陷态失败、修复态通过（`CODEBUDDY.md §5` 形态① / R1.1 / R1.4 / R9.1 / R13.6 / Q9' 已拍板）。

### Requirement: 月亮井回血回蓝、小精灵缠绕金矿与夜间隐身

月亮井回血回蓝（R10.3 / R6.2）与小精灵缠绕金矿（R1.5 `WC3-MELEE-RULES.md:105`）SHALL 作为独立于普通 `Gather` 的机制本期纳入、参数由定义层数据表注入、MUST NOT 硬编码族名 / 白名单（参考对照 R6，AGPLv3 只借语义）。夜间隐身（R10.4）SHALL 按 Q4 已拍板纳入、目标可见性判定按 Q2 已拍板进 `w3-core` 顶层权威状态；**fog 可见性 MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`**（R10.5 / §0.3 T3 / Q2' 已拍板 / D1 明确不借鉴）。B 类 `[待核]` 数值按 Q3' 已拍板不冻结、由人核对（§14）。

#### Scenario: 机制确定性结算且参数数据驱动

- **WHEN** 推进若干逻辑帧触发月亮井回血回蓝 / 小精灵缠绕金矿采集 / 夜间隐身目标判定
- **THEN** 全部按 30 Hz 整数帧、`SimFixed` Q47.16、稳定排序确定性结算、无墙钟 / `f64` / `HashMap` 迭代顺序 / 表现层独立随机源；参数由定义层数据表注入、MUST NOT 硬编码族名 / 白名单、`goldPerInterval/maxNumberOfMiners/radiusOfMiningRing` 等 B 类数值 `[待核]`（R1.5 / R10.3 / R10.4 / R6.2 / ADR 0002 §2-3 / Q2 / Q3' / Q4）

#### Scenario: 夜间隐身进权威状态而 fog 可见性保持只读派生

- **WHEN** 校验夜间隐身目标可见性判定的权威状态归属与迷雾 / 视野可见性路径
- **THEN** 隐身目标判定按 Q2 进 `w3-core` 顶层权威状态并四处同步；迷雾网格 / 当前可见 / 已探索层 / 视野范围仍是表现层只读派生、不进 FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION`（8）定义的快照 / `first_difference()`，vera20k fog 进哈希 D1 仍是「对照差异、明确不借鉴」、MUST NOT 静默改规则（§14）（R10.4 / R10.5 / §0.3 T3 / ADR 0002 §5 / Q2' / Q4）

### Requirement: 昼夜 / 回血全量、英雄 / 单位 / 建筑最小集本期纳入（Q4 已拍板）

昼夜循环全量（R10.2）、R10.3 回血、R5.6 视野阻挡、S3 暗夜夜间隐身特色、R8.4 野怪睡眠 SHALL 按 Q4 已拍板「本期含全量」纳入本期（MUST NOT 静默窄化为后续承接）；暗夜英雄 / 单位 / 建筑最小集 SHALL 齐备、主动技能 MUST 只用既有 `EffectKind::{Damage, Heal, BuffStat, Stun}` 与 `EffectDef` 组合、经既有 `Cast` 命令面结算（R6 / R7.6 / ADR 0002 / N5）。回血条件 MUST 建模为定义层枚举逐 tick 查表、非种族名硬编码（参考对照 R9）。两套昼夜常量口径未统一按 Q3' 已拍板由人核对、不静默改规则（§14）。

#### Scenario: 六类全量机制与暗夜最小集可验收

- **WHEN** 在本期实现上分别触发昼夜循环全量 / R10.3 回血 / R5.6 视野阻挡 / 暗夜夜间隐身 / 野怪睡眠，并枚举暗夜 `4xx` 段最小集 `UnitDef` / 英雄主动技能
- **THEN** 六类机制与暗夜最小集均可执行验证且按 Q2 口径确定性结算、英雄 `mana > 0` 主动技能经既有 `Cast` 命令面按 `EffectDef` 组合结算、所有新增建筑 `speed_raw == 0`（R2 / R3.1 / R4.1–4.6 / R5 / R6 / R7.6 / R10.2 / R10.3 / R10.4 / R5.6 / R8.4 / S3 / Q4 / Q1'）

#### Scenario: 昼夜回血按定义层枚举且常量口径不静默统一

- **WHEN** 校验昼夜回血条件实现与两套昼夜常量（P8.5e `fog.rs:59-62` 的 `600/480` vs `CreepConstants` `600/600`，`creep.rs:153-154`）
- **THEN** 回血条件由定义层枚举逐 tick 查表、非种族名硬编码；两套常量口径未统一 MUST 如实登记、按 Q3' 已拍板由人核对（实施期数值核对、非规格门待拍板范围 / 架构 Q）、MUST NOT 静默改 `WC3-MELEE-RULES.md` R10.2 或把任一值冻结进权威定义（R10.2 `[已核 / 时长待核]` / R10.3 / §14 / Q3'）

### Requirement: 4 族 16 组合中的暗夜席位与双族对抗（Q1' 已拍板）

`MeleeSetup::with_races` / `COMBINATIONS`（`setup.rs:970-975`）SHALL 由 2×2 四组合扩到 **4×4 十六组合**（4 族 4×4，含同族组合；Q1' 已拍板「不死 + 暗夜双族全量」+ P8 人 / 兽两族不重做），暗夜席位用对应 `Race::NightElf` 段 `4xx`、中立资源恒 `9xx`（R2 / R12.7 / Q1'）。系统 MUST 沿用既有 `GameplayCommandKind` 码 1–23 / `resolve_victory` / `AiScript` 命令面与 `EffectDef` 组合、Q2 / Q3 新增族特色机制命令走码 24 起（既有 1–23 不动）、MUST NOT 引入 JASS VM（N5 / R7.6 / ADR 0002）。`with_race` 兼容别名与 `MeleeSetup::new()` 行为 MUST 逐字节不变（R12.7）。

#### Scenario: 十六组合含暗夜席位且既有入口逐字节不变

- **WHEN** 以 4×4 十六组合分别装配开局（含暗夜席位与不死 vs 暗夜双族对抗），并分别以 `with_race(race)` 与 `with_races(race, race.opponent())`、`MeleeSetup::new()` 比对命令字节
- **THEN** 十六组合本地 / AI 席位 `def_ids` 落在各自族段（暗夜 `4xx`）、同族组合双方段位一致、中立恒 `9xx`；三者 `Spawn` / `gameplay_commands` / `kinds` / `def_ids` 逐字节一致，既有回落基线测试仍绿（R2 / R12.7 / Q1' / Q5'）

#### Scenario: CPU 一局打完与逐帧哈希一致

- **WHEN** 以暗夜族 / 不死族装配注入完整一局脚本（采集 → 建造 → 训练 → 英雄施法 → 研发 → 族特色机制触发 → 战斗 → 摧毁敌方主城结算 → 重开）并逐帧推进
- **THEN** 十六组合各能打完一局、`resolve_victory` 只结算一次且按稳定顺序、逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与同脚本两实例 v8 新基线一致、重开无残留、MUST NOT 断言跨组合或 v7 旧基线数值相等（R11.1 / R11.4 / R11.5 / R12.4 / ADR 0002 / `:148` / Q1' / Q3）

### Requirement: `w3-core` 权威状态口径、bump 8 与 R10.5 边界（Q2 / Q3 / Q5' / Q2' 已拍板）

暗夜族特色机制（古树状态 / 昼夜相位 / 隐身目标判定 / 月亮井 / 缠绕金矿 / 夜间隐身）SHALL 按 Q2 已拍板进 `w3-core` 顶层权威状态（`BTreeMap` / `BTreeSet`、稳定 `EntityId` 顺序），并**四处同步**进 `compute_state_hash` / `Snapshot` 顶层字段 / `first_difference()` / `restore_snapshot`（ADR 0002 §6-8 / `:115`）。三版本号 SHALL 按 Q3 / Q5' 已拍板 7→8、旧 v7 明确拒绝、golden hash 按 v8 如实重取、proto tag 46 起、命令码 24 起、`EventKind` 变体、determinism 26 项、反向验证形态①、ADR 复核追加（清单见 `undead-faction` / Q5'）。`git diff -- crates/w3-core/` 非空属预期；fog 可见性 MUST NOT 进权威状态（R10.5 / Q2' / D1 / §14）。

#### Scenario: 新增权威状态四处同步且旧 v7 明确拒绝

- **WHEN** 构造含暗夜族特色机制的 `RtsMatch`、捕获 v8 `Snapshot`、以 v7 快照 / 回放 / 命令包分别尝试恢复与解码，并以「新增权威状态漏进四处之一」退化实现跑 `cargo test -p w3-core --test determinism`（26 项）
- **THEN** 新增段以 `BTreeMap` / `BTreeSet` 承载、四处同步、`first_difference()` 为 `None`、v8 快照头部 `defs_hash` / `content_hash` / `version` / `sim_version` 校验通过；v7 明确拒绝、无静默兼容或跨版本迁移；退化实现令元断言在缺陷态失败、修复态通过（ADR 0002 §6-8 / §7 / `CODEBUDDY.md §5` 形态① / ADR 0005 §3 / Q2 / Q3 / Q5' / Q2'）

#### Scenario: R10.5 边界与对照差异不抹平

- **WHEN** 校验暗夜全量机制的权威状态归属与迷雾 / 视野可见性路径及参考对照 D1 / D2
- **THEN** 玩法机制进权威状态、fog 可见性 MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`；vera20k fog 进哈希 D1 仍是「对照差异、明确不借鉴」、「四族玩法机制权威状态」与「迷雾可见性」不是同一问题（D2）、MUST NOT 静默改规则（§14）、MUST NOT 抹平对照差异（R10.5 / §0.3 T3 / ADR 0002 §5 / Q2'）

### Requirement: 范围边界、1.0 张力已拍板登记与零资产防误用清单

本 change SHALL 按已拍板口径做**不死 + 暗夜双族全量 + 权威状态 + bump 8**（Q1' / Q2 / Q3 / Q4 已拍板）；**1.0 范围张力 MUST 七处显式如实呈现、不静默窄化、不冒充已解决**（`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226` 字面「一并做完」；P8.5e Q2 曾登记为后续承接，`2026-11-22-p8-fog-of-war/decision-memo.md:80,128`）。Q4 / Q1' / Q4' 均已由用户明确拍板——**这是已拍板后的如实登记、不是待拍板项**。零资产 / 防误用清单（`:373-384`）全项适用；`WarsmashModEngine`（AGPLv3）只借语义零 Java 复制、`vera20k`（GPLv3 + §7）只读零复制、MIT 项目借结构不借实现且 `double` 不采用（ADR 0001 / ADR 0004 / §14 / Q7）。

#### Scenario: 张力七处显式呈现且不静默窄化

- **WHEN** 核对四件套 + 纪要中 1.0 范围张力的呈现位置与处置口径
- **THEN** `proposal.md` Scope / What Changes、`design.md` Risks /「## 范围边界」/「## 对照差异」D4、本 Requirement 与 `undead-faction` 范围边界 Requirement、`decision-memo.md`「## 范围张力登记」/「## 阶段声明」、`tasks.md` 第 8 组、8 份 MODIFIED delta 共七处显式呈现字面「一并做完」与本期双族全量的关系，处置按 Q4 / Q4' 已拍板、MUST NOT 写成待拍板结论、MUST NOT 静默窄化用户 2026-11-14 已拍板的 1.0 范围（S3 / §0 / §0.3 T2 / R10.2-5 / §14 / Q4'）

#### Scenario: 干净构造与防误用清单

- **WHEN** 在干净环境构造暗夜族定义与机制冻结常量并核对常量落点、许可证与资产扫描
- **THEN** 构造成功零文件 IO、B 类 `[待核]` 数值不冒充原版、不含 Warsmash `GRID_STEP=128` / `PATHING_RATIO=4` / `DYING_UNIT_VISION_RADIUS` / `abilityBehaviors/*.json` 数值与 rawcode / godot `FACTION_UNIT_ROSTERS.md:37-45` / `CLASSIC_RULES_BASELINE.md:15-19` 网页值 / `melee.jui`，`python tools/verify_all.py` 资产扫描绿、无新增未登记依赖、`NOTICE.md` / `Cargo.lock` 无 diff（ADR 0001 / ADR 0004 / R2 / §14 / Q3' / Q7）

### Requirement: 真窗口人工验收与跨平台 / CI 按 Q8' 保持未勾

`tasks.md` 第 8 组真窗口清单 8.x MUST 全部保持 `- [ ]`「未执行 / 待人工」，不提供替代、不用离屏截图 / CPU 回归冒充真窗口验收、CI MUST NOT 声称已验证 GPU；先离屏 `render_headless_rgba` + `ViewImage` 自证 + CPU 回归全绿再请人工验收（GUI 类修复先自测纪律、不反复试错）。跨平台确定性 MUST 只沿用 ADR 0002 复核记录「Windows 11 / Debian 12 x86_64 / rustc 1.99.1（v3）已逐字节验证」，v4–v7 Debian 侧逐字节比对、**v8（本轮 bump）比对**、macOS / ARM64 / 其它 rustc 仍待验证、不声称全平台达成或有新验证；CI 双平台 `verify_all.py` 本会话未运行、如实标注（R12.4 / `playable-window-entry/spec.md:149-176` / Q5' / Q8'）。

#### Scenario: 清单保持未勾且人工边界如实说明

- **WHEN** 核对 `tasks.md` 第 8 组与人工边界登记
- **THEN** 8.x 全部 `- [ ]`「未执行 / 待人工」、无证据项如实标注不冒充通过、4 族（含暗夜）选族 / 命令卡 / 模型键 / 小地图 / VICTORY 真窗口项未执行、1.0 字面「一并做完」张力保持显式呈现（Q8' / Q4' / S3）

#### Scenario: 跨平台与 CI 不冒称

- **WHEN** 核对 ADR 0002 复核记录、v8 golden hash 基线与 CI 记录
- **THEN** 仅沿用 v3 双平台已验证结论、v4–v8 Debian 侧 / macOS / ARM64 / 其它 rustc 标注待验证、不声称全平台达成或有新验证；CI 双平台 `verify_all.py` 本会话未运行、不冒充已绿，`status: pending` 令 `spec_gate_check()` 判红是 fail-closed 正确信号、不绕过不改门禁（ADR 0002 `:102-108,110-116` / `CODEBUDDY.md §5:54` / Q8' / Q10' / R21）
