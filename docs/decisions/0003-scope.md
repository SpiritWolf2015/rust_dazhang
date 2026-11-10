# ADR 0003：项目范围与分期（Melee 优先）

- **状态**：Accepted
- **日期**：2026-11-07
- **相关**：[ADR 0001](0001-license.md)、[ADR 0002](0002-determinism.md)；`README.md` 范围表

## 背景（Context）

目标是"用 Rust 复刻经典 RTS"，但"复刻"可以指从 Melee 对战一直到含 JASS 自定义地图的完整生态，后者规模远超小团队可交付范围。三个参考项目恰好都在这条线上留了空白：

- `vera20k`：完整复刻 `gamemd.exe` 的 RA2 玩法与联机，但全部绑死 RA2 数据 / 资产 / 2D 渲染，无可移植的游戏逻辑抽象层。
- `godot_warcraft3`：有成熟的 WC3 资产 / 数据管线（MPQ → SLK JSON → BLP/MDX → 地图 JSON），但触发器 / JASS 列为远期 `N∞`（`docs/roadmap/NEXT.md:35`），联机未实现。
- `rts_kernel_cs`：纯 C# 模拟内核契约与移动 / 寻路语义，但明确"网络通信、完整移动规则、战斗、经济、技能等仍有待开发"（`README.md:7`），且不承诺跨平台确定性。

结论：**三者都没有 JASS / 触发器 VM，也都没有可直接搬用的 WC3 战斗 / 经济 / 技能实现**。第一版必须主动收缩范围，否则会卡在最大的空白上。

## 决策（Decision）

### 目标

第一版交付：**确定性模拟内核 + 命令帧同步联机 + Melee 对战**（不含自定义地图脚本）。玩法、数据与资产兼容性以"玩家自备正版经典客户端"为前提（合规边界见 [ADR 0001](0001-license.md) 与 `README.md` 免责声明）。

### 分期

| 阶段 | 交付 | 状态 |
|---|---|---|
| P0 | 定点数、整数帧、实体 ID、命令队列、状态哈希、快照、三类确定性验收 | **已完成**（commit `d7a4e19`） |
| P1 | 读 `godot_warcraft3` 导出的 `slk-exported/` 与 `map-parsed/` JSON，加载一张地图的地形与摆放；**过渡期不写 Rust 原生 MPQ / SLK / MDX 解析器** | **已完成**（归档 `2026-11-07-p1-content-import`） |
| P2 | 模拟竖切：选中 → 移动 → 攻击；移植八方向寻路、动态障碍、朝向 / 坡度、群体编队语义，并换成 [ADR 0002](0002-determinism.md) 的定点数学 | **已完成**（归档 `2026-11-08-p2-simulation-vertical-slice`；坐标契约见 [ADR 0004](0004-frozen-coordinates.md)） |
| P3 | wgpu 3D 渲染：glTF / MDX 骨骼动画、高度场地形、队伍色、粒子 / 挂点（转换规格取自 `godot_warcraft3/docs/design/asset-convert/`） | **已完成并归档**（归档 `2026-11-08-p3-wgpu-3d-renderer`，主 `3d-rendering` spec 已更新） |
| P4 | 移植 lockstep + UDP 可靠层 + 自建 lobby / relay；双客户端同图对战、逐帧 checksum 对比 | **已完成并归档**（归档 `2026-11-08-p4-lockstep-networking`；双客户端逐帧哈希一致，真实 UDP 双进程实机验收通过，主 `lockstep-networking` spec 已更新） |
| P5 | 技能 / 英雄 / 物品 / 科技 / 经济 / 采集 / 建造 / 生产 / 胜负（契约参考 `RTS_ABILITY_SYSTEM.md` 的三层模板 + 效果组合） | **已完成并归档**（commit `d9f5321`，13/13 tasks，已 push；归档 `2026-11-09-p5-gameplay-systems`，主 `gameplay-systems` spec 已建立） |
| P6 | 可玩客户端集成层（交互 / UI / 固定 30 Hz 步进 / 联机 session 接入）：复用既有 `winit 0.30` + `wgpu 30` + `w3-render`，补齐输入→命令、HUD、交互相机与本地 / 联机共用呈现管线；**集成收尾，不新增玩法阶段**（见 change `p6-playable-client`） | **已完成并归档**（归档 `2026-11-10-p6-playable-client`，主 `playable-client` spec 已建立；54/55 tasks，9.9 归档已执行；commit `f5c0555` 已 push） |

### 明确不做（第一版）

- **不引入通用游戏引擎（Bevy 等）**：其 ECS 自动并行调度、异步系统完成顺序与 `f32` / `f64` `Transform` 会与 [ADR 0002](0002-determinism.md) 的整数帧、稳定 `EntityId` / 命令排序、定点权威状态（禁止浮点进入权威状态 / 快照 / 状态哈希）直接冲突；可玩客户端复用既有 `winit 0.30` + `wgpu 30` + `w3-render`，至多加一个轻量 UI 依赖（见 change `p6-playable-client` 的 D1）。
- **JASS / wtg 触发器 VM、自定义地图、战役与过场**：三者皆无实现，属独立大工程；须单独立项后再开 ADR。
- 战争迷雾 / 隐身反隐体系：后续单独机制，不挤进第一版。
- Mod API、配置热更新、跨版本存档迁移、跨运行时（.NET / Godot）兼容。
- 与官方战网 / BNET 兼容；联机只走自建 lobby / relay。
- Godot 宿主、GDScript、Node 场景树；本项目为 Rust + wgpu / winit 自研。
- 原生 `gamemd.exe` 的 Event 线格式（那是 `vera20k` 的 RA2 协议，不复用）。
- 任何暴雪资产的提交或分发。

### 规模与帧率

- P0–P2 以 **10 名玩家 / 300 活跃单位**为验证规模（沿用 `rts_kernel_cs` 的保守起点，`RTS_KERNEL.md:14`）。
- **20,000 单位 / 30 玩家**是 `vera20k` 展示的远期规模目标，**不在第一版承诺**；若要提升，需按 [ADR 0002](0002-determinism.md) 复核点重新做性能与确定性验证，并评估 data-oriented 改造。
- 逻辑帧率 **30 Hz** 已由 ADR 0002 冻结；变更需新 ADR + 全量重放验证。
- 地图 / 单位上限等由内容数据决定，不硬编码进内核。

### 合规

沿用 `godot_warcraft3` 与 `vera20k` 一致的模型：仓库只提交工具与解析代码；MPQ 及其派生文件（含导出的 SLK / 地图 JSON、PNG / GLB / 音效）一律不入库；运行时按 `config.toml` 的本机路径读取。项目名 `rust_dazhang` 为原创通用词，`README.md` 带免责声明。

## 备选方案（Alternatives considered）

| 方案 | 结论 | 理由 |
|---|---|---|
| 第一版即完整复刻（含 JASS / 自定义地图 / 战役） | 否决 | 三个上游都没做触发器 VM；WC3 的 3D + JASS 复杂度显著高于 RA2 2D，范围不可控。 |
| 直接 fork `vera20k` 改造成 WC3 | 否决 | 165 万行中约 97% 绑死 RA2（gamemd 地址、INI、SHP/VXL、2D 渲染、retail 校验），且 GPLv3 附加条款随之进入。 |
| 继续用 Godot 宿主 | 否决 | 目标已定为 Rust 自研引擎；Godot 项目只作为格式规格与导出数据来源。 |
| 先写全套 Rust 原生格式解析器 | 暂缓 | P1 先读现成导出 JSON 让核心跑通，站稳后再逐个替换——这是三个项目之间唯一现成可对接的数据接口。 |

## 后果（Consequences）

**正面**：范围可交付、每阶段有明确验收；P0 已建立确定性基线；合规边界清晰；三个上游各取所长而不背其技术债。

**负面 / 需明确接受**：

- 第一版**不能运行原作自定义地图**，与"完整复刻 WC3"存在明显差距；需在 `README.md` 与对外表述中如实说明。
- P1 依赖 `godot_warcraft3` 的导出 JSON 作为外部输入，其 schema 变化会影响本项目；该依赖须在 P1 记录版本与本地路径（`docs/references/UPSTREAM.md`，待补）。
- 远期规模目标与 30 Hz 帧率存在潜在冲突，提升规模前必须复核。
- JASS 一旦启动，需要单独的方法论（脚本 VM、原生函数支持矩阵），不能靠现有内核契约推导。

## 复核点

- 用户或贡献者要求自定义地图 / 战役支持；
- 规模目标提升至 20,000 单位或玩家数超过 10；
- `godot_warcraft3` 导出格式变化，或决定自研解析器；
- 决定引入战争迷雾、JASS 或跨版本存档。
