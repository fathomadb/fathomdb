"""Fixture invariants for the Slice 135 S03 installed-Python workload."""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s03 as s03  # noqa: E402
import slice135_python_s03_audit as audit  # noqa: E402


def test_fixture_preserves_named_semantic_controls_and_scale() -> None:
    for size in (32, 256):
        records = s03.make_seed(size)
        nodes = [row for row in records if "edge" not in row]
        edges = [row["edge"] for row in records if "edge" in row]
        ids = {row["logical_id"] for row in nodes}
        assert len(ids) == len(nodes)
        assert (
            len([row for row in nodes if row["source_id"] == s03.MEMORY_SOURCE]) == size
        )
        assert {"s03-early", "s03-late", "s02-source", "s02-root", "s02-claim"} <= ids
        assert len(edges) == 1
        assert edges[0]["from"] == "s02-root" and edges[0]["to"] == "s02-claim"
        assert (
            next(row for row in nodes if row["logical_id"] == "s03-early")[
                "valid_until"
            ]
            == 2000
        )
        assert (
            next(row for row in nodes if row["logical_id"] == "s03-late")["valid_from"]
            == 3000
        )
        source = next(row for row in nodes if row["logical_id"] == "s02-source")
        claim = next(row for row in nodes if row["logical_id"] == "s02-claim")
        assert (
            claim["provenance"]["canonical_source_hash"]["digest_hex"]
            == hashlib.sha256(source["body"].encode()).hexdigest()
        )


def test_sample_checker_rejects_semantic_regression() -> None:
    good = {
        "filter": ["alpha structured retrieval document"],
        "temporal_early": ["epoch alpha record"],
        "temporal_boundary": [],
        "temporal_late": ["epoch beta record"],
        "graph": ["s02-claim"],
        "evidence": [s03.SOURCE_BODY],
        "memory_32": ["F0000"],
    }
    for name, value in good.items():
        assert s03.check_case(name, value, 32)
    assert not s03.check_case("temporal_boundary", ["epoch alpha record"], 32)
    assert not s03.check_case("evidence", ["wrong source"], 32)
    assert not s03.check_case("memory_32", ["unknown"], 32)


def test_independent_auditor_rejects_false_semantic_claim() -> None:
    correct = {"observed": ["epoch alpha record"], "semantic_ok": True, "latency_ns": 5}
    assert audit.check_attempt("temporal_early", correct, 32) == 5
    wrong = {**correct, "observed": ["epoch beta record"]}
    try:
        audit.check_attempt("temporal_early", wrong, 32)
    except ValueError as error:
        assert "temporal_early" in str(error)
    else:
        raise AssertionError("auditor accepted an incorrect temporal result")
