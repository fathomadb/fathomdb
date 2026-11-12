"""Slice30 sample-level descriptive statistics; no performance verdicts."""

from collections import Counter, defaultdict
import math
import random
import statistics


def distribution(samples):
    """Summarize nanosecond samples using preregistered nearest ranks."""
    values = sorted(samples)
    count = len(values)
    return {
        "count": count,
        "p50_ms": values[math.ceil(0.5 * count) - 1] / 1000000 if count else None,
        "p99_ms": values[math.ceil(0.99 * count) - 1] / 1000000 if count else None,
        "mean_ms": statistics.mean(values) / 1000000 if count else None,
    }


def aggregate(records, wall_ns):
    """Keep success/error latency populations and all-request throughput separate."""
    good = [r["elapsed_ns"] for r in records if r["ok"]]
    bad = [r["elapsed_ns"] for r in records if not r["ok"]]
    return {
        "requests": len(records), "ok": len(good),
        "errors": dict(Counter(str(r.get("error", "unknown")) for r in records if not r["ok"])),
        "throughput_all_per_s": len(records) * 1000000000 / wall_ns if wall_ns else None,
        "successful_latency": distribution(good), "error_latency": distribution(bad),
    }


def paired_difference(records):
    """Compare matched query/repeat adapter observations with clustered intervals."""
    pairs = defaultdict(dict)
    for row in records:
        if row["adapter"] in ("engine", "sdk") and row["ok"]:
            key = (row["repeat"], row.get("sample_index", row["query_index"]))
            pairs[key][row["adapter"]] = row
    differences = []
    clusters = defaultdict(list)
    for adapters in pairs.values():
        if set(adapters) == {"engine", "sdk"}:
            engine, sdk = adapters["engine"], adapters["sdk"]
            delta = (sdk["elapsed_ns"] - engine["elapsed_ns"]) / 1000000
            differences.append(delta)
            clusters[engine["query_index"]].append(delta)
    means = [statistics.mean(values) for values in clusters.values()]
    interval = None
    if means:
        rng = random.Random(20261111)
        boot = sorted(statistics.mean(rng.choices(means, k=len(means))) for _ in range(2000))
        interval = [boot[math.ceil(0.025 * len(boot)) - 1],
                    boot[math.ceil(0.975 * len(boot)) - 1]]
    return {
        "pairs": len(differences), "distinct_queries": len(clusters),
        "unpaired_calls": len(records) - 2 * len(differences),
        "sdk_minus_engine_mean_ms": statistics.mean(differences) if differences else None,
        "bootstrap95_ms": interval, "bootstrap_seed": 20261111, "resamples": 2000,
    }
