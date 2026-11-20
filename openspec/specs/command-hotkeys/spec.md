# command-hotkeys Specification

## Purpose
定义经典 Melee 键盘 / 命令卡操作层的表现层只读派生契约：键位 → 既有命令、命令卡 12 格与两级 chords 子菜单、编队 / 子组 / `Shift` 队列如何经同一输入翻译链产出既有命令，且全部为表现层局部状态、不进权威状态 / 状态哈希 / 快照。让 S1 / S3 / S4 的联机对战具备经典操作层，同时守住 ADR 0002 确定性契约与 ADR 0001 零资产边界。

> 相关 ADR：ADR 0001（键位 / 命令卡定义表为合成原创编译期数据、零资产）；ADR 0002（键位 / 命令卡 / 子菜单 / 编队 / 选中集合 / 队列全部表现层只读派生，不进哈希 / 快照，`SNAPSHOT_VERSION` 仍 3）；ADR 0004（屏幕坐标经既有冻结边界）；ADR 0005（键盘命令经 lockstep 命令帧、回放逐帧哈希一致）。
> 需求源：`docs/references/WC3-MELEE-RULES.md` §13（`R13.1`–`R13.7`）、`R1.3` / `R1.4`、`R12.2` / `R12.4`；1.0 范围 §0.1 S1 / S3 / S4。具体键位字母 `[待核]`，不冻结进权威定义、不入库。

## Requirements

### Requirement: 命令卡 12 格由定义层数据派生并随选中类型切换

命令卡 SHALL 为 4 列 × 3 行 = 12 格动作按钮，由**编译期整数 `DefId` 组织的表现层只读定义表**填充：格位按 `slot = y * 4 + x` 落格，每格「按钮 → 命令 + 快捷键」为一条定义记录，键位与命令 MUST 解耦。命令卡内容 SHALL 随当前选中集合类型（单位 / 建筑 / 英雄 / 商店）切换，MUST NOT 用建筑名字符串硬编码白名单（`R13.1` / `R13.3` / `R1.2` / ADR 0001）。点击按钮与按对应经典键 SHALL 等效。全部为表现层只读派生，MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照（ADR 0002）。

#### Scenario: 选中农民与选中生产者的命令卡不同（R13.1）

- **WHEN** 玩家分别选中农民与生产建筑（兵营 / 主城）
- **THEN** 命令卡各呈现 12 格定义表条目且内容不同：农民卡含采集 / 建造入口，生产者卡含训练 / 取消；格位按定义表的 `slot = y * 4 + x` 落格，不以类型名白名单推导（R1.2）

#### Scenario: 键位与命令解耦（R13.1）

- **WHEN** 定义表把同一命令配到不同格位或改其快捷键字母（合成 / 原创配置，具体字母 `[待核]`）
- **THEN** 该格生成的既有命令不变、仅键位字母变化，命令卡定义表是单一真值源；切换不改 `compute_state_hash()` 与 `capture_snapshot()` 字节（ADR 0002）

#### Scenario: 空选中不显示命令卡且不生成命令（R13.1）

- **WHEN** 选中集合为空
- **THEN** 不显示单位命令卡，且不生成任何权威命令、逐帧状态哈希不变（ADR 0002）

### Requirement: 经典键位映射汇入同一输入翻译链生成既有命令

经典通用命令键 `M` / `A` / `S` / `G` / `B` / `T` / `Esc` 与 `U`（`U` 主基地升级 / 商店选择使用者，R13.2）SHALL 经**同一输入翻译链**翻译为 `w3-core` 既有命令面：键盘与命令卡点击 MUST 汇入同一处、MUST NOT 新增「命令来源标记」进命令面（`R13.1` / `R13.2` / `R13.6` / ADR 0002 / ADR 0005）；具体键位 `[待核]`。P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A 在 `w3-core` 确定性命令面新增独立 `UpgradeTownHall` 变体（Q2 推荐）：`U` 键与命令卡升级格位 SHALL 由定义层数据驱动、只生成该既有命令面命令、MUST NOT 反写权威状态或另立表现层假升级（R4.1 / R13.3 / ADR 0002 §5）；本 capability 的键位映射本身 MUST NOT 新增变体，新变体属 `sim-owner` 按 ADR 0002 流程承接（连带 `hash_gameplay_command` 追加码 13 / 校验 / 穷尽分发 / ADR 0005 `PROTOCOL_VERSION` bump 与 `Reject` 旧版本）。历史「MUST NOT 新增 `CommandKind` / `GameplayCommandKind` 变体」是 `command-hotkeys` change 的实施期约束、不代表禁止后续 change 按用户拍板改内核（R13.6 范围限定 / `decision-memo.md` C9）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 经典键逐键生成既有命令（R13.2）

- **WHEN** 在有选中集合与合法目标 / 地面点下依次触发 `M` / `A` / `S` / `G` / `B` / `T` / `Esc`（合成键位配置，具体字母 `[待核]`）
- **THEN** 各键经同一 `apply` 产出既有命令面变体（或 `B` 开子菜单、`Esc` 取消且不生成命令），键位映射路径不出现任何新命令变体、命令字节与既有命令面一致；`U` 键升级格位由定义层数据驱动、只经 `w3-core` 既有命令面提交 `UpgradeTownHall`、不另立假升级（R13.2 / R13.6 / R4.1 / ADR 0002）

#### Scenario: 键盘与命令卡点击等价（R13.1 / R13.6）

- **WHEN** 对同一选中集合分别「点击命令卡升级格位」与「按该格绑定的经典 `U` 键」
- **THEN** 两者经同一 `apply` 产出逐字段相同的 `ClientCommand`、不携带命令来源标记；`compute_state_hash()` 与 `capture_snapshot()` 字节不因输入来源不同而变化（ADR 0002 / R13.1 / R13.6）

#### Scenario: A 与 S 的语义边界（R13.2）

- **WHEN** 按 `A` 指定敌方目标、按 `S` 停止
- **THEN** `A` 只生成既有 `Attack { target }`、`S` 只生成既有 `Stop`；`S` MUST NOT 合并 Hold 语义、`A` MUST NOT 化为 attack-move 点地面途中交战（该语义 `[待核]`、由后续 change 承接，R13.2）

### Requirement: 两级 chords 与子菜单栈为表现层局部状态

选中可建造 / 生产实体 SHALL 支持两级 chords（`B` 开建造子菜单、再按第二键选建筑进入放置态后左键定点；生产建筑按训练键训练单位）。子菜单栈 / 放置态 / 键→action 表 MUST 是表现层局部只读状态，换卡时清空重建，MUST NOT 进权威状态 / 状态哈希 / 快照（`R13.3` / `R13.6` / ADR 0002）。第二键列表 SHALL 由定义层数据驱动、MUST NOT 用建筑名字符串白名单（R1.2）。

#### Scenario: B→A 建造祭坛与兵营 F 训练步兵（R13.3）

- **WHEN** 选中可建造农民依次按 `B` 与第二键（建造祭坛的合成键位），以及选中兵营按训练键（训练步兵的合成键位）
- **THEN** 命令卡切到建筑格后由定义表第二键产出既有 `GameplayCommandKind::Train { is_building: true }`（祭坛）与 `Train { is_building: false }`（步兵）；第二键来自定义表记录而非建筑名白名单（R1.2）；放置态是表现层局部状态

#### Scenario: Esc 取消子菜单 / 放置态不生成命令（R13.3）

- **WHEN** 处于建造子菜单或放置态时按 `Esc`
- **THEN** 子菜单栈清空、命令卡回主卡，不生成任何命令，`compute_state_hash()` 与 `capture_snapshot()` 字节不变（R13.6 / ADR 0002）

#### Scenario: 子菜单栈只读派生（R13.3 / R13.6）

- **WHEN** 反复进入 / 退出建造与训练子菜单
- **THEN** 键→action 表每次随当前卡重建、子菜单栈只存在于表现层，逐帧状态哈希与快照字节与无子菜单操作一致，`SNAPSHOT_VERSION` 仍 3（ADR 0002）

### Requirement: 同一字母的含义由当前卡与选中集合决定

同一字母在不同上下文（如 `G` 对农民 / 攻城单位 / 商店，`B` 对农民 / 生产者）SHALL 由**当前选中集合 + 当前命令卡状态**决定，MUST NOT 用全局静态键位映射（`R13.2` / `R13.6`）。当前卡重建时键→action 表 SHALL 同步重建（R13.3）。

#### Scenario: 同一字母随卡切换含义（R13.2）

- **WHEN** 分别在农民主卡、建造子卡、商店卡下按下同一字母（合成键位配置）
- **THEN** 该字母按当前卡重建的键→action 表翻译为不同既有动作，点击该格与按键产出同一命令；切卡后旧卡的键绑定不再生效（R13.3）

### Requirement: 编队 / 子组 / Shift 队列为表现层只读状态

输入层 SHALL 支持 `Ctrl`+数字编组、数字按组选择、`Tab` 混编同类型子组切换与 `Shift`+命令排队；编队 / 选中集合 / 子组游标 / 队列 MUST 全部为表现层只读状态，MUST NOT 进权威状态 / 状态哈希 / 快照（`R13.4` / ADR 0002）。`Shift` 队列命令 SHALL 逐条经既有 `CommandKind::Move { append: true }` 命令面入队，按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序，MUST NOT 旁路写入或改变排序键（R13.4 / ADR 0002）。

#### Scenario: 编组与 Tab 子组（R13.4）

- **WHEN** 玩家 `Ctrl`+数字编组、数字按组选择、在混编选中集合按 `Tab` 切子组
- **THEN** 恢复 / 切换表现层选中集合，权威状态哈希与快照字节不变；子组集合允许含建筑但命令分发仍按 `R1.4` 过滤（R1.4）

#### Scenario: Shift 队列经既有 append 命令面（R13.4）

- **WHEN** 玩家 `Shift`+连续给出多个移动落点
- **THEN** 逐条生成既有 `Move { append: true }` 命令、当前命令完成后依次执行；命令按稳定排序键入队、无旁路写入，golden hash 口径不变（R13.4 / ADR 0002）

### Requirement: 键盘 M / A 路径复用 R1.4 分层可移动性过滤

键盘 `M` / `A` 生成 `Move` / `Attack` 命令时 SHALL 与 `R1.3` 右键路径走同一输入翻译与命令生成处，MUST 由 `input.rs::effective_movability` 分层判据过滤选中集合：frame-0 装配实体用装配期 `UnitDef.speed_raw` 派生的只读映射（权威），运行时动态实体用只读 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动，两条来源 MUST NOT 互相替代。金矿 / 树木 / 主城 / 建筑等不可移动实体 MUST NOT 生成 `Move`、MUST NOT 改变位置；落点按稳定 `EntityId` 顺序、过滤后集合与「只含可移动单位」逐条相同（`R1.4` / `R13.6` / ADR 0002）。MUST NOT 扩展视图、用运行时 `speed` 替代装配映射、或回退建筑类型白名单。

#### Scenario: 混合选中集合键盘 M 只为可移动单位生成 Move（R1.4 / R13.6）

- **WHEN** 框选集合同时含农民、主城与金矿，按 `M` 并指定地面落点
- **THEN** 只为农民生成 `Move`；主城（建筑）与金矿（资源点）MUST NOT 出现在 `Move` 生成集合中、位置不变；落点与「只选中农民」的等效脚本逐条相同、按稳定 `EntityId` 顺序，逐帧状态哈希与快照字节一致（R1.4）

#### Scenario: 运行时动态建筑键盘 M 不响应 Move（R1.4 / R13.6）

- **WHEN** 运行时 `Train` / `Build` 产出建筑（`EntityView.speed == 0`、不在装配期映射）与农民一起框选，按 `M` 指定地面落点
- **THEN** 该运行时建筑 MUST NOT 出现在 `Move` 生成集合中、位置不变；只有农民与产出的可移动单位（`speed > 0`）生成 `Move`（R13.6）

#### Scenario: 全为不可移动实体不生成 Move（R1.4 / R13.6）

- **WHEN** 选中集合只含建筑 / 资源点并按下 `M`
- **THEN** 不生成任何 `Move` 命令，权威状态与状态哈希不变，且有明确的「无可执行目标」反馈位（表现层只读、不进快照）

### Requirement: UI 文案走 i18n 且不进日志 / 哈希 / 快照

命令卡 / 键位提示 / 子菜单标签 SHALL 走 i18n 层，MUST NOT 进入 `log` 记录、权威状态、状态哈希或由 `SNAPSHOT_VERSION`（当前 3）定义的快照；文案切换 MUST NOT 改变逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节（`R13.6` / `CODEBUDDY.md` §3 / ADR 0001 / ADR 0002）。具体键位字母 `[待核]`、以本机导出 SLK 命令卡热键字段或原版客户端核对、不冻结进权威定义、不入库。

#### Scenario: 文案刷新不改哈希（R13.6 / CODEBUDDY.md §3）

- **WHEN** 命令卡随选中类型或子菜单切换刷新中文 / ASCII 文案
- **THEN** 文案不写入任何 `log` 记录，`compute_state_hash()` 与 `capture_snapshot()` 字节与无文案刷新实例一致（ADR 0002）

### Requirement: 确定性契约零冲击与内核零 diff

本 requirement 标题沿用 `command-hotkeys` P6 历史契约名、字面作归档合并身份键（不代表行为断言，详见本 requirement 下 `>` 身份键说明）。本 capability 的键位 / 命令卡 / 子菜单 / 编队 / 选中集合 / 队列历史行为 SHALL 只经 `w3-core` 既有公开 API 与命令面实现、全部为表现层只读派生、MUST NOT 进权威状态 / 状态哈希 / 快照（R13.6 / ADR 0002 §5）。但 P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A **MUST 按 ADR 0002 全流程改 `w3-core`**：新增「主城等级 / tier」顶层权威状态 / 升级队列 / 费用登记同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体 + 升级事件变体；determinism 26 项全绿 +「改坏验红」、golden hash 基线按新实现如实更新、不沿用 P8.5a v4 值；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「`crates/w3-core/` MUST 零改动 / `SNAPSHOT_VERSION` MUST 仍为 3 / golden hash MUST 逐字节不变 / MUST NOT 新增权威状态字段 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5` / `decision-memo.md` C9）。本 capability 的表现层只读派生正向语义不变：HUD 升级进度 / 命令卡状态不进哈希 / 快照。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。历史标题「内核零 diff」「golden hash 不变」字面保留，其口径已在 THEN 内按用户 2026-11-20 拍板修订。

#### Scenario: 内核零 diff（ADR 0002 / R13.6）

- **WHEN** 校验本 capability 的全部改动，以及 P8.5b 主城升级 change 对 `crates/w3-core/` 的全部改动
- **THEN** 键位 / 命令卡 / 子菜单 / 编队 / 选中集合 / 队列仍全为表现层只读派生、不进权威状态 / 哈希 / 快照（R13.6 / ADR 0002 §5）；但 P8.5b 按用户拍板改 `w3-core` 的 `git diff -- crates/w3-core/` **非空属预期**，新增 tier 顶层权威状态 / 升级队列 / 费用登记 MUST 同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」四类退化（Q8），MUST NOT 沿用「`git diff -- crates/w3-core/` 为空 / 既有权威状态字段集合不变 / 仍为 3 或 4 / 逐字节不变」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 状态哈希 / 快照 / golden hash 不变（ADR 0002 / R13.6）

- **WHEN** 在一局中连续切换命令卡 / 子菜单 / 编队 / `Shift` 队列并下达经典键命令后重跑 determinism，并与 P8.5b 新实现基线比对
- **THEN** 表现层操作本身不改变逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节、命令 / 事件排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变、HUD 升级格位只读派生不进哈希 / 快照（R13.6 / ADR 0002 §5）；但 P8.5b 新增 tier 顶层权威状态后 golden hash 基线 MUST 按新实现如实更新、不沿用 P8.5a v4 值、不声称逐字节不变，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），状态哈希字段集合与快照头部布局按 ADR 0002 §6-8 纳入新字段，MUST NOT 沿用「golden hash 逐字节不变 / `SNAPSHOT_VERSION` 仍 3 / 字段集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 联机命令帧与回放逐帧哈希一致

键盘产生的命令 SHALL 与鼠标命令同样经 lockstep 命令帧同步、只传输入命令不传状态；回放 MUST 能逐帧重放同一命令序列且哈希与原局一致（`R12.2` / `R12.4` / `R13.6` / ADR 0005）。键位 / 命令卡 / 子菜单 / 编队状态 MUST NOT 进入命令帧或快照。

#### Scenario: 键盘命令回放一致（R12.2 / R12.4 / R13.6）

- **WHEN** 联机对战录一局含经典键盘命令与两级 chords 命令，之后加载回放
- **THEN** 回放逐帧 `compute_state_hash()` 与原局一致，命令帧只含输入命令、不含任何表现层键位 / 子菜单 / 编队字段（ADR 0005）

### Requirement: 反向验证（改坏验红）

涉及键盘 `M` / `A` 可移动性过滤、子菜单 / 键位映射的新增 / 修改玩法回归 MUST 证明测试在**缺陷态**失败、修复态通过——仅「全绿」不构成验证。证据形态为测试内反例元断言或 A-B 对照（`CODEBUDDY.md` §5 / `openspec/config.yaml` `rules.tasks` 第三条 / `R13.6`）。

#### Scenario: 键盘路径过滤退化令混合集合断言失败（R1.4 / R13.6 / CODEBUDDY.md §5）

- **WHEN** 以「不过滤 / 把建筑当可移动」的退化键盘路径替换正式实现，对含建筑 / 金矿 / 主城的混合选中集合按 `M`
- **THEN** 「不可移动实体 MUST NOT 出现在 `Move` 生成集合」断言在缺陷态失败、正式实现下通过；元断言证明退化实现等价该缺陷（C5）

#### Scenario: 子菜单 / 键位映射退化令 chords 断言失败（R13.3 / CODEBUDDY.md §5）

- **WHEN** 把子菜单栈 / 键→action 表退化为全局静态映射或不清栈
- **THEN** `B`→`A` 两级 chords 与「同一字母随卡切换」断言在缺陷态失败、正式实现下通过（C5）

### Requirement: 新增命令变体由后续 change 承接（R13.6 范围限定）

`P` 巡逻 / `H` 保持位置 / `A` 点地面 attack-move 属**新增命令变体**（`command.rs::CommandKind` 无对应变体），本 capability MUST NOT 实现、MUST NOT 借「只读视图投影 / 实现层约束放松」旁路 ADR 0002 门禁，SHALL 登记为由后续独立 change 承接（新增权威状态 / 命令面须同步状态哈希、快照与 `first_difference()`、跑 determinism，`SNAPSHOT_VERSION` 处置另拍板）（`R13.2` / `R13.6`）。`A` 本 capability 只归既有 `Attack` 指定目标入口（attack-move `[待核]`），`S`=Stop 与 `H`=Hold MUST NOT 合并。

#### Scenario: 承接边界登记（R13.6）

- **WHEN** 校验本 capability 的命令面与 Scenario 集合
- **THEN** 不含 `P` / `H` / attack-move 对应新命令变体、S 与 H 不合并；这些登记在 `design.md` 的承接边界与 `decision-memo.md` 待确认项，由后续 change 按 ADR 0002 流程承接（C4）
