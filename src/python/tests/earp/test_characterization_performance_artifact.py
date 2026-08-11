"""A corpus-scale performance bridge publishes an independent sidecar."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.config import ResolvedScenario
from eval.earp.schema.models import CampaignKind, RetrievalMode
from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    WorkloadRef,
    run_and_write_characterization_performance,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_characterization_performance_writes_only_the_linked_artifact(
    tmp_path: Path,
) -> None:
    from test_characterization import _bed  # noqa: PLC0415

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
        "scenario": {
            "engine": {"use_default_embedder": False},
            "query": {"call": "Engine.search_text_only", "limit": 10},
        },
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
    outcome = run_and_write_characterization_performance(
        workload=WorkloadRef(
            parent_run_id="quality-run",
            evidence_family_id="quality-run",
            config_sha256="a" * 64,
            candidate_sha="b" * 40,
            query_call="Engine.search_text_only",
            effective_knobs={"limit": 10},
        ),
        plan=PerformancePlan(repetitions=1, treatments=("fresh_store", "warm")),
        scenario=scenario,
        config_doc=config,
        experiments_root=tmp_path / "experiments",
        experiment="earp-characterization-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )

    artifact = json.loads((outcome.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    assert artifact["workload"]["parent_run_id"] == "quality-run"
    assert len(artifact["samples"]) == 2
    assert not any(path.name == "earp.result.v1.json" for path in outcome.run_dir.iterdir())
