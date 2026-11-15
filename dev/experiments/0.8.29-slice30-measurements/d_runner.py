"""Guarded D fixed-table entry; runtime selection remains separate from source controls."""

import time

from m_guard import guard


def run_guarded(command, *, allocation, profile, root, task, **options):
    """Own D controller custody and reconcile its shared outer charge.

    The selected existing allocation/profile must fund all work and cleanup.
    Return the actual guardian result; unknown ownership blocks redispatch.
    This callable does not select a scientific profile or grant campaign GO.
    """
    began = options.get("started_ns")
    if began is None:
        began = time.monotonic_ns()
    try:
        result = guard(command, allocation=allocation, profile=profile, root=root, task=task,
                       phase="D", **dict(options, started_ns=began))
        result.setdefault("elapsed_ns", time.monotonic_ns()-began)
        result.setdefault("charged_ns", result.get("accounting", {}).get("charged_ns"))
        return result
    except (OSError, ValueError, RuntimeError, KeyError, TypeError) as error:
        return {"exit": 125, "state": "held", "reason": f"{type(error).__name__}: {error}",
                "elapsed_ns": time.monotonic_ns()-began, "charged_ns": None,
                "cleanup_uncertain": None, "unreconciled": True}
