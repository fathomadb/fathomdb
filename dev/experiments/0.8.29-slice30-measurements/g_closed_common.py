"""Supervised common-work entry; shared-budget attribution is not yet implemented."""

from pathlib import Path
import time

from g_attempt import run_g_attempt


def run_common(allocation, mode, command):
    """Execute common work in an owned bounded child; caller owns the allocation lock."""
    return run_g_attempt({"id": "g-common-" + mode}, Path(allocation) / "common", command,
                         time.monotonic_ns() + 600 * 10**9)
