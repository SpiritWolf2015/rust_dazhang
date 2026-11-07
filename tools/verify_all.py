#!/usr/bin/env python3
"""统一门禁：编码、格式、测试、OpenSpec 校验与资产扫描。

本地与 CI 共用同一入口：python tools/verify_all.py
所有输出为英文（日志/错误规则）。任一步失败返回非零退出码。
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGES_DIR = ROOT / "openspec" / "changes"

# 禁止提交的游戏资产及其派生文件后缀。
BANNED_SUFFIXES = {
    ".mpq", ".blp", ".mdx", ".mdl", ".w3x", ".w3m", ".w3e", ".w3i",
    ".w3r", ".w3u", ".wpm", ".doo", ".slk", ".tga", ".pcx", ".glb",
    ".bin", ".wav", ".mp3",
}
# 禁止提交的导出目录名（导出数据只允许留在本机）。
BANNED_DIRS = {"slk-exported", "map-parsed", "asset-converted", "assets-export"}


def run(label: str, command: list[str]) -> bool:
    """执行一条检查命令，打印结果并返回是否成功。"""
    print(f"[check] {label}")
    result = subprocess.run(command, cwd=ROOT)
    if result.returncode != 0:
        print(f"[fail] {label}: exit {result.returncode}", file=sys.stderr)
        return False
    return True


def active_change_names() -> list[str]:
    """列出 active（未归档）的 change 目录名。"""
    if not CHANGES_DIR.is_dir():
        return []
    return sorted(
        child.name
        for child in CHANGES_DIR.iterdir()
        if child.is_dir()
        and child.name != "archive"
        and (child / ".openspec.yaml").exists()
    )


def openspec_command(name: str) -> list[str]:
    """构造 openspec validate 命令。

    Windows 上 npm 生成的是 openspec.cmd（批处理），CreateProcess 无法直接
    执行，需经 cmd.exe；Linux/macOS 上的全局命令可直接调用。
    """
    if os.name == "nt":
        return ["cmd", "/c", "openspec", "validate", name]
    return ["openspec", "validate", name]


def asset_scan() -> bool:
    """检查 git 跟踪文件里是否混入了游戏资产或导出数据。"""
    result = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True
    )
    problems = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        path = Path(raw.decode("utf-8"))
        if path.suffix.lower() in BANNED_SUFFIXES:
            problems.append(f"{path}: banned asset suffix")
        if any(part in BANNED_DIRS for part in path.parts):
            problems.append(f"{path}: exported asset directory")
    if problems:
        print("[fail] asset scan:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return False
    print("[check] asset scan OK")
    return True


def main() -> int:
    ok = True
    ok &= run("encoding (UTF-8 no BOM, LF)", [sys.executable, "tools/check_encoding.py"])
    ok &= run("cargo fmt", ["cargo", "fmt", "--all", "--", "--check"])
    ok &= run("cargo test", ["cargo", "test", "--workspace"])

    for name in active_change_names():
        ok &= run(f"openspec validate {name}", openspec_command(name))

    ok &= asset_scan()

    if not ok:
        print("Verification FAILED", file=sys.stderr)
        return 1
    print("Verification OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
