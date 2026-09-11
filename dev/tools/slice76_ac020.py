#!/usr/bin/env python3
"""Seal, run, and summarize the bounded Slice 76 AC-020 campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import signal
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


MARKER = re.compile(
    r"^AC020_NUMBERS sequential_ms=(\d+) concurrent_ms=(\d+) bound_ms=(\d+)$",
    re.MULTILINE,
)
RUNNING = re.compile(r"^running (\d+) test(?:s)?$", re.MULTILINE)
RESULT = re.compile(
    r"^test result: (?:ok|FAILED)\. (\d+) passed(?:; (\d+) failed(?:; (\d+) ignored)?)?",
    re.MULTILINE,
)
FORBIDDEN_ENV = {
    "FATHOMDB_PERF_EXPERIMENTS",
    "FATHOMDB_PERF_SQLITE_MEMSTATUS_OFF",
    "FATHOMDB_PERF_SQLITE_PAGECACHE",
    "FATHOMDB_PERF_SQLITE_PCACHE2",
    "FATHOMDB_PERF_SEARCH_LIMIT",
    "FATHOMDB_PERF_READER_PRAGMAS",
    "FATHOMDB_PERF_WRITER_PRAGMAS",
    "LIBSQLITE3_FLAGS",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_run_log(text: str, exit_code: int) -> dict[str, Any]:
    running = RUNNING.findall(text)
    if not running or running == ["0"]:
        raise ValueError("expected exactly one executed test")
    markers = MARKER.findall(text)
    if len(markers) != 1:
        raise ValueError("expected exactly one AC020_NUMBERS marker")
    results = RESULT.findall(text)
    if running != ["1"] or len(results) != 1:
        raise ValueError("expected exactly one executed test")
    passed, failed, ignored = (int(value or 0) for value in results[0])
    if passed + failed != 1 or ignored != 0:
        raise ValueError("expected exactly one executed test with no ignore")
    sequential, concurrent, bound = (int(value) for value in markers[0])
    return {
        "sequential_ms": sequential,
        "concurrent_ms": concurrent,
        "bound_ms": bound,
        "speedup": sequential / concurrent,
        "assertion_passed": exit_code == 0 and passed == 1 and failed == 0,
    }


def validate_observations(
    observations: list[dict[str, Any]], configuration: str, binary_sha256: str
) -> None:
    labels: set[str] = set()
    for observation in observations:
        label = observation.get("label")
        if not isinstance(label, str) or not label:
            raise ValueError("malformed observation label")
        if label in labels:
            raise ValueError(f"duplicate observation label: {label}")
        labels.add(label)
        if observation.get("configuration") != configuration:
            raise ValueError(f"wrong configuration for {label}")
        if observation.get("binary_sha256") != binary_sha256:
            raise ValueError(f"wrong binary identity for {label}")


def validate_campaign(
    observations: list[dict[str, Any]],
    expected_configurations: list[str],
    builds: dict[str, dict[str, Any]],
    expected_labels: list[str] | None = None,
) -> None:
    actual_configurations = [str(item.get("configuration")) for item in observations]
    if actual_configurations != expected_configurations:
        raise ValueError(
            f"wrong configuration order: expected {expected_configurations}, "
            f"got {actual_configurations}"
        )
    if expected_labels is not None:
        actual_labels = [str(item.get("label")) for item in observations]
        if actual_labels != expected_labels:
            raise ValueError(f"wrong label order: expected {expected_labels}, got {actual_labels}")
    if len({item.get("label") for item in observations}) != len(observations):
        raise ValueError("duplicate observation label")
    for observation in observations:
        configuration = str(observation["configuration"])
        expected_sha = builds.get(configuration, {}).get("binary_sha256")
        if observation.get("binary_sha256") != expected_sha:
            raise ValueError(f"wrong binary identity for {observation.get('label')}")


def nearest_rank(values: list[int], percentile: float) -> int:
    ordered = sorted(values)
    rank = max(1, math.ceil(percentile * len(ordered)))
    return ordered[rank - 1]


def distribution(values: list[int]) -> dict[str, int]:
    ordered = sorted(values)
    return {
        "min": ordered[0],
        "q1": nearest_rank(ordered, 0.25),
        "median": nearest_rank(ordered, 0.5),
        "q3": nearest_rank(ordered, 0.75),
        "max": ordered[-1],
        "iqr": nearest_rank(ordered, 0.75) - nearest_rank(ordered, 0.25),
    }


def summarize(observations: list[dict[str, Any]]) -> dict[str, Any]:
    if not observations:
        raise ValueError("cannot summarize zero observations")
    sequential = [int(item["sequential_ms"]) for item in observations]
    concurrent = [int(item["concurrent_ms"]) for item in observations]
    sequential_summary = distribution(sequential)
    concurrent_summary = distribution(concurrent)
    speedups = [s / c for s, c in zip(sequential, concurrent, strict=True)]
    return {
        "labels": [str(item["label"]) for item in observations],
        "sequential_ms": sequential_summary,
        "concurrent_ms": concurrent_summary,
        "ratio_of_medians": sequential_summary["median"] / concurrent_summary["median"],
        "median_of_ratios": sorted(speedups)[len(speedups) // 2],
        "passed": sum(bool(item.get("assertion_passed")) for item in observations),
        "total": len(observations),
    }


def seal_build(
    target_dir: Path, destination: Path, configuration: str, source_sha: str
) -> dict[str, Any]:
    candidates = [
        path
        for path in (target_dir / "release" / "deps").glob("perf_gates-*")
        if path.is_file() and os.access(path, os.X_OK) and path.suffix != ".d"
    ]
    if len(candidates) != 1:
        raise ValueError(f"expected exactly one perf_gates binary, found {len(candidates)}")
    if destination.exists():
        raise ValueError(f"sealed binary already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(candidates[0], destination)
    return {
        "configuration": configuration,
        "source_sha": source_sha,
        "binary_path": str(destination),
        "binary_sha256": sha256(destination),
        "source_binary_path": str(candidates[0]),
    }


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def cmd_seal_build(args: argparse.Namespace) -> int:
    source_sha = args.source_sha
    if source_sha is None:
        source_sha = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    manifest_path = Path(args.runtime_manifest)
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    if args.configuration in manifest.get("builds", {}):
        raise ValueError(f"sealed build record already exists: {args.configuration}")
    record = seal_build(
        Path(args.target_dir), Path(args.binary_out), args.configuration, source_sha
    )
    paths = manifest.get("relevant_tree", {}).get("paths")
    if not isinstance(paths, list) or not all(isinstance(path, str) for path in paths):
        raise ValueError("runtime manifest lacks relevant-tree paths")
    record["relevant_tree_sha256"] = git_tree_sha(source_sha, paths)
    manifest.setdefault("builds", {})[args.configuration] = record
    write_json(manifest_path, manifest)
    print(json.dumps(record, sort_keys=True))
    return 0


def clean_environment() -> dict[str, str]:
    environment = os.environ.copy()
    for name in FORBIDDEN_ENV:
        environment.pop(name, None)
    return environment


def profile_environment(
    arm: str, ready: Path, go: Path, done: Path, finish: Path
) -> dict[str, str]:
    if arm not in {"sequential", "concurrent-after-sequential-warmup"}:
        raise ValueError(f"unsupported profile arm: {arm}")
    environment = clean_environment()
    environment["FATHOMDB_SLICE76_PROFILE_ARM"] = arm
    environment["FATHOMDB_SLICE76_PROFILE_READY"] = str(ready)
    environment["FATHOMDB_SLICE76_PROFILE_GO"] = str(go)
    environment["FATHOMDB_SLICE76_PROFILE_DONE"] = str(done)
    environment["FATHOMDB_SLICE76_PROFILE_FINISH"] = str(finish)
    return environment


def git_tree_sha(source_sha: str, paths: list[str]) -> str:
    listing = subprocess.run(
        ["git", "ls-tree", "-r", source_sha, "--", *paths],
        check=True,
        capture_output=True,
    ).stdout
    return hashlib.sha256(listing).hexdigest()


def manifest_build(manifest_path: Path, configuration: str, binary: Path) -> dict[str, Any]:
    manifest = json.loads(manifest_path.read_text())
    try:
        build = manifest["builds"][configuration]
    except (KeyError, TypeError) as error:
        raise ValueError(f"missing sealed build for {configuration}") from error
    if Path(build["binary_path"]) != binary:
        raise ValueError(f"wrong binary path for {configuration}")
    actual_sha = sha256(binary)
    if build.get("binary_sha256") != actual_sha:
        raise ValueError(
            f"wrong binary identity: expected {build.get('binary_sha256')}, got {actual_sha}"
        )
    paths = manifest.get("relevant_tree", {}).get("paths")
    if not isinstance(paths, list) or not all(isinstance(path, str) for path in paths):
        raise ValueError("runtime manifest lacks relevant-tree paths")
    actual_tree = git_tree_sha(str(build["source_sha"]), paths)
    if build.get("relevant_tree_sha256") != actual_tree:
        raise ValueError(f"wrong relevant-tree identity for {configuration}")
    return build


def wait_for_path(path: Path, process: subprocess.Popen[str], timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise ValueError(
                f"profile harness exited before ready ({process.returncode}):\n{stdout}{stderr}"
            )
        time.sleep(0.01)
    process.terminate()
    process.wait(timeout=5)
    raise ValueError("profile harness did not become ready")


def cmd_profile(args: argparse.Namespace) -> int:
    binary = Path(args.binary)
    build = manifest_build(Path(args.runtime_manifest), args.configuration, binary)
    output = Path(args.output)
    if output.exists():
        raise ValueError(f"profile already exists: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    child: subprocess.Popen[str] | None = None
    profile: subprocess.Popen[str] | None = None
    with tempfile.TemporaryDirectory(prefix="fathomdb-slice76-profile-") as temp:
        root = Path(temp)
        ready = root / "ready"
        go = root / "go"
        done = root / "done"
        finish = root / "finish"
        environment = profile_environment(args.arm, ready, go, done, finish)
        command = [
            str(binary),
            "slice76_profile_gate_phase",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ]
        try:
            child = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=environment,
            )
            wait_for_path(ready, child, 300)
            expected_warmup = 1600 if args.arm == "concurrent-after-sequential-warmup" else 0
            ready_record = json.loads(ready.read_text())
            if ready_record != {"arm": args.arm, "warmup_searches": expected_warmup}:
                raise ValueError(f"wrong profile warmup record: {ready_record}")
            profile = subprocess.Popen(
                [
                    "perf",
                    "record",
                    "--call-graph",
                    "dwarf",
                    "--output",
                    str(output),
                    "--pid",
                    str(child.pid),
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env=clean_environment(),
            )
            time.sleep(0.25)
            if profile.poll() is not None:
                profile_stdout, profile_stderr = profile.communicate()
                raise ValueError(
                    f"perf exited before the search phase ({profile.returncode}):\n"
                    f"{profile_stdout}{profile_stderr}"
                )
            go.touch()
            wait_for_path(done, child, 300)
            done_record = json.loads(done.read_text())
            if done_record != {"arm": args.arm, "searches": 1600}:
                raise ValueError(f"wrong profile completion record: {done_record}")
            profile.send_signal(signal.SIGINT)
            profile_stdout, profile_stderr = profile.communicate(timeout=30)
            finish.touch()
            child_stdout, child_stderr = child.communicate(timeout=30)
        finally:
            for process in (profile, child):
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)
    log = child_stdout + child_stderr
    begin = f"SLICE76_PROFILE_BEGIN arm={args.arm}"
    end = f"SLICE76_PROFILE_END arm={args.arm}"
    if child.returncode != 0 or log.count(begin) != 1 or log.count(end) != 1:
        raise ValueError(
            f"profile harness failed or did not delimit one search phase ({child.returncode}):\n{log}"
        )
    if profile.returncode != 0:
        raise ValueError(
            f"perf failed ({profile.returncode}):\n{profile_stdout}{profile_stderr}"
        )
    record = {
        "configuration": args.configuration,
        "source_sha": build["source_sha"],
        "binary_sha256": build["binary_sha256"],
        "arm": args.arm,
        "profile_path": str(output),
        "profile_sha256": sha256(output),
        "scope": "on_cpu_search_phase_only",
    }
    print(json.dumps(record, sort_keys=True))
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    build = manifest_build(
        Path(args.runtime_manifest), args.configuration, Path(args.binary)
    )
    binary = Path(build["binary_path"])
    actual_sha = str(build["binary_sha256"])
    observations_path = Path(args.observations)
    existing = []
    if observations_path.exists():
        existing = [json.loads(line) for line in observations_path.read_text().splitlines() if line]
    if any(item.get("label") == args.label for item in existing):
        raise ValueError(f"duplicate observation label: {args.label}")
    log_path = Path(args.log)
    if log_path.exists():
        raise ValueError(f"observation log already exists: {log_path}")
    environment = clean_environment()
    environment["AGENT_LONG"] = "1"
    command = [
        str(binary),
        "ac_020_reads_do_not_serialize_on_a_single_reader_connection",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]
    started = datetime.now(UTC).isoformat()
    completed = subprocess.run(command, capture_output=True, text=True, env=environment, check=False)
    ended = datetime.now(UTC).isoformat()
    text = completed.stdout + completed.stderr
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(text)
    parsed = parse_run_log(text, completed.returncode)
    observation = {
        **parsed,
        "schema_version": "fathomdb.slice76-ac020-observation/v1",
        "label": args.label,
        "configuration": args.configuration,
        "source_sha": build["source_sha"],
        "binary_sha256": actual_sha,
        "log_path": str(log_path),
        "log_sha256": sha256(log_path),
        "exit_code": completed.returncode,
        "started_at": started,
        "ended_at": ended,
    }
    observations_path.parent.mkdir(parents=True, exist_ok=True)
    with observations_path.open("a") as stream:
        stream.write(json.dumps(observation, sort_keys=True) + "\n")
    print(json.dumps(observation, sort_keys=True))
    return 0


def cmd_summary(args: argparse.Namespace) -> int:
    manifest = json.loads(Path(args.runtime_manifest).read_text())
    observations = [
        json.loads(line) for line in Path(args.observations).read_text().splitlines() if line
    ]
    expected_configurations = manifest["run_order"][args.campaign]
    expected_labels = manifest["run_labels"][args.campaign]
    validate_campaign(
        observations,
        expected_configurations,
        manifest["builds"],
        expected_labels,
    )
    grouped: dict[str, list[dict[str, Any]]] = {}
    for observation in observations:
        grouped.setdefault(str(observation["configuration"]), []).append(observation)
    result = {configuration: summarize(items) for configuration, items in grouped.items()}
    write_json(Path(args.output), result)
    print(json.dumps(result, sort_keys=True))
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    commands = root.add_subparsers(dest="command", required=True)
    seal = commands.add_parser("seal-build")
    seal.add_argument("--configuration", required=True)
    seal.add_argument("--target-dir", required=True)
    seal.add_argument("--binary-out", required=True)
    seal.add_argument("--runtime-manifest", required=True)
    seal.add_argument("--source-sha")
    seal.set_defaults(handler=cmd_seal_build)

    run = commands.add_parser("run")
    run.add_argument("--configuration", required=True)
    run.add_argument("--label", required=True)
    run.add_argument("--binary", required=True)
    run.add_argument("--runtime-manifest", required=True)
    run.add_argument("--observations", required=True)
    run.add_argument("--log", required=True)
    run.set_defaults(handler=cmd_run)

    summary = commands.add_parser("summary")
    summary.add_argument("--observations", required=True)
    summary.add_argument("--output", required=True)
    summary.add_argument("--runtime-manifest", required=True)
    summary.add_argument("--campaign", choices=["initial_baseline", "matched"], required=True)
    summary.set_defaults(handler=cmd_summary)

    profile = commands.add_parser("profile")
    profile.add_argument("--configuration", required=True)
    profile.add_argument("--runtime-manifest", required=True)
    profile.add_argument("--binary", required=True)
    profile.add_argument("--arm", required=True)
    profile.add_argument("--output", required=True)
    profile.set_defaults(handler=cmd_profile)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        return int(args.handler(args))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"slice76-ac020: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
