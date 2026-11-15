# Tasks

> 按 `design.md` D1–D3 分组；`crates/w3-core/` 零改动。每组自带验证与证据要求。

## 1. 群体移动可移动性过滤（D1）

- [ ] 1.1 **前置：建立并注入只读可移动性判据**：装配期从 `FrozenDefinitions::units()` 的 `UnitDef.speed_raw` 派生 `EntityId → 可移动性` 映射，经 `crates/w3-client/src/app.rs:199` / `:256` / `:370` 注入 `InputState`，`crates/w3-client/src/input.rs:458` 的 `apply()` 增加只读参数；MUST NOT 扩展 `EntityView` / `WorldView`（`crates/w3-core/src/sim_match.rs:180` / `:193`）、MUST NOT 用运行时 `speed`（`setup.rs:200` 强制 `max(1)`）、MUST NOT 回退建筑类型白名单；验证：`git diff -- crates/w3-core/` 为空、既有 `interaction` 单测按新签名适配后全绿
- [ ] 1.2 在 `crates/w3-client/src/input.rs` 的群体 `Move` 生成处按 1.1 映射过滤选中集合；建筑 / 金矿 / 树木不生成 `Move` 命令、位置不变；验证：混合集合单测断言只有农民收到 `Move`
- [ ] 1.3 落点分配保持按稳定 `EntityId` 顺序、只作用于筛后集合，不因过滤改变可移动单位的落点；验证：混合集合与「只含这些农民」的落点逐条相同
- [ ] 1.4 全为不可移动实体时显式反馈、不生成空命令、不改权威状态与状态哈希；验证：单测断言命令数为 0 且哈希不变
- [ ] 1.5 **反向验证（改坏验红，`CODEBUDDY.md:52`）**：定义退化过滤器（不过滤、把所有实体都当可移动），证明 1.2 的断言在退化实现下失败、正式实现下通过；验证：退化版必被抓出、正式版必通过，两条证据都在

## 2. 定义层种族分段与开局装配（D2）

- [ ] 2.1 在 `crates/w3-client/src/melee/defs.rs` 定义 `units` 类别的 DefId 分段常量（人族 / 兽族 / 中立资源）并扩 `synthetic_definitions()`，全为编译期整数常量；三个非 units 类别 ID（10 / 20 / 30）沿用类别内唯一、不纳入分段；验证：`defs_hash` 固定、`units` 类别 DefId 全局唯一、与既有 6 个 `UnitDef` 的 DefId 不冲突，且在不含 `config.toml` / 导出目录的干净环境构造不做任何文件 IO
- [ ] 2.2 `MeleeSetup` 增加种族参数，开局 `Spawn` 命令写对应段 `DefId`；未指定时回落人族段、既有入口行为不变；验证：`setup.rs` 单测覆盖两族装配与回落
- [ ] 2.3 **MUST NOT** 在 `w3-core` 增加 `Player.race` 或任何权威状态字段；验证：`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3
- [ ] 2.4 CPU 端到端：两族参数各装配一局并**打完**，未传种族参数时回落人族段且一局仍能装配并打完；逐帧哈希与基线一致；验证：`melee_loop` 新增 2 项回归 + 回落兼容 1 项全绿

## 3. 门禁、确定性复核与规格门

- [ ] 3.1 `cargo fmt --all` 后 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`；验证：退出码 0
- [ ] 3.2 `cargo test -p w3-core --test determinism`（26 项）、`isolation`（3 项）、`cargo test --workspace`；验证：全绿，`git diff -- crates/w3-core/` 为空
- [ ] 3.3 `openspec validate p8-faction-foundation --strict` 与 `python tools/verify_all.py`（含 `decision memo OK` / `spec gate OK`）；验证：`Verification OK`
- [ ] 3.4 由只读 `critic` 按全清单（含第 10 项 Spec 与决策纪要质量、第 11 项整体调用路径）复审**本 change 全部 diff**；验证：报告区分 blocker / residual、无空泛结论
- [ ] 3.5 确认规格门批准记录已在 `/opsx:apply` **之前**回填（`fingerprint:` / `status: approved` / `approved_by:` / `date:`），且 `tasks.md` 打勾不触发解锁（复选框归一化）；验证：`spec_gate_check()` 不报失配。若执行 3.3 时纪要仍 `pending`，按 `CODEBUDDY.md:106` §9 串联属流程顺序（应先批准回填再 apply），**不改门禁逻辑、不绕过规格门**
- [ ] 3.6 以中文描述 + Conventional Commits 英文 type 前缀本地 commit；验证：`git log -1` 中文描述首行 ≤ 72 字符；`git push` 由人工执行
