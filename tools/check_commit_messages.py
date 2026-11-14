#!/usr/bin/env python3
"""检查 git 提交信息：Conventional Commits 英文 type 前缀 + 中文描述 + 首行 <= 72 字符。

用法：
  python tools/check_commit_messages.py                  # 自动推断范围
  python tools/check_commit_messages.py --base origin/main
  python tools/check_commit_messages.py --base <sha>    # CI 传入 push / PR 的 base
  python tools/check_commit_messages.py --count 5       # 检查最近 5 个
输出为英文（日志 / 错误规则）；注释中文。只用 Python 标准库，跨平台。
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

TYPE_RE = re.compile(
    r"^(feat|fix|docs|chore|refactor|test|perf|build|ci|style|revert)"
    r"(\([^)]+\))?!?: (.+)$"
)
MAX_SUBJECT_LEN = 72


def has_cjk(text: str) -> bool:
    """判断文本是否含 CJK 统一表意文字。"""
    return any("\u4e00" <= ch <= "\u9fff" for ch in text)


def ref_exists(ref: str) -> bool:
    """判断 git ref 是否存在；全 0 sha（首次 push 的 before）视为无效。"""
    if not ref or set(ref) == {"0"}:
        return False
    return subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", ref],
        cwd=ROOT,
        capture_output=True,
    ).returncode == 0


def log_subjects(cmd: list[str]) -> list[tuple[str, str]]:
    """执行一次 git log，返回 (sha, subject) 列表。"""
    result = subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, check=True
    )
    rows = []
    for line in result.stdout.splitlines():
        if "\t" in line:
            sha, _, subject = line.partition("\t")
            rows.append((sha, subject))
    return rows


def subjects(base: str | None, count: int) -> list[tuple[str, str]]:
    """返回待检查提交；指定范围为空时回退到最近 1 个提交。"""
    if base and ref_exists(base):
        rows = log_subjects(
            ["git", "log", f"{base}..HEAD", "--format=%H\t%s"]
        )
        if rows:
            return rows
    return log_subjects(["git", "log", f"-{max(count, 1)}", "--format=%H\t%s"])


def main() -> int:
    parser = argparse.ArgumentParser(description="Check commit message conventions")
    parser.add_argument("--base", default=None, help="base ref for range check")
    parser.add_argument("--count", type=int, default=1, help="fallback commit count")
    args = parser.parse_args()

    base = args.base
    if base is None and ref_exists("origin/main"):
        base = "origin/main"

    rows = subjects(base, args.count)
    problems: list[str] = []
    for sha, subject in rows:
        short = sha[:8]
        if len(subject) > MAX_SUBJECT_LEN:
            problems.append(
                f"{short}: subject too long ({len(subject)} > {MAX_SUBJECT_LEN}): "
                f"{subject[:70]}"
            )
        match = TYPE_RE.match(subject)
        if not match:
            problems.append(
                f"{short}: missing Conventional Commits prefix: {subject[:70]}"
            )
        elif not has_cjk(match.group(3)):
            problems.append(
                f"{short}: non-Chinese description: {subject[:70]}"
            )

    if problems:
        print("Commit message check FAILED:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(f"Commit messages OK: {len(rows)} checked")
    return 0


if __name__ == "__main__":
    sys.exit(main())
