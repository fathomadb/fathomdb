#!/usr/bin/env python3
"""Run the two preregistered SCALE-02 post-boundary reader hypotheses."""

from __future__ import annotations

import argparse
import importlib
import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping

from experiments import _lib, scale_02, scale_02_followup


REPO_ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "scale-02-scale-hypotheses.v1"
RESULT_SCHEMA = "scale-02-scale-hypotheses-result.v1"
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "authorization",
    "dependencies",
    "hypothesis",
    "points",
    "treatments",
    "workload",
    "policy",
    "runtime_source_git_sha",
    "artifact_root",
    "claim_boundary",
}


class Scale02ScaleFollowupError(RuntimeError):
    """Raised when the post-boundary hypothesis contract drifts."""


def _path(value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else REPO_ROOT / path


def _exact(value: Mapping[str, Any], expected: set[str], name: str) -> None:
    if set(value) != expected:
        raise Scale02ScaleFollowupError(f"{name} keys drifted")


def load_config(path: str | Path) -> dict[str, Any]:
    """Load and strictly validate the frozen scale-hypothesis configuration."""
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise Scale02ScaleFollowupError("configuration must be an object")
    _exact(document, _TOP_KEYS, "configuration")
    if document["schema_version"] != SCHEMA or document["program_track"] != "SCALE-02":
        raise Scale02ScaleFollowupError("configuration identity drifted")
    authorization = document["authorization"]
    _exact(
        authorization,
        {"state", "approved_by", "ledger_ref", "maximum_hypotheses"},
        "authorization",
    )
    if authorization != {
        "state": "approved",
        "approved_by": "HITL",
        "ledger_ref": "seq-265",
        "maximum_hypotheses": 2,
    }:
        raise Scale02ScaleFollowupError("authorization drifted")
    if document["points"] != [25000, 40000, 50000]:
        raise Scale02ScaleFollowupError("points drifted")
    treatments = document["treatments"]
    if not isinstance(treatments, list) or len(treatments) != 2:
        raise Scale02ScaleFollowupError("exactly two treatments are required")
    if [item.get("id") for item in treatments] != ["rank_mmap128", "rank_cache64"]:
        raise Scale02ScaleFollowupError("treatments drifted")
    for dependency in document["dependencies"].values():
        _exact(dependency, {"path", "sha256"}, "dependency")
        target = _path(dependency["path"])
        if not target.is_file() or scale_02_followup.sha_file(target) != dependency["sha256"]:
            raise Scale02ScaleFollowupError("dependency digest drifted")
    artifact_root = _path(document["artifact_root"])
    if artifact_root.resolve().is_relative_to(REPO_ROOT.resolve()):
        raise Scale02ScaleFollowupError("artifact root must be outside the repository")
    return document


def evaluate_cell(cell: Mapping[str, Any], policy: Mapping[str, Any]) -> dict[str, Any]:
    """Evaluate one cell against the frozen accuracy, latency, and resource policy."""
    point = str(cell["point"])
    reasons = []
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
    if (
        policy["require_exact_ordered_top10"]
        and (
            equivalence["query_count"] != 100
            or equivalence["mismatch_count"] != 0
        )
    ):
        reasons.append("retrieval_equivalence")
    return {"eligible": not reasons, "reasons": reasons}


def _validate_runtime(config: scale_02.Scale02Config) -> None:
    if Path(sys.executable).resolve() != config.python.resolve():
        raise Scale02ScaleFollowupError("runner is not using the pinned Python runtime")
    module = importlib.import_module("fathomdb._fathomdb")
    loaded = Path(module.__file__).resolve()
    if (
        loaded != config.python_extension.resolve()
        or scale_02_followup.sha_file(loaded) != config.python_extension_sha256
    ):
        raise Scale02ScaleFollowupError("loaded FathomDB extension drifted")
    scale_02._validate_runtime(config)


def _run_cell(
    document: Mapping[str, Any],
    scale_config: scale_02.Scale02Config,
    fixture: scale_02.Fixture,
    treatment: Mapping[str, Any],
    point: int,
    run_root: Path,
    cell_index: int,
) -> dict[str, Any]:
    profile = treatment["reader_profile"]
    cell_root = run_root / treatment["id"] / str(point)
    cell_root.mkdir(parents=True, exist_ok=False)
    rows = scale_02.build_rows(fixture.documents, point, seed=scale_config.growth_seed)
    repetitions = []
    observations = []
    for repetition in range(1, scale_config.repetitions + 1):
        witness = cell_root / f"reader-settings-rep-{repetition}.jsonl"
        with scale_02_followup._treatment_environment(
            profile,
            rank_fast=True,
            reader_witness=witness,
        ):
            repetitions.append(
                scale_02._execute_repetition(
                    scale_config,
                    fixture,
                    rows,
                    point_root=cell_root,
                    point=point,
                    repetition=repetition,
                )
            )
        observations.append(
            scale_02_followup.validate_reader_observations(
                scale_02_followup._jsonl(witness), profile["expected"]
            )
        )

    equivalence_reader = cell_root / "equivalence-reader-settings.jsonl"
    equivalence_routes = cell_root / "equivalence-routes.jsonl"
    database = cell_root / f"point-{point}-rep-1" / "fathomdb.sqlite"
    with scale_02_followup._treatment_environment(
        profile,
        rank_fast=True,
        reader_witness=equivalence_reader,
        route_witness=equivalence_routes,
    ):
        engine = scale_02_followup._open_engine(database)
        try:
            equivalence = scale_02_followup._compare_queries(
                engine,
                list(fixture.queries)[: document["workload"]["equivalence_queries_per_cell"]],
                equivalence_routes,
            )
        finally:
            engine.close()
    equivalence["reader_observation"] = scale_02_followup.validate_reader_observations(
        scale_02_followup._jsonl(equivalence_reader), profile["expected"]
    )

    summary = scale_02_followup._cell_summary(
        {"id": treatment["id"], "query_path": "rank_fast"},
        profile,
        repetitions,
        bootstrap_seed=int(document["workload"]["bootstrap_seed"], 16)
        + cell_index * 10,
        bootstrap_resamples=document["workload"]["bootstrap_resamples"],
    )
    summary["point"] = point
    summary["reader_observations"] = observations
    summary["equivalence"] = {
        "query_count": equivalence["query_count"],
        "mismatch_count": equivalence["mismatch_count"],
        "route_counts": equivalence["route_counts"],
    }
    summary["decision"] = evaluate_cell(summary, document["policy"])
    path = cell_root / "cell-summary.v1.json"
    path.write_text(json.dumps(summary, sort_keys=True) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return summary


def run(config_path: str | Path) -> dict[str, Any]:
    """Execute all six preregistered treatment-by-size cells."""
    document = load_config(config_path)
    base_path = _path(document["dependencies"]["base_execution"]["path"])
    scale_config = scale_02.load_config(base_path)
    _validate_runtime(scale_config)
    fixture = scale_02.load_fixture(scale_config)
    config_sha = _lib.config_sha256(document)
    timestamp = datetime.now(UTC)
    run_id = _lib.make_run_id("scale-02-scale-hypotheses", timestamp, config_sha)
    run_root = _path(document["artifact_root"]) / run_id
    if run_root.exists():
        raise Scale02ScaleFollowupError("artifact root already exists")
    run_root.mkdir(parents=True, mode=0o700)

    cells = []
    for treatment_index, treatment in enumerate(document["treatments"]):
        for point_index, point in enumerate(document["points"]):
            cells.append(
                _run_cell(
                    document,
                    scale_config,
                    fixture,
                    treatment,
                    point,
                    run_root,
                    treatment_index * len(document["points"]) + point_index,
                )
            )
    manifest, manifest_path = scale_02_followup._artifact_manifest(run_root)
    metrics = {
        "schema_version": RESULT_SCHEMA,
        "program_track": "SCALE-02",
        "status": "complete",
        "hypothesis": document["hypothesis"],
        "cells": cells,
        "artifact_manifest": {
            "sha256": scale_02_followup.sha_file(manifest_path),
            "file_count": manifest["file_count"],
            "total_bytes": manifest["total_bytes"],
        },
        "claim_boundary": document["claim_boundary"],
    }
    receipt_run_id, run_directory = _lib.write_record(
        "scale-02-scale-hypotheses",
        ts=timestamp,
        config_obj=document,
        metrics=metrics,
        verdict="complete",
        read="SCALE-02 two post-boundary accuracy-preserving reader hypotheses complete",
        code={
            **_lib.git_info(),
            "baseline_commit": document["runtime_source_git_sha"],
        },
        corpus={
            "source": "TC-5 qualified real prefix plus disclosed derived fixture",
            "manifest_sha256": scale_config.qualified_manifest_sha256,
            "datasets": ["tc5-qualified-real-v2"],
        },
        seeds={
            "query_order": document["workload"]["query_order_seed"],
            "bootstrap": document["workload"]["bootstrap_seed"],
            "growth": scale_config.growth_seed,
        },
        env=_lib.env_info(
            key_deps={
                "fathomdb": "0.8.23",
                "fathomdb_cli_sha256": scale_config.fathomdb_bin_sha256,
                "fathomdb_python_extension_sha256": scale_config.python_extension_sha256,
            }
        ),
        cost_usd=0.0,
        headline={
            "cells": len(cells),
            "eligible_cells": sum(cell["decision"]["eligible"] for cell in cells),
            "equivalence_mismatches": sum(
                cell["equivalence"]["mismatch_count"] for cell in cells
            ),
        },
        n=max(document["points"]),
        config_path=str(config_path),
        tests=["tests/experiments/test_scale_02_scale_followup.py"],
        files_changed=[],
        artifacts=[
            {
                "kind": "external_artifact_manifest",
                "path": "artifact-manifest.v1.json",
                "sha256": scale_02_followup.sha_file(manifest_path),
            }
        ],
        review=None,
        open_questions=[],
    )
    if receipt_run_id != run_id:
        raise Scale02ScaleFollowupError("receipt run ID drifted")
    _lib.regen_index_md()
    record = run_directory / "record.json"
    return {
        "run_id": run_id,
        "record_path": str(record),
        "record_sha256": scale_02_followup.sha_file(record),
        "artifact_manifest_sha256": scale_02_followup.sha_file(manifest_path),
    }


def main(argv: list[str] | None = None) -> int:
    """Validate or execute the frozen post-boundary hypothesis matrix."""
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
    except (OSError, json.JSONDecodeError, Scale02ScaleFollowupError) as exc:
        parser.error(str(exc))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
