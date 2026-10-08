"""Phase 2 matrix cases keep success, empty success and typed refusal distinct."""

from __future__ import annotations

import importlib.util
from copy import deepcopy
import json
from pathlib import Path

import fathomdb
import pytest


SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parent


def _module(name: str):
    path = SCRIPTS / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUNNER = _module("slice135_phase2_matrix_runner")
AUDITOR = _module("slice135_phase2_matrix_audit")


REQUESTS = [
    {"id": "filter_note", "op": "search", "query": "retrieval", "filter": {"kind": "note"}},
    {"id": "empty_status", "op": "search", "query": "retrieval", "filter": {"status": "open"}},
    {"id": "page_note", "op": "canonical_page", "kind": "note", "limit": 1},
    {"id": "invalid_page", "op": "canonical_page", "kind": "note", "limit": 0},
]


def _gold() -> dict:
    source = "phase2-matrix-source"
    return {
        "filter_note": {
            "kind": "success",
            "hits": [{
                "id_space": "logical", "id_value": "phase2-matrix-0", "kind": "note",
                "body": "alpha structured retrieval document", "branch": "text", "source_id": source,
            }],
        },
        "empty_status": {"kind": "success", "hits": []},
        "page_note": {
            "kind": "success",
            "pages": [
                {"items": [{"logical_id": "phase2-matrix-0", "kind": "note", "body": "alpha structured retrieval document"}], "has_more": True},
                {"items": [{"logical_id": "phase2-matrix-1", "kind": "note", "body": "beta structured search payload"}], "has_more": False},
            ],
        },
        "invalid_page": {"kind": "error", "error_type": "PageError", "reason": "invalid_page_limit", "field_path": "/limit"},
    }


def _cases() -> list[dict]:
    return [{"id": request["id"], "before": deepcopy(outcome), "after": deepcopy(outcome)}
            for request in REQUESTS for outcome in [_gold()[request["id"]]]]


def test_runner_records_authored_matrix_against_real_database(tmp_path: Path) -> None:
    fixture = json.loads((ROOT / "src/python/tests/functional_search_fixture.json").read_text())
    engine = fathomdb.Engine.open(str(tmp_path / "matrix.sqlite"), use_default_embedder=False)
    try:
        rows = [
            {"kind": row["kind"], "body": row["body"], "logical_id": f"phase2-matrix-{index}", "source_id": "phase2-matrix-source"}
            for index, row in enumerate(fixture["corpus"])
        ]
        for row in rows:
            engine.write([row])
        engine.drain(timeout_s=30)
        observed = RUNNER.observe_matrix(engine, REQUESTS)
    finally:
        engine.close()
    assert {case["id"]: case["outcome"] for case in observed} == _gold()


def test_auditor_accepts_complete_authored_matrix() -> None:
    assert AUDITOR.audit_cases(_cases(), REQUESTS, _gold()) == {"cases": 4, "reopened_cases": 4}


@pytest.mark.parametrize(
    ("case_id", "replacement"),
    [
        ("filter_note", {"kind": "success", "hits": []}),
        ("empty_status", {"kind": "error", "error_type": "PageError", "reason": "invalid_page_limit", "field_path": "/limit"}),
        ("invalid_page", {"kind": "success", "pages": []}),
    ],
)
def test_auditor_rejects_wrong_filter_empty_or_error(case_id: str, replacement: dict) -> None:
    cases = _cases()
    next(case for case in cases if case["id"] == case_id)["before"] = replacement
    with pytest.raises(ValueError):
        AUDITOR.audit_cases(cases, REQUESTS, _gold())


def test_auditor_rejects_wrong_page_order_after_reopen() -> None:
    cases = _cases()
    next(case for case in cases if case["id"] == "page_note")["after"]["pages"].reverse()
    with pytest.raises(ValueError):
        AUDITOR.audit_cases(cases, REQUESTS, _gold())


def test_auditor_rejects_missing_case() -> None:
    with pytest.raises(ValueError):
        AUDITOR.audit_cases(_cases()[:-1], REQUESTS, _gold())
