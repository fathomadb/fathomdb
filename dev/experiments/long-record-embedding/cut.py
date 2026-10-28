"""Where bge-small's 512-wordpiece truncation falls inside a record body.

The engine tokenizer truncates to 512 tokens including [CLS] and [SEP]
(`fathomdb-embedder/src/candle_bge.rs:66`), so 510 content wordpieces survive.
"""

from __future__ import annotations

from dataclasses import dataclass

KEPT_CONTENT_TOKENS = 510


@dataclass(frozen=True)
class CutInfo:
    tokens: int
    cut_char: int | None
    bytes_total: int
    bytes_past: int


def cut_info(body: str, offsets: list[tuple[int, int]], keep: int = KEPT_CONTENT_TOKENS) -> CutInfo:
    """`offsets` are the content-token char offsets (no special tokens)."""
    total = len(body.encode("utf-8"))
    if len(offsets) <= keep:
        return CutInfo(len(offsets), None, total, 0)
    cut_char = offsets[keep - 1][1]
    return CutInfo(len(offsets), cut_char, total, len(body[cut_char:].encode("utf-8")))


def classify_span(span: tuple[int, int], cut_char: int | None) -> str:
    """'before' (fully embedded), 'straddles', or 'after' (fully invisible to the vector)."""
    start, end = span
    if cut_char is None or end <= cut_char:
        return "before"
    if start >= cut_char:
        return "after"
    return "straddles"
