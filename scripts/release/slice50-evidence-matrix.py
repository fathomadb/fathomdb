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
    expected = required_case_keys()
    missing = expected - seen
    if missing:
        raise ValueError(f"missing graph evidence case: {sorted(missing)}")
    unexpected = seen - expected
    if unexpected:
        raise ValueError(f"unexpected graph evidence case: {sorted(unexpected)}")
    required = {
        "target_index",
        "hop_count",
        "target_ref",
        "terminal_ref",
        "resolved_target_revision",
        "resolved_target_logical_id",
        "resolved_edge_revision",
        "resolved_edge_class",
        "resolved_edge_kind",
        "resolved_edge_from",
        "resolved_edge_to",
        "edge_source",
        "route_provenance",
        "intrinsic_evidence",
    }
    positions: dict[tuple[str, str, int], set[int]] = {}
    for row in materialized:
        absent = required - row.keys()
        if absent:
            raise ValueError(f"missing intrinsic evidence fields: {sorted(absent)}")
        if "ranking_contribution" in row:
            raise ValueError("ranked-only contribution is forbidden in intrinsic evidence")
        target_index = row["target_index"]
        if isinstance(target_index, bool) or not isinstance(target_index, int) or target_index < 0:
            raise ValueError("invalid target index")
        case = case_key(row)
        case_positions = positions.setdefault(case, set())
        if target_index in case_positions:
            raise ValueError(f"duplicate target index for graph evidence case: {case}")
        case_positions.add(target_index)
        route = row["route_provenance"]
        intrinsic = row["intrinsic_evidence"]
        if (
            not isinstance(route, list)
            or len(route) != 5
            or not all(isinstance(value, str) and value for value in route)
            or not isinstance(intrinsic, list)
            or len(intrinsic) != 2
            or not all(isinstance(value, str) and value for value in intrinsic)
        ):
            raise ValueError("empty route or intrinsic provenance")
        if row["resolved_target_revision"] != intrinsic[0]:
            raise ValueError("resolved target revision does not match intrinsic evidence")
        if row["resolved_edge_revision"] != intrinsic[1]:
            raise ValueError("resolved edge revision does not match intrinsic evidence")
        if row["resolved_target_logical_id"] != route[2]:
            raise ValueError("resolved target identity does not match route provenance")
        if row["resolved_edge_class"] != "edge" or row["resolved_edge_kind"] != route[3]:
            raise ValueError("resolved edge identity does not match route provenance")
        if route[4] == "outgoing":
            expected_endpoints = (route[1], route[2])
        elif route[4] == "incoming":
            expected_endpoints = (route[2], route[1])
        else:
            raise ValueError("invalid terminal direction")
        if (row["resolved_edge_from"], row["resolved_edge_to"]) != expected_endpoints:
            raise ValueError("resolved edge endpoints do not match route provenance")
        if row["edge_source"] not in {"ordinary", "actuated"}:
            raise ValueError("invalid edge source")
    edge_sources = {row["edge_source"] for row in materialized}
    if edge_sources != {"ordinary", "actuated"}:
        raise ValueError("matrix must retain ordinary and actuated-edge evidence")
    for case, case_positions in positions.items():
        if case_positions != set(range(len(case_positions))):
            raise ValueError(f"non-contiguous target indexes for graph evidence case: {case}")
    for case in expected:
        if not any(case_key(row) == case and row["hop_count"] == case[2] for row in materialized):
            raise ValueError(f"multihop target missing for graph evidence case: {case}")
