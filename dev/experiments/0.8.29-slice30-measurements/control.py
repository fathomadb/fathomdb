"""Freeze, run and summarize the bounded Slice30 provider-free campaign."""

import argparse
from pathlib import Path


def freeze(cells, corpus, queries, binary, out):
    """Freeze actual input/native/source identities and the balanced cell plan."""
    return {}


def main():
    """Operate an explicitly selected local campaign; never build in timings."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    freeze_parser = sub.add_parser("freeze")
    for field in ("cells", "corpus", "queries", "binary", "out"):
        freeze_parser.add_argument("--" + field, type=Path, required=True)
    args = parser.parse_args()
    freeze(args.cells, args.corpus, args.queries, args.binary, args.out)


if __name__ == "__main__":
    main()
