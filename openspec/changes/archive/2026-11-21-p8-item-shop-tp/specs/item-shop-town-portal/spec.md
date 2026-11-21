# Spec Delta

## Purpose

定义 P8.5d 物品 / 商店 / 回城卷轴的确定性权威契约：英雄六格背包与物品实例独立充能 / 冷却、地面拾取闭环、商店购买事务与定义层货架、售卖、使用道具、丢弃 / 移交、回城卷轴传送全部作为 `w3-core` 权威状态，同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，守住 ADR 0002 确定性硬门禁、R6 / R7 / R9 规则与 1.0 范围 S4 / S6（`docs/references/WC3-MELEE-RULES.md` §0）；承接 P8.5c 明文交接（`creep-camps/spec.md:80,164` / `creep.rs:331`）的掉落事实 / 地面物品 / 物品实例状态。

## ADDED Requirements

### Requirement: 六格背包与物品实例独立权威状态

系统 SHALL 在 `w3-core` 为每个英雄新增**定长 6 格背包**权威状态（R6.2），每格挂一个物品实例的稳定 `EntityId`（R7.5）；实例身份 / 充能 / 冷却 / `EffectDef` MUST 挂实例而非 `ItemDef`（R7.5 / R7.6），同名道具多个实例 MUST NOT 共用充能或冷却。背包与实例归属（owner / slot）SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q4 推荐 6→7）定义的快照与 `first_difference()`（ADR 0002 §6-8）。满包 MUST 显式拒绝、不静默丢弃。全部数值为合成原创编译期整数常量 `[待核]`（ADR 0001 / §14）。

#### Scenario: 六格定长与实例独立充能（R6.2 / R7.5 / ADR 0002）

- **WHEN** 给同一英雄放入 6 件道具（含两件同名卷轴）并分别使用其中一件、推进若干逻辑帧
- **THEN** 背包恰为 6 格、每格挂独立 `EntityId`，两件同名卷轴各有独立 `charges` / `cooldown_frames`（R7.5），第 7 件被显式拒绝；背包 / owner / slot / 充能 / 冷却进 `compute_state_hash()` / `capture_snapshot()` / `first_difference()`，双实例逐帧哈希一致（ADR 0002 §6-8 / R12.4）；「共用实例 / 共用充能」退化实现令该断言失败（Q9）

#### Scenario: 满包拒绝与复活恢复实例身份（R6.2 / R6.3 / R7.5）

- **WHEN** 满包时拾取 / 购买第 7 件道具，并让持包英雄阵亡后经祭坛复活（`complete_revive()`，`sim_match.rs:1526-1558`）
- **THEN** 满包购买 MUST 事务性退款且不生成半实例（R7.3 / R9.3），复活后六格背包按实例 `EntityId` 与剩余充能 / 冷却逐项恢复、MUST NOT 回退为 `bag: Vec<DefId>` 的 DefId 序列（R6.3 / R7.5），逐帧哈希与同脚本基线一致（R12.4）

### Requirement: 拾取地面物品闭环与 P8.5c 交接

系统 SHALL 让英雄拾取 P8.5c 掉落的**无主地面物品**（R1.1 / R7.2）：拾取 MUST 把 `CreepCampState.ground_items` / `item_instances` / `neutral`（`creep.rs:394-419`）的实例身份迁移进六格背包、MUST NOT 新建实例或重置充能 / 冷却（R7.5）。地面物品是「不是 EntityStore 实体、不进 `Move` 生成集合」的中性登记实体（`creep.rs:34-38`），拾取后其无主标记 MUST 清除（R1.1 / R1.4）。拾取半径 / 判据为定义层 `[待核]` 整数常量、MUST NOT 用名字白名单（R1.2 / §14）。

#### Scenario: 拾取迁移实例身份且不重置充能（R1.1 / R7.5 / R12.4）

- **WHEN** 英雄走近一件 P8.5c 掉落的地面物品（其 `ItemInstanceState.charges` 已被消耗 1 次、`cooldown_frames` 未归零）并触发拾取
- **THEN** 地面项从 `ground_items` 移除、`neutral` 的无主标记清除，同一 `EntityId` 的实例进入背包某格、`charges` / `cooldown_frames` 原值不变（R7.5），`compute_state_hash()` / `capture_snapshot()` 反映迁移且双实例逐帧一致（R12.4 / ADR 0002）；「新建实例 / 刷新冷却」退化实现被回归拦截（Q9）

#### Scenario: 地面物品不响应 Move 且拾取幂等（R1.1 / R1.4 / R13.6）

- **WHEN** 框选含英雄 / 地面物品 / 金矿 / 主城的混合集合对地面下达 `Move`，并对同一地面物品重复投递拾取
- **THEN** 只有英雄生成 `Move`、地面物品 / 金矿 / 主城位置不变（R1.1 / R1.4），拾取只成功一次、第二次被显式拒绝（幂等），MUST NOT 借只读视图投影为地面物品或新增命令变体放松旁路门禁（R13.6 `:314`），逐帧哈希与只读基线一致（ADR 0002 §5）

### Requirement: 商店购买事务与定义层货架

商店购买 SHALL 为单一事务：交互半径 / 使用者 / 货架可用性 / 黄金 / 库存 / 背包空位校验 → 精确扣费 → 建实例 → 入六格背包 → 库存递减（R7.3 / R9.1 / R9.3）；任一校验失败或后续步骤失败 MUST 事务性回滚退款、恢复库存、不生成半实例（R9.3）。货架清单 / 类别 / 价格 / 库存上限 / 刷新间隔 / 解锁时间 SHALL 为 `FrozenDefinitions` 冻结定义层常量 `[待核]`、经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部；商店库存 / 刷新计数是**权威状态**进哈希 / 快照 / `first_difference()`（ADR 0002）。商店建模（R9.1 中立建筑实体 vs 定义层货架）属 Q6 待拍板项。

#### Scenario: 扣费入包库存递减事务与失败退款（R7.3 / R9.3 / ADR 0002）

- **WHEN** 英雄在交互半径内购买货架道具，分别在黄金充足 / 不足、背包有空位 / 满包、库存有余 / 售罄三种条件下提交购买命令
- **THEN** 成功路径精确扣 `PlayerEconomy.gold`、建独立实例入包、库存递减且四者同进 `compute_state_hash()` / `capture_snapshot()`；失败路径精确退款 / 恢复库存 / 不生成半实例（复用 `submit_*` / `refund_*` 事务先例，`sim_match.rs:1376,1411,1594,1634`），双实例逐帧哈希一致（R12.4 / R9.3 / ADR 0002 §6-8）

#### Scenario: 货架定义层数据与零资产（R7.3 / R9.1 / ADR 0001 / §14）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造商店冻结定义，改动任一货架价格 / 库存条目，对比 `content_hash()`
- **THEN** 构造成功、零文件 IO、货架全部为合成原创编译期整数 `[待核]`（不含 `ngme` / `nmrk` / 半价 / `goldCost` / `stockMax` 等参考项目值，规则 `:382` / `:373`），`content_hash()` 经 `compute_defs_hash` 改变并进快照头部；交互判据由定义层类别数据驱动、MUST NOT 用建筑名白名单（R9.1 / R1.2 / §14）

#### Scenario: 黄金结算不新增旁路写入（R9.3 / R13.6 / R1.4）

- **WHEN** 校验购买 / 拾取 / 售卖的黄金结算路径与命令生成集合
- **THEN** 黄金变动只经既有命令面与共享结算服务、MUST NOT 新增旁路写入或由表现层反写 `PlayerEconomy.gold`（R9.3 / ADR 0002 §5）；商店（中立建筑）/ 地面物品不可移动、MUST NOT 生成 `Move`（R1.4 / R9.1），新增命令变体 MUST NOT 借 R13.6 `:314` 的只读视图约束放松旁路门禁

### Requirement: 道具售卖与回购价

系统 SHALL 允许把打野 / 购入的道具售卖换回黄金（R7.4），回购价 MUST 显著低于购入价、为定义层 `[待核]` 整数常量；售卖 MUST 以实例为单位移出六格背包、精确入账黄金、登记事实事件，MUST NOT 由表现层反写 `PlayerEconomy.gold`（R9.3 / ADR 0002 §5）。售卖比例 / 冷却 / 是否可售为冻结定义层字段 `[待核]`（R7.4 / §14）。

#### Scenario: 售卖按实例入账且不反写（R7.4 / R9.3 / R12.4）

- **WHEN** 英雄售卖六格背包中一件带剩余充能 / 冷却的道具，随后重放同一脚本
- **THEN** 该实例从背包移除、`PlayerEconomy.gold` 按 `[待核]` 回购价精确增长、实例不残留可再售卖，黄金与背包变动同进状态哈希 / 快照、`first_difference()`，逐帧哈希与同脚本基线一致（R12.4 / ADR 0002 §6-8）；「表现层反写 gold / 重复售卖」退化实现被回归拦截（Q9）

#### Scenario: 回购价不冒充经典值（R7.4 / §14 / ADR 0001）

- **WHEN** 校验售卖回购价的定义层来源
- **THEN** 回购价为合成原创 `[待核]` 编译期整数、MUST NOT 写死 godot 半价 / Warsmash 数据表值当经典规则（规则 `:382` / `:373`），与本机导出不符标 `[待核]` 并回到人拍板、不静默改规则（§14 / ADR 0001 / ADR 0004）

### Requirement: 使用道具、充能冷却与 EffectDef

使用道具 SHALL 为「先验证有效（充能 > 0 / 冷却到期 / 目标合法）再提交」的事务，成功 MUST 递减实例 `charges`、置整数帧冷却、按实例的 `EffectDef` 组合结算效果；失败 MUST 不扣次数（R7.5 / R7.6）。效果 MUST 走既有 `EffectDef` / `EffectKind` 组合、MUST NOT 执行任意 JASS 技能或引入 JASS VM（R7.6 / N5）。消耗品用完即移除实例（R7.1 / R7.5）。充能 / 冷却 / 效果 / 共享冷却组为实例或定义层字段、MUST NOT 用 `float` / 墙钟（ADR 0002 §2-4）。

#### Scenario: 使用事务与实例充能冷却（R7.5 / R7.6 / ADR 0002）

- **WHEN** 分别使用一件充能耗尽 / 冷却未到 / 目标合法的道具，并对合法目标使用消耗品
- **THEN** 前两者显式拒绝且不扣 `charges`（R7.5），合法使用按 `EffectDef` 组合确定性结算、`charges` 递减 / 冷却按整数帧置位、用完移除实例，全程无 JASS / wtg VM 入口（R7.6 / N5），逐帧 `compute_state_hash()` 与同脚本基线一致（R12.4 / ADR 0002 §2-4）

#### Scenario: 共享冷却与非确定退化被拒（R7.5 / ADR 0002 / Q9）

- **WHEN** 同组道具连续使用并校验冷却来源，另以墙钟 / `Time.get_ticks_msec()` / `HashMap` 迭代顺序 / Godot `RandomNumberGenerator` 退化实现替换
- **THEN** 共享冷却按实例组整数帧一致推进、两实例逐帧哈希相等（R12.4）；墙钟 / 引擎浮点 / 全局静态字典 / `HashMap` 顺序退化实现令确定性断言失败（ADR 0002 §2-3 / `design.md` 结论 3「明确不借鉴」/ Q9）

### Requirement: 丢弃与移交保持实例身份

丢弃 / 移交道具 SHALL 保持物品实例 `EntityId` 身份与剩余充能 / 冷却「转交也不能刷新」（R6.2 / R7.5）：丢弃 MUST 把实例从六格背包移回无主地面物品（更新 `ground_items` / `neutral`，R1.1）、位置由确定性整数坐标给出；移交 MUST 在两英雄六格背包间迁移同一实例、源格清空、目标满包则显式拒绝（R6.2）。丢弃 / 移交事实 MUST 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §6-8）。

#### Scenario: 丢弃带剩余冷却转地面（R1.1 / R7.5 / R12.4）

- **WHEN** 英雄丢弃一件刚使用过、冷却未归零的道具，随后另一英雄走近拾取
- **THEN** 实例以同一 `EntityId` 成为无主地面物品、`charges` / `cooldown_frames` 原值带入且继续按整数帧递减（R7.5 / R1.1），拾取后仍保持该身份与剩余冷却，双实例逐帧哈希一致（R12.4 / ADR 0002）；「刷新冷却 / 新建实例」退化实现被拦截（Q9）

#### Scenario: 移交满包拒绝与 Move 过滤（R6.2 / R1.4 / R13.6）

- **WHEN** 把一件道具从英雄 A 移交满包的英雄 B，并框选含 A / 地面物品 / 商店的混合集合对地面下达 `Move`
- **THEN** 满包移交显式拒绝、源格不变（R6.2）；只有可移动单位生成 `Move`、地面物品 / 商店（中立建筑）位置不变（R1.1 / R1.4 / R13.6），逐帧哈希与只读基线一致（ADR 0002 §5）

### Requirement: 回城卷轴目标友方主城、拒单与施法中断

回城卷轴 SHALL 有独立命令 / 事件语义（R7.1），传送目标 MUST 按「友方主城分类」定义层数据驱动、MUST NOT 用名字白名单或复制 War3 rawcode（参考对照结论 7 / R1.2）；无可用友方主城 MUST 显式拒单、不扣次数（R7.5）。施法帧 / 目标 / 中断 / 次数 MUST 为独立权威状态进哈希 / 快照 / `first_difference()`（ADR 0002 §6-8）；施法期间被击 / 主城被毁等中断 MUST 显式取消并结算（R7.5）。回城卷轴 MUST NOT 与大法师 MassTeleport 群体传送混淆（godot `ITEMS_AND_MELEE_AI_PLAN.md:129`）。目标语义与 `Event` 表达属 Q5 / Q6 待拍板项。

#### Scenario: 目标按友方主城分类且无主城拒单（R7.1 / R7.5 / R12.4）

- **WHEN** 在「有可用友方主城」与「全部友方主城已毁」两种对局状态下使用回城卷轴
- **THEN** 前者以友方主城分类选稳定目标、施法帧 / 目标 / 次数进权威状态、传送到整数坐标且逐帧哈希与同脚本基线一致；后者显式拒单、不扣 `charges`（R7.5 / R12.4 / ADR 0002）；MUST NOT 用建筑名字符串或 `852286` / `AItp` / `AIte` rawcode 判定（R1.2 / 规则 `:382`）

#### Scenario: 施法中断与 MassTeleport 区分（R7.5 / R7.6 / ADR 0002）

- **WHEN** 施法期内目标主城被毁 / 持有者被打断，并校验回城卷轴的效果组合路径
- **THEN** 中断显式取消、施法状态清理、次数按冻结定义结算、不产生半传送位置，权威状态进哈希 / 快照（ADR 0002 §6-8）；效果只经既有 `EffectDef` 组合（R7.6 / N5），MUST NOT 实现 `channelEndTick` / `areaOfEffect` / 多单位的群体传送（结论 7 / `design.md`「## 参考项目对照」结论 7）

### Requirement: 命令与事件面、只传命令与回放一致

按 Q5 推荐（待规格门拍板）：拾取 / 购买 / 使用 / 丢弃 / 售卖 / 回城相关 `GameplayCommandKind` / `EventKind` 变体 SHALL 经 `w3-core` 公开提交入口按冻结整数帧号入队、同源校验、稳定排序（R12.2 / ADR 0002），MUST 同步 `hash_gameplay_command` 编码 / 校验 / 穷尽分发；`Event` 无 payload 时 MUST 以 `Event.entity` 查只读 `Snapshot` / 投影表表达事实（照 P8.5c D6）。protobuf 字段编号只追加（`reserved 14 to 32` 保留、新增从 **37** 起）、`PROTOCOL_VERSION` 6→7、`Packet::decode` 对旧版本 `VersionMismatch` 明确拒绝、canonical 约束继续适用（ADR 0005）；只传玩家输入命令、MUST NOT 传游戏状态（R12.2）。

#### Scenario: 帧冻结命令两端一致且不双重注入（R12.2 / R12.4 / ADR 0002）

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一含拾取 / 购买 / 使用 / 回城的命令脚本
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致，命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序执行、重放不双重注入（R12.2 / R12.4 / ADR 0002 §2-3），线格式只传命令不传状态

#### Scenario: 线格式只追加 tag 且明确拒绝旧版本（R12.2 / ADR 0005 / Q5 / T6 条件命中）

- **WHEN** 新增命令 / 事件变体并接入 protobuf 线格式，随后以旧版本包调用 `Packet::decode`
- **THEN** `hash_gameplay_command` 编码 / 校验 / 穷尽分发覆盖新变体、字段编号只从 37 起追加且 `reserved 14 to 32` 与 tag 33–36 不动、`PROTOCOL_VERSION` 同步 bump 到 7、旧版本被 `VersionMismatch` 明确拒绝、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）适用、CI 双平台生成代码字节一致、`w3-render` 穷尽匹配无遗漏分支、双客户端逐帧哈希一致（R12.2 / R12.4 / ADR 0005 / `design.md` 结论 8）

### Requirement: 确定性随机码消费与市场刷新

市场刷新随机码 SHALL 复用 P8.5c match-owned 整数 LCG + `seed` 随对局快照、`rng_state` / `rng_draws` / `next_rng_seq` 事件驱动流 + 显式消费序列契约（R8.6 / ADR 0002 §4），按稳定 `EntityId` 顺序确定性消费；消费序列 / 随机流状态 MUST 同步进 FNV-1a 状态哈希、`SNAPSHOT_VERSION` 快照与 `first_difference()`（ADR 0002 §6-8）。MUST NOT 用本地表现层随机源、墙钟、`f64`、Godot `RandomNumberGenerator` / `randomize()` / `HashMap` 迭代顺序。刷新事件 MUST 幂等、同一刷新批次不重复消费（R7.3 / R8.6）。「独立专用刷新流 vs 复用掉落流」属 Q6 待拍板项。

#### Scenario: 种子随快照且重放逐帧一致（R8.6 / R12.4 / ADR 0002）

- **WHEN** 在帧 N 捕获含随机流状态的快照、销毁实例、恢复后继续跑到帧 M，并与未中断直接跑到帧 M 的实例同脚本比对
- **THEN** `seed` / `rng_state` / 消费序列从快照恢复、每帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`，市场刷新清单逐帧一致（R8.6 / R12.4）；「随机流漏接哈希或快照」的退化实现被反例断言拦截（ADR 0002 §4 / §7 / Q6）

#### Scenario: 事件驱动消费按稳定顺序且两端一致（R8.6 / R12.2 / ADR 0002）

- **WHEN** 同一帧多个商店批次触发刷新，一端逐帧推进、另一端一次推进 8 帧，并对同一刷新批次重复投递通知
- **THEN** 随机码按稳定 `EntityId` 顺序消费、两端逐帧哈希与快照字节逐条一致、重放不双重消费、幂等不重复刷新（R8.6 / R12.2 / ADR 0002 §3-4），随机流单一所有权不被表现层可变对象消费（结论 8）

### Requirement: 新增权威状态进入状态哈希、快照与差异诊断

六格背包 / 实例 owner 与 slot / 充能 / 冷却 / `EffectDef` 绑定、拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖事实、商店库存与刷新计数、回城传送（施法帧 / 目标 / 中断）、确定性随机码消费序列这组新增权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 6；按 Q4 推荐 bump 6→7，连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）定义的快照与 `first_difference()` 差异字段集合；遗漏 MUST 被确定性验收反例断言拦截（ADR 0002 §6-8 / `CODEBUDDY.md §5`）。恢复快照后 SHALL 从帧 N+1 继续、不残留 / 重复结算（R11.5 / R12.4）。

#### Scenario: 新增字段遗漏进哈希或快照被拦截（ADR 0002 / R12.4 / Q9）

- **WHEN** 以退化实现新增六格背包 / 实例充能 / 商店库存 / 回城传送字段但未加入 `compute_state_hash()` 或 `capture_snapshot()` 或 `first_difference()`
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败、阻止合并；正式态 26 项全绿（ADR 0002 §8 / `CODEBUDDY.md §5` / `design.md` D2 / R12.4）

#### Scenario: 双实例与快照往返逐帧一致（R12.4 / R11.5 / ADR 0002 §6-8）

- **WHEN** 两实例喂同一含拾取 / 购买 / 使用 / 售卖 / 回城命令的脚本交错逐帧推进，并在帧 N 捕获快照、销毁、恢复后继续到帧 M
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`，恢复后背包 / 实例 / 库存 / 传送 / 消费序列状态不丢失或重复（R12.4 / R11.5 / ADR 0002 §6-8）

### Requirement: 版本契约与旧版本明确拒绝

`SNAPSHOT_VERSION` / `SIM_VERSION` SHALL 按 Q4 推荐 **bump 6→7**（连同快照头部、单测硬断言、golden hash 基线、主 specs 口径、ADR 0002 复核记录追加；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移，ADR 0002 §7）。`PROTOCOL_VERSION` 按 Q5 推荐 6→7、`Packet::decode` 对旧版本 `VersionMismatch` 明确拒绝（ADR 0005 §3 / R12.2）。新增权威状态 MUST 完整进哈希 / 快照 / `first_difference()`、determinism 26 项全绿；不因「省事」保持 6 而在同一版本号下静默错读新字段（R12.4 / 显式失败不静默降级）。两套版本号独立、各自 bump（R11）。

#### Scenario: 旧版本快照与回放被明确拒绝（ADR 0002 §7 / ADR 0005 §3 / R12.4）

- **WHEN** 以 v6 快照 / 旧 `PROTOCOL_VERSION` 包尝试恢复到 bump 7 后的实现
- **THEN** 分别按 `UnsupportedVersion` / `SimVersionMismatch` 与 `VersionMismatch` 明确拒绝、不产生半初始化对局、不提供跨版本迁移垫片（ADR 0002 §7 / ADR 0005 §3 / R12.4 / `design.md` D3）

#### Scenario: bump 7 全链路同步一致（ADR 0002 / R12.4 / Q4）

- **WHEN** 复核 `snapshot.rs` 常量、`sim_match.rs` 单测硬断言、快照头部校验、golden hash 基线与 ADR 0002 复核记录
- **THEN** `SNAPSHOT_VERSION == SIM_VERSION == 7` 且各处一致，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」；MUST NOT 沿用「保持 6 / 零 diff / 逐字节不变」旧断言（ADR 0002 §6-8 / `CODEBUDDY.md §5` / R12.4 / Q4）

### Requirement: determinism 验收、反向验证与 golden hash 基线

改动 `w3-core` 权威状态 / 哈希编码 / 快照字段 SHALL 通过 `cargo test -p w3-core --test determinism`（26 项）全绿（ADR 0002 §8 / `CODEBUDDY.md §5` / R12.4）；新增 / 修改的物品 / 商店 / 回城回归 MUST 证明缺陷态失败、修复态通过（Q9 推荐形态①：测试内构造反例 + 元断言防退化），仅「全绿」不构成验证。退化类 SHALL 覆盖六格背包 / 实例充能冷却 / 购买事务 / 哈希漏接 / 命令编码漏接（Q9 / `design.md` D4）。golden hash 基线 SHALL 按新实现如实更新、MUST NOT 冒充逐字节不变、不沿用 P8.5c v6 值 `0xc802b63cd6c55b74`；跨平台边界如实标注待验证、不声称全平台达成（R12.4 / Q12）。

#### Scenario: determinism 26 项与改坏验红退化（ADR 0002 §8 / R12.4 / Q9）

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项），并以退化实现（六格背包 / 实例 owner 与 slot / 商店库存 / 回城传送漏接哈希或快照 / 拾取购买不经事务命令面 / 同名实例共用充能 / 刷新用墙钟或非确定随机源 / 消费序列漏接 / 命令或事件漏接编码或穷尽分发）替换正式实现
- **THEN** 正式态 26 项全绿、缺陷态对应反例断言失败且修复后通过（测试内反例 + 元断言防退化），两条证据可复现（`CODEBUDDY.md §5` / R12.4 / Q9 / `design.md` D4）

#### Scenario: golden hash 如实更新且跨平台不冒称（R12.4 / R11 / Q12）

- **WHEN** 以新增权威状态后的实现重取 golden hash 逐帧基线并与 P8.5c v6 旧基线比对
- **THEN** 基线按新实现如实更新、不沿用旧值、不声称逐字节不变；`SNAPSHOT_VERSION` 按 Q4 推荐 bump 7 且与 `SIM_VERSION` / 快照头部 / 单测硬断言 / ADR 0002 复核记录同步一致，Debian 12 侧 v4 / v5 / v6 / v7 逐字节比对 / macOS / ARM64 / 其它 rustc 仍如实标注待验证、不声称全平台达成（R12.4 / R11 / ADR 0002 / Q12）

### Requirement: 表现层 HUD 只读派生、零资产、许可证与范围边界

六格背包 / 商店命令卡 / 购买 / 拾取 / 回城进度 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 P8.5a `ReviveHudModel::derive` / P8.5b `derive_upgrade_slot` 范式），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5 / R13.5）。UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）。全部数值为合成原创编译期整数 `[待核]`、仓库零资产、许可证边界与范围外承接见下列各 Scenario（ADR 0001 / ADR 0004 / §14 / Q2 / Q3）。

#### Scenario: HUD 只读不改哈希（R10.5 / R13.5 / ADR 0002 §5）

- **WHEN** 连续驱动六格背包 / 商店命令卡 / 购买进度 / 回城施法条 HUD 重绘，并与无 HUD 实例同脚本比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、HUD 无 `RtsMatch` 字段写入路径；`w3-render/tests/isolation.rs` 隔离断言全绿（ADR 0002 §5 / R13.5 / R12.4 / `design.md` D8）；键位 / 命令卡状态为表现层只读派生、不进权威状态（R13.5 / R13.6）

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14 / R7.x）

- **WHEN** 校验本 change 的全部改动与新增常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine` 仅借鉴语义无代码复制、无 Java / LibGDX 依赖，`rts_kernel_cs` 的 `double` 路线不采用，无新增未登记依赖（ADR 0001 / ADR 0004 / `design.md` 第八节 / §14）

#### Scenario: 范围边界与后续承接（Q2 / Q3 / R10 / R8.4 / R9.3 / §14）

- **WHEN** 校验本 change 的命令面、拾取 / 购买 / 使用闭环与物品 / 商店行为集合
- **THEN** P8.5e 迷雾下商店 / 地面物品可见性（R10 / R10.5）、R8.4 昼夜睡眠、AI 玩家清野 / 拾取（R12.7 / §0.3 T5）、R9 中立建筑交互全量均显式登记为范围外承接、不冒充已解决；与本机导出不符的数值标 `[待核]` 并回到人拍板、不静默改规则（§14 / Q2 / Q3 / `design.md` Non-Goals）
