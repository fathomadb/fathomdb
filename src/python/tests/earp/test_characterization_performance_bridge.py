"""Independent repeated performance for a resolved corpus-scale EARP workload."""

from __future__ import annotations

import pytest
from pathlib import Path

from eval.performance.earp_adapter import (
    PerformancePlan,
    load_earp_workload,
    run_characterization_repetitions,
)

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_bridge_repeats_the_same_characterization_workload(tmp_path: Path) -> None:
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
    samples = run_characterization_repetitions(
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        scenario=scenario,
        config_doc=config,
    )

    assert [(sample.treatment, sample.repetition) for sample in samples] == [
        ("fresh_store", 0),
        ("fresh_store_warm_query", 0),
    ]
    assert all({"open", "write", "query"} <= set(sample.phases_ms) for sample in samples)
    assert all(sample.treatment_witness["fresh_database"] is True for sample in samples)
    assert samples[1].treatment_witness["unmeasured_query_warmup"] is True
    assert samples[1].treatment_witness["open_write_scope"] == "fresh_store"


def test_bridge_preserves_raw_observation_when_characterization_cell_blocks(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """FIX-3: a blocked characterization cell remains typed matrix evidence."""
    from test_characterization import _bed  # noqa: PLC0415
    import eval.earp.characterize as characterize  # noqa: PLC0415
    from eval.earp.characterize import ArmExecution, run_characterization  # noqa: PLC0415
    from eval.earp.config import ResolvedScenario  # noqa: PLC0415
    from eval.earp.schema.models import (  # noqa: PLC0415
        Blocker,
        BlockerCode,
        CampaignKind,
        RetrievalMode,
    )

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

    def blocked_after_open(**_kwargs: object) -> ArmExecution:
        return ArmExecution(
            blocker=Blocker(
                code=BlockerCode.DENSE_READINESS_TIMEOUT,
                message="readiness timed out after open",
                stage="characterize.readiness",
                detail={},
            ),
            observed_cost={
                "phases_ms": {"open": 1.0, "write": 2.0},
                "counts": {"accepted": 1},
                "storage": {"database_bytes": 4},
            },
        )

    monkeypatch.setattr(characterize, "execute_arm", blocked_after_open)
    cells = run_characterization_repetitions(
        workload=workload,
        plan=PerformancePlan(
            repetitions=1, treatments=("fresh_store", "fresh_store_warm_query")
        ),
        scenario=scenario,
        config_doc=config,
    )
    assert cells[0].status == "invalid"
    assert cells[0].invalidity["code"] == "dense_readiness_timeout"
    assert cells[0].raw_samples[0].phases_ms["open"] == 1.0
    assert [cell.treatment for cell in cells] == ["fresh_store", "fresh_store_warm_query"]
