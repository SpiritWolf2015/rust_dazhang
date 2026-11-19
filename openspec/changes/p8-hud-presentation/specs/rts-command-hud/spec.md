# Spec Delta

## ADDED Requirements

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
