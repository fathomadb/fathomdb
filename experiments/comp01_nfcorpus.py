"""Freeze the local BEIR NFCorpus test split for COMP-01 FTS experiments."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _read_jsonl(path: Path, *, label: str) -> dict[str, dict[str, Any]]:
    records: dict[str, dict[str, Any]] = {}
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{label} line {line_number} is invalid JSON") from exc
        if (
            not isinstance(row, dict)
            or not isinstance(row.get("_id"), str)
            or not row["_id"]
        ):
            raise ValueError(f"{label} line {line_number} lacks an ID")
        identifier = row["_id"]
        if identifier in records:
            raise ValueError(f"duplicate {label} ID: {identifier}")
        records[identifier] = row
    return records


def _qrels(path: Path) -> dict[str, dict[str, int]]:
    gold: dict[str, dict[str, int]] = defaultdict(dict)
    with path.open(encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if reader.fieldnames != ["query-id", "corpus-id", "score"]:
            raise ValueError("qrels/test.tsv header mismatch")
        for row in reader:
            query_id, document_id = row["query-id"], row["corpus-id"]
            if not query_id or not document_id or document_id in gold[query_id]:
                raise ValueError("qrels contains an empty or duplicate judgment ID")
            try:
                grade = int(row["score"])
            except (TypeError, ValueError) as exc:
                raise ValueError("qrels grade is not an integer") from exc
            if grade < 0:
                raise ValueError("qrels grade must be nonnegative")
            gold[query_id][document_id] = grade
    return dict(gold)


def _write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n",
        encoding="utf-8",
    )


def convert(
    source: Path,
    output: Path,
    *,
    expected_documents: int = 3633,
    expected_queries: int = 323,
) -> dict[str, Any]:
    """Convert and hash the declared test split, rejecting ID or count drift.

    Source files remain untouched. The output directory must not exist; its
    converted text and judgments are external experiment inputs, not Git data.
    """
    source = Path(source)
    output = Path(output)
    if output.exists():
        raise FileExistsError(f"converted input directory already exists: {output}")
    paths = {
        "corpus.jsonl": source / "corpus.jsonl",
        "queries.jsonl": source / "queries.jsonl",
        "qrels/test.tsv": source / "qrels/test.tsv",
    }
    corpus_raw = _read_jsonl(paths["corpus.jsonl"], label="corpus")
    queries_raw = _read_jsonl(paths["queries.jsonl"], label="query")
    gold = _qrels(paths["qrels/test.tsv"])
    if len(corpus_raw) != expected_documents or len(gold) != expected_queries:
        raise ValueError("NFCorpus document or test query count drift")
    if not set(gold) <= set(queries_raw):
        raise ValueError("test qrel query ID missing from queries.jsonl")
    judged_ids = {identifier for judgments in gold.values() for identifier in judgments}
    if not judged_ids <= set(corpus_raw):
        raise ValueError("qrel document ID missing from corpus.jsonl")
    corpus: list[dict[str, str]] = []
    body_ids: dict[str, list[str]] = defaultdict(list)
    for document_id, row in sorted(corpus_raw.items()):
        title, document = row.get("title"), row.get("text")
        if not isinstance(title, str) or not isinstance(document, str):
            raise ValueError(f"corpus title/text is not text: {document_id}")
        body = f"{title}\n\n{document}" if title else document
        if not body:
            raise ValueError(f"empty indexed body: {document_id}")
        corpus.append({"id": document_id, "body": body})
        body_ids[body].append(document_id)
    queries: list[dict[str, str]] = []
    for query_id in sorted(gold):
        query = queries_raw[query_id].get("text")
        if not isinstance(query, str) or not query:
            raise ValueError(f"empty test query: {query_id}")
        queries.append({"id": query_id, "text": query})
    groups = sorted(sorted(ids) for ids in body_ids.values() if len(ids) > 1)
    duplicate_ids = {identifier for group in groups for identifier in group}
    relevant_affected = sorted(
        query_id
        for query_id, judgments in gold.items()
        if any(
            grade > 0 and doc_id in duplicate_ids for doc_id, grade in judgments.items()
        )
    )
    output.mkdir(parents=True)
    converted = {
        "corpus.json": corpus,
        "queries.json": queries,
        "gold.json": {
            qid: dict(sorted(judgments.items()))
            for qid, judgments in sorted(gold.items())
        },
    }
    for name, value in converted.items():
        _write_json(output / name, value)
    manifest = {
        "schema_version": "comp01.nfcorpus-input.v1",
        "counts": {
            "documents": len(corpus),
            "queries": len(queries),
            "judgments": sum(len(judgments) for judgments in gold.values()),
        },
        "test_query_ids": sorted(gold),
        "body_mapping": "title + two newlines + text; text if title empty",
        "qrel_grades": "original nonnegative integers; positive grades count as relevant",
        "duplicate_body_groups": groups,
        "queries_with_relevant_duplicate_body": relevant_affected,
        "source_sha256": {name: _sha256(path) for name, path in paths.items()},
        "converted_sha256": {name: _sha256(output / name) for name in converted},
    }
    _write_json(output / "manifest.json", manifest)
    return manifest


def main() -> int:
    """Convert a local raw NFCorpus split into external experiment inputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    manifest = convert(args.source, args.output)
    print(
        json.dumps(
            {
                "counts": manifest["counts"],
                "converted_sha256": manifest["converted_sha256"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
