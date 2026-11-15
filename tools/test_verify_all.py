#!/usr/bin/env python3
"""`verify_all.py` 的只读子命令与四件套指纹口径单元测试。

本地入口：python -m unittest discover -s tools -p "test_*.py"
或：python tools/test_verify_all.py
所有输出为英文（日志/错误规则），注释与 docstring 中文。
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_all as v

BASE_FILES = {
    "proposal.md": "# Proposal\n",
    "design.md": "# Design\n",
    "specs/x/spec.md": "#### Scenario: a\n- **WHEN** x **THEN** y\n",
    "tasks.md": "# Tasks\n\n- [ ] 1.1 do a\n",
}


def make_change(root: Path, files: dict[str, str]) -> Path:
    """在临时目录构造一个 active change，返回其目录。"""
    change = root / "c1"
    change.mkdir(parents=True)
    (change / ".openspec.yaml").write_text("schema: spec-driven\n", encoding="utf-8")
    for rel, content in files.items():
        path = change / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    return change


def broken_fingerprint(change_dir: Path) -> str:
    """退化实现：故意漏读 `design.md`，仅用于反向验证对照。"""
    digest = hashlib.sha256()
    for name in ("proposal.md", "tasks.md"):
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update((change_dir / name).read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


class FingerprintTests(unittest.TestCase):
    def test_design_byte_change_is_detected(self):
        """2.1：四件套仅 `design.md` 差一字节，指纹 MUST 不同。"""
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            a = make_change(root / "a", BASE_FILES)
            b = make_change(root / "b", {**BASE_FILES, "design.md": "# Design\n\nextra\n"})
            self.assertNotEqual(v.spec_artifact_fingerprint(a), v.spec_artifact_fingerprint(b))

    def test_reverse_validation_broken_impl_is_caught(self):
        """2.3 反向验证：退化实现 MUST 把「本应不同」算成相同，正式实现 MUST 通过。"""
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            a = make_change(root / "a", BASE_FILES)
            b = make_change(root / "b", {**BASE_FILES, "design.md": "# Design\n\nextra\n"})
            # 退化版被抓出：证明 2.1 的断言不是恒真。
            self.assertEqual(broken_fingerprint(a), broken_fingerprint(b))
            self.assertNotEqual(v.spec_artifact_fingerprint(a), v.spec_artifact_fingerprint(b))

    def test_tasks_checkbox_flip_does_not_change_fingerprint(self):
        """2.2：`tasks.md` 仅复选框翻转，归一化后指纹 MUST 相同。"""
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            a = make_change(root / "a", BASE_FILES)
            b = make_change(root / "b", {
                **BASE_FILES,
                "tasks.md": BASE_FILES["tasks.md"].replace("- [ ] 1.1", "- [x] 1.1"),
            })
            self.assertEqual(v.spec_artifact_fingerprint(a), v.spec_artifact_fingerprint(b))

    def test_cli_stdout_equals_function(self):
        """2.5：CLI stdout 与 `spec_artifact_fingerprint()` 返回值逐字节相等、退出 0。"""
        with tempfile.TemporaryDirectory() as td:
            changes = Path(td) / "changes"
            change = changes / "c1"
            change.mkdir(parents=True)
            (change / ".openspec.yaml").write_text("schema: spec-driven\n", encoding="utf-8")
            for rel, content in BASE_FILES.items():
                path = change / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            old = v.CHANGES_DIR
            v.CHANGES_DIR = changes
            out = io.StringIO()
            try:
                with contextlib.redirect_stdout(out):
                    code = v.fingerprint_cli("c1")
            finally:
                v.CHANGES_DIR = old
            self.assertEqual(code, 0)
            self.assertEqual(out.getvalue(), v.spec_artifact_fingerprint(change) + "\n")

    def test_cli_negative_cases(self):
        """2.4：未知 change / 缺 `design.md` / 无 `.openspec.yaml` -> 非零退出 + `problem=`。"""
        with tempfile.TemporaryDirectory() as td:
            changes = Path(td) / "changes"
            changes.mkdir()
            good = changes / "c1"
            good.mkdir()
            (good / ".openspec.yaml").write_text("schema: spec-driven\n", encoding="utf-8")
            for rel in ("proposal.md", "design.md", "tasks.md"):
                (good / rel).write_text("# x\n", encoding="utf-8")
            bad = changes / "c2"
            bad.mkdir()
            (bad / ".openspec.yaml").write_text("schema: spec-driven\n", encoding="utf-8")
            (bad / "proposal.md").write_text("# x\n", encoding="utf-8")
            (bad / "tasks.md").write_text("# x\n", encoding="utf-8")
            orphan = changes / "c3"
            orphan.mkdir()  # 无 .openspec.yaml，不在 active 列表
            old = v.CHANGES_DIR
            v.CHANGES_DIR = changes
            out, err = io.StringIO(), io.StringIO()
            try:
                with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                    codes = [
                        v.fingerprint_cli("no-such-change"),
                        v.fingerprint_cli("c2"),
                        v.fingerprint_cli("c3"),
                    ]
            finally:
                v.CHANGES_DIR = old
            self.assertEqual(codes, [1, 1, 1])
            self.assertIn("problem=unknown change", err.getvalue())
            self.assertIn("problem=missing artifacts", err.getvalue())
            self.assertEqual(out.getvalue(), "")

    def test_cli_specs_dir_absent_still_outputs_hex(self):
        """`specs/` 为空沿用空字节口径，退出 0 且输出 64 位 hex。"""
        with tempfile.TemporaryDirectory() as td:
            changes = Path(td) / "changes"
            change = changes / "c1"
            change.mkdir(parents=True)
            (change / ".openspec.yaml").write_text("schema: spec-driven\n", encoding="utf-8")
            for rel in ("proposal.md", "design.md", "tasks.md"):
                (change / rel).write_text("# x\n", encoding="utf-8")
            old = v.CHANGES_DIR
            v.CHANGES_DIR = changes
            out = io.StringIO()
            try:
                with contextlib.redirect_stdout(out):
                    code = v.fingerprint_cli("c1")
            finally:
                v.CHANGES_DIR = old
            self.assertEqual(code, 0)
            self.assertRegex(out.getvalue().strip(), r"^[0-9a-f]{64}$")

    def test_real_entrypoint_unknown_change(self):
        """真实 `__main__` 分发：未知 change 非零退出且 `problem=unknown change`。"""
        script = Path(__file__).resolve().parent / "verify_all.py"
        result = subprocess.run(
            [sys.executable, str(script), "--fingerprint", "no-such-change"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("problem=unknown change", result.stderr)
        self.assertEqual(result.stdout, "")

    def test_real_entrypoint_missing_argument(self):
        """真实 `__main__` 分发：缺 change 名 -> `problem=bad arguments`。"""
        script = Path(__file__).resolve().parent / "verify_all.py"
        result = subprocess.run(
            [sys.executable, str(script), "--fingerprint"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("problem=bad arguments", result.stderr)


class ValidateAllFallbackTests(unittest.TestCase):
    """全量门禁的 `openspec validate --all` 主 specs 兜底（R3）。"""

    def test_command_is_all_non_strict_and_cross_platform(self):
        """命令 MUST 含 `--all`、MUST NOT 含 `--strict`；两种 os.name 分支都要覆盖主 specs。"""
        cmd = v.openspec_validate_all_command()
        self.assertEqual(cmd[-1], "--all")
        self.assertNotIn("--strict", cmd)
        # 与单 change 校验共用同一平台前缀（Windows 经 cmd.exe，其余直调）。
        single = v.openspec_command("c1")
        self.assertEqual(cmd[:-1], single[:-1])
        with mock.patch.object(v.os, "name", "nt"):
            self.assertEqual(
                v.openspec_validate_all_command(),
                ["cmd", "/c", "openspec", "validate", "--all"],
            )
        with mock.patch.object(v.os, "name", "posix"):
            self.assertEqual(
                v.openspec_validate_all_command(),
                ["openspec", "validate", "--all"],
            )

    @staticmethod
    def _main_commands(fallback_override=None):
        """跑一遍 `main()`（外部子进程调用全部 mock），返回 `(退出码, 命令列表)`。

        `fallback_override` 非 None 时把兜底命令替换为该值，用于构造缺陷态：
        等价于让全量校验命令不再出现在 `main()` 的命令序列中（仍保留一次
        `run` 调用），其余检查一字不动。
        """
        with contextlib.ExitStack() as stack:
            run = stack.enter_context(mock.patch.object(v, "run", return_value=True))
            stack.enter_context(mock.patch.object(v, "active_change_names", return_value=[]))
            stack.enter_context(mock.patch.object(v, "asset_scan", return_value=True))
            stack.enter_context(mock.patch.object(v, "decision_memo_check", return_value=True))
            stack.enter_context(mock.patch.object(v, "spec_gate_check", return_value=True))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            if fallback_override is not None:
                stack.enter_context(
                    mock.patch.object(
                        v, "openspec_validate_all_command", return_value=fallback_override
                    )
                )
            code = v.main()
        return code, [call.args[1] for call in run.call_args_list]

    def test_main_runs_validate_all_even_with_no_active_changes(self):
        """正式态：归档后 active change 为空，`main()` 仍 MUST 跑一条全量校验。

        精确匹配 `openspec_validate_all_command()` 的返回值，避免 `cargo fmt
        --all` 之类同样含 `--all` 字样的命令造成假阳性（B1 修复）。
        """
        code, commands = self._main_commands()
        self.assertEqual(code, 0)
        self.assertIn(
            v.openspec_validate_all_command(),
            commands,
            "reason=main_specs_not_validated_when_no_active_changes",
        )

    def test_reverse_validation_missing_fallback_is_caught(self):
        """反向验证（A-B，两遍都真实跑 `main()`）：摘除兜底后，同一条精确断言
        在缺陷态必失败、正式态通过，证明上一条测试不是恒真、缺陷态能被抓出。"""
        fallback = v.openspec_validate_all_command()
        # 正式态：含全量兜底。
        _, formal = self._main_commands()
        self.assertIn(fallback, formal)
        # 缺陷态：把兜底命令退化为空（等价于从调用链摘除），其余检查不变。
        _, broken = self._main_commands(fallback_override=[])
        self.assertNotIn(fallback, broken)
        # 元断言：正式态那条断言在缺陷态必然抛 AssertionError。
        with self.assertRaises(AssertionError):
            self.assertIn(fallback, broken)


if __name__ == "__main__":
    unittest.main()
