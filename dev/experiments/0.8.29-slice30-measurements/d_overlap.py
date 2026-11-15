"""Exact-execution D overlap classification; raw intervals are not exclusive costs."""

import json
from pathlib import Path
import sys


def classify_overlap(case):
    """Return an unavailable verdict until exact interval qualification is implemented."""
    return {"verdict": "INCOMPLETE", "reason": "overlap qualification is not implemented"}


if __name__ == "__main__":
    print(json.dumps(classify_overlap(json.loads(Path(sys.argv[1]).read_text()))))
