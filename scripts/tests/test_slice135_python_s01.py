"""Guard the Slice 135 installed-Python S01 workload against vacuous results."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace

import pytest


RUNNER = Path(__file__).resolve().parents[1] / "slice135_python_s01.py"
SPEC = importlib.util.spec_from_file_location("slice135_python_s01", RUNNER)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_corpus_has_exact_size_and_stable_distinct_ids() -> None:
    rows = MODULE.make_corpus(32)
    assert len(rows) == 32
    assert [row["logical_id"] for row in rows[:2]] == ["A", "B"]
    assert len({row["logical_id"] for row in rows}) == 32
    assert MODULE.make_corpus(32) == rows
    assert len(MODULE.make_corpus(256)) == 256


def test_result_guard_rejects_wrong_text_hit_and_missing_vector_branch() -> None:
    good_text = SimpleNamespace(
        results=[
            SimpleNamespace(
                id=SimpleNamespace(space="logical", value="A"), branch="text"
            )
        ]
    )
    MODULE.assert_result("text", good_text, {"A", "B"})
    wrong_text = SimpleNamespace(
        results=[
            SimpleNamespace(
                id=SimpleNamespace(space="logical", value="B"), branch="text"
            )
        ]
    )
    with pytest.raises(AssertionError, match="text result"):
        MODULE.assert_result("text", wrong_text, {"A", "B"})
    with pytest.raises(AssertionError, match="vector branch"):
        MODULE.assert_result("vector", good_text, {"A", "B"})


def test_result_guard_rejects_unknown_id_and_empty_hybrid() -> None:
    unknown = SimpleNamespace(
        results=[
            SimpleNamespace(
                id=SimpleNamespace(space="logical", value="X"), branch="vector"
            )
        ]
    )
    with pytest.raises(AssertionError, match="unknown id"):
        MODULE.assert_result("vector", unknown, {"A", "B"})
    with pytest.raises(AssertionError, match="empty result"):
        MODULE.assert_result("hybrid", SimpleNamespace(results=[]), {"A", "B"})


def test_result_guard_remains_active_under_optimized_python() -> None:
    code = (
        "import importlib.util, sys, types; "
        "spec=importlib.util.spec_from_file_location('s01', sys.argv[1]); "
        "module=importlib.util.module_from_spec(spec); "
        "spec.loader.exec_module(module); "
        "bad=types.SimpleNamespace(results=[types.SimpleNamespace("
        "id=types.SimpleNamespace(space='logical', value='B'), branch='text')]); "
        "module.assert_result('text', bad, {'A', 'B'})"
    )
    result = subprocess.run(
        [sys.executable, "-O", "-c", code, str(RUNNER)],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode != 0
    assert "text result" in result.stderr
