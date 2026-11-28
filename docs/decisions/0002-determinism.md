# ADR 0002：确定性契约（定点数、整数帧与状态权威）

- **状态**：Accepted
- **日期**：2026-11-07
- **相关**：[ADR 0001](0001-license.md)、[ADR 0003](0003-scope.md)；`crates/w3-core/`
- **实现对应**：P0 骨架 commit `d7a4e19`

## 背景（Context）

命令帧同步联机的前提是：相同代码、相同内容、相同输入与随机流，在任何支持的平台与 CPU 上逐帧产生相同状态。三个参考项目对此给出了不同答案：

| 项目 | 数值路线 | 确定性承诺 |
|---|---|---|
| `vera20k` | `SimFixed` 定点 + 纯 Rust `libm` 处理超越函数 | 明确要求跨平台 / 跨 CPU 确定 |
| `rts_kernel_cs` | `double`（`SimulationTypes.cs:3`），"不引入定点库" | 仅承诺同代码 / 内容 / 受控运行环境；明确"不承诺跨平台 / 跨运行时确定性"（`README.zh.md:7,21`） |
| `godot_warcraft3` | 无独立权威内核 | — |

`rts_kernel_cs` 的路线无法支撑联机帧同步：浮点舍入、数学库实现与编译优化在平台间可能不一致。本项目因此**不沿用其数值语义，只沿用其 API、状态所有权与帧序契约**（借鉴表见 `README.md`）。同时避免两个极端：为"精确复刻 x87"而模拟浮点（`vera20k` 契约明确否决），以及为通用性预先引入第三方数值框架。

## 决策（Decision）

### 1. 定点数

- 权威状态使用 `SimFixed`（`crates/w3-core/src/fixed.rs`）：`i64` 上的 Q47.16，乘除用 `i128` 中间量，整数除法向零截断。
- **禁止 `f32` / `f64` 进入权威状态、快照或状态哈希**。表现层（渲染插值、相机、音频）可使用浮点，但不得反写模拟。
- 溢出与除零显式返回 `FixedError`，不静默回绕。
- 超越函数（sqrt / atan / ln …）P0 未使用。若将来需要，必须使用**纯 Rust、跨平台一致的实现**（如 `libm`），并在本 ADR 追加记录与边界测试；不得调用宿主 C 库。
- P0 保持 crate 零依赖、离线可编译；`fixed` crate 等成熟实现留待实际精度或维护成本出现时评估。

### 2. 帧与时间

- `Frame` 为 `u64` 整数帧号；`FrameClock::advance()` 固定 +1（`frame.rs`）。`RtsMatch::step()` **不接受任何 delta、墙钟或渲染帧率**。
- `logic_fps = 30`（`config.toml.example`），**自本 ADR 起项目级冻结**：对局创建后不可变；任何变更需新 ADR + 全量重放 / 快照基线重新验证。
- 持续时间按整数帧计，保存起止帧而非累减秒数；显式 0 时长允许同帧处理。

### 3. 命令与顺序

- 命令含 `ExecuteFrame / PlayerId / Sequence / EntityId / 参数`，提交时由队列冻结 `ArrivalOrder`（`command.rs`）。
- 每帧 `freeze_frame` 取到期命令，按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序后执行；**到达顺序不决定执行顺序**。
- 执行时重新校验存活 / 在场 / 归属；接收不等于执行成功。
- 同类系统按稳定 `EntityId` 顺序遍历，用 `BTreeSet` / `BTreeMap`；禁止 `HashMap` 迭代顺序、无序集合枚举、异步任务完成顺序影响规则。
- 新生成实体默认下一帧参与常规系统；同帧连锁效果走显式有限队列（P0 尚未实现，列为后续约束）。

### 4. 随机流

- 每场对局独占一条 `u64` LCG 流（`sim_match.rs::next_rng`），每帧恰好消耗一次；宿主不能通过公开对象消费它。
- 流状态、算法版本与消费顺序进入快照；表现层未来须使用独立随机源。

### 5. 状态权威与表现边界

- `RtsMatch` 是唯一写入者，字段全私有；`read_view()` 返回 owned `WorldView`，`drain_events()` 取走事件拷贝。
- 表现层（尚未实现）只读视图与事实事件，不得反写位置、朝向或生命；动画丢帧、资源加载失败不得改变模拟。
- 事件序号在 `drain_events()` 后**不重置**；恢复快照不重放历史事件，只恢复序号。

### 6. 状态哈希与差异诊断

- FNV-1a + 显式小端字节编码（`state_hash.rs`）；禁用 `DefaultHasher`、`GetHashCode`、对象身份。
- 参与字段固定：`tick / rng / next_entity_id / next_arrival / next_event_seq / seed / content_hash / logic_fps / 实体及 motion（按稳定 ID 顺序）/ 未执行命令`。
- 诊断日志、视图消费位置、可重建缓存、历史事件**不参与**权威哈希。
- `first_difference()` 按同一稳定顺序返回首个差异字段名，便于定位 desync。

### 7. 快照

- 定义为帧 N 提交完成后的完整状态，恢复后从 N+1 继续（`snapshot.rs`、`restore_snapshot`）。
- 含对局配置、RNG 流、ID 计数器、实体全部状态、未执行命令、事件序号；不含 Node / 委托 / 闭包 / 历史输出。
- 头部校验 `version / sim_version / content_hash`，不匹配明确拒绝（`SnapshotError`）。
- **不支持跨版本迁移**，不承诺跨运行时兼容。

### 8. 验收门槛

- `cargo test -p w3-core --test determinism`：双实例交错逐帧哈希一致、快照销毁重建后继续一致、命令回放一致、首个差异字段定位、内容不匹配拒绝恢复。
- 任何改动权威状态、帧序、命令排序、哈希编码或快照字段的改动，必须先让这些测试通过。

## 备选方案（Alternatives considered）

| 方案 | 结论 | 理由 |
|---|---|---|
| `f64` / `double`（`rts_kernel_cs` 路线） | 否决 | 无法保证跨平台一致，联机帧同步失去前提。 |
| x87 精确模拟 | 否决 | 与 `vera20k` 契约一致：精确原生算术本身不是模拟 x87 的理由。 |
| 第三方 `fixed` crate / 自建通用数值框架 | 暂缓 | P0 零依赖、离线可编译；实际精度或维护成本出现时再评估。 |
| 只承诺"同机同运行时确定性" | 否决 | 等价于放弃联机。 |
| 帧率可变 / 按墙钟补帧 | 否决 | 破坏整数帧语义与重放。 |

## 后果（Consequences）

**正面**：跨平台一致的算术基础；desync 可定位到字段；快照 / 回放可验证；P0 无外部依赖。

**负面 / 需明确接受**：

- Q47.16 的**数值范围与精度有限**（`i64` 溢出显式报错）；大地图坐标、极长时长需在未来评估是否换 Q 格式或分段表示。
- 所有进入权威状态的量必须显式转换；表现层与模拟层数值类型不同，适配层要写清坐标单位。
- 超越函数尚未验证；一旦引入，需补齐跨平台边界测试。
- `logic_fps = 30` 一旦冻结，改动成本高。
- **目前仅在本机 Windows 11 / rustc 1.91.0 实跑通过**；Linux、macOS 与其他 CPU 架构的跨平台确定性**尚未实测**，属于待验证项，而非已达成结论。

## 复核点

引入浮点或超越函数、修改 `logic_fps` 或帧阶段顺序、引入并行 / 异步仿真、提升规模目标（见 [ADR 0003](0003-scope.md)）、支持跨版本存档、在非 Windows 平台首次实跑。

## 复核记录

- **复核**：本段为本 ADR §「复核点」中「在非 Windows 平台首次实跑」的首次兑现记录，由 OpenSpec change `cross-platform-golden-hash` 的离线 golden-hash 工具完成；本段**只追加**，不改写 §1–§8 正文、第 94 行历史基线数值（`rustc 1.91.0`）、备选方案表与「复核点」原文。
- **触发点**：命中 §「复核点」的「在非 Windows 平台首次实跑」；由 change `cross-platform-golden-hash` 新增的 `crates/w3-core/examples/golden_hash.rs`（独立 example，只调 `w3-core` 公开 API、`w3-core/src/` 零改动）完成离线验证。
- **实测环境**：Windows 11 本机（`c:/open_rts/rust_dazhang`）与 Debian 12（bookworm，内核 `6.1.0-37-amd64`，x86_64）VM（`/srv/rust_dazhang`）；两端均 `rustc 1.99.1`、`protoc 31.1`；`crates/w3-core/examples/golden_hash.rs` 两端源码 sha256 一致（`524c8520...c758`）。
- **方法**：固定 17 条命令脚本、`FRAME_COUNT = 40` 个整数帧，`MatchConfig { seed: 0x5eed, logic_fps: 30, content_hash: 0xC0FFEE }`、`StaticWorld::open_world(16, 16, SimFixed::ONE)`；每帧调用 `compute_state_hash()` 并输出定宽 `0x{:016x}` 的逐帧行，末尾一行汇总只含编译期常量（`frames` / `final_hash` / `schema=golden-hash/1`）；对**整份文本**（全部 40 个中间帧 + 汇总行）逐字节比对，**非只比最终 hash**；`rustc` 等工具链版本只在本记录等验证报告中记录，不进基线文本。
- **结果**：两份输出 sha256 同为 `c6dc62b4b3f8718b7edc4b5ba9f90db44c146cdb068904c67d7898c4303b3775`，41 行 / 1371 字节逐字节 `MATCH`，`diff -u` 无输出，`final_hash=0x3d7e4312f63844e5`。
- **结论边界**：据此把「Linux / Debian 12 x86_64、`rustc 1.99.1`」记为**已验证**（与 Windows 逐帧 hash 序列逐字节一致）；**macOS、其它 CPU 架构（ARM64 等）、其它 `rustc` 版本仍属待验证项**，不得扩大为「全平台 / 全 CPU 确定性已达成」。`logic_fps = 30`、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 命令稳定排序、FNV-1a 状态哈希、`SNAPSHOT_VERSION` v3 快照契约**零改动**；本次是**验证**而非改动，第 94 行历史基线数值 `rustc 1.91.0` 与本次实测 `rustc 1.99.1` 并存、不回改正文。
- **关联 change**：`cross-platform-golden-hash`（离线 golden-hash 工具与双平台逐字节 diff）；`linux-winit-platform-backend`（正式修复 Linux 编译前提：为 `crates/w3-render/Cargo.toml` / `crates/w3-client/Cargo.toml` 追加 Linux target 条件依赖 `x11`，VM 上 `python3 tools/verify_all.py` → `Verification OK`、24 项全绿）。golden diff 按拍板**不纳入** CI / `tools/verify_all.py`，走人工比对。
- **复核（P8.5a `p8-hero-revive`，2026-11-19）**：按 Q1 拍板 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 3→4；新增英雄死亡登记 / 祭坛复活队列 / 费用三组权威状态，均落 `BTreeMap` 确定性有序容器并同步进入 FNV-1a 状态哈希、由快照版本定义的快照与 `first_difference()`；旧 v3 快照 / 回放按 `SnapshotError` / `Reject` 明确拒绝、不提供跨版本迁移（§7）。`cargo test -p w3-core --test determinism`（26 项）全绿并经「改坏验红」反向验证。本段为 Q1「变更须 ADR 复核」的兑现记录，沿用上文「只追加、不改写 §1–§8 正文与历史基线数值」纪律。
- **golden hash 基线（P8.5a）**：因新增权威状态，golden hash 基线按新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0xada6506e1da36c11`、整份文本 sha256 `8b48b8f2035bd7dc3df4b37c8e21c9d896cdc6caa2a367c5dc23f3df04fc99a3`（41 行 / 1371 字节）；上文「结果」记录的旧值 `final_hash=0x3d7e4312f63844e5` 已失效、不作为新基线。**Debian 12 x86_64 侧逐字节离线比对仍待 VM / 人工完成，不得据本段声称 bump 4 后双平台 golden hash 已逐字节验证**；macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项（沿用上文「结论边界」），不声称全平台达成。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。
- **复核（P8.5b `p8-townhall-upgrade`，2026-11-20）**：按 Q1 用户拍板把 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 4→5；新增顶层权威状态 `town_hall: TownHallUpgradeState`（`tiers` / `queue` / `charges` 三组，均落 `BTreeMap` 确定性有序容器、按稳定 `EntityId` 顺序遍历，为独立字段不复用 `PlayerEconomy.upgrades` 布尔科技旗标），同步进入 FNV-1a 状态哈希、由 v5 快照定义的 `Snapshot.town_hall` 与 `first_difference()`；tier 需求链冻结常量经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与 tier 顶层权威状态严格区分）；新增 `UpgradeTownHall` 命令变体（`hash_gameplay_command` 唯一编码 13）与升级开始 / 完成 / 取消 3 个事实事件变体。`cargo test -p w3-core --test determinism`（26 项）全绿，并经 Q8 四类「改坏验红」反向验证。旧 v4 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移（§7）。本段为 Q1「变更须 ADR 复核」的兑现记录，沿用上文「只追加、不改写 §1–§8 正文与历史基线数值」纪律。
- **golden hash 基线（P8.5b）**：因新增权威状态与定义层冻结常量，golden hash 基线按新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0xce89a7baf4d711ec`、整份文本 sha256 `202b7490f94cbc059a266c1663fc21e847e55ebb2c3620750f4346855475742c`（41 行 / 1371 字节）；上文 P8.5a 段 v4 旧值 `final_hash=0xada6506e1da36c11` 已失效、不作为新基线、不沿用、不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 逐字节离线比对仍待 VM / 人工完成，不得据本段声称 bump 5 后双平台 / 全平台确定性已验证或有任何新验证结论**；沿用上文「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4 / v5 侧仅 Windows 重取基线，macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项，不声称全平台达成。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。
- **复核（P8.5c `p8-creep-camps`，2026-11-20）**：按 Q4 用户拍板把 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 5→6（`crates/w3-core/src/snapshot.rs:33-34`）；新增顶层权威状态 `creep_camps: CreepCampState`，以 12 组 `BTreeMap` / `BTreeSet` 承载野怪 / 营地 / 中立归属（`NEUTRAL_OWNER=0` 与冻结 `NeutralCategory`）/ leash `IDLE/ENGAGED/RETURNING` 状态机与 home 锚点 / 击杀收益 / 地面物品 / 物品实例 / 确定性随机码消费序列（独立事件驱动流 + `rng_draws` 显式编码，复用 match-owned 整数 LCG、`seed` 随快照），全部同步进入 FNV-1a 状态哈希、由 v6 快照定义的 `Snapshot.creep_camps` 与 `first_difference()`；掉落表 / 营地脚本 / 中立类别 / 野怪数值为 `FrozenDefinitions` 冻结常量，经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与新增权威状态严格区分）；新增 `GameplayCommandKind::SeedCreepCamp { camp, script }`（`hash_gameplay_command` kind 码 14）与 `CreepCampSeeded` / `CreepEngaged` / `CreepLeashReturning` / `CreepLeashReturned` / `CreepDied` / `ItemDropped` 6 个 `EventKind` 事实变体。`cargo test -p w3-core --test determinism`（26 项）全绿，并按 Q9 形态①（测试内反例 + 元断言）完成 20 项成对反向验证与 20 项正向玩法回归。旧 v5 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移（§7）。本段为 Q4「变更须 ADR 复核」的兑现记录，沿用上文「只追加、不改写 §1–§8 正文与历史基线数值」纪律。
- **golden hash 基线（P8.5c）**：因新增 `creep_camps` 权威状态与定义层冻结常量，golden hash 基线按新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0xc802b63cd6c55b74`、整份文本 sha256 `c5927605d534348e4e92db6ecb4042a42abfa7cd9077fe70da5283717c4f7d7e`（41 行 / 1371 字节）；上文 P8.5b 段 v5 旧值 `final_hash=0xce89a7baf4d711ec` 已失效、不作为新基线、不沿用、不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 / v6 逐字节离线比对仍待 VM / 人工完成，不得据本段声称 bump 6 后双平台 / 全平台确定性已验证或有任何新验证结论**；沿用上文「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4 / v5 / v6 侧仅 Windows 重取基线，macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项，不声称全平台达成。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。
- **复核（P8.5d `p8-item-shop-tp`，2026-11-21）**：按 Q4 用户拍板把 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 6→7（`crates/w3-core/src/snapshot.rs:43-44`）；新增顶层权威状态 `item_shop: ItemShopState`（`Snapshot.item_shop`，`snapshot.rs:145`），14 组权威状态字段含六格定长实例背包 / 死亡背包 / 拾取购买使用丢弃移交售卖回城刷新事实 / 库存与刷新计数 / 幂等刷新批次 / 确定性随机码消费序列 / 回城施法 / 传送次数 / 友方主城登记 / 扣费登记等，全部落 `BTreeMap` / `BTreeSet` 确定性有序容器或定长数组，并把 P8.5c `ItemInstanceState` 扩展出 `holder` / `slot`，全部同步进入 FNV-1a 状态哈希、由 v7 快照定义的 `Snapshot.item_shop` 与 `first_difference()`（`sim_match.rs` 的 `compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference` 四处同步）；货架 / 价格 / 库存上限 / 刷新间隔 / 回城施法帧等 `[待核]` 冻结常量经 `FrozenDefinitions.item_shop` 段、`with_item_shop(...)` 与 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与新增权威状态严格区分）；新增 9 个 `GameplayCommandKind` 变体（`hash_gameplay_command` 唯一 kind 码 15–23，既有 1–14 不动）与 11 个 `EventKind` 事实事件（`Event` 结构不变、无 payload）。`cargo test -p w3-core --test determinism`（26 项）全绿，并按 Q9 形态①（测试内反例 + 元断言防退化）新增正向 `item_shop_town_portal.rs`（25 项）与反向验证 `reverse_validation_item_shop_tp.rs`（19 项）。旧 v6 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移（§7）。本段为 Q4「变更须 ADR 复核」的兑现记录，沿用上文「只追加、不改写 §1–§8 正文与历史基线数值」纪律。
- **golden hash 基线（P8.5d）**：因新增 `item_shop` 权威状态与定义层冻结常量，golden hash 基线按新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0x940cb016f6630f1e`、整份文本 sha256 `d6c0c944d6ac3397cd16995ab82bce3243ecd7673d4788fec336c332936aaa41`（41 行 / 1371 字节）；上文 P8.5c 段 v6 旧值 `final_hash=0xc802b63cd6c55b74` 已失效、不作为新基线、不沿用、不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 / v6 / v7 逐字节离线比对仍待 VM / 人工完成，不得据本段声称 bump 7 后双平台 / 全平台确定性已验证或有任何新验证结论**；沿用上文「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4 / v5 / v6 / v7 侧仅 Windows 重取基线，macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项，不声称全平台达成。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。
- **复核（P9 `p9-undead-foundation`，2026-11-22）**：按用户 2026-11-22 规格门 Q2「族特色机制纳入 `w3-core` 顶层权威状态全套」/ Q3「bump 8 全套」/ Q5'「逐项清单」拍板，把 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 7→8（`crates/w3-core/src/snapshot.rs:43-44`）、`PROTOCOL_VERSION` bump 7→8（`crates/w3-net/src/packet.rs:45`）；新增顶层权威状态 `faction_mechanics: FactionMechanicsState`（`faction_mechanics.rs:415`、`Snapshot.faction_mechanics` `snapshot.rs:166`），以 `BTreeMap` / `BTreeSet` 确定性有序容器承载腐地 / 尸体 / 诅咒金矿 / 缠绕金矿 / 古树扎根·拔根 / 月亮井 / 隐身目标判定 / 反隐 / 夜间隐身 / 昼夜相位 / 视线阻挡 / 野怪睡眠 / 回血条件等族特色机制，按稳定 `EntityId` 顺序遍历，四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）进 FNV-1a 状态哈希、由 v8 快照定义的 `Snapshot.faction_mechanics` 与 `first_difference()`；迷雾 / 视野可见性 MUST 表现层只读派生、fog 可见性 MUST NOT 进权威状态 / 哈希 / 快照 / `first_difference()`（R10.5 / §0.3 T3 / Q2' 已拍板、对照差异 D1 / D2 明确不借鉴、不抹平、§14 不静默改规则）。族特色机制冻结常量与回血条件定义层类别登记经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与新增权威状态严格区分）；新增 13 个 `GameplayCommandKind` 变体（`hash_gameplay_command` 唯一 kind 码 24–36，既有 1–23 不动、校验 / 穷尽分发三处同步）与 22 个 `EventKind` 事实事件变体（`Event` 结构不变、无 payload）；`w3net.proto` 只追加 tag 46–58、`reserved 14 to 32` 保持、字段编号只追加（ADR 0005 §3）。旧 v7 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、`Packet::decode` 对 v7 `VersionMismatch` + relay `Reject`、不提供跨版本迁移（§7）。`cargo test -p w3-core --test determinism`（26 项）全绿并经 `CODEBUDDY.md §5` 形态①（测试内反例 + 元断言防退化）反向验证、`cargo test --workspace` 干净环境全绿。本段为 Q3 / Q5'「变更须 ADR 复核」的兑现记录，沿用上文「只追加、不改写 §1–§8 正文与历史基线数值」纪律。
- **golden hash 基线（P9）**：因新增 `faction_mechanics` 权威状态与定义层冻结常量 / 回血条件类别登记，golden hash 基线按 v8 新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs:75-77` 注释留档）`final_hash=0x0b53a047b1f8e5e0`、整份文本 sha256 `aa0754e2640badd6fb93cbfeb7b73cdb54fcc70d4497a50ca29d2c84d9d628d4`（41 行 / 1371 字节）；上文 P8.5d 段 v7 旧值 `final_hash=0x940cb016f6630f1e` 已失效、不作为新基线、不沿用、不声称逐字节不变。**Debian 12 x86_64 侧 v4 / v5 / v6 / v7 / v8 逐字节离线比对仍待 VM / 人工完成，不得据本段声称 bump 8 后双平台 / 全平台确定性已验证或有任何新验证结论**；沿用上文「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4–v8 侧仅 Windows 重取基线，macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项，不声称全平台达成。真窗口 8.x 人工清单保持「未执行 / 待人工」、CI 双平台 `verify_all.py` 本会话未运行、如实标注、不冒充通过或有新验证（Q8' 已拍板）。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。
- **复核（P11 `p11-ability-system`，2026-11-24）**：按用户 2026-11-24 规格门 Q5「`SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 三者同步 8→9」拍板，本 ADR 侧把 `SNAPSHOT_VERSION` / `SIM_VERSION` bump 8→9（`crates/w3-core/src/snapshot.rs`；`PROTOCOL_VERSION` 8→9 属 ADR 0005 范畴、见其修订记录），新增顶层权威状态 `AbilitySystemState`（`crates/w3-core/src/ability_system.rs`）承载 `ability_hero_skill_points` / `ability_levels` / `ability_zones` / `ability_summons` / `ability_aura_beneficiaries` / `ability_passive_procs` / `ability_transformations` / `ability_buff_categories` / `ability_chains` / `ability_point_targets` / `ability_pending_casts` / `ability_rng_draws` / `ability_next_zone_id` / `ability_next_chain_id` / `ability_next_rng_seq` 共 **15 个字段**，全部落 `BTreeMap` / `BTreeSet` 确定性有序容器、按稳定 `EntityId` 顺序遍历，四处同步（`compute_state_hash` / `capture_snapshot` / `restore_snapshot` / `first_difference`）进 FNV-1a 状态哈希、由 v9 快照定义的 `Snapshot` 与 `first_difference()`，并经 `WorldGameplayState::hash` / 其 `first_difference` / restore 五重校验；`EffectKind` 从 4 扩到 13 变体（码 0–12、旧 0–3 不动）、`BehaviorCatalog` 单一注册表、5 个新命令 `CastPoint` / `CastArea` / `CastDirection` / `LearnAbility` / `LevelUpHero`（`hash_gameplay_command` kind 码 37–41）；技能清单 / 行为分类 / buff 分类等**定义层冻结常量**（数值一律 `[待核]`、合成原创编译期整数）经 `compute_defs_hash` 纳入 `defs_hash` / `content_hash` 进快照头部（定义层内容变化，与 15 个新增权威运行态严格区分）；随机沿用单条 match-owned 整数 LCG + 显式 `rng_draws`、不新增独立流（§4 / P8.5c 先例）。旧 v8 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、不提供跨版本迁移（§7）。`cargo test -p w3-core --test determinism` **30 项全绿**（26 基线 + 4 新增：四族技能逐帧一致 / 三路径一致 / v9 快照往返 / 旧 v8 拒绝）。`CODEBUDDY.md §5` 形态①反向验证结论如实登记：`crates/w3-core/tests/reverse_validation_ability_system.rs` 7 项成对证据（15 字段逐一注入差异的正式态断言 + `clear_field()` 退化态模拟漏写必失败 + 元断言防退化），另有 `reverse_validation_hash_omission` / `first_difference_omission` / `snapshot_and_restore_omission` / `defs_hash_sensitivity` 专项退化态；仅「测试内反例 + 元断言防退化」，不冒充形态② A-B 命令输出。**golden hash 基线（P11）**：因新增 15 字段权威运行态与定义层冻结常量，golden hash 按 v9 新实现如实重取、不沿用旧值、不声称逐字节不变；Windows 11 / x86_64 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0x240467029c2b423c`、整份文本 sha256 `4bb9e9f860a060bdaf1b0785c3307de35fa8c10bd1d8e70759b1b8041350fa60`（41 行 / 1371 字节）；上文 P9 段 v8 旧值 `final_hash=0x0b53a047b1f8e5e0` 已失效、不作为新基线、不沿用、不删改（该段一字不改）。**Debian 12 / macOS / ARM64 / 其它 rustc 逐字节比对仍待 VM / 人工完成，不得据本段声称 bump 9 后双平台 / 全平台确定性已验证或有任何新验证结论**；沿用上文 `:118`「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4–v9 侧仅 Windows 重取基线，macOS / 其它 CPU 架构（ARM64 等）/ 其它 rustc 版本仍属待验证项，不声称全平台达成；真窗口 8.x 人工清单保持「未执行 / 待人工」、CI 双平台 `verify_all.py` 本会话仅 Windows 本机、双平台由人工 push 后验证、如实标注、不冒充通过。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。本段为 Q5「变更须 ADR 复核」的兑现记录，沿用上文「**只追加、不改写 §1–§8 正文与历史基线数值**」纪律。
- **复核（p18 `p18-full-melee-mechanisms`，2026-11-28）**：按用户 2026-11-28 apply 阶段三项明确拍板（① 新增玩法命令变体 `BuybackHero`、`w3net.proto` 只追加 tag 64 起；② 按 design D1 新增权威运行态字段；③ 反向验证以形态①「测试内反例 + 元断言防退化」为主并补齐 p17 R2 生产结算形态② A-B 命令输出），把 `SNAPSHOT_VERSION` / `SIM_VERSION` / `PROTOCOL_VERSION` 三者同步 bump 9→10（遵循 ADR 0002 §6-7 与 ADR 0005 §3 门禁、非放宽红线）；新增顶层权威运行态 `MeleeMechanismsState` 六字段（克制矩阵二维查表产出 / 经验阈值分档 / 买活费用时间公式族 / 分摊分档因子 / 回购价公式 / 三级 leash 结构）全部落 `BTreeMap` / `BTreeSet` 确定性有序容器、按稳定 `EntityId` 顺序遍历，四处同步（`compute_state_hash` / `first_difference` / `capture_snapshot` / `restore_snapshot`）进 FNV-1a 状态哈希与由 `SNAPSHOT_VERSION` 定义的 v10 快照；机制常量属编译期 `[待核]` 合成占位层、不冻结原版真值、MUST NOT 冒充经典规则数值，不编入 `compute_defs_hash()` / `content_hash()`（定义层内容变化与该六字段权威运行态严格区分）。旧 v9 快照 / 回放按 `SnapshotError::UnsupportedVersion` / `SimVersionMismatch` 明确拒绝、`Packet::decode` 对 `version=9` 返回 `VersionMismatch`、relay 回 `Reject`，不提供跨版本迁移 / 不静默降级（§7 / ADR 0005 §3）。**golden hash 基线（p18 / v10）**：因新增权威运行态字段进哈希，基线按 v10 新实现如实重取、不沿用旧值、不声称逐字节不变。Windows 11 / x86_64 / rustc 1.99.1 侧（`crates/w3-core/examples/golden_hash.rs` 注释留档）`final_hash=0x313ebf22ede567ef`、整份文本 sha256 `39b9c82c958109807c8930807cc30d1f8ba2b3f3b9ec1152ddd12f07aa2e969c`（41 行 / 1371 字节，`schema=golden-hash/1` 不变）；上文 P11 段 v9 旧值 `final_hash=0x240467029c2b423c` 已失效、不作为新基线、不沿用、不声称逐字节不变。基线变化的**主因**是新增权威运行态字段进入 FNV-1a 状态哈希，而非版本号字面量——`SNAPSHOT_VERSION` / `SIM_VERSION` 常量本身 MUST NOT 进入 `compute_state_hash()`（哈希参与字段固定，ADR 0002 §6）。`cargo test -p w3-core --test determinism` **30 项全绿**；`CODEBUDDY.md §5` 形态①反向验证（`MeleeMechanismsState` 六字段逐一注入差异的正式态断言 + 元断言防退化、漏写哈希 / 快照 / `first_difference` 退化态、旧 v9 拒绝退化态等八类回归）与 p17 R2 生产结算形态② A-B 命令输出（见 `openspec/changes/p18-full-melee-mechanisms/decision-memo.md`「## apply 证据留档」）均已如实登记，不冒充形态② A-B 之外的验证、不冒充人工验收通过。30 Hz 整数帧、`SimFixed` Q47.16、`ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序、稳定 `EntityId` 顺序、FNV-1a 编码方式不变。**诚实边界**：本次重取侧重 Windows 11 / x86_64 / rustc 1.99.1 一侧；Debian 12 x86_64 侧 v4 / v5 / v6 / v7 / v8 / v9 / v10 逐字节离线比对、macOS、其它 CPU 架构（ARM64 等）与其它 rustc 版本的逐字节比对仍待 VM / 人工完成，CI 双平台 `verify_all.py` / proto 生成代码字节一致 / 真窗口 8.x 人工清单一律「待人工 / 待 VM」、不标通过；沿用上文「结论边界」既有口径——跨平台逐帧一致性仅在 v3 于 Windows 11 / Debian 12 x86_64 / rustc 1.99.1 已逐字节验证，v4–v10 侧仅 Windows 重取基线，不声称全平台 / 双平台确定性已达成、不得据本段声称 bump 10 后有任何新验证结论（Q6 / Q8）。本段按历期体例**只追加**、不改写 §1–§8 正文、第 94 行历史基线数值（`rustc 1.91.0`）、备选方案表、「复核点」原文与上文 P8.5a–d / P9 / P11 全部历期复核记录。
