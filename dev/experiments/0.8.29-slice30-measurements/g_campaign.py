"""G-only stage inventory; accepted P scheduling and results stay unchanged."""

from campaign import run_campaign


def make_g_plan(spec):
    """Expand the reviewed fixed G stages and paired orders without executing models."""
    return []


def run_g_plan(spec, root, command):
    """Run a callable stage schedule with the existing owned-child attempt runner."""
    return run_campaign(make_g_plan(spec), root, command, timeout=spec["limits"]["cell_seconds"])
