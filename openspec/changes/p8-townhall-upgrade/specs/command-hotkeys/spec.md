# Spec Delta

## MODIFIED Requirements

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

### Requirement: 确定性契约零冲击与内核零 diff

本 requirement 标题沿用 `command-hotkeys` P6 历史契约名、字面作归档合并身份键（不代表行为断言，详见本 requirement 下 `>` 身份键说明）。本 capability 的键位 / 命令卡 / 子菜单 / 编队 / 选中集合 / 队列历史行为 SHALL 只经 `w3-core` 既有公开 API 与命令面实现、全部为表现层只读派生、MUST NOT 进权威状态 / 状态哈希 / 快照（R13.6 / ADR 0002 §5）。但 P8.5b 主城升级 change 按用户 2026-11-20 拍板方向 A **MUST 按 ADR 0002 全流程改 `w3-core`**：新增「主城等级 / tier」顶层权威状态 / 升级队列 / 费用登记同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝）；`GameplayCommandKind` 按 Q2 推荐新增独立 `UpgradeTownHall` 变体 + 升级事件变体；determinism 26 项全绿 +「改坏验红」、golden hash 基线按新实现如实更新、不沿用 P8.5a v4 值；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「`crates/w3-core/` MUST 零改动 / `SNAPSHOT_VERSION` MUST 仍为 3 / golden hash MUST 逐字节不变 / MUST NOT 新增权威状态字段 / 变体集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5` / `decision-memo.md` C9）。本 capability 的表现层只读派生正向语义不变：HUD 升级进度 / 命令卡状态不进哈希 / 快照。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。历史标题「内核零 diff」「golden hash 不变」字面保留，其口径已在 THEN 内按用户 2026-11-20 拍板修订。

#### Scenario: 内核零 diff（ADR 0002 / R13.6）

- **WHEN** 校验本 capability 的全部改动，以及 P8.5b 主城升级 change 对 `crates/w3-core/` 的全部改动
- **THEN** 键位 / 命令卡 / 子菜单 / 编队 / 选中集合 / 队列仍全为表现层只读派生、不进权威状态 / 哈希 / 快照（R13.6 / ADR 0002 §5）；但 P8.5b 按用户拍板改 `w3-core` 的 `git diff -- crates/w3-core/` **非空属预期**，新增 tier 顶层权威状态 / 升级队列 / 费用登记 MUST 同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」四类退化（Q8），MUST NOT 沿用「`git diff -- crates/w3-core/` 为空 / 既有权威状态字段集合不变 / 仍为 3 或 4 / 逐字节不变」旧断言（R12.4 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 状态哈希 / 快照 / golden hash 不变（ADR 0002 / R13.6）

- **WHEN** 在一局中连续切换命令卡 / 子菜单 / 编队 / `Shift` 队列并下达经典键命令后重跑 determinism，并与 P8.5b 新实现基线比对
- **THEN** 表现层操作本身不改变逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节、命令 / 事件排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` 不变、HUD 升级格位只读派生不进哈希 / 快照（R13.6 / ADR 0002 §5）；但 P8.5b 新增 tier 顶层权威状态后 golden hash 基线 MUST 按新实现如实更新、不沿用 P8.5a v4 值、不声称逐字节不变，`SNAPSHOT_VERSION` 按 Q1 推荐 bump 4→5（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v4 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝），状态哈希字段集合与快照头部布局按 ADR 0002 §6-8 纳入新字段，MUST NOT 沿用「golden hash 逐字节不变 / `SNAPSHOT_VERSION` 仍 3 / 字段集合不变」旧断言（R12.4 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）
