"""Bounded experiment controller; budgets are not production defaults."""
from itertools import product
import json
from pathlib import Path
import subprocess
import sys
import time

REQUESTS = 1000
CELL_SECONDS = 600
CAMPAIGN_SECONDS = 7200
RSS_BYTES = 8 * 1024**3


def grid():
    """Breadth-first predeclared order, family innermost."""
    return list(product([0, 4, 16], [128, 4096], [1, 8],
                        ["current", "corrected", "prior", "archive", "inactive", "superseded", "both"],
                        ["node", "edge", "property"]))


def stress_grid():
    """Revised diagnostic order, explicitly separate from the original grid."""
    return []


def stop_reason(cell_seconds, campaign_seconds, rss):
    if campaign_seconds >= CAMPAIGN_SECONDS:
        return "campaign_time"
    if rss >= RSS_BYTES:
        return "rss"
    if cell_seconds >= CELL_SECONDS:
        return "cell_time"
    return None


def summary(records, termination):
    successes = sorted(r["ms"] for r in records if r["status"] == "success")
    errors = [r["ms"] for r in records if r["status"] == "error"]
    qualified = len(successes) == REQUESTS and len(records) == REQUESTS and termination is None
    return {"scheduled": REQUESTS, "success": len(successes), "error": len(errors),
            "unfinished": max(0, REQUESTS - len(records)), "qualified": qualified,
            "p50_ms": successes[499] if qualified else None,
            "p99_ms": successes[989] if qualified else None,
            "success_duration_ms": sum(successes), "error_duration_ms": sum(errors),
            "termination": termination}


def rss_bytes(pid):
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except FileNotFoundError:
        pass
    return 0


def main():
    binary = Path(sys.argv[1]).resolve()
    root = Path(sys.argv[2]).resolve()
    root.mkdir(parents=True, exist_ok=True)
    cells = [{"cell": list(cell), "status": "not_run", "scheduled": REQUESTS,
              "success": 0, "error": 0, "unfinished": REQUESTS} for cell in grid()]
    report = root / "campaign.json"
    report.write_text(json.dumps(cells, indent=2) + "\n")
    started = time.monotonic()
    for index, entry in enumerate(cells):
        if time.monotonic() - started >= CAMPAIGN_SECONDS:
            break
        h, payload, callers, view, family = entry["cell"]
        stem = f"{index:03}-{family}-H{h}-B{payload}-C{callers}-{view}"
        log = root / f"{stem}.jsonl"
        error_log = root / f"{stem}.stderr"
        peak = 0
        termination = None
        begin = time.monotonic()
        with log.open("w") as stdout, error_log.open("w") as stderr:
            process = subprocess.Popen([str(binary), family, str(h), str(payload), str(callers), view, str(root / "fixtures")], stdout=stdout, stderr=stderr)
            while process.poll() is None:
                peak = max(peak, rss_bytes(process.pid))
                termination = stop_reason(time.monotonic() - begin, time.monotonic() - started, peak)
                if termination:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                    break
                time.sleep(0.05)
        if process.returncode and termination is None:
            termination = f"exit_{process.returncode}"
        records = []
        for line in log.read_text().splitlines():
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                termination = termination or "truncated_record"
        entry.update(summary(records, termination))
        entry.update(status="complete" if entry["qualified"] else "incomplete", peak_rss_bytes=peak,
                     elapsed_s=time.monotonic() - begin, log=log.name,
                     admitted_documents=records[0].get("documents") if records else None,
                     indexed_bytes=records[0].get("indexed_bytes") if records else None)
        report.write_text(json.dumps(cells, indent=2) + "\n")
        print(json.dumps(entry), flush=True)
    print(json.dumps({"campaign_elapsed_s": time.monotonic() - started,
                      "complete": sum(c["status"] == "complete" for c in cells),
                      "incomplete": sum(c["status"] == "incomplete" for c in cells),
                      "not_run": sum(c["status"] == "not_run" for c in cells)}), flush=True)


if __name__ == "__main__":
    main()
