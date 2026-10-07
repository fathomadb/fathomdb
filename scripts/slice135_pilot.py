#!/usr/bin/env python3
"""Build and run real-engine Slice 135 pilot cells with bound raw evidence."""

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
import tomllib


ROOT = Path(__file__).resolve().parents[1]
WORKLOAD = ROOT / "scripts/slice135_pilot_workload.rs"
VALIDATOR = ROOT / "scripts/slice135_receipt.py"
CELLS = ("text", "close_fresh", "mixed_sequence")
MIXED_STAGES = (
    "open", "governed_write", "projection_drain", "text_before_erase",
    "erasure", "close", "reopen", "text_after_erase", "reclose",
)
STABLE_FIELDS = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler")
GNU_TIME = Path("/usr/bin/time")
RESOURCE_FORMAT = (
    "user_s=%U\nsystem_s=%S\npeak_rss_kib=%M\nfs_inputs=%I\nfs_outputs=%O\n"
)
SOURCE_PATHS = ("Cargo.toml", "Cargo.lock", "src/rust", "third_party")
REQUIRED_SOURCE_PATHS = SOURCE_PATHS[:3]
CORPUS = "".join(
    f"doc-{index}:needle memory document {index} for bounded search\n"
    for index in range(32)
).encode()


def operation_manifest(samples: int) -> dict:
    """Declare the exact operation counts used by this bounded Pareto proxy."""
    return {
        "schema_version": 1,
        "scope": "real-engine pilot proxy; not installed SDK or full S02",
        "cells": {
            "text": {"observations": samples,
                     "timed_operations_per_observation": {"text_search": 1}},
            "close_fresh": {"observations": samples,
                            "timed_operations_per_observation": {"close": 1}},
            "mixed_sequence": {
                "observations": samples,
                "boundary": "fresh-open-through-reopened-materialized-check",
                "stages": list(MIXED_STAGES),
                "counts_per_observation": {stage: 1 for stage in MIXED_STAGES},
                "note": "Stage timing is a separate attribution run; primary mode measures only the whole sequence.",
            },
        },
    }


def validate_build_mode(timing_mode: str, profile_build: bool) -> None:
    """Keep a symbolized profiler build out of the primary latency campaign."""
    if profile_build and timing_mode != "attribution":
        raise ValueError("profile build requires attribution timing mode")


def aggregate_attribution(raw: dict, manifest: dict) -> dict:
    """Rank stage cost only from the separately labeled attribution receipt."""
    if raw.get("features", {}).get("timing_mode") != "attribution":
        raise ValueError("attribution requires a separate attribution-mode receipt")
    specification = manifest["cells"]["mixed_sequence"]
    stages = specification["stages"]
    attempts = raw["cells"]["mixed_sequence"]["attempts"]
    if len(attempts) != specification["observations"]:
        raise ValueError("attribution attempt count differs from manifest")
    costs = {stage: 0 for stage in stages}
    total = 0
    unattributed = 0
    for attempt in attempts:
        elapsed = attempt.get("latency_ns")
        timings = attempt.get("stages_ns")
        if (attempt.get("valid") is not True or not isinstance(elapsed, int)
                or elapsed <= 0 or not isinstance(timings, dict)
                or set(timings) != set(stages)
                or any(not isinstance(value, int) or isinstance(value, bool) or value <= 0
                       for value in timings.values())):
            raise ValueError("attribution stage receipt missing or invalid")
        attributed = sum(timings.values())
        if attributed > elapsed:
            raise ValueError("attribution stage sum exceeds whole sequence")
        total += elapsed
        unattributed += elapsed - attributed
        for stage in stages:
            costs[stage] += timings[stage]
    ranked = sorted(stages, key=lambda stage: (-costs[stage], stage))
    denominator = sum(costs.values())
    threshold = 0.8 * denominator
    cumulative = 0
    pareto = []
    for stage in ranked:
        pareto.append(stage)
        cumulative += costs[stage]
        if cumulative >= threshold:
            break
    return {
        "scope": "mixed-sequence stage-attributed proxy only",
        "stage_cost_ns": costs,
        "stage_invocations": {stage: len(attempts) * specification["counts_per_observation"][stage]
                              for stage in stages},
        "ranked_stages": ranked,
        "pareto_stages_80": pareto,
        "total_sequence_ns": total,
        "unattributed_ns": unattributed,
        "stage_attributed_fraction": denominator / total,
    }


def sha256(data: bytes) -> str:
    """Return a lowercase SHA-256 digest."""
    return hashlib.sha256(data).hexdigest()


def source_tree_hash(checkout: Path) -> str:
    """Fingerprint measured engine source and its dependency lock on disk."""
    digest = hashlib.sha256()
    for relative in SOURCE_PATHS:
        path = checkout / relative
        files = sorted(path.rglob("*")) if path.is_dir() else [path]
        for file in files:
            if file.is_file() and ".git" not in file.parts:
                name = str(file.relative_to(checkout)).encode()
                data = file.read_bytes()
                digest.update(len(name).to_bytes(8, "big") + name)
                digest.update(len(data).to_bytes(8, "big") + data)
    return digest.hexdigest()


def run(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    """Run a command while retaining its full output for the caller."""
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=False)


def verify_source(checkout: Path, source_sha: str) -> dict:
    """Bind an exact Git source to the checked-out product bytes."""
    if len(source_sha) != 40 or any(char not in "0123456789abcdef" for char in source_sha):
        raise ValueError("source SHA must be forty lowercase hex digits")
    if any(not (checkout / relative).exists() for relative in REQUIRED_SOURCE_PATHS):
        raise ValueError("measured Rust source or Cargo manifest is absent")
    resolved = run("git", "rev-parse", f"{source_sha}^{{commit}}", cwd=checkout)
    if resolved.returncode or resolved.stdout.strip() != source_sha:
        raise ValueError("source SHA does not resolve to the requested commit")
    diff = run("git", "diff", "--quiet", source_sha, "--", *SOURCE_PATHS, cwd=checkout)
    if diff.returncode:
        raise ValueError("measured source or Cargo.lock differs from source SHA")
    untracked = run("git", "ls-files", "--others", "--exclude-standard", "--", *SOURCE_PATHS,
                    cwd=checkout)
    if untracked.returncode or untracked.stdout.strip():
        raise ValueError("untracked measured source exists")
    return {"source_sha": source_sha, "source_tree_sha256": source_tree_hash(checkout),
            "cargo_lock_sha256": sha256((checkout / "Cargo.lock").read_bytes())}


def parse_attempts(stream: str, cells: tuple[str, ...] = CELLS) -> tuple[dict, list[str]]:
    """Keep every emitted attempt and report malformed lines without discarding them."""
    attempts = {cell: [] for cell in cells}
    errors = []
    for number, line in enumerate(stream.splitlines(), 1):
        prefix = "SLICE135_ATTEMPT "
        if prefix not in line:
            continue
        try:
            record = json.loads(line.split(prefix, 1)[1])
            if not isinstance(record, dict):
                raise ValueError("attempt is not an object")
            cell = record.pop("cell")
            if cell not in attempts:
                raise ValueError(f"unknown cell {cell}")
            attempts[cell].append(record)
        except (KeyError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"stdout line {number}: {error}")
    return attempts, errors


def environment_invalidators(start: dict, end: dict, min_disk_free_bytes: int) -> list[str]:
    """Explain runtime changes that invalidate an uninstrumented timing block."""
    invalid = []
    if any(not start.get(field) or start.get(field) != end.get(field)
           for field in STABLE_FIELDS):
        invalid.append("environment identity drift or missing field")
    if start["governor"] == "unavailable":
        invalid.append("CPU governor unavailable")
    if start["swap_pages"] != end["swap_pages"]:
        invalid.append("swap activity")
    if start["competing_jobs"] or end["competing_jobs"]:
        invalid.append("competing jobs")
    if min(start["disk_free_bytes"], end["disk_free_bytes"]) < min_disk_free_bytes:
        invalid.append("disk pressure")
    return invalid


def inventory(output: Path) -> dict:
    """Record the host and resource state around the measured block."""
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    vmstat = Path("/proc/vmstat").read_text().splitlines()
    swap = sum(int(line.split()[1]) for line in vmstat
               if line.startswith(("pswpin ", "pswpout ")))
    jobs = []
    for line in run("ps", "-eo", "pid=,comm=").stdout.splitlines():
        pid, name = line.split(maxsplit=1)
        if int(pid) != os.getpid() and name in {
            "cargo", "rustc", "pytest", "maturin", "vllm", "ollama",
        }:
            jobs.append({"pid": int(pid), "name": name})
    cpu = next((line.split(":", 1)[1].strip() for line in
                Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.processor() or "unknown")
    return {
        "time_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "host": platform.node(), "kernel": platform.release(), "cpu": cpu,
        "storage": str(output.stat().st_dev),
        "governor": governor.read_text().strip() if governor.exists() else "unavailable",
        "toolchain": run("rustc", "--version").stdout.strip(), "profiler": "none",
        "resource_tool": run(str(GNU_TIME), "--version").stdout.splitlines()[0]
        if GNU_TIME.is_file() else "unsupported",
        "swap_pages": swap, "competing_jobs": jobs,
        "disk_free_bytes": shutil.disk_usage(output).free,
    }


def read_resource_report(path: Path) -> dict:
    """Read GNU time's child-only CPU, peak RSS and filesystem-block counters."""
    report = {
        "scope": "measured-workload-child-process",
        "method": "gnu-time" if path.is_file() else "unsupported",
        "user_cpu_s": None, "system_cpu_s": None, "peak_rss_kib": None,
        "fs_inputs": None, "fs_outputs": None,
    }
    if not path.is_file():
        report["unsupported"] = ["user_cpu_s", "system_cpu_s", "peak_rss_kib",
                                 "fs_inputs", "fs_outputs"]
        return report
    values = dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    try:
        report.update({
            "user_cpu_s": float(values["user_s"]),
            "system_cpu_s": float(values["system_s"]),
            "peak_rss_kib": int(values["peak_rss_kib"]),
            "fs_inputs": int(values["fs_inputs"]),
            "fs_outputs": int(values["fs_outputs"]),
        })
        report["unsupported"] = []
    except (KeyError, ValueError):
        report["unsupported"] = ["malformed GNU time report"]
    return report


def bundle_bytes() -> bytes:
    """Bind Python orchestration, Rust workload, and receipt validator together."""
    parts = (Path(__file__).read_bytes(), WORKLOAD.read_bytes(), VALIDATOR.read_bytes())
    return b"".join(len(part).to_bytes(8, "big") + part for part in parts)


def manifest(checkout: Path, output: Path) -> Path:
    """Create a standalone release-profile test crate against this exact checkout."""
    build = output / "build"
    build.mkdir()
    engine = checkout / "src/rust/crates/fathomdb-engine"
    path = build / "Cargo.toml"
    root_manifest = tomllib.loads((checkout / "Cargo.toml").read_text())
    patches = []
    for name, spec in root_manifest.get("patch", {}).get("crates-io", {}).items():
        bound = {key: str((checkout / value).resolve()) if key == "path" else value
                 for key, value in spec.items()}
        properties = ", ".join(f"{key} = {json.dumps(value)}" for key, value in bound.items())
        patches.append(f"{name} = {{ {properties} }}")
    path.write_text(
        '[package]\nname = "slice135-pilot"\nversion = "0.1.0"\nedition = "2021"\n'
        '[dependencies]\n'
        f'fathomdb-engine = {{ path = "{engine}", features = ["test-hooks"] }}\n'
        'serde_json = "1"\ntempfile = "3"\n'
        '[[test]]\nname = "slice135_pilot_workload"\n'
        f'path = "{WORKLOAD}"\n'
        '[patch.crates-io]\n' + '\n'.join(patches) + '\n'
    )
    shutil.copyfile(checkout / "Cargo.lock", build / "Cargo.lock")
    return path


def verify_resolved_lock(source_lock: Path, resolved_lock: Path) -> str:
    """Allow pruning unused packages, but reject a changed resolved dependency."""
    source = tomllib.loads(source_lock.read_text())
    resolved = tomllib.loads(resolved_lock.read_text())
    source_packages = {(item["name"], item["version"]): item for item in source["package"]}
    for item in resolved["package"]:
        if item["name"] == "slice135-pilot":
            continue
        original = source_packages.get((item["name"], item["version"]))
        if (original is None or original.get("checksum") != item.get("checksum")
                or original.get("source") != item.get("source")):
            raise ValueError(f"resolved dependency differs from source lock: {item['name']}")
    return sha256(resolved_lock.read_bytes())


def build_binary(checkout: Path, output: Path, profile_build: bool = False) -> Path:
    """Build the measured release-profile executable and retain compiler logs."""
    path = manifest(checkout, output)
    command = ["cargo", "test", "--offline", "--release", "--no-run",
               "--message-format=json", "--manifest-path", str(path),
               "--target-dir", str(output / "target")]
    build_env = {**os.environ}
    if profile_build:
        build_env["CARGO_PROFILE_RELEASE_DEBUG"] = "1"
        build_env["CARGO_PROFILE_RELEASE_STRIP"] = "none"
    (output / "build-command.json").write_text(json.dumps({
        "command": command,
        "profile_build": profile_build,
        "profile_overrides": {name: build_env[name] for name in
                              ("CARGO_PROFILE_RELEASE_DEBUG", "CARGO_PROFILE_RELEASE_STRIP")}
        if profile_build else {},
    }, indent=2) + "\n")
    result = subprocess.run(command, capture_output=True, text=True, check=False, env=build_env)
    (output / "build.stdout.log").write_text(result.stdout)
    (output / "build.stderr.log").write_text(result.stderr)
    if result.returncode:
        raise ValueError(f"release workload build failed with exit {result.returncode}")
    verify_resolved_lock(checkout / "Cargo.lock", output / "build/Cargo.lock")
    binaries = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (message.get("target", {}).get("name") == "slice135_pilot_workload"
                and message.get("executable")):
            binaries.append(Path(message["executable"]))
    if len(binaries) != 1:
        raise ValueError(f"expected one workload binary, found {len(binaries)}")
    binary = output / "slice135_pilot_workload"
    shutil.copyfile(binaries[0], binary)
    binary.chmod(0o755)
    return binary


def reuse_binary(previous: Path, output: Path, source: dict, runner_hash: str,
                 binary_hash: str, profile_build: bool = False) -> Path:
    """Copy only a verified same-source binary and lock into a new pilot block."""
    prior_binary = previous / "slice135_pilot_workload"
    prior_lock = previous / "build/Cargo.lock"
    prior_runner = previous / "runner.bundle"
    prior_source = json.loads((previous / "source.json").read_text())
    prior_protocol = json.loads((previous / "pilot-protocol.json").read_text())
    prior_status = json.loads((previous / "attempt.json").read_text()).get("status")
    if prior_status not in {"SMOKE_ONLY", "VALID_BLOCK_RECEIPT"}:
        raise ValueError("prior build did not yield a usable workload run")
    artifacts = prior_protocol.get("artifact_sha256", {})
    actual_binary_hash = sha256(prior_binary.read_bytes())
    if (len(binary_hash) != 64 or actual_binary_hash != binary_hash
            or artifacts.get("slice135_pilot_workload") != binary_hash):
        raise ValueError("binary hash differs from caller or prior protocol")
    if (sha256(prior_runner.read_bytes()) != runner_hash
            or prior_protocol.get("runner_sha256") != runner_hash):
        raise ValueError("runner differs from prior build")
    if prior_source != source or prior_protocol.get("source_sha") != source["source_sha"]:
        raise ValueError("source differs from prior build")
    if prior_protocol.get("features", {}).get("profile_build", False) is not profile_build:
        raise ValueError("profile build mode differs from prior binary")
    if sha256(prior_lock.read_bytes()) != artifacts.get("build/Cargo.lock"):
        raise ValueError("resolved lock differs from prior build")
    (output / "build").mkdir()
    shutil.copyfile(prior_lock, output / "build/Cargo.lock")
    shutil.copyfile(prior_binary, output / "slice135_pilot_workload")
    binary = output / "slice135_pilot_workload"
    binary.chmod(0o755)
    (output / "build-command.json").write_text(json.dumps({
        "mode": "verified_reuse", "previous": str(previous),
        "binary_sha256": binary_hash,
    }, indent=2) + "\n")
    return binary


def pilot(checkout: Path, source_sha: str, output: Path, samples: int,
          reuse_from: Path | None = None, binary_sha256: str | None = None,
          timing_mode: str = "primary", profile_build: bool = False) -> None:
    """Run a baseline or candidate pilot and retain raw, logs and verdict."""
    if samples < 1:
        raise ValueError("samples must be positive")
    if timing_mode not in {"primary", "attribution"}:
        raise ValueError("timing mode must be primary or attribution")
    validate_build_mode(timing_mode, profile_build)
    output.mkdir(parents=True, exist_ok=False)
    attempt_path = output / "attempt.json"
    try:
        source = verify_source(checkout, source_sha)
        (output / "source.json").write_text(json.dumps(source, indent=2) + "\n")
        (output / "corpus.txt").write_bytes(CORPUS)
        (output / "operation-manifest.json").write_text(
            json.dumps(operation_manifest(samples), indent=2, sort_keys=True) + "\n"
        )
        (output / "runner.bundle").write_bytes(bundle_bytes())
        if reuse_from is None:
            binary = build_binary(checkout, output, profile_build)
        else:
            if binary_sha256 is None:
                raise ValueError("binary hash required for reuse")
            binary = reuse_binary(reuse_from, output, source,
                                  sha256((output / "runner.bundle").read_bytes()),
                                  binary_sha256, profile_build)
            verify_resolved_lock(checkout / "Cargo.lock", output / "build/Cargo.lock")
        artifacts = {name: sha256((output / name).read_bytes()) for name in
                     ("slice135_pilot_workload", "corpus.txt", "build/Cargo.lock",
                      "operation-manifest.json")}
        protocol = {
            "schema_version": 1, "source_sha": source_sha,
            "runner_sha256": sha256((output / "runner.bundle").read_bytes()),
            "artifact_sha256": artifacts,
            "features": {"query": "text_only", "embedder": "none",
                         "timing_mode": timing_mode,
                         "profile_build": profile_build,
                         "resource_method": "gnu-time" if GNU_TIME.is_file() else "unsupported"},
            "settings": {"samples_per_cell": samples, "query": "needle",
                         "seed_records": 32, "concurrency": 1, "warmup_per_cell": 1},
            "min_disk_free_bytes": 1_073_741_824,
            "cells": {
                "text": {"kind": "query", "boundary": "engine-call-to-materialized-output",
                         "expected_checks": {"record_count": 10, "all_bodies_match": True}},
                "close_fresh": {"kind": "lifecycle", "boundary": "close-call-to-return",
                                "expected_checks": {"reopen_ok": True,
                                                    "canonical_rows": [0],
                                                    "second_close_ok": True}},
                "mixed_sequence": {
                    "kind": "lifecycle",
                    "boundary": "fresh-open-through-reopened-materialized-check",
                    "expected_checks": {
                        "fresh_rows": [0], "written_rows": [1], "pre_erase_hits": 1,
                        "pre_erase_body_match": True, "post_erase_rows": [0],
                        "reopened_rows": [0], "post_reopen_hits": 0,
                    },
                },
            },
        }
        protocol_path = output / "pilot-protocol.json"
        protocol_path.write_text(json.dumps(protocol, indent=2, sort_keys=True) + "\n")
        start = inventory(output)
        measured_command = [str(binary), "--ignored", "--exact", "slice135_pilot", "--nocapture"]
        resource_path = output / "resource.txt"
        command = ([str(GNU_TIME), "-o", str(resource_path), "-f", RESOURCE_FORMAT]
                   + measured_command if GNU_TIME.is_file() else measured_command)
        (output / "run-command.json").write_text(json.dumps({
            "wrapper": command, "measured_binary": measured_command,
            "timing_mode": timing_mode,
        }, indent=2) + "\n")
        result = subprocess.run(command, capture_output=True, text=True,
                                env={**os.environ, "SLICE135_COUNT": str(samples),
                                     "SLICE135_TIMING_MODE": timing_mode,
                                     "FATHOMDB_EMBED_DEVICE": "cpu"}, check=False)
        (output / "run.stdout.log").write_text(result.stdout)
        (output / "run.stderr.log").write_text(result.stderr)
        end = inventory(output)
        attempts, parse_errors = parse_attempts(result.stdout)
        resources = read_resource_report(resource_path)
        invalid = environment_invalidators(start, end, protocol["min_disk_free_bytes"])
        invalid.extend(parse_errors)
        if GNU_TIME.is_file() and resources["unsupported"]:
            invalid.append("child resource report incomplete")
        if result.returncode:
            invalid.append(f"workload exit {result.returncode}")
        for cell in CELLS:
            if len(attempts[cell]) != samples:
                invalid.append(f"{cell}: expected {samples} attempts, got {len(attempts[cell])}")
        raw = {
            "schema_version": 1, "source_sha": source_sha,
            "source_tree_sha256": source["source_tree_sha256"],
            "runner_sha256": protocol["runner_sha256"],
            "protocol_sha256": sha256(protocol_path.read_bytes()),
            "artifact_sha256": artifacts, "features": protocol["features"],
            "settings": protocol["settings"],
            "resources": resources,
            "resource_file_sha256": sha256(resource_path.read_bytes())
            if resource_path.is_file() else None,
            "environment": {"start": start, "end": end, "invalidators": invalid},
            "cells": {cell: {"boundary": protocol["cells"][cell]["boundary"],
                             "attempts": attempts[cell]} for cell in CELLS},
        }
        raw_path = output / "raw.json"
        raw_path.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        if invalid:
            raise ValueError("; ".join(invalid))
        if timing_mode == "attribution":
            attribution = aggregate_attribution(raw, operation_manifest(samples))
            (output / "attribution.json").write_text(
                json.dumps(attribution, indent=2, sort_keys=True) + "\n"
            )
        if samples < 100:
            attempt_path.write_text(json.dumps({"status": "SMOKE_ONLY",
                                                "reason": "fewer than 100 valid samples"},
                                               indent=2) + "\n")
            return
        summary = output / "summary.json"
        validation = run(sys.executable, str(VALIDATOR), "--raw", str(raw_path),
                         "--protocol", str(protocol_path), "--runner",
                         str(output / "runner.bundle"), "--artifacts-root", str(output),
                         "--source-sha", source_sha, "--output", str(summary))
        (output / "validation.stdout.log").write_text(validation.stdout)
        (output / "validation.stderr.log").write_text(validation.stderr)
        if validation.returncode:
            raise ValueError(f"receipt validation failed with exit {validation.returncode}")
        attempt_path.write_text(json.dumps({"status": "VALID_BLOCK_RECEIPT",
                                            "raw_sha256": sha256(raw_path.read_bytes())},
                                           indent=2) + "\n")
    except (OSError, ValueError) as error:
        attempt_path.write_text(json.dumps({"status": "INVALID_ATTEMPT", "reason": str(error)},
                                           indent=2) + "\n")
        raise


def main() -> None:
    """Run the named exact-source pilot without replacing an earlier attempt."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--reuse-binary-from", type=Path)
    parser.add_argument("--binary-sha256")
    parser.add_argument("--timing-mode", choices=("primary", "attribution"), default="primary")
    parser.add_argument("--profile-build", action="store_true")
    args = parser.parse_args()
    if (args.reuse_binary_from is None) != (args.binary_sha256 is None):
        parser.error("--reuse-binary-from and --binary-sha256 must be supplied together")
    pilot(args.checkout.resolve(), args.source_sha, args.output.resolve(), args.samples,
          args.reuse_binary_from.resolve() if args.reuse_binary_from else None,
          args.binary_sha256, args.timing_mode, args.profile_build)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError) as error:
        print(f"INVALID_ATTEMPT: {error}", file=sys.stderr)
        raise SystemExit(1) from error
