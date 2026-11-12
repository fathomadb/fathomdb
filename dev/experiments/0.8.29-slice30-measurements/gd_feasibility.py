"""Pure G/D lower-scale feasibility reports; no database, model or launcher."""

import json
import sys


def report(observation):
    """Report prospective dimensions; unavailable components never admit work."""
    return {"time": {"projected_ns": None, "fits": None, "cap_ns": 870_000_000_000}}


def main():
    """Read one lower-scale observation from stdin and print its report."""
    print(json.dumps(report(json.load(sys.stdin))))


if __name__ == "__main__":
    main()
