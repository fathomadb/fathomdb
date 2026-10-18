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
