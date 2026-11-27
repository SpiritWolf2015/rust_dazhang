# Spec Delta

## Purpose

本 capability 定义文档 / 规格口径订正的系统级契约：主 `openspec/specs/**` 表达当前版本号的消费者必须与 `crates/w3-core/src/snapshot.rs` / `crates/w3-net/src/packet.rs` 的三版本号权威定义值一致；本机导出表清单规模元数据必须如实登记、不臆造精确数字；冻结定义哈希契约的表述必须与 ADR 0002 既有 `config.content_hash` 身份字段一致。本 capability 为纯文档 / 规格订正，零新增权威状态、零确定性影响、零资产（ADR 0001 / ADR 0002 / ADR 0004 / ADR 0005）。

## ADDED Requirements

### Requirement: 主 spec 版本号消费者与三版本号权威值一致

主 `openspec/specs/**` 中所有表达**当前版本号**的消费者字面 SHALL 与 `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 的权威定义值（当前 9）一致；历史 delta 合并残留的无条件 `==8` /「保持 8」/「当前 8」/「三版本号 = 8」硬断言 MUST 在 apply 阶段订正为 9。订正 MUST 逐字保留历史 requirement 与 `#### Scenario:` 身份键、口径只改在 THEN；历史沿革叙述里各 change 的「bump 8」实施期史实 MUST NOT 被抹平（P9 Q9' 已拍板「归档后由独立 change 收口」）。

#### Scenario: 全仓 Grep 旧版本号消费者残留为 0

- **WHEN** 对 `openspec/specs/**` 全仓 Grep 无条件 `==8` /「保持 8」/「当前 8」/「`SNAPSHOT_VERSION = SIM_VERSION = PROTOCOL_VERSION = 8`」字面并对照 `crates/w3-core/src/snapshot.rs:67-68` 与 `crates/w3-net/src/packet.rs:50` 的权威值
- **THEN** 表达当前版本号的消费者残留数为 0、三处权威定义 == 9 全仓一致；历史 bump 8 沿革叙述逐字保留；`openspec validate --all` 与 `python tools/verify_all.py` 通过

#### Scenario: 订正不新增 MODIFIED delta 且不改 Scenario 身份键

- **WHEN** apply 阶段订正主 spec 的 THEN 版本号口径
- **THEN** 每个被订正 requirement 的 `### Requirement:` 标题与其下各 `#### Scenario:` 标题逐字不变、仅 THEN 内字面改为 9；不新增第 10 份 MODIFIED delta 凑数；`git diff -- openspec/specs/` 在提案 / 成文阶段为空（主 spec 只在 archive 合并时复核或 apply 阶段改）

### Requirement: 本 capability 零新增权威状态、零确定性影响、零资产

本 capability 的所有订正 SHALL 为纯文档 / 规格口径表述，MUST NOT 改 `crates/w3-core/` 任何权威状态、状态哈希、快照字段、`first_difference()` 或帧序；MUST NOT 新增权威状态（故无需进入状态哈希或由 `SNAPSHOT_VERSION` 定义的快照）、MUST NOT bump 三版本号；`CODEBUDDY.md §5` 硬门禁与形态①反向验证不适用。仓库 MUST NOT 引入任何 `*.slk` / MPQ 派生 JSON / PNG / GLB / 音效。

#### Scenario: 零内核改动与零资产的可执行验证

- **WHEN** 取 `git diff -- crates/w3-core/`、检查三版本号常量未变、并跑 `python tools/verify_all.py` 的资产扫描
- **THEN** `crates/w3-core/` 差异为空、`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 仍 == 9；资产扫描通过、`git ls-files` 无禁用后缀；无需重跑 `cargo test -p w3-core --test determinism`（不触确定性代码）；`python tools/check_encoding.py` 与 `openspec validate p12-ability-doc-reconciliation --strict` 通过

### Requirement: 本机导出表清单规模元数据口径如实

文档 / 主 spec 中登记的本机导出 `AbilityData.json` 清单规模元数据 SHALL 如实反映人工核对口径（803 条 / 95 列），或删除具体列数、统一为「粗统计、以人工核对为准」表述；MUST NOT 保留过时的 91 列数字。登记 SHALL 只含规模与分类，MUST NOT 含任何具体技能名 / 四字符 rawcode / 数值；导出数据留玩家本机、gitignored `config.toml` 提供路径、不入库（ADR 0001 / ADR 0004 / `WC3-MELEE-RULES.md §14`）。

#### Scenario: 清单规模数字订正与归档件零改动

- **WHEN** 核对 P11 归档件 `design.md`「清单规模事实锚」登记的 91 列与 content-owner 本机导出实测 95 列
- **THEN** 后续主 spec / 文档登记 95 列或统一为粗统计口径、其余规模元数据一致；P11 归档件四件套与 `decision-memo.md` 零改动（已 `status: approved` + 指纹锁定、不追溯改写）；全仓 Grep 无「91 列」作为既成事实残留；`python tools/verify_all.py` 资产扫描通过

### Requirement: 冻结定义哈希契约表述准确

涉及冻结定义哈希契约的成文表述 SHALL 与 P5 以来 ADR 0002 既有契约一致：改任一冻结常量会经既有 `config.content_hash`（= `FrozenDefinitions::content_hash()` 冻结内容身份）传导改变 `compute_state_hash()`；定义层冻结常量 MUST NOT 被表述为「作为独立运行态字段二次混入 `compute_state_hash()`」。P11 `tasks.md` 5.2「不改运行时 `compute_state_hash()`」的歧义表述 MUST 在后续主 spec / 文档订正为上述准确口径（成文表述订正、非实现缺陷）。

#### Scenario: 表述订正与既有契约一致

- **WHEN** 核对 P11 `tasks.md` 5.2 字面与 `crates/w3-core/src/sim_match.rs:4328`（`hasher.write_u64(self.config.content_hash)`）/ `:4375`（`hasher.write_u64(self.defs.content_hash())`）/ `docs/decisions/0002-determinism.md §6` 的 `content_hash` 既有参与字段
- **THEN** 订正后的表述写明「会经既有 `config.content_hash` / `defs.content_hash()` 身份传导改变 `compute_state_hash()`；冻结定义未作为独立运行态字段二次混入」；ADR 0002 §1–§8 正文与运行时哈希字段集合零改动、P11 归档件四件套零改动；`openspec validate --all` 通过
