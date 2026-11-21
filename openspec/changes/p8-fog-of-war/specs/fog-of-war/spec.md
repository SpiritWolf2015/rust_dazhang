# Spec Delta

## Purpose

定义 P8.5e 战争迷雾的表现层只读派生契约：三态迷雾网格、已探索层单调累积、当前可见每帧重算、视野与昼夜相位数据驱动、小地图单位点与敌方变化可见性过滤，以及 P8.5c / P8.5d 野怪 / 商店 / 地面物品可见性承接；全部派生 MUST NOT 进入权威状态 / FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 7）定义的快照 / `first_difference()`，守住 ADR 0002 §5 与 `WC3-MELEE-RULES.md` R10.5 的 1.0 范围取舍（§0.3 T3）。

## ADDED Requirements

### Requirement: 三态迷雾网格与已探索层单调累积

系统 SHALL 在表现层为每个本地视角维护一张三态迷雾网格：未探索 MASKED、已探索 FOGGED、当前可见 VISIBLE（R10.1）。网格初始全为 MASKED；当前可见格 SHALL 同时把该格记为已探索；离开视野后 VISIBLE 降级为 FOGGED、FOGGED 单调保留、MUST NOT 回退为 MASKED（R10.1）。该网格是**表现层本地只读派生状态**，MUST NOT 进权威状态 / 状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5）。

#### Scenario: 初始不可见与探索后地形保留（R10.1 / R10.5）

- **WHEN** 一局开局、本地视角尚未让任何单位进入某区域，随后单位进入该区域再离开
- **THEN** 该区域开局为 MASKED 不呈现地形与敌方单位，进入时升为 VISIBLE 呈现地形与实体，离开后降为 FOGGED 且地形底图仍保留、敌方单位 / 建筑变化不再呈现（R10.1）；三态与转移全在表现层网格、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节不变（R10.5 / ADR 0002 §5）

#### Scenario: 已探索层单调不回退（R10.1）

- **WHEN** 同一格反复进出视野、并推进若干逻辑帧与多次重绘
- **THEN** FOGGED 状态单调累积、MUST NOT 因离开视野 / 重绘 / 相机移动 / 插值帧丢失而回退为 MASKED（R10.1）；「表现层假迷雾、无已探索层（每帧只绘制圆形黑幕、不累积探索）」的退化实现令该断言失败（Q1 被否备选② / R10.1）

#### Scenario: 三态为表现层本地状态（R10.5 / ADR 0002 §5）

- **WHEN** 校验迷雾网格的存储位置与写入路径
- **THEN** 网格只驻留表现层本地（随视角一份、不回写 `RtsMatch`）、MUST NOT 出现在 `Snapshot` 任何字段 / `compute_state_hash()` 输入 / `first_difference()` 差异字段集合（R10.5 / ADR 0002 §5 / `snapshot.rs:102-146`）；对照 `vera20k@305ea3a75 src/sim/world/world_hash.rs:4,687-688` 把 fog 进哈希的做法，本项目明确不借鉴（R10.5）

### Requirement: 当前可见每帧重算

当前可见集合 SHALL 每逻辑帧重算一次：先把上一帧全部 VISIBLE 批量降为 FOGGED、再按本帧己方存活单位位置与视野半径做圆形揭示（R10.1）。重算 MUST 只读取只读 `WorldView`（`tick` / `entities` 的 `x` / `y` / `owner` / `alive`）、事实事件拷贝与只读 `Snapshot`，MUST NOT 反写权威状态（R10.5 / ADR 0002 §5）。

#### Scenario: 每帧先降级再揭示（R10.1）

- **WHEN** 单位在帧 N 位于 A 区、帧 N+1 移动到 B 区
- **THEN** 帧 N+1 先把 A 区 VISIBLE 降为 FOGGED（已探索层保留）、再揭示 B 区为 VISIBLE，A 区敌方单位 / 建筑变化不再可见（R10.1）；降级与揭示确定性执行、两实例同脚本逐帧 `compute_state_hash()` 相等（R12.4 / ADR 0002）

#### Scenario: 重算只读派生不改哈希（R10.1 / R10.5 / ADR 0002 §5）

- **WHEN** 连续推进若干帧并反复重算当前可见集合
- **THEN** 重算只经 `read_view()` / `drain_events()` 事件拷贝 / 只读 `Snapshot` 取得输入、无 `RtsMatch` 字段写入路径，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与无迷雾实例一致（R10.5 / ADR 0002 §5 / `sim_match.rs:267,3448-3477`）；墙钟 / 渲染帧率 / `HashMap` 迭代顺序 / 表现层独立随机源的退化实现令确定性断言失败（ADR 0002 §2-3）

### Requirement: 视野范围、昼夜相位与飞行视野数据驱动

视野半径 SHALL 按表现层合成原创编译期常量 `[待核]` 数据驱动（昼 / 夜两值），昼夜相位 SHALL 从 `WorldView.tick` 按整数帧纯派生（`phase = tick % (day_frames + night_frames)`），MUST NOT 新增权威状态或墙钟读取（R10.1 / R10.2 / ADR 0002）。飞行 / 空中单位视野不受地形限制（R5.6）。视野半径 / 昼夜时长 / 格子分辨率常量 MUST NOT 进 `w3-core::FrozenDefinitions` / `UnitDef` / `compute_defs_hash`（R10.5 / ADR 0001 / §14；Q6 推荐）。

#### Scenario: 昼夜两值常量与整数帧相位（R10.1 / R10.2 / ADR 0002）

- **WHEN** 以不同 `tick` 构造派生并切换昼 / 夜相位、改动任一表现层常量
- **THEN** 相位只由 `tick` 与冻结整数帧常量得出、昼 / 夜揭示半径按合成 `[待核]` 常量切换、无墙钟 / `SystemTime` / `Instant` / `f64` 进派生、`compute_state_hash()` / `capture_snapshot()` / `content_hash()` 逐字节不变（R10.1 / R10.2 / ADR 0002 §2-3 / ADR 0004）；「把视野常量塞进 `FrozenDefinitions` 却声称 golden hash 不变」的退化实现被断言拦截（Q6 被否备选③）

#### Scenario: 飞行视野与零资产冻结常量（R5.6 / R10.1 / ADR 0001）

- **WHEN** 在含地形阻挡的区域派生飞行 / 地面单位视野，并校验常量来源
- **THEN** 飞行单位视野不受地形阻挡限制（R5.6），全部半径 / 时长 / 格子分辨率 / 不透明度为表现层合成原创编译期整数常量 `[待核]`、不含 MPQ 派生 / 真实导出 JSON / Warsmash `GRID_STEP=128` / `PATHING_RATIO=4` / `DYING_UNIT_VISION_RADIUS` / godot 网页值（R10.1 / ADR 0001 / ADR 0004 / §14 / 规则 `:373-384`）；是否与本机导出不符一律标 `[待核]` 并回到人拍板、不静默改规则（§14）

### Requirement: 小地图单位点可见性过滤

`derive_minimap` SHALL 在把单位点喂入 `MinimapModel.points` **之前**完成迷雾可见性过滤，把「在场性」（`EntityView.alive`）与「可见性」（迷雾派生）分两层（R10.1 / R10.5，参考对照结论 R2）。过滤后的小地图单位点 MUST NOT 泄露 MASKED / FOGGED 格内的敌方单位；点击小地图仍只平移表现层相机、MUST NOT 生成权威命令（R10.1 / `rts-command-hud/spec.md:43-55`）。

#### Scenario: 单位点喂入前按格状态过滤（R10.1 / R10.5）

- **WHEN** 一局中同时存在 VISIBLE / FOGGED / MASKED 格内的己方与敌方存活单位并刷新小地图
- **THEN** 只有 VISIBLE 格的敌方单位点进入 `MinimapModel.points`、MASKED / FOGGED 格敌方单位点不呈现，己方单位按本地视角呈现，底图 / 视口框独立于迷雾遮罩（R10.1 / R10.5 / `hud.rs:653-724,791-808`）；「不做可见性过滤、直接把 `view.entities.iter().filter(alive)` 全量喂入」的退化实现令该断言失败（Q9 形态①）

#### Scenario: 过滤只读不改哈希与相机平移不生成命令（R10.1 / R10.5 / ADR 0002 §5）

- **WHEN** 连续刷新小地图、点击小地图平移相机并与无迷雾过滤实例同脚本比对
- **THEN** 过滤是纯只读派生、点击只平移相机不生成 `Move` / 任何权威命令，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致（R10.1 / R10.5 / ADR 0002 §5 / `rts-command-hud/spec.md:45,52-55`）；含建筑 / 资源点的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4）

### Requirement: 敌方单位与建筑变化不可见

敌方单位 / 建筑在 MASKED / FOGGED 格内的变化 SHALL 对本地视角不可见（R10.1）。`Scene` 动态实例与 HUD 只读投影在喂入前 MUST 按格状态过滤或冻结上一已知态，MUST NOT 把敌方位置 / 新建 / 死亡 / 外观变化呈现给不可见格（R10.1 / R10.5）。可见性同时约束渲染与可选目标集合的表现层呈现，但不改变 `w3-core` 权威命令面（R10.5 / R13.6）。

#### Scenario: 敌方变化不呈现、离开视野冻结上一已知态（R10.1）

- **WHEN** 敌方单位在 FOGGED 格内移动 / 新建建筑 / 阵亡，随后该格重新进入 VISIBLE
- **THEN** FOGGED 期间变化不呈现（可冻结上一已知态），重新可见后按本帧只读 `WorldView` 重建最新状态（R10.1）；重建只读取只读视图、无权威状态写入（R10.5 / ADR 0002 §5）

#### Scenario: 渲染只读派生不反写（R10.1 / R10.5 / ADR 0002 §5）

- **WHEN** 连续重绘含迷雾过滤的 `Scene` 动态实例并与无迷雾实例同脚本比对
- **THEN** `build_vertices` / 可见性过滤只读取表现层派生结果、稳定顺序、无随机源，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、`crates/w3-render/tests/isolation.rs` 隔离断言全绿（R10.1 / R10.5 / ADR 0002 §5 / `scene.rs:904-933`）；缺帧 / 模型缺失 MUST NOT 改变模拟推进（`3d-rendering/spec.md:10`）

### Requirement: P8.5c / P8.5d 野怪、商店与地面物品可见性承接

系统 SHALL 承接 P8.5c 明文交接（`creep-camps/spec.md:136,143-146,174`）与 P8.5d 明文交接（`item-shop-town-portal/spec.md:195-198`）：野怪 / 商店 / 地面物品在迷雾下的可见性 MUST 按「当前可见格」过滤后再喂 `MinimapModel.points` 与 `Scene`（R10.1 / R10.5 / R8.1 / R9.1）。地面物品不是 `EntityStore` 实体、不出现在 `read_view()` 实体集合（`creep.rs:34-38`），其位置 / 类别 SHALL 从只读 `Snapshot.creep_camps` / `Snapshot.item_shop`（`snapshot.rs:139,145`）查表取得，MUST NOT 只靠 `WorldView.entities`（R10.5 / ADR 0002 §5）。

#### Scenario: 野怪 / 商店 / 地面物品按可见格承接（R10.1 / R10.5 / R8.1 / R9.1）

- **WHEN** 本地视角含 VISIBLE / FOGGED / MASKED 格内的野怪营地、中立商店与掉落地面物品并刷新 HUD 与小地图
- **THEN** 只有 VISIBLE 格的野怪 / 商店 / 地面物品呈现，其余格不呈现；位置 / 类别经只读 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（复用 `sim_match.rs:233-243` 既有只读投影查表范式与 `hud.rs:737-751` 四投影子模型同构），过滤全在 `HudModel` 新增迷雾只读投影子模型内、不反写 `RtsMatch`（R10.1 / R10.5 / ADR 0002 §5）

#### Scenario: 只读派生不进哈希快照且交接措辞收敛（R10.5 / R10.1 / ADR 0002 §5）

- **WHEN** 校验本承接对权威状态与 P8.5c / P8.5d 交接措辞的影响
- **THEN** 迷雾下野怪 / 商店 / 地面物品可见性由 `fog-of-war` capability 承接、MUST NOT 再登记为「范围外承接 / 留后续」，逐帧 `compute_state_hash()` / `capture_snapshot()` / `first_difference()` 不含任何迷雾 / 可见性字段（R10.1 / R10.5 / ADR 0002 §5）；地面物品 / 商店（中立建筑）/ 金矿 / 树木 MUST NOT 出现在 `Move` 命令生成集合（R1.1 / R1.4 / R9.1 / R13.6）

### Requirement: 迷雾 MUST NOT 进权威状态、状态哈希、快照与差异诊断

迷雾网格 / 当前可见 / 已探索层 / 雾遮罩这组表现层本地状态 SHALL 全部为只读派生，MUST NOT 进入 `w3-core` 权威状态、FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 7）定义的快照与 `first_difference()` 差异字段集合（R10.5 / ADR 0002 §5 / §6-8）。`w3-core` SHALL 保持零权威状态改动（理想 `git diff -- crates/w3-core/` 为空）；MUST NOT 给 `first_difference()` 加 fog 字段（R10.5 / ADR 0002 §5）。

#### Scenario: 隔离回归逐字节一致（R10.5 / ADR 0002 §5 / R12.4）

- **WHEN** 以同一命令脚本分别驱动「含迷雾派生」与「无迷雾派生」两实例，交错逐帧推进并比对
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`、`read_view()` / `drain_events()` 输出一致、`crates/w3-render/tests/isolation.rs` 全绿（R10.5 / R12.4 / ADR 0002 §5-8）；「把迷雾网格塞进 `Snapshot` 或 `compute_state_hash()`」的退化实现真经 `capture_snapshot()` / `compute_state_hash()` 生产路径注入后令该断言失败（Q9 形态①）

#### Scenario: w3-core 零权威状态改动（R10.5 / ADR 0002 §5）

- **WHEN** 校验本 change 对 `crates/w3-core/` 的全部改动与 `Snapshot` 结构
- **THEN** `git diff -- crates/w3-core/` 为空属预期、`Snapshot`（`snapshot.rs:102-146`）顶层字段集合不变、无 fog / vision / visibility / explored 字段、FNV-1a 哈希输入集合不变、`first_difference()` 字段集合不变（R10.5 / ADR 0002 §5 / §6-8）；若 Q1 拍板「改内核新增权威状态」则 MUST 按 `CODEBUDDY.md §8` 同步修订四件套并重走规格门（Q1 被否备选①）

#### Scenario: 对照差异不抹平（R10.5 / ADR 0002 §5）

- **WHEN** 复核参考对照报告「fog 进哈希 / 进模拟层」的结论
- **THEN** `vera20k@305ea3a75 src/sim/world/world_hash.rs:589-591,1488-1497` 与 `WarsmashModEngine/.../CPlayerFogOfWar.java:11-35,201-225` 的做法登记为**对照差异、明确不借鉴**（本项目用户已拍板表现层只读派生 R10.5 / §0.3 T3 / `0003-scope.md:90`），MUST NOT 借「参考项目如此」把 fog 带进权威状态（R10.5 / `design.md`「对照差异」）

### Requirement: 版本保持 7 不 bump、golden hash 不变、determinism 不因迷雾改动

`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` SHALL 全部保持 **7** 不 bump；`Snapshot` 结构（含 `item_shop`）、FNV-1a 哈希字段集合、`first_difference()` 字段集合 MUST NOT 改变；golden hash `0x940cb016f6630f1e`（整份 sha256 `d6c0c944…aaa41`）SHALL 逐字节不变；`cargo test -p w3-core --test determinism`（26 项）SHALL 全绿且不因迷雾改动（R10.5 / R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

#### Scenario: 版本号硬断言与 golden hash 不变（R10.5 / R12.4 / ADR 0002）

- **WHEN** 复核 `snapshot.rs:43-44` / `packet.rs:41` 常量、单测硬断言、golden hash 基线与 proto 线格式
- **THEN** `SNAPSHOT_VERSION == SIM_VERSION == PROTOCOL_VERSION == 7` 且各处一致、golden hash 仍为 `0x940cb016f6630f1e`、proto 最大 tag 45 与 `reserved 14 to 32` 不变、不新增命令 / 事件变体、不触 ADR 0005 线格式（R10.5 / R12.4 / ADR 0002 §6-8 / ADR 0005 §3）；MUST NOT 沿用 P8.5a–d「bump 8 / proto tag 46 起 / golden hash 重取」口径（Q4 被否备选）

#### Scenario: determinism 26 项不因迷雾改动（R12.4 / ADR 0002 §8 / CODEBUDDY.md §5）

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项）并以「迷雾派生反写权威状态 / 迷雾字段进哈希 / 快照」退化实现替换正式实现
- **THEN** 正式态 26 项全绿、golden hash 逐字节不变；退化实现令隔离回归断言失败、修复后通过（测试内反例 + 元断言防退化，Q9 形态①）（R10.5 / R12.4 / ADR 0002 §8 / `CODEBUDDY.md §5`）；MUST NOT 冒充「缺陷态红 / 修复态绿两条命令输出留档」的形态②

#### Scenario: 反向验证形态①与跨平台不冒称（R12.4 / Q9 / Q12）

- **WHEN** 复核隔离回归的反向验证形态与跨平台确定性口径
- **THEN** 证据为 `CODEBUDDY.md §5` 形态①（测试内构造反例 + 元断言防退化、退化实现真经生产路径注入）、不冒充形态②；仅沿用 ADR 0002 复核记录「Windows / Debian 12 x86_64（v3）已验证」结论，v4 / v5 / v6 / v7 的 Debian 侧逐字节比对 / macOS / ARM64 / 其它 rustc 仍如实标注待验证、不声称全平台达成或有新验证（R12.4 / ADR 0002 / Q12）

### Requirement: 只读派生联机两端与回放一致

迷雾派生 SHALL 是纯函数 `f(同一权威状态序列, 本地视角, 冻结常量)`：联机两端各自本地派生、逐帧 checksum 比的是**不含迷雾的权威状态哈希**、MUST NOT 传输迷雾状态 / 游戏状态（R12.2 / R12.4 / ADR 0005 §4）。回放 SHALL 从录像（`defs_hash` / `content_hash` / 命令序列 / 帧号）逐帧重建同一权威状态序列 + 同一冻结常量 + 同一视角得到同一迷雾，迷雾不进快照、重放按帧重新累积（R12.4）。

#### Scenario: 两端各自派生且只传命令（R12.2 / R12.4 / ADR 0005 §4）

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一命令脚本，各自本地派生迷雾
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致、`first_difference()` 为 `None`，线格式只传玩家输入命令、不含任何 fog / visibility 字段（R12.2 / R12.4 / ADR 0002 §2-3 / ADR 0005 §4）；各视角的已探索层本就不同、不是 desync（`vera20k@305ea3a75 docs/Networking.md:490` 仅借「各视角各一份」语义）

#### Scenario: 回放逐帧重建同一迷雾（R12.4 / ADR 0002 §7）

- **WHEN** 把一局存为录像、销毁实例、加载回放并从帧 0 逐帧重放，与原局同视角迷雾比对
- **THEN** 回放逐帧重建同一权威状态序列、同一冻结常量、同一视角得到同一三态网格与已探索层，迷雾不进快照、重放按帧重新累积、哈希与原局一致（R12.4 / ADR 0002 §7）；重连 / 中途加入 / 变速 / 多视角观战（R12.3 / R12.5 `[待核]`）的「从帧 0 重放派生 vs 从当前帧起算降级」契约按 Q7 拍板结论落地、不臆造

### Requirement: 只读视图扩展边界与内核零 diff

本期 SHALL 不扩展 `WorldView` / `EntityView` 只读投影字段、不新增 `sight_radius` / `def_id` / `is_flying` 等当前不存在的字段，走装配层 `entity_defs`（`hud.rs:764-766`）+ 表现层合成常量（R10.5 / R13.6 / ADR 0002 §5；Q5 推荐）。`EntityView` 无 `def_id` 是既有已知限制（`runtime-entity-model-binding/spec.md:12,37`），MUST NOT 借本 change 把它偷偷塞进权威状态（R13.6 `:314`）。

#### Scenario: 不扩展视图且内核零 diff（R10.5 / R13.6 / ADR 0002 §5）

- **WHEN** 校验迷雾派生对 `WorldView` / `EntityView` / `w3-core` 结构的全部改动
- **THEN** `WorldView` 字段仍仅 `tick` / `rng` / `entities`、`EntityView` 九字段不变、`git diff -- crates/w3-core/` 为空属预期，派生所需位置 / 归属 / 存活 / 帧号全在既有只读视图内且已受现有状态哈希保护（R10.5 / R13.6 / ADR 0002 §5 / `sim_match.rs:253-273,3479-3489`）；对照 P8.4 `runtime-entity-movability` 归档 `design.md:25-40` D1「只读暴露 speed」先例，投影已有权威字段仍须拍板、投影不存在的字段属新增权威状态，MUST NOT 静默采用（Q5）

#### Scenario: 扩展视图或进内核触发重走规格门（R10.5 / ADR 0002 §5 / Q1）

- **WHEN** Q1 / Q5 拍板结果与推荐不同（改内核新增权威状态 / 扩展 `WorldView` / `EntityView` 新增字段）
- **THEN** MUST 按 `CODEBUDDY.md §8` 同步回查四件套并由 `spec-steward` 修订后重走规格门，`SNAPSHOT_VERSION` / golden hash / determinism 基线按 ADR 0002 全套处置、不静默沿用本期「保持 7 / 零 diff / 逐字节不变」口径（R10.5 / R12.4 / ADR 0002 §6-8 / Q1 / Q4）

### Requirement: 范围边界——隐身、昼夜、暗夜特色按 Q2 拍板收敛

本期最小闭环 SHALL 显式登记为视野 / 已探索层 / 小地图单位点 / 敌方变化不可见 / 只读派生（R10.1 / R10.5 / 路线图 `:129`）。隐身反隐（R10.4）、昼夜循环全量（R10.2）、R10.3 回血、R5.6 视野阻挡、S3 暗夜夜间隐身特色、R8.4 野怪睡眠是否纳入本期 SHALL 按规格门 **Q2** 由用户拍板、MUST NOT 预设结论、MUST NOT 静默窄化 `WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）字面「隐身 / 昼夜 / 暗夜特色随迷雾纳入一并做完」的已拍板 1.0 范围（§14）。

#### Scenario: 1.0 范围张力如实交用户拍板（R10.2 / R10.3 / R10.4 / §0 S3 / §14）

- **WHEN** 校验本期范围与 1.0 已拍板范围的张力
- **THEN** `WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）字面「一并做完」与 `P8-human-orc-melee.md:129` 本期验收线「最小闭环」的张力在 `design.md` Risks 与 `decision-memo.md` Q2 显式呈现、把处置交用户拍板；`creep_camps.rs:911-919` 明确「R8.4 睡眠行为本期不实现」如实引用、不冒充已解决（R10.4 / R8.4 / §14 / Q2）

#### Scenario: 触及权威状态的范围项走 ADR 0002 全套（R10.3 / R10.4 / R5.6 / §0.3 T3）

- **WHEN** Q2 拍板把隐身「不可被选为目标」/ R10.3 回血 / 视野阻挡 / 昼夜全量纳入本期，而这些项须新增权威状态
- **THEN** MUST 按 `P8-human-orc-melee.md:131` 走 ADR 0002 全流程（状态哈希 + 快照 + `first_difference()` + determinism 26 项 + bump + 重取 golden hash）、与本期推荐方向冲突、须先拍板改四件套并重走规格门；若仅是「不绘制 / 不显示小地图点」的表现层可见性过滤，则为表现层只读派生、不触及权威状态（R10.4 / R10.5 / ADR 0002 §5 / Q2）

### Requirement: HUD 只读派生隔离、零资产、许可证与真窗口人工清单

迷雾 HUD 投影 / 小地图 / 雾遮罩 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 `hud.rs:152,236,354,539` 的 `derive(snapshot, events)` 范式），MUST NOT 反写 `RtsMatch` 权威状态 / 进状态哈希 / 快照 / `first_difference()`（R10.5 / R13.6 / ADR 0002 §5）。UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。全部常量为合成原创编译期整数 `[待核]`、仓库零资产、许可证边界与真窗口人工清单保持未勾（ADR 0001 / ADR 0004 / §14）。

#### Scenario: HUD 只读不改哈希（R10.5 / R13.6 / ADR 0002 §5）

- **WHEN** 连续驱动迷雾 HUD 投影 / 小地图 / 雾遮罩重绘并与无 HUD 实例同脚本比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、HUD 无 `RtsMatch` 字段写入路径、`crates/w3-render/tests/isolation.rs` 隔离断言全绿、`w3-core` 零改动（R10.5 / R13.6 / R12.4 / ADR 0002 §5 / `hud.rs:1-14,912-925`）；键位 / 命令卡状态为表现层只读派生、不进权威状态（R13.6 `:315`）

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14 / R10.1）

- **WHEN** 校验本 change 的全部改动与新增常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON / PNG / GLB，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine`（AGPLv3）仅借鉴语义无 Java / LibGDX 代码复制、`vera20k`（GPLv3 + Section 7）`git show 305ea3a75:` 只读零复制、godot / `rts_kernel_cs`（MIT）借结构不借实现且 `double` 路线不采用，无新增未登记依赖、不改 `NOTICE.md` / `Cargo.lock`（ADR 0001 / ADR 0004 / R10.1 / `design.md`「许可证核对」）

#### Scenario: 真窗口人工清单保持未勾与范围外承接（R10.1 / R10.5 / §14 / Q11）

- **WHEN** 校验真窗口人工验收清单与范围外承接登记
- **THEN** `tasks.md` 第 8 组逐项保持 `- [ ]`「未执行 / 待人工」、先离屏 `render_headless_rgba` + `ViewImage` 自证与 CPU 回归全绿再请人工、不提供替代、不用离屏截图 / CPU 回归冒充真窗口验收、CI MUST NOT 声称已验证 GPU、清单未逐项勾选前不得归档（R10.1 / R10.5 / Q11 / `playable-window-entry/spec.md:149-176`）；范围外项（Q2 / Q3 拍板后仍留后续者）显式登记为后续 change / P9+ 四族承接待办、不冒充已解决（§14 / Q3）
