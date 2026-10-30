#!/usr/bin/env python3
"""Tables from the scale-search results.json (`ssweep.py summarize`).

  python3 analyze.py results.json components [--pass hybrid] [--cells BASE,CS64]
      Per traced cell, corpus and n: exclusive page requests, misses and ms
      per search by component (statements grouped by SQL text, below), and
      the scaling exponent b = log(cost@100k / cost@10k) / log(10).
  python3 analyze.py results.json latency [--pass hybrid]
      Untraced p50/p99 per cell, corpus and n, every repeat, with digests,
      soft fallbacks, mean match count and load flags.

Components (exclusive costs; they sum to the traced total):
  fts_content  FTS5 content lookups (search_index_content by rowid)
  fts_docsize  FTS5 docsize lookups (bm25 document lengths)
  fts_main     the top-level text statements' own cost: doclist pages read
               through blob handles, bm25 arithmetic, and (time only) the
               caller's row decoding between steps
  fts_index    FTS5 segment-index lookups (search_index_idx)
  fts_sorter   FTS5's internal ORDER BY rank statement (text-only route)
  elig_vector  the vector-row direct-eligibility EXISTS scan
  elig_nodes   the all-nodes direct-eligibility EXISTS scan
  vector       phase-1 bit-KNN + rerank and vec0's internal statements
  hydrate      canonical_nodes point reads for hits
  other        everything else (including statements below the trace top-N)
"""

from __future__ import annotations

import argparse
import json
import math
import statistics

COMPONENTS = ("fts_main", "fts_content", "fts_docsize", "fts_index", "fts_sorter", "elig_vector",
              "elig_nodes", "vector", "hydrate", "other")


def component(sql: str) -> str:
    if "search_index_content" in sql or "search_index_edges_content" in sql:
        return "fts_content"
    if "search_index_docsize" in sql:
        return "fts_docsize"
    if "ORDER BY bm25(\"search_index\")" in sql or "ORDER BY bm25(\"search_index_edges\")" in sql:
        return "fts_sorter"
    if sql.startswith("SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_rows"):
        return "elig_vector"
    if sql.startswith("SELECT EXISTS(SELECT 1 FROM canonical_nodes n WHERE"):
        return "elig_nodes"
    if "vector_default" in sql:
        return "vector"
    if sql.startswith(("SELECT search_index.body", "SELECT body, kind, write_cursor, bm25",
                       "SELECT rowid, bm25(search_index)", "SELECT body, kind, write_cursor FROM search_index")):
        return "fts_main"
    if "search_index_idx" in sql or "search_index_data" in sql or "MATCH" in sql:
        return "fts_index"
    if sql.startswith("SELECT kind, body, logical_id") or "FROM canonical_nodes WHERE write_cursor" in sql:
        return "hydrate"
    return "other"


def split(trace: dict) -> dict:
    out = {c: {"req": 0.0, "miss": 0.0, "ms": 0.0} for c in COMPONENTS}
    for s in trace["statements"]:
        c = out[component(s["sql"])]
        c["req"] += s["excl_requests_per_search"]
        c["miss"] += s["excl_miss_per_search"]
        c["ms"] += s["excl_ms_per_search"]
    listed = {k: sum(v[k] for v in out.values()) for k in ("req", "miss", "ms")}
    out["other"]["req"] += trace["exclusive_requests_per_search"] - listed["req"]
    out["other"]["miss"] += trace["exclusive_miss_per_search"] - listed["miss"]
    out["other"]["ms"] += trace["exclusive_ms_per_search"] - listed["ms"]
    return out


def parse_key(key: str) -> dict:
    parts = key.split("|")
    d = {"cell": parts[0], "corpus": parts[1]}
    for p in parts[2:]:
        if p.startswith("n="):
            d["n"] = int(p[2:])
        elif p in ("trace", "time"):
            d["kind"] = p
        elif p.startswith("sup="):
            d["sup"] = int(p[4:])
    return d


def cmd_components(data: dict, args) -> None:
    rows: dict = {}
    for key, entry in data.items():
        k = parse_key(key)
        if k.get("corpus") == "gate" or k.get("kind") != "trace" or k.get("sup"):
            continue
        if args.cells and k["cell"] not in args.cells:
            continue
        run = entry["runs"][-1]
        p = run.get("passes", {}).get(args.pass_)
        if not p or not p.get("trace"):
            continue
        rows[(k["cell"], k["corpus"], k["n"])] = (split(p["trace"]), p, run)
    for (cell, corpus, n), (parts, p, run) in sorted(rows.items()):
        m = run["matches"]["mean"]
        print(f"\n{cell} {corpus} n={n} pass={args.pass_} traced p50={p['p50_ms']:.1f} ms "
              f"total {p['trace']['exclusive_ms_per_search']:.1f} ms/search, matches mean "
              f"{m:.0f} ({run['matches']['frac_mean']:.2f} of n)")
        print(f"  {'component':12} {'requests':>10} {'misses':>8} {'ms':>8}  b_req  b_ms")
        base = rows.get((cell, corpus, 10_000))
        for c in COMPONENTS:
            v = parts[c]
            b = ""
            if base and n != 10_000:
                v0 = base[0][c]
                f = math.log(n / 10_000)
                br = math.log(v["req"] / v0["req"]) / f if v["req"] > 0 and v0["req"] > 0 else float("nan")
                bm = math.log(v["ms"] / v0["ms"]) / f if v["ms"] > 0 and v0["ms"] > 0 else float("nan")
                b = f"{br:5.2f} {bm:5.2f}"
            print(f"  {c:12} {v['req']:10.0f} {v['miss']:8.0f} {v['ms']:8.2f}  {b}")


def cmd_latency(data: dict, args) -> None:
    for key in sorted(data, key=lambda k: (parse_key(k).get("corpus"), parse_key(k).get("n", 0), k)):
        k = parse_key(key)
        entry = data[key]
        if k.get("corpus") == "gate":
            vals = [r["result"]["p50_ms"] for r in entry["runs"] if r["result"]]
            print(f"{key:40} gate p50 {vals} loads "
                  f"{[round(r['load_after'], 2) for r in entry['runs']]}")
            continue
        if k.get("kind") != "time":
            continue
        reps = []
        for r in entry["runs"]:
            p = r.get("passes", {}).get(args.pass_)
            if p:
                reps.append((p["p50_ms"], p["p99_ms"], p["digest"], p["ids_digest"],
                             p["soft_fallback_searches"], r["load_after"], r["load_flag"]))
        if not reps:
            continue
        med = statistics.median(x[0] for x in reps)
        print(f"{key:40} {args.pass_} p50 median {med:7.1f} | " + "; ".join(
            f"{x[0]:.1f}/{x[1]:.1f} {x[2]} {x[3]} fb={x[4]} load={x[5]:.2f}{'!' if x[6] else ''}"
            for x in reps))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("results")
    ap.add_argument("what", choices=("components", "latency"))
    ap.add_argument("--pass", dest="pass_", default="hybrid")
    ap.add_argument("--cells", type=lambda s: s.split(","))
    args = ap.parse_args()
    data = json.load(open(args.results))
    {"components": cmd_components, "latency": cmd_latency}[args.what](data, args)


if __name__ == "__main__":
    main()
