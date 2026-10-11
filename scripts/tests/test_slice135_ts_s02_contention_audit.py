"""TypeScript contention receipts need independent interval verification."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_contention_audit as audit  # noqa: E402


def test_reject_falsified_overlap_and_missing_writer_call() -> None:
    writer = [
        {"logical_id": "contend-00", "start_ns": 10, "end_ns": 20},
        {"logical_id": "contend-01", "start_ns": 30, "end_ns": 40},
    ]
    reader = [
        {"start_ns": 15, "end_ns": 25, "vector_hits": 1},
        {"start_ns": 41, "end_ns": 50, "vector_hits": 1},
    ]
    assert audit.audit_timeline(writer, reader, claimed=1, expected=2) == 1
    with pytest.raises(ValueError, match="overlap"):
        audit.audit_timeline(writer, reader, claimed=2, expected=2)
    with pytest.raises(ValueError, match="writer count"):
        audit.audit_timeline(writer[:1], reader, claimed=1, expected=2)


def test_reject_bad_interval_and_nonvector_read() -> None:
    writer = [{"logical_id": "contend-00", "start_ns": 1, "end_ns": 10}]
    reader = [{"start_ns": 2, "end_ns": 3, "vector_hits": 1}]
    with pytest.raises(ValueError, match="interval"):
        audit.audit_timeline([{**writer[0], "end_ns": 1}], reader,
                             claimed=1, expected=1)
    with pytest.raises(ValueError, match="vector"):
        audit.audit_timeline(writer, [{**reader[0], "vector_hits": 0}],
                             claimed=1, expected=1)
