"""Execute explicit G/D manifest cells through the maintained attempt machinery."""

import argparse
import json
from pathlib import Path

from campaign import run_campaign


def run_manifest(manifest, root):
    """Run supplied cells in order with their actual native artifact identities."""
    outcomes = []
    for planned in manifest["cells"]:
        artifact = manifest["artifacts"][planned["artifact"]]
        cell = dict(planned, artifact_identity=artifact)
        outcomes.extend(run_campaign([cell], root, [artifact["path"]],
                                     timeout=manifest["cell_seconds"], completion=_native_complete))
    return outcomes


def _native_complete(cell, attempt):
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
