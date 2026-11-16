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
from metrics import aggregate, paired_difference


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
        "raw_persistence": {"checkpoint_records":16, "unit":"per stream",
            "boundaries":"warmup/pass/diagnostic completion", "fsync":False,
            "scope":"outside API/write timers; included in timed loop throughput",
            "interruption":"up to15 tail records between completed checkpoints; pending flush may be partial"},
    }
    if manifest["corpus"]["count"] < max(cell["n"] for cell in plan):
        raise ValueError("corpus is smaller than a selected cell")
    out.mkdir(parents=True, exist_ok=False)
    for name, value in (("manifest.json", manifest), ("plan.json", plan)):
        path = out / name
        path.write_text(json.dumps(value, indent=2) + "\n")
        path.chmod(0o444)
    return manifest


def run_frozen(root):
    """Execute the frozen native plan under its resource/time limits, recording host load."""
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
            error = monitor()
            if error:
                raise RuntimeError(error)
            return observe()
        return run_campaign(plan, results, [manifest["binary"]["path"]],
                            timeout=manifest["cell_seconds"], before=before, monitor=monitor)


def read_jsonl_records(path, issues):
    """Read available JSONL observations, reporting invalid or missing records."""
    records = []
    if not path.exists():
        issues.append(f"missing {path.name}")
        return records
    for number, line in enumerate(path.read_bytes().splitlines(), 1):
        try:
            records.append(json.loads(line.decode("utf-8")))
        except (UnicodeDecodeError, json.JSONDecodeError):
            issues.append(f"partial/invalid {path.name}:{number}")
    return records


def summarize(root):
    """Report every planned cell, actual raw samples and explicit missing work."""
    root = Path(root)
    plan = json.loads((root / "plan.json").read_text())
    manifest = json.loads((root / "manifest.json").read_text())
    state_path = root / "results" / "campaign-state.json"
    state = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    output = []
    paired = {}
    resource_path = root / "resources.jsonl"
    resource_issues = []
    resources = read_jsonl_records(resource_path, resource_issues) if resource_path.exists() else []
    for cell in plan:
        previous = state.get(cell["id"])
        report = dict(id=cell["id"], family=cell["family"], n=cell["n"],
                      status=previous["status"] if previous else "not_run")
        output.append(report)
        if previous is None:
            continue
        attempt = Path(previous["attempt"])
        issues = []
        rows = read_jsonl_records(attempt / "samples.jsonl", issues)
        warmups = read_jsonl_records(attempt / "warmups.jsonl", issues)
        matches = read_jsonl_records(attempt / "matches.jsonl", issues)
        try:
            result = json.loads((attempt / "result.json").read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            result = {}
            issues.append("missing/partial result.json")
        expected = cell["samples"] * len(cell["adapters"])
        if len(rows) != expected or len(warmups) != cell["warmup"] * len(cell["adapters"]):
            issues.append("actual warmup/timed count differs from plan")
        if report["status"] == "complete" and (issues or result.get("complete") is not True):
            report["status"] = "incomplete"
        load_values = [r["load"] for r in resources if previous.get("pid") is not None and r.get("pid") == previous["pid"]]
        before_load = (previous.get("environment_before") or {}).get("load")
        if before_load is not None:
            load_values.append(before_load)
        report.update(attempt=str(attempt), issues=issues, actual=result, adapters=[],
                      commit_shape=cell.get("commit_shape"), max_load=max(load_values, default=None),
                      noisy=any(load > 2.5 for load in load_values) if load_values else None,
                      resources_available=bool(load_values))
        counts = [row["raw_fts_matches"] for row in matches]
        report["match_counts"] = {"queries":len(counts), "zero":counts.count(0),
            "min":min(counts, default=None), "max":max(counts, default=None),
            "mean":sum(counts) / len(counts) if counts else None}
        report["store_bytes"] = (attempt / "store.sqlite").stat().st_size if (attempt / "store.sqlite").exists() else None
        for adapter in cell["adapters"]:
            selected = [row for row in rows if row["adapter"] == adapter]
            wall = next((p["wall_ns"] for p in result.get("passes", []) if p["adapter"] == adapter), 0)
            stats = aggregate(selected, wall)
            tracked = (cell["family"] == "P1" and cell["n"] == 100000
                and result.get("filtered") is False and result.get("natural_records") == 100000
                and result.get("query_count") == 500 and len(matches) == 500
                and len(selected) == 1000 and stats["ok"] == 1000 and report["status"] == "complete")
            distance = ({"p50":stats["successful_latency"]["p50_ms"] - 80,
                         "p99":stats["successful_latency"]["p99_ms"] - 300} if tracked else None)
            report["adapters"].append({"adapter":adapter,"statistics":stats,
                "tracked_fixture":tracked,"tracked_distance_ms":distance,"gated":False})
        if cell["family"] == "P1" and report["status"] == "complete":
            paired.setdefault(str(cell["n"]), []).extend(rows)
    return {"scope":"P only; whole30/M/F/G/D and release exits are not qualified",
            "manifest":manifest,"cells":output,"gated":False,"resource_issues":resource_issues,
            "complete":bool(output) and all(cell["status"] == "complete" for cell in output),
            "paired_adapters":{n:paired_difference(rows) for n,rows in paired.items()}}


def main():
    """Operate an explicitly selected local campaign; never build in timings."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    freeze_parser = sub.add_parser("freeze")
    for field in ("cells", "corpus", "queries", "binary", "out"):
        freeze_parser.add_argument("--" + field, type=Path, required=True)
    run_parser = sub.add_parser("run")
    run_parser.add_argument("root", type=Path)
    summary_parser = sub.add_parser("summarize")
    summary_parser.add_argument("root", type=Path)
    args = parser.parse_args()
    if args.action == "freeze":
        freeze(args.cells, args.corpus, args.queries, args.binary, args.out)
    elif args.action == "run":
        if os.environ.get("AGENT_LONG") != "1":
            parser.error("manual campaign requires AGENT_LONG=1")
        results = run_frozen(args.root)
        if any(row["status"] != "complete" for row in results):
            raise SystemExit(1)
    else:
        result = summarize(args.root)
        (args.root / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps({"complete":result["complete"],"cells":len(result["cells"])}))
        if not result["complete"]:
            raise SystemExit(1)


if __name__ == "__main__":
    main()
