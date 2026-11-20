# Tasks

> 授权边界（本阶段）：本文件是 `p8-creep-camps` **提案阶段**的实施清单，**只产出四件套 + `decision-memo.md` 文档**；不 apply、不 commit、不 archive、不 self-approve、不回填规格门指纹、不改 `crates/` / 主 `openspec/specs/**` 正文 / `docs/references/WC3-MELEE-RULES.md` / ADR / `docs/decisions/` / `openspec/config.yaml` / `CODEBUDDY.md` / CI 配置。**规格门 `status: pending`、Q1–Q12 尚未拍板、方向 A / B / A′ 未拍板**（`decision-memo.md` P1 记为「推荐 A，待规格门 Q1 由用户拍板」，MUST NOT 写成「用户已确认 / 已拍板」；「## 待确认项」逐条「推荐 + 依据 + 代价 + 备选」）；四件套按「推荐」收敛为 P1=A 单一口径一次成文、不写「可容纳两种结果」、不冒充已锁定；批准记录（`status: approved` + 四件套指纹）须由用户在规格门明确拍板后由 leader 回填，**不拿沉默当授权**。锁定顺延：若拍板结果与推荐不同，MUST 按 `CODEBUDDY.md §8` 同步回查四件套 → leader 重算指纹 → `critic` 重出规格门报告 → 用户第二轮确认锁定。一个 change 一个 owner：`sim-owner`（`crates/w3-core/`）为主 owner，`net-owner` / `client-owner` / `docs-owner` 按 `CODEBUDDY.md §9` 串行协作、不并行改同一 owner 文件。
> 每条任务带可核验的完成判据（命令 / 测试 / 产物）。规则依据 `docs/references/WC3-MELEE-RULES.md` 正文当前行号与规则编号 `R-x.y`（附录 `:352-369` 历史旧行号不沿用）；确定性硬门禁 ADR 0002 / `CODEBUDDY.md §5`；参考结论见 `design.md`「## 参考项目对照」；Q1–Q12 见 `decision-memo.md`。

## 1. 规格门前置（提案阶段收尾，本阶段执行到 1.5 止）

- [ ] 1.1 按 `openspec instructions` 四件套模板一次成文 `proposal.md` / `specs/**`（新增 `creep-camps` 12 条 ADDED + 6 份 14 条 MODIFIED）/ `design.md` / `tasks.md` + `decision-memo.md`，回查 proposal 的 Scope / Capabilities ↔ specs 的 `#### Scenario:` ↔ design 的 D1–D8 alternatives /「## 参考项目对照」↔ tasks 分组步骤互不打架；验证：`openspec validate p8-creep-camps --strict` 零 warning
- [ ] 1.2 复核 `design.md`「## 参考项目对照」逐条完整转写 `reference-researcher` 报告的全部结论与证据锚（第一节 T1 / T4 / T5 命中、T2 / T3 / T6 条件命中、第二节结论 1–6、第三节内核现状 8 项、第四节结论 7、第五节结论 8 与 A / B / A′ 代价、第六节 A / B / C 分类、第七节参考缺口 7 条、第八节许可证核对、附录锚复核：`vera20k` 固定提交 `305ea3a75` / HEAD `590e87068` 偏离、`WC3-MELEE-RULES.md` 附录 `:352-369` 历史旧行号 vs 正文 `:186-216`、Warsmash AGPLv3 只借语义、godot `RandomNumberGenerator` / `randomize()` / 墙钟 / `HashMap` / 建筑名白名单禁借鉴）、`decision-memo.md` C1–C28 逐条带锚且归因声明不误读；验证：人工逐条对照本会话转达报告、`openspec/config.yaml` 的 `rules.design` 第 5 条满足、`critic` 清单第 12 项复核锚可打开性
- [ ] 1.3 跑 `python tools/check_encoding.py` 全绿（UTF-8 无 BOM / LF / 末尾换行 / 无行尾空格）；验证：命令输出 `Encoding check OK` 且 exit 0
- [ ] 1.4 跑 `python tools/verify_all.py --fingerprint p8-creep-camps` 只读重算四件套 sha256 指纹、如实记录 64 位 hex 进交付报告；验证：输出 64 位 hex；**本阶段不回填 `decision-memo.md`「## 规格门」的 `fingerprint` / `status`**（保持 `pending`），不 self-approve
- [ ] 1.5 `critic` 规格门报告（清单第 7 / 10 / 12 / 13 项，尤其复核 6 份 MODIFIED delta 逐字保留全部历史 `#### Scenario:` 身份键、B-1 正面冲突是否消除、其余主 specs 历史口径与 `## Purpose` 是否如实登记为已知限制）→ 用户规格门经 `AskUserQuestion` 逐条拍板 Q1–Q12 并批准；验证：拍板记录（本会话用户消息 / leader 转达，非仓库文件、人工核对）+ 修订后 `openspec validate p8-creep-camps --strict` 零 warning + `python tools/check_encoding.py` 全绿；**若拍板与推荐不同须先同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户第二轮确认锁定；本轮 `status: pending` 不变、不回填指纹、不代替重出报告 / 确认锁定**

## 2. 定义层：野怪 / 营地 / 掉落表冻结常量（按 Q8 推荐全合成原创、单一路径）

- [ ] 2.1 按 Q8 推荐把野怪 DefId / 营地分级、掉落表清单与概率、经验 / 金币、leash 半径、归巢阈值、`acquisitionRange`、昼夜时长、随机池类别编码落到 `FrozenDefinitions` 冻结常量（client 侧合成编译期常量表 / 真实导出 JSON / 参考项目数值为被否备选、不实现），全为合成原创编译期整数常量 `[待核]`、不含 `f64` / MPQ 派生 / 真实导出 JSON / 参考项目单位名或 rawcode；验证：干净环境（无 `config.toml` / 导出目录）构造成功、零文件 IO、`cargo test -p w3-core` 相关用例绿（R8.2 / R8.6 / R1.2 / ADR 0001 / ADR 0004 / §14；`w3-core` MUST NOT 依赖 `w3-client`，`CODEBUDDY.md §5`）
- [ ] 2.2 把掉落表「表 → 互斥 set → 概率条目」分层冻结常量段接入 `compute_defs_hash`（`gameplay.rs:402-462`）→ `defs_hash` / `content_hash` 进快照头部（`snapshot.rs`、`sim_match.rs:2157-2159`）；验证：改常量即改变 `content_hash()`、漏接任一段的退化实现被断言拦截（主 `gameplay-systems/spec.md:8,12-15`），与 `specs/creep-camps/spec.md`「掉落表冻结常量与确定性抽取」Scenario 口径一致
- [ ] 2.3 野怪 / 营地可用性与掉落可用性由定义层类别与 set 数据驱动，MUST NOT 用建筑名字符串白名单或硬编码 DefId；验证：以非经典命名的野怪 / 营地 DefId 仍按冻结定义判定、以白名单驱动的退化实现被回归拦截（R1.2 / R13.3 / D4 / `design.md` 结论 5「明确不借鉴」）
- [ ] 2.4 明确区分野怪 / 营地 / 掉落 / 随机码消费序列 **权威状态**（进哈希 / 快照 / `first_difference()`）与掉落表 **冻结常量**（定义层内容变化、经 `defs_hash` / `content_hash` 进快照头部）；验证：代码审查 + `specs/creep-camps/spec.md` 两处表述一致、无把冻结常量误当权威状态字段的退化实现（ADR 0002 §6-8 / D1 / D4）

## 3. 内核权威状态与哈希 / 快照 / `first_difference()`

- [ ] 3.1 在 `w3-core` 新增野怪 / 营地 / 中立归属（按 Q7 拍板结论）/ leash / home 状态、掉落事实 / 地面物品 / 物品实例、击杀收益登记、确定性随机码消费序列等权威状态，全部落 `BTreeMap` 等确定性有序容器、按稳定 `EntityId` / `PlayerId` 顺序遍历；MUST NOT 用墙钟 / 渲染帧率 / 全局静态字典 / `HashMap` 迭代顺序 / `f64`；验证：`cargo test -p w3-core --lib` 绿、代码审查无 `HashMap` / `SystemTime` / `Instant` / `f64` 进权威状态（ADR 0002 §2-3 / D1 / R8.1 / R1.1）
- [ ] 3.2 把全部新增权威状态同步接入 FNV-1a 状态哈希（`sim_match.rs:1925-1985` hash 段）、由 `SNAPSHOT_VERSION` 定义的快照（`snapshot.rs`）与 `first_difference()` 差异字段集合（`sim_match.rs:1989-2049`）；验证：双实例交错逐帧 `compute_state_hash()` 相等、帧 N 捕获 / 销毁 / 恢复后继续到帧 M 逐帧一致、`first_difference()` 能定位到新字段（ADR 0002 §6-8 / `CODEBUDDY.md §5` / C1 / D5）
- [ ] 3.3 按 Q4 推荐 bump **5→6** 处置 `SNAPSHOT_VERSION`：连同 `SIM_VERSION`、`snapshot.rs:26-27` 常量、`sim_match.rs:2622-2623` 单测硬断言、快照头部校验、golden hash 基线、ADR 0002 复核记录追加（不删改 §1–§8 正文）、路线图 :138 ADR 复核同步更新；验证：`cargo test -p w3-core --lib` 绿、版本不匹配的旧 v5 快照 / 回放被 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝（ADR 0002 §7 / ADR 0005 §3 / D5）
- [ ] 3.4 重开 / 快照恢复后不残留野怪 / 营地 / 掉落 / 随机码消费序列 / 收益登记的反例元断言回归：在测试内构造退化实现（结算后重开仍残留上一局野怪 / 掉落 / 消费序列字段，或帧 N 捕获快照恢复后重复掉落）并加元断言，证明缺陷态断言失败、修复态通过；验证：新一局 `victory.resolved == false`、营地状态回到初始、掉落登记为空、随机码消费序列复位、逐帧 `compute_state_hash()` 与同脚本基线一致，缺陷态红 / 修复态绿两条证据留档（R11.5 / R12.4 / ADR 0002 §7 / `CODEBUDDY.md §5` 反向验证 / Q9 / C23）

## 4. 确定性随机码消费契约（按 Q6 推荐、单一路径）

- [ ] 4.1 按 Q6 推荐实现「按死亡事件以稳定 `EntityId` 顺序确定性消费随机流」：复用 match-owned 整数 LCG（`sim_match.rs:244-248`）+ `seed` 随快照（`:2056,2061`），消费序列 / 随机流状态进状态哈希 / 快照 / `first_difference()`；MUST NOT 用表现层独立随机源 / 墙钟 / `f64` / Godot `RandomNumberGenerator` / `randomize()` / `HashMap` 迭代顺序；验证：同帧多野怪死亡按稳定顺序消费、两端逐帧哈希一致、重放不双重消费（R8.6 / ADR 0002 §2-4 / D2 / `design.md` 结论 2）
- [ ] 4.2 死亡事件幂等 + 同一互斥组不重复掉落：以 `CreepDied` 类事实事件解耦掉落服务、「已处理死亡」登记进权威状态；验证：对同一野怪重复投递死亡通知只结算一次、同 seed 重放两次掉落逐帧一致、互斥组不产生两件（R7.2 / R8.6 / `design.md` 结论 5 / `selftest_item_system.gd:112,117-120,126` 机制语义）
- [ ] 4.3 确定性随机码消费契约的具体设计（独立事件驱动流 / 按帧派生 / 消费序列编码）按 Q6 拍板结论落地、结论未定前 MUST NOT 臆造为「已核」；验证：`decision-memo.md` Q6 拍板记录 + `specs/creep-camps/spec.md`「确定性随机码消费序列」Scenario 与代码一致（C 类参考缺口 2 / §14 / D2）

## 5. 野怪 AI（leash / 归巢）、击杀收益与地面物品

- [ ] 5.1 野怪 leash / 归巢按 `IDLE / ENGAGED / RETURNING` 状态机 + home 锚点 + 营地聚类 + 定义层 `acquisitionRange` 数据驱动实现，MUST NOT 无限追出营地；追击 / 归巢复用既有 `Attack` / `Move` 命令面与 `run_ai` 确定性重算框架（`sim_match.rs:1865-1892`）、不新设 AI × lockstep 架构；验证：引出 leash 半径外停止追击 / 归巢回归绿、逐帧哈希与同脚本基线一致、「无限追击」退化实现被断言拦截（R8.5 / R12.7 / §0.3 T5 / D3）
- [ ] 5.2 击杀收益在共享结算服务按稳定 `EntityId` 顺序只结算一次：经验给附近英雄（R6.1，分摊「进入集合 vs 实际入账」两环节 R6.5）、金币给击杀方（R8.3）、掉落（R7.2）；MUST NOT 由表现层反写 `HeroState.xp` / `PlayerEconomy.gold` / 背包；验证：击杀前后 xp / gold 精确增长且进哈希 / 快照、「击杀无收益 / 表现层反写」退化实现被断言拦截（R8.3 / ADR 0002 §5-6 / D4 / Q9）
- [ ] 5.3 地面物品 / 物品实例权威状态：无主地面物品（R1.1）、同名道具多实例独立状态（R7.5）、效果走既有 `EffectDef` / 不做 JASS VM（R7.6 / N5）；地面物品 MUST NOT 出现在 `Move` 命令生成集合、位置不变；验证：两件同名道具各有独立充能 / 冷却且进哈希 / 快照、框选含地面物品 / 金矿 / 主城的混合集合按 `M` 只生成可移动单位的 `Move`（R1.1 / R1.4 / R7.5 / R7.6 / D4）
- [ ] 5.4 按 Q2 推荐「最小闭环」边界：拾取 / 六格背包 / 商店购买 / 回城卷轴全部留 P8.5d、显式登记为范围外承接；验证：`tasks.md` / `design.md` Non-Goals / `specs/creep-camps/spec.md`「范围边界与后续承接」三处一致、不冒充本期已解决（路线图 :128 / Q2 / D7）

## 6. 命令与事件面、线格式（按 Q5 推荐新增独立变体，单一路径）

- [ ] 6.1 按 Q5 推荐新增野怪击杀 / 掉落事实 / 地面物品相关 `GameplayCommandKind` / `EventKind` 变体（不复用既有 `Died` / `AttackHit` 且不扩展 payload 的 Q5 被否备选、不实现），经公开提交入口按冻结整数帧号入队、同源校验、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；若 `Event` 无 payload 则扩展只读事实事件供表现层消费；验证：逐帧提交 / 一次推进 8 帧两端逐帧哈希一致、重放不双重注入（ADR 0002 §2-3 / R12.4 / R12.7 / D6 / C9）
- [ ] 6.2 同步 `hash_gameplay_command` 编码 / 校验 / 穷尽分发 / `EventKind` 覆盖新增变体；`w3-render/src/scene.rs:201-226` 对新增 `EventKind` 补穷尽分支；验证：`cargo test -p w3-core --lib` 绿、穷尽匹配无遗漏分支、命令哈希编码唯一（C9 / D6）
- [ ] 6.3 按 Q5 推荐新增变体牵动 ADR 0005 protobuf（条件命中矩阵 T6，`net-owner` 按 `CODEBUDDY.md §9` 与 `sim-owner` / `client-owner` 串行协作）：字段编号只追加（现 tag 33/34/35 之后、**新增从 36 起**）、保留 `reserved 14 to 32`、bump `PROTOCOL_VERSION` 5→6、`Packet::decode` 对旧版本 `VersionMismatch` 明确拒绝、canonical 约束（禁 `map` / 禁 `float` / 禁非规范 varint / unknown fields 不透传）继续适用、按 ADR 0005 自有 `W3N1` + protobuf 契约执行不借鉴 `vera20k` 线格式；验证：`cargo test -p w3-net` 绿、CI Windows / Linux 双平台生成代码字节一致、relay 双客户端逐帧哈希一致（`docs/decisions/0005-protobuf-payload.md:34-44,84` / `UPSTREAM.md:88` / C21 / D6）

## 7. 表现层 HUD 只读派生、回归与确定性门禁

- [ ] 7.1 野怪血条 / 掉落标记 / 击杀收益 HUD 只从只读 `WorldView`（必要时只读投影字段）/ 事实事件 / 只读 `Snapshot` 派生（复用 P8.5a `ReviveHudModel::derive(snapshot, events)` / P8.5b `derive_upgrade_slot` 范式），MUST NOT 反写 `RtsMatch` 权威状态 / 进状态哈希 / 快照 / `first_difference()`；只读视图扩展 MUST NOT 引入墙钟 / 全局静态字典 / 建筑名白名单 / `f64`；验证：`cargo test -p w3-render` / `crates/w3-render/tests/isolation.rs` 绿、HUD 连续重绘时逐帧 `compute_state_hash()` / `capture_snapshot()` 字节与无 HUD 实例一致（ADR 0002 §5 / R10.5 / R13 / D7）
- [ ] 7.2 UI 文案走 i18n、不进日志 / 权威状态；日志 / 错误英文结构化 `key=value`、源码注释中文；验证：`python tools/check_source_conventions.py` 全绿（`CODEBUDDY.md §3` / ADR 0002 §5）
- [ ] 7.3 新增 / 修改玩法回归 MUST 证明缺陷态失败、修复态通过（Q9 推荐 `CODEBUDDY.md §5` 形态①：测试内构造反例 + 元断言防退化），退化实现真经生产路径注入、非旁路模拟；至少覆盖 Q9 退化类：① tier / 野怪 / 地面物品 / 随机码消费序列漏接 `compute_state_hash()` 或 `capture_snapshot()`；② 掉落用墙钟 / `HashMap` 迭代顺序 / Godot `randomize()` 等非确定随机源；③ 击杀无经验金币 / 表现层反写 xp 或 gold / 掉落幂等退化；④ 掉落随机码消费序列不进哈希 / 快照 / `first_difference()`；⑤ 野怪 / 掉落命令或事件漏接 `hash_gameplay_command` 编码 / 校验 / 穷尽分发 / 线格式 canonical；验证：每组「缺陷态红 / 修复态绿」元断言证据可复现（`CODEBUDDY.md §5` / C23 / Q9 / D8；先例 `p8-dual-faction` 成对 `reverse_validation_*` / `p8-hero-revive` 7.1 / `p8-townhall-upgrade` 7.1）
- [ ] 7.4 涉及 `w3-core` 权威状态 / 帧序 / 命令排序 / 状态哈希 / 快照字段的改动 MUST 重跑 `cargo test -p w3-core --test determinism`（26 项）全绿；验证：`cargo test -p w3-core --test determinism -- --nocapture` 26 项全绿、输出留档（`CODEBUDDY.md §5` / `openspec/config.yaml` `rules.tasks` 第二、三条 / C1）
- [ ] 7.5 golden hash 基线按新实现如实更新：以新增权威状态后的实现重取逐帧基线、不沿用 P8.5b v5 值、不声称逐字节不变；按 Q4 推荐 bump 6，Windows 侧离线 golden hash 逐字节比对基线重取留档；验证：新基线文件 / diff 输出留档，**MUST NOT 声称 Debian 12 侧 v4 / v5 / v6 逐字节比对已完成、MUST NOT 声称 macOS / ARM64 / 其它 rustc 已验证**（R12.4 / ADR 0002 复核记录 / Q12 / C7 / D5；Debian 侧比对登记为人工 / VM 待办）
- [ ] 7.6 跑 `cargo test --workspace`、`cargo fmt --all` 后 `cargo fmt --all -- --check` 全绿（串行单实例，避免 `model_index` 固定临时目录并发 FAILED，R-j 先例）；验证：命令输出全绿（`CODEBUDDY.md §2 / §5 / §7` / Q12）
- [ ] 7.7 主 specs 6 份 MODIFIED delta 归档前复核：逐字保留全部历史 `#### Scenario:` 身份键、口径改在 THEN、requirement 正文给完整新正文、每条带 `>` 身份键说明、MUST NOT 写 `## Purpose` 覆盖 delta、消除与推荐方向 A 的正面冲突；验证：`openspec validate p8-creep-camps --strict` 零 warning、`critic` 清单第 7 / 10 项复核 `blocker: 0`（B-1 教训 / Q10 / D8）

## 8. 真窗口人工验收清单（按 Q2 / Q3 推荐：范围外承接登记为后续 change 待办；真窗口项未执行 / 待人工，不提供替代、不冒充通过）

> 本组为人工桌面验收项，CLI / 离屏自证无法覆盖 egui overlay 与真实 GPU 画面（主 `playable-window-entry/spec.md:168-176` / 路线图 :117 / C23）。**全部保持 `- [ ]` 与「未执行 / 待人工」，先离屏自证 + CPU 回归全绿再请人工验收；清单未逐项勾选前不得归档；CI MUST NOT 声称已验证 GPU**。GUI 类修复先自测再请人工验收（不反复试错）。Q11 要求本组保持未勾、无证据项如实标注、不冒充通过。

- [ ] 8.1 真窗口启动 `melee` 可玩入口，肉眼确认野怪营地可见、接近营地时野怪进入 ENGAGED 追击、引出 leash 半径后停止追击并归巢到 home 锚点、MUST NOT 无限追出营地 —— 状态：未执行 / 待人工（R8.5）
- [ ] 8.2 击杀野怪后肉眼确认：附近英雄经验增长、金币增长、掉落物出现在地面、HUD 血条 / 掉落标记只读可见；同 seed 重放两次掉落逐帧一致 —— 状态：未执行 / 待人工（R6.1 / R8.3 / R8.6 / R7.2）
- [ ] 8.3 清掉一个完整营地后肉眼确认：清野可重复、掉落幂等（同一野怪不重复掉）、同一互斥组不产生两件、对局未中断、AI 对手继续行动、能打完一局并看到 `VICTORY` —— 状态：未执行 / 待人工（R8.6 / R11 / R12.7；AI 玩家清野按 §0.3 T5 留后续 change）
- [ ] 8.4 右键 / 键盘 `Move` 含地面物品 / 金矿 / 树木 / 主城的选中集合时，肉眼确认只有可移动单位移动、地面物品 / 资源点 / 建筑不动 —— 状态：未执行 / 待人工（R1.1 / R1.4 / R13.6，防止本 change 破坏既有过滤）
- [ ] 8.5 `Resized` 窗口缩放 / 高刷 30 Hz 下 HUD 野怪血条 / 掉落标记与逐帧确定性无异常 —— 状态：未执行 / 待人工（无证据项如实标注，不冒充通过）
- [ ] 8.6 清单项保持 `- [ ]` 与「未执行 / 待人工」、无证据项如实标注、不用离屏截图 / CPU 回归冒充真窗口验收、CI 边界如实说明 —— 状态：未执行 / 待人工（C23 / Q11 先例；Q2 / Q3 范围外承接：拾取 / 六格背包 / 商店 / 回城卷轴 → P8.5d，昼夜睡眠 / AI 清野 / 迷雾 / 中立建筑交互 → 后续 change，当前实现下真窗口不可达、一并如实登记）

## Workflow follow-up

- 规格门：`critic` 规格门报告 → 用户规格门经 `AskUserQuestion` 逐条拍板 Q1–Q12 并批准 → **leader 回填 `status: approved` + 四件套指纹 + `approved_by` / `date` / `reason`** → `/opsx:apply`（`sim-owner` 为主 owner，`net-owner` / `client-owner` / `docs-owner` 按 §9 串行协作）→ `critic` 代码门 → 本地 commit（中文描述 + Conventional Commits 英文 type 前缀）→ 人工 push → CI 双平台 `verify_all.py` → 真窗口清单逐项勾选 → `/opsx:archive` 并确认主 specs 合并（新增 `creep-camps` + 6 份 14 条 MODIFIED）。**本轮 `status: pending` 不变、不回填指纹、不 self-approve、不拿沉默当授权、方向 A / B / A′ 未拍板**；若拍板与推荐不同须「同步修订四件套 → leader 重算指纹 → `critic` 重出报告 → 用户第二轮确认锁定」全流程。
- 已知限制承接（后续独立 change，不改本 change 四件套）：所有已存在 spec 的 `## Purpose` 概述性文字（实测 OpenSpec 1.17.0 delta 不合并，`docs-owner` 收口，见 `design.md`「## 已知限制 / 边界」R-1）；其余 15 个含同类历史口径的主 specs 统一收口（`docs-owner`，是否本 change 一并全量修订属 Q10 待拍板）；`WC3-MELEE-RULES.md` 附录补锚 / 防误用清单补锚 / `:352-369` 历史旧行号与正文 `:186-216` 差异（按 Q10 推荐，`docs-owner`）；拾取 / 六格背包 / 商店购买 / 回城卷轴（按 Q2 推荐，P8.5d）；R8.4 昼夜睡眠 / AI 玩家清野 / 迷雾下野怪可见性 / 中立建筑交互（按 Q3 推荐，后续 change）；ADR 0005 修订记录未随 `PROTOCOL_VERSION` 5→6 追加（Q12 residual）；Debian 12 侧 v4 / v5 / v6 golden hash 逐字节离线比对（Q12，人工 / VM）；跨平台确定性 macOS / ARM64 / 其它 rustc 待验证；`model_index` 固定临时目录并发 FAILED / 串行单实例门禁（R-j）；P8.5a / P8.5b 已知限制承接（真窗口 UI/AI 接线 / T3 全量升级链 / `U` 键绑定 / `snapshot_bytes` Debug 代理 / 中文 `assert_eq!` 消息惯例 / `>500 characters` warning 等，见各自归档纪要）。
- 归档时逐条登记未勾任务原因、已知限制承接、主 `openspec/specs/` 是否变化据实说明；`status: pending` 令 `tools/verify_all.py::spec_gate_check()` 变红是 fail-closed 正确信号（C22），本阶段不绕过 / 不改门禁；`python tools/verify_all.py --fingerprint p8-creep-camps` 只读重算指纹如实记录、不回填门禁。
- 人工边界（非待拍板，仅登记）：本地 `git add` / `commit` 由 AI（中文描述 + Conventional Commits 英文 type 前缀），四件套 + `decision-memo.md` MUST 一并纳入 commit；`push` / `fetch` / `pull` / `clone` 及任何远端同步、CI、真窗口人工验收、`/opsx:archive` 归档全部人工独占且本轮未执行。
