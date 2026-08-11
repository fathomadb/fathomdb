"""Real EARP artifacts expose resolved knobs to the independent runner."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.config import resolve_config
from eval.earp.runner import run_diagnostic
from eval.performance.earp_adapter import load_earp_workload

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")

TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)


def test_real_earp_run_exports_effective_knobs_and_observed_cost(tmp_path: Path) -> None:
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {
                "kind": "doc",
                "body": "the deal sheet is missing for March",
                "source_id": "source",
                "logical_id": "doc-a",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    config = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {
            "fixture": str(fixture),
            "query": {
                "call": "Engine.search",
                "text": "deal sheet",
                "alpha": 0.7,
                "rerank_depth": 20,
                "limit": 10,
            },
        },
    }
    resolution = resolve_config(config)
    assert resolution.scenario is not None, resolution.blockers
    result = run_diagnostic(
        scenario=resolution.scenario,
        config_doc=config,
        experiments_root=tmp_path / "experiments",
        experiment="earp-diagnostic",
        ts=TS,
    )
    assert result.run_id is not None
    assert result.run_dir is not None

    workload = load_earp_workload(tmp_path / "experiments", result.run_id)
    assert workload.effective_knobs == {"alpha": 0.7, "rerank_depth": 20, "limit": 10}
    observed = json.loads((result.run_dir / "earp.observed-cost.v2.json").read_text())
    assert observed["evidence_family_id"] == result.run_id
    assert {"open", "write", "query"} <= set(observed["phases_ms"])
    assert observed["query_samples"][0]["outcome"] == "complete"
    assert observed["unavailable"]["engine_trace"]["code"] == "not_exposed"
    assert observed["provenance"]["candidate_sha"]
