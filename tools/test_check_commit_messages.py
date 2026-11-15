#!/usr/bin/env python3
"""check_commit_messages.py 的跨平台编码回归测试。

背景：CI 的 windows-latest 默认 locale 为 cp1252，而 git 输出的中文提交
信息是 UTF-8 字节；若 subprocess 依赖 `text=True` 的隐式 locale 解码，会
抛 UnicodeDecodeError 或得到乱码，导致 commit message 门禁失败。本测试守住
「显式 UTF-8 解码」契约，并给出反向验证（缺陷态必然失败）。

本地入口：python -m unittest discover -s tools -p "test_*.py"
输出为英文（日志 / 错误规则），注释与 docstring 中文。
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_commit_messages as ccm

# 含中文的提交主题（与仓库真实提交一致，用 UTF-8 字节构造 git 输出）。
CN_SUBJECT = "feat: 实现 p8-faction-foundation 地基"


class LogSubjectsEncodingTests(unittest.TestCase):
    """log_subjects 必须显式锁定 UTF-8，不依赖宿主 locale。"""

    def test_forces_utf8_and_parses_cjk_subject(self):
        """修复态：subprocess 调用带 encoding=utf-8，且能解析中文主题。"""
        fake_result = SimpleNamespace(stdout="abc123\t" + CN_SUBJECT + "\n")
        with patch.object(ccm.subprocess, "run", return_value=fake_result) as run:
            rows = ccm.log_subjects(["git", "log", "-1"])
        # 契约断言：显式编码。缺陷态（去掉 encoding）在此失败 —— 改坏验红。
        self.assertEqual(run.call_args.kwargs.get("encoding"), "utf-8")
        self.assertEqual(rows, [("abc123", CN_SUBJECT)])
        self.assertTrue(ccm.has_cjk(rows[0][1]))

    def test_locale_decode_loses_chinese_is_regression_evidence(self):
        """反向验证元断言：依赖宿主 cp1252 解码 UTF-8 中文必然失真 / 抛错，
        证明「不锁定 encoding」在 Windows 缺陷态会失败。"""
        raw = ("abc123\t" + CN_SUBJECT + "\n").encode("utf-8")
        # 修复态：UTF-8 完整还原中文。
        self.assertIn(CN_SUBJECT, raw.decode("utf-8"))
        # 缺陷态：按 Windows CI 的 cp1252 解码，不可能还原中文（乱码或抛错）。
        try:
            decoded = raw.decode("cp1252")
        except UnicodeDecodeError:
            decoded = ""  # 直接解码失败本身即缺陷态失败的证据
        self.assertNotIn(CN_SUBJECT, decoded)


if __name__ == "__main__":
    unittest.main()
