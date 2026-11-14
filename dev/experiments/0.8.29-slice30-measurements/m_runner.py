"""Committed M commands; source controls cannot qualify natural workloads."""

import argparse
import json
from pathlib import Path
import subprocess
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
    options = parser.parse_args()
    result = source_smoke(options.root, options.cli, options.helper)
    print(json.dumps(result, allow_nan=False))
    return 0 if result["complete"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
