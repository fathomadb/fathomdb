#!/usr/bin/env python3
"""Run the approved SCALE-02 rank-boundary two-factor off-shoot."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import random
import shutil
import sys
from contextlib import contextmanager
from dataclasses import replace
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Iterator, Mapping, Sequence

from experiments import _lib, scale_02, scale_02_followup


REPO_ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "scale-02-rank-boundary.v1"
RESULT_SCHEMA = "scale-02-rank-boundary-result.v1"
POINTS = (25_000, 40_000, 50_000)
CELLS = (
    ("control_default", "shipped_full_sort_fallback", "default"),
    ("stream_default", "stream_complete_boundary_tie", "default"),
    ("control_mmap128", "shipped_full_sort_fallback", "mmap128"),
    ("stream_mmap128", "stream_complete_boundary_tie", "mmap128"),
)
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "authorization",
    "dependencies",
    "hypothesis",
    "points",
    "factors",
    "reader_profiles",
    "matrix",
    "workload",
    "qualification",
    "policy",
    "runtime",
    "artifact_root",
    "claim_boundary",
}


class Scale02RankBoundaryError(RuntimeError):
    """Raised when the rank-boundary contract or evidence drifts."""


def _exact(value: object, expected: set[str], label: str) -> Mapping[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        raise Scale02RankBoundaryError(f"{label} keys drifted")
    return value


def _path(value: object, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise Scale02RankBoundaryError(f"{label} must be a path")
    path = Path(value)
    return path if path.is_absolute() else REPO_ROOT / path


def _digest(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != 64
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Scale02RankBoundaryError(f"{label} must be a lowercase sha256")
    return value


def load_config(path: str | Path) -> dict[str, Any]:
    """Load and strictly validate the approved rank-boundary configuration."""
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    _exact(document, _TOP_KEYS, "configuration")
    if (
        document["schema_version"] != SCHEMA
        or document["program_track"] != "SCALE-02"
        or document["release"] != "0.8.23"
        or document["points"] != list(POINTS)
        or document["claim_boundary"]
        != "experiment_only_pending_post_result_production_decision"
    ):
        raise Scale02RankBoundaryError("configuration identity drifted")
    authorization = _exact(
        document["authorization"],
        {"state", "approved_by", "approved_at", "ledger_ref"},
        "authorization",
    )
    if (
        authorization["state"] != "approved"
        or authorization["approved_by"] != "HITL"
        or authorization["ledger_ref"] != "seq-266"
    ):
        raise Scale02RankBoundaryError("execution is not authorized")
    if (
        not isinstance(authorization["approved_at"], str)
        or not authorization["approved_at"]
    ):
        raise Scale02RankBoundaryError("approval timestamp is missing")

    for dependency in document["dependencies"].values():
        _exact(dependency, {"path", "sha256"}, "dependency")
        target = _path(dependency["path"], "dependency.path")
        if not target.is_file() or scale_02_followup.sha_file(target) != _digest(
            dependency["sha256"], "dependency.sha256"
        ):
            raise Scale02RankBoundaryError("dependency digest drifted")

    factors = document["factors"]
    if factors != [
        {
            "id": "boundary_handling",
            "levels": ["shipped_full_sort_fallback", "stream_complete_boundary_tie"],
        },
        {"id": "reader_profile", "levels": ["default", "mmap128"]},
    ]:
        raise Scale02RankBoundaryError("factor matrix drifted")
    profiles = document["reader_profiles"]
    _exact(profiles, {"default", "mmap128"}, "reader_profiles")
    for name, profile in profiles.items():
        _exact(profile, {"pragmas", "expected"}, f"reader_profiles.{name}")
        _exact(
            profile["expected"],
            {"cache_size", "mmap_size", "temp_store"},
            f"reader_profiles.{name}.expected",
        )
    if document["matrix"] != {
        "design": "full_factorial",
        "cells_per_point": 4,
        "repetitions_per_cell": 5,
        "fresh_databases": 60,
        "balanced_execution_order": True,
    }:
        raise Scale02RankBoundaryError("execution matrix drifted")
    workload = document["workload"]
    if workload != {
        "cold_queries": 100,
        "warmup_queries": 100,
        "steady_queries": 1000,
        "mutations": 20,
        "equivalence_queries_per_cell": 100,
        "query_order_seed": "0x5CA1E0200A0F75",
        "treatment_order_seed": "0x5CA1E02071E5",
        "bootstrap_seed": "0x5CA1E020B00757",
        "bootstrap_resamples": 2000,
        "concurrency": 1,
    }:
        raise Scale02RankBoundaryError("workload drifted")

    qualification = _exact(
        document["qualification"],
        {
            "control_database_root",
            "require_no_temp_order_by_for_stream_statement",
            "reproduce_control_fallback_counts",
            "record_writer_and_reader_journal_mode",
            "record_writer_and_reader_synchronous",
            "stop_on_durability_adr_mismatch",
            "require_exact_ordered_top100_candidates",
            "require_exact_ordered_top10_public_results",
            "require_zero_stream_full_sort_fallbacks",
            "record_rows_consumed_and_boundary_group_size",
        },
        "qualification",
    )
    control_root = _path(
        qualification["control_database_root"],
        "qualification.control_database_root",
    )
    if not control_root.is_dir() or qualification[
        "reproduce_control_fallback_counts"
    ] != {
        "25000": 20,
        "40000": 41,
        "50000": 58,
    }:
        raise Scale02RankBoundaryError("qualification inputs drifted")

    runtime = _exact(
        document["runtime"],
        {
            "base_source_git_sha",
            "candidate_source_git_sha",
            "python",
            "python_extension",
            "python_extension_sha256",
            "fathomdb_bin",
            "fathomdb_bin_sha256",
            "sqlite_version",
        },
        "runtime",
    )
    for key in ("base_source_git_sha", "candidate_source_git_sha"):
        if not isinstance(runtime[key], str) or len(runtime[key]) != 40:
            raise Scale02RankBoundaryError(f"runtime.{key} drifted")
    for path_key, digest_key in (
        ("python_extension", "python_extension_sha256"),
        ("fathomdb_bin", "fathomdb_bin_sha256"),
    ):
        target = _path(runtime[path_key], f"runtime.{path_key}")
        if not target.is_file() or scale_02_followup.sha_file(target) != _digest(
            runtime[digest_key], f"runtime.{digest_key}"
        ):
            raise Scale02RankBoundaryError(f"runtime {path_key} drifted")
    if not _path(runtime["python"], "runtime.python").is_file():
        raise Scale02RankBoundaryError("runtime Python is unavailable")
    artifact_root = _path(document["artifact_root"], "artifact_root")
    if artifact_root.resolve().is_relative_to(REPO_ROOT.resolve()):
        raise Scale02RankBoundaryError(
            "artifact root must remain outside the repository"
        )
    return document


def summarize_boundary_witnesses(
    routes: Sequence[Mapping[str, Any]],
    boundaries: Sequence[Mapping[str, Any]],
    *,
    expected_queries: int,
) -> dict[str, Any]:
    """Validate and summarize content-free streamed-boundary witnesses."""
    if len(routes) != expected_queries or len(boundaries) != expected_queries:
        raise Scale02RankBoundaryError("boundary witness count drifted")
    route_counts: dict[str, int] = {}
    for row in routes:
        if (
            set(row) != {"schema_version", "route"}
            or row.get("schema_version") != "scale-02-fts-route.v1"
        ):
            raise Scale02RankBoundaryError("route witness schema drifted")
        route = row.get("route")
        if not isinstance(route, str):
            raise Scale02RankBoundaryError("route witness value drifted")
        route_counts[route] = route_counts.get(route, 0) + 1
    consumed = []
    group_sizes = []
    for row in boundaries:
        if (
            set(row)
            != {
                "schema_version",
                "route",
                "candidate_limit",
                "rows_consumed",
                "boundary_group_size",
            }
            or row.get("schema_version") != "scale-02-fts-boundary.v1"
        ):
            raise Scale02RankBoundaryError("boundary witness schema drifted")
        if row["candidate_limit"] != 100:
            raise Scale02RankBoundaryError("candidate limit drifted")
        consumed.append(int(row["rows_consumed"]))
        group_sizes.append(int(row["boundary_group_size"]))
    allowed = {"rank_stream_strict_boundary", "rank_stream_tie_completed"}
    fallback_count = sum(
        count for route, count in route_counts.items() if route not in allowed
    )
    consumed_summary = scale_02._latency_summary([float(value) for value in consumed])
    consumed_summary["max"] = max(consumed)
    group_summary = scale_02._latency_summary([float(value) for value in group_sizes])
    group_summary["max"] = max(group_sizes)
    return {
        "route_counts": route_counts,
        "full_sort_fallbacks": fallback_count,
        "rows_consumed": consumed_summary,
        "boundary_group_size": group_summary,
    }


def validate_connection_witnesses(
    rows: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    """Require WAL on all observed connections and NORMAL on every writer."""
    expected = {
        "schema_version",
        "role",
        "journal_mode",
        "synchronous",
        "sqlite_version",
    }
    if not rows:
        raise Scale02RankBoundaryError("connection witness is empty")
    role_counts: dict[str, int] = {}
    versions = set()
    reader_synchronous = set()
    for row in rows:
        if (
            set(row) != expected
            or row.get("schema_version") != "scale-02-connection-settings.v1"
        ):
            raise Scale02RankBoundaryError("connection witness schema drifted")
        role = row.get("role")
        if role not in {"writer", "reader"} or row.get("journal_mode") != "wal":
            raise Scale02RankBoundaryError("connection WAL profile drifted")
        synchronous = int(row["synchronous"])
        if role == "writer" and synchronous != 1:
            raise Scale02RankBoundaryError("writer synchronous mode is not NORMAL")
        if role == "reader":
            reader_synchronous.add(synchronous)
        role_counts[role] = role_counts.get(role, 0) + 1
        versions.add(str(row["sqlite_version"]))
    if "writer" not in role_counts or "reader" not in role_counts or len(versions) != 1:
        raise Scale02RankBoundaryError("connection role or SQLite version drifted")
    return {
        "connection_count": len(rows),
        "role_counts": dict(sorted(role_counts.items())),
        "reader_synchronous_values": sorted(reader_synchronous),
        "sqlite_version": next(iter(versions)),
    }


def validate_query_plan_witness(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Require the pinned engine's streamed statement to avoid a temporary sort."""
    if len(rows) != 1:
        raise Scale02RankBoundaryError("query-plan witness count drifted")
    row = dict(rows[0])
    if row != {
        "schema_version": "scale-02-fts-query-plan.v1",
        "statement": "stream_complete_boundary_tie",
        "uses_temp_btree_for_order_by": False,
    }:
        if row.get("uses_temp_btree_for_order_by") is True:
            raise Scale02RankBoundaryError(
                "stream statement requires a temporary ORDER BY sort"
            )
        raise Scale02RankBoundaryError("query-plan witness drifted")
    return row


def evaluate_stream_cell(
    cell: Mapping[str, Any], policy: Mapping[str, Any]
) -> dict[str, Any]:
    """Apply the registered accuracy, mechanism, latency, and resource policy."""
    reasons = []
    point = str(cell["point"])
    if not cell["complete_repetitions"]:
        reasons.append("incomplete_repetitions")
    if cell["errors"] > policy["max_errors"]:
        reasons.append("errors")
    if cell["timeouts"] > policy["max_timeouts"]:
        reasons.append("timeouts")
    if cell["steady"]["p50"] > policy["steady_p50_ms_by_point"][point]:
        reasons.append("steady_p50")
    if cell["steady"]["p99"] > policy["steady_p99_ms"]:
        reasons.append("steady_p99")
    if cell["upper_95"]["rss_fraction"] > policy["max_rss_fraction"]:
        reasons.append("rss_fraction")
    equivalence = cell["equivalence"]
    if equivalence["query_count"] != 100:
        reasons.append("equivalence_count")
    if equivalence["top100_mismatch_count"] != 0:
        reasons.append("top100_equivalence")
    if equivalence["top10_mismatch_count"] != 0:
        reasons.append("top10_equivalence")
    if equivalence["full_sort_fallbacks"] != 0:
        reasons.append("full_sort_fallback")
    return {"eligible": not reasons, "reasons": reasons}


def select_candidate(cells: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Select the lowest-footprint stream treatment passing every point."""
    eligible_through: dict[str, int | None] = {}
    for name, reader in (("stream_default", "default"), ("stream_mmap128", "mmap128")):
        points = sorted(
            int(cell["point"])
            for cell in cells
            if cell["boundary_handling"] == "stream_complete_boundary_tie"
            and cell["reader_profile"] == reader
            and cell["decision"]["eligible"]
        )
        eligible_through[name] = max(points) if points else None
    recommended = next(
        (
            name
            for name in ("stream_default", "stream_mmap128")
            if eligible_through[name] == 50_000
        ),
        None,
    )
    return {
        "state": "result_pending_hitl",
        "recommended_cell": recommended,
        "eligible_through": eligible_through,
    }


@contextmanager
def _environment(
    profile: Mapping[str, Any],
    *,
    stream: bool,
    reader_witness: Path,
    connection_witness: Path,
    route_witness: Path | None = None,
    boundary_witness: Path | None = None,
    query_plan_witness: Path | None = None,
) -> Iterator[None]:
    keys = (
        "FATHOMDB_PERF_FTS_STREAM_TIES",
        "FATHOMDB_PERF_FTS_BOUNDARY_WITNESS",
        "FATHOMDB_PERF_CONNECTION_PRAGMA_WITNESS",
        "FATHOMDB_PERF_FTS_QUERY_PLAN_WITNESS",
        "FATHOMDB_PERF_FTS_FORCE_FULL_SORT",
    )
    prior = {key: os.environ.get(key) for key in keys}
    if stream:
        os.environ["FATHOMDB_PERF_FTS_STREAM_TIES"] = "1"
    else:
        os.environ.pop("FATHOMDB_PERF_FTS_STREAM_TIES", None)
    if boundary_witness is not None:
        os.environ["FATHOMDB_PERF_FTS_BOUNDARY_WITNESS"] = str(boundary_witness)
    else:
        os.environ.pop("FATHOMDB_PERF_FTS_BOUNDARY_WITNESS", None)
    os.environ["FATHOMDB_PERF_CONNECTION_PRAGMA_WITNESS"] = str(connection_witness)
    if query_plan_witness is not None:
        os.environ["FATHOMDB_PERF_FTS_QUERY_PLAN_WITNESS"] = str(query_plan_witness)
    else:
        os.environ.pop("FATHOMDB_PERF_FTS_QUERY_PLAN_WITNESS", None)
    os.environ.pop("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None)
    try:
        with scale_02_followup._treatment_environment(
            profile,
            rank_fast=True,
            reader_witness=reader_witness,
            route_witness=route_witness,
        ):
            yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def _validate_runtime(
    document: Mapping[str, Any], config: scale_02.Scale02Config
) -> None:
    runtime = document["runtime"]
    if (
        Path(sys.executable).resolve()
        != _path(runtime["python"], "runtime.python").resolve()
    ):
        raise Scale02RankBoundaryError("runner is not using the pinned Python")
    module = importlib.import_module("fathomdb._fathomdb")
    loaded = Path(module.__file__).resolve()
    expected = _path(runtime["python_extension"], "runtime.python_extension").resolve()
    if (
        loaded != expected
        or scale_02_followup.sha_file(loaded) != runtime["python_extension_sha256"]
    ):
        raise Scale02RankBoundaryError("loaded Python extension drifted")
    scale_02._validate_runtime(config)


def _signature(result: Any, limit: int) -> str:
    rows = []
    for hit in result.results[:limit]:
        identifier = getattr(hit, "id", None)
        rows.append(
            {
                "id": str(identifier),
                "write_cursor": getattr(hit, "write_cursor", None),
                "body_sha256": hashlib.sha256(hit.body.encode("utf-8")).hexdigest(),
                "kind": hit.kind,
                "source_id": hit.source_id,
                "score_hex": float(hit.score).hex(),
            }
        )
    return hashlib.sha256(
        json.dumps(rows, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def _compare_queries(
    engine: Any,
    queries: Sequence[str],
    route_witness: Path,
    boundary_witness: Path,
) -> dict[str, Any]:
    comparisons = []
    for query in queries:
        try:
            os.environ["FATHOMDB_PERF_FTS_FORCE_FULL_SORT"] = "1"
            baseline_100 = engine.search_text_only(query, limit=100)
            baseline_10 = engine.search_text_only(query, limit=10)
        finally:
            os.environ.pop("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None)
        candidate_100 = engine.search_text_only(query, limit=100)
        candidate_10 = engine.search_text_only(query, limit=10)
        comparisons.append(
            {
                "query_sha256": hashlib.sha256(query.encode("utf-8")).hexdigest(),
                "baseline_top100_sha256": _signature(baseline_100, 100),
                "candidate_top100_sha256": _signature(candidate_100, 100),
                "baseline_top10_sha256": _signature(baseline_10, 10),
                "candidate_top10_sha256": _signature(candidate_10, 10),
            }
        )
    boundary = summarize_boundary_witnesses(
        scale_02_followup._jsonl(route_witness),
        scale_02_followup._jsonl(boundary_witness),
        expected_queries=len(queries) * 2,
    )
    return {
        "query_count": len(comparisons),
        "top100_mismatch_count": sum(
            row["baseline_top100_sha256"] != row["candidate_top100_sha256"]
            for row in comparisons
        ),
        "top10_mismatch_count": sum(
            row["baseline_top10_sha256"] != row["candidate_top10_sha256"]
            for row in comparisons
        ),
        **boundary,
        "comparisons": comparisons,
    }


def _run_preflight(
    document: Mapping[str, Any],
    fixture: scale_02.Fixture,
    run_root: Path,
) -> dict[str, Any]:
    qualification = document["qualification"]
    source_root = _path(
        qualification["control_database_root"],
        "qualification.control_database_root",
    )
    root = run_root / "qualification"
    root.mkdir(parents=True, exist_ok=False)
    queries = list(fixture.queries)[
        : document["workload"]["equivalence_queries_per_cell"]
    ]
    points = {}
    for point in POINTS:
        point_root = root / str(point)
        point_root.mkdir()
        source = (
            source_root / f"point-{point}" / f"point-{point}-rep-1" / "fathomdb.sqlite"
        )
        database = point_root / "fathomdb.sqlite"
        if not source.is_file():
            raise Scale02RankBoundaryError(f"qualification database missing at {point}")
        shutil.copy2(source, database)
        profile = document["reader_profiles"]["default"]

        control_routes = point_root / "control-routes.jsonl"
        control_reader = point_root / "control-reader.jsonl"
        control_connections = point_root / "control-connections.jsonl"
        mismatches = 0
        with _environment(
            profile,
            stream=False,
            reader_witness=control_reader,
            connection_witness=control_connections,
            route_witness=control_routes,
        ):
            engine = scale_02_followup._open_engine(database)
            try:
                for query in queries:
                    try:
                        os.environ["FATHOMDB_PERF_FTS_FORCE_FULL_SORT"] = "1"
                        baseline = _signature(
                            engine.search_text_only(query, limit=10), 10
                        )
                    finally:
                        os.environ.pop("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None)
                    candidate = _signature(engine.search_text_only(query, limit=10), 10)
                    mismatches += baseline != candidate
            finally:
                engine.close()
        route_counts: dict[str, int] = {}
        for row in scale_02_followup._jsonl(control_routes):
            route = str(row["route"])
            route_counts[route] = route_counts.get(route, 0) + 1
        expected_fallbacks = qualification["reproduce_control_fallback_counts"][
            str(point)
        ]
        if (
            len(scale_02_followup._jsonl(control_routes)) != len(queries)
            or route_counts.get("full_sort_fallback", 0) != expected_fallbacks
            or mismatches != 0
        ):
            raise Scale02RankBoundaryError(f"control qualification drifted at {point}")

        stream_routes = point_root / "stream-routes.jsonl"
        stream_boundaries = point_root / "stream-boundaries.jsonl"
        stream_reader = point_root / "stream-reader.jsonl"
        stream_connections = point_root / "stream-connections.jsonl"
        query_plan = point_root / "stream-query-plan.jsonl"
        with _environment(
            profile,
            stream=True,
            reader_witness=stream_reader,
            connection_witness=stream_connections,
            route_witness=stream_routes,
            boundary_witness=stream_boundaries,
            query_plan_witness=query_plan,
        ):
            engine = scale_02_followup._open_engine(database)
            try:
                stream = _compare_queries(
                    engine, queries, stream_routes, stream_boundaries
                )
            finally:
                engine.close()
        if stream["top100_mismatch_count"] or stream["top10_mismatch_count"]:
            raise Scale02RankBoundaryError(f"stream equivalence drifted at {point}")
        points[str(point)] = {
            "control_route_counts": route_counts,
            "control_equivalence_mismatches": mismatches,
            "stream": {
                key: value for key, value in stream.items() if key != "comparisons"
            },
            "query_plan": validate_query_plan_witness(
                scale_02_followup._jsonl(query_plan)
            ),
            "control_connections": validate_connection_witnesses(
                scale_02_followup._jsonl(control_connections)
            ),
            "control_reader": scale_02_followup.validate_reader_observations(
                scale_02_followup._jsonl(control_reader), profile["expected"]
            ),
            "stream_connections": validate_connection_witnesses(
                scale_02_followup._jsonl(stream_connections)
            ),
            "stream_reader": scale_02_followup.validate_reader_observations(
                scale_02_followup._jsonl(stream_reader), profile["expected"]
            ),
        }
    result = {"schema_version": "scale-02-rank-boundary-preflight.v1", "points": points}
    path = root / "preflight.v1.json"
    path.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return result


def _run_equivalence(
    document: Mapping[str, Any],
    config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    run_root: Path,
    point: int,
    reader: str,
) -> dict[str, Any]:
    root = run_root / "equivalence" / reader / str(point)
    root.mkdir(parents=True, exist_ok=False)
    reader_witness = root / "reader-settings.jsonl"
    connection_witness = root / "connection-settings.jsonl"
    route_witness = root / "routes.jsonl"
    boundary_witness = root / "boundaries.jsonl"
    database = (
        run_root
        / "performance"
        / f"control_{reader}"
        / str(point)
        / f"point-{point}-rep-1"
        / "fathomdb.sqlite"
    )
    profile = document["reader_profiles"][reader]
    with _environment(
        profile,
        stream=True,
        reader_witness=reader_witness,
        connection_witness=connection_witness,
        route_witness=route_witness,
        boundary_witness=boundary_witness,
    ):
        engine = scale_02_followup._open_engine(database)
        try:
            result = _compare_queries(
                engine,
                list(fixture.queries)[
                    : document["workload"]["equivalence_queries_per_cell"]
                ],
                route_witness,
                boundary_witness,
            )
        finally:
            engine.close()
    result["reader_observation"] = scale_02_followup.validate_reader_observations(
        scale_02_followup._jsonl(reader_witness), profile["expected"]
    )
    result["connection_observation"] = validate_connection_witnesses(
        scale_02_followup._jsonl(connection_witness)
    )
    path = root / "equivalence.v1.json"
    path.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return result


def _run_performance(
    document: Mapping[str, Any],
    config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    run_root: Path,
) -> list[dict[str, Any]]:
    repetitions: dict[tuple[str, int], list[dict[str, Any]]] = {
        (cell_id, point): [] for cell_id, _, _ in CELLS for point in POINTS
    }
    observations: dict[tuple[str, int], list[dict[str, Any]]] = {
        key: [] for key in repetitions
    }
    connection_observations: dict[tuple[str, int], list[dict[str, Any]]] = {
        key: [] for key in repetitions
    }
    roots: dict[tuple[str, int], Path] = {}
    rows_by_point = {
        point: scale_02.build_rows(fixture.documents, point, seed=config.growth_seed)
        for point in POINTS
    }
    for cell_id, _, _ in CELLS:
        for point in POINTS:
            root = run_root / "performance" / cell_id / str(point)
            root.mkdir(parents=True, exist_ok=False)
            roots[(cell_id, point)] = root

    randomizer = random.Random(int(document["workload"]["treatment_order_seed"], 16))
    for point in POINTS:
        for repetition in range(1, 6):
            order = list(CELLS)
            randomizer.shuffle(order)
            for cell_id, boundary, reader in order:
                root = roots[(cell_id, point)]
                witness = root / f"reader-settings-rep-{repetition}.jsonl"
                connection_witness = (
                    root / f"connection-settings-rep-{repetition}.jsonl"
                )
                profile = document["reader_profiles"][reader]
                with _environment(
                    profile,
                    stream=boundary == "stream_complete_boundary_tie",
                    reader_witness=witness,
                    connection_witness=connection_witness,
                ):
                    repetitions[(cell_id, point)].append(
                        scale_02._execute_repetition(
                            config,
                            fixture,
                            rows_by_point[point],
                            point_root=root,
                            point=point,
                            repetition=repetition,
                        )
                    )
                observations[(cell_id, point)].append(
                    scale_02_followup.validate_reader_observations(
                        scale_02_followup._jsonl(witness), profile["expected"]
                    )
                )
                connection_observations[(cell_id, point)].append(
                    validate_connection_witnesses(
                        scale_02_followup._jsonl(connection_witness)
                    )
                )

    summaries = []
    for index, (cell_id, boundary, reader) in enumerate(CELLS):
        profile = {
            **document["reader_profiles"][reader],
            "id": reader,
            "footprint_order": 0 if reader == "default" else 1,
        }
        for point_index, point in enumerate(POINTS):
            summary = scale_02_followup._cell_summary(
                {"id": cell_id, "query_path": "rank_fast"},
                profile,
                repetitions[(cell_id, point)],
                bootstrap_seed=int(document["workload"]["bootstrap_seed"], 16)
                + index * 10
                + point_index,
                bootstrap_resamples=document["workload"]["bootstrap_resamples"],
            )
            summary.update(
                {
                    "point": point,
                    "boundary_handling": boundary,
                    "reader_profile": reader,
                    "reader_observations": observations[(cell_id, point)],
                    "connection_observations": connection_observations[
                        (cell_id, point)
                    ],
                }
            )
            summaries.append(summary)
    return summaries


def run(config_path: str | Path) -> dict[str, Any]:
    """Execute the approved 12-cell, 60-database factorial matrix."""
    document = load_config(config_path)
    base = scale_02.load_config(
        _path(document["dependencies"]["base_execution"]["path"], "base_execution")
    )
    runtime = document["runtime"]
    config = replace(
        base,
        python=_path(runtime["python"], "runtime.python"),
        python_extension=_path(runtime["python_extension"], "runtime.python_extension"),
        python_extension_sha256=runtime["python_extension_sha256"],
        fathomdb_bin=_path(runtime["fathomdb_bin"], "runtime.fathomdb_bin"),
        fathomdb_bin_sha256=runtime["fathomdb_bin_sha256"],
    )
    _validate_runtime(document, config)
    fixture = scale_02.load_fixture(config)
    config_sha = _lib.config_sha256(document)
    timestamp = datetime.now(UTC)
    run_id = _lib.make_run_id("scale-02-rank-boundary", timestamp, config_sha)
    run_root = _path(document["artifact_root"], "artifact_root") / run_id
    if run_root.exists():
        raise Scale02RankBoundaryError("artifact root already exists")
    run_root.mkdir(parents=True, mode=0o700)

    preflight = _run_preflight(document, fixture, run_root)
    cells = _run_performance(document, config, fixture, run_root)
    equivalence = {
        (reader, point): _run_equivalence(
            document, config, fixture, run_root, point, reader
        )
        for reader in ("default", "mmap128")
        for point in POINTS
    }
    for cell in cells:
        if cell["boundary_handling"] != "stream_complete_boundary_tie":
            continue
        evidence = equivalence[(cell["reader_profile"], cell["point"])]
        cell["equivalence"] = {
            key: value
            for key, value in evidence.items()
            if key not in {"comparisons", "reader_observation"}
        }
        cell["decision"] = evaluate_stream_cell(cell, document["policy"])

    max_regression = document["policy"]["max_25k_p50_or_p99_regression_fraction"]
    for reader in ("default", "mmap128"):
        control = next(
            cell
            for cell in cells
            if cell["point"] == 25_000
            and cell["reader_profile"] == reader
            and cell["boundary_handling"] == "shipped_full_sort_fallback"
        )
        candidate = next(
            cell
            for cell in cells
            if cell["point"] == 25_000
            and cell["reader_profile"] == reader
            and cell["boundary_handling"] == "stream_complete_boundary_tie"
        )
        if any(
            candidate["steady"][metric]
            > control["steady"][metric] * (1.0 + max_regression)
            for metric in ("p50", "p99")
        ):
            candidate["decision"]["eligible"] = False
            candidate["decision"]["reasons"].append("25k_p50_or_p99_regression")

    selection = select_candidate(cells)
    manifest, manifest_path = scale_02_followup._artifact_manifest(run_root)
    metrics = {
        "schema_version": RESULT_SCHEMA,
        "program_track": "SCALE-02",
        "status": "complete",
        "hypothesis": document["hypothesis"],
        "preflight": preflight,
        "cells": cells,
        "selection": selection,
        "artifact_manifest": {
            "sha256": scale_02_followup.sha_file(manifest_path),
            "file_count": manifest["file_count"],
            "total_bytes": manifest["total_bytes"],
        },
        "claim_boundary": document["claim_boundary"],
    }
    receipt_run_id, run_directory = _lib.write_record(
        "scale-02-rank-boundary",
        ts=timestamp,
        config_obj=document,
        metrics=metrics,
        verdict="complete",
        read="SCALE-02 rank-boundary two-factor off-shoot complete",
        code={**_lib.git_info(), "baseline_commit": runtime["base_source_git_sha"]},
        corpus={
            "source": "TC-5 qualified real prefix plus disclosed derived fixture",
            "manifest_sha256": config.qualified_manifest_sha256,
            "datasets": ["tc5-qualified-real-v2"],
        },
        seeds={
            "query_order": document["workload"]["query_order_seed"],
            "treatment_order": document["workload"]["treatment_order_seed"],
            "bootstrap": document["workload"]["bootstrap_seed"],
            "growth": config.growth_seed,
        },
        env=_lib.env_info(
            key_deps={
                "fathomdb": "0.8.23",
                "fathomdb_cli_sha256": runtime["fathomdb_bin_sha256"],
                "fathomdb_python_extension_sha256": runtime["python_extension_sha256"],
                "sqlite": runtime["sqlite_version"],
            }
        ),
        cost_usd=0.0,
        headline={
            "cells": len(cells),
            "recommended_cell": selection["recommended_cell"],
            "top100_equivalence_mismatches": sum(
                item["top100_mismatch_count"] for item in equivalence.values()
            ),
            "top10_equivalence_mismatches": sum(
                item["top10_mismatch_count"] for item in equivalence.values()
            ),
        },
        n=max(POINTS),
        config_path=str(config_path),
        tests=[
            "tests/experiments/test_scale_02_rank_boundary.py",
            "src/rust/crates/fathomdb-engine/tests/scale02_fts_rank_fast.rs",
        ],
        files_changed=[],
        artifacts=[
            {
                "kind": "external_artifact_manifest",
                "path": "artifact-manifest.v1.json",
                "sha256": scale_02_followup.sha_file(manifest_path),
            }
        ],
        review=None,
        open_questions=["HITL production landing decision remains pending"],
    )
    if receipt_run_id != run_id:
        raise Scale02RankBoundaryError("receipt run ID drifted")
    _lib.regen_index_md()
    record = run_directory / "record.json"
    return {
        "run_id": run_id,
        "record_path": str(record),
        "record_sha256": scale_02_followup.sha_file(record),
        "artifact_manifest_sha256": scale_02_followup.sha_file(manifest_path),
        "selection": selection,
    }


def main(argv: list[str] | None = None) -> int:
    """Validate or execute the approved rank-boundary matrix."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate")
    validate.add_argument("config", type=Path)
    execute = subparsers.add_parser("run")
    execute.add_argument("config", type=Path)
    args = parser.parse_args(argv)
    try:
        result = {"state": "valid"} if args.command == "validate" else run(args.config)
        if args.command == "validate":
            load_config(args.config)
    except (OSError, json.JSONDecodeError, Scale02RankBoundaryError) as exc:
        parser.error(str(exc))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
