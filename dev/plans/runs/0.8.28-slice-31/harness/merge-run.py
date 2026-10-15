#!/usr/bin/env python3
"""Merge one qualification run into run-NNN.json.

Usage: merge-run.py <run-prefix> <exit-code> <wall-ms> <flags>

Reads <prefix>.out (the consumer's JSON line), <prefix>.err (stderr),
<prefix>.host.json and <prefix>.host-after.json, writes <prefix>.json and
prints a one-line summary. The allocator decision is already in the consumer's
JSON (`allocator`, from the open report); nothing is parsed from stderr
except the first lines, kept as `stderrHead`.
"""

import json
import sys
from pathlib import Path
from typing import Any


def main() -> None:
    prefix, rc, wall, flags = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    out_lines = [ln for ln in Path(prefix + ".out").read_text(errors="replace").splitlines() if ln.startswith("{")]
    run: dict[str, Any]
    try:
        run = json.loads(out_lines[-1]) if out_lines else {"outcome": "noresult"}
    except json.JSONDecodeError:
        run = {"outcome": "badjson"}
    run["host"] = json.loads(Path(prefix + ".host.json").read_text())
    after = Path(prefix + ".host-after.json")
    if after.exists():
        run["hostAfter"] = json.loads(after.read_text())
    run["exitCode"] = rc
    run["wallMs"] = wall
    run["flags"] = flags
    run["stderrHead"] = Path(prefix + ".err").read_text(errors="replace").splitlines()[:20]
    Path(prefix + ".json").write_text(json.dumps(run) + "\n")
    alloc = run.get("allocator") or {}
    steady = run.get("timingsMs", {}).get("embedSteady") or []
    med = sorted(steady)[len(steady) // 2] if steady else None
    imp = run.get("timingsMs", {}).get("import")
    print(
        f"{Path(prefix).name} rc={rc} outcome={run.get('outcome')} step={run.get('failedStep')} "
        f"path={alloc.get('path')} reason={alloc.get('reason')} init={alloc.get('moduleLoadInit')} "
        f"steady_med={med and round(med, 1)} import_ms={imp and round(imp, 1)} wall={wall}"
    )


if __name__ == "__main__":
    main()
