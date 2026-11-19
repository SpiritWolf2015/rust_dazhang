# Tasks

> 授权边界（本阶段）：本文件是 `p8-hero-revive` 提案阶段的实施清单，**只产出四件套 + `decision-memo.md` 文档**；不 apply、不 commit、不 archive、不 self-approve、不回填规格门指纹、不改 `crates/` / 主 `openspec/specs/**` / `docs/references/WC3-MELEE-RULES.md` / ADR。Q1–Q6 已于 2026-11-19 规格门经 `AskUserQuestion` 逐条拍板「全部按推荐」（`decision-memo.md`「## 拍板决策」P2 汇总 +「## 待确认项」逐条拍板结果留痕）；因 B-1 选「增补 delta 彻底消除」触发四件套同步修订，规格门批准锁定顺延至「修订四件套 → leader 重算指纹 → `critic` 重出规格门报告 → 用户第二轮确认锁定」，本轮规格门 `status: pending` 不变、不回填 `approved` / 指纹；锁定后由 leader 按 `CODEBUDDY.md §9` 编排 `/opsx:apply`。本清单按已拍板结果收敛为单一路径（Q1 bump 4 / Q2 新增独立 `ReviveHero` / `CancelRevive` 命令 + 事件变体 / Q3 `FrozenDefinitions`），不再两分支并行列出。一个 change 一个 owner：`sim-owner`（`crates/w3-core/`）为主 owner，`client-owner` / `net-owner` 串行协作、不并行改同一 owner 文件。
> 每条任务带可核验的完成判据（命令 / 测试 / 产物）。规则依据 `docs/references/WC3-MELEE-RULES.md` R-x.y；确定性硬门禁 ADR 0002 / `CODEBUDDY.md §5`；参考结论见 `design.md`「## 参考项目对照」。

## 1. 规格门前置（提案阶段收尾，本阶段执行到 1.5 止）

- [x] 1.1 按 `openspec instructions` 四件套模板一次成文 `proposal.md` / `specs/**` / `design.md` / `tasks.md` + `decision-memo.md`，回查 proposal 的 Scope ↔ specs 的 `#### Scenario:` ↔ design 的 D1–D8 alternatives /「## 参考项目对照」↔ tasks 分组步骤互不打架；验证：`openspec validate p8-hero-revive --strict` 零 warning
- [x] 1.2 复核 `design.md`「## 参考项目对照」逐条转写 `reference-researcher` 报告的全部结论 + 证据锚（T1 / T2 / T4 / T5 命中、A/B/C 分类、明确不借鉴、参考缺口）、`decision-memo.md` C1–C27 逐条带锚且归因声明不误读；验证：人工逐条对照本会话转达报告，`openspec/config.yaml` 的 `rules.design` 第 5 条满足
- [x] 1.3 跑 `python tools/check_encoding.py` 全绿（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格）；验证：命令输出 `OK` 且 exit 0
- [x] 1.4 跑 `python tools/verify_all.py --fingerprint p8-hero-revive` 只读重算四件套 sha256 指纹、如实记录进交付报告；验证：输出 64 位 hex；**本阶段不回填 `decision-memo.md`「## 规格门」的 `fingerprint` / `status`**（保持 `pending`），不 self-approve
- [x] 1.5 `critic` 规格门报告（清单第 7 / 10 / 12 / 13 项，已出、blocker B-1）→ 用户 2026-11-19 规格门经 `AskUserQuestion` 逐条拍板 Q1–Q6「全部按推荐」并批准（拍板记录：本会话用户消息 / leader 转达，非仓库文件、人工核对）；因 B-1 选「增补 delta 彻底消除」须**先同步修订四件套（proposal / specs / design / tasks + decision-memo）→ leader 重算指纹 → `critic` 重出规格门报告 → 用户第二轮确认锁定**；验证：修订后 `openspec validate p8-hero-revive --strict` 零 warning + `python tools/check_encoding.py` 全绿，第二轮确认后由 leader 回填 `status: approved` + 新指纹再 `python tools/verify_all.py` → `Verification OK`、`spec gate OK`。**本轮只修订四件套，不代替重出报告 / 确认锁定、`status: pending` 不变**

## 2. 定义层：复活冻结常量与类别驱动（按 Q3 拍板 `FrozenDefinitions` 冻结常量，单一路径）

- [x] 2.1 按 Q3 拍板把复活费用（gold / lumber）/ 时间（整数帧）/「基数 + 等级系数 → 绝对上限」常量落到 `FrozenDefinitions` 冻结常量（client 侧合成编译期常量表为 Q3 被否备选、不实现），全为合成原创编译期整数常量 `[待核]`、不含 `f64` / MPQ 派生 / 真实导出 JSON / 参考项目单位名或 rawcode；验证：干净环境（无 `config.toml` / 导出目录）构造成功、零文件 IO、`cargo test -p w3-core` 相关用例绿（R2 / R6.3 / ADR 0001 / ADR 0004 / §14；`w3-core` MUST NOT 依赖 `w3-client`，`CODEBUDDY.md §5`）
- [x] 2.2 把复活冻结常量段接入 `compute_defs_hash`（`gameplay.rs:144-181`）→ `defs_hash` / `content_hash` 进快照头部（`snapshot.rs:79-81`、`sim_match.rs:1572,1664-1669`）；验证：改常量即改变 `content_hash()`，漏接任一段的退化实现被断言拦截（主 `gameplay-systems/spec.md:8,12-15`），与 `specs/hero-revive/spec.md`「费用随等级确定且进定义哈希」Scenario 口径一致（critic R-2）
- [x] 2.3 祭坛可用性由定义层类别 `CATEGORY_ALTAR`（`defs.rs:42-54`）驱动，MUST NOT 用建筑 id 字符串白名单；验证：以非 `"halt"` 命名的祭坛 DefId 仍可复活、以建筑名字符串硬编码的退化实现被回归拦截（R1.2 / R13.3 / D2）

## 3. 内核权威状态与哈希 / 快照 / `first_difference()`

- [x] 3.1 在 `w3-core` 新增死亡登记（`death_frame: u64` + 稳定 `EntityId` 键 + 英雄状态 / 背包 / 技能等级快照）、复活队列项（目标英雄 `EntityId` / `remaining_frames` / 费用）、费用三组权威状态，全部落 `BTreeMap` 确定性有序容器、按稳定 `EntityId` 顺序遍历；MUST NOT 用墙钟 / 渲染帧率 / 全局静态字典 / `HashMap` 迭代顺序；验证：`cargo test -p w3-core --lib` 绿、代码审查无 `HashMap` / `SystemTime` / `Instant`（ADR 0002 §2-3 / D2 / R6.3 / R11.2）
- [x] 3.2 把三组新增权威状态同步接入 FNV-1a 状态哈希（`gameplay.rs` hash 段）、由 `SNAPSHOT_VERSION`（按 Q1 拍板 bump 4）定义的快照（`snapshot.rs`）与 `first_difference()` 差异字段集合；验证：双实例交错逐帧 `compute_state_hash()` 相等、帧 N 捕获 / 销毁 / 恢复后继续到帧 M 逐帧一致、`first_difference()` 能定位到新字段（ADR 0002 §6-8 / `CODEBUDDY.md §5` / C1）
- [x] 3.3 按 Q1 拍板 bump 4 处置 `SNAPSHOT_VERSION`：连同 `SIM_VERSION`、`snapshot.rs:8-16` 常量、`sim_match.rs:2127-2132` 单测硬断言、快照头部校验、golden hash 基线、ADR 0002 复核记录追加（不删改 §1–§8 正文）、路线图 :138 ADR 复核同步更新；验证：`cargo test -p w3-core --lib` 绿、版本不匹配的旧 v3 快照 / 回放被 `SnapshotError` / `Reject` 明确拒绝（ADR 0002 §7 / ADR 0005 §3）
- [x] 3.4 实现「复活队列完成时恢复同一个英雄实体」：复用死亡实体（`alive=false → true`）、同一 `EntityId`、`hero.level` / `hero.xp` / 技能等级 / 6 格背包逐项保留、按冻结常量恢复生命 / 法力、改到祭坛位置；MUST NOT `entities.spawn()` 新实体、MUST NOT `hero=None` 空背包；验证：等级 L / 经验 X / 满背包英雄阵亡复活后 `EntityId` 与状态逐项一致、新增 `EntityId` 数为 0（D1 / R6.1 / R6.2 / R6.3 / R6.4 / R11.2 / C7 / C8）
- [x] 3.5 英雄死亡只清订单 / 路径 + 发死亡事件、不删实体、不判负（沿用 `apply_effects` :955-972 / `resolve_victory` :1368-1386 现状语义）；验证：一方英雄阵亡但 `player.active` 仍真时 `victory.resolved == false`、可经祭坛复活（R11.2）
- [x] 3.6 重开 / 快照恢复后不残留复活登记的反例元断言回归（critic R-6）：在测试内构造退化实现（结算后重开仍残留上一局死亡登记 / 复活队列 / 费用字段，或帧 N 捕获快照恢复后重复复活）并加元断言，证明缺陷态断言失败、修复态通过；验证：新一局 `victory.resolved == false`、复活登记 / 队列为空、逐帧 `compute_state_hash()` 与同脚本基线一致，缺陷态红 / 修复态绿两条命令输出留档（R11.5 / R12.4 / ADR 0002 §7 / `CODEBUDDY.md §5` 反向验证 / C27）

## 4. 命令与事件面（按 Q2 拍板新增独立 `ReviveHero` / `CancelRevive` 命令 + 事件变体，单一路径）

- [x] 4.1 按 Q2 拍板新增独立 `GameplayCommandKind::ReviveHero` / `CancelRevive` 命令变体 + `HeroDied` / `Revived` 事实事件变体（不复用 `Train` + 扩展 `ProductionItem` 的 Q2 被否备选），经公开提交入口按冻结整数帧号入队、同源校验、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；验证：逐帧提交 / 一次推进 8 帧两端逐帧哈希一致、重放不双重注入（ADR 0002 §2-3 / R12.4 / R12.7 / D6）
- [x] 4.2 同步 `hash_gameplay_command` 编码（`gameplay.rs:638-697`）/ 校验（:701-718）/ 穷尽分发（`sim_match.rs:1180-1206`）/`EventKind`（`sim_match.rs:162-169`）覆盖 `ReviveHero` / `CancelRevive` / `HeroDied` / `Revived`；验证：`cargo test -p w3-core --lib` 绿、穷尽匹配无遗漏分支、命令哈希编码唯一（C3 / C4）
- [x] 4.3 按 Q2 拍板新增变体牵动 ADR 0005 protobuf（critic R-3 条件命中 T6，`net-owner` 按 `CODEBUDDY.md §9` 与 `sim-owner` 串行协作）：字段编号只追加、保留 `reserved 14 to 32`、bump `PROTOCOL_VERSION`、`Reject` 明确拒绝旧版本、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、按 ADR 0005 自有 `W3N1` + protobuf 契约执行不借鉴 `vera20k` 线格式；验证：`cargo test -p w3-net` 绿、CI Windows / Linux 双平台生成代码字节一致、双客户端逐帧哈希一致（`docs/decisions/0005-protobuf-payload.md:34-44,84` / `UPSTREAM.md:88` / C25）

## 5. 费用事务与取消退款

- [x] 5.1 复活扣费提交与队列效果执行分离：提交前校验 gold / lumber / food 充足、从 `PlayerEconomy.gold / lumber`（`gameplay.rs:468-469`）确定性扣减、冻结扣费登记；验证：费用提交 / 效果执行两阶段边界测试绿、资源不足显式拒绝不生成队列不扣费（主 `gameplay-systems/spec.md:19,23-26` / R3.1 / R6.3 / D4）
- [x] 5.2 取消队列 / 英雄再次死亡 / 祭坛被毁 / 对局失败时事务性退款 + 恢复死亡登记到「可再次复活」、不生成半复活实体、不重复扣费；退款与扣费同样进状态哈希 / 快照；验证：四条中止路径 `gold / lumber` 精确回退、逐帧哈希与同脚本基线一致（R6.3 / D4 / C18）
- [x] 5.3 费用随等级按冻结常量确定计算、祭坛与酒馆参数 / 命令分离（酒馆买活按 Q4 拍板留后续独立 change、本期不纳入）；验证：等级 1 与等级 L 费用 / 时间确定变化、逐帧哈希一致、无写死数值（R6.1 / R6.3 / R9.3 / §14 / D3）

## 6. 表现层 HUD 队列只读派生

- [x] 6.1 祭坛复活队列进度 / 费用 / 剩余帧 HUD 只从只读 `WorldView`（必要时只读投影字段）/ 事实事件 / 只读 `Snapshot` 派生，MUST NOT 反写 `RtsMatch` 权威状态 / 进状态哈希 / 快照 / `first_difference()`；验证：`cargo test -p w3-render` / `crates/w3-render/tests/isolation.rs` 绿、HUD 连续重绘时逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与无 HUD 实例一致（ADR 0002 §5 / R10.5 / C9 / D7）
- [x] 6.2 UI 文案走 i18n、不进日志 / 权威状态；日志 / 错误英文结构化 `key=value`；源码注释中文；验证：`python tools/check_source_conventions.py` 全绿（`CODEBUDDY.md §3` / ADR 0002 §5）

## 7. 回归、反向验证（改坏验红）与确定性门禁

- [x] 7.1 新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照），退化实现真经生产路径注入、非旁路模拟；至少覆盖四类退化：① 复活 `spawn` 新实体（丢本体 / 空背包）；② 死亡登记用墙钟 / `HashMap` / 全局静态字典；③ 新增权威状态漏接哈希或快照；④ 扣费不减 gold / 取消不退款；验证：每组「缺陷态红 / 修复态绿」两条命令输出留档（`CODEBUDDY.md §5` / C27 / D8；先例 `p8-dual-faction` 成对 `reverse_validation_*` / `p8-faction-foundation` `tests/movability.rs::degenerate_filter`）
- [x] 7.2 涉及 `w3-core` 权威状态 / 帧序 / 命令排序 / 状态哈希 / 快照字段的改动 MUST 重跑 `cargo test -p w3-core --test determinism`（26 项）全绿；验证：`cargo test -p w3-core --test determinism -- --nocapture` 26 项全绿、输出留档（`CODEBUDDY.md §5` / `openspec/config.yaml` `rules.tasks` 第二、三条 / C1）
- [x] 7.3 golden hash 基线按新实现如实更新：以新增权威状态后的实现重取逐帧基线、不沿用旧值、不声称逐字节不变；按 Q1 拍板 bump 4（快照版本确定变更），离线 golden hash（Windows / Debian 12 x86_64）逐字节比对基线重取、人工比对；验证：新基线文件 / diff 输出留档，**MUST NOT 声称 macOS / ARM64 / 其它 rustc 已验证**（R12.4 / ADR 0002 复核记录 / C10 / D5）
- [x] 7.4 跑 `cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check` 全绿；验证：命令输出全绿（`CODEBUDDY.md §2 / §7`）
- [x] 7.5 `critic` 代码门全清单复核（确定性 / 所有权 / 许可证 / 编码 / 资产入库 / ADR 0002 全流程 / 反向验证 / 四件套一致性）；验证：`critic` 报告 blocker 0（leader 转达，非仓库文件人工核对）；本地 commit 由 AI 执行（中文描述 + Conventional Commits 英文 type 前缀）、`python tools/verify_all.py` → `Verification OK`；**push / fetch / pull / clone 人工独占**（`CODEBUDDY.md §4 / §10`）

## 8. 真窗口人工验收清单（按 Q4 拍板：范围外承接登记为后续 change 待办；真窗口项未执行 / 待人工，不提供替代、不冒充通过）

> 本组为人工桌面验收项，CLI / 离屏自证无法覆盖 egui overlay 与真实 GPU 画面（主 `playable-window-entry/spec.md:168-176` / 路线图 :117 / C27）。**全部保持 `- [ ]` 与「未执行 / 待人工」，先离屏自证 + CPU 回归全绿再请人工验收；清单未逐项勾选前不得归档；CI MUST NOT 声称已验证 GPU**。

- [ ] 8.1 真窗口启动 `melee` 可玩入口，制造英雄阵亡后点击本族祭坛复活按钮，肉眼确认复活队列出现、进度随时间推进、费用 / 剩余帧可见 —— 状态：未执行 / 待人工
- [ ] 8.2 复活完成后肉眼确认：英雄回到祭坛、血 / 蓝恢复、等级 / 经验 / 技能等级 / 6 格背包与阵亡前一致、HUD 选择面板可见英雄本体（非新单位 / 非四面体占位）—— 状态：未执行 / 待人工
- [ ] 8.3 中途取消复活队列，肉眼确认费用回退、队列消失、可再次发起复活；肉眼确认资源不足时按钮 / 命令显式失败提示、不免费复活 —— 状态：未执行 / 待人工
- [ ] 8.4 英雄阵亡期间肉眼确认对局未结束、AI 对手继续行动，复活后能继续打完一局并看到 `VICTORY` —— 状态：未执行 / 待人工（R11.2 / R11.4）
- [ ] 8.5 右键 / 键盘 `Move` 含祭坛 / 金矿 / 树木 / 主城的选中集合时，肉眼确认只有可移动单位移动、建筑 / 资源点不动（回归 R1.4 / R13.6，防止本 change 破坏既有过滤）—— 状态：未执行 / 待人工
- [ ] 8.6 `Resized` 窗口缩放 / 高刷 30 Hz 下 HUD 队列进度与逐帧确定性无异常 —— 状态：未执行 / 待人工（无证据项如实标注，不冒充通过）
- [ ] 8.7 清单项保持 `- [ ]` 与「未执行 / 待人工」、无证据项如实标注、不用离屏截图冒充真窗口验收、CI 边界如实说明 —— 状态：未执行 / 待人工（C27 / Q4 先例）

## Workflow follow-up

- 规格门：`critic` 规格门报告（已出、blocker B-1）→ 用户 2026-11-19 规格门经 `AskUserQuestion` 逐条拍板 Q1–Q6「全部按推荐」并批准（B-1 处置 = 增补 delta 彻底消除）→ **同步修订四件套 → leader 重算指纹 → `critic` 重出规格门报告 → 用户第二轮确认锁定 → leader 回填 `status: approved` + 新四件套指纹** → `/opsx:apply`（`sim-owner` 为主 owner，`net-owner` / `client-owner` 按 §9 串行协作）→ `critic` 代码门 → 本地 commit（中文描述）→ 人工 push → CI 双平台 `verify_all.py` → 真窗口清单逐项勾选 → `/opsx:archive` 并确认主 specs 合并（新增 `hero-revive` + 4 份 MODIFIED，其中 `human-faction` 4 条）。本轮 `status: pending` 不变、不回填指纹。
- 已知限制承接（后续独立 change，不改本 change 四件套）：`melee-match:6` Purpose 下 blockquote 历史注记（实测 OpenSpec 1.17.0 delta 机制不合并 `## Purpose`，`docs-owner` 收口，见 `design.md`「## 已知限制 / 边界」）；其余 15 个主 specs 同类历史口径统一收口（`docs-owner`）；`WC3-MELEE-RULES.md` 附录 R6.5 降级 / R6.3 补锚 / 防误用补两个新锚（按 Q5 拍板，`docs-owner`）；酒馆买活 / 多英雄复活队列 UI / 复活后技能点装备态一致性（按 Q4 拍板）；AI × lockstep 复活命令注入架构（按 Q6 拍板 / §0.3 T5）；跨平台确定性 macOS / ARM64 / 其它 rustc 待验证。
- 归档时逐条登记未勾任务原因、已知限制承接、主 `openspec/specs/` 是否变化据实说明。
