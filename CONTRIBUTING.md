# 贡献指南

开发约定以 [`CODEBUDDY.md`](CODEBUDDY.md) 为**单一权威**：UTF-8 无 BOM / LF 编码基线、源码注释（含 `pub` rustdoc）中文、日志与错误英文、UI 文案走 i18n、Rust 源码遵循官方 rustfmt 默认风格、以及 git 权限矩阵。本文件只引用，不另立规则。

**Git 边界（强制）**：AI 可执行 `status` / `log` / `diff` / `show` / `add` / `commit` 等本地操作；`push` / `fetch` / `pull` / `clone` / `submodule update` / `remote set-url` / `gh pr ...` 等所有远端同步命令**仅由程序员手动执行**，由 `.codebuddy/settings.json` 的 `permissions.deny` 与 `PreToolUse` hook 强制拦截。

## 许可证与署名

- 本项目采用 **GPLv3**（见 `LICENSE`、[ADR 0001](docs/decisions/0001-license.md)）。**提交贡献即表示你同意按 GPLv3 授权你的代码**，并确认你有权这样授权。
- **请勿提交**任何游戏资产、从 MPQ 解包或导出的文件（含 SLK / 地图 JSON、PNG / GLB / 音效），或原作的专有名称与美术素材。这些路径已被 `.gitignore` 排除；法律边界见 `README.md` 免责声明。
- 若引入任何第三方代码，保留其原始版权与许可证头，并在 `NOTICE.md` 登记来源。

## 开发前

```bash
cp config.toml.example config.toml   # 本机路径，不提交
cargo test -p w3-core --lib
cargo test -p w3-core --test determinism -- --nocapture
```

## 确定性是硬门禁

改动以下任何一项，必须先让 `crates/w3-core/tests/determinism.rs` 全绿，并在 PR 描述里说明影响（契约见 [ADR 0002](docs/decisions/0002-determinism.md)）：

- 权威状态字段、定点数语义
- 帧阶段顺序、`logic_fps`
- 命令排序键、命令冻结时机
- 状态哈希的字节编码或参与字段
- 快照 v1 字段或恢复校验

新增权威状态必须**同时**进入快照与状态哈希；新增内部决策必须保证稳定顺序，禁止 `HashMap` 迭代、墙钟、渲染帧率、异步完成顺序影响规则。表现层不得反写模拟状态。

## 测试与格式

- 每个修复或功能配 1–3 个聚焦回归测试，走生产代码，用边界用例，不做网格扫描或全量穷举。
- **写完 Rust 源码先运行 `cargo fmt --all`**，按 Rust 官方 `rustfmt` 默认风格自动排版（仓库无 `rustfmt.toml`）。
- **提交前 MUST 运行 `cargo fmt --all -- --check` 与 `cargo test --workspace`**，两者不通过不得提交；`critic` 在提交前 review 会复跑格式校验。
- 单元测试随模块写在 `#[cfg(test)]` 里（`--lib`）；跨模块验收写成集成测试（`--test <name>`）。

## 分支与提交

- 从 `main` 建 `feature/<topic>` 分支，PR 目标 `main`；不直接向 `main` 提交或推送。
- commit message 用 Conventional Commits：英文 type 前缀（`feat:` / `fix:` / `docs:` / `chore:` 等）保留，**描述一律中文**（与源码中文注释规则一致，见 `CODEBUDDY.md` 第 3 节），首行不超过 72 字符；正文用中文说明"改了什么 / 为什么 / 怎么验证"。
- 一个改动一个 PR，保持小；无关发现另开 Issue，描述位置与影响即可。
- 贡献者署名以你希望公开的 GitHub 用户名或姓名为准；Git 本地配置不会自动建立该偏好。

## 范围

第一版范围见 [ADR 0003](docs/decisions/0003-scope.md)：Melee 优先。JASS / 触发器 VM、自定义地图、战役、迷雾等属于远期；提交这类实现前先开 ADR 讨论。
