"""Pure tests for the CB1 check (owner ruling 2026-10-06, revision 5).

Run: python -m pytest dev/plans/runs/0.8.28-pool-study/harness/test_cb1check.py
"""

from __future__ import annotations

import cb1check as cb

MIB = 1 << 20


def test_primary_check_is_the_pool_bound() -> None:
    assert cb.primary(reserved_high=160 * MIB, max_size=3 << 30) == "pass"
    assert cb.primary(reserved_high=3 << 30, max_size=3 << 30) == "pass"
    assert cb.primary(reserved_high=(3 << 30) + 1, max_size=3 << 30) == "fail"
    assert cb.primary(reserved_high=None, max_size=3 << 30) == "unmeasured"


def test_sanity_is_skipped_unless_the_device_is_integrated() -> None:
    base = [100.0] * 10
    assert cb.sanity(integrated=False, delta_mib=150.0, baseline_mib=base, reserved_high_mib=160.0) == (
        "skipped",
        "discrete GPU: maxSize bounds device memory, not host memory",
    )
    assert cb.sanity(integrated=None, delta_mib=150.0, baseline_mib=base, reserved_high_mib=160.0) == (
        "skipped",
        "integrated flag not reported",
    )


def test_sanity_is_skipped_without_its_source() -> None:
    assert cb.sanity(integrated=True, delta_mib=None, baseline_mib=[1.0] * 10, reserved_high_mib=160.0)[0] == "skipped"


def test_sanity_needs_a_baseline() -> None:
    status, why = cb.sanity(integrated=True, delta_mib=150.0, baseline_mib=[100.0] * 3, reserved_high_mib=160.0)
    assert status == "inconclusive"
    assert "baseline" in why


def test_noisy_baselines_are_inconclusive_not_failures() -> None:
    noisy = [0.0, 400.0, -300.0, 800.0, 50.0, -600.0, 900.0]
    status, _ = cb.sanity(integrated=True, delta_mib=5000.0, baseline_mib=noisy, reserved_high_mib=160.0)
    assert status == "inconclusive"


def test_consistent_and_inconsistent_classification() -> None:
    base = [100.0, 102.0, 98.0, 101.0, 99.0, 100.0]
    # The pool adds at most its reserved high-water mark plus the slack.
    assert cb.sanity(integrated=True, delta_mib=100.0 + 160.0, baseline_mib=base, reserved_high_mib=160.0)[0] == "consistent"
    assert cb.sanity(integrated=True, delta_mib=100.0 + 40.0, baseline_mib=base, reserved_high_mib=160.0)[0] == "consistent"
    status, why = cb.sanity(integrated=True, delta_mib=100.0 + 900.0, baseline_mib=base, reserved_high_mib=160.0)
    assert status == "inconsistent"
    assert "excess" in why


def test_the_noise_band_never_drops_below_its_floor() -> None:
    assert cb.noise_band([100.0] * 10) == cb.NOISE_FLOOR_MIB
    assert cb.noise_band([0.0, 100.0, 0.0, 100.0, 0.0, 100.0]) == cb.NOISE_MAD_FACTOR * 50.0


def test_thresholds_are_named_constants() -> None:
    assert cb.SLACK_MIB == 32
    assert cb.NOISE_FLOOR_MIB == 16
    assert cb.NOISE_MAD_FACTOR == 3
    assert cb.INCONCLUSIVE_NOISE_MIB == 256
    assert cb.MIN_BASELINE == 5
