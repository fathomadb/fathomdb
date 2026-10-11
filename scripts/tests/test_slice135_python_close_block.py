"""S02-L block guards reject altered lifecycle evidence."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_block as block  # noqa: E402


def fixture() -> dict:
    return {
        "schema_version": 1,
        "source_sha": "a" * 40,
        "runner_sha256": "b" * 64,
        "artifact": {"wheel_sha256": "c" * 64, "native_sha256": "d" * 64},
        "semantic_ok": True,
        "reopen_ok": True,
        "vector_dimension": 384,
        "vector_nonzero": True,
        "close_ns": 1_000,
        "open_embed_ns": 10_000,
        "before": {"rss_kib": 10, "pss_kib": 9, "private_kib": 8},
        "opened": {"rss_kib": 20, "pss_kib": 19, "private_kib": 18},
        "closed": {"rss_kib": 15, "pss_kib": 14, "private_kib": 13},
        "closed_idle": {"rss_kib": 15, "pss_kib": 14, "private_kib": 13},
        "dropped": {"rss_kib": 15, "pss_kib": 14, "private_kib": 13},
    }


def test_lifecycle_guard_binds_artifact_runner_and_semantics() -> None:
    raw = fixture()
    manifest = {
        "source_sha": raw["source_sha"],
        "wheel_sha256": raw["artifact"]["wheel_sha256"],
        "native_sha256": raw["artifact"]["native_sha256"],
        "runner_sha256": raw["runner_sha256"],
    }
    assert block.validate_sample(raw, manifest) == raw["close_ns"]
    for field, replacement in (
        ("source_sha", "0" * 40),
        ("runner_sha256", "0" * 64),
        ("vector_nonzero", False),
        ("reopen_ok", False),
    ):
        bad = deepcopy(raw)
        bad[field] = replacement
        with pytest.raises(ValueError):
            block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["artifact"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["closed_idle"]["pss_kib"] = -1
    with pytest.raises(ValueError):
        block.validate_sample(bad, manifest)


def test_block_statistics_keep_all_close_samples() -> None:
    assert block.nearest_rank([2, 10, 4, 8], 0.5) == 4
    assert block.nearest_rank([2, 10, 4, 8], 0.95) == 10
    with pytest.raises(ValueError):
        block.nearest_rank([], 0.5)
