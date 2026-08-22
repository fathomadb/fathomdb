"""Execute the preregistered SCALE-02 A0 local-first efficiency envelope."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import math
import os
import platform
import random
import resource
import sqlite3
import statistics
import subprocess
import threading
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from experiments import _lib
from experiments.fathomdb_test_setup import PreparedDatabase, prepare_test_database


SCHEMA_VERSION = "scale-02-execution.v1"
PROGRAM_TRACK = "SCALE-02"
RELEASE = "0.8.23"
REPO_ROOT = Path(__file__).resolve().parent.parent
_DIGEST_LENGTH = 64
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "approval",
    "dependencies",
    "profile",
    "ladder",
    "corpus",
    "workload",
    "uncertainty",
    "advisory_policy",
    "runtime",
    "claim_boundary",
}


class Scale02Error(ValueError):
    """Raised when a SCALE-02 control, input, or result fails closed."""


@dataclass(frozen=True)
class SourceDocument:
    """One verified real source document used by the scale fixture."""

    document_id: str
    text: str
    content_sha256: str


@dataclass(frozen=True)
class ScaleRow:
    """One canonical database row with its fixture-origin disclosure."""

    logical_id: str
    body: str
    source_id: str
    origin: str


@dataclass(frozen=True)
class Fixture:
    """Verified real prefix and query catalog; payloads remain external."""

    documents: tuple[SourceDocument, ...]
    queries: tuple[str, ...]
    fixture_digest: str


@dataclass(frozen=True)
class Scale02Config:
    """Strict resolved configuration for the A0 envelope."""

    program_track: str
    approval_state: str
    approval_by: str | None
    approval_at: str | None
    profile_id: str
    top_k: int
    embed_device: str
    rerank_device: str
    sizes: tuple[int, ...]
    real_primary_count: int
    repetitions: int
    write_batch_size: int
    cold_query_count: int
    cold_definition: str
    warmup_query_count: int
    steady_query_count: int
    steady_definition: str
    mutation_count: int
    query_order_seed: str
    drain_timeout_seconds: int
    query_timeout_ms: int
    bootstrap_resamples: int
    bootstrap_seed: str
    policy: dict[str, Any]
    corpus_root: Path
    corpus_index: str
    corpus_index_sha256: str
    qualified_manifest: Path
    qualified_manifest_sha256: str
    growth_seed: str
    python: Path
    python_package_version: str
    python_extension: Path
    python_extension_sha256: str
    fathomdb_bin: Path
    fathomdb_bin_sha256: str
    dependencies: dict[str, dict[str, Any]]
    resolved: dict[str, Any]


def _exact(value: object, name: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Scale02Error(
            f"{name} keys drifted: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _digest(value: object, name: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != _DIGEST_LENGTH
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Scale02Error(f"{name} must be a lowercase sha256")
    return value


def _hex_seed(value: object, name: str) -> str:
    if not isinstance(value, str) or not value.startswith("0x"):
        raise Scale02Error(f"{name} must be a hexadecimal seed")
    try:
        int(value, 16)
    except ValueError as exc:
        raise Scale02Error(f"{name} must be a hexadecimal seed") from exc
    return value


def _positive_int(value: object, name: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        raise Scale02Error(f"{name} must be a positive integer")
    return value


def _path(value: object, name: str, *, external: bool = False) -> Path:
    if not isinstance(value, str) or not value:
        raise Scale02Error(f"{name} must be a non-empty path")
    path = Path(value)
    return path if path.is_absolute() or external else REPO_ROOT / path


def _sha_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _canonical_digest(value: object) -> str:
    encoded = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=True,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def resolve_config(document: object) -> Scale02Config:
    """Strictly resolve the SCALE-02 configuration."""
    root = _exact(document, "config", _TOP_KEYS)
    if (
        root["schema_version"] != SCHEMA_VERSION
        or root["program_track"] != PROGRAM_TRACK
        or root["release"] != RELEASE
        or root["claim_boundary"] != "advisory_a0_efficiency_only"
    ):
        raise Scale02Error("SCALE-02 identity or claim boundary drifted")

    approval = _exact(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] not in {"pending_hitl", "approved"}:
        raise Scale02Error("approval.state must be pending_hitl or approved")
    if approval["state"] == "pending_hitl":
        if approval["approved_by"] is not None or approval["approved_at"] is not None:
            raise Scale02Error("pending approval cannot carry an approver or time")
    elif not all(
        isinstance(approval[key], str) and approval[key]
        for key in ("approved_by", "approved_at")
    ):
        raise Scale02Error("approved execution requires approver and timestamp")

    dependencies = _exact(
        root["dependencies"], "dependencies", {"answer_01", "scale_01"}
    )
    expected_dependencies = {
        "answer_01": ("answer-01-shortlist-live-20260822T1234Z-8a050808", "retain_a0"),
        "scale_01": (
            "tc5-gpu-primary-20260822T1605Z-2d574205",
            "fidelity_primary_complete",
        ),
    }
    for name, (run_id, decision) in expected_dependencies.items():
        item = _exact(
            dependencies[name],
            f"dependencies.{name}",
            {"path", "sha256", "run_id", "decision"},
        )
        if item["run_id"] != run_id or item["decision"] != decision:
            raise Scale02Error(f"{name} dependency decision drifted")
        _path(item["path"], f"dependencies.{name}.path")
        _digest(item["sha256"], f"dependencies.{name}.sha256")

    profile = _exact(
        root["profile"],
        "profile",
        {
            "id",
            "retrieval",
            "top_k",
            "embedder",
            "embed_device",
            "reranker",
            "rerank_device",
        },
    )
    if profile != {
        "id": "a0_turn_fts",
        "retrieval": "fts",
        "top_k": 10,
        "embedder": "none",
        "embed_device": "cpu",
        "reranker": "none",
        "rerank_device": "cpu",
    }:
        raise Scale02Error("profile must remain ANSWER-01 A0")
    if root["ladder"] != [10000, 17272, 25000, 40000, 50000]:
        raise Scale02Error("scale ladder drifted")

    corpus = _exact(
        root["corpus"],
        "corpus",
        {
            "root",
            "index",
            "index_sha256",
            "qualified_manifest",
            "qualified_manifest_sha256",
            "real_primary_count",
            "growth",
        },
    )
    growth = _exact(
        corpus["growth"], "corpus.growth", {"mode", "seed", "source", "disclosure"}
    )
    if (
        corpus["real_primary_count"] != 17272
        or growth["mode"] != "deterministic_derived_fixture"
        or growth["source"] != "qualified_real_prefix_round_robin"
        or growth["disclosure"]
        != "points_above_17272_are_efficiency_fixture_not_real_corpus_fidelity"
    ):
        raise Scale02Error("corpus growth disclosure drifted")
    _hex_seed(growth["seed"], "corpus.growth.seed")
    _digest(corpus["index_sha256"], "corpus.index_sha256")
    _digest(corpus["qualified_manifest_sha256"], "corpus.qualified_manifest_sha256")
    if not isinstance(corpus["index"], str) or not corpus["index"]:
        raise Scale02Error("corpus.index must be a non-empty relative path")

    workload = _exact(
        root["workload"],
        "workload",
        {
            "repetitions",
            "write_batch_size",
            "cold_query_count",
            "cold_definition",
            "warmup_query_count",
            "steady_query_count",
            "steady_definition",
            "mutation_count",
            "concurrency",
            "query_order_seed",
            "drain_timeout_seconds",
            "query_timeout_ms",
        },
    )
    for name in (
        "repetitions",
        "write_batch_size",
        "cold_query_count",
        "warmup_query_count",
        "steady_query_count",
        "mutation_count",
        "drain_timeout_seconds",
        "query_timeout_ms",
    ):
        _positive_int(workload[name], f"workload.{name}")
    if workload["concurrency"] != 1 or workload["repetitions"] != 5:
        raise Scale02Error("SCALE-02 must use five serial independent repetitions")
    if workload["cold_query_count"] != 100 or workload["steady_query_count"] != 1000:
        raise Scale02Error("SCALE-02 query sample counts drifted")
    if (
        workload["cold_definition"]
        != "first_query_pass_after_populated_reopen_without_intentional_warmup"
        or workload["steady_definition"]
        != "same_repetition_after_the_declared_100_query_warmup"
    ):
        raise Scale02Error("cold or steady-state definition drifted")
    _hex_seed(workload["query_order_seed"], "workload.query_order_seed")

    uncertainty = _exact(
        root["uncertainty"],
        "uncertainty",
        {"method", "confidence", "resamples", "seed"},
    )
    if (
        uncertainty["method"] != "repetition_level_percentile_bootstrap"
        or uncertainty["confidence"] != 0.95
        or uncertainty["resamples"] != 2000
    ):
        raise Scale02Error("uncertainty contract drifted")
    _hex_seed(uncertainty["seed"], "uncertainty.seed")

    policy = _exact(
        root["advisory_policy"],
        "advisory_policy",
        {
            "state",
            "required_repetitions",
            "max_errors",
            "max_timeouts",
            "steady_fts_p50_ms",
            "steady_fts_p99_ms",
            "mutation_to_ready_p99_ms",
            "max_peak_rss_fraction",
        },
    )
    policy_values = dict(policy)
    policy_state = policy_values.pop("state")
    if policy_state not in {"proposal_pending_hitl", "approved"} or policy_values != {
        "required_repetitions": 5,
        "max_errors": 0,
        "max_timeouts": 0,
        "steady_fts_p50_ms": 20,
        "steady_fts_p99_ms": 150,
        "mutation_to_ready_p99_ms": 5000,
        "max_peak_rss_fraction": 0.8,
    }:
        raise Scale02Error("advisory policy proposal drifted")
    if (approval["state"] == "approved") != (policy_state == "approved"):
        raise Scale02Error("live approval and advisory policy approval must match")

    runtime = _exact(
        root["runtime"],
        "runtime",
        {
            "python",
            "python_package_version",
            "python_extension",
            "python_extension_sha256",
            "fathomdb_bin",
            "fathomdb_bin_sha256",
        },
    )
    if runtime["python_package_version"] != RELEASE:
        raise Scale02Error("Python package release drifted")
    _digest(runtime["python_extension_sha256"], "runtime.python_extension_sha256")
    _digest(runtime["fathomdb_bin_sha256"], "runtime.fathomdb_bin_sha256")
    return Scale02Config(
        program_track=PROGRAM_TRACK,
        approval_state=approval["state"],
        approval_by=approval["approved_by"],
        approval_at=approval["approved_at"],
        profile_id=profile["id"],
        top_k=profile["top_k"],
        embed_device=profile["embed_device"],
        rerank_device=profile["rerank_device"],
        sizes=tuple(root["ladder"]),
        real_primary_count=corpus["real_primary_count"],
        repetitions=workload["repetitions"],
        write_batch_size=workload["write_batch_size"],
        cold_query_count=workload["cold_query_count"],
        cold_definition=workload["cold_definition"],
        warmup_query_count=workload["warmup_query_count"],
        steady_query_count=workload["steady_query_count"],
        steady_definition=workload["steady_definition"],
        mutation_count=workload["mutation_count"],
        query_order_seed=workload["query_order_seed"],
        drain_timeout_seconds=workload["drain_timeout_seconds"],
        query_timeout_ms=workload["query_timeout_ms"],
        bootstrap_resamples=uncertainty["resamples"],
        bootstrap_seed=uncertainty["seed"],
        policy=dict(policy),
        corpus_root=_path(corpus["root"], "corpus.root", external=True),
        corpus_index=corpus["index"],
        corpus_index_sha256=corpus["index_sha256"],
        qualified_manifest=_path(
            corpus["qualified_manifest"], "corpus.qualified_manifest", external=True
        ),
        qualified_manifest_sha256=corpus["qualified_manifest_sha256"],
        growth_seed=growth["seed"],
        python=_path(runtime["python"], "runtime.python"),
        python_package_version=runtime["python_package_version"],
        python_extension=_path(runtime["python_extension"], "runtime.python_extension"),
        python_extension_sha256=runtime["python_extension_sha256"],
        fathomdb_bin=_path(runtime["fathomdb_bin"], "runtime.fathomdb_bin"),
        fathomdb_bin_sha256=runtime["fathomdb_bin_sha256"],
        dependencies={name: dict(value) for name, value in dependencies.items()},
        resolved=json.loads(json.dumps(root)),
    )


def load_config(path: str | Path) -> Scale02Config:
    """Load and strictly validate a SCALE-02 JSON configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Scale02Error("SCALE-02 configuration is unavailable or invalid") from exc
    return resolve_config(document)


def _validate_runtime(config: Scale02Config) -> dict[str, str]:
    paths = (config.python, config.python_extension, config.fathomdb_bin)
    if any(not path.is_file() for path in paths):
        raise Scale02Error("runtime executable or extension is unavailable")
    if _sha_file(config.python_extension) != config.python_extension_sha256:
        raise Scale02Error("FathomDB Python extension digest drifted")
    if _sha_file(config.fathomdb_bin) != config.fathomdb_bin_sha256:
        raise Scale02Error("FathomDB CLI digest drifted")
    try:
        cli_version = subprocess.run(
            [str(config.fathomdb_bin), "--version"],
            check=True,
            capture_output=True,
            text=True,
            timeout=15,
        ).stdout.strip()
        package_version = subprocess.run(
            [
                str(config.python),
                "-c",
                "import fathomdb; print(fathomdb.__version__)",
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=15,
            cwd=REPO_ROOT,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError) as exc:
        raise Scale02Error("FathomDB 0.8.23 runtime attestation failed") from exc
    if cli_version != f"fathomdb {RELEASE}" or package_version != RELEASE:
        raise Scale02Error("FathomDB runtime version drifted")
    return {
        "release": RELEASE,
        "cli_sha256": config.fathomdb_bin_sha256,
        "python_extension_sha256": config.python_extension_sha256,
    }


def _safe_relative_file(root: Path, relative: object) -> Path:
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        raise Scale02Error("fixture payload path is not a safe relative path")
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()) or not path.is_file():
        raise Scale02Error("fixture payload is missing or escapes its root")
    return path


def _read_verified_text(root: Path, row: Mapping[str, object]) -> str:
    path = _safe_relative_file(root, row.get("relative_path"))
    text = path.read_text(encoding="utf-8")
    if not text or hashlib.sha256(text.encode()).hexdigest() != row.get(
        "content_sha256"
    ):
        raise Scale02Error("fixture payload digest drifted")
    return text


def _validate_dependencies(config: Scale02Config) -> None:
    for name, dependency in config.dependencies.items():
        path = _path(dependency["path"], f"dependencies.{name}.path")
        if not path.is_file() or _sha_file(path) != dependency["sha256"]:
            raise Scale02Error(f"{name} dependency receipt drifted")
        record = json.loads(path.read_text(encoding="utf-8"))
        if (
            record.get("run_id") != dependency["run_id"]
            or record.get("verdict") != "complete"
        ):
            raise Scale02Error(f"{name} dependency is not complete")
        if (
            name == "answer_01"
            and record.get("metrics", {}).get("decision") != "retain_a0"
        ):
            raise Scale02Error("ANSWER-01 no longer retains A0")
        if (
            name == "scale_01"
            and record.get("metrics", {}).get("document_count") != 17272
        ):
            raise Scale02Error("SCALE-01 primary scale drifted")


def load_fixture(config: Scale02Config) -> Fixture:
    """Load the qualified all-real prefix and fixed query catalog."""
    _validate_dependencies(config)
    index_path = config.corpus_root / config.corpus_index
    if not index_path.is_file() or _sha_file(index_path) != config.corpus_index_sha256:
        raise Scale02Error("corpus index digest drifted")
    if (
        not config.qualified_manifest.is_file()
        or _sha_file(config.qualified_manifest) != config.qualified_manifest_sha256
    ):
        raise Scale02Error("qualified manifest digest drifted")
    index = json.loads(index_path.read_text(encoding="utf-8"))
    qualified = json.loads(config.qualified_manifest.read_text(encoding="utf-8"))
    raw_documents, raw_queries = index.get("documents"), index.get("queries")
    qualified_documents = qualified.get("documents")
    if (
        index.get("schema_version") != "tc5-corpus-input.v1"
        or not isinstance(raw_documents, list)
        or len(raw_documents) != config.real_primary_count
        or not isinstance(qualified_documents, list)
        or len(qualified_documents) != config.real_primary_count
        or not isinstance(raw_queries, list)
        or len(raw_queries) != 100
    ):
        raise Scale02Error("qualified corpus or query envelope drifted")
    raw_by_id = {
        row.get("document_id"): row for row in raw_documents if isinstance(row, dict)
    }
    qualified_by_id = {
        row.get("document_id"): row
        for row in qualified_documents
        if isinstance(row, dict)
    }
    required = list(
        dict.fromkeys(
            row.get("exclude_document_id")
            for row in raw_queries
            if isinstance(row, dict)
        )
    )
    qualified_ids = [
        row.get("document_id") for row in qualified_documents if isinstance(row, dict)
    ]
    selected_ids = required + [
        identifier for identifier in qualified_ids if identifier not in required
    ]
    if (
        len(selected_ids) != config.real_primary_count
        or len(set(selected_ids)) != config.real_primary_count
    ):
        raise Scale02Error("real-prefix document selection drifted")
    documents: list[SourceDocument] = []
    fixture_rows: list[dict[str, str]] = []
    for identifier in selected_ids:
        raw, qualified_row = raw_by_id.get(identifier), qualified_by_id.get(identifier)
        if (
            not isinstance(raw, dict)
            or not isinstance(qualified_row, dict)
            or raw.get("content_sha256") != qualified_row.get("content_sha256")
        ):
            raise Scale02Error("document selection drifted from qualified manifest")
        text = _read_verified_text(config.corpus_root, raw)
        documents.append(
            SourceDocument(str(identifier), text, str(raw["content_sha256"]))
        )
        fixture_rows.append(
            {
                "document_id": str(identifier),
                "content_sha256": str(raw["content_sha256"]),
            }
        )
    queries: list[str] = []
    query_rows: list[dict[str, str]] = []
    for row in raw_queries:
        if not isinstance(row, dict) or row.get("exclude_document_id") not in raw_by_id:
            raise Scale02Error("query source is outside the qualified corpus")
        text = _read_verified_text(config.corpus_root, row)
        queries.append(text)
        query_rows.append(
            {
                "query_id": str(row.get("query_id")),
                "content_sha256": str(row.get("content_sha256")),
            }
        )
    return Fixture(
        documents=tuple(documents),
        queries=tuple(queries),
        fixture_digest=_canonical_digest(
            {
                "documents": fixture_rows,
                "queries": query_rows,
                "growth_seed": config.growth_seed,
            }
        ),
    )


def build_rows(
    documents: Sequence[SourceDocument], count: int, *, seed: str
) -> tuple[ScaleRow, ...]:
    """Build the real prefix plus disclosed deterministic derived filler."""
    if not documents or count < 1:
        raise Scale02Error(
            "scale fixture requires source documents and a positive count"
        )
    _hex_seed(seed, "fixture growth seed")
    real_count = min(len(documents), count)
    rows = [
        ScaleRow(
            document.document_id,
            document.text,
            f"scale-02:real:{document.document_id}",
            "real",
        )
        for document in documents[:real_count]
    ]
    start = int(seed, 16) % len(documents)
    for offset in range(count - real_count):
        source = documents[(start + offset) % len(documents)]
        logical_id = f"scale02-derived-{offset:05d}-{source.document_id}"
        rows.append(
            ScaleRow(
                logical_id,
                f"{source.text}\nscale02-derived-row-{offset:05d}",
                "scale-02:derived-fixture",
                "derived_fixture",
            )
        )
    return tuple(rows)


def dry_run(config_path: str | Path, *, output_root: Path) -> dict[str, object]:
    """Validate all inputs and controls without creating output or a database."""
    config = load_config(config_path)
    runtime = _validate_runtime(config)
    fixture = load_fixture(config)
    if (
        len(fixture.documents) != config.real_primary_count
        or len(fixture.queries) != 100
    ):
        raise Scale02Error("fixture completeness drifted")
    largest = config.sizes[-1]
    return {
        "schema_version": "scale-02-dry-run.v1",
        "state": "ready" if config.approval_state == "approved" else "awaiting_hitl",
        "program_track": PROGRAM_TRACK,
        "profile": config.profile_id,
        "ladder": list(config.sizes),
        "cell_count": len(config.sizes) * config.repetitions,
        "new_database_count": len(config.sizes) * config.repetitions,
        "fixture_digest": fixture.fixture_digest,
        "largest_point": {
            "canonical_records": largest,
            "real_records": config.real_primary_count,
            "derived_fixture_records": largest - config.real_primary_count,
        },
        "output_root_exists": output_root.exists(),
        "live_execution_authorized": config.approval_state == "approved",
        "policy_state": config.policy["state"],
        "cache_states": {
            "cold": config.cold_definition,
            "steady": config.steady_definition,
        },
        "runtime": runtime,
    }


def _percentile(values: Sequence[float], quantile: float) -> float:
    if not values:
        raise Scale02Error("metric sample is empty")
    ordered = sorted(float(value) for value in values)
    index = max(0, math.ceil(len(ordered) * quantile) - 1)
    return ordered[index]


def _latency_summary(values: Sequence[float]) -> dict[str, float | int]:
    total_ms = sum(values)
    return {
        "n": len(values),
        "p50": _percentile(values, 0.50),
        "p95": _percentile(values, 0.95),
        "p99": _percentile(values, 0.99),
        "throughput_per_second": 0.0
        if total_ms <= 0
        else len(values) * 1000.0 / total_ms,
    }


def _value_summary(values: Sequence[float]) -> dict[str, float | int]:
    return {
        "n": len(values),
        "p50": _percentile(values, 0.50),
        "p95": _percentile(values, 0.95),
        "p99": _percentile(values, 0.99),
    }


def _bootstrap_interval(config: Scale02Config, values: Sequence[float]) -> list[float]:
    if not values:
        raise Scale02Error("uncertainty sample is empty")
    randomizer = random.Random(int(config.bootstrap_seed, 16))
    estimates = []
    for _ in range(config.bootstrap_resamples):
        sample = [values[randomizer.randrange(len(values))] for _ in values]
        estimates.append(statistics.median(sample))
    return [_percentile(estimates, 0.025), _percentile(estimates, 0.975)]


_REPETITION_KEYS = {
    "schema_version",
    "point",
    "repetition",
    "real_records",
    "derived_fixture_records",
    "open_ms",
    "ingest_ack_ms",
    "ready_ms",
    "cold_query_ms",
    "steady_query_ms",
    "mutation_to_ready_ms",
    "errors",
    "timeouts",
    "database_bytes",
    "derived_index_bytes",
    "peak_rss_bytes",
    "host_memory_bytes",
    "peak_host_cpu_fraction",
    "peak_host_memory_fraction",
    "process_cpu_seconds",
    "effective_cpu_cores",
    "device",
    "doctor_sha256",
}


def aggregate_point(
    config: Scale02Config,
    point: int,
    repetitions: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    """Reduce complete payload-free repetitions to one safe point summary."""
    if point not in config.sizes or len(repetitions) != config.repetitions:
        raise Scale02Error("SCALE-02 point requires five repetitions")
    expected_repetitions = set(range(1, config.repetitions + 1))
    actual_repetitions: set[int] = set()
    cold: list[float] = []
    steady: list[float] = []
    mutation: list[float] = []
    for result in repetitions:
        if (
            set(result) != _REPETITION_KEYS
            or result.get("schema_version") != "scale-02-repetition.v1"
        ):
            raise Scale02Error("repetition schema drifted")
        if result["point"] != point:
            raise Scale02Error("repetition point drifted")
        actual_repetitions.add(result["repetition"])
        if result["real_records"] != min(point, config.real_primary_count) or result[
            "derived_fixture_records"
        ] != max(0, point - config.real_primary_count):
            raise Scale02Error("repetition corpus composition drifted")
        if result["device"] != {"embed": "cpu", "rerank": "cpu", "gpu": None}:
            raise Scale02Error("A0 repetition must remain CPU-only")
        _digest(result["doctor_sha256"], "repetition.doctor_sha256")
        ingest_ack = result["ingest_ack_ms"]
        ready = result["ready_ms"]
        if (
            not isinstance(ingest_ack, (int, float))
            or isinstance(ingest_ack, bool)
            or not math.isfinite(ingest_ack)
            or ingest_ack <= 0
            or not isinstance(ready, (int, float))
            or isinstance(ready, bool)
            or not math.isfinite(ready)
            or ready < ingest_ack
        ):
            raise Scale02Error(
                "repetition requires positive ingest timing and ready >= acknowledgement"
            )
        if (
            len(result["cold_query_ms"]) != config.cold_query_count
            or len(result["steady_query_ms"]) != config.steady_query_count
            or len(result["mutation_to_ready_ms"]) != config.mutation_count
        ):
            raise Scale02Error("repetition sample completeness drifted")
        cold.extend(float(value) for value in result["cold_query_ms"])
        steady.extend(float(value) for value in result["steady_query_ms"])
        mutation.extend(float(value) for value in result["mutation_to_ready_ms"])
    if actual_repetitions != expected_repetitions:
        raise Scale02Error("repetition identities are incomplete")

    steady_by_rep = [
        _percentile(result["steady_query_ms"], 0.99) for result in repetitions
    ]
    errors = sum(int(result["errors"]) for result in repetitions)
    timeouts = sum(int(result["timeouts"]) for result in repetitions)
    steady_summary = _latency_summary(steady)
    mutation_summary = _latency_summary(mutation)
    max_memory_fraction = max(
        float(result["peak_rss_bytes"]) / float(result["host_memory_bytes"])
        for result in repetitions
    )
    criteria = {
        "complete_repetitions": len(repetitions)
        == config.policy["required_repetitions"],
        "errors": errors <= config.policy["max_errors"],
        "timeouts": timeouts <= config.policy["max_timeouts"],
        "steady_fts_p50": steady_summary["p50"] <= config.policy["steady_fts_p50_ms"],
        "steady_fts_p99": steady_summary["p99"] <= config.policy["steady_fts_p99_ms"],
        "mutation_to_ready_p99": mutation_summary["p99"]
        <= config.policy["mutation_to_ready_p99_ms"],
        "peak_rss_fraction": max_memory_fraction
        <= config.policy["max_peak_rss_fraction"],
    }
    eligibility = (
        "pending_hitl"
        if config.policy["state"] != "approved"
        else ("pass" if all(criteria.values()) else "fail")
    )
    return {
        "schema_version": "scale-02-point-summary.v1",
        "program_track": PROGRAM_TRACK,
        "profile": config.profile_id,
        "point": point,
        "repetitions": config.repetitions,
        "canonical_records": point,
        "real_records": min(point, config.real_primary_count),
        "derived_fixture_records": max(0, point - config.real_primary_count),
        "cache_states": {
            "cold": {"latency_ms": _latency_summary(cold)},
            "steady": {
                "latency_ms": steady_summary,
                "p99_repetition_median_ci_95": _bootstrap_interval(
                    config, steady_by_rep
                ),
            },
        },
        "operations": {
            "open_ms": _latency_summary(
                [float(result["open_ms"]) for result in repetitions]
            ),
            "ingest_ack_ms": _latency_summary(
                [float(result["ingest_ack_ms"]) for result in repetitions]
            ),
            "ready_ms": _latency_summary(
                [float(result["ready_ms"]) for result in repetitions]
            ),
            "acknowledgement_to_ready_ms": _latency_summary(
                [
                    float(result["ready_ms"] - result["ingest_ack_ms"])
                    for result in repetitions
                ]
            ),
            "ingest_records_per_second": _value_summary(
                [
                    point * 1000.0 / float(result["ingest_ack_ms"])
                    for result in repetitions
                ]
            ),
            "mutation_to_ready_ms": mutation_summary,
        },
        "errors": errors,
        "timeouts": timeouts,
        "resources": {
            "peak_rss_bytes": max(
                int(result["peak_rss_bytes"]) for result in repetitions
            ),
            "host_memory_bytes": max(
                int(result["host_memory_bytes"]) for result in repetitions
            ),
            "max_peak_rss_fraction": max_memory_fraction,
            "peak_host_cpu_fraction": max(
                float(result["peak_host_cpu_fraction"]) for result in repetitions
            ),
            "peak_host_memory_fraction": max(
                float(result["peak_host_memory_fraction"]) for result in repetitions
            ),
            "process_cpu_seconds": sum(
                float(result["process_cpu_seconds"]) for result in repetitions
            ),
            "max_effective_cpu_cores": max(
                float(result["effective_cpu_cores"]) for result in repetitions
            ),
            "gpu": "not_applicable",
        },
        "storage": {
            "database_bytes": max(
                int(result["database_bytes"]) for result in repetitions
            ),
            "derived_index_bytes": max(
                int(result["derived_index_bytes"]) for result in repetitions
            ),
            "projection_amplification": max(
                float(result["derived_index_bytes"])
                / max(
                    1, float(result["database_bytes"] - result["derived_index_bytes"])
                )
                for result in repetitions
            ),
        },
        "advisory": {
            "state": config.policy["state"],
            "eligibility": eligibility,
            "criteria": criteria,
        },
        "provenance": {
            "doctor_sha256s": [str(result["doctor_sha256"]) for result in repetitions],
            "device": {"embed": "cpu", "rerank": "cpu", "gpu": None},
        },
        "claim_boundary": "advisory_a0_efficiency_only",
    }


def _process_peak_rss_bytes() -> int:
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return int(value if platform.system() == "Darwin" else value * 1024)


def _current_rss_bytes() -> int:
    try:
        for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except OSError:
        pass
    return _process_peak_rss_bytes()


def _host_memory_bytes() -> int:
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                return int(line.split()[1]) * 1024
    except OSError:
        pass
    page_size = os.sysconf("SC_PAGE_SIZE")
    return int(page_size * os.sysconf("SC_PHYS_PAGES"))


def _host_memory_fraction() -> float:
    try:
        values: dict[str, int] = {}
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            name, _, remainder = line.partition(":")
            if name in {"MemTotal", "MemAvailable"}:
                values[name] = int(remainder.split()[0])
        return 1.0 - values["MemAvailable"] / values["MemTotal"]
    except (KeyError, OSError, ValueError, ZeroDivisionError):
        return 0.0


def _host_cpu_times() -> tuple[int, int] | None:
    try:
        fields = Path("/proc/stat").read_text(encoding="utf-8").splitlines()[0].split()
        if fields[0] != "cpu":
            return None
        values = [int(value) for value in fields[1:]]
        idle = values[3] + (values[4] if len(values) > 4 else 0)
        return sum(values), sum(values) - idle
    except (IndexError, OSError, ValueError):
        return None


class _ResourceSampler:
    """Sample per-process RSS and whole-host pressure during one repetition."""

    def __init__(self) -> None:
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        self._peak_rss_bytes = 0
        self._peak_host_cpu_fraction = 0.0
        self._peak_host_memory_fraction = 0.0
        self._prior_cpu: tuple[int, int] | None = None
        self._process_started = 0.0
        self._wall_started = 0.0

    def _sample(self) -> None:
        self._peak_rss_bytes = max(self._peak_rss_bytes, _current_rss_bytes())
        self._peak_host_memory_fraction = max(
            self._peak_host_memory_fraction, _host_memory_fraction()
        )
        current = _host_cpu_times()
        if current is not None and self._prior_cpu is not None:
            total_delta = current[0] - self._prior_cpu[0]
            busy_delta = current[1] - self._prior_cpu[1]
            if total_delta > 0:
                self._peak_host_cpu_fraction = max(
                    self._peak_host_cpu_fraction, busy_delta / total_delta
                )
        self._prior_cpu = current

    def _run(self) -> None:
        while not self._stop.wait(0.05):
            self._sample()

    def start(self) -> None:
        self._process_started = time.process_time()
        self._wall_started = time.perf_counter()
        self._sample()
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def stop(self) -> dict[str, float | int]:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=1)
        self._sample()
        process_seconds = max(0.0, time.process_time() - self._process_started)
        wall_seconds = max(0.0, time.perf_counter() - self._wall_started)
        return {
            "peak_rss_bytes": self._peak_rss_bytes,
            "peak_host_cpu_fraction": self._peak_host_cpu_fraction,
            "peak_host_memory_fraction": self._peak_host_memory_fraction,
            "process_cpu_seconds": process_seconds,
            "effective_cpu_cores": 0.0
            if wall_seconds == 0
            else process_seconds / wall_seconds,
        }


def _database_storage(path: Path) -> tuple[int, int]:
    total = sum(
        candidate.stat().st_size
        for candidate in (path, Path(f"{path}-wal"), Path(f"{path}-shm"))
        if candidate.exists()
    )
    derived = 0
    try:
        with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
            rows = connection.execute(
                "SELECT name, SUM(pgsize) FROM dbstat GROUP BY name"
            ).fetchall()
        derived = sum(
            int(size or 0)
            for name, size in rows
            if any(
                marker in str(name).lower()
                for marker in ("fts", "search_index", "vec", "projection")
            )
        )
    except sqlite3.Error:
        derived = 0
    return total, derived


def _ordered_queries(queries: Sequence[str], count: int, seed: int) -> list[str]:
    if not queries:
        raise Scale02Error("query catalog is empty")
    order = list(queries)
    random.Random(seed).shuffle(order)
    return [order[index % len(order)] for index in range(count)]


def _time_queries(
    engine: Any, queries: Sequence[str], config: Scale02Config
) -> tuple[list[float], int, int]:
    elapsed: list[float] = []
    errors = 0
    timeouts = 0
    for query in queries:
        started = time.perf_counter()
        try:
            engine.search_text_only(query, limit=config.top_k)
        except Exception:
            errors += 1
            continue
        duration = (time.perf_counter() - started) * 1000.0
        elapsed.append(duration)
        if duration > config.query_timeout_ms:
            timeouts += 1
    return elapsed, errors, timeouts


def _execute_repetition(
    config: Scale02Config,
    fixture: Fixture,
    rows: Sequence[ScaleRow],
    *,
    point_root: Path,
    point: int,
    repetition: int,
    engine_factory: Callable[[str], Any] | None = None,
    prepare: Callable[..., PreparedDatabase] = prepare_test_database,
) -> dict[str, Any]:
    prepared = prepare(
        point_root,
        test_id=f"point-{point}-rep-{repetition}",
        embed_device=config.embed_device,
        rerank_device=config.rerank_device,
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=str(config.fathomdb_bin),
    )
    if engine_factory is None:

        def open_engine(path: str) -> Any:
            engine_class = importlib.import_module("fathomdb").Engine
            return engine_class.open(path, use_default_embedder=False)

        engine_factory = open_engine
    errors = 0
    timeouts = 0
    sampler = _ResourceSampler()
    sampler.start()
    try:
        ingest_engine = engine_factory(str(prepared.database_path))
        try:
            ingest_started = time.perf_counter()
            for offset in range(0, len(rows), config.write_batch_size):
                batch = [
                    {
                        "kind": "scale_02_memory",
                        "body": row.body,
                        "source_id": row.source_id,
                        "logical_id": row.logical_id,
                    }
                    for row in rows[offset : offset + config.write_batch_size]
                ]
                ingest_engine.write(batch)
            ingest_ack_ms = (time.perf_counter() - ingest_started) * 1000.0
            ingest_engine.drain(timeout_s=config.drain_timeout_seconds)
            ready_ms = (time.perf_counter() - ingest_started) * 1000.0
        finally:
            ingest_engine.close()

        opened = time.perf_counter()
        engine = engine_factory(str(prepared.database_path))
        open_ms = (time.perf_counter() - opened) * 1000.0
        try:
            seed = int(config.query_order_seed, 16) + repetition + point
            cold_queries = _ordered_queries(
                fixture.queries, config.cold_query_count, seed
            )
            cold, cold_errors, cold_timeouts = _time_queries(
                engine, cold_queries, config
            )
            errors += cold_errors
            timeouts += cold_timeouts
            warmup = _ordered_queries(
                fixture.queries, config.warmup_query_count, seed + 1
            )
            _, warmup_errors, warmup_timeouts = _time_queries(engine, warmup, config)
            errors += warmup_errors
            timeouts += warmup_timeouts
            steady_queries = _ordered_queries(
                fixture.queries, config.steady_query_count, seed + 2
            )
            steady, steady_errors, steady_timeouts = _time_queries(
                engine, steady_queries, config
            )
            errors += steady_errors
            timeouts += steady_timeouts
            database_bytes, derived_index_bytes = _database_storage(
                prepared.database_path
            )

            mutation: list[float] = []
            for index in range(config.mutation_count):
                token = f"scale02mutationtoken{point}r{repetition}m{index}"
                began = time.perf_counter()
                engine.write(
                    [
                        {
                            "kind": "scale_02_mutation",
                            "body": f"mutation readiness {token}",
                            "source_id": "scale-02:mutation-probe",
                            "logical_id": token,
                        }
                    ]
                )
                engine.drain(timeout_s=config.drain_timeout_seconds)
                result = engine.search_text_only(token, limit=1)
                mutation.append((time.perf_counter() - began) * 1000.0)
                if not result.results or token not in result.results[0].body:
                    errors += 1
        finally:
            engine.close()
    finally:
        resource_metrics = sampler.stop()
    result = {
        "schema_version": "scale-02-repetition.v1",
        "point": point,
        "repetition": repetition,
        "real_records": min(point, config.real_primary_count),
        "derived_fixture_records": max(0, point - config.real_primary_count),
        "open_ms": open_ms,
        "ingest_ack_ms": ingest_ack_ms,
        "ready_ms": ready_ms,
        "cold_query_ms": cold,
        "steady_query_ms": steady,
        "mutation_to_ready_ms": mutation,
        "errors": errors,
        "timeouts": timeouts,
        "database_bytes": database_bytes,
        "derived_index_bytes": derived_index_bytes,
        "peak_rss_bytes": resource_metrics["peak_rss_bytes"],
        "host_memory_bytes": _host_memory_bytes(),
        "peak_host_cpu_fraction": resource_metrics["peak_host_cpu_fraction"],
        "peak_host_memory_fraction": resource_metrics["peak_host_memory_fraction"],
        "process_cpu_seconds": resource_metrics["process_cpu_seconds"],
        "effective_cpu_cores": resource_metrics["effective_cpu_cores"],
        "device": {
            "embed": config.embed_device,
            "rerank": config.rerank_device,
            "gpu": None,
        },
        "doctor_sha256": _sha_file(prepared.doctor_path),
    }
    output_path = prepared.database_path.parent / "scale-02-repetition.v1.json"
    output_path.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    output_path.chmod(0o600)
    return result


def _registry_root(record_base_dir: Path | None) -> Path:
    return record_base_dir if record_base_dir is not None else _lib.EXPERIMENTS_DIR


def _require_prior_points(
    config: Scale02Config, point: int, record_base_dir: Path | None
) -> None:
    prior_points = config.sizes[: config.sizes.index(point)]
    if not prior_points:
        return
    passed: set[int] = set()
    for path in (_registry_root(record_base_dir) / "runs").glob("*/record.json"):
        try:
            record = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        metrics = record.get("metrics", {})
        if (
            isinstance(metrics, dict)
            and record.get("experiment") == f"scale-02-a0-{metrics.get('point')}"
            and record.get("verdict") == "complete"
            and metrics.get("advisory", {}).get("eligibility") == "pass"
        ):
            passed.add(metrics["point"])
    for prior in prior_points:
        if prior not in passed:
            raise Scale02Error(f"prior ladder point {prior} has no passing receipt")


def _write_point_record(
    config: Scale02Config,
    config_path: str | Path,
    point: int,
    *,
    metrics: dict[str, Any],
    verdict: str,
    read: str,
    fixture_digest: str | None,
    record_base_dir: Path | None,
    artifacts: list[dict[str, str]] | None = None,
    open_questions: list[str] | None = None,
) -> str:
    resolved = json.loads(json.dumps(config.resolved))
    resolved["execution_point"] = point
    code = _lib.git_info()
    code["baseline_commit"] = None
    env = _lib.env_info(
        key_deps={
            "fathomdb": RELEASE,
            "fathomdb_cli_sha256": config.fathomdb_bin_sha256,
            "fathomdb_python_extension_sha256": config.python_extension_sha256,
        }
    )
    run_id, _ = _lib.write_record(
        f"scale-02-a0-{point}",
        ts=datetime.now(UTC),
        config_obj=resolved,
        metrics=metrics,
        verdict=verdict,
        read=read,
        code=code,
        corpus={
            "source": "TC-5 qualified real prefix plus disclosed derived fixture",
            "manifest_sha256": fixture_digest,
            "datasets": ["tc5-qualified-real-v2"] if fixture_digest else [],
        },
        seeds={
            "query_order": config.query_order_seed,
            "bootstrap": config.bootstrap_seed,
            "growth": config.growth_seed,
        },
        env=env,
        cost_usd=0.0,
        headline={"point": point, "eligibility": metrics.get("advisory", {}).get("eligibility", "blocked")},
        n=point,
        config_path=str(config_path),
        tests=["tests/experiments/test_scale_02.py"],
        files_changed=[],
        artifacts=artifacts or [],
        review=None,
        open_questions=open_questions or [],
        base_dir=record_base_dir,
    )
    if record_base_dir is None:
        _lib.regen_index_md()
    else:
        _lib.regen_index_md(
            index_path=record_base_dir / "index.jsonl",
            md_path=record_base_dir / "INDEX.md",
        )
    return run_id


def run_point(
    config_path: str | Path,
    point: int,
    *,
    output_root: Path,
    engine_factory: Callable[[str], Any] | None = None,
    prepare: Callable[..., PreparedDatabase] = prepare_test_database,
    record_base_dir: Path | None = None,
) -> dict[str, Any]:
    """Execute and register one size point; never advances the ladder itself."""
    config = load_config(config_path)
    if config.approval_state != "approved" or config.policy["state"] != "approved":
        raise Scale02Error("live SCALE-02 execution requires explicit HITL approval")
    if point not in config.sizes:
        raise Scale02Error("point is outside the preregistered ladder")
    if output_root.resolve().is_relative_to(REPO_ROOT.resolve()):
        raise Scale02Error("SCALE-02 artifacts must remain outside the repository")
    _require_prior_points(config, point, record_base_dir)
    fixture: Fixture | None = None
    repetitions: list[dict[str, Any]] = []
    try:
        _validate_runtime(config)
        fixture = load_fixture(config)
        rows = build_rows(fixture.documents, point, seed=config.growth_seed)
        point_root = output_root / f"point-{point}"
        if point_root.exists():
            raise Scale02Error("SCALE-02 output point already exists")
        point_root.mkdir(parents=True, mode=0o700)
        for index in range(1, config.repetitions + 1):
            repetitions.append(
                _execute_repetition(
                    config,
                    fixture,
                    rows,
                    point_root=point_root,
                    point=point,
                    repetition=index,
                    engine_factory=engine_factory,
                    prepare=prepare,
                )
            )
        summary = aggregate_point(config, point, repetitions)
    except Exception as exc:
        blocked = {
            "schema_version": "scale-02-blocked.v1",
            "program_track": PROGRAM_TRACK,
            "point": point,
            "completed_repetitions": len(repetitions),
            "error_type": type(exc).__name__,
            "claim_boundary": "no_performance_claim",
        }
        _write_point_record(
            config,
            config_path,
            point,
            metrics=blocked,
            verdict="blocked_execution",
            read=f"SCALE-02 A0 {point:,}-record point blocked during execution",
            fixture_digest=fixture.fixture_digest if fixture else None,
            record_base_dir=record_base_dir,
            open_questions=["resolve the execution failure before retrying this point"],
        )
        raise Scale02Error(f"SCALE-02 point blocked: {exc}") from exc
    summary["fixture_digest"] = fixture.fixture_digest
    summary_path = point_root / "scale-02-point-summary.v1.json"
    summary_path.write_text(
        json.dumps(summary, sort_keys=True) + "\n", encoding="utf-8"
    )
    summary_path.chmod(0o600)

    run_id = _write_point_record(
        config,
        config_path,
        point,
        metrics=summary,
        verdict="complete"
        if summary["advisory"]["eligibility"] == "pass"
        else "advisory_limit_observed",
        read=f"SCALE-02 A0 {point:,}-record point: {summary['advisory']['eligibility']}",
        fixture_digest=fixture.fixture_digest,
        record_base_dir=record_base_dir,
        artifacts=[
            {"kind": "external_safe_summary", "sha256": _sha_file(summary_path)}
        ],
        open_questions=[]
        if summary["advisory"]["eligibility"] == "pass"
        else ["advisory boundary reached at this point"],
    )
    return {
        "run_id": run_id,
        "summary": summary,
        "external_summary_sha256": _sha_file(summary_path),
    }


def main(argv: list[str] | None = None) -> int:
    """Validate, dry-run, or execute one SCALE-02 ladder point."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate")
    validate.add_argument("config", type=Path)
    dry = subparsers.add_parser("dry-run")
    dry.add_argument("config", type=Path)
    dry.add_argument("output_root", type=Path)
    run = subparsers.add_parser("run-point")
    run.add_argument("config", type=Path)
    run.add_argument("point", type=int)
    run.add_argument("output_root", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "validate":
            load_config(args.config)
            result: object = {"state": "valid"}
        elif args.command == "dry-run":
            result = dry_run(args.config, output_root=args.output_root)
        else:
            result = run_point(args.config, args.point, output_root=args.output_root)
    except (OSError, Scale02Error, json.JSONDecodeError) as exc:
        parser.error(str(exc))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
