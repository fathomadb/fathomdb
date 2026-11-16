#!/usr/bin/env python3
"""Slice 130 instrumented-soak math; PASS here never establishes whole-E or shipping proof."""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import selectors
import shutil
import signal
import subprocess
import threading
from concurrent.futures import ThreadPoolExecutor
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


def _counter_failure(point: dict, pool_mode: str | None) -> str | None:
    status = point.get("counter_status")
    if "raw_observation" in point and status is not None:
        try:
            checked = adapt_observation(point["raw_observation"], pool_mode)
            for field in ("counter_status", "private_used_bytes", "private_reserved_bytes"):
                if point.get(field) != checked[field]:
                    return f"counter adapter differs from raw observation: {field}"
        except (ValueError, TypeError) as error:
            return f"raw counter observation invalid ({status}): {error}"
    if pool_mode == "on" and (status != "ok" or any(not _number(point.get(f)) or point[f] < 0 for f in ("private_reserved_bytes", "private_used_bytes"))):
        return f"private counter unavailable or read failed: {status}"
    if pool_mode == "off" and (status != "no_private_pool" or point.get("private_reserved_bytes") is not None or point.get("private_used_bytes") is not None):
        return f"off counter observation invalid (requires no-private, not fabricated zeros): {status}"
    return None


def _u64(value, field):
    if not isinstance(value, str) or not re.fullmatch(r"0|[1-9][0-9]*", value):
        raise ValueError(f"{field} is not a lossless decimal string")
    number = int(value)
    if number > 2**64 - 1:
        raise ValueError(f"{field} exceeds u64")
    return number


def adapt_observation(raw: dict, pool_mode: str, *, pid=None, expected=None) -> dict:
    """Bind a passive native observation to exact counters and stable identity.

    This never calls a driver or fills unavailable counters with zero. The raw
    record remains attached, including decimal-string identities at u64 max.
    """
    if not isinstance(raw, dict) or raw.get("schema") != "fathomdb.cuda-pool-observation/v1":
        raise ValueError("missing actual pool observation schema")
    actual_pid = raw.get("pid")
    if type(actual_pid) is not int or actual_pid <= 0 or (pid is not None and actual_pid != pid):
        raise ValueError("observation PID differs from the worker")
    result = {"raw_observation": raw, "counter_status": raw.get("status"),
              "private_used_bytes": None, "private_reserved_bytes": None}
    if pool_mode == "off":
        if raw.get("status") != "no_private_pool" or any(raw.get(k) is not None for k in ("used_current_bytes", "reserved_current_bytes")):
            raise ValueError("off requires actual no_private_pool with unavailable counters")
        result["identity"] = {"pid": actual_pid}
        return result
    if pool_mode != "on" or raw.get("status") != "private":
        raise ValueError(f"private observation unavailable: {raw.get('status')}")
    identity = {key: raw.get(key) for key in ("pid", "uuid", "device_ordinal", "context_id", "pool_id")}
    if type(identity["device_ordinal"]) is not int or identity["device_ordinal"] < 0:
        raise ValueError("missing decided device ordinal")
    if any(not isinstance(identity[k], str) or not identity[k] for k in ("uuid", "pool_id")):
        raise ValueError("missing checked UUID or pool identity")
    _u64(identity["context_id"], "context_id")
    before = _u64(raw.get("live_models_before"), "live_models_before")
    after = _u64(raw.get("live_models_after"), "live_models_after")
    if before != after:
        raise ValueError("ownership changed during observation")
    if expected is not None and identity != expected:
        raise ValueError("pool/context/device identity changed")
    result.update(identity=identity, live_models=after, counter_status="ok",
                  private_used_bytes=_u64(raw.get("used_current_bytes"), "used_current_bytes"),
                  private_reserved_bytes=_u64(raw.get("reserved_current_bytes"), "reserved_current_bytes"))
    return result


def require_first_close(before: dict, after: dict) -> None:
    """Require max-pool exact reservation retention before any survivor forward."""
    if (before["identity"] != after["identity"] or before["live_models"] != 2
            or after["live_models"] != 1 or before["private_reserved_bytes"] <= 0
            or after["private_reserved_bytes"] != before["private_reserved_bytes"]):
        raise ValueError("first close changed reservation/identity or did not release exactly one owner")


def wait_for_zero(observe, emit, *, expected, started_at=None, clock=time.monotonic, sleep=time.sleep):
    """Poll passive last-release observations for at most 5s, without CUDA work."""
    started = clock() if started_at is None else started_at
    while True:
        raw = observe()
        emit({"event": "last_release", "elapsed_s": clock() - started, "raw_observation": raw})
        point = adapt_observation(raw, "on", expected=expected)
        if clock() - started > 5:
            raise TimeoutError("last-release observation exceeded 5s")
        if point["live_models"] == point["private_used_bytes"] == point["private_reserved_bytes"] == 0:
            return point
        if clock() - started >= 5:
            raise TimeoutError("last release did not reach live/USED/RESERVED zero within 5s")
        sleep(min(0.05, max(0, 5 - (clock() - started))))


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
    if arm.get("exit_code") is None:
        invalid.append("actual process exit has not been observed")
    elif arm["exit_code"] != 0 and arm.get("abort_kind") not in ("resource", "deadline"):
        failures.append("process did not complete with healthy exit")
    if arm.get("aborted") is not None:
        if arm.get("abort_kind") in ("deadline", "resource"):
            invalid.append(f"{arm.get('abort_kind')} invalidated the experiment")
        else:
            failures.append(f"campaign aborted: {arm['aborted']}")
    available_points = ([baseline] if isinstance(baseline, dict) else []) + [p for p in samples if isinstance(p, dict)]
    if any(_number(p.get(f)) and p[f] > 0 for p in available_points for f in ("errors", "mismatches")):
        failures.append("operation errors or output mismatches")
    counter_failures = list(dict.fromkeys(reason for p in available_points if (reason := _counter_failure(p, arm.get("pool_mode")))))
    failures.extend(counter_failures)
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
            if not invalid:
                terminal = samples[30:]
                result.update(terminal_count=len(terminal), rss_slope_mib_per_min=_slope(terminal, "rss_bytes"),
                              rss_first_five_median_mib=_median(terminal[:5], "rss_bytes"),
                              rss_last_five_median_mib=_median(terminal[-5:], "rss_bytes"))
                result["rss_median_growth_mib"] = result["rss_last_five_median_mib"] - result["rss_first_five_median_mib"]
                if result["rss_slope_mib_per_min"] > 0.5 or result["rss_median_growth_mib"] > 16:
                    failures.append("terminal RSS slope or median growth exceeds frozen bound")
        if arm.get("pool_mode") == "on":
            if not invalid and not counter_failures:
                terminal = samples[30:]
                result["reserved_range_mib"] = (max(p["private_reserved_bytes"] for p in terminal) - min(p["private_reserved_bytes"] for p in terminal)) / MIB
                result["reserved_median_growth_mib"] = _median(terminal[-5:], "private_reserved_bytes") - _median(terminal[:5], "private_reserved_bytes")
                if result["reserved_range_mib"] > 32 or result["reserved_median_growth_mib"] > 16:
                    failures.append("terminal private reservation range or growth exceeds frozen bound")
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
    Exceptions, mismatches, failed checked counters and late snapshots stop
    immediately with raw events. A snapshot with no checked adapter is diagnostic
    only and cannot pass evaluate_arm; this loop does not invent a D interface.
    The caller closes real SDK resources; a reaping supervisor must attach the
    actual exit_code afterward. This function never claims successful process exit.
    """
    if pool_mode not in ("on", "off"):
        raise ValueError("pool mode must be on or off")
    result = {"sdk": sdk, "pool_mode": pool_mode, "baseline": None,
              "samples": [], "aborted": None, "abort_kind": None, "exit_code": None}
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
            result["abort_kind"] = "output_mismatch"
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

        def stop_for_counter_failure(point):
            if "counter_status" not in point:
                return False
            failure = _counter_failure(point, pool_mode)
            if failure is None:
                return False
            result["aborted"] = failure
            result["abort_kind"] = "counter_error"
            emit({"event": "error", "phase": phase, "type": "counter_error", "message": failure})
            return True

        phase = "baseline"
        result["baseline"] = capture(0)
        if stop_for_counter_failure(result["baseline"]):
            return result
        if result["baseline"]["elapsed_s"] > DEADLINE_TOLERANCE_SECONDS:
            result["aborted"] = "baseline deadline exceeded"
            result["abort_kind"] = "deadline"
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
                result["abort_kind"] = "output_mismatch"
                break
            if clock() - started < k * SAMPLE_SECONDS:
                continue
            phase = "sample"
            point = capture(k)
            result["samples"].append(point)
            if stop_for_counter_failure(point):
                break
            if point["elapsed_s"] > k * SAMPLE_SECONDS + DEADLINE_TOLERANCE_SECONDS:
                result["aborted"] = f"sample {k} deadline exceeded"
                result["abort_kind"] = "deadline"
                break
            k += 1
    except Exception as error:  # noqa: BLE001 - retain the actual workload/observer failure
        result["aborted"] = f"{phase}: {type(error).__name__}: {error}"
        result["abort_kind"] = "resource" if isinstance(error, ResourceStop) else "counter_error" if phase in ("baseline", "sample") else "operation_error"
        emit({"event": "error", "phase": phase, "type": type(error).__name__, "message": str(error)})
    result["iterations"] = iterations
    return result


class ResourceStop(RuntimeError):
    """A declared resource floor invalidated this measurement window."""


class Worker:
    """Own one bounded JSON-lines SDK child and reap its entire process group.

    Commands are argument arrays without a shell. Native stdout must contain only
    replies; stderr streams directly to a file. Stop/timeout never implies exit0.
    """

    def __init__(self, command, cwd, env, stderr, stop, timeout_s=120, check=lambda: None, stdout_sink=None):
        self.process = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=stderr, start_new_session=True)
        self.stop = stop
        self.stdout_sink = stdout_sink
        self.group_cleanup_needed = False
        self.timeout_s = timeout_s
        self.check = check
        self.buffer = b""
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    def request(self, operation):
        """Read one reply within the declared deadline, noticing sibling failure."""
        deadline = time.monotonic() + self.timeout_s
        self.process.stdin.write((json.dumps({"operation": operation}) + "\n").encode())
        self.process.stdin.flush()
        while b"\n" not in self.buffer:
            self.check()
            if self.stop.is_set():
                raise ResourceStop("sibling resource window stopped") if getattr(self.stop, "reason", None) in ("resource", "deadline") else RuntimeError("sibling worker/campaign stopped")
            if time.monotonic() >= deadline:
                raise TimeoutError(f"worker {operation} exceeded {self.timeout_s}s")
            if self.selector.select(min(0.1, max(0, deadline - time.monotonic()))):
                chunk = os.read(self.process.stdout.fileno(), 65536)
                if not chunk:
                    raise RuntimeError(f"worker exited before {operation} reply (exit={self.process.poll()})")
                if self.stdout_sink is not None:
                    self.stdout_sink.write(chunk)
                self.buffer += chunk
                if len(self.buffer) > 4 * MIB:
                    raise ValueError("worker reply exceeded 4MiB")
        line, self.buffer = self.buffer.split(b"\n", 1)
        reply = json.loads(line)
        if not isinstance(reply, dict) or reply.get("operation") != operation or reply.get("ok") is not True:
            raise RuntimeError(f"worker {operation} failed: {reply}")
        return reply["result"]

    def close(self):
        """Close the real SDK then require a healthy reaped exit within 5s."""
        previous = self.timeout_s
        self.timeout_s = 5
        started = time.monotonic()
        try:
            self.request("close")
            code = self.process.wait(timeout=max(0.001, 5 - (time.monotonic() - started)))
            if code != 0:
                raise RuntimeError(f"worker exit after close: {code}")
            return code
        finally:
            self.timeout_s = previous

    def reap(self):
        """Terminate an unfinished owned group and return its actual exit status."""
        # The group can outlive a cleanly exited leader; always stop owned
        # descendants as well, rather than treating leader exit as containment.
        leader_exited = self.process.poll() is not None
        try:
            os.killpg(self.process.pid, signal.SIGTERM)
            if leader_exited:
                self.group_cleanup_needed = True
        except ProcessLookupError:
            pass
        if self.process.poll() is None:
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=5)
        try:
            os.killpg(self.process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        self.selector.close()
        self.process.stdin.close()
        self.process.stdout.close()
        return self.process.returncode


def run_concurrent_arm(commands, pool_mode, output, limits, *, collect=collect_arm):
    """Run one Node and one Python worker concurrently using the existing sampler.

    Numeric resource bounds are required caller-selected execution inputs. Native
    results are streamed; only the existing baseline/60 snapshots stay in memory.
    This starts no follow-up arm after any failed/incomplete measurement.
    """
    output.mkdir()
    stop = threading.Event()
    started = time.monotonic()
    workers = {}
    files = []
    initial = read_linux_memory()
    limit_fields = ("mem_floor_bytes", "free_floor_bytes", "storage_limit_bytes", "arm_timeout_s", "request_timeout_s")
    if any(not _number(limits.get(k)) or limits[k] <= 0 for k in limit_fields):
        raise ValueError("all declared resource/deadline limits must be positive")
    if limits["request_timeout_s"] > 120 or limits["arm_timeout_s"] <= 3605:
        raise ValueError("request deadline must be <=120s and arm watchdog must include 3600s sampling")

    def check():
        current = read_linux_memory()
        if time.monotonic() - started > limits["arm_timeout_s"]:
            raise ResourceStop("declared arm watchdog exceeded")
        if current["mem_available_bytes"] < limits["mem_floor_bytes"]:
            raise ResourceStop("MemAvailable floor reached")
        if shutil.disk_usage(output).free < limits["free_floor_bytes"]:
            raise ResourceStop("owned storage free-space floor reached")
        if any(current[k] != initial[k] for k in ("pswpin", "pswpout")):
            raise ResourceStop("host swap activity invalidated window")
        # Streams plus bounded result files belong only to this output directory.
        if sum(p.stat().st_size for p in output.rglob("*") if p.is_file()) > limits["storage_limit_bytes"]:
            raise ResourceStop("owned output storage ceiling reached")

    results = {}
    try:
        check()
        for sdk in ("python", "node"):
            config = commands[sdk]
            command = config["command"]
            if not command or not all(isinstance(arg, str) for arg in command) or not Path(command[0]).is_absolute():
                raise ValueError("worker command must be an argv array with an absolute executable")
            err = (output / f"{sdk}.stderr").open("wb")
            events = (output / f"{sdk}.jsonl").open("w", buffering=1)
            stdout = (output / f"{sdk}.stdout").open("wb", buffering=0)
            files.extend((err, events, stdout))
            env = {**os.environ, **config.get("environment", {}), "FATHOMDB_POOL_MODE": pool_mode,
                   "FATHOMDB_POOL_RELEASE_THRESHOLD": "0", "HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1"}
            dbs = output / f"{sdk}-dbs"
            dbs.mkdir()
            env["FATHOMDB_DB_SCRATCH"] = str(dbs.resolve())
            worker = Worker(command, config["cwd"], env, err, stop, limits["request_timeout_s"], check, stdout)
            workers[sdk] = (worker, events)

        def one(sdk):
            worker, events = workers[sdk]
            identity = None

            def emit(event):
                events.write(json.dumps(event, allow_nan=False) + "\n")

            def snapshot():
                nonlocal identity
                reply = worker.request("snapshot")
                raw = reply["raw_observation"]
                emit({"event": "native_observation", "raw_observation": raw})
                checked = adapt_observation(raw, pool_mode, pid=worker.process.pid, expected=identity)
                identity = checked["identity"]
                memory = parse_linux_memory(Path(f"/proc/{worker.process.pid}/status").read_text(),
                                            Path("/proc/meminfo").read_text(), Path("/proc/vmstat").read_text())
                check()
                if memory["vm_swap_bytes"]:
                    raise ResourceStop("worker VmSwap invalidated window")
                return dict(memory, **checked, node_heap=reply.get("node_heap"))

            result = {"sdk": sdk, "pool_mode": pool_mode, "baseline": None, "samples": [],
                      "exit_code": None, "aborted": None, "abort_kind": None}
            try:
                emit({"event": "ready", "result": worker.request("ready"), "pid": worker.process.pid})
                result = collect(sdk, pool_mode, lambda: worker.request("iteration"), snapshot, emit)
                if result.get("aborted") is None:
                    result["exit_code"] = worker.close()
                else:
                    stop.reason = result.get("abort_kind")
                    stop.set()
            except Exception as error:
                result.update(aborted=f"{type(error).__name__}: {error}",
                              abort_kind="resource" if isinstance(error, ResourceStop) else "operation_error")
                emit({"event": "error", "message": result["aborted"]})
                stop.reason = result.get("abort_kind")
                stop.set()
            finally:
                result["exit_code"] = worker.reap()
                if worker.group_cleanup_needed and result["aborted"] is None:
                    result.update(aborted="worker left an owned process group alive", abort_kind="operation_error")
                    stop.reason = "operation_error"
                    stop.set()
                emit({"event": "exit", "exit_code": result["exit_code"], "aborted": result["aborted"]})
                (output / f"{sdk}.json").write_text(json.dumps(result, allow_nan=False))
            return sdk, result

        with ThreadPoolExecutor(max_workers=2) as executor:
            for sdk, result in executor.map(one, ("python", "node")):
                results[sdk] = result
        return results
    finally:
        stop.set()
        for worker, _ in workers.values():
            worker.reap()
        for file in files:
            file.close()


def collect_pair(spec: dict, output: Path) -> dict:
    """Collect the frozen two-arm campaign, refusing an off arm after failed on."""
    if output.exists():
        raise ValueError("campaign output must be a fresh owned directory")
    output.mkdir(parents=True)
    on = run_concurrent_arm(spec["workers"], "on", output / "on", spec["limits"])
    verdicts = {sdk: evaluate_arm(arm) for sdk, arm in on.items()}
    if any(result["status"] != "PASS" for result in verdicts.values()):
        return {"scope": "instrumented_measurement_only", "status": "FAIL" if any(v["status"] == "FAIL" for v in verdicts.values()) else "UNMEASURED",
                "on": verdicts, "off": "NOT_RUN"}
    off = run_concurrent_arm(spec["workers"], "off", output / "off", spec["limits"])
    results = {}
    for sdk in ("python", "node"):
        pair = {"on": on[sdk], "off": off[sdk]}
        (output / f"{sdk}-pair.json").write_text(json.dumps(pair, allow_nan=False))
        results[sdk] = evaluate_pair(**pair)
    return {"scope": "instrumented_measurement_only", "pairs": results}


def run_lifecycle(command, cwd, environment, case, output):
    """Exercise one actual SDK lifecycle child with passive snapshots and no GC.

    Each call is one fresh process/database case. Rust has the matching ignored
    source-SDK integration fixture; this driver handles installed Python/Node.
    """
    if case not in ("max", "zero", "drop", "exit-alive", "cls", "reranker"):
        raise ValueError("unknown lifecycle case")
    output.mkdir()
    env = {**os.environ, **environment, "FATHOMDB_POOL_MODE": "on",
           "FATHOMDB_POOL_RELEASE_THRESHOLD": "0" if case == "zero" else "max",
           "HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1", "FATHOMDB_DB_SCRATCH": str(output.resolve())}
    stop = threading.Event()
    started = time.monotonic()

    def deadline():
        if time.monotonic() - started > 120:
            raise TimeoutError("lifecycle child exceeded 120s")

    with (output / "stderr").open("wb") as stderr, (output / "stdout").open("wb", buffering=0) as stdout, (output / "events.jsonl").open("w", buffering=1) as events:
        worker = Worker(command, cwd, env, stderr, stop, 120, deadline, stdout)

        def emit(event):
            events.write(json.dumps(event, allow_nan=False) + "\n")

        def request(operation):
            result = worker.request(operation)
            emit({"event": operation, "result": result, "monotonic_s": time.monotonic()})
            return result

        def observe():
            return request("snapshot")["raw_observation"]

        try:
            request("ready")
            initial = observe()
            if initial.get("status") != "not_decided" or initial.get("pid") != worker.process.pid:
                raise ValueError("lifecycle child did not start with a fresh undecided pool")
            request("open-two" if case in ("max", "zero", "drop") else "open-one")
            before = adapt_observation(observe(), "on", pid=worker.process.pid)
            expected = before["identity"]
            released_at = None
            if case in ("cls", "reranker"):
                request("singleton-" + case)
                released_at = request("close-last")["closed_monotonic_s"]
                retained = adapt_observation(observe(), "on", expected=expected)
                if retained["live_models"] < 1 or min(retained["private_used_bytes"], retained["private_reserved_bytes"]) <= 0:
                    raise ValueError("intentional singleton did not retain live model memory")
            elif case == "exit-alive":
                if before["live_models"] != 1 or before["private_used_bytes"] <= 0:
                    raise ValueError("unclosed child has no actual live model")
            else:
                if case == "drop":
                    released_at = request("drop-all")["dropped_monotonic_s"]
                else:
                    request("close-first")
                    after = adapt_observation(observe(), "on", pid=worker.process.pid, expected=expected)
                    if case == "max":
                        require_first_close(before, after)
                    elif after["live_models"] != 1 or after["private_used_bytes"] <= 0:
                        raise ValueError("threshold0 first close did not preserve the surviving model")
                    survivor = request("survivor")
                    if survivor.get("unchanged") is not True:
                        raise ValueError("survivor same-input output changed")
                    released_at = request("close-last")["closed_monotonic_s"]
                worker.timeout_s = 5
                wait_for_zero(observe, emit, expected=expected, started_at=released_at)
            worker.timeout_s = 5
            exit_started = time.monotonic() if released_at is None else released_at
            worker.timeout_s = max(0.001, 5 - (time.monotonic() - exit_started))
            request("exit")
            code = worker.process.wait(timeout=max(0.001, 5 - (time.monotonic() - exit_started)))
            if code != 0:
                raise RuntimeError(f"lifecycle child exited {code}")
            return {"case": case, "exit_code": code, "status": "PASS"}
        finally:
            emit({"event": "exit_status", "exit_code": worker.reap()})
            if worker.group_cleanup_needed:
                raise RuntimeError("lifecycle child left an owned process group alive")


def main() -> int:
    """Evaluate an existing JSON SDK pair; never start a campaign or import native SDKs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pair", nargs="?", type=Path, help="JSON object with on and off arm records")
    parser.add_argument("--lifecycle", type=Path, help="one installed SDK worker argv/cwd/environment JSON")
    parser.add_argument("--case", choices=("max", "zero", "drop", "exit-alive", "cls", "reranker"))
    parser.add_argument("--collect", type=Path, help="JSON worker argv/cwd and declared resource limits")
    parser.add_argument("--output", type=Path, help="fresh owned campaign output directory")
    args = parser.parse_args()
    if args.lifecycle:
        if args.collect or args.pair or not args.case or args.output is None:
            parser.error("--lifecycle requires --case and --output")
        config = json.loads(args.lifecycle.read_text())
        print(json.dumps(run_lifecycle(config["command"], config["cwd"], config["environment"], args.case, args.output)))
        return 0
    if args.case:
        parser.error("--case requires --lifecycle")
    if args.collect:
        if args.pair or args.output is None:
            parser.error("--collect requires --output and no evaluation pair")
        result = collect_pair(json.loads(args.collect.read_text()), args.output)
        print(json.dumps(result, allow_nan=False))
        return 0 if all(v["status"] == "PASS" for v in result.get("pairs", {}).values()) and "pairs" in result else 1
    if args.pair is None or args.output:
        parser.error("supply a pair file or --collect SPEC --output DIR")
    pair = json.loads(args.pair.read_text())
    result = evaluate_pair(pair["on"], pair["off"])
    print(json.dumps(result, indent=2, allow_nan=False))
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
