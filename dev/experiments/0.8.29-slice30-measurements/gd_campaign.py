"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
import json
from pathlib import Path
import time

from campaign import run_campaign


def run_manifest(manifest, root, monitor=None, before=None):
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
        if time.monotonic_ns() >= deadline:
            raise _CampaignRefusal("campaign_time_cap")

    outcomes = []
    for planned in manifest["cells"]:
        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        try:
            outcomes.extend(run_campaign([cell], root, [artifact["path"]],
                                         timeout=manifest["cell_seconds"], completion=_native_complete,
                                         monitor=monitor, before=prepare,
                                         poll_interval=0.1, observe_start=monitor is not None))
        except _CampaignRefusal as error:
            outcome = {"id": cell["id"], "cell": cell, "status": "not_run_resource",
                       "reason": str(error), "pid": None, "attempt": None, "exit": None}
            state_path = root / "campaign-state.json"
            previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
            previous[cell["id"]] = outcome
            state_path.write_text(json.dumps(list(previous.values()), indent=2) + "\n")
            outcomes.append(outcome)
    return outcomes


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
