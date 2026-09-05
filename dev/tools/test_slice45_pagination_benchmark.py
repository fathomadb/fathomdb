"""Contract tests for the Slice 45 measurement process plan."""

from __future__ import annotations

import importlib.util
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("slice45_pagination_benchmark.py")
SPEC = importlib.util.spec_from_file_location("slice45_pagination_benchmark", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
BENCHMARK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCHMARK)


def test_every_steady_pair_has_a_separate_balanced_process_plan() -> None:
    plan = BENCHMARK.steady_invocations(2)

    assert len(plan) == 2 * len(BENCHMARK.LATENCY_PAIRS)
    for pair in BENCHMARK.LATENCY_PAIRS:
        assignments = [item for item in plan if item[1] == pair]
        assert [item[0] for item in assignments] == [0, 1]
        assert {item[2] for item in assignments} == {False, True}

    for repetition in (0, 1):
        flags = {item[2] for item in plan if item[0] == repetition}
        assert flags == {False, True}


def test_rss_controls_do_not_require_frozen_setup() -> None:
    assert not BENCHMARK.rss_requires_frozen_fixture("exact_page")
    assert not BENCHMARK.rss_requires_frozen_fixture("current_state")

    for arm in (
        "frozen_page",
        "mint_plus_page",
        "continuation_page",
        "frozen_state",
    ):
        assert BENCHMARK.rss_requires_frozen_fixture(arm)
