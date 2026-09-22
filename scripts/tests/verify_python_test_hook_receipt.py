#!/usr/bin/env python3
"""Validate and emit the canonical Python test-hook candidate receipt."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: verify_python_test_hook_receipt.py RECEIPT NONCE")
    receipt = Path(sys.argv[1]).resolve()
    expected_nonce = sys.argv[2]
    payload = json.loads(receipt.read_text())
    assert set(payload) == {
        "schema",
        "candidate_sha",
        "module_path",
        "module_sha256",
        "nonce",
    }
    assert payload["schema"] == "fathomdb.python-test-hooks-receipt/v1"
    assert payload["nonce"] == expected_nonce
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    assert payload["candidate_sha"] == head
    module = Path(payload["module_path"]).resolve()
    assert module.parent == (ROOT / "src/python/fathomdb").resolve()
    assert module.is_file()
    assert hashlib.sha256(module.read_bytes()).hexdigest() == payload["module_sha256"]
    print(
        "python-native-receipt: "
        f"candidate={head} module={module} sha256={payload['module_sha256']}"
    )


if __name__ == "__main__":
    main()
