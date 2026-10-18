"""Tests for the 512-wordpiece cut arithmetic used by the sizing study."""

from __future__ import annotations

import pytest

from cut import KEPT_CONTENT_TOKENS, CutInfo, classify_span, cut_info


def _offsets(n: int, width: int = 4) -> list[tuple[int, int]]:
    # Token i covers chars [i*width, i*width + width - 1): one trailing space each.
    return [(i * width, i * width + width - 1) for i in range(n)]


def test_kept_tokens_excludes_cls_and_sep():
    assert KEPT_CONTENT_TOKENS == 510


def test_short_record_has_no_cut():
    body = "x" * 100
    info = cut_info(body, _offsets(10))
    assert info == CutInfo(tokens=10, cut_char=None, bytes_total=100, bytes_past=0)


def test_exactly_510_tokens_has_no_cut():
    offs = _offsets(510)
    body = "x" * (offs[-1][1] + 1)
    assert cut_info(body, offs).cut_char is None


def test_511_tokens_cuts_after_token_510():
    offs = _offsets(511)
    body = "x" * (offs[-1][1] + 1)
    info = cut_info(body, offs)
    assert info.cut_char == offs[509][1]
    assert info.bytes_past == len(body) - offs[509][1]


def test_bytes_past_counts_utf8_bytes():
    body = "ab" + "é" * 3
    offs = [(i, i + 1) for i in range(5)]
    info = cut_info(body, offs, keep=2)
    assert info.cut_char == 2
    assert info.bytes_past == 6


@pytest.mark.parametrize(
    ("span", "expected"),
    [((0, 10), "before"), ((95, 105), "straddles"), ((100, 120), "after"), ((150, 160), "after")],
)
def test_classify_span(span, expected):
    assert classify_span(span, cut_char=100) == expected


def test_classify_span_without_cut_is_before():
    assert classify_span((5000, 5100), cut_char=None) == "before"
