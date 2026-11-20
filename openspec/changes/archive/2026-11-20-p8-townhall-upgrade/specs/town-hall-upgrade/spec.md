# Spec Delta

## Purpose

定义 P8.5b 主城升级的确定性权威契约：新增「主城等级 / tier」顶层权威状态（T1→T2→T3 逐级解锁二线单位与科技），升级费用 / 时间与 tier 需求链读冻结合成常量，训练 / 研发按定义层前置判据解锁，取消 / 失败事务性退款；tier 与升级队列 / 费用登记全部同步进入状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，守住 ADR 0002 确定性硬门禁、R4.1 逐级解锁与 1.0 范围 S1 / S2 / S3（`docs/references/WC3-MELEE-RULES.md` §0）。

## ADDED Requirements

### Requirement: 主城等级作为确定性顶层权威状态

系统 SHALL 在 `w3-core` 新增「主城等级 / tier」**顶层权威状态**，按玩家主城实体以稳定 `EntityId` / `PlayerId` 顺序的确定性有序容器登记离散等级 T1 / T2 / T3（R4.1 可升级两次）。该状态 MUST 是独立字段，MUST NOT 复用 `PlayerEconomy.upgrades` 的布尔科技旗标或写进表现层；升级后等级 MUST 在 `w3-core` 确定性命令面可被训练 / 研发判据读取（R4.1 / R4.3 / R4.4）。tier **权威状态**与定义层 tier **需求链冻结常量** MUST 明确区分（R12.4 / ADR 0002 / C2 / C4）。

#### Scenario: 升级完成等级递进且逐级生效（R4.1 / R4.3 / R4.4）

- **WHEN** 一个 T1 主城实体的升级命令推进到队列完成，随后提交一项要求 T2 的二线单位 `Train` 与一项要求 T2 的科技 `Research`
- **THEN** 该主城 tier 变为 T2、二线单位与科技的前置判据通过、`Train` / `Research` 被接受并生效；tier 以稳定 `EntityId` 顺序登记、MUST NOT 落进 `PlayerEconomy.upgrades` 布尔旗标（R4.1 / R4.3 / R4.4 / C5）

#### Scenario: 等级不由表现层或布尔旗标派生（R4.1 / ADR 0002 §5）

- **WHEN** 校验 tier 状态的写入路径与读取路径
- **THEN** tier 只由 `w3-core` 升级结算写入、只经确定性命令面读取，表现层 MUST NOT 反写、MUST NOT 出现「HUD 假升级」旁路；退化实现（把离散等级塞进布尔科技旗标或表现层只读派生）令「升级后训练 / 解锁生效」断言失败（R4.1 / R12.4 / D1）

### Requirement: 新增权威状态进入状态哈希、快照与差异诊断

主城等级 / tier、升级队列与费用登记这组新增权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 4；按 Q1 推荐 bump 4→5，连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）定义的快照与 `first_difference()` 差异字段集合；新增字段遗漏 MUST 被确定性验收反例断言拦截（ADR 0002 §6-8 / `CODEBUDDY.md §5`）。恢复快照后 SHALL 从帧 N+1 继续，不残留或重复升级（R12.4）。

#### Scenario: 新增字段遗漏进哈希或快照被拦截（ADR 0002 / Q8）

- **WHEN** 以退化实现新增 tier / 升级队列字段但未加入 `compute_state_hash()` 或 `capture_snapshot()`
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败，阻止合并（ADR 0002 §8 / `CODEBUDDY.md §5` / `design.md` D5）

#### Scenario: 双实例与快照往返逐帧一致（R12.4 / ADR 0002 §6-8）

- **WHEN** 两实例喂同一含升级命令的脚本交错逐帧推进，并在帧 N 捕获快照、销毁、恢复后继续到帧 M
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`，恢复后 tier / 队列 / 费用登记不丢失或重复（R12.4 / ADR 0002 §6-8）

### Requirement: 定义层 tier 需求链与冻结常量数据驱动

「tier → 解锁二线单位与科技需求链」、升级费用 / 时间（整数帧）/ 差价 SHALL 落 `FrozenDefinitions` 冻结常量、经 `compute_defs_hash` 进入 `defs_hash` / `content_hash` 快照头部（Q3 推荐）。需求链 MUST 由定义层类别与 requires / upgrade 数据驱动，MUST NOT 用建筑名字符串白名单或硬编码 DefId 判定解锁（R1.2 / R13.3 / C13）。全部为合成原创编译期整数常量 `[待核]`、零资产、不含 `f64` / MPQ 派生 / 导出 JSON（ADR 0001 / ADR 0004 / §14）。

#### Scenario: 改常量即改定义哈希且零资产（R1.2 / R13.3 / ADR 0001）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造含 tier 需求链的冻结定义，并改动任意升级常量后对比 `content_hash()`
- **THEN** 构造成功、零文件 IO、每个字段为编译期整数常量，`content_hash()` 经 `compute_defs_hash` 改变并进入快照头部；改常量的退化实现被断言拦截（R2 / ADR 0001 / ADR 0004 / D3）

#### Scenario: 解锁由定义层数据而非白名单驱动（R1.2 / R13.3 / C13）

- **WHEN** 以非经典命名的主城 / 二线单位 DefId 提交要求 T2 的 `Train`
- **THEN** 解锁判定只查冻结需求链数据、不查建筑名字符串或硬编码 id；以白名单驱动的退化实现令对应回归失败（R1.2 / R13.3 / `design.md` D3）

### Requirement: 升级命令与事件经确定性命令面

升级 / 取消升级 SHALL 经 `w3-core` 公开提交入口按冻结整数帧号入队、同源校验、按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；MUST NOT 旁路写入权威状态、MUST NOT 与墙钟调用粒度绑定（ADR 0002 §2-3 / R12.4 / R12.7）。按 Q2 推荐新增独立 `GameplayCommandKind::UpgradeTownHall` 命令变体 + 升级开始 / 完成 / 取消事实事件变体，MUST 同步 `hash_gameplay_command` 编码（追加码 13）/ 校验 / 穷尽分发 / `EventKind` 供只读表现层消费（C3 / C21）。新增命令牵动 ADR 0005 线格式约束见下 Scenario。

#### Scenario: 帧冻结命令两端一致（R12.4 / R12.7 / ADR 0002）

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一升级命令脚本
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致，命令按稳定排序键执行、重放不双重注入（R12.4 / R12.7 / ADR 0002 §2-3）

#### Scenario: 新命令变体编码与线格式同步（ADR 0005 / C21 / T6 条件命中）

- **WHEN** 新增 `UpgradeTownHall` 命令变体与升级事件变体并接入 protobuf 线格式
- **THEN** `hash_gameplay_command` 编码 / 校验 / 穷尽分发均覆盖新变体、protobuf 字段编号只追加且保留 `reserved 14 to 32`、`PROTOCOL_VERSION` 同步 bump、旧版本 `Reject` 明确拒绝、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、`net-owner` 按 `CODEBUDDY.md §9` 串行协作、CI 双平台生成代码字节一致（ADR 0005 / C3 / C21）

### Requirement: 训练与研发按 tier 需求链前置解锁

`enqueue_production` 与 `research` SHALL 在读冻结定义的 `train_frames` / `research_frames` 之外，按冻结 tier 需求链校验当前主城等级：要求 T2 / T3 的二线单位 / 科技在当前 tier 未达时 MUST 被显式拒绝、不静默放行、不生成队列项、不改变权威状态（R4.1 / R4.3 / R4.4 / R3.1）。升级完成后解锁 MUST 在同一确定性命令面立即生效，MUST NOT 沿用「全量可训练」旧行为（C4 / 验收 :126）。显式失败不静默降级。

#### Scenario: 未升级时未解锁 Train 与 Research 被拒绝（R4.1 / R4.3 / R4.4）

- **WHEN** 主城为 T1 时提交一项要求 T2 的二线单位 `Train` 与要求 T2 的科技 `Research`
- **THEN** 两条命令被显式拒绝、不生成生产 / 研究队列项、不扣资源、权威状态哈希不变；「假升级」（表现层已显示 T2 但内核仍 T1）退化实现令该断言失败（R4.1 / R4.3 / R4.4 / Q8）

#### Scenario: 升级完成后解锁立即生效（R4.1 / 路线图 :126）

- **WHEN** 主城升级到 T2 的队列完成、同一帧或下一帧再提交要求 T2 的 `Train` / `Research`
- **THEN** 前置判据通过、命令被接受，逐帧 `compute_state_hash()` 与同脚本基线一致（R4.1 / R4.3 / R4.4 / R12.4）

### Requirement: 升级费用事务与取消退款

提交升级命令时 SHALL 先校验 gold / lumber 充足再冻结扣费登记，MUST 从 `PlayerEconomy` 确定性扣减（R3.1）；扣费提交与升级队列效果执行 MUST 分离（主 `openspec/specs/gameplay-systems/spec.md:19`）。取消队列 / 主城被毁 / 对局失败时 MUST 事务性退款并恢复登记、MUST NOT 静默丢队列 / 重复扣费 / 生成半升级实体。升级费用 / 时间 / 差价全为合成原创 `[待核]` 常量，MUST NOT 把导出总价直接当升级扣款（`CLASSIC_RULES_BASELINE.md:46` / ADR 0001 / §14）。退款 MUST 与扣费同样进入状态哈希 / 快照。

#### Scenario: 取消升级事务性退款（R3.1 / R4.1 / ADR 0002）

- **WHEN** 玩家取消一个已扣费的升级队列项，或该队列项因主城被毁 / 对局失败中止
- **THEN** `gold / lumber` 精确回退到扣费前值、升级登记恢复、不产生半升级实体，逐帧 `compute_state_hash()` 与相同脚本基线一致（R3.1 / ADR 0002 §6-8 / D4）

#### Scenario: 资源不足显式失败（R3.1 / 显式失败不静默降级）

- **WHEN** 以不足的 gold / lumber 提交升级命令
- **THEN** 命令被显式拒绝、不生成队列项、不扣费、不产生权威状态漂移，且 MUST NOT 静默降级为免费升级（R3.1 / ADR 0002 / config.yaml context 硬红线）

### Requirement: 表现层 HUD 只读派生与 `U` 键命令卡格位

主城升级进度 / 费用 / 剩余帧与 `U` 键、命令卡升级格位 SHALL 只从只读 `Snapshot`、事实事件与只读视图派生（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` 范式，C6），MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5 边界）。格位与键位 MUST 由定义层数据驱动、MUST NOT 用建筑名字符串白名单（R13 / R13.2 / R13.3 / R13.7 / R1.2）；具体键位绑定 `[待核]`、以本机导出 SLK / 原版客户端核对、不冻结进权威定义、不入库（§14 / ADR 0001）。

#### Scenario: HUD 只读不改哈希（R13 / ADR 0002 §5）

- **WHEN** 升级队列进度随逻辑帧变化并连续驱动 HUD 重绘、切换 `U` 键命令卡格位
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与无 HUD 实例一致，HUD 无 `RtsMatch` 字段写入路径（ADR 0002 §5 / C6 / D7）

#### Scenario: 升级格位由定义层数据驱动（R13.3 / R1.2 / R13.7）

- **WHEN** 以非经典命名的主城 DefId 选中并显示命令卡升级格位
- **THEN** 格位出现与否只查冻结定义需求链 / 类别登记，不查建筑名字符串白名单；白名单退化实现令回归失败（R13.3 / R1.2 / R13.7 / C15）

### Requirement: determinism 验收、反向验证与 golden hash 基线

改动 `w3-core` 权威状态 / 哈希编码 / 快照字段 SHALL 通过 `cargo test -p w3-core --test determinism`（26 项）全绿（ADR 0002 §8 / `CODEBUDDY.md §5`）；新增 / 修改的升级回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照），仅「全绿」不构成验证（Q8）。golden hash 基线 SHALL 按新实现如实更新、MUST NOT 冒充逐字节不变、不沿用 P8.5a v4 值；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），新增字段 MUST 完整进哈希 / 快照 / `first_difference()`（R12.4 / ADR 0002 / C1）。

#### Scenario: determinism 26 项全绿与改坏验红四类退化（ADR 0002 §8 / Q8）

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项），并以退化实现（tier 权威状态漏接哈希或快照 / 未升级仍接受未解锁 `Train` 的假升级 / 升级费用不扣 gold 或取消不退款 / 升级命令漏接 `hash_gameplay_command` 编码或穷尽分发）替换正式实现
- **THEN** 正式态 26 项全绿、缺陷态对应断言失败且修复后通过，两条证据可复现（`CODEBUDDY.md §5` / C23 / Q8）

#### Scenario: golden hash 基线如实更新且跨平台边界不冒称（R12.4 / Q7）

- **WHEN** 以新增权威状态后的实现重取 golden hash 逐帧基线并与 P8.5a v4 旧基线比对
- **THEN** 基线按新实现如实更新、不沿用旧值、不声称逐字节不变；`SNAPSHOT_VERSION` 按 Q1 推荐 bump 5 且与 `SIM_VERSION` / 快照头部 / 单测硬断言 / ADR 0002 复核记录同步一致，Debian 12 侧逐字节比对 / macOS / ARM64 / 其它 rustc 仍如实标注待验证、不声称全平台达成（R12.4 / ADR 0002 / Q7）

### Requirement: 零资产、许可证与范围边界

升级数值 / tier 名称 / 两族升级链 / 二线单位与科技清单 / 键位绑定全为合成原创编译期整数常量或 `[待核]`（B 类，C17），MUST NOT 含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名 / rawcode / 原创数值、MUST NOT 写进 `WC3-MELEE-RULES.md` 当经典规则（§14 / ADR 0001 / ADR 0004）。`w3-content` 原始 `f64` 坐标不进哈希 / 快照；`WarsmashModEngine`（AGPLv3）**仅借鉴语义、禁止复制任何 Java / LibGDX 代码**，`rts_kernel_cs` 的 `double` 路线不采用（ADR 0002 / C15 / C16）。升级命令复用既有 `SetAi` / `run_ai` 确定性外部命令面，AI 席位决策 / lockstep 注入架构按 §0.3 T5 留后续 change，本 change 不新设架构（Q6 / R12.7）。

#### Scenario: 零资产与许可证边界（ADR 0001 / ADR 0004 / §14）

- **WHEN** 校验本 change 的全部改动与新增常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine` 仅借鉴语义无代码复制，`rts_kernel_cs` 的 `double` 路线不采用（ADR 0001 / ADR 0004 / C15 / C16）

#### Scenario: 范围边界与 AI 命令面承接（Q4 / Q6 / R12.7）

- **WHEN** 校验本 change 的命令面、解锁清单与 AI 升级命令注入路径
- **THEN** T3 全量链 / 两族完整清单 / 差价 / `U` 键绑定 `[待核]` 显式登记为范围外承接、不冒充已解决；AI 升级命令经既有 `SetAi` / `run_ai` 外部命令面、不旁路写入权威状态，AI × lockstep 注入架构按 §0.3 T5 留后续 change（Q4 / Q6 / R12.7 / `design.md` D8）
