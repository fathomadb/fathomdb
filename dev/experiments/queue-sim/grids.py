"""Targeted grids for queue-manager-research.md section 9 inputs 4, 5 and 7.

A depth:     cap x queue depth x agents, grounded fan-out (10-call turn per agent session), depth-only vs arrival test
B overload:  clients with no think time firing 10-call bursts, arrival test vs depth-only shedding
C hol:       one slow read injected every 2 s; round-robin vs shared queue, with and without EP-09 interrupt
"""
import csv, os, sys
from dataclasses import replace
from multiprocessing import Pool
from sim import Config, ServiceModel, Sim, pctl

HERE = os.path.dirname(os.path.abspath(__file__))
SVC = ServiceModel.from_csv(os.path.join(HERE, "results/service_times.csv"))
GROUND = dict(think_median=3.3, think_sigma=0.55, session_prefix=(10,), session_turns=5,
              batch_sizes=(1, 2, 3), batch_weights=(13, 3, 9), honor_retry_after=False,
              threshold=("ewma", 0.2, 1.0, 0.05), pred=("pct", 90, 128))


def run(job):
    grid, tag, cfg = job
    s = Sim(cfg, SVC).run()
    m = s.metrics()
    others = [c for c in s.calls[int(len(s.calls) * cfg.warmup_frac):] if c.outcome and c.cid != -3]
    ok = [c.t_res - c.t_arr for c in others if c.outcome == "ok"]
    T = max(s.T, 1e-9)
    row = {"grid": grid, **tag, "seed": cfg.seed}
    row.update({k: m[k] for k in ("refuse_rate", "timeout_rate", "false_refusal_rate", "late_admit_timeout_rate", "max_queue",
                                  "burst_any_refused", "burst_done_p50", "burst_done_p99", "lone_wait_p99", "lone_wait_max",
                                  "turn_refused", "abandoned_avg", "rf_predicted", "rf_queue_full", "rf_deadline")})
    row.update({"others_ok_p50": pctl(ok, 50), "others_ok_p95": pctl(ok, 95), "others_ok_p99": pctl(ok, 99),
                "others_timeout": sum(c.outcome == "timeout" for c in others) / max(1, len(others)),
                "others_refused": sum(c.outcome == "refused" for c in others) / max(1, len(others)),
                "goodput_per_s": sum(c.outcome == "ok" for c in others) / T})
    return row


def jobs():
    for seed in range(3):
        for cap in (4, 7):
            for depth in (0, 8, 16, 32, 64, None):
                for K in (1, 2, 6, 12, 20):
                    for hyb in (0.0, 1.0):
                        for adm in ("fifo_shed", "fifo_predict"):
                            if depth == 0 and adm == "fifo_predict":
                                continue
                            cfg = Config(seed=seed, K=K, cap=cap, admission=adm, queue_limit=depth, dispatch="P0",
                                         hybrid_frac=hyb, n_calls=20000, **GROUND)
                            yield "A_depth", {"cap": cap, "depth": depth, "K": K, "hybrid": hyb, "admission": adm}, cfg
    for seed in range(3):
        for K in (1, 2, 4):
            for hyb in (0.0, 1.0):
                for slow in (0.0, 0.01):
                    for name, adm, ql, pr in (("refuse", "refuse", None, ("ewma", 0.2)),
                                              ("depth8", "fifo_shed", 8, ("ewma", 0.2)),
                                              ("depth32", "fifo_shed", 32, ("ewma", 0.2)),
                                              ("unbounded", "fifo_shed", None, ("ewma", 0.2)),
                                              ("arrival_ewma_d32", "fifo_predict", 32, ("ewma", 0.2)),
                                              ("arrival_p90_d32", "fifo_predict", 32, ("pct", 90, 128)),
                                              ("arrival_p99_d32", "fifo_predict", 32, ("pct", 99, 128))):
                        cfg = Config(seed=seed, K=K, cap=4, admission=adm, queue_limit=ql, pred=pr, dispatch="P0",
                                     hybrid_frac=hyb, slow_p=slow, slow_median=2.0, slow_sigma=0.8,
                                     think_median=0.001, think_sigma=0.1, batch_sizes=(10,), batch_weights=(1,),
                                     honor_retry_after=False, threshold=("ewma", 0.2, 1.0, 0.05), n_calls=20000)
                        yield "B_overload", {"K": K, "hybrid": hyb, "slow_p": slow, "policy": name}, cfg
    for seed in range(3):
        for workload in ("capacity", "agents"):
            for cap in (4, 7):
                for dur in (1.0, 3.0, 10.0):
                    for disp in ("P0", "P1"):
                        for intr in (False, True):
                            if workload == "capacity":
                                w = dict(K=cap, think_median=0.001, think_sigma=0.1, hybrid_frac=0.0, honor_retry_after=False,
                                         threshold=("ewma", 0.2, 1.0, 0.05))
                            else:
                                w = dict(K=6, hybrid_frac=0.5, **GROUND)
                            cfg = Config(seed=seed, cap=cap, admission="fifo_shed", queue_limit=32, dispatch=disp, interrupt=intr,
                                         slow_injector=(2.0, dur), n_calls=20000 if workload == "agents" else 60000, **w)
                            yield "C_hol", {"workload": workload, "cap": cap, "slow_s": dur, "dispatch": disp, "interrupt": intr}, cfg


if __name__ == "__main__":
    from runner import run_jobs
    J = list(jobs())
    n_err = 0
    for g in ("A_depth", "B_overload", "C_hol"):
        _, e = run_jobs(f"grid {g}", run, [j for j in J if j[0] == g], os.path.join(HERE, "results", f"grid_{g}"),
                        os.path.join(HERE, "results", "fuzz2.log"), every=100)
        n_err += e
    sys.exit(1 if n_err else 0)
