# 决策纪要 — `p8-faction-foundation`

> 取料纪律：基于仓库 artifacts 回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002）。
> 本 change 是 P8 路线图（`docs/roadmap/P8-human-orc-melee.md`）的首个正式 OpenSpec change，走完整规格门流程。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `crates/w3-core/` 零改动；不新增权威状态字段，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合与快照头部布局不变 | `crates/w3-core/src/snapshot.rs:15` / `openspec/specs/melee-match/spec.md:127-129` / `CODEBUDDY.md:48`（§5） |
| C2 | 仓库零资产；两族定义全为合成 / 原创编译期整数常量，无文件 IO、无新增依赖 | `CODEBUDDY.md:56`（§6）/ `crates/w3-client/src/melee/defs.rs:1-6` |
| C3 | 规格门批准**由人拍板**，脚本不得自批、不得写回 `decision-memo.md`；实现期改四件套即解锁、须重走规格门 | `CODEBUDDY.md:106`（§9 串联）/ `CODEBUDDY.md:107`（规格锁定与重审） |
| C4 | 种族维度 MUST 只放在**定义层**（DefId 分段），MUST NOT 新增 `Player.race` 等运行时权威状态 | `openspec/specs/melee-match/spec.md:127-129` / `CODEBUDDY.md:51` / `design.md` D2 |
| C5 | 群体 `Move` 过滤 MUST 在输入翻译层完成，判据 MUST 由 client 侧冻结定义 `UnitDef.speed_raw` 派生的**表现层只读映射**提供（装配期注入 `app.rs` → `InputState::apply()`）；MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`、MUST NOT 回退建筑类型白名单，MUST NOT 改 `w3-core` 命令语义 / 排序键 / 哈希口径 | `crates/w3-client/src/input.rs:369`（`commit_selection`）/ `:458`（`apply()`）/ `:513`（`Move` 生成）/ `crates/w3-client/src/app.rs:199` / `:256` / `:370` / `crates/w3-core/src/sim_match.rs:180`（`EntityView`）/ `:193`（`WorldView`）/ `crates/w3-client/src/melee/setup.rs:200`（`speed.max(1)`）/ `crates/w3-core/src/command.rs:22` / `design.md` D1 |
| C6 | 新增回归 MUST 反向验证（改坏验红），仅「全绿」不构成验证 | `CODEBUDDY.md:52`（§5）/ `openspec/config.yaml:68`（`rules.tasks` 第三条） |
| C7 | `critic` 代码门按全清单审**本 change 全部 diff**，含第 10 项证据锚与第 11 项整体调用路径 | `.codebuddy/agents/critic.md:28-29` / `tasks.md` 3.4 |
| C8 | 四件套一次性成文；局部修改任一 artifact MUST 回查其余三份 | `CODEBUDDY.md:88`（§8 成文顺序与四件套一致性） |
| C9 | 工具脚本跨平台、UTF-8 无 BOM / LF、中文注释与提交描述、英文日志 / 错误；AI 不碰远端 | `CODEBUDDY.md:13`（§2）/ `:24`（§3）/ `:33`（§4） |

## 拍板决策（含被否备选与理由）

- **P1 用户已明确授权立项与更重水位**：用户 2026-11-14 明确「要更重（含 P8.5 全量：英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）」并「已确认，请你把 P8.0 转成首个正式 OpenSpec change（四件套 + `decision-memo.md` → 规格门 → apply → 代码门）」。证据形态：本会话用户消息（**非仓库文件**，依模板三分判据第 3 类；仓库内无可核验 `file:line` 锚，**由人工核对**）。该授权针对**立项与流程**，不是对具体技术方案 D1–D3 的批准。
- **P2（AI 推荐，规格门 2026-11-14 批准锁定）技术方案 D1–D3**：从 client 侧冻结定义 `UnitDef.speed_raw` 派生 `EntityId → 可移动性` 表现层只读映射并注入输入翻译层以过滤群体移动（MUST NOT 扩展 `WorldView` / 用运行时 `speed` / 白名单）/ `units` 类别 DefId 按种族分段且不新增权威状态 / 测试内反例元断言做反向验证。锚 `design.md` Decisions；**技术方案为 AI 推荐，用户拍板的是规格门批准这一动作，未逐条确认方案细节**；被否备选见 R1–R6。
- **P3（AI 推荐，规格门 2026-11-14 批准锁定）Capability 划分**：新增 `faction-definitions`，`unit-orders` 增加一条 ADDED Requirement。锚 `proposal.md` Capabilities / `specs/`。状态同 P2。
- **P4 用户 2026-11-14 明确拍板「1.0 范围三项 + 授权实施」**：① 「局域网联机对战」含**真人 VS AI 敌人玩家**，AI 玩家全部功能属 1.0 必做，「不做单机对战」收敛为仅不做**纯本地单机对局形态**，本地 `melee` 窗口保留作开发夹具；② **战争迷雾 / 隐身纳入 1.0**（表现层只读派生，不进权威状态 / 哈希 / 快照）；③ P8 路线图按**1.0 四族分期口径**对齐（P8 两族竖切为前半段，P9+ 补不死 / 暗夜）。用户并明确「1 和 2 对齐后可以开始实施了」。证据形态：本轮会话用户消息（**非仓库文件**，依模板三分判据第 3 类，由人工核对）。落地锚：`docs/references/WC3-MELEE-RULES.md` §0 / §0.3 T1 / T2 / T5 / §12 R12.7、`CODEBUDDY.md` §1、`openspec/config.yaml` context、`CONTRIBUTING.md` §范围、`docs/decisions/0003-scope.md` 修订记录、`docs/roadmap/P8-human-orc-melee.md` 范围上位说明 / §6。这些均为**纯文档 / 规则改动，不触及四件套**，指纹保持 `0de50d08…` 不变，按 `CODEBUDDY.md:107` 不触发规格解锁重走。被否备选：迷雾不纳入（与已拍板 P8.5 全量含迷雾冲突）、AI 功能移出 1.0（与用户澄清冲突）。

## 被否方向台账

- **R1** 在 `w3-core::sim_match` 收到 `Move` 时丢弃不可移动实体 → 否决：改内核权威路径、要动状态哈希与快照（违反主 `melee-match/spec.md:127-129` 与 ADR 0002）。锚 `design.md` D1 备选 1。
- **R2** 在 `w3-core` 加 `Player.race` 权威状态字段 → 否决：新增权威状态须同步进哈希 / 快照 / `first_difference()` 且可能升 `SNAPSHOT_VERSION`。锚 `design.md` D2 备选 1。
- **R3** 用「建筑类型」硬编码白名单判可移动性 → 否决：违反「数值读 Def 不写死」的工程纪律（引申自 `godot_warcraft3/docs/roadmap/NEXT.md:152`，原文特指造价 / 伤害；「可移动性判据不写死类型」是本项目引申），`EntityKind` 亦无 `is_movable()`。**前提修正（规格门 Blocker 1）**：`speed_raw == 0` 是**定义层**既有事实，但在输入翻译层不可达（`WorldView` / `EntityView` 无定义字段、运行时 `speed` 被抬到 1），故本 change 需新建一条从冻结定义派生 `EntityId → 可移动性` 的表现层只读映射并接线，而非直接读视图；这正是 D1 决策的核心。**方向变更登记**：归档 `melee-playable-loop` Risks 末条建议「按单位类型过滤」，本 change 改判为「读定义层 `speed_raw` 派生映射」，理由是不写死类型。锚 `design.md` D1 / `openspec/changes/archive/2026-11-14-melee-playable-loop/design.md` Risks 末条。
- **R4** 每个种族一份独立 `FrozenDefinitions` / 独立对局 → 否决：一局内两族对抗需同一 `RtsMatch`，两份定义破坏单一真值源与 `defs_hash`。锚 `design.md` D2 备选 2。
- **R5** HUD 选中时就不允许选中建筑 → 否决：改变 `rts-command-hud` 选择契约，建筑仍需可选中以查看 / 训练。锚 `design.md` D1 备选 3。
- **R6** 用字符串 / 枚举 tag 挂实体表达种族 → 否决：引入 `String` 进权威状态、改哈希编码，且与「DefId 已能区分类型」重复。锚 `design.md` D2 备选 3。
- `design.md` D3 的两条备选（只写正向断言、人工改坏代码）属测试实现细节级，按 `openspec/decision-memo.template.md:11` 不纳入方向台账；台账筛选口径为「触及权威状态 / 分层 / 依赖边界的方向级否决」，R1–R6 均属此列。

## 待确认项（如实标注，不冒充已拍板）

- ~~**规格门是否批准**（含 P2 技术方案与 P3 capability 划分）~~ → **已关闭（2026-11-14）**：用户拍板授权实施、`critic` 两轮规格门报告与第 12 / 13 项核验均无 blocker；见 P4 与「规格门」小节。
- ~~**四件套指纹回填值**~~ → **已回填（2026-11-14）**：`python -B tools/verify_all.py --fingerprint p8-faction-foundation` 输出 `0de50d08…`，已置 `status: approved` + `approved_by:` + `date:`。
- **AI 玩家 × 联机 lockstep 接合架构（后续 change）**：AI 玩家作为对战一方的决策在哪一侧计算、命令如何注入 lockstep 帧流、如何保证多端确定性重放，未在本次拍板范围内，登记为 `WC3-MELEE-RULES.md` §0.3 T5 待确认项，由后续 AI / 联机 change 承接，**不阻塞 P8.0**。
- **`file:line` 锚漂移**：上一轮演练（`spec-gate-fingerprint-cli`）的 R1 / B1 发现，四件套内行号锚在 apply 后会失准、改则解锁须重走规格门。本纪要 C1–C9 的锚已在成文前逐条 Grep 核实；apply 后若需更正，按 `CODEBUDDY.md:107` 规格锁定与重审处理。
- **`speed_raw == 0` 语义**：作为可移动性判据是约定；未来「可移动但速度为 0」的单位须在定义层显式区分，见 `design.md` Risks。
- **DefId 段位分配**：人族 `1xx` / 兽族 `2xx` / 中立资源 `9xx` 为 AI 推荐，待规格门拍板；分段**只约束 `units` 类别**，`abilities` / `upgrades` / `items` 三个类别的既有 ID（10 / 20 / 30）沿用类别内唯一、不纳入种族分段；一旦定下，P8.1 / P8.2 新增单位 MUST 落在各自段内。
- **范围承接**：P8.0 只做地基，两族可玩内容由 P8.1 / P8.2 承接；P8.5 全量已由用户拍板纳入必做（`docs/roadmap/P8-human-orc-melee.md` §0 / §3）。
- **工作流规则联动（2026-11-14）**：本轮同时把 `docs/references/UPSTREAM.md` 扩为「按任务类型的强制参考矩阵」并给 `openspec/config.yaml` 的 `rules.design` 加「## 参考项目对照」硬要求；`design.md` 已补该小节（T1 / T2 / T4 / T5）。该补充为纯文档、不改技术决策，`critic` 第 2 轮「可提交人拍板」结论仍适用；四件套指纹因此重算（批准前修订不触发 `CODEBUDDY.md:107` 规格锁定与重审的解锁重走）。
- **参考对照核验修订（2026-11-14，委派 `reference-researcher`）**：`design.md`「## 参考项目对照」上一轮为**未经真实调研**写入，本轮委派只读 `reference-researcher` 逐锚核验，纠正 4 类问题：① T1 把「新增权威状态字段须同步进哈希 / 快照」错误归因给 `vera20k`（实为 `src/net/mod.rs:1-15` 纯命令传输 + `lockstep.rs:36-38` 逐帧 checksum；纪律真实出处是 `CODEBUDDY.md` §5 / ADR 0002 / 主 `melee-match/spec.md:127-129`）；② T4 把 `WarsmashModEngine/resources/Scripts/melee.jui` 误作「对战初始化动作表」（实为 JASS UI 帧定义、零条开局单位生成条目）；③ T2 稳定落点语义锚错在 `NAVIGATION.md`（实在 `MOVEMENT.md:73-81` + `FRAMEWORK.md:64/81`）；④ 参考缺口过度概括（Warsmash `MutableObjectData.java:411` 有 `urac` race 字段、godot `melee_race_preview.gd:52-79` 有种族→单位 ID 表，只是不能照搬）。`design.md` 已按核验版重写、每条锚实际打开；`docs/references/UPSTREAM.md` 矩阵 T4 的错误描述同步修正；`proposal.md` 无参考项目锚、无需改动；C1–C9 锚全部指向本项目文件、无需改动；D1 备选 2 的「数值读 Def 不写死」订正为本项目的引申（原文特指造价 / 伤害）。纯文档修订、不改技术决策、`status: pending` 不变，四件套指纹待拍板时重算。
- **规格门第 1 轮修订记录（2026-11-14）**：`critic` 规格门报告 1 个 blocker + 8 条 residual。Blocker 1（判据 `speed_raw` 在输入翻译层不可达、D1 对现状转述失准、proposal 漏列 `app.rs`、tasks 缺前置任务）已通过修订 `proposal.md` / 两个 spec delta / `design.md` D1 / `tasks.md` 第 1 组 / 本纪要 C5 与 R3 闭合；Residual 1（补 R5/R6）、2（R3 方向变更）、3（人工验收措辞）、4（DefId 类别口径）、5（tasks 验证断言）、6（P8.5 措辞）、8（3.5 流程顺序）均已同步修订；Residual 7（`config.yaml:48` 阶段措辞 P0–P5 落后于 P8）为项目规则陈旧，**不在本 change 修**，由 `spec-steward` 后续处理。修订只动四件套与纪要、未改实现代码、未放宽 ADR 0001 / 0002 约束；四件套指纹在修订后重算，待规格门拍板时由 `spec-steward` 回填（`status: pending` 期间规格门小节不含 `fingerprint:`，不冒充已回填）。
- **归档与人工 push**：AI 不碰远端（`CODEBUDDY.md` §4）。
- **代码门结论（2026-11-14，`critic` 全清单）**：apply 完成后委派只读 `critic` 复审全部 diff（13 项）。确定性 / 状态所有权 / 编码 / 资产 / Git 边界 / 工具脚本跨平台 / 架构原则 / 参考项目对照 / 决策纪要质量均通过；`w3-core` / `w3-render` / `Cargo.lock` / 四件套零 diff、`SNAPSHOT_VERSION=3`、反向验证经真实 `filter_movable` 路径为**真 A-B**（缺陷态必失败、正式态通过）、frame-0 混合集合 R1 符合且落点字节一致、两族 DefId 分段与 Spawn 字节一致性、`melee` 真窗口装配注入链正确。证据：`git diff` / 各门禁命令输出 / `file:line`，见本轮会话 `critic` 代码门报告。
- **B1 代码门 blocker → 已由用户拍板「登记已知限制后放行」（2026-11-14）**：`critic` 报 1 个 blocker——`crates/w3-client/src/input.rs:65` 的 `filter_movable` 对映射中查不到的 `EntityId` 用 `unwrap_or(true)`（默认可移动），与 `derive_movability` 对「定义缺失」返回 `false` 的保守语义相反；`app.rs:243` 的 `ClientApp::new()` 默认空映射、`examples/playable.rs:207` 未注入 `with_movability(...)`，故「默认态」等价于 `tests/movability.rs` 反向验证用空映射定义的「缺陷态」。残留两条 **P8.0 范围外路径**：① `playable` 开发夹具（场景仅 4 个可移动单位、无建筑，观测不到 bug）；② 运行时 `Train` / `Build` 新生成实体不在装配期 `def_ids` 关联中。**根因（leader 复核）**：`CommandKind::Spawn` 不含 `DefId`、`WorldView` / `EntityView` 不暴露类型、P8.0 硬约束「`w3-core` 零改动 + MUST NOT 扩展视图」——运行时新实体可移动性在 P8.0 阶段**无法判定**：`unwrap_or(true)` 放行运行时新建筑（违反 R1）、`unwrap_or(false)` 卡住运行时新士兵（功能退化），二者皆非本 change 可闭合。frame-0 静态路径（含用户真窗口验收的 `melee` 入口，装配期已正确注入映射）R1 已闭合。**用户拍板依据**：地基 change 不交付建造 / 训练玩法（design Non-Goals）、`playable` 为开发夹具、符合「收尾优先记录已知限制而非扩范围」。处置：**不改代码、不改四件套**（指纹 `0de50d08…` 不变、规格锁定未解锁、不重走规格门），登记为已知限制，由后续承接运行时装配映射的 change 处理（须同步评估 `unwrap_or` 语义统一与 `playable` 注入）。
- **本机 GPU native 崩溃（间歇性驱动 / 环境故障，如实记录、不冒充也不夸大）**：apply 期间 `client-owner` 曾报本机 `cargo test --workspace` / `python tools/verify_all.py` 因 2 个 GPU 测试二进制 native 崩溃（`w3-client/tests/real_map_e2e.rs` `0xc0000005 STATUS_ACCESS_VIOLATION` / `w3-render/tests/gpu_headless.rs` `0xc0000374 STATUS_HEAP_CORRUPTION`）未全绿；leader 亦独立复现过一次 `0xc0000005`（单独跑 `gpu_headless`）。证据链认定与本次改动无关：① 两崩溃测试文件相对 HEAD **零改动**、对本次改动符号（`movability` / `derive_movability` / `def_ids` / `Race` / `synthetic_definitions` / `filter_movable`）**零引用**；② `w3-render` / `Cargo.lock` 零 diff；③ 属 Windows native 异常码（非 Rust 断言 exit 101）；④ `client-owner` 在纯净 `d8af94d` 隔离 worktree 复现同崩溃（转述，`critic` 未独立核验）。**间歇性判定 + 提交前连续复测全绿**：`python tools/verify_all.py` 连续 2 次 `Verification OK`（exit 0，含 `cargo test` 步）、单独 `gpu_headless` GPU 测试第 2 次 `ok`、`cargo test --workspace` 不带 skip 全绿（28 个 `test result: ok`、无失败行）、CPU 侧 `determinism 26/26` / `isolation 3/3` / `movability 8/8` / `melee_loop 9/9`。**最新状态：本机 `verify_all.py` `Verification OK`，门禁全绿**；因曾出现间歇性 native 故障，仍建议人工 push 后由 CI 双平台 `verify_all.py` 作环境健壮性的权威复核。AI 未改门禁逻辑、未跳过测试、未把曾发生的崩溃冒充为「稳定失败」，也未把间歇性故障写成「从未发生」。
- **文本层 residual（用户拍板维持规格锁定，下次解锁四件套时一并补齐）**：`critic` 代码门 residual——① 四件套（proposal / design / specs / tasks）未引用 `R-x.y` 规则编号（该硬规则于 `d8af94d` 才加入、四件套成文在 `624a3f4`、规格门批准时未追溯，实质符合 R1.1 / R1.4）；补引用属四件套改动会触发解锁重走规格门，故本轮不补、如实登记；② design D2「开局 `Spawn` 写对应段 `DefId`」字面与实现不一致（`w3-core` 的 `CommandKind::Spawn` 无 `DefId` 字段，种族段位落在装配层 `EntityId→DefId` 关联 `def_ids`，两族 Spawn 字节一致、方向正确且符合内核零改动硬约束）；`specs/faction-definitions/spec.md:30` Scenario「Spawn 命令的 DefId 落在各自段内」无法被直接单测验证（实际验证 `def_ids`）；③ `specs/unit-orders/spec.md` requirement 为无条件表述、3 个 Scenario 仅覆盖 frame-0 静态集合，未覆盖运行时新实体（已并入 B1 已知限制）。三者均为文本 / 追溯层缺口、不影响本 change 已交付的行为与确定性，按 `CODEBUDDY.md:107` 维持规格锁定，由后续解锁一并修订。

## 停止条件（追加阶段 / 归档前）

1. C1–C9 逐条仍有证据锚、无一条被本轮改动证伪。
2. 规格门由人拍板、`fingerprint:` 回填、`status: approved`，且 `verify_all.py` 的 `spec_gate_check()` 不报失配。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛。

## 规格门

- status: approved
- fingerprint: 0de50d08ad4448a7f8b1925193115c4650c71f3afd51bbc8fd49049169064e07
- approved_by: 用户（2026-11-14 拍板「1 和 2 对齐后可以开始实施了」）
- date: 2026-11-14
- reason: `critic` 两轮规格门报告 + 第 12 / 13 项核验无 blocker；用户 2026-11-14 就 1.0 范围三项拍板（P4）并授权实施；本轮为纯文档 / 规则对齐，四件套未改、指纹未变、不触发解锁重走。
