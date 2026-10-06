#!/usr/bin/env python3
"""Merge one pool-study Node run into run-NNN.json (protocol section 4.2).

Usage: merge-run.py <run-prefix> <exit-code> <wall-ms> <node-flags>

Reads <prefix>.out (the consumer's JSON line), <prefix>.err (stderr; the
`fdb-pool-exp` lines become `poolEvents`), <prefix>.host.json and
<prefix>.host-after.json, writes <prefix>.json and prints a one-line summary.
`allocMode` is the alloc_mode of the first `decide` event, `none` when there
is none; `allocModeConsistent` is false if any later `decide` disagrees (a C1
failure).
"""

import json
import sys
from pathlib import Path


def parse_event(line: str) -> dict:
    fields = {}
    for token in line.split()[1:]:
        if "=" in token:
            key, value = token.split("=", 1)
            fields[key] = value
    return fields


def main() -> None:
    prefix, rc, wall, flags = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    out_lines = [ln for ln in Path(prefix + ".out").read_text(errors="replace").splitlines() if ln.startswith("{")]
    try:
        run = json.loads(out_lines[-1]) if out_lines else {"outcome": "noresult"}
    except json.JSONDecodeError:
        run = {"outcome": "badjson"}
    err = Path(prefix + ".err").read_text(errors="replace").splitlines()
    events = [parse_event(ln) for ln in err if ln.startswith("fdb-pool-exp ")]
    decides = [e for e in events if e.get("event") == "decide"]
    run["allocMode"] = decides[0].get("alloc_mode", "none") if decides else "none"
    run["allocModeConsistent"] = all(e.get("alloc_mode") == run["allocMode"] for e in decides)
    run["poolEvents"] = events
    run["host"] = json.loads(Path(prefix + ".host.json").read_text())
    after = Path(prefix + ".host-after.json")
    if after.exists():
        run["hostAfter"] = json.loads(after.read_text())
    run["exitCode"] = rc
    run["wallMs"] = wall
    run["nodeFlags"] = flags
    run["stderrOther"] = [ln for ln in err if not ln.startswith("fdb-pool-exp ")][:20]
    Path(prefix + ".json").write_text(json.dumps(run) + "\n")
    install = next((e for e in events if e.get("event") == "install"), {})
    steady = run.get("timingsMs", {}).get("embedSteady") or []
    med = sorted(steady)[len(steady) // 2] if steady else None
    print(
        f"{Path(prefix).name} rc={rc} outcome={run.get('outcome')} step={run.get('failedStep')} "
        f"alloc={run['allocMode']} create={install.get('create', '-')} probe={install.get('probe', '-')} "
        f"default_pool={install.get('default_pool', '-')} steady_med={med and round(med, 1)} "
        f"import_ms={run.get('timingsMs', {}).get('import') and round(run['timingsMs']['import'], 1)} wall={wall}"
    )


if __name__ == "__main__":
    main()
