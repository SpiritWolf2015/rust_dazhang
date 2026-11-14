# Design

## Context

动机见 `proposal.md` Why，不在此重述。现状（只读核实）：`tools/verify_all.py` 是单入口全量门禁，`main()` 无参数解析（`tools/verify_all.py:239-256`）；`spec_artifact_fingerprint()`（`tools/verify_all.py:158`）已按「相对路径 + 内容」计算四件套 sha256，`spec_gate_check()`（`tools/verify_all.py:190`）复用它判定规格锁定；`active_change_names()`（`tools/verify_all.py:47`）列出 active change。规格门批准时需要把该值回填 `decision-memo.md`，但当前只能靠 `8093cd9` 交付说明里那条未受版本控制的 `python -B -c` 临时命令取得。约束：ADR 0001（零资产）、ADR 0002（`w3-core` 确定性硬门禁，`SNAPSHOT_VERSION` 当前 3）、`CODEBUDDY.md` §2（工具脚本跨平台 Python / Node，平台差异在脚本内判断）、§3（日志 / 错误英文、注释中文）、§4（AI 不碰远端）、§5（回归测试反向验证）。

## Goals / Non-Goals

**Goals:**

- 把「按 change 名取四件套指纹」做成受版本控制、跨平台一致的正式只读子命令。
- 子命令与全量门禁严格隔离，且复用同一指纹口径，杜绝「批准时算出的值」与「校验时算出的值」不一致。
- 失败显式、不静默降级。
- 借此首次从零演练 `93cf95c`…`b3b7b64` 落地的工作流闭环，暴露规则之间是否打架。

**Non-Goals:**

- 不写回 `decision-memo.md`（回填由 `spec-steward` 在人拍板后落笔，脚本不得自批）。
- 不批量输出多个 change 的指纹、不改 `decision-memo.template.md`、不改规格门状态机。
- 不重构 `spec_artifact_fingerprint()` 的算法与字节口径。
- 不改动 `crates/` 下任何 Rust 源码。

## Decisions

### D1 以 `--fingerprint <change>` 子命令扩展单入口，不新增独立脚本

**决策**：在 `tools/verify_all.py` 的 `if __name__ == "__main__"` 处做最小 `sys.argv` 解析；命中 `--fingerprint` 时调用 `spec_artifact_fingerprint()` 并 `print` 单行 hex 后退出，其余情况仍走 `main()`。

**备选方案**：

1. 新增 `tools/fingerprint.py` 独立脚本 —— 否决：同一算法出现两份入口，易与 `spec_gate_check()` 口径漂移（违反单一真值源），且多一个文件要维护跨平台 / 编码基线。
2. 保持 `python -B -c` 临时命令，只写进文档 —— 否决：命令未受版本控制、易拼错、Windows / Linux 引号与路径写法不同，且无法被 `critic` 审查。
3. 引入 `argparse` 子命令框架 —— 否决：当前只有一个开关，帮助体系属过度工程；最小 `sys.argv` 判断即可，开关增多再重构。

**ADR 0002 / 跨平台确定性影响**：该子命令只读 `openspec/changes/<name>/` 下的 UTF-8 文本字节，不参与模拟、不进入状态哈希或快照；`spec_artifact_fingerprint()` 字节口径不变，故 Windows / Linux 对同一 change 输出同一 hex（换行由仓库 LF 基线保证）。**不改变任何跨平台确定性结论**，也不得据本 change 声称 ADR 0002 有新验证。

**架构原则对照**：SRP（子命令只做「读 + 输出」）、OCP（扩展入口而非改写 `main()`）、DIP / CARP（复用既有函数而非复制算法）、显式失败（`problem=` + 非零退出码）；不触及分层依赖、冻结输入、唯一状态所有者等硬红线。

### D2 未知 change 与缺四件套一律 fail-loud

**决策**：先校验 change 名在 `active_change_names()` 中、且 `proposal.md` / `design.md` / `tasks.md` 存在；不满足则英文结构化报错、非零退出、标准输出无 hex。`specs/` 为空不视为不完整，沿用 `spec_artifact_fingerprint()` 既定的空字节口径（与 `spec_gate_check()` 同源，不构成批准值 / 校验值漂移）；无 `.openspec.yaml` 的目录不在 active 列表，归 `problem=unknown change`。

**备选方案**：

1. 对未知 change 输出空指纹并以 0 退出 —— 否决：静默降级会把空值写进 `decision-memo.md`，规格门随后失配，排查成本高。
2. 自动补齐缺失四件套 —— 否决：脚本写 artifact 等于绕过 `spec-steward` 与规格门，越权。

**ADR 0002 / 跨平台确定性影响**：纯输入校验，不触碰模拟状态；只用 `pathlib`，不依赖路径分隔符字面量，无跨平台差异。

**架构原则对照**：显式失败不静默降级（硬红线）、ISP（子命令不暴露写能力）。

### D3 反向验证用「测试内反例元断言」，不依赖人工改坏代码

**决策**：单元测试构造两个临时 change 目录（四件套仅 `design.md` 差一字节），断言 `spec_artifact_fingerprint()` 输出不同；构造一个「`tasks.md` 仅复选框翻转」的目录，断言输出相同。再定义一个**退化的 `broken_fingerprint()`**（故意漏读 `design.md`），断言上述「不同」用例在退化实现下**变相同**——以此证明测试不是恒真，能抓到「漏读文件」这类缺陷。

**备选方案**：

1. 只写「指纹稳定」的正向断言 —— 否决：正向断言在缺陷实现下也可能全绿，正是 `CODEBUDDY.md` §5 反向验证规则要防的。
2. 人工 `git stash` 改坏代码验红 —— 否决：不可复现、不进 CI、无法留证据。

**ADR 0002 / 跨平台确定性影响**：测试只在临时目录构造 UTF-8 文本，不触碰 `w3-core`；本 change 仍包含重跑 `tests/determinism.rs` 的任务（第 3 组）以确认零影响。

**架构原则对照**：正确性优先、可验证性；不引入新依赖（Python 标准库 `unittest` / `tempfile` / `hashlib`）。

## Risks / Trade-offs

- `sys.argv` 最小解析不提供 `--help` / 组合参数；当前只有一个开关，属可接受简化，开关增多时再重构，不为假想需求预建框架。
- 子命令输出是**构建期文档摘要**：若 CRLF 或 BOM 混入，指纹会变；由 `check_encoding.py` 门禁兜住，不属本 change 范围。
- 演练性质：本 change 的首要目的是验证工作流闭环自洽。若 `critic` 规格门报告指出规则之间打架，`spec-steward` MUST 先回到规则 artifact 修订并重走规格门，不带着矛盾继续 `apply`。
- 批准前 `decision-memo.md` 规格门小节为 `status: pending`，`verify_all.py` 会报 `invalid spec gate status`——这是规格门未批准的正确信号，不是缺陷；人拍板并回填 `fingerprint:` 后即绿。是否给脚本增加明确的 `pending` 分支（报 `not approved yet`）列为演练后待确认项。
