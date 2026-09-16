#!/usr/bin/env python3
"""Validate Slice 50's installed graph-evidence profile."""

from __future__ import annotations

from collections.abc import Iterable, Mapping


def required_case_keys() -> set[tuple[str, str, int]]:
    """Return the required seed/direction/depth Cartesian product."""

    return {
        (seed, direction, depth)
        for seed in ("explicit", "query")
        for direction in ("outgoing", "incoming", "both")
        for depth in (1, 2)
    }


def case_key(row: Mapping[str, object]) -> tuple[str, str, int]:
    """Return the matrix key represented by one evidence row."""

    return str(row["seed"]), str(row["direction"]), int(row["depth"])


def validate_rows(rows: Iterable[Mapping[str, object]]) -> None:
    """Reject incomplete, ranked-only, or provenance-poor graph evidence."""

    materialized = list(rows)
    seen = {case_key(row) for row in materialized}
    missing = required_case_keys() - seen
    if missing:
        raise ValueError(f"missing graph evidence case: {sorted(missing)}")
    required = {
        "target_index",
        "hop_count",
        "target_ref",
        "terminal_ref",
        "resolved_target_revision",
        "resolved_edge_revision",
        "edge_source",
        "route_provenance",
        "intrinsic_evidence",
    }
    for row in materialized:
        absent = required - row.keys()
        if absent:
            raise ValueError(f"missing intrinsic evidence fields: {sorted(absent)}")
        if "ranking_contribution" in row:
            raise ValueError("ranked-only contribution is forbidden in intrinsic evidence")
        if not row["route_provenance"] or not row["intrinsic_evidence"]:
            raise ValueError("empty route or intrinsic provenance")
    if not any(row["edge_source"] == "actuated" for row in materialized):
        raise ValueError("matrix must retain actuated-edge evidence")
    for case in required_case_keys():
        if not any(case_key(row) == case and row["hop_count"] == case[2] for row in materialized):
            raise ValueError(f"multihop target missing for graph evidence case: {case}")
