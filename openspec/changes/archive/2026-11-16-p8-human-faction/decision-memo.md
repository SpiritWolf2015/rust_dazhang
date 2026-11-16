# 决策纪要 — `p8-human-faction`

> 取料纪律：基于仓库 artifacts 与只读参考项目回填；「拍板」项须有明确记载，找不到的一律进「待确认项」，**不把 AI 事后归纳写成「用户已确认」**。
> 合规边界：纯文档，不进 FNV-1a 状态哈希、不进 `SNAPSHOT_VERSION`（当前 3）快照、不含任何资产或本机绝对路径（ADR 0001 / ADR 0002 / `CODEBUDDY.md` §6）。
> 本 change 是 P8 路线图 `docs/roadmap/P8-human-orc-melee.md` §3 的 P8.1「人族完整化」立项，走完整规格门流程；**本次只是立项**，规格门由人拍板，AI 绝不 self-approve。`reference-researcher` 已按 `UPSTREAM.md` 矩阵 T1/T2/T4/T5 出对照报告，四条结论与锚由 `spec-steward` 逐条打开复核，见 `design.md`「## 参考项目对照」。

## 承重约束（逐条带锚）

| # | 约束 | 证据锚 |
|---|---|---|
| C1 | `crates/w3-core/` 零 diff：不新增权威状态字段、`SNAPSHOT_VERSION` 仍 3、状态哈希字段集合 / 快照头部布局 / `first_difference()` 不变、命令排序键不变 | `crates/w3-core/src/snapshot.rs:15`（`SNAPSHOT_VERSION: u32 = 3`）/ `crates/w3-core/src/gameplay.rs:859`（`first_difference`）/ `crates/w3-core/src/sim_match.rs:1507` / `openspec/specs/melee-match/spec.md:127-129` / `docs/decisions/0002-determinism.md:55-72`（ADR 0002 §6-8）/ `CODEBUDDY.md:48-54`（§5） |
| C2 | `CommandKind::Spawn` 不含 `DefId`、两族 `Spawn` 字节逐字节一致；种族只经装配层 `EntityId→DefId` 关联 | `crates/w3-core/src/command.rs:14-21`（`Spawn { x, y, hp, speed, attack_damage, attack_range }` 无 `DefId`）/ `crates/w3-client/src/melee/setup.rs:684-700`（`race_does_not_change_authoritative_spawn_commands`）/ `setup.rs:152-156`（`def_ids` 注释） |
| C3 | 新增定义走 `defs_hash` / `content_hash`，是**冻结定义内容变化、不是新权威状态字段** | `crates/w3-core/src/gameplay.rs:144-181`（`compute_defs_hash` 覆盖 units / abilities / items / upgrades）/ `:212-215`（`content_hash()`）/ `crates/w3-client/src/melee/defs.rs:337-366`（`defs_hash_is_fixed_and_content_sensitive`） |
| C4 | R1.4 / R13.6 可移动性过滤：新增建筑 / 资源点 MUST NOT 响应 `Move`；判据由定义层 `UnitDef.speed_raw` 派生的表现层只读映射提供（P8.0 D1），MUST NOT 扩展 `WorldView` / `EntityView`、MUST NOT 用运行时 `speed`（此二项为 P8.0 / P8.1 阶段性实现层约束，2026-11-16 经用户拍板由 change `runtime-entity-movability` 放松为只读视图分层判据，见本纪要「归档前 delta 禁令消解与指纹重算」小节）、MUST NOT 回退建筑类型白名单（至今有效） | `docs/references/WC3-MELEE-RULES.md:93-98`（R1.4）/ `:300-304`（R13.6）/ `crates/w3-client/src/melee/defs.rs:143-153`（`unit()` 写 `speed_raw`）/ `:160-161`（建筑 `speed_raw==0`）/ `:392-411`（`derive_movability_reads_speed_raw_not_type_names`）/ `openspec/changes/archive/2026-11-15-p8-faction-foundation/design.md:15-29`（D1） |
| C5 | 反向验证（改坏验红）：新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照） | `CODEBUDDY.md:52`（§5）/ `openspec/config.yaml:68`（`rules.tasks` 第三条）/ `openspec/changes/archive/2026-11-15-p8-faction-foundation/design.md:45-54`（D3 反例元断言先例） |
| C6 | 零资产 / 数值 `[待核]` 不冻结冒充原版：全为合成 / 原创编译期整数常量；不复制参考项目单位名 / rawcode / 数值 | `docs/decisions/0001-license.md` / `docs/decisions/0004-frozen-coordinates.md` / `docs/references/WC3-MELEE-RULES.md:7` / `:340`（`[待核]` 以本机 SLK 核对）/ `docs/references/UPSTREAM.md:79`（矩阵 T5「原创三族单位名 / 数值不借」）/ `godot_warcraft3/docs/design/game/GAMEPLAY_VERTICAL.md:39-49`（`Hamg` / `hfoo` / `hlum` / `hrif` 为暴雪 rawcode）/ `godot_warcraft3/packages/gameplay/catalog/melee_race_preview.gd:51-78`（`htow` / `hpea` 为暴雪 rawcode） |
| C7 | 1.0 范围：本 change 落在 S1（局域网联机对战，含真人 VS AI 玩家）/ S3（4 种族做完）的**过程分期**；联机 / 回放 / AI 玩家 / 迷雾是「已在能力或 1.0 范围」，不是不做项；N1–N6 不得纳入 | `docs/references/WC3-MELEE-RULES.md:21-49`（§0.1 S1–S6 / §0.2 N1–N6）/ `docs/roadmap/P8-human-orc-melee.md:7`（范围上位说明）/ `:30-32`（联机口径）/ `CODEBUDDY.md:8`（§1） |
| C8 | P8.1 单 change 不并 P8.2–P8.5：兽族 / 双族对抗 / HUD / 深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）由后续独立 change 承接 | `docs/roadmap/P8-human-orc-melee.md:92-131`（§3 P8.1–P8.5）/ `:84`（不并行多个阶段）/ `:161`（P8.1 与 P8.2 串行） |

**归因声明**：C1 的「新增权威状态字段须同步进哈希 / 快照」硬门禁出处是**本项目** ADR 0002 §6/§8 + `CODEBUDDY.md` §5 + 主 `melee-match/spec.md:127-129`，**不是 `vera20k`**；不得冒充参考项目结论（见 `design.md`「参考项目对照」T1 归因纠正）。C6 中来自参考项目的锚均指向 `UPSTREAM.md` 矩阵条目 T4 / T5 或参考项目内可核验路径（`UPSTREAM.md:84-85`）。

## 拍板决策（含被否备选与理由）

- **P1 用户 2026-11-15 指令：做 1.0 游戏业务，别扯不重要的事**。用户在本会话明确要求把精力放在 1.0 游戏业务（人族完整化）上、不为不重要的事扩范围。证据形态：**本会话用户消息（非仓库文件，由人工核对）**——依 `openspec/decision-memo.template.md:8-10` 三分判据第 3 类，仓库内无可核验 `file:line` 锚，**不冒充已落库文档**。该指令是**方向与范围授权**，不是对 P8.1 具体技术方案（D1–D7、capability 划分）的逐条批准。方向落地锚：`docs/roadmap/P8-human-orc-melee.md:92-97`（P8.1 水位）/ `docs/references/WC3-MELEE-RULES.md:21-30`（S1 / S3）。被否：把联机底座 / 回放 / 编辑器 / 工具链调参塞进本 change（与「别扯不重要的事」冲突，见 R5）。
- **P2（AI 推荐，待规格门拍板，MUST NOT 写成用户已逐条确认）P8.1 技术方案 D1–D7**：人族段 `1xx` 新增 6 类 `UnitDef`（远程 / 反重甲 / 人口 / 祭坛 / 研发 / 英雄）+ 1 主动技能（既有 `EffectKind` / `EffectDef` 组合 + 既有 `Cast`）+ 1–2 项 `UpgradeDef`（既有 `Research`）+ `Race` 分派扩展（定义层 / 装配层，不新增 `Player.race`）+ `setup.rs` 的 `def_ids` 关联（`Spawn` 字节不变）+ `ai.rs` 扩展 + CPU 端到端一局 + 反向验证。锚 `design.md` Decisions / `specs/human-faction/spec.md` / `specs/unit-orders/spec.md`。**技术方案为 AI 推荐，待规格门由人拍板**；被否备选见 R1–R5 与 `design.md` 各决策「备选方案」。具体数值为合成原创 `[待核]`（见「待确认项」）。
- **P3（AI 推荐，待规格门拍板）Capability 划分**：新增 `human-faction`（人族定义与开局装配 + 完整一局），`unit-orders` 增加 ADDED Requirement（人族新单位 / 建筑的 R1.4 可移动性 + 端到端一局）。锚 `proposal.md` Capabilities。状态同 P2。

## 被否方向台账

- **R1** 改 `w3-core` 加 `Player.race`（或任何运行时权威状态字段）承载种族 → 否决：C1 / ADR 0002 §6-8；新增权威状态须同步进状态哈希 / 快照 / `first_difference()`、可能升 `SNAPSHOT_VERSION`，违反主 `melee-match/spec.md:127-129`。锚 `design.md` D3 备选 1 / `openspec/changes/archive/2026-11-15-p8-faction-foundation/decision-memo.md:31`（R2 同方向已否）。
- **R2** 给 `CommandKind::Spawn` 加 `DefId` 字段以承载种族 → 否决：C2；改 `w3-core` 命令线格式 / 状态哈希 / 快照，两族 `Spawn` 字节不再一致，违反内核零 diff。种族只经装配层 `def_ids` 关联。锚 `design.md` D4 备选 1 / `crates/w3-core/src/command.rs:14-21`。
- **R3** 照搬 Warsmash rawcode（`AHfs` / `Aslo` / `urac` 等）或 godot 原创单位名 / 暴雪 rawcode（`htow` / `hfoo` / `Hamg`、`FACTION_UNIT_ROSTERS.md` 三族名册） → 否决：C6 / ADR 0001；AGPLv3 仅借鉴语义不复制代码，原创三族不是经典人族复刻，暴雪 rawcode 属资产派生标识不得入库。锚 `design.md`「参考项目对照」T4/T5 不借鉴边界 + C6。
- **R4** 把 P8.5 深度玩法（英雄复活 / 主城升级 / 野怪 / 物品商店 / 迷雾）塞进 P8.1，或为表达英雄技能新增 `EffectKind` / 命令变体 / JASS VM → 否决：C8 / N5；深度玩法由五个独立 change 承接；新增命令变体 / JASS VM 会碰权威状态，MUST 另走 ADR 0002 流程，不得借「零改动」旁路。锚 `docs/roadmap/P8-human-orc-melee.md:119-131` / `docs/references/WC3-MELEE-RULES.md:46`（N5）/ `:304`（R13.6 范围限定）/ `design.md` D2 备选 1-2。
- **R5** 本 change 顺带重做联机底座 / 回放 / 迷雾 / HUD / 工具链调参 → 否决：C7 / C8；联机 / 回放属 1.0 已在能力（S1 / S2），迷雾属 1.0 范围但由 P8.5e 承接，HUD 由 P8.4 承接；用户 2026-11-15 明确「别扯不重要的事」。锚 `proposal.md` Impact / `design.md` Non-Goals。

## 待确认项（如实标注，不冒充已拍板）

- **规格门是否批准**（含 P2 技术方案、P3 capability 划分、参考项目对照结论与参考缺口）→ **已由用户 2026-11-15 指令批准（见规格门小节）**。`critic` 规格门复核实质通过（清单第 7 / 10 / 12 / 13 项）；`status: approved` 与四件套指纹已由 `spec-steward` 依据用户明确指令回填，非 AI 自行批准、非沉默当授权。本条只记规格门批准，下列数值 `[待核]`、运行时动态实体承接、P8.2–P8.5、参考缺口、真窗口人工验收等仍如实为待承接项，不冒充已解决。
- **具体数值为合成原创 `[待核]`** → 人族新单位 / 建筑 / 英雄 / 技能 / 升级的 hp / mana / damage / range / supply / train_frames / cooldown / research_frames 全为编译期整数常量，**不冻结冒充原版**；须以玩家自备正版客户端经 `godot_warcraft3` 导出的本机 SLK 表核对（不入库，`config.toml` 只作表现层接缝）。锚 `WC3-MELEE-RULES.md:340` / `design.md:17`（Decisions 引言 `[待核]` 声明）与 `design.md` Risks 数值 `[待核]` 段 / `godot_warcraft3/docs/roadmap/CLASSIC_RULES_BASELINE.md:3,52`（先核来源再写数值、数据行完整 ≠ 玩法完成）。
- **运行时动态实体承接** → P8.0 已知限制：`CommandKind::Spawn` 不含 `DefId`、运行时 `Train` / `Build` 新实体不在装配期 `def_ids` 关联中，其可移动性在「`w3-core` 零改动 + 不扩展视图」下无法判定。本 change 新增大量可训练单位 / 可建造建筑，**须回到人拍板**由承接 change 决策（是否统一 `unwrap_or` 语义、如何把运行时新实体纳入装配期映射），不臆造、不借本 change 旁路。锚 `openspec/changes/archive/2026-11-15-p8-faction-foundation/decision-memo.md:52`（B1）/ `docs/roadmap/P8-human-orc-melee.md:160`。（2026-11-16 据实更新：该「承接 change」即 `runtime-entity-movability`，已由用户拍板后归档、以只读 `EntityView.speed` 分层判据承接运行时动态实体，旧「不扩展视图」实现层约束已放松；**本 delta 的过时禁令已消解、`p8-human-faction` 归档时不会再污染主 spec**。本条历史待确认记录保留、不删改；本 change 仍 15/20 tasks、真窗口人工验收等待承接项不变，详见「归档前 delta 禁令消解与指纹重算（2026-11-16）」小节）
- **P8.2–P8.5 后续 change** → 兽族完整化 / 双族对抗与 AI / HUD / 五个深度玩法各自单独 change、串行推进；本 change 不承接、不在此时排期细节。锚 `docs/roadmap/P8-human-orc-melee.md:99-131` / `:161`。
- **`file:line` 锚漂移登记** → C1–C8 的 `file:line` 锚在成文前逐条打开核实；apply 后行号可能漂移，按 `CODEBUDDY.md:107` 规格锁定与重审处理：四件套任一 artifact 改动即解锁、须同步回查其余三份并重走规格门。（2026-11-16 补注：本轮四件套改动为纯规则措辞小改动、按 `CODEBUDDY.md` §9 省略重走规格门但已同步回查其余三份、指纹取真值重算回填，处置见「归档前 delta 禁令消解与指纹重算（2026-11-16）」小节；此为 §9 明文例外，不视为违反本条）
- **参考缺口（四项目均无现成做法）** → 编译期整数 DefId 按种族分段且不引入运行时权威状态、1 英雄 + 1 技能最小集、`Spawn` 不含 DefId 用装配层 `def_ids` 关联、CPU 无渲染逐帧哈希一局脚本：属本项目原创取舍，**待规格门拍板**，不臆造、不冒充参考结论。锚 `design.md`「参考项目对照」末「参考缺口」。
- **人工边界** → 本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀）；push / fetch / pull / clone 及任何远端同步人工独占（`CODEBUDDY.md` §4 / §10）。真窗口人工验收不在本轮执行（本 change 为 CPU 侧定义与闭环），如实标注「未执行 / 待补验」。

## apply 已知限制与代码门结论（2026-11-15，如实登记，不冒充通过）

- **本机 GPU headless 原生崩溃（环境项，非 P8.1 引入）**：`cargo test --workspace` 与完整 `python tools/verify_all.py` 在 `w3-client/tests/real_map_e2e.rs` / `w3-render/tests/gpu_headless.rs` 的真 GPU 测试发生 `0xc0000005` / `0xc0000374` 原生崩溃，`[fail] cargo test`，**未达 `Verification OK`**。这些测试文件相对 HEAD 零改动、对本次新增符号零引用；`client-owner` 已用干净 HEAD worktree + 独立 `CARGO_TARGET_DIR` 复现同样崩溃，本机 `config.toml` 指向真实导出资产时触发、CI 无资产条件优雅跳过。CPU 侧 `cargo test --workspace -- --skip <4 个 GPU 测试>` 28 个 `test result: ok` / 0 FAILED、`determinism 26/26`、`cargo fmt --all -- --check` / 编码 262 文件 / 源码约定 86 文件 / `git diff --check` / `spec_gate_check()` / `decision_memo_check()` / `openspec validate --strict` 全部通过。**完整门禁全绿与 CI 双平台复核留人工 push 后确认，本地 28/28 不冒充 100% 全绿**。未改门禁逻辑、未删测试、未绕过规格门。
- **tasks 3.5 / `KeyM` 键位接线（承接，不冒充已解决）**：实现只覆盖既有共享输入翻译接缝 `ContextAction` / `filter_movable`（右键 + 命令卡 `Move`）；`design.md` D7 明确本 change 不重开键位接线、`app.rs` 的 `KeyM` 现状仍为相机平移，`WC3-MELEE-RULES.md` R13.7 已登记 `KeyM` 差距由后续 change 承接。3.5 保留未勾、不新增 `InputAction` 变体、不改键位；不擅自改四件套解锁规格门。frame-0 静态混合集合的 R1.4 过滤已在 `tests/movability.rs` 覆盖。
- **P8.0 B1 未擅自闭合**：运行时 `Train` / `Build` 动态实体承接（见上「运行时动态实体承接」条），未改 `unwrap_or` 语义、未扩 `WorldView` / `EntityView`。
- **`SetAi{enabled:false}` / `Leave` 对开发夹具 `examples/melee.rs` 的定时离席副作用（residual，交后续承接）**：`ai.rs` 扩展的 AI 脚本显式关闭内核内部 AI、在规划窗口末尾提交 `Leave`，对零改动的 `examples/melee.rs` 表现可能形成定时离席；`examples/melee.rs` 是开发夹具、不列 1.0 验收线，真窗口表现需 P8.4 / 人工验收时复核。不影响 CPU 侧确定性契约。
- **代码门结论**：只读 `critic` 全清单复审 **0 blocker、可进入本地 commit**；反向验证为真 A-B（退化定义漏接新定义 / 把建筑 `speed_raw` 改非 0 会让验收断言在缺陷态失败、修复态通过），非 trivially-green。`crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍 3、规格锁定指纹 `02ecdfc…` 未失配。四件套正文零改动、`tasks.md` 仅复选框归一化（不解锁）。
- **数值 / 真窗口 / 参考缺口**：仍为上面「待确认项」中登记的合成原创 `[待核]` / 未执行待补验 / 项目原创取舍，归档前按停止条件逐条复核。

## 归档说明（2026-11-16，用户明确豁免真窗口人工验收）

> 本小节为纯文档登记，不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照、不含资产（ADR 0001 / ADR 0002）。证据锚均为仓库文件或本会话用户拍板（非仓库文件者由人工核对，不冒充已落库文档）。

- **用户拍板（非沉默推定）**：2026-11-16 本会话用户明确指令「先不人工测，只标记未人工测试然后归档，然后新建后续 change 继续朝 1.0 的最终目标迭代开发」。证据形态：本会话用户消息（非仓库文件，由人工核对）。据此**显式豁免**「真窗口人工清单未逐项勾选前不得归档」约束，与归档 `2026-11-14-melee-playable-loop/smoke-checklist.md` 的 2026-11-14 用户明确豁免先例一致；不拿沉默当授权、不冒充通过。
- **真窗口人工验收：全部「未执行 / 待补验」，如实标记、不补测、不冒充通过**。本轮梳理出的剩余真窗口人工项统一登记为未人工测试：① 上版「右键只移动可移动单位」bug 修复后的真窗口复验（用户 2026-11-14 真窗口发现主城 / 金矿随农民移动、明确豁免归档；此后 P8.0 `p8-faction-foundation` 与已归档 `runtime-entity-movability` 仅在 CPU 侧 `tests/movability.rs` 修复并回归，真窗口从未复验）；② 含运行时 `Train` / `Build` 产出建筑的混合选中集合右键 / `M` 移动（B1 承接，仅 CPU 侧验过）；③ 路线图 `P8-human-orc-melee.md:149` / `:117` §5 验收线「能框选、能打完一局并看到 `VICTORY`」（历史 `smoke-checklist.md:88` E 组标未执行；CPU `melee_loop.rs` 验过结算，egui overlay 的 `VICTORY` 字样未人工确认）；④ 人族 6 类新实体在真窗口的可见性 / 命令卡随类型变化 / `KeyM` 等经典键位两级 chords（P8.1 新定义只在装配层 `category_def_ids` 查表、不改开局 `Spawn` 字节，frame-0 仍是 12 个实体；运行时新实体走「模型缺键回退」四面体占位；HUD 属 P8.4、`KeyM` 差距见 `WC3-MELEE-RULES.md` R13.7）；⑤ `SetAi{false}` / `Leave` 对开发夹具 `examples/melee.rs` 的定时离席副作用；⑥ 归档 `smoke-checklist.md:84-90` 标「未执行 / 部分」的 A1–A5 / B1–B10 / D1–D2 / E1–E6 / F1–F3 / G1–G3 组。P8.1 为 CPU 侧定义 + 端到端闭环，CPU 侧已覆盖定义 / `defs_hash` / 反向验证改坏验红 / 逐帧哈希一局 / `cargo test --workspace` / CI 三 job 全绿（`run 43851211008`），真窗口 / GPU / egui overlay 一律未人工观察，**CI 绿灯不代替真窗口验收**。
- **15/20 tasks 未勾原因（如实、不冒充已勾也不冒充未做）**：3.5 因 `app.rs` 的 `KeyM` 未接线为移动、`design.md` D7 明确本 change 不重开键位接线、frame-0 静态混合集合的 R1.4 过滤已由 `tests/movability.rs` 覆盖（后续 change 承接 / R13.7）；4.2 / 4.4 / 4.5 / 4.7 为流程收尾项——`determinism 26/26`、`verify_all.py Verification OK`、只读 `critic` 全清单 0 blocker、中文描述 commit + CI 全绿在 P8.1 apply 与本轮门禁中**实际已执行且有证据**，复选框刻意保持未勾以守「四件套 0 diff / 规格锁定不解锁」（与归档 P8.0 先例一致）。归档工具 `Warning: 5 incomplete task(s)` 由 `-y` 跳过、如实保留。
- **delta 合并去向**：归档后主 `openspec/specs/` 新建 `human-faction` capability、`unit-orders` 并入 ADDED Requirement「人族新增实体沿用可移动性过滤并覆盖端到端一局」（该 delta `:7` 的过时全局禁令已于 2026-11-16 归档前消解为实体来源分层判据，见「归档前 delta 禁令消解与指纹重算」小节，**不污染主 spec、不复活互斥**）。
- **已知限制承接（不冒充已解决）**：真窗口人工验收 A–F 组、本机 GPU headless 间歇性原生崩溃、经典键位接线（R13.7 / 后续 change）、运行时新实体「模型缺键回退」四面体占位、命令卡不随人族新类别变化（P8.4 HUD）、`SetAi` / `Leave` 定时离席、中文上屏（CJK 字体许可证按 `CODEBUDDY.md` §10 人工拍板）、数值 `[待核]` 以本机 SLK 核对、参考缺口 4 项、P8.2 兽族 / P8.3 双族对抗与 AI 选族 / P8.4 HUD / P8.5 五个深度玩法（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）——由后续独立 change 承接，本 change 不扩范围、不补测、不改代码。
- **人工边界**：本地 commit 由 AI（中文描述 + Conventional Commits 英文 type 前缀），push / fetch / pull / clone 及任何远端同步人工独占（`CODEBUDDY.md` §4 / §10）。

## 停止条件（追加阶段 / 归档前）

1. C1–C8 逐条仍有证据锚、无一条被本轮改动证伪。
2. 规格门由人拍板、`fingerprint:` 回填、`status: approved`，且 `verify_all.py` 的 `spec_gate_check()` 不报失配（规格门已批准、指纹已回填，见「规格门」小节）。
3. 待确认项已清空或显式豁免；不以「预设问题问完」「问了 N 轮」当收敛。
4. 四件套一致性按 `CODEBUDDY.md:88` 回查：proposal 的 Scope / specs 的 `#### Scenario:` / design 的 alternatives / tasks 的步骤互不打架。
5. 归档时：delta 合并去向、未勾任务原因、已知限制承接如实登记。（2026-11-16 据实更新：delta `specs/unit-orders/spec.md:7` 的过时全局禁令已在归档前消解为实体来源分层判据，**本 delta 已消解；`p8-human-faction` 归档时不会再污染主 spec**，不再需要「主 spec / 后续 change 承接字面禁令」；详见「归档前 delta 禁令消解与指纹重算（2026-11-16）」小节）

## 归档前 delta 禁令消解与指纹重算（2026-11-16）

> 本小节为纯文档登记，不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照、不含资产（ADR 0001 / ADR 0002）；证据锚均为仓库文件或本会话用户拍板（非仓库文件者由人工核对，不冒充已落库文档）。

- **冲突来源（顺序风险，`critic` R5）**：本 change 仍 active（15/20 tasks），delta `specs/unit-orders/spec.md:7` 的 ADDED Requirement 正文末尾仍写 P8.0 全局禁令「系统 MUST NOT 扩展 `WorldView` / `EntityView`、用运行时 `speed` 或回退建筑类型白名单」。若 `p8-human-faction` 将来原样归档，该句会随 ADDED 追加进主 `openspec/specs/unit-orders/spec.md`，与已归档 change `runtime-entity-movability`（`openspec/changes/archive/2026-11-16-runtime-entity-movability/`）消解后的主 spec `:39`「实体来源分层判据」及新增 Requirement「群体移动过滤覆盖运行时动态实体」再次互斥。锚 `openspec/specs/unit-orders/spec.md:39` / `:56-58` / `docs/references/WC3-MELEE-RULES.md` R1.4 / R13.6 / §14。
- **用户拍板**：2026-11-16 本会话 `AskUserQuestion` 明确选择「现在根除」顺序风险（即在指纹内现在改四件套消解，不留到归档后再处置）。证据形态：本会话用户消息（非仓库文件，由人工核对）；不是 AI 自决、不是沉默当授权。
- **机制（改四件套 → 省略重走规格门）**：依 `CODEBUDDY.md` §9（`:107`）「目标与方向已清楚的小改动、纯文档 / 规则改动可显式声明省略规格门，但人工清单如实标注与 `verify_all.py` 门禁不可省」——本轮是纯规则措辞消解、用户已拍板方向、由 `critic` 聚焦复核承接（leader 另委派），故**省略重走规格门**（不重新出报告 / 不重新逐条拍板技术方案）；`status: approved` / `approved_by` / `date` 保持原样，`fingerprint:` 取 `python -B tools/verify_all.py --fingerprint p8-human-faction` 真值重算回填；人工清单如实标注（本小节）、`verify_all.py` 的 `decision_memo_check()` / `spec_gate_check()` / `check_encoding.py` / `openspec validate` / `git diff --check` 门禁不省。未新增 `## 省略说明` 小节（否则会整体豁免规格门、与「门禁不可省」冲突）。
- **改了哪几处（仅 `openspec/changes/p8-human-faction/` 内 5 文件）**：① delta `specs/unit-orders/spec.md:7` 仅替换 ADDED Requirement 正文末尾过时禁令为与 `:39` 一致的分层判据，保留 Requirement 标题、6 个 `#### Scenario:` 及其余文字、R1.4 核心与白名单禁令；② `design.md:70`（D4 备选 2）/ `:111`（D7 决策）/ `:145`（Risks 运行时动态实体承接）各追加「阶段性、已放松」括注；③ `tasks.md:17`（2.3）追加同类括注、复选框状态不变；④ `proposal.md:29`（代码面零改动）追加「本 change 的零改动是 P8.1 当时事实 / 承接由后续 change 完成」诚实标注，`:33`（P8.5 权威状态例外的 ADR 0002 常设硬门禁）追加「未放松、属不同门禁」的区分标注；⑤ `decision-memo.md` 本小节 + 待确认项 / 停止条件据实更新 + `## 规格门` 指纹重算回填 + `reason:` 追加一句。
- **边界（未触碰，可复核）**：主 `openspec/specs/`（任何文件）、`openspec/changes/archive/`、其它 change、`crates/`、`tools/`、`.github/`、`CODEBUDDY.md`、`openspec/config.yaml`、`docs/`、`.codebuddy/`、`.openspec.yaml` 一律零改动；`tasks.md` 复选框状态（15/20）不变；Requirement / `#### Scenario:` / 决策条目编号不新增不删除；不 commit、不 push / fetch / pull / clone（人工独占）。主 specs 的变化仅来自 leader 本轮之前已完成的 `runtime-entity-movability` 归档，与本 change 无关。
- **结论**：**本 delta 已消解；`p8-human-faction` 归档时不会再污染主 spec**。
- **遗留（不变，不冒充已解决）**：本 change 仍 15/20 tasks（3.5 / 4.2 / 4.4 / 4.5 / 4.7 未勾），其真窗口人工验收等待承接项不变；数值 `[待核]`、参考缺口、P8.2–P8.5 后续 change 仍按原待确认项如实登记。

## 规格门

- status: approved
- fingerprint: 988193fcd22b4a5446272858736d2d5cc4586c87d678be61abbb3e95dde20688
- approved_by: 用户（2026-11-15 本会话指令「做 1.0 游戏业务，别扯这些不重要的事」授权 P8.1 立项并实施；证据形态：本会话用户消息、非仓库文件，由人工核对，不冒充已落库文档）
- date: 2026-11-15
- reason: `critic` 规格门复核实质通过（技术方案 C1–C8 / 参考对照 T1/T2/T4/T5 / 规则编号 R-x.y / 待确认项如实性全部站得住）；B1 编号笔误已订正；P2/P3 技术方案随批准采纳进入 apply；数值 `[待核]` / 运行时动态实体承接 / 参考缺口 4 项 / 真窗口人工验收未执行，均如实登记承接、非冒充通过。`verify_all.py` 的 `spec_gate_check()` 不报失配。2026-11-16 纯规则措辞消解 delta 过时禁令、按 `CODEBUDDY.md` §9 省略重走规格门、`fingerprint:` 取真值重算回填（`988193fc…`）、`status` / `approved_by` / `date` 保持原样，待 `critic` 聚焦复核。
