"""Run the preregistered Slice 40 write, storage, reopen, and status campaign."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import random
import shutil
import sqlite3
import statistics
import subprocess
import tarfile
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib, scale_02


SCHEMA = "scale-02-slice40.v2"
PROGRAM_TRACK = "SCALE-02"
REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE_COMMIT = "0aff1cb08c61a8bb2a004813bbd5604b6ff1a403"
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "approval",
    "baseline",
    "candidate",
    "execution",
    "workload",
    "policy",
    "artifact_root",
    "claim_boundary",
}
_EXECUTION_FILES = {
    "runner_sha256": Path(__file__),
    "record_library_sha256": REPO_ROOT / "experiments/_lib.py",
    "statistics_library_sha256": REPO_ROOT / "experiments/scale_02.py",
    "common_worker_sha256": REPO_ROOT
    / "experiments/rust/slice40_common_measurement.rs",
    "status_test_sha256": REPO_ROOT
    / "src/rust/crates/fathomdb-engine/tests/slice40_status_performance.rs",
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
    workload = _exact(root["workload"], "workload", set(_WORKLOAD))
    if workload != _WORKLOAD:
        raise Slice40ScaleError("workload drifted from the registered matrix")
    policy = _exact(root["policy"], "policy", set(_POLICY))
    if policy != _POLICY:
        raise Slice40ScaleError("policy drifted from the registered limits")
    artifact_root = root["artifact_root"]
    if not isinstance(artifact_root, str) or not Path(artifact_root).is_absolute():
        raise Slice40ScaleError("artifact_root must be absolute and outside the repository")
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
                raise Slice40ScaleError(f"execution.{label} does not match the reviewed source")
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


def load_config(path: str | Path, *, validate_repository: bool = True) -> dict[str, Any]:
    """Load a Slice 40 campaign contract."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Slice40ScaleError("configuration is unavailable") from exc
    return resolve_config(document, validate_repository=validate_repository)


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
        or open_absolute_regression_ms
        <= policy["open_max_absolute_regression_ms"]
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
            raise Slice40ScaleError(f"status result {field} must be a nonnegative integer")
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
        raise Slice40ScaleError("status repetitions drifted from the registered workload")
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


def _build_status_test(
    *, build_variant: str, run_root: Path
) -> dict[str, str]:
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
    result_lines = [line for line in completed.stdout.splitlines() if line.startswith("{")]
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
        build_variant: _build_status_test(build_variant=build_variant, run_root=run_root)
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
                raise Slice40ScaleError("status seed retained an uncheckpointed SQLite sidecar")
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
    metrics_path.write_text(json.dumps(metrics, sort_keys=True) + "\n", encoding="utf-8")
    run_id, record_dir = _lib.write_record(
        "scale-02-slice40-status",
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=metrics,
        verdict="complete" if verdict == "pass" else "advisory_limit_observed",
        read=f"Slice 40 paired CPU/CUDA status campaign: {verdict}",
        code=code,
        corpus={"source": "byte-identical paired 50k status fixtures", "datasets": []},
        seeds={"bootstrap": config["workload"]["bootstrap_seed"]},
        env=_lib.env_info(
            key_deps={
                "fathomdb_candidate": config["candidate"]["source_commit"],
                "cuda_host_witness": nvidia.stdout.strip(),
            }
        ),
        cost_usd=0.0,
        headline={
            "verdict": verdict,
            "maximum_cpu_cuda_p95_difference_ms": maximum_device_p95_difference_ms,
        },
        n=config["workload"]["status_records"],
        config_path=str(config_path),
        tests=[
            "src/rust/crates/fathomdb-engine/tests/slice40_status_performance.rs",
            "tests/experiments/test_scale_02_slice40.py",
        ],
        files_changed=[],
        artifacts=[
            {"kind": "external_safe_summary", "path": str(metrics_path), "sha256": _sha256(metrics_path)},
            {"kind": "cuda_device_witness", "path": str(nvidia_log), "sha256": _sha256(nvidia_log)},
        ],
        review=None,
        open_questions=[] if verdict == "pass" else ["A registered status bound missed"],
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
    example = source / "src/rust/crates/fathomdb-engine/examples/slice40_common_measurement.rs"
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
        raise Slice40ScaleError(f"worker failed; see {output.with_suffix('.stderr.log')}")
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
    metrics_path.write_text(json.dumps(metrics, sort_keys=True) + "\n", encoding="utf-8")
    run_id, record_dir = _lib.write_record(
        "scale-02-slice40-common",
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=metrics,
        verdict="complete" if verdict == "pass" else "advisory_limit_observed",
        read=f"Slice 40 write/storage/reopen comparison: {verdict}",
        code={**code, "baseline_commit": BASELINE_COMMIT},
        corpus={"source": "deterministic Slice 40 representative fixture", "datasets": []},
        seeds={"bootstrap": config["workload"]["bootstrap_seed"]},
        env=_lib.env_info(
            key_deps={
                "fathomdb_baseline": BASELINE_COMMIT,
                "fathomdb_candidate": config["candidate"]["source_commit"],
            }
        ),
        cost_usd=0.0,
        headline={"verdict": verdict, **write_upper},
        n=config["workload"]["records"],
        config_path=str(config_path),
        tests=["tests/experiments/test_scale_02_slice40.py"],
        files_changed=[],
        artifacts=[
            {
                "kind": "external_safe_summary",
                "path": str(metrics_path),
                "sha256": _sha256(metrics_path),
            }
        ],
        review=None,
        open_questions=[] if verdict == "pass" else ["A registered Slice 40 bound missed"],
    )
    _lib.regen_index_md()
    return {
        "run_id": run_id,
        "record_dir": str(record_dir),
        "external_metrics": str(metrics_path),
        "verdict": verdict,
    }


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
    arguments = parser.parse_args(argv)
    if arguments.command == "validate":
        load_config(arguments.config)
        print("ok")
    elif arguments.command == "run":
        print(json.dumps(run(arguments.config), sort_keys=True))
    else:
        print(json.dumps(run_status(arguments.config), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
