# CODEBUDDY.md

本文件是 `rust_dazhang` 项目的**单一权威开发约定**。CodeBuddy Code 与所有 subagent 每次会话自动加载；`CONTRIBUTING.md`、`openspec/config.yaml`、CI 配置只引用本文件，不另立冲突规则。规则变更先改这里。

## 1. 项目速览

- GPLv3、Rust edition 2024 虚拟 workspace；`w3-core`（零依赖确定性模拟内核）、`w3-content`（读取本机导出 JSON）。
- 阶段：P0–P6 全部已实施并归档（最新归档 `openspec/changes/archive/2026-11-10-cross-platform-golden-hash/`）；当前 2 个 active OpenSpec change：`gameplay-command-wire-format`（9/54，未实施）、`playable-window-smoke`（0/35，P6 窗口冒烟入口提案，提案中）。主 specs 共 10 个：`3d-rendering`、`content-import`、`cross-platform-determinism`、`deterministic-simulation`、`gameplay-systems`、`linux-platform-backend`、`lockstep-networking`、`pathfinding`、`playable-client`、`unit-orders`。
- 参考项目只读（共四个）：`vera20k`(GPLv3)、`godot_warcraft3`(MIT)、`rts_kernel_cs`(MIT)、`WarsmashModEngine`(AGPLv3)，见 `docs/references/UPSTREAM.md` 与 `NOTICE.md`。
- 架构设计原则（SRP / OCP / LSP / DIP / ISP / LoD / CARP 与本项目硬红线）已内联到 `openspec/config.yaml` 的 context 与 `critic.md` 审查清单；本文件与 ADR 为单一权威，不另立独立原则文档。

## 2. 编码与换行（硬性基线）

- 所有源码、配置与文档文件：**UTF-8 无 BOM**、**LF** 换行、文件末尾有换行、无行尾空格。
- 权威文件：`.editorconfig`（编辑器）、`.gitattributes`（覆盖 `core.autocrlf=true`，消除 CRLF 警告）。
- 检查命令：`python tools/check_encoding.py`（本地 pre-commit 与 CI 都执行）。
- **Rust 源码格式一律遵循 Rust 官方 `rustfmt` 默认风格**（仓库无 `rustfmt.toml` / `.rustfmt.toml`）。所有写 Rust 源码的 subagent 写完后 MUST 运行 `cargo fmt --all` 格式化；提交前 MUST 运行 `cargo fmt --all -- --check` 校验，不通过不得提交。
- Windows 注意：PowerShell 5.1 的 `-Encoding utf8` **会写入 BOM**，默认更是 UTF-16 LE；**禁止用 PowerShell 生成源码或配置文件**，改用 Write 工具、Node 或 Python（写文件时用 `UTF8Encoding($false)`）。终端使用 Windows Terminal 或 `chcp 65001`。
- **工具脚本一律用跨平台运行时**：需要写工具 / 校验 / 生成脚本时，优先 Python（`tools/*.py`）或 Node（`tools/*.mjs`），**不写 Windows PowerShell（`.ps1`）或平台专属 shell 脚本（`.sh` / `.bat` / `.cmd`）**；入口统一为 `python tools/xxx.py` / `node tools/xxx.mjs`。这样既支持 Windows / Linux 跨平台使用，也避开上一条 PowerShell 5.x 无法输出 UTF-8 无 BOM 的编码坑。需要平台差异时在脚本内用 `os.name` / `process.platform` 判断，不维护两套脚本（现有 `verify_all.py::openspec_command()` 即为例）。
- 本条只约束**工具链**跨平台；[ADR 0002](docs/decisions/0002-determinism.md) 的「模拟内核跨平台确定性」仍是待验证项，不得据本条声称已达成。
- 中文注释本身不会乱码；乱码的根因是编码/换行不一致，不靠改变注释语言解决。注释语言是团队约定，编码基线才是防乱码手段。

## 3. 注释、日志与文案

- **源码注释一律中文**，包括模块级 `//!`、所有 `pub` API 的 rustdoc `///`、行内 `//`；现有英文注释已全部重写。
- **日志输出、错误信息一律英文**：`log` crate + `RUST_LOG`，结构化 `key=value`；`thiserror` / `anyhow` 错误文本与 `panic!` 文本用英文；禁止把中文路径或中文消息拼进日志，禁止散落的 `println!`。
- **UI 文案走 i18n，可用中文**；UI 文本不进日志、不进权威状态、不进状态哈希。
- 工具脚本（`tools/*.py`）的注释中文、运行输出英文，同样遵守本条。
- **git 提交信息（commit message）描述一律中文**，与源码注释规则一致；Conventional Commits 的英文 type 前缀（`feat:` / `fix:` / `docs:` / `chore:` 等）保留，仅描述文本用中文。
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
  3. **CI / 本地门禁**：`tools/verify_all.py` + `.github/workflows/ci.yml`；CI 用 matrix 在 `ubuntu-latest` 与 `windows-latest` **双平台跑同一 `python tools/verify_all.py` 入口**，验证工具链跨平台（`run` 只写单行命令、不含 shell 语法，平台差异由 `verify_all.py` 内部 `os.name` 处理）。这**不等于** ADR 0002 模拟内核跨平台确定性已验证；若托管平台非 GitHub，把同一命令配到其流水线。
- 提交规范：Conventional Commits（英文 type 前缀 `feat:` / `fix:` / `docs:` / `chore:` 保留）、**中文描述首行 ≤ 72 字符**（语言规则见第 3 节）、一个改动一个 commit / PR、PR 描述约 15 行。

## 5. 确定性硬门禁（ADR 0002）

- `SimFixed` Q47.16 整数数学；整数帧、30 Hz 冻结；命令按 `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定排序；稳定 `EntityId` 顺序；FNV-1a 状态哈希；快照（`SNAPSHOT_VERSION`，当前 3）是帧提交后的完整状态，头部含 `defs_hash` / `content_hash`。
- 任何改动 `crates/w3-core/` 的实现，必须通过 `cargo test -p w3-core --test determinism`；新增权威状态字段必须**同时**进入状态哈希与快照。
- `w3-core` 不得依赖 `w3-content` / `render` / `ui` / `audio` / `net`；表现层不得反写模拟状态；`w3-content` 的原始 `f64` 坐标是冻结输入，不进快照与哈希。
- 跨平台确定性已在 Windows 与 Debian 12 x86_64 / rustc 1.99.1 通过离线 golden hash 逐字节比对验证（见 ADR 0002 复核记录与 `openspec/specs/cross-platform-determinism/spec.md`）；macOS、其它 CPU 架构（ARM64 等）与其它 rustc 版本仍属待验证项，不得声称全平台达成。

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
| 格式（提交前校验） | `cargo fmt --all -- --check` |
| 格式（写完自动排版） | `cargo fmt --all` |
| 编码 | `python tools/check_encoding.py` |
| 统一门禁 | `python tools/verify_all.py` |
| 端到端加载 | `cargo run -p w3-content --example load_export -- <map-dir> <table.json>` |
| OpenSpec | `openspec list` / `openspec validate <change>` / `openspec archive <change> -y` |

## 8. OpenSpec 工作流

- schema `spec-driven`，artifact 顺序 `proposal → specs → design → tasks`；项目上下文与 per-artifact 规则在 `openspec/config.yaml`。
- 新建提案：`openspec new change <name>` → `openspec status --change <name> --json` → `openspec instructions <artifact> --change <name> --json` → 写文件 → `openspec validate <name>`；完成后 `/opsx:archive`。
- 已归档的 P0–P6 在 `openspec/changes/archive/`；主 specs 是 source of truth。
- **ADR 记"为什么决定"，OpenSpec 记"系统该做什么"**；spec 必须引用 ADR 编号，避免两份规范打架。
- **决策纪要**：每个 change 在 `proposal` 阶段同步产出 `decision-memo.md`（模板 `openspec/decision-memo.template.md`）。只记三类：拍板决策（含被否备选与理由）、承重约束（逐条带 `file:line` / commit / 命令输出证据锚）、待确认项；**无证据项 MUST NOT 写成「用户已确认」，不拿沉默当授权**。能自查的事实不问人，不确定先给「推荐 + 依据 + 代价 + 备选」再请人拍板。
- 决策纪要是纯文档，不进状态哈希 / 快照、不含资产；目标与方向已清楚的小改动可显式声明省略，但范围 / 方案澄清、事实核对、决定确认三件事不可省。
- **成文顺序与四件套一致性**：决策纪要把承重约束与拍板决策逐条站稳后，才进入成文，按 `proposal → specs → design → tasks`（`/opsx:propose`）在同一轮提案流程内生成完整四件套；MUST NOT 先成文再靠局部补改收敛（会让 proposal 的 Scope、specs 的 `#### Scenario:`、design 的 alternatives、tasks 的步骤互相打架）。实施返工确需改动任一 artifact 时，MUST 同步回查其余三份并一起更新，不允许只改一份；由 `critic` 清单第 7 / 10 项复核。
- 一个 change 一个 owner，配一个只读 critic；`mode` 不使用 `bypassPermissions`。

## 9. Subagent 工作流

| Subagent | 职责 | 可写范围 |
|---|---|---|
| `spec-steward` | OpenSpec change 制作 / validate / archive，维护 ADR 与 spec 一致性 | `openspec/`、`docs/decisions/` |
| `sim-owner` | `w3-core` 唯一实现所有者，守 ADR 0002 | `crates/w3-core/` |
| `content-owner` | `w3-content`，守资产合规 | `crates/w3-content/` |
| `client-owner` | P6 可玩客户端集成层唯一实现所有者，守 ADR 0002 与只读表现边界 | `crates/w3-client/`、`crates/w3-render/` |
| `net-owner` | `w3-net` / `w3-relay` 网络传输层唯一实现所有者，守 ADR 0002 与 ADR 0005 canonical 约束 | `crates/w3-net/`、`crates/w3-relay/` |
| `asset-pipeline` | 跑上游 dev-setup、验证导出、记录 schema 变化 | 只读上游仓库，产物留本机 |
| `critic` | 审查确定性、所有权、许可证、编码、资产入库 | **只读，不写任何文件** |
| `docs-owner` | README / NOTICE / CONTRIBUTING / UPSTREAM 同步 | 文档文件 |

- 定义文件在 `.codebuddy/agents/`；每个 subagent 声明 own files 与 tools，继承 `permissions.deny`；critic 只读；Agent Team 的 leader 只编排不写。
- 串联（**一次规格门 + 一次代码门**）：`/opsx:propose` → `spec-steward` 产出四件套与 `decision-memo.md` → **规格门**：`critic` 按清单第 7 / 10 项与 §8 四件套一致性审规格，按输出格式出报告，**由人拍板**（不拿沉默当授权）；批准记录（`status: approved` + 四件套指纹）写进 `decision-memo.md` 的「规格门」小节（格式见 `openspec/decision-memo.template.md`），此后**规格锁定**并由 `verify_all.py` 机械校验 → `/opsx:apply`（对应 owner）→ `critic` 代码门（全清单）→ 本地 commit → **人工 push** → CI 在 Windows / Linux 双平台跑 `python tools/verify_all.py` 门禁 → `/opsx:archive`。
- **规格锁定与重审**：实现期确需改动四件套任一 artifact 即视为解锁（指纹失配由 `verify_all.py` 判定），MUST 按 §8 同步回查其余三份，由 `spec-steward` 修订后**重走规格门**（重新出报告、重新由人拍板），不得由实现方自行放行。目标与方向已清楚的小改动、纯文档 / 规则改动可显式声明省略规格门，但人工清单如实标注与 `verify_all.py` 门禁不可省。
- **Rust 源码格式化门禁（所有写 Rust 的 subagent）**：写代码时即按 `rustfmt` 默认风格组织；写完后先 `cargo fmt --all`，提交前 `cargo fmt --all -- --check`；`critic` 在提交前 review MUST 复跑该校验，不通过不得提交。提交信息描述按第 3 节用中文。
- **只读参考项目共四个**（见第 1 节 / `docs/references/UPSTREAM.md`）：`WarsmashModEngine` 是 Java / LibGDX 的 WC3 模拟器、AGPLv3，仅作 JASS / 玩法 / 格式语义参考；四个项目均不复制代码进仓库，AGPLv3 与 GPLv3 组合有条件，未经 ADR 复核不得引入其代码。
- **工具脚本跨平台（所有 subagent）**：需要脚本时用 Python（`tools/*.py`）或 Node（`tools/*.mjs`），不写 `.ps1` / `.sh` / `.bat` / `.cmd`；平台差异在脚本内判断，不维护两套。规则与理由见第 2 节。

## 10. AI 与人工的责任边界

- **AI**：实现、测试、文档、本地提交、本地只读查询、本地门禁执行。
- **人工独占**：`push` / `fetch` / `pull` / `clone` 及任何远端变更、发布、合并 PR、服务器与密钥操作、许可证变更、依赖重大升级。
