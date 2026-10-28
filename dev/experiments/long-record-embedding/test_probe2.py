"""Tests for the P2 position intervention and its paired estimator."""

from __future__ import annotations

from probe2 import paired_summary, rotate


def test_rotate_to_front_keeps_relative_order():
    assert rotate(["a", "G1", "b", "G2", "c"], {1, 3}, to_front=True) == ["G1", "G2", "a", "b", "c"]


def test_rotate_to_end_keeps_relative_order():
    assert rotate(["a", "G1", "b", "G2", "c"], {1, 3}, to_front=False) == ["a", "b", "c", "G1", "G2"]


def test_rotate_preserves_the_multiset_of_lines():
    lines = ["x", "y", "G", "y"]
    assert sorted(rotate(lines, {2}, to_front=True)) == sorted(lines)


def _q(stratum: str, vis: dict, hid: dict, qtype: str = "t") -> dict:
    return {"stratum": stratum, "type": qtype, "measured": True, "visible": vis, "hidden": hid}


def test_paired_effect_is_visible_minus_hidden():
    same = {"text@10": 1, "hybrid@10": 1, "vector@10": 1}
    lost = {"text@10": 1, "hybrid@10": 0, "vector@10": 0}
    rows = [_q("P", same, lost) for _ in range(10)] + [_q("B", same, same) for _ in range(10)]
    s = paired_summary(rows)
    assert s["D(vector)"]["point"] == 0.5
    assert s["D(hybrid)"]["point"] == 0.5
    assert s["D(text)"]["point"] == 0.0
    assert s["validity"]["ok"] is True
    assert s["rule"]["vector_failure_shown"] is True


def test_text_disagreement_invalidates_the_run():
    vis = {"text@10": 1, "hybrid@10": 1, "vector@10": 1}
    hid = {"text@10": 0, "hybrid@10": 0, "vector@10": 0}
    s = paired_summary([_q("P", vis, hid) for _ in range(10)])
    assert s["validity"]["ok"] is False
    assert s["rule"] is None
