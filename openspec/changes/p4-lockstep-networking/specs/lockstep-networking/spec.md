## ADDED Requirements

### Requirement: Transmit only input commands

联机 SHALL 只传输玩家输入命令，MUST NOT 传输游戏状态；每个客户端 SHALL 运行相同模拟代码并按相同帧序执行相同命令。

#### Scenario: 相同命令流

- **WHEN** 两个客户端收到相同的冻结命令帧
- **THEN** 逐帧 `compute_state_hash()` 相等

### Requirement: Reliable UDP delivery and ordering

系统 SHALL 在 UDP 之上提供可靠发送队列、重传与按帧排序；到达顺序 MUST NOT 决定执行顺序；缺失输入 SHALL 按既定帧边界处理而非阻塞无限等待。

#### Scenario: 丢包重传

- **WHEN** 一个命令包丢失但被重传补齐
- **THEN** 该命令仍在原定帧执行，且不改变后续帧序

### Requirement: Per-frame checksum comparison

多人对局 SHALL 交换并比对逐帧校验值；不一致 MUST 被检测并可定位到首个差异字段。

#### Scenario: 检测到 desync

- **WHEN** 某客户端状态哈希与对端不一致
- **THEN** 报告差异帧与 `first_difference()` 字段名，不静默继续

### Requirement: Reconnection and replay recovery

断线客户端 SHALL 保留身份与输入游标，并在恢复后读取错过的帧；重放 MUST NOT 重复执行已提交的命令或历史事件。

#### Scenario: 短暂断线

- **WHEN** 客户端断线后重新接入
- **THEN** 追上当前帧且状态哈希与健康客户端一致
