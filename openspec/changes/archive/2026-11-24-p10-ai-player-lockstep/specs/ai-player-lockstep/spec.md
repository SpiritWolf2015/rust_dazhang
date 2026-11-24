# Spec Delta

## Purpose

定义「AI 玩家席位 × 联机 lockstep 接合」的确定性契约：AI 席位的决策在 `w3-core` 内核侧由既有 `run_ai` 各端确定性重算、其决策状态作为权威状态进入状态哈希与快照，联机命令帧流只承载真人玩家命令；使一局 1v1（真人 VS AI）在联机 lockstep session 内可打完、逐帧哈希一致、回放一致。

## ADDED Requirements

### Requirement: 内核各端确定性重算与帧流只承载真人命令（R12.7 / R12.2 / §0.3 T5 / S1）

联机 lockstep session SHALL 启用 `w3-core` 既有 `run_ai` 作为 AI 席位唯一决策源；AI 决策 MUST NOT 作为独立命令注入联机 `CommandBatch`，联机帧流 SHALL 只承载真人玩家命令；各端 SHALL 以相同开局冻结定义与相同外部命令流构造 `RtsMatch`、由 `run_ai` 每帧确定性重算，MUST NOT 旁路写入权威状态（R12.7 / R12.2 / §0.3 T5 / S1 / ADR 0002 / ADR 0005）。

#### Scenario: 各端重算逐帧哈希一致（R12.2 / R12.4 / R12.7）

- **WHEN** 两个联机端点以相同冻结开局定义与同一真人命令帧流推进一局、AI 席位启用 `run_ai`
- **THEN** 每帧 `compute_state_hash()` 相等、`first_difference()` 返回 `None`；联机 `CommandBatch` 只含真人玩家命令、无任何 AI 决策命令、无 `submit_gameplay_command` 本地静默注入（R12.2 / R12.7 / R12.4 / ADR 0002 §5 / ADR 0005 §4）

#### Scenario: 被否方向显式失败（R12.7 / 被否台账 R1 / R2）

- **WHEN** 尝试把 `AiScript` 外部命令经 relay 广播注入联机帧流，或启用「双模式可切换」使两套命令源并存
- **THEN** 联机 session 只保留内核 `run_ai` 单一决策源、该注入路径显式失败或不被接受、不产生双份 AI 命令、逐帧哈希一致不被破坏（`decision-memo.md` P2 已拍板 / R1 / R2 / R12.7 / ADR 0002）

### Requirement: AI 决策状态为权威状态并进入哈希与快照（ADR 0002 / R12.4）

AI 席位决策状态（`AiState`）SHALL 为 `w3-core` 权威状态，复用既有 `SetAi` / `run_ai` 命令面；`ai.enabled` / `next_decision_frame` / `decision_period` / `pending_tasks.len()` MUST 进入 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（当前 8）定义的快照与 `first_difference()`；本期**零新增权威状态字段**为规格门 Q1 推荐口径，若 Q1 拍板确需新增字段 MUST 四处同步（`compute_state_hash` / `Snapshot` 顶层 / `first_difference()` / `restore_snapshot`）并按 ADR 0002 bump 8→9 全套（ADR 0002 §6-8）。

#### Scenario: 状态哈希与 v8 快照覆盖 AI 决策状态（R12.4 / ADR 0002 §6-8）

- **WHEN** 启用 `SetAi{enabled:true}` 推进若干整数帧、捕获 v8 快照、销毁实例恢复后继续推进
- **THEN** `compute_state_hash()` 逐帧一致、`Snapshot` 含 AI 决策状态、`first_difference()` 为 `None`，恢复后 `run_ai` 按 `next_decision_frame` 整数帧周期续算、不重复决策、每帧只消费一次主 LCG（`gameplay.rs:2028-2031` / ADR 0002 §4 / §6-8 / R12.4）

#### Scenario: 新增字段漏进哈希或快照被拦截（ADR 0002 §8 / Q1）

- **WHEN** 按规格门 Q1 结果若确需新增权威状态字段却漏进 `compute_state_hash` / `Snapshot` 顶层 / `first_difference()` / `restore_snapshot` 之一；或零新增口径下误沿用 P8.5e / P8.5a–d 惯性改动版本号 / golden hash
- **THEN** `cargo test -p w3-core --test determinism`（26 项）对应反例断言失败、阻止合并；零新增权威状态字段时 `SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`、golden hash `0x0b53a047b1f8e5e0`、proto tag 46–58 与 `reserved 14 to 32` 保持、不声称逐字节变化（ADR 0002 §6-8 / `CODEBUDDY.md §5` / Q1 / C5）

### Requirement: AI 席位与真人席位对称（R12.7 / R11.4 / R1.1）

AI 席位 SHALL 与真人席位对称——同一 PlayerId 命名空间、同一 `validate_gameplay_command` 校验入口、同一套经济 / 胜负结算，差别只是席位标志；`ClientSession::submit_input` SHALL 把真人玩家普通 / 玩法命令上行同一 `CommandBatch`，MUST NOT 本地静默 `submit_gameplay_command` 注入 AI 命令（R12.7 / R12.2 / R11.4 / ADR 0005 §4）。

#### Scenario: 席位对称与稳定排序（R12.7 / R11.4 / ADR 0002 §3）

- **WHEN** 联机 1v1 以真人席位 + AI 席位装配并推进到结算
- **THEN** 两席位共用同一 PlayerId 命名空间与命令校验入口、`run_ai` 命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、AI 席位不另立 PlayerId 空间、胜负结算按同一稳定顺序只结算一次（R12.7 / R11.4 / ADR 0002 §3 / 参考对照结论 7）

#### Scenario: 禁止本地静默注入（R12.2 / ADR 0005 §4）

- **WHEN** 任一联机端点尝试本地静默 `submit_gameplay_command` 为 AI 席位生成命令而不经 relay 帧流
- **THEN** 该路径被禁止或显式失败、两端 `CommandBatch` 完全一致、逐帧 `compute_state_hash()` 相等、`first_difference()` 为 `None`、不产生 desync（`session.rs:92-93` / R12.2 / ADR 0005 §4 / C4）

### Requirement: 打完一局 1v1 的可执行胜负判据（R11.1 / R11.4 / §0.3 T5 / R12.7）

联机 1v1（真人 VS AI）SHALL 能打完一局并产生唯一胜负；冻结开局外部命令夹具（真人玩家命令流）SHALL 驱动摧毁 AI 席位全部建筑、经既有 `resolve_victory` 结算；`run_ai` 现仅空闲单位采集语义、本期 MUST NOT 注入 `AiScript` 的外部命令 / `Leave` 结算（R11.1 / R11.4 / §0.3 T5 / R12.7）。

#### Scenario: 冻结真人命令流摧毁 AI 全部建筑并回放一致（R11.1 / R11.4 / R12.4 / R11.5）

- **WHEN** 联机 1v1 各端以相同冻结开局定义构造 `RtsMatch`（真人席位 + 启用 `run_ai` 的 AI 席位），注入冻结真人命令流（采集 → 建造 → 训练 → 进攻 → 摧毁 AI 席位全部建筑）逐帧推进到建筑全毁，并以同一命令序列回放
- **THEN** 经既有 `resolve_victory` 产生唯一胜者与结算帧、`victory.winner` 为真人玩家、`victory.resolved == true`、只结算一次且按稳定顺序（R11.1 / R11.4）；两端逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节逐条相等、`first_difference()` 为 `None`；回放逐帧哈希与原局一致（R12.4）；重开后 `victory.resolved == false` 且视图为空（R11.5）；`run_ai` 只按整数帧对空闲单位发 `Gather`、不写回命令队列、不注入重放（`sim_match.rs:3231-3261` / ADR 0002 §3 / §4）

#### Scenario: 胜负不依赖外部脚本的 Leave（R11.1 / R11.3 / R12.7）

- **WHEN** 联机 session 不注入 `AiScript` 的 `Leave` / 进攻命令，仅靠真人命令流与内核 `run_ai` 推进
- **THEN** 胜负仍由 `R11.1` 摧毁全部建筑路径分出、不依赖 `AiScript` 的 `Leave` 结算；`R11.3` 认输 / 时限属本期不定的 C 类边界、不被本期验收依赖；各端 AI 决策状态一致、逐帧哈希一致（P2 / P3 已拍板 / Q4 / R11.1 / R11.3 / R12.7）

### Requirement: R1 可移动性过滤与表现层只读（R1.1 / R1.4 / R13.6 / ADR 0002 §5）

联机对局输入翻译 SHALL 复用 R1.4 可移动性过滤；金矿 / 树木 / 主城 / 建筑 / 资源点等不可移动实体 MUST NOT 生成 `Move`、MUST NOT 改变位置，判据由冻结定义层可移动性数据驱动、MUST NOT 用名字 / 段位 / 白名单；输入翻译与 HUD MUST 为表现层只读派生、不进权威状态 / 哈希 / 快照（R1.1 / R1.4 / R9.1 / R13.6 / ADR 0002 §5）。

#### Scenario: 含建筑选中集合不生成 Move（R1.1 / R1.4 / R9.1 / R13.6）

- **WHEN** 联机对局中含农民 + 主城 + 金矿 / 树木 / 建筑的选中集合对地面下达移动指令，或按经典 `M` 键下达移动
- **THEN** 只为可移动单位生成 `Move`、落点按稳定 `EntityId` 顺序分配、过滤后与「只含可移动单位」的落点逐条相同；建筑 / 资源点 MUST NOT 出现在 `Move` 命令集合中、位置不变；生成的命令经 relay 帧流同步、两端逐帧哈希一致（R1.1 / R1.4 / R9.1 / R13.6 / R12.2 / ADR 0002 §5）

### Requirement: 确定性与反向验证（改坏验红）（R12.4 / ADR 0002 §8 / CODEBUDDY.md §5）

本期确定性回归 SHALL 重跑 `cargo test -p w3-core --test determinism`（26 项）与 `cargo test --workspace`；新增 / 修改回归 MUST 按 `CODEBUDDY.md §5` 形态①（测试内反例 + 元断言防退化）做反向验证，退化实现 MUST 真经生产路径注入、不得冒充形态②（ADR 0002 §8 / R12.4）。

#### Scenario: 反向验证改坏验红（CODEBUDDY.md §5 形态①）

- **WHEN** 以退化实现（AI 命令本地静默注入不经帧流 / 一端不启用 `run_ai` / `AiState` 漏进 `compute_state_hash` 或快照 / `run_ai` 内部命令写回命令队列或注入重放 / 用 `HashMap` 迭代顺序或墙钟 / 含建筑选中集合误生成 `Move`）替换正式实现
- **THEN** 对应断言在缺陷态失败、修复态通过、两条证据可复现；退化实现真经 `capture_snapshot()` / `compute_state_hash()` / `restore_snapshot()` / 公开命令入口 / `step()` 生产路径注入、不建旁路模拟（`CODEBUDDY.md §5` / `openspec/config.yaml:71` / ADR 0002 §8）

#### Scenario: determinism 26 项重跑（ADR 0002 §2-8 / CODEBUDDY.md §5）

- **WHEN** 本期联机 AI 接合改动落盘后重跑 `cargo test -p w3-core --test determinism -- --nocapture`
- **THEN** 26 项全绿、命令排序键 `ExecuteFrame → PlayerId → Sequence → Arrival` / 30 Hz 整数帧 / `SimFixed` Q47.16 / 稳定 `EntityId` 顺序 / FNV-1a / 每帧一次主 LCG 不变；`cargo fmt --all -- --check` / `check_encoding.py` / `openspec validate --strict` 全绿（ADR 0002 §2-8 / `CODEBUDDY.md §5`）

### Requirement: 范围边界与后续承接（S1 / R12.7 / §0.3 T5 / N1–N6）

本期 SHALL 只交付「最小接合」；AI 全量行为（建造 / 训练 / 进攻 / 战斗 / 清野 / 结算）与 C 类边界（`R3.5` / `R5.5` / `R11.3` / `R12.5` / `R13.1` / `R13.5` / `R13.7`）MUST 登记为后续 change / 已知限制、本期不定不扩范围；N1–N6 MUST NOT 纳入、不静默窄化用户 2026-11-14 已拍板的 1.0 范围（S1 / S2 / R12.7 / §0.3 T5）。

#### Scenario: 范围边界如实登记（P3 已拍板 / §14 第 2 条）

- **WHEN** 核对本期 capabilities / tasks 的交付范围与 1.0 范围张力
- **THEN** 本期只含 AI 席位接合 + 打完一局 + 逐帧 / 回放一致；AI 全量行为、C 类边界、N1–N6、联机底座重做均登记为后续 change / 不做、不冒充已实现、不静默改活文档（P3 已拍板 / §14 第 2 条 / S1 / S2 / N1 / R12.7）

#### Scenario: 本地 AiScript 夹具归位（N1 / R12.7 / Q3）

- **WHEN** 本地 `melee` 窗口开发夹具运行既有 16 组合回归
- **THEN** `AiScript`（`SetAi{enabled:false}`）保留为开发夹具、不列 1.0 验收线、既有 16 组合回归不改动；联机 session 与本地夹具 `SetAi` 二值互斥、不双重发命令（P2 已拍板 / Q3 推荐 / N1 / R12.7 / `ai.rs:188-217,238,556`）
