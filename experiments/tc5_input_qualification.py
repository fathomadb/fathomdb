"""Content-free factual qualification for the SCALE-01 TC-5 live inputs.

This module validates a future safe inventory and an all-real manifest.  It
does not open corpus payloads, probe a host, load a model, invoke a driver, or
issue a coordinator release.  Its report is evidence for preflight only.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

from experiments import corpus_matrix
from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    QUERY_COUNT,
    Tc5Manifest,
    validate_manifest,
)


TC5_INPUT_INVENTORY_V1 = "tc5-input-inventory.v1"
TC5_INPUT_POLICY_V1 = "tc5-input-qualification-policy.v1"
TC5_INPUT_REPORT_V1 = "tc5-input-qualification-report.v1"
PROGRAM_TRACK = "SCALE-01"

_SAFE_IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:+-]{0,127}$")
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_POLICY_KEYS = {
    "schema_version",
    "program_track",
    "policy_id",
    "required_supported_claim",
    "required_attestations",
    "claim_boundary",
    "release_state",
}
_INVENTORY_KEYS = {
    "schema_version",
    "program_track",
    "inventory_id",
    "corpus_id",
    "corpus_matrix_sha256",
    "license_copy_sha256",
    "source_revision",
    "source_artifact_sha256",
    "manifest_sha256",
    "cpu_host_attestation_sha256",
    "model_asset_sha256",
    "model_cache_attested",
    "ground_truth_artifact_sha256",
    "vector_stage_runtime_sha256",
    "output_root_attestation_sha256",
}
_ATTESTATION_FIELDS = (
    "license_copy_sha256",
    "source_artifact_sha256",
    "cpu_host_attestation_sha256",
    "model_asset_sha256",
    "ground_truth_artifact_sha256",
    "vector_stage_runtime_sha256",
    "output_root_attestation_sha256",
)
_BLOCKER_CODES = {
    "corpus_matrix_source_selection",
    "license_copy",
    "all_real_manifest",
    "external_corpus_root",
    "external_output_root",
    "cpu_host_attestation",
    "pinned_model_asset",
    "exact_f32_ground_truth",
    "vector_stage_runtime",
}


class Tc5InputQualificationError(ValueError):
    """Raised when a TC-5 factual input is absent, unsafe, or inconsistent."""


@dataclass(frozen=True)
class Tc5InputQualificationPolicy:
    """Frozen requirements for one safe TC-5 factual qualification."""

    policy_id: str
    required_supported_claim: str
    required_attestations: tuple[str, ...]
    claim_boundary: str
    release_state: str


@dataclass(frozen=True)
class Tc5InputQualification:
    """Validated content-free preflight identity, not an execution release."""

    inventory_id: str
    corpus_id: str
    corpus_matrix_sha256: str
    manifest: Tc5Manifest
    license_copy_sha256: str
    source_revision: str
    cpu_host_attestation_sha256: str
    ground_truth_artifact_sha256: str
    vector_stage_runtime_sha256: str
    output_root_attestation_sha256: str
    policy: Tc5InputQualificationPolicy

    @property
    def state(self) -> str:
        """Return the factual state without implying coordinator authorization."""
        return "factual_inputs_qualified"

    def safe_report(self) -> dict[str, object]:
        """Project only hashes, logical identities, and frozen counts for a receipt."""
        provenance = self.manifest.provenance
        return {
            "schema_version": TC5_INPUT_REPORT_V1,
            "program_track": PROGRAM_TRACK,
            "report_id": f"{self.inventory_id}-qualification",
            "state": self.state,
            "eligible_for_coordinator_release": False,
            "inventory_id": self.inventory_id,
            "corpus_id": self.corpus_id,
            "corpus_matrix_sha256": self.corpus_matrix_sha256,
            "manifest_id": self.manifest.manifest_id,
            "manifest_sha256": self.manifest.manifest_sha256,
            "arms": [
                {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
                {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
            ],
            "input_attestations": {
                "license_copy_sha256": self.license_copy_sha256,
                "source_artifact_sha256": self.manifest.source_artifact_sha256,
                "model_asset_sha256": provenance["model_asset_sha256"],
                "cpu_host_attestation_sha256": self.cpu_host_attestation_sha256,
                "ground_truth_artifact_sha256": self.ground_truth_artifact_sha256,
                "vector_stage_runtime_sha256": self.vector_stage_runtime_sha256,
                "output_root_attestation_sha256": self.output_root_attestation_sha256,
            },
            "frozen_measurement": {
                "embed_device": "cpu",
                "candidate_breadth": CANDIDATE_BREADTH,
                "query_count": QUERY_COUNT,
                "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
                "ground_truth": provenance["ground_truth"],
                "sut": provenance["sut"],
            },
            "claim_boundary": self.policy.claim_boundary,
            "next_gate": self.policy.release_state,
            "no_live_execution": True,
        }


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def _identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_IDENTIFIER.fullmatch(value) is None:
        raise Tc5InputQualificationError(f"{label} must be a safe identifier")
    return value


def _label(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_LABEL.fullmatch(value) is None:
        raise Tc5InputQualificationError(f"{label} must be a safe label")
    return value


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Tc5InputQualificationError(f"{label} must be a lowercase sha256")
    return value


def _external_output_path(output_root: str | Path, report_path: str | Path) -> Path:
    root = Path(output_root).resolve()
    destination = Path(report_path).resolve()
    repository = _repository_root()
    if root.is_relative_to(repository) or destination.is_relative_to(repository):
        raise Tc5InputQualificationError("qualification output must remain outside the repository")
    if not root.is_dir():
        raise Tc5InputQualificationError("qualification output root must be an existing external directory")
    if not destination.is_relative_to(root):
        raise Tc5InputQualificationError("qualification report path must remain under output root")
    if destination.exists() or destination.is_symlink():
        raise Tc5InputQualificationError("qualification report destination already exists")
    return destination


def load_policy(path: str | Path) -> Tc5InputQualificationPolicy:
    """Load the committed policy without reading external corpus content."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise Tc5InputQualificationError("qualification policy contains invalid JSON") from exc
    if not isinstance(document, dict) or set(document) != _POLICY_KEYS:
        raise Tc5InputQualificationError("qualification policy keys do not match v1")
    if document["schema_version"] != TC5_INPUT_POLICY_V1 or document["program_track"] != PROGRAM_TRACK:
        raise Tc5InputQualificationError("qualification policy identity is invalid")
    attestations = document["required_attestations"]
    if not isinstance(attestations, list) or tuple(attestations) != _ATTESTATION_FIELDS:
        raise Tc5InputQualificationError("qualification policy attestations are incomplete")
    if document["required_supported_claim"] != "retrieval_fidelity_only":
        raise Tc5InputQualificationError("qualification policy claim boundary is invalid")
    if document["claim_boundary"] != "fidelity_only_no_scale02_or_product_claim":
        raise Tc5InputQualificationError("qualification policy claim wording is invalid")
    if document["release_state"] != "factual_inputs_qualified_pending_coordinator_release":
        raise Tc5InputQualificationError("qualification policy release state is invalid")
    return Tc5InputQualificationPolicy(
        policy_id=_identifier(document["policy_id"], "qualification policy_id"),
        required_supported_claim=document["required_supported_claim"],
        required_attestations=tuple(attestations),
        claim_boundary=document["claim_boundary"],
        release_state=document["release_state"],
    )


def qualify_input_inventory(
    inventory: object,
    manifest_document: object,
    matrix_document: object,
    policy: Tc5InputQualificationPolicy,
) -> Tc5InputQualification:
    """Validate future TC-5 input facts and their exact all-real safe manifest."""
    if not isinstance(inventory, dict) or set(inventory) != _INVENTORY_KEYS:
        raise Tc5InputQualificationError("input inventory keys do not match tc5-input-inventory.v1")
    if inventory["schema_version"] != TC5_INPUT_INVENTORY_V1 or inventory["program_track"] != PROGRAM_TRACK:
        raise Tc5InputQualificationError("input inventory identity is invalid")
    inventory_id = _identifier(inventory["inventory_id"], "inventory_id")
    matrix_sha256 = _sha256(inventory["corpus_matrix_sha256"], "corpus_matrix_sha256")
    if matrix_sha256 != corpus_matrix.canonical_sha256(matrix_document):
        raise Tc5InputQualificationError("corpus_matrix_sha256 does not bind the supplied matrix")
    matrix = corpus_matrix.validate_matrix(matrix_document)
    corpus_id = _identifier(inventory["corpus_id"], "corpus_id")
    try:
        matrix.entry(corpus_id)
    except corpus_matrix.CorpusMatrixError as exc:
        raise Tc5InputQualificationError("corpus_id is not a CORPUS-01 matrix source") from exc
    corpus_rows = matrix_document["corpora"]
    selected_row = next(
        row for row in corpus_rows
        if isinstance(row, dict) and row.get("corpus_id") == corpus_id
    )
    supported_claims = selected_row["supported_claims"]
    if policy.required_supported_claim not in supported_claims:
        raise Tc5InputQualificationError("corpus matrix does not support retrieval_fidelity_only")
    for field in policy.required_attestations:
        _sha256(inventory[field], field)
    _label(inventory["source_revision"], "source_revision")
    if inventory["model_cache_attested"] is not True:
        raise Tc5InputQualificationError("model_cache_attested must be true")
    try:
        manifest = validate_manifest(manifest_document)
    except ValueError as exc:
        raise Tc5InputQualificationError("all-real manifest is invalid") from exc
    if manifest.manifest_sha256 != inventory["manifest_sha256"]:
        raise Tc5InputQualificationError("manifest_sha256 does not bind the exact all-real manifest")
    if manifest.source_artifact_sha256 != inventory["source_artifact_sha256"]:
        raise Tc5InputQualificationError("source_artifact_sha256 does not match the all-real manifest")
    if manifest.provenance["model_asset_sha256"] != inventory["model_asset_sha256"]:
        raise Tc5InputQualificationError("model_asset_sha256 does not match the all-real manifest")
    return Tc5InputQualification(
        inventory_id=inventory_id,
        corpus_id=corpus_id,
        corpus_matrix_sha256=matrix_sha256,
        manifest=manifest,
        license_copy_sha256=inventory["license_copy_sha256"],
        source_revision=inventory["source_revision"],
        cpu_host_attestation_sha256=inventory["cpu_host_attestation_sha256"],
        ground_truth_artifact_sha256=inventory["ground_truth_artifact_sha256"],
        vector_stage_runtime_sha256=inventory["vector_stage_runtime_sha256"],
        output_root_attestation_sha256=inventory["output_root_attestation_sha256"],
        policy=policy,
    )


def blocked_input_report(
    *, report_id: str, missing_prerequisites: Sequence[str], observed_inventory_ids: Sequence[str]
) -> dict[str, object]:
    """Record a precise content-free blocker without implying a candidate is qualified."""
    missing = tuple(_identifier(value, "missing prerequisite") for value in missing_prerequisites)
    if not missing or len(set(missing)) != len(missing) or not set(missing).issubset(_BLOCKER_CODES):
        raise Tc5InputQualificationError("blocked report prerequisites are invalid")
    observed = tuple(_identifier(value, "observed inventory") for value in observed_inventory_ids)
    if len(set(observed)) != len(observed):
        raise Tc5InputQualificationError("blocked report observed inventories must be unique")
    return {
        "schema_version": TC5_INPUT_REPORT_V1,
        "program_track": PROGRAM_TRACK,
        "report_id": _identifier(report_id, "report_id"),
        "state": "blocked_prerequisite",
        "eligible_for_coordinator_release": False,
        "observed_inventory_ids": list(observed),
        "missing_prerequisites": list(missing),
        "claim_boundary": "fidelity_only_no_scale02_or_product_claim",
        "no_live_execution": True,
    }


def write_qualification_report(
    qualification: Tc5InputQualification | Mapping[str, object], *, output_root: str | Path, report_path: str | Path
) -> Path:
    """Write one new content-free qualification or blocked report outside Git."""
    report = qualification.safe_report() if isinstance(qualification, Tc5InputQualification) else dict(qualification)
    if report.get("schema_version") != TC5_INPUT_REPORT_V1 or report.get("program_track") != PROGRAM_TRACK:
        raise Tc5InputQualificationError("qualification report identity is invalid")
    if report.get("no_live_execution") is not True or report.get("eligible_for_coordinator_release") is not False:
        raise Tc5InputQualificationError("qualification report execution boundary is invalid")
    destination = _external_output_path(output_root, report_path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(_canonical_json(report) + b"\n")
    return destination
