# creep-camps Specification

## Purpose
定义 P8.5c 野怪营地与掉落的确定性权威契约：野怪 / 营地 / 中立归属与 leash / 归巢状态、击杀经验与金币收益、掉落表冻结常量、确定性随机码消费序列、地面物品 / 物品实例全部作为 `w3-core` 权威状态，同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，守住 ADR 0002 确定性硬门禁、R8.1–R8.6 野怪规则与 1.0 范围 S4 / S5 / S6（`docs/references/WC3-MELEE-RULES.md` §0）。

## Requirements

### Requirement: 野怪与营地作为确定性权威状态

系统 SHALL 在 `w3-core` 新增野怪实体、营地（home 锚点 / 聚类 / 分级）与中立归属的权威状态，以稳定 `EntityId` 顺序的确定性有序容器登记（R8.1 / R8.2 / R1.1）。野怪 MUST 对所有玩家敌对（R8.1）、地面物品 MUST 无主（R1.1）；中立归属建模（`owner: u8` 玩家号或独立中立段）属 Q7 待拍板项，结论未定前 MUST NOT 臆造为「已核」。全部野怪 DefId / 营地分级 / 数值 MUST 为合成原创编译期整数常量 `[待核]`（ADR 0001 / ADR 0004 / §14）。该组权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`（ADR 0002 §6-8）。

#### Scenario: 清野可重复且野怪对所有玩家敌对（R8.1 / R8.2 / R12.4）

- **WHEN** 两实例以相同冻结定义、开局脚本与 `seed` 各清掉同一营地并逐帧推进到全部野怪死亡
- **THEN** 野怪实体 / 营地归属按稳定 `EntityId` 顺序登记、对任一玩家单位可攻击且被攻击（R8.1），每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`；营地全清后状态不被表现层只读派生重建（R12.4 / ADR 0002 §5-6）

#### Scenario: 中立归属与数值零资产且待核（R1.1 / ADR 0001 / §14）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造野怪 / 营地冻结定义并校验其归属字段
- **THEN** 构造成功、零文件 IO、每个字段均为编译期整数常量 `[待核]`，不含 MPQ 派生 / 真实导出 JSON / 参考项目单位名 / rawcode；地面物品为无主（R1.1），中立归属建模按 Q7 拍板结论落地、未定前 MUST NOT 冒充原版事实（ADR 0004 / §14）

### Requirement: 野怪 leash 与归巢 AI

野怪行为 SHALL 采用 `IDLE / ENGAGED / RETURNING` 确定性状态机，由 home 锚点与定义层 `acquisitionRange` 数据驱动，MUST NOT 无限追出营地（R8.5）；`RETURNING` 途中不索敌 / 不反击，归巢后恢复 IDLE。追击 / 归巢 MUST 复用既有 `Attack` / `Move` 命令面与 `run_ai` 确定性重算框架，MUST NOT 新设 AI × lockstep 架构（`WC3-MELEE-RULES.md` §0.3 T5 / :57）。leash 半径 / 归巢阈值 / `acquisitionRange` 全为合成原创 `[待核]` 整数常量、MUST NOT 借鉴 `WarsmashModEngine` 的硬编码 `distance <= 16f` 或 godot `CAMP_LEASH_WC3=1200`（§14 / `design.md`「## 参考项目对照」）。

#### Scenario: 野怪追击不越出 leash 边界并归巢（R8.5 / ADR 0002）

- **WHEN** 玩家单位把营地野怪引出至冻结 leash 半径之外，随后脱离接触并推进若干逻辑帧
- **THEN** 野怪在边界处停止追击、切 `RETURNING`、按定义层 `acquisitionRange` 判据归巢到 home 锚点后恢复 IDLE，全程位置 / 状态经 `w3-core` 确定性命令面推进、逐帧 `compute_state_hash()` 与快照字节与同脚本基线一致；「无限追出营地」的退化实现令该断言失败（R8.5 / ADR 0002 §2-3 / R4 类反向验证）

#### Scenario: 野怪 AI 复用既有命令面且不新设架构（R8.5 / R12.7 / §0.3 T5）

- **WHEN** 校验野怪追击 / 归巢的命令产出与 AI 重算路径
- **THEN** 命令为帧冻结外部命令、经既有 `Attack` / `Move` 与 `run_ai` 确定性重算产出，按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、重放不双重注入；不存在独立 AI × lockstep 注入通道（R12.7 / §0.3 T5 / ADR 0002 §3）

### Requirement: 击杀收益结算

击杀野怪 SHALL 在 `w3-core` 共享结算服务结算经验给附近英雄（R6.1）、金币给击杀方（R8.3）与掉落（R7.2），MUST NOT 由表现层反写 `HeroState.xp` / `PlayerEconomy.gold` / 背包（ADR 0002 §5）。经验分摊 MUST 区分「进入分摊集合」与「实际入账」两环节（R6.5，`design.md`「## 参考项目对照」结论 4）；经验阈值 / 金币数值 / 分摊范围全为 `[待核]`、满级英雄是否参与分摊 `[待核]` 不冻结（§14 / R6.5）。结算 SHALL 按稳定 `EntityId` 顺序、只结算一次、改动的权威字段同步进状态哈希 / 快照。

#### Scenario: 击杀结算经验金币且逐帧一致（R6.1 / R8.3 / ADR 0002）

- **WHEN** 击杀一个野怪且附近有己方英雄，推进结算帧
- **THEN** `HeroState.xp` 与 `PlayerEconomy.gold` 按冻结定义确定性增长、结算只发生一次，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与同脚本基线一致；「击杀无收益 / 表现层反写 xp 或 gold」的退化实现令该断言失败（R8.3 / ADR 0002 §5-6 / Q9）

#### Scenario: 经验分摊两环节与重开无残留（R6.5 / R11.5 / R12.4）

- **WHEN** 多个英雄处于分摊范围并击杀野怪，随后以相同冻结定义与开局脚本重开一局
- **THEN** 分摊集合判定与经验入账按稳定顺序分离可验证、不重复入账；新一局 `victory.resolved == false`、不残留上一局 xp / gold / 掉落登记（R6.5 / R11.5 / R12.4 / ADR 0002 §8）

### Requirement: 掉落表冻结常量与确定性抽取

掉落 SHALL 由「表 → 多个互斥 set → 概率条目」分层的合成原创编译期整数常量 `[待核]` 定义（R7.2 / R8.6），经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部（ADR 0002）。死亡事件 MUST 幂等（同一死亡不重复掉落），同一互斥组 MUST NOT 同时掉两件；抽取结果 MUST 由冻结随机码决定、重放一致（R8.6）。真实导出 `droppedItemSets` / `itemTablePtr` / `randomItemTables` 与具体清单 / 概率 MUST NOT 入库、不因参考项目而「核实」（ADR 0001 / ADR 0004 / §14）。

#### Scenario: 掉落表分层与零资产（R7.2 / R8.6 / ADR 0001）

- **WHEN** 在干净环境构造含掉落表的冻结定义并改动任意概率条目，对比 `content_hash()`
- **THEN** 构造成功、零文件 IO、表 / set / 条目分层均为编译期整数 `[待核]`，`content_hash()` 经 `compute_defs_hash` 改变并进入快照头部；漏接任一段的退化实现被断言拦截（ADR 0002 / R8.6 / `design.md` 结论 3 / 结论 5）

#### Scenario: 幂等死亡与同 seed 可复现（R8.6 / R7.2 / R12.4）

- **WHEN** 对同一野怪重复投递死亡通知，并以相同 `seed` 重放同一清野脚本两次
- **THEN** 掉落只结算一次、同一互斥组不产生两件、两次重放掉落逐帧一致（被状态哈希 / 快照检出）；墙钟 / `HashMap` 迭代顺序 / Godot `randomize()` 等非确定退化实现令该断言失败（R8.6 / ADR 0002 §2-4 / `design.md` 结论 5）

### Requirement: 确定性随机码消费序列

掉落随机码 SHALL 复用 `w3-core` 现有 match-owned 整数 LCG + `seed` 随对局快照（R8.6 / ADR 0002 §4），按死亡事件以稳定 `EntityId` 顺序确定性消费；消费序列 / 随机流状态 MUST 同步进 FNV-1a 状态哈希、`SNAPSHOT_VERSION` 快照与 `first_difference()`（ADR 0002 §6-8）。MUST NOT 使用本地表现层随机源、墙钟、`f64`、Godot `RandomNumberGenerator` / `randomize()`（ADR 0002 §2-4）。消费契约（独立事件驱动流 / 按帧派生 / 消费序列编码）属 Q6 待拍板项，参考项目无现成答案、MUST NOT 臆造。

#### Scenario: 种子随快照且重放逐帧一致（R8.6 / R12.4 / ADR 0002）

- **WHEN** 在帧 N 捕获含随机流状态的快照、销毁实例、恢复后继续跑到帧 M，并与未中断直接跑到帧 M 的实例同脚本比对
- **THEN** `seed` / 随机流状态 / 消费序列从快照头恢复、每帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`，掉落逐帧一致；「随机流漏接哈希或快照」的退化实现被反例断言拦截（R8.6 / ADR 0002 §4 / §7 / Q6）

#### Scenario: 事件驱动消费按稳定顺序且两端一致（R8.6 / R12.2 / ADR 0002）

- **WHEN** 同一帧多个野怪死亡，一端逐帧推进、另一端一次推进 8 帧
- **THEN** 掉落随机码按稳定 `EntityId` 顺序消费、两端逐帧哈希与快照字节逐条一致、重放不双重消费；随机流单一所有权不被表现层可变对象消费（R12.2 / ADR 0002 §3-4 / `design.md` 结论 2）

### Requirement: 地面物品与物品实例权威状态

掉落物 SHALL 作为地面物品权威状态登记（无主，R1.1），同名道具的多个实例 MUST 有独立状态（充能 / 冷却，R7.5），道具效果 MUST 走既有 `EffectDef` 组合、MUST NOT 引入 JASS VM（R7.6）。地面物品 / 物品实例 MUST 同步进状态哈希 / 快照 / `first_difference()`（ADR 0002）。本期**只做掉落事实 / 地面物品权威状态**，拾取 / 六格背包 / 商店购买 / 回城卷轴全部留 P8.5d（`:128`，Q2 推荐范围、待拍板）。地面物品 MUST NOT 出现在 `Move` 命令生成集合（R1.1 / R1.4）。

#### Scenario: 物品实例独立且掉落事实进权威状态（R7.5 / R7.2 / R1.1）

- **WHEN** 同一营地一次清野掉出两件同名道具并校验其权威状态
- **THEN** 两件各有独立实例身份 / 充能 / 冷却（R7.5）、均为无主地面物品（R1.1），掉落事实进 `compute_state_hash()` / `capture_snapshot()` / `first_difference()`；「共用实例」退化实现令该断言失败（ADR 0002 §6-8 / Q9）

#### Scenario: 范围边界与 Move 过滤（R7.6 / R1.4 / R1.1 / Q2）

- **WHEN** 校验本 change 的道具效果路径，并框选含地面物品 / 金矿 / 主城的混合集合对地面下达 `Move`
- **THEN** 道具效果只经既有 `EffectDef` 组合、无 JASS VM 入口（R7.6 / N5）；地面物品 / 金矿 / 主城 MUST NOT 生成 `Move`、位置不变（R1.1 / R1.4），拾取 / 背包 / 商店 / 回城卷轴显式登记为 P8.5d 范围外承接、不冒充本期已解决（Q2 / `design.md` Non-Goals）

### Requirement: 命令与事件面以及线格式契约

按 Q5 推荐（待规格门拍板）：新增 `GameplayCommandKind` / `EventKind` 变体 SHALL 经 `w3-core` 公开提交入口按冻结整数帧号入队、同源校验、稳定排序（R12.2 / ADR 0002），MUST 同步 `hash_gameplay_command` 编码 / 校验 / 穷尽分发；`Event` 若无 payload MUST 扩展只读事实事件供表现层消费（R7.6 / R9.3）。protobuf 字段编号只追加（`reserved 14 to 32` 保留、新增从 **36** 起）、`PROTOCOL_VERSION` 5→6、`Packet::decode` 对旧版本 `VersionMismatch` 明确拒绝、canonical 约束继续适用（ADR 0005）；`w3-render/src/scene.rs` MUST 补穷尽分支、`net-owner` / `client-owner` 按 `CODEBUDDY.md §9` 串行协作、relay 双客户端确定性。

#### Scenario: 帧冻结命令两端一致且不双重注入（R12.2 / R12.4 / ADR 0002）

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一含野怪击杀 / 掉落的命令脚本
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致，命令按稳定排序键执行、重放不双重注入（R12.2 / R12.4 / ADR 0002 §2-3）

#### Scenario: 线格式只追加 tag 且明确拒绝旧版本（ADR 0005 / Q5 / T6 条件命中）

- **WHEN** 新增命令 / 事件变体并接入 protobuf 线格式，随后以旧版本包调用 `Packet::decode`
- **THEN** `hash_gameplay_command` 编码 / 校验 / 穷尽分发覆盖新变体、字段编号只从 36 起追加且 `reserved 14 to 32` 保留、`PROTOCOL_VERSION` 同步 bump 到 6、旧版本被 `VersionMismatch` 明确拒绝、canonical 约束适用、CI 双平台生成代码字节一致、`w3-render` 穷尽匹配无遗漏分支、双客户端逐帧哈希一致（ADR 0005 / `design.md` 结论 6 / Q5）

### Requirement: 新增权威状态进入状态哈希、快照与差异诊断

野怪 / 营地 / leash / home 状态、掉落表消费序列、击杀收益、地面物品 / 物品实例、掉落随机码消费序列这组新增权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 5；按 Q4 推荐 bump 5→6，连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v5 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）定义的快照与 `first_difference()` 差异字段集合；遗漏 MUST 被确定性验收反例断言拦截（ADR 0002 §6-8 / `CODEBUDDY.md §5`）。恢复快照后 SHALL 从帧 N+1 继续、不残留 / 重复掉落或收益（R11.5 / R12.4）。

#### Scenario: 新增字段遗漏进哈希或快照被拦截（ADR 0002 / Q9）

- **WHEN** 以退化实现新增野怪 / 地面物品 / 随机码消费序列字段但未加入 `compute_state_hash()` 或 `capture_snapshot()` 或 `first_difference()`
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败、阻止合并；正式态 26 项全绿（ADR 0002 §8 / `CODEBUDDY.md §5` / `design.md` D5）

#### Scenario: 双实例与快照往返逐帧一致（R12.4 / ADR 0002 §6-8）

- **WHEN** 两实例喂同一含清野 / 掉落命令的脚本交错逐帧推进，并在帧 N 捕获快照、销毁、恢复后继续到帧 M
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`，恢复后野怪 / 掉落 / 收益 / 随机码状态不丢失或重复（R12.4 / R11.5 / ADR 0002 §6-8）

### Requirement: 版本契约与旧版本明确拒绝

`SNAPSHOT_VERSION` / `SIM_VERSION` SHALL 按 Q4 推荐 **bump 5→6**（连同快照头部、单测硬断言、golden hash 基线、主 specs 口径、ADR 0002 复核记录追加；旧 v5 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移，ADR 0002 §7）。`PROTOCOL_VERSION` 按 Q5 推荐 5→6、`Packet::decode` 对旧版本 `VersionMismatch` 明确拒绝（ADR 0005 §3）。新增权威状态 MUST 完整进哈希 / 快照 / `first_difference()`、determinism 26 项全绿；不因「省事」保持 5 而在同一版本号下静默错读新字段（显式失败不静默降级）。

#### Scenario: 旧版本快照与回放被明确拒绝（ADR 0002 §7 / ADR 0005 §3）

- **WHEN** 以 v5 快照 / 旧 `PROTOCOL_VERSION` 包尝试恢复到 bump 6 后的实现
- **THEN** 分别按 `UnsupportedVersion` / `SimVersionMismatch` 与 `VersionMismatch` 明确拒绝、不产生半初始化对局、不提供跨版本迁移垫片（ADR 0002 §7 / ADR 0005 §3 / `design.md` D5）

#### Scenario: bump 6 全链路同步一致（ADR 0002 / Q4）

- **WHEN** 复核 `snapshot.rs` 常量、`sim_match.rs` 单测硬断言、快照头部校验、golden hash 基线与 ADR 0002 复核记录
- **THEN** `SNAPSHOT_VERSION == SIM_VERSION == 6` 且各处一致，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」；MUST NOT 沿用「保持 5 / 零 diff / 逐字节不变」旧断言（ADR 0002 §6-8 / `CODEBUDDY.md §5` / Q4）

### Requirement: 表现层 HUD 只读派生

野怪营地状态 / 掉落事实 / 击杀收益 / 地面物品 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` 与 P8.5b `derive_upgrade_slot` 范式），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5）。只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单 / `f64`；UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。迷雾下野怪可见性属表现层只读派生、留 P8.5e（R10 / Q3，不进权威状态）。

#### Scenario: HUD 只读不改哈希（R10.5 / ADR 0002 §5）

- **WHEN** 连续驱动野怪血条 / 掉落标记 / 击杀收益 HUD 重绘，并与无 HUD 实例同脚本比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、HUD 无 `RtsMatch` 字段写入路径；`w3-render/tests/isolation.rs` 隔离断言全绿（ADR 0002 §5 / `design.md` D6）

#### Scenario: 只读派生边界与迷雾留后续（R10.5 / Q3）

- **WHEN** 校验本 change 的只读视图字段与野怪可见性路径
- **THEN** 只读投影不含墙钟 / `static var` 全局字典 / 建筑名白名单 / `f64`；迷雾下野怪可见性登记为 P8.5e 表现层只读派生、MUST NOT 进权威状态 / 哈希 / 快照（R10 / §0.3 T3 / Q3）

### Requirement: determinism 验收、反向验证与 golden hash 基线

改动 `w3-core` 权威状态 / 哈希编码 / 快照字段 SHALL 通过 `cargo test -p w3-core --test determinism`（26 项）全绿（ADR 0002 §8 / `CODEBUDDY.md §5`）；新增 / 修改的野怪 / 掉落回归 MUST 证明缺陷态失败、修复态通过（Q9 推荐形态①：测试内构造反例 + 元断言防退化），仅「全绿」不构成验证。退化类 SHALL 覆盖 tier / rng / 掉落 / 收益 / 哈希漏接（Q9 / `design.md` D8）。golden hash 基线 SHALL 按新实现如实更新、MUST NOT 冒充逐字节不变、不沿用 P8.5b v5 值；跨平台边界如实标注待验证、不声称全平台达成（Q12 / ADR 0002 复核记录）。

#### Scenario: determinism 26 项与改坏验红退化（ADR 0002 §8 / Q9）

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项），并以退化实现（野怪 / 地面物品 / 随机码消费序列漏接哈希或快照 / 掉落用墙钟或 `HashMap` 迭代顺序 / 击杀无收益 / 掉落随机码消费序列不进快照 / 命令或事件漏接编码或穷尽分发）替换正式实现
- **THEN** 正式态 26 项全绿、缺陷态对应反例断言失败且修复后通过（测试内反例 + 元断言防退化），两条证据可复现（`CODEBUDDY.md §5` / Q9 / `design.md` D8）

#### Scenario: golden hash 如实更新且跨平台不冒称（R12.4 / Q12）

- **WHEN** 以新增权威状态后的实现重取 golden hash 逐帧基线并与 P8.5b v5 旧基线比对
- **THEN** 基线按新实现如实更新、不沿用旧值、不声称逐字节不变；`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6 且与 `SIM_VERSION` / 快照头部 / 单测硬断言 / ADR 0002 复核记录同步一致，Debian 12 侧 v4 / v5 / v6 逐字节比对 / macOS / ARM64 / 其它 rustc 仍如实标注待验证、不声称全平台达成（R12.4 / ADR 0002 / Q12）

### Requirement: 零资产、许可证与范围边界

野怪 DefId / 单位名 / 营地分级 / 掉落清单与概率 / 经验金币 / leash 半径 / `acquisitionRange` / 昼夜时长 / 随机池类别编码全为合成原创编译期整数常量或 `[待核]`（B 类），MUST NOT 含 MPQ 派生 / 真实导出 JSON / 参考项目单位名 / rawcode / 原创数值、MUST NOT 写进 `WC3-MELEE-RULES.md` 当经典规则（§14 / ADR 0001 / ADR 0004）。`w3-content` 原始 `f64` 坐标不进哈希 / 快照；`WarsmashModEngine`（AGPLv3）**仅借语义、禁复制 Java / LibGDX 代码**，`rts_kernel_cs` 的 `double` 路线不采用（ADR 0001 / ADR 0002）。拾取 / 背包 / 商店 / 回城卷轴（P8.5d）、昼夜（R8.4）、AI 清野（§0.3 T5）、迷雾（P8.5e）、中立建筑（R9.3）登记为范围外承接（Q2 / Q3）。

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14）

- **WHEN** 校验本 change 的全部改动与新增常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine` 仅借鉴语义无代码复制、无 Java / LibGDX 依赖，`rts_kernel_cs` 的 `double` 路线不采用，无新增未登记依赖（ADR 0001 / ADR 0004 / `design.md` 结论 4 / 结论 6 / 第八节）

#### Scenario: 范围边界与后续承接（Q2 / Q3 / R8.4 / R9.3）

- **WHEN** 校验本 change 的命令面、掉落闭环与野怪行为集合
- **THEN** 拾取 / 六格背包 / 商店购买 / 回城卷轴（P8.5d）、昼夜睡眠（R8.4）、AI 玩家清野（R12.7 / §0.3 T5）、迷雾下野怪可见性（P8.5e）、中立建筑交互（R9.3）均显式登记为范围外承接、不冒充已解决；与本机导出不符的数值标 `[待核]` 并回到人拍板、不静默改规则（§14 / Q2 / Q3 / `design.md` Non-Goals）
