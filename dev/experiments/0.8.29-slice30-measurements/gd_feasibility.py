"""Pure G/D lower-scale feasibility reports; no database, model or launcher."""

import json
import sys


def report(observation):
    """Return the time dimension for positive lower/target sizes and elapsed ns.

    Scale complete observed elapsed time by target/lower and fixed1.5 headroom,
    rounding up to ns. Compare with870s work admission, reserving30s in900s.
    Optional peak/retained bytes similarly forecast additional family retention
    against16GiB. These components alone never admit any workload.
    """
    numerator = observation["elapsed_ns"] * observation["target_n"] * 3
    denominator = observation["lower_n"] * 2
    projected = (numerator + denominator - 1) // denominator
    result = {"time": {"projected_ns": projected, "fits": projected <= 870_000_000_000,
                       "cap_ns": 870_000_000_000}}
    if "peak_bytes" in observation and "retained_family_bytes" in observation:
        numerator = observation["peak_bytes"] * observation["target_n"] * 3
        additional = (numerator + denominator - 1) // denominator
        family_bytes = observation["retained_family_bytes"] + additional
        result["storage"] = {"additional_bytes": additional, "family_bytes": family_bytes,
                             "cap_bytes": 16*1024**3, "fits": family_bytes <= 16*1024**3}
    return result


def main():
    """Read one lower-scale observation from stdin and print its report."""
    print(json.dumps(report(json.load(sys.stdin))))


if __name__ == "__main__":
    main()
