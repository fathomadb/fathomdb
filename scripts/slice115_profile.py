#!/usr/bin/env python3
"""Sample exact-workload CPU stacks with GDB/MI when perf cannot run."""

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
    "open_fresh": ("open_with_embedder", "open_managed_connection", "open_runtime_connection"),
    "open_populated": ("open_with_embedder", "open_managed_connection", "open_runtime_connection"),
    "close": ("Engine>::close", "Engine::close", "runtime_lifecycle"),
    "canonical_write": ("Engine>::write", "Engine::write", "write_commit"),
    "text": ("search_text_only", "search_api", "search_fts"),
    "graph_expand": ("Engine>::graph_expand", "graph_expand::execution"),
    "graph_evidence": ("resolve_graph_evidence", "resolve_graph_evidence_v1"),
    "erasure": ("erase_source", "erasure::"),
}


def operation_frame(path: str, stack: str) -> bool:
    """Classify an interrupt as landing inside the timed engine operation."""
    if path not in FRAMES:
        raise ValueError(f"unsupported profile path {path}")
    return any(frame in stack for frame in FRAMES[path])


def completed_operations(path: str, output: str) -> int:
    """Read the workload's matching natural-completion count."""
    match = re.search(rf"PROFILE_DONE {re.escape(path)} operations=(\d+)", output)
    if not match:
        raise ValueError(f"{path}: matching completion count missing")
    return int(match.group(1))


def _command(process: subprocess.Popen, token: int, command: str) -> None:
    assert process.stdin is not None
    process.stdin.write(f"{token}{command}\n")
    process.stdin.flush()


def _read_until(process: subprocess.Popen, predicate, transcript: list[str], timeout: float) -> list[str]:
    assert process.stdout is not None
    pending = getattr(process, "_slice115_pending", [])
    partial = getattr(process, "_slice115_partial", "")
    deadline = time.monotonic() + timeout
    lines = []
    while time.monotonic() < deadline:
        if pending:
            line = pending.pop(0)
            transcript.append(line)
            lines.append(line)
            if predicate(line):
                process._slice115_pending = pending
                process._slice115_partial = partial
                return lines
            continue
        if not select.select([process.stdout.fileno()], [], [], min(0.2, max(0.0, deadline - time.monotonic())))[0]:
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


def sample(binary: Path, path: str, output: Path, seconds: int, count: int,
           control: bool = False) -> dict:
    """Collect interrupt-and-backtrace samples from the workload path."""
    output.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.update({"SLICE115_PROFILE_PATH": path, "SLICE115_PROFILE_SECONDS": str(seconds),
                "FATHOMDB_EMBED_DEVICE": "cpu"})
    argv = ["gdb", "-q", "--interpreter=mi2", "-iex", "set debuginfod enabled off",
            "--args", str(binary), "--ignored", "--exact", "slice115_profile_loop", "--nocapture"]
    control_operations = None
    if control:
        unprofiled = subprocess.run(
            [str(binary), "--ignored", "--exact", "slice115_profile_loop", "--nocapture"],
            text=True, capture_output=True, env=env, timeout=seconds + 30)
        control_log = unprofiled.stdout + unprofiled.stderr
        output.with_suffix(".control.log").write_text(control_log)
        if unprofiled.returncode:
            raise ValueError(f"{path}: unprofiled control exit {unprofiled.returncode}")
        control_operations = completed_operations(path, control_log)
    transcript = []
    stacks = []
    process = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True, bufsize=1, env=env)
    try:
        _read_until(process, lambda line: line.strip() == "(gdb)", transcript, 20)
        _command(process, 1, "-gdb-set mi-async on")
        _read_until(process, lambda line: line.startswith("1^done"), transcript, 20)
        _command(process, 2, "-exec-run")
        _read_until(process, lambda line: "PROFILE_READY" in line, transcript, 30)
        for index in range(count):
            time.sleep(0.08)
            token = 10 + 3 * index
            _command(process, token, "-exec-interrupt")
            _read_until(process, lambda line: line.startswith("*stopped"), transcript, 20)
            _command(process, token + 1,
                     '-interpreter-exec console "thread apply all bt 16"')
            stack = _read_until(process, lambda line: line.startswith(f"{token + 1}^done"), transcript, 20)
            stacks.append("".join(stack))
            _command(process, token + 2, "-exec-continue")
            _read_until(process, lambda line: line.startswith(f"{token + 2}^running"), transcript, 20)
        _read_until(process, lambda line: line.startswith('*stopped,reason="exited-normally"'),
                    transcript, seconds + 30)
    finally:
        output.write_text("".join(transcript))
        if process.poll() is None:
            process.kill()
        process.wait(timeout=10)
    operation_samples = sum(operation_frame(path, stack) for stack in stacks)
    profiled_operations = completed_operations(path, "".join(transcript))
    metadata = {"path": path, "method": "gdb-interrupt-stack",
                "gdb_version": subprocess.check_output(["gdb", "--version"], text=True).splitlines()[0],
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "requested_samples": count, "total_samples": len(stacks),
                "operation_samples": operation_samples,
                "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
                "sampling_interval_seconds": 0.08,
                "profile_loop_seconds": seconds,
                "control_operations": control_operations,
                "profiled_operations": profiled_operations,
                "control_vs_profile_throughput_ratio": (
                    profiled_operations / control_operations if control_operations else None)}
    if operation_samples < 3:
        raise ValueError(f"{path}: only {operation_samples} operation stack samples; profile retained")
    return metadata


def main() -> None:
    """Run one path profiler and retain its stack and metadata files."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--path", choices=tuple(FRAMES), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=15)
    parser.add_argument("--samples", type=int, default=16)
    parser.add_argument("--control", action="store_true")
    args = parser.parse_args()
    metadata = sample(args.binary, args.path, args.output, args.seconds, args.samples, args.control)
    args.output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps({"path": args.path, "operation_samples": metadata["operation_samples"],
                      "total_samples": metadata["total_samples"]}))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"PROFILE_INVALID: {error}", file=sys.stderr)
        raise SystemExit(1) from error
