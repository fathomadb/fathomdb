"""Inherited host-load admission and descriptive noise observations, without sample filtering."""

import math
import os
import time


def observe_load():
    """Read actual one-minute host load with its observation clocks; unavailable values fail."""
    value = os.getloadavg()[0]
    if not math.isfinite(value) or value < 0:
        raise RuntimeError("actual host load observation is unavailable")
    return {"load": value, "monotonic_ns": time.monotonic_ns(), "observed_at_ns": time.time_ns()}


def summarize_load(observations):
    """Retain observed loads and mark only values strictly above2.5 as noisy."""
    observations = list(observations)
    values = [row["load"] for row in observations]
    return {"metric": "host_one_minute_loadavg", "observations": observations,
            "max_load": max(values, default=None),
            "noisy": any(value > 2.5 for value in values) if values else None,
            "available": bool(values)}
