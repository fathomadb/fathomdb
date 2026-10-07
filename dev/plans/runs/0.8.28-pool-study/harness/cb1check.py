"""CB1 (pool cap) checks, owner ruling 2026-10-06 (protocol revision 5).

The primary check is the pool's own bound: its `reserved_high` never exceeds
its `maxSize`. The system-level measure is a sanity check only, and only on
an integrated GPU (`CU_DEVICE_ATTRIBUTE_INTEGRATED` = 1, read at runtime and
reported in the `install` event). Elsewhere, or without its source, it is
skipped with a reason; it never fails a row. It compares how much
`MemAvailable` a process consumed between "before open" and "after rerank"
with the same quantity in the shipped synchronous-path processes of the same
series. A noise band from the baseline's spread absorbs page-cache and
other-process movement; when the band is too wide the result is
"inconclusive", not a failure.
"""

from __future__ import annotations

import statistics

# Allowed excess over the pool's reserved high-water mark (MiB).
SLACK_MIB = 32
# Smallest noise band, for a baseline that happens not to vary (MiB).
NOISE_FLOOR_MIB = 16
# The noise band is this many median absolute deviations of the baseline.
NOISE_MAD_FACTOR = 3
# A wider noise band makes the sanity check inconclusive (MiB).
INCONCLUSIVE_NOISE_MIB = 256
# Fewest baseline processes the comparison needs.
MIN_BASELINE = 5


def primary(reserved_high: int | None, max_size: int) -> str:
    """'pass' when the pool's high-water mark stays within its maxSize."""
    if reserved_high is None:
        return "unmeasured"
    return "pass" if reserved_high <= max_size else "fail"


def noise_band(baseline_mib: list[float]) -> float:
    med = statistics.median(baseline_mib)
    mad = statistics.median(abs(x - med) for x in baseline_mib)
    return max(float(NOISE_FLOOR_MIB), NOISE_MAD_FACTOR * mad)


def sanity(
    integrated: bool | None,
    delta_mib: float | None,
    baseline_mib: list[float],
    reserved_high_mib: float,
) -> tuple[str, str]:
    """(status, reason): status is skipped, inconclusive, consistent or
    inconsistent. `delta_mib` is the process's MemAvailable drop between
    before-open and after-rerank; `baseline_mib` the same drop in the
    shipped synchronous-path processes of the same series."""
    if integrated is None:
        return ("skipped", "integrated flag not reported")
    if not integrated:
        return ("skipped", "discrete GPU: maxSize bounds device memory, not host memory")
    if delta_mib is None:
        return ("skipped", "MemAvailable not recorded at both points")
    if len(baseline_mib) < MIN_BASELINE:
        return ("inconclusive", f"baseline has {len(baseline_mib)} processes, fewer than {MIN_BASELINE}")
    band = noise_band(baseline_mib)
    if band > INCONCLUSIVE_NOISE_MIB:
        return ("inconclusive", f"noise band {band:.0f} MiB exceeds {INCONCLUSIVE_NOISE_MIB} MiB")
    excess = delta_mib - statistics.median(baseline_mib) - reserved_high_mib
    if excess <= SLACK_MIB + band:
        return ("consistent", f"excess {excess:.0f} MiB within {SLACK_MIB} + {band:.0f} MiB")
    return ("inconsistent", f"excess {excess:.0f} MiB over {SLACK_MIB} + {band:.0f} MiB")
