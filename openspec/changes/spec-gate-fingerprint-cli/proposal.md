# Proposal

## Why

规格门（`CODEBUDDY.md` §9）批准后须把四件套 sha256 指纹回填 `decision-memo.md`，但 `verify_all.py` 只提供内部函数 `spec_artifact_fingerprint()`（`tools/verify_all.py:158`），取指纹要靠 `python -B -c "import ..."` 绕路——那是 `8093cd9` 交付说明里给出的临时命令，未受版本控制、易拼错，且 Windows / Linux 的引号与路径写法不同。本 change 把「按 change 名取指纹」做成统一门禁的正式只读子命令，让规格门批准动作可复现、可审查。

本 change 同时是工作流闭环的**首次从零演练**：验证 `93cf95c`…`b3b7b64` 落地的决策纪要、规格门、锁定指纹、`critic` 第 10 / 11 项与反向验证规则能否真正跑通、有无互相打架。

## What Changes

- `tools/verify_all.py` 新增 `--fingerprint <change>` 只读子命令：解析 active change 名，输出该 change 四件套的 sha256（单行、纯 64 位小写 hex），退出码 0；未知 change 名或四件套缺失时以英文结构化信息报错、非零退出，不静默回落、不写文件。
- 全量门禁入口 `main()`（`tools/verify_all.py:239`）保持不变；子命令不执行任何检查、不写文件、不触发 `cargo test` / `openspec validate`，与现有 `python tools/verify_all.py` 行为完全隔离。
- 新增 `verification-cli` capability，固化该子命令的可观察行为契约。
- 新增 Python 标准库单元测试与**反向验证（改坏验红）**用例。
- **BREAKING：无**。不改任何既有公开签名、不删改既有命令行为、不新增依赖。

## Capabilities

### New Capabilities

- `verification-cli`: 统一门禁的只读查询接口——按 change 名输出四件套指纹，以及参数错误与缺失输入的显式失败行为。

### Modified Capabilities

（无。）本 change 不改写任何既有 capability 的 requirements；`deterministic-simulation` / `playable-client` / `cross-platform-determinism` 等行为不变。

## Impact

- **代码面**：仅 `tools/verify_all.py`（新增参数解析与只读分支）与新增 Python 测试文件；**不触碰 `crates/` 下任何 Rust 源码**。
- **API / 依赖**：无新增依赖；`spec_artifact_fingerprint()` 保持原签名与字节口径（子命令复用它、不重构），确保 `spec_gate_check()` 的锁定判定口径不变。
- **确定性（ADR 0002）**：`crates/w3-core/` 零改动；不新增权威状态字段，`SNAPSHOT_VERSION` 仍为 3，状态哈希字段集合与快照头部布局不变；`cargo test -p w3-core --test determinism`（26 项）MUST 全绿。本 change 的指纹是**构建期文档摘要**，不进权威状态、不进状态哈希、不进快照。
- **资产与法律（ADR 0001）**：仓库零资产；无新增第三方依赖，许可证无变化，`NOTICE.md` / `Cargo.lock` 不变。
- **阶段与范围**：P7 工作流工具改进（P0–P6 已归档，`melee-playable-loop` 在飞；`openspec/config.yaml:48` 的 `rules.proposal` 措辞为 P0–P5，此处按项目实际阶段标注）。范围外：不改规格门批准流程本身、不改 `decision-memo.template.md`、不提供批量指纹、不提供写回 `decision-memo.md` 的能力（回填仍由 `spec-steward` 在人拍板后落笔，脚本不得自批）。
- **人工边界**：本 change 无 GPU / 真窗口人工验收项；演练结论由 `critic` 报告与 `verify_all.py` 门禁给出。
