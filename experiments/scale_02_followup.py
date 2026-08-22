"""Run the registered compact SCALE-02 FTS tuning and equivalence follow-up."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import random
import shutil
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from dataclasses import replace
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Iterator, Mapping, Sequence

from experiments import _lib, answer_01, scale_02
from experiments.fathomdb_test_setup import prepare_test_database


PROGRAM_TRACK = "SCALE-02"
INPUT_MANIFEST_SCHEMA = "scale-02-input-manifest.v1"
READER_OBSERVATION_SCHEMA = "scale-02-reader-settings.v1"
REPO_ROOT = Path(__file__).resolve().parent.parent
_DIGEST_LENGTH = 64
_READER_KEYS = {
    "schema_version",
    "requested",
    "cache_size",
    "mmap_size",
    "temp_store",
    "sqlite_version",
}
TUNING_SCHEMA = "scale-02-fts-tuning.v1"
_CONFIG_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "approval",
    "dependencies",
    "inputs",
    "treatments",
    "workload",
    "equivalence",
    "selection_policy",
    "runtime",
    "artifact_root",
    "claim_boundary",
}


class Scale02FollowupError(ValueError):
    """Raised when follow-up configuration or evidence fails closed."""


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Scale02FollowupError(
            f"{label} keys drifted: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _digest(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != _DIGEST_LENGTH
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Scale02FollowupError(f"{label} must be a lowercase sha256")
    return value


def _resolved_path(value: object, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise Scale02FollowupError(f"{label} must be a non-empty path")
    path = Path(value)
    return (path if path.is_absolute() else REPO_ROOT / path).resolve()


def load_config(path: str | Path) -> dict[str, Any]:
    """Load and strictly validate the registered follow-up configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Scale02FollowupError("follow-up configuration is unavailable") from exc
    root = _exact(document, "config", _CONFIG_KEYS)
    if (
        root["schema_version"] != TUNING_SCHEMA
        or root["program_track"] != PROGRAM_TRACK
        or root["release"] != "0.8.23"
        or root["claim_boundary"] != "tuning_and_retrieval_equivalence_only"
    ):
        raise Scale02FollowupError("follow-up identity or claim boundary drifted")
    approval = _exact(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] != "approved" or approval["approved_by"] != "HITL":
        raise Scale02FollowupError("follow-up execution requires HITL approval")
    if not isinstance(approval["approved_at"], str) or not approval["approved_at"]:
        raise Scale02FollowupError("follow-up approval timestamp is missing")

    dependencies = _exact(
        root["dependencies"],
        "dependencies",
        {"baseline", "answer_01", "scale_01", "base_config"},
    )
    expected_runs = {
        "baseline": "scale-02-a0-10000-20260822T1715Z-77c37c77",
        "answer_01": "answer-01-shortlist-live-20260822T1234Z-8a050808",
        "scale_01": "tc5-gpu-primary-20260822T1605Z-2d574205",
    }
    for name, dependency in dependencies.items():
        keys = {"path", "sha256"} | ({"run_id"} if name != "base_config" else set())
        item = _exact(dependency, f"dependencies.{name}", keys)
        dependency_path = _resolved_path(item["path"], f"dependencies.{name}.path")
        if not dependency_path.is_file() or sha_file(dependency_path) != _digest(
            item["sha256"], f"dependencies.{name}.sha256"
        ):
            raise Scale02FollowupError(f"dependency {name} drifted")
        if name != "base_config":
            record = json.loads(dependency_path.read_text(encoding="utf-8"))
            if item["run_id"] != expected_runs[name] or record.get("run_id") != item["run_id"]:
                raise Scale02FollowupError(f"dependency {name} run drifted")

    inputs = _exact(
        root["inputs"],
        "inputs",
        {
            "root",
            "manifest",
            "manifest_sha256",
            "tc5_index",
            "tc5_qualified_manifest",
            "locomo_corpus",
            "locomo_subset",
        },
    )
    input_root = _resolved_path(inputs["root"], "inputs.root")
    manifest_path = input_root / inputs["manifest"]
    if not manifest_path.is_file() or sha_file(manifest_path) != _digest(
        inputs["manifest_sha256"], "inputs.manifest_sha256"
    ):
        raise Scale02FollowupError("input manifest drifted")
    validate_input_manifest(input_root, manifest_path)
    for key in (
        "tc5_index",
        "tc5_qualified_manifest",
        "locomo_corpus",
        "locomo_subset",
    ):
        candidate = (input_root / inputs[key]).resolve()
        if not candidate.is_relative_to(input_root) or not candidate.is_file():
            raise Scale02FollowupError(f"input {key} is unavailable")

    treatments = _exact(
        root["treatments"],
        "treatments",
        {"cells", "reader_profiles", "concurrency", "formal_concurrency"},
    )
    profiles = treatments["reader_profiles"]
    if not isinstance(profiles, list) or [item.get("id") for item in profiles] != [
        "default",
        "mmap128",
        "mmap256",
        "cache64",
        "mmap256_cache64",
    ]:
        raise Scale02FollowupError("reader profile matrix drifted")
    for profile in profiles:
        _exact(profile, "reader profile", {"id", "pragmas", "expected", "footprint_order"})
        _exact(profile["expected"], "reader expected", {"cache_size", "mmap_size", "temp_store"})
        if not isinstance(profile["pragmas"], str):
            raise Scale02FollowupError("reader pragmas must be a string")
    cells = treatments["cells"]
    expected_cells = [
        ("current_default", "current", "default"),
        ("rank_default", "rank_fast", "default"),
        ("rank_mmap128", "rank_fast", "mmap128"),
        ("rank_mmap256", "rank_fast", "mmap256"),
        ("rank_cache64", "rank_fast", "cache64"),
        ("rank_mmap256_cache64", "rank_fast", "mmap256_cache64"),
    ]
    if not isinstance(cells, list):
        raise Scale02FollowupError("tuning cell matrix drifted")
    for cell in cells:
        _exact(cell, "tuning cell", {"id", "query_path", "reader_profile"})
    if [
        (item.get("id"), item.get("query_path"), item.get("reader_profile"))
        for item in cells
    ] != expected_cells:
        raise Scale02FollowupError("tuning cell matrix drifted")
    if treatments["concurrency"] != [1, 2, 4] or treatments["formal_concurrency"] != 1:
        raise Scale02FollowupError("concurrency matrix drifted")

    workload = _exact(
        root["workload"],
        "workload",
        {
            "point",
            "repetitions",
            "cold_queries",
            "warmup_queries",
            "steady_queries",
            "mutations",
            "query_order_seed",
            "bootstrap_seed",
            "bootstrap_resamples",
        },
    )
    if workload != {
        "point": 10000,
        "repetitions": 5,
        "cold_queries": 100,
        "warmup_queries": 100,
        "steady_queries": 1000,
        "mutations": 20,
        "query_order_seed": "0x5CA1E0200A0F75",
        "bootstrap_seed": "0x5CA1E020B00757",
        "bootstrap_resamples": 2000,
    }:
        raise Scale02FollowupError("follow-up workload drifted")
    if root["equivalence"] != {"tc5_queries": 100, "answer_01_queries": 32, "top_k": 10}:
        raise Scale02FollowupError("equivalence workload drifted")
    if root["selection_policy"] != {
        "steady_p50_ms": 20,
        "steady_p99_ms": 150,
        "max_rss_fraction": 0.8,
        "confidence": 0.95,
        "selection": "lowest_footprint_eligible",
    }:
        raise Scale02FollowupError("selection policy drifted")

    runtime = _exact(
        root["runtime"],
        "runtime",
        {
            "python",
            "python_extension",
            "python_extension_sha256",
            "fathomdb_bin",
            "fathomdb_bin_sha256",
            "wheel",
            "wheel_sha256",
            "source_git_sha",
        },
    )
    for key in ("python_extension_sha256", "fathomdb_bin_sha256", "wheel_sha256"):
        _digest(runtime[key], f"runtime.{key}")
    for key, digest_key in (
        ("python_extension", "python_extension_sha256"),
        ("fathomdb_bin", "fathomdb_bin_sha256"),
        ("wheel", "wheel_sha256"),
    ):
        runtime_path = _resolved_path(runtime[key], f"runtime.{key}")
        if not runtime_path.is_file() or sha_file(runtime_path) != runtime[digest_key]:
            raise Scale02FollowupError(f"runtime {key} drifted")
    python = _resolved_path(runtime["python"], "runtime.python")
    if not python.is_file():
        raise Scale02FollowupError("runtime Python is unavailable")
    if not isinstance(runtime["source_git_sha"], str) or len(runtime["source_git_sha"]) != 40:
        raise Scale02FollowupError("runtime source git SHA is invalid")
    artifact_root = _resolved_path(root["artifact_root"], "artifact_root")
    if artifact_root.is_relative_to(REPO_ROOT.resolve()):
        raise Scale02FollowupError("artifact root must remain outside the worktree")
    return root


def validate_input_manifest(root: Path, manifest_path: Path) -> None:
    """Validate every frozen input file against the external manifest."""
    manifest = _exact(
        json.loads(manifest_path.read_text(encoding="utf-8")),
        "input manifest",
        {"schema_version", "program_track", "pack_id", "file_count", "total_bytes", "files"},
    )
    if (
        manifest["schema_version"] != INPUT_MANIFEST_SCHEMA
        or manifest["program_track"] != PROGRAM_TRACK
        or manifest["pack_id"] != "scale-02-input-pack-v1"
        or not isinstance(manifest["files"], list)
        or manifest["file_count"] != len(manifest["files"])
    ):
        raise Scale02FollowupError("input manifest identity drifted")
    total = 0
    for item in manifest["files"]:
        row = _exact(item, "input file", {"path", "bytes", "sha256"})
        relative = Path(row["path"])
        path = (root / relative).resolve()
        if relative.is_absolute() or not path.is_relative_to(root) or not path.is_file():
            raise Scale02FollowupError("input manifest path escaped its root")
        if path.stat().st_size != row["bytes"] or sha_file(path) != _digest(
            row["sha256"], "input file sha256"
        ):
            raise Scale02FollowupError("frozen input file drifted")
        total += row["bytes"]
    if total != manifest["total_bytes"]:
        raise Scale02FollowupError("input manifest byte total drifted")


def sha_file(path: str | Path) -> str:
    """Return a streaming SHA-256 digest for one file."""
    digest = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _file_inventory(root: Path) -> list[dict[str, Any]]:
    files = []
    for path in sorted(candidate for candidate in root.rglob("*") if candidate.is_file()):
        if path.name == "input-manifest.v1.json":
            continue
        relative = path.relative_to(root)
        files.append(
            {
                "path": relative.as_posix(),
                "bytes": path.stat().st_size,
                "sha256": sha_file(path),
            }
        )
    return files


def freeze_inputs(
    *,
    tc5_root: Path,
    tc5_qualified_manifest: Path,
    locomo_corpus: Path,
    locomo_subset: Path,
    output_root: Path,
) -> dict[str, Any]:
    """Copy the qualified input packs once and write a deterministic manifest."""
    sources = (tc5_root, tc5_qualified_manifest, locomo_corpus, locomo_subset)
    if not tc5_root.is_dir() or any(not path.is_file() for path in sources[1:]):
        raise Scale02FollowupError("one or more frozen input sources are unavailable")
    if output_root.exists():
        raise Scale02FollowupError("input pack output already exists")
    output_root.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=".scale-02-input-pack-", dir=output_root.parent)
    )
    try:
        shutil.copytree(tc5_root, staging / "tc5")
        shutil.copy2(tc5_qualified_manifest, staging / "tc5-qualified-manifest.json")
        shutil.copy2(locomo_corpus, staging / "locomo10.json")
        shutil.copy2(locomo_subset, staging / "locomo-fixed-subset.json")
        files = _file_inventory(staging)
        manifest = {
            "schema_version": INPUT_MANIFEST_SCHEMA,
            "program_track": PROGRAM_TRACK,
            "pack_id": "scale-02-input-pack-v1",
            "file_count": len(files),
            "total_bytes": sum(item["bytes"] for item in files),
            "files": files,
        }
        manifest_path = staging / "input-manifest.v1.json"
        manifest_path.write_text(
            json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        manifest_path.chmod(0o600)
        staging.replace(output_root)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "schema_version": "scale-02-input-freeze-result.v1",
        "manifest_sha256": sha_file(output_root / "input-manifest.v1.json"),
        "file_count": len(files),
        "total_bytes": sum(item["bytes"] for item in files),
    }


def validate_reader_observations(
    rows: Sequence[Mapping[str, Any]], expected: Mapping[str, int]
) -> dict[str, Any]:
    """Require every opened reader to report the configured effective settings."""
    if len(rows) < 8:
        raise Scale02FollowupError("reader witness has fewer than eight connections")
    versions: set[str] = set()
    for row in rows:
        if set(row) != _READER_KEYS or row.get("schema_version") != READER_OBSERVATION_SCHEMA:
            raise Scale02FollowupError("reader observation schema drifted")
        for key, value in expected.items():
            if row.get(key) != value:
                raise Scale02FollowupError(f"reader setting {key} drifted")
        version = row.get("sqlite_version")
        if not isinstance(version, str) or not version:
            raise Scale02FollowupError("reader SQLite version is missing")
        versions.add(version)
    if len(versions) != 1:
        raise Scale02FollowupError("reader SQLite versions drifted")
    return {
        "schema_version": "scale-02-reader-observation-summary.v1",
        "connection_count": len(rows),
        "sqlite_version": next(iter(versions)),
        "effective": dict(expected),
    }


def _quality_unchanged(equivalence: Mapping[str, Mapping[str, Any]]) -> bool:
    return (
        set(equivalence) == {"tc5", "answer_01"}
        and equivalence["tc5"].get("query_count") == 100
        and equivalence["answer_01"].get("query_count") == 32
        and equivalence["tc5"].get("mismatch_count") == 0
        and equivalence["answer_01"].get("mismatch_count") == 0
    )


def select_candidate(
    cells: Sequence[Mapping[str, Any]],
    equivalence: Mapping[str, Mapping[str, Any]],
    *,
    policy: Mapping[str, float],
) -> dict[str, Any]:
    """Apply the preregistered quality and upper-bound eligibility rule."""
    quality_unchanged = _quality_unchanged(equivalence)
    decisions = []
    for cell in cells:
        upper = cell.get("upper_95", {})
        reasons = []
        if cell.get("query_path") != "rank_fast":
            reasons.append("not_rank_fast_candidate")
        if not cell.get("complete_repetitions"):
            reasons.append("incomplete_repetitions")
        if cell.get("errors") != 0:
            reasons.append("errors")
        if cell.get("timeouts") != 0:
            reasons.append("timeouts")
        if upper.get("steady_p50_ms", float("inf")) > policy["steady_p50_ms"]:
            reasons.append("steady_p50_upper")
        if upper.get("steady_p99_ms", float("inf")) > policy["steady_p99_ms"]:
            reasons.append("steady_p99_upper")
        if upper.get("rss_fraction", float("inf")) > policy["max_rss_fraction"]:
            reasons.append("rss_upper")
        if not quality_unchanged:
            reasons.append("retrieval_equivalence")
        decisions.append(
            {
                "cell_id": cell.get("id"),
                "eligible": not reasons,
                "reasons": reasons,
                "footprint_order": cell.get("footprint_order"),
            }
        )
    eligible = [item for item in decisions if item["eligible"]]
    eligible.sort(key=lambda item: (item["footprint_order"], item["cell_id"]))
    return {
        "schema_version": "scale-02-fts-selection.v1",
        "state": "recommendation_pending_hitl",
        "quality_applicability": "unchanged" if quality_unchanged else "changed",
        "recommended_cell": eligible[0]["cell_id"] if eligible else None,
        "candidates": decisions,
    }


@contextmanager
def _treatment_environment(
    profile: Mapping[str, Any],
    *,
    rank_fast: bool,
    reader_witness: Path,
    route_witness: Path | None = None,
) -> Iterator[None]:
    keys = (
        "FATHOMDB_PERF_EXPERIMENTS",
        "FATHOMDB_PERF_FTS_RANK_FAST",
        "FATHOMDB_PERF_READER_PRAGMAS",
        "FATHOMDB_PERF_READER_PRAGMA_WITNESS",
        "FATHOMDB_PERF_FTS_ROUTE_WITNESS",
    )
    prior = {key: os.environ.get(key) for key in keys}
    os.environ["FATHOMDB_PERF_EXPERIMENTS"] = "1"
    if rank_fast:
        os.environ["FATHOMDB_PERF_FTS_RANK_FAST"] = "1"
    else:
        os.environ.pop("FATHOMDB_PERF_FTS_RANK_FAST", None)
    pragmas = profile["pragmas"]
    if pragmas:
        os.environ["FATHOMDB_PERF_READER_PRAGMAS"] = pragmas
    else:
        os.environ.pop("FATHOMDB_PERF_READER_PRAGMAS", None)
    os.environ["FATHOMDB_PERF_READER_PRAGMA_WITNESS"] = str(reader_witness)
    if route_witness is None:
        os.environ.pop("FATHOMDB_PERF_FTS_ROUTE_WITNESS", None)
    else:
        os.environ["FATHOMDB_PERF_FTS_ROUTE_WITNESS"] = str(route_witness)
    try:
        yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def _jsonl(path: Path) -> list[dict[str, Any]]:
    try:
        rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
    except (OSError, json.JSONDecodeError) as exc:
        raise Scale02FollowupError(f"invalid JSONL witness: {path.name}") from exc
    if any(not isinstance(row, dict) for row in rows):
        raise Scale02FollowupError(f"invalid JSONL witness: {path.name}")
    return rows


def _bootstrap_upper(
    values: Sequence[float], *, seed: int, resamples: int
) -> float:
    if not values:
        raise Scale02FollowupError("cannot bootstrap an empty metric")
    randomizer = random.Random(seed)
    estimates = []
    for _ in range(resamples):
        sample = [values[randomizer.randrange(len(values))] for _ in values]
        estimates.append(scale_02._percentile(sample, 0.50))
    return scale_02._percentile(estimates, 0.975)


def _cell_summary(
    cell: Mapping[str, Any],
    profile: Mapping[str, Any],
    repetitions: Sequence[Mapping[str, Any]],
    *,
    bootstrap_seed: int,
    bootstrap_resamples: int,
) -> dict[str, Any]:
    p50_by_repetition = [
        scale_02._percentile(result["steady_query_ms"], 0.50) for result in repetitions
    ]
    p99_by_repetition = [
        scale_02._percentile(result["steady_query_ms"], 0.99) for result in repetitions
    ]
    rss_by_repetition = [
        float(result["peak_rss_bytes"]) / float(result["host_memory_bytes"])
        for result in repetitions
    ]
    steady = [float(value) for result in repetitions for value in result["steady_query_ms"]]
    return {
        "id": cell["id"],
        "query_path": cell["query_path"],
        "reader_profile": profile["id"],
        "footprint_order": profile["footprint_order"],
        "repetitions": len(repetitions),
        "complete_repetitions": len(repetitions) == 5,
        "errors": sum(int(result["errors"]) for result in repetitions),
        "timeouts": sum(int(result["timeouts"]) for result in repetitions),
        "steady": scale_02._latency_summary(steady),
        "upper_95": {
            "steady_p50_ms": _bootstrap_upper(
                p50_by_repetition,
                seed=bootstrap_seed,
                resamples=bootstrap_resamples,
            ),
            "steady_p99_ms": _bootstrap_upper(
                p99_by_repetition,
                seed=bootstrap_seed + 1,
                resamples=bootstrap_resamples,
            ),
            "rss_fraction": _bootstrap_upper(
                rss_by_repetition,
                seed=bootstrap_seed + 2,
                resamples=bootstrap_resamples,
            ),
        },
        "resources": {
            "peak_rss_bytes": max(int(result["peak_rss_bytes"]) for result in repetitions),
            "max_effective_cpu_cores": max(
                float(result["effective_cpu_cores"]) for result in repetitions
            ),
        },
        "storage": {
            "database_bytes": max(int(result["database_bytes"]) for result in repetitions),
            "derived_index_bytes": max(
                int(result["derived_index_bytes"]) for result in repetitions
            ),
        },
    }


def _runtime_scale_config(config: Mapping[str, Any]) -> scale_02.Scale02Config:
    dependency = config["dependencies"]["base_config"]
    base = scale_02.load_config(_resolved_path(dependency["path"], "base config"))
    inputs = config["inputs"]
    input_root = _resolved_path(inputs["root"], "inputs.root")
    runtime = config["runtime"]
    return replace(
        base,
        corpus_root=input_root / "tc5",
        corpus_index=Path(inputs["tc5_index"]).relative_to("tc5").as_posix(),
        corpus_index_sha256=sha_file(input_root / inputs["tc5_index"]),
        qualified_manifest=input_root / inputs["tc5_qualified_manifest"],
        qualified_manifest_sha256=sha_file(input_root / inputs["tc5_qualified_manifest"]),
        python=_resolved_path(runtime["python"], "runtime.python"),
        python_extension=_resolved_path(runtime["python_extension"], "runtime.python_extension"),
        python_extension_sha256=runtime["python_extension_sha256"],
        fathomdb_bin=_resolved_path(runtime["fathomdb_bin"], "runtime.fathomdb_bin"),
        fathomdb_bin_sha256=runtime["fathomdb_bin_sha256"],
    )


def _validate_loaded_runtime(config: Mapping[str, Any]) -> None:
    expected_python = _resolved_path(config["runtime"]["python"], "runtime.python")
    if Path(sys.executable).resolve() != expected_python:
        raise Scale02FollowupError("runner is not using the pinned Python runtime")
    module = importlib.import_module("fathomdb._fathomdb")
    loaded = Path(module.__file__).resolve()
    expected_extension = _resolved_path(
        config["runtime"]["python_extension"], "runtime.python_extension"
    )
    if loaded != expected_extension or sha_file(loaded) != config["runtime"][
        "python_extension_sha256"
    ]:
        raise Scale02FollowupError("loaded FathomDB extension drifted")


def _run_performance_cells(
    config: Mapping[str, Any],
    scale_config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    run_root: Path,
) -> tuple[list[dict[str, Any]], str]:
    profiles = {item["id"]: item for item in config["treatments"]["reader_profiles"]}
    rows = scale_02.build_rows(fixture.documents, 10_000, seed=scale_config.growth_seed)
    cell_summaries = []
    for cell_index, cell in enumerate(config["treatments"]["cells"]):
        profile = profiles[cell["reader_profile"]]
        cell_root = run_root / "performance" / cell["id"]
        cell_root.mkdir(parents=True, exist_ok=False)
        repetitions = []
        observations = []
        for repetition in range(1, 6):
            witness = cell_root / f"reader-settings-rep-{repetition}.jsonl"
            with _treatment_environment(
                profile,
                rank_fast=cell["query_path"] == "rank_fast",
                reader_witness=witness,
            ):
                repetitions.append(
                    scale_02._execute_repetition(
                        scale_config,
                        fixture,
                        rows,
                        point_root=cell_root,
                        point=10_000,
                        repetition=repetition,
                    )
                )
            observations.append(
                validate_reader_observations(_jsonl(witness), profile["expected"])
            )
        summary = _cell_summary(
            cell,
            profile,
            repetitions,
            bootstrap_seed=int(config["workload"]["bootstrap_seed"], 16) + cell_index * 10,
            bootstrap_resamples=config["workload"]["bootstrap_resamples"],
        )
        summary["reader_observations"] = observations
        summary_path = cell_root / "cell-summary.v1.json"
        summary_path.write_text(
            json.dumps(summary, sort_keys=True) + "\n", encoding="utf-8"
        )
        summary_path.chmod(0o600)
        cell_summaries.append(summary)
    preliminary = select_candidate(
        cell_summaries,
        {
            "tc5": {"query_count": 100, "mismatch_count": 0},
            "answer_01": {"query_count": 32, "mismatch_count": 0},
        },
        policy=config["selection_policy"],
    )
    selected = preliminary["recommended_cell"]
    if selected is None:
        rank_cells = [item for item in cell_summaries if item["query_path"] == "rank_fast"]
        selected = min(rank_cells, key=lambda item: item["steady"]["p50"])["id"]
    return cell_summaries, selected


def _open_engine(path: Path) -> Any:
    engine_class = importlib.import_module("fathomdb").Engine
    return engine_class.open(str(path), use_default_embedder=False)


def _ingest_scale_rows(
    engine: Any, rows: Sequence[scale_02.ScaleRow], batch_size: int
) -> None:
    for offset in range(0, len(rows), batch_size):
        engine.write(
            [
                {
                    "kind": "scale_02_memory",
                    "body": row.body,
                    "source_id": row.source_id,
                    "logical_id": row.logical_id,
                }
                for row in rows[offset : offset + batch_size]
            ]
        )
    engine.drain(timeout_s=1200)


def _run_concurrency_repetition(
    scale_config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    rows: Sequence[scale_02.ScaleRow],
    *,
    root: Path,
    repetition: int,
    concurrency: int,
) -> dict[str, Any]:
    prepared = prepare_test_database(
        root,
        test_id=f"concurrency-{concurrency}-rep-{repetition}",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=str(scale_config.fathomdb_bin),
    )
    ingest = _open_engine(prepared.database_path)
    try:
        _ingest_scale_rows(ingest, rows, scale_config.write_batch_size)
    finally:
        ingest.close()
    engine = _open_engine(prepared.database_path)
    seed = int(scale_config.query_order_seed, 16) + repetition + 10_000
    warmup = scale_02._ordered_queries(fixture.queries, 100, seed + 1)
    timed = scale_02._ordered_queries(fixture.queries, 1000, seed + 2)
    for query in warmup:
        engine.search_text_only(query, limit=10)
    sampler = scale_02._ResourceSampler()
    sampler.start()
    started = time.perf_counter()

    def execute(query: str) -> tuple[float, bool]:
        query_started = time.perf_counter()
        try:
            engine.search_text_only(query, limit=10)
        except Exception:
            return 0.0, False
        return (time.perf_counter() - query_started) * 1000.0, True

    try:
        with ThreadPoolExecutor(max_workers=concurrency) as executor:
            outcomes = list(executor.map(execute, timed))
    finally:
        wall_seconds = time.perf_counter() - started
        resources = sampler.stop()
        engine.close()
    latencies = [elapsed for elapsed, success in outcomes if success]
    result = {
        "schema_version": "scale-02-concurrency-repetition.v1",
        "concurrency": concurrency,
        "repetition": repetition,
        "query_count": len(timed),
        "latency_ms": latencies,
        "errors": sum(not success for _, success in outcomes),
        "timeouts": sum(elapsed > scale_config.query_timeout_ms for elapsed in latencies),
        "wall_seconds": wall_seconds,
        "throughput_per_second": 0.0 if wall_seconds <= 0 else len(latencies) / wall_seconds,
        "resources": resources,
        "doctor_sha256": sha_file(prepared.doctor_path),
    }
    result_path = prepared.database_path.parent / "concurrency-repetition.v1.json"
    result_path.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    result_path.chmod(0o600)
    return result


def _run_concurrency_cells(
    config: Mapping[str, Any],
    scale_config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    run_root: Path,
    selected_cell: str,
    performance_cells: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    cells = {item["id"]: item for item in config["treatments"]["cells"]}
    profiles = {item["id"]: item for item in config["treatments"]["reader_profiles"]}
    selected = cells[selected_cell]
    profile = profiles[selected["reader_profile"]]
    serial = next(item for item in performance_cells if item["id"] == selected_cell)
    output: dict[str, Any] = {
        "schema_version": "scale-02-concurrency-summary.v1",
        "selected_cell": selected_cell,
        "cells": {
            "1": {
                "source": "selected_performance_cell",
                "steady": serial["steady"],
                "resources": serial["resources"],
            }
        },
    }
    rows = scale_02.build_rows(fixture.documents, 10_000, seed=scale_config.growth_seed)
    for concurrency in (2, 4):
        cell_root = run_root / "concurrency" / str(concurrency)
        cell_root.mkdir(parents=True, exist_ok=False)
        repetitions = []
        observations = []
        for repetition in range(1, 6):
            witness = cell_root / f"reader-settings-rep-{repetition}.jsonl"
            with _treatment_environment(
                profile,
                rank_fast=True,
                reader_witness=witness,
            ):
                repetitions.append(
                    _run_concurrency_repetition(
                        scale_config,
                        fixture,
                        rows,
                        root=cell_root,
                        repetition=repetition,
                        concurrency=concurrency,
                    )
                )
            observations.append(
                validate_reader_observations(_jsonl(witness), profile["expected"])
            )
        latencies = [
            float(value) for result in repetitions for value in result["latency_ms"]
        ]
        output["cells"][str(concurrency)] = {
            "source": "fresh_concurrency_cell",
            "repetitions": 5,
            "latency_ms": scale_02._latency_summary(latencies),
            "throughput_per_second": {
                "p50": scale_02._percentile(
                    [float(result["throughput_per_second"]) for result in repetitions],
                    0.50,
                ),
                "p95": scale_02._percentile(
                    [float(result["throughput_per_second"]) for result in repetitions],
                    0.95,
                ),
            },
            "errors": sum(int(result["errors"]) for result in repetitions),
            "timeouts": sum(int(result["timeouts"]) for result in repetitions),
            "max_effective_cpu_cores": max(
                float(result["resources"]["effective_cpu_cores"])
                for result in repetitions
            ),
            "reader_observations": observations,
        }
    path = run_root / "concurrency" / "summary.v1.json"
    path.write_text(json.dumps(output, sort_keys=True) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return output


def _hit_signature(result: Any) -> str:
    rows = []
    for hit in result.results[:10]:
        identifier = getattr(getattr(hit, "id", None), "value", None)
        rows.append(
            {
                "id": identifier,
                "body_sha256": hashlib.sha256(hit.body.encode("utf-8")).hexdigest(),
                "kind": hit.kind,
                "source_id": hit.source_id,
                "score_hex": float(hit.score).hex(),
            }
        )
    return hashlib.sha256(
        json.dumps(rows, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def _compare_queries(engine: Any, queries: Sequence[str], route_witness: Path) -> dict[str, Any]:
    comparisons = []
    for query in queries:
        os.environ.pop("FATHOMDB_PERF_FTS_RANK_FAST", None)
        baseline = _hit_signature(engine.search_text_only(query, limit=10))
        os.environ["FATHOMDB_PERF_FTS_RANK_FAST"] = "1"
        candidate = _hit_signature(engine.search_text_only(query, limit=10))
        comparisons.append(
            {
                "query_sha256": hashlib.sha256(query.encode("utf-8")).hexdigest(),
                "baseline_sha256": baseline,
                "candidate_sha256": candidate,
                "equal": baseline == candidate,
            }
        )
    routes = _jsonl(route_witness)
    route_counts: dict[str, int] = {}
    for row in routes:
        if set(row) != {"schema_version", "route"} or row.get("schema_version") != "scale-02-fts-route.v1":
            raise Scale02FollowupError("FTS route witness schema drifted")
        route = row.get("route")
        if not isinstance(route, str):
            raise Scale02FollowupError("FTS route witness is invalid")
        route_counts[route] = route_counts.get(route, 0) + 1
    if sum(route_counts.values()) != len(queries):
        raise Scale02FollowupError("FTS route witness count drifted")
    return {
        "query_count": len(queries),
        "mismatch_count": sum(not item["equal"] for item in comparisons),
        "route_counts": route_counts,
        "comparisons": comparisons,
    }


def _run_equivalence(
    config: Mapping[str, Any],
    scale_config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    run_root: Path,
    selected_cell: str,
) -> dict[str, Any]:
    cells = {item["id"]: item for item in config["treatments"]["cells"]}
    profiles = {item["id"]: item for item in config["treatments"]["reader_profiles"]}
    profile = profiles[cells[selected_cell]["reader_profile"]]
    root = run_root / "equivalence"
    root.mkdir(parents=True, exist_ok=False)
    output: dict[str, Any] = {"schema_version": "scale-02-equivalence.v1"}

    tc5_root = root / "tc5"
    tc5_root.mkdir()
    tc5_reader_witness = tc5_root / "reader-settings.jsonl"
    tc5_route_witness = tc5_root / "routes.jsonl"
    with _treatment_environment(
        profile,
        rank_fast=False,
        reader_witness=tc5_reader_witness,
        route_witness=tc5_route_witness,
    ):
        prepared = prepare_test_database(
            tc5_root,
            test_id="tc5-equivalence",
            embed_device="cpu",
            rerank_device="cpu",
            embedder="none",
            warm_cache=False,
            check_reranker=False,
            fathomdb_bin=str(scale_config.fathomdb_bin),
        )
        engine = _open_engine(prepared.database_path)
        try:
            _ingest_scale_rows(
                engine,
                scale_02.build_rows(
                    fixture.documents, 17_272, seed=scale_config.growth_seed
                ),
                scale_config.write_batch_size,
            )
            output["tc5"] = _compare_queries(
                engine, list(fixture.queries), tc5_route_witness
            )
        finally:
            engine.close()
    output["tc5"]["reader_observation"] = validate_reader_observations(
        _jsonl(tc5_reader_witness), profile["expected"]
    )

    input_root = _resolved_path(config["inputs"]["root"], "inputs.root")
    corpus = json.loads(
        (input_root / config["inputs"]["locomo_corpus"]).read_text(encoding="utf-8")
    )
    subset = json.loads(
        (input_root / config["inputs"]["locomo_subset"]).read_text(encoding="utf-8")
    )
    questions = answer_01.select_questions(corpus, subset)
    locomo_root = root / "answer-01"
    locomo_root.mkdir()
    locomo_reader_witness = locomo_root / "reader-settings.jsonl"
    locomo_route_witness = locomo_root / "routes.jsonl"
    with _treatment_environment(
        profile,
        rank_fast=False,
        reader_witness=locomo_reader_witness,
        route_witness=locomo_route_witness,
    ):
        prepared = prepare_test_database(
            locomo_root,
            test_id="answer-01-equivalence",
            embed_device="cpu",
            rerank_device="cpu",
            embedder="none",
            warm_cache=False,
            check_reranker=False,
            fathomdb_bin=str(scale_config.fathomdb_bin),
        )
        engine = _open_engine(prepared.database_path)
        try:
            engine.write(answer_01._ingest_rows(corpus))
            engine.drain(timeout_s=180)
            output["answer_01"] = _compare_queries(
                engine, [question.query for question in questions], locomo_route_witness
            )
        finally:
            engine.close()
    output["answer_01"]["reader_observation"] = validate_reader_observations(
        _jsonl(locomo_reader_witness), profile["expected"]
    )
    path = root / "equivalence.v1.json"
    path.write_text(json.dumps(output, sort_keys=True) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return output


def _artifact_manifest(run_root: Path) -> tuple[dict[str, Any], Path]:
    files = []
    for path in sorted(candidate for candidate in run_root.rglob("*") if candidate.is_file()):
        if path.name == "artifact-manifest.v1.json":
            continue
        relative = path.relative_to(run_root)
        files.append(
            {
                "path": relative.as_posix(),
                "bytes": path.stat().st_size,
                "sha256": sha_file(path),
            }
        )
    manifest = {
        "schema_version": "scale-02-artifact-manifest.v1",
        "program_track": PROGRAM_TRACK,
        "file_count": len(files),
        "total_bytes": sum(item["bytes"] for item in files),
        "files": files,
    }
    path = run_root / "artifact-manifest.v1.json"
    path.write_text(
        json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    path.chmod(0o600)
    return manifest, path


def _code_info(*, baseline_commit: str | None) -> dict[str, Any]:
    code = _lib.git_info()
    code["baseline_commit"] = baseline_commit
    return code


def record_input_pack(config_path: str | Path) -> dict[str, Any]:
    """Commit a safe standard receipt for the preserved external input pack."""
    config = load_config(config_path)
    inputs = config["inputs"]
    root = _resolved_path(inputs["root"], "inputs.root")
    manifest = json.loads((root / inputs["manifest"]).read_text(encoding="utf-8"))
    metrics = {
        "schema_version": "scale-02-input-pack-receipt.v1",
        "program_track": PROGRAM_TRACK,
        "pack_id": manifest["pack_id"],
        "manifest_sha256": inputs["manifest_sha256"],
        "file_count": manifest["file_count"],
        "total_bytes": manifest["total_bytes"],
        "datasets": {
            "tc5": {"document_count": 17_272, "query_count": 100},
            "answer_01": {"question_count": 32},
        },
        "persistent_root_identifier": "fathomdb-data/performance-benchmarking/scale-02",
        "payload_committed": False,
    }
    run_id, record = _lib.write_record(
        "scale-02-input-pack",
        ts=datetime.now(UTC),
        config_obj={
            "schema_version": "scale-02-input-pack-config.v1",
            "program_track": PROGRAM_TRACK,
            "manifest_sha256": inputs["manifest_sha256"],
        },
        metrics=metrics,
        verdict="complete",
        read="SCALE-02 frozen TC-5 and ANSWER-01 inputs preserved and digest-addressed",
        code=_code_info(baseline_commit=None),
        corpus={
            "source": "TC-5 and LOCOMO frozen external input pack",
            "manifest_sha256": inputs["manifest_sha256"],
            "datasets": ["tc5-qualified-real-v2", "locomo10-answer-01-subset-v3"],
        },
        seeds={},
        env=_lib.env_info(key_deps={"fathomdb": "0.8.23"}),
        cost_usd=0.0,
        headline={"file_count": manifest["file_count"], "program_track": PROGRAM_TRACK},
        n=manifest["file_count"],
        config_path=str(config_path),
        tests=["tests/experiments/test_scale_02_followup.py"],
        files_changed=[],
        artifacts=[{"kind": "external_input_manifest", "sha256": inputs["manifest_sha256"]}],
        review=None,
        open_questions=[],
    )
    _lib.regen_index_md()
    return {"run_id": run_id, "record_sha256": sha_file(record)}


def _safe_equivalence(equivalence: Mapping[str, Any]) -> dict[str, Any]:
    return {
        name: {
            "query_count": equivalence[name]["query_count"],
            "mismatch_count": equivalence[name]["mismatch_count"],
            "route_counts": equivalence[name]["route_counts"],
        }
        for name in ("tc5", "answer_01")
    }


def run_tuning(config_path: str | Path) -> dict[str, Any]:
    """Execute all registered fresh-database tuning and equivalence cells."""
    config = load_config(config_path)
    _validate_loaded_runtime(config)
    scale_config = _runtime_scale_config(config)
    scale_02._validate_runtime(scale_config)
    fixture = scale_02.load_fixture(scale_config)
    config_sha = _lib.config_sha256(config)
    timestamp = datetime.now(UTC)
    run_id = _lib.make_run_id("scale-02-fts-tuning", timestamp, config_sha)
    artifact_base = _resolved_path(config["artifact_root"], "artifact_root")
    run_root = artifact_base / run_id
    if run_root.exists():
        raise Scale02FollowupError("tuning artifact root already exists")
    run_root.mkdir(parents=True, mode=0o700)
    cells, selected_cell = _run_performance_cells(
        config, scale_config, fixture, run_root
    )
    concurrency = _run_concurrency_cells(
        config, scale_config, fixture, run_root, selected_cell, cells
    )
    equivalence = _run_equivalence(
        config, scale_config, fixture, run_root, selected_cell
    )
    manifest, manifest_path = _artifact_manifest(run_root)
    safe_equivalence = _safe_equivalence(equivalence)
    metrics = {
        "schema_version": TUNING_SCHEMA,
        "program_track": PROGRAM_TRACK,
        "status": "complete",
        "input_manifest_sha256": config["inputs"]["manifest_sha256"],
        "runtime": {
            "source_git_sha": config["runtime"]["source_git_sha"],
            "python_extension_sha256": config["runtime"]["python_extension_sha256"],
            "fathomdb_bin_sha256": config["runtime"]["fathomdb_bin_sha256"],
            "wheel_sha256": config["runtime"]["wheel_sha256"],
        },
        "performance_cells": cells,
        "concurrency": concurrency,
        "equivalence": safe_equivalence,
        "preliminary_reader_cell": selected_cell,
        "artifact_manifest": {
            "sha256": sha_file(manifest_path),
            "file_count": manifest["file_count"],
            "total_bytes": manifest["total_bytes"],
        },
        "formal_rerun_authorized": False,
    }
    receipt_run_id, record = _lib.write_record(
        "scale-02-fts-tuning",
        ts=timestamp,
        config_obj=config,
        metrics=metrics,
        verdict="complete",
        read="SCALE-02 compact FTS tuning and retrieval-equivalence evidence complete",
        code=_code_info(baseline_commit=config["runtime"]["source_git_sha"]),
        corpus={
            "source": "frozen SCALE-02 input pack v1",
            "manifest_sha256": config["inputs"]["manifest_sha256"],
            "datasets": ["tc5-qualified-real-v2", "locomo10-answer-01-subset-v3"],
        },
        seeds={
            "query_order": config["workload"]["query_order_seed"],
            "bootstrap": config["workload"]["bootstrap_seed"],
        },
        env=_lib.env_info(
            key_deps={
                "fathomdb": "0.8.23",
                "sqlite": next(
                    item["reader_observations"][0]["sqlite_version"] for item in cells
                ),
            }
        ),
        cost_usd=0.0,
        headline={
            "program_track": PROGRAM_TRACK,
            "cells": len(cells),
            "equivalence_mismatches": sum(
                item["mismatch_count"] for item in safe_equivalence.values()
            ),
        },
        n=10_000,
        config_path=str(config_path),
        tests=[
            "tests/experiments/test_scale_02_followup.py",
            "src/rust/crates/fathomdb-engine/tests/scale02_fts_rank_fast.rs",
        ],
        files_changed=[],
        artifacts=[
            {"kind": "external_artifact_manifest", "sha256": sha_file(manifest_path)}
        ],
        review=None,
        open_questions=["HITL selection approval is required before formal 10k rerun"],
    )
    if receipt_run_id != run_id:
        raise Scale02FollowupError("tuning run ID drifted")
    _lib.regen_index_md()
    return {
        "run_id": run_id,
        "record_path": str(record),
        "record_sha256": sha_file(record),
        "artifact_manifest_sha256": sha_file(manifest_path),
    }


def synthesize_selection(
    config_path: str | Path, tuning_record_path: str | Path
) -> dict[str, Any]:
    """Combine tuning, ANSWER-01, and SCALE-01 evidence into a pending proposal."""
    config = load_config(config_path)
    tuning_path = Path(tuning_record_path)
    tuning = json.loads(tuning_path.read_text(encoding="utf-8"))
    metrics = tuning.get("metrics", {})
    if (
        tuning.get("verdict") != "complete"
        or metrics.get("schema_version") != TUNING_SCHEMA
        or metrics.get("input_manifest_sha256") != config["inputs"]["manifest_sha256"]
    ):
        raise Scale02FollowupError("tuning receipt is incomplete or mismatched")
    equivalence = metrics["equivalence"]
    selection = select_candidate(
        metrics["performance_cells"],
        equivalence,
        policy=config["selection_policy"],
    )
    recommended = selection["recommended_cell"]
    cells = {item["id"]: item for item in config["treatments"]["cells"]}
    profiles = {item["id"]: item for item in config["treatments"]["reader_profiles"]}
    selected_profile = profiles[cells[recommended]["reader_profile"]] if recommended else None
    answer_record = json.loads(
        _resolved_path(
            config["dependencies"]["answer_01"]["path"], "answer receipt"
        ).read_text(encoding="utf-8")
    )
    scale_record = json.loads(
        _resolved_path(
            config["dependencies"]["scale_01"]["path"], "scale receipt"
        ).read_text(encoding="utf-8")
    )
    baseline_record = json.loads(
        _resolved_path(
            config["dependencies"]["baseline"]["path"], "baseline receipt"
        ).read_text(encoding="utf-8")
    )
    proposal = {
        "schema_version": "scale-02-execution.v2",
        "program_track": PROGRAM_TRACK,
        "release": "0.8.23",
        "approval": {"state": "pending_hitl", "approved_by": None, "approved_at": None},
        "profile": {
            "id": "a0_turn_fts",
            "top_k": 10,
            "query_path": "rank_fast_pending_production_landing" if recommended else None,
            "reader_profile": selected_profile["id"] if selected_profile else None,
            "reader_pragmas": selected_profile["pragmas"] if selected_profile else None,
            "expected_reader_settings": selected_profile["expected"] if selected_profile else None,
        },
        "inputs": {
            "manifest_sha256": config["inputs"]["manifest_sha256"],
            "point": 10_000,
        },
        "workload": dict(config["workload"]),
        "policy": dict(config["selection_policy"]),
        "candidate_runtime": dict(config["runtime"]),
        "tuning_receipt": {
            "run_id": tuning["run_id"],
            "sha256": sha_file(tuning_path),
        },
        "claim_boundary": "formal_10k_rerun_pending_hitl_and_production_landing",
    }
    proposal_path = REPO_ROOT / "experiments/configs/scale-02/a0-envelope.v2.proposed.json"
    proposal_path.write_text(json.dumps(proposal, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    selection_metrics = {
        **selection,
        "program_track": PROGRAM_TRACK,
        "basis": {
            "baseline": {
                "run_id": baseline_record["run_id"],
                "sha256": config["dependencies"]["baseline"]["sha256"],
                "steady_p50_ms": baseline_record["metrics"]["cache_states"]["steady"][
                    "latency_ms"
                ]["p50"],
            },
            "answer_01": {
                "run_id": answer_record["run_id"],
                "sha256": config["dependencies"]["answer_01"]["sha256"],
                "decision": answer_record["metrics"]["decision"],
                "a0": answer_record["metrics"]["arms"]["a0_turn_fts"],
            },
            "scale_01": {
                "run_id": scale_record["run_id"],
                "sha256": config["dependencies"]["scale_01"]["sha256"],
                "recall_at_10": scale_record["metrics"]["metrics"]["recall_at_10"],
                "ci_95": scale_record["metrics"]["metrics"]["ci_95"],
            },
            "tuning": {"run_id": tuning["run_id"], "sha256": sha_file(tuning_path)},
        },
        "proposed_config": {
            "path": "experiments/configs/scale-02/a0-envelope.v2.proposed.json",
            "sha256": sha_file(proposal_path),
        },
        "formal_rerun_authorized": False,
    }
    resolved = {
        "schema_version": "scale-02-fts-selection-config.v1",
        "program_track": PROGRAM_TRACK,
        "tuning_run_id": tuning["run_id"],
        "tuning_sha256": sha_file(tuning_path),
        "proposal_sha256": sha_file(proposal_path),
    }
    run_id, record = _lib.write_record(
        "scale-02-fts-selection",
        ts=datetime.now(UTC),
        config_obj=resolved,
        metrics=selection_metrics,
        verdict="recommendation_pending_hitl",
        read=(
            f"SCALE-02 recommends {recommended}; formal 10k rerun awaits HITL"
            if recommended
            else "SCALE-02 has no eligible FTS candidate; formal rerun remains blocked"
        ),
        code=_code_info(baseline_commit=config["runtime"]["source_git_sha"]),
        corpus={
            "source": "frozen SCALE-02 input pack v1",
            "manifest_sha256": config["inputs"]["manifest_sha256"],
            "datasets": ["tc5-qualified-real-v2", "locomo10-answer-01-subset-v3"],
        },
        seeds={"bootstrap": config["workload"]["bootstrap_seed"]},
        env=_lib.env_info(key_deps={"fathomdb": "0.8.23"}),
        cost_usd=0.0,
        headline={
            "recommended_cell": recommended,
            "quality_applicability": selection["quality_applicability"],
        },
        n=10_000,
        config_path=str(proposal_path.relative_to(REPO_ROOT)),
        tests=["tests/experiments/test_scale_02_followup.py"],
        files_changed=[str(proposal_path.relative_to(REPO_ROOT))],
        artifacts=[{"kind": "tuning_receipt", "sha256": sha_file(tuning_path)}],
        review=None,
        open_questions=["approve or reject the proposed formal SCALE-02 10k configuration"],
    )
    _lib.regen_index_md()
    return {
        "run_id": run_id,
        "record_path": str(record),
        "record_sha256": sha_file(record),
        "recommended_cell": recommended,
        "proposal_path": str(proposal_path),
        "proposal_sha256": sha_file(proposal_path),
        "state": "recommendation_pending_hitl",
    }


def main(argv: list[str] | None = None) -> int:
    """Freeze, validate, execute, or synthesize the registered follow-up."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    freeze = subparsers.add_parser("freeze-inputs")
    freeze.add_argument("--tc5-root", required=True, type=Path)
    freeze.add_argument("--tc5-qualified-manifest", required=True, type=Path)
    freeze.add_argument("--locomo-corpus", required=True, type=Path)
    freeze.add_argument("--locomo-subset", required=True, type=Path)
    freeze.add_argument("--output-root", required=True, type=Path)
    validate = subparsers.add_parser("validate")
    validate.add_argument("config", type=Path)
    record_inputs = subparsers.add_parser("record-inputs")
    record_inputs.add_argument("config", type=Path)
    run = subparsers.add_parser("run-tuning")
    run.add_argument("config", type=Path)
    select = subparsers.add_parser("synthesize-selection")
    select.add_argument("config", type=Path)
    select.add_argument("tuning_record", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "freeze-inputs":
            result = freeze_inputs(
                tc5_root=args.tc5_root,
                tc5_qualified_manifest=args.tc5_qualified_manifest,
                locomo_corpus=args.locomo_corpus,
                locomo_subset=args.locomo_subset,
                output_root=args.output_root,
            )
        elif args.command == "validate":
            load_config(args.config)
            result = {"state": "valid"}
        elif args.command == "record-inputs":
            result = record_input_pack(args.config)
        elif args.command == "run-tuning":
            result = run_tuning(args.config)
        else:
            result = synthesize_selection(args.config, args.tuning_record)
    except (OSError, Scale02FollowupError, json.JSONDecodeError) as exc:
        parser.error(str(exc))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
