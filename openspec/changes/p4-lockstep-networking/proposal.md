## Why

第一版的核心目标之一是联机。P0 已保证确定性，P2 已有玩法，但还没有把多端命令编排成相同帧的机制。P4 把 `vera20k` 的 lockstep / UDP 可靠层 / relay 架构的设计移植过来，形成自建联机。

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

- 新增 workspace 成员与自建服务；`sim/` 仍 MUST NOT 依赖 `net/`。
- 联机只走自建 lobby/relay，不兼容官方战网；GPLv3 也不授予 vera20k 官方服务器访问权。
- 客户端与 relay 一起发布，不承诺旧客户端兼容。
