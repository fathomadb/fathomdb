"""Fixed D admission seam; actual worker execution/qualification is separate."""


def _qualified_lower(n, mode, records):
    for layer in ("C", "T"):
        for repeat in (0, 1):
            record = records.get(f"d-{layer.lower()}-{mode}-n{n}-r{repeat}")
            qualified = (record is not None and record.get("status") == "complete"
                         and record.get("closed") is True and record.get("error_count") == 0
                         and record.get("physical_count") == n)
            if qualified and layer == "C":
                qualified = (record.get("observations_complete") is True
                             and record.get("dropped_records") == 0
                             and (mode != "overlap" or record.get("actual_overlap") is True))
            if not qualified:
                return False
    return True


def assess_stage(n, mode, records, budget):
    """Return prospective admission for one required size/mode quartet."""
    if n == 10000:
        return {"status": "admit"}
    lower = 10000 if n == 100000 else 100000
    if not _qualified_lower(lower, mode, records):
        return {"status": "not_run_lower_evidence", "lower_n": lower,
                "reason": "required lower C/T repeats are missing or unqualified"}
    predictions = {}
    for layer in ("C", "T"):
        times, sizes = [], []
        for lower_n in (10000, 100000):
            if lower_n >= n or not _qualified_lower(lower_n, mode, records):
                continue
            rows = [records[f"d-{layer.lower()}-{mode}-n{lower_n}-r{repeat}"]
                    for repeat in (0, 1)]
            if any(type(row.get(key)) is not int or row[key] < 0
                   for row in rows for key in ("elapsed_ns", "peak_bytes")):
                return {"status": "not_run_cost_unknown", "reason": "required lower cost is unavailable"}
            times.append((2 * max(row["elapsed_ns"] for row in rows) * n + lower_n - 1) // lower_n)
            sizes.append((2 * max(row["peak_bytes"] for row in rows) * n + lower_n - 1) // lower_n)
        predictions[layer] = {"time_ns": max(times), "active_bytes": max(sizes)}
    quartet_bytes = 2 * sum(value["active_bytes"] for value in predictions.values())
    quartet_time_ns = 2 * sum(value["time_ns"] for value in predictions.values())
    assessment = {"status": "admit", "predictions": predictions, "quartet_bytes": quartet_bytes,
                  "quartet_time_ns": quartet_time_ns}
    if budget["used_ns"] + quartet_time_ns > budget["shared_ns"]:
        assessment.update(status="not_run_cap_feasibility", reason="predicted quartet exceeds remaining shared time")
    if any(value["active_bytes"] >= budget["storage_limit"] for value in predictions.values()) or budget["retained_bytes"] + quartet_bytes >= budget["storage_limit"]:
        assessment.update(status="not_run_cap_feasibility", reason="predicted quartet exceeds retained storage headroom")
    if any(value["time_ns"] >= 900 * 10**9 for value in predictions.values()):
        assessment.update(status="not_run_cap_feasibility", reason="predicted cell time is not strictly below900s")
    return assessment


def _run_required(spec, records, budget, prepare, initial):
    """Retain all required rows; call preparation only for admitted obligations.

    Records and budget are caller-owned observations; this seam never opens a
    measurement store or the live shared allocation. The eventual driver owns
    bounded execution and actual qualification, not this mathematical control.
    """
    outcomes = []
    for n in (10000, 100000, 300000):
        for mode in ("held", "overlap"):
            assessment = ({"status": "not_run_stage_review", "reason": "mandatory review after initial held quartet"}
                          if initial and (n != 10000 or mode != "held")
                          else assess_stage(n, mode, records, budget))
            for cell in spec["cells"]:
                if cell["n"] != n or cell["mode"] != mode:
                    continue
                previous = records.get(cell["id"])
                if previous and previous.get("status") == "complete":
                    outcomes.append({"id": cell["id"], "status": "complete", "record": previous})
                elif assessment["status"] != "admit":
                    outcomes.append({"id": cell["id"], **assessment})
                else:
                    result = prepare(cell)
                    outcomes.append({"id": cell["id"], **result})
    return outcomes


def run_initial_quartet(spec, records, budget, prepare):
    """Run the named initial held quartet; retain every other obligation."""
    return _run_required(spec, records, budget, prepare, initial=True)


def run_required(spec, records, budget, prepare):
    """Assess the fixed24 continuation, preserving every outstanding obligation."""
    return _run_required(spec, records, budget, prepare, initial=False)
