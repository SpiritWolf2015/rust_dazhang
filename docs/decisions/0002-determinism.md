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
