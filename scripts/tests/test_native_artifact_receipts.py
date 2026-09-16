#!/usr/bin/env python3
"""Hermetic contract for five-target native artifact receipts."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/release/native-artifact-receipts.py"
WORKFLOW = ROOT / ".github/workflows/ci.yml"
CANDIDATE = "a" * 40
TARGETS = (
    ("x86_64-unknown-linux-gnu", "linux-x64"),
    ("aarch64-unknown-linux-gnu", "linux-arm64"),
    ("x86_64-apple-darwin", "darwin-x64"),
    ("aarch64-apple-darwin", "darwin-arm64"),
    ("x86_64-pc-windows-msvc", "win32-x64"),
)


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        text=True,
        capture_output=True,
        check=False,
    )


def main() -> None:
    assert SCRIPT.is_file(), f"missing native receipt helper: {SCRIPT}"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        receipts = root / "receipts"
        receipts.mkdir()
        for target, label in TARGETS:
            wheel = root / f"fathomdb-{label}.whl"
            napi = root / f"fathomdb.{label}.node"
            wheel.write_bytes(f"wheel:{label}".encode())
            napi.write_bytes(f"napi:{label}".encode())
            output = receipts / f"native-artifact-receipt-{label}.json"
            result = run(
                "write",
                "--candidate-sha", CANDIDATE,
                "--target", target,
                "--label", label,
                "--runner", f"runner-{label}",
                "--rust", "1.95.0",
                "--python", "3.11",
                "--node", "25.9.0",
                "--wheel", str(wheel),
                "--napi", str(napi),
                "--command", "native-artifact-runtime-validation",
                "--output", str(output),
            )
            assert result.returncode == 0, result.stderr
            payload = json.loads(output.read_text(encoding="utf-8"))
            assert payload["schema_version"] == "fathomdb.native-artifact-receipt/v1"
            assert payload["candidate_sha"] == CANDIDATE
            assert payload["target"] == target
            assert payload["outcome"] == "pass"
            by_kind = {row["kind"]: row for row in payload["artifacts"]}
            assert set(by_kind) == {"wheel", "napi"}
            assert by_kind["wheel"]["size"] == wheel.stat().st_size
            assert by_kind["wheel"]["sha256"] == hashlib.sha256(
                wheel.read_bytes()
            ).hexdigest()

        matrix = root / "native-artifact-matrix.json"
        collected = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert collected.returncode == 0, collected.stderr
        payload = json.loads(matrix.read_text(encoding="utf-8"))
        assert payload["schema_version"] == "fathomdb.native-artifact-matrix/v1"
        assert payload["candidate_sha"] == CANDIDATE
        assert {row["target"] for row in payload["platforms"]} == {
            target for target, _label in TARGETS
        }

        (receipts / "native-artifact-receipt-linux-arm64.json").unlink()
        missing = run(
            "collect",
            "--candidate-sha", CANDIDATE,
            "--receipt-dir", str(receipts),
            "--output", str(matrix),
        )
        assert missing.returncode == 1
        assert "missing target" in missing.stderr, missing.stderr

    workflow = WORKFLOW.read_text(encoding="utf-8")
    assert "native-artifact-receipts.py write" in workflow
    assert "native-artifact-receipt-${{ matrix.label }}" in workflow
    assert "collect-native-artifact-receipts:" in workflow
    assert "native-artifact-receipts.py collect" in workflow
    print("PASS test-native-artifact-receipts")


if __name__ == "__main__":
    main()
