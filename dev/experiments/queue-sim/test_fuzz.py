import math

import fuzz


def test_B2_realistic_band_samples_slow_p_log_uniform_1e3_to_1e2():
    ps = [fuzz.real_config(i).slow_p for i in range(500)]
    assert min(ps) >= 1e-3 and max(ps) <= 1e-2
    logs = sorted(math.log10(p) for p in ps)
    assert abs(logs[len(logs) // 2] - (-2.5)) < 0.1  # log-uniform: median near 10**-2.5


def test_realistic_band_is_k_le_8_gpu_closed_loop():
    for i in range(100):
        c = fuzz.real_config(i)
        assert 1 <= c.K <= 8 and c.speed_mult == 1.0 and c.arrivals == "closed"
