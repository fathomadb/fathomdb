"""Validate and preview the non-executable LOCOMO-01 Phase-A grid."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "locomo-phase-a.v1"
_TOP_LEVEL = {"schema_version", "campaign", "program_track", "execution", "baseline", "corpus", "provenance", "measurement", "grid"}
_BASELINE = {
    "canonical_id", "record_run_id", "source_arm_run_id", "ingest_unit", "retrieval_mode", "top_k",
    "source_retrieval_metrics_sha256", "source_margin_contract_sha256",
}
_CORPUS = {"source", "raw_sha256", "normalized_sha256", "sessions", "source_questions", "a0_evidence_backed_questions", "payload"}
_PROVENANCE = {"source_record_run_id", "turn", "session"}
_MANIFEST = {"identifier", "sha256", "storage"}
_MEASUREMENT = {"m1", "m2", "m4", "m6", "m7", "fast_local", "quality_local_gpu"}
_M1 = {"metric", "paired_confidence_rule", "bootstrap", "margin", "sanity_range"}
_GRID = {"ingest_units", "treatments", "runtime_cells"}
_RUNTIME = {"device", "cache_state"}
_TREATMENT = {"id", "retrieval", "cross_encoder", "candidate_pool", "candidate_depth", "bounded_neighbor_expansion"}
_TREATMENT_SEMANTICS = {
    "fts_only": {
        "retrieval": "fts", "cross_encoder": None, "candidate_pool": None, "candidate_depth": None,
        "bounded_neighbor_expansion": False,
    },
    "hybrid": {
        "retrieval": "hybrid", "cross_encoder": None, "candidate_pool": None, "candidate_depth": None,
        "bounded_neighbor_expansion": False,
    },
    "hybrid_ce_alpha_03_pool_10": {
        "retrieval": "hybrid", "cross_encoder": "alpha_0.3", "candidate_pool": 10, "candidate_depth": 10,
        "bounded_neighbor_expansion": False,
    },
    "hybrid_ce_alpha_10_pool_10": {
        "retrieval": "hybrid", "cross_encoder": "alpha_1.0", "candidate_pool": 10, "candidate_depth": 10,
        "bounded_neighbor_expansion": False,
    },
    "hybrid_ce_alpha_10_pool_20": {
        "retrieval": "hybrid", "cross_encoder": "alpha_1.0", "candidate_pool": 20, "candidate_depth": 20,
        "bounded_neighbor_expansion": False,
    },
    "fts_bounded_neighbor": {
        "retrieval": "fts", "cross_encoder": None, "candidate_pool": None, "candidate_depth": None,
        "bounded_neighbor_expansion": True,
    },
}
_TREATMENT_IDS = set(_TREATMENT_SEMANTICS)
_RUNTIME_CELLS = {("cpu", "cold"), ("cpu", "steady"), ("gpu", "cold"), ("gpu", "steady")}


def _exact(value: object, name: str, expected: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise ValueError(f"{name} keys mismatch: missing={sorted(expected - actual)}, unknown={sorted(actual - expected)}")
    return value


def _sha256(value: object, name: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
        raise ValueError(f"{name} must be a lowercase sha256")
    return value


def _nonempty(value: object, name: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{name} must be a non-empty string")
    return value


def resolve_config(document: object) -> dict[str, Any]:
    """Validate the LOCOMO Phase-A catalog without authorizing execution."""
    root = _exact(document, "config", _TOP_LEVEL)
    if root["schema_version"] != SCHEMA_VERSION or root["campaign"] != "phase_a_grid_preparation":
        raise ValueError("config must declare the LOCOMO Phase-A grid-preparation schema")
    if root["program_track"] != "LOCOMO-01":
        raise ValueError("program_track must be 'LOCOMO-01'")
    if root["execution"] != {"mode": "plan_only", "live_execution": "forbidden"}:
        raise ValueError("LOCOMO Phase-A is plan_only; live execution is forbidden")

    baseline = _exact(root["baseline"], "baseline", _BASELINE)
    if baseline["canonical_id"] != "A0" or baseline["ingest_unit"] != "turn" or baseline["retrieval_mode"] != "fts_only" or baseline["top_k"] != 10:
        raise ValueError("baseline must pin canonical A0 turn-level FTS top-10")
    for key in ("record_run_id", "source_arm_run_id"):
        _nonempty(baseline[key], f"baseline.{key}")
    for key in ("source_retrieval_metrics_sha256", "source_margin_contract_sha256"):
        _sha256(baseline[key], f"baseline.{key}")

    corpus = _exact(root["corpus"], "corpus", _CORPUS)
    if corpus["source"] != "LOCOMO" or corpus["payload"] != "external_only":
        raise ValueError("Phase-A corpus must be external-only LOCOMO")
    for key in ("raw_sha256", "normalized_sha256"):
        _sha256(corpus[key], f"corpus.{key}")
    if any(not isinstance(corpus[key], int) or corpus[key] < 1 for key in ("sessions", "source_questions", "a0_evidence_backed_questions")):
        raise ValueError("Phase-A corpus counts must be positive integers")

    provenance = _exact(root["provenance"], "provenance", _PROVENANCE)
    _nonempty(provenance["source_record_run_id"], "provenance.source_record_run_id")
    for unit in ("turn", "session"):
        manifest = _exact(provenance[unit], f"provenance.{unit}", _MANIFEST)
        _nonempty(manifest["identifier"], f"provenance.{unit}.identifier")
        _sha256(manifest["sha256"], f"provenance.{unit}.sha256")
        if manifest["storage"] != "external_only":
            raise ValueError(f"provenance.{unit} manifest must remain external_only")

    measurement = _exact(root["measurement"], "measurement", _MEASUREMENT)
    m1 = _exact(measurement["m1"], "measurement.m1", _M1)
    if m1["metric"] != "r_at_10" or m1["paired_confidence_rule"] != "one_sided_95_lower_bound_at_least_negative_margin":
        raise ValueError("M1 must remain the pre-registered paired R@10 rule")
    if m1["bootstrap"] != {"seed": 20260814, "resamples": 10000}:
        raise ValueError("M1 bootstrap must pin seed 20260814 and 10000 resamples")
    if m1["margin"] != 0.02405130733344985 or m1["sanity_range"] != [0.01, 0.04]:
        raise ValueError("M1 margin must pin the historical A0 pre-registration")
    if measurement["m2"] != ["mrr", "r_at_1", "ndcg_at_10"]:
        raise ValueError("M2 metrics must be MRR, R@1, and nDCG@10")
    if measurement["m4"] != {"actual": "unmeasured", "proxy": "temporal_evidence_recall"}:
        raise ValueError("M4 must remain an unmeasured temporal-evidence proxy in Phase-A")
    if measurement["m6"] != ["facade_query_ms", "engine_query_ms"] or measurement["m7"] != ["ingest_ack_ms", "ready_to_search_ms"]:
        raise ValueError("Phase-A must define the M6 and M7 timing boundaries")
    if measurement["fast_local"] != {"p95_multiplier": 1.5, "rule": "M1 pass and candidate p95 <= 1.5 * A0 p95"}:
        raise ValueError("fast-local must preserve the pre-registered p95 rule")
    if measurement["quality_local_gpu"] != "report_only_not_a_locomo_acceptance_claim":
        raise ValueError("GPU timing must remain report-only for LOCOMO acceptance")

    grid = _exact(root["grid"], "grid", _GRID)
    if not isinstance(grid["ingest_units"], list) or set(grid["ingest_units"]) != {"turn", "session"} or len(grid["ingest_units"]) != 2:
        raise ValueError("grid must contain each ingest unit exactly once")
    if not isinstance(grid["runtime_cells"], list) or len(grid["runtime_cells"]) != 4:
        raise ValueError("grid must contain four runtime cells")
    runtime_cells = set()
    for runtime in grid["runtime_cells"]:
        runtime = _exact(runtime, "grid.runtime_cell", _RUNTIME)
        runtime_cells.add((runtime["device"], runtime["cache_state"]))
    if runtime_cells != _RUNTIME_CELLS:
        raise ValueError("grid must separate CPU/GPU and cold/steady runtime cells")
    if not isinstance(grid["treatments"], list) or len(grid["treatments"]) != len(_TREATMENT_IDS):
        raise ValueError("grid must contain each retrieval treatment exactly once")
    treatment_ids = set()
    for treatment in grid["treatments"]:
        treatment = _exact(treatment, "grid.treatment", _TREATMENT)
        treatment_id = _nonempty(treatment["id"], "grid.treatment.id")
        expected = _TREATMENT_SEMANTICS.get(treatment_id)
        if expected is None:
            raise ValueError(f"unknown grid treatment: {treatment_id}")
        actual = {key: treatment[key] for key in expected}
        if actual != expected or any(type(actual[key]) is not type(expected[key]) for key in expected):
            raise ValueError(f"grid treatment semantic tuple for {treatment_id} mismatches its frozen mapping")
        treatment_ids.add(treatment_id)
    if treatment_ids != _TREATMENT_IDS:
        raise ValueError("grid retrieval treatments mismatch")
    return root


def grid_cells(config: dict[str, Any]) -> list[dict[str, str]]:
    """Return a deterministic, content-free description of every planned cell."""
    resolved = resolve_config(config)
    cells = [
        {
            "id": f"{ingest_unit}--{treatment['id']}--{runtime['device']}--{runtime['cache_state']}",
            "ingest_unit": ingest_unit,
            "treatment": treatment["id"],
            "device": runtime["device"],
            "cache_state": runtime["cache_state"],
        }
        for ingest_unit in resolved["grid"]["ingest_units"]
        for treatment in resolved["grid"]["treatments"]
        for runtime in resolved["grid"]["runtime_cells"]
    ]
    return sorted(cells, key=lambda cell: cell["id"])


def preview(config: dict[str, Any]) -> dict[str, object]:
    """Produce a safe planning projection with no corpus paths or commands."""
    resolved = resolve_config(config)
    return {
        "schema_version": "locomo-phase-a.preview.v1",
        "program_track": resolved["program_track"],
        "execution": resolved["execution"],
        "cell_count": len(grid_cells(resolved)),
        "cells": grid_cells(resolved),
    }


def run(_config: dict[str, Any]) -> None:
    """Refuse live work; a later authorized track owns execution."""
    raise RuntimeError("LOCOMO Phase-A planning is not authorized to execute a grid")


def main(argv: list[str] | None = None) -> int:
    """Validate or safely preview a LOCOMO-01 Phase-A catalog."""
    parser = argparse.ArgumentParser(description="LOCOMO-01 Phase-A plan-only catalog")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "preview"):
        command = sub.add_parser(name)
        command.add_argument("config", type=Path)
    args = parser.parse_args(argv)
    try:
        config = resolve_config(json.loads(args.config.read_text(encoding="utf-8")))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        parser.error(str(exc))
    if args.command == "preview":
        print(json.dumps(preview(config), indent=2))
    else:
        print("locomo Phase-A plan-only catalog resolves")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
