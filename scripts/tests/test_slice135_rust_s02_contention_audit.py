"""Rust SDK contention audit checks actual call overlap and persisted state."""

from __future__ import annotations

from pathlib import Path
import sqlite3
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_rust_s02_contention_audit as audit  # noqa: E402


def test_timeline_rejects_a_false_overlap_claim() -> None:
    writers = [
        {"logical_id": f"contend-{index:02d}", "start_ns": index * 20,
         "end_ns": index * 20 + 10}
        for index in range(8)
    ]
    readers = [
        {"start_ns": index * 20 + 5, "end_ns": index * 20 + 15,
         "anchor_ok": True, "text_ok": True, "vector_ok": True, "graph_ok": True}
        for index in range(8)
    ]
    assert audit.validate_timeline(writers, readers, claimed_overlap=8) == 8
    with pytest.raises(ValueError, match="overlap"):
        audit.validate_timeline(writers, readers, claimed_overlap=0)
    with pytest.raises(ValueError, match="reader assertion"):
        audit.validate_timeline(writers, [{**readers[0], "graph_ok": False}, *readers[1:]],
                                claimed_overlap=8)


def test_reopened_oracle_rejects_remaining_erased_rows(tmp_path: Path) -> None:
    database = tmp_path / "state.sqlite"
    with sqlite3.connect(database) as connection:
        connection.executescript("""
            CREATE TABLE canonical_nodes (source_id TEXT, logical_id TEXT, body TEXT);
            CREATE TABLE canonical_edges (source_id TEXT);
            INSERT INTO canonical_nodes VALUES ('slice135-rust-corpus', 'A',
                '{"summary": "harbor lantern ferry timetable brightblue"}');
            INSERT INTO canonical_nodes VALUES ('slice135-rust-contended', 'contend-00', 'writer');
        """)
    with pytest.raises(ValueError, match="erased"):
        audit.validate_reopened_state(database, expected_corpus_nodes=1)
    with sqlite3.connect(database) as connection:
        connection.execute("DELETE FROM canonical_nodes WHERE source_id='slice135-rust-contended'")
    assert audit.validate_reopened_state(database, expected_corpus_nodes=1)["integrity_check"] == "ok"


def test_receipt_requires_source_semantics_and_real_reopen(tmp_path: Path) -> None:
    database = tmp_path / "complete.sqlite"
    with sqlite3.connect(database) as connection:
        connection.executescript("""
            CREATE TABLE canonical_nodes (source_id TEXT, logical_id TEXT, body TEXT);
            CREATE TABLE canonical_edges (source_id TEXT);
            INSERT INTO canonical_nodes VALUES ('slice135-rust-corpus', 'A',
                '{"summary": "harbor lantern ferry timetable brightblue"}');
        """)
    writers = [
        {"logical_id": f"contend-{index:02d}", "start_ns": index * 20,
         "end_ns": index * 20 + 10}
        for index in range(8)
    ]
    readers = [
        {"start_ns": index * 20 + 5, "end_ns": index * 20 + 15,
         "anchor_ok": True, "text_ok": True, "vector_ok": True, "graph_ok": True}
        for index in range(8)
    ]
    receipt = {
        "schema_version": 1, "status": "RUST_S02_CONTENTION_FUNCTIONAL_OK",
        "source_sha": audit.SOURCE_SHA, "whole_product_ns": 1_000_000,
        "timeline": {"writer_calls": writers, "reader_calls": readers},
        "observed": {
            "writer_ids": [f"contend-{index:02d}" for index in range(8)],
            "reader_cycles": 8, "overlap_cycles": 8, "readiness": "ready",
            "evidence_resolved": True,
            "erasure": {"graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8},
            "reopened_anchor_retained": True, "reopened_erased_absent": True,
        },
    }
    accepted = audit.validate_receipt(receipt, database, expected_corpus_nodes=1)
    assert accepted["actual_overlap_cycles"] == 8
    with pytest.raises(ValueError, match="source"):
        audit.validate_receipt({**receipt, "source_sha": "0" * 40}, database,
                               expected_corpus_nodes=1)
    with pytest.raises(ValueError, match="semantic"):
        audit.validate_receipt({**receipt, "observed": {
            **receipt["observed"], "evidence_resolved": False,
        }}, database, expected_corpus_nodes=1)
    with sqlite3.connect(database) as connection:
        connection.execute("UPDATE canonical_nodes SET body='changed' WHERE logical_id='A'")
    with pytest.raises(ValueError, match="retained anchor"):
        audit.validate_receipt(receipt, database, expected_corpus_nodes=1)
