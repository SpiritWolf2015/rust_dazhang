# Tasks

> 本文件为 p13 的实施清单，**本轮只成文、不实施代码**，全部复选框保持 `- [ ]` 未勾，apply 是后续独立会话。本 change 承接 P11「已知限制」第 1 条（顺延项全量）与第 11 条（Q1 清单逐项 SLK 对齐），触及 `w3-core` 确定性内核，`CODEBUDDY.md §5` 硬门禁适用：`cargo test -p w3-core --test determinism` 重跑与形态①「改坏验红」为必做任务（第 5 组）。`## 规格门` 保持 `status: pending`、批准记录由 leader 回填；是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数属「待确认项」，由规格门用户拍板、不臆造为已定案。玩法任务引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 S1–S6、给可执行验证。

## 1. 规格门前置与规则文档细化（Q-A 类，pending）

- [ ] 1.1 规格门处置：保持 `decision-memo.md`「## 规格门」为 `status: pending`、不填 `approved` / fingerprint / `approved_by` / `date`、不写「## 省略说明」；待确认项（是否新增权威状态、是否 bump 9→10、清单实际条目数、验收线）由规格门用户拍板，独立 `reference-researcher` 对照报告已产出、锚已逐条核实、状态文字本批回填，四件套指纹 / `critic` 规格门报告 / 批准记录由 leader / 用户回填、AI 未 self-approve。验证：`python tools/verify_all.py` 的 `spec_gate_check()` 对 `pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed 信号，不绕过。
- [ ] 1.2 验锚：按 `critic.md` 清单第 12 项逐条核对 `design.md`「## 参考项目对照」的 T1 承重锚一律为 `vera20k@305ea3a75:<path>:<line>` 双标、未退化成工作树 HEAD 行号；确认未把 Warsmash AGPLv3 代码 / `double` 路线 / 原创三族 / RA2-YR 设定 / `ConcordRTS` README 自述带进来。验证：T3 / T4 / T1 命中均落在对照小节、锚可打开、无 blocker（`UPSTREAM.md:106,108,109,114`）。
- [ ] 1.3 **规则文档细化（Q-A 类，与 P11 同批、but 按已拍板顺序在规格门批准记录回填后的 apply 阶段由 `docs-owner` 落定；本期 `docs/references/WC3-MELEE-RULES.md` 正文零改动）**：按 `§14` 第 5 条核对是否需同步附录「规则 → 实现覆盖对照」R6 技能独立行（`:405`）与 §15 变更记录（`:437`），需同步则只追加、不新增 S / R 编号、不改既有 `R-x.y` 条文与状态标记、按 §14 第 2 条注明日期与依据、不静默改规则。验证：本期 `git diff -- docs/references/WC3-MELEE-RULES.md` 为空；apply 阶段该文档附录 / §15 两处出现且仅追加。

## 2. 清单逐项 SLK 人工核对与真实清单固化（Q1 / 人工确认项）

- [ ] 2.1 按 D2 / D3 由 `content-owner` 对本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）**人工逐项核对**（仓库文件外证据 / 人工确认项），把真实清单条目与分类固化成编译期常量表，替换 `crates/w3-core/src/ability_system.rs:31,121-177` `SYNTHETIC_ABILITY_CATALOG` 与 `crates/w3-client/src/melee/defs.rs:734` `FOUR_RACE_ABILITY_CATALOG` 的合成占位表 72 条结构口径；**实际条目数核对后如实登记、不臆造精确数字**，proposal / design / `decision-memo.md` MUST NOT 冒充「已逐项核对完成」。验证：`compute_defs_hash`（`gameplay.rs:434`）把新定义纳入 `defs_hash` / `content_hash` 快照头部、与新增权威运行态严格区分（`0002-determinism.md:111,115,117`）；`crates/w3-client/src/melee/defs.rs:1-6` 仍纯编译期零文件 IO（S3 / R6 / R2 / ADR 0002 / ADR 0004）。
- [ ] 2.2 两层张力固化：对齐的是**清单条目与分类**（哪些技能、每族几个、英雄 3 普通 + 1 个 6 级大招、单位 / 道具技能），**不是**照搬经典名称 / 四字符 rawcode / 数值；内部仍用合成原创 `DefId`（沿用 `ABILITY=10` / 英雄 `11–14` 并扩四族）、**不引入四字符 ID → `DefId` 映射层**、数值一律 `[待核]` 不冻结进权威定义 / 不入库、SLK 四字符 ID 只作核对依据且 MUST NOT 进权威状态 / 状态哈希 / 快照。验证：无四字符 ID 常量与映射层、`cargo test -p w3-core --lib` 全绿（S3 / R6 / R7.6 / ADR 0001 / 0002 / 0004 / `WC3-MELEE-RULES.md:124,351`）。
- [ ] 2.3 规模数字如实登记：收尾实测的导出表规模（85 单位 / 344 技能 ID、四族 hero=1 16/24/17/17、`levels` 分布等）一律标注「只读粗统计、以人工核对为准、仓库文件外证据」，不作 WC3 玩法事实来源、不写进权威定义、不静默改规则。验证：`decision-memo.md`「待确认项」如实登记人工确认项边界、无「已逐项核对完成 / 四族全量 = 某精确数字」措辞（`WC3-MELEE-RULES.md §14`）。

## 3. 顺延项全量技能定义（S3 / S5 / S6 / R6 / R8 / R9 / R7.6）

- [ ] 3.1 按 D1 把**中立英雄 / 野怪 / 全部单位技能**纳入技能系统全量实现（承接 P11 顺延项），覆盖 1.0 范围 S3 / S5 / S6 与业务规则 R6 / R8 / R9 / R7.6；引用这些编号作需求依据、MUST NOT 把「引用 `R-x.y`」冒充「规则已实现」（附录三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。验证：`openspec validate p13-ability-full-roster --strict` 通过；`design.md` 各决策带 `[已核]` / `[待核]` / `[取舍]` 核实状态、无「已实现」冒充（`WC3-MELEE-RULES.md:27-30,165-171,184,188-218,393,424`）。
- [ ] 3.2 道具技能按 R7.6 走既有 `EffectDef` 组合结算、MUST NOT 直接执行任意 JASS 技能（N5）；N1–N6（尤其 N2 地图编辑器 / N3 RPG 自定义地图 / N5 JASS·wtg 触发器 VM）明确不做。验证：`Settlement`（`gameplay.rs:851-853`）为唯一结算路径、`crates/` 内无 `war3map.j` / 触发器 VM 引用、`cargo test -p w3-content` 全绿（`WC3-MELEE-RULES.md:43-49,184` / `0003-scope.md:15,38`）。

## 4. 全量清单零资产与合规（ADR 0001 / 0004）

- [ ] 4.1 零资产核对：确认本机导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库；不照搬经典名称 / rawcode / 数值、命名合成原创、数值 `[待核]`。验证：`python tools/verify_all.py` 资产扫描通过、`git ls-files` 无 `*.slk` / 导出 JSON / PNG / GLB / `*.j` / `.jui`（ADR 0001 / `WC3-MELEE-RULES.md:7,351,374`）。
- [ ] 4.2 参考边界核对：`WarsmashModEngine`（AGPLv3）只借语义、不复制代码、不引入 Java / LibGDX 依赖；`ConcordRTS`（GPL-3.0-only）只读、其 RA2/YR 数值 / 规则 MUST NOT 作 WC3 事实来源、README 自述验收不作已验证事实；`rts_kernel_cs` 的 `double` 路线不采用。验证：`design.md`「## 参考项目对照」逐条有「不借鉴 / 许可证理由 / 锚」、`UPSTREAM.md:109,112,114` 红线成立（ADR 0001 / ADR 0002）。

## 5. 确定性契约：四处同步、形态①改坏验红、determinism 重跑（ADR 0002 §6-8 / CODEBUDDY.md §5）

- [ ] 5.1 **若**全量实现新增权威状态：用 `BTreeMap` / `BTreeSet` 稳定序、按稳定 `EntityId` 顺序，四处同步写入 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`，并同步 `WorldGameplayState::hash` / 其 `first_difference` / restore 五重校验；定义层冻结常量经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 快照头部、与新增权威运行态严格区分；表现层只读派生、不反写权威状态（R10.5）。验证：四处字段集合一致、`cargo test -p w3-core --lib` 全绿（R12.4 / R10.5 / ADR 0002 §5 / §6-8）。
- [ ] 5.2 **重跑确定性硬门禁**：`cargo test -p w3-core --test determinism`（任何改动权威状态 / 帧序 / 命令排序 / 哈希编码 / 快照字段 MUST 保持全绿）+ 双实例逐帧哈希一致 + 回放一致 + 快照销毁重建后从 N+1 续算一致；`first_difference()` 返回 `None`。验证：命令输出全绿（`CODEBUDDY.md §5` / R12.4）。
- [ ] 5.3 **反向验证（改坏验红，形态①）**：按 `CODEBUDDY.md §5` 在测试内构造反例 + 元断言防退化——故意漏写某一新增权威运行态到哈希 / 快照 / `first_difference` / restore 之一时断言失败、修复态通过；或保留缺陷态 / 修复态 A-B 对照命令输出。无法构造缺陷态的 MUST 在 `design.md` 登记理由与替代证据。验证：缺陷态测试失败、修复态通过的成对证据可复现，由 `critic.md` 清单第 11 项③复核（`CODEBUDDY.md §5`）。
- [ ] 5.4 若「零新增权威状态」：写明依据（清单常量仅为 `FrozenDefinitions` 冻结定义、经 `compute_defs_hash` 进快照头部，不新增运行态字段），仍重跑 5.2 确认哈希 / 快照口径不变。验证：`decision-memo.md`「待确认项」记录该结论由规格门拍板、`cargo test -p w3-core --test determinism` 全绿。

## 6. 版本处置：若规格门拍板 bump 9→10 则全套（ADR 0002 §7 / ADR 0005 §3，条件式）

- [ ] 6.1 按 ADR 0002 评估新增权威状态是否触发版本 bump，结论由规格门用户拍板、不臆造为已定案（进 `decision-memo.md`「待确认项」）；**若拍板 9→10**：把 `SNAPSHOT_VERSION` / `SIM_VERSION`（`snapshot.rs:67-68`）/`PROTOCOL_VERSION`（`packet.rs:50`）三者同步 9→10、快照头部 / 单测硬断言 / ADR 复核记录同步更新。验证：三版本号一致 == 10、`cargo test -p w3-core --test determinism` 全绿（`0002-determinism.md:117` / `0005-protobuf-payload.md:87` P9 先例）。
- [ ] 6.2 若 bump：旧 v9 明确拒绝、不提供跨版本迁移（`UnsupportedVersion` / `SimVersionMismatch` / `VersionMismatch` + relay `Reject`）；`w3net.proto` 只追加 tag、`reserved 14 to 32` 与 `EffectDef reserved 4 to 8` 保持；canonical 往返校验。验证：旧版本拒绝单测全绿、`cargo test -p w3-net` 与 `cargo test -p w3-relay` 全绿（ADR 0002 §7 / ADR 0005 §3）。
- [ ] 6.3 若 bump：golden hash 按新实现**如实重取、不沿用 v9 值**；全仓 Grep 旧版本号 9 在权威版本号消费者侧无残留、三版本号 == 10 全仓一致；ADR 0002 / 0005 各追加一条「复核 / 修订记录」（不改写正文与历史基线数值）。验证：Windows 侧 `final_hash` 为新重取值、`git diff -- docs/decisions/` 为纯追加、`cargo test -p w3-core --test determinism` 全绿；**不得声称 bump 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118`）。

## 7. 验收线、人工清单边界与编码（D5 / §9）

- [ ] 7.1 全量门禁：`cargo test --workspace`、`cargo fmt --all -- --check`、`python tools/verify_all.py`、`python tools/check_encoding.py`、`openspec validate p13-ability-full-roster --strict`。验证：对应命令输出成功（>500 字 requirement 类 WARNING 如实标注、不冒充无告警；`spec_gate_check()` 对 `pending` 的 fail-closed 属预期、不绕过；CI 双平台 `verify_all.py` 由人工 push 后跑、本会话不预先声称通过）。
- [ ] 7.2 **规则文档同步核对**（玩法 change 收尾必填）：在 `decision-memo.md`「## 规则文档同步」小节回答「本次实现是否需同步 `WC3-MELEE-RULES.md`」——按 §14 第 5 条给出结论，需同步则确认附录「规则 → 实现覆盖对照」R6 技能独立行与 §15 变更记录已按已拍板顺序更新、无需同步则写明依据；**MUST NOT** 把「design 引用 `R-x.y`」冒充「规则已实现」（`:393` 三态判定 / §14 第 5 条 / `critic.md:34` 第 13 项）。验证：小节非空且结论真实（提案阶段为「待 apply 阶段同步」如实结论）、真窗口 8.x 人工清单仍「未执行 / 待人工」、不标通过。
- [ ] 7.3 编码与 git 边界自证：确认所有文件 UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格、正文中文；`git status --short` 改动仅落在本轮成文范围内的三个 change 目录，未改 `openspec/specs/**` / `openspec/changes/archive/**` / `docs/**` / `crates/**` / `tools/**` / 根配置 / `CODEBUDDY.md`；不 `git add` / `commit` / `push` / `fetch` / `pull` / `clone` / `archive`。验证：`python tools/check_encoding.py` 通过、`git status --short` 符合。
- [ ] 7.4 决策纪要结构自证：确认 `decision-memo.md` 三必填小节（承重约束 / 拍板决策 / 待确认项）非空、含「## 已知限制」与「## 规则文档同步」小节、「## 规格门」只写 `status: pending`、无 `fingerprint` / `approved_by` / `date`、无「## 省略说明」。验证：`python -c "import sys;sys.path.insert(0,'tools');import verify_all as v;print(v.decision_memo_check())"` 输出 True。

## 8. 真窗口人工清单（待人工，一律 `- [ ]` 未勾）

- [ ] 8.1 真窗口四族 + 顺延项（中立英雄 / 野怪 / 全部单位技能）技能可施放 / 冷却 / 大招 6 级解锁人工观察（待人工，不用离屏截图 / CPU 回归冒充）。
- [ ] 8.2 真窗口技能特效 / HUD 冷却进度可见性人工观察（待人工；呈现不足按已知限制登记、不扩范围）。
- [ ] 8.3 真窗口回放加载 / 联机对局技能一致性人工观察（待人工）。
- [ ] 8.4 高 DPI / 高刷窗口下技能命令卡交互人工观察（待人工；未执行如实标注，不冒充通过）。

## Workflow follow-up

- 规格门：独立 `reference-researcher` 对照报告已产出、锚已逐条核实、状态文字本批回填；待确认项（是否新增权威状态、是否 bump 9→10、清单实际条目数与人工逐项核对、数值 `[待核]`、验收线）由用户拍板（不拿沉默当授权）；`critic` 规格门报告 + 四件套指纹 + `status: approved` + `approved_by` + `date` 由 leader 回填，本轮 `status: pending`、AI 未 self-approve。
- apply 由 `content-owner`（清单逐项人工核对）/ `sim-owner` / `net-owner` / `client-owner` / `docs-owner` 各自目录串行实施（一个 change 一个 owner、不同时改多 owner 文件），`critic` 代码门复核，本地 commit 描述用中文。
- 人工 push / CI 双平台门禁 / golden hash 双平台逐字节比对 / `/opsx:archive` 由人工与 leader 处理；AI 不得 push / fetch / pull / clone、不得 archive、不得本地 commit（本轮统一由 leader 处理）。
