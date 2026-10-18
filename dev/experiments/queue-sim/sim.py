"""Discrete-event simulator: agents -> MCP admission -> embed -> writer mutex -> reader dispatch -> readers.

Pure Python (heapq + random); no simpy, no numpy. Seeded; common random numbers across policies:
each client's i-th call has the same type and service times whatever the policy, because call
attributes, think times and the writer each draw from their own seeded stream.
"""
from __future__ import annotations

import csv
import heapq
import math
import random
from collections import deque
from dataclasses import dataclass, field

KINDS = ("search_text", "search_hybrid", "fetch", "expand")
LATE_BINS = ((10, 12), (12, 15), (15, 20), (20, 30), (30, 60), (60, 120), (120, "inf"))  # latency of late successes, s


# --------------------------------------------------------------------------- service model
class ServiceModel:
    """Empirical per-type service samples in seconds (from measure.py output)."""

    def __init__(self, samples: dict[str, list[float]]):
        self.s = {k: list(v) for k, v in samples.items()}
        med = {k: sorted(v)[len(v) // 2] for k, v in self.s.items()}
        self.text_med = {10: med["search_text_10"], 100: med["search_text_100"]}

    @classmethod
    def from_csv(cls, path):
        d: dict[str, list[float]] = {}
        for r in csv.DictReader(open(path)):
            d.setdefault(r["type"], []).append(float(r["ms"]) / 1000.0)
        return cls(d)

    def pick(self, name, u):
        v = self.s[name]
        return v[min(len(v) - 1, int(u * len(v)))]


def default_sampler(cfg, svc: ServiceModel):
    """Return f(rng) -> (kind, embed_s, reader_s, slow). Draws a fixed number of variates (CRN)."""
    lsm = math.log(cfg.slow_median)

    def f(r: random.Random):
        u_t, u_l, u_h, u_a, u_b, u_s, g = r.random(), r.random(), r.random(), r.random(), r.random(), r.random(), r.gauss(0, 1)
        m = cfg.speed_mult
        embed = 0.0
        if u_t < 0.65:
            lim = 100 if u_l < 0.2 else 10
            reader = svc.pick(f"search_text_{lim}", u_a)
            if u_h < cfg.hybrid_frac:
                kind = "search_hybrid"
                embed = max(0.0003, svc.pick(f"search_hybrid_{lim}", u_b) - svc.text_med[lim])
            else:
                kind = "search_text"
        elif u_t < 0.90:
            kind, reader = "fetch", svc.pick("fetch_10", u_a)
        else:
            kind, reader = "expand", svc.pick("expand_d3", u_a)
        slow = u_s < cfg.slow_p
        if slow:
            reader = math.exp(lsm + cfg.slow_sigma * g)  # slow read: big expand, cold cache, large hybrid
        return kind, embed * m, reader * (m if not slow else 1.0), slow

    return f


# --------------------------------------------------------------------------- config
@dataclass
class Config:
    seed: int = 1
    # clients
    arrivals: str = "closed"  # closed | open | script
    K: int = 6
    open_rate: float = 4.0  # calls/s for open loop
    think_median: float = 1.5
    think_sigma: float = 0.8
    batch_max: int = 1  # parallel tool calls per LLM turn: uniform 1..batch_max (used if batch_sizes empty)
    # grounded fan-out: turn sizes drawn from (sizes, weights); each agent session starts with
    # session_prefix turns (incident transcripts: one serial search, then a 10-call fan-out) and
    # restarts after session_turns turns (a new subagent takes over)
    batch_sizes: tuple = ()
    batch_weights: tuple = ()
    session_prefix: tuple = ()
    session_turns: int = 0
    sync_start: bool = False
    start_jitter: float = 0.0  # with sync_start: agents start uniformly within this many seconds
    honor_retry_after: bool = True
    script: list = field(default_factory=list)  # [(t, kind, embed, reader)] for tests
    # admission
    cap: int = 4
    admission: str = "refuse"  # refuse | fifo | fifo_shed | fifo_predict
    # fifo_predict (queue-manager-research.md section 7): on arrival with no free permit, refuse at once if
    # ceil((q+1)/c_eff) * s > remaining - need, s from pred: ("ewma", alpha) | ("pct", p, n); then as fifo_shed
    pred: tuple = ("ewma", 0.2)
    queue_limit: int | None = None
    threshold: tuple = ("ewma", 0.2, 1.0, 0.05)  # fixed F | pct p n floor | ewma a k floor
    deadline: tuple = ("fixed", 10.0)  # fixed D | derived floor k
    run_limit: float | None = None  # None: the deadline also ends a running call (old rule). Else run to completion:
    #                                 the deadline bounds waiting and starting; a started call is abandoned (or,
    #                                 with interrupt, cancelled) only once it has held its permit this long
    # engine
    dispatch: str = "P0"  # P0 round-robin | P1 shared FIFO | P2 try_send idle then RR
    interrupt: bool = False  # EP-09: at the deadline the caller stops waiting, frees its permit, queued reader
    #                          requests are dropped and a running read is sqlite3_interrupt-ed (worker freed);
    #                          a running embed forward is not interruptible and finishes on its worker
    projection_workers: int = 0  # background projection embed submitters sharing the embed FIFO (hazard C)
    projection_batch_s: float = 1.0  # embed time of one 64-text projection batch
    slow_injector: tuple | None = None  # (period_s, duration_s): one slow text read admitted every period (HOL probe)
    horizon_stop: bool = False  # script mode: stop background actors once every scripted call is resolved
    n_readers: int = 8
    embed_workers: int = 5
    embed_capacity: int = 20
    embed_timeout: float = 30.0
    gpu_serial: bool = True  # every embed forward needs the one GPU (or the shared rayon pool on CPU): FIFO, 1 server
    gpu_contention: float = 1.8  # a forward started while other embeds are in flight runs this much slower
    #                              (live probe: 5.2 ms solo, ~9.5 ms each under concurrency, results/burst_probe.csv)
    mutex_hold: float = 0.0001
    writer_period: float | None = None  # mean seconds between writer mutex holds
    writer_hold: float = 0.05  # median hold, lognormal sigma 1
    writer_script: list = field(default_factory=list)  # [(t, hold)] for tests
    # workload
    hybrid_frac: float = 0.5
    slow_p: float = 0.0
    slow_median: float = 1.0
    slow_sigma: float = 1.0
    speed_mult: float = 1.0
    step: tuple | None = None  # (t_step, factor): calls created at/after t_step are factor x slower
    # run
    n_calls: int = 20000
    t_stop: float | None = None  # also stop issuing new calls at this simulated time
    warmup_frac: float = 0.02
    trace_need: bool = False


class Call:
    __slots__ = ("id", "cid", "kind", "embed", "reader", "slow", "t_arr", "deadline", "state", "t_adm",
                 "t_done", "timed_out", "hol", "t_rq", "t_rs", "embed_wait", "mutex_wait", "soft",
                 "outcome", "reason", "t_res", "need_at_refuse", "remaining_at_refuse", "advice", "hybrid",
                 "cancelled", "proj", "q_ahead")

    def __init__(self, i, cid, kind, embed, reader, slow):
        self.id, self.cid, self.kind, self.embed, self.reader, self.slow = i, cid, kind, embed, reader, slow
        self.hybrid = kind == "search_hybrid"
        self.state = "new"
        self.t_adm = self.t_done = self.t_rq = self.t_rs = None
        self.timed_out = self.soft = False
        self.hol = self.embed_wait = self.mutex_wait = 0.0
        self.outcome = self.reason = None
        self.advice = None
        self.cancelled = False
        self.q_ahead = None
        self.proj = False


class Estimator:
    """Per-kind service-time estimate from completed, on-time calls (permit acquire -> release)."""

    def __init__(self, n=512, alpha=0.2):
        self.win = deque(maxlen=n)
        self.alpha = alpha
        self.ewma = None
        self.count = 0
        self._sorted = None

    def add(self, x):
        self.win.append(x)
        self.count += 1
        self.ewma = x if self.ewma is None else self.ewma + self.alpha * (x - self.ewma)
        self._sorted = None

    def pct(self, p):
        if self._sorted is None:
            self._sorted = sorted(self.win)
        s = self._sorted
        return s[min(len(s) - 1, int(p / 100.0 * len(s)))]


def pctl(v, p):
    if not v:
        return float("nan")
    s = sorted(v)
    return s[min(len(s) - 1, int(p / 100.0 * len(s)))]


# --------------------------------------------------------------------------- simulator
class Sim:
    def __init__(self, cfg: Config, svc: ServiceModel | None = None, sampler=None):
        self.cfg = cfg
        self.sampler = sampler or (default_sampler(cfg, svc) if svc else None)
        self.ev = []
        self.seq = 0
        self.now = 0.0
        self.stopping = False
        # admission
        self.held = 0
        self.queue = deque()
        self.qlen = 0
        self.abandoned = 0
        th = cfg.threshold
        n = th[2] if th[0] == "pct" else 512
        a = th[1] if th[0] == "ewma" else 0.2
        self.est = {k: Estimator(n, a) for k in KINDS}  # threshold estimator
        self.rest = {k: Estimator(512, 0.2) for k in KINDS}  # retry_after ewma + derived-D p99
        # embed
        self.e_busy = 0
        self.e_q = deque()
        self.gpu_busy = False
        self.gpu_q = deque()
        # mutex
        self.m_busy = False
        self.m_q = deque()
        # readers
        R = cfg.n_readers
        self.running = [None] * R
        self.gen = [0] * R
        self.proj_saturated = 0
        self.wq = [deque() for _ in range(R)]
        self.sq = deque()
        self.rr = 0
        self.n_idle = R
        self.hol_t = 0.0
        # accounting
        self.calls: list[Call] = []
        self.n_attempts = 0
        self.busy_integral = 0.0
        self.ab_integral = 0.0
        self.held_integral = 0.0
        self.acc_t = 0.0
        self.need_trace = []
        self.retry_obs = []  # (advised, observed or None if refused again)
        self._retry_watch = {}
        self.release_times = []
        self._rk_t, self._rk_n = None, 0
        self.max_q = 0
        pr = cfg.pred
        self.pest = {k: Estimator(pr[2] if pr[0] == "pct" else 512, pr[1] if pr[0] == "ewma" else 0.2) for k in KINDS}
        self.outstanding = {}
        self.turn_adv = {}
        self.turn_i = {}
        self.soft_fallbacks = 0

    # ---- event plumbing
    def at(self, t, fn, *a):
        heapq.heappush(self.ev, (t, self.seq, fn, a))
        self.seq += 1

    def _acc(self):
        dt = self.now - self.acc_t
        if dt > 0:
            self.busy_integral += dt * (self.cfg.n_readers - self.n_idle)
            self.ab_integral += dt * self.abandoned
            self.held_integral += dt * self.held
            self.acc_t = self.now

    def run(self):
        c = self.cfg
        if c.arrivals == "closed":
            self.r_think = {i: random.Random(c.seed * 1000003 + 7919 * i + 1) for i in range(c.K)}
            self.r_attr = {i: random.Random(c.seed * 1000003 + 7919 * i + 2) for i in range(c.K)}
            for i in range(c.K):
                t0 = self.r_think[i].uniform(0, c.start_jitter) if c.sync_start else self.r_think[i].uniform(0, 2 * c.think_median)
                self.at(t0, self.client_turn, i)
        elif c.arrivals == "open":
            self.r_arr = random.Random(c.seed * 1000003 + 3)
            self.r_attr = {0: random.Random(c.seed * 1000003 + 4)}
            self.at(self.r_arr.expovariate(c.open_rate), self.open_arrival)
        else:
            for j, (t, kind, emb, rd) in enumerate(c.script):
                self.at(t, self.script_arrival, kind, emb, rd)
        if c.writer_period:
            self.r_w = random.Random(c.seed * 1000003 + 5)
            self.at(self.r_w.expovariate(1 / c.writer_period), self.writer_arrive)
        if c.slow_injector:
            self.at(c.slow_injector[0], self.inject_slow)
        for j in range(c.projection_workers):
            self.at(0.0, self.proj_submit)
        for t, hold in c.writer_script:
            self.at(t, self.mutex_acquire, None, hold)
        if c.trace_need:
            self.at(0.0, self.trace)
        while self.ev:
            t, _, fn, a = heapq.heappop(self.ev)
            self.now = t
            fn(*a)
        self.T = self.now
        self._acc()
        return self

    def trace(self):
        if self.past_horizon():
            return
        self.need_trace.append((self.now, {k: self.need(k) for k in KINDS}, self.D_for("search_text")))
        self.at(self.now + 1.0, self.trace)

    # ---- clients
    def new_call(self, cid, rng):
        kind, emb, rd, slow = self.sampler(rng)
        st = self.cfg.step
        if st and self.now >= st[0]:
            emb, rd = emb * st[1], rd * st[1]
        c = Call(len(self.calls), cid, kind, emb, rd, slow)
        self.calls.append(c)
        return c

    def past_horizon(self):
        if self.cfg.t_stop is not None and self.now > self.cfg.t_stop:
            self.stopping = True
        return self.stopping

    def client_turn(self, cid):
        if self.past_horizon():
            return
        c = self.cfg
        r = self.r_think[cid]
        ti = self.turn_i.get(cid, 0)
        self.turn_i[cid] = ti + 1
        if c.session_turns:
            ti %= c.session_turns
        if ti < len(c.session_prefix):
            b = c.session_prefix[ti]
            r.random()  # keep the stream aligned across policies
        elif c.batch_sizes:
            b = r.choices(c.batch_sizes, c.batch_weights)[0]
        else:
            b = 1 if c.batch_max <= 1 else r.randint(1, c.batch_max)
        self.outstanding[cid] = b
        self.turn_adv[cid] = None
        for _ in range(b):
            self.submit(self.new_call(cid, self.r_attr[cid]))

    def open_arrival(self):
        if self.past_horizon():
            return
        self.submit(self.new_call(-1, self.r_attr[0]))
        self.at(self.now + self.r_arr.expovariate(self.cfg.open_rate), self.open_arrival)

    def script_arrival(self, kind, emb, rd):
        c = Call(len(self.calls), -1, kind, emb, rd, False)
        self.calls.append(c)
        self.submit(c)

    def resolve(self, c: Call, outcome, reason=None):
        c.outcome, c.reason, c.t_res = outcome, reason, self.now
        cid = c.cid
        if cid < 0 or self.cfg.arrivals != "closed":
            return
        if outcome == "refused" and c.advice is not None:
            self.turn_adv[cid] = max(self.turn_adv[cid] or 0, c.advice)
        self.outstanding[cid] -= 1
        if self.outstanding[cid] == 0:
            adv = self.turn_adv[cid]
            # draw the think time on every turn so policies stay paired (common random numbers)
            d = self.r_think[cid].lognormvariate(math.log(self.cfg.think_median), self.cfg.think_sigma)
            if adv is not None and self.cfg.honor_retry_after:
                self.at(self.now + adv, self.client_retry_turn, cid, self.now, adv)
            else:
                self.at(self.now + d, self.client_turn, cid)

    def client_retry_turn(self, cid, t_ref, adv):
        if self.past_horizon():
            return
        n0 = len(self.calls)
        self.client_turn(cid)
        if len(self.calls) > n0:
            self._retry_watch[self.calls[n0].id] = (t_ref, adv)

    # ---- deadline / threshold
    def D_for(self, kind):
        d = self.cfg.deadline
        if d[0] == "fixed":
            return d[1]
        e = self.rest[kind]
        if e.count < 8:
            return 10.0
        return max(d[1], d[2] * e.pct(99))

    def need(self, kind):
        th = self.cfg.threshold
        if th[0] == "fixed":
            return th[1]
        e = self.est[kind]
        floor = th[3]
        if e.count < 8:
            return max(floor, 0.05)  # cold start: too few samples to estimate
        if th[0] == "pct":
            return max(floor, e.pct(th[1]))
        return max(floor, th[2] * e.ewma)

    def shed_on(self):
        return self.cfg.admission in ("fifo_shed", "fifo_predict")

    def pred_s(self, kind):
        e = self.pest[kind]
        if e.count < 8:
            return 0.0
        return e.ewma if self.cfg.pred[0] == "ewma" else e.pct(self.cfg.pred[1])

    # ---- admission (Registry::acquire)
    def submit(self, c: Call):
        self.n_attempts += 1
        if self.n_attempts >= self.cfg.n_calls:
            self.stopping = True
        c.t_arr = self.now
        c.deadline = self.now + self.D_for(c.kind)
        cfg = self.cfg
        if cfg.admission == "refuse":
            if self.held < cfg.cap:
                self.admit(c)
            else:
                c.q_ahead = 0
                self.refuse(c, "busy_cap")
            return
        if self.held < cfg.cap and self.qlen == 0:
            if self.shed_on() and c.deadline - self.now < self.need(c.kind):
                self.refuse(c, "deadline")
            else:
                self.admit(c)
            return
        if cfg.queue_limit is not None and self.qlen >= cfg.queue_limit:
            c.q_ahead = self.qlen
            self.refuse(c, "queue_full")
            return
        if cfg.admission == "fifo_predict":
            ceff = cfg.cap - self.abandoned
            w = math.ceil((self.qlen + 1) / ceff) * self.pred_s(c.kind) if ceff > 0 else float("inf")
            if w > c.deadline - self.now - self.need(c.kind):
                c.q_ahead = self.qlen
                self.refuse(c, "predicted")
                return
        c.state = "queued"
        self.queue.append(c)
        self.qlen += 1
        self.max_q = max(self.max_q, self.qlen)
        exp = c.deadline - (self.need(c.kind) if self.shed_on() else 0.0)
        self.at(max(self.now, exp), self.expire, c)

    def expire(self, c):
        if c.state == "queued":
            ahead = 0
            for x in self.queue:
                if x is c:
                    break
                if x.state == "queued":
                    ahead += 1
            self.qlen -= 1
            c.q_ahead = ahead
            self.refuse(c, "deadline")

    def retry_after(self, c, reason):
        e = self.rest[c.kind]
        ceff = self.cfg.cap - self.abandoned
        if e.count < 8 or ceff <= 0 or self.cfg.admission == "refuse":
            return None
        if reason == "queue_full":
            v = e.ewma / ceff
        else:
            v = math.ceil((self.qlen + 1) / ceff) * e.ewma
        return min(max(v, 0.05), c.deadline - c.t_arr)

    def same_instant_rank(self):
        """Arrival refusals at one instant would have queued one behind another (FIFO positions)."""
        if self._rk_t == self.now:
            self._rk_n += 1
        else:
            self._rk_t, self._rk_n = self.now, 0
        return self._rk_n

    def refuse(self, c, reason):
        if reason in ("busy_cap", "queue_full", "predicted") and c.q_ahead is not None:
            c.q_ahead += self.same_instant_rank()
        c.state = "refused"
        c.remaining_at_refuse = c.deadline - self.now
        c.advice = self.retry_after(c, reason)
        w = self._retry_watch.pop(c.id, None)
        if w:
            self.retry_obs.append((w[1], None))
        self.resolve(c, "refused", reason)

    def admit(self, c):
        self._acc()
        self.held += 1
        c.state = "running"
        c.t_adm = self.now
        w = self._retry_watch.pop(c.id, None)
        if w:
            self.retry_obs.append((w[1], self.now - w[0]))
        rl = self.cfg.run_limit
        self.at(c.deadline if rl is None else self.now + rl, self.deadline_check, c)
        if c.hybrid:
            self.embed_submit(c)
        else:
            self.dispatch(c)

    def grant(self):
        cfg = self.cfg
        while self.held < cfg.cap and self.queue:
            c = self.queue.popleft()
            if c.state != "queued":
                continue
            self.qlen -= 1
            if self.shed_on() and c.deadline - self.now < self.need(c.kind):
                self.refuse(c, "deadline")
                continue
            if c.deadline <= self.now:
                self.refuse(c, "deadline")
                continue
            self.admit(c)

    def deadline_check(self, c):
        if c.t_done is None:
            self._acc()
            c.timed_out = True
            if self.cfg.interrupt:
                self.cancel(c)
                self.resolve(c, "timeout")
                return
            self.abandoned += 1
            self.resolve(c, "timeout")

    def cancel(self, c):
        c.cancelled = True
        c.t_done = self.now
        for i, (x, t0) in enumerate(self.e_q):
            if x is c:
                del self.e_q[i]
                break
        for i, (x, t0) in enumerate(self.gpu_q):
            if x is c:  # holds an embed worker slot but has not started on the GPU: drop the forward
                del self.gpu_q[i]
                self.e_busy -= 1
                self.embed_pull()
                break
        for w, q in enumerate(self.wq):
            if c in q:
                self.hol_update()
                q.remove(c)
        if c in self.sq:
            self.sq.remove(c)
        for w, r in enumerate(self.running):
            if r is c:
                self.hol_update()
                self._acc()
                self.gen[w] += 1
                self.running[w] = None
                self.n_idle += 1
                self.pull_next(w)
        self.held -= 1
        self.release_times.append(self.now)
        if self.cfg.admission != "refuse":
            self.grant()

    def finish(self, c):
        if c.cancelled:
            return
        c.t_done = self.now
        self._acc()
        if c.timed_out:
            self.abandoned -= 1
        else:
            hold = self.now - c.t_adm
            self.est[c.kind].add(hold)
            self.rest[c.kind].add(hold)
            self.pest[c.kind].add(hold)
            self.resolve(c, "ok")
        self.held -= 1
        self.release_times.append(self.now)
        if self.cfg.admission != "refuse":
            self.grant()

    # ---- embed stage (H7)
    def embed_submit(self, c):
        if self.e_busy < self.cfg.embed_workers:
            self.e_busy += 1
            self.embed_run(c)
        elif len(self.e_q) < self.cfg.embed_capacity:
            self.e_q.append((c, self.now))
            self.at(self.now + self.cfg.embed_timeout, self.embed_expire, c)
        else:
            c.soft = True
            self.soft_fallbacks += 1
            self.dispatch(c)

    def embed_run(self, c):
        if not self.cfg.gpu_serial:
            self.at(self.now + c.embed, self.embed_done, c)
        elif not self.gpu_busy:
            self.gpu_busy = True
            f = self.cfg.gpu_contention if self.e_busy > 1 else 1.0
            self.at(self.now + c.embed * f, self.gpu_done, c)
        else:
            self.gpu_q.append((c, self.now))

    def gpu_done(self, c):
        if self.gpu_q:
            n, t0 = self.gpu_q.popleft()
            n.embed_wait += self.now - t0
            self.at(self.now + n.embed * self.cfg.gpu_contention, self.gpu_done, n)
        else:
            self.gpu_busy = False
        self.embed_done(c)

    def embed_pull(self):
        while self.e_q and self.e_busy < self.cfg.embed_workers:
            n, t0 = self.e_q.popleft()
            if n.cancelled:
                continue
            n.embed_wait = self.now - t0
            self.e_busy += 1
            self.embed_run(n)

    def embed_expire(self, c):
        """A queued embed request past embedder_call_timeout: the query falls back to text-only."""
        for i, (x, t0) in enumerate(self.e_q):
            if x is c:
                del self.e_q[i]
                if c.proj:
                    self.at(self.now, self.proj_submit)
                elif not c.cancelled:
                    c.soft = True
                    self.soft_fallbacks += 1
                    c.embed_wait = self.now - t0
                    self.dispatch(c)
                return

    def embed_done(self, c):
        self.e_busy -= 1
        self.embed_pull()
        if c.proj:
            self.at(self.now, self.proj_submit)
            return
        if c.cancelled:
            return
        self.mutex_acquire(c, self.cfg.mutex_hold)

    # ---- writer connection mutex (H8)
    def mutex_acquire(self, who, hold):
        if not self.m_busy:
            self.m_busy = True
            self.at(self.now + hold, self.mutex_release, who)
        else:
            self.m_q.append((who, hold, self.now))

    def mutex_release(self, who):
        if who is not None:
            self.dispatch(who)
        if self.m_q:
            n, hold, t0 = self.m_q.popleft()
            if n is not None:
                n.mutex_wait = self.now - t0
            self.at(self.now + hold, self.mutex_release, n)
        else:
            self.m_busy = False

    def writer_arrive(self):
        if self.past_horizon():
            return
        hold = self.r_w.lognormvariate(math.log(self.cfg.writer_hold), 1.0)
        self.mutex_acquire(None, hold)
        self.at(self.now + self.r_w.expovariate(1 / self.cfg.writer_period), self.writer_arrive)

    # ---- reader pool (H10/H11)
    def hol_update(self):
        dt = self.now - self.hol_t
        if dt > 0 and self.n_idle > 0:
            for q in self.wq:
                for c in q:
                    hi = min(self.now, c.deadline)
                    if hi > self.hol_t:
                        c.hol += hi - self.hol_t
        self.hol_t = self.now

    def dispatch(self, c):
        if c.cancelled:
            return
        c.t_rq = self.now
        self.hol_update()
        R = self.cfg.n_readers
        p = self.cfg.dispatch
        if p == "P1":
            if self.n_idle > 0:
                w = self.running.index(None)
                self.start_worker(w, c)
            else:
                self.sq.append(c)
            return
        if p == "P2":
            for i in range(R):
                w = (self.rr + i) % R
                if self.running[w] is None and not self.wq[w]:
                    self.rr = w + 1
                    self.start_worker(w, c)
                    return
        w = self.rr % R
        self.rr += 1
        if self.running[w] is None and not self.wq[w]:
            self.start_worker(w, c)
        else:
            self.wq[w].append(c)

    def start_worker(self, w, c):
        self._acc()
        if self.running[w] is None:
            self.n_idle -= 1
        self.running[w] = c
        c.t_rs = self.now
        self.gen[w] += 1
        self.at(self.now + c.reader, self.worker_done, w, self.gen[w])

    def pull_next(self, w):
        if self.cfg.dispatch == "P1":
            if self.sq:
                self.start_worker(w, self.sq.popleft())
        elif self.wq[w]:
            self.start_worker(w, self.wq[w].popleft())

    def worker_done(self, w, gen):
        if gen != self.gen[w]:
            return  # interrupted
        self.hol_update()
        self._acc()
        c = self.running[w]
        self.running[w] = None
        self.n_idle += 1
        self.pull_next(w)
        self.finish(c)

    def inject_slow(self):
        if self.past_horizon():
            return
        c = Call(len(self.calls), -3, "expand", 0.0, self.cfg.slow_injector[1], True)
        self.calls.append(c)
        self.submit(c)
        self.at(self.now + self.cfg.slow_injector[0], self.inject_slow)

    # ---- projection embedding backlog (hazard C): resubmit at once; on Saturated poll every 25 ms
    def proj_submit(self):
        if self.past_horizon() or (self.cfg.horizon_stop and self.calls and all(c.outcome for c in self.calls)
                             and len(self.calls) == len(self.cfg.script)):
            return
        p = Call(-1, -2, "projection", self.cfg.projection_batch_s * self.cfg.speed_mult, 0.0, False)
        p.proj = True
        if self.e_busy < self.cfg.embed_workers:
            self.e_busy += 1
            self.embed_run(p)
        elif len(self.e_q) < self.cfg.embed_capacity:
            self.e_q.append((p, self.now))
            self.at(self.now + self.cfg.embed_timeout, self.embed_expire, p)
        else:
            self.proj_saturated += 1
            self.at(self.now + 0.025, self.proj_submit)

    # ---- metrics
    def metrics(self):
        cfg = self.cfg
        w0 = int(len(self.calls) * cfg.warmup_frac)
        cs = [c for c in self.calls[w0:] if c.outcome is not None and c.cid != -3]
        n = len(cs)
        out = {"n": n}
        ok = [c for c in cs if c.outcome == "ok"]
        to = [c for c in cs if c.outcome == "timeout"]
        rf = [c for c in cs if c.outcome == "refused"]
        out["ok_rate"] = len(ok) / n
        out["timeout_rate"] = len(to) / n
        out["refuse_rate"] = len(rf) / n
        cause = {"admission": 0, "hol": 0, "run": 0, "other": 0}
        for c in to:
            # every component is measured inside the call's own window [arrival, deadline]
            W = c.deadline
            a = c.t_adm - c.t_arr
            if c.t_rq is None:
                rq = 0.0  # interrupted (EP-09) before reaching the reader pool
            else:
                rq = max(0.0, min(c.t_rs if c.t_rs is not None else W, W) - c.t_rq)
            run = max(0.0, min(c.t_rs + c.reader, W) - c.t_rs) if c.t_rs is not None and c.t_rs < W else 0.0
            if not c.soft:
                run += min(c.embed, max(0.0, W - c.t_adm))
            other = max(0.0, rq - c.hol) + min(c.embed_wait, W - c.t_adm) + c.mutex_wait
            parts = {"admission": a, "hol": c.hol, "run": run, "other": other}
            cause[max(parts, key=parts.get)] += 1
        for k, v in cause.items():
            out[f"to_{k}"] = v / n
        for r in ("busy_cap", "queue_full", "deadline"):
            out[f"rf_{r}"] = sum(1 for c in rf if c.reason == r) / n
        # false shed: refused for time but own service would have fit the remaining deadline
        fs = [c for c in rf if c.reason == "deadline" and c.remaining_at_refuse > 0
              and (c.reader + c.embed) <= c.remaining_at_refuse]
        out["false_shed_rate"] = len(fs) / n
        # exact FIFO counterfactual: a refused call queued with q_ahead waiters in front would get the
        # (q_ahead+1)-th permit release after its refusal (None = a permit was free at that moment)
        import bisect
        rt = self.release_times

        def would_finish(c):
            own = c.reader + c.embed
            if c.q_ahead is None:
                start = c.t_res
            else:
                i = bisect.bisect_right(rt, c.t_res) + c.q_ahead
                if i >= len(rt):
                    return False
                start = rt[i]
            if self.cfg.run_limit is not None:
                return start < c.deadline and own <= self.cfg.run_limit
            return start + own <= c.deadline
        fr = [c for c in rf if would_finish(c)]
        out["false_refusal_rate"] = len(fr) / n
        out["false_refusal_share"] = len(fr) / len(rf) if rf else float("nan")
        out["rf_predicted"] = sum(1 for c in rf if c.reason == "predicted") / n
        out["max_queue"] = self.max_q
        bursts = {}
        for c in cs:
            if c.cid >= 0:
                bursts.setdefault((c.cid, c.t_arr), []).append(c)
        bt = [max(x.t_res for x in v) - v[0].t_arr for v in bursts.values() if len(v) > 1 and all(x.outcome == "ok" for x in v)]
        out["burst_n"] = sum(1 for v in bursts.values() if len(v) > 1)
        out["burst_any_refused"] = (sum(1 for v in bursts.values() if len(v) > 1 and any(x.outcome == "refused" for x in v))
                                    / max(1, out["burst_n"]))
        out["burst_done_p50"] = pctl(bt, 50)
        out["burst_done_p99"] = pctl(bt, 99)
        lone = [v[0].t_adm - v[0].t_arr for v in bursts.values() if len(v) == 1 and v[0].t_adm is not None]
        out["lone_wait_p99"] = pctl(lone, 99)
        out["lone_wait_max"] = max(lone) if lone else float("nan")
        out["turn_refused"] = (sum(1 for v in bursts.values() if any(x.outcome == "refused" for x in v)) / max(1, len(bursts)))
        # an agent re-issues a failed call on its next LLM turn: one retry turn per turn with any failure
        out["turn_failed"] = (sum(1 for v in bursts.values() if any(x.outcome != "ok" for x in v)) / max(1, len(bursts)))
        out["late_admit_timeout_rate"] = sum(1 for c in to if c.t_adm - c.t_arr > 0) / n
        lat = [c.t_res - c.t_arr for c in ok]
        for p in (50, 95, 99):
            out[f"ok_p{p}"] = pctl(lat, p)
        out["ok_p999"] = pctl(lat, 99.9)
        out["fail_rate"] = 1.0 - out["ok_rate"]
        late = [c for c in ok if c.t_res > c.deadline]
        out["ok_late_rate"] = len(late) / n
        out["ok_late_share"] = len(late) / len(ok) if ok else float("nan")
        out["ok_late30_rate"] = sum(1 for c in ok if c.t_res - c.t_arr > 30.0) / n
        out["late_start_ok_rate"] = sum(1 for c in late if c.t_adm > c.t_arr) / n
        for lo, hi in LATE_BINS:
            out[f"late_h_{lo}_{hi}"] = sum(1 for c in late if lo < c.t_res - c.t_arr <= (hi if hi != "inf" else math.inf))
        # time to data: a failed call costs its own wait, one think time and one retry at the mean ok latency
        retry = sum(lat) / len(lat) if lat else 0.0
        ttd = lat + [c.t_res - c.t_arr + cfg.think_median + retry for c in cs if c.outcome != "ok"]
        out["ttd_mean"] = sum(ttd) / len(ttd)
        out["ttd_p99"] = pctl(ttd, 99)
        out["ttd_p999"] = pctl(ttd, 99.9)
        allr = [c.t_res - c.t_arr for c in cs if c.outcome != "refused"]
        out["resp_p95"] = pctl(allr, 95)
        T = max(self.T, 1e-9)
        out["util"] = self.busy_integral / (T * cfg.n_readers)
        out["abandoned_avg"] = self.ab_integral / T
        out["abandoned_share"] = self.ab_integral / self.held_integral if self.held_integral else 0.0
        out["ab_permit_s_per_kcall"] = self.ab_integral / n * 1000
        out["wasted_permit_s_per_kcall"] = sum((c.t_done if c.t_done is not None else self.T) - c.t_adm for c in to) / n * 1000
        # over the span to the last resolution: T also runs on through trailing no-op deadline checks
        out["goodput_per_s"] = len(ok) / max(1e-9, max(c.t_res for c in cs))
        out["soft_fallback_rate"] = self.soft_fallbacks / max(1, len(self.calls))
        ro = self.retry_obs
        if ro:
            errs = [abs(o - a) / a for a, o in ro if o is not None]
            out["retry_n"] = len(ro)
            out["retry_rerefused"] = sum(1 for _, o in ro if o is None) / len(ro)
            out["retry_relerr_med"] = pctl(errs, 50) if errs else float("nan")
        else:
            out["retry_n"] = 0
            out["retry_rerefused"] = float("nan")
            out["retry_relerr_med"] = float("nan")
        return out


def simulate(cfg, svc=None, sampler=None):
    return Sim(cfg, svc, sampler).run()
