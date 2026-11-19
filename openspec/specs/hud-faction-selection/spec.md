# hud-faction-selection Specification

## Purpose
为 P8.4 表现层选族入口定义行为契约：开局前本地族 / AI 族的可见选择（Q1 用户 2026-11-18 拍板**窗口内 egui overlay**）是表现层本地只读状态，经装配期把 `(local_race, ai_race)` 传给既有 `MeleeSetup::with_races` 使开局 DefId 分段正确，承接主 `dual-faction/spec.md` 遗留且不新增权威状态、不触碰 ADR 0002 确定性契约与 1.0 范围 S1 / S3；Q3 已拍板真实两族 glTF（模型绑定契约见 `runtime-entity-model-binding`），本 capability 不涉及 `w3-content` 表项改动。

> 相关 ADR：ADR 0001（选族 UI 字体 / 图标非 MPQ 派生、不入库）；ADR 0002（选族 / HUD 为表现层只读状态，不进哈希 / 快照，`SNAPSHOT_VERSION` 仍 3，`crates/w3-core/` 零改动）；ADR 0003（范围与分期）；ADR 0004（冻结坐标与屏幕坐标边界）；ADR 0005（联机线格式与分层边界）。业务规则依据 `docs/references/WC3-MELEE-RULES.md` §0（S1 / S3）、R1.4、R2、R4、R12.7、R13.1 / R13.3 / R13.6、§14。

## Requirements

### Requirement: 表现层选族入口与装配期传参

系统 SHALL 提供开局前可见的本地族 / AI 族选择入口（**Q1 已拍板：窗口内 egui overlay**，R2 / R12.7），其结果是**表现层本地只读状态**；确认后 SHALL 把 `(local_race, ai_race)` 经装配期传给既有 `MeleeSetup::with_races`，使本地席位实体 `def_ids` 落 `local_race` 段、AI 席位落 `ai_race` 段、中立资源恒 `9xx`（R2）。系统 MUST NOT 新增 `Player.race` / `RtsMatch.race` 或任何权威状态字段；选族状态 MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照（ADR 0002）。以上全部只读派生，不改写 `w3-core` 命令面。

#### Scenario: 选族后开局 DefId 分段正确（R2 / S1 / S3）

- **WHEN** 在表现层分别选择人族 / 兽族为本地族并装配开局
- **THEN** 本地席位实体 `def_ids` 落在 `1xx`（人族）或 `2xx`（兽族）段、AI 席位落在所选 `ai_race` 段、中立金矿 / 树木恒 `9xx`；`MeleeSetup::new()` / `with_race` 既有入口的 `Spawn` / `gameplay_commands` 字节逐字节不变（R12.7 / C2）

#### Scenario: 选族是表现层只读状态、不新增权威字段（ADR 0002）

- **WHEN** 校验选族装配后的权威状态字段集合与 `SNAPSHOT_VERSION`
- **THEN** 无 `race` / `ai_race` 或任何新增权威状态字段，`SNAPSHOT_VERSION` 仍 3，选族 / HUD 刷新不进状态哈希 / 快照 / `first_difference()`；`git diff -- crates/w3-core/` 为空（ADR 0002 §6-8 / C1）

#### Scenario: 未选族时回落既有默认（R2 / R12.7）

- **WHEN** 未作任何选族直接开局
- **THEN** 回落 `MeleeSetup::new()`（本地人族 / AI 兽族），`race_does_not_change_authoritative_spawn_commands` 等既有回落基线测试仍绿、不静默放宽断言（§14）

### Requirement: 确定性契约零冲击与内核零 diff

本 capability SHALL 只经 `w3-core` / `w3-client` 既有公开 API 与装配层只读参数实现；`crates/w3-core/` MUST 零改动，30 Hz 整数帧、命令稳定排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、稳定 `EntityId` 顺序、FNV-1a 状态哈希字段集合、快照头部布局、`first_difference()`、golden hash 逐字节不变（ADR 0002）。本 change **不新增权威状态**，故无需扩展哈希 / 快照字段集合；若将来新增权威状态字段 MUST 同步进入状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照并按 `CODEBUDDY.md` §5 跑 `determinism`。

#### Scenario: 状态哈希 / 快照 / golden hash 不变（ADR 0002）

- **WHEN** 以任一选族组合装配并推进多帧，与同脚本 CPU 基线比对
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节一致、golden hash 逐字节不变、`SNAPSHOT_VERSION` 仍 3、快照头部布局（`defs_hash` / `content_hash`）不变；`cargo test -p w3-core --test determinism`（26 项）与 `crates/w3-render/tests/isolation.rs` 全绿；Q3 的 `w3-content::ENTITY_MODEL_FILES` 表项新增是纯表现层、不进 `defs_hash` / `content_hash`（`melee.rs:38` 固定 `CONTENT_HASH`）（C1）

#### Scenario: 选族 / 相机 / HUD 连续变化不改哈希（ADR 0002 / R13.6）

- **WHEN** 连续切换选族、选中集合、相机与 HUD 文字层并推进若干整数逻辑帧
- **THEN** 逐帧状态哈希与捕获的快照字节与无 HUD 实例一致，帧号与 30 Hz 基准一致，`Resized` 不触发 `step()`（主 `rts-command-hud/spec.md:87` / `playable-window-entry/spec.md:52/:137`）

#### Scenario: 权威状态遗漏被门禁拦截（ADR 0002）

- **WHEN** 假设将来把选族字段转为权威状态却未加入哈希或由 `SNAPSHOT_VERSION` 定义的快照
- **THEN** `cargo test -p w3-core --test determinism` 失败；本 change 不新增权威字段，故无需扩展哈希或快照字段集合（主 `playable-window-entry/spec.md:59-62`）

### Requirement: 范围边界与承接 P8.3 遗留

本 capability SHALL 承接主 `openspec/specs/dual-faction/spec.md:106/:108-111` 明确移交的「HUD 选族可见 / 选族 UI」与「真窗口人工验收」遗留；命令卡按族分派落在 `rts-command-hud` 的 ADDED Requirement、Q3 真实两族 glTF 模型绑定落在 `runtime-entity-model-binding`（含 `w3-content::ENTITY_MODEL_FILES` 表项新增）。中文上屏 / CJK 字体、JASS / wtg VM（N5）、联机底座重做、真窗口人工验收的替代、P8.5 五个深度玩法子系统、不死 / 暗夜两族（P9+）MUST NOT 纳入本 change（§0.2 N1–N6 / §14 / R13.6 范围限定）。Q2 已拍板：选族相关键位 / 图标语义一律 `[待核]`、不冻结进 `FrozenDefinitions`、不入库（ADR 0001 / ADR 0004）。

#### Scenario: 承接关系清晰、不重做 R13 命令卡（S1 / S3 / R13.1）

- **WHEN** 核对本 change 与 `docs/roadmap/P8-human-orc-melee.md §3 P8.4`、`dual-faction/spec.md:106/:108-111` 的承接关系
- **THEN** 仅交付表现层选族入口、命令卡按族分派、运行时实体真实 glTF 模型绑定三项；既有 `command-hotkeys` / `rts-command-hud` 的 12 格定义表 / 键位解耦 / 两级 chords 契约不改写，不冒充已解决范围外项（C5 / C6 / Q1–Q4 已拍板）

#### Scenario: 零资产入库与 `[待核]` 不冒充经典（R2 / ADR 0001 / §14 / Q3 拍板）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境运行**选族装配**路径
- **THEN** 选族装配零文件 IO、零新增定义 / 依赖 / 资产入库，两族差异是既有 `[取舍]` 合成原创常量相对大小，不复制参考项目单位名 / 数值 / rawcode、不冒充经典事实；Q3 真实 glTF 一律留 gitignored 本机、不入库（`w3-content` 仅 `ENTITY_MODEL_FILES` 表项新增，见 `runtime-entity-model-binding`）；N1–N6 不纳入，联机 / 回放 / AI 玩家 / 迷雾是已在能力或 1.0 范围、不是不做项

### Requirement: CPU 回归与反向验证

新增 / 修改的表现层选族回归 MUST 证明缺陷态失败、修复态通过（`CODEBUDDY.md` §5），证据形态用「测试内反例元断言」或 A-B 对照；退化实现 MUST 真经 `MeleeSetup::with_races` / 装配路径注入、非旁路模拟。至少覆盖「选族结果被忽略、退化回硬编码 `MeleeSetup::new()`」一类退化；仅「全绿」不算完成。

#### Scenario: 选族退化回硬编码默认即失败（CODEBUDDY.md §5 / C11）

- **WHEN** 把表现层选族传参退化为忽略 `local_race` / `ai_race`、继续用 `MeleeSetup::new()`
- **THEN** 「本地席位 `def_ids` 落所选族段」断言在缺陷态失败、正式实现通过，两条证据可复现；既有回落基线测试不静默放宽

#### Scenario: 确定性回归与门禁全绿（ADR 0002）

- **WHEN** 运行选族相关 CPU 回归、重跑 `tests/determinism.rs` 与 `crates/w3-render/tests/isolation.rs`
- **THEN** 全部通过，`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍 3、逐帧哈希与快照字节与基线一致（C1）

### Requirement: 真窗口人工验收如实标注未执行（Q4 已拍板）

系统 SHALL 提供可逐项勾选的真窗口人工 GPU 冒烟清单（MUST 至少覆盖：能看到并操作选族入口（Q1 窗口内 egui overlay）、能框选、右键只移动可移动单位（R1.4）、命令卡按族、运行时新实体呈现 Q3 真实 glTF 网格而非四面体、能打完一局并看到 `VICTORY`、1920×1080 窗口、HUD 文字以 ASCII 正常显示），清单项 MUST 标注为**只能人工在有 GPU 与显示设备的桌面执行**。本 change 阶段 MUST 如实标注「未执行 / 待人工」、不冒充通过、不提供替代；离屏自证 MUST NOT 覆盖 egui overlay（路线图 :117 / Q4 用户 2026-11-18 拍板）。

#### Scenario: 先离屏自证 + CPU 回归全绿再请人工验收（C12）

- **WHEN** 交付表现层选族改动并准备请用户人工验收
- **THEN** 先以 `render_headless_rgba` 离屏 readback + `ViewImage` 自证 CPU 可观测部分（本机有资产时含真实网格）、CPU 回归全绿，再请用户人工验收；不自测好就让用户在真窗口反复试

#### Scenario: 人工边界与无 GPU 环境（C12）

- **WHEN** 在无 GPU / 无显示设备的 headless 或 CI runner 上尝试真窗口清单项
- **THEN** 明确报告「未执行 / 待人工」、不静默跳过、不改判为通过；清单未逐项勾选前不得归档（主 `playable-window-entry/spec.md:168-171`）

#### Scenario: CI 边界（C12 / C15）

- **WHEN** CI 在 `ubuntu-latest` / `windows-latest` 跑 `python tools/verify_all.py`
- **THEN** 只覆盖编译与 CPU 测试，MUST NOT 声称 CI 已验证 GPU 窗口链路或真实网格画面；规格门 `status: pending`（Q3 与 critic 推荐不同、§8 重走规格门，待用户第二次拍板）时 `spec_gate_check()` 变红是 fail-closed 正确信号，不绕过、不改门禁（主 `playable-window-entry/spec.md:173-176` / C15）
