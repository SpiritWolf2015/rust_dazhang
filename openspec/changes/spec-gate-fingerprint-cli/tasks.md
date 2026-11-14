# Tasks

> 按 `design.md` D1–D3 决策分组；`crates/` 下 Rust 源码零改动。第 1 组为 CLI 实现，第 2 组为单元测试与反向验证，第 3 组为门禁、确定性复核与演练结论。每组自带验证命令与证据要求。

## 1. `--fingerprint` 只读子命令

- [x] 1.1 在 `tools/verify_all.py` 的 `__main__` 入口增加最小 `sys.argv` 解析：命中 `--fingerprint <change>` 时走只读分支，否则仍调 `main()`；未知开关 / 缺参数以 `problem=bad arguments` 非零退出；验证：不带参数运行的行为与改动前逐项一致
- [x] 1.2 只读分支校验 change 名在 `active_change_names()` 且 `proposal.md` / `design.md` / `tasks.md` 存在；不满足时英文结构化 `problem=` 报错、非零退出、标准输出无 hex；`specs/` 为空不拦截（沿用空字节口径），无 `.openspec.yaml` 归 `problem=unknown change`；验证：`--fingerprint no-such-change`、「仅 scaffold 的 change」、「无 `.openspec.yaml` 的目录」三个负向用例 + 一个「`specs/` 为空仍输出 hex」用例
- [x] 1.3 复用 `spec_artifact_fingerprint()` 输出**单行 64 位小写 hex** 并退出 0；MUST NOT 重构该函数字节口径；验证：输出与 `spec_artifact_fingerprint(CHANGES_DIR / name)` 返回值逐字节相等（见 2.5），且与 `spec_gate_check()` 对同一 change 的记录值一致
- [x] 1.4 子命令不写文件、不跑 `cargo test` / `openspec validate` / 资产扫描；验证：执行前后 `git status --short` 中该 change 目录无变化

## 2. 单元测试与反向验证（改坏验红）

- [x] 2.1 新增 Python 标准库 `unittest` 用例：两个临时 change 目录四件套仅 `design.md` 差一字节 → 指纹不同；验证：用例通过且可重复
- [x] 2.2 新增用例：`tasks.md` 仅 `- [ ]` ↔ `- [x]` 翻转 → 指纹相同（与 `spec_gate_check()` 的归一化口径一致）；验证：用例通过
- [x] 2.3 **反向验证（`CODEBUDDY.md` §5，本 change 属演练性主动加严）**：定义退化的 `broken_fingerprint()`（故意漏读 `design.md`），证明 2.1 的「不同」断言在退化实现下**变相同**、在正式实现下不同；把退化对照与命令输出写进本组验证记录，仅「全绿」不算完成；验证：退化版必被抓出、正式版必通过，两条证据都在
- [x] 2.4 负向用例：未知 change 名 / 缺 `design.md` / 无 `.openspec.yaml` / 缺参数 → 非零退出 + 英文 `problem=`，无 hex；`specs/` 为空 → 退出 0 且输出 hex；验证：全部用例通过
- [x] 2.5 新增 CLI 正向断言：捕获 `fingerprint_cli()` 的标准输出，与 `spec_artifact_fingerprint(CHANGES_DIR / name)` 返回值**逐字节相等**、退出码 0（承接 `spec.md` Scenario「对 active change 取指纹」）；另用 `subprocess` 跑真实入口验证 `__main__` 分发与 `problem=unknown change`；验证：用例通过

## 3. 全量门禁、确定性复核与演练结论

- [x] 3.1 `cargo fmt --all -- --check`、`python tools/check_encoding.py`、`git diff --check`；验证：退出码 0、无 BOM / CRLF / 行尾空格
- [x] 3.2 重跑 `cargo test -p w3-core --test determinism`（26 项）与 `cargo test --workspace`；验证：全绿，`git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3
- [x] 3.3 `openspec validate spec-gate-fingerprint-cli --strict` 与 `python tools/verify_all.py`（含 `decision memo OK` / `spec gate OK`）；验证：`Verification OK`、无资产入库
- [x] 3.4 由只读 `critic` 按全清单（含第 10 项 Spec 与决策纪要质量、第 11 项整体调用路径与集成一致性）复审**本 change 全部 diff**；验证：报告按「问题 / 位置 / 触发场景 / 风险 / 建议」区分 blocker / residual，明确已核查项，无空泛结论
- [x] 3.5 记录演练结论：决策纪要 → 规格门（人拍板 + 指纹回填） → apply → 反向验证 → 代码门 → 门禁 是否跑通、8 个工作流提交的规则之间有无打架；未跑通的环节 MUST NOT 标为通过，写入 `decision-memo.md` 待确认项
- [x] 3.6 以中文描述 + Conventional Commits 英文 type 前缀本地 commit；验证：`git log -1` 中文描述首行 ≤ 72 字符；`git push` 由人工执行（AI 不碰远端）
