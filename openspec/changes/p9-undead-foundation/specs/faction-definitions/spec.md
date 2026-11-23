# Spec Delta

## MODIFIED Requirements

### Requirement: 冻结定义按种族分段

`FrozenDefinitions` 中 `units` 类别的 `DefId` SHALL 按种族分五段：人族 `1xx`、兽族 `2xx`、**不死族 `3xx`（300–399）**、**暗夜族 `4xx`（400–499）**、中立 / 资源 `9xx`，同段内唯一、跨段不冲突（Q1' 已拍板「不死 + 暗夜双族全量」+ P8 人 / 兽两族不重做）；分段只约束 `units` 类别，`abilities` / `upgrades` / `items` 仍各按类别内 ID 唯一。新增 `3xx` / `4xx` 段与按用户 Q2 拍板新增的族特色机制冻结常量全是 client `synthetic_definitions()` 的合成原创编译期整数 `[待核]`、零资产零文件 IO，MUST 逐一核对 `8xxx` / `9xxx` / `70_000` 按钮 / `1_000_000` 槽位命名空间不冲突（R2 / ADR 0001 / ADR 0004 / Q2 / Q5'）。内容哈希 MUST 经 `compute_defs_hash` 进入 `defs_hash` / `content_hash` 快照头部（ADR 0002 §6-8），分段 MUST NOT 引入 `f64`。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 分段唯一且可复现

- **WHEN** 构造含人族 `1xx`、兽族 `2xx`、**不死族 `3xx`**、**暗夜族 `4xx`** 与中立 / 资源 `9xx` 五段的 `FrozenDefinitions`
- **THEN** `units` 类别各 `DefId` 全局唯一、构造可重复、`defs_hash` 固定，与既有合成定义的 6 个 `UnitDef` 及 `8xxx` / `9xxx` 合成段不冲突；三个非 units 类别的 ID 仍各自类别内唯一；新增段与机制冻结常量全合成 `[待核]`、零文件 IO（R2 / ADR 0001 / ADR 0004 / Q1' / Q2 / Q5'）

#### Scenario: 零资产构造

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造五段定义
- **THEN** 构造成功且不做任何文件 IO，不含 MPQ 派生数据 / 真实导出 JSON / 参考项目单位名 / rawcode；新增定义与机制冻结常量经 `compute_defs_hash` 改变 `defs_hash` / `content_hash`，**Q2 已拍板的族特色机制另以 `w3-core` 顶层权威状态段表达、四处同步进入状态哈希 / 快照 / `first_difference()`，与定义层内容变化严格区分**；按 Q3 / Q5' 已拍板 `SNAPSHOT_VERSION` bump 7→8、golden hash 按 v8 新实现如实重取（R2 / ADR 0001 / ADR 0004 / ADR 0002 §6-8 / Q2 / Q3）

### Requirement: 开局按种族装配且不新增权威状态

`MeleeSetup` SHALL 依据种族参数生成对应 `DefId` 段的 `Spawn` 开局命令：本地席位用 `local_race` 段、AI 席位用 `ai_race` 段、中立资源恒 `9xx`，`with_races` 由 2×2 四组合扩到 **4×4 十六组合**（4 族 4×4、含同族组合；Q1' 已拍板双族全量 + P8 两族不重做）；未指定种族时 SHALL 回落人族段（R2 / R12.7 / ADR 0002）。玩家选族 MUST 只以装配层 `EntityId→DefId` 关联表达、`CommandKind::Spawn` 字节 MUST 逐字节不变、族别 MUST NOT 进入 `w3-core` 的 `Player` / `RtsMatch` 分类字段（Q6）。**删除历史断言**：本 requirement 旧正文逐期记载 P8.5a–d 的 bump 3→4→5→6→7 与「MUST NOT 沿用『零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态』旧断言」——该断言是各期实施期口径、不是对 P9 的耐久禁令。**用户 2026-11-22 已明确拍板 Q2「纳入权威状态全套」+ Q3「bump 8 全套」（偏离 AI 推荐、MUST NOT 写成 AI 推荐）**：族特色机制 MUST 新增 `w3-core` 顶层权威状态（`BTreeMap` / `BTreeSet` 确定性有序、稳定 `EntityId` 顺序）并四处同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION`（8）定义的快照与 `first_difference()`（ADR 0002 §6-8 / `openspec/config.yaml:56` / R5）；三版本号 bump 7→8、旧 v7 明确拒绝、golden hash 如实重取、determinism 26 项、proto tag 46 起、命令码 24 起、`EventKind` 变体（Q3 / Q5' 已拍板清单）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「不新增权威状态」「内核零 diff」历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 两族分别装配

- **WHEN** 分别以人族、兽族**与不死族、暗夜族**参数构造开局，并覆盖 4×4 十六组合（含同族组合）
- **THEN** 双方 `Spawn` 命令的 `DefId` 落在各自段内（`1xx` / `2xx` / `3xx` / `4xx`）、中立恒 `9xx`，资源与人口初值按既有冻结常量确定，`with_race` 兼容别名与 `MeleeSetup::new()` 逐字节不变，选族装配不引入 `race` 分类权威状态字段（R2 / R12.7 / Q1' / Q5' / Q6）；Q2 族特色机制以独立顶层权威状态段表达、不复用 race 字段

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的选族装配改动，以及 Q2 / Q3 已拍板口径对 `w3-core` 的影响
- **THEN** 历史标题字面「内核零 diff」是身份键、**不作本期行为断言**：选族装配本身不引入 `race` 分类字段，但按 Q2 已拍板新增族特色机制顶层权威状态 → **`git diff -- crates/w3-core/` 非空属预期**（`faction-definitions/spec.md:24,36` P8.5a–d 先例）；新增权威状态 MUST 四处同步进 `compute_state_hash` / `Snapshot` 顶层字段 / `first_difference()` / `restore_snapshot`、`SNAPSHOT_VERSION` bump 8、旧 v7 明确拒绝、golden hash 按 v8 新实现如实重取、`cargo test -p w3-core --test determinism`（26 项）全绿含「改坏验红」形态①（R12.4 / R7.5 / ADR 0002 §6-8 / `CODEBUDDY.md §5` / Q2 / Q3）；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言

#### Scenario: 回落兼容

- **WHEN** 既有 `playable` 合成入口不传种族参数
- **THEN** 行为与改动前一致，一局仍能装配并打完，且主城升级 / 野怪营地 / 清野 / 掉落与 P8.5d 物品 / 商店 / 回城在 `w3-core` 确定性命令面可用、逐帧哈希与 **v8 新实现基线**一致；新增不死 / 暗夜段与 Q2 机制不改变既有回落路径（R12.7 / R12.4 / R4.1 / R8.1 / R8.6 / R7.5 / R9.3 / Q3）
