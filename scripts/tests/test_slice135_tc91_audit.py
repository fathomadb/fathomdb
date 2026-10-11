"""Negative controls for the Slice 135 projection-fault log auditor."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_tc91_audit as audit  # noqa: E402


def test_rejects_changed_result_and_missing_expected_panic() -> None:
    names = sorted(audit.EXPECTED_TESTS)
    stdout = (
        "running 6 tests\n"
        + "".join(f"test {name} ... ok\n" for name in names)
        + "test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s\n"
    )
    stderr = (
        "thread 'worker' panicked at test: intentional first projection panic\n"
        "thread 'worker' panicked at test: intentional subscriber panic after projection commit failure\n"
    )
    assert audit.check_test_output(stdout, stderr) == names
    with pytest.raises(ValueError, match="six passing tests"):
        audit.check_test_output(stdout.replace(" ... ok", " ... FAILED", 1), stderr)
    with pytest.raises(ValueError, match="expected panic"):
        audit.check_test_output(
            stdout, stderr.replace("intentional first projection panic", "missing")
        )
