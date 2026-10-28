"""Tests for the preregistered effect arithmetic in probe.summarize."""

from __future__ import annotations

from probe import summarize


def _row(stratum: str, text: int, hybrid: int, vector: int) -> dict:
    rec = {"stratum": stratum, "measured": True, "hybrid100_ms": 1.0}
    for arm, v in (("text", text), ("hybrid", hybrid), ("vector", vector)):
        rec[f"{arm}@5"] = rec[f"{arm}@10"] = v
    return rec


def test_effect_subtracts_the_text_gap():
    # Vector finds every B question and no P question; text is equally good on both.
    rows = [_row("B", 1, 1, 1) for _ in range(20)] + [_row("P", 1, 1, 0) for _ in range(20)]
    s = summarize(rows)
    assert s["E(vector)"]["point"] == 1.0
    assert s["E(hybrid)"]["point"] == 0.0
    assert s["rule"] == {"vector_failure_shown": True, "product_failure_shown": False}


def test_difficulty_gap_shared_with_text_is_not_a_truncation_effect():
    rows = [_row("B", 1, 1, 1) for _ in range(20)] + [_row("P", 0, 0, 0) for _ in range(20)]
    s = summarize(rows)
    assert s["E(vector)"]["point"] == 0.0
    assert s["rule"]["vector_failure_shown"] is False


def test_excluded_strata_are_counted_not_scored():
    rows = [_row("B", 1, 1, 1), _row("P", 1, 1, 1), {"stratum": "mixed"}, {"stratum": "none"}]
    s = summarize(rows)
    assert s["n"] == {"B": 1, "P": 1}
    assert s["excluded"] == {"mixed": 1, "none": 1}
