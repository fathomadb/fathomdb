#!/usr/bin/env python3
"""Sample a bound Slice 135 real-engine hot path outside latency measurement."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import select
import subprocess
import sys
import time


FRAMES = {
    "text": ("search_text_only", "search_fts", "search_api"),
    "mixed_sequence": (
        "open_without_embedder", "open_with", "open_runtime_connection",
        "::write", "::drain", "search_text_only", "search_fts",
        "erase_source", "::close",
    ),
}


def sha256(data: bytes) -> str:
    """Hash the exact artifact bytes."""
    return hashlib.sha256(data).hexdigest()


def bound_binary(receipt_dir: Path, source_sha: str) -> Path:
    """Refuse a changed source identity or measured binary."""
    protocol = json.loads((receipt_dir / "pilot-protocol.json").read_text())
    binary = receipt_dir / "slice135_pilot_workload"
    if protocol.get("source_sha") != source_sha:
        raise ValueError("source SHA differs from receipt")
    expected = protocol.get("artifact_sha256", {}).get("slice135_pilot_workload")
    if expected is None or sha256(binary.read_bytes()) != expected:
        raise ValueError("binary differs from receipt")
    return binary


def operation_frame(path: str, stack: str) -> bool:
    """Count a stack only when it includes a relevant engine operation frame."""
    if path not in FRAMES:
        raise ValueError(f"unsupported profile path {path}")
    frames = stack.replace("\\n", "\n").splitlines()
    return any("fathomdb_engine::" in frame
               and any(fragment in frame for fragment in FRAMES[path])
               for frame in frames)


def completed_operations(path: str, output: str) -> int:
    """Read the matching natural-completion count from the workload."""
    match = re.search(rf"PROFILE_DONE {re.escape(path)} operations=(\d+)", output)
    if not match:
        raise ValueError(f"{path}: matching completion count missing")
    return int(match.group(1))


def command(process: subprocess.Popen, token: int, value: str) -> None:
    """Send one numbered GDB/MI command."""
    assert process.stdin is not None
    process.stdin.write(f"{token}{value}\n")
    process.stdin.flush()


def read_until(process: subprocess.Popen, predicate, transcript: list[str],
               timeout: float) -> list[str]:
    """Read complete GDB/MI lines until the expected response arrives."""
    assert process.stdout is not None
    pending = getattr(process, "_slice135_pending", [])
    partial = getattr(process, "_slice135_partial", "")
    deadline = time.monotonic() + timeout
    lines = []
    while time.monotonic() < deadline:
        if pending:
            line = pending.pop(0)
            transcript.append(line)
            lines.append(line)
            if predicate(line):
                process._slice135_pending = pending
                process._slice135_partial = partial
                return lines
            continue
        wait = min(0.2, max(0.0, deadline - time.monotonic()))
        if not select.select([process.stdout.fileno()], [], [], wait)[0]:
            if process.poll() is not None:
                raise ValueError("GDB exited before completing a sample")
            continue
        chunk = os.read(process.stdout.fileno(), 65536)
        if not chunk:
            raise ValueError("GDB closed output before completing a sample")
        partial += chunk.decode("utf-8", errors="replace")
        parts = partial.split("\n")
        partial = parts.pop()
        pending.extend(part + "\n" for part in parts)
    raise ValueError("GDB sample timed out")


def sample(receipt_dir: Path, source_sha: str, path: str, output: Path,
           seconds: int, count: int, control: bool) -> dict:
    """Capture bounded GDB stack samples and an optional unprofiled control."""
    if not (1 <= seconds <= 60) or not (3 <= count <= 100):
        raise ValueError("profile seconds must be 1..60 and samples 3..100")
    binary = bound_binary(receipt_dir, source_sha)
    protocol = json.loads((receipt_dir / "pilot-protocol.json").read_text())
    if protocol.get("features", {}).get("profile_build") is not True:
        raise ValueError("GDB sampling requires a separate symbolized profile build")
    output.parent.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "SLICE135_PROFILE_PATH": path,
           "SLICE135_PROFILE_SECONDS": str(seconds), "FATHOMDB_EMBED_DEVICE": "cpu"}
    workload = [str(binary), "--ignored", "--exact", "slice135_profile_loop", "--nocapture"]
    control_operations = None
    if control:
        result = subprocess.run(workload, text=True, capture_output=True,
                                env=env, timeout=seconds + 30, check=False)
        control_log = result.stdout + result.stderr
        output.with_suffix(".control.log").write_text(control_log)
        if result.returncode:
            raise ValueError(f"{path}: unprofiled control exit {result.returncode}")
        control_operations = completed_operations(path, control_log)
    argv = ["gdb", "-q", "--interpreter=mi2", "-iex", "set debuginfod enabled off",
            "--args", *workload]
    transcript = []
    stacks = []
    process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True, bufsize=1, env=env)
    try:
        read_until(process, lambda line: line.strip() == "(gdb)", transcript, 20)
        command(process, 1, "-gdb-set mi-async on")
        read_until(process, lambda line: line.startswith("1^done"), transcript, 20)
        command(process, 2, "-exec-run")
        startup = read_until(process, lambda line: "PROFILE_READY" in line
                             or line.startswith("2^error"), transcript, 30)
        if any(line.startswith("2^error") for line in startup):
            raise ValueError("GDB could not start the workload; inspect retained transcript")
        for index in range(count):
            time.sleep(0.08)
            token = 10 + 3 * index
            command(process, token, "-exec-interrupt")
            read_until(process, lambda line: line.startswith("*stopped"), transcript, 20)
            command(process, token + 1, '-interpreter-exec console "thread apply all bt 32"')
            stack = read_until(process, lambda line: line.startswith(f"{token + 1}^done"),
                               transcript, 20)
            stacks.append("".join(stack))
            command(process, token + 2, "-exec-continue")
            read_until(process, lambda line: line.startswith(f"{token + 2}^running"),
                       transcript, 20)
        read_until(process, lambda line: line.startswith('*stopped,reason="exited-normally"'),
                   transcript, seconds + 30)
    finally:
        output.write_text("".join(transcript))
        if process.poll() is None:
            process.kill()
        process.wait(timeout=10)
    operation_samples = sum(operation_frame(path, stack) for stack in stacks)
    profiled_operations = completed_operations(path, "".join(transcript))
    metadata = {
        "source_sha": source_sha, "path": path, "method": "gdb-interrupt-stack",
        "purpose": "hot-path attribution only; not a latency receipt",
        "gdb_version": subprocess.check_output(["gdb", "--version"], text=True).splitlines()[0],
        "binary_sha256": sha256(binary.read_bytes()),
        "runner_sha256": protocol["runner_sha256"],
        "protocol_sha256": sha256((receipt_dir / "pilot-protocol.json").read_bytes()),
        "profiler_sha256": sha256(Path(__file__).read_bytes()),
        "requested_samples": count, "total_samples": len(stacks),
        "operation_samples": operation_samples,
        "transcript_sha256": sha256(output.read_bytes()),
        "profile_loop_seconds": seconds,
        "control_operations": control_operations,
        "profiled_operations": profiled_operations,
        "control_vs_profile_throughput_ratio": (
            profiled_operations / control_operations if control_operations else None),
    }
    if operation_samples < 3:
        raise ValueError(f"{path}: only {operation_samples} engine-frame samples; transcript retained")
    return metadata


def main() -> None:
    """Run an isolated sampled profile of an existing bound workload binary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt-dir", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--path", choices=tuple(FRAMES), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=15)
    parser.add_argument("--samples", type=int, default=16)
    parser.add_argument("--control", action="store_true")
    args = parser.parse_args()
    metadata = sample(args.receipt_dir, args.source_sha, args.path, args.output,
                      args.seconds, args.samples, args.control)
    args.output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps({"path": args.path, "operation_samples": metadata["operation_samples"],
                      "total_samples": metadata["total_samples"]}))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"PROFILE_INVALID: {error}", file=sys.stderr)
        raise SystemExit(1) from error
