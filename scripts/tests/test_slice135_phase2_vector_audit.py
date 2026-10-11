"""Phase 2 vector fidelity audit uses exact vector truth and fixed denominators."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import importlib.util
from pathlib import Path
import struct

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_vector_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_vector_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDITOR)


def _hash(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def _f32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


def _receipt() -> dict:
    docs = [
        {"logical_id": "A", "body_sha256": _hash("A"), "vector": [0.0, 0.0]},
        {"logical_id": "B", "body_sha256": _hash("B"), "vector": [0.1, 0.0]},
        {"logical_id": "B2", "body_sha256": _hash("B"), "vector": [0.1, 0.0]},
        {"logical_id": "C", "body_sha256": _hash("C"), "vector": [0.2, 0.0]},
        {"logical_id": "D", "body_sha256": _hash("D"), "vector": [0.3, 0.0]},
    ]
    return {
        "documents": docs,
        "queries": [
            {
                "query_id": "q1",
                "target_body_sha256": _hash("A"),
                "vector": [0.0, 0.0],
                "text_only_count": 0,
                "hits": [
                    {"logical_id": "A", "body_sha256": _hash("A"), "branch": "vector"},
                    {"logical_id": "B", "body_sha256": _hash("B"), "branch": "vector"},
                    {"logical_id": "B2", "body_sha256": _hash("B"), "branch": "vector"},
                    {"logical_id": "C", "body_sha256": _hash("C"), "branch": "vector"},
                    {"logical_id": "D", "body_sha256": _hash("D"), "branch": "vector"},
                ],
            }
        ],
    }


def test_f32_distance_rounds_each_step_like_the_rust_oracle() -> None:
    a = [0.1, 0.2, 0.3]
    b = [0.4, 0.5, 0.6]
    expected = _f32(0.0)
    for left, right in zip(a, b):
        difference = _f32(_f32(left) - _f32(right))
        expected = _f32(expected + _f32(difference * difference))
    assert AUDITOR.f32_squared_l2(a, b) == expected


def test_exact_neighbors_exclude_target_and_deduplicate_by_body() -> None:
    receipt = _receipt()
    assert AUDITOR.exact_neighbors(
        receipt["documents"], [0.0, 0.0], _hash("A"), 2
    ) == [_hash("B"), _hash("C")]
    assert AUDITOR.score_version(receipt, ["q1"], k=2, dimension=2)["per_query"] == {
        "q1": 1.0
    }


def test_wrong_vector_hit_lowers_score_without_changing_denominator() -> None:
    receipt = _receipt()
    receipt["queries"][0]["hits"] = receipt["queries"][0]["hits"][:1] + [
        receipt["queries"][0]["hits"][-1]
    ]
    assert AUDITOR.score_version(receipt, ["q1"], k=2, dimension=2)["per_query"] == {
        "q1": 0.0
    }


@pytest.mark.parametrize(
    "mutate",
    [
        lambda receipt: receipt["queries"].clear(),
        lambda receipt: receipt["queries"][0].update(text_only_count=1),
        lambda receipt: receipt["queries"][0]["hits"][0].update(branch="text"),
        lambda receipt: receipt["queries"][0]["hits"][0].update(logical_id="unknown"),
        lambda receipt: receipt["queries"][0]["hits"][0].update(body_sha256=_hash("wrong")),
        lambda receipt: receipt["documents"][0].update(vector=[0.0]),
        lambda receipt: receipt["queries"][0].update(vector=[float("nan"), 0.0]),
    ],
)
def test_audit_rejects_missing_or_malformed_receipts(mutate) -> None:
    receipt = deepcopy(_receipt())
    mutate(receipt)
    with pytest.raises(ValueError):
        AUDITOR.score_version(receipt, ["q1"], k=2, dimension=2)
