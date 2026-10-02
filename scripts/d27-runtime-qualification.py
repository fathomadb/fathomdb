#!/usr/bin/env python3
"""Validate D27 raw qualification receipts against the frozen protocol.

The workload emitter is a separate Rust test so that the same source file can
be compiled in the historical and candidate engine checkouts. This verifier
never changes an observed value or treats unavailable entry metrics as zero.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import statistics


ROOT = Path(__file__).resolve().parents[1]
PROTOCOL = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
PROTOCOL_V2 = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol-v2.json"
LEGACY_PROTOCOL_SHA256 = "b835f79a426a038e3528a9cf820177876451a02c3defad7aa0c545ebf642441b"
DIRECTIONS = ("projection_heavy", "foreground_heavy")
HISTORICAL_ONLY = frozenset(("embed_queue_wait", "embed_requests_waiting"))


def sha256(path: Path) -> str:
    """Return the SHA-256 digest of a readable artifact without text conversion."""
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def number(value: object, label: str) -> float:
    require(isinstance(value, (int, float)) and not isinstance(value, bool), f"{label}: missing number")
    result = float(value)
    require(math.isfinite(result) and result >= 0, f"{label}: nonfinite or negative")
    return result


def center(values: list[float]) -> dict[str, float]:
    """Calculate the three-repetition median and median absolute deviation."""
    require(len(values) == 3, "repetitions: expected exactly three values")
    median = statistics.median(values)
    return {"median": median, "mad": statistics.median(abs(value - median) for value in values)}


def verify_raw_linkage(receipt: dict, protocol: dict, raw_path: Path) -> None:
    """Recompute every per-repetition metric from retained raw observations."""
    runner_path = ROOT / "scripts/d27-runtime-runner.py"
    spec = importlib.util.spec_from_file_location("d27_raw_runner", runner_path)
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    raw = [json.loads(line) for line in raw_path.read_text().splitlines()]
    order = runner.repetition_order(protocol, receipt["phase"])
    require(len(raw) == len(order), "raw repetition count mismatch")
    by_direction = {direction: [] for direction in DIRECTIONS}
    for item, name in zip(raw, order):
        direction = None
        repetition = None
        for candidate in DIRECTIONS:
            prefix = f"{receipt['phase']}_{candidate}_"
            if name.startswith(prefix):
                direction = candidate
                repetition = int(name[len(prefix):])
                break
        require(direction is not None, "raw repetition order mismatch")
        runner.validate_raw_contract(item, protocol, direction, repetition, receipt["phase"])
        by_direction[direction].append(runner.summarize_raw(item, receipt["phase"], protocol))
    for direction in DIRECTIONS:
        expected = receipt["per_repetition_metrics"][direction]
        require(by_direction[direction] == expected, f"raw metric mismatch: {direction}")


def validate_entry_for_candidate(entry: dict, protocol: dict, protocol_path: Path, artifacts: dict[str, Path] | None) -> dict:
    """Revalidate the historical entry from its original hashed artifacts."""
    require(artifacts is not None and set(artifacts) == {"runner", "binary", "corpus", "raw"} and all(isinstance(path, Path) for path in artifacts.values()), "entry artifacts are required")
    require(isinstance(entry, dict), "entry receipt is required")
    require(entry.get("phase") == "entry", "candidate comparison needs an entry receipt")
    verify_raw_linkage(entry, protocol, artifacts["raw"])
    validated = validate_receipt(entry, protocol, protocol_path, artifacts)
    require(entry.get("aggregate_metrics") == validated["aggregate_metrics"], "entry aggregate differs from raw recomputation")
    return validated


def validate_receipt(
    receipt: dict,
    protocol: dict,
    protocol_path: Path,
    artifacts: dict[str, Path],
    entry: dict | None = None,
) -> dict:
    """Return computed aggregates or raise for an incomplete or false PASS.

    ``artifacts`` names the exact runner, test binary, corpus and raw output
    retained alongside the receipt. Candidate receipts require a validated
    entry aggregate supplied as ``entry``.
    """
    require(protocol == json.loads(protocol_path.read_text()), "frozen protocol content mismatch")
    version = protocol.get("schema_version")
    require(version in (1, 2), "unsupported protocol schema")
    if version == 2:
        require(protocol_path.resolve() == PROTOCOL_V2.resolve(), "unsupported successor protocol path")
        require(sha256(PROTOCOL) == LEGACY_PROTOCOL_SHA256, "historical protocol changed")
        legacy = json.loads(PROTOCOL.read_text())
        stable, successor = copy.deepcopy(legacy), copy.deepcopy(protocol)
        for value in (stable, successor):
            value.pop("schema_version")
            value.pop("swap_policy")
            value["comparison_rule"].pop("invalid_environment")
        require(stable == successor, "successor protocol changed a non-swap rule")
        require(protocol["swap_policy"].get("mode") == "report_only" and
                "max_total_pages_per_repetition" not in protocol["swap_policy"],
                "unsupported successor swap policy")
    for name, field in (
        ("runner", "runner_sha256"),
        ("binary", "binary_sha256"),
        ("corpus", "corpus_sha256"),
        ("raw", "raw_output_sha256"),
    ):
        require(receipt.get(field) == sha256(artifacts[name]), f"{field}: sha256 mismatch")
    require(receipt.get("protocol_sha256") == sha256(protocol_path), "protocol_sha256: sha256 mismatch")

    phase = receipt.get("phase")
    require(phase in ("entry", "candidate"), "phase must be entry or candidate")
    if phase == "entry":
        require(receipt.get("source_sha") == protocol["entry_engine_candidate_sha"], "wrong entry candidate")
        require(set(receipt.get("historical_unavailable", [])) == set(protocol["metrics"]["historical_unavailable"]), "historical unavailable inventory mismatch")
    else:
        require(entry is not None and entry.get("status") == "PASS", "candidate requires validated PASS entry")
        require(receipt.get("source_sha") != protocol["entry_engine_candidate_sha"], "candidate must differ from entry candidate")
        require(receipt.get("corpus_sha256") == entry.get("corpus_sha256"), "candidate corpus sha256 mismatch")
        require(receipt.get("protocol_sha256") == entry.get("protocol_sha256"), "candidate protocol sha256 mismatch")
        require(not receipt.get("historical_unavailable"), "candidate cannot mark dispatch metrics unavailable")

    runner = receipt.get("runner_inventory", {})
    expected = protocol["runner"]
    for key in ("host", "operating_system", "database_storage", "build"):
        require(runner.get(key) == expected[key], f"runner {key} mismatch")
    require(runner.get("features") == expected["features"], "runner features mismatch")
    require(number(runner.get("online_cpus"), "runner online_cpus") >= expected["minimum_online_cpus"], "runner online_cpus below minimum")
    require(number(runner.get("memory_gib"), "runner memory_gib") >= expected["minimum_memory_gib"], "runner memory_gib below minimum")
    for boundary in ("start", "end"):
        environment = receipt.get(f"environment_{boundary}", {})
        require(environment.get("pid_namespace") == expected["host_pid_namespace"], f"environment {boundary} pid namespace mismatch")
        pid_one_namespace = environment.get("pid_one_namespace")
        process_view = (environment.get("pid_one_comm"), environment.get("ps_pid_one_comm"), environment.get("procfs_hidepid"))
        require(pid_one_namespace in ("", expected["host_pid_namespace"]) and process_view == (expected["host_pid_one_comm"], expected["host_pid_one_comm"], "0"), f"environment {boundary} process view mismatch")
        runner_pid = environment.get("runner_pid")
        require(isinstance(runner_pid, int) and not isinstance(runner_pid, bool) and runner_pid > 1 and environment.get("proc_self_pid") == runner_pid and environment.get("ps_self_pid") == runner_pid, f"environment {boundary} process view runner PID mismatch")
        require(environment.get("cpu_governor") == expected["cpu_governor"], f"environment {boundary} governor mismatch")
        require(environment.get("competing_processes") == [], f"environment {boundary} competing process")
        number(environment.get("swap_pages_in"), f"environment {boundary} swap_pages_in")
        number(environment.get("swap_pages_out"), f"environment {boundary} swap_pages_out")
    metrics = protocol["metrics"]
    aggregate = {}
    repetitions = receipt.get("per_repetition_metrics", {})
    require(set(repetitions) == set(DIRECTIONS), "repetition directions mismatch")
    for direction in DIRECTIONS:
        runs = repetitions[direction]
        require(isinstance(runs, list) and len(runs) == protocol["execution"]["repetitions"], f"{direction} repetitions mismatch")
        direction_aggregate = {"throughput": {}, "latency_ms": {}}
        for index, run in enumerate(runs):
            label = f"{direction} repetition {index + 1}"
            require(run.get("environment_valid") is True, f"{label}: invalid environment")
            require(run.get("starvation_pass") is True, f"{label}: starvation")
            swap_deltas = []
            for key in ("swap_pages_in_delta", "swap_pages_out_delta"):
                value = run.get(key)
                require(isinstance(value, int) and not isinstance(value, bool) and value >= 0, f"{label} {key}: missing or negative")
                swap_deltas.append(value)
            if version == 1:
                require(sum(swap_deltas) <= protocol["swap_policy"]["max_total_pages_per_repetition"], f"{label}: combined swap movement exceeds cap")
            for count in metrics["counts"]:
                number(run.get("counts", {}).get(count), f"{label} {count}")
            for high_water in metrics["high_water"]:
                if phase == "entry" and high_water in HISTORICAL_ONLY:
                    require(high_water not in run.get("high_water", {}), f"{label} historical unavailable {high_water} imputed")
                elif phase == "entry" and high_water == "projection_rows_active_plus_queued" and high_water not in run.get("high_water", {}):
                    continue
                else:
                    number(run.get("high_water", {}).get(high_water), f"{label} {high_water}")
            for inventory in metrics["inventory"]:
                if phase == "entry" and inventory == "sqlite_connections":
                    require(inventory not in run.get("inventory", {}), f"{label} historical unavailable sqlite_connections imputed")
                    continue
                number(run.get("inventory", {}).get(inventory), f"{label} {inventory}")
            if phase == "candidate":
                for latency in metrics["candidate_only_latency_classes"]:
                    for percentile in metrics["latency_percentiles"]:
                        number(run.get("latency_ms", {}).get(latency, {}).get(str(percentile)), f"{label} {latency} p{percentile}")
            else:
                require(not HISTORICAL_ONLY.intersection(run.get("latency_ms", {})), f"{label} historical unavailable latency imputed")
        for metric in metrics["throughput_rates_per_second"]:
            values = [number(run.get("throughput", {}).get(metric), f"{direction} {metric}") for run in runs]
            direction_aggregate["throughput"][metric] = center(values)
        for latency in metrics["latency_classes"]:
            direction_aggregate["latency_ms"][latency] = {}
            for percentile in metrics["latency_percentiles"]:
                values = [number(run.get("latency_ms", {}).get(latency, {}).get(str(percentile)), f"{direction} {latency} p{percentile}") for run in runs]
                direction_aggregate["latency_ms"][latency][str(percentile)] = center(values)
        aggregate[direction] = direction_aggregate

    if phase == "candidate":
        for direction in DIRECTIONS:
            for metric in metrics["throughput_rates_per_second"]:
                baseline = entry["aggregate_metrics"][direction]["throughput"][metric]
                floor = baseline["median"] - max(0.10 * baseline["median"], 3 * baseline["mad"])
                require(aggregate[direction]["throughput"][metric]["median"] >= floor, f"{direction} {metric} below throughput floor")
            for latency in metrics["latency_classes"]:
                for percentile in metrics["latency_percentiles"]:
                    baseline = entry["aggregate_metrics"][direction]["latency_ms"][latency][str(percentile)]
                    ceiling = baseline["median"] + max(0.15 * baseline["median"], 3 * baseline["mad"])
                    require(aggregate[direction]["latency_ms"][latency][str(percentile)]["median"] <= ceiling, f"{direction} {latency} p{percentile} above latency ceiling")
    require(receipt.get("status") == "PASS", "receipt status is not PASS")
    return {**receipt, "aggregate_metrics": aggregate, "decision_rule_evaluation": {"status": "PASS", "rule": "D27 frozen median/MAD"}}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    parser.add_argument("--protocol", type=Path, default=PROTOCOL)
    for name in ("runner", "binary", "corpus", "raw"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--entry", type=Path)
    for name in ("runner", "binary", "corpus", "raw"):
        parser.add_argument(f"--entry-{name}", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    artifacts = {name: getattr(args, name) for name in ("runner", "binary", "corpus", "raw")}
    entry = json.loads(args.entry.read_text()) if args.entry else None
    input_receipt = json.loads(args.receipt.read_text())
    protocol = json.loads(args.protocol.read_text())
    verify_raw_linkage(input_receipt, protocol, args.raw)
    if input_receipt.get("phase") == "candidate":
        entry_artifacts = {name: getattr(args, f"entry_{name}") for name in ("runner", "binary", "corpus", "raw")}
        entry = validate_entry_for_candidate(entry, protocol, args.protocol, entry_artifacts)
    result = validate_receipt(input_receipt, protocol, args.protocol, artifacts, entry)
    encoded = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
