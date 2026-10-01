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


ROOT = Path(__file__).resolve().parents[1]
PROTOCOL = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
WORKLOAD = ROOT / "scripts/d27_runtime_workload.rs"
VERIFIER = ROOT / "scripts/d27-runtime-qualification.py"


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
    result = command(["cargo", "test", "--release", "--features", "test-hooks", "--no-run", "--message-format=json", "--manifest-path", str(manifest)], capture_output=True, env=env)
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


def environment() -> dict:
    """Capture environmental invalidators adjacent to each repetition."""
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").read_text().strip()
    vmstat = Path("/proc/vmstat").read_text()
    swap = {name: int(re.search(rf"^{name} (\d+)$", vmstat, re.M).group(1)) for name in ("pswpin", "pswpout")}
    processes = command(["ps", "-eo", "pid=,comm="], capture_output=True).stdout.splitlines()
    device = command(["df", "-P", "/tmp"], capture_output=True).stdout.splitlines()[-1].split()[0]
    competing = []
    for line in processes:
        pid, name = line.strip().split(maxsplit=1)
        if int(pid) != os.getpid() and re.search(r"^(cargo|rustc|pytest|maturin|ollama|vllm|postgres|sqlite3)$", name):
            competing.append({"pid": int(pid), "name": name})
    return {
        "cpu_governor": governor,
        "competing_processes": competing,
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
            pending_throughout = any(admitted <= start and completed > end for admitted, completed in intervals)
            progress = any(start < completed <= end for _, completed in intervals)
            if pending_throughout and not progress:
                return False
    return True


def check_raw_observations(raw: dict, phase: str) -> None:
    """Prove the raw stream has the claimed work and projection completions."""
    if raw.get("smoke"):
        raise ValueError("smoke run cannot qualify")
    before = raw.get("environment_start", {})
    after = raw.get("environment_end", {})
    actual_environment_valid = (
        before.get("cpu_governor") == after.get("cpu_governor") == "performance"
        and before.get("competing_processes") == after.get("competing_processes") == []
        and all(before.get(key) == after.get(key) for key in ("swap_pages_in", "swap_pages_out", "database_device"))
        and str(before.get("database_device", "")).startswith("/dev/nvme")
    )
    if raw.get("environment_valid") is not True or not actual_environment_valid:
        raise ValueError("invalid repetition environment")
    actual = {
        "canonical_writes": sum(op["class"] == "canonical_write" for op in raw["operations"]),
        "foreground_hybrid_queries": sum(op["class"] == "foreground_hybrid_query" for op in raw["operations"]),
        "direct_embeds": sum(op["class"] == "direct_embed" for op in raw["operations"]),
    }
    if actual != raw["operation_counts"]:
        raise ValueError("operation counts diverge from raw observations")
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
    if phase == "candidate":
        scheduler = raw.get("scheduler_runtime_threads", 2)
        embed_workers = raw.get("embedder_pool_size", 1)
        if raw["projection_admission_high_water"] > scheduler * 64:
            raise ValueError("projection admission bound exceeded")
        if raw["embed_requests_waiting_high_water"] > 4 * embed_workers:
            raise ValueError("embed queue bound exceeded")
        if raw["engine_thread_inventory"] != 1 + scheduler + 8 + embed_workers:
            raise ValueError("engine thread inventory mismatch")
        if raw["provider_peak_concurrency"] > embed_workers:
            raise ValueError("provider concurrency bound exceeded")


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
        "projection_completions": len(raw["projection_completions"]) / duration,
        "foreground_hybrid_queries": len(successful["foreground_hybrid_query"]) / duration,
        "direct_embeds": len(successful["direct_embed"]) / duration,
    }
    latency_ns = {name: [(item["completed_ns"] - item["admitted_ns"]) / 1e6 for item in items] for name, items in successful.items()}
    latency_ns["projection_freshness"] = [(item["observed_ns"] - item["committed_ns"]) / 1e6 for item in raw["projection_completions"]]
    latency_ns["close"] = [(raw["close_end_ns"] - raw["close_start_ns"]) / 1e6]
    names = {"canonical_commit": "canonical_write", "projection_freshness": "projection_freshness", "foreground_hybrid_query": "foreground_hybrid_query", "direct_embed": "direct_embed", "close": "close"}
    latency = {name: {str(rank): percentile(latency_ns[key], rank) for rank in (50, 95, 99)} for name, key in names.items()}
    if phase == "candidate":
        latency["embed_queue_wait"] = {str(rank): percentile(raw["embed_queue_wait_ns"], rank) / 1e6 for rank in (50, 95, 99)}
    high_water = {"durable_projection_backlog": raw["projection_backlog_high_water"]}
    inventory = {"provider_concurrency": raw["provider_peak_concurrency"], "engine_threads": raw["engine_thread_inventory"], "residual_workers_after_close": raw["residual_workers_after_close"]}
    if phase == "candidate":
        conn_match = re.search(r"creation=writer:(\d+),readers:(\d+),dispatcher:(\d+),workers:(\d+),probes:(\d+)", raw["connection_inventory"])
        if not conn_match:
            raise ValueError("missing SQLite inventory")
        connections = sum(int(value) for value in conn_match.groups())
        if connections != 1 + 1 + raw.get("scheduler_runtime_threads", 2) + 8:
            raise ValueError("SQLite connection inventory mismatch")
        inventory["sqlite_connections"] = connections
        high_water["projection_rows_active_plus_queued"] = raw["projection_admission_high_water"]
        high_water["embed_requests_waiting"] = raw["embed_requests_waiting_high_water"]
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
    args = parser.parse_args()
    protocol = json.loads(PROTOCOL.read_text())
    source = args.source.resolve()
    source_sha = git_sha(source)
    if args.phase == "entry" and source_sha != protocol["entry_engine_candidate_sha"]:
        raise ValueError("wrong entry candidate")
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
    start_environment = environment()
    for name in repetition_order(protocol, args.phase):
        direction, repetition = re.fullmatch(r"(?:entry|candidate)_(projection_heavy|foreground_heavy)_(\d)", name).groups()
        repetition = int(repetition)
        before = environment()
        path = output / f"{name}.raw.json"
        env = dict(os.environ, D27_DIRECTION=direction, D27_REPETITION=str(repetition), D27_RAW_OUTPUT=str(path), D27_CORPUS=str(corpus))
        command([str(binary), "--ignored", "--exact", "d27_runtime_qualification", "--nocapture"], env=env)
        after = environment()
        raw = json.loads(path.read_text())
        validate_raw_contract(raw, protocol, direction, repetition)
        raw["environment_start"] = before
        raw["environment_end"] = after
        raw["environment_valid"] = before["cpu_governor"] == after["cpu_governor"] == protocol["runner"]["cpu_governor"] and not before["competing_processes"] and not after["competing_processes"] and all(before[key] == after[key] for key in ("swap_pages_in", "swap_pages_out"))
        all_raw.append(raw)
        metrics[direction].append(summarize_raw(raw, args.phase))
    raw_output.write_text("".join(json.dumps(item, sort_keys=True, separators=(",", ":")) + "\n" for item in all_raw))
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
    receipt = verifier.validate_receipt(receipt, protocol, PROTOCOL, {"runner": runner_bundle, "binary": binary, "corpus": corpus, "raw": raw_output}, entry)
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(output / "receipt.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
