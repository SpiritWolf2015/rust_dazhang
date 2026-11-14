# 决策纪要 — `p8-faction-foundation`

> 取料纪律：基于仓库 artifacts 回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002）。
> 本 change 是 P8 路线图（`docs/roadmap/P8-human-orc-melee.md`）的首个正式 OpenSpec change，走完整规格门流程。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `crates/w3-core/` 零改动；不新增权威状态字段，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合与快照头部布局不变 | `crates/w3-core/src/snapshot.rs:15` / `openspec/specs/melee-match/spec.md:127-129` / `CODEBUDDY.md:47`（§5） |
| C2 | 仓库零资产；两族定义全为合成 / 原创编译期整数常量，无文件 IO、无新增依赖 | `CODEBUDDY.md:55`（§6）/ `crates/w3-client/src/melee/defs.rs:1-6` |
| C3 | 规格门批准**由人拍板**，脚本不得自批、不得写回 `decision-memo.md`；实现期改四件套即解锁、须重走规格门 | `CODEBUDDY.md:102`（§9 串联）/ `CODEBUDDY.md:103`（规格锁定与重审） |
| C4 | 种族维度 MUST 只放在**定义层**（DefId 分段），MUST NOT 新增 `Player.race` 等运行时权威状态 | `openspec/specs/melee-match/spec.md:127-129` / `CODEBUDDY.md:50` / `design.md` D2 |
| C5 | 群体 `Move` 过滤 MUST 在输入翻译层完成，判据 MUST 由 client 侧冻结定义 `UnitDef.speed_raw` 派生的**表现层只读映射**提供（装配期注入 `app.rs` → `InputState::apply()`）；MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`、MUST NOT 回退建筑类型白名单，MUST NOT 改 `w3-core` 命令语义 / 排序键 / 哈希口径 | `crates/w3-client/src/input.rs:369`（`commit_selection`）/ `:458`（`apply()`）/ `:513`（`Move` 生成）/ `crates/w3-client/src/app.rs:199` / `:256` / `:370` / `crates/w3-core/src/sim_match.rs:180`（`EntityView`）/ `:193`（`WorldView`）/ `crates/w3-client/src/melee/setup.rs:200`（`speed.max(1)`）/ `crates/w3-core/src/command.rs:22` / `design.md` D1 |
| C6 | 新增回归 MUST 反向验证（改坏验红），仅「全绿」不构成验证 | `CODEBUDDY.md:51`（§5）/ `openspec/config.yaml:61`（`rules.tasks` 第三条） |
| C7 | `critic` 代码门按全清单审**本 change 全部 diff**，含第 10 项证据锚与第 11 项整体调用路径 | `.codebuddy/agents/critic.md:28-29` / `tasks.md` 3.4 |
| C8 | 四件套一次性成文；局部修改任一 artifact MUST 回查其余三份 | `CODEBUDDY.md:85`（§8 成文顺序与四件套一致性） |
| C9 | 工具脚本跨平台、UTF-8 无 BOM / LF、中文注释与提交描述、英文日志 / 错误；AI 不碰远端 | `CODEBUDDY.md:12`（§2）/ `:23`（§3）/ `:32`（§4） |

## 拍板决策（含被否备选与理由）

- **P1 用户已明确授权立项与更重水位**：用户 2026-11-14 明确「要更重（含 P8.5 全量：英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）」并「已确认，请你把 P8.0 转成首个正式 OpenSpec change（四件套 + `decision-memo.md` → 规格门 → apply → 代码门）」。证据形态：本会话用户消息（**非仓库文件**，依模板三分判据第 3 类；仓库内无可核验 `file:line` 锚，**由人工核对**）。该授权针对**立项与流程**，不是对具体技术方案 D1–D3 的批准。
- **P2（AI 推荐，待规格门拍板）技术方案 D1–D3**：从 client 侧冻结定义 `UnitDef.speed_raw` 派生 `EntityId → 可移动性` 表现层只读映射并注入输入翻译层以过滤群体移动（MUST NOT 扩展 `WorldView` / 用运行时 `speed` / 白名单）/ `units` 类别 DefId 按种族分段且不新增权威状态 / 测试内反例元断言做反向验证。锚 `design.md` Decisions；**未获用户明确确认，MUST NOT 写成已拍板**；被否备选见 R1–R6。
- **P3（AI 推荐，待规格门拍板）Capability 划分**：新增 `faction-definitions`，`unit-orders` 增加一条 ADDED Requirement。锚 `proposal.md` Capabilities / `specs/`。状态同 P2。

## 被否方向台账

- **R1** 在 `w3-core::sim_match` 收到 `Move` 时丢弃不可移动实体 → 否决：改内核权威路径、要动状态哈希与快照（违反主 `melee-match/spec.md:127-129` 与 ADR 0002）。锚 `design.md` D1 备选 1。
- **R2** 在 `w3-core` 加 `Player.race` 权威状态字段 → 否决：新增权威状态须同步进哈希 / 快照 / `first_difference()` 且可能升 `SNAPSHOT_VERSION`。锚 `design.md` D2 备选 1。
- **R3** 用「建筑类型」硬编码白名单判可移动性 → 否决：违反「数值读 Def 不写死」，`EntityKind` 亦无 `is_movable()`。**前提修正（规格门 Blocker 1）**：`speed_raw == 0` 是**定义层**既有事实，但在输入翻译层不可达（`WorldView` / `EntityView` 无定义字段、运行时 `speed` 被抬到 1），故本 change 需新建一条从冻结定义派生 `EntityId → 可移动性` 的表现层只读映射并接线，而非直接读视图；这正是 D1 决策的核心。**方向变更登记**：归档 `melee-playable-loop` Risks 末条建议「按单位类型过滤」，本 change 改判为「读定义层 `speed_raw` 派生映射」，理由是不写死类型。锚 `design.md` D1 / `openspec/changes/archive/2026-11-14-melee-playable-loop/design.md` Risks 末条。
- **R4** 每个种族一份独立 `FrozenDefinitions` / 独立对局 → 否决：一局内两族对抗需同一 `RtsMatch`，两份定义破坏单一真值源与 `defs_hash`。锚 `design.md` D2 备选 2。
- **R5** HUD 选中时就不允许选中建筑 → 否决：改变 `rts-command-hud` 选择契约，建筑仍需可选中以查看 / 训练。锚 `design.md` D1 备选 3。
- **R6** 用字符串 / 枚举 tag 挂实体表达种族 → 否决：引入 `String` 进权威状态、改哈希编码，且与「DefId 已能区分类型」重复。锚 `design.md` D2 备选 3。
- `design.md` D3 的两条备选（只写正向断言、人工改坏代码）属测试实现细节级，按 `openspec/decision-memo.template.md:11` 不纳入方向台账；台账筛选口径为「触及权威状态 / 分层 / 依赖边界的方向级否决」，R1–R6 均属此列。

## 待确认项（如实标注，不冒充已拍板）

- **规格门是否批准**（含 P2 技术方案与 P3 capability 划分）：待用户按 `critic` 规格门报告拍板；批准前 `status: pending`。
- **四件套指纹回填值**：人拍板后由 `spec-steward` 用 `python -B tools/verify_all.py --fingerprint p8-faction-foundation`（或 `spec_artifact_fingerprint()`）取 64 位 hex 回填，并置 `status: approved` + `approved_by:` + `date:`。
- **`file:line` 锚漂移**：上一轮演练（`spec-gate-fingerprint-cli`）的 R1 / B1 发现，四件套内行号锚在 apply 后会失准、改则解锁须重走规格门。本纪要 C1–C9 的锚已在成文前逐条 Grep 核实；apply 后若需更正，按 `CODEBUDDY.md:103` 处理。
- **`speed_raw == 0` 语义**：作为可移动性判据是约定；未来「可移动但速度为 0」的单位须在定义层显式区分，见 `design.md` Risks。
- **DefId 段位分配**：人族 `1xx` / 兽族 `2xx` / 中立资源 `9xx` 为 AI 推荐，待规格门拍板；分段**只约束 `units` 类别**，`abilities` / `upgrades` / `items` 三个类别的既有 ID（10 / 20 / 30）沿用类别内唯一、不纳入种族分段；一旦定下，P8.1 / P8.2 新增单位 MUST 落在各自段内。
- **范围承接**：P8.0 只做地基，两族可玩内容由 P8.1 / P8.2 承接；P8.5 全量已由用户拍板纳入必做（`docs/roadmap/P8-human-orc-melee.md` §0 / §3）。
- **工作流规则联动（2026-11-14）**：本轮同时把 `docs/references/UPSTREAM.md` 扩为「按任务类型的强制参考矩阵」并给 `openspec/config.yaml` 的 `rules.design` 加「## 参考项目对照」硬要求；`design.md` 已补该小节（T1 / T2 / T5 / T4）。该补充为纯文档、不改技术决策，`critic` 第 2 轮「可提交人拍板」结论仍适用；四件套指纹因此重算（批准前修订不触发 `CODEBUDDY.md:103` 的解锁重走）。
- **规格门第 1 轮修订记录（2026-11-14）**：`critic` 规格门报告 1 个 blocker + 8 条 residual。Blocker 1（判据 `speed_raw` 在输入翻译层不可达、D1 对现状转述失准、proposal 漏列 `app.rs`、tasks 缺前置任务）已通过修订 `proposal.md` / 两个 spec delta / `design.md` D1 / `tasks.md` 第 1 组 / 本纪要 C5 与 R3 闭合；Residual 1（补 R5/R6）、2（R3 方向变更）、3（人工验收措辞）、4（DefId 类别口径）、5（tasks 验证断言）、6（P8.5 措辞）、8（3.5 流程顺序）均已同步修订；Residual 7（`config.yaml:48` 阶段措辞 P0–P5 落后于 P8）为项目规则陈旧，**不在本 change 修**，由 `spec-steward` 后续处理。修订只动四件套与纪要、未改实现代码、未放宽 ADR 0001 / 0002 约束；四件套指纹在修订后重算，待规格门拍板时由 `spec-steward` 回填（`status: pending` 期间规格门小节不含 `fingerprint:`，不冒充已回填）。
- **归档与人工 push**：AI 不碰远端（`CODEBUDDY.md` §4）。

## 停止条件（追加阶段 / 归档前）

1. C1–C9 逐条仍有证据锚、无一条被本轮改动证伪。
2. 规格门由人拍板、`fingerprint:` 回填、`status: approved`，且 `verify_all.py` 的 `spec_gate_check()` 不报失配。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛。

## 规格门

- status: pending
- reason: 规格门未批准；`critic` 报告见本轮会话，待用户拍板后由 `spec-steward` 回填 `fingerprint:` / `approved_by:` / `date:` 并改为 `status: approved`。批准前 `verify_all.py` 报 `invalid spec gate status` 属正确信号，不是缺陷。
