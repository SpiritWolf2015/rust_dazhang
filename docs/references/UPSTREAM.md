# 上游参考项目（只读）

本文件固定四个只读参考项目的**本地路径、提交与用途**。它们不是本仓库的子模块，代码不进入本仓库；借鉴边界见 [ADR 0001](../decisions/0001-license.md)、[ADR 0003](../decisions/0003-scope.md) 与 `NOTICE.md`。

## 固定参考版本

| 项目 | 本地路径 | 固定提交 | 上游 | 许可证 |
|---|---|---|---|---|
| `vera20k` | `C:/open_rts/vera20k` | `7942296fb` | https://github.com/YuriPlanet/vera20k | GPLv3 + Section 7 附加条款 |
| `godot_warcraft3` | `C:/open_rts/godot_warcraft3` | `18b7a70` | https://github.com/LiGameAcademy/godot_warcraft3 | MIT |
| `rts_kernel_cs` | `C:/open_rts/rts_kernel_cs` | `edb8790` | https://github.com/LiGameAcademy/rts_kernel_cs | MIT |
| `WarsmashModEngine` | `C:/open_rts/WarsmashModEngine` | `f9e0aeed4be372d6016519d0e97b384aa873f374` | https://github.com/Retera/WarsmashModEngine | AGPLv3 |

前三项是 2026-11-07 记录时的本地 HEAD，`WarsmashModEngine` 是 2026-11-08 克隆时的本地 HEAD（提交日期 2025-12-08）。这些提交仅用于定位参考；升级时显式更新本表并重新核对相关文档。

## 各自提供什么

- **`vera20k`**：确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律。其 `src/net/` 与 `services/relay-protocol` 是 P4 的设计参考；RA2 专属部分（gamemd 原生地址、INI 规则、SHP/VXL 资产、2D 等距渲染）不借鉴实现。注意其 `LEGAL.md` 附加条款：**只有真的复制其代码时**才随之进入（见 ADR 0001）。
- **`godot_warcraft3`**：WC3 数据 / 资产格式规格与合规模型。关键文档：`docs/data/PIPELINE.md`、`docs/data/WC3_ASSET_PATHS.md`、`docs/data/CONTENT_PACKS.md`、`docs/architecture/ASSET_LANES.md`、`docs/architecture/MAP_DATA.md`、`docs/design/asset-convert/`（MDX 骨骼与蒙皮、BLP、粒子 PE2、挂点拼装）。工具：`tools/mpq-extract`、`tools/slk-export`、`tools/map-parse`、`tools/asset-convert`。
- **`rts_kernel_cs`**：模拟内核契约（`docs/FRAMEWORK.md`、`docs/NAVIGATION.md`、`docs/MOVEMENT.md`）、移动 / 寻路 / 编队语义，以及游戏仓库 `docs/architecture/RTS_KERNEL.md` 的帧阶段与快照契约。注意其 `double` 数值路线与"不承诺跨平台确定性"**不被本项目采用**（见 [ADR 0002](../decisions/0002-determinism.md)）。
- **`WarsmashModEngine`**：Java / LibGDX 实现的 WC3 模拟器，含 JASS / 触发器解析（`jassparser/`）、FDF / 界面解析、MPQ 与 CASC 数据源、完整单位 / 技能 / 玩法行为。作为 JASS 语义、地图与 UI 格式、玩法行为的**只读参考**（ADR 0003 所列远期项）。其许可证为 **AGPLv3**，与本项目 GPLv3 不同：只借鉴设计，**不得复制代码**；若将来需要引入其代码，须按 [ADR 0001](../decisions/0001-license.md) 复核点重新评估许可证兼容性，不得直接并入。

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

工具与解析代码入库；资产留在玩家本机；仓库不含任何暴雪资产或派生文件。四个参考项目均为只读、不复制代码；其中 `WarsmashModEngine` 为 AGPLv3，与本项目 GPLv3 组合有条件，仅作设计参考。上游依据：`godot_warcraft3/docs/data/LEGAL.md`；本项目依据：`README.md` 免责声明、[ADR 0001](../decisions/0001-license.md)。

## 按任务类型的强制参考矩阵（工作流内建）

> 本节是**工作流规则**，不是背景资料。`reference-researcher` 与各 owner 在动笔 / 动手前 MUST 按本表定位参考；`spec-steward` MUST 把对照结论写进 `design.md` 的「参考项目对照」小节；`critic` 按清单第 12 项复核。所有参考均为**只读**，路径与固定提交见本文开头表格；不复制代码（ADR 0001）。项目目标为复刻经典 WC3，参考项目是语义与格式的权威来源，**不得凭记忆或「我觉得 WC3 是这样」下结论**。

### 任务类型 → 参考项目

| # | 任务类型 | 主参考 | 具体路径（相对项目根） | 借鉴什么 | 禁止 / 边界 | 负责 agent |
|---|---|---|---|---|---|---|
| T1 | 确定性内核 / lockstep / 状态哈希 / 命令排序 / 快照 | `vera20k` | `src/net/`、`services/relay-protocol/`、`docs/` | 定点数、命令队列、全状态哈希、帧编排、crate 拆分与依赖纪律 | RA2 专属部分（gamemd 原生地址、INI、SHP/VXL、2D 等距）不借鉴实现；GPLv3 + Section 7 附加条款**只有真的复制代码时**才进入（ADR 0001） | `sim-owner` / `net-owner` |
| T2 | 移动 / 寻路 / 编队 / 帧阶段语义 | `rts_kernel_cs` | `docs/FRAMEWORK.md`、`docs/MOVEMENT.md`、`docs/NAVIGATION.md`、`src/`、`tests/` | 移动积分、网格 A\*、编队落点、帧阶段与只读视图契约 | **`double` 数值路线与「不承诺跨平台确定性」不采用**（ADR 0002）；C# 实现不复制 | `sim-owner` / `client-owner` |
| T3 | WC3 数据格式 / SLK / 地图解析 / 资产管线 | `godot_warcraft3` | `docs/data/PIPELINE.md`、`WC3_ASSET_PATHS.md`、`CONTENT_PACKS.md`、`docs/architecture/ASSET_LANES.md`、`MAP_DATA.md`、`docs/design/asset-convert/`、`tools/mpq-extract|slk-export|map-parse|asset-convert/` | 42 张 SLK 表 schema、地图 JSON 维度、MDX 骨骼 / 蒙皮、BLP、粒子 PE2、挂点拼装、三车道（视觉 / 数据 / 地图）分工 | 资产与派生 JSON / PNG 一律不入库（ADR 0001）；Godot / GDScript 与 C# 桥接实现不复制 | `content-owner` / `asset-pipeline` |
| T4 | 玩法语义 / JASS / 技能 / 单位行为 / FDF UI / 经典规则 | `WarsmashModEngine` | `jassparser/`、`fdfparser/`、`core/assets/abilityBehaviors/*.json`、`resources/Scripts/Abilities/{Human,Orc}/`、`resources/Scripts/melee.jui`、`core/src/com/etheller/warsmash/units/` | 按种族 / 英雄分组的技能行为结构、单位 / 科技 / 物品数据分层、Melee 初始化动作表、复活 / 经验等经典规则语义 | **AGPLv3，只借鉴设计语义、不得复制任何代码**；引入代码须按 ADR 0001 复核点重新评估；Java / LibGDX 依赖不引入 | 全体（尤其 P8 两族完整化） |
| T5 | 场景装配 / 相机 / HUD / Melee 初始化 / 3D 呈现 | `godot_warcraft3` | `packages/{foundation,content,gameplay,map,rts_content}/`、`apps/game/`、`docs/design/game/{ROADMAP,GAMEPLAY_VERTICAL,HUD,COMBAT_SYSTEM,UNIT_AI,ABILITY_SYSTEM,BUILD_SYSTEM}.md`、`docs/roadmap/{NEXT,ROADMAP,CLASSIC_RULES_BASELINE,ITEMS_AND_MELEE_AI_PLAN}.md`、`docs/design/game/FACTION_UNIT_ROSTERS.md` | A→D→F→E 阶段节奏、F0–F10 人族竖切顺序、Melee Bootstrap 原生动作表、N1 / N2 / N4 收尾与第二种族最小集、U0–U3 灰盒批次与「每族两种开局 + 两种主力组合」判据、数值读 Def 不写死 | 其**原创三族**单位名 / 数值（`FACTION_UNIT_ROSTERS.md`）不是经典人族 / 兽族复刻，只借结构与批次判据；`double` 路线不采用 | `client-owner` / `spec-steward` |
| T6 | 网络传输 / UDP 可靠层 / relay 广播重传 | `vera20k` | `src/net/`、`services/relay-protocol/` | 可靠层、断线重连、帧编排、序列号与校验设计 | 本项目自有 `W3N1` 25 字节帧头与 protobuf payload 契约（ADR 0005）为准，不照搬其线格式 | `net-owner` |

### 触发时机（不是「想起来才查」）

1. **提案阶段**（`spec-steward` 委派 `reference-researcher`）：凡涉及 T1–T6 的 change，MUST 在写 `design.md` 前产出「参考对照报告」——本次参考了哪个项目的哪部分、借鉴了什么、明确不借鉴什么（含许可证理由），每条结论带 `file:line` 或参考项目内可核验路径锚。
2. **成文**：`design.md` MUST 有「## 参考项目对照」小节，逐条引用报告结论；`decision-memo.md` 的承重约束若来自参考项目，锚 MUST 指向本矩阵条目或参考项目内可核验路径。
3. **实现阶段**（各 owner）：遇到玩法 / 格式 / 内核语义疑问时，MUST 先按本表定位参考再动手；不得凭记忆下结论。
4. **复审**（`critic` 清单第 12 项）：核对照是否真实、锚能否打开、是否误把 AGPLv3 代码或 `double` 路线带进来、是否把参考项目的原创设定冒充经典复刻。
5. **参考项目也没有答案**：如实登记「参考缺口」，回到用户拍板，不臆造。

### 维护

- `docs-owner` 维护本矩阵与 `NOTICE.md` 的参考登记；参考项目升级（固定提交变化）时按本文开头表格显式更新并重新核对相关文档。
- 新增任务类型或参考路径，MUST 同步本表、`CODEBUDDY.md` §9 与 `reference-researcher` 定义；**不允许只在某次会话里口头引用**（这正是「说一次查一次」要杜绝的）。
