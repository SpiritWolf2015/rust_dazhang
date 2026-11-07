## Why

命令帧同步联机要求相同代码、内容、输入与随机流在所有平台逐帧产生相同状态。项目从零起步，必须先建立确定性内核，否则后续玩法、回放与联机都无从验证。

## What Changes

- 新增 `w3-core` crate：`SimFixed` Q47.16 定点数（i128 中间量）、整数帧与固定步长、`EntityId`/`EntityStore`、命令队列与稳定排序、FNV-1a 全状态哈希与首个差异字段定位、快照 v1。
- 新增 `RtsMatch` 作为唯一状态所有者，暴露 `create` / `submit_command` / `step` / `read_view` / `drain_events` / `capture_snapshot` / `restore_snapshot` / `compute_state_hash` / `first_difference`。
- 新增双实例交错、快照往返、命令回放三类确定性验收测试。
- 新增 ADR 0002（确定性契约）与 ADR 0003（范围分期）。

## Capabilities

### New Capabilities

- `deterministic-simulation`: 定点数、整数帧、命令排序、RNG 归属、状态哈希与快照的权威语义。

### Modified Capabilities

（无。项目初始，不存在既有 capability。）

## Impact

- 新增 workspace 成员 `crates/w3-core`，零外部依赖，edition 2024。
- 后续所有玩法逻辑 MUST 遵循此契约；改动权威状态、帧序、命令排序、哈希编码或快照字段前必须先通过 `crates/w3-core/tests/determinism.rs`。
- `w3-content`（P1）不得反向依赖内核；坐标转 `SimFixed` 的舍入规则在 P2 冻结。
