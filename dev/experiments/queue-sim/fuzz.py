"""Fuzz driver. usage: python fuzz.py main|thresh|incident|step  (writes results/*.csv)"""
import csv, math, random, sys, os
from dataclasses import asdict, replace
from multiprocessing import Pool
from sim import Config, ServiceModel, simulate

HERE = os.path.dirname(os.path.abspath(__file__))
SVC = ServiceModel.from_csv(os.path.join(HERE, "results/service_times.csv"))
CFG_KEYS = ["seed", "arrivals", "K", "open_rate", "think_median", "think_sigma", "session_prefix", "session_turns", "cap", "queue_limit", "hybrid_frac", "slow_p",
            "slow_median", "slow_sigma", "speed_mult", "writer_period", "writer_hold", "projection_workers", "projection_batch_s"]


def sample_config(i, n_calls=20000):
    """Fuzz ranges. Grounding (see REPORT.md section 2): think time lognormal median 3-5 s, sigma 0.55-1.1
    (incident transcripts: median 3.3 s; TraceLab Claude Code/Codex steps: p50 4.9 s, p90 20 s);
    fan-out either TraceLab-like (mean 1.2 calls/turn, p99 4) or prompted-parallel as in the incident
    transcripts (one F-call fan-out per agent session, then 1-3 call turns); slow reads at
    0.2-8 s with p up to 5% (grounded rare mode p 1e-3..1e-2 at 100-1000x median; above 1% is stress)."""
    r = random.Random(900_000 + i)
    K = r.randint(1, 32)
    cap = r.choice([4, 7])
    arrivals = "open" if r.random() < 0.1 else "closed"
    writer = r.random() < 0.5
    hold = math.exp(r.uniform(math.log(0.001), math.log(0.5)))
    duty = r.uniform(0.01, 0.8)  # writer mutex duty cycle; >1 would be an unstable writer
    think = r.uniform(3.0, 5.0)
    sig = r.uniform(0.55, 1.1)
    fan = r.random() < 0.5
    F = r.randint(4, 12)
    proj = r.choice([0, 0, 0, 0, 1, 2])  # background projection embed batches (hazard C); 2 = engine default workers
    proj_s = math.exp(r.uniform(math.log(0.05), math.log(2.0)))
    return Config(
        seed=i, K=K, arrivals=arrivals, open_rate=K * r.uniform(0.15, 0.6), think_median=think, think_sigma=sig,
        honor_retry_after=False,  # agents re-issue on their next LLM turn; they cannot sleep for retry_after_ms
        batch_sizes=(1, 2, 3) if fan else (1, 2, 3, 4), batch_weights=(13, 3, 9) if fan else (85, 10, 4, 1),
        session_prefix=(F,) if fan else (), session_turns=r.randint(3, 10) if fan else 0,
        cap=cap, queue_limit=r.choice([2 * cap, None]),
        hybrid_frac=r.random(), slow_p=r.uniform(0, 0.05),
        slow_median=math.exp(r.uniform(math.log(0.2), math.log(8.0))), slow_sigma=r.uniform(0.25, 1.5),
        speed_mult=1.0 if r.random() < 0.7 else r.uniform(5, 20),
        writer_period=(hold * math.exp(0.5) / duty) if writer else None,
        writer_hold=hold, projection_workers=proj, projection_batch_s=proj_s,
        n_calls=n_calls)


def real_config(i):
    """Dedicated realistic band: K<=8, slow p log-uniform 1e-3..1e-2, GPU service times."""
    r = random.Random(700_000 + i)
    c = sample_config(100_000 + i)
    return replace(c, seed=100_000 + i, K=r.randint(1, 8), slow_p=math.exp(r.uniform(math.log(1e-3), math.log(1e-2))),
                   speed_mult=1.0, arrivals="closed")


def one(args):
    cfg, tag = args
    m = simulate(cfg, SVC).metrics()
    row = {k: getattr(cfg, k) for k in CFG_KEYS}
    row["writer_duty"] = (cfg.writer_hold * math.exp(0.5) / cfg.writer_period) if cfg.writer_period else 0.0
    row.update(tag)
    row.update(m)
    return row


def write(rows, name):
    with open(os.path.join(HERE, "results", name), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)


SHED = ("ewma", 0.2, 1.0, 0.05)  # need = max(50 ms, ewma of this tool's service time)
ARMS = [("refuse", "P0", False), ("refuse", "P1", False), ("refuse", "P2", False),
        ("fifo", "P0", False), ("fifo", "P1", False),
        ("fifo_shed", "P0", False), ("fifo_shed", "P1", False), ("fifo_shed", "P2", False),
        ("fifo_predict", "P0", False), ("fifo_predict", "P1", False),
        ("fifo_shed", "P0", True), ("fifo_shed", "P1", True),
        ("fifo_predict", "P0", True), ("fifo_predict", "P1", True)]
THRESH = ([("T0", "fifo", ("fixed", 0.0))] + [(f"T1_F{int(F*1000)}", "fifo_shed", ("fixed", F)) for F in (0.05, 0.25, 1.0)]
          + [(f"T2_p{p}_n{n}", "fifo_shed", ("pct", p, n, 0.0)) for p in (5, 50, 90, 95) for n in (32, 128, 512)]
          + [(f"T3_a{a}_k{k}", "fifo_shed", ("ewma", a, k, 0.0)) for a in (0.1, 0.3) for k in (1, 2)])


def main_jobs(n, real=False):
    for i in range(n):
        base = real_config(i) if real else sample_config(i)
        for adm, disp, intr in ARMS:
            yield replace(base, admission=adm, dispatch=disp, interrupt=intr, threshold=SHED, pred=("pct", 90, 128)), \
                {"admission": adm, "dispatch": disp, "interrupt": intr}


PRED = [(f"A_s_{nm}", ("ewma", 0.2) if nm == "ewma" else ("pct", int(nm[1:]), 128)) for nm in ("ewma", "p90", "p99")]


def thresh_jobs(n):
    for i in range(n):
        base = replace(sample_config(i), queue_limit=None, honor_retry_after=True)
        for disp in ("P0", "P1"):
            for name, adm, th in THRESH:
                yield replace(base, admission=adm, threshold=th, dispatch=disp, queue_limit=2 * base.cap), \
                    {"policy": name, "dispatch": disp, "deadline": "fixed10", "queue": "2cap"}
            for dname, d in (("derived_k3", ("derived", 1.0, 3.0)), ("derived_k10", ("derived", 1.0, 10.0))):
                for name, th in (("T1_F250", ("fixed", 0.25)), ("T2_p90_n128", ("pct", 90, 128, 0.0))):
                    yield replace(base, admission="fifo_shed", threshold=th, deadline=d, dispatch=disp,
                                  queue_limit=2 * base.cap), {"policy": name, "dispatch": disp, "deadline": dname, "queue": "2cap"}
            # research section 7: arrival test with backstop depth 32; need = p90 window 128; s varies
            for name, pr in PRED:
                yield replace(base, admission="fifo_predict", threshold=("pct", 90, 128, 0.0), pred=pr, dispatch=disp,
                              queue_limit=32), {"policy": name, "dispatch": disp, "deadline": "fixed10", "queue": "32"}


def incident_jobs():
    for batch in (1, 2, 3, 4, 5, 6):
        for think in (0.5, 1.5, 3.0):
            for sync in (False, True):
                for slow in (0.0, 0.01):
                  for adm in ("refuse", "fifo_shed"):
                    for s in range(200):
                        cfg = Config(seed=10_000 + s, K=6, cap=4, admission=adm, dispatch="P0", batch_max=batch,
                                     think_median=think, sync_start=sync, hybrid_frac=1.0, slow_p=slow, slow_median=2.0,
                                     n_calls=125, warmup_frac=0.0)
                        yield cfg, {"think_median": think, "sync": sync, "rep": s, "admission": adm}


if __name__ == "__main__":
    from runner import run_jobs
    what = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
    jobs = {"main": lambda: main_jobs(n), "real": lambda: main_jobs(n, True), "thresh": lambda: thresh_jobs(n)}[what]()
    rows, n_err = run_jobs(f"fuzz {what}", one, jobs, os.path.join(HERE, "results", f"fuzz_{what}"),
                           os.path.join(HERE, "results", "fuzz2.log"), every=500)
    sys.exit(1 if n_err else 0)
