# Design

## Context

见 `proposal.md` 的 Why / What Changes；本文件只写「p17 顺延的 Melee 机制全量 + bump 9→10 全套契约 + 文档订正，怎么以确定性方式落进 `w3-core` / `w3-net` 的需求规格与回归锁定」。事实来源为**本仓库 artifacts + 用户 2026-11-27 三项明确拍板 + `reference-researcher` 按五个固定提交 `git show` 实地核实的参考对照报告（G1–G8）**（逐条带可核验锚，不臆造）。仓库外一手明文数据只作机制锚、不入库、不冒充已核验；**不引用、转述、暗示任何仓库外对内资料**，成文后按 `CODEBUDDY.md §6` 合规基线由 leader 复核字面零命中（受限词表不内嵌本仓库文件、以免自造命中）；不写本机绝对路径、不写原版具体数值 / 字段名 / rawcode / 技能名进权威定义。

- **用户 2026-11-27 三项明确拍板（本会话，leader 转达，仓库外证据 / 人工确认项）**：① 范围＝「机制全量 + 文档订正」、不是原版数值真值冻结，所有数值 `[待核]` + 合成占位；② bump 9→10 全套由 p18 独占承接（p13 不再单独 bump，见 p13 `decision-memo.md`「bump 归属补充登记」）；③ 本轮终点＝规格门批准 + 锁定指纹，apply 落 `crates/` 是后续独立会话，本轮不实现、不碰 `crates/`、不实际 bump、不重取 golden hash。
- **p17 顺延清单（承接依据）**：`openspec/changes/p17-melee-mechanism-alignment/decision-memo.md`「已知限制」/「apply 代码门复核与已知限制登记（2026-11-27）」R1–R7、`design.md` D1/D3/D7、`proposal.md`「顺延登记」——三类机制运行时接线、R6.3 买活半血无蓝、R6.1 阈值分档、R5.3 克制矩阵、R7.4 回购、R8.5 三级 leash、L2 反编译补 R3.2 / R3.3 upkeep，以及 R1/R2/R4/R5/R6/R7 六项文档 / 测试订正，全部交 p18。
- **需求源与 `[待核]` 口径**：`docs/references/WC3-MELEE-RULES.md` 的 R5.3（`:157`）/ R6.1（`:167`）/ R6.3（`:169`）/ R6.5（`:172`）/ R7.4（`:182`）/ R8.5（`:194`）/ R1.4（`:98-102`）/ R3.2 · R3.3（`:133/:135`，仅作 L2 人工项登记）；1.0 范围 S1–S6（`:23-30`）、N1–N6（`:42-49`）；附录「核实约定」`:353`、防误用清单 `:374-391`、三态判定 `:395`、覆盖对照 `:393-414`、§14 第 5 条 `:424`、§15 `:426-439`。
- **三版本号 / 四处同步现状（本轮只读核实）**：`crates/w3-core/src/snapshot.rs:67-68`（`SNAPSHOT_VERSION` / `SIM_VERSION` == 9）、`crates/w3-net/src/packet.rs:50`（`PROTOCOL_VERSION` == 9）、`crates/w3-core/src/sim_match.rs:4320` `compute_state_hash` / `:4399` `first_difference` / `:4481` `capture_snapshot` / `:4581` `restore_snapshot`；`crates/w3-net/src/proto/w3net.proto:208`（最大 tag = 63 `GameplayLevelUpHero`）/ `:210`（`reserved 14 to 32`）、`crates/w3-net/src/payload.rs:263` `to_pb_gameplay_kind` / `:553` `from_pb_gameplay_kind`。bump 全套先例见 `0002-determinism.md:109-119` / `0005-protobuf-payload.md:87-88`。
- **p17 代码门订正锚（本轮只读核实）**：`crates/w3-core/src/gameplay.rs:43`（「酒馆按 Q4 不在本期范围」、买活半血无蓝未实现）、`crates/w3-core/tests/verified_mechanisms.rs:683-687`（`assert_eq!(frozen.content_hash(), defs().content_hash())` 恒真断言）；p17 `design.md` D3 `:60`（「合成占位编译期整数常量经既有 `CreepConstants::hash()` 进 `defs_hash` / `content_hash`」）、`spec.md` Requirement 8 `:101-104` 与「全量方案改判时四处同步」Scenario `:83-86`、`tasks.md` 7.4/7.5/7.6 保持未勾。
- **capability 结构事实**：`openspec list --specs` 当前 32 个主 spec、**无 `verified-melee-mechanisms`**（p17 未归档、其 ADDED capability 未合并进主 specs）；`openspec list` 显示 p17 20/23 tasks active。故不能 MODIFIED 该主 spec，也不假设先归档 p17 才能成文 p18（见 D2）。
- **Windows 写文件纪律**：本轮用 Write 工具字节级写入（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格），禁止 `Path.write_text()` / `open(..., "w")` 默认模式（会把 LF 转 CRLF、破坏四件套指纹，`CODEBUDDY.md §2`）。
- **门禁实现**：`tools/verify_all.py::decision_memo_check()` 校验三必填小节、`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed、`rule_doc_sync_check()` 对 pending 玩法 change 豁免、`spec_artifact_fingerprint()` 不含 `decision-memo.md`（`:175-195`）。

**参考矩阵命中判定**：本 change 命中 `docs/references/UPSTREAM.md:104-114` 矩阵 **T1（主）、T4（主）、T6（确定命中）**，T2 边缘、T7 可选交叉（仅 T1 方法论）、T3 / T5 不命中；逐条判定与两处改判见「## 参考项目对照」命中表。`reference-researcher` 已按五个固定提交 `git show` 实地核实并产出对照报告（G1–G8），本轮直接逐条引用、不再重复委派；判定待 `critic` 清单第 12 项复核。

## Goals / Non-Goals

**Goals:**
- 成文完整四件套 + `decision-memo.md`，把「机制全量 + bump 全套 + 文档订正」三组以 SHALL/MUST + 可执行 `#### Scenario:`（WHEN/THEN）定义为可验证行为契约；单一新建 ADDED capability `full-melee-mechanisms`、零 MODIFIED delta。
- 显式声明「bump 9→10 是遵循 ADR 0002 / ADR 0005 门禁、不是放宽红线」，四处同步 / golden hash 重取 / 旧 v9 明确拒绝 / proto 只追加 tag / canonical / 反向验证 / determinism 重跑全套。
- 把「是否新增 `GameplayCommandKind`」「proto 追加 tag 起点」「p17 归档时序与 capability 合并」「L2 upkeep」等参数化承接为规格门待确认项，拍板时不推翻 spec requirement 与 task 结构。
- 把 G1–G8 逐条（项目 / 路径 / 借鉴 / 不借鉴含许可证理由 / 锚）与 R7 锚订正、漂移注记、参考缺口、红线边界落进「## 参考项目对照」。
- 同步 ADR 0003 第四次修订（纯追加）、p13 `decision-memo.md` bump 归属补充登记（纯追加），不改其四件套与「## 规格门」approved / 指纹。

**Non-Goals:**
- **本轮只成文**：不改 `crates/**`（含 `w3-core` / `w3-net` / `w3-client` / `w3-relay`、不实际 bump、不改 proto / payload / golden hash / 结算代码）、不改 `docs/references/WC3-MELEE-RULES.md` 正文、不改主 `openspec/specs/**`、不改 `tools/**` / CI / 权限配置 / p12–p17 / 归档件 / ADR 0001 / 0002 / 0004 / 0005 正文（0003 仅文末纯追加 + 文首状态行日期标注、p13 仅 `decision-memo.md` 末节纯追加）。
- **不纳入（只登记人工确认项）**：L2 反编译补 R3.2 / R3.3 upkeep（用户本人本机执行、AI 不跑 `--deep`、不碰原版客户端、不代判合规）、真窗口 8.x 人工清单、golden hash 双平台逐字节比对、CI 双平台 `verify_all.py` / proto 生成代码字节一致；一律「待人工 / 待 VM」、不标通过、不写成 apply 任务。
- **不冻结真值**：所有 `[待核]` 数值 / 倍率 / 阈值 / 半径 / 比例 / 费用 / 时间 / 距离一律合成占位、不写进权威定义、不冒充经典规则事实。
- 不 archive / 不 commit / 不 push / fetch / pull / clone / 不勾任一复选框 / 不 self-approve / 不写「## 省略说明」 / 不填 `status: approved` / 指纹到任何 decision-memo / 不登记第六个参考项目 / 不引用、转述、暗示任何仓库外对内资料。

## Decisions

> 每个决策统一给出：决策、备选方案（alternatives considered，含被否理由）、依据（ADR 编号 + 可核验锚）、代价、**对跨平台确定性的影响（ADR 0002）**、分层依赖 / 架构原则、玩法决策所依据的 `R-x.y` 与核实状态（`[已核机制]` / `[待核]` / `[取舍]`，与 `openspec/config.yaml rules.design` 的 `[已核]/[待核]/[取舍]` 同源）。`[待核]` 数值一律不冻结进权威定义。

### D0 本轮只成文、不碰 `crates/`、不实际 bump（用户拍板边界）

- **决策**：按用户 2026-11-27 第三项拍板，本轮终点＝规格门批准 + 锁定指纹；只写 `openspec/changes/p18-full-melee-mechanisms/`（四件套 + `decision-memo.md`）、`docs/decisions/0003-scope.md`（纯追加）、p13 `decision-memo.md`（纯追加）。四件套引用版本号一律写「成文期当前 9、apply 阶段 bump 10」，不预先改代码、不重取 golden hash。
- **备选方案（被否）**：① **本轮直接改 `crates/` 把 bump / 机制落地**（被否：任务边界明确本轮只成文、apply 是规格门批准后的后续独立会话、一个 change 一个 owner，`CODEBUDDY.md §9`）；② **本轮顺手重取 golden hash / 跑双平台**（被否：不碰 `crates/` 则无新实现可重取；双平台属人工 / VM 项，不标通过）；③ **把「本轮只成文」写成「已过规格门 / 已锁定指纹」**（被否：`## 规格门` 保持 `status: pending`、AI 未 self-approve）。
- **依据**：用户 2026-11-27 三项拍板（leader 转达，仓库外证据 / 人工确认项）；`CODEBUDDY.md §8 / §9`；`tools/verify_all.py:207-253`。
- **代价**：apply 阶段须据规格门拍板收敛待确认项后再落代码；本轮四件套中「成文期措辞」在 apply 后字面失效，按 p17 体例在 `decision-memo.md`「已知限制」登记、不追溯改四件套。
- **规则编号与核实状态**：ADR 0002 / ADR 0005 契约 `[取舍]`（确定性 / 版本门禁）；不直接承接 `R-x.y` 实现、仅约束其落地方式。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——纯文档 / 规格元数据，`decision-memo.md` 不在四件套指纹集合内，不进状态哈希 / 快照、不改命令排序、不 bump。
- **分层依赖 / 架构原则**：唯一状态所有者、批准由人拍板、不拿沉默当授权、诚实边界（pending 不冒充放行）。

### D1 机制全量运行时接线 + 数值 `[待核]` 合成占位不冻结真值（用户拍板范围）

- **决策**：按用户 2026-11-27 第一项拍板，范围＝「机制全量 + 文档订正」、不是原版数值真值冻结。三类经验机制接线（R6.5 / R6.1）、R6.3 买活半血无蓝、R6.1 阈值分档、R5.3 克制矩阵、R7.4 回购、R8.5 三级 leash 全部落需求规格；所有数值 `[待核]` + 合成占位、不写进权威定义、不冒充经典事实。机制结构借鉴 G3/G4/G5（布尔开关 + 分档表 + 公式族 / 二维矩阵 / 货架清单驱动 / 可打断独立行为），不借真值。
- **备选方案（被否）**：① **冻结参考项目 / 公开攻略的具体数值**（被否：附录核实约定 `:353`、防误用清单 `:374-391`、ADR 0001 / 0004 禁止把 `[待核]` 数值冻结进权威定义；G3/G4 的布尔默认值 / 折减因子 / 各档递减因子 / 矩阵倍率在参考项目里全从数据表读、代码里没有权威真值）；② **读本机导出 SLK / MiscGame 表核对真值**（被否：用户拍板不冻结真值、T3 判定不命中、资产不入库）；③ **用 `f64` 表达折减因子 / 倍率**（被否：ADR 0002 §1 禁浮点进权威状态 / 快照 / 状态哈希）；④ **照抄 `WarsmashModEngine` 的 Java 常量 / 代码**（被否：AGPLv3 只借语义、禁复制代码，`openspec/config.yaml rules.design` 硬要求）。
- **依据**：用户 2026-11-27 拍板（仓库外证据 / 人工确认项）；`WC3-MELEE-RULES.md:157,167,169,172,182,194,353,374-391`；G3/G4/G5（见「## 参考项目对照」）；`crates/w3-core/src/gameplay.rs:43`、`creep.rs` / `gameplay.rs` / `sim_match.rs` 机制骨架（p17 `decision-memo.md` C5/C6）；ADR 0001 / 0002 / 0004。
- **代价**：`[待核]` 合成占位值与原版数值不等价，不冒充经典规则事实（`[取舍]`）；机制全量会改变权威状态字段集合、触发 D3 全套门禁。
- **规则编号与核实状态**：R6.1 `[待核]`、R6.3 `[已核机制 / 待核数值]`、R6.5 `[已核机制 / 待核数值]`、R5.3 `[已核框架 / 待核数值]`、R7.4 `[待核]`、R8.5 `[待核]`；机制结构 `[取舍]`。
- **对跨平台确定性的影响（ADR 0002）**：**字段集合可能变 → 全套门禁**——新增权威状态 MUST 进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION` 定义的快照（成文期 9 / apply 10）、四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）+ bump 9→10 + golden hash 重取 + 反向验证 + `cargo test -p w3-core --test determinism`；`SimFixed` Q47.16 / 30 Hz 整数帧 / 稳定 `EntityId` 顺序 / 命令稳定排序不变；macOS / ARM64 / 其它 rustc / Debian 侧 bump 后逐字节比对仍待 VM / 人工、不声称达成。
- **分层依赖 / 架构原则**：冻结输入边界（定义层常量 ≠ 权威运行态）、DIP（机制由数据驱动、不写死）、SRP（每机制独立结算环节）、诚实边界（合成占位不冒充原版）。

### D2 capability 结构：单一新建 ADDED `full-melee-mechanisms` + 零 MODIFIED delta（结构风险处置）

- **决策**：新建单一独立 capability `full-melee-mechanisms`（ADDED Requirements），零 MODIFIED delta、主 `openspec/specs/**` 零改动；用独立名避免与 p17 ADDED `verified-melee-mechanisms` 重名冲突，在 `decision-memo.md` 登记与 p17 的衔接（p17 顺延清单）与未来归档关系（p17 归档后其 capability 才进主 specs；两者是否合并 / 去重由规格门拍板）。
- **备选方案（被否）**：① **MODIFIED `verified-melee-mechanisms` 主 spec**（被否：`openspec list --specs` 无此项、p17 未归档，MODIFIED 须逐字保留不存在的历史 `#### Scenario:` 身份键，结构歧义；brief 第八节明确不假设可 MODIFIED）；② **MODIFIED 多个既有主 spec**（`creep-camps` / `hero-revive` / `gameplay-systems` / `ability-system` 等）（被否：p18 覆盖内容多为新增契约 bump 全套 / proto / canonical / 文档订正，散布 MODIFIED 需逐字保留大量历史 Scenario 身份键、口径只改在 THEN，跨多 owner 文件且一致性风险高）；③ **要求先归档 p17 再成文 p18**（被否：不擅自 archive p17、不臆造前置依赖；如实登记为待确认项回报 leader）；④ **新建多个细粒度 capability**（被否：一个 change 一个 owner、四件套一致性收敛成本，p17 单一 capability 先例）。
- **依据**：`openspec status --change p18-full-melee-mechanisms --json` / `openspec instructions specs --change ... --json`（capability 新建规则、`## Purpose` 与 ADDED 形态）；`openspec list --specs`（无 `verified-melee-mechanisms`）；`openspec list`（p17 20/23 active）；`CODEBUDDY.md §8` 四件套一致性；brief 第八节。
- **代价**：与 p17 的 capability 未来可能存在语义重叠，需规格门拍板合并 / 去重时机；本轮主 specs 零改动、不冒充已对齐（p17 `decision-memo.md` 登记的 `creep-camps/spec.md:36` 历史措辞不一致仍留待收口）。
- **规则编号与核实状态**：`openspec` 结构 `[取舍]`；不直接承接 `R-x.y` 实现。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——spec delta 是行为契约文档，不改权威状态 / 哈希 / 快照 / 命令排序 / 版本号；真正影响由 D1/D3 的 apply 阶段承接。
- **分层依赖 / 架构原则**：ADR 记「为什么」与 spec 记「该做什么」不打架、参数与策略分离（OCP）、诚实边界（结构歧义如实登记不臆造）。

### D3 bump 9→10 是「遵循」ADR 0002 / ADR 0005 门禁、不是放宽红线（用户拍板）

- **决策**：按用户 2026-11-27 第二项拍板，bump 9→10 全套由 p18 独占承接（p13 不再单独 bump）：`SNAPSHOT_VERSION` / `SIM_VERSION`（`snapshot.rs:67-68`）9→10、`PROTOCOL_VERSION`（`packet.rs:50`）9→10；新增权威状态字段四处同步（`sim_match.rs:4320/4399/4481/4581`）进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION` 定义的 v10 快照；golden hash 按新实现如实重取、不沿用 v9 值；旧 v9 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移。**关键措辞**：这是**遵循** ADR 0002（四处同步 / golden hash 重取 / 反向验证）与 ADR 0005（proto 只追加 tag / relay 版本拒绝 / canonical）门禁，不是放宽任何红线；ADR 0001 零资产 / 不复制代码 / 不绕 EULA · DRM · 登录 · 反作弊、ADR 0002 确定性门禁、1.0 范围 S1–S6、「明确不做」清单、五个只读参考项目、长期目标像素级复刻口径全部不变。
- **备选方案（被否）**：① **不 bump、沿用 9 硬塞新字段**（被否：ADR 0002 §7 快照版本契约、旧 v9 快照 / 回放会被错误解释、状态哈希口径不一致）；② **只 bump `SNAPSHOT_VERSION` / `SIM_VERSION`、不动 `PROTOCOL_VERSION`**（被否：若新增玩法命令变体则线格式变化、ADR 0005 §3「proto schema 变更必须同步 bump `PROTOCOL_VERSION`」；即使不新增命令，用户已把三版本号同步定为 p18 确定范围）；③ **只 bump 一处、四处同步省略**（被否：ADR 0002 §6-7 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 必须同步，`0002-determinism.md:109-119` 历期 bump 全套先例）；④ **为旧 v9 提供兼容垫片 / 静默降级**（被否：显式失败不静默降级、旧端把新字段当 unknown field 静默丢弃会导致不可见 desync）；⑤ **把 bump 解读为放宽红线 / 允许证据进门禁**（被否：用户拍板明确「遵循门禁不是放宽」、ADR 0003 第四次修订红线不变、D8）。
- **依据**：用户 2026-11-27 拍板（仓库外证据 / 人工确认项）；`ADR 0002 §6-8` / `0002-determinism.md:109-119`（P8.5a–d / P9 / P11 历期 bump 全套先例、旧版本明确拒绝、golden hash 重取）；`ADR 0005 §3` / `0005-protobuf-payload.md:87-88`；`crates/w3-core/src/snapshot.rs:67-68`、`crates/w3-net/src/packet.rs:50`、`crates/w3-core/src/sim_match.rs:4320,4399,4481,4581`；`CODEBUDDY.md §5`。
- **代价**：旧 v9 快照 / 回放全部失效（明确拒绝、不迁移）；golden hash 新基线仅 Windows 侧重取，Debian / macOS / ARM64 / 其它 rustc 侧逐字节比对仍待 VM / 人工；全仓旧版本号 Grep 无残留须 apply 阶段自证。
- **规则编号与核实状态**：ADR 0002 / ADR 0005 契约 `[取舍]`（确定性 / 版本门禁）；不直接承接 `R-x.y` 实现、仅约束其落地方式。
- **对跨平台确定性的影响（ADR 0002）**：**正向锁定但待验证**——四处同步把新增字段显式纳入哈希 / 快照，防未来退化；`SimFixed` Q47.16 / 30 Hz / `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序 / FNV-1a 编码 / 单 LCG 不变；bump 后 golden hash 逐字节双平台比对、CI 双平台 `verify_all.py` 一律「待 VM / 人工」、不声称全平台 / 双平台确定性已达成（沿用 `0002-determinism.md:118` 口径）。
- **分层依赖 / 架构原则**：唯一状态所有者、显式失败、冻结输入边界、ADR 0005 canonical 约束、不拿沉默当授权。

### D4 proto 只追加 tag / payload 穷尽映射 / relay 版本拒绝 / canonical（ADR 0005 §3），新增 `GameplayCommandKind` 属待确认项

- **决策**：`w3net.proto` 的 `GameplayCommand.oneof` 只追加 tag（当前最大 63 `GameplayLevelUpHero` `:208`、新增从 64 起、`reserved 14 to 32` `:210` 保持、既有 tag 一字不动、每个新 message 字段只追加且尾部保留 `reserved`）；`payload.rs:263/:553` 双向穷尽映射一一对应；`Packet::decode` 在解码 payload 前按版本拒绝、relay 回 `Reject`；ADR 0005 §3 约束继续全部适用（禁 `map`、禁 `float` / `double`、字段只追加 / 删除保留 `reserved`、unknown fields 不透传 / 回显、锁定 `prost` / `prost-build` 版本写入 `Cargo.lock`、禁非规范 varint、canonical「解码→再编码」往返、CI 双平台生成代码字节一致）；§4 红线不变（只传输入命令、不传游戏状态）。**「是否新增 `GameplayCommandKind`」「proto 追加 tag 起点」为规格门待确认项，不臆造**；若新增则连带 proto tag / relay / canonical 全套，不旁路门禁（T6 确定命中与是否新增命令无关，见命中表）。
- **备选方案（被否）**：① **复用 / 改名既有 tag 或插进 `reserved 14 to 32`**（被否：ADR 0005 §3「字段编号只追加、不复用、不改名；删除保留 `reserved`」、旧 tag 被重新解释会破坏字节确定性）；② **旧端把新字段当 unknown field 静默丢弃**（被否：两端命令流不等 → 不可见 desync、违反显式失败不静默降级，`0005-protobuf-payload.md:84-88` 历期拒绝先例）；③ **用 `map` / `float` / `double` 表达矩阵 / 倍率**（被否：迭代顺序不确定 / 浮点进权威边界，ADR 0002）；④ **照搬 `vera20k` 的线格式**（被否：以本项目 `W3N1` 25B 帧头 + ADR 0005 为准，G8）；⑤ **本轮臆断「一定新增 / 一定不新增 `GameplayCommandKind`」**（被否：brief 明确为规格门待确认项、不臆造；两种结论都须在 spec 行为式 SHALL / tasks 条件式措辞下不推翻结构）。
- **依据**：`ADR 0005 §3 / §4`、`0005-protobuf-payload.md:87-88`（P9 tag 46–58 / P11 tag 59–63 只追加先例、版本拒绝、canonical）；`crates/w3-net/src/proto/w3net.proto:208,210`、`crates/w3-net/src/payload.rs:263,553`；G8（`vera20k/services/relay-protocol/src/envelope.rs:27-28` 解码前按版本拒绝、`netsim/compaction.rs:126-127,138` canonical `ensure(*known == entry.canonical)`）；`docs/references/UPSTREAM.md:111`（T6）。
- **代价**：新增命令与否的拍板会改变 apply 阶段 proto / relay / canonical 工作量；CI 双平台生成代码字节一致待人工 push 后验证、不冒充已验证。
- **规则编号与核实状态**：ADR 0005 / ADR 0002 契约 `[取舍]`；R12.2（`:246` 只传输入命令）作边界依据。
- **对跨平台确定性的影响（ADR 0002）**：**零状态影响 + 线格式确定性锁定**——`W3N1` 25B 帧头 / `FrameScheduler` / `stabilize` 稳定排序 / `apply_frame` / 状态哈希 / 快照 / 断线重连游标不变；canonical 往返防止不同版本客户端产生不同字节；`w3-core` 不反向依赖 `w3-net`；跨平台确定性待验证项不因本决策变化。
- **分层依赖 / 架构原则**：显式失败、冻结线格式边界、ADR 0005 canonical、参数与策略分离（OCP，待确认项二选一不推翻四件套结构）。

### D5 反向验证（改坏验红）+ determinism 重跑 + 补齐 p17 R2 生产结算 A-B（CODEBUDDY.md §5）

- **决策**：新增 CPU 确定性回归断言（三类机制接线 / 买活半血无蓝 / 阈值分档 / 克制矩阵 / 回购 / leash / v10 快照往返 / 旧 v9 拒绝）；MUST 重跑 `cargo test -p w3-core --test determinism` 全绿；MUST 按 `CODEBUDDY.md §5` 形态①（测试内反例 + 元断言防退化）或②（A-B 对照 / 先写失败测试 / 刻意改坏被测代码验红并留命令输出）证明缺陷态失败、修复后通过。另**补齐 p17 R2 缺口**：p17 经验分摊侧 formal 是测试内复刻 `formal_settlement`、与生产结算零调用关系，缺一次「改生产 `xp_candidates` → 正向回归变红 → 恢复」的 §5 形态② A-B、且 `sim-owner` 转述的 `exit 101 / 15 passed` 输出未在任何 artifact 留档；p18 apply 阶段 MUST 补该 A-B 并留命令输出。
- **备选方案（被否）**：① **仅「全绿」当验证**（被否：`CODEBUDDY.md §5` 明确仅全绿不构成验证）；② **把反向验证顺延 / 只在 design 写理由**（被否：机制断言 / 四处同步缺陷态可构造、不满足「无法构造」豁免条件）；③ **把证据台账 / 离屏对照接进 determinism / CI 门禁**（被否：ADR 0002 证据不进门禁、视觉 oracle 非业务逻辑 oracle）；④ **不重跑 determinism**（被否：`CODEBUDDY.md §5` 任何改动 `crates/w3-core/` 实现 MUST 通过）。
- **依据**：`CODEBUDDY.md §5`；`0002-determinism.md:69-72,109-119`（§8 验收门槛 / 历期反向验证）；`crates/w3-core/tests/determinism.rs` / `tests/reverse_validation_*.rs`（既有形态①先例）；p17 `decision-memo.md`「apply 代码门复核」R2 / R4；`.codebuddy/agents/critic.md:29`（清单第 11 项③）。
- **代价**：新增回归须覆盖多类缺陷态，apply 阶段工作量落在 `sim-owner`；p17 R4 恒真断言（`verified_mechanisms.rs:683-687`）改固定基线比较或删除后，证据力改由 `golden_hash` example + 参数分离 `assert_ne!` 承担。
- **规则编号与核实状态**：R6.5 / R6.3 `[已核机制]`（回归锁定的是机制结构、不是 `[待核]` 数值）；ADR 0002 契约 `[取舍]`。
- **对跨平台确定性的影响（ADR 0002）**：**正向锁定**——回归把 30 Hz 整数帧 / 稳定 `EntityId` 顺序 / 逐帧 `compute_state_hash()` 一致性 / 四处同步显式断言，防止未来退化；不冒充全平台确定性达成（双平台待 VM / 人工）。
- **分层依赖 / 架构原则**：显式失败、测试本身也要审（`critic` 清单第 11 项③）、诚实边界。

### D6 文档订正 R1/R2/R4/R5/R6/R7、R7 锚实地核验与规则文档同步待办（§14 第 5 条）

- **决策**：落实 p17 代码门交办的六项订正——R1 订正 p17 `design.md` D3「合成占位常量经 `CreepConstants::hash()` 进 `defs_hash` / `content_hash`」与实现不符（apply 未编入）的措辞；R2（见 D5 生产结算 A-B）；R4 把恒真断言改固定基线比较或删除；R5 成文期快照锚失效、R6 成文期 `#### Scenario:` 字面失效与 7.4/7.5/7.6 tasks 保持未勾的如实登记；R7 参考项目源码锚未实地打开的缺口（本轮已由 `reference-researcher` 补齐、结论见「## 参考项目对照」）。玩法 change 收尾按 `WC3-MELEE-RULES.md §14` 第 5 条在 `decision-memo.md`「## 规则文档同步」给结论、并在 apply / 归档前由 `docs-owner` 更新附录「规则 → 实现覆盖对照」对应行与 §15；本轮 `status: pending` / 纯成文、只写「需同步、待 apply 阶段按 Q-A 已拍板顺序同步」的如实结论、保持非空、不改规则文档正文、不把「引用 `R-x.y`」冒充「规则已实现 / 已人工验收」（三态判定 `:395`）。
- **备选方案（被否）**：① **本轮就改 `WC3-MELEE-RULES.md` 正文 / 覆盖对照**（被否：用户明确本轮不改正文，§14 第 2 条不静默改规则）；② **追溯改写 p17 四件套订正 R1/R5/R6**（被否：改 p17 四件套会改变其已锁定指纹 `ea47ef80…`、触发 `CODEBUDDY.md §8` 规格解锁须重走规格门；按 p17 体例只在纪要登记、由 p18 承接）；③ **省略规格门或写「## 省略说明」求门禁变绿**（被否：本 change 非小改动豁免场景）；④ **把「引用 `R-x.y`」写成「规则已实现」**（被否：`:395` 三态判定 / §14 第 5 条 / `critic.md:34` 清单第 13 项）。
- **依据**：p17 `decision-memo.md`「apply 代码门复核与已知限制登记」R1–R7 / `design.md` D3 `:60` / `spec.md:83-86,101-104` / `tasks.md` 7.4-7.6；`WC3-MELEE-RULES.md:393-414,424,426-439`；`tools/verify_all.py:256-294`；`openspec/decision-memo.template.md:48-53`；`.codebuddy/agents/critic.md:34`。
- **代价**：`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed、如实登记、不绕过；完整 `python tools/verify_all.py` 由 leader 跑、本轮不跑；`rule_doc_sync_check()` 对 pending 玩法 change 豁免。
- **规则编号与核实状态**：§14 / §15 / 附录核实约定属活文档维护流程 `[取舍]`；R6.5 已由 commit 拍板 `[已核机制 / 待核数值]`、本 change 不重复改规则。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——纪要 / 规格门 / 同步结论是纯文档 / 门禁元数据，`decision-memo.md` 不在四件套指纹集合内（`tools/verify_all.py:175-195`），不进状态哈希 / 快照、不改命令排序、不 bump。
- **分层依赖 / 架构原则**：参数与策略分离（OCP）、不拿沉默当授权、批准由人拍板、诚实边界（pending 不冒充放行 / 引用不冒充已实现）。

### D7 R1 可移动性边界不因机制全量落地退化（R1.4）

- **决策**：金矿 / 树木 / 建筑 / 资源点等不可移动实体 MUST NOT 响应 `Move`、MUST NOT 出现在 `Move` 命令的 Scenario 中作为移动主体、MUST NOT 改变位置；框选含建筑时 MUST 只为可移动单位生成 `Move`、落点按稳定 `EntityId` 顺序（`WC3-MELEE-RULES.md:98-102`）。机制全量 / leash / 回购落地 MUST NOT 借新命令旁路改写该过滤；新增回归含「改坏验红」反例（删过滤则断言失败）。
- **备选方案（被否）**：① **本 change 不写 R1 边界**（被否：`critic` 清单第 13 项 / `openspec/config.yaml rules.specs` 特别校验 R1 可移动性、玩法 change 无该约束按 blocker 处理）；② **在 leash / 回购 Scenario 里让建筑 / 资源点作 `Move` 主体**（被否：直接违反 R1.4、令历史 bug 从新机制路径复发）；③ **用建筑名字符串白名单判可移动性**（被否：R1.2 / `p8-faction-foundation` D1 明确 MUST 由定义层可移动性数据驱动）。
- **依据**：`WC3-MELEE-RULES.md:63-102`（R1 是 bug 根因章节）/ `:101`（框选含建筑时只给可移动单位生成 `Move`）；`openspec/config.yaml rules.specs`；`.codebuddy/agents/critic.md:32`（清单第 13 项）。
- **代价**：须在 spec 显式写一条 R1 边界 requirement + 反例回归，略增四件套篇幅。
- **规则编号与核实状态**：R1.4 `[已核]` + `[取舍]`（分层判据是本项目取舍、非原版事实）。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——可移动性过滤是表现层输入翻译层的只读派生、不进权威状态 / 哈希 / 快照；回归只断言命令生成结果、不改排序键 / 帧序。
- **分层依赖 / 架构原则**：表现层只读派生、冻结输入边界、显式失败（反例必红）。

### D8 数值 / 证据 / 资产 / 涉密红线（ADR 0001 / 0002 / 0004）

- **决策**：所有 `[待核]` 数值保持合成占位、不冻结进权威定义；仓库外一手明文数据 / 证据台账 / 反编译产物 MUST NOT 进仓库、权威状态、FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照、`first_difference()`、golden hash、determinism 验收、CI 门禁。禁令二分（沿用 p17 `design.md:8`）：**禁止入库** —— 本机一手明文取证字段名 / 四字符 rawcode / 技能名 / 具体数值；**允许引用** —— 只读参考项目源码佐证锚与防误用清单登记值（仅可在 `design.md`「## 参考项目对照」引用、不作 WC3 数值事实源、不冻结进权威定义、不随发行物分发）。四件套 / ADR 0003 第四次修订 / p13 追加 / `decision-memo.md` MUST NOT 引用、转述、暗示任何仓库外对内资料或其对出处 / 原文 / 细节的转述；成文后按 `CODEBUDDY.md §6` 合规基线由 leader 复核字面零命中（受限词表不内嵌本仓库文件、以免自造命中）。零资产、零新增依赖、`Cargo.lock` 不变、不登记 `NOTICE.md`。
- **备选方案（被否）**：① **把证据台账 / 明文一手数据写进仓库当 oracle**（被否：ADR 0001 零资产 + ADR 0002 证据不进门禁，且台账为仓库外人工确认项）；② **把 `[待核]` 数值冻结进 `FrozenDefinitions`**（被否：`:353` / `:374-391` / ADR 0004）；③ **对无锚缺口臆造数值**（被否：`CODEBUDDY.md §8` 无证据项 MUST NOT 写成「用户已确认」、参考缺口如实登记回到人拍板）；④ **为凑完整度把仓库外对内资料转述进文档**（被否：ADR 0001 零资产 + `CODEBUDDY.md §6` 合规基线，此类资料一律留会话与仓库外本机、不入库、不分发、不冒充公开来源）。
- **依据**：`WC3-MELEE-RULES.md:7,353,374-391`；`0001-license.md:80-82` / `0002-determinism.md:55-72` / `0003-scope.md:125-138`（第三次修订红线）；`CODEBUDDY.md §5 / §6`；`tools/check_encoding.py`；本轮按 `CODEBUDDY.md §6` 合规基线由 leader 复核字面零命中、受限词表不内嵌本仓库文件（复核结论见交付回报）；p17 `design.md:8` 禁令二分。
- **代价**：合成占位不冒充原版、参考缺口七类保持 `[待核]`、不承诺真值已对齐；文档只作机制锚、不作数值事实源。
- **规则编号与核实状态**：ADR 0001 / 0002 / 0004 契约 `[取舍]`；R-x.y 数值侧全部 `[待核]`。
- **对跨平台确定性的影响（ADR 0002）**：**零影响且为硬边界**——证据 / `[待核]` 数值不进权威状态 / 哈希 / 快照 / golden hash / CI；`SimFixed` Q47.16 / 30 Hz / 稳定排序 / 单 LCG 不变。
- **分层依赖 / 架构原则**：冻结输入边界、显式失败、诚实边界、零资产 / 零涉密红线。

### D9 参考项目对照结论逐条落、缺口回到用户拍板（T1–T7 / critic 清单第 12 项）

- **决策**：把 `reference-researcher` 按五个固定提交 `git show` 实地核实的 G1–G8 逐条（项目 / 路径 / 借鉴 / 不借鉴含许可证理由 / 锚）落进「## 参考项目对照」，附命中表（含 T6「条件→确定」上调、T3「边缘→不命中」下调两处改判）、R7 锚实地核验结论与 G4 路径前缀订正、漂移注记、参考缺口七类、红线边界。参考缺口一律 `[待核]` + 合成占位、回到用户拍板、不臆造（`UPSTREAM.md:122`）。
- **备选方案（被否）**：① **从参考项目取真值补缺口 / 复制 AGPLv3 代码**（被否：ADR 0001、G7「只借语义、禁复制代码」、`openspec/config.yaml rules.design` 硬要求）；② **T4 玩法语义以 `ConcordRTS` 为参考**（被否：`UPSTREAM.md:114` T3/T4/T5 MUST NOT 以其为参考、RA2/YR 不作 WC3 事实源）；③ **把 `vera20k` / `ConcordRTS` 工作区 HEAD 行号当固定锚**（被否：`UPSTREAM.md:19-25` 先例「保留原锚、不据前进改写」，MUST 用 `git -C <根> show <固定提交>:<文件>` 复核）；④ **省略「## 参考项目对照」**（被否：T1–T7 命中 change 无该小节按 `critic` 清单第 12 项 blocker 处理）。
- **依据**：`docs/references/UPSTREAM.md:9-15,19-25,104-114,118-122`；`openspec/config.yaml rules.design`；`CODEBUDDY.md §9`（凡 T1–T7 命中 MUST 有对照小节）；`.codebuddy/agents/critic.md:31`（清单第 12 项）；`reference-researcher` 对照报告（G1–G8，仓库外只读证据 / 人工确认项）。
- **代价**：G1/G2 锚须标「按固定提交 `git show` 核实、工作区行号可能漂移、非逐行全量核验」；本轮结论不需参考项目代码、未触发 `0001-license.md:66-73` 复核点。
- **规则编号与核实状态**：`R-x.y` 机制结构 `[已核机制]`、数值 `[待核]`、参考缺口 `[待核]`；许可证边界 `[取舍]`。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——对照结论是只读方法论 / 语义锚，不进权威状态 / 哈希 / 快照 / CI；`ConcordRTS` README 自述「逐字节一致 / 验收完成 N 项」不作本项目已验证事实、不据其声称跨平台确定性已达成。
- **分层依赖 / 架构原则**：诚实边界、不拿沉默当授权、只读参考不复制代码。

### D10 L2 upkeep / 真窗口 / 双平台只登记人工确认项、规格门 pending（ADR 0001 / 0003 / CODEBUDDY.md §4）

- **决策**：L2 反编译补 R3.2 / R3.3 upkeep 只登记为「人工确认项 / 待用户拍板」，由用户本人本机执行，AI 不跑 `--deep`、不碰原版客户端、不代判合规，**MUST NOT 写成 `sim-owner` 的 apply 任务**（`0003-scope.md` 第三次修订复核点 / ADR 0001）。真窗口 8.x 人工清单、golden hash v10 双平台逐字节比对（macOS / ARM64 / 其它 rustc / Debian）、CI 双平台 `verify_all.py` / proto 生成代码字节一致一律「待人工 / 待 VM」、不标通过。「## 规格门」置于末尾且只有本节以「规格门」开头，严格只写 `- status: pending` + 说明、不填 `fingerprint` / `approved_by` / `date` / `reason`、不写「## 省略说明」、不 self-approve。
- **备选方案（被否）**：① **把 L2 upkeep 写成 apply 任务**（被否：任务硬边界「只登记人工确认项」、AI 不跑 `--deep` / 不代判合规）；② **自填 `status: approved` + 指纹 / 写省略说明求门禁变绿**（被否：`CODEBUDDY.md §8 / §9` 由人拍板、AI 不得 self-approve、本 change 非小改动豁免场景）；③ **把真窗口 / 双平台标通过**（被否：CPU 回归 / determinism 已过不等于人工验收通过，沿用「收尾阶段优先记录已知限制」口径）。
- **依据**：用户 2026-11-27 三项拍板（仓库外证据 / 人工确认项）；`0003-scope.md:125-138`（第三次修订复核点）/ `0001-license.md:80-82`；`CODEBUDDY.md §4 / §8 / §9`；`tools/verify_all.py:143-253`；`openspec/decision-memo.template.md:14,55-61`。
- **代价**：`spec_gate_check()` 对 `status: pending` 报 `invalid spec gate status 'pending'` 属预期 fail-closed、如实登记、不绕过；完整 `verify_all.py` 由 leader 跑、本轮不跑。
- **规则编号与核实状态**：R3.2 / R3.3（`:133/:135`）`[待核]`、仅作人工项登记；ADR 0001 / 0003 契约 `[取舍]`。
- **对跨平台确定性的影响（ADR 0002）**：**零影响**——人工项 / 规格门元数据是纯文档，不进权威状态 / 哈希 / 快照、不 bump；不声称全平台确定性已达成。
- **分层依赖 / 架构原则**：批准由人拍板、不拿沉默当授权、诚实边界（pending 不冒充放行 / 人工清单不标通过）。

## 参考项目对照

> 按 `openspec/config.yaml rules.design` 与 `CODEBUDDY.md §9`：凡命中 `docs/references/UPSTREAM.md:104-114` 矩阵 T1–T7 的 change MUST 有本小节，逐条列「项目 / 路径 / 借鉴 / 不借鉴（含许可证理由）/ 可核验锚」。本 change 命中 **T1（主）、T4（主）、T6（确定命中）**，T2 边缘、T7 可选交叉（仅 T1 方法论）、T3 / T5 不命中。`reference-researcher` 已按五个固定提交 `git show` 实地核实对照报告（G1–G8，仓库外只读证据 / 人工确认项），本轮直接逐条引用、不再重复委派；判定待 `critic` 清单第 12 项复核。锚一律按固定提交 `git -C <项目根> show <固定提交>:<文件>` 复核，路径相对 `UPSTREAM.md:9-15` 各自项目根。

| 报告条目 | 项目 / 路径（相对各自项目根） | 借鉴 | 不借鉴（含许可证理由）/ 锚 |
|---|---|---|---|
| **G1 [T1 主]** | `vera20k`（GPLv3 + Section 7）：`docs/Networking.md:253,599`（回放状态哈希 / 逐帧 checksum）、`src/net/command_exchange.rs:248`、`src/net/lockstep.rs:353`（固定提交 `305ea3a75`） | 全状态哈希 + 逐帧 checksum + 帧编排方法论：新增字段须同进 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 四处（ADR 0002 §6 / §7）；bump 后 golden hash 重取、旧 v9 明确拒绝（D3） | 不借鉴 RA2 专属实现（`gamemd` 原生地址 / INI / SHP / VXL / 2D 等距）；GPLv3 + Section 7 **只有真的复制代码时**才进入，本轮只读零复制 → 不进入。锚：`UPSTREAM.md:106`（矩阵 T1）/ `0001-license.md:14,30,66-73` |
| **G2 [T1→T7 可选交叉]** | `ConcordRTS`（GPL-3.0-only，同许可族）：`README.md:22`、`src/core/determinism/RuntimeAuthorityHashPrimitives.h:21-33`（FNV 风格稳定哈希）、`src/network/snapshots/RuntimeMultiplayerSnapshotSession.h:24`、`API.md:296,310`（固定提交 `32ab245`） | 内核 / 表现分离、权威哈希原语、快照恢复契约**方法论**；仅对 D3「四处同步 / 分层边界 / golden hash 重取」作 T1 通用架构交叉 | 不借鉴 RA2/YR 专属格式（SHP / VXL / TMP / MIX / INI）/ 规则 / 地图 / 单位 / 科技 / 数值 / 键位；`README.md:24-26` 自述「逐字节一致 / 验收完成 N 项」**不作本项目已验证事实**；**对 T4 MUST NOT 参考**；只读、不复制、不引依赖、不作子模块、无需 `ADR 0001:66-73` 复核点重评；不得据此声称跨平台确定性已达成。锚：`UPSTREAM.md:112,114` |
| **G3 [T4 主]** | `WarsmashModEngine`（**AGPLv3**）：`core/src/com/etheller/warsmash/viewer5/handlers/w3x/simulation/CGameplayConstants.java:220-222,238,426-436`（三布尔开关 `globalExperience` / `maxLevelHeroesDrainExp` / `buildingKillsGiveExp`、`heroExpRange`、三 getter）、`CUnit.java:3041,3083,3106-3109`（`xpReceivingHeroes` 集合、`1f/size * getHeroFactorXp(level)` 两环节分离） | 借鉴「布尔开关 + 分档表 + 公式族」「进入分摊集合」与「经验入账 / 等级系数」两环节分离、Misc 表字段驱动（D1 / 三类机制接线）。**关键**：这些布尔默认值 / 折减因子 / 各档递减因子在该项目里全是 `miscData.getField*()` 从数据表读、**代码里没有权威真值**，正好支撑 p18「合成占位、不冻结真值」 | 不借真值；**AGPLv3 只借 JASS / 玩法语义、禁复制任何 Java 代码、不引 Java / LibGDX 依赖**；本轮结论不需代码、未触发 `0001-license.md:66-73` 复核点，若将来需要代码 MUST 标注重评。锚：`UPSTREAM.md:109`（矩阵 T4）/ `openspec/config.yaml rules.design` |
| **G4 [T4 主]** | `WarsmashModEngine`（AGPLv3）：`CGameplayConstants.java:16,158-160,172,224-246,323-324,426-436`（`float[][] damageBonusTable` 二维矩阵、`:231-236` 祭坛复活 vs 酒馆买活参数分离 / `heroReviveManaStart` / `heroReviveLifeFactor` / `heroAwaken*`、`:323-324` `getDamageRatioAgainst` 二维查表）、`combat/CAttackType.java:5-13`、`combat/CDefenseType.java:5-13`、`core/src/com/etheller/warsmash/units/manager/MutableObjectData.java:411-428`（`:411` `urac` / `:423` `arac` / `:425` `frac` / `:428` `grac` race 分类字段） | 借鉴 R6.1 阈值分档表驱动 / 递推公式族结构、R6.3「基数 + 等级系数 + 绝对上限」+ 参数分离、R5.3 二维矩阵结构、定义层数据驱动（D1） | 不借金额 / 秒数 / 倍率 / 阈值真值（一律 `[待核]`）；AGPLv3 同 G3。**路径前缀订正见下「R7 锚实地核验结论」**：`MutableObjectData.java` 按 `UPSTREAM.md:109` 矩阵**全路径** `core/src/com/etheller/warsmash/units/manager/...` 解析（**无** `viewer5/handlers/w3x/simulation/` 前缀），避免按同组 Java 锚基路径拼接打不开。锚：`UPSTREAM.md:109` / `WC3-MELEE-RULES.md:366-367` |
| **G5 [T4 主，有真值缺口]** | `WarsmashModEngine`（AGPLv3）：`players/CPlayerState.java:42`（`NO_CREEP_SLEEP`）、`abilities/item/shop/CAbilitySellItems.java:25,28`（货架清单 `itemsSold`）、`behaviors/CBehaviorAttackMove.java:37,41`（`:37` TODO、`:41` 硬编码 `distance <= 16f`） | 借鉴 leash「可打断独立行为 / 睡眠开关化」、售卖「货架清单驱动」结构；三级 leash 状态机只作结构登记（D1） | 不借鉴 `distance <= 16f`（带 TODO）；归巢半径 / 触发阈值 / 三级判据 / 回购比例 / 回购价公式 / 交互距离**无权威锚 → 参考缺口**，一律 `[待核]` + 合成占位；`godot_warcraft3/docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md:122-123`「半价出售」是项目自有值、不得当经典事实（`WC3-MELEE-RULES.md:385`）。锚：`UPSTREAM.md:109` / `WC3-MELEE-RULES.md:369,385,387` |
| **G6 [T2/T3/T5 边界，不作主参考]** | `godot_warcraft3`（MIT）：`docs/design/game/FACTION_UNIT_ROSTERS.md:4,5,9`（原创三族种子、不逐项镜像经典）；`rts_kernel_cs`（MIT）：`README.zh.md:7,21`（不承诺跨平台确定性）、`src/Rts.Kernel/SimulationTypes.cs:3`（`SimVector2(double X, double Y)`） | 借鉴无（只核边界） | 不借原创三族名 / 数值、`double` 路线、C# 实现、资产入库 / Godot 宿主（ADR 0001 / 0002 / `UPSTREAM.md:79,107,110`）；T2 只对齐 R8.5「可打断独立行为」结构、不触移动积分 / A*；T3 / T5 本 change 不命中。锚：`UPSTREAM.md:107,108,110` |
| **G7 [事实源 / 许可证]** | 汇总：`UPSTREAM.md:114,122`；`WC3-MELEE-RULES.md:157,167,169,172,182,194,353,374-391,395`；ADR 0001 / 0002 / 0004 | 「系统该做什么」以 `R-x.y` 为唯一需求源；参考项目源码只佐证结构 / 布尔开关 / 公式族、**不作数值事实源**；`[待核]` 按 `:353` 处置 | 不登记第六个参考项目；参考项目 / 公开攻略 / 网页值一律不作真值来源（`UPSTREAM.md:122` 第 5 条 / ADR 0001 / 0004）；`WarsmashModEngine` AGPLv3 只借 JASS / 玩法语义、不复制代码、不引 Java / LibGDX |
| **G8 [T6 主，p18 新增确定命中]** | `vera20k`（GPLv3 + Section 7）：`services/relay-protocol/src/envelope.rs:27-28`（解码前按版本拒绝）、`services/relay-protocol/src/netsim/compaction.rs:126-127,138`（canonical `ensure(*known == entry.canonical)`）；本项目 `crates/w3-net/src/packet.rs:50`、`crates/w3-net/src/proto/w3net.proto:208,210`、ADR 0005 §3 | 借鉴版本拒绝（旧端把新字段当 unknown field 静默丢弃会导致不可见 desync）、canonical 往返校验、序列号 / 帧编排方法论；承接 p18 `PROTOCOL_VERSION` 9→10 + proto 只追加 tag + payload 双向穷尽映射 + relay 版本拒绝 + canonical 测试（D3 / D4） | 不借鉴其线格式（以 `W3N1` 25B + ADR 0005 为准，本项目自有）；§4 红线不变（只传输入命令、不传游戏状态）；**新增 `GameplayCommandKind` 与否、tag 起点（max 63 → 64）属规格门待确认项**。锚：`UPSTREAM.md:111`（矩阵 T6）/ `0005-protobuf-payload.md:87-88` |

**矩阵命中逐条判定（T1–T7，含两处改判）：**

| 矩阵 | 判定 | 理由 / 边界 |
|---|---|---|
| T1 确定性内核 / 状态哈希 / 命令排序 / 快照（`UPSTREAM.md:106`） | **主命中** | 机制全量落 `w3-core`、字段集合可能变 → 四处同步 + bump 9→10 + golden hash 重取 + 反向验证。主参考 `vera20k`（G1），G2 可选交叉 |
| T4 玩法语义 / JASS / 经典规则（`UPSTREAM.md:109`） | **主命中** | R6.1 / R6.3 / R6.5 / R5.3 / R7.4 / R8.5 玩法语义。主参考 `WarsmashModEngine`（AGPLv3 只借语义，G3/G4/G5） |
| T6 网络传输 / relay（`UPSTREAM.md:111`） | **确定命中（由「条件命中」上调）** | 用户拍板 p18 独占 bump 已把 `PROTOCOL_VERSION` 9→10 + proto 只追加 tag + relay 版本拒绝 + canonical 定为**确定范围**，ADR 0005 连锁无条件触发（与是否新增 `GameplayCommandKind` 无关）。主参考 `vera20k src/net/` + `services/relay-protocol/`（G8），线格式以本项目 `W3N1` + ADR 0005 为准 |
| T2 移动 / 寻路 / 帧阶段（`UPSTREAM.md:107`） | 边缘 | 仅 R8.5 三级 leash「可打断独立行为 / 归巢状态机」结构对齐，不触移动积分 / A*；`rts_kernel_cs` `double` 路线不采用（G6） |
| T7 通用 RTS 架构 / 跨平台验收方法论（`UPSTREAM.md:112`） | 可选交叉（仅 T1 方法论） | bump + 四处同步 + golden hash 重取对应 T7 方法论域（G2）；**对 T4 MUST NOT 参考**；RA2/YR 与 README 自述不作 WC3 事实 / 已验证结论 |
| T3 WC3 数据格式 / SLK / 资产管线（`UPSTREAM.md:108`） | **不命中（由「边缘」下调）** | 用户拍板不冻结真值 → 不读本机导出 SLK / MiscGame 表、不改导出管线 / schema、资产不入库，SLK 核对条件未触发 |
| T5 场景装配 / 相机 / HUD / 3D（`UPSTREAM.md:110`） | 不命中 | 纯 `w3-core` 内核机制 + 文档订正，不触场景 / 相机 / HUD / Melee Bootstrap / 3D |

**R7 锚实地核验结论（p17 遗留缺口、本轮已补齐，写进本小节与 `decision-memo.md`）：**

- 17 个 G1/G2/G3/G4/G5 锚按五个固定提交 `git show` **全部可打开、0 行号漂移、0 内容不存在**（`reference-researcher` 只读对照报告，仓库外证据 / 人工确认项）。
- **唯一 1 处路径前缀歧义（p18 必须订正）**：p17 `design.md` G4 把 `MutableObjectData.java:411-428` 写成相对缩写；若按同组 Java 锚基路径 `core/src/com/etheller/warsmash/viewer5/handlers/w3x/simulation/` 拼接 → 固定提交下**不存在**；按 `UPSTREAM.md:109` 矩阵**全路径** `core/src/com/etheller/warsmash/units/manager/MutableObjectData.java:411-428`（**无** `viewer5/handlers/w3x/simulation/` 前缀）→ 存在、1048 行、`:411/423/425/428` 恰为 `urac` / `arac` / `frac` / `grac`。**本小节 G4 该锚写矩阵全路径并注明「按 `UPSTREAM.md:109` 矩阵条目解析、不带 `simulation/` 前缀」**，避免 `critic` 复核按拼接路径打不开而误判。
- **漂移注记**：`vera20k` 当前工作区 HEAD `d1e28004a`、`ConcordRTS` `029e0b1`，固定提交 `305ea3a75` / `32ab245` 分别是二者**祖先**、工作区已前移 → G1/G2 锚 MUST 标注「按固定提交 `git show` 核实、工作区行号可能漂移、非逐行全量核验」（沿用 p17 `design.md:115,162` 口径）。`WarsmashModEngine` / `godot_warcraft3` / `rts_kernel_cs` 工作区 == 固定提交、无漂移。`critic` 清单第 12 项须用同一 `git -C <根> show <固定提交>:<文件>` 方式复核，勿直读 `vera20k` / `ConcordRTS` 工作区。

**参考缺口（全 `[待核]` + 合成占位、不冻结真值、回到用户拍板 `UPSTREAM.md:122`）：**

1. R6.1 经验阈值分档 / 递推公式（代码无真值）。
2. R6.3 复活 · 买活费用时间 + 半血无蓝参数（分离机制可借、金额 / 秒数 / 倍率无锚）。
3. R5.3 克制矩阵倍率（二维结构可借、倍率由数据表注入、代码无真值）。
4. R7.4 回购比例 / 回购价公式 / 交互距离（无锚）。
5. R8.5 归巢半径 / 触发阈值 / 三级判据（`distance<=16f` 带 TODO 不得借鉴）。
6. 三类机制接线的布尔默认值 / 折减因子 / 各档递减因子查表值（G3 只给结构、代码无真值）。
7. T6 待确认项：是否新增 `GameplayCommandKind`、proto 追加 tag 起点 —— 不臆造、规格门拍板。

**红线边界（逐条声明）：**

- `WarsmashModEngine` **AGPLv3 只借 JASS / 玩法语义、禁复制代码**、不引 Java / LibGDX、不作数值事实源；本轮结论不需代码、未触发 `0001-license.md:66-73` 复核点，若将来需要代码 MUST 标注重评（`CODEBUDDY.md §9` / `openspec/config.yaml rules.design` 硬要求）。
- `ConcordRTS` GPL-3.0-only 只读 / 不复制 / 不引依赖 / 不作子模块；RA2/YR 格式规则数值不作 WC3 事实来源；README 自述不作已验证事实；T4 MUST NOT 参考（`UPSTREAM.md:35,112,114`）。
- `vera20k` GPLv3 + Section 7 只读零复制 → 不进入附加条款；沿用 `W3N1` 25B 帧头 + ADR 0005 线格式（`UPSTREAM.md:50,106` / `0001-license.md:14,30`）。
- `rts_kernel_cs` 的 `double` 不采用；`godot_warcraft3` 原创三族不借；其「半价出售」项目自有值不作经典事实（`WC3-MELEE-RULES.md:385`）。
- 资产与派生 JSON / PNG / GLB / 音效、MPQ 派生数据一律不入库（ADR 0001 / `UPSTREAM.md:63-92`）；仓库外一手明文证据只作机制锚、不进四件套正文数值 / 权威状态 / 哈希 / 快照 / golden hash / CI、不入库、不分发、不冒充已核验。
- 五个只读参考项目不变、不登记第六个（`CODEBUDDY.md §1` / `UPSTREAM.md:9-15`）；**不声称跨平台确定性已达成**（macOS / ARM64 / 其它 rustc / Debian 侧 bump 后逐字节比对仍待 VM / 人工）。

## Risks / Trade-offs

- **[Risk] 把「bump 9→10」误读成放宽 ADR 0002 / 0005 红线、或把证据 / `[待核]` 数值接进权威状态 / CI** → Mitigation：D3 / D8 逐处显式否定「遵循门禁不是放宽」、四处同步 + golden hash 重取 + 反向验证，证据不进门禁；红线锚 `0001-license.md:80-82` / `0003-scope.md:125-138` / `CODEBUDDY.md §5-6`。
- **[Risk] 用户三项拍板被误读成「已过规格门 / 已改代码 / 已冻结真值 / 已实际 bump / 已锁定指纹」** → Mitigation：proposal / spec / D0 / D3 / D10 逐处显式否定；`## 规格门` 保持 `status: pending`、AI 未 self-approve、不填指纹；本轮三版本号仍 9。
- **[Risk] `[待核]` 数值被冻结进权威定义、或据参考项目硬编码值 / 公开攻略臆造结论** → Mitigation：D1 / D6 / D8 逐处 `[待核]` + 合成占位、参考缺口七类如实登记回到用户拍板；锚 `WC3-MELEE-RULES.md:353,374-391`。
- **[Risk] 借用 `WarsmashModEngine`（AGPLv3）具体常量 / 复制 Java 代码，或把 `ConcordRTS` 当 T4 玩法语义 / 已验证事实源** → Mitigation：G3 / G4 / G5 / G7 / 红线边界逐条「不借鉴真值 / 禁复制代码」、`UPSTREAM.md:114` T4 禁止以其为参考、README 自述不作已验证事实，待 `critic` 清单第 12 项复核。
- **[Risk] 新增 `GameplayCommandKind` 与否被臆造、或新增命令未连带 proto tag / relay / canonical / 版本拒绝** → Mitigation：D4 / spec 待确认项 Scenario 参数化承接，若新增 MUST 全套 ADR 0005 §3 门禁；T6 确定命中与是否新增命令无关、不旁路。
- **[Risk] 新增权威状态漏一处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）或跳过反向验证 / determinism 重跑** → Mitigation：D1 / D3 / D5 四处同步 + §5 形态①/② 改坏验红 + `cargo test -p w3-core --test determinism`；补齐 p17 R2 生产结算 A-B 留档。
- **[Risk] R1 可移动性被新机制路径绕过、金矿 / 建筑 / 资源点出现在 `Move` 命令 Scenario** → Mitigation：D7 / spec R1 边界 requirement + 改坏验红反例；`critic` 清单第 13 项复核。
- **[Risk] capability 结构歧义：假设可 MODIFIED `verified-melee-mechanisms` 主 spec、或擅自 archive p17** → Mitigation：D2 依 `openspec list --specs` 判定用独立 ADDED 名、零 MODIFIED；与 p17 衔接 / 未来合并登记为 `decision-memo.md` 待确认项回报 leader、不臆造、不擅自 archive。
- **[Risk] 参考对照锚行号漂移 / 被当成「已逐行核验」，或 G4 `MutableObjectData.java` 按拼接路径打不开** → Mitigation：G4 写 `UPSTREAM.md:109` 全路径、漂移注记标「按固定提交 `git show` 核实 / 非逐行全量核验」、`critic` 用同一 `git -C <根> show <固定提交>:<文件>` 复核；R7 锚实地核验结论如实登记。
- **[Risk] 规则编号引用被冒充规则已实现、真窗口 / 双平台人工清单被标通过** → Mitigation：spec「规则文档同步」requirement + `decision-memo.md`「## 规则文档同步」待办结论（§14 第 5 条 / 三态判定 `:395`）、`critic` 清单第 13 项内容层复核。
- **[Risk] `spec_gate_check()` 对 `status: pending` fail-closed 被误当失败去 self-approve / 写省略说明** → Mitigation：D6 / D10 如实登记为预期信号、不绕过、不填指纹 / `approved_by` / `date`；完整 `verify_all.py` 由 leader 跑。
- **[Risk] 仓库外对内资料被转述 / 受限词表字面命中，或本机绝对路径 / 原版字段名 / rawcode / 技能名写进权威定义** → Mitigation：D8 禁令二分 + 受限词表不内嵌本仓库文件、由 leader 复核字面零命中 + 红线边界；四件套 / ADR 0003 第四次修订 / p13 追加 / `decision-memo.md` 一律不引用、转述、暗示任何仓库外对内资料。
- **[Risk] p13 / ADR 0003 纯追加被误做成改四件套 / 改「## 规格门」approved / 指纹** → Mitigation：Non-Goals 明确纯追加边界、p13 指纹 `7c776a14…` 复核不变、`decision-memo.md` 不在四件套指纹集合内（`tools/verify_all.py:175-195`）。

## Migration Plan

- **本轮（成文期）**：只在 `openspec/changes/p18-full-melee-mechanisms/` 内产出四件套 + `decision-memo.md`，外加 `docs/decisions/0003-scope.md` 文末纯追加第四次修订（+ 文首状态行日期标注）、`openspec/changes/p13-ability-full-roster/decision-memo.md` 末节纯追加 bump 归属补充登记；`## 规格门` 保持 `status: pending`、AI 未 self-approve、不勾任一复选框；不碰 `crates/**` / 主 specs / 规则文档正文 / `tools/**` / CI / 权限配置 / p12–p17 / 归档件 / ADR 其余正文。
- **规格门（本轮终点）**：`critic` 按清单第 7 / 10 / 11 / 12 / 13 项与 `CODEBUDDY.md §8` 四件套一致性出报告 → 用户拍板（不拿沉默当授权）→ 由 leader / `spec-steward` 在人确认后回填 `status: approved` + 四件套指纹（`python tools/verify_all.py --fingerprint p18-full-melee-mechanisms`）+ `approved_by` + `date`（本轮不填）；批准即收敛「是否新增 `GameplayCommandKind` / proto tag 起点 / p17 归档时序与 capability 合并」等待确认项。此后规格锁定，改任一 artifact MUST 按 §8 同步回查其余三份并重走规格门。
- **apply（后续独立会话，按 owner 串行、一个 change 一个 owner、不同时改多 owner 文件）**：按 D1–D10 落地——`sim-owner` 改 `crates/w3-core/**`（机制接线 / 买活半血无蓝 / 阈值分档 / 克制矩阵 / 回购 / leash / 四处同步 / 三版本号 9→10 / 新增 CPU 回归 + 反向验证 + p17 R2 A-B + R4 恒真断言订正 / golden hash 重取 / 旧 v9 拒绝）、`net-owner` 改 `crates/w3-net/**` / `crates/w3-relay/**`（`PROTOCOL_VERSION` 9→10 / proto 只追加 tag / payload 双向穷尽映射 / relay 版本拒绝 / canonical）、`docs-owner` 改规则文档（§14 第 5 条收尾同步）；重跑 `cargo test -p w3-core --test determinism` / `cargo test --workspace` / `cargo fmt --all -- --check` / `python tools/verify_all.py`；真值不冻结、零资产。
- **收尾 / 规则文档同步**：按 §14 第 5 条在 `decision-memo.md`「## 规则文档同步」给结论、由 `docs-owner` 更新附录「规则 → 实现覆盖对照」R5 / R6 / R7 / R8 对应行与 §15（注明日期与依据、不静默改规则、不写 `[待核]` 数值 / 本机导出字段名 / rawcode / 技能名），`critic` 清单第 13 项内容层复核；真窗口 8.x 人工清单、golden hash v10 双平台逐字节比对、CI 双平台一律「待人工 / 待 VM」。
- **回滚**：本 change 成文产物为纯文档 + 后续 `w3-core` / `w3-net` 机制与 bump 全套（含新增权威状态 / 版本号 / proto tag / golden hash），删除 / 回退文档与代码即可；旧 v9 快照 / 回放按明确拒绝处理、不提供迁移；证据台账留仓库外本机、不随发行物分发；若规格门改判全量方案动了权威状态 / bump，回滚面另按 ADR 0002 复核。
- **人工 / VM 项**：L2 反编译补 R3.2 / R3.3 upkeep 由用户本人本机执行、只登记人工确认项；人工 push → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` / proto 生成代码字节一致 → `/opsx:archive`；AI 不 push / fetch / pull / clone、不 archive、不本地 commit（本轮由 leader 统一处理）。

## Open Questions

> 以下均为「可安全延后而不改变 specs / approach / task breakdown」的未知项，已由 D0 / D1 / D2 / D3 / D4 / D6 / D10 参数化承接并登记在 `decision-memo.md`「待确认项」，由规格门 / leader 拍板；本轮**不臆造为已定案**：

- **是否新增 `GameplayCommandKind`、proto 追加 tag 起点**（brief 点名，任务点名）→ 待规格门用户拍板；若新增则连带 proto tag / relay 版本拒绝 / canonical 全套（D4 / G8），不旁路 ADR 0005 §3；二选一不推翻 spec 行为式 SHALL / tasks 条件式结构。
- **p17 归档时序与 capability 合并 / 去重**：p17 未归档、`verified-melee-mechanisms` 未进主 specs，p18 用独立 ADDED 名；两者归档后是否合并 capability、p17 归档时序是否影响 p18 归档 → 待规格门 / leader 拍板，不擅自 archive p17、不臆造前置依赖（D2）。
- **机制全量各子项是否全部改变 `HeroState` / 新建 capability 字段集合**：R6.1 阈值分档 / R5.3 克制矩阵可能改权威状态字段集合 → 具体字段集合由规格门 / apply 阶段 `sim-owner` 按 ADR 0002 四处同步定（D1 / D3），本轮不臆造字段。
- **七类参考缺口（`[待核]` 数值）的处置**：R6.1 / R6.3 / R5.3 / R7.4 / R8.5 / 三类机制查表值 / T6 tag 起点一律 `[待核]` + 合成占位；是否补 L2 反编译取证由用户拍板、不臆造（D9 / `UPSTREAM.md:122`）。
- **反向验证具体形态（§5 形态① 测试内反例 / 形态② A-B 对照）与缺陷态构造**、p17 R2 生产结算 A-B 的命令输出留档 → 待规格门 / apply 阶段 `sim-owner` 按 §5 定（D5）。
- **规则文档同步（附录覆盖对照 / §15）的收尾落实、R1/R5/R6 成文表述瑕疵的收口时机** → 待规格门批准记录回填后的 apply 阶段由 `docs-owner` 按 Q-A 已拍板顺序落实（D6 / §14 第 5 条）。
- **规格门批准记录回填**（四件套指纹 / `critic` 报告 / `approved_by` / `date`）→ 待用户拍板后由 leader / `spec-steward` 回填，本轮一律不填、AI 未 self-approve（D0 / D10）。
- **参考矩阵 T1–T7 命中判定与锚漂移复核** → 本轮据 `UPSTREAM.md:104-114` 判定 T1 / T4 / T6 主命中（T6 确定命中上调、T3 不命中下调）、T2 边缘、T7 可选交叉、T5 不命中，待 `critic` 清单第 12 项复核；若改判命中须补对照报告并回到用户拍板。
- **真窗口 / 双平台验收线** → 真窗口 8.x 人工清单、golden hash v10 双平台逐字节比对（macOS / ARM64 / 其它 rustc / Debian）、CI 双平台 `verify_all.py` / proto 生成代码字节一致一律「待人工 / 待 VM」、不标通过（D10）。
