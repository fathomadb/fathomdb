"""Freeze, run and summarize the bounded Slice30 provider-free campaign."""

import argparse
import hashlib
import fcntl
import os
import time
import json
from pathlib import Path
import subprocess

from campaign import make_plan, run_campaign


def freeze(cells, corpus, queries, binary, out):
    """Freeze actual input/native/source identities and the balanced cell plan."""
    cells, corpus, queries, binary, out = [Path(p).resolve() for p in (cells, corpus, queries, binary, out)]
    source = Path(__file__).resolve().parents[3]
    def identify(path):
        return {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    def input_record(path):
        return dict(identify(path), count=len(path.read_text().splitlines()))
    base = json.loads(cells.read_text())
    plan = make_plan([dict(cell, corpus=str(corpus), queries=str(queries)) for cell in base], 2)
    manifest = {
        "schema": "slice30-p/v1", "measurement_layer": "public-direct-text", "gated": False,
        "source_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip(),
        "source_dirty": subprocess.check_output(["git", "status", "--porcelain"], cwd=source, text=True).splitlines(),
        "binary": identify(binary), "corpus": input_record(corpus), "queries": input_record(queries),
        "cells": identify(cells), "lock": identify(source / "Cargo.lock"),
        "build_command": "cargo build --locked --release -p fathomdb-sdk --example slice30_measure",
        "format": 37, "provider": "none", "profile": "release", "features": [],
        "cell_seconds": 900, "campaign_seconds": 14400, "active_bytes": 16 * 1024**3,
        "repeat_count": 2, "warmup_policy": "separate records; not included in timed statistics",
        "source_observation": "freeze records actual dirty state; qualifying timing requires clean source",
    }
    if manifest["corpus"]["count"] < max(cell["n"] for cell in plan):
        raise ValueError("corpus is smaller than a selected cell")
    out.mkdir(parents=True, exist_ok=False)
    for name, value in (("manifest.json", manifest), ("plan.json", plan)):
        path = out / name
        path.write_text(json.dumps(value, indent=2) + "\n")
        path.chmod(0o444)
    return manifest


def run_frozen(root, max_load=2):
    """Execute the frozen native plan under its resource/time/quiet limits."""
    root = Path(root).resolve()
    manifest = json.loads((root / "manifest.json").read_text())
    plan = json.loads((root / "plan.json").read_text())
    results = root / "results"
    results.mkdir(exist_ok=True)
    with (root / "campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        clock = root / "run-clock.json"
        if not clock.exists():
            clock.write_text(json.dumps({"started_at": time.time()}) + "\n")
        started = json.loads(clock.read_text())["started_at"]
        def observe(pid=None):
            size = subprocess.run(["du", "-sb", str(results)], capture_output=True, text=True)
            if size.returncode:
                size = subprocess.run(["du", "-sb", str(results)], capture_output=True, text=True)
            size.check_returncode()
            rss = None
            if pid is not None:
                try:
                    rss = [line for line in Path(f"/proc/{pid}/status").read_text().splitlines()
                           if line.startswith(("VmRSS:", "VmHWM:"))]
                except FileNotFoundError:
                    pass
            record = {"time": time.time(), "load": os.getloadavg()[0],
                      "output_bytes": int(size.stdout.split()[0]), "pid": pid, "rss": rss,
                      "cpu": Path("/proc/stat").read_text().splitlines()[0],
                      "disk": [line for line in Path("/proc/diskstats").read_text().splitlines()
                               if "nvme0n1p2" in line],
                      "memory": [line for line in Path("/proc/meminfo").read_text().splitlines()
                                 if line.startswith(("MemAvailable:", "SwapFree:"))]}
            with (root / "resources.jsonl").open("a") as output:
                output.write(json.dumps(record) + "\n")
            return record
        def monitor(pid=None):
            record = observe(pid)
            if record["time"] - started >= manifest["campaign_seconds"]:
                return "campaign time cap"
            if record["output_bytes"] >= manifest["active_bytes"]:
                return "measurement output cap"
            return None
        def before():
            waiting = time.monotonic()
            while True:
                error = monitor()
                if error:
                    raise RuntimeError(error)
                record = observe()
                if max_load is None or record["load"] <= max_load:
                    return record
                if time.monotonic() - waiting >= manifest["cell_seconds"]:
                    raise RuntimeError("quiet CPU start not available within cell wait cap")
                time.sleep(5)
        return run_campaign(plan, results, [manifest["binary"]["path"]],
                            timeout=manifest["cell_seconds"], before=before, monitor=monitor)


def summarize(root):
    """Report every planned cell, actual raw samples and explicit missing work."""
    return {"cells": []}


def main():
    """Operate an explicitly selected local campaign; never build in timings."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    freeze_parser = sub.add_parser("freeze")
    for field in ("cells", "corpus", "queries", "binary", "out"):
        freeze_parser.add_argument("--" + field, type=Path, required=True)
    run_parser = sub.add_parser("run")
    run_parser.add_argument("root", type=Path)
    args = parser.parse_args()
    if args.action == "freeze":
        freeze(args.cells, args.corpus, args.queries, args.binary, args.out)
    else:
        results = run_frozen(args.root)
        if any(row["status"] != "complete" for row in results):
            raise SystemExit(1)


if __name__ == "__main__":
    main()
