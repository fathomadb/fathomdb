"""Non-executing release boundary for the SCALE-01 TC-5 characterization.

This module composes :mod:`experiments.tc5_manifest` without changing that
shared preparation helper.  It validates the frozen CPU configuration and
projects a content-free receipt for a later, explicitly released executor.  It
never reads a corpus payload, invokes EU7, loads a model, or runs a benchmark.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    PROGRAM_TRACK,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
    Tc5Manifest,
    Tc5ManifestError,
    load_manifest,
)


TC5_EXECUTION_CONFIG_V1 = "tc5-execution-config.v1"
TC5_EXECUTION_RECEIPT_V1 = "tc5-execution-receipt.v1"
_AWAITING_RELEASE = "awaiting_independent_review_and_coordinator_release"
_HISTORICAL_EU7_OUTPUT = Path("dev/plans/runs/eu7-latest-measurements.json")
_CONFIG_KEYS = {
    "schema_version",
    "program_track",
    "configuration_id",
    "release_state",
    "execution_enabled",
    "arms",
    "frozen_configuration",
    "artifact_contract",
    "claim_boundary",
}
_FROZEN_CONFIGURATION = {
    "embed_device": "cpu",
    "model_identity": "fathomdb-bge-small-en-v1.5",
    "candidate_breadth": CANDIDATE_BREADTH,
    "query_count": QUERY_COUNT,
    "query_select_seed": QUERY_SELECT_SEED,
    "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
    "bootstrap_seed": BOOTSTRAP_SEED,
    "ground_truth": "exact-f32-same-model-top-10",
    "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
}
_ARMS = [
    {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
    {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
]
_ARTIFACT_CONTRACT = {
    "external_manifest_required": True,
    "external_corpus_root_required": True,
    "external_output_root_required": True,
    "historical_eu7_output_forbidden": True,
    "repository_artifacts_forbidden": True,
    "experiment_index_receipt_required_after_both_arms": True,
}
_CLAIM_BOUNDARY = {
    "scale_02_capacity_claim": False,
    "latency_or_slo_claim": False,
    "only_fidelity_and_uncertainty_after_complete_arms": True,
}


class Tc5CharacterizationError(ValueError):
    """Raised when a TC-5 characterization invocation is not safely qualified."""


@dataclass(frozen=True)
class Tc5ExecutionConfig:
    """Frozen, deliberately disabled execution configuration for one TC-5 run."""

    configuration_id: str
    release_state: str
    execution_enabled: bool
    arms: tuple[dict[str, object], ...]
    frozen_configuration: Mapping[str, object]
    artifact_contract: Mapping[str, bool]
    claim_boundary: Mapping[str, bool]
    config_sha256: str


@dataclass(frozen=True)
class PreparedCharacterization:
    """Qualified, content-free request awaiting independent review and release."""

    config: Tc5ExecutionConfig
    manifest: Tc5Manifest
    output_root: Path
    receipt: Mapping[str, object]


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def _safe_identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or len(value) > 128:
        raise Tc5CharacterizationError(f"{label} must be a non-empty safe identifier")
    if any(character not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._:-" for character in value):
        raise Tc5CharacterizationError(f"{label} must be a non-empty safe identifier")
    return value


def _external_directory(path: str | Path, label: str) -> Path:
    resolved = Path(path).resolve()
    if resolved.is_relative_to(_repository_root()):
        raise Tc5CharacterizationError(f"{label} must remain outside the repository")
    if not resolved.is_dir():
        raise Tc5CharacterizationError(f"{label} must be an existing external directory")
    return resolved


def _load_json(path: str | Path, label: str) -> dict[str, object]:
    source = Path(path)
    if not source.is_file():
        raise Tc5CharacterizationError(f"{label} must be an existing JSON file")
    try:
        document = json.loads(source.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise Tc5CharacterizationError(f"{label} contains invalid JSON") from exc
    if not isinstance(document, dict):
        raise Tc5CharacterizationError(f"{label} must be a JSON object")
    return document


def load_execution_config(path: str | Path) -> Tc5ExecutionConfig:
    """Load the disabled TC-5 CPU configuration, rejecting any drift or release."""
    document = _load_json(path, "execution configuration")
    if set(document) != _CONFIG_KEYS:
        raise Tc5CharacterizationError("execution configuration keys do not match tc5-execution-config.v1")
    if document["schema_version"] != TC5_EXECUTION_CONFIG_V1:
        raise Tc5CharacterizationError("execution configuration schema_version does not match tc5-execution-config.v1")
    if document["program_track"] != PROGRAM_TRACK:
        raise Tc5CharacterizationError("execution configuration program_track must be SCALE-01")
    configuration_id = _safe_identifier(document["configuration_id"], "configuration_id")
    if document["release_state"] != _AWAITING_RELEASE:
        raise Tc5CharacterizationError("execution configuration release_state is not awaiting independent review")
    if document["execution_enabled"] is not False:
        raise Tc5CharacterizationError("execution configuration execution_enabled must remain false before release")
    if document["arms"] != _ARMS:
        raise Tc5CharacterizationError("execution configuration arms do not match the frozen bridge and primary contract")
    if document["frozen_configuration"] != _FROZEN_CONFIGURATION:
        raise Tc5CharacterizationError(
            "execution configuration frozen_configuration does not match CPU TC-5 pins "
            f"(including embed_device={_FROZEN_CONFIGURATION['embed_device']!r})"
        )
    if document["artifact_contract"] != _ARTIFACT_CONTRACT:
        raise Tc5CharacterizationError("execution configuration artifact_contract does not match the safe receipt boundary")
    if document["claim_boundary"] != _CLAIM_BOUNDARY:
        raise Tc5CharacterizationError("execution configuration claim_boundary does not match SCALE-01 scope")
    return Tc5ExecutionConfig(
        configuration_id=configuration_id,
        release_state=_AWAITING_RELEASE,
        execution_enabled=False,
        arms=tuple(dict(arm) for arm in _ARMS),
        frozen_configuration=dict(_FROZEN_CONFIGURATION),
        artifact_contract=dict(_ARTIFACT_CONTRACT),
        claim_boundary=dict(_CLAIM_BOUNDARY),
        config_sha256=hashlib.sha256(_canonical_json(document)).hexdigest(),
    )


def _receipt(
    config: Tc5ExecutionConfig,
    manifest: Tc5Manifest,
    *,
    corpus_root: Path,
    output_root: Path,
) -> dict[str, object]:
    provenance = manifest.provenance
    return {
        "schema_version": TC5_EXECUTION_RECEIPT_V1,
        "program_track": PROGRAM_TRACK,
        "configuration_id": config.configuration_id,
        "configuration_sha256": config.config_sha256,
        "manifest_id": manifest.manifest_id,
        "manifest_sha256": manifest.manifest_sha256,
        "arms": list(config.arms),
        "frozen_configuration": dict(config.frozen_configuration),
        "provenance": {
            "source_artifact_sha256": manifest.source_artifact_sha256,
            "source_commit": provenance["source_commit"],
            "cargo_lock_sha256": provenance["cargo_lock_sha256"],
            "model_asset_sha256": provenance["model_asset_sha256"],
            "rust_version": provenance["rust_version"],
            "cpu_identity": provenance["cpu_identity"],
            "os_identity": provenance["os_identity"],
            "engine_features": provenance["engine_features"],
        },
        "artifact_refs": {
            "corpus_root_sha256": hashlib.sha256(str(corpus_root).encode("utf-8")).hexdigest(),
            "output_root_sha256": hashlib.sha256(str(output_root).encode("utf-8")).hexdigest(),
        },
        "experiment_index_projection": {
            "schema_version": "experiments.index-row.v1",
            "append_status": "not_appended",
            "eligibility": "both bridge and primary arms complete with safe receipt",
        },
        "claim_boundary": dict(config.claim_boundary),
        "execution": {
            "status": config.release_state,
            "smoke_performed": False,
            "measurement_performed": False,
            "complete_arm_results_required": ["bridge", "primary"],
        },
    }


def prepare_characterization(
    *,
    config_path: str | Path,
    manifest_path: str | Path,
    corpus_root: str | Path,
    output_root: str | Path,
) -> PreparedCharacterization:
    """Validate a no-live TC-5 request and create its content-free receipt projection."""
    config = load_execution_config(config_path)
    try:
        manifest = load_manifest(manifest_path)
    except Tc5ManifestError as exc:
        raise Tc5CharacterizationError(str(exc)) from exc
    corpus = _external_directory(corpus_root, "corpus root")
    output = _external_directory(output_root, "output root")
    return PreparedCharacterization(
        config=config,
        manifest=manifest,
        output_root=output,
        receipt=_receipt(config, manifest, corpus_root=corpus, output_root=output),
    )


def write_execution_receipt(prepared: PreparedCharacterization, *, receipt_path: str | Path) -> Path:
    """Write only the safe, pre-execution receipt under the declared external output root."""
    destination = Path(receipt_path).resolve()
    historical = (_repository_root() / _HISTORICAL_EU7_OUTPUT).resolve()
    if destination == historical:
        raise Tc5CharacterizationError("historical eu7 output must never be used by TC-5")
    if destination.is_relative_to(_repository_root()):
        raise Tc5CharacterizationError("receipt path must remain outside the repository")
    if not destination.is_relative_to(prepared.output_root):
        raise Tc5CharacterizationError("receipt path must remain under the declared external output root")
    destination.write_bytes(_canonical_json(prepared.receipt) + b"\n")
    return destination


def run_characterization(**_: object) -> None:
    """Refuse live execution until independent review and coordinator release land."""
    raise Tc5CharacterizationError(
        "TC-5 configuration is not released for execution; this runner is preparation-only"
    )
