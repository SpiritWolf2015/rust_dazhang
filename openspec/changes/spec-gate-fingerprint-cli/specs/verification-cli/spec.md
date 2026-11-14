# Spec Delta

## Purpose

统一门禁需要对外提供可复现、可审查的只读查询能力，让规格门批准时取四件套指纹不再依赖未受版本控制的临时命令，且与校验侧共用同一口径。

## ADDED Requirements

### Requirement: 输出指定 change 的四件套指纹

`python tools/verify_all.py --fingerprint <change>` SHALL 在标准输出打印**单行、纯 64 位小写 hex** 的 sha256，其值为该 change 的 `proposal.md`、`design.md`、`specs/**/*.md`（路径稳定排序）与 `tasks.md`（复选框状态归一化）按 `spec_artifact_fingerprint()` 口径计算的内容指纹；SHALL 以退出码 0 结束。

该命令 MUST NOT 执行 `cargo test` / `openspec validate` / 资产扫描 / 决策纪要校验 / 规格门校验，MUST NOT 写任何文件。该指纹是构建期文档摘要，不构成权威状态，**不进入状态哈希与由 `SNAPSHOT_VERSION`（当前 3）定义的快照**（ADR 0002）。

#### Scenario: 对 active change 取指纹

- **WHEN** 对含完整四件套的 active change 执行 `python tools/verify_all.py --fingerprint spec-gate-fingerprint-cli`
- **THEN** 标准输出为单行 64 位小写 hex，与 `spec_artifact_fingerprint(CHANGES_DIR / "spec-gate-fingerprint-cli")` 返回值逐字节相同，退出码 0

#### Scenario: 与规格门口径一致

- **WHEN** 把该命令输出写入 `decision-memo.md` 规格门小节的 `fingerprint:` 字段
- **THEN** `python tools/verify_all.py` 的 `spec_gate_check()` 对该 change 不报 `fingerprint mismatch`

#### Scenario: 全量门禁行为不变

- **WHEN** 不带任何参数执行 `python tools/verify_all.py`
- **THEN** 仍依次执行编码 / fmt / 全量测试 / openspec validate / 资产扫描 / decision memo / spec gate 检查，输出 `Verification OK`，与引入子命令前一致

### Requirement: 参数与输入的失败必须显式

命令 SHALL 对未知 change、四件套不完整、参数错误三类输入以非零退出码结束，并在标准错误输出**英文**结构化信息（含 `problem=` 与所给 change 名）；MUST NOT 静默回落到空指纹、MUST NOT 打印 hex、MUST NOT 创建或补齐四件套。完整性以 `proposal.md` / `design.md` / `tasks.md` 为准；`specs/` 为空沿用 `spec_artifact_fingerprint()` 既定空字节口径，与 `spec_gate_check()` 同源。

#### Scenario: 未知 change 名

- **WHEN** 执行 `python tools/verify_all.py --fingerprint no-such-change`
- **THEN** 退出码非 0，标准错误含 `problem=unknown change`，标准输出无 64 位 hex

#### Scenario: 四件套不完整的 active change

- **WHEN** 对有 `.openspec.yaml` 但缺 `design.md` 的 active change 执行该命令
- **THEN** 退出码非 0，标准错误含 `problem=missing artifacts`，不输出指纹

#### Scenario: 无 `.openspec.yaml` 的目录

- **WHEN** 对存在但无 `.openspec.yaml` 的目录名执行该命令
- **THEN** 退出码非 0，标准错误含 `problem=unknown change`，不输出指纹

#### Scenario: `specs/` 为空不静默降级

- **WHEN** 对 `proposal.md` / `design.md` / `tasks.md` 齐备但无 `specs/` 的 active change 执行该命令
- **THEN** 退出码 0，输出按 `spec_artifact_fingerprint()` 既定空字节口径计算的 64 位 hex，不报 `missing artifacts`

#### Scenario: 参数错误

- **WHEN** 执行 `python tools/verify_all.py --fingerprint`（缺 change 名）或带未知开关
- **THEN** 退出码非 0，标准错误含 `problem=bad arguments`，标准输出无 hex

#### Scenario: 不写文件

- **WHEN** 对任一 change 执行该命令前后
- **THEN** `git status --short` 中该 change 目录无新增或修改条目
