#!/usr/bin/env python3
"""Contract for Slice 50's mechanically complete installed graph profile."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/release/slice50-evidence-matrix.py"


def load_module():
    spec = importlib.util.spec_from_file_location("slice50_evidence_matrix", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def row(seed: str, direction: str, depth: int, edge_source: str = "ordinary") -> dict:
    predecessor = "root"
    target = f"target:{seed}:{direction}:{depth}"
    terminal_direction = "incoming" if direction == "incoming" else "outgoing"
    edge_from, edge_to = (
        (target, predecessor)
        if terminal_direction == "incoming"
        else (predecessor, target)
    )
    return {
        "seed": seed,
        "direction": direction,
        "depth": depth,
        "hop_count": depth,
        "target_index": 0,
        "target_ref": f"target:{seed}:{direction}:{depth}",
        "terminal_ref": f"terminal:{seed}:{direction}:{depth}",
        "resolved_target_revision": "target-r1",
        "resolved_target_logical_id": target,
        "resolved_edge_revision": "edge-r1",
        "resolved_edge_class": "edge",
        "resolved_edge_kind": "supports",
        "resolved_edge_from": edge_from,
        "resolved_edge_to": edge_to,
        "edge_source": edge_source,
        "route_provenance": [
            "seed",
            predecessor,
            target,
            "supports",
            terminal_direction,
        ],
        "intrinsic_evidence": ["target-r1", "edge-r1"],
    }


def main() -> None:
    assert SCRIPT.is_file(), f"missing evidence matrix helper: {SCRIPT}"
    module = load_module()
    expected = {
        (seed, direction, depth)
        for seed in ("explicit", "query")
        for direction in ("outgoing", "incoming", "both")
        for depth in (1, 2)
    }
    assert module.required_case_keys() == expected
    rows = [row(*key) for key in sorted(expected)]
    rows[0]["edge_source"] = "actuated"
    module.validate_rows(rows)

    for omitted in expected:
        incomplete = [item for item in rows if module.case_key(item) != omitted]
        try:
            module.validate_rows(incomplete)
        except ValueError as error:
            assert "missing graph evidence case" in str(error)
        else:
            raise AssertionError(f"accepted matrix without {omitted}")

    ordinary = [dict(item, edge_source="ordinary") for item in rows]
    try:
        module.validate_rows(ordinary)
    except ValueError as error:
        assert "actuated" in str(error)
    else:
        raise AssertionError("accepted matrix without actuated-edge evidence")

    polluted = [dict(item) for item in rows]
    polluted[1]["ranking_contribution"] = 0.5
    try:
        module.validate_rows(polluted)
    except ValueError as error:
        assert "ranked-only" in str(error)
    else:
        raise AssertionError("accepted ranked-only contribution in intrinsic profile")

    mislabeled = [
        dict(item, hop_count=1) if item["depth"] == 2 else dict(item)
        for item in rows
    ]
    try:
        module.validate_rows(mislabeled)
    except ValueError as error:
        assert "multihop" in str(error)
    else:
        raise AssertionError("accepted depth-two labels without a two-hop target")

    for mutation, expected_error in (
        ({"target_index": -1}, "target index"),
        ({"resolved_target_revision": "other-r1"}, "target revision"),
        ({"resolved_edge_revision": "other-edge-r1"}, "edge revision"),
        ({"resolved_target_logical_id": "other-target"}, "target identity"),
        ({"resolved_edge_from": "other-source"}, "edge endpoints"),
    ):
        invalid = [dict(item) for item in rows]
        invalid[0].update(mutation)
        try:
            module.validate_rows(invalid)
        except ValueError as error:
            assert expected_error in str(error)
        else:
            raise AssertionError(f"accepted invalid graph evidence: {mutation}")

    duplicate_position = [dict(item) for item in rows]
    duplicate_position.insert(1, dict(duplicate_position[0]))
    try:
        module.validate_rows(duplicate_position)
    except ValueError as error:
        assert "duplicate target index" in str(error)
    else:
        raise AssertionError("accepted duplicate target position")
    print("PASS test-slice50-evidence-matrix")


if __name__ == "__main__":
    main()
