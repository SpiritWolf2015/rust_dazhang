# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实，锚一律「路径 + 符号名」，本轮 `crates/w3-client/` 会被 owner 改动、不写 `:NN` 行号）：

- `crates/w3-client/src/input.rs::InputAction` 现有 14 个动作：`SelectStart` / `SelectDrag` / `SelectEnd` / `ContextAction` / `Cast` / `Train` / `CancelTrain` / `Gather` / `Build` / `Stop` / `AssignGroup` / `SelectGroup`；`input.rs::InputState` 持有 `selection` / `groups` / `next_sequence` / `next_group` / `no_movable_target` 等表现层状态；`input.rs::MovabilityMap` / `derive_movability` / `effective_movability` / `filter_movable` 构成 R1.4 分层判据（frame-0 装配映射原样保留 + 运行时动态实体按只读 `EntityView.speed != 0` 补齐）；`input.rs::InputState::apply` 的群体 `Move` 分支已调 `effective_movability`。`CommandKind::Move { group, append }` 既有 `append` 字段可承载 `Shift` 队列。
- `crates/w3-client/src/app.rs::KeyboardInput` 现有键位：`Q`/`T`/`C`/`G`/`B`/`H` → 命令、`W`/`A`/`S`/`D` 相机平移、`Z`/`X` 旋转、`F1`–`F9` 编组（`assign_group_number`）、数字选组（`select_group_number`）；`app.rs` 的命令卡点击回传把 `hud::CommandAction` 映射为 `InputAction` 并存入 `pending_input_action`（因 GPU 不可变借用，由 `RedrawRequested` 在借用结束后统一提交）；`app.rs::with_entity_kinds` / `with_movability` 注入表现层映射。
- `crates/w3-client/src/hud.rs::CommandAction` 现有 7 个变体（Move / Attack / Stop / Gather / Train / Build / Cancel）+ `label()` ASCII；`hud.rs::derive_commands` 由 `EntityKind` 派生（Peasant → Gather / Build；`is_producer()` → Train / Cancel）；命令卡当前是 `ui.horizontal_wrapped` 线性按钮、**非 12 格 4×3 定义表**；`hud.rs::CommandInteraction`（`command_action` / `minimap_click`）是只读交互结果。
- `crates/w3-client/src/melee/setup.rs::EntityKind` 现有 6 个变体（Peasant / Soldier / TownHall / Barracks / GoldMine / Tree）+ `model_key()` / `is_producer()` / `is_resource()`；`crates/w3-client/src/melee/defs.rs::Race::category_def_ids` 给出人族 6 类新 `DefId`（`HUMAN_RANGED` / `HUMAN_SIEGE` / `HUMAN_FARM` / `HUMAN_ALTAR` / `HUMAN_SMITHY` / `HUMAN_HERO`）的只读查表，全为编译期整数常量。`synthetic_definitions()` 中建筑 `speed_raw == 0`。
- 内核只读事实：`crates/w3-core/src/command.rs::CommandKind` 含 `Move { group, append }` / `Stop` / `Attack { attacker, target }`（**无 Patrol / Hold / AttackMove 变体**）；`crates/w3-core/src/gameplay.rs::GameplayCommandKind` 含 `Cast` / `Train { is_building }` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `RegisterBehavior` / `SpawnWave` / `SetAi` / `Leave`；`crates/w3-core/src/snapshot.rs::SNAPSHOT_VERSION = 3`。主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」requirement 已把「零改动 / 不新增权威状态字段 / `SNAPSHOT_VERSION` 仍 3」写成规范；主 `rts-command-hud/spec.md` 已有「编队与群体命令」「HUD 只读派生与不改确定性」契约；主 `unit-orders/spec.md` 已有 R1.4 分层判据口径。
- 约束：ADR 0001 / 0002 / 0003 / 0004 / 0005、`CODEBUDDY.md` §5 / §8 / §9、`docs/references/WC3-MELEE-RULES.md` §0（S1–S6 / N1–N6）/ §0.1 操作快捷键口径 / §13 `R13.1`–`R13.7` / `R1.2` / `R1.3` / `R1.4` / `R12.2` / `R12.4` / §14、已归档 `2026-11-16-p8-human-faction` 与 `2026-11-16-runtime-entity-movability`。

## Goals / Non-Goals

**Goals:** 在 `crates/w3-client/src/{app.rs,input.rs,hud.rs}` 与 `melee/{setup.rs,defs.rs}` 补出「键位 / 命令卡定义表（编译期整数 `DefId` 组织、`slot = y*4+x`）+ 命令卡 12 格随选中类型切换 + 经典键位映射 + 两级 chords 子菜单栈 + 编队 / 子组 / `Shift` 队列」的表现层只读派生层；键盘 `M` / `A` 复用 R1.4 分层过滤并做反向验证；`crates/w3-core/` 零 diff、零资产、golden hash 逐字节不变。

**Non-Goals:** 不新增任何 `w3-core` 权威状态字段 / `CommandKind` / `GameplayCommandKind` 变体 / 事件；`P` 巡逻 / `H` 保持位置 / `A` 点地面 attack-move（新增命令变体，须另走 ADR 0002 流程，MUST NOT 借本 change「只读视图投影 / 实现层约束放松」旁路，`R13.6`）；把键位 / 命令卡 / 子菜单 / 编队进权威状态；照搬 Warsmash rawcode / godot 原创单位名 / 暴雪 abilId；把具体键位冻结进权威定义；P8.5 深度玩法 / 兽族完整化 / 双族对抗 / 不死暗夜 / 中文上屏（CJK 字体许可证人工拍板）/ 运行时新实体真实模型绑定 / 真窗口人工验收（本轮如实标注「未执行 / 待补验」）。N1–N6 不纳入，尤其 N5（JASS / wtg VM）与 N6（资产入库，ADR 0001）。

## Decisions

> 玩法决策的规则依据与核实状态按 `openspec/config.yaml` 的 `rules.design` 标注：`[已核]` 规则性事实、`[待核]` 具体键位 / 数值（不冻结进权威定义、不入库）、`[取舍]` 本项目实现决定。具体经典键位字母（英雄技能、各族建造 / 训练第二键）、grid / `CustomKeys` 支持程度一律 `[待核]`（`R13.1` / `R13.7`）。承接边界见 D7。

### D1 落点：表现层键位 / 命令卡定义表由编译期整数 DefId 组织，键盘与点击汇入同一 apply

**决策**：新增一张**表现层只读定义表**，由编译期整数 `DefId` 组织「按钮 → 命令 + 快捷键 + 格位」记录；格位按 `slot = y * 4 + x`（4 列 × 3 行 = 12 格）落格、键位与命令**解耦**。当前卡（主卡 / 建造子卡 / 训练子卡 / 商店卡）由「选中集合类型 + 表现层子菜单栈」决定；每次换卡重建「键 → action」表，使同一字母在不同上下文含义不同（`R13.2`）。子菜单栈、放置态、编队、选中集合、`Shift` 队列全部是 `InputState` / HUD 侧的**表现层局部状态**。**键盘事件与命令卡点击 MUST 汇入同一 `input.rs::InputState::apply`**（键盘改经 `app.rs::KeyboardInput` 归一化为 `InputAction`；命令卡点击经 `app.rs` 既有 `pending_input_action` 回传同一 `InputAction`），MUST NOT 新增「命令来源标记」进命令面、MUST NOT 新增分支翻译链。**MUST NOT 新增 `w3-core` 命令变体**：所有键只映射到既有 `CommandKind::{Move, Attack, Stop}` 与 `GameplayCommandKind::{Cast, Train, CancelTrain, Research, Gather, ...}`（`R13.1` / `R13.2` / `R13.3` / `R13.6` `[已核]` 框架、定义表组织 `[取舍]`、具体键位 `[待核]`）。规则依据：R1.2（判据读定义层、不写死类型）、R13.1 / R13.2 / R13.3 / R13.6。

**备选方案**：
1. 把键位表 / 命令卡布局塞进 `w3-core` 的 `FrozenDefinitions` —— 否决：新增权威状态须同步进状态哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`（C1 / 主 `melee-match/spec.md`「内核零改动」）；键位是表现层输入偏好，不是权威规则。
2. 全局静态 `char → CommandKind` 映射表 —— 否决：违反 `R13.2`「同一字母由当前选中集合 + 命令卡状态决定、MUST NOT 全局静态映射」；`B` 在农民 / 生产者、`G` 在农民 / 攻城 / 商店含义不同。
3. 键盘走独立命令生成路径、命令卡点击走 `pending_input_action` —— 否决：`R13.6` 要求键盘与右键 / 点击同一输入翻译链，否则 R1.4 过滤会从键盘路径漏接、「金矿 / 主城跟着移动」bug 复发（C2 / C3）。
4. 用「命令来源标记」区分键盘 / 点击以分别计费或展示 —— 否决：`R13.6` / ADR 0002 明确不新增命令面字段；godot 的 `UnitOrder.Source.HOTKEY` 不借鉴（见「参考项目对照」T5）。

**理由**：Warsmash 的 `AbilityDataUI` / `IconUI` / `MeleeUI` 展示「按钮 → 命令 + 快捷键 + 格位是一张由定义数据填充的表、键位与命令解耦」；godot 的 `command_button_catalog.gd` 展示 `slot = Buttonpos.y * 4 + Buttonpos.x`、`command_card_module.gd` 展示「换卡 `clear()` 后重建键表」。二者都把键位 / 子菜单放在 UI / client 模块（表现层），与本项目 ADR 0002 只读派生边界一致（见「参考项目对照」T4 / T5）。

**ADR 0002 / 跨平台确定性影响**：定义表是编译期整数常量、子菜单 / 编队 / 队列是表现层局部状态，全部不进权威状态 / FNV-1a 状态哈希 / `SNAPSHOT_VERSION`（当前 3）快照；命令仍只经既有 `apply` 产出，排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、稳定 `EntityId` 顺序、FNV-1a、golden hash 逐字节不变。全为整数查表、Windows / Linux 无差异；**不改变任何跨平台确定性结论**，不得据本 change 声称 ADR 0002 有新验证（macOS / ARM64 / 其它 rustc 仍待验证）。

**架构原则对照**：SRP（定义表只管「按钮 → 命令 + 键 + 格」、`apply` 只管翻译）、OCP（新增键 / 卡只改定义表记录，不改翻译链）、DIP（翻译链依赖只读定义表而非具体 `EntityKind`）、唯一状态所有者（表现层状态不反写内核）、显式失败（无选中 / 无目标按既有 `InputError` 返回）。

### D2 命令卡 12 格随选中类型切换，判据读定义层、不用建筑名白名单

**决策**：`hud.rs::derive_commands` 从「按 `EntityKind` 线性 push 标签」改为「按选中集合类型查定义表产出 12 格记录（含空格位）」，`hud.rs::draw_with_minimap` 按 4×3 网格渲染；`hud.rs::CommandAction` 可扩展动作但**每格命令仍映射到既有命令面**。人族 6 类新 `DefId` 的命令卡由 `defs.rs::Race::category_def_ids` 一类**只读查表**驱动，MUST NOT 用建筑名字符串硬编码白名单（`R1.2` / `R13.3` `[已核]`、`[取舍]` 用 `DefId` 而非 `String typeId`）。`setup.rs::EntityKind` 只有 6 个变体、不足以区分 P8.1 新增 6 类，故命令卡派生 MUST 以「装配层 `EntityId → DefId` 只读关联 + 定义表」为真值源，而非扩 `EntityKind` 变体硬编码（Risks 登记）。

**备选方案**：
1. 给 `EntityKind` 增 6 个变体、在 `derive_commands` 里 `match` 出每类命令 —— 否决：把定义数据焊进枚举 match、新增类别就改代码，违反 `R1.2` / `R13.3`「由定义层数据驱动、不写死白名单」；且运行时动态实体无 `EntityKind` 标签。
2. 用 godot 的 `String typeId` 查表 —— 否决：本项目权威定义是编译期整数 `DefId`，`String` 进查表会引入字符串比较与潜在哈希口径污染（C6 / godot T5 边界）。
3. 保持现有线性按钮、只补经典键位 —— 否决：`R13.1` 明确 4×3 = 12 格、每格黄色高亮字母；线性列表无法表达格位 / 子菜单换卡。

**理由**：Warsmash `MeleeUI.java` 的 `COMMAND_CARD_WIDTH = 4` / `HEIGHT = 3` / `CommandCardIcon[][]` 与 `commandCardGridHotkeys = QWER/ASDF/ZXCV`、`switch(INPUT_HOTKEY_MODE)` 命中后都调同一 `onClick(LEFT)` 证明「键位表表现层可切换、命令是格里的 orderId」（见「参考项目对照」T4）；godot `command_card.gd::for_unit(unit_id, state)` 按 `build_menu_open` / `hero_skill_menu_open` / `ShopCatalog` / `BuildingCatalog` 切卡证明「内容随选中类型 + 卡状态重建」（T5）。

**ADR 0002 / 跨平台确定性影响**：命令卡 12 格是 HUD 只读派生、不生成权威命令、不进哈希 / 快照；`BTreeMap` / 有序迭代保证跨平台一致。不改变跨平台结论。

**架构原则对照**：OCP（加类别只改定义表）、SRP（渲染与派生分离）、LSP（新 `DefId` 记录与既有记录对同一派生函数行为一致）、DIP（只读定义注入）。

### D3 两级 chords / 子菜单栈 / 放置态为表现层局部状态，Esc 清空不生成命令

**决策**：在 `InputState` 增「子菜单栈」（当前卡 ID 序列）与「放置态」（待定点命令）两类表现层局部字段；`B` 入栈建造子卡、第二键选建筑转放置态、左键定点时才产出既有 `Train { is_building }`；生产建筑训练键直接产出既有 `Train`。`Esc` 取消瞄准 / 回主卡（`openMenu(0)` 语义清空栈）且**不生成命令**。换卡时 `clear()` 后重建「键 → action」表，使同一字母随卡决定含义（`R13.2` / `R13.3` `[已核]` 框架、`[待核]` 具体第二键）。子菜单栈 / 放置态 MUST NOT 进权威状态 / 哈希 / 快照（ADR 0002）。

**备选方案**：
1. 把子菜单 / 放置态做成 `w3-core` 的「当前命令」权威字段 —— 否决：新增权威状态须同步哈希 / 快照 / `first_difference()`、`SNAPSHOT_VERSION` 处置另拍板（C1 / C4）；参考项目也只把 `subMenuOrderIdStack` / `_build_menu_open` 放 UI / client 模块（T4 / T5 现象，参考缺口 G1）。
2. 每个子菜单一个独立 `InputAction` 变体、不做栈 —— 否决：两级 chords 是复合键序，栈才能表达「返回上级命令卡」「Esc 取消」与嵌套深度；散落变体无法验证 chords。
3. 放置态即时产出命令、左键再改 —— 否决：会产生「子菜单选择即改权威状态」的命令、违反 `R13.3`「`Esc` 取消子菜单 / 放置态、不生成命令」。

**理由**：Warsmash `MeleeUI.java::openMenu(0)` 清空 `subMenuOrderIdStack` 回主卡、`CommandCardPopulatingAbilityVisitor` 的 `menuBaseOrderId` 驱动建造 / 训练 / 英雄技能子菜单、第二键来自 `getStructuresBuilt()` / `getUnitsTrained()` / `getResearchesAvailable()` 定义数据（**不是建筑名白名单**）；godot `command_card_module.gd::_build_menu_open` 局部 bool + `HUD.md` 工人主卡 `open_build` 入口 + 二级建筑列表 + `CmdCancelBuild` + Esc 回主卡。二者共同印证「两级子菜单 + 定义数据驱动第二键 + Esc 取消」（T4 / T5）。

**ADR 0002 / 跨平台确定性影响**：栈 / 放置态是表现层局部、不进哈希 / 快照；只有左键定点最终产出的既有命令进入命令帧。排序键 / golden hash 不变。不改变跨平台结论。

**架构原则对照**：SRP（栈只管 UI 卡状态）、OCP（加子卡只加定义表记录）、显式失败（Esc 取消不静默产出命令）、LoD（子菜单不直接触内核）。

### D4 编队 / 子组 / Shift 队列复用既有命令面与稳定排序键

**决策**：`Ctrl`+数字编组（写 `InputState::groups`，表现层）、数字按组选择、`Tab` 在混编选中集合切同类型子组（表现层游标）、`Shift`+命令把命令排入队列——队列命令逐条经既有 `CommandKind::Move { append: true }`（`command.rs::CommandKind::Move` 既有 `append` 字段）入队，当前命令完成后依次执行。**不新增命令变体、不旁路写入、不改 `ExecuteFrame → PlayerId → Sequence → Arrival` 排序键**（`R13.4` `[已核]` / ADR 0002）。编队允许建筑进入选中集合，但命令分发仍按 `R1.4` 过滤（C3）。`F1`–`F9` 既有编组可作可配置别名 `[待核 / 待用户拍板]`。

**备选方案**：
1. 在 `w3-core` 新增「队列」权威状态字段 —— 否决：新增权威状态须同步哈希 / 快照 / `first_difference()`（C1）；内核 `Move.append` 已表达队列语义，无需新字段。
2. 表现层缓存整段队列、到点一次性提交一条新命令 —— 否决：改命令线格式 / 排序键，回放与联机重放易错配（R12.2 / R12.4）。
3. 照搬 rts_kernel_cs 的 `MoveGroup` / v9 快照 —— 否决：其「编队」是群体落点 / formation、不是 `Ctrl`+数字编队 / 选中集合 / `Tab` 子组语义（T2 边界）；`double` 路线与「不承诺跨平台确定性」不采用（ADR 0002）。

**理由**：rts_kernel_cs `MOVEMENT.md` 的 `OrderMode.Append`（队列最多 64 单、逐条从实际当前位置重新寻路、完成 / 失败下一帧最多激活一个）可校核 `R13.4` 的 `Shift` 队列语义（只借语义校核、不复制实现）；`FRAMEWORK.md` 的排序键与 `UnitOrderBook` 独占订单确认「命令按稳定排序键入队」不因队列改变（T2）。

**ADR 0002 / 跨平台确定性影响**：队列命令仍是既有 `Move { append }`、逐条按稳定排序键入队；编队 / 子组游标是表现层局部、不进哈希 / 快照；golden hash 不变。不改变跨平台结论。

**架构原则对照**：SRP（编队只管选中集合）、DIP（经既有 `submit` 入口）、唯一状态所有者（队列意图在表现层、执行在内核既有订单）、显式失败（命令被拒英文日志告警）。

### D5 键盘 M / A 复用 R1.4 分层判据，与右键走同一 apply（C3，最关键）

**决策**：键盘 `M` → `Move` / `A` → `Attack` **MUST** 与 `R1.3` 右键智能指令路径走同一输入翻译与命令生成处（`input.rs::InputState::apply`），由 `input.rs::effective_movability` 分层判据过滤选中集合：frame-0 装配实体用装配期 `UnitDef.speed_raw` 派生的表现层只读映射（权威；因 `CommandQueue::submit` 强制 `speed.max(1)`，frame-0 建筑运行时视图 `speed > 0`、不可用视图 speed 区分），运行时动态实体（`Train` / `Build` 产出）用只读 `EntityView.speed == 0` 判不可移动、`> 0` 判可移动，两条来源 MUST NOT 互相替代。金矿 / 树木 / 主城 / 建筑 MUST NOT 生成 `Move`、MUST NOT 改变位置；落点按稳定 `EntityId` 顺序、过滤后集合与「只含可移动单位」逐条相同（`R1.4` / `R13.6` / §14 `[已核]` 语义、`[取舍]` 分层判据）。MUST NOT 扩展 `WorldView` / `EntityView`（已是只读视图字段）、MUST NOT 用运行时 `speed` 替代装配映射、MUST NOT 回退建筑类型白名单。`A` 本 change 只归既有 `Attack { attacker, target }` 指定目标入口。

**备选方案**：
1. 键盘 `M` 直接对 `selection` 全体生成 `Move`、不过滤 —— 否决：R1 / R13.6 已知 bug「金矿 / 主城跟着移动」从键盘路径复发（C3 / 已归档 `2026-11-14-melee-playable-loop/design.md` Risks 末条）。
2. 键盘单独做一套「建筑名白名单」排除 —— 否决：违反 `R1.2` / P8.0 D1「判据读定义层 `speed_raw`、不写死类型」，新增 6 类人族定义就要改表。
3. frame-0 也用运行时视图 `speed` —— 否决：`Builder::spawn` 经 `CommandQueue::submit` 把 frame-0 建筑 `speed` 抬到 `max(1)`，会误判建筑可移动、R1 bug 复发（已归档 `runtime-entity-movability` C3）。
4. 在 `w3-core` 收到 `Move` 时丢弃不可移动实体 —— 否决：改内核权威路径、要动哈希 / 快照（ADR 0002 / P8.0 D1 备选 1）。

**理由**：`input.rs::InputState::apply` 的群体 `Move` 分支已调 `effective_movability` 并过滤；键盘只需把 `M` 归一化为与右键地面相同的 `InputAction`，即可自动复用既有过滤与落点分配，零接线改动到 R1.4 判据本身。godot `HUD.md` 的 `is_controllable` / 非己方空卡只提供「非己方不下达命令」现象，不提供实体来源分层判据（参考缺口 G3）；分层判据是本项目 R1.4 / `runtime-entity-movability` 原创取舍。

**ADR 0002 / 跨平台确定性影响**：过滤只在表现层输入翻译、`CommandKind::Move` 线格式 / 排序键 / 哈希 / 快照全部不变；只读 `EntityView.speed` 是投影内核已有权威 `UnitState.speed` 的视图字段、不新增权威状态。`SNAPSHOT_VERSION` 仍 3、golden hash 逐字节不变。不改变跨平台结论。

**架构原则对照**：SRP（过滤只在 `apply` 一处）、DIP（只读冻结定义 / 只读视图）、LSP（frame-0 与运行时实体对同一 `effective_movability` 行为一致）、显式失败（全不可移动给反馈位、不生成空命令）。

### D6 经典键位映射与 WASD 相机冲突的处置为 [取舍 / 待核]

**决策**：经典键 `M` / `A` / `S` / `G` / `B` / `T` / `Esc` 映射到既有命令面（D1）；但现状 `app.rs::KeyboardInput` 把 `A` / `S` / `W` / `D` 用作相机平移、`H` 用作 `Stop`。本 change **不静默改相机键位**：`A` / `S` 的经典语义与相机平移冲突的处置（相机改绑方向键 / 边缘滚动 / 修饰键，或经典键加修饰键区分）一律标 `[待核 / 待用户拍板]`，进 `decision-memo.md` 待确认项；现有夹具键位是否保留为可配置别名同 `[待核]`（`R13.7`）。`S`=Stop 与 `H`=Hold 语义不同、MUST NOT 合并，本 change 只做 Stop（Hold 承接，C4 / `R13.2`）。具体键位字母以本机导出 SLK 命令卡热键字段或原版客户端核对、不冻结进权威定义、不入库（`R13.1` / `R13.2` `[待核]`）。

**备选方案**：
1. 直接把 `A` / `S` 改成经典攻击 / 停止、相机平移删掉 —— 否决：相机是开发夹具既有能力，未经用户拍板删除属静默改行为；且 `WASD` 冲突处置口径未定（`R13.7`）。
2. 冻结一份「经典键位表」进 `FrozenDefinitions` —— 否决：具体键位 `[待核]`、以本机导出数据核对、不冻结进权威定义、不入库（C6 / ADR 0001 / ADR 0004）。
3. 照搬 Warsmash `InputHotkeyMode=1`（grid `QWER/ASDF/ZXCV`）为默认 —— 否决：那是其 ini 默认值、`R13.1` 明确本项目以**经典布局**为基线、grid 布局 `[待核 / 待拍板]`（T4「不借鉴」）。

**理由**：`R13.7` 已如实登记现状键位差距、并要求「处置 `[待核 / 待用户拍板]`、后续 change 不静默改键位、不静默改规则」；本项目按 §14 第 2 条「不静默改规则」把冲突交回人拍板。

**ADR 0002 / 跨平台确定性影响**：键位是表现层输入偏好、不进权威状态 / 哈希 / 快照；`[待核]` 不冻结进定义。不改变跨平台结论。

**架构原则对照**：显式失败（冲突不静默降级）、ISP（相机与命令键位分离暴露）、单一权威（键位口径以活文档 §13 + 人拍板为准）。

### D7 R13.6 范围限定：P / H / attack-move 由后续 change 承接（C4，最关键）

**决策**：本 change **只复用既有命令面**（`CommandKind::{Move, Attack, Stop}` + `GameplayCommandKind::{Cast, Train, CancelTrain, Research, Gather, ...}`）实现经典键。`P` 巡逻 / `H` 保持位置 / `A` 点地面 attack-move（行军途中遇敌自动交战）属**新增命令变体**——`command.rs::CommandKind` 无对应变体，MUST 由后续独立 change 承接并按 ADR 0002 复核：新增权威状态 / 命令面 MUST 同步状态哈希、快照与 `first_difference()`、跑 `cargo test -p w3-core --test determinism`、`SNAPSHOT_VERSION` 处置另行拍板；**MUST NOT 借本 change「只读视图投影 / 实现层约束放松」旁路门禁**。`A` 本 change 只归既有 `Attack` 指定目标入口（attack-move `[待核]`）；`S`=Stop 与 `H`=Hold 语义不同 MUST NOT 合并（本 change 只做 Stop，Hold 承接）（`R13.2` / `R13.6` `[已核]` 框架 / `[取舍]` 承接）。

**备选方案**：
1. 在本 change 用既有 `Move` + `Attack` 组合近似 attack-move / 巡逻 —— 否决：语义不等价（attack-move 点地面途中交战、巡逻两点往返），会丢失命令语义或偷偷扩命令面（C4 / `R13.2`）；godot `COMBAT_SYSTEM.md` 明确 `ATTACK` / `ATTACK_MOVE` / `HOLD` 是**不同 FSM 模式**（T2 旁证）。
2. 新增 `CommandKind::Patrol` / `Hold` / `AttackMove` 变体一并做完 —— 否决：新增命令面须同步哈希 / 快照 / `first_difference()` / determinism / `SNAPSHOT_VERSION` 处置，属 ADR 0002 流程与另一个 change 的范围，不是「表现层只读派生」（C1 / C4）。
3. 借 `runtime-entity-movability` 的「只读视图投影 / 实现层约束放松」把新命令当只读派生塞进来 —— 否决：`R13.6` 第 3 条范围限定明确放松只适用于「复用既有 `Move` / `Attack` 路径 + R1.4 分层判据过滤」，MUST NOT 被借用于新增任何权威状态 / 命令 / 事件。

**理由**：`R13.6` 第 3 条以「范围限定（防过度承诺）」逐字划界；`command.rs::CommandKind` 现状只读核实无对应变体；godot `COMBAT_SYSTEM.md` 的 FSM 模式差异佐证「Stop 与 Hold 不得合并、attack-move 不可简化成 `Attack{target}`」（T2）。

**ADR 0002 / 跨平台确定性影响**：本 change 不碰命令面 / 权威状态，故零冲击；后续承接 change MUST 另走 ADR 0002 全流程（哈希 / 快照 / `first_difference()` / determinism / `SNAPSHOT_VERSION` 拍板）。不得据本 change 声称跨平台确定性有新验证。

**架构原则对照**：SRP（一个 change 只做操作层表现派生）、显式失败（不静默扩命令面）、唯一状态所有者（新命令面归后续 change 单一 owner）。

### D8 内核零 diff 与确定性契约零冲击（C1 / C2）

**决策**：`crates/w3-core/` MUST 零 diff——不新增权威状态字段 / `CommandKind` / `GameplayCommandKind` 变体 / 事件；`snapshot.rs::SNAPSHOT_VERSION` 仍 3；状态哈希字段集合、快照头部布局（`defs_hash` / `content_hash`）、`first_difference()`、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、稳定 `EntityId` 顺序、FNV-1a、golden hash 逐字节不变。键位 / 命令卡 12 格 / 子菜单栈 / 放置态 / 编队 / 选中集合 / `Shift` 队列 MUST 全是表现层局部状态、MUST NOT 进权威状态 / 状态哈希 / `SNAPSHOT_VERSION`（当前 3）快照；键盘与命令卡点击 MUST 汇入同一 `apply`（C2）。`cargo test -p w3-core --test determinism`（26 项）、`cargo test --workspace`、`git diff -- crates/w3-core/` 为空为验收线（ADR 0002 §6-8 / `CODEBUDDY.md` §5 / 主 `melee-match/spec.md`「内核零改动」）。

**备选方案**：
1. 为「键位配置」在 `Player` 加一个权威字段方便联机同步 —— 否决：键位是本地输入偏好、不参与对局状态；新增权威状态须同步哈希 / 快照 / `first_difference()`（C1）。
2. 把子菜单 / 编队状态写进快照以「方便回放显示」 —— 否决：表现层只读派生、不进哈希 / 快照（ADR 0002 §5 / `R13.4` / `R13.6`）；回放只重放命令序列（R12.2 / R12.4）。

**理由**：主 `rts-command-hud/spec.md`「HUD 只读派生与不改确定性」、主 `melee-match/spec.md`「内核零改动与确定性契约」已是既有规范；本 change 只在 `w3-client` 表现层扩能力。

**ADR 0002 / 跨平台确定性影响**：零冲击即本决策目标；`defs_hash` / `content_hash` 不因键位定义表变化（定义表是表现层、不是 `FrozenDefinitions` 内容）。不改变跨平台结论，macOS / ARM64 / 其它 rustc 仍待验证。

**架构原则对照**：唯一状态所有者、w3-core 不反向依赖上层、显式失败、只读参考不复制代码。

### D9 反向验证（改坏验红）与 CPU 回归（C5）

**决策**：新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过——仅「全绿」不构成验证（`CODEBUDDY.md` §5）。证据形态用「测试内反例元断言」或 A-B 对照，参照已归档 `runtime-entity-movability` 的 `tests/movability.rs::degenerate_effective_movability` 与 P8.0 `degenerate_filter` 先例。至少覆盖：① 键盘 `M` / `A` 路径退化成「不过滤 / 把建筑当可移动 / frame-0 误用视图 speed」→ 混合选中集合含建筑时「不可移动实体 MUST NOT 出现在 `Move` 生成集合」断言失败；② 子菜单 / 键位映射退化（全局静态映射、不清栈）→ `B`→`A` chords 与「同一字母随卡切换」断言失败。另补键→命令表、12 格 `slot = y*4+x`、chords、R1.4 键盘路径、哈希 / 快照 / `SNAPSHOT_VERSION=3` 不变的 CPU 回归，并**重跑 `cargo test -p w3-core --test determinism`**（涉及确定性契约改动，按 `CODEBUDDY.md` §5 / `rules.tasks` 第二条）。

**备选方案**：
1. 只加正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，不构成反向验证。
2. 人工改坏代码验红 —— 否决：不可复现、不进 CI（P8.0 D3 备选 2）；除非与元断言配合做一次 A-B 留命令输出。
3. 只比最终 hash、不比逐帧 —— 否决：ADR 0002 复核记录要求逐帧文本 / 字节比对，最终 hash 相同可能掩盖中间帧 desync。

**理由**：`CODEBUDDY.md` §5 与 `openspec/config.yaml` `rules.tasks` 第三条为硬门禁；`runtime-entity-movability` 已给出 w3-client 侧真 A-B 先例（退化实现撤回合并函数接线 → 4 条断言变红、恢复后全绿）。

**ADR 0002 / 跨平台确定性影响**：测试只在 CPU 侧构造视图 / 命令 / 键位表、不触碰 `w3-core` 权威状态；determinism 26 项确认零影响。不得据 CPU 全绿声称跨平台确定性有新验证。

**架构原则对照**：SRP（验收脚本独立于实现）、显式失败（每阶段断言）、CARP（复用既有 `tests/movability.rs` 反例辅助）。

### D10 零资产 / 具体键位 [待核] / 1.0 范围（C6 / C7 / C8）

**决策**：键位 / 命令卡定义表全为合成 / 原创编译期整数常量或表现层只读映射，零文件 IO、零 MPQ 派生数据、零真实导出 JSON、零新增依赖 / `Cargo.lock` 改动。经典键位字母（尤其英雄技能、各族建造 / 训练第二键）、grid / `CustomKeys` 支持程度一律 `[待核 / 待用户拍板]`、以本机导出 SLK 命令卡热键字段或原版客户端核对、不冻结进权威定义、不入库（ADR 0001 / ADR 0004 / `R13.1` / `R13.7` / §14）。MUST NOT 照搬 Warsmash `ahky` / `abpx` rawcode 字段名作常量、godot 原创三族单位名 / 数值、暴雪 abilId（`AHbu` / `AObu`）。本 change 落在 S1（局域网联机对战，含真人 VS AI 敌人玩家）/ S3（4 种族）/ S4（道具）的**操作层细化**、不新增范围编号（S1–S6 / N1–N6 不变）；N1–N6 不纳入（尤其 N5 JASS VM / N6 资产入库）；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项（`WC3-MELEE-RULES.md` §0 / §0.1 操作快捷键口径 / R13 章首 / `R12.2` / `R12.4` / ADR 0005）。兽族完整化 / 双族对抗 / 不死暗夜 / P8.5 深度玩法 / 中文上屏 / 运行时新实体真实模型绑定 / 真窗口人工验收由后续独立 change 承接（C8 / `docs/roadmap/P8-human-orc-melee.md` §3 / §6 / 已归档 `p8-human-faction` 归档说明）。

**备选方案**：
1. 把公开攻略的经典键位表硬编码进代码 —— 否决：具体键位 `[待核]`、以本机导出数据核对、不锁定冒充原版、不入库（C6 / §14）。
2. 顺手把 P8.4 HUD 剩余项 / P8.5 深度玩法塞进本 change —— 否决：单 change 单 owner、C8 不扩范围（用户 2026-11-16 只授权「R13 操作层与 HUD」方向）。
3. 因「联机 / 回放 / AI 玩家」不在本 change 交付就写成不做项 —— 否决：C7 明确它们是 1.0 已在能力 / 范围、不是不做项。

**理由**：ADR 0001 / 0004 与活文档 §0 / §13 / §14 是单一权威；参考项目只借字段名 / 结构、不携带数值 / 字符串表（「参考项目对照」许可证边界）。

**ADR 0002 / 跨平台确定性影响**：`[待核]` 键位不进权威定义 / 状态哈希 / 快照；零资产不改 `defs_hash` / `content_hash`。不改变跨平台结论。

**架构原则对照**：正确性 / 确定性优先、显式失败（缺口如实标注不冒充）、唯一状态所有者。

## 参考项目对照

> 按 `docs/references/UPSTREAM.md`「按任务类型的强制参考矩阵」（`UPSTREAM.md` §「按任务类型的强制参考矩阵」、T1–T6 表、触发时机第 1–5 条）与 `openspec/config.yaml` 的 `rules.design`。本 change 命中 **T2**（输入 / 编队 / 帧阶段语义，D4 / D5）、**T4**（命令卡数据组织 / 玩法语义，D1 / D2 / D3）、**T5**（HUD / 命令卡，D1 / D2 / D3）；`R12.2` 纯命令传输旁证 **T1 / T6**（D8）。四条结论由只读 `reference-researcher` 按矩阵出「参考对照报告」（leader 本会话已接收），下列所有锚已由 `spec-steward` **逐条实际打开核实存在、语义相符**（本轮打开的是 `C:/open_rts/<项目>` 固定提交，见 `UPSTREAM.md` 开头固定版本表，不在本文重复本机绝对路径）。四个参考项目均为**只读**、代码不进入本仓库（ADR 0001）；本机绝对路径 / 资产 / 派生数据一律不入库。许可证：`vera20k` GPLv3 + Section 7 附加条款（仅真的复制代码时才进入）、`godot_warcraft3` / `rts_kernel_cs` MIT、`WarsmashModEngine` **AGPLv3 仅借鉴语义、不得复制任何代码**。参考缺口按 `UPSTREAM.md` 触发时机第 5 条如实登记、回到人拍板、不臆造。

- **T4 命令卡数据组织 → `WarsmashModEngine`（AGPLv3，仅语义）**。锚（逐条核实）：`core/src/com/etheller/warsmash/viewer5/handlers/w3x/rendersim/ability/AbilityDataUI.java:25,40-42,86,100,141-152`（`"Hotkey"` / `"Unhotkey"` / `"Researchhotkey"` / `"Buttonpos"` 是定义表字段名、`getHotkey()` / `getFieldAsInteger(ICON_NORMAL_XY, 0/1)` 读格位与键）；`.../rendersim/ability/IconUI.java:8-12,49-51`（`buttonPositionX` / `buttonPositionY` / `char hotkey` 数据字段）；`.../rendersim/commandbuttons/CommandCardPopulatingAbilityVisitor.java:470-517`（`addCommandButton(...)` 把 `buttonPosX/Y` / `orderId` / `menuButton` / `hotkey` 交 `commandButtonListener`）；`.../ui/MeleeUI.java:217-218,309,408-410,4001-4021`（`COMMAND_CARD_WIDTH = 4` / `COMMAND_CARD_HEIGHT = 3`、`CommandCardIcon[][]`、`commandCardGridHotkeys = QWER/ASDF/ZXCV`、`switch(WarsmashConstants.INPUT_HOTKEY_MODE)` 命中后都调 `onClick(Input.Buttons.LEFT)` —— 键位表表现层可切换、命令是格里的 `orderId`）；`.../util/WarsmashConstants.java:53,82` + `core/assets/warsmash.ini:59`（`InputHotkeyMode` 由 ini 读入、`InputHotkeyMode=1` 注释为 grid 模式）；`.../ui/CommandCardIcon.java:212-224`（点击与按键汇入同一 `onClick` / `openMenu`）；`CommandCardPopulatingAbilityVisitor.java:62,70,89,344-359,398-418,566-607,665-689`（`menuBaseOrderId` 驱动建造 / 训练 / 英雄技能子菜单，第二键列表来自 `getStructuresBuilt()` / `getUnitsTrained()` / `getResearchesAvailable()` **定义数据、不是建筑名白名单**）；`.../ui/MeleeUI.java:3937-3959,1467-1477`（`openMenu(0)` 清空 `subMenuOrderIdStack` 回主卡）；行为表按种族分组 `core/assets/abilityBehaviors/`（`humanUnitActives.json:1-45`、`orcHeroActives.json:1-2`）、`.../units/manager/MutableObjectData.java:408-428`（`urac` / `arac` / `frac` / `grac` race 分类字段）。**借鉴**：仅「按钮 → 命令 + 快捷键 + 格位是一张由定义数据填充的表、键位与命令解耦」「键位表表现层可切换」「子菜单栈由 `menuBaseOrderId` 驱动、第二键来自定义数据」「点击与按键汇入同一 `onClick`」的**语义**（D1 / D2 / D3）。**不借鉴 / 边界**：AGPLv3 不复制任何代码 / 不引入 Java / LibGDX 依赖；`ahky` / `abpx` / `auhk` 等 rawcode 字段名不作本项目常量（其源码注释 `// replaced from 'ahky'` 是内部沿革，不是经典事实）；暴雪图标路径 / tooltip / 单位名不借；`InputHotkeyMode=1`（grid）默认值不作经典事实（本项目 R13.1 以经典布局为基线、grid `[待核]`）；`Esc=0x7E` 内部约定不作经典事实；`resources/Scripts/melee.jui` 仅为 JASS UI 帧定义、不作命令卡数据来源（`UPSTREAM.md` T4 矩阵条目 / `docs/roadmap/P8-human-orc-melee.md` §1）。许可证理由：AGPLv3 与本项目 GPLv3 组合有条件、未经 ADR 0001 复核不得引入其代码，故只借语义。

- **T5 HUD / 命令卡 → `godot_warcraft3`（MIT）**。锚（逐条核实）：`docs/design/game/ABILITY_SYSTEM.md:3,12-13,29,117,126`（`Data → Logic → Present` 分层、`:29` `command_button_catalog.gd ← Func+Strings → HUD 槽位/图标`、`:117` 自承「**Logic 仍按行为类型分**（无法纯数据驱动）」→ **不得写成全数据驱动**）；`docs/roadmap/NEXT.md:151-155,166`（`:152` 数值读 SLK Def、禁止逻辑里写死造价 / 伤害、`:153` 技能拼 Effect + 注册 Behavior、`:166` PR 标明所处层）；`apps/game/client/command_card/command_card.gd:1-10,68-73,85-120,280-330`（`_empty_card().resize(12)`、`for_unit(unit_id, state)` 按 `build_menu_open` / `hero_skill_menu_open` / `ShopCatalog` / `BuildingCatalog` 切卡、`"slot"` / `"buttonpos"` 格位数据）；`packages/gameplay/catalog/command_button_catalog.gd:9,175`（`slot = Buttonpos.y * 4 + Buttonpos.x`、`clampi(pos.y,0,2) * 4 + clampi(pos.x,0,3)` —— **12 格格位公式的精确落点**）；`.../command_card_module.gd:47-49,668-686`（`_build_menu_open` 局部 bool、换卡 `clear()` 后重建 `_card_hotkey_actions[hk] = id` → **同一字母不同上下文含义由当前卡重建决定**，呼应 `R13.2`）；`docs/design/game/HUD.md:19,41,94-113,115-122,189,230,241`（`CommandCard.for_unit(primary.typeId, …)`、`:113` 非己方空卡、`:81` `Tab` → `cycle_primary`、`:117-119` 工人主卡只放 `open_build` 入口 + 二级建筑列表 + `CmdCancelBuild` + Esc 回主卡、`:241` 阶段 B 建造二级菜单已落地、`:13` 明写「**不复刻 WC3 Console 贴图外壳**」是其产品取舍 → 本项目复刻经典、**不借该布局决策**）；`apps/game/client/interaction/input/match_input_controller.gd:118-180`（输入顺序「Esc 取消瞄准 / 回主卡 → 右键智能指令 → Tab 子组 → 命令卡热键」、`handle_submenu_escape()`、`try_hotkey` → `dispatch_action(id, Source.HOTKEY)`）；`docs/design/game/FACTION_UNIT_ROSTERS.md:1-17,29-45`（原创三族单位名 / 数值，「名称、造价和数值是种子、不是最终平衡承诺」）。**借鉴**：`Data → Logic → Present` 分层、`slot = y*4+x` 格位公式、换卡重建键表、工人主卡 + 二级 + Esc 的输入顺序、分批灰盒 / 结束比赛判据的**排期结构**。**不借鉴 / 边界**：不复制 GDScript；不借 `String typeId`（本项目编译期整数 `DefId`）；不借 `_RACE_BUILD_ABIL` 的暴雪 abilId（`AHbu` / `AObu`）；不借硬编码 fallback `KEY_B` / `KEY_O` / `KEY_ESCAPE`（其注释自承 Func 常缺 Tip / Hotkey）；**不借 `UnitOrder.Source.HOTKEY` 这一「命令来源标记」进权威状态**（R13.6 要求键盘与右键同一输入翻译链、不新增命令面字段，D1）；不借 `HUD.md:13`「不复刻 Console 外壳」的布局取舍（本项目复刻经典）；`FACTION_UNIT_ROSTERS.md` 原创三族单位名 / 数值不借，只借排期结构；`ABILITY_SYSTEM.md:117` 自承 Logic 仍按行为类型分 → 本项目命令卡**不得写成全数据驱动**、只是「按钮 → 既有命令」查表层。许可证理由：MIT 借结构、不复制实现代码。

- **T2 输入 / 编队 / 帧阶段语义 → `rts_kernel_cs`（MIT）+ `godot_warcraft3`（MIT，COMBAT_SYSTEM FSM 旁证）**。锚（逐条核实）：`rts_kernel_cs/docs/MOVEMENT.md:36,73-81,98-113`（`CommandEnvelope.MoveGroup` 用 "deterministic minimum squared-distance matching" 分配稳定 ideal slots、`ReadMoveOrders` 返回路径副本不改对局）；`rts_kernel_cs/docs/FRAMEWORK.md:9-22,62-64,105-109`（排序键 "ExecuteFrame → PlayerId → Sequence → ArrivalOrder"、`UnitOrderBook` 独占订单、`Read*()` 返回独立只读视图、"不纳入：…战斗/经济/技能迁移"）；`MOVEMENT.md:17,23-27,36`（`OrderMode.Append` 队列最多 64 单、逐条从实际当前位置重新寻路、"完成/失败后下一帧最多激活一个后续订单" → 校核 `R13.4` Shift 队列语义）；`godot_warcraft3/docs/design/game/COMBAT_SYSTEM.md:19,27,45-52`（`ATTACK` / `ATTACK_MOVE` / `HOLD` 是**不同 FSM 模式**：固定目标追击 / `find_acquire_target` 有目标则追 / 仅射程内不追 → 佐证 `R13.2`「attack-move 不可简化成 `Attack{target}`」「Stop 与 Hold 不得合并」）。**借鉴**：稳定 ID 落点 / 稳定集合枚举顺序、`Append` 队列语义校核、排序键口径、不同战斗模式的语义区分（D4 / D5 / D7）。**不借鉴 / 边界**：`double` 路线与「未声明跨平台确定性 / 不承诺跨平台确定性」（`MOVEMENT.md:40,71`、`FRAMEWORK.md:34-38`）不采用（ADR 0002）；其「编队」是 `MoveGroup` 群体落点 / formation、**不是** `Ctrl`+数字编队 / 选中集合 / `Tab` 子组语义，**不冒充后者参考**（参考缺口 G5）；`MoveGroup` / v9 快照不复制，`SNAPSHOT_VERSION` 仍 3；`DamagePipeline` / `CombatRng` 等 GDScript / C# 战斗结算实现不移植（本项目走既有 `Attack` / `Cast`）。许可证理由：MIT 借语义、不复制实现代码。

- **T1 / T6 旁证 `R12.2` 纯命令传输 → `vera20k`（GPLv3 + Section 7，仅范式）**。锚（逐条核实）：`src/net/lockstep.rs:1-20,35-39,264-269,424-435,462-488,539-570,631-649`（`issue()` / `admit()` 两条 FIFO "Append one locally issued command in issue order" / "Admit one received/replay command in arrival order"、**不区分输入设备**；`FrameInfo { checksum }`；`frame_info()` 内 `checksum_history.compare(frame_info)?` 逐帧比对（`:648`）；`SynchronizedCommand` 以字节为权威、解码只在 admit 之后）。**借鉴**：lockstep「只传输入命令、不传状态」「逐帧 checksum 比对」「命令入队不按输入设备区分」的**范式**，呼应键盘命令与鼠标命令同样经命令帧（D8 / `R12.2` / `R13.6`）。**不借鉴 / 边界**：RA2 专属 `gamemd.exe` 地址 / opcode、`CommandRecord` / `FRAMEINFO` 线格式不照搬（本项目 W3N1 + protobuf，ADR 0005）；未复制任何代码，故 **GPLv3 + Section 7 附加条款不进入本项目**（ADR 0001）。许可证理由：仅借范式、不复制代码。

- **两处归因纠正（MUST 写进 design、不得误挂参考项目）**：①「新增权威状态字段须同步进状态哈希 / 快照 / `first_difference()`」硬门禁的出处是**本项目** `CODEBUDDY.md` §5、`docs/decisions/0002-determinism.md` §6-8、主 `openspec/specs/melee-match/spec.md`「内核零改动与确定性契约」requirement，**不是任何参考项目**；C1 / D8 的依据指向这三者，`decision-memo.md` 该承重约束锚指向本项目权威文件或矩阵条目 `UPSTREAM.md` §「按任务类型的强制参考矩阵」触发时机第 2 条，**不指向参考项目路径**。②「表现层只读派生、不进权威状态 / 哈希 / 快照」是本项目 ADR 0002 §5 + `R13.4` / `R13.6` 的**取舍**，参考项目（Warsmash `MeleeUI.java:312,327-329,1467-1477` 的 `subMenuOrderIdStack` 是表现层 UI 字段、godot `command_card_module.gd:48` 的 `_build_menu_open` 是 client 局部 bool）只提供「子菜单状态确实放 UI / client 模块」的**实现现象**，不提供 ADR 0002 式确定性契约；**不得据本 change 声称跨平台确定性有新验证**（macOS / ARM64 / 其它 rustc 仍待验证，ADR 0002 复核记录）。

- **参考缺口 G1–G5（如实登记、不臆造、不冒充参考结论）**：**G1** 子菜单 / 放置态「只读派生不进哈希 / 快照」——Warsmash `subMenuOrderIdStack`（`MeleeUI.java:312,327-329,1467-1477`）是表现层 UI 字段、godot `_build_menu_open`（`command_card_module.gd:48`）是 client bool，但四个项目都**无 ADR 0002 式契约**；本项目按 ADR 0002 §5 / `R13.6` 取舍落地，不冒充参考结论。**G2** 编译期整数 `DefId` + 表现层只读映射派生可移动性——Warsmash `War3ID` / godot `String typeId`，四项目均**无** `R1.4` / `R13.6` 的 `effective_movability` 分层判据（frame-0 装配映射 + 运行时 `EntityView.speed`）；本项目原创取舍。**G3** 键盘 `M` / `A` 复用 `R1.4` 过滤防「金矿 / 主城跟着移动」bug 复发——godot 有 `is_controllable` / 非己方空卡（`HUD.md:113`）但**非实体来源分层判据**；需求源是本项目 §13 §1。**G4** `CustomKeys` / 运行时可配置键位——Warsmash 固定提交**无** `CustomKeys.txt` 实体、全仓字面零命中，只有 `InputHotkeyMode` 0/1 模式级切换；godot 全仓 `CustomKeys` **零命中**（本轮 `grep` 实测 count=0）、编队 / 队列字面零命中 → 本项目按 `R13.1` / `R13.7` 维持 `[待核 / 待用户拍板]`。**G5** `Ctrl`+数字编队 / `Tab` 子组 / `Shift` 命令队列完整语义——rts_kernel_cs `MoveGroup` 只覆盖群体落点、godot `HUD.md:81` 只有 `Tab` → `cycle_primary` 切主选（**不是**同类型子组）、Warsmash 未逐锚确认 → 需求源仍为本项目 §13，不臆造参考结论。**这些缺口方向已由用户 2026-11-16 拍板放行（等价规格门批准），但逐项键位 / 支持程度仍 `[待核]`、不冒充已对齐**（`decision-memo.md` 待确认项）。

- **许可证边界**：`WarsmashModEngine`（AGPLv3）仅借语义、不复制代码 / rawcode / 暴雪单位名 / 数据表内容、不引入 Java / LibGDX 依赖；`godot_warcraft3` / `rts_kernel_cs`（MIT）借结构、不复制 GDScript / C#；`vera20k`（GPLv3 + Section 7）未复制代码故附加条款不进入。`CommandFunc.txt` / `CommandStrings.txt` / `Hotkey` / `Buttonpos` 在参考项目是从玩家本机 MPQ dataSource **运行时读取**、固定提交内无实体文件；本项目只引用**字段名 / 结构**、不携带任何数值 / 字符串表，资产与派生数据一律不入库（ADR 0001 / ADR 0004）。

- **锚点校正（如实登记）**：① 对照报告把「`slot = Buttonpos.y * 4 + x`」挂在 `command_card.gd`，本轮逐锚核实其**精确公式**在 `packages/gameplay/catalog/command_button_catalog.gd:9,175`，`command_card.gd:68-73,280-330` 只负责 `_empty_card().resize(12)` 与按 `"slot"` / `"buttonpos"` 落格；本文按精确落点引用（T5）。② Warsmash `AbilityDataUI.java` 的 `Hotkey` / `Buttonpos` 是**可读字段名**、rawcode 名（`ahky` / `abpx`）只在源码注释 `// replaced from ...` 中出现，故「不照搬 rawcode 作常量」与「借字段名」不矛盾（T4）。③ vera20k `checksum_history.compare(frame_info)` 精确在 `lockstep.rs:648`（`frame_info()` 内），报告给出的 `:631-649` 区间覆盖该行、语义相符（T1/T6）。

## Risks / Trade-offs

- **键位冲突：`WASD` 相机 vs 经典 `A` / `S` / `H`** → 现状 `app.rs::KeyboardInput` 把 `A` / `S` / `W` / `D` 用作相机平移、`H` 用作 `Stop`，与经典 `A` 攻击 / `S` 停止 / `H` 保持位置冲突（`R13.7`）。缓解：D6 不静默改相机键位，冲突处置（改绑方向键 / 边缘滚动 / 修饰键 / 可配置别名）一律 `[待核 / 待用户拍板]`、进 `decision-memo.md` 待确认项；`S`=Stop 与 `H`=Hold 不合并、本 change 只做 Stop，Hold 后续承接（C4）。具体键位以本机导出 SLK 核对、不冻结（C6）。
- **`EntityKind` 只有 6 类、如何承载人族 6 类新 `DefId` 的命令卡** → `setup.rs::EntityKind`（Peasant / Soldier / TownHall / Barracks / GoldMine / Tree）与 `hud.rs::CommandAction`（7 变体）都不足以区分 P8.1 新增的 `HUMAN_RANGED` / `HUMAN_SIEGE` / `HUMAN_FARM` / `HUMAN_ALTAR` / `HUMAN_SMITHY` / `HUMAN_HERO`。缓解：D2 用「装配层 `EntityId → DefId` 只读关联 + 定义表」作命令卡真值源、不扩 `EntityKind` match 白名单；运行时动态实体无 `EntityKind` 标签，按 `Race::category_def_ids` 一类只读查表派生；运行时新实体的命令卡随类型变化待 P8.4 承接（`docs/roadmap/P8-human-orc-melee.md` §3 / 已归档 `p8-human-faction` 归档说明，C8）。
- **子菜单状态与 GPU 借用 / 命令提交时序** → `app.rs` 命令卡点击因 `gpu` 不可变借用先把 `InputAction` 存入 `pending_input_action`、由 `RedrawRequested` 在借用结束后提交；键盘事件走 `report_input` → `apply_input`。缓解：D1 / D3 让两者汇入同一 `InputState::apply`，子菜单栈 / 放置态只在表现层变更、定点时才产出命令；不在 render pass 借用期间写权威状态。若键位命中与 GPU 提交跨帧，命令序号 / 执行帧仍按冻结帧号推导（`input.rs::execute_frame`），不引入墙钟（ADR 0002）。
- **运行时新实体 `model_keys` 回退** → P8.1 已知限制：运行时 `Train` / `Build` 产出实体走模型缺键回退。本 change 不改渲染 / 模型绑定，命令卡对运行时实体按只读定义表派生、不依赖模型键；真实模型绑定由后续 change 承接（C8）。
- **`[待核]` 数值 / 键位不冒充原版** → 经典键位字母、英雄技能 / 各族建造训练第二键、grid / `CustomKeys` 支持程度全 `[待核]`，以本机导出 SLK 命令卡热键字段或原版客户端核对、不入库；命名不照搬 Warsmash rawcode / godot 原创单位名 / 暴雪 abilId（C6 / 参考缺口 G4）。验收线是「键→命令表 / 12 格 / chords / 键盘路径过滤可验证且哈希不变」，不是「键位对齐原版」。
- **R13.6 承接边界（防过度承诺）** → 本 change 的 `w3-core` 零 diff / 只读派生**仅适用于**复用既有 `Move` / `Attack` / `Stop` 与 `GameplayCommandKind` 既有变体 + R1.4 分层过滤。`P` / `H` / attack-move 等新命令变体 MUST 由后续 change 按 ADR 0002 流程承接（哈希 / 快照 / `first_difference()` / determinism / `SNAPSHOT_VERSION` 处置另拍板），MUST NOT 借本 change「只读视图投影 / 实现层约束放松」旁路（D7 / C4）。
- **`file:line` 锚漂移** → 本轮 `crates/w3-client/` 会被 owner 改动，四件套正文与 `decision-memo.md` 的**本项目代码锚一律「路径 + 符号名」**、不写 `:NN` 行号（P8.1 行号漂移返工教训）；参考项目为只读固定提交、其 `file:line` 锚不漂移、按对照报告原样保留。四件套任一 artifact 改动按 `CODEBUDDY.md` §9 规格锁定与重审处理、MUST 同步回查其余三份并重走规格门。
- **真窗口人工验收未执行** → 本轮为规格成文 + 参考对照 + 承重约束站稳；真窗口键鼠手感 / 命令卡布局人工验收**未执行 / 待补验**，CPU 侧以回归 + 门禁自证，不冒充通过（`decision-memo.md` 待确认项 / `tasks.md` Workflow follow-up）。
- **范围外承接** → 兽族 / 双族对抗 / 不死暗夜 / P8.5 深度玩法 / 中文上屏由后续独立 change 承接（C8），联机 / 回放 / AI 玩家 / 迷雾是 1.0 已在能力或范围、不是不做项（C7）；本 change 不因「顺手」扩范围。
