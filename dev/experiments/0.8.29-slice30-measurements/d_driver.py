"""D fixed-table driver seam; live supervision and scientific runtime remain gated."""

import time

from d_controller import run_initial_quartet, run_required


def run_accounted_stage(spec, records, budget, execute, *, initial=True, clock=time.monotonic_ns):
    """Drive fixed obligations using one outer clock and caller-owned aggregate state."""
    del clock
    run = run_initial_quartet if initial else run_required
    return run(spec, records, budget, lambda cell: execute(cell, None))
