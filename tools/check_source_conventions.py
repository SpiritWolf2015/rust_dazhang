#!/usr/bin/env python3
"""检查源码约定：中文注释、英文日志、禁散落 println!、工具脚本跨平台。

用法：python tools/check_source_conventions.py
输出为英文（日志 / 错误规则）；注释中文。只用 Python 标准库，跨平台。
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def has_cjk(text: str) -> bool:
    """判断文本是否含 CJK 统一表意文字。"""
    return any("\u4e00" <= ch <= "\u9fff" for ch in text)


# 日志 / 错误 / panic 等宏与 thiserror 属性中的字符串字面量。
LOG_MACROS = re.compile(
    r"(?:\b(?:log::)?(?:info|warn|error|debug|trace|panic|anyhow)!\s*\(\s*"
    r"|\bexpect\s*\(\s*"
    r"|#\[error\s*\(\s*)"
    r"\"((?:[^\"\\]|\\.)*)\""
)
PRINT_MACROS = re.compile(r"\b(?:println|eprintln)!\s*\(")
BANNED_SCRIPT_SUFFIXES = {".ps1", ".sh", ".bat", ".cmd"}

# 无 CJK 的注释正文命中以下任一形态则豁免：公式、命令、路径、纯符号、单标识符。
FORMULA_CHARS = "×^=→±∑√≤≥÷∞≈"
CJK_PUNCT = "，。；：、「」『』（）【】《》"
COMMAND_PREFIXES = (
    "cargo ", "python ", "python3 ", "node ", "npm ", "git ", "rustc ",
    "openspec ", "make ", "bash ", "sh ", "$ ", "#[",
)


def source_files() -> list[Path]:
    """返回 git 跟踪与未跟踪（排除 gitignore）的 Rust 源码文件。"""
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
        capture_output=True,
        check=True,
    )
    return [
        ROOT / raw.decode("utf-8")
        for raw in result.stdout.split(b"\0")
        if raw and raw.decode("utf-8").endswith(".rs")
    ]


def comment_is_exempt(body: str) -> bool:
    """判断不含 CJK 的注释正文是否属于可豁免形态。"""
    body = body.strip()
    if not body or has_cjk(body):
        return True
    if any(ch in body for ch in CJK_PUNCT):
        return True
    if any(ch in body for ch in FORMULA_CHARS):
        return True
    # 枚举 / 映射表注释：形如 `0 -> EffectKind::Damage`，不是散文句子。
    if re.match(r"^\s*\d+\s*->\s*[A-Za-z_][A-Za-z0-9_:]*\s*$", body):
        return True
    if body.startswith(COMMAND_PREFIXES):
        return True
    # 单标识符 / 路径 / 版本号 / 纯标记：无空格或仅路径分隔符。
    if re.fullmatch(r"[A-Za-z0-9_\-./:#\[\]()]+", body):
        return True
    # 去掉反引号代码与 URL 后，若只剩符号与数字则视为公式 / 标记。
    stripped = re.sub(r"`[^`]*`", "", body)
    stripped = re.sub(r"https?://\S+", "", stripped)
    if not re.sub(r"[^A-Za-z]", "", stripped):
        return True
    return False


def main() -> int:
    problems: list[str] = []
    checked = 0

    for path in source_files():
        if not path.is_file():
            continue
        checked += 1
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")
        is_lib_source = "/src/" in f"/{rel}" and not rel.endswith("build.rs")

        for lineno, line in enumerate(text.splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//"):
                # 去掉 //、///、//! 前缀，只留注释正文。
                body = stripped.lstrip("/")
                if body.startswith("!"):
                    body = body[1:]
                if not comment_is_exempt(body):
                    problems.append(
                        f"{rel}:{lineno}: non-Chinese comment: {body[:70]}"
                    )
            for match in LOG_MACROS.finditer(line):
                if has_cjk(match.group(1)):
                    problems.append(
                        f"{rel}:{lineno}: non-English log/error text: {match.group(1)[:70]}"
                    )
            if is_lib_source and PRINT_MACROS.search(line):
                problems.append(
                    f"{rel}:{lineno}: println!/eprintln! in library source"
                )

    tools_dir = ROOT / "tools"
    if tools_dir.is_dir():
        for script in tools_dir.iterdir():
            if script.is_file() and script.suffix.lower() in BANNED_SCRIPT_SUFFIXES:
                problems.append(
                    f"tools/{script.name}: platform-specific script not allowed"
                )

    if problems:
        print("Source conventions check FAILED:", file=sys.stderr)
        for problem in problems[:50]:
            print(f"  {problem}", file=sys.stderr)
        if len(problems) > 50:
            print(f"  ... and {len(problems) - 50} more", file=sys.stderr)
        return 1

    print(f"Source conventions OK: {checked} Rust files checked")
    return 0


if __name__ == "__main__":
    sys.exit(main())
