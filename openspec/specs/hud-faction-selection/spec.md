# hud-faction-selection Specification

## Purpose
为 P8.4 表现层选族入口定义行为契约：开局前本地族 / AI 族的可见选择（Q1 用户 2026-11-18 拍板**窗口内 egui overlay**）是表现层本地只读状态，经装配期把 `(local_race, ai_race)` 传给既有 `MeleeSetup::with_races` 使开局 DefId 分段正确，承接主 `dual-faction/spec.md` 遗留且不新增权威状态、不触碰 ADR 0002 确定性契约与 1.0 范围 S1 / S3；Q3 已拍板真实两族 glTF（模型绑定契约见 `runtime-entity-model-binding`），本 capability 不涉及 `w3-content` 表项改动。

> 相关 ADR：ADR 0001（选族 UI 字体 / 图标非 MPQ 派生、不入库）；ADR 0002（选族 / HUD 为表现层只读状态，不进哈希 / 快照，`SNAPSHOT_VERSION` 仍 3，`crates/w3-core/` 零改动）；ADR 0003（范围与分期）；ADR 0004（冻结坐标与屏幕坐标边界）；ADR 0005（联机线格式与分层边界）。业务规则依据 `docs/references/WC3-MELEE-RULES.md` §0（S1 / S3）、R1.4、R2、R4、R12.7、R13.1 / R13.3 / R13.6、§14。

## Requirements

### Requirement: 表现层选族入口与装配期传参

系统 SHALL 提供开局前选族 overlay（**4 族：人 / 兽 / 不死 / 暗夜**，Q1' 已拍板不死 + 暗夜双族全量 + P8 两族不重做；`3xx` / `4xx` 段）与 `FactionSelection` 纯表现层只读状态，经 `MeleeSetup::with_races` / `with_faction_selection` / `apply_faction_selection` 派生开局与 `CommandCatalog::for_race` 注入（`faction.rs:28-35,86-103`、`app.rs:330-360`、R2 / S1 / S3 / R13.1）。`i18n`（`i18n.rs:38-50,205-211`）与 overlay / 命令卡目录 SHALL 按表扩展 4 族文案键；选族 MUST 只停表现层本地只读状态、不新增 `w3-core` 分类权威字段、不进状态哈希 / 快照 / `first_difference()`（ADR 0002 §5 / Q6 / Q2 / R13.6）。未选族时 SHALL 回落既有默认人族段（R2 / R12.7）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 选族后开局 DefId 分段正确（R2 / S1 / S3）

- **WHEN** 分别以 4 族（人 / 兽 / 不死 / 暗夜）选族 overlay 确认并装配开局，且未选族时走默认入口
- **THEN** 双方 `Spawn` 关联的 `def_ids` 落在所选族段（`1xx` / `2xx` / `3xx` / `4xx`）、中立恒 `9xx`，4 族命令卡目录 / i18n 中文键 / 模型键段按表命中，`Race` 4 变体不进 `w3-core` 分类字段（Q2 族特色机制以独立顶层权威状态段表达、不复用）；未选族回落人族段（R2 / S1 / S3 / R13.1 / Q1' / Q5' / Q6 / Q2）

#### Scenario: 选族是表现层只读状态、不新增权威字段（ADR 0002）

- **WHEN** 连续切换 4 族选族 / 相机 / HUD 并校验 `RtsMatch` / `Player` 字段集合与 `SNAPSHOT_VERSION`
- **THEN** `FactionSelection` / `faction_overlay` 只在 `crates/w3-client/` 本地只读状态、无 `race` 分类权威字段、选族动作本身逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与不选族基线一致；**历史口径「`SNAPSHOT_VERSION` 仍为 3」不作行为断言**：按用户 Q2 / Q3 已拍板 `SNAPSHOT_VERSION` bump 8、`w3-core` 新增族特色机制顶层权威状态段且四处同步（非空 diff 属预期），选族 / HUD 只读派生不反写（ADR 0002 §5-8 / Q2 / Q3 / Q6 / R13.6）

#### Scenario: 未选族时回落既有默认（R2 / R12.7）

- **WHEN** 既有 `playable` 入口不传种族参数或不触发 overlay 确认
- **THEN** 行为与改动前一致、回落人族段且一局仍能装配并打完（逐帧哈希与 v8 新基线一致），4 族扩展与 Q2 / Q4 机制不改变既有回落路径、不静默改既有测试（R2 / R12.7 / Q3 / Q5' / Q1'）

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

本 change SHALL 按用户已拍板口径做 4 族选族入口与按表扩展、不重做 R13 命令卡 / 快捷键全集；**1.0 范围张力 MUST 如实呈现、不静默窄化、不冒充已解决**：`WC3-MELEE-RULES.md:27`（S3）/ `:34`（§0）/ `:54`（§0.3 T2）/ `:223-226`（R10.2 / R10.3 / R10.4）字面「暗夜夜间隐身 / 昼夜 / 隐身反隐随迷雾纳入 1.0 一并做完」，而 P8.5e 规格门 Q2「最小闭环」曾把它们登记为「后续 change / P9+ 四族承接」（`2026-11-22-p8-fog-of-war/decision-memo.md:80,128`）。**Q4 已由用户 2026-11-22 明确拍板本期含全量（不是 AI 推荐、不是静默窄化）、Q1' 已拍板双族全量、Q4' 已拍板张力保留显式**——暗夜 overlay / 夜间隐身 / 昼夜全量按 Q2 / Q4 本期纳入、不再写「后续 change 承接」；这是已拍板后的如实登记、不是待拍板项。零资产 / `[待核]` 数值不冒充经典（R2 / ADR 0001 / §14 / Q3'）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 承接关系清晰、不重做 R13 命令卡（S1 / S3 / R13.1）

- **WHEN** 校验本 change 的能力边界与 R13 命令卡 / 快捷键的承接关系
- **THEN** 4 族只经按表扩展命令卡目录 / i18n / overlay，不重写 R13 全集、不把 Q2 / Q3 新增 `GameplayCommandKind` 码 24 起塞进快捷键白名单；Q4 已拍板本期含全量机制（R10.2 / R10.3 / R10.4 / R5.6 / S3 / R8.4）按 Q2 权威状态 / Q1' 双族全量纳入、不再登记为「后续 change 承接」，但 1.0 字面「一并做完」与本期双族全量的张力按 Q4' 已拍板七处显式如实登记、不预设结论、不静默窄化 1.0 范围（S1 / S3 / R13.1 / R10.2-5 / Q1' / Q4'）

#### Scenario: 零资产入库与 `[待核]` 不冒充经典（R2 / ADR 0001 / §14 / Q3 拍板）

- **WHEN** 在干净环境运行 4 族选族装配并核对常量落点
- **THEN** 零文件 IO / 零资产入库，4 族文案 / 数值为 `[待核]` 合成原创常量、不冒充原版；按 Q2 / Q3 已拍板 `SNAPSHOT_VERSION` bump 8、族特色机制冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部，两套昼夜常量口径未统一与全量机制 B 类 `[待核]` 数值按 Q3' 已拍板由人核对（实施期数值核对、非规格门待拍板范围 / 架构 Q）、不静默改规则（R2 / ADR 0001 / ADR 0004 / §14 / Q2 / Q3 / Q3'）

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
