"""Phase 2 audit must reject plausible false query-correctness receipts."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_contract_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_contract_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


FIXTURE = {
    "corpus": [
        {"kind": "note", "body": "alpha structured retrieval document"},
        {"kind": "note", "body": "beta structured search payload"},
        {"kind": "doc", "body": "delta retrieval and ranking notes"},
    ],
    "queries": [
        {
            "query": "structured",
            "expected_bodies": [
                "alpha structured retrieval document",
                "beta structured search payload",
            ],
        },
        {
            "query": "retrieval",
            "expected_bodies": [
                "alpha structured retrieval document",
                "delta retrieval and ranking notes",
            ],
        },
    ],
}
PROTOCOL = {
    "schema_version": 1,
    "fixture_sha256": "f" * 64,
    "runner_sha256": "r" * 64,
    "source_id": "slice135-phase2-x1",
    "logical_id_prefix": "phase2-x1-",
    "ordered_case": {
        "query": "retrieval",
        "expected_bodies": [
            "alpha structured retrieval document",
            "delta retrieval and ranking notes",
        ],
    },
    "versions": {
        "baseline": {"source_sha": "b" * 40, "wheel_sha256": "1" * 64, "native_sha256": "2" * 64},
        "candidate": {"source_sha": "c" * 40, "wheel_sha256": "3" * 64, "native_sha256": "4" * 64},
    },
}


def _hit(index: int) -> dict[str, str]:
    row = FIXTURE["corpus"][index]
    return {
        "id_space": "logical",
        "id_value": f"phase2-x1-{index}",
        "kind": row["kind"],
        "body": row["body"],
        "branch": "text",
        "source_id": "slice135-phase2-x1",
    }


def _raw() -> dict:
    return {
        "schema_version": 1,
        "protocol_sha256": "p" * 64,
        "fixture_sha256": "f" * 64,
        "runner_sha256": "r" * 64,
        "source_sha": "c" * 40,
        "wheel_sha256": "3" * 64,
        "native_sha256": "4" * 64,
        "version": "candidate",
        "cases": [
            {"query": "structured", "before": [_hit(0), _hit(1)], "after": [_hit(0), _hit(1)]},
            {"query": "retrieval", "before": [_hit(0), _hit(2)], "after": [_hit(0), _hit(2)]},
        ],
    }


def _audit(raw: dict) -> dict:
    return AUDIT.audit(
        raw,
        PROTOCOL,
        FIXTURE,
        version="candidate",
        protocol_sha256="p" * 64,
        fixture_sha256="f" * 64,
        runner_sha256="r" * 64,
    )


def test_complete_paired_shape_passes_without_using_version_agreement() -> None:
    result = _audit(_raw())
    assert result["queries"] == 2
    assert result["reopened_queries"] == 2
    assert result["exact_order_queries"] == 1


@pytest.mark.parametrize(
    ("change", "message"),
    [
        (lambda raw: raw["cases"][0]["before"][0].update(id_value="phase2-x1-2"), "id"),
        (lambda raw: raw["cases"][1]["after"].reverse(), "order"),
        (lambda raw: raw["cases"].pop(), "cases"),
        (lambda raw: raw.update(wheel_sha256="0" * 64), "wheel"),
        (lambda raw: raw["cases"][0]["after"][0].update(source_id="wrong"), "source"),
        (lambda raw: raw["cases"][0]["before"].append(_hit(0)), "duplicate"),
    ],
)
def test_independent_audit_rejects_false_result(change, message: str) -> None:
    raw = _raw()
    change(raw)
    with pytest.raises(ValueError, match=message):
        _audit(raw)
