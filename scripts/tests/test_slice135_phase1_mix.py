"""The Phase 1 query mix preserves slow calls and refuses unverified observations."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_phase1_mix as mix  # noqa: E402


def test_equal_frequency_cost_prefix_uses_sum_of_all_calls() -> None:
    ranked = mix.rank_costs(
        {
            "evidence": [10, 10, 10, 10, 10, 10],
            "filter": [5, 5, 5, 5, 5, 5],
            "graph": [2, 2, 2, 2, 2, 2],
        }
    )
    assert [row["path"] for row in ranked] == ["evidence", "filter", "graph"]
    assert [row["total_ns"] for row in ranked] == [60, 30, 12]
    assert mix.prefix_at_least(ranked, 0.8) == ["evidence", "filter"]
    assert ranked[-1]["cumulative_fraction"] == 1.0


def test_s01_selection_spans_full_block_and_rejects_false_result() -> None:
    warm = [{"latency_ns": n + 1, "semantic_ok": True} for n in range(1000)]
    assert [item["latency_ns"] for item in mix.select_s01(warm)] == list(
        range(1, 1000, 10)
    )
    warm[990]["semantic_ok"] = False
    with pytest.raises(ValueError, match="semantic"):
        mix.select_s01(warm)
    with pytest.raises(ValueError, match="count"):
        mix.select_s01(warm[:-1])


def test_cost_rank_refuses_missing_and_nonpositive_attempts() -> None:
    with pytest.raises(ValueError, match="count"):
        mix.rank_costs({"a": [1, 2], "b": [3]})
    with pytest.raises(ValueError, match="positive"):
        mix.rank_costs({"a": [0], "b": [1]})
