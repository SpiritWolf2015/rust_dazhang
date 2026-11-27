# Proposal

> 本轮**只成文、不实施代码**：改动仅落在 `openspec/changes/p17-melee-mechanism-alignment/`（四件套 + `decision-memo.md`）与 `docs/decisions/0003-scope.md`（第三次修订、纯追加）；**不碰 `crates/`**（那是规格门批准后 `sim-owner` 的 apply 阶段）。本 change 是**玩法 change**：把「本机一手明文数据核对出的 WC3 业务规则机制性结论」正式落进 `w3-core` 的需求规格，供后续规格门与 apply。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。一手明文证据台账为**仓库外 / 人工确认项 / 行号可能漂移**，不入库、不冒充已核验。

## Why

用户已拍板引入 clean-room 反编译 / 一手明文数据取证流程（ADR 0003 第二次修订、p15 形态 A 试点、p16 流程说明文档），但此前所有产出都停在「流程规范 / 证据脚本 / 文档」，**一行游戏代码没动**；用户要求「真改 `w3-core`」并授权「规格门 → 真改 `w3-core`」走完整 `CODEBUDDY.md §9` 串联。2026-11-26 已产出两份仓库外证据台账（事实源为用户合法自备经典客户端经 `godot_warcraft3` 导出的明文 JASS / MiscGame / MiscData / SLK 表），并已按 `docs/references/WC3-MELEE-RULES.md §14` 第 2 条把 **R6.5**「满级英雄参与经验分摊」写进规则文档（commit `9f2d3e7`，§15 有该行）。但 `crates/w3-core/src/sim_match.rs:4035-4036` 注释仍是旧 `[待核]` 措辞、且 R6.5 / R6.1 / R6.3 等已实现机制**无确定性回归锁定**，机制性结论尚未正式进入 `w3-core` 需求规格。本 change 补齐这一缺口：把 **R6.1 / R6.3 / R6.5 / R7.4 / R8.5 / R5.3** 的机制结构 / 布尔开关 / 公式族落进规格并锁定防退化；落在 1.0 范围 **S1（联机对战）/ S3（四族英雄）/ S4（道具）/ S5（野怪）/ S6（中立建筑）** 之内，**不触碰 N1–N6**（JASS VM / 自定义地图 / 战役 / 资产入库等）。`[待核]` 口径以附录「核实约定」(`WC3-MELEE-RULES.md:353`) 为准，不沿用「§11 核实约定」历史误称、不静默改规则文档正文（§14）。

## What Changes

- **新增单一 capability `verified-melee-mechanisms`**（ADDED Requirements）：以 SHALL/MUST + 可执行 `#### Scenario:` 定义「R6.5 满级英雄参与分摊」「经验结算两环节 / 稳定 `EntityId` 顺序 / 均分」「布尔与因子机制以合成占位表达」「R6.3 复活费用 / 时间公式族上限」「确定性回归 + 反向验证（改坏验红）」「证据 / 数值 / 资产 / 确定性边界」「参考缺口如实登记、顺延不臆造」「规则文档同步与成文表述瑕疵登记」八类行为契约；每条引用 `R-x.y` 与 ADR 编号。
- **推荐首期范围（最小可验收闭环，待规格门由人拍板、`decision-memo.md` 待确认项）**：① 订正 `sim_match.rs:4035-4036` 注释为「2026-11-26 按 §14 核对：满级英雄参与分摊（见 §15）」，**结算行为不变**；② 补齐「建筑击杀是否授予经验」「召唤物击杀经验折减」「等级经验因子分档递减」三类**布尔 / 因子机制结构**（用合成占位值表达、不改字段集合；若实现需改字段则顺延 p18）；③ 新增 CPU 确定性回归 + 反向验证，断言「满级英雄参与分摊」「集合内英雄均分」「稳定 `EntityId` 顺序」「复活费用 / 时间公式族上限生效」，并**重跑 `cargo test -p w3-core --test determinism`**。
- **明确不做（硬边界）**：不新增 / 不修改权威状态字段集合、不新增 `GameplayCommandKind`（避免连带 T6 / ADR 0005 proto schema 与 `PROTOCOL_VERSION` bump）、不 bump 三版本号（当前 9）、不连锁 `w3-net` / `w3-client` / `w3-relay`；不冻结任何原版真值（数值一律 `[待核]` + 合成占位）；不写本机绝对路径 / 原版具体数值 / 字段名 / rawcode / 技能名；不把仓库外台账写成「已入库 / 已逐行核验」；不改 `docs/references/WC3-MELEE-RULES.md` 正文 / 主 `openspec/specs/**` / `tools/**` / CI / 权限配置 / p12–p16 / 归档件。
- **顺延登记（已知限制 / 全量方案备选，建议 p18）**：R6.1 经验阈值分档表、R5.3 克制矩阵新建 capability、R7.4 回购价公式、R8.5 三级 leash、R6.3 买活半血无蓝、L2 反编译补 R3.2 / R3.3 upkeep —— 均改权威状态结构、需 bump 三版本号与 golden hash 重取、连锁网络 / 客户端层，风险与工作量显著更大。
- **决策纪要同步产出** `decision-memo.md`（三必填小节 + 「## 规则文档同步」待办结论 + 「## 规格门」`status: pending`）；**ADR 0003 第三次修订**（纯追加）明确把取证范围从「单个呈现特性」正式扩到「业务规则机制性结论」，红线不变、1.0 范围不变，省略规格门条款引用 `CODEBUDDY.md §9`（非 §8）。

## Capabilities

### New Capabilities
- `verified-melee-mechanisms`: 一手明文数据核对后的 Melee 业务规则机制性结论在 `w3-core` 的确定性落地契约 —— 承载经验分摊 / 复活公式族 / 布尔与因子机制 / 反向验证 / 证据与数值边界等 requirements，随 p18 全量方案继续演进；用 SHALL/MUST 与可执行验证表达，不写内部类名 / 字段名 / 具体数值 / 本机绝对路径。

### Modified Capabilities
<!-- 无。本 change 只新增一个 capability；本轮成文不改任何既有主 spec 的 requirement（不新增 MODIFIED delta），主 `openspec/specs/**` 零改动、`git diff -- openspec/specs/` 为空。既有主 spec `creep-camps/spec.md:36` 的「满级英雄是否参与分摊 `[待核]` 不冻结」历史措辞与 2026-11-26 拍板的字面不一致，如实登记为 `decision-memo.md`「已知限制」与收尾「规则文档同步核对」待办，由规格门拍板收口时机，不静默改主 spec、不冒充已对齐。 -->

## Impact

- **路线图阶段**：P17（P0–P5 及 P6–P16 全部已实施并归档 / 成文，见 `CODEBUDDY.md §1` 与 `openspec list`）；本 change 属 1.0 范围内既有玩法的**机制对齐 / 回归锁定**，不新增范围编号、不动 S1–S6 / N1–N6、不改状态标记口径。
- **ADR**：0001（零资产 / 一手明文数据留本机仓库外 / 不复制代码 / 不绕 EULA · DRM · 登录 · 反作弊）、0002（`SimFixed` Q47.16 / 30 Hz / 稳定排序 / FNV-1a / 快照 9 四处同步 / 证据不进门禁 / 反向验证）、0003（第三次修订：取证范围扩到业务规则机制性结论、红线不变、1.0 范围不变）、0004（冻结坐标 / `[待核]` 数值不冻结）、0005（不新增命令变体即不连带 proto schema 与 `PROTOCOL_VERSION`）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）；本 change **不新增 / 不改任何 ADR 正文**（0003 仅纯追加修订记录）。
- **确定性契约**：首期**零新增权威状态字段**（故无需进入 FNV-1a 状态哈希或由 `SNAPSHOT_VERSION` 定义的快照，当前 9，锚 `crates/w3-core/src/snapshot.rs:67-68`）、不改 `first_difference()` / 帧序 / 命令排序 / 哈希编码、不 bump 三版本号（`crates/w3-net/src/packet.rs:50` 亦为 9）。但本 change **真改 `w3-core` 确定性代码**（注释订正 + 机制补齐 + 回归），MUST 重跑 `cargo test -p w3-core --test determinism`、MUST 提供 `CODEBUDDY.md §5` 形态① / ②「反向验证（改坏验红）」；若规格门改判全量方案而动权威状态字段，MUST 四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）+ bump + 反向验证，详见 `design.md` D7。证据台账 MUST NOT 进权威状态 / 状态哈希 / 快照 / golden hash / determinism / CI。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效入库；不新增第三方依赖、`Cargo.lock` 不变、不登记 `NOTICE.md`；仅用户合法自备经典客户端、不绕 EULA / DRM / 登录 / 反作弊、不声称获暴雪授权；明文一手数据与证据台账留本机仓库外、不入库不分发；不写本机真实绝对路径 / 凭证。
- **代码 / 系统**：本轮只成文，改动仅落在 `openspec/changes/p17-melee-mechanism-alignment/` 与 `docs/decisions/0003-scope.md`；apply 阶段由 `sim-owner` 改 `crates/w3-core/src/sim_match.rs` / `gameplay.rs` / `creep.rs` 与 `crates/w3-core/tests/**`（一个 change 一个 owner），不改 `crates/w3-net/**` / `w3-client/**` / `w3-relay/**` / 主 specs / `docs/references/**` / `tools/**`。
- **参考项目矩阵**：`docs/references/UPSTREAM.md:104-114` 命中 **T1（主：机制落 `w3-core` + 权威状态四处同步）、T4（主：R6.5 / R6.1 / R6.3 / R7.4 / R8.5 / R5.3 玩法语义）**、T7 仅对 T1 架构方法论可选交叉（对 T4 玩法语义 MUST NOT 参考）、T2 边缘（leash 只对齐机制结构）、T3 / T5 / T6 不命中；`reference-researcher` 已产出并核实对照报告（G1–G7），逐条落进 `design.md`「## 参考项目对照」，缺口如实登记回到人拍板，待 `critic` 清单第 12 项复核。
- **依赖 / CI**：不新增依赖、不接 CI；参考缺口（R6.1 阈值分档 / R6.3 费用时间 / R5.3 倍率 / R7.4 回购价公式 / R8.5 归巢半径与触发阈值）一律 `[待核]` + 合成占位、不臆造；真窗口 8.x 人工清单 / golden hash v9 双平台 / CI 双平台一律「待人工 / 待 VM」、不标通过。
