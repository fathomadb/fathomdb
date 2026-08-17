"""Human-intended tests for content-free SCALE-01 TC-5 input qualification."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import corpus_matrix
from experiments import tc5_input_qualification as qualification
from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
)


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = ROOT / "experiments/configs/corpus-01/corpus-matrix.v1.json"
POLICY_PATH = ROOT / "experiments/configs/scale-01/tc5-input-qualification.v1.json"


def _sha(number: int) -> str:
    return f"{number:064x}"


def _manifest() -> dict[str, object]:
    documents = [
        {"document_id": f"document-{number:05d}", "content_sha256": _sha(number), "origin": "real"}
        for number in range(PRIMARY_DOCUMENT_COUNT)
    ]
    return {
        "schema_version": "tc5-manifest.v1",
        "program_track": "SCALE-01",
        "manifest_id": "tc5-qualified-all-real-18472",
        "source_artifact_sha256": "a" * 64,
        "documents": documents,
        "bridge_document_ids": [row["document_id"] for row in documents[:BRIDGE_DOCUMENT_COUNT]],
        "provenance": {
            "source_commit": "b" * 40,
            "cargo_lock_sha256": "c" * 64,
            "rust_version": "1.90.0",
            "cpu_identity": "qualified-cpu-host",
            "os_identity": "qualified-os",
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


def _inventory(manifest: dict[str, object], matrix: dict[str, object]) -> dict[str, object]:
    manifest_sha = hashlib.sha256(
        json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return {
        "schema_version": "tc5-input-inventory.v1",
        "program_track": "SCALE-01",
        "inventory_id": "tc5-ir-c-qualified-inputs-001",
        "corpus_id": "ir_c",
        "corpus_matrix_sha256": corpus_matrix.canonical_sha256(matrix),
        "license_copy_sha256": "e" * 64,
        "source_revision": "ir-c-qualified-source-revision",
        "source_artifact_sha256": manifest["source_artifact_sha256"],
        "manifest_sha256": manifest_sha,
        "cpu_host_attestation_sha256": "f" * 64,
        "model_asset_sha256": manifest["provenance"]["model_asset_sha256"],
        "model_cache_attested": True,
        "ground_truth_artifact_sha256": "1" * 64,
        "vector_stage_runtime_sha256": "2" * 64,
        "output_root_attestation_sha256": "3" * 64,
    }


def _matrix() -> dict[str, object]:
    return json.loads(MATRIX_PATH.read_text(encoding="utf-8"))


def test_qualification_binds_exact_all_real_manifest_and_projects_no_document_ids():
    matrix = _matrix()
    manifest = _manifest()

    result = qualification.qualify_input_inventory(
        _inventory(manifest, matrix), manifest, matrix, qualification.load_policy(POLICY_PATH)
    )

    assert result.state == "factual_inputs_qualified"
    report = result.safe_report()
    assert report["arms"] == [
        {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
        {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
    ]
    assert report["claim_boundary"] == "fidelity_only_no_scale02_or_product_claim"
    serialized = json.dumps(report)
    assert "document-00000" not in serialized
    assert "qualified-cpu-host" not in serialized


def test_qualification_rejects_a_manifest_digest_that_is_not_the_exact_safe_manifest():
    matrix = _matrix()
    manifest = _manifest()
    inventory = _inventory(manifest, matrix)
    inventory["manifest_sha256"] = "0" * 64

    with pytest.raises(qualification.Tc5InputQualificationError, match="manifest_sha256"):
        qualification.qualify_input_inventory(
            inventory, manifest, matrix, qualification.load_policy(POLICY_PATH)
        )


def test_qualification_respects_the_corpus_matrix_claim_boundary():
    matrix = _matrix()
    manifest = _manifest()
    inventory = _inventory(manifest, matrix)
    inventory["corpus_id"] = "locomo"

    with pytest.raises(qualification.Tc5InputQualificationError, match="retrieval_fidelity_only"):
        qualification.qualify_input_inventory(
            inventory, manifest, matrix, qualification.load_policy(POLICY_PATH)
        )


def test_qualification_requires_host_model_ground_truth_runtime_and_output_attestations():
    matrix = _matrix()
    manifest = _manifest()
    inventory = _inventory(manifest, matrix)
    inventory["model_cache_attested"] = False

    with pytest.raises(qualification.Tc5InputQualificationError, match="model_cache_attested"):
        qualification.qualify_input_inventory(
            inventory, manifest, matrix, qualification.load_policy(POLICY_PATH)
        )


def test_report_writer_keeps_the_qualification_projection_external_and_content_free(tmp_path):
    matrix = _matrix()
    manifest = _manifest()
    result = qualification.qualify_input_inventory(
        _inventory(manifest, matrix), manifest, matrix, qualification.load_policy(POLICY_PATH)
    )
    output_root = tmp_path / "external-output"
    output_root.mkdir()

    report_path = qualification.write_qualification_report(
        result, output_root=output_root, report_path=output_root / "qualification.json"
    )

    text = report_path.read_text(encoding="utf-8")
    assert str(output_root) not in text
    assert "document-00000" not in text
