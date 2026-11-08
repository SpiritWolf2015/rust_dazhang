## Why

第一版的核心目标之一是联机。P0 已保证确定性，P2 已有玩法，但还没有把多端命令编排成相同帧的机制。P4 把 `vera20k` 的 lockstep / UDP 可靠层 / relay 架构的设计移植过来，形成自建联机。本 change 对应 [ADR 0003](../../../docs/decisions/0003-scope.md) 路线图的 **P4 阶段**（命令帧同步联机）；确定性契约见 ADR 0002，资产与法律边界见 ADR 0001，冻结坐标与定点单位见 ADR 0004。

## What Changes

- 新增 `w3-net` crate：确定性 lockstep 命令编排、UDP 可靠传输、Event 编解码、逐帧 checksum 交换与比对。
- 新增 `w3-relay` 服务：自建 lobby 与认证 relay 时钟、不可变命令流、断线重连与回放恢复。
- 定义本项目自己的 Event 线格式（不复用 `vera20k` 的 RA2 `FRAMEINFO`/`FRAMESYNC` 包）。

## Capabilities

### New Capabilities

- `lockstep-networking`: 多端命令帧编排、可靠传输、一致性校验与断线恢复。

### Modified Capabilities

（无。）

## Impact

对资产/法律边界（ADR 0001）：不新增、不提交任何 MPQ 派生的导出资产（含导出 JSON / PNG / GLB / 音效）与任何游戏资产，导出数据仍留玩家本机、按 gitignored 的 `config.toml` 路径读取；只读参考 `vera20k` 架构设计、不复制其代码，自建服务不引入新许可证依赖；在线对局 SaaS 化不在 GPLv3 覆盖内，必要时评估 AGPLv3。对确定性契约（ADR 0002）：本 change 不新增权威状态、不改动帧序、命令排序、状态哈希与快照 v1；网络抖动、丢包重传与墙钟计时只影响实时速率，MUST NOT 进入 `compute_state_hash()` / 快照 v1，模拟内核跨平台确定性仍为待验证项（Linux / macOS 尚未实跑）。冻结坐标与定点单位（ADR 0004）只在冻结边界单向转换，不反写模拟。

- 新增 workspace 成员与自建服务；`sim/` 仍 MUST NOT 依赖 `net/`。
- 联机只走自建 lobby/relay，不兼容官方战网；这是本项目基于资产/法律边界（ADR 0001）与范围（ADR 0003）的独立决定，并非 GPLv3 固有限制——「不给官方 lobby/relay 访问权」是 `vera20k` 在 GPLv3 之上自行附加的 Section 7 条款，仅在真的引入其代码时才随之进入。
- 客户端与 relay 一起发布，不承诺旧客户端兼容。
