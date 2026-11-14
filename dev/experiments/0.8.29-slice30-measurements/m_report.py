"""M client outcome populations; no exclusive timing or numerical verdict."""

from metrics import aggregate


def summarize_invocations(records, wall_ns, *, stderr=(), log_level="warn"):
    """Summarize returned outcomes using the existing sample statistics."""
    completed = [row for row in records if row["status"] == "completed"]
    return aggregate(completed, wall_ns)
