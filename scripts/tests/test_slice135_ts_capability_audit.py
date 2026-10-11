"""Negative controls for independent TypeScript capability accounting."""
import pytest

from scripts.slice135_ts_capability_audit import recount


def test_recount_rejects_omitted_operation():
    with pytest.raises(ValueError, match="canonical"):
        recount(["engine.open", "read.get"], {"engine.open": {"status": "executed"}})


def test_recount_rejects_vacuous_execution():
    with pytest.raises(ValueError, match="evidence"):
        recount(["engine.open"], {"engine.open": {"status": "executed"}})


def test_recount_disjoins_statuses():
    assert recount(["engine.open", "read.get"], {
        "engine.open": {"status": "failed", "error": "error"},
        "read.get": {"status": "gap", "reason": "missing", "owner": "Slice 135"},
    }) == {"supported": 2, "executed": 0, "failed": 1, "gap": 1, "unavailable": 0}


def test_recount_requires_positive_closure_and_trace_cases():
    for operation, positive, negative in (
        ("engine.read_dependency_closure", "closure_committed", "closure_absent"),
        ("engine.trace_dependency", "trace_success", "trace_refusal"),
    ):
        row = {
            "status": "executed",
            "positive": {"case": positive},
            "negative": {"case": negative},
            "reopen": {"databases": 1},
        }
        assert recount([operation], {operation: row})["executed"] == 1
        row["positive"]["case"] = negative
        with pytest.raises(ValueError, match="positive route"):
            recount([operation], {operation: row})
