"""Guarded D fixed-table entry; runtime selection remains separate from source controls."""

import argparse
import json
import os
from pathlib import Path
import sys
import time

from d_driver import reviewed_quartet, run_supervised_stage
from d_controller import cell_limit_ns
from d_attempt import inherited_settings
from g_campaign import active_bytes
from m_guard import guard, write_budget


def run_guarded(command, *, allocation, profile, root, task, **options):
    """Own D controller custody and reconcile its shared outer charge.

    The selected existing allocation/profile must fund all work and cleanup.
    Return the actual guardian result; unknown ownership blocks redispatch.
    This callable does not select a scientific profile or grant campaign GO.
    """
    began = options.get("started_ns")
    if began is None:
        began = time.monotonic_ns()
    try:
        result = guard(command, allocation=allocation, profile=profile, root=root, task=task,
                       phase="D", **dict(options, started_ns=began))
        result.setdefault("elapsed_ns", time.monotonic_ns()-began)
        result.setdefault("charged_ns", result.get("accounting", {}).get("charged_ns"))
        return result
    except (OSError, ValueError, RuntimeError, KeyError, TypeError) as error:
        return {"exit": 125, "state": "held", "reason": f"{type(error).__name__}: {error}",
                "elapsed_ns": time.monotonic_ns()-began, "charged_ns": None,
                "cleanup_uncertain": None, "unreconciled": True}


def _control(root, fd):
    from m_runtime import controller_admission
    owner = controller_admission(fd, root)
    entered = time.monotonic_ns()
    config = json.loads((root/"d-config.json").read_text())
    allocation, profile = Path(config["allocation"]), config["profile"]
    ledger_path, state_path = allocation/"g-budget.json", root/"d-state.json"
    ledger = json.loads(ledger_path.read_text())
    if ledger.get("d_guardian") != owner:
        raise RuntimeError("D controller lacks its actual shared-ledger guardian")
    budget = dict(ledger, shared_ns=profile["total_shared_ns"]-profile["reserved_other_work_ns"]-config["common_limit_ns"],
                  storage_limit=profile["total_shared_retained_bytes"]-profile["reserved_other_work_bytes"],
                  retained_bytes=active_bytes(allocation), cell_limits_ns=profile.get("cell_limits_ns", {}))
    records = json.loads(state_path.read_text()) if state_path.exists() else {}
    spec = json.loads(Path(__file__).with_name("d-cells.json").read_text())

    def publish():
        budget["retained_bytes"] = active_bytes(allocation)
        durable = dict(ledger, **{key: value for key, value in budget.items()
            if key not in ("shared_ns", "storage_limit", "retained_bytes", "cell_limits_ns")})
        write_budget(state_path, records)
        write_budget(ledger_path, durable)

    decision = ({"status": "admit", "cells": [cell["id"] for cell in spec["cells"][:4]]}
                if config["initial"] else reviewed_quartet(spec, records, budget, config.get("review")))
    rows = run_supervised_stage(spec, records, budget, root/"attempts", config["command"],
        publish=publish, initial=config["initial"], review=config.get("review"))
    source = config["scope"] == "isolated-harmless-source-control"
    source_complete = (source and all(row["status"] == "failed"
        and isinstance(row.get("native"), dict) and row["native"].get("unit_diagnostic") is True
        and row.get("cleanup_uncertain") is False for row in rows[:4])
        and all(row["status"] == "not_run_stage_review" for row in rows[4:]))
    target = [row for row in rows if row["id"] in decision.get("cells", [])]
    stage_complete = decision["status"] == "admit" and len(target) == 4 and all(row["status"] == "complete" for row in target)
    result = {"scope": config["scope"], "qualifies_natural": False, "controller_started_ns": entered,
              "source_control_complete": source_complete, "stage_complete": stage_complete, "stage_decision": decision,
              "campaign_complete": all(row["status"] == "complete" for row in rows), "rows": rows}
    write_budget(root/"d-report.json", result)
    return 0 if source_complete or stage_complete or result["campaign_complete"] else 1


def _source_config(root):
    root.mkdir(parents=True, exist_ok=False)
    write_budget(root/"g-budget.json", {"schema": "slice30-gd-budget/v1", "used_ns": 123,
        "inflight": None, "cell_used_ns": {"g-prior": 101}, "common_used_ns": {"M": 17}})
    profile = {"schema": "slice30-d-budget/v1", "status": "APPROVED",
        "ledger": str(root/"g-budget.json"), "whole_cell_ns": 900*10**9,
        "total_shared_ns": 60*10**9, "total_shared_retained_bytes": 1024**3,
        "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0}
    code = "\n".join([
        "import json,pathlib,sys",
        "config=json.loads(pathlib.Path(sys.argv[1]).read_text())",
        f"budget=json.loads(pathlib.Path({str(root/'g-budget.json')!r}).read_text())",
        "assert budget['inflight']['cell']==config['id']",
        f"with pathlib.Path({str(root/'dispatches.jsonl')!r}).open('a') as stream: stream.write(config['id']+'\\n')",
        "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':False,'unit_diagnostic':True}))",
    ])
    return {"scope": "isolated-harmless-source-control", "allocation": str(root),
            "profile": profile, "command": [sys.executable, "-c", code],
            "initial": True, "common_limit_ns": 40*10**9}


def _interrupted_rows(root, allocation, guardian):
    state = root/"d-state.json"
    try:
        records = json.loads(state.read_text()) if state.exists() else {}
    except (OSError, ValueError):
        records = {}
    try:
        budget = json.loads((Path(allocation)/"g-budget.json").read_text())
    except (OSError, ValueError):
        budget = {}
    active = budget.get("inflight") or {}
    if (guardian.get("controller_pid") and active.get("cell")
            and records.get(active["cell"], {}).get("status") not in ("complete", "failed", "incomplete", "interrupted")):
        records[active["cell"]] = {"id": active["cell"], "status": "interrupted",
            "error": "owned controller stopped before publishing its attempted record",
            "elapsed_ns": budget.get("cell_used_ns", {}).get(active["cell"]),
            "peak_bytes": None, "native": None, "cleanup_uncertain": guardian.get("cleanup_uncertain")}
        write_budget(state, records)
    spec = json.loads(Path(__file__).with_name("d-cells.json").read_text())
    return [{"id": cell["id"], **records[cell["id"]]} if cell["id"] in records else
            {"id": cell["id"], "status": "not_run_prerequisite", "reason": "D controller report unavailable"}
            for cell in spec["cells"]]


def main():
    """Route an explicit guarded D source control or selected fixed24 invocation.

    Scientific run requires AGENT_LONG and an existing approved bounded profile,
    original shared ledger, and an owned native executable. Source control creates
    only an isolated harmless allocation and never qualifies natural execution.
    """
    began = time.monotonic_ns()
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="action", required=True)
    actions.add_parser("source-control").add_argument("--root", required=True)
    run = actions.add_parser("run")
    run.add_argument("--options", required=True)
    internal = actions.add_parser("_controller")
    internal.add_argument("--root", required=True)
    internal.add_argument("--admission-fd", type=int, required=True)
    options = parser.parse_args()
    if options.action == "_controller":
        return _control(Path(options.root).resolve(), options.admission_fd)
    task = Path(os.environ["S30_TASK"]).resolve()
    if options.action == "source-control":
        root = Path(options.root).resolve()
        if not root.is_relative_to(task):
            raise ValueError("D source control must stay within its owned S30_TASK")
        config = _source_config(root)
    else:
        if os.environ.get("AGENT_LONG") != "1":
            raise RuntimeError("D scientific entry requires explicit selected runtime admission")
        selected = json.loads(Path(options.options).read_text())
        root, allocation = Path(selected["root"]).resolve(), Path(selected["allocation"]).resolve()
        native = Path(selected["binary"]).resolve()
        if (not root.is_relative_to(allocation) or not allocation.is_relative_to(task) or not native.is_relative_to(task)
                or not native.is_file() or not os.access(native, os.X_OK)):
            raise ValueError("D allocation and native executable must be owned by S30_TASK")
        config = {"scope": "fixed24-scientific", "allocation": str(allocation),
            "profile": json.loads(Path(selected["profile"]).read_text()), "command": [str(native)],
            "initial": selected.get("initial", True), "common_limit_ns": selected["common_limit_ns"],
            "review": json.loads(Path(selected["review"]).read_text()) if selected.get("review") else None}
        root.mkdir(parents=True, exist_ok=True)
    if type(config["initial"]) is not bool:
        raise ValueError("D initial stage selector must be boolean")
    if type(config["common_limit_ns"]) is not int or config["common_limit_ns"] <= 20*10**9:
        raise ValueError("D common allowance must fund bounded guardian cleanup")
    if config["scope"] == "fixed24-scientific":
        size = 10000 if config["initial"] else (config.get("review") or {}).get("n")
        if config["profile"].get("whole_cell_ns") != cell_limit_ns(size, config["profile"].get("cell_limits_ns")):
            raise ValueError("selected D profile must match the declared whole-cell allowance for its size")
    inherited = inherited_settings()

    def command():
        if inherited:
            raise ValueError("inherited settings would change D default execution: "+json.dumps(inherited))
        write_budget(root/"d-config.json", config)
        return [sys.executable, str(Path(__file__).resolve()), "_controller", "--root", str(root)]

    result = run_guarded(command, allocation=config["allocation"], profile=config["profile"],
        root=root, task=task, admission=True, common_limit_ns=config["common_limit_ns"], started_ns=began)
    report_path = root/"d-report.json"
    try:
        report = json.loads(report_path.read_text())
    except (OSError, ValueError):
        report = {}
    if inherited:
        spec = json.loads(Path(__file__).with_name("d-cells.json").read_text())
        review = config.get("review") or {}
        selected = {cell["id"] for cell in spec["cells"][:4]} if config["initial"] else {
            cell["id"] for cell in spec["cells"] if (cell["n"], cell["mode"]) == (review.get("n"), review.get("mode"))}
        rows = _interrupted_rows(root, config["allocation"], result)
        for row in rows:
            if row["status"].startswith("not_run"):
                row.update(status="not_run_stage_review", reason="outside the requested quartet")
                if row["id"] in selected:
                    row.update(status="not_run_inherited_settings", inherited_settings=inherited,
                               reason="inherited settings would change D default execution")
        report = {"scope": config["scope"], "qualifies_natural": False, "state": "held",
                  "source_control_complete": False, "stage_complete": False,
                  "inherited_settings": inherited, "rows": rows}
    elif (not isinstance(report.get("controller_started_ns"), int)
            or report["controller_started_ns"] < began):
        report = {"scope": config["scope"], "qualifies_natural": False, "source_control_complete": False,
            "stage_complete": False, "rows": _interrupted_rows(root, config["allocation"], result)}
    report["guardian"] = result
    if result.get("charged_ns") is not None:
        write_budget(report_path, report)
    print(json.dumps(report))
    qualified = report.get("source_control_complete") or report.get("stage_complete") or report.get("campaign_complete")
    return 0 if qualified and result["exit"] == 0 and result["cleanup_uncertain"] is False else 1


if __name__ == "__main__":
    raise SystemExit(main())
