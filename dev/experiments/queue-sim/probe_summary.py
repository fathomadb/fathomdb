import csv, sys, statistics as st
from collections import defaultdict
g = defaultdict(list)
for r in csv.DictReader(open(sys.argv[1])):
    g[(r["phase"], r["cap"], r["mode"], r["k"], r["rep"])].append(r)
agg = defaultdict(lambda: defaultdict(list))
for (ph, cap, mode, k, rep), v in g.items():
    ok = [float(x["ms"]) for x in v if x["outcome"] == "ok"]
    bz = [float(x["ms"]) for x in v if x["outcome"] == "busy"]
    a = agg[(ph, cap, mode, k)]
    a["n_ok"].append(len(ok)); a["n_busy"].append(len(bz))
    if ok: a["last_ok"].append(max(ok)); a["first_ok"].append(min(ok))
    if bz: a["busy"].append(max(bz))
def q(v, p):
    s = sorted(v); return s[min(len(s) - 1, int(p / 100 * len(s)))] if s else float("nan")
print("| phase | cap | mode | k | reps | ok/busy per burst | busy returned (ms, p50) | first ok (ms, p50) | burst cleared = last ok (ms) p50 / p90 / max |")
print("|---|---|---|---|---|---|---|---|---|")
for key in sorted(agg, key=lambda k: (k[0], int(k[1]), k[2], int(k[3]))):
    a = agg[key]
    print(f"| {' | '.join(key)} | {len(a['n_ok'])} | {st.mean(a['n_ok']):.1f}/{st.mean(a['n_busy']):.1f} | {q(a['busy'],50):.2f} | {q(a['first_ok'],50):.2f} | {q(a['last_ok'],50):.2f} / {q(a['last_ok'],90):.2f} / {max(a['last_ok']) if a['last_ok'] else float('nan'):.2f} |")
