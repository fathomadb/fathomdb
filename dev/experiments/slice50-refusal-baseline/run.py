"""Run the unchanged-engine refusal matrix and preserve its output and provenance."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[3]
PROBE = ROOT / "src/rust/crates/fathomdb-engine/tests/slice50_refusal_baseline.rs"
OPERATIONS = ["drain", "transition", "configure_projections", "actuate", "purge",
              "erase_source", "excise_source", "rebuild_projections", "rebuild_vec0"]
STATES = ["refused_pending", "refused_empty", "absent_pending", "accepted_ready"]


def main():
    """Execute the matrix, reject missing cells, and save JSON plus raw output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--threads", type=int, default=4)
    args = parser.parse_args()
    if args.repeats < 1 or args.threads < 1:
        parser.error("repeats and threads must be positive")
    args.out.mkdir(parents=True, exist_ok=True)
    command = ["cargo", "test", "--offline", "-p", "fathomdb-engine", "--features",
               "test-hooks,operator", "--test", "slice50_refusal_baseline", "--",
               "--ignored", "--nocapture", f"--test-threads={args.threads}"]
    env = os.environ.copy()
    env["REFUSAL_REPEATS"] = str(args.repeats)
    started = datetime.now(timezone.utc).isoformat()
    with (args.out / "raw.log").open("w") as log:
        completed = subprocess.run(command, cwd=ROOT, env=env, stdout=log,
                                   stderr=subprocess.STDOUT, check=False)
    records, recoveries = [], []
    for line in (args.out / "raw.log").read_text().splitlines():
        for marker, destination in [("REFUSAL_BASELINE ", records),
                                    ("REFUSAL_RECOVERY ", recoveries)]:
            if marker in line:
                destination.append(json.loads(line.split(marker, 1)[1]))
    def key(row):
        return row["operation"], row["state"], row["repeat"]

    expected = {(op, state, rep) for op in OPERATIONS for state in STATES
                for rep in range(args.repeats)}
    complete = (len(records) == len(expected) == len(recoveries)
                and {key(row) for row in records} == expected
                and {key(row) for row in recoveries} == expected)
    result = {"started_utc": started, "ended_utc": datetime.now(timezone.utc).isoformat(),
              "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "git_status": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
              "host": platform.platform(), "command": command, "repeats": args.repeats,
              "probe_sha256": hashlib.sha256(PROBE.read_bytes()).hexdigest(),
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "exit_code": completed.returncode, "complete": complete,
              "records": sorted(records, key=key), "recoveries": sorted(recoveries, key=key)}
    (args.out / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"matrix: {len(records)}/{len(expected)} observations; {len(recoveries)} recoveries; exit={completed.returncode}; complete={complete}")
    raise SystemExit(completed.returncode or (0 if complete else 1))


if __name__ == "__main__":
    main()
