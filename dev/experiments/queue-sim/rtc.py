"""Run-to-completion study (PREREG-run-to-completion.md). usage: python rtc.py [n_real] [n_stress_scan]

Writes results/rtc.jsonl (streamed, resumable) and results/rtc.csv; progress and errors go to results/rtc.log.
"""
import math
import os
import random
import sys
from dataclasses import replace

import fuzz
from runner import run_jobs

HERE = os.path.dirname(os.path.abspath(__file__))
CHECKS = {"p90": ("fifo_shed", ("pct", 90, 128, 0.0)), "p95": ("fifo_shed", ("pct", 95, 128, 0.0)),
          "p99": ("fifo_shed", ("pct", 99, 128, 0.0)), "wait": ("fifo", ("pct", 90, 128, 0.0))}


def arms():
    """(arm, check, run_limit, interrupt). 'old' = the deadline also ends a running call."""
    yield "old_p90", "p90", None, False
    yield "old_p90_ep09", "p90", None, True
    for chk in ("p90", "p95", "p99", "wait"):
        for rl in (30.0, 60.0, 120.0):
            yield f"rtc_{chk}_rl{int(rl)}", chk, rl, False
    for chk in ("p90", "wait"):
        for rl in (10.0, 30.0, 60.0):
            yield f"rtc_{chk}_ep09_rl{int(rl)}", chk, rl, True


def bands(n_real, n_scan):
    """Realistic band = fuzz.real_config (REPORT § 7); stress band = main-fuzz configs with K >= 16, slow >= 2 %."""
    for i in range(n_real):
        yield "realistic", i, fuzz.real_config(i)
    for i in range(n_scan):
        c = fuzz.sample_config(i)
        if c.K >= 16 and c.slow_p >= 0.02:
            yield "stress", i, c


def long_tail(cfg, i):
    """Same slow-read probability, magnitude 10-30 s: median log-uniform 10-30 s, sigma 0.25."""
    r = random.Random(500_000 + cfg.seed)
    return replace(cfg, slow_median=math.exp(r.uniform(math.log(10.0), math.log(30.0))), slow_sigma=0.25)


def jobs(n_real=500, n_scan=2000):
    for band, i, base in bands(n_real, n_scan):
        base = replace(base, queue_limit=32)
        for tail in ("base", "long"):
            cfg0 = base if tail == "base" else long_tail(base, i)
            for disp in ("P0", "P2"):
                for arm, chk, rl, intr in arms():
                    adm, th = CHECKS[chk]
                    cfg = replace(cfg0, admission=adm, threshold=th, run_limit=rl, interrupt=intr, dispatch=disp)
                    yield cfg, {"band": band, "cfg_i": i, "tail": tail, "dispatch": disp, "arm": arm, "check": chk,
                                "run_limit": rl if rl is not None else "", "ep09": intr}


if __name__ == "__main__":
    n_real = int(sys.argv[1]) if len(sys.argv) > 1 else 500
    n_scan = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
    _, n_err = run_jobs("rtc", fuzz.one, jobs(n_real, n_scan), os.path.join(HERE, "results", "rtc"),
                        os.path.join(HERE, "results", "rtc.log"), every=1000)
    sys.exit(1 if n_err else 0)
