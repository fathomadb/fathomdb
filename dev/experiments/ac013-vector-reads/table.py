#!/usr/bin/env python3
"""Print the candidate table from a vsweep.py summary (results.json).

  python3 dev/experiments/ac013-vector-reads/table.py dev/experiments/ac013-vector-reads/results.json

One Markdown row per cell, every repeat listed: gate p50 / p99 / seed (ms),
probe page requests and misses per hybrid search, phase-1 (vector statement)
requests and misses from the trace plus page requests made outside any
statement (direct blob reads), whether the results digest equals BASE's
(at the same corpus size) in every run, ac_013b recall, REQ40-I6 worst route ratio per repeat, AC40-97,
MCP p50 / p99 at 1 in flight, and the checkpointed store size. A trailing
`*` marks a run whose load after exceeded the flag threshold.
"""

from __future__ import annotations

import json
import sys

PHASE1 = "WITH candidates AS"
CANDIDATES = "SELECT rowid FROM vector_default WHERE embedding_bin MATCH"
ROWIDS = "vector_default_rowids"
F32 = "_fathomdb_vector_f32"
DISTANCE = "SELECT vec_distance_l2(?1, ?2)"


def star(entry: dict) -> str:
    return "*" if entry.get("load_flag") else ""


def vector_stage(entry: dict) -> tuple[float, float] | None:
    """Exclusive requests and misses per search of the vector-stage statements,
    plus the requests made outside any statement (the CHUNKED cell's direct
    blob reads): the probe's reader total minus the trace's attributed total."""
    trace = entry.get("trace")
    if not trace:
        return None
    hybrid = entry.get("hybrid") or {}
    total_req = hybrid.get("cache_hit_per_search", 0) + hybrid.get("cache_miss_per_search", 0)
    req = max(total_req - trace["exclusive_requests_per_search"], 0.0)
    miss = max(hybrid.get("cache_miss_per_search", 0) - trace["exclusive_miss_per_search"], 0.0)
    for s in trace["statements"]:
        sql = s["sql"]
        if (sql.startswith(PHASE1) or sql.startswith(CANDIDATES) or ROWIDS in sql
                or F32 in sql or sql.startswith(DISTANCE) or "vector_default_chunks" in sql
                or sql.startswith("SELECT vec_f32(")):
            req += s["excl_requests_per_search"]
            miss += s["excl_miss_per_search"]
    return req, miss


META = {"repeat", "load_before", "load_after", "load_flag", "exit"}


def routes(entry: dict) -> dict:
    return {k: v for k, v in entry.items() if k not in META and isinstance(v, (int, float))}


def fmt_list(values, digits=0) -> str:
    return ", ".join("—" if v is None else f"{v:,.{digits}f}" for v in values) or "—"


def main() -> None:
    cells = json.load(open(sys.argv[1]))
    rows = []
    for name, c in cells.items():
        suffix = name[name.index("@"):] if "@" in name else ""
        base = cells.get("BASE" + suffix, {})
        base_digest = {e.get("results_digest") for e in base.get("vreads", []) + base.get("vtrace", [])}
        gate = c.get("gate", [])
        vr = c.get("vreads", [])
        vt = c.get("vtrace", [])
        digests = {e.get("results_digest") for e in vr + vt}
        same = "yes" if digests and digests <= base_digest else ("—" if not digests else "NO")
        stage = [vector_stage(e) for e in vt]
        rows.append([
            name,
            ", ".join(f"{e.get('p50_ms')}{star(e)}" for e in gate) or "—",
            ", ".join(f"{e.get('p99_ms')}{star(e)}" for e in gate) or "—",
            fmt_list([e.get("seed_ms") for e in gate]),
            fmt_list([(e["hybrid"]["cache_hit_per_search"] + e["hybrid"]["cache_miss_per_search"])
                      for e in vr if e.get("hybrid")]),
            fmt_list([e["hybrid"]["cache_miss_per_search"] for e in vr if e.get("hybrid")]),
            ", ".join(f"{s[0]:,.0f} / {s[1]:,.0f}" for s in stage if s) or "—",
            same,
            fmt_list([e.get("recall_at_10") for e in c.get("recall", [])], 4),
            ", ".join(f"{max(routes(e).values()):.3f}{star(e)}"
                      for e in c.get("i6", []) if routes(e)) or "—",
            ", ".join(e.get("other_readers_keep_their_page_caches_after_a_heavy_read", "—")
                      for e in c.get("iso", [])) or "—",
            ", ".join(f"{e.get('p50_ms', 0):.1f} / {e.get('p99_ms', 0):.1f}"
                      for e in c.get("mcp", [])) or "—",
            fmt_list([e.get("file_bytes_checkpointed", 0) / 1e6 for e in vr], 1),
        ])
    head = ["Cell", "Gate p50", "Gate p99", "Seed", "Requests/search", "Misses/search",
            "Vector stage req / miss", "Same results", "ac_013b", "REQ40-I6", "AC40-97",
            "MCP p50 / p99", "Store MB"]
    print("| " + " | ".join(head) + " |")
    print("|" + " --- |" * len(head))
    for r in rows:
        print("| " + " | ".join(r) + " |")


if __name__ == "__main__":
    main()
