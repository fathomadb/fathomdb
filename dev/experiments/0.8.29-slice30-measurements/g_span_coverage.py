"""Read-only stored-span witness after a native G store has closed."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys


def stored_coverage(store):
    """Read committed span metadata without inference or changing projection policy."""
    with closing(sqlite3.connect(Path(store).resolve().as_uri() + "?mode=ro", uri=True)) as connection:
        rows = connection.execute(
            "SELECT window_number, byte_start, byte_end, body_length "
            "FROM _fathomdb_vector_spans ORDER BY write_cursor, window_number"
        ).fetchall()
    spans = [dict(zip(("window_number", "byte_start", "byte_end", "body_length"), row, strict=True))
             for row in rows]
    known = [row for row in spans if row["byte_end"] is not None]
    return {
        "span_count": len(spans),
        "spans": spans,
        "full_body_spans": sum(row["byte_start"] == 0 and row["byte_end"] == row["body_length"]
                               for row in known),
        "partial_body_spans": sum(row["byte_start"] != 0 or row["byte_end"] != row["body_length"]
                                  for row in known),
        "unknown_coverage_spans": len(spans) - len(known),
        "body_bytes": sum(row["body_length"] for row in spans),
        "known_covered_bytes": sum(row["byte_end"] - row["byte_start"] for row in known) if known else None,
    }


if __name__ == "__main__":
    print(json.dumps(stored_coverage(sys.argv[1])))
