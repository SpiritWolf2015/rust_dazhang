# Proposal

## Why

承接 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 个种族功能做完）/ **S4**（各种道具）的**操作层细化**（`docs/references/WC3-MELEE-RULES.md` §0.1「操作快捷键口径」，不新增范围编号）。用户 2026-11-16 本会话指令「先不人工测，只标记未人工测试然后归档，然后新建后续 change 继续朝 1.0 的最终目标迭代开发」，并在 `AskUserQuestion` 明确选择方向「**R13 操作层与 HUD（推荐）**」。按 `R13.7` 现状差距：现有 `app.rs::KeyboardInput` 是开发夹具期自定义键位（`Q`→Cast / `T`→Train / `C`→CancelTrain / `G`→Gather / `B`→Build / `H`→Stop + `WASD` 相机 + `Z`/`X` 旋转 + `F1`–`F9` 编组 + 数字选组），缺经典 `M` 移动 / `A` 攻击、建造与训练**两级 chords**（`B`→`A`）、命令卡 12 格随选中类型切换、子菜单、英雄 / 物品 / 商店键、`Shift` 队列、`Tab` 子组、自定义键位；主 `rts-command-hud/spec.md` 已定义命令卡 / 生产队列 / 编队的显示与只读契约，但尚未定义经典键位映射与两级 chords（`R13.1` / `R13.2` / `R13.3` / `R13.4` / `R13.7`）。经典键盘 / 命令卡操作层是 1.0 必做（§0.1），且快捷键生成的 `Move` / `Attack` MUST 复用 `R1.4` 分层可移动性过滤（`R13.6`），否则「金矿 / 主城跟着移动」的已知 bug 会从键盘路径复发。本 change 承接已归档 `2026-11-16-p8-human-faction` 的人族六类新 `DefId` 与已归档 `2026-11-16-runtime-entity-movability` 的分层判据，把「输入 → 命令」主入口补齐到经典基线。**N1–N6 一律不纳入**；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」、不是不做项（C7）。

## What Changes

- **命令卡 12 格随选中类型切换**：改为 4 列 × 3 行 = 12 格、由**编译期整数 `DefId` 组织的表现层只读定义表**填充（格位 `slot = y * 4 + x`、「按钮 → 命令 + 快捷键」解耦），随选中集合类型（单位 / 建筑 / 英雄 / 商店）切换；点击按钮与按对应经典键等效，二者 MUST 汇入同一 `input.rs::InputState::apply`（`R13.1` / `R13.6` / C2）。
- **经典键位映射**：`M`→Move、`A`→Attack（仅既有指定目标入口）、`S`→Stop、`G`→Gather、`B`→Build（建造子菜单入口）、`T`→Train、`Esc`→取消子菜单 / 放置态；同一字母含义由**当前卡 + 选中集合**决定、随卡重建「键 → action」表（`R13.2`）。`WASD` 相机与经典 `A` / `S` / `H` 冲突的处置为 `[取舍 / 待核]`，登记见 `design.md` Risks。
- **两级 chords（B→A）**：选中可建造单位按 `B` 打开建造子菜单（命令卡切建筑格），再按第二键选建筑、进入放置态后左键定点（人族 `B`→`A` 英雄祭坛、兵营 `F` 训练步兵等示例）；子菜单栈 / 放置态 MUST 是表现层局部只读状态、`Esc` 清空栈回主卡且不生成命令（`R13.3`）。第二键列表 MUST 由定义层数据驱动、MUST NOT 用建筑名字符串硬编码白名单（`R1.2` / C6）。
- **编队 / 子组 / 队列**：`Ctrl`+数字编组、数字选组、`Tab` 混编同类型子组切换、`Shift`+命令经既有 `CommandKind::Move { append }` 排入队列；全部表现层只读状态、经既有命令面与 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序（`R13.4` / ADR 0002）。
- **键盘路径复用 `R1.4` 分层判据**：键盘 `M` / `A` 生成 `Move` / `Attack` 时 MUST 与 `R1.3` 右键路径走同一输入翻译与命令生成处、MUST 由 `input.rs::effective_movability`（frame-0 装配期 `UnitDef.speed_raw` 只读映射权威 + 运行时动态实体 `EntityView.speed == 0` 判不可移动，两条不互替）过滤；金矿 / 树木 / 主城 / 建筑 MUST NOT 生成 `Move`、MUST NOT 改变位置（`R1.4` / `R13.6` / C3）。
- **BREAKING：无**。既有命令面、既有右键路径、既有编组（`F1`–`F9` 可作可配置别名 `[待核]`）行为不删改；`crates/w3-core/` 零 diff。
- **范围限定（防过度承诺）**：`P` 巡逻 / `H` 保持位置 / `A` 点地面 attack-move 属**新增命令变体**（`command.rs::CommandKind` 无对应变体），MUST 由后续独立 change 承接并按 ADR 0002 复核（新增权威状态 / 命令面须同步哈希 / 快照 / `first_difference()`、跑 determinism、`SNAPSHOT_VERSION` 处置另拍板），MUST NOT 借本 change「只读视图投影 / 实现层约束放松」旁路门禁；`A` 本 change 只归既有 `Attack` 指定目标入口（attack-move `[待核]`），`S`=Stop 与 `H`=Hold 语义不同 MUST NOT 合并、本 change 只做 Stop（`R13.2` / `R13.6` / C4）。

## Capabilities

### New Capabilities

- `command-hotkeys`: 经典键位、命令卡 12 格、两级 chords / 子菜单栈、编队 / 子组 / `Shift` 队列的**表现层只读派生契约**；含「键位 / 命令卡定义表由编译期整数 `DefId` 组织、格位 `slot = y*4+x`、键→action 随卡重建、键盘与命令卡点击汇入同一输入翻译链、复用 `R1.4` 分层过滤、不新增权威状态 / 命令变体」的 ADR 0002 边界。

### Modified Capabilities

- `rts-command-hud`: 增加一条 **ADDED Requirement**——经典键位与命令卡 12 格 / 两级 chords；既有「命令卡与选择面板 / 生产队列可见 / 小地图 / 编队与群体命令 / UI 文案 i18n / HUD 只读派生与不改确定性」六条 requirement 不改写。
- `unit-orders`: 增加一条 **ADDED Requirement**——键盘 `M` / `A` 路径复用 `R1.4` 分层可移动性过滤（含 frame-0 装配实体与运行时动态实体建筑）；既有 requirement（含运行时动态实体过滤、人族新增实体过滤）不改写。

## Impact

- **代码面**：仅 `crates/w3-client/src/{app.rs,input.rs,hud.rs}`、`crates/w3-client/src/melee/{setup.rs,defs.rs}`（表现层定义表 / `EntityKind` 命令卡派生）与 `crates/w3-client/tests/` 下新增 / 扩展 CPU 回归；**`crates/w3-core/` 零 diff**——不新增权威状态字段 / `CommandKind` / `GameplayCommandKind` 变体 / 事件，`snapshot.rs::SNAPSHOT_VERSION` 仍 3，状态哈希字段集合、快照头部布局（`defs_hash` / `content_hash`）、`first_difference()`、`ExecuteFrame → PlayerId → Sequence → Arrival` 排序键、稳定 `EntityId` 顺序、FNV-1a、golden hash 逐字节不变（C1）。`w3-render` / `w3-net` / `w3-relay` / `Cargo.lock` 零改动。
- **确定性契约（ADR 0002）**：键位 / 命令卡 12 格 / 子菜单栈 / 放置态 / 编队 / 选中集合 / `Shift` 队列全为表现层局部只读状态，MUST NOT 进权威状态 / FNV-1a 状态哈希 / `SNAPSHOT_VERSION`（当前 3）快照；键盘与命令卡点击 MUST 汇入同一 `input.rs::InputState::apply`、MUST NOT 新增「命令来源标记」进命令面（C2 / `R13.6`）。`cargo test -p w3-core --test determinism`（26 项）与 `cargo test --workspace` MUST 全绿、`git diff -- crates/w3-core/` 为空、逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致。键盘命令经 lockstep 命令帧同步、回放逐帧哈希一致（`R12.2` / `R12.4` / ADR 0005）。
- **资产 / 法律（ADR 0001 / ADR 0004）**：仓库零资产、零 MPQ 派生数据、零真实导出 JSON、零新增依赖；全为合成 / 原创编译期整数常量或表现层只读映射。**具体经典键位字母（尤其英雄技能、各族建造 / 训练第二键）、grid / `CustomKeys` 支持程度一律 `[待核 / 待用户拍板]`**，以本机导出 SLK 命令卡热键字段或原版客户端核对、不冻结进权威定义、不入库；MUST NOT 照搬 Warsmash rawcode / godot 原创单位名 / 暴雪 abilId（C6）。四个参考项目只读、不复制代码：`WarsmashModEngine`（AGPLv3）仅借鉴语义，见 `design.md`「## 参考项目对照」。
- **反向验证（C5 / `CODEBUDDY.md` §5）**：新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照，参照已归档 `runtime-entity-movability` 的 `tests/movability.rs::degenerate_effective_movability` / P8.0 `degenerate_filter` 先例）；至少覆盖「键盘 `M` / `A` 退化成不过滤 / 把建筑当可移动」与「子菜单 / 键位映射退化 → chords 失败」。仅「全绿」不算完成。
- **ADR 与阶段**：相关 ADR 0001 / 0002 / 0003（范围与分期）/ 0004 / 0005。阶段 **P9**（`docs/roadmap/P8-human-orc-melee.md` §3 / §6 之后的 1.0 操作层收敛）；`openspec/config.yaml` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 / P8.1 一致）。本 change 落在 S1 / S3 / S4 的操作层细化、不新增范围编号；S2（回放）/ S5（野怪）/ S6（中立建筑）不在本 change 交付但属 1.0 范围。
- **范围外承接（C8，后续独立 change）**：兽族完整化 / 双族对抗 / 不死暗夜、P8.5 深度玩法（英雄复活 / 主城升级 / 野怪 / 物品商店 / 迷雾）、中文上屏（CJK 字体许可证人工拍板）、运行时新实体真实模型绑定（模型缺键回退已知限制）、`P` / `H` / attack-move 新命令变体、真窗口人工验收。本 change 不扩范围、不冒充已解决。
- **人工边界**：规格门已由用户 2026-11-16 本会话指令 + `AskUserQuestion` 方向拍板批准（`decision-memo.md` 的 `status: approved`，等价批准、不 self-approve、非 pending）；`critic` **代码门待 apply 完成后复核**，真窗口人工验收**未执行 / 待补验**，如实标注不冒充通过；push / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。
