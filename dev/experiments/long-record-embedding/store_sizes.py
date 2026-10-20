"""Vector-table bytes per store, from SQLite's dbstat (read-only).

Usage: python store_sizes.py STORE.sqlite [...]   writes results/store_sizes.json
"""

from __future__ import annotations

import json
import sqlite3
import sys
from pathlib import Path


def sizes(db: Path) -> dict:
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        vec = con.execute(
            "SELECT coalesce(sum(pgsize), 0) FROM dbstat WHERE name LIKE 'vector_default%'"
        ).fetchone()[0]
        rows = con.execute("SELECT count(*) FROM _fathomdb_vector_rows").fetchone()[0]
    finally:
        con.close()
    return {"store": db.name, "file_bytes": db.stat().st_size, "vector_bytes": vec,
            "vector_rows": rows, "bytes_per_vector": round(vec / rows) if rows else None}


def main() -> None:
    out = [sizes(Path(p)) for p in sys.argv[1:]]
    (Path(__file__).parent / "results/store_sizes.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
