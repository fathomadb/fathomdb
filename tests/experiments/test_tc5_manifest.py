"""Human-intended tests for the SCALE-01 TC-5 manifest-only preparation path."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest
from hypothesis import given, strategies as st

from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
    Tc5ManifestError,
    load_manifest,
    prepare_planning_receipt,
    validate_manifest,
    validate_selection_ids,
    write_planning_receipt,
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


def _write_manifest(path: Path, document: dict[str, object]) -> Path:
    path.write_text(json.dumps(document), encoding="utf-8")
    return path


def test_manifest_loader_freezes_two_all_real_arms_and_content_free_receipt(tmp_path):
    manifest_path = _write_manifest(tmp_path / "manifest.json", _manifest())
    corpus_root = tmp_path / "external-corpus"
    output_root = tmp_path / "external-output"
    corpus_root.mkdir()
    output_root.mkdir()

    manifest = load_manifest(manifest_path)
    receipt = prepare_planning_receipt(
        manifest, corpus_root=corpus_root, output_root=output_root
    )

    assert manifest.document_count == PRIMARY_DOCUMENT_COUNT
    assert manifest.bridge_document_ids == tuple(
        f"document-{number:05d}" for number in range(BRIDGE_DOCUMENT_COUNT)
    )
    assert receipt["schema_version"] == "tc5-planning-receipt.v1"
    assert receipt["program_track"] == "SCALE-01"
    assert receipt["arms"] == [
        {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
        {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
    ]
    assert receipt["frozen_configuration"]["embed_device"] == "cpu"
    assert receipt["frozen_configuration"]["model_asset_sha256"] == "d" * 64
    assert receipt["execution"] == {
        "status": "planned_not_executed",
        "smoke_performed": False,
        "measurement_performed": False,
        "synthetic_document_count": 0,
        "historical_eu7_output_used": False,
    }
    serialized = json.dumps(receipt)
    assert str(corpus_root) not in serialized
    assert str(output_root) not in serialized
    assert "document-00000" not in serialized


@pytest.mark.parametrize(
    ("mutate", "message"),
    [
        (lambda value: value["documents"].pop(), "primary document count"),
        (lambda value: value["documents"].append(value["documents"][0].copy()), "duplicate document"),
        (lambda value: value["bridge_document_ids"].__setitem__(0, "missing-document"), "bridge document"),
        (lambda value: value["documents"].__setitem__(0, {"document_id": "document-00000"}), "document row"),
        (lambda value: value["documents"][0].__setitem__("origin", "synthetic"), "synthetic"),
        (lambda value: value["provenance"].pop("model_asset_sha256"), "provenance"),
    ],
)
def test_manifest_fails_closed_on_unqualified_documents_or_provenance(mutate, message):
    document = _manifest()
    mutate(document)

    with pytest.raises(Tc5ManifestError, match=message):
        validate_manifest(document)


def test_receipt_writer_rejects_repository_and_historical_eu7_output(tmp_path):
    manifest = validate_manifest(_manifest())
    corpus_root = tmp_path / "external-corpus"
    corpus_root.mkdir()
    repository = Path(__file__).resolve().parents[2]
    historical_output = repository / "dev/plans/runs/eu7-latest-measurements.json"

    with pytest.raises(Tc5ManifestError, match="outside the repository"):
        prepare_planning_receipt(
            manifest, corpus_root=corpus_root, output_root=repository / "unsafe-output"
        )
    with pytest.raises(Tc5ManifestError, match="historical eu7 output"):
        write_planning_receipt(
            manifest,
            corpus_root=corpus_root,
            output_root=tmp_path / "external-output",
            receipt_path=historical_output,
        )


def test_writer_emits_only_the_safe_planning_projection(tmp_path):
    manifest = validate_manifest(_manifest())
    corpus_root = tmp_path / "external-corpus"
    output_root = tmp_path / "external-output"
    corpus_root.mkdir()
    output_root.mkdir()

    receipt_path = write_planning_receipt(
        manifest,
        corpus_root=corpus_root,
        output_root=output_root,
        receipt_path=output_root / "tc5-planning-receipt.json",
    )

    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert receipt["manifest_sha256"] == hashlib.sha256(
        json.dumps(_manifest(), sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    assert set(receipt) == {
        "schema_version",
        "program_track",
        "manifest_id",
        "manifest_sha256",
        "arms",
        "frozen_configuration",
        "provenance",
        "artifact_refs",
        "execution",
    }
    assert "document_id" not in json.dumps(receipt)


@given(st.lists(st.integers(min_value=0, max_value=1000), unique=True, max_size=30))
def test_selection_validation_preserves_each_unique_safe_identifier(values):
    identifiers = tuple(f"document-{value}" for value in values)

    assert validate_selection_ids(identifiers, expected_count=len(identifiers), label="property") == identifiers
