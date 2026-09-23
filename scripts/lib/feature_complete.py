#!/usr/bin/env python3
"""The feature-complete test gate behind `scripts/test-feature-complete.sh`.

Runs every workspace test target the workspace gate (`cargo test
--workspace`) cannot, once per required feature set from the committed
`scripts/test-feature-matrix.toml`, with CUDA on the two RTX 3090s and real
model weights. A test that skips itself or is `#[ignore]`d fails the gate
unless `scripts/test-skip-allowlist.toml` names it with a class and reason; an
allowlist entry that matches nothing is stale and also fails the gate.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from typing import Callable, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))
import test_targets  # noqa: E402


REPO_ROOT = test_targets.REPO_ROOT
CUDA_ROOT = Path("/usr/local/cuda")
REQUIRED_GPU = "RTX 3090"
SKIP_MARKERS = ("[SKIP]", "[skip]", "PENDING_EXTERNAL", "skipping")
MARKER_CLASSES = frozenset({"ignored-by-design", "opt-in-experiment", "benign-message"})
_RUNNING = re.compile(r"^\s*Running (\S+) \(")
_TEST_LINE = re.compile(r"^test (\S+) \.\.\. ?(.*)$")
_RESULT = re.compile(r"^test result: (\S+)\.")


class FeatureCompleteError(Exception):
    """A preflight or gate failure."""


class ScanResult:
    """Skip markers, ignored tests, and failures seen in one cargo run."""

    __slots__ = ("markers", "ignored", "failed", "ran_targets")

    def __init__(self) -> None:
        self.markers: list[tuple[str, str]] = []
        self.ignored: list[str] = []
        self.failed: list[str] = []
        self.ran_targets: list[str] = []


def scan_output(text: str, crate: test_targets.Crate) -> ScanResult:
    """Attribute every marker to the test whose `test <name> ...` line precedes
    it (the gate runs one test thread per binary)."""

    by_source = {target.source: target for target in crate.targets}
    result = ScanResult()
    target_id: str | None = None
    current: str | None = None
    for line in text.splitlines():
        running = _RUNNING.match(line)
        if running is not None:
            source = running.group(1)
            target = by_source.get(source)
            target_id = target.id if target is not None else f"{crate.name}::{source}"
            result.ran_targets.append(target_id)
            current = None
            continue
        rest = line
        test_line = _TEST_LINE.match(line)
        if test_line is not None and target_id is not None:
            current = f"{target_id}::{test_line.group(1)}"
            rest = test_line.group(2)
            status = rest.strip()
            if status == "ignored" or status.startswith("ignored,"):
                result.ignored.append(current)
                continue
            if status == "FAILED":
                result.failed.append(current)
                continue
        elif line.strip() == "FAILED" and current is not None:
            result.failed.append(current)
            continue
        summary = _RESULT.match(line)
        if summary is not None:
            if summary.group(1) != "ok":
                result.failed.append(f"{target_id}: test result {summary.group(1)}")
            current = None
            continue
        for marker in SKIP_MARKERS:
            if marker in rest:
                owner = current or target_id or crate.name
                result.markers.append((owner, marker))
                break
    return result


def _matches(entry_id: str, test_id: str) -> bool:
    return test_id == entry_id or test_id.startswith(entry_id + "::")


def contract_failures(
    results: Sequence[ScanResult], allowlist: Sequence[dict[str, str]]
) -> list[str]:
    """Apply the skip contract: unexcused markers and ignored tests, failed
    tests, and stale allowlist entries each fail the gate."""

    entries = [e for e in allowlist if e.get("class") != "platform-excluded"]
    used: set[int] = set()
    failures: list[str] = []
    for result in results:
        failures.extend(f"test failed: {name}" for name in result.failed)
        for test_id, marker in result.markers:
            hits = [n for n, e in enumerate(entries) if _matches(e["id"], test_id)]
            used.update(hits)
            if not any(entries[n]["class"] in MARKER_CLASSES for n in hits):
                failures.append(f"skip marker {marker!r} from {test_id} is not allowlisted")
        for test_id in result.ignored:
            hits = [n for n, e in enumerate(entries) if _matches(e["id"], test_id)]
            used.update(hits)
            excused = any(
                entries[n]["class"] == "ignored-by-design"
                or (entries[n]["id"] != test_id and entries[n]["class"] == "opt-in-experiment")
                for n in hits
            )
            if not excused:
                failures.append(f"ignored test {test_id} is not allowlisted as ignored-by-design")
    for n, entry in enumerate(entries):
        if n not in used:
            failures.append(
                f"stale allowlist entry {entry['id']!r} ({entry['class']}): matched no skip or ignored test"
            )
    return failures


def cuda_environment(
    base: dict[str, str],
    cuda_root: Path,
    run: Callable[[list[str], dict[str, str]], str],
) -> dict[str, str]:
    """Return `base` with the CUDA toolchain on the path and the two RTX 3090s
    selected by PCI bus order; fail if either is not an RTX 3090."""

    nvcc = cuda_root / "bin" / "nvcc"
    if not os.access(nvcc, os.X_OK):
        raise FeatureCompleteError(f"nvcc not found at {nvcc}")
    env = dict(base)
    env["PATH"] = f"{cuda_root / 'bin'}:{base.get('PATH', '')}".rstrip(":")
    env["CUDA_ROOT"] = str(cuda_root)
    env["CUDA_PATH"] = str(cuda_root)
    libraries = [str(cuda_root / "lib64"), str(cuda_root / "lib64" / "stubs")]
    if base.get("LIBRARY_PATH"):
        libraries.append(base["LIBRARY_PATH"])
    env["LIBRARY_PATH"] = ":".join(libraries)
    env["CUDA_DEVICE_ORDER"] = "PCI_BUS_ID"
    env["CUDA_VISIBLE_DEVICES"] = "0,1"
    listing = run(
        ["nvidia-smi", "--query-gpu=index,pci.bus_id,name", "--format=csv,noheader"], env
    )
    names = {}
    for line in listing.splitlines():
        fields = [field.strip() for field in line.split(",")]
        if len(fields) >= 3:
            names[fields[0]] = fields[2]
    for index in ("0", "1"):
        if REQUIRED_GPU not in names.get(index, ""):
            raise FeatureCompleteError(
                f"CUDA device {index} in PCI bus order is {names.get(index)!r}, not an {REQUIRED_GPU}"
            )
    return env


def runner_environment(base: dict[str, str], scratch: Path) -> dict[str, str]:
    env = {k: v for k, v in base.items() if k != "FATHOMDB_SKIP_NETWORK_TESTS"}
    env["FATHOMDB_SLICE72_RUNNER"] = "approved-nvidia"
    env["FATHOMDB_SLICE72_RECEIPT_DIR"] = str(scratch / "slice72-receipts")
    return env


def test_command(package: str, features: Sequence[str], targets: Sequence[str]) -> list[str]:
    command = ["cargo", "test", "--locked", "-p", package, "--no-default-features"]
    if features:
        command += ["--features", ",".join(features)]
    for target in targets:
        command += ["--test", target]
    return command + ["--", "--nocapture", "--test-threads=1"]


def _nvidia_smi(command: list[str], env: dict[str, str]) -> str:
    return subprocess.run(command, env=env, check=True, capture_output=True, text=True).stdout


def _warm_embedder(env: dict[str, str], log: Path) -> None:
    build = ["cargo", "build", "--locked", "-p", "fathomdb-cli", "--features", "default-embedder"]
    subprocess.run(build, cwd=REPO_ROOT, env=env, check=True)
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
            cwd=REPO_ROOT,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    target_dir = Path(metadata["target_directory"])
    binaries = [
        target["name"]
        for package in metadata["packages"]
        if package["name"] == "fathomdb-cli"
        for target in package["targets"]
        if "bin" in target["kind"]
    ]
    if len(binaries) != 1:
        raise FeatureCompleteError(f"expected one fathomdb-cli binary, found {binaries}")
    completed = subprocess.run(
        [str(target_dir / "debug" / binaries[0]), "doctor", "warm-cache"],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    log.write_text(completed.stdout + completed.stderr)
    if completed.returncode != 0:
        raise FeatureCompleteError(f"doctor warm-cache failed ({completed.returncode}); see {log}")


def run_gate(scratch: Path) -> int:
    host = test_targets.host_triple()
    crates = test_targets.read_workspace(REPO_ROOT)
    matrix = test_targets.load_matrix(test_targets.MATRIX_PATH)
    allowlist = test_targets.load_allowlist(test_targets.ALLOWLIST_PATH)
    workspace = test_targets.workspace_features(REPO_ROOT, host)
    failures, _ = test_targets.check_coverage(crates, workspace, matrix, allowlist, host)
    if failures:
        for failure in failures:
            print(f"FAIL coverage: {failure}", file=sys.stderr)
        return 1
    env = cuda_environment(dict(os.environ), CUDA_ROOT, _nvidia_smi)
    env = runner_environment(env, scratch)
    Path(env["FATHOMDB_SLICE72_RECEIPT_DIR"]).mkdir(parents=True, exist_ok=True)
    _warm_embedder(env, scratch / "warm-cache.log")
    by_crate = {crate.name: crate for crate in crates}
    results: list[ScanResult] = []
    summary: list[dict[str, object]] = []
    plan = test_targets.gate_plan(crates, workspace, matrix, allowlist, host)
    for package, features, targets in plan:
        command = test_command(package, features, targets)
        label = f"{package}[{','.join(features) or 'none'}]"
        log = scratch / f"{package}--{'+'.join(features) or 'none'}.log"
        print(f"==> {label}: {len(targets)} target(s)", flush=True)
        started = time.monotonic()
        with open(log, "w", encoding="utf-8") as handle:
            completed = subprocess.run(
                command, cwd=REPO_ROOT, env=env, stdout=handle, stderr=subprocess.STDOUT, check=False
            )
        seconds = round(time.monotonic() - started)
        result = scan_output(log.read_text(encoding="utf-8", errors="replace"), by_crate[package])
        if completed.returncode != 0:
            result.failed.append(f"{label}: cargo test exited {completed.returncode} (see {log})")
        expected = {f"{package}::{target}" for target in targets}
        missing = sorted(expected - set(result.ran_targets))
        result.failed.extend(f"{target}: did not run" for target in missing)
        results.append(result)
        summary.append(
            {
                "set": label,
                "targets": targets,
                "exit": completed.returncode,
                "seconds": seconds,
                "markers": result.markers,
                "ignored": result.ignored,
                "log": str(log),
            }
        )
        print(f"    exit {completed.returncode} in {seconds}s; log {log}", flush=True)
    failures = contract_failures(results, allowlist)
    (scratch / "summary.json").write_text(json.dumps({"plan": summary, "failures": failures}, indent=2))
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(
        f"feature-complete: {len(plan)} feature set(s), "
        f"{sum(len(t) for _, _, t in plan)} target(s), {len(failures)} failure(s); "
        f"summary {scratch / 'summary.json'}"
    )
    return 1 if failures else 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scratch", type=Path, help="log directory (default: a new temporary one)")
    args = parser.parse_args(argv)
    scratch = args.scratch or Path(tempfile.mkdtemp(prefix="fathomdb-feature-complete-"))
    scratch.mkdir(parents=True, exist_ok=True)
    try:
        return run_gate(scratch)
    except (FeatureCompleteError, test_targets.TestTargetsError, subprocess.CalledProcessError) as exc:
        print(f"feature-complete: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
