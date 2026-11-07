#!/usr/bin/env python3
"""PreToolUse 守卫：阻止 AI 执行任何与远端同步的 git / gh 命令。

从 stdin 读取 hook 的 JSON 输入，检出 Bash 命令中的远端同步模式时输出
deny 决策。所有输出为英文（日志/错误规则）。
"""

from __future__ import annotations

import json
import re
import sys

# 与 CODEBUDDY.md 的权限矩阵保持一致：任何远端交换都交给人工。
BLOCKED_PATTERNS = (
    r"\bgit\s+push\b",
    r"\bgit\s+fetch\b",
    r"\bgit\s+pull\b",
    r"\bgit\s+clone\b",
    r"\bgit\s+submodule\s+update\b",
    r"\bgit\s+remote\s+(add|set-url|remove|rename|prune)\b",
    r"\bgh\s+(pr|repo|release|issue)\b",
    r"\bglab\b",
)

DENY_REASON = (
    "Remote-syncing git commands are reserved for the human developer "
    "(CODEBUDDY.md section 4). Run this command manually."
)


def extract_command(payload: object) -> str:
    """从 hook 输入中取出 Bash 命令字符串。"""
    if not isinstance(payload, dict):
        return ""
    tool_input = payload.get("tool_input")
    if isinstance(tool_input, dict):
        return str(tool_input.get("command") or "")
    return str(payload.get("command") or "")


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (ValueError, OSError):
        return 0

    command = extract_command(payload)
    if not command:
        return 0

    for pattern in BLOCKED_PATTERNS:
        if re.search(pattern, command, re.IGNORECASE):
            print(
                json.dumps(
                    {
                        "hookSpecificOutput": {
                            "hookEventName": "PreToolUse",
                            "permissionDecision": "deny",
                            "permissionDecisionReason": DENY_REASON,
                        }
                    }
                )
            )
            return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
