import rtc


def test_arms_are_the_preregistered_twenty_plus_six_exploratory():
    a = list(rtc.arms())
    assert len(a) == 26 and len({x[0] for x in a}) == 26
    assert ("old_p90", "p90", None, False) in a and ("rtc_wait_rl60", "wait", 60.0, False) in a
    assert ("rtc_p90_ep09_rl10", "p90", 10.0, True) in a
    # addendum 2026-10-18: p90 with need capped below the wait deadline (review finding B1)
    assert ("rtc_p90cap_rl60", "p90cap", 60.0, False) in a and ("rtc_p90cap_ep09_rl10", "p90cap", 10.0, True) in a
    assert rtc.CHECKS["p90cap"] == ("fifo_shed", ("pct", 90, 128, 0.0, 9.95))


def test_long_tail_keeps_slow_p_and_sets_10_to_30_s_reads():
    for b, i, c in rtc.bands(50, 0):
        lt = rtc.long_tail(c)
        assert lt.slow_p == c.slow_p and 10.0 <= lt.slow_median <= 30.0 and lt.slow_sigma == 0.25


def test_arms_of_one_configuration_differ_only_in_the_arm_knobs():
    js = list(rtc.jobs(1, 0))
    assert len(js) == 2 * 2 * 26
    knobs = {"admission", "threshold", "run_limit", "interrupt", "dispatch", "slow_median", "slow_sigma"}
    ref = vars(js[0][0])
    for cfg, _ in js:
        assert cfg.queue_limit == 32
        assert {k for k, v in vars(cfg).items() if v != ref[k]} <= knobs
    old = [cfg for cfg, t in js if t["arm"] == "old_p90"]
    assert old and all(cfg.run_limit is None for cfg in old)
