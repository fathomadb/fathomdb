"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
import json
import subprocess
from pathlib import Path
import time

from campaign import run_campaign
from gd_resources import _cleanup_owned_group, _owned_subreaper


def run_manifest(manifest, root, monitor=None, before=None, resources=None):
    """Run supplied cells in order with their actual native artifact identities."""
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    clock_path = root / "campaign-clock.json"
    boot_id = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    if not clock_path.exists():
        clock_path.write_text(json.dumps({"started_ns": time.monotonic_ns(), "boot_id": boot_id}) + "\n")
    clock = json.loads(clock_path.read_text())
    deadline = clock["started_ns"] + int(min(manifest["campaign_seconds"], 14400) * 1_000_000_000)

    def prepare():
        check_clock()
        context = before() if before else None
        check_clock()
        return context

    def check_clock():
        if clock["boot_id"] != boot_id:
            raise _CampaignRefusal("campaign_clock_unavailable")
        now = time.monotonic_ns()
        if now >= deadline:
            raise _CampaignRefusal("campaign_time_cap")
        if now >= work_deadline:
            raise _CampaignRefusal("cell_work_deadline")

    outcomes = []
    for planned in manifest["cells"]:
        hard_deadline = min(deadline, time.monotonic_ns()+int(min(manifest["cell_seconds"],900)*1_000_000_000))
        work_deadline = hard_deadline-30_000_000_000
        bounds = {"work_deadline_ns": work_deadline, "hard_deadline_ns": hard_deadline,
                  "shared_deadline_ns": deadline}

        def execute(cell, command, config, attempt, observe):
            return _run_native(cell, command, config, attempt, observe, bounds)

        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        try:
            outcomes.extend(run_campaign([cell], root, [artifact["path"]],
                                         timeout=manifest["cell_seconds"], completion=_native_complete,
                                         monitor=monitor, before=prepare,
                                         poll_interval=0.1, observe_start=monitor is not None, execute=execute))
        except _CampaignRefusal as error:
            outcome = {"id": cell["id"], "cell": cell, "status": "not_run_resource",
                       "reason": str(error), "pid": None, "attempt": None, "exit": None}
            state_path = root / "campaign-state.json"
            previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
            previous[cell["id"]] = outcome
            state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
            outcomes.append(outcome)
    return outcomes


def _run_native(cell, command, config, attempt, monitor, bounds):
    native = dict(cell)
    for key, limit in bounds.items():
        native[key] = min(native.get(key, limit), limit)
    hard_ns = min(native["hard_deadline_ns"], native["shared_deadline_ns"])
    native["work_deadline_ns"] = min(native["work_deadline_ns"], max(0, hard_ns-30_000_000_000))
    config.write_text(json.dumps(native, indent=2) + "\n")
    refusal = {"exit":125, "error":"native_prelaunch_deadline", "pid":None, "pgid":None,
               "child_exit":None, "cleanup":{"complete":True,"not_started":True}}
    if time.monotonic_ns() >= native["work_deadline_ns"]:
        return refusal
    work = native["work_deadline_ns"] / 1_000_000_000
    hard = hard_ns / 1_000_000_000
    error = None
    with _owned_subreaper(), (attempt / "child.log").open("wb") as log:
        if time.monotonic_ns() >= native["work_deadline_ns"]:
            return refusal
        child = subprocess.Popen([*command, str(config), str(attempt)], stdout=log,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        try:
            while True:
                now = time.monotonic()
                if now >= work:
                    error = "cell work deadline"
                    break
                if child.poll() is not None:
                    break
                if monitor:
                    error = monitor(child.pid)
                    if error:
                        break
                remaining = work-time.monotonic()
                if remaining <= 0:
                    error = "cell work deadline"
                    break
                try:
                    child.wait(timeout=min(0.1, remaining))
                except subprocess.TimeoutExpired:
                    pass
            if monitor and error is None:
                error = monitor(child.pid)
            if time.monotonic() >= work:
                error = error or "cell work deadline"
        finally:
            if child.poll() is None:
                cooperative = min(time.monotonic()+5, hard-25)
                if cooperative > time.monotonic():
                    try:
                        child.wait(timeout=cooperative-time.monotonic())
                    except subprocess.TimeoutExpired:
                        pass
            cleanup = _cleanup_owned_group(child, deadline=min(hard-5, time.monotonic()+20))
        if not cleanup["complete"]:
            error = error or "owned group cleanup incomplete"
        if time.monotonic() >= hard:
            error = error or "cell hard deadline"
        return {"exit":124 if error else child.returncode, "error":error,
                "pid":child.pid, "pgid":child.pid, "child_exit":child.returncode, "cleanup":cleanup}


class _CampaignRefusal(Exception):
    pass


def _native_complete(cell, attempt):
    if cell["family"] in {"G1_PLAN_FIXTURE", "G1_CELL"}:
        base = attempt / "cell" if cell["family"] == "G1_PLAN_FIXTURE" else attempt
        passes = cell["passes"]
        query_count = len(Path(cell["queries"]).read_text().splitlines())
        warmups = cell.get("warmups", 0)
        for ordinal, mode in enumerate(passes):
            path = base / f"pass-{ordinal}" / "api-proof.json"
            if not path.exists():
                return False
            proof = json.loads(path.read_text())
            if (proof.get("mode"), proof.get("warmup_api_calls"), proof.get("timed_api_calls"),
                    proof.get("api_calls")) != (mode, warmups, query_count, warmups+query_count):
                return False
        return bool(passes)
    name = "complete.json" if cell["family"] == "D_QUERY_FIXTURE" else "result.json"
    path = attempt / name
    return path.exists() and json.loads(path.read_text()).get("complete") is True


def main():
    """Execute an explicit manifest and print actual per-cell outcomes."""
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    outcomes = run_manifest(json.loads(args.manifest.read_text()), args.output)
    print(json.dumps(outcomes, indent=2))
    return int(any(row["status"] != "complete" for row in outcomes))


if __name__ == "__main__":
    raise SystemExit(main())
