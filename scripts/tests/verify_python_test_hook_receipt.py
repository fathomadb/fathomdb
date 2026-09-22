#!/usr/bin/env python3
"""Validate and emit the canonical Python test-hook candidate receipt."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"invalid Python native receipt: {message}")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: verify_python_test_hook_receipt.py RECEIPT NONCE")
    receipt = Path(sys.argv[1]).resolve()
    expected_nonce = sys.argv[2]
    payload = json.loads(receipt.read_text())
    require(set(payload) == {
        "schema",
        "candidate_sha",
        "module_path",
        "module_sha256",
        "nonce",
    }, "unexpected fields")
    require(
        payload["schema"] == "fathomdb.python-test-hooks-receipt/v1",
        "unsupported schema",
    )
    require(payload["nonce"] == expected_nonce, "nonce mismatch")
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    status = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=ROOT,
        text=True,
    )
    require(not status, f"worktree is dirty:\n{status}")
    require(payload["candidate_sha"] == head, "candidate SHA does not match HEAD")
    module = Path(payload["module_path"]).resolve()
    require(
        module.parent == (ROOT / "src/python/fathomdb").resolve(),
        "module is outside the source package",
    )
    require(module.is_file(), "module is missing")
    require(
        hashlib.sha256(module.read_bytes()).hexdigest() == payload["module_sha256"],
        "module digest mismatch",
    )
    print(
        "python-native-receipt: "
        f"candidate={head} module={module} sha256={payload['module_sha256']}"
    )


if __name__ == "__main__":
    main()
