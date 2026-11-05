"""Validate and summarize complete operation cells from the initial and corrected runs."""

import json
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parent
OPERATIONS = {"drain", "transition", "configure_projections", "actuate", "purge",
              "erase_source", "excise_source", "rebuild_projections", "rebuild_vec0"}
STATES = {"refused_pending", "refused_empty", "absent_pending", "accepted_ready"}


def main():
    """Select only complete, valid fixtures and retain links to both source records."""
    original = json.loads((ROOT / "results/results.json").read_text())
    corrected = json.loads((ROOT / "purge-corrected/results.json").read_text())
    assert corrected["complete"] and corrected["exit_code"] == 0
    records = [r for r in original["records"] if r["operation"] != "purge"]
    records += corrected["records"]
    recoveries = [r for r in original["recoveries"] if r["operation"] != "purge"]
    recoveries += corrected["recoveries"]

    def key(row):
        return row["operation"], row["state"], row["repeat"]

    expected = {(op, state, rep) for op in OPERATIONS for state in STATES for rep in range(3)}
    assert len(records) == len(recoveries) == len(expected) == 108
    assert {key(r) for r in records} == {key(r) for r in recoveries} == expected
    assert all(r["pending"] == 0 and not r["dense_disabled"] for r in recoveries)
    for r in records:
        outcome = ("Scheduler" if r["state"] == "refused_pending" else
                   "EmbedderRequired" if r["state"] == "absent_pending" and
                   (r["operation"] == "drain" or r["operation"].startswith("rebuild")) else "Ok")
        assert r["result"] == outcome, r
        if outcome != "Ok":
            assert r["unchanged"] and r["pending_before"] == r["pending_after"]
        if r["state"] == "refused_pending":
            assert r["embed_calls_during_operation"] == 0
    cells = []
    for op in sorted(OPERATIONS):
        for state in sorted(STATES):
            rows = [r for r in records if r["operation"] == op and r["state"] == state]
            durations = [r["elapsed_ms"] for r in rows]
            cells.append({"operation": op, "state": state, "n": len(rows),
                          "result": rows[0]["result"],
                          "pending_before": sorted({r["pending_before"] for r in rows}),
                          "pending_after": sorted({r["pending_after"] for r in rows}),
                          "min_ms": min(durations),
                          "median_ms": statistics.median(durations), "max_ms": max(durations)})
    summary = {"retained_observations": len(records), "retained_recoveries": len(recoveries),
               "all_failed_calls_unchanged": True, "all_refused_pending_calls_embedded_nothing": True,
               "original_source": "results/results.json",
               "corrected_source": "purge-corrected/results.json",
               "excluded": {"source": "results/results.json", "operation": "purge",
                            "observations": 2, "reason": "Active target; purge requires Deleted"},
               "cells": cells}
    (ROOT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print("108 valid observations and 108 accepted-runtime recoveries; all assertions pass")
    for cell in cells:
        if cell["state"] == "refused_pending":
            print(f"{cell['operation']}: {cell['median_ms'] / 1000:.6f} s, {cell['result']} (n=3)")


if __name__ == "__main__":
    main()
