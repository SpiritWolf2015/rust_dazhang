# Tasks

> 按 `design.md` D1–D6 分组；本 change 首次动 `crates/w3-core/`，但只加只读视图字段、ADR 0002 零冲击（golden hash 逐字节不变、`SNAPSHOT_VERSION` 仍 3），零资产（ADR 0001 / `CODEBUDDY.md` §5）。每组自带验证与证据要求；玩法任务引用 `docs/references/WC3-MELEE-RULES.md` 的 `R-x.y` 与 1.0 范围 S1 / S3；锚一律「路径 + 符号名」、不写 `:NN` 行号。

## 1. `w3-core` 只读投影 `UnitState.speed`（D1）

- [x] 1.1 在 `sim_match.rs::EntityView` 加只读 `pub speed: i64`，在 `sim_match.rs::read_view` 投影**已存在**的 `UnitState.speed.raw()`；字段 rustdoc / 注释中文、说明「只读派生、不进哈希 / 快照」；验证：`cargo check -p w3-core` 通过、`read_view` 其余字段与 `WorldView` 结构不变、`EntityView`「修改不影响模拟」语义不变（ADR 0002 §5）
- [x] 1.2 给 `EntityView` **全部构造点**补 `speed` 字段：`sim_match.rs::read_view`、`hud.rs`、`tests/interaction.rs`、`tests/movability.rs`、`w3-render/src/model.rs`、`w3-render/src/scene.rs` 及任何新增构造点（先 `grep -rn "EntityView {"` 全仓核实，不漏一处）；验证：`cargo check --workspace` 全绿、无遗漏构造点、测试构造值用合成原创 `[待核]` 常量（C6 / `CODEBUDDY.md` §2）
- [x] 1.3 **重跑确定性 + golden hash 逐字节比对**：`cargo test -p w3-core --test determinism -- --nocapture`，与改动前 golden hash / `compute_state_hash()` / `capture_snapshot()` 字节逐条比对；验证：全绿且 golden hash 逐字节不变、`SNAPSHOT_VERSION` 仍 3、状态哈希字段集合 / 快照头部布局（`defs_hash` / `content_hash`）/ `first_difference()` / `ExecuteFrame → PlayerId → Sequence → Arrival` 排序键 / 稳定 `EntityId` 顺序均不变（ADR 0002 / `CODEBUDDY.md` §5 / C1）
- [x] 1.4 `git diff -- crates/w3-core/` 范围核验：确认 diff **只含** `EntityView` 只读字段与 `read_view` 投影，MUST NOT 出现 `UnitState` 新字段、`compute_state_hash` / `capture_snapshot` / `first_difference()` / 命令 / 事件 / 排序键 / `SNAPSHOT_VERSION` 的任何改动；验证：diff 输出人工可核、无新增权威状态字段（C1 / C2 / D1）

## 2. 分层判据与合并函数（D2 / D3）

- [x] 2.1 在 `input.rs` 新增合并函数：把 `WorldView` 里「装配期 `MovabilityMap` 未登记」的实体按 `EntityView.speed != 0` 补进一张有效映射；frame-0 已登记项 MUST 原样保留（P8.0 D1 权威不变）、运行时未登记项按视图 `speed == 0` 判不可移动 / `> 0` 判可移动；验证：合并函数对「已登记 / 未登记 / 建筑 / 单位」四类输入的纯函数断言全绿，两条来源不互相替代（R1.4 / R13.6 / D2）
- [x] 2.2 在 `input.rs::InputState::apply` 的群体 `Move` 分支调用合并函数，把产出的有效映射传给 `filter_movable`；`filter_movable` 的签名与对已登记实体的行为 MUST NOT 改变、`apply` 已持有 `view: &WorldView` 故 MUST NOT 改其签名；验证：`Move` 生成集合按稳定 `EntityId` 顺序、落点分配与「只含可移动单位」逐条相同（R1.4 / R13.6 / D3）
- [x] 2.3 保护既有回归：跑 `tests/movability.rs` 全部用例，确认 5 处直接调 `filter_movable` / `degenerate_filter` 的反向验证断言**零改动仍成立**；验证：frame-0 农民 + 主城 + 金矿混合选中仍只为农民生成 `Move`、全为不可移动实体时显式反馈、落点不受过滤影响（R1.4 / P8.0 回归不退化 / C4）

## 3. 反向验证（改坏验红）与运行时动态实体回归（D4）

- [x] 3.1 新增 CPU 回归：构造「运行时经 `Train` / `Build` 产出建筑（`speed == 0`、不在装配期 `def_ids` 关联中）+ 战斗单位（`speed > 0`）」的 `WorldView` 与选中集合，经真实 `input.rs::InputState::apply` 群体 `Move` 路径断言「建筑 MUST NOT 出现在 `Move` 生成集合、战斗单位 SHALL 正常生成 `Move`」；验证：断言全绿且走真实路径、非旁路模拟（R1.4 / R13.6 / S1 / S3）
- [x] 3.2 **反向验证（改坏验红，`CODEBUDDY.md` §5 / C5）**：以退化实现（合并函数缺席、映射未登记实体回到 `unwrap_or(true)` 默认放行）替换正式实现，证明同 3.1 断言在**缺陷态失败、正式实现通过**；证据用测试内反例元断言或 A-B 对照，参照归档 P8.0 `tests/movability.rs::degenerate_filter` 先例；验证：两条证据均可复现并进 CI，仅「全绿」不算完成（D4 / C5）

## 4. playable / 默认空映射兜底（D5）

- [x] 4.1 验证合并函数兜底后默认态不再等价缺陷态：`app.rs::ClientApp::new` 默认 `MovabilityMap::new()` 空映射、`examples/playable.rs` 未注入 `with_movability(...)` 的场景下，`view` 含运行时生产建筑（`EntityView.speed == 0`）时 MUST NOT 生成 `Move`；验证：默认态与「显式注入正确映射」效果一致、空映射不再触发 `unwrap_or(true)` 全放行（R1.4 / R13.6 / D5）
- [x] 4.2 由 client-owner **评估并显式处置** `examples/playable.rs`（场景 4 个可移动单位、无建筑）与 `ClientApp::new` 默认态：补一个明确注入或如实登记「兜底已覆盖」，**不得**假装处理而实际没做；处置结论回报后写入 `decision-memo.md` 待确认项；验证：有明确结论与证据、不冒充已解决（D5 / B1 另一半）

## 5. CPU 端到端回归与范围边界（D6 / R1 / ADR 0002）

- [x] 5.1 CPU 端到端回归：以既有 `MeleeSetup` 装配一局，先 frame-0 混合选中右键 / `M` 移动（P8.0 回归），再运行时 `Train` / `Build` 产出建筑与单位后混合选中下达 `Move`，逐帧断言两实例 `compute_state_hash()` / `capture_snapshot()` 字节一致；验证：frame-0 建筑 / 金矿 / 主城 MUST NOT 出现在任何 `Move` 生成集合，运行时建筑不响应、战斗单位正常响应，按稳定 `EntityId` 顺序（R1.4 / R13.6 / ADR 0002 / S1 / S3）
- [x] 5.2 零资产 / 范围核验：确认无文件 IO、无 MPQ 派生数据、无真实导出 JSON、数值全为合成原创 `[待核]`、无新增依赖 / `Cargo.lock` 改动、无 N1–N6 内容；验证：`git diff --stat` / 资产扫描无命中、1.0 范围仅 S1 / S3 过程分期（ADR 0001 / ADR 0004 / C6 / C7）

## 6. 门禁、规格门与代码门（D6 / §8 / §9）

- [x] 6.1 写完 Rust 源码后 `cargo fmt --all`，提交前 `cargo fmt --all -- --check`；`python tools/check_encoding.py`、`python tools/check_source_conventions.py`、`git diff --check`；验证：全部退出码 0，UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格、中文注释（`CODEBUDDY.md` §2 / §3）
- [x] 6.2 `cargo test --workspace` 与 `openspec validate runtime-entity-movability --strict`；验证：`is valid` 且无 WARNING、`Verification` 相关门禁绿；如任一四件套有 WARNING / 错误，按 `CODEBUDDY.md` §8 **同步回查其余三份一起改**，不允许只改一份（C8）
- [x] 6.3 由只读 `critic` 按全清单复审**本 change 全部 diff**：第 7 / 10 项四件套一致性、第 11 项反向验证真 A-B、第 12 项参考项目对照锚与许可证、第 13 项 `R-x.y` 符合性、确定性 / 状态所有权 / 编码 / 资产 / Git 边界 / 工具脚本跨平台；验证：报告区分 blocker / residual、无空泛结论、锚可打开（C8 / `CODEBUDDY.md` §9）
- [x] 6.4 由 `critic` 代码门复核「首次动 `w3-core` 但 ADR 0002 零冲击」：golden hash 逐字节不变、`git diff -- crates/w3-core/` 只含视图字段与投影、`SNAPSHOT_VERSION` 仍 3、未借本 change 旁路确定性门禁；验证：命令输出与 diff 证据齐全、结论不含「已验证跨平台新平台」等夸大（C1 / D1 / `CODEBUDDY.md` §5）

## Workflow follow-up

- 规格门已由用户 2026-11-16 本会话指令 + `AskUserQuestion` 方案拍板批准（`decision-memo.md` `status: approved` + 真值 fingerprint），成文后进入 `/opsx:apply`（`sim-owner` 守 `w3-core`、`client-owner` 守 `w3-client` / `w3-render`），再走 `critic` 代码门 → 本地 commit（中文描述 + Conventional Commits 英文 type 前缀）→ 人工 push → CI 双平台 `python tools/verify_all.py` → `/opsx:archive`；归档后确认主 `openspec/specs/` 新增 `runtime-entity-orders` capability、`unit-orders` 并入 ADDED Requirement。
- 活文档 `WC3-MELEE-RULES.md` R1.4 / R13.6 字面禁令由并行 owner 按 §14 同步修订；主 spec / `p8-human-faction`（active、规格锁定）正文由后续 change / 归档复核承接，本 change 不改写（Context / D6 / `decision-memo.md` 待确认项）。
- P8.5 深度玩法 / 野怪 / 物品 / 迷雾 / HUD / 键位接线、真窗口人工验收、playable 默认态处置结论由后续独立 change / owner 承接；`git push` / fetch / pull / clone 人工独占（`CODEBUDDY.md` §4 / §10）。
