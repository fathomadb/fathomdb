"""Fixed D admission seam; actual worker execution/qualification is separate."""


def assess_stage(n, mode, records, budget):
    """Return prospective admission for one required size/mode quartet."""
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
