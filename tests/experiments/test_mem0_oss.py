"""Tests for the native Mem0 OSS experiment receipt adapter."""

from __future__ import annotations

import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from experiments import mem0_oss  # noqa: E402


UTC = timezone.utc
TS = datetime(2026, 8, 13, 12, 30, 0, tzinfo=UTC)


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _config(tmp_path: Path) -> dict:
    dataset = tmp_path / "locomo10.json"
    dataset.write_text("[]\n", encoding="utf-8")
    override = tmp_path / "compose.override.yaml"
    override.write_text("services: {}\n", encoding="utf-8")
    mem0_config = tmp_path / "mem0-airlock.yaml"
    mem0_config.write_text("version: v1.1\n", encoding="utf-8")
    base_compose = tmp_path / "docker-compose.yml"
    base_compose.write_text("services: {}\n", encoding="utf-8")
    return {
        "schema_version": "mem0-oss.v1",
        "campaign": "native_locomo_predict",
        "program_track": "MEMORY-01",
        "harness": {
            "checkout": str(tmp_path / "memory-benchmarks"),
            "python": "/tmp/fathomdb-mem0-memory-benchmarks/.venv/bin/python",
            "git_sha": "4b61c5d31b9c668a12b4f5e78064248a02c82d2b",
        },
        "corpus": {
            "dataset_path": str(dataset),
            "raw_sha256": _sha(dataset),
            "normalized_sha256": "e9999b551ac67e899f0008c9ec446cecce937ce12f01f2f08cefa9f690fc4c7c",
            "sessions": 272,
            "eligible_questions": 1443,
        },
        "mem0": {
            "host": "http://127.0.0.1:8888",
            "compose_project": "fathomdb-mem0-locomo",
            "llm_model": "gpt-4o-mini",
            "embedder_model": "text-embedding-3-small",
        },
        "airlock": {
            "base_url": "http://host.docker.internal:4000/v1",
            "host_gateway": True,
            "llm_alias": "gpt-4o-mini",
            "embedder_alias": "text-embedding-3-small",
            "redaction_smoke": "required",
        },
        "benchmark": {
            "project_name": "fathomdb-mem0-locomo-native",
            "conversations": "0",
            "categories": "1,2,3,4",
            "top_k": 10,
            "top_k_cutoffs": [10],
            "max_workers": 10,
            "rpm": 200,
            "predict_only": True,
            "resume": True,
        },
        "output": {"external_root": str(tmp_path / "mem0-output")},
        "compose": {"base_file": str(base_compose)},
        "provenance_artifacts": [
            {"name": "compose_override", "path": str(override), "sha256": _sha(override)},
            {"name": "mem0_airlock_config", "path": str(mem0_config), "sha256": _sha(mem0_config)},
        ],
    }


def test_resolve_config_rejects_secrets_and_accepts_local_loopback_airlock(tmp_path):
    config = _config(tmp_path)
    config["mem0"]["api_key"] = "sk-direct-openai-key"
    with pytest.raises(ValueError, match="secrets"):
        mem0_oss.resolve_config(config)

    config = _config(tmp_path)
    config["airlock"]["base_url"] = "http://127.0.0.1:4000/v1"
    config["airlock"]["host_gateway"] = False
    assert mem0_oss.resolve_config(config)["airlock"]["base_url"] == "http://127.0.0.1:4000/v1"


def test_resolve_config_requires_predict_only_resume_and_hashed_artifacts(tmp_path):
    config = _config(tmp_path)
    config["benchmark"]["predict_only"] = False
    with pytest.raises(ValueError, match="predict_only"):
        mem0_oss.resolve_config(config)

    config = _config(tmp_path)
    config["provenance_artifacts"][0]["sha256"] = "0" * 64
    with pytest.raises(ValueError, match="sha256 mismatch"):
        mem0_oss.resolve_config(config)

    config = _config(tmp_path)
    config["benchmark"]["top_k"] = 11
    with pytest.raises(ValueError, match="top_k"):
        mem0_oss.resolve_config(config)


def test_native_mem0_arm_requires_its_track_runner_identifier(tmp_path):
    config = _config(tmp_path)
    config["program_track"] = "LOCOMO-01"

    with pytest.raises(ValueError, match="program_track"):
        mem0_oss.resolve_config(config)


def test_build_command_is_native_oss_predict_only_and_credential_free(tmp_path):
    config = mem0_oss.resolve_config(_config(tmp_path))
    command = mem0_oss.build_harness_command(config, run_id="native-run", raw_dir=tmp_path / "raw")

    assert command[:3] == [config["harness"]["python"], "-m", "benchmarks.locomo.run"]
    assert "--backend" in command and command[command.index("--backend") + 1] == "oss"
    assert "--predict-only" in command
    assert "--resume" in command
    assert command[command.index("--mem0-host") + 1] == "http://127.0.0.1:8888"
    assert command[command.index("--dataset-path") + 1] == config["corpus"]["dataset_path"]
    assert all("key" not in item.lower() and "secret" not in item.lower() for item in command)


def test_compose_up_starts_only_the_campaign_mem0_and_qdrant_services(tmp_path):
    config = mem0_oss.resolve_config(_config(tmp_path))
    command = mem0_oss.compose_command(config, "up")

    assert command[-4:] == ["up", "-d", "mem0", "qdrant"]
    assert command[command.index("-p") + 1] == "fathomdb-mem0-locomo"


def test_write_receipt_uses_generic_run_layout_and_typed_unavailable_cost(tmp_path):
    config = mem0_oss.resolve_config(_config(tmp_path))
    raw = tmp_path / "raw"
    raw.mkdir()
    native = raw / "predicted_fathomdb-mem0-locomo-native"
    native.mkdir()
    output = native / "conv0_q0.json"
    output.write_text(
        json.dumps({"question_id": "conv0_q0", "retrieval": {"search_results": [], "search_latency_ms": 3.0, "total_results": 0}}),
        encoding="utf-8",
    )
    (native / "_ingestion_0.json").write_text(
        json.dumps({"conversation_idx": 0, "total_chunks_failed": 0}), encoding="utf-8"
    )

    run_id, run_dir = mem0_oss.write_receipt(
        config,
        ts=TS,
        base_dir=tmp_path / "experiments",
        code={"git_sha": "abc", "dirty": False, "branch": "benchmark", "baseline_commit": None},
        env={"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        verdict="complete",
        read="native Mem0 LOCOMO predict-only completed",
        raw_dir=raw,
    )

    assert run_dir == tmp_path / "experiments" / "runs" / run_id
    assert {path.name for path in run_dir.iterdir()} >= {"record.json", "config.resolved.yaml", "metrics.json"}
    assert not (run_dir / "raw").exists()
    assert not (run_dir / "provenance").exists()
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    assert record["experiment"] == "mem0-oss-locomo-native"
    assert record["schema_version"] == "experiments.record.v1"
    assert record["config"]["resolved"]["program_track"] == "MEMORY-01"
    assert record["cost_usd"] is None
    assert record["metrics"]["cost"]["status"] == "unavailable"
    assert str(output) not in (run_dir / "record.json").read_text(encoding="utf-8")
    index_row = json.loads((tmp_path / "experiments" / "index.jsonl").read_text(encoding="utf-8"))
    assert index_row["run_id"] == run_id
    assert run_id in (tmp_path / "experiments" / "INDEX.md").read_text(encoding="utf-8")


def test_external_artifact_manifest_has_only_safe_aggregate_fields(tmp_path):
    raw = tmp_path / "raw"
    raw.mkdir()
    (raw / "stdout.log").write_text("question: secret corpus text", encoding="utf-8")
    manifest = mem0_oss.external_artifact_manifest(raw)

    assert manifest["schema_version"] == "mem0-oss.external-artifacts.v1"
    assert manifest["file_count"] == 1
    assert "question" not in json.dumps(manifest)
    assert "stdout.log" not in json.dumps(manifest)


def test_preflight_rejects_literal_overlay_credentials(tmp_path):
    config = _config(tmp_path)
    Path(config["provenance_artifacts"][0]["path"]).write_text(
        "host.docker.internal:host-gateway\nOPENAI_API_KEY: ${AIRLOCK_VIRTUAL_KEY}\n:/app/config.yaml:ro\n",
        encoding="utf-8",
    )
    mem0_config = Path(config["provenance_artifacts"][1]["path"])
    mem0_config.write_text(
        "gpt-4o-mini\ntext-embedding-3-small\nhttp://host.docker.internal:4000/v1\napi_key: direct-value\n",
        encoding="utf-8",
    )
    config["provenance_artifacts"][0]["sha256"] = _sha(Path(config["provenance_artifacts"][0]["path"]))
    config["provenance_artifacts"][1]["sha256"] = _sha(mem0_config)

    assert "overlay contains a literal credential" in mem0_oss.preflight(config)


def test_predict_completion_is_incomplete_when_selected_question_output_is_missing(tmp_path):
    config = _config(tmp_path)
    dataset = tmp_path / "selected-locomo10.json"
    dataset.write_text(json.dumps([{"qa": [{"category": 1}, {"category": 2}]}]), encoding="utf-8")
    config["corpus"]["dataset_path"] = str(dataset)
    config["corpus"]["raw_sha256"] = _sha(dataset)
    config["benchmark"]["conversations"] = "0"
    config["benchmark"]["categories"] = "1,2"
    config = mem0_oss.resolve_config(config)
    native = tmp_path / "raw" / "predicted_fathomdb-mem0-locomo-native"
    native.mkdir(parents=True)
    (native / "conv0_q0.json").write_text(json.dumps({"question_id": "conv0_q0", "retrieval": {"search_results": [], "search_latency_ms": 1.0, "total_results": 0}}), encoding="utf-8")
    (native / "_ingestion_0.json").write_text(
        json.dumps({"conversation_idx": 0, "total_chunks_failed": 0}), encoding="utf-8"
    )

    completion = mem0_oss.predict_completion(config, native.parent)

    assert completion["complete"] is False
    assert completion["missing_question_ids"] == ["conv0_q1"]


def test_predict_completion_reads_native_ingestion_checkpoints_from_prediction_dir(tmp_path):
    config = _config(tmp_path)
    dataset = tmp_path / "selected-locomo10.json"
    dataset.write_text(json.dumps([{"qa": [{"category": 1}]}]), encoding="utf-8")
    config["corpus"]["dataset_path"] = str(dataset)
    config["corpus"]["raw_sha256"] = _sha(dataset)
    config["benchmark"]["conversations"] = "0"
    config["benchmark"]["categories"] = "1"
    config = mem0_oss.resolve_config(config)
    native = tmp_path / "raw" / "predicted_fathomdb-mem0-locomo-native"
    native.mkdir(parents=True)
    (native / "conv0_q0.json").write_text(
        json.dumps({"question_id": "conv0_q0", "retrieval": {"search_results": [], "search_latency_ms": 1.0, "total_results": 0}}),
        encoding="utf-8",
    )
    (native / "_ingestion_0.json").write_text(
        json.dumps({"conversation_idx": 0, "total_chunks_failed": 0}), encoding="utf-8"
    )

    assert mem0_oss.predict_completion(config, native.parent)["complete"] is True


def test_write_receipt_rejects_raw_output_inside_experiment_runs(tmp_path):
    config = mem0_oss.resolve_config(_config(tmp_path))
    raw = tmp_path / "experiments" / "runs" / "unsafe"
    raw.mkdir(parents=True)

    with pytest.raises(ValueError, match="outside experiments/runs"):
        mem0_oss.write_receipt(
            config,
            ts=TS,
            base_dir=tmp_path / "experiments",
            code={"git_sha": "abc", "dirty": False, "branch": "benchmark", "baseline_commit": None},
            env={"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
            verdict="incomplete",
            read="unsafe",
            raw_dir=raw,
        )


def test_run_closes_a_typed_blocked_receipt_when_harness_is_missing(tmp_path):
    config = mem0_oss.resolve_config(_config(tmp_path))

    _, run_dir, returncode = mem0_oss.run(config, base_dir=tmp_path / "experiments")

    assert returncode == 2
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    assert record["verdict"] == "blocked_prerequisite"
    assert "harness.checkout" in record["read"]


def test_run_closes_a_typed_blocked_receipt_when_harness_is_not_a_git_checkout(tmp_path):
    config = _config(tmp_path)
    config["harness"]["checkout"] = str(tmp_path)
    config = mem0_oss.resolve_config(config)

    _, run_dir, returncode = mem0_oss.run(config, base_dir=tmp_path / "experiments")

    assert returncode == 2
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    assert record["verdict"] == "blocked_prerequisite"
    assert "cannot inspect harness cleanliness" in record["read"]
