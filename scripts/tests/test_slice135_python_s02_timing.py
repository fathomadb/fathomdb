"""Guard the S02 product timer against swallowing validation work."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_timing as timing  # noqa: E402


def test_product_timer_ends_before_independent_state_check() -> None:
    events: list[str] = []
    times = iter((10, 40, 100))

    def product() -> dict[str, str]:
        events.append("product")
        return {"result": "materialized"}

    def verify(result: dict[str, str]) -> str:
        assert result == {"result": "materialized"}
        events.append("verify")
        return "valid"

    result, check, product_ns, verification_ns = timing.measure_product_then_verify(
        product, verify, clock=lambda: next(times)
    )
    assert (result, check, product_ns, verification_ns) == (
        {"result": "materialized"},
        "valid",
        30,
        60,
    )
    assert events == ["product", "verify"]


def test_invalid_product_output_cannot_become_valid_timing_sample() -> None:
    times = iter((1, 2))

    with pytest.raises(ValueError, match="reopened source remains"):
        timing.measure_product_then_verify(
            lambda: {"after_reopen": {"source_absent": False}},
            lambda result: timing.require_reopened_source_absent(result),
            clock=lambda: next(times),
        )
