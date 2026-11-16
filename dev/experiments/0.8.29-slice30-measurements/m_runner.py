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
from m_record import CONTROL_MAX_BYTES, OutputLimitError, write_json


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
    began = time.monotonic_ns()
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    smoke = commands.add_parser("source-smoke")
    for name in ("root", "cli", "helper"):
        smoke.add_argument(f"--{name}", required=True)
    source_dispatch = commands.add_parser("source-dispatch")
    for name in ("root", "cli", "helper"):
        source_dispatch.add_argument(f"--{name}", required=True)
    source_dispatch.add_argument("--source-total-ns", type=int, default=60 * 10**9)
    tools = commands.add_parser("build")
    tools.add_argument("--variant", choices=("providerfree", "helper", "ordinary"), required=True)
    tools.add_argument("--output", required=True)
    freezing = commands.add_parser("freeze")
    freezing.add_argument("--options", required=True)
    freezing.add_argument("--root", required=True)
    calibration = commands.add_parser("calibrate")
    for name in ("options", "root", "cell"):
        calibration.add_argument(f"--{name}", required=True)
    selected_internal = commands.add_parser("_selected")
    for name in ("options", "root", "cell"):
        selected_internal.add_argument(f"--{name}", required=True)
    selected_internal.add_argument("--admission-fd", type=int, required=True)
    calibration_internal = commands.add_parser("_calibrate")
    for name in ("options", "root", "cell"):
        calibration_internal.add_argument(f"--{name}", required=True)
    calibration_internal.add_argument("--admission-fd", type=int, required=True)
    for action in ("run", "report"):
        selected = commands.add_parser(action)
        selected.add_argument("root")
        if action == "run":
            selected.add_argument("--options")
            selected.add_argument("--cell")
    internal = commands.add_parser("_controller")
    internal.add_argument("--root", required=True)
    internal.add_argument("--admission-fd", type=int, required=True)
    source_internal = commands.add_parser("_source_controller")
    source_internal.add_argument("--root", required=True)
    source_internal.add_argument("--admission-fd", type=int, required=True)
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
    if options.action == "run" and options.options:
        result = run_selected(options.options, options.root, options.cell, began)
        return 0 if isinstance(result, dict) and result["exit"] == 0 and not result["cleanup_uncertain"] else 1
    if options.action == "calibrate":
        from m_calibration import selected_cell
        from m_freeze import approved
        from m_guard import guard
        if os.environ.get("AGENT_LONG") != "1":
            raise RuntimeError("calibration requires explicit AGENT_LONG=1 and selected approved allocation")
        selected_cell(options.cell)
        selected = json.loads(Path(options.options).read_text())
        profile = json.loads(Path(selected["profile"]).read_text())
        approved(profile, selected["allocation"])
        limits = selected["calibration"]
        root = Path(options.root).resolve()
        if root.exists():
            raise ValueError("calibration requires a new retained root; cumulative allowance never renews")
        result = guard([sys.executable, str(Path(__file__).resolve()), "_calibrate",
            "--options", str(Path(options.options).resolve()), "--root", str(root), "--cell", options.cell],
            allocation=selected["allocation"], profile=profile, root=root, task=os.environ["S30_TASK"],
            admission=True, calibration=limits, common_limit_ns=limits["phase_ns"]-limits["payload_ns"],
            started_ns=began)
        print(json.dumps(result))
        return 0 if result["exit"] == 0 and not result["cleanup_uncertain"] else 1
    elif options.action == "source-dispatch":
        from g_freeze import identify
        from m_controller import COMPONENTS
        from m_guard import guard
        root = Path(options.root).resolve()
        task = Path(os.environ["S30_TASK"]).resolve()
        if not root.is_relative_to(task):
            raise ValueError("source dispatch requires a new root within its owned S30_TASK")
        root.mkdir(parents=True, exist_ok=False)
        (root / "g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
            "used_ns": 0, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
        profile = {"schema": "slice30-m-budget/v1", "status": "APPROVED",
            "ledger": str(root / "g-budget.json"), "whole_cell_ns": 900 * 10**9,
            "total_shared_ns": options.source_total_ns, "total_shared_retained_bytes": 256 * 1024**2,
            "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0,
            "other_work_cost_status": "BOUNDED",
            "reservation_disposition": "Tiny source control only; no external work or natural qualification"}
        # These literal source-control bounds cannot name or qualify a natural subject.
        components = dict.fromkeys(COMPONENTS, 0)
        components.update(copy=200000000, first=100000000, close=100000000)
        common = dict.fromkeys(COMPONENTS, 0)
        common.update(packaging=10 * 10**9, close=10 * 10**9)
        manifest = {"schema": "slice30-m-source-control/v1", "scope": "tiny-provider-free-source-control",
            "uuid": f"source-{time.time_ns()}", "source_sha": "source-control-unqualified",
            "root": str(root), "task": str(task), "allocation": str(root), "profile": profile,
            "plan": load_plan()[:2], "builds": {name: {"binary": identify(path)} for name, path in
                (("providerfree", options.cli), ("helper", options.helper))},
            "forecasts": {"cells": {cell["id"]: {"components_ns": components,
                "new_retained_bytes": 1024**2} for cell in load_plan()[:2]},
                "common_components_ns": common, "common_new_retained_bytes": 1024**2}}
        (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        try:
            outcome = guard([sys.executable, str(Path(__file__).resolve()), "_source_controller",
                             "--root", str(root)], allocation=root, profile=profile, root=root, task=task,
                             admission=True, common_limit_ns=40 * 10**9, started_ns=began)
        except (RuntimeError, ValueError, OSError) as error:
            result = {"complete": False, "qualifies_natural": False, "rows": [
                {"id": cell["id"], "status": "held", "complete": False, "reason": str(error)}
                for cell in manifest["plan"]]}
            print(json.dumps(result))
            return 1
        reports = sorted((root / "reports").glob("report-*.json"))
        result = json.loads(reports[-1].read_text()) if reports else {"complete": False, "rows": []}
        result.update(qualifies_natural=False, scope=manifest["scope"], guardian=outcome)
        result["complete"] &= outcome["exit"] == 0 and not outcome["cleanup_uncertain"]
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
            if selected.get("scope") == "bounded-natural-calibration":
                raise ValueError("calibration cannot enter primary run/report admission; its guarded stage retains reports")
            profile, allocation, forecast = selected["profile"], selected["allocation"], selected["forecasts"]
            command = [sys.executable, str(Path(__file__).resolve()),
                       "_controller" if options.action == "run" else "_report",
                       "--root", str(Path(options.root).resolve())]
        approved(profile, allocation)
        common_limit = 2 * sum(forecast["common_components_ns"].values())
        if common_limit <= 20 * 10**9:
            raise ValueError("complete common forecast must fund bounded guardian cleanup")
        result = guard(command, allocation=allocation, profile=profile, root=options.root,
                       task=os.environ["S30_TASK"], admission=True, common_limit_ns=common_limit,
                       started_ns=began)
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
            if options.action == "_selected":
                from m_calibration import control as selected_control
                from m_freeze import freeze_selected
                manifest = freeze_selected(json.loads(Path(options.options).read_text()), options.root, options.cell)
                result = selected_control(options.root, owner, manifest)
                report(options.root)
            elif options.action == "_calibrate":
                from m_calibration import control as calibrate
                from m_freeze import freeze_calibration
                manifest = freeze_calibration(json.loads(Path(options.options).read_text()), options.root, options.cell)
                result = calibrate(options.root, owner, manifest)
                report(options.root)
            elif options.action == "_freeze":
                from m_freeze import freeze
                result = freeze(json.loads(Path(options.options).read_text()), options.root)
            elif options.action == "_report":
                result = report(options.root)
            else:
                source = json.loads((Path(options.root) / "manifest.json").read_text()) if options.action == "_source_controller" else None
                rows = control(options.root, owner, source_manifest=source)
                result = {"complete": all(row["complete"] for row in rows), "rows": rows}
                report(options.root)
        print(json.dumps(result, allow_nan=False))
        return 0 if result.get("complete", options.action == "_freeze") else 1
    print(json.dumps(result, allow_nan=False))
    return 0 if result["complete"] else 1


def run_selected(options, root, cell_id, began):
    """Run one canonical required M cell through its finite original-budget guardian."""
    from m_calibration import registered_cell, validate_limits
    from m_freeze import approved
    from m_guard import guard
    if os.environ.get("AGENT_LONG") != "1":
        raise RuntimeError("selected M cell requires explicit AGENT_LONG=1")
    registered_cell(cell_id)
    selected = json.loads(Path(options).read_text())
    profile = json.loads(Path(selected["profile"]).read_text())
    approved(profile, selected["allocation"])
    limits = selected["cell_allowance"]
    validate_limits(limits)
    root = Path(root).resolve()
    if root.exists():
        raise ValueError("selected M cell requires a fresh root; cumulative allowance never renews")
    command = [sys.executable, str(Path(__file__).resolve()), "_selected", "--options", str(Path(options).resolve()),
               "--root", str(root), "--cell", cell_id]
    result = guard(command, allocation=selected["allocation"], profile=profile, root=root,
        task=os.environ["S30_TASK"], admission=True, started_ns=began,
        cell_allowance={"cell_id": cell_id, "limits": limits},
        common_limit_ns=limits["phase_ns"]-limits["payload_ns"])
    if result["exit"] == 0 and result["cleanup_uncertain"] is False and (root / "manifest.json").is_file():
        result["retired_working_copies"] = retire_working_copies(root)
        result["final_report"] = publish_selected(root)
        if not result["final_report"]["phase_qualified"]:
            result.update(exit=124, stop="selected final publication allowance")
    print(json.dumps(result))
    return result


def publish_selected(root):
    """Publish selected-cell credit only after original final cleanup, charging final report work."""
    import fcntl
    from m_guard import write_budget
    began = time.monotonic_ns()
    root = Path(root).resolve()
    final_guardian(root)
    manifest = json.loads((root / "manifest.json").read_text())
    allocation, profile = Path(manifest["allocation"]), manifest["profile"]
    identity = manifest["plan"][0]["id"]
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        ledger_path = allocation / "g-budget.json"
        ledger = json.loads(ledger_path.read_text())
        if any(ledger.get(key) for key in ("inflight", "m_guardian", "d_guardian", "closed_guardian")):
            raise ValueError("live shared ownership blocks selected final publication")
        usage = ledger["selected_cell_usage"][identity]
        remaining = min(manifest["cell_allowance"]["phase_ns"] - usage["wall_ns"],
                        profile["total_shared_ns"] - profile["reserved_other_work_ns"] - ledger["used_ns"])
        value = None
        report_path = root / "reports" / f"report-{time.time_ns()}.json"
        try:
            value = report(root, deadline_ns=began + remaining - 10**9, output_path=report_path)
        finally:
            elapsed = time.monotonic_ns() - began
            qualified = _charge_final_work(root, ledger_path, ledger, usage, elapsed,
                                          manifest["cell_allowance"], profile,
                                          failed=value is None or not value["phase_qualified"])
            if value is not None and value["phase_qualified"] and not qualified:
                value.update(complete=False, phase_qualified=False)
                for row in value["rows"]:
                    if row["id"] == identity and row.get("complete"):
                        row.update(execution_complete=True, complete=False, status="incomplete",
                                   reason="selected final report write exhausted its allowance")
                write_json(report_path, value)
                correction_ns = time.monotonic_ns() - began - elapsed
                _charge_final_work(root, ledger_path, ledger, usage, correction_ns,
                                   manifest["cell_allowance"], profile, failed=True)
    return value


def retire_working_copies(root):
    """Retire successful disposable copies after strict cleanup, preserving science and charges."""
    import fcntl
    from m_guard import write_budget
    began = time.monotonic_ns()
    root = Path(root).resolve()
    final_guardian(root)
    manifest = json.loads((root / "manifest.json").read_text())
    allocation = Path(manifest["allocation"]).resolve()
    state_path = root / "results/campaign-state.json"
    outcomes = json.loads(state_path.read_text())
    if len(outcomes) != 1:
        raise ValueError("one original selected M outcome required for retirement")
    outcome = outcomes[0]
    if (not outcome.get("complete") or outcome.get("status") != "complete" or outcome.get("exit") != 0
            or outcome.get("cleanup_uncertain") is not False or outcome.get("live_owned_pids") != []
            or outcome["cell"]["family"] not in ("M-R", "M-C", "M-L")):
        return []
    attempt = Path(outcome["attempt"]).resolve()
    if not root.is_relative_to(allocation) or not attempt.is_relative_to(root / "results" / outcome["id"]):
        raise ValueError("owned selected M retirement identity required")
    if json.loads((attempt / "result.json").read_text()).get("complete") is not True:
        return []
    prepared = json.loads((attempt / "prepared.json").read_text())
    paths = [Path(copy["destination"]).resolve() for copy in prepared["copies"]]
    if any(path.parent != attempt / "stores" for path in paths):
        raise ValueError("retirement is limited to prepared disposable store copies")
    calibration = manifest.get("scope") == "bounded-natural-calibration"
    usage_key = "calibration_usage" if calibration else "selected_cell_usage"
    limits = manifest["calibration" if calibration else "cell_allowance"]
    profile = manifest["profile"]
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        ledger_path = allocation / "g-budget.json"
        ledger = json.loads(ledger_path.read_text())
        if any(ledger.get(key) for key in ("inflight", "m_guardian", "d_guardian", "closed_guardian")):
            raise ValueError("live shared ownership blocks working-copy retirement")
        usage = ledger[usage_key][outcome["id"]]
        retired = list(outcome.get("retired_working_copies", []))
        try:
            for path in paths:
                elapsed = time.monotonic_ns() - began
                if (usage["wall_ns"] + elapsed + 20 * 10**9 >= limits["phase_ns"]
                        or ledger["used_ns"] + elapsed + profile["reserved_other_work_ns"] + 20 * 10**9
                        >= profile["total_shared_ns"]):
                    break
                if path.exists():
                    path.unlink()
                    retired.append(str(path))
                    outcome["retired_working_copies"] = retired
                    write_budget(attempt / "attempt.json", outcome)
                    write_budget(state_path, outcomes)
        finally:
            elapsed = time.monotonic_ns() - began
            if calibration:
                ledger["used_ns"] += elapsed
                usage["wall_ns"] += elapsed
                common = ledger.setdefault("common_used_ns", {})
                common["M"] = common.get("M", 0) + elapsed
                write_budget(ledger_path, ledger)
            else:
                _charge_final_work(root, ledger_path, ledger, usage, elapsed, limits, profile)
    return retired


def _charge_final_work(root, ledger_path, ledger, usage, elapsed, limits, profile, *, failed=False):
    from m_guard import write_budget
    ledger["used_ns"] += elapsed
    usage["wall_ns"] += elapsed
    common = ledger.setdefault("common_used_ns", {})
    common["M"] = common.get("M", 0) + elapsed
    path = next((root / "guardians").glob("*/guardian.json"))
    receipt = json.loads(path.read_text())
    receipt["post_guardian_finalization_ns"] = receipt.get("post_guardian_finalization_ns", 0) + elapsed
    accounting = receipt.setdefault("accounting", {})
    accounting["charged_ns"] = accounting.get("charged_ns", 0) + elapsed
    if (failed or usage["wall_ns"] >= limits["phase_ns"]
            or ledger["used_ns"] + profile["reserved_other_work_ns"] >= profile["total_shared_ns"]):
        receipt.update(exit=124, stop="selected final publication allowance")
    write_budget(path, receipt)
    write_budget(ledger_path, ledger)
    return receipt.get("exit") == 0


def final_guardian(root):
    """Require the fresh root's sole original phase to have strict successful final cleanup."""
    paths = list((Path(root) / "guardians").glob("*/guardian.json"))
    if len(paths) != 1:
        raise ValueError("one original final guardian required")
    receipt = json.loads(paths[0].read_text())
    boundary = receipt.get("boundary") or {}
    if (receipt.get("exit") != 0 or receipt.get("child_exit") != 0
            or receipt.get("cleanup_uncertain") is not False
            or receipt.get("boundary_populated_at_controller_exit") is not False
            or boundary.get("status") != "quiescent" or boundary.get("populated_after") is not False
            or boundary.get("removed_exact_empty_boundary") is not True):
        raise ValueError("original guardian did not qualify final cleanup")
    return receipt


def _loop_population(result, attempt, cell, deadline_ns=None):
    phases = (("first", cell["first"]), ("warmup", cell["warmups"]), ("timed", cell["timed"]))
    streams = {}
    for name in ("submissions", "invocations"):
        with (attempt / "wire" / (name + ".jsonl")).open() as stream:
            streams[name] = []
            for line in stream:
                if deadline_ns is not None and time.monotonic_ns() >= deadline_ns:
                    raise TimeoutError("selected final report allowance exhausted")
                streams[name].append(json.loads(line))
    for phase, count in phases:
        value = result.get("phases", {}).get(phase, {})
        summary = value.get("summary", {})
        submitted, completed = value.get("submitted", []), value.get("completed", [])
        if (any(summary.get(key) != count for key in ("submitted", "completed", "success"))
                or summary.get("unresolved") != 0 or summary.get("errors") != {}
                or len(submitted) != count or len(set(submitted)) != count
                or len(completed) != count or set(submitted) != set(completed)):
            raise ValueError("complete registered population unavailable")
        sent = [row for row in streams["submissions"] if row.get("phase") == phase]
        received = [row for row in streams["invocations"] if row.get("phase") == phase]
        if ([row.get("rpc_id") for row in sent] != submitted
                or [row.get("query_index") for row in sent] != [i % 500 for i in range(count)]
                or len(received) != count or {row.get("rpc_id") for row in received} != set(completed)
                or any(row.get("status") != "completed" or row.get("pid") != result["pid"]
                       for row in received)):
            raise ValueError("complete ordered raw population unavailable")


def open_invocation_log(root, kind):
    """Create a distinct original invocation log beside its selected fresh root."""
    root = Path(root).resolve()
    if kind not in ("service", "supervisor", "samples"):
        raise ValueError("registered M invocation log kind required")
    suffix = ".jsonl" if kind == "samples" else ".log"
    return (root.parent / (root.name + "-" + kind + suffix)).open("x", encoding="utf-8")


def retry_history(manifest):
    """Retain strictly cleaned failed predecessors without supplying completion credit."""
    history, seen = [], {Path(manifest["root"]).resolve()}
    current = manifest
    while current.get("retry_from"):
        previous = Path(current["retry_from"]).resolve()
        allocation = Path(manifest["allocation"]).resolve()
        if previous in seen or not previous.is_relative_to(allocation):
            raise ValueError("M predecessor must be a distinct root in the original allocation")
        seen.add(previous)
        original = json.loads((previous / "manifest.json").read_text())
        for key in ("allocation", "compiler_inputs", "closed_subject", "package", "inputs", "model",
                    "gpu", "builds", "plan", "cell_allowance"):
            if original.get(key) != manifest.get(key):
                raise ValueError("M predecessor input/native/configuration/allowance identity differs")
        if (original.get("scope") != "registered-M-selected-cell"
                or Path(original["root"]).resolve() != previous):
            raise ValueError("original selected M predecessor root required")
        paths = list((previous / "guardians").glob("*/guardian.json"))
        if len(paths) != 1:
            raise ValueError("one original M predecessor guardian required")
        guardian = json.loads(paths[0].read_text())
        boundary = guardian.get("boundary") or {}
        if (guardian.get("cleanup_uncertain") is not False
                or guardian.get("boundary_populated_at_controller_exit") is not False
                or boundary.get("status") != "quiescent" or boundary.get("populated_after") is not False
                or boundary.get("removed_exact_empty_boundary") is not True
                or (boundary.get("path") and Path(boundary["path"]).exists())):
            raise ValueError("M predecessor lacks strict terminal cleanup")
        cell_id = manifest["plan"][0]["id"]
        outcomes = json.loads((previous / "results/campaign-state.json").read_text())
        usage = json.loads((allocation / "g-budget.json").read_text()).get("selected_cell_usage", {}).get(cell_id, {})
        if (len(outcomes) != 1 or outcomes[0].get("id") != cell_id
                or outcomes[0].get("cleanup_uncertain") is not False
                or outcomes[0].get("live_owned_pids") != []
                or usage.get("limits") != manifest["cell_allowance"]
                or str(previous) not in usage.get("roots", [])):
            raise ValueError("M predecessor outcome and original cumulative allowance required")
        outcome = outcomes[0]
        if outcome.get("complete") is True and guardian.get("exit") == 0:
            raise ValueError("a qualified M predecessor cannot be replayed")
        attempt = Path(outcome["attempt"]).resolve()
        if not attempt.is_relative_to(previous / "results" / cell_id):
            raise ValueError("M predecessor attempt identity differs")
        history.append(dict(outcome, complete=False, source_sha=original["source_sha"],
                            predecessor_root=str(previous), result_path=str(attempt / "result.json")))
        current = original
    return list(reversed(history))


def reused_rows(manifest, *, deadline_ns=None):
    """Reference complete matched original evidence without changing its scope or charges."""
    from m_calibration import CALIBRATION_ID, CELL_ID, registered_cell
    rows = {}
    for reference in manifest.get("reuse_from", []):
        if deadline_ns is not None and time.monotonic_ns() >= deadline_ns:
            raise TimeoutError("selected final report allowance exhausted")
        root = Path(reference["root"]).resolve()
        if not root.is_relative_to(Path(manifest["allocation"]).resolve()):
            raise ValueError("reused M evidence must stay in its original allocation")
        original = json.loads((root / "manifest.json").read_text())
        if original.get("scope") not in ("bounded-natural-calibration", "registered-M-selected-cell"):
            raise ValueError("explicit completed selected M evidence required")
        cell = registered_cell(reference["cell_id"])
        calibration = original["scope"] == "bounded-natural-calibration"
        identity = CALIBRATION_ID if calibration else cell["id"]
        if calibration and cell["id"] != CELL_ID:
            raise ValueError("calibration supplies only its identical first primary row")
        for key in ("allocation", "compiler_inputs", "closed_subject", "package", "inputs", "model", "gpu"):
            if original[key] != manifest[key]:
                raise ValueError("reused M input/compiler/device identity differs")
        final_guardian(root)
        outcomes = json.loads((root / "results/campaign-state.json").read_text())
        if len(outcomes) != 1 or outcomes[0]["id"] != identity:
            raise ValueError("one explicit completed M evidence row required")
        outcome = outcomes[0]
        attempt = Path(outcome["attempt"]).resolve()
        if not attempt.is_relative_to(root / "results" / identity):
            raise ValueError("original M attempt identity unavailable")
        result_path = attempt / "result.json"
        result = json.loads(result_path.read_text())
        actual_cell = dict(result["cell"])
        if calibration:
            if actual_cell.pop("origin_id", None) != cell["id"]:
                raise ValueError("original calibration configuration unavailable")
            actual_cell["id"] = cell["id"]
        variant = ("ordinary" if cell["family"] in ("M-R", "M-C", "M-L") else
                   "helper" if cell["family"] == "M-E" else "providerfree")
        if (actual_cell != cell or result.get("freeze_uuid") != original["uuid"]
                or result.get("selected_build") != original["builds"][variant]
                or original["builds"][variant].get("compiler_inputs") != manifest["compiler_inputs"]
                or original["builds"][variant] != manifest["builds"].get(variant, original["builds"][variant])):
            raise ValueError("original M workload/build identity differs")
        if (outcome.get("complete") is not True or outcome.get("status") != "complete"
                or outcome.get("exit") != 0 or outcome.get("cleanup_uncertain") is not False
                or outcome.get("live_owned_pids") != [] or result.get("complete") is not True
                or result.get("gpu", {}).get("complete") is not True or result["gpu"].get("failures")):
            raise ValueError("original complete M result and GPU qualification required")
        if cell["family"] in ("M-R", "M-C"):
            _loop_population(result, attempt, cell, deadline_ns)
        rows[cell["id"]] = {"id": cell["id"], "cell": cell, "status": "complete", "complete": True,
            "satisfied_by": {"root": str(root), "result": str(result_path), "attempt": str(attempt),
                "accounting_id": identity, "scope": original["scope"], "source_sha": original["source_sha"],
                "build_source_sha": result["selected_build"]["source_sha"], "freeze_uuid": original["uuid"]},
            "attempts": [*retry_history(original), dict(outcome, result=result)]}
    return rows


def report(root, *, deadline_ns=None, output_path=None):
    """Retain all46 required cells and every attempt; incomplete outcomes never disappear."""
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text())
    state = root / "results/campaign-state.json"
    current = {row["id"]: row for row in json.loads(state.read_text())} if state.exists() else {}
    current = dict(reused_rows(manifest, deadline_ns=deadline_ns), **current)
    prior_attempts = retry_history(manifest) if manifest.get("retry_from") else []
    rows = []
    calibration = manifest.get("scope") == "bounded-natural-calibration"
    plan = load_plan() if calibration or manifest.get("scope") == "registered-M-selected-cell" else manifest["plan"]
    for cell in [*plan, *(manifest["plan"] if calibration else [])]:
        row = dict(current.get(cell["id"], {"id": cell["id"], "cell": cell,
                   "status": "held", "complete": False, "reason": "no completed registered attempt"}))
        history = list(row.get("attempts", []))
        if prior_attempts and cell["id"] == manifest["plan"][0]["id"]:
            history.extend(prior_attempts)
        for path in sorted((root / "results" / cell["id"]).glob("attempt-*/attempt.json")):
            attempt = json.loads(path.read_text())
            result = path.with_name("result.json")
            if result.exists():
                attempt["result"] = json.loads(result.read_text())
            load = path.with_name("load.json")
            if load.exists():
                attempt["load"] = json.loads(load.read_text())
            history.append(attempt)
        row["attempts"] = history
        rows.append(row)
    phase_qualified = True
    if manifest.get("scope") == "registered-M-selected-cell":
        try:
            final_guardian(root)
        except ValueError:
            phase_qualified = False
        if deadline_ns is not None and time.monotonic_ns() >= deadline_ns:
            phase_qualified = False
        if not phase_qualified:
            for row in rows:
                if row["id"] == manifest["plan"][0]["id"] and row.get("complete"):
                    row.update(execution_complete=True, complete=False, status="incomplete",
                               reason="original final guardian/publication allowance is unqualified")
    value = {"schema": "slice30-m-report/v1", "freeze_uuid": manifest["uuid"],
             "source_sha": manifest["source_sha"], "complete": all(row["complete"] for row in rows),
             "rows": rows, "verdict": None, "calibrated_threshold": None,
             "phase_qualified": phase_qualified}
    value["complete"] &= phase_qualified
    if calibration:
        value.update(scope=manifest["scope"], qualifies_primary=False,
                     rows=rows[:46], calibrations=rows[46:], complete=False,
                     calibration_complete=all(row["complete"] for row in rows[46:]))
    value["phase_load"] = [{"receipt": str(path), "load": json.loads(path.read_text()).get("load")}
                           for path in sorted((root / "guardians").glob("*/guardian.json"))]
    value["phase_load_scope"] = "completed guardian phases before this report"
    destination = root / "reports"
    destination.mkdir(exist_ok=True)
    path = destination / f"report-{time.time_ns()}.json" if output_path is None else Path(output_path)
    try:
        write_json(path, value)
    except OutputLimitError as error:
        value.update(complete=False, phase_qualified=False, output_overflow=str(error))
        value["rows"] = [{"id": row["id"], "complete": False, "status": "incomplete",
                          "reason": "report capture limit", "attempts": [
                              {"attempt": item.get("attempt"), "complete": False}
                              for item in row.get("attempts", [])]} for row in rows]
        write_json(path, value, limit=CONTROL_MAX_BYTES)
    return value


if __name__ == "__main__":
    raise SystemExit(main())
