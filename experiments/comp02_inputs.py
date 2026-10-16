"""Build COMP-02 inputs from the qualified SCALE-02 TC-5 input pack.

Output directory per tier: ``corpus.json`` (``[{id, body, cohort}]`` in pack
order), ``queries.json`` (the 100 TC-5 queries), ``probes.json`` (readiness
probes), and ``manifest.json`` (counts, hashes, observed cohort sizes).

Cohorts are disjoint, deterministic, and target 1%, 10%, and 33% of documents
(``c01``, ``c10``, ``c33``; the remainder is ``rest``). A readiness probe is a
document at a fixed position (first, quartiles, last) and its rarest plain
lowercase token, so any lexical engine that has indexed the document returns it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

PACK = Path("data/performance-benchmarking/scale-02/inputs/input-pack-v1/tc5")
COHORTS = (("c01", 1), ("c10", 10), ("c33", 33))
_TOKEN = re.compile(r"[a-z]{6,}")


def cohort_for(document_id: str) -> str:
    bucket = int(hashlib.sha256(f"bench_cohort:{document_id}".encode()).hexdigest(), 16) % 100
    floor = 0
    for name, percent in COHORTS:
        if bucket < floor + percent:
            return name
        floor += percent
    return "rest"


def probe_positions(count: int) -> list[int]:
    return sorted({0, count // 4, count // 2, (3 * count) // 4, count - 1})


def _tokens(text: str) -> set[str]:
    return set(_TOKEN.findall(text.lower()))


def build(pack: Path, size: int, output: Path) -> dict[str, Any]:
    """Write one tier's inputs; ``size`` documents are taken in pack order."""
    index = json.loads((pack / "tc5-corpus-input.v1.json").read_text(encoding="utf-8"))
    entries = index["documents"][:size]
    if len(entries) != size:
        raise ValueError(f"pack has only {len(entries)} documents")
    corpus = []
    for entry in entries:
        body = (pack / entry["relative_path"]).read_text(encoding="utf-8")
        if hashlib.sha256(body.encode()).hexdigest() != entry["content_sha256"]:
            raise ValueError(f"content hash mismatch: {entry['document_id']}")
        corpus.append({"id": entry["document_id"], "body": body, "cohort": cohort_for(entry["document_id"])})
    queries = [
        {"id": q["query_id"], "text": (pack / q["relative_path"]).read_text(encoding="utf-8").strip()}
        for q in index["queries"]
    ]
    df: Counter[str] = Counter()
    for row in corpus:
        df.update(_tokens(row["body"]))
    probes = []
    for position in probe_positions(size):
        row = corpus[position]
        candidates = sorted(_tokens(row["body"]), key=lambda t: (df[t], t))
        if not candidates:
            raise ValueError(f"no probe token for document {row['id']}")
        probes.append({"position": position, "id": row["id"], "text": candidates[0], "df": df[candidates[0]]})
    output.mkdir(parents=True, exist_ok=False)
    files = {"corpus.json": corpus, "queries.json": queries, "probes.json": probes}
    for name, value in files.items():
        (output / name).write_text(json.dumps(value, ensure_ascii=False) + "\n", encoding="utf-8")
    observed = Counter(row["cohort"] for row in corpus)
    manifest = {
        "schema_version": "comp02.input.v1",
        "documents": size,
        "queries": len(queries),
        "fixture": "qualified real TC-5 prefix",
        "cohort_counts": dict(sorted(observed.items())),
        "sha256": {name: hashlib.sha256((output / name).read_bytes()).hexdigest() for name in files},
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, default=PACK)
    parser.add_argument("--size", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(build(args.pack, args.size, args.output), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
