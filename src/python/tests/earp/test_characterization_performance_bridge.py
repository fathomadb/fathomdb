"""Independent repeated performance for a resolved corpus-scale EARP workload."""

from __future__ import annotations

import pytest
from pathlib import Path

from eval.performance.earp_adapter import (
    PerformancePlan,
    WorkloadRef,
    run_characterization_repetitions,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_bridge_repeats_the_same_characterization_workload(tmp_path: Path) -> None:
    from test_characterization import _bed  # noqa: PLC0415
    from eval.earp.config import ResolvedScenario  # noqa: PLC0415
    from eval.earp.schema.models import CampaignKind, RetrievalMode  # noqa: PLC0415

    bed = _bed(tmp_path)
    config = {
        "schema_version": "earp.v1",
        "campaign": "characterization",
        "corpus": {"snapshot": str(bed["snapshot_path"]), "data_root": str(bed["data_root"])},
        "gold": {
            "path": str(bed["gold_path"]),
            "sha256": bed["gold_sha256"],
            "corpus_hash": bed["corpus_hash"],
            "qrels_version": bed["qrels_version"],
        },
        "scenario": {"engine": {"use_default_embedder": False}, "query": {"call": "Engine.search_text_only", "limit": 10}},
        "metrics": {"evidence_recall_k": [5, 10]},
    }
    scenario = ResolvedScenario(
        campaign=CampaignKind.CHARACTERIZATION,
        config_sha256="a" * 64,
        query_call="Engine.search_text_only",
        retrieval_mode=RetrievalMode.FTS_ONLY,
        max_measurable_k=10,
        use_default_embedder=False,
        query_params={"limit": 10},
        evidence_recall_k=(5, 10),
        document_metrics=(),
        corpus=config["corpus"],
        gold=config["gold"],
        decision_rule=None,
        consumed_paths=frozenset(),
        carried_paths=frozenset(),
    )
    workload = WorkloadRef(
        parent_run_id="quality-run",
        evidence_family_id="quality-run",
        config_sha256="a" * 64,
        candidate_sha="b" * 40,
        query_call="Engine.search_text_only",
        effective_knobs={"limit": 10},
    )

    samples = run_characterization_repetitions(
        workload=workload,
        plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "warm")),
        scenario=scenario,
        config_doc=config,
    )

    assert [(sample.treatment, sample.repetition) for sample in samples] == [
        ("fresh_store", 0),
        ("warm", 0),
    ]
    assert all({"open", "write", "query"} <= set(sample.phases_ms) for sample in samples)
