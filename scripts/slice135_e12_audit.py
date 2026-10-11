#!/usr/bin/env python3
"""Independently audit E01–E12 raw attempts and recompute nearest-rank statistics."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess


def _hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _artifact_hash(path: Path) -> str:
    if path.exists():
        return _hash(path)
    archive = path.with_name(path.name + ".gz")
    if not archive.exists():
        raise ValueError(f"artifact absent: {path.name}")
    try:
        with gzip.open(archive, "rb") as stream:
            hasher = hashlib.sha256()
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                hasher.update(chunk)
            return hasher.hexdigest()
    except (OSError, EOFError) as error:
        raise ValueError(f"artifact archive malformed: {path.name}") from error


def _equal(actual: object, expected: object, label: str) -> None:
    if actual != expected:
        raise ValueError(f"{label} mismatch")


def _rank(values: list[int], percentile: int) -> int:
    return values[math.ceil(len(values) * percentile / 100) - 1]


def audit_resource_report(recorded: dict, report: str) -> dict:
    """Independently parse GNU Time and reject changed recorded resource values."""
    fields = dict(re.findall(r"^\s*([^\n]+?):\s+([^\n]+)$", report, re.MULTILINE))
    names = {"user_cpu_s": "User time (seconds)",
             "system_cpu_s": "System time (seconds)",
             "peak_rss_kib": "Maximum resident set size (kbytes)",
             "major_faults": "Major (requiring I/O) page faults",
             "swap_events": "Swaps", "fs_inputs": "File system inputs",
             "fs_outputs": "File system outputs"}
    try:
        elapsed = fields["Elapsed (wall clock) time (h:mm:ss or m:ss)"].split(":")
        duration = sum(float(part) * 60**index for index, part in enumerate(reversed(elapsed)))
        parsed = {"method": "gnu-time", "scope": "measured child", "elapsed_wall_s": duration}
        for key, name in names.items():
            parsed[key] = float(fields[name]) if key in ("user_cpu_s", "system_cpu_s") else int(fields[name])
    except (KeyError, ValueError) as error:
        raise ValueError("GNU Time resource report malformed") from error
    for key, value in recorded.items():
        _equal(value, parsed.get(key), f"resource {key}")
    return parsed


def audit_runner_sources(root: Path, shared: Path) -> None:
    """Verify archived runner bundle names exact retained producer source bytes."""
    bundle = json.loads((root / "runner").read_text())
    _equal(bundle.get("workload"), _hash(shared / "workload.rs"), "workload source")
    candidates = [shared / "pilot-adapter.py", shared / "candidate-adapter.py"]
    if bundle.get("adapter") not in {_hash(path) for path in candidates if path.exists()}:
        raise ValueError("adapter source mismatch")


def audit_receipt(raw: dict, protocol: dict, root: Path,
                  model_dir: Path | None = None) -> dict:
    """Reject wrong bindings, state or attempts without trusting the producer's summary."""
    for value in (raw, protocol):
        if type(value.get("schema_version")) is not int or value["schema_version"] != 1:
            raise ValueError("schema version invalid")
    source = protocol.get("source_sha")
    if not isinstance(source, str) or not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("protocol source invalid")
    _equal(raw.get("source_sha"), source, "source")
    _equal(protocol.get("runner_sha256"), _hash(root / "runner"), "protocol runner")
    _equal(raw.get("runner_sha256"), _hash(root / "runner"), "raw runner")
    protocol_bytes = json.dumps(protocol, sort_keys=True).encode()
    _equal(raw.get("protocol_sha256"), hashlib.sha256(protocol_bytes).hexdigest(), "protocol hash")
    assets = protocol.get("artifact_sha256")
    if not isinstance(assets, dict) or not assets:
        raise ValueError("artifact map missing")
    _equal(raw.get("artifact_sha256"), assets, "raw artifact map")
    for name, expected in assets.items():
        if not isinstance(name, str) or Path(name).is_absolute() or ".." in Path(name).parts:
            raise ValueError("artifact path invalid")
        path = root / name
        if not path.exists() and model_dir is not None and name in {
                "config.json", "model.safetensors", "tokenizer.json"}:
            path = model_dir / name
        _equal(_artifact_hash(path), expected, f"artifact {name}")
    if "binary" in assets and "binary_sha256" in raw:
        _equal(raw["binary_sha256"], assets["binary"], "workload binary")
    if "corpus" in assets and "corpus_sha256" in raw:
        _equal(raw["corpus_sha256"], assets["corpus"], "seed corpus")
    _equal(raw.get("settings"), protocol.get("settings"), "settings")
    environment = raw.get("environment")
    if not isinstance(environment, dict) or environment.get("invalidators") != []:
        raise ValueError("environment invalid")
    start, end = environment.get("start"), environment.get("end")
    if not isinstance(start, dict) or not isinstance(end, dict):
        raise ValueError("environment snapshots missing")
    for field in ("host", "kernel", "cpu", "storage", "governor", "toolchain"):
        if field == "host" or field in start or field in end:
            if not start.get(field) or start[field] != end.get(field):
                raise ValueError(f"environment {field} invalid")
    for snapshot in (start, end):
        if snapshot.get("competing_jobs", []) != []:
            raise ValueError("competing jobs")
        if "disk_free_bytes" in snapshot and snapshot["disk_free_bytes"] < protocol.get("min_disk_free_bytes", 0):
            raise ValueError("disk pressure")
    if raw.get("resources", {}).get("swap_events", 0) != 0:
        raise ValueError("child swap events")
    warnings = []
    if "swap_pages" in start or "swap_pages" in end:
        before, after = start.get("swap_pages"), end.get("swap_pages")
        if type(before) is not int or type(after) is not int or after < before:
            raise ValueError("host swap counters invalid")
        if after > before:
            warnings.append(f"host swap drift: {after - before} pages; child swap events: 0")
    expected_cells = protocol.get("cells")
    actual_cells = raw.get("cells")
    if not isinstance(expected_cells, dict) or not expected_cells or not isinstance(actual_cells, dict):
        raise ValueError("cells absent")
    _equal(set(actual_cells), set(expected_cells), "cell names")
    sample_count = protocol["settings"].get("samples")
    if type(sample_count) is not int or sample_count < 100:
        raise ValueError("sample rule invalid")
    summary = {}
    for cell_name, rule in expected_cells.items():
        attempts = actual_cells[cell_name]
        if not isinstance(attempts, list):
            raise ValueError(f"{cell_name}: attempts absent")
        valid = []
        failures = 0
        for position, attempt in enumerate(attempts):
            if not isinstance(attempt, dict):
                raise ValueError(f"{cell_name} attempt {position} malformed")
            if attempt.get("valid") is False:
                if not isinstance(attempt.get("reason"), str) or not attempt["reason"].strip():
                    raise ValueError(f"{cell_name} invalid attempt lacks reason")
                if attempt.get("semantic_ok") is False or ("observed_checks" in attempt and
                        attempt["observed_checks"] != rule["expected_checks"]):
                    raise ValueError(f"{cell_name}: semantic failure marked invalid")
                failures += 1
                continue
            if attempt.get("valid") is not True or attempt.get("semantic_ok", True) is not True:
                raise ValueError(f"{cell_name}: false validity")
            if attempt.get("observed_checks") != rule["expected_checks"]:
                raise ValueError(f"{cell_name}: state or output mismatch at attempt {position}")
            elapsed = attempt.get("latency_ns")
            if type(elapsed) is not int or elapsed <= 0:
                raise ValueError(f"{cell_name}: latency invalid")
            valid.append(elapsed)
        if len(valid) != sample_count:
            raise ValueError(f"{cell_name}: wrong sample count {len(valid)}")
        valid.sort()
        result = {"valid_samples": len(valid), "invalid_attempts": failures,
                  "valid_attempt_fraction": len(valid) / len(attempts),
                  "p50_ns": _rank(valid, 50), "p95_ns": _rank(valid, 95),
                  "maximum_ns": valid[-1]}
        if rule.get("kind") == "query" and len(valid) >= 1000:
            result["p99_ns"] = _rank(valid, 99)
        else:
            result["unsupported_statistics"] = ["p99"]
        summary[cell_name] = result
    return {"schema_version": 1, "source_sha": source, "cells": summary,
            "environment_warnings": warnings,
            "protocol_sha256": hashlib.sha256(protocol_bytes).hexdigest()}


def main() -> None:
    """Read a retained attempt and reject source, protocol, artifact or output drift."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--source-ref", default="HEAD")
    parser.add_argument("--expectations", type=Path, required=True)
    parser.add_argument("--model-dir", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.receipt.resolve()
    if (root.parent / "shared").is_dir():
        audit_runner_sources(root, root.parent / "shared")
    raw_bytes = (root / "raw.json").read_bytes()
    protocol_bytes = (root / "protocol.json").read_bytes()
    raw = json.loads(raw_bytes)
    protocol = json.loads(protocol_bytes)
    _equal(protocol_bytes, json.dumps(protocol, sort_keys=True).encode(), "protocol bytes")
    source = subprocess.check_output(["git", "rev-parse", args.source_ref], cwd=args.checkout, text=True).strip()
    _equal(protocol.get("source_sha"), source, "checkout source")
    changed = subprocess.check_output(["git", "diff", "--name-only", source, "HEAD", "--",
                                       "src/rust/crates", "Cargo.toml", "Cargo.lock"],
                                      cwd=args.checkout, text=True).strip()
    if changed:
        raise ValueError(f"measured source changed since {source}: {changed}")
    dirty = subprocess.check_output(["git", "status", "--porcelain", "--", "src/rust/crates", "Cargo.toml", "Cargo.lock"],
                                    cwd=args.checkout, text=True).strip()
    if dirty:
        raise ValueError("measured checkout dirty")
    _equal(protocol["settings"]["cargo_lock_sha256"], _hash(args.checkout / "Cargo.lock"), "cargo lock")
    expectations = json.loads(args.expectations.read_text())
    for name, rule in protocol["cells"].items():
        _equal(rule["expected_checks"], expectations[name], f"{name} pinned checks")
    provenance = json.loads((root / "build-provenance.json").read_text())
    for name, expected in (("source_sha", source), ("binary_sha256", _artifact_hash(root / "binary")),
                           ("cargo_lock_sha256", _hash(args.checkout / "Cargo.lock"))):
        _equal(provenance.get(name), expected, f"build provenance {name}")
    gnu_time = (root / "run.stderr.log").read_text()
    resources = audit_resource_report(raw.get("resources", {}), gnu_time)
    summary = audit_receipt(raw, protocol, root, args.model_dir)
    summary["raw_sha256"] = hashlib.sha256(raw_bytes).hexdigest()
    summary["audit_sha256"] = _hash(Path(__file__))
    summary["run_stderr_sha256"] = _hash(root / "run.stderr.log")
    summary["resources"] = resources
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": "PASS", "cells": {k: v["valid_samples"] for k, v in summary["cells"].items()}}))


if __name__ == "__main__":
    main()
