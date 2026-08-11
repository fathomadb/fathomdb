"""0.8.24 performance-evidence contracts.

These tests deliberately exercise the two evidence planes without opening a
native engine: correctness of the durable artifact contract must not depend on
whether a developer has a built extension or a cached embedder.
"""

from __future__ import annotations

from pathlib import Path

from eval.earp.observed_cost import (
    Observation,
    capture_sqlite_storage,
)
from eval.performance.earp_adapter import (
    PerformancePlan,
    RunSample,
    summarize_samples,
)

SHA = "a" * 64


def test_observed_cost_sidecar_is_explicitly_one_run_evidence(tmp_path: Path) -> None:
    db = tmp_path / "scenario.db"
    db.write_bytes(b"main")
    (tmp_path / "scenario.db-wal").write_bytes(b"wal")

    observation = Observation(
        evidence_family_id="quality-run-1",
        config_sha256=SHA,
        phases_ms={"open": 1.25, "write": 3.5, "query": 0.75},
        counts={"accepted": 2, "queries": 1, "results": 1},
        storage=capture_sqlite_storage(db),
        query_samples=(
            {"query_id": "q-1", "wall_ms": 0.75, "result_count": 1, "outcome": "complete"},
        ),
        unavailable={"engine_trace": {"code": "not_exposed", "message": "no trace hook"}},
        provenance={"candidate_sha": "b" * 40, "clean": True, "device": {"kind": "cpu"}},
    )

    document = observation.as_document()
    assert document["schema_version"] == "earp.observed-cost.v2"
    assert document["scope"] == "one_run_observation"
    assert document["evidence_family_id"] == "quality-run-1"
    assert document["storage"] == {"database_bytes": 4, "wal_bytes": 3, "shm_bytes": 0}
    assert document["query_samples"][0]["query_id"] == "q-1"
    assert document["unavailable"]["engine_trace"]["code"] == "not_exposed"
    assert document["provenance"]["candidate_sha"] == "b" * 40


def test_short_repetition_summary_cannot_claim_percentiles() -> None:
    plan = PerformancePlan(
        repetitions=2, treatments=("fresh_store", "fresh_store_warm_query")
    )
    samples = (
        RunSample("fresh_store", 0, {"query": 4.0}, {"results": 1}),
        RunSample("fresh_store", 1, {"query": 6.0}, {"results": 1}),
        RunSample("fresh_store_warm_query", 0, {"query": 2.0}, {"results": 1}),
        RunSample("fresh_store_warm_query", 1, {"query": 3.0}, {"results": 1}),
    )
    summary = summarize_samples(samples)
    for treatment in plan.treatments:
        assert summary[treatment]["query"] == {
            "n": 2,
            "min_ms": 2.0 if treatment.endswith("warm_query") else 4.0,
            "max_ms": 3.0 if treatment.endswith("warm_query") else 6.0,
            "mean_ms": 2.5 if treatment.endswith("warm_query") else 5.0,
        }

def test_summary_percentiles_require_the_declared_completed_sample_count() -> None:
    """Percentile fields are eligibility-gated, never decorative labels."""
    def samples(count: int) -> tuple[RunSample, ...]:
        return tuple(
            RunSample("fresh_store", index, {"query": float(index + 1)}, {"results": 1})
            for index in range(count)
        )

    under_twenty = summarize_samples(samples(19))["fresh_store"]["query"]
    assert set(under_twenty) == {"n", "min_ms", "max_ms", "mean_ms"}

    descriptive = summarize_samples(samples(20))["fresh_store"]["query"]
    assert set(descriptive) == {
        "n",
        "min_ms",
        "max_ms",
        "mean_ms",
        "p50_ms",
        "p95_ms",
        "aggregation_scope",
    }
    assert descriptive["aggregation_scope"] == "descriptive_empirical_order_statistic"

    hundred = summarize_samples(samples(100))["fresh_store"]["query"]
    assert "p99_ms" in hundred
