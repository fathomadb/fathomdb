#!/usr/bin/env python3
"""Cross-platform stdio and --check-config smoke for a fathomdb-mcp binary."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

EXPECTED_READONLY_TOOLS = [
    "expand",
    "feedback",
    "fetch",
    "list_stores",
    "orient",
    "search",
    "use_store",
]


def _run_check_config(binary: Path, config_path: Path, expected_exit: int, expected_valid: bool) -> None:
    proc = subprocess.run(
        [str(binary), "--config", str(config_path), "--check-config"],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if proc.returncode != expected_exit:
        raise AssertionError(
            f"--check-config exit mismatch for {config_path.name}: "
            f"expected {expected_exit}, got {proc.returncode}; stderr={proc.stderr!r}"
        )
    if proc.stdout != "":
        raise AssertionError(
            f"--check-config must leave stdout empty; got {proc.stdout!r}"
        )
    report = json.loads(proc.stderr.strip())
    if report.get("valid") is not expected_valid:
        raise AssertionError(
            f"--check-config valid mismatch: expected {expected_valid}, got {report!r}"
        )


def _run_stdio_session(binary: Path, config_path: Path) -> None:
    proc = subprocess.Popen(
        [str(binary), "--config", str(config_path)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    assert proc.stdin is not None and proc.stdout is not None and proc.stderr is not None

    def send(payload: dict) -> None:
        proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        proc.stdin.flush()

    def recv() -> dict:
        line = proc.stdout.readline()
        if not line:
            stderr_tail = proc.stderr.read()
            raise AssertionError(f"unexpected EOF on stdout; stderr={stderr_tail!r}")
        return json.loads(line)

    try:
        send(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "fathomdb-mcp-smoke", "version": "1"},
                },
            }
        )
        init = recv()
        if init.get("id") != 1 or "result" not in init:
            raise AssertionError(f"invalid initialize response: {init!r}")
        server_name = init["result"].get("serverInfo", {}).get("name")
        if server_name != "fathomdb-mcp":
            raise AssertionError(f"unexpected serverInfo.name: {server_name!r}")

        send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})

        send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        tools_resp = recv()
        if tools_resp.get("id") != 2 or "result" not in tools_resp:
            raise AssertionError(f"invalid tools/list response: {tools_resp!r}")
        tool_names = sorted(t["name"] for t in tools_resp["result"]["tools"])
        if tool_names != EXPECTED_READONLY_TOOLS:
            raise AssertionError(
                f"read-only tools/list mismatch: expected {EXPECTED_READONLY_TOOLS}, got {tool_names}"
            )

        send(
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "list_stores", "arguments": {}},
            }
        )
        call_resp = recv()
        if call_resp.get("id") != 3 or "result" not in call_resp:
            raise AssertionError(f"invalid list_stores response: {call_resp!r}")
        if call_resp["result"].get("isError") is True:
            raise AssertionError(f"list_stores returned error: {call_resp!r}")
        text = call_resp["result"]["content"][0]["text"]
        if "stores: 0" not in text:
            raise AssertionError(f"unexpected list_stores content: {text!r}")

        proc.stdin.close()
        stderr = proc.stderr.read()
        rc = proc.wait(timeout=30)
        if rc != 0:
            raise AssertionError(f"fathomdb-mcp exited {rc} after clean stdin close; stderr={stderr!r}")
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=10)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        print(f"mcp-stdio-smoke: binary not found: {binary}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory(prefix="fathomdb-mcp-smoke-") as tmp:
        root = Path(tmp)
        valid_cfg = root / "valid.json"
        valid_cfg.write_text('{"stores": {}}\n', encoding="utf-8")
        invalid_cfg = root / "invalid-sc01.json"
        invalid_cfg.write_text(
            '{"stores": {"bad": {"path": "bad.sqlite", "allow_writes": true}}}\n',
            encoding="utf-8",
        )

        _run_check_config(binary, valid_cfg, expected_exit=0, expected_valid=True)
        _run_check_config(binary, invalid_cfg, expected_exit=2, expected_valid=False)
        if (root / "bad.sqlite").exists() or (root / "bad.sqlite.lock").exists():
            raise AssertionError("SC-01 --check-config created database or lock files")
        _run_stdio_session(binary, valid_cfg)

    print(f"mcp-stdio-smoke: ok ({binary.name})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
