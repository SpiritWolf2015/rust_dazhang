# hero-revive Specification

## Purpose
定义 P8.5a 英雄复活的确定性权威契约：英雄阵亡后经本族祭坛排队复活、恢复同一个英雄实体（等级 / 经验 / 技能等级 / 6 格背包），费用与时间读冻结合成常量且随等级计算，取消 / 失败事务性退款；死亡登记、复活队列与费用全部作为新增权威状态同步进入状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，守住 ADR 0002 确定性硬门禁与 1.0 范围 S1 / S2 / S3（`docs/references/WC3-MELEE-RULES.md` §0）。

## Requirements

### Requirement: 英雄死亡登记为确定性权威状态

英雄阵亡时 SHALL 在 `w3-core` 登记一条死亡记录，键为稳定 `EntityId`、含整数逻辑帧号 `death_frame: u64` 与待恢复的英雄状态 / 背包 / 技能等级快照；登记 MUST 落在 `BTreeMap` 确定性有序容器，按稳定 `EntityId` 顺序遍历。MUST NOT 使用墙钟毫秒、渲染帧率、全局静态字典或 `HashMap` 迭代顺序（ADR 0002 §2-3、`docs/decisions/0002-determinism.md:30-41`）。死亡实体 MUST NOT 被删除、英雄死亡 MUST NOT 结束对局（R6.3 / R11.2）。

#### Scenario: 死亡登记由帧号与定义层类别驱动

- **WHEN** 某英雄在整数逻辑帧 N 阵亡、且其归属玩家拥有 `CATEGORY_ALTAR` 类别的祭坛
- **THEN** 死亡记录以 `death_frame == N` + 稳定 `EntityId` 写入 `BTreeMap`，实体 `alive == false` 且 `hero` / `bag` 保留，祭坛判定只由定义层类别驱动、MUST NOT 用建筑 id 字符串白名单（R1.2 / R13.3 / ADR 0002 / R4.2）

#### Scenario: 墙钟与全局静态状态被拒绝

- **WHEN** 以退化实现（`Time.get_ticks_msec()` 之类墙钟或 `static var` 全局字典）实现死亡登记，并以正式实现（整数帧号 `death_frame: u64` + `BTreeMap`）作对照
- **THEN** 缺陷态（墙钟 / 全局静态字典）对应的确定性反例断言 MUST 失败、该实现 MUST 被判定违反 ADR 0002 §2-3 而拒绝；正式态两实例交错推进逐帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`（ADR 0002 §2-3 / §6-8 / R12.4 / C19 / `CODEBUDDY.md §5`）

### Requirement: 死亡登记进入状态哈希、快照与差异诊断

死亡登记、复活队列与费用这组新增权威状态 SHALL 同步进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（按 Q1 拍板 bump 4；连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新，旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）定义的快照与 `first_difference()` 差异字段集合；新增字段遗漏 MUST 被确定性验收测试拦截（ADR 0002 §6-8 / `CODEBUDDY.md §5` / 主 `openspec/specs/gameplay-systems/spec.md:4,19`）。恢复快照后 SHALL 从帧 N+1 继续，不残留或重复复活（R12.4）。

#### Scenario: 新增字段遗漏进哈希被拦截

- **WHEN** 新增死亡登记 / 复活队列字段但未加入状态哈希或快照
- **THEN** `cargo test -p w3-core --test determinism` 对应反例断言失败，阻止合并（ADR 0002 §8 / `CODEBUDDY.md §5`）

#### Scenario: 双实例与快照往返逐帧一致

- **WHEN** 两实例喂同一含复活命令的脚本交错逐帧推进，并在帧 N 捕获快照、销毁、恢复后继续到帧 M
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`，恢复后死亡登记 / 队列 / 费用不丢失或重复（R12.4 / ADR 0002 §6-8）

### Requirement: 祭坛复活队列恢复同一英雄实体

祭坛复活 SHALL 作为训练 / 建造队列的一类队列项，携带目标英雄 `EntityId` 与费用 / 时间；队列完成时 MUST **恢复同一个英雄实体**（同一 `EntityId`、保留 `hero.level` / `hero.xp` / 6 格背包 / 技能等级），置回存活、按冻结常量恢复生命 / 法力、改到祭坛位置，MUST NOT `entities.spawn()` 新实体、MUST NOT 得到 `hero=None` 空背包（R4.2 / R6.1 / R6.2 / R6.3 / R6.4 / R11.2）。英雄死亡不删实体、不判负。

#### Scenario: 复活恢复本体而非新实体

- **WHEN** 等级 L、经验 X、持 6 格背包的英雄阵亡后，其祭坛复活队列项推进到 `remaining_frames == 0`
- **THEN** 复活后实体 `EntityId` 与阵亡前相同、`hero.level == L`、`hero.xp == X`、背包与技能等级逐项一致、`alive == true`、位置为祭坛，且无新增 `EntityId`（R6.1 / R6.2 / R6.3 / C7 / C14 / C18）

#### Scenario: 英雄死亡不结束对局

- **WHEN** 一方英雄阵亡但该玩家仍有 `player.active`
- **THEN** `resolve_victory` 不产生胜负结果，英雄可经祭坛排队复活，复活后逐帧 `compute_state_hash()` 与同脚本基线一致（R11.2 / ADR 0002）

### Requirement: 复活费用与时间读冻结合成常量

复活费用（gold / lumber）与复活时间（整数帧）SHALL 读冻结的**合成 / 原创编译期整数常量**、随英雄等级按「基数 + 等级系数 → 绝对上限」表驱动计算，MUST 与祭坛训练 / 酒馆买活参数分离；MUST NOT 在逻辑里写死数值、MUST NOT 写入任何 `[待核]` 数值进权威定义冒充原版、MUST NOT 含 `f64` 或读取导出资产 / `config.toml`（R6.1 / R6.3 / R9.3 / §14 / ADR 0001 / ADR 0004）。费用提交事务与效果执行链 MUST 分离（主 `openspec/specs/gameplay-systems/spec.md:19`）。

#### Scenario: 干净环境构造且零资产

- **WHEN** 在不含 `config.toml` 与任何导出目录的干净环境构造含复活常量的冻结定义
- **THEN** 构造成功、不做任何文件 IO、每个字段为编译期整数常量，不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名或 rawcode，不含 `f64`（R2 / ADR 0001 / ADR 0004 / §14）

#### Scenario: 费用随等级确定且进定义哈希

- **WHEN** 分别以等级 1 与等级 L 的英雄提交复活、并对比前后 `FrozenDefinitions` 的 `content_hash()`
- **THEN** 费用 / 时间按冻结常量确定变化、经 `compute_defs_hash` 进入 `defs_hash` / `content_hash` 快照头部，扣费金额确定、逐帧哈希与同脚本基线一致（R6.3 / ADR 0002 / C15）

### Requirement: 复活费用事务与取消退款

提交复活命令时 SHALL 先校验 gold / lumber / food 充足再冻结扣费登记，MUST 从 `PlayerEconomy.gold / lumber` 确定性扣减（R3.1）；取消队列 / 英雄再次死亡 / 祭坛被毁 / 对局失败时 MUST 事务性退款并恢复死亡登记，MUST NOT 静默丢队列、MUST NOT 重复扣费、已提交的效果不回滚（R6.3 / 主 `openspec/specs/gameplay-systems/spec.md:19,23-26` / ADR 0002）。退款 MUST 与扣费同样进入状态哈希 / 快照。

#### Scenario: 取消复活事务性退款

- **WHEN** 玩家取消一个已扣费的祭坛复活队列项，或该队列项因祭坛被毁 / 对局失败而中止
- **THEN** `gold / lumber` 精确回退到扣费前值、死亡登记恢复到可再次复活状态、不产生半复活实体，逐帧 `compute_state_hash()` 与相同脚本基线一致（R6.3 / ADR 0002 / C6 / C18）

#### Scenario: 资源不足显式失败

- **WHEN** 以不足的 gold / lumber 提交复活命令
- **THEN** 命令被显式拒绝、不生成队列项、不扣费、不产生权威状态漂移，且 MUST NOT 静默降级为免费复活（显式失败不静默降级 / ADR 0002 / R3.1）

### Requirement: 复活命令与事件经确定性命令面

复活 / 取消复活 SHALL 经 `w3-core` 公开提交入口按冻结整数帧号入队、同源校验、按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；MUST NOT 旁路写入权威状态、MUST NOT 与墙钟调用粒度绑定（ADR 0002 §2-3 / R12.7 / R12.4）。按 Q2 拍板新增独立 `ReviveHero` / `CancelRevive` `GameplayCommandKind` 变体 + `HeroDied` / `Revived` 事实事件变体（不复用 `Train` + 扩展 `ProductionItem` 的被否备选），MUST 同步 `hash_gameplay_command` 编码 / 校验 / 穷尽分发，并扩展死亡 / 复活事实事件供只读表现层消费（ADR 0002 §5）。新增命令牵动 ADR 0005 protobuf 的字段与版本约束见下 Scenario（ADR 0005 / C25）。

#### Scenario: 帧冻结命令两端一致

- **WHEN** 一端逐帧推进、另一端一次推进 8 帧、两端共推进相同整数帧数并注入同一复活命令脚本
- **THEN** 两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条一致，命令按稳定排序键执行、重放不双重注入（R12.4 / R12.7 / ADR 0002）

#### Scenario: 新命令变体编码与校验同步

- **WHEN** 新增 `ReviveHero` / `CancelRevive` 命令变体与 `HeroDied` / `Revived` 事件变体并接入 protobuf 线格式
- **THEN** `hash_gameplay_command` 编码、命令校验与穷尽分发均覆盖新变体、protobuf 字段编号只追加且保留 `reserved 14 to 32`、`PROTOCOL_VERSION` 同步 bump、旧版本 `Reject` 明确拒绝、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、`net-owner` 按 `CODEBUDDY.md §9` 串行协作、CI 双平台生成代码字节一致，双客户端逐帧哈希一致（ADR 0005 / `docs/decisions/0005-protobuf-payload.md:34-44,84` / `UPSTREAM.md:88` / C3 / C25）

### Requirement: 表现层只读派生与零资产边界

祭坛复活队列进度 / 费用 / 剩余帧的 HUD 呈现 SHALL 只从只读 `WorldView`、事实事件与只读 `Snapshot` 派生，MUST NOT 反写 `RtsMatch` 权威状态、MUST NOT 进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / R10.5 边界 / C9）。只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单。UI 文案走 i18n、不进日志 / 权威状态（`CODEBUDDY.md §3`）；复活数值全为合成原创 `[待核]`、零资产入库、不引入参考项目代码或数值（ADR 0001 / §14）。

#### Scenario: HUD 只读不改哈希

- **WHEN** 复活队列进度随逻辑帧变化并连续驱动 HUD 重绘
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与无 HUD 实例一致，HUD 无 `RtsMatch` 字段写入路径（ADR 0002 §5 / C9）

#### Scenario: 零资产与许可证边界

- **WHEN** 校验本 change 的全部改动与新增常量
- **THEN** 仓库零暴雪资产 / MPQ 派生 / 真实导出 JSON，`w3-content` 原始 `f64` 坐标不进哈希 / 快照，`WarsmashModEngine`（AGPLv3）仅借鉴语义无代码复制，`rts_kernel_cs` 的 `double` 路线不采用（ADR 0001 / ADR 0004 / C20 / C22）

### Requirement: determinism 验收与反向验证

改动 `w3-core` 权威状态 / 哈希编码 / 快照字段 SHALL 通过 `cargo test -p w3-core --test determinism`（26 项）全绿（ADR 0002 §8 / `CODEBUDDY.md §5`）；新增 / 修改的复活回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照），仅「全绿」不构成验证（`CODEBUDDY.md §5`）。golden hash 基线 SHALL 按新实现如实更新、MUST NOT 冒充逐字节不变；`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），新增字段 MUST 完整进哈希 / 快照 / `first_difference()`（R12.4 / ADR 0002 / C1）。

#### Scenario: determinism 26 项全绿与改坏验红

- **WHEN** 跑 `cargo test -p w3-core --test determinism`（26 项），并以退化实现（复活 `spawn` 新实体 / 死亡登记用墙钟 / 新增字段漏接哈希或快照 / 扣费不减 gold）替换正式实现
- **THEN** 正式态 26 项全绿、缺陷态对应断言失败且修复后通过，两条证据可复现（`CODEBUDDY.md §5` / C27）

#### Scenario: golden hash 基线如实更新

- **WHEN** 以新增权威状态后的实现重取 golden hash 逐帧基线并与旧基线比对
- **THEN** 基线按新实现如实更新、不沿用旧值、不声称逐字节不变；`SNAPSHOT_VERSION` 按 Q1 拍板 bump 4 且与 `SIM_VERSION` / 快照头部 / 单测硬断言 / ADR 0002 复核记录同步一致（R12.4 / ADR 0002 / C10）
