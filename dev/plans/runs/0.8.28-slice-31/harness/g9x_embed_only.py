"""PERF-03 analysis using analyze.py's own runs/per_process/boot/med, accepting
runs whose only failure is the rerank step (x86 release feature set embed-cuda
has no reranker). Usage: g9x_embed_only.py <harness-dir> <series-root>"""
import sys
sys.path.insert(0, sys.argv[1])
import analyze as A
root = sys.argv[2]
def coll(d):
    acc, n = {}, 0
    for r in A.runs(d):
        if not (r.get("outcome") == "pass" or r.get("failedStep") == "rerank"):
            continue
        n += 1
        for k, v in A.per_process(r).items():
            acc.setdefault(k, []).append(v)
    return acc, n
p, np_ = coll(f"{root}/g9x/newP")
b, nb = coll(f"{root}/g9x/baseP")
print(f"PERF-03 RTX 3090 embed-only (runs used new={np_}, base={nb}; run-000 excluded by A.runs)")
for m in A.MEASURES:
    a, c = p.get(m, []), b.get(m, [])
    if a and c:
        lo, hi = A.boot(a, c, "ratio")
        print(f"{m:8} new {A.med(a):8.3f} base {A.med(c):8.3f}  new/base {A.med(a)/A.med(c):.3f} [{lo:.3f}, {hi:.3f}]")
a, c = p["embed"], b["embed"]
A.verdict(A.med(a) / A.med(c) <= 1.05, "steady-embed median new/base <= 1.05 (95 % CI reported)")
