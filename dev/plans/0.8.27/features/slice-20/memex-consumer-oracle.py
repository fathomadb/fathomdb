"""Frozen Memex 0.6.0 correction-to-erasure acceptance for Slice 20.

Run from the Memex checkout with its ``src`` and ``tests`` directories plus an
isolated FathomDB candidate wheel on ``PYTHONPATH``. The fixture import is
intentional: setup remains owned by Memex; only the formerly failing oracle is
replaced here.
"""

from __future__ import annotations

import json
from pathlib import Path
import sqlite3

import fathomdb
from memex.memory_mutation import memory_scope_sha256
from memex.memory_runtime import MemoryRuntime
from memex.memory_semantics import EraseSource
from test_memory_semantics_r1 import SCOPE, _closed_world


PROJECTION_TABLES = (
    ("search_index", "write_cursor"),
    ("search_index_v2", "write_cursor"),
    ("search_index_edges", "write_cursor"),
    # The stdlib SQLite connection cannot load the vec0 extension. Its
    # authoritative owner rows are covered through _fathomdb_vector_rows.
    ("_fathomdb_vector_rows", "write_cursor"),
    ("_fathomdb_projection_terminal", "write_cursor"),
    ("canonical_attributes", "write_cursor"),
    ("property_search_index", "write_cursor"),
)


def _inventory(database: Path, source_ref: str) -> tuple[list[int], int, int, int]:
    with sqlite3.connect(database) as connection:
        node_cursors = [
            row[0]
            for row in connection.execute(
                "SELECT write_cursor FROM canonical_nodes WHERE source_id=?",
                (source_ref,),
            )
        ]
        edge_cursors = [
            row[0]
            for row in connection.execute(
                "SELECT write_cursor FROM canonical_edges WHERE source_id=?",
                (source_ref,),
            )
        ]
        cursors = node_cursors + edge_cursors
        projections = sum(
            connection.execute(
                f"SELECT COUNT(*) FROM {table} WHERE {column}=?", (cursor,)
            ).fetchone()[0]
            for cursor in cursors
            for table, column in PROJECTION_TABLES
        )
    return cursors, len(node_cursors), len(edge_cursors), projections


def test_memex_correction_then_native_erasure(tmp_path: Path) -> None:
    database = tmp_path / "r1-db" / "memory.db"
    physical_source = f"mms1:{memory_scope_sha256(SCOPE)}:source-1"

    with MemoryRuntime() as runtime:
        world, _ = _closed_world(runtime, tmp_path)
        cursors, nodes, edges, projections = _inventory(database, physical_source)
        assert nodes > 0 and projections > 0

        receipt = world.admin.erase_source(
            world._cmd(EraseSource, "erase-corrected", world.source.source_id)
        )
        assert receipt.outcome == "erased"
        assert receipt.erased_source_id == world.source.source_id
        assert receipt.reason_codes == ()
        assert world.visible("after") == ([], [], [])

    # Independent raw reopen: prove physical absence and exact accepted proof
    # rows before an idempotent retry can append its own zero-count audit.
    with sqlite3.connect(database) as connection:
        assert connection.execute(
            "SELECT COUNT(*) FROM canonical_nodes WHERE source_id=?", (physical_source,)
        ).fetchone() == (0,)
        assert connection.execute(
            "SELECT COUNT(*) FROM canonical_edges WHERE source_id=?", (physical_source,)
        ).fetchone() == (0,)
        for cursor in cursors:
            for table, column in PROJECTION_TABLES:
                assert connection.execute(
                    f"SELECT COUNT(*) FROM {table} WHERE {column}=?", (cursor,)
                ).fetchone() == (0,)

        audit_rows = connection.execute(
            "SELECT payload_json FROM operational_mutations "
            "WHERE collection_name='excise_source_audit' AND record_key=?",
            (physical_source,),
        ).fetchall()
        assert len(audit_rows) == 1
        payload = json.loads(audit_rows[0][0])
        assert set(payload) == {
            "edges_excised",
            "excised_at",
            "nodes_excised",
            "projections_invalidated",
            "source_id",
        }
        assert payload["source_id"] == physical_source
        assert payload["nodes_excised"] == nodes
        assert payload["edges_excised"] == edges
        assert payload["projections_invalidated"] == projections
        assert payload["excised_at"] > 0
        proof_rows = connection.execute(
            "SELECT root_kind,root_value,cause,phase "
            "FROM _fathomdb_dependency_closures WHERE cause='source_erased'"
        ).fetchall()
        assert proof_rows == [
            ("source_bucket", physical_source, "source_erased", "complete")
        ]

    engine = fathomdb.Engine.open(str(database))
    retry = engine.erase_source(physical_source)
    assert (
        retry.nodes_excised,
        retry.edges_excised,
        retry.projections_invalidated,
    ) == (0, 0, 0)
    engine.close()
