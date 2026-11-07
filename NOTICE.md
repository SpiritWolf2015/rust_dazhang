# NOTICE — rust_dazhang

本文件记录本项目的版权、许可证与上游致谢。**本仓库不包含任何暴雪游戏资产**（见 `README.md` 免责声明与 [ADR 0001](docs/decisions/0001-license.md)）。

## 本项目

- 版权：Copyright (C) 2026 rust_dazhang contributors
- 许可证：GNU General Public License v3.0（见 `LICENSE`）
- 本项目为独立从零编写，**未复制任何上游仓库的源代码**。

## 上游参考项目

下表三个项目为**只读参考**，用于设计借鉴与格式规格；它们的代码不进入本仓库，也不作为 git 子模块引入。

| 项目 | 许可证 / 版权 | 借鉴内容 | 本地参考路径 |
|---|---|---|---|
| `vera20k` | GPLv3 + Section 7 附加条款；Copyright © 2026 Halvor Nikolai Rusten and VERA20K contributors | 确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律 | `C:/open_rts/vera20k` |
| `godot_warcraft3` | MIT；Copyright (c) 2026 李维民 | WC3 MPQ / SLK / BLP / MDX / W3X 格式规格、资产三车道与"工具入库、资产留本机"合规模型；P1 起读取其本机导出 JSON | `C:/open_rts/godot_warcraft3` |
| `rts_kernel_cs` | MIT；Copyright (c) 2026 李维民 | 模拟内核 API 契约、唯一状态所有者、帧阶段顺序、命令稳定排序、快照与状态哈希设计 | `C:/open_rts/rts_kernel_cs` |

固定参考的提交与各自用途详见 [`docs/references/UPSTREAM.md`](docs/references/UPSTREAM.md)。

MIT 条款要求保留版权与许可声明。若将来引入其中任一项目的代码片段，须在对应文件保留其原始版权头，并在此处登记来源与许可证。

## 第三方资产与商标

Warcraft、Warcraft III、The Frozen Throne 及相关名称、美术、音乐与游戏内容归其各自权利人所有。本项目与暴雪娱乐无关联、未获其授权或背书。玩家须自备合法获得的经典客户端；仓库不分发任何资产或由资产派生的文件。

## 许可与署名问题

请通过仓库 Issue 提出。
