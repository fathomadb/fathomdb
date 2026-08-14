"""Tests for the FathomDB arm of the official LOCOMO harness."""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

from experiments import fathomdb_locomo


TS = datetime(2026, 8, 13, 12, 30, tzinfo=timezone.utc)


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _config(tmp_path: Path) -> dict:
    dataset = tmp_path / "locomo.json"
    dataset.write_text("[]", encoding="utf-8")
    return {
        "schema_version": "fathomdb-locomo.v1",
        "campaign": "official_seam_predict_only",
        "harness": {"checkout": str(tmp_path / "harness"), "python": "/bin/python", "git_sha": "abc123def456"},
        "corpus": {"dataset_path": str(dataset), "raw_sha256": _sha(dataset), "normalized_sha256": "0" * 64, "sessions": 1, "eligible_questions": 1},
        "benchmark": {"project_name": "fathomdb-locomo", "conversations": "0", "categories": "1", "top_k": 10, "top_k_cutoffs": [10], "max_workers": 1, "rpm": 1, "predict_only": True, "resume": True},
        "facade": {"python": "/bin/python", "host": "127.0.0.1", "port": 8889},
        "output": {"external_root": str(tmp_path / "external")},
    }


def test_fathomdb_arm_command_uses_the_official_runner_and_local_facade(tmp_path):
    config = fathomdb_locomo.resolve_config(_config(tmp_path))
    command = fathomdb_locomo.build_harness_command(config, run_id="r", raw_dir=tmp_path / "raw")

    assert command[:3] == ["/bin/python", "-m", "benchmarks.locomo.run"]
    assert command[command.index("--mem0-host") + 1] == "http://127.0.0.1:8889"
    assert command[command.index("--top-k") + 1] == "10"


def test_predict_only_harness_environment_replaces_any_direct_openai_key(monkeypatch):
    monkeypatch.setenv("OPENAI_API_KEY", "sk-direct-key-must-not-reach-harness")

    environment = fathomdb_locomo.predict_only_harness_env()

    assert environment["OPENAI_API_KEY"] == "predict-only-placeholder"


def test_fathomdb_arm_receipt_has_a_typed_safe_result_sidecar(tmp_path):
    config = fathomdb_locomo.resolve_config(_config(tmp_path))
    raw = tmp_path / "external" / "raw"
    raw.mkdir(parents=True)
    run_id, run_dir = fathomdb_locomo.write_receipt(
        config, ts=TS, base_dir=tmp_path / "experiments", raw_dir=raw,
        verdict="incomplete", read="missing output", completion={"complete": False, "expected_questions": 1},
    )

    sidecar = json.loads((run_dir / "fathomdb-locomo-arm.result.v1.json").read_text())
    assert sidecar["schema_version"] == "fathomdb-locomo-arm.result.v1"
    assert sidecar["run_id"] == run_id
    record = json.loads((run_dir / "record.json").read_text())
    assert record["experiment"] == "fathomdb-locomo-official-seam"
