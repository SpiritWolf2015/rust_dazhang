# full-melee-mechanisms Specification

## Purpose

把 p17 顺延的 Melee 机制性结论（三类经验机制运行时接线 / R6.3 买活半血无蓝 / R6.1 阈值分档 / R5.3 克制矩阵 / R7.4 回购 / R8.5 三级 leash）连同权威状态字段集合变更所需的 bump 9→10 全套确定性契约、proto 只追加 tag 与 canonical、反向验证、文档订正和证据红线，定义为 `w3-core` 与 `w3-net` 的可验证行为契约：只落机制结构 / 布尔开关 / 公式族 / 矩阵结构，数值一律 `[待核]` 合成占位、不冻结真值。

## Requirements

### Requirement: 三类经验机制接入确定性结算路径（R6.1 / R6.5）

系统 SHALL 把三类机制接入击杀结算的运行时路径：建筑击杀是否授予经验的布尔开关、召唤物击杀经验折减因子、按英雄等级分档递减的经验因子查表（R6.5 / R6.1）。三者数值一律 `[待核]` + 合成占位、不冻结原版真值，MUST NOT 引入 `f64`（ADR 0001 / 0002 / 0004 / `WC3-MELEE-RULES.md:353`）。机制结构只借鉴 `design.md`「## 参考项目对照」G3 的「布尔开关 + 分档表 + 公式族」与「进入分摊集合 / 经验入账」两环节分离语义、不借真值。p17 仅有编译期合成占位、未接入结算路径，本 change 补齐接线。

#### Scenario: 三类机制接线生效且确定（R6.1 / R6.5 / ADR 0002）

- **WHEN** 在冻结定义下构造建筑击杀 / 召唤物击杀 / 不同等级英雄同场，推进击杀结算帧，并以相同冻结定义与开局脚本重开一局
- **THEN** 布尔开关决定建筑击杀是否进入经验分摊、折减因子按合成占位值作用于召唤物击杀收益、等级分档因子按英雄等级查表递减；逐帧 `compute_state_hash()` 与同脚本基线一致、`state.xp` 确定性增长、按稳定 `EntityId` 顺序、重开无残留（R11.5 / R12.4）；`Grep` 结算路径命中三类机制消费者，MUST NOT 仅被测试引用（p17 R3 缺口）

#### Scenario: 三类机制缺陷态被反向验证拦截（R6.1 / R6.5 / CODEBUDDY.md §5）

- **WHEN** 以退化实现（布尔开关恒真 / 折减因子置 1 / 分档表退化为单档）替换正式实现并跑新增回归
- **THEN** 对应反例断言失败、修复后通过，两条证据可复现（`CODEBUDDY.md §5` 形态① / ②），仅「全绿」不构成验证

### Requirement: R6.3 买活半血无蓝参数落地（R6.3）

系统 SHALL 在酒馆买活路径落地「复活时半血无蓝」参数，祭坛复活与酒馆买活参数 MUST 分离；复活费用 / 时间继续读冻结的 `[待核]` 合成整数常量、按「基数 + 等级系数 → 绝对上限」表驱动计算，`i128` 中间量、整数截断、MUST NOT 引入浮点（R6.3 / R9.3 / `WC3-MELEE-RULES.md:169,353` / ADR 0002 / 0004）。`crates/w3-core/src/gameplay.rs:43` 的「酒馆按 Q4 不在本期范围」注释 SHALL 随实现订正。数值一律 `[待核]`、不冻结真值。

#### Scenario: 买活半血无蓝与参数分离（R6.3 / ADR 0002）

- **WHEN** 分别以祭坛复活与酒馆买活提交同一等级阵亡英雄、并推进复活结算帧
- **THEN** 买活英雄以半血无蓝状态入场、祭坛复活不受该参数影响，费用 / 时间在绝对上限处截断、不越界无负值；逐帧哈希与同脚本基线一致（`compute_state_hash()`）；`Grep pattern="酒馆按 Q4|半血无蓝|half" path="crates/w3-core/src/gameplay.rs"` 反映实现、旧「不在本期范围」注释不再作为未实现声明

### Requirement: R6.1 经验阈值分档表与新增权威状态四处同步（R6.1 / ADR 0002）

系统 SHALL 以英雄等级分档表驱动升级阈值（R6.1 / `WC3-MELEE-RULES.md:167`），阈值数值一律 `[待核]` + 合成占位。若实现新增 / 修改权威状态字段集合（如 `HeroState` 字段、分档表运行态），该字段 MUST 同时进入 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION` 定义的快照（成文期当前 9、apply 阶段 bump 10），并同步 `compute_state_hash` / `first_difference` / `capture_snapshot` / `restore_snapshot` 四处（`crates/w3-core/src/sim_match.rs:4320,4399,4481,4581`）；MUST NOT 由表现层反写（ADR 0002）。

#### Scenario: 阈值分档与字段进哈希 / 快照（R6.1 / ADR 0002）

- **WHEN** 以分档表驱动英雄升级、并对比两局仅该字段取值不同的权威状态
- **THEN** 升级按冻结合成分档常量确定发生；`compute_state_hash()` 与 `first_difference()` 报告该字段差异、`capture_snapshot` / `restore_snapshot` 往返逐字节一致；`cargo test -p w3-core --test determinism` 全绿并经反向验证（漏写该字段的缺陷态断言必失败）

### Requirement: R5.3 攻击类型 × 护甲类型克制倍率矩阵（R5.3）

系统 SHALL 以「攻击类型 × 护甲类型」二维矩阵结算伤害克制（R5.3 / `WC3-MELEE-RULES.md:157`），矩阵倍率一律 `[待核]` + 合成占位、由数据表注入、MUST NOT 冻结原版真值或据参考项目硬编码值臆造（`WC3-MELEE-RULES.md:353,374-391` / `design.md` G4）。矩阵结构只作对齐登记、不冒充倍率已核实；`SimFixed` 整数运算、MUST NOT 引入 `f64`（ADR 0002 / 0004）。

#### Scenario: 二维矩阵查表且数值占位（R5.3 / ADR 0002）

- **WHEN** 在干净环境（不含 `config.toml` 与导出目录）构造冻结定义、以不同攻击 / 护甲类型组合提交伤害结算
- **THEN** 伤害按二维查表确定产出、逐帧哈希一致；每个倍率为编译期合成整数常量、`[待核]` 标注齐全、不含 MPQ 派生数据 / 原版字段名 / rawcode / 具体数值；`Grep` 无原版具体倍率

### Requirement: R7.4 回购价公式与交互距离（R7.4）

系统 SHALL 以「固定回购比例 + 交互距离」公式结算已售物品 / 单位的回购价（R7.4 / `WC3-MELEE-RULES.md:182`）：回购价显著低于购入价、交互须在定义的距离内经既有命令面与资源结算完成（R9.3）。比例 / 距离数值一律 `[待核]` + 合成占位、不冻结真值，不借鉴 `design.md` G5 登记的项目自有「半价出售」值（`WC3-MELEE-RULES.md:385`）。

#### Scenario: 回购价公式与交互距离生效（R7.4 / R9.3 / ADR 0002）

- **WHEN** 英雄在交互距离内 / 距离外分别提交回购、并核对扣费与权威状态
- **THEN** 距离内按固定回购比例扣费并返还物品实例、距离外拒单不改资源；费用经既有命令面结算、逐帧哈希一致；`Grep` 无原版具体金额 / 比例 / 距离数值

### Requirement: R8.5 野怪三级 leash 归巢判据（R8.5）

系统 SHALL 以三级（警戒 / 追击 / 归巢）可打断独立行为状态机表达野怪 leash，归巢判据由冻结的 `[待核]` 合成常量（归巢半径 / 触发阈值）驱动、MUST NOT 无限追出营地（R8.5 / `WC3-MELEE-RULES.md:194`）。半径 / 阈值数值一律 `[待核]` + 合成占位，MUST NOT 借鉴 `design.md` G5 登记的带 TODO 硬编码距离常量（`WC3-MELEE-RULES.md:369,387`）；状态机为权威运行态、按稳定 `EntityId` 顺序、进状态哈希与快照（ADR 0002）。

#### Scenario: 三级 leash 归巢且确定（R8.5 / ADR 0002）

- **WHEN** 引诱野怪越过归巢半径、推进若干整数帧并以相同脚本重开一局
- **THEN** 野怪由追击切归巢、返回 home 锚点后回警戒，MUST NOT 无限追击；状态迁移逐帧哈希一致、`capture_snapshot` / `restore_snapshot` 往返一致；`Grep` 无原版具体半径 / 阈值

### Requirement: R1 可移动性边界不因机制落地退化（R1 / R1.4）

系统 SHALL 保持 R1.4 命令可移动性过滤（`WC3-MELEE-RULES.md:98-102`）：金矿 / 树木 / 建筑 / 资源点等不可移动实体 MUST NOT 响应 `Move`、MUST NOT 改变位置；机制全量 / leash / 回购落地 MUST NOT 借新命令旁路改写该过滤。框选含建筑时 MUST 只为可移动单位生成 `Move`、落点按稳定 `EntityId` 顺序。

#### Scenario: 不可移动实体不生成 Move（R1.4 / ADR 0002）

- **WHEN** 对含「工人 + 主城 + 金矿」的选中集合下达移动、并推进结算帧
- **THEN** 仅工人生成 `Move` 命令、主城与金矿位置不变；`Grep` 命令分发处命中定义层可移动性过滤、新增机制消费者不出现对不可移动实体的 `Move` 生成路径；该回归含「改坏验红」反例（删过滤则断言失败）

### Requirement: bump 9→10 全套三版本号 / 四处同步 / 版本拒绝（ADR 0002 / ADR 0005）

系统 SHALL 把 `SNAPSHOT_VERSION` / `SIM_VERSION`（`crates/w3-core/src/snapshot.rs:67-68`）与 `PROTOCOL_VERSION`（`crates/w3-net/src/packet.rs:50`）由 9 bump 到 10；新增权威状态字段 MUST 经 `compute_state_hash` / `first_difference` / `capture_snapshot` / `restore_snapshot` 四处同步进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION` 定义的 v10 快照；golden hash 按新实现如实重取、不沿用 v9 值、不声称逐字节不变。旧 v9 快照 / 回放 MUST 按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移（ADR 0002 §7）。bump 是遵循 ADR 0002 / ADR 0005 门禁，不是放宽红线。

#### Scenario: 三版本号与四处同步、旧版本拒绝（ADR 0002 / ADR 0005）

- **WHEN** 收尾核对三版本号、加载一份旧 v9 快照 / 回放、并跑 `cargo test -p w3-core --test determinism`
- **THEN** 三版本号均为 10、`Grep pattern="= 9|: u32 = 9|: u8 = 9"` 在版本号定义处无残留；新增字段四处同步、`determinism` 全绿并经反向验证；v9 快照 / 回放被 `UnsupportedVersion` / `SimVersionMismatch` 拒绝（不改状态、不静默降级）

#### Scenario: golden hash 重取且双平台待人工（ADR 0002 / CODEBUDDY.md §5）

- **WHEN** 以 v10 新实现运行 `crates/w3-core/examples/golden_hash.rs` 并对照 v9 基线
- **THEN** 新基线如实重取、旧值登记为失效、不声称逐字节不变；`cargo test -p w3-core --test determinism` 全绿；macOS / ARM64 / 其它 rustc / Debian 逐字节比对、CI 双平台一律「待 VM / 人工」、不标通过、不声称全平台确定性已达成

### Requirement: proto 只追加 tag / payload 穷尽映射 / relay 版本拒绝 / canonical（ADR 0005 §3）

系统 SHALL 在 `w3net.proto` 只追加字段编号（当前最大 tag 63、新增从 64 起、`reserved 14 to 32` 保持、既有 tag 一字不动）、保持 `payload.rs:263/:553` 双向穷尽映射、relay 在解码 payload 前按版本拒绝；ADR 0005 §3 全部约束与 §4 红线继续适用（R12.2 / ADR 0002 / ADR 0005）。逐条明细见下 Scenario。

#### Scenario: proto 追加 tag 与 canonical 往返（ADR 0005 §3）

- **WHEN** 追加新玩法命令 tag（若规格门拍板新增 `GameplayCommandKind`）、跑 payload 编解码往返与 canonical 校验、并注入旧 v9 版本包
- **THEN** `crates/w3-net/src/proto/w3net.proto:208` 最大 tag 63 `GameplayLevelUpHero`、新增从 64 起、`:210` `reserved 14 to 32` 与既有 tag 不动、每个新 message 字段只追加且尾部保留 `reserved`；`payload.rs:263/:553` `to_pb_gameplay_kind` / `from_pb_gameplay_kind` 双向穷尽映射无遗漏、新增变体不改既有编码、canonical「解码→再编码」往返字节一致、禁非规范 varint；`Packet::decode` 在解码前按版本拒绝、relay 回 `Reject`、旧 v9 端不把新字段当 unknown field 静默丢弃（显式失败不静默降级）；ADR 0005 §3 禁 `map` / 禁 `float` · `double` / 字段只追加 · 删除保留 `reserved` / unknown fields 不透传回显 / 锁定 `prost` · `prost-build` 版本写入 `Cargo.lock` / CI 双平台生成代码字节一致（待人工 / VM）；§4 只传玩家输入命令、MUST NOT 传游戏状态、`w3-core` 不反向依赖上层

#### Scenario: 是否新增 GameplayCommandKind 属待确认项（ADR 0005 / CODEBUDDY.md §8）

- **WHEN** 规格门核对本 change 的命令面决策记录
- **THEN** 「是否新增 `GameplayCommandKind`」「proto 追加 tag 起点」在 `decision-memo.md`「待确认项」如实标注、由用户拍板、不臆造；若新增则连带 proto tag / relay 版本拒绝 / canonical 全套、不旁路门禁

### Requirement: 反向验证（改坏验红）与 determinism 重跑（CODEBUDDY.md §5 / ADR 0002）

本 change 真改 `w3-core` 确定性代码与权威状态字段集合，SHALL 新增 CPU 确定性回归断言（三类机制接线 / 买活半血无蓝 / 阈值分档 / 克制矩阵 / 回购 / leash / v10 快照往返 / 旧 v9 拒绝），MUST 重跑 `cargo test -p w3-core --test determinism` 全绿；新增 / 修改回归 MUST 证明缺陷态失败、修复态通过（测试内反例 + 元断言，或 A-B 对照 / 改坏被测代码验红并留命令输出），仅「全绿」不构成验证。

#### Scenario: 反向验证与 determinism 重跑（CODEBUDDY.md §5 / ADR 0002）

- **WHEN** 跑 `cargo test -p w3-core --test determinism -- --nocapture`，并以退化实现替换正式实现
- **THEN** 正式态全绿、缺陷态对应反例断言失败且修复后通过、两条证据可复现并留命令输出；并补齐 p17 R2 缺口——「改生产 `xp_candidates` → 正向回归变红 → 恢复」的 §5 形态② A-B 命令输出留档（`design.md` 文档订正项）

### Requirement: 数值 [待核] 合成占位与证据 / 资产不进门禁（ADR 0001 / 0002 / 0004）

所有 `[待核]` 数值 / 倍率 / 阈值 / 半径 / 比例 / 费用 / 时间 / 距离 SHALL 保持合成占位、不冻结进权威定义、不冒充经典规则事实（`WC3-MELEE-RULES.md:353,374-391` / ADR 0001 / 0004）。仓库外一手明文证据、参考项目源码锚、反编译产物 MUST NOT 进仓库、权威状态、FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照、`first_difference()`、golden hash、determinism 验收或 CI 门禁；不写本机绝对路径 / 凭证 / 原版字段名 / rawcode / 技能名 / 具体数值进权威定义。

#### Scenario: 零资产与证据隔离自证（ADR 0001 / 0002 / 0004）

- **WHEN** 收尾核对本 change 的变更边界与门禁产物
- **THEN** `git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 派生 JSON / PNG / GLB / 音效；权威定义内无本机盘符绝对路径 / 原版具体数值 / 字段名 / rawcode / 技能名；参考项目源码锚仅出现在 `design.md`「## 参考项目对照」（只读佐证、不作数值事实源）；证据不出现在任何仓库 artifact；四件套 / ADR 0003 第四次修订 / p13 追加 / `decision-memo.md` MUST NOT 引用、转述、暗示任何仓库外对内资料 / 二手转述 / 原文摘录 / 出处行号锚 / 具体细节 / 专有名词（按 `CODEBUDDY.md §6` 合规基线；受限词表不内嵌本仓库文件、以免自造命中，由 leader 用受限词表复核字面零命中）

### Requirement: 文档订正与参考缺口 / 规则文档同步 / 证据边界登记（§14 第 5 条 / p17 R1–R7）

系统（文档侧）SHALL 落实 p17 代码门交办的 R1 / R2 / R4 / R5 / R6 / R7 六项文档与测试订正的如实登记（逐项明细见下 Scenario）；玩法 change 收尾 SHALL 在 `decision-memo.md`「## 规则文档同步」给结论、并在 apply / 归档前按 §14 第 5 条更新附录「规则 → 实现覆盖对照」对应行与 §15 变更记录。参考缺口 SHALL 如实登记、回到用户拍板、不臆造（`UPSTREAM.md:122`）。MUST NOT 把「引用 `R-x.y`」冒充「规则已完整实现 / 已人工验收」（三态判定 `WC3-MELEE-RULES.md:395`）。

#### Scenario: 文档订正与规则文档同步可核验（§14 第 5 条 / p17 R1–R7）

- **WHEN** 核对 `decision-memo.md` 的文档订正登记、参考缺口清单与「## 规则文档同步」小节
- **THEN** 六项订正逐条非空——R1 订正 `design.md` D3「合成占位常量经 `CreepConstants::hash()` 进 `defs_hash` / `content_hash`」与实现不符的措辞、R2 生产结算「改 `xp_candidates` → 正向回归变红 → 恢复」A-B 验红留命令输出、R4 `verified_mechanisms.rs:683-687` 恒真断言 `assert_eq!(frozen.content_hash(), defs().content_hash())` 改固定基线比较或删除、R5 成文期快照锚失效、R6 成文期 `#### Scenario:` 字面失效与 7.4/7.5/7.6 tasks 保持未勾、R7 参考项目源码锚未实地打开（本轮已由 `reference-researcher` 补齐）；参考缺口七类逐条 `[待核]` + 合成占位、无一条写成「已核实 / 已对齐」；「## 规则文档同步」非空并给出「需同步、本轮只成文、待 apply 阶段由 `docs-owner` 按 §14 第 5 条更新附录覆盖对照 R5 / R6 / R7 / R8 对应行与 §15」的如实结论；本轮 `git diff -- docs/references/WC3-MELEE-RULES.md` 为空、真窗口 8.x 人工清单「未执行 / 待人工」

### Requirement: 本轮只成文边界与人工确认项（ADR 0001 / 0003 / CODEBUDDY.md §4）

本 change 成文期 SHALL 只写 `openspec/changes/p18-full-melee-mechanisms/` 与两份纯追加文档、MUST NOT 改 `crates/**` / 主 `openspec/specs/**` / 规则文档正文 / `tools/**` / CI / 权限配置 / p12–p17 / 归档件 / 其它 ADR 正文；不实际 bump、不重取 golden hash、不勾任一复选框、不填 `status: approved`、不写「## 省略说明」、不 self-approve、不 archive / commit / push / fetch / pull / clone。L2 反编译补 R3.2 / R3.3 upkeep 由用户本人本机执行、只登记为人工确认项、MUST NOT 写成 `sim-owner` 的 apply 任务（`docs/decisions/0003-scope.md` 第三次修订复核点 / ADR 0001）；真窗口 / 双平台一律「待人工 / 待 VM」、不标通过。

#### Scenario: 成文边界与人工项如实登记（ADR 0001 / 0003 / CODEBUDDY.md §4）

- **WHEN** 收尾核对 `git status --short` 与 `decision-memo.md` 人工项清单
- **THEN** 改动仅落本 change 目录 + `docs/decisions/0003-scope.md` 纯追加 + p13 `decision-memo.md` 纯追加；`git diff -- crates/` / `-- openspec/specs/` / `-- tools/` / `-- docs/references/` 为空、三版本号仍 9；L2 upkeep / 真窗口 8.x / golden hash 双平台 / CI 双平台均标「待人工 / 待 VM」、不标通过；「## 规格门」保持 `status: pending`
