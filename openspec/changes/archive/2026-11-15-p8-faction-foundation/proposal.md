# Proposal

## Why

P8 路线图（`docs/roadmap/P8-human-orc-melee.md`，用户 2026-11-14 拍板「人族 + 兽族做完」且含 P8.5 全量）要求两族完整化。当前 `crates/w3-client/src/melee/defs.rs:16-27` 只有 6 个 `DefId` 且无种族维度，双方共用同一套定义；`melee-playable-loop` 归档时真窗口验收还留下「右键群体 `Move` 把主城 / 金矿也一起移动」的已知限制（见 `openspec/changes/archive/2026-11-14-melee-playable-loop/design.md` Risks 末条）。P8.0 是两族地基：先修该缺口，再引入**定义层**种族维度，为 P8.1 人族 / P8.2 兽族完整化铺路，且不破坏 ADR 0002 确定性硬门禁。

## What Changes

- 修「群体 `Move` 未过滤建筑 / 资源点」：在装配期从 client 侧冻结定义 `FrozenDefinitions::units()` 的 `UnitDef.speed_raw` 派生一份 `EntityId → 可移动性` 的**表现层只读映射**，经 `crates/w3-client/src/app.rs` 注入输入翻译层；`crates/w3-client/src/input.rs` 的群体 `Move` 生成处按该映射过滤，建筑 / 金矿 / 树木 MUST NOT 生成 `Move` 命令、MUST NOT 改变位置。MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时实体 `speed`、MUST NOT 回退到建筑类型白名单；补针对性回归（含反向验证）。
- 引入**定义层**种族维度：`crates/w3-client/src/melee/defs.rs` 的 `DefId` 按种族分段（人族段 / 兽族段 / 中立资源段），`FrozenDefinitions` 仍为编译期整数常量；`MeleeSetup` 开局按种族参数生成对应 `DefId` 的 `Spawn` 命令。
- **MUST NOT** 新增运行时权威状态字段（如 `Player.race`）：种族选择只体现在开局 `Spawn` 的 `DefId` 上，从而 `crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍为 3、状态哈希字段集合与快照头部布局不变（主 `openspec/specs/melee-match/spec.md:127-129` 已把「内核零改动」写成 requirement）。
- 新增 `faction-definitions` capability 的 delta spec，并给 `unit-orders` 增加一条「群体移动只作用于可移动单位」requirement。
- **BREAKING：无**。既有 `playable` / `melee` 入口与合成定义在无种族参数时回落人族段，行为不变；不删改既有公开签名。

## Capabilities

### New Capabilities

- `faction-definitions`: 冻结定义按种族分段的 DefId 契约、开局按种族装配，以及「种族选择不引入运行时权威状态」的确定性边界。

### Modified Capabilities

- `unit-orders`: 增加「群体移动只作用于可移动单位」的要求（ADDED Requirement），既有「命令校验 / 确定性移动积分 / 同帧攻击结算」三条 requirement 不改写。

## Impact

- **代码面**：`crates/w3-client/src/app.rs`（判据映射注入与 `apply_input()` 调用点）、`crates/w3-client/src/input.rs`（`apply()` 签名与过滤）、`crates/w3-client/src/melee/defs.rs`、`crates/w3-client/src/melee/setup.rs` 及对应测试；**`crates/w3-core/` 零改动**（`UnitDef` / `FrozenDefinitions` / `WorldView` / `EntityView` / `CommandKind::Move` 均不修改）。
- **API / 依赖**：无新增依赖；`FrozenDefinitions::new` 契约、`CommandKind` 线格式、`SNAPSHOT_VERSION` 均不变。
- **确定性（ADR 0002）**：不新增权威状态字段；`defs_hash` 天然覆盖分段定义差异；`cargo test -p w3-core --test determinism`（26 项）、`isolation`（3 项）、`cargo test --workspace` MUST 全绿；逐帧状态哈希与 `capture_snapshot()` 字节与基线一致。
- **资产与法律（ADR 0001）**：仓库零资产；两族定义全为合成 / 原创编译期整数常量，真实导出数据仍只作表现层接缝、不入库；只读参考项目不复制代码（Warsmash AGPLv3 仅借鉴语义）。
- **阶段与范围**：P8.0（`docs/roadmap/P8-human-orc-melee.md` §3；`openspec/config.yaml:48` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注）。本 change 不承接、由后续独立 change 承接：两族完整单位 / 建筑 / 英雄 / 技能（P8.1 / P8.2）、AI 按种族（P8.3）、表现层模型键扩充（P8.4）、**P8.5 五个深度玩法 change（必做全量）**。
- **人工边界**：本 change 的修复目标在真窗口表现为「右键只移动可移动单位」；真窗口人工验收**不在本轮执行**，如实标注「未执行 / 待补验」，CPU 侧以回归测试 + 门禁自证。
