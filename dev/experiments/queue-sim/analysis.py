"""Turn results/*.csv into markdown tables (results/tables.md)."""
import csv
import os
import statistics as st
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "results")
out = []
P = out.append


def R(n):
    return list(csv.DictReader(open(os.path.join(RES, n))))


def has(n):
    return os.path.exists(os.path.join(RES, n))


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return float("nan")


def pc(x, d=2):
    return "—" if x != x else f"{100 * x:.{d}f}%"


def ms(x):
    return "—" if x != x else (f"{1000 * x:.0f} ms" if x < 10 else f"{x:.1f} s")


def mean(v):
    v = [x for x in v if x == x]
    return sum(v) / len(v) if v else float("nan")


def med(v):
    v = [x for x in v if x == x]
    return st.median(v) if v else float("nan")


def q(v, p):
    v = sorted(x for x in v if x == x)
    return v[min(len(v) - 1, int(p / 100 * len(v)))] if v else float("nan")


def table(head, rows):
    P("| " + " | ".join(head) + " |")
    P("|" + " --- |" * len(head))
    for r in rows:
        P("| " + " | ".join(str(x) for x in r) + " |")
    P("")


def sse(ys):
    if not ys:
        return 0.0
    m = sum(ys) / len(ys)
    return sum((y - m) ** 2 for y in ys)


# ---------------------------------------------------------------- measured service times
d = defaultdict(list)
for r in R("service_times.csv"):
    d[r["type"]].append(float(r["ms"]))
P("## T1. Measured service times, sequential (ms; conv4, RTX 3090 #0, client-side at stdio)\n")
table(["call", "n", "p50", "p90", "p99", "max"],
      [[k, len(v), *(f"{q(v, p):.2f}" for p in (50, 90, 99)), f"{max(v):.2f}"] for k, v in d.items()])

if has("burst_probe_summary.md"):
    P("## T2. Live burst probe at the stdio boundary (ms from one stdin write of k calls)\n")
    P(open(os.path.join(RES, "burst_probe_summary.md")).read())

# ---------------------------------------------------------------- grids
if has("grid_A_depth.csv"):
    g = defaultdict(list)
    for r in R("grid_A_depth.csv"):
        g[(r["cap"], r["depth"], r["K"], r["hybrid"], r["admission"])].append(r)
    P("## T3. Queue depth (research § 9 input 4): grounded agents, 10-call fan-out turn per session, P0, mean of 3 seeds\n")
    P("Cells: busy-refused share of calls / share of agent turns with ≥1 refusal / burst completion p50 · p99 / max queue / worst lone-call wait.\n")
    for hyb in ("0.0", "1.0"):
        for adm in ("fifo_shed", "fifo_predict"):
            P(f"**{'hybrid' if hyb == '1.0' else 'text'}, {adm}**\n")
            rows = []
            for cap in ("4", "7"):
                for depth in ("0", "8", "16", "32", "64", ""):
                    if adm == "fifo_predict" and depth == "0":
                        continue
                    row = [cap, "unbounded" if depth == "" else depth]
                    for K in ("1", "2", "6", "12", "20"):
                        v = g[(cap, depth, K, hyb, adm)]
                        row.append(f"{pc(mean([f(x['refuse_rate']) for x in v]), 1)} / {pc(mean([f(x['turn_refused']) for x in v]), 1)} / "
                                   f"{ms(mean([f(x['burst_done_p50']) for x in v]))} · {ms(mean([f(x['burst_done_p99']) for x in v]))} / "
                                   f"{max(int(f(x['max_queue'])) for x in v)} / {ms(q([f(x['lone_wait_max']) for x in v], 100))}")
                    rows.append(row)
            table(["cap", "depth", "K=1", "K=2", "K=6", "K=12", "K=20"], rows)

if has("grid_B_overload.csv"):
    g = defaultdict(list)
    for r in R("grid_B_overload.csv"):
        g[(r["K"], r["hybrid"], r["slow_p"], r["policy"])].append(r)
    P("## T4. Sustained overload (input 5): clients with no think time firing 10-call bursts, cap 4, P0\n")
    P("Cells: goodput ok/s / refused / false refusals / ok p99 / timeouts.\n")
    pols = ["refuse", "depth8", "depth32", "unbounded", "arrival_ewma_d32", "arrival_p90_d32", "arrival_p99_d32"]
    rows = []
    for K in ("1", "4"):
        for hyb in ("0.0", "1.0"):
            for slow in ("0.0", "0.01"):
                row = [K, "hybrid" if hyb == "1.0" else "text", slow]
                for p in pols:
                    v = g[(K, hyb, slow, p)]
                    row.append(f"{mean([f(x['goodput_per_s']) for x in v]):.0f} / {pc(mean([f(x['refuse_rate']) for x in v]), 0)} / "
                               f"{pc(mean([f(x['false_refusal_rate']) for x in v]), 1)} / {ms(mean([f(x['others_ok_p99']) for x in v]))} / "
                               f"{pc(mean([f(x['timeout_rate']) for x in v]), 2)}")
                rows.append(row)
    table(["clients", "mode", "slow p (2 s)"] + pols, rows)

if has("grid_C_hol.csv"):
    g = defaultdict(list)
    for r in R("grid_C_hol.csv"):
        g[(r["workload"], r["cap"], r["slow_s"], r["dispatch"], r["interrupt"])].append(r)
    P("## T5. Head-of-line probe (input 7): one slow read admitted every 2 s; fifo_shed, depth 32\n")
    P("Cells: other calls' ok p95 · p99 / their timeout rate. 'capacity' = cap clients with no think (design § 7.2 HOL probe); 'agents' = 6 grounded agents.\n")
    rows = []
    for wl in ("capacity", "agents"):
        for cap in ("4", "7"):
            for dur in ("1.0", "3.0", "10.0"):
                row = [wl, cap, dur]
                for disp in ("P0", "P1"):
                    for intr in ("False", "True"):
                        v = g[(wl, cap, dur, disp, intr)]
                        row.append(f"{ms(mean([f(x['others_ok_p95']) for x in v]))} · {ms(mean([f(x['others_ok_p99']) for x in v]))} / "
                                   f"{pc(mean([f(x['others_timeout']) for x in v]), 2)}")
                rows.append(row)
    table(["workload", "cap", "slow read s", "P0", "P0 + EP-09", "P1", "P1 + EP-09"], rows)

# ---------------------------------------------------------------- incident
if has("incident.csv"):
    ir = R("incident.csv")
    g = defaultdict(list)
    for r in ir:
        g[(r["admission"], r["queue_limit"], r["cap"], r["prefix"])].append(r)
    P("## T6. Incident reproduction: 6 agents, one shared session, grounded turns (10-call fan-out, then 1–3-call turns, think median 3.3 s), 300 seeds\n")
    rows = []
    for k, v in sorted(g.items()):
        ref = [f(r["refused"]) / f(r["n"]) for r in v]
        fo = sum(f(r["refused_in_fanout"]) for r in v) / max(1, sum(f(r["refused"]) for r in v))
        six = 0
        tot = 0
        for r in v:
            pb = eval(r["per_burst"]) if isinstance(r["per_burst"], str) else r["per_burst"]
            for (kk, nb), n in pb.items():
                if kk >= 10:
                    tot += n
                    six += n if nb == (10 - int(k[2])) else 0
        rows.append([k[0], "unbounded" if k[1] in ("", "None") else k[1], k[2], k[3], pc(mean(ref), 1), f"{100 * fo:.0f}%",
                     pc(six / max(1, tot), 1), f"{med([f(r['ok_p99_ms']) for r in v]):.1f}"])
    table(["admission", "queue limit", "cap", "session prefix", "refused / calls", "refusals inside fan-out turns",
           "fan-out turns with exactly 10−cap refused", "ok p99 ms"], rows)

# ---------------------------------------------------------------- main fuzz
ARMS = [("refuse", "P0", "False"), ("refuse", "P1", "False"), ("refuse", "P2", "False"),
        ("fifo", "P0", "False"), ("fifo", "P1", "False"),
        ("fifo_shed", "P0", "False"), ("fifo_shed", "P1", "False"), ("fifo_shed", "P2", "False"),
        ("fifo_predict", "P0", "False"), ("fifo_predict", "P1", "False"),
        ("fifo_shed", "P0", "True"), ("fifo_shed", "P1", "True"),
        ("fifo_predict", "P0", "True"), ("fifo_predict", "P1", "True")]


def arm_name(a):
    return f"{a[0]} · {a[1]}" + (" · EP-09" if a[2] == "True" else "")


def load_main(name):
    rows = R(name)
    by = defaultdict(dict)
    for r in rows:
        r["fail"] = f(r["timeout_rate"]) + f(r["refuse_rate"])
        by[r["seed"]][(r["admission"], r["dispatch"], r["interrupt"])] = r
    return [c for c in by.values() if len(c) == len(ARMS)]


def base(c):
    return c[("refuse", "P0", "False")]


def band_table(cfgs, title):
    P(f"### {title} — {len(cfgs)} configurations, means over configurations (paired, common random numbers)\n")
    rows = []
    for a in ARMS:
        v = [c[a] for c in cfgs]
        rows.append([arm_name(a)] + [pc(mean([f(x[k]) for x in v]), 3) for k in
                                     ("timeout_rate", "refuse_rate", "false_refusal_rate", "turn_refused")] +
                    [pc(mean([f(x["to_hol"]) for x in v]), 3), ms(mean([f(x["ok_p95"]) for x in v])),
                     ms(mean([f(x["ok_p99"]) for x in v])), f"{mean([f(x['abandoned_avg']) for x in v]):.3f}",
                     pc(sum(1 for x in v if x["fail"] > 0.01) / max(1, len(v)), 0)])
    table(["arm", "timeout", "refused", "false refusal", "turns with a refusal", "timeouts from HOL", "ok p95", "ok p99",
           "abandoned permits", "configs with >1% failing"], rows)


def real(c): return int(c["K"]) <= 8 and f(c["slow_p"]) <= 0.01 and f(c["speed_mult"]) == 1.0
def stress(c): return int(c["K"]) >= 16 and f(c["slow_p"]) >= 0.02
cfgs = []
if has("fuzz_real.csv"):
    rc = load_main("fuzz_real.csv")
    P("## T7. Realistic band (dedicated fuzz: K 1–8 agents, slow p 0.1–1 %, GPU service times)\n")
    band_table(rc, "realistic")
    band_table([c for c in rc if base(c)["session_prefix"] not in ("()", "", "[]")], "realistic, prompted-parallel agents (one F-call fan-out per session)")
    band_table([c for c in rc if base(c)["session_prefix"] in ("()", "", "[]")], "realistic, TraceLab-like turns (mean 1.2 calls)")
if has("fuzz_main.csv"):
    cfgs = load_main("fuzz_main.csv")
    P("## T8. Full fuzz\n")
    band_table([c for c in cfgs if stress(base(c))], "stress (K ≥ 16, slow p ≥ 2 %)")
    band_table(cfgs, "all")

    def ex(c, a, b):
        return c[a]["fail"] - c[b]["fail"]

    S0, S1, S0i = ("fifo_shed", "P0", "False"), ("fifo_shed", "P1", "False"), ("fifo_shed", "P0", "True")
    P("### T9. HOL map: mean paired failure excess of round-robin over the shared queue (fifo_shed P0 − P1), all configs\n")
    sp_b = [(0, 0.005), (0.005, 0.01), (0.01, 0.02), (0.02, 0.05)]
    sm_b = [(0.2, 0.5), (0.5, 1.0), (1.0, 2.0), (2.0, 4.0), (4.0, 8.0)]
    rows = []
    for a, b in sp_b:
        row = [f"slow p {a * 100:g}–{b * 100:g}%"]
        for x, y in sm_b:
            v = [ex(c, S0, S1) for c in cfgs if a <= f(base(c)["slow_p"]) < b and x <= f(base(c)["slow_median"]) < y]
            row.append(f"{pc(mean(v), 2)} (n{len(v)})")
        rows.append(row)
    table(["", *(f"median {x}–{y} s" for x, y in sm_b)], rows)
    rows = []
    for kb in [(1, 4), (5, 8), (9, 16), (17, 32)]:
        row = [f"K {kb[0]}–{kb[1]}"]
        for cap in ("4", "7"):
            sel = [c for c in cfgs if kb[0] <= int(base(c)["K"]) <= kb[1] and base(c)["cap"] == cap]
            row.append(f"P0−P1 {pc(mean([ex(c, S0, S1) for c in sel]), 2)}; P0−(P0+EP-09) {pc(mean([ex(c, S0, S0i) for c in sel]), 2)}; "
                       f"P1 base {pc(mean([c[S1]['fail'] for c in sel]), 2)}")
        rows.append(row)
    table(["agents", "cap 4", "cap 7"], rows)

    FEAT = ["K", "cap", "slow_p", "slow_median", "slow_sigma", "speed_mult", "hybrid_frac", "think_median",
            "session_prefix", "writer_duty", "projection_workers", "projection_batch_s"]

    def feat(c, k):
        v = base(c).get(k, "")
        if k == "session_prefix":
            return f(v.strip("()[],") or 0)
        return f(v) if v not in ("", "None") else 0.0

    def grow(pts, depth, pre=""):
        ys = [y for _, y in pts]
        if depth == 0 or len(pts) < 80:
            P(f"{pre}→ mean excess {pc(mean(ys), 2)} (n={len(pts)}; configs with excess >1%: {sum(y > 0.01 for y in ys)})")
            return
        best = None
        for j in range(len(FEAT)):
            vals = sorted(set(x[j] for x, _ in pts))
            cand = [vals[int(len(vals) * k / 20)] for k in range(1, 20)] if len(vals) > 20 else vals[1:]
            for t in cand:
                L = [y for x, y in pts if x[j] < t]
                Rr = [y for x, y in pts if x[j] >= t]
                if len(L) < 40 or len(Rr) < 40:
                    continue
                s_ = sse(L) + sse(Rr)
                if best is None or s_ < best[0]:
                    best = (s_, j, t)
        if best is None:
            P(f"{pre}→ mean excess {pc(mean(ys), 2)} (n={len(pts)})")
            return
        _, j, t = best
        P(f"{pre}if {FEAT[j]} < {t:.3g}:")
        grow([p for p in pts if p[0][j] < t], depth - 1, pre + "    ")
        P(f"{pre}else ({FEAT[j]} ≥ {t:.3g}):")
        grow([p for p in pts if p[0][j] >= t], depth - 1, pre + "    ")

    P("### T10. Decision tree (regression tree, depth 3) on the paired excess fifo_shed P0 − P1\n")
    P("```text")
    grow([([feat(c, k) for k in FEAT], ex(c, S0, S1)) for c in cfgs], 3)
    P("```\n")

    def eta2(xs, ys, nb=8):
        pairs = sorted(zip(xs, ys))
        n = len(pairs)
        if len(set(xs)) <= nb:
            gg = defaultdict(list)
            for x, y in pairs:
                gg[x].append(y)
            gs = list(gg.values())
        else:
            gs = [[y for _, y in pairs[i * n // nb:(i + 1) * n // nb]] for i in range(nb)]
        tot = sse(ys)
        return 1 - sum(sse(gx) for gx in gs) / tot if tot else 0.0

    targets = {"fail, today (refuse·P0)": lambda c: base(c)["fail"],
               "fail, fifo_shed·P0": lambda c: c[S0]["fail"],
               "fail, fifo_shed·P1": lambda c: c[S1]["fail"],
               "fail, fifo_predict·P1·EP-09": lambda c: c[("fifo_predict", "P1", "True")]["fail"],
               "excess P0−P1": lambda c: ex(c, S0, S1),
               "gain refuse→fifo_shed (P0)": lambda c: base(c)["fail"] - c[S0]["fail"]}
    P("### T11. Knob importance: share of variance explained (η², 8 quantile bins), all configs\n")
    table(["knob"] + list(targets),
          [[k] + [f"{eta2([feat(c, k) for c in cfgs], [t(c) for c in cfgs]):.3f}" for t in targets.values()] for k in FEAT])

# ---------------------------------------------------------------- thresholds
if has("fuzz_thresh.csv"):
    tr_ = R("fuzz_thresh.csv")
    cfg_t = defaultdict(dict)
    for r in tr_:
        r["fail"] = f(r["timeout_rate"]) + f(r["refuse_rate"])
        cfg_t[(r["seed"], r["dispatch"])][(r["policy"], r["deadline"])] = r
    pols = []
    for r in tr_:
        if (r["policy"], r["deadline"]) not in pols:
            pols.append((r["policy"], r["deadline"]))
    for disp in ("P0", "P1"):
        keys = [k for k in cfg_t if k[1] == disp]
        keys = [k for k in keys if ("T0", "fixed10") in cfg_t[k]]  # S5: skip cells missing their T0 baseline
        t0 = {k: cfg_t[k][("T0", "fixed10")]["fail"] for k in keys}
        for lab, pred in (("all", lambda k: True), ("loaded: T0 fails > 0.5 % of calls", lambda k: t0[k] > 0.005)):
            sel = [k for k in keys if pred(k)]
            P(f"### T12. Admission cutoff — dispatch {disp}, {lab}: {len(sel)} configurations (paired)\n")
            rows = []
            for p in pols:
                v = [cfg_t[k][p] for k in sel if p in cfg_t[k]]
                rows.append([p[0] + ("" if p[1] == "fixed10" else f" · D={p[1]}")] +
                            [pc(mean([f(x[m]) for x in v]), 3) for m in
                             ("false_refusal_rate", "late_admit_timeout_rate", "timeout_rate", "refuse_rate", "fail")] +
                            [f"{mean([f(x['abandoned_avg']) for x in v]):.3f}", ms(mean([f(x['ok_p95']) for x in v]))])
            table(["policy", "false refusal", "late admit → timeout", "timeout", "refused", "fail total",
                   "abandoned permits", "ok p95"], rows)
    v = [r for r in tr_ if int(f(r["retry_n"]) or 0) >= 5 and r["policy"] == "T1_F250" and r["deadline"] == "fixed10"]
    if v:
        P(f"### T13. retry_after_ms accuracy (T1_F250 cells, clients that honour the hint, ≥5 advised retries: {len(v)} cells)\n")
        P(f"Median over cells of median |observed − advised| / advised: {med([f(r['retry_relerr_med']) for r in v]):.2f}; "
          f"median re-refused share: {pc(med([f(r['retry_rerefused']) for r in v]), 1)}; cells meeting the design's ship rule "
          f"(error ≤ 0.5 and re-refused < 20 %): {sum(f(r['retry_relerr_med']) <= 0.5 and f(r['retry_rerefused']) < 0.2 for r in v)}/{len(v)}.\n")

if has("step.csv"):
    sr = R("step.csv")
    g = defaultdict(list)
    for r in sr:
        g[(r["scenario"], r["dispatch"], r["policy"])].append(r)
    P("## T14. Step shift: every service time ×10 at t = 4000 s (5 seeds, mean)\n")
    for sc in ("cpu_k8", "cpu_k16"):
        for disp in ("P0", "P1"):
            P(f"### {sc}, {disp}\n")
            rows = []
            for (s_, d_, p), v in g.items():
                if s_ != sc or d_ != disp:
                    continue
                def m(k, v=v): return mean([f(x[k]) for x in v])
                rows.append([p, f"{m('need_pre') * 1000:.0f} → {m('need_post') * 1000:.0f}", f"{m('adapt_s'):.0f}", f"{m('need_cv_late'):.2f}",
                             pc(m("first300_to") + m("first300_rf"), 2), pc(m("first300_late"), 2), pc(m("first300_fs"), 2),
                             pc(m("post_to"), 2), pc(m("post_rf"), 2), pc(m("post_late"), 2), ms(m("post_p95"))])
            table(["policy", "need ms pre → post", "adapt s", "need CV late", "fail, first 300 s", "late-admit TO, first 300 s",
                   "false shed, first 300 s", "post TO", "post refused", "post late-admit TO", "post ok p95"], rows)

open(os.path.join(RES, "tables.md"), "w").write("\n".join(out))
print("\n".join(out))
