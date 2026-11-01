#!/usr/bin/env python3
"""Build paired scan probes and evaluate the predeclared Slice 50 drain rules."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tarfile
import threading
import time

REPO = Path(__file__).resolve().parents[6]
PROBE = Path("src/rust/crates/fathomdb-engine/tests/slice50_drain_scaling.rs")


def parse_probe(output: str) -> dict:
    """Return the single probe JSON record; reject missing or duplicate records."""
    records = [
        json.loads(line.split("SLICE50_DRAIN ", 1)[1])
        for line in output.splitlines()
        if "SLICE50_DRAIN " in line
    ]
    if len(records) != 1:
        raise ValueError(f"expected one probe record, got {len(records)}")
    return records[0]


def analyze(rows: list[dict]) -> dict:
    """Apply the frozen medians, paired improvement and logical-store rules."""
    rows = list(
        {(r["label"], r["n"], r["repeat"], r["mode"]): r for r in rows}.values()
    )
    cells = {}
    for label in ["base", "head"]:
        for n in [10000, 30000, 100000]:
            selected = [
                r
                for r in rows
                if r["label"] == label and r["n"] == n and r["mode"] == "held"
            ]
            by_repeat = {r["repeat"]: r for r in selected}
            if set(by_repeat) != {0, 1}:
                raise ValueError(f"missing repeat in {label}/{n}")
            cells[label, n] = [by_repeat[i] for i in [0, 1]]
    medians = {
        key: statistics.median(r["drain_ms"] * 1000 / r["n"] for r in cell)
        for key, cell in cells.items()
    }
    ratio = medians["head", 100000] / medians["head", 10000]
    regression = medians["head", 10000] / medians["base", 10000]
    identical = all(
        len({r["digest"] for label in ["base", "head"] for r in cells[label, n]}) == 1
        for n in [10000, 30000, 100000]
    )
    paired = [
        cells["head", 100000][i]["drain_ms"] / cells["base", 100000][i]["drain_ms"]
        for i in [0, 1]
    ]
    return dict(
        linear=ratio <= 1.3,
        no_regression_10k=regression <= 1.05,
        identical=identical,
        head_100k_to_10k=ratio,
        head_to_base_10k=regression,
        head_to_base_100k=paired,
        improvement_reported=all(v <= 0.8 for v in paired),
        base_100k_to_10k=medians["base", 100000] / medians["base", 10000],
        median_us_per_vector={
            f"{label}/{n}": value for (label, n), value in medians.items()
        },
        load_flagged=[
            f"{r['label']}/{r['n']}/{r['repeat']}"
            for r in rows
            if r.get("load_peak", 0) > 2.5
        ],
    )


def build(args: argparse.Namespace) -> None:
    """Export source and build using one serialized shared target directory."""
    args.scratch.mkdir(parents=True, exist_ok=True)
    sha = subprocess.check_output(
        ["git", "rev-parse", args.sha], cwd=REPO, text=True
    ).strip()
    export = args.scratch / f"source-{args.label}-{sha[:12]}"
    export.mkdir()
    archive = args.scratch / f"{args.label}-source.tar"
    with archive.open("wb") as output:
        subprocess.run(
            [
                "git",
                "archive",
                sha,
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                "src",
            ],
            cwd=REPO,
            stdout=output,
            check=True,
        )
    with tarfile.open(archive) as source:
        source.extractall(export, filter="data")
    shutil.copy2(REPO / PROBE, export / PROBE)
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target.resolve()))
    command = [
        "cargo",
        "test",
        "--release",
        "-p",
        "fathomdb-engine",
        "--features",
        "test-hooks",
        "--test",
        "slice50_drain_scaling",
        "--no-run",
        "--message-format=json",
    ]
    with (args.scratch / "build.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        result = subprocess.run(
            command, cwd=export, env=env, text=True, stdout=subprocess.PIPE, check=True
        )
        artifacts = [
            json.loads(line)
            for line in result.stdout.splitlines()
            if line.startswith("{")
        ]
        executable = next(
            Path(row["executable"])
            for row in artifacts
            if row.get("reason") == "compiler-artifact"
            and row.get("target", {}).get("name") == "slice50_drain_scaling"
            and row.get("executable")
        )
        binary = args.scratch / f"probe-{args.label}"
        shutil.copy2(executable, binary)
    metadata = dict(sha=sha, binary=str(binary.resolve()))
    (args.scratch / f"{args.label}.json").write_text(
        json.dumps(metadata, indent=2) + "\n"
    )
    print(json.dumps(metadata))


def run_one(binary: str, label: str, n: int, repeat: int, mode: str) -> dict:
    """Run one load-observed probe; retain timing and identity in its record."""
    loads = [os.getloadavg()[0]]
    stop = threading.Event()

    def sample() -> None:
        while not stop.wait(0.25):
            loads.append(os.getloadavg()[0])

    sampler = threading.Thread(target=sample)
    sampler.start()
    try:
        result = subprocess.run(
            [binary, "--ignored", "--nocapture", "--test-threads=1"],
            env=dict(os.environ, SLICE50_N=str(n), SLICE50_MODE=mode),
            text=True,
            capture_output=True,
            check=True,
        )
    finally:
        stop.set()
        sampler.join()
    row = parse_probe(result.stdout)
    loads.append(os.getloadavg()[0])
    row.update(
        label=label,
        repeat=repeat,
        load_before=loads[0],
        load_peak=max(loads),
        load_after=loads[-1],
    )
    return row


def run(args: argparse.Namespace) -> None:
    """Interleave paired runs and repeat a load-flagged cell at most once."""
    metadata = {
        label: json.loads((args.scratch / f"{label}.json").read_text())
        for label in ["base", "head"]
    }
    output = args.scratch / "runs.jsonl"
    with (args.scratch / "run.lock").open("w") as lock, output.open("w") as stream:
        fcntl.flock(lock, fcntl.LOCK_EX)
        cells = [
            (n, repeat, "held") for repeat in [0, 1] for n in [10000, 30000, 100000]
        ]
        cells += [(100000, 0, "overlap")]
        for n, repeat, mode in cells:
            for label in ["base", "head"] if repeat == 0 else ["head", "base"]:
                for attempt in [0, 1]:
                    started = time.monotonic()
                    while os.getloadavg()[0] > 2:
                        if time.monotonic() - started > args.max_load_wait:
                            raise RuntimeError(
                                "load remained above 2; no timed cell started"
                            )
                        time.sleep(1)
                    row = run_one(metadata[label]["binary"], label, n, repeat, mode)
                    row.update(sha=metadata[label]["sha"], attempt=attempt)
                    stream.write(json.dumps(row) + "\n")
                    stream.flush()
                    print(json.dumps(row), flush=True)
                    if row["load_peak"] <= 2.5:
                        break


def main() -> None:
    """CLI: build each label, run the matrix, then analyze its JSONL records."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    sub = parser.add_subparsers(dest="action", required=True)
    builder = sub.add_parser("build")
    builder.add_argument("--sha", required=True)
    builder.add_argument("--label", choices=["base", "head"], required=True)
    builder.add_argument("--target", type=Path, default=REPO / "target")
    runner = sub.add_parser("run")
    runner.add_argument("--max-load-wait", type=float, default=600)
    sub.add_parser("analyze")
    args = parser.parse_args()
    if args.action == "build":
        build(args)
    elif args.action == "run":
        run(args)
    else:
        rows = [
            json.loads(line)
            for line in (args.scratch / "runs.jsonl").read_text().splitlines()
        ]
        summary = analyze(rows)
        (args.scratch / "results.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
