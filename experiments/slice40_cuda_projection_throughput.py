"""Run the preregistered descriptive Slice 40 CUDA projection observation."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import sqlite3
import statistics
import sys
import time
from pathlib import Path
from typing import Any, Mapping, Sequence


SCHEMA = "scale-02-slice40-cuda-throughput.v1"
PRODUCT_COMMIT = "2313fd34ea5ca68346a468d5bba58ba245306c08"
CUDA_UUID = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"
WORKLOAD = {
    "records": 1024,
    "write_batch_size": 128,
    "repetitions": 5,
    "warmup_records": 64,
    "drain_timeout_seconds": 900,
}
RUNTIME = {
    "embed_device": "cuda:0",
    "cuda_uuid": CUDA_UUID,
    "embedder": "fathomdb-bge-small-en-v1.5",
    "network": "none",
}
POLICY = {
    "classification": "descriptive",
    "decision_threshold": None,
    "excluded_time": ["artifact_install", "model_load", "model_warmup"],
    "included_time": ["configure_projection", "projection_backfill", "drain"],
}
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "approval",
    "candidate",
    "execution",
    "workload",
    "runtime",
    "policy",
    "artifact_root",
    "claim_boundary",
}


class ThroughputContractError(ValueError):
    """Raised when the registered observation contract or evidence drifts."""


def _closed(
    value: object, label: str, keys: set[str]
) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise ThroughputContractError(
            f"{label} keys drifted: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _digest(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != 64
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise ThroughputContractError(f"{label} must be a lowercase SHA-256 digest")
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
    """Validate and return the closed preregistered observation contract."""

    root = _closed(document, "config", _TOP_KEYS)
    if (
        root["schema_version"] != SCHEMA
        or root["program_track"] != "SCALE-02"
        or root["release"] != "0.8.25"
        or root["claim_boundary"]
        != "cuda_projection_backfill_throughput_descriptive"
    ):
        raise ThroughputContractError("configuration identity drifted")
    approval = _closed(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] != "approved" or approval["approved_by"] != "HITL":
        raise ThroughputContractError("execution is not HITL-approved")
    candidate = _closed(
        root["candidate"], "candidate", {"product_commit", "wheel_sha256"}
    )
    if candidate["product_commit"] != PRODUCT_COMMIT:
        raise ThroughputContractError("candidate product commit drifted")
    _digest(candidate["wheel_sha256"], "candidate.wheel_sha256")
    execution = _closed(
        root["execution"],
        "execution",
        {"runner_sha256", "measurement_plan_sha256"},
    )
    _digest(execution["runner_sha256"], "execution.runner_sha256")
    _digest(
        execution["measurement_plan_sha256"],
        "execution.measurement_plan_sha256",
    )
    workload = _closed(root["workload"], "workload", set(WORKLOAD))
    if workload != WORKLOAD:
        raise ThroughputContractError("workload drifted from the registered matrix")
    runtime = _closed(root["runtime"], "runtime", set(RUNTIME))
    if runtime != RUNTIME:
        raise ThroughputContractError("runtime drifted from forced CUDA contract")
    policy = _closed(root["policy"], "policy", set(POLICY))
    if policy != POLICY:
        raise ThroughputContractError("policy drifted from descriptive contract")
    artifact_root = root["artifact_root"]
    if not isinstance(artifact_root, str) or not Path(artifact_root).is_absolute():
        raise ThroughputContractError("artifact_root must be absolute")
    if validate_repository:
        runner = Path(__file__).resolve()
        if _sha256(runner) != execution["runner_sha256"]:
            raise ThroughputContractError("runner SHA-256 does not match preregistration")
    return copy.deepcopy(root)


def summarize_observations(
    observations: Sequence[Mapping[str, object]], config: Mapping[str, Any]
) -> dict[str, object]:
    """Validate complete CUDA observations and calculate descriptive statistics."""

    workload = config["workload"]
    runtime = config["runtime"]
    expected_repetitions = workload["repetitions"]
    if len(observations) != expected_repetitions:
        raise ThroughputContractError("observation repetition count drifted")
    rates: list[float] = []
    repetitions: set[int] = set()
    for observation in observations:
        repetition = observation.get("repetition")
        if not isinstance(repetition, int) or repetition in repetitions:
            raise ThroughputContractError("repetition IDs must be unique integers")
        repetitions.add(repetition)
        if observation.get("selected_device") != "cuda:0":
            raise ThroughputContractError("every observation must execute on cuda:0")
        if observation.get("selected_cuda_uuid") != runtime["cuda_uuid"]:
            raise ThroughputContractError("selected CUDA UUID drifted")
        if observation.get("records") != workload["records"]:
            raise ThroughputContractError("record count drifted")
        if observation.get("vector_rows") != workload["records"]:
            raise ThroughputContractError("vector_rows did not reach the record count")
        if observation.get("readiness") != "ready":
            raise ThroughputContractError("projection did not reach ready")
        rate = observation.get("records_per_second")
        elapsed = observation.get("elapsed_seconds")
        if (
            not isinstance(rate, (int, float))
            or isinstance(rate, bool)
            or rate <= 0
            or not isinstance(elapsed, (int, float))
            or isinstance(elapsed, bool)
            or elapsed <= 0
        ):
            raise ThroughputContractError("timing observations must be positive")
        rates.append(float(rate))
    return {
        "classification": "descriptive",
        "repetitions": expected_repetitions,
        "records_per_repetition": workload["records"],
        "throughput_records_per_second": {
            "minimum": min(rates),
            "median": statistics.median(rates),
            "maximum": max(rates),
        },
        "verdict": "observed",
    }


def _write_json_exclusive(path: Path, value: object) -> None:
    encoded = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True).encode("utf-8")
        + b"\n"
    )
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(encoded)


def _document(index: int) -> dict[str, str]:
    return {
        "kind": "doc",
        "logical_id": f"slice40-throughput-{index:05d}",
        "source_id": "slice40-cuda-throughput-v1",
        "body": (
            f"Document {index} records a durable personal-memory observation "
            f"about project {index % 31}, participant {index % 17}, and day "
            f"{index % 29}. The distinct sequence token is evidence-{index:05d}."
        ),
    }


def _vector_rows(database: Path) -> int:
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        row = connection.execute(
            "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind = 'doc'"
        ).fetchone()
    assert row is not None
    return int(row[0])


def run_observation(
    config: Mapping[str, Any], output_dir: Path
) -> tuple[list[dict[str, object]], dict[str, object]]:
    """Execute the forced-CUDA projection backfill observation."""

    if os.environ.get("FATHOMDB_EMBED_DEVICE") != "cuda:0":
        raise ThroughputContractError("FATHOMDB_EMBED_DEVICE must be cuda:0")
    from fathomdb import Engine, ProjectionRole, ProjectionSpec, read
    from fathomdb import _fathomdb as native_module

    output_dir.mkdir(parents=True, mode=0o700)
    observations: list[dict[str, object]] = []
    workload = config["workload"]
    expected_uuid = config["runtime"]["cuda_uuid"]
    documents = [_document(index) for index in range(workload["records"])]
    for repetition in range(1, workload["repetitions"] + 1):
        database = output_dir / f"repetition-{repetition}.fathomdb"
        engine = Engine.open(str(database), use_default_embedder=True)
        try:
            report = engine.open_report()
            resolution = report.embedder_device_resolution
            if resolution is None:
                raise ThroughputContractError("open report omitted device resolution")
            effective = resolution.effective_device
            cuda_device = effective.cuda_device
            selected_device = (
                f"cuda:{cuda_device.ordinal}" if cuda_device is not None else effective.kind
            )
            if (
                selected_device != "cuda:0"
                or resolution.selected_cuda_uuid != expected_uuid
            ):
                raise ThroughputContractError("installed artifact did not select registered GPU")
            for offset in range(0, len(documents), workload["write_batch_size"]):
                engine.write(documents[offset : offset + workload["write_batch_size"]])
            for index in range(workload["warmup_records"]):
                vector = engine.embed(f"excluded warmup text {index}")
                if len(vector) != 384:
                    raise ThroughputContractError("warmup returned wrong vector dimension")
            started = time.perf_counter()
            engine.configure_projections(
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
        vector_rows = _vector_rows(database)
        observations.append(
            {
                "repetition": repetition,
                "elapsed_seconds": elapsed,
                "records": workload["records"],
                "records_per_second": workload["records"] / elapsed,
                "selected_device": selected_device,
                "selected_cuda_uuid": resolution.selected_cuda_uuid,
                "vector_rows": vector_rows,
                "readiness": status.readiness,
                "runtime_state": status.runtime_state,
                "pending_count": status.pending_count,
                "failed_count": status.failed_count,
                "generation_id": status.generation_id,
                "database_sha256": _sha256(database),
            }
        )
    provenance = {
        "python": sys.version,
        "fathomdb_module": str(Path(sys.modules["fathomdb"].__file__).resolve()),
        "native_module": str(Path(native_module.__file__).resolve()),
    }
    return observations, provenance


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--measurement-plan", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)

    config = resolve_config(json.loads(args.config.read_text(encoding="utf-8")))
    if _sha256(args.wheel) != config["candidate"]["wheel_sha256"]:
        raise ThroughputContractError("executed wheel SHA-256 drifted")
    if (
        _sha256(args.measurement_plan)
        != config["execution"]["measurement_plan_sha256"]
    ):
        raise ThroughputContractError("measurement-plan SHA-256 drifted")
    if args.output.exists():
        raise ThroughputContractError("output directory must not already exist")
    observations, provenance = run_observation(config, args.output)
    summary = summarize_observations(observations, config)
    _write_json_exclusive(args.output / "config.resolved.json", config)
    _write_json_exclusive(args.output / "observations.json", observations)
    _write_json_exclusive(args.output / "metrics.json", summary)
    _write_json_exclusive(
        args.output / "record.json",
        {
            "schema_version": SCHEMA,
            "claim_boundary": config["claim_boundary"],
            "candidate": config["candidate"],
            "execution": config["execution"],
            "runtime": config["runtime"],
            "policy": config["policy"],
            "provenance": provenance,
            "observations": observations,
            "metrics": summary,
        },
    )
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
