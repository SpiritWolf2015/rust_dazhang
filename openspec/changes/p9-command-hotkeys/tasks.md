# Tasks

> 实现由 `client-owner` 承担（`crates/w3-client/`、`crates/w3-render/` 唯一实现所有者），守 ADR 0002 与只读表现边界；`crates/w3-core/` 零 diff。`spec-steward` 只成文、不写实现代码。涉及确定性契约改动的验收含重跑 `cargo test -p w3-core --test determinism`（`CODEBUDDY.md` §5 / `rules.tasks` 第二条）；新增 / 修改玩法回归 MUST 含反向验证（改坏验红，`CODEBUDDY.md` §5 / `rules.tasks` 第三条）。真窗口人工验收未执行 / 待补验，如实标注、不冒充通过。

## 1. 表现层定义表与命令卡 12 格（D1 / D2，R13.1 / R1.2）

- [x] 1.1 在 `crates/w3-client/src/melee/defs.rs`（或新建表现层模块）定义「按钮 → 命令 + 快捷键 + 格位」只读定义表，由编译期整数 `DefId` 组织、格位 `slot = y * 4 + x`（4 列 × 3 行 = 12 格），全为合成 / 原创编译期常量、零文件 IO（`R13.1` / `R1.2` / C6）；验证：`cargo test -p w3-client` 新增单测断言 12 格、`slot` 公式、键位与命令解耦、具体键位标注 `[待核]` 且不冻结进 `FrozenDefinitions`
- [x] 1.2 改 `crates/w3-client/src/hud.rs::derive_commands` 从「按 `EntityKind` 线性 push」改为「按选中集合类型 + 装配层 `EntityId → DefId` 只读关联查定义表产出 12 格」，MUST NOT 用建筑名字符串白名单（`R13.1` / `R1.2` / D2）；验证：新增单测断言农民卡 / 生产者卡 / 空选中三类 12 格内容不同、判据来自定义表而非 `EntityKind` match
- [x] 1.3 改 `crates/w3-client/src/hud.rs::draw_with_minimap` 的命令卡渲染为 4×3 网格（12 格、含空格位），标签走既有 `i18n` 边界（`R13.1` / `R13.6`）；验证：CPU 侧单测断言网格落格与标签文本不进日志 / 哈希，`cargo test -p w3-client` 全绿
- [x] 1.4 补 `crates/w3-client/src/melee/setup.rs::EntityKind` 与 `defs.rs::Race::category_def_ids` 的只读查表接线，使 P8.1 人族 6 类新 `DefId` 的命令卡由定义表派生（不扩 `EntityKind` 白名单，D2 / Risks）；验证：单测断言 6 类新 `DefId` 各有定义表条目、运行时动态实体无 `EntityKind` 标签时仍能按只读查表派生

## 2. 经典键位映射与同一输入翻译链（D1 / D6，R13.2 / R13.6）

- [x] 2.1 在 `crates/w3-client/src/input.rs::InputAction` 补经典键动作（`M`→Move 瞄准 / `A`→Attack 瞄准 / `S`→Stop / `Esc`→取消子菜单；`G` / `B` / `T` 复用既有动作语义），键位字母为合成配置、`[待核]`（`R13.2` / `R13.7`）；验证：`cargo test -p w3-client` 单测断言每键归一化为预期 `InputAction`、无新增 `CommandKind` / `GameplayCommandKind` 变体
- [x] 2.2 改 `crates/w3-client/src/app.rs::KeyboardInput` 键位映射，把经典键与命令卡点击都回传为同一 `InputAction`、汇入同一 `input.rs::InputState::apply`（键盘经 `report_input`、点击经既有 `pending_input_action`，D1 / C2）；验证：CPU 单测对同一选中集合分别模拟「按键」与「点击对应格」，断言产出逐字段相同的 `ClientCommand`、不携带命令来源标记
- [x] 2.3 处置 `WASD` 相机与经典 `A` / `S` / `H` 冲突：不静默改相机键位，把「相机改绑 / 经典键修饰键 / `F1`–`F9` 是否保留为别名」显式登记为 `[待核 / 待用户拍板]` 并写入 `decision-memo.md` 待确认项（D6 / `R13.7`）；验证：读代码确认无相机键位删除、`decision-memo.md` 含该待确认项
- [x] 2.4 断言 `S`=Stop 与 `H`=Hold 不合并、`A` 只归既有 `Attack { target }` 指定目标入口（attack-move `[待核]` 承接，D7 / `R13.2`）；验证：单测断言 `S` 只产出既有 `Stop`、`A` 只产出既有 `Attack`、无 Hold / Patrol / AttackMove 新命令变体

## 3. 两级 chords 子菜单栈与放置态（D3，R13.3 / R13.6）

- [x] 3.1 在 `crates/w3-client/src/input.rs::InputState` 增「子菜单栈」（当前卡 ID 序列）与「放置态」（待定点命令）两类表现层局部字段，中文注释显式声明不进权威状态 / 哈希 / 快照（D3 / C2 / ADR 0002）；验证：`cargo test -p w3-client` 单测断言两类字段只存在于 `InputState`、`compute_state_hash()` / `capture_snapshot()` 不因栈变化而改变
- [x] 3.2 实现 `B` 开建造子菜单、第二键选建筑转放置态、左键定点才产出既有 `GameplayCommandKind::Train { is_building: true }`；第二键列表由定义表记录驱动、MUST NOT 建筑名白名单（人族 `B`→`A` 祭坛 / 兵营 `F` 步兵为示例合成键位、`[待核]`，`R13.3` / `R1.2`）；验证：CPU 单测断言 `B`→第二键→定点产出预期既有 `Train`、`Esc` 前不生成任何命令
- [x] 3.3 实现 `Esc` 清空子菜单栈回主卡 / 取消放置态且不生成命令、换卡 `clear()` 后重建「键 → action」表（`openMenu(0)` 语义，D3）；验证：单测断言 `Esc` 后卡回主卡、命令序列为空、状态哈希 / 快照字节不变
- [x] 3.4 实现「同一字母随当前卡 + 选中集合决定含义」（`G` / `B` 多义），换卡重建键表、旧卡绑定失效（D1 / D4 / `R13.2`）；验证：单测在农民主卡 / 建造子卡 / 商店卡按下同一字母，断言翻译为不同既有动作

## 4. 编队 / 子组 / Shift 队列（D4，R13.4 / ADR 0002）

- [x] 4.1 实现 `Ctrl`+数字编组（写 `InputState::groups`）、数字按组选择、`Tab` 混编同类型子组切换（表现层游标），全部表现层只读状态（D4 / `R13.4`）；验证：单测断言编组 / 选组 / `Tab` 只改选中集合、`compute_state_hash()` 不变、编队允许含建筑但命令分发仍按 `R1.4` 过滤
- [x] 4.2 实现 `Shift`+命令排队：逐条经既有 `CommandKind::Move { append: true }` 入队、按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、不旁路写入、不改排序键（D4 / `R13.4` / ADR 0002）；验证：单测断言 `Shift` 连续落点产出逐条 `Move { append: true }`、无新命令变体、golden hash 口径不变
- [x] 4.3 断言编队 / 子菜单 / 放置态 / `Shift` 队列局部状态全部不进权威状态 / 状态哈希 / `SNAPSHOT_VERSION`（当前 3）快照（C2 / D8）；验证：`cargo test -p w3-client` 单测比对「有 / 无这些局部状态」两实例的逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致

## 5. 键盘 M / A 路径复用 R1.4 分层过滤（D5 / C3，R1.4 / R13.6，最关键）

- [x] 5.1 把键盘 `M` / `A` 归一化为与 `R1.3` 右键地面 / 敌方目标**同一**输入翻译与命令生成路径（`input.rs::InputState::apply`），复用既有 `input.rs::effective_movability` 分层判据（frame-0 装配期 `UnitDef.speed_raw` 只读映射权威 + 运行时动态实体 `EntityView.speed == 0` 判不可移动，两条不互替）、`filter_movable` 与稳定 `EntityId` 落点分配；MUST NOT 扩展视图 / 用运行时 `speed` 替代装配映射 / 回退建筑类型白名单（D5 / C3）；验证：读代码确认键盘分支与右键分支共用 `effective_movability` 调用处、`crates/w3-core/` 无改动
- [x] 5.2 补 `crates/w3-client/tests/` 键盘路径 R1.4 回归：frame-0 混合集合（农民 + 主城 + 金矿）按 `M` 只为农民生成 `Move`、主城（建筑）与金矿（资源点）MUST NOT 出现在 `Move` 生成集合（`R1.4` / `R13.6`）；验证：单测断言 `Move` 生成集合只含农民、落点与「只含可移动单位」逐条相同、逐帧状态哈希一致
- [x] 5.3 补运行时动态实体键盘路径回归：用真实 `RtsMatch` + `Train { is_building: true }` / `Build` 产出建筑（`EntityView.speed == 0`）与农民一起框选按 `M`，断言该运行时建筑不出现在 `Move` 生成集合、产出的战斗单位（`speed > 0`）正常生成 `Move`（`R13.6` / §14 / 已归档 `runtime-entity-movability` 先例）；验证：`cargo test -p w3-client --test <name>` 真 A-B 视图 / 命令断言全绿
- [x] 5.4 补全为不可移动实体按 `M` 不生成 `Move`、有明确「无可执行目标」反馈位（表现层只读、不进哈希 / 快照）的回归（`R1.4`）；验证：单测断言命令序列为空、状态哈希不变、反馈位可读
- [x] 5.5 补 `A` 生成既有 `Attack { attacker, target }` 且复用同一 `apply`、不出现 attack-move 点地面途中交战语义（`[待核]` 承接，`R13.2` / `R13.6` / C4）；验证：单测断言 `A` 产出既有 `Attack`、无新命令变体

## 6. 反向验证（改坏验红）与 CPU 回归（D9 / C5，CODEBUDDY.md §5）

- [x] 6.1 补「键盘路径过滤退化」反例元断言 / A-B 对照：以「不过滤 / 把建筑当可移动 / frame-0 误用视图 `speed`」的退化实现替换正式实现，对含建筑 / 金矿 / 主城的混合选中集合按 `M`，断言「不可移动实体 MUST NOT 出现在 `Move` 生成集合」在缺陷态失败、修复态通过（参照已归档 `runtime-entity-movability` 的 `tests/movability.rs::degenerate_effective_movability` / P8.0 `degenerate_filter`，D9 / C5）；验证：保留缺陷态 `cargo test` 变红的命令输出与修复态全绿输出，元断言证明退化解等价该缺陷
- [x] 6.2 补「子菜单 / 键位映射退化」反例：把子菜单栈 / 键→action 表退化为全局静态映射或不清栈，断言 `B`→`A` 两级 chords 与「同一字母随卡切换」在缺陷态失败、修复态通过（`R13.3` / C5）；验证：保留 A-B 命令输出、两条断言在缺陷态失败 / 修复态通过
- [x] 6.3 补键→命令表、12 格 `slot = y*4+x`、chords、编队 / `Tab` / `Shift` 队列、i18n 文案不进日志 / 哈希的 CPU 回归（`R13.1`–`R13.4` / `R13.6`）；验证：`cargo test -p w3-client` 全绿、文案刷新不改 `compute_state_hash()` 与 `capture_snapshot()` 字节
- [x] 6.4 补状态哈希 / 快照 / `SNAPSHOT_VERSION=3` / golden hash 不变的回归：在一局中连续切换命令卡 / 子菜单 / 编队 / `Shift` 队列并下达经典键命令后逐帧比对基线（D8 / C1）；验证：逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与基线一致、`SNAPSHOT_VERSION` 仍 3

## 7. 确定性门禁、编码、格式与代码门（D8 / C1 / C2）

- [x] 7.1 重跑 `cargo test -p w3-core --test determinism`（26 项，涉及 `w3-core` 契约相邻改动按 `CODEBUDDY.md` §5 / `rules.tasks` 第二条）与 `cargo test --workspace`，确认 golden hash 逐字节不变、`SNAPSHOT_VERSION` 仍 3（C1 / D8）；验证：两条命令全绿、保留输出
- [x] 7.2 运行 `git diff -- crates/w3-core/` 确认为空（内核零 diff，C1）；验证：命令无输出；并用 `git status` 确认未改 `crates/w3-core/` / `tools/` / `.github/` / `CODEBUDDY.md` / `openspec/config.yaml` / `.codebuddy/agents/` / 主 `openspec/specs/` / `docs/`
- [x] 7.3 运行 `cargo fmt --all`（写完自动排版）与 `cargo fmt --all -- --check`（提交前校验）、`python tools/check_encoding.py`、`python tools/check_source_conventions.py`、`git diff --check`（`CODEBUDDY.md` §2 / §3 / §9）；验证：全部通过、源码注释与提交信息描述中文 / 日志英文 / UTF-8 无 BOM / LF
- [x] 7.4 运行 `openspec validate p9-command-hotkeys --strict` 确认 `is valid`、无新增 WARNING（`CODEBUDDY.md` §8）；验证：命令输出 `is valid`
- [ ] 7.5 委派只读 `critic` 执行**代码门**全清单复审全部 diff（确定性零冲击 / 所有权 / 许可证 / 编码 / 资产入库 / 第 7·10 项四件套一致性 / 第 11 项③ 反向验证真 A-B / 第 12 项参考对照逐锚 / 第 13 项 `R-x.y`），结论回报前不冒充通过（`CODEBUDDY.md` §9 / `decision-memo.md` 待确认项）；验证：`critic` 报告 blocker 0、`decision-memo.md` 回填代码门结论
- [x] 7.6 归档相关：确认 `openspec validate p9-command-hotkeys --strict`、`decision_memo_check()` / `spec_gate_check()` 两项机械校验为 True、指纹未失配（规格锁定未解锁）；验证：`python -B -c "import sys; sys.path.insert(0,'tools'); import verify_all as v; print(v.decision_memo_check()); print(v.spec_gate_check())"` 输出两个 True

## Workflow follow-up

- 真窗口人工验收（经典键手感 / 命令卡 12 格布局 / 两级 chords 定点 / 编队 `Tab` / `Shift` 队列）**未执行 / 待补验**，不冒充通过；由人工在 GUI 侧另行验收（承接 `feedback` GUI 修复先自测再请人工验收、先写足 CPU 回归）。
- 具体经典键位字母（英雄技能 / 各族建造训练第二键）、grid / `CustomKeys` 支持程度、`WASD` 相机冲突处置、`P` / `H` / attack-move 新命令变体承接，均 `[待核 / 待用户拍板]`、由后续独立 change 承接（C4 / C6 / C8 / `R13.7`）。
- 规格门已由用户 2026-11-16 本会话指令 + `AskUserQuestion` 方向拍板批准（`status: approved`）；本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀），push / fetch / pull / clone 及任何远端同步人工独占（`CODEBUDDY.md` §4 / §10）；归档后确认主 `openspec/specs/` 新增 `command-hotkeys` capability、`rts-command-hud` / `unit-orders` 各并入 ADDED Requirement。
