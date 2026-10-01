#!/usr/bin/env python3
"""Build and run the D27 mixed workload against an unmodified engine checkout."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import socket
import subprocess
import sys
import threading


ROOT = Path(__file__).resolve().parents[1]
PROTOCOL = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
WORKLOAD = ROOT / "scripts/d27_runtime_workload.rs"
VERIFIER = ROOT / "scripts/d27-runtime-qualification.py"
HOST_PID_NAMESPACE = "pid:[4026531836]"
HOST_PID_ONE_COMM = "systemd"


def sha256(path: Path) -> str:
    """Hash an artifact as bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def generate_corpus(protocol: dict, path: Path) -> None:
    """Write the frozen 10,000-record, 512-byte, 1,024-token corpus."""
    fixture = protocol["fixture"]
    if fixture["generator"] != "d27_runtime_qualification_v1" or fixture["body_bytes"] != 512 or fixture["vocabulary_size"] != 1024:
        raise ValueError("unsupported frozen corpus generator")
    seed = int(fixture["seed"], 16)
    with path.open("wb") as stream:
        for index in range(fixture["initial_canonical_nodes"]):
            body = "".join(f"{(seed + index + offset) % 1024:03x} " for offset in range(128))
            row = {"body": body, "logical_id": f"d27-seed-{index:05d}"}
            stream.write(json.dumps(row, separators=(",", ":"), sort_keys=True).encode() + b"\n")


def repetition_order(protocol: dict, phase: str) -> list[str]:
    """Return the immutable interleaved order for an entry or candidate run."""
    if phase not in ("entry", "candidate"):
        raise ValueError("phase must be entry or candidate")
    entry = protocol["execution"]["repetition_order"]
    return [item if phase == "entry" else item.replace("entry_", "candidate_", 1) for item in entry]


def validate_raw_contract(raw: dict, protocol: dict, direction: str, repetition: int) -> None:
    """Reject a workload that changed duration, order, epoch size, or ratios."""
    execution = protocol["execution"]
    if raw.get("direction") != direction or raw.get("repetition") != repetition:
        raise ValueError("raw direction or repetition mismatch")
    if raw.get("warmup_seconds") != execution["warmup_seconds"]:
        raise ValueError("warmup mismatch")
    if raw.get("measurement_seconds") != execution["measurement_seconds"]:
        raise ValueError("measurement duration mismatch")
    if raw.get("epoch_size") != execution["operation_epoch_size"]:
        raise ValueError("epoch size mismatch")
    counts = raw.get("operation_counts", {})
    target = execution[f"{direction}_epoch"]
    epochs = counts.get("canonical_writes", 0) // target["canonical_writes"]
    if epochs < 1 or any(counts.get(key) != epochs * value for key, value in target.items()):
        raise ValueError("operation ratios mismatch")
    elapsed = raw.get("measurement_elapsed_ns", execution["measurement_seconds"] * 1_000_000_000)
    if elapsed < execution["measurement_seconds"] * 1_000_000_000:
        raise ValueError("measurement stopped early")


def command(args: list[str], **kwargs) -> subprocess.CompletedProcess:
    """Run a required command and preserve its complete diagnostics on failure."""
    return subprocess.run(args, check=True, text=True, **kwargs)


def git_sha(source: Path) -> str:
    sha = command(["git", "-C", str(source), "rev-parse", "HEAD"], capture_output=True).stdout.strip()
    dirty = command(["git", "-C", str(source), "status", "--porcelain", "--untracked-files=no"], capture_output=True).stdout
    if dirty:
        raise ValueError("source checkout has tracked modifications")
    return sha


def build_manifest(source: Path, directory: Path) -> Path:
    """Select exact-source path dependencies in an external Cargo test crate."""
    rust = source / "src/rust/crates"
    manifest = directory / "Cargo.toml"
    manifest.write_text(
        "[package]\nname = \"d27-runtime-runner\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        "[workspace]\n"
        "[features]\ntest-hooks = [\"fathomdb-engine/test-hooks\"]\n"
        "[dependencies]\n"
        f"fathomdb-engine = {{ path = {json.dumps(str(rust / 'fathomdb-engine'))} }}\n"
        f"fathomdb-embedder-api = {{ path = {json.dumps(str(rust / 'fathomdb-embedder-api'))} }}\n"
        f"fathomdb-schema = {{ path = {json.dumps(str(rust / 'fathomdb-schema'))} }}\n"
        "serde_json = \"1\"\nrusqlite = { version = \"0.40\", features = [\"bundled\"] }\n"
        "tempfile = \"3\"\n"
        "[[test]]\nname = \"d27_runtime_workload\"\n"
        f"path = {json.dumps(str(WORKLOAD))}\n"
    )
    return manifest


def build_binary(manifest: Path, output: Path) -> Path:
    env = dict(os.environ, CARGO_TARGET_DIR=str(output / "target"))
    try:
        result = command(["cargo", "test", "--release", "--features", "test-hooks", "--no-run", "--message-format=json", "--manifest-path", str(manifest)], capture_output=True, env=env)
    except subprocess.CalledProcessError as error:
        if error.stderr:
            sys.stderr.write(error.stderr)
        raise
    paths = [Path(message["executable"]) for line in result.stdout.splitlines() if (message := json.loads(line)).get("target", {}).get("name") == "d27_runtime_workload" and message.get("executable")]
    if len(paths) != 1:
        raise ValueError(f"expected one D27 test binary, found {len(paths)}")
    return paths[0]


def memory_gib() -> float:
    meminfo = Path("/proc/meminfo").read_text()
    match = re.search(r"^MemTotal:\s+(\d+) kB$", meminfo, re.M)
    if not match:
        raise ValueError("cannot read total memory")
    return int(match.group(1)) * 1024 / (1024**3)


def procfs_hidepid() -> str:
    """Report the effective /proc mount's process-visibility restriction."""
    effective = None
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        mount, separator, filesystem = line.partition(" - ")
        mount_fields = mount.split()
        fs_fields = filesystem.split()
        if separator and len(mount_fields) > 4 and mount_fields[4] == "/proc" and len(fs_fields) > 2 and fs_fields[0] == "proc":
            effective = next((option.split("=", 1)[1] for option in fs_fields[2].split(",") if option.startswith("hidepid=")), "0")
    return effective if effective is not None else "unavailable"


def environment() -> dict:
    """Capture environmental invalidators adjacent to each repetition."""
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").read_text().strip()
    vmstat = Path("/proc/vmstat").read_text()
    swap = {name: int(re.search(rf"^{name} (\d+)$", vmstat, re.M).group(1)) for name in ("pswpin", "pswpout")}
    processes = command(["ps", "-eo", "pid=,comm="], capture_output=True).stdout.splitlines()
    device = command(["df", "-P", "/tmp"], capture_output=True).stdout.splitlines()[-1].split()[0]
    competing = []
    ps_pid_one_comm = None
    for line in processes:
        pid, name = line.strip().split(maxsplit=1)
        if int(pid) == 1:
            ps_pid_one_comm = name
        if int(pid) != os.getpid() and re.search(r"^(cargo|rustc|pytest|maturin|ollama|vllm|postgres|sqlite3)$", name):
            competing.append({"pid": int(pid), "name": name})
    try:
        pid_one_namespace = os.readlink("/proc/1/ns/pid")
        pid_one_comm = Path("/proc/1/comm").read_text().strip()
    except OSError:
        pid_one_namespace = None
        pid_one_comm = None
    return {
        "cpu_governor": governor,
        "competing_processes": competing,
        "pid_namespace": os.readlink("/proc/self/ns/pid"),
        "pid_one_namespace": pid_one_namespace,
        "pid_one_comm": pid_one_comm,
        "ps_pid_one_comm": ps_pid_one_comm,
        "procfs_hidepid": procfs_hidepid(),
        "swap_pages_in": swap["pswpin"],
        "swap_pages_out": swap["pswpout"],
        "kernel": platform.release(),
        "cpu_model": next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")), "unknown"),
        "online_cpus": os.cpu_count(),
        "memory_available_kib": int(re.search(r"^MemAvailable:\s+(\d+) kB$", Path("/proc/meminfo").read_text(), re.M).group(1)),
        "load_average": os.getloadavg(),
        "database_device": device,
    }


def runner_inventory() -> dict:
    device = command(["df", "-P", "/tmp"], capture_output=True).stdout.splitlines()[-1].split()[0]
    return {
        "host": socket.gethostname(),
        "operating_system": f"{platform.system()} {platform.machine()}",
        "online_cpus": os.cpu_count(),
        "memory_gib": memory_gib(),
        "database_storage": "local NVMe" if device.startswith("/dev/nvme") else "unknown",
        "database_device": device,
        "build": "cargo test --release",
        "features": ["test-hooks"],
        "rustc": command(["rustc", "--version"], capture_output=True).stdout.strip(),
    }


def process_view_invalidators(observation: dict, label: str) -> list[str]:
    """Require the host namespace and an unrestricted host /proc view."""
    namespace = observation.get("pid_namespace")
    reasons = []
    if namespace != HOST_PID_NAMESPACE:
        reasons.append(f"{label} pid namespace={namespace!r}; expected host {HOST_PID_NAMESPACE}")
    pid_one_namespace = observation.get("pid_one_namespace")
    observed = (observation.get("pid_one_comm"), observation.get("ps_pid_one_comm"), observation.get("procfs_hidepid"))
    expected = (HOST_PID_ONE_COMM, HOST_PID_ONE_COMM, "0")
    if pid_one_namespace not in ("", HOST_PID_NAMESPACE) or observed != expected:
        reasons.append(f"{label} process view={(pid_one_namespace, *observed)!r}; expected unrestricted host PID1 identity and {expected!r}")
    return reasons


def environment_invalidators(raw: dict) -> list[str]:
    """Name each observed reason that makes a repetition non-comparable."""
    before = raw.get("environment_start", {})
    after = raw.get("environment_end", {})
    samples = raw.get("environment_samples")
    reasons = []
    if not isinstance(samples, list) or not samples:
        reasons.append("environment samples missing")
        samples = []
    for label, observation in [("before", before), *[(f"sample[{index}]", sample) for index, sample in enumerate(samples)], ("after", after)]:
        reasons.extend(process_view_invalidators(observation, label))
        governor = observation.get("cpu_governor")
        if governor != "performance":
            reasons.append(f"{label} cpu governor={governor!r}; expected performance")
        competitors = observation.get("competing_processes")
        if competitors != []:
            if isinstance(competitors, list) and competitors:
                for process in competitors:
                    reasons.append(f"{label} competing process {process.get('name')} pid={process.get('pid')}")
            else:
                reasons.append(f"{label} competing processes unavailable")
        for key in ("swap_pages_in", "swap_pages_out"):
            if observation.get(key) != before.get(key):
                reasons.append(f"{label} {key} changed from {before.get(key)!r} to {observation.get(key)!r}")
        device = observation.get("database_device")
        if device != before.get("database_device") or not str(device).startswith("/dev/nvme"):
            reasons.append(f"{label} database_device={device!r}; expected local NVMe {before.get('database_device')!r}")
    return reasons


def run_repetition(binary: Path, env: dict[str, str]) -> list[dict]:
    """Sample invalidators throughout the measured child process."""
    stop = threading.Event()
    samples = []
    errors = []

    def observe() -> None:
        while not stop.is_set():
            try:
                samples.append(environment())
            except Exception as error:
                errors.append(error)
                stop.set()
            stop.wait(0.1)

    observer = threading.Thread(target=observe, name="d27-environment-observer")
    observer.start()
    try:
        command([str(binary), "--ignored", "--exact", "d27_runtime_qualification", "--nocapture"], env=env)
    finally:
        stop.set()
        observer.join()
    if errors:
        raise RuntimeError("environment monitor failed") from errors[0]
    if not samples:
        raise ValueError("environment monitor recorded no samples")
    return samples


def percentile(values: list[float], rank: int) -> float:
    if not values:
        raise ValueError("missing applicable raw metric")
    values = sorted(values)
    index = (len(values) - 1) * rank / 100
    low = int(index)
    high = min(low + 1, len(values) - 1)
    return values[low] + (values[high] - values[low]) * (index - low)


def starvation_from_raw(raw: dict, window_seconds: int = 5) -> bool:
    """Check full five-second windows with unfinished admitted work."""
    windows = raw["measurement_elapsed_ns"] // (window_seconds * 1_000_000_000)
    classes = {
        name: [(item["admitted_ns"], item["completed_ns"]) for item in raw["operations"] if item["class"] == name]
        for name in ("canonical_write", "foreground_hybrid_query", "direct_embed")
    }
    classes["projection"] = [(item["committed_ns"], item["observed_ns"]) for item in raw["projection_completions"]]
    for intervals in classes.values():
        for index in range(windows):
            start = index * window_seconds * 1_000_000_000
            end = start + window_seconds * 1_000_000_000
            pending_in_window = any(admitted < end and completed > start for admitted, completed in intervals)
            progress = any(start < completed <= end for _, completed in intervals)
            if pending_in_window and not progress:
                return False
    return True


def interval_peak(intervals: list[tuple[int, int]]) -> int:
    events = []
    for start, end in intervals:
        if end < start:
            raise ValueError("dispatch trace has reversed interval")
        if end > start:
            events.extend(((start, 1), (end, -1)))
    active = 0
    peak = 0
    for _, delta in sorted(events, key=lambda event: (event[0], event[1])):
        active += delta
        peak = max(peak, active)
    return peak


def dispatch_from_raw(raw: dict, embed_workers: int) -> dict:
    """Derive candidate queue and provider facts from engine request events."""
    events = raw.get("embed_dispatch_events")
    if not isinstance(events, list) or not events:
        raise ValueError("candidate dispatch trace missing")
    operations = {op["sequence"]: op for op in raw["operations"]}
    write_cursors = {op["cursor"] for op in raw["operations"] if op["class"] == "canonical_write" and op["outcome"] == "completed"}
    covered_foreground = set()
    covered_projection = set()
    ids = set()
    waits = []
    queued_intervals = []
    running_intervals = []
    for event in events:
        request_id = event.get("request_id")
        if event.get("source") != "engine" or not isinstance(request_id, int) or request_id in ids:
            raise ValueError("dispatch trace request identity/source invalid")
        ids.add(request_id)
        admitted = event.get("admitted_ns")
        started = event.get("started_ns")
        terminal = event.get("terminal_ns")
        if not isinstance(admitted, int) or not isinstance(terminal, int) or admitted < 0 or terminal < admitted or (started is not None and (not isinstance(started, int) or not admitted <= started <= terminal)):
            raise ValueError("dispatch trace timestamp invalid")
        owner = event.get("owner", {})
        if "operation_sequence" in owner and "projection_cursors" not in owner:
            sequence = owner["operation_sequence"]
            if sequence not in operations or operations[sequence]["class"] not in ("foreground_hybrid_query", "direct_embed"):
                raise ValueError("dispatch trace foreground owner invalid")
            covered_foreground.add(sequence)
        elif "projection_cursors" in owner and "operation_sequence" not in owner:
            cursors = owner["projection_cursors"]
            if not isinstance(cursors, list) or not cursors or not set(cursors).issubset(write_cursors):
                raise ValueError("dispatch trace projection owner invalid")
            covered_projection.update(cursors)
        else:
            raise ValueError("dispatch trace owner missing")
        queue_exit = started if started is not None else terminal
        waits.append(queue_exit - admitted)
        queued_intervals.append((admitted, queue_exit))
        if started is not None:
            running_intervals.append((started, terminal))
    required_foreground = {sequence for sequence, op in operations.items() if op["class"] in ("foreground_hybrid_query", "direct_embed")}
    if covered_foreground != required_foreground or covered_projection != write_cursors:
        raise ValueError("dispatch trace does not cover every measured request")
    waiting_peak = interval_peak(queued_intervals)
    provider_peak = interval_peak(running_intervals)
    if waiting_peak > 4 * embed_workers or provider_peak > embed_workers:
        raise ValueError("dispatch trace capacity bound exceeded")
    if raw.get("provider_peak_concurrency") != provider_peak:
        raise ValueError("dispatch trace provider concurrency mismatch")
    return {"queue_wait_ns": waits, "waiting_peak": waiting_peak, "provider_peak": provider_peak}


def check_raw_observations(raw: dict, phase: str) -> None:
    """Prove the raw stream has the claimed work and projection completions."""
    if raw.get("smoke"):
        raise ValueError("smoke run cannot qualify")
    reasons = environment_invalidators(raw)
    if reasons:
        raise ValueError("invalid repetition environment: " + "; ".join(reasons))
    if raw.get("environment_valid") is not True:
        raise ValueError("environment_valid flag disagrees with observations")
    actual = {
        "canonical_writes": sum(op["class"] == "canonical_write" for op in raw["operations"]),
        "foreground_hybrid_queries": sum(op["class"] == "foreground_hybrid_query" for op in raw["operations"]),
        "direct_embeds": sum(op["class"] == "direct_embed" for op in raw["operations"]),
    }
    if actual != raw["operation_counts"]:
        raise ValueError("operation counts diverge from raw observations")
    ordered = sorted(raw["operations"], key=lambda operation: operation.get("sequence", -1))
    if not ordered or len(ordered) % 10 or any(operation.get("sequence") != ordered[0]["sequence"] + index for index, operation in enumerate(ordered)) or ordered[0]["sequence"] % 10:
        raise ValueError("epoch sequence gap or duplicate")
    for epoch_start in range(0, len(ordered), 10):
        epoch = ordered[epoch_start:epoch_start + 10]
        writes = 4 if raw["direction"] == "projection_heavy" else 1
        queries = 4 if raw["direction"] == "projection_heavy" else 7
        expected_classes = ["canonical_write"] * writes + ["foreground_hybrid_query"] * queries + ["direct_embed"] * 2
        if [operation["class"] for operation in epoch] != expected_classes:
            raise ValueError("epoch operation ratio or class order mismatch")
    for operation in raw["operations"]:
        if not 0 <= operation["admitted_ns"] <= operation["completed_ns"]:
            raise ValueError("nonmonotonic operation timestamps")
        if operation["admitted_ns"] > raw["measurement_elapsed_ns"]:
            raise ValueError("operation admitted after stop")
    writes = {op["cursor"]: op["completed_ns"] for op in raw["operations"] if op["class"] == "canonical_write" and op["outcome"] == "completed"}
    completions = raw["projection_completions"]
    if len(completions) != len(writes) or {item["cursor"] for item in completions} != set(writes):
        raise ValueError("projection completions do not match committed writes")
    if any(item["committed_ns"] != writes[item["cursor"]] or item["observed_ns"] < item["committed_ns"] for item in completions):
        raise ValueError("projection completion timestamps mismatch")
    if raw["close_result"] != "Ok(())":
        raise ValueError("close did not succeed")
    if raw["residual_workers_after_close"] != 0:
        raise ValueError("residual workers after close")
    if phase == "entry" and raw.get("connection_inventory") != "Err(Storage)":
        raise ValueError("historical SQLite inventory must retain observed Err(Storage)")
    if phase == "candidate":
        observed = raw.get("configuration_observation")
        if not isinstance(observed, dict) or observed.get("source") != "engine":
            raise ValueError("candidate configuration observation missing")
        scheduler = observed.get("scheduler_runtime_threads")
        embed_workers = observed.get("embedder_pool_size")
        if not isinstance(scheduler, int) or not 1 <= scheduler <= 64 or not isinstance(embed_workers, int) or not 1 <= embed_workers <= 64:
            raise ValueError("candidate configuration observation invalid")
        dispatch = dispatch_from_raw(raw, embed_workers)
        projection = raw.get("projection_admission_observation")
        if not isinstance(projection, dict) or projection.get("source") != "engine" or not isinstance(projection.get("active_plus_queued_high_water"), int):
            raise ValueError("projection admission observation missing")
        if projection["active_plus_queued_high_water"] > scheduler * 64:
            raise ValueError("projection admission bound exceeded")
        if raw["engine_thread_inventory"] != 1 + scheduler + 8 + embed_workers:
            raise ValueError("engine thread inventory mismatch")
        if raw.get("embed_requests_waiting_high_water") is not None and raw["embed_requests_waiting_high_water"] != dispatch["waiting_peak"]:
            raise ValueError("embed queue observation differs from dispatch trace")


def summarize_raw(raw: dict, phase: str) -> dict:
    """Derive unrounded rates and latency percentiles from monotonic ns."""
    check_raw_observations(raw, phase)
    operations = raw["operations"]
    duration = raw["measurement_elapsed_ns"] / 1e9
    classes = {name: [item for item in operations if item["class"] == name] for name in ("canonical_write", "foreground_hybrid_query", "direct_embed")}
    successful = {name: [item for item in items if item["outcome"] == "completed"] for name, items in classes.items()}
    counts = {name: 0 for name in ("admitted", "completed", "timed_out", "overloaded", "provider_error", "invalid_vector", "panic")}
    counts["admitted"] = len(operations)
    counts["completed"] = sum(len(items) for items in successful.values())
    for item in operations:
        if item["outcome"] == "completed":
            continue
        label = str(item["outcome"]).lower()
        outcomes = (
            ("timed_out", ("timeout", "deadline")),
            ("overloaded", ("overloaded", "saturated")),
            ("invalid_vector", ("dimension", "invalidvector")),
            ("panic", ("panic",)),
        )
        bucket = next((name for name, needles in outcomes if any(needle in label for needle in needles)), "provider_error")
        counts[bucket] += 1
    throughput = {
        "canonical_commits": len(successful["canonical_write"]) / duration,
        "projection_completions": sum(item["observed_ns"] <= raw["measurement_elapsed_ns"] for item in raw["projection_completions"]) / duration,
        "foreground_hybrid_queries": len(successful["foreground_hybrid_query"]) / duration,
        "direct_embeds": len(successful["direct_embed"]) / duration,
    }
    latency_ns = {name: [(item["completed_ns"] - item["admitted_ns"]) / 1e6 for item in items] for name, items in successful.items()}
    latency_ns["projection_freshness"] = [(item["observed_ns"] - item["committed_ns"]) / 1e6 for item in raw["projection_completions"]]
    latency_ns["close"] = [(raw["close_end_ns"] - raw["close_start_ns"]) / 1e6]
    names = {"canonical_commit": "canonical_write", "projection_freshness": "projection_freshness", "foreground_hybrid_query": "foreground_hybrid_query", "direct_embed": "direct_embed", "close": "close"}
    latency = {name: {str(rank): percentile(latency_ns[key], rank) for rank in (50, 95, 99)} for name, key in names.items()}
    if phase == "candidate":
        dispatch = dispatch_from_raw(raw, raw["configuration_observation"]["embedder_pool_size"])
        latency["embed_queue_wait"] = {str(rank): percentile(dispatch["queue_wait_ns"], rank) / 1e6 for rank in (50, 95, 99)}
    high_water = {"durable_projection_backlog": raw["projection_backlog_high_water"]}
    inventory = {"provider_concurrency": raw["provider_peak_concurrency"], "engine_threads": raw["engine_thread_inventory"], "residual_workers_after_close": raw["residual_workers_after_close"]}
    if phase == "candidate":
        conn_match = re.search(r"creation=writer:(\d+),readers:(\d+),dispatcher:(\d+),workers:(\d+),probes:(\d+)", raw["connection_inventory"])
        if not conn_match:
            raise ValueError("missing SQLite inventory")
        connections = sum(int(value) for value in conn_match.groups())
        if connections != 1 + 1 + raw["configuration_observation"]["scheduler_runtime_threads"] + 8:
            raise ValueError("SQLite connection inventory mismatch")
        inventory["sqlite_connections"] = connections
        high_water["projection_rows_active_plus_queued"] = raw["projection_admission_observation"]["active_plus_queued_high_water"]
        high_water["embed_requests_waiting"] = dispatch["waiting_peak"]
    return {
        "throughput": throughput,
        "latency_ms": latency,
        "counts": counts,
        "high_water": high_water,
        "inventory": inventory,
        "starvation_pass": starvation_from_raw(raw),
        "environment_valid": raw["environment_valid"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True, help="clean historical or candidate checkout")
    parser.add_argument("--phase", choices=("entry", "candidate"), required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--entry", type=Path, help="validated entry receipt for candidate comparison")
    for name in ("runner", "binary", "corpus", "raw"):
        parser.add_argument(f"--entry-{name}", type=Path)
    args = parser.parse_args()
    protocol = json.loads(PROTOCOL.read_text())
    source = args.source.resolve()
    source_sha = git_sha(source)
    if args.phase == "entry" and source_sha != protocol["entry_engine_candidate_sha"]:
        raise ValueError("wrong entry candidate")
    if (protocol["runner"]["host_pid_namespace"], protocol["runner"]["host_pid_one_comm"]) != (HOST_PID_NAMESPACE, HOST_PID_ONE_COMM):
        raise ValueError("D27 host process view protocol mismatch")
    initial_environment = environment()
    if reasons := process_view_invalidators(initial_environment, "runner"):
        raise ValueError("; ".join(reasons) + "; run with unrestricted host /proc")
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    corpus = output / "corpus.jsonl"
    generate_corpus(protocol, corpus)
    runner_bundle = output / "runner.bundle"
    runner_bundle.write_bytes(Path(__file__).read_bytes() + b"\n--RUST--\n" + WORKLOAD.read_bytes() + b"\n--VERIFIER--\n" + VERIFIER.read_bytes())
    build_dir = output / "build"
    build_dir.mkdir(exist_ok=True)
    binary = build_binary(build_manifest(source, build_dir), output)
    raw_output = output / "raw-output.jsonl"
    metrics = {"projection_heavy": [], "foreground_heavy": []}
    all_raw = []
    start_environment = initial_environment
    for name in repetition_order(protocol, args.phase):
        direction, repetition = re.fullmatch(r"(?:entry|candidate)_(projection_heavy|foreground_heavy)_(\d)", name).groups()
        repetition = int(repetition)
        before = environment()
        path = output / f"{name}.raw.json"
        env = dict(os.environ, D27_DIRECTION=direction, D27_REPETITION=str(repetition), D27_RAW_OUTPUT=str(path), D27_CORPUS=str(corpus))
        samples = run_repetition(binary, env)
        after = environment()
        raw = json.loads(path.read_text())
        raw["environment_start"] = before
        raw["environment_end"] = after
        raw["environment_samples"] = samples
        reasons = environment_invalidators(raw)
        raw["environment_valid"] = not reasons
        attempt = output / f"{name}.attempt.json"
        attempt.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        all_raw.append(raw)
        raw_output.write_text("".join(json.dumps(item, sort_keys=True, separators=(",", ":")) + "\n" for item in all_raw))
        try:
            validate_raw_contract(raw, protocol, direction, repetition)
            metrics[direction].append(summarize_raw(raw, args.phase))
        except Exception as error:
            invalidation = {
                "status": "INVALID_ENVIRONMENT" if reasons else "INVALID_REPETITION",
                "source_sha": source_sha,
                "protocol_sha256": sha256(PROTOCOL),
                "binary_sha256": sha256(binary),
                "attempt_sha256": sha256(attempt),
                "raw_output_sha256": sha256(raw_output),
                "reasons": reasons or [str(error)],
                "error": str(error),
            }
            (output / f"{name}.invalidation.json").write_text(json.dumps(invalidation, indent=2, sort_keys=True) + "\n")
            raise
    end_environment = environment()
    receipt = {
        "phase": args.phase, "source_sha": source_sha, "binary_sha256": sha256(binary),
        "runner_sha256": sha256(runner_bundle), "protocol_sha256": sha256(PROTOCOL),
        "corpus_sha256": sha256(corpus), "raw_output_sha256": sha256(raw_output),
        "runner_inventory": runner_inventory(), "environment_start": start_environment,
        "environment_end": end_environment, "per_repetition_metrics": metrics,
        "historical_unavailable": protocol["metrics"]["historical_unavailable"] if args.phase == "entry" else [],
        "aggregate_metrics": {}, "decision_rule_evaluation": {}, "status": "PASS",
    }
    import importlib.util

    spec = importlib.util.spec_from_file_location("d27_verifier", VERIFIER)
    verifier = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(verifier)
    entry = json.loads(args.entry.read_text()) if args.entry else None
    verifier.verify_raw_linkage(receipt, protocol, raw_output)
    if args.phase == "candidate":
        entry_artifacts = {name: getattr(args, f"entry_{name}") for name in ("runner", "binary", "corpus", "raw")}
        entry = verifier.validate_entry_for_candidate(entry, protocol, PROTOCOL, entry_artifacts)
    receipt = verifier.validate_receipt(receipt, protocol, PROTOCOL, {"runner": runner_bundle, "binary": binary, "corpus": corpus, "raw": raw_output}, entry)
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(output / "receipt.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
