# Tasks

> 授权边界（本阶段）：本文件是 `p8-townhall-upgrade` **提案阶段**的实施清单，**只产出四件套 + `decision-memo.md` 文档**；不 apply、不 commit、不 archive、不 self-approve、不回填规格门指纹、不改 `crates/` / 主 `openspec/specs/**` 正文 / `docs/references/WC3-MELEE-RULES.md` / ADR / `openspec/config.yaml` / `CODEBUDDY.md` / CI 配置。**规格门 `status: pending`、Q1–Q8 尚未拍板**（`decision-memo.md`「## 待确认项」逐条「推荐 + 依据 + 代价 + 备选」）；四件套按「推荐」收敛为 P1=A 单一口径一次成文、不冒充已锁定；批准记录（`status: approved` + 四件套指纹）须由用户在规格门明确拍板后由 leader 回填，**不拿沉默当授权**。锁定顺延：若拍板结果与推荐不同，MUST 按 `CODEBUDDY.md §8` 同步回查四件套 → leader 重算指纹 → `critic` 重出规格门报告 → 用户第二轮确认锁定。一个 change 一个 owner：`sim-owner`（`crates/w3-core/`）为主 owner，`net-owner` / `client-owner` 按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。
> 每条任务带可核验的完成判据（命令 / 测试 / 产物）。规则依据 `docs/references/WC3-MELEE-RULES.md` R-x.y（R4.1 只引用规则编号、不引用附录 `:137` 旧行号）；确定性硬门禁 ADR 0002 / `CODEBUDDY.md §5`；参考结论见 `design.md`「## 参考项目对照」；Q1–Q8 见 `decision-memo.md`。

## 1. 规格门前置（提案阶段收尾，本阶段执行到 1.5 止）

- [x] 1.1 按 `openspec instructions` 四件套模板一次成文 `proposal.md` / `specs/**`（新增 `town-hall-upgrade` 9 条 ADDED + 6 份 14 条 MODIFIED）/ `design.md` / `tasks.md` + `decision-memo.md`，回查 proposal 的 Scope / Capabilities ↔ specs 的 `#### Scenario:` ↔ design 的 D1–D8 alternatives /「## 参考项目对照」↔ tasks 分组步骤互不打架；验证：`openspec validate p8-townhall-upgrade --strict` 零 warning
- [x] 1.2 复核 `design.md`「## 参考项目对照」逐条完整转写 `reference-researcher` 报告的全部结论 + 证据锚（T1 / T2 / T4 / T5 命中、T3 不命中、T6 条件命中、A / B / C 分类、明确不借鉴、内核现状差距 6 项、A / B / A′ 路径代价、附录锚复核、vera20k HEAD 偏离 530 commits 如实登记、参考缺口）、`decision-memo.md` C1–C24 逐条带锚且归因声明不误读；验证：人工逐条对照本会话转达报告、`openspec/config.yaml` 的 `rules.design` 第 5 条满足
- [x] 1.3 跑 `python tools/check_encoding.py` 全绿（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格）；验证：命令输出 `Encoding check OK` 且 exit 0
- [x] 1.4 跑 `python tools/verify_all.py --fingerprint p8-townhall-upgrade` 只读重算四件套 sha256 指纹、如实记录 64 位 hex 进交付报告；验证：输出 64 位 hex；**本阶段不回填 `decision-memo.md`「## 规格门」的 `fingerprint` / `status`**（保持 `pending`），不 self-approve
- [x] 1.5 `critic` 规格门报告（清单第 7 / 10 / 12 / 13 项，尤其复核 `orc-faction` / `command-hotkeys` 正面冲突是否已消除、`## Purpose` 与其余 15 个主 specs 历史口径是否如实登记为已知限制）→ 用户规格门经 `AskUserQuestion` 逐条拍板 Q1–Q8 并批准；验证：拍板记录（本会话用户消息 / leader 转达，非仓库文件、人工核对）+ 修订后 `openspec validate p8-townhall-upgrade --strict` 零 warning + `python tools/check_encoding.py` 全绿；**若拍板与推荐不同须先同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户第二轮确认锁定；本轮 `status: pending` 不变、不回填指纹、不代替重出报告 / 确认锁定**

## 2. 定义层：tier 需求链与冻结常量（按 Q3 推荐 `FrozenDefinitions` 冻结常量，单一路径）

- [x] 2.1 按 Q3 推荐把升级费用（gold / lumber）/ 时间（整数帧）/ 差价 /「tier → 解锁二线单位与科技需求链」落到 `FrozenDefinitions` 冻结常量（client 侧合成编译期常量表为 Q3 被否备选、不实现），全为合成原创编译期整数常量 `[待核]`、不含 `f64` / MPQ 派生 / 真实导出 JSON / 参考项目单位名或 rawcode；验证：干净环境（无 `config.toml` / 导出目录）构造成功、零文件 IO、`cargo test -p w3-core` 相关用例绿（R1.2 / R4.1 / R13.3 / ADR 0001 / ADR 0004 / §14；`w3-core` MUST NOT 依赖 `w3-client`，`CODEBUDDY.md §5`）
- [x] 2.2 把 tier 需求链冻结常量段接入 `compute_defs_hash`（`gameplay.rs:268-316`）→ `defs_hash` / `content_hash` 进快照头部（`snapshot.rs:79-87`、`sim_match.rs:1800-1897`）；验证：改常量即改变 `content_hash()`、漏接任一段的退化实现被断言拦截（主 `gameplay-systems/spec.md:8,12-15`），与 `specs/town-hall-upgrade/spec.md`「定义层 tier 需求链与冻结常量数据驱动」Scenario 口径一致
- [x] 2.3 主城升级可用性与二线单位 / 科技解锁判据由定义层类别与 requires / upgrade 链数据驱动（按 tier 取需求），MUST NOT 用建筑名字符串白名单或硬编码 DefId；验证：以非经典命名的主城 / 二线单位 DefId 仍按冻结需求链判定、以白名单驱动的退化实现被回归拦截（R1.2 / R13.3 / D2 / C13 / C15）
- [x] 2.4 明确区分 tier **顶层权威状态**（进哈希 / 快照 / `first_difference()`）与 tier **需求链冻结常量**（定义层内容变化、经 `defs_hash` / `content_hash` 进快照头部）；验证：代码审查 + `specs/town-hall-upgrade/spec.md` 两处表述一致、无把需求链误当权威状态字段的退化实现（ADR 0002 §6-8 / D1 / D2）

## 3. 内核权威状态与哈希 / 快照 / `first_difference()`

- [x] 3.1 在 `w3-core` 新增「主城等级 / tier」顶层权威状态（离散 T1/T2/T3、按玩家主城实体）、升级队列项、费用登记三组权威状态，全部落 `BTreeMap` 等确定性有序容器、按稳定 `EntityId` / `PlayerId` 顺序遍历；独立字段、MUST NOT 复用 `PlayerEconomy.upgrades: BTreeMap<DefId, bool>` 布尔科技旗标、MUST NOT 用墙钟 / 渲染帧率 / 全局静态字典 / `HashMap` 迭代顺序；验证：`cargo test -p w3-core --lib` 绿、代码审查无 `HashMap` / `SystemTime` / `Instant` / `f64` 进权威状态（ADR 0002 §2-3 / D1 / R4.1 / C5）
- [x] 3.2 把三组新增权威状态同步接入 FNV-1a 状态哈希（`sim_match.rs:1680-1740` hash 段）、由 `SNAPSHOT_VERSION` 定义的快照（`snapshot.rs`）与 `first_difference()` 差异字段集合（`sim_match.rs:1742-1798`）；验证：双实例交错逐帧 `compute_state_hash()` 相等、帧 N 捕获 / 销毁 / 恢复后继续到帧 M 逐帧一致、`first_difference()` 能定位到新字段（ADR 0002 §6-8 / `CODEBUDDY.md §5` / C1 / D5）
- [x] 3.3 按 Q1 推荐 bump **4→5** 处置 `SNAPSHOT_VERSION`：连同 `SIM_VERSION`、`snapshot.rs:20-21` 常量、`sim_match.rs:2368-2369` 单测硬断言、快照头部校验、golden hash 基线、ADR 0002 复核记录追加（不删改 §1–§8 正文）、路线图 :138 ADR 复核同步更新；验证：`cargo test -p w3-core --lib` 绿、版本不匹配的旧 v4 快照 / 回放被 `SnapshotError` / `Reject` 明确拒绝（ADR 0002 §7 / ADR 0005 §3 / D5）
- [x] 3.4 重开 / 快照恢复后不残留 tier / 升级队列 / 费用登记的反例元断言回归：在测试内构造退化实现（结算后重开仍残留上一局 tier / 队列 / 费用字段，或帧 N 捕获快照恢复后重复升级）并加元断言，证明缺陷态断言失败、修复态通过；验证：新一局 `victory.resolved == false`、tier 回到 T1、升级登记为空、逐帧 `compute_state_hash()` 与同脚本基线一致，缺陷态红 / 修复态绿两条命令输出留档（R11.5 / R12.4 / ADR 0002 §7 / `CODEBUDDY.md §5` 反向验证 / Q8 / C23）

## 4. 命令与事件面（按 Q2 推荐新增独立 `UpgradeTownHall` 命令 + 升级事件变体，单一路径）

- [x] 4.1 按 Q2 推荐新增独立 `GameplayCommandKind::UpgradeTownHall`（指定主城实体 + 目标 tier）命令变体 + 升级开始 / 完成 / 取消事实事件变体（不复用 `Research` + 扩展 `UpgradeDef` 的 Q2 被否备选），经公开提交入口按冻结整数帧号入队、同源校验、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；验证：逐帧提交 / 一次推进 8 帧两端逐帧哈希一致、重放不双重注入（ADR 0002 §2-3 / R12.4 / R12.7 / D3 / C3）
- [x] 4.2 同步 `hash_gameplay_command` 编码（追加码 13，`gameplay.rs:932-1011`）/ 校验（`:1013-1037`）/ 穷尽分发（`sim_match.rs:1207-1239`）/ `EventKind` 覆盖 `UpgradeTownHall` 与升级事件；验证：`cargo test -p w3-core --lib` 绿、穷尽匹配无遗漏分支、命令哈希编码唯一（C3 / D3）
- [x] 4.3 按 Q2 推荐新增变体牵动 ADR 0005 protobuf（条件命中矩阵 T6，`net-owner` 按 `CODEBUDDY.md §9` 与 `sim-owner` 串行协作）：字段编号只追加（现 tag 33/34 之后）、保留 `reserved 14 to 32`、bump `PROTOCOL_VERSION`、`Reject` 明确拒绝旧版本、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、按 ADR 0005 自有 `W3N1` + protobuf 契约执行不借鉴 `vera20k` 线格式；验证：`cargo test -p w3-net` 绿、CI Windows / Linux 双平台生成代码字节一致、双客户端逐帧哈希一致（`docs/decisions/0005-protobuf-payload.md:34-44,84` / `UPSTREAM.md:88` / C21 / D3）

## 5. 升级与解锁费用事务

- [x] 5.1 在 `enqueue_production` / `research` 增加冻结 tier 需求链前置校验：要求 T2 / T3 的二线单位 / 科技在当前 tier 未达时显式拒绝、不生成队列项、不扣资源、不改变权威状态；升级完成后解锁同一命令面立即生效；验证：未升级 `Train` / `Research` 被拒 + 升级完成后被接受两组回归绿、逐帧哈希与同脚本基线一致、`w3-core` 仍接受未解锁命令的「假升级」退化实现被断言拦截（R4.1 / R4.3 / R4.4 / 路线图 :126 / D4 / Q8）
- [x] 5.2 升级扣费提交与队列效果执行分离：提交前校验 gold / lumber 充足、从 `PlayerEconomy` 确定性扣减、冻结扣费登记；验证：费用提交 / 效果执行两阶段边界测试绿、资源不足显式拒绝不生成队列不扣费（主 `gameplay-systems/spec.md:19,23-26` / R3.1 / R4.1 / D4）
- [x] 5.3 取消队列 / 主城被毁 / 对局失败时事务性退款 + 恢复升级登记到可再次升级、不生成半升级实体、不重复扣费；退款与扣费同样进状态哈希 / 快照；验证：四条中止路径 `gold / lumber` 精确回退、逐帧哈希与同脚本基线一致（R3.1 / D4 / C1）；升级费用 / 时间 / 差价全为合成原创 `[待核]` 常量、MUST NOT 把导出总价直接当升级扣款（`CLASSIC_RULES_BASELINE.md:46` / §14 / ADR 0001）

## 6. 表现层 HUD 只读派生（复用 P8.5a `ReviveHudModel` 范式）

- [x] 6.1 主城升级进度 / 费用 / 剩余帧与 `U` 键、命令卡升级格位只从只读 `WorldView`（必要时只读投影字段）/ 事实事件 / 只读 `Snapshot` 派生，MUST NOT 反写 `RtsMatch` 权威状态 / 进状态哈希 / 快照 / `first_difference()`；只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单；验证：`cargo test -p w3-render` / `crates/w3-render/tests/isolation.rs` 绿、HUD 连续重绘时逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与无 HUD 实例一致（ADR 0002 §5 / R13 / R13.2 / R13.3 / R13.7 / C6 / D6）
- [x] 6.2 升级格位 / 键位由定义层数据驱动、具体绑定 `[待核]` 不冻结进权威定义、不入库；UI 文案走 i18n、不进日志 / 权威状态；日志 / 错误英文结构化 `key=value`、源码注释中文；验证：`python tools/check_source_conventions.py` 全绿、以非经典命名 DefId 仍按定义链显示格位（R1.2 / R13.3 / §14 / `CODEBUDDY.md §3` / ADR 0002 §5）

## 7. 回归、反向验证（改坏验红）与确定性门禁

- [x] 7.1 新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照），退化实现真经生产路径注入、非旁路模拟；至少覆盖 Q8 四类退化：① tier 权威状态漏接 `compute_state_hash()` 或 `capture_snapshot()`；② 未升级仍接受未解锁 `Train` / `Research` 的「假升级」；③ 升级费用不扣 gold / 取消不退款，或升级进度用墙钟 / `HashMap` 迭代顺序；④ 升级命令漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发；验证：每组「缺陷态红 / 修复态绿」两条命令输出留档（`CODEBUDDY.md §5` / C23 / Q8 / D8；先例 `p8-dual-faction` 成对 `reverse_validation_*` / `p8-hero-revive` 7.1）
- [x] 7.2 涉及 `w3-core` 权威状态 / 帧序 / 命令排序 / 状态哈希 / 快照字段的改动 MUST 重跑 `cargo test -p w3-core --test determinism`（26 项）全绿；验证：`cargo test -p w3-core --test determinism -- --nocapture` 26 项全绿、输出留档（`CODEBUDDY.md §5` / `openspec/config.yaml` `rules.tasks` 第二、三条 / C1）
- [x] 7.3 golden hash 基线按新实现如实更新：以新增权威状态后的实现重取逐帧基线、不沿用 P8.5a v4 值 `0xada6506e1da36c11`、不声称逐字节不变；按 Q1 推荐 bump 5，Windows 侧离线 golden hash（Windows / Debian 12 x86_64）逐字节比对基线重取留档；验证：新基线文件 / diff 输出留档，**MUST NOT 声称 Debian 12 侧 v4 / v5 逐字节比对已完成、MUST NOT 声称 macOS / ARM64 / 其它 rustc 已验证**（R12.4 / ADR 0002 复核记录 / Q7 / C7 / D5；Debian 侧比对登记为人工 / VM 待办）
- [x] 7.4 跑 `cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check` 全绿；验证：命令输出全绿（`CODEBUDDY.md §2 / §7`）
- [x] 7.5 主 specs 6 份 MODIFIED delta 归档前复核：逐字保留全部历史 `#### Scenario:` 身份键、口径改在 THEN、requirement 正文给完整新正文、每条带 `>` 身份键说明，`orc-faction` / `command-hotkeys` 正面冲突在规范性层面消除；验证：`openspec validate p8-townhall-upgrade --strict` 零 warning、`critic` 清单第 7 / 10 项复核 `blocker: 0`（B-1 教训）

## 8. 真窗口人工验收清单（按 Q4 推荐：范围外承接登记为后续 change 待办；真窗口项未执行 / 待人工，不提供替代、不冒充通过）

> 本组为人工桌面验收项，CLI / 离屏自证无法覆盖 egui overlay 与真实 GPU 画面（主 `playable-window-entry/spec.md:168-176` / 路线图 :117 / C23）。**全部保持 `- [ ]` 与「未执行 / 待人工」，先离屏自证 + CPU 回归全绿再请人工验收；清单未逐项勾选前不得归档；CI MUST NOT 声称已验证 GPU**。GUI 类修复先自测再请人工验收（不反复试错）。

- [ ] 8.1 真窗口启动 `melee` 可玩入口，选中主城按 `U` / 命令卡升级格位，肉眼确认升级队列出现、进度随时间推进、费用 / 剩余帧可见 —— 状态：未执行 / 待人工
- [ ] 8.2 升级完成后肉眼确认：主城等级变化、原 T1 被拒的二线单位 / 科技训练格位解锁可用、解锁训练出的单位按冻结定义生效、HUD 选择面板可见 —— 状态：未执行 / 待人工（R4.1 / R4.3 / R4.4）
- [ ] 8.3 中途取消升级队列，肉眼确认费用回退、队列消失、可再次发起升级；肉眼确认资源不足时按钮 / 命令显式失败提示、不免费升级；未升级时二线单位格位不可训练 —— 状态：未执行 / 待人工（R3.1 / R4.1）
- [ ] 8.4 升级期间肉眼确认对局未结束、AI 对手继续行动，升级后能继续打完一局并看到 `VICTORY` —— 状态：未执行 / 待人工（R11 / R11.4；AI × lockstep 升级命令注入架构按 §0.3 T5 留后续 change）
- [ ] 8.5 右键 / 键盘 `Move` 含主城 / 祭坛 / 金矿 / 树木的选中集合时，肉眼确认只有可移动单位移动、建筑 / 资源点不动（回归 R1.4 / R13.6，防止本 change 破坏既有过滤；主城实体升级后仍 MUST NOT 响应 `Move`）—— 状态：未执行 / 待人工
- [ ] 8.6 `Resized` 窗口缩放 / 高刷 30 Hz 下 HUD 升级进度与逐帧确定性无异常 —— 状态：未执行 / 待人工（无证据项如实标注，不冒充通过）
- [ ] 8.7 清单项保持 `- [ ]` 与「未执行 / 待人工」、无证据项如实标注、不用离屏截图冒充真窗口验收、CI 边界如实说明 —— 状态：未执行 / 待人工（C23 / Q4 先例）

## Workflow follow-up

- 规格门：`critic` 规格门报告 → 用户规格门经 `AskUserQuestion` 逐条拍板 Q1–Q8 并批准 → **leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`** → `/opsx:apply`（`sim-owner` 为主 owner，`net-owner` / `client-owner` 按 §9 串行协作）→ `critic` 代码门 → 本地 commit（中文描述 + Conventional Commits 英文 type 前缀）→ 人工 push → CI 双平台 `verify_all.py` → 真窗口清单逐项勾选 → `/opsx:archive` 并确认主 specs 合并（新增 `town-hall-upgrade` + 6 份 14 条 MODIFIED）。**本轮 `status: pending` 不变、不回填指纹、不 self-approve、不拿沉默当授权**；若拍板与推荐不同须「同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户第二轮确认锁定」全流程。
- 已知限制承接（后续独立 change，不改本 change 四件套）：所有已存在 spec 的 `## Purpose` 概述性文字（实测 OpenSpec 1.17.0 delta 不合并，`docs-owner` 收口，见 `design.md`「## 已知限制 / 边界」R-1）；其余 15 个含同类历史口径的主 specs 统一收口（`docs-owner`，是否本 change 一并全量修订属 Q5 待拍板）；`WC3-MELEE-RULES.md` 附录 R4.1 机制锚 / 防误用补锚 / `R4.1 :137` 旧行号差异（按 Q5 推荐，`docs-owner`）；T3 全量升级链 / 两族完整二线单位与科技清单 / 差价扣费 / 命令卡第二键与 `U` 绑定 B 类 `[待核]`（按 Q4 推荐）；AI × lockstep 升级命令注入架构（按 Q6 推荐 / §0.3 T5）；Debian 12 侧 v4 / v5 golden hash 逐字节离线比对（Q7，人工 / VM）；跨平台确定性 macOS / ARM64 / 其它 rustc 待验证。
- 归档时逐条登记未勾任务原因、已知限制承接、主 `openspec/specs/` 是否变化据实说明；`status: pending` 令 `tools/verify_all.py::spec_gate_check()` 变红是 fail-closed 正确信号（C22），本阶段不绕过 / 不改门禁；`python tools/verify_all.py --fingerprint p8-townhall-upgrade` 只读重算指纹如实记录、不回填门禁。
- 人工边界（非待拍板，仅登记）：本地 `git add` / `commit` 由 AI（中文描述 + Conventional Commits 英文 type 前缀），四件套 + `decision-memo.md` MUST 一并纳入 commit；`push` / `fetch` / `pull` / `clone` 及任何远端同步、CI、真窗口人工验收、`/opsx:archive` 归档全部人工独占且本轮未执行。
