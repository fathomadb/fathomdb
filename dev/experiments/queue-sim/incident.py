"""Incident reproduction from the six Claude-subagent transcripts (results/incident-transcripts/).

Ground truth: 97 MCP calls in 31 turns; 29 busy, all in five 10-call fan-out turns (6 busy + 4 ok each,
one 5 busy). Turn sizes 1x14, 2x3, 3x9, 10x5. Inter-turn gap median ~3.3 s (1.5-14.5 s).
"""
import csv, os, statistics as st
from collections import Counter, defaultdict
from multiprocessing import Pool
from sim import Config, ServiceModel, simulate

HERE = os.path.dirname(os.path.abspath(__file__))
SVC = ServiceModel.from_csv(os.path.join(HERE, "results/service_times.csv"))
GROUND = dict(K=6, cap=4, dispatch="P0", hybrid_frac=1.0, think_median=3.3, think_sigma=0.55,
              session_prefix=(10,), session_turns=5, batch_sizes=(1, 2, 3), batch_weights=(13, 3, 9),
              sync_start=True, start_jitter=3.5, n_calls=97, warmup_frac=0.0)


def run(args):
    adm, ql, cap, seed, prefix = args
    cfg = Config(seed=seed, **{**GROUND, "admission": adm, "queue_limit": ql, "cap": cap, "session_prefix": prefix})
    s = simulate(cfg, SVC)
    cs = [c for c in s.calls if c.outcome]
    bursts = defaultdict(list)
    for c in cs:
        bursts[(c.cid, round(c.t_arr, 9))].append(c)
    per_burst = Counter()
    cross = 0
    for (cid, t), v in bursts.items():
        nb = sum(c.outcome == "refused" for c in v)
        per_burst[(len(v), nb)] += 1
    # cross-agent overlap: a refused call while the other in-flight calls belong to other agents
    big = [v for v in bursts.values() if len(v) >= 10]
    big_ref = sum(sum(c.outcome == "refused" for c in v) for v in big)
    tot_ref = sum(c.outcome == "refused" for c in cs)
    lat = sorted(c.t_res - c.t_arr for c in cs if c.outcome == "ok")
    return {"admission": adm, "queue_limit": ql, "cap": cap, "seed": seed, "prefix": str(prefix), "n": len(cs),
            "refused": tot_ref, "refused_in_fanout": big_ref, "timeouts": sum(c.outcome == "timeout" for c in cs),
            "fanout_turns": len(big), "per_burst": str(dict(per_burst)),
            "ok_p99_ms": 1000 * lat[int(0.99 * len(lat))] if lat else float("nan")}


if __name__ == "__main__":
    jobs = [(adm, ql, cap, s, pre) for adm, ql in (("refuse", None), ("fifo_shed", 8), ("fifo_shed", None))
            for cap in (4, 7) for pre in ((10,), (1, 10)) for s in range(300)]
    from runner import run_jobs
    rows, n_err = run_jobs("incident", run, jobs, os.path.join(HERE, "results", "incident"),
                           os.path.join(HERE, "results", "fuzz2.log"), every=500)
    for r in rows:
        r["per_burst"] = eval(r["per_burst"]) if isinstance(r["per_burst"], str) else r["per_burst"]
    g = defaultdict(list)
    for r in rows:
        g[(r["admission"], r["queue_limit"], r["cap"], r["prefix"])].append(r)
    for k, v in g.items():
        ref = [r["refused"] / r["n"] for r in v]
        fo = sum(r["refused_in_fanout"] for r in v) / max(1, sum(r["refused"] for r in v))
        pb = Counter()
        for r in v:
            for kk, n in r["per_burst"].items():
                if kk[0] >= 10:
                    pb[kk[1]] += n
        print(k, f"refused {100*st.mean(ref):.1f}% [p10 {100*sorted(ref)[30]:.1f} p90 {100*sorted(ref)[270]:.1f}]",
              f"share in fan-out {100*fo:.0f}%", "busy per 10-burst", dict(sorted(pb.items())),
              f"timeouts {sum(r['timeouts'] for r in v)}", f"ok p99 {st.median(r['ok_p99_ms'] for r in v):.1f} ms")
