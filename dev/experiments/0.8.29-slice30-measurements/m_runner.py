"""Committed M commands; source controls cannot qualify natural workloads."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from m_cell import execute
from m_controller import load_plan


def source_smoke(root, cli, helper):
    """Exercise the public M0 bindings using new retained SDK-created tiny stores."""
    root = Path(root).resolve()
    root.mkdir(parents=True, exist_ok=False)
    stores = root / "stores"
    seeded = subprocess.run([str(Path(helper).resolve()), "--seed-fixture", str(stores)],
                            capture_output=True, timeout=15)
    (root / "seed.stdout").write_bytes(seeded.stdout)
    (root / "seed.stderr").write_bytes(seeded.stderr)
    if seeded.returncode:
        raise RuntimeError("source-control SDK fixture failed")
    rows = []
    for cell in (row for row in load_plan() if row["family"] == "M0"):
        path = root / f"{cell['id']}.json"
        path.write_text(json.dumps(dict(cell["config"], stores={name: {
            "path": str(stores / f"{name}.sqlite")} for name in ("a", "b")}, default_store="a")))
        value = execute(cell, [str(Path(cli).resolve()), "--config", str(path)], root / cell["id"],
                        queries=["alpha"], deadline_ns=time.monotonic_ns() + 60 * 10**9)
        rows.append(dict(value, id=cell["id"]))
    result = {"scope": "tiny-provider-free-source-control", "qualifies_natural": False,
              "complete": all(row["complete"] for row in rows), "rows": rows}
    (root / "source-report.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """Dispatch explicit owned M source or future selected campaign commands."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    smoke = commands.add_parser("source-smoke")
    for name in ("root", "cli", "helper"):
        smoke.add_argument(f"--{name}", required=True)
    source_dispatch = commands.add_parser("source-dispatch")
    for name in ("root", "cli", "helper"):
        source_dispatch.add_argument(f"--{name}", required=True)
    tools = commands.add_parser("build")
    tools.add_argument("--variant", choices=("providerfree", "helper", "ordinary"), required=True)
    tools.add_argument("--output", required=True)
    freezing = commands.add_parser("freeze")
    freezing.add_argument("--options", required=True)
    freezing.add_argument("--root", required=True)
    for action in ("run", "report"):
        commands.add_parser(action).add_argument("root")
    internal = commands.add_parser("_controller")
    internal.add_argument("--root", required=True)
    internal.add_argument("--admission-fd", type=int, required=True)
    internal_freeze = commands.add_parser("_freeze")
    internal_freeze.add_argument("--options", required=True)
    internal_freeze.add_argument("--root", required=True)
    internal_freeze.add_argument("--admission-fd", type=int, required=True)
    internal_report = commands.add_parser("_report")
    internal_report.add_argument("--root", required=True)
    internal_report.add_argument("--admission-fd", type=int, required=True)
    cell = commands.add_parser("_cell")
    cell.add_argument("--root", required=True)
    cell.add_argument("--deadline-ns", type=int, required=True)
    cell.add_argument("config")
    cell.add_argument("attempt")
    options = parser.parse_args()
    if options.action == "source-dispatch":
        root = Path(options.root).resolve()
        root.mkdir(parents=True, exist_ok=False)
        (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
            "used_ns": 0, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
        result = source_smoke(root / "tiny", options.cli, options.helper)
    elif options.action == "source-smoke":
        result = source_smoke(options.root, options.cli, options.helper)
    elif options.action == "build":
        from m_freeze import build
        print(json.dumps(build(options.output, options.variant)))
        return 0
    elif options.action in ("freeze", "run", "report"):
        from m_guard import guard
        from m_freeze import approved
        if os.environ.get("AGENT_LONG") != "1":
            raise RuntimeError("M campaign commands require explicit AGENT_LONG=1 and selected approved allocation")
        if options.action == "freeze":
            selected = json.loads(Path(options.options).read_text())
            profile = json.loads(Path(selected["profile"]).read_text())
            allocation = selected["allocation"]
            forecast = json.loads(Path(selected["forecasts"]).read_text())
            command = [sys.executable, str(Path(__file__).resolve()), "_freeze", "--options",
                       str(Path(options.options).resolve()), "--root", str(Path(options.root).resolve())]
        else:
            selected = json.loads((Path(options.root) / "manifest.json").read_text())
            profile, allocation, forecast = selected["profile"], selected["allocation"], selected["forecasts"]
            command = [sys.executable, str(Path(__file__).resolve()),
                       "_controller" if options.action == "run" else "_report",
                       "--root", str(Path(options.root).resolve())]
        approved(profile, allocation)
        common_limit = 2 * sum(forecast["common_components_ns"].values())
        if common_limit <= 20 * 10**9:
            raise ValueError("complete common forecast must fund bounded guardian cleanup")
        result = guard(command, allocation=allocation, profile=profile, root=options.root,
                       task=os.environ["S30_TASK"], admission=True, common_limit_ns=common_limit)
        print(json.dumps(result))
        return 0 if result["exit"] == 0 and not result["cleanup_uncertain"] else 1
    else:
        from m_runtime import control, controller_admission, payload
        if options.action == "_cell":
            manifest = json.loads((Path(options.root) / "manifest.json").read_text())
            cell = json.loads(Path(options.config).read_text())
            result = payload(manifest, cell, options.attempt, options.deadline_ns)
        else:
            owner = controller_admission(options.admission_fd, options.root)
            if options.action == "_freeze":
                from m_freeze import freeze
                result = freeze(json.loads(Path(options.options).read_text()), options.root)
            elif options.action == "_report":
                result = report(options.root)
            else:
                rows = control(options.root, owner)
                result = {"complete": all(row["complete"] for row in rows), "rows": rows}
                report(options.root)
        print(json.dumps(result, allow_nan=False))
        return 0 if result.get("complete", options.action == "_freeze") else 1
    print(json.dumps(result, allow_nan=False))
    return 0 if result["complete"] else 1


def report(root):
    """Retain all46 required cells and every attempt; incomplete outcomes never disappear."""
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text())
    state = root / "results/campaign-state.json"
    current = {row["id"]: row for row in json.loads(state.read_text())} if state.exists() else {}
    rows = []
    for cell in manifest["plan"]:
        row = dict(current.get(cell["id"], {"id": cell["id"], "cell": cell,
                   "status": "held", "complete": False, "reason": "no completed registered attempt"}))
        history = []
        for path in sorted((root / "results" / cell["id"]).glob("attempt-*/attempt.json")):
            attempt = json.loads(path.read_text())
            result = path.with_name("result.json")
            if result.exists():
                attempt["result"] = json.loads(result.read_text())
            history.append(attempt)
        row["attempts"] = history
        rows.append(row)
    value = {"schema": "slice30-m-report/v1", "freeze_uuid": manifest["uuid"],
             "source_sha": manifest["source_sha"], "complete": all(row["complete"] for row in rows),
             "rows": rows, "verdict": None, "calibrated_threshold": None}
    destination = root / "reports"
    destination.mkdir(exist_ok=True)
    (destination / f"report-{time.time_ns()}.json").write_text(json.dumps(value, indent=2) + "\n")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
