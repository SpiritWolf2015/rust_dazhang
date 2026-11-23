# Spec Delta

## Purpose

定义 P9「不死族全量」在用户 2026-11-22 第二轮规格门 Q1' 已拍板「不死 + 暗夜双族全量」后的行为契约：不死族 `3xx` 冻结定义分段与零资产、4 族按表扩展、`COMBINATIONS` 4→16 中的不死席位与双族对抗、CPU 一局打完与逐帧哈希一致；族特色机制纳入 `w3-core` 顶层权威状态（Q2 已拍板）、全量机制本期纳入（Q4 已拍板）、bump 8 全套与旧 v7 明确拒绝（Q3 / Q5' 已拍板）、新增权威状态进入状态哈希与由 `SNAPSHOT_VERSION` 定义的快照及四处同步；守住 ADR 0001 / 0002 / 0004 / 0005 与 `WC3-MELEE-RULES.md` S3 / R1–R13 / R10.5 / §14；Q1'–Q10' 均已拍板、真窗口清单未勾。

## ADDED Requirements

### Requirement: 不死族 `3xx` 冻结定义分段与零资产

系统 SHALL 在 client `synthetic_definitions()` 新增不死族 `3xx` 段（300–399）定义：工人、主城、人口建筑、军事建筑、英雄祭坛、1–2 战斗单位、1 英雄 + 1 主动技能，对齐 `docs/roadmap/P8-human-orc-melee.md:17-26` 每族最小集（R2 / R3.1 / R3.4 / R4.1–4.6 / R5 / R6 / R7.6）。每条 DefId MUST 全局唯一、与 `1xx` / `2xx` / `4xx` / `9xx` / `8xxx` / `9xxx` / `70_000` / `1_000_000` 命名空间逐一核对不冲突；全部字段 MUST 为合成原创编译期整数常量 `[待核]`、零资产、零文件 IO（R2 / ADR 0001 / ADR 0004）；新增建筑 `speed_raw` MUST 为 0（R1.4 / R13.6）。

#### Scenario: 分段唯一且零资产构造

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造含五段（`1xx` / `2xx` / `3xx` / `4xx` / `9xx`）的 `FrozenDefinitions` 并枚举 `units` 类别
- **THEN** 不死族定义全部落在 `3xx` 段、DefId 全局唯一且不与 `1xx` / `2xx` / `4xx` / `8xxx`（`TIER2_UNIT=8101` / `TOWN_HALL_PORTAL_DEF=8002`）/ `9xxx`（`CREEP_UNIT=9301` / 道具 9001–9402）冲突，所有新增建筑 `speed_raw == 0`，构造不做任何文件 IO、不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名或 rawcode（R2 / R1.4 / ADR 0001 / ADR 0004 / Q1' / Q5'）

#### Scenario: 内容哈希随新增定义与冻结常量变化

- **WHEN** 对比补齐不死族 / 暗夜族定义与族特色机制冻结常量前后两份 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 哈希不同且差异经 `compute_defs_hash`（`gameplay.rs:421-504`）的 units / abilities / upgrades / items 与机制冻结常量段进入 `defs_hash` / `content_hash` 快照头部；该变化是定义层内容变化，与 Q2 新增 `w3-core` 顶层权威状态严格区分（ADR 0002 §6-8 / R12.4 / Q2 / Q3）

### Requirement: 4 族按表扩展与命名空间避冲突

系统 SHALL 把 `Race`（`defs.rs:287-293`）扩到 4 变体 `Human` / `Orc` / `Undead` / `NightElf`（Q1' 已拍板双族全量 + P8 两族不重做）；`DefId` 段 `1xx` / `2xx` / `3xx` / `4xx` / `9xx` 与 `8xxx` / `9xxx` 合成段、`70_000`（`command_card.rs:47`）、`1_000_000`（`CATEGORY_SLOT_BASE`）逐一避冲突；`RACE_ROWS` 4、`RACE_TRAIN_TARGETS` 4 行×9 列、其余按表项扩到 4 族（逐项见下方 Scenario）。分派 MUST 保持「单一查表处 + 数据驱动」、`race_row()` / `race_segment()` 仍为唯一两处查表点、MUST NOT 散落 `if race` 白名单（R1.2 / R2 / R13.3 / `BUILD_SYSTEM.md:296-298,344-350` / D6）。

#### Scenario: 命名空间逐一核对无冲突且单一查表处

- **WHEN** 枚举 `1xx` / `2xx` / `3xx` / `4xx` / `9xx` / `8xxx` / `9xxx` 段、`70_000` 按钮 ID、`1_000_000` 槽位 ID 与既有全部 DefId，并核对 `race_row()` / `race_segment()`
- **THEN** 无任一重复或跨界重叠，`RACE_ROWS == 4`、`RACE_TRAIN_TARGETS` 4 行、按表扩展后仍由数据驱动、不散落 `if race` 白名单（R2 / R1.2 / R13.3 / Q1' / Q5' / D6）

#### Scenario: 分段 / 查表回归与反向验证

- **WHEN** 以退化实现（漏接不死段 / 暗夜段 `defs_hash` 敏感性、把 `3xx` / `4xx` 段混入 `8xxx`、或散落 `if race` 白名单）替换正式实现并跑回归
- **THEN** 对应断言在缺陷态失败、修复态通过，两条证据可复现（`CODEBUDDY.md §5` 形态① / `defs.rs:1151-1527` 先例 / Q9' 已拍板）

### Requirement: `COMBINATIONS` 4→16、双族对抗与不死席位

`MeleeSetup::with_races` / `COMBINATIONS`（`setup.rs:970-975`）SHALL 由 2×2 四组合扩到 **4×4 十六组合**（含同族组合；Q1' 已拍板双族全量 + P8 两族不重做），本地席位用 `local_race` 段、AI 席位用 `ai_race` 段、中立恒 `9xx`（R2 / R12.7 / Q1'）。系统 MUST 沿用既有 `GameplayCommandKind` 码 1–23 / `resolve_victory` / `AiScript` 命令面与 `EffectDef` 组合；Q2 / Q3 新增机制命令走码 24 起（既有 1–23 不动、另见「新增命令 / 事件变体」Requirement）、MUST NOT 引入 JASS VM（N5 / R7.6 / ADR 0002）。`with_race` 别名与 `MeleeSetup::new()` MUST 逐字节不变（R12.7）。

#### Scenario: 十六组合装配段位正确且既有入口不变

- **WHEN** 以 4×4 十六组合分别装配开局（含不死 / 暗夜席位与不死 vs 暗夜双族对抗），并分别以 `with_race(race)` 与 `with_races(race, race.opponent())`、`MeleeSetup::new()` 比对命令字节
- **THEN** 十六组合本地 / AI 席位 `def_ids` 落在各自族段（`1xx` / `2xx` / `3xx` / `4xx`）、同族组合双方段位一致、中立恒 `9xx`；三者 `Spawn` / `gameplay_commands` / `kinds` / `def_ids` 逐字节一致，既有回落基线测试仍绿（R2 / R12.7 / Q1' / Q5'）

#### Scenario: 既有命令码不变且 AI 命令按自身族段

- **WHEN** 以十六组合各生成 `AiScript` 并枚举 `AI_PLAYER` 命令，同时校验 `GameplayCommandKind` 既有 1–23 码
- **THEN** AI 训练 / 施法 / 研发 DefId 落在 `ai_race` 段（含 `3xx` / `4xx`）、命令为帧冻结外部命令且按 `(execute_frame, player_id, sequence)` 稳定排序；既有码 1–23 编码与字节不变、无 JASS VM 入口（R2 / R4.3 / R4.4 / R6 / R7.6 / R12.7 / ADR 0002 / Q1' / Q3）

### Requirement: CPU 一局打完与逐帧哈希一致、重开无残留

系统 SHALL 支持 CPU 无渲染一局按「采集 → 建造 → 训练 → 英雄施法 → 研发 → 族特色机制触发 → 战斗 → 摧毁全部建筑结算 → 重开」跑完（`docs/roadmap/P8-human-orc-melee.md:148`），含不死族 / 暗夜族对局与 Q2 / Q4 全量机制；每阶段 MUST 可执行验证、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节 MUST 与同脚本两实例 **v8 新基线**一致（R3.1 / R4.1–4.6 / R5 / R6 / R10.2-4 / R11.1 / R11.4 / R11.5 / R12.4 / ADR 0002）。胜负 MUST 只结算一次、按稳定顺序，重开 MUST 不残留上一局权威状态（R11.4 / R11.5）。验收口径 MUST 为「同一组合两实例逐帧哈希一致」、MUST NOT 断言跨组合或与改动前 v7 旧基线哈希数值相等（新增权威状态 + 定义层内容变化 → golden hash 按新实现如实重取）。

#### Scenario: 双族全量一局全流程与结算

- **WHEN** 以不死族 / 暗夜族装配注入完整一局脚本并推进到敌方主城被摧毁、触发 `Leave`
- **THEN** 采集阶段黄金确定增长、建造 / 训练生成 `speed_raw == 0` 的新建筑与可移动新单位、英雄 `Cast` 按 `EffectDef` 组合结算、`Research` 生效、腐地 / 昼夜 / 隐身目标判定 / 月亮井 / 缠绕金矿等机制按 Q2 权威状态确定性结算，既有 `resolve_victory` 只结算一次、`victory.winner` 为本地玩家且结算帧进入权威状态（R3.1 / R4.1–4.6 / R5 / R6 / R7.6 / R10.2-4 / R11.1 / R11.4 / Q2 / Q4 / Q1'）

#### Scenario: 组内逐帧哈希一致与重开无残留

- **WHEN** 同一组合两实例注入同一脚本逐帧推进全部阶段，并在结算后以相同冻结定义与开局脚本重开
- **THEN** 每帧 `compute_state_hash()` / `capture_snapshot()` 字节逐条相等（不只比最终 hash）、`first_difference()` 为 `None`、不比 v7 旧基线数值 / 不跨组合相等；新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态（R11.5 / R12.4 / ADR 0002 / Q3 / Q5'）

### Requirement: 族特色机制纳入 `w3-core` 顶层权威状态（Q2 已拍板）

族特色机制——不死族腐地建造约束（R4.6 `WC3-MELEE-RULES.md:147`）、尸体 / 食尸回血 / 召唤骷髅（`:119`）、诅咒金矿采集（R1.5 `:105`）、古树状态、昼夜相位、隐身目标判定——SHALL **纳入本期并进 `w3-core` 顶层权威状态**（用户 2026-11-22 Q2 明确拍板、偏离 AI 推荐、MUST NOT 写成 AI 推荐）。新增权威状态 MUST 落 `BTreeMap` / `BTreeSet` 确定性有序容器、按稳定 `EntityId` 顺序遍历（ADR 0002 §6-8）；MUST NOT 复用 `Player.race` / `RtsMatch.race` 分类字段（Q6）、MUST NOT 把 `fog` / 视野可见性带进权威状态（R10.5 / §0.3 T3 / Q2' 已拍板）。`git diff -- crates/w3-core/` **非空属预期**（`faction-definitions/spec.md:24,36` P8.5a–d 先例）。

#### Scenario: 顶层权威状态段确定性有序且四处同步

- **WHEN** 构造含族特色机制的 `RtsMatch` 并枚举其新增顶层权威状态段
- **THEN** 每组段以 `BTreeMap` / `BTreeSet` 承载、按稳定 `EntityId` 顺序遍历，且同步进入 `compute_state_hash`（`sim_match.rs:3479-3546`）、由 `SNAPSHOT_VERSION`（bump 8）定义的 `Snapshot` 新增顶层字段（`snapshot.rs:102-146`）、`first_difference()`（`sim_match.rs:3550-3617`）、`restore_snapshot`（`sim_match.rs:3856-3859`）**四处同步**（ADR 0002 §6-8 / `:115` / R5 / Q2 / Q3）

#### Scenario: 机制按整数帧确定性结算且无种族名硬编码

- **WHEN** 推进若干逻辑帧触发腐地扩张 / 尸体衰减 / 诅咒金矿采集 / 昼夜相位切换 / 隐身目标判定
- **THEN** 全部按 30 Hz 整数帧、`SimFixed` Q47.16、稳定排序确定性结算、无墙钟 / `f64` / `HashMap` 迭代顺序 / 表现层独立随机源；合法性由定义层数据驱动、MUST NOT 用建筑名 / 种族名白名单（R1.2 / R4.6 / R1.5 / R10.2 / ADR 0002 §2-3 / 对照差异 R5 / R7）

#### Scenario: 本期纳入的机制不冒充「全部族特色机制已完成」

- **WHEN** 核对 Q2 已拍板机制与 Q4' 已拍板 1.0 范围张力
- **THEN** 本期按 Q2 纳入的机制 MUST NOT 冒充「1.0 四族全部族特色机制已完成」；`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226` 字面「一并做完」与本期双族全量的关系七处显式如实登记、不静默窄化、不冒充已解决（Q4' 已拍板 / §14 / R4）

### Requirement: 全量机制本期纳入（Q4 已拍板）

隐身反隐（R10.4）、昼夜循环全量（R10.2）、R10.3 回血、R5.6 视野阻挡、S3 暗夜夜间隐身特色、R8.4 野怪睡眠 SHALL **纳入本期**（用户 2026-11-22 Q4 明确拍板「本期含全量」、偏离 AI 推荐、MUST NOT 写成 AI 推荐、MUST NOT 静默窄化为「后续 change / P9+ 四族承接」）。这些机制 MUST 与 R10.5 / §0.3 T3「迷雾 / 视野表现层只读派生、不进权威状态 / 哈希 / 快照 / `first_difference()`」显式区分：迷雾可见性 MUST NOT 被带进权威状态（Q2' 已拍板边界论证、不抹平对照差异 D1）。B 类 `[待核]` 数值与两套昼夜常量口径按 Q3' 已拍板 / §14 由人核对（实施期数值核对、非规格门待拍板范围 / 架构 Q）。

#### Scenario: 全量机制本期可验收且 fog 可见性保持只读派生

- **WHEN** 在本期实现上分别触发隐身反隐 / 昼夜循环 / R10.3 回血 / R5.6 视野阻挡 / 暗夜夜间隐身 / 野怪睡眠，并校验迷雾 / 视野可见性路径
- **THEN** 六类机制均可执行验证且按 Q2 口径确定性结算；迷雾网格 / 当前可见 / 已探索层 / 视野范围仍是表现层只读派生、不进 FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION` 定义的快照 / `first_difference()`，vera20k fog 进哈希 D1 仍是「对照差异、明确不借鉴」（R10.2 / R10.3 / R10.4 / R10.5 / R5.6 / R8.4 / S3 / §0.3 T3 / ADR 0002 §5 / Q2' / Q4）

#### Scenario: 双族全量范围已收敛且 1.0 张力保留显式

- **WHEN** 校验 Q1' 已拍板「不死 + 暗夜双族全量」与 Q4 全量（含 S3 暗夜夜间隐身特色）的落地关系
- **THEN** 「本期不死 + 暗夜双族全量收敛」为确定性 SHALL、`COMBINATIONS` 16 组合、不重做 P8 人 / 兽两族；1.0 字面「一并做完」与本期双族全量的张力七处显式呈现、不静默窄化、不冒充已解决（Q1' / Q4' 已拍板 / R2 / R4）

### Requirement: bump 8 全套与旧 v7 明确拒绝（Q3 / Q5' 已拍板）

`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` SHALL 按 Q2 结果 **7→8**（用户 2026-11-22 Q3 / Q5' 明确拍板全套、MUST NOT 沿用 P8.5e「保持 7 不 bump」口径背书、MUST NOT 沿用 P8.5a–d「逐期 bump」惯性反推——本次是 Q2 拍板结果、不是惯性）。旧 v7 快照 / 回放 MUST 按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移；`Packet::decode` 对 v7 MUST 返回 `VersionMismatch`、relay MUST `Reject`（ADR 0002 §7 / ADR 0005 §3 / Q5'）。版本号硬断言、`w3-render` 版本号硬断言、`openspec/config.yaml:13,56` 与 `CODEBUDDY.md §5:50` 的「当前 7」口径须按 Q5' 已拍板清单同步为 8。

#### Scenario: 三版本号一致 bump 8 且旧 v7 明确拒绝

- **WHEN** 校验全仓版本号硬断言并以 v7 快照 / 回放 / 数据包分别尝试恢复与解码
- **THEN** `SNAPSHOT_VERSION == SIM_VERSION == PROTOCOL_VERSION == 8` 且全仓一致；v7 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝，`Packet::decode(v7)` 返回 `VersionMismatch`、relay `Reject`，无静默兼容或跨版本迁移（R12.4 / ADR 0002 §6-8 / ADR 0005 §3 / Q3 / Q5'）

#### Scenario: 不静默沿用旧断言且不冒充已锁定

- **WHEN** 核对 bump 8 口径与首轮四件套「保持 7 不 bump」历史断言、旧指纹 `2776dd7f…` / `1d01514b…` 状态
- **THEN** 历史「保持 7 / golden hash 逐字节不变 / `git diff -- crates/w3-core/` 为空」断言 MUST NOT 沿用；本轮按 `CODEBUDDY.md §8` 触发规格解锁、旧两枚指纹均作废、不回填、`status: pending`、不 self-approve、不把首轮④「批准并锁定（推荐）」当成已锁定（R6 / R19 / R21 / Q5' / Q10'）

### Requirement: 新增权威状态进入状态哈希与快照、四处同步

按 Q2 新增的族特色机制顶层权威状态 MUST 声明进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（HEAD 7 → 本 change bump 8）定义的 `Snapshot` 新增顶层字段与 `first_difference()`，并在 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` **四处同步**（ADR 0002 §6-8 / `:115` / `CODEBUDDY.md §5:51` / `openspec/config.yaml:56`）。冻结机制常量 MUST 经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与新增权威状态严格区分）；三者字段集合按 Q2 新增段如实扩展、MUST NOT 沿用「字段集合不变」旧断言（R12.4 / Q2 / Q3 / Q5'）。

#### Scenario: 新增字段四处同步且遗漏被门禁拦截

- **WHEN** 以「新增权威状态漏进 `compute_state_hash` / `Snapshot` / `first_difference()` 之一」的退化实现替换正式实现，并跑 `cargo test -p w3-core --test determinism`（26 项）
- **THEN** 「改坏验红」元断言在缺陷态失败、修复态通过，四处同步回归绿；冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部、与新增权威状态严格区分（ADR 0002 §6-8 / `CODEBUDDY.md §5` 形态① / Q2 / Q3 / Q5'）

#### Scenario: 快照恢复与逐帧哈希一致

- **WHEN** 捕获含族特色机制权威状态的 v8 `Snapshot`、销毁实例、`restore_snapshot` 恢复后逐帧推进
- **THEN** 恢复后逐帧 `compute_state_hash()` 与原实例一致、`first_difference()` 为 `None`、v8 快照头部 `defs_hash` / `content_hash` / `version` / `sim_version` 校验通过；旧 v7 快照明确拒绝（ADR 0002 §7 / §6-8 / R12.4 / Q3）

### Requirement: 新增命令 / 事件变体与 proto / 传输层拒绝

系统 SHALL 按 Q2 / Q3 新增 `GameplayCommandKind` 变体（`hash_gameplay_command` 唯一 kind 码 **24 起**、既有 1–23 不动、**校验 / 穷尽分发三处同步**）与 `EventKind` 事实事件变体（`Event` 结构不变、无 payload）；`w3net.proto` MUST 只追加 tag **46 起**、`reserved 14 to 32` 保持、字段编号只追加（ADR 0005 §3）；`w3-net` / `w3-relay` 改动 MUST 只限 payload 编解码 + 版本拒绝，W3N1 25 字节帧头 / `FrameScheduler` / `stabilize` / `apply_frame` / 断线重连游标不动；`w3-core` MUST NOT 反向依赖 `w3-net` / `w3-relay`（R16 / ADR 0005 §4）。

#### Scenario: 只追加编码与穷尽分发三处同步

- **WHEN** 枚举新增 `GameplayCommandKind` 码 24 起与 `EventKind` 变体、proto tag 46 起，并校验既有 1–23 码与 `reserved 14 to 32`
- **THEN** 新码 / 新 tag 只追加、既有 1–23 编码与字节逐字节不变、`reserved` 段不动；`hash_gameplay_command` 与校验 / 穷尽分发三处同步、无遗漏分支默认放行；新增命令经既有公开命令入口确定性结算（R12.2 / R12.4 / ADR 0002 / ADR 0005 §3 / Q2 / Q3 / Q5'）

#### Scenario: 线格式只传命令且旧版本拒绝

- **WHEN** 一端逐帧推进、另一端通过 relay 提交 v8 与 v7 命令包并校验 payload
- **THEN** 线格式只忠实传输玩家输入命令、MUST NOT 含游戏状态 / fog 可见性；v8 命令包解码成功、v7 命令包 `Packet::decode` 返回 `VersionMismatch` + relay `Reject`，两端逐帧哈希一致（R12.2 / ADR 0005 §3-4 / Q3 / Q5'）

### Requirement: golden hash 如实重取与 determinism 26 项

因 Q2 新增权威状态与定义层冻结常量改变逐帧哈希布局，golden hash 基线 MUST 按 v8 新实现**如实重取**、不沿用旧值 `0x940cb016f6630f1e`、不声称逐字节不变（ADR 0002 复核记录 P8.5a–d 先例、R6）。系统 MUST 重跑 `cargo test -p w3-core --test determinism -- --nocapture`（26 项）全绿，证明命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、30 Hz 整数帧、稳定 `EntityId` 顺序、`SimFixed` Q47.16、FNV-1a 编码方式不变；ADR 0002 / ADR 0005 复核记录按 Q5' 已拍板清单追加、不删改 §1–§8 正文。

#### Scenario: 逐帧文本重取且 v8 Debian 侧比对待人工

- **WHEN** 用 `crates/w3-core/examples/golden_hash.rs` 固定 17 条命令脚本 / 40 帧重新生成逐帧哈希文本
- **THEN** Windows 侧 `final_hash` 与整份文本 sha256 按 v8 新实现重取、记录值仅作人工比对留档、不作自动断言；**v8 Debian 12 侧逐字节离线比对仍待 VM / 人工**，macOS / ARM64 / 其它 rustc 待验证、不声称全平台达成或有新验证（ADR 0002 `:102-108,110-116` / `CODEBUDDY.md §5:54` / Q5' / Q8'）

#### Scenario: determinism 26 项全绿

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项）与 `cargo test --workspace`
- **THEN** 26 项与全量测试全绿，命令排序键 / 整数帧 / 稳定 `EntityId` 顺序 / FNV-1a 编码不变、golden_hash.rs 基线按新实现留档（`CODEBUDDY.md §5` / `openspec/config.yaml` `rules.tasks` 第二、三条 / R12.4 / ADR 0002 §8 / Q3）

### Requirement: 反向验证形态①与门禁

新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（`CODEBUDDY.md §5`），采用**证据形态①**（测试内反例 + 元断言防退化）；退化实现 MUST 真经 `capture_snapshot()` / `compute_state_hash()` / `restore_snapshot()` / `first_difference()` / `synthetic_definitions()` / `race_row()` / `MeleeSetup::with_races` 生产路径注入、非旁路模拟，MUST NOT 冒充形态②。系统 MUST 跑 `cargo test --workspace` / `cargo fmt --all -- --check` / `check_encoding.py` / `openspec validate --strict`；`status: pending` 令 `spec_gate_check()` 判红是 fail-closed 正确信号、不绕过不改门禁（Q10'）。

#### Scenario: 形态①成对元断言覆盖权威状态 / 哈希 / 快照

- **WHEN** 以退化实现（新增权威状态漏进 `compute_state_hash` / `Snapshot` / `first_difference()` / `restore_snapshot` 之一、用 `HashMap` 迭代顺序、把 fog 可见性反写权威状态、把新建筑 `speed_raw` 改非 0）替换正式实现
- **THEN** 每组「缺陷态失败 / 修复态通过」元断言证据可复现，含建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6），`cargo test -p w3-core --test determinism -- --nocapture` 26 项全绿（ADR 0002 §8 / `CODEBUDDY.md §5` / Q2 / Q3 / Q4）

#### Scenario: 联机两端与回放逐帧一致

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一命令脚本，再把一局存为录像加载逐帧重放
- **THEN** 两端逐帧 `compute_state_hash()` / `capture_snapshot()` 字节逐条一致、`first_difference()` 为 `None`；回放从同一命令序列 + 同一冻结定义哈希 / 内容哈希重建、逐帧哈希与原局一致，线格式只传玩家输入命令、MUST NOT 含游戏状态（R12.2 / R12.4 / ADR 0005 §4 / Q3）

### Requirement: `w3-core` 口径按 Q6（无 `Race` 分类字段）

`Race` 分类字段 MUST NOT 升为 `w3-core` 定义层 / 权威状态字段（Q6 按推荐、用户拍板生效）；族别仍由 client `DefId` 段表达、`w3-core/src/` 全仓 `\bRace\b` 零匹配、无 `Player.race` / `RtsMatch.race`、`DefId = u32` 不变（`defs.rs:287-293` 硬红线、`dual-faction/spec.md:10`）。**与 Q2 的区分**：Q2 新增的是族特色机制的独立顶层权威状态段，不是 race 分类字段；两者 MUST NOT 混同。`w3-core` MUST NOT 依赖 `w3-content` / render / ui / audio / net；族分派 MUST 集中单一查表处、由定义层数据决定、MUST NOT 用名字 / 白名单（R1.2 / ADR 0002 §5 / 硬红线）。

#### Scenario: 内核无 Race 且权威状态段不复用 race 字段

- **WHEN** 校验 `crates/w3-core/` 的字段集合、依赖方向与族分派实现
- **THEN** 无 `Race` / `race` 分类字段、`Spawn` 字节不变；Q2 新增的 `blight` / `corpses` / `haunted_mine` / `ancient` / `day_night` / `stealth` 等顶层权威状态段是独立段、不复用 race 分类；`w3-core` 不反向依赖上层、表现层只读不反写（R2 / R1.2 / ADR 0002 / Q2 / Q6）

#### Scenario: 单一查表处与 4 族数据驱动

- **WHEN** 以 4 族分别调用 `race_row()` / `race_segment()` 并校验族规则分派
- **THEN** 4 族均按表命中本族段、分派集中在唯一两处查表点、由定义层数据决定、MUST NOT 散落 `if race` 白名单或按 `DefId` 段位写死机制（R1.2 / R13.3 / D6 / Q1' / Q6）

### Requirement: 范围边界、1.0 张力已拍板登记与 R10.5 边界

本 change SHALL 按用户已拍板口径做**不死 + 暗夜双族全量 + 权威状态 + bump 8**（Q1' / Q2 / Q3 / Q4 已拍板）；**1.0 范围张力 MUST 七处显式如实呈现、不静默窄化、不冒充已解决**：`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226`（R10.2 / R10.3 / R10.4）字面「暗夜夜间隐身 / 昼夜 / 隐身反隐随迷雾纳入 1.0 一并做完」，而 P8.5e 规格门 Q2「最小闭环」曾把它们登记为后续承接（`2026-11-22-p8-fog-of-war/decision-memo.md:80,128`）。Q4 已由用户明确拍板本期含全量（不是 AI 推荐、不是静默窄化）；Q1' 已拍板双族全量；Q4' 已拍板张力保留显式——这是**已拍板后的如实登记、不是待拍板项**。

#### Scenario: 张力七处显式呈现且不静默窄化

- **WHEN** 核对四件套 + 纪要中 1.0 范围张力的呈现位置与处置口径
- **THEN** `proposal.md` Scope / What Changes、`design.md` Risks /「## 范围边界」/「## 对照差异」D4、本 Requirement 与 `night-elf-faction` 范围边界 Requirement、`decision-memo.md`「## 范围张力登记」/「## 阶段声明」、`tasks.md` 第 8 组、8 份 MODIFIED delta 共七处显式呈现字面「一并做完」与本期双族全量的关系，处置按 Q4 / Q4' 已拍板、MUST NOT 写成待拍板结论、MUST NOT 静默窄化用户 2026-11-14 已拍板的 1.0 范围（S3 / §0 / §0.3 T2 / R10.2-5 / §14 / Q4'）

#### Scenario: Q2' R10.5 边界与范围外项显式登记

- **WHEN** 校验本期不做项与 Q2' / Q7' 已拍板承接关系
- **THEN** 迷雾 / 视野可见性 MUST 表现层只读派生、fog 可见性 MUST NOT 进权威状态、vera20k fog 进哈希 D1 仍是「对照差异、明确不借鉴」、MUST NOT 静默改规则（§14）；AI×lockstep 接合（§0.3 T5）/ R9 中立建筑交互全量 / R13 grid / R3.5 资源共享 / R5.5 高地加成 / R12.5 观战·断线重连 / N1–N6 / P4 底座重做全部显式登记为后续 change / 已知限制、不冒充已解决（R10.5 / §0.3 T3 / §0 / §14 / Q2' / Q7'）

### Requirement: 零资产、许可证与 B 类 `[待核]` 常量落点按 Q3' / Q7

B 类 `[待核]` 数值 / 键位 / 昼夜时长 / 腐地扩张速率 / 尸体衰减 / 月亮井恢复量 / 缠绕·诅咒金矿间隔 / 隐身反隐半径 MUST 不采信、不冻结、不入库，权威来源仅本机导出 SLK / MiscGame 或原版客户端，与导出不符先标 `[待核]` 按 §14 由人核对（Q3' 已拍板；实施期数值核对、非规格门待拍板范围 / 架构 Q）。两套昼夜常量（`fog.rs:59-62` 的 `600/480` vs `CreepConstants` `600/600`，`creep.rs:153-154`）口径未统一 MUST 按 Q3' 由人核对、不静默改规则；`[待核]` 常量 MUST NOT 冒充原版。仓库 MUST 零资产、不读 `config.toml`、不跑导出脚本；AGPLv3 只借语义零 Java 复制、`vera20k` 零复制、MIT 项目借结构不借实现且 `double` 不采用（ADR 0001 / ADR 0004 / §14 / `:373-384` / Q3'）。

#### Scenario: 干净构造与防误用清单

- **WHEN** 在干净环境构造不死族 / 暗夜族定义与族特色机制冻结常量并核对常量落点、许可证与资产扫描
- **THEN** 构造成功零文件 IO、B 类 `[待核]` 数值不冒充原版、不含 Warsmash `GRID_STEP=128` / `PATHING_RATIO=4` / `DYING_UNIT_VISION_RADIUS` / `abilityBehaviors/*.json` 数值与 rawcode / godot `FACTION_UNIT_ROSTERS.md:37-45` / `CLASSIC_RULES_BASELINE.md:15-19` 网页值 / `melee.jui`，`python tools/verify_all.py` 资产扫描绿、无新增未登记依赖、`NOTICE.md` / `Cargo.lock` 无 diff（ADR 0001 / ADR 0004 / R2 / §14 / Q3' / Q7）

#### Scenario: 两套昼夜常量口径未统一由人核对

- **WHEN** 核对表现层 `FOG_DAY_FRAMES=600` / `FOG_NIGHT_FRAMES=480`（`fog.rs:59-62`）与 `CreepConstants` `day_frames=600` / `night_frames=600`（`creep.rs:128-131,153-154`）的差异
- **THEN** 差异 MUST 如实登记为口径未统一、按 Q3' 已拍板由人核对（实施期数值核对）、MUST NOT 静默改 `WC3-MELEE-RULES.md` R10.2 或把任一值冻结进权威定义冒充经典事实（R10.2 `[已核 / 时长待核]` / §14 / Q3'）

### Requirement: 真窗口人工验收与跨平台 / CI 按 Q8' 保持未勾

`tasks.md` 第 8 组真窗口清单 8.x MUST 全部保持 `- [ ]`「未执行 / 待人工」，不提供替代、不用离屏截图 / CPU 回归冒充真窗口验收、CI MUST NOT 声称已验证 GPU；先离屏 `render_headless_rgba` + `ViewImage` 自证 + CPU 回归全绿再请人工验收（GUI 类修复先自测纪律、不反复试错）。跨平台确定性 MUST 只沿用 ADR 0002 复核记录「Windows 11 / Debian 12 x86_64 / rustc 1.99.1（v3）已逐字节验证」，v4–v7 Debian 侧逐字节比对、**v8（本轮 bump）比对**、macOS / ARM64 / 其它 rustc 仍待验证、不声称全平台达成或有新验证；CI 双平台 `verify_all.py` 本会话未运行、如实标注（R12.4 / `playable-window-entry/spec.md:149-176` / Q5' / Q8'）。

#### Scenario: 清单保持未勾且人工边界如实说明

- **WHEN** 核对 `tasks.md` 第 8 组与人工边界登记
- **THEN** 8.x 全部 `- [ ]`「未执行 / 待人工」、无证据项如实标注不冒充通过、Q1'–Q10' 已拍板结论下 1.0 字面「一并做完」张力保持显式呈现（Q8' / Q4' / S3）

#### Scenario: 跨平台与 CI 不冒称

- **WHEN** 核对 ADR 0002 复核记录、v8 golden hash 基线与 CI 记录
- **THEN** 仅沿用 v3 双平台已验证结论、v4–v8 Debian 侧 / macOS / ARM64 / 其它 rustc 标注待验证、不声称全平台达成或有新验证；CI 双平台 `verify_all.py` 本会话未运行、不冒充已绿，`status: pending` 令 `spec_gate_check()` 判红是 fail-closed 正确信号、不绕过不改门禁（ADR 0002 `:102-108,110-116` / `CODEBUDDY.md §5:54` / Q8' / Q10' / R21）
