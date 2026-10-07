"""Reject altered capability accounting and persisted-state observations."""

from copy import deepcopy
import hashlib
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_capabilities_audit as audit  # noqa: E402


def snapshot() -> dict:
    """Build a one-row canonical state with an independently derived digest."""
    row = [1, "doc", "body", "source", "logical", None, None, None, "active"]
    rows = {"canonical_nodes": [row]}
    digest = hashlib.sha256(repr({"canonical_nodes": [tuple(row)]}).encode()).hexdigest()
    return {
        "persisted_rows": rows,
        "table_counts": {"canonical_nodes": 1},
        "canonical_operational_sha256": digest,
        "active_bodies": [["logical", "body"]],
        "active_bodies_checked": 1,
        "missing_id": None,
    }


def test_reopen_snapshot_rejects_changed_body_and_digest() -> None:
    observed = snapshot()
    audit.audit_snapshot(observed)
    wrong_body = deepcopy(observed)
    wrong_body["active_bodies"][0][1] = "wrong"
    with pytest.raises(ValueError, match="active bodies"):
        audit.audit_snapshot(wrong_body)
    wrong_rows = deepcopy(observed)
    wrong_rows["persisted_rows"]["canonical_nodes"][0][2] = "wrong"
    with pytest.raises(ValueError, match="snapshot digest"):
        audit.audit_snapshot(wrong_rows)


def test_operation_accounting_rejects_false_count_and_missing_case() -> None:
    observed = snapshot()
    cases = {
        "positive": {"status": "passed", "reopen": [observed]},
        "negative": {"status": "passed", "reopen": [observed]},
    }
    operations = {
        "engine.open": {
            "status": "executed",
            "positive": {"case": "positive"},
            "negative": {"case": "negative"},
            "reopen": {"case": "positive", "checks": [observed]},
        }
    }
    counts = {"supported": 1, "executed": 1, "failed": 0, "gap": 0, "unavailable": 0, "unexecuted": 0}
    audit.audit_operations(["engine.open"], operations, cases, counts)
    bad_counts = counts | {"executed": 0}
    with pytest.raises(ValueError, match="operation counts"):
        audit.audit_operations(["engine.open"], operations, cases, bad_counts)
    missing_case = deepcopy(operations)
    missing_case["engine.open"]["positive"]["case"] = "absent"
    with pytest.raises(ValueError, match="missing or failed case"):
        audit.audit_operations(["engine.open"], missing_case, cases, counts)
