#!/usr/bin/env python3
"""Fail-closed contract for Python artifact receipt and harness routing."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
VALIDATOR = ROOT / "scripts/tests/verify_python_test_hook_receipt.py"


def main() -> None:
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    with tempfile.TemporaryDirectory() as directory:
        receipt = Path(directory) / "receipt.json"
        receipt.write_text(
            json.dumps(
                {
                    "schema": "fathomdb.python-test-hooks-receipt/v1",
                    "candidate_sha": head,
                    "module_path": str(ROOT / "src/python/fathomdb/missing.abi3.so"),
                    "module_sha256": hashlib.sha256(b"missing").hexdigest(),
                    "nonce": "actual",
                }
            )
        )
        environment = dict(os.environ)
        environment["PYTHONOPTIMIZE"] = "1"
        wrong_nonce = subprocess.run(
            [sys.executable, str(VALIDATOR), str(receipt), "wrong"],
            cwd=ROOT,
            env=environment,
            capture_output=True,
            text=True,
        )
        assert wrong_nonce.returncode != 0, wrong_nonce.stdout

    harness = (ROOT / "scripts/agent-test.sh").read_text()
    assert 'python_receipt_skip_reason="candidate receipt requires' in harness
    assert 'napi_execute_skip_reason="src/ts/node_modules not installed"' in harness
    print("ok    python-artifact-gate-contract")


if __name__ == "__main__":
    main()
