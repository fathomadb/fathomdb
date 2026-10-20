"""Check that P2's rotated gold bodies hold the same lines and FTS tokens as the originals.

Usage: python diag_bodies.py --original A.sqlite --rotated B.sqlite --sid SID [...]
"""

from __future__ import annotations

import argparse
import sqlite3
from pathlib import Path


def fetch(db: Path, sid: str) -> list[tuple]:
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        return con.execute(
            "SELECT cn.write_cursor, cn.body, si.body FROM canonical_nodes cn "
            "LEFT JOIN search_index si ON si.write_cursor = cn.write_cursor WHERE cn.source_id = ?",
            (sid,),
        ).fetchall()
    finally:
        con.close()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--original", type=Path, required=True)
    ap.add_argument("--rotated", type=Path, required=True)
    ap.add_argument("--sid", nargs="+", required=True)
    a = ap.parse_args()
    for sid in a.sid:
        o, r = fetch(a.original, sid), fetch(a.rotated, sid)
        print(sid, "rows", len(o), len(r))
        for (oc, ob, ofts), (rc, rb, rfts) in zip(o, r):
            print("  cursor", oc, rc, "| body len", len(ob), len(rb),
                  "| same lines", sorted(ob.split("\n")) == sorted(rb.split("\n")),
                  "| fts==body", ofts == ob, rfts == rb)


if __name__ == "__main__":
    main()
