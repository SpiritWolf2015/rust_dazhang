# Spec Delta

## Purpose

本规范定义 P11 技能系统「系统该做什么」：四族技能清单按本机导出 SLK 清单逐项对齐并人工固化成编译期常量表（内部标识仍为合成原创 `DefId`）、英雄技能学习 / 等级 / 大招、行为原语注册表与施法命令面、道具技能经 `EffectDef` 组合，以及新增权威运行态进入 FNV-1a 状态哈希与 `SNAPSHOT_VERSION`（当前 8、本期 bump 9）快照的确定性契约；本期范围 = 四族英雄技能全量 + 各族代表性单位 / 道具技能，中立英雄 / 野怪 / 全部单位技能顺延标已知限制。相关 ADR：0001 / 0002 / 0003 / 0004 / 0005；业务规则以 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 S1–S6 为需求源。

> 相关 ADR：ADR 0001（资产 / 法律）、ADR 0002（确定性契约）、ADR 0003（范围 / 不做 JASS VM）、ADR 0004（冻结定义）、ADR 0005（protobuf canonical）。本 capability 为新增，`deterministic-simulation/spec.md:44` 已含「若拍板确需新增字段 MUST 四处同步并按 ADR 0002 bump 8→9 全套」转义条款，故不写 MODIFIED delta（见 `design.md` D0）。**编号复用澄清**：此处 Q1 为 **P10 规格门 Q1**（版本与权威状态处置），与 P11 Q1（技能清单来源）编号复用含义不同；P11 授权来自用户 2026-11-24 拍板 P4『四族全量 + bump 9 全套』，**不依赖该转义条款**。

## ADDED Requirements

### Requirement: 四族技能清单按 SLK 逐项对齐并人工固化

系统 SHALL 把四族技能清单逐项按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）人工核对后固化成编译期常量表进 `FrozenDefinitions`；对齐的是清单条目与分类、MUST NOT 照搬经典名称 / rawcode / 数值。内部仍用合成原创 `DefId` 作键、MUST NOT 引入四字符 ID → `DefId` 映射层、MUST NOT 冒充经典具名技能，数值一律 `[待核]` 且 MUST NOT 冻结进权威定义（S3 / R2 / R6 / R7.6 / ADR 0001 / 0002 / 0004）。定义层冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部，与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）。

#### Scenario: 四族技能清单全量对齐与冻结常量

- **WHEN** 按 S3 / R6 以本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）逐项核对四族技能清单并人工固化成 `FrozenDefinitions` 编译期常量表
- **THEN** 清单条目与分类逐项对齐经典具名技能（哪些技能、每族几个、英雄 3 普通 + 1 个 6 级大招、代表性单位技能、道具技能）；本期范围 = 四族英雄技能全量（经典清单结构每族 4 英雄 × 4 技能）+ 各族代表性单位 / 道具技能，中立英雄 / 野怪 / 全部单位技能顺延并如实标已知限制、不臆造「四族全量 = 某精确数字」（实际条目数在 apply 阶段人工核对后如实登记，`WC3-MELEE-RULES.md §14`）；`DefId` 类别内唯一且仍为合成原创（沿用 `ABILITY=10` / 英雄 `11–14` 并扩四族）、无四字符 ID → `DefId` 映射层、SLK 四字符 ID 只作核对依据不进权威状态 / 状态哈希 / 快照 / 不入库；冻结常量经 `compute_defs_hash`（`gameplay.rs:434`）进入 `defs_hash` / `content_hash` 快照头部、与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）；`cargo test -p w3-core --test determinism` 全绿（S3 / R6 / R2 / ADR 0002）

#### Scenario: 两层张力与零资产合规

- **WHEN** 核对技能定义来源、清单对齐层次与仓库跟踪文件
- **THEN** 「清单结构 / 条目逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」被显式区分为两个层次——对齐的是清单条目与分类、MUST NOT 照搬经典名称 / 四字符 rawcode / 数值；命名非经典具名、数值均 `[待核]`、以本机导出 SLK 人工核对、不静默改规则（`WC3-MELEE-RULES.md §14`）；SLK 四字符 ID 只作核对依据、不进权威状态 / 状态哈希 / 快照；无 `*.slk` / MPQ 派生 JSON / PNG / GLB 入库（`python tools/verify_all.py` 资产扫描通过），本机导出表规模元数据只登记规模与分类、不登记任何具体技能名 / rawcode / 数值（见 `design.md`「清单规模事实锚」，导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库，ADR 0001）（R6 / R7.6 / ADR 0001 / 0002 / 0004 / `WC3-MELEE-RULES.md:7,124,351,374`）

### Requirement: 英雄技能点 / 等级 / 学习 / 升级与大招

系统 SHALL 以确定性权威状态承载英雄技能点、等级与已学技能等级，6 级解锁大招；学习 / 升级 MUST 经确定性命令面结算、逐帧可重放（R6.1 / R6.4 / R12.4 / ADR 0002）。

#### Scenario: 技能学习逐帧确定

- **WHEN** 两实例交错推进同一「升级 → 学习 → 6 级学大招」命令帧流
- **THEN** 技能点 / 等级 / 各技能等级逐帧一致、`compute_state_hash()` 相等、`first_difference()` 返回 `None`；6 级前大招不可学、`cargo test -p w3-core --test determinism` 全绿（R6.1 / R12.4 / ADR 0002 §6-8）

### Requirement: 行为原语注册表与 EffectKind 最小扩展

系统 SHALL 扩展最小必要 `EffectKind` 原语集（持续 AOE / zone、召唤、光环受益集、被动 proc 订阅、变身 / 状态切换、点目标、buff 分类移除、链接弹射、随机消费），并经单一 `BehaviorCatalog` 注册表按行为类型分派；注册新行为 MUST 为扩展而非散落分支（R6 / R5 / R7.6 / ADR 0002 / OCP）。

#### Scenario: OCP 注册表与四处同步

- **WHEN** 新增一种行为原语并在 `BehaviorCatalog` 注册后跑结算与哈希
- **THEN** 结算入口查表分派、无散落 `if behavior == ...` 分支；每个原语对应的权威运行态进入状态哈希 / 快照 / `first_difference()` / `restore_snapshot()` 四处同步，`cargo test -p w3-core --test determinism` 反例断言全绿（R6 / R12.4 / ADR 0002 §6-8 / SRP / OCP / DIP）

### Requirement: 施法命令面、目标形态与 R1 可移动性

施法 SHALL 走 `GameplayCommandKind::Cast`（点目标 / 范围 / 方向按「只追加 kind 码 / proto tag」扩展）；建造走 `Train { is_building: true }`，攻击 / 移动 / 停止走 `CommandKind::{Attack, Move, Stop}`；MUST NOT 出现 `GameplayCommandKind::Attack` / `::Build`，命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序（R1 / R1.4 / R12.2 / ADR 0002 / ADR 0005）。

#### Scenario: 命令面分派与穷尽分发

- **WHEN** 提交施法 / 建造 / 攻击 / 移动 / 停止命令并跑穷尽分发与 protobuf 往返
- **THEN** 分别落到 `Cast` / `Train { is_building: true }` / `CommandKind::{Attack, Move, Stop}`，无 `::Attack` / `::Build` 变体（`gameplay.rs:1322-1546` / `command.rs:11-49`）；同源校验 + 稳定排序通过（R1 / R12.2 / ADR 0002 §2-3 / ADR 0005 §3）

#### Scenario: 框选含建筑只为可移动单位生成 Move

- **WHEN** 框选同时含金矿 / 树木 / 建筑 / 资源点与可移动单位并下达 `Move`
- **THEN** 只为可移动单位生成 `Move`、落点按稳定 `EntityId` 顺序；金矿 / 树木 / 建筑 / 资源点 MUST NOT 出现在 `Move` 命令中、MUST NOT 改变位置（R1.1 / R1.4 / R13.6 / `unit-orders` spec），`cargo test -p w3-core --lib` 的 R1 回归全绿

### Requirement: 道具技能走 EffectDef 组合（R7.6）

道具 abilList / 效果 SHALL 一律走既有 `EffectDef` 组合结算，MUST NOT 直接执行任意 JASS 技能、MUST NOT 引入 JASS / wtg 触发器 VM（R7.6 / N5 / ADR 0003）。

#### Scenario: 道具效果不经 JASS 路径

- **WHEN** 使用任一含 abilList 的道具并核对效果结算路径
- **THEN** 效果经 `Settlement` 单一结算所有者（`gameplay.rs:851-853`）+ `EffectDef` 组合消费，无 JASS 直调；`crates/` 内无 `war3map.j` / 触发器 VM 引用（R7.6 / `gameplay.rs:2318` / `0003-scope.md:15,38`）

### Requirement: 新增权威运行态进入状态哈希与 bump 9 快照

技能系统新增的全部权威运行态（zone / 召唤 / 光环受益集 / 被动 proc / 变身 / buff 分类 / 链接 / `rng_draws`）SHALL 用 `BTreeMap` / `BTreeSet` 稳定序、按稳定 `EntityId` 顺序，同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8、本期 bump 9）定义的快照、`first_difference()` 与 `restore_snapshot()` 四处（R12.4 / ADR 0002 §6-8）。

#### Scenario: 四处同步与改坏验红

- **WHEN** 故意漏将某一技能权威运行态字段写入哈希或快照其一，再以完整实现重跑
- **THEN** `cargo test -p w3-core --test determinism` 的反例元断言在缺陷态失败、修复态通过（形态①）；完整实现下四处同步、双实例逐帧哈希一致、快照销毁重建后从 N+1 续算一致（R12.4 / `CODEBUDDY.md §5` / ADR 0002 §6-8）

#### Scenario: v9 快照恢复不重复施放

- **WHEN** 捕获含进行中施法 / 冷却 / buff / zone / 召唤 / `rng_draws` 的 v9 快照，销毁实例后恢复并回放
- **THEN** `SNAPSHOT_VERSION == SIM_VERSION == PROTOCOL_VERSION == 9`，技能运行态完整恢复、不重复施放、不重复扣费、不重复消费随机码（R12.4 / ADR 0002 §7 / `deterministic-simulation/spec.md:58-61` 快照往返先例）

### Requirement: 随机流消费契约

技能相关随机 SHALL 复用单条 match-owned 整数 LCG、每帧恰好消费一次，消费顺序以显式 `rng_draws` 编码进快照 / `first_difference()`；MUST NOT 新增 per-skill 独立流或让表现层消费权威流（R12.4 / ADR 0002 §4）。

#### Scenario: 单流逐帧一致

- **WHEN** 两实例交错推进含被动 proc / 闪避 / 暴击 / 链接随机消费的技能帧流并捕获快照
- **THEN** 每帧只消费一次主 LCG、`rng_draws` 序列逐帧相等、`first_difference()` 返回 `None`、恢复后续算不重复（R12.4 / ADR 0002 §4 / P8.5c `0002-determinism.md:113` 先例）

### Requirement: bump 9 全套与旧 v8 明确拒绝

系统 SHALL 把 `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 三者同步 bump 8→9；`w3net.proto` 只追加 tag、`reserved 14 to 32` 与 `EffectDef reserved 4 to 8` 保持；旧 v8 快照 / 回放 / 包 MUST 按 `UnsupportedVersion` / `SimVersionMismatch` / `VersionMismatch` + relay `Reject` 明确拒绝（R12.4 / ADR 0002 §7-8 / ADR 0005 §3）。

#### Scenario: 旧 v8 拒绝与三版本号一致

- **WHEN** 以旧 v8 快照 / v8 protobuf 包调用恢复与 `Packet::decode`
- **THEN** 恢复返回 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch`、decode 返回 `VersionMismatch`（`packet.rs:154-156`）且 relay 回 `Reject`、不提供跨版本迁移；新 v9 三版本号一致、`cargo test --workspace` 全绿（R12.4 / ADR 0002 §7 / ADR 0005 §3 / `0005-protobuf-payload.md:87`）

### Requirement: golden hash 重取与跨平台待验证登记

golden hash SHALL 按 v9 新实现如实重取、MUST NOT 沿用 v8 值 `0x0b53a047b1f8e5e0`；Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对 MUST 登记为「待 VM / 人工」，MUST NOT 声称 bump 9 后双平台 / 全平台确定性已验证（R12.4 / ADR 0002 / `0002-determinism.md:118`）。

#### Scenario: v9 基线重取且不冒用旧值

- **WHEN** 运行 `crates/w3-core/examples/golden_hash.rs` 并核对 ADR 0002 追加的复核记录
- **THEN** Windows 侧 `final_hash` 为 v9 重取值、未沿用 `0x0b53a047b1f8e5e0`（`golden_hash.rs:75-77`）；复核记录显式写「Debian 12 / macOS / ARM64 / 其它 rustc 待 VM / 人工」，无「已验证全平台」措辞（R12.4 / ADR 0002 §8 / `0002-determinism.md:118`）

### Requirement: AI 施法边界（本期不含 AI 自主施法）

本期 SHALL NOT 引入 AI 自主施法 / AI 全量行为（顺延 P12、基于 v9 再评估）；技能 MUST 可被真人命令 / 回放 / 联机 lockstep 确定性驱动，`AiScript` 现有取 `Cast` 目标 ID 的只读夹具不顺延改动（R12.7 / §0.3 T5 / S1）。

#### Scenario: 三路径驱动一致且无新增 AI 施法源

- **WHEN** 核对本期 `crates/w3-core/` 是否新增 AI 自主施法决策路径，并以真人 / 回放 / 联机三路径喂同一 `Cast` 帧流
- **THEN** 无新增 AI 施法命令源 / 决策字段；三路径逐帧哈希一致，`SetAi` / `run_ai` 既有行为不变（R12.7 / R12.2 / S1 / `dual-faction/spec.md:36`）

### Requirement: 表现层只读派生与零资产合规

技能特效 / HUD / 冷却显示 SHALL 为表现层只读派生、MUST NOT 反写权威状态或进状态哈希 / 快照 / `first_difference()`；MUST NOT 引入 Warsmash Java / LibGDX 代码或 `*.j` / `.jui`、MUST NOT 引入暴雪资产或 MPQ 派生数据（R10.5 / R7.6 / ADR 0001 / ADR 0002 §5）。

#### Scenario: 表现层丢帧不改权威状态

- **WHEN** 表现层渲染技能特效 / 冷却进度且动画丢帧或资源加载失败
- **THEN** `compute_state_hash()` 与快照逐字节不变、无权威字段被反写；`python tools/verify_all.py` 资产扫描通过、无 AGPLv3 代码复制（R10.5 / ADR 0002 §5 / ADR 0001 / `UPSTREAM.md:109,114`）

### Requirement: 确定性验收与真窗口人工清单

验收 SHALL 含 `cargo test -p w3-core --test determinism`、`cargo test --workspace`、`cargo fmt --all -- --check`、`python tools/verify_all.py` 与四族技能逐帧哈希一致 / 回放一致 / 旧 v8 拒绝；新增确定性回归 MUST 含形态①反向验证；真窗口 8.x 人工项 MUST 保持未勾「待人工」、不冒充通过（R12.4 / `CODEBUDDY.md §5` / §9）。

#### Scenario: CPU 验收全绿且人工清单不冒充

- **WHEN** 执行规格门 Q8 已由用户逐条拍板定案的全部 CPU 验收命令并核对 `tasks.md` 第 8 组
- **THEN** determinism / workspace / fmt / verify_all 全绿、反向验证缺陷态失败修复态通过；真窗口人工清单 `8.x` 全部 `- [ ]` 未勾、标「待人工」（R12.4 / `CODEBUDDY.md §5` / 结论 G8）
