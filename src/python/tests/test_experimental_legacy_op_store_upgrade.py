"""Integration contract for the unsupported legacy op-store copy tool.

The fixture models only the pre-rewrite table shape reported by Memex issue
0007.  It never opens or mutates a user database.
"""

from __future__ import annotations

import hashlib
import os
import sqlite3
import subprocess
import sys
from pathlib import Path


_ROOT = Path(__file__).resolve().parents[3]
_TOOL = _ROOT / "scripts" / "experimental" / "upgrade_legacy_op_store.py"
_LEGACY_COLUMNS = [
    "id",
    "collection_name",
    "record_key",
    "op_kind",
    "payload_json",
    "source_ref",
    "created_at",
    "mutation_order",
]


def _tool_environment() -> dict[str, str]:
    """Run the standalone tool as a consumer would, without test-path shadowing."""
    environment = dict(os.environ)
    environment.pop("PYTHONPATH", None)
    return environment


def _legacy_database(path: Path, *, unexpected_column: bool = False) -> None:
    extra = ", unexpected TEXT" if unexpected_column else ""
    with sqlite3.connect(path) as connection:
        connection.execute(
            """
            CREATE TABLE operational_mutations(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_name TEXT NOT NULL,
                record_key TEXT NOT NULL,
                op_kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                source_ref TEXT,
                created_at INTEGER NOT NULL,
                mutation_order INTEGER NOT NULL%s
            )
            """ % extra,
        )
        connection.execute(
            """
            INSERT INTO operational_mutations(
                collection_name, record_key, op_kind, payload_json,
                source_ref, created_at, mutation_order
            ) VALUES ('schema_migrations', '0001', 'append', '{"revision":"0001"}',
                      'memex', 1, 1)
            """,
        )


def _legacy_wal_database(path: Path) -> sqlite3.Connection:
    """Create an open legacy WAL fixture whose source and sidecars are witnessed."""
    connection = sqlite3.connect(path)
    connection.execute("PRAGMA journal_mode = WAL")
    connection.execute("PRAGMA wal_autocheckpoint = 0")
    connection.execute(
        """
        CREATE TABLE operational_mutations(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_name TEXT NOT NULL,
            record_key TEXT NOT NULL,
            op_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            source_ref TEXT,
            created_at INTEGER NOT NULL,
            mutation_order INTEGER NOT NULL
        )
        """,
    )
    connection.execute(
        """
        INSERT INTO operational_mutations(
            collection_name, record_key, op_kind, payload_json,
            source_ref, created_at, mutation_order
        ) VALUES ('schema_migrations', '0001', 'append', '{"revision":"0001"}',
                  'memex', 1, 1)
        """,
    )
    connection.commit()
    assert Path(f"{path}-wal").is_file()
    assert Path(f"{path}-shm").is_file()
    return connection


def _columns(path: Path) -> list[str]:
    with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
        return [row[1] for row in connection.execute("PRAGMA table_info(operational_mutations)")]


def _digest(path: Path) -> str:
    """Return a byte-level input witness for the no-write invariant."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _database_witness(path: Path) -> dict[str, str]:
    """Hash the main database and both SQLite WAL sidecars without opening SQLite."""
    files = (path, Path(f"{path}-wal"), Path(f"{path}-shm"))
    assert all(file.is_file() for file in files)
    return {file.name: _digest(file) for file in files}


def test_copy_only_tool_upgrades_exact_legacy_shape_and_opens_with_fathomdb(tmp_path: Path) -> None:
    """The input stays byte-for-byte shape-stable while the output opens and reads."""
    source = tmp_path / "legacy.sqlite"
    output = tmp_path / "upgraded.sqlite"
    _legacy_database(source)
    source_digest = _digest(source)

    completed = subprocess.run(
        [sys.executable, str(_TOOL), "--input", str(source), "--output", str(output)],
        text=True,
        capture_output=True,
        check=False,
        env=_tool_environment(),
    )

    assert completed.returncode == 0, completed.stderr
    assert "NOT OFFICIALLY SUPPORTED" in completed.stderr
    assert _digest(source) == source_digest
    assert _columns(source) == _LEGACY_COLUMNS
    assert _columns(output) == [*_LEGACY_COLUMNS, "schema_id", "write_cursor"]
    with sqlite3.connect(f"file:{output}?mode=ro", uri=True) as connection:
        assert connection.execute("PRAGMA integrity_check").fetchone() == ("ok",)
        assert connection.execute("SELECT schema_id, write_cursor FROM operational_mutations").fetchone() == (
            None,
            0,
        )


def test_copy_only_tool_refuses_a_nearby_unknown_schema(tmp_path: Path) -> None:
    """An unfamiliar table is refused before any output copy is created."""
    source = tmp_path / "unknown.sqlite"
    output = tmp_path / "must-not-exist.sqlite"
    _legacy_database(source, unexpected_column=True)

    completed = subprocess.run(
        [sys.executable, str(_TOOL), "--input", str(source), "--output", str(output)],
        text=True,
        capture_output=True,
        check=False,
        env=_tool_environment(),
    )

    assert completed.returncode != 0
    assert "refusing unknown operational_mutations schema" in completed.stderr
    assert not output.exists()
    assert _columns(source) == [*_LEGACY_COLUMNS, "unexpected"]


def test_copy_only_tool_never_opens_or_mutates_a_live_wal_input(tmp_path: Path) -> None:
    """A live WAL source and both sidecars remain byte-identical after success."""
    source = tmp_path / "legacy-live.sqlite"
    output = tmp_path / "upgraded.sqlite"
    writer = _legacy_wal_database(source)
    before = _database_witness(source)
    try:
        completed = subprocess.run(
            [sys.executable, str(_TOOL), "--input", str(source), "--output", str(output)],
            text=True,
            capture_output=True,
            check=False,
            env=_tool_environment(),
        )
        after = _database_witness(source)
    finally:
        writer.close()

    assert completed.returncode == 0, completed.stderr
    assert after == before
    assert _columns(output) == [*_LEGACY_COLUMNS, "schema_id", "write_cursor"]
