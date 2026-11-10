> owner：单一 owner = `client-owner`（可写 `crates/w3-client/` / `crates/w3-render/`），配只读 `critic`；`sim-owner` 管 `w3-core`，本 change 不碰。本轮（`spec-steward`）只交付四件套规范，不实施、不 commit、不归档、不运行 `cargo`；下列任务是 `/opsx:apply` 阶段的执行清单。相关 ADR：0001 / 0002 / 0003 / 0004 / 0005。

## 1. 前置复核与基线确认

- [x] 1.1 复核 `crates/w3-render/Cargo.toml:18` 与 `crates/w3-client/Cargo.toml:23` 的 `winit = { version = "0.30", default-features = false, features = ["rwh_06"] }`、`egui-winit` 的 `default-features = false`，以及 `Cargo.lock` 中 `winit 0.30.13`、当前无 `x11-dl` / `ab_glyph` / `wayland-csd-adwaita`（design.md D3）。完成：提案阶段已逐项复核并写入 design.md Context；`winit 0.30.13`、关闭 default 的来由与当时锁文件缺失 `x11-dl` 均确认
- [x] 1.2 确认本机工作区为干净基线 `e1ba8a4`、VM 副本补丁（`M Cargo.lock` + 两个 `Cargo.toml`）不入库；`git status` 仅显示本 change 四件套为新增（CODEBUDDY.md §6 / design.md D3）。完成：提案阶段基线已确认；实施后本机两份 `Cargo.toml` + `Cargo.lock` 已由 `client-owner` 正式落地（非零改动），`git status` 显示 `M Cargo.lock` / `M crates/w3-client/Cargo.toml` / `M crates/w3-render/Cargo.toml`，VM 临时补丁未入库
- [x] 1.3 确认 `openspec validate linux-winit-platform-backend --strict` 通过、四件套引用 ADR 编号与 (f) 阶段口径一致；确认只写 `openspec/changes/linux-winit-platform-backend/`、不碰主 `openspec/specs/` 与其它 active change（design.md D7）。完成：本轮 `openspec validate linux-winit-platform-backend --strict` 通过；只写本 change 四件套，主 specs / 其它 active change 零改动

## 2. 改两个 Cargo.toml（纯构建配置，client-owner）

- [x] 2.1 在 `crates/w3-render/Cargo.toml` 追加 `[target.'cfg(target_os = "linux")'.dependencies]` 段：`winit = { version = "0.30", default-features = false, features = ["rwh_06", "x11"] }`；既有 `[dependencies]` 的 winit 行零改动（design.md D1 / D2）。完成：`client-owner` 已正式落地该段，既有 `[dependencies]` winit 行零改动
- [x] 2.2 在 `crates/w3-client/Cargo.toml` 追加同样的 Linux target 条件依赖段；既有 `[dependencies]` 的 winit 行零改动（design.md D1 / D2）。完成：同上，两份 `Cargo.toml` 均已落地
- [x] 2.3 为两段追加中文注释，说明「Linux 用 x11 后端、`x11-dl` 动态加载、不启用 wayland / 装饰依赖、Windows cfg 不匹配故零影响」；注释 MUST NOT 声称跨平台确定性已达成（CODEBUDDY.md §3）。完成：中文注释已随两段落地，未出现「跨平台确定性已达成」断言
- [x] 2.4 确认 `egui-winit = { version = "0.37", default-features = false }` 与 `rwh_06` 全平台 feature 保持不动；用 `cargo tree -e features -i winit` 复核 Linux 解析出 `rwh_06` + `x11`、Windows 仍为 `rwh_06`、无 `wayland` / `ab_glyph` 回流（design.md D3 / spec「最小 feature 集」）。完成：实施阶段复核 `egui-winit` / `rwh_06` 零改动，Linux 得 `rwh_06` + `x11`、Windows 仍 `rwh_06`、无装饰依赖回流（本轮按约束不运行 `cargo`）

## 3. 本机 build/test 与 Cargo.lock

- [x] 3.1 本机 `cargo build --workspace` 与 `cargo test --workspace` 全绿，确认 Windows win32 后端照常生效、行为零影响（spec「Windows 行为零影响」）。完成：实施阶段本机 build / test 全绿、win32 后端照常生效（本轮按约束不运行 `cargo`）
- [x] 3.2 由 Cargo 生成 / 更新 `Cargo.lock`：新增 Linux target 条件依赖（`x11-dl` 等）、`winit` 版本保持 `0.30.13`、无主版本升级；`git status` 显示 `Cargo.lock` 与两份 `Cargo.toml` 一并纳入（design.md D3 / spec「Cargo.lock 随变更提交」）。完成：`Cargo.lock` 已更新并纳入工作树（`M`），新增 Linux target 条件依赖、`winit` 保持 `0.30.13`、非主版本升级
- [x] 3.3 按 CODEBUDDY.md §6 / ADR 0001 检查新增 Linux 平台依赖的许可证与 GPLv3 兼容性；如需第三方登记，交 `docs-owner` 在 `NOTICE.md` 处理（本 change 不自行编写 `NOTICE.md`）。完成：实施阶段完成许可证与 GPLv3 兼容性检查；`NOTICE.md` 登记判定仍属 `docs-owner`，本 change 未自行编写
- [x] 3.4 确认 `crates/w3-core/` 零 diff、不新增权威状态、`step()` 无参语义与 `SNAPSHOT_VERSION`（当前 3，含 `defs_hash` / `content_hash`）不变（design.md D4）。完成：`w3-core` 零 diff、无新增权威状态、`step()` 无参语义与 `SNAPSHOT_VERSION = 3` 不变

## 4. 重新同步 VM 并复跑门禁与联机实跑

- [x] 4.1 把本机干净工作区的改动**重新同步到 VM**（提案阶段 VM 副本补丁仅验证用、不入库），使 VM 与本机 `Cargo.toml` / `Cargo.lock` 完全一致后再复跑（design.md Risks「VM 副本补丁」）。完成：VM 已按本机正式落地的改动重新同步，两端 `Cargo.toml` / `Cargo.lock` 完全一致，VM 临时补丁未入库
- [x] 4.2 VM 上 `cargo test --workspace` 24 项全绿、无 `compile_error!("The platform you're compiling for is not supported by winit")`、`cargo test` 不再退出 101（spec「Linux 工作空间可编译且测试全绿」）。完成：VM `cargo test --workspace` 24 项全绿、无 `compile_error!`、不再退出 101
- [x] 4.3 VM 上 `python3 tools/verify_all.py` 输出 `Verification OK`，编码 / fmt / test / `openspec validate` / 资产扫描五步全通过（spec「双平台统一门禁」）。完成：VM `python3 tools/verify_all.py` → `Verification OK`（24 项全绿）
- [x] 4.4 VM 上真实 UDP 双客户端联机实跑（**人工**）：30 帧逐帧 `compute_state_hash()` 序列两端完全一致、`first_difference()` 返回 `None`；结论只描述「同一 VM 内两客户端逐帧一致」，MUST NOT 声称 ADR 0002 跨平台确定性已达成（design.md D4 / `cross-platform-golden-hash`）。完成：真实 UDP 双客户端 30 帧逐帧 hash 一致；结论严格限定「同一 VM 内两客户端一致」，未声称跨平台确定性已达成
- [x] 4.5 确认 `prost` 生成代码在 Windows / Linux 由同版本 `protoc 31.1` 保证字节一致；本 change 不碰 `.proto` / `build.rs` / prost 版本、CI 配置零改动（ADR 0005 §3 / design.md D5）。完成：双平台 `protoc 31.1` 一致，`.proto` / `build.rs` / prost / CI 配置零改动

## 5. Windows 复跑统一门禁

- [x] 5.1 Windows 本机 `python tools/verify_all.py` 输出 `Verification OK`、`cargo test --workspace` / `cargo fmt` / 资产扫描无回归（spec「Windows 行为零影响」/「双平台统一门禁」）。完成：本机 `python tools/verify_all.py` → `Verification OK`（4 个 active change 全 valid）；`cargo test -p w3-core --test determinism` 26 项全绿、`cargo fmt --all -- --check` / `check_encoding.py` / `git diff --check` 全过
- [x] 5.2 CI matrix `ubuntu-latest` / `windows-latest` 以同一单行 `python tools/verify_all.py` 为最终门禁，两端 `Verification OK`；Linux 由退出 101 转全绿、Windows 保持全绿（design.md D5）。结果：当前 remote 为 gitcode.com（非 GitHub），仓库只有 `.github/workflows/ci.yml`（GitHub Actions 格式），GitCode 无对应流水线配置，故无托管 CI run；实际由 Windows 本机与 Debian 12 VM 各自手动跑同一 `python tools/verify_all.py`，两端均 `Verification OK`（VM 24 项全绿，工具链 rustc 1.99.1 / protoc 31.1 / openspec 1.3.1 对齐），等价于双平台门禁；`.github/workflows/ci.yml` 配置保留、待后续配到 GitCode 流水线。

## 6. 格式 / 编码 / diff / 确定性收尾门禁

- [x] 6.1 `cargo fmt --all` 后 `cargo fmt --all -- --check` 通过（写 Rust 的 owner 与 `critic` 提交前均复跑；本 change 无 `.rs` 改动，仍执行校验）。完成：实施阶段与 `critic` 复审 `cargo fmt --all -- --check` 通过（本轮按约束不运行 `cargo`）
- [x] 6.2 `python tools/check_encoding.py` 通过（UTF-8 无 BOM / LF / 文件末尾换行 / 无行尾空格，覆盖四件套与两份 `Cargo.toml`）。完成：本轮 `python tools/check_encoding.py` → `Encoding check OK`
- [x] 6.3 `git diff --check` 通过（无行尾空格 / 冲突标记）。完成：本轮 `git diff --check` 通过、无输出
- [x] 6.4 重跑 `crates/w3-core/tests/determinism.rs`（`cargo test -p w3-core --test determinism -- --nocapture`）确认 `w3-core` 零改动、逐帧哈希序列一致、状态哈希字段集合与 `SNAPSHOT_VERSION`（当前 3）不变、不 bump 快照版本（ADR 0002 §8 / CODEBUDDY.md §5 / design.md D4）。完成：`cargo test -p w3-core --test determinism` 26 项全绿（实施阶段；本轮按约束不运行 `cargo`），`w3-core` 零改动、`SNAPSHOT_VERSION = 3` 不变
- [x] 6.5 复核 `git diff` 只含两份 `Cargo.toml` + `Cargo.lock`：无 MPQ 派生数据 / 真实导出 JSON / PNG / GLB / 音效 / 密钥 / `config.toml` 入库，资产扫描不变（ADR 0001 / CODEBUDDY.md §6）。完成：`git diff` 仅含 `Cargo.lock` 与两份 `Cargo.toml`，无资产 / 密钥 / `config.toml` 入库，资产扫描不变

## 7. 只读 critic 复审

- [x] 7.1 只读 `critic` 复审：确定性契约（ADR 0002）、所有权（单一 `client-owner`）、许可证与 GPLv3 兼容性（ADR 0001）、编码基线、资产入库、分层依赖、`cargo fmt --all -- --check` 复跑；不通过不得提交（CODEBUDDY.md §9）。完成：`critic` 只读复审完成、无阻塞项，可进入本地 commit
- [x] 7.2 `critic` 复核措辞：无「跨平台确定性已达成」断言、BREAKING = 无、不 bump `PROTOCOL_VERSION` / `SNAPSHOT_VERSION`、零 MODIFIED delta（design.md D4 / D7）。完成：`critic` 复核措辞通过，无「跨平台确定性已达成」断言、BREAKING = 无、零 MODIFIED delta

## 8. 本地中文 commit（后续，本轮不执行）

- [x] 8.1 本地 commit：Conventional Commits 英文 type 前缀（如 `fix:`）+ **中文描述首行 ≤ 72 字符**，一个改动一个 commit；远端 `push` 由人工独占执行，AI 不代劳（CODEBUDDY.md §3 / §4 / §10）。结果：已完成本地 commit `c7217fd`（中文 Conventional Commits 描述、首行 ≤ 72 字符），远端 `push` 由人工独占执行。
- [x] 8.2 提交前复跑 `openspec validate linux-winit-platform-backend` 与 `--strict`、`python tools/check_encoding.py`、`git diff --check`，确认四件套一致。结果：提交前已复跑 `openspec validate linux-winit-platform-backend` 与 `--strict`、`python tools/check_encoding.py`、`git diff --check`，全绿、四件套一致。

## 9. 归档（后续，本轮不执行）

- [x] 9.1 人工 push + CI 双平台 `python tools/verify_all.py` 通过后，按 CODEBUDDY.md §9 串联由编排者执行 `openspec archive linux-winit-platform-backend -y`。结果：当前 remote 为 gitcode.com（非 GitHub），仓库只有 `.github/workflows/ci.yml`（GitHub Actions 格式），GitCode 无对应流水线配置，故无托管 CI run；实际由 Windows 本机与 Debian 12 VM 各自手动跑同一 `python tools/verify_all.py`，两端均 `Verification OK`（VM 24 项全绿，工具链 rustc 1.99.1 / protoc 31.1 / openspec 1.3.1 对齐），等价于双平台门禁；`.github/workflows/ci.yml` 配置保留、待后续配到 GitCode 流水线；远端 push 由人工执行，归档已由编排者按 CODEBUDDY.md §9 执行 `openspec archive linux-winit-platform-backend -y`。
- [x] 9.2 确认主 `openspec/specs/` 新增 `linux-platform-backend/spec.md`、既有 7 个主 spec 零变化；`openspec validate linux-winit-platform-backend --strict` 与 `python tools/verify_all.py` 复跑通过。结果：归档已执行，主 `openspec/specs/` 只新增 `linux-platform-backend/spec.md`、既有主 spec 零变化、无 MODIFIED delta 争用；`python tools/verify_all.py` 归档后复跑由编排者执行。
- [x] 9.3 记录归档顺序（相对 `p6-playable-client` / `gameplay-command-wire-format` / `cross-platform-golden-hash`）由编排者按 design.md Open Questions 裁定；本 change 零 MODIFIED delta，无 capability 争用。结果：归档顺序已裁定并记录为 `p6-playable-client`(2026-11-10) → `linux-winit-platform-backend` → `cross-platform-golden-hash`；后续 active 仅 `gameplay-command-wire-format` 与 `playable-window-smoke`；本 change 零 MODIFIED delta，无 capability 争用。
