"""Run to completion (queue-admission.md § 6.1, owner 2026-10-18): the wait deadline bounds waiting and
starting only; a started call runs until it finishes or passes run_limit, counted from the permit.
run_limit=None keeps the old rule (the deadline also applies to a running call)."""
import pytest

from sim import Config, ServiceModel, simulate


def script_run(script, **kw):
    cfg = Config(arrivals="script", script=script, n_calls=10**9, warmup_frac=0.0, **kw)
    return simulate(cfg)


def T(kind="search_text", reader=0.01, embed=0.0):
    return (kind, embed, reader)


P90 = ("pct", 90, 128, 0.0)


def flat_svc(x=0.005):
    return ServiceModel({k: [x] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                          "search_hybrid_100", "fetch_10", "expand_d3")})


# ---------------------------------------------------------------- the rule


def test_a_started_call_past_the_wait_deadline_returns_its_result():
    s = script_run([(0.0, *T(reader=12.0))], cap=1, admission="fifo_shed", threshold=P90, run_limit=60.0)
    a = s.calls[0]
    assert a.outcome == "ok" and a.t_res == pytest.approx(12.0)
    assert s.ab_integral == 0.0
    assert s.est["search_text"].count == 1  # a late completion is still a service-time sample


def test_a_running_call_is_abandoned_at_the_run_limit_and_keeps_its_permit():
    sc = [(0.0, *T(reader=70.0)), (61.0, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0)
    a, b = s.calls
    assert a.outcome == "timeout" and a.t_res == pytest.approx(60.0)
    assert b.t_adm == pytest.approx(70.0) and b.outcome == "ok"  # waited for the abandoned read
    assert s.ab_integral == pytest.approx(10.0)
    assert s.est["search_text"].count == 1  # the abandoned call left no sample; b did


def test_the_run_limit_is_counted_from_the_permit_not_from_arrival():
    sc = [(0.0, *T(reader=5.0)), (0.1, *T(reader=62.0))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0)
    b = s.calls[1]
    assert b.t_adm == pytest.approx(5.0)
    assert b.outcome == "timeout" and b.t_res == pytest.approx(65.0)


def test_wait_only_start_check_starts_a_call_with_little_time_left():
    # permit frees at 9.95 s; b arrived at 0.1 s, has 0.15 s of wait deadline left, starts and runs 2 s
    sc = [(0.0, *T(reader=9.95)), (0.1, *T(reader=2.0))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0)
    b = s.calls[1]
    assert b.t_adm == pytest.approx(9.95) and b.outcome == "ok" and b.t_res == pytest.approx(11.95)


def test_p90_start_check_still_sheds_under_run_to_completion():
    # 8 trained 2 s calls -> need = p90 = 2 s. Permit frees at 29.5 s; c arrived at 20.0, so 0.5 s left < 2 s
    sc = [(i * 2.0, *T(reader=2.0)) for i in range(8)]
    sc += [(16.0, *T(reader=13.5)), (20.0, *T(reader=1.0))]
    s = script_run(sc, cap=1, admission="fifo_shed", threshold=P90, run_limit=60.0)
    c = s.calls[9]
    assert s.calls[8].outcome == "ok" and s.calls[8].t_res == pytest.approx(29.5)
    assert c.reason == "deadline" and c.t_res == pytest.approx(30.0 - 2.0)


def test_ep09_interrupts_at_the_run_limit_and_frees_the_permit():
    sc = [(0.0, *T(reader=70.0)), (61.0, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0, interrupt=True)
    a, b = s.calls
    assert a.outcome == "timeout" and a.t_res == pytest.approx(60.0)
    assert b.t_adm == pytest.approx(61.0) and b.t_rs == pytest.approx(61.0)
    assert s.ab_integral == 0.0



# ---------------------------------------------------------------- metrics
def test_late_success_metrics():
    # three calls on separate permits: 5 s (on time), 13 s and 40 s (late successes)
    sc = [(0.0, *T(reader=5.0)), (0.0, *T(reader=13.0)), (0.0, *T(reader=40.0))]
    s = script_run(sc, cap=4, admission="fifo", run_limit=60.0)
    m = s.metrics()
    assert m["ok_rate"] == 1.0
    assert m["ok_late_rate"] == pytest.approx(2 / 3)  # answered after the 10 s wait deadline
    assert m["ok_late30_rate"] == pytest.approx(1 / 3)
    assert m["ok_late_share"] == pytest.approx(2 / 3)
    assert m["late_h_10_12"] == 0 and m["late_h_12_15"] == 1 and m["late_h_30_60"] == 1
    assert m["ok_p999"] == pytest.approx(40.0)


def test_abandoned_work_and_fail_metrics():
    sc = [(0.0, *T(reader=70.0)), (61.0, *T(reader=0.1))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0)
    m = s.metrics()
    assert m["fail_rate"] == pytest.approx(0.5)
    assert m["ab_permit_s_per_kcall"] == pytest.approx(10.0 / 2 * 1000)
    assert m["wasted_permit_s_per_kcall"] == pytest.approx(70.0 / 2 * 1000)  # the whole hold of the abandoned call
    assert m["goodput_per_s"] == pytest.approx(1 / 70.1)


def test_late_start_success_is_counted():
    sc = [(0.0, *T(reader=9.0)), (0.5, *T(reader=3.0))]
    s = script_run(sc, cap=1, admission="fifo", run_limit=60.0)
    m = s.metrics()
    assert s.calls[1].t_res == pytest.approx(12.0)
    assert m["late_start_ok_rate"] == pytest.approx(0.5)


def test_turn_failed_counts_turns_with_any_failed_call():
    cfg = Config(K=1, cap=4, admission="refuse", session_prefix=(1, 10), session_turns=2, n_calls=11,
                 warmup_frac=0.0, hybrid_frac=0.0)
    m = simulate(cfg, flat_svc()).metrics()
    assert m["turn_failed"] == pytest.approx(0.5)


def test_time_to_data_charges_a_failure_one_think_and_one_retry():
    # a: ok in 1 s; b: refused at once at 0.5 s. ttd(b) = 0 + think_median 3 s + mean ok latency 1 s
    sc = [(0.0, *T(reader=1.0)), (0.5, *T())]
    s = script_run(sc, cap=1, admission="refuse", think_median=3.0)
    m = s.metrics()
    assert m["ttd_mean"] == pytest.approx((1.0 + 4.0) / 2)
    assert m["ttd_p99"] == pytest.approx(4.0)


def test_false_refusal_under_run_to_completion_means_it_would_have_started_in_time():
    # p90 sheds c at 28.0; the permit frees at 29.5, before c's wait deadline 30.0: under run to completion
    # c would have started and answered, so the shed counts as a false refusal
    sc = [(i * 2.0, *T(reader=2.0)) for i in range(8)]
    sc += [(16.0, *T(reader=13.5)), (20.0, *T(reader=1.0))]
    s = script_run(sc, cap=1, admission="fifo_shed", threshold=P90, run_limit=60.0)
    assert s.metrics()["false_refusal_rate"] == pytest.approx(1 / 10)


def test_timeout_cause_window_ends_at_the_run_limit():
    # b waits 6 s for a, then runs 70 s and is abandoned at 66 s: inside [0, 66] its own run (60 s) dominates
    # its admission wait (6 s). With the window cut at the 10 s wait deadline, admission would win (6 > 4)
    s = script_run([(0.0, *T(reader=6.0)), (0.0, *T(reader=70.0))], cap=1, admission="fifo", run_limit=60.0)
    m = s.metrics()
    assert s.calls[1].outcome == "timeout" and s.calls[1].t_res == pytest.approx(66.0)
    assert m["to_run"] == pytest.approx(0.5) and m["to_admission"] == 0.0
