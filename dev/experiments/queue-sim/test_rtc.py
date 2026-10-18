import rtc


def test_arms_are_the_preregistered_twenty():
    a = list(rtc.arms())
    assert len(a) == 20 and len({x[0] for x in a}) == 20
    assert ("old_p90", "p90", None, False) in a and ("rtc_wait_rl60", "wait", 60.0, False) in a
    assert ("rtc_p90_ep09_rl10", "p90", 10.0, True) in a


def test_wait_only_arm_starts_any_call_with_a_permit_before_its_deadline():
    assert rtc.CHECKS["wait"][0] == "fifo" and rtc.CHECKS["p90"][0] == "fifo_shed"


def test_stress_band_is_main_fuzz_k16_slow2():
    st = [c for b, _, c in rtc.bands(0, 300) if b == "stress"]
    assert st and all(c.K >= 16 and c.slow_p >= 0.02 for c in st)


def test_long_tail_keeps_slow_p_and_sets_10_to_30_s_reads():
    for b, i, c in rtc.bands(50, 0):
        lt = rtc.long_tail(c, i)
        assert lt.slow_p == c.slow_p and 10.0 <= lt.slow_median <= 30.0 and lt.slow_sigma == 0.25


def test_each_config_yields_paired_jobs_with_queue_32():
    js = list(rtc.jobs(1, 0))
    assert len(js) == 2 * 2 * 20
    assert all(cfg.queue_limit == 32 and cfg.seed == js[0][0].seed for cfg, _ in js)
    old = [cfg for cfg, t in js if t["arm"] == "old_p90"]
    assert all(cfg.run_limit is None for cfg in old)
