from __future__ import annotations

import copy
from pathlib import Path

import pytest

from experiments import graph_expand_01


ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "experiments/configs/graph-expand-01/bounded-traversal.v1.json"


def test_checked_in_config_freezes_scales_and_registered_cells() -> None:
    config = graph_expand_01.load_config(CONFIG)

    assert config.program_track == "GRAPH-EXPAND-01"
    assert config.generator == {"algorithm": "graph-expand-powerlaw-v1", "seed": 20260921}
    assert [(item["id"], item["nodes"], item["edges"]) for item in config.scales] == [
        ("smoke", 1_000, 5_000),
        ("small", 10_000, 50_000),
        ("medium", 50_000, 1_000_000),
        ("large", 100_000, 5_000_000),
    ]
    assert len(config.cells) == 27
    assert all(cell["depth"] in {0, 1, 2} for cell in config.cells)

    document = copy.deepcopy(config.resolved)
    document["cells"][0]["depth"] = 3
    with pytest.raises(graph_expand_01.GraphExpandBenchmarkError, match="depth"):
        graph_expand_01.resolve_config(document)


def test_bfs_oracle_counts_raw_rows_and_uses_public_ordering() -> None:
    nodes = {
        "a": {"kind": "seed", "eligible": True},
        "b": {"kind": "fact", "eligible": True},
        "c": {"kind": "fact", "eligible": True},
        "hidden": {"kind": "excluded", "eligible": False},
    }
    edges = [
        graph_expand_01.Edge("z", "zeta", "a", "c", 1, True),
        graph_expand_01.Edge("a", "alpha", "a", "c", 2, True),
        graph_expand_01.Edge("b", "alpha", "a", "b", 3, True),
        graph_expand_01.Edge("hidden", "link", "a", "hidden", 4, True),
        graph_expand_01.Edge("dead", "link", "a", "b", 5, False),
    ]

    result = graph_expand_01.bfs_oracle(
        nodes=nodes,
        edges=edges,
        seeds=("a",),
        direction="outgoing",
        max_depth=1,
        result_limit=10,
        eligible=lambda logical_id: bool(nodes[logical_id]["eligible"]),
    )

    assert result.work_units == 5
    assert [target.logical_id for target in result.targets] == ["b", "c"]
    assert result.targets[1].edge_kind == "alpha"
    assert result.targets[1].edge_logical_id == "a"


def test_query_seed_expectation_precedes_self_consistent_traversal() -> None:
    expected = ("node-000010", "node-000011", "node-000012", "node-000013", "node-000014")
    graph_expand_01.validate_query_seeds(expected, expected)
    with pytest.raises(graph_expand_01.GraphExpandBenchmarkError, match="query seeds"):
        graph_expand_01.validate_query_seeds(expected, ("wrong",))


def test_generator_is_deterministic_and_exact() -> None:
    first = graph_expand_01.generate_graph(node_count=50, edge_count=200, seed=17)
    second = graph_expand_01.generate_graph(node_count=50, edge_count=200, seed=17)
    changed = graph_expand_01.generate_graph(node_count=50, edge_count=200, seed=18)

    assert first == second
    assert len(first["nodes"]) == 50
    assert len(first["edges"]) == 200
    assert first["adjacency_sha256"] != changed["adjacency_sha256"]


def test_bound_probe_rules_stay_inside_public_api() -> None:
    assert graph_expand_01.bound_budgets(3) == {"below": 2, "exact": 3, "above": 4}
    assert graph_expand_01.bound_budgets(0) == {"minimum_valid": 1}
    assert graph_expand_01.bound_budgets(1) == {"exact": 1, "above": 2}
    assert graph_expand_01.bound_budgets(10_000) == {"below": 9_999, "exact": 10_000}


def test_bfs_oracle_deduplicates_cycles_by_least_public_origin() -> None:
    nodes = {key: {"kind": "fact"} for key in ("a", "b", "c", "d")}
    edges = [
        graph_expand_01.Edge("ab", "link", "a", "b", 1),
        graph_expand_01.Edge("bc", "link", "b", "c", 2),
        graph_expand_01.Edge("ca", "link", "c", "a", 3),
        graph_expand_01.Edge("dc", "link", "d", "c", 4),
    ]

    result = graph_expand_01.bfs_oracle(
        nodes=nodes,
        edges=edges,
        seeds=("a", "d"),
        direction="outgoing",
        max_depth=2,
        result_limit=10,
    )

    assert [target.logical_id for target in result.targets] == ["b", "c"]
    c = next(target for target in result.targets if target.logical_id == "c")
    assert (c.hop_count, c.seed_ordinal, c.predecessor_logical_id) == (1, 1, "d")
