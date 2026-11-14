"""Fixed D admission seam; actual worker execution/qualification is separate."""


def assess_stage(n, mode, records, budget):
    """Return prospective admission for one required size/mode quartet."""
    if n > 10000:
        lower = 10000 if n == 100000 else 100000
        for layer in ("C", "T"):
            for repeat in (0, 1):
                record = records.get(f"d-{layer.lower()}-{mode}-n{lower}-r{repeat}")
                qualified = (record is not None and record.get("status") == "complete"
                             and record.get("closed") is True and record.get("error_count") == 0
                             and record.get("physical_count") == lower)
                if qualified and layer == "C":
                    qualified = (record.get("observations_complete") is True
                                 and record.get("dropped_records") == 0
                                 and (mode != "overlap" or record.get("actual_overlap") is True))
                if not qualified:
                    return {"status": "not_run_lower_evidence", "lower_n": lower,
                            "reason": "required lower C/T repeats are missing or unqualified"}
    return {"status": "admit"}


def run_required(spec, records, budget, prepare):
    """Retain all required rows; call preparation only for admitted obligations.

    Records and budget are caller-owned observations; this seam never opens a
    measurement store or the live shared allocation. The eventual driver owns
    bounded execution and actual qualification, not this mathematical control.
    """
    outcomes = []
    for n in (10000, 100000, 300000):
        for mode in ("held", "overlap"):
            assessment = assess_stage(n, mode, records, budget)
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
