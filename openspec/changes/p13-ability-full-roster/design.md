# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「怎么实现这两项承接」。事实来源为已归档 `openspec/changes/archive/2026-11-25-p11-ability-system/` 的「## 已知限制」与本仓库 artifacts（可核验锚）：

- **P11 顺延项**：本期范围 = 四族英雄技能全量 + 各族代表性单位 / 道具技能；**中立英雄 / 野怪 / 全部单位技能顺延、标已知限制**（P11 `decision-memo.md`「已知限制」第 1 条、`design.md` D1 /「Risks」、`tasks.md` 2.1、`spec.md` Req1 Scenario）。依据 1.0 范围 S3 / S5 / S6（`WC3-MELEE-RULES.md:27-30`）与 R6（`:165-171`）/ R8（`:188-196`）/ R9（`:200-218`）/ R7.6（`:184`）/ §14（`:418-424`）。
- **Q1 清单水位**：P11 apply 实际落地是「经典结构口径的合成占位表 72 条」——`crates/w3-core/src/ability_system.rs:31`（`ABILITY_CATALOG_LEN = 72`）/ `:121-177`（`SYNTHETIC_ABILITY_CATALOG`，4 席位 ×(4 英雄 × 4 技能 + 1 单位 + 1 道具)，`levels` / `level_req` 硬编码结构值）、`crates/w3-client/src/melee/defs.rs:734`（`FOUR_RACE_ABILITY_CATALOG`，四席位同构）/ `:783`（编译期与内核 72 对齐）/ `:1321-1322`（自述「72 条合成原创 `DefId` 占位」）——**均无任何条目来自本机 SLK 逐项核对**（P11 `decision-memo.md`「已知限制」第 11 条）。
- **收尾实测（仓库文件外证据 / 人工确认项）**：leader 只读实测本机导出 `UnitAbilities.json` heroAbilList 非空 85 单位 / 344 技能 ID、每英雄技能数 {3:1,4:79,5:5}、`reqLevel=6` 大招每英雄 {0:33,1:51,2:1}；`AbilityData.json` 四族 `hero=1` 技能 human 16 / orc 24 / undead 17 / nightelf 17、`levels` {0:18,1:668,2:6,3:93,4:14}、大招 44 条——与「每族 4 英雄 × 4 技能、英雄 3 普通 + 1 大招」整齐结构**系统性不一致**（P11 `decision-memo.md`「已知限制」第 11 条）。导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库（ADR 0001）。用户 2026-11-25 已拍板「接受水位为已知限制、由后续独立 change 承接」。
- **确定性现状**：三版本号权威值 **9**（`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`）；四处同步在 `crates/w3-core/src/sim_match.rs:4328`（`compute_state_hash` 写 `config.content_hash`）/ `:4375`（`defs.content_hash()`）/ `:4418-4419`（`first_difference`）、定义层 `compute_defs_hash`；`CODEBUDDY.md §5` 形态①反向验证、`tests/determinism.rs` 为硬门禁。是否新增权威状态 / 是否 bump 9→10 **未经本轮拍板**（待确认项）。
- **参考矩阵命中**：`docs/references/UPSTREAM.md:108`（T3 数据格式 / SLK）、`:109`（T4 玩法语义 / 技能）、`:106`（T1 确定性内核 / 状态哈希，因涉及可能 bump）命中；可选交叉 `:110`（T5）`/`:112`（T7）；`:107`（T2 移动 / 寻路）、`:111`（T6 网络传输 W3N1 帧头 / relay）**不命中**（本 change 不改移动语义 / 帧头 / 可靠层，版本号与 proto tag 属 ADR 0002 / 0005）。对照小节见本文件「## 参考项目对照」，沿用 P11 归档件已核实结论、缺口如实登记。

## Goals / Non-Goals

**Goals:**
- 承接 P11 已知限制 1（顺延项全量）与 11（Q1 清单逐项 SLK 对齐），把技能清单由「合成占位表 72 条结构口径」替换为「本机导出逐项人工核对后的真实条目与分类」，并保持合成原创 `DefId`、两层张力、零资产合规。
- 玩法 requirement / Scenario / tasks 引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 S1–S6、给可执行验证；引用只作需求依据、不冒充已实现。
- 若新增权威状态，完成 ADR 0002 四处同步 + `WorldGameplayState::hash` + bump 评估 + `determinism` 形态①改坏验红；守表现层只读边界。

**Non-Goals:**
- 不做 N1–N6，尤其 **N2 地图编辑器 / N3 RPG 自定义地图 / N5 JASS·wtg 触发器 VM**（`WC3-MELEE-RULES.md:43-49` / `0003-scope.md:15,38`）；道具技能经既有 `EffectDef` 组合走 R7.6、MUST NOT 直接执行任意 JASS。
- 不照搬经典名称 / 四字符 rawcode / 数值；不引入四字符 ID → `DefId` 映射层；不把本机导出数据 / 数值 `[待核]` 写进四件套或仓库、不进权威状态 / 哈希 / 快照（ADR 0001 / 0002 / 0004）。
- 不冒充「清单已逐项人工核对完成」、不臆造精确条目数；真窗口 8.x / golden hash 双平台 / CI 双平台 / 参考缺口如实登记为待人工 / 待 VM，不写成已验证。
- 不纳入 p12 的三项文档订正（历史 `==8` 字面 / 91→95 列 / tasks 5.2 表述）；本轮只成文、不实施代码，不改 `crates/**` / `docs/**` / `tools/**` / 根配置 / `CODEBUDDY.md`；不 archive / commit / push / self-approve。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（`R-x.y` + 核实状态 `[已核]` / `[待核]` / `[取舍]` + ADR）、代价、对跨平台确定性的影响（ADR 0002）、分层依赖 / 架构原则（SRP / OCP / DIP / CARP）。`[待核]` 数值一律不冻结进权威定义、回到人拍板。**用户本轮只拍板「建两个 change、各自 Scope 划分」，其余范围 / 方案 / 数值 / 验收线一律进 `decision-memo.md`「待确认项」，不臆造为已定案**；以下决策只固定「怎么承接已拍板的两项」，具体实现口径（是否新增权威状态、是否 bump、清单实际条目数）保持待拍板。

### D0 capability 组织：MODIFIED 既有 `ability-system` + ADDED 新 requirements

- **决策**：沿用 P11 已归档主 spec `openspec/specs/ability-system/spec.md`，用 **MODIFIED** 改既有 requirement「四族技能清单按 SLK 逐项对齐并人工固化」的**本期范围口径**（顺延标已知限制 → 顺延项全量；合成占位表 → 逐项 SLK 人工核对后固化），**逐字保留其两个历史 `#### Scenario:` 身份键**（「四族技能清单全量对齐与冻结常量」/「两层张力与零资产合规」）、口径只改在 THEN；另 ADDED 6 条 requirements 覆盖顺延项规则依据 / 人工确认项边界 / 两层张力零资产 / 确定性契约 / 验收线 / 规则文档同步。delta 为既有 capability，**不写 `## Purpose`**（不覆盖主 spec 既有 Purpose，`openspec instructions specs`）、delta 顶部不写说明块、不新增凑数的 MODIFIED delta。
- **备选方案（被否）**：① 新增近重复 capability（如 `ability-full-roster`）—— 被否：`openspec instructions proposal` 要求复用既有 capability 精确路径、不引入近重复名，且会与 `ability-system` 的行为契约打架（`critic` 第 7 / 10 项）；② 新增第 N 份 MODIFIED delta 逐份改主 spec —— 被否：用户明确「不新增凑数 delta」、主 spec 只在 archive 合并时复核 / apply 阶段改；③ 只写 ADDED、不改既有 Req1 的「顺延标已知限制」THEN —— 被否：会令主 spec 旧口径与本 change 全量口径并列打架（`CODEBUDDY.md §8` 四件套一致性）。
- **依据**：P11 `design.md` D0「只写 delta、不写凑数 MODIFIED」先例、`archive/2026-11-24-p10-ai-player-lockstep/specs/deterministic-simulation/spec.md` 完整 MODIFIED 先例；`tools/verify_all.py:175-195` 指纹集合不含主 spec。`[已核]` 组织决策 / R12.4 / ADR 0002。
- **代价**：MODIFIED 须复制整块 requirement（含全部历史 Scenario），文件较大；归档合并时由 `spec-steward` 复核身份键未漂移。
- **对跨平台确定性的影响**：无（纯 spec 组织决策，不触权威状态）。
- **分层依赖 / 架构原则**：SRP（一个 capability 拥有技能系统契约）、避免重复定义、成文口径一次收敛。

### D1 顺延项全量的范围与规则依据

- **决策**：把中立英雄 / 野怪 / 全部单位技能纳入技能系统全量实现，覆盖 1.0 范围 S3 / S5 / S6 与 R6 / R8 / R9 / R7.6（`WC3-MELEE-RULES.md:27-30,165-171,184,188-218`）；玩法 requirement / Scenario / tasks 引用这些编号并给可执行验证，**引用只作需求依据、MUST NOT 冒充「规则已实现」**（附录三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。规则文档同步按 Q-A 类在 apply 阶段由 `docs-owner` 落定、提案期正文零改动（见 D6）。
- **备选方案（被否）**：① 继续把顺延项标已知限制、本期只补清单对齐 —— 被否：与用户拍板「顺延项全量由后续独立 change 承接」的 Scope 冲突；② 把「引用 `R-x.y`」写成「已实现」—— 被否：`WC3-MELEE-RULES.md:393` 三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项；③ 借 `ConcordRTS` 的 RA2/YR 规则推导 WC3 顺延项 —— 被否：`UPSTREAM.md:114` / `WC3-MELEE-RULES.md:3,351` 唯一权威。
- **依据**：S3 `[已核]` 范围 / R6 `[已核]` 机制 `[待核]` 数值 / R8 `[已核]` 机制 `[待核]` 数值 / R9 `[已核]` 机制 / R7.6 `[取舍]` / §14 `[已核]` 流程；ADR 0003（不做 N5）。
- **代价**：顺延项全量排期落到 apply 阶段（各 owner）；具体技能条目 / 分类须人工核对（D3）。
- **对跨平台确定性的影响**：若顺延项新增权威运行态，须按 D4 四处同步；未新增时零影响；定义层冻结常量经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部、与权威运行态严格区分（ADR 0002 §6-8）。
- **分层依赖 / 架构原则**：SRP / OCP（新技能 = 新定义条目）、DIP（表现层只消费冻结定义）。

### D2 Q1 清单逐项 SLK 对齐：人工核对后固化真实条目与分类、保留两层张力

- **决策**：按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）**人工逐项核对后**，把真实清单条目与分类固化成编译期常量表（替换 `SYNTHETIC_ABILITY_CATALOG` / `FOUR_RACE_ABILITY_CATALOG` 的整齐结构占位口径）。**两层张力显式写清**：对齐的是**清单条目与分类**（哪些技能、每族几个、英雄普通技能 + 6 级大招、单位 / 道具技能），**不是**照搬经典名称 / 四字符 rawcode / 数值；内部仍用合成原创 `DefId`（沿用 `ABILITY=10` / 英雄 `11–14` 并扩四族）、**不引入四字符 ID → `DefId` 映射层**、数值一律 `[待核]` 不入库、**不进权威状态 / 状态哈希 / 快照**（`WC3-MELEE-RULES.md:124,351`、ADR 0001 / 0002 / 0004、`crates/w3-client/src/melee/defs.rs:1-6` 纯编译期零文件 IO）。
- **备选方案（被否）**：① 继续用合成占位表 72 条、只补顺延项 —— 被否：用户拍板「Q1 逐项 SLK 对齐由后续独立 change 承接」，且收尾实测证明与导出表系统性不一致；② 严格照搬经典具名清单（名称 / rawcode / 数值复刻）—— 被否：违反 ADR 0001 零资产 / `WC3-MELEE-RULES.md:7,351` / 用户 Q1↔Q2 调和口径「清单对齐 SLK、标识仍合成」；③ 引入四字符 ID → `DefId` 映射层 —— 被否：P11 `design.md` D2 已拍板不引入、无本期收益、增加一层间接。
- **依据**：S3 / R6 `[已核]` 机制 `[待核]` 数值 / R7.6 `[取舍]` / R2 `[已核]` / `WC3-MELEE-RULES.md:124,351` / §14 `[已核]`；ADR 0001 / 0002 / 0004；P11 `design.md` D1 / D2 两层张力。
- **代价**：清单逐项人工核对（仓库文件外证据、人工确认项）与数值 `[待核]` 排期落到 apply 阶段（`content-owner`）；另写全量技能定义表与 CPU 回归。**不产出「照搬经典名称 / rawcode / 数值」的承诺**。
- **对跨平台确定性的影响**：清单定义与分类常量为 `FrozenDefinitions` 冻结常量，经 `compute_defs_hash` 进 `defs_hash` / `content_hash` 快照头部；全部编译期整数常量、零文件 IO、零 `f64`，SLK 四字符 ID / 导出数据不进权威状态 / 哈希 / 快照 / 仓库，不引入平台差异（ADR 0002 §6 / ADR 0004）。
- **分层依赖 / 架构原则**：DIP（表现层不读 SLK、只消费冻结定义）、SRP（定义层与运行态分离）、OCP（新技能 = 新定义条目而非散落分支）、CARP（效果组合而非继承）。

### D3 人工确认项边界：不冒充已完成、不臆造精确数字

- **决策**：清单逐项人工核对是**仓库文件外证据 / 人工确认项**，proposal / design / `decision-memo.md` MUST NOT 冒充「已逐项核对完成」；实际条目数**核对后如实登记、不臆造精确数字**；数值 `[待核]` 不冻结进权威定义、不静默改规则、回到人拍板（`WC3-MELEE-RULES.md §14`）。收尾实测的规模数字（85 单位 / 344 ID、四族 hero=1 16/24/17/17 等）一律标注为「导出表的只读粗统计、以人工核对为准」，不作 WC3 玩法事实来源、不写进权威定义。
- **备选方案（被否）**：① 把收尾实测数字直接写成「已逐项核对完成 / 已实现」—— 被否：用户 2026-11-25 明确「接受水位为已知限制、由后续独立 change 承接」，且本轮未做逐项核对、不臆造；② 把 `[待核]` 数值冻结进 `FrozenDefinitions` —— 被否：`WC3-MELEE-RULES.md:7,351` / ADR 0001 / 0004；③ 把「实际条目数」写死成某精确数字 —— 被否：用户明确「实际条目数核对后如实登记、不臆造精确数字」。
- **依据**：P11 `decision-memo.md`「已知限制」第 11 条（收尾实测、用户拍板接受水位）/「清单规模元数据为粗统计」条；`WC3-MELEE-RULES.md §14` 第 2 / 5 条 `[待核]` / ADR 0001 / 0004。
- **代价**：真实清单条目数在 apply 阶段人工核对后才能登记，本轮保持待确认项。
- **对跨平台确定性的影响**：无（人工确认项，不进哈希 / 快照）。
- **分层依赖 / 架构原则**：显式失败不静默降级、不拿沉默当授权、`CODEBUDDY.md §8` 决策纪要纪律。

### D4 确定性契约：若新增权威状态则四处同步 + bump 评估 + 形态①

- **决策**：**若**全量实现新增权威状态，SHALL 令其用 `BTreeMap` / `BTreeSet` 稳定序、按稳定 `EntityId` 顺序，同步进入 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 四处与 `WorldGameplayState::hash`（及其 `first_difference` / restore 五重校验），并进入 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 9）定义的快照；按 ADR 0002 评估是否 bump `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`，**若规格门拍板 9→10 则全套同步**（全仓消费者 / Grep 旧版本号无残留 / 旧 v9 明确拒绝 / `w3net.proto` 只追加 tag + 保留 `reserved` / golden hash 如实重取 / ADR 0002·0005 复核记录）。`cargo test -p w3-core --test determinism` 必须全绿并含 `CODEBUDDY.md §5` 形态①「改坏验红」反向验证（测试内反例 + 元断言防退化）。表现层只读派生、不反写权威状态（R10.5）。
- **备选方案（被否）**：① 本轮即拍板「一定新增权威状态 / 一定 bump 9→10」—— 被否：用户本轮只拍板了「建两个 change、各自 Scope 划分」，是否新增权威状态 / 是否 bump 属待确认项、不臆造；② 新增权威状态却不进四处同步或跳过形态①—— 被否：ADR 0002 §6-8 / §8 / `CODEBUDDY.md §5` 硬门禁、`critic.md` 清单第 11 项③；③ 沿用旧 golden hash 或声称 bump 后双平台已验证 —— 被否：`0002-determinism.md:118` 口径、CI 双平台由人工 push 后跑、不预先声称通过。
- **依据**：R12.4 `[已核]` / R10.5 `[取舍]`；ADR 0002 §4 / §5 / §6-8 / §7 复核记录（P9 7→8、P11 8→9 全套先例）/ ADR 0005 §3；`crates/w3-core/src/sim_match.rs:4328,4375,4418-4419`、`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`。
- **代价**：若新增权威状态，客户端与 relay 同步发布、无兼容垫片、旧 v9 全部拒绝；golden hash 新值须如实重取、Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对登记为待 VM / 人工。
- **对跨平台确定性的影响**：定宽整数 + 显式小端 + 稳定 ID 顺序不变、新增字段须四处同步；proto 生成代码 Windows / Linux 双平台字节一致（ADR 0005 §3）；**不得声称 bump 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118`）。
- **分层依赖 / 架构原则**：唯一状态所有者（`RtsMatch` 独占）、`w3-core` 不反向依赖上层、表现层只读、显式失败不静默降级、测试即契约。

### D5 验收线与人工清单边界：不冒充通过

- **决策**：CPU 侧 = `cargo test -p w3-core --test determinism` + 形态①反向验证 + `cargo test --workspace` + `cargo fmt --all -- --check` + `python tools/verify_all.py` + `python tools/check_encoding.py` + `openspec validate p13-ability-full-roster --strict`；**真窗口 8.x 人工清单保持 `- [ ]` 未勾、标「待人工」**，不用离屏截图 / CPU 回归冒充；**golden hash 双平台 / CI 双平台 / 参考缺口如实登记为「待 VM / 人工」**，不写成已验证。N1–N6（尤其 N5 JASS VM）明确不做、道具技能走 R7.6。
- **备选方案（被否）**：① 本期即要求真窗口打完一局 1v1 Melee —— 被否：与已拍板验收线冲突、`feedback_gui_fix_selfcheck` 约定 CLI 不可观测项先自测再请人工验收；② 把 CI 双平台 / golden hash 双平台写成已验证 —— 被否：AI 不得 push / fetch / pull / clone、CI 由人工 push 后触发、本会话不预先声称通过（Q8 / P11 `design.md` D8）。
- **依据**：R12.4 `[已核]` / R12.2 `[已核]` / R7.6 `[取舍]` / `WC3-MELEE-RULES.md:43-49`（N1–N6）；`CODEBUDDY.md §5` / §9；`critic.md` 清单第 11 项③ / 第 13 项。
- **代价**：真窗口 / 双平台验收留缺口、如实登记为已知限制，不冒充通过。
- **对跨平台确定性的影响**：CPU 验收在 Windows 侧跑；CI 双平台 `verify_all.py` 由人工 push 后触发，不预先声称通过。
- **分层依赖 / 架构原则**：显式失败不静默降级、测试即契约、收尾阶段优先记录已知限制而非扩范围。

### D6 规则文档同步（Q-A 类）：待 apply 阶段、提案期正文零改动

- **决策**：收尾在 `decision-memo.md`「## 规则文档同步」小节给出结论；按 `WC3-MELEE-RULES.md §14` 第 5 条，玩法 change 收尾需回答是否同步附录「规则 → 实现覆盖对照」（R6 技能独立行，`:405`）+ §15 变更记录（`:437`）。**提案阶段给「待 apply 阶段按已拍板顺序由 `docs-owner` 同步」的如实结论**，本期对活文档**正文零改动**（`git diff -- docs/references/WC3-MELEE-RULES.md` 为空）；`tasks.md` 以一条复选框承接。MUST NOT 把「design 引用 `R-x.y`」冒充「规则已实现」（`:393` 三态判定 / §14 第 5 条 / `critic.md` 清单第 13 项）。
- **备选方案（被否）**：① 提案阶段即改活文档 —— 被否：越权、规格未拍板时改活文档会与四件套打架（用户已明确拍板前置顺序，P11 `design.md` D9 Q-A）；② 把「引用 `R-x.y`」冒充「已实现」—— 被否：三态判定。
- **依据**：`WC3-MELEE-RULES.md §14` 第 2 / 5 条（`:415-421,424`）、附录覆盖对照（`:393-414`）、§15（`:426-437`）；`openspec/config.yaml rules.proposal / rules.tasks`；`critic.md:34` 第 13 项。`[已核]` 流程 / Q-A 类待 apply 阶段拍板顺序。
- **代价**：活文档本期不改，覆盖对照 R6 技能行在 apply 阶段才更新；`rule_doc_sync_check()` 对 `pending` 豁免、结构门禁待 `approved` 后生效。
- **对跨平台确定性的影响**：无（纯文档，不进哈希 / 快照）。
- **分层依赖 / 架构原则**：ADR 记「为什么」、spec 记「系统该做什么」、活文档记「规则基线」，三者不互相改写（`CODEBUDDY.md §8`）。

### D7 参考对照：沿用 P11 已核实结论、独立对照报告已出并回填、缺口回到人拍板

- **决策**：`## 参考项目对照` 小节逐条引用（项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚），命中 T3 / T4 / T1（可选 T5 / T7）。本轮**沿用 P11 归档件 `design.md`「## 参考项目对照」已核实、带锚的与 T3/T4/T1 相关的 5 条结论（G1/G2/G3/G5/G6） + 许可证核对 + 参考缺口**（该归档件是 `reference-researcher` 在 P11 产出的对照报告结论、经 `critic` 复核、已锁定），据实标注「本轮已产出独立 `reference-researcher` 对照报告（T3/T4/T1 命中、G1/G2/G3/G5/G6 五条锚按固定提交逐条打开成立、许可证红线守住、参考缺口如实）、并已在本批回填状态文字」；**参考缺口如实登记、回到用户拍板、不臆造**。T1 承重锚一律 `vera20k@305ea3a75:<path>:<line>` 双标、MUST NOT 退化成工作树 HEAD 行号（`UPSTREAM.md:11,19-25`）。
- **备选方案（被否）**：① 「报告已产出并回填」不作为被否备选 —— 本轮独立 `reference-researcher` 对照报告已产出、锚已逐条核实、状态文字本批回填，用户 2026-11-25 已拍板「批准，p13 先修订再锁定」；按 `critic` 清单第 10 项②「前提变化须整体重估、不沿用旧否决」，原「用户明确本轮不碰」的前提已翻转、该否决不再沿用；② 把 `vera20k` 双标改成工作树 HEAD（`:38` / `:50` / `:74`）行号 —— 被否：P11 `design.md` D10 / 结论 9 已核实 blob 不同、漂移为 HEAD 行号不成立；③ 把参考结论当 WC3 玩法事实来源 —— 被否：`WC3-MELEE-RULES.md` 唯一权威。
- **依据**：`openspec/config.yaml rules.design` / `CODEBUDDY.md §9` 串联；`docs/references/UPSTREAM.md:104-112,114,116-122`；P11 `design.md`「## 参考项目对照」结论 1–10 / 许可证核对结论 / 参考缺口 1–6 / `decision-memo.md` C6 / C7 / C9 / C10。`[已核]`（沿用）/ `[已核]`（独立对照报告已产出并逐锚核实、本批回填）。
- **代价**：正式对照报告与参考缺口的拍板回填留到规格门阶段、如实登记为待确认项；本小节不冒充已完成本轮独立调研。
- **对跨平台确定性的影响**：无（T1 只作确定性内核 / 哈希 / 排序 / 快照方法论对照，不复制代码）。
- **分层依赖 / 架构原则**：只读参考不复制代码（ADR 0001）、不拿沉默当授权、参考缺口回到人拍板。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `UPSTREAM.md` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴了什么 / 明确不借鉴什么（含许可证理由）/ 可核验锚」。本次命中 **T3（`UPSTREAM.md:108`）/ T4（`:109`）/ T1（`:106`）**，可选交叉 T5（`:110`）/ T7（`:112`）；**T2（`:107`）/ T6（`:111`）不命中**（本 change 不改移动 / 寻路语义、不改 W3N1 帧头 / UDP 可靠层 / relay 重传，版本号与 proto tag 属 ADR 0002 / 0005）。**结论沿用 P11 归档件 `archive/2026-11-25-p11-ability-system/design.md`「## 参考项目对照」已核实、带锚的与 T3/T4/T1 相关的 5 条结论（G1/G2/G3/G5/G6）**；本轮已产出独立 `reference-researcher` 对照报告（T3/T4/T1 命中、G1/G2/G3/G5/G6 五条锚按固定提交逐条打开成立、许可证红线守住、参考缺口如实）、并已在本批回填状态文字（用户 2026-11-25 拍板批准，D7 如实登记）。五个参考项目只读、不复制代码（ADR 0001）。

### 结论 G1（沿用 P11 结论 1，T3）清单数据来源是本体 MPQ 数据表、与地图脚本分离

- **参考项目 / 路径**：`godot_warcraft3`（MIT）`docs/data/WC3_ASSET_PATHS.md:86-107,245,274`、`docs/data/PIPELINE.md:118-121,136-149`、`apps/game/config/development_asset_request.source:88-142`、`packages/content/definitions/units/unit_abilities_def.gd:70-71`、`tools/asset-convert/src/development-references.js:17`；`WarsmashModEngine`（AGPLv3）`.../CUnitData.java:164`、`.../StandardObjectData.java:50,217`。
- **借鉴**：定义层形态 = `AbilityData`（数值）+ `UnitAbilities.abilList` / `heroAbilList`（挂载）+ `*AbilityFunc.txt`（order/art）+ `AbilityBuffData`（buff）→ 映射为 `FrozenDefinitions` 内冻结定义（`crates/w3-core/src/gameplay.rs:367-369`）；Q1 逐项清单核对即沿用此数据分层（D2），数值 `[待核]`、以本机导出 SLK 人工核对、不入库。
- **不借鉴（含许可证理由）**：导出的 `assets/slk-exported/**` 内容不入库（ADR 0001）；Godot / GDScript 实现不复制（MIT 允许但本项目不用其宿主）。
- **可核验锚**：矩阵条目 `UPSTREAM.md:108`（T3）；参考项目内路径 `WC3_ASSET_PATHS.md:86-107`（`UnitAbilities.slk` / `AbilityData.slk` / `*AbilityFunc.txt` / `ItemData.slk`）、`:274`（地图脚本在地图包内 `war3map.j`，不在此目录）、`PIPELINE.md:118-121`、P11 C6（矩阵 `UPSTREAM.md:108`）。

### 结论 G2（沿用 P11 结论 2，T4，论证性结论）具名技能 = 本体数据 + 引擎原生行为、Melee 不依赖 WE

- **诚实边界**：这是由「本体表 vs 地图脚本路径分离」+「Data / Logic / Present 分层」+「Melee Bootstrap 原生实现」+「ADR 0003 范围决策」共同推出的**推论**；参考项目没有一句原文直书「Melee 具名技能不是 `war3map.j`」。Warsmash 复刻引擎并存 Java 原生类 / `*.j` / `abilityBehaviors/*.json` 三形态，属其实现选择 ≠ 经典 Melee 依赖地图脚本；本项目 N5 不做 JASS VM，**只借数据分层 / 行为分类 / 效果组合语义**。
- **参考项目 / 路径**：`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:12-13,38-48,63-74,117-126`、`docs/design/game/ROADMAP.md:12,23-24,39,42,74-76`、`docs/roadmap/NEXT.md:35`；`WarsmashModEngine/.../WarsmashConstants.java:60`。
- **借鉴**：Data / Logic / Present 分层、按行为类型分类、效果组合语义。
- **不借鉴（含许可证理由）**：`war3map.j` / vJASS / `*.j` 脚本实现路径（N5）；Warsmash 为 AGPLv3 只借语义、**禁止复制代码**、不引入 Java / LibGDX 依赖。
- **可核验锚**：矩阵条目 `UPSTREAM.md:109`（T4）；参考项目内路径 `WarsmashModEngine/LICENSE:1-3`（AGPLv3）；`WC3-MELEE-RULES.md:43-46`（N2 / N3 / N5）；P11 C7。

### 结论 G3（沿用 P11 结论 3 / 7，T4）许可证与合规边界 / R7.6

- **参考项目 / 路径**：`WarsmashModEngine/LICENSE:1-3`（实开 AGPLv3）、`jassparser/`、`fdfparser/`、`resources/Scripts/melee.jui:1-18`；`godot_warcraft3/docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md:49,67`（道具 abilList 仅白名单 `AIhe/AIma/AIde`）。
- **借鉴**：仅 FDF / UI 格式语义、行为分类语义、道具 abilList 白名单式边界语义（只借结构）。
- **不借鉴（含许可证理由）**：**MUST NOT** 引入 Warsmash Java / LibGDX 代码或 `*.j` / `.jui` / `jassparser/` 代码；AGPLv3 与本项目 GPLv3 组合有条件，引入代码须按 `0001-license.md:17,59,66-73` 复核点重评（本 change 未触发）；`0003-scope.md:15,38`（JASS / wtg VM 不做）。**R7.6**（`WC3-MELEE-RULES.md:182`）：道具 abilList / 效果 MUST 走既有 `EffectDef` 组合、MUST NOT 直接执行任意 JASS。
- **可核验锚**：矩阵条目 `UPSTREAM.md:109,114`；参考项目内路径 `WarsmashModEngine/LICENSE:1-3`；`WC3-MELEE-RULES.md:7,182,351,374,376-385`；P11 C10。

### 结论 G5（沿用 P11 结论 5，T1 双标）新增权威状态触发 ADR 0002 全套

- **参考项目 / 路径**：`vera20k@305ea3a75:src/net/lockstep.rs:1-20,35-39`、`vera20k@305ea3a75:src/net/command_exchange.rs:39,52,80`、`vera20k@305ea3a75:docs/Networking.md`（**T1 承重锚双标**，见下「锚纪律」）。
- **借鉴**：`0002-determinism.md:55-60,62-67,69-72`（哈希 / 快照 / 验收门槛）、`:117`（P9 7→8 全套先例）、`:113`（P8.5c `rng_draws`）、`:44-47`（单 LCG）、`:49-53`（表现层只读）；`0005-protobuf-payload.md:38-43,87`（proto 只追加 tag / 删字段保留 `reserved` / schema 变更同步 bump）。
- **不借鉴（含许可证理由）**：`vera20k` 为 GPLv3 + Section 7 附加条款，**只有真的复制其代码时**才进入（`0001-license.md:30` / `UPSTREAM.md:50`）；本次只借确定性内核 / 哈希 / 排序 / 快照方法论，不复制代码、不引入其 crate 依赖。
- **边界**：三版本号当前 **9**，是否 bump 9→10 属待确认项（D4）；golden hash v9 值不沿用、如 bump 须按新实现如实重取；**不得声称 bump 后双平台 / 全平台确定性已验证**（`0002-determinism.md:118`）。
- **可核验锚**：矩阵条目 `UPSTREAM.md:106`（T1）+ **双标** `vera20k@305ea3a75:src/net/lockstep.rs:1-20,35-39` 等；P11 C9 / `design.md` D10 / 结论 9（`UPSTREAM.md:11,19-25` 固定提交 + 「保留原锚、不据工作树 HEAD 前进改写」先例）。

### 结论 G6（沿用 P11 结论 6，T4 / T5 / T7）catalog 架构与 RA2 红线

- **参考项目 / 路径**：`godot_warcraft3/docs/design/game/ABILITY_SYSTEM.md:7-14,16-60,51-56,117-126,214-219`（`ability_catalog.gd` / `ability_behavior_catalog.gd` 唯一注册表 / `ability_executor.gd`）；`WarsmashModEngine/core/assets/abilityBehaviors/`（15 个 JSON 按种族 / 英雄分组、兽族仅 `orcHeroActives.json`）、`.../MutableObjectData.java:411,423,425,428`（`urac` / `arac` / `frac` / `grac` race 字段）。
- **借鉴（架构原则）**：catalog 单一职责（SRP）、注册新行为 = 扩展非散落分支（OCP）、数值门面委托抽象注册表（DIP）、Present / Logic 分离。
- **不借鉴 / 红线**：`ConcordRTS` 是 C++20 的 RA2/YR 引擎（`UPSTREAM.md:32-36`），SHP / VXL / TMP / MIX / INI 格式、RA2 规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染**绝不作 WC3 事实来源**（`UPSTREAM.md:114` 明令 T3 / T4 / T5 MUST NOT 以 ConcordRTS 为参考；`WC3-MELEE-RULES.md:3,351` 唯一权威）；`ConcordRTS/README.md:25` 自述「逐字节一致」**不得当本项目已验证事实**（`UPSTREAM.md:35`）；`godot_warcraft3/docs/design/game/FACTION_UNIT_ROSTERS.md:1-9` 原创三族只借结构与批次判据；`rts_kernel_cs` 的 `double` 路线不采用（ADR 0002）；Warsmash AGPLv3 只借语义。
- **可核验锚**：矩阵条目 `UPSTREAM.md:109,110,112`；参考项目内路径上列 `file:line`；`UPSTREAM.md:35,114`。

### 许可证核对结论（沿用 P11）

- **AGPLv3 触及**（`WarsmashModEngine`）但只借语义、未复制代码、未引入 Java / LibGDX 依赖；若任何结论需引用其代码须按 `0001-license.md` 复核点重评（**本 change 未触发**）。
- **Section 7**（`vera20k` GPLv3 + 附加条款）只读不复制、不进入（`0001-license.md:30` / `UPSTREAM.md:50`）。
- **资产派生**（`godot_warcraft3` `assets/slk-exported/**`）只引用文档路径与文件名作 schema 证据、不引用导出数值、不入库；MIT 许可允许但本项目按 ADR 0001 零资产。
- **`ConcordRTS`**（GPL-3.0-only）与本项目同许可族、只读不复制、无需重评（`UPSTREAM.md:33`），其 RA2/YR 专属设定不作 WC3 事实来源、README 自述验收不作已验证事实。
- **`rts_kernel_cs`**（MIT）T2 不命中、不借鉴其 `double` 路线。

### 锚纪律（T1 承重锚双标，沿用 P11 D10 / 结论 9）

- `vera20k` 工作树 HEAD（P11 收尾实测 `926a036e`）≠ `UPSTREAM.md:11` 固定提交 `305ea3a75`；实测同文件 blob 不同、工作树 HEAD 侧行号漂移为 `:38` / `:50` / `:74`。**T1 承重锚一律写 `vera20k@305ea3a75:<path>:<line>` 双标**，MUST NOT 退化成工作树 HEAD 行号；按 `UPSTREAM.md:19-25` 先例保留原锚、不据前进改写任何 ADR。锚：`UPSTREAM.md:11,19-25`、P11 `decision-memo.md` C9 / `design.md` 结论 9。

### 参考缺口（如实登记、回到用户拍板、不臆造）

1. **四族全量具名技能无参考项目权威清单**（对应 Q1 / D2）——参考项目无逐项权威表；用户 2026-11-25 已拍板改按**本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData` 清单逐项人工核对**，清单逐项固化由 apply 阶段人工核对、数值 `[待核]`、不静默改规则（`WC3-MELEE-RULES.md §14`）。**本轮不冒充「已逐项核对完成」、不臆造精确条目数**。
2. **具名技能数值 / 快捷键参考项目不作权威**（对应 D2 / D3）——`WC3-MELEE-RULES.md:7,351,374` 一律 `[待核]`、以本机导出 SLK 核对、不入库。
3. **无「Melee 不依赖 JASS」直接原文断言**（结论 G2 为多锚推论）——已标「论证性结论」+ 诚实边界。
4. **独立对照报告已产出并回填**——本轮独立 `reference-researcher` 对照报告已产出并回填；报告结论与现有对照小节实质一致，未新增 / 删改任何 Scope / Requirement / Scenario / 决策 / 验证口径；本小节沿用 P11 归档件已核实结论、参考缺口如实登记、不臆造。
5. **golden hash 双平台比对排期**（对应 D4 / D5）——Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对登记为待 VM / 人工、CI 双平台由人工 push 后跑，不写成已验证。
6. **`vera20k` 工作树 HEAD 前进**（人事项）——按 `UPSTREAM.md:19-25` 先例保留原锚，登记为待 `docs-owner` 独立 change 处理，本 change 不因此阻塞。

## Risks / Trade-offs

- **[Risk] 把 Q1 两层张力判成自相矛盾**：「清单条目与分类逐项对齐经典具名技能」与「合成原创命名、不冒充经典具名技能 / 数值」→ Mitigation：D2 / `spec.md` MODIFIED Req1 与 ADDED Req「全量清单的两层张力」显式区分两个层次——对齐的是清单条目与分类、不是照搬名称 / rawcode / 数值。
- **[Risk] 清单逐项人工核对被冒充「已完成」或臆造精确条目数** → Mitigation：D3 + `spec.md` ADDED Req「人工确认项边界」+ `decision-memo.md`「待确认项」如实登记，收尾实测数字标为粗统计 / 仓库文件外证据。
- **[Risk] 新增权威状态漏进哈希 / 快照 / `first_difference` / restore / `WorldGameplayState::hash` 之一** → Mitigation：D4 + `tasks.md` 形态①反向验证（缺陷态失败 / 修复态通过）+ 四处同步清单 + `critic` 第 11 项③。
- **[Risk] 是否新增权威状态 / 是否 bump 9→10 被臆造为已定案** → Mitigation：D4 条件式表述、「待确认项」明列由规格门拍板、不臆造；无 bump 时写明「零新增权威状态」依据。
- **[Risk] `@305ea3a75` 双标退化成工作树 HEAD 行号** → Mitigation：本文件「锚纪律」+ `critic` 第 12 项验锚。
- **[Risk] 把 Warsmash AGPLv3 代码 / `double` 路线 / 原创三族 / RA2-YR 设定 / README 自述带进来** → Mitigation：结论 G2 / G6 红线 + `UPSTREAM.md:35,114`；WC3 业务规则唯一权威 `WC3-MELEE-RULES.md`。
- **[Risk] 提案阶段误改活文档 / 把「引用 `R-x.y`」冒充「已实现」** → Mitigation：D6 + `tasks.md` 单条复选框 + `decision-memo.md`「## 规则文档同步」诚实边界 + `WC3-MELEE-RULES.md:393` 三态判定。
- **[Risk] 真窗口 8.x / golden hash 双平台 / CI 双平台 / 参考缺口被冒充通过** → Mitigation：D5 + `tasks.md` 人工清单全部 `- [ ]` 未勾、标「待人工 / 待 VM / 人工」+ `decision-memo.md`「已知限制」如实登记。
- **[Risk] 把 p12 的三项文档订正串进本 change** → Mitigation：Scope 严格划分（用户拍板），`proposal.md`「不纳入」+ 本文件 Non-Goals；`git status --short` 两 change 目录互不串味。
- **[Risk] 规格门 `pending` 被绕过或填 `approved` / fingerprint / 省略说明** → Mitigation：`decision-memo.md`「## 规格门」只写 `status: pending`、由 leader 在用户拍板后回填；`spec_gate_check()` fail-closed 属预期。
- **[Trade-off] 按本机导出清单逐项人工核对 vs 继续合成占位表** → 用户 2026-11-25 拍板承接逐项对齐；代价 = 清单逐项人工核对 + 数值 `[待核]` 排期落到 apply 阶段 + 真实条目数本轮保持待确认项。

## Migration Plan

1. **本轮（成文、不实施）**：按 `openspec/decision-memo.template.md` 站稳承重约束与拍板决策后，四件套 + `decision-memo.md` 按 `CODEBUDDY.md §8` 一次成文；`openspec validate p13-ability-full-roster --strict` + `python tools/check_encoding.py`；规格门 `status: pending`，不改 `crates/**` / 活文档 / CI / 权限配置，不 archive / commit / push。
2. **规格门**：`critic` 按清单第 7 / 10 / 12 / 13 项出报告 → **用户拍板**（不拿沉默当授权；待确认项：是否新增权威状态 / 是否 bump 9→10 / 清单实际条目数 / 验收线）→ leader 回填 `status: approved` + 四件套指纹 + `approved_by` + `date` → 规格锁定。`reference-researcher` 独立对照报告由 leader / 用户在此阶段回填、不臆造。
3. **apply 阶段（后续独立会话）**：先按 Q-A 已拍板顺序由 `docs-owner` 落定活文档细化（附录 R6 技能独立行 / §15，不新增 S / R 编号）；再由 `content-owner` 对本机导出 SLK 清单**人工逐项核对**（仓库文件外证据）、`sim-owner` / `net-owner` / `client-owner` 各自目录串行实施全量技能定义与（若拍板）bump 9→10 全套；`critic` 代码门 → 本地 commit（中文描述）→ **人工 push** → CI 双平台 `verify_all.py` → `/opsx:archive`。一个 change 一个 owner，不同时改多 owner 文件。
4. **回滚**：若 bump 9→10，不提供跨版本迁移（ADR 0002 §7）；旧 v9 快照 / 回放 / 包明确拒绝，回滚到 v9 实现须整组 revert + golden hash 按 v9 重取，不在本期承诺。
5. **待 VM / 人工**：golden hash 新值的 Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对、CI 双平台门禁、真窗口 8.x 人工清单一律如实登记为「待」，不冒充通过。

## Open Questions

Q1–Q7 等范围 / 方案 / 数值 / 验收线未决项一律列入 `decision-memo.md`「待确认项」（是否新增权威状态、是否 bump 9→10、清单实际条目数与人工逐项核对、数值 `[待核]` 核实、规则文档同步拍板顺序、真窗口 8.x / golden hash 双平台 / CI 双平台 / 参考缺口与独立对照报告回填、规格门批准记录），由规格门用户拍板，不在 tasks 里烘焙未拍板假设。本 change 无「可以安全延后而不改变 spec / 方案 / task 拆解」的 Open Question。
