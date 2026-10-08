"""Phase 2 audit checks retained real SQLite state, not only runner JSON."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sqlite3

import pytest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_contract_audit.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_contract_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)

FIXTURE = {
    "corpus": [
        {"kind": "note", "body": "alpha"},
        {"kind": "doc", "body": "beta"},
    ],
    "queries": [{"query": "alpha", "expected_bodies": ["alpha"]}],
}


def _database(path: Path) -> None:
    with sqlite3.connect(path) as connection:
        connection.execute(
            "CREATE TABLE canonical_nodes (logical_id TEXT, kind TEXT, body TEXT, "
            "source_id TEXT, state TEXT, superseded_at INTEGER)"
        )
        connection.executemany(
            "INSERT INTO canonical_nodes VALUES (?, ?, ?, ?, ?, ?)",
            [
                ("phase2-x1-0", "note", "alpha", "source", "active", None),
                ("phase2-x1-1", "doc", "beta", "source", "active", None),
            ],
        )


def test_retained_database_matches_seed_and_integrity(tmp_path: Path) -> None:
    path = tmp_path / "contract.sqlite"
    _database(path)
    assert AUDIT.audit_database(
        path, FIXTURE, source_id="source", id_prefix="phase2-x1-"
    ) == {"canonical_rows": 2, "integrity": "ok"}


@pytest.mark.parametrize(
    "sql",
    [
        "UPDATE canonical_nodes SET body='wrong' WHERE logical_id='phase2-x1-0'",
        "DELETE FROM canonical_nodes WHERE logical_id='phase2-x1-1'",
        "UPDATE canonical_nodes SET state='deleted' WHERE logical_id='phase2-x1-0'",
    ],
)
def test_retained_database_rejects_wrong_persisted_state(tmp_path: Path, sql: str) -> None:
    path = tmp_path / "contract.sqlite"
    _database(path)
    with sqlite3.connect(path) as connection:
        connection.execute(sql)
    with pytest.raises(ValueError, match="persisted"):
        AUDIT.audit_database(path, FIXTURE, source_id="source", id_prefix="phase2-x1-")
