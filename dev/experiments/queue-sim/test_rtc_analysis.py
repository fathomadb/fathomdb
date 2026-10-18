import rtc
import rtc_analysis as ra


def test_paired_ci_brackets_a_constant_difference():
    lo, mean, hi = ra.paired_ci([1.0, 2.0, 3.0, 4.0], [0.5, 1.5, 2.5, 3.5])
    assert lo == mean == hi == 0.5


def test_paired_ci_excludes_zero_only_when_the_difference_is_consistent():
    a = [0.0] * 50 + [1.0] * 50
    b = [0.0] * 100
    lo, mean, hi = ra.paired_ci(a, b)
    assert lo > 0 and abs(mean - 0.5) < 1e-12
    lo, _, hi = ra.paired_ci([1.0, -1.0] * 50, [0.0] * 100)
    assert lo < 0 < hi


def test_pair_aligns_two_arms_by_configuration():
    rows = [{"band": "realistic", "cfg_i": "1", "tail": "base", "dispatch": "P2", "arm": "a", "fail_rate": "0.1"},
            {"band": "realistic", "cfg_i": "1", "tail": "base", "dispatch": "P2", "arm": "b", "fail_rate": "0.3"},
            {"band": "realistic", "cfg_i": "2", "tail": "base", "dispatch": "P2", "arm": "a", "fail_rate": "0.2"}]
    x, y = ra.pair(ra.index(rows), "realistic", "base", "P2", "a", "b", "fail_rate")
    assert x == [0.1] and y == [0.3]  # cfg 2 has no arm b: dropped from the pair


# ---------------------------------------------------------------- decision rules on synthetic results
ARMS = [a for a, *_ in rtc.arms()]
BASE = {"fail_rate": 0.001, "refuse_rate": 0.001, "timeout_rate": 0.0, "ttd_p99": 1.0, "ok_late_share": 0.0,
        "ok_late30_rate": 0.0, "rf_with_permit": 0.0}


def synth(over=None, n=40):
    """Rows for every arm of the realistic band; over[(tail, dispatch, arm)] = {metric: value or f(cfg_i)}."""
    over = over or {}
    rows = []
    for i in range(n):
        for t in ("base", "long"):
            for d in ("P2", "P0"):
                for a in ARMS:
                    r = {"band": "realistic", "cfg_i": str(i), "tail": t, "dispatch": d, "arm": a, **BASE}
                    for k, v in over.get((t, d, a), {}).items():
                        r[k] = v(i) if callable(v) else v
                    rows.append({k: str(v) for k, v in r.items()})
    return ra.index(rows)


def test_r1_drops_the_check_when_wait_only_fails_no_more_and_the_guard_holds():
    r = ra.r1(synth({("base", "P2", "rtc_p90_rl60"): {"fail_rate": 0.002}}))
    assert r["chk"] == "wait" and r["verdict"].startswith("DROP")


def test_r1_keeps_a_percentile_check_and_picks_the_lowest_failing_one():
    ix = synth({("base", "P2", "rtc_p90_rl60"): {"fail_rate": lambda i: 0.0005 + 1e-6 * i},
                ("base", "P2", "rtc_p95_rl60"): {"fail_rate": 0.0004},
                ("base", "P2", "rtc_p99_rl60"): {"fail_rate": 0.0003, "ttd_p99": 2.0}})
    r = ra.r1(ix)
    assert r["verdict"].startswith("KEEP") and r["chk"] == "p95"  # p99's time to data is 2x wait-only's


def test_r1_reports_to_the_owner_when_the_late_answer_guard_fails():
    r = ra.r1(synth({("long", "P2", "rtc_wait_rl60"): {"ok_late30_rate": 0.002}}))
    assert r["verdict"].startswith("REPORT")


def test_r1_can_exclude_configurations_and_compare_another_check():
    ix = synth({("base", "P2", "rtc_p90_rl60"): {"fail_rate": lambda i: 0.2 if i < 2 else 0.0005 + 1e-6 * i}})
    assert ra.r1(ix)["chk"] == "wait"
    assert ra.r1(ix, exclude={"0", "1"})["verdict"].startswith("KEEP")
    assert ra.r1(ix, check="p90cap")["chk"] == "wait"  # p90cap equals wait-only here


def test_lockups_are_configurations_refusing_more_than_1_percent_with_a_free_permit():
    ix = synth({("base", "P2", "rtc_p90_rl60"): {"rf_with_permit": lambda i: 0.02 if i in (3, 7) else 0.0}})
    assert ra.lockups(ix, "realistic", "base", "P2", "rtc_p90_rl60") == {"3", "7"}


def test_lockups_count_arrival_refusals_only_when_recorded():
    ix = synth({("base", "P2", "rtc_p90_rl60"): {"rf_with_permit": 0.05,
                                                 "rf_arrival_free": lambda i: 0.02 if i == 4 else 0.0}})
    assert ra.lockups(ix, "realistic", "base", "P2", "rtc_p90_rl60") == {"4"}


def test_r2_picks_the_smallest_run_limit_meeting_both_tails():
    assert ra.r2(synth({("long", "P2", "rtc_wait_rl30"): {"timeout_rate": 0.001}}), "wait") == (60, True)
    ix = synth({("long", "P2", f"rtc_wait_rl{r}"): {"timeout_rate": 0.001} for r in (30, 60, 120)})
    assert ra.r2(ix, "wait") == (120, False)


def test_load_jsonl_keeps_only_configurations_with_every_arm(tmp_path):
    import json
    p = tmp_path / "x.jsonl"
    full = [{"band": "realistic", "cfg_i": 0, "arm": a, "tail": t, "dispatch": d, "_key": "k"}
            for a in ARMS for t in ("base", "long") for d in ("P0", "P2")]
    part = [dict(full[0], cfg_i=1)]
    p.write_text("\n".join(json.dumps(r) for r in full + part) + '\n{"_key": "e", "_error": "x"}\n{"torn')
    rows = ra.load(str(p))
    assert len(rows) == len(full) and {r["cfg_i"] for r in rows} == {"0"} and "_key" not in rows[0]


def test_r4_uses_p90_ep09_arms_when_the_check_has_none():
    chk, pick = ra.r4(synth({("base", "P2", "rtc_p90_ep09_rl10"): {"fail_rate": 0.5}}), "p95")
    assert chk == "p90" and pick == 30
