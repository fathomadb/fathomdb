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
    return {
        "seed": seed,
        "direction": direction,
        "depth": depth,
        "target_index": 0,
        "target_ref": f"target:{seed}:{direction}:{depth}",
        "terminal_ref": f"terminal:{seed}:{direction}:{depth}",
        "resolved_target_revision": 1,
        "resolved_edge_revision": 1,
        "edge_source": edge_source,
        "route_provenance": ["edge:0"],
        "intrinsic_evidence": ["matched_node"],
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
    print("PASS test-slice50-evidence-matrix")


if __name__ == "__main__":
    main()
