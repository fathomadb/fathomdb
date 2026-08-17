"""Safe, release-gated execution adapter for the LOCOMO-01/PARENT-01 grid.

The module validates only content-free configuration and calls an injected cell
executor.  It never acquires LOCOMO, selects a device, invokes a model, or
constructs an external-output path itself.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from experiments import _lib, locomo_phase_a


SCHEMA_VERSION = "locomo-phase-b.v1"
RELEASE_SCHEMA_VERSION = "locomo-phase-b.release.v1"
PROGRAM_TRACK = "LOCOMO-01"
PARENT_TRACK = "PARENT-01"
PARENT_TREATMENT = "parent_child_turn_session_v1"
_REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
_SAFE_IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_TOP_LEVEL = {
    "schema_version", "campaign", "program_track", "execution", "phase_a_grid",
    "external_inputs", "metric_families", "cells", "dry_run_cell_ids", "receipt",
}
_EXECUTION = {"default_mode", "release_required"}
_PHASE_A_GRID = {"path", "sha256"}
_EXTERNAL_INPUTS = {"corpus", "turn_provenance", "session_provenance", "dry_run_subset"}
_EXTERNAL_REF = {"identifier", "sha256", "storage"}
_DRY_RUN_SUBSET = {"identifier", "sha256", "question_count", "storage"}
_METRIC_FAMILIES = {"m1", "m2", "m4_proxy", "m6", "m7", "class_metrics"}
_RECEIPT = {"schema_version", "external_metrics_ref", "historical_output_paths"}
_RUNTIME = {"device", "cache_state"}
_PARENT_TEMPLATE = {"kind", "program_track", "ingest_unit", "retrieval", "runtime_cells", "parent_child"}
_PARENT_CHILD = {
    "version", "parent_relation", "child_top_k", "parent_bundle_limit", "parent_rank", "fusion",
    "neighbors_each_side", "max_turns_per_bundle", "max_turns_total", "safe_attribution",
}
_PARENT_CHILD_FROZEN = {
    "version": PARENT_TREATMENT,
    "parent_relation": "exact_enclosing_session",
    "child_top_k": 10,
    "parent_bundle_limit": 5,
    "parent_rank": "best_child_original_rank_then_parent_session_id",
    "fusion": "none",
    "neighbors_each_side": 1,
    "max_turns_per_bundle": 3,
    "max_turns_total": 15,
    "safe_attribution": ["parent_session_id", "seed_child_id", "ordered_neighbor_ids", "trace_source_id"],
}
_RUNTIME_CELLS = {
    ("cpu", "cold"), ("cpu", "steady"), ("gpu", "cold"), ("gpu", "steady"),
}
_METRICS = {
    "m1": "r_at_10_paired_one_sided_95",
    "m2": ["mrr", "r_at_1", "ndcg_at_10"],
    "m4_proxy": "temporal_evidence_recall",
    "m6": ["facade_query_ms", "engine_query_ms"],
    "m7": ["ingest_ack_ms", "ready_to_search_ms"],
    "class_metrics": ["factoid", "temporal", "multi_session"],
}
_RELEASE_KEYS = {
    "schema_version", "release_id", "config_sha256", "issued_by", "independent_review_sha256", "authorizations",
}


class LocomoPhaseBError(ValueError):
    """Raised when LOCOMO Phase-B execution inputs are incomplete or unsafe."""


@dataclass(frozen=True)
class GridCell:
    """One resolved, content-free LOCOMO or PARENT execution cell."""

    cell_id: str
    program_track: str
    ingest_unit: str
    treatment: str
    retrieval: str
    runtime: dict[str, str]
    parent_child: dict[str, object] | None


@dataclass(frozen=True)
class LocomoPhaseBPlan:
    """A frozen executable plan whose default behavior remains preview-only."""

    config: dict[str, object]
    config_sha256: str
    program_track: str
    cells: tuple[GridCell, ...]
    dry_run_cell_ids: tuple[str, ...]
    metric_families: dict[str, object]


@dataclass(frozen=True)
class CellExecutionRequest:
    """The only cell information released to a later external executor."""

    cell: GridCell
    mode: str
    external_root: Path


@dataclass(frozen=True)
class CellExecutionResult:
    """A safe reference to one external cell result, never raw output."""

    external_metrics_ref: str
    external_metrics_sha256: str
    metric_summary: Mapping[str, object]


@dataclass(frozen=True)
class SafeReceipt:
    """Location and identity of the common experiment receipt written safely."""

    run_id: str
    run_dir: Path


def _exact(value: object, label: str, expected: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise LocomoPhaseBError(
            f"{label} keys mismatch: missing={sorted(expected - actual)}, unknown={sorted(actual - expected)}"
        )
    return value


def _identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_IDENTIFIER.fullmatch(value) is None:
        raise LocomoPhaseBError(f"{label} must be a safe identifier")
    return value


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise LocomoPhaseBError(f"{label} must be a lowercase sha256")
    return value


def _canonical_sha256(document: object) -> str:
    encoded = json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _validate_external_ref(value: object, label: str, *, subset: bool = False) -> dict[str, object]:
    expected = _DRY_RUN_SUBSET if subset else _EXTERNAL_REF
    ref = _exact(value, label, expected)
    _identifier(ref["identifier"], f"{label}.identifier")
    _sha256(ref["sha256"], f"{label}.sha256")
    if ref["storage"] != "external_only":
        raise LocomoPhaseBError(f"{label} must remain external_only")
    if subset and (not isinstance(ref["question_count"], int) or ref["question_count"] != 32):
        raise LocomoPhaseBError("dry-run subset must freeze exactly 32 questions")
    return ref


def _load_frozen_phase_a(reference: object) -> tuple[dict[str, object], list[dict[str, str]]]:
    phase_a = _exact(reference, "phase_a_grid", _PHASE_A_GRID)
    if phase_a["path"] != "experiments/configs/locomo-01/phase-a-grid.v1.json":
        raise LocomoPhaseBError("Phase-B must consume the frozen LOCOMO Phase-A catalog")
    expected_sha = _sha256(phase_a["sha256"], "phase_a_grid.sha256")
    path = _REPOSITORY_ROOT / phase_a["path"]
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected_sha:
        raise LocomoPhaseBError("Phase-A catalog pin does not match the checked-in catalog")
    try:
        phase_a_document = json.loads(path.read_text(encoding="utf-8"))
        phase_a_config = locomo_phase_a.resolve_config(phase_a_document)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        raise LocomoPhaseBError("frozen Phase-A catalog is invalid") from exc
    return phase_a_config, locomo_phase_a.grid_cells(phase_a_config)


def _runtime_cells(value: object) -> list[dict[str, str]]:
    if not isinstance(value, list) or len(value) != 4:
        raise LocomoPhaseBError("parent treatment must declare four matching runtime cells")
    normalized: list[dict[str, str]] = []
    for raw in value:
        runtime = _exact(raw, "parent runtime cell", _RUNTIME)
        if not isinstance(runtime["device"], str) or not isinstance(runtime["cache_state"], str):
            raise LocomoPhaseBError("parent runtime cell values must be strings")
        normalized.append({"device": runtime["device"], "cache_state": runtime["cache_state"]})
    if {(runtime["device"], runtime["cache_state"]) for runtime in normalized} != _RUNTIME_CELLS:
        raise LocomoPhaseBError("parent treatment must match hybrid CPU/GPU cold/steady runtime cells")
    return normalized


def _resolve_cells(templates: object, phase_a_config: dict[str, object], phase_a_cells: list[dict[str, str]]) -> tuple[GridCell, ...]:
    if not isinstance(templates, list) or len(templates) != 2:
        raise LocomoPhaseBError("Phase-B must contain exactly the LOCOMO grid and one PARENT treatment")
    phase_a_template, parent_template = templates
    if phase_a_template != {"kind": "phase_a_grid", "program_track": PROGRAM_TRACK}:
        raise LocomoPhaseBError("LOCOMO cells must be the frozen Phase-A grid")
    parent = _exact(parent_template, "parent treatment", _PARENT_TEMPLATE)
    if parent["kind"] != PARENT_TREATMENT or parent["program_track"] != PARENT_TRACK:
        raise LocomoPhaseBError("PARENT treatment identifier or program track drifted")
    if parent["ingest_unit"] != "turn" or parent["retrieval"] != "hybrid":
        raise LocomoPhaseBError("PARENT treatment must retrieve individual turn children with hybrid top-10")
    runtime_cells = _runtime_cells(parent["runtime_cells"])
    parent_child = _exact(parent["parent_child"], "parent_child", _PARENT_CHILD)
    if parent_child != _PARENT_CHILD_FROZEN:
        raise LocomoPhaseBError("PARENT treatment semantics drifted from parent_child_turn_session_v1")

    treatments = {item["id"]: item for item in phase_a_config["grid"]["treatments"]}
    resolved: list[GridCell] = []
    for item in phase_a_cells:
        treatment = treatments[item["treatment"]]
        resolved.append(GridCell(
            cell_id=item["id"], program_track=PROGRAM_TRACK, ingest_unit=item["ingest_unit"],
            treatment=item["treatment"], retrieval=treatment["retrieval"],
            runtime={"device": item["device"], "cache_state": item["cache_state"]}, parent_child=None,
        ))
    for runtime in runtime_cells:
        resolved.append(GridCell(
            cell_id=f"turn--{PARENT_TREATMENT}--{runtime['device']}--{runtime['cache_state']}",
            program_track=PARENT_TRACK, ingest_unit="turn", treatment=PARENT_TREATMENT, retrieval="hybrid",
            runtime=runtime, parent_child=dict(parent_child),
        ))
    resolved.sort(key=lambda cell: cell.cell_id)
    if len(resolved) != 52 or len({cell.cell_id for cell in resolved}) != 52:
        raise LocomoPhaseBError("resolved LOCOMO/PARENT grid must contain 52 unique cells")
    return tuple(resolved)


def load_config(document: object) -> LocomoPhaseBPlan:
    """Validate a content-free LOCOMO/PARENT execution configuration."""
    root = _exact(document, "config", _TOP_LEVEL)
    if root["schema_version"] != SCHEMA_VERSION or root["campaign"] != "authorized_phase_b_execution":
        raise LocomoPhaseBError("config must declare the authorized LOCOMO Phase-B execution schema")
    if root["program_track"] != PROGRAM_TRACK:
        raise LocomoPhaseBError("program_track must be LOCOMO-01")
    execution = _exact(root["execution"], "execution", _EXECUTION)
    if execution != {"default_mode": "preview_only", "release_required": True}:
        raise LocomoPhaseBError("Phase-B must remain preview-only until a coordinator release")
    external_inputs = _exact(root["external_inputs"], "external_inputs", _EXTERNAL_INPUTS)
    for key in ("corpus", "turn_provenance", "session_provenance"):
        _validate_external_ref(external_inputs[key], f"external_inputs.{key}")
    _validate_external_ref(external_inputs["dry_run_subset"], "external_inputs.dry_run_subset", subset=True)
    if root["metric_families"] != _METRICS:
        raise LocomoPhaseBError("LOCOMO metric families drifted from the frozen contract")
    receipt = _exact(root["receipt"], "receipt", _RECEIPT)
    if receipt["schema_version"] != "locomo-phase-b.safe-receipt.v1":
        raise LocomoPhaseBError("receipt schema must remain locomo-phase-b.safe-receipt.v1")
    if receipt["external_metrics_ref"] != "locomo-phase-b-metrics.v1":
        raise LocomoPhaseBError("receipt must retain the fixed external metrics reference")
    if receipt["historical_output_paths"] != []:
        raise LocomoPhaseBError("historical output paths are not valid LOCOMO Phase-B destinations")
    phase_a_config, phase_a_cells = _load_frozen_phase_a(root["phase_a_grid"])
    cells = _resolve_cells(root["cells"], phase_a_config, phase_a_cells)
    dry_run_cell_ids = root["dry_run_cell_ids"]
    if not isinstance(dry_run_cell_ids, list) or any(not isinstance(cell_id, str) for cell_id in dry_run_cell_ids):
        raise LocomoPhaseBError("dry_run_cell_ids must be a list of safe cell identifiers")
    if tuple(dry_run_cell_ids) != (
        "turn--fts_only--cpu--cold", "turn--hybrid--cpu--steady", "session--fts_only--cpu--cold",
        "session--hybrid--cpu--steady", "turn--parent_child_turn_session_v1--cpu--cold",
    ):
        raise LocomoPhaseBError("fixed-subset dry run cell selection drifted")
    available = {cell.cell_id for cell in cells}
    if not set(dry_run_cell_ids).issubset(available) or len(set(dry_run_cell_ids)) != len(dry_run_cell_ids):
        raise LocomoPhaseBError("fixed-subset dry run contains unavailable or duplicate cells")
    return LocomoPhaseBPlan(
        config=dict(root), config_sha256=_canonical_sha256(root), program_track=PROGRAM_TRACK,
        cells=cells, dry_run_cell_ids=tuple(dry_run_cell_ids), metric_families=dict(root["metric_families"]),
    )


def preview(plan: LocomoPhaseBPlan) -> dict[str, object]:
    """Return a content-free summary without an external root or executable command."""
    return {
        "schema_version": "locomo-phase-b.preview.v1",
        "program_track": plan.program_track,
        "execution": {"default_mode": "preview_only", "release_required": True},
        "cell_count": len(plan.cells),
        "dry_run_cell_ids": list(plan.dry_run_cell_ids),
        "cells": [
            {"cell_id": cell.cell_id, "program_track": cell.program_track, "runtime": cell.runtime}
            for cell in plan.cells
        ],
    }


def _external_root(path: str | Path) -> Path:
    root = Path(path).resolve()
    if root.is_relative_to(_REPOSITORY_ROOT):
        raise LocomoPhaseBError("external root must remain outside the repository and historic output paths")
    if not root.is_dir():
        raise LocomoPhaseBError("external root must already exist before execution")
    return root


def _validate_release(plan: LocomoPhaseBPlan, release: object) -> None:
    token = _exact(release, "release", _RELEASE_KEYS)
    if token["schema_version"] != RELEASE_SCHEMA_VERSION:
        raise LocomoPhaseBError("release schema mismatch")
    _identifier(token["release_id"], "release.release_id")
    if _sha256(token["config_sha256"], "release.config_sha256") != plan.config_sha256:
        raise LocomoPhaseBError("release config hash does not match the frozen plan")
    if token["issued_by"] != "track-runner-coordinator":
        raise LocomoPhaseBError("release must be issued by the Track Runner coordinator")
    _sha256(token["independent_review_sha256"], "release.independent_review_sha256")
    if token["authorizations"] != ["seq-249", "seq-250"]:
        raise LocomoPhaseBError("release must retain the LOCOMO and PARENT HITL authorizations")


def _validate_result(result: object) -> CellExecutionResult:
    if not isinstance(result, CellExecutionResult):
        raise LocomoPhaseBError("cell executor must return CellExecutionResult")
    _identifier(result.external_metrics_ref, "external metrics reference")
    _sha256(result.external_metrics_sha256, "external metrics sha256")
    _safe_metric_mapping(result.metric_summary, "metric_summary")
    return result


def _safe_metric_mapping(value: object, label: str) -> None:
    if not isinstance(value, Mapping):
        raise LocomoPhaseBError(f"{label} must be a metric mapping")
    for key, item in value.items():
        _identifier(key, f"{label} key")
        if isinstance(item, Mapping):
            _safe_metric_mapping(item, f"{label}.{key}")
        elif isinstance(item, list):
            if any(isinstance(entry, (Mapping, list, str)) or not isinstance(entry, (bool, int, float, type(None))) for entry in item):
                raise LocomoPhaseBError(f"{label}.{key} must contain safe scalar metrics")
        elif isinstance(item, str) or not isinstance(item, (bool, int, float, type(None))):
            raise LocomoPhaseBError(f"{label}.{key} must be a safe scalar metric")


def execute(
    plan: LocomoPhaseBPlan,
    *,
    release: object,
    external_root: str | Path,
    executor: Callable[[CellExecutionRequest], CellExecutionResult],
    mode: str = "dry_run",
) -> tuple[CellExecutionResult, ...]:
    """Execute only through a reviewed coordinator release and injected adapter."""
    _validate_release(plan, release)
    root = _external_root(external_root)
    if mode not in {"dry_run", "full_grid"}:
        raise LocomoPhaseBError("execution mode must be dry_run or full_grid")
    selected = set(plan.dry_run_cell_ids) if mode == "dry_run" else {cell.cell_id for cell in plan.cells}
    results = [
        _validate_result(executor(CellExecutionRequest(cell=cell, mode=mode, external_root=root)))
        for cell in plan.cells if cell.cell_id in selected
    ]
    if len(results) != len(selected):
        raise LocomoPhaseBError("cell executor did not cover every selected frozen cell")
    return tuple(results)


def parent_child_bundles(hits: Sequence[object]) -> list[dict[str, object]]:
    """Deduplicate ranked turn hits into the approved bounded session contexts."""
    if not isinstance(hits, Sequence) or isinstance(hits, (str, bytes)):
        raise LocomoPhaseBError("parent-child hits must be a sequence")
    selected: dict[str, tuple[int, dict[str, object]]] = {}
    for raw in hits:
        if not isinstance(raw, Mapping) or set(raw) != {
            "child_id", "parent_session_id", "rank", "neighbors", "trace_source_id",
        }:
            raise LocomoPhaseBError("parent-child hit fields mismatch")
        child_id = _identifier(raw["child_id"], "child_id")
        parent_session_id = _identifier(raw["parent_session_id"], "parent_session_id")
        trace_source_id = _identifier(raw["trace_source_id"], "trace_source_id")
        rank = raw["rank"]
        if not isinstance(rank, int) or isinstance(rank, bool) or not 1 <= rank <= 10:
            raise LocomoPhaseBError("parent-child rank must preserve one hybrid top-10 child rank")
        neighbors = raw["neighbors"]
        if not isinstance(neighbors, list) or len(neighbors) > 2:
            raise LocomoPhaseBError("parent-child neighbors exceed the one-per-side bound")
        neighbor_ids: list[str] = []
        for neighbor in neighbors:
            if isinstance(neighbor, str):
                neighbor_id = _identifier(neighbor, "neighbor id")
            elif isinstance(neighbor, Mapping) and set(neighbor) == {"id", "parent_session_id"}:
                neighbor_id = _identifier(neighbor["id"], "neighbor id")
                if neighbor["parent_session_id"] != parent_session_id:
                    raise LocomoPhaseBError("cross-session neighbor is forbidden")
            else:
                raise LocomoPhaseBError("neighbor must preserve safe session attribution")
            if neighbor_id == child_id or neighbor_id in neighbor_ids:
                raise LocomoPhaseBError("neighbor identity is ambiguous")
            neighbor_ids.append(neighbor_id)
        bundle = {
            "parent_session_id": parent_session_id,
            "seed_child_id": child_id,
            "ordered_neighbor_ids": neighbor_ids,
            "trace_source_id": trace_source_id,
        }
        prior = selected.get(parent_session_id)
        if prior is None or (rank, parent_session_id) < (prior[0], parent_session_id):
            selected[parent_session_id] = (rank, bundle)
    ordered = sorted(selected.values(), key=lambda item: (item[0], item[1]["parent_session_id"]))[:5]
    return [bundle for _, bundle in ordered]


def write_safe_receipt(
    plan: LocomoPhaseBPlan,
    *,
    release: object,
    base_dir: str | Path,
    external_root: str | Path,
    status: str,
    result_refs: Sequence[object],
) -> SafeReceipt:
    """Write one normal receipt/index row using content-free external references."""
    _validate_release(plan, release)
    _external_root(external_root)
    if status not in {"dry_run_proof", "complete"}:
        raise LocomoPhaseBError("receipt status must be dry_run_proof or complete")
    results = [_validate_result(result) for result in result_refs]
    if not results:
        raise LocomoPhaseBError("safe receipt requires at least one external result reference")
    artifacts = [
        {"path": result.external_metrics_ref, "sha256": result.external_metrics_sha256}
        for result in results
    ]
    run_id, run_dir = _lib.write_record(
        "locomo-phase-b",
        ts=datetime.now(timezone.utc).replace(second=0, microsecond=0),
        config_obj=plan.config,
        metrics={"status": status, "result_count": len(results), "metric_summaries": [dict(result.metric_summary) for result in results]},
        verdict="complete",
        read="LOCOMO/PARENT fixed-subset proof" if status == "dry_run_proof" else "LOCOMO/PARENT grid completed",
        code=_lib.git_info(),
        corpus={"source": "LOCOMO", "manifest_sha256": plan.config["external_inputs"]["corpus"]["sha256"], "datasets": []},
        seeds={"m1_bootstrap_seed": 20260814, "m1_bootstrap_resamples": 10000},
        env=_lib.env_info(),
        cost_usd=0.0,
        headline={"program_track": PROGRAM_TRACK, "status": status},
        n=(plan.config["external_inputs"]["dry_run_subset"]["question_count"]
           if status == "dry_run_proof" else None),
        artifacts=artifacts,
        base_dir=base_dir,
    )
    return SafeReceipt(run_id=run_id, run_dir=run_dir)


def main(argv: list[str] | None = None) -> int:
    """Validate or preview the release-gated plan without enabling execution."""
    parser = argparse.ArgumentParser(description="LOCOMO-01/PARENT-01 Phase-B safe executor")
    subcommands = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "preview"):
        command = subcommands.add_parser(name)
        command.add_argument("config", type=Path)
    args = parser.parse_args(argv)
    try:
        plan = load_config(json.loads(args.config.read_text(encoding="utf-8")))
    except (OSError, json.JSONDecodeError, LocomoPhaseBError) as exc:
        parser.error(str(exc))
    if args.command == "preview":
        print(json.dumps(preview(plan), indent=2))
    else:
        print("locomo Phase-B release-gated execution plan resolves")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
