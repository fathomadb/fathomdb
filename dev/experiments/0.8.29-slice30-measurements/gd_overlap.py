"""Explain worker/query/write overlap on one attempt-local monotonic clock."""

import json
import sys


def analyze(queries, writes, workers, readiness, enabled=True, dropped=0):
    """Return the overlap verdict; disabled worker attribution is unavailable."""
    return {"status": "NOT_EVALUATED"}


def main():
    """Read one JSON analysis request and emit its analysis report."""
    print(json.dumps(analyze(**json.load(sys.stdin))))


if __name__ == "__main__":
    main()
