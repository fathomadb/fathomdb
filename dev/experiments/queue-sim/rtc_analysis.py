"""Tables and decision rules R1-R4 of PREREG-run-to-completion.md from results/rtc.csv -> results/rtc_tables.md."""
import csv
import json
import os
import random
import statistics as st
import sys
from collections import Counter, defaultdict

import rtc

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "results")
LATE_KEYS = ["late_h_10_12", "late_h_12_15", "late_h_15_20", "late_h_20_30", "late_h_30_60", "late_h_60_120",
             "late_h_120_inf"]
DECIDE = ("realistic", "base", "P2")
COMPACT = ["old_p90", "rtc_p90_rl60", "rtc_p95_rl60", "rtc_p99_rl60", "rtc_p90cap_rl60", "rtc_wait_rl30",
           "rtc_wait_rl60", "rtc_wait_rl120", "old_p90_ep09", "rtc_wait_ep09_rl60"]


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return float("nan")


def load(path):
    """Rows of a .csv, or of a runner .jsonl keeping only configurations that have every arm (all values str)."""
    if not path.endswith(".jsonl"):
        return list(csv.DictReader(open(path)))
    rows = []
    for line in open(path):
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue  # a torn last line
        if "_error" not in r:
            rows.append({k: ("" if v is None else str(v)) for k, v in r.items() if k != "_key"})
    n = 4 * len(list(rtc.arms()))
    cnt = Counter((r["band"], r["cfg_i"]) for r in rows)
    return [r for r in rows if cnt[(r["band"], r["cfg_i"])] == n]


def overlay(ix, rows, key, check="fail_rate"):
    """Copy `key` from rerun rows into ix by configuration; returns the count. A rerun row whose `check` metric
    differs from the indexed row is not the same simulation, and raises ValueError."""
    n = 0
    for r in rows:
        tgt = ix[(r["band"], r["tail"], r["dispatch"], r["arm"])].get(r["cfg_i"])
        if tgt is None:
            continue
        if f(tgt[check]) != f(r[check]):
            raise ValueError(f"rerun does not reproduce {check} for {r['band']}/{r['tail']}/{r['dispatch']}/"
                             f"{r['arm']}/{r['cfg_i']}")
        tgt[key] = r[key]
        n += 1
    return n


def index(rows):
    """{(band, tail, dispatch, arm): {cfg_i: row}}"""
    ix = defaultdict(dict)
    for r in rows:
        ix[(r["band"], r["tail"], r["dispatch"], r["arm"])][r["cfg_i"]] = r
    return ix


def pair(ix, band, tail, disp, a, b, key, exclude=None):
    ra, rb = ix[(band, tail, disp, a)], ix[(band, tail, disp, b)]
    ks = sorted((set(ra) & set(rb)) - (exclude or set()), key=lambda k: int(k))
    return [f(ra[k][key]) for k in ks], [f(rb[k][key]) for k in ks]


def paired_ci(a, b, n=2000, seed=1):
    """(lo, mean, hi) of mean(a - b): 95 % bootstrap over configurations."""
    d = [x - y for x, y in zip(a, b) if x == x and y == y]
    m = sum(d) / len(d)
    r = random.Random(seed)
    bs = sorted(sum(r.choice(d) for _ in d) / len(d) for _ in range(n))
    return bs[int(0.025 * n)], m, bs[int(0.975 * n) - 1]


def vals(ix, band, tail, disp, arm, key, exclude=None):
    rows = ix[(band, tail, disp, arm)]
    return [f(r[key]) for k, r in rows.items() if f(r[key]) == f(r[key]) and k not in (exclude or set())]


def mean(v):
    return sum(v) / len(v) if v else float("nan")


def med(v):
    return st.median(v) if v else float("nan")


def bands_of(ix):
    return [b for b in ("realistic", "stress") if any(k[0] == b for k in ix)]


def ttd_ratio(ix, band, tail, disp, a, b, exclude=None):
    """Median over configurations of ttd_p99(a) / ttd_p99(b)."""
    xa, xb = pair(ix, band, tail, disp, a, b, "ttd_p99", exclude)
    return med([x / y for x, y in zip(xa, xb) if y > 0 and x == x])


def lockups(ix, band, tail, disp, arm):
    """Configurations that refused more than 1 % of calls at arrival with a permit free (rf_arrival_free; older
    rows without it fall back to rf_with_permit, which also counts grant-time sheds)."""
    rows = ix[(band, tail, disp, arm)]
    return {k for k, r in rows.items() if f(r.get("rf_arrival_free") or r["rf_with_permit"]) > 0.01}


def r1(ix, check="p90", exclude=None):
    """R1: start check `check` against wait-only at run limit 60 s; deciding cell realistic/base/P2."""
    a, w = f"rtc_{check}_rl60", "rtc_wait_rl60"
    cells = {}
    for b in bands_of(ix):
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                if not ix[(b, t, d, a)]:
                    continue
                ex = exclude((b, t, d)) if callable(exclude) else exclude
                cells[(b, t, d)] = {
                    "dF": paired_ci(*pair(ix, b, t, d, a, w, "fail_rate", ex)),
                    "dbusy": paired_ci(*pair(ix, b, t, d, a, w, "refuse_rate", ex)),
                    "dab": paired_ci(*pair(ix, b, t, d, a, w, "timeout_rate", ex)),
                    "ratio": ttd_ratio(ix, b, t, d, w, a, ex),
                    "late": mean(vals(ix, b, t, d, w, "ok_late_share", ex)),
                    "late30": mean(vals(ix, b, t, d, w, "ok_late30_rate", ex))}
    c = cells[DECIDE]
    exclude = exclude(DECIDE) if callable(exclude) else exclude
    guard = all(cells[("realistic", t, "P2")]["late"] <= 0.01 and cells[("realistic", t, "P2")]["late30"] <= 0.001
                for t in ("base", "long"))
    c1 = not c["dF"][2] < 0
    c2 = c["ratio"] <= 1.10
    if not guard:
        verdict, chk = "REPORT TO OWNER: the late-answer guard (condition 3) fails", check
    elif c1 and c2:
        verdict, chk = "DROP the start check: start any call that gets a permit before its wait deadline", "wait"
    else:
        cands = ["p90", "p95", "p99"] if check == "p90" else [check]
        ok = [x for x in cands if ttd_ratio(ix, *DECIDE, f"rtc_{x}_rl60", w, exclude) <= 1.10] or [check]
        chk = min(ok, key=lambda x: mean(vals(ix, *DECIDE, f"rtc_{x}_rl60", "fail_rate", exclude)))
        verdict = f"KEEP a percentile start check: {chk}"
    flips = [k for k, x in cells.items() if k != DECIDE and (x["dF"][1] > 0) != (c["dF"][1] > 0)
             and (x["dF"][0] > 0 or x["dF"][2] < 0)]
    return {"verdict": verdict, "chk": chk, "c1": c1, "c2": c2, "guard": guard, "cells": cells, "flips": flips}


def r2(ix, chk):
    """R2: (smallest run limit meeting the rule, met?) for check `chk`, realistic/P2."""
    for rl in (30, 60, 120):
        hb = mean(vals(ix, "realistic", "base", "P2", f"rtc_{chk}_rl{rl}", "timeout_rate"))
        hl = mean(vals(ix, "realistic", "long", "P2", f"rtc_{chk}_rl{rl}", "timeout_rate"))
        if hb <= 1e-4 and hl <= 5e-4:
            return rl, True
    return 120, False


def r4(ix, chk):
    """R4: (check whose EP-09 arms are used, smallest EP-09 run limit within 0.01 points of 60 s, one-sided)."""
    if not ix[("realistic", "base", "P2", f"rtc_{chk}_ep09_rl60")]:
        chk = "p90"
    ref = {t: mean(vals(ix, "realistic", t, "P2", f"rtc_{chk}_ep09_rl60", "fail_rate")) for t in ("base", "long")}
    for r in (10, 30, 60):
        if all(mean(vals(ix, "realistic", t, "P2", f"rtc_{chk}_ep09_rl{r}", "fail_rate")) - ref[t] <= 0.0001
               for t in ("base", "long")):
            return chk, r
    return chk, 60


# ---------------------------------------------------------------- formatting
def pc(x, d=3):
    return "—" if x != x else f"{100 * x:.{d}f} %"


def ms(x):
    return "—" if x != x else (f"{1000 * x:.0f} ms" if x < 1 else f"{x:.2f} s")


def ci_s(ci, d=3):
    lo, m, hi = ci
    return f"{100 * m:+.{d}f} pp [{100 * lo:+.{d}f}, {100 * hi:+.{d}f}]"


def table(out, head, rows):
    out.append("| " + " | ".join(head) + " |")
    out.append("|" + " --- |" * len(head))
    for r in rows:
        out.append("| " + " | ".join(str(x) for x in r) + " |")
    out.append("")


def r1_section(out, res, title):
    out.append(title + "\n")
    rows = [[*k, ci_s(x["dF"]), ci_s(x["dbusy"]), ci_s(x["dab"]), f"{x['ratio']:.3f}", pc(x["late"]),
             pc(x["late30"], 4)] for k, x in res["cells"].items()]
    table(out, ["band", "tail", "dispatch", "ΔF check − wait", "Δ busy", "Δ abandoned",
                "ttd p99 ratio wait/check (median)", "wait: late / ok", "wait: > 30 s / calls"], rows)
    out.append(f"**Verdict:** {res['verdict']}. Condition 1 {'holds' if res['c1'] else 'fails'}, condition 2 "
               f"{'holds' if res['c2'] else 'fails'}, late-answer guard {'holds' if res['guard'] else 'fails'}. "
               "Cells pointing the other way: " + (", ".join("/".join(k) for k in res["flips"]) or "none") + ".\n")


def arms_table(out, ix, band, tail, disp, arms):
    head = ["arm", "ok", "busy", "abandoned", "fail", "busy with permit free", "ok p50", "ok p95", "ok p99",
            "ok p99.9", "late / ok", "> 30 s / calls", "goodput /s", "abandoned permit-s / 1k", "turns failed",
            "ttd p99"]
    rows = []
    for a in arms:
        if not ix[(band, tail, disp, a)]:
            continue

        def v(k):
            return vals(ix, band, tail, disp, a, k)
        rows.append([a, pc(mean(v("ok_rate"))), pc(mean(v("refuse_rate"))), pc(mean(v("timeout_rate"))),
                     pc(mean(v("fail_rate"))), pc(mean(v("rf_with_permit"))), ms(med(v("ok_p50"))),
                     ms(med(v("ok_p95"))), ms(med(v("ok_p99"))), ms(med(v("ok_p999"))), pc(mean(v("ok_late_share"))),
                     pc(mean(v("ok_late30_rate")), 4), f"{mean(v('goodput_per_s')):.2f}",
                     f"{mean(v('ab_permit_s_per_kcall')):.1f}", pc(mean(v("turn_failed"))), ms(med(v("ttd_p99")))])
    table(out, head, rows)


def main(path):
    rows = load(path)
    ix = index(rows)
    lk_path = os.path.join(os.path.dirname(path) or ".", "rtc_lockup.csv")
    n_lk = overlay(ix, load(lk_path), "rf_arrival_free") if os.path.exists(lk_path) else 0
    seen = {r["arm"] for r in rows}
    arms = [a for a, *_ in rtc.arms() if a in seen]
    bands = bands_of(ix)
    out = ["# Run-to-completion study tables (generated by rtc_analysis.py; PREREG-run-to-completion.md)", ""]
    out.append(f"{len(rows)} simulations; configurations per band: "
               + ", ".join(f"{b} {len(ix[(b, 'base', 'P2', 'old_p90')])}" for b in bands) + ".")
    out.append("Rates are means over configurations; latency percentiles are medians over configurations of the "
               "per-configuration value. pp = percentage points; intervals are 95 % paired bootstrap. "
               "F = fail rate = busy + abandoned.\n")

    out.append("## Arms by band (P2 unless marked)\n")
    crows = []
    for b in bands:
        for t in ("base", "long"):
            for a in COMPACT:
                def v(k, d="P2"):
                    return vals(ix, b, t, d, a, k)
                crows.append([f"{b}, {t}", f"`{a}`", pc(mean(v("fail_rate"))), pc(mean(v("refuse_rate"))),
                              pc(mean(v("timeout_rate"))), pc(mean(v("ok_late_share"))),
                              pc(mean(v("ok_late30_rate")), 4), ms(med(v("ok_p999"))), ms(med(v("ttd_p99"))),
                              pc(mean(v("fail_rate", "P0")))])
    table(out, ["band, tail", "arm", "fail", "busy", "abandoned", "late / ok", "> 30 s / calls", "ok p99.9",
                "ttd p99", "fail, P0"], crows)

    res = r1(ix)
    r1_section(out, res, "## R1. Start check: p90 against wait-only, run limit 60 s (preregistered)")
    chk = res["chk"]

    out.append("## Lock-ups (addendum): configurations refusing > 1 % of calls at arrival with a permit free\n")
    out.append(f"rf_arrival_free from the rerun of the four percentile arms ({n_lk} rows, every fail_rate "
               "reproduced); columns without it (wait-only, old p90) use rf_with_permit, which also counts "
               "grant-time sheds.\n")
    lk = []
    for b in bands:
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                n = len(ix[(b, t, d, "old_p90")])
                lk.append([b, t, d, *(f"{len(lockups(ix, b, t, d, f'rtc_{x}_rl60'))} / {n}"
                                      for x in ("p90", "p95", "p99", "p90cap", "wait")),
                           f"{len(lockups(ix, b, t, d, 'old_p90'))} / {n}"])
    table(out, ["band", "tail", "dispatch", "p90", "p95", "p99", "p90 capped", "wait-only", "old p90"], lk)
    n_locked = len(lockups(ix, *DECIDE, "rtc_p90_rl60"))
    r1_section(out, r1(ix, exclude=lambda cell: lockups(ix, *cell, "rtc_p90_rl60")),
               f"### R1 without each cell's locked configurations ({n_locked} in the deciding cell; exploratory)")
    r1_section(out, r1(ix, check="p90cap"), "### R1x: capped p90 against wait-only (exploratory)")

    rl, met = r2(ix, chk)
    out.append(f"## R2. Run limit: run-limit hits (abandoned after start) with the {chk} check\n")
    r2rows = []
    for b in bands:
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                hits = [mean(vals(ix, b, t, d, f"rtc_{chk}_rl{x}", "timeout_rate")) for x in (30, 60, 120)]
                r2rows.append([b, t, d, *(pc(h, 4) for h in hits),
                               *(f"{mean(vals(ix, b, t, d, f'rtc_{chk}_rl{x}', 'ab_permit_s_per_kcall')):.1f}"
                                 for x in (30, 60, 120))])
    table(out, ["band", "tail", "dispatch", "hits rl 30 s", "rl 60 s", "rl 120 s", "abandoned permit-s / 1k: 30 s",
                "60 s", "120 s"], r2rows)
    out.append(f"**R2 verdict:** run_limit_ms = {rl * 1000}" + ("" if met else " (no limit met the rule; reported "
                                                                "to the owner)") + ".\n")

    out.append("## Q1. Former timeouts under the old rule, and what run to completion makes of them\n")
    a = f"rtc_{chk}_rl{rl}"
    q1, hist = [], {}
    for b in bands:
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                q1.append([b, t, d, pc(mean(vals(ix, b, t, d, "old_p90", "timeout_rate"))),
                           pc(mean(vals(ix, b, t, d, "old_p90", "late_admit_timeout_rate"))),
                           pc(mean(vals(ix, b, t, d, "old_p90", "refuse_rate"))),
                           pc(mean(vals(ix, b, t, d, a, "ok_late_rate"))),
                           pc(mean(vals(ix, b, t, d, a, "late_start_ok_rate"))),
                           pc(mean(vals(ix, b, t, d, a, "timeout_rate")), 4),
                           pc(mean(vals(ix, b, t, d, a, "refuse_rate")))])
                h = [0] * len(LATE_KEYS)
                for r in ix[(b, t, d, a)].values():
                    for j, k in enumerate(LATE_KEYS):
                        h[j] += int(f(r[k]))
                hist[(b, t, d)] = h
    table(out, ["band", "tail", "dispatch", "old: abandoned", "old: late-admit abandoned", "old: busy",
                f"{a}: late ok / calls", "late-start ok", "run-limit hits", "busy"], q1)
    for h_arm in dict.fromkeys([a, f"rtc_{chk}_rl60"]):
        hrows = []
        for (b, t, d) in hist:
            h = [0] * len(LATE_KEYS)
            for r in ix[(b, t, d, h_arm)].values():
                for j, k in enumerate(LATE_KEYS):
                    h[j] += int(f(r[k]))
            hrows.append([b, t, d, *h])
        out.append(f"Lateness of late successes, {h_arm} (pooled counts over configurations):\n")
        table(out, ["band", "tail", "dispatch", "(10,12] s", "(12,15]", "(15,20]", "(20,30]", "(30,60]", "(60,120]",
                    "> 120"], hrows)

    out.append(f"## R3. Queue depth and head-of-line: old_p90 against {a}\n")
    r3rows, changed = [], []
    for b in bands:
        for t in ("base", "long"):
            dq = paired_ci(*pair(ix, b, t, "P2", a, "old_p90", "rf_queue_full"))
            mq = [mean([1.0 if x >= 32 else 0.0 for x in vals(ix, b, t, "P2", arm, "max_queue")])
                  for arm in ("old_p90", a)]
            ex, lat = {}, {}
            for arm in ("old_p90", a):
                x0, x2 = pair(ix, b, t, "P0", arm, arm, "fail_rate")[0], pair(ix, b, t, "P2", arm, arm, "fail_rate")[0]
                ex[arm] = paired_ci(x0, x2)
                l0 = pair(ix, b, t, "P0", arm, arm, "ok_p99")[0]
                l2 = pair(ix, b, t, "P2", arm, arm, "ok_p99")[0]
                lat[arm] = med([p - q for p, q in zip(l0, l2) if p == p and q == q])
            ch = (abs(dq[1]) > 0.001 or mq[1] - mq[0] > 0.01 or abs(ex[a][1] - ex["old_p90"][1]) > 0.0005
                  or (lat["old_p90"] and abs(lat[a] - lat["old_p90"]) > 0.25 * abs(lat["old_p90"])))
            if ch:
                changed.append(f"{b}/{t}")
            r3rows.append([b, t, ci_s(dq), f"{pc(mq[0], 1)} / {pc(mq[1], 1)}", ci_s(ex["old_p90"]), ci_s(ex[a]),
                           ms(lat["old_p90"]), ms(lat[a])])
    table(out, ["band", "tail", "Δ queue_full (rtc − old, P2)", "max_queue ≥ 32: old / rtc (P2)",
                "P0 − P2 fail: old", "P0 − P2 fail: rtc", "P0 − P2 ok p99: old", "P0 − P2 ok p99: rtc"], r3rows)
    out.append("**R3 verdict:** run to completion changes the depth or head-of-line result in: "
               + (", ".join(changed) if changed else "no band or tail") + ".\n")

    c4, pick = r4(ix, chk)
    out.append(f"## R4. EP-09: the {c4} check with an interrupting run limit\n")
    r4rows = []
    for b in bands:
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                for arm in (f"rtc_{c4}_rl60", f"rtc_{c4}_ep09_rl10", f"rtc_{c4}_ep09_rl30", f"rtc_{c4}_ep09_rl60",
                            "old_p90_ep09"):
                    def v(k):
                        return vals(ix, b, t, d, arm, k)
                    r4rows.append([b, t, d, arm, pc(mean(v("fail_rate"))), pc(mean(v("refuse_rate"))),
                                   pc(mean(v("timeout_rate"))), pc(mean(v("ok_late_share"))), ms(med(v("ok_p99"))),
                                   ms(med(v("ok_p999"))), f"{mean(v('ab_permit_s_per_kcall')):.1f}"])
    table(out, ["band", "tail", "dispatch", "arm", "fail", "busy", "interrupted", "late / ok", "ok p99", "ok p99.9",
                "abandoned permit-s / 1k"], r4rows)
    out.append(f"**R4 verdict:** with EP-09, run_limit_ms can come down to {pick * 1000} ({c4} check).\n")

    out.append("## All arms\n")
    for b in bands:
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                out.append(f"### {b}, {t} tail, {d}\n")
                arms_table(out, ix, b, t, d, arms)
    open(os.path.join(RES, "rtc_tables.md"), "w").write("\n".join(out))
    print("\n".join(x for x in out if x.startswith("**")))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(RES, "rtc.csv"))
