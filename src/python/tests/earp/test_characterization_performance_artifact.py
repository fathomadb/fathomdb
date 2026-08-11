"""A corpus-scale performance bridge publishes an independent sidecar."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.performance.earp_adapter import (
    PERFORMANCE_RESULT_NAME,
    PerformancePlan,
    load_earp_workload,
    run_and_write_characterization_performance,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_characterization_performance_writes_only_the_linked_artifact(
    tmp_path: Path,
) -> None:
    from test_characterization import _bed  # noqa: PLC0415
    from eval.earp.characterize import run_characterization  # noqa: PLC0415
    from eval.earp.config import ResolvedScenario  # noqa: PLC0415
    from eval.earp.schema.models import CampaignKind, RetrievalMode  # noqa: PLC0415

    bed = _bed(tmp_path)
    quality = run_characterization(**bed)
    assert quality.run_id is not None
    assert quality.run_dir is not None
    assert (quality.run_dir / "earp.workload-manifest.v1.json").is_file()
    workload = load_earp_workload(bed["experiments_root"], quality.run_id)
    config = quality.config_doc
    assert config is not None
    scenario = ResolvedScenario(
        campaign=CampaignKind.CHARACTERIZATION,
        config_sha256=workload.config_sha256,
        query_call=workload.query_call,
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
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        scenario=scenario,
        config_doc=config,
        experiments_root=bed["experiments_root"],
        experiment="earp-characterization-performance",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )

    artifact = json.loads((outcome.run_dir / PERFORMANCE_RESULT_NAME).read_text())
    assert artifact["workload"]["parent_run_id"] == quality.run_id
    assert len(artifact["samples"]) == 2
    assert not any(path.name == "earp.result.v1.json" for path in outcome.run_dir.iterdir())
