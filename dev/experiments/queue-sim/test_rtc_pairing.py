import pytest

import rtc_pairing as rp
from sim import Config, ServiceModel, simulate


def svc():
    return ServiceModel({k: [0.005] for k in ("search_text_10", "search_text_100", "search_hybrid_10",
                                              "search_hybrid_100", "fetch_10", "expand_d3")})


def test_calls_pair_by_client_and_ordinal_under_common_random_numbers():
    base = dict(K=3, n_calls=300, slow_p=0.2, slow_median=12.0, slow_sigma=0.1, seed=4, cap=4, queue_limit=32,
                threshold=("pct", 90, 128, 0.0), warmup_frac=0.0)
    a = simulate(Config(admission="fifo_shed", **base), svc())
    b = simulate(Config(admission="fifo", run_limit=60.0, **base), svc())
    pairs = rp.pair_calls(a, b)
    assert pairs and all(x.cid == y.cid and x.reader == y.reader and x.kind == y.kind for x, y in pairs)


def test_fate_of_former_timeouts():
    base = dict(K=3, n_calls=300, slow_p=0.2, slow_median=12.0, slow_sigma=0.1, seed=4, cap=4, queue_limit=32,
                threshold=("pct", 90, 128, 0.0), warmup_frac=0.0)
    a = simulate(Config(admission="fifo_shed", **base), svc())
    b = simulate(Config(admission="fifo", run_limit=60.0, **base), svc())
    fate = rp.fate(a, b)
    n_to = sum(1 for c in a.calls if c.outcome == "timeout" and c.cid >= 0)
    assert n_to > 0 and sum(fate["counts"].values()) == fate["paired_timeouts"] <= n_to
    assert fate["counts"]["ok_late"] > 0  # 12 s reads that timed out at 10 s now answer after 10 s
    assert all(x > 10.0 for x in fate["late_latency"])
    assert fate["late_from_ok"] >= 0
    assert set(fate["counts"]) == {"ok_on_time", "ok_late", "run_limit", "refused"}
    assert sum(fate["late_hist"]) == fate["counts"]["ok_late"]
    assert fate["late_hist"][0] == pytest.approx(sum(1 for x in fate["late_latency"] if x <= 12.0))
