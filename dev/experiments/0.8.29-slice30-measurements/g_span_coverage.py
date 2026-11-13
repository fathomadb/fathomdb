"""Read-only stored-span witness after a native G store has closed."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys


def stored_coverage(store):
    """Read committed span metadata without inference or changing projection policy."""
    with closing(sqlite3.connect(Path(store).resolve().as_uri() + "?mode=ro", uri=True)) as connection:
        count = connection.execute("SELECT COUNT(*) FROM _fathomdb_vector_spans").fetchone()[0]
    return {"span_count": count}


if __name__ == "__main__":
    print(json.dumps(stored_coverage(sys.argv[1])))
