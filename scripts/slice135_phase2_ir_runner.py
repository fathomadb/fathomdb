#!/usr/bin/env python3
"""Collect installed-wheel judged retrieval observations on fresh databases."""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import platform
import sys
from typing import Any
import zipfile


def sha256(path: Path) -> str:
    """Hash a local input without loading it into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _hash_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def build_documents(source_paths: dict[str, Path]) -> list[dict[str, str]]:
    """Map each source document to one stable public write ID and JSON body."""
    if not source_paths:
        raise ValueError("no corpus sources")
    documents: list[dict[str, str]] = []
    for source, path in source_paths.items():
        if not source or not path.is_file():
            raise ValueError("missing corpus source")
        rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        seen: set[str] = set()
        for row in sorted(rows, key=lambda item: item["doc_id"]):
            doc_id = row.get("doc_id")
            body = row.get("body")
            if not isinstance(doc_id, str) or not doc_id or not isinstance(body, str) or not body.strip():
                raise ValueError("invalid source document")
            if doc_id in seen:
                raise ValueError("duplicate source document ID")
            seen.add(doc_id)
            documents.append({
                "kind": "doc",
                "body": json.dumps({"summary": body}, ensure_ascii=False, separators=(",", ":")),
                "logical_id": f"{source}:{doc_id}",
                "source_id": f"slice135-ir:{source}",
                "source": source,
                "source_doc_id": doc_id,
            })
    return documents


def select_positive_queries(
    gold: dict[str, Any], document_ids: set[str],
) -> tuple[list[dict[str, str]], int]:
    """Validate required-document gold and reserve negatives for abstention."""
    raw_queries = gold.get("queries")
    if not isinstance(raw_queries, list) or not raw_queries:
        raise ValueError("missing gold queries")
    positive: list[dict[str, str]] = []
    negative = 0
    seen: set[str] = set()
    for query in raw_queries:
        if not isinstance(query, dict):
            raise ValueError("malformed gold query")
        query_id = query.get("query_id")
        source = query.get("source")
        text = query.get("query")
        query_class = query.get("query_class")
        if (not isinstance(query_id, str) or not query_id or query_id in seen
                or not isinstance(source, str) or not source
                or not isinstance(text, str) or not text.strip()):
            raise ValueError("duplicate or malformed gold query")
        seen.add(query_id)
        expected = query.get("expected_top_k_doc_ids")
        evidence = query.get("required_evidence")
        if not isinstance(expected, list) or not isinstance(evidence, list):
            raise ValueError("invalid required-document gold")
        if query_class == "negative":
            if expected or evidence:
                raise ValueError("negative query has required evidence")
            negative += 1
            continue
        if query_class not in {"exact_fact", "exploratory"}:
            raise ValueError("unsupported gold class")
        if (len(expected) != 1 or not isinstance(expected[0], str) or not expected[0]
                or len(evidence) != 1 or not isinstance(evidence[0], dict)
                or evidence[0].get("doc_id") != expected[0]
                or evidence[0].get("necessity") != "required"):
            raise ValueError("required evidence disagrees with named document")
        required_id = f"{source}:{expected[0]}"
        if required_id not in document_ids:
            raise ValueError("required document absent from corpus")
        positive.append({
            "query_id": query_id, "source": source, "query_class": query_class,
            "text": text, "required_id": required_id,
        })
    return positive, negative


def installed_identity(wheel: Path, wheel_hash: str, native_hash: str) -> dict[str, str]:
    """Verify imported installed package and native bytes against one wheel."""
    import fathomdb
    import fathomdb._fathomdb as native

    if sha256(wheel) != wheel_hash or fathomdb.__file__ is None or native.__file__ is None:
        raise ValueError("installed wheel identity mismatch")
    package = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not package.is_relative_to(prefix) or not native_path.is_relative_to(prefix):
        raise ValueError("installed SDK import escaped selected environment")
    with zipfile.ZipFile(wheel) as archive:
        for member, installed in (
            ("fathomdb/__init__.py", package),
            ("fathomdb/_fathomdb.abi3.so", native_path),
        ):
            if _hash_bytes(archive.read(member)) != sha256(installed):
                raise ValueError(f"installed file differs from wheel: {member}")
    if sha256(native_path) != native_hash:
        raise ValueError("installed native module mismatch")
    return {"module_path": str(package), "native_path": str(native_path)}


def run(
    *, protocol_path: Path, wheel: Path, version: str, source_root: Path,
    gold_path: Path, output: Path,
) -> Path:
    """Run one exact installed identity and retain every positive query outcome."""
    import fathomdb

    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if protocol.get("schema_version") != 1 or version not in protocol.get("versions", {}):
        raise ValueError("protocol schema or version mismatch")
    if sha256(Path(__file__).resolve()) != protocol.get("runner_sha256"):
        raise ValueError("runner bytes differ from frozen protocol")
    scorer = Path(__file__).with_name("slice135_phase2_ir_scorer.py")
    if sha256(scorer) != protocol.get("scorer_sha256"):
        raise ValueError("scorer bytes differ from frozen protocol")
    specification = protocol["versions"][version]
    identity = installed_identity(wheel, specification["wheel_sha256"], specification["native_sha256"])
    if sha256(gold_path) != protocol["gold"]["sha256"]:
        raise ValueError("gold SHA-256 mismatch")
    gold = json.loads(gold_path.read_text())
    if gold.get("qrels_version") != protocol["gold"]["qrels_version"] or gold.get("corpus_hash") != protocol["gold"]["corpus_hash"]:
        raise ValueError("gold identity mismatch")
    source_paths: dict[str, Path] = {}
    for item in protocol["sources"]:
        path = source_root / item["filename"]
        if sha256(path) != item["sha256"]:
            raise ValueError(f"{item['name']}: source SHA-256 mismatch")
        source_paths[item["name"]] = path
    documents = build_documents(source_paths)
    positive, negative = select_positive_queries(gold, {row["logical_id"] for row in documents})
    classes = Counter(row["query_class"] for row in positive)
    by_source = Counter(row["source"] for row in documents)
    expected = protocol["denominator"]
    if (len(documents) != expected["documents"] or len(positive) != expected["positive_queries"]
            or negative != expected["negative_queries_omitted"]
            or dict(classes) != expected["positive_by_class"]
            or dict(by_source) != expected["documents_by_source"]):
        raise ValueError("declared corpus or query denominator changed")

    output.mkdir(parents=True, exist_ok=False)
    database = output / "retrieval.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if engine.open_report().default_embedder.name != protocol["model_name"]:
            raise ValueError("default embedder identity changed")
        engine.configure_projections([fathomdb.ProjectionSpec(
            name="summary", roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}), vector=True,
        )])
        batch_size = protocol["write_batch_size"]
        for start in range(0, len(documents), batch_size):
            engine.write([{key: row[key] for key in ("kind", "body", "logical_id", "source_id")}
                          for row in documents[start:start + batch_size]])
            engine.drain(timeout_s=600)
            print(f"{version}: projected {min(start + batch_size, len(documents))}/{len(documents)}", file=sys.stderr, flush=True)
        readiness = next(
            spec.vector_dense_readiness for spec in fathomdb.read.projections(engine)
            if spec.name == "summary"
        )
        if readiness != "ready":
            raise ValueError("vector projection not ready")
    finally:
        engine.close()

    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if engine.open_report().default_embedder.name != protocol["model_name"]:
            raise ValueError("reopened embedder identity changed")
        observations = []
        for index, query in enumerate(positive, 1):
            record: dict[str, Any] = {
                "query_id": query["query_id"], "query_class": query["query_class"],
                "source": query["source"], "text": query["text"],
                "text_sha256": _hash_bytes(query["text"].encode()),
                "required_id": query["required_id"],
            }
            try:
                hits = engine.search(query["text"], limit=protocol["result_limit"]).results
                record.update({"status": "ok", "hits": [
                    {"doc_id": hit.id.value, "branch": hit.branch} for hit in hits
                ]})
            except Exception as error:
                record.update({"status": "error", "error_type": type(error).__name__, "hits": []})
            observations.append(record)
            if index % 100 == 0:
                print(f"{version}: queried {index}/{len(positive)}", file=sys.stderr, flush=True)
    finally:
        engine.close()
    raw = {
        "schema_version": 1, "version": version,
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": _hash_bytes(protocol_bytes),
        "runner_sha256": sha256(Path(__file__).resolve()),
        "scorer_sha256": sha256(scorer),
        "gold_sha256": sha256(gold_path),
        "database_sha256": sha256(database),
        "identity": identity,
        "environment": {
            "python_executable": str(Path(sys.executable).resolve()),
            "python_version": sys.version, "platform": platform.platform(),
        },
        "model_name": protocol["model_name"],
        "documents": [{
            "logical_id": row["logical_id"], "source": row["source"],
            "source_doc_id": row["source_doc_id"],
            "body_sha256": _hash_bytes(row["body"].encode()),
        } for row in documents],
        "negative_queries_omitted": negative,
        "queries": observations,
    }
    path = output / "raw.json"
    path.write_text(json.dumps(raw, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n")
    return path


def main() -> None:
    """Parse one identity-bound paired-retrieval collection request."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--version", required=True, choices=("baseline", "candidate"))
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--gold", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    print(run(
        protocol_path=args.protocol, wheel=args.wheel, version=args.version,
        source_root=args.source_root, gold_path=args.gold, output=args.output,
    ))


if __name__ == "__main__":
    main()
