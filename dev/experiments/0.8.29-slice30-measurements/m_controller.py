"""M table dispatch primitives; campaign entry/qualification remains separate."""

import json
from pathlib import Path

from campaign import make_plan, run_campaign


def load_plan():
    """Select the current measurement inventory using the existing balanced selector."""
    return make_plan(json.loads(Path(__file__).with_name("cells.json").read_text()), 2)


def run_table(root, command, *, plan=None):
    """Dispatch an explicit plan to bounded caller-owned children in fresh attempts."""
    return run_campaign(load_plan() if plan is None else plan, root, command)
