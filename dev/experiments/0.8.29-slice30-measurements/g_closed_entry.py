"""Closed-profile public operations under the named common reserve and guardian."""

import fcntl
import json
import os
from pathlib import Path
import sys
import time
import uuid

from g_closed_common import run_common


def worker_command(job):
    """Return the fixed committed common worker command, with no shell evaluation."""
    return [sys.executable, str(Path(__file__).with_name("g_runner.py")), "_common", str(job)]


def _call(allocation, mode, action, options):
    jobs = allocation / "common" / "jobs"
    jobs.mkdir(parents=True, exist_ok=True)
    job = jobs / f"{action}-{uuid.uuid4().hex}.json"
    job.write_text(json.dumps({"action": action, "options": options}, default=str) + "\n")
    outcome = run_common(allocation, mode, worker_command(job))
    if outcome["status"] != "complete":
        raise RuntimeError(f"supervised {action} incomplete: {outcome}")
    return json.loads(Path(outcome["attempt"], "result.json").read_text())["value"]


def freeze_entry(options):
    """Supervise blocking freeze before publication; outer guardian funds entry residual."""
    from g_boundary import require_known_containment
    require_known_containment(os.environ["S30_TASK"])
    allocation = Path(options["allocation"])
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return _call(allocation, options["stage"], "freeze", options)


def run_entry(root):
    """Supervise preflight/cost preparation and finish around the fixed controller mode."""
    from g_boundary import require_known_containment
    from g_runner import run_prepared
    require_known_containment(os.environ["S30_TASK"])
    root = Path(root).resolve()
    bootstrap = json.loads((root / "manifest.json").read_text())
    allocation, mode = Path(bootstrap["allocation"]), bootstrap["stage"]
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        prepared = _call(allocation, mode, "preflight", {"root": str(root)})
        rows = run_prepared(prepared, root)
        _call(allocation, mode, "finish", {"root": str(root)})
    return rows


def report_entry(root):
    """Generate a retained report in a monitored common worker, never an uncharged parent."""
    from g_boundary import require_known_containment
    require_known_containment(os.environ["S30_TASK"])
    root = Path(root).resolve()
    manifest = json.loads((root / "manifest.json").read_text())
    allocation = Path(manifest["allocation"])
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return _call(allocation, manifest["stage"], "report", {"root": str(root)})


def _preflight(root):
    from g_freeze import validate_frozen
    from g_gpu import inspect_gpu
    manifest = validate_frozen(root)
    if inspect_gpu(manifest["gpu"]["uuid"])["applications"]:
        raise RuntimeError("selected3090 has foreign/current compute work")
    if manifest["stage"] != "closed-copy-calibration":
        from g_campaign import active_bytes
        from g_closed_costs import load_calibration
        from g_closed_forecast import forecast_closed_read
        calibration = load_calibration(manifest["calibration_from"], manifest)
        manifest["calibration"] = {key: value for key, value in calibration.items() if key != "records"}
        if manifest["stage"] == "closed-copy-read-r1":
            budget = json.loads(Path(manifest["allocation"], "g-budget.json").read_text())
            r0 = manifest["reused"]["g-r-n100000-r0"]
            observed = {"elapsed_ns": max(r0["elapsed_ns"], budget.get("cell_used_ns", {}).get(r0["id"], 0)),
                        "retained_bytes": active_bytes(r0["attempt"]), "n": 100000, "repeat": 0}
            seed = manifest["reused"]["g1-n100000"]
            old_seed = 2 * max(seed["elapsed_ns"], budget.get("cell_used_ns", {}).get(seed["id"], 0))
            cell = next(row for row in manifest["plan"] if row["id"] == "g-r-n100000-r1")
            prediction = forecast_closed_read(cell, observed, calibration["costs"],
                manifest["closed_limits"]["campaign_seconds"] * 10**9 - budget["used_ns"],
                active_bytes(manifest["allocation"]), old_seed)
            manifest["_closed_predictions"] = {cell["id"]: prediction}
        else:
            raise ValueError("write-matrix requires recorded reviewed costs after qualified R1")
    manifest.update(corpus=manifest["inputs"]["corpus"]["path"], queries=manifest["inputs"]["queries"]["path"],
                    commands={key: [value["path"]] for key, value in manifest["binaries"].items()})
    manifest["_prepared_manifest"] = str(root / f"prepared-closed-{uuid.uuid4().hex}.json")
    Path(manifest["_prepared_manifest"]).write_text(json.dumps(manifest, indent=2) + "\n")
    Path(manifest["_prepared_manifest"]).chmod(0o444)
    # The controller needs only small routing/cost metadata; the child reads the
    # complete prepared subject under its own absolute deadline.
    return {key: value for key, value in manifest.items() if key not in ("closed_subject", "model_files")}


def _finish(root):
    from g_campaign import completed_record
    from g_gpu import inspect_gpu
    manifest = json.loads((root / "manifest.json").read_text())
    if inspect_gpu(manifest["gpu"]["uuid"])["applications"]:
        raise RuntimeError("selected3090 is not quiet after owned work")
    rows = json.loads((root / "results/g-outcomes.json").read_text())
    mode = manifest["stage"]
    calibration_path = root / "results/calibration-outcomes.json"
    calibration = json.loads(calibration_path.read_text()) if calibration_path.exists() else []
    if mode == "closed-copy-calibration":
        complete = len(calibration) == 4 and all(completed_record(row) for row in calibration)
    elif mode == "closed-copy-read-r1":
        complete = any(row["id"] == "g-r-n100000-r1" and completed_record(row) for row in rows)
    else:
        complete = len(rows) == 84 and all(completed_record(row) for row in rows)
    if any(row.get("cleanup_uncertain") for row in [*rows, *calibration]):
        raise RuntimeError("owned cell cleanup is uncertain")
    invocation = root / "invocations" / f"run-{uuid.uuid4().hex}"
    invocation.mkdir(parents=True)
    value = {"path": str(invocation), "environment_ok": True, "error": None,
             "finished_at_ns": time.time_ns(), "stage": mode, "stage_complete": complete,
             "campaign_complete": len(rows) == 84 and all(completed_record(row) for row in rows),
             "guardian_receipt": os.environ.get("S30_CLOSED_RECEIPT"),
             "scope": "cell quiescence/quiet check; final outer-boundary proof is the guardian receipt"}
    for path in (invocation / "invocation.json", root / "last-run.json"):
        path.write_text(json.dumps(value, indent=2) + "\n")
    return value


def common_worker(job, output):
    """Execute only fixed freeze/preflight/report/finish operations in the owned worker."""
    request = json.loads(Path(job).read_text())
    action, options = request["action"], request["options"]
    if action == "freeze":
        from g_freeze import freeze
        value = freeze(options)
    elif action == "preflight":
        value = _preflight(Path(options["root"]).resolve())
    elif action == "finish":
        value = _finish(Path(options["root"]).resolve())
    elif action == "report":
        from g_report import summarize
        root = Path(options["root"]).resolve()
        value = summarize(root)
        with (root / f"summary-g-{uuid.uuid4().hex}.json").open("x") as stream:
            stream.write(json.dumps(value, indent=2) + "\n")
    else:
        raise ValueError("unsupported closed common operation")
    Path(output, "result.json").write_text(json.dumps({"complete": True, "value": value}) + "\n")
