"""Human-intended tests for the non-executing SCALE-01 TC-5 runner contract."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments.tc5_characterization import (
    TC5_EXECUTION_CONFIG_V1,
    TC5_EXECUTION_RECEIPT_V1,
    Tc5CharacterizationError,
    load_execution_config,
    prepare_characterization,
    run_characterization,
    write_execution_receipt,
)
from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
)


def _sha(number: int) -> str:
    return f"{number:064x}"


def _manifest() -> dict[str, object]:
    documents = [
        {
            "document_id": f"document-{number:05d}",
            "content_sha256": _sha(number),
            "origin": "real",
        }
        for number in range(PRIMARY_DOCUMENT_COUNT)
    ]
    return {
        "schema_version": "tc5-manifest.v1",
        "program_track": "SCALE-01",
        "manifest_id": "eu7-tc5-all-real-18472",
        "source_artifact_sha256": "a" * 64,
        "documents": documents,
        "bridge_document_ids": [row["document_id"] for row in documents[:BRIDGE_DOCUMENT_COUNT]],
        "provenance": {
            "source_commit": "b" * 40,
            "cargo_lock_sha256": "c" * 64,
            "rust_version": "1.90.0",
            "cpu_identity": "cpu-identity-pending-external-run",
            "os_identity": "os-identity-pending-external-run",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "model_asset_sha256": "d" * 64,
            "engine_features": ["default-embedder", "operator"],
            "embed_device": "cpu",
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
    }


def _config() -> dict[str, object]:
    return {
        "schema_version": TC5_EXECUTION_CONFIG_V1,
        "program_track": "SCALE-01",
        "configuration_id": "tc5-cpu-all-real-v1",
        "release_state": "awaiting_independent_review_and_coordinator_release",
        "execution_enabled": False,
        "arms": [
            {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
            {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
        ],
        "frozen_configuration": {
            "embed_device": "cpu",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
        "artifact_contract": {
            "external_manifest_required": True,
            "external_corpus_root_required": True,
            "external_output_root_required": True,
            "historical_eu7_output_forbidden": True,
            "repository_artifacts_forbidden": True,
            "experiment_index_receipt_required_after_both_arms": True,
        },
        "claim_boundary": {
            "scale_02_capacity_claim": False,
            "latency_or_slo_claim": False,
            "only_fidelity_and_uncertainty_after_complete_arms": True,
        },
    }


def _write_json(path: Path, document: dict[str, object]) -> Path:
    path.write_text(json.dumps(document), encoding="utf-8")
    return path


def test_committed_configuration_is_the_disabled_frozen_cpu_contract():
    config_path = Path(__file__).resolve().parents[2] / "experiments/configs/scale-01/tc5-execution.v1.json"

    config = load_execution_config(config_path)

    assert config.configuration_id == "tc5-cpu-all-real-v1"
    assert config.execution_enabled is False
    assert list(config.arms) == _config()["arms"]


def test_preparation_freezes_cpu_two_arm_receipt_without_external_paths_or_measurement(tmp_path):
    manifest_path = _write_json(tmp_path / "manifest.json", _manifest())
    config_path = _write_json(tmp_path / "tc5-execution.json", _config())
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()

    prepared = prepare_characterization(
        config_path=config_path,
        manifest_path=manifest_path,
        corpus_root=corpus_root,
        output_root=output_root,
    )

    assert prepared.config.configuration_id == "tc5-cpu-all-real-v1"
    assert prepared.manifest.document_count == PRIMARY_DOCUMENT_COUNT
    assert prepared.receipt["schema_version"] == TC5_EXECUTION_RECEIPT_V1
    assert prepared.receipt["execution"] == {
        "status": "awaiting_independent_review_and_coordinator_release",
        "smoke_performed": False,
        "measurement_performed": False,
        "complete_arm_results_required": ["bridge", "primary"],
    }
    assert prepared.receipt["claim_boundary"] == _config()["claim_boundary"]
    serialized = json.dumps(prepared.receipt)
    assert str(corpus_root) not in serialized
    assert str(output_root) not in serialized
    assert "document-00000" not in serialized


def test_default_config_refuses_execution_before_any_live_side_effect(tmp_path):
    manifest_path = _write_json(tmp_path / "manifest.json", _manifest())
    config_path = _write_json(tmp_path / "tc5-execution.json", _config())
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()

    with pytest.raises(Tc5CharacterizationError, match="not released for execution"):
        run_characterization(
            config_path=config_path,
            manifest_path=manifest_path,
            corpus_root=corpus_root,
            output_root=output_root,
        )

    assert list(output_root.iterdir()) == []


@pytest.mark.parametrize(
    ("mutate", "message"),
    [
        (lambda value: value.__setitem__("execution_enabled", True), "execution_enabled"),
        (lambda value: value.__setitem__("release_state", "released"), "release_state"),
        (lambda value: value["arms"].pop(), "arms"),
        (lambda value: value["frozen_configuration"].__setitem__("embed_device", "cuda"), "embed_device"),
        (lambda value: value["claim_boundary"].__setitem__("scale_02_capacity_claim", True), "claim_boundary"),
    ],
)
def test_execution_config_fails_closed_when_a_frozen_or_claim_boundary_drifts(tmp_path, mutate, message):
    config = _config()
    mutate(config)
    config_path = _write_json(tmp_path / "tc5-execution.json", config)

    with pytest.raises(Tc5CharacterizationError, match=message):
        load_execution_config(config_path)


def test_preparation_rejects_repository_manifest_and_historical_or_repository_receipt_paths(tmp_path):
    config_path = _write_json(tmp_path / "tc5-execution.json", _config())
    manifest_path = _write_json(tmp_path / "manifest.json", _manifest())
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()

    with pytest.raises(Tc5CharacterizationError, match="manifest path must remain outside the repository"):
        prepare_characterization(
            config_path=config_path,
            manifest_path=Path(__file__).resolve(),
            corpus_root=corpus_root,
            output_root=output_root,
        )

    prepared = prepare_characterization(
        config_path=config_path,
        manifest_path=manifest_path,
        corpus_root=corpus_root,
        output_root=output_root,
    )
    with pytest.raises(Tc5CharacterizationError, match="historical eu7 output"):
        write_execution_receipt(
            prepared,
            receipt_path=Path(__file__).resolve().parents[2]
            / "dev/plans/runs/eu7-latest-measurements.json",
        )
    with pytest.raises(Tc5CharacterizationError, match="declared external output root"):
        write_execution_receipt(prepared, receipt_path=tmp_path / "outside-output.json")


def test_receipt_can_only_be_written_under_declared_external_output_root_after_preparation(tmp_path):
    manifest_path = _write_json(tmp_path / "manifest.json", _manifest())
    config_path = _write_json(tmp_path / "tc5-execution.json", _config())
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()
    prepared = prepare_characterization(
        config_path=config_path,
        manifest_path=manifest_path,
        corpus_root=corpus_root,
        output_root=output_root,
    )

    receipt_path = write_execution_receipt(prepared, receipt_path=output_root / "tc5-receipt.json")

    assert receipt_path == output_root / "tc5-receipt.json"
    assert json.loads(receipt_path.read_text(encoding="utf-8"))["program_track"] == "SCALE-01"
