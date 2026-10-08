"""Negative controls for exact-candidate TypeScript S01 receipt auditing."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s01_integrated_audit as audit  # noqa: E402


def test_current_candidate_artifact_expectations() -> None:
    base = audit.load_base_auditor()
    assert base.NATIVE_SHA256["candidate"] == (
        "2d26e58cda598d702c3dc333beaeec355e268c093c342fbf1efd26e4ccd5186d"
    )
    assert base.ARCHIVES["candidate"] == (
        "candidate-main.tgz",
        "candidate-platform.tgz",
    )


def test_run_order_rejects_missing_or_wrong_role() -> None:
    expected = [(32, 1, "baseline"), (32, 1, "candidate")]
    records = [
        {
            "name": f"32-01-{role}",
            "rows": 32,
            "pair": 1,
            "role": role,
            "samples": 1000,
            "exit_code": 0,
            "attempt_status": "VALID_TS_S01_PAIRED_BLOCK",
            "invalidators": [],
            "protocol_sha256": "a" * 64,
        }
        for _, _, role in expected
    ]
    audit.check_run_order(records, expected, "a" * 64)
    with pytest.raises(ValueError, match="count"):
        audit.check_run_order(records[:-1], expected, "a" * 64)
    changed = deepcopy(records)
    changed[1]["role"] = "baseline"
    with pytest.raises(ValueError, match="order"):
        audit.check_run_order(changed, expected, "a" * 64)
