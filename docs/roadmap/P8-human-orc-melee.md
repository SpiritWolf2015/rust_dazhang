# P8 路线图：人族 / 兽族完整化

> 制定日期：2026-11-14。用户想法「把人族和兽族做完」。
> 本文是**制作进度水平安排**（跨多个 OpenSpec change 的路线图），不是单个 change 的规格；正式立项时把各阶段转成独立 change 的 proposal / specs / design / tasks + `decision-memo.md`，按 `CODEBUDDY.md` §9 走规格门。
> 依据：`godot_warcraft3`（MIT）与 `WarsmashModEngine`（AGPLv3）均为**只读参考**，路径与许可证见 `docs/references/UPSTREAM.md`；不复制任何代码，资产零入库（ADR 0001）。
>
> **范围上位说明（2026-11-14 用户后续拍板 1.0）**：1.0 最终范围与业务规则以 `docs/references/WC3-MELEE-RULES.md` §0 为准——只做**局域网联机对战 + 回放**，覆盖**4 个种族 / 道具 / 野怪 / 中立建筑**，**不做单机对战 / 地图编辑器 / RPG 自定义地图 / 剧情战役**。本文 P8 的「两族」「玩家选族 vs AI 对手」「单机打完一局」是**过程性制作安排**，与 1.0 范围的两处张力（4 族范围、单机 AI）已如实登记在 `WC3-MELEE-RULES.md` §0.3 T1 / T2，**待用户拍板**再对齐；本文不擅自改写 1.0 范围，各阶段玩法 change MUST 同时引用业务规则 `R-x.y`。

---

## 0. 「做完」的水位定义（推荐口径，可调整）

**目标**：两族各自都能打完一局完整 Melee 1v1（玩家选族 vs AI 对手），不靠作弊补洞。

**P8 必做（推荐水位）**：

| 维度 | 每族最小集 |
|---|---|
| 经济 | 1 工人（采金 / 伐木 / 送回）、主城 / 大厅、金矿、树木 |
| 建筑 | 人口建筑、军事建筑、英雄祭坛、1 个科技 / 升级建筑 |
| 战斗 | 2–3 个可训练战斗单位（前排 / 远程 / 反重甲至少各一） |
| 英雄 | 1 名英雄 + 1 个主动技能（合成 / 原创定义） |
| 科技 | 1–2 项升级（攻 / 防或解锁单位） |
| 对局 | 双玩家开局、AI 对手、胜负结算、重开干净 |
| HUD | 命令卡 / 选择面板 / 小地图 / 生产队列、**可选族** |
| 数据 | 全部为合成 / 原创**编译期整数常量**，零资产读取（ADR 0001） |

**P8 必做 · 深度玩法（用户 2026-11-14 明确拍板「要更重，含 P8.5 全量」）**：英雄复活、主城升级（Keep / Stronghold）、野怪营地与掉落、物品 / 商店 / 回城卷轴、战争迷雾；同时纳入窗口内重开按键、中文上屏（CJK 字体许可证按 `CODEBUDDY.md` §10 人工拍板）。

> 水位变更：P8.5 由「可选深度」提升为**必做**，与 P8.0–P8.4 同属 P8 验收范围；五个子系统各单独开 change，不塞回前期阶段。相对水位因此更接近 Warsmash 的完整 WC3 语义，但仍不做 JASS / wtg VM、全科技树、联机、战役（N∞ 远期）。

**明确不做（N∞ 远期，门禁后才开）**：JASS / wtg 触发器 VM、自定义地图、战役、全科技树、全单位全技能插件化、联机 / 录像、物体编辑器、Mod API、官方战网。对齐 ADR 0003 第一版范围。

> 与参考项目的相对水位：`godot_warcraft3/docs/roadmap/NEXT.md` 的 N4-C 只提「第二种族**最小集**：苦工 → 大厅 → 猎头 → 英雄一技能」；本路线图比它重一档（两族 each 有前排 / 远程 / 反重甲 + 科技 + 双族对抗），但**远轻于** Warsmash 的完整 WC3 语义（后者按种族分组的技能行为定义约 1.6 万行）。

---

## 1. 两个参考项目提供什么

### `godot_warcraft3`（MIT，只读）

| 可借鉴 | 出处 |
|---|---|
| 分阶段节奏：场景壳 A → 会话 B → Melee Bootstrap C → 最小交互 D → **人族竖切 F0–F10** → 触发器 E 远期 | `docs/design/game/ROADMAP.md` |
| 人族收尾清单 N1：英雄复活、Keep 升级、铁匠武器 / 护甲升级、生产队列 HUD、单位自然回血回蓝、野怪 AI 手测 | `docs/roadmap/NEXT.md` |
| 战斗手感 N2：编队 / 群体寻路、弹道 FX、Hold / Patrol / 攻击优先级 | 同上 |
| **第二种族建议**：N4-C 明确「建议兽族：苦工 → 大厅 → 猎头 → 英雄一技能」，且警告「不要并行三条」 | 同上 |
| Melee 对战初始化动作表（初始资源、创建开局单位、清开始点附近野怪、AI 脚本、胜负条件）先用**原生模块**而非 wtg VM | `docs/design/game/ROADMAP.md` §0 / §2-C |
| 工程纪律：**数值读 SLK Def，禁止逻辑里写死造价 / 伤害**；技能拼 Effect + 注册 Behavior；分层 Data / Catalog / Logic / Present | `docs/roadmap/NEXT.md` §4、`docs/roadmap/CLASSIC_RULES_BASELINE.md` |
| 灰盒批次 U0–U3，U3「结束比赛」判据：**每族至少两种开局和两种主力组合都能结束比赛** | `docs/design/game/FACTION_UNIT_ROSTERS.md` §9 |
| 对战 AI A0–A4 闭环（经营 AI、军队调度、清野拾取、胜负结算）与道具 I0–I4 | `docs/roadmap/ITEMS_AND_MELEE_AI_PLAN.md` |

**不借鉴**：Godot / GDScript / C# 实现、`double` 数值路线（与 ADR 0002 冲突）、其「原创三族」单位设定（`FACTION_UNIT_ROSTERS.md` 是原创世界观，不是经典人族 / 兽族复刻，只借**结构与批次判据**，不借单位名与数值）。

### `WarsmashModEngine`（AGPLv3，只读语义参考）

| 可借鉴（仅设计语义） | 出处 |
|---|---|
| 完整 WC3 玩法的**结构**：单位 / 技能 / 科技 / 英雄 / 物品 / JASS 行为如何分层 | 仓库 `core/`、`jassparser/`、`fdfparser/` |
| 按种族 / 英雄分组的技能行为定义（人族单位 1538 行、人族英雄 829 行、兽族英雄 222 行、通用被动 2253 行、reactions 2064 行；合计约 1.6 万行） | `core/assets/abilityBehaviors/*.json` |
| 英雄技能清单：`Human/{Archmage,MountainKing,Paladin}`、`Orc/{TaurenChieftain}` | `resources/Scripts/Abilities/` |
| JASS UI 帧定义，仅作 FDF / UI 格式参考；`melee.jui` **不是**对战初始化动作表，该语义见 `godot_warcraft3/docs/design/game/ROADMAP.md` §0 / §2-C | `resources/Scripts/melee.jui` |
| 经典规则待核清单（复活常量、UnitBalance 836 行、技能 799 项、补丁差异） | 借鉴「先核来源再写数值」的做法 |

**硬性边界**：AGPLv3 与本项目 GPLv3 组合有条件，**只借鉴设计语义、不得复制代码**；若将来要引入其代码，须按 ADR 0001 复核点重新评估许可证，不得直接并入（`docs/references/UPSTREAM.md:21`）。本项目也不引入其 Java / LibGDX 依赖。

---

## 2. 现状差距（2026-11-14，已归档 `melee-playable-loop` 之后）

- `crates/w3-client/src/melee/defs.rs`：仅 **6 个 `UnitDef`**（`PEASANT` / `SOLDIER` / `TOWN_HALL` / `BARRACKS` / `GOLD_MINE` / `TREE`）+ 1 `AbilityDef` + 1 `ItemDef` + 1 `UpgradeDef`，全为编译期整数常量，**无种族维度**；`ExportSource::definitions()` 恒回落合成定义、零文件 IO。
- `MeleeSetup`：12 个开局绑定、6 类模型键，**双方同族**（共用同一套定义）。
- `w3-core`：命令面已齐（`Spawn` / `Move` / `Stop` / `Attack` / `Cast` / `Train` / `CancelTrain` / `Research` / `BuyItem` / `Gather` / `SpawnWave` / `SetAi` / `Leave`）、`FrozenDefinitions` / `PlayerEconomy` / `VictoryState::resolve_victory` / `run_ai`；但 AI 与胜负是**单一种族语义**。
- 表现层：真实 glTF 模型 + GPU `texture_2d_array` 管线已有；运行时 `Train` / `Build` 新实体走模型缺键回退（已知限制）。
- 主 specs：14 capabilities（含刚归档合入的 `melee-match` 8 条、`rts-command-hud` 6 条）。
- 本期验收遗留的**已知限制**（见 `openspec/changes/archive/2026-11-14-melee-playable-loop/design.md` Risks 末条）：**右键群体 `Move` 未按可移动性过滤**，主城 / 金矿随农民一起移动；用户明确不在本期修。

**差距核心**：`FrozenDefinitions` 没有种族维度；两族要扩成「按种族分段的 DefId 定义表 + 开局装配按种族选择 + AI / HUD / 表现层跟随」，且**尽量不新增权威状态字段**。

---

## 3. 分阶段进度（每阶段一个独立 OpenSpec change）

编号即推荐顺序；**不并行多个阶段**（借 `godot_warcraft3` N4「不要并行三条」）。每阶段自带 CPU 回归与验收线。

### P8.0 承接与地基（必做，先修已知限制）

- 修「群体 `Move` 未过滤建筑 / 资源点」：输入翻译与 `Move` 命令生成处按单位可移动性过滤，补「建筑 / 资源点不响应移动」回归测试（含反向验证）。
- 引入**定义层**种族维度：DefId 按种族分段（如人族 1xx、兽族 2xx），`UnitDef` / `AbilityDef` / `UpgradeDef` 仍在编译期常量中；**玩家选族体现在开局 `Spawn` 命令的 DefId 上，不新增 Player.race 权威状态**，从而 `w3-core` 零改动、`SNAPSHOT_VERSION` 仍 3、`defs_hash` 天然覆盖定义差异（ADR 0002）。
- **验收**：回归测试全绿；`cargo test -p w3-core --test determinism`（26 项）全绿；`git diff -- crates/w3-core/` 为空；两族 DefId 分段无冲突。

### P8.1 人族完整化（对齐 `godot_warcraft3` N1 水位）

- 人族集：农民、前排、远程、主城、人口建筑、兵营、铁匠铺（科技）、祭坛（英雄）。
- 1 英雄 + 1 主动技能（借 Warsmash `Human/Archmage` 的**技能结构**，用合成原创定义与数值）。
- 1–2 项升级 / 科技；`Research` 与 `Train{is_building}` 路径接通。
- **验收**：CPU 端到端「采集 → 建造 → 训练 → 英雄 → 战斗 → 结算」一局可打完；逐帧哈希与基线一致。

### P8.2 兽族完整化（比 `godot_warcraft3` N4-C 更完整）

- 兽族对应集：苦工、前排、远程 / 猎头类、大厅、人口建筑、兵营、战争磨坊、祭坛。
- 1 英雄 + 1 主动技能（借 Warsmash `Orc/TaurenChieftain` 语义）。
- 数值与人族**有差异**（更肉 / 更快 / 更低远程稳定度），但只用合成整数常量；不复制 `FACTION_UNIT_ROSTERS.md` 的原创单位名。
- **验收**：兽族 CPU 端到端一局可打完。

### P8.3 双族对抗与 AI

- 玩家可选族（入口参数 / HUD），AI 可选另一族；`run_ai` 按种族产出不同训练 / 进攻命令（命令脚本按 DefId 分段，不新增权威状态）。
- 两族混战 CPU 端到端：**人 vs 人 / 人 vs 兽 / 兽 vs 人 / 兽 vs 兽** 四种组合各能分出胜负、重开干净。
- **验收**：四组合脚本全绿、逐帧哈希一致、无上一局残留。

### P8.4 HUD 与表现层

- 命令卡按种族 / 单位类型显示、生产队列、选择面板、小地图；选族入口可见。
- 表现层模型键扩充到两族单位 / 建筑；修「运行时新实体模型缺键回退」（本期已知限制）。
- 可选：中文上屏（CJK 字体与许可证需人工拍板，按 `CODEBUDDY.md` §10）。
- **验收**：真窗口能选族、能框选、右键只移动可移动单位、能打完一局并看到 `VICTORY`（人工项，离屏自证不覆盖 egui overlay）。

### P8.5 深度玩法（必做，全量；用户 2026-11-14 拍板纳入）

参考 `godot_warcraft3` N1 / N2 / N4-A、`ITEMS_AND_MELEE_AI_PLAN.md`（道具 I0–I4、AI A0–A4）与 Warsmash `Abilities/{Human,Orc}/` / `abilityBehaviors/*.json`（`melee.jui` 经核验仅为 JASS UI 帧定义，不作对战初始化语义）。**五个子系统各单独开 change，串行推进，不塞回 P8.0–P8.4**：

| 子阶段 | 内容 | 参照 | 验收 |
|---|---|---|---|
| P8.5a 英雄复活 | 祭坛复活队列、费用 / 时间读冻结常量、英雄状态与背包恢复 | godot N1 `HeroDeathRegistry` / 复活常量待本机导出表核对（不指向 `melee.jui`） | 英雄阵亡后可复活、复活后状态与哈希一致 |
| P8.5b 主城升级 | 主城 Keep / Stronghold 升级、解锁二线单位与科技 | godot N1 `building_upgrade`、F7 | 升级后训练 / 解锁生效，快照哈希稳定 |
| P8.5c 野怪营地与掉落 | 中立野怪 AI（leash / 归巢）、死亡掉落表、确定性随机码 | godot `UNIT_AI.md` / `ItemDropTable` | 清野可重复、掉落逐帧一致 |
| P8.5d 物品 / 商店 / 回城卷轴 | 六格背包、地面拾取、商店购买、回城卷轴 | godot `ITEM_SYSTEM.md` I0–I4、N4-A | 拾取 / 购买 / 使用闭环，实例不共用充能 |
| P8.5e 战争迷雾 | 视野 / 已探索层、小地图单位点、表现层只读 | godot N4-D、Warsmash 视野语义 | 迷雾 MUST NOT 进权威状态 / 哈希 / 快照 |

**硬约束**：迷雾与掉落随机码若需进权威状态，MUST 按第 4 节走 ADR 0002 流程（状态哈希 + 快照 + `first_difference()` + determinism 26 项）；否则一律做成**表现层只读派生**，内核零改动。

---

## 4. 全程硬约束

1. **ADR 0001 零资产**：两族定义全为合成 / 原创编译期整数常量；真实导出数据（SLK / 地图 / glTF / PNG）只经 gitignored `config.toml` 作**表现层**接缝，不进仓库、不进权威定义。
2. **ADR 0002 确定性**：`w3-core` 尽量零改动；若确需新增权威状态字段，MUST 同时进入状态哈希、快照与 `first_difference()`，并跑 `cargo test -p w3-core --test determinism`（26 项）。`SNAPSHOT_VERSION`（当前 3）变更须 ADR 复核。30 Hz 整数帧、命令稳定排序键、稳定 `EntityId` 顺序、FNV-1a 哈希一律不变。
3. **ADR 0004 冻结边界**：`w3-content` 原始 `f64` 坐标 MUST NOT 直接转 `SimFixed`、MUST NOT 进哈希 / 快照；`ExportSource` 的 provider 边界保持。
4. **许可证**：四个参考项目只读；Warsmash AGPLv3 仅借鉴语义、不复制代码；新增依赖须核许可并登记 `NOTICE.md` / `Cargo.lock`。
5. **工作流**（`CODEBUDDY.md` §8 / §9）：一个 change 一个 owner、配只读 `critic`；proposal 阶段出 `decision-memo.md` 与规格门、`critic` 复审、四件套一致性、机械门禁 `verify_all.py`；`push` / `fetch` / `pull` / `clone` 人工独占。
6. **编码基线**：UTF-8 无 BOM / LF、中文注释与提交描述、英文日志与错误、Rust 官方 rustfmt、工具脚本跨平台 Python / Node。

---

## 5. 验收线（「做完」的判据）

- **CPU**：四种种族组合各能打完一局，逐帧哈希一致、重开无残留；`determinism` 26 项、`cargo test --workspace`、`verify_all.py` → `Verification OK`。
- **真窗口（人工）**：能选族、能框选、右键只移动可移动单位（P8.0 修复）、能打完一局并看到 `VICTORY`；无证据项如实标注未执行，不冒充通过。
- **玩法深度**（借 `FACTION_UNIT_ROSTERS.md` U3 判据）：每族至少 **2 种开局 + 2 种主力组合**都能结束比赛；同人口 / 同资源 / 同生产时间下比较，不凭单挑定价。
- **P8.5 全量必做**（用户 2026-11-14 拍板）：英雄可复活、主城可升级并解锁、野怪营地可清且掉落可复现、物品 / 商店 / 回城卷轴闭环可用、战争迷雾生效且不污染权威状态；五个子系统各有一组 CPU 回归与（必要时）人工真窗口判据。
- **合规**：仓库零资产、无新增未登记依赖、`git diff -- crates/w3-core/` 为空（或按第 4 节走完 ADR 0002 流程）。

---

## 6. 建议下一步

1. ~~用户确认水位~~ → **已确认（2026-11-14）**：采用更重水位，P8.5 全量（英雄复活 / 主城升级 / 野怪 / 物品 / 迷雾）纳入必做。
2. ~~P8.0 立项~~ → **进行中（2026-11-14）**：`p8-faction-foundation` 已 `openspec new change`，正在走四件套 + `decision-memo.md` → 规格门（人拍板 + 指纹回填）→ apply → `critic` 代码门 → 门禁。
3. P8.1 与 P8.2 **串行**推进，各自单独 change；P8.3 在两族 CPU 闭环都绿之后开。
4. 每阶段结束按 `feedback` 惯例如实登记已知限制，不为呈现问题扩范围。
