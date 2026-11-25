# Tasks

> 本文件为按用户 2026-11-24 规格门逐条拍板定案口径收敛修订的实施清单，**本轮不执行任何任务**、全部复选框保持 `- [ ]` 未勾。Q1–Q8 / Q10 已由用户规格门逐条拍板定案（Q1 改按备选「SLK 清单逐项对齐」+ Q1↔Q2 调和、全量边界、Q5 三者同步 8→9、其余 7 项按推荐）；Q9 / Q-A 已拍板形态与顺序、属流程约束；`critic` 规格门报告 9 条 B 类 residual 按用户拍板「批准前修订再锁定」逐条吸收。`## 规格门` 保持 `status: pending`，批准记录由 leader 回填后进入 apply。

## 1. 规格门前处置与活文档细化（Q-A / Q9 / Q10）

- [x] 1.1 规格门处置：保持 `decision-memo.md`「## 规格门」为 `status: pending`、不填 `approved` / fingerprint / `approved_by` / `date`、不写「## 省略说明」；用户已逐条拍板定案、`critic` 规格门报告 `blocker: 0` 已出、9 条 B 类 residual 已批准前修订，由 leader 回填批准记录与四件套指纹（`status: approved` + `fingerprint` + `approved_by` + `date`）。验证：`python tools/verify_all.py` 的 `spec_gate_check()` 对 `pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed 信号，不绕过。
- [x] 1.2 Q10 验锚：按 `critic.md:31` 清单第 12 项逐条打开 `design.md`「## 参考项目对照」的可核验锚，确认 T1 承重锚一律为 `vera20k@305ea3a75:<path>:<line>` 双标、未退化成工作树 HEAD 行号（`926a036e`）；确认未把 Warsmash AGPLv3 代码 / `double` 路线 / 原创三族 / RA2-YR 设定 / README 自述带进来。验证：抽查锚可打开且 `design.md` 命中 T1/T3/T4 均落在对照小节，无 blocker。
- [x] 1.3 Q-A 活文档细化（**与 P11 同批、但按用户 2026-11-24 已拍板顺序在规格门批准记录回填后的 apply 阶段由 `docs-owner` 落定；本期 `docs/references/WC3-MELEE-RULES.md` 正文零改动**）：S3 下细化「英雄 / 单位技能全量」、附录「规则 → 实现覆盖对照」新增 R6 技能独立行、§15 变更记录追加一条，**不新增 S 编号**；按 §14 第 2 条注明 **2026-11-24** 日期与依据、不静默改规则。验证：`git diff -- docs/references/WC3-MELEE-RULES.md` 在本期为空；apply 阶段该文档 S3 / 附录 / §15 三处出现且仅追加、不改既有 `R-x.y` 条文与状态标记。

## 2. 定义层与四族技能体系（Q1 / Q2）

- [x] 2.1 按 Q1 已拍板定案（改按备选「按本机导出 SLK 经典具名清单逐项对齐」）：四族技能**清单**逐项按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）人工核对后，**固化成编译期常量表**进 `FrozenDefinitions`；本期范围 = 四族英雄技能全量（经典清单结构每族 4 英雄 × 4 技能 = 英雄 3 普通 + 1 个 6 级大招，含技能点 / 等级 / 学习 / 升级）+ 各族代表性单位 / 道具技能，**中立英雄 / 野怪 / 全部单位技能顺延、如实标已知限制**（`decision-memo.md`「已知限制」/ `design.md`「Risks」/ `spec.md` Req1 Scenario）。两层张力：对齐的是**清单条目与分类**（哪些技能、每族几个、英雄 3 普通 + 1 大招、单位 / 道具技能），**不是照搬经典名称 / 四字符 rawcode / 数值**；实际清单条目数在 apply 阶段人工核对后如实登记、不臆造「四族全量 = 某精确数字」（规模元数据只见 `design.md`「清单规模事实锚」、粗统计以人工核对为准，`WC3-MELEE-RULES.md §14`）。验证：`compute_defs_hash`（`gameplay.rs:434`）把新定义纳入 `defs_hash` / `content_hash` 快照头部、与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）；`crates/w3-client/src/melee/defs.rs:1-6` 仍纯编译期零文件 IO；`cargo test -p w3-core --lib` 全绿。
- [x] 2.2 按 Q2 已拍板定案（Q1↔Q2 调和口径「清单对齐 SLK、标识仍合成」）：内部仍用合成原创 `DefId` 作键（沿用 `defs.rs:120,122,124,279,281` 的 `ABILITY=10` / 英雄 `11–14` 并扩四族），**不引入「四字符 ID / 具名技能名 → `DefId`」映射层**；数值一律 `[待核]`、不冻结进权威定义、以本机导出 SLK 人工核对、名称合成原创不冒充经典具名技能、SLK 四字符 ID 只作核对依据且 MUST NOT 进权威状态 / 状态哈希 / 快照 / 不入库（`WC3-MELEE-RULES.md:124,351`、ADR 0001 / 0002 / 0004）。验证：`crates/w3-client/src/melee/defs.rs` 无四字符 ID 常量与映射层，`python tools/verify_all.py` 资产扫描通过、`git ls-files` 无 `*.slk` / 导出 JSON / PNG / GLB。
- [x] 2.3 道具技能按 R7.6 走既有 `EffectDef` 组合结算、MUST NOT 直接执行任意 JASS 技能（N5）。验证：`Settlement`（`gameplay.rs:851-853`）为唯一结算路径、`gameplay.rs:2318` 掉落表 EffectDef 段进定义哈希；`crates/` 内无 `war3map.j` / 触发器 VM 引用；`cargo test -p w3-content` 全绿。

## 3. 行为原语注册表与 EffectKind 最小扩展（Q3）

- [x] 3.1 按 Q3 已拍板定案（按推荐）扩展最小必要 `EffectKind` 原语集（持续 AOE / zone、召唤、光环受益集、被动 proc 订阅、变身 / 状态切换、点目标、buff 分类移除、链接弹射、随机消费），稳定码只追加、旧码 0–3 不动。验证：`effect_kind_code`（`gameplay.rs:416-421`）旧码逐字节不变、新码唯一；`cargo test -p w3-core --lib` 全绿。
- [x] 3.2 新增单一 `BehaviorCatalog` 注册表（OCP）：按行为类型查表分派、注册新行为 = 扩展非散落 `if behavior == ...` 分支；对应 Data / Logic / Present 分层与 `Settlement` 单一结算所有者（结论 G6）。验证：新增一个原语不改动既有分派分支（代码 review + 单测），SRP / OCP / DIP 由 `critic` 复核。

## 4. 命令面、目标形态与 proto 纪律（Q3 / Q5 / G8）

- [x] 4.1 施法走 `GameplayCommandKind::Cast`（点目标 / 范围 / 方向按「只追加 kind 码 / proto tag」扩展）；建造走 `Train { is_building: true }`；攻击 / 移动 / 停止走 `CommandKind::{Attack, Move, Stop}`；MUST NOT 写成 `GameplayCommandKind::Attack` / `::Build`（不存在）。验证：`gameplay.rs:1322-1546` / `command.rs:11-49` 复核、穷尽分发 + 同源校验 + `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序单测全绿。
- [x] 4.2 R1 可移动性回归：框选含金矿 / 树木 / 建筑 / 资源点时 MUST 只为可移动单位生成 `Move`、这些实体 MUST NOT 响应 `Move`、落点按稳定 `EntityId` 顺序。验证：`cargo test -p w3-core --lib` 的 R1 / `unit-orders` 相关回归全绿（R1.1 / R1.4 / R13.6）。
- [x] 4.3 新增技能命令 / 效果字段按 ADR 0005 §3：`w3net.proto` 只追加 tag（kind 码 37 起 / proto tag 59 起）、`reserved 14 to 32` 与 `EffectDef reserved 4 to 8` 保持、禁 map / float / double、unknown fields 不透传、canonical 往返校验、CI 双平台生成代码字节一致。验证：`to_pb_gameplay_kind` / `from_pb_gameplay_kind` 双向穷尽映射一一对应、`cargo test -p w3-net` 全绿。

## 5. 新增权威运行态四处同步（Q3 / Q5 / ADR 0002 §6-8）

- [x] 5.1 为每个新增权威运行态（zone / 召唤 / 光环受益集 / 被动 proc / 变身 / buff 分类 / 链接 / `rng_draws`）用 `BTreeMap` / `BTreeSet` 稳定序、按稳定 `EntityId` 顺序，四处同步写入 `compute_state_hash`（`sim_match.rs:3784`）、`capture_snapshot`（`:3941`）、`restore_snapshot`（`:4038`）、`first_difference`（`:3861`），并同步 `WorldGameplayState::hash`（`gameplay.rs:1994-2059`）/ 其 `first_difference`（`:2061-2077`）/ restore 五重校验（`:4043-4064`）。验证：四处字段集合一致、`cargo test -p w3-core --test determinism` 全绿。
- [x] 5.2 定义层冻结常量（技能清单 / 行为分类 / buff 分类；数值一律 `[待核]`、不冻结进权威定义）经 `compute_defs_hash`（`gameplay.rs:434`）纳入 `defs_hash` / `content_hash` 快照头部，与新增权威运行态严格区分、MUST NOT 混入 `compute_state_hash()`（`0002-determinism.md:111,115,117`）。验证：改任一冻结常量即改变 `content_hash()`、不改运行时 `compute_state_hash()`；`cargo test -p w3-core --test determinism` 全绿。
- [x] 5.3 表现层只读派生：技能特效 / HUD / 冷却显示 MUST NOT 反写权威状态、不进哈希 / 快照 / `first_difference()`；动画丢帧 / 资源加载失败不改模拟（ADR 0002 §5 / R10.5）。验证：`cargo test -p w3-core --test determinism` 中表现层只读断言全绿、无 `w3-core → w3-client` 反向依赖。

## 6. 随机流消费契约（Q4）

- [x] 6.1 按 Q4 已拍板定案（按推荐）复用单条 match-owned 整数 LCG、每帧恰好消费一次，技能 proc / 闪避 / 暴击 / 链接随机以显式 `rng_draws` 编码进快照 / `first_difference()`；MUST NOT 新增 per-skill 独立流、表现层使用独立随机源（P8.5c 先例 `0002-determinism.md:113` / ADR 0002 §4）。验证：两实例交错帧流 `rng_draws` 序列逐帧相等、`first_difference()` 返回 `None`、恢复后续算不重复；`cargo test -p w3-core --test determinism` 全绿。

## 7. bump 9 全套、版本拒绝、golden hash 与确定性验收（Q5 / Q6 / Q8）

- [x] 7.1 按 Q5 已拍板定案（三者同步 8→9）把 `SNAPSHOT_VERSION` / `SIM_VERSION`（`snapshot.rs:56-57`）/`PROTOCOL_VERSION`（`packet.rs:45`）三者同步 bump **8→9**（**待执行动作、执行前为 8**）；快照头部、单测硬断言、ADR 复核记录同步更新。验证：三版本号一致 == 9、`cargo test -p w3-core --test determinism` 全绿（`0002-determinism.md:117` / `0005-protobuf-payload.md:87` P9 先例）。
- [x] 7.2 旧 v8 明确拒绝、不提供跨版本迁移：v8 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch`、`Packet::decode`（`packet.rs:154-156`）对 v8 返回 `VersionMismatch` + relay `Reject`（在解码 payload 之前按版本拒绝）。验证：旧版本拒绝单测全绿、`cargo test -p w3-net` 与 `cargo test -p w3-relay` 全绿（ADR 0002 §7 / ADR 0005 §3）。
- [x] 7.3 golden hash 按 v9 新实现**如实重取、不沿用 `0x0b53a047b1f8e5e0`**（`golden_hash.rs:75-77`）；按 Q6 已拍板定案（按推荐）把 Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对登记为「待 VM / 人工」，**不得声称 bump 9 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118`）。验证：Windows 侧 `final_hash` 为 v9 重取值、`crates/w3-core/examples/golden_hash.rs` 注释与 ADR 0002 复核记录如实、无「已验证全平台」措辞。
- [x] 7.4 重跑 `cargo test -p w3-core --test determinism`（确定性硬门禁，任何改动权威状态 / 帧序 / 命令排序 / 哈希编码 / 快照字段 MUST 保持全绿）+ 四族技能逐帧哈希一致 + 回放一致 + 旧 v8 拒绝 + 快照销毁重建后从 N+1 续算一致。验证：命令输出全绿、`first_difference()` 返回 `None`（`CODEBUDDY.md §5` / R12.4）。
- [x] 7.5 新增确定性回归测试**反向验证（改坏验红）**：按 `CODEBUDDY.md §5` 形态①在测试内构造反例 + 元断言防退化——故意漏写某一新权威运行态到哈希 / 快照 / `first_difference` / restore 之一时断言失败、修复态通过；或保留缺陷态 / 修复态 A-B 对照命令输出。无法构造缺陷态的 MUST 在 `design.md` 登记理由与替代证据。验证：缺陷态测试失败、修复态通过的成对证据可复现，由 `critic.md` 清单第 11 项③复核。
- [x] 7.6 表现层只读派生边界不改（D5 末条）、AI 自主施法本期不含：不新增 AI 施法命令源 / 决策字段，`AiScript` 现有取 `Cast` 目标 ID 的只读夹具不改；真人 / 回放 / 联机 lockstep 三路径对同一 `Cast` 帧流逐帧哈希一致（Q7 已拍板定案·按推荐）。验证：`git diff -- crates/w3-core/` 无 AI 施法决策路径，三路径一致性单测全绿。
- [x] 7.7 ADR 0002 / 0005 各追加一条「复核记录 / 修订记录」（不改写正文与历史基线数值、`reserved 14 to 32` 保持、旧值不删）；`CODEBUDDY.md §5` 形态①反向验证结论如实登记。验证：`git diff -- docs/decisions/` 为纯追加、既有行零改动（ADR 记「为什么决定」、spec 记「系统该做什么」）。
- [x] 7.8 全量门禁：`cargo test --workspace`、`cargo fmt --all -- --check`、`python tools/verify_all.py`、`python tools/check_encoding.py`、`openspec validate p11-ability-system --strict`。验证：全部命令输出成功（CI 双平台 `verify_all.py` 由人工 push 后跑、本会话不预先声称通过）。
- [x] 7.9 **规则文档同步核对**（玩法 change 收尾必填）：在 `decision-memo.md`「## 规则文档同步」小节回答「本次实现是否需同步 `WC3-MELEE-RULES.md`」——需同步则确认 Q-A 已更新附录「规则 → 实现覆盖对照」R6 技能独立行与 §15 变更记录、无需同步则写明依据；**MUST NOT** 把「design 引用 `R-x.y`」冒充「规则已实现」（`:393` 三态判定 / §14 第 5 条 / `critic.md:34` 第 13 项）。验证：小节非空且结论真实、真窗口 8.x 人工清单仍「未执行 / 待人工」、不标通过。
- [x] 7.10 **bump 8→9 全仓版本号消费者同步 + 全仓 Grep 旧版本号无残留验证**（先例 P9 `archive/2026-11-23-p9-undead-foundation/tasks.md:39,53`）：至少覆盖 `CODEBUDDY.md:50`、`openspec/config.yaml:13,59`、`CONTRIBUTING.md:35`；主 spec 无条件 `==8` 断言（`undead-faction/spec.md:104` / `dual-faction/spec.md:82` / `faction-definitions/spec.md:22`）由 `spec-steward` 按 `CODEBUDDY.md §8` 在归档合并时复核、**归档后主 spec 历史 `==8` 字面按 P9 Q9' 已知限制、由后续独立 change 收口**（见 `decision-memo.md`「已知限制」、不作为本期改主 spec 的越权动作）；源码 rustdoc 十余处；硬断言 `crates/w3-core/tests/reverse_validation_faction_mechanics.rs:683` / `crates/w3-client/tests/p8_hud_presentation.rs:1458` / `crates/w3-client/tests/melee_loop.rs:2745`。验证：全仓 Grep 旧版本号 8 在权威版本号消费者侧无残留、三版本号 == 9 全仓一致、`git diff -- openspec/specs/` 在本期提案阶段为空（主 spec 只在 archive 阶段合并）、`cargo test -p w3-core --test determinism` 全绿。

## 8. 真窗口人工清单（待人工，一律 `- [ ]` 未勾）

- [ ] 8.1 真窗口四族技能可施放 / 冷却 / 大招 6 级解锁人工观察（待人工，不用离屏截图 / CPU 回归冒充）。
- [ ] 8.2 真窗口技能特效 / HUD 冷却进度可见性人工观察（待人工；呈现不足按已知限制登记、不扩范围）。
- [ ] 8.3 真窗口回放加载 / 联机对局技能一致性人工观察（待人工）。
- [ ] 8.4 高 DPI / 高刷窗口下技能命令卡交互人工观察（待人工；未执行如实标注，不冒充通过）。

## Workflow follow-up

- 规格门已由用户 2026-11-24 逐条拍板定案（Q1 改备选 + Q1↔Q2 调和 + 全量边界、Q5 三者同步 8→9、其余 7 项按推荐、Q9 / Q-A 已拍板流程约束），`critic` 规格门报告 `blocker: 0`、9 条 B 类 residual 批准前修订已吸收；`status: approved` + 四件套指纹 + `approved_by` + `date` 由 leader 回填、本轮 `status: pending` 不自批。
- apply 由 `sim-owner` / `net-owner` / `client-owner` / `content-owner` 各自目录串行实施，`critic` 代码门复核，本地 commit 描述用中文。
- 人工 push / CI 双平台门禁 / `/opsx:archive` 由人工与 leader 处理；AI 不得 push / fetch / pull / clone、不得 archive、不得本地 commit（本轮统一由 leader 处理）。
