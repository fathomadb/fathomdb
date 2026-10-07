#!/usr/bin/env python3
"""Build and validate source-bound Slice 135 E01–E12 engine measurements."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time
import tomllib


ROOT = Path(__file__).resolve().parents[1]
WORKLOAD = ROOT / "scripts/slice135_e12_workload.rs"
MODEL = (Path.home() / ".cache/huggingface/hub/models--BAAI--bge-small-en-v1.5"
         / "snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a")
PINNED_MODEL_ASSETS = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
    "tokenizer.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
}
PATHS = ("open_fresh", "open_populated", "close", "canonical_write", "projection",
         "model_cpu", "text", "vector_stage", "hybrid", "graph_expand",
         "graph_evidence", "erasure")
QUERY_PATHS = frozenset(("text", "vector_stage", "hybrid", "graph_expand", "graph_evidence"))
SHA40 = re.compile(r"[0-9a-f]{40}\Z")
SHA64 = re.compile(r"[0-9a-f]{64}\Z")


def digest(data: bytes) -> str:
    """Return the SHA-256 digest of exact bytes."""
    return hashlib.sha256(data).hexdigest()


def file_hash(path: Path) -> str:
    """Hash an existing artifact, including resolved symlink targets."""
    return digest(path.read_bytes())


def validate_model_assets(model_dir: Path, expected: dict[str, str]) -> None:
    """Reject changed CPU model bytes before deriving any timed protocol."""
    for name, pinned_hash in expected.items():
        if file_hash(model_dir / name) != pinned_hash:
            raise ValueError(f"pinned model asset {name} mismatch")


def canonical_bytes(value: object) -> bytes:
    """Serialize a protocol with stable bytes for its receipt binding."""
    return json.dumps(value, sort_keys=True, allow_nan=False).encode()


def _assert_binding(name: str, actual: object, expected: object, width: int = 64) -> None:
    if not isinstance(actual, str) or not re.fullmatch(rf"[0-9a-f]{{{width}}}", actual):
        raise ValueError(f"{name}: malformed binding")
    if actual != expected:
        raise ValueError(f"{name}: binding mismatch")


def _rank(values: list[int], fraction: float) -> int:
    return values[math.ceil(len(values) * fraction) - 1]


def parse_resources(report: str) -> dict:
    """Parse the measured GNU Time child report; fail on missing resource fields."""
    fields = dict(re.findall(r"^\s*([^\n]+?):\s+([^\n]+)$", report, re.MULTILINE))
    def _value(name: str) -> str:
        if name not in fields:
            raise ValueError(f"GNU Time {name} missing")
        return fields[name].strip()

    elapsed = _value("Elapsed (wall clock) time (h:mm:ss or m:ss)")
    parts = [float(part) for part in elapsed.split(":")]
    if len(parts) not in (2, 3):
        raise ValueError("GNU Time elapsed malformed")
    elapsed_s = sum(part * 60**index for index, part in enumerate(reversed(parts)))
    return {"method": "gnu-time", "scope": "measured child",
            "user_cpu_s": float(_value("User time (seconds)")),
            "system_cpu_s": float(_value("System time (seconds)")),
            "elapsed_wall_s": elapsed_s,
            "peak_rss_kib": int(_value("Maximum resident set size (kbytes)")),
            "major_faults": int(_value("Major (requiring I/O) page faults")),
            "swap_events": int(_value("Swaps")),
            "fs_inputs": int(_value("File system inputs")),
            "fs_outputs": int(_value("File system outputs"))}


def validate_measurement(raw: dict, protocol: dict, artifacts_root: Path) -> dict:
    """Reject a degraded attempt and independently summarize exact bound output."""
    if raw.get("schema_version") != 1 or protocol.get("schema_version") != 1:
        raise ValueError("schema version mismatch")
    expected_source = protocol.get("source_sha")
    if not isinstance(expected_source, str) or not SHA40.fullmatch(expected_source):
        raise ValueError("source_sha malformed")
    _assert_binding("source_sha", raw.get("source_sha"), expected_source, 40)
    runner_hash = file_hash(artifacts_root / "runner")
    _assert_binding("runner_sha256", protocol.get("runner_sha256"), runner_hash)
    _assert_binding("runner_sha256", raw.get("runner_sha256"), runner_hash)
    _assert_binding("protocol_sha256", raw.get("protocol_sha256"), digest(canonical_bytes(protocol)))
    expected_artifacts = protocol.get("artifact_sha256")
    if not isinstance(expected_artifacts, dict) or not expected_artifacts:
        raise ValueError("artifact bindings missing")
    if raw.get("artifact_sha256") != expected_artifacts:
        raise ValueError("artifact bindings changed")
    root = artifacts_root.resolve()
    for name, expected in expected_artifacts.items():
        if not isinstance(name, str) or Path(name).is_absolute() or ".." in Path(name).parts:
            raise ValueError(f"{name}: invalid artifact name")
        path = root / name
        _assert_binding(f"artifact {name}", file_hash(path), expected)
    if raw.get("settings") != protocol.get("settings"):
        raise ValueError("settings changed")
    environment = raw.get("environment")
    if not isinstance(environment, dict) or environment.get("invalidators") != []:
        raise ValueError("environment invalidators")
    start, end = environment.get("start"), environment.get("end")
    if not isinstance(start, dict) or not isinstance(end, dict):
        raise ValueError("environment snapshots missing")
    if not start.get("host") or start.get("host") != end.get("host"):
        raise ValueError("host changed or missing")
    for name in ("kernel", "cpu", "storage", "governor", "toolchain"):
        if name in start or name in end:
            if not start.get(name) or start.get(name) != end.get(name):
                raise ValueError(f"environment {name} changed or missing")
    if start.get("competing_jobs", []) or end.get("competing_jobs", []):
        raise ValueError("competing jobs")
    if "disk_free_bytes" in start and (start["disk_free_bytes"] < protocol.get("min_disk_free_bytes", 0)
                                       or end["disk_free_bytes"] < protocol.get("min_disk_free_bytes", 0)):
        raise ValueError("disk pressure")
    if raw.get("resources", {}).get("swap_events", 0) != 0:
        raise ValueError("child swap events")
    specs, cells = protocol.get("cells"), raw.get("cells")
    if not isinstance(specs, dict) or not specs or not isinstance(cells, dict) or set(cells) != set(specs):
        raise ValueError("cells changed or missing")
    summary = {}
    for name, spec in specs.items():
        samples = cells[name]
        if not isinstance(samples, list):
            raise ValueError(f"{name}: attempts missing")
        values = []
        invalid = 0
        for index, attempt in enumerate(samples):
            if not isinstance(attempt, dict):
                raise ValueError(f"{name} attempt {index}: malformed")
            if attempt.get("valid") is False:
                if not attempt.get("reason"):
                    raise ValueError(f"{name} attempt {index}: invalid reason missing")
                if attempt.get("semantic_ok") is False or ("observed_checks" in attempt and
                        attempt["observed_checks"] != spec["expected_checks"]):
                    raise ValueError(f"{name} attempt {index}: semantic failure")
                invalid += 1
                continue
            if attempt.get("valid") is not True or attempt.get("semantic_ok", True) is not True:
                raise ValueError(f"{name} attempt {index}: validity missing")
            if attempt.get("observed_checks") != spec["expected_checks"]:
                raise ValueError(f"{name} attempt {index}: output mismatch")
            ns = attempt.get("latency_ns")
            if type(ns) is not int or ns <= 0:
                raise ValueError(f"{name} attempt {index}: latency invalid")
            values.append(ns)
        required = protocol["settings"]["samples"]
        if len(values) != required:
            raise ValueError(f"{name}: {required} valid samples required, got {len(values)}")
        values.sort()
        cell = {"valid_samples": len(values), "invalid_attempts": invalid,
                "valid_attempt_fraction": len(values) / len(samples),
                "p50_ns": _rank(values, .50), "p95_ns": _rank(values, .95),
                "maximum_ns": values[-1]}
        if spec["kind"] == "query" and len(values) >= 1000:
            cell["p99_ns"] = _rank(values, .99)
        else:
            cell["unsupported_statistics"] = ["p99"]
        summary[name] = cell
    return {"schema_version": 1, "source_sha": expected_source,
            "protocol_sha256": digest(canonical_bytes(protocol)), "cells": summary}


def _run(command: list[str], *, cwd: Path | None = None, env: dict | None = None,
         timeout: int = 900) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True,
                          timeout=timeout, check=False)


def inventory() -> dict:
    """Record host state surrounding the measured child process."""
    cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.processor())
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    swap = sum(int(line.split()[1]) for line in Path("/proc/vmstat").read_text().splitlines()
               if line.startswith(("pswpin ", "pswpout ")))
    competing = []
    for line in _run(["ps", "-eo", "pid=,comm="]).stdout.splitlines():
        pid, name = line.split(maxsplit=1)
        if int(pid) != os.getpid() and name in {"cargo", "rustc", "maturin", "pytest", "vllm", "ollama"}:
            competing.append({"pid": int(pid), "name": name})
    return {"time_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "host": platform.node(), "kernel": platform.release(), "cpu": cpu,
            "storage": os.statvfs("/tmp").f_fsid, "governor": governor.read_text().strip()
            if governor.exists() else "unavailable", "toolchain": _run(["rustc", "--version"]).stdout.strip(),
            "swap_pages": swap, "disk_free_bytes": shutil.disk_usage("/tmp").free,
            "competing_jobs": competing}


def source_identity(checkout: Path) -> str:
    """Require an exact clean source and dependency lock for measured code."""
    sha = _run(["git", "rev-parse", "HEAD"], cwd=checkout).stdout.strip()
    if not SHA40.fullmatch(sha):
        raise ValueError("source SHA unavailable")
    dirty = _run(["git", "status", "--porcelain", "--", "src/rust/crates", "Cargo.lock", "Cargo.toml"],
                 cwd=checkout).stdout.strip()
    if dirty:
        raise ValueError(f"measured source dirty: {dirty}")
    return sha


def manifest(checkout: Path, build_dir: Path) -> Path:
    """Bind a private test crate to the selected checkout and its Cargo lock."""
    build_dir.mkdir(parents=True, exist_ok=True)
    patch = tomllib.loads((checkout / "Cargo.toml").read_text()).get("patch", {}).get("crates-io", {})
    lines = []
    for name, values in patch.items():
        fields = {key: str((checkout / value).resolve()) if key == "path" else value
                  for key, value in values.items()}
        lines.append(f"{name} = {{ " + ", ".join(f"{key} = {json.dumps(value)}"
                                                for key, value in fields.items()) + " }")
    path = build_dir / "Cargo.toml"
    path.write_text('[package]\nname = "slice135-e12"\nversion = "0.1.0"\nedition = "2021"\n'
                    '[dependencies]\n'
                    f'fathomdb-engine = {{ path = "{checkout}/src/rust/crates/fathomdb-engine", features = ["test-hooks", "default-embedder"] }}\n'
                    f'fathomdb-embedder-api = {{ path = "{checkout}/src/rust/crates/fathomdb-embedder-api" }}\n'
                    'serde_json = "1"\nsha2 = "0.11"\ntempfile = "3"\n'
                    '[[test]]\nname = "slice135_e12_workload"\n'
                    f'path = "{WORKLOAD}"\n[patch.crates-io]\n' + "\n".join(lines) + "\n")
    shutil.copyfile(checkout / "Cargo.lock", build_dir / "Cargo.lock")
    return path


def main() -> None:
    """Build once or reuse a hash-verified binary and retain each raw attempt."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--binary", type=Path, help="Reuse an already built exact workload binary")
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--cells", nargs="+", choices=PATHS, default=list(PATHS))
    args = parser.parse_args()
    checkout, output = args.checkout.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    source_sha = source_identity(checkout)
    validate_model_assets(MODEL, PINNED_MODEL_ASSETS)
    if args.samples < 100 or len(set(args.cells)) != len(args.cells):
        raise ValueError("at least 100 samples and unique cells required")
    bundle = output / "runner"
    bundle.write_bytes(canonical_bytes({"adapter": file_hash(Path(__file__)),
                                        "workload": file_hash(WORKLOAD)}))
    corpus = output / "corpus"
    corpus.write_bytes(("\n".join([f"doc-{i}:needle memory document {i} for bounded search"
                                   for i in range(32)] + ["graph-source:canonical graph source bytes",
                                   "graph-root:root claim", "graph-target:target claim",
                                   "graph-edge:graph-root:supports:graph-target"]) + "\n").encode())
    for file in ("config.json", "model.safetensors", "tokenizer.json"):
        (output / file).symlink_to(MODEL / file)
    binary = output / "binary"
    env = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu",
           "CARGO_TARGET_DIR": "/tmp/fathomdb-slice135-e12-target"}
    if args.binary:
        previous = json.loads((args.binary.parent / "build-provenance.json").read_text())
        for key, value in (("source_sha", source_sha),
                           ("workload_sha256", file_hash(WORKLOAD)),
                           ("cargo_lock_sha256", file_hash(checkout / "Cargo.lock")),
                           ("binary_sha256", file_hash(args.binary))):
            if previous.get(key) != value:
                raise ValueError(f"reused binary {key} mismatch")
        shutil.copyfile(args.binary, binary)
    else:
        build = _run(["cargo", "test", "--offline", "--release", "--no-run", "--message-format=json",
                      "--manifest-path", str(manifest(checkout, output / "build"))], env=env)
        (output / "build.stdout.log").write_text(build.stdout)
        (output / "build.stderr.log").write_text(build.stderr)
        if build.returncode:
            raise ValueError(f"build failed: {build.returncode}: {build.stderr[-3000:]}")
        executables = [Path(message["executable"]) for line in build.stdout.splitlines()
                       if (message := json.loads(line)).get("target", {}).get("name") == "slice135_e12_workload"
                       and message.get("executable")]
        if len(executables) != 1:
            raise ValueError(f"expected one binary, found {executables}")
        shutil.copyfile(executables[0], binary)
    binary.chmod(0o755)
    (output / "build-provenance.json").write_text(json.dumps({
        "source_sha": source_sha, "workload_sha256": file_hash(WORKLOAD),
        "cargo_lock_sha256": file_hash(checkout / "Cargo.lock"),
        "binary_sha256": file_hash(binary), "reused_from": str(args.binary) if args.binary else None,
    }, indent=2, sort_keys=True) + "\n")
    # The observed checks are pinned before timing from an audited probe, never copied from timed output.
    expected = json.loads((ROOT / "dev/plans/0.8.27/features/slice-135/e12-expected-checks.json").read_text())
    protocol = {"schema_version": 1, "source_sha": source_sha,
                "runner_sha256": file_hash(bundle),
                "artifact_sha256": {file: file_hash(output / file) for file in
                                    ("binary", "corpus", "config.json", "model.safetensors", "tokenizer.json")},
                "settings": {"samples": args.samples, "device": "cpu", "cells": args.cells,
                             "cargo_lock_sha256": file_hash(checkout / "Cargo.lock")},
                "min_disk_free_bytes": 2_000_000_000,
                "cells": {name: {"kind": "query" if name in QUERY_PATHS else "lifecycle",
                                 "expected_checks": expected[name]} for name in args.cells}}
    (output / "protocol.json").write_bytes(canonical_bytes(protocol))
    before = inventory()
    env.update({"SLICE135_E12_RAW_PATH": str(output / "workload-raw.json"),
                "SLICE135_E12_SAMPLES": str(args.samples),
                "SLICE135_E12_CELLS": ",".join(args.cells),
                "SLICE135_E12_SOURCE_SHA": source_sha,
                "SLICE135_E12_PROTOCOL_SHA256": digest(canonical_bytes(protocol)),
                "SLICE135_E12_RUNNER_SHA256": file_hash(bundle),
                "SLICE135_E12_BINARY_SHA256": file_hash(binary)})
    command = ["/usr/bin/time", "-v", str(binary), "--ignored", "--exact",
               "slice135_e12_measurement", "--nocapture"]
    (output / "command.json").write_text(json.dumps(command, indent=2) + "\n")
    run = _run(command, env=env, timeout=3600)
    (output / "run.stdout.log").write_text(run.stdout)
    (output / "run.stderr.log").write_text(run.stderr)
    after = inventory()
    invalidators = []
    if run.returncode:
        invalidators.append(f"workload exit {run.returncode}")
    for key in ("host", "kernel", "cpu", "storage", "governor", "toolchain"):
        if before[key] != after[key]:
            invalidators.append(f"{key} drift")
    if before["competing_jobs"] or after["competing_jobs"]:
        invalidators.append("competing heavy job")
    if before["disk_free_bytes"] < 2_000_000_000 or after["disk_free_bytes"] < 2_000_000_000:
        invalidators.append("disk pressure")
    if not (output / "workload-raw.json").exists():
        invalidators.append("workload raw absent")
        raw = {"cells": {}}
    else:
        raw = json.loads((output / "workload-raw.json").read_text())
        for key, expected in (("source_sha", source_sha),
                              ("runner_sha256", file_hash(bundle)),
                              ("protocol_sha256", digest(canonical_bytes(protocol))),
                              ("binary_sha256", file_hash(binary)),
                              ("corpus_sha256", file_hash(corpus))):
            if raw.get(key) != expected:
                invalidators.append(f"workload {key} mismatch")
    raw.update({"schema_version": 1, "source_sha": source_sha,
                "runner_sha256": file_hash(bundle), "protocol_sha256": digest(canonical_bytes(protocol)),
                "artifact_sha256": protocol["artifact_sha256"], "settings": protocol["settings"],
                "environment": {"start": before, "end": after, "invalidators": invalidators},
                "resources": parse_resources(run.stderr)})
    (output / "raw.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
    if invalidators:
        raise ValueError("; ".join(invalidators))
    summary = validate_measurement(raw, protocol, output)
    summary["raw_sha256"] = file_hash(output / "raw.json")
    summary["validator_sha256"] = file_hash(Path(__file__))
    (output / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"source_sha": source_sha, "cells": {key: value["valid_samples"]
                        for key, value in summary["cells"].items()}, "output": str(output)}))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"INVALID_ATTEMPT: {error}", file=sys.stderr)
        raise SystemExit(1) from error
