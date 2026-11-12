"""Adapt the selected P corpus JSONL to the private normal-G record input."""

import argparse
import json
from pathlib import Path


def encode_records(corpus, output):
    """Write selected bodies in order to a fresh record input and return its count."""
    count = 0
    with Path(corpus).open("rb") as source, Path(output).open("xb") as stream:
        for index, line in enumerate(source):
            record = {"id": f"s30doc-{index}", "kind": "doc", "body": json.loads(line)}
            stream.write((json.dumps(record) + "\n").encode("utf-8"))
            count += 1
    return {"records": count}


def main():
    """Encode an existing selected corpus; never reselect turns or questions."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(encode_records(args.corpus, args.out)))


if __name__ == "__main__":
    main()
