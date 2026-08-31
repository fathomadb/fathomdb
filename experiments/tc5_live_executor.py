"""Released, external-only executor for the SCALE-01 TC-5 CPU study.

This module deliberately leaves :mod:`experiments.tc5_characterization` in
its preparation-only state.  A live invocation is possible only with a
coordinator-signed release sidecar whose pins match this checkout, the frozen
configuration, and one qualified external manifest.  It invokes an external
runner once for the bridge and once for the primary arm, in that order.

The executor never writes the in-repository experiment index.  It materializes
only strict, content-free external arm sidecars and a safe two-arm receipt;
coordinator review decides whether that receipt may later be indexed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import subprocess
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Sequence

from experiments.tc5_characterization import Tc5CharacterizationError, Tc5ExecutionConfig, load_execution_config
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


TC5_LIVE_EXECUTOR_CONFIG_V1 = "tc5-live-executor-config.v1"
TC5_RELEASE_RECORD_V1 = "tc5-execution-release.v1"
TC5_ARM_RESULT_V1 = "tc5-arm-result.v1"
TC5_LIVE_EXECUTOR_RECEIPT_V1 = "tc5-live-executor-receipt.v1"

_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_GIT_SHA = re.compile(r"^[0-9a-f]{40}$")
_SAFE_IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_LIVE_CONFIG_KEYS = {
    "schema_version",
    "program_track",
    "executor_id",
    "execution_config_sha256",
    "arms",
    "approved_actions",
    "result_contract",
}
_RELEASE_KEYS = {
    "schema_version",
    "program_track",
    "release_id",
    "issued_by",
    "integrated_sha",
    "live_config_sha256",
    "manifest_sha256",
    "approved_actions",
    "expires_at",
    "runner_argv",
    "runner_sha256",
    "release_sha256",
}
_ARM_RESULT_KEYS = {
    "schema_version",
    "program_track",
    "action",
    "arm",
    "document_count",
    "manifest_sha256",
    "ground_truth_sha256",
    "sut_result_sha256",
    "query_completion_count",
    "bootstrap_resamples",
    "synthetic_document_count",
    "metrics",
    "provenance",
}
_RESULT_CONTRACT = {
    "schema_version": TC5_ARM_RESULT_V1,
    "required_ground_truth": "exact-f32-same-model-top-10",
    "required_embed_device": "cpu",
    "prohibit_payloads_and_raw_paths": True,
    "prohibit_scale_02_claims": True,
}
_ARMS = (("bridge", BRIDGE_DOCUMENT_COUNT), ("primary", PRIMARY_DOCUMENT_COUNT))
_ACTIONS = ("tc5-smoke", "tc5-long-cpu-characterization")


class Tc5LiveExecutorError(ValueError):
    """Raised when a requested SCALE-01 execution is not safely released."""


@dataclass(frozen=True)
class Tc5LiveExecutorConfig:
    """Live-executor pins bound to the existing disabled TC-5 configuration."""

    executor_id: str
    config_sha256: str
    execution_config_sha256: str
    arms: tuple[tuple[str, int], ...]
    approved_actions: tuple[str, ...]


@dataclass(frozen=True)
class Tc5ReleaseRecord:
    """Coordinator-issued authorization for one immutable TC-5 invocation."""

    release_id: str
    integrated_sha: str
    approved_actions: tuple[str, ...]
    runner_argv: tuple[str, ...]
    record_sha256: str
    runner_sha256: str


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False
    ).encode("utf-8")


def _require_sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Tc5LiveExecutorError(f"{label} must be a lowercase sha256")
    return value


def _require_identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_IDENTIFIER.fullmatch(value) is None:
        raise Tc5LiveExecutorError(f"{label} must be a safe identifier")
    return value


def _load_json(path: str | Path, label: str, *, external: bool) -> dict[str, object]:
    source = Path(path).resolve()
    if external and source.is_relative_to(_repository_root()):
        raise Tc5LiveExecutorError(f"{label} must remain outside the repository")
    if not source.is_file():
        raise Tc5LiveExecutorError(f"{label} must be an existing JSON file")
    try:
        document = json.loads(source.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise Tc5LiveExecutorError(f"{label} contains invalid JSON") from exc
    if not isinstance(document, dict):
        raise Tc5LiveExecutorError(f"{label} must be a JSON object")
    return document


def _external_directory(path: str | Path, label: str) -> Path:
    resolved = Path(path).resolve()
    if resolved.is_relative_to(_repository_root()):
        raise Tc5LiveExecutorError(f"{label} must remain outside the repository")
    if not resolved.is_dir():
        raise Tc5LiveExecutorError(f"{label} must be an existing external directory")
    return resolved


def _safe_external_output_path(output_root: Path, *parts: str, label: str) -> Path:
    """Resolve one output destination and reject repository or symlink escapes."""
    candidate = output_root.joinpath(*parts)
    resolved = candidate.resolve(strict=False)
    if not resolved.is_relative_to(output_root):
        raise Tc5LiveExecutorError(f"{label} escapes the declared external output root")
    if resolved.is_relative_to(_repository_root()):
        raise Tc5LiveExecutorError(f"{label} must remain outside the repository")
    return resolved


def _execution_config_sha256(path: str | Path) -> str:
    document = _load_json(path, "execution configuration", external=False)
    return hashlib.sha256(_canonical_json(document)).hexdigest()


def _frozen_arms(config: Tc5ExecutionConfig) -> tuple[tuple[str, int], ...]:
    return tuple((str(arm["name"]), int(arm["document_count"])) for arm in config.arms)


def load_live_executor_config(
    path: str | Path, *, base_execution_config_path: str | Path
) -> Tc5LiveExecutorConfig:
    """Load the strict executor config and bind it to disabled TC-5 pins."""
    try:
        base_config = load_execution_config(base_execution_config_path)
    except Tc5CharacterizationError as exc:
        raise Tc5LiveExecutorError(str(exc)) from exc
    document = _load_json(path, "live executor configuration", external=False)
    if set(document) != _LIVE_CONFIG_KEYS:
        raise Tc5LiveExecutorError("live executor configuration keys do not match tc5-live-executor-config.v1")
    if document["schema_version"] != TC5_LIVE_EXECUTOR_CONFIG_V1:
        raise Tc5LiveExecutorError("live executor configuration schema_version does not match")
    if document["program_track"] != PROGRAM_TRACK:
        raise Tc5LiveExecutorError("live executor configuration program_track must be SCALE-01")
    executor_id = _require_identifier(document["executor_id"], "executor_id")
    if document["execution_config_sha256"] != _execution_config_sha256(base_execution_config_path):
        raise Tc5LiveExecutorError("live executor configuration does not bind the frozen execution configuration")
    arms = tuple((str(row.get("name")), row.get("document_count")) for row in document["arms"] if isinstance(row, dict)) if isinstance(document["arms"], list) else ()
    if arms != _frozen_arms(base_config) or arms != _ARMS:
        raise Tc5LiveExecutorError("live executor configuration arms do not match bridge then primary")
    actions = document["approved_actions"]
    if not isinstance(actions, list) or tuple(actions) != _ACTIONS:
        raise Tc5LiveExecutorError("live executor configuration approved_actions do not match TC-5 scope")
    if document["result_contract"] != _RESULT_CONTRACT:
        raise Tc5LiveExecutorError("live executor configuration result_contract does not match safe result boundary")
    execution_config_sha256 = _execution_config_sha256(base_execution_config_path)
    return Tc5LiveExecutorConfig(
        executor_id=executor_id,
        config_sha256=hashlib.sha256(_canonical_json(document)).hexdigest(),
        execution_config_sha256=execution_config_sha256,
        arms=_ARMS,
        approved_actions=_ACTIONS,
    )


def _current_integrated_sha() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=_repository_root(),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=15,
    )
    value = result.stdout.strip()
    if result.returncode != 0 or _GIT_SHA.fullmatch(value) is None:
        raise Tc5LiveExecutorError("current checkout does not expose an integrated Git SHA")
    return value


def _parse_expiration(value: object) -> datetime:
    if not isinstance(value, str):
        raise Tc5LiveExecutorError("release expires_at must be an ISO-8601 UTC timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise Tc5LiveExecutorError("release expires_at must be an ISO-8601 UTC timestamp") from exc
    if parsed.tzinfo is None:
        raise Tc5LiveExecutorError("release expires_at must include UTC offset")
    return parsed.astimezone(UTC)


def _load_release(
    path: str | Path,
    *,
    config: Tc5LiveExecutorConfig,
    manifest: Tc5Manifest,
    action: str,
) -> Tc5ReleaseRecord:
    document = _load_json(path, "release record", external=True)
    if set(document) != _RELEASE_KEYS:
        raise Tc5LiveExecutorError("release record keys do not match tc5-execution-release.v1")
    if document["schema_version"] != TC5_RELEASE_RECORD_V1 or document["program_track"] != PROGRAM_TRACK:
        raise Tc5LiveExecutorError("release record does not identify SCALE-01 tc5-execution-release.v1")
    release_id = _require_identifier(document["release_id"], "release_id")
    if document["issued_by"] != "track-runner-coordinator":
        raise Tc5LiveExecutorError("release record must be issued by track-runner-coordinator")
    released_sha = document["integrated_sha"]
    if not isinstance(released_sha, str) or _GIT_SHA.fullmatch(released_sha) is None:
        raise Tc5LiveExecutorError("release record integrated_sha must be a lowercase Git SHA")
    integrity_document = {key: value for key, value in document.items() if key != "release_sha256"}
    if document["release_sha256"] != hashlib.sha256(_canonical_json(integrity_document)).hexdigest():
        raise Tc5LiveExecutorError("release_sha256 does not match the release record")
    if released_sha != _current_integrated_sha():
        raise Tc5LiveExecutorError("release record integrated_sha does not match this checkout")
    if document["live_config_sha256"] != config.config_sha256:
        raise Tc5LiveExecutorError("release record does not match the live executor configuration")
    if document["manifest_sha256"] != manifest.manifest_sha256:
        raise Tc5LiveExecutorError("release record does not match the qualified manifest")
    actions = document["approved_actions"]
    if not isinstance(actions, list) or not all(item in config.approved_actions for item in actions):
        raise Tc5LiveExecutorError("release record approved_actions do not match TC-5 scope")
    if action not in actions:
        raise Tc5LiveExecutorError(f"action {action} is not approved by the release record")
    if _parse_expiration(document["expires_at"]) <= datetime.now(UTC):
        raise Tc5LiveExecutorError("release record is stale")
    argv = document["runner_argv"]
    if not isinstance(argv, list) or not argv or not all(isinstance(item, str) and item for item in argv):
        raise Tc5LiveExecutorError("release record runner_argv must be a non-empty string list")
    executable = Path(argv[0]).resolve()
    if executable.is_relative_to(_repository_root()) or not executable.is_file():
        raise Tc5LiveExecutorError("release runner must be an existing external file")
    runner_sha256 = _require_sha256(document["runner_sha256"], "release record runner_sha256")
    if runner_sha256 != hashlib.sha256(executable.read_bytes()).hexdigest():
        raise Tc5LiveExecutorError("release record runner_sha256 does not match the external runner")
    return Tc5ReleaseRecord(
        release_id=release_id,
        integrated_sha=released_sha,
        approved_actions=tuple(actions),
        runner_argv=tuple(argv),
        record_sha256=_require_sha256(document["release_sha256"], "release record release_sha256"),
        runner_sha256=runner_sha256,
    )


def _result_provenance(manifest: Tc5Manifest) -> dict[str, object]:
    provenance = manifest.provenance
    return {
        "source_artifact_sha256": manifest.source_artifact_sha256,
        "source_commit": provenance["source_commit"],
        "cargo_lock_sha256": provenance["cargo_lock_sha256"],
        "model_asset_sha256": provenance["model_asset_sha256"],
        "rust_version": provenance["rust_version"],
        "engine_features": list(provenance["engine_features"]),
        "cpu_identity": provenance["cpu_identity"],
        "os_identity": provenance["os_identity"],
        "embed_device": "cpu",
        "model_identity": "fathomdb-bge-small-en-v1.5",
        "candidate_breadth": CANDIDATE_BREADTH,
        "query_count": QUERY_COUNT,
        "query_select_seed": QUERY_SELECT_SEED,
        "bootstrap_seed": BOOTSTRAP_SEED,
        "ground_truth": "exact-f32-same-model-top-10",
        "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }


def _safe_metric(value: object, label: str) -> float:
    if (
        not isinstance(value, (int, float))
        or isinstance(value, bool)
        or not math.isfinite(float(value))
        or not 0.0 <= float(value) <= 1.0
    ):
        raise Tc5LiveExecutorError(f"{label} must be a finite probability")
    return float(value)


def _validate_arm_result(
    path: Path,
    *,
    action: str,
    arm: str,
    document_count: int,
    manifest: Tc5Manifest,
) -> dict[str, object]:
    result = _load_json(path, "arm result", external=True)
    if "scale_02_claim" in result or "scale-02" in json.dumps(result).lower():
        raise Tc5LiveExecutorError("arm result must not contain a SCALE-02 claim")
    result_keys = set(result)
    if result_keys != _ARM_RESULT_KEYS:
        raise Tc5LiveExecutorError(
            "arm result keys do not match tc5-arm-result.v1: "
            f"missing={sorted(_ARM_RESULT_KEYS - result_keys)}, "
            f"unknown={sorted(result_keys - _ARM_RESULT_KEYS)}"
        )
    if result["schema_version"] != TC5_ARM_RESULT_V1 or result["program_track"] != PROGRAM_TRACK:
        raise Tc5LiveExecutorError("arm result schema does not identify SCALE-01")
    if result["action"] != action or result["arm"] != arm:
        raise Tc5LiveExecutorError("arm result action or arm does not match released invocation")
    if result["document_count"] != document_count or result["synthetic_document_count"] != 0:
        raise Tc5LiveExecutorError("arm result document_count proves substitution or padding")
    if result["manifest_sha256"] != manifest.manifest_sha256:
        raise Tc5LiveExecutorError("arm result manifest provenance drift")
    _require_sha256(result["ground_truth_sha256"], "arm result ground_truth_sha256")
    _require_sha256(result["sut_result_sha256"], "arm result sut_result_sha256")
    if result["query_completion_count"] != QUERY_COUNT:
        raise Tc5LiveExecutorError("arm result query_completion_count is partial")
    if result["bootstrap_resamples"] != BOOTSTRAP_RESAMPLES:
        raise Tc5LiveExecutorError("arm result bootstrap_resamples is incomplete")
    metrics = result["metrics"]
    if not isinstance(metrics, dict) or set(metrics) != {"recall_at_10", "ci_95", "bootstrap_sigma"}:
        raise Tc5LiveExecutorError("arm result metrics do not match the safe aggregate contract")
    recall = _safe_metric(metrics["recall_at_10"], "arm result recall_at_10")
    ci = metrics["ci_95"]
    if not isinstance(ci, list) or len(ci) != 2:
        raise Tc5LiveExecutorError("arm result ci_95 must have two probabilities")
    ci_low, ci_high = (_safe_metric(value, "arm result ci_95") for value in ci)
    if ci_low > ci_high or not ci_low <= recall <= ci_high:
        raise Tc5LiveExecutorError("arm result ci_95 does not contain recall_at_10")
    sigma = metrics["bootstrap_sigma"]
    if (
        not isinstance(sigma, (int, float))
        or isinstance(sigma, bool)
        or not math.isfinite(float(sigma))
        or float(sigma) < 0.0
    ):
        raise Tc5LiveExecutorError("arm result bootstrap_sigma must be finite and non-negative")
    expected_provenance = _result_provenance(manifest)
    if not isinstance(result["provenance"], dict) or set(result["provenance"]) != set(expected_provenance):
        raise Tc5LiveExecutorError("arm result input provenance keys drift")
    engine_features = result["provenance"].get("engine_features")
    if not isinstance(engine_features, list) or engine_features != sorted(engine_features):
        raise Tc5LiveExecutorError("arm result provenance engine_features drift")
    for field, expected in expected_provenance.items():
        if result["provenance"][field] != expected:
            raise Tc5LiveExecutorError(f"arm result provenance {field} drift")
    return {
        "name": arm,
        "document_count": document_count,
        "ground_truth_sha256": result["ground_truth_sha256"],
        "sut_result_sha256": result["sut_result_sha256"],
        "query_completion_count": QUERY_COUNT,
        "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
        "synthetic_document_count": 0,
        "metrics": {
            "recall_at_10": recall,
            "ci_95": [ci_low, ci_high],
            "bootstrap_sigma": float(sigma),
        },
    }


def _run_arm(
    release: Tc5ReleaseRecord,
    *,
    action: str,
    arm: str,
    document_count: int,
    manifest_path: Path,
    manifest: Tc5Manifest,
    corpus_root: Path,
    output_root: Path,
) -> dict[str, object]:
    arm_directory = _safe_external_output_path(output_root, action, arm, label="arm directory")
    destination = _safe_external_output_path(
        output_root, action, arm, "result.json", label="arm result destination"
    )
    if destination.exists():
        return _validate_arm_result(
            destination, action=action, arm=arm, document_count=document_count, manifest=manifest
        )
    arm_directory.mkdir(parents=True, exist_ok=True)
    destination = _safe_external_output_path(
        output_root, action, arm, "result.json", label="arm result destination"
    )
    environment = dict(os.environ)
    environment.update(
        {
            "TC5_ACTION": action,
            "TC5_ARM": arm,
            "TC5_DOCUMENT_COUNT": str(document_count),
            "TC5_MANIFEST_PATH": str(manifest_path),
            "TC5_MANIFEST_SHA256": manifest.manifest_sha256,
            "TC5_CORPUS_ROOT": str(corpus_root),
            "TC5_OUTPUT_ROOT": str(output_root),
            "TC5_ARM_RESULT_PATH": str(destination),
            "TC5_EMBED_DEVICE": "cpu",
            "TC5_MODEL_IDENTITY": "fathomdb-bge-small-en-v1.5",
            "TC5_MODEL_ASSET_SHA256": str(manifest.provenance["model_asset_sha256"]),
            "TC5_CANDIDATE_BREADTH": str(CANDIDATE_BREADTH),
            "TC5_QUERY_COUNT": str(QUERY_COUNT),
            "TC5_QUERY_SELECT_SEED": QUERY_SELECT_SEED,
            "TC5_BOOTSTRAP_RESAMPLES": str(BOOTSTRAP_RESAMPLES),
            "TC5_BOOTSTRAP_SEED": BOOTSTRAP_SEED,
            "TC5_GROUND_TRUTH": "exact-f32-same-model-top-10",
            "TC5_SUT": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        }
    )
    try:
        completed = subprocess.run(
            release.runner_argv,
            cwd=output_root,
            env=environment,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=86_400,
        )
    except OSError as exc:
        raise Tc5LiveExecutorError(f"external arm runner could not start: {exc}") from exc
    if completed.returncode != 0:
        raise Tc5LiveExecutorError(f"external arm runner failed for {arm}")
    destination = _safe_external_output_path(
        output_root, action, arm, "result.json", label="arm result destination"
    )
    return _validate_arm_result(
        destination, action=action, arm=arm, document_count=document_count, manifest=manifest
    )


def _receipt(
    release: Tc5ReleaseRecord,
    config: Tc5LiveExecutorConfig,
    manifest: Tc5Manifest,
    *,
    action: str,
    corpus_root: Path,
    output_root: Path,
    arms: Sequence[dict[str, object]],
) -> dict[str, object]:
    return {
        "schema_version": TC5_LIVE_EXECUTOR_RECEIPT_V1,
        "program_track": PROGRAM_TRACK,
        "release_id": release.release_id,
        "integrated_sha": release.integrated_sha,
        "executor_id": config.executor_id,
        "action": action,
        "manifest_id": manifest.manifest_id,
        "manifest_sha256": manifest.manifest_sha256,
        "artifact_refs": {
            "corpus_root_sha256": hashlib.sha256(str(corpus_root).encode("utf-8")).hexdigest(),
            "output_root_sha256": hashlib.sha256(str(output_root).encode("utf-8")).hexdigest(),
        },
        "input_digests": {
            "live_executor_config_sha256": config.config_sha256,
            "execution_config_sha256": config.execution_config_sha256,
            "release_record_sha256": release.record_sha256,
            "runner_sha256": release.runner_sha256,
        },
        "frozen_configuration": {
            "embed_device": "cpu",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "model_asset_sha256": manifest.provenance["model_asset_sha256"],
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
        "arms": list(arms),
        "experiment_index_projection": {
            "schema_version": "experiments.index-row.v1",
            "append_status": "not_appended",
            "eligibility": "eligible_after_complete_safe_two_arm_receipt",
        },
    }


def execute_tc5(
    *,
    action: str,
    release_path: str | Path,
    live_config_path: str | Path,
    base_execution_config_path: str | Path,
    manifest_path: str | Path,
    corpus_root: str | Path,
    output_root: str | Path,
) -> Path:
    """Execute released bridge then primary arms and write one safe receipt.

    The caller must provide an independently reviewed, coordinator-issued
    external release record.  This function never appends ``experiments/index``;
    that remains a coordinator operation after reviewing the complete receipt.
    """
    if action not in _ACTIONS:
        raise Tc5LiveExecutorError("action is outside the frozen TC-5 scope")
    config = load_live_executor_config(
        live_config_path, base_execution_config_path=base_execution_config_path
    )
    try:
        manifest = load_manifest(manifest_path)
    except Tc5ManifestError as exc:
        raise Tc5LiveExecutorError(str(exc)) from exc
    manifest_file = Path(manifest_path).resolve()
    corpus = _external_directory(corpus_root, "corpus root")
    output = _external_directory(output_root, "output root")
    release = _load_release(release_path, config=config, manifest=manifest, action=action)
    receipt_destination = _safe_external_output_path(
        output, f"tc5-{action}-receipt.json", label="receipt destination"
    )
    arm_results = [
        _run_arm(
            release,
            action=action,
            arm=arm,
            document_count=document_count,
            manifest_path=manifest_file,
            manifest=manifest,
            corpus_root=corpus,
            output_root=output,
        )
        for arm, document_count in config.arms
    ]
    receipt = _receipt(
        release,
        config,
        manifest,
        action=action,
        corpus_root=corpus,
        output_root=output,
        arms=arm_results,
    )
    receipt_destination = _safe_external_output_path(
        output, f"tc5-{action}-receipt.json", label="receipt destination"
    )
    receipt_destination.write_bytes(_canonical_json(receipt) + b"\n")
    return receipt_destination


def main(argv: Sequence[str] | None = None) -> int:
    """Run a released TC-5 invocation from explicit, external paths only."""
    parser = argparse.ArgumentParser(description="SCALE-01 TC-5 released external executor")
    parser.add_argument("--action", choices=_ACTIONS, required=True)
    parser.add_argument("--release", required=True)
    parser.add_argument("--live-config", required=True)
    parser.add_argument("--execution-config", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--corpus-root", required=True)
    parser.add_argument("--output-root", required=True)
    args = parser.parse_args(argv)
    receipt = execute_tc5(
        action=args.action,
        release_path=args.release,
        live_config_path=args.live_config,
        base_execution_config_path=args.execution_config,
        manifest_path=args.manifest,
        corpus_root=args.corpus_root,
        output_root=args.output_root,
    )
    print(receipt)
    return 0


if __name__ == "__main__":  # pragma: no cover - CLI wrapper
    raise SystemExit(main())
