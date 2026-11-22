# rts-command-hud Specification

## Purpose
定义可玩 RTS 的命令界面契约：命令卡、选择面板、小地图、可见的生产队列与编队/群体输入，让玩家能看懂并按得动一局 Melee，且所有呈现只读派生、不触碰权威状态。

> 相关 ADR：ADR 0001（UI 字体 / 图标非 MPQ 派生、不入库）；ADR 0002（HUD / 选中集合 / 小地图点击为表现层状态，不进哈希 / 快照，输入只经公开提交入口）；ADR 0004（屏幕坐标经冻结边界）；ADR 0005（本地呈现管线接缝不变）。

## Requirements

### Requirement: 命令卡与选择面板

HUD SHALL 显示当前选中单位或建筑的可执行命令卡（移动、攻击、停止、采集、训练、建造、取消）与主选择面板（类型、生命、归属、资源），且随选中集合变化刷新；全部数据 MUST 只读派生。

#### Scenario: 选中农民

- **WHEN** 玩家选中一个或多个农民
- **THEN** 命令卡显示采集与建造等可用动作，选择面板显示其类型与生命

#### Scenario: 选中生产者

- **WHEN** 玩家选中主城或兵营
- **THEN** 命令卡显示可训练的单位与建筑动作

#### Scenario: 空选中

- **WHEN** 选中集合为空
- **THEN** 不显示单位命令卡，且不生成任何权威命令

### Requirement: 生产队列可见

HUD SHALL 为每个选中的生产者显示生产队列条目、单位类型与剩余进度；队列条目完成或取消后 MUST 立即刷新，MUST NOT 出现「队列生效但画面不可见」的呈现。

#### Scenario: 训练中队列可见

- **WHEN** 对生产者下达训练命令
- **THEN** HUD 出现队列条目且剩余帧数随逻辑帧递减，进度可观察

#### Scenario: 完成与取消刷新

- **WHEN** 队列条目完成或被取消
- **THEN** 对应条目从 HUD 消失，完成后新实体在对局画面中出现

### Requirement: 小地图

HUD SHALL 呈现只读小地图，显示地形轮廓、己方/敌方单位、建筑、资源点位置与相机视口框；点击小地图 SHALL 只平移表现层相机，MUST NOT 生成权威命令。本 change P8.5e 承接 `fog-of-war` capability：`derive_minimap` MUST 在把单位点喂入 `MinimapModel.points` **之前**按三态迷雾网格完成可见性过滤，把「在场性」（`EntityView.alive`）与「可见性」（迷雾派生）分两层（R10.1 / R10.5，参考对照结论 R2），只有 VISIBLE 格的敌方单位点 SHALL 呈现、MASKED / FOGGED 格敌方单位点 MUST NOT 呈现；野怪 / 商店 / 地面物品的位置 MUST 从只读 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（P8.5c / P8.5d 交接，R8.1 / R9.1）。过滤全为表现层只读派生，MUST NOT 反写 `RtsMatch` / 进状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5）；三态网格 / 已探索层 / 当前可见为表现层本地状态，`SNAPSHOT_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变、`w3-core` 零权威状态改动（R10.5 / ADR 0002 §6-8 / Q1 / Q4）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。本 delta 新增一条 `#### Scenario:`（喂入前可见性过滤），历史两条逐字保留。

#### Scenario: 小地图显示实体

- **WHEN** 一局进行中且存在多个实体
- **THEN** 小地图按队伍色标出实体位置并绘制当前相机视口框；P8.5e 起该显示 MUST 先经迷雾可见性过滤，MASKED / FOGGED 格敌方单位点不呈现、只有 VISIBLE 格敌方单位点进入 `MinimapModel.points`（R10.1 / R10.5 / `fog-of-war/spec.md`「小地图单位点可见性过滤」），己方单位按本地视角呈现、底图 / 视口框独立于迷雾遮罩，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节不变（ADR 0002 §5）

#### Scenario: 点击小地图平移

- **WHEN** 玩家点击小地图某处
- **THEN** 相机平移到该处，逐帧状态哈希与快照字节不变；点击小地图 MUST NOT 生成 `Move` / 任何权威命令，迷雾过滤不改变该只读语义（R10.1 / R10.5 / ADR 0002 §5 / R1.1 / R1.4）

#### Scenario: 单位点喂入前按迷雾可见性过滤（R10.1 / R10.5 / P8.5c / P8.5d）

- **WHEN** 一局中同时存在 VISIBLE / FOGGED / MASKED 格内的己方、敌方单位、野怪、中立商店与掉落地面物品并刷新小地图
- **THEN** `derive_minimap` 在喂入 `MinimapModel.points` 前完成可见性过滤（R10.1 / R10.5 / `hud.rs:653-724,791-808`），野怪 / 商店 / 地面物品位置经只读 `Snapshot.creep_camps` / `Snapshot.item_shop` 查表（`creep.rs:34-38` 地面物品不在 `read_view()` 实体集合，R8.1 / R9.1 / P8.5c / P8.5d 交接）；「不做过滤、直接 `view.entities.iter().filter(alive)` 全量喂入」或「把迷雾网格塞进 `Snapshot`」的退化实现令隔离回归断言失败（Q9 形态① / R10.5 / ADR 0002 §5）

### Requirement: 编队与群体命令

输入层 SHALL 支持编队（编组与按组选择）与群体命令（多选后单次移动或攻击按稳定实体 ID 分配落点）；选中集合 MUST 保持为表现层状态，MUST NOT 生成权威命令或进入状态哈希。

#### Scenario: 编组选择

- **WHEN** 玩家把当前选中集合编组后再按组号选择
- **THEN** 恢复该组的表现层选中集合，权威状态哈希不变

#### Scenario: 群体移动

- **WHEN** 多选单位后对地面下达一次移动命令
- **THEN** 生成带稳定分组信息的 `Move` 命令，落点按稳定实体 ID 分配

### Requirement: UI 文案 i18n 与可见性

UI 文案 SHALL 走 i18n 层，MUST NOT 进入日志、权威状态或状态哈希；HUD 所有文字层 SHALL 以已启用的开源内置字体正常显示；MUST NOT 使用 MPQ 派生字体或向仓库提交字体文件。中文上屏（CJK 字形）SHALL 留后续独立 change（涉及新增字体依赖与许可证人工拍板）。

#### Scenario: 文字层可见

- **WHEN** 在有显示设备的桌面运行
- **THEN** 命令卡、选择面板、资源与队列文字均可读，无文字层整体缺失的降级

#### Scenario: 文案不进日志与哈希

- **WHEN** HUD 刷新中文或 ASCII 文案
- **THEN** 文案不写入任何 `log` 记录，`compute_state_hash()` 与快照字节不变

### Requirement: HUD 只读派生与不改确定性

HUD 所有数据 SHALL 只从只读 `WorldView`、事实事件与只读 `Snapshot` 派生；MUST NOT 反写模拟状态；`w3-core` MUST 零改动、`SNAPSHOT_VERSION` MUST 仍为 3，`crates/w3-render/tests/isolation.rs` MUST 全绿。

#### Scenario: HUD 刷新不改哈希

- **WHEN** 选中、相机、命令卡或小地图随交互连续变化
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节与无 HUD 实例一致

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的全部改动
- **THEN** `git diff -- crates/w3-core/` 为空，既有权威状态字段集合不变

### Requirement: 经典键位与命令卡 12 格 / 两级 chords

HUD SHALL 显示 4 列 × 3 行 = 12 格命令卡，随选中集合类型（单位 / 建筑 / 英雄 / 商店）切换，由编译期整数 `DefId` 组织的表现层只读定义表按 `slot = y * 4 + x` 落格（键位与命令解耦、MUST NOT 用建筑名字符串白名单，`R13.1` / `R13.3` / `R1.2`）。经典键 `M` / `A` / `S` / `G` / `B` / `T` / `Esc` 与命令卡点击 SHALL 汇入同一输入翻译链生成既有命令（`R13.2` / `R13.6`）。两级 chords（`B` 开建造子菜单、第二键选建筑；生产建筑训练键）与子菜单栈 / 放置态 SHALL 支持且为表现层局部只读状态；`Esc` SHALL 取消子菜单 / 放置态、不生成命令（`R13.3`）。以上数据全部只读派生，MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照（ADR 0002）。具体键位字母 `[待核]`、不冻结进权威定义、不入库（ADR 0001 / ADR 0004）。

#### Scenario: 命令卡 12 格随选中类型切换（R13.1）

- **WHEN** 玩家由选中农民改选生产建筑（兵营 / 主城）
- **THEN** 命令卡由定义表重建为 12 格、内容随选中类型变化，格位按 `slot = y * 4 + x` 落格、键位与命令解耦；`compute_state_hash()` 与 `capture_snapshot()` 字节不变（ADR 0002）

#### Scenario: 点击命令卡与按经典键等效（R13.1 / R13.2 / R13.6）

- **WHEN** 对同一选中集合分别点击命令卡某格与按该格绑定的经典键
- **THEN** 二者经同一输入翻译链产出逐字段相同的既有 `ClientCommand`、不携带命令来源标记，命令卡刷新不改逐帧状态哈希（ADR 0002 / ADR 0005）

#### Scenario: 两级 chords 与子菜单只读态（R13.3）

- **WHEN** 选中可建造农民按 `B` 打开建造子菜单、再按第二键选建筑进入放置态（合成键位配置，具体字母 `[待核]`）
- **THEN** 命令卡切为建筑格、由定义表第二键产出既有建造 / 训练命令；子菜单栈 / 放置态为表现层局部状态，换卡清空重建，不进权威状态 / 哈希 / 快照（ADR 0002）

#### Scenario: Esc 取消子菜单不生成命令（R13.3 / R13.6）

- **WHEN** 处于建造子菜单或放置态按 `Esc`
- **THEN** 子菜单栈清空、命令卡回主卡，不生成任何命令，逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节不变、`SNAPSHOT_VERSION` 仍 3（ADR 0002）

#### Scenario: 具体键位不进权威定义（R13.1 / ADR 0001）

- **WHEN** 校验经典键位字母（尤其英雄技能、各族建造 / 训练第二键）与 grid / `CustomKeys` 支持程度
- **THEN** 一律标注 `[待核 / 待用户拍板]`、以本机导出 SLK 命令卡热键字段或原版客户端核对，不冻结进 `FrozenDefinitions`、不入库、不进状态哈希 / 快照（ADR 0001 / ADR 0004）

### Requirement: 命令卡按本地族查表分派训练 / 建造目标

HUD 的 Producer / Build / Train 命令卡 SHALL 把 `TrainUnit { unit_def, .. }` 的目标改为**按本地族只读查表**分派：本地人族卡产 `1xx` 段、本地兽族卡产 `2xx` 段目标（R2 / R4 / R13.1 / R13.3）。分派 MUST 由编译期整数 `DefId` 定义表驱动，MUST NOT 散落 `if race` / 在 Present 层判玩法规则 / 用建筑名或 id 前缀推断（R1.2 / 主 `rts-command-hud/spec.md:99-101`）；未知族 / 缺表项 SHALL 明确默认回落且可诊断（C8 / C9）。`CommandCatalog` MUST 只读派生、MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照（ADR 0002）。

#### Scenario: 兽族建筑点训练产兽族目标（R2 / R4 / R13.3）

- **WHEN** 以本地兽族装配并选中兽族兵营 / 大厅点击训练格
- **THEN** 生成的 `TrainUnit { unit_def, .. }` 落在 `2xx` 段（如 `ORC_SOLDIER` / `ORC_RANGED` / `ORC_SIEGE` / `ORC_HERO` 对应新类别），MUST NOT 出现 `HUMAN_*` 目标；本地人族装配时仍产 `1xx` 段、既有行为逐字节不变（C8 / R2）

#### Scenario: 查表分派不散落 if race、不白名单（R1.2 / R13.1）

- **WHEN** 审阅 Producer / Build / Train 卡构造路径（现状 `crates/w3-client/src/melee/command_card.rs:235-360` 全写死 `HUMAN_*`）
- **THEN** 目标 `DefId` 来自按本地族只读查表；无散落 `if race` 分支、无「建筑名字符串 / `H`/`O` id 前缀」白名单、不在 Present 层判玩法规则；12 格按 `slot = y * 4 + x` 落格且键位与命令解耦的既有契约不改写（godot `BUILD_SYSTEM.md:298/:348-349` / C5 / C8）

#### Scenario: 未知族 / 缺表项明确默认回落（C8 / C9）

- **WHEN** 构造未知族或查表键缺失的命令卡
- **THEN** 回落到明确默认（如人族段）并以英文结构化 `key=value` 日志可诊断，不静默产出错误族目标、不写死中文名 / 图标表（godot `command_card.gd:85-99/:441-445/:863-893` / `CODEBUDDY.md §3`）

#### Scenario: 命令卡按族刷新不改哈希与快照（ADR 0002 / R13.6）

- **WHEN** 切换本地族、连续刷新命令卡 / 选中集合并推进若干整数逻辑帧
- **THEN** `compute_state_hash()` 与 `capture_snapshot()` 字节与无 HUD 实例一致、`SNAPSHOT_VERSION` 仍 3、`git diff -- crates/w3-core/` 为空；`CommandCatalog` / `def_kinds` 只读派生不反写权威状态（主 `rts-command-hud/spec.md:87` / C1 / C3）

#### Scenario: 具体键位 / 图标语义仍 `[待核]`（R13.1 / ADR 0001 / §14）

- **WHEN** 校验命令卡各族建造 / 训练第二键与图标语义
- **THEN** **Q2 用户 2026-11-18 已拍板按推荐**：一律标注 `[待核]`、以合成原创编译期定义表表达，不冻结进 `FrozenDefinitions`、不入库、不冒充经典原版；`def_kinds` 表补齐两族 DefId 登记但仍只读（主 `rts-command-hud/spec.md:123-126` / C10 / `decision-memo.md` 拍板决策 P2）

#### Scenario: 本 Requirement 只追加、不是重做 R13 命令卡（C5 / C8）

- **WHEN** 核对本 delta 与主 `openspec/specs/rts-command-hud/spec.md` 既有 requirement 正文
- **THEN** 既有 12 格定义表按 `slot = y * 4 + x` 落格、键位与命令解耦、两级 chords、子菜单栈、`Esc` 取消不生成命令、小地图点击只平移相机、生产队列可见等契约原样保留、正文零改写；本 delta 只追加一条 ADDED Requirement（`rts-command-hud/spec.md:12/:31/:45/:54-55/:73/:87/:99-126` / `design.md` D2）

#### Scenario: 反向验证退化回写死 HUMAN_* 即失败（CODEBUDDY.md §5 / C11）

- **WHEN** 把命令卡分派退化为继续写死 `HUMAN_*` 并以本地兽族装配注入
- **THEN** 「兽族建筑训练产兽族目标」断言在缺陷态失败、正式实现通过，两条证据可复现；退化实现 MUST 真经 `CommandCatalog` 构造路径注入、非旁路模拟，用「测试内反例元断言」或 A-B 对照，仅「全绿」不算完成；既有 12 格 / 键位解耦 / 回落基线测试不静默放宽（§14）

#### Scenario: 确定性门禁全绿（ADR 0002 / C1）

- **WHEN** 运行命令卡按族分派 CPU 回归、重跑 `cargo test -p w3-core --test determinism`（26 项）与 `crates/w3-render/tests/isolation.rs`
- **THEN** 全部通过，逐帧状态哈希 / 快照 / golden hash 与基线一致、`SNAPSHOT_VERSION` 仍 3、`git diff -- crates/w3-core/` 为空（ADR 0002 / C11）
