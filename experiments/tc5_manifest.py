"""Manifest-only, all-real preparation contract for the SCALE-01 TC-5 study.

This module deliberately validates metadata and writes a safe planning receipt
only. It neither reads a corpus payload nor invokes the historical EU7 harness,
an embedder, or a measurement.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence


TC5_MANIFEST_V1 = "tc5-manifest.v1"
TC5_PLANNING_RECEIPT_V1 = "tc5-planning-receipt.v1"
PROGRAM_TRACK = "SCALE-01"
PRIMARY_DOCUMENT_COUNT = 18_472
BRIDGE_DOCUMENT_COUNT = 7_667
CANDIDATE_BREADTH = 192
QUERY_COUNT = 100
QUERY_SELECT_SEED = "0x0E77C0125E1EC7"
BOOTSTRAP_RESAMPLES = 1_000
BOOTSTRAP_SEED = "0x0E77B007574A9"

_SAFE_IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:+-]{0,127}$")
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_GIT_SHA = re.compile(r"^[0-9a-f]{40}$")
_DOCUMENT_KEYS = {"document_id", "content_sha256", "origin"}
_MANIFEST_KEYS = {
    "schema_version",
    "program_track",
    "manifest_id",
    "source_artifact_sha256",
    "documents",
    "bridge_document_ids",
    "provenance",
}
_PROVENANCE_KEYS = {
    "source_commit",
    "cargo_lock_sha256",
    "rust_version",
    "cpu_identity",
    "os_identity",
    "model_identity",
    "model_asset_sha256",
    "engine_features",
    "embed_device",
    "candidate_breadth",
    "query_count",
    "query_select_seed",
    "bootstrap_resamples",
    "bootstrap_seed",
    "ground_truth",
    "sut",
}
_HISTORICAL_EU7_OUTPUT = Path("dev/plans/runs/eu7-latest-measurements.json")


class Tc5ManifestError(ValueError):
    """Raised when a TC-5 preparation input is unsafe, incomplete, or unqualified."""


@dataclass(frozen=True)
class Tc5Manifest:
    """Validated, content-free identity selection for the two frozen TC-5 arms."""

    manifest_id: str
    manifest_sha256: str
    source_artifact_sha256: str
    document_count: int
    bridge_document_ids: tuple[str, ...]
    provenance: Mapping[str, object]


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def _require_identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_IDENTIFIER.fullmatch(value) is None:
        raise Tc5ManifestError(f"{label} must be a safe identifier")
    return value


def _require_label(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_LABEL.fullmatch(value) is None:
        raise Tc5ManifestError(f"{label} must be a safe label")
    return value


def _require_sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Tc5ManifestError(f"{label} must be a lowercase sha256")
    return value


def validate_selection_ids(
    identifiers: Sequence[object], *, expected_count: int, label: str
) -> tuple[str, ...]:
    """Validate a canonical all-real selection without reading corpus payloads."""
    selected = tuple(_require_identifier(identifier, f"{label} identifier") for identifier in identifiers)
    if len(set(selected)) != len(selected):
        raise Tc5ManifestError(f"{label} contains a duplicate document identifier")
    if len(selected) != expected_count:
        raise Tc5ManifestError(f"{label} count does not match the frozen contract")
    return selected


def _validate_provenance(value: object) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != _PROVENANCE_KEYS:
        raise Tc5ManifestError("provenance keys do not match tc5-manifest.v1")
    if not isinstance(value["source_commit"], str) or _GIT_SHA.fullmatch(value["source_commit"]) is None:
        raise Tc5ManifestError("provenance source_commit must be a lowercase Git SHA")
    _require_sha256(value["cargo_lock_sha256"], "provenance cargo_lock_sha256")
    _require_sha256(value["model_asset_sha256"], "provenance model_asset_sha256")
    for field in ("rust_version", "cpu_identity", "os_identity", "model_identity", "ground_truth", "sut"):
        _require_label(value[field], f"provenance {field}")
    engine_features = value["engine_features"]
    if not isinstance(engine_features, list) or not engine_features:
        raise Tc5ManifestError("provenance engine_features must be a non-empty list")
    normalized_features = [_require_identifier(feature, "provenance engine feature") for feature in engine_features]
    if normalized_features != sorted(normalized_features) or len(set(normalized_features)) != len(normalized_features):
        raise Tc5ManifestError("provenance engine_features must be unique canonical order")
    frozen = {
        "embed_device": "cpu",
        "candidate_breadth": CANDIDATE_BREADTH,
        "query_count": QUERY_COUNT,
        "query_select_seed": QUERY_SELECT_SEED,
        "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
        "bootstrap_seed": BOOTSTRAP_SEED,
        "model_identity": "fathomdb-bge-small-en-v1.5",
        "ground_truth": "exact-f32-same-model-top-10",
        "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }
    for field, expected in frozen.items():
        if value[field] != expected:
            raise Tc5ManifestError(f"provenance {field} does not match the frozen TC-5 contract")
    return dict(value)


def validate_manifest(document: object) -> Tc5Manifest:
    """Validate one externally supplied TC-5 manifest without opening its corpus."""
    if not isinstance(document, dict) or set(document) != _MANIFEST_KEYS:
        raise Tc5ManifestError("manifest keys do not match tc5-manifest.v1")
    if document["schema_version"] != TC5_MANIFEST_V1:
        raise Tc5ManifestError("manifest schema_version must be tc5-manifest.v1")
    if document["program_track"] != PROGRAM_TRACK:
        raise Tc5ManifestError("manifest program_track must be SCALE-01")
    manifest_id = _require_identifier(document["manifest_id"], "manifest_id")
    source_artifact_sha256 = _require_sha256(
        document["source_artifact_sha256"], "source_artifact_sha256"
    )
    documents = document["documents"]
    if not isinstance(documents, list):
        raise Tc5ManifestError("documents must be a list")
    document_ids: list[object] = []
    for row in documents:
        if not isinstance(row, dict) or set(row) != _DOCUMENT_KEYS:
            raise Tc5ManifestError("document row keys do not match tc5-manifest.v1")
        document_ids.append(row["document_id"])
        _require_sha256(row["content_sha256"], "document content_sha256")
        if row["origin"] != "real":
            raise Tc5ManifestError("synthetic or unknown document origin is not permitted")
    selected_ids = validate_selection_ids(
        document_ids, expected_count=PRIMARY_DOCUMENT_COUNT, label="primary document"
    )
    if list(selected_ids) != sorted(selected_ids):
        raise Tc5ManifestError("primary document identifiers must use canonical order")
    bridge_ids = validate_selection_ids(
        document["bridge_document_ids"], expected_count=BRIDGE_DOCUMENT_COUNT, label="bridge document"
    ) if isinstance(document["bridge_document_ids"], list) else _reject_bridge_type()
    if bridge_ids != selected_ids[:BRIDGE_DOCUMENT_COUNT]:
        raise Tc5ManifestError("bridge document IDs must be the canonical primary-manifest subset")
    provenance = _validate_provenance(document["provenance"])
    return Tc5Manifest(
        manifest_id=manifest_id,
        manifest_sha256=hashlib.sha256(_canonical_json(document)).hexdigest(),
        source_artifact_sha256=source_artifact_sha256,
        document_count=len(selected_ids),
        bridge_document_ids=bridge_ids,
        provenance=provenance,
    )


def _reject_bridge_type() -> tuple[str, ...]:
    raise Tc5ManifestError("bridge document IDs must be a list")


def _external_directory(path: str | Path, label: str) -> Path:
    resolved = Path(path).resolve()
    if resolved.is_relative_to(_repository_root()):
        raise Tc5ManifestError(f"{label} must remain outside the repository")
    if not resolved.is_dir():
        raise Tc5ManifestError(f"{label} must be an existing external directory")
    return resolved


def _external_manifest_path(path: str | Path) -> Path:
    resolved = Path(path).resolve()
    if resolved.is_relative_to(_repository_root()):
        raise Tc5ManifestError("manifest path must remain outside the repository")
    if not resolved.is_file():
        raise Tc5ManifestError("manifest path must be an existing external file")
    return resolved


def load_manifest(path: str | Path) -> Tc5Manifest:
    """Load and validate an external content-free TC-5 manifest file."""
    manifest_path = _external_manifest_path(path)
    try:
        document = json.loads(manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise Tc5ManifestError("manifest path contains invalid JSON") from exc
    return validate_manifest(document)


def prepare_planning_receipt(
    manifest: Tc5Manifest, *, corpus_root: str | Path, output_root: str | Path
) -> dict[str, object]:
    """Project a validated manifest into a content-free, non-execution receipt."""
    corpus = _external_directory(corpus_root, "corpus root")
    output = _external_directory(output_root, "output root")
    provenance = manifest.provenance
    return {
        "schema_version": TC5_PLANNING_RECEIPT_V1,
        "program_track": PROGRAM_TRACK,
        "manifest_id": manifest.manifest_id,
        "manifest_sha256": manifest.manifest_sha256,
        "arms": [
            {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
            {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
        ],
        "frozen_configuration": {
            "embed_device": "cpu",
            "model_identity": provenance["model_identity"],
            "model_asset_sha256": provenance["model_asset_sha256"],
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": provenance["ground_truth"],
            "sut": provenance["sut"],
        },
        "provenance": {
            "source_artifact_sha256": manifest.source_artifact_sha256,
            "source_commit": provenance["source_commit"],
            "cargo_lock_sha256": provenance["cargo_lock_sha256"],
            "rust_version": provenance["rust_version"],
            "cpu_identity": provenance["cpu_identity"],
            "os_identity": provenance["os_identity"],
            "engine_features": provenance["engine_features"],
        },
        "artifact_refs": {
            "corpus_root_sha256": hashlib.sha256(str(corpus).encode("utf-8")).hexdigest(),
            "output_root_sha256": hashlib.sha256(str(output).encode("utf-8")).hexdigest(),
        },
        "execution": {
            "status": "planned_not_executed",
            "smoke_performed": False,
            "measurement_performed": False,
            "synthetic_document_count": 0,
            "historical_eu7_output_used": False,
        },
    }


def write_planning_receipt(
    manifest: Tc5Manifest,
    *,
    corpus_root: str | Path,
    output_root: str | Path,
    receipt_path: str | Path,
) -> Path:
    """Write the safe planning receipt externally, never to the EU7 historical output."""
    historical = (_repository_root() / _HISTORICAL_EU7_OUTPUT).resolve()
    destination = Path(receipt_path).resolve()
    if destination == historical:
        raise Tc5ManifestError("historical eu7 output must never be used by TC-5")
    output = _external_directory(output_root, "output root")
    if destination.is_relative_to(_repository_root()):
        raise Tc5ManifestError("receipt path must remain outside the repository")
    if not destination.is_relative_to(output):
        raise Tc5ManifestError("receipt path must remain under the declared external output root")
    receipt = prepare_planning_receipt(manifest, corpus_root=corpus_root, output_root=output)
    destination.write_bytes(_canonical_json(receipt) + b"\n")
    return destination
