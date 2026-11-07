## Context

P0 保证跨平台确定性（ADR 0002），P2 有玩法。`vera20k` 的 `src/net/` 与 `services/relay-protocol` 提供了生产级的 lockstep、UDP 可靠层、认证数据报、排序时钟、自适应冗余、断线重连与回放设计，但其 Event 线格式复刻 RA2 `FRAMEINFO` / `FRAMESYNC`（`64B4D0` 等），不能照搬。

## Goals / Non-Goals

**Goals:** 双客户端同图对战、逐帧 checksum 对比、断线恢复；与官方服务器无关。

**Non-Goals:** gamemd/IPX 线兼容；官方战网/BNET；预测回滚；反作弊；第一版 30 人规模承诺。

## Decisions

- **借鉴架构、重写线格式**：移植 lockstep / reliable UDP / relay 时钟的设计，定义自己的 Event 包。否决复用 RA2 原生包布局。
- **relay 持有时钟**：2 人以上对局用 relay 认证时钟与不可变命令流；LAN 直连走独立 UDP。否决"纯 P2P 无权威时钟"。
- **只传命令**：由既定队列相位把命令交给模拟；`sim/` 不读 socket。否决传输层持有第二份命令队列。
- **自建服务**：按 ADR 0001，GPLv3 不授予官方 lobby/relay 访问权。

## Risks / Trade-offs

- [客户端与 relay 需同步发布] → 不承诺旧版本兼容，版本不匹配明确拒绝。
- [在线对局的 SaaS 化不在 GPLv3 覆盖内] → 见 ADR 0001；必要时评估 AGPLv3。
- [30 人 / 20,000 单位规模未验证] → 第一版以双客户端与小规模验收为准。

## Migration Plan

新增 `w3-net` / `w3-relay`；app 层接入。不改动 `w3-core` 权威状态语义。

## Open Questions

- Event 编解码与压缩缓存的具体格式。
- 是否引入 relay 自适应调速（替代原生"降到最慢玩家"）。
