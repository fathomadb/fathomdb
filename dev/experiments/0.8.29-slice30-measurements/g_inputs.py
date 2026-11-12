"""Adapt the selected P corpus JSONL to the private normal-G record input."""

import argparse
import json
from pathlib import Path


def encode_records(corpus, output):
    """Write selected bodies in order to a fresh record input and return its count."""
    payload = Path(corpus).read_bytes()
    bodies = [json.loads(line) for line in payload.splitlines()]
    with Path(output).open("xb") as stream:
        stream.write(payload)
    return {"records": len(bodies)}


def main():
    """Encode an existing selected corpus; never reselect turns or questions."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(encode_records(args.corpus, args.out)))


if __name__ == "__main__":
    main()
