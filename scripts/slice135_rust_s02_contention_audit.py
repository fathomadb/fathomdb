#!/usr/bin/env python3
"""Inspect Rust SDK shared-engine contention and reopened real-database state."""

from __future__ import annotations

from pathlib import Path
import sqlite3


WRITER_IDS = [f"contend-{index:02d}" for index in range(8)]
READER_CHECKS = ("anchor_ok", "text_ok", "vector_ok", "graph_ok")
SOURCE_SHA = "3f29d649d0213e595c0dab251a449d92fd625792"
ANCHOR_BODY = '{"summary": "harbor lantern ferry timetable brightblue"}'
EXPECTED_OBSERVED = {
    "writer_ids": WRITER_IDS,
    "reader_cycles": 8,
    "readiness": "ready",
    "evidence_resolved": True,
    "erasure": {"graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8},
    "reopened_anchor_retained": True,
    "reopened_erased_absent": True,
}


def validate_timeline(writer: list[dict], reader: list[dict], *, claimed_overlap: int) -> int:
    """Reject missing calls, failed reads and a fabricated overlap count."""
    if [row.get("logical_id") for row in writer] != WRITER_IDS or len(reader) != 8:
        raise ValueError("Rust contention call count or writer IDs changed")
    for calls in (writer, reader):
        if any(
            type(row.get("start_ns")) is not int
            or type(row.get("end_ns")) is not int
            or row["start_ns"] >= row["end_ns"]
            for row in calls
        ):
            raise ValueError("Rust contention call interval invalid")
        if any(left["start_ns"] >= right["start_ns"] for left, right in zip(calls, calls[1:])):
            raise ValueError("Rust contention call order invalid")
    if any(any(row.get(check) is not True for check in READER_CHECKS) for row in reader):
        raise ValueError("Rust contention reader assertion failed")
    actual = sum(
        any(write["start_ns"] < read["end_ns"] and write["end_ns"] > read["start_ns"]
            for write in writer)
        for read in reader
    )
    if actual < 1 or claimed_overlap != actual:
        raise ValueError("Rust contention overlap claim differs from intervals")
    return actual


def validate_reopened_state(database: Path, *, expected_corpus_nodes: int) -> dict:
    """Check retained and erased canonical rows with a new SQLite connection."""
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        anchor = connection.execute(
            "SELECT body FROM canonical_nodes WHERE source_id='slice135-rust-corpus' "
            "AND logical_id='A'"
        ).fetchone()
        state = {
            "corpus_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-corpus'"
            ).fetchone()[0],
            "graph_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-graph'"
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE source_id='slice135-rust-graph'"
            ).fetchone()[0],
            "contended_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-rust-contended'"
            ).fetchone()[0],
            "integrity_check": connection.execute("PRAGMA integrity_check").fetchone()[0],
        }
    if anchor != (ANCHOR_BODY,):
        raise ValueError("Rust contention retained anchor changed")
    if state["corpus_nodes"] != expected_corpus_nodes:
        raise ValueError("Rust contention retained corpus changed")
    if any(state[key] for key in ("graph_nodes", "graph_edges", "contended_nodes")):
        raise ValueError("Rust contention erased rows remain")
    if state["integrity_check"] != "ok":
        raise ValueError("Rust contention SQLite integrity changed")
    return state


def validate_receipt(receipt: dict, database: Path, *, expected_corpus_nodes: int = 32) -> dict:
    """Match declared source, timed semantics, overlap and independent state."""
    if (
        receipt.get("schema_version") != 1
        or receipt.get("status") != "RUST_S02_CONTENTION_FUNCTIONAL_OK"
        or receipt.get("source_sha") != SOURCE_SHA
    ):
        raise ValueError("Rust contention source or receipt schema invalid")
    timeline = receipt.get("timeline", {})
    observed = receipt.get("observed", {})
    expected = {**EXPECTED_OBSERVED, "overlap_cycles": observed.get("overlap_cycles")}
    if observed != expected:
        raise ValueError("Rust contention semantic claim changed")
    actual_overlap = validate_timeline(
        timeline.get("writer_calls", []), timeline.get("reader_calls", []),
        claimed_overlap=observed["overlap_cycles"],
    )
    elapsed = receipt.get("whole_product_ns")
    latest = max(row["end_ns"] for row in [
        *timeline["writer_calls"], *timeline["reader_calls"]
    ])
    if type(elapsed) is not int or not latest <= elapsed < 120_000_000_000:
        raise ValueError("Rust contention whole timer invalid")
    state = validate_reopened_state(database, expected_corpus_nodes=expected_corpus_nodes)
    return {
        "status": "ACCEPTED_RUST_S02_CONTENTION",
        "source_sha": SOURCE_SHA,
        "whole_product_ns": elapsed,
        "actual_overlap_cycles": actual_overlap,
        "reopened_state": state,
    }
