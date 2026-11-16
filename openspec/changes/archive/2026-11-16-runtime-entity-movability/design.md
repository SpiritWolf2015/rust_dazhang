# Design

## Context

动机与 B1 承接见 `proposal.md` Why，此处只记 **leader 本会话只读核实**的现状与约束。锚一律用「文件路径 + 函数名 / 符号名」，不写 `:NN` 行号（本轮 `w3-core` / `w3-client` / `w3-render` 会被 owner 改动，行号会漂移）；对未改动项目文件用章节号 / 规则编号。

内核只读事实：`sim_match.rs::complete_production` 生成运行时实体时同时握有 `EntityId`（`self.entities.spawn()`）与 `unit_def`（`def.speed_raw`，建筑为 `0`），经 `UnitState::new(..., SimFixed::from_raw(def.speed_raw), ...)` 写入 `UnitState.speed`（`sim_match.rs::UnitState` 的既有权威字段，建筑 `speed == 0`、单位 `> 0`），但生成后关联全丢——不把 `unit_def` 存进 `UnitState`、不 push 任何 `Event`、`EntityView` 不暴露 `DefId` / `speed`。`UnitState.speed` **已经**在 `compute_state_hash` 与 `capture_snapshot` 的 `PositionRecord.speed` 中；`sim_match.rs::read_view` 目前只投影 `id` / `x` / `y` / `facing` / `hp` / `owner` / `alive` / `has_target`，未投影 `speed`。`GameplayCommandKind::Train { producer, unit_def, is_building }` 携带 `DefId`，但那是**命令**、不是实体视图。`EntityView` 的构造点分布在 `sim_match.rs::read_view`、`hud.rs`、`tests/interaction.rs`、`tests/movability.rs`、`w3-render/src/model.rs`、`w3-render/src/scene.rs`，加字段须逐一补齐。

frame-0 只读事实：`command.rs::CommandKind::Spawn` 不含 `DefId`；`command.rs::CommandQueue::submit` 校验 `speed.raw() <= 0 → CommandError::InvalidSpawn`，故 `setup.rs::Builder::spawn` 强制 `speed.max(1)`——**frame-0 建筑的运行时视图 `speed > 0`，无法用视图 `speed` 区分建筑**。P8.0 D1 因此以 `input.rs::derive_movability` 从 `FrozenDefinitions::units()` 的 `UnitDef.speed_raw` 派生 `EntityId → 可移动性` 只读映射（`MeleeSetup::movability`），装配期注入。`input.rs::filter_movable` 的 `movability.get(id).copied().unwrap_or(true)` 把「映射未登记的运行时新实体（含建筑）」默认放行（违反 R1.4）；`app.rs::ClientApp::new` 默认 `MovabilityMap::new()` 空映射、`examples/playable.rs` 未注入（B1 另一半）。`input.rs::InputState::apply` 已持有 `view: &WorldView`，群体 `Move` 分支调用 `filter_movable(&self.selection, movability)`。

约束：ADR 0002（`SNAPSHOT_VERSION` 当前 3、FNV-1a 状态哈希、30 Hz 整数帧、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键、稳定 `EntityId` 顺序）、ADR 0001（零资产）、`CODEBUDDY.md` §5（改 `w3-core` 必过 `tests/determinism.rs` + 反向验证改坏验红）、§8（四件套一次性成文、互不打架）、§9（R1 可移动性 / 参考矩阵）、`docs/references/WC3-MELEE-RULES.md` R1.4 / R13.6 / §14。相关 ADR：0001 / 0002 / 0003（范围，1.0 已修订）/ 0004 / 0005。

> 与既有规范的张力（如实登记、不冒充已解决）：主 `openspec/specs/unit-orders/spec.md`「群体移动只作用于可移动单位」与 `WC3-MELEE-RULES.md` R1.4 / R13.6 现有字面写有「MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`、`crates/w3-core/` 零改动」。这是 **P8.0 / P8.1 阶段的实现层禁令**：本 change 经用户 2026-11-16 拍板**首次放松该实现层约束**（改为只读视图投影 + 分层判据），ADR 0002 确定性契约一条不放松。活文档 `WC3-MELEE-RULES.md` 由并行 owner 按 §14 流程同步修订（不静默改规则），`docs/references/` / 主 specs / `p8-human-faction` 四件套正文均不在本 change 可写范围；本 change 只新增 ADDED Requirement、不改写既有 requirement 正文，字面一致性由后续 change / 归档复核承接。

## Goals / Non-Goals

**Goals:** 给 `EntityView` 加只读 `speed` 并投影已有 `UnitState.speed`；以分层判据（frame-0 装配映射 + 运行时视图 speed）把 R1.4 / R13.6 的 `Move` 过滤覆盖到运行时 `Train` / `Build` 动态实体；经合并函数兜底默认空映射；用反向验证证明缺陷态失败、正式实现通过；保持状态哈希 / 快照 / `SNAPSHOT_VERSION` / 排序键 / golden hash 逐字节不变。

**Non-Goals:** 不新增任何权威状态字段、命令变体、事件变体；不改 `CommandKind::Spawn` / `Builder::spawn` / `CommandQueue::submit` 校验；不重写 `filter_movable` 签名或既有 5 处 `tests/movability.rs` 断言；不改主 specs / 活文档 / `p8-human-faction` 四件套正文；不做 P8.5 深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）、联机底座 / 回放 / HUD / 键位接线；不碰 N1–N6（尤其 N5 JASS VM、N6 资产入库）；真窗口人工验收不在本轮执行。数值全为合成原创 `[待核]`、不入库。

## Decisions

> 玩法决策的规则依据与核实状态按 `openspec/config.yaml` 的 `rules.design` 标注：`[已核]` 规则性事实、`[待核]` 数值（不冻结进权威定义）、`[取舍]` 本项目实现决定。本 change 无新增玩法数值，规则依据为 R1.4 / R13.6 `[已核]` 的过滤契约与 `[取舍]` 的分层 / 只读投影实现。

### D1 首次动 `w3-core`：只读投影 `UnitState.speed`，不新增权威状态

**决策**：`sim_match.rs::EntityView` 加只读 `pub speed: i64`；`sim_match.rs::read_view` 投影**已存在**的 `UnitState.speed.raw()`。MUST NOT 新增权威状态字段，MUST NOT 改 `compute_state_hash` / `first_difference()` / `capture_snapshot` 的字段集合与布局 / `SNAPSHOT_VERSION`（仍 3）/ 命令 / 事件 / 排序键；`cargo test -p w3-core --test determinism` 的 golden hash MUST 逐字节不变。论证：`EntityView` 是 `read_view()` 派生的**只读视图**、不进哈希 / 快照；`UnitState.speed` 早已存在且已在哈希 / 快照中，本次仅把它投影到只读视图（R1.4 / R13.6 `[已核]` 契约、只读投影 `[取舍]`）。全部 `EntityView` 构造点补 `speed` 字段（编译期强制）。

**备选方案**：
1. 在 `complete_production` 里 push 携带 `DefId` / `speed` 的生产 `Event` → **否决**：改事件序列会动 `next_event_seq` 与逐帧哈希，golden hash 全变；且新增事件变体属权威行为，违反「零哈希冲击」。
2. 坚持内核零改动、纯表现层命令流 + diff 追踪 `EntityId` → **否决**：`Train` 命令带 `DefId` 但那是命令不是实体视图，运行时 `spawn` 出的 `EntityId` 与命令无稳定关联，联机 / 回放 / 重放下易错配；B1 正是此路不可判定。
3. `EntityView` 直接暴露 `DefId` → **否决**：需把 `DefId` 存进 `UnitState`（新增权威状态字段）或把装配层映射塞进内核，都要动哈希 / 快照 / 单一状态所有者边界。
4. 改 `CommandKind::Spawn` 校验放开 `speed = 0` → **否决**：改命令语义 / 线格式，破坏 P8.0 已定的两族 `Spawn` 字节一致性，且 frame-0 装配路径全面变动。
5. 新增 `Player.race` 或任一运行时权威状态字段承载可移动性 → **否决**：新增权威状态须同步进哈希 / 快照 / `first_difference()`，可能升 `SNAPSHOT_VERSION`（ADR 0002 §6-8）。

**理由**：这是把「已经权威、已经进哈希 / 快照」的值补投影到只读视图，是最小且唯一不触碰确定性契约的内核改动；用户 2026-11-16 拍板「只读暴露 speed（推荐）」正是此方案。

**ADR 0002 / 跨平台确定性影响**：零冲击。`read_view` 是帧提交后派生，修改 `EntityView` 不影响模拟；`UnitState.speed` 的哈希 / 快照编码一字不动；golden hash 逐字节比对是验收线（非仅声称）。Windows / Linux 无平台差异，但**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

**架构原则对照**：SRP（视图是只读派生，不承载权威状态）、唯一状态所有者（`UnitState` 仍是 speed 唯一所有者）、表现层只读边界（视图不反写）、OCP（加只读字段不改内核命令 / 类型语义）、DIP（表现层依赖只读视图而非权威可变结构）。

### D2 分层判据：frame-0 装配映射 vs 运行时视图 speed，两条来源不可互相替代

**决策**：统一为 speed 语义、但按**实体来源**分层（R1.4 / R13.6 `[已核]`、分层 `[取舍]`）：
- frame-0 装配实体 → 装配期 `UnitDef.speed_raw` 派生映射（`input.rs::derive_movability` / `MeleeSetup::movability`，P8.0 D1 不变、权威）；
- 运行时动态实体（不在装配映射里）→ 只读 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动。

**不可互相替代的核心论证（写透）**：`command.rs::CommandQueue::submit` 校验 `speed.raw() <= 0 → InvalidSpawn`，故 `setup.rs::Builder::spawn` 把 frame-0 全部实体（含建筑）`speed.max(1)`；**frame-0 建筑的运行时视图 `speed > 0`，用视图 speed 会把建筑误判为可移动、R1 bug 复发**。相反，运行时实体走 `sim_match.rs::complete_production` 的 `UnitState::new(..., SimFixed::from_raw(def.speed_raw), ...)`，建筑 `speed == 0`、单位 `speed > 0`，视图 speed 才是可靠判据；而装配期 `def_ids` 关联根本不为运行时新实体登记条目，映射对它们无判据。两个命名空间（frame-0 被抬过的 speed / 运行时真实 speed）语义不同，MUST NOT 用任一条来源替代另一条。

**备选方案**：
1. 全部改用视图 `speed` → **否决**：frame-0 建筑被 `max(1)` 抬过，误判可移动（R1.4 从 frame-0 路径复发）。
2. 全部沿用装配期映射、未登记实体保持 `unwrap_or(true)` → **否决**：即 B1 原缺陷，运行时建筑被放行。
3. 全部 `unwrap_or(false)` → **否决**：运行时新士兵被卡住、功能退化（R13.6 训练产出单位须能响应 `Move`）。
4. 放开 `Builder::spawn` 的 `max(1)` 让 frame-0 建筑也 `speed == 0` → **否决**：改 `CommandQueue::submit` 校验与 `Spawn` 命令语义 / 字节，动 `w3-core` 权威路径与两族 `Spawn` 一致性（P8.0 C2）。

**理由**：分层判据是两条来源各自在其可靠域内取用，既不改写 P8.0 已闭合的 frame-0 路径，又为运行时动态实体补上唯一可达的判据。

**ADR 0002 / 跨平台确定性影响**：判据全在表现层输入翻译层，命令生成集合按稳定 `EntityId` 顺序，不改 `CommandKind::Move` 线格式 / 排序键 / 哈希 / 快照；两条来源均为只读派生，跨平台生成同一命令序列。

**架构原则对照**：SRP（过滤只在输入翻译一处）、ISP（按来源暴露窄判据）、显式失败不静默降级（不再 `unwrap_or(true)` 全放行）、LSP（新实体对同一过滤函数行为一致）。

### D3 最小改动落点：新增合并函数，`filter_movable` 零改动

**决策**：`input.rs::filter_movable` **保持既有签名与对已登记实体的行为不变**；新增一个合并函数——把 `WorldView` 里「装配映射未登记」的实体按 `EntityView.speed != 0` 补进一张**有效映射**（frame-0 已登记项原样保留、运行时未登记项按视图 speed 补齐）——在 `input.rs::InputState::apply` 的群体 `Move` 分支调用。`apply` 已持有 `view: &WorldView`，MUST NOT 改其签名。既有 `tests/movability.rs` 里 5 处直接调 `filter_movable` / `degenerate_filter` 的反向验证断言零改动仍成立（R1.4 / R13.6 `[已核]`、合并函数 `[取舍]`）。

**备选方案**：
1. 给 `filter_movable` 加 `view` 参数、在函数内查视图 → **否决**：5 处直接调用 + `degenerate_filter` 反向验证断言全要改签名，破坏既有回归（P8.0 D3 真 A-B 证据）。
2. 直接把 `unwrap_or(true)` 改成查视图 → **否决**：`filter_movable` 无 `view`，且会改变对已登记实体 / 缺陷态的既有行为，令 `degenerate_filter` 证据失效。
3. 在选中 / 生产时把新实体写进 `self.movability` 持久化映射 → **否决**：引入需监听生产事件的表现层持久状态，与命令 / 实体错配、难以在回放 / 重放下保持一致。
4. 改 `ClientApp::apply` 签名传 `view` → **否决**：波及所有调用点，非最小改动。

**理由**：合并函数是纯函数、每次 `apply` 从当前 `view` 现算，无持久化、无错配；既有回归一字不动，`apply` 无需新参数。

**ADR 0002 / 跨平台确定性影响**：纯表现层派生、不进权威状态；`Move` 命令集合与落点按稳定 `EntityId` 顺序不变；哈希 / 快照 / 排序键零改动。

**架构原则对照**：OCP（新增合并函数而不改既有过滤函数）、CARP（组合既有 `filter_movable` / `MovabilityMap`）、LoD（输入层经只读视图取判据、不触内核）、SRP。

### D4 反向验证（改坏验红）

**决策**：新增回归 MUST 证明「去掉动态实体兜底、退回旧 `unwrap_or(true)` 默认放行」会让「运行时生产出的建筑不响应 `Move`」断言**失败**、正式实现**通过**；证据用测试内反例元断言或 A-B 对照，参照归档 P8.0 `tests/movability.rs::degenerate_filter` 先例。缺陷态可构造：空 / 退化映射下 frame-0 建筑因 `max(1)` 的 `speed > 0` 仍被误判可移动 → 同一断言在退化实现下失败。仅「全绿」MUST NOT 视为完成（`CODEBUDDY.md` §5 / `openspec/config.yaml` `rules.tasks` 第三条）。

**备选方案**：
1. 只写正向断言 → **否决**：缺陷实现下也可能全绿，不构成反向验证。
2. 人工改坏代码验红、不进测试 → **否决**：不可复现、不进 CI（P8.0 D3 备选 2）。
3. 声称「无法构造缺陷态」而跳过 → **否决**：`degenerate_filter` 已证缺陷态可构造，须真 A-B。

**理由**：B1 的本质是「默认放行」这一缺陷态，反向验证必须让该缺陷态在断言下显式失败。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图与命令、不触碰 `w3-core` 状态；同时包含重跑 `cargo test -p w3-core --test determinism` 任务以确认零影响。

**架构原则对照**：显式失败（缺陷态必红）、可复现证据进 CI、验收脚本独立于实现（SRP）。

### D5 playable / 默认空映射兜底（B1 另一半）

**决策**：合并函数兜底后，`view` 里所有实体（含默认态）都有 speed 判据、空映射不再等价「全部可移动」缺陷态。`examples/playable.rs`（场景 4 个可移动单位、无建筑）与 `app.rs::ClientApp::new` 默认空映射由 client-owner **评估并显式处置**：补一个明确注入或如实登记「兜底已覆盖」，**不得**假装处理而实际没做（R1.4 / R13.6 `[已核]`、兜底 `[取舍]`）。处置结论回报后写入 `decision-memo.md` 待确认项，不冒充已解决。

**备选方案**：
1. 只给 `playable` 注入映射、默认态不动 → **否决**：掩盖默认态缺陷，`ClientApp::new` 仍等价缺陷态。
2. 把默认空映射初值改成「全不可移动」 → **否决**：卡住默认态所有可移动单位、功能退化。
3. 不处置默认态、只修 `melee` 入口 → **否决**：B1 另一半未闭合（P8.0 / P8.1 均登记该半）。

**理由**：兜底使「默认态」与「缺陷态」解耦，注入变为可选优化而非正确性前提；显式登记避免「假装处理」。

**ADR 0002 / 跨平台确定性影响**：默认态是表现层初值、不进权威状态；兜底为纯派生，跨平台一致。

**架构原则对照**：显式失败不静默降级、唯一状态所有者（映射是表现层只读派生）、诚实登记已知限制。

### D6 范围、零资产与正交边界

**决策**：零资产 / 零文件 IO / 零 MPQ 派生数据 / 零真实导出 JSON；全部数值为合成原创 `[待核]`、不入库、不冒充原版（ADR 0001 / ADR 0004 / `WC3-MELEE-RULES.md` §14）。落在 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 种族做完）的过程分期；**N1–N6 不纳入**，尤其 N5（JASS / wtg VM）与 N6（资产入库）。P8.1 已归档、`p8-human-faction` 仍 active 且规格锁定，本 change 与它们**正交、不碰其四件套正文**；不把 P8.5 深度玩法塞进来（R1.4 / R13.6 `[已核]`、范围 `[取舍]`）。

**备选方案**：
1. 借本 change 改活文档 R1.4 / R13.6 或主 spec 字面 → **否决**：`docs/references/` / 主 specs 不在可写范围，由并行 owner 按活文档流程处理，本 change 不静默改规则。
2. 顺带把 P8.5 深度玩法 / HUD / 键位接线并入 → **否决**：扩范围、破坏正交与单 change 单 owner。
3. 为动态实体新增命令变体 / 权威状态字段 → **否决**：违反 ADR 0002 零冲击，且属 P8.5 / 后续 change。

**理由**：本 change 只闭合 B1 的「可移动性判定」，其余承接项各有独立 change，避免「顺手」扩范围。

**ADR 0002 / 跨平台确定性影响**：无新增权威状态 / 命令 / 事件 / 数值，`SNAPSHOT_VERSION` 仍 3；零资产不改变任何跨平台结论。

**架构原则对照**：SRP（单 change 单一关注点）、唯一状态所有者、正确性 / 确定性优先、诚实边界（不冒充已解决 / 不静默改规则）。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」（§「按任务类型的强制参考矩阵」T1 / T4）与 `openspec/config.yaml` 的 `rules.design`。本 change 命中 **T1**（只读视图投影 / 确定性内核零冲击）与 **T4**（运行时生产实体类型信息 / 单位行为）。报告由只读 `reference-researcher` 出、`spec-steward` 成文前**逐锚打开复核**（本轮已实际打开 `CUnit.java` / `CUnitData.java` / `CSimulation.java` / `RenderUnit.java` / `command_exchange.rs` / `FRAMEWORK.md` 对应内容）。四个参考项目只读、不复制代码（ADR 0001）；固定提交见 `UPSTREAM.md` §「固定参考版本」，本机绝对路径不在本文重复。
>
> **锚写法区分（如实声明）**：本项目本轮会被 owner 改动的代码锚按 `proposal.md` / 本文件 Context 的约定用**符号名**；参考项目为只读固定提交、行号不漂移，其锚沿用对照报告给出的 `file:line` 原样（可核验、且是「逐锚引用」要求），不构成 §「避免行号漂移返工」的对象。

- **T4 运行时实体类型 / 可移动性语义 → `WarsmashModEngine`（AGPLv3，仅借鉴语义、禁止复制代码 / 不引入 Java / LibGDX）**。路径：`core/src/com/etheller/warsmash/viewer5/handlers/w3x/simulation/CUnit.java`、`simulation/data/CUnitData.java`、`simulation/CSimulation.java`、`rendersim/RenderUnit.java`。锚：`CUnit.java:192`（`CUnitType unitType` 字段）、`:296-298`（构造注入 `War3ID typeId` + `CUnitType`）、`:314` / `:319`（`structure = unitType.isBuilding()`）、`:1729`（`getTypeId()`）；`CUnitData.java:245-260`（`create` 按 `typeId` 取定义、把 `speed` / `typeId` / `unitType` 注入 `new CUnit`）、`:294`（`if (speed > 0)` 才挂 `CAbilityMove`）、`:394-399`（`speed <= 0` 移除移动能力）；`CSimulation.java:292-298`（`internalCreateUnit(typeId, …)` 运行时调用）；`RenderUnit.java:47` / `:152` / `:211`（表现层持 `simulationUnit` 引用、读 `isBuilding()` / `getSpeed()`）。**借鉴**：运行时实体**直接持有 unit type 与 rawcode**（非装配期映射）、以 `speed == 0` 判不可移动的语义，与本次「视图 speed 作动态实体判据」同构——支撑 D2。**不借鉴 / 边界**：其表现层**直接引用权威可变 `CUnit`**，与本项目 ADR 0002 §5「表现层只读 `WorldView`、不得持有 / 反写权威状态」相反，本项目只借「类型 / 可移动性随实体走 + `speed == 0`」语义、只读视图投影由 D1 原创承担；AGPLv3 仅借鉴设计语义、**不得复制任何代码**，不引入 Java / LibGDX 依赖，引入代码须按 ADR 0001 复核点重新评估许可证。许可证理由：`UPSTREAM.md` §「各自提供什么」T4 行 + ADR 0001。

- **T1 只读投影范式 → `vera20k`（GPLv3 + Section 7 附加条款，仅借范式）**。路径：`src/net/command_exchange.rs`。锚：`command_exchange.rs:154-162`（`RetainedSendProgress` 注释 `"Read-only projection of the retained transaction"`，`Copy`、投影已有字段、不逃逸独立状态）。**借鉴**：**只读投影范式**——投影已有字段、不新增权威状态、不进哈希 / 快照，支撑 D1「`EntityView` 是只读派生、不进哈希 / 快照」的论证。**不借鉴 / 边界**：它是网络命令交换进度视图，**非实体视图**；GPLv3 + Section 7 附加条款**只有真的复制代码时**才进入（ADR 0001 / `UPSTREAM.md` T1 行），本 change 不复制。许可证理由：`UPSTREAM.md` §「固定参考版本」+ T1 行。

- **T2 只读分层视图范式（仅一句，不展开）→ `rts_kernel_cs`（MIT）**。路径：`docs/FRAMEWORK.md`。锚：`FRAMEWORK.md:81`（只读 `ById` 视图 / detached `ReadAll`）。**借鉴**：只读分层范式一句，支撑「表现层只读视图」边界。**不借鉴 / 边界**：其 `double` 数值路线与「不承诺跨平台确定性」**不采用**（ADR 0002 / `UPSTREAM.md` T2 行）；本 change 不展开 T2、不复制 C# 实现。许可证理由：MIT、`UPSTREAM.md` T2 行。

- **`godot_warcraft3`（MIT）**：本次未命中矩阵条目，按 `UPSTREAM.md` §「触发时机」不展开、不臆造锚。

- **参考缺口（如实登记、不臆造）**：四个参考项目均**无**「只读实体视图投影已有 `speed` 作可移动性判据」的精确对应物。分层判据（frame-0 `speed_raw` 映射 + 动态实体视图 `speed == 0`）、`CommandKind::Spawn` 不含 `DefId` 用装配层 `def_ids` 关联、把已进哈希 / 快照的 `UnitState.speed` 投影到只读 `EntityView` 且保持 golden hash 逐字节不变——均属本项目在 ADR 0002 / ADR 0001 零资产约束下的**原创取舍**（D1 / D2 / D3），不冒充参考结论。该缺口已随用户 2026-11-16 方案拍板放行（见 `decision-memo.md` 规格门），不再回到人拍板、不臆造「参考项目已验证」。

- **许可证边界（归因纠正）**：「新增权威状态字段须同步进哈希 / 快照 / `first_difference()`」硬门禁的出处是**本项目** ADR 0002 §6-8 + `CODEBUDDY.md` §5 + 主 `melee-match/spec.md`，**不是任何参考项目**；D1 否决备选 1 / 3 / 5 的依据指向本项目权威文件，不得冒充参考结论。四项目均不复制代码入库、不引入 AGPLv3 / Java / LibGDX 依赖。

## Risks / Trade-offs

- **golden hash 不变需实证、不能只声称** → D1 加 `EntityView.speed` 字段与 `read_view` 投影虽不改哈希编码，但 `w3-core` 首次被改动，`CODEBUDDY.md` §5 要求过 `tests/determinism.rs`。缓解：tasks 含「重跑 `cargo test -p w3-core --test determinism` + golden hash 逐字节比对 + `git diff -- crates/w3-core/` 范围核验」，以命令输出为证据；不通过即视为未交付。
- **`EntityView` 加字段波及所有构造点** → 构造点分布 `sim_match.rs::read_view`、`hud.rs`、`tests/interaction.rs`、`tests/movability.rs`、`w3-render/src/model.rs`、`w3-render/src/scene.rs`，漏补即编译失败；补字段时若误把 `speed` 用作权威状态会破坏只读边界。缓解：tasks 列「全部构造点补字段」并逐个验证，编译期强制；只读视图「修改不影响模拟」语义不变。
- **分层判据一致性** → frame-0 与运行时两个 speed 命名空间若混用，会从 frame-0 或动态实体任一侧复发 R1 bug。缓解：D2 写透 `max(1)` vs `speed == 0` 不可互替；两个 spec delta 的 Scenario 分别覆盖 frame-0 回归不退化与运行时建筑 / 单位判定；合并函数只对「装配映射未登记」实体补判据、已登记项原样保留。
- **frame-0 与运行时命名空间隔离** → `CommandKind::Spawn` 不含 `DefId`、装配层 `def_ids` 关联不为运行时新实体登记，二者 MUST NOT 互相回查。缓解：D2 明确「frame-0 以装配期映射为唯一权威、运行时以视图 speed 为唯一判据」；合并函数以「映射是否登记」为分界，不引入跨命名空间推断。
- **P8.0 B1 未擅自扩大到新权威状态** → 本 change 只放松「实现层零改动 / 不扩展视图」约束，未新增权威状态 / 命令 / 事件 / 排序键 / `SNAPSHOT_VERSION` 变更；不得被后续 change 借「本 change 动了 `w3-core`」旁路 ADR 0002 门禁。缓解：proposal Impact / D1 / 两个 spec 的「状态哈希 / 快照 / `SNAPSHOT_VERSION` 不变」Scenario 与 `critic` 代码门复核。
- **既有规范字面张力的承接** → 主 `unit-orders/spec.md` / `WC3-MELEE-RULES.md` R1.4 / R13.6 / `p8-human-faction` 既有字面禁令与本 change 放松后的实现层约束不一致；本 change 不改写它们、不冒充已解决。缓解：如实登记于 Context / D6 / `decision-memo.md` 待确认项，活文档由并行 owner 按 §14 修订，主 spec / active change 四件套正文由后续 change / 归档复核承接，本 change 只新增 ADDED Requirement。
- **反向验证必须真 A-B** → 只加正向「全绿」断言不构成 `CODEBUDDY.md` §5 验证。缓解：D4 / 两个 spec 的「改坏验红」Scenario / tasks 明确退化实现令断言失败、正式实现通过，证据经真实 `InputState::apply` 群体 `Move` 路径触发。
- **`file:line` 锚漂移** → 本轮 `w3-core` / `w3-client` / `w3-render` 会被 owner 改动；本 change 四件套与决策纪对本项目代码一律用「路径 + 符号名」、对未改动文档用章节号 / 规则编号，规避 P8.1 的行号漂移返工。参考项目为只读固定提交，其 `file:line` 锚不漂移（见「参考项目对照」开头声明）。
- **范围外承接** → P8.5 深度玩法 / 野怪 / 物品 / 迷雾 / HUD / 键位接线、真窗口人工验收、`critic` 代码门结论、playable 默认态处置结论均由后续独立 change / owner 承接，本 change 不因「顺手」扩范围、不冒充已执行。
