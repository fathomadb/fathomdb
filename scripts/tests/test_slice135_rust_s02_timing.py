"""Exercise the external Rust SDK S02 timing consumer against a real database."""

from __future__ import annotations

import json
import os
from pathlib import Path
import sqlite3
import subprocess

import pytest


STAGES = {
    "open", "write", "configure_projection", "drain", "text",
    "vector_lexical_control", "vector", "hybrid", "evidence_search",
    "evidence_resolve", "graph_expand", "graph_target_resolve",
    "graph_edge_resolve", "graph_neighbors", "erase", "erase_again",
    "close", "reopen", "reopened_close",
}


def test_timed_rust_sdk_s02_sequence_has_product_boundary_and_reopened_state(
    tmp_path: Path,
) -> None:
    executable = os.environ.get("SLICE135_RUST_S02_BINARY")
    if executable is None:
        pytest.skip("set SLICE135_RUST_S02_BINARY to an exact-source consumer binary")
    database = tmp_path / "s02.sqlite"
    result = subprocess.run(
        [executable, str(database)], capture_output=True, text=True,
        check=True, timeout=120,
    )
    lines = result.stdout.splitlines()
    assert len(lines) == 1 and lines[0].startswith("SLICE135_RUST_S02 ")
    raw = json.loads(lines[0].split(" ", 1)[1])
    assert raw["schema_version"] == 1
    assert raw["status"] == "TIMED_RUST_SDK_S02_FUNCTIONAL_OK"
    assert raw["source_sha"] == "3f29d649d0213e595c0dab251a449d92fd625792"
    assert type(raw["whole_product_ns"]) is int and raw["whole_product_ns"] > 0
    assert set(raw["stage_ns"]) == STAGES
    assert all(type(value) is int and value > 0 for value in raw["stage_ns"].values())
    assert raw["whole_product_ns"] >= sum(raw["stage_ns"].values())
    assert raw["observed"] == {
        "corpus_nodes": 32,
        "erased_graph_nodes": 3,
        "erased_graph_edges": 1,
        "reopened_graph_absent": True,
        "reopened_anchor_retained": True,
        "evidence_resolved": True,
        "projection_ready": True,
    }
    with sqlite3.connect(database) as connection:
        assert connection.execute("PRAGMA integrity_check").fetchone() == ("ok",)
        assert connection.execute(
            "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-corpus'"
        ).fetchone() == (32,)
        assert connection.execute(
            "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-graph'"
        ).fetchone() == (0,)
        assert connection.execute(
            "SELECT COUNT(*) FROM canonical_edges WHERE source_id='slice135-rust-graph'"
        ).fetchone() == (0,)
