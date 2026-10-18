"""Step-shift stability: 10x slowdown mid-run; how fast does each admission threshold adapt, does it oscillate."""
import csv, os, statistics as st
from multiprocessing import Pool
from sim import Config, ServiceModel, simulate, KINDS

HERE = os.path.dirname(os.path.abspath(__file__))
SVC = ServiceModel.from_csv(os.path.join(HERE, "results/service_times.csv"))
T_STEP = 4000.0
POL = [("T0", "fifo", ("fixed", 0.0)), ("T1_F250", "fifo_shed", ("fixed", 0.25)), ("T1_F1000", "fifo_shed", ("fixed", 1.0))] + \
      [(f"T2_p{p}_n{n}", "fifo_shed", ("pct", p, n, 0.0)) for p in (5, 50, 90, 95) for n in (32, 128, 512)] + \
      [(f"T3_a{a}_k{k}", "fifo_shed", ("ewma", a, k, 0.0)) for a in (0.1, 0.3) for k in (1, 2)]
GROUNDED = dict(think_median=3.3, think_sigma=0.55, session_prefix=(10,), session_turns=5,
                batch_sizes=(1, 2, 3), batch_weights=(13, 3, 9))
SCEN = {"cpu_k8": dict(K=8, cap=4, speed_mult=10.0, slow_p=0.02, slow_median=0.5, slow_sigma=0.8, hybrid_frac=0.5, **GROUNDED),
        "cpu_k16": dict(K=16, cap=4, speed_mult=10.0, slow_p=0.02, slow_median=0.5, slow_sigma=0.8, hybrid_frac=0.5, **GROUNDED)}


def run(args):
    scen, disp, seed, (name, adm, th) = args
    cfg = Config(seed=seed, n_calls=10**9, t_stop=3 * T_STEP, admission=adm, threshold=th, dispatch=disp, queue_limit=8,
                 step=(T_STEP, 10.0), trace_need=True, **SCEN[scen])
    s = simulate(cfg, SVC)
    tr = [(t, n["search_text"]) for t, n, _ in s.need_trace]
    post = [v for t, v in tr if t > T_STEP]
    steady = st.median(post[len(post) // 2:]) if post else float("nan")
    pre = st.median([v for t, v in tr if T_STEP - 1000 < t < T_STEP])
    adapt = next((t - T_STEP for t, v in tr if t > T_STEP and v >= 0.9 * steady), float("nan"))
    late = [v for t, v in tr if t > T_STEP + 1000]
    cv = st.pstdev(late) / st.mean(late) if late and st.mean(late) > 0 else 0.0
    def win(a, b):
        cs = [c for c in s.calls if c.outcome and a <= c.t_arr < b]
        n = max(1, len(cs))
        to = sum(c.outcome == "timeout" for c in cs) / n
        rf = sum(c.outcome == "refused" for c in cs) / n
        fs = sum(c.outcome == "refused" and c.reason == "deadline" and c.reader + c.embed <= c.remaining_at_refuse for c in cs) / n
        la = sum(c.outcome == "timeout" and c.t_adm - c.t_arr > 0 for c in cs) / n
        okl = sorted(c.t_res - c.t_arr for c in cs if c.outcome == "ok")
        p95 = okl[int(0.95 * len(okl))] if okl else float("nan")
        return to, rf, fs, la, p95
    tmax = max(c.t_arr for c in s.calls)
    r = {"scenario": scen, "dispatch": disp, "seed": seed, "policy": name, "need_pre": pre, "need_post": steady,
         "adapt_s": adapt, "need_cv_late": cv}
    for lab, (a, b) in {"pre": (1000, T_STEP), "first300": (T_STEP, T_STEP + 300), "post": (T_STEP + 1000, tmax)}.items():
        for k, v in zip(("to", "rf", "fs", "late", "p95"), win(a, b)):
            r[f"{lab}_{k}"] = v
    return r


if __name__ == "__main__":
    import sys
    from runner import run_jobs
    jobs = [(sc, d, sd, p) for sc in SCEN for d in ("P0", "P1") for sd in range(5) for p in POL]
    rows, n_err = run_jobs("step", run, jobs, os.path.join(HERE, "results", "step"),
                           os.path.join(HERE, "results", "fuzz2.log"), every=50)
    sys.exit(1 if n_err else 0)
