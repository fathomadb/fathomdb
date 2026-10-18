"""Deterministic mechanics tests + queueing-theory sanity for sim.py."""
import math
import random

import pytest

from sim import Config, simulate


def script_run(script, **kw):
    cfg = Config(arrivals="script", script=script, n_calls=10**9, warmup_frac=0.0, **kw)
    return simulate(cfg)


def T(kind="search_text", reader=0.01, embed=0.0):
    return (kind, embed, reader)


# ---------------------------------------------------------------- admission
def test_refuse_policy_refuses_immediately_at_cap():
    s = script_run([(0.0, *T(reader=1.0)), (0.1, *T(reader=1.0))], cap=1, admission="refuse")
    a, b = s.calls
    assert a.outcome == "ok" and b.outcome == "refused" and b.reason == "busy_cap"
    assert b.t_res == pytest.approx(0.1)


def test_fifo_waiter_runs_after_permit_frees():
    s = script_run([(0.0, *T(reader=1.0)), (0.1, *T(reader=0.5))], cap=1, admission="fifo")
    a, b = s.calls
    assert b.outcome == "ok" and b.t_adm == pytest.approx(1.0) and b.t_res == pytest.approx(1.5)


def test_fifo_admits_in_arrival_order():
    sc = [(0.0, *T(reader=1.0))] + [(0.1 * i, *T(reader=0.2)) for i in (1, 2, 3)]
    s = script_run(sc, cap=1, admission="fifo")
    adm = [c.t_adm for c in s.calls]
    assert adm == sorted(adm) and adm[1:] == pytest.approx([1.0, 1.2, 1.4])


def test_queue_full_refuses_at_once():
    sc = [(0.0, *T(reader=5.0)), (0.1, *T()), (0.2, *T())]
    s = script_run(sc, cap=1, admission="fifo", queue_limit=1)
    assert s.calls[2].reason == "queue_full" and s.calls[2].t_res == pytest.approx(0.2)


def test_early_shed_fires_at_deadline_minus_need():
    sc = [(0.0, *T(reader=20.0)), (1.0, *T())]
    s = script_run(sc, cap=1, admission="fifo_shed", threshold=("fixed", 0.25))
    b = s.calls[1]
    assert b.reason == "deadline" and b.t_res == pytest.approx(1.0 + 10.0 - 0.25)


def test_t0_waits_full_deadline_then_refuses():
    sc = [(0.0, *T(reader=20.0)), (1.0, *T())]
    s = script_run(sc, cap=1, admission="fifo")
    assert s.calls[1].reason == "deadline" and s.calls[1].t_res == pytest.approx(11.0)


def test_late_permit_is_refused_and_passed_on():
    # permit frees at 10.9; waiter b has 0.1 s left < need 0.25 -> refused; c (later deadline) admitted
    sc = [(0.0, *T(reader=10.9)), (1.0, *T()), (2.0, *T())]
    s = script_run(sc, cap=1, admission="fifo_shed", threshold=("fixed", 0.25))
    a, b, c = s.calls
    # b expired at 10.75 already (deadline - need), before the release
    assert b.reason == "deadline" and b.t_res == pytest.approx(10.75)
    assert c.outcome == "ok" and c.t_adm == pytest.approx(10.9)


def test_timed_out_call_keeps_permit_until_engine_finishes():
    sc = [(0.0, *T(reader=12.0)), (10.5, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo")
    a, b = s.calls
    assert a.outcome == "timeout" and a.t_res == pytest.approx(10.0)
    assert b.t_adm == pytest.approx(12.0)  # waited for the abandoned work
    assert s.ab_integral == pytest.approx(2.0)  # abandoned 10.0 -> 12.0


def test_retry_after_uses_effective_capacity():
    # cap 2: train ewma with 8 calls of 0.1 s, then hold one abandoned + one running, queue 2
    sc = [(i * 0.2, *T(reader=0.1)) for i in range(8)]
    t = 2.0
    sc += [(t, *T(reader=30.0)), (t, *T(reader=30.0)), (t + 0.01, *T()), (t + 0.02, *T()), (t + 0.03, *T())]
    s = script_run(sc, cap=2, admission="fifo_shed", queue_limit=2, threshold=("fixed", 0.05))
    full = s.calls[12]
    assert full.reason == "queue_full"
    assert full.advice == pytest.approx(0.1 / 2)  # ewma / c_eff, no abandonment yet


# ---------------------------------------------------------------- reader dispatch / HOL
def hol_script():
    # slow read on worker 0, then 8 quick reads 0.1 s apart: read #8 (9th call) lands on worker 0 under P0
    return [(0.0, *T(reader=5.0))] + [(0.1 * i, *T(reader=0.01)) for i in range(1, 9)]


def test_p0_hand_traced_head_of_line():
    s = script_run(hol_script(), cap=7 + 100, admission="refuse", dispatch="P0")  # cap out of the way
    last = s.calls[8]
    assert last.t_rs == pytest.approx(5.0)
    assert last.hol == pytest.approx(5.0 - 0.8)  # other workers idle the whole wait
    assert all(c.t_rs == pytest.approx(c.t_arr) for c in s.calls[1:8])


@pytest.mark.parametrize("pol", ["P1", "P2"])
def test_work_conserving_policies_avoid_hol(pol):
    s = script_run(hol_script(), cap=107, admission="refuse", dispatch=pol)
    assert all(c.t_rs == pytest.approx(c.t_arr) and c.hol == 0 for c in s.calls)


def test_p2_falls_back_to_round_robin_when_all_busy():
    sc = [(0.0, *T(reader=1.0)) for _ in range(8)] + [(0.1, *T(reader=0.1))]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P2")
    assert s.calls[8].t_rs == pytest.approx(1.0)


def test_hol_cause_attribution():
    sc = [(0.0, *T(reader=30.0))] + [(0.1 * i, *T(reader=0.01)) for i in range(1, 9)]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P0")
    m = s.metrics()
    assert s.calls[8].outcome == "timeout"
    assert m["to_hol"] == pytest.approx(1 / 9) and m["to_run"] == pytest.approx(1 / 9)


# ---------------------------------------------------------------- embed + mutex
def test_embed_pool_saturation_falls_back_to_text():
    sc = [(0.0, "search_hybrid", 1.0, 0.01) for _ in range(5 + 20 + 1)]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P1", gpu_serial=False)
    assert sum(c.soft for c in s.calls) == 1 and s.soft_fallbacks == 1


def test_serialized_gpu_expires_queued_embeds_into_text_fallback():
    # 25 one-second embeds on one GPU at 1.8x contention: the queue tail passes the 30 s embed timeout
    sc = [(0.0, "search_hybrid", 1.0, 0.01) for _ in range(25)]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P1", deadline=("fixed", 1e9))
    assert s.soft_fallbacks > 0 and all(c.outcome == "ok" for c in s.calls)


def test_writer_mutex_delays_hybrid():
    sc = [(0.5, "search_hybrid", 0.01, 0.01), (0.5, "search_text", 0.0, 0.01)]
    s = script_run(sc, cap=4, writer_script=[(0.0, 0.6)])
    h, x = s.calls
    assert h.mutex_wait == pytest.approx(0.6 - 0.51) and h.t_rs == pytest.approx(0.6 + 0.0001)
    assert x.t_rs == pytest.approx(0.5)  # text path does not take the writer mutex


# ---------------------------------------------------------------- queueing theory: Erlang C for P1
def erlang_c_wq(lam, mu, c):
    a = lam / mu
    rho = a / c
    s = sum(a**k / math.factorial(k) for k in range(c))
    pc = (a**c / math.factorial(c)) / (1 - rho)
    pw = pc / (s + pc)
    return pw / (c * mu - lam), pw


@pytest.mark.parametrize("c,rho", [(2, 0.7), (8, 0.8)])
def test_mmc_matches_erlang_c(c, rho):
    mu = 1.0
    lam = rho * c * mu
    rng_seed = 7

    def sampler(r):
        return "search_text", 0.0, r.expovariate(mu), False

    cfg = Config(arrivals="open", open_rate=lam, n_calls=200000, cap=10**6, admission="refuse",
                 dispatch="P1", n_readers=c, deadline=("fixed", 1e9), seed=rng_seed, warmup_frac=0.05)
    s = simulate(cfg, sampler=sampler)
    cs = s.calls[10000:]
    wq = sum(x.t_rs - x.t_rq for x in cs) / len(cs)
    pw = sum(1 for x in cs if x.t_rs - x.t_rq > 1e-12) / len(cs)
    ewq, epw = erlang_c_wq(lam, mu, c)
    assert wq == pytest.approx(ewq, rel=0.06)
    assert pw == pytest.approx(epw, rel=0.04)


def test_p0_round_robin_is_worse_than_p1_under_load():
    # sanity: same arrivals, P0 (8 x D/M/1-like) waits more than P1 (M/M/8)
    def sampler(r):
        return "search_text", 0.0, r.expovariate(1.0), False

    base = dict(arrivals="open", open_rate=6.0, n_calls=60000, cap=10**6, admission="refuse",
                deadline=("fixed", 1e9), seed=3)
    w = {}
    for p in ("P0", "P1"):
        s = simulate(Config(dispatch=p, **base), sampler=sampler)
        w[p] = sum(x.t_rs - x.t_rq for x in s.calls) / len(s.calls)
    assert w["P0"] > 2 * w["P1"]


# ---------------------------------------------------------------- common random numbers
def test_crn_same_call_sequence_across_policies(tmp_path):
    from sim import ServiceModel
    svc = ServiceModel({k: [0.001, 0.002, 0.003] for k in
                        ("search_text_10", "search_text_100", "search_hybrid_10", "search_hybrid_100",
                         "fetch_10", "expand_d3")})
    seqs = []
    for adm, disp in [("refuse", "P0"), ("fifo_shed", "P1")]:
        s = simulate(Config(K=4, n_calls=400, admission=adm, dispatch=disp, slow_p=0.05, seed=11), svc)
        seqs.append([(c.cid, c.kind, c.reader) for c in s.calls])
    by_client = [{cid: [x[1:] for x in seq if x[0] == cid][:50] for cid in range(4)} for seq in seqs]
    assert by_client[0] == by_client[1]


def test_retry_after_deadline_reason_uses_effective_capacity_and_queue():
    sc = [(i * 0.2, *T(reader=0.1)) for i in range(8)]
    sc += [(2.0, *T(reader=30.0)), (12.1, *T(reader=20.0)), (12.2, *T()), (12.21, *T())]
    s = script_run(sc, cap=2, admission="fifo_shed", threshold=("fixed", 9.95))
    c = s.calls[10]
    assert s.calls[8].outcome == "timeout"
    assert c.reason == "deadline" and c.t_res == pytest.approx(12.25)
    # c_eff = 2 - 1 abandoned; one still queued behind it -> ceil((1+1)/1) * 0.1
    assert c.advice == pytest.approx(0.2)


def test_retry_after_omitted_when_stuck():
    sc = [(i * 0.2, *T(reader=0.1)) for i in range(8)]
    sc += [(2.0, *T(reader=30.0)), (12.5, *T())]
    s = script_run(sc, cap=1, admission="fifo_shed", threshold=("fixed", 0.05))
    assert s.calls[9].reason == "deadline" and s.calls[9].advice is None


def test_session_prefix_burst_refuses_beyond_cap_at_once():
    from sim import ServiceModel
    svc = ServiceModel({k: [0.005] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                              "search_hybrid_100", "fetch_10", "expand_d3")})
    cfg = Config(K=1, cap=4, admission="refuse", session_prefix=(1, 10), session_turns=2, n_calls=11,
                 warmup_frac=0.0, hybrid_frac=0.0)
    s = simulate(cfg, svc)
    burst = s.calls[1:11]
    assert len({c.t_arr for c in burst}) == 1
    assert sum(c.outcome == "refused" for c in burst) == 6


def test_fifo_absorbs_burst_within_queue_limit():
    from sim import ServiceModel
    svc = ServiceModel({k: [0.005] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                              "search_hybrid_100", "fetch_10", "expand_d3")})
    for ql, refused in ((8, 0), (4, 2)):
        cfg = Config(K=1, cap=4, admission="fifo_shed", queue_limit=ql, session_prefix=(1, 10),
                     session_turns=2, n_calls=11, warmup_frac=0.0, hybrid_frac=0.0)
        s = simulate(cfg, svc)
        assert sum(c.outcome == "refused" for c in s.calls[1:11]) == refused


# ---------------------------------------------------------------- EP-09 interrupt, projection contention
def test_interrupt_frees_permit_and_worker_at_deadline():
    sc = [(0.0, *T(reader=12.0)), (10.5, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo", interrupt=True)
    a, b = s.calls
    assert a.outcome == "timeout" and b.t_adm == pytest.approx(10.5) and b.t_rs == pytest.approx(10.5)
    assert s.ab_integral == 0.0


def test_interrupt_unblocks_reads_queued_behind_slow_read():
    # worker 0: slow 30 s read, then read #8 (5 s) and read #16 queued behind it under P0
    sc = [(0.0, *T(reader=30.0))] + [(0.1 * i, *T(reader=5.0 if i == 8 else 0.01)) for i in range(1, 17)]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P0", interrupt=True)
    r8, r16 = s.calls[8], s.calls[16]
    assert r8.t_rs == pytest.approx(10.0) and r8.outcome == "timeout"  # its own deadline 10.8 cuts it
    assert r16.t_rs == pytest.approx(10.8) and r16.outcome == "ok"
    assert s.calls[0].outcome == "timeout"


@pytest.mark.parametrize("gpu_serial,wait", [(False, 0.5), (True, 4.5)])
def test_projection_backlog_delays_query_embedding(gpu_serial, wait):
    # five 1 s projection batches hold all five embed workers from t=0; on one GPU they also run one
    # after another, so the query gets a worker at 1.0 s and then waits for batches 2-5 on the GPU
    sc = [(0.5, "search_hybrid", 0.01, 0.01)]
    s = script_run(sc, cap=4, projection_workers=5, projection_batch_s=1.0, horizon_stop=True, gpu_serial=gpu_serial,
                    gpu_contention=1.0)
    assert s.calls[0].embed_wait == pytest.approx(wait)


def test_gpu_serializes_concurrent_query_embeddings():
    sc = [(0.0, "search_hybrid", 0.009, 0.001) for _ in range(4)]
    s = script_run(sc, cap=4, gpu_serial=True, gpu_contention=1.0)
    ends = sorted(c.t_res for c in s.calls)
    assert ends == pytest.approx([0.009 * i + 0.001 + 0.0001 * j for i, j in zip((1, 2, 3, 4), (1, 1, 1, 1))], abs=2e-4)
    s2 = script_run(sc, cap=4, gpu_serial=False)
    assert max(c.t_res for c in s2.calls) < 0.012


def test_arrival_test_refuses_at_once_when_predicted_wait_exceeds_deadline():
    # ewma trained to 3 s; need = ewma = 3 s; cap 1. At 40.3 s: one running, two queued ->
    # predicted wait ceil(3/1) * 3 = 9 s > 10 - 3 -> refused at once with reason "predicted"
    sc = [(i * 4.0, *T(reader=3.0)) for i in range(8)]
    sc += [(40.0, *T(reader=3.0)), (40.1, *T(reader=3.0)), (40.2, *T(reader=3.0)), (40.3, *T(reader=3.0))]
    s = script_run(sc, cap=1, admission="fifo_predict", threshold=("ewma", 0.2, 1.0, 0.05), pred=("ewma", 0.2))
    assert [c.outcome for c in s.calls[8:11]] == ["ok", "ok", "ok"]
    last = s.calls[11]
    assert last.reason == "predicted" and last.t_res == pytest.approx(40.3)


def test_arrival_test_with_tiny_need_admits_late():
    # same load, need = 50 ms: the 4th call is queued, admitted at 49 s with 1.3 s left, and times out
    sc = [(i * 4.0, *T(reader=3.0)) for i in range(8)]
    sc += [(40.0, *T(reader=3.0)), (40.1, *T(reader=3.0)), (40.2, *T(reader=3.0)), (40.3, *T(reader=3.0))]
    s = script_run(sc, cap=1, admission="fifo_predict", threshold=("fixed", 0.05), pred=("ewma", 0.2))
    assert s.calls[11].outcome == "timeout" and s.calls[11].t_adm == pytest.approx(49.0)


def test_counterfactual_false_refusal():
    # cap 1, queue_limit 0: B refused while A runs 0.2 s; B (0.1 s) would have finished at 0.3 < deadline
    sc = [(0.0, *T(reader=0.2)), (0.05, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo", queue_limit=0)
    m = s.metrics()
    assert s.calls[1].reason == "queue_full" and m["false_refusal_rate"] == pytest.approx(0.5)


def test_calibration_against_live_burst_probe():
    """Simulated clearing time of a k-call burst at cap 7 vs the live probe (results/burst_probe.csv)."""
    import csv, os
    from collections import defaultdict
    from sim import ServiceModel, Sim
    here = os.path.dirname(os.path.abspath(__file__))
    svc = ServiceModel.from_csv(os.path.join(here, "results/service_times.csv"))
    live = defaultdict(lambda: defaultdict(float))
    for r in csv.DictReader(open(os.path.join(here, "results/burst_probe.csv"))):
        if r["phase"] == "concurrency_sweep":
            key = (r["mode"], int(r["k"]), r["rep"])
            live[key[:2]][key[2]] = max(live[key[:2]][key[2]], float(r["ms"]))
    ok = []
    for mode in ("text", "hybrid"):
        for k in (1, 2, 4, 7):
            obs = sorted(live[(mode, k)].values())[len(live[(mode, k)]) // 2]
            sims = []
            for rep in range(40):
                cfg = Config(seed=rep, K=1, cap=7, admission="refuse", dispatch="P0", hybrid_frac=1.0 if mode == "hybrid" else 0.0,
                             session_prefix=(k,), session_turns=1, n_calls=k, warmup_frac=0.0)
                from sim import default_sampler
                samp = default_sampler(cfg, svc)
                def only_search(r, samp=samp):
                    while True:
                        x = samp(r)
                        if x[0].startswith("search"):
                            return x
                s = Sim(cfg, sampler=only_search).run()
                sims.append(1000 * max(c.t_res - c.t_arr for c in s.calls))
            med = sorted(sims)[len(sims) // 2]
            print(mode, k, "live", round(obs, 2), "sim", round(med, 2))
            # the probe's numbers include stdio + JSON serialisation per response (~0.3-1 ms on text)
            if mode == "text" and k >= 5:
                # known gap: live text bursts show a knee at 5 concurrent calls (+2-4 ms) that the model
                # lacks; candidates: 5 WAL read-mark slots, global page-cache mutex, response serialisation
                ok.append(med < obs)
            else:
                ok.append(med == pytest.approx(obs, rel=0.35, abs=1.0))
    assert all(ok)


def test_metrics_with_call_interrupted_before_reaching_reader():
    # hybrid call stuck behind a 20 s embed on the GPU; EP-09 cancels it at its deadline before dispatch
    sc = [(0.0, "search_hybrid", 20.0, 0.01), (0.1, "search_hybrid", 0.01, 0.01)]
    s = script_run(sc, cap=4, interrupt=True, gpu_contention=1.0, embed_timeout=60.0)
    m = s.metrics()
    assert s.calls[1].outcome == "timeout" and s.calls[1].t_rq is None
    assert m["timeout_rate"] == pytest.approx(1.0)


def test_t_stop_ends_arrivals_at_a_time_horizon():
    from sim import ServiceModel
    svc = ServiceModel({k: [0.001] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                              "search_hybrid_100", "fetch_10", "expand_d3")})
    s = simulate(Config(K=16, n_calls=10**9, t_stop=500.0, think_median=1.0), svc)
    assert max(c.t_arr for c in s.calls) <= 500.0 + 1e-9 and max(c.t_arr for c in s.calls) > 490.0


# ---------------------------------------------------------------- review findings (2026-10-18), RED first
def test_B1_simultaneous_refusals_take_successive_fifo_positions():
    # cap 1, A runs 5 s; three 4.95 s calls refused at once at 0.1: only the first would have finished
    sc = [(0.0, *T(reader=5.0))] + [(0.1, *T(reader=4.95)) for _ in range(3)]
    s = script_run(sc, cap=1, admission="refuse")
    assert s.metrics()["false_refusal_rate"] * 4 == pytest.approx(1)


def test_B3_retry_turn_consumes_a_think_draw():
    from sim import Sim
    cfg = Config(K=1, arrivals="closed", n_calls=10**9, honor_retry_after=True)
    s = Sim(cfg)
    s.r_think = {0: __import__("random").Random(1)}
    s.outstanding = {0: 1}
    s.turn_adv = {0: 0.5}
    before = s.r_think[0].getstate()
    c = __import__("sim").Call(0, 0, "search_text", 0.0, 0.01, False)
    c.advice = 0.5
    s.resolve(c, "refused", "deadline")
    assert s.r_think[0].getstate() != before


def test_B4_cancel_drops_a_queued_gpu_forward():
    # call 0 holds the GPU 0-20 s; call 1's 5 s forward is queued and the call is cancelled at 10.1 s.
    # Without the fix the cancelled forward still runs 20-25 s and is the last event.
    sc = [(0.0, "search_hybrid", 20.0, 0.01), (0.1, "search_hybrid", 5.0, 0.01)]
    s = script_run(sc, cap=4, interrupt=True, gpu_contention=1.0, embed_timeout=60.0)
    assert s.T == pytest.approx(20.0)
    assert s.e_busy == 0 and not s.gpu_q


def test_B5_embed_queue_entry_expires_at_its_timeout():
    sc = [(0.0, "search_hybrid", 100.0, 0.01) for _ in range(5)] + [(0.1, "search_hybrid", 0.01, 0.01)]
    s = script_run(sc, cap=100, admission="refuse", gpu_serial=False, deadline=("fixed", 1e9))
    q = s.calls[5]
    assert q.soft and q.t_rq == pytest.approx(30.1)


def test_S1_hol_accrues_only_until_the_deadline():
    sc = [(0.0, *T(reader=40.0))] + [(0.1 * i, *T(reader=0.01)) for i in range(1, 9)]
    s = script_run(sc, cap=100, admission="refuse", dispatch="P0")
    assert s.calls[8].hol == pytest.approx(10.0)  # waited 0.8 -> 40 s, but only 0.8 -> 10.8 s is inside its deadline


def test_S4_slow_injector_calls_are_excluded_from_metrics():
    from sim import ServiceModel
    svc = ServiceModel({k: [0.001] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                              "search_hybrid_100", "fetch_10", "expand_d3")})
    s = simulate(Config(K=2, n_calls=2000, slow_injector=(20.0, 15.0), admission="fifo_shed", cap=4, dispatch="P1"), svc)
    m = s.metrics()
    assert m["timeout_rate"] == 0.0  # only the injected 15 s reads time out
