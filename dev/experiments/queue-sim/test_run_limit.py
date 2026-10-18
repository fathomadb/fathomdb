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
