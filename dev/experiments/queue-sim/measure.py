"""Measure sequential service times of the real fathomdb-mcp server over stdio.

usage: python -I measure.py <binary> <config.json> <locomo10.json> <out.csv>
One call at a time; client-side wall time per tools/call (includes stdio + JSON).
"""
import csv, json, os, random, subprocess, sys, time

BIN, CFG, LOCOMO, OUT = sys.argv[1:5]
N = int(os.environ.get("N_PER_TYPE", "300"))
WARM = 30
env = {**os.environ, "CUDA_VISIBLE_DEVICES": "0", "FATHOMDB_EMBED_DEVICE": "cuda:0"}
p = subprocess.Popen([BIN, "--config", CFG], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=subprocess.DEVNULL, text=True, env=env)
nid = 0


def call(method, params=None):
    global nid
    nid += 1
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": nid, "method": method,
                              **({"params": params} if params is not None else {})}) + "\n")
    p.stdin.flush()
    while True:
        o = json.loads(p.stdout.readline())
        if o.get("id") == nid:
            return o


def tool(name, args):
    t0 = time.perf_counter()
    r = call("tools/call", {"name": name, "arguments": args})["result"]
    return (time.perf_counter() - t0) * 1000.0, r


call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "measure", "version": "0"}})
p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n"); p.stdin.flush()

data = json.load(open(LOCOMO))
qs = [q["question"] for conv in data for q in conv["qa"]]
conv4 = [q["question"] for q in data[4]["qa"]]
rng = random.Random(20261017)
pool = conv4 + rng.sample(qs, max(0, 2 * N - len(conv4)))
rng.shuffle(pool)

# warm-up + collect refs for fetch/expand
refs = []
for q in pool[:WARM]:
    for mode in ("text", "hybrid"):
        _, r = tool("search", {"query": q, "mode": mode, "limit": 20, "max_tokens": 100000})
        refs += [h["ref"] for h in r["structuredContent"]["results"]]
refs = sorted(set(refs))
print("refs collected", len(refs), file=sys.stderr)

types = [("search_text_10", "search", lambda q: {"query": q, "mode": "text", "limit": 10}),
         ("search_text_100", "search", lambda q: {"query": q, "mode": "text", "limit": 100, "max_tokens": 200000}),
         ("search_hybrid_10", "search", lambda q: {"query": q, "mode": "hybrid", "limit": 10}),
         ("search_hybrid_100", "search", lambda q: {"query": q, "mode": "hybrid", "limit": 100, "max_tokens": 200000}),
         ("fetch_10", "fetch", lambda q: {"refs": rng.sample(refs, 10), "max_tokens": 200000}),
         ("expand_d3", "expand", lambda q: {"refs": [rng.choice(refs)], "max_depth": 3, "limit": 50, "max_tokens": 200000})]
# interleave call types so drift affects all equally
schedule = [(t, i) for i in range(N) for t in types]
rows = []
for (name, tname, mk), i in schedule:
    ms, r = tool(tname, mk(pool[(WARM + i) % len(pool)]))
    sc = r.get("structuredContent") or {}
    n = len(sc.get("results", sc.get("items", sc.get("targets", []))) or [])
    rows.append({"type": name, "i": i, "ms": round(ms, 3), "is_error": bool(r.get("isError")), "n": n})
with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
p.stdin.close(); p.wait(30)
