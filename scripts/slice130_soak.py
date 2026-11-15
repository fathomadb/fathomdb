#!/usr/bin/env python3
"""Slice 130 instrumented-soak math; PASS here never establishes whole-E or shipping proof."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import statistics
import time

MIB = 1048576
SAMPLE_SECONDS = 60
SAMPLE_COUNT = 60
DEADLINE_TOLERANCE_SECONDS = 5


def _number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _slope(points: list[dict], field: str) -> float:
    xs = [p["elapsed_s"] / 60 for p in points]
    ys = [p[field] / MIB for p in points]
    xmean, ymean = statistics.mean(xs), statistics.mean(ys)
    return sum((x - xmean) * (y - ymean) for x, y in zip(xs, ys, strict=True)) / sum((x - xmean) ** 2 for x in xs)


def _median(points: list[dict], field: str) -> float:
    return statistics.median(p[field] / MIB for p in points)


def _status(failures: list[str], invalid: list[str]) -> str:
    return "FAIL" if failures else "UNMEASURED" if invalid else "PASS"


def evaluate_arm(arm: dict) -> dict:
    """Check frozen E measurement criteria, preserving failures and invalid experiment reasons.

    Input has a separate baseline (k=0), samples k=1..60, cumulative operation
    counters, bytes and actual elapsed monotonic seconds. A passive D adapter
    must supply counter_status=ok and checked private counters for pool-on;
    pool-off supplies no_private_pool with null private counters. Missing values
    never become zero. Invalid/missing measurements remain UNMEASURED; observed
    operation, counter or numerical failures remain FAIL even alongside gaps.
    """
    invalid: list[str] = []
    failures: list[str] = []
    samples = arm.get("samples", [])
    baseline = arm.get("baseline")
    result = {"scope": "instrumented_measurement_only", "sample_count": len(samples),
              "terminal_count": 0, "reserved_range_mib": None, "reserved_median_growth_mib": None}
    if arm.get("pool_mode") not in ("on", "off"):
        invalid.append("pool mode must be on or off")
    if not _number(arm.get("started_monotonic_s")):
        invalid.append("missing actual arm start")
    if arm.get("exit_code") != 0 or arm.get("aborted") is not None:
        failures.append("process did not complete with healthy exit")
    if not isinstance(baseline, dict) or len(samples) != SAMPLE_COUNT or not all(isinstance(p, dict) for p in samples):
        invalid.append("separate baseline and exactly 60 scheduled samples required")
    else:
        points = [baseline, *samples]
        if [p.get("k") for p in points] != list(range(61)):
            invalid.append("sample indices must be baseline 0 then exactly 1..60 in order")
        fields = ("elapsed_s", "rss_bytes", "vm_swap_bytes", "pswpin", "pswpout", "errors", "mismatches", "iterations")
        if any(not _number(p.get(f)) or p[f] < 0 for p in points for f in fields):
            invalid.append("missing, negative or nonfinite measurement")
        else:
            if any(abs(p["elapsed_s"] - k * SAMPLE_SECONDS) > DEADLINE_TOLERANCE_SECONDS for k, p in enumerate(points)):
                invalid.append("sample missed its 5-second deadline")
            if any(b["elapsed_s"] <= a["elapsed_s"] or b["iterations"] <= a["iterations"] for a, b in zip(points, points[1:])):
                invalid.append("samples require advancing monotonic time and completed work")
            if any(p["vm_swap_bytes"] != 0 or p["pswpin"] != baseline["pswpin"] or p["pswpout"] != baseline["pswpout"] for p in points):
                invalid.append("process or host swap invalidates the experiment")
            if any(p["errors"] != 0 or p["mismatches"] != 0 for p in points):
                failures.append("operation errors or output mismatches")
            if not invalid:
                terminal = samples[30:]
                result.update(terminal_count=len(terminal), rss_slope_mib_per_min=_slope(terminal, "rss_bytes"),
                              rss_first_five_median_mib=_median(terminal[:5], "rss_bytes"),
                              rss_last_five_median_mib=_median(terminal[-5:], "rss_bytes"))
                result["rss_median_growth_mib"] = result["rss_last_five_median_mib"] - result["rss_first_five_median_mib"]
                if result["rss_slope_mib_per_min"] > 0.5 or result["rss_median_growth_mib"] > 16:
                    failures.append("terminal RSS slope or median growth exceeds frozen bound")
        if arm.get("pool_mode") == "on":
            if any(p.get("counter_status") != "ok" or any(not _number(p.get(f)) or p[f] < 0 for f in ("private_reserved_bytes", "private_used_bytes")) for p in points):
                failures.append("private counter unavailable or read failed")
            elif not invalid:
                terminal = samples[30:]
                result["reserved_range_mib"] = (max(p["private_reserved_bytes"] for p in terminal) - min(p["private_reserved_bytes"] for p in terminal)) / MIB
                result["reserved_median_growth_mib"] = _median(terminal[-5:], "private_reserved_bytes") - _median(terminal[:5], "private_reserved_bytes")
                if result["reserved_range_mib"] > 32 or result["reserved_median_growth_mib"] > 16:
                    failures.append("terminal private reservation range or growth exceeds frozen bound")
        elif arm.get("pool_mode") == "off" and any(p.get("counter_status") != "no_private_pool" or p.get("private_reserved_bytes") is not None or p.get("private_used_bytes") is not None for p in points):
            failures.append("off arm must report named no-private counters, not fabricated zeros")
    result.update(status=_status(failures, invalid), reasons=failures + invalid)
    return result


def evaluate_pair(on: dict, off: dict) -> dict:
    """Check one SDK's sequential on/off control; median uses paired same-k differences.

    Both arm results are retained. Shipping lifecycle/witness and reviewed build
    equivalence remain separate mandatory evidence outside this numerical utility.
    """
    on_result, off_result = evaluate_arm(on), evaluate_arm(off)
    invalid: list[str] = []
    failures: list[str] = []
    result = {"scope": "instrumented_measurement_only", "on": on_result, "off": off_result}
    if on.get("pool_mode") != "on" or off.get("pool_mode") != "off" or not on.get("sdk") or on.get("sdk") != off.get("sdk"):
        invalid.append("pair requires the same SDK in pool-on then pool-off order")
    if on_result["status"] == "FAIL" or off_result["status"] == "FAIL":
        failures.append("one or both arm measurements failed")
    if on_result["status"] == "UNMEASURED" or off_result["status"] == "UNMEASURED":
        invalid.append("one or both arm measurements incomplete or invalid")
    if not invalid and not failures:
        if off["started_monotonic_s"] < on["started_monotonic_s"] + on["samples"][-1]["elapsed_s"]:
            invalid.append("off arm overlaps or precedes on arm")
        result["on_minus_off_slope_mib_per_min"] = on_result["rss_slope_mib_per_min"] - off_result["rss_slope_mib_per_min"]
        result["on_minus_off_median_rss_mib"] = statistics.median((a["rss_bytes"] - b["rss_bytes"]) / MIB for a, b in zip(on["samples"][30:], off["samples"][30:], strict=True))
        if result["on_minus_off_slope_mib_per_min"] > 0.25 or result["on_minus_off_median_rss_mib"] > 64:
            failures.append("on-minus-off RSS control exceeds frozen bound")
    result.update(status=_status(failures, invalid), reasons=failures + invalid)
    return result


def parse_linux_memory(status: str, meminfo: str, vmstat: str) -> dict:
    """Convert explicit procfs counters without treating missing fields as zero."""
    mappings = {"VmRSS": "rss_bytes", "RssAnon": "rss_anon_bytes", "RssFile": "rss_file_bytes",
                "RssShmem": "rss_shmem_bytes", "VmSwap": "vm_swap_bytes", "MemAvailable": "mem_available_bytes"}
    rows = {}
    for text in (status, meminfo):
        for line in text.splitlines():
            name, separator, value = line.partition(":")
            if separator and name in mappings:
                fields = value.split()
                if len(fields) != 2 or fields[1] != "kB" or not fields[0].isdigit():
                    raise ValueError(f"invalid procfs counter {name}")
                rows[mappings[name]] = int(fields[0]) * 1024
    for name, field in mappings.items():
        if field not in rows:
            raise ValueError(f"missing procfs counter {name}")
    for line in vmstat.splitlines():
        fields = line.split()
        if fields and fields[0] in ("pswpin", "pswpout"):
            if len(fields) != 2 or not fields[1].isdigit():
                raise ValueError(f"invalid procfs counter {fields[0]}")
            rows[fields[0]] = int(fields[1])
    for name in ("pswpin", "pswpout"):
        if name not in rows:
            raise ValueError(f"missing procfs counter {name}")
    return rows


def read_linux_memory(proc_root: Path = Path("/proc")) -> dict:
    """Read the current process and host counters; missing/unreadable data raises.

    No GPU calls or allocation churn are introduced. Procfs kB values are KiB;
    pswpin/pswpout retain their native cumulative page counts, never bytes.
    """
    return parse_linux_memory((proc_root / "self/status").read_text(),
                              (proc_root / "meminfo").read_text(), (proc_root / "vmstat").read_text())


def collect_arm(sdk, pool_mode, iteration, snapshot, emit, *, clock=time.monotonic):
    """Run one warm-up and the frozen 60-sample schedule between completed iterations.

    Caller preloads real models and supplies an iteration returning latency_ms,
    outputs and a per-iteration mismatches count. snapshot returns memory metrics
    plus an untouched raw passive observation and its checked counter adapter.
    emit must stream each event to retained JSONL (including every latency/output
    record); this loop retains only baseline plus 60 snapshots. Neither callable
    may manufacture completion through extra forward/trim/synchronize/GC work.
    Exceptions, mismatches and late snapshots stop immediately with raw events.
    The caller closes real SDK resources; a reaping supervisor must attach the
    actual exit_code afterward. This function never claims successful process exit.
    """
    if pool_mode not in ("on", "off"):
        raise ValueError("pool mode must be on or off")
    result = {"sdk": sdk, "pool_mode": pool_mode, "baseline": None,
              "samples": [], "aborted": None, "exit_code": None}
    iterations = 0
    mismatches = 0
    phase = "warmup"
    try:
        warmup_start = clock()
        warmup = iteration()
        result["warmup_duration_s"] = clock() - warmup_start
        emit({"event": "warmup", "result": warmup, "duration_s": result["warmup_duration_s"]})
        if warmup["mismatches"]:
            result["aborted"] = "warmup output mismatch"
            return result
        started = clock()
        result["started_monotonic_s"] = started

        def capture(k):
            sample_start = clock()
            point = dict(snapshot())
            observed = clock()
            point.update(k=k, elapsed_s=observed - started, sampler_duration_s=observed - sample_start,
                         iterations=iterations, errors=0, mismatches=mismatches)
            emit({"event": "baseline" if k == 0 else "sample", "point": point})
            return point

        phase = "baseline"
        result["baseline"] = capture(0)
        if result["baseline"]["elapsed_s"] > DEADLINE_TOLERANCE_SECONDS:
            result["aborted"] = "baseline deadline exceeded"
            return result
        k = 1
        while k <= SAMPLE_COUNT:
            phase = "iteration"
            details = iteration()
            iterations += 1
            mismatches += details["mismatches"]
            emit({"event": "iteration", "iteration": iterations, "elapsed_s": clock() - started,
                  "result": details})
            if mismatches:
                result["aborted"] = "operation output mismatch"
                break
            if clock() - started < k * SAMPLE_SECONDS:
                continue
            phase = "sample"
            point = capture(k)
            result["samples"].append(point)
            if point["elapsed_s"] > k * SAMPLE_SECONDS + DEADLINE_TOLERANCE_SECONDS:
                result["aborted"] = f"sample {k} deadline exceeded"
                break
            k += 1
    except Exception as error:  # noqa: BLE001 - retain the actual workload/observer failure
        result["aborted"] = f"{phase}: {type(error).__name__}: {error}"
        emit({"event": "error", "phase": phase, "type": type(error).__name__, "message": str(error)})
    result["iterations"] = iterations
    return result


def main() -> int:
    """Evaluate an existing JSON SDK pair; never start a campaign or import native SDKs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pair", type=Path, help="JSON object with on and off arm records")
    args = parser.parse_args()
    pair = json.loads(args.pair.read_text())
    result = evaluate_pair(pair["on"], pair["off"])
    print(json.dumps(result, indent=2, allow_nan=False))
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
