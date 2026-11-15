"""Private Slice50 scope inside the existing P/G measurement entry."""

import json
from pathlib import Path


def plan():
    """Return explicit ordinary, diagnostic and separate lower-stage obligations."""
    return {"primary": [], "diagnostics": [], "calibration": []}


def report(root):
    """Keep all current obligations visible when no native member has run."""
    return {"scope": "slice50-pair", "complete": False, **plan()}
