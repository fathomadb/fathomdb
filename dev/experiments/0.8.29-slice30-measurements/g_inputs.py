"""Adapt selected P corpus/question JSONL to private normal-G record inputs."""

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


def encode_queries(queries, output):
    """Write selected questions in source order to a fresh input and return its count."""
    count = 0
    with Path(queries).open("rb") as source, Path(output).open("xb") as stream:
        for index, line in enumerate(source):
            record = {"id": index, "query": json.loads(line)}
            stream.write((json.dumps(record) + "\n").encode("utf-8"))
            count += 1
    return {"records": count}


def main():
    """Encode selected bodies/questions without reselecting turns or questions."""
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--corpus", type=Path)
    source.add_argument("--queries", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = encode_records(args.corpus, args.out) if args.corpus is not None else encode_queries(args.queries, args.out)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
