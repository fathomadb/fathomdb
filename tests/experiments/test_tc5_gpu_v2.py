"""Contract tests for the minimal GPU-primary TC-5 v2 runner."""

from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path

import pytest

from experiments import tc5_gpu_v2


CONFIG = Path("experiments/configs/scale-01/tc5-gpu-v2.json")


def test_committed_configuration_freezes_the_two_gpu_fidelity_arms():
    config = tc5_gpu_v2.load_config(CONFIG)

    assert config.arms == {"bridge": 7667, "primary": 17272}
    assert config.candidate_k == 192
    assert config.top_k == 10
    assert config.query_count == 100
    assert config.bootstrap_resamples == 1000
    assert config.cuda_uuid.startswith("GPU-")
    assert config.model_asset_directory.is_dir()


def test_dry_run_qualifies_inputs_without_creating_a_database(tmp_path, monkeypatch):
    config = tc5_gpu_v2.load_config(CONFIG)
    python = tmp_path / "python"
    fathomdb_bin = tmp_path / "fathomdb"
    model = tmp_path / "model"
    python.touch()
    fathomdb_bin.touch()
    model.mkdir()
    monkeypatch.setattr(
        tc5_gpu_v2,
        "load_config",
        lambda _path: replace(
            config,
            python=python,
            fathomdb_bin=fathomdb_bin,
            model_asset_directory=model,
        ),
    )
    monkeypatch.setattr(
        tc5_gpu_v2,
        "_load_arm_inputs",
        lambda _config, _arm: tc5_gpu_v2.ArmInputs(
            tuple({"document_id": str(index), "text": "x"} for index in range(7667)),
            tuple(
                {
                    "query_id": str(index),
                    "query": "q",
                    "exclude_document_id": "0",
                }
                for index in range(100)
            ),
            "1" * 64,
        ),
    )
    report = tc5_gpu_v2.dry_run(CONFIG, "bridge", output_root=tmp_path / "not-created")

    assert report["schema_version"] == "tc5-gpu-dry-run.v2"
    assert report["state"] == "ready"
    assert report["document_count"] == 7667
    assert report["query_count"] == 100
    assert report["new_database_required"] is True
    assert not (tmp_path / "not-created").exists()


def test_binary_environment_scrubs_fathomdb_controls_and_selects_one_gpu():
    environment = tc5_gpu_v2.binary_environment(
        {
            "PATH": "/bin",
            "FATHOMDB_EMBED_DEVICE": "cuda:1",
            "FATHOMDB_UNRELATED": "forbidden",
            "CUDA_VISIBLE_DEVICES": "1,2",
            "NVIDIA_VISIBLE_DEVICES": "all",
        },
        "GPU-pinned",
    )

    assert environment["CUDA_VISIBLE_DEVICES"] == "GPU-pinned"
    assert "NVIDIA_VISIBLE_DEVICES" not in environment
    assert not any(key.startswith("FATHOMDB_") for key in environment)


def test_query_job_binds_source_exclusion_and_all_fidelity_pins(tmp_path):
    config = tc5_gpu_v2.load_config(CONFIG)
    manifest, spec = tc5_gpu_v2.build_query_job(
        config,
        query={
            "query_id": "tc5-q-001",
            "query": "qualified query",
            "exclude_document_id": "document-001",
        },
        database_path=tmp_path / "fathomdb.sqlite",
        document_count=7667,
        fixture_digest="1" * 64,
        index_digest="2" * 64,
        binary_digest="3" * 64,
        manifest_path=tmp_path / "query.manifest.json",
    )

    assert manifest["exclude_logical_id"] == "document-001"
    assert manifest["expected_vector_rows"] == 7666
    assert manifest["selection_digest"] == tc5_gpu_v2.selection_digest(
        "doc", "document-001"
    )
    assert manifest["cuda_uuid"] == config.cuda_uuid
    assert spec["candidate_k"] == 192
    assert spec["top_k"] == 10
    assert spec["settings_digest"] == tc5_gpu_v2.settings_digest(spec, manifest)


def test_aggregate_emits_fidelity_and_uncertainty_without_payloads_or_latency_claims():
    config = tc5_gpu_v2.load_config(CONFIG)
    results = []
    for index in range(100):
        results.append(
            {
                "version": 1,
                "status": "measurement_complete",
                "candidate_k": 192,
                "top_k": 10,
                "embedding_device": "cuda:0",
                "cuda_uuid": config.cuda_uuid,
                "selected_vector_rows": 7666,
                "candidate_count": 192,
                "rerank_count": 10,
                "ground_truth_count": 10,
                "recall_at_top_k": 0.9 if index else 0.8,
                "candidate_execution": "cpu/sqlite-vec",
                "rerank_execution": "cpu/sqlite-vec",
                "rerank_ids_digest": f"{index:064x}",
                "ground_truth_ids_digest": f"{index + 100:064x}",
                "vector_stage_route_count": 1,
                "search_route_count": 0,
                "fts_route_count": 0,
                "fusion_route_count": 0,
                "graph_route_count": 0,
                "cross_encoder_route_count": 0,
                "model_asset_digest": config.model_asset_digest,
            }
        )

    receipt = tc5_gpu_v2.aggregate_results(
        config,
        arm="bridge",
        results=results,
        fixture_digest="1" * 64,
        index_digest="2" * 64,
        binary_digest="3" * 64,
    )

    assert receipt["schema_version"] == "tc5-gpu-arm-result.v2"
    assert receipt["query_completion_count"] == 100
    assert receipt["metrics"]["recall_at_10"] == pytest.approx(0.899)
    assert len(receipt["metrics"]["ci_95"]) == 2
    assert receipt["claim_boundary"] == "fidelity_and_uncertainty_only"
    assert receipt["provenance"]["embedding_execution"] == "cuda:0"
    assert receipt["provenance"]["exact_f32_rerank_execution"] == "cpu/sqlite-vec"
    assert receipt["provenance"]["cross_encoder_route"] == "disabled"
    encoded = json.dumps(receipt)
    assert '"query":' not in encoded
    assert "exclude_document_id" not in encoded
    assert "latency" not in encoded
    assert "elapsed" not in encoded


def test_aggregate_rejects_partial_or_wrong_device_results():
    config = tc5_gpu_v2.load_config(CONFIG)

    with pytest.raises(tc5_gpu_v2.Tc5GpuV2Error, match="100 complete"):
        tc5_gpu_v2.aggregate_results(
            config,
            arm="bridge",
            results=[],
            fixture_digest="1" * 64,
            index_digest="2" * 64,
            binary_digest="3" * 64,
        )
