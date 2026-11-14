#!/usr/bin/env python3
"""检查仓库文本文件的编码基线：UTF-8 无 BOM、LF 换行。

用法：python tools/check_encoding.py
检查 git 跟踪与未跟踪（排除 gitignore）的文本文件；二进制文件按扩展名跳过。
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

# 需要检查编码的文本后缀。
TEXT_SUFFIXES = {
    ".rs", ".toml", ".json", ".yaml", ".yml", ".md", ".py", ".sh",
    ".ps1", ".xml", ".txt", ".cfg", ".example", ".wgsl", ".proto", ".js", ".mjs",
    ".ts", ".gitignore", ".gitattributes", ".editorconfig",
}

# 无后缀但仍是文本的文件名。
TEXT_NAMES = {
    ".gitignore", ".gitattributes", ".editorconfig", "LICENSE", "NOTICE.md",
    "CODEBUDDY.md", "CONTRIBUTING.md", "README.md", "Makefile",
}

# BOM 前缀 -> 人类可读名称（英文，作为工具输出）。
BOMS = (
    (b"\xef\xbb\xbf", "UTF-8 BOM"),
    (b"\xff\xfe", "UTF-16 LE BOM"),
    (b"\xfe\xff", "UTF-16 BE BOM"),
    (b"\xff\xfe\x00\x00", "UTF-32 LE BOM"),
    (b"\x00\x00\xfe\xff", "UTF-32 BE BOM"),
)


def tracked_files() -> list[Path]:
    """返回 git 跟踪与未跟踪（排除 gitignore）的文本文件列表。

    加入未跟踪文件，避免新增但尚未 `git add` 的源码 / schema 绕过编码门禁。
    """
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        capture_output=True,
        check=True,
    )
    return [
        Path(raw.decode("utf-8"))
        for raw in result.stdout.split(b"\0")
        if raw
    ]


def is_text_file(path: Path) -> bool:
    """按后缀或文件名判断是否为需要检查的文本文件。"""
    return path.suffix.lower() in TEXT_SUFFIXES or path.name in TEXT_NAMES


def main() -> int:
    problems: list[str] = []
    checked = 0

    for path in tracked_files():
        if not is_text_file(path) or not path.is_file():
            continue
        checked += 1
        raw = path.read_bytes()

        for prefix, label in BOMS:
            if raw.startswith(prefix):
                problems.append(f"{path}: {label} detected")
                break

        if b"\r\n" in raw:
            problems.append(f"{path}: CRLF line ending detected")

        if raw and not raw.endswith(b"\n"):
            problems.append(f"{path}: missing final newline")

        for lineno, line in enumerate(raw.split(b"\n"), 1):
            if line.rstrip(b" \t") != line:
                problems.append(f"{path}:{lineno}: trailing whitespace")

        try:
            raw.decode("utf-8")
        except UnicodeDecodeError as error:
            problems.append(f"{path}: invalid UTF-8 ({error})")

    if problems:
        print("Encoding check FAILED:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(f"Encoding check OK: {checked} text files checked")
    return 0


if __name__ == "__main__":
    sys.exit(main())
