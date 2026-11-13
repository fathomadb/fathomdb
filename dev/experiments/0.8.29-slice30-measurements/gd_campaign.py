"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
import json
from pathlib import Path

from campaign import run_campaign


def run_manifest(manifest, root, monitor=None):
    """Run supplied cells in order with their actual native artifact identities."""
    outcomes = []
    for planned in manifest["cells"]:
        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        outcomes.extend(run_campaign([cell], root, [artifact["path"]],
                                     timeout=manifest["cell_seconds"], completion=_native_complete, monitor=monitor,
                                     poll_interval=0.1, observe_start=monitor is not None))
    return outcomes


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
