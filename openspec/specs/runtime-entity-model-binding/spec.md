# runtime-entity-model-binding Specification

## Purpose
为 P8.4 Q3 拍板的真实两族 glTF 模型绑定定义行为契约：`Train` / `Build` 产出的运行时动态实体 SHALL 按只读 `EntityView.owner`（1 本地 / 2 AI / 其它中立）→ `FactionSelection` 族段（`1xx` 人族 / `2xx` 兽族 / `9xx` 中立）+ `EntityView.speed==0` 判建筑 → 编译期常量 `SEGMENT_MODEL_KEYS` 段代表键 → `ModelCatalog::lookup` → `BakedModel` → `Scene::expand_model` 真实 glTF 网格，而非一律回落 `"default"` 四面体；含大小写归一命中、本机资产缺失 / 无绑定路径时显式可诊断回退、CI 无资产口径、R-1 加宽已知限制如实登记、真实资产零入库与 `w3-core` 零 diff 边界（ADR 0001 / ADR 0002 / ADR 0004）。机制措辞按代码门 B1 → `CODEBUDDY.md §9` 用户 2026-11-18 拍板「解锁规格修订措辞」同步回查修订，Q3 决策不变。

> 相关 ADR：ADR 0001（真实 glTF / `.bin` / PNG 一律留 gitignored 本机 `config.toml` 路径、不入库；`ENTITY_MODEL_FILES` 新增的只是**逻辑路径字符串**合成键表，不是资产内容；`WarsmashModEngine` AGPLv3 仅借鉴语义不复制代码）；ADR 0002（模型键 / 几何 / 日志为表现层只读派生，不进状态哈希 / 快照 / `first_difference()`，`SNAPSHOT_VERSION` 仍 3，`crates/w3-core/` 零改动）；ADR 0004（`w3-content` 原始 `f64` 坐标不进哈希 / 快照，静态摆放冻结边界）；ADR 0005（呈现管线接缝不变）。业务规则依据 `docs/references/WC3-MELEE-RULES.md` §0（S1 / S3）、R1.4、R2、R4、R13.6、§14；根因见归档 `2026-11-14-melee-playable-loop/design.md:149` 与路线图 `P8-human-orc-melee.md:74/:115`；资产侧锚见 `proposal.md`「## 资产侧前置事实」与 `design.md` D3 / 「## 资产侧前置事实」。

## Requirements

### Requirement: 运行时新实体按族段与单位/建筑粒度派生真实两族 glTF 模型键

运行时 `Train` / `Build` 新实体 SHALL 按只读 `EntityView.owner`（1 本地 / 2 AI / 其它中立）→ `FactionSelection` 族段（`1xx` / `2xx` / `9xx`）+ `EntityView.speed==0` 判建筑 → 编译期常量 `SEGMENT_MODEL_KEYS` 段代表键 → `ModelCatalog::lookup` → `BakedModel` → `Scene::expand_model` 真实 glTF 网格；MUST NOT 一律回落 `"default"` 四面体（R2 / R4 / C9）。`crates/w3-core/` 零 diff、不扩展视图字段下 `EntityView` 无 `def_id`、`complete_production` 不发 `Spawned`，无法获得运行时精确 `DefId`，故 MUST NOT 声称「按精确 `DefId` 关联解析」（C21）。解析 MUST NOT 进权威状态 / 状态哈希 / 快照（ADR 0002）。

#### Scenario: 族段 → 段代表真实模型键的映射与烘焙（R2 / R4 / C16–C18 / C21）

- **WHEN** 以人族 / 兽族各装配并枚举 `SEGMENT_MODEL_KEYS` 段表、`ENTITY_MODEL_FILES` 表项与 `BakedModel` 烘焙结果
- **THEN** 人族 `1xx` / 兽族 `2xx` / 中立 `9xx` 三段各有 1 个单位段代表键 + 1 个建筑段代表键（人族 `soldier` / `town-hall`、兽族 `orc-grunt` / `orc-great-hall`、中立 `tree` / `gold-mine`）；`ENTITY_MODEL_FILES` 14 键（人族 5 + 中立 1 + 兽族 8）经 `model_assembly.rs:304-311` 全部烘焙为真实网格（`real_map_e2e.rs:235-263` 严格 `assert_eq!(...,14)` + 逐键 `triangle_count()>0`）；兽族 8 键磁盘产物非空且 `asset.version=2.0`、含 mesh、`missingBin=0` / `missingImageUri=0`（补跑「转换 64 跳过 2 错误 0」EXIT 0 / 全库 `.gltf` 922→988 / 新增 66 兽族网格，锚 `proposal.md`「## 资产侧前置事实」第 2 / 3 条）。**烘焙全命中不代表运行时全可达**，见下方 R-1 加宽 Scenario

#### Scenario: 大小写归一命中真实大小写磁盘文件（C18，不是待修缺陷）

- **WHEN** 以小写逻辑路径 `units/orc/grunt/grunt` 查询 `ModelCatalog::resolve_entity_key` → `lookup`
- **THEN** `crates/w3-content/src/model_index.rs:109-111` 的 `normalize()`（`to_ascii_lowercase()` + `\`→`/`）已在 `scan_gltf():290` 对磁盘 stem 归一、`:296` `by_path.insert(key, path)` 记录真实磁盘路径、在 `lookup():189` 对输入 `file` 归一，`by_path` 键与查询键都是全小写 → 小写逻辑路径命中 `Units/Orc/Grunt/grunt.gltf`，大小写敏感文件系统也成立；`lookup` 返回的 `PathBuf` 是真实磁盘路径。**MUST NOT 改归一逻辑**、不写成待修缺陷（C18）

#### Scenario: 兽族运行时新实体呈现真实 glTF 网格而非四面体（R2 / R4 / C9 / C21）

- **WHEN** 以本地兽族开局并经 `Train` / `Build` 产出新实体，且本机 gitignored `config.toml` 指向含 8 键的 `asset-converted/`
- **THEN** `resolve_with_reason`（`model.rs:582-609`）按 `EntityView.owner=1` → `FactionSelection` 兽族 `2xx` 段 + `speed==0` 判建筑 → 段代表键（`orc-grunt` / `orc-great-hall`）→ `BakedModel` → `Scene::expand_model` 呈现真实兽族 glTF 网格，MUST NOT 因回落 `"default"` 变成四面体；人族开局新实体同理按 `1xx` 段命中 `soldier` / `town-hall`。两族段代表键不同 → Q3 验收层口径达成（C7 / C9）

#### Scenario: 静态摆放链路保持不变（ADR 0004 / C7）

- **WHEN** 复核 `units.json` / `doodads.json` 静态摆放的解析路径
- **THEN** 仍走 `ModelCatalog::resolve_unit` / `resolve_doodad_paths` 预展开（`crates/w3-content/src/model_index.rs:201-258`），不改写静态摆放解析顺序、不引入 staging / `.cache` 回退（godot `ASSET_LANES.md:53-58`）；动态实体链路本次只**新增 `ENTITY_MODEL_FILES` 表项**，`resolve_entity_key` / `lookup` / `scan_gltf` 解析逻辑零改动

#### Scenario: R-1 加宽：八类走段代表键、6 键不可达、reason=None 为已知限制（C21 / C22，用户 2026-11-18 拍板接受）

- **WHEN** 核对 R-1 八类 `ORC_SIEGE`(206) / `ORC_HERO`(210) + 人族六类 `HUMAN_RANGED/SIEGE/FARM/ALTAR/SMITHY/HERO` 的运行时解析，以及 8 个兽族键的生产可达性
- **THEN** 如实登记为**加宽的已知限制**：① 8 个兽族键中仅 `orc-grunt` / `orc-great-hall` 两个段代表键在生产解析路径可达，其余 6 个（`orc-peon` / `orc-headhunter` / `orc-barracks` / `orc-burrow` / `orc-altar` / `orc-war-mill`）烘焙为真实网格却永不被实例解析到；② 上述 8 类运行时静默取得所属段代表键的（错误类别）真实网格、`ModelResolution.reason == None`、无 `reason=model_key_missing` 诊断（`model.rs:613-628`）；③ `def_segment` / `def_id_to_model_key` / `R1_UNMAPPED_DEFS` 精确 `DefId → 模型键` 查表零生产调用者（仅测试，`runtime_models.rs:135-191`）；④ `real_map_e2e.rs:235-263` 的 `assert_eq!(...,14)` 只证明烘焙、不证明运行时可达。**MUST NOT 写成「R-1 八类已走显式可诊断回退」或「8 键覆盖 8/10 兽族 DefId」**；精确类别级真实 / 缺键需内核只读视图或事件携带 DefId、触发规格解锁重走门，由后续独立 change 承接

### Requirement: 缺键 / 本机资产缺失时显式、可诊断回退

模型 / 资源键解析在缺键或**本机资产缺失**时 MUST 显式、可诊断、回落到明确默认（godot `command_card.gd:863-893` 缺 UI 行显示 id、`construction_profile_catalog.gd:22-26` 未知回落 human、`wc3_game_cursor.gd:219/:243-246` 缺文件 `push_error`、`ASSET_LANES.md:53-58` 解析顺序结束）：缺键 / 缺资产 SHALL 以英文结构化 `key=value` 日志记录并回落到明确默认几何 / 明确默认族键，MUST NOT 静默降级、MUST NOT 写死中文名 / 图标表、MUST NOT 只在「无本机资产」时才可诊断（C9）。`"default"` 兜底 SHALL 保留但改为显式可诊断的最后兜底，MUST NOT 作为运行时新实体的常规路径。

#### Scenario: 段键未命中库 / 无席位绑定时显式可诊断回退、不静默降级（C9 / Q3 拍板 / CODEBUDDY.md §3 / C22）

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境（CI runner）装配运行时实体，或令 `resolve_with_reason` 走 ③ 既有队伍绑定缺失 / ④ `"default"` 最后兜底分支、段代表键未插入库
- **THEN** 以英文结构化 `key=value` 日志明确报告 `REASON_ASSET_MISSING`（`reason=asset_missing`，段键未命中库）或 `REASON_MODEL_KEY_MISSING`（`reason=model_key_missing`，落到 `"default"`）、回落到明确默认几何 / 默认族键，不静默吞掉、不写死中文名 / 图标表；模拟推进与逐帧状态哈希不变（ADR 0002）；MUST NOT 声称已验证真实网格画面（C12）。**该可诊断回退只覆盖 ③④ 兜底路径，不覆盖 R-1 八类静默取段代表键（`reason=None`）**，后者见上方 R-1 加宽 Scenario

#### Scenario: 回退不是「仅无资产时」才发生（C7 / C9）

- **WHEN** 复核归档 `2026-11-14-melee-playable-loop/design.md:149` 登记的「运行时新实体缺键回退」根因
- **THEN** 承认该回退在有本机资产时也因运行时实体不在 `MeleeSetup::model_keys()` 的 12 个开局绑定内而发生；本 change 修的是这条常规路径，MUST NOT 把「无资产」当唯一触发条件，MUST NOT 冒充「2588 静态 + 6 类型键全命中即每个运行时实例都是真实网格」

#### Scenario: CI 无本机资产不冒充验证真实网格（C12 / Q3 拍板 / C21）

- **WHEN** CI 在 `ubuntu-latest` / `windows-latest` 无 gitignored 导出目录跑编译与 CPU 测试
- **THEN** 只验「族段 → 段代表键映射表项 + 段级派生（owner / speed）+ 缺资产 / 无绑定路径显式可诊断回退 + `w3-core` 零 diff / 哈希快照不变」，MUST NOT 声称 CI 已验证真实 glTF 网格画面或 GPU 窗口链路，MUST NOT 声称按精确 `DefId` 关联解析；真实网格观感仍由真窗口人工验收（Q4，如实标注未执行）承接（主 `playable-window-entry/spec.md:168-176` / 路线图 :117）

### Requirement: 表现层只读与确定性契约零冲击（双 owner 边界）

本 capability 的所有解析 / 几何 / 日志 SHALL 只从只读 `WorldView`（`EntityView.owner` / `speed`）、事实事件、只读 `Snapshot` 与表现层本地只读 `FactionSelection` + 编译期 `SEGMENT_MODEL_KEYS` 段表派生；MUST NOT 声称依赖运行时精确 `EntityId → DefId` 关联（仅覆盖 frame-0 12 个实体，C21）、MUST NOT 反写模拟状态、MUST NOT 进 FNV-1a 状态哈希 / 由 `SNAPSHOT_VERSION`（当前 3）定义的快照 / `first_difference()`（ADR 0002）。`crates/w3-core/` MUST 零改动、不新增权威状态字段；`crates/w3-content/` 本期**仅** `ENTITY_MODEL_FILES` 表项新增（`content-owner`），其余为双 owner 边界（C13）。

#### Scenario: 内核零 diff 与哈希快照不变（ADR 0002 / C1）

- **WHEN** 以两族各装配并打完若干整数帧后校验内核与快照头部
- **THEN** `git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍 3、状态哈希字段集合 / 快照头部布局 / `first_difference()` / FNV-1a / golden hash 逐字节不变；`cargo test -p w3-core --test determinism`（26 项）与 `crates/w3-render/tests/isolation.rs` 全绿；`ENTITY_MODEL_FILES` 表项不进 `defs_hash` / `content_hash`（`crates/w3-client/examples/melee.rs:38` 固定 `CONTENT_HASH`）；本 change 不新增权威状态字段故无需扩展哈希 / 快照字段集合，若将来新增 MUST 同步进入二者并跑 `determinism`（C1）；`w3-content` 仅表项新增、解析逻辑 / 权威状态零改动（C13 撤销「单 owner / `w3-content` 零 diff」）

#### Scenario: 模型键 / 几何不反写权威状态（ADR 0002 / R13.6）

- **WHEN** 运行时新实体连续生成、模型键刷新、相机与选中集合变化并推进若干逻辑帧
- **THEN** 逐帧 `compute_state_hash()` 与 `capture_snapshot()` 字节与无 HUD 实例一致，实体位置 / 生命 / 归属不因呈现变化而改变（主 `rts-command-hud/spec.md:87` / `melee-match/spec.md:115`）

#### Scenario: 双 owner 与真实资产零入库边界（C4 / C13 / ADR 0001 / Q3 拍板）

- **WHEN** 核对本 capability 的改动文件清单与 `git status`
- **THEN** 仅 `crates/w3-client/` / `crates/w3-render/`（`client-owner`）+ `crates/w3-content/src/model_index.rs` 的 `ENTITY_MODEL_FILES` 表项（`content-owner`）；`w3-core` / `w3-net` / `w3-relay` / `Cargo.lock` 零改动；仓库无任何 MPQ 派生数据 / 真实 glTF / `.bin` / PNG / 导出 JSON / 音效入库（代码门 `git status --short` = 12 个 `M` 源码/测试 + 3 个新文件 + `?? openspec/changes/p8-hud-presentation/`，资产扫描全绿），真实 glTF 一律留 gitignored 本机 `config.toml` 路径（ADR 0001 / `CODEBUDDY.md §6` / `proposal.md`「## 资产侧前置事实」第 3 条）。本轮 §9 解锁只改 change 目录下文档、不碰 `crates/`

### Requirement: CPU 回归、离屏自证与反向验证

新增 / 修改的模型绑定回归 MUST 证明缺陷态失败、修复态通过（`CODEBUDDY.md` §5），用「测试内反例元断言」或 A-B 对照；退化实现 MUST 真经生产解析路径注入、非旁路模拟。至少覆盖「退化回一律 `"default"` → 四面体」「段键未命中 / 无绑定路径时静默降级」两类退化；仅「全绿」不算完成。本 capability SHALL 先以 `render_headless_rgba` 离屏 readback + `ViewImage` 自证（本机有资产时自证真实两族网格），再补 CPU 回归与 `verify_all.py` 门禁（§9 解锁重走门、用户再拍一次后）。

#### Scenario: 退化回一律四面体即失败（CODEBUDDY.md §5 / C11）

- **WHEN** 把运行时新实体解析退化为「始终回落 `"default"`、走 `append_unit_marker`」
- **THEN** 「兽族新实体按 `2xx` 段解析真实 glTF 网格」断言在缺陷态失败、正式实现通过，两条证据可复现；退化注入真经 `Scene` / `ModelLibrary::resolve` 生产路径

#### Scenario: 缺资产静默降级退化即失败（CODEBUDDY.md §5 / C11 / Q3 拍板）

- **WHEN** 把缺资产回退退化为「静默吞掉、不写 `key=value` 日志」
- **THEN** 「本机资产缺失时显式可诊断回退」断言在缺陷态失败、正式实现通过；退化真经 `ModelCatalog::lookup` 生产路径注入，两条证据可复现

#### Scenario: 离屏自证真实两族网格（C12 / Q3 拍板 / C21）

- **WHEN** 本机 gitignored `config.toml` 指向含 8 键的 `asset-converted/`，以 `render_headless_rgba` 离屏渲染人族 / 兽族各自运行时新实体并 readback
- **THEN** `ViewImage` 目视确认两族段代表键对应实体为**真实 glTF 网格**且可区分、不再是同一四面体占位；截图落在 gitignored 本机路径、不入库（主 `playable-window-entry/spec.md:206-218` / ADR 0001）；GPU 端画面未验证项如实登记（`proposal.md`「资产侧前置事实」第 6 条 / C20）；MUST NOT 声称已验证精确类别级真实网格（R-1 加宽，C22）

#### Scenario: 门禁与真窗口人工验收如实标注（C12 / C15）

- **WHEN** 代码门 B1 触发 §9 解锁、四件套同步回查修订措辞后重走规格门，用户再拍一次并回填新指纹后运行 `python tools/verify_all.py` 与 `openspec validate --strict`
- **THEN** 编译与 CPU 侧门禁全绿；真窗口 GPU 清单项仍如实标注「未执行 / 待人工」、不冒充通过，CI MUST NOT 声称已验证 GPU 窗口链路或真实网格画面；`status: pending`（解锁重走门）期间 `spec_gate_check()` 变红属 fail-closed 正确信号、不绕过、不改门禁、不 self-approve（C12 / C15）
