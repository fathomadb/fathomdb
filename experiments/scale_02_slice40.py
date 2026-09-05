"""Run the preregistered Slice 40 write, storage, reopen, and status campaign."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import os
import random
import shutil
import sqlite3
import statistics
import subprocess
import sys
import tarfile
import time
import zipfile
from dataclasses import asdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib, measurement_classification, scale_02
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA = "scale-02-slice40.v3"
PROGRAM_TRACK = "SCALE-02"
REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE_COMMIT = "0aff1cb08c61a8bb2a004813bbd5604b6ff1a403"
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "campaign",
    "approval",
    "baseline",
    "candidate",
    "execution",
    "measurement_plan",
    "workload",
    "policy",
    "artifact_root",
    "claim_boundary",
}
_EXECUTION_FILES = {
    "runner_sha256": Path(__file__),
    "record_library_sha256": REPO_ROOT / "experiments/_lib.py",
    "statistics_library_sha256": REPO_ROOT / "experiments/scale_02.py",
    "classification_library_sha256": REPO_ROOT
    / "experiments/measurement_classification.py",
    "common_worker_sha256": REPO_ROOT
    / "experiments/rust/slice40_common_measurement.rs",
    "status_test_sha256": REPO_ROOT
    / "src/rust/crates/fathomdb-engine/tests/slice40_status_performance.rs",
}
_THROUGHPUT_EXECUTION_FILES = {
    "runner_sha256": Path(__file__),
    "record_library_sha256": REPO_ROOT / "experiments/_lib.py",
    "classification_library_sha256": REPO_ROOT
    / "experiments/measurement_classification.py",
    "test_setup_library_sha256": REPO_ROOT / "experiments/fathomdb_test_setup.py",
}
_THROUGHPUT_SCHEMA = "scale-02-slice40-cuda-throughput.v2"
_THROUGHPUT_PRODUCT_COMMIT = "2313fd34ea5ca68346a468d5bba58ba245306c08"
_THROUGHPUT_BUILD_COMMIT = "47afff50ec0e9f906c24c4422186eb407b2ff467"
_THROUGHPUT_CLI_SOURCE_COMMIT = "92faf4b693ad1c5ea6f76acb0f883d9e112793b4"
_THROUGHPUT_CLI_SHA256 = (
    "fd2e26a487b48d8d18d0ed7bdc91497aa3e43ab3d57cb4abdea8b89bde546df9"
)
_THROUGHPUT_UUID = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"
_THROUGHPUT_WORKLOAD = {
    "records": 1024,
    "write_batch_size": 128,
    "repetitions": 5,
    "warmup_records": 64,
    "drain_timeout_seconds": 900,
}
_THROUGHPUT_RUNTIME = {
    "embed_device": "cuda:0",
    "cuda_uuid": _THROUGHPUT_UUID,
    "embedder": {
        "name": "fathomdb-bge-small-en-v1.5",
        "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
        "dimension": 384,
    },
    "network": "offline_environment",
}
_THROUGHPUT_POLICY = {
    "classification": "descriptive",
    "decision_threshold": None,
    "excluded_time": ["artifact_install", "model_load", "model_warmup"],
    "included_time": ["configure_projection", "projection_backfill", "drain"],
}
_WORKLOAD = {
    "records": 10_000,
    "batch_size": 128,
    "expected_receipts": 79,
    "repetitions": 5,
    "status_records": 50_000,
    "status_warmups": 100,
    "status_samples": 1_000,
    "open_warmups": 5,
    "open_samples": 20,
    "bootstrap_seed": "0x5CA1E025400001",
    "bootstrap_resamples": 2_000,
}
_POLICY = {
    "confidence": 0.95,
    "write_max_relative_regression": 0.03,
    "open_max_relative_regression": 0.10,
    "open_max_absolute_regression_ms": 25.0,
    "status_p95_ms": 5.0,
    "status_p99_ms": 10.0,
    "storage_max_increase_bytes": 65_536,
    "cpu_cuda_status_p95_max_difference_ms": 2.0,
}
_STATUS_RESULT_KEYS = {
    "schema_version",
    "build_variant",
    "status_compute_device",
    "measurement_embedder",
    "records",
    "warmups",
    "samples",
    "errors",
    "timeouts",
    "cold_generation_ms",
    "cold_mutation_ms",
    "configuration_transition_ms",
    "post_transition_generation_ms",
    "post_transition_mutation_ms",
    "generation_p95_ms",
    "generation_p99_ms",
    "mutation_p95_ms",
    "mutation_p99_ms",
    "steady_full_owner_scans",
    "epoch_rollover_full_owner_scans",
    "query_plans",
}


class Slice40ScaleError(ValueError):
    """Raised when the Slice 40 contract or observed evidence is invalid."""


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Slice40ScaleError(
            f"{label} keys drifted: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _commit(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != 40
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Slice40ScaleError(f"{label} must be a full lowercase commit")
    return value


def _digest(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != 64
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Slice40ScaleError(f"{label} must be a lowercase SHA-256 digest")
    return value


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def resolve_config(
    document: object, *, validate_repository: bool = True
) -> dict[str, Any]:
    """Validate the exact preregistered Slice 40 campaign contract."""
    root = _exact(document, "config", _TOP_KEYS)
    if (
        root["schema_version"] != SCHEMA
        or root["program_track"] != PROGRAM_TRACK
        or root["release"] != "0.8.25"
        or root["claim_boundary"] != "slice40_projection_generation_readiness"
    ):
        raise Slice40ScaleError("configuration identity drifted")
    if root["campaign"] not in {"common", "status"}:
        raise Slice40ScaleError("campaign must be common or status")
    approval = _exact(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] != "approved" or approval["approved_by"] != "HITL":
        raise Slice40ScaleError("execution is not HITL-approved")
    baseline = _exact(root["baseline"], "baseline", {"source_commit"})
    candidate = _exact(root["candidate"], "candidate", {"source_commit"})
    execution = _exact(root["execution"], "execution", set(_EXECUTION_FILES))
    if _commit(baseline["source_commit"], "baseline.source_commit") != BASELINE_COMMIT:
        raise Slice40ScaleError("baseline source drifted")
    _commit(candidate["source_commit"], "candidate.source_commit")
    for label, digest in execution.items():
        _digest(digest, f"execution.{label}")
    plan = _exact(
        root["measurement_plan"],
        "measurement_plan",
        {"path", "sha256", "plan_id"},
    )
    _digest(plan["sha256"], "measurement_plan.sha256")
    if plan["plan_id"] != f"slice40-{root['campaign']}-v1":
        raise Slice40ScaleError("measurement_plan.plan_id does not match campaign")
    workload = _exact(root["workload"], "workload", set(_WORKLOAD))
    if workload != _WORKLOAD:
        raise Slice40ScaleError("workload drifted from the registered matrix")
    policy = _exact(root["policy"], "policy", set(_POLICY))
    if policy != _POLICY:
        raise Slice40ScaleError("policy drifted from the registered limits")
    artifact_root = root["artifact_root"]
    if not isinstance(artifact_root, str) or not Path(artifact_root).is_absolute():
        raise Slice40ScaleError(
            "artifact_root must be absolute and outside the repository"
        )
    if Path(artifact_root).resolve().is_relative_to(REPO_ROOT):
        raise Slice40ScaleError("artifact_root must remain outside the repository")
    if validate_repository:
        for label, commit in (
            ("baseline", baseline["source_commit"]),
            ("candidate", candidate["source_commit"]),
        ):
            check = subprocess.run(
                ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
                cwd=REPO_ROOT,
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            if check.returncode != 0:
                raise Slice40ScaleError(f"{label} source commit is unavailable")
        for label, path in _EXECUTION_FILES.items():
            if _sha256(path) != execution[label]:
                raise Slice40ScaleError(
                    f"execution.{label} does not match the reviewed source"
                )
        plan_path = (REPO_ROOT / plan["path"]).resolve()
        if not plan_path.is_relative_to(REPO_ROOT):
            raise Slice40ScaleError("measurement plan must be repository-local")
        try:
            plan_document = json.loads(plan_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise Slice40ScaleError("measurement plan is unavailable") from exc
        if (
            measurement_classification.canonical_sha256(plan_document) != plan["sha256"]
            or plan_document.get("plan_id") != plan["plan_id"]
        ):
            raise Slice40ScaleError("measurement plan reference drifted")
    return root


def require_clean_execution(
    config: Mapping[str, Any], *, require_candidate_product_tree: bool
) -> dict[str, Any]:
    """Fail closed on dirty, unreviewed, or product-drifted execution source."""
    code = _lib.git_info()
    if code["dirty"]:
        raise Slice40ScaleError("campaign requires a clean execution checkout")
    for label, path in _EXECUTION_FILES.items():
        if _sha256(path) != config["execution"][label]:
            raise Slice40ScaleError(f"execution.{label} drifted after review")
    if require_candidate_product_tree:
        comparison = subprocess.run(
            [
                "git",
                "diff",
                "--quiet",
                config["candidate"]["source_commit"],
                "HEAD",
                "--",
                "Cargo.toml",
                "Cargo.lock",
                "src/rust",
            ],
            cwd=REPO_ROOT,
            check=False,
        )
        if comparison.returncode != 0:
            raise Slice40ScaleError(
                "status execution product tree differs from the reviewed candidate"
            )
    return code


def load_config(
    path: str | Path, *, validate_repository: bool = True
) -> dict[str, Any]:
    """Load a Slice 40 campaign contract."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Slice40ScaleError("configuration is unavailable") from exc
    return resolve_config(document, validate_repository=validate_repository)


def resolve_cuda_throughput_config(
    document: object, *, validate_repository: bool = True
) -> dict[str, Any]:
    """Validate the closed Slice 40 CUDA projection-throughput campaign."""

    keys = {
        "schema_version",
        "program_track",
        "release",
        "campaign",
        "approval",
        "candidate",
        "execution",
        "measurement_plan",
        "workload",
        "runtime",
        "policy",
        "artifact_root",
        "claim_boundary",
    }
    root = _exact(document, "throughput config", keys)
    if (
        root["schema_version"] != _THROUGHPUT_SCHEMA
        or root["program_track"] != PROGRAM_TRACK
        or root["release"] != "0.8.25"
        or root["campaign"] != "cuda-throughput"
        or root["claim_boundary"]
        != "cuda_projection_backfill_throughput_descriptive"
    ):
        raise Slice40ScaleError("throughput configuration identity drifted")
    approval = _exact(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] != "approved" or approval["approved_by"] != "HITL":
        raise Slice40ScaleError("throughput execution is not HITL-approved")
    candidate = _exact(
        root["candidate"],
        "candidate",
        {
            "product_commit",
            "artifact_build_commit",
            "wheel_sha256",
            "cli_source_commit",
            "cli_sha256",
        },
    )
    if (
        candidate["product_commit"] != _THROUGHPUT_PRODUCT_COMMIT
        or candidate["artifact_build_commit"] != _THROUGHPUT_BUILD_COMMIT
        or candidate["cli_source_commit"] != _THROUGHPUT_CLI_SOURCE_COMMIT
        or candidate["cli_sha256"] != _THROUGHPUT_CLI_SHA256
    ):
        raise Slice40ScaleError("throughput candidate boundary drifted")
    _commit(candidate["cli_source_commit"], "candidate.cli_source_commit")
    _digest(candidate["wheel_sha256"], "candidate.wheel_sha256")
    _digest(candidate["cli_sha256"], "candidate.cli_sha256")
    execution = _exact(
        root["execution"], "execution", set(_THROUGHPUT_EXECUTION_FILES)
    )
    for label, digest in execution.items():
        _digest(digest, f"execution.{label}")
    plan = _exact(
        root["measurement_plan"],
        "measurement_plan",
        {"path", "sha256", "plan_id"},
    )
    _digest(plan["sha256"], "measurement_plan.sha256")
    if plan["plan_id"] != "slice40-cuda-throughput-v2":
        raise Slice40ScaleError("throughput measurement-plan ID drifted")
    workload = _exact(
        root["workload"], "workload", set(_THROUGHPUT_WORKLOAD)
    )
    if workload != _THROUGHPUT_WORKLOAD:
        raise Slice40ScaleError("workload drifted from the registered matrix")
    runtime = _exact(root["runtime"], "runtime", set(_THROUGHPUT_RUNTIME))
    if runtime != _THROUGHPUT_RUNTIME:
        raise Slice40ScaleError("runtime drifted from the registered CUDA policy")
    policy = _exact(root["policy"], "policy", set(_THROUGHPUT_POLICY))
    if policy != _THROUGHPUT_POLICY:
        raise Slice40ScaleError("policy drifted from the descriptive contract")
    artifact_root = root["artifact_root"]
    if not isinstance(artifact_root, str) or not Path(artifact_root).is_absolute():
        raise Slice40ScaleError("artifact_root must be absolute")
    if Path(artifact_root).resolve().is_relative_to(REPO_ROOT):
        raise Slice40ScaleError("artifact_root must remain outside the repository")
    if validate_repository:
        for label, path in _THROUGHPUT_EXECUTION_FILES.items():
            if _sha256(path) != execution[label]:
                raise Slice40ScaleError(f"execution.{label} drifted")
        plan_path = (REPO_ROOT / plan["path"]).resolve()
        if not plan_path.is_relative_to(REPO_ROOT):
            raise Slice40ScaleError("measurement plan must be repository-local")
        try:
            authority = json.loads(plan_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise Slice40ScaleError("measurement plan is unavailable") from exc
        if (
            measurement_classification.canonical_sha256(authority) != plan["sha256"]
            or authority.get("plan_id") != plan["plan_id"]
        ):
            raise Slice40ScaleError("measurement plan reference drifted")
    return root


def load_cuda_throughput_config(
    path: str | Path, *, validate_repository: bool = True
) -> dict[str, Any]:
    """Load a Slice 40 CUDA projection-throughput contract."""

    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Slice40ScaleError("throughput configuration is unavailable") from exc
    return resolve_cuda_throughput_config(
        document, validate_repository=validate_repository
    )


def summarize_cuda_throughput_observations(
    observations: Sequence[Mapping[str, object]], config: Mapping[str, Any]
) -> dict[str, object]:
    """Validate complete new-vector observations and return descriptive rates."""

    workload = config["workload"]
    runtime = config["runtime"]
    if len(observations) != workload["repetitions"]:
        raise Slice40ScaleError("throughput repetition count drifted")
    rates: list[float] = []
    repetitions: set[int] = set()
    for observation in observations:
        repetition = observation.get("repetition")
        if not isinstance(repetition, int) or repetition in repetitions:
            raise Slice40ScaleError("throughput repetition IDs must be unique")
        repetitions.add(repetition)
        if observation.get("selected_device") != "cuda:0":
            raise Slice40ScaleError("every throughput cell must execute on cuda:0")
        if observation.get("selected_cuda_uuid") != runtime["cuda_uuid"]:
            raise Slice40ScaleError("throughput selected CUDA UUID drifted")
        if observation.get("records") != workload["records"]:
            raise Slice40ScaleError("throughput record count drifted")
        if observation.get("pre_vector_rows") != 0:
            raise Slice40ScaleError("pre_vector_rows must be zero before timing")
        delta = observation.get("projection_delta")
        if not isinstance(delta, dict) or (
            delta.get("built") != ["slice40_throughput"]
            or delta.get("deferred") != ["slice40_throughput"]
            or delta.get("unchanged") is not False
        ):
            raise Slice40ScaleError(
                "projection configuration must be a non-noop built transition"
            )
        if observation.get("vector_rows") != workload["records"]:
            raise Slice40ScaleError("vector_rows did not reach the record count")
        if observation.get("readiness") != "ready":
            raise Slice40ScaleError("throughput projection did not reach ready")
        rate = observation.get("records_per_second")
        elapsed = observation.get("elapsed_seconds")
        if (
            isinstance(rate, bool)
            or not isinstance(rate, (int, float))
            or not math.isfinite(rate)
            or rate <= 0
        ):
            raise Slice40ScaleError("throughput rate must be finite and positive")
        if (
            isinstance(elapsed, bool)
            or not isinstance(elapsed, (int, float))
            or not math.isfinite(elapsed)
            or elapsed <= 0
        ):
            raise Slice40ScaleError("throughput elapsed time must be finite and positive")
        derived_rate = workload["records"] / float(elapsed)
        if not math.isclose(float(rate), derived_rate, rel_tol=1e-12, abs_tol=0.0):
            raise Slice40ScaleError(
                "reported throughput rate must equal records / elapsed_seconds"
            )
        rates.append(derived_rate)
    if repetitions != set(range(1, workload["repetitions"] + 1)):
        raise Slice40ScaleError("throughput repetition IDs must be exactly 1..5")
    return {
        "classification": "descriptive",
        "repetitions": workload["repetitions"],
        "records_per_repetition": workload["records"],
        "throughput_records_per_second": {
            "minimum": min(rates),
            "median": statistics.median(rates),
            "maximum": max(rates),
        },
        "verdict": "observed",
    }


def decision_metrics(campaign: str, detail: Mapping[str, Any]) -> dict[str, Any]:
    """Select the preregistered decision leaves from a complete raw result."""
    if campaign == "common":
        write = detail["write_upper_95_relative_regression"]
        return {
            "schema_version": "scale-02-slice40-decision.v1",
            "campaign": campaign,
            "verdict": detail["verdict"],
            "write_p50_upper_relative_regression": write["p50"],
            "write_p95_upper_relative_regression": write["p95"],
            "open_upper_relative_regression": detail[
                "open_upper_95_relative_regression"
            ],
            "open_mean_absolute_regression_ms": detail[
                "open_mean_absolute_regression_ms"
            ],
            "maximum_paired_storage_increase_bytes": detail[
                "maximum_paired_storage_increase_bytes"
            ],
        }
    if campaign == "status":
        bounds = detail["status_bounds"]
        return {
            "schema_version": "scale-02-slice40-decision.v1",
            "campaign": campaign,
            "verdict": detail["verdict"],
            "maximum_cpu_cuda_p95_difference_ms": detail[
                "maximum_cpu_cuda_p95_difference_ms"
            ],
            "maximum_generation_p95_ms": bounds["maximum_generation_p95_ms"],
            "maximum_generation_p99_ms": bounds["maximum_generation_p99_ms"],
            "maximum_mutation_p95_ms": bounds["maximum_mutation_p95_ms"],
            "maximum_mutation_p99_ms": bounds["maximum_mutation_p99_ms"],
            "maximum_steady_full_owner_scans": bounds[
                "maximum_steady_full_owner_scans"
            ],
            "total_errors": bounds["total_errors"],
            "total_timeouts": bounds["total_timeouts"],
        }
    if campaign == "cuda-throughput":
        return {
            "schema_version": "scale-02-slice40-decision.v1",
            "campaign": campaign,
            "classification": detail["classification"],
            "verdict": detail["verdict"],
            "records_per_repetition": detail["records_per_repetition"],
            "repetitions": detail["repetitions"],
            "throughput_records_per_second": detail[
                "throughput_records_per_second"
            ],
        }
    raise Slice40ScaleError("unknown campaign for decision metrics")


def _repository_locator(path: Path, label: str) -> str:
    resolved = path.resolve()
    if not resolved.is_relative_to(REPO_ROOT):
        raise Slice40ScaleError(f"{label} must be repository-local")
    return str(resolved.relative_to(REPO_ROOT))


def _git_artifact(commit: str, artifact_id: str, path: Path) -> dict[str, Any]:
    relative = _repository_locator(path, artifact_id)
    locator = f"{commit}:{relative}"
    completed = subprocess.run(
        ["git", "show", locator],
        cwd=REPO_ROOT,
        check=False,
        stdout=subprocess.PIPE,
    )
    if completed.returncode != 0:
        raise Slice40ScaleError(f"classification source is unavailable: {locator}")
    return {
        "id": artifact_id,
        "role": "implementation",
        "locator_kind": "git_blob",
        "locator": locator,
        "sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "measurement_root_json_pointers": [],
    }


def _write_run_classification(
    *,
    run_id: str,
    run_dir: Path,
    config: Mapping[str, Any],
    config_path: Path,
    code_git_sha: str,
    execution_files: Mapping[str, Path] = _EXECUTION_FILES,
    outcome: str = "complete",
    blocked_reason: Mapping[str, object] | None = None,
) -> None:
    plan_ref = config["measurement_plan"]
    plan_path = (REPO_ROOT / plan_ref["path"]).resolve()
    authority = json.loads(plan_path.read_text(encoding="utf-8"))
    artifacts = [
        {
            "id": "detail",
            "role": "derivation_spec",
            "locator_kind": "repository_path",
            "locator": _repository_locator(run_dir / "detail.json", "detail"),
            "sha256": _sha256(run_dir / "detail.json"),
            "measurement_root_json_pointers": [],
        },
        {
            "id": "record",
            "role": "record",
            "locator_kind": "repository_path",
            "locator": _repository_locator(run_dir / "record.json", "record"),
            "sha256": _sha256(run_dir / "record.json"),
            "measurement_root_json_pointers": [],
        },
        {
            "id": "configuration",
            "role": "configuration",
            "locator_kind": "repository_path",
            "locator": _repository_locator(config_path, "configuration"),
            "sha256": _sha256(config_path),
            "measurement_root_json_pointers": [],
        },
        {
            "id": "plan",
            "role": "configuration",
            "locator_kind": "repository_path",
            "locator": _repository_locator(plan_path, "plan"),
            "sha256": _sha256(plan_path),
            "measurement_root_json_pointers": [],
        },
        *[
            _git_artifact(code_git_sha, label.removesuffix("_sha256"), path)
            for label, path in execution_files.items()
        ],
    ]
    if outcome == "complete":
        artifacts.insert(
            0,
            {
                "id": "metrics",
                "role": "metrics_payload",
                "locator_kind": "repository_path",
                "locator": _repository_locator(run_dir / "metrics.json", "metrics"),
                "sha256": _sha256(run_dir / "metrics.json"),
                "measurement_root_json_pointers": authority["measurement_roots"][
                    0
                ]["json_pointers"],
            },
        )
    document = {
        "schema_version": measurement_classification.SCHEMA_VERSION,
        "classifier_version": measurement_classification.CLASSIFIER_VERSION,
        "classification_id": "",
        "run_id": run_id,
        "outcome": outcome,
        "blocked_reason": blocked_reason,
        "measurement_plan_id": authority["plan_id"],
        "source_artifacts": artifacts,
        "components": authority["components"],
        "call_paths": authority["call_paths"],
        "execution_witnesses": [],
        "metrics": authority["metric_bindings"] if outcome == "complete" else [],
        "metric_exclusions": authority["metric_exclusions"],
        "comparisons": authority["comparisons"],
        "claims": authority["claims"] if outcome == "complete" else [],
        "migration": {
            "kind": "native",
            "manifest_path": None,
            "manifest_entry_sha256": None,
            "measurement_plan_id": authority["plan_id"],
            "measurement_plan_sha256": plan_ref["sha256"],
        },
    }
    document["classification_id"] = measurement_classification.classification_id(
        document
    )
    measurement_classification.write_classification(
        run_dir,
        document,
        repository_root=REPO_ROOT,
        authority=authority,
    )


def _register_run(
    *,
    experiment: str,
    config: Mapping[str, Any],
    config_path: Path,
    decision_metrics: Mapping[str, Any],
    detail: Mapping[str, Any],
    record_arguments: Mapping[str, Any],
    code_git_sha: str,
    execution_files: Mapping[str, Path] = _EXECUTION_FILES,
    classification_outcome: str = "complete",
    blocked_reason: Mapping[str, object] | None = None,
) -> tuple[str, Path]:
    """Write detail and classification before making a run discoverable."""

    def before_index(run_id: str, run_dir: Path) -> None:
        (run_dir / "detail.json").write_text(
            json.dumps(detail, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        _write_run_classification(
            run_id=run_id,
            run_dir=run_dir,
            config=config,
            config_path=config_path,
            code_git_sha=code_git_sha,
            execution_files=execution_files,
            outcome=classification_outcome,
            blocked_reason=blocked_reason,
        )

    return _lib.write_record(
        experiment,
        config_obj=config,
        metrics=decision_metrics,
        config_path=_repository_locator(config_path, "configuration"),
        before_index=before_index,
        **record_arguments,
    )


def paired_relative_upper(
    baseline: Sequence[float],
    candidate: Sequence[float],
    *,
    seed: int,
    resamples: int,
) -> float:
    """Return the one-sided paired-bootstrap upper relative regression."""
    if len(baseline) != len(candidate) or not baseline:
        raise Slice40ScaleError("paired samples must be non-empty and equal length")
    ratios = [(new / old) - 1.0 for old, new in zip(baseline, candidate, strict=True)]
    randomizer = random.Random(seed)
    estimates = [
        statistics.fmean(ratios[randomizer.randrange(len(ratios))] for _ in ratios)
        for _ in range(resamples)
    ]
    return scale_02._percentile(estimates, 0.95)


def comparison_verdict(
    *,
    write_upper: Mapping[str, float],
    open_upper_relative: float,
    open_absolute_regression_ms: float,
    storage_increase_bytes: int,
    policy: Mapping[str, Any],
) -> str:
    """Apply all preregistered common-path comparison limits."""
    write_pass = all(
        value <= policy["write_max_relative_regression"]
        for value in write_upper.values()
    )
    open_pass = (
        open_upper_relative <= policy["open_max_relative_regression"]
        or open_absolute_regression_ms <= policy["open_max_absolute_regression_ms"]
    )
    storage_pass = storage_increase_bytes <= policy["storage_max_increase_bytes"]
    return "pass" if write_pass and open_pass and storage_pass else "fail"


def validate_worker_counts(
    result: Mapping[str, Any], workload: Mapping[str, Any]
) -> None:
    """Reject a worker that did not execute the exact registered workload."""
    expected = {
        "records": workload["records"],
        "batch_size": workload["batch_size"],
        "receipt_count": workload["expected_receipts"],
        "operation_count": workload["records"],
        "pending_receipt_count": workload["expected_receipts"],
        "pending_cursor_count": workload["records"],
        "errors": 0,
        "timeouts": 0,
    }
    if any(result.get(key) != value for key, value in expected.items()):
        raise Slice40ScaleError("worker counts drifted from the registered workload")


def parse_status_result(document: object) -> dict[str, Any]:
    """Validate one exact CPU or CUDA status-measurement result."""
    result = _exact(document, "status result", _STATUS_RESULT_KEYS)
    if result["schema_version"] != "scale-02-slice40-status.v2":
        raise Slice40ScaleError("status result schema drifted")
    if result["build_variant"] not in {"cpu", "cuda"}:
        raise Slice40ScaleError("status result build variant drifted")
    if result["status_compute_device"] != "cpu":
        raise Slice40ScaleError("SQLite status computation must be identified as CPU")
    if result["measurement_embedder"] != "fixed-test-cpu":
        raise Slice40ScaleError("status fixture embedder identity drifted")
    query_plans = result["query_plans"]
    if (
        not isinstance(query_plans, list)
        or not query_plans
        or any(not isinstance(value, str) or not value for value in query_plans)
    ):
        raise Slice40ScaleError("status query plans must be a non-empty string list")
    for field in (
        "records",
        "warmups",
        "samples",
        "errors",
        "timeouts",
        "steady_full_owner_scans",
        "epoch_rollover_full_owner_scans",
    ):
        value = result[field]
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise Slice40ScaleError(
                f"status result {field} must be a nonnegative integer"
            )
    for field in _STATUS_RESULT_KEYS - {
        "schema_version",
        "build_variant",
        "status_compute_device",
        "measurement_embedder",
        "query_plans",
        "records",
        "warmups",
        "samples",
        "errors",
        "timeouts",
        "steady_full_owner_scans",
        "epoch_rollover_full_owner_scans",
    }:
        value = result[field]
        if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
            raise Slice40ScaleError(f"status result {field} must be nonnegative")
    return result


def inspect_status_database(path: Path) -> dict[str, int]:
    """Return closed-database storage and retained-generation observations."""
    wal = path.with_name(path.name + "-wal")
    with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
        page_count = int(connection.execute("PRAGMA page_count").fetchone()[0])
        page_size = int(connection.execute("PRAGMA page_size").fetchone()[0])
        generations = int(
            connection.execute(
                "SELECT COUNT(*) FROM _fathomdb_projection_generations"
            ).fetchone()[0]
        )
        serving = int(
            connection.execute(
                "SELECT COUNT(*) FROM _fathomdb_projection_generations "
                "WHERE role='serving'"
            ).fetchone()[0]
        )
    return {
        "database_bytes": path.stat().st_size,
        "generation_history_rows": generations,
        "page_count": page_count,
        "page_size_bytes": page_size,
        "serving_generation_rows": serving,
        "wal_bytes": wal.stat().st_size if wal.exists() else 0,
    }


def status_verdict(
    cpu_runs: Sequence[object],
    cuda_runs: Sequence[object],
    workload: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> str:
    """Apply registered status bounds to five paired CPU/CUDA observations."""
    repetitions = workload["repetitions"]
    if len(cpu_runs) != repetitions or len(cuda_runs) != repetitions:
        raise Slice40ScaleError(
            "status repetitions drifted from the registered workload"
        )
    cpu = [parse_status_result(run) for run in cpu_runs]
    cuda = [parse_status_result(run) for run in cuda_runs]
    expected_shape = {
        "records": workload["status_records"],
        "warmups": workload["status_warmups"],
        "samples": workload["status_samples"],
        "errors": 0,
        "timeouts": 0,
    }
    for run, build_variant in [
        *((value, "cpu") for value in cpu),
        *((value, "cuda") for value in cuda),
    ]:
        if run["build_variant"] != build_variant or any(
            run[key] != value for key, value in expected_shape.items()
        ):
            raise Slice40ScaleError("status result counts or build variant drifted")
    bounded = all(
        run["generation_p95_ms"] <= policy["status_p95_ms"]
        and run["mutation_p95_ms"] <= policy["status_p95_ms"]
        and run["generation_p99_ms"] <= policy["status_p99_ms"]
        and run["mutation_p99_ms"] <= policy["status_p99_ms"]
        for run in [*cpu, *cuda]
    )
    device_difference = max(
        abs(left[field] - right[field])
        for left, right in zip(cpu, cuda, strict=True)
        for field in ("generation_p95_ms", "mutation_p95_ms")
    )
    return (
        "pass"
        if bounded
        and device_difference <= policy["cpu_cuda_status_p95_max_difference_ms"]
        else "fail"
    )


def _build_status_test(*, build_variant: str, run_root: Path) -> dict[str, str]:
    features = "test-hooks" if build_variant == "cpu" else "test-hooks,embed-cuda"
    target_dir = run_root / f"target-{build_variant}"
    log = run_root / f"build-{build_variant}.jsonl"
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--release",
            "--no-run",
            "--message-format=json-render-diagnostics",
            "-p",
            "fathomdb-engine",
            "--features",
            features,
            "--test",
            "slice40_status_performance",
        ],
        cwd=REPO_ROOT,
        env={**os.environ, "CARGO_TARGET_DIR": str(target_dir)},
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    log.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        raise Slice40ScaleError(f"status {build_variant} build failed; see {log}")
    executables = []
    for line in completed.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = message.get("target", {})
        executable = message.get("executable")
        if (
            message.get("reason") == "compiler-artifact"
            and target.get("name") == "slice40_status_performance"
            and isinstance(executable, str)
        ):
            executables.append(Path(executable))
    if len(executables) != 1 or not executables[0].is_file():
        raise Slice40ScaleError(
            f"status {build_variant} build did not identify exactly one test executable"
        )
    executable = executables[0]
    return {
        "build_variant": build_variant,
        "features": features,
        "path": str(executable),
        "sha256": _sha256(executable),
        "build_log": str(log),
        "build_log_sha256": _sha256(log),
    }


def _status_test_command(
    *,
    executable: Path,
    database: Path,
    device: str,
    workload: Mapping[str, Any],
    log: Path,
    seed_only: bool = False,
) -> dict[str, Any] | None:
    environment = {
        **os.environ,
        "FATHOM_SLICE40_STATUS_DATABASE": str(database),
        "FATHOM_SLICE40_STATUS_BUILD_VARIANT": "cpu" if device == "cpu" else "cuda",
        "FATHOM_SLICE40_STATUS_RECORDS": str(workload["status_records"]),
        "FATHOM_SLICE40_STATUS_WARMUPS": str(workload["status_warmups"]),
        "FATHOM_SLICE40_STATUS_SAMPLES": str(workload["status_samples"]),
    }
    if seed_only:
        environment["FATHOM_SLICE40_STATUS_SEED_ONLY"] = "1"
    completed = subprocess.run(
        [
            str(executable),
            "--ignored",
            "--nocapture",
        ],
        cwd=REPO_ROOT,
        env=environment,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    log.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        raise Slice40ScaleError(f"status worker failed; see {log}")
    result_lines = [
        line for line in completed.stdout.splitlines() if line.startswith("{")
    ]
    if seed_only:
        if result_lines:
            raise Slice40ScaleError("seed-only status worker emitted a measurement")
        return None
    if len(result_lines) != 1:
        raise Slice40ScaleError("status worker did not emit exactly one JSON result")
    try:
        return parse_status_result(json.loads(result_lines[0]))
    except json.JSONDecodeError as exc:
        raise Slice40ScaleError("status worker emitted malformed JSON") from exc


def run_status(config_path: str | Path) -> dict[str, Any]:
    """Execute the paired CPU/CUDA 50k status campaign and register its receipt."""
    config_path = Path(config_path).resolve()
    config = load_config(config_path)
    if config["campaign"] != "status":
        raise Slice40ScaleError("status command requires the status campaign")
    code = require_clean_execution(config, require_candidate_product_tree=True)
    nvidia = subprocess.run(
        ["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if nvidia.returncode != 0 or not nvidia.stdout.strip():
        raise Slice40ScaleError("CUDA device witness is unavailable")

    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    run_root = Path(config["artifact_root"]) / f"slice40-status-{stamp}"
    run_root.mkdir(parents=True, mode=0o700)
    nvidia_log = run_root / "nvidia-smi.log"
    nvidia_log.write_text(nvidia.stdout, encoding="utf-8")
    executables = {
        build_variant: _build_status_test(
            build_variant=build_variant, run_root=run_root
        )
        for build_variant in ("cpu", "cuda")
    }
    results: dict[str, list[dict[str, Any]]] = {"cpu": [], "cuda": []}
    database_observations: dict[str, list[dict[str, int]]] = {"cpu": [], "cuda": []}
    databases: list[dict[str, str]] = []
    for repetition in range(1, config["workload"]["repetitions"] + 1):
        repetition_root = run_root / f"repetition-{repetition}"
        repetition_root.mkdir()
        seed_database = repetition_root / "seed.fathomdb"
        _status_test_command(
            executable=Path(executables["cpu"]["path"]),
            database=seed_database,
            device="cpu",
            workload=config["workload"],
            log=repetition_root / "seed.log",
            seed_only=True,
        )
        for suffix in ("-wal", "-shm"):
            if seed_database.with_name(seed_database.name + suffix).exists():
                raise Slice40ScaleError(
                    "status seed retained an uncheckpointed SQLite sidecar"
                )
        cpu_database = repetition_root / "cpu.fathomdb"
        cuda_database = repetition_root / "cuda.fathomdb"
        shutil.copy2(seed_database, cpu_database)
        shutil.copy2(seed_database, cuda_database)
        if _sha256(cpu_database) != _sha256(cuda_database):
            raise Slice40ScaleError("paired status databases are not byte-identical")
        cpu = _status_test_command(
            executable=Path(executables["cpu"]["path"]),
            database=cpu_database,
            device="cpu",
            workload=config["workload"],
            log=repetition_root / "cpu.log",
        )
        cuda = _status_test_command(
            executable=Path(executables["cuda"]["path"]),
            database=cuda_database,
            device="cuda:0",
            workload=config["workload"],
            log=repetition_root / "cuda.log",
        )
        assert cpu is not None and cuda is not None
        results["cpu"].append(cpu)
        results["cuda"].append(cuda)
        database_observations["cpu"].append(inspect_status_database(cpu_database))
        database_observations["cuda"].append(inspect_status_database(cuda_database))
        databases.append(
            {
                "seed_sha256": _sha256(seed_database),
                "cpu_after_sha256": _sha256(cpu_database),
                "cuda_after_sha256": _sha256(cuda_database),
            }
        )

    verdict = status_verdict(
        results["cpu"], results["cuda"], config["workload"], config["policy"]
    )
    maximum_device_p95_difference_ms = max(
        abs(left[field] - right[field])
        for left, right in zip(results["cpu"], results["cuda"], strict=True)
        for field in ("generation_p95_ms", "mutation_p95_ms")
    )
    status_bounds = {
        "maximum_generation_p95_ms": max(
            result["generation_p95_ms"]
            for treatment in results.values()
            for result in treatment
        ),
        "maximum_generation_p99_ms": max(
            result["generation_p99_ms"]
            for treatment in results.values()
            for result in treatment
        ),
        "maximum_mutation_p95_ms": max(
            result["mutation_p95_ms"]
            for treatment in results.values()
            for result in treatment
        ),
        "maximum_mutation_p99_ms": max(
            result["mutation_p99_ms"]
            for treatment in results.values()
            for result in treatment
        ),
        "maximum_steady_full_owner_scans": max(
            result["steady_full_owner_scans"]
            for treatment in results.values()
            for result in treatment
        ),
        "total_errors": sum(
            result["errors"] for treatment in results.values() for result in treatment
        ),
        "total_timeouts": sum(
            result["timeouts"] for treatment in results.values() for result in treatment
        ),
    }
    metrics = {
        "schema_version": "scale-02-slice40-status-result.v1",
        "program_track": PROGRAM_TRACK,
        "claim_boundary": config["claim_boundary"],
        "verdict": verdict,
        "cuda_host_witness": nvidia.stdout.strip(),
        "measurement_boundary": {
            "status_compute_device": "cpu",
            "fixture_embedder": "fixed-test-cpu",
            "cuda_interpretation": "CUDA-linked build parity; real GPU execution is verified by the separate CUDA preflight and package smoke",
        },
        "maximum_cpu_cuda_p95_difference_ms": maximum_device_p95_difference_ms,
        "status_bounds": status_bounds,
        "treatments": results,
        "paired_database_hashes": databases,
        "database_observations": database_observations,
        "executed_test_artifacts": executables,
        "runner_sha256": _sha256(Path(__file__)),
        "status_test_source_sha256": _sha256(
            REPO_ROOT
            / "src/rust/crates/fathomdb-engine/tests/slice40_status_performance.rs"
        ),
    }
    metrics_path = run_root / "metrics.json"
    metrics_path.write_text(
        json.dumps(metrics, sort_keys=True) + "\n", encoding="utf-8"
    )
    run_id, record_dir = _register_run(
        experiment="scale-02-slice40-status",
        config=config,
        config_path=config_path,
        decision_metrics=decision_metrics("status", metrics),
        detail=metrics,
        code_git_sha=code["git_sha"],
        record_arguments={
            "ts": datetime.now(UTC),
            "verdict": "complete" if verdict == "pass" else "advisory_limit_observed",
            "read": f"Slice 40 paired CPU/CUDA status campaign: {verdict}",
            "code": code,
            "corpus": {
                "source": "byte-identical paired 50k status fixtures",
                "datasets": [],
            },
            "seeds": {"bootstrap": config["workload"]["bootstrap_seed"]},
            "env": _lib.env_info(
                key_deps={
                    "fathomdb_candidate": config["candidate"]["source_commit"],
                    "cuda_host_witness": nvidia.stdout.strip(),
                }
            ),
            "cost_usd": 0.0,
            "headline": {
                "verdict": verdict,
                "maximum_cpu_cuda_p95_difference_ms": maximum_device_p95_difference_ms,
            },
            "n": config["workload"]["status_records"],
            "tests": [
                "src/rust/crates/fathomdb-engine/tests/slice40_status_performance.rs",
                "tests/experiments/test_scale_02_slice40.py",
            ],
            "files_changed": [],
            "artifacts": [],
            "review": None,
            "open_questions": []
            if verdict == "pass"
            else ["A registered status bound missed"],
        },
    )
    _lib.regen_index_md()
    return {
        "run_id": run_id,
        "record_dir": str(record_dir),
        "external_metrics": str(metrics_path),
        "verdict": verdict,
    }


def _extract_source(commit: str, destination: Path) -> None:
    archive = subprocess.run(
        ["git", "archive", "--format=tar", commit],
        cwd=REPO_ROOT,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    destination.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as bundle:
        bundle.extractall(destination, filter="data")


def _build_worker(source: Path, treatment_root: Path) -> Path:
    helper = REPO_ROOT / "experiments/rust/slice40_common_measurement.rs"
    example = (
        source
        / "src/rust/crates/fathomdb-engine/examples/slice40_common_measurement.rs"
    )
    example.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(helper, example)
    target = treatment_root / "target"
    log = treatment_root / "build.log"
    completed = subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "fathomdb-engine",
            "--features",
            "test-hooks",
            "--example",
            "slice40_common_measurement",
        ],
        cwd=source,
        env={**__import__("os").environ, "CARGO_TARGET_DIR": str(target)},
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    log.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        raise Slice40ScaleError(f"worker build failed; see {log}")
    executable = target / "release/examples/slice40_common_measurement"
    if not executable.is_file():
        raise Slice40ScaleError("worker executable is unavailable")
    return executable


def _worker_result(
    executable: Path,
    treatment: str,
    database: Path,
    workload: Mapping[str, Any],
    output: Path,
) -> dict[str, Any]:
    completed = subprocess.run(
        [
            str(executable),
            treatment,
            str(database),
            str(workload["records"]),
            str(workload["batch_size"]),
            str(workload["open_warmups"]),
            str(workload["open_samples"]),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    output.with_suffix(".stdout.log").write_text(completed.stdout, encoding="utf-8")
    output.with_suffix(".stderr.log").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode != 0:
        raise Slice40ScaleError(
            f"worker failed; see {output.with_suffix('.stderr.log')}"
        )
    try:
        result = json.loads(completed.stdout.splitlines()[-1])
    except (IndexError, json.JSONDecodeError) as exc:
        raise Slice40ScaleError("worker did not emit valid JSON") from exc
    validate_worker_counts(result, workload)
    output.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    return result


def run(config_path: str | Path) -> dict[str, Any]:
    """Execute the common-path five-repetition Slice 40 comparison."""
    config_path = Path(config_path).resolve()
    config = load_config(config_path)
    if config["campaign"] != "common":
        raise Slice40ScaleError("run command requires the common campaign")
    code = require_clean_execution(config, require_candidate_product_tree=False)
    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    run_root = Path(config["artifact_root"]) / f"slice40-{stamp}"
    run_root.mkdir(parents=True, mode=0o700)
    results: dict[str, list[dict[str, Any]]] = {"baseline": [], "candidate": []}
    binaries: dict[str, dict[str, str]] = {}
    one_operation_observation: dict[str, Any] | None = None
    for treatment in ("baseline", "candidate"):
        treatment_root = run_root / treatment
        source = treatment_root / "source"
        _extract_source(config[treatment]["source_commit"], source)
        executable = _build_worker(source, treatment_root)
        binaries[treatment] = {
            "path": str(executable),
            "sha256": _sha256(executable),
            "source_commit": config[treatment]["source_commit"],
        }
        for repetition in range(1, config["workload"]["repetitions"] + 1):
            result_path = treatment_root / f"repetition-{repetition}.json"
            results[treatment].append(
                _worker_result(
                    executable,
                    treatment,
                    treatment_root / f"repetition-{repetition}.fathomdb",
                    config["workload"],
                    result_path,
                )
            )
        if treatment == "candidate":
            one_workload = {
                **config["workload"],
                "records": 1,
                "batch_size": 1,
                "expected_receipts": 1,
            }
            one_operation_observation = _worker_result(
                executable,
                "candidate_one_operation",
                treatment_root / "one-operation.fathomdb",
                one_workload,
                treatment_root / "one-operation.json",
            )

    seed = int(config["workload"]["bootstrap_seed"], 16)
    write_upper = {
        metric: paired_relative_upper(
            [item[f"write_{metric}_ms"] for item in results["baseline"]],
            [item[f"write_{metric}_ms"] for item in results["candidate"]],
            seed=seed + offset,
            resamples=config["workload"]["bootstrap_resamples"],
        )
        for offset, metric in enumerate(("p50", "p95"))
    }
    baseline_open = [item["open_p95_ms"] for item in results["baseline"]]
    candidate_open = [item["open_p95_ms"] for item in results["candidate"]]
    open_upper = paired_relative_upper(
        baseline_open,
        candidate_open,
        seed=seed + 2,
        resamples=config["workload"]["bootstrap_resamples"],
    )
    open_absolute = max(
        0.0, statistics.fmean(candidate_open) - statistics.fmean(baseline_open)
    )
    storage_increase = max(
        candidate["database_bytes_after_checkpoint"]
        - baseline["database_bytes_after_checkpoint"]
        for baseline, candidate in zip(
            results["baseline"], results["candidate"], strict=True
        )
    )
    verdict = comparison_verdict(
        write_upper=write_upper,
        open_upper_relative=open_upper,
        open_absolute_regression_ms=open_absolute,
        storage_increase_bytes=storage_increase,
        policy=config["policy"],
    )
    metrics = {
        "schema_version": "scale-02-slice40-common-result.v1",
        "program_track": PROGRAM_TRACK,
        "claim_boundary": config["claim_boundary"],
        "verdict": verdict,
        "write_upper_95_relative_regression": write_upper,
        "open_upper_95_relative_regression": open_upper,
        "open_mean_absolute_regression_ms": open_absolute,
        "maximum_paired_storage_increase_bytes": storage_increase,
        "treatments": results,
        "binaries": binaries,
        "one_operation_candidate_observation": one_operation_observation,
        "runner_sha256": _sha256(Path(__file__)),
        "worker_source_sha256": _sha256(
            REPO_ROOT / "experiments/rust/slice40_common_measurement.rs"
        ),
    }
    metrics_path = run_root / "metrics.json"
    metrics_path.write_text(
        json.dumps(metrics, sort_keys=True) + "\n", encoding="utf-8"
    )
    run_id, record_dir = _register_run(
        experiment="scale-02-slice40-common",
        config=config,
        config_path=config_path,
        decision_metrics=decision_metrics("common", metrics),
        detail=metrics,
        code_git_sha=code["git_sha"],
        record_arguments={
            "ts": datetime.now(UTC),
            "verdict": "complete" if verdict == "pass" else "advisory_limit_observed",
            "read": f"Slice 40 write/storage/reopen comparison: {verdict}",
            "code": {**code, "baseline_commit": BASELINE_COMMIT},
            "corpus": {
                "source": "deterministic Slice 40 representative fixture",
                "datasets": [],
            },
            "seeds": {"bootstrap": config["workload"]["bootstrap_seed"]},
            "env": _lib.env_info(
                key_deps={
                    "fathomdb_baseline": BASELINE_COMMIT,
                    "fathomdb_candidate": config["candidate"]["source_commit"],
                }
            ),
            "cost_usd": 0.0,
            "headline": {"verdict": verdict, **write_upper},
            "n": config["workload"]["records"],
            "tests": ["tests/experiments/test_scale_02_slice40.py"],
            "files_changed": [],
            "artifacts": [],
            "review": None,
            "open_questions": []
            if verdict == "pass"
            else ["A registered Slice 40 bound missed"],
        },
    )
    _lib.regen_index_md()
    return {
        "run_id": run_id,
        "record_dir": str(record_dir),
        "external_metrics": str(metrics_path),
        "verdict": verdict,
    }


_ATTESTED_PACKAGE_SUFFIXES = (".py", ".pyi", ".so", ".pyd", ".dll", ".dylib")


def _wheel_package_manifest(wheel: Path) -> dict[str, str]:
    with zipfile.ZipFile(wheel) as archive:
        members = {
            name: hashlib.sha256(archive.read(name)).hexdigest()
            for name in archive.namelist()
            if name.startswith("fathomdb/")
            and not name.endswith("/")
            and name.endswith(_ATTESTED_PACKAGE_SUFFIXES)
        }
    if not members or not any(name.endswith((".so", ".pyd")) for name in members):
        raise Slice40ScaleError("candidate wheel omitted its FathomDB package")
    return members


def _verify_installed_package_files(
    wheel: Path, package_dir: Path, *, prefix: Path
) -> list[dict[str, str]]:
    """Prove every executable package file is exactly from the pinned wheel."""

    prefix = prefix.resolve()
    package_dir = package_dir.resolve()
    if not package_dir.is_relative_to(prefix):
        raise Slice40ScaleError("imported FathomDB is outside the fresh virtualenv")
    site_root = package_dir.parent
    manifest = _wheel_package_manifest(wheel)
    actual = {
        str(path.relative_to(site_root))
        for path in package_dir.rglob("*")
        if path.is_file() and path.name.endswith(_ATTESTED_PACKAGE_SUFFIXES)
    }
    expected = set(manifest)
    extra = sorted(actual - expected)
    if extra:
        raise Slice40ScaleError(
            f"installed FathomDB contains extra executable files: {extra}"
        )
    if expected - actual:
        raise Slice40ScaleError("installed FathomDB files do not match the wheel")
    attested: list[dict[str, str]] = []
    for member, digest in sorted(manifest.items()):
        installed = (site_root / member).resolve()
        if not installed.is_relative_to(package_dir) or _sha256(installed) != digest:
            raise Slice40ScaleError("installed FathomDB files do not match the wheel")
        attested.append(
            {
                "wheel_member": member,
                "installed_path": str(installed),
                "sha256": digest,
            }
        )
    return attested


def _installed_artifact_attestation(
    wheel: Path, config: Mapping[str, Any]
) -> dict[str, object]:
    """Prove the imported Python wrapper and native module came from the wheel."""

    if sys.prefix == sys.base_prefix:
        raise Slice40ScaleError("throughput execution requires a fresh virtualenv")
    if _sha256(wheel) != config["candidate"]["wheel_sha256"]:
        raise Slice40ScaleError("executed wheel SHA-256 drifted")
    import fathomdb
    from fathomdb import _fathomdb as native_module

    prefix = Path(sys.prefix).resolve()
    package_path = Path(fathomdb.__file__).resolve()
    native_path = Path(native_module.__file__).resolve()
    package_files = _verify_installed_package_files(
        wheel, package_path.parent, prefix=prefix
    )
    attested_paths = {item["installed_path"] for item in package_files}
    if str(package_path) not in attested_paths or str(native_path) not in attested_paths:
        raise Slice40ScaleError("loaded FathomDB files do not match the wheel")
    return {
        "wheel_path": str(wheel),
        "wheel_sha256": _sha256(wheel),
        "python_prefix": str(prefix),
        "package_path": str(package_path),
        "native_path": str(native_path),
        "package_files": package_files,
    }


def _cli_artifact_attestation(
    fathomdb_bin: Path, config: Mapping[str, Any]
) -> dict[str, str]:
    """Bind setup and doctor execution to the preregistered CLI artifact."""

    binary = fathomdb_bin.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise Slice40ScaleError("preregistered FathomDB CLI is not executable")
    digest = _sha256(binary)
    if digest != config["candidate"]["cli_sha256"]:
        raise Slice40ScaleError("executed CLI SHA-256 drifted")
    return {
        "cli_path": str(binary),
        "cli_sha256": digest,
        "cli_source_commit": config["candidate"]["cli_source_commit"],
    }


def _throughput_document(index: int) -> dict[str, str]:
    return {
        "kind": "doc",
        "logical_id": f"slice40-throughput-{index:05d}",
        "source_id": "slice40-cuda-throughput-v2",
        "body": (
            f"Document {index} records a durable personal-memory observation "
            f"about project {index % 31}, participant {index % 17}, and day "
            f"{index % 29}. The distinct sequence token is evidence-{index:05d}."
        ),
    }


def _vector_row_count(database: Path) -> int:
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        row = connection.execute(
            "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind='doc'"
        ).fetchone()
    if row is None:
        raise Slice40ScaleError("vector-row observation returned no row")
    return int(row[0])


def _run_cuda_throughput_cells(
    config: Mapping[str, Any], *, run_root: Path, fathomdb_bin: Path
) -> tuple[list[dict[str, object]], dict[str, object]]:
    from fathomdb import Engine, ProjectionRole, ProjectionSpec, read

    runtime = config["runtime"]
    workload = config["workload"]
    for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE"):
        if os.environ.get(name) != "1":
            raise Slice40ScaleError(f"{name}=1 is required for offline execution")
    if os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
        raise Slice40ScaleError("FATHOMDB_EMBED_DEVICE must be cuda:0")
    documents = [
        _throughput_document(index) for index in range(workload["records"])
    ]
    observations: list[dict[str, object]] = []
    for repetition in range(1, workload["repetitions"] + 1):
        prepared = prepare_test_database(
            run_root,
            test_id=f"repetition-{repetition}",
            embed_device="cuda:0",
            rerank_device="cpu",
            embedder="default",
            warm_cache=True,
            check_reranker=False,
            fathomdb_bin=str(fathomdb_bin),
        )
        setup = json.loads(prepared.doctor_path.read_text(encoding="utf-8"))
        opened = setup["open_report"]
        if opened["embedder_download_ms"] is not None or any(
            event.get("kind") == "DefaultEmbedderDownload"
            for event in opened["embedder_events"]
        ):
            raise Slice40ScaleError("offline setup observed a model download")
        engine = Engine.open(str(prepared.database_path), use_default_embedder=True)
        try:
            report = engine.open_report()
            resolution = report.embedder_device_resolution
            identity = report.default_embedder
            if resolution is None:
                raise Slice40ScaleError("throughput open omitted device resolution")
            if (
                resolution.effective_device.kind != "cuda"
                or resolution.selected_cuda_uuid != runtime["cuda_uuid"]
                or asdict(identity) != runtime["embedder"]
                or report.embedder_download_ms is not None
                or any(
                    event.get("kind") == "DefaultEmbedderDownload"
                    for event in report.embedder_events
                )
            ):
                raise Slice40ScaleError("throughput runtime attestation drifted")
            for offset in range(0, len(documents), workload["write_batch_size"]):
                engine.write(documents[offset : offset + workload["write_batch_size"]])
            for index in range(workload["warmup_records"]):
                if len(engine.embed(f"excluded warmup text {index}")) != 384:
                    raise Slice40ScaleError("warmup returned wrong vector dimension")
            pre_vector_rows = _vector_row_count(prepared.database_path)
            started = time.perf_counter()
            delta = engine.configure_projections(
                [
                    ProjectionSpec(
                        name="slice40_throughput",
                        roles=frozenset({ProjectionRole.SEARCHABLE}),
                        vector=True,
                    )
                ]
            )
            engine.drain(timeout_s=workload["drain_timeout_seconds"])
            elapsed = time.perf_counter() - started
            status = read.projection_generation_status(engine)
        finally:
            engine.close()
        vector_rows = _vector_row_count(prepared.database_path)
        observations.append(
            {
                "repetition": repetition,
                "elapsed_seconds": elapsed,
                "records": workload["records"],
                "records_per_second": workload["records"] / elapsed,
                "selected_device": "cuda:0",
                "selected_cuda_uuid": resolution.selected_cuda_uuid,
                "embedder_identity": asdict(identity),
                "pre_vector_rows": pre_vector_rows,
                "projection_delta": {
                    "built": list(delta.built),
                    "deferred": list(delta.deferred),
                    "unchanged": delta.unchanged,
                },
                "vector_rows": vector_rows,
                "readiness": status.readiness,
                "runtime_state": status.runtime_state,
                "pending_count": status.pending_count,
                "failed_count": status.failed_count,
                "generation_id": status.generation_id,
                "database_sha256": _sha256(prepared.database_path),
                "setup_config": str(prepared.config_path.relative_to(run_root)),
                "setup_config_sha256": _sha256(prepared.config_path),
                "doctor_evidence": str(prepared.doctor_path.relative_to(run_root)),
                "doctor_evidence_sha256": _sha256(prepared.doctor_path),
            }
        )
    summary = summarize_cuda_throughput_observations(observations, config)
    return observations, summary


def _blocked_reason(exc: Exception) -> dict[str, object]:
    return {
        "code": "database_setup_failed",
        "stage": "cuda_projection_throughput",
        "message": "Slice 40 CUDA projection-throughput execution did not complete.",
        "detail": {"error_type": type(exc).__name__, "error": str(exc)},
    }


def run_cuda_throughput(
    config_path: str | Path, *, wheel: Path, fathomdb_bin: Path
) -> dict[str, Any]:
    """Execute and register the standard descriptive CUDA throughput cell."""

    config_path = Path(config_path).resolve()
    config = load_cuda_throughput_config(config_path)
    code = _lib.git_info()
    if code["dirty"]:
        raise Slice40ScaleError("throughput campaign requires a clean checkout")
    artifact_root = Path(config["artifact_root"]).resolve()
    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    run_root = artifact_root / f"slice40-cuda-throughput-{stamp}"
    if run_root.exists() or not run_root.parent.is_relative_to(artifact_root):
        raise Slice40ScaleError("throughput output must be new beneath artifact_root")
    run_root.mkdir(parents=True, mode=0o700)
    blocker: dict[str, object] | None = None
    attestation: dict[str, object] | None = None
    cli_attestation: dict[str, str] | None = None
    try:
        attestation = _installed_artifact_attestation(wheel.resolve(), config)
        cli_attestation = _cli_artifact_attestation(fathomdb_bin, config)
        observations, summary = _run_cuda_throughput_cells(
            config, run_root=run_root, fathomdb_bin=fathomdb_bin.resolve()
        )
        detail: dict[str, Any] = {
            "schema_version": "scale-02-slice40-cuda-throughput-result.v2",
            "program_track": PROGRAM_TRACK,
            "claim_boundary": config["claim_boundary"],
            "candidate": config["candidate"],
            "installed_artifact": attestation,
            "cli_artifact": cli_attestation,
            "observations": observations,
            "summary": summary,
        }
        metrics = decision_metrics("cuda-throughput", summary)
        verdict = "complete"
        read = "Slice 40 CUDA projection backfill throughput observed."
    except Exception as exc:
        blocker = _blocked_reason(exc)
        observations = []
        detail = {
            "schema_version": "scale-02-slice40-cuda-throughput-blocked.v1",
            "program_track": PROGRAM_TRACK,
            "claim_boundary": "no_performance_claim",
            "candidate": config["candidate"],
            "installed_artifact": attestation,
            "cli_artifact": cli_attestation,
            "blocked_reason": blocker,
        }
        metrics = {
            "schema_version": "scale-02-slice40-cuda-throughput-blocked.v1",
            "campaign": "cuda-throughput",
            "state": "blocked",
            "blocked_reason": blocker,
        }
        verdict = "blocked_execution"
        read = "Slice 40 CUDA projection backfill throughput blocked."
    external_detail = run_root / "result-detail.json"
    external_detail.write_text(
        json.dumps(detail, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    run_id, record_dir = _register_run(
        experiment="scale-02-slice40-cuda-throughput",
        config=config,
        config_path=config_path,
        decision_metrics=metrics,
        detail=detail,
        code_git_sha=code["git_sha"],
        execution_files=_THROUGHPUT_EXECUTION_FILES,
        classification_outcome="complete" if blocker is None else "blocked",
        blocked_reason=blocker,
        record_arguments={
            "ts": datetime.now(UTC),
            "verdict": verdict,
            "read": read,
            "code": {**code, "baseline_commit": None},
            "corpus": {
                "source": "deterministic Slice 40 projection-throughput fixture",
                "datasets": [],
            },
            "seeds": {},
            "env": _lib.env_info(
                key_deps={
                    "fathomdb_product_candidate": config["candidate"][
                        "product_commit"
                    ],
                    "fathomdb_artifact_build": config["candidate"][
                        "artifact_build_commit"
                    ],
                    "candidate_wheel_sha256": config["candidate"]["wheel_sha256"],
                    "fathomdb_cli_source_commit": config["candidate"][
                        "cli_source_commit"
                    ],
                    "fathomdb_cli_sha256": config["candidate"]["cli_sha256"],
                    "network_policy": config["runtime"]["network"],
                }
            ),
            "cost_usd": 0.0,
            "headline": metrics,
            "n": config["workload"]["records"],
            "tests": [
                "tests/experiments/test_scale_02_slice40.py",
                "tests/experiments/test_slice40_cuda_projection_throughput.py",
            ],
            "files_changed": [],
            "artifacts": [
                {
                    "path": str(external_detail),
                    "sha256": _sha256(external_detail),
                }
            ],
            "review": None,
            "open_questions": []
            if blocker is None
            else ["resolve the blocked CUDA throughput cell before Slice 40 closes"],
        },
    )
    _lib.regen_index_md()
    result = {
        "run_id": run_id,
        "record_dir": str(record_dir),
        "external_detail": str(external_detail),
        "verdict": verdict,
    }
    if blocker is not None:
        raise Slice40ScaleError(f"CUDA throughput cell blocked: {blocker['detail']}")
    return result


def main(argv: list[str] | None = None) -> int:
    """Validate or execute the registered campaign."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate")
    validate.add_argument("config", type=Path)
    execute = subparsers.add_parser("run")
    execute.add_argument("config", type=Path)
    status = subparsers.add_parser("status")
    status.add_argument("config", type=Path)
    validate_throughput = subparsers.add_parser("validate-throughput")
    validate_throughput.add_argument("config", type=Path)
    throughput = subparsers.add_parser("throughput")
    throughput.add_argument("config", type=Path)
    throughput.add_argument("--wheel", type=Path, required=True)
    throughput.add_argument("--fathomdb-bin", type=Path, required=True)
    arguments = parser.parse_args(argv)
    if arguments.command == "validate":
        load_config(arguments.config)
        print("ok")
    elif arguments.command == "run":
        print(json.dumps(run(arguments.config), sort_keys=True))
    elif arguments.command == "status":
        print(json.dumps(run_status(arguments.config), sort_keys=True))
    elif arguments.command == "validate-throughput":
        load_cuda_throughput_config(arguments.config)
        print("ok")
    else:
        print(
            json.dumps(
                run_cuda_throughput(
                    arguments.config,
                    wheel=arguments.wheel,
                    fathomdb_bin=arguments.fathomdb_bin,
                ),
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
