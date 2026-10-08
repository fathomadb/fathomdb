#!/usr/bin/env python3
"""Map source-owned MuSiQue and LOCOMO support labels to local eval inputs."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
from typing import Any


def _hash(value: str) -> str:
    return hashlib.sha256(value.encode()).hexdigest()


def build_musique(
    path: Path, *, quotas: dict[int, int], salt: str,
) -> dict[str, Any]:
    """Select answerable rows by a fixed hash and preserve complete support sets."""
    if not path.is_file() or not quotas or not salt or any(
        hop not in {2, 3, 4} or count < 1 for hop, count in quotas.items()
    ):
        raise ValueError("invalid MuSiQue source or selection")
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    answerable = [row for row in rows if row.get("answerable") is True]
    ids = [row.get("id") for row in answerable]
    if any(not isinstance(value, str) or not value for value in ids) or len(set(ids)) != len(ids):
        raise ValueError("duplicate or invalid answerable question ID")
    selected = []
    for hop, count in sorted(quotas.items()):
        pool = sorted(
            (row for row in answerable if row.get("hop_count") == hop),
            key=lambda row: _hash(f"{salt}:{row['id']}"),
        )
        if len(pool) < count:
            raise ValueError("MuSiQue class underfilled")
        selected.extend(pool[:count])
    documents: dict[str, dict[str, str]] = {}
    queries = []
    for row in selected:
        hop = row["hop_count"]
        paragraphs = row.get("paragraphs")
        question = row.get("question")
        answer = row.get("answer")
        if (not isinstance(paragraphs, list) or not paragraphs
                or not isinstance(question, str) or not question.strip()
                or not isinstance(answer, str) or not answer.strip()):
            raise ValueError("invalid MuSiQue answerable row")
        indices: set[int] = set()
        required = []
        for paragraph in sorted(paragraphs, key=lambda item: item["idx"]):
            idx = paragraph.get("idx")
            title = paragraph.get("title")
            passage = paragraph.get("text")
            if (isinstance(idx, bool) or not isinstance(idx, int) or idx in indices
                    or not isinstance(title, str) or not title.strip()
                    or not isinstance(passage, str) or not passage.strip()
                    or not isinstance(paragraph.get("is_supporting"), bool)):
                raise ValueError("invalid MuSiQue paragraph or duplicate index")
            indices.add(idx)
            body = f"{title}\n{passage}"
            logical_id = f"musique:{_hash(body)}"
            existing = documents.get(logical_id)
            if existing is not None and existing["body"] != body:
                raise ValueError("MuSiQue body hash collision")
            documents[logical_id] = {
                "logical_id": logical_id, "body": body, "source_id": "slice135-musique",
            }
            if paragraph["is_supporting"]:
                required.append(logical_id)
        if len(required) != hop or len(set(required)) != hop:
            raise ValueError("incomplete or duplicate MuSiQue support set")
        queries.append({
            "query_id": row["id"], "query_class": {
                2: "two_hop", 3: "three_hop", 4: "four_hop",
            }[hop],
            "text": question, "required_ids": required, "answers": [answer],
        })
    return {
        "dataset": "musique", "selection": {"salt": salt, "quotas": quotas},
        "documents": [documents[key] for key in sorted(documents)],
        "queries": queries,
    }


def build_locomo(path: Path) -> dict[str, Any]:
    """Map the existing LOCOMO loader's positive labels to session documents."""
    if not path.is_file():
        raise ValueError("LOCOMO source missing")
    loader_path = Path(__file__).resolve().parents[1] / "src/python/eval/locomo_loader.py"
    specification = importlib.util.spec_from_file_location("slice135_locomo_loader", loader_path)
    assert specification is not None and specification.loader is not None
    loader = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(loader)
    source_documents, source_queries = loader.load_locomo(path)
    if not source_documents or not source_queries:
        raise ValueError("LOCOMO loader produced empty positive corpus")
    documents = []
    for logical_id, body in sorted(source_documents.items()):
        if not logical_id or not isinstance(body, str) or not body.strip():
            raise ValueError("invalid LOCOMO session document")
        conversation = logical_id.split(":session_", 1)[0]
        documents.append({
            "logical_id": logical_id, "body": body,
            "source_id": f"slice135-locomo:{conversation}",
        })
    allowed = {document["logical_id"] for document in documents}
    queries = []
    seen: set[str] = set()
    for row in source_queries:
        query_id = row.get("query_id")
        query_class = row.get("query_class")
        text = row.get("query")
        required = [e.get("doc_id") for e in row.get("required_evidence", [])]
        answers = row.get("answers")
        if (not isinstance(query_id, str) or not query_id or query_id in seen
                or query_class not in {"factoid", "temporal", "multi_session"}
                or not isinstance(text, str) or not text.strip()
                or not required or len(set(required)) != len(required)
                or any(value not in allowed for value in required)
                or not isinstance(answers, list) or not answers):
            raise ValueError("invalid LOCOMO positive query or support set")
        seen.add(query_id)
        queries.append({
            "query_id": query_id, "query_class": query_class, "text": text,
            "required_ids": required, "answers": answers,
            "strict_multi_session": query_class == "multi_session" and len(required) >= 2,
        })
    return {"dataset": "locomo", "selection": {"classes": [
        "factoid", "temporal", "multi_session",
    ]}, "documents": documents, "queries": queries}


def main() -> None:
    """Write one eval-only derived corpus outside Git."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset", required=True, choices=("musique", "locomo"))
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.dataset == "musique":
        data = build_musique(args.source, quotas={2: 40, 3: 35, 4: 25}, salt="slice135-musique-v1")
    else:
        data = build_locomo(args.source)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.output.exists():
        raise ValueError("derived input already exists")
    args.output.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
