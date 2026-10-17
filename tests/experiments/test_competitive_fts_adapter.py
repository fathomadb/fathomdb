"""Real database smoke for the reusable FathomDB FTS competitive adapter."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

import pytest

from experiments import competitive_runner


ROOT = Path(__file__).resolve().parents[2]
ADAPTER = ROOT / "experiments/adapters/fathomdb_fts.py"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_real_fts_smoke_writes_measurement_artifact_and_receipt(tmp_path, monkeypatch):
    cli = os.environ.get("FATHOMDB_COMPETITIVE_CLI")
    if not cli:
        pytest.skip("set FATHOMDB_COMPETITIVE_CLI to run the real database smoke")
    binary = Path(cli).resolve(strict=True)
    corpus = tmp_path / "corpus.json"
    queries = tmp_path / "queries.json"
    corpus.write_text(
        json.dumps(
            [
                {"id": "doc-1", "body": "copper maple alpha"},
                {"id": "doc-2", "body": "silver cedar beta"},
                {"id": "doc-3", "body": "bronze birch gamma"},
            ]
        ),
        encoding="utf-8",
    )
    queries.write_text(json.dumps(["alpha", "beta", "gamma"]), encoding="utf-8")
    definition = {
        "schema_version": "competitive.run.v1",
        "case_id": "fathomdb-fts-smoke",
        "scoreboard": "latency_capacity",
        "product": {"name": "FathomDB", "version": "0.8.27", "deployment": "embedded"},
        "evidence": {"kind": "local", "reference_url": None},
        "workload": {
            "name": "synthetic-fts-smoke",
            "corpus_sha256": _digest(corpus),
            "queries_sha256": _digest(queries),
            "items": 3,
            "queries": 3,
            "search_mode": "fts",
            "top_k": 10,
            "filter": "none",
        },
        "measurement": {
            "boundary": "Python Engine.search through materialized hits",
            "cache_state": "steady",
            "concurrency": 1,
            "repetitions": 2,
            "seed": 7,
        },
        "target_hardware": {
            "cpu": "local smoke host",
            "cores": 1,
            "ram_gib": 1,
            "storage": "local filesystem",
            "gpu": "none",
        },
        "settings": {
            "cli_path": str(binary),
            "cli_sha256": _digest(binary),
            "embed_device": "cpu",
            "rerank_device": "cpu",
        },
    }
    plan = {
        "schema_version": "competitive.experiment.v1",
        "program_track": None,
        "run": definition,
        "adapter": {
            "protocol": "competitive.adapter.v1",
            "id": "fathomdb-fts",
            "path": str(ADAPTER),
            "sha256": _digest(ADAPTER),
            "timeout_s": 60,
        },
        "inputs": {
            "corpus": {"path": str(corpus), "sha256": _digest(corpus)},
            "queries": {"path": str(queries), "sha256": _digest(queries)},
        },
        "output": {
            "schema_version": "competitive.result.v1",
            "metrics": [
                "p50_ms",
                "p95_ms",
                "p99_ms",
                "queries_per_s",
                "ingest_items_per_s",
                "ready_to_search_ms",
                "peak_rss_mib",
                "storage_mib",
            ],
        },
    }
    plan_path = tmp_path / "experiment.json"
    plan_path.write_text(json.dumps(plan), encoding="utf-8")
    monkeypatch.setenv("PYTHONPATH", str(ROOT))
    _, receipt_dir = competitive_runner.run_experiment(
        plan_path, tmp_path / "output", tmp_path / "receipts"
    )
    receipt = json.loads((receipt_dir / "record.json").read_text())
    result = receipt["metrics"]["competitive_result"]
    assert result["status"] == "complete"
    assert result["observed"]["queries_completed"] == 6
    assert result["observed"]["errors"] == 0
    assert set(result["metrics"]) == set(plan["output"]["metrics"])
    summary = json.loads((tmp_path / "output" / "summary.json").read_text())
    assert len(summary["latency_ms"]) == 6
    assert summary["hit_counts"] == [1, 1, 1, 1, 1, 1]
    assert (tmp_path / "output" / "fathomdb-cell" / "fathomdb-doctor.v1.json").is_file()
