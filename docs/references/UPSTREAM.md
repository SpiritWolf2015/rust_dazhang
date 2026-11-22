# 上游参考项目（只读）

本文件固定五个只读参考项目的**本地路径、提交与用途**。它们不是本仓库的子模块，代码不进入本仓库；借鉴边界见 [ADR 0001](../decisions/0001-license.md)、[ADR 0003](../decisions/0003-scope.md) 与 `NOTICE.md`。

> **区分**：本目录另有 [`WC3-MELEE-RULES.md`](WC3-MELEE-RULES.md)——那是本项目**自己整理的 WC3 对战业务规则基线**（1.0 范围与 `R-x.y` 规则编号），**不是上游参考项目**。玩法实现 / review / 单测引用该文的规则编号；参考项目对照仍按本文件的「按任务类型的强制参考矩阵」执行。两者互补：`reference-researcher` 查「参考项目怎么做」，业务规则文档定「系统该做什么」（`CODEBUDDY.md` §9）。

## 固定参考版本

| 项目 | 本地路径 | 固定提交 | 上游 | 许可证 |
|---|---|---|---|---|
| `vera20k` | `C:/open_rts/vera20k` | `305ea3a75` | https://github.com/YuriPlanet/vera20k | GPLv3 + Section 7 附加条款 |
| `godot_warcraft3` | `C:/open_rts/godot_warcraft3` | `18b7a70` | https://github.com/LiGameAcademy/godot_warcraft3 | MIT |
| `rts_kernel_cs` | `C:/open_rts/rts_kernel_cs` | `edb8790` | https://github.com/LiGameAcademy/rts_kernel_cs | MIT |
| `WarsmashModEngine` | `C:/open_rts/WarsmashModEngine` | `f9e0aeed4be372d6016519d0e97b384aa873f374` | https://github.com/Retera/WarsmashModEngine | AGPLv3 |
| `ConcordRTS` | `C:/open_rts/ConcordRTS` | `32ab245` | https://github.com/AdrianLuo-X/ConcordRTS.git | GPL-3.0-only（GNU GPL v3 标准全文，无 Section 7 附加条款） |

`godot_warcraft3` 与 `rts_kernel_cs` 是 2026-11-07 记录时的本地 HEAD，`WarsmashModEngine` 是 2026-11-08 克隆时的本地 HEAD（提交日期 2025-12-08），三者均未变动；`vera20k` 已于 2026-11-17 升级复核后更新（见下「2026-11-17 升级复核」小节）。这些提交仅用于定位参考；升级时显式更新本表并重新核对相关文档。`ConcordRTS` 是 2026-11-22 准入复核时记录的本地 HEAD（末次提交 2026-11-05），详见下「2026-11-22 准入复核」小节。

### 2026-11-17 升级复核（vera20k）

- **变更**：`vera20k` 本地 HEAD 由 `7942296fb`（2026-11-07，PR #4035）前进至 `305ea3a75a3a146f1a24cc0d6eeef0f19fa6015f`（2026-11-17，PR #4566）；旧提交是新 HEAD 的**祖先**，属正常前进、非换分支。区间含 531 个 PR、3324 文件变更。另三个参考项目 HEAD 与开头「固定参考版本」表格一致、未变动。
- **路径复核**：T1 / T6 引用的 `src/net/`（`lockstep.rs` / `reliable.rs` / `command_exchange.rs` / `event_packet.rs` 等）、`services/relay-protocol/`（`src/link/`、`src/envelope.rs`、`src/journal.rs`）、`docs/Networking.md`、`LEGAL.md`、`LICENSE-GPL`、`docs/` 在新 HEAD 下全部仍存在；矩阵 T1 / T6 条目的「具体路径」列与「各自提供什么」段中 `vera20k` 条目的引用继续有效，**无需改动**。
- **许可证复核**：`LEGAL.md` 新增 `## Code from Ares` / `## Code from Phobos` 两节（对应 `LICENSES/Ares.txt`、`docs/phobos.md`），即上游引入 Ares / Phobos 第三方代码来源；Section 7 附加条款与「只有真的复制其代码时才随之进入」的结论未变。本项目**不复制** `vera20k` 代码，按 ADR 0001 该变化**不影响本项目合规姿态**，「各自提供什么」段的结论不变。若将来按 `ADR 0001:19` 与 `vera20k` 交换代码，MUST 先按 ADR 0001 复核点重新评估新增的 Ares / Phobos 第三方代码许可兼容性；本次不涉及代码交换，仅登记。
- **相关文档核对结论**：`NOTICE.md:17`（不含提交号、借鉴内容仍成立）、`openspec/config.yaml:21`（只列项目名与许可证）、`docs/decisions/0001` / `0002` / `0003`（历史决策记录、语义结论未变）、`docs/references/WC3-MELEE-RULES.md`（2026-11-17 的 T3 / T4 / T5 只读调研未从 `vera20k` 取证）、`openspec/changes/archive/` 下含 `vera20k` 引用的归档文件（历史快照）——**均无需改动**。
- **证据锚标注**：`ADR 0001:14` 的 `README.md:142` 行号基于 2026-11-07 提交；新 HEAD `README.md` 为 154 行、行号可能偏移，但引用仍指向 `LEGAL.md` 法律声明、语义未变。按「ADR 记为什么决定、不删改原决策」保留原锚，**不据本次升级改写任何 ADR**。

### 2026-11-22 准入复核（ConcordRTS）

用户 2026-11-22 拍板「一次性全量固化」，把与本项目同级（`C:/open_rts/`，本表体例内的本机记录）的 `ConcordRTS` 作为**第五个只读参考项目**加入文档与 subagent 工作流，并新增矩阵任务类型 **T7**。以下为只读核实的第一手事实与证据锚，不作二次推断。

- **来源与固定提交**：真实 git 仓库，上游 `https://github.com/AdrianLuo-X/ConcordRTS.git`；本地 HEAD = `origin/main` = `32ab245784f21492d1981f57223baf519a13444f`（短 `32ab245`），共 22 提交，作者 `AdrianLuo-XH`，末次提交 2026-11-05。与现有四项目同级；固定提交见本文开头表格。
- **定位**：`README.md:1-5`——「使用 C++20 开发的 RTS 引擎工程，面向《命令与征服：红色警戒 2》及《尤里的复仇》（RA2/YR）的规则、地图和资源格式」；「公开作者自行编写的引擎源码、构建工具、测试代码与示例，不附带原版游戏程序或官方素材」。
- **许可证结论**：`LICENSE` 为 **GNU GPL v3 标准全文、无 Section 7 附加条款**；`README.md:760-768`「版权与源码许可」明确 `SPDX: GPL-3.0-only`，并声明 RA2/YR 名称 / 商标 / 原版程序及素材归各自权利人、非官方工程、源码许可不授予原版游戏或素材的使用 / 复制 / 再分发许可。**与本项目 GPLv3 同许可族**；按 ADR 0001 只读参考、不复制代码、不引入依赖、不作 git 子模块即可合规，**无需走 `ADR 0001:66-73` 复核点重评**（与 AGPLv3 的 `WarsmashModEngine` 不同）。
- **零资产边界**：全仓无 `*.shp` / `*.vxl` / `*.tmp` / `*.mix` / `*.mpq` / `*.wav` / `*.png` / `*.bmp` / `*.blp` / `*.mdx` / `*.glb`；`README.md:304,401,756-758` 明确自编 INI / MAP 仅测试夹具、原版数据目录与素材经 `.gitignore` 排除、官方素材 / EXE / DLL / MIX 提取内容不属发布范围。
- **来源声明处置**：`README.md:5` 作者自行编写；`README.md:78` 有「清理相关历史外部引用、注释和诊断表述」记录。以作者声明为准，本项目只读不复制；按 `UPSTREAM.md:22` 对 `vera20k` 引入 Ares / Phobos 第三方代码的先例，该记录**不影响本项目合规姿态、仅登记**。**`README.md` 更新记录中「开发验收完成 N 项」「逐字节一致」等均为作者自述，本项目 MUST NOT 当作已验证事实或跨平台确定性已达成的证据**；ADR 0002 的跨平台确定性仍属待验证项，不得据其声称已达成。
- **借鉴与 RA2 边界**：借的是**通用 RTS 引擎架构 / 确定性内核与表现层分离 / 平台适配与跨平台逐字节验收方法论 / 锁步·快照·回放·视野工程实践**，不是 WC3 语义或格式权威。**RA2/YR ≠ WC3**：SHP / VXL / TMP / MIX / INI 格式、RA2 规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染绝不作 WC3 玩法 / 格式 / 数值 / 规则的事实来源；WC3 业务规则仍以 [`WC3-MELEE-RULES.md`](WC3-MELEE-RULES.md) 为唯一权威（与本文「各自提供什么」段 `vera20k` 的「RA2 专属部分不借鉴实现」措辞一致）。
- **可核验锚清单（已只读打开）**：`src/core/determinism/RuntimeAuthorityHashPrimitives.h`、`src/network/lockstep/RuntimeMultiplayerLockstep.{h,cpp}`、`src/network/snapshots/RuntimeMultiplayerSnapshotSession.{h,cpp}`、`src/render/visibility/`（shroud / vision 全套）、`src/runtime/contracts/RuntimeReplayEventSnapshot.h`、`API.md:296` §保存、回放封包与恢复 / `:310` §共享生产会话与平台接口 / `:492` §TCP 边界与联机快照会话 / `:713` §确定性候选任务 / `:753` §视野与战斗反应、`README.md:22-25`（无窗口核心与完整游戏复用同一套模拟 / 命令 / 保存恢复、确定性哈希与权威更新顺序、Windows / Linux 逐字节一致验收方法）。
- **本轮同步范围（11 个文件，一次性全量固化于同一提交）**：`CODEBUDDY.md`、本文（`UPSTREAM.md`）、`NOTICE.md`、`README.md`，以及 `.codebuddy/agents/` 下 `reference-researcher.md` / `docs-owner.md` / `net-owner.md` / `sim-owner.md` / `spec-steward.md` / `critic.md` 六个定义，加 `openspec/config.yaml`（由 `spec-steward` 同步参考项目 context 与 `rules.design` 措辞）。分两棒落盘、合计 11 个文件一次性全量固化；agent 定义只同步参考项目数量、矩阵编号与边界措辞，不改各 owner 的职责 / tools / 可写范围结构。
- **明确不改清单与已知限制（如实登记，不冒充已全量对齐）**：下列文件残留「四个 / 四者」措辞，按用户拍板由后续独立 change / 对应 change 收口，本轮一律零改动；本轮只固化上列 11 个文件，不宣称全仓措辞已全量对齐：
  - `docs/decisions/0001-license.md`（`:10`「四个参考项目的许可证」）——ADR 记「为什么决定」、不删改原决策，沿用 `UPSTREAM.md:24`「不据复核改写任何 ADR」先例；后续独立 change 收口。
  - `docs/references/WC3-MELEE-RULES.md`（`:5,354,373` 的「四个参考项目」措辞）——活文档，按 §14 与历期先例由后续独立 change 收口；本轮不改规则、不改编号、不改状态标记。
  - `docs/roadmap/P8-human-orc-melee.md:140`（历史沿革）——后续独立 change 收口。
  - `openspec/changes/archive/**`（归档历史快照）——历史快照不回填，后续独立 change 收口。
  - **`openspec/changes/p9-undead-foundation/**`（整个目录一律零改动）**：用户刚拍板「暂停、不锁定」的 active change，其 `design.md:91` 含「四个参考项目」措辞；改四件套正文会改变规格门指纹 `910530f229120e90d4fed27795543f3b8070bc23eecf7d92c3f82660bc02f107`，触发 `CODEBUDDY.md` §8 规格解锁重走规格门，与用户拍板冲突。**登记为「该 change 后续收口」**，本轮零改动。
  - `crates/**`、CI 配置、`.codebuddy/settings.json`、`tools/**`、`.gitignore`、`Cargo.lock`——本轮零改动（`openspec/config.yaml` 已由 `spec-steward` 在本轮同步，不在不改清单内）。
  - 历期先例：不为凑数全量改写历史文件（Q10 先例），残留措辞如实标注为已知限制。

## 各自提供什么

- **`vera20k`**：确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律。其 `src/net/` 与 `services/relay-protocol` 是 P4 的设计参考；RA2 专属部分（gamemd 原生地址、INI 规则、SHP/VXL 资产、2D 等距渲染）不借鉴实现。注意其 `LEGAL.md` 附加条款：**只有真的复制其代码时**才随之进入（见 ADR 0001）。
- **`godot_warcraft3`**：WC3 数据 / 资产格式规格与合规模型。关键文档：`docs/data/PIPELINE.md`、`docs/data/WC3_ASSET_PATHS.md`、`docs/data/CONTENT_PACKS.md`、`docs/architecture/ASSET_LANES.md`、`docs/architecture/MAP_DATA.md`、`docs/design/asset-convert/`（MDX 骨骼与蒙皮、BLP、粒子 PE2、挂点拼装）。工具：`tools/mpq-extract`、`tools/slk-export`、`tools/map-parse`、`tools/asset-convert`。
- **`rts_kernel_cs`**：模拟内核契约（`docs/FRAMEWORK.md`、`docs/NAVIGATION.md`、`docs/MOVEMENT.md`）、移动 / 寻路 / 编队语义，以及游戏仓库 `docs/architecture/RTS_KERNEL.md` 的帧阶段与快照契约。注意其 `double` 数值路线与"不承诺跨平台确定性"**不被本项目采用**（见 [ADR 0002](../decisions/0002-determinism.md)）。
- **`WarsmashModEngine`**：Java / LibGDX 实现的 WC3 模拟器，含 JASS / 触发器解析（`jassparser/`）、FDF / 界面解析、MPQ 与 CASC 数据源、完整单位 / 技能 / 玩法行为。作为 JASS 语义、地图与 UI 格式、玩法行为的**只读参考**（ADR 0003 所列远期项）。其许可证为 **AGPLv3**，与本项目 GPLv3 不同：只借鉴设计，**不得复制代码**；若将来需要引入其代码，须按 [ADR 0001](../decisions/0001-license.md) 复核点重新评估许可证兼容性，不得直接并入。
- **`ConcordRTS`**（GPL-3.0-only，C++20 的 RA2/YR RTS 引擎工程，矩阵 T7）：无窗口核心与完整游戏复用同一套模拟 / 命令 / 快照恢复契约、确定性哈希与权威更新顺序、平台适配层私有实现、跨平台逐字节一致**验收方法论**、视野 / 迷雾表现层（shroud / vision）、显式失败 / 软件回退工程实践；锁步 / 快照 / 回放封包的 C++ 实现对照（`src/network/lockstep/`、`src/network/snapshots/`、`src/runtime/contracts/`）。**仅作通用 RTS 引擎架构与工程实践的只读参考**：RA2/YR 专属格式（SHP / VXL / TMP / MIX / INI）、规则 / 地图 / 单位 / 科技 / 数值 / 键位、战役、Westwood Online、2D 等距渲染不借鉴，**绝不作 WC3 玩法 / 格式 / 数值 / 规则的事实来源**（WC3 业务规则仍以 `WC3-MELEE-RULES.md` 为唯一权威）；`README.md` 自述「开发验收完成 N 项」「逐字节一致」不作本项目已验证事实，不得据其声称跨平台确定性已达成（ADR 0002 待验证）；只读、不复制代码、不引入依赖、不作 git 子模块。准入复核与证据锚见上「2026-11-22 准入复核」小节。

## 本机游戏客户端

- 路径：`D:/soft/game/war3/1.27a`（经典 MPQ 客户端，非 Reforged/CASC）。
- 实测归档：`war3.mpq`（**注意小写 w**）、`War3x.mpq`、`War3xLocal.mpq`、`War3Patch.mpq`；**无 `Data/` 目录**，符合 `godot_warcraft3/README.md:13` 与 `PIPELINE.md:49` 的要求。
- 未发现 `War3Local.mpq`（`CONTENT_PACKS.md:29` 的覆盖顺序含它）。若 P1 的 SLK 导出出现基础单位本地化字符串缺失，先查这里。
- 跨平台注意：Windows 大小写不敏感，但 Rust 读取 MPQ / 路径时须**大小写不敏感匹配**（已写入 `config.toml.example` 注释）。

## 导出数据（仓库外，不提交）

| 目录（本机） | 内容 |
|---|---|
| `C:/open_rts/godot_warcraft3/assets/slk-exported/` | SLK 表 JSON、UnitFunc / Strings、UI txt |
| `C:/open_rts/godot_warcraft3/assets/map-parsed/<slug>/` | `summary.json`、`info.json`、`terrain.json`、`terrain-heightfield.json`、`units.json`、`doodads.json`、`strings.json` 等 |
| `C:/open_rts/godot_warcraft3/assets/asset-converted/` | PNG / GLTF / `.scn` / PathTextures / Sound |

**当前状态（2026-11-07）**：已运行 `dev-setup` 的 `slk` 与 `maps` 步骤，生成 **42 张 SLK 表**与 **Echo Isles 地图 JSON**（129×97 顶点、97 单位、2491 装饰物）。`extract` 步骤因单个 `Units/Undead/Ghoul/GhoulWarcry1.wav` 解包失败而以 exit 2 退出（上游把任何单文件失败当致命错误），但 17,490 个文件已成功写入 staging，因此用 `--only` 跳过 extract 完成导出：

```bash
cd /c/open_rts/godot_warcraft3
# 首次：install + extract（extract 会报 1 个错误，数据已写入 staging）
node tools/dev-setup.mjs --game-dir "D:/soft/game/war3/1.27a" --only install --only extract --only slk --only maps --maps echoisles
# 之后重导 SLK / 地图（跳过 extract）：
node tools/dev-setup.mjs --game-dir "D:/soft/game/war3/1.27a" --only slk --only maps --maps echoisles
```

端到端验证（本机，`w3-content/examples/load_export.rs`）：

```bash
cd /c/open_rts/rust_dazhang
cargo run -p w3-content --example load_export -- \
  "C:/open_rts/godot_warcraft3/assets/map-parsed/echoisles" \
  "C:/open_rts/godot_warcraft3/assets/slk-exported/Units/UnitBalance.json"
```

结果：`heightfield 129x97 = 12513 vertices`、`units 97`、`doodads 2491`、`UnitBalance 61 cols / 836 rows`，P1 读取器无需改动即可解析真实 schema。

这些目录里的 JSON / PNG / GLB 属于**从 MPQ 派生的资产**，一律不得提交进本仓库（`.gitignore` 已排除）；运行时通过 `config.toml` 的本机路径读取。本项目 P1 的回归测试仍用合成 fixture，不依赖真实导出文件。

## 合规

工具与解析代码入库；资产留在玩家本机；仓库不含任何暴雪资产或派生文件。五个参考项目均为只读、不复制代码；其中 `WarsmashModEngine` 为 AGPLv3，与本项目 GPLv3 组合有条件，仅作设计参考。`ConcordRTS` 为 GPL-3.0-only（GNU GPL v3 标准全文、无 Section 7 附加条款），与本项目 GPLv3 **同许可族**，按 ADR 0001 只读参考、不复制代码、不引入依赖、不作 git 子模块即可合规，无需 ADR 复核点重评；其仓库零游戏资产，RA2/YR 名称 / 商标 / 原版程序及素材归各自权利人，源码许可不覆盖原版内容（上游依据 `ConcordRTS/README.md:760-768`，详见「2026-11-22 准入复核」小节）。上游依据：`godot_warcraft3/docs/data/LEGAL.md`；本项目依据：`README.md` 免责声明、[ADR 0001](../decisions/0001-license.md)。

## 按任务类型的强制参考矩阵（工作流内建）

> 本节是**工作流规则**，不是背景资料。`reference-researcher` 与各 owner 在动笔 / 动手前 MUST 按本表定位参考；`spec-steward` MUST 把对照结论写进 `design.md` 的「参考项目对照」小节；`critic` 按清单第 12 项复核。所有参考均为**只读**，路径与固定提交见本文开头表格；不复制代码（ADR 0001）。项目目标为复刻经典 WC3，参考项目是语义与格式的权威来源。**`ConcordRTS` 是 RA2/YR 引擎，仅作通用 RTS 引擎架构与工程实践的只读参考，不是 WC3 语义或格式的权威来源**；**不得凭记忆或「我觉得 WC3 是这样」下结论**。

### 任务类型 → 参考项目

| # | 任务类型 | 主参考 | 具体路径（相对项目根） | 借鉴什么 | 禁止 / 边界 | 负责 agent |
|---|---|---|---|---|---|---|
| T1 | 确定性内核 / lockstep / 状态哈希 / 命令排序 / 快照 | `vera20k` | `src/net/`、`services/relay-protocol/`、`docs/` | 定点数、命令队列、全状态哈希、帧编排、crate 拆分与依赖纪律 | RA2 专属部分（gamemd 原生地址、INI、SHP/VXL、2D 等距）不借鉴实现；GPLv3 + Section 7 附加条款**只有真的复制代码时**才进入（ADR 0001） | `sim-owner` / `net-owner` |
| T2 | 移动 / 寻路 / 编队 / 帧阶段语义 | `rts_kernel_cs` | `docs/FRAMEWORK.md`、`docs/MOVEMENT.md`、`docs/NAVIGATION.md`、`src/`、`tests/` | 移动积分、网格 A\*、编队落点、帧阶段与只读视图契约 | **`double` 数值路线与「不承诺跨平台确定性」不采用**（ADR 0002）；C# 实现不复制 | `sim-owner` / `client-owner` |
| T3 | WC3 数据格式 / SLK / 地图解析 / 资产管线 | `godot_warcraft3` | `docs/data/PIPELINE.md`、`WC3_ASSET_PATHS.md`、`CONTENT_PACKS.md`、`docs/architecture/ASSET_LANES.md`、`MAP_DATA.md`、`docs/design/asset-convert/`、`tools/mpq-extract|slk-export|map-parse|asset-convert/` | 42 张 SLK 表 schema、地图 JSON 维度、MDX 骨骼 / 蒙皮、BLP、粒子 PE2、挂点拼装、三车道（视觉 / 数据 / 地图）分工 | 资产与派生 JSON / PNG 一律不入库（ADR 0001）；Godot / GDScript 与 C# 桥接实现不复制 | `content-owner` / `asset-pipeline` |
| T4 | 玩法语义 / JASS / 技能 / 单位行为 / FDF UI / 经典规则 | `WarsmashModEngine` | `jassparser/`、`fdfparser/`、`core/assets/abilityBehaviors/*.json`（部分按种族命名、分组不对称：兽族仅 `orcHeroActives.json`）、`resources/Scripts/Abilities/{Human,Orc}/`、`core/src/com/etheller/warsmash/units/manager/MutableObjectData.java`（`urac` / `arac` / `frac` / `grac` race 字段）、`resources/Scripts/melee.jui`（**JASS UI 帧定义，仅作 FDF / UI 格式参考**） | 按种族 / 英雄分组的技能行为结构（部分按种族命名）、单位 / 科技 / 物品数据分层与 race 分类字段、复活 / 经验等经典规则语义；**Melee 初始化动作表语义不在此仓库固定提交内**，见 `godot_warcraft3/docs/design/game/ROADMAP.md` §0 / §2-C | **AGPLv3，只借鉴设计语义、不得复制任何代码**；引入代码须按 ADR 0001 复核点重新评估；Java / LibGDX 依赖不引入 | 全体（尤其 P8 两族完整化） |
| T5 | 场景装配 / 相机 / HUD / Melee 初始化 / 3D 呈现 | `godot_warcraft3` | `packages/{foundation,content,gameplay,map,rts_content}/`、`apps/game/`、`docs/design/game/{ROADMAP,GAMEPLAY_VERTICAL,HUD,COMBAT_SYSTEM,UNIT_AI,ABILITY_SYSTEM,BUILD_SYSTEM}.md`、`docs/roadmap/{NEXT,ROADMAP,CLASSIC_RULES_BASELINE,ITEMS_AND_MELEE_AI_PLAN}.md`、`docs/design/game/FACTION_UNIT_ROSTERS.md` | A→D→F→E 阶段节奏、F0–F10 人族竖切顺序、Melee Bootstrap 原生动作表、N1 / N2 / N4 收尾与第二种族最小集、U0–U3 灰盒批次与「每族两种开局 + 两种主力组合」判据、数值读 Def 不写死 | 其**原创三族**单位名 / 数值（`FACTION_UNIT_ROSTERS.md`）不是经典人族 / 兽族复刻，只借结构与批次判据；`double` 路线不采用 | `client-owner` / `spec-steward` |
| T6 | 网络传输 / UDP 可靠层 / relay 广播重传 | `vera20k` | `src/net/`、`services/relay-protocol/` | 可靠层、断线重连、帧编排、序列号与校验设计 | 本项目自有 `W3N1` 25 字节帧头与 protobuf payload 契约（ADR 0005）为准，不照搬其线格式 | `net-owner` |
| T7 | 通用 RTS 引擎架构 / 确定性内核与表现层分离 / 平台适配与跨平台逐字节验收方法论 | `ConcordRTS` | `src/core/determinism/RuntimeAuthorityHashPrimitives.h`、`src/network/lockstep/`、`src/network/snapshots/`、`src/runtime/contracts/`、`src/render/visibility/`、`API.md` §保存、回放封包与恢复 / §共享生产会话与平台接口 / §TCP 边界与联机快照会话 / §确定性候选任务 / §视野与战斗反应、`README.md` 更新记录 | 无窗口核心与完整游戏复用同一套模拟/命令/快照恢复契约；确定性哈希与权威更新顺序；平台适配层私有实现与跨平台逐字节一致验收方法论；视野/迷雾表现层；显式失败/软件回退工程实践 | **RA2/YR 专属**：SHP/VXL/TMP/MIX/INI 格式、RA2 规则/地图/单位/科技/数值/键位、战役、Westwood Online、2D 等距渲染绝不作 WC3 玩法/格式/数值/规则的事实来源（WC3 业务规则仍以 `WC3-MELEE-RULES.md` 为唯一权威）；`README.md` 自述「开发验收完成 N 项」不得当本项目已验证事实；GPL-3.0-only 与本项目 GPLv3 同族，只读、不复制代码、不引入依赖、不作子模块 | `sim-owner` / `net-owner` / `client-owner` |

> **交叉参考注记**：T1 / T2 / T6 可**可选交叉参考** T7 `ConcordRTS` 的 `src/core/determinism/`、`src/network/lockstep/`、`src/network/snapshots/` 作为 C++ 实现对照，但各自的主参考与禁止边界不变；T3 / T4 / T5（WC3 数据格式 / 玩法语义 / 场景 HUD）MUST NOT 以 `ConcordRTS` 为参考。

### 触发时机（不是「想起来才查」）

1. **提案阶段**（`spec-steward` 委派 `reference-researcher`）：凡涉及 T1–T7 的 change，MUST 在写 `design.md` 前产出「参考对照报告」——本次参考了哪个项目的哪部分、借鉴了什么、明确不借鉴什么（含许可证理由），每条结论带 `file:line` 或参考项目内可核验路径锚。
2. **成文**：`design.md` MUST 有「## 参考项目对照」小节，逐条引用报告结论；`decision-memo.md` 的承重约束若来自参考项目，锚 MUST 指向本矩阵条目或参考项目内可核验路径。
3. **实现阶段**（各 owner）：遇到玩法 / 格式 / 内核语义疑问时，MUST 先按本表定位参考再动手；不得凭记忆下结论。
4. **复审**（`critic` 清单第 12 项）：核对照是否真实、锚能否打开、是否误把 AGPLv3 代码或 `double` 路线带进来、是否把参考项目的原创设定冒充经典复刻；T7 命中时另核 `ConcordRTS` 边界——是否把 RA2/YR 专属格式 / 规则 / 数值冒充 WC3 玩法 / 格式 / 数值 / 规则的事实来源、是否把 `README.md` 自述「开发验收完成 N 项」「逐字节一致」当本项目已验证事实、是否误把 GPL-3.0-only 当作需 ADR 复核点重评的 AGPLv3 类项目。
5. **参考项目也没有答案**：如实登记「参考缺口」，回到用户拍板，不臆造。

### 维护

- `docs-owner` 维护本矩阵与 `NOTICE.md` 的参考登记；参考项目升级（固定提交变化）时按本文开头表格显式更新并重新核对相关文档。
- 新增任务类型或参考路径，MUST 同步本表、`CODEBUDDY.md` §9 与 `reference-researcher` 定义；**不允许只在某次会话里口头引用**（这正是「说一次查一次」要杜绝的）。
- 新增参考项目时，MUST 同步「准入复核」小节（来源 / 固定提交 / 许可证 / 零资产 / 来源声明 / 借鉴与 RA2 边界 / 可核验锚清单 / 明确不改清单与已知限制）、11 个文件（`CODEBUDDY.md`、`UPSTREAM.md`、`NOTICE.md`、`README.md`、六个 agent 定义共 10 个，由 `docs-owner` 落盘；加 `openspec/config.yaml` 由 `spec-steward` 同步）与矩阵任务类型，并在 `NOTICE.md` 登记许可证。残留「四个 / 四者」措辞按「准入复核」小节的不改清单如实登记为已知限制、由后续独立 change 收口，不冒充已全量对齐。
