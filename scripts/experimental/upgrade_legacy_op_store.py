#!/usr/bin/env python3
"""Copy-only, unsupported upgrade for one pre-0.6 operational-store shape.

This tool exists solely to make the historical ``operational_mutations`` table
reported in Memex issue 0007 readable by the current FathomDB runtime.  It is
not a general database upgrader and deliberately refuses every other schema.
"""

from __future__ import annotations

import argparse
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path


_LEGACY_COLUMNS = (
    "id",
    "collection_name",
    "record_key",
    "op_kind",
    "payload_json",
    "source_ref",
    "created_at",
    "mutation_order",
)
_UNSUPPORTED_WARNING = (
    "NOT OFFICIALLY SUPPORTED: this experimental tool recognizes exactly one "
    "pre-0.6 operational_mutations schema. It never mutates its input."
)


class UpgradeRefused(RuntimeError):
    """The requested input is outside this deliberately narrow tool's contract."""


def _check_integrity(connection: sqlite3.Connection, label: str) -> None:
    """Refuse a database whose SQLite integrity or quick check is not clean."""
    for pragma in ("integrity_check", "quick_check"):
        rows = [str(row[0]) for row in connection.execute(f"PRAGMA {pragma}")]
        if rows != ["ok"]:
            raise UpgradeRefused(f"{label}: PRAGMA {pragma} did not return exactly ok: {rows!r}")


def _operational_mutation_columns(connection: sqlite3.Connection) -> tuple[str, ...]:
    """Return the physical column order of ``operational_mutations``."""
    rows = list(connection.execute("PRAGMA table_info(operational_mutations)"))
    if not rows:
        raise UpgradeRefused("refusing unknown schema: operational_mutations table is absent")
    return tuple(str(row[1]) for row in rows)


def _require_exact_legacy_schema(connection: sqlite3.Connection) -> None:
    """Allow only the Memex-issue-0007 legacy table shape."""
    columns = _operational_mutation_columns(connection)
    if columns != _LEGACY_COLUMNS:
        raise UpgradeRefused(
            "refusing unknown operational_mutations schema: "
            f"expected {list(_LEGACY_COLUMNS)!r}, found {list(columns)!r}",
        )


def _backup_copy(source: sqlite3.Connection, output: Path) -> None:
    """Make a consistent SQLite backup snapshot without writing the source."""
    with sqlite3.connect(output) as destination:
        source.backup(destination)


def _copy_input_to_private_snapshot(input_path: Path, staging_dir: Path) -> Path:
    """Copy the SQLite main file and live-WAL sidecars without opening the input.

    SQLite's read-only URI can still write lock state into a live ``-shm`` file.
    This helper uses ordinary file reads only; all SQLite access starts after the
    files have been copied under ``staging_dir``.
    """
    snapshot = staging_dir / "snapshot.sqlite"
    shutil.copy2(input_path, snapshot)
    for suffix in ("-wal", "-shm"):
        sidecar = Path(f"{input_path}{suffix}")
        if sidecar.exists():
            if not sidecar.is_file():
                raise UpgradeRefused(f"input sidecar is not a regular file: {sidecar}")
            shutil.copy2(sidecar, Path(f"{snapshot}{suffix}"))
    return snapshot


def _add_compatibility_columns(output: Path) -> None:
    """Add only the two columns the current reader requires, without data rewrites."""
    with sqlite3.connect(output) as connection:
        connection.execute("BEGIN IMMEDIATE")
        try:
            connection.execute("ALTER TABLE operational_mutations ADD COLUMN schema_id TEXT")
            connection.execute(
                "ALTER TABLE operational_mutations "
                "ADD COLUMN write_cursor INTEGER NOT NULL DEFAULT 0",
            )
        except BaseException:
            connection.rollback()
            raise
        connection.commit()
        _check_integrity(connection, "upgraded copy")


def _verify_with_fathomdb(output: Path) -> None:
    """Require the invoking Python runtime to open and read the copied database."""
    verifier = r'''
import sqlite3
import sys

from fathomdb import Engine, read

path = sys.argv[1]
with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
    row = connection.execute(
        "SELECT collection_name FROM operational_mutations ORDER BY id LIMIT 1"
    ).fetchone()
collection = row[0] if row is not None else "__fathomdb_upgrade_probe__"
engine = Engine.open(path, use_default_embedder=False)
try:
    read.collection(engine, collection, after_id=None, limit=1)
finally:
    engine.close()
'''
    completed = subprocess.run(
        [sys.executable, "-c", verifier, str(output)],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "no diagnostic"
        raise UpgradeRefused(f"FathomDB open/read.collection verification failed: {detail}")


def upgrade_copy(input_path: Path, output_path: Path) -> None:
    """Copy and narrowly upgrade a recognized legacy database, never its input."""
    source = input_path.resolve(strict=True)
    output = output_path.resolve()
    if not source.is_file():
        raise UpgradeRefused(f"input is not a regular file: {source}")
    if output.exists():
        raise UpgradeRefused(f"refusing to overwrite output path: {output}")
    if source == output:
        raise UpgradeRefused("input and output paths must differ")

    if not output.parent.is_dir():
        raise UpgradeRefused(f"output parent directory does not exist: {output.parent}")

    with tempfile.TemporaryDirectory(
        prefix=f".{output.name}.legacy-op-store-",
        dir=output.parent,
    ) as temporary_dir:
        snapshot = _copy_input_to_private_snapshot(source, Path(temporary_dir))
        with sqlite3.connect(snapshot) as connection:
            _check_integrity(connection, "private input snapshot")
            _require_exact_legacy_schema(connection)
            _backup_copy(connection, output)

    _add_compatibility_columns(output)
    _verify_with_fathomdb(output)


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path, help="legacy SQLite database; never modified")
    parser.add_argument("--output", required=True, type=Path, help="new output SQLite database path")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Run the tool and return a shell-compatible status code."""
    print(_UNSUPPORTED_WARNING, file=sys.stderr)
    args = _parse_args(argv)
    try:
        upgrade_copy(args.input, args.output)
    except (OSError, sqlite3.Error, UpgradeRefused) as error:
        print(f"legacy op-store upgrade refused: {error}", file=sys.stderr)
        return 1
    print(f"created verified upgraded copy: {args.output.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
