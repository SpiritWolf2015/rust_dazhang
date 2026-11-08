# rust_dazhang

用 Rust 从零复刻经典 RTS（即时战略）玩法的开源项目。第一期目标是**确定性模拟内核 + 命令帧同步联机**，玩法范围限定为 Melee 对战。

> **状态：P0 早期开发。** 当前仓库只有工程骨架、配置模板与设计决策，尚无可运行的游戏。

## 许可证

本项目采用 **GPLv3**（见 [LICENSE](LICENSE)）。选择理由、它挡得住和挡不住什么、以及备选方案的权衡，见 [决策记录 0001](docs/decisions/0001-license.md)。

GPLv3 保证的是：任何分发的修改版都必须以同协议提供源码，不能把本项目代码并入闭源专有产品。**它不禁止收费销售或商业服务，也不限制未分发的私有修改。许可证只覆盖代码，不覆盖任何游戏资产或商标**（见下节）。

## 范围

| 做（P0） | 暂不做 |
|---|---|
| 定点数数学、整数逻辑帧、实体 ID、命令队列与稳定排序 | 渲染 / UI / 音频 |
| 全状态哈希、字段级差异定位、快照捕获与恢复 | 网络联机（P4 才接入） |
| 双实例一致性、快照往返、回放三类确定性验收 | MPQ / SLK / MDX / W3X 原生解析器（P1 先读现成导出 JSON） |
| Melee 对战竖切：选中 → 移动 → 攻击 | 战斗 / 经济 / 技能 / 英雄的完整实现 |
| | JASS / 触发器 VM、自定义地图、战役（远期，需单独立项） |

范围一经冻结即写入 `docs/decisions/`；逻辑帧率、数值语义等确定性契约单独记录。

## 资产与法律声明

**本仓库不包含、也不会分发任何暴雪游戏资产。** 开发或运行前，你必须自备**合法获得**的经典客户端（含 `war3.mpq` / `War3x.mpq` 的 MPQ 版本，不是仅含 `Data/` 的 Reforged/CASC 版本）。资产由你本机的工具解包到本地缓存，运行时通过 `config.toml` 指定的路径读取；解包产物、导出的 SLK/地图 JSON、贴图、模型、音效等一律不得提交进仓库，也不得随发行物分发。

项目名 `rust_dazhang` 为通用词自定名，与原作商标无关；游戏内专名、图标、美术、文本、音乐等具体表达应由本项目原创或中性化替换。

### Disclaimer

> This project is an independent, community-led reimplementation written from
> scratch. It contains **no** Blizzard game files: no MPQ archives, no extracted
> models, textures, sounds, maps, UI assets, nor any JSON or other data derived
> from them. Users must supply their own **legally obtained** copy of the classic
> client and are solely responsible for ensuring that use is lawful in their
> jurisdiction.
>
> `rust_dazhang` is **not affiliated with, endorsed by, sponsored by, or
> approved by Blizzard Entertainment, Inc.** or any of its affiliates. Warcraft,
> Warcraft III, The Frozen Throne and all related marks, names, artwork and
> game content are the property of their respective owners. Any references to
> those works in this repository are descriptive and nominative only, for the
> purpose of identifying compatibility with user-supplied files, and do not
> imply authorization or official status.
>
> The GPLv3 license applies to **source code only**. It grants no rights to any
> third-party game assets, trademarks or data.

## 快速开始

```bash
# 1. 复制本机配置（config.toml 已被 .gitignore 排除）
cp config.toml.example config.toml
#    按需修改其中的 wc3_dir / wc3_exported 路径

# 2. 单元测试与确定性验收（P0）
cargo test -p w3-core --lib
cargo test -p w3-core --test determinism -- --nocapture

# 3. P1 内容读取器
cargo test -p w3-content
```

Rust 版本见 `rust-toolchain.toml`。`--lib` 跑各模块的单元测试；确定性验收是 `w3-core` 的集成测试，位于 `crates/w3-core/tests/determinism.rs`。P0 共 19 个单元测试 + 5 个确定性测试；P1 的 `w3-content` 用合成 fixture 测试，不依赖真实导出数据。

## 目录（P0–P1）

```text
rust_dazhang/
├─ Cargo.toml              workspace：w3-core（P0）+ w3-content（P1）
├─ Cargo.lock              提交（应用项目）
├─ LICENSE                 GPLv3 全文
├─ NOTICE.md / CONTRIBUTING.md
├─ rust-toolchain.toml     stable
├─ config.toml.example     本机路径模板
├─ crates/w3-core/         确定性模拟内核（零依赖）
│   ├─ src/{fixed,frame,entity,command,state_hash,snapshot,sim_match}.rs
│   └─ tests/determinism.rs  双实例一致性 / 快照往返 / 回放
├─ crates/w3-content/      P1：只读上游导出 JSON（serde / serde_json）
│   ├─ src/{heightfield,placements,table,load}.rs
│   └─ tests/fixtures/      合成 JSON，不含任何游戏派生资产
├─ docs/decisions/         ADR 0001–0003
└─ docs/references/UPSTREAM.md
```

P0 只建 `w3-core` 一个 crate；P1 才增加 `w3-content`，因为出现了真实消费者（内容读取）。**过渡期不写 MPQ / SLK / MDX / W3X 原生解析器**——真实导出数据由上游 `godot_warcraft3` 的 Node 工具在本机生成，运行时通过 `config.toml` 的路径读取，绝不入库。本机已生成 42 张 SLK 表与 Echo Isles 地图 JSON，并用 `w3-content/examples/load_export.rs` 完成端到端加载验证（129×97 顶点 / 97 单位 / 2491 装饰物 / UnitBalance 836 行）；导出命令与绕过方式见 [`docs/references/UPSTREAM.md`](docs/references/UPSTREAM.md)。

内核模块名为 `sim_match` 而非 `match`（Rust 关键字）；`RtsMatch` 已由 `lib.rs` 重导出，调用方直接用 `w3_core::RtsMatch`。`w3-content` 的坐标是上游 JSON 的原始 `f64`，属于冻结输入，不进权威状态 / 快照 / 哈希；P2 进入模拟前须按 ADR 0002 冻结定点转换规则。

## 参考的上游项目

四者均为**只读参考**，不复制其代码进本仓库，也不作为 git submodule 引入；本地路径与固定提交记录在 `docs/references/UPSTREAM.md`。

| 项目 | 许可证 | 借鉴内容 |
|---|---|---|
| `vera20k` | GPLv3 + 附加条款 | 确定性 lockstep、定点数、命令队列、全状态哈希、UDP 可靠层与 relay 架构、crate 拆分与依赖纪律 |
| `godot_warcraft3` | MIT | MPQ / SLK / BLP / MDX / W3X 格式规格、资产三车道与"工具入库、资产留本机"的合规模型；过渡期直接读其导出 JSON |
| `rts_kernel_cs` | MIT | 模拟内核 API 契约、唯一状态所有者、每帧阶段顺序、命令稳定排序、快照与状态哈希设计 |
| `WarsmashModEngine` | AGPLv3 | Java / LibGDX 的 WC3 模拟器：JASS / 触发器语义、MPQ 与 CASC 数据源、单位与技能玩法行为（只读参考，不复制代码） |

致谢与版权声明见 `NOTICE.md`。四者均不含游戏资产，本项目的资产合规模型与它们一致；`WarsmashModEngine` 为 AGPLv3，与本项目 GPLv3 组合有条件，未经 ADR 复核不得引入其代码。

## 贡献

贡献代码即表示按 **GPLv3** 授权（详见 `CONTRIBUTING.md`，待补）。请勿提交任何游戏资产、从资产解包或导出的文件、或专有名词与美术素材。
