"""Pure G/D lower-scale feasibility reports; no database, model or launcher."""

import json
import sys


def report(observation):
    """Return the time dimension for positive lower/target sizes and elapsed ns.

    Scale complete observed elapsed time by target/lower and fixed1.5 headroom,
    rounding up to ns. Compare with870s work admission, reserving30s in900s.
    This component alone never admits a database/model/campaign workload.
    """
    numerator = observation["elapsed_ns"] * observation["target_n"] * 3
    denominator = observation["lower_n"] * 2
    projected = (numerator + denominator - 1) // denominator
    return {"time": {"projected_ns": projected, "fits": projected <= 870_000_000_000,
                     "cap_ns": 870_000_000_000}}


def main():
    """Read one lower-scale observation from stdin and print its report."""
    print(json.dumps(report(json.load(sys.stdin))))


if __name__ == "__main__":
    main()
