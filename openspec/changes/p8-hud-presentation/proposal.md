# Proposal

## Why

承接 1.0 范围 **S1**（局域网联机对战，含真人 VS AI 敌人玩家）/ **S3**（4 个种族功能做完）（`docs/references/WC3-MELEE-RULES.md` §0），落实路线图 `docs/roadmap/P8-human-orc-melee.md` §3 的 **P8.4「HUD 与表现层」**（:112-117）。用户 2026-11-18 经 `AskUserQuestion` 拍板 **P8.4 HUD 选族（推荐）**；同日规格门「规格批准（按推荐）」：Q1 窗口内 egui overlay、Q2 `[待核]` 合成占位、Q4 真窗口人工验收不替代按推荐通过，**Q3 拍板「本期就上真实两族 glTF」**（按 §8 同步回查后第二次拍板批准并锁定）。apply 代码已落盘；`critic` 代码门判 1 个规格一致性 blocker（B1：D3 / tasks 5.1 / Req1「按 `EntityId → DefId` 关联解析」措辞与段级派生实现不符，且该机制在 `w3-core` 零 diff 下不可能实现），用户 2026-11-18 拍板「解锁规格修订措辞」（§9）→ 本轮四件套 + 纪要一次成文同步回查修订机制措辞（Q3 决策不变）、`status` 回到 `pending` 重走规格门、待用户再拍一次。主 `openspec/specs/dual-faction/spec.md:106/:108-111` 把三项遗留移交后续独立 change——**本 change 正是承接者**。现状三处缺口：`examples/melee.rs:98` 硬编码 `MeleeSetup::new()`、HUD 无选族入口；`command_card.rs:235-360` 训练卡目标全写死 `HUMAN_*`；运行时新实体回落 `"default"` 退化为四面体（归档 `2026-11-14-melee-playable-loop/design.md:149`）。兽族 glTF 本机可得、零入库（详见「## 资产侧前置事实」）。依据 R1.4、R2、R4、R13.1 / R13.3 / R13.6、§14；守 ADR 0001 / 0002 / 0003 / 0004 / 0005。

## What Changes

- **选族入口（表现层）**：在 HUD / 输入层提供开局前本地族 / AI 族的可见选择（Q1 已拍板：**窗口内 egui overlay**），选族结果是**表现层本地只读状态**，经装配期把 `(local_race, ai_race)` 传给既有 `MeleeSetup::with_races`；MUST NOT 新增 `Player.race` 等权威状态字段。选族后开局 `def_ids` 分段正确、既有 `MeleeSetup::new()` / `with_race` 入口逐字节不变（R2 / R12.7）。
- **命令卡按族查表分派**：把 `command_card.rs` 写死的 `HUMAN_*` 目标改为**按本地族只读查表**分派 Producer / Build / Train 卡的 `TrainUnit { unit_def, .. }`（人族 `1xx` / 兽族 `2xx`），兽族建筑训练 MUST 产兽族目标；MUST NOT 散落 `if race`、MUST NOT 在 Present 层判规则、MUST NOT 用 id 前缀推断；`CommandCatalog` 保持只读、明确默认回落。Q2 已拍板：**沿用 `[待核]` 合成原创键位 / 图标占位，不冻结进 `FrozenDefinitions`、不入库、不冒充原版**。**不是重做 R13 命令卡**（既有 12 格 / 两级 chords / 键位解耦契约原样保留）。
- **Q3 已拍板：本期就上真实两族 glTF**（决策不变，代码门 B1 后 §9 解锁修订**机制措辞**）：运行时 `Train` / `Build` 动态新实体按只读 `EntityView.owner`（1 本地 / 2 AI / 其它中立）→ `FactionSelection` 族段（`1xx` / `2xx` / `9xx`）+ `EntityView.speed==0` 判建筑 → 编译期常量 `SEGMENT_MODEL_KEYS` 段代表键 → `ModelCatalog::lookup` → `BakedModel` → `Scene::expand_model` 真实 glTF 管线；补齐 `w3-content::ENTITY_MODEL_FILES` 两族键表（**仅新增表项、不改解析逻辑、不改权威状态**，兽族 8 键用小写逻辑路径命中真实大小写磁盘文件，`normalize()` 已消除大小写风险）。因 `w3-core` 零 diff / 不扩展视图字段下无法获得运行时精确 `DefId`，MUST NOT 声称「按 `EntityId → DefId` 关联解析」；**R-1 加宽已知限制**如实登记：8 键仅 2 键运行时可达、6 键烘焙不可达、8 类静默取段代表键（错误类别）真实网格 `reason=None` 无诊断，MUST NOT 写成「显式可诊断回退已落地」（用户 2026-11-18 拍板接受）。**本机资产缺失 / 无绑定路径时**显式、可诊断回退（英文结构化 `key=value` 日志 + 明确默认），MUST NOT 静默降级；CI 无本机资产只能验「段级派生 + 缺资产可诊断回退」，**不得声称已验证真实网格画面**，真窗口人工验收（Q4）如实标注未执行。静态摆放链路不变。
- **CPU 回归与反向验证**：新增 / 修改回归 MUST 证明缺陷态失败、修复态通过（测试内反例元断言或 A-B 对照，`CODEBUDDY.md` §5），至少覆盖「选族退化为硬编码」「命令卡退化回写死 `HUMAN_*`」「模型键退化回一律四面体」「缺资产时静默降级」；重跑 `tests/determinism.rs`（26 项）。
- **离屏自证 + 真窗口人工验收如实标注**：先以 `render_headless_rgba` 离屏 readback + CPU 回归全绿自证（本机有资产时可自证真实网格），再请用户人工验收；真窗口项如实标注「未执行 / 待人工」、不冒充通过、不提供替代（路线图 :117 / 主 `playable-window-entry/spec.md:149-151/:168-176`）。
- **BREAKING：无**。既有命令面、既有公开 API、既有主 spec 的 requirement 正文均不改写；`crates/w3-core/` 零 diff、`SNAPSHOT_VERSION` 仍 3；`crates/w3-content/` 本期有**纯表现层键表**改动（`ENTITY_MODEL_FILES` 仅新增逻辑路径字符串表项，非资产内容、非解析逻辑、非权威状态），不影响权威状态 / 状态哈希 / 快照 / 回放字节（ADR 0001 / ADR 0002 / ADR 0004）。

## Capabilities

### New Capabilities

- `hud-faction-selection`: 表现层选族入口（Q1 窗口内 egui overlay）与装配期传参的只读契约——选族 / HUD 状态不进权威状态、开局 DefId 分段正确、确定性契约零冲击、承接 `dual-faction` 遗留、范围边界与真窗口人工验收如实标注。
- `runtime-entity-model-binding`: Q3 拍板的真实两族 glTF 契约（代码门 B1 后 §9 解锁修订措辞）——运行时动态实体按 `EntityView.owner` 族段 + `speed==0` 判建筑 → 段代表键 → `ENTITY_MODEL_FILES` 两族键表 → 真实 glTF 网格，含大小写归一命中、本机资产缺失 / 无绑定路径显式可诊断回退、R-1 加宽已知限制、CI 无资产口径、零资产入库与 `w3-core` 零 diff 边界。

### Modified Capabilities

- `rts-command-hud`: 增加一条 **ADDED Requirement**「命令卡按本地族查表分派」；既有 7 条 requirement（含 `:99-101` 的 12 格定义表 / 键位解耦、`:73` 中文上屏留后续、`:123-126` 键位 `[待核]`）**不改写**。Q2 已拍板沿用 `[待核]` 合成占位。选既有路径而非新建近似 capability：命令卡归该 capability 所有，按族分派是其新增行为，避免近重复命名（`openspec instructions proposal` 明确要求复用既有路径，先例见归档 `2026-11-17-p9-command-hotkeys`）。

## 资产侧前置事实（asset-pipeline 只读诊断 + 补跑，2026-11-18 已核实）

> 以下均为**本机 / 只读参考项目**侧事实，锚为 `file:line` 或命令输出；资产**一律留 gitignored 本机、不入库**（ADR 0001）。`reference-researcher` 对照报告之外的资产事实由 `asset-pipeline` 产出、leader 转达，`spec-steward` 逐锚复核。

1. **根因（已证实，非推测）**：`godot_warcraft3/tools/asset-convert/scripts/convert-echo-isles.mjs:13-72`（尤其 `:44-60`）的 **49 条** `--include` 只覆盖 `Buildings/Human/**`、`Units/Human/**`、`Units/Creeps/**`、`Buildings/Other/**`、Lordaeron 地形与 UI，**不含 `Units/Orc/**` / `Buildings/Orc/**`**；`tools/dev-setup.mjs:345-350` 默认 `--profile game` 固定调该脚本；`docs/data/PIPELINE.md:253` 明写「默认 ≈ Echo Isles + 人族 Melee 子集」。**不是 extract 数据缺失**：`assets/.staging/wc3-assets/` 17,490 文件、manifest 逐条 sha256，`Units/Orc/` 56 个 `.mdx`、`buildings/Orc/` 10 个（含 `grunt.mdx` / `peon.mdx` / `GreatHall.mdx` / `OrcBarracks.mdx`）；`UPSTREAM.md:48` 的 `GhoulWarcry1.wav`（Undead 音频）失败与兽族模型无因果关系。
2. **补跑命令**（cwd `C:/open_rts/godot_warcraft3/tools/asset-convert`，上游既有 CLI、**未改上游源码**）：`node src/cli.js --in "../../assets/.staging/wc3-assets" --include "Units/Orc/**" --include "Buildings/Orc/**" --models-only --skip-scn --skip-clean --skip-passthrough` → `转换 64, 跳过 2, 错误 0`，EXIT 0（`src/cli.js:58-63/:107-122` 确认 flags 存在）。**未跑** `dev-setup.mjs --only convert`（`:63/:159-161/:177-180` 的 `--only` 不透传 `--include`）；`--profile full` 会全量重转 3264 个 MDX 约 30 分钟，不推荐。
3. **补跑结果**：全库 `.gltf` 922 → **988**；新增 **66** 个兽族 `.gltf` + **66** 个 `.bin`（`Units/Orc` 56 + `buildings/Orc` 10）、零字节文件 0；`gltf=66 valid2.0=66 withMesh=66 missingBin=0 missingImageUri=0`。P8.4 八个键全部生成且非空（磁盘真实大小写，字节数）：`Units/Orc/Grunt/grunt.gltf`(477124B)、`Units/Orc/Peon/peon.gltf`(786114B)、`Units/Orc/HeadHunter/Headhunter.gltf`(515014B)、`buildings/Orc/GreatHall/GreatHall.gltf`(809460B)、`buildings/Orc/OrcBarracks/OrcBarracks.gltf`(278415B)、`buildings/Orc/AltarOfStorms/AltarofStorms.gltf`(443903B)、`buildings/Orc/WarMill/WarMill.gltf`(278191B)、`buildings/Orc/TrollBurrow/TrollBurrow.gltf`(143596B)；另有 58 个额外兽族网格。人族 5 键 + `.bin` + 两张 PNG 的 sha256 逐字节未变；`load_export` 端到端 EXIT 0（`heightfield 129x97`、`units 97`、`doodads 2491`、`UnitBalance 61 cols / 836 rows`）。`rust_dazhang` 工作树仍只有 `?? openspec/changes/p8-hud-presentation/`，**无任何资产入库**；上游工作树干净。
4. **大小写风险已被现有代码消除（不是待修缺陷）**：`crates/w3-content/src/model_index.rs:109-111` 的 `normalize()`（`to_ascii_lowercase()` + `\`→`/`）在 `scan_gltf():290` 对磁盘 stem 归一、`:296` `by_path.insert(key, path)` 记录真实磁盘路径、在 `lookup():189` 对输入 `file` 归一，`by_path` 键与查询键都是全小写 → `ENTITY_MODEL_FILES` 用小写逻辑路径（如 `units/orc/grunt/grunt`）即可命中 `Units/Orc/Grunt/grunt.gltf`，大小写敏感文件系统也成立；`lookup` 返回的 `PathBuf` 是真实磁盘路径。**不需要改归一逻辑**。
5. **格式口径**：上游产物是 `.gltf` + 同目录外链 `.bin` / PNG，**不是 GLB**；`docs/data/PIPELINE.md:91` 与 `tools/dev-setup.mjs:395` 的 "GLB" 文案过时（`tmp/convert-report.md:158` 已登记）；`dev-setup --profile game` 不含兽族，需显式 `--include`。
6. **未验证项（如实标注）**：未验证 GPU 端加载新 gltf 后的画面（只验证文件存在 / 非空 / JSON 可解析 / buffers-images URI 可解析）；未跑 extract、未生成 `.scn`（无 Godot）、未转 `Abilities/Spells/Orc/**` 特效；`UPSTREAM.md:48` 的 extract 已知失败仍未修复、未绕过；真窗口人工验收仍是 Q4「未执行 / 待人工」。

## Impact

- **代码面 / 双 owner**：**撤销**上一版「仅 `client-owner` 单 owner / `w3-content` 零 diff / `ENTITY_MODEL_FILES` 扩表留后续 change」（Q3 用户拍板真实两族 glTF，按 `CODEBUDDY.md §8` 同步回查）。改为 `client-owner`（`crates/w3-client/`、`crates/w3-render/`）+ `content-owner`（**仅** `crates/w3-content/src/model_index.rs` 的 `ENTITY_MODEL_FILES` 表项新增，不改 `normalize()` / `lookup()` / `scan_gltf()` 解析逻辑、不改权威状态）双 owner；`crates/w3-core/` **零 diff**（`git diff -- crates/w3-core/` 为空）；`w3-net` / `w3-relay` / `Cargo.lock` 零改动、不引新 crate / 新依赖 / `[[bin]]`。
- **确定性契约（ADR 0002）**：`w3-core` 零改动；30 Hz 整数帧、命令稳定排序键 `ExecuteFrame → PlayerId → Sequence → Arrival`、稳定 `EntityId` 顺序、FNV-1a 状态哈希、`first_difference()` 字段集合、`SNAPSHOT_VERSION`（当前 **3**）、golden hash 逐字节不变；**不新增 `Player.race` 权威状态**；选族 / HUD / 命令卡 / 模型键全为表现层只读派生，MUST NOT 进权威状态 / 状态哈希 / 由 `SNAPSHOT_VERSION` 定义的快照。`ENTITY_MODEL_FILES` 表项不参与 `defs_hash` / `content_hash`（`crates/w3-client/examples/melee.rs:38` 固定 `CONTENT_HASH` 常量 + `FrozenDefinitions` 冻结定义）。本 change 无新增权威状态字段，故无需扩展哈希 / 快照字段集合；若将来新增 MUST 同步进入二者并按 `CODEBUDDY.md` §5 跑 `determinism`。
- **资产 / 法律（ADR 0001 / ADR 0004）**：两族定义仍是合成 / 原创编译期整数常量；真实 glTF / `.bin` / PNG 一律留 gitignored 本机 `config.toml` 路径、不入库；`ENTITY_MODEL_FILES` 新增的只是**逻辑路径字符串**（合成键表），不是资产内容；本机导出由 `asset-pipeline` 前置完成（命令 / 8 键清单见「## 资产侧前置事实」）；`w3-content` 原始 `f64` 坐标不进哈希 / 快照；不引入暴雪 rawcode、参考项目原创单位名与数值；`WarsmashModEngine`（AGPLv3）仅借鉴语义、禁止复制代码；`rts_kernel_cs` 的 `double` 路线不采用。
- **玩法规则编号（MUST 引用）**：proposal / specs / design / tasks 统一引用 `WC3-MELEE-RULES.md` §0（S1 / S3）、R1.4、R2、R4、R13.1 / R13.3 / R13.6、§14；数值 / 键位 / 图标语义一律 `[待核]`、不冻结进 `FrozenDefinitions`、不入库（§14 不静默改规则）。N1–N6 不纳入；联机 / 回放 / AI 玩家 / 迷雾是已在能力或 1.0 范围、不是不做项。
- **阶段与 ADR**：阶段 **P8.4**（路线图 §3 :112-117）；`openspec/config.yaml` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注（与已归档 P8.0 / P8.1 / P8.3 一致）。相关 ADR：0001（零资产 / 许可证）、0002（确定性）、0003（范围与分期）、0004（冻结坐标）、0005（联机线格式与分层边界）。
- **范围外承接（后续独立 change）**：P8.5 五个深度玩法子系统、不死 / 暗夜两族（P9+）、中文上屏 / CJK 字体许可证、JASS / wtg VM、联机底座重做、AI × lockstep 接合（§0.3 T5）、真窗口人工验收的替代、兽族特效模型 / `.scn` 烘焙 / 全量 MDX 转换；本 change 不扩范围、不冒充已解决。
- **人工边界 / 规格解锁（§9）**：代码门 B1「规格一致性 blocker」→ 用户 2026-11-18 拍板「解锁规格修订措辞」→ 四件套 + 纪要**一次成文同步回查修订措辞**（Q3 决策不变）→ 重走规格门；规格门 `status` 回到 **`pending`**（重走门 fail-closed 正确信号），不保留 `approved`、不回填新指纹、不 self-approve；`verify_all.py::spec_gate_check()` 在 pending 下变红，不绕过、不改门禁；本阶段不 apply、不 commit、不 archive、**不改 `crates/` 任何文件**（代码门已过、代码不动）、不改主 `openspec/specs/` / 归档目录。`critic` 重出报告 → 用户再拍一次 → 重算指纹 → commit 由后续流程承接。

## Scope

- **做**：① 表现层选族入口（Q1 窗口内 egui overlay）→ 传参 `MeleeSetup::with_races`；② 命令卡按本地族查表分派（Q2 `[待核]` 合成占位）；③ **Q3 真实两族 glTF**：键表按族段扩两族（`ENTITY_MODEL_FILES` 表项新增）+ 运行时新实体按 `owner` 族段 + `speed` 判建筑的段级派生 + 修缺键回退（段键未命中 / 无绑定路径显式可诊断回退）+ R-1 加宽已知限制如实登记；④ CPU 回归与反向验证（改坏验红，含缺资产静默降级退化）；⑤ 离屏自证（本机有资产时自证真实网格）；⑥ 资产前置复核 / 零入库复核 / CI 无资产口径；⑦ `verify_all.py` 门禁（§9 解锁重走门、用户再拍一次后）；⑧ 真窗口人工验收清单（Q4 如实标注未执行、不冒充通过）。本轮交付边界：四件套 + 纪要同步回查修订**文档措辞**，`crates/` 代码不动。
- **不做**：中文上屏 / CJK 字体、JASS VM、联机底座重做、真窗口人工验收的替代、P8.5 五个子系统、不死 / 暗夜两族；不改 `w3-core`、不改主 `openspec/specs/` 正文；不把真实 glTF / `.bin` / PNG / MPQ 派生数据 / 导出 JSON 入库。
- **本阶段交付边界**：仅产出 `openspec/changes/p8-hud-presentation/` 下的四件套 + `decision-memo.md`；不 apply、不 commit、不 archive、不 push / fetch / pull / clone。主 `openspec/specs/` **不变化**（归档时才合并 delta）。
