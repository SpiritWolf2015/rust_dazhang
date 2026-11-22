# Spec Delta

## MODIFIED Requirements

### Requirement: HUD rendered from read-only view

HUD SHALL 只从只读 `WorldView`、事实事件与只读 `Snapshot`（`capture_snapshot()` 公开只读 API）派生绘制选中框、血条、资源、生产队列、（可选）技能栏、小地图与 P8.5e 战争迷雾；MUST NOT 反写模拟状态（ADR 0002 §5）。`WorldView` 字段仅 `tick` / `rng` / `entities`，经济 / 人口 / 生产队列 / `max_hp` 等由 `HudModel::collect` 经只读 `Snapshot` 的 `gameplay` 字段派生；此只读派生不扩展 `WorldView`、不改动 `w3-core`（零改动）、不反写权威状态，也不进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION`（当前 7，含 `defs_hash` 头）定义的快照，权威状态哈希与快照字节保持不变（`tests/interaction.rs` 的 `hud_lifecycle_keeps_state_hash_and_snapshot_bytes` 已验证）。本 change P8.5e 把**三态迷雾网格 / 当前可见 / 已探索层**作为新的表现层本地只读派生状态补进 HUD 只读派生清单：它们是随本地视角一份、跨帧累积的纯派生缓存（对照 `rts_kernel_cs/docs/FRAMEWORK.md:29`「派生缓存恢复后重建」先例），SHALL 与 `revive` / `town_hall` / `creep` / `item_shop` 四个既有只读投影子模型同构（复用 `hud.rs:152,236,354,539` 的 `derive(snapshot, events)` 范式）；MUST NOT 进权威状态 / FNV-1a 状态哈希 / 快照 / `first_difference()`（R10.5 / ADR 0002 §5）。`HudModel` 派生所需位置 / 归属 / 存活 / 帧号全在既有只读 `WorldView` 内且已受现有状态哈希保护，故本期不扩展 `WorldView` / `EntityView` 投影字段、不新增 `def_id` / `sight_radius` / `is_flying`（Q5 推荐 / `runtime-entity-model-binding/spec.md:12,37` 已知限制 / R13.6 `:314`）。`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 保持 7 不 bump、golden hash `0x940cb016f6630f1e` 不变（R10.5 / ADR 0002 §6-8 / Q4）。UI 文案 SHALL 走 i18n（可用中文），MUST NOT 进入日志、权威状态或状态哈希（CODEBUDDY.md §3）。UI 字体 / 图标 MUST NOT 为 MPQ 派生数据或任何暴雪资产，且 MUST NOT 被提交进仓库（ADR 0001）。资源或字体加载失败 MUST 以英文日志明确报告并降级为不绘制对应元素，MUST NOT 改变模拟推进。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。历史 requirement 正文含「`SNAPSHOT_VERSION`（当前 3）」概述口径，按 Q10 推荐如实登记为已知限制、由 `docs-owner` 后续独立 change 统一收口，不在本 delta 内全量改写所有历史硬禁令。

#### Scenario: HUD 只读

- **WHEN** 生产队列、资源或单位生命发生变化，或 P8.5e 迷雾网格 / 当前可见 / 已探索层随逻辑帧刷新
- **THEN** HUD 依据 `WorldView` / 事件刷新显示，迷雾只读投影为表现层本地只读派生、跨帧累积但不反写 `RtsMatch`，且 `compute_state_hash()` 与无 HUD 实例一致、`capture_snapshot()` 字节不含任何 fog / visibility / explored 字段（R10.5 / ADR 0002 §5 / R12.4 / Q9 形态①隔离回归）

#### Scenario: 中文本地化文案

- **WHEN** HUD 显示中文标签
- **THEN** 文案不写入任何 `log` 记录、不进入权威状态、不进入状态哈希；迷雾遮罩 / 可见性标记的表现层呈现同样不进日志 / 权威状态 / 状态哈希（R10.5 / ADR 0002 §5 / `CODEBUDDY.md §3`）

#### Scenario: 资源缺失

- **WHEN** UI 字体或图标缺失
- **THEN** 以英文结构化日志记录错误、对应元素不绘制，且 `crates/w3-render/tests/isolation.rs` 的状态哈希比对仍全绿；迷雾遮罩纹理 / 网格资源缺失同样只降级为不绘制该元素、MUST NOT 改变模拟推进或把降级状态写回权威状态（R10.5 / ADR 0002 §5 / `3d-rendering/spec.md:12-15`）
