"""Bind retained D observations to one execution; native capture remains separate."""


def bind_execution(cell, actual, observation=None):
    """Return exact-execution qualification without borrowing another cell's evidence."""
    return {"cell": cell, "complete": False, "observations_complete": None,
            "committed_success_rows": None, "committed_cursors": None,
            "worker_groups": None, "actual_overlap": None, "overlap": {"verdict": "INCOMPLETE"},
            "phases": {}, "issues": ["D execution report binding is not implemented"]}
