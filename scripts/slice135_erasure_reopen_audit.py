#!/usr/bin/env python3
"""Independently validate the Slice 135 interrupted-erasure reopen witness."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


EXPECTED = {
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


def audit_log(log: str) -> dict:
    """Require one successful test and the exact raw-state transition record."""
    prefix = "SLICE135_ERASURE_REOPEN "
    records = [line[len(prefix) :] for line in log.splitlines() if line.startswith(prefix)]
    if len(records) != 1:
        raise ValueError(f"expected one erasure reopen record, found {len(records)}")
    success = "test pending_telemetry_redaction_survives_engine_reopen ... ok"
    if log.splitlines().count(success) != 1:
        raise ValueError("named erasure reopen test did not pass exactly once")
    result_lines = [line for line in log.splitlines() if line.startswith("test result:")]
    if len(result_lines) != 1 or not result_lines[0].startswith(
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;"
    ):
        raise ValueError("test result is missing or unsuccessful")
    observed = json.loads(records[0])
    if not isinstance(observed, dict):
        raise ValueError("erasure reopen record must be an object")
    if set(observed) != set(EXPECTED):
        raise ValueError("erasure reopen record field set changed")
    for field, expected in EXPECTED.items():
        value = observed[field]
        if type(value) is not type(expected) or value != expected:
            raise ValueError(f"{field} differs from the required state oracle")
    return {
        "status": "PASS",
        "log_sha256": hashlib.sha256(log.encode()).hexdigest(),
        "observed": observed,
    }


def main() -> None:
    """Audit one retained raw log and write its checked summary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    summary = audit_log(args.log.read_text())
    args.out.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
