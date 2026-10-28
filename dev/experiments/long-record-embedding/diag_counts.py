"""Row counts of the search-relevant tables per store (read-only).

Usage: python diag_counts.py STORE.sqlite [...]
"""

from __future__ import annotations

import sqlite3
import sys

TABLES = ["canonical_nodes", "canonical_edges", "search_index", "search_index_v2",
          "search_index_edges", "property_search_index", "_fathomdb_vector_rows"]


def main() -> None:
    for db in sys.argv[1:]:
        con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        counts = {}
        for t in TABLES:
            try:
                counts[t] = con.execute(f"SELECT count(*) FROM {t}").fetchone()[0]
            except sqlite3.Error as exc:
                counts[t] = str(exc)
        con.close()
        print(db.rsplit("/", 1)[-1], counts)


if __name__ == "__main__":
    main()
