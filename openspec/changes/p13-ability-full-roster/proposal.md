# Proposal

> 本轮只成文、不实施代码（apply 是后续独立会话）。本 change 为承接已归档 `2026-11-25-p11-ability-system` 决策纪要「## 已知限制」的**玩法全量实现**独立 change，用户 2026-11-25 明确拍板「建两个 change（文档 + 玩法各一）」，本 change 为玩法侧，文档侧见 `p12-ability-doc-reconciliation`。本 change 触及 `w3-core` 确定性内核（`CODEBUDDY.md §5` 硬门禁适用）。`## 规格门` 保持 `status: pending`、由人拍板、AI 未 self-approve（见 `decision-memo.md`）。

## Why

按 1.0 范围 **S3**（4 个种族功能做完，`WC3-MELEE-RULES.md:27`）、**S5**（地图上的野怪，`:29`）、**S6**（中立建筑功能，`:30`）、**S4**（各种道具，`:28`）与业务规则 **R6**（英雄技能 / 等级 / 6 级大招，`:165-171`）、**R8**（野怪，`:188-196`）、**R9**（中立建筑，`:200-218`）、**R7.6**（道具 abilList 走既有 `EffectDef` 组合，`:184`）、R2（四族，`:113-126`）、R12.4（回放确定性，`:248`）、R10.5（表现层只读，`:229`）、§14（活文档维护，`:418-424`）：P11 本期只做「四族英雄技能全量 + 各族代表性单位 / 道具技能」，**中立英雄 / 野怪 / 全部单位技能顺延、标已知限制**（P11 `decision-memo.md`「已知限制」第 1 条）；且 Q1「清单逐项按本机导出 SLK 核对」在 apply 阶段实际落地为**经典结构口径的合成占位表 72 条**（`crates/w3-core/src/ability_system.rs:31,121-177` `SYNTHETIC_ABILITY_CATALOG`、`crates/w3-client/src/melee/defs.rs:734` `FOUR_RACE_ABILITY_CATALOG`），**未经本机导出 SLK 逐项核对**，与收尾经 leader 只读实测的导出表系统性不一致（P11 `decision-memo.md`「已知限制」第 11 条，仓库文件外证据 / 人工确认项）。用户 2026-11-25 已拍板「接受水位为已知限制、由后续独立 change 承接」（`archive/2026-11-25-p11-ability-system/decision-memo.md:100`）。本 change 即承接这两项的玩法全量实现。**明确不做** N1–N6，尤其 **N5 JASS / wtg 触发器 VM**（`WC3-MELEE-RULES.md:43-49`）。

## What Changes

- **顺延项全量**（承接 P11 已知限制 1）：把中立英雄 / 野怪 / 全部单位技能纳入全量实现，依据 S3 / S5 / S6 / R6 / R8 / R9 / R7.6 / §14（规则编号引用作需求依据、不冒充已实现；规则文档同步结论见 `decision-memo.md`「## 规则文档同步」）。
- **Q1 清单逐项 SLK 对齐**（承接 P11 已知限制 11）：按本机导出 `UnitAbilities.heroAbilList` / `abilList` + `AbilityData`（`code` / `race` / `hero` / `levels` / `reqLevel` 列）**人工逐项核对后**把真实清单条目与分类固化；替换合成占位表 72 条的结构口径。
- **两层张力显式写清**：对齐的是**清单条目与分类**（哪些技能、每族几个、英雄 3 普通 + 1 大招、单位 / 道具技能），**不是**照搬经典名称 / 四字符 rawcode / 数值；内部仍用合成原创 `DefId`、**不引入四字符 ID → `DefId` 映射层**、数值一律 `[待核]` 不入库、**不进权威状态 / 状态哈希 / 快照**（ADR 0001 / 0002 / 0004）。
- **人工确认项边界**：清单逐项人工核对是**仓库文件外证据、人工确认项**，proposal / design / `decision-memo.md` MUST NOT 冒充「已逐项核对完成」；实际条目数核对后如实登记、**不臆造精确数字**；数值 `[待核]` 不静默改规则（`WC3-MELEE-RULES.md §14`）。
- **确定性契约（若新增权威状态）**：MUST 四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）+ `WorldGameplayState::hash`，并按 ADR 0002 评估是否 bump `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION`（当前权威值均为 **9**，若 bump 则 **9→10 全套**）；`cargo test -p w3-core --test determinism` + `CODEBUDDY.md §5` 形态①改坏验红。是否新增权威状态 / 是否 bump 属「待确认项」，不臆造为已定案。
- **验收边界**：真窗口 8.x 人工清单保持「待人工、不冒充通过」；golden hash 双平台 / CI 双平台 / 参考缺口如实登记，不写成已验证；N1–N6（尤其 N5 JASS VM）明确不做。
- **不纳入**：主 spec 历史 `==8` 字面 / 清单规模 91→95 列 / tasks 5.2 表述订正（属 `p12-ability-doc-reconciliation`）；真窗口 / golden hash / CI 的实际执行（人工 / VM 项）。

## Capabilities

### New Capabilities
<!-- 无。本 change 只修改既有 `ability-system` capability（P11 已归档主 spec `openspec/specs/ability-system/spec.md`），不新增近重复 capability。 -->

### Modified Capabilities
- `ability-system`: 把既有 requirement「四族技能清单按 SLK 逐项对齐并人工固化」的**本期范围口径**由「顺延标已知限制 + 合成占位表」改为「顺延项全量 + 清单逐项 SLK 人工核对后固化」（MODIFIED，逐字保留其两个历史 `#### Scenario:` 身份键、口径只改在 THEN）；并 ADDED 新 requirements 覆盖顺延项全量的规则依据、清单逐项人工核对的人工确认项边界、两层张力与零资产、新增权威状态的确定性契约与 bump 评估、验收线与人工清单边界。delta 为既有 capability，**不写 `## Purpose`**（不覆盖主 spec 既有 Purpose）、delta 顶部不写说明块、不新增凑数的 MODIFIED delta。

## Impact

- **路线图阶段**：P13（承接 P11 已归档的玩法全量独立 change）；P0–P5 及 P6–P12 均已实施并归档，见 `CODEBUDDY.md §1`。玩法 change 引用 `WC3-MELEE-RULES.md` 的 `R-x.y` 与 S1–S6 为需求源；余项（清单实际条目数、验收线、数值、bump 与否）进 `decision-memo.md`「待确认项」。
- **ADR**：0001（资产 / 法律：只借清单结构与分类语义、不复制 AGPLv3 代码、数值 `[待核]` 不入库）、0002（确定性硬门禁：新增权威状态进哈希 / 快照、四处同步、bump 评估、golden hash 重取、形态①反向验证、跨平台待验证口径）、0003（范围：N1–N6 不做、尤其 N5 JASS VM）、0004（冻结定义：清单人工核对后固化成编译期常量表、原始导出数据不进快照 / 哈希）、0005（若 bump 9→10：proto 只追加 tag、删字段保留 `reserved`、schema 变更同步 bump `PROTOCOL_VERSION`）。ADR 记「为什么决定」、spec 记「系统该做什么」（`CODEBUDDY.md §8`）。
- **确定性契约**：**若**新增权威状态 MUST 同时进入 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION`（当前 9；若 bump 则 10）定义的快照，四处同步并 `WorldGameplayState::hash`；`cargo test -p w3-core --test determinism` 必须全绿并含 `CODEBUDDY.md §5` 形态①「改坏验红」反向验证；30 Hz 整数帧 / `SimFixed` Q47.16 / `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序 / 稳定 `EntityId` 顺序 / FNV-1a 编码不变；表现层只读派生、不反写权威状态（R10.5）。**不得声称 bump 后双平台 / 全平台确定性已验证**（沿用 `0002-determinism.md:118` 口径）。
- **资产 / 法律边界**：零暴雪资产、零 MPQ 派生 JSON / PNG / GLB / 音效、零 `*.j` / `.jui` / JASS 代码入库；`WarsmashModEngine`（AGPLv3）只借鉴语义、不复制代码；`ConcordRTS`（GPL-3.0-only）只读、其 RA2/YR 数值 / 规则 MUST NOT 作 WC3 事实来源；本机导出 SLK 的具体技能名 / 四字符 rawcode / 数值 MUST NOT 写进四件套或仓库，只登记规模与分类（导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库，ADR 0001）；`WC3-MELEE-RULES.md` 为 WC3 业务规则唯一权威。
- **代码 / 系统（apply 阶段由对应 owner 分工，本轮只成文）**：`crates/w3-core/`（`sim-owner`：清单常量表 / 权威状态 / 四处同步 / `tests/determinism.rs`）、`crates/w3-net/` + `crates/w3-relay/`（`net-owner`：若 bump 则 `packet.rs` / `w3net.proto`）、`crates/w3-client/`（`client-owner`：`defs.rs` 四族 / 顺延项技能定义）、`crates/w3-content/`（`content-owner`：本机导出 SLK 清单逐项人工核对、数值 `[待核]`、不入库）、`docs/references/WC3-MELEE-RULES.md`（`docs-owner`，仅 apply 阶段按已拍板顺序）。一个 change 一个 owner，apply 阶段串行、不同时改多 owner 文件。
- **依赖**：不新增第三方依赖；`prost` / `prost-build` 版本锁定不变（ADR 0005 §3）。
