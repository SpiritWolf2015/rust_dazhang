# melee-match Specification

## Purpose
定义一局 1v1 Melee 对战的可玩会话契约：从开局布置、资源采集、建造训练、战斗到 AI 对手与胜负结算全程可在窗口内完成，把 `w3-core` 已交付的玩法命令串成「能打完一局」的闭环，并让对局实体可辨识。

> 相关 ADR：ADR 0001（合成定义零资产、导出数据不入库）；ADR 0002（内核零改动、状态哈希 / 快照 / `SNAPSHOT_VERSION` 不变、外部命令按帧冻结）；ADR 0003（第一版以 Melee 对战优先）；ADR 0004（冻结坐标与定点单位）。

## Requirements

### Requirement: 一局 Melee 对战可装配并打完

系统 SHALL 提供一局 1v1 Melee 对战的确定性会话：开局为双方布置主城、农民与资源点，玩家经窗口输入完成采集、建造、训练、进攻，直到分出胜负。对局的**权威逻辑闭环**（`FrozenDefinitions`、开局命令脚本、内核推进与胜负结算）MUST NOT 依赖任何本机导出资产或配置文件；真实地图表现层（地形 / 地表贴图 / 小地图底图）经 gitignored 的本机 `config.toml` 可选接入，属只读表现层接缝，MUST NOT 进入权威状态、状态哈希或快照（ADR 0001 / ADR 0002）。

#### Scenario: 开局布置

- **WHEN** 启动 `melee` 可玩入口（需 gitignored `config.toml` 指向本机 Echo Isles 导出）
- **THEN** 双方各有主城、农民与资源点，资源与人口有确定初值，且全部权威坐标由冻结整数常量构造

#### Scenario: 打完一局

- **WHEN** 玩家依次采集资源、建造兵营、训练战斗单位并摧毁敌方主城
- **THEN** 内核按既有结算阶段产生唯一胜负结果，HUD 显示胜负，且玩家可重开新一局

#### Scenario: CPU 对局装配零资产

- **WHEN** 在不含 `config.toml` 与任何导出目录的干净环境运行 CPU 对局装配（`MeleeDriver` / `melee_loop.rs`）或既有 `playable` 合成入口
- **THEN** 一局仍能启动并打完，权威逻辑不读取任何资产文件

#### Scenario: 真实视觉入口依赖本机导出

- **WHEN** 启动 `melee` 可玩 example，且已提供 gitignored 的 `config.toml` 指向本机 Echo Isles 导出
- **THEN** 真实地形 / 地表贴图 / 小地图底图作为只读表现层呈现，权威状态、状态哈希与快照不变
- **WHEN** 启动 `melee` 可玩 example 但 `config.toml` 缺失或导出数据无效
- **THEN** 显式失败、不回落合成色块 demo（与 `proposal.md` / `design.md` D7 一致）

> 该入口对 gitignored 本机 `config.toml` 的依赖属表现层接缝，MUST NOT 进入权威状态、状态哈希或快照（ADR 0001 / ADR 0002）。

### Requirement: 冻结内容定义零资产

对局 SHALL 以编译期整数常量构造的冻结定义启动，至少覆盖农民、主城、兵营、战斗单位、资源点、技能与升级；内容哈希 MUST 进入快照头部；**`FrozenDefinitions` 构造路径** MUST NOT 读取 `config.toml`、导出 JSON 或任何游戏资产（真实导出数据仅用于只读表现层，见「一局 Melee 对战可装配并打完」的边界澄清）。

#### Scenario: 定义冻结

- **WHEN** 一局开始
- **THEN** `FrozenDefinitions` 校验通过、内容哈希固定，对局中定义不被热更新

#### Scenario: 拒绝不匹配恢复

- **WHEN** 用不同内容哈希的定义恢复快照
- **THEN** 明确拒绝恢复，不产生半初始化的对局

### Requirement: 资源采集

系统 SHALL 允许玩家选中农民并指向资源点进行采集，采集命令经公开提交入口产生，资源随整数逻辑帧确定性增长，且 HUD 可见。

#### Scenario: 命令采集

- **WHEN** 选中农民后对资源点下达采集命令
- **THEN** 生成 `Gather` 玩法命令，资源数值随帧确定增长，选中集合本身不生成权威命令

#### Scenario: 采集不改哈希口径

- **WHEN** 采集推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与相同命令脚本的基线一致

### Requirement: 建造与训练

系统 SHALL 允许玩家用主城或兵营建造建筑、训练单位；生产队列的剩余帧数 MUST 随逻辑帧递减，完成后 MUST 生成实体并按定义扣增资源与人口，HUD MUST 显示队列进度。

#### Scenario: 训练完成生成实体

- **WHEN** 对生产者下达训练命令并推进至队列完成
- **THEN** 生成新单位实体、更新资源与人口，HUD 队列项消失且新实体可见

#### Scenario: 取消生产

- **WHEN** 玩家取消某个生产队列条目
- **THEN** 该条目不再推进、不生成实体，资源按既有规则处理

### Requirement: 确定性 AI 对手

系统 SHALL 为对手玩家提供确定性命令源，按冻结帧号经公开提交入口产生采集、训练与进攻命令；AI 命令 MUST NOT 覆盖仍有效的玩家命令，完整重放 MUST NOT 双重注入内部 AI 命令。

#### Scenario: 对手会行动

- **WHEN** 一局推进若干逻辑帧且未分胜负
- **THEN** 对手单位产生采集、训练或进攻行为，玩家可见其状态变化

#### Scenario: 重放不双重执行

- **WHEN** 重放只注入外部玩家命令并重新运行 AI 命令源
- **THEN** 逐帧状态哈希与原始对局一致

### Requirement: 胜负结算与重开

胜负 SHALL 在既有内核结算阶段按稳定顺序只结算一次；一方主城被毁或离开后 MUST 产生唯一胜负结果并在 HUD 显示；胜负后 MUST 能以相同冻结定义与开局脚本重新构造一局，不残留上一局权威状态。

#### Scenario: 摧毁主城结算

- **WHEN** 敌方主城被摧毁
- **THEN** 经既有 `Leave` / 胜负路径产生唯一胜者与结算帧，HUD 显示结果

#### Scenario: 同帧只结算一次

- **WHEN** 同一帧内多个胜负条件同时满足
- **THEN** 只产生一次结算与一组事件

#### Scenario: 重开

- **WHEN** 胜负已结算后以相同冻结定义与开局脚本重新构造对局
- **THEN** 新一局从初始权威状态开始，不残留上一局胜负 / 实体 / 资源状态

### Requirement: 对局实体可辨识呈现

系统 SHALL 按实体类型以不同几何、队伍色、选中环与血条呈现农民、战斗单位、建筑与资源点，使玩家可分辨类型与敌我；呈现 MUST NOT 反写权威状态、进入状态哈希或快照。本 change P8.5e 把该呈现明确纳入 `fog-of-war` capability 的可见性过滤：已方 / 敌方单位、建筑、资源点、野怪、中立商店与地面物品在 MASKED / FOGGED 格内的变化 SHALL 对本地视角不可见（R10.1），只有 VISIBLE 格的敌方实体变化进入 `Scene` 动态实例 / `MinimapModel.points` / HUD 只读投影（R10.5）。迷雾呈现 / 雾遮罩 MUST NOT 反写 `RtsMatch`、进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 7）定义的快照或 `first_difference()`（R10.5 / ADR 0002 §5 / §6-8）；`w3-core` 保持零权威状态改动、`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变（R10.5 / Q1 / Q4）。含地面物品 / 商店（中立建筑）/ 金矿 / 树木 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 类型与敌我可辨

- **WHEN** 观察含农民、战斗单位、建筑、资源点与双方单位的窗口，且同时存在 VISIBLE / FOGGED / MASKED 格
- **THEN** 各类实体外观可区分、敌我以队伍色区分、选中单位有选中环与血条；P8.5e 起敌方实体只在 VISIBLE 格呈现、MASKED / FOGGED 格敌方单位 / 建筑变化不呈现（R10.1 / R10.5），该可见性过滤是表现层只读派生、不改变实体类型 / 队伍色的既有呈现语义（ADR 0002 §5）

#### Scenario: 呈现只读

- **WHEN** 实体外观随逻辑帧变化并连续重绘，且叠加迷雾可见性过滤与雾遮罩
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节和只读基线一致、迷雾网格 / 已探索层 / 当前可见为表现层本地只读派生状态、不进权威状态 / 状态哈希 / 快照 / `first_difference()`，`crates/w3-render/tests/isolation.rs` 全绿（R10.5 / R12.4 / ADR 0002 §5 / Q9 形态①）

### Requirement: 内核零改动与确定性契约

本 requirement 标题沿用 P6 历史契约名，口径按用户 2026-11-20 / 2026-11-21 拍板方向 A 再修订：已交付的历史行为 SHALL 只经 `w3-core` 既有公开 API 与命令实现；P8.5b 主城升级 / P8.5c 野怪营地与掉落 change MUST 按 ADR 0002 全流程改 `w3-core`，新增「主城等级 / tier」/「野怪 / 营地 / leash / home / 掉落 / 地面物品 / 物品实例 / 随机码消费序列」顶层权威状态 MUST 同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`（R12.4 / ADR 0002 §6-8）。本 change P8.5d 物品 / 商店 / 回城卷轴按推荐方向 A（待规格门 Q1 拍板）继续按 ADR 0002 全流程改 `w3-core`：新增英雄六格背包与物品实例（owner / slot / 充能 / 冷却 / `EffectDef`）、拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖事实、商店库存与刷新、回城传送（施法帧 / 目标 / 中断）、确定性随机码消费序列等权威状态（R6.2 / R7.5 / R7.6 / R9.3），全部 MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`；按 Q5 推荐（条件项、待规格门拍板）新增 `GameplayCommandKind` / `EventKind` 变体（proto 只追加 tag 37 起 / `PROTOCOL_VERSION` 6→7 / `Reject` 旧版本）；determinism 26 项全绿 +「改坏验红」、golden hash 基线按新实现如实更新、不沿用 P8.5c v6 值 `0xc802b63cd6c55b74`；`SNAPSHOT_VERSION` / `SIM_VERSION` 按 Q4 推荐（待规格门拍板）bump 6→7（连同快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R7.5 / R9.3 / R12.4 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 内核零 diff

- **WHEN** 校验 P8.5d 物品 / 商店 / 回城卷轴对 `crates/w3-core/` 的全部改动
- **THEN** `git diff -- crates/w3-core/` 非空属预期，新增六格背包 / 物品实例 owner 与 slot / 商店库存 / 回城传送 / 确定性随机码消费序列等权威状态 MUST 同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」退化（Q9），`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7，MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R12.4 / R7.5 / R9.3 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 哈希与快照字节不变

- **WHEN** 跑完一局并与 P8.5d 新实现的 CPU 基线比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与 P8.5d 新实现基线一致、MUST NOT 断言与 P8.5a / P8.5b / P8.5c / P8.5d 前旧基线逐字节相等，golden hash 基线按新实现如实更新、不沿用 P8.5c v6 值，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7（连同 `SIM_VERSION`、快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝），MUST NOT 沿用「仍为 3、4、5 或 6 / 逐字节不变」旧断言（R12.4 / R7.5 / R9.3 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）

### Requirement: 四组合双族对局的会话装配与逐帧哈希一致（R2 / R11 / R12.4 / R12.7）

系统 SHALL 支持由 2×2 四组合扩到 **4×4 十六组合**（人 / 兽 / 不死 / 暗夜 4 族各对本族、异族与同族；Q1' 已拍板「不死 + 暗夜双族全量」+ P8 两族不重做）的会话装配并各打完一局 CPU 无渲染对局（采集 → 建造 → 训练 → 英雄施法 → 研发 → **Q2 / Q4 全量机制触发** → 战斗 → 摧毁敌方主城结算 → 重开），沿用既有 `MeleeSetup::with_races` / `resolve_victory` / `AiScript` 命令面与 `EffectDef` 组合（既有 `GameplayCommandKind` 码 1–23 不动、Q2 / Q3 新增码 24 起另见 `specs/undead-faction/spec.md`）、不引入 JASS / wtg VM（N5 / R7.6 / ADR 0002）（`docs/roadmap/P8-human-orc-melee.md:148`）。`resolve_victory` MUST 只结算一次、按稳定顺序、结算帧进入权威状态（R11.1 / R11.4）。验收口径 MUST 为「同一组合两实例逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致」（R12.4 / ADR 0002）；因按 Q2 新增权威状态 + 定义层内容变化 + Q3 bump 8，MUST NOT 断言与改动前 v7 旧基线数值相等、MUST NOT 断言跨组合哈希数值相等。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「四组合」「双族」「快照版本不变」等历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 四组合会话装配并打完一局（R11.1 / R11.4 / R12.7）

- **WHEN** 十六组合（含原四组合）各注入一份完整一局脚本并推进到敌方主城被摧毁、触发 `Leave`
- **THEN** 全部组合经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为本地玩家且按稳定顺序，结算帧进入权威状态；本地 / AI 席位 `def_ids` 落在各自族段（`1xx` / `2xx` / `3xx` / `4xx`）、中立恒 `9xx`，腐地 / 古树 / 昼夜 / 隐身目标判定 / 月亮井 / 缠绕金矿等 Q2 / Q4 机制按整数帧确定性结算、新增不死 / 暗夜组合不破坏原四组合基线（R11.1 / R11.4 / R12.7 / R2 / R10.2-4 / R1.5 / Q1' / Q2 / Q4 / Q5'）

#### Scenario: 组内逐帧哈希一致且既有两族基线不破坏（R12.4 / ADR 0002）

- **WHEN** 同一组合两实例注入同一脚本并逐帧推进全部阶段，同时复核既有人 / 兽两族基线测试
- **THEN** 每帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、不只比最终 hash；既有 `melee_loop.rs::both_races_play_full_match_with_baseline_frame_hashes` 与 `race_fallback_matches_human_segment_in_full_match` MUST 按 v8 新基线如实更新后仍绿、不静默改断言、不断言跨组合或 v7 旧基线数值相等（R12.4 / ADR 0002 / Q3 / C4）

#### Scenario: 重开干净与快照版本不变（R11.5 / ADR 0002）

- **WHEN** 任一组合结算后以相同冻结定义与开局脚本重新构造一局并校验快照头部
- **THEN** 新一局 `victory.resolved == false`、视图为空、不含上一局任何权威状态；**历史标题字面「快照版本不变」是身份键、不作本期行为断言**：按用户 Q2 / Q3 已明确拍板（偏离 AI 推荐、MUST NOT 写成 AI 推荐）`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` bump 7→8、`Snapshot` 新增族特色机制顶层字段并四处同步、旧 v7 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、golden hash 按 v8 新实现如实重取（R11.5 / ADR 0002 §6-8 / R6 / Q2 / Q3 / Q5'）

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5）

- **WHEN** 以退化实现（漏接不死 / 暗夜段 `defs_hash` 敏感性 / `3xx` / `4xx` 段混入 `8xxx` / `with_races` 退回单参数 `opponent()` / `race_row()` 散落白名单 / 新增建筑 `speed_raw` 改非 0 / 框选含建筑误生成 `Move` / 新增权威状态漏进 `compute_state_hash` / `Snapshot` / `first_difference()` 之一）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过、两条证据可复现（按 `CODEBUDDY.md §5` 形态①测试内反例 + 元断言防退化、退化实现真经 `capture_snapshot()` / `compute_state_hash()` / 公开命令入口生产路径注入、不冒充形态②）；含建筑 / 资源点 / 主城的选中集合 MUST NOT 生成 `Move`（R1.1 / R1.4 / R9.1 / R13.6 / ADR 0002 §8 / Q1' / Q2 / Q3 / `openspec/config.yaml:57-58`）
