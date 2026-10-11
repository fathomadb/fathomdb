"""Independent checks for the interrupted-erasure reopen receipt."""

from __future__ import annotations

import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_erasure_reopen_audit as audit  # noqa: E402


OBSERVED = {
    "case": "pending_telemetry_redaction_survives_engine_reopen",
    "fault_point": "telemetry sink rotated after search and before excise",
    "first_error": "ErasureIncomplete:telemetry_redaction",
    "retry_without_sink_error": "ErasureIncomplete:telemetry_redaction",
    "pending_after_failure": 1,
    "pending_after_reopen": 1,
    "pending_after_retry": 0,
    "victim_rows_after_failure": 0,
    "victim_rows_after_retry": 0,
    "control_rows_after_retry": 1,
    "victim_in_rotated_sink_before_restore": True,
    "victim_in_restored_sink_after_retry": False,
    "control_in_restored_sink_after_retry": True,
    "integrity_check": "ok",
}


def log_for(value: dict) -> str:
    return (
        "running 1 test\n"
        f"SLICE135_ERASURE_REOPEN {json.dumps(value)}\n"
        "test pending_telemetry_redaction_survives_engine_reopen ... ok\n"
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out\n"
    )


def test_accepts_exact_reopen_state() -> None:
    result = audit.audit_log(log_for(OBSERVED))
    assert result["status"] == "PASS"
    assert result["observed"] == OBSERVED


@pytest.mark.parametrize(
    ("field", "wrong"),
    [
        ("pending_after_reopen", 0),
        ("pending_after_retry", 1),
        ("victim_rows_after_retry", 1),
        ("control_in_restored_sink_after_retry", False),
        ("integrity_check", "corrupt"),
    ],
)
def test_rejects_wrong_state(field: str, wrong: object) -> None:
    changed = dict(OBSERVED)
    changed[field] = wrong
    with pytest.raises(ValueError, match=field):
        audit.audit_log(log_for(changed))


def test_rejects_duplicate_or_missing_result() -> None:
    sample = log_for(OBSERVED)
    with pytest.raises(ValueError, match="record"):
        audit.audit_log(sample + sample)
    with pytest.raises(ValueError, match="test result"):
        audit.audit_log(sample.replace("test result: ok.", "test result: FAILED."))
