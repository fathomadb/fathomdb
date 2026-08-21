"""Tests for the FathomDB arm of the official LOCOMO harness."""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from experiments import _lib, fathomdb_locomo


TS = datetime(2026, 8, 13, 12, 30, tzinfo=timezone.utc)


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _config(tmp_path: Path) -> dict:
    dataset = tmp_path / "locomo.json"
    dataset.write_text("[]", encoding="utf-8")
    provenance = tmp_path / "locomo-provenance.json"
    provenance.write_text('{"schema_version":"locomo-provenance.v1","entries":[]}', encoding="utf-8")
    return {
        "schema_version": "fathomdb-locomo.v1",
        "campaign": "official_seam_predict_only",
        "program_track": "LOCOMO-01",
        "harness": {"checkout": str(tmp_path / "harness"), "python": "/bin/python", "git_sha": "abc123def456"},
        "corpus": {"dataset_path": str(dataset), "raw_sha256": _sha(dataset), "normalized_sha256": "0" * 64, "sessions": 1, "eligible_questions": 1},
        "benchmark": {"project_name": "fathomdb-locomo", "conversations": "0", "categories": "1", "top_k": 10, "top_k_cutoffs": [10], "max_workers": 1, "rpm": 1, "predict_only": True, "resume": True},
        "facade": {
            "python": "/bin/python", "host": "127.0.0.1", "port": 8889,
            "provenance_manifest": str(provenance), "provenance_manifest_sha256": _sha(provenance),
        },
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


def test_fathomdb_arm_rejects_a_missing_or_tampered_provenance_manifest(tmp_path):
    config = _config(tmp_path)
    config["facade"]["provenance_manifest_sha256"] = "0" * 64

    with pytest.raises(ValueError, match="provenance manifest"):
        fathomdb_locomo.resolve_config(config)


def test_fathomdb_arm_requires_its_track_runner_identifier(tmp_path):
    config = _config(tmp_path)
    config["program_track"] = "MEMORY-01"

    with pytest.raises(ValueError, match="program_track"):
        fathomdb_locomo.resolve_config(config)


def test_capture_facade_sidecar_writes_only_the_loopback_json_payload(tmp_path, monkeypatch):
    class _Response:
        status = 200

        def read(self):
            return b'{"schema_version":"locomo-facade-provenance.v1","requests":{}}'

        def __enter__(self):
            return self

        def __exit__(self, *_args):
            return False

    monkeypatch.setattr(fathomdb_locomo, "urlopen", lambda *_args, **_kwargs: _Response())

    payload = fathomdb_locomo._capture_facade_sidecar("127.0.0.1", 8889, "provenance", tmp_path)

    assert payload["requests"] == {}
    assert json.loads((tmp_path / "facade-provenance.v1.json").read_text()) == payload


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
    assert record["config"]["resolved"]["program_track"] == "LOCOMO-01"
    serialized = json.dumps(record)
    assert str(raw) not in serialized
    assert str(config["harness"]["checkout"]) not in serialized
    assert record["artifacts"][1]["path"] == "external-artifacts.manifest.v1"


def test_run_allocates_raw_output_under_its_public_receipt_id(tmp_path, monkeypatch):
    class _FrozenDateTime:
        @classmethod
        def now(cls, _tz):
            return TS

    config = _config(tmp_path)
    monkeypatch.setattr(fathomdb_locomo, "datetime", _FrozenDateTime)

    run_id, _run_dir, returncode = fathomdb_locomo.run(
        config, base_dir=tmp_path / "experiments"
    )

    assert returncode == 2
    assert (Path(config["output"]["external_root"]) / run_id).is_dir()


def test_committed_receipts_use_only_logical_artifact_names():
    for receipt_path in sorted((_lib.EXPERIMENTS_DIR / "runs").glob("*/record.json")):
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        for artifact in receipt["artifacts"]:
            assert not Path(artifact["path"]).is_absolute(), receipt_path
