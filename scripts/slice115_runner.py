#!/usr/bin/env python3
"""Build and run the frozen Slice 115 engine cells against one source candidate."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
SLICE = ROOT / "dev/plans/0.8.27/features/slice-115"
PROTOCOL = SLICE / "protocol.json"
WORKLOAD = ROOT / "scripts/slice115_workload.rs"
VALIDATOR = ROOT / "scripts/slice115_receipt.py"
PROFILER = ROOT / "scripts/slice115_profile.py"
MODEL = Path.home() / ".cache/huggingface/hub/models--BAAI--bge-small-en-v1.5/snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"


def sha256(path: Path) -> str:
    """Hash the bytes actually present on disk."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require_lock(expected: str) -> None:
    """Reject a dependency lock that differs from the frozen protocol."""
    actual = sha256(ROOT / "Cargo.lock")
    if actual != expected:
        raise ValueError(f"Cargo.lock hash mismatch: expected {expected}, got {actual}")


def bundle_bytes() -> bytes:
    """Bind the builder, measured workload, and validator as one runner."""
    return b"".join(len(data).to_bytes(8, "big") + data for data in
                    (Path(__file__).read_bytes(), WORKLOAD.read_bytes(),
                     VALIDATOR.read_bytes(), PROFILER.read_bytes()))


def corpus_bytes() -> bytes:
    """Serialize the exact seeded row bodies and graph facts in source order."""
    rows = [f"doc-{index}:needle memory document {index} for bounded search"
            for index in range(32)]
    rows.extend(("graph-source:canonical graph source bytes", "graph-root:root claim",
                 "graph-target:target claim", "graph-edge:graph-root:supports:graph-target"))
    return ("\n".join(rows) + "\n").encode()


def command(argv: list[str], **kwargs) -> subprocess.CompletedProcess:
    """Run a required command and preserve its exit status."""
    return subprocess.run(argv, check=True, text=True, **kwargs)


def inventory() -> dict:
    """Capture environment fields needed to reject mismatched measurements."""
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    swaps = Path("/proc/vmstat").read_text().splitlines()
    swap = {line.split()[0]: int(line.split()[1]) for line in swaps
            if line.startswith(("pswpin ", "pswpout "))}
    competitors = []
    for line in command(["ps", "-eo", "pid=,comm="], capture_output=True).stdout.splitlines():
        pid, name = line.split(maxsplit=1)
        if int(pid) != os.getpid() and name in {"cargo", "rustc", "pytest", "maturin", "vllm", "ollama"}:
            competitors.append({"pid": int(pid), "name": name})
    return {
        "time_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "host": platform.node(), "kernel": platform.release(),
        "cpu": next((line.split(":", 1)[1].strip() for line in
                     Path("/proc/cpuinfo").read_text().splitlines()
                     if line.startswith("model name")), platform.processor()),
        "logical_cpus": os.cpu_count(),
        "memory_total_kib": next((int(line.split()[1]) for line in
                                  Path("/proc/meminfo").read_text().splitlines()
                                  if line.startswith("MemTotal:")), None),
        "rustc": command(["rustc", "--version"], capture_output=True).stdout.strip(),
        "governor": governor.read_text().strip() if governor.exists() else None,
        "swap_pages": swap,
        "competing_jobs": competitors,
        "storage": command(["df", "-P", "/tmp"], capture_output=True).stdout.splitlines()[-1].split()[0],
        "perf_event_paranoid": Path("/proc/sys/kernel/perf_event_paranoid").read_text().strip(),
    }


def manifest(directory: Path) -> Path:
    """Create a private Cargo test crate using the exact checkout dependencies."""
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "Cargo.toml"
    engine = ROOT / "src/rust/crates/fathomdb-engine"
    embedder_api = ROOT / "src/rust/crates/fathomdb-embedder-api"
    path.write_text(
        '[package]\nname = "slice115-measurement"\nversion = "0.1.0"\nedition = "2021"\n'
        '[dependencies]\n'
        f'fathomdb-engine = {{ path = "{engine}", features = ["test-hooks", "default-embedder"] }}\n'
        f'fathomdb-embedder-api = {{ path = "{embedder_api}" }}\n'
        'serde_json = "1"\nsha2 = "0.11"\ntempfile = "3"\n'
        '[[test]]\nname = "slice115_workload"\n'
        f'path = "{WORKLOAD}"\n'
    )
    shutil.copyfile(ROOT / "Cargo.lock", directory / "Cargo.lock")
    return path


def build(path: Path, output: Path) -> Path:
    """Build the release-profile workload and copy the exact binary to output."""
    result = subprocess.run(["cargo", "test", "--offline", "--release", "--no-run",
                             "--message-format=json", "--manifest-path", str(path)],
                            text=True, capture_output=True)
    if result.returncode:
        print(result.stderr, file=sys.stderr, end="")
        raise ValueError(f"workload compilation failed with exit {result.returncode}")
    binaries = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("target", {}).get("name") == "slice115_workload" and message.get("executable"):
            binaries.append(Path(message["executable"]))
    if len(binaries) != 1:
        raise ValueError(f"expected one workload binary, found {len(binaries)}")
    binary = output / "slice115_workload"
    binary.write_bytes(binaries[0].read_bytes())
    binary.chmod(0o755)
    return binary


def main() -> None:
    """Retain raw and invalid attempts, then validate the exact candidate."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    protocol = json.loads(PROTOCOL.read_text())
    source_sha = protocol["engine_source_sha"]
    require_lock(protocol["cargo_lock_sha256"])
    diff = command(["git", "diff", "--name-only", source_sha, "--", "src/rust/crates"],
                   cwd=ROOT, capture_output=True).stdout.strip()
    dirty = command(["git", "status", "--porcelain", "--", "src/rust/crates"],
                    cwd=ROOT, capture_output=True).stdout.strip()
    if diff or dirty:
        raise ValueError(f"engine source differs from frozen {source_sha}: {diff} {dirty}")
    assets = {file: sha256(MODEL / file) for file in
              ("config.json", "model.safetensors", "tokenizer.json")}
    if assets != protocol["model_assets_sha256"]:
        raise ValueError("pinned model asset hash mismatch")
    (output / "model-assets.json").write_text(json.dumps(assets, indent=2) + "\n")
    runner_bundle = output / "runner.bundle"
    runner_bundle.write_bytes(bundle_bytes())
    (output / "protocol.json").write_bytes(PROTOCOL.read_bytes())
    (output / "corpus.txt").write_bytes(corpus_bytes())
    start = inventory()
    (output / "environment-start.json").write_text(json.dumps(start, indent=2) + "\n")
    binary = build(manifest(output / "build"), output)
    before = inventory()
    env = os.environ.copy()
    env.update({
        "SLICE115_RAW_PATH": str(output / "raw.json"),
        "SLICE115_SOURCE_SHA": source_sha,
        "SLICE115_PROTOCOL_SHA256": sha256(PROTOCOL),
        "SLICE115_RUNNER_SHA256": sha256(runner_bundle),
        "SLICE115_BINARY_SHA256": sha256(binary),
        "FATHOMDB_EMBED_DEVICE": "cpu",
    })
    command_log = {"build": "cargo test --release --no-run --message-format=json",
                   "run": [str(binary), "--ignored", "--exact", "slice115_measurement", "--nocapture"]}
    (output / "commands.json").write_text(json.dumps(command_log, indent=2) + "\n")
    result = subprocess.run(command_log["run"], text=True, capture_output=True, env=env)
    (output / "run.stdout.log").write_text(result.stdout)
    (output / "run.stderr.log").write_text(result.stderr)
    after = inventory()
    (output / "environment-end.json").write_text(json.dumps(after, indent=2) + "\n")
    invalid = []
    for key in ("governor", "storage"):
        if before[key] != after[key]:
            invalid.append(f"{key} changed")
    if any(after["swap_pages"][key] != before["swap_pages"][key]
           for key in before["swap_pages"]):
        invalid.append("swap activity")
    if before["competing_jobs"] or after["competing_jobs"]:
        invalid.append("competing heavy job")
    if result.returncode != 0:
        invalid.append(f"workload exit {result.returncode}")
    if not (output / "raw.json").exists():
        invalid.append("raw receipt absent")
    else:
        raw = json.loads((output / "raw.json").read_text())
        if raw["corpus_sha256"] != sha256(output / "corpus.txt"):
            invalid.append("corpus hash mismatch")
        if invalid:
            (output / "raw-original.json").write_bytes((output / "raw.json").read_bytes())
            raw["environment_valid"] = False
            (output / "raw.json").write_text(json.dumps(raw, indent=2) + "\n")
    (output / "attempt.json").write_text(json.dumps({
        "status": "INVALID" if invalid else "RAW_CAPTURED", "reasons": invalid,
        "binary_sha256": sha256(binary), "runner_sha256": sha256(runner_bundle),
        "protocol_sha256": sha256(PROTOCOL), "model_assets": assets,
    }, indent=2) + "\n")
    if invalid:
        raise ValueError("; ".join(invalid))
    print(f"Raw samples retained at {output / 'raw.json'}; sampled profiles and summary remain required")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"INVALID_ATTEMPT: {error}", file=sys.stderr)
        raise SystemExit(1) from error
