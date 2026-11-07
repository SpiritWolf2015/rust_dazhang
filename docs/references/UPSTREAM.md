# 上游参考项目（只读）

本文件固定三个只读参考项目的**本地路径、提交与用途**。它们不是本仓库的子模块，代码不进入本仓库；借鉴边界见 [ADR 0001](../decisions/0001-license.md)、[ADR 0003](../decisions/0003-scope.md) 与 `NOTICE.md`。

## 固定参考版本

| 项目 | 本地路径 | 固定提交 | 上游 | 许可证 |
|---|---|---|---|---|
| `vera20k` | `C:/open_rts/vera20k` | `7942296fb` | https://github.com/YuriPlanet/vera20k | GPLv3 + Section 7 附加条款 |
| `godot_warcraft3` | `C:/open_rts/godot_warcraft3` | `18b7a70` | https://github.com/LiGameAcademy/godot_warcraft3 | MIT |
| `rts_kernel_cs` | `C:/open_rts/rts_kernel_cs` | `edb8790` | https://github.com/LiGameAcademy/rts_kernel_cs | MIT |

这些提交是 2026-11-07 记录时的本地 HEAD，仅用于定位参考；升级时显式更新本表并重新核对相关文档。

## 各自提供什么

- **`vera20k`**：确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律。其 `src/net/` 与 `services/relay-protocol` 是 P4 的设计参考；RA2 专属部分（gamemd 原生地址、INI 规则、SHP/VXL 资产、2D 等距渲染）不借鉴实现。注意其 `LEGAL.md` 附加条款：**只有真的复制其代码时**才随之进入（见 ADR 0001）。
- **`godot_warcraft3`**：WC3 数据 / 资产格式规格与合规模型。关键文档：`docs/data/PIPELINE.md`、`docs/data/WC3_ASSET_PATHS.md`、`docs/data/CONTENT_PACKS.md`、`docs/architecture/ASSET_LANES.md`、`docs/architecture/MAP_DATA.md`、`docs/design/asset-convert/`（MDX 骨骼与蒙皮、BLP、粒子 PE2、挂点拼装）。工具：`tools/mpq-extract`、`tools/slk-export`、`tools/map-parse`、`tools/asset-convert`。
- **`rts_kernel_cs`**：模拟内核契约（`docs/FRAMEWORK.md`、`docs/NAVIGATION.md`、`docs/MOVEMENT.md`）、移动 / 寻路 / 编队语义，以及游戏仓库 `docs/architecture/RTS_KERNEL.md` 的帧阶段与快照契约。注意其 `double` 数值路线与"不承诺跨平台确定性"**不被本项目采用**（见 [ADR 0002](../decisions/0002-determinism.md)）。

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

**当前状态（2026-11-07）**：三个目录只有 `.gitkeep` / `.gdignore`，说明本机尚未运行 `dev-setup` 流水线。P1 的真实数据需要先在上游仓库执行：

```bash
cd /c/open_rts/godot_warcraft3
node tools/dev-setup.mjs --game-dir "D:/soft/game/war3/1.27a" --profile game
```

这些目录里的 JSON / PNG / GLB 属于**从 MPQ 派生的资产**，一律不得提交进本仓库（`.gitignore` 已排除）；运行时通过 `config.toml` 的本机路径读取。本项目 P1 的读取器用合成的最小 fixture 做回归测试，不依赖真实导出文件。

## 合规

工具与解析代码入库；资产留在玩家本机；仓库不含任何暴雪资产或派生文件。上游依据：`godot_warcraft3/docs/data/LEGAL.md`；本项目依据：`README.md` 免责声明、[ADR 0001](../decisions/0001-license.md)。
