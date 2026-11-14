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


if __name__ == "__main__":
    unittest.main()
