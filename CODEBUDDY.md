# CODEBUDDY.md

本文件是 `rust_dazhang` 项目的**单一权威开发约定**。CodeBuddy Code 与所有 subagent 每次会话自动加载；`CONTRIBUTING.md`、`openspec/config.yaml`、CI 配置只引用本文件，不另立冲突规则。规则变更先改这里。

## 1. 项目速览

- GPLv3、Rust edition 2024 虚拟 workspace；`w3-core`（零依赖确定性模拟内核）、`w3-content`（读取本机导出 JSON）。
- 阶段：P0 / P1 已实现并归档，P2–P5 为 active OpenSpec changes（见 ADR 0003）。主 specs：`deterministic-simulation`、`content-import`。
- 参考项目只读：`vera20k`(GPLv3)、`godot_warcraft3`(MIT)、`rts_kernel_cs`(MIT)，见 `docs/references/UPSTREAM.md` 与 `NOTICE.md`。

## 2. 编码与换行（硬性基线）

- 所有源码、配置与文档文件：**UTF-8 无 BOM**、**LF** 换行、文件末尾有换行、无行尾空格。
- 权威文件：`.editorconfig`（编辑器）、`.gitattributes`（覆盖 `core.autocrlf=true`，消除 CRLF 警告）。
- 检查命令：`python tools/check_encoding.py`（本地 pre-commit 与 CI 都执行）。
- Windows 注意：PowerShell 5.1 的 `-Encoding utf8` **会写入 BOM**，默认更是 UTF-16 LE；**禁止用 PowerShell 生成源码或配置文件**，改用 Write 工具、Node 或 Python（写文件时用 `UTF8Encoding($false)`）。终端使用 Windows Terminal 或 `chcp 65001`。
- 中文注释本身不会乱码；乱码的根因是编码/换行不一致，不靠改变注释语言解决。注释语言是团队约定，编码基线才是防乱码手段。

## 3. 注释、日志与文案

- **源码注释一律中文**，包括模块级 `//!`、所有 `pub` API 的 rustdoc `///`、行内 `//`；现有英文注释已全部重写。
- **日志输出、错误信息一律英文**：`log` crate + `RUST_LOG`，结构化 `key=value`；`thiserror` / `anyhow` 错误文本与 `panic!` 文本用英文；禁止把中文路径或中文消息拼进日志，禁止散落的 `println!`。
- **UI 文案走 i18n，可用中文**；UI 文本不进日志、不进权威状态、不进状态哈希。
- 工具脚本（`tools/*.py`）的注释中文、运行输出英文，同样遵守本条。
- 例外：由工具生成、会被自动覆盖的文件不纳入手工注释语言规则，其文本由生成器决定——包括 `Cargo.lock`、OpenSpec archive 生成的主 spec 标题，以及 `.codebuddy/commands/` 与 `.codebuddy/skills/` 中 OpenSpec init 生成的指令文件。项目自己编写的 `.codebuddy/agents/` 必须中文。

## 4. Git 权限矩阵（AI 强制）

| 类别 | 命令 | 执行者 |
|---|---|---|
| 本地只读 | `git status` / `log` / `diff` / `show` / `blame` / `branch` | AI |
| 本地写入 | `git add` / `commit` / `tag`（本地）/ `stash` / `worktree` / `branch` 增删 | AI |
| **远端同步** | `git push`（含 `--force` / `--tags` / `--all`）、`git fetch`、`git pull`、`git clone`、`git submodule update`、`git remote add/set-url/remove/prune`、`gh pr create/merge/comment`、`gh repo/release`、`glab` | **仅程序员手动** |

- **所有与远端交换的命令一律由人工执行**，AI 不得代劳；这也包括"从远端同步到本地"的 `fetch` / `pull` / `clone`。
- 三层技术强制：
  1. **文档约定**：本文件 + `CONTRIBUTING.md`。
  2. **工具层**：`.codebuddy/settings.json` 的 `permissions.deny`（优先级最高，`bypassPermissions` 也不能绕过）+ `PreToolUse` hook `tools/guard_git_remote.py`（正则兜底，防止规则语法漏匹配）。
  3. **CI / 本地门禁**：`tools/verify_all.py` + `.github/workflows/ci.yml`（若托管平台非 GitHub，把同一命令配到其流水线）。
- 提交规范：Conventional Commits（`feat:` / `fix:` / `docs:` / `chore:`）、英文祈使句首行 ≤ 72 字符、一个改动一个 commit / PR、PR 描述约 15 行。

## 5. 确定性硬门禁（ADR 0002）

- `SimFixed` Q47.16 整数数学；整数帧、30 Hz 冻结；命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；稳定 `EntityId` 顺序；FNV-1a 状态哈希；快照 v1 是帧提交后的完整状态。
- 任何改动 `crates/w3-core/` 的实现，必须通过 `cargo test -p w3-core --test determinism`；新增权威状态字段必须**同时**进入状态哈希与快照。
- `w3-core` 不得依赖 `w3-content` / `render` / `ui` / `audio` / `net`；表现层不得反写模拟状态；`w3-content` 的原始 `f64` 坐标是冻结输入，不进快照与哈希。
- 跨平台确定性目前仅在 Windows / rustc 1.91 实跑；Linux / macOS 属待验证项，不得声称已达成。

## 6. 资产与法律（ADR 0001）

- 仓库**不含**任何暴雪资产，也不含 MPQ 派生的 JSON / PNG / GLB / 音效；导出数据留在玩家本机，由 gitignored 的 `config.toml` 提供路径。
- 禁止提交 `*.mpq` / `*.blp` / `*.mdx` / `*.mdl` / `*.w3x` / `*.w3m` / `*.w3e` / `*.doo` / `*.slk` 及导出目录内容；`.gitignore` 与 CI 资产扫描双重把关。
- 新增依赖前检查许可证与 GPLv3 兼容性；`Cargo.lock` 提交；第三方代码在 `NOTICE.md` 登记。
- 密钥、`config.toml`、relay ticket、`.env` 一律不入库。

## 7. 命令速查

| 用途 | 命令 |
|---|---|
| 单元测试 | `cargo test -p w3-core --lib` |
| 确定性验收 | `cargo test -p w3-core --test determinism -- --nocapture` |
| 内容层测试 | `cargo test -p w3-content` |
| 全量测试 | `cargo test --workspace` |
| 格式 | `cargo fmt --all -- --check` |
| 编码 | `python tools/check_encoding.py` |
| 统一门禁 | `python tools/verify_all.py` |
| 端到端加载 | `cargo run -p w3-content --example load_export -- <map-dir> <table.json>` |
| OpenSpec | `openspec list` / `openspec validate <change>` / `openspec archive <change> -y` |

## 8. OpenSpec 工作流

- schema `spec-driven`，artifact 顺序 `proposal → specs → design → tasks`；项目上下文与 per-artifact 规则在 `openspec/config.yaml`。
- 新建提案：`openspec new change <name>` → `openspec status --change <name> --json` → `openspec instructions <artifact> --change <name> --json` → 写文件 → `openspec validate <name>`；完成后 `/opsx:archive`。
- 已归档的 P0 / P1 在 `openspec/changes/archive/`；主 specs 是 source of truth。
- **ADR 记"为什么决定"，OpenSpec 记"系统该做什么"**；spec 必须引用 ADR 编号，避免两份规范打架。
- 一个 change 一个 owner，配一个只读 critic；`mode` 不使用 `bypassPermissions`。

## 9. Subagent 工作流

| Subagent | 职责 | 可写范围 |
|---|---|---|
| `spec-steward` | OpenSpec change 制作 / validate / archive，维护 ADR 与 spec 一致性 | `openspec/`、`docs/decisions/` |
| `sim-owner` | `w3-core` 唯一实现所有者，守 ADR 0002 | `crates/w3-core/` |
| `content-owner` | `w3-content`，守资产合规 | `crates/w3-content/` |
| `asset-pipeline` | 跑上游 dev-setup、验证导出、记录 schema 变化 | 只读上游仓库，产物留本机 |
| `critic` | 审查确定性、所有权、许可证、编码、资产入库 | **只读，不写任何文件** |
| `docs-owner` | README / NOTICE / CONTRIBUTING / UPSTREAM 同步 | 文档文件 |

- 定义文件在 `.codebuddy/agents/`；每个 subagent 声明 own files 与 tools，继承 `permissions.deny`；critic 只读；Agent Team 的 leader 只编排不写。
- 串联：`/opsx:propose` → `spec-steward` → `/opsx:apply`（对应 owner）→ `critic` → 本地 commit → **人工 push** → `/opsx:archive`。

## 10. AI 与人工的责任边界

- **AI**：实现、测试、文档、本地提交、本地只读查询、本地门禁执行。
- **人工独占**：`push` / `fetch` / `pull` / `clone` 及任何远端变更、发布、合并 PR、服务器与密钥操作、许可证变更、依赖重大升级。
