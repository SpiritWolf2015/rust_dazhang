#!/usr/bin/env python3
"""联机玩法命令双进程冒烟验收脚本（tasks 8.6 的人工辅助入口）。

以真实 UDP 启动 1 个 relay 与 2 个 lockstep_client 客户端，各跑 30 帧，
逐帧比对两端权威状态哈希，并断言施法 / 生产 / 取消生产三类玩法命令
确实上行并进入权威帧批。

本脚本**不纳入** verify_all.py 与 CI：8.6 是人工验收项，CI 只覆盖编译
与 CPU 测试。请在具备回环 UDP 的桌面手动执行：

    python tools/gameplay_online_smoke.py

日志写入临时目录，脚本结束自动清理，产物不入库。运行输出一律英文。
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Windows 下 cargo 产出的可执行文件带 .exe，其余平台不带。
EXE_SUFFIX = ".exe" if os.name == "nt" else ""
RELAY_BIN = ROOT / "target" / "debug" / f"w3-relay{EXE_SUFFIX}"
CLIENT_BIN = ROOT / "target" / "debug" / "examples" / f"lockstep_client{EXE_SUFFIX}"

FRAME_RE = re.compile(r"frame applied: frame=(\d+) hash=(0x[0-9a-fA-F]+)")
DONE_RE = re.compile(
    r"done: frames=(\d+) last_hash=(0x[0-9a-fA-F]+) gameplay_applied=(\d+)"
)
UPLINK_RE = re.compile(r"gameplay uplink: frame=(\d+) kind=(\w+)")
EXPECTED_KINDS = {"cast", "train", "cancel_train"}


def build_binaries() -> bool:
    """预编译 relay 与客户端，避免并发编译争锁，也避免日志混入 cargo 输出。"""
    commands = [
        ["cargo", "build", "-p", "w3-relay"],
        ["cargo", "build", "-p", "w3-net", "--example", "lockstep_client"],
    ]
    for command in commands:
        print(f"[build] {' '.join(command)}")
        if subprocess.run(command, cwd=ROOT).returncode != 0:
            print(f"[fail] build failed: {' '.join(command)}", file=sys.stderr)
            return False
    return True


def wait_for_line(path: Path, needle: str, timeout: float, proc: subprocess.Popen) -> bool:
    """轮询日志直到出现目标行；进程提前退出或超时则返回 False。"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        if needle in path.read_text(encoding="utf-8", errors="replace"):
            return True
        time.sleep(0.1)
    return False


def parse_log(path: Path):
    """解析单端日志：逐帧哈希序列、收尾行、上行玩法命令种类。"""
    text = path.read_text(encoding="utf-8", errors="replace")
    frames = [(int(frame), value.lower()) for frame, value in FRAME_RE.findall(text)]
    done = DONE_RE.search(text)
    kinds = {kind for _, kind in UPLINK_RE.findall(text)}
    return frames, done, kinds


def verify(client1_log: Path, client2_log: Path, expected_frames: int) -> bool:
    """逐项比对两端结果，结论以英文输出。"""
    frames1, done1, kinds1 = parse_log(client1_log)
    frames2, done2, kinds2 = parse_log(client2_log)
    ok = True

    def check(label: str, condition: bool, detail: str = "") -> None:
        nonlocal ok
        suffix = f" {detail}" if detail else ""
        print(f"[check] {label}: {'OK' if condition else 'FAIL'}{suffix}")
        if not condition:
            ok = False

    check("client1 frame log present", bool(frames1), f"count={len(frames1)}")
    check("client2 frame log present", bool(frames2), f"count={len(frames2)}")
    check("both clients reached done line", done1 is not None and done2 is not None)
    if done1 is None or done2 is None:
        return False

    n1, h1, g1 = int(done1.group(1)), done1.group(2).lower(), int(done1.group(3))
    n2, h2, g2 = int(done2.group(1)), done2.group(2).lower(), int(done2.group(3))
    check("client1 frame count", n1 == expected_frames, f"frames={n1}")
    check("client2 frame count", n2 == expected_frames, f"frames={n2}")
    check("per-frame hash sequence equal", frames1 == frames2,
          f"len={len(frames1)}/{len(frames2)}")
    check("last hash equal", h1 == h2, f"hash={h1}")
    check("gameplay commands entered authoritative frame batch", g1 > 0 and g2 > 0,
          f"applied={g1}/{g2}")
    check("gameplay applied count equal", g1 == g2, f"applied={g1}/{g2}")
    check("client1 covers all gameplay kinds", EXPECTED_KINDS <= kinds1,
          f"kinds={','.join(sorted(kinds1))}")
    check("client2 covers all gameplay kinds", EXPECTED_KINDS <= kinds2,
          f"kinds={','.join(sorted(kinds2))}")
    return ok


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Real-UDP two-process smoke check for gameplay commands (tasks 8.6)."
    )
    parser.add_argument("--address", default="127.0.0.1:9000", help="relay bind address")
    parser.add_argument("--frames", type=int, default=30, help="frames per client")
    parser.add_argument("--timeout", type=float, default=120.0, help="client timeout in seconds")
    args = parser.parse_args()

    if not build_binaries():
        return 1
    for binary in (RELAY_BIN, CLIENT_BIN):
        if not binary.is_file():
            print(f"[fail] binary not found: {binary}", file=sys.stderr)
            print("[hint] check CARGO_TARGET_DIR and rerun cargo build", file=sys.stderr)
            return 1

    with tempfile.TemporaryDirectory(prefix="w3-online-smoke-") as tmp:
        tmp_dir = Path(tmp)
        relay_log = tmp_dir / "relay.log"
        client1_log = tmp_dir / "client1.log"
        client2_log = tmp_dir / "client2.log"
        processes: list[subprocess.Popen] = []
        try:
            # relay 是常驻进程：直接跑二进制而非 cargo run，确保 terminate 能杀到本体。
            with relay_log.open("w", encoding="utf-8") as relay_out:
                relay = subprocess.Popen(
                    [str(RELAY_BIN), args.address, "2"],
                    cwd=ROOT, stdout=subprocess.DEVNULL, stderr=relay_out,
                )
            processes.append(relay)
            if not wait_for_line(relay_log, "listening:", 15.0, relay):
                print("[fail] relay did not become ready", file=sys.stderr)
                return 1
            print("[run] relay ready")

            clients = []
            for player, log_path in ((1, client1_log), (2, client2_log)):
                with log_path.open("w", encoding="utf-8") as client_out:
                    proc = subprocess.Popen(
                        [str(CLIENT_BIN), args.address, str(player), str(args.frames)],
                        cwd=ROOT, stdout=subprocess.DEVNULL, stderr=client_out,
                    )
                clients.append(proc)
                processes.append(proc)
            print(f"[run] two clients started: frames={args.frames}")

            deadline = time.monotonic() + args.timeout
            for index, proc in enumerate(clients, start=1):
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    print("[fail] client timeout", file=sys.stderr)
                    return 1
                try:
                    proc.wait(timeout=remaining)
                except subprocess.TimeoutExpired:
                    print("[fail] client timeout", file=sys.stderr)
                    return 1
                if proc.returncode != 0:
                    print(f"[fail] client{index} exited with {proc.returncode}", file=sys.stderr)
                    return 1
            print("[run] both clients exited cleanly")

            if relay.poll() is not None:
                print("[fail] relay exited before clients finished", file=sys.stderr)
                return 1

            if verify(client1_log, client2_log, args.frames):
                print("Verification OK")
                return 0
            print("Verification FAILED", file=sys.stderr)
            return 1
        finally:
            # 统一收尾：relay 常驻、客户端可能超时，终止所有仍在运行的进程。
            for proc in processes:
                if proc.poll() is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        proc.kill()


if __name__ == "__main__":
    sys.exit(main())
