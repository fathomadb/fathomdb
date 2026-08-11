"""Observed-cost sidecar staging around EARP's shared writer."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from eval.earp.observed_cost import OBSERVED_COST_NAME, Observation
from eval.earp.schema.models import RunVerdict
from eval.earp.writer import write_run

TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)
SHA = "a" * 64


def test_observed_cost_is_staged_before_shared_record_and_index(tmp_path: Path) -> None:
    config = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {"query": {"call": "Engine.search_text_only"}},
    }
    observed = Observation(
        evidence_family_id="pending",
        config_sha256=SHA,
        phases_ms={"open": 1.0, "query": 2.0},
        counts={"queries": 1},
        storage={"database_bytes": 7, "wal_bytes": 0, "shm_bytes": 0},
    ).as_document()
    outcome = write_run(
        experiment="earp-diagnostic",
        ts=TS,
        config_doc=config,
        experiments_root=tmp_path / "experiments",
        verdict=RunVerdict.COMPLETE,
        read="diagnostic with observed cost",
        metrics={},
        code={"git_sha": "0" * 40, "dirty": False, "branch": "t", "baseline_commit": None},
        env={"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        corpus={"source": None, "manifest_sha256": None, "datasets": []},
        seeds={},
        cost_usd=0.0,
        observed_cost=observed,
    )

    assert outcome.run_dir is not None
    sidecar = json.loads((outcome.run_dir / OBSERVED_COST_NAME).read_text(encoding="utf-8"))
    assert sidecar["evidence_family_id"] == outcome.run_id
    record = json.loads((outcome.run_dir / "record.json").read_text(encoding="utf-8"))
    assert f"runs/{outcome.run_id}/{OBSERVED_COST_NAME}" in record["artifacts"]
