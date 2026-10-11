"""S02-L independent audit must catch changed identity and state."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_audit as audit  # noqa: E402


def observation() -> dict:
    return {
        "schema_version": 1,
        "source_sha": "a" * 40,
        "runner_sha256": "b" * 64,
        "artifact": {"wheel_sha256": "c" * 64, "native_sha256": "d" * 64},
        "semantic_ok": True,
        "reopen_ok": True,
        "vector_dimension": 384,
        "vector_nonzero": True,
        "open_embed_ns": 100,
        "close_ns": 50,
        "embedder": {"name": "fathomdb-bge-small-en-v1.5", "dimension": 384},
        "host": {"embed_device": "cpu"},
        **{
            name: {"rss_kib": 100, "pss_kib": 90, "private_kib": 80}
            for name in ("before", "opened", "closed", "closed_idle", "dropped")
        },
    }


def test_independent_sample_rejects_state_or_identity_mutations() -> None:
    raw = observation()
    manifest = {
        "source_sha": "a" * 40,
        "runner_sha256": "b" * 64,
        "wheel_sha256": "c" * 64,
        "native_sha256": "d" * 64,
        "embed_device": "cpu",
        "embedder_name": "fathomdb-bge-small-en-v1.5",
    }
    assert audit.check_sample(raw, manifest) == (50, 0)
    for field, replacement in (
        ("source_sha", "0" * 40),
        ("runner_sha256", "0" * 64),
        ("reopen_ok", False),
        ("vector_nonzero", False),
    ):
        bad = deepcopy(raw)
        bad[field] = replacement
        with pytest.raises(ValueError):
            audit.check_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["closed_idle"]["pss_kib"] = -1
    with pytest.raises(ValueError):
        audit.check_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["artifact"]["native_sha256"] = "0" * 64
    with pytest.raises(ValueError):
        audit.check_sample(bad, manifest)


def test_nearest_rank_and_pair_delta() -> None:
    assert audit.nearest_rank([2, 10, 4, 8], 0.5) == 4
    assert audit.nearest_rank([2, 10, 4, 8], 0.95) == 10
    assert audit.pair_delta(100, 125) == 25.0
    with pytest.raises(ValueError):
        audit.pair_delta(0, 125)
